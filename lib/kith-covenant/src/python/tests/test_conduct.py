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

"""Pytest suite for the conduct / sponsorship capture-signal surface."""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, composition_graph, demo_capture_scenario, feature_concern_profile,
    reputation_of, sponsor_stake,
)


@pytest.fixture
def scenario():
    """A fully-signed demo archive plus the DIDs needed to query it."""
    s = demo_capture_scenario()
    archive = Archive.load_json(s.archive_json)
    archive.verify_all()  # crypto-verifies including the new record kinds
    return archive, s


def test_conduct_projects_onto_drifting_entrant(scenario):
    archive, s = scenario
    view = reputation_of(archive, s.drifting_entrant_did)
    assert len(view.conduct) == 2
    assert {c.kind for c in view.conduct} == {"Concern"}
    assert all(c.ladder_rung is None for c in view.conduct), "Concern is pre-indictment"
    bearings = {c.bearing for c in view.conduct}
    assert bearings == {
        "AntiSpiral(NonUltimacyOfCompact)",
        "AntiSpiral(EnemyLoveOperational)",
    }


def test_healthy_entrant_has_only_commendation(scenario):
    archive, s = scenario
    view = reputation_of(archive, s.healthy_entrant_did)
    assert len(view.conduct) == 1
    assert view.conduct[0].kind == "Commendation"


def test_sponsorships_project_to_both_endpoints(scenario):
    archive, s = scenario
    hub = reputation_of(archive, s.hub_did)
    assert len(hub.sponsorships_given) == 2
    assert not hub.sponsorships_received

    entrant = reputation_of(archive, s.drifting_entrant_did)
    assert len(entrant.sponsorships_received) == 1
    assert entrant.sponsorships_received[0].sponsor_community == s.hub_did


def test_composition_graph_flags_hub_concentration(scenario):
    archive, s = scenario
    graph = composition_graph(archive)
    assert len(graph.edges) == 2
    top_sponsor, out_degree = graph.by_concentration[0]
    assert top_sponsor == s.hub_did
    assert out_degree == 2


def test_sponsor_stake_flags_tainted_sponsee(scenario):
    archive, s = scenario
    report = sponsor_stake(archive, s.hub_did)
    assert report.total_sponsees == 2
    # Only the drifting entrant accrued adverse conduct in probation.
    assert report.tainted_sponsees == 1
    assert report.taint_rate == pytest.approx(0.5)
    tainted = [o for o in report.sponsees if o.adverse_conduct]
    assert len(tainted) == 1
    assert tainted[0].entrant == s.drifting_entrant_did
    assert len(tainted[0].adverse_conduct) == 2


def test_feature_concern_profile_is_the_substrate_shadow(scenario):
    archive, s = scenario
    profile = feature_concern_profile(archive, s.drifting_entrant_did)
    per_feature = dict(profile.per_feature)
    assert per_feature.get("NonUltimacyOfCompact") == 1
    assert per_feature.get("EnemyLoveOperational") == 1
    assert profile.non_feature_adverse == 0
    assert len(profile.evidence) == 2
    assert profile.perspectives == ("charter-originalist observer",)


def test_healthy_entrant_has_clean_profile(scenario):
    archive, s = scenario
    profile = feature_concern_profile(archive, s.healthy_entrant_did)
    # A commendation is not adverse, so the profile is empty.
    assert profile.per_feature == ()
    assert profile.non_feature_adverse == 0
    assert profile.evidence == ()
