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

"""A polity's Stele profile (D42) and its border file (#11), from Python.

The Rust suites pin the rules; this checks the surface a host drives: the roll
sets a profile at a majority, a loosening needs more than a majority, nothing
crosses Stele's floors, and the exported file is the root key's signature over
what the roll decided.
"""
from __future__ import annotations

import json

import nacl.signing
import pytest

import ethne as e
from ethne.authoring import amend_stele_profile, stele_profile_state

STRICT = {
    "name": "acme-strict",
    "replicas": 3,
    "tier": 3,
    "require": ["gitleaks:probe", "osv-scanner:probe"],
    "ignore": [],
    "severity_floor": "low",
    "onset_days": 1,
    "half_life_days": 14,
    "team": "worst",
}


def test_a_polity_with_no_profile_is_under_stele_default(found):
    p = found(5)
    st = stele_profile_state(p.archive, p.did, as_of=300)
    assert st["profile"]["name"] == "stele-default"
    assert (st["profile"]["replicas"], st["profile"]["tier"]) == (2, 3)
    assert (st["set_by"], st["seq"]) == (None, 0)
    with pytest.raises(Exception):
        e.stele_profile(p.archive, p.did, p.priv, p.pub, as_of=300)


def test_a_majority_sets_a_profile_and_it_exports_signed(found):
    p = found(5)
    amend_stele_profile(p.archive, p.did, STRICT, p.founders[:3], decided_at=200)
    assert p.archive.authority_violations() == []
    st = stele_profile_state(p.archive, p.did, as_of=300)
    assert st["profile"] == STRICT and st["seq"] == 1 and st["set_by"]

    text = e.stele_profile(p.archive, p.did, p.priv, p.pub, as_of=300)
    outer = json.loads(text)
    body = json.loads(outer["body"])
    assert body["typename"] == "stele-profile"
    assert body["issuer"] == p.pub.hex()
    assert (body["seq"], body["set_by"]) == (1, st["set_by"])
    assert body["profile"] == STRICT
    nacl.signing.VerifyKey(p.pub).verify(
        b"stele-profile-v1|" + outer["body"].encode(), bytes.fromhex(outer["sig"]))


def test_a_profile_may_cap_a_repository_at_its_worst_dependency(found):
    """The supply-chain view: `deps` travels only when it caps, and dropping
    the cap is a loosening."""
    p = found(5)
    worst = dict(STRICT, deps="worst")
    amend_stele_profile(p.archive, p.did, worst, p.founders[:3], decided_at=200)
    assert p.archive.authority_violations() == []
    assert stele_profile_state(p.archive, p.did, as_of=300)["profile"] == worst
    body = json.loads(json.loads(e.stele_profile(p.archive, p.did, p.priv, p.pub,
                                                 as_of=300))["body"])
    assert body["profile"]["deps"] == "worst"
    amend_stele_profile(p.archive, p.did, STRICT, p.founders[:3], decided_at=300)
    assert any("loosening the Stele profile" in why for _, why in p.archive.authority_violations())


def test_loosening_needs_more_than_a_majority(found):
    p = found(5)
    amend_stele_profile(p.archive, p.did, STRICT, p.founders[:3], decided_at=200)
    looser = dict(STRICT, replicas=1)
    amend_stele_profile(p.archive, p.did, looser, p.founders[:3], decided_at=300)
    assert any("loosening the Stele profile" in why for _, why in p.archive.authority_violations())
    assert stele_profile_state(p.archive, p.did, as_of=400)["profile"] == STRICT

    amend_stele_profile(p.archive, p.did, looser, p.founders[:4], decided_at=500)
    st = stele_profile_state(p.archive, p.did, as_of=600)
    assert st["profile"] == looser and st["seq"] == 2


def test_no_roll_crosses_a_floor(found):
    p = found(5)
    amend_stele_profile(p.archive, p.did, dict(STRICT, replicas=0), p.founders, decided_at=200)
    assert any("Stele profile of" in why for _, why in p.archive.authority_violations())
    assert stele_profile_state(p.archive, p.did, as_of=300)["seq"] == 0
