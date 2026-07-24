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
) -> str:
    """Ratify a proposal into the polity's genesis charter. The ratifiers must be
    a majority of the proposed roll — emergence proposes, the polity ratifies."""
    dids, privs = _split(ratifiers)
    return archive.ratify_charter(proposal_json, dids, privs, founded_at)


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
]
