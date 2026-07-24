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

CohesionReport = _core.CohesionReport
FoundingCandidate = _core.FoundingCandidate

founding_candidate = _core.founding_candidate
manual_founding = _core.manual_founding
propose_from_candidate = _core.propose_from_candidate


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
    """

    reputation_floor: float = DEFAULT_THRESHOLDS["reputation_floor"]
    attrition_notice: float = DEFAULT_THRESHOLDS["attrition_notice"]
    attrition_concern: float = DEFAULT_THRESHOLDS["attrition_concern"]
    mean_reputation_notice: float = DEFAULT_THRESHOLDS["mean_reputation_notice"]
    mean_reputation_concern: float = DEFAULT_THRESHOLDS["mean_reputation_concern"]
    breach_notice: float = DEFAULT_THRESHOLDS["breach_notice"]
    breach_concern: float = DEFAULT_THRESHOLDS["breach_concern"]


def cohesion_report(
    chartered_roll: Sequence[str],
    candidate: FoundingCandidate,
    opaque_boundary: bool = False,
    thresholds: Optional[Thresholds] = None,
) -> CohesionReport:
    """Grade a polity's coherence erosion: the chartered roll against what the
    substrate currently shows. Under an opaque boundary the attrition component is
    capped at `Notice`, because a disappearance behind an opaque edge cannot be
    attributed to a departure."""
    kwargs = asdict(thresholds) if thresholds is not None else {}
    return _core.cohesion_report(
        list(chartered_roll), candidate, opaque_boundary, **kwargs
    )


__all__ = [
    "DEFAULT_THRESHOLDS",
    "CohesionReport",
    "FoundingCandidate",
    "Thresholds",
    "cohesion_report",
    "founding_candidate",
    "manual_founding",
    "propose_from_candidate",
]
