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

"""The rendezvous office and the relay roster it feeds (D36), from Python.

The Rust suites pin the rules; this checks the surface a host actually drives: the
roll confers the office, the operator declares its endpoint, the roster comes out
as AutonomousTrust's own signed file, and revoking the office empties the next one.
"""
from __future__ import annotations

import hashlib
import json

import pytest

import ethne as e
from ethne import _core
from ethne.authoring import (
    Signer, confer_office, declare_relay_endpoint, revoke_office, withdraw_relay_endpoint,
)

UUID = "0f1e2d3c-4b5a-4968-8776-655443322110"


_B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def pub_of(did: str) -> bytes:
    """The raw ed25519 key inside a `did:key` (multibase base58btc, multicodec
    0xed01), decoded here rather than through ethne so the check is independent."""
    assert did.startswith("did:key:z")
    n = 0
    for ch in did[len("did:key:z"):]:
        n = n * 58 + _B58.index(ch)
    raw = n.to_bytes((n.bit_length() + 7) // 8, "big")
    assert raw[:2] == b"\xed\x01" and len(raw) == 34
    return raw[2:]


def seat(polity, member: Signer, at: int = 200) -> None:
    """Confer RendezvousOperator on `member` at a majority of the founders."""
    confer_office(polity.archive, polity.did, "RendezvousOperator", member,
                  polity.founders[:2], decided_at=at)


def test_the_roll_seats_an_operator_and_the_roster_pins_its_key(polity):
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "relay.example.org", 27790,
                           declared_at=300)
    polity.archive.check_authority()

    assert polity.archive.rendezvous_state(polity.did, 400) == [
        (op.did, UUID, "relay.example.org", 27790, 300, [])
    ]
    roster = e.rendezvous_roster(polity.archive, polity.did, polity.priv, polity.pub,
                                 seq=400, as_of=400)
    outer = json.loads(roster)
    assert set(outer) == {"body", "sig"}
    body = e.roster_dict(roster)
    assert body["typename"] == "at-relay-roster" and body["v"] == 1
    assert body["issuer"] == polity.pub.hex()
    assert body["seq"] == 400
    (hint,) = body["relays"]
    assert hint.startswith(f"relay://{UUID}:") and hint.endswith("@relay.example.org:27790")


def test_the_pin_is_sha256_of_the_operators_own_key(polity):
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "relay.example.org", 27790,
                           declared_at=300)
    body = e.roster_dict(e.rendezvous_roster(polity.archive, polity.did, polity.priv,
                                             polity.pub, seq=1, as_of=400))
    fp = hashlib.sha256(pub_of(op.did)).hexdigest()[:32]
    assert body["relays"] == [f"relay://{UUID}:{fp}@relay.example.org:27790"]


def test_revoking_the_office_empties_the_next_roster(polity):
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "relay.example.org", 27790,
                           declared_at=300)
    revoke_office(polity.archive, polity.did, "RendezvousOperator", op.did,
                  polity.founders[:2], decided_at=500)
    polity.archive.check_authority()
    before = e.roster_dict(e.rendezvous_roster(polity.archive, polity.did, polity.priv,
                                               polity.pub, seq=450, as_of=450))
    after = e.roster_dict(e.rendezvous_roster(polity.archive, polity.did, polity.priv,
                                              polity.pub, seq=600, as_of=600))
    assert len(before["relays"]) == 1
    assert after["relays"] == [], "an empty roster is how the polity says it runs none"


def test_a_member_without_the_office_is_refused(polity):
    declare_relay_endpoint(polity.archive, polity.did, polity.founders[1], UUID,
                           "relay.example.org", 27790, declared_at=300)
    with pytest.raises(RuntimeError, match="without holding RendezvousOperator"):
        polity.archive.check_authority()
    assert polity.archive.rendezvous_state(polity.did, 400) == []


def test_a_withdrawal_drops_the_relay(polity):
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "relay.example.org", 27790,
                           declared_at=300)
    withdraw_relay_endpoint(polity.archive, polity.did, op, UUID, declared_at=400)
    polity.archive.check_authority()
    assert polity.archive.rendezvous_state(polity.did, 500) == []


def test_the_office_is_named():
    assert "RendezvousOperator" in e.OFFICES


def test_a_root_key_that_is_not_the_politys_is_refused(polity):
    stranger_priv, stranger_pub, _ = _core.generate_ed25519_keypair()
    with pytest.raises(RuntimeError, match="does not match the supplied root key"):
        e.rendezvous_roster(polity.archive, polity.did, stranger_priv, stranger_pub, seq=1)


def test_a_hub_names_its_areas_in_the_state_and_the_roster(polity):
    """D38: a rendezvous relay may also be an area hub, for geohash areas it lists."""
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "hub.example.org", 27790,
                           declared_at=300, areas=["u4pr", "gcpv"])
    polity.archive.check_authority()
    assert polity.archive.rendezvous_state(polity.did, 400) == [
        (op.did, UUID, "hub.example.org", 27790, 300, ["u4pr", "gcpv"])
    ]
    body = e.roster_dict(e.rendezvous_roster(polity.archive, polity.did, polity.priv,
                                             polity.pub, seq=1, as_of=400))
    (hint,) = body["relays"]
    assert body["areas"] == {hint: ["u4pr", "gcpv"]}


def test_a_hub_with_an_area_no_hub_could_serve_is_refused(polity):
    op = polity.founders[2]
    seat(polity, op)
    declare_relay_endpoint(polity.archive, polity.did, op, UUID, "hub.example.org", 27790,
                           declared_at=300, areas=["u4pa"])
    with pytest.raises(Exception):
        polity.archive.check_authority()
    assert polity.archive.rendezvous_state(polity.did, 400) == []
