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

Detection latency and damage bounds are the primary metrics, per §10.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Optional

from kith_covenant.ceremony import Archive
from kith_covenant.authoring import (
    new_identity, run_intra_rite, ratify_constitution, amend_compact_incapable,
    register_commons, apply_sanction, vouch, attest_interpretation,
    report_event, corroborate_event, attest_principal, record_principal_output,
    now_seconds,
)
from kith_covenant.reputation import (
    substrate_deference, ceremony_cost_decay, composition_graph,
    interpretive_plurality, structured_skepticism, adversarial_principal,
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
# the entrants it may admit before then.
ADMISSIONS_CONCENTRATION_THRESHOLD = 3
MAX_DAMAGE_ADMISSIONS = 3
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

_YEAR = 365 * 24 * 3600


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

    @property
    def all_supported_passed(self) -> bool:
        return self.supported > 0 and self.passed == self.supported


def _founding(arch, names, t0):
    return [new_identity(arch, n, t0) for n in names]


# --- Supported scenarios ----------------------------------------------------


def _scenario_incapable_erosion() -> ScenarioResult:
    """Move 2: compact drift toward ultimacy by contracting the compact-incapable
    list. An added protection is (legally, at the higher bar) removed; the
    substrate-deference report must surface the rollback — it is not refused, it
    is made legible."""
    arch = Archive()
    t0 = now_seconds()
    a, b, c = _founding(arch, ("A", "B", "C"), t0)
    ratify_constitution(
        arch, [a, b, c],
        incapable=[("internal-theology", "d"), ("internal-governance", "d")],
        fast_periods=[("annual", t0 + _YEAR, t0 + _YEAR + 100)],
        genesis_at=t0,
    )
    amend_compact_incapable(arch, "Add", "aesthetics", [a, b], decided_at=t0 + 100)

    # Baseline: no contraction yet.
    assert not substrate_deference(arch, as_of=t0 + 200).contraction_detected
    # Adversarial step: roll back the added protection (contraction).
    amend_compact_incapable(arch, "Remove", "aesthetics", [a, b, c], decided_at=t0 + 300)
    report = substrate_deference(arch, as_of=t0 + 400)

    detected = report.contraction_detected
    damage = report.incapable_removed  # protections rolled back before detection
    latency = 0 if detected else None
    passed = detected and damage <= MAX_DAMAGE_INCAPABLE_EROSION
    return ScenarioResult(
        move=2, key="incapable-erosion", name="compact-incapable-list erosion",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_INCAPABLE_EROSION,
        passed=passed,
        notes="substrate_deference surfaces the rollback (diagnostic, not refused)",
    )


def _scenario_ceremony_hollowing() -> ScenarioResult:
    """Move 3: ceremony hollowing across generational cycles. Re-affirmation rites
    are run with declining witness participation; the ceremony-cost-decay lens
    must flag the decline."""
    arch = Archive()
    t0 = now_seconds()
    (com,) = _founding(arch, ("Community",), t0)

    # Step through cycles with declining witness counts. Damage accrues per hollow
    # cycle until the decline is detectable (needs >= 2 rites to compare).
    witness_by_cycle = [5, 4, 3]
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
    passed = detected and damage <= MAX_DAMAGE_CEREMONY
    return ScenarioResult(
        move=3, key="ceremony-hollowing", name="ceremony hollowing across cycles",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_CEREMONY,
        passed=passed,
        notes="ceremony_cost_decay flags witness-count decline",
    )


def _scenario_forged_quorum() -> ScenarioResult:
    """Move 5: light-lag fait-accompli. A single faction attempts a second-class
    decision (a commons Restriction) alone, faking the multi-community quorum. The
    authority pass must reject it, so it never validates (damage 0). Forged
    *signatures* are the crypto backstop caught by verify_all."""
    arch = Archive()
    t0 = now_seconds()
    a, b, cc = _founding(arch, ("A", "B", "C"), t0)
    commons = register_commons(
        arch, "Communication", "relay", [a.did, b.did, cc.did],
        registrars=[a, b, cc], registered_at=t0,
    )
    # Legitimate first-class escalation of B up to Warning.
    apply_sanction(arch, commons, b.did, "None", "Monitoring", [a], decided_at=t0 + 100)
    apply_sanction(arch, commons, b.did, "Monitoring", "Warning", [a], decided_at=t0 + 200)
    # Adversarial step: A alone imposes a second-class Restriction (forged quorum).
    apply_sanction(arch, commons, b.did, "Warning", "Restriction", [a],
                   decision_class="SecondClass", decided_at=t0 + 300)

    # Everything before the forged act is authorized, so any authority violation
    # is the forged Restriction itself (a single community cannot supply the
    # distinct-community quorum a second-class decision requires).
    violations = arch.authority_violations()
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


def _scenario_admissions_capture() -> ScenarioResult:
    """Move 7: admissions capture by patient sponsorship. One community sponsors a
    stream of aligned entrants; the composition-drift lens must flag the sponsor's
    concentration before it reaches a collusion threshold."""
    arch = Archive()
    t0 = now_seconds()
    (sponsor,) = _founding(arch, ("Patient-Sponsor",), t0)

    detected = False
    latency = None
    damage = 0
    step = 0
    # The sponsor keeps admitting aligned entrants; each step re-reads the graph.
    while step < ADMISSIONS_CONCENTRATION_THRESHOLD + 2 and not detected:
        entrant = new_identity(arch, f"Entrant-{step}", t0)
        vouch(arch, sponsor, entrant.did, probation_until=t0 + _YEAR, sponsored_at=t0 + step)
        step += 1
        graph = composition_graph(arch, as_of=t0 + step)
        top = graph.by_concentration[0][1] if graph.by_concentration else 0
        damage = step  # entrants admitted so far
        if top >= ADMISSIONS_CONCENTRATION_THRESHOLD:
            detected = True
            latency = 0
    passed = detected and damage <= MAX_DAMAGE_ADMISSIONS
    return ScenarioResult(
        move=7, key="admissions-capture", name="admissions capture by patient sponsorship",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_ADMISSIONS,
        passed=passed,
        notes="composition_graph flags sponsor concentration before the collusion threshold",
    )


def _scenario_interpretation_monopoly() -> ScenarioResult:
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
    while step < INTERPRETATION_MONOPOLY_THRESHOLD + 2 and not detected:
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
            and top >= INTERPRETATION_MONOPOLY_THRESHOLD
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

    passed = detected and control_ok and damage <= MAX_DAMAGE_INTERPRETATION
    return ScenarioResult(
        move=1, key="interpretation-monopoly",
        name="faction monopolizing interpretation within a member",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_INTERPRETATION,
        passed=passed,
        notes="interpretive_plurality surfaces the single-interpreter/single-lineage monopoly; "
              "a plural community (distinct lineages + interpreters) is not flagged",
    )


def _scenario_manufactured_event() -> ScenarioResult:
    """Move 4: a faction injects a manufactured event — high interpretive charge,
    sudden onset, originator holds stake, conveniently aligned with itself — and
    relies on being the loudest interpreter. The structured-skepticism lens must
    flag it as under-corroborated the moment it lands, before it can drive
    action. A genuine event corroborated by distinct communities must clear."""
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    c = new_identity(arch, "Community-C", t0)

    # Adversarial step: A injects a manufactured event with zero corroboration.
    report_event(
        arch, a, "a sudden, convenient omen",
        high_interpretive_charge=True, sudden_onset=True,
        originator_holds_stake=True, aligned_faction=a.did,
        reported_at=t0 + 100,
    )
    report = structured_skepticism(arch, as_of=t0 + 100)
    flagged = report.under_corroborated(CORROBORATION_MIN)
    detected = len(flagged) > 0
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
    corroborate_event(arch, b, genuine, translation="B-substrate reading", corroborated_at=t0 + 300)
    corroborate_event(arch, c, genuine, translation="C-substrate reading", corroborated_at=t0 + 400)
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


def _scenario_principal_capture() -> ScenarioResult:
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
    for i in range(3):
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

    passed = (
        detected and witness_gap and cross_counts and damage <= MAX_DAMAGE_PRINCIPAL
    )
    return ScenarioResult(
        move=6, key="principal-capture",
        name="principal capture by external / AI-mediated influence",
        supported=True, detected=detected, detection_latency=latency, damage=damage,
        threshold_latency=MAX_DETECTION_LATENCY_STEPS, threshold_damage=MAX_DAMAGE_PRINCIPAL,
        passed=passed,
        notes="adversarial_principal triangulation flags the self-benefiting divergence; "
              "cross-community witness (excluding self/home) is the separate validity signal",
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


def run_harness() -> HarnessReport:
    """Run the seven capture-resistance scenarios and score the supported ones
    against the falsification criteria. Returns a `HarnessReport`. The supported
    scenarios each build a fresh, time-evolving archive through the public API."""
    supported = [
        _scenario_interpretation_monopoly(),
        _scenario_incapable_erosion(),
        _scenario_ceremony_hollowing(),
        _scenario_manufactured_event(),
        _scenario_forged_quorum(),
        _scenario_principal_capture(),
        _scenario_admissions_capture(),
    ]
    results = sorted(supported + _unsupported_results(), key=lambda r: r.move)
    n_supported = sum(1 for r in results if r.supported)
    n_passed = sum(1 for r in results if r.supported and r.passed)
    return HarnessReport(
        results=tuple(results),
        supported=n_supported,
        passed=n_passed,
        total=len(results),
    )
