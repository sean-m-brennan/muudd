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
        SubstantiveReEngagementSummary,
        reputation_of,
        SponseeOutcome, SponsorStakeReport, CompositionGraph,
        FeatureConcernProfile, composition_graph, sponsor_stake,
        feature_concern_profile,
        RejectionEvent, RejectionPattern, CeremonyCostDecay,
        rejection_pattern, ceremony_cost_decay,
    )
    from kith_covenant.authoring import (
        Signer, new_identity, delegate_speaker, revoke_delegation, vouch,
        record_conduct, record_concern, designate_anathema, lift_designation,
        record_non_admission, run_intra_rite, new_mediator,
        register_mediators, file_dispute, decide_mediation,
        file_ladder_transition, decide_designation, impose_restitution,
        verify_restitution, complete_reentry, invoke_substantive_reengagement,
        now_seconds, DELEGATION_SCOPES, DECISION_OUTCOMES,
        DESIGNATION_TIERS, DESIGNATION_URGENCIES,
    )
    __all__ += [
        'Archive', 'CeremonyResult', 'CaptureScenario', 'cid_of',
        'generate_keypair', 'run_two_community_ceremony', 'demo_capture_scenario',
        'ReputationView', 'IntraRiteSummary', 'InterAdmissionSummary',
        'InterNonAdmissionSummary', 'RejectionSummary', 'DelegationSummary',
        'WitnessSummary', 'ConductSummary', 'SponsorshipSummary',
        'DisputeSummary', 'DesignationSummary', 'ReEntrySummary',
        'RestitutionSummary', 'SubstantiveReEngagementSummary', 'reputation_of',
        'SponseeOutcome', 'SponsorStakeReport', 'CompositionGraph',
        'FeatureConcernProfile', 'composition_graph', 'sponsor_stake',
        'feature_concern_profile',
        'RejectionEvent', 'RejectionPattern', 'CeremonyCostDecay',
        'rejection_pattern', 'ceremony_cost_decay',
        'Signer', 'new_identity', 'delegate_speaker', 'revoke_delegation',
        'vouch', 'record_conduct', 'record_concern', 'designate_anathema',
        'lift_designation', 'record_non_admission', 'run_intra_rite',
        'new_mediator', 'register_mediators', 'file_dispute', 'decide_mediation',
        'file_ladder_transition', 'decide_designation', 'impose_restitution',
        'verify_restitution', 'complete_reentry', 'invoke_substantive_reengagement',
        'now_seconds', 'DELEGATION_SCOPES', 'DECISION_OUTCOMES',
        'DESIGNATION_TIERS', 'DESIGNATION_URGENCIES',
    ]
except ModuleNotFoundError:
    # kith_covenant._core not built; ceremony.py / reputation.py unavailable.
    pass
