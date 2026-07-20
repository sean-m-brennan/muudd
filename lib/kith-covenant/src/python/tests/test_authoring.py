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

"""Pytest suite for the Python authoring surface.

Builds a live archive from scratch through `authoring` (no hardcoded Rust
`demo_capture_scenario`), crypto-verifies it, and proves the same three
capture signals fire — i.e. an operator with real keys can produce the demo
scenario, not just replay it.
"""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, Signer, new_identity, delegate_speaker, vouch, record_conduct,
    record_concern, now_seconds, reputation_of, composition_graph,
    sponsor_stake, feature_concern_profile,
)

# Same offsets as the Rust demo_capture_scenario (see kc-pyo3 lib.rs).
DAY = 86400


def build_authored_scenario():
    """Reproduce demo_capture_scenario purely through the authoring API:
    a hub sponsors two entrants, files two anti-spiral Concerns against one of
    them (in probation), and commends the other. Returns (archive, dids)."""
    arch = Archive()
    hub = new_identity(arch, "Hub")
    drifting = new_identity(arch, "Drifting")
    healthy = new_identity(arch, "Healthy")

    now = now_seconds()
    probation_until = now + 300 * DAY
    sponsored_at = now - 30 * DAY

    for entrant in (drifting, healthy):
        vouch(arch, hub, entrant.did, probation_until=probation_until,
              sponsored_at=sponsored_at, statement="vouched by the hub")

    recorded_at = now - DAY
    for feature in ("NonUltimacyOfCompact", "EnemyLoveOperational"):
        record_concern(
            arch, hub, drifting.did, feature=feature,
            perspective="charter-originalist observer",
            narrative=f"{feature}: decided a matter its charter reserves",
            recorded_at=recorded_at, observed_at=recorded_at,
        )
    record_conduct(
        arch, hub, healthy.did, "Commendation", "AntiSpiral",
        feature="GraceOverMerit",
        narrative="restored a lapsed member rather than expelling",
        recorded_at=recorded_at, observed_at=recorded_at,
    )
    return arch, hub, drifting, healthy


@pytest.fixture
def authored():
    arch, hub, drifting, healthy = build_authored_scenario()
    arch.verify_all()  # authored records must crypto-verify against published ids
    return arch, hub, drifting, healthy


def test_new_identity_returns_signer_and_is_published(authored):
    arch, hub, _drifting, _healthy = authored
    assert isinstance(hub, Signer)
    assert hub.did.startswith("did:key:")
    assert isinstance(hub.priv, bytes) and isinstance(hub.pub, bytes)
    # 3 identities + 2 sponsorships + 3 conduct records
    assert len(arch) == 8


def test_signer_repr_does_not_leak_key_material(authored):
    _arch, hub, _drifting, _healthy = authored
    text = repr(hub)
    assert "redacted" in text
    assert hub.priv.hex() not in text


def test_authored_archive_verifies(authored):
    arch, _hub, _drifting, _healthy = authored
    arch.verify_all()  # idempotent; raises on any bad signature / unresolved id


def test_conduct_projects_onto_drifting_entrant(authored):
    arch, _hub, drifting, _healthy = authored
    view = reputation_of(arch, drifting.did)
    assert len(view.conduct) == 2
    assert {c.kind for c in view.conduct} == {"Concern"}
    assert all(c.ladder_rung is None for c in view.conduct)
    assert {c.bearing for c in view.conduct} == {
        "AntiSpiral(NonUltimacyOfCompact)",
        "AntiSpiral(EnemyLoveOperational)",
    }


def test_healthy_entrant_has_only_commendation(authored):
    arch, _hub, _drifting, healthy = authored
    view = reputation_of(arch, healthy.did)
    assert len(view.conduct) == 1
    assert view.conduct[0].kind == "Commendation"


def test_sponsorships_project_to_both_endpoints(authored):
    arch, hub, drifting, _healthy = authored
    hub_view = reputation_of(arch, hub.did)
    assert len(hub_view.sponsorships_given) == 2
    assert not hub_view.sponsorships_received

    entrant = reputation_of(arch, drifting.did)
    assert len(entrant.sponsorships_received) == 1
    assert entrant.sponsorships_received[0].sponsor_community == hub.did


def test_composition_graph_flags_hub_concentration(authored):
    arch, hub, _drifting, _healthy = authored
    graph = composition_graph(arch)
    assert len(graph.edges) == 2
    top_sponsor, out_degree = graph.by_concentration[0]
    assert top_sponsor == hub.did
    assert out_degree == 2


def test_sponsor_stake_flags_tainted_sponsee(authored):
    arch, hub, drifting, _healthy = authored
    report = sponsor_stake(arch, hub.did)
    assert report.total_sponsees == 2
    assert report.tainted_sponsees == 1
    assert report.taint_rate == pytest.approx(0.5)
    tainted = [o for o in report.sponsees if o.adverse_conduct]
    assert len(tainted) == 1
    assert tainted[0].entrant == drifting.did
    assert len(tainted[0].adverse_conduct) == 2


def test_feature_concern_profile_matches_demo(authored):
    arch, _hub, drifting, _healthy = authored
    profile = feature_concern_profile(arch, drifting.did)
    per_feature = dict(profile.per_feature)
    assert per_feature.get("NonUltimacyOfCompact") == 1
    assert per_feature.get("EnemyLoveOperational") == 1
    assert profile.non_feature_adverse == 0
    assert len(profile.evidence) == 2
    assert profile.perspectives == ("charter-originalist observer",)


def test_record_conduct_accepts_reporter_quorum(authored):
    """Second-class kinds may carry a multi-community reporter quorum; the
    multi-sig envelope must verify with every reporter's key."""
    arch, hub, drifting, healthy = authored
    cid = record_conduct(
        arch, [hub, healthy], drifting.did, "Violation", "CommonsObligation",
        severity="Serious", decision_class="SecondClass",
        ladder_rung="RegisteredProtest",
        narrative="quorum-attested commons breach",
    )
    assert cid
    arch.verify_all()
    view = reputation_of(arch, drifting.did)
    violations = [c for c in view.conduct if c.kind == "Violation"]
    assert len(violations) == 1
    assert violations[0].ladder_rung == "RegisteredProtest"


def test_unpublished_signer_fails_verification():
    """A record signed by a Signer whose identity was never published must fail
    verify_all — authoring does not silently trust unknown keys."""
    from kith_covenant import generate_keypair
    arch = Archive()
    hub = new_identity(arch, "Hub")
    # A ghost sponsor: real keys, but its identity is never published.
    priv, pub, did = generate_keypair()
    ghost = Signer(did=did, priv=priv, pub=pub)
    now = now_seconds()
    vouch(arch, ghost, hub.did, probation_until=now + 100 * DAY)
    with pytest.raises(Exception):
        arch.verify_all()


def test_bad_enum_value_is_rejected(authored):
    arch, hub, drifting, _healthy = authored
    with pytest.raises(Exception):
        record_conduct(arch, hub, drifting.did, "NotAKind", "AntiSpiral",
                       feature="GraceOverMerit")
    with pytest.raises(Exception):
        # AntiSpiral bearing requires a feature.
        record_conduct(arch, hub, drifting.did, "Concern", "AntiSpiral")


# ---- Delegation-based signing ---------------------------------------------


def test_signer_principal_defaults_to_did():
    """A root identity speaks for itself: principal == did."""
    arch = Archive()
    hub = new_identity(arch, "Hub")
    assert hub.principal == hub.did


def test_delegate_speaker_returns_delegate_signer():
    arch = Archive()
    hub = new_identity(arch, "Hub")
    speaker = delegate_speaker(arch, hub, "Envoy")
    assert isinstance(speaker, Signer)
    assert speaker.did != hub.did          # a distinct signing key
    assert speaker.principal == hub.did     # speaks for the hub
    assert speaker.priv != hub.priv
    # The SpeakerDelegation is now an archive entry (2 = identity + delegation).
    assert len(arch) == 2
    arch.verify_all()


def test_delegate_signs_concern_on_behalf_of_community():
    """A Concern signed by a delegate key verifies through the delegation chain
    and still projects onto the community, not the delegate key."""
    arch = Archive()
    hub = new_identity(arch, "Hub")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    speaker = delegate_speaker(arch, hub, "Envoy", issued_at=now - 10 * DAY)
    record_concern(arch, speaker, subject.did, feature="NonUltimacyOfCompact",
                   perspective="charter-originalist observer",
                   recorded_at=now - DAY, observed_at=now - DAY)
    arch.verify_all()

    view = reputation_of(arch, subject.did)
    assert len(view.conduct) == 1
    # Attribution is the community (hub), never the speaker key.
    assert list(view.conduct[0].reporters) == [hub.did]
    assert speaker.did not in view.conduct[0].reporters


def test_delegated_scenario_reproduces_capture_signals():
    """Full parity, but every record is signed by a delegate speaker rather than
    the hub root key. The three capture signals are identical."""
    arch = Archive()
    hub = new_identity(arch, "Hub")
    drifting = new_identity(arch, "Drifting")
    healthy = new_identity(arch, "Healthy")
    now = now_seconds()
    # Delegation must be active at the earliest record (sponsored_at = now-30d).
    speaker = delegate_speaker(arch, hub, "Envoy", issued_at=now - 60 * DAY)

    probation_until = now + 300 * DAY
    sponsored_at = now - 30 * DAY
    for entrant in (drifting, healthy):
        vouch(arch, speaker, entrant.did, probation_until=probation_until,
              sponsored_at=sponsored_at, statement="vouched by the hub")
    recorded_at = now - DAY
    for feature in ("NonUltimacyOfCompact", "EnemyLoveOperational"):
        record_concern(arch, speaker, drifting.did, feature=feature,
                       perspective="charter-originalist observer",
                       recorded_at=recorded_at, observed_at=recorded_at)
    arch.verify_all()

    graph = composition_graph(arch)
    assert graph.by_concentration[0] == (hub.did, 2)
    stake = sponsor_stake(arch, hub.did)
    assert stake.total_sponsees == 2
    assert stake.tainted_sponsees == 1
    assert stake.taint_rate == pytest.approx(0.5)
    profile = feature_concern_profile(arch, drifting.did)
    assert dict(profile.per_feature) == {
        "NonUltimacyOfCompact": 1, "EnemyLoveOperational": 1,
    }
    assert profile.perspectives == ("charter-originalist observer",)


def test_expired_delegation_fails_verification():
    arch = Archive()
    hub = new_identity(arch, "Hub")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    # Delegation window closed a day before the record was signed.
    speaker = delegate_speaker(arch, hub, "Envoy",
                               issued_at=now - 10 * DAY, expires_at=now - 2 * DAY)
    record_concern(arch, speaker, subject.did, feature="NonUltimacyOfCompact",
                   recorded_at=now - DAY, observed_at=now - DAY)
    with pytest.raises(Exception):
        arch.verify_all()


def test_not_yet_issued_delegation_fails_verification():
    arch = Archive()
    hub = new_identity(arch, "Hub")
    subject = new_identity(arch, "Subject")
    now = now_seconds()
    # Delegation is issued a day AFTER the record it purports to authorize.
    speaker = delegate_speaker(arch, hub, "Envoy", issued_at=now + DAY)
    record_concern(arch, speaker, subject.did, feature="NonUltimacyOfCompact",
                   recorded_at=now - DAY, observed_at=now - DAY)
    with pytest.raises(Exception):
        arch.verify_all()


def test_bad_delegation_scope_is_rejected():
    arch = Archive()
    hub = new_identity(arch, "Hub")
    with pytest.raises(Exception):
        delegate_speaker(arch, hub, "NotAScope")
