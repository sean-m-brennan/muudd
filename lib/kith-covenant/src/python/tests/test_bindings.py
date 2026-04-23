"""Pytest tests for the kith_covenant._core PyO3 bindings.

These exercise the Python surface — the underlying Rust logic is covered by
`cargo test --workspace`. The goal here is: (1) ensure the bindings compile
and load, (2) confirm the `kith_covenant.ceremony` wrappers behave
identically to calling `_core` directly, (3) confirm the AT seam
(`interface.py`, `policy.py`) still imports.
"""

from __future__ import annotations

import json

import pytest

from kith_covenant import (
    Archive,
    CeremonyResult,
    cid_of,
    generate_keypair,
    run_two_community_ceremony,
)
from kith_covenant import _core


ALPHA_MOVEMENTS = ["a1", "a2", "a3", "a4", "a5"]
BETA_MOVEMENTS = ["b1", "b2", "b3", "b4", "b5"]


def test_core_version_and_alg_constant():
    assert isinstance(_core.__version__, str)
    assert _core.ALG_ED25519 == "Ed25519"


def test_generate_keypair_shape():
    priv, pub, did = generate_keypair()
    assert isinstance(priv, bytes) and len(priv) == 32
    assert isinstance(pub, bytes) and len(pub) == 32
    assert isinstance(did, str) and did.startswith("did:key:z6Mk")


def test_generate_keypair_unique():
    _, _, did1 = generate_keypair()
    _, _, did2 = generate_keypair()
    assert did1 != did2


def test_cid_of_bytes_is_deterministic():
    a = cid_of(b"hello")
    b = cid_of(b"hello")
    c = cid_of(b"goodbye")
    assert a == b
    assert a != c
    assert a.startswith("b3:") and len(a) == 3 + 64


def test_ceremony_succeeds_and_verifies():
    r = run_two_community_ceremony("Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS)
    assert isinstance(r, CeremonyResult)
    assert r.alpha_did.startswith("did:key:z6Mk")
    assert r.beta_did.startswith("did:key:z6Mk")
    assert r.alpha_did != r.beta_did
    assert r.witness_cid.startswith("b3:")
    assert r.archive_size == 11
    assert r.compact_label == "two-community-compact"


def test_ceremony_archive_json_reloads_and_verifies():
    r = run_two_community_ceremony("Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS)
    arch = Archive.load_json(r.archive_json)
    assert len(arch) == r.archive_size
    arch.verify_all()  # must not raise

    # Re-dump and confirm it's valid JSON and still loads.
    redumped = arch.dump_json()
    parsed = json.loads(redumped)
    assert parsed["version"] == 1
    assert len(parsed["entries"]) == r.archive_size

    arch2 = Archive.load_json(redumped)
    arch2.verify_all()


def test_ceremony_archive_entries_kinds():
    r = run_two_community_ceremony("Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS)
    arch = Archive.load_json(r.archive_json)
    kinds = [k for (k, _) in arch.entries()]
    # 2 identities + 4 delegations + 2 witness registries + 2 intra rites + 1 inter witness = 11
    assert kinds.count("CommunityIdentityPublished") == 2
    assert kinds.count("SpeakerDelegated") == 4
    assert kinds.count("WitnessRegistryUpdated") == 2
    assert kinds.count("IntraCommunityRite") == 2
    assert kinds.count("InterCommunityWitness") == 1


def test_ceremony_rejects_wrong_movement_count():
    with pytest.raises(ValueError, match="alpha_movements"):
        run_two_community_ceremony("A", ["x"], "B", BETA_MOVEMENTS)
    with pytest.raises(ValueError, match="beta_movements"):
        run_two_community_ceremony("A", ALPHA_MOVEMENTS, "B", ["x"])


def test_tampered_archive_json_fails_verify():
    r = run_two_community_ceremony("Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS)
    data = json.loads(r.archive_json)
    # Corrupt the first signature byte in the first entry.
    first = data["entries"][0]
    # Each entry variant is a singleton-key map; dig into its value.
    (_variant_name, inner), = first.items()
    inner["signatures"][0]["signature"][0] ^= 0x01
    corrupted = json.dumps(data)
    arch = Archive.load_json(corrupted)
    with pytest.raises(RuntimeError, match="signature"):
        arch.verify_all()


def test_archive_len_and_cids_consistent():
    r = run_two_community_ceremony("Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS)
    arch = Archive.load_json(r.archive_json)
    assert len(arch) == len(arch.cids()) == len(arch.entries())


def test_at_seam_still_imports():
    # Confirms refactor didn't break the AT integration surface.
    from kith_covenant.interface import PeerAdmitted
    from kith_covenant.policy import KithCovenantPolicy, strict_policy

    policy = strict_policy()
    assert isinstance(policy, KithCovenantPolicy)
    assert policy.max_group_size == 10
