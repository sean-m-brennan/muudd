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
# §5 what a designation's mandated re-examination concluded.
REEXAMINATION_OUTCOMES = ("Confirmed", "Reversed", "Vacated")
# The standing functions whose exercise is attested on a cadence (§4 and §8).
ATTESTED_FUNCTIONS = ("Mediation", "Observation")
# §9 capabilities an interim injunction can suspend, in two kinds. A SERVICE is a
# relation of exchange; an OFFICE is a role that selects who is sanctioned. They are
# revoked independently, so a member can be held out of judgment while every relation
# of exchange it holds stays intact.
SERVICE_CAPABILITIES = (
    "RescueCoordination", "Navigation", "Trade", "CommonsWithdrawal", "Communication",
)
OFFICE_CAPABILITIES = (
    "MediatorSeat", "ObserverSeat", "ExaminerSeat", "Deliberation", "EmergencyPanelSeat",
)
CAPABILITIES = SERVICE_CAPABILITIES + OFFICE_CAPABILITIES
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

    `enc_priv`/`enc_pub` are a **second** keypair, present only where the role needs
    one — today, a mediator, whose §8 custody share of a whistleblower's identity is
    sealed to `enc_pub` by key agreement. It is deliberately not the signing key: one
    key under two primitives is what the encryption key exists to end, and the
    authority pass refuses a registration where the two are equal.

    `priv` is a secret; treat this object as sensitive. `Signer` deliberately
    does not print its key material.
    """

    did: str
    priv: bytes
    pub: bytes
    principal: str = field(default=None)  # type: ignore[assignment]
    enc_priv: Optional[bytes] = None
    enc_pub: Optional[bytes] = None

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


def attest_interpreter_independence(
    archive,
    attester: Signer,
    community_did: str,
    interpreter_did: str,
    *,
    basis: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """Vouch that `interpreter_did`, a declared interpreter of `community_did`'s
    substrate, is an independent principal (capture move 1, anti-sockpuppet).
    Attributed to `attester.principal`, signed by `attester.did`/`.priv`. The
    `interpretive_plurality` lens counts the vouching toward that interpreter's
    independence only when the attester is neither the community nor the
    interpreter — a community cannot self-certify identities it minted. Returns
    the CID."""
    if attested_at is None:
        attested_at = now_seconds()
    return archive.attest_interpreter_independence(
        attester.priv, attester.did, attester.principal, community_did,
        interpreter_did, basis, attested_at,
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


def invoke_event_action(
    archive,
    acting: Signer,
    event_cid: str,
    action: str,
    *,
    invoked_at: Optional[int] = None,
) -> str:
    """Record a formal action premised on an event report (capture move 4).
    Attributed to `acting.principal`, signed by `acting.did`/`.priv`. This is
    where structured skepticism has teeth: the archive refuses the action while
    the cited *qualifying* event is still within its structured-skepticism hold
    and under-corroborated — call it only after the hold elapses or enough
    distinct communities have run `corroborate_event`. Returns the CID."""
    if invoked_at is None:
        invoked_at = now_seconds()
    return archive.invoke_event_action(
        acting.priv, acting.did, acting.principal, event_cid, action, invoked_at,
    )


def find_undisclosed_interest(
    archive,
    finder: Signer,
    event_cid: str,
    undisclosed_interest: str,
    *,
    found_at: Optional[int] = None,
) -> str:
    """Retroactively find that `event_cid`'s originator held an interest it did
    not disclose at intake (capture move 4). Attributed to `finder.principal`,
    signed by `finder.did`/`.priv`. The `structured_skepticism` lens then marks
    the event impugned — unless the finder is the event's own originator, which is
    honest late disclosure, not bad faith. The consequence of bad faith is the
    conduct machinery's; this only surfaces the finding. Returns the CID."""
    if found_at is None:
        found_at = now_seconds()
    return archive.find_undisclosed_interest(
        finder.priv, finder.did, finder.principal, event_cid,
        undisclosed_interest, found_at,
    )


def attest_principal(
    archive,
    witness: Signer,
    principal_did: str,
    principal_home: str,
    *,
    basis: str = "",
    prev: Optional[str] = None,
    attested_at: Optional[int] = None,
) -> str:
    """Record `witness` cross-attesting a principal's validity (capture move 6).
    Attributed to `witness.principal`, signed by `witness.did`/`.priv`. The
    attestation counts toward independent validation only when the witness is
    neither the principal nor its `principal_home` community — that filter lives
    in the `adversarial_principal` lens, not here.

    `prev` optionally names the previous attestation's CID in this principal's
    chain, making continuous attestation a tamper-evident hash-linked chain (the
    `principal_witness_chain` lens); omit it for the chain root. Returns the CID."""
    if attested_at is None:
        attested_at = now_seconds()
    return archive.append_principal_witness(
        witness.priv, witness.did, witness.principal, principal_did,
        principal_home, basis, prev, attested_at,
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


# --- §2 Admissions integrity: admission, sponsorship, the examining pool -------


def record_admission(
    archive,
    signers: Sequence[Signer],
    participants: Sequence[str],
    *,
    compact_label: str = "compact",
    witnessed_at: Optional[int] = None,
) -> str:
    """Append a signed inter-community admission record (§2).

    `participants` are the communities the record speaks for — the sitting members
    and the entrant together, as the ceremony seals them; `signers` are the
    cross-boundary signers.

    The first record for a `compact_label` is that compact's **formation** and is
    exempt from the §2 admission rules: its participants are founders, not
    entrants. Every later record is an admission into a standing compact and
    `check_authority` requires it to carry an admitting quorum of sitting,
    non-probationary members plus an examining panel of seated examiners spanning
    at least two communities. Returns the CID."""
    if not signers:
        raise ValueError("at least one signer is required")
    if not participants:
        raise ValueError("at least one participant is required")
    return archive.append_admission(
        [s.priv for s in signers], [s.did for s in signers],
        list(participants), compact_label, witnessed_at,
    )





def new_examiner(archive=None) -> Signer:
    """Mint a fresh keypair to serve as an examiner identity.

    An examiner's public key is carried in its community's `ExaminerRegistry`
    entry (see `register_examiners`) and resolves from there. `archive` is
    accepted and ignored for call-site symmetry with `new_identity`. Returns a
    `Signer`; the key is a secret, treat it as sensitive.

    In a real ceremony the examiner is the community's cross-boundary signer for
    that credentialing — the witness record carries exactly one signer per
    participant, and principal binding requires it to be the community's own root
    or `Envoy`. Seating that signer is what §2's caps constrain."""
    _ = archive
    priv, pub, did = _core.generate_ed25519_keypair()
    return Signer(did=did, priv=priv, pub=pub)


def register_examiners(
    archive,
    community: Signer,
    examiners: Sequence[Signer],
    *,
    valid_from: Optional[int] = None,
    valid_until: Optional[int] = None,
) -> str:
    """Seat `community`'s examiners (§2), signed by the community root.

    Each examiner's `(pub, did)` holds a seat over `[valid_from, valid_until)`;
    `valid_from` defaults to now. The compact-wide examining pool is the union of
    every community's registry, and a later registry from the same community
    supersedes its earlier one.

    `valid_until=None` authors fine and is refused by `check_authority`: §2 allows
    no open-ended examining standing, because a permanent examiner is the standing
    admissions cadre the rotation rule exists to prevent. A probationary community
    may not seat examiners at all. Returns the CID."""
    if not examiners:
        raise ValueError("at least one examiner is required")
    if valid_from is None:
        valid_from = now_seconds()
    return archive.register_examiners(
        community.priv, community.did,
        [(e.pub, e.did) for e in examiners],
        valid_from, valid_until,
    )


def new_member(archive=None) -> Signer:
    """Mint a fresh keypair to serve as an individual member identity.

    A member's public key is carried in its community's `MemberRoll` (see
    `publish_member_roll`) and must be the key its DID resolves to. `archive` is
    accepted and ignored for call-site symmetry with `new_identity`. Returns a
    `Signer`; the key is a secret, treat it as sensitive.

    A member is not a speaker: being on the roll authorizes no record at all. What
    it grants is a place in the §8 insider ring, so a member who has no delegation
    can still file a whistleblower report anonymously."""
    _ = archive
    priv, pub, did = _core.generate_ed25519_keypair()
    return Signer(did=did, priv=priv, pub=pub)


def publish_member_roll(
    archive,
    community: Signer,
    members: Sequence[Signer],
    *,
    as_of: Optional[int] = None,
    published_at: Optional[int] = None,
) -> str:
    """Publish `community`'s member roll (§8), signed by the community root.

    The roll is what makes the whistleblower channel real. Without one the insider
    ring is the community's root key plus its live speaker delegations, so a member
    holding no delegation is not in it, and a community with a single key has no
    anonymous channel at all.

    **A roll only ever widens the ring.** Rolls are unioned rather than superseded,
    so publishing a shorter roll later removes nobody — once named an insider, always
    an insider. That is deliberate: the alternative hands a community the power to
    shrink the suspect set around whoever just left. A departed member keeps the
    channel they had, which is the direction to err in.

    Naming who belongs is not delegable: an `Envoy` cannot publish a roll, only the
    root. Each member's key must be the one its DID resolves to, so a roll cannot
    name a real member against an outsider's key.

    `as_of` records the instant the roll was read from whatever models membership
    upstream — for a polity, its charter — and is provenance only; this tier cannot
    check it. Defaults to now. Returns the CID."""
    if not members:
        raise ValueError("at least one member is required")
    if as_of is None:
        as_of = now_seconds()
    return archive.publish_member_roll(
        community.priv, community.did,
        [(m.pub, m.did) for m in members],
        as_of, published_at,
    )


def insider_set_size(archive, community_did: str, at: Optional[int] = None) -> int:
    """How many keys `community_did`'s anonymous channel can hide a source among.

    1 means there is no channel: a ring of one names its signer, and the authority
    pass refuses to file at all. 2 is a channel in name only — the key image is
    published on every filing, so a suspect set of two is nearly a name. Publish a
    member roll to widen it.

    This is `len(insider_ring(...))`, offered separately because the count is the
    part a community should be watching and the keys are not."""
    return len(archive.insider_ring(community_did, at))


def probationary_at(archive, community_did: str, at: Optional[int] = None) -> bool:
    """Is `community_did` inside a probation window at `at` (default now)?

    The §2 predicate the authority rules enforce: a received sponsorship whose
    stake still rides on this community's conduct, or a completed re-entry before
    its window closes. Exposed so a caller can see why an act would be refused
    before authoring it — a probationary community may not sponsor an entrant, may
    not seat mediators or examiners, and carries no weight toward an admitting or
    designation quorum.

    This is the same function `reputation_of(...).standing == "Probationary"` is
    derived from, so the two cannot disagree."""
    if at is None:
        at = now_seconds()
    return archive.probationary_at(community_did, at)


# --- §4 Mediation: mediator pool, dispute intake, paneled decisions ----------


def new_mediator(archive=None) -> Signer:
    """Mint a fresh keypair to serve as a mediator identity.

    A mediator's public key is carried in its community's `MediatorRegistry`
    entry (see `register_mediators`) and resolves from there, so — unlike a
    community — a mediator key does NOT need a separately published identity to
    have its panel co-signature verify. `archive` is accepted and ignored for
    call-site symmetry with `new_identity`. Returns a `Signer` (principal ==
    did); the key is a secret, treat it as sensitive.

    A mediator gets **two** keypairs. The second (`enc_pub`/`enc_priv`) is registered
    as its encryption key and never signs anything: a mediator seated on a §8
    whistleblower reception panel holds a share of the source's identity, and that
    share is sealed to the encryption key by key agreement. Sealing to the signing key
    would put one key under two primitives, and the authority pass refuses a
    registration where the two are equal."""
    _ = archive
    priv, pub, did = _core.generate_ed25519_keypair()
    enc_priv, enc_pub, _enc_did = _core.generate_ed25519_keypair()
    return Signer(did=did, priv=priv, pub=pub, enc_priv=enc_priv, enc_pub=enc_pub)


def register_mediators(
    archive,
    community: Signer,
    mediators: Sequence[Signer],
    *,
    valid_from: Optional[int] = None,
    valid_until: Optional[int] = None,
    trained_by: Optional[Sequence[Optional[str]]] = None,
) -> str:
    """Publish `community`'s roster of nominated mediators (§4), signed by the
    community root. Each mediator's `(pub, did)` becomes an active entry over
    `[valid_from, valid_until)`; `valid_from` defaults to now. The commons-wide
    mediator pool is the union of every community's registry; a later registry from
    the same community supersedes its earlier one (latest `valid_from` wins).

    `valid_until` is **required** by the §4 caps. A seat with no end is a permanent
    mediator class, and the doctrine is direct about it: mediators serve "with term
    limits that ensure return to the home community... a permanent mediator class is
    the first step toward an inquisitorial caste". The authority pass also bounds the
    term's length, requires a sit-out before re-nomination, and refuses a roster that
    would give one community a majority of the active pool — the same four rules §8's
    observers and §2's examiners carry, which the mediator pool did not until this.

    `trained_by` optionally records each mediator's apprenticeship, by DID and in the
    same order as `mediators`. It **gates nothing**: the source documents warn against
    "a certification barrier that becomes a gatekeeping mechanism", and the whitepaper
    lists the training pipeline as an open question. It is read only by
    `mediator_pool_concentration`, which reports when one trainer is behind a majority
    of the pool — the apprenticeship model's own named weakness, made visible without
    being made a gate.

    Each entry also carries the mediator's **encryption key**, distinct from its
    signing key, which is what §8 custody shares are sealed to. `new_mediator` mints
    both; a `Signer` without one cannot be registered as a mediator. Returns the CID."""
    if not mediators:
        raise ValueError("at least one mediator is required")
    missing = [m.did for m in mediators if not m.enc_pub]
    if missing:
        raise ValueError(
            "a mediator needs an encryption key distinct from its signing key "
            f"(missing for {', '.join(missing)}); use new_mediator()"
        )
    if valid_from is None:
        valid_from = now_seconds()
    return archive.register_mediators(
        community.priv, community.did,
        [(m.pub, m.did, m.enc_pub) for m in mediators],
        valid_from, valid_until, list(trained_by) if trained_by is not None else None,
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


ARGUMENT_ROLES = ("Party", "Panelist")


def file_argument(
    archive,
    author: Signer,
    dispute_cid: str,
    role: str,
    position: str,
    *,
    evidence: Optional[Sequence[str]] = None,
    submitted_at: Optional[int] = None,
) -> str:
    """Submit one argument on `dispute_cid` (§4 "structured argument submission"),
    signed by `author`. `role` is one of `ARGUMENT_ROLES`.

    This is the record that makes a decision auditable for **how** it was reached and
    not only what it concluded, which is what the §8 observer corps and the §9 erosion
    signals both assume of the archive.

    `"Party"` is a party's own case, and only the complainant or respondent may make
    one; its envoy may sign for it, exactly as for the dispute filing. `"Panelist"` is
    a seated mediator's position, must be signed by that mediator alone, and the author
    must be an active registered mediator from neither party community.

    `position` must be non-empty and is never read for content: no rule can weigh an
    argument, only record that it was put. `evidence` cites the record CIDs it rests on
    (§4's evidence packaging). Returns the CID — pass it to `decide_mediation`."""
    if role not in ARGUMENT_ROLES:
        raise ValueError(f"role must be one of {ARGUMENT_ROLES}, got {role!r}")
    return archive.file_argument(
        author.priv, author.did, dispute_cid, role, position,
        list(evidence or []), submitted_at,
    )


RECUSAL_ANSWERS = ("Granted", "Declined")

CONSENT_ANSWERS = ("Given", "Withheld")

REVIEW_ANSWERS = ("Upheld", "Pretextual")

APPROPRIATOR_LEVELS = ("Community", "Compact")

SKEPTICISM_CRITERIA = (
    "HighInterpretiveCharge",
    "SuddenOnset",
    "OriginatorHoldsStake",
    "AlignedFaction",
)


def request_recusal(
    archive,
    party: Signer,
    dispute_cid: str,
    mediator_did: str,
    grounds: str,
    *,
    requested_at: Optional[int] = None,
) -> str:
    """Ask that `mediator_did` step down from `dispute_cid` (§4), signed by `party`.

    Panel *composition* was already enforced — registered, non-party, minimum count —
    but a party had no way to say "this one, on this dispute" and have the request
    recorded and answered. Nothing blocks on it: an unanswered request is visible
    rather than paralysing, since a request that halted a mediation by existing would
    be the delay tactic §4's own review guards against.

    `grounds` must be non-empty and is never read for content. Returns the CID — pass
    it to `decide_recusal`."""
    return archive.request_recusal(
        party.priv, party.did, dispute_cid, mediator_did, grounds, requested_at,
    )


def decide_recusal(
    archive,
    request_cid: str,
    answer: str,
    deciders: Sequence[Signer],
    *,
    reasoning: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Answer a recusal request (§4). `answer` is one of `RECUSAL_ANSWERS`.

    Who may sign depends on the answer, and that asymmetry is the rule. A `"Granted"`
    answer is signed by exactly the mediator stepping down: stepping down is always
    available to them and nobody else may condition it. A `"Declined"` answer must come
    from registered non-party mediators *other than* the one named, and must state its
    `reasoning` — a panelist who could dismiss requests against themselves is precisely
    the conflict the request exists to raise, and a refusal to step aside is the answer
    that owes an account. Returns the CID."""
    if answer not in RECUSAL_ANSWERS:
        raise ValueError(f"answer must be one of {RECUSAL_ANSWERS}, got {answer!r}")
    if not deciders:
        raise ValueError("a recusal decision needs at least one decider")
    return archive.decide_recusal(
        [d.priv for d in deciders], [d.did for d in deciders],
        request_cid, answer, reasoning, decided_at,
    )


def designate_protecting_power(
    archive,
    disputant: Signer,
    protector: Signer,
    dispute_cid: str,
    *,
    statement: str = "",
    designated_at: Optional[int] = None,
) -> str:
    """Designate `protector` as protecting power for `disputant` on `dispute_cid` (§4),
    co-signed by both.

    The Geneva analogue: a neutral community that looks after a disputant's interests
    during a dispute — that the ladder is climbed correctly, that the disputant has
    access to the mediators, that communications are not interdicted, that the dispute
    is not a pretext for violating the common virtues. **"It is not an arbitrator; it is
    a witness with standing"**, so this gates nothing: no decision waits on it and no
    panel is invalid without one.

    What the pass enforces is neutrality — the protector may be neither party and may
    not itself seat mediators, or the witness and the judge are one body — and that both
    sides consented, since standing for another community is a duty and is never
    conscripted, and a protector the disputant did not accept is a watcher imposed on
    them. Returns the CID."""
    return archive.designate_protecting_power(
        disputant.priv, disputant.did, protector.priv, protector.did,
        dispute_cid, statement, designated_at,
    )


def record_panel_consent(
    archive,
    party: Signer,
    dispute_cid: str,
    panel_dids: Sequence[str],
    answer: str,
    *,
    statement: str = "",
    given_at: Optional[int] = None,
) -> str:
    """Record `party`'s consent (or withholding) to the panel as constituted (§4).
    `answer` is one of `CONSENT_ANSWERS`.

    Objection already had a record — a recusal request names one panelist and is
    answered — and structural consent already had one, since a protecting power is
    co-signed by both sides. What had no record was the ordinary case: a party saying
    that this panel, these mediators, may decide its dispute. It was inferred from the
    party having filed an argument, which is a different act.

    Self-signed, because nobody consents on another's behalf. The panel is named in
    full and consent does not carry across a change to it: a party that accepted three
    mediators has not accepted a fourth.

    **Consent gates exactly one thing.** Requiring it to decide at all would hand either
    party a veto over its own dispute, and the escalation ladder exists because parties
    disagree. But a `"Settled"` outcome is a claim that *the parties* reached a
    settlement, so it needs `"Given"` from both, for exactly the deciding panel. A party
    that refuses to settle blocks nothing — the panel upholds, dismisses or refers.
    Returns the CID."""
    if answer not in CONSENT_ANSWERS:
        raise ValueError(f"answer must be one of {CONSENT_ANSWERS}, got {answer!r}")
    if not panel_dids:
        raise ValueError("panel consent must name the panel it consents to")
    return archive.record_panel_consent(
        party.priv, party.did, dispute_cid, list(panel_dids), answer, statement, given_at,
    )


def invoke_skepticism(
    archive,
    panel: Sequence[Signer],
    event_cid: str,
    criteria: Sequence[str],
    grounds: str,
    hold_seconds: int,
    *,
    invoked_at: Optional[int] = None,
) -> str:
    """The mediator pool invokes the structured-skepticism protocol on `event_cid` (§4),
    co-signed by every member of `panel`. `criteria` are drawn from
    `SKEPTICISM_CRITERIA`.

    This is the half of move 4 the pool was always meant to hold. Whether an event
    qualifies used to be four booleans the *originating community* set on its own
    report, so a manufacturer that declared its own event mundane was never held and
    nothing in the compact could hold it. The invocation reaches exactly that event.

    The pass refuses a panel containing the originating community's own mediators —
    placing the pool's hold on your own event is what the self-declared triggers already
    are — requires the criteria and grounds to be documented, and bounds `hold_seconds`
    to the range the protocol calibrates to: a nominal hold is no delay, an unbounded one
    is suppression. Returns the CID."""
    if not panel:
        raise ValueError("an invoking panel needs at least one mediator")
    for c in criteria:
        if c not in SKEPTICISM_CRITERIA:
            raise ValueError(f"criteria must be drawn from {SKEPTICISM_CRITERIA}, got {c!r}")
    return archive.invoke_skepticism(
        [m.priv for m in panel], [m.did for m in panel],
        event_cid, list(criteria), grounds, hold_seconds, invoked_at,
    )


def review_skepticism(
    archive,
    panel: Sequence[Signer],
    event_cid: str,
    answer: str,
    reasoning: str,
    *,
    reviewed_at: Optional[int] = None,
) -> str:
    """The pool's internal review of the hold on `event_cid`, against pretextual use
    (§4). `answer` is one of `REVIEW_ANSWERS`; `"Pretextual"` releases the event.

    The review is against the **hold**, not against an invocation record, which is what
    lets it reach a self-declared hold — the case that used to answer to nobody, where a
    community could put its own urgent matter beyond action and no reviewer was on the
    record anywhere.

    Mediators who co-signed an invocation on this event may not review it: a panel that
    reviews its own invocation answers the question the review was raised to ask.
    Reasoning is required on either answer, because an audit legible only when the hold
    falls tells a reader nothing about the holds that stood. Returns the CID."""
    if not panel:
        raise ValueError("a reviewing panel needs at least one mediator")
    if answer not in REVIEW_ANSWERS:
        raise ValueError(f"answer must be one of {REVIEW_ANSWERS}, got {answer!r}")
    return archive.review_skepticism(
        [m.priv for m in panel], [m.did for m in panel],
        event_cid, answer, reasoning, reviewed_at,
    )


def open_decision_share(archive, decision_cid: str, custodian: Signer) -> str:
    """Open `custodian`'s own share of a sealed decision's proceedings (§4), returning
    it as JSON to be gathered into an `unseal_decision` call.

    A share is useless alone: it takes the seal's threshold of them to read anything."""
    return archive.open_decision_share(decision_cid, custodian.priv, custodian.did)


def unseal_decision(
    archive,
    decision_cid: str,
    custodians: Sequence[Signer],
    disclosed_reasoning: str,
    shares: Sequence[str],
    *,
    cause: str = "",
    unsealed_at: Optional[int] = None,
) -> str:
    """Open a sealed decision's proceedings (§4), co-signed by the `custodians` whose
    `shares` are presented.

    The pass requires the shares to actually recover the sealed text and that text to
    open the decision's commitment, so an unsealing cannot publish a reasoning the panel
    never wrote — the same rule §8's source reveal carries, for the same reason.

    Opening before the seal's own `reveal_at` must cite a `cause`; after the generational
    cycle has run it is bookkeeping. Returns the CID."""
    if not custodians:
        raise ValueError("an unsealing needs at least one custodian")
    return archive.unseal_decision(
        [c.priv for c in custodians], [c.did for c in custodians],
        decision_cid, disclosed_reasoning, list(shares), cause, unsealed_at,
    )


def decide_mediation(
    archive,
    dispute_cid: str,
    panel: Sequence[Signer],
    outcome: str,
    reasoning: str,
    arguments: Sequence[str],
    *,
    decided_at: Optional[int] = None,
    seal_reasoning: Optional[str] = None,
    seal_custodian_dids: Optional[Sequence[str]] = None,
    seal_threshold: Optional[int] = None,
    seal_reveal_at: Optional[int] = None,
) -> str:
    """Record a mediation panel's decision on `dispute_cid` (§4), co-signed by
    every member of `panel`. `outcome` is one of `DECISION_OUTCOMES`.

    The archive's authority pass (`check_authority`, `panel_min` default 3) enforces
    the §4 structural rules: every panelist must be an active registered mediator, the
    panel must span at least `panel_min` of them, and none may come from either party's
    community (non-overlap).

    `reasoning` and `arguments` are both **required**, and are what the pass added once
    §4's own "mandatory reasoning trail" was measured and found to be enforced on
    nothing. `reasoning` must be non-empty. `arguments` must cite one `"Panelist"`
    argument per co-signer — three signatures on one author's text is a rubber stamp,
    not a panel — and must answer every `"Party"` argument already in the log, so a
    panel cannot decide while ignoring a case put to it.

    A `"Settled"` outcome additionally needs both parties' recorded consent to this exact
    panel (see `record_panel_consent`): a settlement the parties did not consent to is
    not a settlement.

    **To seal the proceedings** instead of publishing them (§4's "sealed decisions with
    threshold-reveal"), leave `reasoning` empty and pass the text as `seal_reasoning`
    with `seal_custodian_dids`. What is sealed is the reasoning: dispute, panel and
    outcome stay public, so every rule above keeps running over a sealed decision.

    Custodians are **member communities**, not mediators. §4 caps a mediator's seat at
    two years, because a permanent mediator class is the first step toward an
    inquisitorial caste, while the seal opens at twenty-five: a share held by a key the
    archive stops resolving could never be opened, and the signature on the opening could
    not be verified. Communities resolve at any instant. The pass refuses a party as
    custodian, and refuses a custodian set of which the deciding panel's own communities
    hold a quorum — a panel that can unseal itself has deferred publication at its own
    discretion rather than sealed anything. `seal_reveal_at` defaults to one generational
    cycle from the decision. Returns the CID."""
    if not panel:
        raise ValueError("a panel needs at least one mediator")
    if outcome not in DECISION_OUTCOMES:
        raise ValueError(f"outcome must be one of {DECISION_OUTCOMES}, got {outcome!r}")
    if seal_reasoning is not None and not seal_custodian_dids:
        raise ValueError("sealing a decision needs `seal_custodian_dids` to hold the shares")
    return archive.decide_mediation(
        dispute_cid,
        [m.priv for m in panel],
        [m.did for m in panel],
        outcome, reasoning, list(arguments), decided_at,
        seal_reasoning,
        list(seal_custodian_dids) if seal_custodian_dids else None,
        seal_threshold, seal_reveal_at,
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


def conclude_reexamination(
    archive,
    designation_cid: str,
    examiners: Sequence[Signer],
    outcome: str,
    reasoning: str,
    *,
    next_reexamine_after: Optional[int] = None,
    concluded_at: Optional[int] = None,
) -> str:
    """Record what a designation's mandated re-examination concluded (§5), co-signed by
    `examiners`. `outcome` is one of `REEXAMINATION_OUTCOMES`.

    The attested null of the severest act the compact can take. Before this record a
    re-examination that *confirmed* a designation left no trace, so a panel that looked
    at the mandated date and let the designation stand read exactly like a compact that
    had forgotten, forever: the erosion lens punished the diligent panel. Now a
    confirmation answers its deadline and sets the next one.

    Two rules keep it from evading what it records. It may not be dated before the
    deadline it answers, or a compact would confirm at designation time and buy
    permanent silence. And a `Confirmed` outcome must set `next_reexamine_after`, later
    than `concluded_at`, because a confirmation renews the obligation rather than
    discharging it. A `Reversed` outcome requires the reversal itself to be recorded by
    the path that already exists (a lifting record or a re-entry), so the archive keeps
    one definition of a reversed designation. Returns the CID."""
    if not examiners:
        raise ValueError("a re-examination needs at least one examining community")
    if outcome not in REEXAMINATION_OUTCOMES:
        raise ValueError(
            f"outcome must be one of {REEXAMINATION_OUTCOMES}, got {outcome!r}"
        )
    return archive.append_reexamination(
        [s.priv for s in examiners], [s.did for s in examiners],
        [s.did for s in examiners], designation_cid, outcome, reasoning,
        next_reexamine_after, concluded_at if concluded_at is not None else now_seconds(),
    )


GRIEVANCE_GROUNDS = ("NeverPursued", "ClosedUnresolved")


def attest_non_pursuit(
    archive,
    attestor: Signer,
    subject_did: str,
    evidence: Sequence[str],
    reasoning: str,
    *,
    ground: str = "NeverPursued",
    closing_decision: Optional[str] = None,
    filed_at: Optional[int] = None,
) -> str:
    """Record `attestor`'s attestation that adverse evidence about `subject_did` has
    gone unanswered (§5).

    The counterpart of `file_designation_dissent`, on the other side of the same act. A
    dissent records a refusal to *join* a designation; this records the compact's
    refusal to *open* one. Measured before it existed: against a complicit majority the
    honest complainant reached limited sanctions alone and stopped, and a compact that
    dismissed the same evidence four times read identically, on every instrument, to one
    that prosecuted it to collective sanctions.

    `ground` is `NeverPursued` (nothing ever cited the evidence) or `ClosedUnresolved`
    (a named decision closed a dispute citing it without resolving the matter, which
    `closing_decision` must name).

    It alleges no verdict: no protocol can tell a wrong dismissal from a right one. What
    it claims is structural and therefore checkable, and the authority pass checks it
    rather than taking it — refusing an attestation whose evidence is not adverse or not
    about the subject, whose attestor is the subject or shares no compact with it, whose
    `NeverPursued` ground is contradicted by a dispute already citing the evidence, whose
    named decision closes no dispute citing it, or whose evidence is still inside the
    90-day cadence. One signer, first-class: requiring a quorum to record that the
    quorum will not act would hand the shielding majority a veto over being noticed.
    Returns the CID."""
    if not evidence:
        raise ValueError("a non-pursuit attestation must cite the evidence it is about")
    if ground not in GRIEVANCE_GROUNDS:
        raise ValueError(f"ground must be one of {GRIEVANCE_GROUNDS}, got {ground!r}")
    if ground == "ClosedUnresolved" and not closing_decision:
        raise ValueError("the ClosedUnresolved ground must name its closing decision")
    if ground == "NeverPursued" and closing_decision:
        raise ValueError("the NeverPursued ground names no closing decision")
    return archive.append_non_pursuit(
        attestor.priv, attestor.did, attestor.did, subject_did,
        list(evidence), ground, reasoning, closing_decision, filed_at,
    )


def file_designation_dissent(
    archive,
    dissenter: Signer,
    designation_cid: str,
    reasoning: str,
    *,
    filed_at: Optional[int] = None,
) -> str:
    """Record `dissenter`'s refusal to join the designation at `designation_cid` (§5).

    A designation carries its deliberators and their reasoning and nothing else, so
    before this record a compact in which two communities designated and four would
    not read exactly like one that was unanimous. The one question a later reader most
    needs answered about the gravest act in the life of a compact, whether the compact
    agreed, could not be answered from the record.

    A dissent **vetoes nothing**: the quorum is the quorum, and a member's veto over
    being outvoted would be the capturing faction's veto over being named. What it
    does is put the split where `designation_consensus` can read it.

    The authority pass requires the designation to resolve, the dissent not to predate
    it, the author not to be among its deliberators (co-signing and refusing are not
    both available), the author not to be its subject (a subject's objection is its own
    defence, and it has the report response, the injunction challenge and the re-entry
    accounting), and the author to sit in a compact with at least one deliberator.
    One signer, first-class: this is a member speaking for itself. Returns the CID."""
    if filed_at is None:
        filed_at = now_seconds()
    return archive.append_designation_dissent(
        dissenter.priv, dissenter.did, dissenter.principal,
        designation_cid, reasoning, filed_at,
    )


def attest_activity(
    archive,
    community: Signer,
    function: str,
    window_start: int,
    window_end: int,
    *,
    acts: Optional[Sequence[str]] = None,
    statement: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """Account for one window in a standing function `community` holds (§4 and §8).
    `function` is one of `ATTESTED_FUNCTIONS`.

    **An empty `acts` is the point of the record**: it is the attested null the erosion
    audit asks for (remediation item 5). Without it a mediator pool that took no
    disputes because none arose and one that stopped answering are the same silence to
    every measure, and so are an observer corps with nothing to report and one that
    stopped looking. A null must state its account, because "no disputes were filed" and
    "we stopped convening" are both nulls and are not the same thing.

    Signed by the nominating community rather than by individual mediators or observers:
    the obligation is the community's for as long as it holds seats. Nothing is refused
    for a missing attestation; `silent_functions` and the erosion alert read it, since a
    compact that has stopped acting offers no act to refuse. Returns the CID."""
    if function not in ATTESTED_FUNCTIONS:
        raise ValueError(
            f"function must be one of {ATTESTED_FUNCTIONS}, got {function!r}"
        )
    return archive.append_activity_attestation(
        [community.priv], [community.did], community.did, function,
        window_start, window_end, list(acts or []), statement,
        attested_at if attested_at is not None else now_seconds(),
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
    draw_allowance: Optional[int] = None,
    level: str = "Community",
) -> str:
    """Register a commons (§7), co-signed by `registrars`. `kind` is one of
    `COMMONS_KINDS`; `appropriators` are the DIDs with draw rights (Ostrom's boundary).
    Registration is a second-class act: the authority pass requires a distinct-community
    quorum, and every registrar must itself be an appropriator. Returns the commons CID
    (pass it to `record_usage`, `apply_sanction`, and the reputation gate).

    `draw_allowance` is `provision_rule`'s machine-readable half: the per-period cap on
    one appropriator's draw, in the commons' own units. Without it the stated rule binds
    nothing, and the commons cannot host sealed usage since there would be no bound to
    prove against.

    `level` is one of `APPROPRIATOR_LEVELS`. At `"Compact"` the appropriators are whole
    compacts (see `charter_compact`) rather than communities, and **subsidiarity**
    applies: a sanction or escalation on that commons names a compact, never one of its
    member communities. A compact answers for its members; its members answer to it."""
    if kind not in COMMONS_KINDS:
        raise ValueError(f"kind must be one of {COMMONS_KINDS}, got {kind!r}")
    if level not in APPROPRIATOR_LEVELS:
        raise ValueError(f"level must be one of {APPROPRIATOR_LEVELS}, got {level!r}")
    if not registrars:
        raise ValueError("registration needs at least one registrar")
    if not appropriators:
        raise ValueError("a commons needs at least one appropriator")
    return archive.register_commons(
        [s.priv for s in registrars], [s.did for s in registrars],
        kind, name, description, list(appropriators), provision_rule, registered_at,
        draw_allowance, level,
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
    seal_custodian_dids: Optional[Sequence[str]] = None,
    seal_threshold: Optional[int] = None,
) -> str:
    """Record `community`'s self-reported usage of a commons (§7), signed by and
    attributed to it. First-class; the authority pass requires the reporter be an
    appropriator of `commons_cid`. `consumed`/`contributed` are dimensionless
    units in the commons' own terms and feed `free_rider_signals`. Returns the
    CID.

    **To account under the surveillance threshold** rather than in public, pass
    `seal_custodian_dids`. The figures are committed instead of published, a
    zero-knowledge proof that the draw is within the commons' `draw_allowance` goes on
    the record, and the figures themselves are sealed under k-of-n custody keyed to those
    communities. §7 asks for "configurable disclosure levels", and a community that must
    publish every figure to account honestly is disclosing its capacity, its consumption
    pattern and its reserves to every other member.

    The custodians must be fellow appropriators and may not include the reporter, so a
    community cannot hold a quorum over its own account. A quorum opens the figures later
    with `open_usage`.

    Sealing does not hide a community from `free_rider_signals`: the lens counts sealed
    records separately, reports the commons' accounting as incomplete, and turns each
    proof into a bound on what is hidden."""
    return archive.record_usage(
        community.priv, community.did, commons_cid,
        int(consumed), int(contributed), period_start, period_end, recorded_at,
        list(seal_custodian_dids) if seal_custodian_dids else None,
        seal_threshold,
    )


def open_usage_share(archive, usage_cid: str, custodian: Signer) -> str:
    """Open `custodian`'s own share of a sealed usage record (§7), returning it as JSON
    to be gathered into an `open_usage` call. A share is useless alone."""
    return archive.open_usage_share(usage_cid, custodian.priv, custodian.did)


def open_usage(
    archive,
    usage_cid: str,
    custodians: Sequence[Signer],
    consumed: int,
    contributed: int,
    shares: Sequence[str],
    cause: str,
    *,
    opened_at: Optional[int] = None,
) -> str:
    """Open a sealed usage record's figures (§7), co-signed by the `custodians` whose
    `shares` are presented and citing a `cause`.

    The pass requires the shares to recover figures that open the record's own
    commitments, so a custodian quorum cannot publish a number the community never
    committed to. Unlike a sealed mediation decision there is no waiting period: the
    threshold bounds *routine* disclosure, and an audit or a dispute is exactly the
    occasion for opening. Returns the CID."""
    if not custodians:
        raise ValueError("an opening needs at least one custodian")
    if not cause.strip():
        raise ValueError("opening a sealed usage record must cite a cause")
    return archive.open_usage(
        [c.priv for c in custodians], [c.did for c in custodians],
        usage_cid, int(consumed), int(contributed), list(shares), cause, opened_at,
    )


def charter_compact(
    archive,
    label: str,
    compact: Signer,
    members: Sequence[str],
    charterers: Sequence[Signer],
    *,
    chartered_at: Optional[int] = None,
) -> str:
    """Charter a compact (§7, Ostrom principle 8): give it a DID of its own, co-signed by
    the compact's key and a quorum of its member communities.

    Ostrom's nested-enterprise principle gives three layers, and
    `commons_federation.md` left the third as a forward pointer: commons shared between
    compacts, if compacts federate. Nothing at that level can be said until a compact is
    a thing that can be named, which before this was only a self-declared string.

    A community's identity is self-signed, because a community exists by its own act. A
    compact does not: it is constituted by its members. Its own signature is required
    too, or the DID would bind to a key nobody has shown they hold. Returns the CID.

    Pass the compact's DID as an appropriator of a `level="Compact"` commons."""
    if not members:
        raise ValueError("a compact is constituted by its members and this names none")
    if not charterers:
        raise ValueError("a charter needs at least one chartering member")
    return archive.charter_compact(
        compact.priv, compact.pub, label, list(members),
        [c.priv for c in charterers], [c.did for c in charterers], chartered_at,
    )


def escalate_commons(
    archive,
    commons_cid: str,
    subject_did: str,
    basis: str,
    cited_sanctions: Sequence[str],
    movers: Sequence[Signer],
    *,
    escalated_at: Optional[int] = None,
) -> str:
    """Find that a community's abuse of a commons has passed beyond what graduated
    sanctions can answer (§7), co-signed by `movers`. Second-class.

    This is the commons equivalent of the anathema designation, which
    `commons_federation.md` lists under open questions: the ladder and the designation
    track both existed and nothing joined them. The pass enforces four conditions, in the
    shape §5's own four-condition test carries:

    1. the subject is at `"Exclusion"` (graduated sanctions are the answer until they
       have demonstrably failed);
    2. it has arrived there more than once, which is defiance rather than compliance;
    3. it has stood there long enough that restoration was available and not taken;
    4. one arrival suffices on a life-critical commons (life support, rescue), which the
       compact keeps running when it pauses everything else.

    The finding gates nothing by itself. It is what a §5 designation may cite as its
    commons-side basis, so the escalation ladder is untouched; the threshold lives in the
    refusal to record it. Returns the CID."""
    if not movers:
        raise ValueError("a commons escalation needs at least one mover")
    if not cited_sanctions:
        raise ValueError("a commons escalation must cite the sanctions it rests on")
    return archive.escalate_commons(
        [m.priv for m in movers], [m.did for m in movers],
        commons_cid, subject_did, basis, list(cited_sanctions), escalated_at,
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


def insider_ring(archive, community_did: str, *, at: Optional[int] = None):
    """The keys the archive resolves as insiders of `community_did` — its root key
    plus every live speaker delegation — as `(did, pubkey_bytes)` pairs. This is the
    ring a whistleblower hides in; a community with fewer than two resolvable keys has
    no anonymous channel, because a ring of one names its signer."""
    return archive.insider_ring(community_did, at)


def attest_insider(
    archive,
    accused_did: str,
    source: Signer,
    claim: str,
    *,
    filed_at: Optional[int] = None,
) -> str:
    """The **source's** step of a whistleblower filing (§8): ring-sign an insider
    attestation against `accused_did` and commit to the source's identity.

    Returns the out-of-band packet the source hands the reception panel. The panel
    cannot forge it, and the source never surrenders its key. `source` must be one of
    the accused community's insiders (its root key or a live speaker delegate) — see
    `insider_ring`."""
    return archive.attest_insider(source.priv, accused_did, claim, filed_at)


def file_whistleblower(
    archive,
    accused_did: str,
    claim: str,
    reception_panel: Sequence[Signer],
    *,
    attestation: Optional[str] = None,
    source: Optional[Signer] = None,
    threshold: Optional[int] = None,
    references: Optional[Sequence[str]] = None,
    filed_at: Optional[int] = None,
) -> str:
    """The **panel's** step: file an insider whistleblower report against
    `accused_did` (§8), co-signed by a mediator `reception_panel`.

    Pass either `attestation` (the packet from `attest_insider`, which is how a real
    filing works — the source produces it separately and hands it over) or `source`
    as a convenience that performs both steps here. The panel seals the source
    identity under k-of-n custody keyed to its own members, `threshold` defaulting to
    a majority, so no minority of the panel can name the source later.

    The authority pass requires a non-party reception panel, an attestation from a
    real insider of the accused community bound to this exact claim, and custody held
    by exactly this panel. Returns the report CID."""
    if not reception_panel:
        raise ValueError("a reception panel needs at least one mediator")
    if (attestation is None) == (source is None):
        raise ValueError("pass exactly one of `attestation` or `source`")
    if attestation is None:
        attestation = attest_insider(
            archive, accused_did, source, claim, filed_at=filed_at,
        )
    return archive.file_whistleblower(
        [s.priv for s in reception_panel], [s.did for s in reception_panel],
        accused_did, claim, attestation, threshold, list(references or []),
    )


def open_source_share(archive, report_cid: str, holder: Signer) -> str:
    """One panelist opens its own custody share of a filed report's source (§8).
    A share is useless alone; it takes the filing's threshold of them to name
    anybody. Gather them and pass the list to `reveal_source`.

    Opened with the holder's **encryption** key, which is what the share was sealed
    to — not the key it signs with."""
    if not holder.enc_priv:
        raise ValueError(
            "a custody share is sealed to the holder's encryption key; this Signer has "
            "none (use new_mediator())"
        )
    return archive.open_source_share(report_cid, holder.enc_priv, holder.did)


def reveal_source(
    archive,
    report_cid: str,
    panel: Sequence[Signer],
    cause: str,
    disclosed_source: str,
    *,
    shares: Optional[Sequence[str]] = None,
    revealed_at: Optional[int] = None,
) -> str:
    """Disclose a whistleblower's source (§8), co-signed by a mediator `panel`,
    citing a non-empty `cause`, and carrying enough opened custody `shares` to
    actually recover the identity.

    If `shares` is omitted, every panelist in `panel` opens its own. The disclosure is
    gated twice over: the panel must be a non-party quorum, and the shares must
    genuinely reconstruct the identity that the filing committed to — a reveal cannot
    name someone the shares do not produce. Returns the CID."""
    if not panel:
        raise ValueError("a reveal needs at least one panelist")
    if not cause.strip():
        raise ValueError("a source reveal must cite a cause")
    if shares is None:
        shares = [open_source_share(archive, report_cid, s) for s in panel]
    return archive.reveal_source(
        [s.priv for s in panel], [s.did for s in panel],
        report_cid, cause, disclosed_source, list(shares), revealed_at,
    )


def exculpate_source(
    archive,
    report_cid: str,
    claimant: Signer,
    *,
    exculpated_at: Optional[int] = None,
) -> str:
    """Publish a proof that a whistleblower filing's key image is **not**
    `claimant`'s (§8) — the answer available to a member a reveal named wrongly.

    Needs nothing but the claimant's own key: no panel, no quorum, no permission. The
    archive refuses it if the proof is bad, if the claimant was never in the filing's
    ring, or if the claimed image matches the filing's (which would make the record a
    confession rather than a denial). Returns the CID."""
    return archive.exculpate_source(
        claimant.priv, claimant.did, report_cid, exculpated_at,
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
# §9 the four sites of the adopted minimum content (natural_law.md "The four sites").
MINIMUM_SITES = (
    "NonMemberDuties", "HostisHumaniGeneris", "RestrictedMember", "BoundaryRefusal",
)
# §9 minimum-content amendment directions; Narrow carries the higher bar.
FORBEARANCE_CHANGES = ("Broaden", "Narrow")


def ratify_constitution(
    archive,
    ratifiers: Sequence[Signer],
    incapable: Sequence[tuple[str, str]],
    fast_periods: Sequence[tuple],
    *,
    genesis_at: Optional[int] = None,
) -> str:
    """Ratify the founding constitution (§9), co-signed by `ratifiers`. `incapable`
    is a list of `(key, description)` compact-incapable matters (genesis-immutable);
    `fast_periods` is a list of `(label, start, end)` structured-fast windows, or
    `(label, start, end, recurrence)` where `recurrence` is a cycle in seconds for a
    repeating fast (a calendar rhythm). Second-class (needs a distinct-community
    quorum) and a singleton per archive. Returns the CID."""
    if not ratifiers:
        raise ValueError("ratification needs at least one ratifier")

    def _period(p):
        lbl, start, end = p[0], int(p[1]), int(p[2])
        recurrence = int(p[3]) if len(p) > 3 and p[3] is not None else None
        return (lbl, start, end, recurrence)

    return archive.ratify_constitution(
        [s.priv for s in ratifiers], [s.did for s in ratifiers],
        [(k, d) for (k, d) in incapable],
        [_period(p) for p in fast_periods],
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
    recurrence: Optional[int] = None,
) -> str:
    """Amend the structured-fast cadence (§9). `change` is one of `FAST_CHANGES`;
    `Contract` carries the higher bar. `(label, start, end)` is the resulting fast
    period; pass `recurrence` (a cycle in seconds) to make it a repeating fast.
    Returns the CID."""
    if change not in FAST_CHANGES:
        raise ValueError(f"change must be one of {FAST_CHANGES}, got {change!r}")
    if not deciders:
        raise ValueError("an amendment needs at least one decider")
    return archive.amend_fast_cadence(
        [s.priv for s in deciders], [s.did for s in deciders],
        change, label, int(start), int(end), rationale, decision_class, decided_at,
        recurrence,
    )


def adopt_minimum_content(
    archive,
    adopters: Sequence[Signer],
    forbearances: Sequence[tuple],
    *,
    decision_class: str = "SecondClass",
    adopted_at: Optional[int] = None,
) -> str:
    """Adopt the functional minimum as the ground of the compact's own conduct (§9),
    co-signed by `adopters`. `forbearances` is a list of `(key, description)` or
    `(key, description, site)` where site is one of `MINIMUM_SITES`. Second-class and
    a singleton per archive. Returns the CID.

    Not an entry on the compact-incapable list: that list names matters belonging to a
    member, this one binds the compact's own conduct at its boundary."""
    if not adopters:
        raise ValueError("an adoption needs at least one adopter")
    if not forbearances:
        raise ValueError("an adoption must name at least one forbearance")

    def _forbearance(f):
        key, description = f[0], f[1]
        site = f[2] if len(f) > 2 and f[2] is not None else "NonMemberDuties"
        if site not in MINIMUM_SITES:
            raise ValueError(f"site must be one of {MINIMUM_SITES}, got {site!r}")
        return (key, description, site)

    return archive.adopt_minimum_content(
        [s.priv for s in adopters], [s.did for s in adopters],
        [_forbearance(f) for f in forbearances],
        decision_class, adopted_at,
    )


def amend_minimum_content(
    archive,
    change: str,
    forbearance_key: str,
    deciders: Sequence[Signer],
    *,
    description: str = "",
    site: str = "NonMemberDuties",
    rationale: str = "",
    decision_class: str = "SecondClass",
    decided_at: Optional[int] = None,
) -> str:
    """Amend the adopted minimum content (§9). `change` is one of
    `FORBEARANCE_CHANGES`. `Broaden` uses the normal quorum; `Narrow` requires the
    higher contraction bar — at every site, and for a founding forbearance too, which
    is where this artifact parts from the compact-incapable list. Returns the CID."""
    if change not in FORBEARANCE_CHANGES:
        raise ValueError(f"change must be one of {FORBEARANCE_CHANGES}, got {change!r}")
    if site not in MINIMUM_SITES:
        raise ValueError(f"site must be one of {MINIMUM_SITES}, got {site!r}")
    if not deciders:
        raise ValueError("an amendment needs at least one decider")
    return archive.amend_minimum_content(
        [s.priv for s in deciders], [s.did for s in deciders],
        change, forbearance_key, description, site, rationale, decision_class,
        decided_at,
    )


__all__ = [
    "Signer", "new_identity", "delegate_speaker", "revoke_delegation", "vouch",
    "attest_interpretation", "attest_interpreter_independence",
    "report_event", "corroborate_event",
    "invoke_event_action", "find_undisclosed_interest",
    "attest_principal", "record_principal_output",
    "record_conduct", "record_concern", "designate_anathema", "lift_designation",
    "record_non_admission", "record_admission", "run_intra_rite",
    "file_designation_dissent",
    "new_mediator", "new_examiner",
    "register_examiners", "probationary_at",
    "new_member", "publish_member_roll", "insider_set_size",
    "register_mediators", "file_dispute", "decide_mediation",
    "record_panel_consent", "invoke_skepticism", "review_skepticism",
    "open_decision_share", "unseal_decision",
    "CONSENT_ANSWERS", "REVIEW_ANSWERS", "SKEPTICISM_CRITERIA",
    "file_ladder_transition", "decide_designation", "impose_restitution",
    "verify_restitution", "complete_reentry", "invoke_substantive_reengagement",
    "issue_injunction", "confirm_injunction", "challenge_injunction",
    "register_commons", "record_usage", "apply_sanction",
    "open_usage_share", "open_usage", "charter_compact", "escalate_commons",
    "APPROPRIATOR_LEVELS",
    "new_observer", "register_observers", "publish_observer_report",
    "respond_to_report", "insider_ring", "attest_insider", "file_whistleblower",
    "open_source_share", "reveal_source", "exculpate_source",
    "ratify_constitution", "amend_compact_incapable", "amend_fast_cadence",
    "adopt_minimum_content", "amend_minimum_content",
    "AMENDMENT_KINDS", "FAST_CHANGES", "MINIMUM_SITES", "FORBEARANCE_CHANGES",
    "now_seconds",
    "CONDUCT_KINDS", "ANTI_SPIRAL_FEATURES", "BEARINGS", "SEVERITIES",
    "LADDER_RUNGS", "DECISION_CLASSES", "DELEGATION_SCOPES", "DECISION_OUTCOMES",
    "DESIGNATION_TIERS", "DESIGNATION_URGENCIES", "CAPABILITIES",
    "SERVICE_CAPABILITIES", "OFFICE_CAPABILITIES",
    "COMMONS_KINDS", "SANCTION_LEVELS",
]
