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
  admit / depart / expel, confer / resign / revoke an office, induct / re-affirm,
  dissolve.
- `ethne.monitor` — the substrate side: a `FoundingCandidate` and the
  `cohesion_report` that grades drift away from the chartered roll.
- `ethne.uplift` — the three artifacts that cross to kith-covenant.

The governing discipline shows up in the API: a proposal carries no authority, so
`propose_charter` only returns JSON and `ratify_charter` needs a majority of the
roll to co-sign it. Emergence proposes; the polity ratifies; KC recognizes.
"""

from ethne.polity import (
    AMENDMENT_KINDS,
    EROSION_LEVELS,
    OFFICES,
    AffirmationState,
    Archive,
    CharterState,
    OfficeState,
    cid_of,
    generate_keypair,
    majority,
    now_seconds,
    propose_charter,
)
from ethne.authoring import (
    AFFIRMATION_KINDS,
    MEMBERSHIP_KINDS,
    OFFICE_KINDS,
    Signer,
    admit_member,
    amend_commitment,
    confer_office,
    depart_member,
    dissolve_charter,
    expel_member,
    induct_member,
    ratify_charter,
    reaffirm_member,
    resign_office,
    revoke_office,
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
)
from ethne.uplift import (
    FissionOutcome,
    FusionOutcome,
    coherence_concern,
    community_identity,
    fission,
    fusion,
)

__all__ = [
    # polity
    "AMENDMENT_KINDS",
    "EROSION_LEVELS",
    "OFFICES",
    "AffirmationState",
    "Archive",
    "CharterState",
    "OfficeState",
    "cid_of",
    "generate_keypair",
    "majority",
    "now_seconds",
    "propose_charter",
    # authoring
    "AFFIRMATION_KINDS",
    "MEMBERSHIP_KINDS",
    "OFFICE_KINDS",
    "Signer",
    "admit_member",
    "amend_commitment",
    "confer_office",
    "depart_member",
    "dissolve_charter",
    "expel_member",
    "induct_member",
    "ratify_charter",
    "reaffirm_member",
    "resign_office",
    "revoke_office",
    # monitor
    "DEFAULT_THRESHOLDS",
    "CohesionReport",
    "FoundingCandidate",
    "Thresholds",
    "cohesion_report",
    "founding_candidate",
    "manual_founding",
    "propose_from_candidate",
    # uplift
    "FissionOutcome",
    "FusionOutcome",
    "coherence_concern",
    "community_identity",
    "fission",
    "fusion",
]
