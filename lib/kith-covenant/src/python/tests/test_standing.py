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

"""Standing derivation (Probationary/Anathema) and the authoring primitives
that were previously buildable only inside the Rust ceremonies. These exercise
the ceremony-cost-decay and rejection-pattern lenses with REAL records, closing
the binding-surface-only gap from the v0 jester-lens slice.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, new_identity, vouch, designate_anathema, lift_designation,
    record_non_admission, run_intra_rite, now_seconds, reputation_of,
    rejection_pattern, ceremony_cost_decay, run_two_community_ceremony,
    delegate_speaker, revoke_delegation, record_concern,
)

DAY = 86400
MOV = ["m1", "m2", "m3", "m4", "m5"]


# --- Anathema ---------------------------------------------------------------

def test_designation_drives_anathema_and_lift_reverts():
    arch = Archive()
    quorum = new_identity(arch, "Quorum")
    subject = new_identity(arch, "Subject")
    cid = designate_anathema(arch, quorum, subject.did, narrative="hostis")
    arch.verify_all()
    assert reputation_of(arch, subject.did).standing == "Anathema"

    lift_designation(arch, quorum, subject.did, cid, narrative="restored")
    arch.verify_all()
    # Back to identity-only standing once the designation is lifted.
    assert reputation_of(arch, subject.did).standing == "Identified"


def test_anathema_overrides_admission():
    result = run_two_community_ceremony("Alpha", MOV, "Beta", MOV)
    arch = Archive.load_json(result.archive_json)
    assert reputation_of(arch, result.alpha_did).standing == "InCompact"
    quorum = new_identity(arch, "Quorum")
    designate_anathema(arch, quorum, result.alpha_did)
    arch.verify_all()
    assert reputation_of(arch, result.alpha_did).standing == "Anathema"


# --- Probationary -----------------------------------------------------------

def test_admitted_and_sponsored_in_window_is_probationary():
    result = run_two_community_ceremony("Alpha", MOV, "Beta", MOV)
    arch = Archive.load_json(result.archive_json)
    hub = new_identity(arch, "Hub")
    now = now_seconds()
    vouch(arch, hub, result.alpha_did,
          probation_until=now + 300 * DAY, sponsored_at=now - DAY)
    arch.verify_all()
    assert reputation_of(arch, result.alpha_did).standing == "Probationary"


def test_probation_elapsed_is_plain_incompact():
    result = run_two_community_ceremony("Alpha", MOV, "Beta", MOV)
    arch = Archive.load_json(result.archive_json)
    hub = new_identity(arch, "Hub")
    now = now_seconds()
    # Probation window already closed.
    vouch(arch, hub, result.alpha_did,
          probation_until=now - DAY, sponsored_at=now - 2 * DAY)
    arch.verify_all()
    assert reputation_of(arch, result.alpha_did).standing == "InCompact"


# --- Ceremony-cost-decay lens with REAL declining rites ---------------------

def test_ceremony_cost_decay_with_real_declining_rites():
    arch = Archive()
    comm = new_identity(arch, "Reaffirmer")
    # Three re-affirmations: witnesses 5 -> 3 -> 1; gaps 100 then 200 (widening).
    run_intra_rite(arch, comm, witness_count=5, sealed_at=1000)
    run_intra_rite(arch, comm, witness_count=3, sealed_at=1100)
    run_intra_rite(arch, comm, witness_count=1, sealed_at=1300)
    arch.verify_all()

    d = ceremony_cost_decay(arch, comm.did)
    assert d.rite_count == 3
    assert d.first_witness_count == 5
    assert d.last_witness_count == 1
    assert d.witness_count_declined is True
    assert d.gaps_seconds == (100, 200)
    assert d.latest_gap_widening is True
    assert d.seconds_since_last is not None and d.seconds_since_last > 0


def test_run_intra_rite_rejects_zero_witnesses():
    arch = Archive()
    comm = new_identity(arch, "C")
    with pytest.raises((ValueError, RuntimeError)):
        run_intra_rite(arch, comm, witness_count=0, sealed_at=1000)


# --- Rejection-pattern lens with REAL non-admission records -----------------

def test_rejection_pattern_with_real_non_admissions():
    arch = Archive()
    a = new_identity(arch, "A")   # rejector
    b = new_identity(arch, "B")   # presenting / rejected twice
    c = new_identity(arch, "C")   # presenting
    d = new_identity(arch, "D")   # rejector
    # A rejects B and C on FallenActor; D rejects B on EnemyLove (later).
    record_non_admission(arch, a, [a.did, b.did],
                         [(a.did, b.did, "FallenActorAnthropology", "no")],
                         decided_at=1000)
    record_non_admission(arch, a, [a.did, c.did],
                         [(a.did, c.did, "FallenActorAnthropology", "no")],
                         decided_at=1100)
    record_non_admission(arch, d, [d.did, b.did],
                         [(d.did, b.did, "EnemyLoveOperational", "no")],
                         decided_at=1200)
    arch.verify_all()

    p = rejection_pattern(arch)
    assert p.total_rejections == 3
    by_rejector = dict(p.by_rejector)
    assert by_rejector[a.did] == 2
    assert by_rejector[d.did] == 1
    by_presenting = dict(p.by_presenting)
    assert by_presenting[b.did] == 2
    assert by_presenting[c.did] == 1
    by_feature = dict(p.by_feature)
    assert by_feature["FallenActorAnthropology"] == 2
    assert by_feature["EnemyLoveOperational"] == 1
    # Concentration ranking heads with the busiest rejector.
    assert p.by_concentration[0][0] == a.did
    assert len(p.perspectives) == 2


def test_rejection_pattern_respects_as_of_with_real_records():
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    record_non_admission(arch, a, [a.did, b.did],
                         [(a.did, b.did, "FallenActorAnthropology", "no")],
                         decided_at=1000)
    record_non_admission(arch, a, [a.did, b.did],
                         [(a.did, b.did, "EnemyLoveOperational", "no")],
                         decided_at=2000)
    arch.verify_all()
    p = rejection_pattern(arch, as_of=1500)
    assert p.total_rejections == 1
    assert dict(p.by_feature).get("EnemyLoveOperational") is None


# --- Delegation revocation --------------------------------------------------

def test_revocation_stops_verification_and_clears_active_delegation():
    arch = Archive()
    comm = new_identity(arch, "Comm")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    delegate = delegate_speaker(arch, comm, "Reciter", issued_at=now - 100)

    # A record the delegate signs before revocation verifies, and the
    # delegation shows as active.
    record_concern(arch, delegate, subject.did,
                   feature="NonUltimacyOfCompact", recorded_at=now - 50)
    arch.verify_all()
    view = reputation_of(arch, comm.did)
    assert any(d.subject_did == delegate.did for d in view.active_delegations)

    # Revoke effective now; the delegation drops out of active_delegations.
    revoke_delegation(arch, comm, delegate.did, issued_at=now, revoked_at=now)
    view2 = reputation_of(arch, comm.did)
    assert not any(d.subject_did == delegate.did for d in view2.active_delegations)

    # The pre-revocation record still verifies on its own (non-retroactive)...
    arch.verify_all()

    # ...but a record the delegate signs AFTER revocation fails verification.
    record_concern(arch, delegate, subject.did,
                   feature="EnemyLoveOperational", recorded_at=now + 100)
    with pytest.raises(RuntimeError):
        arch.verify_all()


def test_community_can_delegate_again_after_revocation():
    # Same-key re-grant reactivation is covered authoritatively by the Rust
    # test `appended_revocation_supersedes_grant_in_active_delegations`. Here we
    # show a revocation does not poison the community: it can delegate a fresh
    # key that is active.
    arch = Archive()
    comm = new_identity(arch, "Comm")
    now = now_seconds()
    d1 = delegate_speaker(arch, comm, "Reciter", issued_at=now - 100)
    revoke_delegation(arch, comm, d1.did, issued_at=now - 10, revoked_at=now - 10)
    assert not reputation_of(arch, comm.did).active_delegations

    d2 = delegate_speaker(arch, comm, "Reciter", issued_at=now - 5)
    view = reputation_of(arch, comm.did)
    assert len(view.active_delegations) == 1
    assert view.active_delegations[0].subject_did == d2.did
