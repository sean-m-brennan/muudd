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

"""The charter lifecycle from Python: founding, the roll, offices, transmission,
dissolution — each followed by the state it derives."""
from __future__ import annotations

import pytest

import ethne as e
from ethne.authoring import (
    Signer,
    admit_member,
    amend_commitment,
    authorize_envoy,
    confer_office,
    depart_member,
    designate_derived,
    dissolve_charter,
    expel_member,
    induct_member,
    ratify_charter,
    reaffirm_member,
    release_guardian,
    resign_envoy,
    resign_office,
    revoke_envoy,
    revoke_office,
    rotate_guardian,
)
from ethne.monitor import manual_founding, propose_from_candidate

from .conftest import FOUNDED_AT, PROPOSED_AT


def test_founding_derives_charter_state(found):
    p = found(3, commitments=[("mutual-aid", "carry each other's load")])
    p.archive.verify_all()
    p.archive.check_authority()

    st = p.archive.charter_state(p.did, as_of=300)
    assert st.chartered and not st.dissolved
    assert len(st.members) == 3
    assert st.founded_at == FOUNDED_AT
    assert st.substrate_name == "tradition"
    assert st.substrate_anchor is None
    assert st.commitments == [("mutual-aid", "carry each other's load")]


def test_a_proposal_carries_no_authority_until_ratified(polity):
    """`propose_charter` is JSON, not an entry — only ratification appends."""
    proposal = e.propose_charter(
        polity.did, "tradition", "answers to X", member_dids=[s.did for s in polity.founders]
    )
    assert isinstance(proposal, str)
    assert len(polity.archive) == 1  # unchanged by proposing


def test_manual_founding_candidate_path(found):
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    candidate = manual_founding(
        [s.did for s in founders], "tradition", "answers to X above itself"
    )
    assert len(candidate.members) == 3
    assert candidate.substrate_name == "tradition"
    assert candidate.member_reputation(founders[0].did) == 1.0

    proposal = propose_from_candidate(
        candidate, did, min_reputation=0.5, proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.verify_all()
    arch.check_authority()
    # The candidate's own substrate carried through; no override needed.
    assert arch.charter_state(did, as_of=300).substrate_statement == (
        "answers to X above itself"
    )


def test_min_reputation_trims_the_proposed_roll():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    candidate = e.founding_candidate(
        [
            (founders[0].did, 0.9, None, None, False),
            (founders[1].did, 0.8, None, None, False),
            (founders[2].did, 0.1, None, None, False),  # below the floor
        ],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    proposal = propose_from_candidate(candidate, did, min_reputation=0.5, proposed_at=PROPOSED_AT)
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.check_authority()
    roll = arch.charter_state(did, as_of=300).members
    assert set(roll) == {founders[0].did, founders[1].did}


def test_commitment_amendment_add_then_remove(polity):
    amend_commitment(
        polity.archive, polity.did, "Add", "quiet-hours",
        polity.founders[:2], description="silence at night", decided_at=200,
    )
    assert polity.archive.charter_state(polity.did, as_of=250).commitments == [
        ("quiet-hours", "silence at night")
    ]
    # Removal is the harder direction: a majority + 1 of a roll of three is all three.
    amend_commitment(
        polity.archive, polity.did, "Remove", "quiet-hours", polity.founders, decided_at=300
    )
    polity.archive.verify_all()
    polity.archive.check_authority()
    assert polity.archive.charter_state(polity.did, as_of=350).commitments == []


def test_admission_grows_the_roll_and_leaves_the_joiner_uninducted(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    polity.archive.verify_all()
    polity.archive.check_authority()

    assert joiner.did in polity.archive.charter_state(polity.did, as_of=250).members
    af = polity.archive.affirmation_state(polity.did, as_of=250)
    assert not af.is_inducted(joiner.did)
    assert af.last_affirmed(joiner.did) is None
    assert joiner.did in af.stale_members(500, 250)


def test_induction_then_reaffirmation(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=200)
    # The roll is four now, so three must witness.
    induct_member(polity.archive, polity.did, joiner, polity.founders, affirmed_at=300)
    polity.archive.verify_all()
    polity.archive.check_authority()

    af = polity.archive.affirmation_state(polity.did, as_of=350)
    assert af.is_inducted(joiner.did)
    assert af.last_affirmed(joiner.did) == 300

    reaffirm_member(polity.archive, polity.did, joiner, affirmed_at=900)
    polity.archive.check_authority()
    assert polity.archive.affirmation_state(polity.did, as_of=1000).last_affirmed(
        joiner.did
    ) == 900


def test_founding_members_are_inducted_at_founding(polity):
    af = polity.archive.affirmation_state(polity.did, as_of=300)
    assert len(af.entries) == 3
    assert all(inducted for _did, inducted, _last in af.entries)
    assert all(last == FOUNDED_AT for _did, _inducted, last in af.entries)


def test_stale_members_tracks_the_cadence(polity):
    reaffirm_member(polity.archive, polity.did, polity.founders[0], affirmed_at=900)
    af = polity.archive.affirmation_state(polity.did, as_of=1000)
    stale = af.stale_members(500, 1000)
    assert polity.founders[0].did not in stale          # renewed 100s ago
    assert polity.founders[1].did in stale              # last affirmed at founding
    assert af.stale_members(5_000, 1000) == []          # a lax cadence forgives all


def test_office_confer_resign_revoke(polity):
    holder = polity.founders[2]
    confer_office(polity.archive, polity.did, "Convener", holder, polity.founders[:2], decided_at=200)
    os_ = polity.archive.office_state(polity.did, as_of=250)
    assert os_.holdings == [("Convener", holder.did)]
    assert os_.holds(holder.did, "Convener")
    assert os_.holders("Convener") == [holder.did]
    assert os_.offices_of(holder.did) == ["Convener"]

    resign_office(polity.archive, polity.did, "Convener", holder, decided_at=300)
    assert polity.archive.office_state(polity.did, as_of=350).holdings == []

    confer_office(polity.archive, polity.did, "CharterKeeper", holder, polity.founders[:2], decided_at=400)
    revoke_office(polity.archive, polity.did, "CharterKeeper", holder.did, polity.founders[:2], decided_at=500)
    polity.archive.verify_all()
    polity.archive.check_authority()
    assert polity.archive.office_state(polity.did, as_of=550).holdings == []


def test_a_polity_specific_office(polity):
    holder = polity.founders[1]
    confer_office(polity.archive, polity.did, "quartermaster", holder, polity.founders[:2], decided_at=200)
    polity.archive.check_authority()
    os_ = polity.archive.office_state(polity.did, as_of=250)
    assert os_.holds(holder.did, "quartermaster")
    assert os_.offices_of(holder.did) == ["quartermaster"]


def test_an_office_lapses_when_its_holder_leaves_the_roll(wide):
    holder = wide.founders[3]
    confer_office(wide.archive, wide.did, "CharterKeeper", holder, wide.founders[:3], decided_at=200)
    assert wide.archive.office_state(wide.did, as_of=250).holds(holder.did, "CharterKeeper")

    depart_member(wide.archive, wide.did, holder, decided_at=300)
    wide.archive.verify_all()
    wide.archive.check_authority()
    assert not wide.archive.office_state(wide.did, as_of=350).holds(holder.did, "CharterKeeper")


def test_voluntary_departure_and_expulsion(found):
    """Expulsion is the hardest contraction: a majority + 1 with the target
    excluded, which a roll of five is the smallest to be able to muster (four
    deciders out of the four non-targets)."""
    p = found(5)
    expelled = p.founders[4]
    expel_member(p.archive, p.did, expelled.did, p.founders[:4], decided_at=200)
    assert expelled.did not in p.archive.charter_state(p.did, as_of=250).members

    leaver = p.founders[3]
    depart_member(p.archive, p.did, leaver, decided_at=300)
    p.archive.verify_all()
    p.archive.check_authority()
    assert p.archive.charter_state(p.did, as_of=350).members == [
        s.did for s in p.founders[:3]
    ]


def test_dissolution_winds_the_polity_up(polity):
    _p, _pk, successor = e.generate_keypair()
    dissolve_charter(
        polity.archive, polity.did, polity.founders[:2],
        successors=[successor], rationale="merged", dissolved_at=200,
    )
    polity.archive.verify_all()
    polity.archive.check_authority()

    st = polity.archive.charter_state(polity.did, as_of=300)
    assert st.dissolved
    assert st.dissolved_at == 200
    assert st.successors == [successor]


def test_state_is_read_as_of_a_moment(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner, polity.founders[:2], decided_at=500)
    assert len(polity.archive.charter_state(polity.did, as_of=400).members) == 3
    assert len(polity.archive.charter_state(polity.did, as_of=600).members) == 4


def test_unknown_amendment_kind_is_a_value_error(polity):
    with pytest.raises(ValueError, match="amendment kind"):
        amend_commitment(
            polity.archive, polity.did, "Rewrite", "x", polity.founders[:2], decided_at=200
        )

# --- D14: offices in use -----------------------------------------------------


def test_a_held_inductor_must_preside_or_the_roll_must_reach_a_higher_bar(found):
    """A roll of five: majority 3, so the roll's override is 4 witnesses."""
    p = found(5)
    joiner = Signer.generate()
    admit_member(p.archive, p.did, joiner, p.founders[:3], decided_at=200)
    # Roll of six now, so conferring needs 4.
    confer_office(p.archive, p.did, "Inductor", p.founders[0], p.founders[:4], decided_at=250)

    # Four witnesses is a majority of six, but the Inductor is not among them.
    induct_member(p.archive, p.did, joiner, p.founders[1:5], affirmed_at=300)
    assert any("Inductor" in m for _cid, m in p.archive.authority_violations())


def test_the_inductor_presiding_authorizes_at_a_plain_majority(found):
    p = found(5)
    joiner = Signer.generate()
    admit_member(p.archive, p.did, joiner, p.founders[:3], decided_at=200)
    confer_office(p.archive, p.did, "Inductor", p.founders[0], p.founders[:4], decided_at=250)

    induct_member(p.archive, p.did, joiner, p.founders[:4], affirmed_at=300)
    p.archive.verify_all()
    p.archive.check_authority()
    assert p.archive.affirmation_state(p.did, as_of=350).is_inducted(joiner.did)


def test_the_roll_may_induct_without_its_inductor_at_its_own_override(found):
    """The gate is defeasible by construction: an office raises the bar for the
    rite and can never block it, so the roll stays sovereign.

    A roll of seven (six founders + the joiner): majority 4, override 5 — and five
    witnesses can be drawn without touching the Inductor, which is what makes this
    the override route rather than the presiding one.
    """
    p = found(6)
    joiner = Signer.generate()
    admit_member(p.archive, p.did, joiner, p.founders[:4], decided_at=200)
    confer_office(p.archive, p.did, "Inductor", p.founders[0], p.founders[:4], decided_at=250)

    induct_member(p.archive, p.did, joiner, p.founders[1:6], affirmed_at=300)
    p.archive.verify_all()
    p.archive.check_authority()
    assert p.archive.affirmation_state(p.did, as_of=350).is_inducted(joiner.did)


def test_an_envoy_authorizer_authorizes_on_its_own_signature(polity):
    officer, envoy = polity.founders[0], polity.founders[2]
    confer_office(polity.archive, polity.did, "EnvoyAuthorizer", officer,
                  polity.founders[:2], decided_at=200)

    # One signature where the roll would need a majority — the delegated power.
    authorize_envoy(polity.archive, polity.did, envoy, [officer], decided_at=300)
    polity.archive.verify_all()
    polity.archive.check_authority()

    st = polity.archive.envoy_state(polity.did, as_of=350)
    assert st.is_active(envoy.did)
    assert st.envoys() == [envoy.did]
    assert st.standings == [(envoy.did, None, 300)]


def test_a_lone_authorizer_holding_no_office_is_unauthorized(polity):
    authorize_envoy(polity.archive, polity.did, polity.founders[2],
                    [polity.founders[0]], decided_at=300)
    assert any("EnvoyAuthorizer" in m for _cid, m in polity.archive.authority_violations())


def test_the_roll_authorizes_an_envoy_at_a_majority_with_no_office_held(polity):
    envoy = polity.founders[2]
    authorize_envoy(polity.archive, polity.did, envoy, polity.founders[:2], decided_at=300)
    polity.archive.check_authority()
    assert polity.archive.envoy_state(polity.did, as_of=350).is_active(envoy.did)


def test_a_standing_ends_by_resignation_and_by_revocation(polity):
    officer, envoy = polity.founders[0], polity.founders[2]
    confer_office(polity.archive, polity.did, "EnvoyAuthorizer", officer,
                  polity.founders[:2], decided_at=200)

    authorize_envoy(polity.archive, polity.did, envoy, [officer], decided_at=300)
    resign_envoy(polity.archive, polity.did, envoy, decided_at=400)
    assert not polity.archive.envoy_state(polity.did, as_of=450).is_active(envoy.did)

    authorize_envoy(polity.archive, polity.did, envoy, [officer], decided_at=500)
    revoke_envoy(polity.archive, polity.did, envoy.did, [officer], decided_at=600)
    polity.archive.verify_all()
    polity.archive.check_authority()
    assert not polity.archive.envoy_state(polity.did, as_of=650).is_active(envoy.did)


def test_a_standing_expires_and_lapses_off_the_roll(wide):
    officer, envoy = wide.founders[0], wide.founders[3]
    confer_office(wide.archive, wide.did, "EnvoyAuthorizer", officer,
                  wide.founders[:3], decided_at=200)
    authorize_envoy(wide.archive, wide.did, envoy, [officer], expires_at=1000, decided_at=300)
    wide.archive.check_authority()

    # The boundary instant is still inside the standing.
    assert wide.archive.envoy_state(wide.did, as_of=1000).is_active(envoy.did)
    assert not wide.archive.envoy_state(wide.did, as_of=1001).is_active(envoy.did)

    # And leaving the roll suspends it regardless of the expiry.
    depart_member(wide.archive, wide.did, envoy, decided_at=400)
    wide.archive.check_authority()
    assert not wide.archive.envoy_state(wide.did, as_of=500).is_active(envoy.did)

# --- D15: the chartered guardian edge ----------------------------------------


def test_designate_rotate_and_release_move_the_edge(polity):
    machine = polity.founders[2].did
    first, second = Signer.generate(), Signer.generate()

    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=first, decided_at=200)
    st = polity.archive.guardian_state(polity.did, as_of=250)
    assert st.is_derived(machine)
    assert st.guardian_of(machine) == first.did
    assert st.guarded_members() == [machine]

    rotate_guardian(polity.archive, polity.did, machine, second, polity.founders[:2],
                    decided_at=300)
    assert polity.archive.guardian_state(polity.did, as_of=350).guardian_of(machine) == second.did

    # The guardian steps away on their own signature.
    release_guardian(polity.archive, polity.did, machine, guardian=second, decided_at=400)
    polity.archive.verify_all()
    polity.archive.check_authority()
    st = polity.archive.guardian_state(polity.did, as_of=450)
    # The designation survives: a machine does not become human by losing its human.
    assert st.is_derived(machine)
    assert st.guardian_of(machine) is None
    assert st.unguarded_members() == [machine]


def test_a_guardian_must_consent_to_the_designation(polity):
    # There is no way to author this through the surface without the guardian's key —
    # which is the point: answerability is accepted, never assigned.
    with pytest.raises(ValueError, match="consent"):
        polity.archive.designate_derived(
            polity.did, polity.founders[2].did,
            [s.did for s in polity.founders[:2]], [s.priv for s in polity.founders[:2]],
            Signer.generate().did, None, "", 200,
        )


def test_the_roll_may_record_that_a_guardian_is_gone(polity):
    machine = polity.founders[2].did
    human = Signer.generate()
    designate_derived(polity.archive, polity.did, machine, polity.founders[:2],
                      guardian=human, decided_at=200)
    release_guardian(polity.archive, polity.did, machine, deciders=polity.founders[:2],
                     decided_at=300)
    polity.archive.check_authority()
    assert polity.archive.guardian_state(polity.did, as_of=350).guardian_of(machine) is None


def test_a_machine_majority_polity_refuses_an_office_for_an_unguarded_machine(polity):
    human = Signer.generate()
    # Two of the three are machines; one answers to nobody.
    designate_derived(polity.archive, polity.did, polity.founders[1].did,
                      polity.founders[:2], guardian=human, decided_at=200)
    designate_derived(polity.archive, polity.did, polity.founders[2].did,
                      polity.founders[:2], decided_at=210)
    assert polity.archive.guardian_state(polity.did, as_of=250).is_machine_majority(3)

    confer_office(polity.archive, polity.did, "Convener", polity.founders[2],
                  polity.founders[:2], decided_at=300)
    assert any("no guardian" in m for _cid, m in polity.archive.authority_violations())


def test_the_same_conferral_goes_through_once_a_human_answers_for_it(polity):
    human, second = Signer.generate(), Signer.generate()
    designate_derived(polity.archive, polity.did, polity.founders[1].did,
                      polity.founders[:2], guardian=human, decided_at=200)
    designate_derived(polity.archive, polity.did, polity.founders[2].did,
                      polity.founders[:2], decided_at=210)
    # An already-designated member acquires a guardian by rotation.
    rotate_guardian(polity.archive, polity.did, polity.founders[2].did, second,
                    polity.founders[:2], decided_at=220)
    confer_office(polity.archive, polity.did, "Convener", polity.founders[2],
                  polity.founders[:2], decided_at=300)
    polity.archive.verify_all()
    polity.archive.check_authority()


def test_the_office_rule_sleeps_in_a_mostly_human_polity(found):
    p = found(5)
    designate_derived(p.archive, p.did, p.founders[4].did, p.founders[:3], decided_at=200)
    assert not p.archive.guardian_state(p.did, as_of=250).is_machine_majority(5)
    confer_office(p.archive, p.did, "Convener", p.founders[4], p.founders[:3], decided_at=300)
    p.archive.check_authority()


def test_a_designation_lapses_off_the_roll(wide):
    human = Signer.generate()
    machine = wide.founders[3]
    designate_derived(wide.archive, wide.did, machine.did, wide.founders[:3],
                      guardian=human, decided_at=200)
    assert wide.archive.guardian_state(wide.did, as_of=250).is_derived(machine.did)

    depart_member(wide.archive, wide.did, machine, decided_at=300)
    wide.archive.check_authority()
    assert not wide.archive.guardian_state(wide.did, as_of=350).is_derived(machine.did)



# ---- The founding boundary: what the charter says it left out (D21) ----------


def _cluster(n=30):
    """A substrate reading of `n` nodes, and a polity DID to charter them under."""
    nodes = [Signer.generate() for _ in range(n)]
    _priv, _pub, did = e.generate_keypair()
    candidate = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in nodes],
        substrate_name="tradition",
        substrate_statement="answers to what it did not author",
    )
    return nodes, did, candidate


def test_a_faction_may_not_silently_charter_the_cluster_it_came_from():
    """The Q1 hole, as a regression: one node chartering thirty.

    It may still do it — nothing here judges a founding narrow — but the roll it
    claims must add up against the boundary it says it read.
    """
    nodes, did, candidate = _cluster()
    # A proposal that keeps one member while claiming to have read all thirty, and
    # accounts for none of the other twenty-nine.
    bad = e.propose_charter(
        did,
        "tradition",
        "answers to what it did not author",
        [nodes[0].did],
        proposed_at=PROPOSED_AT,
    )
    import json

    doc = json.loads(bad)
    doc["attestation"]["candidate_size"] = 30  # the boundary it claims to have seen
    arch = e.Archive()
    ratify_charter(arch, json.dumps(doc), nodes[:1], founded_at=FOUNDED_AT)
    with pytest.raises(Exception) as excinfo:
        arch.check_authority()
    assert "boundary" in str(excinfo.value).lower()


def test_a_narrow_founding_goes_through_when_it_names_who_it_left_out():
    """Accounting is gated; judgement is only graded. Thirty down to one, honestly."""
    nodes, did, candidate = _cluster()
    proposal = e.select_from_candidate(
        candidate, did, [nodes[0].did], "not of this founding", proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, nodes[:1], founded_at=FOUNDED_AT)
    arch.verify_all()
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.source == "Substrate"
    assert leg.candidate_size == 30
    assert leg.roll_size == 1
    assert leg.excluded == 29
    assert leg.unreasoned_exclusions == 0
    assert leg.ratifiers == 1
    # The single number Q1 asks for: one node in thirty consented.
    assert leg.consent_share == pytest.approx(1 / 30)
    assert leg.basis == "Narrowed"


def test_the_whole_cluster_chartering_itself_is_corroborated():
    nodes, did, candidate = _cluster()
    proposal = propose_from_candidate(candidate, did, proposed_at=PROPOSED_AT)
    arch = e.Archive()
    ratify_charter(arch, proposal, nodes[: e.majority(30)], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.excluded == 0
    assert leg.basis == "Corroborated"
    assert leg.consent_share == pytest.approx(e.majority(30) / 30)


def test_a_convener_asserted_roll_is_labelled_so_permanently():
    """`manual_founding` and the explicit-roll path keep working; the charter simply
    never pretends the boundary was corroborated. An honest chartered state."""
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    candidate = manual_founding([s.did for s in founders], "tradition", "answers to X")
    proposal = propose_from_candidate(candidate, did, proposed_at=PROPOSED_AT)
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.source == "ConvenerAsserted"
    assert leg.basis == "SelfAttested"
    # Nothing was hidden — the convener simply saw only what they asserted.
    assert leg.candidate_size == 3
    assert leg.excluded == 0


def test_a_python_convener_may_name_exclusions_on_the_explicit_roll():
    founders = [Signer.generate() for _ in range(3)]
    left_out = Signer.generate()
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "tradition",
        "answers to X",
        [s.did for s in founders],
        exclusions=[(left_out.did, "declined the founding")],
        proposed_at=PROPOSED_AT,
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.source == "ConvenerAsserted"
    assert leg.candidate_size == 4
    assert leg.excluded == 1
    assert leg.unreasoned_exclusions == 0
    assert leg.basis == "SelfAttested"


def test_an_exclusion_may_not_also_sit_on_the_roll():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "tradition",
        "answers to X",
        [s.did for s in founders],
        exclusions=[(founders[2].did, "left out")],
        proposed_at=PROPOSED_AT,
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    with pytest.raises(Exception):
        arch.check_authority()


def test_the_floor_records_why_each_trimmed_member_was_trimmed():
    """D18's unrated-vs-rated-low distinction, said out loud where the excluded can
    read it."""
    keep = [Signer.generate() for _ in range(3)]
    low = Signer.generate()
    unrated = Signer.generate()
    _priv, _pub, did = e.generate_keypair()
    candidate = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in keep]
        + [(low.did, 0.21, None, None, False), (unrated.did, None, None, None, False)],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    proposal = propose_from_candidate(
        candidate, did, min_reputation=0.3, proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, keep[:2], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.candidate_size == 5
    assert leg.excluded == 2
    assert leg.unreasoned_exclusions == 0
    reasons = sorted(r for _did, r in leg.exclusions)
    assert "reputation 0.21 below the floor 0.30" in reasons[0]
    assert "unrated" in reasons[1]
