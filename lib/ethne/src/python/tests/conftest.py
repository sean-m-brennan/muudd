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

"""Shared fixtures: a founded polity with fixed timestamps.

Timestamps are pinned (proposed at 50, founded at 100) so the affirmation-freshness
assertions are deterministic rather than clock-dependent.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Sequence

import pytest

import ethne as e
from ethne.authoring import Signer, ratify_charter

PROPOSED_AT = 50
FOUNDED_AT = 100


@dataclass
class Polity:
    """A founded polity plus the keys needed to act for and as it."""

    archive: e.Archive
    did: str
    priv: bytes
    pub: bytes
    founders: list[Signer]

    def roll(self, as_of: int) -> list[str]:
        return self.archive.charter_state(self.did, as_of).members

    def quorum(self, as_of: int) -> list[Signer]:
        """A majority of the live roll, drawn from the founders."""
        need = e.majority(len(self.roll(as_of)))
        assert need <= len(self.founders), "fixture cannot form this quorum"
        return self.founders[:need]


@pytest.fixture
def found():
    """Factory: found a polity of `n` members, ratified by a bare majority."""

    def _found(n: int = 3, commitments: Sequence[tuple[str, str]] = ()) -> Polity:
        founders = [Signer.generate() for _ in range(n)]
        priv, pub, did = e.generate_keypair()
        proposal = e.propose_charter(
            did,
            "tradition",
            "answers to what it did not author",
            member_dids=[s.did for s in founders],
            commitments=list(commitments),
            proposed_at=PROPOSED_AT,
        )
        archive = e.Archive()
        ratify_charter(
            archive, proposal, founders[: e.majority(n)], founded_at=FOUNDED_AT
        )
        return Polity(
            archive=archive, did=did, priv=priv, pub=pub, founders=founders
        )

    return _found


@pytest.fixture
def polity(found) -> Polity:
    """A three-member polity, chartered and clean."""
    return found()


@pytest.fixture
def wide(found) -> Polity:
    """A four-member polity — the smallest roll that can shed a member without
    breaching the self-govern floor."""
    return found(4)
