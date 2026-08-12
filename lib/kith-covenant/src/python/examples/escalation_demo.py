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

"""Demonstrate the §5 escalation ladder and anathema designation.

A dispute climbs the ladder one rung at a time and then de-escalates (every
concrete act is reversible). Separately, a community that accepts and then
re-violates restitution three times is designated anathema after the full
four-condition procedure — and then earns its way back through the re-entry
rite, which reverses the designation and leaves it on probation. Finally two
moves the protocol must refuse are shown with the violation each reports.

Everything below is authored through the public Python API, crypto-verified,
and passes the §5 authority check.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, new_identity, new_mediator, register_mediators, file_dispute,
    file_ladder_transition, decide_designation, impose_restitution,
    verify_restitution, complete_reentry, record_conduct, reputation_of,
    now_seconds,
)

WAITING = 7 * 24 * 3600  # the designation deliberation waiting period
MOVEMENTS = (
    "We anchor above ourselves.",
    "We acknowledge we caused harm.",
    "We recognize those we wronged.",
    "We recommit to the common virtues.",
    "This compact is not our ultimate.",
)

# §4 caps a mediator's term: a seat with no end is a permanent mediator class.
MEDIATOR_TERM = 365 * 24 * 3600


def _first_reason(violations):
    return violations[0][1] if violations else "(none)"


def main() -> int:
    arch = Archive()
    t0 = now_seconds()

    # --- The escalation ladder --------------------------------------------
    alpha = new_identity(arch, "Alpha", t0)
    beta = new_identity(arch, "Beta", t0)
    dispute = file_dispute(arch, alpha, beta.did, "RegisteredProtest", filed_at=t0)
    print("Filed a dispute at RegisteredProtest.")
    file_ladder_transition(arch, dispute, "RegisteredProtest", "ReciprocalSuspension",
                           [alpha], reasoning="named courtesies suspended", transitioned_at=t0 + 10)
    file_ladder_transition(arch, dispute, "ReciprocalSuspension", "MediatedArbitration",
                           [alpha], reasoning="referred to a panel", transitioned_at=t0 + 20)
    # A face-saving retreat: de-escalate to a lower rung.
    file_ladder_transition(arch, dispute, "MediatedArbitration", "RegisteredProtest",
                           [alpha], reasoning="parties stepped back", transitioned_at=t0 + 30)
    v = reputation_of(arch, alpha.did, as_of=t0 + 100)
    print(f"Dispute filed at {v.disputes_filed[0].rung}, "
          f"now at {v.disputes_filed[0].current_rung} after escalation + de-escalation.")

    # --- The anathema designation (four-condition procedure) --------------
    subject = new_identity(arch, "Drifter", t0)
    wronged = new_identity(arch, "Wronged", t0)
    # A non-party mediator pool to verify restitution completions.
    pool = []
    for name in ("Pool-A", "Pool-B", "Pool-C"):
        c = new_identity(arch, name, t0)
        m = new_mediator()
        register_mediators(arch, c, [m], valid_from=t0 - 10, valid_until=t0 - 10 + MEDIATOR_TERM)
        pool.append(m)

    completions = []
    act = None
    for i in range(3):
        base = t0 + 200 + i * 100
        ob = impose_restitution(arch, subject.did, wronged.did, [wronged],
                                f"restore round {i + 1}", imposed_at=base)
        completions.append(verify_restitution(arch, ob, pool, True, verified_at=base + 10))
        act = record_conduct(arch, wronged, subject.did, "Concern", "Charity",
                             narrative="re-violated after restitution", recorded_at=base + 20)
    print("Subject accepted and then re-violated restitution three times.")

    cycles = [
        (0, [wronged.did], t0 + 2000, t0 + 2100, "first consideration"),
        (1, [wronged.did], t0 + 2100 + WAITING, t0 + 2200 + WAITING, "second consideration"),
    ]
    designation = decide_designation(
        arch, subject.did, [wronged, alpha],
        "sustained bad faith: three graver failed restorations",
        tier="Anathema", cited_acts=[act], failed_restoration_cycles=completions,
        deliberation_cycles=cycles, reversal_conditions="sustained changed conduct on probation",
        reexamine_after=t0 + 999_999, designated_at=t0 + 3000 + WAITING,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 4000 + WAITING)
    print(f"Designation stands. Subject standing: {v.standing}.")
    assert v.standing == "Anathema"

    # --- Re-entry: the way back -------------------------------------------
    w1 = new_identity(arch, "Witness-1", t0)
    w2 = new_identity(arch, "Witness-2", t0)
    # The admitting quorum is two communities other than the petitioner: §2 does not
    # count the returning community toward its own readmission, because "no single
    # sponsor cluster can admit alone" is emptiest when the cluster is the entrant.
    complete_reentry(
        arch, designation, subject, [wronged, alpha], MOVEMENTS,
        [(w1, True), (w2, True)],
        conduct_accounting="we account for the pattern that led to designation",
        public_acknowledgement="what we did was wrong",
        probation_until=t0 + 10_000_000, completed_at=t0 + 5000 + WAITING,
    )
    arch.verify_all()
    arch.check_authority()
    v = reputation_of(arch, subject.did, as_of=t0 + 6000 + WAITING)
    print(f"After re-entry the designation is reversed; standing: {v.standing}. "
          f"A probation window (until {v.reentries[0].probation_until}) is on record; "
          f"a re-admitted member would read as Probationary within it.")
    assert v.designations[0].reversed
    assert v.standing != "Anathema"

    # --- Two moves the protocol refuses -----------------------------------
    bad1 = Archive.load_json(arch.dump_json())
    d2 = file_dispute(bad1, alpha, beta.did, "DisputeRegistered", filed_at=t0 + 100)
    file_ladder_transition(bad1, d2, "DisputeRegistered", "LimitedSanctions", [alpha],
                           transitioned_at=t0 + 110)  # skips three rungs
    print(f"\nRefused (illegal ladder skip): {_first_reason(bad1.authority_violations())}")

    bad2 = Archive.load_json(arch.dump_json())
    subj2 = new_identity(bad2, "Drifter2", t0)
    decide_designation(
        bad2, subj2.did, [wronged, alpha], "vague bad faith", tier="Anathema",
        cited_acts=[act], failed_restoration_cycles=[],  # condition 1 unmet
        deliberation_cycles=cycles, reversal_conditions="rev", designated_at=t0 + 3000 + WAITING,
    )
    print(f"Refused (designation without the cycle pattern): "
          f"{_first_reason(bad2.authority_violations())}")

    print("\n✓ escalation_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
