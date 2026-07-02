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

"""Reputation-lookup convenience layer.

Thin Python facade over the PyO3 ReputationView. Users can either call
archive.reputation_of(...) on a kith_covenant.Archive directly (returning
the native PyO3 class), or use reputation_of(...) below to get an
equivalent frozen dataclass.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from kith_covenant import _core


@dataclass(frozen=True)
class IntraRiteSummary:
    record_cid: str
    sealed_at: int
    speaker_did: str
    witness_count: int


@dataclass(frozen=True)
class InterAdmissionSummary:
    record_cid: str
    witnessed_at: int
    counterparties: tuple[str, ...]


@dataclass(frozen=True)
class RejectionSummary:
    rejecting_community: str
    presenting_community: str
    feature: str
    rationale: str


@dataclass(frozen=True)
class InterNonAdmissionSummary:
    record_cid: str
    decided_at: int
    counterparties: tuple[str, ...]
    rejections: tuple[RejectionSummary, ...]


@dataclass(frozen=True)
class DelegationSummary:
    record_cid: str
    subject_did: str
    scope: str
    active_from: int
    active_until: Optional[int]


@dataclass(frozen=True)
class WitnessSummary:
    record_cid: str
    member_did: str
    role: str
    active_from: int
    active_until: Optional[int]


@dataclass(frozen=True)
class ReputationView:
    community_did: str
    as_of: int
    standing: str
    identity_cid: Optional[str]
    intra_rites: tuple[IntraRiteSummary, ...]
    inter_admissions: tuple[InterAdmissionSummary, ...]
    inter_nonadmissions: tuple[InterNonAdmissionSummary, ...]
    active_delegations: tuple[DelegationSummary, ...]
    active_witnesses: tuple[WitnessSummary, ...]


def _to_frozen(v) -> ReputationView:
    return ReputationView(
        community_did=v.community_did,
        as_of=v.as_of,
        standing=v.standing,
        identity_cid=v.identity_cid,
        intra_rites=tuple(
            IntraRiteSummary(s.record_cid, s.sealed_at, s.speaker_did, s.witness_count)
            for s in v.intra_rites
        ),
        inter_admissions=tuple(
            InterAdmissionSummary(s.record_cid, s.witnessed_at, tuple(s.counterparties))
            for s in v.inter_admissions
        ),
        inter_nonadmissions=tuple(
            InterNonAdmissionSummary(
                record_cid=s.record_cid,
                decided_at=s.decided_at,
                counterparties=tuple(s.counterparties),
                rejections=tuple(
                    RejectionSummary(r.rejecting_community, r.presenting_community,
                                      r.feature, r.rationale)
                    for r in s.rejections
                ),
            )
            for s in v.inter_nonadmissions
        ),
        active_delegations=tuple(
            DelegationSummary(s.record_cid, s.subject_did, s.scope,
                              s.active_from, s.active_until)
            for s in v.active_delegations
        ),
        active_witnesses=tuple(
            WitnessSummary(s.record_cid, s.member_did, s.role,
                           s.active_from, s.active_until)
            for s in v.active_witnesses
        ),
    )


def reputation_of(archive, did: str, as_of: Optional[int] = None) -> ReputationView:
    """Project `archive` into a ReputationView for `did`.

    `archive` is a kith_covenant.Archive (the PyO3 class). Returns a
    frozen dataclass so callers can hash/share without worrying about the
    PyO3 object lifecycle.
    """
    native = archive.reputation_of(did, as_of)
    return _to_frozen(native)
