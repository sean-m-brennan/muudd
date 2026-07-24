/*
  Copyright 2026 Sean M. Brennan and contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! Charter authority rules. These range over the whole archive; they are kept
//! separate from signature verification (`entry::verify_all`), which is purely
//! cryptographic.
//!
//! Rules:
//! - **Genesis singleton** — at most one charter per polity DID.
//! - **Majority ratification** — a founding needs a majority of its roll to
//!   ratify (and co-sign). This is the v1 answer to "what stops a faction from
//!   chartering the whole cluster over the majority's head" (design open
//!   question Q1); a stricter or provider-attested rule may supersede it.
//! - **Genesis-immutable core** — the substrate has no amendment path at all,
//!   and founding commitments cannot be removed.
//! - **Asymmetric amendment** — a non-founding commitment is *added* at a
//!   majority and *removed* only at a majority **+ 1**, so contraction (the
//!   capture-relevant direction) is always strictly harder than expansion.
//! - **Membership lifecycle** — the roll changes only by signed record, with the
//!   same asymmetry: *Admit* needs a majority of the current roll **and** the
//!   joiner's own co-signature (no conscription); *DepartVoluntary* is signed by
//!   the leaving member alone (no forced stay); *Expel* needs a majority **+ 1**
//!   with the target excluded. A contraction is additionally refused when it would
//!   drop the roll below the **self-govern floor** — the point at which it could
//!   no longer form the `majority + 1` its own contraction rules require.
//! - **Office lifecycle** — offices are conferred by the *roll*, never by other
//!   offices (so the founding roll confers the first ones and there is no
//!   regress): *Confer* needs a majority of the current roll **and** the holder's
//!   co-signature (consent), on a current member; *Resign* is signed by the holder
//!   alone; *Revoke* needs a majority with the holder excluded. A holding is
//!   effective only while its holder is on the roll.
//! - **Transmission** — admission puts someone on the roll; *induction* is the
//!   separate step by which they acquire the substrate, and it is **witnessed by
//!   the roll**: a majority of current members co-sign alongside the inductee (the
//!   community receives them, and they participate). Founding members are inducted
//!   by ratifying the charter, so they take no induction record and cannot be
//!   inducted again. *Re-affirmation* is self-signed by an already-inducted member
//!   — renewal is the member's own act, and its freshness decays
//!   ([`crate::AffirmationState::stale_members`]).
//! - **Terminal dissolution** — once a polity is dissolved, no further charter
//!   event (amendment, membership change, office change, affirmation, or a second
//!   dissolution) is authorized.
//!
//! Like kith-covenant's constitution rules, state is reconstructed by walking
//! entries *before* the one under test in insertion order; an unauthorized prior
//! amendment still shifts the walked state, so accept and reject cases must be
//! tested in separate archives.

use std::collections::BTreeSet;

use en_archive::Archive;
use en_core::{Cid, DidKey, KcError, KcResult};

use crate::entry::CharterEntry;
use crate::record::{AffirmationKind, AmendmentKind, MembershipKind, Office, OfficeKind};

/// Majority of `n`: `n / 2 + 1`.
pub fn majority(n: usize) -> usize {
    n / 2 + 1
}

/// Whether a roll of size `n` can still form the `majority + 1` its own
/// contraction rules require — the **self-govern floor**. False below `n = 3`.
/// A departure or expulsion that would drop the roll below this is refused, so a
/// polity can never shrink itself into a state where it cannot act on its own
/// contraction rules (a roll can only shed a member from size ≥ 4).
fn can_self_govern(n: usize) -> bool {
    majority(n) + 1 <= n
}

fn did_set(dids: &[DidKey]) -> BTreeSet<String> {
    dids.iter().map(|d| d.as_str().to_string()).collect()
}

struct Reconstructed {
    chartered: bool,
    dissolved: bool,
    members: Vec<DidKey>,
    founding: BTreeSet<String>,
    in_force: BTreeSet<String>,
    /// Raw office grants (Confer − Resign/Revoke), not yet intersected with the
    /// live roll; the authority pass computes the effective holdings.
    offices: Vec<(Office, DidKey)>,
    /// The founding roll, set once at genesis and never mutated: founding members
    /// are inducted by ratifying the charter.
    founding_members: Vec<DidKey>,
    /// Members with a prior `Induction` record.
    inducted: Vec<DidKey>,
}

/// Reconstruct a polity's charter state from entries strictly before `upto`.
fn reconstruct(archive: &Archive<CharterEntry>, polity: &DidKey, upto: usize) -> Reconstructed {
    let mut r = Reconstructed {
        chartered: false,
        dissolved: false,
        members: Vec::new(),
        founding: BTreeSet::new(),
        in_force: BTreeSet::new(),
        offices: Vec::new(),
        founding_members: Vec::new(),
        inducted: Vec::new(),
    };
    for (idx, (_cid, e)) in archive.iter().enumerate() {
        if idx >= upto {
            break;
        }
        match e {
            CharterEntry::PolityChartered(s) if &s.payload.polity == polity && !r.chartered => {
                r.chartered = true;
                r.members = s.payload.members.clone();
                r.founding_members = s.payload.members.clone();
                for c in &s.payload.founding_commitments {
                    r.founding.insert(c.key.clone());
                    r.in_force.insert(c.key.clone());
                }
            }
            CharterEntry::CommitmentAmended(s) if &s.payload.polity == polity => match s.payload.kind {
                AmendmentKind::Add => {
                    r.in_force.insert(s.payload.commitment.key.clone());
                }
                AmendmentKind::Remove => {
                    r.in_force.remove(&s.payload.commitment.key);
                }
            },
            CharterEntry::MembershipChanged(s) if &s.payload.polity == polity => {
                let m = &s.payload;
                match m.kind {
                    MembershipKind::Admit => {
                        if !r.members.contains(&m.member) {
                            r.members.push(m.member.clone());
                        }
                    }
                    MembershipKind::DepartVoluntary | MembershipKind::Expel => {
                        r.members.retain(|d| d != &m.member);
                    }
                }
            }
            CharterEntry::OfficeChanged(s) if &s.payload.polity == polity => {
                let o = &s.payload;
                let held = |h: &(Office, DidKey)| h.0 == o.office && h.1 == o.holder;
                match o.kind {
                    OfficeKind::Confer => {
                        if !r.offices.iter().any(|h| held(h)) {
                            r.offices.push((o.office.clone(), o.holder.clone()));
                        }
                    }
                    OfficeKind::Resign | OfficeKind::Revoke => {
                        r.offices.retain(|h| !held(h));
                    }
                }
            }
            CharterEntry::MemberAffirmed(s) if &s.payload.polity == polity => {
                let a = &s.payload;
                if a.kind == AffirmationKind::Induction && !r.inducted.contains(&a.member) {
                    r.inducted.push(a.member.clone());
                }
            }
            CharterEntry::CharterDissolved(s) if &s.payload.polity == polity => {
                r.dissolved = true;
            }
            _ => {}
        }
    }
    r
}

/// All authority violations in the archive, each as `(entry_cid, reason)`.
pub fn authority_violations(archive: &Archive<CharterEntry>) -> Vec<(Cid, String)> {
    let mut out = Vec::new();
    for (idx, (cid, e)) in archive.iter().enumerate() {
        match e {
            CharterEntry::PolityChartered(s) => {
                let p = &s.payload;
                let prior = reconstruct(archive, &p.polity, idx);
                if prior.chartered {
                    out.push((*cid, format!("second charter for {}; genesis is a singleton", p.polity)));
                    continue;
                }
                let members = did_set(&p.members);
                let ratifiers = did_set(&p.ratifiers);
                if p.ratifiers.is_empty() {
                    out.push((*cid, "charter has no ratifiers".into()));
                    continue;
                }
                if ratifiers.len() != p.ratifiers.len() {
                    out.push((*cid, "duplicate ratifier".into()));
                    continue;
                }
                if !ratifiers.is_subset(&members) {
                    out.push((*cid, "ratifier is not in the membership roll".into()));
                    continue;
                }
                let need = majority(p.members.len());
                if p.ratifiers.len() < need {
                    out.push((*cid, format!(
                        "ratification needs a majority of the roll (>= {need}), got {}",
                        p.ratifiers.len()
                    )));
                    continue;
                }
                if e.signer_dids() != ratifiers {
                    out.push((*cid, "charter signers must be exactly the ratifiers".into()));
                    continue;
                }
            }
            CharterEntry::CommitmentAmended(s) => {
                let a = &s.payload;
                let prior = reconstruct(archive, &a.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("commitment amendment for {} precedes its charter", a.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("commitment amendment for {} after its dissolution", a.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&a.deciders);
                if a.deciders.is_empty() {
                    out.push((*cid, "amendment has no deciders".into()));
                    continue;
                }
                if deciders.len() != a.deciders.len() {
                    out.push((*cid, "duplicate decider".into()));
                    continue;
                }
                if !deciders.is_subset(&members) {
                    out.push((*cid, "decider is not a member of the polity".into()));
                    continue;
                }
                if e.signer_dids() != deciders {
                    out.push((*cid, "amendment signers must be exactly the deciders".into()));
                    continue;
                }
                let m = prior.members.len();
                let key = &a.commitment.key;
                match a.kind {
                    AmendmentKind::Add => {
                        if prior.in_force.contains(key) {
                            out.push((*cid, format!("Add of an already in-force commitment '{key}'")));
                            continue;
                        }
                        let need = majority(m);
                        if a.deciders.len() < need {
                            out.push((*cid, format!("Add needs a majority (>= {need}), got {}", a.deciders.len())));
                            continue;
                        }
                    }
                    AmendmentKind::Remove => {
                        if !prior.in_force.contains(key) {
                            out.push((*cid, format!("Remove of an absent commitment '{key}'")));
                            continue;
                        }
                        if prior.founding.contains(key) {
                            out.push((*cid, format!(
                                "commitment '{key}' is a founding commitment and is genesis-immutable"
                            )));
                            continue;
                        }
                        let need = majority(m) + 1;
                        if a.deciders.len() < need {
                            out.push((*cid, format!(
                                "Remove needs more than a majority (>= {need}), got {}",
                                a.deciders.len()
                            )));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::MembershipChanged(s) => {
                let mc = &s.payload;
                let prior = reconstruct(archive, &mc.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("membership change for {} precedes its charter", mc.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("membership change for {} after its dissolution", mc.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&mc.deciders);
                let signers = e.signer_dids();
                let member = mc.member.as_str().to_string();
                let roll = prior.members.len();
                match mc.kind {
                    MembershipKind::Admit => {
                        if members.contains(&member) {
                            out.push((*cid, "Admit of an already-current member".into()));
                            continue;
                        }
                        if deciders.len() != mc.deciders.len() {
                            out.push((*cid, "duplicate decider".into()));
                            continue;
                        }
                        if deciders.contains(&member) {
                            out.push((*cid, "admitted member cannot also be a decider".into()));
                            continue;
                        }
                        if !deciders.is_subset(&members) {
                            out.push((*cid, "decider is not a member of the polity".into()));
                            continue;
                        }
                        let need = majority(roll);
                        if mc.deciders.len() < need {
                            out.push((*cid, format!("Admit needs a majority (>= {need}), got {}", mc.deciders.len())));
                            continue;
                        }
                        // Signers must be exactly the deciders PLUS the joiner: the
                        // joiner co-signs to consent (no conscription).
                        let mut required = deciders.clone();
                        required.insert(member.clone());
                        if signers != required {
                            out.push((*cid, "Admit signers must be exactly the deciders plus the joiner (consent)".into()));
                            continue;
                        }
                    }
                    MembershipKind::DepartVoluntary => {
                        if !members.contains(&member) {
                            out.push((*cid, "voluntary departure of a non-member".into()));
                            continue;
                        }
                        if !mc.deciders.is_empty() {
                            out.push((*cid, "voluntary departure records no deciders".into()));
                            continue;
                        }
                        // Self-signed by the departing member alone — the roll
                        // cannot force them to stay.
                        let required: BTreeSet<String> = std::iter::once(member.clone()).collect();
                        if signers != required {
                            out.push((*cid, "voluntary departure must be signed by exactly the departing member".into()));
                            continue;
                        }
                        if !can_self_govern(roll - 1) {
                            out.push((*cid, "departure would leave the roll unable to self-govern".into()));
                            continue;
                        }
                    }
                    MembershipKind::Expel => {
                        if !members.contains(&member) {
                            out.push((*cid, "expulsion of a non-member".into()));
                            continue;
                        }
                        if deciders.contains(&member) {
                            out.push((*cid, "expelled member cannot be among the deciders".into()));
                            continue;
                        }
                        if deciders.len() != mc.deciders.len() {
                            out.push((*cid, "duplicate decider".into()));
                            continue;
                        }
                        if !deciders.is_subset(&members) {
                            out.push((*cid, "decider is not a member of the polity".into()));
                            continue;
                        }
                        if signers != deciders {
                            out.push((*cid, "expulsion signers must be exactly the deciders".into()));
                            continue;
                        }
                        let need = majority(roll) + 1;
                        if mc.deciders.len() < need {
                            out.push((*cid, format!(
                                "Expel needs more than a majority (>= {need}), got {}",
                                mc.deciders.len()
                            )));
                            continue;
                        }
                        if !can_self_govern(roll - 1) {
                            out.push((*cid, "expulsion would leave the roll unable to self-govern".into()));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::OfficeChanged(s) => {
                let oc = &s.payload;
                let prior = reconstruct(archive, &oc.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("office change for {} precedes its charter", oc.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("office change for {} after its dissolution", oc.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&oc.deciders);
                let signers = e.signer_dids();
                let holder = oc.holder.as_str().to_string();
                let roll = prior.members.len();
                let is_member = prior.members.contains(&oc.holder);
                let raw_holds = prior.offices.iter().any(|(o, h)| o == &oc.office && h == &oc.holder);
                // A holding is effective only while its holder is on the roll.
                let holds = is_member && raw_holds;
                match oc.kind {
                    OfficeKind::Confer => {
                        if !is_member {
                            out.push((*cid, "office conferred on a non-member".into()));
                            continue;
                        }
                        if raw_holds {
                            out.push((*cid, "office already held by this member".into()));
                            continue;
                        }
                        if deciders.len() != oc.deciders.len() {
                            out.push((*cid, "duplicate decider".into()));
                            continue;
                        }
                        if !deciders.is_subset(&members) {
                            out.push((*cid, "decider is not a member of the polity".into()));
                            continue;
                        }
                        let need = majority(roll);
                        if oc.deciders.len() < need {
                            out.push((*cid, format!("conferral needs a majority (>= {need}), got {}", oc.deciders.len())));
                            continue;
                        }
                        // Signers must be exactly the deciders plus the holder (the
                        // holder co-signs to consent; overlap with the deciders is
                        // fine, since the holder is already a member).
                        let mut required = deciders.clone();
                        required.insert(holder.clone());
                        if signers != required {
                            out.push((*cid, "conferral signers must be exactly the deciders plus the holder (consent)".into()));
                            continue;
                        }
                    }
                    OfficeKind::Resign => {
                        if !holds {
                            out.push((*cid, "resignation of an office not held".into()));
                            continue;
                        }
                        if !oc.deciders.is_empty() {
                            out.push((*cid, "resignation records no deciders".into()));
                            continue;
                        }
                        let required: BTreeSet<String> = std::iter::once(holder.clone()).collect();
                        if signers != required {
                            out.push((*cid, "resignation must be signed by exactly the resigning holder".into()));
                            continue;
                        }
                    }
                    OfficeKind::Revoke => {
                        if !holds {
                            out.push((*cid, "revocation of an office not held".into()));
                            continue;
                        }
                        if deciders.contains(&holder) {
                            out.push((*cid, "revoked holder cannot be among the deciders".into()));
                            continue;
                        }
                        if deciders.len() != oc.deciders.len() {
                            out.push((*cid, "duplicate decider".into()));
                            continue;
                        }
                        if !deciders.is_subset(&members) {
                            out.push((*cid, "decider is not a member of the polity".into()));
                            continue;
                        }
                        if signers != deciders {
                            out.push((*cid, "revocation signers must be exactly the deciders".into()));
                            continue;
                        }
                        let need = majority(roll);
                        if oc.deciders.len() < need {
                            out.push((*cid, format!("revocation needs a majority (>= {need}), got {}", oc.deciders.len())));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::MemberAffirmed(s) => {
                let af = &s.payload;
                let prior = reconstruct(archive, &af.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("affirmation for {} precedes its charter", af.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("affirmation for {} after its dissolution", af.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let witnesses = did_set(&af.witnesses);
                let signers = e.signer_dids();
                let member = af.member.as_str().to_string();
                let roll = prior.members.len();
                if !members.contains(&member) {
                    out.push((*cid, "affirmation by a non-member".into()));
                    continue;
                }
                // Ratifying the charter is a founding member's induction.
                let is_inducted = prior.founding_members.contains(&af.member)
                    || prior.inducted.contains(&af.member);
                if witnesses.len() != af.witnesses.len() {
                    out.push((*cid, "duplicate witness".into()));
                    continue;
                }
                if !witnesses.is_subset(&members) {
                    out.push((*cid, "witness is not a member of the polity".into()));
                    continue;
                }
                match af.kind {
                    AffirmationKind::Induction => {
                        if is_inducted {
                            out.push((*cid, "induction of an already-inducted member".into()));
                            continue;
                        }
                        let need = majority(roll);
                        if af.witnesses.len() < need {
                            out.push((*cid, format!(
                                "induction needs a majority of the roll to witness (>= {need}), got {}",
                                af.witnesses.len()
                            )));
                            continue;
                        }
                        // The community receives them and they participate: signers
                        // are exactly the witnesses plus the inductee (overlap is
                        // fine — the inductee is already on the roll).
                        let mut required = witnesses.clone();
                        required.insert(member.clone());
                        if signers != required {
                            out.push((*cid, "induction signers must be exactly the witnesses plus the inductee".into()));
                            continue;
                        }
                    }
                    AffirmationKind::ReAffirmation => {
                        if !is_inducted {
                            out.push((*cid, "re-affirmation by a member who was never inducted".into()));
                            continue;
                        }
                        if !af.witnesses.is_empty() {
                            out.push((*cid, "re-affirmation records no witnesses".into()));
                            continue;
                        }
                        // Renewal is the member's own act.
                        let required: BTreeSet<String> = std::iter::once(member.clone()).collect();
                        if signers != required {
                            out.push((*cid, "re-affirmation must be signed by exactly the affirming member".into()));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::CharterDissolved(s) => {
                let d = &s.payload;
                let prior = reconstruct(archive, &d.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("dissolution of {} precedes its charter", d.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("{} is already dissolved", d.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&d.deciders);
                if d.deciders.is_empty() {
                    out.push((*cid, "dissolution has no deciders".into()));
                    continue;
                }
                if deciders.len() != d.deciders.len() {
                    out.push((*cid, "duplicate decider".into()));
                    continue;
                }
                if !deciders.is_subset(&members) {
                    out.push((*cid, "decider is not a member of the polity".into()));
                    continue;
                }
                if e.signer_dids() != deciders {
                    out.push((*cid, "dissolution signers must be exactly the deciders".into()));
                    continue;
                }
                let need = majority(prior.members.len());
                if d.deciders.len() < need {
                    out.push((*cid, format!(
                        "dissolution needs a majority of the roll (>= {need}), got {}",
                        d.deciders.len()
                    )));
                    continue;
                }
            }
        }
    }
    out
}

/// Ok if the archive has no authority violations, else the first as an error.
pub fn check_authority(archive: &Archive<CharterEntry>) -> KcResult<()> {
    match authority_violations(archive).into_iter().next() {
        Some((_cid, msg)) => Err(KcError::Authority(msg)),
        None => Ok(()),
    }
}
