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

"""Demonstrate the §9 substrate-deference test: the compact-incapable list and
the structured-fast cadence as the operational proof of the compact's
non-ultimacy.

Founding communities ratify a constitution naming the matters the compact will
not decide (a member's internal theology and governance) and an annual fast
window. The compact-incapable gate then refuses to treat those matters as the
compact's to decide. Expansion of the list is easy (a single second-class
quorum); contraction is hard (a higher bar) and a genesis matter cannot be
removed at all. The erosion report makes drift toward ultimacy legible: net
contraction of the list, fast shrinkage, and a deliberative act initiated during
a fast (surfaced as a deviation, not hard-refused). Finally three moves the
protocol refuses are shown with the violation each reports.

Everything below is authored through the public Python API, crypto-verified, and
passes the §9 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity,
    ratify_constitution, amend_compact_incapable, amend_fast_cadence,
    file_dispute, register_mediators, new_mediator,
    constitution_state, compact_incapable, in_structured_fast, substrate_deference,
    now_seconds,
)


def _first_reason(violations):
    return violations[0][1] if violations else "(none)"


def main() -> int:
    arch = Archive()
    t0 = now_seconds()
    a = new_identity(arch, "Alpha", t0)
    b = new_identity(arch, "Beta", t0)
    c = new_identity(arch, "Gamma", t0)

    fast_start, fast_end = t0 + 1000, t0 + 2000

    # --- Ratify the founding constitution (second-class, singleton) -------
    ratify_constitution(
        arch, [a, b, c],
        incapable=[
            ("internal-theology", "the theology of any member's substrate"),
            ("internal-governance", "the internal governance of any member"),
        ],
        fast_periods=[("annual", fast_start, fast_end)],
        genesis_at=t0,
    )
    arch.verify_all()
    arch.check_authority()
    st = constitution_state(arch, as_of=t0 + 10)
    print(f"Ratified. {len(st.compact_incapable)} compact-incapable matters, "
          f"{len(st.fast_periods)} fast period.")
    assert compact_incapable(arch, "internal-theology", as_of=t0 + 10)
    assert not compact_incapable(arch, "commons-allocation", as_of=t0 + 10)

    # The temporal-deference gate: is the compact fasting?
    assert in_structured_fast(arch, fast_start + 500) == "annual"
    assert in_structured_fast(arch, fast_end + 500) is None
    print("Categorical + temporal deference gates live.")

    # --- Expansion is easy; contraction of a genesis matter is impossible --
    amend_compact_incapable(arch, "Add", "internal-family-structure", [a, b],
                            matter_description="the family structure of any member",
                            rationale="clarifying a matter left implicit", decided_at=t0 + 100)
    arch.verify_all()
    arch.check_authority()
    print(f"Expanded the list to {len(constitution_state(arch, as_of=t0 + 200).compact_incapable)} matters "
          f"(easy: normal quorum).")

    # --- Erosion: a deliberative act initiated during the fast -------------
    # (A dispute filed inside the fast window is surfaced as a deviation.)
    file_dispute(arch, a, b.did, "MediatedArbitration", filed_at=fast_start + 300)
    arch.verify_all()
    arch.check_authority()
    report = substrate_deference(arch, as_of=t0 + 5000)
    print(f"Erosion report: {report.incapable_added} added / {report.incapable_removed} removed "
          f"(contraction_detected={report.contraction_detected}); "
          f"{len(report.fast_deviations)} fast deviation "
          f"('{report.fast_deviations[0].kind}' during '{report.fast_deviations[0].fast_label}').")
    assert report.incapable_added == 1
    assert report.contraction_detected is False
    assert len(report.fast_deviations) == 1

    # --- Three moves the protocol refuses ---------------------------------
    # (a) A second genesis (singleton).
    bad1 = Archive.load_json(arch.dump_json())
    ratify_constitution(bad1, [a, b], incapable=[("x", "x")], fast_periods=[], genesis_at=t0 + 600)
    print(f"\nRefused (second genesis): {_first_reason(bad1.authority_violations())}")

    # (b) Removing a genesis-original matter (immutable).
    bad2 = Archive.load_json(arch.dump_json())
    amend_compact_incapable(bad2, "Remove", "internal-theology", [a, b, c],
                            rationale="pragmatic pressure", decided_at=t0 + 700)
    print(f"Refused (remove genesis matter): {_first_reason(bad2.authority_violations())}")

    # (c) Contracting the list at the normal quorum (needs the higher bar).
    bad3 = Archive.load_json(arch.dump_json())
    amend_compact_incapable(bad3, "Remove", "internal-family-structure", [a, b],
                            rationale="two is not enough", decided_at=t0 + 800)
    print(f"Refused (contraction below the higher bar): {_first_reason(bad3.authority_violations())}")

    print("\n✓ constitution_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
