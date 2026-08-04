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

"""Demonstrate the capture-signal detectors over a signed audit commons.

Builds the demo scenario (a hub sponsors two entrants and files two anti-spiral
`Concern` records against one of them), reloads and crypto-verifies the archive,
then runs the jester lenses. Each lens surfaces flags with provenance and
renders no verdict — adjudication is the mediator pool's job.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, composition_graph, demo_capture_scenario, deliberative_capacity,
    record_admission, vouch,
    feature_concern_profile, reputation_of, sponsor_stake,
    rejection_pattern, ceremony_cost_decay,
    new_identity, run_intra_rite, record_non_admission, designate_anathema,
    now_seconds,
)


def main() -> int:
    s = demo_capture_scenario()
    arch = Archive.load_json(s.archive_json)
    arch.verify_all()
    print(f"Signed audit commons: {s.archive_size} entries, all verify OK\n")

    print("Composition-drift lens (who is concentrating admissions):")
    graph = composition_graph(arch)
    for sponsor, degree in graph.by_concentration:
        tag = "  <-- hub" if sponsor == s.hub_did else ""
        print(f"  {sponsor[:24]}… sponsored {degree}{tag}")

    print("\nSponsor-stake lens (does this sponsor's vouching precede misconduct):")
    stake = sponsor_stake(arch, s.hub_did)
    print(f"  sponsees={stake.total_sponsees} tainted={stake.tainted_sponsees} "
          f"taint_rate={stake.taint_rate}")
    for o in stake.sponsees:
        print(f"    {o.entrant[:24]}…: {len(o.adverse_conduct)} adverse-in-probation")

    print("\nFeature-concern lens (behavioral shadow of a hollow substrate):")
    prof = feature_concern_profile(arch, s.drifting_entrant_did)
    for feature, count in prof.per_feature:
        print(f"    {feature}: {count} adverse")
    print(f"    declared perspectives: {list(prof.perspectives)}")
    print(f"    evidence CIDs: {len(prof.evidence)} (every count is traceable)")

    # The jester's pre-indictment flags are real records but carry no ladder rung.
    view = reputation_of(arch, s.drifting_entrant_did)
    assert all(c.ladder_rung is None for c in view.conduct)

    # v0 jester lenses over REAL authored records ----------------------------
    # A second, purpose-built commons: three communities that reject newcomers,
    # a community whose re-affirmation ceremonies decay, and one designated
    # anathema. Everything below is authored through the public API and
    # crypto-verified.
    live = Archive()
    a = new_identity(live, "Gatekeeper-A")
    b = new_identity(live, "Applicant-B")
    c = new_identity(live, "Applicant-C")
    # Both parties to the credentialing ceremony co-sign its outcome.
    record_non_admission(live, [a, b], [a.did, b.did],
                         [(a.did, b.did, "FallenActorAnthropology", "insufficient")],
                         decided_at=1000)
    record_non_admission(live, [a, c], [a.did, c.did],
                         [(a.did, c.did, "FallenActorAnthropology", "insufficient")],
                         decided_at=1100)

    reaffirmer = new_identity(live, "Reaffirmer")
    for wc, at in ((5, 2000), (3, 2100), (1, 2300)):
        run_intra_rite(live, reaffirmer, witness_count=wc, sealed_at=at)

    designated = new_identity(live, "Designated")
    # Anathema is a second-class decision: it needs a distinct-community quorum,
    # so two communities co-sign the designation.
    designate_anathema(live, [a, reaffirmer], designated.did,
                       narrative="acted as hostis")
    live.verify_all()          # crypto integrity
    live.check_authority()     # §5: principal-binding + scope + quorum

    print("\nRejection-pattern lens (admissions integrity):")
    rej = rejection_pattern(live)
    for who, n in rej.by_concentration:
        print(f"    {who[:24]}… issued {n} rejection(s)")
    print(f"    by feature: {dict(rej.by_feature)}")

    print("\nCeremony-cost-decay lens (Reaffirmer):")
    decay = ceremony_cost_decay(live, reaffirmer.did)
    print(f"    rites={decay.rite_count} "
          f"witnesses first→last={decay.first_witness_count}→{decay.last_witness_count} "
          f"declined={decay.witness_count_declined} widening={decay.latest_gap_widening}")

    # §2 deliberative-capacity lens: what the compact can still decide -------
    # A compact of three seasons its members, then admits two of them into a new
    # probation window all at once. Nobody leaves the roll — but the weight that
    # validates a second-class decision does.
    grown = Archive()
    founders = [new_identity(grown, f"Founder-{i}") for i in range(3)]
    record_admission(grown, founders, [m.did for m in founders],
                     compact_label="demo-compact", witnessed_at=1000)
    print("\nDeliberative-capacity lens (§2 — what the compact can still decide):")
    before = deliberative_capacity(grown, "demo-compact", as_of=1100)
    print(f"    seasoned: roll={len(before.members)} standing={before.standing} "
          f"share={before.standing_share:.2f} "
          f"can_act={before.can_act} can_contract={before.can_contract}")
    for m in founders[1:]:
        vouch(grown, founders[0], m.did, probation_until=2_000_000, sponsored_at=1200)
    after = deliberative_capacity(grown, "demo-compact", as_of=1300)
    print(f"    after admitting a cohort: roll={len(after.members)} "
          f"standing={after.standing} share={after.standing_share:.2f} "
          f"can_act={after.can_act} can_contract={after.can_contract}")
    print("    nobody left the roll; the compact simply cannot act until a window "
          "closes —")
    print("    the honest consequence of the withholding, reported rather than "
          "met as a refusal")
    recovered = deliberative_capacity(grown, "demo-compact", as_of=2_000_001)
    print(f"    once the windows close: standing={recovered.standing} "
          f"can_act={recovered.can_act} can_contract={recovered.can_contract}")

    print("\nStanding derivation:")
    print(f"    Designated community standing: "
          f"{reputation_of(live, designated.did).standing}")

    print("\n✓ capture_signals_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
