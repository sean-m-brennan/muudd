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

"""The artifacts that cross to kith-covenant, as JSON: the identity handoff, the
coherence Concern, the two lifecycle changes (fission and fusion), the envoy
delegation, the member roll, and the rite."""
from __future__ import annotations

import json

import pytest

import ethne as e
from ethne.monitor import cohesion_report, founding_candidate
from ethne.authoring import (
    Signer, admit_member, authorize_envoy, confer_office, depart_member,
    resign_envoy,
)
from ethne.uplift import (
    coherence_concern,
    intra_rite,
    rite_dict,
    community_identity,
    concern_dict,
    delegation_dict,
    envoy_delegation,
    fission,
    fusion,
    identity_dict,
    member_roll,
)


def _identity(found, name):
    """A separately chartered polity, handed off as a signed identity."""
    p = found(3)
    return p, community_identity(p.archive, p.did, name, p.priv, p.pub, created_at=500)


def _concern_level_report(polity):
    roll = polity.roll(as_of=300)
    return cohesion_report(roll, founding_candidate([(roll[0], 0.9, None, None, False)]))


def test_community_identity_is_self_signed_and_matches_the_charter(polity):
    payload = identity_dict(
        community_identity(polity.archive, polity.did, "Ourania", polity.priv, polity.pub, created_at=500)
    )
    assert payload["payload"]["did"] == polity.did
    assert payload["payload"]["name"] == "Ourania"
    assert payload["signatures"], "the identity carries its own signature"


def test_identity_with_a_foreign_root_key_is_refused(polity):
    other_priv, other_pub, _did = e.generate_keypair()
    with pytest.raises(RuntimeError, match="does not match"):
        community_identity(polity.archive, polity.did, "Ourania", other_priv, other_pub, created_at=500)


def test_identity_needs_a_charter_in_this_log(polity):
    _priv, _pub, stranger = e.generate_keypair()
    with pytest.raises(ValueError, match="no charter"):
        community_identity(polity.archive, stranger, "Nowhere", polity.priv, polity.pub)


def test_no_concern_below_concern_level_erosion(polity):
    roll = polity.roll(as_of=300)
    healthy = cohesion_report(roll, founding_candidate([(d, 0.9, None, None, False) for d in roll]))
    assert healthy.erosion == "None"
    assert coherence_concern(healthy, polity.did, polity.founders[0], at=600) is None
    assert concern_dict(None) is None


def test_a_concern_crosses_only_at_concern_level(polity):
    report = _concern_level_report(polity)
    assert report.erosion == "Concern"

    record = concern_dict(coherence_concern(report, polity.did, polity.founders[0], at=600))
    payload = record["payload"]
    assert payload["kind"] == "Concern"
    assert payload["bearing"] == {"Other": "coherence-erosion"}
    assert payload["subject_community"] == polity.did
    assert payload["reporters"] == [polity.founders[0].did]
    # KC discounts a monitor-originated signal, so the perspective is explicit.
    assert payload["reporter_perspective"] == "ethne cohesion monitor"
    assert payload["ladder_rung"] is None


def test_fission_emits_a_link_per_successor_and_a_dissolution(polity, found):
    _a, alpha = _identity(found, "Alpha")
    _b, beta = _identity(found, "Beta")

    outcome = fission(
        polity.archive, polity.did, [alpha, beta], polity.priv,
        deciders=polity.founders[:2], rationale="irreconcilable readings", at=800,
    )

    successor_dids = [identity_dict(s)["payload"]["did"] for s in (alpha, beta)]
    assert len(outcome.links) == len(outcome.successors) == 2
    for link_json, successor in zip(outcome.links, successor_dids):
        link = json.loads(link_json)["payload"]
        assert link["previous_did"] == polity.did
        assert link["new_did"] == successor
    dissolution = json.loads(outcome.dissolution)["payload"]
    assert dissolution["polity"] == polity.did
    assert dissolution["successors"] == successor_dids
    assert dissolution["rationale"] == "irreconcilable readings"
    # Emission, not appending: the log still holds only the charter.
    assert len(polity.archive) == 1


def test_a_three_way_split_is_one_act(polity, found):
    # The D25 point: before this, three successors took two chained fissions, which
    # recorded an intermediate polity nobody lived in.
    idents = [_identity(found, n)[1] for n in ("A", "B", "C")]
    outcome = fission(
        polity.archive, polity.did, idents, polity.priv,
        deciders=polity.founders[:2], rationale="three ways", at=800,
    )
    assert len(outcome.successors) == 3
    assert len(outcome.links) == 3
    dids = [identity_dict(i)["payload"]["did"] for i in idents]
    assert json.loads(outcome.dissolution)["payload"]["successors"] == dids


def test_a_split_into_one_is_not_a_split(polity, found):
    _a, alpha = _identity(found, "Alpha")
    with pytest.raises(RuntimeError, match="at least two"):
        fission(polity.archive, polity.did, [alpha], polity.priv, at=800)


def test_fission_rejects_one_successor_named_twice(polity, found):
    _a, alpha = _identity(found, "Alpha")
    with pytest.raises(RuntimeError, match="twice"):
        fission(polity.archive, polity.did, [alpha, alpha], polity.priv, at=800)


def test_a_polity_cannot_succeed_itself(polity, found):
    # The predecessor's own identity offered as one of its successors.
    itself = community_identity(
        polity.archive, polity.did, "Self", polity.priv, polity.pub, created_at=700
    )
    _b, beta = _identity(found, "Beta")
    with pytest.raises(RuntimeError, match="cannot succeed itself"):
        fission(polity.archive, polity.did, [itself, beta], polity.priv, at=800)


def test_fusion_merges_predecessors_from_one_log(found):
    """Both predecessors' charters live in a shared log, as a KC-side view would."""
    first = found(3)
    second = found(3)
    merged_log = e.Archive.load_json(first.archive.dump_json())
    # `merge_json`, not `append_json`: two independent logs folded into one view is
    # a different operation from applying the next segment of this one (D20).
    merged_log.merge_json(second.archive.dump_json())
    # The merged polity is chartered in its own right before it is handed off.
    fused_holder = found(3)
    fused = community_identity(
        fused_holder.archive, fused_holder.did, "Merged",
        fused_holder.priv, fused_holder.pub, created_at=700,
    )

    outcome = fusion(
        merged_log,
        [
            (first.did, first.priv, first.founders[:2]),
            (second.did, second.priv, second.founders[:2]),
        ],
        fused,
        rationale="one people now",
        at=900,
    )

    assert identity_dict(outcome.fused)["payload"]["name"] == "Merged"
    assert len(outcome.links) == len(outcome.dissolutions) == 2
    for did, link_json, diss_json in zip(
        (first.did, second.did), outcome.links, outcome.dissolutions
    ):
        assert json.loads(link_json)["payload"]["previous_did"] == did
        diss = json.loads(diss_json)["payload"]
        assert diss["polity"] == did
        assert diss["successors"] == [fused_holder.did]


def test_fusion_needs_at_least_two_predecessors(polity, found):
    holder = found(3)
    fused = community_identity(holder.archive, holder.did, "Merged", holder.priv, holder.pub, created_at=700)
    with pytest.raises(RuntimeError, match="at least two"):
        fusion(polity.archive, [(polity.did, polity.priv, polity.founders[:2])], fused, at=900)


def test_fusion_refuses_a_repeated_predecessor(polity, found):
    holder = found(3)
    fused = community_identity(holder.archive, holder.did, "Merged", holder.priv, holder.pub, created_at=700)
    twice = [(polity.did, polity.priv, polity.founders[:2])] * 2
    with pytest.raises(RuntimeError, match="twice"):
        fusion(polity.archive, twice, fused, at=900)


def test_fission_rejects_a_non_identity_payload(polity, found):
    _a, alpha = _identity(found, "Alpha")
    with pytest.raises(ValueError, match="signed community identity"):
        fission(polity.archive, polity.did, [alpha, "{}"], polity.priv, at=800)

# --- the envoy artifact (D14) ------------------------------------------------


def test_an_authorized_envoy_crosses_as_a_kc_speaker_delegation(polity):
    officer, envoy = polity.founders[0], polity.founders[2]
    confer_office(polity.archive, polity.did, "EnvoyAuthorizer", officer,
                  polity.founders[:2], decided_at=200)
    authorize_envoy(polity.archive, polity.did, envoy, [officer],
                    expires_at=9_000, decided_at=300)
    polity.archive.check_authority()

    signed = delegation_dict(envoy_delegation(
        polity.archive, polity.did, envoy.did, polity.priv, polity.pub,
        issued_at=400, as_of=400,
    ))
    payload = signed["payload"]
    assert payload["community_did"] == polity.did
    assert payload["speaker_did"] == envoy.did
    assert payload["scope"] == "Envoy"
    assert payload["revoked_at"] is None
    # The polity's own expiry travels with it, so the tiers cannot disagree about
    # when the standing ends.
    assert payload["expires_at"] is not None
    # Signed by the polity root — that is what makes it resolvable by community.
    assert [sig["signer_did"] for sig in signed["signatures"]] == [polity.did]


def test_no_delegation_without_a_standing(polity):
    with pytest.raises(RuntimeError, match="no envoy standing"):
        envoy_delegation(
            polity.archive, polity.did, polity.founders[2].did,
            polity.priv, polity.pub, issued_at=400, as_of=400,
        )


def test_a_resigned_envoy_can_no_longer_be_delegated(polity):
    """The use-time rule reaching the border: the standing is read as of `as_of`,
    so the delegation cannot outlive it."""
    officer, envoy = polity.founders[0], polity.founders[2]
    confer_office(polity.archive, polity.did, "EnvoyAuthorizer", officer,
                  polity.founders[:2], decided_at=200)
    authorize_envoy(polity.archive, polity.did, envoy, [officer], decided_at=300)
    resign_envoy(polity.archive, polity.did, envoy, decided_at=400)
    polity.archive.check_authority()

    # Before the resignation the delegation was mintable; after it, not.
    envoy_delegation(polity.archive, polity.did, envoy.did, polity.priv, polity.pub,
                     issued_at=350, as_of=350)
    with pytest.raises(RuntimeError, match="no envoy standing"):
        envoy_delegation(polity.archive, polity.did, envoy.did, polity.priv, polity.pub,
                         issued_at=500, as_of=500)


def test_a_delegation_needs_the_polity_root_key(polity):
    officer, envoy = polity.founders[0], polity.founders[2]
    confer_office(polity.archive, polity.did, "EnvoyAuthorizer", officer,
                  polity.founders[:2], decided_at=200)
    authorize_envoy(polity.archive, polity.did, envoy, [officer], decided_at=300)
    other_priv, other_pub, _did = e.generate_keypair()
    with pytest.raises(RuntimeError, match="does not match"):
        envoy_delegation(polity.archive, polity.did, envoy.did, other_priv, other_pub,
                         issued_at=400, as_of=400)


# --- the member roll: KC's insider ring gets a feed --------------------------


def test_the_roll_crosses_as_a_kc_member_roll(polity):
    signed = identity_dict(member_roll(
        polity.archive, polity.did, polity.priv, polity.pub,
        as_of=300, published_at=400,
    ))
    payload = signed["payload"]
    assert payload["community_did"] == polity.did
    # Exactly the live roll, and every member carries its own verifying key.
    on_roll = {m["member_did"] for m in payload["members"]}
    assert on_roll == set(polity.roll(as_of=300))
    assert all(m["member_pubkey"] for m in payload["members"])
    assert payload["as_of"] is not None
    # Signed by the polity root — naming who belongs is not delegable in KC.
    assert [sig["signer_did"] for sig in signed["signatures"]] == [polity.did]


def test_the_roll_tracks_admissions(polity):
    joiner = Signer.generate()
    admit_member(polity.archive, polity.did, joiner,
                 polity.quorum(as_of=300), decided_at=350)
    polity.archive.check_authority()

    before = identity_dict(member_roll(
        polity.archive, polity.did, polity.priv, polity.pub, as_of=300,
    ))["payload"]["members"]
    after = identity_dict(member_roll(
        polity.archive, polity.did, polity.priv, polity.pub, as_of=400,
    ))["payload"]["members"]
    assert len(after) == len(before) + 1
    assert joiner.did in {m["member_did"] for m in after}


def test_a_departed_member_leaves_the_polity_roll_but_keeps_the_kc_channel(found):
    """The one asymmetry between the tiers worth stating: ethne's roll shrinks on a
    departure, and KC's insider ring does not. KC unions rolls rather than
    superseding them, so a polity cannot shrink the suspect set around whoever just
    left by re-publishing. What crosses here is the smaller roll; what KC does with
    it is keep both.

    Founded with five, because a three-member roll cannot lose anyone without
    dropping below the self-govern floor."""
    polity = found(5)
    leaver = polity.founders[4]
    depart_member(polity.archive, polity.did, leaver, decided_at=350)
    polity.archive.check_authority()

    after = identity_dict(member_roll(
        polity.archive, polity.did, polity.priv, polity.pub, as_of=400,
    ))["payload"]["members"]
    assert leaver.did not in {m["member_did"] for m in after}


def test_a_roll_needs_the_polity_root_key(polity):
    other_priv, other_pub, _did = e.generate_keypair()
    with pytest.raises(RuntimeError, match="does not match"):
        member_roll(polity.archive, polity.did, other_priv, other_pub, as_of=300)


# --- The rite ------------------------------------------------------------------
#
# The artifact whose absence stopped a polity from being admitted to a compact at
# all: KC's inter-community ceremony takes a sealed intra rite as its input, and the
# border emitted no reciter delegation and no witness registry, so there was nothing
# to seal.

COMMITMENTS = (("mutual-aid", "we carry each other"), ("truthfulness", "we do not lie"))


@pytest.fixture
def reciting(found):
    """A polity with commitments, an Inductor, and one complete rite standing.

    The Inductor attends the rite as well, which is the ordinary case in a small
    polity and the one the border has to handle: the attendees are the witnesses, and
    the reciter cannot be among them.
    """
    p = found(4, commitments=COMMITMENTS)
    inductor = p.founders[0]
    confer_office(p.archive, p.did, "Inductor", inductor, p.quorum(as_of=300),
                  rationale="presides over the rite", decided_at=300)
    p.renew(p.founders[1], at=400, attending=[inductor, p.founders[2]],
            commitments=[k for k, _ in COMMITMENTS])
    p.archive.check_authority()
    return p, inductor


def test_the_inductor_recites_and_the_attendees_witness(reciting):
    p, inductor = reciting
    handoff = rite_dict(intra_rite(p.archive, p.did, inductor.did, p.priv, p.pub, at=500))

    delegation = handoff["reciter"]["payload"]
    assert delegation["community_did"] == p.did
    assert delegation["speaker_did"] == inductor.did
    assert delegation["scope"] == "Reciter", "the Envoy scope is representation, not recital"

    registry = handoff["witnesses"]["payload"]
    seated = {entry["witness_did"] for entry in registry["entries"]}
    # The other attendee witnesses; the reciter does not witness its own recitation.
    assert seated == {p.founders[2].did}
    assert all(entry["role"] == "IntraCommunity" for entry in registry["entries"])


def test_the_recitation_carries_the_polity_s_own_commitments(reciting):
    p, inductor = reciting
    handoff = rite_dict(intra_rite(p.archive, p.did, inductor.did, p.priv, p.pub, at=500))
    recitation = handoff["recitation"]

    assert recitation["speaker_did"] == inductor.did
    assert recitation["movement_substrate"] == "answers to what it did not author"
    recited = [c["key"] for c in recitation["movement_common_virtue"]["commitments"]]
    assert recited == [k for k, _ in COMMITMENTS]
    # Unsigned on purpose: the reciter holds that key, and this seam does not.
    assert "signatures" not in recitation


def test_a_member_who_holds_no_inductor_office_may_not_recite(reciting):
    p, _inductor = reciting
    with pytest.raises(RuntimeError, match="holds no Inductor office"):
        intra_rite(p.archive, p.did, p.founders[3].did, p.priv, p.pub, at=500)


def test_a_rite_is_not_mintable_from_a_ceremony_nobody_performed(found):
    """A polity that has held no rite of its own cannot hand KC one. The office is
    conferred and the commitments are in force; what is missing is the ceremony."""
    p = found(4, commitments=COMMITMENTS)
    inductor = p.founders[0]
    confer_office(p.archive, p.did, "Inductor", inductor, p.quorum(as_of=300),
                  decided_at=300)
    with pytest.raises(RuntimeError, match="no complete rite stands"):
        intra_rite(p.archive, p.did, inductor.did, p.priv, p.pub, at=500)


def test_a_polity_with_no_commitments_recites_no_movement_four(found):
    """KC refuses a Movement 4 that commits to nothing, so the border refuses first
    rather than drafting one KC will reject."""
    p = found(4)
    inductor = p.founders[0]
    confer_office(p.archive, p.did, "Inductor", inductor, p.quorum(as_of=300),
                  decided_at=300)
    p.renew(p.founders[1], at=400, attending=[inductor, p.founders[2]])
    p.archive.check_authority()
    with pytest.raises(RuntimeError, match="no commitments in force"):
        intra_rite(p.archive, p.did, inductor.did, p.priv, p.pub, at=500)


def test_a_rite_needs_the_polity_root_key(reciting):
    p, inductor = reciting
    other_priv, other_pub, _did = e.generate_keypair()
    with pytest.raises(RuntimeError, match="does not match"):
        intra_rite(p.archive, p.did, inductor.did, other_priv, other_pub, at=500)
