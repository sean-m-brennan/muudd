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

//! The charter record shapes and the ratify-not-replace founding API.

use std::collections::BTreeSet;

use en_core::{Cid, DidKey, KcError, KcResult, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// What the polity holds above itself — its non-ultimacy anchor. **Genesis-
/// immutable**: there is deliberately no amendment record that changes a
/// substrate, because a polity that re-decides what it defers to has become a
/// different polity. (For a derived — e.g. machine-majority — polity, `anchor`
/// names the human/guardian ground the substrate rests on.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Substrate {
    pub name: String,
    pub statement: String,
    pub anchor: Option<DidKey>,
}

/// One member's **derived-party** designation: this member is a machine, and the
/// human it answers to (design **D15**, and D8's node→guardian edge made
/// structural). KC's machine-participation model admits a machine only in a
/// derived form "irrevocably tied to one specific named human"; a `Derivation` is
/// that tie, ratified rather than merely observed.
///
/// `guardian: None` is a **real chartered state** — a machine with nobody behind
/// it — not a missing value. Recording it honestly is what lets the office rule
/// bite: in a machine-majority polity such a member may hold no office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Derivation {
    pub member: DidKey,
    pub guardian: Option<DidKey>,
}

/// How the boundary a charter ratifies was established (design **D21**, resolving
/// open question **Q1**).
///
/// The distinction the codebase already made informally now has a home:
/// `en-substrate`'s `ManualFounding` gives asserted members `Some(1.0)` rather than
/// `None` precisely because *a convener vouching for a roll is the reading here*.
/// This says the same thing in the charter, permanently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundarySource {
    /// An independent reading of the substrate.
    Substrate,
    /// The convener's own assertion — nobody else attests this boundary.
    ConvenerAsserted,
}

/// Someone the boundary showed whom the roll does not name, and why.
///
/// Named, never merely absent. An excluded party can point at the charter forever
/// and see both that they were left out and what was said about it — which is the
/// whole of what this slice buys, since nothing here can stop a founding from being
/// narrow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exclusion {
    pub member: DidKey,
    /// Why. An empty reason is still accounting — the member is named — but it is
    /// accounting without justification, and `founding_legitimacy` counts it.
    pub reason: String,
}

/// The boundary a charter claims to ratify: where it came from, how big it was, and
/// who was left out of it (design **D21**).
///
/// **Why the charter must carry this.** Before D21 the genesis quorum was
/// `majority(members.len())` — the denominator was the proposal's *own roll* — and
/// `FoundingCandidate::propose` discarded the candidate it drew that roll from. The
/// charter had no referent outside itself, so the bar was satisfiable by
/// construction: measured, **one node chartered a thirty-node cluster as a polity of
/// itself, with one signature, passing both verification passes**, and the exclusion
/// of the other twenty-nine was not merely unpunished but *unrecorded*.
///
/// **What it is and is not.** This is the proposer's signed *claim*, and it is
/// accountability rather than proof: the archive holds no provider, so nothing here
/// can check `candidate_size` against what the substrate actually showed. A
/// hand-built proposal can still understate it — but doing so is now an explicit
/// false statement rather than an omission, which is the difference between
/// negligence and fraud, and what makes a later audit possible at all. The residual
/// exposure is open question **Q5** (a captured provider feeding a gerrymandered
/// boundary), which D21 does not close.
///
/// Carried into [`PolityCharter`] by [`FoundingProposal::ratify`], so it is
/// **genesis-immutable**: the charter's core has no amendment path, and anyone
/// holding the charter can re-derive [`crate::founding_legitimacy`] from it forever.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryAttestation {
    pub source: BoundarySource,
    /// How many members the boundary this roll was drawn from showed.
    pub candidate_size: usize,
    /// Everyone it showed that the roll does not name, each with a reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclusions: Vec<Exclusion>,
}

impl BoundaryAttestation {
    /// A convener asserting a roll of `size` with nobody excluded — the honest
    /// attestation for a founding with no independent reading behind it.
    ///
    /// This is not a weaker claim smuggled in as a default: it is the *accurate*
    /// one when a convener names a roll directly. What the charter then records is
    /// that nobody but the convener attests this boundary, which
    /// [`crate::founding_legitimacy`] reports as `SelfAttested` forever.
    pub fn convener_asserted(size: usize) -> Self {
        BoundaryAttestation {
            source: BoundarySource::ConvenerAsserted,
            candidate_size: size,
            exclusions: Vec::new(),
        }
    }

    /// A convener asserting exactly this roll, plus `exclusions` they are declaring
    /// out of a wider boundary they say they saw.
    pub fn convener_asserted_excluding(roll_size: usize, exclusions: Vec<Exclusion>) -> Self {
        BoundaryAttestation {
            source: BoundarySource::ConvenerAsserted,
            candidate_size: roll_size + exclusions.len(),
            exclusions,
        }
    }

    /// Whether the accounting adds up: everyone the boundary showed is either on
    /// `roll` or explicitly excluded. The rule genesis authority enforces.
    pub fn accounts_for(&self, roll_size: usize) -> bool {
        self.candidate_size == roll_size + self.exclusions.len()
    }
}

/// A self-binding rule the polity commits to. Founding commitments are
/// genesis-immutable; later commitments are added at a simple majority and
/// removed only at a higher bar — the asymmetry that resists capture by erosion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub key: String,
    pub description: String,
}

/// A **proposed** founding: the candidate boundary and substrate, as read from
/// the emergent structure (a `SubstrateProvider`) or hand-authored, and offered
/// for ratification. It carries **no authority on its own** — that is the
/// "ratify, do not replace" discipline made structural: emergence *proposes*, and
/// only [`FoundingProposal::ratify`] (members co-signing) turns a proposal into a
/// [`PolityCharter`]. A proposal is never appended to the archive; only the
/// ratified charter is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundingProposal {
    /// The polity's root DID — the persistent entity the charter founds.
    pub polity: DidKey,
    pub substrate: Substrate,
    pub founding_commitments: Vec<Commitment>,
    /// The candidate membership roll (the crystallized emergent boundary).
    pub proposed_members: Vec<DidKey>,
    /// Which candidates the substrate reads as **derived** (machine) parties, and
    /// the human each answers to — the guardian edge offered for ratification
    /// alongside the boundary. Empty for a wholly human polity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed_derivations: Vec<Derivation>,
    /// The boundary this roll was drawn from: where it came from, how big it was,
    /// and who it leaves out (**D21**). Required, because a proposal that cannot say
    /// what it narrowed is exactly what Q1 was about.
    pub attestation: BoundaryAttestation,
    pub proposed_at: Timestamp,
}

impl FoundingProposal {
    /// Ratify the proposal into a [`PolityCharter`]. `ratifiers` are the founding
    /// members who consent (they must co-sign the resulting record). This checks
    /// the *local* invariants — ratifiers are distinct, non-empty, and a subset
    /// of the proposed roll — so a charter cannot be minted from thin air. The
    /// *quorum* (a majority of the roll) and *singleton* (one founding per polity)
    /// rules range over the archive and are enforced by the authority pass.
    /// A proposal whose boundary is the convener's own assertion of `members` —
    /// the explicit-roll path, as against drawing one from a substrate candidate.
    ///
    /// Exists so the attestation cannot be forgotten or fudged on the path that has
    /// no provider behind it: the roll a convener names *is* the boundary they saw,
    /// so `candidate_size` is its length and there is nothing to exclude. A convener
    /// who saw more than they are chartering should say so — build the attestation
    /// with [`BoundaryAttestation::convener_asserted_excluding`].
    pub fn convener_asserted(
        polity: DidKey,
        substrate: Substrate,
        founding_commitments: Vec<Commitment>,
        proposed_members: Vec<DidKey>,
        proposed_derivations: Vec<Derivation>,
        proposed_at: Timestamp,
    ) -> Self {
        let attestation = BoundaryAttestation::convener_asserted(proposed_members.len());
        FoundingProposal {
            polity,
            substrate,
            founding_commitments,
            proposed_members,
            proposed_derivations,
            attestation,
            proposed_at,
        }
    }

    pub fn ratify(self, ratifiers: Vec<DidKey>, founded_at: Timestamp) -> KcResult<PolityCharter> {
        if ratifiers.is_empty() {
            return Err(KcError::Authority(
                "ratification requires at least one ratifier".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for r in &ratifiers {
            if !seen.insert(r.as_str()) {
                return Err(KcError::Authority(format!("duplicate ratifier: {r}")));
            }
        }
        let roll: BTreeSet<&str> = self.proposed_members.iter().map(DidKey::as_str).collect();
        for r in &ratifiers {
            if !roll.contains(r.as_str()) {
                return Err(KcError::Authority(format!(
                    "ratifier {r} is not in the proposed membership roll"
                )));
            }
        }
        Ok(PolityCharter {
            polity: self.polity,
            substrate: self.substrate,
            founding_commitments: self.founding_commitments,
            members: self.proposed_members,
            derivations: self.proposed_derivations,
            attestation: self.attestation,
            ratifiers,
            proposed_at: self.proposed_at,
            founded_at,
        })
    }
}

/// The ratified founding charter — a **genesis singleton** for one polity.
/// Produced only via [`FoundingProposal::ratify`]. `members` is the ratified
/// roll (the whole proposed boundary becomes members); `ratifiers` are the
/// members who actively consented and must have co-signed the record. A majority
/// of the roll must ratify (enforced by the authority pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolityCharter {
    pub polity: DidKey,
    pub substrate: Substrate,
    pub founding_commitments: Vec<Commitment>,
    pub members: Vec<DidKey>,
    /// Which founding members are **derived** (machine) parties and the human each
    /// answers to (D15). Ratified with the rest of the charter, so the guardian
    /// edge is a fact the polity asserted about itself rather than a reading taken
    /// of it. Later designations, rotations and releases are `GuardianChange`
    /// records; the effective picture is [`crate::guardian_state`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derivations: Vec<Derivation>,
    /// The boundary this charter ratifies (**D21**, resolving **Q1**) — the
    /// substrate reading or convener assertion it was drawn from, and everyone that
    /// showed whom this roll does not name. Genesis-immutable with the rest of the
    /// charter's core, so `founding_legitimacy` is re-derivable forever.
    pub attestation: BoundaryAttestation,
    pub ratifiers: Vec<DidKey>,
    pub proposed_at: Timestamp,
    pub founded_at: Timestamp,
}

/// Whether a commitment amendment adds (expands) or removes (contracts) a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmendmentKind {
    Add,
    Remove,
}

impl AmendmentKind {
    /// A contraction rolls back a self-binding commitment — the capture-relevant
    /// direction, held to the higher bar.
    pub fn is_contraction(&self) -> bool {
        matches!(self, AmendmentKind::Remove)
    }
}

/// An amendment to a polity's (non-founding) commitments. There is no amendment
/// for the substrate (genesis-immutable) and none for a founding commitment
/// (also genesis-immutable — rejected by the authority pass).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentAmendment {
    pub polity: DidKey,
    pub kind: AmendmentKind,
    pub commitment: Commitment,
    pub rationale: String,
    /// The members deciding the amendment; they must co-sign. The bar depends on
    /// `kind` (majority to Add, majority+1 to Remove).
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// The deliberate winding-up of a polity's charter — the terminal charter-
/// lifecycle event. kith-covenant has no "dissolve" primitive (a covenant party
/// simply goes quiet); dissolution is an **ethne-side** record, because it is the
/// polity's own founding that a fission (or a decided wind-down) closes.
///
/// A dissolution names the `successors` the roll ratified — zero for a plain
/// wind-down, two for a fission — so a reader can follow the polity's line
/// forward. Like an amendment, it is authorized only by a **majority of the
/// current roll** co-signing (the `deciders`); "ratify, do not replace" applies
/// here too — a polity is not dissolved out from under its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharterDissolution {
    pub polity: DidKey,
    /// The polities the roll ratified as this one's line forward (two for a
    /// fission, empty for a plain wind-down).
    pub successors: Vec<DidKey>,
    pub rationale: String,
    /// The members deciding the dissolution; they must co-sign, and a majority of
    /// the current roll is required.
    pub deciders: Vec<DidKey>,
    pub dissolved_at: Timestamp,
}

/// How a [`MembershipChange`] moves the roll. The asymmetry mirrors
/// [`AmendmentKind`]: growing the polity is easier than shrinking it, and — as
/// with the covenant it sits under — a member cannot be conscripted (a join needs
/// the joiner's consent) nor forced to stay (a voluntary exit is the member's own
/// signed act).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MembershipKind {
    /// Admit a new member. Needs a majority of the current roll **and** the
    /// joiner's own co-signature (consent — no conscription).
    Admit,
    /// A member leaves of their own accord. Signed by that member alone; the
    /// roll cannot force them to stay.
    DepartVoluntary,
    /// The roll removes a member against their will. The capture-relevant
    /// contraction, held to the higher bar (a majority **+ 1**), with the target
    /// excluded from the deciders.
    Expel,
}

impl MembershipKind {
    /// A contraction shrinks the roll — the capture-relevant direction, held to
    /// the self-govern floor. (`Admit` grows it.)
    pub fn is_contraction(&self) -> bool {
        matches!(self, MembershipKind::DepartVoluntary | MembershipKind::Expel)
    }
}

/// A change to a polity's membership roll after founding. The founding roll is
/// fixed by the genesis charter; every later admission, voluntary departure, and
/// expulsion is one of these records, so the *current* roll is the founding roll
/// plus admits minus departures, in order. The authority bar depends on `kind`
/// (see [`MembershipKind`]); a contraction is additionally refused when it would
/// leave the roll unable to form the `majority + 1` its own contraction rules
/// require (the self-govern floor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipChange {
    pub polity: DidKey,
    pub kind: MembershipKind,
    /// The member being admitted, departing, or expelled.
    pub member: DidKey,
    pub rationale: String,
    /// The members deciding the change; they must co-sign. Empty for a
    /// `DepartVoluntary` (the departing `member` is the sole signer). For `Admit`
    /// the joiner co-signs *in addition* to the deciders; for `Expel` the target
    /// is excluded from the deciders.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// A conferred internal role. Offices are set by a deliberate, signed act of the
/// roll and held until relinquished — never a readout of earned reputation
/// (design §3). The four canonical roles are named; `Other` lets a polity confer
/// a role of its own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Office {
    /// Presides over the roll and its deliberations.
    Convener,
    /// Presides over the intra-community induction / re-affirmation rite.
    Inductor,
    /// Authorizes the polity's envoys for kith-covenant recognition.
    EnvoyAuthorizer,
    /// Custodian of the charter log.
    CharterKeeper,
    /// A polity-specific office named by the string.
    Other(String),
}

/// How an [`OfficeChange`] moves an office holding. The three-kind shape mirrors
/// [`MembershipKind`], for the same reasons: an office is a duty, so it cannot be
/// conscripted (a conferral needs the holder's consent) nor forced to continue (a
/// holder may resign); and the roll may remove an officer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfficeKind {
    /// Confer the office on a member. Needs a majority of the current roll **and**
    /// the holder's own co-signature (consent).
    Confer,
    /// The holder steps down of their own accord. Signed by the holder alone.
    Resign,
    /// The roll removes the holder from the office. Signed by a majority of the
    /// roll, with the holder excluded from the deciders.
    Revoke,
}

impl OfficeKind {
    /// Whether this ends a holding (`Resign` / `Revoke`) rather than creating one
    /// (`Confer`).
    pub fn is_relinquishment(&self) -> bool {
        matches!(self, OfficeKind::Resign | OfficeKind::Revoke)
    }
}

/// A change to who holds an internal office. The founding roll confers the first
/// offices (Q3: the roll is the standing conferring authority, so no office ever
/// confers another and there is no regress); thereafter every conferral,
/// resignation, and revocation is one of these records. An office holding is
/// **effective only while its holder is on the current roll** — it lapses if the
/// holder departs and revives if they return; the roll ends it for good by
/// `Revoke`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficeChange {
    pub polity: DidKey,
    pub kind: OfficeKind,
    pub office: Office,
    /// The member the office is conferred on, resigned by, or revoked from.
    pub holder: DidKey,
    pub rationale: String,
    /// The members deciding the change; they must co-sign. Empty for a `Resign`
    /// (the `holder` is the sole signer). For `Confer` the holder co-signs *in
    /// addition* (consent); for `Revoke` the holder is excluded.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// One member's reading at one observed instant.
///
/// This deliberately mirrors `en_substrate::MemberCandidate`, and deliberately does
/// **not** reuse it: the dependency runs `en-substrate → en-charter`, so a record in
/// this crate cannot name a provider type. The duplication buys the record its
/// independence from whichever provider produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberReading {
    pub member: DidKey,
    pub reputation: f64,
    /// The guardian edge as *observed* — against the charter's `Derivation`, which
    /// is what the polity ratified. A monitor comparing the two is how a lost edge
    /// is found.
    pub guardian: Option<DidKey>,
    /// When this member's guardian was last verified present, as the substrate
    /// reported it. Carried as a stamp, never a freshness verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attested_at: Option<Timestamp>,
}

/// **What one monitor saw, at one instant** (design **D16**) — the record that gives
/// the polity's log a memory of its own observation, so a measure can speak about
/// *duration* and not only about now.
///
/// An observation **asserts nothing about the polity.** It is one party's
/// perspectival evidence, in the sense kith-covenant's observer reports are: signed
/// by whoever looked, weighted by who that is, and never a ratified fact. Hence no
/// deciders, no majority, and no requirement that the observer be a member — a
/// monitor may perfectly well run outside the roll. What the authority pass does
/// enforce is that the record was signed by exactly the observer it names, so an
/// observation cannot be put in somebody else's mouth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub polity: DidKey,
    /// Who looked. Not necessarily a member.
    pub observer: DidKey,
    pub observed_at: Timestamp,
    /// Every member the observer saw, with its reading. A member absent from this
    /// list was not observed — which is not the same as having no guardian.
    pub readings: Vec<MemberReading>,
    /// Whether the observed boundary was (partly) an intentional opaque edge.
    pub opaque_boundary: bool,
}

/// **A ratified dropping of old observations** (design **D17**) — the record that
/// lets the observation log forget without the polity losing track of *that* it
/// forgot.
///
/// Observations are the one entry class a polity may drop, and the reason is
/// structural rather than a matter of taste: an observation contributes nothing to
/// reconstructed authority state (see `authority::reconstruct`, which matches it
/// and does nothing), so removing one cannot rewrite what the polity decided.
/// Every other record here governs something, and dropping it would.
///
/// What makes this a record rather than a housekeeping sweep is that a Concern
/// crossing to kith-covenant rests on observations. If evidence could vanish
/// silently, the reading count that D16 made the *evidence somebody was watching*
/// would be unfalsifiable, and an audit could not tell "nobody looked" from "we
/// deleted it". So the polity signs for what it drops, and carries a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationPruning {
    pub polity: DidKey,
    /// Observations with `observed_at` **strictly before** this are dropped.
    pub horizon: Timestamp,
    /// How many observation records this pruning drops.
    pub dropped: usize,
    /// The receipt: a digest over the dropped CIDs in append order, via
    /// [`pruning_digest`]. Anyone still holding the originals — an earlier copy of
    /// the log, another member's replica — can recompute it and check that what was
    /// dropped is what the polity said it dropped.
    pub digest: Cid,
    pub rationale: String,
    /// The members deciding the pruning; they must co-sign. One effective
    /// `CharterKeeper` suffices beyond the settling horizon (the delegated power
    /// that office's custody *is*); otherwise a majority of the roll.
    pub deciders: Vec<DidKey>,
    pub pruned_at: Timestamp,
}

/// The receipt over a set of dropped observation CIDs, in append order.
///
/// One definition, used by both the authoring side and the authority pass, so the
/// two can never drift — the same discipline `stale_members` follows for staleness.
pub fn pruning_digest(dropped: &[Cid]) -> KcResult<Cid> {
    let keys: Vec<String> = dropped.iter().map(|c| c.to_string()).collect();
    Cid::of(&keys)
}

/// How a [`GuardianChange`] moves a member's guardian edge. Three kinds, on the
/// pattern of [`OfficeKind`] and [`EnvoyKind`] but for a different reason: a human
/// cannot be conscripted into answerability for a machine, cannot be held to it,
/// and the polity must be able to record honestly that a machine now answers to
/// nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardianKind {
    /// Mark a member a derived (machine) party. Needs a majority of the roll
    /// **and**, where a guardian is named, that guardian's own co-signature —
    /// answerability is accepted, never assigned.
    Designate,
    /// Point a designated member at a (new) guardian — its operator changed, or a
    /// machine chartered with nobody behind it acquires one. This is the operation
    /// on the *edge*, where `Designate` is the operation on the designation, so it
    /// is also how an absent edge is filled. A majority plus the **incoming**
    /// guardian's consent; the outgoing one is released rather than held, since
    /// nobody is forced to keep serving.
    Rotate,
    /// Record that the guardian edge is gone — the machine now answers to nobody.
    /// Either the guardian alone (stepping away) or a majority of the roll
    /// recording the fact that the human is simply no longer there.
    Release,
}

impl GuardianKind {
    /// Whether this ends the guardian edge rather than setting one.
    pub fn is_release(&self) -> bool {
        matches!(self, GuardianKind::Release)
    }
}

/// A change to a member's derived-party designation or its guardian edge — the
/// node→guardian relation's lifecycle after founding (design **D15**). Founding
/// derivations live in the [`PolityCharter`]; every later designation, rotation and
/// release is one of these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianChange {
    pub polity: DidKey,
    pub kind: GuardianKind,
    /// The member whose designation this concerns.
    pub member: DidKey,
    /// The **incoming** guardian: `Some` for a `Rotate`, `Some` or `None` for a
    /// `Designate` (a machine may be chartered with nobody behind it), and always
    /// `None` for a `Release`.
    pub guardian: Option<DidKey>,
    pub rationale: String,
    /// The members deciding the change; they must co-sign. May be empty for a
    /// `Release` signed by the departing guardian alone.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// How an [`EnvoyAuthorization`] moves an envoy standing. The three kinds mirror
/// [`OfficeKind`], for the same reasons: speaking for the polity is a duty, so it
/// cannot be conscripted (an authorization needs the envoy's co-signature) nor
/// forced to continue (an envoy may stand down); and the polity may withdraw the
/// standing it granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvoyKind {
    /// Authorize a member to speak for the polity. Valid on **either** of two
    /// routes: an effective `EnvoyAuthorizer` among the `authorizers` — one
    /// signature, the delegated power the office *is* — or a majority of the roll.
    /// The envoy co-signs in either case (consent).
    Authorize,
    /// The envoy stands down of their own accord. Signed by the envoy alone.
    Resign,
    /// The polity withdraws the standing, on the same two routes as `Authorize`,
    /// with the envoy excluded from the authorizers.
    Revoke,
}

impl EnvoyKind {
    /// Whether this ends a standing (`Resign` / `Revoke`) rather than creating one.
    pub fn is_withdrawal(&self) -> bool {
        matches!(self, EnvoyKind::Resign | EnvoyKind::Revoke)
    }
}

/// Who may speak for the polity beyond it — the act the [`Office::EnvoyAuthorizer`]
/// exists to perform, and the one place an office's authority is *used* rather
/// than merely held (design **D14**). An authorization is a purely ethne-side
/// record in ethne's own vocabulary; `en-uplift` is what turns an active one into
/// kith-covenant's envoy `SpeakerDelegation`, so nothing about KC's delegation
/// scopes reaches this crate.
///
/// An envoy standing is **effective only while the envoy is on the current roll**
/// (it lapses on departure and revives on return, exactly as an office holding
/// does) and only until `expires_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvoyAuthorization {
    pub polity: DidKey,
    pub kind: EnvoyKind,
    /// The member authorized to speak for the polity.
    pub envoy: DidKey,
    /// The envoy's public key. Carried because the KC-side delegation names the
    /// speaker by key, not only by DID.
    pub envoy_pubkey: PubKey,
    pub rationale: String,
    /// The members granting or withdrawing the standing; they must co-sign. Empty
    /// for a `Resign` (the envoy is the sole signer). For `Authorize` the envoy
    /// co-signs *in addition* (consent); for `Revoke` the envoy is excluded.
    pub authorizers: Vec<DidKey>,
    /// When the standing lapses of its own accord, if ever.
    pub expires_at: Option<Timestamp>,
    pub decided_at: Timestamp,
}

/// Whether an affirmation is a member's first acquisition of the substrate or a
/// periodic renewal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AffirmationKind {
    /// A member acquires the substrate for the first time — the induction rite.
    /// Witnessed by a majority of the roll (the community receives them).
    Induction,
    /// A member renews their standing affirmation of the substrate. Self-signed;
    /// its freshness decays, so a community that stops re-affirming shows erosion.
    ReAffirmation,
}

/// A member affirming the polity's substrate — the "transmission" half of
/// longevity (design §5): induction brings a new member into the substrate, and
/// re-affirmation renews it. Admission (a [`MembershipChange`]) puts someone on
/// the roll; **induction is the separate step by which they acquire the
/// substrate**, so a member can be on the roll yet un-inducted. Founding members
/// are inducted implicitly by ratifying the charter (dated `founded_at`), so they
/// need no `Induction` record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Affirmation {
    pub polity: DidKey,
    pub member: DidKey,
    pub kind: AffirmationKind,
    /// For an `Induction`, the roll members who witness (receive) the inductee;
    /// they co-sign alongside the inductee. Empty for a `ReAffirmation` (the
    /// member is the sole signer).
    pub witnesses: Vec<DidKey>,
    pub affirmed_at: Timestamp,
}
