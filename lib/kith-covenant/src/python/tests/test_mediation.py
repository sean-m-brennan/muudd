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

"""§4 mediation: a mediator pool (per-community registries, union = pool),
dispute intake, and paneled decisions. The §4 structural rules — every panelist
an active registered mediator, a panel spanning at least `panel_min` of them,
and non-overlap with either party's community — are enforced by the §5 authority
pass (`check_authority`), a distinct step from the cryptographic `verify_all`.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    decide_mediation, reputation_of, now_seconds,
)


def _compact():
    """A→B dispute setting with a non-party mediator pool.

    Communities A (complainant) and B (respondent) are parties. Communities C
    and D each nominate mediators, so the pool is entirely non-party. Returns
    (arch, A, B, panel, t0) where `panel` is three registered, non-party
    mediators and `t0` is a base timestamp all records are dated after.
    """
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    c = new_identity(arch, "C")
    d = new_identity(arch, "D")
    t0 = now_seconds()
    m1, m2 = new_mediator(), new_mediator()
    m3 = new_mediator()
    register_mediators(arch, c, [m1, m2], valid_from=t0 - 100)
    register_mediators(arch, d, [m3], valid_from=t0 - 100)
    return arch, a, b, [m1, m2, m3], t0


def test_valid_panel_decides_dispute():
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(
        arch, a, b.did, "MediatedArbitration",
        summary="B decided a matter A's charter reserves", filed_at=t0,
    )
    decide_mediation(
        arch, dispute, panel, "Upheld",
        reasoning="the three non-party mediators concur", decided_at=t0 + 10,
    )
    arch.verify_all()
    arch.check_authority()  # panel rule satisfied: 3 non-party registered mediators

    # The dispute + outcome project onto both parties. `as_of` must be at or
    # after the decision for the outcome to be visible (the projection is a
    # point-in-time view).
    va = reputation_of(arch, a.did, as_of=t0 + 100)
    assert len(va.disputes_filed) == 1
    assert va.disputes_filed[0].respondent == b.did
    assert va.disputes_filed[0].rung == "MediatedArbitration"
    assert va.disputes_filed[0].decided_outcome == "Upheld"
    assert not va.disputes_against

    vb = reputation_of(arch, b.did, as_of=t0 + 100)
    assert len(vb.disputes_against) == 1
    assert vb.disputes_against[0].complainant == a.did
    assert vb.disputes_against[0].decided_outcome == "Upheld"

    # As of a moment after filing but before the decision, the dispute is open.
    open_view = reputation_of(arch, a.did, as_of=t0 + 5)
    assert len(open_view.disputes_filed) == 1
    assert open_view.disputes_filed[0].decided_outcome is None


def test_panel_too_small_is_unauthorized():
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    decide_mediation(arch, dispute, panel[:2], "Dismissed", decided_at=t0 + 10)
    arch.verify_all()  # crypto is fine
    with pytest.raises(RuntimeError):
        arch.check_authority()  # default panel_min=3, only 2 mediators
    # ...but a compact that lowers the minimum accepts the same ad-hoc panel.
    arch.check_authority(panel_min=2)


def test_party_mediator_on_panel_is_unauthorized():
    arch, a, b, panel, t0 = _compact()
    # A nominates one of its own as a "mediator" and slips onto the panel.
    a_mediator = new_mediator()
    register_mediators(arch, a, [a_mediator], valid_from=t0 - 100)
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    decide_mediation(
        arch, dispute, [panel[0], panel[1], a_mediator], "Upheld",
        decided_at=t0 + 10,
    )
    arch.verify_all()  # crypto is fine: A's mediator key resolves from the registry
    with pytest.raises(RuntimeError):
        arch.check_authority()  # non-overlap: no panelist from a party community
    violations = arch.authority_violations()
    assert any("party community" in reason for _cid, reason in violations)


def test_unregistered_mediator_does_not_resolve():
    arch, a, b, panel, t0 = _compact()
    stranger = new_mediator()  # never registered in any community's roster
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    decide_mediation(
        arch, dispute, [panel[0], panel[1], stranger], "Upheld",
        decided_at=t0 + 10,
    )
    # The stranger's co-signature cannot resolve to a key, so crypto fails.
    with pytest.raises(RuntimeError):
        arch.verify_all()
