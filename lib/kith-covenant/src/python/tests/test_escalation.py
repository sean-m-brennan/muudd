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

"""§5 escalation & designation: end-to-end authoring, authority, and projection
through the Python surface. The exhaustive rule-by-rule coverage lives in the
Rust `kc-archive` authority tests; these confirm the bindings and facades wire
the whole chain (author -> verify_all -> check_authority -> reputation)."""
import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    file_ladder_transition, decide_designation, impose_restitution,
    verify_restitution, complete_reentry, record_conduct, reputation_of,
    now_seconds,
)

WAITING = 7 * 24 * 3600  # DEFAULT_WAITING_PERIOD_SECONDS
MOVEMENTS = ("substrate", "fallenness", "recognition", "virtue", "non-ultimacy")


def _pool(arch, t0):
    """Three non-party communities C/D/E, each with one registered mediator."""
    meds = []
    for name in ("C", "D", "E"):
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10)
        meds.append(m)
    return meds


# ---- ladder ----------------------------------------------------------------

def test_ladder_walk_projects_current_rung():
    arch = Archive()
    t0 = now_seconds()
    x = new_identity(arch, "X", t0)
    y = new_identity(arch, "Y", t0)
    d = file_dispute(arch, x, y.did, "MediatedArbitration", filed_at=t0)
    file_ladder_transition(arch, d, "MediatedArbitration", "LimitedSanctions", [x],
                           transitioned_at=t0 + 10)
    file_ladder_transition(arch, d, "LimitedSanctions", "RegisteredProtest", [x],
                           transitioned_at=t0 + 20)  # de-escalate
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, x.did, as_of=t0 + 100)
    assert v.disputes_filed[0].rung == "MediatedArbitration"
    assert v.disputes_filed[0].current_rung == "RegisteredProtest"


def test_ladder_illegal_skip_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    x = new_identity(arch, "X", t0)
    y = new_identity(arch, "Y", t0)
    d = file_dispute(arch, x, y.did, "DisputeRegistered", filed_at=t0)
    file_ladder_transition(arch, d, "DisputeRegistered", "ReciprocalSuspension", [x],
                           transitioned_at=t0 + 10)  # skips a rung
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_collective_sanctions_needs_quorum_and_second_class():
    arch = Archive()
    t0 = now_seconds()
    x = new_identity(arch, "X", t0)
    y = new_identity(arch, "Y", t0)
    d = file_dispute(arch, x, y.did, "LimitedSanctions", filed_at=t0)
    # Unilateral, first-class label -> unauthorized.
    file_ladder_transition(arch, d, "LimitedSanctions", "CollectiveSanctions", [x],
                           decision_class="FirstClass", transitioned_at=t0 + 10)
    assert any("distinct" in r or "second-class" in r or "first-class" in r
               for _cid, r in arch.authority_violations())


# ---- designation four-condition test + standing ----------------------------

def _three_bad_faith_cycles(arch, subject, beneficiary, panel, t0):
    """Impose/verify/re-violate three restitution cycles; return their completion
    CIDs (what a designation cites) plus one cited-act CID."""
    completions = []
    act_cid = None
    for i in range(3):
        base = t0 + i * 100
        ob = impose_restitution(arch, subject.did, beneficiary.did, [beneficiary],
                                "restore", imposed_at=base)
        done = verify_restitution(arch, ob, panel, True, verified_at=base + 10)
        completions.append(done)
        act_cid = record_conduct(arch, beneficiary, subject.did, "Concern", "Charity",
                                 recorded_at=base + 20)
    return completions, act_cid


def _designation_fixture():
    arch = Archive()
    t0 = now_seconds()
    s = new_identity(arch, "S", t0)
    b = new_identity(arch, "B", t0)
    meds = _pool(arch, t0)
    completions, act = _three_bad_faith_cycles(arch, s, b, meds, t0)
    cycles = [
        (0, [b.did], t0 + 1000, t0 + 1100, "first cycle"),
        (1, [b.did], t0 + 1100 + WAITING, t0 + 1200 + WAITING, "second cycle"),
    ]
    return arch, t0, s, b, completions, act, cycles


def test_normal_designation_makes_subject_anathema():
    arch, t0, s, b, completions, act, cycles = _designation_fixture()
    c = new_identity(arch, "Deliberator2", t0)  # a second deliberating community
    decide_designation(
        arch, s.did, [b, c], "three graver failed restorations",
        tier="Anathema", cited_acts=[act], failed_restoration_cycles=completions,
        deliberation_cycles=cycles, reversal_conditions="changed conduct over probation",
        designated_at=t0 + 5000,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 6000)
    assert v.standing == "Anathema"
    assert len(v.designations) == 1
    assert v.designations[0].tier == "Anathema"
    assert not v.designations[0].reversed


def test_designation_without_reversal_conditions_is_unauthorized():
    arch, t0, s, b, completions, act, cycles = _designation_fixture()
    c = new_identity(arch, "Deliberator2", t0)
    decide_designation(
        arch, s.did, [b, c], "criteria", tier="Anathema",
        cited_acts=[act], failed_restoration_cycles=completions,
        deliberation_cycles=cycles, reversal_conditions=None,  # condition 4 violated
        designated_at=t0 + 5000,
    )
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_reentry_reverses_designation():
    arch, t0, s, b, completions, act, cycles = _designation_fixture()
    c = new_identity(arch, "Deliberator2", t0)
    dz = decide_designation(
        arch, s.did, [b, c], "criteria", tier="Anathema",
        cited_acts=[act], failed_restoration_cycles=completions,
        deliberation_cycles=cycles, reversal_conditions="rev", designated_at=t0 + 5000,
    )
    w1, w2 = new_identity(arch, "W1", t0), new_identity(arch, "W2", t0)
    complete_reentry(
        arch, dz, s, [s, b], MOVEMENTS, [(w1, True), (w2, True)],
        conduct_accounting="we account for it", public_acknowledgement="it was wrong",
        probation_until=t0 + 20000, completed_at=t0 + 6000,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 7000)
    assert v.designations[0].reversed
    assert len(v.reentries) == 1
    assert v.standing != "Anathema"


# ---- restitution verifier panel --------------------------------------------

def test_restitution_party_verifier_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    s = new_identity(arch, "S", t0)
    b = new_identity(arch, "B", t0)
    meds = _pool(arch, t0)
    # The subject nominates its own mediator and seats it on the verifier panel.
    s_med = new_mediator()
    register_mediators(arch, s, [s_med], valid_from=t0 - 10)
    ob = impose_restitution(arch, s.did, b.did, [b], "restore", imposed_at=t0 + 100)
    verify_restitution(arch, ob, [meds[0], meds[1], s_med], True, verified_at=t0 + 200)
    assert any("party community" in r for _cid, r in arch.authority_violations())


def test_restitution_projection_joins_verification():
    arch = Archive()
    t0 = now_seconds()
    s = new_identity(arch, "S", t0)
    b = new_identity(arch, "B", t0)
    meds = _pool(arch, t0)
    ob = impose_restitution(arch, s.did, b.did, [b], "restore", imposed_at=t0 + 100)
    verify_restitution(arch, ob, meds, True, verified_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, s.did, as_of=t0 + 300)
    assert len(v.restitutions) == 1
    assert v.restitutions[0].satisfied is True
