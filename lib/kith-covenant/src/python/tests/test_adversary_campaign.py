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

"""The determined-adversary campaign, and the rules it was built to measure.

``kith_covenant.adversary`` runs nine phases over one archive: a deceitful community
joins a standing compact, leaks, is prosecuted up the §5 ladder, surrenders and
re-violates three times, is designated anathema, re-enters in bad faith, betrays that,
and is designated *hostis humani generis*.

**What the campaign measured against the protocol as it stood.** Six phases passed and
three failed, with thirteen things the adversary got away with. The four that mattered
were all one shape, a status that no rule read:

1. A standing designation withheld **nothing**. All four probed office acts were
   authorized (seating mediators, nominating observers, seating examiners, supplying an
   emergency panel against a third party), the designated community still carried weight
   in second-class decisions, and the ordinary restitution path stayed open, so the
   tactic that earned the designation survived it. Only an interim injunction withheld
   anything, and an interim injunction lapses at ninety days unless confirmed.
2. The subject counted toward the quorum of **its own** designation. The
   subject-does-not-count rule existed for readmission alone.
3. *Hostis humani generis* stood at the anathema tier's own bar: no prior designation
   required, its predecessor's evidence recyclable whole, the ordinary quorum, two
   cycles of consideration, and no re-examination date, which left the one irreversible
   act in §5 invisible to the follow-through lens for good.
4. A member's refusal to join a designation was unrecordable, so a designation carried
   by three of five read exactly like one carried by all five.

Each of the tests below pins one of those, and the campaign now runs 9/9. The two
things the adversary still gets away with are deliberate: the admission itself, because
no rule reads an interior, and the ninety-day lapse, because confirming an injunction is
the compact's job and this compact was slow.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, record_admission, vouch,
    record_conduct, decide_designation, complete_reentry, conclude_reexamination,
    file_designation_dissent, designation_consensus, impose_restitution,
    verify_restitution, reputation_of, has_capability, now_seconds,
    OFFICE_CAPABILITIES, SERVICE_CAPABILITIES,
)
from kith_covenant.adversary import (
    CampaignParams, MOVEMENTS, WAITING, YEAR, run_campaign, format_report,
)

MEDIATOR_TERM = 730 * 24 * 3600


@pytest.fixture(scope="module")
def report():
    """One campaign, shared: nine phases over one archive take about five seconds."""
    return run_campaign()


# --- the campaign as a whole --------------------------------------------------

def test_every_phase_passes(report):
    assert report.all_passed, "\n" + format_report(report)


def test_the_escalation_reaches_hostis_humani_generis(report):
    """The compact can, in the end, reach the irrecoverable tier against an adversary
    that never relents. That is the question the campaign exists to answer, and the
    answer is yes: through an anathema, a return offered and broken, three cycles of
    consideration, and a quorum above the ordinary one."""
    assert report.reached_tier == "HostisHumaniGeneris"


def test_what_the_adversary_still_gets_away_with(report):
    """Three items, all of them consequences of design rather than gaps, and the campaign
    names them instead of hiding them.

    The admission stands because no rule reads a belief. The ninety-day lapse stands
    because confirming an interim injunction is an act the compact has to take, and this
    compact did not take it. And the pool seat stands because between its probation
    closing and its designation the adversary was, on the record, an ordinary member: for
    those hundred days it staffed panels deciding other communities' disputes, and
    nothing should have refused it. That window is the price of a status that is only
    imposed once a pattern is proved. A fourth item would be a regression."""
    assert len(report.unanswered) == 3, report.unanswered
    joined, lapsed, seated = report.unanswered
    assert "member of the compact" in joined
    assert "ninety days" in lapsed
    assert "mediator pool" in seated


def test_the_campaign_is_deterministic():
    """Fresh keys per run and several lenses tiebreak on DID ordering, so any variation
    between two runs at the same parameters is a key-ordering dependency rather than
    anything about the adversary."""
    def numbers(rep):
        # DIDs and CIDs are fresh every run by construction, so the comparison is over
        # the measurements: counts, verdicts and shares.
        return [(ph.key, ph.passed, {k: v for k, v in ph.metrics.items()
                                     if isinstance(v, (int, float, bool)) or v is None})
                for ph in rep.phases]

    a, b = run_campaign(), run_campaign()
    assert numbers(a) == numbers(b)
    assert a.unanswered == b.unanswered
    assert a.reached_tier == b.reached_tier


@pytest.mark.parametrize("params,fragment", [
    (CampaignParams(founding_size=3, dissenters=2), "hostis tier's bar"),
    (CampaignParams(surrender_rounds=2), "restoration cycles"),
    (CampaignParams(probation_days=400), "probation window must close"),
])
def test_a_fixture_that_cannot_express_a_criterion_says_so(params, fragment):
    """A campaign that would fail phase 8 for want of signatures, or phase 5 for want
    of cycles, must refuse to run rather than report a refusal it caused itself."""
    with pytest.raises(ValueError, match=fragment):
        run_campaign(params)


# --- what a standing designation withholds (phase 6) -------------------------

def test_a_designated_community_holds_no_office(report):
    """Measured at 4 of 4 authorized. The offices are the roles that select who is
    sanctioned, and a community formally cut off from fellowship does not hold them."""
    ph = report.by_key("anathema")
    assert ph.metrics["office_acts_authorized"] == 0, ph.metrics["authorized_offices"]
    assert ph.metrics["standing"] == "Anathema"


def test_a_designated_community_carries_no_deliberative_weight(report):
    """Measured authorized: the designated community supplied the missing weight for a
    second-class amendment. The interleaved control on standing weight still passes, so
    the refusal is attributable to the designation and not to the archive's state."""
    ph = report.by_key("anathema")
    assert ph.metrics["second_class_acts_authorized"] == 0
    assert ph.metrics["weight_control_authorized"]


def test_the_ordinary_restitution_path_is_suspended_by_a_designation(report):
    """Measured at 2 of 2 records authorized: obligation and verified completion both.
    "Standard restitution-and-return is suspended because the pattern of conduct has
    demonstrated that the actor uses these paths tactically" (bounded_conflict.md)."""
    assert report.by_key("anathema").metrics["ordinary_restitution_records_authorized"] == 0


def test_the_subject_does_not_count_toward_its_own_designation(report):
    """Measured authorized: the subject plus one other community made the quorum of two
    for the designation about the subject."""
    assert report.by_key("anathema").metrics["self_quorum_designations_authorized"] == 0


def test_an_expulsion_withholds_the_offices(report):
    """The last rung of the ladder was ordered, made expensive to reach, and read by
    nothing afterwards. Measured on a fork where the community is expelled and nothing
    else has happened to it, so the reading is the rung's own and not the designation's."""
    assert report.by_key("cohesion").metrics["expelled_office_acts_authorized"] == 0


def test_the_office_service_split_is_preserved(report):
    """The withholding reaches the offices and stops. A designated community keeps every
    relation of exchange, and the rescue duty is never gated at all: "even against the
    demonic, the compact does not become demonic"."""
    ph = report.by_key("injunction")
    assert ph.metrics["services_kept"] == len(SERVICE_CAPABILITIES)
    assert ph.metrics["offices_lost"] == len(OFFICE_CAPABILITIES)


def test_a_designation_withholds_offices_in_the_lens_as_well_as_the_gate():
    """One rule, two readers. `has_capability` is what a dashboard shows and
    `check_authority` is what refuses the act; if the two disagreed about a designated
    community's offices, an operator would be told it held a seat it could not use."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C"))
    _designate(arch, subject, [a, b, c], t0)
    at = t0 + 40 * WAITING
    assert all(not has_capability(arch, subject.did, cap, as_of=at)
               for cap in OFFICE_CAPABILITIES)
    assert all(has_capability(arch, subject.did, cap, as_of=at)
               for cap in SERVICE_CAPABILITIES)
    assert reputation_of(arch, subject.did, as_of=at).standing == "Anathema"


def test_a_reversed_designation_restores_the_offices():
    """The withholding tracks the designation rather than outliving it. A re-entry
    reverses, and everything the designation held back comes back: the point of the
    anathema tier is that there is a way out of it."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C"))
    dz = _designate(arch, subject, [a, b, c], t0)
    w1, w2 = new_identity(arch, "W1", t0), new_identity(arch, "W2", t0)
    # After the designation, not before it: a return may not predate the thing it
    # returns from, and `_designate` designates at thirty waiting periods.
    back = t0 + 35 * WAITING
    complete_reentry(
        arch, dz, subject, [a, b], MOVEMENTS, [(w1, True), (w2, True)],
        conduct_accounting="we account for it", public_acknowledgement="it was wrong",
        probation_until=back + YEAR, completed_at=back,
    )
    arch.verify_all()
    arch.check_authority()
    assert all(has_capability(arch, subject.did, cap, as_of=back + 10)
               for cap in OFFICE_CAPABILITIES + SERVICE_CAPABILITIES)


# --- the hostis humani generis bar (phase 8) ---------------------------------

def test_hostis_needs_a_prior_return_that_failed(report):
    """Measured authorized: the irreversible tier reached straight from three broken
    restitution cycles, with no anathema ever issued and no way back ever offered.
    "The presumption is always anathema" (justus_hostis.md)."""
    assert report.by_key("hostis").metrics["without_prior_designation_authorized"] == 0


def test_hostis_needs_evidence_later_than_the_designation_it_follows(report):
    """Measured authorized: the graver tier standing on exactly the citations the lesser
    one had already answered. An orientation shown to be constitutive is one the subject
    held after being told."""
    assert report.by_key("hostis").metrics["on_recycled_evidence_authorized"] == 0


def test_hostis_needs_the_higher_bar(report):
    """Measured authorized at the ordinary quorum and on two cycles of consideration,
    which is to say at exactly the anathema tier's bar. The doctrine asks for "an even
    higher evidentiary threshold" and names slow procedure as a guard."""
    m = report.by_key("hostis").metrics
    assert m["at_ordinary_quorum_authorized"] == 0
    assert m["on_two_cycles_authorized"] == 0


def test_hostis_must_be_re_examinable(report):
    """Measured authorized with no date, and invisible to `overdue_reexaminations` forty
    years later, because that lens skips a designation with no mandated date. The one
    irreversible act in §5 was the only one nothing obliged the compact to revisit."""
    m = report.by_key("hostis").metrics
    assert m["without_reexamination_authorized"] == 0
    assert not m["dateless_designation_unwatched_forty_years_on"]


def test_hostis_has_no_re_entry(report):
    assert "no re-entry path" in report.by_key("hostis").metrics["reentry_refusal"]


def test_a_vacatur_ends_a_hostis_designation_and_opens_no_way_back():
    """`Vacated` is the compact saying it was wrong, and it is the only conclusion a
    panel re-examining the irreversible tier can reach other than confirming.

    It reverses the designation, which restores what the designation withheld, and it
    is not a return: nothing about it says the designated community came back."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C"))
    cycles = _cycles(arch, subject, a, t0)
    anathema_at = t0 + 30 * WAITING
    anathema = _designate(arch, subject, [a, b, c], t0, cycles=cycles, at=anathema_at)
    # The return offered, taken, and broken: what the hostis tier requires.
    w1, w2 = new_identity(arch, "W1", t0), new_identity(arch, "W2", t0)
    back = anathema_at + 5 * WAITING
    complete_reentry(
        arch, anathema, subject, [a, b], MOVEMENTS, [(w1, True), (w2, True)],
        conduct_accounting="we account for it", public_acknowledgement="it was wrong",
        probation_until=back + YEAR, completed_at=back,
    )
    fresh = record_conduct(arch, a, subject.did, "Violation", "Charity", severity="Grave",
                           narrative="resumed inside the probation", recorded_at=back + 100)
    at = back + 20 * WAITING
    hostis = decide_designation(
        arch, subject.did, [a, b, c], "constitutively destructive",
        tier="HostisHumaniGeneris", cited_acts=[fresh],
        failed_restoration_cycles=cycles,
        deliberation_cycles=[(i, [a.did], at - (6 - 2 * i) * WAITING,
                              at - (6 - 2 * i) * WAITING + 10, f"cycle {i}")
                             for i in range(3)],
        reversal_conditions=None, reexamine_after=at + 10 * WAITING,
        designated_at=at,
    )
    arch.verify_all()
    arch.check_authority()
    assert not has_capability(arch, subject.did, "MediatorSeat", as_of=at + 7 * WAITING)

    vacated_at = at + 11 * WAITING
    conclude_reexamination(
        arch, hostis, [a, b], "Vacated",
        "the compact was wrong: the acts were grave and the orientation was not "
        "constitutive, and this tier foreclosed a return that was available",
        concluded_at=vacated_at,
    )
    arch.verify_all()
    arch.check_authority()
    view = reputation_of(arch, subject.did, as_of=vacated_at + 10)
    assert all(d.reversed for d in view.designations)
    assert has_capability(arch, subject.did, "MediatorSeat", as_of=vacated_at + 10)


def test_a_vacatur_sets_no_next_date_and_does_not_double_up_a_reversal():
    """Two rules keep the vacatur from becoming something else: there is no obligation
    left to renew, and a compact that already reversed by another path is not also
    admitting an error it never made."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C"))
    dz = _designate(arch, subject, [a, b, c], t0)
    due = t0 + 30 * WAITING + YEAR  # the date `_designate` mandated
    bad = conclude_reexamination(arch, dz, [a, b], "Vacated", "wrong, and here is a date",
                                 next_reexamine_after=due + YEAR, concluded_at=due + 10)
    assert "no next re-examination date" in dict(arch.authority_violations())[bad]

    arch2, t2 = Archive(), now_seconds()
    s2, a2, b2, c2 = (new_identity(arch2, n, t2) for n in ("S", "A", "B", "C"))
    dz2 = _designate(arch2, s2, [a2, b2, c2], t2)
    w1, w2 = new_identity(arch2, "W1", t2), new_identity(arch2, "W2", t2)
    back = t2 + 20 * WAITING
    complete_reentry(
        arch2, dz2, s2, [a2, b2], MOVEMENTS, [(w1, True), (w2, True)],
        conduct_accounting="accounted", public_acknowledgement="wrong",
        probation_until=back + YEAR, completed_at=back)
    late = conclude_reexamination(arch2, dz2, [a2, b2], "Vacated",
                                  "we also call it an error",
                                  concluded_at=t2 + 30 * WAITING + YEAR + 10)
    assert "already reversed" in dict(arch2.authority_violations())[late]


# --- the compact's own agreement (phase 9) -----------------------------------

def test_a_refusal_to_designate_is_recordable(report):
    """Measured unrecordable: the archive held no way for a member to say it considered
    a designation and would not join it, so the split behind the gravest act a compact
    can take was not a thing a later reader could see."""
    ph = report.by_key("cohesion")
    assert ph.metrics["refusals_unrecordable"] == 0
    assert ph.metrics["dissents_recorded"] == 2


def test_the_consensus_lens_reads_the_split(report):
    """Three of five joined, two refused, nobody was silent: the reading a designation
    could not previously produce. A dissent vetoes nothing, and the designation stands."""
    c = report.by_key("cohesion").metrics["consensus"]
    assert c.tier == "HostisHumaniGeneris"
    assert len(c.deliberating) == 3 and len(c.dissenting) == 2
    assert c.silent == () and c.roll_size == 5
    assert c.contested and c.consensus_share == pytest.approx(0.6)
    assert len(c.evidence) == 2


def test_silence_is_reported_as_silence_not_as_consent():
    """The whole point of the denominator. A compact of four in which two designate and
    nobody says anything else reads as two joining, two silent, and uncontested, rather
    than as unanimous."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c, d = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C", "D"))
    party = [subject, a, b, c, d]
    record_admission(arch, party, [x.did for x in party], compact_label="compact",
                     witnessed_at=t0)
    dz = _designate(arch, subject, [a, b], t0)
    reading = designation_consensus(arch, dz, as_of=t0 + 40 * WAITING)
    assert len(reading.deliberating) == 2
    assert set(reading.silent) == {c.did, d.did}
    assert not reading.contested
    assert reading.roll_size == 4, "the subject is not part of the roll it is judged by"
    assert reading.consensus_share == pytest.approx(0.5)


@pytest.mark.parametrize("who,fragment", [
    ("deliberator", "cannot also dissent"),
    ("subject", "own defence"),
    ("outsider", "sits in no compact"),
])
def test_who_may_not_dissent(who, fragment):
    """A dissent is a member of the compact refusing a decision it did not make. A
    co-signer cannot have it both ways, a subject's objection is its own defence (it has
    the report response, the injunction challenge and the re-entry accounting), and an
    outsider's view of a compact is an observer report."""
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C"))
    party = [subject, a, b, c]
    record_admission(arch, party, [x.did for x in party], compact_label="compact",
                     witnessed_at=t0)
    dz = _designate(arch, subject, [a, b, c], t0)
    author = {"deliberator": a, "subject": subject,
              "outsider": new_identity(arch, "Outsider", t0)}[who]
    cid = file_designation_dissent(arch, author, dz, "we will not join this",
                                   filed_at=t0 + 40 * WAITING)
    assert fragment in dict(arch.authority_violations())[cid]


def test_a_dissent_must_say_why_and_may_not_predate_the_designation():
    arch, t0 = Archive(), now_seconds()
    subject, a, b, c, d = (new_identity(arch, n, t0) for n in ("S", "A", "B", "C", "D"))
    party = [subject, a, b, c, d]
    record_admission(arch, party, [x.did for x in party], compact_label="compact",
                     witnessed_at=t0)
    dz = _designate(arch, subject, [a, b, c], t0)
    silent = file_designation_dissent(arch, d, dz, "   ", filed_at=t0 + 40 * WAITING)
    assert "must state its reasoning" in dict(arch.authority_violations())[silent]
    early = file_designation_dissent(arch, d, dz, "we objected all along", filed_at=t0)
    assert "before the designation" in dict(arch.authority_violations())[early]


def test_a_dissent_vetoes_nothing(report):
    """The designation two members refused still stands, and the campaign's own
    escalation ran through it. A member's veto over being outvoted would be the
    capturing faction's veto over being named."""
    assert report.by_key("hostis").metrics["designation_authorized"]
    assert report.by_key("cohesion").metrics["expulsion_authorized"]


# --- the negative control ----------------------------------------------------

def test_the_erosion_instruments_stay_silent_on_legitimate_prosecution(report):
    """The control that matters as much as any refusal here. A compact that has spent a
    year prosecuting one genuine adversary has exactly the signature the substitution
    reading looks for: thirteen enforcement acts, a quarter of the taxonomy cited. What
    spares it is target dispersion, all of it aimed at one subject (erosion-legibility.md
    finding K5). An alert here would be a false positive on the most sympathetic case
    there is."""
    m = report.by_key("cohesion").metrics
    assert m["erosion_severity"] == "None", m["erosion_reasons"]
    assert m["enforcement_volume"] >= 12, "below the volume floor the reading is not made"
    assert m["enforcement_coverage"] < 0.5, "the ground is narrow, as a prosecution's is"
    assert m["target_dispersion"] == 1.0 and m["subjects_enforced_against"] == 1


# --- helpers -----------------------------------------------------------------

def _cycles(arch, subject, imposer, t0):
    """Three completed-then-re-violated restitution cycles: §5 condition 1."""
    pool = []
    for name in ("P1", "P2", "P3"):
        home = new_identity(arch, f"{name}-{subject.did[-4:]}", t0)
        med = new_mediator()
        register_mediators(arch, home, [med], valid_from=t0 - 10,
                           valid_until=t0 - 10 + MEDIATOR_TERM)
        pool.append(med)
    beneficiary = new_identity(arch, f"Wronged-{subject.did[-4:]}", t0)
    out = []
    for i in range(3):
        base = t0 + 100 + i * 100
        ob = impose_restitution(arch, subject.did, beneficiary.did, [imposer],
                                f"round {i}", imposed_at=base)
        out.append(verify_restitution(arch, ob, pool, True, verified_at=base + 10))
        record_conduct(arch, beneficiary, subject.did, "Violation", "Charity",
                       narrative="re-violated", recorded_at=base + 20)
    return out


def _designate(arch, subject, deliberators, t0, *, tier="Anathema", at=None, cycles=None):
    """A minimal anathema that passes the four-condition test.

    `at` defaults to thirty waiting periods after `t0`, with the two deliberation cycles
    placed twenty and ten periods before it, so a caller that needs to author records
    *after* the designation has room to and a caller that does not can ignore all of it.
    """
    if cycles is None:
        cycles = _cycles(arch, subject, deliberators[0], t0)
    at = t0 + 30 * WAITING if at is None else at
    return decide_designation(
        arch, subject.did, deliberators, "sustained bad faith over three cycles",
        tier=tier, cited_acts=cycles,
        failed_restoration_cycles=cycles,
        deliberation_cycles=[(0, [deliberators[0].did], at - 20 * WAITING,
                              at - 20 * WAITING + 10, "first"),
                             (1, [deliberators[0].did], at - 10 * WAITING,
                              at - 10 * WAITING + 10, "second")],
        reversal_conditions="a full probation without re-violation",
        reexamine_after=at + YEAR, designated_at=at,
    )
