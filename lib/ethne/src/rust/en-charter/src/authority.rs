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
//!   community receives them, and they participate). The charter's ratifiers are inducted
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
    pruning_digest, AffirmationKind, AmendmentKind, AttestedRole, BoundarySource, DissentKind,
    EnvoyKind, GuardianEvidence, GuardianKind, MembershipKind, Movement, Observation, Office,
    OfficeKind,
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
// `majority(n) + 1 <= n` rather than the equivalent `majority(n) < n` that
// clippy::int_plus_one prefers: "majority + 1" is a named threshold across this
// tier (D14's roll override, the contraction rules above), and the arithmetically
// identical form no longer says which rule this enforces.
#[allow(clippy::int_plus_one)]
fn can_self_govern(n: usize) -> bool {
    majority(n) + 1 <= n
}

fn did_set(dids: &[DidKey]) -> BTreeSet<String> {
    dids.iter().map(|d| d.as_str().to_string()).collect()
}

/// One recorded movement, as the authority pass needs it: who is being inducted, where
/// the movement sits in the sequence, who was there, when, and which entry it is — the
/// completion cites these CIDs, so they are part of the state (**D29**).
struct RitePoint {
    member: DidKey,
    /// Which ceremony this movement declared itself part of (**D34**). A rite's kind is
    /// its opening movement's; every later movement must agree.
    kind: AffirmationKind,
    order: u8,
    attending: Vec<DidKey>,
    recited_at: Timestamp,
    cid: Cid,
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
    /// The founding roll, set once at genesis and never mutated. This is the
    /// *boundary* the founding drew — what D21's exclusions are measured against and
    /// what a `BoundaryDissent` is checked for coherence with. It is deliberately
    /// **not** the induction seed: see `ratifiers`.
    founding_members: Vec<DidKey>,
    /// Who actually signed the charter. Ratifying *is* a founding member's induction
    /// (D13), so this — never the founding roll — is what seeds the inducted set
    /// (**D29**). A founding needs only a majority, so the two sets differ whenever a
    /// listed member did not sign, and treating them as one handed the substrate to
    /// members who had recited nothing at an instant they were absent from.
    ratifiers: Vec<DidKey>,
    /// Members with a prior `Induction` record.
    inducted: Vec<DidKey>,
    /// The opening-movement CID of every rite an affirmation has already completed
    /// (**D34**). A rite is signed off **once**: an induction was already protected by
    /// the no-double-induction rule, but a renewal is repeatable by nature, so without
    /// this a member could cite one re-recitation for two renewals and reset their own
    /// freshness twice off a single ceremony.
    rites_completed: std::collections::HashSet<Cid>,
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
    /// The chartered substrate's name, set once at genesis and never mutated. What a
    /// rite's first movement must name (**D29**): a rite that recites some other
    /// ground is not this polity's rite. Checked for identity, never read for meaning.
    substrate_name: String,
    /// Every rite movement recorded so far for this polity, in log order (**D29**). A
    /// member's *current* rite is the tail since their last movement 1, because opening
    /// a rite abandons any attempt in progress — which is what stops an abandoned rite
    /// from deadlocking a member forever.
    rite_movements: Vec<RitePoint>,
    /// Every reader that signed a reading behind the founding boundary, set once at
    /// genesis (design **D24**). An independence attestation must name one of these:
    /// like the anchor, a reader is *in* the record, so the subject of the claim is
    /// checkable even though the claim itself never can be.
    founding_readers: BTreeSet<String>,
    /// Every observer the log has carried so far — a filed `Observed` entry, or an
    /// observation carried as guardian evidence at founding or on a `GuardianChange`
    /// (design **D28**). The observer counterpart of `founding_readers`, and unlike it
    /// this one **accrues**: monitors arrive over the polity's life, so an attestation
    /// about one is checkable against the log up to the filing rather than against
    /// genesis.
    observers: BTreeSet<String>,
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
        ratifiers: Vec::new(),
        inducted: Vec::new(),
        rites_completed: std::collections::HashSet::new(),
        derivations: Vec::new(),
        envoys: Vec::new(),
        founding_exclusions: BTreeSet::new(),
        anchor: None,
        substrate_name: String::new(),
        rite_movements: Vec::new(),
        founding_readers: BTreeSet::new(),
        observers: BTreeSet::new(),
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
                r.ratifiers = s.payload.ratifiers.clone();
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
                r.substrate_name = s.payload.substrate.name.clone();
                for rd in s.payload.attestation.source.readings() {
                    r.founding_readers.insert(rd.payload.reader.as_str().to_string());
                }
                // D28 — a monitor whose observation the founding CARRIED as guardian
                // evidence has observed this polity as surely as one who filed an
                // entry of its own; the record is just held somewhere else.
                for d in &s.payload.derivations {
                    if let Some(GuardianEvidence::Observed(ob)) = &d.evidence {
                        r.observers.insert(ob.payload.observer.as_str().to_string());
                    }
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
                        if !r.offices.iter().any(held) {
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
                if let Some(GuardianEvidence::Observed(ob)) = &gc.evidence {
                    r.observers.insert(ob.payload.observer.as_str().to_string());
                }
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
                if let Some(first) = a.movements.first() {
                    r.rites_completed.insert(*first);
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
            //
            // An observation still records *that a monitor spoke*, which is not the
            // polity's state but is a fact about the log — the same distinction the
            // founding readers already draw. `observers` is that ledger, and D28's
            // coherence gate is the only thing that reads it.
            CharterEntry::Observed(s) if &s.payload.polity == polity => {
                r.observers.insert(s.payload.observer.as_str().to_string());
            }
            // A movement decides nothing, but the rite's progress is state: the next
            // movement's order, and the completion's attendance count, both read it.
            CharterEntry::RiteMovementRecorded(s) if &s.payload.polity == polity => {
                let m = &s.payload;
                r.rite_movements.push(RitePoint {
                    member: m.member.clone(),
                    kind: m.kind,
                    order: m.movement.order(),
                    attending: m.attending.clone(),
                    recited_at: m.recited_at,
                    cid: *_cid,
                });
            }
            CharterEntry::RiteMovementRecorded(_) => {}
            CharterEntry::Observed(_)
            | CharterEntry::BoundaryDissented(_)
            | CharterEntry::SubstrateDisavowed(_)
            | CharterEntry::IndependenceAttested(_) => {}
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

/// The members who hold the substrate, and so carry deliberative weight (**D29**).
///
/// D13 made induction *"the distinct step by which they acquire the substrate"* and
/// nothing read it: measured, a roll of three inducted founders plus six members who
/// never were let the six confer an office on one of their own, amend the polity's
/// commitments, expel a founder, and preside over a newcomer's induction into a
/// substrate none of them had received — every pass green. A polity is constituted by
/// its substrate, so governing it is for those who hold it.
///
/// The whole roll is still the roll: un-inducted members are members, may depart, may
/// be expelled, may hold their own first-class acts. What they do not carry is weight.
fn seasoned_members(prior: &Reconstructed) -> BTreeSet<String> {
    prior
        .members
        .iter()
        .filter(|m| prior.ratifiers.contains(m) || prior.inducted.contains(m))
        .map(|m| m.as_str().to_string())
        .collect()
}

/// How many of `deciders` carry deliberative weight (**D29**).
///
/// A de-weighting, not a refusal: an un-inducted decider may be named and may sign —
/// the record still says it acted — and its signature simply does not count toward the
/// threshold. Refusing the record instead would mean a polity had to know who was
/// inducted before it could author anything, and would throw away the honest fact that
/// the member was there.
fn deciding_weight(prior: &Reconstructed, deciders: &[DidKey]) -> usize {
    let seasoned = seasoned_members(prior);
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    deciders
        .iter()
        .filter(|d| seen.insert(d.as_str()))
        .filter(|d| seasoned.contains(d.as_str()))
        .count()
}

/// The movements of `member`'s **current** rite, in log order: the tail since their
/// last movement 1 (design **D29**).
///
/// A `Substrate` movement opens a rite and abandons whatever was in progress, so the
/// tail is the whole answer — no rite identifier is needed and an abandoned attempt
/// cannot deadlock a member who wants to start again.
fn current_rite<'a>(prior: &'a Reconstructed, member: &DidKey) -> Vec<&'a RitePoint> {
    let mine: Vec<&RitePoint> =
        prior.rite_movements.iter().filter(|p| &p.member == member).collect();
    match mine.iter().rposition(|p| p.order == 1) {
        Some(i) => mine[i..].to_vec(),
        None => Vec::new(),
    }
}

/// The members who attended **every** movement of `member`'s current rite, and the
/// rite's span, or `None` if the rite is not complete (design **D29**).
///
/// Attendance is an intersection rather than a union because that is the difference
/// between a ceremony and a stack of signatures: five movements signed by five
/// disjoint groups is a rite nobody was at. D28 made the same correction for
/// observations — count the watchers, not the filings.
fn completed_rite(prior: &Reconstructed, member: &DidKey) -> Option<CompleteRite> {
    let rite = current_rite(prior, member);
    if rite.len() != Movement::COUNT as usize {
        return None;
    }
    // The ordering rule enforced at authoring time means a complete tail is 1..=5.
    if rite.iter().enumerate().any(|(i, p)| p.order != i as u8 + 1) {
        return None;
    }
    let mut throughout: Option<BTreeSet<String>> = None;
    for p in &rite {
        let here = did_set(&p.attending);
        throughout = Some(match throughout {
            None => here,
            Some(prev) => prev.intersection(&here).cloned().collect(),
        });
    }
    Some(CompleteRite {
        kind: rite[0].kind,
        throughout: throughout.unwrap_or_default(),
        cids: rite.iter().map(|p| p.cid).collect(),
        last_at: rite[rite.len() - 1].recited_at,
    })
}

/// A complete rite as the completion rule needs it (**D29**).
struct CompleteRite {
    /// The ceremony the rite declared itself to be (**D34**), taken from its opening
    /// movement. The completing affirmation's own kind must match it, so an induction
    /// cannot sign off on a renewal's recitation or the reverse.
    kind: AffirmationKind,
    /// Members who attended every movement — the only ones who may witness.
    throughout: BTreeSet<String>,
    /// The movement entries, in order; the completion must cite exactly these.
    cids: Vec<Cid>,
    /// When the last movement was recited; a completion cannot predate it.
    last_at: Timestamp,
}

/// Who *effectively* holds `office` as of the reconstructed point: a raw grant
/// counts only while its holder is on the roll (D12 — a holding lapses when its
/// holder departs and revives when they return). Both use-time rules (D14) read
/// the holdings through this, never the raw grants, so a departed officer neither
/// presides nor authorizes.
fn effective_holders(prior: &Reconstructed, office: &Office) -> BTreeSet<String> {
    // D29 — and a holding is effective only while its holder holds the *substrate*,
    // for the reason the roll rule exists: an office is the polity acting through a
    // member, and a member who has not received what the polity is cannot act for it.
    // Like the roll rule this lapses and revives rather than revoking: a holder who is
    // later inducted presides again, with no new conferral.
    let seasoned = seasoned_members(prior);
    prior
        .offices
        .iter()
        .filter(|(o, h)| o == office && prior.members.contains(h))
        .filter(|(_, h)| seasoned.contains(h.as_str()))
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


/// Does a carried observation actually show the edge a [`GuardianChange`] claims?
///
/// The evidence half of D15's substrate backing. Everything asserted here is a
/// property of the carried record itself, so it stays true no matter what the log
/// later drops (D17) — which is the whole reason the evidence is carried rather than
/// cited.
///
/// The observation's *own* well-formedness (signed by exactly its observer, no
/// duplicate readings, no stamp postdating the look) is enforced where standalone
/// observations are, and its signature by `verify_all`. What is checked here is only
/// the relation between this evidence and this claim.
fn evidence_shows_edge(
    ob: &en_core::Signed<Observation>,
    polity: &DidKey,
    member: &DidKey,
    guardian: Option<&DidKey>,
    decided_at: Timestamp,
) -> Result<(), String> {
    let o = &ob.payload;
    if &o.polity != polity {
        return Err(format!(
            "guardian evidence observes {} but the change is for {polity}", o.polity
        ));
    }
    // A reading cannot back a decision it postdates: evidence is what was seen
    // *before* the polity decided, not a justification assembled afterwards.
    if o.observed_at > decided_at {
        return Err(format!(
            "guardian evidence was observed at {} , after the change decided at {decided_at}",
            o.observed_at
        ));
    }
    // An observation must be signed by exactly its observer wherever it appears, so
    // carried evidence cannot be put in somebody else's mouth either.
    let signers: BTreeSet<String> =
        ob.signatures.iter().map(|sig| sig.signer_did.clone()).collect();
    let required: BTreeSet<String> =
        std::iter::once(o.observer.as_str().to_string()).collect();
    if signers != required {
        return Err("guardian evidence must be signed by exactly its observer".into());
    }
    let Some(reading) = o.readings.iter().find(|r| &r.member == member) else {
        return Err(format!(
            "guardian evidence carries no reading for {member}; a member absent from the \
             readings was not observed"
        ));
    };
    if reading.guardian.as_ref() != guardian {
        return Err(match (&reading.guardian, guardian) {
            (Some(saw), Some(claimed)) => format!(
                "guardian evidence shows {member} guarded by {saw}, not the claimed {claimed}"
            ),
            (None, Some(claimed)) => format!(
                "guardian evidence shows {member} with no guardian, not the claimed {claimed}"
            ),
            _ => format!("guardian evidence disagrees with the claimed edge for {member}"),
        });
    }
    Ok(())
}

/// All authority violations in the archive, each as `(entry_cid, reason)`.
pub fn authority_violations(archive: &Archive<CharterEntry>) -> Vec<(Cid, String)> {
    let mut out = Vec::new();
    // D37 — the content policy each polity lives under, advanced ONLY by accepted
    // amendments. Unlike the commitments `reconstruct` folds, a policy amendment is
    // judged against the policy it replaces, so the baseline must be one that
    // passed: measured against a refused extreme loosening, a real loosening would
    // read as a tightening and pass at a bare majority.
    let mut policies: std::collections::BTreeMap<String, crate::ContentPolicy> =
        std::collections::BTreeMap::new();
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
                // D27 — the founding edge must show what stood behind it. D15 demands
                // this of every later `GuardianChange`; founding derivations asserted
                // an edge and carried nothing, which was not a cosmetic asymmetry: the
                // GROUNDED rule below turns on whether *any* edge is guarded, so one
                // claimed edge with a co-signing confederate exempted a whole machine
                // cluster from naming a ground. Measured before the slice as
                // `probe`-shaped: three machines, one asserted edge, no anchor, both
                // passes green.
                //
                // Run *above* the GROUNDED rule on purpose. That rule now counts backed
                // edges, and validating the evidence first means it can never be reached
                // with an `Observed` that does not survive `evidence_shows_edge` —
                // otherwise the order of the two would be load-bearing by accident,
                // which is the trap the note below this one already names.
                let mut bad_evidence = false;
                for d in &p.derivations {
                    match (&d.evidence, &d.guardian) {
                        (None, Some(_)) => {
                            out.push((*cid, format!(
                                "founding derivation for {} claims a guardian edge with no \
                                 substrate evidence and no declared override (D27)", d.member
                            )));
                            bad_evidence = true;
                            break;
                        }
                        (Some(_), None) => {
                            // Nothing is claimed, so there is nothing to evidence. A
                            // machine chartered as answering to nobody is a state, not
                            // an assertion about the world.
                            out.push((*cid, format!(
                                "founding derivation for {} carries substrate evidence but \
                                 claims no guardian edge", d.member
                            )));
                            bad_evidence = true;
                            break;
                        }
                        (None, None) => {}
                        (Some(GuardianEvidence::Unbacked { reason }), Some(_)) => {
                            if reason.trim().is_empty() {
                                out.push((*cid, format!(
                                    "an unbacked founding guardian edge for {} must say why \
                                     (D27 override)", d.member
                                )));
                                bad_evidence = true;
                                break;
                            }
                        }
                        (Some(GuardianEvidence::Observed(ob)), Some(g)) => {
                            if let Err(why) = evidence_shows_edge(
                                ob, &p.polity, &d.member, Some(g), p.founded_at,
                            ) {
                                out.push((*cid, why));
                                bad_evidence = true;
                                break;
                            }
                        }
                    }
                }
                if bad_evidence {
                    continue;
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
                //
                // Since D27 an edge only counts as guarding if an observation **backed**
                // it. A declared `Unbacked` override is honest and permitted, but it
                // cannot buy the exemption: post-founding that override is safe precisely
                // because it can only weaken a record, and letting it satisfy this rule
                // would make a self-declaration strengthening — what D21 and D22 each
                // had to delete.
                //
                // Nor may the observation come from inside. A founder who signs one
                // about the cluster's own edges is the polity vouching for its own
                // ground, which is the closure D23 refused for the anchor and D25 for a
                // successor. `is_backed` carries that rule, so the grade cannot drift
                // from the gate.
                let derived_on_roll: BTreeSet<&str> = p
                    .derivations
                    .iter()
                    .map(|d| d.member.as_str())
                    .filter(|m| members.contains(*m))
                    .collect();
                let all_derived = derived_on_roll.len() == members.len();
                let none_guarded = !p
                    .derivations
                    .iter()
                    .any(|d| d.is_backed(&p.polity, |o| members.contains(o)));
                if all_derived && none_guarded && p.substrate.anchor.is_none() {
                    let claimed = p.derivations.iter().filter(|d| d.guardian.is_some()).count();
                    out.push((*cid, if claimed == 0 {
                        format!(
                            "a roll of {} machines answering to nobody must name a substrate \
                             anchor: nobody here can hold a tradition",
                            p.members.len()
                        )
                    } else {
                        // The D27 cases: edges are claimed, but their whole warrant comes
                        // from inside the polity — either declared unbacked outright, or
                        // observed by the roll about itself. Both are the polity's own
                        // say-so, and the refusal says which it is looking at, since the
                        // remedy differs (write down a ground, or have somebody outside
                        // look).
                        let inside = p
                            .derivations
                            .iter()
                            .filter(|d| {
                                matches!(d.evidence, Some(GuardianEvidence::Observed(_)))
                                    && d.guardian.is_some()
                            })
                            .count();
                        format!(
                            "a roll of {} machines must name a substrate anchor: of its \
                             {claimed} guardian edge(s), {inside} rest on an observation \
                             taken from inside this polity and {} are self-declared — \
                             nobody here can hold a tradition",
                            p.members.len(),
                            claimed - inside,
                        )
                    }));
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
                // D29 — a majority of those who hold the substrate, counted over the
                // deciders who carry weight.
                let m = seasoned_members(&prior).len();
                let weighed = deciding_weight(&prior, &a.deciders);
                let key = &a.commitment.key;
                match a.kind {
                    AmendmentKind::Add => {
                        if prior.in_force.contains(key) {
                            out.push((*cid, format!("Add of an already in-force commitment '{key}'")));
                            continue;
                        }
                        let need = majority(m);
                        if weighed < need {
                            out.push((*cid, format!("Add needs a majority (>= {need}), got {weighed}")));
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
                        if weighed < need {
                            out.push((*cid, format!(
                                "Remove needs more than a majority (>= {need}), got {weighed}"
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
                // D29 — majorities are of the members who hold the substrate.
                let roll = seasoned_members(&prior).len();
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
                        let weighed = deciding_weight(&prior, &mc.deciders);
                        if weighed < need {
                            out.push((*cid, format!("Admit needs a majority (>= {need}), got {weighed}")));
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
                        let weighed = deciding_weight(&prior, &mc.deciders);
                        if weighed < need {
                            out.push((*cid, format!(
                                "Expel needs more than a majority (>= {need}), got {weighed}"
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
                // D29 — majorities are of the members who hold the substrate.
                let roll = seasoned_members(&prior).len();
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
                        // D29 — refused outright rather than de-weighted, because there
                        // is no count here to de-weight: a conferral either happens or
                        // it does not, and an office nobody could exercise is exactly
                        // the inert record this tier keeps refusing to write. (The same
                        // reasoning KC's §2 gives for refusing a probationary sponsor.)
                        if !seasoned_members(&prior).contains(oc.holder.as_str()) {
                            out.push((*cid, format!(
                                "office conferred on {}, who is on the roll but has not been \
                                 inducted; an office is the polity acting through a member",
                                oc.holder
                            )));
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
                        let weighed = deciding_weight(&prior, &oc.deciders);
                        if weighed < need {
                            out.push((*cid, format!("conferral needs a majority (>= {need}), got {weighed}")));
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
                        let weighed = deciding_weight(&prior, &oc.deciders);
                        if weighed < need {
                            out.push((*cid, format!("revocation needs a majority (>= {need}), got {weighed}")));
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
                // D29 — majorities are of the members who hold the substrate.
                let roll = seasoned_members(&prior).len();
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
                // D15's substrate backing: an edge that is *claimed* must carry what
                // the substrate showed, or say outright that nothing did. Checked
                // here rather than only at the authoring site because an
                // emitter-side guard is walked around by a hand-built record, and
                // `check_authority` is what a consumer verifies (D25's finding).
                let claims_edge = match gc.kind {
                    GuardianKind::Designate => gc.guardian.is_some(),
                    GuardianKind::Rotate => true,
                    GuardianKind::Release => false,
                };
                match (&gc.evidence, claims_edge) {
                    (None, true) => {
                        out.push((*cid, format!(
                            "guardian change for {} claims an edge with no substrate evidence \
                             and no declared override (D15)", gc.member
                        )));
                        continue;
                    }
                    (Some(_), false) => {
                        // A release claims an edge is *gone*; an observation showing
                        // one present would argue the opposite, and one showing it
                        // absent is not the same object (D16: a member absent from
                        // the readings was not observed).
                        out.push((*cid, "a release carries no substrate evidence; it claims the edge is gone".into()));
                        continue;
                    }
                    (None, false) => {}
                    (Some(GuardianEvidence::Unbacked { reason }), true) => {
                        if reason.trim().is_empty() {
                            out.push((*cid, "an unbacked guardian edge must say why (D15 override)".into()));
                            continue;
                        }
                    }
                    (Some(GuardianEvidence::Observed(ob)), true) => {
                        if let Err(why) = evidence_shows_edge(
                            ob, &gc.polity, &gc.member, gc.guardian.as_ref(), gc.decided_at,
                        ) {
                            out.push((*cid, why));
                            continue;
                        }
                    }
                }
                match gc.kind {
                    GuardianKind::Designate => {
                        if current.is_some() {
                            out.push((*cid, "member is already designated a derived party".into()));
                            continue;
                        }
                        let weighed = deciding_weight(&prior, &gc.deciders);
                            if weighed < need {
                                out.push((*cid, format!(
                                "designation needs a majority of the roll (>= {need}), got {weighed}"
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
                        let weighed = deciding_weight(&prior, &gc.deciders);
                            if weighed < need {
                                out.push((*cid, format!(
                                "rotation needs a majority of the roll (>= {need}), got {weighed}"
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
                            let weighed = deciding_weight(&prior, &gc.deciders);
                                if weighed < need {
                                    out.push((*cid, format!(
                                    "a release the roll records needs a majority (>= {need}), got {weighed}"
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
                // D29 — majorities are of the members who hold the substrate.
                let roll = seasoned_members(&prior).len();
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
                        let weighed = deciding_weight(&prior, &ea.authorizers);
                        if !by_officer && weighed < need {
                            out.push((*cid, format!(
                                "authorization needs an EnvoyAuthorizer other than the envoy among \
                                 the authorizers, or a majority of the roll (>= {need}), got {weighed}"
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
                        let weighed = deciding_weight(&prior, &ea.authorizers);
                        if !by_officer && weighed < need {
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
                // D29 — majorities are of the members who hold the substrate.
                let roll = seasoned_members(&prior).len();
                if !members.contains(&member) {
                    out.push((*cid, "affirmation by a non-member".into()));
                    continue;
                }
                // Ratifying the charter is a founding member's induction.
                // Ratifying the charter is a founding member's induction — and only
                // the ratifiers'. A listed founding member who never signed is on the
                // roll and un-inducted (D29).
                let is_inducted = prior.ratifiers.contains(&af.member)
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
                        if let Some(r) = completed_rite(&prior, &af.member) {
                            if r.kind != AffirmationKind::Induction {
                                out.push((*cid,
                                    "induction cites a renewal's re-recitation; an induction \
                                     completes an induction rite".into()));
                                continue;
                            }
                        }
                        // D29 — the rite happened, and this record completes it. The
                        // citation must be *exactly* the current rite's five movements
                        // in order, so a completion can neither borrow somebody else's
                        // rite nor sign off on an attempt that was abandoned.
                        let Some(done) = completed_rite(&prior, &af.member) else {
                            let so_far = current_rite(&prior, &af.member).len();
                            out.push((*cid, format!(
                                "induction completes a rite of {} movements, but {so_far} of {} \
                                 have been recorded",
                                Movement::COUNT,
                                Movement::COUNT
                            )));
                            continue;
                        };
                        if af.movements != done.cids {
                            out.push((*cid,
                                "an induction must cite the five movements of its own rite, in order"
                                    .into()));
                            continue;
                        }
                        if prior.rites_completed.contains(&done.cids[0]) {
                            out.push((*cid,
                                "this rite has already been signed off; a ceremony completes one                                  affirmation".into()));
                            continue;
                        }
                        if af.affirmed_at < done.last_at {
                            out.push((*cid,
                                "induction is dated before the last movement of its rite".into()));
                            continue;
                        }
                        // Only those present for the WHOLE rite witness it. Five
                        // movements signed by five disjoint groups is a rite nobody
                        // attended, which is D28's arithmetic one record over.
                        if let Some(absent) =
                            af.witnesses.iter().find(|w| !done.throughout.contains(w.as_str()))
                        {
                            out.push((*cid, format!(
                                "witness {absent} did not attend every movement of the rite"
                            )));
                            continue;
                        }
                        let need = majority(roll);
                        if af.witnesses.len() < need {
                            out.push((*cid, format!(
                                "induction needs a majority of the inducted roll to witness \
                                 (>= {need}), got {}",
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
                        // **D34** — a renewal carries a full re-recitation of the five
                        // movements, which is what the doctrine means by periodic
                        // re-affirmation. Before D34 this rule refused any citation, so
                        // the renewal was a bare record: the recurring ceremony was the
                        // one with no cost, and a polity that actually performed the rite
                        // produced five violations for its trouble.
                        //
                        // The citation rule is the induction's, reused rather than
                        // restated: exactly the current rite's five movements, in order,
                        // so a renewal can neither borrow another rite nor sign off on an
                        // abandoned attempt.
                        let Some(done) = completed_rite(&prior, &af.member) else {
                            let so_far = current_rite(&prior, &af.member).len();
                            out.push((*cid, format!(
                                "re-affirmation completes a re-recitation of {} movements, but \
                                 {so_far} of {} have been recorded",
                                Movement::COUNT,
                                Movement::COUNT
                            )));
                            continue;
                        };
                        if af.movements != done.cids {
                            out.push((*cid,
                                "a re-affirmation must cite the five movements of its own \
                                 re-recitation, in order".into()));
                            continue;
                        }
                        if done.kind != AffirmationKind::ReAffirmation {
                            out.push((*cid,
                                "re-affirmation cites an induction's rite; a renewal re-recites \
                                 its own".into()));
                            continue;
                        }
                        // A re-recitation completes ONE renewal. Nothing else stops a
                        // member replaying a single ceremony to reset their freshness
                        // repeatedly: the induction path is protected by the
                        // no-double-induction rule, and a renewal is repeatable by nature.
                        if prior.rites_completed.contains(&done.cids[0]) {
                            out.push((*cid,
                                "this re-recitation has already been renewed on; a ceremony \
                                 completes one affirmation".into()));
                            continue;
                        }
                        if af.affirmed_at < done.last_at {
                            out.push((*cid,
                                "re-affirmation is dated before the last movement of its \
                                 re-recitation".into()));
                            continue;
                        }
                        // Attendance is **recorded, not gated**: it must be exactly who
                        // was there throughout, so a member can neither over- nor
                        // under-report it, and it may be empty — nobody's cooperation is
                        // required for a renewal. There is deliberately no threshold here.
                        let attended: BTreeSet<String> = done.throughout.clone();
                        if witnesses != attended {
                            out.push((*cid, format!(
                                "a re-affirmation records exactly who attended every movement of \
                                 its re-recitation ({} of them), not a chosen list",
                                attended.len()
                            )));
                            continue;
                        }
                        // Renewal is still the member's own act (**D14**): those who
                        // attended have already signed the movements, and nothing here
                        // lets them condition the renewal itself.
                        let required: BTreeSet<String> = std::iter::once(member.clone()).collect();
                        if signers != required {
                            out.push((*cid, "re-affirmation must be signed by exactly the affirming member".into()));
                            continue;
                        }
                    }
                }
            }
            CharterEntry::RiteMovementRecorded(s) => {
                let m = &s.payload;
                let prior = reconstruct(archive, &m.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("rite movement for {} precedes its charter", m.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("rite movement for {} after its dissolution", m.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let member = m.member.as_str().to_string();
                if !members.contains(&member) {
                    out.push((*cid, "rite movement for a non-member".into()));
                    continue;
                }
                // The movement's declared ceremony must match the member's actual state
                // (**D34**). Admission puts you on the roll and induction is how you
                // acquire the substrate, so someone who already holds it has nothing to
                // acquire — but they do have something to *renew*, which is what the
                // doctrine asks of them periodically, and which before D34 this rule
                // refused outright.
                let holds_substrate =
                    prior.ratifiers.contains(&m.member) || prior.inducted.contains(&m.member);
                match m.kind {
                    AffirmationKind::Induction if holds_substrate => {
                        out.push((*cid, "induction rite movement for an already-inducted member".into()));
                        continue;
                    }
                    AffirmationKind::ReAffirmation if !holds_substrate => {
                        out.push((*cid, "re-affirmation rite movement for a member who was never \
                                         inducted; there is nothing yet to renew".into()));
                        continue;
                    }
                    _ => {}
                }
                // A movement is performed. An empty one was not.
                if m.statement.trim().is_empty() {
                    out.push((*cid, format!(
                        "movement {} carries no statement; a movement is performed, not listed",
                        m.movement.order()
                    )));
                    continue;
                }
                let attending = did_set(&m.attending);
                if attending.len() != m.attending.len() {
                    out.push((*cid, "duplicate attendee at a rite movement".into()));
                    continue;
                }
                if !attending.is_subset(&members) {
                    out.push((*cid, "rite movement attendee is not a member of the polity".into()));
                    continue;
                }
                // Nobody attends their own ceremony (**D34**). At an induction this was
                // already impossible — attendees must hold the substrate and the inductee
                // does not — but a renewing member holds it, so without this rule they
                // could list themselves and their own rite would report an attendee. That
                // is D28's arithmetic again: count the people present, not the signatures
                // collected.
                if attending.contains(&member) {
                    out.push((*cid, "the reciting member may not attend their own rite movement".into()));
                    continue;
                }
                // The substrate is transmitted by those who hold it (**D29**).
                if let Some(who) = m
                    .attending
                    .iter()
                    .find(|w| !prior.ratifiers.contains(w) && !prior.inducted.contains(w))
                {
                    out.push((*cid, format!(
                        "rite movement attendee {who} is not inducted; the substrate is \
                         transmitted by those who hold it"
                    )));
                    continue;
                }
                // Everyone present signs, and nobody else does: the inductee recites
                // and those attending receive them.
                let mut required = attending.clone();
                required.insert(member.clone());
                if e.signer_dids() != required {
                    out.push((*cid,
                        "a rite movement must be signed by exactly its attendees plus the reciting member"
                            .into()));
                    continue;
                }
                // Order is semantic. Movement 1 always opens (or re-opens) a rite;
                // every other movement must be the immediate successor of the last.
                let rite = current_rite(&prior, &m.member);
                let order = m.movement.order();
                if order == 1 {
                    // Always permitted: opening a rite abandons any attempt in progress.
                } else {
                    let last = rite.last();
                    match last {
                        None => {
                            out.push((*cid, format!(
                                "movement {order} with no rite open; a rite opens at movement 1"
                            )));
                            continue;
                        }
                        Some(p) => {
                            // A rite is one ceremony throughout (**D34**): its kind is
                            // set by the opening movement and every later one must agree,
                            // or a renewal could be finished off as an induction.
                            if p.kind != m.kind {
                                out.push((*cid, format!(
                                    "movement {order} declares a different ceremony than the rite                                      it continues; a rite's kind is set by its opening movement"
                                )));
                                continue;
                            }
                            let (prev, prev_at) = (p.order, p.recited_at);
                            if order != prev + 1 {
                                out.push((*cid, format!(
                                    "movement {order} follows movement {prev}; the rite's order is \
                                     semantic and its movements may not be skipped or repeated"
                                )));
                                continue;
                            }
                            if m.recited_at < prev_at {
                                out.push((*cid, format!(
                                    "movement {order} is dated before movement {prev}"
                                )));
                                continue;
                            }
                        }
                    }
                }
                // Coherence with the charter, never content (D24/D28's line).
                match &m.movement {
                    Movement::Substrate { names } => {
                        if names != &prior.substrate_name {
                            out.push((*cid, format!(
                                "movement 1 names substrate {names:?} but the charter's is {:?}",
                                prior.substrate_name
                            )));
                            continue;
                        }
                    }
                    Movement::CommonVirtue { commits_to } => {
                        let said: BTreeSet<String> = commits_to.iter().cloned().collect();
                        if said != prior.in_force {
                            out.push((*cid, format!(
                                "movement 4 commits to {} of the polity's {} commitments in force",
                                said.intersection(&prior.in_force).count(),
                                prior.in_force.len()
                            )));
                            continue;
                        }
                    }
                    Movement::NonUltimacy { names_anchor } => {
                        if names_anchor != &prior.anchor {
                            out.push((*cid,
                                "movement 5 does not name the anchor the charter names".into()));
                            continue;
                        }
                    }
                    // No rule can read contrition or recognition, and a gate that
                    // pretended to would be the hollowest thing in the tier.
                    Movement::Fallenness | Movement::Recognition => {}
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
                let need = majority(seasoned_members(&prior).len());
                let weighed = deciding_weight(&prior, &pr.deciders);
                if !by_officer && weighed < need {
                    out.push((*cid, format!(
                        "pruning needs a CharterKeeper among the deciders and a horizon at least \
                         {PRUNE_SETTLING_SECONDS}s old, or a majority of the roll (>= {need}), got {weighed}"
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
            CharterEntry::RelayEndpointDeclared(s) => {
                let d = &s.payload;
                let prior = reconstruct(archive, &d.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("relay endpoint for {} precedes its charter", d.polity)));
                    continue;
                }
                // A governing act's shadow, so it dies with the polity: a dissolved
                // polity runs no service, and a roster read after dissolution must
                // not find one declared later (D36).
                if prior.dissolved {
                    out.push((*cid, format!("relay endpoint for {} after its dissolution", d.polity)));
                    continue;
                }
                // Nobody puts an endpoint in somebody else's mouth.
                let signers = e.signer_dids();
                let sole: BTreeSet<String> =
                    std::iter::once(d.member.as_str().to_string()).collect();
                if signers != sole {
                    out.push((*cid, "a relay endpoint must be signed by exactly its member".into()));
                    continue;
                }
                if let Err(why) = d.check_shape() {
                    out.push((*cid, format!("relay endpoint of {}: {why}", d.member)));
                    continue;
                }
                // Whether the member serves is the roll's decision, made by
                // conferring the office; the declaration only says where. A
                // withdrawal needs no office, so an operator who was revoked can
                // still take back what it said.
                if !d.withdrawn
                    && !effective_holders(&prior, &Office::RendezvousOperator)
                        .contains(d.member.as_str())
                {
                    out.push((*cid, format!(
                        "{} declared a relay endpoint without holding RendezvousOperator",
                        d.member
                    )));
                    continue;
                }
            }
            CharterEntry::ContentPolicyAmended(s) => {
                let a = &s.payload;
                let prior = reconstruct(archive, &a.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!("content policy for {} precedes its charter", a.polity)));
                    continue;
                }
                if prior.dissolved {
                    out.push((*cid, format!("content policy for {} after its dissolution", a.polity)));
                    continue;
                }
                if let Err(why) = a.policy.check_shape() {
                    out.push((*cid, format!("content policy of {}: {why}", a.polity)));
                    continue;
                }
                let members = did_set(&prior.members);
                let deciders = did_set(&a.deciders);
                if a.deciders.is_empty() {
                    out.push((*cid, "content policy amendment has no deciders".into()));
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
                // D29 weight, and D11's asymmetry: a majority binds the polity tighter,
                // and only more than a majority lets anything back through.
                let m = seasoned_members(&prior).len();
                let weighed = deciding_weight(&prior, &a.deciders);
                let key = a.polity.as_str().to_string();
                let in_force = policies.get(&key).cloned().unwrap_or_default();
                if a.policy.is_loosening_of(&in_force) {
                    let need = majority(m) + 1;
                    if weighed < need {
                        out.push((*cid, format!(
                            "loosening the content policy needs more than a majority (>= {need}), got {weighed}"
                        )));
                        continue;
                    }
                } else {
                    let need = majority(m);
                    if weighed < need {
                        out.push((*cid, format!(
                            "a content policy needs a majority (>= {need}), got {weighed}"
                        )));
                        continue;
                    }
                }
                policies.insert(key, a.policy.clone());
            }
            CharterEntry::IndependenceAttested(s) => {
                let a = &s.payload;
                let prior = reconstruct(archive, &a.polity, idx);
                if !prior.chartered {
                    out.push((*cid, format!(
                        "an attestation about {} precedes its founding", a.polity
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
                // is the same line D22's dissent arm draws. A party the log never
                // carried in the named role cannot be vouched for or challenged *in
                // that role*: there is no such party in the record.
                //
                // The gate is parameterized by role rather than duplicated (**D28**),
                // and the two halves are checkable against different things: a
                // boundary reader is fixed at genesis, while observers accrue over the
                // polity's life, so the observer half reads the log up to this filing.
                // Note the consequence — a monitor must have **spoken before it can be
                // vouched for**. Vouching for a key that has said nothing yet would be
                // pre-registering a witness, which is the sockpuppet move with a
                // longer setup.
                //
                // Note what is NOT checked here. Whether the attester is disinterested
                // is a matter for the LENS (`reader_independence` /
                // `observer_independence`), not for this pass: an interested
                // attestation is a real fact somebody signed, and refusing it would
                // destroy information, where merely not crediting it costs nothing.
                // Gate the signature, grade the counting.
                let known = match a.role {
                    AttestedRole::BoundaryReader => {
                        prior.founding_readers.contains(a.subject.as_str())
                    }
                    AttestedRole::Observer => prior.observers.contains(a.subject.as_str()),
                };
                if !known {
                    out.push((*cid, format!(
                        "{} is not {} {}; there is nothing to attest",
                        a.subject,
                        a.role.noun(),
                        a.polity
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
                let need = majority(seasoned_members(&prior).len());
                let weighed = deciding_weight(&prior, &d.deciders);
                    if weighed < need {
                        out.push((*cid, format!(
                        "dissolution needs a majority of the roll (>= {need}), got {weighed}"
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
