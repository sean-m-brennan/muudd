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

"""The charter log and the keys that sign into it.

`Archive` is the polity's append-only, content-addressed charter log — the same
object the Rust tier writes, exposed unchanged because it is already Pythonic. It
carries the authoring methods (see `ethne.authoring` for the `Signer`-flavoured
wrappers over them), the two verification passes, and the three derived
read-models:

    from ethne import Archive, generate_keypair, propose_charter

    convener, member_b, member_c = (generate_keypair() for _ in range(3))
    polity_priv, polity_pub, polity_did = generate_keypair()

    proposal = propose_charter(
        polity_did, "tradition", "answers to what it did not author",
        member_dids=[convener[2], member_b[2], member_c[2]],
    )
    arch = Archive()
    arch.ratify_charter(proposal,
                        [convener[2], member_b[2]],      # a majority ratifies
                        [convener[0], member_b[0]])
    arch.verify_all()        # signatures
    arch.check_authority()   # charter rules
    arch.charter_state(polity_did).members

Timestamps are epoch seconds throughout; every `*_at` argument defaults to now.
"""
from __future__ import annotations

from ethne import _core

Archive = _core.Archive
CharterState = _core.CharterState
OfficeState = _core.OfficeState
AffirmationState = _core.AffirmationState

generate_keypair = _core.generate_ed25519_keypair
now_seconds = _core.now_seconds
propose_charter = _core.propose_charter

#: The four offices ethne names; any other string is a polity-specific office.
OFFICES = ("Convener", "Inductor", "EnvoyAuthorizer", "CharterKeeper")
#: Commitment amendment kinds.
AMENDMENT_KINDS = ("Add", "Remove")
#: Coherence-erosion levels, in rising order.
EROSION_LEVELS = ("None", "Notice", "Concern")


def cid_of(data: bytes) -> str:
    """Blake3 content address of a byte string, in `b3:<hex>` form."""
    return _core.cid_of_bytes(data)


def majority(n: int) -> int:
    """The bar most charter decisions clear: `n // 2 + 1`. Contraction of the
    commitments in force, and expulsion, need one more than this."""
    return n // 2 + 1


__all__ = [
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
]
