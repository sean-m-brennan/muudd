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
        Archive, CeremonyResult, cid_of, generate_keypair,
        run_two_community_ceremony,
    )
    from kith_covenant.reputation import (
        ReputationView, IntraRiteSummary, InterAdmissionSummary,
        InterNonAdmissionSummary, RejectionSummary, DelegationSummary,
        WitnessSummary, reputation_of,
    )
    __all__ += [
        'Archive', 'CeremonyResult', 'cid_of', 'generate_keypair',
        'run_two_community_ceremony',
        'ReputationView', 'IntraRiteSummary', 'InterAdmissionSummary',
        'InterNonAdmissionSummary', 'RejectionSummary', 'DelegationSummary',
        'WitnessSummary', 'reputation_of',
    ]
except ModuleNotFoundError:
    # kith_covenant._core not built; ceremony.py / reputation.py unavailable.
    pass
