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
report. Separately, a mediator reception panel files an insider whistleblower
report whose source is only a commitment — never stored — and later, for cause,
a panel discloses it. The observer dashboard tallies the compact-wide signals.
Finally three moves the protocol refuses are shown with the violation each
reports: an observer reporting on its own community, a non-subject trying to
respond, and a whistleblower reception panel drawn from the accused community.

Everything below is authored through the public Python API, crypto-verified, and
passes the §8 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, new_observer, register_mediators,
    register_observers, publish_observer_report, respond_to_report,
    file_whistleblower, reveal_source, reputation_of, observer_dashboard,
    cid_of, now_seconds,
)


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
    register_observers(arch, reporter_home, [(obs, "labor-rights")], valid_from=t0 - 10)
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

    # --- Whistleblower: source is only a commitment -----------------------
    panel = _mediator_pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    commitment = cid_of(b"source:crew-member-7||nonce:9f3a")
    wb = file_whistleblower(
        arch, subject.did, commitment,
        claim="coerced labor in the maintenance rota, unredressed internally",
        reception_panel=panel, filed_at=t0 + 300,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    print(f"Whistleblower reports against subject: {len(v.whistleblower_reports_against)} "
          f"(source_revealed={v.whistleblower_reports_against[0].source_revealed}).")
    assert v.whistleblower_reports_against[0].source_revealed is False

    # For cause, a mediator panel discloses the source.
    reveal_source(arch, wb, panel,
                  cause="the source recanted and the claim is now contested",
                  disclosed_source="did:key:zCrewMember7", revealed_at=t0 + 400)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 500)
    print(f"After for-cause reveal, source_revealed="
          f"{v.whistleblower_reports_against[0].source_revealed}.")
    assert v.whistleblower_reports_against[0].source_revealed is True

    # --- Observer dashboard (composition-drift board) ---------------------
    dash = observer_dashboard(arch, as_of=t0 + 500)
    print(f"Dashboard: observer reports by subject {list(dash.observer_reports_by_subject)}; "
          f"whistleblowers by accused {list(dash.whistleblowers_by_accused)}.")
    assert dash.observer_reports_by_subject == ((subject.did, 1),)
    assert dash.whistleblowers_by_accused == ((subject.did, 1),)

    # --- Three moves the protocol refuses ---------------------------------
    # (a) An observer reporting on its own community.
    bad1 = Archive.load_json(arch.dump_json())
    own_obs = new_observer()
    register_observers(bad1, subject, [(own_obs, "internal")], valid_from=t0 - 10)
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
    file_whistleblower(bad3, subject.did, commitment, claim="c",
                       reception_panel=[panel[0], panel[1], own_med], filed_at=t0 + 600)
    print(f"Refused (reception panel from the accused community): "
          f"{_first_reason(bad3.authority_violations())}")

    print("\n✓ observer_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
