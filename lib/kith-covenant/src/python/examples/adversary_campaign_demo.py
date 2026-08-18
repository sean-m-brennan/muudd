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

"""Demonstrate the determined-adversary campaign end to end.

One deceitful community joins a standing compact by passing a rite it does not
believe, leaks its orientation, is prosecuted up the whole §5 ladder, accepts and
breaks restitution three times, is designated anathema, earns its way back through
the re-entry rite, betrays that too, and is finally designated *hostis humani
generis* and put out of the compact. Nine phases over one archive, each scored
against a criterion stated in advance.

Then the three questions the campaign was built to answer, printed with the numbers
behind them: what a designation actually withholds, what the irrecoverable tier
costs to reach, and whether the compact agreed about any of it.

Everything below is authored through the public Python API, crypto-verified, and
passes the authority check.
"""
from __future__ import annotations

import sys

from kith_covenant.adversary import run_campaign, format_report


def main() -> int:
    report = run_campaign()
    print(format_report(report))

    print("\n--- What a standing designation withholds --------------------------")
    teeth = report.by_key("anathema")
    print(f"Office acts a designated community may still take: "
          f"{teeth.metrics['office_acts_authorized']} of 4. "
          f"Second-class acts its weight may carry: "
          f"{teeth.metrics['second_class_acts_authorized']}. "
          f"Ordinary restitution records it may open: "
          f"{teeth.metrics['ordinary_restitution_records_authorized']} of 2. "
          f"Designations of itself it may help carry: "
          f"{teeth.metrics['self_quorum_designations_authorized']}.")
    print("Its five services are untouched throughout, and the rescue duty is never "
          "gated at all: even against the demonic, the compact does not become demonic.")

    print("\n--- What the irrecoverable tier costs -----------------------------")
    hh = report.by_key("hostis")
    for label, key in (
        ("with no prior designation and no return ever offered",
         "without_prior_designation_authorized"),
        ("on its predecessor's evidence, recycled whole", "on_recycled_evidence_authorized"),
        ("at the ordinary quorum", "at_ordinary_quorum_authorized"),
        ("on the anathema tier's two cycles of consideration", "on_two_cycles_authorized"),
        ("with no re-examination date", "without_reexamination_authorized"),
    ):
        print(f"  reachable {label}: {'yes' if hh.metrics[key] else 'no'}")
    print(f"  and with all five satisfied: "
          f"{'authorized' if hh.metrics['designation_authorized'] else 'refused'}")
    print(f"  the way back: {hh.metrics['reentry_refusal']}")

    print("\n--- Did the compact agree? ----------------------------------------")
    c = report.by_key("cohesion").metrics["consensus"]
    print(f"Of a roll of {c.roll_size}, {len(c.deliberating)} designated, "
          f"{len(c.dissenting)} refused on the record, and {len(c.silent)} said nothing. "
          f"Contested: {c.contested}. Consensus share: {c.consensus_share:.0%}.")
    print("The two refusals vetoed nothing: the designation stands and the expulsion "
          "followed. What they change is that a later reader can see the split.")

    print("\n--- The negative control ------------------------------------------")
    m = report.by_key("cohesion").metrics
    print(f"A compact that spent a year prosecuting one real adversary: "
          f"{m['enforcement_volume']} enforcement acts, "
          f"{m['enforcement_coverage']:.0%} of the shared taxonomy cited, "
          f"target dispersion {m['target_dispersion']}, "
          f"aimed at {m['subjects_enforced_against']} subject. "
          f"Erosion reading: {m['erosion_severity']}.")
    print("Narrow ground and high volume are exactly the substitution signature. What "
          "spares this compact is that legitimate concentration lands on a target, "
          "while substitution concentrates a ground across many targets.")

    print(f"\n--- Still unanswered ({len(report.unanswered)}), all of it by design ---")
    for item in report.unanswered:
        print(f"  - {item}")

    assert report.all_passed, "every phase must pass"
    assert report.reached_tier == "HostisHumaniGeneris"
    print("\n✓ adversary_campaign_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
