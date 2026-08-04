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
import pytest

import kith_covenant.harness as H
from kith_covenant import run_harness
from kith_covenant.harness import (
    MAX_DAMAGE_FORGED_QUORUM, ADMISSIONS_CONCENTRATION_THRESHOLD,
    INTERPRETATION_MONOPOLY_THRESHOLD, MAX_DAMAGE_MANUFACTURED_EVENT,
    MAX_DAMAGE_PRINCIPAL, MAX_DAMAGE_QUIET, MIN_SEVERITY_QUIET,
    HarnessParams, PARAM_RANGES, run_continuous, replay, params_for_seed,
    shrink_failure, sweep, max_damage_ceremony, max_damage_admissions,
    max_damage_interpretation, MAX_DAMAGE_CEREMONY, MAX_DAMAGE_ADMISSIONS,
    MAX_DAMAGE_INTERPRETATION, CORROBORATION_MIN, DEFAULT_QUORUM,
    MAX_DAMAGE_PROBATIONARY_WEIGHT,
)


def test_all_scenarios_present_all_supported():
    """Seven capture moves plus scenario 8, the quiet polity — which is not a
    capture move (it has no adversary) but is scored the same way. Move 7 carries
    two scenarios: who a faction can seat, and what a seated entrant can carry."""
    report = run_harness()
    assert report.total == 9
    assert report.supported == 9
    moves = sorted(r.move for r in report.results)
    assert moves == [1, 2, 3, 4, 5, 6, 7, 7, 8]
    # Every move has a scenario, and no scenario key is duplicated.
    assert set(moves) == {1, 2, 3, 4, 5, 6, 7, 8}
    keys = [r.key for r in report.results]
    assert len(set(keys)) == len(keys)


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
    r = next(r for r in run_harness().results if r.key == "admissions-capture")
    assert r.supported and r.detected
    # Flagged as soon as the sponsor reaches the concentration threshold.
    assert r.damage <= ADMISSIONS_CONCENTRATION_THRESHOLD


def test_no_scenarios_remain_unsupported():
    # All seven capture-resistance moves, plus the quiet polity, now have a built
    # and scored detector.
    report = run_harness()
    unsupported = [r for r in report.results if not r.supported]
    assert unsupported == []
    assert report.supported == report.total == 9


def test_probationary_weight_never_carries_a_second_class_act():
    """Move 7's second half. The criterion is 0 rather than a bound: an entrant's
    weight spent before its window closes must never validate, at the ordinary
    quorum or at the contraction bar. The scenario pairs each refusal with a
    standing-only control, so `detected` also asserts that no *unrelated* violation
    is doing the work."""
    r = next(r for r in run_harness().results if r.key == "probationary-weight")
    assert r.supported and r.detected and r.passed
    assert r.damage == MAX_DAMAGE_PROBATIONARY_WEIGHT == 0
    assert "refused 2 of 2" in r.notes
    assert "0 unrelated violation(s)" in r.notes


def test_probationary_weight_holds_across_the_swept_quorum():
    """The claim is general, not a property of quorum 2: at every quorum in the
    sweep, a faction one community short cannot top itself up with an entrant, and
    one at the quorum cannot reach the contraction bar that way."""
    for quorum in (2, 3, 4):
        p = HarnessParams(quorum_threshold=quorum, founding_size=quorum + 1).normalized()
        r = next(r for r in run_harness(p).results if r.key == "probationary-weight")
        assert r.passed and r.damage == 0, (quorum, r.notes)


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


# --- §10.3 continuous execution ---------------------------------------------
#
# Running `run_harness()` on a loop achieves nothing: with the parameters fixed it
# is a constant function. These cover the machinery that makes repeated runs
# informative — parameter variation, criteria stated as formulas, accumulation
# across rounds, shrinking, and sweeps — plus the two controls that show the
# machinery can actually fail, without which "no failures found" would be vacuous.

# The hand-authored case's damage, by move. Pinned so a parameterization that
# quietly moves the canonical fixture is caught rather than absorbed.
#
# Move 7 was 3 while the compact's only answer to admissions capture was legibility:
# the entrants a faction could seat before the composition lens flagged it. §2 turned
# that answer into refusal, and the faction can now seat exactly one community — its
# co-founder at compact formation, where there are no sitting members to draw an
# admitting quorum from. The 1 is a property of the gate, not of this fixture.
# Keyed by scenario key rather than by move: move 7 has two scenarios, and a
# move-keyed dict silently collapsed them into whichever ran last — which is exactly
# the kind of quiet absorption this pin exists to prevent.
_CANONICAL_DAMAGE = {
    "interpretation-monopoly": 3,
    "incapable-erosion": 1,
    "ceremony-hollowing": 1,
    "manufactured-event": 0,
    "forged-quorum": 0,
    "principal-capture": 1,
    "admissions-capture": 1,
    "probationary-weight": 0,
    "quiet-polity": 0,
}


def test_canonical_params_reproduce_the_hand_authored_results():
    default = run_harness()
    explicit = run_harness(HarnessParams.canonical())
    assert default.all_supported_passed and explicit.all_supported_passed
    assert {r.key: r.damage for r in default.results} == _CANONICAL_DAMAGE
    assert {r.key: r.damage for r in explicit.results} == _CANONICAL_DAMAGE
    assert all(r.detection_latency == 0 for r in default.results if r.supported)
    # The canonical witness sequence is the literal one the scenario used to hold.
    assert HarnessParams.canonical().witness_sequence() == [5, 4, 3]


def test_canonical_metrics_are_stable_across_runs():
    """Fresh Ed25519 keys are generated every run and several lenses tiebreak on DID
    ordering, so identical metrics across runs is a real claim: the detectors are
    key-order-independent. It is also what lets a seed reproduce a fixture without
    reproducing key material. Measured over 200 runs when the drivers were built;
    kept here at a size the suite can afford."""
    baseline = {r.key: (r.detection_latency, r.damage) for r in run_harness().results}
    for _ in range(4):
        again = {r.key: (r.detection_latency, r.damage) for r in run_harness().results}
        assert again == baseline


def test_criteria_formulas_reduce_to_their_constants_at_canonical():
    """The constants are retained as the canonical-case values of the formulas, so
    the docs and the demo can keep citing them by name."""
    p = HarnessParams.canonical()
    assert max_damage_ceremony(p) == MAX_DAMAGE_CEREMONY
    assert max_damage_admissions(p) == MAX_DAMAGE_ADMISSIONS
    assert max_damage_interpretation(p) == MAX_DAMAGE_INTERPRETATION


def test_drawn_parameters_are_internally_consistent():
    """A draw must never describe an impossible compact — a contraction needs
    quorum + 1 distinct communities, a forged act needs to be sub-quorum, and a
    control event cannot be corroborated by communities that do not exist. If a draw
    could break those, a scenario would fail because its *adversary* could not act,
    which is not what any of these criteria are about."""
    for seed in range(60):
        p = params_for_seed(seed)
        assert p == p.normalized()
        assert p.founding_size >= 3
        assert DEFAULT_QUORUM <= p.quorum_threshold <= p.founding_size - 1
        assert 1 <= p.forged_faction_size < p.quorum_threshold
        assert CORROBORATION_MIN <= p.control_corroborations <= p.founding_size - 1
        assert p.manufactured_corroborations < CORROBORATION_MIN
        assert p.ceremony_declines >= 1


def test_continuous_search_finds_no_failures_at_a_fixed_seed():
    """A fixed seed, so the suite can never redden on an unlucky draw. The wider
    search (400 rounds) and the full sweep are run out of band."""
    rep = run_continuous(rounds=8, seed=20260729, stop_on_failure=False)
    assert rep.rounds_run == 8
    assert rep.failures == ()
    assert rep.all_passed
    for s in rep.stats:
        assert s.runs == 8 and s.all_passed and s.pass_rate == 1.0


def test_continuous_metrics_actually_move():
    """The point of the whole exercise. If a future change re-fixes the fixture, the
    accumulated damage collapses back to one value per scenario and this fails —
    which is the only way to notice that continuous execution has gone back to
    recomputing a constant."""
    rep = run_continuous(rounds=10, seed=11, stop_on_failure=False)
    varying = [s.key for s in rep.stats if len(s.damage_histogram) > 1]
    assert "ceremony-hollowing" in varying
    assert "interpretation-monopoly" in varying
    # `admissions-capture` deliberately no longer varies, and that is not this
    # control going slack: since §2, what a faction can seat is decided by the gate
    # rather than by how long the lens takes to notice, and the gate's answer is one
    # co-founder whatever the parameters say. Pinned as a constant so the invariance
    # is asserted rather than merely tolerated — if it ever moves, that is a gate
    # regression and this fails.
    admissions = next(s for s in rep.stats if s.key == "admissions-capture")
    assert set(dict(admissions.damage_histogram)) == {1}, admissions.damage_histogram


def test_the_old_constant_criterion_fails_on_a_flat_prefix():
    """The finding, pinned. `MAX_DAMAGE_CEREMONY = 1` was not a property of the
    ceremony-decay detector — it encoded `witness_by_cycle = [5, 4, 3]`. A flat
    prefix offers the lens nothing to compare, so charging those cycles as
    undetected damage blames the detector for the fixture's shape."""
    original = H.max_damage_ceremony
    try:
        H.max_damage_ceremony = lambda p: H.MAX_DAMAGE_CEREMONY  # the pre-slice form
        old = sweep("ceremony_flat_prefix")
        assert old[0].value == 0 and old[0].passed, "the canonical case passed before too"
        assert [pt.passed for pt in old[1:]] == [False] * (len(old) - 1)
    finally:
        H.max_damage_ceremony = original
    # With the formula, every prefix passes and the damage tracks the prefix exactly.
    for pt in sweep("ceremony_flat_prefix"):
        r = pt.result("ceremony-hollowing")
        assert pt.passed
        assert r.damage == 1 + pt.value == r.threshold_damage


def test_a_broken_criterion_is_caught_shrunk_and_replayable():
    """Without this the search's silence would be unfalsifiable: a harness that
    cannot report a failure always passes."""
    original = H.max_damage_ceremony
    try:
        H.max_damage_ceremony = lambda p: 0  # impossible: any decline costs 1
        rep = run_continuous(rounds=5, seed=3, stop_on_failure=True)
        assert rep.failures, "an impossible bound must be reported"
        f = rep.failures[0]
        assert f.key == "ceremony-hollowing"
        assert f.damage > f.threshold_damage
        # The shrunk set still fails and is no further from canonical than the draw.
        assert not run_harness(f.shrunk).all_supported_passed
        canonical = HarnessParams.canonical()
        assert f.shrunk.ceremony_flat_prefix <= f.params.ceremony_flat_prefix
        assert f.shrunk.ceremony_flat_prefix == canonical.ceremony_flat_prefix
        # And the seed alone reproduces the round.
        assert not replay(f.seed).all_supported_passed
    finally:
        H.max_damage_ceremony = original
    assert run_harness().all_supported_passed


def test_shrinking_walks_a_distant_failure_back_to_canonical():
    original = H.max_damage_ceremony
    try:
        H.max_damage_ceremony = lambda p: 0
        far = HarnessParams(
            founding_size=5, quorum_threshold=4, forged_faction_size=3,
            ceremony_flat_prefix=3, ceremony_declines=3, admissions_threshold=5,
            interpretation_threshold=5, cohort_size=5, quiet_steps=6,
            control_corroborations=4,
        )
        shrunk = shrink_failure(far, "ceremony-hollowing")
        assert not run_harness(shrunk).all_supported_passed
        # Every field the failure does not need is back at its canonical value.
        canonical = HarnessParams.canonical()
        assert shrunk.ceremony_flat_prefix == canonical.ceremony_flat_prefix
        assert shrunk.founding_size == canonical.founding_size
        assert shrunk.admissions_threshold == canonical.admissions_threshold
    finally:
        H.max_damage_ceremony = original


def test_shrinking_a_passing_set_is_a_no_op():
    p = params_for_seed(5)
    assert shrink_failure(p, "ceremony-hollowing") == p


def test_sweep_covers_its_declared_range():
    pts = sweep("ceremony_flat_prefix")
    assert tuple(pt.value for pt in pts) == PARAM_RANGES["ceremony_flat_prefix"]
    for pt in pts:
        r = pt.result("ceremony-hollowing")
        assert pt.passed and pt.failed_keys == ()
        # The bound moves with the parameter — the criterion is the formula, not 1.
        assert r.threshold_damage == 1 + pt.value

    # And a sweep whose bound is deliberately flat: since §2 refuses the chain, the
    # entrants a faction may seat do not depend on how patient it is.
    for pt in sweep("admissions_threshold"):
        assert pt.passed and pt.failed_keys == ()
        assert pt.result("admissions-capture").threshold_damage == 1


def test_sweep_rejects_an_unknown_parameter():
    with pytest.raises(ValueError, match="unknown parameter"):
        sweep("no_such_knob")


def test_run_continuous_respects_a_wall_clock_budget():
    """Bounded and returning, not a daemon: the cadence belongs to the caller."""
    rep = run_continuous(rounds=500, seed=1, stop_on_failure=False,
                         time_budget_seconds=0.5)
    assert rep.rounds_run < 500
    assert rep.elapsed_seconds < 30
