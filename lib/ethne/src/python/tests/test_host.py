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

"""The host cycle from Python (design **D20**, bound by **D31**).

`en-host` was Rust-side only until D31, so a Python host reimplemented the cadence
bookkeeping and the read-model assembly. The assembly is the half that fails silently:
`cohesion_report` takes seven optional readings and an omitted one makes its component
report *nothing*, which a reader cannot tell from health. What that cost was measured
before this was built, and the measurement is the last test in this file.
"""
from __future__ import annotations

import pytest

import ethne as e
from ethne.authoring import (
    Signer,
    disavow_substrate,
    dissolve_charter,
    ratify_charter,
    record_observation,
)
from ethne.host import HostConfig, HostCycle
from ethne.monitor import Thresholds, cohesion_report, founding_candidate
from ethne.polity import RetentionPolicy

DAY = 86_400
FOUNDED_AT = 100


def kinds(actions) -> list[str]:
    return [a.kind for a in actions]


@pytest.fixture
def cycle(polity) -> HostCycle:
    return HostCycle(HostConfig(polity.did))


def healthy(polity, as_of: int = FOUNDED_AT + DAY):
    """Everyone present and well regarded: a healthy substrate reading."""
    return founding_candidate(
        [(d, 0.9, None, None, False) for d in polity.roll(as_of)]
    )


def observed(polity, times, monitors=None):
    """File one observation per instant, rotating over three monitors, because the
    retention floor counts distinct watchers rather than filings (D28)."""
    monitors = monitors or [Signer.generate() for _ in range(3)]
    roll = polity.roll(FOUNDED_AT + DAY)
    for i, at in enumerate(times):
        record_observation(
            polity.archive,
            polity.did,
            monitors[i % len(monitors)],
            [(d, 0.9, None, at) for d in roll],
            observed_at=at,
        )
    return monitors


# ---- The cadence ------------------------------------------------------------


def test_everything_is_due_on_the_first_tick(polity, cycle):
    due = cycle.tick(polity.archive, healthy(polity), as_of=FOUNDED_AT + DAY)
    assert kinds(due) == ["Save", "Cohesion"], "no observations yet, nothing to prune"


def test_a_duty_waits_for_its_cadence_and_then_comes_due(polity):
    cycle = HostCycle(HostConfig(polity.did, cohesion_cadence_seconds=DAY))
    first = FOUNDED_AT + DAY
    assert "Cohesion" in kinds(cycle.tick(polity.archive, healthy(polity), as_of=first))
    soon = cycle.tick(polity.archive, healthy(polity), as_of=first + 60)
    assert "Cohesion" not in kinds(soon)
    later = cycle.tick(polity.archive, healthy(polity), as_of=first + DAY)
    assert "Cohesion" in kinds(later)


def test_a_zero_cadence_means_every_tick(polity, cycle):
    assert cycle.resolved.save_cadence_seconds == 0
    for at in (FOUNDED_AT + DAY, FOUNDED_AT + DAY + 1, FOUNDED_AT + DAY + 2):
        assert "Save" in kinds(cycle.tick(polity.archive, healthy(polity), as_of=at))


def test_a_clock_that_goes_backwards_under_acts_rather_than_regrading(polity, cycle):
    at = FOUNDED_AT + 10 * DAY
    assert "Cohesion" in kinds(cycle.tick(polity.archive, healthy(polity), as_of=at))
    back = cycle.tick(polity.archive, healthy(polity), as_of=at - DAY)
    assert "Cohesion" not in kinds(back), "the conservative direction"


def test_a_polity_that_has_ended_is_only_persisted(polity, cycle):
    """Nothing but `Save` for a dissolved polity: the log is what remains of it."""
    as_of = FOUNDED_AT + 5 * DAY
    dissolve_charter(
        polity.archive, polity.did, polity.quorum(as_of), rationale="done",
        dissolved_at=as_of,
    )
    assert kinds(cycle.tick(polity.archive, healthy(polity), as_of=as_of + DAY)) == ["Save"]


# ---- What comes back --------------------------------------------------------


def test_a_healthy_polity_grades_without_raising_a_concern(polity, cycle):
    due = cycle.tick(polity.archive, healthy(polity), as_of=FOUNDED_AT + DAY)
    (report,) = [a.report for a in due if a.kind == "Cohesion"]
    assert report.erosion != "Concern"
    assert report.omitted_readings == [], "the cycle supplies every reading"


def test_a_withdrawn_ground_comes_back_as_raise_concern(found):
    """A ground that withdraws is a `Concern` alone, with no threshold (D23), and it is
    the level that crosses to kith-covenant. `tick` says it is warranted and signs
    nothing: the record needs the observer's key, which is the host's."""
    ground = Signer.generate()
    founders = [Signer.generate() for _ in range(3)]
    priv, pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "the operators",
        "answers to what it did not author",
        member_dids=[s.did for s in founders],
        substrate_anchor=ground.did,
        proposed_at=50,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, founders, founded_at=FOUNDED_AT, anchor=ground)
    roll = archive.charter_state(did, as_of=FOUNDED_AT + DAY).members
    candidate = founding_candidate([(d, 0.9, None, None, False) for d in roll])

    cycle = HostCycle(HostConfig(did))
    clean = cycle.tick(archive, candidate, as_of=FOUNDED_AT + DAY)
    assert kinds(clean) == ["Save", "Cohesion"]

    disavow_substrate(
        archive, did, ground, rationale="I do not stand above these people",
        disavowed_at=FOUNDED_AT + 2 * DAY,
    )
    after = cycle.tick(archive, candidate, as_of=FOUNDED_AT + 3 * DAY)
    assert "RaiseConcern" in kinds(after)
    (report,) = [a.report for a in after if a.kind == "RaiseConcern"]
    assert report.erosion == "Concern"
    assert report.ground_disavowed
    # The host signs it. `tick` returned a report, not a record.
    record = e.coherence_concern(report, did, Signer.generate(), at=FOUNDED_AT + 3 * DAY)
    assert record is not None


def test_pruning_comes_due_only_once_there_is_something_worth_dropping(polity):
    cycle = HostCycle(HostConfig(polity.did))
    young = FOUNDED_AT + 2 * DAY
    observed(polity, [FOUNDED_AT + DAY])
    assert "Prune" not in kinds(cycle.tick(polity.archive, healthy(polity), as_of=young))

    # 150 filings a day apart, rotating three watchers, is past both the `min_dropped`
    # floor and the retention span.
    observed(polity, [FOUNDED_AT + (2 + i) * DAY for i in range(150)])
    at = FOUNDED_AT + 200 * DAY
    due = cycle.tick(polity.archive, healthy(polity), as_of=at)
    assert "Prune" in kinds(due)
    (rec,) = [a.recommendation for a in due if a.kind == "Prune"]
    assert rec.dropped >= cycle.resolved.retention["min_dropped"]
    assert rec.horizon < at


def test_a_lone_monitor_pins_the_horizon(polity):
    """The retention floor counts distinct **watchers** per member (D28), so one monitor
    filing every day never clears it. This is what left both reference hosts with a dead
    compaction path until it was measured: one monitor recommends nothing where three
    recommend a prune over the same log."""
    one = [Signer.generate()]
    observed(polity, [FOUNDED_AT + (1 + i) * DAY for i in range(150)], monitors=one)
    at = FOUNDED_AT + 200 * DAY
    cycle = HostCycle(HostConfig(polity.did))
    assert "Prune" not in kinds(cycle.tick(polity.archive, healthy(polity), as_of=at))


def test_a_due_action_carries_exactly_one_payload(polity, cycle):
    observed(polity, [FOUNDED_AT + (1 + i) * DAY for i in range(150)])
    # 49 days past the last filing, so the absence is sustained and witnessed: this is a
    # `RaiseConcern` rather than a `Cohesion`, and exactly one of the two is ever due.
    due = cycle.tick(polity.archive, healthy(polity), as_of=FOUNDED_AT + 200 * DAY)
    assert kinds(due) == ["Save", "RaiseConcern", "Prune"]
    graded = [a for a in due if a.kind in ("Cohesion", "RaiseConcern")]
    assert len(graded) == 1
    for action in due:
        carried = [x for x in (action.report, action.recommendation) if x is not None]
        assert len(carried) == (0 if action.kind == "Save" else 1), action.kind


# ---- Configuration ----------------------------------------------------------


def test_thresholds_and_retention_reach_the_grade(polity):
    """The overrides are not decoration: a config that carried them and did not apply
    them would be the same class of silent failure this slice is about."""
    config = HostConfig(
        polity.did,
        thresholds=Thresholds(attrition_concern=0.01, attendance_cadence_seconds=999),
        retention=RetentionPolicy(retain_seconds=7 * DAY, min_dropped=5),
    )
    cycle = HostCycle(config)
    assert cycle.resolved.thresholds["attrition_concern"] == 0.01
    assert cycle.resolved.retention["retain_seconds"] == 7 * DAY
    assert cycle.resolved.retention["min_dropped"] == 5
    # The two derived cadences follow what was passed rather than the defaults.
    assert cycle.resolved.cohesion_cadence_seconds == 999
    assert cycle.resolved.observation_window_seconds == 7 * DAY
    # And the threshold actually grades: one member of three gone is 33% attrition,
    # which is under the default 40% and over this config's 1%.
    as_of = FOUNDED_AT + DAY
    thinned = founding_candidate(
        [(d, 0.9, None, None, False) for d in polity.roll(as_of)[:-1]]
    )
    assert cycle.grade(polity.archive, thinned, as_of=as_of).erosion == "Concern"
    default = HostCycle(HostConfig(polity.did))
    assert default.grade(polity.archive, thinned, as_of=as_of).erosion == "Notice"


def test_the_cycle_keeps_its_own_config(polity):
    config = HostConfig(polity.did, save_cadence_seconds=60)
    cycle = HostCycle(config)
    assert cycle.config is config
    assert cycle.resolved.save_cadence_seconds == 60
    assert polity.did in repr(cycle)


# ---- D31: what the grade was not given --------------------------------------


def test_grade_omits_nothing_where_a_hand_assembled_call_cannot_say(polity, cycle):
    as_of = FOUNDED_AT + DAY
    through = cycle.grade(polity.archive, healthy(polity), as_of=as_of)
    assert through.omitted_readings == []
    assert through.silent_measures == []
    assert not through.blind_to_concern

    by_hand = cohesion_report(polity.roll(as_of), healthy(polity), as_of=as_of)
    assert len(by_hand.omitted_readings) == 7
    assert by_hand.blind_to_concern
    assert "ground_disavowed" in by_hand.silent_measures


def test_only_a_concern_capable_reading_blinds_the_grade(polity):
    """`rites` is capped at `Notice` and the watcher lens is read by no level, so
    neither absence hides an escalation. The split is a property of the levels."""
    as_of = FOUNDED_AT + DAY
    roll = polity.roll(as_of)
    supplied = dict(
        guardians=polity.archive.guardian_state(polity.did, as_of=as_of),
        affirmations=polity.archive.affirmation_state(polity.did, as_of=as_of),
        trends=polity.archive.decay_trends(polity.did, as_of=as_of),
        window=polity.archive.observation_window(polity.did, since=0, as_of=as_of),
        ground=polity.archive.substrate_disavowal(polity.did, as_of=as_of),
        observers=polity.archive.independent_observation(polity.did, as_of=as_of),
        rites=polity.archive.rite_state(polity.did, as_of=as_of),
    )
    for name, blinds in (
        ("rites", False),
        ("observers", False),
        ("guardians", True),
        ("affirmations", True),
        ("trends", True),
        ("window", True),
        ("ground", True),
    ):
        inputs = dict(supplied, **{name: None})
        report = cohesion_report(roll, healthy(polity), as_of=as_of, **inputs)
        assert report.omitted_readings == [name]
        assert report.silent_measures, name
        assert report.blind_to_concern is blinds, name


def test_the_measured_hole_a_withdrawn_ground_a_hand_grade_cannot_see(found):
    """**The measurement, as a regression.** A healthy polity whose chartered ground has
    withdrawn is a `Concern` with no threshold. Graded through the cycle it is one;
    graded the way ethne's own Python demo graded until D31, one of seven readings
    supplied, it is `"None"` and nothing in the report distinguished that from health.

    Legibility, not prevention: the Concern is still lost. What is bought is that the
    second report can no longer be read as a clean bill.
    """
    ground = Signer.generate()
    founders = [Signer.generate() for _ in range(3)]
    priv, pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "the operators",
        "answers to what it did not author",
        member_dids=[s.did for s in founders],
        substrate_anchor=ground.did,
        proposed_at=50,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, founders, founded_at=FOUNDED_AT, anchor=ground)
    as_of = FOUNDED_AT + 3 * DAY
    roll = archive.charter_state(did, as_of=as_of).members
    candidate = founding_candidate([(d, 0.9, None, None, False) for d in roll])
    disavow_substrate(
        archive, did, ground, rationale="I do not stand above these people",
        disavowed_at=FOUNDED_AT + 2 * DAY,
    )

    through = HostCycle(HostConfig(did)).grade(archive, candidate, as_of=as_of)
    assert through.erosion == "Concern"
    assert through.ground_disavowed

    as_the_demo_did = cohesion_report(
        roll,
        candidate,
        affirmations=archive.affirmation_state(did, as_of=as_of),
        as_of=as_of,
    )
    assert as_the_demo_did.erosion == "None", "the hole, unchanged"
    assert not as_the_demo_did.ground_disavowed
    assert "ground" in as_the_demo_did.omitted_readings
    assert "ground_disavowed" in as_the_demo_did.silent_measures
    assert as_the_demo_did.blind_to_concern, "and now it says so"
