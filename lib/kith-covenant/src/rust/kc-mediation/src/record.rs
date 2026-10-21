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

//! The §4 mediation payloads and their classifying enum.

use kc_conduct::LadderRung;
use kc_core::{Cid, DidKey, KcResult, PubKey, RevealedShare, ThresholdCustody, Timestamp};
use serde::{Deserialize, Serialize};

/// One nominated mediator: a key a community vouches as competent to mediate,
/// valid within a window. The nominating community is the enclosing
/// [`MediatorRegistry`], so the non-overlap rule (no mediator from a party
/// community) can always determine a mediator's home community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatorEntry {
    pub mediator_did: DidKey,
    pub mediator_pubkey: PubKey,
    /// The key §8 custody shares are sealed to, distinct from the signing key above.
    ///
    /// A mediator seated on a whistleblower reception panel holds a share of the
    /// source's identity, and sealing that share is a key agreement. Doing it against
    /// the signing key would put one key under two primitives; this is the separate key,
    /// and the authority pass refuses a registration where the two are equal. It is
    /// still an Ed25519 key — the Edwards-form agreement path is unchanged and no new
    /// algorithm enters the compact — but it never signs anything.
    pub encryption_pubkey: PubKey,
    pub valid_from: Timestamp,
    /// When this nomination ends. **Required** by the §4 caps: a seat with no end is a
    /// permanent mediator class, and "a permanent mediator class is the first step
    /// toward an inquisitorial caste" (`liminal_institutions.md` "Discernment
    /// training"). Optional in the type because the same shape serves the observer and
    /// examiner rosters, whose rule refuses `None` in exactly the same place.
    pub valid_until: Option<Timestamp>,
    /// Who trained this mediator, if anyone: the apprenticeship half of §4's
    /// "Discernment training", where prospective mediators "serve under experienced
    /// mediators for an extended period, absorbing judgment through practice".
    ///
    /// **Recorded and gating nothing.** The same passage names the model's weakness —
    /// "susceptible to capture by the incumbent generation's biases" — and the source
    /// documents warn against "a certification barrier that becomes a gatekeeping
    /// mechanism"; the whitepaper lists the training pipeline as an open question
    /// ("Who trains the first generation?"). So no rule reads this to admit or refuse
    /// anyone. It is read only by the lineage lens, which reports when a whole pool
    /// traces to one trainer — making the named weakness legible without turning
    /// lineage into the gate the doctrine refuses to make it.
    ///
    /// `None` means untrained-or-unrecorded, and the two are deliberately not
    /// distinguished: a compact that does not practise apprenticeship is not thereby
    /// irregular.
    pub trained_by: Option<DidKey>,
}

impl MediatorEntry {
    /// Is this mediator nomination live at instant `t`?
    pub fn active_at(&self, t: Timestamp) -> bool {
        self.valid_from <= t && self.valid_until.is_none_or(|end| t < end)
    }
}

/// A community's roster of nominated mediators. Signed by the community root.
/// Registries for the same `nominating_community` supersede by latest
/// `issued_at`; the compact-wide pool is the union across all communities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatorRegistry {
    pub nominating_community: DidKey,
    pub entries: Vec<MediatorEntry>,
    pub issued_at: Timestamp,
}

/// A dispute filed by one community against another, classified by the
/// escalation-ladder rung it is entering (kc-conduct::LadderRung; the full
/// ladder state machine is §5). Signed by the complainant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeRecord {
    pub complainant: DidKey,
    pub respondent: DidKey,
    pub rung: LadderRung,
    pub summary: String,
    /// Prior archive records this dispute draws on (conduct, prior disputes).
    pub references: Vec<Cid>,
    pub filed_at: Timestamp,
}

/// Who is speaking in an [`ArgumentRecord`].
///
/// The two roles are held to different rules because they answer different questions.
/// A party's argument is its own case, and only a party may make it. A panelist's is
/// the position the decision must answer, and only an active non-party mediator may
/// make one — the §4 panel rule, one record over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArgumentRole {
    /// A party to the dispute making its own case.
    Party,
    /// A seated panelist's own position.
    Panelist,
}

impl ArgumentRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Party => "Party",
            Self::Panelist => "Panelist",
        }
    }
}

/// One argument submitted on a filed dispute: §4's "structured argument submission"
/// and "evidence packaging".
///
/// Append-only and self-signed by its author, because an argument is the author's own
/// act and no one else may condition it. The record exists so the compact can be
/// audited for **how** it decided and not only what it decided, which is what the §8
/// observer corps and the §9 erosion signals both assume of it. Before this record the
/// archive held the outcome and a free-text `reasoning` that no rule read, so an
/// observer auditing a decision had nothing to read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentRecord {
    pub dispute_cid: Cid,
    /// The author, who is also the sole signer.
    pub author: DidKey,
    pub role: ArgumentRole,
    /// The position advanced. Required non-empty — an argument is *made*, and an empty
    /// one was not — but never read for content: no rule can weigh an argument, only
    /// record that it was put.
    pub position: String,
    /// What this argument rests on (archive CIDs): §4's evidence packaging.
    pub evidence: Vec<Cid>,
    pub submitted_at: Timestamp,
}

/// A party's request that a named panelist step down from a dispute (§4 "recusal and
/// consent tracking").
///
/// Panel *composition* was already enforced — registered, non-party, minimum count —
/// but a party had no way to say "this one, on this dispute" and have the request
/// recorded and answered. Self-signed by the requesting party.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecusalRequest {
    pub dispute_cid: Cid,
    /// The party asking. Must be the complainant or the respondent.
    pub requested_by: DidKey,
    /// The mediator asked to step down.
    pub mediator: DidKey,
    /// Why. Required non-empty, and never read for content: a rule can record that a
    /// ground was stated, not weigh it.
    pub grounds: String,
    pub requested_at: Timestamp,
}

/// How a [`RecusalRequest`] was answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecusalAnswer {
    /// The mediator steps down. Their own act — nobody else may refuse it for them.
    Granted,
    /// The request is declined, with reasons, by mediators other than the one named.
    Declined,
}

impl RecusalAnswer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Granted => "Granted",
            Self::Declined => "Declined",
        }
    }
}

/// The answer to a recusal request (§4). Append-only: a request that is answered stays
/// answered, and the answer never removes the request.
///
/// The two answers are signed by different people, and that asymmetry is the rule. A
/// **grant** is the named mediator's own act, because stepping down is always available
/// to them and no one else may condition it. A **decline** may not be signed by the
/// mediator whose own recusal is in question — a panelist who could dismiss requests
/// against themselves is the conflict the request exists to raise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecusalDecision {
    pub request_cid: Cid,
    pub answer: RecusalAnswer,
    /// Who answered: the named mediator for a grant, other registered non-party
    /// mediators for a decline.
    pub deciders: Vec<DidKey>,
    /// Required non-empty for a `Declined` answer, since a refusal to step aside is the
    /// one that owes an account. Optional for a grant.
    pub reasoning: String,
    pub decided_at: Timestamp,
}

/// A neutral community standing as **protecting power** for one disputant (§4).
///
/// The Geneva analogue, stated in `liminal_institutions.md` "The protecting-powers
/// analogue": a neutral party that looks after a disputant's interests during the
/// dispute — that the ladder is climbed correctly, that the disputant has access to the
/// mediators, that communications are not interdicted, that the dispute is not a
/// pretext for violating the common virtues.
///
/// **"It is not an arbitrator; it is a witness with standing."** So this record gates
/// nothing: no decision waits on it, no panel is invalid without it. What the rules do
/// enforce is neutrality — the protector may be neither party, and may not also sit on
/// the panel, or the witness and the judge are the same body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectingPower {
    pub dispute_cid: Cid,
    /// The disputant whose interests are being looked after.
    pub disputant: DidKey,
    /// The neutral community standing for them.
    pub protector: DidKey,
    /// The protector accepts the role and the disputant accepts the protector: both
    /// co-sign, because standing for another community is a duty and is never
    /// conscripted, and a protector a disputant did not accept is a watcher imposed on
    /// them.
    pub statement: String,
    pub designated_at: Timestamp,
}

/// Whether a party consents to the panel as constituted (§4 "recusal and consent
/// tracking").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsentAnswer {
    /// The party accepts this panel.
    Given,
    /// The party does not, and says so on the record.
    Withheld,
}

impl ConsentAnswer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Given => "Given",
            Self::Withheld => "Withheld",
        }
    }
}

/// A party's consent to the panel **as constituted** (§4).
///
/// Objection already had a record — a [`RecusalRequest`] names one panelist and is
/// answered — and structural consent already had one, since a [`ProtectingPower`] is
/// co-signed by both sides. What had no record was the ordinary case: a party saying
/// that this panel, these mediators, may decide its dispute. It was inferred from the
/// party having filed an argument, which is not the same act.
///
/// Self-signed, because nobody consents on another's behalf. The panel is named in
/// full and consent does not carry across a change to it: a party that accepted three
/// mediators has not accepted a fourth.
///
/// **Consent does not gate the mediation.** Requiring it to decide would hand either
/// party a veto over its own dispute, and the escalation ladder exists precisely
/// because parties disagree. It gates exactly one thing, where the word means what it
/// says: a [`DecisionOutcome::Settled`] outcome is a claim that *the parties* reached a
/// settlement, so both must have consented to the panel recording it. A party that
/// refuses to settle blocks nothing — the panel upholds, dismisses or refers instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelConsent {
    pub dispute_cid: Cid,
    /// The party speaking, who is also the sole signer. Must be the complainant or the
    /// respondent.
    pub party: DidKey,
    /// The panel as constituted, in full.
    pub panel: Vec<DidKey>,
    pub answer: ConsentAnswer,
    /// Required non-empty for `Withheld`, since withholding is the answer that owes an
    /// account. Never read for content.
    pub statement: String,
    pub given_at: Timestamp,
}

/// How a mediation resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecisionOutcome {
    /// The complaint was sustained.
    Upheld,
    /// The complaint was rejected.
    Dismissed,
    /// The parties reached a mediated settlement.
    Settled,
    /// Referred onward (e.g. to the next rung or the designation track).
    Referred,
}

impl DecisionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upheld => "Upheld",
            Self::Dismissed => "Dismissed",
            Self::Settled => "Settled",
            Self::Referred => "Referred",
        }
    }
}

/// A mediation's proceedings held under threshold encryption, to be opened one
/// generational cycle later (§4 "sealed decisions with threshold-reveal").
///
/// The sealed-proceedings exception to the public-archive principle is "a concession to
/// candor at the expense of transparency" (`jester_and_dissent.md`): parties and
/// panelists say things in a mediation that they will not say into a permanent public
/// record, and a compact that cannot hold anything in confidence gets a shallower
/// mediation rather than a more transparent one.
///
/// **What is sealed is the reasoning, not the decision.** The dispute, the panel and the
/// outcome stay public, so every rule already built on a decision keeps running: the
/// panel is still derived from the co-signers, every panelist still has to have argued,
/// every party argument still has to be answered. A compact can see that it was decided,
/// by whom, and how it came out. What waits is why.
///
/// **Custody is the member communities', and it is not the panel's.** The whitepaper asks
/// for keys "distributed across the mediator pool so no single custodian can unseal
/// early" — but it also asks for the seal to open at the 25-to-30-year mark, and §4 caps
/// a mediator's seat at two years to keep a permanent mediator class from forming. A
/// share sealed to a mediator would be held by a key the archive stops resolving two
/// decades before the seal opens. So the shares go to communities, which resolve at any
/// instant, and the authority pass keeps the substance: no party may hold one, and no
/// quorum of them may sit with the deciding panel's own communities. A panel that could
/// unseal itself has sealed nothing — it has merely deferred publication at its own
/// discretion, which is the opposite of the guarantee that makes candor possible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedReasoning {
    /// A commitment to the reasoning text: [`SealedReasoning::commitment`] of it. What a
    /// [`DecisionUnsealed`] must open. The archive never holds the text in the clear
    /// until the seal is opened, and the commitment is what stops the opening panel
    /// substituting a more flattering account for what was actually written.
    pub reasoning_commitment: Cid,
    /// The reasoning text, sealed so that a quorum of custodians — and only a quorum —
    /// can recover it.
    pub custody: ThresholdCustody,
    /// When the seal opens without cause. One generational cycle: the whitepaper's "25
    /// to 30 year mark", enforced as a range by the authority pass rather than carried
    /// as a number nothing reads.
    pub reveal_at: Timestamp,
}

impl SealedReasoning {
    /// The commitment a sealed decision must carry for `reasoning`.
    pub fn commitment(reasoning: &str) -> KcResult<Cid> {
        Cid::of(&reasoning.to_string())
    }
}

/// The opening of a sealed decision's proceedings (§4). Co-signed by the custodians who
/// opened their shares.
///
/// Mirrors §8's `SourceReveal`, and for the same reason: a quorum rule about who signs
/// is not a guarantee about what is disclosed. The shares must actually recover the
/// text, and the text must open the decision's commitment, so a custodian quorum cannot
/// publish a reasoning the panel never wrote.
///
/// Opening **before** `reveal_at` needs a named cause, exactly as a source reveal does.
/// After it, the seal has expired on its own terms and the opening is bookkeeping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionUnsealed {
    /// The `MediationDecided` entry whose proceedings are opened.
    pub decision_cid: Cid,
    /// The custodians opening it, each of whom co-signs. **Derived, not chosen**: the
    /// authority pass requires this to be exactly the co-signers.
    pub custodians: Vec<DidKey>,
    /// Required non-empty when opened before `reveal_at`; optional after.
    pub cause: String,
    /// The proceedings, in the clear at last. Kept as text so the archive is readable
    /// without doing crypto; the authority pass requires it to be what `shares` recover.
    pub disclosed_reasoning: String,
    /// Custody shares opened by their holders, at least `threshold_k` of them.
    pub shares: Vec<RevealedShare>,
    pub unsealed_at: Timestamp,
}

/// A panel's decision on a dispute (referenced by CID). Co-signed by the panel
/// mediators; the archive authority layer enforces that the panel is >= 3
/// registered mediators, none from a party community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediationDecision {
    pub dispute_cid: Cid,
    /// The mediator DIDs on the deciding panel.
    ///
    /// **Derived, not chosen**: the authority pass requires this to be exactly the
    /// envelope's co-signers. It is restated on the payload for a reader who has the
    /// record without the envelope to hand, the same reason
    /// `ObserverReport::declared_perspective` restates the registry's standpoint — but
    /// until the rule existed nothing read it, so it could name the two *party*
    /// communities while three legitimate non-party mediators signed.
    pub panel: Vec<DidKey>,
    pub outcome: DecisionOutcome,
    /// Why. Required non-empty **unless the proceedings are sealed**: §4's reasoning
    /// trail is *mandatory* (protocol-components.md §4 "Decision publication"), a
    /// requirement the record carried and no rule enforced. When `sealed` is present
    /// this must be empty, so a decision carries exactly one reasoning trail and a
    /// public summary can never contradict the sealed text. A panel wanting to say
    /// something publicly files it as a `Panelist` [`ArgumentRecord`].
    pub reasoning: String,
    /// The sealed proceedings, if this decision is sealed rather than published (§4
    /// "sealed decisions with threshold-reveal").
    pub sealed: Option<SealedReasoning>,
    /// The [`ArgumentRecord`]s this decision answers, by CID.
    ///
    /// Every co-signing panelist must have authored at least one cited `Panelist`
    /// argument, or three signatures ride on one author's text — the same arithmetic as
    /// the witness rules: count the people who deliberated, not the signatures
    /// collected. Every `Party` argument already in the log must be cited too, so a
    /// panel cannot decide while ignoring a case put to it.
    pub arguments: Vec<Cid>,
    pub decided_at: Timestamp,
}
