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
    confer_office,
    depart_member,
    dissolve_charter,
    expel_member,
    induct_member,
    ratify_charter,
    reaffirm_member,
    resign_office,
    revoke_office,
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
            (founders[0].did, 0.9, None),
            (founders[1].did, 0.8, None),
            (founders[2].did, 0.1, None),  # below the floor
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
