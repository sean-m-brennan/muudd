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

"""Demonstrate §4 mediation over a signed audit commons.

Two communities become parties to a dispute; two *other* communities each
nominate mediators into the commons-wide pool. A three-person panel drawn
entirely from the non-party pool decides the dispute, and the decision passes
the §5 authority check. Then the same panel is rebuilt two ways that the
protocol must refuse — too few mediators, and a mediator from a party
community — and `check_authority` reports each violation with provenance.

Everything below is authored through the public Python API and crypto-verified.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    decide_mediation, reputation_of, now_seconds,
)


def main() -> int:
    arch = Archive()
    # Parties to the dispute.
    alpha = new_identity(arch, "Alpha")     # complainant
    beta = new_identity(arch, "Beta")       # respondent
    # Non-party communities that staff the mediator pool.
    gamma = new_identity(arch, "Gamma")
    delta = new_identity(arch, "Delta")

    t0 = now_seconds()
    # The pool = the union of every community's mediator registry. Gamma
    # nominates two mediators, Delta one; none belongs to a party community.
    g1, g2, d1 = new_mediator(), new_mediator(), new_mediator()
    register_mediators(arch, gamma, [g1, g2], valid_from=t0 - 100)
    register_mediators(arch, delta, [d1], valid_from=t0 - 100)
    print("Mediator pool (union across communities):")
    print("  Gamma nominates 2, Delta nominates 1 — 3 non-party mediators\n")

    # Alpha files a dispute against Beta, reaching for mediated arbitration.
    dispute = file_dispute(
        arch, alpha, beta.did, "MediatedArbitration",
        summary="Beta decided a matter Alpha's charter reserves",
        filed_at=t0,
    )
    print("Alpha filed a dispute against Beta (rung=MediatedArbitration)")

    # A three-person panel from the non-party pool decides it.
    panel = [g1, g2, d1]
    decide_mediation(
        arch, dispute, panel, "Upheld",
        reasoning="the three non-party mediators concur", decided_at=t0 + 10,
    )
    arch.verify_all()          # crypto integrity of the whole commons
    arch.check_authority()     # §4 panel rule + §5 authority: PASSES
    print("Panel of 3 non-party mediators decided: Upheld")
    print("  verify_all() + check_authority(): PASS\n")

    # The dispute and its outcome project onto both parties. The projection is
    # a point-in-time view, so ask as of a moment at or after the decision.
    va = reputation_of(arch, alpha.did, as_of=t0 + 100)
    vb = reputation_of(arch, beta.did, as_of=t0 + 100)
    print("Projection:")
    print(f"  Alpha.disputes_filed={len(va.disputes_filed)} "
          f"outcome={va.disputes_filed[0].decided_outcome}")
    print(f"  Beta.disputes_against={len(vb.disputes_against)} "
          f"outcome={vb.disputes_against[0].decided_outcome}\n")

    # --- Panels the protocol must refuse ------------------------------------
    # (a) Too few mediators for the default three-person rule.
    bad_small = Archive.load_json(arch.dump_json())
    d2 = file_dispute(bad_small, alpha, beta.did, "MediatedArbitration",
                      filed_at=t0)
    decide_mediation(bad_small, d2, panel[:2], "Dismissed", decided_at=t0 + 10)
    bad_small.verify_all()  # crypto is fine
    violations = bad_small.authority_violations()
    print("Rejected panel — only 2 mediators (default panel_min=3):")
    print(f"  {_first_reason(violations)}")

    # (b) A mediator from a party community (non-overlap violated).
    bad_party = Archive.load_json(arch.dump_json())
    alpha_insider = new_mediator()
    register_mediators(bad_party, alpha, [alpha_insider], valid_from=t0 - 100)
    d3 = file_dispute(bad_party, alpha, beta.did, "MediatedArbitration",
                      filed_at=t0)
    decide_mediation(bad_party, d3, [g1, g2, alpha_insider], "Upheld",
                     decided_at=t0 + 10)
    bad_party.verify_all()  # crypto is fine: the insider's key resolves
    violations = bad_party.authority_violations()
    print("Rejected panel — a mediator from a party community (non-overlap):")
    print(f"  {_first_reason(violations)}")

    print("\n✓ mediation_demo (Python): SUCCESS")
    return 0


def _first_reason(violations) -> str:
    """The reason string of the first authority violation, or a sentinel."""
    return violations[0][1] if violations else "(no violation — unexpected!)"


if __name__ == "__main__":
    sys.exit(main())
