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

"""The autonomous_trust substrate surface (**D33**)."""
from __future__ import annotations

import pytest

from ethne import _core
from ethne.at import AT_FFI, AtProvider, at_ffi_available, did_from_pubkey
from ethne.polity import generate_keypair

NOW = 1_721_800_000


def key():
    """A fresh keypair as (priv, pub, did)."""
    return generate_keypair()


# --- the blocker: a DID for an observed peer's key ----------------------------------


def test_did_from_pubkey_agrees_with_the_generator():
    """Until D33 there was no route from a raw public key to a DID, so a host holding an
    observed AT peer's signing key could not name it. The DID this returns must be the
    one the generator reported for the same key, or the roll names somebody else."""
    _priv, pub, did = key()
    assert did_from_pubkey(pub) == did


def test_did_from_pubkey_refuses_a_key_that_is_not_one():
    with pytest.raises(Exception):
        did_from_pubkey(b"\x01\x02\x03")


# --- THE REGRESSION TEST for the measured divergence -------------------------------


def test_an_unverified_operator_yields_no_guardian_and_no_attendance():
    """**The defect this slice was built to make unrepresentable.**

    AT sends `operator_bound=False` for a node whose operator credential never verified,
    and it can still carry a guardian key and an attendance stamp on the wire. Reading
    those through without the gate reports an *unguarded* machine as guarded and
    attended: it defeats the office rule D15 exists for, invents D8's human-above-machine
    anchor from a key AT declined to vouch for, and hands the cohesion monitor a
    freshness stamp with nothing behind it. Measured against a hand-rolled Python fold,
    this was 2 divergences on one peer, both toward health, with nothing in the output
    looking wrong.
    """
    at = AtProvider()
    _p, node, node_did = key()
    _gp, guardian, _gd = key()
    at.observe_peer(
        node,
        reputation=0.66,
        operator_bound=False,          # AT never verified the credential ...
        guardian_pubkey=guardian,      # ... but both of these are on the wire
        operator_attested_at=NOW,
    )
    at.refresh()

    members = {m[0]: m for m in at.founding_candidate().members}
    did, reputation, guardian_did, attested_at, derived = members[node_did]
    assert reputation == 0.66, "the reading itself still crosses"
    assert guardian_did is None, "no verified operator, no guardian edge"
    assert attested_at is None, "a stamp on an unverified node attests to nothing"
    assert derived is True, "an AT peer is a machine party either way"


def test_a_verified_operator_yields_both():
    """The other direction. Without this, the rule above could be "refuse everything"
    and the guardian edge would never cross at all."""
    at = AtProvider()
    _p, node, node_did = key()
    _gp, guardian, guardian_did = key()
    at.observe_peer(
        node,
        reputation=0.83,
        operator_bound=True,
        guardian_pubkey=guardian,
        operator_attested_at=NOW,
    )
    at.refresh()

    members = {m[0]: m for m in at.founding_candidate().members}
    _did, reputation, got_guardian, attested_at, derived = members[node_did]
    assert reputation == 0.83
    assert got_guardian == guardian_did
    assert attested_at == NOW
    assert derived is True


def test_operator_bound_without_a_stamp_keeps_the_guardian_edge():
    """The two signals are independent: verified hardware with nobody at the console
    keeps its guardian and simply reports no attendance. AT sends exactly this."""
    at = AtProvider()
    _p, node, node_did = key()
    _gp, guardian, guardian_did = key()
    at.observe_peer(node, reputation=0.8, operator_bound=True, guardian_pubkey=guardian)
    at.refresh()

    members = {m[0]: m for m in at.founding_candidate().members}
    assert members[node_did][2] == guardian_did
    assert members[node_did][3] is None


# --- the fold ----------------------------------------------------------------------


def test_the_roll_folds_upserts_departures_and_orders_by_did():
    at = AtProvider()
    _a, k_a, did_a = key()
    _b, k_b, did_b = key()
    _c, k_c, did_c = key()

    at.observe_peer(k_a, reputation=0.9)
    at.observe_peer(k_b, reputation=0.6)
    at.observe_peer(k_c, reputation=0.5)
    at.observe_peer(k_b, reputation=0.4)   # an upsert, not a second member
    at.depart_peer(k_c)
    assert at.refresh() == 5

    assert at.observed_len() == 2
    assert len(at) == 2
    dids = [m[0] for m in at.founding_candidate().members]
    assert dids == sorted(dids), "deterministic order regardless of arrival"
    assert set(dids) == {did_a, did_b}
    assert at.member_reputation(did_b) == 0.4, "the later reading wins"
    assert at.member_reputation(did_c) is None, "a departed peer has no reading"


def test_an_unrated_peer_is_unrated_not_rated_low():
    at = AtProvider()
    _p, k, did = key()
    at.observe_peer(k)                     # no reputation at all
    at.refresh()
    assert at.member_reputation(did) is None
    assert at.founding_candidate().members[0][1] is None


def test_a_private_boundary_is_recorded_and_is_not_incompleteness():
    at = AtProvider()
    _p, k, _did = key()
    assert at.has_private_boundary() is False
    at.boundary_private()
    at.observe_peer(k, reputation=0.9)
    at.refresh()
    assert at.has_private_boundary() is True
    assert at.observed_len() == 1, "the visible member is still in the roll"


def test_a_substrate_can_be_articulated_for_the_cluster():
    at = AtProvider(substrate_name="Test Polity", substrate_statement="a cohering cluster")
    candidate = at.founding_candidate()
    assert candidate.substrate_name == "Test Polity"
    assert candidate.substrate_statement == "a cohering cluster"
    assert candidate.substrate_anchor is None
    # A pure emergent cluster has none, which is what Q4 requires it to resolve before
    # it can charter.
    assert AtProvider().founding_candidate().substrate_name is None


def test_a_substrate_needs_both_a_name_and_a_statement():
    with pytest.raises(ValueError):
        AtProvider(substrate_name="half a substrate")


# --- the change stream (D16) -------------------------------------------------------


def test_appearance_then_movements_then_silence():
    at = AtProvider()
    _p, node, node_did = key()
    _gp, guardian, guardian_did = key()

    at.observe_peer(node, reputation=0.8, operator_bound=True,
                    guardian_pubkey=guardian, operator_attested_at=NOW)
    changes = at.changes()
    kinds = [c.kind for c in changes]
    assert kinds == ["MemberAppeared", "AttendanceStamped"]
    appeared = changes[0]
    assert appeared.member == node_did
    assert appeared.to_reputation == 0.8
    assert appeared.guardian == guardian_did
    assert appeared.derived is True
    assert changes[1].attested_at == NOW
    assert "MemberAppeared" in repr(appeared)

    # Draining is the contract: the same change is not reported twice.
    assert at.changes() == []

    # A reputation move and the guardian edge going away are separate movements.
    at.observe_peer(node, reputation=0.4, operator_bound=False)
    changes = at.changes()
    assert [c.kind for c in changes] == ["ReputationChanged", "GuardianEdgeChanged"]
    assert (changes[0].from_reputation, changes[0].to_reputation) == (0.8, 0.4)
    assert changes[1].guardian is None

    # An unchanged re-observation is silence — a substrate that keeps saying the same
    # thing must not fill an observation log.
    at.observe_peer(node, reputation=0.4, operator_bound=False)
    assert at.changes() == []


def test_only_an_advancing_attendance_stamp_is_a_re_attestation():
    at = AtProvider()
    _p, node, _did = key()
    _gp, guardian, _gd = key()

    def observe(stamp):
        at.observe_peer(node, reputation=0.8, operator_bound=True,
                        guardian_pubkey=guardian, operator_attested_at=stamp)

    observe(NOW)
    assert [c.kind for c in at.changes()] == ["MemberAppeared", "AttendanceStamped"]

    observe(NOW + 600)
    changes = at.changes()
    assert [c.kind for c in changes] == ["AttendanceStamped"]
    assert changes[0].attested_at == NOW + 600

    # The same stamp again is not a re-attestation — exactly the case a count of
    # readings must not mistake for presence.
    observe(NOW + 600)
    assert at.changes() == []


def test_the_boundary_going_opaque_is_reported_once():
    at = AtProvider()
    at.boundary_private()
    changes = at.changes()
    assert [c.kind for c in changes] == ["BoundaryOpacityChanged"]
    assert changes[0].opaque is True
    assert changes[0].member is None
    at.boundary_private()
    assert at.changes() == [], "an opaque edge does not re-open"


def test_a_departure_is_a_vanishing():
    at = AtProvider()
    _p, k, did = key()
    at.observe_peer(k, reputation=0.9)
    at.changes()
    at.depart_peer(k)
    changes = at.changes()
    assert [c.kind for c in changes] == ["MemberVanished"]
    assert changes[0].member == did
    # A second departure for somebody already gone is not a change.
    at.depart_peer(k)
    assert at.changes() == []


# --- the at-ffi gate ---------------------------------------------------------------


def test_the_gate_reports_which_build_this_is():
    assert at_ffi_available() is AT_FFI
    assert isinstance(AT_FFI, bool)


@pytest.mark.skipif(AT_FFI, reason="this is an at-ffi build")
def test_without_at_ffi_the_live_couplings_are_absent_not_failing():
    """The answer to the question this item carried: a caller sees the live constructors
    **absent**, and a fact it can branch on. A binding that is present and can only ever
    fail is the dead-field pattern D23 and KC's `source_commitment` both turned out to
    be."""
    assert AT_FFI is False
    for name in ("attached", "attached_existing", "embedded",
                 "request_roster", "held_reputations", "alive", "ready",
                 "wait_ready", "pid"):
        assert not hasattr(AtProvider, name), f"{name} must not exist without at-ffi"
    assert not hasattr(_core, "AtLogLevel")
    # And the replayed-feed provider is entirely usable in this build, which is why most
    # callers never need to ask.
    at = AtProvider()
    _p, k, _did = key()
    at.observe_peer(k, reputation=0.5)
    at.refresh()
    assert at.observed_len() == 1


@pytest.mark.skipif(not AT_FFI, reason="needs an at-ffi build")
def test_with_at_ffi_the_live_couplings_exist():
    assert AT_FFI is True
    for name in ("attached", "attached_existing", "embedded"):
        assert hasattr(AtProvider, name)
    # A replayed-feed provider under this build refuses the daemon-only methods, naming
    # what it is rather than what it lacks.
    at = AtProvider()
    with pytest.raises(RuntimeError, match="daemon-backed"):
        at.held_reputations()
