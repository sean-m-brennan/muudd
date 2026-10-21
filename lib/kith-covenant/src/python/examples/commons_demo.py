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

"""Demonstrate §7 commons governance: registry, usage accounting, and the
graduated-sanction ladder (Ostrom's design principles applied to a shared
resource).

Three communities register a communication-spectrum commons and declare their
draw rights (Ostrom principle 1: clear boundaries). Each reports usage; the
free-rider lens surfaces the one community drawing far more than it returns
(principle 4: monitoring; the tragedy-of-the-virtues). The compact then walks
that community up the graduated-sanction ladder (principle 5) —
Monitoring → Warning → Restriction — with the second-class Restriction carrying a
distinct-community quorum, and finally restores it. Finally three moves the
protocol refuses are shown with the violation each reports: a single-community
registration, a sanction that skips a rung, and a restriction imposed by one
community alone.

Everything below is authored through the public Python API, crypto-verified, and
passes the §7 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity,
    register_commons, record_usage, apply_sanction,
    reputation_of, commons_level, free_rider_signals, now_seconds,
)


def _first_reason(violations):
    return violations[0][1] if violations else "(none)"


def main() -> int:
    arch = Archive()
    t0 = now_seconds()

    # Three member communities, all appropriators of one commons.
    alpha = new_identity(arch, "Alpha", t0)
    beta = new_identity(arch, "Beta", t0)
    gamma = new_identity(arch, "Gamma", t0)
    appropriators = [alpha.did, beta.did, gamma.did]

    # --- Register the commons (second-class: co-signed by a quorum) -------
    commons = register_commons(
        arch, "Communication", "inner-system relay spectrum", appropriators,
        registrars=[alpha, beta, gamma],
        description="the shared electromagnetic spectrum and relays",
        provision_rule="provision scales with draw; jamming and monopoly are the gravest violations",
        registered_at=t0,
    )
    arch.verify_all()
    arch.check_authority()
    print(f"Registered a Communication commons among {len(appropriators)} communities.")

    # --- Usage accounting (first-class, self-reported) --------------------
    record_usage(arch, commons, beta, consumed=800, contributed=100, recorded_at=t0 + 10)
    record_usage(arch, commons, beta, consumed=400, contributed=100, recorded_at=t0 + 20)
    record_usage(arch, commons, gamma, consumed=100, contributed=500, recorded_at=t0 + 10)
    record_usage(arch, commons, alpha, consumed=300, contributed=300, recorded_at=t0 + 10)
    arch.verify_all()
    arch.check_authority()

    report = free_rider_signals(arch, commons, as_of=t0 + 100)
    print(f"Usage: total drawn {report.total_consumed}, total returned "
          f"{report.total_contributed}.")
    print(f"Net-draw ranking (drew more than returned): {list(report.by_net_draw)}")
    assert report.by_net_draw[0][0] == beta.did
    assert report.by_net_draw[0][1] == 1000  # 1200 consumed - 200 contributed

    # --- Graduated sanctions against the free-rider (Beta) ----------------
    # Monitoring and Warning are first-class (a single appropriator may move them).
    apply_sanction(arch, commons, beta.did, "None", "Monitoring", [alpha],
                   basis="draw far exceeds contribution over two periods", decided_at=t0 + 200)
    apply_sanction(arch, commons, beta.did, "Monitoring", "Warning", [alpha],
                   basis="pattern continued after monitoring", decided_at=t0 + 300)
    # Restriction is second-class: it needs a distinct-community quorum.
    apply_sanction(arch, commons, beta.did, "Warning", "Restriction", [alpha, gamma],
                   basis="sustained overdraw; restricting drawdown pending restoration",
                   decision_class="SecondClass", decided_at=t0 + 400)
    arch.verify_all()
    arch.check_authority()

    v = reputation_of(arch, beta.did, as_of=t0 + 500)
    assert len(v.commons_sanctions) == 1
    cs = v.commons_sanctions[0]
    print(f"Beta's commons standing: {cs.kind} → {cs.current_level}.")
    assert cs.current_level == "Restriction"
    assert commons_level(arch, beta.did, commons, as_of=t0 + 500) == "Restriction"
    # Alpha and Gamma are unsanctioned.
    assert commons_level(arch, alpha.did, commons, as_of=t0 + 500) == "None"

    # --- Restoration: a single first-class de-escalation back to rest -----
    apply_sanction(arch, commons, beta.did, "Restriction", "None", [alpha],
                   basis="draw brought back within the provision rule", decided_at=t0 + 900)
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, beta.did, as_of=t0 + 1000)
    print(f"After restoration, Beta's standing commons sanctions: {len(v.commons_sanctions)}.")
    assert not v.commons_sanctions
    assert commons_level(arch, beta.did, commons, as_of=t0 + 1000) == "None"

    # --- Three moves the protocol refuses ---------------------------------
    # (a) A single-community registration (second-class, below quorum).
    bad1 = Archive()
    solo = new_identity(bad1, "Solo", t0)
    register_commons(bad1, "LifeSupport", "solo loop", [solo.did],
                     registrars=[solo], registered_at=t0)
    print(f"\nRefused (single-community registration): "
          f"{_first_reason(bad1.authority_violations())}")

    # (b) A sanction that skips a rung (None -> Restriction).
    bad2 = Archive.load_json(arch.dump_json())
    apply_sanction(bad2, commons, gamma.did, "None", "Restriction", [alpha, beta],
                   decision_class="SecondClass", decided_at=t0 + 1100)
    print(f"Refused (sanction skips a rung): {_first_reason(bad2.authority_violations())}")

    # (c) A Restriction imposed by one community alone (below the quorum).
    bad3 = Archive.load_json(arch.dump_json())
    apply_sanction(bad3, commons, gamma.did, "None", "Monitoring", [alpha], decided_at=t0 + 1100)
    apply_sanction(bad3, commons, gamma.did, "Monitoring", "Warning", [alpha], decided_at=t0 + 1200)
    apply_sanction(bad3, commons, gamma.did, "Warning", "Restriction", [alpha],
                   decision_class="SecondClass", decided_at=t0 + 1300)
    print(f"Refused (single-community restriction): {_first_reason(bad3.authority_violations())}")

    print("\n✓ commons_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
