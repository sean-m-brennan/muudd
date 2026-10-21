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

"""Author a signed audit commons from Python, then read the capture signals.

`capture_signals_demo.py` reads a fixed scenario baked into Rust. This demo
shows the other half: a real operator holding real keys *writes* that same
commons through the `authoring` surface — publish an identity, vouch for
entrants, file anti-spiral `Concern` records — and only then verifies and reads
it. The archive it builds is not a replay; it reproduces every capture signal
from first principles, which is the proof that the write path and the read path
agree.

Signing happens in the Rust core; each `Signer` carries the raw key bytes the
operator is responsible for. Records are not trusted on append — the signer's
identity must already be published — so the whole commons is crypto-verified
with a single `verify_all()` once authoring is complete.
"""
from __future__ import annotations

import sys

from kith_covenant import (
    Archive, composition_graph, delegate_speaker, feature_concern_profile,
    new_identity, now_seconds, record_concern, record_conduct, reputation_of,
    sponsor_stake, vouch,
)

DAY = 86400


def main() -> int:
    arch = Archive()

    # Each operator generates a keypair and holds it. new_identity publishes the
    # self-signed community identity so later records signed by these keys resolve.
    hub = new_identity(arch, "Hub")
    drifting = new_identity(arch, "Drifting")
    healthy = new_identity(arch, "Healthy")
    print("Published 3 community identities (operators hold their own keys):")
    for label, s in (("hub", hub), ("drifting", drifting), ("healthy", healthy)):
        print(f"  {label:>8}: {s.did[:32]}…  key=<held by operator>")

    now = now_seconds()
    probation_until = now + 300 * DAY
    sponsored_at = now - 30 * DAY

    # The hub vouches for both entrants — staking its standing on their conduct.
    for entrant in (drifting, healthy):
        vouch(arch, hub, entrant.did, probation_until=probation_until,
              sponsored_at=sponsored_at, statement="vouched by the hub")
    print("\nHub vouched for 2 entrants (sponsor stakes its standing on them).")

    # The hub, as a charter-originalist observer, files two pre-indictment
    # Concerns against the drifting entrant — signed, attributable, no ladder rung.
    recorded_at = now - DAY
    for feature in ("NonUltimacyOfCompact", "EnemyLoveOperational"):
        record_concern(
            arch, hub, drifting.did, feature=feature,
            perspective="charter-originalist observer",
            narrative=f"{feature}: decided a matter its charter reserves",
            recorded_at=recorded_at, observed_at=recorded_at,
        )
    # …and commends the healthy entrant (a commendation is not adverse).
    record_conduct(
        arch, hub, healthy.did, "Commendation", "AntiSpiral",
        feature="GraceOverMerit",
        narrative="restored a lapsed member rather than expelling",
        recorded_at=recorded_at, observed_at=recorded_at,
    )
    print("Filed 2 anti-spiral Concerns + 1 Commendation.\n")

    # Nothing above was trusted on append; verify the whole commons at once.
    arch.verify_all()
    print(f"Authored audit commons: {len(arch)} entries, all verify OK\n")

    print("Composition-drift lens (who is concentrating admissions):")
    for sponsor, degree in composition_graph(arch).by_concentration:
        tag = "  <-- hub" if sponsor == hub.did else ""
        print(f"  {sponsor[:24]}… sponsored {degree}{tag}")

    print("\nSponsor-stake lens (does this sponsor's vouching precede misconduct):")
    stake = sponsor_stake(arch, hub.did)
    print(f"  sponsees={stake.total_sponsees} tainted={stake.tainted_sponsees} "
          f"taint_rate={stake.taint_rate}")
    for o in stake.sponsees:
        print(f"    {o.entrant[:24]}…: {len(o.adverse_conduct)} adverse-in-probation")

    print("\nFeature-concern lens (behavioral shadow of a hollow substrate):")
    prof = feature_concern_profile(arch, drifting.did)
    for feature, count in prof.per_feature:
        print(f"    {feature}: {count} adverse")
    print(f"    declared perspectives: {list(prof.perspectives)}")
    print(f"    evidence CIDs: {len(prof.evidence)} (every count is traceable)")

    # The pre-indictment flags are real records but carry no ladder rung.
    view = reputation_of(arch, drifting.did)
    assert all(c.ladder_rung is None for c in view.conduct)

    # Delegated authority: the hub need not sign with its root key. It can
    # authorize an envoy, and records the envoy signs are attributed to the hub
    # yet verify through the published delegation chain.
    print("\nDelegated signing (envoy signs on behalf of the hub):")
    envoy = delegate_speaker(arch, hub, "Envoy", issued_at=recorded_at - DAY)
    print(f"  envoy key:  {envoy.did[:32]}…")
    print(f"  speaks for: {envoy.principal[:32]}…  (== hub)")
    record_concern(
        arch, envoy, drifting.did, feature="ScapegoatDisarming",
        perspective="charter-originalist observer",
        narrative="ScapegoatDisarming: singled out a member under pressure",
        recorded_at=recorded_at, observed_at=recorded_at,
    )
    arch.verify_all()  # the envoy-signed record resolves via the delegation
    prof = feature_concern_profile(arch, drifting.did)
    print(f"  after envoy's Concern, drifting entrant's feature concerns: "
          f"{dict(prof.per_feature)}")
    assert prof.per_feature  # the envoy's flag landed, attributed to the hub

    print("\n✓ authoring_demo (Python): SUCCESS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
