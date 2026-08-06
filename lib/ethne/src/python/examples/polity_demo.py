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

import json
import os
import time

import ethne as e
from ethne.authoring import (
    Signer,
    attest_observer_independence,
    designate_derived,
    attest_reader_independence,
    disavow_substrate,
    dissent_boundary,
    dissolve_charter,
    prune_observations,
    admit_member,
    record_observation,
    authorize_envoy,
    confer_office,
    induct_member,
    perform_rite,
    MOVEMENTS,
    ratify_charter,
    reaffirm_member,
    rotate_guardian,
)
from ethne.host import HostConfig, HostCycle
from ethne.monitor import (
    cohesion_report,
    founding_candidate,
    manual_founding,
    propose_from_candidate,
    select_from_candidate,
)
from ethne.uplift import (
    fission,
    coherence_concern,
    community_identity,
    concern_dict,
    delegation_dict,
    envoy_delegation,
    identity_dict,
)

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
    # All three ratify. Since D29 ratifying *is* a founding member's induction, so a
    # bare-majority founding would leave a member on the roll holding no substrate —
    # legal, and a different demo.
    ratify_charter(archive, proposal, founders, founded_at=t0 + DAY)
    print(f"chartered {short(polity_did)} — ratified by all 3, "
          "which is what inducts them")

    # The roll — never an office — confers the offices. Conferring an Inductor is
    # not decoration: from here the induction rite needs them among its witnesses,
    # or a higher bar from the roll itself (D14).
    confer_office(archive, polity_did, "Inductor", founders[1], founders[:2],
                  decided_at=t0 + 95 * DAY)

    # A newcomer joins the roll, and is then inducted into the substrate: two
    # distinct acts. Between them they are a member who has not received anything.
    joiner = Signer.generate()
    admit_member(archive, polity_did, joiner, founders[:2], decided_at=t0 + 100 * DAY)
    af = archive.affirmation_state(polity_did, as_of=t0 + 101 * DAY)
    print(f"admitted {short(joiner.did)} — inducted? {af.is_inducted(joiner.did)}")

    # The rite itself, movement by movement (D29). Each is a signed record with its
    # own instant, so the log shows the rite happening rather than only its result —
    # and only those present for every movement may witness the completion.
    # The rite is read off the charter, never hardcoded: movement 1 must name the
    # substrate the polity actually chartered and movement 4 the commitments actually
    # in force, or the rite is refused. (Writing this demo tripped exactly that rule.)
    chartered = archive.charter_state(polity_did, as_of=t0 + 101 * DAY)
    movements = perform_rite(
        archive, polity_did, joiner, founders,
        substrate_name=chartered.substrate_name,
        commitments=[k for k, _ in chartered.commitments],
        start=t0 + 101 * DAY, seconds_between=DAY,
    )
    print(f"rite: {len(movements)} movements recited over "
          f"{len(MOVEMENTS)} days — {', '.join(MOVEMENTS)}")

    induct_member(archive, polity_did, joiner, founders, movements,
                  affirmed_at=t0 + 107 * DAY)
    af = archive.affirmation_state(polity_did, as_of=t0 + 108 * DAY)
    offices = archive.office_state(polity_did, as_of=t0 + 108 * DAY)
    print(f"inducted, witnessed by {len(founders)} of the roll of 4 "
          f"(the Inductor {short(offices.holders('Inductor')[0])} among them) — "
          f"inducted? {af.is_inducted(joiner.did)}")
    rites = archive.rite_state(polity_did, as_of=t0 + 108 * DAY)
    for member, recited, span, completed, throughout, attrition in rites.rites:
        print(f"  rite for {short(member)}: {recited} movements over "
              f"{span // DAY if span else 0}d, attended throughout by "
              f"{len(throughout)}, lost {attrition}, "
              f"completed? {completed is not None}")

    confer_office(archive, polity_did, "Convener", founders[0], founders[:3],
                  decided_at=t0 + 110 * DAY)
    confer_office(archive, polity_did, "EnvoyAuthorizer", founders[0], founders[:3],
                  decided_at=t0 + 111 * DAY)
    offices = archive.office_state(polity_did, as_of=t0 + 112 * DAY)
    print(f"offices: {[(o, short(h)) for o, h in offices.holdings]}")

    # The other kind of teeth: the EnvoyAuthorizer names the polity's voice on its
    # own signature, where the roll would otherwise need a majority. The envoy
    # co-signs — speaking for a community is a duty, so it is never conscripted.
    authorize_envoy(archive, polity_did, joiner, [founders[0]],
                    decided_at=t0 + 115 * DAY)
    envoys = archive.envoy_state(polity_did, as_of=t0 + 116 * DAY)
    print(f"envoy authorized by 1 officer where the roll would need "
          f"{e.majority(4)}: {[short(d) for d in envoys.envoys()]}")

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
    affirmations = archive.affirmation_state(polity_did, as_of=now)
    stale = affirmations.stale_members(cadence_seconds=180 * DAY, as_of=now)
    print(f"{len(stale)} member(s) overdue on a 180-day re-affirmation cadence")

    # Time passes and the substrate shows only two of the four still cooperating.
    # The monitor reads both sides: what the provider observes, and — via the same
    # affirmation state printed above — whether the substrate is still being carried.
    thinned = founding_candidate([(founders[0].did, 0.8, None, None, False), (joiner.did, 0.7, None, None, False)])
    # Through the host cycle, which is the only call that supplies **every** read-model
    # the archive can offer. `cohesion_report` takes seven optional readings and an
    # omitted one makes its component report *nothing*, so a hand-assembled grade is
    # silent in the direction of health. This demo passed one of the seven until D31,
    # and a polity whose chartered ground had withdrawn would have graded clean here.
    cycle = HostCycle(HostConfig(polity_did))
    report = cycle.grade(archive, thinned, as_of=now)
    print(f"cohesion: {report.observed}/{report.chartered} observed, "
          f"{report.rated_members} rated, "
          f"attrition {report.attrition:.0%}, "
          f"{report.stale_affirmations}/{report.affirming_members} lapsed on affirmation, "
          f"erosion {report.erosion}")
    by_hand = cohesion_report(state.members, thinned, affirmations=affirmations, as_of=now)
    print(f"  the same read with one of seven readings: erosion {by_hand.erosion}, "
          f"blind to a Concern it could not see: {by_hand.blind_to_concern} "
          f"({len(by_hand.omitted_readings)} readings omitted)")

    # D18: unrated is not failing. The SAME roll, once with no reputation reading at
    # all and once rated genuinely low, used to be indistinguishable — both landed
    # below the 0.3 failure floor and graded Concern, so a freshly founded polity
    # crossed to KC for being new.
    # Deliberately narrow, and now legibly so: these two grades differ in exactly one
    # thing, so they pass no readings at all rather than adding signals that would move
    # both. `omitted_readings` says as much on each of them.
    unrated = founding_candidate([(d, None, None, None, False) for d in state.members])
    low = founding_candidate([(d, 0.2, None, None, False) for d in state.members])
    u = cohesion_report(state.members, unrated, as_of=now)
    l = cohesion_report(state.members, low, as_of=now)
    print(f"unrated roll: {u.rated_members} rated of {u.observed} observed, "
          f"{u.reputation_floor_breaches} breach(es) — erosion {u.erosion} "
          "(capped: a young cohort and a provider that stopped rating look alike)")
    print(f"the same roll rated 0.2: {l.reputation_floor_breaches} breach(es) — "
          f"erosion {l.erosion} (unknown and failing now diverge)")

    # The handoff, and — only at Concern — the one signal that crosses to KC.
    identity = community_identity(
        archive, polity_did, "Ourania", polity_priv, polity_pub, created_at=now
    )
    print(f"handoff: CommunityIdentity for {identity_dict(identity)['payload']['name']}")

    # The fourth border artifact, and the only one an office produces.
    delegation = delegation_dict(envoy_delegation(
        archive, polity_did, joiner.did, polity_priv, polity_pub, issued_at=now, as_of=now
    ))
    print(f"envoy delegation: scope {delegation['payload']['scope']} for "
          f"{short(delegation['payload']['speaker_did'])}")

    concern = concern_dict(coherence_concern(report, polity_did, founders[0], at=now))
    if concern is None:
        print("no Concern crossed the boundary — the strain is the polity's own business")
    else:
        print(f"Concern filed: {concern['payload']['bearing']} "
              f"({concern['payload']['reporter_perspective']})")


def ratified_split() -> None:
    """A split into N successors, in one act (D25).

    The monitor never decides this — the community ratifies the successor foundings and
    this only emits the records. What D25 changed is arity and guards: a three-way split
    took two chained fissions before, which recorded an intermediate polity nobody ever
    lived in, and fission carried none of the guards its mirror image `fusion` had.
    """
    print("\n--- a ratified split, N ways at once (D25) -----------------------")
    t0 = e.now_seconds() - 5 * DAY
    now = t0 + DAY

    def polity(name: str):
        """Charter a polity and return (did, priv, its CommunityIdentity JSON)."""
        founders = [Signer.generate() for _ in range(3)]
        priv, pub, did = e.generate_keypair()
        arch = e.Archive()
        ratify_charter(
            arch,
            e.propose_charter(did, "tradition", "answers to what it did not author",
                              [s.did for s in founders], proposed_at=t0),
            founders[:2],
            founded_at=t0,
        )
        return did, priv, founders, community_identity(
            arch, did, name, priv, pub, created_at=t0
        ), arch

    did, priv, founders, identity, arch = polity("Ourania")
    heirs = [polity(n)[3] for n in ("Ourania-North", "Ourania-South", "Ourania-Deep")]

    split = fission(
        arch, did, heirs, priv,
        deciders=founders[:2], rationale="three readings, three peoples", at=now,
    )
    named = json.loads(split.dissolution)["payload"]["successors"]
    print(f"three ways in one act: {len(split.successors)} successors, "
          f"{len(split.links)} links, a dissolution naming {len(named)}")
    print("  (before D25 this was two chained fissions, and the intermediate polity "
          "was a step nobody lived in)")

    # The guards fission had none of. Each of these passed before D25.
    for label, args in (
        ("a split into one", [heirs[0]]),
        ("one successor named twice", [heirs[0], heirs[0]]),
        ("the polity as its own heir", [identity, heirs[0]]),
    ):
        try:
            fission(arch, did, args, priv, deciders=founders[:2], at=now)
            print(f"  UNREACHED: {label}")
        except Exception as ex:
            print(f"  {label}: {str(ex).split('ethne: ')[-1]}")

    # And the archive half, which is where it actually bites: en-uplift only EMITS,
    # while `check_authority` is what a consumer verifies. Before D25 nothing looked at
    # a dissolution's successors at all.
    probe = e.Archive.load_json(arch.dump_json())
    dissolve_charter(probe, did, founders[:2], rationale="into myself",
                     successors=[did], dissolved_at=now)
    print(f"  a hand-built dissolution naming itself: "
          f"{probe.authority_violations()[0][1]}")


def founding_boundary() -> None:
    """What a founding says about the cluster it came from (D21).

    A founding may narrow the emergent boundary — a group of five inside a
    thirty-node cluster may have every right to charter alone. What it may not do is
    narrow it *silently*. The charter carries the boundary it claims to ratify, and
    since the charter's core has no amendment path, it carries it forever.
    """
    print("\n--- the founding boundary: who consented (D21) --------------------")

    def cluster(n=30):
        nodes = [Signer.generate() for _ in range(n)]
        _priv, _pub, did = e.generate_keypair()
        cand = founding_candidate(
            [(s.did, 0.9, None, None, False) for s in nodes],
            substrate_name="tradition",
            substrate_statement="answers to what it did not author",
        )
        return nodes, did, cand

    t0 = e.now_seconds() - 10 * DAY
    print("the substrate shows a cluster of 30.\n")
    print(f"{'faction':>8} {'signed':>7} {'saw':>5} {'named':>6} {'consent':>8}  basis")

    for n in (30, 5, 3, 1):
        nodes, did, cand = cluster()
        faction = nodes[:n]
        # `select_from_candidate`: the DELIBERATE narrowing. Every member the
        # candidate showed and the roll does not name is recorded, with a reason.
        proposal = select_from_candidate(
            cand, did, [s.did for s in faction],
            "not of this founding", proposed_at=t0,
        )
        arch = e.Archive()
        sigs = e.majority(n)
        ratify_charter(arch, proposal, faction[:sigs], founded_at=t0 + DAY)
        arch.verify_all()
        arch.check_authority()
        leg = arch.founding_legitimacy(did)
        print(f"{n:>8} {leg.ratifiers:>7} {leg.candidate_size:>5} "
              f"{leg.excluded:>6} {leg.consent_share:>8.3f}  {leg.basis}")

    print("\nbefore D21 every row above read `chartered` and nothing more: the roll\n"
          "was its own denominator, so one node chartering thirty passed the majority\n"
          "bar at a single signature, and the other 29 were not merely outvoted —\n"
          "they were unrecorded.")

    # The accounting is GATED; the judgement is only GRADED. A proposal that claims
    # a boundary it does not account for is refused outright.
    nodes, did, _cand = cluster()
    bad = json.loads(e.propose_charter(
        did, "tradition", "answers to what it did not author",
        [nodes[0].did], proposed_at=t0,
    ))
    bad["attestation"]["candidate_size"] = 30   # claims 30, accounts for 1
    arch = e.Archive()
    ratify_charter(arch, json.dumps(bad), nodes[:1], founded_at=t0 + DAY)
    try:
        arch.check_authority()
        print("\nUNREACHED")
    except Exception as ex:
        print(f"\na charter claiming 30 and accounting for 1 is refused:\n  {ex}")

    # And the convener's own word keeps working — labelled as exactly that, forever.
    founders = [Signer.generate() for _ in range(3)]
    _priv, _pub, did = e.generate_keypair()
    manual = manual_founding([s.did for s in founders], "tradition",
                             "answers to what it did not author")
    arch = e.Archive()
    ratify_charter(arch, e.propose_from_candidate(manual, did, proposed_at=t0),
                   founders[:2], founded_at=t0 + DAY)
    leg = arch.founding_legitimacy(did)
    print(f"a convener-asserted founding: basis {leg.basis} (source {leg.source}) — "
          f"chartered,\n  and honest about the fact that nobody else attests this "
          "boundary. The same\n  honest chartered state as an unguarded machine's "
          "`guardian: None` (D15).")


def attributable_boundary() -> None:
    """Who says the boundary was real (D22, resolving Q5).

    D21 recorded *that* a boundary came from a reading. It could not say whose — so
    the label was an unsigned self-report, and one edit to a public field promoted a
    convener's bare assertion to a corroborated reading. D22 makes the `Substrate`
    arm carry its evidence.
    """
    print("\n--- who says the boundary was real (D22) -------------------------")
    t0 = e.now_seconds() - 10 * DAY

    def cluster(n=4):
        nodes = [Signer.generate() for _ in range(n)]
        _p, _q, did = e.generate_keypair()
        cand = founding_candidate(
            [(s.did, 0.9, None, None, False) for s in nodes],
            substrate_name="tradition",
            substrate_statement="answers to what it did not author",
        )
        return nodes, did, cand

    # 1. The forgery D21 could not stop. One field, no provider, nothing signed.
    nodes, did, cand = cluster()
    doc = json.loads(e.propose_charter(
        did, "tradition", "answers to what it did not author",
        [s.did for s in nodes], proposed_at=t0,
    ))
    doc["attestation"]["source"] = {"Substrate": {"readings": []}}
    arch = e.Archive()
    ratify_charter(arch, json.dumps(doc), nodes[:3], founded_at=t0 + DAY)
    try:
        arch.check_authority()
        print("UNREACHED")
    except Exception as ex:
        print(f"claiming a reading nobody took:\n  {ex}")

    # 2. With a signature it goes through — and the charter names who looked. The
    #    provider never signs: it holds no keys, and D20 already settled that
    #    signing is the host's.
    nodes, did, cand = cluster()
    reader = Signer.generate()
    reading = e.sign_reading(cand, did, reader.did, reader.priv, read_at=t0)
    arch = e.Archive()
    ratify_charter(
        arch,
        e.propose_from_candidate(cand, did, readings=[reading], proposed_at=t0),
        nodes[:3], founded_at=t0 + DAY,
    )
    arch.verify_all()
    arch.check_authority()
    leg = arch.founding_legitimacy(did)
    print(f"with a signed reading: basis {leg.basis}, read by {short(leg.readers[0])}")

    # 3. A reader who saw something else corroborates nothing — and says so forever.
    nodes, did, cand = cluster()
    other = founding_candidate(
        [(Signer.generate().did, 0.9, None, None, False) for _ in range(4)],
        substrate_name="tradition", substrate_statement="answers to what it did not author",
    )
    r_ok, r_no = Signer.generate(), Signer.generate()
    arch = e.Archive()
    ratify_charter(
        arch,
        e.propose_from_candidate(cand, did, proposed_at=t0, readings=[
            e.sign_reading(cand, did, r_ok.did, r_ok.priv, read_at=t0),
            e.sign_reading(other, did, r_no.did, r_no.priv, read_at=t0),
        ]),
        nodes[:3], founded_at=t0 + DAY,
    )
    arch.check_authority()
    leg = arch.founding_legitimacy(did)
    print(f"two readers, one disagreeing: {len(leg.readers)} read, "
          f"{len(leg.corroborating_readers)} corroborate, "
          f"{len(leg.dissenting_readers)} contradict the founding on its own charter")
    print("  (a lens, never a gate: did:key is free, so a captured reader can mint\n"
          "   its own corroborators. Independence attestation is the next slice.)")

    # 4. And the excluded answer back — the one thing that can contradict an
    #    understated boundary, since the archive holds no provider.
    nodes, did, cand = cluster(3)
    arch = e.Archive()
    ratify_charter(
        arch,
        e.propose_charter(did, "tradition", "answers to what it did not author",
                          [s.did for s in nodes], proposed_at=t0),
        nodes[:2], founded_at=t0 + DAY,
    )
    omitted = Signer.generate()
    dissent_boundary(arch, did, omitted, "Unnamed",
                     "I was in that cluster and the accounting never named me",
                     dissented_at=t0 + 2 * DAY)
    # Winding the polity up does not close this record.
    dissolve_charter(arch, did, nodes[:2], dissolved_at=t0 + 3 * DAY)
    later = Signer.generate()
    dissent_boundary(arch, did, later, "Unnamed", "nor me",
                     dissented_at=t0 + 9 * DAY)
    arch.verify_all()
    arch.check_authority()
    d = arch.founding_dissent(did, as_of=e.now_seconds())
    print(f"dissent filed: {len(d.unnamed)} say the accounting omitted them "
          f"({len(d)} records, one of them after dissolution —\n"
          "  the only entry class a winding-up does not close, because otherwise\n"
          "  founders silence the excluded by dissolving)")


def machine_polity() -> None:
    """A machine-majority polity: the guardian edge is chartered, not merely
    observed, and from that one fact both D15 rules follow."""
    print("\n--- a machine-majority polity ------------------------------------")
    nodes = [Signer.generate() for _ in range(3)]
    humans = [Signer.generate() for _ in range(2)]
    polity_priv, polity_pub, polity_did = e.generate_keypair()
    t0 = e.now_seconds() - 10 * DAY

    # Emergence proposes the guardian edges along with the boundary: all three are
    # machines (derived), two have a human behind them, the third has nobody. Since
    # D18 the candidate can SAY that third thing — `derived` is carried separately
    # from the guardian edge, so an unguarded machine is ratified as what it is.
    candidate = founding_candidate([
        (nodes[0].did, 0.9, humans[0].did, t0, True),
        (nodes[1].did, 0.9, humans[1].did, t0, True),
        (nodes[2].did, 0.9, None, None, True),
    ])
    # A monitor ran over the cluster before the founding, and the proposal carries what
    # it saw (D27). Founding edges are held to the same standard as every later change:
    # claiming one and offering nothing for it was a gate evasion, not a cosmetic gap —
    # the grounded rule turns on whether any edge is guarded, so one asserted edge used
    # to exempt a whole machine cluster from naming a ground. The monitor is outside the
    # roll on purpose; a founder observing its own polity is that same self-issued
    # exemption in two steps.
    monitor = Signer.generate()
    seen_at_founding = e.sign_observation(
        polity_did,
        monitor.did,
        monitor.priv,
        [(nodes[0].did, 0.9, humans[0].did, t0),
         (nodes[1].did, 0.9, humans[1].did, t0)],
        observed_at=t0 - 3600,
    )
    proposal = e.propose_from_candidate(
        candidate,
        polity_did,
        substrate_name="the guardians' ground",
        substrate_statement="answers to the humans whose standing its machines carry",
        commitments=[("answer-for-our-machines", "each node has a human")],
        observation=seen_at_founding,
        proposed_at=t0,
    )
    # Every named guardian co-signs the founding: answerability is accepted, never
    # assigned.
    ratify_charter(archive := e.Archive(), proposal, nodes, founded_at=t0 + DAY,
                   guardians=humans)
    edges = archive.guardian_state(polity_did, as_of=t0 + 2 * DAY)
    print(f"chartered {len(edges.derivations)} derived members, "
          f"{len(edges.guarded_members())} with a human, "
          f"{len(edges.unguarded_members())} answering to nobody — machine-majority? "
          f"{edges.is_machine_majority(3)}")
    print("  (all three straight from the provider's reading: before D18 the third "
          "node\n   proposed silently as a HUMAN, and reaching its true state needed "
          "a\n   deliberate designate_derived after founding)")
    print(f"  grounding={archive.founding_legitimacy(polity_did).grounding} — the "
          f"monitor's observation is what earns that;\n   unbacked edges read Bare and "
          f"a wholly machine roll must then name an anchor")

    # It may hold no office here. The refusal is about the missing human, not about the machine —
    # tried in a throwaway copy of the log, since a refused entry stays in it.
    refused = e.Archive.load_json(archive.dump_json())
    confer_office(refused, polity_did, "Convener", nodes[2], nodes[:2], decided_at=t0 + 4 * DAY)
    print("conferral on the unguarded machine: "
          f"{[m for _cid, m in refused.authority_violations()][0][:72]}…")

    # Give that node a human, and the identical conferral goes through. The edge has
    # to be *backed*: a monitor reports that this human now answers for the node, and
    # the change carries a copy of that reading (D15). Carried rather than cited, so a
    # later pruning of the observation log cannot un-authorize the edge.
    watcher = Signer.generate()
    seen = record_observation(
        archive, polity_did, watcher,
        [(nodes[2].did, 0.9, humans[0].did, t0 + 4 * DAY)],
        observed_at=t0 + 4 * DAY + 3600,
    )
    rotate_guardian(archive, polity_did, nodes[2].did, humans[0], nodes[:2],
                    evidence_cid=seen, decided_at=t0 + 5 * DAY)
    confer_office(archive, polity_did, "Convener", nodes[2], nodes[:2], decided_at=t0 + 6 * DAY)
    archive.verify_all()
    archive.check_authority()
    print("the same conferral, once a human answers for it: authorized")

    # An edge the charter records but the substrate no longer shows is erosion —
    # graded against what the polity ratified, not against a snapshot alone.
    later = t0 + 20 * DAY
    thinned = founding_candidate([
        (nodes[0].did, 0.9, humans[0].did, later, True),
        (nodes[1].did, 0.9, None, None, True),    # its human is simply gone
        (nodes[2].did, 0.9, None, None, True),
    ])
    roll = archive.charter_state(polity_did, as_of=later).members
    edges = archive.guardian_state(polity_did, as_of=later)
    report = cohesion_report(roll, thinned, guardians=edges, as_of=later)
    print(f"guardian edges: {report.guardian_edges_lost} of "
          f"{report.chartered_guardian_members} chartered edges lost — "
          f"erosion {report.erosion}")
    concern = concern_dict(coherence_concern(report, polity_did, nodes[0], at=later))
    print("Concern crossed to KC" if concern else "no Concern crossed the boundary")


def watched_absence() -> None:
    """The one absence signal that escalates. A snapshot cannot tell a night shift
    from an abandoned polity — so the attendance component is capped at Notice by
    construction. What lifts the cap is not a looser threshold but *evidence*: a log
    of what the polity's monitors actually saw, over time."""
    print("\n--- a watched absence -------------------------------------------")
    nodes = [Signer.generate() for _ in range(2)]
    human = Signer.generate()
    # Three monitors, none of them members — a monitor need not be one, and since
    # D28 the escalation counts distinct WATCHERS rather than filings: one key
    # reporting three times is one party's word, and stays capped at Notice.
    monitors = [Signer.generate() for _ in range(3)]
    polity_priv, polity_pub, polity_did = e.generate_keypair()
    t0 = e.now_seconds() - 10 * DAY

    proposal = e.propose_charter(
        polity_did, "the guardians' ground", "answers to the humans behind it",
        member_dids=[n.did for n in nodes], proposed_at=t0,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, nodes, founded_at=t0 + DAY)
    watched = nodes[0].did

    # One snapshot: the guardian is not present *now*. That is all a snapshot knows.
    roll = archive.charter_state(polity_did, as_of=t0 + 2 * DAY).members
    seen_now = founding_candidate([(n.did, 0.9, human.did, None, True) for n in nodes])
    snapshot = cohesion_report(roll, seen_now, as_of=t0 + 2 * DAY)
    print(f"snapshot alone: {snapshot.unattended} of {snapshot.guardian_members} "
          f"unattended — erosion {snapshot.erosion} (capped: a night shift and an "
          f"abandoned polity look alike)")

    # Three recorded observations across a day, the human present only in the first.
    base = t0 + 2 * DAY
    record_observation(archive, polity_did, monitors[0],
                       [(watched, 0.9, human.did, base)], observed_at=base)
    for i, offset in enumerate((DAY // 2, DAY)):
        record_observation(archive, polity_did, monitors[i + 1],
                           [(watched, 0.9, human.did, None)], observed_at=base + offset)
    archive.verify_all()
    archive.check_authority()

    later = base + DAY + 60
    w = archive.observation_window(polity_did, since=base, as_of=later)
    print(f"observed {w.reading_count()} times by {w.watchers_for(watched)} distinct "
          f"monitors over {w.span_seconds() // 3600}h; "
          f"unattended for {w.unattended_span(watched, as_of=later) // 3600}h")

    report = cohesion_report(roll, seen_now, window=w, as_of=later)
    print(f"with the record: {report.sustained_absences} sustained absence(s) — "
          f"erosion {report.erosion}")
    concern = concern_dict(coherence_concern(report, polity_did, nodes[0], at=later))
    print("Concern crossed to KC" if concern else "no Concern crossed the boundary")
    _ = polity_priv, polity_pub
    return archive, polity_did, nodes, watched, later


def pruning(archive, polity_did, nodes, watched, now):
    """A log that forgets, and says so.

    The observation record is the only one a polity may drop — it decides nothing,
    so losing it cannot rewrite what was resolved. What it *can* do is make an
    absence unmeasurable, so the dropping is itself a signed act with a receipt, and
    what survives is clamped so pruning can only ever weaken a reading.
    """
    print("\n--- pruning the observation log --------------------------------")
    keeper = nodes[0]
    roll = archive.charter_state(polity_did, as_of=now).members
    quorum = nodes[: e.majority(len(roll))]
    confer_office(archive, polity_did, "CharterKeeper", keeper, quorum, decided_at=now)
    print(f"CharterKeeper conferred on {keeper.did[:24]}…")

    # The custodian reaching for recent evidence: refused.
    speculative = e.Archive.load_json(archive.dump_json())
    prune_observations(speculative, polity_did, horizon=now - DAY,
                       deciders=[keeper], pruned_at=now)
    try:
        speculative.check_authority()
        print("  (unexpected: recent evidence was prunable by one officer)")
    except RuntimeError as err:
        print(f"  a keeper pruning yesterday: REFUSED — {str(err).split(',')[0]}")

    # A second monitor reports in, carrying a staler reading than the first
    # observation held. This is the case the clamp exists for.
    second = Signer.generate()
    first_seen = now - DAY - 60
    record_observation(archive, polity_did, second,
                       [(watched, 0.9, None, first_seen - 3600)], observed_at=now)
    before = archive.observation_window(polity_did, since=0, as_of=now)
    span_before = before.unattended_span(watched, as_of=now)

    # The same act by the roll, inside the settling period: allowed. The roll is
    # sovereign over its own log; the floor binds the officer route, not the polity.
    horizon = first_seen + 1
    prune_observations(archive, polity_did, horizon=horizon, deciders=quorum,
                       rationale="housekeeping", pruned_at=now)
    archive.check_authority()

    compacted = archive.apply_prunings(polity_did, as_of=now)
    dropped = len(archive) - len(compacted)
    kinds = [k for _cid, k in compacted.entries()]
    print(f"the roll pruned {dropped} observation(s); {len(kinds)} entries remain, "
          f"every governing one intact ({kinds.count('Observed')} observations left)")

    after = compacted.observation_window(polity_did, since=0, as_of=now)
    span_after = after.unattended_span(watched, as_of=now)
    unclamped = now - after.last_attested(watched)
    print(f"unattended span {span_before // 3600}h → {span_after // 3600}h; "
          f"unclamped it would read {unclamped // 3600}h")
    print("  the pruned interval held the fresher stamp, so dropping it would have "
          "LENGTHENED\n  the absence — pruning manufacturing a Concern. The clamp "
          "forbids it: a signal\n  may only ever weaken.")

    # Persistence: a host appends what is new instead of rewriting the whole log.
    cursor = e.Cursor(compacted.generation, len(compacted) - 1)
    segment = compacted.dump_json_since(cursor)
    print(f"incremental save: {len(segment)} bytes for the tail vs "
          f"{len(compacted.dump_json())} for the whole log")


def storage():
    """A log that survives a restart — including the prunings.

    The incremental save above is what a growing log needs, and it is also where a
    host can quietly destroy its own evidence. A cursor means nothing across a
    compaction: entries go from the *middle*, so the same index is a different entry,
    and an index past the new end used to read as "already current" — so the save
    wrote nothing, the pruning record never reached storage, and the observations it
    dropped were still sitting in the earlier segments. Reloading brought them back,
    and every check said the log was fine.

    `FileStore` owns the cursor so that cannot happen.
    """
    print("\n--- storage that survives a pruning ------------------------------")
    import tempfile

    nodes = [Signer.generate() for _ in range(3)]
    # Three monitors, rotated: since D28 the retention floor counts distinct
    # WATCHERS, so a lone monitor filing four hundred times is one watcher and pins
    # the horizon completely — nothing would be recommended at all.
    monitors = [Signer.generate() for _ in range(3)]
    polity_priv, polity_pub, polity_did = e.generate_keypair()
    now = e.now_seconds()
    t0 = now - 400 * DAY

    proposal = e.propose_charter(
        polity_did, "the kept record", "answers to what it did not author",
        member_dids=[n.did for n in nodes], proposed_at=t0,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, nodes[:2], founded_at=t0 + 60)
    keeper, watched = nodes[0], nodes[2].did
    confer_office(archive, polity_did, "CharterKeeper", keeper, nodes[:2], decided_at=t0 + 120)

    path = tempfile.mktemp(suffix=".log", prefix="ethne-demo-")
    store = e.FileStore(path)
    out = store.save(archive)
    print(f"founded and saved: {out.appended} entries, {out.bytes} bytes")

    # A year of observation, persisted incrementally as it arrives.
    appended = 0
    for i in range(120):
        record_observation(archive, polity_did, monitors[i % 3],
                           [(watched, 0.9, None, None)], observed_at=t0 + 200 + i * 3 * DAY)
        appended += store.save(archive).appended
    for i, at in enumerate((now - 3 * DAY, now - DAY, now - 3600)):
        record_observation(archive, polity_did, monitors[i % 3],
                           [(watched, 0.9, None, None)], observed_at=at)
        appended += store.save(archive).appended
    print(f"{appended} observations appended one at a time (never rewriting the log)")

    # Now prune, which is where a hand-rolled cursor goes wrong.
    rec = archive.prune_recommendation(polity_did, as_of=now)
    prune_observations(archive, polity_did, horizon=rec.horizon, deciders=[keeper],
                       rationale="retention policy", pruned_at=now)
    archive = archive.apply_prunings(polity_did, as_of=now)
    print(f"pruned {rec.dropped}; the log is now generation {archive.generation}")

    out = store.save(archive)
    print(f"the store noticed and REWROTE: {out.rewrote}, {out.appended} entries, "
          f"{out.bytes} bytes")

    # Restart.
    reloaded = e.FileStore(path).load()
    live_obs = [k for _c, k in archive.entries()].count("Observed")
    back_obs = [k for _c, k in reloaded.entries()].count("Observed")
    reloaded.verify_all()
    reloaded.check_authority()
    w = reloaded.observation_window(polity_did, since=0, as_of=now)
    print(f"reloaded: {len(reloaded)} entries, {back_obs} observations "
          f"(live has {live_obs}), generation {reloaded.generation}")
    print(f"  the clamp came back with it: pruned_before is set, span "
          f"{w.unattended_span(watched, as_of=now) // DAY}d")
    assert reloaded.cids() == archive.cids(), "storage must agree with the live log"
    print("  storage and the live log agree CID for CID — nothing resurrected")

    # And a stale cursor cannot be used by hand either.
    try:
        archive.dump_json_since(e.Cursor(0, 5))
        print("  (unexpected: a pre-compaction cursor was accepted)")
    except RuntimeError as err:
        print(f"  a cursor from before the pruning: REFUSED — "
              f"{str(err).split('—')[0].split(':')[-1].strip()}")
    os.remove(path)
    _ = polity_priv, polity_pub


def retention():
    """When to prune, and who can sign it — without clipping a live signal.

    The act of pruning is D17's; what a host still had to invent for itself was the
    *horizon*, and getting it wrong is silent. Too aggressive and it clips the
    evidence a Concern rests on; too timid and the log grows without bound. So the
    tier offers a read-model — it recommends, and the polity still ratifies.

    A policy, never a schedule: nothing here calls the clock. The loop belongs to the
    host, exactly as the storage backend does.
    """
    print("\n--- what pruning to run, and who may sign it ---------------------")
    nodes = [Signer.generate() for _ in range(3)]
    # Three monitors, rotated: since D28 the retention floor counts distinct
    # WATCHERS, so a lone monitor filing four hundred times is one watcher and pins
    # the horizon completely — nothing would be recommended at all.
    monitors = [Signer.generate() for _ in range(3)]
    polity_priv, polity_pub, polity_did = e.generate_keypair()
    now = e.now_seconds()
    t0 = now - 400 * DAY

    proposal = e.propose_charter(
        polity_did, "the long watch", "answers to what it did not author",
        member_dids=[n.did for n in nodes], proposed_at=t0,
    )
    archive = e.Archive()
    ratify_charter(archive, proposal, nodes[:2], founded_at=t0 + 60)
    keeper, watched = nodes[0], nodes[2].did
    confer_office(archive, polity_did, "CharterKeeper", keeper, nodes[:2],
                  decided_at=t0 + 120)

    # A year of watching, then a few readings inside the live window.
    for i in range(120):
        record_observation(archive, polity_did, monitors[i % 3],
                           [(watched, 0.9, None, None)], observed_at=t0 + 200 + i * 3 * DAY)
    for i, at in enumerate((now - 3 * DAY, now - DAY, now - 3600)):
        record_observation(archive, polity_did, monitors[i % 3],
                           [(watched, 0.9, None, None)], observed_at=at)
    archive.check_authority()
    print(f"{len(archive)} entries: a year of observation, kept because nothing "
          f"said to stop")

    # 1. Ask. The defaults are en-charter's own; the host carries no second copy.
    rec = archive.prune_recommendation(polity_did, as_of=now)
    print(f"recommendation: drop {rec.dropped} observations before "
          f"{(now - rec.horizon) // DAY}d ago, keeping "
          f"{rec.seconds_retained // DAY}d — routes {rec.routes}")
    print(f"  one CharterKeeper can carry it alone: {rec.keeper_may_authorize} "
          f"(otherwise {rec.roll_majority_need} of {len(nodes)} must co-sign)")

    # 2. The safe default is also the cheap one — a coincidence worth seeing.
    shorter = archive.prune_recommendation(polity_did, retain_seconds=30 * DAY - 1,
                                           as_of=now)
    print(f"  a second less retention and the office route closes: "
          f"{shorter.routes} — the settling floor is what the custody rests on")

    # 3. And retention can never be tuned below what the polity's own grading reads.
    #    Two floors, because a sustained absence is graded on a span AND on a witness
    #    count: here the three surviving WATCHERS are what holds the horizon back,
    #    past even the 24h the span alone would need.
    greedy = archive.prune_recommendation(polity_did, retain_seconds=60, as_of=now)
    span_only = archive.prune_recommendation(polity_did, retain_seconds=60,
                                             min_witness_watchers=0, as_of=now)
    print(f"  asked to keep 60s, it keeps {greedy.seconds_retained // 3600}h — "
          f"{span_only.seconds_retained // 3600}h for the span a sustained absence "
          f"reads back over,")
    print(f"  and the rest to leave 3 watchers standing. Prune to the span alone and "
          f"the witness\n  count falls below the grade: the Concern becomes a Notice "
          f"with nothing saying why.")

    # 4. Act on it — the officer route, at exactly the horizon returned.
    prune_observations(archive, polity_did, horizon=rec.horizon, deciders=[keeper],
                       rationale="retention policy", pruned_at=now)
    archive.verify_all()
    archive.check_authority()
    compacted = archive.apply_prunings(polity_did, as_of=now)
    print(f"authorized by one officer and accepted: {len(archive)} → "
          f"{len(compacted)} entries "
          f"({len(compacted.dump_json())} bytes, was {len(archive.dump_json())})")

    w = compacted.observation_window(polity_did, since=0, as_of=now)
    print(f"  {w.reading_count()} readings survive inside the retained window; "
          f"the log now says it forgot before {(now - w.pruned_before) // DAY}d ago")

    # 5. And it goes quiet, because a receding horizon would claim what is gone.
    print(f"asked again: {compacted.prune_recommendation(polity_did, as_of=now)} "
          f"— nothing further worth a permanent record")
    _ = polity_priv, polity_pub


def independent_readers() -> None:
    """Whether a founding's readers were their own principals (D24, closing the residue
    D22 named as its own successor).

    D22 made a boundary's reading attributable: signed by the reader it names, bound to
    that founding. What it could not do is tell one reader from five faces of the same
    actor — and `did:key` costs nothing, so the count of distinct signing readers cannot
    mean what it appears to mean.
    """
    print("\n--- were the readers their own principals? (D24) -----------------")
    t0 = e.now_seconds() - 10 * DAY
    now = t0 + 3 * DAY

    nodes = [Signer.generate() for _ in range(4)]
    _p, _q, did = e.generate_keypair()
    candidate = founding_candidate(
        [(s.did, 0.9, None, None, False) for s in nodes],
        substrate_name="tradition",
        substrate_statement="answers to what it did not author",
    )

    # One actor, a chorus of minted readers — and what it costs them.
    start = time.perf_counter()
    chorus = [Signer.generate() for _ in range(100)]
    readings = [
        e.sign_reading(candidate, did, r.did, r.priv, read_at=t0) for r in chorus
    ]
    minted = time.perf_counter() - start
    print(f"one actor minted {len(chorus)} readers and signed {len(readings)} identical "
          f"readings in {minted * 1000:.1f}ms ({minted / len(chorus) * 1e6:.0f}µs each)")

    arch = e.Archive()
    ratify_charter(
        arch,
        propose_from_candidate(candidate, did, readings=readings, proposed_at=t0),
        nodes[: e.majority(4)],
        founded_at=t0 + DAY,
    )
    arch.verify_all()
    arch.check_authority()

    leg = arch.founding_legitimacy(did)
    ic = arch.independent_corroboration(did, now)
    print(f"D22 reads: basis={leg.basis}, corroborating_readers="
          f"{len(leg.corroborating_readers)} — the highest grade the tier awards")
    print(f"D24 reads: independently corroborated by {len(ic)}")

    # A founding member vouching for the founding's own reader: the sockpuppet move one
    # step out. Recorded, because somebody signed their name to it — and not credited.
    attest_reader_independence(
        arch, did, chorus[0].did, nodes[0], basis="I know them", attested_at=t0 + 2 * DAY
    )
    ri = arch.reader_independence(did, now)
    print(f"a founder vouches: interested_attesters={len(ri.interested_attesters)}, "
          f"counted={len(ri.independent_readers)}, "
          f"independently corroborated={len(arch.independent_corroboration(did, now))}")

    # One outsider willing to be named for the claim.
    outsider = Signer.generate()
    attest_reader_independence(
        arch,
        did,
        chorus[0].did,
        outsider,
        basis="distinct operator, separate funding",
        attested_at=t0 + 2 * DAY,
    )
    arch.check_authority()
    ic = arch.independent_corroboration(did, now)
    print(f"one outsider vouches: independently corroborated={len(ic)} "
          f"(of {len(leg.corroborating_readers)} who agree)")

    # And somebody who thinks the lens is being fooled says so. Reported as contested,
    # never as disqualified — nothing here adjudicates.
    sceptic = Signer.generate()
    attest_reader_independence(
        arch,
        did,
        chorus[0].did,
        sceptic,
        kind="Controlled",
        basis="same hosting, same billing address",
        attested_at=t0 + 2 * DAY + 3600,
    )
    arch.check_authority()
    ic = arch.independent_corroboration(did, now)
    print(f"a sceptic contests it: counted={len(ic)}, contested={len(ic.contested)}, "
          f"uncontested={len(ic.uncontested)}")

    # A founder may contest even though it may not vouch: interest disqualifies praise,
    # not warning.
    attest_reader_independence(
        arch, did, chorus[1].did, nodes[1], kind="Controlled",
        basis="that one is ours", attested_at=t0 + 2 * DAY + 7200,
    )
    arch.check_authority()
    print("  a founder's WARNING counts where its praise did not: "
          f"contested readers={len(arch.reader_independence(did, now).contested)}")

    # Evidence about a founding outlives the polity — the third such class, and by now a
    # category rather than an exception.
    dissolve_charter(arch, did, nodes[:3], rationale="wound up", dissolved_at=now)
    late = Signer.generate()
    attest_reader_independence(
        arch, did, chorus[2].did, late, basis="filed afterwards", attested_at=now + DAY
    )
    arch.verify_all()
    arch.check_authority()
    print(f"filed after dissolution and still authorized: "
          f"{len(arch.reader_independence(did, now + 2 * DAY).attesters)} readers vouched for")


def standing_ground() -> None:
    """What the polity stands on (D23, resolving Q4).

    Q4 asked whether a purely emergent group — a fresh AT cluster of machines with no
    articulated tradition — can charter at all. Measured before the slice: it could,
    and so could a polity whose entire ground was two empty strings, or one that named
    ITSELF as the thing it defers to. Both verification passes were green, because
    `substrate.anchor` was read by no rule anywhere.
    """
    print("\n--- what the polity stands on (D23) ------------------------------")
    t0 = e.now_seconds() - 10 * DAY
    founded = t0 + DAY

    def machines(n=3):
        """A fresh cluster: n nodes, and the polity DID they might charter as."""
        nodes = [Signer.generate() for _ in range(n)]
        _p, _q, did = e.generate_keypair()
        return nodes, did

    def cluster_proposal(nodes, did, anchor=None):
        """Q4's exact case: every member a machine, nobody behind any of them."""
        return e.propose_charter(
            did,
            "the operators",
            "answers to what it did not author",
            member_dids=[s.did for s in nodes],
            substrate_anchor=anchor.did if anchor is not None else None,
            derivations=[(s.did, None) for s in nodes],
            proposed_at=t0,
        )

    def refusal(proposal, nodes, anchor=None):
        """Ratify, then report the single authority violation it earns."""
        arch = e.Archive()
        ratify_charter(arch, proposal, nodes[:2], founded_at=founded, anchor=anchor)
        arch.verify_all()  # the signatures are always fine; the ground is the problem
        return arch.authority_violations()[0][1]

    # 1. The blank ground.
    nodes, did = machines()
    blank = e.propose_charter(did, "", "", [s.did for s in nodes], proposed_at=t0)
    print(f"a ground of two empty strings:\n  {refusal(blank, nodes)}")

    # 2. A polity naming itself — the hole a presence-only rule would have let past.
    nodes, did = machines()
    circular = e.propose_charter(
        did, "us", "we answer to ourselves", [s.did for s in nodes],
        substrate_anchor=did, proposed_at=t0,
    )
    print(f"a polity as its own ground:\n  {refusal(circular, nodes)}")

    # 3. The fresh machine cluster with nothing above it.
    nodes, did = machines()
    print(f"a cluster of machines answering to nobody:"
          f"\n  {refusal(cluster_proposal(nodes, did), nodes)}")

    # 4. Named a ground without asking it. Answerability is accepted, never assigned,
    #    so naming a key you do not hold buys nothing.
    nodes, did = machines()
    ground = Signer.generate()
    grounded = cluster_proposal(nodes, did, anchor=ground)
    print(f"named a ground that did not consent:\n  {refusal(grounded, nodes)}")

    # 5. The same proposal, with the ground co-signing.
    arch = e.Archive()
    ratify_charter(arch, grounded, nodes[:2], founded_at=founded, anchor=ground)
    arch.verify_all()
    arch.check_authority()
    print(f"with the ground co-signing: chartered, "
          f"grounding={arch.founding_legitimacy(did).grounding}")

    # A human roll with no anchor and no machines is a different picture, and legal:
    # what it defers to may have no key at all.
    people, human_did = machines()
    human = e.Archive()
    ratify_charter(
        human,
        e.propose_charter(human_did, "tradition", "answers to what it did not author",
                          [s.did for s in people], proposed_at=t0),
        people[:2],
        founded_at=founded,
    )
    print(f"  a human roll holding an unkeyed tradition: "
          f"grounding={human.founding_legitimacy(human_did).grounding}")

    # 6. Nobody but the named ground may withdraw it.
    intruded = e.Archive()
    ratify_charter(intruded, grounded, nodes[:2], founded_at=founded, anchor=ground)
    disavow_substrate(intruded, did, Signer.generate(), "not mine to give up", founded + 1)
    print(f"someone else withdrawing it:\n  {intruded.authority_violations()[0][1]}")

    # 7. The ground changes its mind. It cannot un-charter the polity — the substrate
    #    is genesis-immutable — but the record says so permanently, and the monitor
    #    reads it as the Concern that crosses to kith-covenant.
    now = t0 + 3 * DAY
    roll = arch.charter_state(did, now).members
    candidate = manual_founding(
        [s.did for s in nodes], "the operators", "answers to what it did not author"
    )
    before = cohesion_report(
        roll, candidate, ground=arch.substrate_disavowal(did, now), as_of=now
    )
    disavow_substrate(arch, did, ground, "I do not stand above these people", t0 + 2 * DAY)
    after = cohesion_report(
        roll, candidate, ground=arch.substrate_disavowal(did, now), as_of=now
    )
    arch.check_authority()
    state = arch.charter_state(did, now)
    print(f"the ground withdraws: erosion {before.erosion} → {after.erosion}, "
          f"still chartered={state.chartered}, dissolved={state.dissolved}")
    print(f"  the charter still reads grounding="
          f"{arch.founding_legitimacy(did).grounding} — the founding WAS anchored, "
          f"and there is no record that edits it")

    # 8. And winding the polity up does not silence it: the second entry class a
    #    dissolution does not close.
    late = e.Archive()
    ratify_charter(late, grounded, nodes[:2], founded_at=founded, anchor=ground)
    dissolve_charter(late, did, nodes[:2], rationale="done", dissolved_at=t0 + 2 * DAY)
    disavow_substrate(late, did, ground, "and I never did", now)
    late.verify_all()
    late.check_authority()
    print(f"filed after dissolution and still authorized: "
          f"{late.substrate_disavowal(did, now).disavowed}")


def founding_edges() -> None:
    """What stood behind a founding guardian edge (D27).

    D15 made every *post-founding* guardian change carry its evidence. Founding
    derivations carried nothing, and that was a gate evasion rather than a cosmetic
    asymmetry: D23's grounded rule exempts a machine roll the moment any edge is
    guarded, so one asserted edge — a sentence, costing a confederate's signature —
    bought a whole cluster out of naming a ground.
    """
    print("\n--- what stood behind a founding edge (D27) ----------------------")
    t0 = e.now_seconds() - 10 * DAY
    founded = t0 + DAY

    def cluster(n=3):
        nodes = [Signer.generate() for _ in range(n)]
        _p, _q, did = e.generate_keypair()
        return nodes, did

    def charter(nodes, did, derivations, guardian, anchor=None):
        """Ratify a machine roll whose edges are described by `derivations`."""
        proposal = e.propose_charter(
            did,
            "the operators",
            "answers to what it did not author",
            member_dids=[s.did for s in nodes],
            substrate_anchor=anchor.did if anchor is not None else None,
            derivations=derivations,
            proposed_at=t0,
        )
        arch = e.Archive()
        ratify_charter(
            arch, proposal, nodes[:2], founded_at=founded,
            guardians=[guardian], anchor=anchor,
        )
        arch.verify_all()  # the signatures were never the problem
        return arch

    def refusal(arch):
        return arch.authority_violations()[0][1]

    def observation(did, nodes, guardian, observer):
        """One signed reading of the whole cluster, as a monitor would take it."""
        return e.sign_observation(
            did, observer.did, observer.priv,
            [(s.did, 0.9, guardian.did, None) for s in nodes],
            observed_at=t0 - 10,
        )

    # 1. An edge asserted with nothing offered for it — refused one record later in
    #    the same log since D15, and until now waved through at founding.
    nodes, did = cluster()
    guardian = Signer.generate()
    bare = charter(nodes, did, [(s.did, guardian.did) for s in nodes], guardian)
    print(f"an edge claimed and nothing offered:\n  {refusal(bare)}")

    # 2. THE MEASURED HOLE. Declared honestly as unobserved — which post-founding is
    #    sound, because there it can only ever weaken a record. Here it would buy the
    #    anchor exemption, so the asymmetry inverts and the claim earns nothing.
    nodes, did = cluster()
    unbacked = charter(
        nodes, did,
        [(s.did, guardian.did, None, "no monitor had run at founding") for s in nodes],
        guardian,
    )
    print(f"claimed, and honestly unobserved:\n  {refusal(unbacked)}")
    print(f"  grade: grounding={unbacked.founding_legitimacy(did).grounding} — before "
          f"this slice both passes were green and it read Guarded")

    # 3. The bootstrap path stays open: real guardians nobody has looked at yet, and
    #    a ground written down. This is the accepted cost of the rule.
    nodes, did = cluster()
    ground = Signer.generate()
    named = charter(
        nodes, did,
        [(s.did, guardian.did, None, "no monitor had run at founding") for s in nodes],
        guardian, anchor=ground,
    )
    named.check_authority()
    print(f"the same cluster naming a ground: chartered, "
          f"grounding={named.founding_legitimacy(did).grounding}")

    # 4. Measured while building the slice, and the reason the rule is about *who
    #    looked*: a founder signed a real observation of its own cluster's edges and
    #    the charter graded Guarded with no anchor, both passes green. That is the
    #    closure shape D23 refused for the anchor and D25 for a successor, reaching
    #    the evidence this time — a polity vouching for its own ground.
    nodes, did = cluster()
    inside = observation(did, nodes, guardian, nodes[0])
    self_seen = charter(
        nodes, did, [(s.did, guardian.did, inside, None) for s in nodes], guardian
    )
    print(f"a founder observing its own cluster:\n  {refusal(self_seen)}")
    print(f"  grade: grounding={self_seen.founding_legitimacy(did).grounding} — the "
          f"observation stays in the log, it just cannot credit the exemption")

    # 5. The control on that: same charter, same edges, only the observer differs.
    nodes, did = cluster()
    outside = observation(did, nodes, guardian, Signer.generate())
    watched = charter(
        nodes, did, [(s.did, guardian.did, outside, None) for s in nodes], guardian
    )
    watched.check_authority()
    print(f"an outside monitor looking at the same edges: chartered with no anchor, "
          f"grounding={watched.founding_legitimacy(did).grounding}")

    # 6. A guardian observing the edge it is about to accept counts, and that is a
    #    judgment call rather than an oversight. It is not on the roll, it co-signs
    #    either way, and what it asserts by observing is its own answerability — a
    #    burden taken up, not a credit awarded. Interest disqualifies praise, not an
    #    admission (D24).
    nodes, did = cluster()
    accepted = observation(did, nodes, guardian, guardian)
    owned = charter(
        nodes, did, [(s.did, guardian.did, accepted, None) for s in nodes], guardian
    )
    owned.check_authority()
    print(f"the guardian itself vouching for what it accepts: "
          f"grounding={owned.founding_legitimacy(did).grounding} — the keyed human "
          f"outside the roll IS the ground")


def who_was_watching() -> None:
    """Who was doing the watching, and who will say so (D28).

    D15 and D27 each closed a hole in guardian evidence and each named D24's
    independence attestation as the successor for what was left — that the evidence is
    still one observer's perspectival reading. Measured, the successor did not reach:
    its coherence gate demands the subject be a *reader of the boundary*, and an
    observer is not one. The same measurement found the sharper case, and it points
    OUTWARD: a stranger can manufacture the Concern that crosses to kith-covenant.
    """
    print("\n--- who was watching, and who will say so (D28) ------------------")
    now = e.now_seconds()
    t0 = now - 20 * DAY

    def polity_of(n=3):
        nodes = [Signer.generate() for _ in range(n)]
        _p, _q, did = e.generate_keypair()
        arch = e.Archive()
        ratify_charter(
            arch,
            e.propose_charter(did, "the long watch",
                              "answers to what it did not author",
                              member_dids=[s.did for s in nodes], proposed_at=t0),
            nodes[:2], founded_at=t0 + 60,
        )
        return arch, did, nodes

    def absence(arch, did, nodes, watchers, human):
        """A guardian unattended for ten days, watched three times by `watchers`."""
        machine = nodes[2].did
        designate_derived(arch, did, machine, nodes[:2], guardian=human,
                          unbacked_reason="demo: no monitor reading",
                          decided_at=t0 + 120)
        stale = now - 10 * DAY
        for i, at in enumerate((stale, now - 2 * DAY, now - DAY)):
            record_observation(arch, did, watchers[i % len(watchers)],
                               [(machine, 0.9, human.did, stale)], observed_at=at)
        arch.verify_all()
        arch.check_authority()
        w = arch.observation_window(did, since=0, as_of=now)
        roll = arch.charter_state(did, as_of=now).members
        cand = founding_candidate(
            [(d, 0.9, human.did if d == machine else None, None, d == machine)
             for d in roll])
        report = cohesion_report(
            roll, cand, window=w, as_of=now,
            observers=arch.independent_observation(did, as_of=now),
        )
        return machine, w, report

    # 1. THE MEASURED HOLE. One key, three filings, and nothing to do with the polity.
    arch, did, nodes = polity_of()
    solo = Signer.generate()
    human = Signer.generate()
    machine, w, report = absence(arch, did, nodes, [solo], human)
    print(f"one stranger's key, filing three times: readings={w.readings_for(machine)}, "
          f"watchers={w.watchers_for(machine)}")
    print(f"  sustained absences={report.sustained_absences}, erosion={report.erosion} "
          f"— before D28 this read 3 witnesses and crossed to KC as a Concern")

    # 2. The control: three keys really are three watchers, and the escalation is real.
    arch3, did3, nodes3 = polity_of()
    trio = [Signer.generate() for _ in range(3)]
    human3 = Signer.generate()
    machine3, w3, report3 = absence(arch3, did3, nodes3, trio, human3)
    print(f"three distinct monitors: watchers={w3.watchers_for(machine3)}, "
          f"sustained absences={report3.sustained_absences}, erosion={report3.erosion}")
    print(f"  watchers lens (distinct, vouched, contested): {report3.watchers}")

    # 3. D24's record could not speak about a monitor at all — the successor that did
    #    not reach. The generalized one can, and only about a monitor that has spoken.
    #    Filed against the first polity, which is done being read: an unauthorized
    #    record stays in the log, so the refusal has to be shown somewhere it can.
    silent = Signer.generate()
    attest_observer_independence(arch, did, silent.did, Signer.generate(),
                                 basis="I know them", attested_at=now)
    arch.verify_all()  # the signature was never the problem
    print(f"vouching for a monitor that has never spoken:\n  "
          f"{arch.authority_violations()[0][1]}")

    # 4. Vouching for one that has. An outsider counts; a founder does not.
    attest_observer_independence(arch3, did3, trio[0].did, nodes3[0],
                                 basis="one of us knows them", attested_at=now)
    attest_observer_independence(arch3, did3, trio[1].did, Signer.generate(),
                                 basis="separate operator, separate funding",
                                 attested_at=now)
    arch3.check_authority()
    ind = arch3.independent_observation(did3, as_of=now)
    print(f"a founder vouches for one monitor, an outsider for another: "
          f"vouched={len(ind.observers)}, unvouched={len(ind.unvouched)}")

    report3 = cohesion_report(
        arch3.charter_state(did3, as_of=now).members,
        founding_candidate([(d, 0.9, None, None, False)
                            for d in arch3.charter_state(did3, as_of=now).members]),
        window=w3, as_of=now, observers=ind,
    )
    print(f"  the lens moves: watchers={report3.watchers} — and the erosion level "
          f"does NOT,\n  because a polity that vouches for nobody must not thereby "
          f"become unwatchable")

    # 5. A voucher is for a party IN A CAPACITY. The same key read no boundary here,
    #    so its observer voucher buys it nothing as a reader.
    readers = arch3.reader_independence(did3, as_of=now)
    print(f"the same monitor as a boundary READER: "
          f"attested={readers.is_attested(trio[1].did)} — vouching is for a "
          f"relationship, not an identity")


if __name__ == "__main__":
    main()
    ratified_split()
    founding_boundary()
    attributable_boundary()
    standing_ground()
    independent_readers()
    founding_edges()
    who_was_watching()
    machine_polity()
    pruning(*watched_absence())
    retention()
    storage()
