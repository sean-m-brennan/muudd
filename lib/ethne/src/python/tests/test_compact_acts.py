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

"""Compact acts (D40) and a policy that acts on standing (D39), from Python.

The Rust suites pin the rules; this checks the surface a host drives: a majority
joins and sponsors, bearing down on another community needs more than a majority,
and a policy carries what a standing does.
"""
from __future__ import annotations

from ethne.authoring import (
    COMPACT_ACT_KINDS, amend_content_policy, compact_state, content_policy_state,
    decide_compact_act,
)

STANDING = {
    "operator_bound_required": False,
    "min_tier": 0,
    "daily_post_cap": None,
    "labels": {},
    "standing": {"Anathema": "Hide", "Denied": "Warn"},
}


def test_the_kinds_are_exported():
    assert COMPACT_ACT_KINDS == ("Join", "Admit", "Sponsor", "Refuse", "Anathematize", "Lift")


def test_a_majority_joins_and_sponsors(found):
    p = found(5)
    other = found(3)
    decide_compact_act(p.archive, p.did, "Join", "river", [other.did], p.founders[:3],
                       decided_at=200)
    decide_compact_act(p.archive, p.did, "Sponsor", "river", [other.did], p.founders[:3],
                       probation_days=90, decided_at=210)
    assert p.archive.authority_violations() == []
    acts = compact_state(p.archive, p.did, as_of=300)
    assert [a["act"]["kind"] for a in acts] == ["Join", "Sponsor"]
    assert acts[1]["act"]["probation_days"] == 90


def test_anathematizing_needs_more_than_a_majority(found):
    p = found(5)
    other = found(3)
    decide_compact_act(p.archive, p.did, "Anathematize", "river", [other.did], p.founders[:3],
                       decided_at=200)
    assert any("more than a majority" in why for _, why in p.archive.authority_violations())
    assert compact_state(p.archive, p.did, as_of=300) == []
    decide_compact_act(p.archive, p.did, "Anathematize", "river", [other.did], p.founders[:4],
                       decided_at=400)
    assert [a["act"]["kind"] for a in compact_state(p.archive, p.did, as_of=500)] == ["Anathematize"]


def test_a_policy_carries_what_a_standing_does(found):
    p = found(5)
    amend_content_policy(p.archive, p.did, STANDING, p.founders[:3], decided_at=200)
    assert content_policy_state(p.archive, p.did, as_of=300)["policy"] == STANDING
    weaker = dict(STANDING, standing={"Anathema": "Warn", "Denied": "Warn"})
    amend_content_policy(p.archive, p.did, weaker, p.founders[:3], decided_at=300)
    assert any("loosening" in why for _, why in p.archive.authority_violations())
