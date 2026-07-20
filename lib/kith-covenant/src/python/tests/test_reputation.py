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
    run_two_community_ceremony, rejection_pattern, ceremony_cost_decay,
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


# --- v0 jester lenses (binding surface + degenerate paths) ------------------
# The rich multi-rejection / declining-witness paths are covered exhaustively
# by the Rust unit tests; the Python-reachable archive (a single two-community
# ceremony) exercises the binding surface and the empty/single-point paths, and
# cross-checks the ceremony-cost lens against the ReputationView projection.


def test_rejection_pattern_inert_on_admission_only_archive(ceremony_archive):
    archive, _, _ = ceremony_archive
    p = rejection_pattern(archive)
    assert p.total_rejections == 0
    assert p.events == ()
    assert p.by_rejector == ()
    assert p.by_presenting == ()
    assert p.by_feature == ()
    assert p.by_concentration == ()
    assert p.perspectives == ()


def test_ceremony_cost_decay_single_rite_matches_projection(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    d = ceremony_cost_decay(archive, alpha)
    assert d.rite_count == 1
    assert d.gaps_seconds == ()
    assert d.witness_count_declined is False
    assert d.latest_gap_widening is None
    assert d.seconds_since_last is not None and d.seconds_since_last >= 0
    assert d.first_witness_count == d.last_witness_count
    # The lens and the ReputationView must agree on the same rite.
    view = reputation_of(archive, alpha)
    assert len(view.intra_rites) == 1
    assert d.rites[0][0] == view.intra_rites[0].record_cid
    assert d.rites[0][2] == view.intra_rites[0].witness_count


def test_ceremony_cost_decay_empty_for_stranger(ceremony_archive):
    archive, _, _ = ceremony_archive
    _, _, stranger = generate_keypair()
    d = ceremony_cost_decay(archive, stranger)
    assert d.rite_count == 0
    assert d.rites == ()
    assert d.gaps_seconds == ()
    assert d.first_witness_count is None
    assert d.latest_gap_widening is None
    assert d.seconds_since_last is None


def test_ceremony_cost_decay_as_of_before_genesis_is_empty(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    d = ceremony_cost_decay(archive, alpha, as_of=0)
    assert d.rite_count == 0


def test_jester_lens_invalid_did_raises(ceremony_archive):
    archive, _, _ = ceremony_archive
    with pytest.raises(ValueError):
        ceremony_cost_decay(archive, "not-a-did")
