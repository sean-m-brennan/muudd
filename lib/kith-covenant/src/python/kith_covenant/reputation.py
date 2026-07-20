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
class ConductSummary:
    record_cid: str
    kind: str
    bearing: str
    severity: str
    ladder_rung: Optional[str]
    decision_class: str
    reporters: tuple[str, ...]
    reporter_perspective: Optional[str]
    references: tuple[str, ...]
    recorded_at: int


@dataclass(frozen=True)
class SponsorshipSummary:
    record_cid: str
    sponsor_community: str
    entrant_community: str
    admission_cid: Optional[str]
    probation_until: int
    sponsored_at: int


@dataclass(frozen=True)
class DisputeSummary:
    record_cid: str
    complainant: str
    respondent: str
    rung: str
    # The dispute's current rung after any ladder transitions (§5).
    current_rung: str
    summary: str
    filed_at: int
    # None while open; the DecisionOutcome name once a panel has decided.
    decided_outcome: Optional[str]


@dataclass(frozen=True)
class DesignationSummary:
    record_cid: str
    tier: str
    urgency: str
    criteria: str
    reversal_conditions: Optional[str]
    reexamine_after: Optional[int]
    reversed: bool
    designated_at: int


@dataclass(frozen=True)
class ReEntrySummary:
    record_cid: str
    designation_cid: str
    probation_until: int
    completed_at: int


@dataclass(frozen=True)
class RestitutionSummary:
    record_cid: str
    beneficiary: str
    terms: str
    satisfied: Optional[bool]
    imposed_at: int


@dataclass(frozen=True)
class SubstantiveReEngagementSummary:
    record_cid: str
    required_reaffirmation_by: int
    invoked_at: int


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
    conduct: tuple[ConductSummary, ...]
    sponsorships_given: tuple[SponsorshipSummary, ...]
    sponsorships_received: tuple[SponsorshipSummary, ...]
    disputes_filed: tuple[DisputeSummary, ...]
    disputes_against: tuple[DisputeSummary, ...]
    designations: tuple[DesignationSummary, ...]
    reentries: tuple[ReEntrySummary, ...]
    restitutions: tuple[RestitutionSummary, ...]
    substantive_reengagements: tuple[SubstantiveReEngagementSummary, ...]


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
        conduct=tuple(
            ConductSummary(
                record_cid=s.record_cid,
                kind=s.kind,
                bearing=s.bearing,
                severity=s.severity,
                ladder_rung=s.ladder_rung,
                decision_class=s.decision_class,
                reporters=tuple(s.reporters),
                reporter_perspective=s.reporter_perspective,
                references=tuple(s.references),
                recorded_at=s.recorded_at,
            )
            for s in v.conduct
        ),
        sponsorships_given=tuple(_sponsorship(s) for s in v.sponsorships_given),
        sponsorships_received=tuple(_sponsorship(s) for s in v.sponsorships_received),
        disputes_filed=tuple(_dispute(s) for s in v.disputes_filed),
        disputes_against=tuple(_dispute(s) for s in v.disputes_against),
        designations=tuple(
            DesignationSummary(
                record_cid=s.record_cid,
                tier=s.tier,
                urgency=s.urgency,
                criteria=s.criteria,
                reversal_conditions=s.reversal_conditions,
                reexamine_after=s.reexamine_after,
                reversed=s.reversed,
                designated_at=s.designated_at,
            )
            for s in v.designations
        ),
        reentries=tuple(
            ReEntrySummary(s.record_cid, s.designation_cid, s.probation_until, s.completed_at)
            for s in v.reentries
        ),
        restitutions=tuple(
            RestitutionSummary(s.record_cid, s.beneficiary, s.terms, s.satisfied, s.imposed_at)
            for s in v.restitutions
        ),
        substantive_reengagements=tuple(
            SubstantiveReEngagementSummary(
                s.record_cid, s.required_reaffirmation_by, s.invoked_at
            )
            for s in v.substantive_reengagements
        ),
    )


def _dispute(s) -> DisputeSummary:
    return DisputeSummary(
        record_cid=s.record_cid,
        complainant=s.complainant,
        respondent=s.respondent,
        rung=s.rung,
        current_rung=s.current_rung,
        summary=s.summary,
        filed_at=s.filed_at,
        decided_outcome=s.decided_outcome,
    )


def _sponsorship(s) -> SponsorshipSummary:
    return SponsorshipSummary(
        record_cid=s.record_cid,
        sponsor_community=s.sponsor_community,
        entrant_community=s.entrant_community,
        admission_cid=s.admission_cid,
        probation_until=s.probation_until,
        sponsored_at=s.sponsored_at,
    )


def reputation_of(archive, did: str, as_of: Optional[int] = None) -> ReputationView:
    """Project `archive` into a ReputationView for `did`.

    `archive` is a kith_covenant.Archive (the PyO3 class). Returns a
    frozen dataclass so callers can hash/share without worrying about the
    PyO3 object lifecycle.
    """
    native = archive.reputation_of(did, as_of)
    return _to_frozen(native)


# --- Capture-signal analytics ----------------------------------------------
#
# These are the jester's lenses (capture_resistance.md). Each surfaces flags
# with provenance (CIDs, counts, edges) and renders no verdict — adjudication
# is the mediator pool's job.


@dataclass(frozen=True)
class SponseeOutcome:
    entrant: str
    sponsorship_cid: str
    sponsored_at: int
    probation_until: int
    adverse_conduct: tuple[str, ...]


@dataclass(frozen=True)
class SponsorStakeReport:
    sponsor: str
    sponsees: tuple[SponseeOutcome, ...]
    total_sponsees: int
    tainted_sponsees: int
    taint_rate: Optional[float]


@dataclass(frozen=True)
class CompositionGraph:
    # (record_cid, sponsor, entrant, sponsored_at, probation_until)
    edges: tuple[tuple[str, str, str, int, int], ...]
    # (sponsor_did, out_degree), descending by out-degree
    by_concentration: tuple[tuple[str, int], ...]


@dataclass(frozen=True)
class FeatureConcernProfile:
    # (anti_spiral_feature, adverse_count)
    per_feature: tuple[tuple[str, int], ...]
    non_feature_adverse: int
    evidence: tuple[str, ...]
    perspectives: tuple[str, ...]


def composition_graph(archive, as_of: Optional[int] = None) -> CompositionGraph:
    """Sponsorship graph over the whole archive: edges + concentration ranking."""
    g = archive.composition_graph(as_of)
    return CompositionGraph(
        edges=tuple(tuple(e) for e in g.edges),
        by_concentration=tuple(tuple(c) for c in g.by_concentration),
    )


def sponsor_stake(archive, sponsor_did: str, as_of: Optional[int] = None) -> SponsorStakeReport:
    """For each community `sponsor_did` vouched for, the adverse conduct it
    accrued during probation ("sponsorship that repeatedly precedes misconduct
    is itself a capture signature")."""
    r = archive.sponsor_stake(sponsor_did, as_of)
    return SponsorStakeReport(
        sponsor=r.sponsor,
        sponsees=tuple(
            SponseeOutcome(
                entrant=o.entrant,
                sponsorship_cid=o.sponsorship_cid,
                sponsored_at=o.sponsored_at,
                probation_until=o.probation_until,
                adverse_conduct=tuple(o.adverse_conduct),
            )
            for o in r.sponsees
        ),
        total_sponsees=r.total_sponsees,
        tainted_sponsees=r.tainted_sponsees,
        taint_rate=r.taint_rate,
    )


def feature_concern_profile(
    archive, community_did: str, as_of: Optional[int] = None
) -> FeatureConcernProfile:
    """Per-anti-spiral-feature adverse-conduct counts for a community: the
    behavioral shadow of a hollow or absent substrate."""
    p = archive.feature_concern_profile(community_did, as_of)
    return FeatureConcernProfile(
        per_feature=tuple(tuple(x) for x in p.per_feature),
        non_feature_adverse=p.non_feature_adverse,
        evidence=tuple(p.evidence),
        perspectives=tuple(p.perspectives),
    )


# --- v0 jester lenses ------------------------------------------------------
#
# Read-only dashboard-facing views over the existing projection. Descriptive
# only: any "declined"/"widening" flag is a mechanical comparison of recorded
# numbers, never a capture verdict.


@dataclass(frozen=True)
class RejectionEvent:
    non_admission_cid: str
    rejecting: str
    presenting: str
    feature: str
    decided_at: int


@dataclass(frozen=True)
class RejectionPattern:
    events: tuple[RejectionEvent, ...]
    # (rejecting_did, count)
    by_rejector: tuple[tuple[str, int], ...]
    # (presenting_did, count)
    by_presenting: tuple[tuple[str, int], ...]
    # (anti_spiral_feature, count)
    by_feature: tuple[tuple[str, int], ...]
    # rejecting communities ranked by descending rejection count
    by_concentration: tuple[tuple[str, int], ...]
    total_rejections: int
    perspectives: tuple[str, ...]


@dataclass(frozen=True)
class CeremonyCostDecay:
    # (record_cid, sealed_at, witness_count), oldest first
    rites: tuple[tuple[str, int, int], ...]
    gaps_seconds: tuple[int, ...]
    seconds_since_last: Optional[int]
    rite_count: int
    first_witness_count: Optional[int]
    last_witness_count: Optional[int]
    min_witness_count: Optional[int]
    max_witness_count: Optional[int]
    witness_count_declined: bool
    latest_gap_widening: Optional[bool]


def rejection_pattern(archive, as_of: Optional[int] = None) -> RejectionPattern:
    """Compact-wide rejection tallies (admissions integrity): who rejects whom,
    on which anti-spiral feature, with the rejecting-community concentration
    ranking. Flags with provenance, no verdict."""
    p = archive.rejection_pattern(as_of)
    return RejectionPattern(
        events=tuple(
            RejectionEvent(e[0], e[1], e[2], e[3], e[4]) for e in p.events
        ),
        by_rejector=tuple(tuple(x) for x in p.by_rejector),
        by_presenting=tuple(tuple(x) for x in p.by_presenting),
        by_feature=tuple(tuple(x) for x in p.by_feature),
        by_concentration=tuple(tuple(x) for x in p.by_concentration),
        total_rejections=p.total_rejections,
        perspectives=tuple(p.perspectives),
    )


def ceremony_cost_decay(
    archive, community_did: str, as_of: Optional[int] = None
) -> CeremonyCostDecay:
    """A community's recognition-ceremony cost over time: witness-count and
    re-affirmation-gap trend plus staleness since the last rite. Descriptive
    only; the dashboard renders the trend, the mediator adjudicates."""
    d = archive.ceremony_cost_decay(community_did, as_of)
    return CeremonyCostDecay(
        rites=tuple(tuple(r) for r in d.rites),
        gaps_seconds=tuple(d.gaps_seconds),
        seconds_since_last=d.seconds_since_last,
        rite_count=d.rite_count,
        first_witness_count=d.first_witness_count,
        last_witness_count=d.last_witness_count,
        min_witness_count=d.min_witness_count,
        max_witness_count=d.max_witness_count,
        witness_count_declined=d.witness_count_declined,
        latest_gap_widening=d.latest_gap_widening,
    )
