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

"""Author charter records from Python, in terms of `Signer`s.

The `_core` methods take co-signers as two parallel lists (DIDs and key bytes).
These wrappers take a sequence of `Signer` instead, which is what callers
actually hold, and flatten it — the same relationship `kith_covenant.authoring`
has to its own bindings. Signing happens in the Rust core; a `Signer` holds raw
key bytes the caller is responsible for.

The usual arc of a polity's life:

    from ethne import Archive, Signer, generate_keypair, propose_charter
    from ethne.authoring import admit_member, induct_member, ratify_charter

    founders = [Signer.generate() for _ in range(3)]
    polity = Signer.generate()

    proposal = propose_charter(polity.did, "tradition", "answers to X above itself",
                              member_dids=[s.did for s in founders])
    arch = Archive()
    ratify_charter(arch, proposal, founders[:2])      # a majority ratifies
    joiner = Signer.generate()
    admit_member(arch, polity.did, joiner, founders[:2])
    induct_member(arch, polity.did, joiner, founders)  # majority of the grown roll
    arch.verify_all()
    arch.check_authority()

Records do not verify on append — a batch may be assembled out of order — so call
`archive.verify_all()` and `archive.check_authority()` once it is complete.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Optional, Sequence

from ethne import _core

#: Membership-lifecycle record kinds, for discoverability.
MEMBERSHIP_KINDS = ("Admit", "DepartVoluntary", "Expel")
#: Office-lifecycle record kinds.
OFFICE_KINDS = ("Confer", "Resign", "Revoke")
#: Guardian-edge record kinds — which members are machines, and whose.
GUARDIAN_KINDS = ("Designate", "Rotate", "Release")
#: Envoy-standing record kinds — who may speak for the polity beyond it.
ENVOY_KINDS = ("Authorize", "Resign", "Revoke")
#: Affirmation kinds — the transmission of the substrate, and its renewal.
AFFIRMATION_KINDS = ("Induction", "ReAffirmation")


@dataclass(frozen=True)
class Signer:
    """A key that signs charter records, and the DID it signs as.

    `priv` is a secret; treat this object as sensitive — it deliberately does not
    print its key material. For a polity's own root identity (the key the uplift
    calls sign with) `did` is the polity DID; for a member it is the member's.
    """

    priv: bytes
    did: str

    @classmethod
    def generate(cls) -> "Signer":
        """A fresh Ed25519 identity."""
        priv, _pub, did = _core.generate_ed25519_keypair()
        return cls(priv=priv, did=did)

    def __repr__(self) -> str:  # never leak key material
        return f"Signer(did={self.did!r}, priv=<{len(self.priv)} bytes>)"


def _split(signers: Sequence[Signer]) -> tuple[list[str], list[bytes]]:
    """A signer sequence as the two parallel lists `_core` takes."""
    return [s.did for s in signers], [s.priv for s in signers]


# ---- Founding ---------------------------------------------------------------


def ratify_charter(
    archive,
    proposal_json: str,
    ratifiers: Sequence[Signer],
    founded_at: Optional[int] = None,
    guardians: Sequence[Signer] = (),
    anchor: Optional[Signer] = None,
) -> str:
    """Ratify a proposal into the polity's genesis charter. The ratifiers must be
    a majority of the proposed roll — emergence proposes, the polity ratifies.

    Where the proposal carries derivations, every named `guardian` must co-sign as
    well: answerability for a machine is accepted, never assigned.

    Where its substrate names an `anchor` (D23), that key co-signs too — the same
    principle one level up. You may not conscript what you claim to stand under, so a
    ground nobody signed for is not a ground the founding may claim."""
    dids, privs = _split(ratifiers)
    g_dids, g_privs = _split(guardians)
    return archive.ratify_charter(
        proposal_json,
        dids,
        privs,
        g_dids,
        g_privs,
        anchor.did if anchor is not None else None,
        anchor.priv if anchor is not None else None,
        founded_at,
    )


# ---- Commitments ------------------------------------------------------------


def amend_commitment(
    archive,
    polity_did: str,
    kind: str,
    key: str,
    deciders: Sequence[Signer],
    description: str = "",
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Add (majority) or remove (majority + 1) a commitment. Founding commitments
    are genesis-immutable, and the substrate has no amendment path at all."""
    dids, privs = _split(deciders)
    return archive.amend_commitment(
        polity_did, kind, key, description, rationale, dids, privs, decided_at
    )


def amend_content_policy(
    archive,
    polity_did: str,
    policy: dict,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Replace the polity's content policy whole (D37). ``policy`` is a dict with
    ``operator_bound_required`` (bool), ``min_tier`` (0..4), ``daily_post_cap``
    (int or None) and ``labels`` ({label: "Allow"|"Warn"|"Hide"}). A majority
    tightens it; loosening anything needs a majority + 1."""
    dids, privs = _split(deciders)
    return archive.amend_content_policy(
        polity_did, json.dumps(policy, sort_keys=True), rationale, dids, privs, decided_at
    )


def content_policy_state(archive, polity_did: str, as_of: Optional[int] = None) -> dict:
    """The policy in force as of ``as_of`` and who may label under it (D37):
    ``{"policy": dict, "set_by": cid or None, "moderators": [did]}``."""
    policy, set_by, moderators = archive.content_policy_state(polity_did, as_of)
    return {"policy": json.loads(policy), "set_by": set_by, "moderators": moderators}


# ---- Compacts (D40) -----------------------------------------------------------

COMPACT_ACT_KINDS = ("Join", "Admit", "Sponsor", "Refuse", "Anathematize", "Lift")


def decide_compact_act(
    archive,
    polity_did: str,
    kind: str,
    compact: str,
    counterparties: Sequence[str],
    deciders: Sequence[Signer],
    probation_days: Optional[int] = None,
    rationale: str = "",
    decided_at: Optional[int] = None,
    expires_at: Optional[int] = None,
) -> str:
    """Record the roll's decision about a kith-covenant compact (D40): the mandate
    the polity's envoy signs under at KC's border. ``kind`` is one of
    :data:`COMPACT_ACT_KINDS`. A majority decides; ``Refuse`` and ``Anathematize``
    bear down on another community and need a majority + 1."""
    dids, privs = _split(deciders)
    return archive.decide_compact_act(
        polity_did, kind, compact, list(counterparties), dids, privs,
        probation_days, rationale, decided_at, expires_at,
    )


def compact_state(archive, polity_did: str, as_of: Optional[int] = None) -> list:
    """The compact acts in force as of ``as_of`` (D40): ``[{"cid", "act"}]``."""
    return json.loads(archive.compact_state(polity_did, as_of))


# ---- Membership -------------------------------------------------------------


def admit_member(
    archive,
    polity_did: str,
    joiner: Signer,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Admit `joiner` at a majority of the live roll, with the joiner's own
    co-signature (no conscription). Admission puts them on the roll; it does not
    transmit the substrate — see `induct_member`."""
    dids, privs = _split(deciders)
    return archive.admit_member(
        polity_did, joiner.did, joiner.priv, dids, privs, rationale, decided_at
    )


def depart_member(
    archive,
    polity_did: str,
    member: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """A member leaves of their own accord: self-signed, no deciders. Refused only
    if it would drop the roll below the self-govern floor."""
    return archive.depart_member(
        polity_did, member.did, member.priv, rationale, decided_at
    )


def expel_member(
    archive,
    polity_did: str,
    member_did: str,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Expel a member at a majority + 1, with the target excluded from the
    deciders."""
    dids, privs = _split(deciders)
    return archive.expel_member(
        polity_did, member_did, dids, privs, rationale, decided_at
    )


# ---- Offices ----------------------------------------------------------------


def confer_office(
    archive,
    polity_did: str,
    office: str,
    holder: Signer,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Confer an office at a majority, with the holder's consent. The roll confers
    offices; offices never confer offices."""
    dids, privs = _split(deciders)
    return archive.confer_office(
        polity_did, office, holder.did, holder.priv, dids, privs, rationale, decided_at
    )


def declare_relay_endpoint(
    archive,
    polity_did: str,
    member: Signer,
    at_uuid: str,
    host: str,
    port: int,
    declared_at: Optional[int] = None,
    areas: Optional[list] = None,
) -> str:
    """A rendezvous operator states where its relay listens (D36). Signed by the
    member alone, and authorized only while it holds ``RendezvousOperator``.
    ``at_uuid`` is the member node's AT identity uuid, lowercase and hyphenated.
    ``areas`` (D38) makes the relay an area hub for those geohash areas."""
    return archive.declare_relay_endpoint(
        polity_did, member.did, member.priv, at_uuid, host, port, declared_at, areas
    )


def withdraw_relay_endpoint(
    archive,
    polity_did: str,
    member: Signer,
    at_uuid: str,
    declared_at: Optional[int] = None,
) -> str:
    """The member takes back its relay declaration. Needs no office, so a revoked
    operator can still withdraw."""
    return archive.withdraw_relay_endpoint(
        polity_did, member.did, member.priv, at_uuid, declared_at
    )


def resign_office(
    archive,
    polity_did: str,
    office: str,
    holder: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The holder lays an office down: self-signed."""
    return archive.resign_office(
        polity_did, office, holder.did, holder.priv, rationale, decided_at
    )


def revoke_office(
    archive,
    polity_did: str,
    office: str,
    holder_did: str,
    deciders: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The roll takes an office back at a majority, holder excluded."""
    dids, privs = _split(deciders)
    return archive.revoke_office(
        polity_did, office, holder_did, dids, privs, rationale, decided_at
    )


# ---- Founding-boundary dissent ----------------------------------------------


def dissent_boundary(
    archive,
    polity_did: str,
    dissenter: Signer,
    kind: str,
    rationale: str = "",
    dissented_at: Optional[int] = None,
) -> str:
    """File a dissent from a founding's boundary (D22).

    `kind` is `"Unnamed"` — the accounting never mentioned me, which is the claim
    that contradicts an understated `candidate_size` and the one thing nothing else
    in the tier can reach — or `"ReasonRejected"`, where the dissenter is named as
    excluded and rejects the reason given for it.

    Perspectival evidence, exactly as an observation is: signed by the dissenter
    alone, no deciders, no majority, and the dissenter need not be a member — one
    required to sit inside the roll could be silenced by never being admitted. It is
    also the only record a **dissolved** polity still accepts, because otherwise
    founders silence the parties they excluded by winding the polity up.

    A dissent asserts; it does not adjudicate. Nothing decides whether it is true."""
    return archive.dissent_boundary(
        polity_did, dissenter.did, dissenter.priv, kind, rationale, dissented_at
    )


# ---- Substrate disavowal ----------------------------------------------------


def disavow_substrate(
    archive,
    polity_did: str,
    anchor: Signer,
    rationale: str = "",
    disavowed_at: Optional[int] = None,
) -> str:
    """File the chartered substrate anchor's withdrawal from a polity (D23).

    D23 made a keyed anchor a *consenting* party — it co-signs the founding, because
    answerability is accepted and never assigned. A ground that can consent can also
    change its mind, and this is how it says so.

    Only the anchor the charter names may file this, and it signs alone. That is
    stricter than a dissent, which has to admit any filer because an omitted party is
    by definition absent from the record; the ground is *on* the record.

    **It asserts; it does not un-charter.** The substrate is genesis-immutable, so
    nothing here voids the founding, dissolves the polity, or stops it governing
    itself — a withdrawal with that power would be a unilateral kill switch in every
    anchor's hands. What it produces is a legible fact, which `cohesion_report` reads
    as a Concern. Like a dissent it survives dissolution and cannot be pruned."""
    return archive.disavow_substrate(
        polity_did, anchor.did, anchor.priv, rationale, disavowed_at
    )


# ---- Independence attestation -----------------------------------------------


def attest_reader_independence(
    archive,
    polity_did: str,
    reader_did: str,
    attester: Signer,
    kind: str = "Independent",
    basis: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """File a third party's account of whether a reader of a founding is an independent
    principal (D24).

    `kind` is `"Independent"` (vouching) or `"Controlled"` — the counter-claim, so the
    lens can be contradicted by somebody who thinks it is being fooled.

    **Why this record exists.** D22 made a boundary's reading attributable: signed by
    the reader it names, bound to that founding. What it could not do is tell one reader
    from five faces of the same actor — measured, one party minted five keys, signed five
    identical readings, and the charter graded `Corroborated`, at 32 microseconds a
    voice. A count of distinct signing readers therefore cannot mean what it appears to.

    The attester signs alone, and `reader_did` must actually be a reader of that
    founding. Whether the attester is *disinterested* is the lens' concern, not this
    call's: an attestation from the polity, the reader itself, or a founding member is
    accepted and reported separately rather than refused.

    **What it buys, plainly:** not proof of independence, and nothing adjudicates the
    claim. It moves the cost of faking corroboration from one key to a second party
    willing to be named for it, permanently. Like a dissent and a disavowal it survives
    dissolution and cannot be pruned."""
    return attest_independence(
        archive, polity_did, reader_did, attester,
        role="BoundaryReader", kind=kind, basis=basis, attested_at=attested_at,
    )


def attest_observer_independence(
    archive,
    polity_did: str,
    observer_did: str,
    attester: Signer,
    kind: str = "Independent",
    basis: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """File a third party's account of whether one of a polity's **monitors** is an
    independent principal (D28).

    **Why this exists.** D15 and D27 each closed a hole in guardian evidence and each
    named the reader attestation above as the successor for what was left — that the
    evidence is still one observer's perspectival reading. Measured, the successor did
    not reach: its coherence gate demanded the subject be a *reader of the boundary*,
    and an observer is not one, so filing about a monitor was refused outright.

    The same measurement found the sharper case, and it points **outward**. A
    confederate observer, on nobody's roll and costing one keypair, signs an
    observation showing any guardian edge it likes and the machine cluster grades
    `Guarded` with no anchor — and, filing three times, manufactures the
    sustained-absence `Concern` that crosses to kith-covenant. The residue's wording
    only ever described a polity flattering itself.

    `observer_did` must actually have observed this polity already: a monitor has to
    **speak before it can be vouched for**, since pre-registering a witness is the
    sockpuppet move with a longer setup.

    **Nothing refuses an unvouched monitor**, deliberately. A polity that simply
    declines to vouch for anybody must not thereby become unwatchable."""
    return attest_independence(
        archive, polity_did, observer_did, attester,
        role="Observer", kind=kind, basis=basis, attested_at=attested_at,
    )


def attest_independence(
    archive,
    polity_did: str,
    subject_did: str,
    attester: Signer,
    role: str = "BoundaryReader",
    kind: str = "Independent",
    basis: str = "",
    attested_at: Optional[int] = None,
) -> str:
    """File an independence attestation about a party behind `polity_did` in `role`
    (D24, generalized to observers by D28).

    `role` is `"BoundaryReader"` or `"Observer"`; `kind` is `"Independent"` or
    `"Controlled"`. The two roles share one record and one set of counting rules,
    because they are the same claim about the same kind of party; only the coherence
    gate differs, since each is checkable against a different part of the log.

    A key may hold both roles for the same polity, and a voucher for one is **not** a
    voucher for the other: vouching is always for a party *in a capacity*, since what
    an outsider can speak to is a relationship rather than an identity in the
    abstract."""
    return archive.attest_independence(
        polity_did, subject_did, attester.did, attester.priv,
        role, kind, basis, attested_at,
    )


# ---- Observation ------------------------------------------------------------


def record_observation(
    archive,
    polity_did: str,
    observer: Signer,
    readings: Sequence[tuple[str, float, Optional[str], Optional[int]]],
    opaque_boundary: bool = False,
    observed_at: Optional[int] = None,
) -> str:
    """Record what a monitor saw: `(member_did, reputation, guardian_did, attested_at)`
    per member observed.

    An observation asserts nothing about the polity — it is one party's perspectival
    evidence, signed by whoever looked, and the observer need not be a member. What it
    makes possible is duration: only a record of observation can say how long a
    guardian has been absent, and only that can escalate."""
    return archive.record_observation(
        polity_did, observer.did, observer.priv, list(readings), opaque_boundary, observed_at
    )


def prune_observations(
    archive,
    polity_did: str,
    horizon: int,
    deciders: Sequence[Signer],
    rationale: str = "",
    pruned_at: Optional[int] = None,
) -> str:
    """Drop the observations recorded before `horizon`, leaving a signed record of
    the fact and a receipt for what went.

    Observations are the one entry class a polity may drop: they decide nothing, so
    removing one cannot rewrite what the polity resolved. An effective
    `CharterKeeper` among the deciders suffices beyond the settling horizon (30
    days) — the custody that office is — while a majority of the roll may prune
    anything. Recent evidence is deliberately beyond any single custodian's reach,
    because destroying it is exactly the move a captured party makes.

    This authors the record. Call `archive.apply_prunings(polity_did)` to get an
    archive with the observations actually dropped."""
    return archive.prune_observations(
        polity_did, horizon, [d.did for d in deciders], [d.priv for d in deciders],
        rationale, pruned_at,
    )


# ---- The guardian edge ------------------------------------------------------


def designate_derived(
    archive,
    polity_did: str,
    member_did: str,
    deciders: Sequence[Signer],
    guardian: Optional[Signer] = None,
    evidence_cid: Optional[str] = None,
    unbacked_reason: Optional[str] = None,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Designate a member a derived (machine) party at a majority, with the named
    guardian co-signing its acceptance. Passing no guardian charters a machine with
    nobody behind it — honest, and refused an office where machines are the
    majority.

    Naming a guardian claims an edge, so D15 requires it to be backed: pass
    `evidence_cid`, the CID of an observation in this charter whose reading for
    `member_did` shows that guardian, or `unbacked_reason` to declare outright that no
    monitor has reported it. The observation is **copied into** the record rather than
    referenced, so a later pruning of the observation log cannot un-authorize the edge.

    A guardian whose key is not on this host signs through
    `pending_guardian_change` / `append_guardian_change_signed` instead."""
    dids, privs = _split(deciders)
    return archive.designate_derived(
        polity_did, member_did, dids, privs,
        guardian.did if guardian else None,
        guardian.priv if guardian else None,
        evidence_cid, unbacked_reason,
        rationale, decided_at,
    )


def rotate_guardian(
    archive,
    polity_did: str,
    member_did: str,
    guardian: Signer,
    deciders: Sequence[Signer],
    evidence_cid: Optional[str] = None,
    unbacked_reason: Optional[str] = None,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Point a designated member at a (new) guardian — the operator changed, or an
    unguarded machine acquires one. A majority plus the incoming guardian's consent.

    A rotation always claims an edge, so it always needs backing: see
    `designate_derived` for `evidence_cid` and `unbacked_reason`."""
    dids, privs = _split(deciders)
    return archive.rotate_guardian(
        polity_did, member_did, guardian.did, guardian.priv, dids, privs,
        evidence_cid, unbacked_reason, rationale, decided_at,
    )


def pending_guardian_change(
    archive,
    polity_did: str,
    member_did: str,
    kind: str,
    deciders: Sequence[Signer],
    guardian_did: Optional[str] = None,
    evidence_cid: Optional[str] = None,
    unbacked_reason: Optional[str] = None,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> tuple:
    """Build a guardian change without signing it. Returns
    `(cid, payload_bytes, description)`.

    The detached path, for the arrangement AT's keystore location assumes: the
    guardian's key lives with the guardian, not on the polity's authoring host. The
    bytes travel to them, they sign in their own process (AT's operator CLI has a verb
    for it), and `append_guardian_change_signed` puts the signatures back on. The
    private key never enters this process.

    **Show `description` to whoever is signing.** Asking a human to sign opaque bytes
    teaches them to sign anything.

    `deciders` are named, not signed for, here — they may sign the same bytes by any
    route. Pass an explicit `decided_at` if the exchange may outlive the process: the
    pending record is held in memory and the CID is a function of every argument."""
    dids, _ = _split(deciders)
    return archive.pending_guardian_change(
        polity_did, member_did, kind, dids, guardian_did,
        evidence_cid, unbacked_reason, rationale, decided_at,
    )


def append_guardian_change_signed(archive, cid: str, signatures: Sequence[tuple]) -> str:
    """Attach collected detached signatures to a pending guardian change and append it.

    `signatures` is a sequence of `(signer_did, signature_bytes)`. Every one is
    verified against the pending payload before anything is appended, so a wrong key or
    a stale exchange fails here with a reason rather than later as an unauthorized
    record. Returns the entry CID."""
    return archive.append_guardian_change_signed(cid, list(signatures))


def sign_detached(signer: Signer, payload_bytes: bytes) -> tuple:
    """Sign exported payload bytes, returning `(did, signature)` for
    `append_guardian_change_signed`.

    Provided for the co-located case and for tests. A guardian whose key is in their
    own keystore signs with AT's operator verb instead — that is the whole point of
    the detached path."""
    return (signer.did, _core.sign_detached(signer.priv, payload_bytes))


def release_guardian(
    archive,
    polity_did: str,
    member_did: str,
    guardian: Optional[Signer] = None,
    deciders: Sequence[Signer] = (),
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """Record that a member's guardian edge is gone. Either the departing `guardian`
    signs (stepping away) or a majority of the `deciders` records that the human is
    no longer there. The designation survives the release."""
    dids, privs = _split(deciders)
    return archive.release_guardian(
        polity_did, member_did,
        guardian.did if guardian else None,
        guardian.priv if guardian else None,
        dids, privs, rationale, decided_at,
    )


# ---- Envoy standing ---------------------------------------------------------


def authorize_envoy(
    archive,
    polity_did: str,
    envoy: Signer,
    authorizers: Sequence[Signer],
    rationale: str = "",
    expires_at: Optional[int] = None,
    decided_at: Optional[int] = None,
) -> str:
    """Authorize an envoy to speak for the polity beyond it. Either a single
    `EnvoyAuthorizer` (the office's delegated power) or a majority of the roll may
    do it, and the envoy always co-signs — the duty is never conscripted."""
    dids, privs = _split(authorizers)
    return archive.authorize_envoy(
        polity_did, envoy.did, envoy.priv, dids, privs, rationale, expires_at, decided_at
    )


def resign_envoy(
    archive,
    polity_did: str,
    envoy: Signer,
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The envoy stands down: self-signed."""
    return archive.resign_envoy(
        polity_did, envoy.did, envoy.priv, rationale, decided_at
    )


def revoke_envoy(
    archive,
    polity_did: str,
    envoy_did: str,
    authorizers: Sequence[Signer],
    rationale: str = "",
    decided_at: Optional[int] = None,
) -> str:
    """The polity withdraws the standing, on the same two routes, envoy excluded."""
    dids, privs = _split(authorizers)
    return archive.revoke_envoy(
        polity_did, envoy_did, dids, privs, rationale, decided_at
    )


# ---- Transmission -----------------------------------------------------------


MOVEMENTS = ("Substrate", "Fallenness", "Recognition", "CommonVirtue", "NonUltimacy")
"""The five movements of the rite, in order (D29). Order is semantic."""


def recite_movement(
    archive,
    polity_did: str,
    member: Signer,
    movement: str,
    statement: str,
    attending: Sequence[Signer],
    names: Optional[str] = None,
    commits_to: Sequence[str] = (),
    names_anchor: Optional[str] = None,
    recited_at: Optional[int] = None,
    kind: str = "Induction",
) -> str:
    """Recite one movement of `member`'s rite, returning its CID.

    A rite opens with `"Substrate"` and each later movement must follow its
    predecessor. `statement` is the polity's own words: required, never read. What is
    checked is coherence with the charter — `names` must be the chartered substrate's
    name, `commits_to` the commitments in force, `names_anchor` the chartered anchor.
    Everyone attending co-signs, attendees must already hold the substrate, and nobody
    attends their own rite.

    `kind` says which ceremony this is (**D34**): `"Induction"` for a member's first
    acquisition of the substrate, `"ReAffirmation"` for the periodic re-recitation a
    renewal carries. It is checked against the member's actual state, so an induction
    movement for someone who already holds the substrate is refused, and a renewal
    movement for someone who never received it likewise. A rite's kind is set by its
    opening movement and every later movement must agree.
    """
    dids, privs = _split(attending)
    return archive.recite_movement(
        polity_did, member.did, member.priv, movement, statement, dids, privs,
        names, list(commits_to), names_anchor, recited_at, kind,
    )


def perform_rite(
    archive,
    polity_did: str,
    member: Signer,
    attending: Sequence[Signer],
    substrate_name: str,
    commitments: Sequence[str] = (),
    anchor: Optional[str] = None,
    start: Optional[int] = None,
    seconds_between: int = 1,
    kind: str = "Induction",
) -> list[str]:
    """Recite all five movements in order, returning their CIDs for the affirmation.

    The convenience path for a polity whose rite happens in one sitting; a polity
    holding its rite over a week calls `recite_movement` five times with its own
    instants, which is the difference the record now carries.

    Pass `kind="ReAffirmation"` for the re-recitation a renewal carries (**D34**), then
    hand the CIDs to `reaffirm_member`. `attending` may be empty for a renewal — nobody's
    attendance is required — and must never include `member`, who is reciting.
    """
    cids = []
    for i, movement in enumerate(MOVEMENTS):
        at = None if start is None else start + i * seconds_between
        cids.append(
            recite_movement(
                archive, polity_did, member, movement,
                f"{movement}: as we have received it", attending,
                names=substrate_name if movement == "Substrate" else None,
                commits_to=commitments if movement == "CommonVirtue" else (),
                names_anchor=anchor if movement == "NonUltimacy" else None,
                recited_at=at, kind=kind,
            )
        )
    return cids


def induct_member(
    archive,
    polity_did: str,
    member: Signer,
    witnesses: Sequence[Signer],
    movement_cids: Sequence[str],
    affirmed_at: Optional[int] = None,
) -> str:
    """Complete a member's induction into the substrate (D29).

    `movement_cids` must be the five movements of this member's rite, in order; the
    witnesses must have attended every one of them and must be a majority of the
    members who already hold the substrate. The charter's ratifiers need no
    induction — ratifying was theirs.
    """
    dids, privs = _split(witnesses)
    return archive.induct_member(
        polity_did, member.did, member.priv, dids, privs, list(movement_cids), affirmed_at
    )


def reaffirm_member(
    archive,
    polity_did: str,
    member: Signer,
    movement_cids: Sequence[str],
    witnesses: Sequence[str] = (),
    affirmed_at: Optional[int] = None,
) -> str:
    """Renew an inducted member's affirmation on a **full re-recitation of the five
    movements** (**D34**), which is what the doctrine means by periodic re-affirmation.

    `movement_cids` are the five `perform_rite(..., kind="ReAffirmation")` records, in
    order; `witnesses` must be exactly those who attended every one of them, which may be
    nobody. Attendance is recorded, never required — the renewal is signed by the member
    alone, because it is precisely the act no one else may condition.

    Freshness is what `AffirmationState.stale_members` reads. A bare renewal with no
    re-recitation is refused: before D34 it was the only form, which made the recurring
    ceremony the one with no measurable cost.
    """
    return archive.reaffirm_member(
        polity_did, member.did, member.priv, list(movement_cids), list(witnesses),
        affirmed_at,
    )


def renew_member(
    archive,
    polity_did: str,
    member: Signer,
    substrate_name: str,
    attending: Sequence[Signer] = (),
    commitments: Sequence[str] = (),
    anchor: Optional[str] = None,
    start: Optional[int] = None,
    seconds_between: int = 1,
) -> str:
    """Perform a member's re-recitation and record the renewal it completes — the
    one-sitting convenience path for `perform_rite(kind="ReAffirmation")` plus
    `reaffirm_member` (**D34**).

    `attending` may be empty. Whoever attends is recorded as a witness, since the rule
    is that witnesses are exactly the attendance-throughout rather than a chosen list.
    """
    cids = perform_rite(
        archive, polity_did, member, attending, substrate_name,
        commitments=commitments, anchor=anchor, start=start,
        seconds_between=seconds_between, kind="ReAffirmation",
    )
    at = None if start is None else start + (len(MOVEMENTS) - 1) * seconds_between
    return reaffirm_member(
        archive, polity_did, member, cids,
        witnesses=[s.did for s in attending], affirmed_at=at,
    )


# ---- Dissolution ------------------------------------------------------------


def dissolve_charter(
    archive,
    polity_did: str,
    deciders: Sequence[Signer],
    successors: Sequence[str] = (),
    rationale: str = "",
    dissolved_at: Optional[int] = None,
) -> str:
    """Wind the polity up at a majority, optionally naming successors. Terminal:
    no charter event is authorized afterwards."""
    dids, privs = _split(deciders)
    return archive.dissolve_charter(
        polity_did, dids, privs, list(successors), rationale, dissolved_at
    )


__all__ = [
    "AFFIRMATION_KINDS",
    "ENVOY_KINDS",
    "GUARDIAN_KINDS",
    "MEMBERSHIP_KINDS",
    "OFFICE_KINDS",
    "Signer",
    "admit_member",
    "amend_commitment",
    "authorize_envoy",
    "confer_office",
    "depart_member",
    "designate_derived",
    "dissolve_charter",
    "expel_member",
    "induct_member",
    "ratify_charter",
    "dissent_boundary",
    "record_observation",
    "reaffirm_member",
    "release_guardian",
    "resign_envoy",
    "resign_office",
    "declare_relay_endpoint",
    "withdraw_relay_endpoint",
    "amend_content_policy",
    "content_policy_state",
    "decide_compact_act",
    "compact_state",
    "COMPACT_ACT_KINDS",
    "revoke_envoy",
    "revoke_office",
    "rotate_guardian",
]
