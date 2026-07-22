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
    file_whistleblower, reveal_source, reputation_of, observer_dashboard,
    observer_pool_concentration, cid_of, now_seconds,
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
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    commitment = cid_of(b"source||nonce")
    wb = file_whistleblower(arch, subject.did, commitment, claim="coerced labor",
                            reception_panel=panel, filed_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert len(v.whistleblower_reports_against) == 1
    assert v.whistleblower_reports_against[0].source_revealed is False
    # No source field is exposed anywhere on the summary.
    assert not hasattr(v.whistleblower_reports_against[0], "disclosed_source")

    reveal_source(arch, wb, panel, cause="recanted",
                  disclosed_source="did:key:zMember", revealed_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    assert v.whistleblower_reports_against[0].source_revealed is True


def test_whistleblower_reception_panel_from_accused_is_unauthorized():
    arch, t0, subject, _obs = _fixture()
    panel = _mediator_pool(arch, ("Med-A", "Med-B"), t0)
    own = new_mediator()
    register_mediators(arch, subject, [own], valid_from=t0 - 10)
    file_whistleblower(arch, subject.did, cid_of(b"s"), claim="c",
                       reception_panel=[panel[0], panel[1], own], filed_at=t0 + 100)
    assert any("non-overlap" in r or "accused" in r
               for _cid, r in arch.authority_violations())


def test_reveal_without_cause_rejected_at_facade():
    arch, t0, subject, _obs = _fixture()
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    wb = file_whistleblower(arch, subject.did, cid_of(b"s"), claim="c",
                            reception_panel=panel, filed_at=t0 + 100)
    with pytest.raises(ValueError):
        reveal_source(arch, wb, panel, cause="   ", disclosed_source="x")


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
