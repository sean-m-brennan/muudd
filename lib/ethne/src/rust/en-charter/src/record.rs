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

use en_core::{Cid, DidKey, KcError, KcResult, PubKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

/// What the polity holds above itself — its non-ultimacy anchor. **Genesis-
/// immutable**: there is deliberately no amendment record that changes a
/// substrate, because a polity that re-decides what it defers to has become a
/// different polity.
///
/// `name` and `statement` must both be **articulated** — non-empty — for a
/// founding to carry authority (design **D23**, resolving open question **Q4**).
/// Before D23 an empty pair chartered: measured, a polity whose entire ground was
/// two empty strings passed both verification passes. What is checked is
/// *presence*, never content: no rule can read a tradition, and this one does not
/// pretend to.
///
/// For a human roll the statement **is** the anchor, and unkeyed on purpose — a
/// tradition, a rule, the dead. `anchor` is the *keyed* ground, and it exists for
/// the derived case: a machine cannot itself hold a tradition, so where nobody on
/// the roll could, the ground must be a key that **consents**. Hence, since D23:
///
/// - an `anchor` may be neither the polity nor anyone on its own roll, exactly as a
///   machine may not guard a machine — a ground inside what it grounds is no ground;
/// - a present `anchor` **co-signs the founding**, because answerability is accepted
///   and never assigned;
/// - a roll of nothing but machines answering to nobody *must* carry one.
///
/// A named anchor may later withdraw ([`SubstrateDisavowal`]), which the archive
/// records and the cohesion monitor reads. It does not change this field: the
/// polity does not get to relabel its ground, and neither does the ground get to
/// erase having stood there.
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

/// One party's signed account of what the substrate showed (design **D22**,
/// resolving open question **Q5**).
///
/// **Why this is signed and named.** D21 recorded *that* a boundary came from a
/// reading; it could not say *whose*, so the label was an unsigned self-report and
/// therefore evidence of nothing. Measured: flipping one public field promoted a
/// convener's bare assertion to `Substrate`, and the grade with it to
/// `Corroborated`, with no provider ever consulted and both verification passes
/// green. A reading nobody signed cannot be told apart from a reading nobody took.
///
/// The reader's key is recovered from its DID (`did:key` self-resolves), so this
/// costs no registry. What it buys is that a convener can no longer *claim* a
/// reading: forging one needs a key they do not hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateReading {
    /// The founding this reading is offered for. Bound deliberately: without it an
    /// honest reader's account of one cluster could be replayed into somebody
    /// else's founding, and the signature would still verify.
    pub polity: DidKey,
    /// Who looked. Mirrors [`Observation::observer`] — the reader need not be a
    /// member, and must be the sole signer of its own reading.
    pub reader: DidKey,
    pub read_at: Timestamp,
    /// Every member the reader saw, as it saw them.
    pub members: Vec<DidKey>,
    /// Whether the observed boundary was (partly) an intentional opaque edge.
    #[serde(default)]
    pub opaque_boundary: bool,
}

impl SubstrateReading {
    /// The members this reading showed, as a set — what the charter's roll and
    /// exclusions are checked against.
    pub fn member_set(&self) -> BTreeSet<&str> {
        self.members.iter().map(DidKey::as_str).collect()
    }
}

/// How the boundary a charter ratifies was established (design **D21**, resolving
/// open question **Q1**; readings added by **D22**, resolving **Q5**).
///
/// The distinction the codebase already made informally now has a home:
/// `en-substrate`'s `ManualFounding` gives asserted members `Some(1.0)` rather than
/// `None` precisely because *a convener vouching for a roll is the reading here*.
/// This says the same thing in the charter, permanently.
///
/// D22 made the `Substrate` arm **carry its evidence** rather than assert it. The
/// variant holds the readings themselves, each signed by the reader it names, so
/// "this boundary was read" is true by construction rather than by claim — there is
/// no way to be `Substrate` without producing at least one signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundarySource {
    /// One or more independent readings, each signed by the reader it names.
    /// Non-empty, enforced at genesis.
    Substrate { readings: Vec<Signed<SubstrateReading>> },
    /// The convener's own assertion — nobody else attests this boundary.
    ConvenerAsserted,
}

impl BoundarySource {
    /// The readings behind this boundary; empty for a convener's assertion.
    pub fn readings(&self) -> &[Signed<SubstrateReading>] {
        match self {
            BoundarySource::Substrate { readings } => readings,
            BoundarySource::ConvenerAsserted => &[],
        }
    }

    /// Whether an independent reading stands behind this boundary at all.
    pub fn is_read(&self) -> bool {
        matches!(self, BoundarySource::Substrate { .. })
    }
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

    /// A boundary drawn from one or more signed readings (design **D22**).
    ///
    /// `candidate_size` is taken from the readings rather than passed in, because
    /// the whole point of D22 is that the count stops being the proposer's word.
    pub fn read(readings: Vec<Signed<SubstrateReading>>, exclusions: Vec<Exclusion>) -> Self {
        let candidate_size = readings.first().map_or(0, |r| r.payload.members.len());
        BoundaryAttestation {
            source: BoundarySource::Substrate { readings },
            candidate_size,
            exclusions,
        }
    }

    /// Whether the accounting adds up: everyone the boundary showed is either on
    /// `roll` or explicitly excluded. The rule genesis authority enforces.
    ///
    /// This is **arithmetic against a self-report**, and D21 could offer nothing
    /// stronger. [`Self::corroborated_by`] is the D22 upgrade: identity against a
    /// signature.
    pub fn accounts_for(&self, roll_size: usize) -> bool {
        self.candidate_size == roll_size + self.exclusions.len()
    }

    /// Whether `reading` actually shows the boundary this charter ratified: its
    /// members are exactly the roll plus the named exclusions, no more and no less
    /// (design **D22**).
    ///
    /// This is the rule with teeth. D21's arithmetic passes for any internally
    /// consistent fabrication, because the proposer supplies both sides of the sum;
    /// here the right-hand side is signed by somebody else.
    pub fn corroborated_by(&self, roll: &[DidKey], reading: &SubstrateReading) -> bool {
        let mut claimed: BTreeSet<&str> = roll.iter().map(DidKey::as_str).collect();
        claimed.extend(self.exclusions.iter().map(|e| e.member.as_str()));
        claimed == reading.member_set()
    }

    /// Every reader whose signed reading agrees with the ratified boundary, in
    /// record order — the honest content of "corroborated" (design **D22**).
    ///
    /// Deliberately *agreement*, not a count of readings: a reader who signed a
    /// different boundary corroborates nothing, and their disagreement is itself
    /// worth reading off the charter.
    pub fn corroborating_readers(&self, roll: &[DidKey]) -> Vec<DidKey> {
        self.source
            .readings()
            .iter()
            .filter(|r| self.corroborated_by(roll, &r.payload))
            .map(|r| r.payload.reader.clone())
            .collect()
    }
}

/// What an excluded party says back (design **D22**).
///
/// D21 named the excluded in someone else's record; this lets them answer. It is
/// **the only thing in the tier that can contradict an understated
/// `candidate_size`** — the residue D21 recorded as beyond its reach, since the
/// archive holds no provider and the excluded are therefore the only witnesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DissentKind {
    /// "The accounting never mentioned me." The dissenter appears neither on the
    /// roll nor among the exclusions, so the boundary they say they were inside was
    /// understated.
    Unnamed,
    /// "I was excluded, and I reject the reason given." The dissenter is named in
    /// the charter's exclusions; the accounting is correct and the justification is
    /// disputed.
    ReasonRejected,
}

/// One party's signed dissent from a founding's boundary (design **D22**).
///
/// Perspectival evidence in the shape D16 established for [`Observation`]: signed
/// by the dissenter alone, **no deciders and no majority**, and the dissenter need
/// not be a member — an excluded party who had to sit inside the roll could be
/// silenced by simply never being admitted.
///
/// A dissent **asserts; it does not adjudicate**. Nothing in the tier decides
/// whether a dissenter is telling the truth, and nothing should: that would be the
/// "replace emergence" move the tier exists to refuse.
///
/// Two properties are deliberate and are the reason this record is worth anything:
///
/// - It may be filed **after dissolution** — the only entry class that may. The
///   alternative is that founders silence their victims by winding the polity up,
///   which is D17's "destroying evidence is the move a captured party makes" wearing
///   a different costume. A founding's legitimacy outlives its polity in any case:
///   kith-covenant holds a `CommunityIdentity` derived from that charter.
/// - It is **unprunable**, and for free: [`crate::is_prunable`] matches only
///   observations, which D17 made the one prunable class precisely because they are
///   inert. A dissent is the opposite of inert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryDissent {
    pub polity: DidKey,
    /// Who is dissenting. Must be the sole signer — nobody may put a dissent in
    /// somebody else's mouth.
    pub dissenter: DidKey,
    pub kind: DissentKind,
    pub rationale: String,
    pub dissented_at: Timestamp,
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

/// The named substrate anchor's signed withdrawal (design **D23**).
///
/// D23 made a keyed anchor a *consenting* party: it co-signs the founding, because
/// answerability is accepted and never assigned. The moment a ground can consent it
/// can also change its mind, and D15 answered exactly that question for guardians
/// with [`GuardianChange`]. This is the same question for the ground itself, and it
/// gets a deliberately weaker answer.
///
/// **A disavowal asserts; it cannot un-charter.** The substrate is
/// genesis-immutable — a polity that re-decides what it defers to has become a
/// different polity — so nothing here rewrites [`Substrate`], reconstructs any
/// authority state, or dissolves anything. What it produces is a legible fact: this
/// polity is now standing on a ground that disowns it. The members' honest options
/// are to live with that or to re-charter, which is what the fission machinery is
/// for. Handing a withdrawal the power to void a charter would let any anchor
/// destroy a polity unilaterally, which is a worse hole than the one D23 closed.
///
/// Unlike a [`BoundaryDissent`], the filer **is checkable**: the charter names its
/// anchor, so only that key may withdraw. A dissent has to admit anyone because an
/// omitted party is by definition not in the record; a ground is in the record.
///
/// Shares the two deliberate properties of a dissent, for the same reasons:
/// - it may be filed **after dissolution**, so founders cannot silence their ground
///   by winding the polity up;
/// - it is **unprunable**, and for free ([`crate::is_prunable`] matches only
///   observations).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateDisavowal {
    pub polity: DidKey,
    /// The withdrawing anchor. Must be the polity's chartered anchor, and must be
    /// the sole signer — nobody may put a withdrawal in the ground's mouth, and
    /// nobody else's withdrawal means anything.
    pub disavower: DidKey,
    pub rationale: String,
    pub disavowed_at: Timestamp,
}

/// Whether an attestation vouches for a reader's independence or denies it (design
/// **D24**).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndependenceKind {
    /// "This reader is its own principal." The vouching that answers D22's residue.
    Independent,
    /// "This reader is not independent of the founders." The counter-claim, so the
    /// lens can be contradicted by somebody who thinks it is being fooled.
    Controlled,
}

/// A third party's account of whether a named reader of a founding is an
/// *independent* principal (design **D24**, closing the residue **D22** named as its
/// own successor).
///
/// **Why this exists.** D22 made a boundary's reading attributable: signed by the
/// reader it names, bound to that founding. What it could not do is tell one reader
/// from five faces of the same actor. Measured: one party minted five `did:key`s,
/// signed five identical readings, and the charter graded `Corroborated` — the
/// highest basis the tier awards — with both verification passes green. The whole
/// chorus cost **32 µs a voice**. `did:key` being free is not a figure of speech, so
/// a count of distinct signing readers cannot mean what it appears to mean.
///
/// kith-covenant hit this exactly, for capture move 1, and its
/// `InterpreterIndependenceAttestation` is the shape borrowed here: an outside party
/// vouches, and the lens counts the vouching rather than the self-declaration.
///
/// **What it does and does not buy.** It does not prove independence, and nothing
/// here adjudicates the claim. It moves the cost of faking corroboration from one key
/// to a second party willing to be **named** for the claim, permanently. That is
/// strictly more than nothing and strictly less than proof, and the counting stays a
/// **lens, never a gate** — a young honest founding has no attestations at all, and
/// refusing it would be refusing newness.
///
/// Like a [`BoundaryDissent`] and a [`SubstrateDisavowal`], this may be filed **after
/// dissolution** and cannot be pruned. Those three now form a category rather than a
/// list of exceptions: records that are **evidence about the founding** outlive the
/// polity, because the founding's legitimacy does — kith-covenant holds a
/// `CommunityIdentity` derived from that charter — where **governing acts** die with
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderIndependenceAttestation {
    /// The founding whose reader is being vouched for or challenged.
    pub polity: DidKey,
    /// The reader in question. Must actually appear among the founding's readings —
    /// an attestation about a reader the charter never carried is incoherent with the
    /// record it claims to be about, and is refused.
    pub reader: DidKey,
    /// Who is speaking. Must be the sole signer. Counts toward independence only when
    /// it is neither the polity, nor the reader itself, nor anyone on the founding
    /// roll — the founders being precisely the interested party. An interested
    /// attestation is still **recorded**, and reported separately: refusing it would
    /// destroy a fact somebody asserted, while crediting it would be the sockpuppet
    /// move one step out.
    pub attesting_party: DidKey,
    pub kind: IndependenceKind,
    /// The basis for the claim (free text, legible to whoever audits it).
    pub basis: String,
    pub attested_at: Timestamp,
}
