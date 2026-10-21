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

"""§9 substrate-deference test through the Python surface. Exhaustive rule-by-rule
coverage lives in the Rust kc-archive authority and kc-reputation diagnostic
tests; these confirm the bindings and facades wire the whole chain."""
import pytest

from kith_covenant import (
    Archive, new_identity, ratify_constitution, amend_compact_incapable,
    amend_fast_cadence, file_dispute, constitution_state, compact_incapable,
    in_structured_fast, substrate_deference, now_seconds,
    erosion_alert, observer_dashboard, run_intra_rite,
    decide_designation, overdue_reexaminations,
    conclude_reexamination, attest_activity, silent_functions, unstaffed_functions,
    new_mediator, register_mediators, new_observer, register_observers,
    record_conduct, adopt_minimum_content, amend_minimum_content,
    minimum_forbearance,
    AMENDMENT_KINDS, FAST_CHANGES, REEXAMINATION_OUTCOMES, ATTESTED_FUNCTIONS,
    MINIMUM_SITES, FORBEARANCE_CHANGES,
)

# The default generational re-affirmation cadence, in seconds (30 years).
CADENCE = 30 * 365 * 86_400


def _founded():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    c = new_identity(arch, "C", t0)
    ratify_constitution(
        arch, [a, b, c],
        incapable=[("internal-theology", "d1"), ("internal-governance", "d2")],
        fast_periods=[("annual", t0 + 1000, t0 + 2000)],
        genesis_at=t0,
    )
    return arch, t0, a, b, c


def test_constant_tuples():
    assert AMENDMENT_KINDS == ("Add", "Remove")
    assert FAST_CHANGES == ("Extend", "Contract")


def test_ratify_and_gates():
    arch, t0, a, b, c = _founded()
    arch.verify_all()
    arch.check_authority()
    st = constitution_state(arch, as_of=t0 + 10)
    assert st.ratified
    assert len(st.compact_incapable) == 2
    assert compact_incapable(arch, "internal-theology", as_of=t0 + 10)
    assert not compact_incapable(arch, "commons-allocation", as_of=t0 + 10)
    assert in_structured_fast(arch, t0 + 1500) == "annual"
    assert in_structured_fast(arch, t0 + 5000) is None


def test_constant_tuples_for_the_minimum_content():
    assert MINIMUM_SITES == (
        "NonMemberDuties", "HostisHumaniGeneris", "RestrictedMember", "BoundaryRefusal",
    )
    assert FORBEARANCE_CHANGES == ("Broaden", "Narrow")


def test_adopt_the_ground_and_the_two_gates_stay_distinct():
    """The compact's own ground, through the Python surface. It is a separate artifact
    from the compact-incapable list on purpose: that list names matters belonging to a
    member, this one binds the compact's conduct where consent has run out."""
    arch, t0, a, b, c = _founded()
    adopt_minimum_content(
        arch, [a, b],
        forbearances=[
            ("rescue-at-the-frontier", "rescue where distress is present", "NonMemberDuties"),
            ("no-victim-supplied", "no channel supplies a victim", "BoundaryRefusal"),
        ],
        adopted_at=t0 + 10,
    )
    arch.verify_all()
    arch.check_authority()

    st = constitution_state(arch, as_of=t0 + 20)
    assert st.minimum_adopted
    assert len(st.minimum_content) == 2
    assert st.minimum_content[0].site in MINIMUM_SITES
    # Each gate answers its own question and neither answers the other's.
    assert minimum_forbearance(arch, "rescue-at-the-frontier", as_of=t0 + 20)
    assert not compact_incapable(arch, "rescue-at-the-frontier", as_of=t0 + 20)
    assert not minimum_forbearance(arch, "internal-theology", as_of=t0 + 20)
    assert compact_incapable(arch, "internal-theology", as_of=t0 + 20)


def test_narrowing_the_ground_needs_the_higher_bar_and_shows_on_the_alert():
    """Broadening is ordinary business; narrowing carries the contraction bar and is
    surfaced as erosion. Founding forbearances are included — a bar, not a lock, which
    is the one place this artifact departs from the compact-incapable list."""
    arch, t0, a, b, c = _founded()
    adopt_minimum_content(
        arch, [a, b],
        forbearances=[("rescue-at-the-frontier", "rescue where distress is present")],
        adopted_at=t0 + 10,
    )
    amend_minimum_content(arch, "Broaden", "no-victim-supplied", [a, b],
                          description="no channel supplies a victim",
                          site="BoundaryRefusal", decided_at=t0 + 20)
    arch.check_authority()
    assert erosion_alert(arch, as_of=t0 + 30).severity == "None"

    # Two deciders do not narrow it.
    amend_minimum_content(arch, "Narrow", "no-victim-supplied", [a, b],
                          site="BoundaryRefusal", decided_at=t0 + 40)
    assert any(
        "narrowing the minimum content needs" in reason
        for _cid, reason in arch.authority_violations()
    )

    # Three do, including for the founding forbearance, and the corps is told.
    fresh, u0, x, y, z = _founded()
    adopt_minimum_content(
        fresh, [x, y],
        forbearances=[("rescue-at-the-frontier", "rescue where distress is present")],
        adopted_at=u0 + 10,
    )
    amend_minimum_content(fresh, "Narrow", "rescue-at-the-frontier", [x, y, z],
                          decided_at=u0 + 40)
    fresh.verify_all()
    fresh.check_authority()
    r = substrate_deference(fresh, as_of=u0 + 50)
    assert r.minimum_narrowed == 1 and r.minimum_current == 0
    assert r.contraction_detected
    al = erosion_alert(fresh, as_of=u0 + 50)
    assert al.minimum_contraction and not al.list_contraction
    assert al.severity == "Notice"
    assert any("adopted ground narrowed" in reason for reason in al.reasons)


def test_the_adoption_is_a_singleton_and_rejects_an_empty_ground():
    arch, t0, a, b, c = _founded()
    adopt_minimum_content(
        arch, [a, b],
        forbearances=[("rescue-at-the-frontier", "d")],
        adopted_at=t0 + 10,
    )
    adopt_minimum_content(
        arch, [a, b], forbearances=[("courtesy-to-non-members", "d")], adopted_at=t0 + 20,
    )
    assert any(
        "adoption is a singleton" in reason
        for _cid, reason in arch.authority_violations()
    )
    # The facade refuses a ground with nothing in it before it reaches the archive.
    with pytest.raises(ValueError):
        adopt_minimum_content(arch, [a, b], forbearances=[], adopted_at=t0 + 30)
    with pytest.raises(ValueError):
        amend_minimum_content(arch, "Repeal", "rescue-at-the-frontier", [a, b])


def test_recurring_fast_gate_and_state():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    # A 1000s fast recurring every 10_000s (a calendar rhythm).
    ratify_constitution(
        arch, [a, b],
        incapable=[("x", "x")],
        fast_periods=[("condolence", t0 + 1000, t0 + 2000, 10_000)],
        genesis_at=t0,
    )
    arch.check_authority()
    # The gate fires in each cycle, not just the first window.
    assert in_structured_fast(arch, t0 + 1500) == "condolence"
    assert in_structured_fast(arch, t0 + 2500) is None
    assert in_structured_fast(arch, t0 + 11_500) == "condolence"
    st = constitution_state(arch, as_of=t0 + 10)
    assert st.fast_periods[0].recurrence == 10_000


def test_amend_fast_to_recurring():
    arch, t0, a, b, c = _founded()
    # Convert the one-shot "annual" fast into a recurring one via an Extend amendment.
    amend_fast_cadence(
        arch, "Extend", "annual", t0 + 1000, t0 + 2000, [a, b, c],
        recurrence=10_000, decided_at=t0 + 100,
    )
    arch.check_authority()
    assert in_structured_fast(arch, t0 + 11_500) == "annual"


def test_single_ratifier_fails_quorum():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    ratify_constitution(arch, [a], incapable=[("x", "x")], fast_periods=[], genesis_at=t0)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_second_genesis_is_singleton_violation():
    arch, t0, a, b, c = _founded()
    ratify_constitution(arch, [a, b], incapable=[("y", "y")], fast_periods=[], genesis_at=t0 + 500)
    assert any("singleton" in r or "already ratified" in r
               for _cid, r in arch.authority_violations())


def test_expansion_easy_contraction_asymmetry():
    arch, t0, a, b, c = _founded()
    # Add at the normal quorum (2): authorized.
    amend_compact_incapable(arch, "Add", "aesthetics", [a, b], decided_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    assert compact_incapable(arch, "aesthetics", as_of=t0 + 200)

    # Remove the added matter at the normal quorum (2): fails the higher bar.
    bad = Archive.load_json(arch.dump_json())
    amend_compact_incapable(bad, "Remove", "aesthetics", [a, b], decided_at=t0 + 300)
    with pytest.raises(RuntimeError):
        bad.check_authority()

    # Remove at the higher bar (3 distinct): authorized.
    good = Archive.load_json(arch.dump_json())
    amend_compact_incapable(good, "Remove", "aesthetics", [a, b, c], decided_at=t0 + 300)
    good.verify_all()
    good.check_authority()
    assert not compact_incapable(good, "aesthetics", as_of=t0 + 400)


def test_genesis_matter_is_immutable():
    arch, t0, a, b, c = _founded()
    amend_compact_incapable(arch, "Remove", "internal-theology", [a, b, c], decided_at=t0 + 100)
    assert any("genesis-immutable" in r for _cid, r in arch.authority_violations())


def test_erosion_report_surfaces_contraction_and_fast_deviation():
    arch, t0, a, b, c = _founded()
    # A dispute filed during the fast window is a deviation.
    file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0 + 1500)
    arch.verify_all()
    arch.check_authority()
    r = substrate_deference(arch, as_of=t0 + 5000)
    assert r.ratified
    assert len(r.fast_deviations) == 1
    assert r.fast_deviations[0].kind == "DisputeFiled"
    assert r.fast_deviations[0].fast_label == "annual"
    assert r.contraction_detected is False


def test_erosion_alert_none_when_healthy():
    arch, t0, a, b, c = _founded()
    # Expansion is not erosion.
    amend_compact_incapable(arch, "Add", "aesthetics", [a, b], decided_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    al = erosion_alert(arch, as_of=t0 + 5000)
    assert al.severity == "None"
    assert al.reasons == ()
    assert not al.list_contraction and not al.fast_contraction
    # The dashboard bundles the same alert.
    assert observer_dashboard(arch, as_of=t0 + 5000).erosion.severity == "None"


def test_erosion_alert_notice_on_single_fast_deviation():
    arch, t0, a, b, c = _founded()
    file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0 + 1500)
    arch.verify_all()
    arch.check_authority()
    al = erosion_alert(arch, as_of=t0 + 5000)
    assert al.severity == "Notice"
    assert al.active_fast_deviations == 1
    assert len(al.reasons) == 1


def test_erosion_alert_escalates_to_alert_on_multiple_dimensions():
    arch, t0, a, b, c = _founded()
    # List contraction (add then remove at the higher bar) + a fast deviation.
    amend_compact_incapable(arch, "Add", "aesthetics", [a, b], decided_at=t0 + 100)
    amend_compact_incapable(arch, "Remove", "aesthetics", [a, b, c], decided_at=t0 + 300)
    file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=t0 + 1500)
    arch.verify_all()
    arch.check_authority()
    al = erosion_alert(arch, as_of=t0 + 5000)
    assert al.severity == "Alert"
    assert al.list_contraction
    assert al.active_fast_deviations == 1
    assert len(al.reasons) == 2


def test_unknown_kind_rejected_at_facade():
    arch, t0, a, b, c = _founded()
    with pytest.raises(ValueError):
        amend_compact_incapable(arch, "Obliterate", "x", [a, b])
    with pytest.raises(ValueError):
        amend_fast_cadence(arch, "Abolish", "annual", t0 + 1000, t0 + 1200, [a, b])


# --- Uniform drift: the omission signals on the alert -----------------------
#
# See doc/architecture/erosion-legibility.md. The three original signals all
# require a record — somebody amends the list, shortens a fast, or acts during
# one — so a polity that merely stops emits none of them. These cover the two
# signals whose reference is not the polity's own recent behaviour.


def test_erosion_alert_flags_a_polity_that_merely_stopped():
    """The regression this slice exists for: nothing rolled back, no act of any
    kind, the community simply stopped re-affirming."""
    arch, t0, a, b, c = _founded()
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 100)
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 100 + CADENCE + 1)
    assert not al.list_contraction and not al.fast_contraction
    assert al.active_fast_deviations == 0
    assert al.severity == "Notice"
    assert len(al.overdue_reaffirmations) == 1
    # The dashboard bundles the same alert.
    dash = observer_dashboard(arch, as_of=t0 + 100 + CADENCE + 1)
    assert dash.erosion.severity == "Notice"


def test_erosion_alert_quiet_within_the_cadence():
    """Negative control: read on the last second before the cadence elapses, the
    same archive must be None — otherwise the signal is only a clock."""
    arch, t0, a, b, c = _founded()
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 100)
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 100 + CADENCE)
    assert al.severity == "None"
    assert al.overdue_reaffirmations == ()
    assert al.reasons == ()


def test_erosion_alert_overdue_carries_its_provenance():
    arch, t0, a, b, c = _founded()
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 100)
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 500)
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 500 + CADENCE + 7)
    (o,) = al.overdue_reaffirmations
    assert o.community_did == a.did
    assert o.seconds_since_last == CADENCE + 7
    assert o.cadence_seconds == CADENCE
    assert o.last_rite_cid  # traces back to a signed record


def test_erosion_alert_cadence_is_a_parameter():
    """protocol-components.md records that the 25-30y default lacks empirical
    support and wants a compact-formation parameter, so it must be reachable."""
    arch, t0, a, b, c = _founded()
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 100)
    arch.verify_all()

    as_of = t0 + 100 + 5000
    assert erosion_alert(arch, as_of=as_of).severity == "None"
    tight = erosion_alert(arch, as_of=as_of, rite_cadence_seconds=3600)
    assert tight.severity == "Notice"
    assert tight.overdue_reaffirmations[0].cadence_seconds == 3600


def test_erosion_alert_flags_hollowing_against_the_founding_rite():
    """Witnesses 5 -> 4 -> 4: below the FIRST, which a rolling baseline would
    have absorbed. Read inside the cadence so hollowing is the only signal."""
    arch, t0, a, b, c = _founded()
    run_intra_rite(arch, a, witness_count=5, sealed_at=t0 + 100)
    run_intra_rite(arch, a, witness_count=4, sealed_at=t0 + 200)
    run_intra_rite(arch, a, witness_count=4, sealed_at=t0 + 300)
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 800)
    assert al.hollowing_communities == (a.did,)
    assert al.overdue_reaffirmations == ()
    assert al.severity == "Notice"


def test_erosion_alert_flags_a_designation_left_past_re_examination():
    """The severest act the compact can take, with the least follow-through: an
    expedited designation carries a mandatory re-examination date, and before this
    lens nothing computed whether it had arrived."""
    arch, t0, a, b, c = _founded()
    decide_designation(
        arch, c.did, [a, b], "named pattern",
        urgency="Expedited", reversal_conditions="rev",
        cited_acts=[arch.cids()[0]], reexamine_after=t0 + 1_000, designated_at=t0 + 100,
    )
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 1_500)
    assert al.severity == "Notice"
    (o,) = al.overdue_reexaminations
    assert o.subject == c.did
    assert o.tier == "Anathema"
    assert o.seconds_overdue == 500
    # Reachable as a standalone lens too.
    assert len(overdue_reexaminations(arch, as_of=t0 + 1_500)) == 1


def test_erosion_alert_quiet_before_the_re_examination_date():
    """Negative control: one second before the date, nothing is owed yet."""
    arch, t0, a, b, c = _founded()
    decide_designation(
        arch, c.did, [a, b], "named pattern",
        urgency="Expedited", reversal_conditions="rev",
        cited_acts=[arch.cids()[0]], reexamine_after=t0 + 1_000, designated_at=t0 + 100,
    )
    arch.verify_all()

    al = erosion_alert(arch, as_of=t0 + 999)
    assert al.overdue_reexaminations == ()
    assert al.severity == "None"


# --- Erosion remediation 5 and 6 through the Python surface -------------------
#
# The measured table this slice was built from. Exhaustive rule coverage is in the
# Rust suites; these confirm the whole chain (record, rule, lens, alert) is reachable
# from Python and reads the way its §8 siblings do.

YEAR = 365 * 86_400


def test_constant_tuples_for_the_new_records():
    # `Vacated` arrived with the hostis-humani-generis bar: that tier has no reversal
    # path, so a panel re-examining it had no conclusion available except to confirm,
    # and "willingness to be wrong about past designations" needed somewhere to go.
    assert REEXAMINATION_OUTCOMES == ("Confirmed", "Reversed", "Vacated")
    assert ATTESTED_FUNCTIONS == ("Mediation", "Observation")


def _designated(arch, t0, a, b, c, due, *, subject=None, at=None):
    """An expedited designation, due at `due`, that passes the authority pass.

    The expedited path needs `quorum + 1` distinct non-probationary deliberators and a
    deliberation cycle with named representatives, so the subject is a fourth community
    and A, B and C all deliberate. (The pre-existing lens tests here never called
    `check_authority`, so they could designate with two.)"""
    at = t0 + 100 if at is None else at
    subj = subject if subject is not None else new_identity(arch, "Subject", t0)
    # The expedited path also needs a cited *covenant-facing* act by the subject: an
    # attested adverse conduct record, not merely some record in the archive. This
    # fixture used to cite `arch.cids()[0]`, community A's identity, which the rule
    # refuses — so the two tests below were failing before this slice touched them.
    act = record_conduct(
        arch, a, subj.did, "Concern", "Charity",
        severity="Serious", narrative="withheld rescue at the boundary",
        observed_at=at - 40, recorded_at=at - 30,
    )
    return decide_designation(
        arch, subj.did, [a, b, c], "named pattern",
        urgency="Expedited", reversal_conditions="rev",
        cited_acts=[act], reexamine_after=due, designated_at=at,
        deliberation_cycles=[(0, [a.did], at - 20, at - 10, "we deliberated")],
    )


def test_a_confirmed_re_examination_answers_the_deadline_and_sets_the_next():
    """Rows 1 and 2 of the measured table, and the pair that matters. Before this
    record a panel that re-examined at the mandated date and let the designation
    stand read exactly like a compact that had forgotten, forever."""
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 1_000)
    conclude_reexamination(
        arch, dz, [a, b, c], "Confirmed",
        "the pattern that named them has not changed",
        next_reexamine_after=t0 + 1_000 + 10 * YEAR, concluded_at=t0 + 1_010,
    )
    arch.verify_all()
    arch.check_authority()

    # Answered: nothing owed until the date the confirmation set.
    al = erosion_alert(arch, as_of=t0 + 1_500)
    assert al.overdue_reexaminations == ()
    assert al.severity == "None"

    # Owed again once that date passes, now on its first renewal.
    later = overdue_reexaminations(arch, as_of=t0 + 1_000 + 10 * YEAR + 5)
    assert len(later) == 1
    assert later[0].confirmations == 1
    assert later[0].seconds_overdue == 5


def test_a_re_examination_may_not_be_held_in_advance():
    """The evasion the rule exists for: confirm at designation time and buy
    permanent silence."""
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 1_000)
    conclude_reexamination(
        arch, dz, [a, b, c], "Confirmed", "nothing has changed since a moment ago",
        next_reexamine_after=t0 + 9_000, concluded_at=t0 + 110,
    )
    assert any(
        "before the mandated date" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_a_confirmation_must_renew_rather_than_discharge():
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 1_000)
    conclude_reexamination(
        arch, dz, [a, b, c], "Confirmed", "it stands", concluded_at=t0 + 1_010,
    )
    assert any(
        "must set the next re-examination date" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_reporting_a_reversal_needs_the_reversal_itself():
    """One definition of "reversed": a conclusion cannot become a second, cheaper way
    to lift a designation."""
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 1_000)
    conclude_reexamination(
        arch, dz, [a, b, c], "Reversed", "we consider it lifted", concluded_at=t0 + 1_010,
    )
    assert any(
        "no lifting record or re-entry answers it" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_a_lapsed_designation_may_not_be_cited_by_a_new_one():
    """Erosion remediation's one refusal. Everything else in this slice is reported;
    here there is an act to hang a gate on, and the most severe act the compact can
    take may not rest on an obligation the compact let lapse."""
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 1_000)
    other = new_identity(arch, "D", t0)
    # D's own covenant-facing act, so the expedited floor is met and what is on trial
    # here is the citation of a lapsed obligation and nothing else.
    act = record_conduct(
        arch, a, other.did, "Concern", "Charity",
        narrative="consorted with the designated", recorded_at=t0 + 4_000,
    )
    decide_designation(
        arch, other.did, [a, b, c], "consorting with the designated",
        urgency="Expedited", reversal_conditions="rev", cited_acts=[dz, act],
        reexamine_after=t0 + 50_000, designated_at=t0 + 5_000,
        deliberation_cycles=[(0, [a.did], t0 + 4_980, t0 + 4_990, "we deliberated")],
    )
    assert any(
        "may not rest on a lapsed obligation" in reason
        for _cid, reason in arch.authority_violations()
    )


def test_a_designation_cited_inside_its_window_passes_the_gate():
    """Negative control: the gate fires on lapse, not on citation."""
    arch, t0, a, b, c = _founded()
    dz = _designated(arch, t0, a, b, c, t0 + 100_000)
    other = new_identity(arch, "D", t0)
    act = record_conduct(
        arch, a, other.did, "Concern", "Charity",
        narrative="consorted with the designated", recorded_at=t0 + 4_000,
    )
    decide_designation(
        arch, other.did, [a, b, c], "consorting with the designated",
        urgency="Expedited", reversal_conditions="rev", cited_acts=[dz, act],
        reexamine_after=t0 + 200_000, designated_at=t0 + 5_000,
        deliberation_cycles=[(0, [a.did], t0 + 4_980, t0 + 4_990, "we deliberated")],
    )
    arch.verify_all()
    arch.check_authority()


DAY = 86_400
# The longest legal terms: §4 caps a mediator at two years, §8 an observer at one.
# The function cadence has to sit inside both, which is why it is ninety days.
MEDIATOR_TERM = 730 * DAY
OBSERVER_TERM = 365 * DAY


def _staffed(arch, community, t0, *, term=None):
    """Seat a mediator and an observer on the longest legal terms, so a reading inside
    them measures silence rather than vacancy."""
    register_mediators(
        arch, community, [new_mediator()],
        valid_from=t0, valid_until=t0 + (term or MEDIATOR_TERM),
    )
    register_observers(
        arch, community, [(new_observer(), "outsider")],
        valid_from=t0, valid_until=t0 + (term or OBSERVER_TERM),
    )


def test_an_attested_null_is_the_difference_between_quiet_and_stopped():
    """Row 5, and the record's whole purpose. Two compacts, both with an active pool,
    both with no disputes: one said so and one did not. Before `attest_activity` the
    two were the same silence, and reporting it would have been reporting missing
    data as a finding."""
    quiet, t0, a, _b, _c = _founded()
    _staffed(quiet, a, t0)
    attest_activity(
        quiet, a, "Mediation", t0, t0 + 200 * DAY,
        statement="no disputes were filed in this window",
        attested_at=t0 + 200 * DAY,
    )
    quiet.verify_all()
    quiet.check_authority()

    stopped, s_t0, sa, _sb, _sc = _founded()
    _staffed(stopped, sa, s_t0)
    stopped.check_authority()

    # Read 240 days in: past the 90-day cadence from the seat, inside it from the null.
    quiet_silent = {f.function for f in silent_functions(quiet, as_of=t0 + 240 * DAY)}
    assert "Mediation" not in quiet_silent, "an attested null is evidence, not silence"
    stopped_silent = {
        f.function for f in silent_functions(stopped, as_of=s_t0 + 240 * DAY)
    }
    assert "Mediation" in stopped_silent


def test_a_function_with_nobody_seated_is_reported():
    """Row 6, the hole neither remediation item named: measured before the fix, a
    compact with zero active mediators and zero active observers for forty-three
    years read clean on every instrument."""
    arch, t0, a, _b, _c = _founded()
    _staffed(arch, a, t0, term=YEAR)
    arch.verify_all()
    arch.check_authority()

    as_of = t0 + 43 * YEAR
    vacant = unstaffed_functions(arch, as_of=as_of)
    assert {v.function for v in vacant} == {"Mediation", "Observation"}
    assert all(v.seconds_vacant == 42 * YEAR for v in vacant)
    al = erosion_alert(arch, as_of=as_of)
    assert len(al.unstaffed_functions) == 2
    assert al.severity == "Alert"


def test_a_function_never_staffed_is_not_abandoned():
    """The negative control for row 6: declining to open an office is not abandoning
    one, and the two must not read alike."""
    arch, t0, a, _b, _c = _founded()
    register_mediators(
        arch, a, [new_mediator()], valid_from=t0, valid_until=t0 + MEDIATOR_TERM,
    )
    arch.check_authority()
    assert unstaffed_functions(arch, as_of=t0 + 200 * DAY) == ()


def test_silence_inside_the_cadence_is_not_reported():
    """The other negative control: a pool seated last month has not gone quiet."""
    arch, t0, a, _b, _c = _founded()
    _staffed(arch, a, t0)
    assert silent_functions(arch, as_of=t0 + 89 * DAY) == ()


def test_an_attestation_needs_seats_and_an_account():
    """The two rules that stop the null from being a way to say nothing while having
    looked at nothing."""
    arch, t0, a, _b, _c = _founded()
    _staffed(arch, a, t0)
    attest_activity(
        arch, a, "Mediation", t0, t0 + 100 * DAY, statement="  ",
        attested_at=t0 + 100 * DAY,
    )
    assert any(
        "must state its account" in reason
        for _cid, reason in arch.authority_violations()
    )

    unseated, u_t0, _ua, ub, _uc = _founded()
    attest_activity(
        unseated, ub, "Mediation", u_t0, u_t0 + 100 * DAY,
        statement="nothing to report", attested_at=u_t0 + 100 * DAY,
    )
    assert any(
        "has no account to give" in reason
        for _cid, reason in unseated.authority_violations()
    )
