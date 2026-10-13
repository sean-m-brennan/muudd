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

use std::collections::{BTreeMap, BTreeSet};

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
///
/// **The founding edge must show what stood behind it (design D27).** D15 made every
/// *post-founding* [`GuardianChange`] carry [`GuardianEvidence`]; founding
/// derivations carried nothing, and that was a gate evasion rather than a cosmetic
/// gap. D23 refuses a wholly machine roll that names no ground, but the rule turns on
/// *no edge being guarded* — so one asserted edge, with a single co-signing
/// confederate and nothing observed, exempted the whole cluster. Measured before the
/// slice: three machines, two answering to nobody, one claimed edge, no anchor, both
/// passes green, grading `Guarded`.
///
/// So `evidence` is required exactly when this derivation *claims* an edge, and
/// [`Unbacked`](GuardianEvidence::Unbacked) does **not** count as guarding at
/// founding. That last part is not an inconsistency with the post-founding rule but
/// the reason the post-founding rule is safe: `Unbacked` is sound there because it can
/// only ever weaken a record. Here it would buy exemption from the anchor rule, which
/// would make a self-declaration *strengthening* — the shape D21 and D22 each deleted.
/// A bootstrap cluster whose guardians are real but unobserved names an anchor until a
/// monitor has run, and may then move the edge with a backed [`GuardianChange`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Derivation {
    pub member: DidKey,
    pub guardian: Option<DidKey>,
    /// What the substrate showed behind this edge, or a declared override saying
    /// nothing did. Required when `guardian` is `Some`, refused when it is `None` —
    /// a machine chartered as answering to nobody claims no edge, so there is
    /// nothing for evidence to be evidence *of*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<GuardianEvidence>,
}

impl Derivation {
    /// A derivation claiming no edge: a chartered machine that answers to nobody.
    pub fn unguarded(member: DidKey) -> Self {
        Self { member, guardian: None, evidence: None }
    }

    /// A derivation whose edge the substrate was seen to show.
    pub fn observed(member: DidKey, guardian: DidKey, ob: Signed<Observation>) -> Self {
        Self {
            member,
            guardian: Some(guardian),
            evidence: Some(GuardianEvidence::Observed(Box::new(ob))),
        }
    }

    /// A derivation whose edge nothing backed, saying so. Does not count as guarding
    /// at founding — see the type docs.
    pub fn unbacked(member: DidKey, guardian: DidKey, reason: impl Into<String>) -> Self {
        Self {
            member,
            guardian: Some(guardian),
            evidence: Some(GuardianEvidence::Unbacked { reason: reason.into() }),
        }
    }

    /// Whether this edge is backed by an observation somebody outside the polity
    /// took — the predicate D23's grounded rule and the `Guarded` grade both turn
    /// on, defined once so the two cannot drift.
    ///
    /// **The observer may not be the polity or anyone on its roll.** Measured while
    /// building D27: requiring a signed observation was not enough on its own, since
    /// a founder signed one about the cluster's own edges and the charter graded
    /// `Guarded` with no anchor, both passes green. That is the closure shape D23
    /// refused for the anchor and D25 refused for a successor, reaching the evidence
    /// this time — a polity vouching for its own ground.
    ///
    /// A named **guardian** observing the edge it is about to accept does count, and
    /// that is a judgment call rather than an oversight. A guardian is not on the
    /// roll (a machine cannot guard a machine), it co-signs the founding either way,
    /// and what it asserts by observing is its own answerability — a burden taken up,
    /// not a credit awarded. In that case the keyed human outside the roll *is* the
    /// ground, which is precisely D23's picture of `Guarded`. Interest disqualifies
    /// praise, not a warning, and not an admission (**D24**).
    pub fn is_backed(&self, polity: &DidKey, on_roll: impl Fn(&str) -> bool) -> bool {
        self.guardian.is_some()
            && match &self.evidence {
                Some(GuardianEvidence::Observed(ob)) => {
                    let who = &ob.payload.observer;
                    who != polity && !on_roll(who.as_str())
                }
                _ => false,
            }
    }

    /// This derivation as an effective-state edge, dropping the per-record evidence.
    pub fn edge(&self) -> DerivedEdge {
        DerivedEdge { member: self.member.clone(), guardian: self.guardian.clone() }
    }
}

/// A member's *effective* derived-party edge: who it is, and the human it answers to
/// as of some point in the log.
///
/// Distinct from [`Derivation`] deliberately. A `Derivation` is one **record's** claim
/// and carries the evidence offered for it; an edge is the **state** you get by
/// folding the charter together with every later [`GuardianChange`], where
/// per-record evidence has no meaning. [`GuardianState`](crate::GuardianState) held
/// `Derivation` only because that happened to be the pair type, and carrying an
/// `evidence` field there that is always `None` would be one more field present and
/// read by nothing — the thing D22's `BoundarySource` and D23's `anchor` both were.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedEdge {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Runs one of the polity's first-contact rendezvous relays (design **D36**).
    /// Holding it is what lets a member declare a [`RelayEndpoint`], and the
    /// polity's relay roster lists exactly the holders with a live declaration, so
    /// conferring and revoking this office is how the polity decides who serves.
    RendezvousOperator,
    /// Labels content posted in the polity's name (design **D37**). The roll sets
    /// the [`ContentPolicy`] — what a label *does* — and a holder of this office
    /// applies labels alone, because moderation that waits on a quorum arrives
    /// after the harm. The roll keeps the last word: it may revoke the office at a
    /// majority, and the policy's actions are its own decision, not the holder's.
    Moderator,
    /// Authorizes the polity's software releases (design **D41**). Holding it is
    /// what lets a member sign a [`ReleaseAuthorization`], and the authorization is
    /// what makes a release the polity's: a finding against that release lands on
    /// the officer who authorized it, because answerability is accepted with the
    /// office, never assigned after the fact.
    ReleaseOfficer,
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

/// **What the substrate showed** behind a claimed guardian edge (design **D15**).
///
/// D15 charters the node→guardian edge, and the cohesion monitor measures what the
/// substrate reports *against* that charter — so an edge the substrate never showed
/// leaves the measure comparing a ratified claim to nothing. Before this, exactly
/// that was authorized: five tests moved a guardian edge, none recorded an
/// observation, and the corpus asserted the result correct. The same shape as
/// `substrate.anchor` in D23 and kith-covenant's `source_commitment` — a rule whose
/// evidence nothing supplied.
///
/// **Carried, not cited.** The evidence travels inside the record rather than as a
/// CID pointing at an [`Observation`] in the log, because D17 lets a polity *drop*
/// observations by a signed act and [`apply_prunings`](crate::apply_prunings) really
/// removes them. A citation would leave a record authorized today and unauthorized
/// after legitimate housekeeping — authority would stop being a property of the log
/// and become a property of when you looked. Measured, and kept as
/// `a_cited_observation_would_not_survive_a_legitimate_pruning`. Carrying is the D22
/// pattern (`Substrate` carries its readings for the same reason) and has no such
/// interaction: a copy inside a record cannot be pruned without breaking the
/// record's own signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GuardianEvidence {
    /// The signed observation whose reading for this member showed this guardian.
    /// Its signature is checked by `verify_all` along with the record's own.
    Observed(Box<Signed<Observation>>),
    /// **Nothing backed this edge, and the record says so.** The override, for
    /// bootstrap and offline authoring where no monitor has yet reported.
    ///
    /// Self-declared, and safe only because of an asymmetry: it can never
    /// *strengthen* a record, only disclose that no reading stood behind it. A
    /// faction gains nothing by claiming less than it has, which is why the field
    /// D21/D22 found unenforceable elsewhere is sound here. `reason` may not be
    /// empty — an unexplained override is the thing this exists to prevent.
    Unbacked { reason: String },
}

/// A change to a member's derived-party designation or its guardian edge — the
/// node→guardian relation's lifecycle after founding (design **D15**). Founding
/// derivations live in the [`PolityCharter`]; every later designation, rotation and
/// release is one of these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuardianChange {
    pub polity: DidKey,
    pub kind: GuardianKind,
    /// The member whose designation this concerns.
    pub member: DidKey,
    /// The **incoming** guardian: `Some` for a `Rotate`, `Some` or `None` for a
    /// `Designate` (a machine may be chartered with nobody behind it), and always
    /// `None` for a `Release`.
    pub guardian: Option<DidKey>,
    /// What the substrate showed behind the edge — required exactly when an edge is
    /// *claimed* (a `Designate` naming a guardian, or any `Rotate`) and forbidden
    /// otherwise.
    ///
    /// A `Release` carries none deliberately: it claims an edge is *gone*, and D16
    /// is explicit that a member absent from an observation's readings was not
    /// observed, which is not the same as having no guardian. Absence evidence is a
    /// different object and is not faked here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<GuardianEvidence>,
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
    ///
    /// Since **D34** a renewal carries a **full re-recitation of the five movements**,
    /// as the doctrine requires of it, and is refused without one. Before D34 it was a
    /// bare record, which made the *recurring* ceremony the one with no measurable cost:
    /// a polity could hold its transmission measure green forever without performing the
    /// rite once, and a polity that did perform it produced five authority violations.
    /// It stays **self-signed** — attendance is recorded through the movements and never
    /// required, because a renewal is precisely the act no one else may condition (D14).
    ReAffirmation,
}

/// One movement of the recognition rite (design **D29**).
///
/// The five movements are `recognition_protocols.md`'s, which places them at exactly
/// this tier: *"each candidate member community performs the five-movement rite on its
/// own envoys and representative machines… the five movements carry their full content
/// at this layer"*. Order is semantic, and each variant carries only what its own
/// movement can be held to — so a rule cannot check the wrong field, and a movement
/// that names nothing checkable says so in the type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Movement {
    /// **1 — the naming of the substrate.** `names` must be the chartered substrate's
    /// name: a rite that recites some other ground is not this polity's rite. The
    /// *content* of the naming is the `statement`, which no rule reads.
    Substrate { names: String },
    /// **2 — the acknowledgment of fallenness.** Nothing checkable: no rule can read
    /// contrition, and pretending otherwise would be the hollowest kind of gate.
    Fallenness,
    /// **3 — the recognition of the other.** Also nothing checkable, for the same
    /// reason. At this tier "the other" is the rest of the roll, who are present.
    Recognition,
    /// **4 — the commitment to the common virtues.** `commits_to` must be exactly the
    /// polity's in-force commitment keys at that instant — committing to yesterday's
    /// commitments, or to a subset one finds congenial, is not this commitment.
    CommonVirtue { commits_to: Vec<String> },
    /// **5 — the submission of the polity itself.** `names_anchor` must match the
    /// charter: the named anchor where there is one, and `None` where the charter is
    /// `Bare` (D23's honest state — a polity may have no keyed ground, and the
    /// movement is still performed).
    NonUltimacy { names_anchor: Option<DidKey> },
}

impl Movement {
    /// The movement's place in the sequence, 1–5. Order is semantic: the substrate is
    /// named before anything is said about the people under it.
    pub fn order(&self) -> u8 {
        match self {
            Movement::Substrate { .. } => 1,
            Movement::Fallenness => 2,
            Movement::Recognition => 3,
            Movement::CommonVirtue { .. } => 4,
            Movement::NonUltimacy { .. } => 5,
        }
    }

    /// How many movements a complete rite has.
    pub const COUNT: u8 = 5;
}

/// One movement of one member's induction rite, signed by the inductee and everyone
/// attending it (design **D29**).
///
/// Until D29 the tier recorded an induction's *completion* and nothing else, so a rite
/// performed over a week with an officer presiding and one performed in a second by a
/// majority signing a blank produced identical records. The movements are what the
/// rite actually consists of, and recording them separately is what lets the log show
/// a rite **in progress**, a rite **abandoned**, and how many of the witnesses were
/// there for the whole of it.
///
/// A `Substrate` movement always opens a rite, and opening one abandons whatever was
/// in progress for that member; every other movement must be the immediate successor
/// of the last one recorded. So a rite cannot be interleaved, cannot skip, and cannot
/// deadlock a member forever on an abandoned attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiteMovement {
    pub polity: DidKey,
    /// The member reciting. They recite, so they sign.
    pub member: DidKey,
    /// Which ceremony this movement belongs to (design **D34**): a first acquisition of
    /// the substrate, or its periodic renewal.
    ///
    /// Carried rather than inferred from whether the member happens to be inducted at
    /// this point in the log, so the authority rule can **check** the claim against
    /// their actual state — `Induction` requires that they do not yet hold the
    /// substrate, `ReAffirmation` that they do. A mismatch is then refused instead of
    /// silently reinterpreted, which is the same reason [`Movement`]'s variants carry
    /// only what each can be held to.
    pub kind: AffirmationKind,
    pub movement: Movement,
    /// What was said, in the polity's own terms. Required to be non-empty — a
    /// movement is *performed*, and an empty one was not — but never read: the tier
    /// checks that the rite happened, never whether it was meant.
    pub statement: String,
    /// The members present for this movement, who co-sign it. They must be inducted:
    /// the substrate is transmitted by those who hold it.
    pub attending: Vec<DidKey>,
    pub recited_at: Timestamp,
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
    /// they co-sign alongside the inductee.
    ///
    /// Since **D29** a witness counts only if they attended **every** movement of the
    /// rite. Otherwise a week-long ceremony is five records with disjoint signers and
    /// nobody actually attended it — D28's arithmetic, one record over: count the
    /// people present, not the signatures collected.
    ///
    /// For a `ReAffirmation` since **D34** this records who attended the re-recitation,
    /// and it is **derived, not chosen**: it must equal the rite's attendance-throughout
    /// exactly, so a member can neither over- nor under-report who was there. It may be
    /// empty — nobody's attendance is required — and it is never a threshold. The
    /// renewal's signer is still the member alone.
    pub witnesses: Vec<DidKey>,
    /// The [`RiteMovement`] records this affirmation completes, in order (design
    /// **D29** for an induction, **D34** for a renewal). Five of them, always: a
    /// re-affirmation without a full re-recitation is refused, because that is what the
    /// doctrine means by periodic re-affirmation.
    ///
    /// **Cited, not carried** — the opposite of D15/D27's rule for guardian evidence,
    /// and for the reason that rule exists: a citation is unsafe when what it points
    /// at can be *pruned*, and `is_prunable` matches nothing but an observation. A
    /// movement record cannot vanish, so citing it is sound; and citing is what puts
    /// the rite in the log as it happens, which carrying it inline would hide.
    pub movements: Vec<Cid>,
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

/// Which kind of party an [`IndependenceAttestation`] speaks about (design **D28**).
///
/// The two roles are the same claim about the same kind of party — "is this key its
/// own principal?" — so they share one record and one set of counting rules. What
/// differs is only the coherence gate: each role is checkable against a different
/// part of the log, and a subject who appears in neither is a party the record has
/// never heard of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AttestedRole {
    /// A reader who signed a reading behind the founding boundary (**D22**/**D24**).
    /// Checkable against the charter's own attestation.
    BoundaryReader,
    /// A monitor who has filed an [`Observation`] about this polity, or whose
    /// observation was carried as guardian evidence (**D16**/**D27**). Checkable
    /// against the log up to the filing.
    Observer,
}

impl AttestedRole {
    /// How the role reads in a refusal.
    pub fn noun(self) -> &'static str {
        match self {
            AttestedRole::BoundaryReader => "a reader of",
            AttestedRole::Observer => "an observer of",
        }
    }
}

/// A third party's account of whether a named party behind a founding — a boundary
/// reader, or a monitor whose observations the polity is graded on — is an
/// *independent* principal (design **D24**, generalized to observers by **D28**).
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
/// **Why it covers observers too (D28).** D15 and D27 each closed a hole in guardian
/// evidence and each named *this record* as the successor for what was left — that
/// the evidence is still one observer's perspectival reading. Measured, the successor
/// did not reach: the coherence gate demanded the subject be a *reader of the
/// boundary*, and an observer is not one, so filing about a monitor was refused
/// outright. The same measurement found the sharper case. A confederate observer,
/// on nobody's roll and costing one keypair, signs an observation showing any
/// guardian edge it likes and the machine cluster grades `Guarded` with no anchor —
/// and, filing three times, manufactures the sustained-absence `Concern` that crosses
/// to kith-covenant. **The attack points outward as well as inward**, which the
/// residue's wording never said.
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
pub struct IndependenceAttestation {
    /// The founding whose reader, or the polity whose observer, is being vouched for
    /// or challenged.
    pub polity: DidKey,
    /// The party in question. Must actually appear in the record in the named
    /// [`role`](Self::role) — among the founding's readings for a `BoundaryReader`,
    /// among the observations filed before this one for an `Observer`. An attestation
    /// about a party the log has never carried is incoherent with the record it claims
    /// to be about, and is refused.
    pub subject: DidKey,
    /// Which capacity the subject is being spoken about in. A key may hold both roles
    /// for the same polity, and a voucher for one is not a voucher for the other:
    /// vouching is always for a *party in a capacity*, since what an outsider can
    /// speak to is a relationship, not an identity in the abstract.
    pub role: AttestedRole,
    /// Who is speaking. Must be the sole signer. Counts toward independence only when
    /// it is neither the polity, nor the subject itself, nor anyone on the founding
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

/// A member's own statement of where it serves first-contact rendezvous for the
/// polity (design **D36**), or its withdrawal of that statement.
///
/// **Signed by the member alone.** Where a relay listens is a fact about one
/// machine that only its operator can state, and nobody may put an endpoint in
/// somebody else's mouth, which is the rule an [`Observation`] follows. What the
/// roll decides is *whether* the member serves: the declaration is refused unless
/// its signer holds [`Office::RendezvousOperator`] at `declared_at`. A later
/// revocation or resignation needs no withdrawal: the roster reads office holdings
/// as of its own instant, so the endpoint drops out on its own.
///
/// The key a client pins is not carried here. It is the member's own `did:key`,
/// which for an AT machine is that node's signing key, so a roster cannot name a
/// relay under a key that is not the member's.
///
/// A relay may also serve as an **area hub** (design **D38**): the geohash areas it
/// keeps a directory of nearby people for, listed in `areas`. Empty is a plain
/// rendezvous relay, and serializes as it did before D38, so every earlier
/// declaration keeps its bytes and its signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayEndpoint {
    pub polity: DidKey,
    /// The member stating it, and its sole signer.
    pub member: DidKey,
    /// The member node's AT identity uuid, lowercase and hyphenated: the half of
    /// an AT relay pin that the key does not give.
    pub at_uuid: String,
    /// Where the relay listens. Ignored when `withdrawn`.
    pub host: String,
    pub port: u16,
    pub declared_at: Timestamp,
    /// `true` takes back the member's previous declaration.
    pub withdrawn: bool,
    /// The geohash areas this relay is a hub for (D38): 2 to 5 lowercase base32
    /// characters each, at most [`RELAY_AREAS_MAX`], no repeats. Ignored when
    /// `withdrawn`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub areas: Vec<String>,
}

/// The most areas one relay may serve as a hub (AT's `AT_RELAY_ROSTER_AREAS_MAX`:
/// past it AT refuses the whole roster, so a declaration cannot name more).
pub const RELAY_AREAS_MAX: usize = 8;

/// A geohash area as AT's hubs take one: 2 to 5 characters of lowercase geohash
/// base32 (`0-9`, `b-z` without `a`, `i`, `l`, `o`). Five characters is about 5 km,
/// which is as fine as an area gets: coarse on purpose.
pub fn is_geohash_area(s: &str) -> bool {
    (2..=5).contains(&s.len())
        && s.bytes().all(|c| c.is_ascii_digit() || (c.is_ascii_lowercase() && !matches!(c, b'a' | b'i' | b'l' | b'o')))
}

impl RelayEndpoint {
    /// Whether the declared endpoint can be written into an AT relay pin
    /// (`relay://<uuid>:<fp>@host:port`) unambiguously. A withdrawal carries no
    /// endpoint and always passes. `Err` names the first problem.
    pub fn check_shape(&self) -> Result<(), String> {
        if !is_lower_uuid(&self.at_uuid) {
            return Err(format!("at_uuid {:?} is not a lowercase hyphenated uuid", self.at_uuid));
        }
        if self.withdrawn {
            return Ok(());
        }
        if self.port == 0 {
            return Err("port 0 is not an endpoint".into());
        }
        let bad = |c: char| c.is_whitespace() || c.is_control() || matches!(c, '@' | '/' | '[' | ']' | ',');
        if self.host.is_empty() || self.host.len() > 253 || self.host.chars().any(bad) {
            return Err(format!("host {:?} cannot be written into a relay pin", self.host));
        }
        if self.areas.len() > RELAY_AREAS_MAX {
            return Err(format!("{} areas, and a hub serves at most {RELAY_AREAS_MAX}", self.areas.len()));
        }
        for (i, a) in self.areas.iter().enumerate() {
            if !is_geohash_area(a) {
                return Err(format!("area {a:?} is not 2 to 5 characters of lowercase geohash"));
            }
            if self.areas[..i].contains(a) {
                return Err(format!("area {a:?} is listed twice"));
            }
        }
        Ok(())
    }
}

fn is_lower_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
        })
}

/// What a content label does where a polity's [`ContentPolicy`] applies (design
/// **D37**). Ordered from the loosest to the strictest, so `a < b` reads "`a` does
/// less than `b`": the order is what [`ContentPolicy::is_loosening_of`] measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LabelAction {
    /// Show the content, with the label as a tag.
    Allow,
    /// Show the content behind a warning the reader taps through.
    Warn,
    /// Hide the content, saying which label hid it.
    Hide,
}

/// The longest label a policy may name, in bytes. A label travels in a post's
/// signed canonical form behind a one-byte length (Agora `post.h`), so this is the
/// wire's own limit and not a taste.
pub const CONTENT_LABEL_MAX: usize = 32;

/// The most labels a policy may name. A policy travels to readers as one bounded,
/// signed bundle, and this with [`CONTENT_LABEL_MAX`] is what keeps the worst case
/// inside it: a policy that did not fit would reach nobody.
pub const CONTENT_LABELS_MAX: usize = 32;

/// How content posted in a polity's name is moderated where the polity is heard
/// (design **D37**): the requirements an author must meet, and what each label does.
///
/// A policy is the polity's statement about its **own** space. It binds no reader:
/// a reader applies it because they follow the polity, and may set it aside for
/// themselves. That is what makes moderation here local and per-community rather
/// than global — there is no policy above the polities, and no polity's policy
/// reaches content posted outside it.
///
/// The empty policy ([`ContentPolicy::default`]) is the loosest one, and it is the
/// policy of a polity that has never amended one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentPolicy {
    /// Only an author with an operator bound to their node — a human who answers
    /// for the account — may post in the polity's name.
    pub operator_bound_required: bool,
    /// The lowest trust tier (0..=4) a reader must hold the author at. Each reader
    /// measures it with their own view of the author, as every tier in AT is.
    pub min_tier: u8,
    /// The most posts one author may make in the polity's name in one UTC day, as
    /// each reader counts what it received. `None` is no cap.
    pub daily_post_cap: Option<u32>,
    /// What each label does. A label the policy does not name is allowed.
    pub labels: BTreeMap<String, LabelAction>,
    /// What a post made in **another** community's name does, by that community's
    /// standing in this polity's compacts (design **D39**). A standing the policy
    /// does not name is allowed. Left out when empty, so every D37 policy keeps its
    /// bytes, its signature and its CID.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub standing: BTreeMap<CommunityStanding, LabelAction>,
}

/// Another community's standing as this polity's compacts read it (design **D39**),
/// the key of [`ContentPolicy::standing`]. This tier's own word for it, so the charter
/// keeps no dependency on kith-covenant: the consumer maps KC's reading onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CommunityStanding {
    /// Admitted to a compact this polity is in, and still inside its probation.
    Probationary,
    /// Refused admission by a compact this polity is in.
    Denied,
    /// Designated anathema by a compact this polity is in.
    Anathema,
    /// In none of this polity's compacts.
    Outside,
}

impl ContentPolicy {
    /// Whether this policy lets through anything `prev` held back — the question
    /// that sets the quorum, because loosening a policy takes **more** than a
    /// majority (D11's asymmetry, the rule commitments follow). A change that only
    /// tightens is not a loosening, and neither is no change at all.
    ///
    /// Loosening is any of: dropping the operator requirement, lowering the tier
    /// floor, raising or removing the cap, weakening a label's action, or removing
    /// a label that did more than allow. A change that tightens one rule and
    /// loosens another is a loosening.
    pub fn is_loosening_of(&self, prev: &ContentPolicy) -> bool {
        if prev.operator_bound_required && !self.operator_bound_required {
            return true;
        }
        if self.min_tier < prev.min_tier {
            return true;
        }
        match (prev.daily_post_cap, self.daily_post_cap) {
            (Some(_), None) => return true,
            (Some(p), Some(n)) if n > p => return true,
            _ => {}
        }
        let label_loosened = prev.labels.iter().any(|(label, was)| {
            let now = self.labels.get(label).copied().unwrap_or(LabelAction::Allow);
            now < *was
        });
        // D39: a standing is loosened exactly as a label is.
        label_loosened
            || prev.standing.iter().any(|(standing, was)| {
                let now = self.standing.get(standing).copied().unwrap_or(LabelAction::Allow);
                now < *was
            })
    }

    /// Whether the policy can travel: a tier inside 0..=4, and labels that fit a
    /// post's canonical form. `Err` names the first problem.
    pub fn check_shape(&self) -> Result<(), String> {
        if self.min_tier > 4 {
            return Err(format!("min_tier {} is not a trust tier (0..=4)", self.min_tier));
        }
        if self.labels.len() > CONTENT_LABELS_MAX {
            return Err(format!(
                "{} labels exceed the {CONTENT_LABELS_MAX} a post can carry",
                self.labels.len()
            ));
        }
        for label in self.labels.keys() {
            check_label(label)?;
        }
        Ok(())
    }
}

/// Whether `label` can be written into a post's canonical form and read back as the
/// same label: 1..=[`CONTENT_LABEL_MAX`] bytes of lowercase ASCII letters, digits,
/// `-` and `_`. One spelling per label is what lets an author's label and a
/// policy's name meet without a normalisation step either side could get wrong.
pub fn check_label(label: &str) -> Result<(), String> {
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if label.is_empty() || label.len() > CONTENT_LABEL_MAX || !label.chars().all(ok) {
        return Err(format!(
            "label {label:?} must be 1..={CONTENT_LABEL_MAX} of [a-z0-9_-]"
        ));
    }
    Ok(())
}

/// What the roll decided its polity does in a kith-covenant compact (design **D40**).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompactActKind {
    /// Found or join `compact` together with `counterparties`.
    Join,
    /// Accept `counterparties` into a compact the polity sits in.
    Admit,
    /// Vouch for `counterparties`, staking the polity's standing on their conduct.
    Sponsor,
    /// Refuse `counterparties` admission.
    Refuse,
    /// Report `counterparties` for designation anathema.
    Anathematize,
    /// Lift a designation the polity reported.
    Lift,
}

impl CompactActKind {
    /// The acts that bear down on another community. They take **more** than a
    /// majority, as loosening a policy does (D11's asymmetry, turned outward): a
    /// polity binds itself at a majority, and speaks against another only above one.
    pub fn is_severe(self) -> bool {
        matches!(self, CompactActKind::Refuse | CompactActKind::Anathematize)
    }
}

/// The longest compact label a polity may name, in bytes.
pub const COMPACT_LABEL_MAX: usize = 64;

/// The most counterparties one compact act may name. A ceremony carries at most
/// three participants on Agora's wire, so this is generous.
pub const COMPACT_COUNTERPARTIES_MAX: usize = 8;

/// The roll's decision about a kith-covenant compact (design **D40**). It is the
/// polity's **mandate**: an envoy may sign at KC's border only what an act in force
/// covers, the way D14 makes an envoy speak only while the roll keeps them standing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactAct {
    pub polity: DidKey,
    pub kind: CompactActKind,
    /// The compact's label, as KC's records name it.
    pub compact: String,
    /// The other communities the act concerns.
    pub counterparties: Vec<DidKey>,
    /// For [`CompactActKind::Sponsor`]: how long the stake rides, in days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probation_days: Option<u32>,
    pub rationale: String,
    /// The members deciding it; they must co-sign.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
    /// When the mandate lapses, if it does. A lapsed act was validly decided and
    /// covers nothing after this instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<Timestamp>,
}

impl CompactAct {
    /// Whether the act can be read at KC's border: a label KC can carry, one to
    /// [`COMPACT_COUNTERPARTIES_MAX`] counterparties with no repeats and not the
    /// polity itself, and a probation only on a sponsorship.
    pub fn check_shape(&self) -> Result<(), String> {
        let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
        if self.compact.is_empty()
            || self.compact.len() > COMPACT_LABEL_MAX
            || !self.compact.chars().all(ok)
        {
            return Err(format!(
                "compact {:?} must be 1..={COMPACT_LABEL_MAX} of [a-z0-9_-]",
                self.compact
            ));
        }
        if self.counterparties.is_empty() || self.counterparties.len() > COMPACT_COUNTERPARTIES_MAX {
            return Err(format!(
                "a compact act names 1..={COMPACT_COUNTERPARTIES_MAX} counterparties, got {}",
                self.counterparties.len()
            ));
        }
        let mut seen = BTreeSet::new();
        for c in &self.counterparties {
            if c == &self.polity {
                return Err("a polity is not its own counterparty".into());
            }
            if !seen.insert(c.as_str()) {
                return Err(format!("counterparty {c} named twice"));
            }
        }
        match (self.kind, self.probation_days) {
            (CompactActKind::Sponsor, None) => Err("a sponsorship names its probation".into()),
            (CompactActKind::Sponsor, Some(0)) => Err("a probation of no days stakes nothing".into()),
            (CompactActKind::Sponsor, Some(_)) => Ok(()),
            (_, Some(_)) => Err("only a sponsorship carries a probation".into()),
            (_, None) => Ok(()),
        }
    }

    /// Whether the act is in force at `t`.
    pub fn in_force_at(&self, t: Timestamp) -> bool {
        self.decided_at <= t && self.expires_at.is_none_or(|end| t < end)
    }
}

/// The roll replacing its polity's [`ContentPolicy`] whole (design **D37**).
///
/// Replacement rather than a per-rule delta keeps one question per act: the
/// deciders sign the policy they will live under, and the bar is set by comparing
/// it with the policy it replaces — a majority to tighten, more than a majority to
/// loosen ([`ContentPolicy::is_loosening_of`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentPolicyAmendment {
    pub polity: DidKey,
    pub policy: ContentPolicy,
    pub rationale: String,
    /// The members deciding it; they must co-sign.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// The least a finding must be rated for a [`SteleProfile`] to count it (design
/// **D42**). Ordered from the loosest to the strictest bar, so a higher floor sets
/// more findings aside. There is no `Critical` floor: a confirmed critical finding
/// always counts, whatever profile a consumer applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeverityFloor {
    /// Every finding counts.
    #[default]
    Low,
    /// Findings rated low are concerns, not failures.
    Medium,
    /// Findings rated low or medium are concerns, not failures.
    High,
}

/// Whether a repository's verdict also answers for its team (design **D42**).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamCoupling {
    /// The team's standing is shown beside the repository's and changes nothing.
    #[default]
    Show,
    /// The repository stands no better than its team.
    Worst,
}

/// Whether a repository's verdict also answers for what it is built from: its
/// worst dependency that resolves to a repository Stele reads (design **D42**,
/// the supply-chain view).
///
/// Serialized only when it is [`DepsCoupling::Worst`], so a profile that leaves it
/// at [`DepsCoupling::Show`] has exactly the bytes, signature and CID it had
/// before the field existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DepsCoupling {
    /// Dependencies' standings are shown beside the repository's and change nothing.
    #[default]
    Show,
    /// The repository stands no better than its worst resolved dependency.
    Worst,
}

impl DepsCoupling {
    /// Whether this is the default, which a profile leaves unwritten.
    pub fn is_show(&self) -> bool {
        *self == DepsCoupling::Show
    }
}

/// The longest profile name, in bytes.
pub const STELE_PROFILE_NAME_MAX: usize = 64;
/// The most checks a profile may require, and the most it may set aside.
pub const STELE_PROFILE_CHECKS_MAX: usize = 16;
/// The evidence channels a Stele check is filed under. A replication is filed
/// under the channel of what it replicates, so it is not one of them.
pub const STELE_CHECK_CHANNELS: [&str; 4] = ["probe", "certificate", "self_consistency", "physical"];

/// What "safe enough" means to whoever follows a polity's Stele profile (design
/// **D42**).
///
/// The chain Stele reads records neutral facts with reasons: which verifier ran
/// which scanner on which commit, what it found, and who agreed. A profile is a
/// reading of those facts, never a change to them. Like a [`ContentPolicy`], it
/// binds no reader: a consumer applies it because they follow the polity, and may
/// set it aside for its own.
///
/// It can tighten Stele's reading freely and relax it only inside floors that no
/// profile may cross. [`SteleProfile::check_shape`] refuses one that tries, so a
/// refused profile never reaches a reader at all.
///
/// [`SteleProfile::default`] is Stele's own reading, `stele-default`, and it is
/// the profile of a polity that has never amended one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteleProfile {
    /// Shown beside every verdict read under the profile.
    pub name: String,
    /// How many counted verifiers must agree before a check is confirmed (1..=16).
    pub replicas: u8,
    /// The least AT tier (1..=4) at which a verifier counts, in the reader's roster.
    pub tier: u8,
    /// `scanner:channel` checks that must be present and confirmed clean before a
    /// repository reads as verified safe.
    pub require: BTreeSet<String>,
    /// `scanner:channel` checks the profile sets aside: still shown, never counted.
    pub ignore: BTreeSet<String>,
    /// Findings rated below this are concerns, not failures.
    pub severity_floor: SeverityFloor,
    /// How long a clean reading lasts at full weight, in days (0..=30).
    pub onset_days: u16,
    /// The half-life of a clean reading after that, in days (1..=365). A failure
    /// never fades, under any profile.
    pub half_life_days: u16,
    /// Whether the team's standing caps the repository's.
    pub team: TeamCoupling,
    /// Whether the worst resolved dependency caps the repository's. Absent reads
    /// as [`DepsCoupling::Show`].
    #[serde(default, skip_serializing_if = "DepsCoupling::is_show")]
    pub deps: DepsCoupling,
}

impl Default for SteleProfile {
    fn default() -> Self {
        SteleProfile {
            name: "stele-default".into(),
            replicas: 2,
            tier: 3,
            require: BTreeSet::new(),
            ignore: BTreeSet::new(),
            severity_floor: SeverityFloor::Low,
            onset_days: 1,
            half_life_days: 30,
            team: TeamCoupling::Show,
            deps: DepsCoupling::Show,
        }
    }
}

impl SteleProfile {
    /// Whether this profile lets anything read safer than `prev` would — the
    /// question that sets the quorum, as for a [`ContentPolicy`]: tightening takes
    /// a majority, loosening more than one.
    ///
    /// Loosening is any of: fewer agreeing verifiers, a lower tier, a check no
    /// longer required, a check newly set aside, a higher severity floor, a longer
    /// onset or half-life, or a team or dependency that no longer caps its
    /// repositories. A
    /// change that tightens one rule and loosens another is a loosening. The name
    /// is not a rule.
    pub fn is_loosening_of(&self, prev: &SteleProfile) -> bool {
        self.replicas < prev.replicas
            || self.tier < prev.tier
            || !self.require.is_superset(&prev.require)
            || !self.ignore.is_subset(&prev.ignore)
            || self.severity_floor > prev.severity_floor
            || self.onset_days > prev.onset_days
            || self.half_life_days > prev.half_life_days
            || (prev.team == TeamCoupling::Worst && self.team == TeamCoupling::Show)
            || (prev.deps == DepsCoupling::Worst && self.deps == DepsCoupling::Show)
    }

    /// Whether the profile stays inside Stele's floors and can travel. `Err` names
    /// the first problem.
    ///
    /// The floors: at least one counted verifier, a real AT tier, a severity floor
    /// no higher than [`SeverityFloor::High`] (the type itself has no higher one),
    /// a freshness the reader can still see change, and well-formed checks that are
    /// not both required and set aside.
    pub fn check_shape(&self) -> Result<(), String> {
        let name_ok = |c: char| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.'
        };
        if self.name.is_empty() || self.name.len() > STELE_PROFILE_NAME_MAX || !self.name.chars().all(name_ok) {
            return Err(format!(
                "profile name {:?} must be 1..={STELE_PROFILE_NAME_MAX} of [a-z0-9._-]",
                self.name
            ));
        }
        if !(1..=16).contains(&self.replicas) {
            return Err(format!("replicas {} must be 1..=16", self.replicas));
        }
        if !(1..=4).contains(&self.tier) {
            return Err(format!("tier {} is not a counting AT tier (1..=4)", self.tier));
        }
        if self.onset_days > 30 {
            return Err(format!("onset_days {} exceeds 30", self.onset_days));
        }
        if !(1..=365).contains(&self.half_life_days) {
            return Err(format!("half_life_days {} must be 1..=365", self.half_life_days));
        }
        for (what, set) in [("require", &self.require), ("ignore", &self.ignore)] {
            if set.len() > STELE_PROFILE_CHECKS_MAX {
                return Err(format!(
                    "{what} names {} checks, more than {STELE_PROFILE_CHECKS_MAX}",
                    set.len()
                ));
            }
            for check in set {
                check_stele_check(check)?;
            }
        }
        if let Some(both) = self.require.intersection(&self.ignore).next() {
            return Err(format!("check {both:?} is both required and set aside"));
        }
        Ok(())
    }
}

/// Whether `check` names a Stele check: `scanner:channel`, with a scanner of
/// 1..=32 bytes of `[a-z0-9._-]` and a channel from [`STELE_CHECK_CHANNELS`].
pub fn check_stele_check(check: &str) -> Result<(), String> {
    let ok = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' || c == '.';
    let Some((scanner, channel)) = check.split_once(':') else {
        return Err(format!("check {check:?} is not scanner:channel"));
    };
    if scanner.is_empty() || scanner.len() > 32 || !scanner.chars().all(ok) {
        return Err(format!("check {check:?}: scanner must be 1..=32 of [a-z0-9._-]"));
    }
    if !STELE_CHECK_CHANNELS.contains(&channel) {
        return Err(format!(
            "check {check:?}: channel must be one of {}",
            STELE_CHECK_CHANNELS.join(", ")
        ));
    }
    Ok(())
}

/// The roll replacing its polity's [`SteleProfile`] whole (design **D42**), under
/// the rule a [`ContentPolicyAmendment`] follows: a majority to tighten, more than
/// a majority to loosen ([`SteleProfile::is_loosening_of`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SteleProfileAmendment {
    pub polity: DidKey,
    pub profile: SteleProfile,
    pub rationale: String,
    /// The members deciding it; they must co-sign.
    pub deciders: Vec<DidKey>,
    pub decided_at: Timestamp,
}

/// A release officer's authorization of one commit of one repository as the
/// polity's release (design **D41**).
///
/// **Signed by the officer alone**, and refused unless the officer holds
/// [`Office::ReleaseOfficer`] at `authorized_at`. The roll decides *who* may
/// release, by conferring and revoking the office; the officer decides *what*, and
/// answers for it. A safety finding against the commit lands on the officer as
/// well as on the polity, which is the reason the act is the officer's own and not
/// a majority's: a release decided by everyone would be answered for by no one.
///
/// `built_by` names the member machine that built the release, when one did. Its
/// guardian (design **D15**) then answers for the machine, so a team cannot
/// dissolve accountability by shipping from a box nobody owns. It must be on the
/// roll when the release is authorized.
///
/// The fields are the ones Stele's repository ad carries, and are held to Stele's
/// canonical-form rules (no `|`, no control characters), so the exported
/// authorization can be checked against an ad byte for byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAuthorization {
    pub polity: DidKey,
    /// The authorizing officer, and the sole signer.
    pub officer: DidKey,
    /// The repository's name, as the polity publishes it.
    pub repo: String,
    /// Its clone URL.
    pub url: String,
    /// The released commit, as lowercase hex (40 or 64 characters).
    pub commit: String,
    /// The member machine that built the release, if one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_by: Option<DidKey>,
    pub authorized_at: Timestamp,
}

/// Longest repository name a [`ReleaseAuthorization`] accepts.
pub const RELEASE_REPO_MAX: usize = 128;
/// Longest clone URL a [`ReleaseAuthorization`] accepts.
pub const RELEASE_URL_MAX: usize = 1024;

impl ReleaseAuthorization {
    /// Whether the authorization can be carried in a Stele ad unambiguously.
    /// `Err` names the first problem.
    pub fn check_shape(&self) -> Result<(), String> {
        let bad = |c: char| c == '|' || c.is_control();
        if self.repo.is_empty() || self.repo.len() > RELEASE_REPO_MAX || self.repo.chars().any(bad) {
            return Err(format!("repo {:?} cannot be carried in a release", self.repo));
        }
        if self.url.is_empty() || self.url.len() > RELEASE_URL_MAX || self.url.chars().any(bad) {
            return Err(format!("url {:?} cannot be carried in a release", self.url));
        }
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if !matches!(self.commit.len(), 40 | 64) || !self.commit.chars().all(hex) {
            return Err(format!("commit {:?} is not a lowercase hex object id", self.commit));
        }
        Ok(())
    }
}
