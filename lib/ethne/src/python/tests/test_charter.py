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
    attest_reader_independence,
    disavow_substrate,
    dissent_boundary,
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
from ethne.monitor import cohesion_report, manual_founding, propose_from_candidate

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


def _read(candidate, polity, reader=None):
    """A signed reading of `polity` by a fresh reader — the host's act (D22)."""
    reader = reader or Signer.generate()
    return e.sign_reading(candidate, polity, reader.did, reader.priv, read_at=40), reader


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
    reading, reader = _read(candidate, did)
    proposal = e.select_from_candidate(
        candidate,
        did,
        [nodes[0].did],
        "not of this founding",
        readings=[reading],
        proposed_at=PROPOSED_AT,
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
    # And since D22 the boundary is attributable: the reader who saw all thirty
    # signed for exactly the roll-plus-exclusions this charter ratified.
    assert leg.readers == [reader.did]
    assert leg.corroborating_readers == [reader.did]


def test_the_whole_cluster_chartering_itself_is_corroborated():
    nodes, did, candidate = _cluster()
    reading, reader = _read(candidate, did)
    proposal = propose_from_candidate(
        candidate, did, readings=[reading], proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, nodes[: e.majority(30)], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.excluded == 0
    assert leg.basis == "Corroborated"
    assert leg.consent_share == pytest.approx(e.majority(30) / 30)
    assert leg.corroborating_readers == [reader.did]


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


# ---- The boundary is attributable, and the excluded answer back (D22) --------


def test_a_convener_cannot_claim_a_reading_they_do_not_hold():
    """The Q5 regression, measured before the slice: one edit to a public field
    promoted a bare assertion to `Substrate` — and the grade to `Corroborated` —
    with no provider consulted and both verification passes green."""
    import json

    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    doc = json.loads(
        e.propose_charter(
            did, "tradition", "answers to X", [s.did for s in founders],
            proposed_at=PROPOSED_AT,
        )
    )
    assert doc["attestation"]["source"] == "ConvenerAsserted"
    # The old forgery: claim a reading nobody took.
    doc["attestation"]["source"] = {"Substrate": {"readings": []}}
    arch = e.Archive()
    ratify_charter(arch, json.dumps(doc), founders[:2], founded_at=FOUNDED_AT)
    with pytest.raises(Exception) as excinfo:
        arch.check_authority()
    assert "carries none" in str(excinfo.value)


def test_a_signed_reading_makes_the_boundary_attributable():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    candidate = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in founders],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    reading, reader = _read(candidate, did)
    proposal = propose_from_candidate(
        candidate, did, readings=[reading], proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.verify_all()
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert leg.source == "Substrate"
    assert leg.readers == [reader.did]
    assert leg.corroborating_readers == [reader.did]
    assert leg.dissenting_readers == []
    assert leg.basis == "Corroborated"


def test_a_reading_of_one_cluster_does_not_travel_to_another_founding():
    """Anti-replay: without the binding an honest reader's account of one cluster
    founds somebody else's polity, and the signature still verifies."""
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    _p2, _q2, elsewhere = e.generate_keypair()
    candidate = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in founders],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    stolen, _reader = _read(candidate, elsewhere)  # read for a DIFFERENT polity
    proposal = propose_from_candidate(
        candidate, did, readings=[stolen], proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    with pytest.raises(Exception) as excinfo:
        arch.check_authority()
    assert "is for polity" in str(excinfo.value)


def test_corroboration_counts_readers_who_agree():
    """A reader who signed a different boundary corroborates nothing — and the gap
    is that reader contradicting the founding, on the charter, permanently."""
    founders = [Signer.generate() for _ in range(3)]
    stranger = Signer.generate()
    _priv, _pub, did = e.generate_keypair()
    candidate = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in founders],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    agrees, agreeing = _read(candidate, did)
    # A second reader who saw a different set of three.
    other = e.founding_candidate(
        [(s.did, 0.9, None, None, False) for s in [founders[0], stranger, Signer.generate()]],
        substrate_name="tradition",
        substrate_statement="answers to X",
    )
    disagrees, disagreeing = _read(other, did)

    proposal = propose_from_candidate(
        candidate, did, readings=[agrees, disagrees], proposed_at=PROPOSED_AT
    )
    arch = e.Archive()
    ratify_charter(arch, proposal, founders[:2], founded_at=FOUNDED_AT)
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    assert sorted(leg.readers) == sorted([agreeing.did, disagreeing.did])
    assert leg.corroborating_readers == [agreeing.did]
    assert leg.dissenting_readers == [disagreeing.did]


def test_the_excluded_may_answer_and_the_record_reads_it_back(found):
    """The two claims a dissent can make, and the read-model that reports them
    without adjudicating either."""
    p = found(3)
    omitted = Signer.generate()
    dissent_boundary(
        p.archive, p.did, omitted, "Unnamed",
        "I was inside that cluster", dissented_at=200,
    )
    p.archive.verify_all()
    p.archive.check_authority()

    d = p.archive.founding_dissent(p.did, as_of=300)
    assert len(d) == 1
    assert d.unnamed == [omitted.did]
    assert d.reasons_rejected == []
    assert d.dissenters == [omitted.did]
    assert d.entries[0][1] == "Unnamed"
    # Read as of a moment, like every other derivation here.
    assert len(p.archive.founding_dissent(p.did, as_of=150)) == 0


def test_nobody_may_put_a_dissent_in_somebody_elses_mouth(found):
    """Two protections, and this is the one Python can reach.

    The binding always labels the signature with the dissenter's own DID, so filing
    under someone else's name means signing with a key that does not match it — and
    the signature check refuses it. (The authority-level rule, that the signer *set*
    is exactly the dissenter, is exercised in the Rust suite, where an entry naming
    one party and signed by another can actually be constructed.)"""
    p = found(3)
    omitted = Signer.generate()
    p.archive.dissent_boundary(
        p.did, omitted.did, p.founders[0].priv, "Unnamed", "not mine to file", 200
    )
    with pytest.raises(Exception):
        p.archive.verify_all()


def test_an_unnamed_dissent_from_someone_the_founding_names_is_refused(found):
    p = found(3)
    dissent_boundary(p.archive, p.did, p.founders[2], "Unnamed", "", dissented_at=200)
    with pytest.raises(Exception) as excinfo:
        p.archive.check_authority()
    assert "but it names them" in str(excinfo.value)


def test_rejecting_an_exclusion_reason_requires_being_excluded(found):
    p = found(3)
    stranger = Signer.generate()
    dissent_boundary(p.archive, p.did, stranger, "ReasonRejected", "", dissented_at=200)
    with pytest.raises(Exception) as excinfo:
        p.archive.check_authority()
    assert "excludes no such member" in str(excinfo.value)


def test_an_unknown_dissent_kind_is_a_value_error(found):
    p = found(3)
    with pytest.raises(ValueError):
        dissent_boundary(p.archive, p.did, Signer.generate(), "Grumbling")


def test_dissent_outlives_the_polity_it_contests(found):
    """The one entry class a dissolution does not close — otherwise founders silence
    the parties they excluded by winding the polity up."""
    p = found(3)
    omitted = Signer.generate()
    dissolve_charter(p.archive, p.did, p.founders[:2], dissolved_at=300)
    dissent_boundary(p.archive, p.did, omitted, "Unnamed", "", dissented_at=400)
    p.archive.verify_all()
    p.archive.check_authority()
    assert p.archive.founding_dissent(p.did, as_of=500).unnamed == [omitted.did]


# ---- D23: the ground a polity stands on (Q4) --------------------------------


def _machine_cluster(anchor=None, n=3, at=PROPOSED_AT):
    """A proposal for a roll of nothing but machines answering to nobody — Q4's exact
    case, a fresh AT cluster. Returns `(proposal_json, polity_did, priv, founders)`."""
    founders = [Signer.generate() for _ in range(n)]
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "the operators",
        "answers to what it did not author",
        member_dids=[s.did for s in founders],
        substrate_anchor=anchor.did if anchor is not None else None,
        derivations=[(s.did, None) for s in founders],
        proposed_at=at,
    )
    return proposal, did, founders


def test_a_polity_whose_whole_ground_is_two_empty_strings_is_refused():
    # The Q4 regression, measured before the slice: an empty substrate chartered, and
    # both verification passes were green.
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did, "", "", member_dids=[s.did for s in founders], proposed_at=PROPOSED_AT
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, founders[:2], founded_at=FOUNDED_AT)
    # The signatures are fine. It is the ground that is missing.
    archive.verify_all()
    violations = archive.authority_violations()
    assert len(violations) == 1
    assert "not articulated" in violations[0][1]


def test_a_polity_cannot_be_its_own_ground():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "us",
        "we answer to ourselves",
        member_dids=[s.did for s in founders],
        substrate_anchor=did,
        proposed_at=PROPOSED_AT,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, founders[:2], founded_at=FOUNDED_AT)
    assert "cannot ground itself" in archive.authority_violations()[0][1]


def test_a_ground_inside_what_it_grounds_is_no_ground():
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    proposal = e.propose_charter(
        did,
        "boss",
        "we answer to the first of us",
        member_dids=[s.did for s in founders],
        substrate_anchor=founders[0].did,
        proposed_at=PROPOSED_AT,
    )
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=founders[0]
    )
    assert "no ground" in archive.authority_violations()[0][1]


def test_a_roll_of_machines_answering_to_nobody_must_name_an_anchor():
    proposal, _did, founders = _machine_cluster()
    archive = e.Archive()
    ratify_charter(archive, proposal, founders[:2], founded_at=FOUNDED_AT)
    assert "must name a substrate anchor" in archive.authority_violations()[0][1]


def test_a_machine_roll_charters_once_a_ground_consents():
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    archive.verify_all()
    archive.check_authority()
    leg = archive.founding_legitimacy(did)
    assert leg.grounding == "Anchored"
    assert leg.anchor == ground.did
    assert leg.grounding in e.SUBSTRATE_GROUNDINGS


def test_a_ground_cannot_be_conscripted():
    # Named and did not sign. Nothing else is wrong with the charter.
    ground = Signer.generate()
    proposal, _did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(archive, proposal, founders[:2], founded_at=FOUNDED_AT)
    assert "consent" in archive.authority_violations()[0][1]


def test_a_human_roll_holds_its_own_tradition(polity):
    # No anchor, no derived parties. The statement IS the ground, unkeyed on purpose.
    assert polity.archive.founding_legitimacy(polity.did).grounding == "Held"


def test_the_named_ground_may_withdraw():
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    disavow_substrate(archive, did, ground, "I do not stand above these people", 500)
    archive.verify_all()
    archive.check_authority()
    standing = archive.substrate_disavowal(did, 600)
    assert standing.disavowed
    assert standing.first_disavowed_at == 500
    assert len(standing) == 1
    # And invisible before it happened.
    assert not archive.substrate_disavowal(did, 400).disavowed


def test_a_withdrawal_does_not_un_charter_the_polity():
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    disavow_substrate(archive, did, ground, disavowed_at=500)
    state = archive.charter_state(did, 600)
    assert state.chartered
    assert not state.dissolved
    assert state.substrate_anchor == ground.did
    # The charter-alone read-model says exactly what it said before: this polity WAS
    # anchored. Whether the ground still stands there is a different question of a
    # different read-model.
    assert archive.founding_legitimacy(did).grounding == "Anchored"


def test_only_the_named_ground_may_withdraw():
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    stranger = Signer.generate()
    disavow_substrate(archive, did, stranger, disavowed_at=500)
    assert "only the named ground may withdraw" in archive.authority_violations()[0][1]


def test_nobody_may_put_a_withdrawal_in_the_grounds_mouth():
    # The binding labels the signature with the signer's own DID, so a borrowed key
    # fails the CRYPTOGRAPHIC check from Python; the authority-level rule (signer set
    # == exactly the disavower) is exercised in the Rust suite, where such an entry can
    # actually be constructed. Both protections exist, each tested where it is
    # reachable.
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    impostor = Signer(did=ground.did, priv=Signer.generate().priv)
    disavow_substrate(archive, did, impostor, disavowed_at=500)
    with pytest.raises(Exception):
        archive.verify_all()


def test_there_is_nothing_to_withdraw_from_an_unanchored_polity(polity):
    stranger = Signer.generate()
    disavow_substrate(polity.archive, polity.did, stranger, disavowed_at=500)
    assert "nothing to withdraw" in polity.archive.authority_violations()[0][1]


def test_a_disowned_polity_cannot_silence_its_ground_by_winding_up():
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    dissolve_charter(archive, did, founders[:2], rationale="done", dissolved_at=400)
    disavow_substrate(archive, did, ground, disavowed_at=500)
    archive.verify_all()
    archive.check_authority()
    assert archive.substrate_disavowal(did, 600).disavowed


def test_a_withdrawn_ground_is_a_concern_on_its_own():
    # Everything else about this polity is healthy. The ground has withdrawn, and that
    # alone reaches the level that crosses to kith-covenant.
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    roll = archive.charter_state(did, 600).members
    candidate = manual_founding([s.did for s in founders], "the operators", "x")

    before = cohesion_report(
        roll, candidate, ground=archive.substrate_disavowal(did, 600), as_of=600
    )
    assert not before.ground_disavowed
    assert before.erosion == "None"

    disavow_substrate(archive, did, ground, disavowed_at=500)
    after = cohesion_report(
        roll, candidate, ground=archive.substrate_disavowal(did, 600), as_of=600
    )
    assert after.ground_disavowed
    assert after.erosion == "Concern"


def test_an_unsupplied_ground_reading_says_nothing():
    # The same discipline every optional input follows: a caller who did not ask gets
    # silence, not false health.
    ground = Signer.generate()
    proposal, did, founders = _machine_cluster(anchor=ground)
    archive = e.Archive()
    ratify_charter(
        archive, proposal, founders[:2], founded_at=FOUNDED_AT, anchor=ground
    )
    disavow_substrate(archive, did, ground, disavowed_at=500)
    roll = archive.charter_state(did, 600).members
    candidate = manual_founding([s.did for s in founders], "the operators", "x")
    r = cohesion_report(roll, candidate, as_of=600)
    assert not r.ground_disavowed
    assert r.erosion == "None"


# ---- D24: independence attestation for readers -------------------------------


def _chorus(candidate, polity, n=5):
    """`n` readings of the same boundary, each by a freshly minted reader — a sockpuppet
    chorus if one actor holds all `n` keys, which costs it 32 microseconds a voice."""
    pairs = [_read(candidate, polity) for _ in range(n)]
    return [r for r, _ in pairs], [s for _, s in pairs]


def _chorus_charter(n=5, roll=3):
    """A founding corroborated by `n` readers, all agreeing."""
    nodes, did, candidate = _cluster(roll)
    readings, readers = _chorus(candidate, did, n)
    proposal = propose_from_candidate(
        candidate, did, readings=readings, proposed_at=PROPOSED_AT
    )
    archive = e.Archive()
    ratify_charter(
        archive, proposal, nodes[: e.majority(roll)], founded_at=FOUNDED_AT
    )
    return archive, did, nodes, readers


def test_a_sockpuppet_chorus_corroborates_nothing_independently():
    # The D24 regression, measured before the slice: five minted keys, five identical
    # readings, and the charter graded `Corroborated` with both passes green.
    archive, did, _nodes, readers = _chorus_charter(5)
    archive.verify_all()
    archive.check_authority()

    # D22's reading is unchanged and still says what it always said.
    leg = archive.founding_legitimacy(did)
    assert len(leg.corroborating_readers) == 5
    assert leg.basis == "Corroborated"

    # D24's reading is the one 16 milliseconds cannot buy.
    ic = archive.independent_corroboration(did, 600)
    assert len(ic) == 0
    assert not ic.any
    assert archive.reader_independence(did, 600).independent_readers == []
    assert len(readers) == 5


def test_one_outsider_willing_to_be_named_turns_it():
    archive, did, _nodes, readers = _chorus_charter(5)
    outsider = Signer.generate()
    attest_reader_independence(
        archive, did, readers[0].did, outsider, basis="separate operator", attested_at=500
    )
    archive.verify_all()
    archive.check_authority()

    ic = archive.independent_corroboration(did, 600)
    assert len(ic) == 1
    assert ic.any
    assert ic.readers == [readers[0].did]
    assert ic.uncontested == [readers[0].did]
    # And not before it was filed.
    assert len(archive.independent_corroboration(did, 400)) == 0


def test_a_founder_vouching_for_the_foundings_own_reader_does_not_count():
    # The sockpuppet move one step out: recorded, reported, not credited.
    archive, did, nodes, readers = _chorus_charter(3)
    attest_reader_independence(archive, did, readers[0].did, nodes[0], attested_at=500)
    archive.check_authority()

    ri = archive.reader_independence(did, 600)
    assert ri.independent_readers == []
    assert dict(ri.interested_attesters) == {readers[0].did: [nodes[0].did]}
    assert len(archive.independent_corroboration(did, 600)) == 0


def test_a_reader_cannot_vouch_for_itself():
    archive, did, _nodes, readers = _chorus_charter(3)
    attest_reader_independence(archive, did, readers[0].did, readers[0], attested_at=500)
    archive.verify_all()
    archive.check_authority()
    ri = archive.reader_independence(did, 600)
    assert ri.independent_readers == []
    assert dict(ri.interested_attesters) == {readers[0].did: [readers[0].did]}


def test_there_is_nothing_to_attest_about_a_reader_the_founding_never_carried():
    # Coherence with the record spoken about — never its truth.
    archive, did, _nodes, _readers = _chorus_charter(3)
    stranger = Signer.generate()
    outsider = Signer.generate()
    attest_reader_independence(archive, did, stranger.did, outsider, attested_at=500)
    assert "is not a reader of" in archive.authority_violations()[0][1]


def test_independence_may_be_contested():
    # The counter-claim. A contested reader is reported as contested, never as
    # disqualified: nothing here adjudicates.
    archive, did, _nodes, readers = _chorus_charter(3)
    outsider = Signer.generate()
    sceptic = Signer.generate()
    attest_reader_independence(archive, did, readers[0].did, outsider, attested_at=500)
    attest_reader_independence(
        archive, did, readers[0].did, sceptic, kind="Controlled", attested_at=600
    )
    archive.check_authority()

    ri = archive.reader_independence(did, 700)
    assert ri.is_attested(readers[0].did)
    assert ri.is_contested(readers[0].did)

    ic = archive.independent_corroboration(did, 700)
    assert len(ic) == 1  # still counted...
    assert ic.contested == [readers[0].did]  # ...and the objection travels with it
    assert ic.uncontested == []


def test_a_founder_may_contest_even_though_it_may_not_vouch():
    # Deliberately asymmetric: interest disqualifies praise, not warning. A founder
    # breaking ranks is the most informative possible source for the claim.
    archive, did, nodes, readers = _chorus_charter(3)
    attest_reader_independence(
        archive, did, readers[0].did, nodes[1], kind="Controlled", attested_at=500
    )
    archive.check_authority()
    assert archive.reader_independence(did, 600).is_contested(readers[0].did)


def test_an_attestation_outlives_the_polity():
    # The THIRD entry class a dissolution does not close — a category now, not an
    # exception: evidence about a founding outlives the polity.
    archive, did, nodes, readers = _chorus_charter(3)
    outsider = Signer.generate()
    dissolve_charter(archive, did, nodes[:2], rationale="done", dissolved_at=400)
    attest_reader_independence(archive, did, readers[0].did, outsider, attested_at=500)
    archive.verify_all()
    archive.check_authority()
    assert len(archive.independent_corroboration(did, 600)) == 1


def test_two_vouchers_for_one_reader_are_still_one_reader():
    archive, did, _nodes, readers = _chorus_charter(3)
    x, y = Signer.generate(), Signer.generate()
    attest_reader_independence(archive, did, readers[0].did, x, attested_at=500)
    attest_reader_independence(archive, did, readers[0].did, y, attested_at=600)
    attest_reader_independence(archive, did, readers[0].did, x, attested_at=700)
    archive.check_authority()
    assert len(dict(archive.reader_independence(did, 800).attesters)[readers[0].did]) == 2
    assert len(archive.independent_corroboration(did, 800)) == 1


def test_an_unknown_independence_kind_is_a_value_error():
    archive, did, _nodes, readers = _chorus_charter(3)
    outsider = Signer.generate()
    with pytest.raises(ValueError, match="independence kind"):
        attest_reader_independence(
            archive, did, readers[0].did, outsider, kind="Probably", attested_at=500
        )


def test_a_convener_asserted_founding_has_no_readers_to_attest(polity):
    assert dict(polity.archive.reader_independence(polity.did, 500).attesters) == {}
    assert len(polity.archive.independent_corroboration(polity.did, 500)) == 0
    assert "Independent" in e.INDEPENDENCE_KINDS
