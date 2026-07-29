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

"""Ethne — the polity tier, from Python.

Ethne turns a set of cooperating agents into a **polity**: a bounded,
self-governing community that stands for a substrate, acts with one voice, and
persists across the turnover of its members. This package is a thin Pythonic
surface over the Rust tier (`ethne._core`, built from `en-pyo3` by maturin).

Four modules, following the tier's own shape:

- `ethne.polity` — the charter log (`Archive`), keys, and the derived read-models.
- `ethne.authoring` — `Signer` plus one call per charter record: found, amend,
  admit / depart / expel, confer / resign / revoke an office, designate / rotate /
  release a guardian edge, authorize / resign / revoke an envoy, induct / re-affirm,
  record an observation, dissolve.
- `ethne.monitor` — the substrate side: a `FoundingCandidate` and the
  `cohesion_report` that grades drift away from the chartered roll.
- `ethne.uplift` — the four artifacts that cross to kith-covenant.

The governing discipline shows up in the API: a proposal carries no authority, so
`propose_charter` only returns JSON and `ratify_charter` needs a majority of the
roll to co-sign it. Emergence proposes; the polity ratifies; KC recognizes.
"""

from ethne.polity import (
    AMENDMENT_KINDS,
    DEFAULT_RETENTION,
    EROSION_LEVELS,
    OFFICES,
    AffirmationState,
    Archive,
    CharterState,
    BOUNDARY_SOURCES,
    Cursor,
    DecayTrends,
    EnvoyState,
    FOUNDING_BASES,
    SUBSTRATE_GROUNDINGS,
    FileStore,
    FoundingLegitimacy,
    GroundStanding,
    GuardianState,
    ObservationWindow,
    OfficeState,
    PruneRecommendation,
    RetentionPolicy,
    SaveOutcome,
    cid_of,
    generate_keypair,
    majority,
    now_seconds,
    propose_charter,
)
from ethne.authoring import (
    AFFIRMATION_KINDS,
    ENVOY_KINDS,
    GUARDIAN_KINDS,
    MEMBERSHIP_KINDS,
    OFFICE_KINDS,
    Signer,
    admit_member,
    amend_commitment,
    authorize_envoy,
    confer_office,
    depart_member,
    designate_derived,
    dissolve_charter,
    expel_member,
    induct_member,
    ratify_charter,
    reaffirm_member,
    prune_observations,
    disavow_substrate,
    dissent_boundary,
    record_observation,
    release_guardian,
    resign_envoy,
    resign_office,
    revoke_envoy,
    revoke_office,
    rotate_guardian,
)
from ethne.monitor import (
    DEFAULT_THRESHOLDS,
    CohesionReport,
    FoundingCandidate,
    Thresholds,
    cohesion_report,
    founding_candidate,
    manual_founding,
    propose_from_candidate,
    select_from_candidate,
    sign_reading,
)
from ethne.uplift import (
    FissionOutcome,
    FusionOutcome,
    coherence_concern,
    community_identity,
    envoy_delegation,
    fission,
    fusion,
)

__all__ = [
    # polity
    "AMENDMENT_KINDS",
    "DEFAULT_RETENTION",
    "EROSION_LEVELS",
    "OFFICES",
    "AffirmationState",
    "Archive",
    "CharterState",
    "BOUNDARY_SOURCES",
    "Cursor",
    "DecayTrends",
    "EnvoyState",
    "FOUNDING_BASES",
    "SUBSTRATE_GROUNDINGS",
    "FileStore",
    "FoundingLegitimacy",
    "GroundStanding",
    "GuardianState",
    "ObservationWindow",
    "OfficeState",
    "PruneRecommendation",
    "RetentionPolicy",
    "SaveOutcome",
    "cid_of",
    "generate_keypair",
    "majority",
    "now_seconds",
    "propose_charter",
    # authoring
    "AFFIRMATION_KINDS",
    "ENVOY_KINDS",
    "GUARDIAN_KINDS",
    "MEMBERSHIP_KINDS",
    "OFFICE_KINDS",
    "Signer",
    "admit_member",
    "amend_commitment",
    "authorize_envoy",
    "confer_office",
    "depart_member",
    "designate_derived",
    "dissolve_charter",
    "expel_member",
    "induct_member",
    "ratify_charter",
    "reaffirm_member",
    "prune_observations",
    "disavow_substrate",
    "dissent_boundary",
    "record_observation",
    "release_guardian",
    "resign_envoy",
    "resign_office",
    "revoke_envoy",
    "revoke_office",
    "rotate_guardian",
    # monitor
    "DEFAULT_THRESHOLDS",
    "CohesionReport",
    "FoundingCandidate",
    "Thresholds",
    "cohesion_report",
    "founding_candidate",
    "manual_founding",
    "propose_from_candidate",
    "select_from_candidate",
    "sign_reading",
    # uplift
    "FissionOutcome",
    "FusionOutcome",
    "coherence_concern",
    "community_identity",
    "envoy_delegation",
    "fission",
    "fusion",
]
