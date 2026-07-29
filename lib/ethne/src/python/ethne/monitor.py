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

"""The substrate side: what a provider observes, and how far it has drifted.

A `FoundingCandidate` is the emergent boundary a founding ratifies — the roll a
substrate currently shows, each member with an earned reputation and an optional
guardian edge. `manual_founding` is the convener-driven provider that needs no
substrate service at all; `founding_candidate` builds one from observations
directly (an `autonomous-trust` gateway subtree reduces to exactly this shape).

`cohesion_report` then grades the strain between what a polity chartered and what
the substrate now shows. It never decides a split: a `Concern` is an early
warning, and a split is always a deliberate re-charter.

    from ethne.monitor import manual_founding, cohesion_report, propose_from_candidate

    candidate = manual_founding([a.did, b.did, c.did], "tradition", "answers to X")
    proposal = propose_from_candidate(candidate, polity.did, min_reputation=0.5)
    # ... later, only two of the three are still observed:
    report = cohesion_report(chartered_roll, thinned_candidate)
    report.erosion            # "None" | "Notice" | "Concern"
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Optional, Sequence

from ethne import _core

# Named here only so `cohesion_report`'s signature can annotate it; the canonical
# export of the transmission read-model is `ethne.polity`, which derives it.
AffirmationState = _core.AffirmationState
GuardianState = _core.GuardianState
DecayTrends = _core.DecayTrends
ObservationWindow = _core.ObservationWindow
GroundStanding = _core.GroundStanding
CohesionReport = _core.CohesionReport
FoundingCandidate = _core.FoundingCandidate

founding_candidate = _core.founding_candidate
manual_founding = _core.manual_founding
propose_from_candidate = _core.propose_from_candidate
select_from_candidate = _core.select_from_candidate
sign_reading = _core.sign_reading


#: `en-cohesion`'s own defaults, read from the Rust core so there is only ever one
#: copy of them.
DEFAULT_THRESHOLDS = _core.DEFAULT_THRESHOLDS


@dataclass(frozen=True)
class Thresholds:
    """Where each cohesion signal tips into `Notice` and into `Concern`.

    Field defaults come from `DEFAULT_THRESHOLDS`, i.e. from `en-cohesion` itself.
    `reputation_floor` is the per-member floor a breach counts against; the
    attrition / mean-reputation / breach pairs are the two grading steps of each
    component. Pass an instance to `cohesion_report` to override — the cadence of
    judgement stays a query parameter, not a charter term.

    Attendance is the exception to the pair convention: `attendance_cadence_seconds`
    says how long a guardian's verified presence still counts as present, and
    `unattended_notice` is the share of guardians away that raises a Notice. There
    is no `unattended_concern` — absent humans never escalate, because a snapshot
    cannot tell a night shift from an abandoned polity.

    `unrated_notice` is capped the same way and for the same kind of reason: it is
    the share of observed members the provider has no reputation reading for, and a
    young cohort nobody has had occasion to rate looks exactly like a provider that
    has stopped rating. There is no `unrated_concern`. Its job is to stop the two
    reputation components going *silently* dark — they report nothing when nobody is
    rated, and something has to say that they are not reporting.

    Transmission decay does get a full pair: `affirmation_cadence_seconds` is how
    long a member's affirmation of the substrate stays current, and
    `affirmation_stale_notice` / `affirmation_stale_concern` are its two grading
    steps. It escalates where attendance does not, because duration is already
    inside the measure — a member a whole cadence past re-affirming is not at lunch.

    `guardian_lost_notice` / `guardian_lost_concern` grade the chartered
    node→guardian edges the substrate no longer shows. This one escalates too, and
    for a different reason again: it compares what the polity *ratified* against
    what is observed, over members the provider still sees — so a vanished member is
    attrition, never guardian loss, and no cap is needed.

    The **trends** are shapes over time rather than states: `churn_window_seconds`
    with `churn_notice` / `churn_concern` grade how fast the roll is being redrawn,
    while a widening affirmation gap and a thinning witness turnout warn but never
    escalate — a trend is not yet a lapse, and the lapse it points at is already
    escalatable on its own.

    `sustained_absence_seconds` / `sustained_absence_readings` are the exception that
    proves the attendance cap: the *same* absence, given a duration and a witness,
    does reach `Concern`. The readings figure is not "N absences" — the span is the
    absence, and the count is evidence that somebody was watching while it lasted.
    """

    reputation_floor: float = DEFAULT_THRESHOLDS["reputation_floor"]
    attrition_notice: float = DEFAULT_THRESHOLDS["attrition_notice"]
    attrition_concern: float = DEFAULT_THRESHOLDS["attrition_concern"]
    mean_reputation_notice: float = DEFAULT_THRESHOLDS["mean_reputation_notice"]
    mean_reputation_concern: float = DEFAULT_THRESHOLDS["mean_reputation_concern"]
    breach_notice: float = DEFAULT_THRESHOLDS["breach_notice"]
    breach_concern: float = DEFAULT_THRESHOLDS["breach_concern"]
    attendance_cadence_seconds: int = DEFAULT_THRESHOLDS["attendance_cadence_seconds"]
    unattended_notice: float = DEFAULT_THRESHOLDS["unattended_notice"]
    unrated_notice: float = DEFAULT_THRESHOLDS["unrated_notice"]
    affirmation_cadence_seconds: int = DEFAULT_THRESHOLDS["affirmation_cadence_seconds"]
    affirmation_stale_notice: float = DEFAULT_THRESHOLDS["affirmation_stale_notice"]
    affirmation_stale_concern: float = DEFAULT_THRESHOLDS["affirmation_stale_concern"]
    guardian_lost_notice: float = DEFAULT_THRESHOLDS["guardian_lost_notice"]
    guardian_lost_concern: float = DEFAULT_THRESHOLDS["guardian_lost_concern"]
    churn_window_seconds: int = DEFAULT_THRESHOLDS["churn_window_seconds"]
    churn_notice: float = DEFAULT_THRESHOLDS["churn_notice"]
    churn_concern: float = DEFAULT_THRESHOLDS["churn_concern"]
    sustained_absence_seconds: int = DEFAULT_THRESHOLDS["sustained_absence_seconds"]
    sustained_absence_readings: int = DEFAULT_THRESHOLDS["sustained_absence_readings"]


def cohesion_report(
    chartered_roll: Sequence[str],
    candidate: FoundingCandidate,
    opaque_boundary: bool = False,
    thresholds: Optional[Thresholds] = None,
    guardians: Optional[GuardianState] = None,
    affirmations: Optional[AffirmationState] = None,
    trends: Optional[DecayTrends] = None,
    window: Optional[ObservationWindow] = None,
    ground: Optional[GroundStanding] = None,
    as_of: Optional[int] = None,
) -> CohesionReport:
    """Grade a polity's coherence erosion: the chartered roll against what the
    substrate currently shows. Under an opaque boundary the attrition component is
    capped at `Notice`, because a disappearance behind an opaque edge cannot be
    attributed to a departure. Absent guardians are capped at `Notice` too, and
    always.

    Pass `affirmations` — `archive.affirmation_state(polity, as_of)` — to include
    transmission decay: members who have stopped re-affirming the substrate. That
    component *can* reach `Concern`, unlike absent guardians. Omitted, it reports
    nothing rather than implying everyone is current.

    Pass `guardians` — `archive.guardian_state(polity, as_of)` — to include lost
    guardian edges: members the charter says answer to a human, whom the substrate
    still shows but without one. Omitted, that component says nothing too.

    Pass `trends` — `archive.decay_trends(polity, as_of)` — for the decay shapes, and
    `window` — `archive.observation_window(polity, since, as_of)` — for what the
    polity's monitors actually recorded seeing. The window is what makes a *sustained*
    absence gradeable at all; without it an absent guardian can never be more than the
    capped snapshot reading.

    Pass `ground` — `archive.substrate_disavowal(polity, as_of)` — to include whether
    the polity's chartered substrate anchor has withdrawn (D23). That component is the
    one here that grades a single act rather than a quantity, and it reaches `Concern`
    alone: there is nothing gradual about a ground saying it does not stand above
    these people, so there is no threshold for it either. Omitted, it says nothing.

    `as_of` (epoch seconds, default now) is the moment every freshness measure is
    judged against."""
    kwargs = asdict(thresholds) if thresholds is not None else {}
    return _core.cohesion_report(
        list(chartered_roll),
        candidate,
        opaque_boundary,
        guardians=guardians,
        affirmations=affirmations,
        trends=trends,
        window=window,
        ground=ground,
        as_of=as_of,
        **kwargs,
    )


__all__ = [
    "DEFAULT_THRESHOLDS",
    "CohesionReport",
    "GroundStanding",
    "FoundingCandidate",
    "Thresholds",
    "cohesion_report",
    "founding_candidate",
    "manual_founding",
    "propose_from_candidate",
    "select_from_candidate",
    "sign_reading",
]
