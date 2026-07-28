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

"""The cohesion monitor from Python: the chartered roll against what the substrate
currently shows, graded into an erosion level."""
from __future__ import annotations

import pytest

import ethne as e
from ethne.authoring import (
    Signer,
    confer_office,
    designate_derived,
    prune_observations,
    record_observation,
)
from ethne.monitor import Thresholds, cohesion_report, founding_candidate


def _candidate(dids, reputation=0.9):
    return founding_candidate([(d, reputation, None, None, False) for d in dids])


def test_a_fully_present_healthy_roll_shows_no_erosion(polity):
    roll = polity.roll(as_of=300)
    report = cohesion_report(roll, _candidate(roll))
    assert report.erosion == "None"
    assert report.attrition == 0.0
    assert report.observed == report.chartered == 3
    assert report.reputation_floor_breaches == 0
    assert not report.opaque_boundary


def test_attrition_grades_notice_then_concern(polity):
    roll = polity.roll(as_of=300)
    # One of three gone: a third of the roll — past the Notice bar, short of Concern.
    assert cohesion_report(roll, _candidate(roll[:2])).erosion == "Notice"
    # Two of three gone.
    concern = cohesion_report(roll, _candidate(roll[:1]))
    assert concern.erosion == "Concern"
    assert round(concern.attrition, 3) == 0.667


def test_an_opaque_boundary_caps_attrition_at_notice(polity):
    roll = polity.roll(as_of=300)
    thinned = _candidate(roll[:1])
    assert cohesion_report(roll, thinned, opaque_boundary=False).erosion == "Concern"
    capped = cohesion_report(roll, thinned, opaque_boundary=True)
    assert capped.erosion == "Notice"
    assert capped.opaque_boundary


def test_reputation_decay_and_floor_breaches(polity):
    roll = polity.roll(as_of=300)
    faded = founding_candidate([(d, 0.2, None, None, False) for d in roll])
    report = cohesion_report(roll, faded)
    assert report.reputation_floor_breaches == 3
    assert report.mean_reputation == pytest.approx(0.2)
    assert report.erosion == "Concern"


def test_thresholds_can_be_loosened_or_tightened(polity):
    roll = polity.roll(as_of=300)
    thinned = _candidate(roll[:1])
    assert cohesion_report(roll, thinned).erosion == "Concern"
    # Raise the Concern bar above the observed attrition: Notice remains.
    loose = cohesion_report(roll, thinned, thresholds=Thresholds(attrition_concern=0.9))
    assert loose.erosion == "Notice"
    # Lower both bars under it: even a full roll now grades Concern.
    strict = cohesion_report(
        roll, _candidate(roll), thresholds=Thresholds(mean_reputation_concern=0.95)
    )
    assert strict.erosion == "Concern"


def test_candidate_exposes_members_and_reputations(polity):
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    candidate = founding_candidate(
        [(roll[0], 0.75, guardian, None, True)] + [(d, 0.5, None, None, False) for d in roll[1:]],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    assert candidate.members[0] == (roll[0], 0.75, guardian, None, True)
    assert candidate.member_reputation(roll[1]) == 0.5
    assert candidate.member_reputation(e.generate_keypair()[2]) is None
    assert candidate.substrate_name == "tradition"
    assert candidate.substrate_anchor is None


# --- attendance: are the humans answerable for these machines present? -------

ATTENDED_AT = 1_721_800_000


def _guarded(dids, guardian, attested_at=ATTENDED_AT, reputation=0.9):
    """A candidate whose members all answer to `guardian`, last seen present at
    `attested_at` (pass None for never attended)."""
    return founding_candidate([(d, reputation, guardian, attested_at, True) for d in dids])


def test_guardians_present_shows_no_erosion(polity):
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    report = cohesion_report(roll, _guarded(roll, guardian), as_of=ATTENDED_AT + 60)
    assert report.guardian_members == len(roll)
    assert report.unattended == 0
    assert report.erosion == "None"


def test_stale_attendance_raises_a_notice(polity):
    # Read an hour after the last verified presence: past the 300s cadence.
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    report = cohesion_report(roll, _guarded(roll, guardian), as_of=ATTENDED_AT + 3600)
    assert report.unattended == len(roll)
    assert report.erosion == "Notice"


def test_absent_guardians_never_reach_concern(polity):
    # The cap, from Python: nobody present anywhere is the worst this signal can
    # say, and it still stops at Notice. Otherwise a fleet whose operators went
    # home would cross to kith-covenant every night.
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    report = cohesion_report(roll, _guarded(roll, guardian, attested_at=None))
    assert report.unattended == len(roll)
    assert report.erosion == "Notice"


def test_members_without_a_guardian_are_outside_the_measure(polity):
    # The default candidate has no guardians at all, so attendance says nothing
    # and a machine-only polity does not sit at Notice.
    roll = polity.roll(as_of=300)
    report = cohesion_report(roll, _candidate(roll))
    assert report.guardian_members == 0
    assert report.unattended == 0
    assert report.erosion == "None"


def test_the_attendance_cadence_is_a_query_parameter(polity):
    # Same stamps, same instant, different tolerance for staleness.
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    candidate = _guarded(roll, guardian)
    read_at = ATTENDED_AT + 3600

    strict = cohesion_report(roll, candidate, as_of=read_at)
    assert strict.erosion == "Notice"

    tolerant = cohesion_report(
        roll,
        candidate,
        thresholds=Thresholds(attendance_cadence_seconds=7200),
        as_of=read_at,
    )
    assert tolerant.unattended == 0
    assert tolerant.erosion == "None"


def test_candidate_exposes_the_attendance_stamp(polity):
    roll = polity.roll(as_of=300)
    guardian = e.generate_keypair()[2]
    candidate = _guarded(roll[:1], guardian)
    assert candidate.members[0] == (roll[0], 0.9, guardian, ATTENDED_AT, True)


# --- transmission decay: is the substrate still being re-affirmed? ------------

DAY = 24 * 60 * 60


def test_a_currently_affirming_roll_shows_no_erosion(polity):
    roll = polity.roll(as_of=300)
    affirmations = polity.archive.affirmation_state(polity.did, as_of=300)
    report = cohesion_report(
        roll, _candidate(roll), affirmations=affirmations, as_of=300
    )
    assert report.affirming_members == len(roll)
    assert report.stale_affirmations == 0
    assert report.erosion == "None"


def test_lapsed_affirmations_reach_concern(polity):
    # The counterpart to the attendance cap: transmission decay DOES escalate,
    # because a member a whole cadence past re-affirming is not merely away.
    roll = polity.roll(as_of=300)
    later = 300 + 400 * DAY
    affirmations = polity.archive.affirmation_state(polity.did, as_of=later)
    report = cohesion_report(
        roll, _candidate(roll), affirmations=affirmations, as_of=later
    )
    assert report.stale_affirmations == len(roll)
    assert report.erosion == "Concern"


def test_the_affirmation_cadence_is_a_query_parameter(polity):
    roll = polity.roll(as_of=300)
    later = 300 + 400 * DAY
    affirmations = polity.archive.affirmation_state(polity.did, as_of=later)

    strict = cohesion_report(
        roll, _candidate(roll), affirmations=affirmations, as_of=later
    )
    assert strict.erosion == "Concern"

    tolerant = cohesion_report(
        roll,
        _candidate(roll),
        thresholds=Thresholds(affirmation_cadence_seconds=1000 * DAY),
        affirmations=affirmations,
        as_of=later,
    )
    assert tolerant.stale_affirmations == 0
    assert tolerant.erosion == "None"


def test_omitting_affirmations_is_not_evaluated_not_healthy(polity):
    # No reading supplied → the component says nothing. It must not report zero
    # stale out of zero members as health.
    roll = polity.roll(as_of=300)
    report = cohesion_report(roll, _candidate(roll), as_of=300 + 400 * DAY)
    assert report.affirming_members == 0
    assert report.stale_affirmations == 0
    assert report.erosion == "None"

# --- D15: chartered guardian edges vs what the substrate shows ---------------


def _designate(polity, members, guardian):
    """Charter each member a derived party answering to `guardian`."""
    for m in members:
        designate_derived(polity.archive, polity.did, m, polity.quorum(as_of=200),
                          guardian=guardian, decided_at=200)


def test_a_chartered_guardian_the_substrate_no_longer_shows_is_erosion(wide):
    human = Signer.generate()
    roll = wide.roll(as_of=250)
    _designate(wide, roll, human)
    guardians = wide.archive.guardian_state(wide.did, as_of=250)
    assert len(guardians.guarded_members()) == 4

    # All four still observed; two no longer carry a guardian at all.
    observed = [
        (roll[0], 0.9, human.did, 250, True),
        (roll[1], 0.9, human.did, 250, True),
        (roll[2], 0.9, None, None, True),
        (roll[3], 0.9, None, None, True),
    ]
    report = cohesion_report(roll, founding_candidate(observed),
                             guardians=guardians, as_of=250)
    assert report.chartered_guardian_members == 4
    assert report.guardian_edges_lost == 2
    # Half the edges gone: this component escalates, unlike absent attendance.
    assert report.erosion == "Concern"


def test_an_unobserved_member_is_attrition_not_a_lost_edge(wide):
    human = Signer.generate()
    roll = wide.roll(as_of=250)
    _designate(wide, roll, human)
    guardians = wide.archive.guardian_state(wide.did, as_of=250)

    # Only two of four observed, both still guarded.
    observed = [(roll[0], 0.9, human.did, 250, True), (roll[1], 0.9, human.did, 250, True)]
    report = cohesion_report(roll, founding_candidate(observed),
                             guardians=guardians, as_of=250)
    assert report.chartered_guardian_members == 2, "the denominator is observed, not chartered"
    assert report.guardian_edges_lost == 0
    assert report.attrition == 0.5


def test_omitting_the_guardian_state_is_not_evaluated(wide):
    roll = wide.roll(as_of=250)
    observed = [(d, 0.9, None, None, False) for d in roll]
    report = cohesion_report(roll, founding_candidate(observed), as_of=250)
    assert report.chartered_guardian_members == 0
    assert report.guardian_edges_lost == 0
    assert report.erosion == "None"


def test_the_guardian_bounds_are_query_parameters(wide):
    human = Signer.generate()
    roll = wide.roll(as_of=250)
    _designate(wide, roll, human)
    guardians = wide.archive.guardian_state(wide.did, as_of=250)
    observed = [
        (roll[0], 0.9, None, None, False),
        (roll[1], 0.9, human.did, 250, True),
        (roll[2], 0.9, human.did, 250, True),
        (roll[3], 0.9, human.did, 250, True),
    ]
    # One of four: a Notice by default...
    assert cohesion_report(roll, founding_candidate(observed),
                           guardians=guardians, as_of=250).erosion == "Notice"
    # ...and nothing at all under a looser bound.
    tolerant = Thresholds(guardian_lost_notice=0.5, guardian_lost_concern=0.9)
    assert cohesion_report(roll, founding_candidate(observed), thresholds=tolerant,
                           guardians=guardians, as_of=250).erosion == "None"

# --- D16: trends and the observation window ----------------------------------

DAY = 86_400


def test_the_trend_series_comes_from_the_polity_own_records(polity):
    """No provider involved: gap-widening and witness turnout are charter-log facts."""
    joiner = Signer.generate()
    from ethne.authoring import admit_member, induct_member, reaffirm_member

    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    induct_member(polity.archive, polity.did, joiner, polity.founders, affirmed_at=300)
    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=400)
    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=5_400)
    polity.archive.check_authority()

    t = polity.archive.decay_trends(polity.did, as_of=6_000)
    assert [w for _at, w, kind in t.affirmations if kind == "Induction"] == [3]
    assert t.gaps_seconds == [100, 5_000]
    # The latest gap dwarfs the median of the prior ones.
    assert t.latest_gap_widening() is True
    assert [kind for _at, kind in t.membership_changes] == ["Admit"]


def test_a_widening_gap_warns_but_does_not_escalate(polity):
    from ethne.authoring import admit_member, induct_member, reaffirm_member

    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    induct_member(polity.archive, polity.did, joiner, polity.founders, affirmed_at=300)
    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=400)
    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=5_400)

    roll = polity.roll(as_of=6_000)
    observed = [(d, 0.9, None, None, False) for d in roll]
    report = cohesion_report(
        roll, founding_candidate(observed),
        trends=polity.archive.decay_trends(polity.did, as_of=6_000),
        as_of=6_000,
    )
    assert report.latest_gap_widening is True
    assert report.erosion == "Notice", "a trend is not yet a lapse"


def test_a_watched_absence_that_outlasts_a_day_reaches_concern(polity):
    """The escape hatch the attendance cap has always pointed at."""
    monitor = Signer.generate()
    human = Signer.generate()
    machine = polity.founders[2].did
    # Present at t=0, then nobody for a day, watched throughout.
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, 0)], observed_at=0)
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, None)], observed_at=DAY // 2)
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, None)], observed_at=DAY)
    polity.archive.verify_all()
    polity.archive.check_authority()

    w = polity.archive.observation_window(polity.did, since=0, as_of=DAY + 1)
    assert w.reading_count() == 3
    assert w.readings_for(machine) == 3
    assert w.unattended_span(machine, as_of=DAY + 1) == DAY + 1

    roll = polity.roll(as_of=DAY + 1)
    observed = [(d, 0.9, human.did if d == machine else None, 0, d == machine) for d in roll]
    report = cohesion_report(roll, founding_candidate(observed), window=w, as_of=DAY + 1)
    assert report.sustained_absences == 1
    assert report.erosion == "Concern"


def test_one_stale_reading_with_nobody_watching_escalates_nothing(polity):
    monitor = Signer.generate()
    human = Signer.generate()
    machine = polity.founders[2].did
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, 0)], observed_at=0)

    w = polity.archive.observation_window(polity.did, since=0, as_of=10 * DAY)
    roll = polity.roll(as_of=10 * DAY)
    observed = [(d, 0.9, human.did if d == machine else None, 0, d == machine) for d in roll]
    report = cohesion_report(roll, founding_candidate(observed), window=w, as_of=10 * DAY)
    assert report.sustained_absences == 0, "the span is the absence; the count is the witness"
    # The snapshot signal still speaks, and is still capped.
    assert report.erosion == "Notice"


def test_an_observation_must_be_signed_by_its_observer(polity):
    monitor = Signer.generate()
    # There is no way to author someone else's observation through the surface: the
    # observer's own key is what signs it.
    record_observation(polity.archive, polity.did, monitor,
                       [(polity.founders[0].did, 0.9, None, None)], observed_at=100)
    polity.archive.check_authority()
    w = polity.archive.observation_window(polity.did, since=0, as_of=200)
    assert w.observations[0][1] == monitor.did



# --- D17: pruning the observation log ---------------------------------------

SETTLED = 40 * DAY  # comfortably past the 30-day settling horizon


def test_a_majority_prunes_and_compaction_drops_only_observations(polity):
    monitor = Signer.generate()
    watched = polity.founders[2].did
    for when in (100, 200, 5 * DAY):
        record_observation(polity.archive, polity.did, monitor,
                           [(watched, 0.9, None, None)], observed_at=when)
    before = len(polity.archive)

    prune_observations(polity.archive, polity.did, horizon=DAY,
                       deciders=polity.quorum(as_of=6 * DAY), pruned_at=6 * DAY)
    polity.archive.verify_all()
    polity.archive.check_authority()

    compacted = polity.archive.apply_prunings(polity.did, as_of=6 * DAY)
    # Two observations went; the charter and the pruning record stayed.
    assert len(compacted) == before + 1 - 2
    kinds = [k for _cid, k in compacted.entries()]
    assert "PolityChartered" in kinds and "ObservationsPruned" in kinds
    assert kinds.count("Observed") == 1
    compacted.check_authority()

    w = compacted.observation_window(polity.did, since=0, as_of=6 * DAY)
    assert w.reading_count() == 1
    assert w.pruned_before == DAY


def test_a_charter_keeper_prunes_alone_only_past_the_settling_horizon(polity):
    """The custody that office had no record to exercise until now."""
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    monitor = Signer.generate()
    record_observation(polity.archive, polity.did, monitor,
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=300)

    # Beyond the horizon: one signature carries it.
    prune_observations(polity.archive, polity.did, horizon=SETTLED,
                       deciders=[keeper], pruned_at=SETTLED + 31 * DAY)
    polity.archive.check_authority()


def test_a_charter_keeper_may_not_prune_recent_evidence(polity):
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    monitor = Signer.generate()
    record_observation(polity.archive, polity.did, monitor,
                       [(polity.founders[2].did, 0.9, None, None)], observed_at=300)
    # A horizon one day back: inside the settling period, so the roll must decide.
    prune_observations(polity.archive, polity.did, horizon=10 * DAY,
                       deciders=[keeper], pruned_at=11 * DAY)

    with pytest.raises(RuntimeError, match="CharterKeeper"):
        polity.archive.check_authority()


def test_pruning_only_ever_weakens_the_absence_it_measures(polity):
    """The invariant: evidence destroyed can shorten a measured absence, never
    lengthen one."""
    monitor = Signer.generate()
    human = Signer.generate()
    machine = polity.founders[2].did
    # Present at 5*DAY, then a staler reading reported at 6*DAY.
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, 5 * DAY)], observed_at=5 * DAY)
    record_observation(polity.archive, polity.did, monitor,
                       [(machine, 0.9, human.did, DAY)], observed_at=6 * DAY)
    polity.archive.check_authority()

    as_of = 10 * DAY
    before = polity.archive.observation_window(polity.did, since=0, as_of=as_of)
    span_before = before.unattended_span(machine, as_of=as_of)
    assert span_before == 5 * DAY

    prune_observations(polity.archive, polity.did, horizon=6 * DAY - 1,
                       deciders=polity.quorum(as_of=7 * DAY), pruned_at=7 * DAY)
    compacted = polity.archive.apply_prunings(polity.did, as_of=7 * DAY)
    after = compacted.observation_window(polity.did, since=0, as_of=as_of)

    assert after.pruned_before == 6 * DAY - 1
    assert after.last_attested(machine) == DAY, "the fresher stamp was pruned away"
    span_after = after.unattended_span(machine, as_of=as_of)
    # Without the clamp this would read 9 days — nearly twice the true absence.
    assert span_after <= span_before
    assert span_after == as_of - (6 * DAY - 1)


def test_a_stamp_may_not_postdate_the_observation_reporting_it(polity):
    """A monitor cannot have seen an attestation that had not happened yet — the
    rule the pruning clamp's soundness rests on."""
    monitor = Signer.generate()
    record_observation(polity.archive, polity.did, monitor,
                       [(polity.founders[2].did, 0.9, None, 900)], observed_at=300)
    with pytest.raises(RuntimeError, match="postdates the observation"):
        polity.archive.check_authority()


def test_a_settled_cadence_slip_is_caught_by_the_pinned_reading(polity):
    """The shifting-baseline case through the Python surface: gaps 100, 400, 400,
    400. The rolling median has absorbed the drift and goes quiet; the reading
    pinned to the founding cadence still sees the slip, and still warns."""
    from ethne.authoring import admit_member, induct_member, reaffirm_member

    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    induct_member(polity.archive, polity.did, joiner, polity.founders, affirmed_at=300)
    for at in (400, 800, 1_200, 1_600):
        reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=at)

    t = polity.archive.decay_trends(polity.did, as_of=1_700)
    assert t.gaps_seconds == [100, 400, 400, 400]
    assert t.latest_gap_widening() is False
    assert t.gap_widening_since_founding() is True

    roll = polity.roll(as_of=1_700)
    report = cohesion_report(
        roll, founding_candidate([(d, 0.9, None, None, False) for d in roll]),
        trends=t, as_of=1_700,
    )
    assert report.latest_gap_widening is False
    assert report.gap_widening_since_founding is True
    assert report.erosion == "Notice", "still capped at Notice"


# --- D19: the retention policy over the observation log ----------------------
#
# A policy, not a schedule: the loop and the clock stay with the host. What the
# tier owes is the safety floor — which history a live measure still needs, and
# which authorization route a horizon leaves open.


def _observed_log(polity, count, first_at, step):
    """`count` observations of one member, one per `step` seconds from `first_at`."""
    monitor = Signer.generate()
    watched = polity.founders[2].did
    for i in range(count):
        record_observation(polity.archive, polity.did, monitor,
                           [(watched, 0.9, None, None)], observed_at=first_at + i * step)
    return watched


def test_a_recommendation_names_what_it_drops_and_who_may_authorize_it(polity):
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    _observed_log(polity, 5, first_at=DAY, step=DAY)
    as_of = 100 * DAY

    rec = polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of)
    assert rec is not None
    assert rec.seconds_retained == e.DEFAULT_RETENTION["retain_seconds"]
    assert rec.horizon == as_of - 30 * DAY
    assert rec.dropped == 5
    # Both routes are open at the default retention, cheapest first.
    assert rec.routes == ["CharterKeeper", "RollMajority"]
    assert rec.keeper_may_authorize is True
    assert rec.keeper_holders == [keeper.did]
    assert rec.roll_majority_need == e.majority(len(polity.roll(as_of)))


def test_the_default_retention_is_the_cheapest_to_authorize(polity):
    """The safe default is also the cheap one: 30 days is the smallest retention
    that leaves the single-`CharterKeeper` route open."""
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    _observed_log(polity, 5, first_at=DAY, step=DAY)
    as_of = 100 * DAY

    at_default = polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of)
    assert at_default.keeper_may_authorize is True

    # One second shorter and the office can no longer carry it alone.
    shorter = polity.archive.prune_recommendation(
        polity.did, retain_seconds=30 * DAY - 1, min_dropped=1, min_witness_readings=0,
        as_of=as_of)
    assert shorter.keeper_may_authorize is False
    assert shorter.routes == ["RollMajority"]
    assert shorter.keeper_holders == []


def test_a_short_retention_can_never_clip_a_live_measure(polity):
    """`retain_seconds` is honoured up to the live-measure floor and no further."""
    _observed_log(polity, 5, first_at=DAY, step=DAY)
    as_of = 100 * DAY

    rec = polity.archive.prune_recommendation(
        polity.did, retain_seconds=60, min_dropped=1, min_witness_readings=0, as_of=as_of)
    # Asked for a minute; floored at the day a sustained absence reads back over.
    assert rec.seconds_retained == e.DEFAULT_RETENTION["live_measure_seconds"]
    assert rec.horizon == as_of - DAY


def test_nothing_worth_doing_recommends_nothing(polity):
    _observed_log(polity, 3, first_at=DAY, step=DAY)
    as_of = 100 * DAY
    # Below the churn floor: three observations do not justify a permanent record.
    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=4, min_witness_readings=0, as_of=as_of) is None
    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=3, min_witness_readings=0, as_of=as_of) is not None


def test_the_recommendation_goes_quiet_once_acted_on(polity):
    _observed_log(polity, 5, first_at=DAY, step=DAY)
    as_of = 100 * DAY
    rec = polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of)

    prune_observations(polity.archive, polity.did, horizon=rec.horizon,
                       deciders=polity.quorum(as_of=as_of), pruned_at=as_of)
    # A horizon that does not advance would claim to drop what is already gone.
    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of) is None


def test_the_default_policy_fires_on_a_year_of_attendance_cadence(polity):
    """The defaults are not decorative: a polity still being watched, with a long
    tail behind it, clears `min_dropped` of 100 with no override at all."""
    as_of = 200 * DAY
    _observed_log(polity, 150, first_at=50 * DAY, step=DAY)
    rec = polity.archive.prune_recommendation(polity.did, as_of=as_of)
    # The duration floor binds: everything older than 30 days goes, and the three
    # readings the witness floor protects are well inside that.
    assert rec is not None and rec.dropped == 120
    assert rec.seconds_retained == 30 * DAY


def test_a_recommendation_only_ever_proposes_what_authority_accepts(polity):
    """The property the whole read-model exists for. Author a pruning at exactly the
    horizon it returns, on the officer route it says is open, and the authority pass
    must accept it — then acting on it must weaken no live signal."""
    keeper = polity.founders[0]
    confer_office(polity.archive, polity.did, "CharterKeeper", keeper,
                  polity.quorum(as_of=200), decided_at=200)
    monitor = Signer.generate()
    human = Signer.generate()
    machine = polity.founders[2].did
    designate_derived(polity.archive, polity.did, machine, polity.quorum(as_of=300),
                      guardian=human, decided_at=300)
    as_of = 100 * DAY
    # An old attended history, well past the horizon — this is what goes.
    for i in range(5):
        record_observation(polity.archive, polity.did, monitor,
                           [(machine, 0.9, human.did, (i + 1) * DAY)],
                           observed_at=(i + 1) * DAY)
    # An absence that *begins inside* the retained window: attested ten days ago,
    # unattended since, and still being watched. This is the signal that must survive.
    stale = as_of - 10 * DAY
    for at in (stale, as_of - 2 * DAY, as_of - DAY):
        record_observation(polity.archive, polity.did, monitor,
                           [(machine, 0.9, human.did, stale)], observed_at=at)

    before = polity.archive.observation_window(polity.did, since=0, as_of=as_of)
    span_before = before.unattended_span(machine, as_of=as_of)
    assert span_before == 10 * DAY, "the signal under test must be non-trivial"

    rec = polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of)
    assert rec.keeper_may_authorize and rec.keeper_holders == [keeper.did]
    assert rec.dropped == 5, "and the pruning under test must actually drop something"

    prune_observations(polity.archive, polity.did, horizon=rec.horizon,
                       deciders=[keeper], pruned_at=as_of)
    polity.archive.verify_all()
    polity.archive.check_authority()   # <- the property

    compacted = polity.archive.apply_prunings(polity.did, as_of=as_of)
    after = compacted.observation_window(polity.did, since=0, as_of=as_of)
    assert after.unattended_span(machine, as_of=as_of) == span_before, \
        "acting on the recommendation weakened no live signal"


def test_an_unchartered_or_dissolved_polity_recommends_nothing(polity):
    from ethne.authoring import dissolve_charter

    empty = e.Archive()
    assert empty.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=100 * DAY) is None

    _observed_log(polity, 5, first_at=DAY, step=DAY)
    as_of = 100 * DAY
    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of) is not None
    dissolve_charter(polity.archive, polity.did, polity.quorum(as_of=as_of),
                     rationale="done", dissolved_at=as_of - 1)
    # A dissolved polity has no housekeeping to do; its log is what remains of it.
    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of) is None


def _absent_machine(polity, observed_at, as_of):
    """A guardian-answering member unattended since `stale`, watched at each of
    `observed_at`. Returns a reader giving (sustained_absences, erosion, span)."""
    mon, human = Signer.generate(), Signer.generate()
    machine = polity.founders[2].did
    designate_derived(polity.archive, polity.did, machine, polity.quorum(as_of=300),
                      guardian=human, decided_at=300)
    stale = min(observed_at)
    for at in observed_at:
        record_observation(polity.archive, polity.did, mon,
                           [(machine, 0.9, human.did, stale)], observed_at=at)
    roll = polity.roll(as_of=as_of)
    cand = founding_candidate(
        [(d, 0.9, human.did if d == machine else None, None, d == machine) for d in roll])

    def read(arch):
        w = arch.observation_window(polity.did, since=0, as_of=as_of)
        report = cohesion_report(roll, cand, window=w, as_of=as_of)
        return report.sustained_absences, report.erosion
    return machine, read


def test_a_hand_picked_horizon_can_silently_downgrade_a_concern(polity):
    """The defect the duration floor alone does not catch, stated as the failure it
    is. A sustained absence is graded on a span *and* on a witness count; pruning
    clamps the span, but nothing protected the count."""
    as_of = 100 * DAY
    sparse = [as_of - 3 * DAY, as_of - 2 * DAY, as_of - DAY, as_of - 3600]
    _machine, read = _absent_machine(polity, sparse, as_of)
    assert read(polity.archive) == (1, "Concern")

    # A host reasoning "a sustained absence reads back one day, so keep one day".
    prune_observations(polity.archive, polity.did, horizon=as_of - DAY,
                       deciders=polity.quorum(as_of=as_of), pruned_at=as_of)
    after = polity.archive.apply_prunings(polity.did, as_of=as_of)
    assert read(after) == (0, "Notice"), \
        "two readings survive, one short of the witness count the grade needs"


def test_the_recommendation_preserves_the_concern_at_any_cadence(polity, found):
    """And the fix: the same absence, the same retention ask, graded the same before
    and after — whether the monitors were watching four times or forty."""
    as_of = 100 * DAY
    for step, count in ((DAY, 4), (3600, 40)):
        p = found(3)
        times = [as_of - 3 * DAY + i * step for i in range(count)]
        machine, read = _absent_machine(p, [t for t in times if t <= as_of], as_of)
        before = read(p.archive)
        assert before == (1, "Concern"), f"cadence {step}s must start at Concern"

        # The same ask a host would make by hand, put through the read-model.
        rec = p.archive.prune_recommendation(
            p.did, retain_seconds=DAY, min_dropped=1, as_of=as_of)
        assert rec is not None and rec.dropped > 0, "the prune under test must be real"
        # And it keeps MORE than the day asked for — the witness floor holding it
        # back is the whole difference from the hand-picked horizon above.
        assert rec.seconds_retained > DAY
        prune_observations(p.archive, p.did, horizon=rec.horizon,
                           deciders=p.quorum(as_of=as_of), pruned_at=as_of)
        p.archive.check_authority()
        after = p.archive.apply_prunings(p.did, as_of=as_of)
        assert read(after) == before, f"cadence {step}s lost its grade"


def test_a_thinly_watched_member_pins_the_horizon(polity):
    """Per member, because `readings_for` is per member. The polity may still prune
    by majority — what is refused is *recommending* it."""
    mon = Signer.generate()
    watched, thin = polity.founders[1].did, polity.founders[2].did
    for i in range(20):
        record_observation(polity.archive, polity.did, mon,
                           [(watched, 0.9, None, None)], observed_at=DAY + i * 3600)
    for i in range(3):
        record_observation(polity.archive, polity.did, mon,
                           [(thin, 0.9, None, None)], observed_at=DAY + i * 60)
    as_of = 100 * DAY

    assert polity.archive.prune_recommendation(
        polity.did, min_dropped=1, as_of=as_of) is None, \
        "the thinly-watched member has no reading to spare"
    # Switch the floor off and the duration floor would drop all twenty-three.
    loose = polity.archive.prune_recommendation(
        polity.did, min_dropped=1, min_witness_readings=0, as_of=as_of)
    assert loose.dropped == 23
