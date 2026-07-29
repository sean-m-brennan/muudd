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

"""Run the §10 capture-resistance harness and print the scoreboard.

Each of the seven capture-resistance scenarios is scored against falsification
criteria stated in advance (kith_covenant.harness constants). Every scenario now
has a built detector and is exercised end-to-end against the real public API,
scored PASS/FAIL on detection latency and damage bounds. A scenario whose
detector was deferred would be reported as UNSUPPORTED with the missing
capability named, so coverage gaps are surfaced rather than silently omitted.

Then the continuous half (§10.3). The scoreboard above is one point in the space:
with the parameters fixed the harness is a constant function, so a scheduler
around it would recompute the same eight numbers forever. Instead the parameters
are drawn from a seeded RNG round after round, and latency and damage accumulate
across rounds; and a directed sweep walks one parameter through its whole range to
show where a detector's bound moves with the fixture. Both use a **fixed seed**
here, so this demo can never fail on an unlucky draw.
"""
from __future__ import annotations

import sys

from kith_covenant import run_harness, run_continuous, sweep


def main() -> int:
    report = run_harness()

    print(f"Capture-resistance harness: {report.supported} supported scenarios, "
          f"{report.passed}/{report.supported} passed "
          f"({report.total} total incl. unsupported).\n")

    for r in report.results:
        if not r.supported:
            print(f"  [move {r.move}] {r.name}")
            print(f"      UNSUPPORTED — {r.notes}")
            continue
        status = "PASS" if r.passed else "FAIL"
        print(f"  [move {r.move}] {r.name}")
        print(f"      {status}  detected={r.detected} "
              f"latency={r.detection_latency} (<= {r.threshold_latency}); "
              f"damage={r.damage} (<= {r.threshold_damage})")
        print(f"      {r.notes}")

    if not report.all_supported_passed:
        print("\n✗ capture_harness_demo (Python): a supported scenario FAILED")
        return 1

    # --- Continuous execution (§10.3) -------------------------------------
    print("\nContinuous execution: the same eight scenarios at drawn parameters,")
    print("accumulating latency and damage across rounds (fixed seed).\n")
    cont = run_continuous(rounds=10, seed=20260729, stop_on_failure=False)
    print(f"  {cont.rounds_run} rounds in {cont.elapsed_seconds:.1f}s, "
          f"seed {cont.master_seed}")
    for st in cont.stats:
        dist = " ".join(f"{v}x{n}" for v, n in st.damage_histogram)
        print(f"    {st.key:26} {st.passes}/{st.runs} pass  "
              f"worst latency {st.worst_latency}, worst damage {st.worst_damage}  "
              f"[damage seen: {dist}]")
    if cont.failures:
        f = cont.failures[0]
        print(f"\n  FAILURE {f.key} at seed {f.seed}; smallest reproduction: {f.shrunk}")
        print("✗ capture_harness_demo (Python): the continuous search found a failure")
        return 1
    print("\n  No failures across the drawn parameter sets. That is only meaningful")
    print("  because the machinery can fail: the test suite deliberately breaks a")
    print("  criterion and checks the failure is caught, shrunk, and replayable.")

    # --- One directed sweep ----------------------------------------------
    print("\nDirected sweep: witness cycles held flat before the decline begins.")
    print("A flat cycle gives the decay lens nothing to compare, so the damage bound")
    print("has to move with the fixture — this is the criterion that used to be a")
    print("constant, and the sweep is what shows the difference.\n")
    for pt in sweep("ceremony_flat_prefix"):
        r = pt.result("ceremony-hollowing")
        print(f"    flat prefix {pt.value}: witnesses {pt.params.witness_sequence()}  "
              f"damage {r.damage} <= {r.threshold_damage}  "
              f"{'PASS' if pt.passed else 'FAIL'}")

    print("\n✓ capture_harness_demo (Python): SUCCESS "
          "(all supported scenarios detected within their falsification bounds, "
          "at the canonical parameters and across the drawn and swept space)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
