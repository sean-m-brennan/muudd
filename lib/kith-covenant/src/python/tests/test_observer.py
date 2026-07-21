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
    cid_of, now_seconds,
)


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
    register_observers(arch, home, [(obs, "labor-rights")], valid_from=t0 - 10)
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
    register_observers(arch, home, [(obs, "internal")], valid_from=t0 - 10)
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
