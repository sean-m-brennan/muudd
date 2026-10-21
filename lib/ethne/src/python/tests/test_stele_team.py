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

"""The release office and the Stele files it feeds (D41), from Python.

The Rust suites pin the rules and the bytes; this checks the surface a team's host
drives: the roll confers ReleaseOfficer, the officer authorizes a commit, and the
team roster and the release authorization come out as Stele's signed files.
"""
from __future__ import annotations

import json

import pytest

import ethne as e
from ethne import _core
from ethne.authoring import Signer, authorize_release, confer_office, revoke_office

UUIDS = (
    "0f1e2d3c-4b5a-4968-8776-655443322110",
    "1a2b3c4d-5e6f-4071-8293-a4b5c6d7e8f9",
    "2b3c4d5e-6f70-4182-93a4-b5c6d7e8f90a",
)
URL = "http://localhost:3000/stele/demo.git"
COMMIT = "c3cd66ac5a89e0d1f2a3b4c5d6e7f8091a2b3c4d"


_B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def pub_of(did: str) -> bytes:
    """The raw ed25519 key inside a `did:key`, as test_rendezvous decodes it."""
    n = 0
    for ch in did[len("did:key:z"):]:
        n = n * 58 + _B58.index(ch)
    raw = n.to_bytes((n.bit_length() + 7) // 8, "big")
    assert raw[:2] == b"\xed\x01" and len(raw) == 34
    return raw[2:]


def seat(polity, member: Signer, at: int = 200) -> None:
    confer_office(polity.archive, polity.did, "ReleaseOfficer", member,
                  polity.founders[:2], decided_at=at)


def members(polity):
    """Each founder listed with its node's consent. A founder's DID key stands in
    for its AT node key here, which is what an AT member's DID is (D33)."""
    out = []
    for signer, uuid in zip(polity.founders, UUIDS):
        msg = e.stele_consent_message(polity.pub.hex(), signer.did, uuid)
        out.append((signer.did, uuid, bytes(_core.sign_detached(signer.priv, msg)).hex()))
    return out


def body_of(text: str) -> dict:
    return json.loads(json.loads(text)["body"])


def test_an_officer_releases_and_the_release_is_exported_under_its_key(polity):
    officer = polity.founders[2]
    seat(polity, officer)
    cid = authorize_release(polity.archive, polity.did, officer, "demo", URL, COMMIT,
                            built_by=polity.founders[1].did, authorized_at=300)
    polity.archive.check_authority()

    officers, releases = polity.archive.release_state(polity.did, 400)
    assert officers == [officer.did]
    assert releases == [(cid, officer.did, "demo", URL, COMMIT, polity.founders[1].did, 300)]

    text = e.stele_release_authorization(polity.archive, polity.did, cid,
                                         officer.priv, pub_of(officer.did), as_of=400)
    body = body_of(text)
    assert body["typename"] == "stele-release"
    assert body["officer"] == officer.did and body["officer_key"] == pub_of(officer.did).hex()
    assert body["issuer"] == polity.pub.hex()
    assert (body["url"], body["commit"], body["authorized_at"]) == (URL, COMMIT, 300)

    # Only the officer's own key exports it, and only an accepted release.
    other = polity.founders[0]
    with pytest.raises(Exception):
        e.stele_release_authorization(polity.archive, polity.did, cid, other.priv, pub_of(other.did), 400)
    with pytest.raises(Exception):
        e.stele_release_authorization(polity.archive, polity.did, "bafy-nope",
                                      officer.priv, pub_of(officer.did), 400)


def test_a_member_without_the_office_releases_nothing(polity):
    authorize_release(polity.archive, polity.did, polity.founders[1], "demo", URL, COMMIT,
                      authorized_at=300)
    with pytest.raises(Exception):
        polity.archive.check_authority()
    assert polity.archive.release_state(polity.did, 400) == ([], [])


def test_the_team_roster_carries_consents_officers_and_the_root_signature(polity):
    officer = polity.founders[2]
    seat(polity, officer)
    text = e.stele_team_roster(polity.archive, polity.did, members(polity),
                               polity.priv, polity.pub, seq=7, as_of=400)
    outer = json.loads(text)
    assert set(outer) == {"body", "sig"}
    body = body_of(text)
    assert body["typename"] == "stele-team" and body["v"] == 1
    assert body["polity"] == polity.did and body["issuer"] == polity.pub.hex()
    assert body["seq"] == 7
    assert body["release_officers"] == [officer.did]
    assert sorted(m["at_uuid"] for m in body["members"]) == sorted(UUIDS)


def test_a_revoked_officer_leaves_the_roster_but_not_the_release(polity):
    officer = polity.founders[2]
    seat(polity, officer)
    authorize_release(polity.archive, polity.did, officer, "demo", URL, COMMIT,
                      authorized_at=300)
    revoke_office(polity.archive, polity.did, "ReleaseOfficer", officer.did,
                  polity.founders[:2], decided_at=500)
    officers, releases = polity.archive.release_state(polity.did, 600)
    assert officers == [] and len(releases) == 1
    body = body_of(e.stele_team_roster(polity.archive, polity.did, members(polity),
                                       polity.priv, polity.pub, seq=8, as_of=600))
    assert body["release_officers"] == []


def test_an_officer_with_no_node_cannot_be_rostered(polity):
    seat(polity, polity.founders[2])
    with pytest.raises(Exception):
        e.stele_team_roster(polity.archive, polity.did, members(polity)[:2],
                            polity.priv, polity.pub, seq=1, as_of=400)


def test_release_officer_is_a_named_office():
    from ethne.polity import OFFICES
    assert "ReleaseOfficer" in OFFICES
