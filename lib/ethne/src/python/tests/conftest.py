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
from typing import Optional, Sequence

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
        """A majority of the members who carry weight, drawn from the founders.

        Since D29 a majority is of the members who hold the substrate, so this counts
        the inducted roll rather than the whole one.
        """
        st = self.archive.affirmation_state(self.did, as_of)
        inducted = sum(1 for m, ind, _ in st.entries if ind)
        need = e.majority(inducted)
        assert need <= len(self.founders), "fixture cannot form this quorum"
        return self.founders[:need]

    def rite(
        self,
        member: Signer,
        attending: Sequence[Signer],
        start: int,
        commitments: Sequence[str] = (),
    ) -> list[str]:
        """Perform `member`'s five-movement rite (D29), returning the movement CIDs
        the induction must cite."""
        return e.perform_rite(
            self.archive, self.did, member, list(attending),
            substrate_name="tradition", commitments=commitments, start=start,
        )


@pytest.fixture
def found():
    """Factory: found a polity of `n` members, ratified by all of them."""

    def _found(
        n: int = 3,
        commitments: Sequence[tuple[str, str]] = (),
        ratifiers: Optional[int] = None,
    ) -> Polity:
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
        # The whole roll ratifies by default. Since D29 ratifying is what inducts a
        # founding member, so a bare-majority ratification (the old default) leaves
        # members on the roll who carry no weight, hold no office and witness no rite —
        # correct, and not what most tests are about. Pass `ratifiers` for the cases
        # where partial ratification is the point.
        signing = founders[: ratifiers if ratifiers is not None else n]
        ratify_charter(archive, proposal, signing, founded_at=FOUNDED_AT)
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
