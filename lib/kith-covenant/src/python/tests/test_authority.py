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

"""§5 authority enforcement: check_authority is a pass distinct from the
cryptographic verify_all. It enforces principal-binding, scope-appropriateness,
and a distinct-community quorum for second-class decisions.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, Signer, new_identity, delegate_speaker, record_concern,
    designate_anathema, now_seconds, run_two_community_ceremony, run_intra_rite,
)

MOV = ["m1", "m2", "m3", "m4", "m5"]


def test_real_ceremony_archive_is_authorized():
    result = run_two_community_ceremony("Alpha", MOV, "Beta", MOV)
    arch = Archive.load_json(result.archive_json)
    arch.verify_all()
    arch.check_authority()  # must not raise


def test_envoy_delegate_may_sign_conduct():
    arch = Archive()
    comm = new_identity(arch, "Comm")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    envoy = delegate_speaker(arch, comm, "Envoy", issued_at=now - 100)
    record_concern(arch, envoy, subject.did,
                   feature="NonUltimacyOfCompact", recorded_at=now)
    arch.verify_all()
    arch.check_authority()  # Envoy scope authorizes a conduct record


def test_wrong_scope_delegate_is_unauthorized():
    arch = Archive()
    comm = new_identity(arch, "Comm")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    reciter = delegate_speaker(arch, comm, "Reciter", issued_at=now - 100)
    record_concern(arch, reciter, subject.did,
                   feature="NonUltimacyOfCompact", recorded_at=now)
    arch.verify_all()  # crypto is fine
    with pytest.raises(RuntimeError):
        arch.check_authority()  # a Reciter cannot sign conduct


def test_cross_community_delegate_is_unauthorized():
    arch = Archive()
    x = new_identity(arch, "X")
    y = new_identity(arch, "Y")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    envoy = delegate_speaker(arch, x, "Envoy", issued_at=now - 100)
    # Forge attribution to Y while signing with X's envoy key.
    rogue = Signer(did=envoy.did, priv=envoy.priv, pub=envoy.pub, principal=y.did)
    record_concern(arch, rogue, subject.did,
                   feature="NonUltimacyOfCompact", recorded_at=now)
    arch.verify_all()  # crypto is fine: the envoy key resolves
    with pytest.raises(RuntimeError):
        arch.check_authority()  # X's envoy has no authority to speak for Y


def test_second_class_single_community_fails_quorum():
    arch = Archive()
    a = new_identity(arch, "A")
    subject = new_identity(arch, "Subject")
    designate_anathema(arch, a, subject.did)  # SecondClass, one community
    arch.verify_all()
    with pytest.raises(RuntimeError):
        arch.check_authority()
    violations = arch.authority_violations()
    assert len(violations) == 1
    assert "second-class" in violations[0][1]


def test_second_class_two_community_quorum_passes():
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    subject = new_identity(arch, "Subject")
    designate_anathema(arch, [a, b], subject.did)  # two-community quorum
    arch.verify_all()
    arch.check_authority()  # quorum satisfied


def test_quorum_threshold_is_configurable():
    arch = Archive()
    a = new_identity(arch, "A")
    b = new_identity(arch, "B")
    subject = new_identity(arch, "Subject")
    designate_anathema(arch, [a, b], subject.did)
    arch.check_authority(quorum_threshold=2)  # ok
    with pytest.raises(RuntimeError):
        arch.check_authority(quorum_threshold=3)  # needs 3 distinct communities


# --- §1 the rite is all five movements ----------------------------------------


def test_a_blank_movement_is_refused_by_the_facade():
    """The facade checked that five movements were supplied and never that any of
    them said anything, so a caller could recite nothing and seal it."""
    arch = Archive()
    comm = new_identity(arch, "Comm")
    with pytest.raises(ValueError, match="blank"):
        run_intra_rite(arch, comm, movements=["m1", "m2", "m3", "   ", "m5"],
                       sealed_at=now_seconds())


def test_a_complete_rite_still_seals():
    arch = Archive()
    comm = new_identity(arch, "Comm")
    cid = run_intra_rite(arch, comm, movements=MOV, sealed_at=now_seconds())
    assert cid
    arch.verify_all()
    arch.check_authority()
