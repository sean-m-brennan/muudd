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

"""§10 capture-resistance harness: the supported scenarios detect within their
stated falsification bounds; the unsupported ones are surfaced (not omitted)."""
from kith_covenant import run_harness
from kith_covenant.harness import (
    MAX_DAMAGE_FORGED_QUORUM, ADMISSIONS_CONCENTRATION_THRESHOLD,
    INTERPRETATION_MONOPOLY_THRESHOLD, MAX_DAMAGE_MANUFACTURED_EVENT,
    MAX_DAMAGE_PRINCIPAL, MAX_DAMAGE_QUIET, MIN_SEVERITY_QUIET,
)


def test_all_scenarios_present_all_supported():
    """Seven capture moves plus scenario 8, the quiet polity — which is not a
    capture move (it has no adversary) but is scored the same way."""
    report = run_harness()
    assert report.total == 8
    assert report.supported == 8
    moves = sorted(r.move for r in report.results)
    assert moves == [1, 2, 3, 4, 5, 6, 7, 8]


def test_principal_capture_flagged_at_divergence():
    r = next(r for r in run_harness().results if r.move == 6)
    assert r.supported and r.detected
    # The self-benefiting divergence is flagged the step it lands.
    assert r.damage <= MAX_DAMAGE_PRINCIPAL


def test_interpretation_monopoly_flagged_at_threshold():
    r = next(r for r in run_harness().results if r.move == 1)
    assert r.supported and r.detected
    # Flagged as soon as the lone interpreter reaches the concentration threshold.
    assert r.damage <= INTERPRETATION_MONOPOLY_THRESHOLD


def test_manufactured_event_flagged_before_action():
    r = next(r for r in run_harness().results if r.move == 4)
    assert r.supported and r.detected
    # Flagged at intake, before it can drive action.
    assert r.damage == MAX_DAMAGE_MANUFACTURED_EVENT == 0


def test_supported_scenarios_all_pass_within_bounds():
    report = run_harness()
    assert report.all_supported_passed
    for r in report.results:
        if not r.supported:
            continue
        assert r.detected, f"move {r.move} not detected"
        assert r.detection_latency is not None
        assert r.detection_latency <= r.threshold_latency
        assert r.damage <= r.threshold_damage


def test_forged_quorum_never_validates():
    r = next(r for r in run_harness().results if r.move == 5)
    assert r.supported and r.detected
    assert r.damage == MAX_DAMAGE_FORGED_QUORUM == 0


def test_admissions_capture_flagged_at_threshold():
    r = next(r for r in run_harness().results if r.move == 7)
    assert r.supported and r.detected
    # Flagged as soon as the sponsor reaches the concentration threshold.
    assert r.damage <= ADMISSIONS_CONCENTRATION_THRESHOLD


def test_no_scenarios_remain_unsupported():
    # All seven capture-resistance moves, plus the quiet polity, now have a built
    # and scored detector.
    report = run_harness()
    unsupported = [r for r in report.results if not r.supported]
    assert unsupported == []
    assert report.supported == report.total == 8


def test_the_quiet_polity_is_caught_by_an_absolute_cadence():
    """Scenario 8: no adversary, no act, nobody lying — the polity simply stops.
    Every act-triggered signal is inert, so only the absolute re-affirmation
    cadence can see it. Before that cadence was wired in, this scored None."""
    r = next(r for r in run_harness().results if r.move == 8)
    assert r.supported and r.detected
    assert r.damage is not None and r.damage <= MAX_DAMAGE_QUIET
    assert r.passed
    assert MIN_SEVERITY_QUIET in r.notes
    assert "no act-triggered signal fired" in r.notes
