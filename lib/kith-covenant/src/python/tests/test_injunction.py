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

"""§9 interim injunction & capability-gating through the Python surface. The
exhaustive rule-by-rule coverage lives in the Rust kc-archive authority and
kc-reputation projection tests; these confirm the bindings and facades wire the
whole chain (author -> verify_all -> check_authority -> gate query)."""
import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators,
    issue_injunction, confirm_injunction, challenge_injunction,
    reputation_of, has_capability, now_seconds, CAPABILITIES,
)

NINETY_DAYS = 90 * 24 * 3600


def _pool(arch, names, t0):
    """One registered non-party mediator per named community."""
    meds = []
    for name in names:
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10)
        meds.append(m)
    return meds


def _fixture():
    arch = Archive()
    t0 = now_seconds()
    subject = new_identity(arch, "Subject", t0)
    panel = _pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    return arch, t0, subject, panel


def test_capabilities_tuple_is_the_five_gated_services():
    assert set(CAPABILITIES) == {
        "RescueCoordination", "Navigation", "Trade", "CommonsWithdrawal", "Communication",
    }


def test_issue_revokes_capabilities_and_gate_reflects_it():
    arch, t0, s, panel = _fixture()
    assert has_capability(arch, s.did, "Trade", as_of=t0)
    issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                     revoked_capabilities=["Trade", "Navigation"], issued_at=t0)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 100)
    assert len(v.injunctions) == 1
    assert v.injunctions[0].status == "Active"
    assert v.revoked_capabilities == ("Navigation", "Trade")
    assert not has_capability(arch, s.did, "Trade", as_of=t0 + 100)
    assert not has_capability(arch, s.did, "Navigation", as_of=t0 + 100)
    assert has_capability(arch, s.did, "RescueCoordination", as_of=t0 + 100)
    # An injunction does not change recognition standing.
    assert v.standing != "Anathema"


def test_party_mediator_panel_is_unauthorized():
    arch, t0, s, panel = _fixture()
    own = new_mediator()
    register_mediators(arch, s, [own], valid_from=t0 - 10)
    issue_injunction(arch, s.did, [panel[0], panel[1], own],
                     harm="h", protective_measures="m",
                     revoked_capabilities=["Trade"], issued_at=t0)
    assert any("party community" in r for _cid, r in arch.authority_violations())


def test_small_panel_is_unauthorized():
    arch, t0, s, panel = _fixture()
    issue_injunction(arch, s.did, panel[:2], harm="h", protective_measures="m",
                     revoked_capabilities=["Trade"], issued_at=t0)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_unknown_capability_name_rejected_at_facade():
    arch, t0, s, panel = _fixture()
    with pytest.raises(ValueError):
        issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                         revoked_capabilities=["FasterThanLight"], issued_at=t0)


def test_confirmed_injunction_persists_past_expiry():
    arch, t0, s, panel = _fixture()
    a = new_identity(arch, "Conf-A", t0)
    b = new_identity(arch, "Conf-B", t0)
    inj = issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                           revoked_capabilities=["Trade"], issued_at=t0,
                           expires_at=t0 + 1000)
    confirm_injunction(arch, inj, [a, b], confirmed_at=t0 + 500)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 5000)
    assert v.injunctions[0].status == "Confirmed"
    assert not has_capability(arch, s.did, "Trade", as_of=t0 + 5000)


def test_confirmation_needs_distinct_quorum():
    arch, t0, s, panel = _fixture()
    a = new_identity(arch, "Conf-A", t0)
    inj = issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                           revoked_capabilities=["Trade"], issued_at=t0)
    confirm_injunction(arch, inj, [a], confirmed_at=t0 + 10)  # single community
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_lapsed_injunction_restores_capabilities():
    arch, t0, s, panel = _fixture()
    issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                     revoked_capabilities=["Trade"], issued_at=t0, expires_at=t0 + 1000)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 5000)  # past the window, unconfirmed
    assert v.injunctions[0].status == "Lapsed"
    assert v.revoked_capabilities == ()
    assert has_capability(arch, s.did, "Trade", as_of=t0 + 5000)


def test_overturn_by_different_panel_restores_capabilities():
    arch, t0, s, panel = _fixture()
    reviewing = _pool(arch, ("Med-D", "Med-E", "Med-F"), t0)
    inj = issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                           revoked_capabilities=["Communication"], issued_at=t0)
    challenge_injunction(arch, inj, reviewing, overturned=True, decided_at=t0 + 300)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 400)
    assert v.injunctions[0].status == "Overturned"
    assert has_capability(arch, s.did, "Communication", as_of=t0 + 400)


def test_challenge_by_issuing_panel_is_unauthorized():
    arch, t0, s, panel = _fixture()
    inj = issue_injunction(arch, s.did, panel, harm="h", protective_measures="m",
                           revoked_capabilities=["Trade"], issued_at=t0)
    challenge_injunction(arch, inj, panel, overturned=True, decided_at=t0 + 300)
    assert any("different-panel" in r or "issuing panel" in r
               for _cid, r in arch.authority_violations())
