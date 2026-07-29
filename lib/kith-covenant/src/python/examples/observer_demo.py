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

"""Demonstrate the §8 observer / jester corps: cross-community journalism with a
right of response, and the whistleblower channel with a quorum-gated reveal.

A community nominates a traveling observer with a declared perspective. The
observer files an attributed public report on *another* community's troubling-
but-legal internal practice (the "hard middle case" the escalation ladder cannot
reach); the subject answers with an appended response that never suppresses the
report.

Separately, an insider of the accused community files a whistleblower report. The
source ring-signs its own attestation — proving it is an insider without saying
which one — and the panel seals its identity under k-of-n threshold custody, so no
minority of the panel can name it. Later, for cause, a quorum opens its shares and
the disclosure is checked against what the shares actually recover. A member the
disclosure could have named answers with a proof that the filing's key image is not
theirs, needing no panel's leave. The observer dashboard tallies the compact-wide
signals — including how many *distinct* insiders have filed, so one voice repeating
itself does not read as many — and carries the observer-corps composition
(pool-share / rotation) and the substrate-deference erosion report.

Finally the moves the protocol refuses are shown with the violation each reports: an
observer reporting on its own community, a non-subject trying to respond, a
whistleblower reception panel drawn from the accused community, a panel
manufacturing a filing with no source behind it, a reveal naming somebody the
custody shares do not produce, a reveal one share short of the threshold, the true
source trying to disown its own filing, one community packing a majority of the
observer corps, re-nominating an observer inside the rotation period, and chaining
overlapping windows to extend service past the term limit.

Everything below is authored through the public Python API, crypto-verified, and
passes the §8 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, new_observer, register_mediators,
    register_observers, publish_observer_report, respond_to_report,
    delegate_speaker, insider_ring, attest_insider, file_whistleblower,
    open_source_share, reveal_source, exculpate_source, reputation_of,
    observer_dashboard, now_seconds,
)

_DAY = 86400


def _first_reason(violations):
    return violations[0][1] if violations else "(none)"


def _mediator_pool(arch, names, t0):
    """One registered non-party mediator per named community."""
    out = []
    for name in names:
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10)
        out.append(m)
    return out


def main() -> int:
    arch = Archive()
    t0 = now_seconds()

    subject = new_identity(arch, "Deep-Habitat", t0)
    reporter_home = new_identity(arch, "Way-Station", t0)

    # --- Nominate a traveling observer, file an attributed report ---------
    obs = new_observer()
    # A bounded term (100 days): §8 refuses open-ended observer standing.
    register_observers(arch, reporter_home, [(obs, "labor-rights")],
                       valid_from=t0 - 10, valid_until=t0 + 100 * _DAY)
    report = publish_observer_report(
        arch, obs, subject.did, "labor-rights",
        summary="internal maintenance rota is far more coercive than compact peers",
        reported_at=t0 + 100,
    )
    arch.verify_all()
    arch.check_authority()

    # The subject answers publicly; the report is not suppressed.
    respond_to_report(arch, subject, report,
                      "rota participation is by council consent; see our charter",
                      responded_at=t0 + 200)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    print(f"Observer report on subject: {len(v.observer_reports)} "
          f"(perspective '{v.observer_reports[0].declared_perspective}', "
          f"{len(v.observer_reports[0].responses)} response).")
    assert len(v.observer_reports) == 1
    assert len(v.observer_reports[0].responses) == 1

    # --- Whistleblower: an anonymous insider, a threshold-held identity ----
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    # The accused community's insiders are the keys the archive can resolve for it:
    # its root key plus its live speaker delegations. Two crew members hold
    # delegations, so a filing by either is hidden among three keys.
    crew7 = delegate_speaker(arch, subject, "Reciter", issued_at=t0 - 10)
    crew9 = delegate_speaker(arch, subject, "Reciter", issued_at=t0 - 10)
    ring = insider_ring(arch, subject.did, at=t0 + 300)
    print(f"\nInsider ring for the accused: {len(ring)} keys "
          f"(root + {len(ring) - 1} delegated speakers).")

    # The source's step, performed alone: ring-sign an attestation and commit to its
    # own identity. The packet goes to the panel out of band; the key never does.
    claim = "coerced labor in the maintenance rota, unredressed internally"
    packet = attest_insider(arch, subject.did, crew7, claim, filed_at=t0 + 300)
    # The panel's step: seal the identity so 2 of the 3 are needed to name it.
    wb = file_whistleblower(
        arch, subject.did, claim, reception_panel=panel,
        attestation=packet, threshold=2,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    summary = v.whistleblower_reports_against[0]
    print(f"Whistleblower reports against subject: {len(v.whistleblower_reports_against)} "
          f"(source_revealed={summary.source_revealed}, "
          f"source_tag={summary.source_tag[:16]}…).")
    assert summary.source_revealed is False
    # The tag identifies the filing's source as *a* source, never as a person.
    assert summary.source_tag

    # One panelist alone cannot name the source: the shares are the gate, not a promise.
    one_share = [open_source_share(arch, wb, panel[0])]
    solo = Archive.load_json(arch.dump_json())
    reveal_source(solo, wb, panel,
                  cause="acting alone",
                  disclosed_source=crew7.did, shares=one_share,
                  revealed_at=t0 + 400)
    print(f"Refused (one panelist short of the custody threshold): "
          f"{_first_reason(solo.authority_violations())}")

    # For cause, a quorum opens its shares and the disclosure is checked against
    # what they actually recover.
    reveal_source(arch, wb, panel,
                  cause="the source recanted and the claim is now contested",
                  disclosed_source=crew7.did, revealed_at=t0 + 400)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    print(f"After for-cause reveal, source_revealed="
          f"{v.whistleblower_reports_against[0].source_revealed}.")
    assert v.whistleblower_reports_against[0].source_revealed is True

    # The other delegated crew member — who did nothing — clears itself with a proof
    # that the filing's key image is not its own. No panel is asked.
    exculpate_source(arch, wb, crew9, exculpated_at=t0 + 450)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    print(f"A member cleared itself without a panel: "
          f"exculpations={v.whistleblower_reports_against[0].exculpations}.")
    assert v.whistleblower_reports_against[0].exculpations == 1

    # --- Observer dashboard (composition-drift board) ---------------------
    dash = observer_dashboard(arch, as_of=t0 + 500)
    print(f"Dashboard: observer reports by subject {list(dash.observer_reports_by_subject)}; "
          f"whistleblowers by accused {list(dash.whistleblowers_by_accused)}; "
          f"distinct sources {list(dash.distinct_sources_by_accused)}.")
    print(f"  Pool: {dash.pool.total_active} active observer(s), "
          f"shares {list(dash.pool.shares)}, majority_held={dash.pool.majority_held}.")
    print(f"  Substrate-deference: ratified={dash.substrate.ratified}, "
          f"contraction_detected={dash.substrate.contraction_detected}.")
    assert dash.observer_reports_by_subject == ((subject.did, 1),)
    assert dash.whistleblowers_by_accused == ((subject.did, 1),)
    assert dash.distinct_sources_by_accused == ((subject.did, 1),)
    assert dash.pool.total_active == 1

    # --- Five moves the protocol refuses ----------------------------------
    # (a) An observer reporting on its own community.
    bad1 = Archive.load_json(arch.dump_json())
    own_obs = new_observer()
    register_observers(bad1, subject, [(own_obs, "internal")],
                       valid_from=t0 - 10, valid_until=t0 + 100 * _DAY)
    publish_observer_report(bad1, own_obs, subject.did, "internal",
                            summary="self-report", reported_at=t0 + 600)
    print(f"\nRefused (observer reporting on its own community): "
          f"{_first_reason(bad1.authority_violations())}")

    # (b) A non-subject community trying to respond to the report.
    bad2 = Archive.load_json(arch.dump_json())
    interloper = new_identity(bad2, "Interloper", t0)
    respond_to_report(bad2, interloper, report, "we object", responded_at=t0 + 600)
    print(f"Refused (non-subject response): {_first_reason(bad2.authority_violations())}")

    # (c) A whistleblower reception panel drawn from the accused community.
    bad3 = Archive.load_json(arch.dump_json())
    own_med = new_mediator()
    register_mediators(bad3, subject, [own_med], valid_from=t0 - 10)
    packet3 = attest_insider(bad3, subject.did, crew7, "c", filed_at=t0 + 600)
    file_whistleblower(bad3, subject.did, "c",
                       reception_panel=[panel[0], panel[1], own_med],
                       attestation=packet3, threshold=2)
    print(f"Refused (reception panel from the accused community): "
          f"{_first_reason(bad3.authority_violations())}")

    # (c2) A panel manufacturing a filing with no insider behind it. It can only
    # ring-sign with keys it holds, and those are not the accused's insider set, so
    # the attestation cannot even be built.
    bad3b = Archive.load_json(arch.dump_json())
    try:
        forged = attest_insider(bad3b, subject.did, panel[0], "invented",
                                filed_at=t0 + 600)
        file_whistleblower(bad3b, subject.did, "invented", reception_panel=panel,
                           attestation=forged, threshold=2)
        print(f"Refused (panel-manufactured filing): "
              f"{_first_reason(bad3b.authority_violations())}")
    except ValueError as e:
        print(f"Refused (panel-manufactured filing): {e}")

    # (c3) A reveal naming somebody the custody shares do not produce. The panel has
    # a real quorum and a real cause — and still cannot name the wrong crew member.
    bad3c = Archive.load_json(arch.dump_json())
    packet3c = attest_insider(bad3c, subject.did, crew7, "rota", filed_at=t0 + 600)
    wb3c = file_whistleblower(bad3c, subject.did, "rota", reception_panel=panel,
                              attestation=packet3c, threshold=2)
    reveal_source(bad3c, wb3c, panel, cause="for cause",
                  disclosed_source=crew9.did, revealed_at=t0 + 700)
    print(f"Refused (reveal naming someone the shares do not recover): "
          f"{_first_reason(bad3c.authority_violations())}")

    # (c4) The true source trying to disown its own filing. An honest proof from
    # crew7 reproduces the filing's key image, so the record is a confession.
    bad3d = Archive.load_json(arch.dump_json())
    exculpate_source(bad3d, wb, crew7, exculpated_at=t0 + 700)
    print(f"Refused (the true source disowning its own filing): "
          f"{_first_reason(bad3d.authority_violations())}")

    # (d) A single community packing a majority of the observer corps. The base
    # archive already carries one observer (Way-Station); Other adds one more, so
    # Packer's three make a corps of five in which Packer holds a majority.
    bad4 = Archive.load_json(arch.dump_json())
    packer = new_identity(bad4, "Packer", t0)
    other = new_identity(bad4, "Other", t0)
    until = t0 + 100 * _DAY
    register_observers(bad4, other, [(new_observer(), "a")],
                       valid_from=t0, valid_until=until)
    register_observers(bad4, packer, [(new_observer(), "p"), (new_observer(), "q"),
                                       (new_observer(), "r")],
                       valid_from=t0, valid_until=until)
    print(f"Refused (one community holding a majority of the corps): "
          f"{_first_reason(bad4.authority_violations())}")

    # (e) Re-nominating an observer before the rotation period elapses.
    bad5 = Archive.load_json(arch.dump_json())
    home2 = new_identity(bad5, "Home-2", t0)
    rotator = new_observer()
    register_observers(bad5, home2, [(rotator, "labor-rights")],
                       valid_from=t0, valid_until=t0 + 100 * _DAY)
    register_observers(bad5, home2, [(rotator, "labor-rights")],
                       valid_from=t0 + 130 * _DAY, valid_until=t0 + 200 * _DAY)
    print(f"Refused (re-nomination inside the rotation period): "
          f"{_first_reason(bad5.authority_violations())}")

    # (f) Chaining overlapping windows to extend service past the term limit. Each
    # window is under the cap alone and the second overlaps the first, so the
    # rotation-gap rule never fires — but the continuous-service union exceeds it.
    bad6 = Archive.load_json(arch.dump_json())
    home3 = new_identity(bad6, "Home-3", t0)
    lifer = new_observer()
    register_observers(bad6, home3, [(lifer, "labor-rights")],
                       valid_from=t0, valid_until=t0 + 350 * _DAY)
    register_observers(bad6, home3, [(lifer, "labor-rights")],
                       valid_from=t0 + 300 * _DAY, valid_until=t0 + 640 * _DAY)
    print(f"Refused (overlap-chained service past the term limit): "
          f"{_first_reason(bad6.authority_violations())}")

    print("\n✓ observer_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
