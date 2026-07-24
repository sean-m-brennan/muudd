#!/usr/bin/env python
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

"""A polity's life, end to end, through the Python surface.

Founding (from an emergent boundary, ratified), the roll growing, the substrate
being transmitted to the new member, an office conferred, affirmations aging, the
coherence monitor grading the drift, and finally the handoff to kith-covenant.

    python src/python/examples/polity_demo.py
"""
from __future__ import annotations

import ethne as e
from ethne.authoring import (
    Signer,
    admit_member,
    confer_office,
    induct_member,
    ratify_charter,
    reaffirm_member,
)
from ethne.monitor import cohesion_report, founding_candidate, manual_founding
from ethne.uplift import coherence_concern, community_identity, concern_dict, identity_dict

DAY = 86_400


def short(did: str) -> str:
    return did[:16] + "…"


def main() -> None:
    # A convener and two others have been cooperating; the substrate provider (here
    # the manual one — no trust service involved) reads them as the boundary.
    founders = [Signer.generate() for _ in range(3)]
    polity_priv, polity_pub, polity_did = e.generate_keypair()
    t0 = e.now_seconds() - 400 * DAY

    candidate = manual_founding(
        [s.did for s in founders],
        "tradition",
        "answers to what it did not author, and did not vote for",
    )
    print(f"substrate provider reads {len(candidate.members)} candidate members")

    # Emergence proposes; the polity ratifies. The proposal carries no authority
    # until a majority of the roll co-signs it.
    proposal = e.propose_from_candidate(
        candidate,
        polity_did,
        commitments=[("mutual-aid", "carry each other's load")],
        min_reputation=0.5,
        proposed_at=t0,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, founders[:2], founded_at=t0 + DAY)
    print(f"chartered {short(polity_did)} — ratified by {e.majority(3)} of 3")

    # A newcomer joins the roll, and is then inducted into the substrate: two
    # distinct acts. Between them they are a member who has not received anything.
    joiner = Signer.generate()
    admit_member(archive, polity_did, joiner, founders[:2], decided_at=t0 + 100 * DAY)
    af = archive.affirmation_state(polity_did, as_of=t0 + 101 * DAY)
    print(f"admitted {short(joiner.did)} — inducted? {af.is_inducted(joiner.did)}")

    induct_member(archive, polity_did, joiner, founders, affirmed_at=t0 + 102 * DAY)
    af = archive.affirmation_state(polity_did, as_of=t0 + 103 * DAY)
    print(f"inducted, witnessed by {len(founders)} of the roll of 4 — "
          f"inducted? {af.is_inducted(joiner.did)}")

    # The roll — never an office — confers the first offices.
    confer_office(archive, polity_did, "Convener", founders[0], founders[:3],
                  decided_at=t0 + 110 * DAY)
    offices = archive.office_state(polity_did, as_of=t0 + 111 * DAY)
    print(f"offices: {offices.holdings}")

    # Two members renew; the rest have not affirmed since the founding.
    now = t0 + 380 * DAY
    for member in (founders[0], joiner):
        reaffirm_member(archive, polity_did, member, affirmed_at=now - 10 * DAY)

    archive.verify_all()
    archive.check_authority()
    print(f"log: {len(archive)} entries, signatures verified, "
          f"{len(archive.authority_violations())} authority violations")

    state = archive.charter_state(polity_did, as_of=now)
    print(f"roll of {len(state.members)}, commitments in force: "
          f"{[k for k, _ in state.commitments]}")
    stale = archive.affirmation_state(polity_did, as_of=now).stale_members(
        cadence_seconds=180 * DAY, as_of=now
    )
    print(f"{len(stale)} member(s) overdue on a 180-day re-affirmation cadence")

    # Time passes and the substrate shows only two of the four still cooperating.
    thinned = founding_candidate([(founders[0].did, 0.8, None), (joiner.did, 0.7, None)])
    report = cohesion_report(state.members, thinned)
    print(f"cohesion: {report.observed}/{report.chartered} observed, "
          f"attrition {report.attrition:.0%}, erosion {report.erosion}")

    # The handoff, and — only at Concern — the one signal that crosses to KC.
    identity = community_identity(
        archive, polity_did, "Ourania", polity_priv, polity_pub, created_at=now
    )
    print(f"handoff: CommunityIdentity for {identity_dict(identity)['payload']['name']}")

    concern = concern_dict(coherence_concern(report, polity_did, founders[0], at=now))
    if concern is None:
        print("no Concern crossed the boundary — the strain is the polity's own business")
    else:
        print(f"Concern filed: {concern['payload']['bearing']} "
              f"({concern['payload']['reporter_perspective']})")


if __name__ == "__main__":
    main()
