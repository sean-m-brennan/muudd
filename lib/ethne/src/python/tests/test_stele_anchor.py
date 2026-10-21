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

"""The group anchor (D43, border artifact #12), from Python: the polity root signs
the AT group its work is verified in, and every node it lists has consented under
the key it is listed with. The Rust suite pins the bytes."""
from __future__ import annotations

import json

import pytest

import ethne as e
from ethne import _core

UUIDS = (
    "0f1e2d3c-4b5a-4968-8776-655443322110",
    "1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9",
    "2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a",
)
_B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def pub_of(did: str) -> bytes:
    """The raw ed25519 key inside a `did:key` (as test_stele_team decodes it)."""
    n = 0
    for ch in did[len("did:key:z"):]:
        n = n * 58 + _B58.index(ch)
    raw = n.to_bytes((n.bit_length() + 7) // 8, "big")
    assert raw[:2] == b"\xed\x01" and len(raw) == 34
    return raw[2:]

GROUP = "3c4d5e6f-7081-4293-a4b5-c6d7e8f90a1b"


def anchor_members(polity, tiers=(3, 3, 1)):
    """Each founder's key standing in for its AT node's key (D33)."""
    out = []
    for signer, uuid, tier in zip(polity.founders, UUIDS, tiers):
        key = pub_of(signer.did).hex()
        msg = e.stele_anchor_consent_message(polity.pub.hex(), GROUP, uuid, key)
        out.append((uuid, key, tier, bytes(_core.sign_detached(signer.priv, msg)).hex()))
    return out


CHECKPOINTS = [("", 1, "11" * 32, 0, 6), ("", 2, "22" * 32, 0, 7)]


def test_the_root_signs_the_group_and_its_consenting_nodes(polity):
    text = e.stele_group_anchor(polity.archive, polity.did, GROUP, anchor_members(polity),
                                CHECKPOINTS, polity.priv, polity.pub, 3)
    outer = json.loads(text)
    body = json.loads(outer["body"])
    assert body["typename"] == "stele-group-anchor" and body["seq"] == 3
    assert body["issuer"] == polity.pub.hex() and body["group_uuid"] == GROUP
    assert [m["at_uuid"] for m in body["members"]] == sorted(UUIDS)
    assert [c["epoch"] for c in body["checkpoints"]] == [1, 2]
    import nacl.signing
    nacl.signing.VerifyKey(polity.pub).verify(
        b"stele-anchor-v1|" + outer["body"].encode(), bytes.fromhex(outer["sig"]))


def test_a_node_that_did_not_consent_cannot_be_listed(polity):
    listed = anchor_members(polity)
    uuid, key, tier, _ = listed[0]
    other = polity.founders[1]
    msg = e.stele_anchor_consent_message(polity.pub.hex(), GROUP, uuid, key)
    listed[0] = (uuid, key, tier, bytes(_core.sign_detached(other.priv, msg)).hex())
    with pytest.raises(Exception):
        e.stele_group_anchor(polity.archive, polity.did, GROUP, listed, CHECKPOINTS,
                             polity.priv, polity.pub, 1)
    with pytest.raises(Exception):   # a tier AT does not have
        e.stele_group_anchor(polity.archive, polity.did, GROUP,
                             anchor_members(polity, tiers=(3, 3, 7)), CHECKPOINTS,
                             polity.priv, polity.pub, 1)
