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

"""The upward interface: what a polity hands to kith-covenant.

Three artifacts cross the boundary, and nothing else:

1. **The handoff** — `community_identity` turns a chartered polity into a
   self-signed `CommunityIdentity` KC can recognize.
2. **The Concern** — `coherence_concern` emits KC's existing pre-indictment flag,
   and only at `Concern`-level erosion. Below that, the strain is the polity's own
   business. The monitor never decides a split.
3. **A lifecycle change** — `fission` (one polity ratifies a split into two) and
   `fusion` (N polities ratify a merger into one). KC sees new communities seeking
   recognition and old ones going quiet; it learns no new vocabulary.

Every artifact crosses as **JSON**, which a KC-side consumer appends to its own
log verbatim; each function here has a `*_dict` twin that parses it for
inspection. These are emissions, not appends: to record a dissolution in the
polity's own charter log — and have it pass the authority rules — the deciders
must co-sign it through `ethne.authoring.dissolve_charter`.
"""
from __future__ import annotations

import json
from typing import Optional, Sequence

from ethne import _core
from ethne.authoring import Signer

FissionOutcome = _core.FissionOutcome
FusionOutcome = _core.FusionOutcome


def community_identity(
    archive,
    polity_did: str,
    name: str,
    root_priv: bytes,
    root_pub: bytes,
    created_at: Optional[int] = None,
) -> str:
    """Hand the chartered polity off as a self-signed `CommunityIdentity` (JSON).
    The root keypair must be the polity's own: its DID is derived from `root_pub`,
    and a mismatch with the charter is an error."""
    return archive.community_identity(
        polity_did, name, root_priv, root_pub, created_at
    )


def envoy_delegation(
    archive,
    polity_did: str,
    envoy_did: str,
    root_priv: bytes,
    root_pub: bytes,
    issued_at: Optional[int] = None,
    as_of: Optional[int] = None,
) -> str:
    """KC's envoy `SpeakerDelegation` (JSON) for an envoy the polity has authorized
    — the one border artifact an *office* produces. The standing is read from the
    log as of `as_of`, so an envoy who resigned, was revoked, left the roll, or
    whose authorization expired is an error, not a delegation."""
    return archive.envoy_delegation(
        polity_did, envoy_did, root_priv, root_pub, issued_at, as_of
    )


def coherence_concern(
    report,
    subject_did: str,
    observer: Signer,
    at: Optional[int] = None,
) -> Optional[str]:
    """KC's `Concern` about a polity (JSON), or `None` unless `report.erosion` has
    reached `"Concern"`. Filed with an explicit monitor perspective, so KC
    discounts it as a partial signal rather than a neutral finding."""
    return _core.coherence_concern(
        report, subject_did, observer.did, observer.priv, at
    )


def fission(
    archive,
    polity_did: str,
    successor_a_json: str,
    successor_b_json: str,
    root_priv: bytes,
    deciders: Sequence[Signer] = (),
    rationale: str = "",
    at: Optional[int] = None,
) -> FissionOutcome:
    """Emit a ratified split. The community has already chartered two successors
    (pass their identities as JSON); this signs, under the predecessor's root, a
    successor link to each plus the predecessor's dissolution naming both."""
    return archive.fission(
        polity_did,
        successor_a_json,
        successor_b_json,
        root_priv,
        [s.did for s in deciders],
        rationale,
        at,
    )


def fusion(
    archive,
    predecessors: Sequence[tuple[str, bytes, Sequence[Signer]]],
    fused_json: str,
    rationale: str = "",
    at: Optional[int] = None,
) -> FusionOutcome:
    """Emit a ratified merger of N (>= 2) predecessors into the already-chartered
    `fused` identity. Each predecessor is `(polity_did, root_priv, deciders)`; the
    returned `links` and `dissolutions` are index-aligned with that sequence."""
    parties = [
        (did, root_priv, [s.did for s in deciders])
        for did, root_priv, deciders in predecessors
    ]
    return archive.fusion(parties, fused_json, rationale, at)


def identity_dict(identity_json: str) -> dict:
    """A signed `CommunityIdentity` (or any emitted artifact) parsed for
    inspection. The JSON itself is what crosses to KC."""
    return json.loads(identity_json)


def delegation_dict(delegation_json: str) -> dict:
    """`envoy_delegation`'s output parsed for inspection."""
    return json.loads(delegation_json)


def concern_dict(concern_json: Optional[str]) -> Optional[dict]:
    """`coherence_concern`'s output parsed, passing `None` through."""
    return None if concern_json is None else json.loads(concern_json)


__all__ = [
    "FissionOutcome",
    "FusionOutcome",
    "coherence_concern",
    "community_identity",
    "concern_dict",
    "delegation_dict",
    "envoy_delegation",
    "fission",
    "fusion",
    "identity_dict",
]
