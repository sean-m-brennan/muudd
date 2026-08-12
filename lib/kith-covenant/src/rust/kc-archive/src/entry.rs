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

//! Typed archive entries. One variant per record kind that the protocol
//! produces. Every entry carries a `Signed<...>` envelope; the archive stores
//! the enum and indexes by `Cid`.

use kc_ceremony::inter::{
    ExaminerRegistry, InterCommunityWitnessRecord, NonAdmissionRecord, SponsorshipRecord,
};
use kc_ceremony::intra::IntraCommunityRiteRecord;
use kc_ceremony::lineage::{InterpreterIndependenceAttestation, SubstrateInterpretation};
use kc_commons::{CommonsRegistration, CommonsSanction, UsageRecord};
use kc_conduct::{ActivityAttestation, ConductRecord};
use kc_constitution::{CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment};
use kc_core::{Cid, KcResult, Signed};
use kc_observer::{
    ObserverRegistry, ObserverReport, ReportResponse, SourceExculpation, SourceReveal,
    WhistleblowerReport,
};
use kc_escalation::{
    Designation, InjunctionChallenge, InjunctionConfirmation, InterimInjunction, LadderTransition,
    ReEntry, ReexaminationConclusion, RestitutionCompletion, RestitutionObligation,
    SubstantiveReEngagement,
};
use kc_identity::{CommunityIdentity, MemberRoll, SpeakerDelegation, WitnessRegistry};
use kc_mediation::{
    ArgumentRecord, DecisionUnsealed, DisputeRecord, MediationDecision, MediatorRegistry,
    PanelConsent, ProtectingPower, RecusalDecision, RecusalRequest,
};
use kc_principal::{PrincipalOutput, PrincipalWitnessAttestation};
use kc_provenance::{
    EventActionInvoked, EventCorroboration, EventReport, RetroactiveInterestFinding,
    SkepticismInvocation, SkepticismReview,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveEntry {
    CommunityIdentityPublished(Signed<CommunityIdentity>),
    SpeakerDelegated(Signed<SpeakerDelegation>),
    WitnessRegistryUpdated(Signed<WitnessRegistry>),
    IntraCommunityRite(Signed<IntraCommunityRiteRecord>),
    InterCommunityWitness(Signed<InterCommunityWitnessRecord>),
    InterCommunityNonAdmission(Signed<NonAdmissionRecord>),
    /// An existing member vouches for an entrant (capture_resistance.md §7).
    SponsorshipVouched(Signed<SponsorshipRecord>),
    /// An attributable observation about a community's conduct, including the
    /// jester's pre-indictment `Concern` (kc-conduct).
    ConductAttested(Signed<ConductRecord>),
    /// A community's signed account of one window in a standing function it holds,
    /// including an account of having done nothing (the attested null; §4 and §8).
    ActivityAttested(Signed<ActivityAttestation>),
    /// A community's roster of nominated mediators (§4). The pool is the union
    /// across communities; supersedes by latest `issued_at` per community.
    MediatorRegistryUpdated(Signed<MediatorRegistry>),
    /// A dispute filed by one community against another (§4).
    DisputeFiled(Signed<DisputeRecord>),
    /// One argument submitted on a filed dispute (§4), self-signed by its author.
    ArgumentFiled(Signed<ArgumentRecord>),
    /// A party's request that a named panelist step down from a dispute (§4).
    RecusalRequested(Signed<RecusalRequest>),
    /// The answer to a recusal request (§4).
    RecusalDecided(Signed<RecusalDecision>),
    /// A neutral community standing as protecting power for one disputant (§4).
    ProtectingPowerDesignated(Signed<ProtectingPower>),
    /// A party's consent (or withholding) to the panel as constituted (§4),
    /// self-signed. Gates a `Settled` outcome and nothing else.
    PanelConsentRecorded(Signed<PanelConsent>),
    /// A mediation panel's decision on a dispute (§4), co-signed by the panel.
    MediationDecided(Signed<MediationDecision>),
    /// The opening of a sealed decision's proceedings (§4), co-signed by the custodians
    /// who opened their shares. Must actually recover the sealed text.
    DecisionUnsealed(Signed<DecisionUnsealed>),
    /// A dispute's advance (or de-escalation) along the escalation ladder (§5).
    LadderAdvanced(Signed<LadderTransition>),
    /// A community's designation as anathema (§5), co-signed by the quorum.
    DesignationDecided(Signed<Designation>),
    /// The recorded conclusion of a designation's mandated re-examination (§5).
    ReexaminationConcluded(Signed<ReexaminationConclusion>),
    /// A restitution obligation imposed on a community (§5).
    RestitutionObligated(Signed<RestitutionObligation>),
    /// A verification that a restitution obligation was met (§5), co-signed by a
    /// non-party mediator panel.
    RestitutionVerified(Signed<RestitutionCompletion>),
    /// A community's re-entry from an anathema designation (§5), co-signed by the
    /// admitting quorum.
    ReEntryCompleted(Signed<ReEntry>),
    /// A deliberate invocation of the substantive-re-engagement rung (§5).
    SubstantiveReEngagementInvoked(Signed<SubstantiveReEngagement>),
    /// An interim injunction issued by an emergency mediator panel (§9).
    InjunctionIssued(Signed<InterimInjunction>),
    /// The full compact's confirmation of an injunction (§9), co-signed by quorum.
    InjunctionConfirmed(Signed<InjunctionConfirmation>),
    /// A challenge to an injunction heard by a different panel (§9).
    InjunctionChallenged(Signed<InjunctionChallenge>),
    /// Registration of a compact-level commons (§7), second-class, co-signed by
    /// the registering communities.
    CommonsRegistered(Signed<CommonsRegistration>),
    /// A community's self-reported usage accounting for a commons (§7).
    UsageAccounted(Signed<UsageRecord>),
    /// A step along a commons' graduated-sanction ladder (§7); heavier levels are
    /// co-signed by the imposing quorum.
    SanctionApplied(Signed<CommonsSanction>),
    /// A community's roster of seated examiners (§2). The examining pool is the
    /// union across communities; supersedes by latest `issued_at` per community.
    /// Bounded terms, rotation gaps and pool share are enforced by `ExaminerCaps`,
    /// which shares one implementation with the §8 observer caps.
    ExaminerRegistryUpdated(Signed<ExaminerRegistry>),
    /// A community's roster of nominated observers (§8). The corps is the union
    /// across communities; supersedes by latest `issued_at` per community.
    ObserverRegistryUpdated(Signed<ObserverRegistry>),
    /// A community's member roll — the individual key-holders it names as its own,
    /// typically published from the polity tier's charter. Unlike every other
    /// registry here it **does not supersede**: rolls are unioned, so the insider
    /// set a whistleblower hides in can only ever widen. See [`kc_identity::roll`].
    MemberRollPublished(Signed<MemberRoll>),
    /// An attributed public observer report on a community (§8), signed by a
    /// non-party registered observer.
    ObserverReportPublished(Signed<ObserverReport>),
    /// A subject community's appended response to an observer report (§8).
    ReportResponsePublished(Signed<ReportResponse>),
    /// An insider whistleblower report (§8), co-signed by a mediator reception
    /// panel; carries only a commitment to the source, never the source itself.
    WhistleblowerFiled(Signed<WhistleblowerReport>),
    /// A quorum-gated, for-cause disclosure of a whistleblower's source (§8). The
    /// disclosure must open the filing's commitment with real custody shares.
    SourceRevealed(Signed<SourceReveal>),
    /// A member's proof that a whistleblower filing's key image is not theirs (§8) —
    /// the one §8 remedy a member can exercise without a panel's leave.
    SourceExculpated(Signed<SourceExculpation>),
    /// The founding constitutional artifact (§9): the initial compact-incapable
    /// list and structured-fast cadence, quorum-ratified. Singleton per archive.
    ConstitutionRatified(Signed<ConstitutionGenesis>),
    /// A second-class amendment to the compact-incapable list (§9).
    CompactIncapableAmended(Signed<CompactIncapableAmendment>),
    /// A second-class amendment to the structured-fast cadence (§9).
    FastCadenceAmended(Signed<FastCadenceAmendment>),
    /// A community's attested reading of its own substrate (capture move 1): one
    /// point on a traceable interpretive lineage. Multiple distinct lineages /
    /// interpreters are the plurality the protocol requires; the
    /// `interpretive_plurality` lens surfaces a monopoly.
    SubstrateInterpreted(Signed<SubstrateInterpretation>),
    /// A third party's attestation that a community's declared interpreter is an
    /// independent principal (capture move 1, anti-sockpuppet). The
    /// `interpretive_plurality` lens counts it toward independence only when the
    /// attester is neither the community nor the interpreter.
    InterpreterIndependenceAttested(Signed<InterpreterIndependenceAttestation>),
    /// A claim injected into the compact, provenance-tagged at intake (capture
    /// move 4). The `structured_skepticism` lens surfaces a qualifying event
    /// that is under-corroborated or missing its provenance tags.
    EventReported(Signed<EventReport>),
    /// An independent corroboration of an event report's observable facts by
    /// another community (capture move 4).
    EventCorroborated(Signed<EventCorroboration>),
    /// The mediator pool placing an event under the structured-skepticism protocol on
    /// its own initiative (capture move 4, §4). Co-signed by the invoking panel; this
    /// is what reaches an event whose originator declared it mundane.
    SkepticismInvoked(Signed<SkepticismInvocation>),
    /// The pool's internal review of a hold against pretextual use (§4). A
    /// `Pretextual` finding releases the event, whether the hold was invoked or
    /// self-declared.
    SkepticismReviewed(Signed<SkepticismReview>),
    /// A formal action premised on an event report (capture move 4). The
    /// structured-skepticism hold refuses this while the cited qualifying event is
    /// still within its hold window and under-corroborated.
    EventActionInvoked(Signed<EventActionInvoked>),
    /// A retroactive finding that an event's originator held an undisclosed
    /// interest at intake (capture move 4). The `structured_skepticism` lens
    /// surfaces the event as impugned; the bad-faith consequence is conduct-side.
    RetroactiveInterestFound(Signed<RetroactiveInterestFinding>),
    /// A cross-community attestation of a principal's validity (capture move 6).
    /// The `adversarial_principal` lens counts it only when the witness is
    /// neither the principal nor its home community.
    PrincipalWitnessed(Signed<PrincipalWitnessAttestation>),
    /// A machine output recorded for triangulation against similarly-situated
    /// principals (capture move 6). Divergence toward the principal's own
    /// benefit is the capture signature the lens surfaces.
    PrincipalOutputRecorded(Signed<PrincipalOutput>),
}

impl ArchiveEntry {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CommunityIdentityPublished(_) => "CommunityIdentityPublished",
            Self::SpeakerDelegated(_) => "SpeakerDelegated",
            Self::WitnessRegistryUpdated(_) => "WitnessRegistryUpdated",
            Self::IntraCommunityRite(_) => "IntraCommunityRite",
            Self::InterCommunityWitness(_) => "InterCommunityWitness",
            Self::InterCommunityNonAdmission(_) => "InterCommunityNonAdmission",
            Self::SponsorshipVouched(_) => "SponsorshipVouched",
            Self::ConductAttested(_) => "ConductAttested",
            Self::ActivityAttested(_) => "ActivityAttested",
            Self::MediatorRegistryUpdated(_) => "MediatorRegistryUpdated",
            Self::DisputeFiled(_) => "DisputeFiled",
            Self::ArgumentFiled(_) => "ArgumentFiled",
            Self::RecusalRequested(_) => "RecusalRequested",
            Self::RecusalDecided(_) => "RecusalDecided",
            Self::ProtectingPowerDesignated(_) => "ProtectingPowerDesignated",
            Self::PanelConsentRecorded(_) => "PanelConsentRecorded",
            Self::MediationDecided(_) => "MediationDecided",
            Self::DecisionUnsealed(_) => "DecisionUnsealed",
            Self::LadderAdvanced(_) => "LadderAdvanced",
            Self::DesignationDecided(_) => "DesignationDecided",
            Self::ReexaminationConcluded(_) => "ReexaminationConcluded",
            Self::RestitutionObligated(_) => "RestitutionObligated",
            Self::RestitutionVerified(_) => "RestitutionVerified",
            Self::ReEntryCompleted(_) => "ReEntryCompleted",
            Self::SubstantiveReEngagementInvoked(_) => "SubstantiveReEngagementInvoked",
            Self::InjunctionIssued(_) => "InjunctionIssued",
            Self::InjunctionConfirmed(_) => "InjunctionConfirmed",
            Self::InjunctionChallenged(_) => "InjunctionChallenged",
            Self::CommonsRegistered(_) => "CommonsRegistered",
            Self::UsageAccounted(_) => "UsageAccounted",
            Self::SanctionApplied(_) => "SanctionApplied",
            Self::ExaminerRegistryUpdated(_) => "ExaminerRegistryUpdated",
            Self::ObserverRegistryUpdated(_) => "ObserverRegistryUpdated",
            Self::MemberRollPublished(_) => "MemberRollPublished",
            Self::ObserverReportPublished(_) => "ObserverReportPublished",
            Self::ReportResponsePublished(_) => "ReportResponsePublished",
            Self::WhistleblowerFiled(_) => "WhistleblowerFiled",
            Self::SourceRevealed(_) => "SourceRevealed",
            Self::SourceExculpated(_) => "SourceExculpated",
            Self::ConstitutionRatified(_) => "ConstitutionRatified",
            Self::CompactIncapableAmended(_) => "CompactIncapableAmended",
            Self::FastCadenceAmended(_) => "FastCadenceAmended",
            Self::SubstrateInterpreted(_) => "SubstrateInterpreted",
            Self::InterpreterIndependenceAttested(_) => "InterpreterIndependenceAttested",
            Self::EventReported(_) => "EventReported",
            Self::EventCorroborated(_) => "EventCorroborated",
            Self::SkepticismInvoked(_) => "SkepticismInvoked",
            Self::SkepticismReviewed(_) => "SkepticismReviewed",
            Self::EventActionInvoked(_) => "EventActionInvoked",
            Self::RetroactiveInterestFound(_) => "RetroactiveInterestFound",
            Self::PrincipalWitnessed(_) => "PrincipalWitnessed",
            Self::PrincipalOutputRecorded(_) => "PrincipalOutputRecorded",
        }
    }

    /// Hash the entry itself for archive-level addressing. (This is distinct
    /// from any nested `Signed<T>.payload_cid`, which addresses the payload.)
    pub fn cid(&self) -> KcResult<Cid> {
        Cid::of(self)
    }
}

/// What lets [`kc_log::Archive`] hold these entries — the append-only,
/// content-addressed mechanism this crate is built on (ethne decision **D32**).
///
/// Both methods delegate to the inherent ones above, which stay: an inherent method
/// wins over a trait method in call syntax, so every existing `entry.kind()` and
/// `entry.cid()` call site resolves exactly as it did before.
impl kc_log::Entry for ArchiveEntry {
    fn kind(&self) -> &'static str {
        ArchiveEntry::kind(self)
    }

    fn cid(&self) -> KcResult<Cid> {
        ArchiveEntry::cid(self)
    }
}
