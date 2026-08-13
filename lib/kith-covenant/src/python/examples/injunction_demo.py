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

"""Demonstrate the §9 interim injunction and its capability-gating teeth.

An emergency mediator panel issues an interim injunction against a community,
revoking named compact-gated capabilities while the harm is investigated. The
capability gate reflects the revocation immediately. The full compact confirms
the injunction before it would lapse, so it persists past its 90-day window. A
*different* mediator panel later overturns it on challenge, and the revoked
capabilities are restored.

A capability is a service or an office, and the second half of the demo shows the
distinction biting: a community held out of the offices keeps every relation of
exchange and may no longer staff the rosters or sit on the panels that decide who is
sanctioned. Finally two moves the protocol refuses are shown with the violation each
reports: a panel drawn from the subject's own community, and a challenge heard by the
very panel that issued the injunction.

Everything below is authored through the public Python API, crypto-verified,
and passes the §9 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators,
    issue_injunction, confirm_injunction, challenge_injunction,
    reputation_of, has_capability, now_seconds,
    SERVICE_CAPABILITIES, OFFICE_CAPABILITIES,
)

# §4 caps a mediator's term: a seat with no end is a permanent mediator class.
MEDIATOR_TERM = 365 * 24 * 3600

NINETY_DAYS = 90 * 24 * 3600


def _first_reason(violations):
    return violations[0][1] if violations else "(none)"


def _pool(arch, names, t0):
    """One registered mediator per named non-party community."""
    out = []
    for name in names:
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10, valid_until=t0 - 10 + MEDIATOR_TERM)
        out.append(m)
    return out


def main() -> int:
    arch = Archive()
    t0 = now_seconds()

    subject = new_identity(arch, "Beacon-Keeper", t0)
    alpha = new_identity(arch, "Alpha", t0)
    beta = new_identity(arch, "Beta", t0)
    # Two disjoint emergency panels: one issues, a different one hears the challenge.
    issuing = _pool(arch, ("Med-A", "Med-B", "Med-C"), t0)
    reviewing = _pool(arch, ("Med-D", "Med-E", "Med-F"), t0)

    # Before any injunction, membership implies every capability.
    assert has_capability(arch, subject.did, "Trade", as_of=t0)
    print("Before: subject holds Trade and Navigation.")

    # --- Issue the interim injunction -------------------------------------
    inj = issue_injunction(
        arch, subject.did, issuing,
        harm="withholding a shared beacon during a transit window",
        protective_measures="suspend trade and navigation pending review",
        revoked_capabilities=["Trade", "Navigation"],
        issued_at=t0,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 100)
    print(f"Issued. Injunction status: {v.injunctions[0].status}; "
          f"revoked: {list(v.revoked_capabilities)}.")
    assert v.injunctions[0].status == "Active"
    assert not has_capability(arch, subject.did, "Trade", as_of=t0 + 100)
    assert not has_capability(arch, subject.did, "Navigation", as_of=t0 + 100)
    # The rescue-coordination service is untouched (and the rescue duty is never gated).
    assert has_capability(arch, subject.did, "RescueCoordination", as_of=t0 + 100)
    print("Gate: Trade and Navigation revoked; RescueCoordination unaffected.")

    # --- The compact confirms it before it would lapse --------------------
    confirm_injunction(arch, inj, [alpha, beta], confirmed_at=t0 + 10)
    arch.verify_all()
    arch.check_authority()
    past_expiry = t0 + 2 * NINETY_DAYS
    v = reputation_of(arch, subject.did, as_of=past_expiry)
    print(f"Confirmed. Well past the 90-day window, status: {v.injunctions[0].status}.")
    assert v.injunctions[0].status == "Confirmed"
    assert not has_capability(arch, subject.did, "Trade", as_of=past_expiry)

    # --- A different panel overturns it on challenge ----------------------
    ruled_at = t0 + NINETY_DAYS + 5000
    challenge_injunction(arch, inj, reviewing, overturned=True,
                         reasoning="the beacon was restored; no ongoing harm", decided_at=ruled_at)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=ruled_at + 100)
    print(f"Overturned by a different panel. Status: {v.injunctions[0].status}; "
          f"revoked now: {list(v.revoked_capabilities)}.")
    assert v.injunctions[0].status == "Overturned"
    assert has_capability(arch, subject.did, "Trade", as_of=ruled_at + 100)

    # --- Services and offices are separate holdings -----------------------
    # An injunction that suspends every relation of exchange leaves the rosters open;
    # one that names the offices holds the community out of judgment and nothing else.
    offices = Archive.load_json(arch.dump_json())
    off_at = ruled_at + 10_000
    issue_injunction(
        offices, subject.did, issuing,
        harm="repeated failures of the forbearance owed at the boundary",
        protective_measures="held out of the roles that select who is sanctioned",
        revoked_capabilities=list(OFFICE_CAPABILITIES), issued_at=off_at,
    )
    offices.verify_all()
    offices.check_authority()
    kept = [c for c in SERVICE_CAPABILITIES
            if has_capability(offices, subject.did, c, as_of=off_at + 100)]
    lost = [c for c in OFFICE_CAPABILITIES
            if not has_capability(offices, subject.did, c, as_of=off_at + 100)]
    print(f"\nHeld out of office: keeps {len(kept)} services, loses {len(lost)} offices "
          f"({', '.join(lost)}).")
    assert len(kept) == 5 and len(lost) == 5

    # And the roster itself is refused, naming the office it withheld.
    register_mediators(offices, subject, [new_mediator()],
                       valid_from=off_at + 100,
                       valid_until=off_at + 100 + MEDIATOR_TERM)
    print(f"Refused (seating mediators while held out): "
          f"{_first_reason(offices.authority_violations())}")

    # --- Two moves the protocol refuses -----------------------------------
    # (a) A panel that includes a mediator from the subject's own community.
    bad1 = Archive.load_json(arch.dump_json())
    own = new_mediator()
    register_mediators(bad1, subject, [own], valid_from=t0 - 10, valid_until=t0 - 10 + MEDIATOR_TERM)
    issue_injunction(
        bad1, subject.did, [issuing[0], issuing[1], own],
        harm="h", protective_measures="m", revoked_capabilities=["Trade"], issued_at=t0 + 20,
    )
    print(f"\nRefused (subject's own mediator on the panel): "
          f"{_first_reason(bad1.authority_violations())}")

    # (b) A challenge heard by the very panel that issued the injunction.
    bad2 = Archive.load_json(arch.dump_json())
    inj2 = issue_injunction(
        bad2, subject.did, issuing, harm="h", protective_measures="m",
        revoked_capabilities=["Trade"], issued_at=t0 + 30,
    )
    challenge_injunction(bad2, inj2, issuing, overturned=True, decided_at=t0 + 40)
    print(f"Refused (challenge heard by the issuing panel): "
          f"{_first_reason(bad2.authority_violations())}")

    print("\n✓ injunction_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
