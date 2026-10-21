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

"""Dashboard prototype: the seed commons exercises every capture-signal lens,
and the read-only JSON API is a faithful projection of it.

The seed tests need only the kith_covenant extension. The API tests need
fastapi + httpx (the dashboard's extra deps) and skip cleanly without them, so
the suite still passes in an env that has not installed the dashboard extras.
"""
from __future__ import annotations

import pytest

from dashboard.seed import build_commons
from kith_covenant import (
    ceremony_cost_decay,
    composition_graph,
    feature_concern_profile,
    rejection_pattern,
    reputation_of,
    sponsor_stake,
)


@pytest.fixture(scope="module")
def commons():
    return build_commons()


# --- the seed exercises every lens with real, verified records --------------

def test_seed_builds_verifies_and_names_six_distinct_actors(commons):
    assert commons.size > 0
    commons.archive.verify_all()      # crypto integrity
    commons.archive.check_authority()  # principal-binding + scope + quorum
    dids = {commons.hub_did, commons.drifting_did, commons.healthy_did,
            commons.gatekeeper_did, commons.reaffirmer_did, commons.designated_did}
    assert len(dids) == 6
    assert all(d.startswith("did:key:") for d in dids)


def test_composition_concentrates_on_the_hub(commons):
    by = dict(composition_graph(commons.archive).by_concentration)
    assert by.get(commons.hub_did) == 2


def test_sponsor_stake_reports_half_tainted(commons):
    s = sponsor_stake(commons.archive, commons.hub_did)
    assert len(s.sponsees) == 2
    assert s.taint_rate == pytest.approx(0.5)


def test_feature_concern_profile_lists_two_features(commons):
    feats = dict(feature_concern_profile(commons.archive, commons.drifting_did).per_feature)
    assert feats == {"EnemyLoveOperational": 1, "NonUltimacyOfCompact": 1}


def test_rejection_pattern_concentrates_on_the_gatekeeper(commons):
    by = dict(rejection_pattern(commons.archive).by_concentration)
    assert by.get(commons.gatekeeper_did) == 2


def test_ceremony_cost_decay_shows_the_hollowing_trend(commons):
    d = ceremony_cost_decay(commons.archive, commons.reaffirmer_did)
    assert d.rite_count == 3
    assert (d.first_witness_count, d.last_witness_count) == (5, 1)
    assert d.witness_count_declined is True


def test_standing_derivation(commons):
    assert reputation_of(commons.archive, commons.designated_did).standing == "Anathema"
    # a rejected applicant does not derive the same standing as an anathema
    assert reputation_of(commons.archive, commons.gatekeeper_did).standing != "Anathema"


# --- the JSON API is a faithful projection (needs fastapi + httpx) ----------

@pytest.fixture(scope="module")
def client():
    pytest.importorskip("fastapi", reason="dashboard extra: fastapi")
    pytest.importorskip("httpx", reason="fastapi TestClient needs httpx")
    from fastapi.testclient import TestClient

    from dashboard.app import app
    with TestClient(app) as c:
        yield c


def test_meta_lists_all_roles(client):
    r = client.get("/api/meta")
    assert r.status_code == 200
    body = r.json()
    assert body["size"] > 0
    roles = {a["role"] for a in body["actors"]}
    assert {"hub", "drifting", "healthy", "gatekeeper", "reaffirmer", "designated"} <= roles


@pytest.mark.parametrize("path", [
    "/api/composition",
    "/api/sponsor-stake",
    "/api/feature-concern",
    "/api/rejection-pattern",
    "/api/ceremony-cost-decay",
    "/api/reputation",
])
def test_endpoints_return_json_objects(client, path):
    r = client.get(path)
    assert r.status_code == 200
    assert isinstance(r.json(), dict)


def test_index_page_is_served(client):
    r = client.get("/")
    assert r.status_code == 200
    assert "kith-covenant" in r.text


def test_did_query_param_switches_the_projection(client):
    by_role = {a["role"]: a["did"] for a in client.get("/api/meta").json()["actors"]}
    designated = client.get(f"/api/reputation?did={by_role['designated']}").json()
    gatekeeper = client.get(f"/api/reputation?did={by_role['gatekeeper']}").json()
    assert designated["standing"] == "Anathema"
    assert gatekeeper["standing"] != designated["standing"]
