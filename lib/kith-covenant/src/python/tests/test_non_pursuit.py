#  Copyright 2026 Sean M. Brennan and contributors
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

"""§5 non-pursuit: the compact that keeps every form and will not move against one
member.

The measured table this slice was built from. Against a complicit majority, the honest
complainant reached limited sanctions alone and stopped there, and a compact that
dismissed the same evidence four times read identically — on the erosion alert and on
every convergence, direction, concentration, silence and vacancy reading the protocol
had — to one that prosecuted the same evidence to collective sanctions, and to one with
nothing to prosecute at all. Exhaustive rule coverage is in the Rust suites
(``kc-archive::tests::escalation::designation_teeth`` and
``kc-reputation::tests::non_pursuit``); this is the Python surface and the headline
regression.
"""
import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, now_seconds,
    record_admission, record_conduct, register_mediators,
    file_dispute, file_ladder_transition, decide_mediation, file_argument,
    attest_non_pursuit, unanswered_grievances, erosion_alert,
    reputation_of, GRIEVANCE_GROUNDS,
)

DAY = 86_400
CADENCE = 90 * DAY
MEDIATOR_TERM = 730 * DAY


def _compact(n=6):
    """A sealed compact of `n`, so `shares_a_compact` holds, with §4's panel of three
    seated by the last three members — never parties to the disputes these tests file,
    so no recusal rule touches them. Capture is legal by construction; the question
    this file is about is whether it is legible."""
    arch = Archive()
    t0 = now_seconds()
    members = [new_identity(arch, chr(ord("A") + i), t0) for i in range(n)]
    record_admission(
        arch, members, [m.did for m in members],
        compact_label="compact", witnessed_at=t0,
    )
    panel = []
    for home in members[-3:]:
        med = new_mediator()
        register_mediators(
            arch, home, [med], valid_from=t0, valid_until=t0 + MEDIATOR_TERM,
        )
        panel.append(med)
    return arch, t0, members, panel


def _adverse(arch, reporter, subject_did, at, *, rung=None, kind="Violation"):
    return record_conduct(
        arch, reporter, subject_did, kind, "CommonsObligation",
        severity="Serious", ladder_rung=rung,
        narrative="diverted a shared allocation and denied the diversion",
        recorded_at=at,
    )


def _dismiss(arch, complainant, respondent, evidence, panel, at, outcome="Dismissed"):
    """A dispute citing `evidence`, closed by the §4 panel."""
    d = file_dispute(
        arch, complainant, respondent.did, "DisputeRegistered",
        summary="the diversion", references=list(evidence), filed_at=at,
    )
    args = [
        file_argument(arch, complainant, d, "Party", "the record establishes it",
                      evidence=list(evidence), submitted_at=at + DAY),
        file_argument(arch, respondent, d, "Party", "the allocation was ambiguous",
                      submitted_at=at + DAY),
    ] + [
        file_argument(arch, m, d, "Panelist", f"panelist {i}: the complainant overreads it",
                      submitted_at=at + DAY)
        for i, m in enumerate(panel)
    ]
    dec = decide_mediation(arch, d, panel, outcome, "so decided", args,
                           decided_at=at + 2 * DAY)
    return d, dec


# --- the headline regression -------------------------------------------------

def test_burial_and_prosecution_no_longer_read_alike():
    """Three compacts, the same evidence, and the reading that separates them.

    Before this instrument existed all three graded `None` with no reasons.
    """
    def buried():
        arch, t0, m, panel = _compact()
        ev = _adverse(arch, m[1], m[0].did, t0 + DAY, rung="RegisteredProtest")
        _dismiss(arch, m[1], m[0], [ev], panel, t0 + 10 * DAY)
        arch.verify_all()
        arch.check_authority()
        return arch, t0

    def prosecuted():
        arch, t0, m, panel = _compact()
        ev = _adverse(arch, m[1], m[0].did, t0 + DAY, rung="RegisteredProtest")
        _dismiss(arch, m[1], m[0], [ev], panel, t0 + 10 * DAY, outcome="Upheld")
        arch.verify_all()
        arch.check_authority()
        return arch, t0

    def calm():
        arch, t0, _m, _panel = _compact()
        arch.verify_all()
        return arch, t0

    b, tb = buried()
    p, tp = prosecuted()
    c, tc = calm()

    assert len(unanswered_grievances(b, tb + 200 * DAY, CADENCE)) == 1
    assert unanswered_grievances(p, tp + 200 * DAY, CADENCE) == ()
    assert unanswered_grievances(c, tc + 200 * DAY, CADENCE) == ()

    # The silence reading is held off with a long attestation cadence, so what the
    # three alerts differ on is the dimension under test rather than the fixture's own
    # youth. Every other dimension is inert in all three, which is the finding.
    def alert(a, t0):
        return erosion_alert(
            a, t0 + 200 * DAY,
            attestation_cadence_seconds=400 * DAY,
            grievance_cadence_seconds=CADENCE,
        )

    ab, ap, ac = alert(b, tb), alert(p, tp), alert(c, tc)
    assert ab.severity != ap.severity
    assert ap.severity == "None" and ac.severity == "None"
    assert len(ab.unanswered_grievances) == 1
    assert ab.reasons and all(
        "no proceeding standing against them" in r for r in ab.reasons
    ), ab.reasons
    assert ap.reasons == () and ac.reasons == ()
    # Every other instrument is inert on the buried archive, which is why this one
    # had to exist.
    assert ab.silent_functions == () and ab.unstaffed_functions == ()
    assert ab.enforcement_convergence is None and ab.enforcement_direction is None
    assert ab.overdue_reexaminations == () and ab.thinning_deliberation is None


def test_the_burial_signature_is_the_same_seats_closing_the_same_evidence():
    """Four dismissals of one record by one panel: pursuits 4, dismissals 4, and the
    seat count that a single accident does not have."""
    arch, t0, m, panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    for k in range(4):
        _dismiss(arch, m[1], m[0], [ev], panel, t0 + (10 + 5 * k) * DAY)
    arch.verify_all()
    arch.check_authority()
    g = unanswered_grievances(arch, t0 + 200 * DAY, CADENCE)
    assert len(g) == 1
    assert g[0].ground == "ClosedUnresolved"
    assert g[0].pursuits == 4
    assert g[0].dismissals == 4
    assert g[0].repeat_panelists == 3
    assert g[0].severity == "Serious"


def test_an_enforcement_act_awaits_no_proceeding():
    """A record at a rung above registered protest is the compact having moved.
    Registration and protest are a wronged party's own acts."""
    for rung, expected in (
        (None, 1), ("DisputeRegistered", 1), ("RegisteredProtest", 1),
        ("ReciprocalSuspension", 0), ("LimitedSanctions", 0),
    ):
        arch, t0, m, _panel = _compact()
        _adverse(arch, m[1], m[0].did, t0 + DAY, rung=rung)
        arch.verify_all()
        got = len(unanswered_grievances(arch, t0 + 200 * DAY, CADENCE))
        assert got == expected, f"at rung {rung!r}"


def test_inside_the_cadence_nothing_is_late():
    arch, t0, m, _panel = _compact()
    _adverse(arch, m[1], m[0].did, t0 + DAY)
    assert unanswered_grievances(arch, t0 + 80 * DAY, CADENCE) == ()


# --- the record --------------------------------------------------------------

def test_the_two_grounds_and_the_attested_flag():
    arch, t0, m, panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    att = attest_non_pursuit(
        arch, m[1], m[0].did, [ev],
        "we filed it and the compact has not taken it up",
        ground="NeverPursued", filed_at=t0 + 100 * DAY,
    )
    arch.verify_all()
    arch.check_authority()
    assert att
    g = unanswered_grievances(arch, t0 + 200 * DAY, CADENCE)
    assert len(g) == 1 and g[0].attested

    # And the other ground, on its own archive.
    arch2, t2, m2, panel2 = _compact()
    ev2 = _adverse(arch2, m2[1], m2[0].did, t2 + DAY)
    _d, dec = _dismiss(arch2, m2[1], m2[0], [ev2], panel2, t2 + 10 * DAY)
    attest_non_pursuit(
        arch2, m2[1], m2[0].did, [ev2],
        "the panel dismissed it and the diversion stands",
        ground="ClosedUnresolved", closing_decision=dec, filed_at=t2 + 100 * DAY,
    )
    arch2.verify_all()
    arch2.check_authority()


def test_the_facade_refuses_a_malformed_attestation():
    arch, t0, m, _panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    with pytest.raises(ValueError, match="must cite the evidence"):
        attest_non_pursuit(arch, m[1], m[0].did, [], "why")
    with pytest.raises(ValueError, match="ground must be one of"):
        attest_non_pursuit(arch, m[1], m[0].did, [ev], "why", ground="Whatever")
    with pytest.raises(ValueError, match="must name its closing decision"):
        attest_non_pursuit(arch, m[1], m[0].did, [ev], "why", ground="ClosedUnresolved")
    with pytest.raises(ValueError, match="names no closing decision"):
        attest_non_pursuit(arch, m[1], m[0].did, [ev], "why", ground="NeverPursued",
                           closing_decision=ev)


def test_the_claim_is_checked_against_the_archive():
    """The record is not an accusation a faction can mint: an attestation whose ground
    the archive contradicts is refused."""
    arch, t0, m, panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    file_dispute(arch, m[1], m[0].did, "DisputeRegistered",
                 references=[ev], filed_at=t0 + 5 * DAY)
    bad = attest_non_pursuit(arch, m[1], m[0].did, [ev], "nobody took it up",
                             ground="NeverPursued", filed_at=t0 + 100 * DAY)
    violations = dict(arch.authority_violations())
    assert "was pursued" in violations[bad]

    # The subject's own account of its unprosecuted conduct is not a grievance.
    own = attest_non_pursuit(arch, m[0], m[0].did, [ev], "nobody has prosecuted me",
                             ground="NeverPursued", filed_at=t0 + 101 * DAY)
    assert "is the subject of this evidence" in dict(arch.authority_violations())[own]


def test_constant_tuple():
    assert GRIEVANCE_GROUNDS == ("NeverPursued", "ClosedUnresolved")


# --- the two defects, through the Python surface -----------------------------

def test_a_bystander_may_not_de_escalate_anothers_dispute():
    """Measured: two of two attempts by a complicit non-party were authorized, and the
    dispute went back to registration each time."""
    arch, t0, m, _panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    d = file_dispute(arch, m[1], m[0].did, "DisputeRegistered",
                     references=[ev], filed_at=t0 + 5 * DAY)
    file_ladder_transition(arch, d, "DisputeRegistered", "RegisteredProtest", [m[1]],
                           reasoning="up", transitioned_at=t0 + 6 * DAY)
    bystander = file_ladder_transition(
        arch, d, "RegisteredProtest", "DisputeRegistered", [m[2]],
        reasoning="this is a misunderstanding", transitioned_at=t0 + 7 * DAY,
    )
    why = dict(arch.authority_violations())[bystander]
    assert "without its complainant" in why

    # The complainant itself may concede, which is what the rule is for.
    arch2, t2, m2, _panel2 = _compact()
    ev2 = _adverse(arch2, m2[1], m2[0].did, t2 + DAY)
    d2 = file_dispute(arch2, m2[1], m2[0].did, "DisputeRegistered",
                      references=[ev2], filed_at=t2 + 5 * DAY)
    file_ladder_transition(arch2, d2, "DisputeRegistered", "RegisteredProtest", [m2[1]],
                           reasoning="up", transitioned_at=t2 + 6 * DAY)
    file_ladder_transition(arch2, d2, "RegisteredProtest", "DisputeRegistered", [m2[1]],
                           reasoning="conceded", transitioned_at=t2 + 7 * DAY)
    arch2.verify_all()
    arch2.check_authority()


def test_a_closing_decision_does_not_reach_backwards():
    """Measured: a dismissal dated day 60 retroactively unauthorized three transitions
    dated days 22 to 30, every one authorized when it was made."""
    arch, t0, m, panel = _compact()
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    d = file_dispute(arch, m[1], m[0].did, "DisputeRegistered",
                     references=[ev], filed_at=t0 + 5 * DAY)
    moves = [
        file_ladder_transition(arch, d, frm, to, [m[1]], reasoning="up",
                               transitioned_at=t0 + day * DAY)
        for frm, to, day in (
            ("DisputeRegistered", "RegisteredProtest", 22),
            ("RegisteredProtest", "ReciprocalSuspension", 25),
            ("ReciprocalSuspension", "MediatedArbitration", 30),
        )
    ]
    arch.verify_all()
    arch.check_authority()

    args = [
        file_argument(arch, m[1], d, "Party", "established", evidence=[ev],
                      submitted_at=t0 + 55 * DAY),
        file_argument(arch, m[0], d, "Party", "ambiguous", submitted_at=t0 + 55 * DAY),
    ] + [
        file_argument(arch, mm, d, "Panelist", f"panelist {i}: no violation",
                      submitted_at=t0 + 56 * DAY)
        for i, mm in enumerate(panel)
    ]
    decide_mediation(arch, d, panel, "Dismissed", "dismissed", args,
                     decided_at=t0 + 60 * DAY)
    arch.verify_all()
    violations = dict(arch.authority_violations())
    assert not [c for c in moves if c in violations], (
        "a later decision must not unauthorize earlier transitions"
    )

    # And the closure does bind what comes after it.
    after = file_ladder_transition(
        arch, d, "MediatedArbitration", "LimitedSanctions", [m[1]],
        reasoning="onward", transitioned_at=t0 + 70 * DAY,
    )
    assert "closed by a mediation decision" in dict(arch.authority_violations())[after]


def test_the_honest_minority_reaches_limited_sanctions_and_stops():
    """The seam capture severs: registration through limited sanctions need no office,
    and collective sanctions need the compact's floor."""
    arch, t0, m, _panel = _compact(n=7)
    ev = _adverse(arch, m[1], m[0].did, t0 + DAY)
    d = file_dispute(arch, m[1], m[0].did, "DisputeRegistered",
                     references=[ev], filed_at=t0 + 5 * DAY)
    for frm, to, day in (
        ("DisputeRegistered", "RegisteredProtest", 6),
        ("RegisteredProtest", "ReciprocalSuspension", 7),
        ("ReciprocalSuspension", "MediatedArbitration", 8),
        ("MediatedArbitration", "LimitedSanctions", 9),
    ):
        file_ladder_transition(arch, d, frm, to, [m[1]], reasoning="up",
                               transitioned_at=t0 + day * DAY)
    arch.verify_all()
    arch.check_authority(quorum_threshold=4)
    assert reputation_of(arch, m[1].did, as_of=t0 + 10 * DAY) \
        .disputes_filed[0].current_rung == "LimitedSanctions"

    blocked = file_ladder_transition(
        arch, d, "LimitedSanctions", "CollectiveSanctions", [m[1], m[2]],
        decision_class="SecondClass", reasoning="the two of us",
        transitioned_at=t0 + 11 * DAY,
    )
    why = dict(arch.authority_violations(quorum_threshold=4))[blocked]
    assert "requires >= 4 distinct communities" in why
