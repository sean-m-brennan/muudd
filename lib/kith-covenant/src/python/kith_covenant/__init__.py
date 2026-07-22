#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

"""Kith-Covenant protocol interface library."""

from kith_covenant.interface import (
    PeerAdmitted, ReputationUpdate, NegotiationStarted,
    NegotiationOutcome, EvaluatorResult, KithCovenantEvaluatorProtocol,
)
from kith_covenant.policy import (
    KithCovenantPolicy, null_policy, permissive_policy, strict_policy,
)

__all__ = [
    'PeerAdmitted', 'ReputationUpdate', 'NegotiationStarted',
    'NegotiationOutcome', 'EvaluatorResult', 'KithCovenantEvaluatorProtocol',
    'KithCovenantPolicy', 'null_policy', 'permissive_policy', 'strict_policy',
]

# Protocol bindings (Rust core via PyO3). Optional import — AT seam above
# works without the bindings compiled.
try:
    from kith_covenant.ceremony import (
        Archive, CeremonyResult, CaptureScenario, cid_of, generate_keypair,
        run_two_community_ceremony, demo_capture_scenario,
    )
    from kith_covenant.reputation import (
        ReputationView, IntraRiteSummary, InterAdmissionSummary,
        InterNonAdmissionSummary, RejectionSummary, DelegationSummary,
        WitnessSummary, ConductSummary, SponsorshipSummary, DisputeSummary,
        DesignationSummary, ReEntrySummary, RestitutionSummary,
        SubstantiveReEngagementSummary, InjunctionSummary,
        CommonsSanctionSummary, reputation_of, has_capability, commons_level,
        SponseeOutcome, SponsorStakeReport, CompositionGraph,
        FeatureConcernProfile, composition_graph, sponsor_stake,
        feature_concern_profile,
        RejectionEvent, RejectionPattern, CeremonyCostDecay,
        rejection_pattern, ceremony_cost_decay,
        InterpretivePluralityReport, interpretive_plurality,
        EventProvenance, SkepticismReport, structured_skepticism,
        CrossWitness, OutputDivergence, PrincipalValidationReport,
        adversarial_principal,
        UsageTally, FreeRiderReport, free_rider_signals,
        ObserverReportSummary, ReportResponseSummary, WhistleblowerSummary,
        ObserverDashboard, observer_dashboard,
        ServingObserver, ObserverPoolConcentration, observer_pool_concentration,
        IncapableMatter, FastPeriod, ConstitutionState, FastDeviation,
        SubstrateDeferenceReport, constitution_state, compact_incapable,
        in_structured_fast, substrate_deference,
    )
    from kith_covenant.harness import (
        ScenarioResult, HarnessReport, run_harness,
    )
    from kith_covenant.authoring import (
        Signer, new_identity, delegate_speaker, revoke_delegation, vouch,
        attest_interpretation, attest_interpreter_independence,
        report_event, corroborate_event,
        invoke_event_action, find_undisclosed_interest,
        attest_principal, record_principal_output,
        record_conduct, record_concern, designate_anathema, lift_designation,
        record_non_admission, run_intra_rite, new_mediator,
        register_mediators, file_dispute, decide_mediation,
        file_ladder_transition, decide_designation, impose_restitution,
        verify_restitution, complete_reentry, invoke_substantive_reengagement,
        issue_injunction, confirm_injunction, challenge_injunction,
        register_commons, record_usage, apply_sanction,
        new_observer, register_observers, publish_observer_report,
        respond_to_report, file_whistleblower, reveal_source,
        ratify_constitution, amend_compact_incapable, amend_fast_cadence,
        now_seconds, DELEGATION_SCOPES, DECISION_OUTCOMES,
        DESIGNATION_TIERS, DESIGNATION_URGENCIES, CAPABILITIES,
        COMMONS_KINDS, SANCTION_LEVELS, AMENDMENT_KINDS, FAST_CHANGES,
    )
    __all__ += [
        'Archive', 'CeremonyResult', 'CaptureScenario', 'cid_of',
        'generate_keypair', 'run_two_community_ceremony', 'demo_capture_scenario',
        'ReputationView', 'IntraRiteSummary', 'InterAdmissionSummary',
        'InterNonAdmissionSummary', 'RejectionSummary', 'DelegationSummary',
        'WitnessSummary', 'ConductSummary', 'SponsorshipSummary',
        'DisputeSummary', 'DesignationSummary', 'ReEntrySummary',
        'RestitutionSummary', 'SubstantiveReEngagementSummary', 'InjunctionSummary',
        'CommonsSanctionSummary', 'reputation_of', 'has_capability', 'commons_level',
        'SponseeOutcome', 'SponsorStakeReport', 'CompositionGraph',
        'FeatureConcernProfile', 'composition_graph', 'sponsor_stake',
        'feature_concern_profile',
        'RejectionEvent', 'RejectionPattern', 'CeremonyCostDecay',
        'rejection_pattern', 'ceremony_cost_decay',
        'InterpretivePluralityReport', 'interpretive_plurality',
        'EventProvenance', 'SkepticismReport', 'structured_skepticism',
        'CrossWitness', 'OutputDivergence', 'PrincipalValidationReport',
        'adversarial_principal',
        'UsageTally', 'FreeRiderReport', 'free_rider_signals',
        'ObserverReportSummary', 'ReportResponseSummary', 'WhistleblowerSummary',
        'ObserverDashboard', 'observer_dashboard',
        'ServingObserver', 'ObserverPoolConcentration', 'observer_pool_concentration',
        'IncapableMatter', 'FastPeriod', 'ConstitutionState', 'FastDeviation',
        'SubstrateDeferenceReport', 'constitution_state', 'compact_incapable',
        'in_structured_fast', 'substrate_deference',
        'ScenarioResult', 'HarnessReport', 'run_harness',
        'Signer', 'new_identity', 'delegate_speaker', 'revoke_delegation',
        'vouch', 'attest_interpretation', 'attest_interpreter_independence',
        'report_event', 'corroborate_event',
        'invoke_event_action', 'find_undisclosed_interest',
        'attest_principal', 'record_principal_output',
        'record_conduct', 'record_concern', 'designate_anathema',
        'lift_designation', 'record_non_admission', 'run_intra_rite',
        'new_mediator', 'register_mediators', 'file_dispute', 'decide_mediation',
        'file_ladder_transition', 'decide_designation', 'impose_restitution',
        'verify_restitution', 'complete_reentry', 'invoke_substantive_reengagement',
        'issue_injunction', 'confirm_injunction', 'challenge_injunction',
        'register_commons', 'record_usage', 'apply_sanction',
        'new_observer', 'register_observers', 'publish_observer_report',
        'respond_to_report', 'file_whistleblower', 'reveal_source',
        'ratify_constitution', 'amend_compact_incapable', 'amend_fast_cadence',
        'now_seconds', 'DELEGATION_SCOPES', 'DECISION_OUTCOMES',
        'DESIGNATION_TIERS', 'DESIGNATION_URGENCIES', 'CAPABILITIES',
        'COMMONS_KINDS', 'SANCTION_LEVELS', 'AMENDMENT_KINDS', 'FAST_CHANGES',
    ]
except ModuleNotFoundError:
    # kith_covenant._core not built; ceremony.py / reputation.py unavailable.
    pass
