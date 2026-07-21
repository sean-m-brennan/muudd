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

"""Author signed records into a live archive from Python.

Signing happens in the Rust core; a `Signer` holds the raw Ed25519 key bytes
the caller is responsible for. A record's signer identity must already be
published (`new_identity`) for it to verify, so the usual flow is:

    arch = Archive()
    hub = new_identity(arch, "Hub")
    entrant = new_identity(arch, "Entrant")
    vouch(arch, hub, entrant.did, probation_until=now_seconds() + 300*86400)
    record_concern(arch, hub, entrant.did, feature="NonUltimacyOfCompact",
                   perspective="charter-originalist observer")
    arch.verify_all()   # crypto-verifies the whole commons

Records do not auto-verify on append (the signer identity may be published in
the same batch); call `archive.verify_all()` once the batch is complete.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional, Sequence, Union

from kith_covenant import _core

now_seconds = _core.now_seconds

# Valid string values for the conduct enums, for discoverability / validation.
CONDUCT_KINDS = (
    "Commendation", "Concern", "Violation",
    "DesignationAnathema", "DesignationLifted", "RestorationCompleted",
)
ANTI_SPIRAL_FEATURES = (
    "FallenActorAnthropology", "GraceOverMerit", "ScapegoatDisarming",
    "EnemyLoveOperational", "NonUltimacyOfCompact",
)
BEARINGS = ("AntiSpiral", "CommonsObligation", "Charity", "Rescue", "Other")
SEVERITIES = ("Minor", "Serious", "Grave")
LADDER_RUNGS = (
    "DisputeRegistered", "RegisteredProtest", "ReciprocalSuspension",
    "MediatedArbitration", "LimitedSanctions", "CollectiveSanctions",
    "Expulsion", "SubstantiveReEngagement",
)
DECISION_CLASSES = ("FirstClass", "SecondClass")
DELEGATION_SCOPES = (
    "Reciter", "Envoy", "InternalWitness", "CrossBoundaryWitness",
)
# §4 mediation-panel decision outcomes.
DECISION_OUTCOMES = ("Upheld", "Dismissed", "Settled", "Referred")
# §5 designation tiers and urgencies.
DESIGNATION_TIERS = ("Anathema", "HostisHumaniGeneris")
DESIGNATION_URGENCIES = ("Normal", "Expedited")
# §9 compact-gated capabilities an interim injunction can suspend.
CAPABILITIES = (
    "RescueCoordination", "Navigation", "Trade", "CommonsWithdrawal", "Communication",
)
# §7 the five compact-level commons.
COMMONS_KINDS = (
    "LifeSupport", "Communication", "Navigation", "KnowledgeArchive", "Rescue",
)
# §7 graduated-sanction ladder levels (rising severity; None is the resting state).
SANCTION_LEVELS = ("None", "Monitoring", "Warning", "Restriction", "Exclusion")


@dataclass(frozen=True)
class Signer:
    """A key that signs records, plus the community the signature speaks for.

    `did` identifies the signing key: for a community's root identity it is the
    community DID; for a delegate (see `delegate_speaker`) it is the delegate
    key's DID. `principal` is the community a signature is *attributed* to in the
    record payload — it defaults to `did`, so a root identity speaks for itself,
    while a delegate speaks for the community that delegated it.

    `priv` is a secret; treat this object as sensitive. `Signer` deliberately
    does not print its key material.
    """

    did: str
    priv: bytes
    pub: bytes
    principal: str = field(default=None)  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.principal is None:  # a root identity speaks for itself
            object.__setattr__(self, "principal", self.did)

    def __repr__(self) -> str:  # never leak key bytes in logs/reprs
        return f"Signer(did={self.did!r}, principal={self.principal!r}, priv=<redacted>)"


def new_identity(archive, name: str, created_at: Optional[int] = None) -> Signer:
    """Generate a fresh community keypair, publish its self-signed identity into
    `archive`, and return a `Signer` bound to it (principal == did)."""
    priv, pub, _did = _core.generate_ed25519_keypair()
    did, _cid = archive.publish_identity(name, priv, pub, created_at)
    return Signer(did=did, priv=priv, pub=pub)


def delegate_speaker(
    archive,
    community: Signer,
    scope: str,
    *,
    issued_at: Optional[int] = None,
    expires_at: Optional[int] = None,
) -> Signer:
    """Authorize a fresh delegate key to sign in `scope` on `community`'s behalf.

    Generates a new keypair, publishes a `SpeakerDelegation` (signed by the
    community root key) into `archive`, and returns a delegate `Signer` whose
    `did` is the new key and whose `principal` is `community.principal`. Records
    authored with the returned signer are attributed to the community but signed
    by the delegate key, and verify through the delegation chain.

    `scope` is one of `DELEGATION_SCOPES`. The delegation is active from
    `issued_at` (default: now) until `expires_at` (default: open-ended). It must
    be active at each record's `recorded_at`/`sponsored_at`, so to author
    back-dated records pass an earlier `issued_at`.

    Note: verification confirms the signature comes from a validly-delegated,
    active key, but does not independently bind the delegation to `principal` or
    check that `scope` authorizes the act — that authority enforcement is a
    separate protocol concern. This helper links them by construction.
    """
    if issued_at is None:
        issued_at = now_seconds()
    priv, pub, did = _core.generate_ed25519_keypair()
    archive.delegate_speaker(
        community.priv, community.did, pub, did, scope, issued_at, expires_at,
    )
    return Signer(did=did, priv=priv, pub=pub, principal=community.principal)


def revoke_delegation(
    archive,
    community: Signer,
    speaker_did: str,
    *,
    issued_at: Optional[int] = None,
    revoked_at: Optional[int] = None,
) -> str:
    """Revoke a prior delegation to `speaker_did`, appending a superseding
    `SpeakerDelegation` tombstone signed by the community root. Because
    resolution is latest-issued-at wins, records the delegate signs dated at or
    after `revoked_at` stop verifying and the delegation drops out of
    active_delegations; a later re-delegation reactivates it. `community` is the
    root `Signer`. Returns the tombstone CID."""
    return archive.revoke_delegation(
        community.priv, community.did, speaker_did, issued_at, revoked_at,
    )


def vouch(
    archive,
    sponsor: Signer,
    entrant_did: str,
    probation_until: int,
    sponsored_at: Optional[int] = None,
    statement: str = "",
    admission_cid: Optional[str] = None,
) -> str:
    """Record `sponsor` vouching for `entrant_did`, staking the sponsor's
    standing on the entrant's conduct until `probation_until`. The vouch is
    attributed to `sponsor.principal` (the community) and signed by
    `sponsor.did`/`.priv` (which may be a delegate key). Returns the CID."""
    if sponsored_at is None:
        sponsored_at = now_seconds()
    return archive.append_sponsorship(
        sponsor.priv, sponsor.did, sponsor.principal, entrant_did,
        sponsored_at, probation_until, statement, admission_cid,
    )


def attest_interpretation(
    archive,
    community: Signer,
    lineage_id: str,
    interpreter_did: str,
    *,
    divergence: str = "",
    interprets_cid: Optional[str] = None,
    references: Optional[Sequence[str]] = None,
    recorded_at: Optional[int] = None,
) -> str:
    """Record one point on `community`'s interpretive lineage of its own
    substrate (capture move 1). The reading is attributed to
    `community.principal` and signed by `community.did`/`.priv` (which may be a
    delegate key). `lineage_id` names the interpretive line; `interpreter_did`
    is the internal author (the plurality unit). Multiple records under distinct
    lineages and interpreters demonstrate the plurality the protocol requires;
    a monopoly is what the `interpretive_plurality` lens surfaces. Returns the
    CID."""
    if recorded_at is None:
        recorded_at = now_seconds()
    return archive.append_interpretation(
        community.priv, community.did, community.principal, lineage_id,
        interpreter_did, divergence, interprets_cid,
        list(references) if references else [], recorded_at,
    )


def report_event(
    archive,
    originator: Signer,
    summary: str,
    *,
    high_interpretive_charge: bool = False,
    sudden_onset: bool = False,
    originator_holds_stake: bool = False,
    aligned_faction: Optional[str] = None,
    declared_incentives: str = "",
    information_history: str = "",
    disclosed_interests: Optional[Sequence[str]] = None,
    onset_at: Optional[int] = None,
    reported_at: Optional[int] = None,
) -> str:
    """Inject an event report into the compact, provenance-tagged at intake
    (capture move 4). Attributed to `originator.principal`, signed by
    `originator.did`/`.priv`. The booleans are the structured-skepticism
    qualifying triggers; `aligned_faction` is the conveniently-aligned faction
    DID (or None). Returns the entry CID (pass it to `corroborate_event`)."""
    if reported_at is None:
        reported_at = now_seconds()
    return archive.append_event(
        originator.priv, originator.did, originator.principal, summary,
        high_interpretive_charge, sudden_onset, originator_holds_stake,
        aligned_faction, declared_incentives, information_history,
        list(disclosed_interests) if disclosed_interests else [],
        onset_at, reported_at,
    )


def corroborate_event(
    archive,
    community: Signer,
    event_cid: str,
    *,
    translation: Optional[str] = None,
    corroborated_at: Optional[int] = None,
) -> str:
    """Independently corroborate an event report's observable facts (capture
    move 4). Attributed to `community.principal`, signed by
    `community.did`/`.priv`. Distinct communities other than the originator are
    what the structured-skepticism lens counts. Returns the CID."""
    if corroborated_at is None:
        corroborated_at = now_seconds()
    return archive.append_corroboration(
        community.priv, community.did, community.principal, event_cid,
        translation, corroborated_at,
    )


def attest_principal(
    archive,
    witness: Signer,
    principal_did: str,
    principal_home: str,
    *,
    basis: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """Record `witness` cross-attesting a principal's validity (capture move 6).
    Attributed to `witness.principal`, signed by `witness.did`/`.priv`. The
    attestation counts toward independent validation only when the witness is
    neither the principal nor its `principal_home` community — that filter lives
    in the `adversarial_principal` lens, not here. Returns the CID."""
    if attested_at is None:
        attested_at = now_seconds()
    return archive.append_principal_witness(
        witness.priv, witness.did, witness.principal, principal_did,
        principal_home, basis, attested_at,
    )


def record_principal_output(
    archive,
    home: Signer,
    principal_did: str,
    cohort: str,
    matter: str,
    decision: str,
    *,
    favors_home: bool = False,
    produced_at: Optional[int] = None,
) -> str:
    """Record a machine principal's output for triangulation (capture move 6).
    The output is attributed to `home.principal` (the home community recording
    its principal's output) and signed by `home.did`/`.priv`. Outputs sharing a
    `cohort` and `matter` are compared; `favors_home` declares whether the
    decision benefits the principal's own side. Returns the CID."""
    if produced_at is None:
        produced_at = now_seconds()
    return archive.append_principal_output(
        home.priv, home.did, principal_did, home.principal, cohort, matter,
        decision, favors_home, produced_at,
    )


def record_conduct(
    archive,
    reporters: Union[Signer, Sequence[Signer]],
    subject_did: str,
    kind: str,
    bearing: str,
    *,
    feature: Optional[str] = None,
    bearing_detail: Optional[str] = None,
    severity: str = "Minor",
    ladder_rung: Optional[str] = None,
    decision_class: str = "FirstClass",
    references: Optional[Sequence[str]] = None,
    narrative: str = "",
    observed_at: Optional[int] = None,
    recorded_at: Optional[int] = None,
    perspective: Optional[str] = None,
) -> str:
    """Append a signed conduct record. `reporters` may be a single `Signer` or a
    quorum of them (for second-class kinds). Each report is attributed to the
    reporter's `principal` (the community) and signed by its `did`/`.priv` (which
    may be a delegate key). Returns the CID."""
    if isinstance(reporters, Signer):
        reporters = [reporters]
    if not reporters:
        raise ValueError("at least one reporter is required")
    if recorded_at is None:
        recorded_at = now_seconds()
    if observed_at is None:
        observed_at = recorded_at
    return archive.append_conduct(
        [r.priv for r in reporters],
        [r.did for r in reporters],
        [r.principal for r in reporters],
        subject_did, kind, bearing, recorded_at, observed_at,
        feature, bearing_detail, severity, ladder_rung, decision_class,
        list(references or []), narrative, perspective,
    )


def record_concern(
    archive,
    reporter: Signer,
    subject_did: str,
    feature: str,
    *,
    perspective: Optional[str] = None,
    narrative: str = "",
    recorded_at: Optional[int] = None,
    observed_at: Optional[int] = None,
) -> str:
    """Convenience for the jester's pre-indictment flag: a `Concern` bearing on
    an anti-spiral `feature`, first-class, no ladder rung. Returns the CID."""
    return record_conduct(
        archive, reporter, subject_did, "Concern", "AntiSpiral",
        feature=feature, perspective=perspective, narrative=narrative,
        recorded_at=recorded_at, observed_at=observed_at,
    )


def designate_anathema(
    archive,
    quorum: Union[Signer, Sequence[Signer]],
    subject_did: str,
    *,
    references: Optional[Sequence[str]] = None,
    narrative: str = "",
    recorded_at: Optional[int] = None,
) -> str:
    """Designate `subject_did` anathema: a `DesignationAnathema` conduct record,
    second-class (`quorum` co-signs), grave. Drives `RecognitionStanding.Anathema`
    (which overrides all other standings). Returns the CID."""
    return record_conduct(
        archive, quorum, subject_did, "DesignationAnathema", "Other",
        bearing_detail="designation", severity="Grave",
        decision_class="SecondClass", references=references, narrative=narrative,
        recorded_at=recorded_at,
    )


def lift_designation(
    archive,
    quorum: Union[Signer, Sequence[Signer]],
    subject_did: str,
    anathema_cid: str,
    *,
    narrative: str = "",
    recorded_at: Optional[int] = None,
) -> str:
    """Lift a prior anathema: a `DesignationLifted` conduct record referencing
    `anathema_cid`, superseding it so the subject is no longer Anathema. Returns
    the CID."""
    return record_conduct(
        archive, quorum, subject_did, "DesignationLifted", "Other",
        bearing_detail="designation-lifted", decision_class="SecondClass",
        references=[anathema_cid], narrative=narrative, recorded_at=recorded_at,
    )


# --- Records that were previously only buildable inside the Rust ceremonies ---

_DEFAULT_MOVEMENTS = (
    "substrate", "fallenness", "recognition", "common virtue", "non-ultimacy",
)


def record_non_admission(
    archive,
    signers: Union[Signer, Sequence[Signer]],
    participant_dids: Sequence[str],
    rejections: Sequence[tuple],
    *,
    compact_label: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Append a signed inter-community non-admission record. `signers` are the
    participant(s) co-signing it; `participant_dids` name all parties to the
    ceremony; `rejections` is a sequence of
    `(rejecting_did, presenting_did, feature, rationale)`. Feeds the
    rejection-pattern lens. Signer identities must already be published. Returns
    the CID."""
    if isinstance(signers, Signer):
        signers = [signers]
    if not signers:
        raise ValueError("at least one signer is required")
    return archive.append_non_admission(
        [s.priv for s in signers],
        [s.did for s in signers],
        list(participant_dids),
        [tuple(r) for r in rejections],
        compact_label,
        decided_at,
    )


def run_intra_rite(
    archive,
    community: Signer,
    *,
    movements: Optional[Sequence[str]] = None,
    witness_count: int = 1,
    sealed_at: Optional[int] = None,
) -> str:
    """Author and append a re-affirmation intra rite for `community` (its root
    `Signer`, whose identity is published). Provisions a reciter delegation and
    `witness_count` internal witnesses and runs the ceremony runner. Repeated
    calls at rising `sealed_at` with varying `witness_count` feed the
    ceremony-cost-decay lens. Returns the rite CID."""
    movs = list(movements) if movements is not None else list(_DEFAULT_MOVEMENTS)
    if len(movs) != 5:
        raise ValueError(f"movements must have 5 entries, got {len(movs)}")
    if sealed_at is None:
        sealed_at = now_seconds()
    return archive.append_intra_rite(
        community.priv, community.did, movs, witness_count, sealed_at,
    )


# --- §4 Mediation: mediator pool, dispute intake, paneled decisions ----------


def new_mediator(archive=None) -> Signer:
    """Mint a fresh keypair to serve as a mediator identity.

    A mediator's public key is carried in its community's `MediatorRegistry`
    entry (see `register_mediators`) and resolves from there, so — unlike a
    community — a mediator key does NOT need a separately published identity to
    have its panel co-signature verify. `archive` is accepted and ignored for
    call-site symmetry with `new_identity`. Returns a `Signer` (principal ==
    did); the key is a secret, treat it as sensitive."""
    _ = archive
    priv, pub, did = _core.generate_ed25519_keypair()
    return Signer(did=did, priv=priv, pub=pub)


def register_mediators(
    archive,
    community: Signer,
    mediators: Sequence[Signer],
    *,
    valid_from: Optional[int] = None,
    valid_until: Optional[int] = None,
) -> str:
    """Publish `community`'s roster of nominated mediators (§4), signed by the
    community root. Each mediator's `(pub, did)` becomes an active entry over
    `[valid_from, valid_until)`; `valid_from` defaults to now and `valid_until`
    open-ended. The commons-wide mediator pool is the union of every community's
    registry; a later registry from the same community supersedes its earlier
    one (latest `valid_from` wins). Returns the CID."""
    if not mediators:
        raise ValueError("at least one mediator is required")
    if valid_from is None:
        valid_from = now_seconds()
    return archive.register_mediators(
        community.priv, community.did,
        [(m.pub, m.did) for m in mediators],
        valid_from, valid_until,
    )


def file_dispute(
    archive,
    complainant: Signer,
    respondent_did: str,
    rung: str,
    *,
    summary: str = "",
    references: Optional[Sequence[str]] = None,
    filed_at: Optional[int] = None,
) -> str:
    """File a dispute against `respondent_did` (§4), signed by and attributed to
    `complainant`. `rung` is one of `LADDER_RUNGS` naming the escalation rung the
    complaint reaches for. `references` may cite prior record CIDs (e.g. conduct
    attestations). Returns the dispute's entry CID — pass it to
    `decide_mediation` to reference the dispute."""
    if rung not in LADDER_RUNGS:
        raise ValueError(f"rung must be one of {LADDER_RUNGS}, got {rung!r}")
    return archive.file_dispute(
        complainant.priv, complainant.did, respondent_did, rung,
        summary, list(references or []), filed_at,
    )


def decide_mediation(
    archive,
    dispute_cid: str,
    panel: Sequence[Signer],
    outcome: str,
    *,
    reasoning: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Record a mediation panel's decision on `dispute_cid` (§4), co-signed by
    every member of `panel`. `outcome` is one of `DECISION_OUTCOMES`. The
    archive's authority pass (`check_authority`, `panel_min` default 3) enforces
    the §4 structural rules: every panelist must be an active registered
    mediator, the panel must span at least `panel_min` of them, and none may
    come from either party's community (non-overlap). Returns the CID."""
    if not panel:
        raise ValueError("a panel needs at least one mediator")
    if outcome not in DECISION_OUTCOMES:
        raise ValueError(f"outcome must be one of {DECISION_OUTCOMES}, got {outcome!r}")
    return archive.decide_mediation(
        dispute_cid,
        [m.priv for m in panel],
        [m.did for m in panel],
        outcome, reasoning, decided_at,
    )


# --- §5 Escalation & designation --------------------------------------------
#
# These facades author root-signed escalation records (each signer speaks for
# itself). The archive's authority pass (`check_authority`) enforces the
# structural rules: a legal ladder move, the four-condition anathema test, a
# non-party restitution-verifier panel, and the re-entry sincerity bar.


def file_ladder_transition(
    archive,
    dispute_cid: str,
    from_rung: str,
    to_rung: str,
    initiators: Sequence[Signer],
    *,
    decision_class: str = "FirstClass",
    reasoning: str = "",
    transitioned_at: Optional[int] = None,
) -> str:
    """Advance (or de-escalate) a dispute along the ladder (§5), co-signed by
    `initiators`. `from_rung`/`to_rung` are `LADDER_RUNGS` names. A transition may
    escalate exactly one rung or de-escalate to any lower rung; the second-class
    rungs (CollectiveSanctions, Expulsion) require `decision_class="SecondClass"`
    and at least two distinct initiating communities. Returns the CID."""
    if not initiators:
        raise ValueError("a transition needs at least one initiator")
    for r in (from_rung, to_rung):
        if r not in LADDER_RUNGS:
            raise ValueError(f"rung must be one of {LADDER_RUNGS}, got {r!r}")
    if decision_class not in DECISION_CLASSES:
        raise ValueError(f"decision_class must be one of {DECISION_CLASSES}")
    return archive.file_ladder_transition(
        [s.priv for s in initiators], [s.did for s in initiators],
        dispute_cid, from_rung, to_rung, decision_class, reasoning, transitioned_at,
    )


def decide_designation(
    archive,
    subject_did: str,
    deliberators: Sequence[Signer],
    criteria: str,
    *,
    tier: str = "Anathema",
    urgency: str = "Normal",
    cited_acts: Optional[Sequence[str]] = None,
    failed_restoration_cycles: Optional[Sequence[str]] = None,
    deliberation_cycles: Optional[Sequence[tuple]] = None,
    reversal_conditions: Optional[str] = None,
    reexamine_after: Optional[int] = None,
    designated_at: Optional[int] = None,
) -> str:
    """Designate `subject_did` anathema (§5), co-signed by `deliberators`. `tier`
    is one of `DESIGNATION_TIERS`, `urgency` one of `DESIGNATION_URGENCIES`.
    `deliberation_cycles` is a sequence of `(cycle_index, [rep_did], opened_at,
    concluded_at, reasoning)`. The authority pass enforces the four conditions:
    (1) >= 3 completed-then-re-violated restitution cycles cited (relaxed for the
    Expedited path); (2) >= 2 deliberation cycles separated by the waiting period
    (Expedited: 1 cycle + higher quorum + a mandatory `reexamine_after`);
    (3) non-empty `criteria` + `cited_acts`; (4) reversal conditions present for
    Anathema, absent for HostisHumaniGeneris. Returns the CID."""
    if not deliberators:
        raise ValueError("a designation needs at least one deliberator")
    if tier not in DESIGNATION_TIERS:
        raise ValueError(f"tier must be one of {DESIGNATION_TIERS}, got {tier!r}")
    if urgency not in DESIGNATION_URGENCIES:
        raise ValueError(f"urgency must be one of {DESIGNATION_URGENCIES}, got {urgency!r}")
    return archive.decide_designation(
        [s.priv for s in deliberators], [s.did for s in deliberators],
        subject_did, tier, urgency, criteria,
        list(cited_acts or []), list(failed_restoration_cycles or []),
        [tuple(c) for c in (deliberation_cycles or [])],
        reversal_conditions, reexamine_after, designated_at,
    )


def impose_restitution(
    archive,
    subject_did: str,
    beneficiary_did: str,
    imposers: Sequence[Signer],
    terms: str,
    *,
    dispute_cid: Optional[str] = None,
    decision_class: str = "FirstClass",
    due_by: Optional[int] = None,
    imposed_at: Optional[int] = None,
) -> str:
    """Impose a restitution obligation on `subject_did` for `beneficiary_did`
    (§5), co-signed by `imposers`. Returns the obligation CID (referenced by a
    later `verify_restitution`)."""
    if not imposers:
        raise ValueError("an obligation needs at least one imposer")
    if decision_class not in DECISION_CLASSES:
        raise ValueError(f"decision_class must be one of {DECISION_CLASSES}")
    return archive.impose_restitution(
        [s.priv for s in imposers], [s.did for s in imposers],
        subject_did, beneficiary_did, terms, dispute_cid, decision_class, due_by, imposed_at,
    )


def verify_restitution(
    archive,
    obligation_cid: str,
    verifiers: Sequence[Signer],
    satisfied: bool,
    *,
    narrative: str = "",
    verified_at: Optional[int] = None,
) -> str:
    """Verify (or reject) a restitution obligation (§5), co-signed by `verifiers`.
    The authority pass enforces a non-party mediator panel (>= panel_min active
    registered mediators, none from the subject or beneficiary community).
    Returns the CID."""
    if not verifiers:
        raise ValueError("a verification needs at least one verifier")
    return archive.verify_restitution(
        [s.priv for s in verifiers], [s.did for s in verifiers],
        obligation_cid, satisfied, narrative, verified_at,
    )


def complete_reentry(
    archive,
    designation_cid: str,
    petitioner: Signer,
    deliberators: Sequence[Signer],
    movements: tuple,
    witnesses: Sequence[tuple],
    *,
    conduct_accounting: str,
    public_acknowledgement: str,
    probation_until: int,
    completed_at: Optional[int] = None,
) -> str:
    """Complete a re-entry from an anathema designation (§5), co-signed by the
    admitting `deliberators`. `movements` is a 5-tuple (substrate, fallenness,
    recognition, common_virtue, non_ultimacy); the petitioner's recitation and
    each witness attestation are built and signed internally. `witnesses` is a
    sequence of `(Signer, sincerity_affirmed: bool)`. The authority pass requires
    an Anathema (not Hostis) designation, a complete recitation, a non-empty
    accounting and acknowledgement, and >= 2 sincerity-affirmed witnesses.
    Returns the CID."""
    if not deliberators:
        raise ValueError("a re-entry needs at least one admitting community")
    if len(movements) != 5:
        raise ValueError("movements must be a 5-tuple of movement texts")
    return archive.complete_reentry(
        [s.priv for s in deliberators], [s.did for s in deliberators],
        designation_cid, petitioner.priv, petitioner.did, tuple(movements),
        [(w.priv, w.did, bool(affirmed)) for (w, affirmed) in witnesses],
        conduct_accounting, public_acknowledgement, probation_until, completed_at,
    )


def invoke_substantive_reengagement(
    archive,
    subject_did: str,
    invokers: Sequence[Signer],
    required_reaffirmation_by: int,
    *,
    decay_evidence: Optional[Sequence[str]] = None,
    decision_class: str = "FirstClass",
    invoked_at: Optional[int] = None,
) -> str:
    """Invoke the substantive-re-engagement rung against `subject_did` (§5) — a
    deliberate, signed act referencing decay evidence (NOT an automatic
    consequence of the ceremony-cost lens). Co-signed by `invokers`. Returns the
    CID."""
    if not invokers:
        raise ValueError("an invocation needs at least one invoker")
    return archive.invoke_substantive_reengagement(
        [s.priv for s in invokers], [s.did for s in invokers],
        subject_did, list(decay_evidence or []), required_reaffirmation_by,
        decision_class, invoked_at,
    )


def issue_injunction(
    archive,
    subject_did: str,
    panel: Sequence[Signer],
    harm: str,
    protective_measures: str,
    revoked_capabilities: Sequence[str],
    *,
    dispute_cid: Optional[str] = None,
    issued_at: Optional[int] = None,
    expires_at: Optional[int] = None,
) -> str:
    """Issue an interim injunction against `subject_did` (§9), co-signed by the
    emergency mediator `panel`. `revoked_capabilities` are `CAPABILITIES` names
    suspended while the injunction is in force. `expires_at` defaults to
    `issued_at + 90 days`. The authority pass enforces a non-party mediator panel
    (>= panel_min active registered mediators, none from the subject or, if
    `dispute_cid` is given, its parties). Returns the injunction CID (referenced
    by `confirm_injunction` / `challenge_injunction`)."""
    if not panel:
        raise ValueError("an injunction needs at least one panelist")
    for cap in revoked_capabilities:
        if cap not in CAPABILITIES:
            raise ValueError(f"capability must be one of {CAPABILITIES}, got {cap!r}")
    return archive.issue_injunction(
        [s.priv for s in panel], [s.did for s in panel],
        subject_did, harm, protective_measures, list(revoked_capabilities),
        dispute_cid, issued_at, expires_at,
    )


def confirm_injunction(
    archive,
    injunction_cid: str,
    confirmers: Sequence[Signer],
    *,
    confirmed_at: Optional[int] = None,
) -> str:
    """Confirm an injunction (§9) — the full compact ratifying it, co-signed by a
    distinct-community quorum. A confirmation arriving before the injunction's
    expiry lets it persist past its 90-day window. Returns the CID."""
    if not confirmers:
        raise ValueError("a confirmation needs at least one confirmer")
    return archive.confirm_injunction(
        [s.priv for s in confirmers], [s.did for s in confirmers],
        injunction_cid, confirmed_at,
    )


def challenge_injunction(
    archive,
    injunction_cid: str,
    panel: Sequence[Signer],
    overturned: bool,
    *,
    reasoning: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Challenge an injunction (§9), heard by a *different* mediator `panel` than
    the one that issued it. The authority pass enforces disjointness from the
    issuing panel plus the non-party rule. `overturned=True` ends the injunction.
    Returns the CID."""
    if not panel:
        raise ValueError("a challenge needs at least one panelist")
    return archive.challenge_injunction(
        [s.priv for s in panel], [s.did for s in panel],
        injunction_cid, bool(overturned), reasoning, decided_at,
    )


# --- §7 Commons governance --------------------------------------------------
#
# These facades author root-signed commons records. The archive's authority pass
# (`check_authority`) enforces the structural rules: a boundary-defined,
# second-class registration; usage self-reported by an appropriator; and a
# graduated sanction that follows the current level, does not skip, and carries a
# distinct-community quorum at its second-class levels (Restriction, Exclusion).


def register_commons(
    archive,
    kind: str,
    name: str,
    appropriators: Sequence[str],
    registrars: Sequence[Signer],
    *,
    description: str = "",
    provision_rule: str = "",
    registered_at: Optional[int] = None,
) -> str:
    """Register a compact-level commons (§7), co-signed by `registrars`. `kind` is
    one of `COMMONS_KINDS`; `appropriators` are the member DIDs with draw rights
    (Ostrom's boundary). Registration is a second-class act: the authority pass
    requires a distinct-community quorum, and every registrar must itself be an
    appropriator. Returns the commons CID (pass it to `record_usage`,
    `apply_sanction`, and the reputation gate)."""
    if kind not in COMMONS_KINDS:
        raise ValueError(f"kind must be one of {COMMONS_KINDS}, got {kind!r}")
    if not registrars:
        raise ValueError("registration needs at least one registrar")
    if not appropriators:
        raise ValueError("a commons needs at least one appropriator")
    return archive.register_commons(
        [s.priv for s in registrars], [s.did for s in registrars],
        kind, name, description, list(appropriators), provision_rule, registered_at,
    )


def record_usage(
    archive,
    commons_cid: str,
    community: Signer,
    consumed: int,
    contributed: int,
    *,
    period_start: Optional[int] = None,
    period_end: Optional[int] = None,
    recorded_at: Optional[int] = None,
) -> str:
    """Record `community`'s self-reported usage of a commons (§7), signed by and
    attributed to it. First-class; the authority pass requires the reporter be an
    appropriator of `commons_cid`. `consumed`/`contributed` are dimensionless
    units in the commons' own terms and feed `free_rider_signals`. Returns the
    CID."""
    return archive.record_usage(
        community.priv, community.did, commons_cid,
        int(consumed), int(contributed), period_start, period_end, recorded_at,
    )


def apply_sanction(
    archive,
    commons_cid: str,
    subject_did: str,
    from_level: str,
    to_level: str,
    imposers: Sequence[Signer],
    *,
    basis: str = "",
    references: Optional[Sequence[str]] = None,
    decision_class: str = "FirstClass",
    decided_at: Optional[int] = None,
) -> str:
    """Apply a graduated-sanction step against `subject_did` on a commons (§7),
    co-signed by `imposers`. `from_level`/`to_level` are `SANCTION_LEVELS` names.
    The authority pass requires the move to leave the pair's current level and
    either escalate one step or de-escalate to any lower level (restoration); the
    second-class levels (Restriction, Exclusion) require
    `decision_class="SecondClass"` and at least two distinct imposing communities.
    Returns the CID."""
    if not imposers:
        raise ValueError("a sanction needs at least one imposer")
    for lvl in (from_level, to_level):
        if lvl not in SANCTION_LEVELS:
            raise ValueError(f"level must be one of {SANCTION_LEVELS}, got {lvl!r}")
    if decision_class not in DECISION_CLASSES:
        raise ValueError(f"decision_class must be one of {DECISION_CLASSES}")
    return archive.apply_sanction(
        [s.priv for s in imposers], [s.did for s in imposers],
        commons_cid, subject_did, from_level, to_level,
        basis, list(references or []), decision_class, decided_at,
    )


# --- §8 Observer / jester corps ---------------------------------------------
#
# The cross-community journalism function. Observers are nominated by member
# communities and report publicly and attributably on *other* communities; the
# subject answers with an appended response, never by suppression. The
# whistleblower channel carries only a commitment to the source, revealable by a
# mediator panel for cause. The archive's authority pass enforces the structural
# rules (non-party observer, subject-only response, non-party reception panel).


def new_observer(archive=None) -> Signer:
    """Mint a fresh keypair to serve as an observer identity. Like a mediator, an
    observer's public key is carried in its community's `ObserverRegistry` entry
    (see `register_observers`) and resolves from there, so it needs no separately
    published identity for its report signature to verify. `archive` is accepted
    and ignored for call-site symmetry. Returns a `Signer`."""
    _ = archive
    priv, pub, did = _core.generate_ed25519_keypair()
    return Signer(did=did, priv=priv, pub=pub)


def register_observers(
    archive,
    community: Signer,
    observers: Sequence[tuple[Signer, str]],
    *,
    valid_from: Optional[int] = None,
    valid_until: Optional[int] = None,
) -> str:
    """Publish `community`'s roster of nominated observers (§8), signed by the
    community root. Each entry is `(observer_signer, declared_perspective)`; the
    declared perspective is kept explicit so readers weigh a report as advocacy.
    The compact-wide corps is the union of every community's registry (latest per
    community supersedes). Returns the CID."""
    if not observers:
        raise ValueError("at least one observer is required")
    if valid_from is None:
        valid_from = now_seconds()
    return archive.register_observers(
        community.priv, community.did,
        [(o.pub, o.did, perspective) for (o, perspective) in observers],
        valid_from, valid_until,
    )


def publish_observer_report(
    archive,
    observer: Signer,
    subject_did: str,
    declared_perspective: str,
    summary: str,
    *,
    references: Optional[Sequence[str]] = None,
    observed_at: Optional[int] = None,
    reported_at: Optional[int] = None,
) -> str:
    """Publish an attributed observer report on `subject_did` (§8), signed by
    `observer`. The authority pass requires the observer be a registered,
    non-party member of the corps (an observer may not report on its own
    community). Publication is irrevocable. Returns the report CID (pass it to
    `respond_to_report`)."""
    return archive.publish_observer_report(
        observer.priv, observer.did, subject_did, declared_perspective, summary,
        list(references or []), observed_at, reported_at,
    )


def respond_to_report(
    archive,
    community: Signer,
    report_cid: str,
    response: str,
    *,
    responded_at: Optional[int] = None,
) -> str:
    """Append `community`'s public response to an observer report (§8). Only the
    report's subject community may respond; the response never suppresses the
    report. Returns the CID."""
    return archive.respond_to_report(
        community.priv, community.did, report_cid, response, responded_at,
    )


def file_whistleblower(
    archive,
    accused_did: str,
    source_commitment: str,
    claim: str,
    reception_panel: Sequence[Signer],
    *,
    references: Optional[Sequence[str]] = None,
    filed_at: Optional[int] = None,
) -> str:
    """File an insider whistleblower report against `accused_did` (§8), co-signed
    by a mediator `reception_panel`. `source_commitment` is a CID committing to
    the source (e.g. `cid_of` of identity+nonce); the source identity is never
    passed in or stored. The authority pass requires a non-party reception panel.
    Returns the report CID."""
    if not reception_panel:
        raise ValueError("a reception panel needs at least one mediator")
    return archive.file_whistleblower(
        [s.priv for s in reception_panel], [s.did for s in reception_panel],
        accused_did, source_commitment, claim, list(references or []), filed_at,
    )


def reveal_source(
    archive,
    report_cid: str,
    panel: Sequence[Signer],
    cause: str,
    disclosed_source: str,
    *,
    revealed_at: Optional[int] = None,
) -> str:
    """Disclose a whistleblower's source (§8), co-signed by a mediator `panel` and
    citing a non-empty `cause`. The disclosure is gated on the panel quorum; use
    only for named cause. Returns the CID."""
    if not panel:
        raise ValueError("a reveal needs at least one panelist")
    if not cause.strip():
        raise ValueError("a source reveal must cite a cause")
    return archive.reveal_source(
        [s.priv for s in panel], [s.did for s in panel],
        report_cid, cause, disclosed_source, revealed_at,
    )


# --- §9 Substrate-deference (constitutional layer) --------------------------
#
# The compact-incapable list and the structured-fast cadence: the operational
# test of the compact's non-ultimacy. Genesis is a quorum-ratified singleton with
# genesis-immutable matters; amendments are second-class and asymmetric —
# expansion easy, contraction hard (Remove of a genesis matter refused entirely).

# §9 compact-incapable amendment directions.
AMENDMENT_KINDS = ("Add", "Remove")
# §9 structured-fast amendment directions.
FAST_CHANGES = ("Extend", "Contract")


def ratify_constitution(
    archive,
    ratifiers: Sequence[Signer],
    incapable: Sequence[tuple[str, str]],
    fast_periods: Sequence[tuple[str, int, int]],
    *,
    genesis_at: Optional[int] = None,
) -> str:
    """Ratify the founding constitution (§9), co-signed by `ratifiers`. `incapable`
    is a list of `(key, description)` compact-incapable matters (genesis-immutable);
    `fast_periods` is a list of `(label, start, end)` structured-fast windows.
    Second-class (needs a distinct-community quorum) and a singleton per archive.
    Returns the CID."""
    if not ratifiers:
        raise ValueError("ratification needs at least one ratifier")
    return archive.ratify_constitution(
        [s.priv for s in ratifiers], [s.did for s in ratifiers],
        [(k, d) for (k, d) in incapable],
        [(lbl, int(start), int(end)) for (lbl, start, end) in fast_periods],
        genesis_at,
    )


def amend_compact_incapable(
    archive,
    kind: str,
    matter_key: str,
    deciders: Sequence[Signer],
    *,
    matter_description: str = "",
    rationale: str = "",
    decision_class: str = "SecondClass",
    decided_at: Optional[int] = None,
) -> str:
    """Amend the compact-incapable list (§9). `kind` is one of `AMENDMENT_KINDS`.
    `Add` (expansion) uses the normal quorum; `Remove` (contraction) requires the
    higher bar and is refused for a genesis-original matter. Returns the CID."""
    if kind not in AMENDMENT_KINDS:
        raise ValueError(f"kind must be one of {AMENDMENT_KINDS}, got {kind!r}")
    if not deciders:
        raise ValueError("an amendment needs at least one decider")
    return archive.amend_compact_incapable(
        [s.priv for s in deciders], [s.did for s in deciders],
        kind, matter_key, matter_description, rationale, decision_class, decided_at,
    )


def amend_fast_cadence(
    archive,
    change: str,
    label: str,
    start: int,
    end: int,
    deciders: Sequence[Signer],
    *,
    rationale: str = "",
    decision_class: str = "SecondClass",
    decided_at: Optional[int] = None,
) -> str:
    """Amend the structured-fast cadence (§9). `change` is one of `FAST_CHANGES`;
    `Contract` carries the higher bar. `(label, start, end)` is the resulting fast
    period. Returns the CID."""
    if change not in FAST_CHANGES:
        raise ValueError(f"change must be one of {FAST_CHANGES}, got {change!r}")
    if not deciders:
        raise ValueError("an amendment needs at least one decider")
    return archive.amend_fast_cadence(
        [s.priv for s in deciders], [s.did for s in deciders],
        change, label, int(start), int(end), rationale, decision_class, decided_at,
    )


__all__ = [
    "Signer", "new_identity", "delegate_speaker", "revoke_delegation", "vouch",
    "attest_interpretation", "report_event", "corroborate_event",
    "attest_principal", "record_principal_output",
    "record_conduct", "record_concern", "designate_anathema", "lift_designation",
    "record_non_admission", "run_intra_rite", "new_mediator",
    "register_mediators", "file_dispute", "decide_mediation",
    "file_ladder_transition", "decide_designation", "impose_restitution",
    "verify_restitution", "complete_reentry", "invoke_substantive_reengagement",
    "issue_injunction", "confirm_injunction", "challenge_injunction",
    "register_commons", "record_usage", "apply_sanction",
    "new_observer", "register_observers", "publish_observer_report",
    "respond_to_report", "file_whistleblower", "reveal_source",
    "ratify_constitution", "amend_compact_incapable", "amend_fast_cadence",
    "AMENDMENT_KINDS", "FAST_CHANGES",
    "now_seconds",
    "CONDUCT_KINDS", "ANTI_SPIRAL_FEATURES", "BEARINGS", "SEVERITIES",
    "LADDER_RUNGS", "DECISION_CLASSES", "DELEGATION_SCOPES", "DECISION_OUTCOMES",
    "DESIGNATION_TIERS", "DESIGNATION_URGENCIES", "CAPABILITIES",
    "COMMONS_KINDS", "SANCTION_LEVELS",
]
