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
//!   effective only while its holder is on the roll. In a **machine-majority**
//!   polity a conferral additionally refuses a *derived member with no guardian*
//!   (D15): the polity substantially is its guardian assembly, so every office must
//!   trace to a human.
//! - **The guardian edge** (D15) — which members are derived (machine) parties and
//!   the human each answers to. Founding derivations are ratified in the charter
//!   (and every named guardian co-signs it, since answerability is accepted rather
//!   than assigned); afterwards *Designate* needs a majority plus the named
//!   guardian's consent, *Rotate* a majority plus the **incoming** guardian's, and
//!   *Release* either the departing guardian's own signature or a majority of the
//!   roll recording that the human is gone. No guardian may itself be a derived
//!   member of the same polity — a machine cannot guard a machine.
//! - **Offices in use** (D14) — an office is not only held, it *bites*, in two
//!   different ways. The `Inductor` is a **presiding requirement**: where the
//!   office is effectively held, one of its holders other than the inductee must
//!   witness an induction, and the roll may proceed without them only at a
//!   majority **+ 1** — its own override, so no officer can block the polity's
//!   rite. The `EnvoyAuthorizer` is a **delegated authority**: one holder's
//!   signature stands in for the roll's majority on an envoy record, and because
//!   the majority route always remains open, filling the office costs the roll
//!   nothing and lets the officer stall nothing. `Convener` and `CharterKeeper`
//!   gate nothing — the roll stays sovereign over its own deliberations.
//! - **Envoy standing** — who may speak for the polity beyond it: *Authorize* on
//!   either route above **and** the envoy's co-signature (no conscription), on a
//!   current member whose public key derives their DID; *Resign* is signed by the
//!   envoy alone; *Revoke* takes the same two routes with the envoy excluded. A
//!   standing is effective only while the envoy is on the roll and unexpired.
//! - **Transmission** — admission puts someone on the roll; *induction* is the
//!   separate step by which they acquire the substrate, and it is **witnessed by
//!   the roll**: a majority of current members co-sign alongside the inductee (the
//!   community receives them, and they participate). Founding members are inducted
//!   by ratifying the charter, so they take no induction record and cannot be
//!   inducted again. *Re-affirmation* is self-signed by an already-inducted member
//!   — renewal is the member's own act, and its freshness decays
//!   ([`crate::AffirmationState::stale_members`]).
//! - **Terminal dissolution** — once a polity is dissolved, no further charter
//!   event (amendment, membership change, office change, guardian change, envoy
//!   authorization, affirmation, or a second dissolution) is authorized.
//!
//! Like kith-covenant's constitution rules, state is reconstructed by walking
//! entries *before* the one under test in insertion order; an unauthorized prior
//! amendment still shifts the walked state, so accept and reject cases must be
//! tested in separate archives.

use std::collections::BTreeSet;

use en_archive::Archive;
use en_core::{Cid, DidKey, KcError, KcResult, Timestamp};

use crate::entry::CharterEntry;
use crate::state::is_prunable;
use crate::record::{
    pruning_digest, AffirmationKind, AmendmentKind, BoundarySource, DissentKind, EnvoyKind,
    GuardianKind, MembershipKind, Office, OfficeKind,
};

/// How far back an effective `CharterKeeper` must reach before its delegated power
/// over observation-pruning applies: **30 days**.
///
/// This is a constant, not a tunable threshold, and the difference is deliberate.
/// Cadences in this tier are query-time parameters — `stale_members` takes one,
/// `Thresholds` carries the rest — because they are policy. This one is *law*: it
/// exists to keep one officer from reaching evidence that grades a live Concern,
/// and a floor the party it constrains could lower is not a floor. The roll can
/// still prune anything more recent; it simply has to do so as the roll, at a
/// majority, rather than through a single custodian.
///
/// Thirty days is thirty times the longest observation-based measure
/// (`sustained_absence_seconds`, one day), so the officer route cannot touch
/// anything a current grading rests on.
pub const PRUNE_SETTLING_SECONDS: i64 = 30 * 86_400;

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
    /// Derived-party designations (founding derivations + `GuardianChange`s), as
    /// `(member, guardian)`. Not yet intersected with the live roll; a designation
    /// lapses while its member is off the roll, exactly as an office holding does.
    derivations: Vec<(DidKey, Option<DidKey>)>,
    /// Raw envoy standings (`Authorize` − `Resign`/`Revoke`), not yet intersected
    /// with the live roll. Expiry is a derivation-time concern
    /// ([`crate::envoy_state`]), not an authority one: a standing that has lapsed
    /// of its own accord was still validly granted.
    envoys: Vec<DidKey>,
    /// Everyone the founding boundary named as excluded, set once at genesis. The
    /// dissent rules read the FOUNDING accounting (roll + exclusions), never the
    /// live roll: a member admitted years later was still not part of the boundary
    /// a dissenter is contesting.
    founding_exclusions: BTreeSet<String>,
    /// The most recent observation-pruning horizon for this polity, if any. A
    /// pruning must advance it: a horizon that recedes claims to drop what an
    /// earlier one already dropped, so it could only ever mislead.
    prune_horizon: Option<Timestamp>,
    /// The substrate anchor the founding chartered, set once at genesis and never
    /// mutated — the substrate has no amendment path (design **D23**). This is who
    /// a disavowal must be: the ground is *in* the record, unlike an omitted party,
    /// so unlike a dissent the filer is checkable.
    anchor: Option<DidKey>,
    /// Every reader that signed a reading behind the founding boundary, set once at
    /// genesis (design **D24**). An independence attestation must name one of these:
    /// like the anchor, a reader is *in* the record, so the subject of the claim is
    /// checkable even though the claim itself never can be.
    founding_readers: BTreeSet<String>,
}

/// Reconstruct a polity's charter state from entries strictly before `upto`.
fn reconstruct(archive: &Archive<CharterEntry>, polity: &DidKey, upto: usize) -> Reconstructed {
    let mut r = Reconstructed {
        chartered: false,
        dissolved: false,
        prune_horizon: None,
        members: Vec::new(),
        founding: BTreeSet::new(),
        in_force: BTreeSet::new(),
        offices: Vec::new(),
        founding_members: Vec::new(),
        inducted: Vec::new(),
        derivations: Vec::new(),
        envoys: Vec::new(),
        founding_exclusions: BTreeSet::new(),
        anchor: None,
        founding_readers: BTreeSet::new(),
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
                for d in &s.payload.derivations {
                    r.derivations.push((d.member.clone(), d.guardian.clone()));
                }
                for x in &s.payload.attestation.exclusions {
                    r.founding_exclusions.insert(x.member.as_str().to_string());
                }
                r.anchor = s.payload.substrate.anchor.clone();
                for rd in s.payload.attestation.source.readings() {
                    r.founding_readers.insert(rd.payload.reader.as_str().to_string());
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
            CharterEntry::GuardianChanged(s) if &s.payload.polity == polity => {
                let gc = &s.payload;
                match gc.kind {
                    GuardianKind::Designate => {
                        if !r.derivations.iter().any(|(m, _)| m == &gc.member) {
                            r.derivations.push((gc.member.clone(), gc.guardian.clone()));
                        }
                    }
                    GuardianKind::Rotate => {
                        for (m, g) in r.derivations.iter_mut() {
                            if m == &gc.member {
                                *g = gc.guardian.clone();
                            }
                        }
                    }
                    GuardianKind::Release => {
                        // The designation survives; only the edge is cleared. A
                        // machine does not become human by losing its human.
                        for (m, g) in r.derivations.iter_mut() {
                            if m == &gc.member {
                                *g = None;
                            }
                        }
                    }
                }
            }
            CharterEntry::EnvoyAuthorized(s) if &s.payload.polity == polity => {
                let ea = &s.payload;
                match ea.kind {
                    EnvoyKind::Authorize => {
                        if !r.envoys.contains(&ea.envoy) {
                            r.envoys.push(ea.envoy.clone());
                        }
                    }
                    EnvoyKind::Resign | EnvoyKind::Revoke => {
                        r.envoys.retain(|d| d != &ea.envoy);
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
            // Neither an observation nor a dissent changes any state: each
            // records what one party says, and the tier never adjudicates it.
            // None of these three changes reconstructed state. An observation
            // records what someone saw; a dissent is an excluded party's assertion;
            // and a disavowal is the ground's, which cannot rewrite a
            // genesis-immutable substrate — a withdrawal that voided a charter would
            // hand any anchor a unilateral kill switch over a polity, which is a
            // worse hole than the one D23 closed.
            // None of these four changes reconstructed state. They are **evidence
            // about the founding** rather than governing acts: an observation records
            // what someone saw; a dissent is an excluded party's assertion; a
            // disavowal is the ground's, which cannot rewrite a genesis-immutable
            // substrate; and an independence attestation is a third party's account of
            // a reader, counted by a lens and enforced by no rule.
            CharterEntry::Observed(_)
            | CharterEntry::BoundaryDissented(_)
            | CharterEntry::SubstrateDisavowed(_)
            | CharterEntry::ReaderIndependenceAttested(_) => {}
            CharterEntry::ObservationsPruned(s) if &s.payload.polity == polity => {
                let h = s.payload.horizon;
                if r.prune_horizon.is_none_or(|prev| h > prev) {
                    r.prune_horizon = Some(h);
                }
            }
            _ => {}
        }
    }
    r
}

/// Who *effectively* holds `office` as of the reconstructed point: a raw grant
/// counts only while its holder is on the roll (D12 — a holding lapses when its
/// holder departs and revives when they return). Both use-time rules (D14) read
/// the holdings through this, never the raw grants, so a departed officer neither
/// presides nor authorizes.
fn effective_holders(prior: &Reconstructed, office: &Office) -> BTreeSet<String> {
    prior
        .offices
        .iter()
        .filter(|(o, h)| o == office && prior.members.contains(h))
        .map(|(_, h)| h.as_str().to_string())
        .collect()
}

/// The designations in force as of the reconstructed point: a designation counts
/// only while its member is on the roll (the same lapse-and-revive rule offices
/// follow — see [`effective_holders`]).
fn effective_derivations(prior: &Reconstructed) -> Vec<(&DidKey, &Option<DidKey>)> {
    prior
        .derivations
        .iter()
        .filter(|(m, _)| prior.members.contains(m))
        .map(|(m, g)| (m, g))
        .collect()
}

/// Whether the polity is **machine-majority** at the reconstructed point: derived
/// members are at least a majority of the live roll. This is what conditions the
/// office rule (D15) — a wholly or mostly human polity is left alone.
fn is_machine_majority(prior: &Reconstructed) -> bool {
    let roll = prior.members.len();
    roll > 0 && effective_derivations(prior).len() >= majority(roll)
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
                // D21 — the boundary must ACCOUNT for itself (resolving Q1).
                //
                // Nothing here refuses a *narrow* founding: a genuinely separate
                // group of three inside a thirty-node cluster is indistinguishable
                // from a faction capturing it, without a substantive judgement about
                // the tradition that no rule can make. What is refused is the
                // SILENCE — before this, the quorum's denominator was the proposal's
                // own roll, so one node could charter a thirty-node cluster as a
                // polity of itself and the other twenty-nine went unrecorded.
                let att = &p.attestation;
                if !att.accounts_for(p.members.len()) {
                    out.push((*cid, format!(
                        "boundary does not account for itself: it showed {} but the roll \
                         names {} and only {} are excluded",
                        att.candidate_size,
                        p.members.len(),
                        att.exclusions.len()
                    )));
                    continue;
                }
                let mut excluded: BTreeSet<String> = BTreeSet::new();
                let mut bad_exclusion = false;
                for e in &att.exclusions {
                    if !excluded.insert(e.member.as_str().to_string()) {
                        out.push((*cid, format!("duplicate exclusion for {}", e.member)));
                        bad_exclusion = true;
                        break;
                    }
                    if members.contains(e.member.as_str()) {
                        out.push((*cid, format!(
                            "{} is excluded and also on the roll", e.member
                        )));
                        bad_exclusion = true;
                        break;
                    }
                }
                if bad_exclusion {
                    continue;
                }
                // D22 — the boundary must be ATTRIBUTABLE, not merely claimed
                // (resolving Q5).
                //
                // D21 could check only that the numbers added up, and it checked
                // them against the proposer's own report — so any internally
                // consistent fabrication passed, and flipping one field promoted a
                // bare assertion to a "reading" nobody took. Here the `Substrate`
                // arm carries its evidence: each reading is signed by the reader it
                // names, bound to this founding, and must actually show the boundary
                // that was ratified.
                //
                // As everywhere in this tier, none of this refuses a NARROW
                // founding, and none of it can tell an honest reader from a captured
                // one. What it removes is anonymity: a gerrymandered boundary is now
                // attributable to a named key, permanently.
                let mut bad_reading = false;
                if let BoundarySource::Substrate { readings } = &att.source {
                    if readings.is_empty() {
                        out.push((
                            *cid,
                            "boundary claims a substrate reading but carries none".to_string(),
                        ));
                        continue;
                    }
                    for r in readings {
                        let rd = &r.payload;
                        // Nobody may put a reading in somebody else's mouth — the
                        // same rule D16 applies to an observation.
                        let signers = r
                            .signatures
                            .iter()
                            .map(|sg| sg.signer_did.clone())
                            .collect::<BTreeSet<String>>();
                        let sole: BTreeSet<String> =
                            std::iter::once(rd.reader.as_str().to_string()).collect();
                        if signers != sole {
                            out.push((*cid, format!(
                                "reading names {} as its reader but was not signed by it alone",
                                rd.reader
                            )));
                            bad_reading = true;
                            break;
                        }
                        // Bound to THIS founding. Without it, an honest reader's
                        // account of one cluster replays into somebody else's
                        // founding and the signature still verifies.
                        if rd.polity != p.polity {
                            out.push((*cid, format!(
                                "reading by {} is for polity {}, not {}",
                                rd.reader, rd.polity, p.polity
                            )));
                            bad_reading = true;
                            break;
                        }
                        // The count stops being the proposer's word.
                        if rd.members.len() != att.candidate_size {
                            out.push((*cid, format!(
                                "reading by {} shows {} members but the boundary claims {}",
                                rd.reader,
                                rd.members.len(),
                                att.candidate_size
                            )));
                            bad_reading = true;
                            break;
                        }
                    }
                    if bad_reading {
                        continue;
                    }
                    // The teeth: at least one signed reading must show the roll plus
                    // the named exclusions EXACTLY — identity, where D21 had only
                    // arithmetic. Readers may disagree with each other and with the
                    // founding; what may not happen is a founding no reader supports.
                    if !readings.iter().any(|r| att.corroborated_by(&p.members, &r.payload)) {
                        out.push((
                            *cid,
                            "no signed reading shows the boundary this charter ratifies"
                                .to_string(),
                        ));
                        continue;
                    }
                }
                // D23 — the polity must stand on something (resolving Q4).
                //
                // Non-ultimacy is the tier's whole reason for a substrate, and until
                // D23 nothing checked it: `anchor` was read by no rule anywhere, and
                // measured, a charter whose entire ground was two empty strings passed
                // both verification passes. A polity with nothing above itself is
                // exactly the failure kith-covenant's non-ultimacy feature guards
                // against, so it may not be the thing this tier hands upward.
                //
                // ARTICULATED, and only that. Presence is checkable; content is not.
                // No rule can read a tradition and this one does not pretend to — what
                // it refuses is the blank, which is not a tradition anyone could have
                // been mistaken about.
                if p.substrate.name.trim().is_empty() || p.substrate.statement.trim().is_empty() {
                    out.push((
                        *cid,
                        "substrate is not articulated: it must carry both a name and a statement"
                            .into(),
                    ));
                    continue;
                }
                // NON-CLOSURE. Q4 asks whether a group can charter with no ground; the
                // sharper hole is that it can charter with itself as its ground. A
                // presence-only rule is defeated by naming your own DID, so the anchor
                // may be neither the polity nor anyone on its roll. This is the
                // guardian loop rule below, one level out: a ground inside what it
                // grounds is no ground.
                if let Some(a) = &p.substrate.anchor {
                    if a == &p.polity {
                        out.push((
                            *cid,
                            "substrate anchor is the polity itself; a polity cannot ground itself"
                                .into(),
                        ));
                        continue;
                    }
                    if members.contains(a.as_str()) {
                        out.push((*cid, format!(
                            "substrate anchor {a} is on the polity's own roll; a ground inside \
                             what it grounds is no ground"
                        )));
                        continue;
                    }
                }
                // GROUNDED. The keyed anchor is demanded exactly where nobody could
                // hold an unkeyed one: every member a machine, and not one of them
                // answering to a human. A human member holds the statement; a guarded
                // machine already has a consenting human above it, which is D15's model
                // of legitimate machine participation. Neither present, and the
                // articulated statement is a sentence the polity wrote about itself.
                //
                // Counted over the *distinct derivations that are actually on the roll*,
                // so the rule cannot be dodged by padding `derivations` with names the
                // roll never had — the checks that refuse such padding run below this
                // one, and a rule that relied on their order would be load-bearing by
                // accident.
                let derived_on_roll: BTreeSet<&str> = p
                    .derivations
                    .iter()
                    .map(|d| d.member.as_str())
                    .filter(|m| members.contains(*m))
                    .collect();
                let all_derived = derived_on_roll.len() == members.len();
                let none_guarded = !p.derivations.iter().any(|d| d.guardian.is_some());
                if all_derived && none_guarded && p.substrate.anchor.is_none() {
                    out.push((*cid, format!(
                        "a roll of {} machines answering to nobody must name a substrate anchor: \
                         nobody here can hold a tradition",
                        p.members.len()
                    )));
                    continue;
                }
                // D15 — the founding derivations. Every designated member is on the
                // roll, named once, and no guardian is itself a derived member of
                // this same polity: a machine cannot guard a machine, or the
                // human-above-machine anchor closes into a loop.
                let mut designated: BTreeSet<String> = BTreeSet::new();
                let mut bad_derivation = false;
                for d in &p.derivations {
                    if !designated.insert(d.member.as_str().to_string()) {
                        out.push((*cid, format!("duplicate derivation for {}", d.member)));
                        bad_derivation = true;
                        break;
                    }
                    if !members.contains(d.member.as_str()) {
                        out.push((*cid, format!(
                            "derivation names {}, who is not in the membership roll", d.member
                        )));
                        bad_derivation = true;
                        break;
                    }
                }
                if bad_derivation {
                    continue;
                }
                if let Some(g) = p
                    .derivations
                    .iter()
                    .filter_map(|d| d.guardian.as_ref())
                    .find(|g| designated.contains(g.as_str()))
                {
                    out.push((*cid, format!(
                        "guardian {g} is itself a derived member; a machine cannot guard a machine"
                    )));
                    continue;
                }
                // Answerability is accepted, never assigned — so a named guardian
                // co-signs the founding alongside the ratifiers, exactly as a joiner
                // co-signs their own admission.
                //
                // And since **D23**, so does a named substrate anchor. The same
                // principle reaches the ground: you may not conscript what you claim to
                // stand under. Without this the strongest available grounding — a keyed
                // party outside the roll — would be a name the founders could write
                // down about somebody who never agreed to it, which is the anonymity
                // D22 spent a whole slice removing from the boundary.
                let mut required = ratifiers.clone();
                for d in &p.derivations {
                    if let Some(g) = &d.guardian {
                        required.insert(g.as_str().to_string());
                    }
                }
                if let Some(a) = &p.substrate.anchor {
                    required.insert(a.as_str().to_string());
                }
                if e.signer_dids() != required {
                    out.push((*cid, "charter signers must be exactly the ratifiers plus every named guardian and the substrate anchor (consent)".into()));
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
                        // D15 — in a machine-majority polity every office must trace
                        // to a human: the polity substantially *is* its guardian
                        // assembly, so a machine with nobody behind it may hold no
                        // office. Conditional on machine-majority *at this point in
                        // the log*; a mostly human polity is left alone.
                        if is_machine_majority(&prior)
                            && effective_derivations(&prior)
                                .iter()
                                .any(|(m, g)| *m == &oc.holder && g.is_none())
                        {
                            out.push((*cid, format!(
                                "office conferred on {}, a derived member with no guardian, in a \
                                 machine-majority polity",
                                oc.holder
                            )));
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
            CharterEntry::GuardianChanged(s) => {
                let gc = &s.payload;
                let prior = reconstruct(archive, &gc.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "guardian change for {} precedes its charter", gc.polity
                    )));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!(
                        "guardian change for {} after its dissolution", gc.polity
                    )));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&gc.deciders);
                let signers = e.signer_dids();
                let member = gc.member.as_str().to_string();
                let roll = prior.members.len();
                let need = majority(roll);
                let designated: Vec<(&DidKey, &Option<DidKey>)> = effective_derivations(&prior);
                let current = designated.iter().find(|(m, _)| *m == &gc.member).map(|(_, g)| *g);
                if !members.contains(&member) {
                    out.push((*cid, "guardian change for a non-member".into()));
                    continue;
                }
                if deciders.len() != gc.deciders.len() {
                    out.push((*cid, "duplicate decider".into()));
                    continue;
                }
                if !deciders.is_subset(&members) {
                    out.push((*cid, "decider is not a member of the polity".into()));
                    continue;
                }
                // A machine cannot guard a machine (the same rule the founding
                // applies), and nothing may be its own guardian.
                if let Some(g) = &gc.guardian {
                    if g == &gc.member {
                        out.push((*cid, "a member cannot be its own guardian".into()));
                        continue;
                    }
                    if designated.iter().any(|(m, _)| *m == g) {
                        out.push((*cid, format!(
                            "guardian {g} is itself a derived member; a machine cannot guard a machine"
                        )));
                        continue;
                    }
                }
                match gc.kind {
                    GuardianKind::Designate => {
                        if current.is_some() {
                            out.push((*cid, "member is already designated a derived party".into()));
                            continue;
                        }
                        if gc.deciders.len() < need {
                            out.push((*cid, format!(
                                "designation needs a majority of the roll (>= {need}), got {}",
                                gc.deciders.len()
                            )));
                            continue;
                        }
                        // Signers = the deciders plus the named guardian, who accepts
                        // the answerability. A designation with no guardian (a machine
                        // with nobody behind it) is the roll's own act alone.
                        let mut required = deciders.clone();
                        if let Some(g) = &gc.guardian {
                            required.insert(g.as_str().to_string());
                        }
                        if signers != required {
                            out.push((*cid, "designation signers must be exactly the deciders plus the named guardian (consent)".into()));
                            continue;
                        }
                    }
                    GuardianKind::Rotate => {
                        if current.is_none() {
                            out.push((*cid, "rotation for a member who is not a designated derived party".into()));
                            continue;
                        }
                        let Some(incoming) = &gc.guardian else {
                            out.push((*cid, "rotation must name the incoming guardian (use Release to clear the edge)".into()));
                            continue;
                        };
                        if current.map(|g| g.as_ref()) == Some(Some(incoming)) {
                            out.push((*cid, "rotation to the guardian already in place".into()));
                            continue;
                        }
                        if gc.deciders.len() < need {
                            out.push((*cid, format!(
                                "rotation needs a majority of the roll (>= {need}), got {}",
                                gc.deciders.len()
                            )));
                            continue;
                        }
                        // The *incoming* guardian consents; the outgoing one is
                        // released rather than held (nobody is forced to keep serving).
                        let mut required = deciders.clone();
                        required.insert(incoming.as_str().to_string());
                        if signers != required {
                            out.push((*cid, "rotation signers must be exactly the deciders plus the incoming guardian (consent)".into()));
                            continue;
                        }
                    }
                    GuardianKind::Release => {
                        let Some(outgoing) = current else {
                            out.push((*cid, "release for a member who is not a designated derived party".into()));
                            continue;
                        };
                        if gc.guardian.is_some() {
                            out.push((*cid, "a release names no incoming guardian".into()));
                            continue;
                        }
                        let Some(outgoing) = outgoing else {
                            out.push((*cid, "release of a guardian edge that is already absent".into()));
                            continue;
                        };
                        // Two routes: the guardian steps away on their own signature,
                        // or the roll records at a majority that the human is gone.
                        let by_guardian: BTreeSet<String> =
                            std::iter::once(outgoing.as_str().to_string()).collect();
                        if gc.deciders.is_empty() {
                            if signers != by_guardian {
                                out.push((*cid, "a release with no deciders must be signed by exactly the departing guardian".into()));
                                continue;
                            }
                        } else {
                            if signers != deciders {
                                out.push((*cid, "release signers must be exactly the deciders".into()));
                                continue;
                            }
                            if gc.deciders.len() < need {
                                out.push((*cid, format!(
                                    "a release the roll records needs a majority (>= {need}), got {}",
                                    gc.deciders.len()
                                )));
                                continue;
                            }
                        }
                    }
                }
            }
            CharterEntry::EnvoyAuthorized(s) => {
                let ea = &s.payload;
                let prior = reconstruct(archive, &ea.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "envoy authorization for {} precedes its charter", ea.polity
                    )));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!(
                        "envoy authorization for {} after its dissolution", ea.polity
                    )));
                    continue;
                }
                // The record names the speaker by key as well as by DID, because the
                // KC-side delegation does; a key that does not derive the DID would
                // let an unrelated key speak for the polity.
                if DidKey::from_pubkey(&ea.envoy_pubkey).ok().as_ref() != Some(&ea.envoy) {
                    out.push((*cid, "envoy_pubkey does not derive the envoy DID".into()));
                    continue;
                }
                let members = did_set(&prior.members);
                let authorizers = did_set(&ea.authorizers);
                let signers = e.signer_dids();
                let envoy = ea.envoy.as_str().to_string();
                let roll = prior.members.len();
                let is_member = prior.members.contains(&ea.envoy);
                let raw_standing = prior.envoys.contains(&ea.envoy);
                // A standing is effective only while the envoy is on the roll.
                let stands = is_member && raw_standing;
                // D14 — the EnvoyAuthorizer's *delegated power*: one holder's
                // signature stands in for the roll's majority. Never the envoy's
                // own, so an authorizer cannot appoint themselves unilaterally —
                // self-appointment must go through the roll. The majority route
                // always stays open, so filling the office costs the roll nothing
                // and the officer can never stall an authorization.
                let officers = effective_holders(&prior, &Office::EnvoyAuthorizer);
                let by_officer = authorizers.iter().any(|a| a != &envoy && officers.contains(a));
                let need = majority(roll);
                match ea.kind {
                    EnvoyKind::Authorize => {
                        if !is_member {
                            out.push((*cid, "envoy authorized who is not a member of the polity".into()));
                            continue;
                        }
                        if raw_standing {
                            out.push((*cid, "envoy standing already authorized".into()));
                            continue;
                        }
                        if authorizers.len() != ea.authorizers.len() {
                            out.push((*cid, "duplicate authorizer".into()));
                            continue;
                        }
                        if !authorizers.is_subset(&members) {
                            out.push((*cid, "authorizer is not a member of the polity".into()));
                            continue;
                        }
                        if !by_officer && ea.authorizers.len() < need {
                            out.push((*cid, format!(
                                "authorization needs an EnvoyAuthorizer other than the envoy among \
                                 the authorizers, or a majority of the roll (>= {need}), got {}",
                                ea.authorizers.len()
                            )));
                            continue;
                        }
                        // Signers are exactly the authorizers plus the envoy: speaking
                        // for the polity is a duty, so it is never conscripted.
                        let mut required = authorizers.clone();
                        required.insert(envoy.clone());
                        if signers != required {
                            out.push((*cid, "authorization signers must be exactly the authorizers plus the envoy (consent)".into()));
                            continue;
                        }
                    }
                    EnvoyKind::Resign => {
                        if !stands {
                            out.push((*cid, "resignation of an envoy standing not held".into()));
                            continue;
                        }
                        if !ea.authorizers.is_empty() {
                            out.push((*cid, "envoy resignation records no authorizers".into()));
                            continue;
                        }
                        let required: BTreeSet<String> = std::iter::once(envoy.clone()).collect();
                        if signers != required {
                            out.push((*cid, "envoy resignation must be signed by exactly the resigning envoy".into()));
                            continue;
                        }
                    }
                    EnvoyKind::Revoke => {
                        if !stands {
                            out.push((*cid, "revocation of an envoy standing not held".into()));
                            continue;
                        }
                        if authorizers.contains(&envoy) {
                            out.push((*cid, "revoked envoy cannot be among the authorizers".into()));
                            continue;
                        }
                        if authorizers.len() != ea.authorizers.len() {
                            out.push((*cid, "duplicate authorizer".into()));
                            continue;
                        }
                        if !authorizers.is_subset(&members) {
                            out.push((*cid, "authorizer is not a member of the polity".into()));
                            continue;
                        }
                        if signers != authorizers {
                            out.push((*cid, "envoy revocation signers must be exactly the authorizers".into()));
                            continue;
                        }
                        if !by_officer && ea.authorizers.len() < need {
                            out.push((*cid, format!(
                                "envoy revocation needs an EnvoyAuthorizer among the authorizers, \
                                 or a majority of the roll (>= {need}), got {}",
                                ea.authorizers.len()
                            )));
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
                        // D14 — the Inductor presides, *defeasibly*. Where the
                        // office is effectively held, one of its holders (never the
                        // inductee: nobody presides over their own reception) must
                        // be among the witnesses. The roll may still induct without
                        // them, but only at a majority + 1 — its own override, on
                        // D11's asymmetric-bar pattern — so an officer can raise the
                        // bar for the rite and never block it. A vacant office
                        // leaves the plain-majority rule untouched.
                        let inductors = effective_holders(&prior, &Office::Inductor);
                        if !inductors.is_empty()
                            && !witnesses.iter().any(|w| w != &member && inductors.contains(w))
                        {
                            let over = need + 1;
                            if af.witnesses.len() < over {
                                out.push((*cid, format!(
                                    "induction needs an Inductor other than the inductee among the \
                                     witnesses, or the roll's override of >= {over} witnesses, got {}",
                                    af.witnesses.len()
                                )));
                                continue;
                            }
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
            CharterEntry::Observed(s) => {
                let ob = &s.payload;
                let prior = reconstruct(archive, &ob.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("observation of {} precedes its charter", ob.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("observation of {} after its dissolution", ob.polity)));
                    continue;
                }
                // An observation is evidence, not a decision: no deciders, no
                // majority, and the observer need not be a member — a monitor may
                // run outside the roll, and one that must be inside it could be
                // silenced by expulsion. What is enforced is only that nobody may
                // put an observation in somebody else's mouth.
                let signers = e.signer_dids();
                let required: BTreeSet<String> =
                    std::iter::once(ob.observer.as_str().to_string()).collect();
                if signers != required {
                    out.push((*cid, "an observation must be signed by exactly its observer".into()));
                    continue;
                }
                let mut seen: BTreeSet<&str> = BTreeSet::new();
                if let Some(dup) = ob.readings.iter().find(|r| !seen.insert(r.member.as_str())) {
                    out.push((*cid, format!("duplicate reading for {} in one observation", dup.member)));
                    continue;
                }
                // An attendance stamp cannot postdate the look that reported it: a
                // monitor claiming an attestation from after the moment it observed
                // is claiming to have seen the future. Beyond being incoherent, this
                // is what makes the pruning clamp sound (D17) — a dropped
                // observation can only ever have carried stamps from before the
                // horizon, so dropping it can never lengthen a measured absence.
                if let Some(ahead) = ob
                    .readings
                    .iter()
                    .find(|r| r.attested_at.is_some_and(|s| s > ob.observed_at))
                {
                    out.push((*cid, format!(
                        "attendance stamp for {} postdates the observation that reports it",
                        ahead.member
                    )));
                    continue;
                }
            }
            CharterEntry::ObservationsPruned(s) => {
                let pr = &s.payload;
                let prior = reconstruct(archive, &pr.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("pruning of {} precedes its charter", pr.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("pruning of {} after its dissolution", pr.polity)));
                    continue;
                }
                // A horizon must advance. A receding one claims to drop what an
                // earlier pruning already dropped, so it can only mislead a reader
                // about how far back the log's evidence actually goes.
                if let Some(prev) = prior.prune_horizon {
                    if pr.horizon <= prev {
                        out.push((*cid, format!(
                            "pruning horizon must advance past the previous one ({prev})"
                        )));
                        continue;
                    }
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&pr.deciders);
                let signers = e.signer_dids();
                if deciders.len() != pr.deciders.len() {
                    out.push((*cid, "duplicate decider".into()));
                    continue;
                }
                if !deciders.is_subset(&members) {
                    out.push((*cid, "decider is not a member of the polity".into()));
                    continue;
                }
                if signers != deciders {
                    out.push((*cid, "a pruning must be signed by exactly its deciders".into()));
                    continue;
                }
                // D17 — the CharterKeeper's *delegated power*, the teeth that office
                // had none of: one holder's signature stands in for the roll's
                // majority, but only beyond the settling horizon. Destroying
                // evidence is precisely the move a captured party makes, so the
                // recent record — the part a live grading rests on — stays out of
                // any single custodian's reach. The majority route is always open,
                // so the office can stall nothing.
                let keepers = effective_holders(&prior, &Office::CharterKeeper);
                let settled =
                    pr.horizon.as_second() <= pr.pruned_at.as_second() - PRUNE_SETTLING_SECONDS;
                let by_officer = settled && deciders.iter().any(|d| keepers.contains(d));
                let need = majority(prior.members.len());
                if !by_officer && pr.deciders.len() < need {
                    out.push((*cid, format!(
                        "pruning needs a CharterKeeper among the deciders and a horizon at least \
                         {PRUNE_SETTLING_SECONDS}s old, or a majority of the roll (>= {need}), got {}",
                        pr.deciders.len()
                    )));
                    continue;
                }
                // The receipt, checked *conditionally*. While the observations are
                // still here, the record must say truthfully what it drops. Once the
                // pruning has been applied they are gone, and a rule that recomputed
                // the digest regardless would then find nothing and report every
                // historical pruning as a violation. So: verify what is still
                // verifiable, and treat an already-applied pruning as beyond this
                // log's power to check rather than as a lie. The receipt keeps its
                // force for anyone holding an earlier copy.
                let dropped: Vec<Cid> = archive
                    .iter()
                    .take(idx)
                    .filter(|(_, entry)| is_prunable(entry, &pr.polity, pr.horizon))
                    .map(|(c, _)| *c)
                    .collect();
                if !dropped.is_empty() {
                    if dropped.len() != pr.dropped {
                        out.push((*cid, format!(
                            "pruning claims {} dropped, but {} observations precede its horizon",
                            pr.dropped,
                            dropped.len()
                        )));
                        continue;
                    }
                    match pruning_digest(&dropped) {
                        Ok(d) if d == pr.digest => {}
                        Ok(_) => {
                            out.push((*cid, "pruning digest does not match what it drops".into()));
                            continue;
                        }
                        Err(err) => {
                            out.push((*cid, format!("pruning digest: {err}")));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::BoundaryDissented(s) => {
                let d = &s.payload;
                let prior = reconstruct(archive, &d.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "dissent from {}'s founding precedes it", d.polity
                    )));
                    continue;
                }
                // DELIBERATELY no dissolution guard — the only entry class without
                // one (design **D22**). Every other act governs a polity that no
                // longer exists; this one contests a founding that did. Blocking it
                // would let founders silence the parties they excluded by winding
                // the polity up, which is D17's "destroying evidence is the move a
                // captured party makes" wearing a different costume. The founding's
                // legitimacy outlives the polity in any case: kith-covenant holds a
                // CommunityIdentity derived from that charter.
                //
                // Like an observation, a dissent is evidence and not a decision: no
                // deciders, no majority, and the dissenter need not be a member —
                // an excluded party required to sit inside the roll could be
                // silenced by simply never being admitted.
                let signers = e.signer_dids();
                let sole: BTreeSet<String> =
                    std::iter::once(d.dissenter.as_str().to_string()).collect();
                if signers != sole {
                    out.push((
                        *cid,
                        "a dissent must be signed by exactly its dissenter".to_string(),
                    ));
                    continue;
                }
                // What the tier CAN check is that the dissent is coherent with the
                // record it contests — never whether it is true.
                let who = d.dissenter.as_str();
                let on_roll = prior.founding_members.iter().any(|m| m.as_str() == who);
                let excluded = prior.founding_exclusions.contains(who);
                match d.kind {
                    // "I was excluded and I reject the reason." You may reject the
                    // reason given for you, not one given for somebody else.
                    DissentKind::ReasonRejected if !excluded => {
                        out.push((*cid, format!(
                            "{} rejects an exclusion reason but the founding excludes no such member",
                            d.dissenter
                        )));
                        continue;
                    }
                    // "The accounting never mentioned me." A record that does
                    // mention them contradicts the claim outright.
                    DissentKind::Unnamed if on_roll || excluded => {
                        out.push((*cid, format!(
                            "{} claims the founding accounting omitted them, but it names them",
                            d.dissenter
                        )));
                        continue;
                    }
                    _ => {}
                }
            }
            CharterEntry::SubstrateDisavowed(s) => {
                let d = &s.payload;
                let prior = reconstruct(archive, &d.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "disavowal of {}'s substrate precedes its founding", d.polity
                    )));
                    continue;
                }
                // Like a dissent, DELIBERATELY no dissolution guard — the second
                // entry class without one, for the same reason in a new costume: a
                // polity that could silence its ground by winding up would make the
                // ground's consent worth exactly as much as the founders' patience.
                //
                // Nobody may put a withdrawal in the ground's mouth.
                let signers = e.signer_dids();
                let sole: BTreeSet<String> =
                    std::iter::once(d.disavower.as_str().to_string()).collect();
                if signers != sole {
                    out.push((
                        *cid,
                        "a disavowal must be signed by exactly its disavower".to_string(),
                    ));
                    continue;
                }
                // And here this record is STRICTER than a dissent, because it can
                // afford to be. A dissent must admit any filer: an omitted party is
                // by definition absent from the record, so requiring them to appear
                // in it would be the silence D22 removed. A ground is *named on the
                // charter*, so only that key can withdraw — anyone else's withdrawal
                // is not a contested claim, it is noise about somebody else's
                // relationship.
                match &prior.anchor {
                    None => {
                        out.push((*cid, format!(
                            "{} has no chartered substrate anchor; there is nothing to withdraw",
                            d.polity
                        )));
                        continue;
                    }
                    Some(a) if a != &d.disavower => {
                        out.push((*cid, format!(
                            "{} is not {}'s chartered substrate anchor; only the named ground may \
                             withdraw",
                            d.disavower, d.polity
                        )));
                        continue;
                    }
                    Some(_) => {}
                }
            }
            CharterEntry::ReaderIndependenceAttested(s) => {
                let a = &s.payload;
                let prior = reconstruct(archive, &a.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "an attestation about {}'s readers precedes its founding", a.polity
                    )));
                    continue;
                }
                // DELIBERATELY no dissolution guard — the THIRD such class, and at
                // this point it is a category rather than a list of exceptions.
                // Dissent, disavowal and this are all *evidence about the founding*,
                // and a founding's legitimacy outlives its polity (kith-covenant holds
                // a CommunityIdentity derived from that charter). Governing acts die
                // with the polity; accounts of how it came to be do not.
                //
                // Nobody vouches in somebody else's name.
                let signers = e.signer_dids();
                let sole: BTreeSet<String> =
                    std::iter::once(a.attesting_party.as_str().to_string()).collect();
                if signers != sole {
                    out.push((
                        *cid,
                        "an attestation must be signed by exactly its attesting party"
                            .to_string(),
                    ));
                    continue;
                }
                // Coherence with the record it speaks about — never its truth, which
                // is the same line D22's dissent arm draws. A reader the founding never
                // carried cannot be vouched for or challenged *as a reader of this
                // founding*: there is no such party in the record.
                //
                // Note what is NOT checked here. Whether the attester is disinterested
                // is a matter for the LENS (`reader_independence`), not for this pass:
                // an interested attestation is a real fact somebody signed, and
                // refusing it would destroy information, where merely not crediting it
                // costs nothing. Gate the signature, grade the counting.
                if !prior.founding_readers.contains(a.reader.as_str()) {
                    out.push((*cid, format!(
                        "{} is not a reader of {}'s founding; there is nothing to attest",
                        a.reader, a.polity
                    )));
                    continue;
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
                // D25 — the successors a dissolution names must be coherent.
                //
                // Until this, `successors` was checked by NOTHING: the emitter
                // (`en_uplift::fission`) carried no guards either, so a dissolution
                // could name its own polity, or one successor twice, and pass. That
                // matters more here than in the emitter, because en-uplift only
                // *emits* records while this pass is what a consumer verifies.
                let mut named: BTreeSet<&str> = BTreeSet::new();
                let mut bad_successor = false;
                for succ in &d.successors {
                    if !named.insert(succ.as_str()) {
                        out.push((*cid, format!(
                            "{succ} is named twice as a successor"
                        )));
                        bad_successor = true;
                        break;
                    }
                    // A polity that survives its own dissolution has not dissolved.
                    // Terminal dissolution forbids any later act, so a self-successor
                    // records a continuation the archive can never honour.
                    if succ == &d.polity {
                        out.push((*cid, format!(
                            "{succ} names itself as its own successor; a polity cannot succeed itself"
                        )));
                        bad_successor = true;
                        break;
                    }
                    // The closure gate, third of its family in this tier: a machine
                    // cannot guard a machine, a ground may not sit inside what it
                    // grounds, and a polity's successor may not be one of its own
                    // members. A member is a person or a machine; a successor is a
                    // polity, and confusing the two makes the line of descent point at
                    // somebody's keyring.
                    if members.contains(succ.as_str()) {
                        out.push((*cid, format!(
                            "successor {succ} is a member of {}, not a polity",
                            d.polity
                        )));
                        bad_successor = true;
                        break;
                    }
                }
                if bad_successor {
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
