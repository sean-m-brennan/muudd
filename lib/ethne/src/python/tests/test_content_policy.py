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

"""A polity's content policy and the Moderator office (D37), from Python.

The Rust suites pin the rules; this checks the surface a host drives: the roll sets
a policy at a majority, a loosening needs more than a majority, and the office puts
a member on the moderator list.
"""
from __future__ import annotations

import ethne as e
from ethne.authoring import (
    amend_content_policy, confer_office, content_policy_state, revoke_office,
)

STRICT = {
    "operator_bound_required": True,
    "min_tier": 2,
    "daily_post_cap": 5,
    "labels": {"nsfw": "Hide", "spoiler": "Warn"},
}


def test_the_names_are_exported():
    assert "Moderator" in e.OFFICES
    assert e.LABEL_ACTIONS == ("Allow", "Warn", "Hide")


def test_a_majority_sets_a_policy(found):
    p = found(5)
    amend_content_policy(p.archive, p.did, STRICT, p.founders[:3], decided_at=200)
    st = content_policy_state(p.archive, p.did, as_of=300)
    assert st["policy"] == STRICT
    assert st["set_by"] is not None
    assert st["moderators"] == []
    assert p.archive.authority_violations() == []


def test_loosening_needs_more_than_a_majority(found):
    p = found(5)
    amend_content_policy(p.archive, p.did, STRICT, p.founders[:3], decided_at=200)
    looser = dict(STRICT, labels={"nsfw": "Warn", "spoiler": "Warn"})
    amend_content_policy(p.archive, p.did, looser, p.founders[:3], decided_at=300)
    assert any("loosening" in why for _, why in p.archive.authority_violations())
    assert content_policy_state(p.archive, p.did, as_of=400)["policy"] == STRICT

    amend_content_policy(p.archive, p.did, looser, p.founders[:4], decided_at=500)
    assert content_policy_state(p.archive, p.did, as_of=600)["policy"] == looser


def test_the_office_makes_a_moderator_and_revoking_it_unmakes_one(found):
    p = found(5)
    mod = p.founders[4]
    confer_office(p.archive, p.did, "Moderator", mod, p.founders[:3], decided_at=200)
    assert content_policy_state(p.archive, p.did, as_of=250)["moderators"] == [mod.did]
    revoke_office(p.archive, p.did, "Moderator", mod.did, p.founders[:3], decided_at=300)
    assert content_policy_state(p.archive, p.did, as_of=350)["moderators"] == []
