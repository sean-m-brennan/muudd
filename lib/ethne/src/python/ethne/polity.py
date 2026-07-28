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
wrappers over them), the two verification passes, and the derived read-models:

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

from dataclasses import dataclass

from ethne import _core

Archive = _core.Archive
CharterState = _core.CharterState
OfficeState = _core.OfficeState
EnvoyState = _core.EnvoyState
GuardianState = _core.GuardianState
DecayTrends = _core.DecayTrends
ObservationWindow = _core.ObservationWindow
AffirmationState = _core.AffirmationState
PruneRecommendation = _core.PruneRecommendation
FoundingLegitimacy = _core.FoundingLegitimacy
Cursor = _core.Cursor
SaveOutcome = _core.SaveOutcome
FileStore = _core.FileStore

generate_keypair = _core.generate_ed25519_keypair
now_seconds = _core.now_seconds
propose_charter = _core.propose_charter

#: The four offices ethne names; any other string is a polity-specific office.
OFFICES = ("Convener", "Inductor", "EnvoyAuthorizer", "CharterKeeper")
#: Commitment amendment kinds.
AMENDMENT_KINDS = ("Add", "Remove")
#: Coherence-erosion levels, in rising order.
EROSION_LEVELS = ("None", "Notice", "Concern")
#: How a charter's boundary was established (D21). **Not ordered** — a founding is
#: not eroding, and these are not comparable harms.
FOUNDING_BASES = ("Corroborated", "Narrowed", "SelfAttested")
#: Where a charter's boundary came from: an independent reading, or the convener's
#: own word with nobody else attesting it.
BOUNDARY_SOURCES = ("Substrate", "ConvenerAsserted")


#: `en-charter`'s own retention defaults, read from the Rust core so there is only
#: ever one copy of them.
DEFAULT_RETENTION = _core.DEFAULT_RETENTION


@dataclass(frozen=True)
class RetentionPolicy:
    """How long a polity keeps its observations, and how much must have aged out
    before dropping them is worth a record.

    A policy, never a schedule. This tier's libraries do not call `now()` — every
    cadence is a query parameter — so the loop and the clock stay with the host,
    exactly as the storage backend does. What is *not* the host's to know is which
    history a live measure still depends on and which authorization route a given
    horizon leaves open; that is what `Archive.prune_recommendation` answers.

    Field defaults come from `DEFAULT_RETENTION`, i.e. from `en-charter` itself.

    `retain_seconds` (30 days) is the smallest value that does two things at once: it
    keeps every observation-based measure intact — the longest is
    `sustained_absence_seconds`, one day — and it keeps the single-`CharterKeeper`
    route open, since that route needs a horizon at least the settling period old.
    The safe default is therefore also the cheap one to authorize, and the cheap one
    to store: a keeper-authorized pruning carries one signature where a roll-majority
    one carries a majority of them. A shorter retention is permitted and forces the
    roll route.

    `min_dropped` exists because **a pruning record is itself unprunable**.
    Observations are the one entry class a polity may drop; the records that drop
    them are governing records and stay forever. Pruning per observation would trade
    a reclaimable few kilobytes for a permanent three — worse the longer the polity
    runs, not better.

    `live_measure_seconds` and `min_witness_readings` are the two safety floors the
    horizon can never cross, whatever `retain_seconds` asks for. They are separate
    knobs from retention because they are a different claim: one is how much history
    the polity wants to keep, the others are how much its own grading still needs.
    Collapsing them would let a caller tune away a safety property while believing
    they were only tuning storage.

    Both floors are needed, and that was measured rather than assumed. A sustained
    absence is graded on a **span** and on a **witness count**. Pruning clamps the
    span, so a duration floor protects it — but nothing protected the count: prune to
    a 24-hour horizon on a polity observed twice a day and `readings_for` falls from
    four to two, so a `Concern` silently becomes a `Notice` and nothing says that
    evidence, not conduct, is what changed. `min_witness_readings` is per member,
    because that is what `readings_for` counts. A member the monitors mention rarely
    can therefore pin the horizon — the correct answer, and not a deadlock: the roll
    may still prune anything by majority. What is refused is *recommending* it.
    """

    retain_seconds: int = DEFAULT_RETENTION["retain_seconds"]
    min_dropped: int = DEFAULT_RETENTION["min_dropped"]
    live_measure_seconds: int = DEFAULT_RETENTION["live_measure_seconds"]
    min_witness_readings: int = DEFAULT_RETENTION["min_witness_readings"]


def cid_of(data: bytes) -> str:
    """Blake3 content address of a byte string, in `b3:<hex>` form."""
    return _core.cid_of_bytes(data)


def majority(n: int) -> int:
    """The bar most charter decisions clear: `n // 2 + 1`. Contraction of the
    commitments in force, and expulsion, need one more than this."""
    return n // 2 + 1


__all__ = [
    "AMENDMENT_KINDS",
    "BOUNDARY_SOURCES",
    "FOUNDING_BASES",
    "EROSION_LEVELS",
    "OFFICES",
    "AffirmationState",
    "Archive",
    "CharterState",
    "EnvoyState",
    "DecayTrends",
    "GuardianState",
    "DEFAULT_RETENTION",
    "ObservationWindow",
    "OfficeState",
    "Cursor",
    "FileStore",
    "FoundingLegitimacy",
    "PruneRecommendation",
    "RetentionPolicy",
    "SaveOutcome",
    "cid_of",
    "generate_keypair",
    "majority",
    "now_seconds",
    "propose_charter",
]
