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

These artifacts cross the boundary, and nothing else:

1. **The handoff** — `community_identity` turns a chartered polity into a
   self-signed `CommunityIdentity` KC can recognize.
2. **The Concern** — `coherence_concern` emits KC's existing pre-indictment flag,
   and only at `Concern`-level erosion. Below that, the strain is the polity's own
   business. The monitor never decides a split.
3. **A lifecycle change** — `fission` (one polity ratifies a split into N) and
   `fusion` (N polities ratify a merger into one). KC sees new communities seeking
   recognition and old ones going quiet; it learns no new vocabulary.
4. **An envoy** — `envoy_delegation` turns whoever the polity has authorized to
   speak for it into a KC `SpeakerDelegation`. The one artifact an *office*
   produces, and only while the authorization is still in force.
5. **The member roll** — `member_roll` publishes who belongs, which is what makes
   KC's anonymous whistleblower channel real. KC models communities and speakers,
   not members; this tier models members.
6. **The rite** — `intra_rite` turns the polity's own five-movement ceremony into
   the records KC's intra-community rite is held to: a reciter delegation, a witness
   registry, and the drafted recitation. Without it a polity could be identified in
   KC and never admitted to a compact, because KC's inter-community ceremony takes a
   sealed intra rite as its input.
7. **The relay roster** — `rendezvous_roster` lists the members the polity has
   made rendezvous operators, in AutonomousTrust's own signed `at-relay-roster`
   file rather than a KC record (D36): it crosses to the trust substrate, which
   verifies it with nothing but the polity key its operator pinned.

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
RiteHandoff = _core.RiteHandoff
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


def member_roll(
    archive,
    polity_did: str,
    root_priv: bytes,
    root_pub: bytes,
    as_of: Optional[int] = None,
    published_at: Optional[int] = None,
) -> str:
    """KC's `MemberRoll` (JSON) for this polity — the border artifact that gives it a
    real anonymous whistleblower channel.

    KC models communities and the speakers they delegate, so without a roll an insider
    holding no delegation is not in the §8 insider ring at all, and a polity that has
    delegated nobody has no anonymous channel. This tier is what models membership, so
    it is the feed. The roll is read from the log as of `as_of` — the founding roll
    plus admissions minus departures — and each member's verifying key comes from its
    own `did:key`.

    KC **unions** rolls rather than superseding them: publishing a later, shorter roll
    removes nobody, so a member who has left keeps the channel they had. Republish
    whenever the roll grows. The polity itself is dropped from its own roll (its root
    key is already an insider), and a roll with no one else on it is an error."""
    return archive.member_roll(
        polity_did, root_priv, root_pub, as_of, published_at
    )


def rendezvous_roster(
    archive,
    polity_did: str,
    root_priv: bytes,
    root_pub: bytes,
    seq: int,
    as_of: Optional[int] = None,
) -> str:
    """The polity's relay roster as AT's signed ``at-relay-roster`` file (D36):
    ``{"body": <exact JSON>, "sig": <hex>}``, signed by the polity root key over
    ``"at-relay-roster-v1|" + body``.

    It lists, as of ``as_of``, every member who holds ``RendezvousOperator`` and has
    a live relay declaration, each pinned to the member's own key. Write the text
    where the polity's AT nodes read rosters; AT accepts it only from an issuer its
    operator pinned.

    A higher ``seq`` **replaces** the previous roster whole, unlike the member
    roll's union: that is how revoking the office removes a relay. So ``seq`` must
    exceed every seq published before (the seconds of ``as_of`` are a natural
    choice), and an empty roster is how the polity says it runs none now."""
    return archive.rendezvous_roster(polity_did, root_priv, root_pub, seq, as_of)


def roster_dict(roster_text: str) -> dict:
    """The roster's signed body, parsed, for inspection."""
    return json.loads(json.loads(roster_text)["body"])


def intra_rite(
    archive,
    polity_did: str,
    inductor_did: str,
    root_priv: bytes,
    root_pub: bytes,
    at: Optional[int] = None,
) -> RiteHandoff:
    """The records KC's intra-community rite is held to: `reciter`, `witnesses` and
    `recitation`, each JSON.

    The polity's **Inductor** recites, because the office that presides over the
    polity's own rite is the office that carries one across the border; KC's `Envoy`
    scope is for representation and deliberately not this. The **attendees** witness:
    the members present for every movement of the polity's own rite, so the KC record
    answers to the people who were there rather than to whoever holds a key. The five
    movements are drafted from the charter, and the polity's in-force commitment keys
    travel into Movement 4 verbatim, which is what lets KC grade a compact of polities
    against the object each one actually holds.

    A rite is not mintable from nothing: an error unless a rite of the polity's stands
    complete as of `at`, unless `inductor_did` holds the Inductor office and has been
    inducted, and unless the polity has commitments in force.

    Only the two authorizations are signed here, under the polity root. `recitation`
    is unsigned on purpose — sealing it needs the reciter's key and every witness's,
    which this seam does not hold. Feed it to KC's `IntraCommunityRiteRunner`, where
    the reciter signs the recitation, each witness signs its own attestation, and the
    runner seals."""
    return archive.intra_rite(polity_did, inductor_did, root_priv, root_pub, at)


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
    successor_jsons: Sequence[str],
    root_priv: bytes,
    deciders: Sequence[Signer] = (),
    rationale: str = "",
    at: Optional[int] = None,
) -> FissionOutcome:
    """Emit a ratified split into N (>= 2) successors. The community has already
    chartered them (pass their identities as JSON); this signs, under the
    predecessor's root, a successor link to each plus the predecessor's dissolution
    naming them all.

    N-ary since D25, mirroring `fusion` — a three-way split is one act, where before
    it took two chained fissions and recorded an intermediate polity nobody lived in.
    The guards are fusion's in the other direction: at least two successors, none
    named twice, none that is the predecessor itself, and every successor's own
    self-signature verified (without which the predecessor's root would sign a link to
    whatever it was handed)."""
    return archive.fission(
        polity_did,
        list(successor_jsons),
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


def rite_dict(handoff: RiteHandoff) -> dict:
    """An `intra_rite` handoff parsed for inspection, keyed as the handoff is. The
    JSON strings themselves are what cross to KC."""
    return {
        "reciter": json.loads(handoff.reciter),
        "witnesses": json.loads(handoff.witnesses),
        "recitation": json.loads(handoff.recitation),
    }


def concern_dict(concern_json: Optional[str]) -> Optional[dict]:
    """`coherence_concern`'s output parsed, passing `None` through."""
    return None if concern_json is None else json.loads(concern_json)


__all__ = [
    "FissionOutcome",
    "FusionOutcome",
    "RiteHandoff",
    "coherence_concern",
    "community_identity",
    "concern_dict",
    "delegation_dict",
    "envoy_delegation",
    "fission",
    "fusion",
    "identity_dict",
    "intra_rite",
    "member_roll",
    "rendezvous_roster",
    "roster_dict",
    "rite_dict",
]
