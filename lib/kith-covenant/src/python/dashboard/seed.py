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

"""Seed a single signed audit commons that exercises every capture-signal lens.

Everything below is authored through the public API and crypto-verified; the
dashboard renders the resulting trends and the mediator pool adjudicates. One
archive drives all six panels:

* composition-drift and sponsor-stake  -- a hub vouches for two entrants
* feature-concern                      -- Concerns filed against the drifter
* rejection-pattern                    -- a gatekeeper rejects two applicants
* ceremony-cost-decay                  -- a community whose rites hollow out
                                          (witness count 5 -> 3 -> 1)
* standing                             -- a designated-anathema community
"""
from __future__ import annotations

from dataclasses import dataclass

from kith_covenant import (
    Archive,
    designate_anathema,
    new_identity,
    now_seconds,
    record_concern,
    record_conduct,
    record_non_admission,
    run_intra_rite,
    vouch,
)

DAY = 86_400


@dataclass(frozen=True)
class SeededCommons:
    """The archive plus the DIDs the dashboard defaults its per-actor panels to."""

    archive: Archive
    hub_did: str
    drifting_did: str
    healthy_did: str
    gatekeeper_did: str
    reaffirmer_did: str
    designated_did: str
    size: int


def build_commons() -> SeededCommons:
    arch = Archive()
    now = now_seconds()

    # --- Sponsorship structure: composition-drift + sponsor-stake ----------
    hub = new_identity(arch, "Hub")
    drifting = new_identity(arch, "Drifting")
    healthy = new_identity(arch, "Healthy")

    probation_until = now + 300 * DAY
    sponsored_at = now - 30 * DAY
    for entrant in (drifting, healthy):
        vouch(arch, hub, entrant.did, probation_until=probation_until,
              sponsored_at=sponsored_at, statement="vouched by the hub")

    # --- Feature-concern: the behavioral shadow of a hollow substrate ------
    recorded_at = now - DAY
    for feature in ("NonUltimacyOfCompact", "EnemyLoveOperational"):
        record_concern(
            arch, hub, drifting.did, feature=feature,
            perspective="charter-originalist observer",
            narrative=f"{feature}: decided a matter its charter reserves",
            recorded_at=recorded_at, observed_at=recorded_at,
        )
    record_conduct(
        arch, hub, healthy.did, "Commendation", "AntiSpiral",
        feature="GraceOverMerit",
        narrative="restored a lapsed member rather than expelling",
        recorded_at=recorded_at, observed_at=recorded_at,
    )

    # --- Rejection-pattern: a gatekeeper concentrating non-admissions -------
    gatekeeper = new_identity(arch, "Gatekeeper")
    applicant_b = new_identity(arch, "Applicant-B")
    applicant_c = new_identity(arch, "Applicant-C")
    record_non_admission(
        arch, [gatekeeper, applicant_b], [gatekeeper.did, applicant_b.did],
        [(gatekeeper.did, applicant_b.did, "FallenActorAnthropology", "insufficient")],
        decided_at=now - 20 * DAY,
    )
    record_non_admission(
        arch, [gatekeeper, applicant_c], [gatekeeper.did, applicant_c.did],
        [(gatekeeper.did, applicant_c.did, "FallenActorAnthropology", "insufficient")],
        decided_at=now - 15 * DAY,
    )

    # --- Ceremony-cost decay: re-affirmation rites hollowing out ------------
    reaffirmer = new_identity(arch, "Reaffirmer")
    for wc, offset in ((5, 90), (3, 60), (1, 20)):
        run_intra_rite(arch, reaffirmer, witness_count=wc, sealed_at=now - offset * DAY)

    # --- Standing: a designated-anathema community --------------------------
    designated = new_identity(arch, "Designated")
    designate_anathema(arch, [gatekeeper, reaffirmer], designated.did,
                       narrative="acted as hostis")

    # Nothing above was trusted on append; verify the whole commons at once.
    arch.verify_all()      # crypto integrity
    arch.check_authority()  # principal-binding + scope + quorum

    return SeededCommons(
        archive=arch,
        hub_did=hub.did,
        drifting_did=drifting.did,
        healthy_did=healthy.did,
        gatekeeper_did=gatekeeper.did,
        reaffirmer_did=reaffirmer.did,
        designated_did=designated.did,
        size=len(arch),
    )


if __name__ == "__main__":
    c = build_commons()
    print(f"Seeded + verified commons: {c.size} entries")
    for name in ("hub", "drifting", "healthy", "gatekeeper", "reaffirmer", "designated"):
        print(f"  {name:>11}: {getattr(c, name + '_did')}")
