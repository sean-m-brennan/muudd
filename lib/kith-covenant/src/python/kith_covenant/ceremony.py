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

"""Thin Python wrappers over the Rust `kith_covenant._core` bindings.

Exposes a Pythonic surface for the two-community ceremony pipeline:
- `generate_keypair()` → `(priv: bytes, pub: bytes, did: str)`
- `cid_of(bytes) -> str`
- `CeremonyResult` frozen dataclass
- `run_two_community_ceremony(...)` → `CeremonyResult`
- `Archive` (re-exports `_core.PyArchive` unchanged — it already is Pythonic)

The underlying `_core` extension module is produced by the `kc-pyo3` Rust
crate via maturin. If it is not built / not importable, this module raises
`ModuleNotFoundError` on import — `kith_covenant.interface` and
`kith_covenant.policy` (the AT seam) do not depend on the bindings and
remain importable on their own.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

from kith_covenant import _core

Archive = _core.PyArchive
generate_keypair = _core.generate_ed25519_keypair


def cid_of(data: bytes) -> str:
    """Blake3 content-ID of a byte string, in `b3:<hex>` form."""
    return _core.cid_of_bytes(data)


@dataclass(frozen=True)
class CeremonyResult:
    """Summary of a completed two-community credentialing ceremony."""

    alpha_did: str
    beta_did: str
    alpha_identity_cid: str
    beta_identity_cid: str
    alpha_intra_cid: str
    beta_intra_cid: str
    witness_cid: str
    archive_size: int
    archive_json: str
    compact_label: str


def run_two_community_ceremony(
    alpha_name: str,
    alpha_movements: Sequence[str],
    beta_name: str,
    beta_movements: Sequence[str],
    compact_label: str = "two-community-compact",
) -> CeremonyResult:
    """Run a full ceremony with mutual acceptance of all five anti-spiral
    feature claims. Archive is crypto-verified before returning.

    Each `movements` sequence must contain exactly 5 strings corresponding to
    the movements:
        [substrate, fallenness, recognition, common_virtue, non_ultimacy]
    """
    alpha_list = list(alpha_movements)
    beta_list = list(beta_movements)
    r = _core.run_two_community_ceremony(
        alpha_name, alpha_list, beta_name, beta_list, compact_label
    )
    return CeremonyResult(
        alpha_did=r.alpha_did,
        beta_did=r.beta_did,
        alpha_identity_cid=r.alpha_identity_cid,
        beta_identity_cid=r.beta_identity_cid,
        alpha_intra_cid=r.alpha_intra_cid,
        beta_intra_cid=r.beta_intra_cid,
        witness_cid=r.witness_cid,
        archive_size=r.archive_size,
        archive_json=r.archive_json,
        compact_label=r.compact_label,
    )


__all__ = [
    "Archive",
    "CeremonyResult",
    "cid_of",
    "generate_keypair",
    "run_two_community_ceremony",
]
