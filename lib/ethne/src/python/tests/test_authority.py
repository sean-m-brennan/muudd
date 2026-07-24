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

"""The authority pass as Python sees it: signatures can be perfectly valid while
the decision behind them is not authorized. `check_authority` raises;
`authority_violations` reports every `(cid, reason)` instead.

Each unauthorized case gets its own archive: the rules reconstruct state by
walking prior entries, so an unauthorized record still shifts what follows.
"""
from __future__ import annotations

import pytest

import ethne as e
from ethne.authoring import (
    Signer,
    admit_member,
    expel_member,
    induct_member,
    ratify_charter,
    reaffirm_member,
)

from .conftest import FOUNDED_AT, PROPOSED_AT


def _proposal(members, polity_did):
    return e.propose_charter(
        polity_did,
        "tradition",
        "answers to X above itself",
        member_dids=[s.did for s in members],
        proposed_at=PROPOSED_AT,
    )


def _reasons(archive):
    return [reason for _cid, reason in archive.authority_violations()]


def test_sub_majority_ratification_is_unauthorized():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    arch = e.Archive()
    ratify_charter(arch, _proposal(founders, did), founders[:1], founded_at=FOUNDED_AT)

    arch.verify_all()  # the one signature is genuine
    assert any("majority" in r for r in _reasons(arch))
    with pytest.raises(RuntimeError, match="authority"):
        arch.check_authority()


def test_a_clean_polity_reports_no_violations(polity):
    assert polity.archive.authority_violations() == []
    polity.archive.check_authority()


def test_expulsion_at_a_bare_majority_is_unauthorized(wide):
    """Expulsion needs a majority + 1 — three of four is a majority, not enough."""
    expel_member(wide.archive, wide.did, wide.founders[3].did, wide.founders[:2], decided_at=200)
    wide.archive.verify_all()
    assert any("Expel needs" in r for r in _reasons(wide.archive))


def test_a_departure_that_breaches_the_self_govern_floor_is_refused(polity):
    """A roll of three may grow or dissolve, but it may not shed: two members
    could no longer form the majority + 1 its own contraction rules require."""
    from ethne.authoring import depart_member

    depart_member(polity.archive, polity.did, polity.founders[2], decided_at=200)
    polity.archive.verify_all()
    assert any("self-govern" in r for r in _reasons(polity.archive))


def test_induction_needs_a_majority_of_the_roll_to_witness(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    # The roll is four; two witnesses is short of the three required.
    induct_member(polity.archive, polity.did, joiner, polity.founders[:2], affirmed_at=300)
    polity.archive.verify_all()
    assert any("witness" in r for r in _reasons(polity.archive))


def test_a_founding_member_cannot_be_inducted_again(polity):
    induct_member(polity.archive, polity.did, polity.founders[2], polity.founders[:2], affirmed_at=200)
    assert any("already-inducted" in r for r in _reasons(polity.archive))


def test_reaffirmation_presupposes_induction(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=300)
    polity.archive.verify_all()
    assert any("never inducted" in r for r in _reasons(polity.archive))


def test_affirmation_by_a_non_member_is_unauthorized(polity):
    stranger = Signer.generate()
    reaffirm_member(polity.archive, polity.did, stranger, affirmed_at=200)
    assert any("non-member" in r for r in _reasons(polity.archive))


def test_nothing_is_authorized_after_dissolution(polity):
    from ethne.authoring import dissolve_charter

    dissolve_charter(polity.archive, polity.did, polity.founders[:2], dissolved_at=200)
    reaffirm_member(polity.archive, polity.did, polity.founders[0], affirmed_at=300)
    polity.archive.verify_all()
    assert any("after its dissolution" in r for r in _reasons(polity.archive))


def test_an_office_change_before_the_charter_is_unauthorized():
    from ethne.authoring import confer_office

    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    arch = e.Archive()
    confer_office(arch, did, "Convener", founders[2], founders[:2], decided_at=200)
    arch.verify_all()
    assert any("precedes its charter" in r for r in _reasons(arch))


def test_violations_carry_the_offending_cid(polity):
    reaffirm_member(polity.archive, polity.did, Signer.generate(), affirmed_at=200)
    violations = polity.archive.authority_violations()
    assert len(violations) == 1
    cid, _reason = violations[0]
    assert cid in polity.archive.cids()
