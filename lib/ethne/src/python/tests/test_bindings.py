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

"""The bindings themselves: keys, content addressing, the log, JSON round-trip.

The charter logic underneath is covered by `cargo test --workspace`; these tests
check that the Python surface loads and that its plumbing is faithful.
"""
from __future__ import annotations

import json

from dataclasses import asdict

import pytest

import ethne as e
from ethne import _core
from ethne.authoring import Signer, record_observation


def test_core_version_and_alg_constant():
    assert isinstance(_core.__version__, str)
    assert _core.ALG_ED25519 == "Ed25519"


def test_generate_keypair_shape():
    priv, pub, did = e.generate_keypair()
    assert isinstance(priv, bytes) and len(priv) == 32
    assert isinstance(pub, bytes) and len(pub) == 32
    assert did.startswith("did:key:z6Mk")


def test_generate_keypair_is_unique():
    assert e.generate_keypair()[2] != e.generate_keypair()[2]


def test_signer_generate_and_repr_hides_key():
    s = Signer.generate()
    assert s.did.startswith("did:key:")
    assert len(s.priv) == 32
    assert s.priv.hex() not in repr(s)
    assert s.did in repr(s)


def test_cid_of_is_deterministic_and_prefixed():
    first = e.cid_of(b"substrate")
    assert first == e.cid_of(b"substrate")
    assert first != e.cid_of(b"Substrate")
    assert first.startswith("b3:")


def test_majority_helper():
    assert [e.majority(n) for n in (1, 2, 3, 4, 5)] == [1, 2, 2, 3, 3]


def test_empty_archive():
    arch = e.Archive()
    assert len(arch) == 0
    assert arch.cids() == []
    assert arch.entries() == []
    assert arch.authority_violations() == []
    arch.verify_all()
    arch.check_authority()


def test_entries_report_cid_and_kind(polity):
    entries = polity.archive.entries()
    assert entries == [(polity.archive.cids()[0], "PolityChartered")]
    assert polity.archive.contains(entries[0][0])


def test_dump_json_round_trip_preserves_cids(polity):
    dumped = polity.archive.dump_json()
    parsed = json.loads(dumped)
    # A versioned envelope, so the format can move without silently misreading.
    # Version 2 added the generation and start position (D20).
    assert parsed["version"] == 2
    assert parsed["generation"] == 0 and parsed["start"] == 0
    assert len(parsed["entries"]) == 1 and "PolityChartered" in parsed["entries"][0]

    restored = e.Archive.load_json(dumped)
    assert len(restored) == len(polity.archive)
    assert restored.cids() == polity.archive.cids()
    restored.verify_all()
    restored.check_authority()


def test_load_json_rejects_garbage():
    with pytest.raises(ValueError):
        e.Archive.load_json("{not json}")


def test_bad_did_is_a_value_error():
    arch = e.Archive()
    with pytest.raises(ValueError):
        arch.charter_state("not-a-did")


def test_mismatched_signer_lists_are_a_value_error(polity):
    with pytest.raises(ValueError, match="same length"):
        polity.archive.expel_member(
            polity.did, polity.founders[2].did, [polity.founders[0].did], []
        )


def test_default_thresholds_exposes_the_rust_values():
    assert set(e.DEFAULT_THRESHOLDS) == {
        "reputation_floor",
        "attrition_notice",
        "attrition_concern",
        "mean_reputation_notice",
        "mean_reputation_concern",
        "breach_notice",
        "breach_concern",
        "attendance_cadence_seconds",
        "unattended_notice",
        "unrated_notice",
        "affirmation_cadence_seconds",
        "affirmation_stale_notice",
        "affirmation_stale_concern",
        "guardian_lost_notice",
        "guardian_lost_concern",
        "churn_window_seconds",
        "churn_notice",
        "churn_concern",
        "sustained_absence_seconds",
        "sustained_absence_readings",
    }
    # The dataclass defaults are those same values, not a second copy.
    assert e.Thresholds().attrition_concern == e.DEFAULT_THRESHOLDS["attrition_concern"]
    # Every dataclass field must be a real keyword on the Rust side: monitor.py
    # splats asdict(thresholds), so a name that drifts raises TypeError at the
    # first override rather than being silently ignored.
    assert set(asdict(e.Thresholds())) == set(e.DEFAULT_THRESHOLDS)


def test_default_retention_exposes_the_rust_values(polity):
    assert set(e.DEFAULT_RETENTION) == {
        "retain_seconds",
        "min_dropped",
        "live_measure_seconds",
        "min_witness_readings",
    }
    # The dataclass defaults are those same values, not a second copy.
    assert e.RetentionPolicy().retain_seconds == e.DEFAULT_RETENTION["retain_seconds"]
    assert set(asdict(e.RetentionPolicy())) == set(e.DEFAULT_RETENTION)
    # And every field is a real keyword on the Rust side, so a host can splat a
    # policy straight through. A drifted name would raise TypeError here.
    assert polity.archive.prune_recommendation(
        polity.did, **asdict(e.RetentionPolicy()), as_of=100
    ) is None


def test_the_envoy_surface_is_exported():
    # The read-model class and every authoring/uplift entry point for D14.
    assert e.EnvoyState is _core.EnvoyState
    assert e.ENVOY_KINDS == ("Authorize", "Resign", "Revoke")
    for name in ("authorize_envoy", "resign_envoy", "revoke_envoy", "envoy_delegation"):
        assert name in e.__all__ and callable(getattr(e, name))


def test_envoy_state_of_an_empty_log_is_empty(polity):
    st = polity.archive.envoy_state(polity.did)
    assert st.standings == [] and st.envoys() == []
    assert not st.is_active(polity.founders[0].did)


def test_the_guardian_surface_is_exported():
    assert e.GuardianState is _core.GuardianState
    assert e.GUARDIAN_KINDS == ("Designate", "Rotate", "Release")
    for name in ("designate_derived", "rotate_guardian", "release_guardian"):
        assert name in e.__all__ and callable(getattr(e, name))


def test_guardian_state_of_a_wholly_human_polity_is_empty(polity):
    st = polity.archive.guardian_state(polity.did)
    assert st.derivations == []
    assert not st.is_derived(polity.founders[0].did)
    assert st.guardian_of(polity.founders[0].did) is None
    assert not st.is_machine_majority(3)


def test_the_trend_and_observation_surface_is_exported():
    assert e.DecayTrends is _core.DecayTrends
    assert e.ObservationWindow is _core.ObservationWindow
    assert "record_observation" in e.__all__ and callable(e.record_observation)


def test_an_empty_log_has_no_trends_and_no_observations(polity):
    t = polity.archive.decay_trends(polity.did)
    assert t.affirmations == [] and t.gaps_seconds == []
    assert t.seconds_since_last is None
    # Not enough series to compare: unknown, which is not the same as "no".
    assert t.latest_gap_widening() is None
    assert t.witness_count_declined() is False

    w = polity.archive.observation_window(polity.did, since=0)
    assert w.reading_count() == 0
    assert w.span_seconds() is None
    assert w.unattended_span(polity.founders[0].did) is None


def test_load_json_still_reads_a_bare_array(polity):
    """The shape earlier builds emitted. Only the envelope is ever written now, but
    a dump taken then must still load."""
    entries = json.loads(polity.archive.dump_json())["entries"]
    restored = e.Archive.load_json(json.dumps(entries))
    assert restored.cids() == polity.archive.cids()


def test_incremental_dump_carries_only_what_is_new(polity):
    """A host persisting a growing log appends what is new rather than rewriting
    the whole archive each time — which is what the observation log needs."""
    monitor = Signer.generate()
    cursor = polity.archive.cursor()
    record_observation(
        polity.archive, polity.did, monitor,
        [(polity.founders[0].did, 0.9, None, None)],
    )
    segment = polity.archive.dump_json_since(cursor)
    assert len(json.loads(segment)["entries"]) == 1

    replica = e.Archive.load_json(polity.archive.dump_json())
    assert replica.cids() == polity.archive.cids()

    # Fold the segment onto a replica that stopped at the cursor.
    behind = e.Archive.load_json(
        json.dumps({"version": 2, "generation": 0, "start": 0,
                    "entries": json.loads(polity.archive.dump_json())["entries"][:cursor.index]})
    )
    assert behind.append_json(segment) == 1
    assert behind.cids() == polity.archive.cids()

    # A cursor already at the end costs nothing, and replaying raises.
    assert behind.append_json(polity.archive.dump_json_since(polity.archive.cursor())) == 0
    with pytest.raises(ValueError):
        behind.append_json(segment)
