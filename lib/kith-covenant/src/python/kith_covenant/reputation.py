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
    consumed: int
    contributed: int
    record_count: int
    evidence: tuple[str, ...]


@dataclass(frozen=True)
class FreeRiderReport:
    tallies: tuple[UsageTally, ...]
    total_consumed: int
    total_contributed: int
    # (community_did, net_draw), descending net draw, for net-positive drawers.
    by_net_draw: tuple[tuple[str, int], ...]


def free_rider_signals(
    archive, commons_cid: str, as_of: Optional[int] = None
) -> FreeRiderReport:
    """§7 free-rider lens over a commons's usage records (Ostrom monitoring /
    tragedy-of-the-virtues): per-appropriator consumed/contributed tallies plus
    the net-draw ranking. Surfaces the asymmetry between what communities drew and
    returned; renders no verdict on whether it is free-riding."""
    r = archive.free_rider_signals(commons_cid, as_of)
    return FreeRiderReport(
        tallies=tuple(
            UsageTally(
                community=t[0],
                consumed=t[1],
                contributed=t[2],
                record_count=t[3],
                evidence=tuple(t[4]),
            )
            for t in r.tallies
        ),
        total_consumed=r.total_consumed,
        total_contributed=r.total_contributed,
        by_net_draw=tuple(tuple(x) for x in r.by_net_draw),
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
class ConstitutionState:
    ratified: bool
    compact_incapable: tuple[IncapableMatter, ...]
    fast_periods: tuple[FastPeriod, ...]


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
    )


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
class OverdueReexamination:
    designation_cid: str
    # the designated community, whose exclusion is being left unexamined
    subject: str
    # "Anathema"; HostisHumaniGeneris is forbidden a re-examination date
    tier: str
    reexamine_after: int
    seconds_overdue: int


def overdue_reexaminations(
    archive, as_of: Optional[int] = None
) -> tuple[OverdueReexamination, ...]:
    """§5 designation follow-through: designations whose mandated
    ``reexamine_after`` has passed and which are still standing, longest-overdue
    first. "Still standing" means not reversed — no ``DesignationLifted`` conduct
    record naming it and no re-entry answering it.

    Known limit: a re-examination that *confirms* a designation leaves no record,
    so this cannot distinguish "nobody looked" from "the panel looked and let it
    stand". It reports the weaker claim, which is the honest one."""
    return tuple(
        OverdueReexamination(cid, subject, tier, due, overdue)
        for cid, subject, tier, due, overdue in archive.overdue_reexaminations(as_of)
    )


@dataclass(frozen=True)
class ErosionAlert:
    # "None" | "Notice" | "Alert" — escalates as erosion signals compound.
    severity: str
    # one human-readable reason per active erosion signal
    reasons: tuple[str, ...]
    list_contraction: bool
    fast_contraction: bool
    active_fast_deviations: int
    # communities overdue for re-affirmation, longest-overdue first
    overdue_reaffirmations: tuple[OverdueReaffirmation, ...]
    # communities drawing fewer witnesses than at founding
    hollowing_communities: tuple[str, ...]
    # designations past a mandated re-examination and still standing
    overdue_reexaminations: tuple[OverdueReexamination, ...]


def erosion_alert(
    archive,
    as_of: Optional[int] = None,
    rite_cadence_seconds: Optional[int] = None,
) -> ErosionAlert:
    """§9 erosion-detection alerting: a graded, derived reading that escalates
    (None→Notice→Alert) as the erosion signals compound.

    Five signals across five dimensions. Three fire on an *act* — a list
    contraction, a fast contraction, deliberative acts during a fast. Two fire on
    *omission and a pinned baseline*: a re-affirmation overdue against an absolute
    generational cadence, and a ceremony drawing fewer witnesses than it did at
    founding. The asymmetry is deliberate; an alert built only from act-triggered
    signals reads a polity that has stopped functioning as a polity in perfect
    health (see ``doc/architecture/erosion-legibility.md``).

    ``rite_cadence_seconds`` overrides the cadence (default 30 years). The
    calibration is an open question in ``protocol-components.md``, so it is a
    parameter rather than a constant.

    Diagnostic only: surfaced to the observer corps, never a verdict and never a
    refusal (non-ultimacy is a disposition demonstrated, not a rule enforced). An
    overdue re-affirmation is *reported*, never *enforced*. A human observer who
    sees an ``Alert`` may then choose to publish an attributed ``ObserverReport``;
    that step is intentionally not automated."""
    return _erosion_from(archive.erosion_alert(as_of, rite_cadence_seconds))


def _erosion_from(a) -> ErosionAlert:
    return ErosionAlert(
        severity=a.severity,
        reasons=tuple(a.reasons),
        list_contraction=a.list_contraction,
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
        overdue_reexaminations=tuple(
            OverdueReexamination(cid, subject, tier, due, overdue)
            for cid, subject, tier, due, overdue in a.overdue_reexaminations
        ),
    )


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
    )
