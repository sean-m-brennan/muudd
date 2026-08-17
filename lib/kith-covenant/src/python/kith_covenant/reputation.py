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
class InjunctionSummary:
    record_cid: str
    harm: str
    protective_measures: str
    revoked_capabilities: tuple[str, ...]
    # One of "Active", "Confirmed", "Lapsed", "Overturned".
    status: str
    issued_at: int
    expires_at: int


@dataclass(frozen=True)
class CommonsSanctionSummary:
    record_cid: str
    commons_cid: str
    kind: str
    # One of "Monitoring", "Warning", "Restriction", "Exclusion".
    current_level: str
    basis: str
    decided_at: int


@dataclass(frozen=True)
class ReportResponseSummary:
    record_cid: str
    response: str
    responded_at: int


@dataclass(frozen=True)
class ObserverReportSummary:
    record_cid: str
    observer_did: str
    declared_perspective: str
    summary: str
    observed_at: int
    reported_at: int
    responses: tuple[ReportResponseSummary, ...]


@dataclass(frozen=True)
class WhistleblowerSummary:
    record_cid: str
    claim: str
    filed_at: int
    # True once a for-cause reveal has disclosed the source; the source itself
    # is never projected.
    source_revealed: bool
    # The filing's anonymous source tag: its ring-signature key image, in hex. The
    # one publishable fact about a source — equal across every filing by the same
    # insider, and tied to no identity.
    source_tag: str
    # How many members have published a proof that this filing's key image is not
    # theirs. Non-zero after a reveal means the disclosure is contested by
    # arithmetic, not merely denied.
    exculpations: int


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
    injunctions: tuple[InjunctionSummary, ...]
    # Capabilities currently revoked (union across Active/Confirmed injunctions),
    # sorted. The effective set is CAPABILITIES minus these.
    revoked_capabilities: tuple[str, ...]
    # Commons on which a graduated sanction currently stands (§7), at their
    # current level, oldest first. Commons at rest (level None) are omitted.
    commons_sanctions: tuple[CommonsSanctionSummary, ...]
    # Attributed observer reports naming this community as subject (§8), each with
    # its appended responses, oldest first.
    observer_reports: tuple[ObserverReportSummary, ...]
    # Whistleblower reports naming this community as accused (§8), oldest first.
    # The source is never included.
    whistleblower_reports_against: tuple[WhistleblowerSummary, ...]


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
        injunctions=tuple(
            InjunctionSummary(
                record_cid=s.record_cid,
                harm=s.harm,
                protective_measures=s.protective_measures,
                revoked_capabilities=tuple(s.revoked_capabilities),
                status=s.status,
                issued_at=s.issued_at,
                expires_at=s.expires_at,
            )
            for s in v.injunctions
        ),
        revoked_capabilities=tuple(v.revoked_capabilities),
        commons_sanctions=tuple(
            CommonsSanctionSummary(
                record_cid=s.record_cid,
                commons_cid=s.commons_cid,
                kind=s.kind,
                current_level=s.current_level,
                basis=s.basis,
                decided_at=s.decided_at,
            )
            for s in v.commons_sanctions
        ),
        observer_reports=tuple(
            ObserverReportSummary(
                record_cid=s.record_cid,
                observer_did=s.observer_did,
                declared_perspective=s.declared_perspective,
                summary=s.summary,
                observed_at=s.observed_at,
                reported_at=s.reported_at,
                responses=tuple(
                    ReportResponseSummary(r.record_cid, r.response, r.responded_at)
                    for r in s.responses
                ),
            )
            for s in v.observer_reports
        ),
        whistleblower_reports_against=tuple(
            WhistleblowerSummary(s.record_cid, s.claim, s.filed_at, s.source_revealed,
                                 s.source_tag, s.exculpations)
            for s in v.whistleblower_reports_against
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


def has_capability(
    archive, did: str, capability: str, as_of: Optional[int] = None
) -> bool:
    """§9 capability gate: True unless `capability` is currently revoked from
    `did` by an active or confirmed interim injunction, as of `as_of` (default
    now). `capability` must be one of `authoring.CAPABILITIES`."""
    return archive.has_capability(did, capability, as_of)


def commons_level(
    archive, did: str, commons_cid: str, as_of: Optional[int] = None
) -> str:
    """§7 commons gate: the current SanctionLevel name standing against `did` on
    the commons at `commons_cid`, as of `as_of` (default now). Returns "None"
    when no sanction stands; a caller enforcing a restriction checks whether the
    result is "Restriction" or "Exclusion"."""
    return archive.commons_level(did, commons_cid, as_of)


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


@dataclass(frozen=True)
class DeliberativeCapacity:
    """How much of a compact's roll carries second-class weight (§2).

    Probation withholds weight from every second-class quorum, so a compact
    that has just admitted a cohort holds fewer *deciding* members than
    members — and if it admits enough at once, `can_act` goes False and no
    second-class decision is available until a window closes. That is the
    withholding working, not a fault, and it is reported so the refusal it
    causes is accountable.
    """

    compact_label: str
    members: tuple[str, ...]
    probationary: tuple[str, ...]
    standing: int
    # None for an empty roll: not evaluated, rather than a zero share.
    standing_share: Optional[float]
    quorum_threshold: int
    can_act: bool
    can_contract: bool
    as_of: int


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
    # pinned to the FIRST gap; stays true after a settled drift, where the
    # rolling reading above goes quiet
    gap_widening_since_founding: Optional[bool]


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


@dataclass(frozen=True)
class UsageTally:
    community: str
    # Summed over this community's PUBLIC records only. A nonzero `sealed_count`
    # means these understate the real draw.
    consumed: int
    contributed: int
    # Every record filed, sealed or public.
    record_count: int
    # How many were sealed, and so counted here without being summed.
    sealed_count: int
    evidence: tuple[str, ...]


@dataclass(frozen=True)
class FreeRiderReport:
    tallies: tuple[UsageTally, ...]
    total_consumed: int
    total_contributed: int
    # (community_did, net_draw), descending net draw, for net-positive drawers.
    by_net_draw: tuple[tuple[str, int], ...]
    # Whether every record is public, so the totals are the whole account and not a floor.
    is_complete: bool
    # (community_did, sealed_record_count) for communities that sealed at least one.
    partially_sealed: tuple[tuple[str, int], ...]
    # The most the sealed records could hide: sealed x allowance. None when the commons
    # states no allowance, which is also when nothing on it can be sealed.
    max_hidden_draw: Optional[int]
    # The commons' per-period draw allowance, if it states one.
    allowance: Optional[int]


def free_rider_signals(
    archive, commons_cid: str, as_of: Optional[int] = None
) -> FreeRiderReport:
    """§7 free-rider lens over a commons's usage records (Ostrom monitoring /
    tragedy-of-the-virtues): per-appropriator consumed/contributed tallies plus
    the net-draw ranking. Surfaces the asymmetry between what communities drew and
    returned; renders no verdict on whether it is free-riding.

    The tallies sum **public** records only. A community accounting under §7's
    surveillance threshold seals its figures, and a sealed record is counted in
    `record_count` and `sealed_count` without being summed, so `is_complete` says whether
    the totals are the whole account or a floor, and `max_hidden_draw` turns the sealed
    records' bound proofs into the worst case a reader has to reason about. Without that
    a sealed record would simply vanish from the tally, and sealing would be a way to
    disappear from the detector."""
    r = archive.free_rider_signals(commons_cid, as_of)
    return FreeRiderReport(
        tallies=tuple(
            UsageTally(
                community=t[0],
                consumed=t[1],
                contributed=t[2],
                record_count=t[3],
                sealed_count=t[4],
                evidence=tuple(t[5]),
            )
            for t in r.tallies
        ),
        total_consumed=r.total_consumed,
        total_contributed=r.total_contributed,
        by_net_draw=tuple(tuple(x) for x in r.by_net_draw),
        is_complete=r.is_complete,
        partially_sealed=tuple(tuple(x) for x in r.partially_sealed),
        max_hidden_draw=r.max_hidden_draw,
        allowance=r.allowance,
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
        gap_widening_since_founding=d.gap_widening_since_founding,
    )


@dataclass(frozen=True)
class InterpretivePluralityReport:
    # (record_cid, lineage_id, interpreter, recorded_at), oldest first
    interpretations: tuple[tuple[str, str, str, int], ...]
    # (lineage_id, count)
    lineages: tuple[tuple[str, int], ...]
    # (interpreter_did, count), descending — the concentration ranking
    interpreter_concentration: tuple[tuple[str, int], ...]
    distinct_lineages: int
    distinct_interpreters: int
    plurality_demonstrated: bool
    # (interpreter_did, (attester_did, ...)) — distinct third parties who attested
    # each interpreter's independence (move 1 anti-sockpuppet)
    independent_attesters: tuple[tuple[str, tuple[str, ...]], ...]
    distinct_independent_interpreters: int
    independent_plurality_demonstrated: bool


def interpretive_plurality(
    archive, community_did: str, as_of: Optional[int] = None
) -> InterpretivePluralityReport:
    """Move 1 lens: a community's interpretive lineages of its own substrate —
    per-lineage and per-interpreter counts plus the interpreter-concentration
    ranking. A single interpreter/lineage cornering the function is the capture
    signature; `plurality_demonstrated` is the positive signal move 1 asks for.
    No verdict — the monopoly threshold is the caller's."""
    r = archive.interpretive_plurality(community_did, as_of)
    return InterpretivePluralityReport(
        interpretations=tuple(tuple(p) for p in r.interpretations),
        lineages=tuple(tuple(x) for x in r.lineages),
        interpreter_concentration=tuple(tuple(x) for x in r.interpreter_concentration),
        distinct_lineages=r.distinct_lineages,
        distinct_interpreters=r.distinct_interpreters,
        plurality_demonstrated=r.plurality_demonstrated,
        independent_attesters=tuple(
            (k, tuple(atts)) for k, atts in r.independent_attesters
        ),
        distinct_independent_interpreters=r.distinct_independent_interpreters,
        independent_plurality_demonstrated=r.independent_plurality_demonstrated,
    )


@dataclass(frozen=True)
class EventProvenance:
    event_cid: str
    originator: str
    reported_at: int
    qualifies: bool
    triggers: tuple[str, ...]
    provenance_complete: bool
    # distinct corroborating communities, excluding the originator
    corroborators: tuple[str, ...]
    # the subset of corroborators that supplied a cross-substrate translation; an
    # interpretation surviving only in the originating framing is suspect (move 4)
    translated_corroborators: tuple[str, ...]
    # retroactive (finder_did, undisclosed_interest) findings against the
    # originator, from finders other than the originator (move 4 bad-faith signal)
    undisclosed_interests_found: tuple[tuple[str, str], ...]
    aligned_faction: Optional[str]


@dataclass(frozen=True)
class SkepticismReport:
    events: tuple[EventProvenance, ...]
    qualifying_events: int

    def under_corroborated(self, minimum: int) -> tuple[str, ...]:
        """CIDs of qualifying events with fewer than `minimum` distinct
        corroborators — the loudest-interpreter window still open. The threshold
        is the caller's policy, not the detector's."""
        return tuple(
            e.event_cid
            for e in self.events
            if e.qualifies and len(e.corroborators) < minimum
        )

    def under_translated(self, minimum: int) -> tuple[str, ...]:
        """CIDs of qualifying events with fewer than `minimum` distinct
        corroborators supplying a cross-substrate translation — interpretations
        still surviving only in the originating framing. Distinct from
        `under_corroborated`: an event can be well-corroborated in its native
        terms yet never translated, which is its own capture signal."""
        return tuple(
            e.event_cid
            for e in self.events
            if e.qualifies and len(e.translated_corroborators) < minimum
        )

    def retroactively_impugned(self) -> tuple[str, ...]:
        """CIDs of qualifying events for which a third party has retroactively
        found an undisclosed originator interest — events now shown to be authored
        in bad faith. Independent of corroboration; the consequence is conduct's,
        this only surfaces the finding."""
        return tuple(
            e.event_cid
            for e in self.events
            if e.qualifies and e.undisclosed_interests_found
        )


def structured_skepticism(archive, as_of: Optional[int] = None) -> SkepticismReport:
    """Move 4 lens: every event report's provenance tags, structured-skepticism
    qualifying triggers, and distinct-community corroborators (excluding the
    originator — self-corroboration does not count). A qualifying event that is
    under-corroborated is one whose loudest interpreter has not yet been checked.
    No verdict; the corroboration threshold is the caller's."""
    r = archive.structured_skepticism(as_of)
    return SkepticismReport(
        events=tuple(
            EventProvenance(
                event_cid=e[0],
                originator=e[1],
                reported_at=e[2],
                qualifies=e[3],
                triggers=tuple(e[4]),
                provenance_complete=e[5],
                corroborators=tuple(e[6]),
                translated_corroborators=tuple(e[7]),
                undisclosed_interests_found=tuple(tuple(x) for x in e[8]),
                aligned_faction=e[9],
            )
            for e in r.events
        ),
        qualifying_events=r.qualifying_events,
    )


@dataclass(frozen=True)
class CrossWitness:
    record_cid: str
    witness_community: str
    attested_at: int


@dataclass(frozen=True)
class OutputDivergence:
    output_cid: str
    cohort: str
    matter: str
    principal_decision: str
    cohort_majority: str
    peers_compared: int
    favors_home: bool


@dataclass(frozen=True)
class PrincipalValidationReport:
    # cross-community witnesses (excluding the principal and its home community)
    cross_witnesses: tuple[CrossWitness, ...]
    divergences: tuple[OutputDivergence, ...]
    distinct_cross_witnesses: int
    self_benefiting_divergences: int
    # continuous-behavior triangulation: outputs that had a peer cohort, and the
    # fraction of them that self-dealt (move 6 statistical signal)
    outputs_triangulated: int
    self_dealing_rate: float

    def systematic_self_dealing(self, min_outputs: int, rate_threshold: float) -> bool:
        """Whether the principal shows *systematic* self-dealing: enough
        triangulated outputs to be meaningful and a self-dealing rate at or above
        `rate_threshold`. Both bounds are caller policy; the lens gives no verdict."""
        return (
            self.outputs_triangulated >= min_outputs
            and self.self_dealing_rate >= rate_threshold
        )


def adversarial_principal(
    archive, principal_did: str, as_of: Optional[int] = None
) -> PrincipalValidationReport:
    """Move 6 lens: a principal's cross-community witness attestations (validity
    that does not rest on its own or its home community's word) and the matters
    where its machine's output diverged from its cohort's majority — flagged
    `favors_home` when the divergence benefits the principal's own side. Also
    aggregates that into a continuous-behavior `self_dealing_rate` over the
    triangulated outputs. No verdict; the caller decides what count/rate is
    actionable."""
    r = archive.adversarial_principal(principal_did, as_of)
    return PrincipalValidationReport(
        cross_witnesses=tuple(
            CrossWitness(w[0], w[1], w[2]) for w in r.cross_witnesses
        ),
        divergences=tuple(
            OutputDivergence(d[0], d[1], d[2], d[3], d[4], d[5], d[6])
            for d in r.divergences
        ),
        distinct_cross_witnesses=r.distinct_cross_witnesses,
        self_benefiting_divergences=r.self_benefiting_divergences,
        outputs_triangulated=r.outputs_triangulated,
        self_dealing_rate=r.self_dealing_rate,
    )


@dataclass(frozen=True)
class CohortDivergence:
    cohort: str
    matter: str
    cohort_majority: str
    cross_cohort_consensus: str
    cohorts_compared: int
    # the cohort's majority position benefits a strict majority of its own members
    self_benefiting: bool


@dataclass(frozen=True)
class CohortCollusionReport:
    divergences: tuple[CohortDivergence, ...]
    self_benefiting_cohort_divergences: int


def collusive_cohorts(archive, as_of: Optional[int] = None) -> CohortCollusionReport:
    """Move 6 lens (cross-cohort): per-principal triangulation compares a principal
    against its own cohort's majority, so a wholesale-colluding cohort shows no
    internal divergence. This lifts the comparison a level — for each matter decided
    by two or more cohorts, each cohort's majority vs. the cross-cohort consensus —
    and surfaces cohorts that diverge, flagging `self_benefiting` when the bloc
    position benefits the cohort's own members. No verdict; the threshold is the
    caller's."""
    r = archive.collusive_cohorts(as_of)
    return CohortCollusionReport(
        divergences=tuple(
            CohortDivergence(d[0], d[1], d[2], d[3], d[4], d[5]) for d in r.divergences
        ),
        self_benefiting_cohort_divergences=r.self_benefiting_cohort_divergences,
    )


@dataclass(frozen=True)
class WitnessLink:
    cid: str
    witness_community: str
    # the prior attestation's CID in this principal's chain, or None for the root
    prev: Optional[str]
    attested_at: int
    # the declared `prev` names another link present in this principal's chain
    prev_resolved: bool
    # signed by a community other than the principal and its home — separable
    cross_community: bool


@dataclass(frozen=True)
class WitnessChainReport:
    # the principal's attestation links, oldest first
    links: tuple[WitnessLink, ...]
    # cross-community (separable) links the home community cannot forge
    separable_links: int
    # links whose declared `prev` dangles (no matching predecessor)
    broken_links: int
    # at most one root and no forks
    is_linear: bool
    # no dangling `prev` and linear
    is_intact: bool


def principal_witness_chain(
    archive, principal_did: str, as_of: Optional[int] = None
) -> WitnessChainReport:
    """Move 6 lens: the witness-attestation chain for a principal — its
    attestation links oldest-first, made tamper-evident by `prev` hash-links (each
    entry CID commits its content incl. `prev`, so altering or reordering a link
    dangles every pointer after it). Reports chain integrity (`broken_links`,
    `is_linear`, `is_intact`) and separability (`separable_links`, the
    cross-community attestations the home community cannot forge). No verdict.

    Honest boundary: this does not defeat shared-key sockpuppets (a witness
    colluding with the home, or a home controlling a second key) — that is move-1
    third-party-verification territory; the chain proves only that distinct keys
    attested and the sequence is unbroken."""
    r = archive.principal_witness_chain(principal_did, as_of)
    return WitnessChainReport(
        links=tuple(
            WitnessLink(l[0], l[1], l[2], l[3], l[4], l[5]) for l in r.links
        ),
        separable_links=r.separable_links,
        broken_links=r.broken_links,
        is_linear=r.is_linear,
        is_intact=r.is_intact,
    )


@dataclass(frozen=True)
class IncapableMatter:
    key: str
    description: str


@dataclass(frozen=True)
class FastPeriod:
    label: str
    start: int
    end: int
    # cycle in seconds for a repeating fast (a calendar rhythm), or None for a
    # single window
    recurrence: Optional[int] = None


@dataclass(frozen=True)
class Forbearance:
    key: str
    description: str
    # one of MINIMUM_SITES: where in the compact's conduct the forbearance operates
    site: str


@dataclass(frozen=True)
class ConstitutionState:
    ratified: bool
    compact_incapable: tuple[IncapableMatter, ...]
    fast_periods: tuple[FastPeriod, ...]
    # whether the compact has adopted the functional minimum as the ground of its own
    # conduct — a separate act from ratification
    minimum_adopted: bool = False
    # the forbearances in force; never compact-incapable matters, and none immutable
    minimum_content: tuple[Forbearance, ...] = ()


@dataclass(frozen=True)
class FastDeviation:
    record_cid: str
    kind: str
    at: int
    fast_label: str


@dataclass(frozen=True)
class SubstrateDeferenceReport:
    ratified: bool
    incapable_current: int
    incapable_added: int
    incapable_removed: int
    # True when any protection has been rolled back (a matter removed or a fast
    # contracted) — the erosion signal, surfaced not judged.
    contraction_detected: bool
    fast_period_count: int
    fast_seconds_total: int
    fast_contractions: int
    fast_deviations: tuple[FastDeviation, ...]
    # forbearances in force, and the two directions of amendment. A narrowing is folded
    # into contraction_detected: the pressure on the compact's own ground is the same
    # one-way pressure the deference instruments carry.
    minimum_current: int = 0
    minimum_broadened: int = 0
    minimum_narrowed: int = 0


def constitution_state(archive, as_of: Optional[int] = None) -> ConstitutionState:
    """§9 constitutional state as of `as_of`: whether ratified, the in-force
    compact-incapable matters, and the current structured-fast cadence."""
    s = archive.constitution_state(as_of)
    return ConstitutionState(
        ratified=s.ratified,
        compact_incapable=tuple(IncapableMatter(k, d) for (k, d) in s.compact_incapable),
        fast_periods=tuple(
            FastPeriod(lbl, start, end, recurrence)
            for (lbl, start, end, recurrence) in s.fast_periods
        ),
        minimum_adopted=s.minimum_adopted,
        minimum_content=tuple(Forbearance(k, d, site) for (k, d, site) in s.minimum_content),
    )


def minimum_forbearance(archive, forbearance_key: str, as_of: Optional[int] = None) -> bool:
    """§9 self-binding gate: True if `forbearance_key` is part of the ground the
    compact has bound its own conduct to, as of `as_of`. The twin of
    `compact_incapable`, and the two never answer each other's question: that one is
    about a matter belonging to a member, this one about what the compact will not do
    itself where consent has run out."""
    return archive.minimum_forbearance(forbearance_key, as_of)


def compact_incapable(archive, matter_key: str, as_of: Optional[int] = None) -> bool:
    """§9 categorical-deference gate: True if `matter_key` is a matter the compact
    will not decide, as of `as_of`. A caller contemplating a first-order decision
    on a member's internal affairs checks this and declines when True."""
    return archive.compact_incapable(matter_key, as_of)


def in_structured_fast(archive, at: int) -> Optional[str]:
    """§9 temporal-deference gate: the label of the structured-fast period
    containing `at`, or None if the compact is not fasting then."""
    return archive.in_structured_fast(int(at))


def substrate_deference(archive, as_of: Optional[int] = None) -> SubstrateDeferenceReport:
    """§9 substrate-deference / erosion report: the size and drift of the
    compact-incapable list, the fast cadence and its contraction count, and
    deliberative acts initiated during a fast. Makes the compact's non-ultimacy
    legible; renders no verdict."""
    return _substrate_from(archive.substrate_deference(as_of))


def _substrate_from(r) -> SubstrateDeferenceReport:
    return SubstrateDeferenceReport(
        ratified=r.ratified,
        incapable_current=r.incapable_current,
        incapable_added=r.incapable_added,
        incapable_removed=r.incapable_removed,
        contraction_detected=r.contraction_detected,
        fast_period_count=r.fast_period_count,
        fast_seconds_total=r.fast_seconds_total,
        fast_contractions=r.fast_contractions,
        fast_deviations=tuple(
            FastDeviation(d[0], d[1], d[2], d[3]) for d in r.fast_deviations
        ),
        minimum_current=r.minimum_current,
        minimum_broadened=r.minimum_broadened,
        minimum_narrowed=r.minimum_narrowed,
    )


@dataclass(frozen=True)
class OverdueReaffirmation:
    community_did: str
    # the last sealed rite, so the finding traces to a signed record
    last_rite_cid: str
    seconds_since_last: int
    # the cadence it was judged against, reported rather than assumed
    cadence_seconds: int


@dataclass(frozen=True)
class SilentFunction:
    community_did: str
    # "Mediation" or "Observation"
    function: str
    # seconds since the last act or attested null. Measured from the seat's start
    # when there has never been either: a function that has done nothing since
    # the day it was staffed is the case the audit is most interested in
    seconds_since: int
    cadence_seconds: int
    # True when the last thing on the record was an attested null rather than an
    # act: the compact said "nothing happened", which is evidence, and then
    # stopped saying even that
    last_was_a_null: bool


@dataclass(frozen=True)
class UnstaffedFunction:
    community_did: str
    function: str
    # when the last seat in this function expired
    vacant_since: int
    seconds_vacant: int


def silent_functions(
    archive,
    as_of: Optional[int] = None,
    cadence_seconds: Optional[int] = None,
) -> tuple[SilentFunction, ...]:
    """Standing functions that have gone quiet: no act and no attested null inside the
    cadence, longest silence first (erosion remediation items 5 and 6).

    An **absolute** measure, which is the whole point. Every relative reading of a
    mediator pool or an observer corps is unchanged by the pool simply stopping: the
    shares are the same shares, the concentration is the same concentration, and a corps
    with nothing to report and a corps that stopped looking file the same number of
    reports, namely none.

    The attested null is what makes the reading fair. Before ``attest_activity`` existed a
    quiet pool had no way to say so, and reporting its silence would have been reporting
    missing data as a finding. Now silence is either evidence (an attestation inside the
    cadence) or a fact about the compact, and this reports only the second. No verdict and
    no refusal: a compact that has stopped acting offers no act to refuse."""
    return tuple(
        SilentFunction(did, fn, since, cadence, was_null)
        for did, fn, since, cadence, was_null
        in archive.silent_functions(as_of, cadence_seconds)
    )


def unstaffed_functions(
    archive, as_of: Optional[int] = None
) -> tuple[UnstaffedFunction, ...]:
    """Standing functions a community once staffed and now has nobody seated in, longest
    vacancy first.

    The sharper half of the same failure, and the one no remediation item had named.
    Measuring found that a compact with **zero** active mediators and **zero** active
    observers for forty-three years, keeping its re-affirmation rites perfectly, read
    clean on every instrument: ``deliberative_capacity`` reads the member roll and
    probation, the pool lenses report shares of an empty pool without remarking that it is
    empty, and no alert dimension looked at staffing at all.

    A function **never** staffed is absent from this reading. A compact that never seated
    observers has not abandoned the office, it declined to open one, and the absence of an
    office is not the same as an abandoned one."""
    return tuple(
        UnstaffedFunction(did, fn, vacant_since, seconds)
        for did, fn, vacant_since, seconds in archive.unstaffed_functions(as_of)
    )


@dataclass(frozen=True)
class OverdueReexamination:
    designation_cid: str
    # the designated community, whose exclusion is being left unexamined
    subject: str
    # "Anathema"; HostisHumaniGeneris is forbidden a re-examination date
    tier: str
    # the deadline standing UNANSWERED: the designation's own until a
    # re-examination confirms it, then the date that confirmation set
    reexamine_after: int
    seconds_overdue: int
    # how many times the designation has been re-examined and confirmed. A
    # designation overdue on its third renewal and one nobody ever looked at were
    # the same number before the conclusion record existed: zero
    confirmations: int


def overdue_reexaminations(
    archive, as_of: Optional[int] = None
) -> tuple[OverdueReexamination, ...]:
    """§5 designation follow-through: designations whose mandated
    ``reexamine_after`` has passed and which are still standing, longest-overdue
    first. "Still standing" means not reversed — no ``DesignationLifted`` conduct
    record naming it and no re-entry answering it.

    The limit this used to carry is closed. A re-examination that *confirmed* a
    designation left no record, so the reading could not distinguish "nobody looked"
    from "the panel looked and let it stand" and reported the weaker claim about
    both: the lens punished the diligent panel. ``conclude_reexamination`` is the
    signed record the erosion audit asked for (remediation item 5), so
    ``reexamine_after`` here is the deadline standing *unanswered* and
    ``confirmations`` counts the renewals behind it."""
    return tuple(
        OverdueReexamination(cid, subject, tier, due, overdue, confirmations)
        for cid, subject, tier, due, overdue, confirmations
        in archive.overdue_reexaminations(as_of)
    )


@dataclass(frozen=True)
class ThinningDeliberation:
    """Why the compact's deliberation reads as thinning, with both readings carried
    so a corps member can see which one fired.

    Read against the compact's own **first** decision, never its recent average: a
    deliberation that thins a little each time drags a rolling baseline behind it,
    and only an absolute measure sees a drift with nobody driving it."""

    decisions: int
    first_argument_count: int
    latest_argument_count: int
    first_span_seconds: Optional[int]
    latest_span_seconds: Optional[int]
    # the latest decision answered fewer arguments than the first
    argument_count_declined: bool
    # the latest decision was reached in less time than the first
    span_shortened: bool


@dataclass(frozen=True)
class EnforcementConvergence:
    """What a compact's enforcement is aimed at over a window: how many acts, how
    much of the shared taxonomy they still cite, and how widely they are spread
    over subject communities.

    The substitution reading (``doc/background/substitution.md``). A compact keeping
    every form and intensifying it — rites ahead of cadence, witness counts rising,
    nothing amended, no office vacant — can still narrow what it actually enforces
    down to a single ground, and every instrument that counts observances reads that
    polity as healthy. This counts what the observances are *about*."""

    # the enforcing community, or None for the compact-wide reading
    community: Optional[str]
    volume: int
    # concrete ground -> acts citing it; grounds never cited are absent
    per_bearing: dict[str, int]
    # Other(_) ground string -> acts citing it
    per_other: dict[str, int]
    # subject community -> acts aimed at it
    per_subject: dict[str, int]
    window_seconds: int
    evidence: tuple[str, ...]
    # distinct concrete grounds cited over the whole concrete taxonomy. Absolute by
    # construction: the denominator is a fixed enum, never the compact's own recent
    # behaviour. Other(_) is outside it, so a polity filing everything off-taxonomy
    # scores 0.0 here — which is why no separate off-taxonomy rule is needed
    coverage: float
    # Herfindahl over grounds; None when there is nothing to index
    concentration: Optional[float]
    # Herfindahl over subject communities. THE DISCRIMINATOR: a polity with one
    # genuinely recurring problem also shows high volume and low coverage, and what
    # separates it is that legitimate concentration lands on a TARGET while
    # substitution concentrates on a GROUND across many targets
    target_dispersion: Optional[float]
    other_share: Optional[float]


@dataclass(frozen=True)
class EnforcementDirection:
    """Which way a compact's enforcement runs: at communities holding reduced
    standing, at communities holding full standing, or between equals.

    The Girardian half of the substitution reading. A narrowed ground does not land
    evenly; it lands on whoever is least able to refuse it, and §2 probation is the
    compact's own name for reduced standing, so the axis is already on the record."""

    community: Optional[str]
    # subject in probation, no reporter in probation
    downward: int
    # a reporter in probation, subject not
    upward: int
    # neither in probation, or both. Enforcement between equals
    level: int
    window_seconds: int
    # the downward acts only
    evidence: tuple[str, ...]
    total: int
    downward_share: Optional[float]


def _convergence_from(c) -> Optional[EnforcementConvergence]:
    if c is None:
        return None
    return EnforcementConvergence(
        community=c.community,
        volume=c.volume,
        per_bearing=dict(c.per_bearing),
        per_other=dict(c.per_other),
        per_subject=dict(c.per_subject),
        window_seconds=c.window_seconds,
        evidence=tuple(c.evidence),
        coverage=c.coverage,
        concentration=c.concentration,
        target_dispersion=c.target_dispersion,
        other_share=c.other_share,
    )


def _direction_from(d) -> Optional[EnforcementDirection]:
    if d is None:
        return None
    return EnforcementDirection(
        community=d.community,
        downward=d.downward,
        upward=d.upward,
        level=d.level,
        window_seconds=d.window_seconds,
        evidence=tuple(d.evidence),
        total=d.total,
        downward_share=d.downward_share,
    )


def enforcement_convergence(
    archive, as_of: Optional[int] = None, window_seconds: Optional[int] = None
) -> EnforcementConvergence:
    """The compact-wide enforcement reading over the window ending at ``as_of``.

    **This is the level the erosion alert reads, and the per-community map is not.**
    Measured: a compact whose communities each specialise — one filing only on
    ``Charity``, another only on ``Rescue`` — scores as converged on every single
    per-community reading while the compact as a whole still enforces the entire
    object. That is division of labour in an observer corps, not a narrowing of what
    the compact holds, and a per-community trigger would report it as erosion.

    No verdict, like every lens here: the numbers come with the CIDs behind them and
    the cut-points are applied by ``erosion_alert``."""
    return _convergence_from(archive.enforcement_convergence(as_of, window_seconds))


def enforcement_convergence_all(
    archive, as_of: Optional[int] = None, window_seconds: Optional[int] = None
) -> dict[str, EnforcementConvergence]:
    """The same reading per **enforcing** community, keyed by reporter DID.

    The drill-down that names who, after the compact-wide reading has said whether
    there is anything to name. An act filed by three reporters counts once for each
    of them: all three filed it."""
    return {
        did: _convergence_from(c)
        for did, c in archive.enforcement_convergence_all(as_of, window_seconds).items()
    }


def enforcement_direction(
    archive, as_of: Optional[int] = None, window_seconds: Optional[int] = None
) -> EnforcementDirection:
    """Which way the compact's enforcement runs over the window ending at ``as_of``.

    Standing is resolved at the instant each act was recorded rather than at
    ``as_of``: the question is who held what standing when the compact moved against
    them."""
    return _direction_from(archive.enforcement_direction(as_of, window_seconds))


def enforcement_direction_all(
    archive, as_of: Optional[int] = None, window_seconds: Optional[int] = None
) -> dict[str, EnforcementDirection]:
    """The same reading per **enforcing** community, keyed by reporter DID."""
    return {
        did: _direction_from(d)
        for did, d in archive.enforcement_direction_all(as_of, window_seconds).items()
    }


@dataclass(frozen=True)
class ErosionAlert:
    # "None" | "Notice" | "Alert" — escalates as erosion signals compound.
    severity: str
    # one human-readable reason per active erosion signal
    reasons: tuple[str, ...]
    list_contraction: bool
    # a forbearance narrowed out of the compact's own adopted ground
    minimum_contraction: bool
    fast_contraction: bool
    active_fast_deviations: int
    # communities overdue for re-affirmation, longest-overdue first
    overdue_reaffirmations: tuple[OverdueReaffirmation, ...]
    # communities drawing fewer witnesses than at founding
    hollowing_communities: tuple[str, ...]
    # the compact's deliberation is thinner than its own first decision
    thinning_deliberation: Optional[ThinningDeliberation]
    # designations past a mandated re-examination and still standing
    overdue_reexaminations: tuple[OverdueReexamination, ...]
    # standing functions with no act and no attested null inside the cadence
    silent_functions: tuple[SilentFunction, ...]
    # standing functions with nobody seated at all
    unstaffed_functions: tuple[UnstaffedFunction, ...]
    # enforcement narrowed onto part of the taxonomy while spread across subjects
    enforcement_convergence: Optional[EnforcementConvergence]
    # enforcement running at communities holding reduced standing
    enforcement_direction: Optional[EnforcementDirection]


def erosion_alert(
    archive,
    as_of: Optional[int] = None,
    rite_cadence_seconds: Optional[int] = None,
    attestation_cadence_seconds: Optional[int] = None,
    enforcement_window_seconds: Optional[int] = None,
    enforcement_volume_floor: Optional[int] = None,
    enforcement_coverage_floor_permille: Optional[int] = None,
    enforcement_dispersion_ceiling_permille: Optional[int] = None,
    enforcement_downward_floor_permille: Optional[int] = None,
) -> ErosionAlert:
    """§9 erosion-detection alerting: a graded, derived reading that escalates
    (None→Notice→Alert) as the erosion signals compound.

    Twelve signals, divided three ways, and the division is the whole design. Four fire
    on an *act*: a contraction of the compact-incapable list, a narrowing of the compact's
    own adopted ground, a fast contraction, deliberative acts during a fast. Six fire on
    *omission against a reference that is not the current population*, which is the only
    kind that sees a drift with nobody driving it: a re-affirmation overdue against an
    absolute generational cadence, a ceremony drawing fewer witnesses than it did at
    founding, a deliberation thinner than the compact's first, a designation past a
    mandated re-examination, a standing function that has neither acted nor attested a
    null inside its cadence, and a standing function with nobody seated in it at all. Two
    fire on *intensification*: a compact enforcing harder and narrower than it recites,
    and enforcement running at communities holding reduced standing.

    The asymmetry is deliberate. An alert built only from act-triggered signals reads a
    polity that has stopped functioning as a polity in perfect health; one built only from
    those two families reads a polity that has kept every form and substituted a smaller
    object underneath them as its most vigorous member (see
    ``doc/architecture/erosion-legibility.md`` and ``doc/background/substitution.md``).

    ``rite_cadence_seconds`` overrides the generational cadence (default 30 years) and
    ``attestation_cadence_seconds`` the function cadence (default 1 year). The five
    ``enforcement_*`` parameters override the substitution cut-points: the window the
    reading looks back over (default 5 years), the fewest acts it will read at all
    (default 12), and the coverage floor, target-dispersion ceiling and downward-share
    floor in permille (default 500, 500, 800). Every one of these calibrations is an open
    question in ``protocol-components.md``, so they are parameters rather than constants.

    An unstaffed function is *not* also counted as silent. It cannot attest, so a lapsed
    roster would otherwise fire two dimensions for one lapse and reach ``Alert`` on
    compounding alone; emptiness is the stronger and more specific claim.

    Diagnostic only: surfaced to the observer corps, never a verdict and never a
    refusal (non-ultimacy is a disposition demonstrated, not a rule enforced). An
    overdue re-affirmation is *reported*, never *enforced*. A human observer who
    sees an ``Alert`` may then choose to publish an attributed ``ObserverReport``;
    that step is intentionally not automated."""
    return _erosion_from(
        archive.erosion_alert(
            as_of,
            rite_cadence_seconds,
            attestation_cadence_seconds,
            enforcement_window_seconds,
            enforcement_volume_floor,
            enforcement_coverage_floor_permille,
            enforcement_dispersion_ceiling_permille,
            enforcement_downward_floor_permille,
        )
    )


def _erosion_from(a) -> ErosionAlert:
    return ErosionAlert(
        severity=a.severity,
        reasons=tuple(a.reasons),
        list_contraction=a.list_contraction,
        minimum_contraction=a.minimum_contraction,
        fast_contraction=a.fast_contraction,
        active_fast_deviations=a.active_fast_deviations,
        overdue_reaffirmations=tuple(
            OverdueReaffirmation(
                community_did=did,
                last_rite_cid=cid,
                seconds_since_last=since,
                cadence_seconds=cadence,
            )
            for did, cid, since, cadence in a.overdue_reaffirmations
        ),
        hollowing_communities=tuple(a.hollowing_communities),
        thinning_deliberation=(
            None
            if a.thinning_deliberation is None
            else ThinningDeliberation(*a.thinning_deliberation)
        ),
        overdue_reexaminations=tuple(
            OverdueReexamination(cid, subject, tier, due, overdue, confirmations)
            for cid, subject, tier, due, overdue, confirmations
            in a.overdue_reexaminations
        ),
        silent_functions=tuple(
            SilentFunction(did, fn, since, cadence, was_null)
            for did, fn, since, cadence, was_null in a.silent_functions
        ),
        unstaffed_functions=tuple(
            UnstaffedFunction(did, fn, vacant_since, seconds)
            for did, fn, vacant_since, seconds in a.unstaffed_functions
        ),
        enforcement_convergence=_convergence_from(a.enforcement_convergence),
        enforcement_direction=_direction_from(a.enforcement_direction),
    )


@dataclass(frozen=True)
class ServingMediator:
    mediator_did: str
    nominating_community: str
    # term end (unix seconds); None only for legacy open-ended entries
    term_end: Optional[int]
    # who trained this mediator, if the apprenticeship was recorded at all
    trained_by: Optional[str]


@dataclass(frozen=True)
class MediatorPoolConcentration:
    # (community_did, active_mediators) descending — the domination front
    shares: tuple[tuple[str, int], ...]
    total_active: int
    # mediators currently seated, soonest term-end first (the rotation view)
    serving: tuple[ServingMediator, ...]
    # largest single-community active count
    top_active: int
    # True when one community already holds a majority of the pool
    majority_held: bool
    # (trainer_did, active_mediators_trained) descending. Mediators with no
    # recorded trainer are absent rather than grouped under a blank: an
    # unrecorded apprenticeship and no apprenticeship are not distinguished.
    lineages: tuple[tuple[str, int], ...]
    # active mediators whose lineage is recorded — the denominator the lineage
    # readings are against, so a pool that records nothing reads as unknown
    # rather than healthy
    with_lineage: int
    # largest single-trainer active count
    top_lineage: int
    # True when one trainer accounts for a majority of the mediators whose
    # lineage is recorded. False when nothing is recorded, so silence is never
    # read as a finding.
    lineage_majority_held: bool


def _mediator_pool_from(p) -> MediatorPoolConcentration:
    return MediatorPoolConcentration(
        shares=tuple(tuple(s) for s in p.shares),
        total_active=p.total_active,
        serving=tuple(ServingMediator(s[0], s[1], s[2], s[3]) for s in p.serving),
        top_active=p.top_active,
        majority_held=p.majority_held,
        lineages=tuple(tuple(s) for s in p.lineages),
        with_lineage=p.with_lineage,
        top_lineage=p.top_lineage,
        lineage_majority_held=p.lineage_majority_held,
    )


def mediator_pool_concentration(
    archive, as_of: Optional[int] = None
) -> MediatorPoolConcentration:
    """§4 mediator pool-share / rotation / lineage lens: the active pool's
    composition by nominating community, the mediators currently seated, and who
    trained them. No verdict — the no-majority pool-share cap and the term and
    rotation limits are enforced by ``check_authority``, and ``trained_by`` is
    read by no rule at all. The lineage half is here because §4's apprenticeship
    model carries its own named weakness: mediators trained under experienced
    mediators are susceptible to capture by the incumbent generation's biases,
    and a pool that all traces to one trainer is that weakness realized. Making
    it legible is the whole of what this does; the same documents warn against a
    certification barrier becoming a gatekeeping mechanism."""
    return _mediator_pool_from(archive.mediator_pool_concentration(as_of))


@dataclass(frozen=True)
class ServingObserver:
    observer_did: str
    nominating_community: str
    # term end (unix seconds); None only for legacy open-ended entries
    term_end: Optional[int]


@dataclass(frozen=True)
class ObserverPoolConcentration:
    # (community_did, active_observers) descending — the domination front
    shares: tuple[tuple[str, int], ...]
    total_active: int
    # observers currently serving, soonest term-end first (the rotation view)
    serving: tuple[ServingObserver, ...]
    # largest single-community active count
    top_active: int
    # True when one community already holds a majority of the corps
    majority_held: bool


def _pool_from(p) -> ObserverPoolConcentration:
    return ObserverPoolConcentration(
        shares=tuple(tuple(s) for s in p.shares),
        total_active=p.total_active,
        serving=tuple(ServingObserver(s[0], s[1], s[2]) for s in p.serving),
        top_active=p.top_active,
        majority_held=p.majority_held,
    )


def observer_pool_concentration(
    archive, as_of: Optional[int] = None
) -> ObserverPoolConcentration:
    """§8 observer pool-share / rotation lens: the active corps' composition by
    nominating community and the observers currently serving. No verdict — the
    no-majority pool-share cap and term/rotation limits are enforced by
    ``check_authority``; this board makes the finer drift toward domination
    visible before the hard cap bites."""
    return _pool_from(archive.observer_pool_concentration(as_of))


@dataclass(frozen=True)
class ObserverDashboard:
    composition: CompositionGraph
    rejections: RejectionPattern
    # (subject_did, observer_report_count), descending
    observer_reports_by_subject: tuple[tuple[str, int], ...]
    # (accused_did, whistleblower_count), descending; never the source
    whistleblowers_by_accused: tuple[tuple[str, int], ...]
    # (accused_did, distinct_source_count), descending. How many separate insiders
    # filed, never who: one voice repeating itself does not read as many.
    distinct_sources_by_accused: tuple[tuple[str, int], ...]
    # the observer-corps composition (pool-share / rotation view)
    pool: ObserverPoolConcentration
    # the substrate-deference / erosion report (§9 half of the board)
    substrate: SubstrateDeferenceReport
    # the graded erosion alert derived from `substrate` (the crossed-threshold signal)
    erosion: ErosionAlert
    # (community_did, insider_set_size), ASCENDING — thinnest anonymous channels
    # first. 1 means no channel at all; 2 is a channel in name only.
    anonymity_set_by_community: tuple[tuple[str, int], ...]


def observer_dashboard(archive, as_of: Optional[int] = None) -> ObserverDashboard:
    """§8 observer dashboard (the composition-drift board): the sponsorship graph
    and rejection pattern bundled with per-community observer- and
    whistleblower-report tallies, the observer-corps composition (pool-share /
    rotation), and the substrate-deference erosion report (§9 half). Flags with
    provenance, no verdict."""
    d = archive.observer_dashboard(as_of)
    return ObserverDashboard(
        composition=CompositionGraph(
            edges=tuple(tuple(e) for e in d.composition.edges),
            by_concentration=tuple(tuple(c) for c in d.composition.by_concentration),
        ),
        rejections=RejectionPattern(
            events=tuple(RejectionEvent(e[0], e[1], e[2], e[3], e[4]) for e in d.rejections.events),
            by_rejector=tuple(tuple(x) for x in d.rejections.by_rejector),
            by_presenting=tuple(tuple(x) for x in d.rejections.by_presenting),
            by_feature=tuple(tuple(x) for x in d.rejections.by_feature),
            by_concentration=tuple(tuple(x) for x in d.rejections.by_concentration),
            total_rejections=d.rejections.total_rejections,
            perspectives=tuple(d.rejections.perspectives),
        ),
        observer_reports_by_subject=tuple(tuple(x) for x in d.observer_reports_by_subject),
        whistleblowers_by_accused=tuple(tuple(x) for x in d.whistleblowers_by_accused),
        distinct_sources_by_accused=tuple(
            tuple(x) for x in d.distinct_sources_by_accused
        ),
        pool=_pool_from(d.pool),
        substrate=_substrate_from(d.substrate),
        erosion=_erosion_from(d.erosion),
        anonymity_set_by_community=tuple(
            tuple(x) for x in d.anonymity_set_by_community
        ),
    )


def deliberative_capacity(
    archive,
    compact_label: str,
    quorum_threshold: int = 2,
    as_of: Optional[int] = None,
) -> DeliberativeCapacity:
    """The share of `compact_label`'s roll carrying second-class weight, and
    whether the compact can act at the ordinary quorum or the contraction bar."""
    c = archive.deliberative_capacity(compact_label, quorum_threshold, as_of)
    return DeliberativeCapacity(
        compact_label=c.compact_label,
        members=tuple(c.members),
        probationary=tuple(c.probationary),
        standing=c.standing,
        standing_share=c.standing_share,
        quorum_threshold=c.quorum_threshold,
        can_act=c.can_act,
        can_contract=c.can_contract,
        as_of=c.as_of,
    )
