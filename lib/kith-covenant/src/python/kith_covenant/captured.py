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

"""The captured prosecution: a compact that will not move against one of its own.

The inverse of the [determined adversary](adversary.py), and the harder case. There
the compact is sound and the member is not, and every question is about what a status
*does* once it has been imposed. Here the member is guilty, the evidence is filed, and
the majority is complicit. Nothing turns on detection. What it turns on is whether a
compact declining to move against one member looks any different, on any instrument the
protocol has, from a compact with nothing to move against.

The fixture is a compact of seven: two loyal members, five complicit, a designation
quorum of four, and a §4 panel of three seated by three complicit communities that are
party to nothing. **Capture is legal by construction here.** The question is never
whether the protocol can forbid a compact from being wrong, which it cannot, but whether
being wrong leaves a mark.

Six probes, each stating its criterion as a constant before the run, authoring every act
through the public facades, and running counterfactuals on forked archives so a probe
never contaminates the record the next one reads. The narrative reading of what they
found is `doc/architecture/captured-prosecution.md`; the regression tests are
`src/python/tests/test_non_pursuit.py` and `kc-reputation::tests::non_pursuit`.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from kith_covenant.ceremony import Archive
from kith_covenant.authoring import (
    new_identity, new_mediator, register_mediators, record_admission,
    record_conduct, file_dispute, file_argument, decide_mediation,
    file_ladder_transition, decide_designation, now_seconds,
)
from kith_covenant.reputation import (
    erosion_alert, unanswered_grievances, enforcement_direction,
    reporter_targeting, reputation_of,
)


DAY = 24 * 3600
# §5's non-pursuit cadence: how long a filed record may sit with no proceeding
# against it before the reading counts it.
CADENCE = 90 * DAY
MEDIATOR_TERM = 730 * DAY
# Held long so the *silence* readings stay off and what the alerts differ on is the
# dimension under test rather than the fixture's own youth.
ATTESTATION_CADENCE = 400 * DAY

# --- Falsification criteria, stated in advance -------------------------------
#
# Where a criterion is 0 it is an absolute: not a bound to stay under but a thing that
# must never happen once.

# Probe 1. Registration, protest, reciprocal suspension, arbitration and limited
# sanctions all authorize on the complainant's own signature: five rungs of seven with
# no help from any office at all.
RUNGS_WITHOUT_OFFICE = 5
# And then it stops. Collective sanctions need the initiating floor and a designation
# needs the quorum, and the two loyal members are below both.
SEAM_REFUSALS = 2

# Probe 2. Burial must not read as prosecution, and must not read as calm. This is the
# one the unmodified stack failed: all three arms graded `None` with no reasons.
ARMS_THAT_MUST_SEPARATE = True

# Probe 3. Four dismissals of one record by panels drawn from the same three seats.
# A count rather than a threshold: a small compact has few mediators and some
# repetition follows from arithmetic rather than from intent.
EXPECTED_PURSUITS = 4
EXPECTED_DISMISSALS = 4
MIN_REPEAT_SEATS = 3

# Probe 4. A de-escalation is the complainant's concession, not a veto held by a
# bystander, and what the quorum imposed only that quorum lifts.
BYSTANDER_DEESCALATIONS_AUTHORIZED = 0
RESPONDENT_DEESCALATIONS_AUTHORIZED = 0

# Probe 5. A captured panel may end a proceeding, which is its office. It may not
# unmake the account the complainant made of having brought one.
RETROACTIVE_UNAUTHORIZATIONS = 0

# Probe 6. The complicit majority holds the weight to run the honest member up the
# ladder on its own signature. Measured before the reporter-targeting reading existed,
# doing exactly that moved zero instruments.
RETALIATION_MUST_BE_LEGIBLE = True
# And the control on the same axis: an ordinary quarrel, at the same volume, where a
# minority of the acts answer a matter their subject had raised, must stay silent.
ORDINARY_RECIPROCITY_MUST_STAY_SILENT = True


@dataclass(frozen=True)
class ProbeResult:
    index: int
    key: str
    name: str
    # The criterion, in words, as stated before the run.
    criterion: str
    metrics: dict
    passed: bool
    notes: str
    # What the captured compact got away with in this probe, in words.
    unanswered: tuple[str, ...] = ()


@dataclass(frozen=True)
class CapturedReport:
    probes: tuple[ProbeResult, ...]
    passed: int
    total: int
    unanswered: tuple[str, ...] = ()

    @property
    def all_passed(self) -> bool:
        return self.total > 0 and self.passed == self.total

    def by_key(self, key: str) -> ProbeResult:
        for pr in self.probes:
            if pr.key == key:
                return pr
        raise KeyError(key)


@dataclass(frozen=True)
class CapturedParams:
    """The fixture, in one object so a run is never adrift from it."""

    founding_size: int = 7
    # Two loyal members. Everyone else is complicit, including the subject.
    loyal: int = 2
    # §5's designation quorum and the initiating floor for collective sanctions.
    quorum_threshold: int = 4
    panel_min: int = 3
    dismissals: int = 4


class _Probe:
    """One fixture, six probes. Forked copies are used wherever a probe has to attempt
    something the compact would not actually do."""

    def __init__(self, params: CapturedParams) -> None:
        self.p = params
        self.t0 = now_seconds()
        self.results: list[ProbeResult] = []
        self.arch, self.members, self.panel = self._fixture()
        # A: the shielded member. B, C: loyal. D..G: the complicit majority.
        self.shielded = self.members[0]
        self.honest = self.members[1]
        self.second = self.members[2]
        self.complicit = self.members[3:]

    # --- fixture -------------------------------------------------------------

    def day(self, n: float) -> int:
        return int(self.t0 + n * DAY)

    def _fixture(self, n: Optional[int] = None):
        """A sealed compact of `n`, so `shares_a_compact` holds, with §4's panel seated
        by the last three members. None of them is ever a party, so no recusal rule
        touches them and the burial is legal all the way down."""
        n = n or self.p.founding_size
        arch = Archive()
        members = [new_identity(arch, chr(ord("A") + i), self.t0) for i in range(n)]
        record_admission(
            arch, members, [m.did for m in members],
            compact_label="compact", witnessed_at=self.t0,
        )
        panel = []
        for home in members[-self.p.panel_min:]:
            med = new_mediator()
            register_mediators(
                arch, home, [med],
                valid_from=self.t0, valid_until=self.t0 + MEDIATOR_TERM,
            )
            panel.append(med)
        return arch, members, panel

    def fork(self, arch=None):
        return Archive.load_json((arch or self.arch).dump_json())

    def violations(self, arch=None) -> dict:
        return dict((arch or self.arch).authority_violations(
            quorum_threshold=self.p.quorum_threshold, panel_min=self.p.panel_min,
        ))

    def refusal(self, cid: str, arch=None) -> Optional[str]:
        """The reason `cid` was refused, or None if it stands."""
        return self.violations(arch).get(cid)

    def adverse(self, arch, reporters, subject_did, at, *, rung=None,
                kind="Violation", narrative=None):
        return record_conduct(
            arch, reporters, subject_did, kind, "CommonsObligation",
            severity="Serious", ladder_rung=rung,
            narrative=narrative or "diverted a shared allocation and denied it",
            recorded_at=at,
        )

    def dismiss(self, arch, complainant, respondent, evidence, at,
                outcome="Dismissed"):
        """A dispute citing `evidence`, closed by the §4 panel."""
        d = file_dispute(
            arch, complainant, respondent.did, "DisputeRegistered",
            summary="the diversion", references=list(evidence), filed_at=at,
        )
        args = [
            file_argument(arch, complainant, d, "Party",
                          "the record establishes it", evidence=list(evidence),
                          submitted_at=at + DAY),
            file_argument(arch, respondent, d, "Party",
                          "the allocation was ambiguous", submitted_at=at + DAY),
        ] + [
            file_argument(arch, m, d, "Panelist",
                          f"panelist {i}: the complainant overreads it",
                          submitted_at=at + DAY)
            for i, m in enumerate(self.panel)
        ]
        dec = decide_mediation(arch, d, self.panel, outcome, "so decided", args,
                               decided_at=at + 2 * DAY)
        return d, dec

    def alert(self, arch, at):
        return erosion_alert(
            arch, at,
            attestation_cadence_seconds=ATTESTATION_CADENCE,
            grievance_cadence_seconds=CADENCE,
        )

    def record(self, result: ProbeResult) -> ProbeResult:
        self.results.append(result)
        return result

    # --- probe 1 -------------------------------------------------------------

    def where_escalation_stops(self) -> ProbeResult:
        arch, _t = self.fork(), None
        m = self.members
        ev = self.adverse(arch, m[1], m[0].did, self.day(1), rung="RegisteredProtest")
        d = file_dispute(arch, m[1], m[0].did, "DisputeRegistered",
                         references=[ev], filed_at=self.day(5))
        climb = [
            ("DisputeRegistered", "RegisteredProtest"),
            ("RegisteredProtest", "ReciprocalSuspension"),
            ("ReciprocalSuspension", "MediatedArbitration"),
            ("MediatedArbitration", "LimitedSanctions"),
        ]
        # The filing is itself the first rung, so the count is the registration plus
        # the four climbs above it.
        alone = 1 if self.refusal(d, arch) is None else 0
        for i, (frm, to) in enumerate(climb):
            cid = file_ladder_transition(arch, d, frm, to, [m[1]],
                                         reasoning="up",
                                         transitioned_at=self.day(6 + i))
            if self.refusal(cid, arch) is None:
                alone += 1
        rung = reputation_of(arch, m[1].did, as_of=self.day(11)) \
            .disputes_filed[0].current_rung

        # The seam. Two loyal members, at the two acts that need the compact.
        collective = file_ladder_transition(
            arch, d, "LimitedSanctions", "CollectiveSanctions", [m[1], m[2]],
            decision_class="SecondClass", reasoning="the two of us",
            transitioned_at=self.day(12),
        )
        designation = decide_designation(
            arch, m[0].did, [m[1], m[2]], "a course of diversion",
            cited_acts=[ev], reversal_conditions="restitution and a year",
            deliberation_cycles=[
                (0, [m[1].did], self.day(12), self.day(13), "first"),
                (1, [m[1].did], self.day(25), self.day(26), "second"),
            ],
            designated_at=self.day(30),
        )
        refusals = [self.refusal(c, arch) for c in (collective, designation)]
        refused = sum(1 for r in refusals if r is not None)
        # Registration through limited sanctions is the wronged party's own
        # instrument; the two acts above are the compact's.
        passed = (alone == RUNGS_WITHOUT_OFFICE
                  and rung == "LimitedSanctions"
                  and refused == SEAM_REFUSALS)
        return self.record(ProbeResult(
            index=1, key="seam", name="where escalation stops",
            criterion=(f"the honest complainant climbs {RUNGS_WITHOUT_OFFICE} rungs on "
                       f"its own signature and is refused at both of the "
                       f"{SEAM_REFUSALS} acts that need the compact"),
            metrics={
                "rungs_climbed_alone": alone,
                "rung_reached": rung,
                "seam_refusals": refused,
                "collective_refusal": refusals[0],
                "designation_refusal": refusals[1],
            },
            passed=passed,
            notes=("capture severs the ladder exactly at the seam where it turns from "
                   "an instrument of a party into one of the compact"),
            unanswered=(
                "the honest minority cannot designate: it is below the quorum, and "
                "the quorum is the protection",
            ),
        ))

    # --- probe 2 -------------------------------------------------------------

    def burial_prosecution_and_calm(self) -> ProbeResult:
        """Three archives, the same evidence in each, only the compact differing."""
        def arm(outcome=None):
            arch, members, _panel = self._fixture()
            self.panel_of = _panel
            if outcome is None:
                arch.verify_all()
                return arch
            ev = self.adverse(arch, members[1], members[0].did, self.day(1),
                              rung="RegisteredProtest")
            saved, self.panel = self.panel, _panel
            self.dismiss(arch, members[1], members[0], [ev], self.day(10),
                         outcome=outcome)
            self.panel = saved
            arch.verify_all()
            arch.check_authority(quorum_threshold=self.p.quorum_threshold,
                                 panel_min=self.p.panel_min)
            return arch

        buried, prosecuted, calm = arm("Dismissed"), arm("Upheld"), arm()
        at = self.day(200)
        ab, ap, ac = (self.alert(a, at) for a in (buried, prosecuted, calm))
        grievances = tuple(len(unanswered_grievances(a, at, CADENCE))
                           for a in (buried, prosecuted, calm))
        # Every *other* instrument, on the buried archive: the finding is that they
        # are all inert, which is why the non-pursuit reading had to exist.
        others_inert = (
            ab.silent_functions == () and ab.unstaffed_functions == ()
            and ab.enforcement_convergence is None
            and ab.enforcement_direction is None
            and ab.reporter_targeting is None
            and ab.overdue_reexaminations == () and ab.thinning_deliberation is None
        )
        passed = (ab.severity != ap.severity and ap.severity == "None"
                  and ac.severity == "None" and grievances == (1, 0, 0)
                  and others_inert) == ARMS_THAT_MUST_SEPARATE
        return self.record(ProbeResult(
            index=2, key="arms", name="burial, prosecution and calm",
            criterion="burial reads as neither prosecution nor calm",
            metrics={
                "severities": (ab.severity, ap.severity, ac.severity),
                "grievances": grievances,
                "buried_reasons": ab.reasons,
                "every_other_dimension_inert_on_burial": others_inert,
            },
            passed=passed,
            notes=("burial PRODUCES observances: a panel sat, arguments were filed and "
                   "answered, a decision was recorded, so every instrument that counts "
                   "observances read the captured compact as a diligent one"),
        ))

    # --- probe 3 -------------------------------------------------------------

    def the_burial_signature(self) -> ProbeResult:
        arch, members, panel = self._fixture()
        saved, self.panel = self.panel, panel
        ev = self.adverse(arch, members[1], members[0].did, self.day(1))
        for k in range(self.p.dismissals):
            self.dismiss(arch, members[1], members[0], [ev],
                         self.day(10 + 5 * k))
        self.panel = saved
        arch.verify_all()
        arch.check_authority(quorum_threshold=self.p.quorum_threshold,
                             panel_min=self.p.panel_min)
        g = unanswered_grievances(arch, self.day(200), CADENCE)
        one = g[0] if g else None
        passed = bool(one) and (one.ground == "ClosedUnresolved"
                                and one.pursuits == EXPECTED_PURSUITS
                                and one.dismissals == EXPECTED_DISMISSALS
                                and one.repeat_panelists >= MIN_REPEAT_SEATS)
        return self.record(ProbeResult(
            index=3, key="signature", name="the burial signature",
            criterion=(f"{EXPECTED_DISMISSALS} dismissals of one record show "
                       f"{EXPECTED_PURSUITS} pursuits and at least "
                       f"{MIN_REPEAT_SEATS} repeat seats"),
            metrics={
                "ground": one.ground if one else None,
                "pursuits": one.pursuits if one else 0,
                "dismissals": one.dismissals if one else 0,
                "repeat_panelists": one.repeat_panelists if one else 0,
            },
            passed=passed,
            notes=("a compact whose panels close the same evidence again and again out "
                   "of the same few seats is a different object from one whose "
                   "independent panels each reached the same conclusion"),
            unanswered=(
                "the re-file loop is unbounded: a dismissal ends a proceeding and the "
                "same panel may dismiss the next filing on the same evidence",
            ),
        ))

    # --- probe 4 -------------------------------------------------------------

    def a_bystander_is_not_a_mover(self) -> ProbeResult:
        arch, members, panel = self._fixture()
        saved, self.panel = self.panel, panel
        ev = self.adverse(arch, members[1], members[0].did, self.day(1),
                          rung="RegisteredProtest")
        d = file_dispute(arch, members[1], members[0].did, "DisputeRegistered",
                         references=[ev], filed_at=self.day(5))
        for i, (frm, to) in enumerate((
            ("DisputeRegistered", "RegisteredProtest"),
            ("RegisteredProtest", "ReciprocalSuspension"),
            ("ReciprocalSuspension", "MediatedArbitration"),
            ("MediatedArbitration", "LimitedSanctions"),
        )):
            file_ladder_transition(arch, d, frm, to, [members[1]],
                                   reasoning="up", transitioned_at=self.day(6 + i))
        self.panel = saved

        # A complicit community that is party to neither side, walking it back.
        by = self.fork(arch)
        bystander = file_ladder_transition(
            by, d, "LimitedSanctions", "DisputeRegistered", [members[3]],
            reasoning="we see no need for this",
            transitioned_at=self.day(20),
        )
        # The accused reducing its own sanction.
        rp = self.fork(arch)
        respondent = file_ladder_transition(
            rp, d, "LimitedSanctions", "DisputeRegistered", [members[0]],
            reasoning="I concede nothing and withdraw everything",
            transitioned_at=self.day(20),
        )
        # The control that keeps the rule from being a prohibition on descent: the
        # complainant's own concession stands.
        ok = self.fork(arch)
        conceded = file_ladder_transition(
            ok, d, "LimitedSanctions", "MediatedArbitration", [members[1]],
            reasoning="we will try arbitration",
            transitioned_at=self.day(20),
        )
        by_refused = self.refusal(bystander, by)
        rp_refused = self.refusal(respondent, rp)
        conceded_refused = self.refusal(conceded, ok)
        passed = (by_refused is not None and rp_refused is not None
                  and conceded_refused is None)
        return self.record(ProbeResult(
            index=4, key="deescalation", name="a bystander is not a mover",
            criterion=("no de-escalation authorizes without the complainant among its "
                       "initiators, and the complainant's own concession still does"),
            metrics={
                "bystander_authorized":
                    int(by_refused is None),
                "respondent_authorized":
                    int(rp_refused is None),
                "complainant_concession_authorized": int(conceded_refused is None),
                "bystander_refusal": by_refused,
            },
            passed=passed,
            notes=("the decision-class check read the rung being ENTERED, which guarded "
                   "the imposing of collective sanctions and left their removal "
                   "unguarded: two communities to expel and one to un-expel"),
        ))

    # --- probe 5 -------------------------------------------------------------

    def a_closing_decision_does_not_reach_backwards(self) -> ProbeResult:
        arch, members, panel = self._fixture()
        saved, self.panel = self.panel, panel
        ev = self.adverse(arch, members[1], members[0].did, self.day(1),
                          rung="RegisteredProtest")
        d = file_dispute(arch, members[1], members[0].did, "DisputeRegistered",
                         references=[ev], filed_at=self.day(20))
        climbed = []
        for i, (frm, to) in enumerate((
            ("DisputeRegistered", "RegisteredProtest"),
            ("RegisteredProtest", "ReciprocalSuspension"),
            ("ReciprocalSuspension", "MediatedArbitration"),
        )):
            climbed.append(file_ladder_transition(
                arch, d, frm, to, [members[1]], reasoning="up",
                transitioned_at=self.day(22 + 4 * i)))
        # The captured panel ends the proceeding, forty days after the last climb.
        args = [
            file_argument(arch, members[1], d, "Party", "the record establishes it",
                          evidence=[ev], submitted_at=self.day(58)),
            file_argument(arch, members[0], d, "Party", "it was ambiguous",
                          submitted_at=self.day(58)),
        ] + [
            file_argument(arch, m, d, "Panelist", f"panelist {i}: no",
                          submitted_at=self.day(58))
            for i, m in enumerate(panel)
        ]
        decide_mediation(arch, d, panel, "Dismissed", "so decided", args,
                         decided_at=self.day(60))
        self.panel = saved
        retro = sum(1 for c in climbed if self.refusal(c, arch) is not None)
        # And the control: a transition attempted AFTER the closing is refused, or the
        # rule above would have made a closed dispute climbable.
        after = file_ladder_transition(
            arch, d, "MediatedArbitration", "LimitedSanctions", [members[1]],
            reasoning="up again", transitioned_at=self.day(70))
        after_refused = self.refusal(after, arch)
        passed = (retro == RETROACTIVE_UNAUTHORIZATIONS
                  and after_refused is not None)
        return self.record(ProbeResult(
            index=5, key="retroactivity", name="a closing decision reaches forwards only",
            criterion=("a dismissal unauthorizes no transition that preceded it, and "
                       "authorizes none that follows it"),
            metrics={
                "retroactive_unauthorizations": retro,
                "transitions_before_the_closing": len(climbed),
                "after_the_closing_refused": after_refused is not None,
            },
            passed=passed,
            notes=("a captured panel may end a proceeding, which is its office; it may "
                   "not unmake the account the complainant made of having brought one"),
        ))

    # --- probe 6 -------------------------------------------------------------

    def the_ladder_turned_on_the_complainant(self) -> ProbeResult:
        """The complicit majority answers the accusation by prosecuting the accuser."""
        def compact_with(counter: int, prior: bool, named=True):
            """`counter` enforcement acts against the honest member, of which the ones
            answering a matter it had raised are however many `prior` allows.

            With `named` false the answer is brought by allies the grievance never
            named, which is the proxy the reading cannot see."""
            arch, members, _panel = self._fixture()
            honest, shielded = members[1], members[0]
            allies = members[3:]
            if prior:
                # The honest member puts the diversion on the record, naming the
                # shielded member.
                self.adverse(arch, honest, shielded.did, self.day(1),
                             rung="RegisteredProtest")
            bringers = ([shielded, allies[0], allies[1]] if named
                        else [allies[0], allies[1], allies[2]])
            for k in range(counter):
                # The answer: an enforcement act against the accuser, brought by the
                # community it named together with two of its allies.
                self.adverse(
                    arch, bringers, honest.did,
                    self.day(20 + 5 * k), rung="ReciprocalSuspension",
                    kind="Violation",
                    narrative="obstructed the compact's business with the same filing",
                )
            arch.verify_all()
            arch.check_authority(quorum_threshold=self.p.quorum_threshold,
                                 panel_min=self.p.panel_min)
            return arch

        retaliation = compact_with(6, prior=True)
        # The control at the same volume, differing on one axis: the subject raised
        # nothing first, so these are six enforcement acts and nothing more.
        unprovoked = compact_with(6, prior=False)
        # And the gap, measured rather than asserted: the same answer to the same
        # grievance, brought by three allies the grievance never named.
        proxy = compact_with(6, prior=True, named=False)
        at = self.day(200)

        r_t = reporter_targeting(retaliation, at)
        u_t = reporter_targeting(unprovoked, at)
        p_t = reporter_targeting(proxy, at)
        # The lens that cannot make this reading, on the same archive.
        r_d = enforcement_direction(retaliation, at)
        ar, au = self.alert(retaliation, at), self.alert(unprovoked, at)

        # Every one of the counter-acts is counted. The denominator carries one act
        # more than the numerator can: the grievance being answered is itself an
        # enforcement act inside the window, and it answers nothing.
        legible = (ar.reporter_targeting is not None
                   and r_t.against_reporters == 6
                   and r_t.acts == 7
                   and any("complainants" in s for s in ar.reasons))
        silent = (au.reporter_targeting is None and u_t.against_reporters == 0)
        # Reported, not scored: the proxy is the reading's stated limit, and a probe
        # that failed on its own limit would be filing a defect against a design
        # decision.
        proxy_seen = p_t.against_reporters > 0
        passed = (legible == RETALIATION_MUST_BE_LEGIBLE
                  and silent == ORDINARY_RECIPROCITY_MUST_STAY_SILENT)
        return self.record(ProbeResult(
            index=6, key="retaliation", name="the ladder turned on the complainant",
            criterion=("a counter-prosecution is legible, and the same volume of "
                       "enforcement answering nothing is not"),
            metrics={
                "acts": r_t.acts,
                "against_reporters": r_t.against_reporters,
                "share": r_t.share,
                "pairs": r_t.pairs,
                "control_against_reporters": u_t.against_reporters,
                "proxy_against_reporters": p_t.against_reporters,
                "proxy_seen": proxy_seen,
                "direction_downward_upward_level":
                    (r_d.downward, r_d.upward, r_d.level),
                "severity": (ar.severity, au.severity),
            },
            passed=passed,
            notes=("`enforcement_direction` buckets by STANDING, and an honest "
                   "complainant holds full standing, so it read this as enforcement "
                   "between equals; the axis had to be the relation between an act and "
                   "a prior grievance"),
            unanswered=(
                "a proxy prosecution is not seen: the reading joins on the pair the "
                "grievance names, so allies who were never named may bring the answer",
            ),
        ))


PROBES = (
    "where_escalation_stops",
    "burial_prosecution_and_calm",
    "the_burial_signature",
    "a_bystander_is_not_a_mover",
    "a_closing_decision_does_not_reach_backwards",
    "the_ladder_turned_on_the_complainant",
)


def run_probe(params: Optional[CapturedParams] = None) -> CapturedReport:
    """Run the six probes over the captured fixture and score each.

    Deterministic at fixed parameters. Fresh Ed25519 keys are generated per run and
    several lenses tiebreak on DID ordering, so a report that varies between runs is
    reporting a key-ordering dependency rather than anything about capture."""
    probe = _Probe(params or CapturedParams())
    for name in PROBES:
        getattr(probe, name)()
    probes = tuple(probe.results)
    return CapturedReport(
        probes=probes,
        passed=sum(1 for pr in probes if pr.passed),
        total=len(probes),
        unanswered=tuple(u for pr in probes for u in pr.unanswered),
    )


def format_report(report: CapturedReport) -> str:
    """The probe as a readable transcript: one block each, then the tally."""
    lines = []
    for pr in report.probes:
        mark = "PASS" if pr.passed else "FAIL"
        lines.append(f"[{mark}] probe {pr.index}: {pr.name}")
        lines.append(f"        criterion: {pr.criterion}")
        for key, value in pr.metrics.items():
            lines.append(f"        {key}: {value}")
        lines.append(f"        note: {pr.notes}")
        for item in pr.unanswered:
            lines.append(f"        UNANSWERED: {item}")
        lines.append("")
    lines.append(f"{report.passed}/{report.total} probes passed")
    if report.unanswered:
        lines.append(f"{len(report.unanswered)} thing(s) the captured compact still "
                     f"gets away with:")
        for item in report.unanswered:
            lines.append(f"  - {item}")
    return "\n".join(lines)
