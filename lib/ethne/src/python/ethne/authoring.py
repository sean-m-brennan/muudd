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

"""Author charter records from Python, in terms of `Signer`s.

The `_core` methods take co-signers as two parallel lists (DIDs and key bytes).
These wrappers take a sequence of `Signer` instead, which is what callers
actually hold, and flatten it — the same relationship `kith_covenant.authoring`
has to its own bindings. Signing happens in the Rust core; a `Signer` holds raw
key bytes the caller is responsible for.

The usual arc of a polity's life:

    from ethne import Archive, Signer, generate_keypair, propose_charter
    from ethne.authoring import admit_member, induct_member, ratify_charter

    founders = [Signer.generate() for _ in range(3)]
    polity = Signer.generate()

    proposal = propose_charter(polity.did, "tradition", "answers to X above itself",
                              member_dids=[s.did for s in founders])
    arch = Archive()
    ratify_charter(arch, proposal, founders[:2])      # a majority ratifies
    joiner = Signer.generate()
    admit_member(arch, polity.did, joiner, founders[:2])
    induct_member(arch, polity.did, joiner, founders)  # majority of the grown roll
    arch.verify_all()
    arch.check_authority()

Records do not verify on append — a batch may be assembled out of order — so call
`archive.verify_all()` and `archive.check_authority()` once it is complete.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Optional, Sequence

from ethne import _core

#: Membership-lifecycle record kinds, for discoverability.
MEMBERSHIP_KINDS = ("Admit", "DepartVoluntary", "Expel")
#: Office-lifecycle record kinds.
OFFICE_KINDS = ("Confer", "Resign", "Revoke")
#: Guardian-edge record kinds — which members are machines, and whose.
GUARDIAN_KINDS = ("Designate", "Rotate", "Release")
#: Envoy-standing record kinds — who may speak for the polity beyond it.
ENVOY_KINDS = ("Authorize", "Resign", "Revoke")
#: Affirmation kinds — the transmission of the substrate, and its renewal.
AFFIRMATION_KINDS = ("Induction", "ReAffirmation")


@dataclass(frozen=True)
class Signer:
    """A key that signs charter records, and the DID it signs as.

    `priv` is a secret; treat this object as sensitive — it deliberately does not
    print its key material. For a polity's own root identity (the key the uplift
    calls sign with) `did` is the polity DID; for a member it is the member's.
    """

    priv: bytes
    did: str

    @classmethod
    def generate(cls) -> "Signer":
        """A fresh Ed25519 identity."""
        priv, _pub, did = _core.generate_ed25519_keypair()
        return cls(priv=priv, did=did)

    def __repr__(self) -> str:  # never leak key material
        return f"Signer(did={self.did!r}, priv=<{len(self.priv)} bytes>)"


def _split(signers: Sequence[Signer]) -> tuple[list[str], list[bytes]]:
    """A signer sequence as the two parallel lists `_core` takes."""
    return [s.did for s in signers], [s.priv for s in signers]


# ---- Founding ---------------------------------------------------------------


def ratify_charter(
    archive,
    proposal_json: str,
    ratifiers: Sequence[Signer],
    founded_at: Optional[int] = None,
    guardians: Sequence[Signer] = (),
) -> str:
    """Ratify a proposal into the polity's genesis charter. The ratifiers must be
    a majority of the proposed roll — emergence proposes, the polity ratifies.

    Where the proposal carries derivations, every named `guardian` must co-sign as
    well: answerability for a machine is accepted, never assigned."""
    dids, privs = _split(ratifiers)
    g_dids, g_privs = _split(guardians)
    return archive.ratify_charter(proposal_json, dids, privs, g_dids, g_privs, founded_at)


# ---- Commitments ------------------------------------------------------------


def amend_commitment(
    archive,
    polity_did: str,
    kind: str,
    key: str,
    deciders: Sequence[Signer],
    description: str = "",
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Add (majority) or remove (majority + 1) a commitment. Founding commitments
    are genesis-immutable, and the substrate has no amendment path at all."""
    dids, privs = _split(deciders)
    return archive.amend_commitment(
        polity_did, kind, key, description, rationale, dids, privs, decided_at
    )


# ---- Membership -------------------------------------------------------------


def admit_member(
    archive,
    polity_did: str,
    joiner: Signer,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Admit `joiner` at a majority of the live roll, with the joiner's own
    co-signature (no conscription). Admission puts them on the roll; it does not
    transmit the substrate — see `induct_member`."""
    dids, privs = _split(deciders)
    return archive.admit_member(
        polity_did, joiner.did, joiner.priv, dids, privs, rationale, decided_at
    )


def depart_member(
    archive,
    polity_did: str,
    member: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """A member leaves of their own accord: self-signed, no deciders. Refused only
    if it would drop the roll below the self-govern floor."""
    return archive.depart_member(
        polity_did, member.did, member.priv, rationale, decided_at
    )


def expel_member(
    archive,
    polity_did: str,
    member_did: str,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Expel a member at a majority + 1, with the target excluded from the
    deciders."""
    dids, privs = _split(deciders)
    return archive.expel_member(
        polity_did, member_did, dids, privs, rationale, decided_at
    )


# ---- Offices ----------------------------------------------------------------


def confer_office(
    archive,
    polity_did: str,
    office: str,
    holder: Signer,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Confer an office at a majority, with the holder's consent. The roll confers
    offices; offices never confer offices."""
    dids, privs = _split(deciders)
    return archive.confer_office(
        polity_did, office, holder.did, holder.priv, dids, privs, rationale, decided_at
    )


def resign_office(
    archive,
    polity_did: str,
    office: str,
    holder: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The holder lays an office down: self-signed."""
    return archive.resign_office(
        polity_did, office, holder.did, holder.priv, rationale, decided_at
    )


def revoke_office(
    archive,
    polity_did: str,
    office: str,
    holder_did: str,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The roll takes an office back at a majority, holder excluded."""
    dids, privs = _split(deciders)
    return archive.revoke_office(
        polity_did, office, holder_did, dids, privs, rationale, decided_at
    )


# ---- Observation ------------------------------------------------------------


def record_observation(
    archive,
    polity_did: str,
    observer: Signer,
    readings: Sequence[tuple[str, float, Optional[str], Optional[int]]],
    opaque_boundary: bool = False,
    observed_at: Optional[int] = None,
) -> str:
    """Record what a monitor saw: `(member_did, reputation, guardian_did, attested_at)`
    per member observed.

    An observation asserts nothing about the polity — it is one party's perspectival
    evidence, signed by whoever looked, and the observer need not be a member. What it
    makes possible is duration: only a record of observation can say how long a
    guardian has been absent, and only that can escalate."""
    return archive.record_observation(
        polity_did, observer.did, observer.priv, list(readings), opaque_boundary, observed_at
    )


def prune_observations(
    archive,
    polity_did: str,
    horizon: int,
    deciders: Sequence[Signer],
    rationale: str = "",
    pruned_at: Optional[int] = None,
) -> str:
    """Drop the observations recorded before `horizon`, leaving a signed record of
    the fact and a receipt for what went.

    Observations are the one entry class a polity may drop: they decide nothing, so
    removing one cannot rewrite what the polity resolved. An effective
    `CharterKeeper` among the deciders suffices beyond the settling horizon (30
    days) — the custody that office is — while a majority of the roll may prune
    anything. Recent evidence is deliberately beyond any single custodian's reach,
    because destroying it is exactly the move a captured party makes.

    This authors the record. Call `archive.apply_prunings(polity_did)` to get an
    archive with the observations actually dropped."""
    return archive.prune_observations(
        polity_did, horizon, [d.did for d in deciders], [d.priv for d in deciders],
        rationale, pruned_at,
    )


# ---- The guardian edge ------------------------------------------------------


def designate_derived(
    archive,
    polity_did: str,
    member_did: str,
    deciders: Sequence[Signer],
    guardian: Optional[Signer] = None,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Designate a member a derived (machine) party at a majority, with the named
    guardian co-signing its acceptance. Passing no guardian charters a machine with
    nobody behind it — honest, and refused an office where machines are the
    majority."""
    dids, privs = _split(deciders)
    return archive.designate_derived(
        polity_did, member_did, dids, privs,
        guardian.did if guardian else None,
        guardian.priv if guardian else None,
        rationale, decided_at,
    )


def rotate_guardian(
    archive,
    polity_did: str,
    member_did: str,
    guardian: Signer,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Point a designated member at a (new) guardian — the operator changed, or an
    unguarded machine acquires one. A majority plus the incoming guardian's consent."""
    dids, privs = _split(deciders)
    return archive.rotate_guardian(
        polity_did, member_did, guardian.did, guardian.priv, dids, privs, rationale, decided_at
    )


def release_guardian(
    archive,
    polity_did: str,
    member_did: str,
    guardian: Optional[Signer] = None,
    deciders: Sequence[Signer] = (),
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Record that a member's guardian edge is gone. Either the departing `guardian`
    signs (stepping away) or a majority of the `deciders` records that the human is
    no longer there. The designation survives the release."""
    dids, privs = _split(deciders)
    return archive.release_guardian(
        polity_did, member_did,
        guardian.did if guardian else None,
        guardian.priv if guardian else None,
        dids, privs, rationale, decided_at,
    )


# ---- Envoy standing ---------------------------------------------------------


def authorize_envoy(
    archive,
    polity_did: str,
    envoy: Signer,
    authorizers: Sequence[Signer],
    rationale: str = "",
    expires_at: Optional[int] = None,
    decided_at: Optional[int] = None,
) -> str:
    """Authorize an envoy to speak for the polity beyond it. Either a single
    `EnvoyAuthorizer` (the office's delegated power) or a majority of the roll may
    do it, and the envoy always co-signs — the duty is never conscripted."""
    dids, privs = _split(authorizers)
    return archive.authorize_envoy(
        polity_did, envoy.did, envoy.priv, dids, privs, rationale, expires_at, decided_at
    )


def resign_envoy(
    archive,
    polity_did: str,
    envoy: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The envoy stands down: self-signed."""
    return archive.resign_envoy(
        polity_did, envoy.did, envoy.priv, rationale, decided_at
    )


def revoke_envoy(
    archive,
    polity_did: str,
    envoy_did: str,
    authorizers: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The polity withdraws the standing, on the same two routes, envoy excluded."""
    dids, privs = _split(authorizers)
    return archive.revoke_envoy(
        polity_did, envoy_did, dids, privs, rationale, decided_at
    )


# ---- Transmission -----------------------------------------------------------


def induct_member(
    archive,
    polity_did: str,
    member: Signer,
    witnesses: Sequence[Signer],
    affirmed_at: Optional[int] = None,
) -> str:
    """Induct a member into the substrate: a majority of the roll witnesses, and
    the inductee co-signs. Founding members need no induction — ratifying the
    charter was theirs."""
    dids, privs = _split(witnesses)
    return archive.induct_member(
        polity_did, member.did, member.priv, dids, privs, affirmed_at
    )


def reaffirm_member(
    archive,
    polity_did: str,
    member: Signer,
    affirmed_at: Optional[int] = None,
) -> str:
    """Renew an inducted member's affirmation: self-signed, unwitnessed. Freshness
    is what `AffirmationState.stale_members` reads."""
    return archive.reaffirm_member(polity_did, member.did, member.priv, affirmed_at)


# ---- Dissolution ------------------------------------------------------------


def dissolve_charter(
    archive,
    polity_did: str,
    deciders: Sequence[Signer],
    successors: Sequence[str] = (),
    rationale: str = "",
    dissolved_at: Optional[int] = None,
) -> str:
    """Wind the polity up at a majority, optionally naming successors. Terminal:
    no charter event is authorized afterwards."""
    dids, privs = _split(deciders)
    return archive.dissolve_charter(
        polity_did, dids, privs, list(successors), rationale, dissolved_at
    )


__all__ = [
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
    "record_observation",
    "reaffirm_member",
    "release_guardian",
    "resign_envoy",
    "resign_office",
    "revoke_envoy",
    "revoke_office",
    "rotate_guardian",
]
