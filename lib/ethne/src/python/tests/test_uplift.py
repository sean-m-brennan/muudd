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

"""The three artifacts that cross to kith-covenant, as JSON: the identity handoff,
the coherence Concern, and the two lifecycle changes (fission and fusion)."""
from __future__ import annotations

import json

import pytest

import ethne as e
from ethne.monitor import cohesion_report, founding_candidate
from ethne.uplift import (
    coherence_concern,
    community_identity,
    concern_dict,
    fission,
    fusion,
    identity_dict,
)


def _identity(found, name):
    """A separately chartered polity, handed off as a signed identity."""
    p = found(3)
    return p, community_identity(p.archive, p.did, name, p.priv, p.pub, created_at=500)


def _concern_level_report(polity):
    roll = polity.roll(as_of=300)
    return cohesion_report(roll, founding_candidate([(roll[0], 0.9, None)]))


def test_community_identity_is_self_signed_and_matches_the_charter(polity):
    payload = identity_dict(
        community_identity(polity.archive, polity.did, "Ourania", polity.priv, polity.pub, created_at=500)
    )
    assert payload["payload"]["did"] == polity.did
    assert payload["payload"]["name"] == "Ourania"
    assert payload["signatures"], "the identity carries its own signature"


def test_identity_with_a_foreign_root_key_is_refused(polity):
    other_priv, other_pub, _did = e.generate_keypair()
    with pytest.raises(RuntimeError, match="does not match"):
        community_identity(polity.archive, polity.did, "Ourania", other_priv, other_pub, created_at=500)


def test_identity_needs_a_charter_in_this_log(polity):
    _priv, _pub, stranger = e.generate_keypair()
    with pytest.raises(ValueError, match="no charter"):
        community_identity(polity.archive, stranger, "Nowhere", polity.priv, polity.pub)


def test_no_concern_below_concern_level_erosion(polity):
    roll = polity.roll(as_of=300)
    healthy = cohesion_report(roll, founding_candidate([(d, 0.9, None) for d in roll]))
    assert healthy.erosion == "None"
    assert coherence_concern(healthy, polity.did, polity.founders[0], at=600) is None
    assert concern_dict(None) is None


def test_a_concern_crosses_only_at_concern_level(polity):
    report = _concern_level_report(polity)
    assert report.erosion == "Concern"

    record = concern_dict(coherence_concern(report, polity.did, polity.founders[0], at=600))
    payload = record["payload"]
    assert payload["kind"] == "Concern"
    assert payload["bearing"] == {"Other": "coherence-erosion"}
    assert payload["subject_community"] == polity.did
    assert payload["reporters"] == [polity.founders[0].did]
    # KC discounts a monitor-originated signal, so the perspective is explicit.
    assert payload["reporter_perspective"] == "ethne cohesion monitor"
    assert payload["ladder_rung"] is None


def test_fission_emits_two_links_and_a_dissolution(polity, found):
    _a, alpha = _identity(found, "Alpha")
    _b, beta = _identity(found, "Beta")

    outcome = fission(
        polity.archive, polity.did, alpha, beta, polity.priv,
        deciders=polity.founders[:2], rationale="irreconcilable readings", at=800,
    )

    successor_dids = [identity_dict(s)["payload"]["did"] for s in (alpha, beta)]
    for link_json, successor in zip((outcome.link_a, outcome.link_b), successor_dids):
        link = json.loads(link_json)["payload"]
        assert link["previous_did"] == polity.did
        assert link["new_did"] == successor
    dissolution = json.loads(outcome.dissolution)["payload"]
    assert dissolution["polity"] == polity.did
    assert dissolution["successors"] == successor_dids
    assert dissolution["rationale"] == "irreconcilable readings"
    # Emission, not appending: the log still holds only the charter.
    assert len(polity.archive) == 1


def test_fusion_merges_predecessors_from_one_log(found):
    """Both predecessors' charters live in a shared log, as a KC-side view would."""
    first = found(3)
    second = found(3)
    merged_log = e.Archive.load_json(
        json.dumps(
            json.loads(first.archive.dump_json()) + json.loads(second.archive.dump_json())
        )
    )
    # The merged polity is chartered in its own right before it is handed off.
    fused_holder = found(3)
    fused = community_identity(
        fused_holder.archive, fused_holder.did, "Merged",
        fused_holder.priv, fused_holder.pub, created_at=700,
    )

    outcome = fusion(
        merged_log,
        [
            (first.did, first.priv, first.founders[:2]),
            (second.did, second.priv, second.founders[:2]),
        ],
        fused,
        rationale="one people now",
        at=900,
    )

    assert identity_dict(outcome.fused)["payload"]["name"] == "Merged"
    assert len(outcome.links) == len(outcome.dissolutions) == 2
    for did, link_json, diss_json in zip(
        (first.did, second.did), outcome.links, outcome.dissolutions
    ):
        assert json.loads(link_json)["payload"]["previous_did"] == did
        diss = json.loads(diss_json)["payload"]
        assert diss["polity"] == did
        assert diss["successors"] == [fused_holder.did]


def test_fusion_needs_at_least_two_predecessors(polity, found):
    holder = found(3)
    fused = community_identity(holder.archive, holder.did, "Merged", holder.priv, holder.pub, created_at=700)
    with pytest.raises(RuntimeError, match="at least two"):
        fusion(polity.archive, [(polity.did, polity.priv, polity.founders[:2])], fused, at=900)


def test_fusion_refuses_a_repeated_predecessor(polity, found):
    holder = found(3)
    fused = community_identity(holder.archive, holder.did, "Merged", holder.priv, holder.pub, created_at=700)
    twice = [(polity.did, polity.priv, polity.founders[:2])] * 2
    with pytest.raises(RuntimeError, match="twice"):
        fusion(polity.archive, twice, fused, at=900)


def test_fission_rejects_a_non_identity_payload(polity, found):
    _a, alpha = _identity(found, "Alpha")
    with pytest.raises(ValueError, match="signed community identity"):
        fission(polity.archive, polity.did, alpha, "{}", polity.priv, at=800)
