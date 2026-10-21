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

//! The §5 escalation-and-designation payloads and the dispute-ladder ordering.
//!
//! Every payload is data only; it is wrapped in `kc_core::Signed<T>` by its
//! signer(s) and appended to the archive. The structural rules — legal ladder
//! transitions, the four-condition anathema test, the restitution-verifier
//! panel, the re-entry sincerity bar — are enforced in `kc_archive::authority`,
//! because they must range over the whole archive.

use kc_ceremony::Recitation;
use kc_conduct::{DecisionClass, LadderRung};
use kc_core::{Cid, DidKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

// -------------------------------------------------------------------------
// Ladder state machine
// -------------------------------------------------------------------------

/// Rank of a rung on the canonical *dispute* ladder
/// (bounded_conflict.md §"Protocol sketch"): registration → protest →
/// reciprocal suspension → mediated arbitration → limited sanctions →
/// collective sanctions → expulsion. `SubstantiveReEngagement` is the
/// ceremony-cost-decay track, **not** a dispute rung, so it has no rank and can
/// never appear in a [`LadderTransition`]. The ordering lives here rather than
/// on `kc_conduct::LadderRung` because it is escalation semantics, not a bare
/// data label (kc-conduct/record.rs: "escalation semantics live in component 5").
pub fn dispute_ladder_rank(rung: LadderRung) -> Option<u8> {
    use LadderRung::*;
    Some(match rung {
        DisputeRegistered => 0,
        RegisteredProtest => 1,
        ReciprocalSuspension => 2,
        MediatedArbitration => 3,
        LimitedSanctions => 4,
        CollectiveSanctions => 5,
        Expulsion => 6,
        SubstantiveReEngagement => return None,
    })
}

/// The two rungs that are second-class decisions: they impose compact-wide cost
/// and so require a distinct-community quorum and a non-unilateral initiator set
/// (bounded_conflict.md §"Protocol sketch" item 3; commons_federation.md
/// §"Light-lag-honest decision classes").
pub fn is_second_class_rung(rung: LadderRung) -> bool {
    matches!(rung, LadderRung::CollectiveSanctions | LadderRung::Expulsion)
}

/// One step of a dispute along (or back down) the escalation ladder. Append-only:
/// the current rung of a dispute is the `to_rung` of its latest transition. Each
/// transition names the rung it leaves and the rung it enters so the archive can
/// reject out-of-order or skipping moves; de-escalation (a lower `to_rung`) is a
/// first-class, always-legal move (reversibility, bounded_conflict.md
/// §"Reversibility and face-saving exits"). Signed by the `initiators`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LadderTransition {
    /// The dispute this advances; must resolve to a filed `DisputeRecord`.
    pub dispute_cid: Cid,
    pub from_rung: LadderRung,
    pub to_rung: LadderRung,
    /// The communities moving the dispute. For a second-class rung these are the
    /// quorum, and must be at least two distinct communities (non-unilateral).
    pub initiators: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub reasoning: String,
    pub transitioned_at: Timestamp,
}

// -------------------------------------------------------------------------
// Designation (the anathema tier)
// -------------------------------------------------------------------------

/// Which designation tier. `Anathema` is the compact's primary operational
/// category (formal cutting-off, re-entry available). `HostisHumaniGeneris` is
/// the rare, irrecoverable tier — no reversal path, correspondingly higher
/// evidentiary threshold (bounded_conflict.md §"Handling the anathema").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DesignationTier {
    Anathema,
    HostisHumaniGeneris,
}

impl DesignationTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anathema => "Anathema",
            Self::HostisHumaniGeneris => "HostisHumaniGeneris",
        }
    }
}

/// Whether a designation ran the full deliberation cadence or the expedited
/// (emergency) path. `Expedited` collapses to a single deliberation cycle but
/// the archive demands a higher immediate quorum and a mandatory near-term
/// `reexamine_after`; it is forbidden for the `HostisHumaniGeneris` tier. The
/// emergency variant and its misuse-resistance are a design choice this
/// implementation makes; the research leaves the expedited path open
/// (bounded_conflict.md §"Open questions").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DesignationUrgency {
    Normal,
    Expedited,
}

impl DesignationUrgency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Expedited => "Expedited",
        }
    }
}

/// One round of compact-wide deliberation on a proposed designation, with named
/// representatives and published reasoning. Condition 2 requires at least two
/// such cycles separated by a defined waiting period (the Iroquois back-and-forth
/// analogue; bounded_conflict.md §"Handling the anathema").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliberationCycle {
    pub cycle_index: u32,
    pub representatives: Vec<DidKey>,
    pub opened_at: Timestamp,
    pub concluded_at: Timestamp,
    pub reasoning: String,
}

/// A designation of a community as anathema. Second-class; co-signed by the
/// deliberating quorum. Carries the structured evidence the four-condition test
/// (bounded_conflict.md §"Handling the anathema") reads: the failed restoration
/// cycles (condition 1), the deliberation cycles (condition 2), the stated
/// criteria and cited acts (condition 3), and the reversal conditions (condition
/// 4 — present iff the tier is `Anathema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Designation {
    pub subject_community: DidKey,
    pub tier: DesignationTier,
    pub urgency: DesignationUrgency,
    /// The co-signing quorum; the record's principals.
    pub deliberators: Vec<DidKey>,
    /// Condition 3: the specific pattern being named. Must be non-empty; vague
    /// "bad faith" is forbidden.
    pub criteria: String,
    /// Condition 3: specific acts / violations cited (archive CIDs).
    pub cited_acts: Vec<Cid>,
    /// Condition 1: the completed-then-re-violated restoration cycles. Each CID
    /// must resolve to a satisfied `RestitutionCompletion` followed by a later
    /// re-violation; at least three are required.
    pub failed_restoration_cycles: Vec<Cid>,
    /// Condition 2: the deliberation cycles. Normal requires >= 2 separated by a
    /// waiting period; Expedited allows 1.
    pub deliberation_cycles: Vec<DeliberationCycle>,
    /// Condition 4: named conditions for reversal. `Some` for `Anathema`,
    /// `None` for `HostisHumaniGeneris` (no reversal path).
    pub reversal_conditions: Option<String>,
    /// When the designation must be re-examined (generational cadence, or sooner
    /// for the expedited path). Mandatory for `Expedited`.
    pub reexamine_after: Option<Timestamp>,
    pub decision_class: DecisionClass,
    pub designated_at: Timestamp,
}

/// What a re-examination of a standing designation concluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReexaminationOutcome {
    /// The designation stands. Requires a next date: a confirmation renews the
    /// obligation, it does not discharge it.
    Confirmed,
    /// The designation is reversed. Requires the reversal itself to be recorded by the
    /// one path that already exists, so "reversed" keeps a single definition.
    Reversed,
    /// The designation was an **error of the compact** and falls on this record alone.
    ///
    /// Distinct from `Reversed`, and the distinction is what makes the irrecoverable
    /// tier examinable at all. `Reversed` reports a reversal that some other record
    /// already carries, and for an `Anathema` that record is a re-entry or a lifting;
    /// `HostisHumaniGeneris` has neither, so before this variant existed there was no
    /// conclusion a panel re-examining that tier could reach except to confirm it.
    ///
    /// The doctrine names the guards against the compact's own response becoming
    /// demonic as "public deliberation, slow procedure, periodic re-examination,
    /// willingness to be wrong about past designations", and holds them to matter most
    /// for this tier (justus_hostis.md "Design implications"). Willingness to be wrong
    /// needs somewhere to be written down.
    ///
    /// What it does **not** do is open a way back for the designated: vacating says the
    /// compact should not have designated, not that the designated has returned. Re-entry
    /// stays unavailable for `HostisHumaniGeneris` either way.
    Vacated,
}

impl ReexaminationOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "Confirmed",
            Self::Reversed => "Reversed",
            Self::Vacated => "Vacated",
        }
    }
}

/// The recorded conclusion of a designation's mandated re-examination (§5).
///
/// The attested null of the severest act the compact can take. `Designation::reexamine_after`
/// is an absolute deadline, mandatory on the expedited path, and before this record the
/// only thing that could clear it was a *reversal*: a panel that re-examined a designation
/// at the mandated date and let it stand left no trace, so it read exactly like a compact
/// that had forgotten, forever. Diligence and neglect were the same reading, which is the
/// erosion audit's finding K3 and the case its remediation item 5 names as the first slice.
///
/// Two rules keep the record from becoming an evasion of the thing it records. It may not
/// be dated **before** the deadline it answers, or a compact would confirm at designation
/// time and buy permanent silence. And a `Confirmed` outcome must carry the next date, or
/// one confirmation would discharge a generational obligation once and for all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReexaminationConclusion {
    /// The `DesignationDecided` record being re-examined.
    pub designation_cid: Cid,
    pub outcome: ReexaminationOutcome,
    /// Who examined: the co-signing quorum, held to the same distinct-community floor
    /// the designation itself was.
    pub examiners: Vec<DidKey>,
    /// Why it concluded as it did. Required non-empty: a re-examination that says nothing
    /// is the form without the substance, and §5 already holds `DeliberationCycle` and
    /// `MediationDecision` to the same standard.
    pub reasoning: String,
    /// When the designation must next be re-examined. Mandatory for `Confirmed`, and must
    /// be later than `concluded_at`.
    pub next_reexamine_after: Option<Timestamp>,
    pub decision_class: DecisionClass,
    pub concluded_at: Timestamp,
}

/// A member community's recorded refusal to join a designation (§5).
///
/// A designation carries its deliberators and their published reasoning, and nothing
/// else, so a compact of seven in which two designate and five decline reads exactly
/// like one in which all seven did. Measured: the refusal was unrecordable. The archive
/// held no way for a community to say "I considered this and I will not join it", which
/// means the one question a reader most needs answered about the gravest act in the life
/// of a compact — did the compact agree? — could not be answered from the record at all.
///
/// The doctrine wants this legible from both ends. Designation "cannot be unilateral nor
/// can it be by simple majority" and "requires explicit deliberation by all compact
/// members" (`bounded_conflict.md` condition 2), which presumes that a member's position
/// is a thing that exists; and the danger the whole tier is hedged against is a compact
/// talking itself into an enemy, for which the first evidence is always that some of its
/// members would not go along.
///
/// A dissent **vetoes nothing.** The quorum is the quorum, and handing any member a veto
/// over a designation would hand the capturing faction the same veto over being named.
/// What it does is put the split on the record, where `designation_consensus` can read
/// it and the observer corps can weigh it.
///
/// One signer: this is a member speaking for itself, not a decision. Filed by the
/// dissenter, attributed to the dissenter, and irrevocable like any other record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignationDissent {
    /// The `DesignationDecided` record being dissented from; must resolve.
    pub designation_cid: Cid,
    /// The refusing community; the record's principal.
    pub dissenter: DidKey,
    /// Why. Required non-empty, on the same ground as a deliberation cycle's reasoning
    /// and a mediation decision's: a refusal that says nothing is a mood, and the
    /// compact's own account of a disagreement is the part a later reader needs.
    pub reasoning: String,
    pub filed_at: Timestamp,
}

// -------------------------------------------------------------------------
// The withheld proceeding
// -------------------------------------------------------------------------

/// Why a grievance stands unanswered: nothing ever reached it, or something closed it
/// without resolving it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrievanceGround {
    /// No dispute in the archive cites the evidence at all. The compact did not decide
    /// the matter; it never took it up.
    NeverPursued,
    /// A dispute citing the evidence was closed by a decision and the attestor says the
    /// matter is unresolved. The decision is named in `closing_decision`.
    ClosedUnresolved,
}

impl GrievanceGround {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NeverPursued => "NeverPursued",
            Self::ClosedUnresolved => "ClosedUnresolved",
        }
    }
}

/// A member's attestation that evidence on the record has gone unanswered (§5).
///
/// The counterpart of [`DesignationDissent`], on the other side of the same act. A
/// dissent records a refusal to *join* a designation, so a reading over it can say
/// whether the compact agreed. Nothing recorded a refusal to *open* one, and the
/// measurement is that the second gap is the worse of the two: against a complicit
/// majority the honest complainant reached limited sanctions alone and stopped there,
/// and a compact that dismissed the same evidence four times read identically, on every
/// instrument the protocol has, to one that prosecuted it to collective sanctions.
///
/// It decides nothing and alleges no verdict. No protocol can tell a wrong dismissal
/// from a right one, and this does not try. What it records is the **structural** fact
/// that a grievance stands with no proceeding against it, which is checkable — so the
/// authority pass checks it, and an attestation whose claim is false when filed (a
/// dispute is open on that evidence, or the named decision closes no dispute citing it)
/// is refused. The record is therefore not an accusation a faction can mint; it is a
/// claim about the archive that the archive either bears out or does not.
///
/// One signer, first-class: a member speaking for itself. Requiring a quorum to record
/// that the quorum will not act would hand the shielding majority a veto over being
/// noticed, which is the whole of what it wants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonPursuitAttestation {
    /// The adverse conduct records that stand unanswered. Non-empty; each must resolve
    /// to a conduct record whose subject is `subject`.
    pub evidence: Vec<Cid>,
    /// The community the evidence is about.
    pub subject: DidKey,
    /// The member attesting; the record's principal.
    pub attestor: DidKey,
    pub ground: GrievanceGround,
    /// The closing decision, required for `ClosedUnresolved` and refused otherwise.
    pub closing_decision: Option<Cid>,
    /// Why the matter is unresolved. Required non-empty, on the same ground as a
    /// dissent's reasoning: an attestation that says nothing leaves the next reader
    /// exactly where an unrecorded one would.
    pub reasoning: String,
    pub filed_at: Timestamp,
}

// -------------------------------------------------------------------------
// Restitution tracking
// -------------------------------------------------------------------------

/// A restitution obligation imposed on a community, with terms and a beneficiary.
/// The `ConductKind::RestorationCompleted` conduct kind remains a coarse flag;
/// this record plus its [`RestitutionCompletion`] carry the structured
/// satisfied/re-violated signal the designation condition-1 test counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestitutionObligation {
    pub subject_community: DidKey,
    pub beneficiary: DidKey,
    /// The dispute or violation the obligation arises from, if any.
    pub dispute_cid: Option<Cid>,
    pub terms: String,
    /// The communities imposing the obligation; the record's principals.
    pub imposed_by: Vec<DidKey>,
    pub due_by: Option<Timestamp>,
    pub decision_class: DecisionClass,
    pub imposed_at: Timestamp,
}

/// A verification that a restitution obligation was (or was not) satisfied.
/// Verified by a panel of active registered mediators, none from the subject or
/// beneficiary community (the same non-overlap rule as a mediation panel). A
/// `satisfied == false` completion followed by later conduct is the re-violation
/// signal condition 1 counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestitutionCompletion {
    /// Must resolve to a `RestitutionObligation` in the archive.
    pub obligation_cid: Cid,
    /// The verifying mediator panel (each co-signs the envelope).
    pub verifiers: Vec<DidKey>,
    pub satisfied: bool,
    pub narrative: String,
    pub verified_at: Timestamp,
}

// -------------------------------------------------------------------------
// Re-entry
// -------------------------------------------------------------------------

/// A witness's attestation to a re-entry recitation, carrying an explicit
/// sincerity judgement. Distinct from the ordinary intra-rite witness
/// attestation: re-entry demands a *more* demanding sincerity assessment than
/// first admission (exit_and_succession.md §"Re-entry after expulsion").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReEntryWitnessAttestation {
    pub witness_did: DidKey,
    pub recitation_cid: Cid,
    pub sincerity_affirmed: bool,
    pub attested_at: Timestamp,
}

/// A petitioning community's return from an `Anathema` designation. Structurally
/// the recognition ceremony (a fresh five-movement recitation with witnesses
/// assessing sincerity) plus the extra burden of accounting for the conduct that
/// led to the designation and publicly acknowledging it (exit_and_succession.md;
/// bounded_conflict.md §"Re-entry procedure"). Second-class; co-signed by the
/// admitting quorum. Reverses the designation named by `designation_cid` and
/// places the petitioner in a probationary window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReEntry {
    /// The `Anathema` designation being answered (must resolve; the
    /// `HostisHumaniGeneris` tier has no re-entry).
    pub designation_cid: Cid,
    pub petitioner: DidKey,
    /// The fresh five-movement recitation (reused recognition-ceremony machinery).
    pub recitation: Signed<Recitation>,
    /// Account of the conduct that led to designation (re-entry burden).
    pub conduct_accounting: String,
    /// Public acknowledgement that the conduct was wrong (the precondition for
    /// return; without it there is no re-entry).
    pub public_acknowledgement: String,
    pub witnesses: Vec<Signed<ReEntryWitnessAttestation>>,
    /// End of the post-re-entry probation window; a re-entered actor does not
    /// return to clean-slate status.
    pub probation_until: Timestamp,
    /// The admitting quorum; the record's principals.
    pub deliberators: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub completed_at: Timestamp,
}

// -------------------------------------------------------------------------
// Interim injunction (§9 enforcement, capability-gating)
// -------------------------------------------------------------------------

/// Something an injunction can suspend (protocol-components.md §9 "capability-based
/// access"; §7 commons). Membership implies all of them unless an active injunction
/// revokes it.
///
/// Two kinds, and the distinction is the point. A **service** is a relation of
/// exchange: rescue coordination, navigation, trade, commons withdrawal,
/// communication. An **office** is a role that selects who is sanctioned: a
/// mediator-pool seat, an observer or jester seat, an examining seat, deliberative
/// weight, a seat on an emergency panel. The two sets are revoked independently, so a
/// member can be held out of the roles that judge while every relation of exchange it
/// holds stays intact — the option the design wants and could not express while every
/// capability was a service.
///
/// What may trigger the withholding is bounded by the covenant, not by this type: an
/// in-force injunction is the only thing that revokes anything, and an injunction
/// answers to conduct at the boundary. Barring a community from office for what it
/// does at home would be a verdict on a member's interior, which the compact does not
/// form (natural_law.md "The interior of a member community").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Rescue-coordination endpoints (the inviolable rescue duty is separate;
    /// this is the coordination service, not the duty).
    RescueCoordination,
    /// Shared navigation / beacon services.
    Navigation,
    /// Reputation-dependent trade access.
    Trade,
    /// Withdrawals from shared commons pools.
    CommonsWithdrawal,
    /// Access to compact communication relays.
    Communication,
    /// Seating mediators in the compact's pool (§4) — the office that supplies the
    /// panels which decide disputes, injunctions and restitution.
    MediatorSeat,
    /// Nominating observers to the corps (§8), the jester's office included: the corps
    /// carries the jester as a function of the same seat rather than a separate role.
    ObserverSeat,
    /// Seating examiners for anti-spiral-feature credentialing (§2) — the office that
    /// decides who is admitted at all.
    ExaminerSeat,
    /// Deliberative weight in the compact's second-class decisions: a designation, a
    /// contraction of the deference instruments, a charter. Withholding this leaves
    /// membership and every service intact and removes the vote.
    Deliberation,
    /// Supplying a mediator to an emergency panel (§9) — the injunction and
    /// whistleblower-reception panels, which act before any hearing.
    EmergencyPanelSeat,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RescueCoordination => "RescueCoordination",
            Self::Navigation => "Navigation",
            Self::Trade => "Trade",
            Self::CommonsWithdrawal => "CommonsWithdrawal",
            Self::Communication => "Communication",
            Self::MediatorSeat => "MediatorSeat",
            Self::ObserverSeat => "ObserverSeat",
            Self::ExaminerSeat => "ExaminerSeat",
            Self::Deliberation => "Deliberation",
            Self::EmergencyPanelSeat => "EmergencyPanelSeat",
        }
    }

    /// True for the offices — the roles that select who is sanctioned.
    pub fn is_office(&self) -> bool {
        matches!(
            self,
            Self::MediatorSeat
                | Self::ObserverSeat
                | Self::ExaminerSeat
                | Self::Deliberation
                | Self::EmergencyPanelSeat
        )
    }

    /// The relations of exchange.
    pub fn services() -> [Capability; 5] {
        [
            Self::RescueCoordination,
            Self::Navigation,
            Self::Trade,
            Self::CommonsWithdrawal,
            Self::Communication,
        ]
    }

    /// The roles that select who is sanctioned.
    pub fn offices() -> [Capability; 5] {
        [
            Self::MediatorSeat,
            Self::ObserverSeat,
            Self::ExaminerSeat,
            Self::Deliberation,
            Self::EmergencyPanelSeat,
        ]
    }

    /// Every capability — the default set a member holds absent any revocation.
    pub fn all() -> [Capability; 10] {
        [
            Self::RescueCoordination,
            Self::Navigation,
            Self::Trade,
            Self::CommonsWithdrawal,
            Self::Communication,
            Self::MediatorSeat,
            Self::ObserverSeat,
            Self::ExaminerSeat,
            Self::Deliberation,
            Self::EmergencyPanelSeat,
        ]
    }
}

/// A temporary protective "stop" order applied to a community before a full
/// designation is completed (liminal_institutions.md "Proposal: the interim
/// injunction"). Issued by an emergency panel of mediators (co-signed); names the
/// harm and the protective measures, and lists the capabilities it revokes for
/// the duration. Expires at `expires_at` (issued + 90 days) unless the full
/// compact confirms it before then. Enforcement is capability-gating: while the
/// injunction is active (or confirmed) and not overturned, the subject loses the
/// `revoked_capabilities`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterimInjunction {
    pub subject_community: DidKey,
    /// The dispute this arises from, if any (its parties are also non-overlap).
    pub dispute_cid: Option<Cid>,
    pub harm: String,
    pub protective_measures: String,
    pub revoked_capabilities: Vec<Capability>,
    /// The emergency mediator panel that issues it (each co-signs).
    pub panel: Vec<DidKey>,
    pub issued_at: Timestamp,
    /// Lapses here unless confirmed before it (issued + 90 days).
    pub expires_at: Timestamp,
}

/// The full compact's confirmation of an injunction (second-class; co-signed by
/// the compact quorum). Counts only if `confirmed_at <= injunction.expires_at`;
/// a confirmed injunction persists past its expiry until overturned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjunctionConfirmation {
    pub injunction_cid: Cid,
    pub confirmers: Vec<DidKey>,
    pub confirmed_at: Timestamp,
}

/// A challenge to an injunction, heard by a *different* mediator panel than the
/// one that issued it (co-signed). The challenge does not suspend the injunction;
/// `overturned == true` ends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InjunctionChallenge {
    pub injunction_cid: Cid,
    pub panel: Vec<DidKey>,
    pub overturned: bool,
    pub reasoning: String,
    pub decided_at: Timestamp,
}

// -------------------------------------------------------------------------
// Substantive re-engagement (ceremony-cost-decay track)
// -------------------------------------------------------------------------

/// A deliberate invocation of the substantive-re-engagement rung: a heavier
/// re-affirmation cycle demanded of a member community whose ceremony cost has
/// decayed (recognition_protocols.md §"Tracking ceremony cost"; the rung sits
/// between ordinary re-affirmation and anathema). It is a *signed, attributable*
/// act referencing the decay evidence — not an automatic consequence of the
/// read-only `ceremony_cost_decay` lens, which only surfaces the signal (an
/// auto-trigger would impose status with no attributable signer). This is a
/// design choice; the research leaves the enforced-vs-observed question open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstantiveReEngagement {
    pub subject_community: DidKey,
    /// Rite / decay-lens CIDs evidencing the erosion.
    pub decay_evidence: Vec<Cid>,
    /// The communities invoking the rung; the record's principals.
    pub invoked_by: Vec<DidKey>,
    pub required_reaffirmation_by: Timestamp,
    pub decision_class: DecisionClass,
    pub invoked_at: Timestamp,
}
