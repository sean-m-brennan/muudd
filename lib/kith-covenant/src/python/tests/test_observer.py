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

"""§8 observer / jester corps through the Python surface. Exhaustive rule-by-rule
coverage lives in the Rust kc-archive authority and kc-reputation projection
tests; these confirm the bindings and facades wire the whole chain (author ->
verify_all -> check_authority -> projection/dashboard query)."""
import pytest

from kith_covenant import (
    Archive, new_identity, new_mediator, new_observer, register_mediators,
    register_observers, publish_observer_report, respond_to_report,
    delegate_speaker, insider_ring, attest_insider, file_whistleblower,
    open_source_share, reveal_source, exculpate_source, reputation_of,
    observer_dashboard, observer_pool_concentration, now_seconds,
    new_member, publish_member_roll, insider_set_size, Signer,
)

_DAY = 86400


def _mediator_pool(arch, names, t0):
    out = []
    for name in names:
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10)
        out.append(m)
    return out


def _fixture():
    arch = Archive()
    t0 = now_seconds()
    subject = new_identity(arch, "Subject", t0)
    home = new_identity(arch, "Home", t0)
    obs = new_observer()
    # A bounded term (100 days) — the §8 term limit refuses open-ended standing.
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0 - 10, valid_until=t0 + 100 * 86400)
    return arch, t0, subject, obs


def _with_insiders(arch, subject, t0, n=2):
    """Give the accused community `n` delegated speakers, so its resolvable key set
    is wide enough to hide a whistleblower in."""
    return [delegate_speaker(arch, subject, "Reciter", issued_at=t0 - 10)
            for _ in range(n)]


def test_observer_report_and_response_project_onto_subject():
    arch, t0, subject, obs = _fixture()
    report = publish_observer_report(arch, obs, subject.did, "labor-rights",
                                     summary="authoritarian rota", reported_at=t0 + 100)
    respond_to_report(arch, subject, report, "by consent", responded_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert len(v.observer_reports) == 1
    assert v.observer_reports[0].declared_perspective == "labor-rights"
    assert len(v.observer_reports[0].responses) == 1


def test_observer_reporting_on_own_community_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    obs = new_observer()
    register_observers(arch, home, [(obs, "internal")],
                       valid_from=t0 - 10, valid_until=t0 + 100 * 86400)
    publish_observer_report(arch, obs, home.did, "internal",
                            summary="self", reported_at=t0 + 100)
    assert any("non-overlap" in r or "own community" in r
               for _cid, r in arch.authority_violations())


def test_only_subject_may_respond():
    arch, t0, subject, obs = _fixture()
    other = new_identity(arch, "Other", t0)
    report = publish_observer_report(arch, obs, subject.did, "labor-rights",
                                     summary="x", reported_at=t0 + 100)
    respond_to_report(arch, other, report, "we object", responded_at=t0 + 200)
    with pytest.raises(RuntimeError):
        arch.check_authority()


def test_whistleblower_projects_without_source_and_reveal_flips_flag():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, "coerced labor",
                            reception_panel=panel, source=crew[0],
                            threshold=2, filed_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert len(v.whistleblower_reports_against) == 1
    assert v.whistleblower_reports_against[0].source_revealed is False
    # No source field is exposed anywhere on the summary.
    assert not hasattr(v.whistleblower_reports_against[0], "disclosed_source")
    # An anonymous tag, though: the same source filing twice is countable as one.
    assert v.whistleblower_reports_against[0].source_tag

    reveal_source(arch, wb, panel, cause="recanted",
                  disclosed_source=crew[0].did, revealed_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert v.whistleblower_reports_against[0].source_revealed is True


def test_a_filing_needs_a_real_insider_behind_it():
    # The manufactured filing: a reception panel cannot ring-sign as an insider of
    # the community it is accusing, so it cannot invent a source.
    arch, t0, subject, _obs = _fixture()
    _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    with pytest.raises(ValueError, match="not an insider"):
        attest_insider(arch, subject.did, panel[0], "invented", filed_at=t0 + 100)


def test_a_community_with_one_key_has_no_anonymous_channel():
    # A ring of one names its signer; the API refuses rather than pretend.
    arch, t0, subject, _obs = _fixture()
    assert len(insider_ring(arch, subject.did, at=t0 + 100)) == 1
    with pytest.raises(ValueError):
        attest_insider(arch, subject.did, subject, "claim", filed_at=t0 + 100)


def test_insider_ring_widens_with_delegations():
    arch, t0, subject, _obs = _fixture()
    _with_insiders(arch, subject, t0, n=3)
    ring = insider_ring(arch, subject.did, at=t0 + 100)
    # Root plus three delegated speakers.
    assert len(ring) == 4


def test_a_mediator_needs_an_encryption_key_distinct_from_its_signing_key():
    # §8 custody seals a share of a source's identity to the encryption key by key
    # agreement. new_mediator() mints both; a bare Signer cannot be seated.
    arch, t0, _subject, _obs = _fixture()
    c = new_identity(arch, "Nominator", t0)
    m = new_mediator()
    assert m.enc_pub and m.enc_pub != m.pub
    bare = Signer(did=m.did, priv=m.priv, pub=m.pub)
    with pytest.raises(ValueError, match="encryption key"):
        register_mediators(arch, c, [bare], valid_from=t0 - 10)


def test_registering_the_signing_key_as_the_encryption_key_is_refused():
    # Through the raw binding, since the facade will not build it: reusing one key for
    # two primitives is what the field exists to end.
    arch, t0, _subject, _obs = _fixture()
    c = new_identity(arch, "Nominator", t0)
    m = new_mediator()
    arch.register_mediators(c.priv, c.did, [(m.pub, m.did, m.pub)], t0 - 10, None)
    with pytest.raises(RuntimeError, match="signing key as its encryption key"):
        arch.check_authority()


def test_a_share_is_opened_with_the_encryption_key_not_the_signing_key():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    att = attest_insider(arch, subject.did, crew[0], "coerced labor", filed_at=t0 + 100)
    wb = file_whistleblower(arch, subject.did, "coerced labor", panel,
                            attestation=att, threshold=2)
    arch.verify_all()
    arch.check_authority()
    # The real custodian opens its own share.
    assert open_source_share(arch, wb, panel[0])
    # The same identity holding only its signing key cannot.
    signing_only = Signer(did=panel[0].did, priv=panel[0].priv, pub=panel[0].pub)
    with pytest.raises(ValueError, match="encryption key"):
        open_source_share(arch, wb, signing_only)


def test_a_member_roll_gives_a_community_an_anonymous_channel():
    # The polity-tier feed: without a roll this community's channel is one key deep
    # and it cannot file at all (see the test above).
    arch, t0, subject, _obs = _fixture()
    assert insider_set_size(arch, subject.did, at=t0 + 100) == 1
    members = [new_member() for _ in range(4)]
    publish_member_roll(arch, subject, members, as_of=t0 - 10)
    assert insider_set_size(arch, subject.did, at=t0 + 100) == 5
    arch.verify_all()
    arch.check_authority()


def test_a_rolled_member_with_no_delegation_can_file_anonymously():
    arch, t0, subject, _obs = _fixture()
    members = [new_member() for _ in range(4)]
    publish_member_roll(arch, subject, members, as_of=t0 - 10)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    # The source holds no delegation of any kind — only a place on the roll.
    att = attest_insider(arch, subject.did, members[2], "coerced labor",
                         filed_at=t0 + 100)
    file_whistleblower(arch, subject.did, "coerced labor", panel,
                       attestation=att, filed_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()


def test_a_shorter_later_roll_removes_nobody():
    # Union-only: a community cannot shrink the suspect set around whoever just left.
    arch, t0, subject, _obs = _fixture()
    members = [new_member() for _ in range(4)]
    publish_member_roll(arch, subject, members, as_of=t0 - 10)
    assert insider_set_size(arch, subject.did, at=t0 + 100) == 5
    publish_member_roll(arch, subject, members[:1], as_of=t0 - 5)
    assert insider_set_size(arch, subject.did, at=t0 + 100) == 5


def test_a_roll_cannot_give_a_member_a_key_its_did_does_not_resolve_to():
    # Reached through the raw binding: the `publish_member_roll` facade takes Signers
    # and so always pairs a member with its own key. Root-only publication is the
    # same story — the facade signs as the community it names, which makes
    # "an envoy publishes a roll" unrepresentable here; the Rust authority tests
    # cover that rule directly.
    arch, t0, subject, _obs = _fixture()
    member = new_member()
    outsider = new_member()
    arch.publish_member_roll(
        subject.priv, subject.did,
        [(outsider.pub, member.did)],   # a real member's DID, an outsider's key
        t0 - 10, None,
    )
    with pytest.raises(RuntimeError, match="does not resolve"):
        arch.check_authority()


def test_an_empty_member_roll_is_refused_before_it_is_authored():
    arch, t0, subject, _obs = _fixture()
    with pytest.raises(ValueError, match="at least one member"):
        publish_member_roll(arch, subject, [], as_of=t0)


def test_the_dashboard_shows_the_thinnest_anonymity_sets_first():
    arch, t0, subject, _obs = _fixture()
    members = [new_member() for _ in range(3)]
    publish_member_roll(arch, subject, members, as_of=t0 - 10)
    dash = observer_dashboard(arch, t0 + 100)
    sizes = dict(dash.anonymity_set_by_community)
    assert sizes[subject.did] == 4
    # Ascending: whichever community has no channel at all is the first row.
    counts = [n for _did, n in dash.anonymity_set_by_community]
    assert counts == sorted(counts)


def test_a_reveal_cannot_name_someone_the_shares_do_not_recover():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, "coerced labor",
                            reception_panel=panel, source=crew[0],
                            threshold=2, filed_at=t0 + 100)
    # A real quorum, a real cause — and the wrong crew member.
    reveal_source(arch, wb, panel, cause="for cause",
                  disclosed_source=crew[1].did, revealed_at=t0 + 200)
    assert any("recover" in r or "commitment" in r
               for _cid, r in arch.authority_violations())


def test_a_reveal_below_the_custody_threshold_is_unauthorized():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, "coerced labor",
                            reception_panel=panel, source=crew[0],
                            threshold=3, filed_at=t0 + 100)
    two = [open_source_share(arch, wb, panel[0]), open_source_share(arch, wb, panel[1])]
    reveal_source(arch, wb, panel, cause="for cause",
                  disclosed_source=crew[0].did, shares=two, revealed_at=t0 + 200)
    assert arch.authority_violations()


def test_a_wrongly_named_member_can_clear_itself_and_the_source_cannot():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, "coerced labor",
                            reception_panel=panel, source=crew[0],
                            threshold=2, filed_at=t0 + 100)
    # The innocent delegate clears itself, with no panel involved.
    exculpate_source(arch, wb, crew[1], exculpated_at=t0 + 300)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert v.whistleblower_reports_against[0].exculpations == 1

    # The true source's honest proof reproduces the filing's image: a confession.
    exculpate_source(arch, wb, crew[0], exculpated_at=t0 + 400)
    assert any("confession" in r for _cid, r in arch.authority_violations())


def test_the_dashboard_separates_one_source_from_many():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0, n=3)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    # One insider files three times.
    for i in range(3):
        file_whistleblower(arch, subject.did, f"claim {i}",
                           reception_panel=panel, source=crew[0],
                           threshold=2, filed_at=t0 + 100 + i)
    arch.verify_all()
    arch.check_authority()
    dash = observer_dashboard(arch, as_of=t0 + 500)
    assert dict(dash.whistleblowers_by_accused)[subject.did] == 3
    assert dict(dash.distinct_sources_by_accused)[subject.did] == 1


def test_whistleblower_reception_panel_from_accused_is_unauthorized():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B"), t0)
    own = new_mediator()
    register_mediators(arch, subject, [own], valid_from=t0 - 10)
    file_whistleblower(arch, subject.did, "c",
                       reception_panel=[panel[0], panel[1], own], source=crew[0],
                       threshold=2, filed_at=t0 + 100)
    assert any("non-overlap" in r or "accused" in r
               for _cid, r in arch.authority_violations())


def test_reveal_without_cause_rejected_at_facade():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, "c", reception_panel=panel,
                            source=crew[0], threshold=2, filed_at=t0 + 100)
    with pytest.raises(ValueError):
        reveal_source(arch, wb, panel, cause="   ", disclosed_source="x")


def test_filing_needs_exactly_one_of_attestation_or_source():
    arch, t0, subject, _obs = _fixture()
    crew = _with_insiders(arch, subject, t0)
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    with pytest.raises(ValueError, match="exactly one"):
        file_whistleblower(arch, subject.did, "c", reception_panel=panel,
                           filed_at=t0 + 100)
    packet = attest_insider(arch, subject.did, crew[0], "c", filed_at=t0 + 100)
    with pytest.raises(ValueError, match="exactly one"):
        file_whistleblower(arch, subject.did, "c", reception_panel=panel,
                           attestation=packet, source=crew[0])


def test_observer_dashboard_tallies():
    arch, t0, subject, obs = _fixture()
    s2 = new_identity(arch, "Subject2", t0)
    publish_observer_report(arch, obs, subject.did, "labor-rights", summary="a", reported_at=t0 + 100)
    publish_observer_report(arch, obs, subject.did, "labor-rights", summary="b", reported_at=t0 + 110)
    publish_observer_report(arch, obs, s2.did, "labor-rights", summary="c", reported_at=t0 + 120)
    arch.verify_all()
    arch.check_authority()
    dash = observer_dashboard(arch, as_of=t0 + 500)
    assert dash.observer_reports_by_subject[0] == (subject.did, 2)


# --- §8 observer-corps caps (pool-share + term + rotation) -------------------

def test_open_ended_observer_term_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    register_observers(arch, home, [(new_observer(), "labor-rights")], valid_from=t0)
    assert any("open-ended" in r or "term" in r
               for _cid, r in arch.authority_violations())


def test_overlong_observer_term_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    register_observers(arch, home, [(new_observer(), "labor-rights")],
                       valid_from=t0, valid_until=t0 + 400 * _DAY)
    assert any("term" in r for _cid, r in arch.authority_violations())


def test_renomination_within_rotation_period_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    obs = new_observer()
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0, valid_until=t0 + 100 * _DAY)
    # Re-nominated only 30 days after the term ended — inside the 90-day period.
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0 + 130 * _DAY, valid_until=t0 + 200 * _DAY)
    assert any("rotation" in r for _cid, r in arch.authority_violations())


def test_overlapping_renomination_exceeding_term_in_union_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    obs = new_observer()
    # First window: 350 days — under the per-window cap on its own.
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0, valid_until=t0 + 350 * _DAY)
    # A second window opened before the first ends (overlap): neither the per-window
    # nor the rotation-gap rule sees it, but the continuous-service union exceeds the
    # term limit.
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0 + 300 * _DAY, valid_until=t0 + 640 * _DAY)
    assert any("continuous service" in r or "term" in r
               for _cid, r in arch.authority_violations())


def test_overlapping_renomination_within_term_in_union_is_authorized():
    arch = Archive()
    t0 = now_seconds()
    home = new_identity(arch, "Home", t0)
    obs = new_observer()
    # Two overlapping windows whose union (day 0 -> day 200) stays under the cap:
    # contiguous extension is allowed, only over-the-limit chaining is refused.
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0, valid_until=t0 + 100 * _DAY)
    register_observers(arch, home, [(obs, "labor-rights")],
                       valid_from=t0 + 50 * _DAY, valid_until=t0 + 200 * _DAY)
    assert not any("term" in r or "continuous service" in r
                   for _cid, r in arch.authority_violations())


def test_single_community_majority_of_corps_is_unauthorized():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    until = t0 + 100 * _DAY
    register_observers(arch, b, [(new_observer(), "x"), (new_observer(), "y")],
                       valid_from=t0, valid_until=until)
    # A seats 3 → corps of 5 in which A holds a majority.
    register_observers(arch, a, [(new_observer(), "p"), (new_observer(), "q"),
                                  (new_observer(), "r")],
                       valid_from=t0, valid_until=until)
    assert any("pool-share" in r or "majority" in r
               for _cid, r in arch.authority_violations())


def test_pool_concentration_and_enriched_dashboard():
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "A", t0)
    b = new_identity(arch, "B", t0)
    until = t0 + 100 * _DAY
    register_observers(arch, a, [(new_observer(), "p"), (new_observer(), "q"),
                                  (new_observer(), "r")],
                       valid_from=t0, valid_until=until)
    register_observers(arch, b, [(new_observer(), "x"), (new_observer(), "y")],
                       valid_from=t0, valid_until=until)

    pool = observer_pool_concentration(arch, as_of=t0 + 10)
    assert pool.total_active == 5
    assert pool.shares[0] == (a.did, 3)
    assert pool.top_active == 3
    assert pool.majority_held is True
    assert len(pool.serving) == 5

    # The dashboard bundles the pool and the substrate-deference half.
    dash = observer_dashboard(arch, as_of=t0 + 10)
    assert dash.pool.total_active == 5
    assert dash.substrate.ratified is False
