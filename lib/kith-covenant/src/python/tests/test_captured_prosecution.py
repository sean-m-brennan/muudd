#  Copyright 2026 Sean M. Brennan and contributors
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

"""The captured-prosecution probe, and the claims it owns.

``kith_covenant.captured`` runs six probes over one fixture: a compact of seven whose
majority is complicit with the member it should be prosecuting. It is the source of the
measurements in ``doc/architecture/captured-prosecution.md``, and this file is what
keeps the document honest — every number the document states is asserted here against a
live run.

**What the probe measured against the protocol as it stood.** Burial, prosecution and
calm read identically on every instrument the protocol had, because burial *produces*
observances: a panel sat, arguments were filed and answered, a decision was recorded.
A bystander that was party to neither side could walk a dispute back down the ladder
and hold it there. A dismissal reached backwards and unauthorized transitions that had
been sound when they were made. And the complicit majority could run the honest
complainant up the ladder without moving a single instrument, because enforcement
direction reads standing and an honest complainant holds full standing.

The rule coverage is in the Rust suites and in ``test_non_pursuit.py``; what this file
asserts is that the probe still measures what the document says it measured.
"""
from __future__ import annotations

import pytest

from kith_covenant.captured import (
    CapturedParams, MIN_REPEAT_SEATS, RUNGS_WITHOUT_OFFICE, SEAM_REFUSALS,
    format_report, run_probe,
)


@pytest.fixture(scope="module")
def report():
    """One run, shared: six probes over several forked archives take a few seconds."""
    return run_probe()


# --- the probe as a whole -----------------------------------------------------

def test_every_probe_passes(report):
    assert report.all_passed, format_report(report)


def test_what_the_captured_compact_still_gets_away_with(report):
    """Three, and all three are reported rather than fixed. A compact whose majority is
    complicit cannot be made to act by a rule; it can only be made legible."""
    assert len(report.unanswered) == 3
    joined = " ".join(report.unanswered)
    assert "cannot designate" in joined
    assert "re-file loop" in joined
    assert "proxy prosecution" in joined


def test_the_probe_is_deterministic():
    """Fresh keys per run and several lenses tiebreak on DID ordering, so a report that
    varies between runs is reporting a key-ordering dependency."""
    a, b = run_probe(), run_probe()
    assert a.passed == b.passed == a.total
    for x, y in zip(a.probes, b.probes):
        assert x.key == y.key and x.passed == y.passed


# --- where the ladder stops ---------------------------------------------------

def test_the_honest_complainant_climbs_alone_and_then_stops(report):
    """Registration through limited sanctions need no office. Collective sanctions and
    a designation need the compact, and the two loyal members are below both floors."""
    seam = report.by_key("seam")
    assert seam.metrics["rungs_climbed_alone"] == RUNGS_WITHOUT_OFFICE
    assert seam.metrics["rung_reached"] == "LimitedSanctions"
    assert seam.metrics["seam_refusals"] == SEAM_REFUSALS
    assert "distinct communities" in seam.metrics["collective_refusal"]


# --- the reading that separates burial from diligence -------------------------

def test_burial_prosecution_and_calm_no_longer_read_alike(report):
    arms = report.by_key("arms")
    assert arms.metrics["severities"] == ("Alert", "None", "None")
    assert arms.metrics["grievances"] == (1, 0, 0)


def test_every_other_dimension_is_inert_on_the_buried_archive(report):
    """The finding that made the non-pursuit reading necessary: silence, vacancy,
    convergence, direction, targeting, re-examination and deliberation all read a
    captured compact exactly as they read a calm one."""
    assert report.by_key("arms").metrics["every_other_dimension_inert_on_burial"]


def test_the_burial_signature_is_the_same_seats_closing_the_same_evidence(report):
    sig = report.by_key("signature")
    assert sig.metrics["ground"] == "ClosedUnresolved"
    assert sig.metrics["pursuits"] == 4 and sig.metrics["dismissals"] == 4
    assert sig.metrics["repeat_panelists"] >= MIN_REPEAT_SEATS


# --- the two rules the measurement produced -----------------------------------

def test_a_de_escalation_is_the_complainants_concession(report):
    """Neither a bystander nor the accused may walk the ladder back down; the
    complainant still may, or the rule would be a prohibition on descent."""
    de = report.by_key("deescalation")
    assert de.metrics["bystander_authorized"] == 0
    assert de.metrics["respondent_authorized"] == 0
    assert de.metrics["complainant_concession_authorized"] == 1


def test_a_closing_decision_reaches_forwards_only(report):
    r = report.by_key("retroactivity")
    assert r.metrics["retroactive_unauthorizations"] == 0
    assert r.metrics["transitions_before_the_closing"] == 3
    assert r.metrics["after_the_closing_refused"]


# --- the ladder turned on the complainant -------------------------------------

def test_the_counter_prosecution_is_legible(report):
    """Six enforcement acts against the member that filed the evidence, brought by the
    community it named. The reading fires and names the pair."""
    r = report.by_key("retaliation")
    assert r.metrics["against_reporters"] == 6 and r.metrics["acts"] == 7
    assert r.metrics["share"] > 0.5
    assert len(r.metrics["pairs"]) == 1
    assert r.metrics["severity"][0] == "Alert"


def test_the_standing_lens_still_cannot_see_it(report):
    """The whole reason the dimension had to exist: `enforcement_direction` buckets by
    standing, and every one of those six acts lands on a community holding full
    standing, so it reads as enforcement between equals."""
    down, up, level = report.by_key("retaliation") \
        .metrics["direction_downward_upward_level"]
    assert (down, up) == (0, 0) and level == 7


def test_the_proxy_prosecution_is_not_seen(report):
    """The reading's stated limit, measured rather than asserted. The same answer to
    the same grievance, brought by three allies the grievance never named, reads zero:
    the join is on the pair the grievance names, and a majority willing to prosecute
    its accuser is also willing to choose who signs. Reported rather than fixed,
    because the alternatives — anyone in the compact, anyone enforcing after a
    grievance — are readings of enforcement rather than of retaliation."""
    r = report.by_key("retaliation")
    assert r.metrics["proxy_against_reporters"] == 0
    assert r.metrics["proxy_seen"] is False


def test_the_same_volume_answering_nothing_stays_silent(report):
    """The control that differs on one axis. Six enforcement acts against the same
    member, at the same times, by the same communities, with no prior grievance for
    them to answer: the reading must not fire, or it would be a count of enforcement
    rather than a reading of retaliation."""
    r = report.by_key("retaliation")
    assert r.metrics["control_against_reporters"] == 0
    assert r.metrics["severity"][1] == "None"


# --- the fixture --------------------------------------------------------------

def test_the_fixture_is_a_complicit_majority():
    """Capture is legal by construction here: five of seven complicit against a quorum
    of four, with a panel of three seated by three of the five. No recusal rule touches
    any of it, which is the point."""
    p = CapturedParams()
    assert p.founding_size - p.loyal == 5
    assert p.quorum_threshold == 4 and p.loyal < p.quorum_threshold
