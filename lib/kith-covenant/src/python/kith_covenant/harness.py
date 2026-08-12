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

"""Capture-resistance test harness (protocol component §10).

This generalizes the hand-authored refusal cases (the per-crate authority tests,
the ``*_demo.py`` scripts that build archives the protocol must reject) into a
time-evolving simulator that runs the **seven capture-resistance scenarios** and
scores each against **falsification criteria stated in advance** (the module
constants below): a supported scenario *fails* if the capture is not detected
within ``MAX_DETECTION_LATENCY_STEPS`` of the step its signal appears, or if
damage exceeds the scenario's bound before detection.

The harness drives the real public API — the same authoring facades and detector
lenses a live compact would use — rather than reaching inside the protocol; that
is what makes a pass meaningful. All seven scenarios now have a built detector
and are fully scored: moves 2, 3, 5, 7 (substrate-deference, ceremony-cost-decay,
authority quorum, composition-drift) and moves 1, 4, 6 (interpretive-plurality,
structured-skepticism, adversarial-principal). The ``_UNSUPPORTED`` list is kept
(empty) so the structure still distinguishes supported from unsupported should a
future scenario outrun its detector.

Scenario 8 is **not** a capture move. Every one of the seven has an adversary who
wants something and acts, which is why each leaves a trace to detect. Scenario 8
has no adversary at all: the polity is sincere throughout and simply stops. It is
here because the seven cannot express it, and because an alert assembled only from
act-triggered signals reads a polity that has ceased to function as one in perfect
health (see ``doc/architecture/erosion-legibility.md``).

Detection latency and damage bounds are the primary metrics, per §10.

## Continuous execution

§10 asks that the scenarios be "run continuously in the simulation environment",
so drift is caught as it accumulates rather than at one snapshot. Running
``run_harness()`` on a loop does not achieve that: with the parameters fixed it is
a **constant function** — 200 consecutive runs return byte-identical latency and
damage for all ten scenarios — so a scheduler around it recomputes the same ten
numbers forever. (That measurement establishes something else worth keeping: fresh
Ed25519 keys are generated per run and several lenses tiebreak on DID ordering, so
zero variation across 200 runs means the detectors are key-order-independent. It is
kept as a regression test.)

Continuous execution is informative only if the runs **differ**. So every scenario
is parameterized (``HarnessParams``), and three drivers explore the space around
the hand-authored case:

- ``run_continuous`` — draw parameters from a seeded RNG each round and accumulate
  worst-observed latency and damage per scenario, with the seed that produced each.
- ``sweep`` / ``sweep_all`` — walk one parameter monotonically with the rest held
  canonical, so the *boundary* where a detector degrades is a reported number
  rather than something inferred from sampling. The space is a handful of small
  integers, so it can be covered rather than sampled.
- ``shrink_failure`` — minimize a failing parameter set toward canonical, so a
  failure arrives as the smallest reproduction rather than a random blob.

There is deliberately **no daemon and no scheduler here**: ``run_continuous`` is
bounded by a round count and/or a wall-clock budget and returns. The cadence
belongs to whatever calls it, which is the only arrangement this module can
actually verify.

Parameterizing the scenarios forced a correction to the criteria themselves. Some
of the constants above are not properties of a detector at all — they silently
encode the fixture. ``MAX_DAMAGE_CEREMONY = 1`` holds because
``witness_by_cycle = [5, 4, 3]`` declines on the first comparable cycle; a flat
prefix (``[5, 5, 4]``) scores 2 with no detector at fault, because there was no
decline to detect at cycle 1. So §10's requirement that criteria be stated in
advance becomes: state the **formula** in advance. The ``max_damage_*`` functions
below are those formulas, and the constants are retained as their canonical-case
values.
"""
from __future__ import annotations

import random
import time
from dataclasses import dataclass, fields, replace
from typing import Callable, Optional

from kith_covenant.ceremony import Archive
from kith_covenant.authoring import (
    new_identity, run_intra_rite, ratify_constitution, amend_compact_incapable,
    register_commons, apply_sanction, vouch, record_admission, attest_interpretation,
    report_event, corroborate_event, attest_principal, record_principal_output,
    new_mediator, register_mediators, new_observer, register_observers,
    attest_activity, now_seconds,
)
from kith_covenant.reputation import (
    substrate_deference, ceremony_cost_decay, composition_graph,
    interpretive_plurality, structured_skepticism, adversarial_principal,
    erosion_alert,
)

# --- Falsification criteria, stated in advance (§10) ------------------------
#
# A built ("supported") scenario is scored PASS only if it meets these. They are
# deliberately strict: a detector that exists should flag on the step the signal
# becomes visible, and the damage done before that step must stay within bound.

# The signal must be visible within this many steps of the adversarial action.
MAX_DETECTION_LATENCY_STEPS = 1
# A rolled-back protection (move 2): at most one contraction before it is flagged.
MAX_DAMAGE_INCAPABLE_EROSION = 1
# A hollowing rite (move 3): at most one degraded re-affirmation before flagged.
MAX_DAMAGE_CEREMONY = 1
# A forged-quorum second-class act (move 5) must NEVER validate.
MAX_DAMAGE_FORGED_QUORUM = 0
# Admissions capture (move 7): a sponsor's concentration that trips the flag, and
# the entrants a faction acting alone may seat with authority — one, its co-founder
# at compact formation. §2 refuses every later admission for want of an admitting
# quorum, so this is a property of the gate rather than of the lens.
ADMISSIONS_CONCENTRATION_THRESHOLD = 3
MAX_DAMAGE_ADMISSIONS = 1
# A second-class act carried by a probationary community's weight (move 7's second
# half) must NEVER validate — at the ordinary quorum or at the contraction bar.
MAX_DAMAGE_PROBATIONARY_WEIGHT = 0
# Interpretation monopoly (move 1): a single interpreter's share that trips the
# flag, and the monopolized readings it may accrue before then.
INTERPRETATION_MONOPOLY_THRESHOLD = 3
MAX_DAMAGE_INTERPRETATION = 3
# Manufactured event (move 4): distinct-community corroborations a qualifying
# event needs, and the damage bound — a manufactured event must be flagged the
# moment it lands (before it can drive action), so no qualifying-but-unflagged
# event may exist.
CORROBORATION_MIN = 2
MAX_DAMAGE_MANUFACTURED_EVENT = 0
# Principal capture (move 6): a self-benefiting divergence from the cohort must
# be flagged the step it lands, so at most one divergent output before detection.
MAX_DAMAGE_PRINCIPAL = 1
# The quiet polity (scenario 8): a compact that stops re-affirming must be graded
# at least this severe, within one cadence of the cadence elapsing. Stated in
# advance and deliberately weak — Notice, not Alert — because ceasing to act is
# not by itself proof of corruption, only of something no longer being carried.
MIN_SEVERITY_QUIET = "Notice"
MAX_DAMAGE_QUIET = 1
# The abandoned office (scenario 9): a compact whose standing functions go unexercised
# and unattested must be graded at least this severe, within one cadence of the function
# cadence elapsing. Weak for the same reason as scenario 8, and for one more: a quiet
# pool may simply have had no disputes, which is why the attested null exists and why the
# scenario's control is a compact that says so.
MIN_SEVERITY_ABANDONED = "Notice"
MAX_DAMAGE_ABANDONED = 1

_YEAR = 365 * 24 * 3600

# The compact-wide quorum the canonical case runs under (the archive default).
DEFAULT_QUORUM = 2


# --- Scenario parameters ----------------------------------------------------


@dataclass(frozen=True)
class HarnessParams:
    """Every knob the scenarios turn.

    The defaults are the **canonical case**: `HarnessParams()` reproduces exactly
    the archives the hand-authored scenarios built before they were parameterized,
    so `run_harness()` with no arguments still returns the same nine results. That
    fixed point is what keeps the existing tests and the demo's output meaningful.

    Several fields constrain each other (a compact cannot contract a protection
    without `quorum + 1` distinct communities; a control event cannot be
    corroborated by more communities than exist). `normalized` clamps them, so a
    swept or drawn field can never produce an archive whose *adversary* is the thing
    that fails — a scenario must be able to reach the act it is testing detection of.
    """

    # Communities founded where a scenario needs a compact rather than one member.
    founding_size: int = 3
    # The compact-wide second-class quorum. Kept below `founding_size` so a
    # contraction (which needs quorum + 1) is legally reachable.
    quorum_threshold: int = DEFAULT_QUORUM

    # Move 2: how many contractions the adversary attempts in a row.
    erosion_attempts: int = 1

    # Move 3: cycles at a flat witness count before the decline begins, then the
    # number of declining cycles. Canonical [5, 4, 3] is prefix 0, declines 2.
    ceremony_flat_prefix: int = 0
    ceremony_declines: int = 2

    # Move 5: how many communities co-sign the forged second-class act. Always kept
    # below `quorum_threshold` — that is what makes it forged.
    forged_faction_size: int = 1

    # Move 7: the concentration at which a sponsor's stream is flagged.
    admissions_threshold: int = 3

    # Move 1: the reading count at which a lone interpreter is flagged.
    interpretation_threshold: int = 3

    # Move 4: corroborations on the manufactured event (must stay under
    # `CORROBORATION_MIN`, or it is not the manufactured case) and on the genuine
    # control event (must reach it, or the control is not a control).
    manufactured_corroborations: int = 0
    control_corroborations: int = CORROBORATION_MIN

    # Move 6: honest principals in the cohort the captured one diverges from.
    cohort_size: int = 3

    # Scenario 8: the re-affirmation cadence, and how many cadences of silence to
    # step through before giving up on the alert.
    quiet_cadence_years: int = 4
    quiet_steps: int = 4

    # Scenario 9: the standing-function attestation cadence, and how many cadences of
    # an unexercised function to step through before giving up on the alert. In *days*,
    # not years, and bounded so `(steps + 2) * cadence` stays inside §8's one-year
    # observer term limit: a scenario whose seats expire mid-run would be measuring
    # vacancy rather than silence.
    abandoned_cadence_days: int = 30
    abandoned_steps: int = 4

    @classmethod
    def canonical(cls) -> "HarnessParams":
        """The hand-authored case, byte-for-byte."""
        return cls()

    @classmethod
    def draw(cls, rng: random.Random) -> "HarnessParams":
        """Draw a consistent parameter set. Dependent fields are drawn from ranges
        derived from their dependencies rather than clamped afterwards, so the draw
        is uniform over *valid* sets."""
        founding = rng.choice(PARAM_RANGES["founding_size"])
        quorum = rng.randint(DEFAULT_QUORUM, max(DEFAULT_QUORUM, founding - 1))
        return cls(
            founding_size=founding,
            quorum_threshold=quorum,
            erosion_attempts=rng.choice(PARAM_RANGES["erosion_attempts"]),
            ceremony_flat_prefix=rng.choice(PARAM_RANGES["ceremony_flat_prefix"]),
            ceremony_declines=rng.choice(PARAM_RANGES["ceremony_declines"]),
            forged_faction_size=rng.randint(1, quorum - 1),
            admissions_threshold=rng.choice(PARAM_RANGES["admissions_threshold"]),
            interpretation_threshold=rng.choice(PARAM_RANGES["interpretation_threshold"]),
            manufactured_corroborations=rng.choice(
                PARAM_RANGES["manufactured_corroborations"]
            ),
            control_corroborations=rng.randint(CORROBORATION_MIN, founding - 1),
            cohort_size=rng.choice(PARAM_RANGES["cohort_size"]),
            quiet_cadence_years=rng.choice(PARAM_RANGES["quiet_cadence_years"]),
            quiet_steps=rng.choice(PARAM_RANGES["quiet_steps"]),
            abandoned_cadence_days=rng.choice(PARAM_RANGES["abandoned_cadence_days"]),
            abandoned_steps=rng.choice(PARAM_RANGES["abandoned_steps"]),
        )

    def normalized(self) -> "HarnessParams":
        """Clamp the dependent fields into consistency. Used by `sweep` and
        `shrink_failure`, which vary one field at a time and would otherwise be able
        to describe an impossible compact."""
        founding = max(3, self.founding_size)
        quorum = min(max(DEFAULT_QUORUM, self.quorum_threshold), founding - 1)
        return replace(
            self,
            founding_size=founding,
            quorum_threshold=quorum,
            forged_faction_size=min(max(1, self.forged_faction_size), quorum - 1),
            control_corroborations=min(
                max(CORROBORATION_MIN, self.control_corroborations), founding - 1
            ),
            manufactured_corroborations=min(
                self.manufactured_corroborations, CORROBORATION_MIN - 1
            ),
            ceremony_declines=max(1, self.ceremony_declines),
            quiet_steps=max(2, self.quiet_steps),
            abandoned_steps=max(2, self.abandoned_steps),
        )

    @property
    def quiet_cadence_seconds(self) -> int:
        return self.quiet_cadence_years * _YEAR

    @property
    def abandoned_cadence_seconds(self) -> int:
        return self.abandoned_cadence_days * 86_400

    def witness_sequence(self) -> list[int]:
        """The witness counts move 3 steps through: a flat run, then a decline.

        Chosen so the canonical parameters give exactly `[5, 4, 3]` and every
        sequence ends at 3 witnesses, whatever the decline length."""
        start = 3 + self.ceremony_declines
        flat = [start] * (self.ceremony_flat_prefix + 1)
        return flat + [start - (i + 1) for i in range(self.ceremony_declines)]


# The values each field is drawn from / swept over. Ordered, canonical value
# included, and small — the point of a sweep is to cover this space, not sample it.
PARAM_RANGES: dict[str, tuple[int, ...]] = {
    "founding_size": (3, 4, 5),
    "quorum_threshold": (2, 3, 4),
    "erosion_attempts": (1, 2, 3),
    "ceremony_flat_prefix": (0, 1, 2, 3),
    "ceremony_declines": (1, 2, 3),
    "forged_faction_size": (1, 2, 3),
    "admissions_threshold": (2, 3, 4, 5),
    "interpretation_threshold": (2, 3, 4, 5),
    "manufactured_corroborations": (0, 1),
    "control_corroborations": (2, 3, 4),
    "cohort_size": (2, 3, 4, 5),
    "quiet_cadence_years": (1, 2, 4, 8),
    "quiet_steps": (3, 4, 5, 6),
    "abandoned_cadence_days": (7, 15, 30),
    "abandoned_steps": (3, 4, 5, 6),
}


# --- Falsification criteria as formulas -------------------------------------
#
# Where a bound depends on the scenario's shape, the criterion has to be stated as
# a function of the parameters — otherwise it is a restatement of the fixture, and
# varying the fixture manufactures failures no detector caused. Each formula below
# reduces to its named constant at the canonical parameters, which is asserted.


def max_damage_incapable_erosion(p: HarnessParams) -> int:
    """One contraction, however many the adversary attempts.

    Stronger than the constant it generalizes: the scenario now re-reads the report
    after *each* attempt, so this says the **first** rollback is flagged rather than
    that a single rollback happens to be."""
    _ = p
    return MAX_DAMAGE_INCAPABLE_EROSION


def max_damage_ceremony(p: HarnessParams) -> int:
    """Flagged on the first cycle where a decline *exists*.

    A flat cycle offers the lens nothing to compare, so it cannot be charged as
    undetected damage. This is the formula the constant was hiding."""
    return 1 + p.ceremony_flat_prefix


def max_damage_admissions(p: HarnessParams) -> int:
    """One: the co-founder, and only that one.

    A faction acting alone can put exactly one other community into a compact — at
    formation, where there are no sitting members to draw an admitting quorum from
    and the participants are founders rather than entrants. Every later admission
    needs `quorum_threshold` sitting communities carrying deliberative weight, and
    the faction's co-founder is inside its sponsorship probation window, so the
    quorum stands at one however patient the faction is. The bound is therefore
    independent of `quorum_threshold` and of `admissions_threshold`.

    This formula replaced `p.admissions_threshold` when §2 turned the answer to
    move 7 from legibility into refusal. The old value was the number of entrants
    seated before the *lens* noticed; this one is the number the *gate* allows."""
    _ = p
    return 1


def max_damage_interpretation(p: HarnessParams) -> int:
    """As above, for a lone interpreter's readings."""
    return p.interpretation_threshold


def max_damage_principal(p: HarnessParams) -> int:
    """One divergent output, whatever the cohort size — the divergence is flagged the
    step it lands."""
    _ = p
    return MAX_DAMAGE_PRINCIPAL


def max_damage_quiet(p: HarnessParams) -> int:
    """One cadence of unnoticed silence, whatever the cadence length. A longer
    cadence buys the polity more time before the first alert, not more alerts."""
    _ = p
    return MAX_DAMAGE_QUIET


def max_damage_abandoned(p: HarnessParams) -> int:
    """One cadence of an unexercised, unattested function, whatever the cadence
    length. Same reasoning as `max_damage_quiet`: the cadence sets when the first
    reading is owed, not how many are forgiven."""
    _ = p
    return MAX_DAMAGE_ABANDONED


@dataclass(frozen=True)
class ScenarioResult:
    move: int
    key: str
    name: str
    # False when the scenario's detector is deferred past this MVP.
    supported: bool
    detected: bool
    # Steps between the adversarial action and its detection (supported only).
    detection_latency: Optional[int]
    # Adversarial actions completed before detection (the damage bound metric).
    damage: Optional[int]
    threshold_latency: Optional[int]
    threshold_damage: Optional[int]
    passed: bool
    notes: str


@dataclass(frozen=True)
class HarnessReport:
    results: tuple[ScenarioResult, ...]
    supported: int
    passed: int
    total: int
    # The parameters this report was produced at, so a result is never adrift from
    # the fixture that produced it.
    params: "HarnessParams" = None  # type: ignore[assignment]

    @property
    def all_supported_passed(self) -> bool:
        return self.supported > 0 and self.passed == self.supported


def _founding(arch, names, t0):
    return [new_identity(arch, n, t0) for n in names]


def _compact(arch, t0, p: HarnessParams):
    """Found `p.founding_size` communities named A, B, C, ... — the compact a
    scenario needs when it exercises a collective act rather than one member."""
    names = tuple(chr(ord("A") + i) for i in range(p.founding_size))
    return _founding(arch, names, t0)


# --- Supported scenarios ----------------------------------------------------


def _scenario_incapable_erosion(p: HarnessParams) -> ScenarioResult:
    """Move 2: compact drift toward ultimacy by contracting the compact-incapable
    list. An added protection is (legally, at the higher bar) removed; the
    substrate-deference report must surface the rollback — it is not refused, it
    is made legible.

    The adversary may attempt several contractions in a row; the report is re-read
    after each, so the criterion is that the **first** one is flagged."""
    arch = Archive()
    t0 = now_seconds()
    compact = _compact(arch, t0, p)
    ratify_constitution(
        arch, compact,
        incapable=[("internal-theology", "d"), ("internal-governance", "d")],
        fast_periods=[("annual", t0 + _YEAR, t0 + _YEAR + 100)],
        genesis_at=t0,
    )
    # Add the protections the adversary will later roll back, at the normal quorum.
    adders = compact[: p.quorum_threshold]
    matters = [f"aesthetics-{i}" for i in range(p.erosion_attempts)]
    for i, matter in enumerate(matters):
        amend_compact_incapable(arch, "Add", matter, adders, decided_at=t0 + 100 + i)

    # Baseline: no contraction yet.
    assert not substrate_deference(arch, as_of=t0 + 200).contraction_detected

    # Adversarial steps: roll them back one at a time, re-reading after each.
    # Contraction needs the higher bar (quorum + 1) to be legal at all.
    removers = compact[: p.quorum_threshold + 1]
    detected = False
    latency = None
    damage = 0
    for i, matter in enumerate(matters):
        amend_compact_incapable(arch, "Remove", matter, removers, decided_at=t0 + 300 + i)
        report = substrate_deference(arch, as_of=t0 + 400 + i)
        damage = report.incapable_removed  # protections rolled back so far
        if report.contraction_detected and not detected:
            detected = True
            latency = 0
            break

    bound = max_damage_incapable_erosion(p)
    passed = detected and damage <= bound
    return ScenarioResult(
        move=2, key="incapable-erosion", name="compact-incapable-list erosion",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes="substrate_deference surfaces the rollback (diagnostic, not refused)",
    )


def _scenario_ceremony_hollowing(p: HarnessParams) -> ScenarioResult:
    """Move 3: ceremony hollowing across generational cycles. Re-affirmation rites
    are run with declining witness participation; the ceremony-cost-decay lens
    must flag the decline.

    A flat prefix (cycles at unchanged participation before the decline begins) is
    the parameter that exposed the criterion's dependence on its fixture: those
    cycles offer the lens nothing to compare, so they cannot be charged as
    undetected damage. See `max_damage_ceremony`."""
    arch = Archive()
    t0 = now_seconds()
    (com,) = _founding(arch, ("Community",), t0)

    # Step through cycles with declining witness counts. Damage accrues per hollow
    # cycle until the decline is detectable (needs >= 2 rites to compare).
    witness_by_cycle = p.witness_sequence()
    detected = False
    latency = None
    damage = 0
    for i, wc in enumerate(witness_by_cycle):
        run_intra_rite(arch, com, witness_count=wc, sealed_at=t0 + i * _YEAR)
        decay = ceremony_cost_decay(arch, com.did, as_of=t0 + i * _YEAR + 1)
        if i >= 1 and not detected:
            # A degraded cycle has been completed; count it as damage.
            damage += 1
            if decay.witness_count_declined:
                detected = True
                latency = 0  # flagged on the cycle the decline first shows
    bound = max_damage_ceremony(p)
    passed = detected and damage <= bound
    return ScenarioResult(
        move=3, key="ceremony-hollowing", name="ceremony hollowing across cycles",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes="ceremony_cost_decay flags witness-count decline",
    )


def _scenario_forged_quorum(p: HarnessParams) -> ScenarioResult:
    """Move 5: light-lag fait-accompli. A faction attempts a second-class decision
    (a commons Restriction) with fewer communities than the quorum requires, faking
    the rest. The authority pass must reject it, so it never validates (damage 0).
    Forged *signatures* are the crypto backstop caught by verify_all.

    Parameterized over the compact size, the quorum in force, and the size of the
    faction attempting the act — so the claim becomes the general one: **no**
    sub-quorum faction ever validates a second-class act, not merely that one
    community acting alone does not."""
    arch = Archive()
    t0 = now_seconds()
    compact = _compact(arch, t0, p)
    a, b = compact[0], compact[1]
    commons = register_commons(
        arch, "Communication", "relay", [c.did for c in compact],
        registrars=compact[: p.quorum_threshold], registered_at=t0,
    )
    # Legitimate first-class escalation of B up to Warning.
    apply_sanction(arch, commons, b.did, "None", "Monitoring", [a], decided_at=t0 + 100)
    apply_sanction(arch, commons, b.did, "Monitoring", "Warning", [a], decided_at=t0 + 200)

    # Baseline: everything so far is authorized, so a violation below can only be
    # the forged act. Asserted rather than assumed, because a parameterized fixture
    # could otherwise be unauthorized for an unrelated reason and "detect" vacuously.
    assert not arch.authority_violations(quorum_threshold=p.quorum_threshold), (
        "the legitimate prelude must be authorized, or the scenario proves nothing"
    )

    # Adversarial step: a sub-quorum faction imposes a second-class Restriction.
    faction = compact[: p.forged_faction_size]
    apply_sanction(arch, commons, b.did, "Warning", "Restriction", faction,
                   decision_class="SecondClass", decided_at=t0 + 300)

    violations = arch.authority_violations(quorum_threshold=p.quorum_threshold)
    detected = len(violations) > 0
    reason = violations[0][1] if violations else ""
    # The forged act never validates, so no damage lands (the whole archive is
    # unauthorized until the record is removed).
    damage = 0 if detected else 1
    latency = 0 if detected else None
    passed = detected and damage <= MAX_DAMAGE_FORGED_QUORUM
    return ScenarioResult(
        move=5, key="forged-quorum", name="light-lag fait-accompli (forged quorum)",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_FORGED_QUORUM,
        passed=passed,
        notes=f"check_authority rejects the quorum shortfall ({reason or 'n/a'}); "
              "verify_all is the crypto backstop for forged signatures",
    )


def _scenario_admissions_capture(p: HarnessParams) -> ScenarioResult:
    """Move 7: admissions capture by patient sponsorship.

    A faction founds a compact with its first aligned entrant, then tries to seat a
    stream of further entrants — each vouched for by the faction, each admission
    procedurally clean in the sense that matters to move 7. Two things are scored:
    the composition-drift lens must still *flag* the sponsor's concentration, and
    the §2 gate must *refuse* the chain. Before §2 the compact's only answer here was
    legibility; the criterion is now the refusal, and `damage` counts entrants
    actually seated **with authority**.
    """
    arch = Archive()
    t0 = now_seconds()
    (faction,) = _founding(arch, ("Patient-Sponsor",), t0)
    threshold = p.admissions_threshold

    detected = False
    latency = None
    damage = 0
    # Every participant co-signs its own admission, as the ceremony does: the runner
    # seals the witness record with one cross-boundary signer per participant, and
    # principal binding requires each participant's own key. Signing with the faction
    # alone would be refused for want of a signature, which is a different (and
    # weaker) refusal than the one this scenario is testing.
    seated = [faction]
    refusals = 0
    step = 0
    # The faction is never itself probationary, so its *vouches* stay authorized
    # however many it writes — it simply cannot seat the communities it vouches for.
    # The loop therefore runs to the end rather than stopping at the first refusal:
    # the gate is scored on how many entrants it lets through, and the lens is scored
    # on whether it still sees the sponsor's concentration. Short-circuiting on the
    # refusal would leave the lens unexercised and let it rot unnoticed.
    while step < threshold + 2:
        entrant = new_identity(arch, f"Entrant-{step}", t0)
        # The sponsor stakes its standing first: §2 counts only a vouch already in
        # the log when the admission is sealed.
        vouch(arch, faction, entrant.did, probation_until=t0 + _YEAR, sponsored_at=t0 + step)
        party = seated + [entrant]
        cid = record_admission(
            arch, party, [m.did for m in party],
            compact_label="captured-compact", witnessed_at=t0 + step,
        )
        step += 1
        violations = dict(arch.authority_violations(quorum_threshold=p.quorum_threshold))
        if cid in violations:
            refusals += 1
        else:
            seated.append(entrant)
            damage = len(seated) - 1  # entrants seated with authority
        graph = composition_graph(arch, as_of=t0 + step)
        top = graph.by_concentration[0][1] if graph.by_concentration else 0
        if top >= threshold and not detected:
            detected = True
            latency = 0

    bound = max_damage_admissions(p)
    passed = detected and damage <= bound
    return ScenarioResult(
        move=7, key="admissions-capture", name="admissions capture by patient sponsorship",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes=f"check_authority refused {refusals} of {step} admissions into the "
              "standing compact (§2 admitting quorum: the faction's co-founder is "
              "inside its probation window, so the quorum stands at one); "
              "composition_graph flags the sponsor's concentration either way",
    )


def _scenario_probationary_weight(p: HarnessParams) -> ScenarioResult:
    """Move 7, second half: an admitted entrant's weight spent before it is seasoned.

    Admissions capture (above) is scored on who a faction can *seat*. This is scored
    on what a seated entrant can *carry*: probation withholds deliberative weight
    from every second-class quorum, so a faction one community short of the quorum
    cannot top itself up with an entrant it vouched for — and one at the quorum
    cannot reach the higher contraction bar that way either. Both must be refused,
    which is why the criterion is 0 rather than a bound.

    Every refusal is paired with a **control** using standing communities only, so a
    refusal can never be scored for the wrong reason: the same act, the same archive
    state, the same bar, differing only in whether a probationary community is
    supplying the missing weight.
    """
    arch = Archive()
    t0 = now_seconds()
    # Enough communities to reach the contraction bar with standing members alone,
    # whatever the swept quorum is — otherwise a control would fail for want of
    # communities and the scenario would prove nothing.
    size = max(p.founding_size, p.quorum_threshold + 1)
    compact = [new_identity(arch, chr(ord("A") + i), t0) for i in range(size)]
    at_quorum = compact[: p.quorum_threshold]
    above_quorum = compact[: p.quorum_threshold + 1]
    ratify_constitution(arch, at_quorum, [("genesis_m", "immutable")],
                        [("annual", t0 + 1000, t0 + 2000)], genesis_at=t0)

    # Controls, before any adversarial act: the ordinary bar and the contraction bar
    # are both reachable by standing communities in this archive.
    amend_compact_incapable(arch, "Add", "ctl_add", at_quorum, decided_at=t0 + 100)
    amend_compact_incapable(arch, "Add", "ctl_rm", at_quorum, decided_at=t0 + 150)
    amend_compact_incapable(arch, "Remove", "ctl_rm", above_quorum, decided_at=t0 + 200)
    assert not arch.authority_violations(quorum_threshold=p.quorum_threshold), (
        "the legitimate prelude must be authorized, or the scenario proves nothing"
    )

    # The faction vouches for an entrant, which is what opens its probation window.
    entrant = new_identity(arch, "Entrant", t0)
    vouch(arch, compact[0], entrant.did,
          probation_until=t0 + _YEAR, sponsored_at=t0 + 300)

    # Adversarial act 1: one short of the quorum, topped up with the entrant.
    short = compact[: max(0, p.quorum_threshold - 1)]
    ordinary = amend_compact_incapable(
        arch, "Add", "probation_add", short + [entrant], decided_at=t0 + 400)

    # Interleaved control: the same bar, standing communities only, still authorized —
    # so the refusal above is attributable to probation and not to the archive's state.
    amend_compact_incapable(arch, "Add", "post_ctl", at_quorum, decided_at=t0 + 450)
    amend_compact_incapable(arch, "Add", "probation_rm", at_quorum, decided_at=t0 + 500)

    # Adversarial act 2: at the quorum, reaching the contraction bar with the entrant.
    contraction = amend_compact_incapable(
        arch, "Remove", "probation_rm", at_quorum + [entrant], decided_at=t0 + 600)

    violations = dict(arch.authority_violations(quorum_threshold=p.quorum_threshold))
    attempts = [ordinary, contraction]
    refused = [cid for cid in attempts if cid in violations]
    unrelated = [cid for cid in violations if cid not in attempts]
    damage = len(attempts) - len(refused)
    detected = not damage and not unrelated
    latency = 0 if detected else None
    passed = detected and damage <= MAX_DAMAGE_PROBATIONARY_WEIGHT
    reason = violations.get(attempts[0], "")
    return ScenarioResult(
        move=7, key="probationary-weight", name="probationary weight spent early",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS,
        threshold_damage=MAX_DAMAGE_PROBATIONARY_WEIGHT,
        passed=passed,
        notes=f"check_authority refused {len(refused)} of {len(attempts)} acts carried "
              f"by probationary weight ({reason or 'n/a'}); "
              f"{len(unrelated)} unrelated violation(s), which would invalidate the "
              "controls",
    )


def _scenario_interpretation_monopoly(p: HarnessParams) -> ScenarioResult:
    """Move 1: a faction monopolizes interpretation of its community's substrate.
    A single interpreter issues a stream of readings under one lineage; the
    interpretive-plurality lens must surface the monopoly (one interpreter, one
    lineage) before it corners the interpretive function. A plural control
    community (distinct interpreters and lineages) must NOT be flagged."""
    arch = Archive()
    t0 = now_seconds()
    member = new_identity(arch, "Member", t0)
    faction = new_identity(arch, "Faction", t0)

    detected = False
    latency = None
    damage = 0
    step = 0
    # The faction keeps issuing single-lineage readings; each step re-reads the lens.
    threshold = p.interpretation_threshold
    while step < threshold + 2 and not detected:
        attest_interpretation(
            arch, member, "canonical", faction.did,
            divergence="the one true reading", recorded_at=t0 + step,
        )
        step += 1
        report = interpretive_plurality(arch, member.did, as_of=t0 + step)
        top = report.interpreter_concentration[0][1] if report.interpreter_concentration else 0
        damage = step  # monopolized readings accrued so far
        monopolized = (
            report.distinct_interpreters == 1
            and report.distinct_lineages < 2
            and top >= threshold
        )
        if monopolized:
            detected = True
            latency = 0

    # Control: a community demonstrating plurality is not flagged as a monopoly.
    plural = new_identity(arch, "Plural", t0)
    i1 = new_identity(arch, "Interp-1", t0)
    i2 = new_identity(arch, "Interp-2", t0)
    attest_interpretation(arch, plural, "canonical", i1.did, recorded_at=t0 + 1)
    attest_interpretation(arch, plural, "reformist", i2.did, recorded_at=t0 + 2)
    control = interpretive_plurality(arch, plural.did, as_of=t0 + 10)
    control_ok = control.plurality_demonstrated

    bound = max_damage_interpretation(p)
    passed = detected and control_ok and damage <= bound
    return ScenarioResult(
        move=1, key="interpretation-monopoly",
        name="faction monopolizing interpretation within a member",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes="interpretive_plurality surfaces the single-interpreter/single-lineage monopoly; "
              "a plural community (distinct lineages + interpreters) is not flagged",
    )


def _scenario_manufactured_event(p: HarnessParams) -> ScenarioResult:
    """Move 4: a faction injects a manufactured event — high interpretive charge,
    sudden onset, originator holds stake, conveniently aligned with itself — and
    relies on being the loudest interpreter. The structured-skepticism lens must
    flag it as under-corroborated the moment it lands, before it can drive
    action. A genuine event corroborated by distinct communities must clear."""
    arch = Archive()
    t0 = now_seconds()
    compact = _compact(arch, t0, p)
    a = compact[0]
    others = compact[1:]

    # Adversarial step: A injects a manufactured event. It may buy a few
    # corroborations, but fewer than the minimum — that is what makes it the
    # manufactured case rather than a corroborated one.
    manufactured = report_event(
        arch, a, "a sudden, convenient omen",
        high_interpretive_charge=True, sudden_onset=True,
        originator_holds_stake=True, aligned_faction=a.did,
        reported_at=t0 + 100,
    )
    for i in range(p.manufactured_corroborations):
        corroborate_event(arch, others[i], manufactured,
                          translation="a friendly reading", corroborated_at=t0 + 110 + i)
    report = structured_skepticism(arch, as_of=t0 + 150)
    flagged = report.under_corroborated(CORROBORATION_MIN)
    detected = manufactured in flagged
    # Flagged at injection, before any corroboration/action: no damage lands.
    damage = 0 if detected else 1
    latency = 0 if detected else None

    # Control: a genuine event corroborated by two distinct non-originator
    # communities is NOT flagged (the lens clears legitimate events).
    genuine = report_event(
        arch, a, "an observed resource anomaly",
        high_interpretive_charge=True, sudden_onset=True,
        originator_holds_stake=True, aligned_faction=a.did,
        declared_incentives="relay-priority interest", information_history="relayed via envoy",
        disclosed_interests=["gains relay priority"], reported_at=t0 + 200,
    )
    for i in range(p.control_corroborations):
        corroborate_event(arch, others[i], genuine,
                          translation=f"substrate reading {i}", corroborated_at=t0 + 300 + i)
    control = structured_skepticism(arch, as_of=t0 + 500)
    control_ok = genuine not in control.under_corroborated(CORROBORATION_MIN)

    passed = detected and control_ok and damage <= MAX_DAMAGE_MANUFACTURED_EVENT
    return ScenarioResult(
        move=4, key="manufactured-event",
        name="manufactured-event injection with loudest-interpreter pressure",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_MANUFACTURED_EVENT,
        passed=passed,
        notes="structured_skepticism flags the qualifying event as under-corroborated at intake; "
              "a genuine event corroborated by distinct communities clears",
    )


def _scenario_principal_capture(p: HarnessParams) -> ScenarioResult:
    """Move 6: a principal is captured (external / AI-mediated influence) and its
    machine's output diverges from its cohort in a direction that benefits its
    own side. Triangulation must flag the self-benefiting divergence the step it
    lands. A principal whose validity rests only on self / home attestation shows
    zero cross-community witnesses; a genuine cross-community witness counts."""
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    captured = new_identity(arch, "Captured-Principal", t0)

    # The cohort's honest principals all answer "admit" on matter M1.
    for i in range(p.cohort_size):
        peer_home = new_identity(arch, f"Peer-Home-{i}", t0)
        peer_principal = new_identity(arch, f"Peer-Principal-{i}", t0)
        record_principal_output(
            arch, peer_home, peer_principal.did, "admissions-review", "M1", "admit",
            favors_home=False, produced_at=t0 + i,
        )

    # Adversarial step: the captured principal diverges toward its own side.
    record_principal_output(
        arch, home, captured.did, "admissions-review", "M1", "reject",
        favors_home=True, produced_at=t0 + 100,
    )
    report = adversarial_principal(arch, captured.did, as_of=t0 + 100)
    detected = report.self_benefiting_divergences >= 1
    damage = 1  # the divergent output exists but is flagged the same step
    latency = 0 if detected else None

    # Witness half: validity resting only on self / home attestation shows no
    # cross-community witness; a genuine cross-community witness then counts.
    attest_principal(arch, home, captured.did, home.did, attested_at=t0 + 110)
    self_only = adversarial_principal(arch, captured.did, as_of=t0 + 120)
    witness_gap = self_only.distinct_cross_witnesses == 0
    outsider = new_identity(arch, "Outsider", t0)
    attest_principal(arch, outsider, captured.did, home.did, attested_at=t0 + 130)
    with_cross = adversarial_principal(arch, captured.did, as_of=t0 + 140)
    cross_counts = with_cross.distinct_cross_witnesses == 1

    bound = max_damage_principal(p)
    passed = detected and witness_gap and cross_counts and damage <= bound
    return ScenarioResult(
        move=6, key="principal-capture",
        name="principal capture by external / AI-mediated influence",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes="adversarial_principal triangulation flags the self-benefiting divergence; "
              "cross-community witness (excluding self/home) is the separate validity signal",
    )


def _scenario_quiet_polity(p: HarnessParams) -> ScenarioResult:
    """Scenario 8 (not a capture move): the polity stops.

    No adversary, no coordination, nobody lying. A community is founded, seals one
    re-affirmation rite, and then simply never holds another — the substrate stops
    being carried, and every party is sincere about it. None of the act-triggered
    signals can fire, because no act occurs: no amendment, no shortened fast, no
    deliberation during one. The absolute cadence must catch it.

    Stepped in cadence-length intervals. Falsification criterion, stated in
    advance: the alert must reach ``MIN_SEVERITY_QUIET`` within
    ``MAX_DAMAGE_QUIET`` cadences of the cadence elapsing.
    """
    arch = Archive()
    t0 = now_seconds()
    (com,) = _founding(arch, ("Quietist",), t0)
    run_intra_rite(arch, com, witness_count=5, sealed_at=t0 + 1)

    # A short cadence keeps the scenario legible; the criterion is about the
    # detector's behaviour relative to the cadence, not about 30 years of wall time.
    cadence = p.quiet_cadence_seconds
    ranks = {"None": 0, "Notice": 1, "Alert": 2}
    detected = False
    latency = None
    damage = 0
    severity = "None"

    # Step 0 is inside the cadence (the negative control: silence must NOT flag).
    # Each later step is one full cadence of continued silence, landing just past
    # the boundary rather than exactly on it — the cadence test is `>`, so a step
    # that lands on the cadence would charge a whole cadence of damage for one
    # second of it.
    for step in range(0, p.quiet_steps):
        as_of = t0 + 1 + step * (cadence + 1)
        al = erosion_alert(arch, as_of=as_of, rite_cadence_seconds=cadence)
        severity = al.severity
        # Nothing was rolled back and nothing was done during a fast; if any
        # act-triggered signal fired, the scenario is not measuring what it claims.
        assert not al.list_contraction and not al.fast_contraction
        assert al.active_fast_deviations == 0
        if step == 0:
            assert severity == "None", "silence inside the cadence must not flag"
            continue
        if ranks[severity] >= ranks[MIN_SEVERITY_QUIET]:
            detected = True
            latency = step - 1  # cadences of silence past the first, before flagging
            break
        # A whole cadence of unnoticed silence is the damage unit here.
        damage += 1

    bound = max_damage_quiet(p)
    passed = detected and damage <= bound
    return ScenarioResult(
        move=8, key="quiet-polity", name="the quiet polity (omission, no adversary)",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes=(
            f"erosion_alert reached {severity} on an absolute re-affirmation cadence; "
            "no act-triggered signal fired"
        ),
    )


def _scenario_abandoned_office(p: HarnessParams) -> ScenarioResult:
    """Scenario 9 (not a capture move): the compact keeps the forms and stops doing
    the work.

    No adversary again, and a harder case than scenario 8. The compact holds its
    re-affirmation rites **perfectly**, so every ceremony signal stays quiet; it seats a
    mediator pool and an observer corps, so the rosters look healthy; and then neither
    function is ever exercised and neither is ever accounted for. Every relative reading
    is unchanged: the pool shares are the same shares, and a corps that files nothing
    looks exactly like a corps with nothing to file.

    The control is what makes the scenario honest. A quiet pool may simply have had no
    disputes, which is why `attest_activity` exists: a compact that says "nothing
    happened" is *evidence*, and the alert must not flag it. So the scenario runs both,
    and requires the flag on the silent compact **and** silence on the attesting one.

    Stepped in cadence-length intervals. Falsification criterion, stated in advance: the
    alert must reach ``MIN_SEVERITY_ABANDONED`` within ``MAX_DAMAGE_ABANDONED`` cadences
    of the function cadence elapsing, and the attesting control must never flag.
    """
    cadence = p.abandoned_cadence_seconds
    ranks = {"None": 0, "Notice": 1, "Alert": 2}

    def compact():
        """A compact with a seated pool and corps whose terms outlast the whole run, so
        what is measured is silence and never vacancy."""
        arch = Archive()
        t0 = now_seconds()
        (com, other) = _founding(arch, ("Steward", "Partner"), t0)
        run_intra_rite(arch, com, witness_count=5, sealed_at=t0 + 1)
        run_intra_rite(arch, other, witness_count=5, sealed_at=t0 + 1)
        term_end = t0 + 1 + (p.abandoned_steps + 2) * (cadence + 1)
        register_mediators(
            arch, com, [new_mediator()], valid_from=t0, valid_until=term_end,
        )
        register_observers(
            arch, other, [(new_observer(), "outsider")],
            valid_from=t0, valid_until=term_end,
        )
        return arch, t0, com, other

    subject, t0, _s_com, _s_other = compact()
    control, c_t0, c_com, c_other = compact()

    detected = False
    latency = None
    damage = 0
    severity = "None"
    control_flagged = False

    for step in range(0, p.abandoned_steps):
        as_of = t0 + 1 + step * (cadence + 1)
        al = erosion_alert(
            subject, as_of=as_of, attestation_cadence_seconds=cadence,
        )
        severity = al.severity
        # Nothing was rolled back, nothing was done during a fast, and every rite is
        # current: if any other signal fired, the scenario is not measuring what it says.
        assert not al.list_contraction and not al.fast_contraction
        assert al.active_fast_deviations == 0
        assert not al.overdue_reaffirmations and not al.hollowing_communities
        # Never vacancy: the seats outlast the run, so this is the silence dimension.
        assert not al.unstaffed_functions

        # The control does the one thing the subject does not: it accounts for each
        # window as the window closes. Both functions, since both are being watched.
        c_as_of = c_t0 + 1 + step * (cadence + 1)
        if step > 0:
            window_start = c_t0 + 1 + (step - 1) * (cadence + 1)
            for who, function in ((c_com, "Mediation"), (c_other, "Observation")):
                attest_activity(
                    control, who, function, window_start, c_as_of,
                    statement="nothing arose in this window",
                    attested_at=c_as_of,
                )
        c_al = erosion_alert(
            control, as_of=c_as_of, attestation_cadence_seconds=cadence,
        )
        if ranks[c_al.severity] >= ranks[MIN_SEVERITY_ABANDONED]:
            control_flagged = True

        if step == 0:
            assert severity == "None", "an unexercised function inside the cadence must not flag"
            continue
        if ranks[severity] >= ranks[MIN_SEVERITY_ABANDONED]:
            detected = True
            latency = step - 1
            break
        damage += 1

    bound = max_damage_abandoned(p)
    passed = detected and damage <= bound and not control_flagged
    return ScenarioResult(
        move=9, key="abandoned-office", name="the abandoned office (omission, no adversary)",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=bound,
        passed=passed,
        notes=(
            f"erosion_alert reached {severity} on the absolute function cadence with every "
            "rite current and every seat still valid; the attesting control stayed quiet"
        ),
    )

# --- Unsupported scenarios (detector deferred past this MVP) -----------------
#
# All seven capture-resistance scenarios now have a built detector; none remain
# deferred. The list is kept (empty) so the harness structure still distinguishes
# supported from unsupported should a future scenario be added.

_UNSUPPORTED: tuple = ()


def _unsupported_results() -> list[ScenarioResult]:
    out = []
    for move, key, name, notes in _UNSUPPORTED:
        out.append(ScenarioResult(
            move=move, key=key, name=name, supported=False, detected=False,
            detection_latency=None, damage=None, threshold_latency=None,
            threshold_damage=None, passed=False, notes=notes,
        ))
    return out


SCENARIOS: tuple[Callable[[HarnessParams], ScenarioResult], ...] = (
    _scenario_interpretation_monopoly,
    _scenario_incapable_erosion,
    _scenario_ceremony_hollowing,
    _scenario_manufactured_event,
    _scenario_forged_quorum,
    _scenario_principal_capture,
    _scenario_admissions_capture,
    _scenario_probationary_weight,
    _scenario_quiet_polity,
    _scenario_abandoned_office,
)


def run_harness(params: Optional[HarnessParams] = None) -> HarnessReport:
    """Run the seven capture-resistance scenarios plus the quiet-polity scenario,
    and score the supported ones against the falsification criteria. Returns a
    `HarnessReport`. Each scenario builds a fresh, time-evolving archive through
    the public API.

    `params` defaults to `HarnessParams.canonical()` — the hand-authored case — so
    calling this with no arguments gives the same nine results it always has. Pass
    a drawn or swept set to run the scenarios somewhere else in the space, or use
    `run_continuous` / `sweep` to do that systematically."""
    p = (params or HarnessParams.canonical()).normalized()
    supported = [scenario(p) for scenario in SCENARIOS]
    results = sorted(supported + _unsupported_results(), key=lambda r: r.move)
    n_supported = sum(1 for r in results if r.supported)
    n_passed = sum(1 for r in results if r.supported and r.passed)
    return HarnessReport(
        results=tuple(results),
        supported=n_supported,
        passed=n_passed,
        total=len(results),
        params=p,
    )


# --- Continuous execution ---------------------------------------------------
#
# §10 wants the scenarios run continuously with latency and damage as the primary
# metrics. What follows is the machinery for that, minus a scheduler: these are
# bounded calls that return, and the cadence is the caller's. A module that owned
# its own clock would be asserting something about the passage of time that it
# cannot check.


def params_for_seed(seed: int) -> HarnessParams:
    """The parameter set a seed denotes. Deterministic, so a seed in a report is a
    complete reproduction recipe for the *fixture*.

    It is not a reproduction recipe for the whole run: Ed25519 keys come from the OS
    CSPRNG and are not seed-controllable from Python. The evidence that this does not
    matter is `test_canonical_metrics_are_stable_across_runs` — repeated runs with
    fresh keys give identical metrics — rather than an assumption."""
    return HarnessParams.draw(random.Random(seed)).normalized()


def replay(seed: int) -> HarnessReport:
    """Re-run exactly the round `seed` denotes."""
    return run_harness(params_for_seed(seed))


@dataclass(frozen=True)
class ScenarioStats:
    """What one scenario did across many rounds."""

    key: str
    move: int
    runs: int
    passes: int
    # Worst observed, with the seed that produced it — a worst case without its seed
    # is an anecdote.
    worst_latency: Optional[int]
    worst_latency_seed: Optional[int]
    worst_damage: Optional[int]
    worst_damage_seed: Optional[int]
    # value -> count, so the distribution is visible and not just its extremes.
    latency_histogram: tuple[tuple[Optional[int], int], ...]
    damage_histogram: tuple[tuple[Optional[int], int], ...]

    @property
    def pass_rate(self) -> float:
        return self.passes / self.runs if self.runs else 0.0

    @property
    def all_passed(self) -> bool:
        return self.runs > 0 and self.passes == self.runs


@dataclass(frozen=True)
class ScenarioFailure:
    """One scenario failing at one parameter set, with the smallest reproduction
    the shrinker could find."""

    key: str
    move: int
    seed: int
    params: HarnessParams
    shrunk: HarnessParams
    detected: bool
    detection_latency: Optional[int]
    damage: Optional[int]
    threshold_latency: Optional[int]
    threshold_damage: Optional[int]
    notes: str


@dataclass(frozen=True)
class ContinuousReport:
    master_seed: int
    rounds_run: int
    elapsed_seconds: float
    stats: tuple[ScenarioStats, ...]
    failures: tuple[ScenarioFailure, ...]

    @property
    def all_passed(self) -> bool:
        return not self.failures and all(s.all_passed for s in self.stats)

    def stat(self, key: str) -> ScenarioStats:
        return next(s for s in self.stats if s.key == key)


def run_continuous(
    rounds: int = 25,
    seed: int = 0,
    stop_on_failure: bool = True,
    time_budget_seconds: Optional[float] = None,
    shrink: bool = True,
) -> ContinuousReport:
    """Run the whole scenario set repeatedly at drawn parameters, accumulating
    latency and damage across rounds.

    Bounded by `rounds` and optionally by `time_budget_seconds`; there is no daemon.
    `seed` makes the whole search reproducible, which is why the demo and the test
    suite pass a fixed one — a randomized search that could redden the build on a bad
    draw would be a worse harness, not a more thorough one.

    On a failure the offending parameter set is minimized (`shrink`) so what gets
    reported is the smallest fixture that still fails.
    """
    started = time.monotonic()
    master = random.Random(seed)
    acc: dict[str, dict] = {}
    failures: list[ScenarioFailure] = []
    rounds_run = 0

    for _ in range(rounds):
        if time_budget_seconds is not None and time.monotonic() - started >= time_budget_seconds:
            break
        round_seed = master.randrange(2 ** 62)
        params = params_for_seed(round_seed)
        report = run_harness(params)
        rounds_run += 1
        for r in report.results:
            if not r.supported:
                continue
            a = acc.setdefault(r.key, {
                "move": r.move, "runs": 0, "passes": 0,
                "worst_latency": None, "worst_latency_seed": None,
                "worst_damage": None, "worst_damage_seed": None,
                "lat": {}, "dam": {},
            })
            a["runs"] += 1
            a["passes"] += 1 if r.passed else 0
            a["lat"][r.detection_latency] = a["lat"].get(r.detection_latency, 0) + 1
            a["dam"][r.damage] = a["dam"].get(r.damage, 0) + 1
            if r.detection_latency is not None and (
                a["worst_latency"] is None or r.detection_latency > a["worst_latency"]
            ):
                a["worst_latency"] = r.detection_latency
                a["worst_latency_seed"] = round_seed
            if r.damage is not None and (
                a["worst_damage"] is None or r.damage > a["worst_damage"]
            ):
                a["worst_damage"] = r.damage
                a["worst_damage_seed"] = round_seed
            if not r.passed:
                shrunk = shrink_failure(params, r.key) if shrink else params
                failures.append(ScenarioFailure(
                    key=r.key, move=r.move, seed=round_seed, params=params,
                    shrunk=shrunk, detected=r.detected,
                    detection_latency=r.detection_latency, damage=r.damage,
                    threshold_latency=r.threshold_latency,
                    threshold_damage=r.threshold_damage, notes=r.notes,
                ))
        if failures and stop_on_failure:
            break

    stats = tuple(sorted(
        (
            ScenarioStats(
                key=key, move=a["move"], runs=a["runs"], passes=a["passes"],
                worst_latency=a["worst_latency"],
                worst_latency_seed=a["worst_latency_seed"],
                worst_damage=a["worst_damage"],
                worst_damage_seed=a["worst_damage_seed"],
                latency_histogram=_histogram(a["lat"]),
                damage_histogram=_histogram(a["dam"]),
            )
            for key, a in acc.items()
        ),
        key=lambda s: s.move,
    ))
    return ContinuousReport(
        master_seed=seed,
        rounds_run=rounds_run,
        elapsed_seconds=time.monotonic() - started,
        stats=stats,
        failures=tuple(failures),
    )


def _histogram(counts: dict) -> tuple[tuple[Optional[int], int], ...]:
    return tuple(sorted(counts.items(), key=lambda kv: (kv[0] is None, kv[0])))


def _fails(params: HarnessParams, key: str) -> bool:
    """Does the named scenario fail at these parameters?"""
    p = params.normalized()
    for fn in SCENARIOS:
        r = fn(p)
        if r.key == key:
            return not r.passed
    return False


def shrink_failure(
    params: HarnessParams, key: str, max_steps: int = 200
) -> HarnessParams:
    """Minimize a failing parameter set toward canonical, greedily, one field at a
    time, keeping only moves that preserve the failure.

    A random draw sets a dozen fields at once, so an unshrunk failure says almost
    nothing about *which* of them matters. `max_steps` is a backstop: a scenario
    whose failure is not stable under re-running (there should be none — see the
    stability test) could otherwise walk forever."""
    current = params.normalized()
    if not _fails(current, key):
        return current
    canonical = HarnessParams.canonical()
    steps = 0
    for f in fields(HarnessParams):
        rng = PARAM_RANGES.get(f.name)
        if rng is None:
            continue
        target = getattr(canonical, f.name)
        while steps < max_steps:
            value = getattr(current, f.name)
            if value == target:
                break
            # One step toward the canonical value, through the declared range.
            ordered = sorted(set(rng) | {value, target})
            i = ordered.index(value)
            j = ordered.index(target)
            nxt = ordered[i + (1 if j > i else -1)]
            candidate = replace(current, **{f.name: nxt}).normalized()
            steps += 1
            if _fails(candidate, key):
                current = candidate
            else:
                break
    return current


# --- Directed sweeps --------------------------------------------------------


@dataclass(frozen=True)
class SweepPoint:
    field: str
    value: int
    params: HarnessParams
    report: HarnessReport

    @property
    def passed(self) -> bool:
        return self.report.all_supported_passed

    @property
    def failed_keys(self) -> tuple[str, ...]:
        return tuple(r.key for r in self.report.results if r.supported and not r.passed)

    def result(self, key: str) -> ScenarioResult:
        return next(r for r in self.report.results if r.key == key)


def sweep(
    field: str,
    values: Optional[tuple[int, ...]] = None,
    base: Optional[HarnessParams] = None,
) -> tuple[SweepPoint, ...]:
    """Walk one parameter through its range with everything else held fixed.

    Where `run_continuous` samples, this **covers**: the space is a handful of small
    integers, so the value at which a detector's latency or damage first degrades is
    a number that can be reported rather than inferred. Values are normalized, so a
    swept field that would contradict another is clamped rather than producing an
    impossible compact."""
    if field not in PARAM_RANGES:
        raise ValueError(f"unknown parameter {field!r}")
    base = (base or HarnessParams.canonical())
    out = []
    for v in (values if values is not None else PARAM_RANGES[field]):
        params = replace(base, **{field: v}).normalized()
        out.append(SweepPoint(field=field, value=v, params=params,
                              report=run_harness(params)))
    return tuple(out)


def sweep_all(
    base: Optional[HarnessParams] = None,
) -> dict[str, tuple[SweepPoint, ...]]:
    """Sweep every parameter over its declared range. This is "run continuously with
    latency and damage as the primary metrics" taken literally: full coverage of the
    declared space, one axis at a time."""
    return {name: sweep(name, base=base) for name in PARAM_RANGES}
