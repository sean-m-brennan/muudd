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


__all__ = [
    "Signer", "new_identity", "delegate_speaker", "revoke_delegation", "vouch",
    "record_conduct", "record_concern", "designate_anathema", "lift_designation",
    "record_non_admission", "run_intra_rite", "new_mediator",
    "register_mediators", "file_dispute", "decide_mediation",
    "file_ladder_transition", "decide_designation", "impose_restitution",
    "verify_restitution", "complete_reentry", "invoke_substantive_reengagement",
    "now_seconds",
    "CONDUCT_KINDS", "ANTI_SPIRAL_FEATURES", "BEARINGS", "SEVERITIES",
    "LADDER_RUNGS", "DECISION_CLASSES", "DELEGATION_SCOPES", "DECISION_OUTCOMES",
    "DESIGNATION_TIERS", "DESIGNATION_URGENCIES",
]
