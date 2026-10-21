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

"""§4.1 — the three parts §4 specifies and the tier did not carry.

Each began as a measurement against the code as it stood:

- A panel recorded that the parties **settled** a dispute neither party had ever
  agreed to settle, and it was authorized. Consent to the panel as constituted was
  not a record.
- Whether an event fell under the structured-skepticism protocol was four booleans
  the *originating community* set on its own report, so a manufacturer that declared
  its own event mundane was never held — and the mediator pool, which §4 makes the
  invoking authority, had no record with which to hold it. The mirror case, a
  community putting its own event beyond action, answered to no reviewer.
- A decision had to publish its reasoning in the clear or be refused. There was no
  third state, so the sealed-mediation-proceedings exception could not be recorded.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    file_argument, decide_mediation, record_panel_consent, now_seconds,
    report_event, invoke_event_action, invoke_skepticism, review_skepticism,
    open_decision_share, unseal_decision,
)

# §4 caps a mediator's term: a seat with no end is a permanent mediator class.
MEDIATOR_TERM = 365 * 24 * 3600
DAY = 24 * 3600
# One generational cycle, the near end of the range §4 seals for.
SEAL_SPAN = 25 * 365 * 24 * 3600


def _compact():
    """A→B dispute with a three-mediator non-party pool, and two further
    communities that hold nothing yet."""
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    c = new_identity(arch, "C")
    d = new_identity(arch, "D")
    e = new_identity(arch, "E")
    t0 = now_seconds()
    panel = [new_mediator(), new_mediator(), new_mediator()]
    register_mediators(arch, c, panel[:2], valid_from=t0 - 100,
                       valid_until=t0 - 100 + MEDIATOR_TERM)
    register_mediators(arch, d, panel[2:], valid_from=t0 - 100,
                       valid_until=t0 - 100 + MEDIATOR_TERM)
    return arch, a, b, c, d, e, panel, t0


def _argue_all(arch, dispute, panel, at):
    n = len(panel)
    return [
        file_argument(arch, m, dispute, "Panelist", "as we read it", submitted_at=at - n + i)
        for i, m in enumerate(panel)
    ]


# --- Consent to the panel as constituted -------------------------------------


def test_settlement_without_consent_is_unauthorized():
    """THE MEASUREMENT, now the rule."""
    arch, a, b, _c, _d, _e, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Settled", "they came to terms",
                     arguments, decided_at=t0 + 10)
    arch.verify_all()  # crypto is fine
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("is not a settlement" in reason
               for _cid, reason in arch.authority_violations())


def test_both_parties_consenting_authorizes_the_settlement():
    """NEGATIVE CONTROL for the above: the consent is what changed."""
    arch, a, b, _c, _d, _e, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    dids = [m.did for m in panel]
    record_panel_consent(arch, a, dispute, dids, "Given", given_at=t0 + 1)
    record_panel_consent(arch, b, dispute, dids, "Given", given_at=t0 + 1)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(arch, dispute, panel, "Settled", "they came to terms",
                     arguments, decided_at=t0 + 10)
    arch.verify_all()
    arch.check_authority()


def test_consent_does_not_carry_to_a_different_panel():
    """A party that accepted three mediators has not accepted a fourth."""
    arch, a, b, _c, _d, e, panel, t0 = _compact()
    extra = new_mediator()
    register_mediators(arch, e, [extra], valid_from=t0 - 100,
                       valid_until=t0 - 100 + MEDIATOR_TERM)
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    three = [m.did for m in panel]
    record_panel_consent(arch, a, dispute, three, "Given", given_at=t0 + 1)
    record_panel_consent(arch, b, dispute, three, "Given", given_at=t0 + 1)

    four = panel + [extra]
    arguments = _argue_all(arch, dispute, four, t0 + 5)
    decide_mediation(arch, dispute, four, "Settled", "the four of us record terms",
                     arguments, decided_at=t0 + 10)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_withheld_consent_blocks_only_settlement():
    """Requiring consent to decide at all would hand either party a veto over its
    own dispute. It gates the settlement and nothing else."""
    for outcome in ("Upheld", "Dismissed", "Referred"):
        arch, a, b, _c, _d, _e, panel, t0 = _compact()
        dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
        dids = [m.did for m in panel]
        record_panel_consent(arch, a, dispute, dids, "Withheld",
                             statement="we do not accept this panel", given_at=t0 + 1)
        record_panel_consent(arch, b, dispute, dids, "Withheld",
                             statement="nor do we", given_at=t0 + 1)
        arguments = _argue_all(arch, dispute, panel, t0 + 5)
        decide_mediation(arch, dispute, panel, outcome, "we decide regardless",
                         arguments, decided_at=t0 + 10)
        arch.verify_all()
        arch.check_authority()


def test_a_non_party_has_no_consent_to_give():
    arch, a, b, _c, _d, e, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    record_panel_consent(arch, e, dispute, [m.did for m in panel], "Given", given_at=t0 + 1)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("not a party to this dispute" in reason
               for _cid, reason in arch.authority_violations())


def test_withheld_consent_must_state_something():
    arch, a, b, _c, _d, _e, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    record_panel_consent(arch, a, dispute, [m.did for m in panel], "Withheld",
                         statement="  ", given_at=t0 + 1)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_consent_answer_is_validated_in_python():
    arch, a, b, _c, _d, _e, panel, t0 = _compact()
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    with pytest.raises(ValueError):
        record_panel_consent(arch, a, dispute, [m.did for m in panel], "Maybe")


# --- Pool authority over the skepticism protocol -----------------------------


def test_the_pool_holds_an_event_its_originator_declared_mundane():
    """THE MEASUREMENT, now the rule. The originator declares nothing, so the
    self-declared hold does not reach the event — but the pool's invocation does."""
    arch, _a, _b, _c, _d, e, panel, t0 = _compact()
    ev = report_event(arch, e, "a sudden, faction-convenient omen",
                      declared_incentives="none that we see",
                      information_history="first-hand", reported_at=t0)
    # Before the invocation this is authorized, which is exactly the hole.
    invoke_event_action(arch, e, ev, "designate the rival", invoked_at=t0 + 3600)
    arch.verify_all()
    arch.check_authority()

    invoke_skepticism(
        arch, panel, ev, ["SuddenOnset", "OriginatorHoldsStake"],
        "onset within the hour of the allocation vote, and the witness stands to gain",
        14 * DAY, invoked_at=t0 + 60,
    )
    invoke_event_action(arch, e, ev, "designate the rival", invoked_at=t0 + 7200)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("structured-skepticism hold" in reason
               for _cid, reason in arch.authority_violations())


def test_a_pretextual_review_releases_a_self_declared_hold():
    """The mirror half: a community putting its own event beyond action is now
    answerable, because the review is against the hold rather than the invocation."""
    arch, _a, _b, _c, _d, e, panel, t0 = _compact()
    ev = report_event(arch, e, "our own urgent matter", high_interpretive_charge=True,
                      declared_incentives="i", information_history="h", reported_at=t0)
    invoke_event_action(arch, e, ev, "act on it", invoked_at=t0 + 3600)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()

    review_skepticism(
        arch, panel, ev, "Pretextual",
        "the originator flagged its own rescue request to keep the compact from answering it",
        reviewed_at=t0 + 1800,
    )
    arch.verify_all()
    arch.check_authority()


def test_the_invoking_panel_may_not_review_itself():
    arch, _a, _b, _c, _d, e, panel, t0 = _compact()
    ev = report_event(arch, e, "an omen", declared_incentives="i",
                      information_history="h", reported_at=t0)
    invoke_skepticism(arch, panel, ev, ["SuddenOnset"], "onset at the vote",
                      14 * DAY, invoked_at=t0 + 60)
    review_skepticism(arch, panel, ev, "Upheld", "we reviewed ourselves",
                      reviewed_at=t0 + 120)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("co-signed the invocation under review" in reason
               for _cid, reason in arch.authority_violations())


def test_an_invoked_hold_is_bounded_and_documented():
    arch, _a, _b, _c, _d, e, panel, t0 = _compact()
    ev = report_event(arch, e, "an omen", declared_incentives="i",
                      information_history="h", reported_at=t0)
    # An unbounded hold is suppression, not skepticism.
    invoke_skepticism(arch, panel, ev, ["SuddenOnset"], "onset at the vote",
                      400 * DAY, invoked_at=t0 + 60)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()

    # An invocation naming no criterion is refused in Python before it is authored.
    with pytest.raises(ValueError):
        invoke_skepticism(arch, panel, ev, ["NoSuchCriterion"], "because", 14 * DAY)


def test_review_answer_is_validated_in_python():
    arch, _a, _b, _c, _d, e, panel, t0 = _compact()
    ev = report_event(arch, e, "an omen", reported_at=t0)
    with pytest.raises(ValueError):
        review_skepticism(arch, panel, ev, "Probably", "hard to say")


# --- Sealed decisions with threshold-reveal ----------------------------------


def _seal_fixture():
    """A dispute, its panel, and three custodian communities that seat no mediator.

    Custodians are **communities**: §4 caps a mediator's seat at two years, and the
    seal opens at twenty-five, so a share sealed to a mediator would be held by a key
    the archive stops resolving two decades before the seal opens.
    """
    arch, a, b, _c, _d, _e, panel, t0 = _compact()
    holders = [new_identity(arch, n) for n in ("F", "G", "H")]
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    return arch, a, b, panel, holders, dispute, t0


def test_a_panel_seals_its_proceedings_and_custodians_open_them():
    """THE MEASUREMENT, now the rule: there is a third state between publishing the
    reasoning and being refused."""
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    text = "the respondent's account of the boundary is the older one"
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    dec = decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning=text,
        seal_custodian_dids=[h.did for h in holders],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    arch.verify_all()
    arch.check_authority()

    # A generation later, two of the three custodians open their shares.
    openers = holders[:2]
    shares = [open_decision_share(arch, dec, h) for h in openers]
    unseal_decision(arch, dec, openers, text, shares,
                    unsealed_at=t0 + 11 + SEAL_SPAN)
    arch.verify_all()
    arch.check_authority()


def test_a_panel_may_not_hold_a_quorum_of_its_own_shares():
    """A panel that can unseal itself has deferred publication at its own discretion
    rather than sealed anything.

    The panel here is seated by C and D (see `_compact`), so C and D are its home
    communities and two of them at a threshold of two is a quorum the panel controls."""
    arch, a, b, c, d, _e, panel, t0 = _compact()
    f = new_identity(arch, "F")
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning="the proceedings",
        seal_custodian_dids=[c.did, d.did, f.did],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("the panel can unseal its own" in reason
               for _cid, reason in arch.authority_violations())


def test_one_panel_home_among_the_custodians_is_below_the_quorum():
    """NEGATIVE CONTROL for the above: one is not a quorum at a threshold of two."""
    arch, a, b, c, _d, _e, panel, t0 = _compact()
    f = new_identity(arch, "F")
    g = new_identity(arch, "G")
    dispute = file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0)
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning="the proceedings",
        seal_custodian_dids=[c.did, f.did, g.did],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    arch.verify_all()
    arch.check_authority()


def test_a_party_may_not_hold_a_share_of_its_own_dispute():
    arch, a, _b, panel, holders, dispute, t0 = _seal_fixture()
    text = "the proceedings"
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning=text,
        seal_custodian_dids=[a.did, holders[0].did, holders[1].did],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("is a party to this dispute" in reason
               for _cid, reason in arch.authority_violations())


def test_a_custodian_must_be_a_community_that_will_still_resolve():
    """A mediator's seat is capped at two years against a twenty-five-year seal, so a
    share sealed to one could never be opened."""
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning="the proceedings",
        seal_custodian_dids=[panel[0].did, holders[0].did, holders[1].did],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("not a published member community" in reason
               for _cid, reason in arch.authority_violations())


def test_a_sealed_decision_carries_no_public_summary():
    """One decision, one account of itself."""
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    # Caught in the binding before the record is authored at all.
    with pytest.raises(ValueError):
        decide_mediation(
            arch, dispute, panel, "Upheld", "in short, the complainant lost",
            arguments, decided_at=t0 + 10,
            seal_reasoning="the real proceedings",
            seal_custodian_dids=[h.did for h in holders],
            seal_threshold=2,
            seal_reveal_at=t0 + 10 + SEAL_SPAN,
        )


def test_an_unsealing_cannot_publish_what_the_panel_never_wrote():
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    text = "the respondent's account is the older one"
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    dec = decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning=text,
        seal_custodian_dids=[h.did for h in holders],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    openers = holders[:2]
    shares = [open_decision_share(arch, dec, h) for h in openers]
    unseal_decision(arch, dec, openers,
                    "the panel found unanimously for the complainant", shares,
                    unsealed_at=t0 + 11 + SEAL_SPAN)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("do not recover" in reason
               for _cid, reason in arch.authority_violations())


def test_opening_a_seal_early_must_cite_a_cause():
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    text = "the proceedings"
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    dec = decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning=text,
        seal_custodian_dids=[h.did for h in holders],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + SEAL_SPAN,
    )
    openers = holders[:2]
    shares = [open_decision_share(arch, dec, h) for h in openers]
    unseal_decision(arch, dec, openers, text, shares, unsealed_at=t0 + 20)
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("must cite a cause" in reason
               for _cid, reason in arch.authority_violations())


def test_the_seal_opens_within_a_generational_cycle():
    arch, _a, _b, panel, holders, dispute, t0 = _seal_fixture()
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    decide_mediation(
        arch, dispute, panel, "Upheld", "", arguments, decided_at=t0 + 10,
        seal_reasoning="the proceedings",
        seal_custodian_dids=[h.did for h in holders],
        seal_threshold=2,
        seal_reveal_at=t0 + 10 + 365 * DAY,  # next year is not a generation
    )
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    assert any("outside the generational cycle" in reason
               for _cid, reason in arch.authority_violations())


def test_sealing_needs_custodians():
    arch, _a, _b, panel, _holders, dispute, t0 = _seal_fixture()
    arguments = _argue_all(arch, dispute, panel, t0 + 5)
    with pytest.raises(ValueError):
        decide_mediation(arch, dispute, panel, "Upheld", "", arguments,
                         decided_at=t0 + 10, seal_reasoning="the proceedings")
