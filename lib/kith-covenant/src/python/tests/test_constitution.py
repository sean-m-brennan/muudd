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

"""§9 substrate-deference test through the Python surface. Exhaustive rule-by-rule
coverage lives in the Rust kc-archive authority and kc-reputation diagnostic
tests; these confirm the bindings and facades wire the whole chain."""
import pytest

from kith_covenant import (
    Archive, new_identity, ratify_constitution, amend_compact_incapable,
    amend_fast_cadence, file_dispute, constitution_state, compact_incapable,
    in_structured_fast, substrate_deference, now_seconds,
    AMENDMENT_KINDS, FAST_CHANGES,
)


def _founded():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    c = new_identity(arch, "C", t0)
    ratify_constitution(
        arch, [a, b, c],
        incapable=[("internal-theology", "d1"), ("internal-governance", "d2")],
        fast_periods=[("annual", t0 + 1000, t0 + 2000)],
        genesis_at=t0,
    )
    return arch, t0, a, b, c


def test_constant_tuples():
    assert AMENDMENT_KINDS == ("Add", "Remove")
    assert FAST_CHANGES == ("Extend", "Contract")


def test_ratify_and_gates():
    arch, t0, a, b, c = _founded()
    arch.verify_all()
    arch.check_authority()
    st = constitution_state(arch, as_of=t0 + 10)
    assert st.ratified
    assert len(st.compact_incapable) == 2
    assert compact_incapable(arch, "internal-theology", as_of=t0 + 10)
    assert not compact_incapable(arch, "commons-allocation", as_of=t0 + 10)
    assert in_structured_fast(arch, t0 + 1500) == "annual"
    assert in_structured_fast(arch, t0 + 5000) is None


def test_single_ratifier_fails_quorum():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    ratify_constitution(arch, [a], incapable=[("x", "x")], fast_periods=[], genesis_at=t0)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_second_genesis_is_singleton_violation():
    arch, t0, a, b, c = _founded()
    ratify_constitution(arch, [a, b], incapable=[("y", "y")], fast_periods=[], genesis_at=t0 + 500)
    assert any("singleton" in r or "already ratified" in r
               for _cid, r in arch.authority_violations())


def test_expansion_easy_contraction_asymmetry():
    arch, t0, a, b, c = _founded()
    # Add at the normal quorum (2): authorized.
    amend_compact_incapable(arch, "Add", "aesthetics", [a, b], decided_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    assert compact_incapable(arch, "aesthetics", as_of=t0 + 200)

    # Remove the added matter at the normal quorum (2): fails the higher bar.
    bad = Archive.load_json(arch.dump_json())
    amend_compact_incapable(bad, "Remove", "aesthetics", [a, b], decided_at=t0 + 300)
    with pytest.raises(RuntimeError):
        bad.check_authority()

    # Remove at the higher bar (3 distinct): authorized.
    good = Archive.load_json(arch.dump_json())
    amend_compact_incapable(good, "Remove", "aesthetics", [a, b, c], decided_at=t0 + 300)
    good.verify_all()
    good.check_authority()
    assert not compact_incapable(good, "aesthetics", as_of=t0 + 400)


def test_genesis_matter_is_immutable():
    arch, t0, a, b, c = _founded()
    amend_compact_incapable(arch, "Remove", "internal-theology", [a, b, c], decided_at=t0 + 100)
    assert any("genesis-immutable" in r for _cid, r in arch.authority_violations())


def test_erosion_report_surfaces_contraction_and_fast_deviation():
    arch, t0, a, b, c = _founded()
    # A dispute filed during the fast window is a deviation.
    file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0 + 1500)
    arch.verify_all()
    arch.check_authority()
    r = substrate_deference(arch, as_of=t0 + 5000)
    assert r.ratified
    assert len(r.fast_deviations) == 1
    assert r.fast_deviations[0].kind == "DisputeFiled"
    assert r.fast_deviations[0].fast_label == "annual"
    assert r.contraction_detected is False


def test_unknown_kind_rejected_at_facade():
    arch, t0, a, b, c = _founded()
    with pytest.raises(ValueError):
        amend_compact_incapable(arch, "Obliterate", "x", [a, b])
    with pytest.raises(ValueError):
        amend_fast_cadence(arch, "Abolish", "annual", t0 + 1000, t0 + 1200, [a, b])
