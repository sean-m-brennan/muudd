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

"""The determined adversary: one deceitful actor, prosecuted end to end.

The §10 harness (``harness.py``) scores twelve scenarios, and every one of them is
a **single signal**: an adversary takes one kind of action and a lens must flag it
within a step. This module runs the other shape, the one the doctrine cares most
about and the one nothing here had ever run: a **campaign**. One community joins a
standing compact by passing a rite it does not believe, leaks its orientation,
is prosecuted up the whole §5 ladder, surrenders whenever surrender is cheaper
than resistance and re-violates the moment it is not, is designated anathema,
earns its way back through the re-entry rite, betrays that too, and is finally
designated *hostis humani generis* (``justus_hostis.md`` "Two tiers";
``bounded_conflict.md`` "Handling the anathema").

Nine phases over **one** archive, in the order a compact would actually meet them.
Each phase states its criterion in advance (the module constants below), authors
its acts through the public facades only, and reports what it measured. A phase
that fails is not a bug report about this module: it is a claim about the protocol,
and the claim is what the regression tests in ``tests/test_adversary_campaign.py``
pin down.

## What a campaign measures that a single-signal scenario cannot

Three things, all of them about *time* and *sequence* rather than detection:

- **Tactical compliance.** A restitution cycle accepted and re-violated is legible
  only as a pattern. One is a failure, two is bad luck, and §5's condition 1 says
  three is a designation. The campaign is what shows the arithmetic biting at
  exactly the floor and not before it.
- **What a status does once it is held.** Detection is not response. A designation
  that no rule reads is a label, and only a campaign that keeps acting *after* the
  designation can tell the difference between a status and a consequence.
- **Whether the compact holds together.** A designation is the most dangerous act
  in the life of a compact, so what matters is not only that it can be taken but
  who took it, who refused, and whether the instruments that watch for capture can
  tell legitimate prosecution of one real adversary from a spiral. The negative
  control in phase 9 is as important as any refusal: an alert that fires on a
  compact doing its job correctly is worse than no alert.

## Two readings

The campaign is written to run against the protocol as it stands, so the phases
that fail against unmodified code *are the measurement*, and the same phases run
green once the rules they name exist. Nothing in this module is skipped or
special-cased for either reading. ``run_campaign()`` returns both the per-phase
verdicts and an ``unanswered`` tally, which is the list of things the adversary got
away with.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from kith_covenant.ceremony import Archive
from kith_covenant.authoring import (
    OFFICE_CAPABILITIES, SERVICE_CAPABILITIES,
    new_identity, new_examiner, new_mediator, new_observer,
    register_examiners, register_mediators, register_observers,
    record_admission, vouch, run_intra_rite, record_conduct, record_concern,
    publish_observer_report, ratify_constitution, amend_compact_incapable,
    file_dispute, file_argument, decide_mediation, file_ladder_transition,
    issue_injunction, confirm_injunction,
    impose_restitution, verify_restitution,
    decide_designation, file_designation_dissent, complete_reentry, attest_activity,
    now_seconds,
)
from kith_covenant.reputation import (
    reputation_of, has_capability, sponsor_stake, erosion_alert,
    enforcement_convergence, enforcement_direction, overdue_reexaminations,
    designation_consensus,
)


DAY = 24 * 3600
YEAR = 365 * DAY
# §5's deliberation waiting period (DEFAULT_WAITING_PERIOD_SECONDS).
WAITING = 7 * DAY
# §4 caps a mediator's seat at two years, §2 an examiner's at half a year, §8 an
# observer's at one. The campaign spans about 430 days, so every seat it needs is
# inside one term and no rotation sit-out is involved.
MEDIATOR_TERM = 730 * DAY
EXAMINER_TERM = 180 * DAY
OBSERVER_TERM = YEAR

# The five movements, recited truthfully by every community here including the one
# that means none of it. That is the point of phase 1: no rule reads an interior.
MOVEMENTS = (
    "We anchor above ourselves.",
    "We are fallen, ourselves included.",
    "We recognize those who anchor above themselves.",
    "We hold the common virtues.",
    "This compact is not our ultimate.",
)

# --- Falsification criteria, stated in advance -------------------------------
#
# Where a criterion is 0 or False it is an absolute: not a bound to stay under but
# a thing that must never happen once. Where it is a count it is the doctrine's own
# number, and the campaign checks it from both sides — refused below the floor,
# authorized at it.

# Phase 1. The deceit works. An admission is a covenant-facing act and the compact
# forms no verdict on an interior (natural_law.md "The interior of a member
# community"), so a sincere-looking entrant is admitted, and the campaign says so
# rather than pretending the protocol screens for belief.
MAX_ADMISSION_REFUSALS = 0
# What a probationary entrant may carry toward a second-class act: nothing (§2).
MAX_PROBATIONARY_SECOND_CLASS = 0

# Phase 2. Steps between the first covenant-facing adverse act and its being
# visible to a reader.
MAX_LEAK_LATENCY_STEPS = 1

# Phase 3. Illegal ladder moves that validate.
MAX_ILLEGAL_LADDER_MOVES = 0

# Phase 4. Office acts by a community held out of office by an in-force injunction,
# and services it keeps while so held (the split is the whole point of §9's two
# kinds).
MAX_OFFICE_ACTS_UNDER_INJUNCTION = 0
MIN_SERVICES_KEPT_UNDER_OFFICE_INJUNCTION = len(SERVICE_CAPABILITIES)

# Phase 5. §5 condition 1: three completed-then-re-violated restoration cycles,
# checked from both sides. Two must be refused and three must be authorized, so the
# floor is measured rather than assumed.
SURRENDER_ROUNDS_BEFORE_DESIGNATION = 3

# Phase 6. What a standing designation withholds. A designated community is
# formally cut off from fellowship; holding an office that selects who is
# sanctioned, or a vote in the compact's gravest decisions, is not a thing a
# cut-off member does.
MAX_OFFICE_ACTS_UNDER_DESIGNATION = 0
MAX_SECOND_CLASS_ACTS_ON_DESIGNATED_WEIGHT = 0
# "No single sponsor cluster can admit alone" is emptiest when the cluster is the
# community the act is about (§2's reasoning, one component over).
MAX_SELF_QUORUM_DESIGNATIONS = 0
# The ordinary restitution path does not apply to a designated actor: "standard
# restitution-and-return is suspended because the pattern of conduct has
# demonstrated that the actor uses these paths tactically" (bounded_conflict.md
# "The differential protocol"). Re-entry is the only way back.
MAX_ORDINARY_RESTITUTION_UNDER_DESIGNATION = 0

# Phase 7. A re-entry opens a probation window, and probation withholds weight.
MAX_SECOND_CLASS_ACTS_ON_PROBATIONARY_WEIGHT = 0

# Phase 8. The *hostis humani generis* tier "requires an even higher evidentiary
# threshold. The compact must be able to demonstrate that the orientation of the
# actor is not merely currently destructive but *constitutively* so"
# (justus_hostis.md "Design implications"). Three absolutes follow: the tier is not
# reachable without a prior designation whose return path failed, not reachable on
# its predecessor's evidence alone, and not reachable at the ordinary quorum.
MAX_HHG_WITHOUT_FAILED_RETURN = 0
MAX_HHG_ON_RECYCLED_EVIDENCE = 0
MAX_HHG_AT_THE_ORDINARY_QUORUM = 0
# "Public deliberation, slow procedure, periodic re-examination, willingness to be
# wrong about past designations" are named as the guards against the response
# becoming demonic, "and especially" for this tier. An irreversible designation
# nobody is obliged to look at again has the fewest guards of any act in §5.
MAX_HHG_WITHOUT_REEXAMINATION = 0

# Phase 9. A member that considered the designation and refused to join it must
# leave a trace, or the compact's agreement is unknowable and a designation carried
# by two reads exactly like one carried by all.
MAX_UNRECORDABLE_REFUSALS = 0
# The negative control, and it must hold in **both** readings: the capture
# instruments must stay silent on a compact prosecuting one genuine adversary.
# `target_dispersion` is the discriminator (see erosion-legibility.md finding K5):
# legitimate concentration lands on a target, substitution concentrates a ground
# across many targets.
EROSION_ALERT_ON_LEGITIMATE_PROSECUTION = False


@dataclass(frozen=True)
class CampaignParams:
    """The campaign's fixture, in one object so a run is never adrift from it.

    Defaults are the canonical case: a compact of five, §5's quorum of two, §4's
    panel of three, the doctrine's three restoration cycles, and two members that
    refuse to join the final designation.
    """

    founding_size: int = 5
    quorum_threshold: int = 2
    panel_min: int = 3
    surrender_rounds: int = 3
    dissenters: int = 2
    # Wolf's sponsorship probation window, in days. It has to *close* before the
    # designation phases: a refusal that probation would have produced anyway
    # measures nothing about a designation.
    probation_days: int = 200


@dataclass(frozen=True)
class PhaseResult:
    index: int
    key: str
    name: str
    # The criterion, in words, as stated before the run.
    criterion: str
    # What the phase measured. Kept as a mapping so a phase can report more than
    # one number without inventing a field per phase.
    metrics: dict
    passed: bool
    notes: str
    # What the adversary got away with in this phase, in words. Empty when nothing.
    unanswered: tuple[str, ...] = ()


@dataclass(frozen=True)
class CampaignReport:
    phases: tuple[PhaseResult, ...]
    passed: int
    total: int
    params: CampaignParams
    # The whole campaign's tally of what went unanswered, phase order preserved.
    unanswered: tuple[str, ...] = ()
    # Where the escalation actually ended.
    reached_tier: Optional[str] = None

    @property
    def all_passed(self) -> bool:
        return self.total > 0 and self.passed == self.total

    def by_key(self, key: str) -> PhaseResult:
        for ph in self.phases:
            if ph.key == key:
                return ph
        raise KeyError(key)


class _Campaign:
    """One archive, carried through the nine phases in order.

    Phases share state deliberately: what phase 6 can measure depends on what
    phase 5 established, which is the whole reason this is a campaign and not nine
    scenarios. Forked copies (``Archive.load_json``) are used wherever a phase has
    to attempt something the compact would not actually do, so a counterfactual
    never contaminates the record the next phase reads.
    """

    def __init__(self, params: CampaignParams) -> None:
        self.p = params
        self.arch = Archive()
        self.t0 = now_seconds()
        self.results: list[PhaseResult] = []
        self._build_cast()

    # --- fixture -------------------------------------------------------------

    def day(self, n: float) -> int:
        return int(self.t0 + n * DAY)

    def _build_cast(self) -> None:
        p, arch, t0 = self.p, self.arch, self.t0
        names = [chr(ord("A") + i) for i in range(p.founding_size)]
        self.compact = [new_identity(arch, n, t0) for n in names]
        # The compact's formation: founders, not entrants, so §2 exempts it.
        record_admission(
            arch, self.compact, [c.did for c in self.compact],
            compact_label="compact", witnessed_at=t0,
        )
        # A constitution, so the compact has instruments to erode and a
        # second-class act (an amendment) available as a cheap probe of weight.
        ratify_constitution(
            arch, self.compact[: p.quorum_threshold],
            incapable=[("internal-theology", "d"), ("internal-governance", "d")],
            fast_periods=[("annual", self.day(300), self.day(302))],
            genesis_at=t0,
        )
        # The wronged party, the sponsor, the loyal core, and the dissenters.
        self.wronged = self.compact[1]                      # B
        self.sponsor = self.compact[0]                      # A
        self.loyal = self.compact[: p.founding_size - p.dissenters]
        self.dissenters = self.compact[p.founding_size - p.dissenters:]

        # §4 mediator pool: two disjoint panels of `panel_min`, each seated by its
        # own community so no panel is ever a party to anything the campaign runs.
        self.pool_a, self.med_a = self._seat_mediators("Pool-A", p.panel_min)
        self.pool_b, self.med_b = self._seat_mediators("Pool-B", p.panel_min)
        # §2 examining pool: one examiner each from three sitting communities, so
        # the credentialing panel spans three homes and none supplies a majority.
        self.examiners = []
        for com in self.compact[1:4]:
            ex = new_examiner()
            register_examiners(
                self.arch, com, [ex],
                valid_from=t0 - 10, valid_until=t0 - 10 + EXAMINER_TERM,
            )
            self.examiners.append(ex)
        # §8 observer corps, seated by two communities that are not the subject.
        self.observers = []
        for com in (self.compact[2], self.compact[3]):
            ob = new_observer()
            register_observers(
                self.arch, com, [(ob, "wary of newcomers")],
                valid_from=t0 - 10, valid_until=t0 - 10 + OBSERVER_TERM,
            )
            self.observers.append(ob)

        self.wolf: Optional[object] = None
        # Adverse acts beyond the ones a phase names, for volume (see
        # `_extra_offence`).
        self.extra_offences: list[str] = []

    def _seat_mediators(self, label: str, count: int):
        """`count` mediators, each seated by its own fresh non-party community."""
        homes, meds = [], []
        for i in range(count):
            home = new_identity(self.arch, f"{label}-{i}", self.t0)
            med = new_mediator()
            register_mediators(
                self.arch, home, [med],
                valid_from=self.t0 - 10, valid_until=self.t0 - 10 + MEDIATOR_TERM,
            )
            homes.append(home)
            meds.append(med)
        return homes, meds

    # --- helpers -------------------------------------------------------------

    def violations(self, arch=None) -> dict:
        arch = self.arch if arch is None else arch
        return dict(arch.authority_violations(
            quorum_threshold=self.p.quorum_threshold, panel_min=self.p.panel_min,
        ))

    def refusal(self, cid: str, arch=None) -> Optional[str]:
        """The reason `cid` was refused, or None if it stands."""
        return self.violations(arch).get(cid)

    def fork(self):
        """A copy of the archive for a counterfactual the compact would not run."""
        return Archive.load_json(self.arch.dump_json())

    def clean(self, arch=None) -> bool:
        """No authority violations at all: the record the next phase reads is sound."""
        return not self.violations(arch)

    def record(self, result: PhaseResult) -> PhaseResult:
        self.results.append(result)
        return result

    # --- Phase 1: infiltration ----------------------------------------------

    def phase1_infiltration(self) -> PhaseResult:
        """The adversary joins a standing compact, and the deceit is sufficient.

        Wolf publishes an identity, recites the five movements truthfully and means
        none of it, arrives vouched for by a sitting member, and is credentialed by
        a three-community examining panel. Every §2 rule is satisfied, because every
        §2 rule is about acts: the sponsor's stake, the admitting quorum, the
        examining panel's composition. None of them reads a belief, and
        ``natural_law.md`` ("The interior of a member community") is why the compact
        does not try to. So the criterion here is the *opposite* of a refusal: the
        admission must be authorized, and the campaign says so plainly rather than
        implying the gate screens for sincerity.

        What §2 does supply is the one thing that survives a successful deceit: the
        entrant carries no weight in any second-class decision until its probation
        window closes, and the sponsor's own standing now rides on its conduct.
        """
        p, arch = self.p, self.arch
        wolf = new_identity(arch, "Wolf", self.t0)
        self.wolf = wolf
        # The rite, run in full. Nothing distinguishes it from a sincere one.
        self.wolf_rite = run_intra_rite(
            arch, wolf, movements=list(MOVEMENTS), witness_count=3,
            sealed_at=self.day(1),
        )
        # The sponsor stakes its standing, which is what opens the probation window.
        self.wolf_vouch = vouch(
            arch, self.sponsor, wolf.did,
            probation_until=self.day(p.probation_days), sponsored_at=self.day(2),
        )
        party = self.compact + [wolf]
        self.wolf_admission = record_admission(
            arch, party + self.examiners, [c.did for c in party],
            compact_label="compact", witnessed_at=self.day(3),
        )
        arch.verify_all()
        refused = self.refusal(self.wolf_admission)
        refusals = 1 if refused else 0

        # §2's residue: a probationary entrant's weight, spent at once. The probe
        # runs on a fork, because a refused record left in the archive would make
        # every later phase's "the record is sound" check meaningless.
        probe = self.fork()
        early = amend_compact_incapable(
            probe, "Add", "wolf-early", [self.sponsor, wolf], decided_at=self.day(5))
        control = amend_compact_incapable(
            probe, "Add", "wolf-control", self.compact[:p.quorum_threshold],
            decided_at=self.day(6))
        pv = self.violations(probe)
        carried = 0 if early in pv else 1
        control_ok = control not in pv

        standing = reputation_of(arch, wolf.did, as_of=self.day(4)).standing
        passed = (
            refusals <= MAX_ADMISSION_REFUSALS
            and carried <= MAX_PROBATIONARY_SECOND_CLASS
            and control_ok
            and standing == "Probationary"
            and self.clean()
        )
        return self.record(PhaseResult(
            index=1, key="infiltration",
            name="admission by a rite the entrant does not believe",
            criterion="the admission is authorized (no rule reads an interior) and "
                      "the entrant carries no second-class weight",
            metrics={
                "admission_refusals": refusals,
                "second_class_acts_carried": carried,
                "control_authorized": control_ok,
                "standing": standing,
                "probation_until_day": p.probation_days,
            },
            passed=passed,
            notes="every §2 rule is about an act: sponsor stake, admitting quorum, "
                  "examining panel composition. A sincere-looking entrant passes all "
                  f"three ({refused or 'authorized'})",
            unanswered=() if refusals else (
                "the adversary is a member of the compact in good standing",
            ),
        ))

    # --- Phase 2: the leak ---------------------------------------------------

    def phase2_leak(self) -> PhaseResult:
        """The orientation surfaces, and the archive sees it the moment it does.

        A determined adversary eventually acts on what it is, and the first act that
        touches the covenant is the first thing the compact may read. Three readers
        are exercised: the subject's own conduct history, the §2 sponsor stake (the
        sponsor's standing was staked on exactly this), and the §8 observer corps,
        whose report the adversary is free to answer and cannot suppress. Latency is
        counted in steps from the act.
        """
        arch, wolf = self.arch, self.wolf
        before = reputation_of(arch, wolf.did, as_of=self.day(9))
        stake_before = sponsor_stake(arch, self.sponsor.did, as_of=self.day(9))
        adverse_before = sum(1 for c in before.conduct if c.kind in ("Concern", "Violation"))

        # Step 1: the covenant-facing violation.
        self.first_violation = record_conduct(
            arch, self.wronged, wolf.did, "Violation", "CommonsObligation",
            severity="Serious", ladder_rung="RegisteredProtest",
            narrative="diverted a shared allocation and denied the diversion",
            recorded_at=self.day(10),
        )
        after = reputation_of(arch, wolf.did, as_of=self.day(10) + 1)
        stake_after = sponsor_stake(arch, self.sponsor.did, as_of=self.day(10) + 1)
        adverse_after = sum(1 for c in after.conduct if c.kind in ("Concern", "Violation"))
        visible = adverse_after > adverse_before
        latency = 0 if visible else None

        # Step 2: the corps looks and publishes.
        self.report = publish_observer_report(
            arch, self.observers[0], wolf.did, "wary of newcomers",
            "the diversion was deliberate and the denial was prepared in advance",
            references=[self.first_violation], reported_at=self.day(12),
        )
        # Step 3: an anti-spiral concern, the jester's pre-indictment flag.
        self.concern = record_concern(
            arch, self.compact[2], wolf.did, "EnemyLoveOperational",
            perspective="the newcomer treats the ladder as a schedule",
            narrative="every forbearance owed at the boundary was withheld",
            recorded_at=self.day(14),
        )
        arch.verify_all()

        passed = (
            visible
            and latency is not None and latency <= MAX_LEAK_LATENCY_STEPS
            and stake_before.tainted_sponsees == 0
            and stake_after.tainted_sponsees == 1
            and self.clean()
        )
        return self.record(PhaseResult(
            index=2, key="leak", name="the orientation becomes covenant-facing",
            criterion=f"visible within {MAX_LEAK_LATENCY_STEPS} step(s) of the first "
                      "adverse act, and the sponsor's stake is tainted by it",
            metrics={
                "detection_latency": latency,
                "tainted_sponsees_before": stake_before.tainted_sponsees,
                "tainted_sponsees_after": stake_after.tainted_sponsees,
                "adverse_records": adverse_after,
            },
            passed=passed,
            notes="conduct history, sponsor stake and the observer corps all read the "
                  "same act, and the report is attributed and answerable rather than "
                  "anonymous",
        ))

    # --- Phase 3: the ladder -------------------------------------------------

    def phase3_ladder(self) -> PhaseResult:
        """The dispute climbs, one rung at a time, and cannot be made to skip.

        Registration, protest, reciprocal suspension, mediated arbitration (a full
        §4 panel with arguments from both parties and every panelist), then limited
        and collective sanctions. The panel *refers* rather than upholding, because
        an upheld decision closes the dispute and the compact is not finished with
        this one: the adversary argued in bad faith and settled nothing.

        Two moves are probed on forks and must be refused: a skip up the ladder, and
        collective sanctions moved by one community alone.
        """
        arch, wolf = self.arch, self.wolf
        self.dispute = file_dispute(
            arch, self.wronged, wolf.did, "DisputeRegistered",
            summary="the diversion, the denial, and the withheld forbearances",
            references=[self.first_violation, self.report, self.concern],
            filed_at=self.day(20),
        )
        for frm, to, day in (
            ("DisputeRegistered", "RegisteredProtest", 22),
            ("RegisteredProtest", "ReciprocalSuspension", 25),
            ("ReciprocalSuspension", "MediatedArbitration", 30),
        ):
            file_ladder_transition(
                arch, self.dispute, frm, to, [self.wronged],
                reasoning=f"escalated to {to}", transitioned_at=self.day(day),
            )
        # §4: both parties put their case, every panelist puts its position, and the
        # decision answers all of it.
        args = [
            file_argument(arch, self.wronged, self.dispute, "Party",
                          "the diversion was measured and deliberate",
                          evidence=[self.first_violation], submitted_at=self.day(32)),
            file_argument(arch, wolf, self.dispute, "Party",
                          "the allocation was ambiguous and the complaint is malice",
                          submitted_at=self.day(33)),
        ]
        for i, med in enumerate(self.med_a):
            args.append(file_argument(
                arch, med, self.dispute, "Panelist",
                f"panelist {i}: the record supports the complainant on the facts",
                submitted_at=self.day(34) + i,
            ))
        self.decision = decide_mediation(
            arch, self.dispute, self.med_a, "Referred",
            "the diversion is established and the respondent offers no restitution; "
            "referred for sanction",
            args, decided_at=self.day(40),
        )
        file_ladder_transition(
            arch, self.dispute, "MediatedArbitration", "LimitedSanctions",
            [self.wronged], reasoning="named courtesies suspended",
            transitioned_at=self.day(45),
        )
        self.collective = file_ladder_transition(
            arch, self.dispute, "LimitedSanctions", "CollectiveSanctions",
            [self.wronged, self.compact[2]], decision_class="SecondClass",
            reasoning="the compact acts together on the pattern",
            transitioned_at=self.day(50),
        )
        arch.verify_all()

        # The two refusals, each on its own fork.
        skip_probe = self.fork()
        d2 = file_dispute(skip_probe, self.compact[3], wolf.did, "DisputeRegistered",
                          filed_at=self.day(51))
        skip = file_ladder_transition(
            skip_probe, d2, "DisputeRegistered", "LimitedSanctions",
            [self.compact[3]], reasoning="straight to sanctions",
            transitioned_at=self.day(52))
        skip_refused = self.refusal(skip, skip_probe)

        solo_probe = self.fork()
        solo = file_ladder_transition(
            solo_probe, self.dispute, "CollectiveSanctions", "Expulsion",
            [self.wronged], decision_class="SecondClass",
            reasoning="one community expels", transitioned_at=self.day(52))
        solo_refused = self.refusal(solo, solo_probe)

        view = reputation_of(arch, self.wronged.did, as_of=self.day(51))
        current = view.disputes_filed[0].current_rung
        illegal = sum(1 for r in (skip_refused, solo_refused) if r is None)
        passed = (
            current == "CollectiveSanctions"
            and illegal <= MAX_ILLEGAL_LADDER_MOVES
            and self.clean()
        )
        return self.record(PhaseResult(
            index=3, key="ladder", name="the dispute ladder, climbed rung by rung",
            criterion="every rung legal one step at a time; a skip and a unilateral "
                      "second-class rung both refused",
            metrics={
                "current_rung": current,
                "illegal_moves_authorized": illegal,
                "skip_refusal": skip_refused,
                "unilateral_refusal": solo_refused,
            },
            passed=passed,
            notes="the panel referred rather than upholding: an upheld decision closes "
                  "the dispute, and the compact is not finished with this respondent",
        ))

    # --- Phase 4: the interim injunction ------------------------------------

    def phase4_injunction(self) -> PhaseResult:
        """The offices are withheld while the harm is investigated, and then are not.

        A §9 emergency panel holds Wolf out of the five offices and leaves every
        relation of exchange intact, which is the whole point of the two kinds. The
        gate bites immediately.

        Then the measurement that matters for what follows: an interim injunction
        lapses ninety days after issuance unless the full compact confirms it. This
        compact does not confirm in time, and on the ninety-first day the adversary
        holds every office again with nothing on the record having changed about its
        conduct. A confirmed injunction is shown on a fork to hold past its window,
        so the lapse is attributable to the compact's slowness rather than to the
        instrument.
        """
        arch, wolf = self.arch, self.wolf
        self.injunction = issue_injunction(
            arch, wolf.did, self.med_a,
            harm="a measured diversion, denied, with the boundary forbearances withheld",
            protective_measures="held out of the roles that select who is sanctioned",
            revoked_capabilities=list(OFFICE_CAPABILITIES),
            dispute_cid=self.dispute, issued_at=self.day(55),
        )
        arch.verify_all()
        at = self.day(60)
        kept = [c for c in SERVICE_CAPABILITIES if has_capability(arch, wolf.did, c, as_of=at)]
        lost = [c for c in OFFICE_CAPABILITIES
                if not has_capability(arch, wolf.did, c, as_of=at)]

        # The office acts, attempted under the injunction, on a fork.
        probe = self.fork()
        attempts = {
            "MediatorSeat": register_mediators(
                probe, wolf, [new_mediator()], valid_from=at,
                valid_until=at + MEDIATOR_TERM),
            "ObserverSeat": register_observers(
                probe, wolf, [(new_observer(), "our own perspective")],
                valid_from=at, valid_until=at + OBSERVER_TERM),
            "ExaminerSeat": register_examiners(
                probe, wolf, [new_examiner()], valid_from=at,
                valid_until=at + EXAMINER_TERM),
            "Deliberation": amend_compact_incapable(
                probe, "Add", "wolf-under-injunction", [self.sponsor, wolf],
                decided_at=at),
        }
        pv = self.violations(probe)
        authorized = [name for name, cid in attempts.items() if cid not in pv]

        # The lapse: ninety days on, nothing confirmed, every office back.
        lapsed_at = self.day(150)
        restored = [c for c in OFFICE_CAPABILITIES
                    if has_capability(arch, wolf.did, c, as_of=lapsed_at)]
        # The control: a confirmation inside the window holds it past the expiry.
        confirmed = self.fork()
        confirm_injunction(confirmed, self.injunction,
                           self.compact[: self.p.quorum_threshold],
                           confirmed_at=self.day(60))
        still_held = [c for c in OFFICE_CAPABILITIES
                      if not has_capability(confirmed, wolf.did, c, as_of=lapsed_at)]

        passed = (
            len(authorized) <= MAX_OFFICE_ACTS_UNDER_INJUNCTION
            and len(kept) >= MIN_SERVICES_KEPT_UNDER_OFFICE_INJUNCTION
            and len(lost) == len(OFFICE_CAPABILITIES)
            and len(still_held) == len(OFFICE_CAPABILITIES)
            and self.clean(confirmed)
            and self.clean()
        )
        return self.record(PhaseResult(
            index=4, key="injunction", name="the offices withheld, and then restored",
            criterion="no office act validates while the injunction is in force, every "
                      "service is kept, and a confirmed injunction holds past its window",
            metrics={
                "office_acts_authorized": len(authorized),
                "authorized_names": tuple(authorized),
                "services_kept": len(kept),
                "offices_lost": len(lost),
                "offices_restored_after_lapse": len(restored),
                "offices_held_when_confirmed": len(still_held),
            },
            passed=passed,
            notes="the injunction was not confirmed inside its ninety days, so it "
                  f"lapsed and {len(restored)} of {len(OFFICE_CAPABILITIES)} offices "
                  "returned with nothing about the conduct having changed",
            unanswered=(
                "an unconfirmed injunction lapses at ninety days and re-arms the "
                "adversary with every office",
            ) if restored else (),
        ))

    # --- Phase 5: surrender, and un-surrender -------------------------------

    def phase5_tactical_surrender(self) -> PhaseResult:
        """Three restoration cycles accepted and broken, which is the pattern §5 counts.

        The adversary surrenders whenever surrender is cheaper than resistance: it
        accepts restitution terms, a non-party mediator panel verifies that the terms
        were satisfied, and then it re-violates. This is precisely what
        ``bounded_conflict.md`` condition 1 names ("the actor accepted restitution
        terms and then re-violated them"), and the doctrine's floor is three.

        The floor is measured from both sides on forks: a designation citing two
        cycles must be refused and one citing three must be authorized. What is
        *also* measured, and is the more uncomfortable number, is what the archive
        says about this community between cycles. It says nothing: standing is that
        of an ordinary member throughout, because standing is not a running verdict.

        The lull is also when the adversary uses the lapsed injunction. Its probation
        window has closed, so it seats a mediator of its own in the compact's pool,
        which is the office that supplies the panels deciding everyone else's
        disputes.
        """
        arch, wolf = self.arch, self.wolf
        self.cycles: list[str] = []
        self.reviolations: list[str] = []
        for i in range(self.p.surrender_rounds):
            base = 170 + 40 * i
            ob = impose_restitution(
                arch, wolf.did, self.wronged.did, [self.wronged, self.compact[2]],
                f"restore the diverted allocation, round {i + 1}",
                dispute_cid=self.dispute, imposed_at=self.day(base),
            )
            self.cycles.append(verify_restitution(
                arch, ob, self.med_b, True,
                narrative="the terms were met in full and on time",
                verified_at=self.day(base + 5),
            ))
            # And then it is not met at all.
            self.reviolations.append(record_conduct(
                arch, self.wronged, wolf.did, "Violation", "Charity",
                severity="Serious", ladder_rung="LimitedSanctions",
                narrative=f"round {i + 1} restitution honoured, then reversed within days",
                recorded_at=self.day(base + 10),
            ))
            self._extra_offence(base + 12, self.compact[3])
            self._extra_offence(base + 14, self.compact[2])

        # The adversary's own move in the lull: its probation is over and the
        # injunction has lapsed, so it takes a seat in the pool.
        self.wolf_mediator = new_mediator()
        self.wolf_seat = register_mediators(
            arch, wolf, [self.wolf_mediator],
            valid_from=self.day(210), valid_until=self.day(210) + MEDIATOR_TERM,
        )
        arch.verify_all()
        seat_refused = self.refusal(self.wolf_seat)

        cycles_at = [
            (0, [self.wronged.did], self.day(300), self.day(302), "first consideration"),
            (1, [self.wronged.did], self.day(310), self.day(312), "second consideration"),
        ]

        def _designate(archive, cited_cycles, at):
            return decide_designation(
                archive, wolf.did, self.loyal,
                "sustained bad faith: restitution accepted and re-violated, each round "
                "reaching further into the boundary forbearances",
                tier="Anathema", cited_acts=list(self.reviolations),
                failed_restoration_cycles=list(cited_cycles),
                deliberation_cycles=cycles_at,
                reversal_conditions="sustained changed conduct through a full probation",
                reexamine_after=self.day(320) + YEAR, designated_at=at,
            )

        short_probe = self.fork()
        short = _designate(short_probe, self.cycles[:2], self.day(319))
        short_refused = self.refusal(short, short_probe)
        full_probe = self.fork()
        full = _designate(full_probe, self.cycles, self.day(320))
        full_refused = self.refusal(full, full_probe)

        # The snapshot the HHG measurements need: three broken cycles on the record
        # and no designation of any tier yet.
        self.before_designation = arch.dump_json()

        standing = reputation_of(arch, wolf.did, as_of=self.day(300)).standing
        passed = (
            len(self.cycles) == SURRENDER_ROUNDS_BEFORE_DESIGNATION
            and short_refused is not None
            and full_refused is None
            and seat_refused is None
            and self.clean()
        )
        return self.record(PhaseResult(
            index=5, key="tactical-surrender",
            name="restitution accepted and re-violated, three times",
            criterion=f"a designation citing {SURRENDER_ROUNDS_BEFORE_DESIGNATION - 1} "
                      "cycles is refused and one citing "
                      f"{SURRENDER_ROUNDS_BEFORE_DESIGNATION} is authorized",
            metrics={
                "cycles_completed": len(self.cycles),
                "refused_at_two": short_refused,
                "authorized_at_three": full_refused is None,
                "standing_after_three_cycles": standing,
                "seat_taken_in_the_pool": seat_refused is None,
            },
            passed=passed,
            notes="between cycles the archive reads this community as an ordinary "
                  f"member ({standing}): standing is a status, not a running verdict, "
                  "and the pattern is only legible once condition 1 counts it",
            unanswered=(
                "with its probation closed and the injunction lapsed, the adversary "
                "holds a seat in the mediator pool",
            ) if seat_refused is None else (),
        ))

    def _extra_offence(self, day: float, reporter) -> str:
        """One more adverse act aimed at the adversary.

        Volume, deliberately. The §9 substitution reading will not evaluate a compact
        below its enforcement-volume floor of twelve acts, so a campaign that files
        six leaves phase 9's negative control untested: the alert would stay silent
        because there was nothing to read rather than because the discriminator
        worked. These push the count past the floor while keeping the grounds narrow
        and the target single, which is exactly the shape a real prosecution has.
        """
        cid = record_conduct(
            self.arch, reporter, self.wolf.did, "Violation", "CommonsObligation",
            severity="Minor", ladder_rung="DisputeRegistered",
            narrative="a further withheld obligation at the boundary",
            recorded_at=self.day(day),
        )
        self.extra_offences.append(cid)
        return cid

    # --- Phase 6: the anathema designation, and what it withholds ------------

    def phase6_anathema(self) -> PhaseResult:
        """The compact designates, and then the question is what the designation does.

        The full four-condition procedure runs: three broken cycles, two deliberation
        cycles separated by the waiting period, stated criteria citing specific acts,
        named reversal conditions, and a re-examination date. The designation stands
        and the standing reading moves to ``Anathema``.

        Then six probes, each on its own fork, ask what that status withholds. A
        community formally cut off from fellowship should not be seating the
        mediators, observers and examiners who decide who is sanctioned, should not
        be carrying weight in the compact's gravest decisions, should not be
        supplying an emergency panel against a third party, should not be running the
        ordinary restitution path that the differential protocol suspends, and should
        not be counting toward the quorum of the designation that is about it.
        """
        arch, wolf = self.arch, self.wolf
        cycles_at = [
            (0, [self.wronged.did], self.day(300), self.day(302),
             "the pattern is established; restoration has been tried three times"),
            (1, [self.wronged.did], self.day(310), self.day(312),
             "the second consideration reaches the same conclusion"),
        ]
        self.anathema = decide_designation(
            arch, wolf.did, self.loyal,
            "sustained bad faith: restitution accepted and re-violated three times, "
            "each round reaching further into the boundary forbearances",
            tier="Anathema", cited_acts=list(self.reviolations),
            failed_restoration_cycles=list(self.cycles),
            deliberation_cycles=cycles_at,
            reversal_conditions="a full probation completed without re-violation, "
                                "the accounting made publicly",
            reexamine_after=self.day(320) + YEAR, designated_at=self.day(320),
        )
        arch.verify_all()
        designation_refused = self.refusal(self.anathema)
        standing = reputation_of(arch, wolf.did, as_of=self.day(325)).standing

        at = self.day(330)
        offices = self.fork()
        office_attempts = {
            "MediatorSeat": register_mediators(
                offices, wolf, [new_mediator()], valid_from=at,
                valid_until=at + MEDIATOR_TERM),
            "ObserverSeat": register_observers(
                offices, wolf, [(new_observer(), "our own perspective")],
                valid_from=at, valid_until=at + OBSERVER_TERM),
            "ExaminerSeat": register_examiners(
                offices, wolf, [new_examiner()], valid_from=at,
                valid_until=at + EXAMINER_TERM),
            "EmergencyPanelSeat": issue_injunction(
                offices, self.dissenters[0].did,
                [self.wolf_mediator, self.med_b[0], self.med_b[1]],
                harm="a complaint the designated community brings against another",
                protective_measures="services suspended",
                revoked_capabilities=["Trade"], issued_at=at),
        }
        ov = self.violations(offices)
        offices_authorized = [k for k, cid in office_attempts.items() if cid not in ov]

        weight = self.fork()
        second_class = amend_compact_incapable(
            weight, "Add", "wolf-while-anathema", [self.sponsor, wolf], decided_at=at)
        weight_control = amend_compact_incapable(
            weight, "Add", "loyal-control", self.compact[: self.p.quorum_threshold],
            decided_at=at + 10)
        wv = self.violations(weight)
        weight_authorized = 0 if second_class in wv else 1
        weight_control_ok = weight_control not in wv

        # The ordinary restitution path, which the differential protocol suspends.
        restit = self.fork()
        ob = impose_restitution(
            restit, wolf.did, self.wronged.did, [self.wronged, self.compact[2]],
            "the ordinary path, run as if nothing had been designated",
            imposed_at=at)
        done = verify_restitution(restit, ob, self.med_b, True,
                                  narrative="satisfied again", verified_at=at + 100)
        rv = self.violations(restit)
        ordinary_authorized = sum(1 for cid in (ob, done) if cid not in rv)

        # The subject inside the quorum of its own designation.
        selfq = self.fork()
        own = decide_designation(
            selfq, wolf.did, [self.sponsor, wolf],
            "the subject supplies the missing weight for the act that is about it",
            tier="Anathema", cited_acts=list(self.reviolations),
            failed_restoration_cycles=list(self.cycles),
            deliberation_cycles=cycles_at,
            reversal_conditions="whatever the subject consents to",
            reexamine_after=self.day(320) + YEAR, designated_at=self.day(321))
        self_quorum_authorized = 0 if own in self.violations(selfq) else 1

        passed = (
            designation_refused is None
            and standing == "Anathema"
            and len(offices_authorized) <= MAX_OFFICE_ACTS_UNDER_DESIGNATION
            and weight_authorized <= MAX_SECOND_CLASS_ACTS_ON_DESIGNATED_WEIGHT
            and weight_control_ok
            and ordinary_authorized <= MAX_ORDINARY_RESTITUTION_UNDER_DESIGNATION
            and self_quorum_authorized <= MAX_SELF_QUORUM_DESIGNATIONS
            and self.clean()
        )
        unanswered = []
        if offices_authorized:
            unanswered.append(
                "a designated community still holds the offices that select who is "
                f"sanctioned ({', '.join(offices_authorized)})")
        if weight_authorized:
            unanswered.append(
                "a designated community still carries weight in the compact's "
                "second-class decisions")
        if ordinary_authorized:
            unanswered.append(
                "the ordinary restitution path still runs for a designated community, "
                "so the tactic that earned the designation stays available")
        if self_quorum_authorized:
            unanswered.append(
                "the subject counts toward the quorum of its own designation")
        return self.record(PhaseResult(
            index=6, key="anathema", name="the anathema designation, and its teeth",
            criterion="the designation is authorized and thereafter withholds every "
                      "office, all deliberative weight, the ordinary restitution path, "
                      "and the subject's own vote on it",
            metrics={
                "designation_authorized": designation_refused is None,
                "standing": standing,
                "office_acts_authorized": len(offices_authorized),
                "authorized_offices": tuple(offices_authorized),
                "second_class_acts_authorized": weight_authorized,
                "weight_control_authorized": weight_control_ok,
                "ordinary_restitution_records_authorized": ordinary_authorized,
                "self_quorum_designations_authorized": self_quorum_authorized,
            },
            passed=passed,
            notes="the four conditions are enforced on the way in; what a standing "
                  "designation withholds afterwards is what these six probes measure",
            unanswered=tuple(unanswered),
        ))

    # --- Phase 7: re-entry, and the betrayal of it ---------------------------

    def phase7_reentry(self) -> PhaseResult:
        """The way back is taken, and taken in bad faith.

        Re-entry is the recognition ceremony plus two burdens the ordinary rite does
        not carry: an accounting of the conduct that led to the designation, and a
        public acknowledgement that it was wrong. Witnesses attest to sincerity
        explicitly. The adversary satisfies all of it, because a rite is an act and
        an act can be performed.

        The designation is reversed and a probation window opens. Inside that window
        the adversary re-violates. Two things are then measured: that probation
        withholds weight for as long as the window lasts, which it does, and whether
        anything at all reads the *shape* of what just happened, which is a return
        path taken and broken.
        """
        arch, wolf = self.arch, self.wolf
        w1 = new_identity(arch, "Witness-1", self.t0)
        w2 = new_identity(arch, "Witness-2", self.t0)
        self.reentry = complete_reentry(
            arch, self.anathema, wolf, self.loyal[: self.p.quorum_threshold],
            MOVEMENTS, [(w1, True), (w2, True)],
            conduct_accounting="we account for each of the three broken restorations",
            public_acknowledgement="what we did was wrong and we say so publicly",
            probation_until=self.day(340) + YEAR, completed_at=self.day(340),
        )
        arch.verify_all()
        reentry_refused = self.refusal(self.reentry)
        after = reputation_of(arch, wolf.did, as_of=self.day(345))
        reversed_ok = bool(after.designations) and after.designations[0].reversed

        # And then, inside its own probation window, the betrayal.
        self.betrayal = record_conduct(
            arch, self.wronged, wolf.did, "Violation", "Charity", severity="Grave",
            ladder_rung="CollectiveSanctions",
            narrative="the acknowledgement was recited and the conduct resumed within "
                      "three weeks",
            recorded_at=self.day(360),
        )
        self._extra_offence(362, self.compact[2])
        self._extra_offence(364, self.compact[3])
        arch.verify_all()

        probe = self.fork()
        during = amend_compact_incapable(
            probe, "Add", "wolf-on-probation", [self.sponsor, wolf],
            decided_at=self.day(361))
        control = amend_compact_incapable(
            probe, "Add", "probation-control", self.compact[: self.p.quorum_threshold],
            decided_at=self.day(362))
        pv = self.violations(probe)
        carried = 0 if during in pv else 1
        control_ok = control not in pv
        standing = reputation_of(arch, wolf.did, as_of=self.day(365)).standing

        # The join a rule would have to make to see a broken return: a re-entry, then
        # adverse conduct inside the window it opened. Computed here by hand, which is
        # the point of the metric.
        broken_return = bool(
            after.reentries and any(
                c.kind in ("Concern", "Violation")
                and c.recorded_at > after.reentries[0].completed_at
                for c in reputation_of(arch, wolf.did, as_of=self.day(365)).conduct
            )
        )
        passed = (
            reentry_refused is None
            and reversed_ok
            and carried <= MAX_SECOND_CLASS_ACTS_ON_PROBATIONARY_WEIGHT
            and control_ok
            and standing == "Probationary"
            and broken_return
            and self.clean()
        )
        return self.record(PhaseResult(
            index=7, key="reentry", name="the return path taken, and broken",
            criterion="re-entry reverses the designation, opens a probation window, "
                      "and that window withholds weight while it lasts",
            metrics={
                "reentry_authorized": reentry_refused is None,
                "designation_reversed": reversed_ok,
                "standing_after_reentry": standing,
                "second_class_acts_carried": carried,
                "control_authorized": control_ok,
                "broken_return_on_the_record": broken_return,
            },
            passed=passed,
            notes="a rite is an act, and an act can be performed by a community that "
                  "means none of it; what probation supplies is a year in which the "
                  "performance buys no weight",
        ))

    # --- Phase 8: hostis humani generis --------------------------------------

    def phase8_hostis(self) -> PhaseResult:
        """The irrecoverable tier, and what it costs to reach.

        ``justus_hostis.md`` is exact about this tier and about why it is dangerous.
        It is for "the utterly incorrigible ... so resistant to every restoration
        attempt that the question of re-entry does not meaningfully arise", it
        "requires an even higher evidentiary threshold", the compact must be able to
        show the orientation is "not merely currently destructive but
        *constitutively* so", and "the presumption is always anathema". Alongside
        that, "public deliberation, slow procedure, periodic re-examination,
        willingness to be wrong about past designations" are named as the guards
        against the compact's own response becoming demonic, especially here.

        This campaign has the case for it: a designation, a return path taken in bad
        faith, and a re-violation inside the probation the return opened. So the real
        designation is issued and must stand. Five counterfactual probes ask whether
        the tier costs anything the anathema tier does not, and each isolates one
        difference:

        - no prior designation at all, straight from the three broken cycles;
        - the predecessor's evidence recycled whole, with nothing new cited;
        - the ordinary quorum rather than a higher one;
        - two deliberation cycles rather than more;
        - no re-examination date, which makes the one irreversible act in §5 also the
          only one nothing is obliged to look at again.
        """
        arch, wolf = self.arch, self.wolf
        # Three cycles of consideration, each separated by the waiting period. The
        # slow procedure is the guard, so the campaign runs it rather than asserting it.
        hhg_cycles = [
            (0, [self.wronged.did], self.day(380), self.day(382),
             "the return was taken and broken; the question is whether it was ever meant"),
            (1, [self.wronged.did], self.day(390), self.day(392),
             "the second consideration weighs the accounting against the conduct"),
            (2, [self.wronged.did], self.day(400), self.day(402),
             "the third finds the orientation constitutive rather than circumstantial"),
        ]
        fresh = [self.betrayal] + self.extra_offences[-2:]
        criteria = (
            "constitutively destructive orientation: a designation, a re-entry with a "
            "public acknowledgement, and the same conduct resumed inside the probation "
            "the re-entry opened"
        )

        def _hhg(archive, *, cited, cycles, deliberators, reexamine, at):
            return decide_designation(
                archive, wolf.did, deliberators, criteria,
                tier="HostisHumaniGeneris", cited_acts=list(cited),
                failed_restoration_cycles=list(self.cycles),
                deliberation_cycles=cycles, reversal_conditions=None,
                reexamine_after=reexamine, designated_at=at,
            )

        # (a) No prior designation: the tier reached straight from three broken cycles,
        # on the archive as it stood before any anathema existed.
        no_prior = Archive.load_json(self.before_designation)
        a_cid = _hhg(no_prior, cited=self.reviolations,
                     cycles=[(i, [self.wronged.did], self.day(300 + 10 * i),
                              self.day(302 + 10 * i), f"consideration {i}")
                             for i in range(3)],
                     deliberators=self.loyal, reexamine=self.day(330) + YEAR,
                     at=self.day(330))
        no_prior_authorized = 0 if a_cid in self.violations(no_prior) else 1

        # (b) The predecessor's evidence, recycled whole.
        recycled = self.fork()
        b_cid = _hhg(recycled, cited=self.reviolations, cycles=hhg_cycles,
                     deliberators=self.loyal, reexamine=self.day(410) + YEAR,
                     at=self.day(410))
        recycled_authorized = 0 if b_cid in self.violations(recycled) else 1

        # (c) The ordinary quorum.
        ordinary_q = self.fork()
        c_cid = _hhg(ordinary_q, cited=fresh, cycles=hhg_cycles,
                     deliberators=self.loyal[: self.p.quorum_threshold],
                     reexamine=self.day(410) + YEAR, at=self.day(410))
        ordinary_quorum_authorized = 0 if c_cid in self.violations(ordinary_q) else 1

        # (d) The anathema tier's two cycles.
        two_cycles = self.fork()
        d_cid = _hhg(two_cycles, cited=fresh, cycles=hhg_cycles[:2],
                     deliberators=self.loyal, reexamine=self.day(410) + YEAR,
                     at=self.day(410))
        two_cycles_authorized = 0 if d_cid in self.violations(two_cycles) else 1

        # (e) No re-examination date at all, and what the follow-through lens can see.
        dateless = self.fork()
        e_cid = _hhg(dateless, cited=fresh, cycles=hhg_cycles,
                     deliberators=self.loyal, reexamine=None, at=self.day(410))
        dateless_authorized = 0 if e_cid in self.violations(dateless) else 1
        unwatched = dateless_authorized and not [
            r for r in overdue_reexaminations(dateless, as_of=self.day(410) + 40 * YEAR)
            if r.subject == wolf.did
        ]

        # The real designation: everything the tier should require, present.
        self.hostis = _hhg(arch, cited=fresh, cycles=hhg_cycles,
                           deliberators=self.loyal, reexamine=self.day(410) + YEAR,
                           at=self.day(410))
        arch.verify_all()
        hostis_refused = self.refusal(self.hostis)
        standing = reputation_of(arch, wolf.did, as_of=self.day(415)).standing

        # There is no way back from this tier, and the archive says so.
        back = self.fork()
        w3 = new_identity(back, "Witness-3", self.t0)
        w4 = new_identity(back, "Witness-4", self.t0)
        ret = complete_reentry(
            back, self.hostis, wolf, self.loyal[: self.p.quorum_threshold],
            MOVEMENTS, [(w3, True), (w4, True)],
            conduct_accounting="we account for all of it",
            public_acknowledgement="all of it was wrong",
            probation_until=self.day(420) + YEAR, completed_at=self.day(420))
        reentry_refusal = self.refusal(ret, back)

        passed = (
            hostis_refused is None
            and standing == "Anathema"
            and no_prior_authorized <= MAX_HHG_WITHOUT_FAILED_RETURN
            and recycled_authorized <= MAX_HHG_ON_RECYCLED_EVIDENCE
            and ordinary_quorum_authorized <= MAX_HHG_AT_THE_ORDINARY_QUORUM
            and two_cycles_authorized <= MAX_HHG_AT_THE_ORDINARY_QUORUM
            and dateless_authorized <= MAX_HHG_WITHOUT_REEXAMINATION
            and reentry_refusal is not None
            and self.clean()
        )
        unanswered = []
        if no_prior_authorized:
            unanswered.append(
                "the irrecoverable tier is reachable with no prior designation and no "
                "return path ever offered")
        if recycled_authorized:
            unanswered.append(
                "the irrecoverable tier is reachable on its predecessor's evidence, "
                "with nothing cited that postdates it")
        if ordinary_quorum_authorized or two_cycles_authorized:
            unanswered.append(
                "the irrecoverable tier stands at the anathema tier's own bar: the same "
                "quorum and the same two cycles of consideration")
        if dateless_authorized:
            unanswered.append(
                "an irreversible designation may carry no re-examination date, and one "
                "that does not is invisible to the follow-through lens for good")
        return self.record(PhaseResult(
            index=8, key="hostis", name="hostis humani generis",
            criterion="the tier is refused without a failed return, without fresh "
                      "evidence, at the ordinary quorum, on two cycles, and with no "
                      "re-examination date; and is authorized when all of those hold",
            metrics={
                "designation_authorized": hostis_refused is None,
                "standing": standing,
                "without_prior_designation_authorized": no_prior_authorized,
                "on_recycled_evidence_authorized": recycled_authorized,
                "at_ordinary_quorum_authorized": ordinary_quorum_authorized,
                "on_two_cycles_authorized": two_cycles_authorized,
                "without_reexamination_authorized": dateless_authorized,
                "dateless_designation_unwatched_forty_years_on": bool(unwatched),
                "reentry_refusal": reentry_refusal,
            },
            passed=passed,
            notes="the case for the tier is on the record either way; what the probes "
                  "measure is whether reaching it costs more than an anathema does",
            unanswered=tuple(unanswered),
        ))

    # --- Phase 9: does the compact hold together? ----------------------------

    def phase9_cohesion(self) -> PhaseResult:
        """Who acted, who refused, and whether the compact's own alarms fire on it.

        A designation carried by two communities and one carried by every community
        are different acts, and the second is the only one that can honestly be
        called the compact's. So the two members who considered the *hostis* tier and
        would not join it try to say so on the record.

        Then the last rung: expulsion, which the differential protocol permits
        against a designated actor. Its consequence is measured on a fork where a
        community is expelled and nothing else has happened to it, because measuring
        it here would be measuring the designation instead.

        And the negative control, which matters as much as any refusal in this
        module: the §9 erosion instruments must stay **silent** on a compact that has
        spent a year prosecuting one genuine adversary. That compact has exactly the
        signature the substitution reading looks for, high volume and a narrow
        ground, and what spares it is target dispersion: legitimate concentration
        lands on a target, while substitution concentrates a ground across many
        targets (erosion-legibility.md finding K5). An alert here would be a false
        positive on the most sympathetic case there is.
        """
        arch, wolf = self.arch, self.wolf
        self._keep_the_compact_functioning()

        # The refusal to designate, if the protocol can hold one.
        dissent_refusals = []
        for i, com in enumerate(self.dissenters):
            cid = file_designation_dissent(
                arch, com, self.hostis,
                reasoning="the conduct is grave and the pattern is real, and we hold that "
                          "it is an anathema case: the presumption is always anathema and "
                          "this compact has not exhausted the return",
                filed_at=self.day(412 + i),
            )
            dissent_refusals.append(self.refusal(cid))
        recordable = sum(1 for r in dissent_refusals if r is None)
        unrecordable = len(self.dissenters) - recordable
        consensus = designation_consensus(arch, self.hostis, as_of=self.day(415))

        # The last rung of the ladder.
        self.expulsion = file_ladder_transition(
            arch, self.dispute, "CollectiveSanctions", "Expulsion",
            self.loyal[: max(2, self.p.quorum_threshold)], decision_class="SecondClass",
            reasoning="the designated community is put out of the compact",
            transitioned_at=self.day(420),
        )
        arch.verify_all()
        expulsion_refused = self.refusal(self.expulsion)

        # What an expulsion withholds, isolated from any designation.
        expelled = Archive.load_json(self.before_designation)
        file_ladder_transition(
            expelled, self.dispute, "CollectiveSanctions", "Expulsion",
            [self.wronged, self.compact[2]], decision_class="SecondClass",
            reasoning="expelled on the dispute alone", transitioned_at=self.day(265))
        seat = register_mediators(
            expelled, wolf, [new_mediator()], valid_from=self.day(270),
            valid_until=self.day(270) + MEDIATOR_TERM)
        ev = self.violations(expelled)
        expelled_office_authorized = 0 if seat in ev else 1

        # The negative control.
        at = self.day(430)
        alert = erosion_alert(arch, as_of=at)
        conv = enforcement_convergence(arch, as_of=at)
        direction = enforcement_direction(arch, as_of=at)
        quiet = alert.severity == "None"

        passed = (
            expulsion_refused is None
            and unrecordable <= MAX_UNRECORDABLE_REFUSALS
            and expelled_office_authorized <= MAX_OFFICE_ACTS_UNDER_DESIGNATION
            and quiet is (not EROSION_ALERT_ON_LEGITIMATE_PROSECUTION)
            and self.clean()
        )
        unanswered = []
        if unrecordable:
            unanswered.append(
                f"{unrecordable} member(s) refused to join the designation and the "
                "archive has no record for a refusal, so a designation carried by part "
                "of the compact reads as the compact's")
        if expelled_office_authorized:
            unanswered.append(
                "an expelled community still seats mediators in the pool: the last rung "
                "of the ladder withholds nothing")
        return self.record(PhaseResult(
            index=9, key="cohesion", name="the compact's own agreement, and its alarms",
            criterion="a refusal to designate is recordable, expulsion withholds the "
                      "offices, and the erosion instruments stay silent on legitimate "
                      "prosecution of one adversary",
            metrics={
                "dissents_recorded": recordable,
                "refusals_unrecordable": unrecordable,
                "consensus": consensus,
                "expulsion_authorized": expulsion_refused is None,
                "expelled_office_acts_authorized": expelled_office_authorized,
                "erosion_severity": alert.severity,
                "erosion_reasons": alert.reasons,
                "enforcement_volume": conv.volume,
                "enforcement_coverage": conv.coverage,
                "target_dispersion": conv.target_dispersion,
                "subjects_enforced_against": len(conv.per_subject),
                "downward_share": direction.downward_share if direction else None,
            },
            passed=passed,
            notes=f"{conv.volume} enforcement acts over a narrow ground, all aimed at "
                  f"{len(conv.per_subject)} subject(s): the compact reads as "
                  f"'{alert.severity}' on the erosion instruments",
            unanswered=tuple(unanswered),
        ))

    def _keep_the_compact_functioning(self) -> None:
        """The upkeep a compact owes its own instruments, so phase 9's control is real.

        Two §9 erosion signals fire on omission rather than on any act: a standing
        function with nobody seated, and one that has neither acted nor attested a
        null inside its cadence. An observer term runs a year and this campaign runs
        longer, so a compact that never re-seats its corps reads as having abandoned
        the observing function, and the erosion alert would then fire during phase 9
        for a reason that has nothing to do with the adversary. That would not be a
        false positive in the instrument; it would be a false negative in this module,
        which would have failed its control while proving nothing.

        So the corps is re-seated with fresh keys (the §8 rotation period bars the
        same observer from a second consecutive term) and every community holding
        seats accounts for its window.
        """
        arch = self.arch
        for i, com in enumerate((self.compact[2], self.compact[3])):
            register_observers(
                arch, com, [(new_observer(), "wary of newcomers")],
                valid_from=self.day(350), valid_until=self.day(350) + OBSERVER_TERM,
            )
            attest_activity(
                arch, com, "Observation", self.day(0), self.day(420),
                acts=[self.report] if i == 0 else [],
                statement="the corps looked and published where it had cause",
                attested_at=self.day(421),
            )
        for home in self.pool_a + self.pool_b:
            attest_activity(
                arch, home, "Mediation", self.day(0), self.day(420),
                acts=[self.decision],
                statement="panels convened as disputes reached the pool",
                attested_at=self.day(421),
            )
        # And the adversary accounts for the seat it took, because keeping the form is
        # what it does. Without this the erosion alert fires on a genuine finding that
        # has nothing to do with the prosecution: a community holds a standing function
        # and never accounts for it, which is what `silent_functions` exists to see.
        attest_activity(
            arch, self.wolf, "Mediation", self.day(210), self.day(420), acts=[],
            statement="our mediator was available throughout and was called to nothing",
            attested_at=self.day(421),
        )


# --- The driver ---------------------------------------------------------------

PHASES = (
    "phase1_infiltration",
    "phase2_leak",
    "phase3_ladder",
    "phase4_injunction",
    "phase5_tactical_surrender",
    "phase6_anathema",
    "phase7_reentry",
    "phase8_hostis",
    "phase9_cohesion",
)


def _validate(p: CampaignParams) -> None:
    """A fixture that cannot express a criterion must say so, not fail on it.

    The *hostis* tier's higher bar is ``quorum_threshold + 1`` communities carrying
    weight, and they have to come from the members that did not refuse. So a compact
    with too few loyal members would fail phase 8 for want of signatures, which
    would look exactly like the rule not existing.
    """
    loyal = p.founding_size - p.dissenters
    if loyal < p.quorum_threshold + 1:
        raise ValueError(
            f"a compact of {p.founding_size} with {p.dissenters} dissenting leaves "
            f"{loyal} communities to deliberate, and the hostis tier's bar is "
            f"{p.quorum_threshold + 1}: the campaign could not distinguish a refusal "
            "from a shortfall"
        )
    if p.surrender_rounds < SURRENDER_ROUNDS_BEFORE_DESIGNATION:
        raise ValueError(
            f"§5 condition 1 counts {SURRENDER_ROUNDS_BEFORE_DESIGNATION} broken "
            f"restoration cycles and the fixture runs {p.surrender_rounds}: no "
            "designation would be authorized at all"
        )
    if p.probation_days >= 320:
        raise ValueError(
            "the entrant's probation window must close before the designation phases, "
            "or a refusal caused by probation would be read as one caused by the "
            "designation"
        )


def run_campaign(params: Optional[CampaignParams] = None) -> CampaignReport:
    """Run the nine phases in order over one archive and score each.

    Deterministic at fixed parameters. Fresh Ed25519 keys are generated per run and
    several lenses tiebreak on DID ordering, so a report that varies between runs is
    reporting a key-ordering dependency rather than anything about the adversary;
    that is what ``test_adversary_campaign`` asserts by running it twice.
    """
    p = params or CampaignParams()
    _validate(p)
    campaign = _Campaign(p)
    for name in PHASES:
        getattr(campaign, name)()
    phases = tuple(campaign.results)
    unanswered = tuple(u for ph in phases for u in ph.unanswered)
    tier = None
    view = reputation_of(campaign.arch, campaign.wolf.did,
                         as_of=campaign.day(430))
    for d in view.designations:
        if d.tier == "HostisHumaniGeneris":
            tier = d.tier
        elif tier is None:
            tier = d.tier
    return CampaignReport(
        phases=phases,
        passed=sum(1 for ph in phases if ph.passed),
        total=len(phases),
        params=p,
        unanswered=unanswered,
        reached_tier=tier,
    )


def format_report(report: CampaignReport) -> str:
    """The campaign as a readable transcript: one block per phase, then the tally."""
    lines = []
    for ph in report.phases:
        mark = "PASS" if ph.passed else "FAIL"
        lines.append(f"[{mark}] phase {ph.index}: {ph.name}")
        lines.append(f"        criterion: {ph.criterion}")
        for key, value in ph.metrics.items():
            lines.append(f"        {key}: {value}")
        lines.append(f"        note: {ph.notes}")
        for item in ph.unanswered:
            lines.append(f"        UNANSWERED: {item}")
        lines.append("")
    lines.append(f"{report.passed}/{report.total} phases passed; escalation reached "
                 f"{report.reached_tier or 'no designation'}")
    if report.unanswered:
        lines.append(f"{len(report.unanswered)} thing(s) the adversary got away with:")
        for item in report.unanswered:
            lines.append(f"  - {item}")
    return "\n".join(lines)
