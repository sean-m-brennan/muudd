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
from ethne.monitor import Thresholds, cohesion_report, founding_candidate


def _candidate(dids, reputation=0.9):
    return founding_candidate([(d, reputation, None) for d in dids])


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
    faded = founding_candidate([(d, 0.2, None) for d in roll])
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
        [(roll[0], 0.75, guardian)] + [(d, 0.5, None) for d in roll[1:]],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    assert candidate.members[0] == (roll[0], 0.75, guardian)
    assert candidate.member_reputation(roll[1]) == 0.5
    assert candidate.member_reputation(e.generate_keypair()[2]) is None
    assert candidate.substrate_name == "tradition"
    assert candidate.substrate_anchor is None
