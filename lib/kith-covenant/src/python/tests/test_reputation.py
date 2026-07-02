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

"""Pytest suite for the kith_covenant reputation-lookup surface."""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, ReputationView, generate_keypair, reputation_of,
    run_two_community_ceremony,
)


ALPHA_MOVEMENTS = ["a1", "a2", "a3", "a4", "a5"]
BETA_MOVEMENTS = ["b1", "b2", "b3", "b4", "b5"]


@pytest.fixture
def ceremony_archive():
    """Run a fresh two-community ceremony and return (archive, alpha_did, beta_did)."""
    result = run_two_community_ceremony(
        "Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS,
        compact_label="test-compact",
    )
    archive = Archive.load_json(result.archive_json)
    return archive, result.alpha_did, result.beta_did


def test_reputation_of_alpha_in_compact(ceremony_archive):
    archive, alpha, beta = ceremony_archive
    view = reputation_of(archive, alpha)
    assert isinstance(view, ReputationView)
    assert view.community_did == alpha
    assert view.standing == "InCompact"
    assert len(view.intra_rites) == 1
    assert len(view.inter_admissions) == 1
    assert view.inter_admissions[0].counterparties == (beta,)
    assert not view.inter_nonadmissions


def test_reputation_of_beta_in_compact(ceremony_archive):
    archive, alpha, beta = ceremony_archive
    view = reputation_of(archive, beta)
    assert view.standing == "InCompact"
    assert view.inter_admissions[0].counterparties == (alpha,)


def test_unknown_for_unrelated_did(ceremony_archive):
    archive, _, _ = ceremony_archive
    _, _, stranger_did = generate_keypair()
    view = reputation_of(archive, stranger_did)
    assert view.standing == "Unknown"
    assert not view.intra_rites
    assert not view.inter_admissions
    assert not view.inter_nonadmissions
    assert view.identity_cid is None


def test_view_is_stable_through_json_roundtrip(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    before = reputation_of(archive, alpha)
    reloaded = Archive.load_json(archive.dump_json())
    after = reputation_of(reloaded, alpha)
    assert before == after


def test_as_of_before_genesis_returns_unknown(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    view = reputation_of(archive, alpha, as_of=0)
    assert view.standing == "Unknown"


def test_invalid_did_raises_value_error(ceremony_archive):
    archive, _, _ = ceremony_archive
    with pytest.raises(ValueError):
        reputation_of(archive, "not-a-did")
