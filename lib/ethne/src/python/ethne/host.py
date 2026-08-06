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

"""What a host running a polity has to do on a cadence, and when (design **D20**).

**`tick` decides and computes; the host signs and does I/O.** Nothing in this module
writes a store, signs a record or reads a clock. `as_of` arrives like every other
cadence in this tier, so the same cycle drives a simulated timeline in a test and a
real one in production, and a year of host behaviour is testable in milliseconds.

    from ethne.host import HostConfig, HostCycle

    cycle = HostCycle(HostConfig(polity_did))
    for action in cycle.tick(archive, candidate, as_of=now):
        if action.kind == "Save":
            store.save(archive)
        elif action.kind == "RaiseConcern":
            record = coherence_concern(action.report, polity_did, observer)
        elif action.kind == "Prune":
            prune_observations(archive, polity_did, deciders,
                               horizon=action.recommendation.horizon)

Raising a Concern and pruning both need keys, and the keys are the host's, so both
come back as recommendations over a computed report. That is the same posture
`Archive.prune_recommendation` already takes, and the same principle the tier is built
on: emergence proposes, the polity ratifies. Housekeeping and alarm-raising are no
exception.

`HostCycle.grade` is worth reaching for on its own. It supplies **every** read-model
the archive can offer, which is the part a caller would otherwise have to know to do:
`cohesion_report` takes seven optional readings, and an omitted one makes its component
report *nothing* rather than complain. Since **D31** a report says which readings it was
given (`omitted_readings`, `silent_measures`, `blind_to_concern`), so a hand-assembled
grade is at least legible about what it could not see. Going through the cycle is how
you avoid needing that.

Until D31 this module did not exist: `en-host` was Rust-side only, so a Python host got
the store and the read-models and had to reimplement the cadence bookkeeping and the
assembly. What that cost was measured rather than assumed. A healthy polity whose
chartered ground had withdrawn -- a `Concern` on its own, with no threshold -- graded
`"None"` from Python, because the grade was assembled by hand and the ground reading was
one of the six left out.

The cadenced duties, and only those: persist, grade cohesion, notice that the grade has
reached `Concern`, and notice that the observation log is due for pruning. Membership,
induction, office conferral and envoy authorization are **deliberate acts of the
polity**, not scheduled ones, and nothing here initiates them.
"""
from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Optional, Sequence

from ethne import _core
from ethne.monitor import CohesionReport, FoundingCandidate, Thresholds
from ethne.polity import Archive, PruneRecommendation, RetentionPolicy

DueAction = _core.DueAction


@dataclass(frozen=True)
class HostConfig:
    """How often a host attends to each duty, and on what terms.

    The cadences are **policy**, not charter: a polity's charter does not say how often
    its host wakes up, so nothing here is a ratified term and nothing here is signed.

    `thresholds` and `retention` take the same `Thresholds` and `RetentionPolicy`
    dataclasses `cohesion_report` and `Archive.prune_recommendation` take, because a
    host configured one way and a hand call configured another would be two answers to
    one question. Omitted, each carries the Rust tier's own defaults.

    `cohesion_cadence_seconds` defaults to the attendance cadence, the fastest-moving
    input the grade has, and `observation_window_seconds` to the retention span:
    everything the polity has chosen to keep and no more, since beyond that there is
    nothing to read anyway. Both follow whatever thresholds and retention you pass, so
    tightening retention tightens the window with it.

    `retention_cadence_seconds` is daily, because housekeeping is not urgent and
    `min_dropped` already guards churn. `save_cadence_seconds` is zero, i.e. every tick:
    the log is evidence, and the store writes nothing when nothing has changed.
    """

    polity: str
    thresholds: Optional[Thresholds] = None
    retention: Optional[RetentionPolicy] = None
    opaque_boundary: bool = False
    cohesion_cadence_seconds: Optional[int] = None
    retention_cadence_seconds: Optional[int] = None
    save_cadence_seconds: Optional[int] = None
    observation_window_seconds: Optional[int] = None

    def _native(self) -> "_core.HostConfig":
        kwargs = {}
        if self.thresholds is not None:
            kwargs.update(asdict(self.thresholds))
        if self.retention is not None:
            kwargs.update(asdict(self.retention))
        return _core.HostConfig(
            self.polity,
            opaque_boundary=self.opaque_boundary,
            cohesion_cadence_seconds=self.cohesion_cadence_seconds,
            retention_cadence_seconds=self.retention_cadence_seconds,
            save_cadence_seconds=self.save_cadence_seconds,
            observation_window_seconds=self.observation_window_seconds,
            **kwargs,
        )


class HostCycle:
    """A host's cadence state: what it last attended to, and when.

    Holds no clock and no archive. Both arrive at `tick`, so the same cycle can be
    driven over a simulated timeline in a test and a real one in production, with
    identical results.
    """

    def __init__(self, config: HostConfig):
        self._config = config
        self._inner = _core.HostCycle(config._native())

    @property
    def config(self) -> HostConfig:
        """The configuration this cycle was built with."""
        return self._config

    @property
    def resolved(self) -> "_core.HostConfig":
        """What that configuration resolved to, defaults filled in: the cadences, and
        the `thresholds` / `retention` dicts the grade will actually use."""
        return self._inner.config

    def tick(
        self,
        archive: Archive,
        candidate: FoundingCandidate,
        as_of: Optional[int] = None,
    ) -> Sequence[DueAction]:
        """What is due at `as_of` (epoch seconds, default now).

        Everything is due on the first tick. A duty is marked attended when it is
        *computed*, not when the host acts on the result, so ignoring a returned action
        does not make the host spin on it: the next cadence recomputes. A polity that is
        not yet chartered, or already dissolved, gets `Save` and nothing else, since the
        log is what remains of it.

        A clock that goes backwards reports nothing due until it catches up. That is the
        conservative direction: a host with a bad clock under-acts rather than grading
        the same moment over and over.
        """
        return self._inner.tick(archive, candidate, as_of=as_of)

    def grade(
        self,
        archive: Archive,
        candidate: FoundingCandidate,
        as_of: Optional[int] = None,
    ) -> CohesionReport:
        """Grade cohesion with **every** read-model the archive can supply.

        The safe form of `cohesion_report`. Assembling the seven readings is the part a
        host would otherwise have to know to do, and omitting one is silent in the
        direction of health: the returned report's `omitted_readings` is empty, which
        `en-host`'s own test asserts and this is the only Python call that guarantees.
        """
        return self._inner.grade(archive, candidate, as_of=as_of)

    def __repr__(self) -> str:
        return f"HostCycle(polity={self._config.polity})"


__all__ = [
    "DueAction",
    "HostConfig",
    "HostCycle",
    "PruneRecommendation",
]
