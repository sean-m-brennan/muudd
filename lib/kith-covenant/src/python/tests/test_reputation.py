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

"""Pytest suite for the kith_covenant reputation-lookup surface."""
from __future__ import annotations

import pytest

from kith_covenant import (
    Archive, ReputationView, generate_keypair, reputation_of,
    run_two_community_ceremony, rejection_pattern, ceremony_cost_decay,
    interpretive_plurality, new_identity, attest_interpretation,
    attest_interpreter_independence, now_seconds,
    structured_skepticism, report_event, corroborate_event, invoke_event_action,
    find_undisclosed_interest,
    adversarial_principal, attest_principal, record_principal_output,
)


ALPHA_MOVEMENTS = ["a1", "a2", "a3", "a4", "a5"]
BETA_MOVEMENTS = ["b1", "b2", "b3", "b4", "b5"]


@pytest.fixture
def ceremony_archive():
    """Run a fresh two-community ceremony and return (archive, alpha_did, beta_did)."""
    result = run_two_community_ceremony(
        "Alpha", ALPHA_MOVEMENTS, "Beta", BETA_MOVEMENTS,
        compact_label="test-compact",
    )
    archive = Archive.load_json(result.archive_json)
    return archive, result.alpha_did, result.beta_did


def test_reputation_of_alpha_in_compact(ceremony_archive):
    archive, alpha, beta = ceremony_archive
    view = reputation_of(archive, alpha)
    assert isinstance(view, ReputationView)
    assert view.community_did == alpha
    assert view.standing == "InCompact"
    assert len(view.intra_rites) == 1
    assert len(view.inter_admissions) == 1
    assert view.inter_admissions[0].counterparties == (beta,)
    assert not view.inter_nonadmissions


def test_reputation_of_beta_in_compact(ceremony_archive):
    archive, alpha, beta = ceremony_archive
    view = reputation_of(archive, beta)
    assert view.standing == "InCompact"
    assert view.inter_admissions[0].counterparties == (alpha,)


def test_unknown_for_unrelated_did(ceremony_archive):
    archive, _, _ = ceremony_archive
    _, _, stranger_did = generate_keypair()
    view = reputation_of(archive, stranger_did)
    assert view.standing == "Unknown"
    assert not view.intra_rites
    assert not view.inter_admissions
    assert not view.inter_nonadmissions
    assert view.identity_cid is None


def test_view_is_stable_through_json_roundtrip(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    before = reputation_of(archive, alpha)
    reloaded = Archive.load_json(archive.dump_json())
    after = reputation_of(reloaded, alpha)
    assert before == after


def test_as_of_before_genesis_returns_unknown(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    view = reputation_of(archive, alpha, as_of=0)
    assert view.standing == "Unknown"


def test_invalid_did_raises_value_error(ceremony_archive):
    archive, _, _ = ceremony_archive
    with pytest.raises(ValueError):
        reputation_of(archive, "not-a-did")


# --- v0 jester lenses (binding surface + degenerate paths) ------------------
# The rich multi-rejection / declining-witness paths are covered exhaustively
# by the Rust unit tests; the Python-reachable archive (a single two-community
# ceremony) exercises the binding surface and the empty/single-point paths, and
# cross-checks the ceremony-cost lens against the ReputationView projection.


def test_rejection_pattern_inert_on_admission_only_archive(ceremony_archive):
    archive, _, _ = ceremony_archive
    p = rejection_pattern(archive)
    assert p.total_rejections == 0
    assert p.events == ()
    assert p.by_rejector == ()
    assert p.by_presenting == ()
    assert p.by_feature == ()
    assert p.by_concentration == ()
    assert p.perspectives == ()


def test_ceremony_cost_decay_single_rite_matches_projection(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    d = ceremony_cost_decay(archive, alpha)
    assert d.rite_count == 1
    assert d.gaps_seconds == ()
    assert d.witness_count_declined is False
    assert d.latest_gap_widening is None
    assert d.seconds_since_last is not None and d.seconds_since_last >= 0
    assert d.first_witness_count == d.last_witness_count
    # The lens and the ReputationView must agree on the same rite.
    view = reputation_of(archive, alpha)
    assert len(view.intra_rites) == 1
    assert d.rites[0][0] == view.intra_rites[0].record_cid
    assert d.rites[0][2] == view.intra_rites[0].witness_count


def test_ceremony_cost_decay_empty_for_stranger(ceremony_archive):
    archive, _, _ = ceremony_archive
    _, _, stranger = generate_keypair()
    d = ceremony_cost_decay(archive, stranger)
    assert d.rite_count == 0
    assert d.rites == ()
    assert d.gaps_seconds == ()
    assert d.first_witness_count is None
    assert d.latest_gap_widening is None
    assert d.seconds_since_last is None


def test_ceremony_cost_decay_as_of_before_genesis_is_empty(ceremony_archive):
    archive, alpha, _ = ceremony_archive
    d = ceremony_cost_decay(archive, alpha, as_of=0)
    assert d.rite_count == 0


def test_jester_lens_invalid_did_raises(ceremony_archive):
    archive, _, _ = ceremony_archive
    with pytest.raises(ValueError):
        ceremony_cost_decay(archive, "not-a-did")


# --- move 1 interpretive-plurality lens (binding surface) -------------------


def test_interpretive_plurality_flags_monopoly():
    arch = Archive()
    t0 = now_seconds()
    member = new_identity(arch, "Member", t0)
    faction = new_identity(arch, "Faction", t0)
    for i in range(3):
        attest_interpretation(arch, member, "canonical", faction.did, recorded_at=t0 + i)
    r = interpretive_plurality(arch, member.did, as_of=t0 + 10)
    assert r.distinct_lineages == 1
    assert r.distinct_interpreters == 1
    assert r.plurality_demonstrated is False
    assert r.interpreter_concentration[0] == (faction.did, 3)
    assert len(r.interpretations) == 3


def test_interpretive_plurality_clears_a_plural_community():
    arch = Archive()
    t0 = now_seconds()
    member = new_identity(arch, "Member", t0)
    i1 = new_identity(arch, "Interp-1", t0)
    i2 = new_identity(arch, "Interp-2", t0)
    attest_interpretation(arch, member, "canonical", i1.did, recorded_at=t0 + 1)
    attest_interpretation(arch, member, "reformist", i2.did, recorded_at=t0 + 2)
    r = interpretive_plurality(arch, member.did, as_of=t0 + 10)
    assert r.distinct_lineages == 2
    assert r.distinct_interpreters == 2
    assert r.plurality_demonstrated is True
    # Declared plurality alone does not prove independence (anti-sockpuppet).
    assert r.distinct_independent_interpreters == 0
    assert r.independent_plurality_demonstrated is False


def test_third_party_attestation_establishes_independent_plurality():
    arch = Archive()
    t0 = now_seconds()
    member = new_identity(arch, "Member", t0)
    i1 = new_identity(arch, "Interp-1", t0)
    i2 = new_identity(arch, "Interp-2", t0)
    witness = new_identity(arch, "Witness", t0)
    attest_interpretation(arch, member, "canonical", i1.did, recorded_at=t0 + 1)
    attest_interpretation(arch, member, "reformist", i2.did, recorded_at=t0 + 2)
    attest_interpreter_independence(arch, witness, member.did, i1.did, attested_at=t0 + 3)
    attest_interpreter_independence(arch, witness, member.did, i2.did, attested_at=t0 + 4)
    r = interpretive_plurality(arch, member.did, as_of=t0 + 10)
    assert r.distinct_independent_interpreters == 2
    assert r.independent_plurality_demonstrated is True
    keyed = dict(r.independent_attesters)
    assert keyed[i1.did] == (witness.did,)


def test_community_self_attestation_does_not_prove_independence():
    arch = Archive()
    t0 = now_seconds()
    member = new_identity(arch, "Member", t0)
    i1 = new_identity(arch, "Interp-1", t0)
    i2 = new_identity(arch, "Interp-2", t0)
    attest_interpretation(arch, member, "canonical", i1.did, recorded_at=t0 + 1)
    attest_interpretation(arch, member, "reformist", i2.did, recorded_at=t0 + 2)
    # The community vouching for its own minted identities is not independence.
    attest_interpreter_independence(arch, member, member.did, i1.did, attested_at=t0 + 3)
    attest_interpreter_independence(arch, member, member.did, i2.did, attested_at=t0 + 4)
    r = interpretive_plurality(arch, member.did, as_of=t0 + 10)
    assert r.distinct_independent_interpreters == 0
    assert r.independent_plurality_demonstrated is False


def test_interpretive_plurality_empty_for_stranger():
    arch = Archive()
    _, _, stranger = generate_keypair()
    r = interpretive_plurality(arch, stranger)
    assert r.interpretations == ()
    assert r.distinct_lineages == 0
    assert r.plurality_demonstrated is False


def test_interpretive_plurality_invalid_did_raises():
    arch = Archive()
    with pytest.raises(ValueError):
        interpretive_plurality(arch, "not-a-did")


# --- move 4 structured-skepticism lens (binding surface) --------------------


def _manufactured(arch, originator, at):
    return report_event(
        arch, originator, "a sudden, convenient omen",
        high_interpretive_charge=True, sudden_onset=True,
        originator_holds_stake=True, aligned_faction=originator.did,
        reported_at=at,
    )


def test_structured_skepticism_flags_uncorroborated_manufactured_event():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    _manufactured(arch, a, t0 + 100)
    r = structured_skepticism(arch, as_of=t0 + 1000)
    assert r.qualifying_events == 1
    assert r.events[0].corroborators == ()
    assert r.events[0].provenance_complete is False
    assert len(r.under_corroborated(2)) == 1


def test_structured_skepticism_clears_corroborated_event():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    c = new_identity(arch, "Community-C", t0)
    ev = _manufactured(arch, a, t0 + 100)
    # Self-corroboration and a duplicate ally do not count.
    corroborate_event(arch, a, ev, corroborated_at=t0 + 120)
    corroborate_event(arch, b, ev, corroborated_at=t0 + 200)
    corroborate_event(arch, b, ev, corroborated_at=t0 + 250)
    assert len(structured_skepticism(arch, as_of=t0 + 1000).under_corroborated(2)) == 1
    # A second distinct community clears it.
    corroborate_event(arch, c, ev, corroborated_at=t0 + 300)
    cleared = structured_skepticism(arch, as_of=t0 + 1000)
    assert cleared.events[0].corroborators == (b.did, c.did) or set(
        cleared.events[0].corroborators
    ) == {b.did, c.did}
    assert cleared.under_corroborated(2) == ()


def test_structured_skepticism_inert_on_empty_archive():
    arch = Archive()
    r = structured_skepticism(arch)
    assert r.events == ()
    assert r.qualifying_events == 0
    assert r.under_corroborated(2) == ()


def test_native_framing_corroboration_is_under_translated():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    c = new_identity(arch, "Community-C", t0)
    ev = _manufactured(arch, a, t0 + 100)
    # Corroborated by two distinct communities, but only in the native framing.
    corroborate_event(arch, b, ev, corroborated_at=t0 + 200)
    corroborate_event(arch, c, ev, corroborated_at=t0 + 300)
    r = structured_skepticism(arch, as_of=t0 + 1000)
    assert len(r.events[0].corroborators) == 2
    assert r.events[0].translated_corroborators == ()
    assert r.under_corroborated(2) == ()
    # The translation signal still flags it.
    assert len(r.under_translated(2)) == 1


def test_cross_substrate_translation_clears_the_translation_signal():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    c = new_identity(arch, "Community-C", t0)
    ev = _manufactured(arch, a, t0 + 100)
    corroborate_event(arch, b, ev, translation="reads as a boundary rite", corroborated_at=t0 + 200)
    corroborate_event(arch, c, ev, translation="reads as a debt ledger", corroborated_at=t0 + 300)
    r = structured_skepticism(arch, as_of=t0 + 1000)
    assert len(r.events[0].translated_corroborators) == 2
    assert r.under_translated(2) == ()


def test_retroactive_finding_impugns_the_event():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    ev = _manufactured(arch, a, t0 + 100)
    find_undisclosed_interest(arch, b, ev, "owns the disputed claim", found_at=t0 + 500)
    r = structured_skepticism(arch, as_of=t0 + 1000)
    found = r.events[0].undisclosed_interests_found
    assert found == ((b.did, "owns the disputed claim"),)
    assert r.retroactively_impugned() == (ev,)


def test_originator_self_finding_is_not_bad_faith():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    ev = _manufactured(arch, a, t0 + 100)
    # The originator disclosing its own omission late is honest, not bad faith.
    find_undisclosed_interest(arch, a, ev, "a late self-disclosure", found_at=t0 + 500)
    r = structured_skepticism(arch, as_of=t0 + 1000)
    assert r.events[0].undisclosed_interests_found == ()
    assert r.retroactively_impugned() == ()


# --- move 4 structured-skepticism hold enforcement (invoke_event_action) ----

_HOLD = 7 * 24 * 3600  # DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS


def test_action_on_qualifying_event_within_hold_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    ev = _manufactured(arch, a, t0 + 100)
    # Acted on immediately, no corroboration — the hold refuses it.
    invoke_event_action(arch, a, ev, "escalate to the pool", invoked_at=t0 + 200)
    assert any("structured-skepticism hold" in r
               for _cid, r in arch.authority_violations())


def test_action_after_hold_elapses_is_authorized():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    ev = _manufactured(arch, a, t0 + 100)
    invoke_event_action(arch, a, ev, "escalate", invoked_at=t0 + 100 + _HOLD + 1)
    assert not any("structured-skepticism" in r
                   for _cid, r in arch.authority_violations())


def test_corroboration_releases_the_hold_early():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    b = new_identity(arch, "Community-B", t0)
    c = new_identity(arch, "Community-C", t0)
    ev = _manufactured(arch, a, t0 + 100)
    corroborate_event(arch, b, ev, corroborated_at=t0 + 200)
    corroborate_event(arch, c, ev, corroborated_at=t0 + 300)
    # Still inside the hold, but two distinct communities corroborated.
    invoke_event_action(arch, a, ev, "escalate", invoked_at=t0 + 400)
    assert not any("structured-skepticism" in r
                   for _cid, r in arch.authority_violations())


def test_action_on_non_qualifying_event_is_unconstrained():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Faction-A", t0)
    # No qualifying triggers → structured skepticism does not apply.
    ev = report_event(arch, a, "a routine notice", reported_at=t0 + 100)
    invoke_event_action(arch, a, ev, "note", invoked_at=t0 + 200)
    assert not any("structured-skepticism" in r
                   for _cid, r in arch.authority_violations())


# --- move 6 adversarial-principal lens (binding surface) --------------------


def _cohort(arch, t0):
    """A home + captured principal, plus three cohort peers answering 'admit'."""
    home = new_identity(arch, "Home", t0)
    captured = new_identity(arch, "Captured", t0)
    for i in range(3):
        ph = new_identity(arch, f"PeerHome-{i}", t0)
        pp = new_identity(arch, f"Peer-{i}", t0)
        record_principal_output(
            arch, ph, pp.did, "admissions-review", "M1", "admit",
            favors_home=False, produced_at=t0 + i,
        )
    return home, captured


def test_adversarial_principal_flags_self_benefiting_divergence():
    arch = Archive()
    t0 = now_seconds()
    home, captured = _cohort(arch, t0)
    record_principal_output(
        arch, home, captured.did, "admissions-review", "M1", "reject",
        favors_home=True, produced_at=t0 + 100,
    )
    r = adversarial_principal(arch, captured.did, as_of=t0 + 1000)
    assert r.self_benefiting_divergences == 1
    d = r.divergences[0]
    assert d.principal_decision == "reject"
    assert d.cohort_majority == "admit"
    assert d.peers_compared == 3
    assert d.favors_home is True


def test_adversarial_principal_cross_witness_excludes_self_and_home():
    arch = Archive()
    t0 = now_seconds()
    home, captured = _cohort(arch, t0)
    attest_principal(arch, home, captured.did, home.did, attested_at=t0 + 100)
    assert adversarial_principal(arch, captured.did, as_of=t0 + 1000).distinct_cross_witnesses == 0
    outsider = new_identity(arch, "Outsider", t0)
    attest_principal(arch, outsider, captured.did, home.did, attested_at=t0 + 200)
    assert adversarial_principal(arch, captured.did, as_of=t0 + 1000).distinct_cross_witnesses == 1


def test_adversarial_principal_inert_for_stranger():
    arch = Archive()
    _, _, stranger = generate_keypair()
    r = adversarial_principal(arch, stranger)
    assert r.cross_witnesses == ()
    assert r.divergences == ()
    assert r.self_benefiting_divergences == 0
    assert r.outputs_triangulated == 0
    assert r.self_dealing_rate == 0.0


def _matter_round(arch, home, captured, matter, own, favors_home, base_t):
    """Three peers answer 'admit' on `matter`; the principal answers `own`."""
    for i in range(3):
        ph = new_identity(arch, f"PH-{matter}-{i}", base_t)
        pp = new_identity(arch, f"PP-{matter}-{i}", base_t)
        record_principal_output(
            arch, ph, pp.did, "cohort", matter, "admit",
            favors_home=False, produced_at=base_t + i,
        )
    record_principal_output(
        arch, home, captured.did, "cohort", matter, own,
        favors_home=favors_home, produced_at=base_t + 10,
    )


def test_self_dealing_rate_aggregates_continuous_behavior():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    captured = new_identity(arch, "Captured", t0)
    # Self-deals on 3 of 4 triangulated matters.
    _matter_round(arch, home, captured, "M1", "reject", True, t0 + 100)
    _matter_round(arch, home, captured, "M2", "reject", True, t0 + 200)
    _matter_round(arch, home, captured, "M3", "reject", True, t0 + 300)
    _matter_round(arch, home, captured, "M4", "admit", False, t0 + 400)
    r = adversarial_principal(arch, captured.did, as_of=t0 + 1000)
    assert r.outputs_triangulated == 4
    assert r.self_benefiting_divergences == 3
    assert abs(r.self_dealing_rate - 0.75) < 1e-9
    assert r.systematic_self_dealing(4, 0.5) is True
    assert r.systematic_self_dealing(4, 0.8) is False


def test_small_sample_is_not_systematic():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    captured = new_identity(arch, "Captured", t0)
    _matter_round(arch, home, captured, "M1", "reject", True, t0 + 100)
    r = adversarial_principal(arch, captured.did, as_of=t0 + 1000)
    assert abs(r.self_dealing_rate - 1.0) < 1e-9
    # Rate is 1.0 but a single output is too little to call systematic.
    assert r.systematic_self_dealing(3, 0.5) is False
