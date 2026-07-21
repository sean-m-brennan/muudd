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
"""
from __future__ import annotations

import sys

from kith_covenant import run_harness


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

    print()
    if not report.all_supported_passed:
        print("✗ capture_harness_demo (Python): a supported scenario FAILED")
        return 1
    print("✓ capture_harness_demo (Python): SUCCESS "
          "(all supported scenarios detected within their falsification bounds)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
