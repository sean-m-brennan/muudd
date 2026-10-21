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

"""§7 commons governance through the Python surface. The exhaustive rule-by-rule
coverage lives in the Rust kc-archive authority and kc-reputation projection
tests; these confirm the bindings and facades wire the whole chain (author ->
verify_all -> check_authority -> projection/gate query)."""
import pytest

from kith_covenant import (
    Archive, new_identity,
    register_commons, record_usage, apply_sanction,
    reputation_of, commons_level, free_rider_signals, now_seconds,
    COMMONS_KINDS, SANCTION_LEVELS,
)


def _fixture():
    """Three appropriators A/B/C of one registered commons; C is the subject."""
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    c = new_identity(arch, "C", t0)
    commons = register_commons(
        arch, "Communication", "relay", [a.did, b.did, c.did],
        registrars=[a, b, c], registered_at=t0,
    )
    return arch, t0, a, b, c, commons


def test_constants_are_the_expected_sets():
    assert set(COMMONS_KINDS) == {
        "LifeSupport", "Communication", "Navigation", "KnowledgeArchive", "Rescue",
    }
    assert SANCTION_LEVELS == ("None", "Monitoring", "Warning", "Restriction", "Exclusion")


def test_register_usage_and_free_rider_ranking():
    arch, t0, a, b, c, commons = _fixture()
    arch.verify_all()
    arch.check_authority()
    record_usage(arch, commons, b, 800, 100, recorded_at=t0 + 10)
    record_usage(arch, commons, b, 400, 100, recorded_at=t0 + 20)
    record_usage(arch, commons, c, 100, 500, recorded_at=t0 + 10)
    arch.verify_all()
    arch.check_authority()
    report = free_rider_signals(arch, commons, as_of=t0 + 100)
    assert report.total_consumed == 1300
    assert report.total_contributed == 700
    assert report.by_net_draw == ((b.did, 1000),)
    b_tally = next(t for t in report.tallies if t.community == b.did)
    assert b_tally.record_count == 2


def test_graduated_sanction_projects_and_gates():
    arch, t0, a, b, c, commons = _fixture()
    apply_sanction(arch, commons, c.did, "None", "Monitoring", [a], decided_at=t0 + 100)
    apply_sanction(arch, commons, c.did, "Monitoring", "Warning", [a], decided_at=t0 + 200)
    apply_sanction(arch, commons, c.did, "Warning", "Restriction", [a, b],
                   decision_class="SecondClass", decided_at=t0 + 300)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, c.did, as_of=t0 + 500)
    assert len(v.commons_sanctions) == 1
    assert v.commons_sanctions[0].current_level == "Restriction"
    assert v.commons_sanctions[0].kind == "Communication"
    assert commons_level(arch, c.did, commons, as_of=t0 + 500) == "Restriction"
    assert commons_level(arch, a.did, commons, as_of=t0 + 500) == "None"


def test_restoration_drops_sanction_from_projection():
    arch, t0, a, b, c, commons = _fixture()
    apply_sanction(arch, commons, c.did, "None", "Monitoring", [a], decided_at=t0 + 100)
    apply_sanction(arch, commons, c.did, "Monitoring", "None", [a], decided_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, c.did, as_of=t0 + 500)
    assert not v.commons_sanctions
    assert commons_level(arch, c.did, commons, as_of=t0 + 500) == "None"


def test_single_community_registration_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    solo = new_identity(arch, "Solo", t0)
    register_commons(arch, "LifeSupport", "loop", [solo.did],
                     registrars=[solo], registered_at=t0)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_sanction_skipping_a_rung_is_unauthorized():
    arch, t0, a, b, c, commons = _fixture()
    apply_sanction(arch, commons, c.did, "None", "Restriction", [a, b],
                   decision_class="SecondClass", decided_at=t0 + 100)
    assert any("escalate one step" in r for _cid, r in arch.authority_violations())


def test_second_class_restriction_needs_quorum():
    arch, t0, a, b, c, commons = _fixture()
    apply_sanction(arch, commons, c.did, "None", "Monitoring", [a], decided_at=t0 + 100)
    apply_sanction(arch, commons, c.did, "Monitoring", "Warning", [a], decided_at=t0 + 200)
    apply_sanction(arch, commons, c.did, "Warning", "Restriction", [a],
                   decision_class="SecondClass", decided_at=t0 + 300)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_usage_by_non_appropriator_is_unauthorized():
    arch, t0, a, b, c, commons = _fixture()
    stray = new_identity(arch, "Stray", t0)
    record_usage(arch, commons, stray, 10, 0, recorded_at=t0 + 100)
    assert any("appropriator" in r for _cid, r in arch.authority_violations())


def test_unknown_kind_and_level_rejected_at_facade():
    arch, t0, a, b, c, commons = _fixture()
    with pytest.raises(ValueError):
        register_commons(arch, "Dark-Matter", "x", [a.did], registrars=[a])
    with pytest.raises(ValueError):
        apply_sanction(arch, commons, c.did, "None", "Banishment", [a])
