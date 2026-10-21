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

"""Demonstrate §5 non-pursuit: the compact that keeps every form and will not move
against one member.

Three compacts, the same evidence in each, and only the compact differs. One prosecutes
it. One dismisses it, four times, through panels drawn from the same three seats. One has
nothing to prosecute at all. Before the non-pursuit reading existed, all three graded
identically on every instrument the protocol had — because burial produces observances,
and every instrument that counts observances reads it as diligence.

Then the two rules the same measurement produced: a de-escalation is the complainant's
concession rather than a bystander's veto, and a closing decision does not reach
backwards to unauthorize the transitions that preceded it.

Everything below is authored through the public Python API, crypto-verified, and passes
the §5 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, now_seconds,
    record_admission, record_conduct, register_mediators,
    file_dispute, file_ladder_transition, decide_mediation, file_argument,
    attest_non_pursuit, unanswered_grievances, erosion_alert,
)

DAY = 86_400
CADENCE = 90 * DAY
MEDIATOR_TERM = 730 * DAY


def compact(n=6):
    """A sealed compact of `n` with §4's panel of three, seated by the last three
    members so no panelist is ever a party. Capture is legal by construction; what
    this demo is about is whether it is legible."""
    arch = Archive()
    t0 = now_seconds()
    members = [new_identity(arch, chr(ord("A") + i), t0) for i in range(n)]
    record_admission(arch, members, [m.did for m in members],
                     compact_label="compact", witnessed_at=t0)
    panel = []
    for home in members[-3:]:
        med = new_mediator()
        register_mediators(arch, home, [med], valid_from=t0,
                           valid_until=t0 + MEDIATOR_TERM)
        panel.append(med)
    return arch, t0, members, panel


def the_evidence(arch, reporter, subject, at):
    """One serious, covenant-facing violation, at the rung a wronged party reaches on
    its own. Registration and protest need no office; above them the compact's own
    machinery has to engage."""
    return record_conduct(
        arch, reporter, subject.did, "Violation", "CommonsObligation",
        severity="Serious", ladder_rung="RegisteredProtest",
        narrative="diverted a shared allocation and denied the diversion",
        recorded_at=at,
    )


def hear_it(arch, complainant, respondent, evidence, panel, at, outcome):
    d = file_dispute(arch, complainant, respondent.did, "DisputeRegistered",
                     summary="the diversion and the denial",
                     references=[evidence], filed_at=at)
    args = [
        file_argument(arch, complainant, d, "Party",
                      "the diversion is established by the record",
                      evidence=[evidence], submitted_at=at + DAY),
        file_argument(arch, respondent, d, "Party",
                      "the allocation was ambiguous and the complaint is malice",
                      submitted_at=at + DAY),
    ] + [
        file_argument(arch, m, d, "Panelist",
                      f"panelist {i}: the complainant overreads an ambiguity",
                      submitted_at=at + DAY)
        for i, m in enumerate(panel)
    ]
    return d, decide_mediation(arch, d, panel, outcome, "so decided", args,
                              decided_at=at + 2 * DAY)


def read(arch, t0, label):
    al = erosion_alert(arch, t0 + 200 * DAY,
                       attestation_cadence_seconds=400 * DAY,
                       grievance_cadence_seconds=CADENCE)
    print(f"  {label}")
    print(f"      erosion alert           {al.severity}")
    for r in al.reasons:
        print(f"        reason: {r}")
    for g in al.unanswered_grievances:
        print(f"        {g.ground}/{g.severity}, {g.seconds_since // DAY} days: "
              f"pursuits={g.pursuits} dismissals={g.dismissals} "
              f"repeat_panelists={g.repeat_panelists} attested={g.attested}")
    print(f"      other instruments       "
          f"silent={len(al.silent_functions)} unstaffed={len(al.unstaffed_functions)} "
          f"convergence={al.enforcement_convergence is not None} "
          f"direction={al.enforcement_direction is not None} "
          f"overdue_reexaminations={len(al.overdue_reexaminations)} "
          f"thinning={al.thinning_deliberation is not None}")
    return al


def main() -> int:
    print("§5 non-pursuit: the same evidence, three compacts\n")

    # 1. The compact that prosecutes.
    up, t_up, m, panel = compact()
    ev = the_evidence(up, m[1], m[0], t_up + DAY)
    hear_it(up, m[1], m[0], ev, panel, t_up + 10 * DAY, "Upheld")
    up.verify_all()
    up.check_authority()

    # 2. The compact that buries it: the same evidence, dismissed four times, and every
    #    dismissal drawn from the same three seats.
    bur, t_bur, mb, panel_b = compact()
    ev_b = the_evidence(bur, mb[1], mb[0], t_bur + DAY)
    for k in range(4):
        hear_it(bur, mb[1], mb[0], ev_b, panel_b, t_bur + (10 + 8 * k) * DAY, "Dismissed")
    bur.verify_all()
    bur.check_authority()

    # 3. The compact with nothing to prosecute.
    calm, t_calm, _mc, _pc = compact()
    calm.verify_all()

    read(up, t_up, "prosecuted to an upheld decision")
    read(bur, t_bur, "buried: four dismissals, one panel")
    read(calm, t_calm, "calm: nothing to prosecute")

    # The member's own word for it. The record is not an accusation a faction can mint:
    # the authority pass checks the claim against the archive rather than taking it.
    print("\nthe complainant puts it on the record")
    _d, dec = hear_it(bur, mb[1], mb[0], ev_b, panel_b, t_bur + 50 * DAY, "Dismissed")
    att = attest_non_pursuit(
        bur, mb[1], mb[0].did, [ev_b],
        "five panels, the same three seats, and the diversion stands",
        ground="ClosedUnresolved", closing_decision=dec, filed_at=t_bur + 100 * DAY,
    )
    bur.verify_all()
    bur.check_authority()
    print(f"      attestation authorized: {att[:16]}…")
    g = unanswered_grievances(bur, t_bur + 200 * DAY, CADENCE)[0]
    print(f"      the reading now carries it: attested={g.attested} "
          f"dismissals={g.dismissals} repeat_panelists={g.repeat_panelists}")

    # A false claim is refused rather than recorded: the same attestation on the
    # NeverPursued ground, which the archive contradicts.
    false = attest_non_pursuit(
        bur, mb[1], mb[0].did, [ev_b], "nobody ever took it up",
        ground="NeverPursued", filed_at=t_bur + 101 * DAY,
    )
    why = dict(bur.authority_violations())[false]
    print(f"      and a false ground is refused: {why.split(':')[-1].strip()[:72]}…")

    # The two rules the same measurement produced.
    print("\ntwo refusals the measurement asked for")
    arch, t0, mm, _p = compact()
    e2 = the_evidence(arch, mm[1], mm[0], t0 + DAY)
    d2 = file_dispute(arch, mm[1], mm[0].did, "DisputeRegistered",
                      references=[e2], filed_at=t0 + 5 * DAY)
    file_ladder_transition(arch, d2, "DisputeRegistered", "RegisteredProtest", [mm[1]],
                           reasoning="escalated", transitioned_at=t0 + 6 * DAY)
    bystander = file_ladder_transition(
        arch, d2, "RegisteredProtest", "DisputeRegistered", [mm[2]],
        reasoning="this is a misunderstanding", transitioned_at=t0 + 7 * DAY)
    print(f"      a bystander de-escalating another's dispute: "
          f"{dict(arch.authority_violations())[bystander].split(':')[-1].strip()[:64]}…")

    arch2, t2, m2, panel2 = compact()
    e3 = the_evidence(arch2, m2[1], m2[0], t2 + DAY)
    d3 = file_dispute(arch2, m2[1], m2[0].did, "DisputeRegistered",
                      references=[e3], filed_at=t2 + 5 * DAY)
    climb = file_ladder_transition(
        arch2, d3, "DisputeRegistered", "RegisteredProtest", [m2[1]],
        reasoning="escalated", transitioned_at=t2 + 22 * DAY)
    args = [
        file_argument(arch2, m2[1], d3, "Party", "established", evidence=[e3],
                      submitted_at=t2 + 55 * DAY),
        file_argument(arch2, m2[0], d3, "Party", "ambiguous", submitted_at=t2 + 55 * DAY),
    ] + [
        file_argument(arch2, mx, d3, "Panelist", f"panelist {i}: no violation",
                      submitted_at=t2 + 56 * DAY)
        for i, mx in enumerate(panel2)
    ]
    decide_mediation(arch2, d3, panel2, "Dismissed", "dismissed", args,
                     decided_at=t2 + 60 * DAY)
    arch2.verify_all()
    still_good = climb not in dict(arch2.authority_violations())
    print(f"      a climb dated day 22, after a dismissal dated day 60: "
          f"{'still authorized' if still_good else 'RETROACTIVELY REFUSED'}")
    print("\nauthority checks passed on every archive above.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
