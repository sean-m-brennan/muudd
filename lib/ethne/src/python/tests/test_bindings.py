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

"""The bindings themselves: keys, content addressing, the log, JSON round-trip.

The charter logic underneath is covered by `cargo test --workspace`; these tests
check that the Python surface loads and that its plumbing is faithful.
"""
from __future__ import annotations

import json

import pytest

import ethne as e
from ethne import _core
from ethne.authoring import Signer


def test_core_version_and_alg_constant():
    assert isinstance(_core.__version__, str)
    assert _core.ALG_ED25519 == "Ed25519"


def test_generate_keypair_shape():
    priv, pub, did = e.generate_keypair()
    assert isinstance(priv, bytes) and len(priv) == 32
    assert isinstance(pub, bytes) and len(pub) == 32
    assert did.startswith("did:key:z6Mk")


def test_generate_keypair_is_unique():
    assert e.generate_keypair()[2] != e.generate_keypair()[2]


def test_signer_generate_and_repr_hides_key():
    s = Signer.generate()
    assert s.did.startswith("did:key:")
    assert len(s.priv) == 32
    assert s.priv.hex() not in repr(s)
    assert s.did in repr(s)


def test_cid_of_is_deterministic_and_prefixed():
    first = e.cid_of(b"substrate")
    assert first == e.cid_of(b"substrate")
    assert first != e.cid_of(b"Substrate")
    assert first.startswith("b3:")


def test_majority_helper():
    assert [e.majority(n) for n in (1, 2, 3, 4, 5)] == [1, 2, 2, 3, 3]


def test_empty_archive():
    arch = e.Archive()
    assert len(arch) == 0
    assert arch.cids() == []
    assert arch.entries() == []
    assert arch.authority_violations() == []
    arch.verify_all()
    arch.check_authority()


def test_entries_report_cid_and_kind(polity):
    entries = polity.archive.entries()
    assert entries == [(polity.archive.cids()[0], "PolityChartered")]
    assert polity.archive.contains(entries[0][0])


def test_dump_json_round_trip_preserves_cids(polity):
    dumped = polity.archive.dump_json()
    parsed = json.loads(dumped)
    assert len(parsed) == 1 and "PolityChartered" in parsed[0]

    restored = e.Archive.load_json(dumped)
    assert len(restored) == len(polity.archive)
    assert restored.cids() == polity.archive.cids()
    restored.verify_all()
    restored.check_authority()


def test_load_json_rejects_garbage():
    with pytest.raises(ValueError):
        e.Archive.load_json("{not json}")


def test_bad_did_is_a_value_error():
    arch = e.Archive()
    with pytest.raises(ValueError):
        arch.charter_state("not-a-did")


def test_mismatched_signer_lists_are_a_value_error(polity):
    with pytest.raises(ValueError, match="same length"):
        polity.archive.expel_member(
            polity.did, polity.founders[2].did, [polity.founders[0].did], []
        )


def test_default_thresholds_exposes_the_rust_values():
    assert set(e.DEFAULT_THRESHOLDS) == {
        "reputation_floor",
        "attrition_notice",
        "attrition_concern",
        "mean_reputation_notice",
        "mean_reputation_concern",
        "breach_notice",
        "breach_concern",
    }
    # The dataclass defaults are those same values, not a second copy.
    assert e.Thresholds().attrition_concern == e.DEFAULT_THRESHOLDS["attrition_concern"]
