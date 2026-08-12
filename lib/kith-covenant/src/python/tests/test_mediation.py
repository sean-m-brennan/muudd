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
    file_argument, decide_mediation, reputation_of, now_seconds,
    mediator_pool_concentration,
)

# §4 caps a mediator's term: a seat with no end is a permanent mediator class.
MEDIATOR_TERM = 365 * 24 * 3600


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
    register_mediators(arch, c, [m1, m2], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    register_mediators(arch, d, [m3], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    return arch, a, b, [m1, m2, m3], t0


def _argue_all(arch, dispute, panel, at):
    """Every panelist files their own argument, ending at `at`, and the CIDs come back
    in panel order. A decision must cite one per co-signer, so this is what "a panel
    that deliberated" looks like in a fixture."""
    n = len(panel)
    return [
        file_argument(arch, m, dispute, "Panelist", "as we read it",
                      submitted_at=at - n + i)
        for i, m in enumerate(panel)
    ]


def test_valid_panel_decides_dispute():
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(
        arch, a, b.did, "MediatedArbitration",
        summary="B decided a matter A's charter reserves", filed_at=t0,
    )
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld",
        "the three non-party mediators concur", arguments, decided_at=t0 + 10,
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
    arguments = _argue_all(arch, dispute, panel[:2], t0 + 5)
    decide_mediation(
        arch, dispute, panel[:2], "Dismissed", "the two of us agree", arguments,
        decided_at=t0 + 10,
    )
    arch.verify_all()  # crypto is fine
    with pytest.raises(RuntimeError):
        arch.check_authority()  # default panel_min=3, only 2 mediators
    # ...but a compact that lowers the minimum accepts the same ad-hoc panel.
    arch.check_authority(panel_min=2)


def test_party_mediator_on_panel_is_unauthorized():
    arch, a, b, panel, t0 = _compact()
    # A nominates one of its own as a "mediator" and slips onto the panel.
    a_mediator = new_mediator()
    register_mediators(arch, a, [a_mediator], valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    seated = [panel[0], panel[1], a_mediator]
    arguments = _argue_all(arch, dispute, seated, t0 + 5)
    decide_mediation(
        arch, dispute, seated, "Upheld", "we concur", arguments, decided_at=t0 + 10,
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
    seated = [panel[0], panel[1], stranger]
    arguments = _argue_all(arch, dispute, seated, t0 + 5)
    decide_mediation(
        arch, dispute, seated, "Upheld", "we concur", arguments, decided_at=t0 + 10,
    )
    # The stranger's signature cannot resolve to a key, so crypto fails — now on their
    # argument as well as on the decision they co-signed.
    with pytest.raises(RuntimeError):
        arch.verify_all()


def test_a_decision_must_state_its_reasoning():
    """§4: "public decisions with **mandatory** reasoning trail". The field existed and
    no rule read it, so a panel could publish a decision that said nothing about why."""
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Upheld", "   ", arguments, decided_at=t0 + 10)
    arch.verify_all()
    assert any(
        "must state its reasoning" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_every_co_signer_must_have_argued():
    """Three signatures riding on one author's text is a rubber stamp, not a panel."""
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    only_one = [
        file_argument(arch, panel[0], dispute, "Panelist", "as I read it",
                      submitted_at=t0 + 5)
    ]
    decide_mediation(arch, dispute, panel, "Upheld", "we concur", only_one,
                     decided_at=t0 + 10)
    arch.verify_all()
    assert any(
        "without having argued it" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_a_party_case_filed_before_the_decision_must_be_answered():
    """A panel may not decide while ignoring a case put to it — and a party filing
    *after* the decision cannot unmake it, or a losing party could invalidate any
    concluded mediation with one more record."""
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    answer = file_argument(arch, b, dispute, "Party", "B answers", submitted_at=t0 + 1)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Upheld", "we concur", arguments,
                     decided_at=t0 + 10)
    arch.verify_all()
    assert any(
        "does not answer the party argument" in reason
        for _cid, reason in arch.authority_violations()
    )

    # Answering it is enough; nothing requires the panel to agree.
    heard, a2, b2, panel2, t2 = _compact()
    d2 = file_dispute(heard, a2, b2.did, "MediatedArbitration", filed_at=t2)
    ans2 = file_argument(heard, b2, d2, "Party", "B answers", submitted_at=t2 + 1)
    args2 = _argue_all(heard, d2, panel2, t2 + 5) + [ans2]
    decide_mediation(heard, d2, panel2, "Upheld", "we concur", args2, decided_at=t2 + 10)
    heard.verify_all()
    heard.check_authority()

    # And a case filed afterwards leaves the decision standing.
    file_argument(heard, b2, d2, "Party", "one more thing", submitted_at=t2 + 500)
    heard.verify_all()
    heard.check_authority()


def test_only_a_party_argues_as_one_and_only_a_mediator_as_a_panelist():
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)

    # A mediator is not a party to the dispute.
    file_argument(arch, panel[0], dispute, "Party", "not my case", submitted_at=t0 + 1)
    assert any(
        "is not a party to this dispute" in reason
        for _cid, reason in arch.authority_violations()
    )

    # And a party community is not a panelist, however it labels itself.
    fresh, a2, b2, _panel2, t2 = _compact()
    d2 = file_dispute(fresh, a2, b2.did, "MediatedArbitration", filed_at=t2)
    file_argument(fresh, a2, d2, "Panelist", "we should decide our own", submitted_at=t2 + 1)
    assert any(
        "not an active registered mediator" in reason
        for _cid, reason in fresh.authority_violations()
    )


def test_an_argument_must_state_a_position():
    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    file_argument(arch, panel[0], dispute, "Panelist", "  ", submitted_at=t0 + 1)
    assert any(
        "must state a position" in reason for _cid, reason in arch.authority_violations()
    )


# --- §4.2: the mediator pool's caps, and the three records it lacked -------------

def test_a_mediator_seat_must_have_an_end():
    """A seat with no end is a permanent mediator class, and the doctrine is direct:
    "a permanent mediator class is the first step toward an inquisitorial caste"."""
    arch = Archive()
    c = new_identity(arch, "C")
    t0 = now_seconds()
    register_mediators(arch, c, [new_mediator()], valid_from=t0 - 100)
    assert any(
        "open-ended term" in reason for _cid, reason in arch.authority_violations()
    )


def test_no_community_may_hold_a_majority_of_the_mediator_pool():
    """One community holding a majority staffs every panel in the compact, which is
    what the no-majority share exists to prevent. Below the floor it does not bite: a
    small compact must be able to staff mediation at all."""
    arch = Archive()
    c, d = new_identity(arch, "C"), new_identity(arch, "D")
    t0 = now_seconds()
    register_mediators(arch, d, [new_mediator() for _ in range(4)],
                       valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    register_mediators(arch, c, [new_mediator() for _ in range(9)],
                       valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    assert any("a majority" in reason for _cid, reason in arch.authority_violations())

    small = Archive()
    s = new_identity(small, "S")
    register_mediators(small, s, [new_mediator() for _ in range(4)],
                       valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    small.check_authority()


def test_a_party_may_request_a_recusal_and_only_the_named_mediator_grants_it():
    from kith_covenant import request_recusal, decide_recusal

    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    req = request_recusal(arch, a, dispute, panel[0].did, "prior dealings",
                          requested_at=t0 + 1)
    arch.check_authority()

    # Someone else cannot grant it for them.
    wrong = Archive.load_json(arch.dump_json())
    decide_recusal(wrong, req, "Granted", [panel[1]], decided_at=t0 + 2)
    assert any(
        "granted by exactly the mediator stepping down" in reason
        for _cid, reason in wrong.authority_violations()
    )

    # Nor may the named mediator decline their own.
    own = Archive.load_json(arch.dump_json())
    decide_recusal(own, req, "Declined", [panel[0]], reasoning="I am fine",
                   decided_at=t0 + 2)
    assert any(
        "may not answer the request for their own recusal" in reason
        for _cid, reason in own.authority_violations()
    )

    # The mediator's own grant stands, and the mediation still decides: a request
    # neither blocks nor unseats by itself.
    decide_recusal(arch, req, "Granted", [panel[0]], decided_at=t0 + 2)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Upheld", "we concur", arguments,
                     decided_at=t0 + 10)
    arch.verify_all()
    arch.check_authority()


def test_a_declined_recusal_must_state_its_reasoning():
    from kith_covenant import request_recusal, decide_recusal

    arch, a, b, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    req = request_recusal(arch, a, dispute, panel[0].did, "prior dealings",
                          requested_at=t0 + 1)
    decide_recusal(arch, req, "Declined", [panel[1], panel[2]], decided_at=t0 + 2)
    assert any(
        "must state its reasoning" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_a_protecting_power_must_be_neutral_and_accepted_by_both():
    """"It is not an arbitrator; it is a witness with standing" — so it gates nothing.
    What is enforced is that it is genuinely neutral, and that both sides consented."""
    from kith_covenant import designate_protecting_power

    arch, a, b, panel, t0 = _compact()
    neutral = new_identity(arch, "Neutral")   # seats no mediators
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    designate_protecting_power(arch, a, neutral, dispute, designated_at=t0 + 1)
    arch.verify_all()
    arch.check_authority()

    # The mediation decides exactly as it would have: a witness, not an arbitrator.
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Upheld", "we concur", arguments,
                     decided_at=t0 + 10)
    arch.check_authority()

    # The opposing party cannot stand for its opponent.
    other = Archive.load_json(arch.dump_json())
    designate_protecting_power(other, a, b, dispute, designated_at=t0 + 2)
    assert any(
        "cannot stand neutral in it" in reason
        for _cid, reason in other.authority_violations()
    )


def test_the_pool_board_reports_seating_and_lineage():
    """The lineage half is the only reader of `trained_by`, and it reports rather than
    gates: §4's apprenticeship model is "susceptible to capture by the incumbent
    generation's biases", and a pool that all traces to one trainer is that weakness."""
    arch = Archive()
    c, d = new_identity(arch, "C"), new_identity(arch, "D")
    elder, other_elder = new_identity(arch, "Elder"), new_identity(arch, "Other")
    t0 = now_seconds()
    for community in (c, d):
        register_mediators(
            arch, community, [new_mediator() for _ in range(3)],
            valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM,
            trained_by=[elder.did, elder.did, other_elder.did],
        )
    arch.check_authority()

    # Read through the module-level lens, as its §8 sibling is read: the frozen
    # view, not the extension object.
    board = mediator_pool_concentration(arch, as_of=t0)
    assert board.total_active == 6
    assert not board.majority_held          # 3 and 3 between the communities
    assert board.with_lineage == 6
    assert board.top_lineage == 4
    assert board.lineage_majority_held      # one elder trained 4 of the 6
    assert board.lineages[0] == (elder.did, 4)

    # The rotation view carries each seat's term end and its trainer, so an
    # auditor reads the lineage beside the seat it qualifies.
    assert len(board.serving) == 6
    assert all(s.term_end == t0 - 100 + MEDIATOR_TERM for s in board.serving)
    assert sum(s.trained_by == elder.did for s in board.serving) == 4
    assert {s.nominating_community for s in board.serving} == {c.did, d.did}

    # A pool that records nothing reads as unknown, never as clean.
    quiet = Archive()
    q = new_identity(quiet, "Q")
    register_mediators(quiet, q, [new_mediator() for _ in range(4)],
                       valid_from=t0 - 100, valid_until=t0 - 100 + MEDIATOR_TERM)
    qb = mediator_pool_concentration(quiet, as_of=t0)
    assert (qb.total_active, qb.with_lineage) == (4, 0)
    assert not qb.lineage_majority_held
    assert all(s.trained_by is None for s in qb.serving)
