/*
  Copyright 2026 Sean M. Brennan and contributors

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

     http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! End-to-end tests for the intra-community rite runner.

use jiff::ToSpan;
use kc_core::{keys, time, Alg, Cid, DidKey, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};

use crate::recitation::{Commitment, CommonVirtueMovement, Recitation, RecitationVersion};

use super::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};

struct Fixture {
    community: Signed<CommunityIdentity>,
    root_priv: PrivKey,
    speaker_did: DidKey,
    speaker_priv: PrivKey,
    delegations: Vec<Signed<SpeakerDelegation>>,
    witness_registry: Signed<WitnessRegistry>,
    witnesses: Vec<(DidKey, PrivKey)>,
    now: Timestamp,
}

fn mk_fixture(name: &str, n_witnesses: usize) -> Fixture {
    let now = time::now();
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
    let community = CommunityIdentity::new_self_signed(
        name.into(),
        &root_priv,
        root_pub.clone(),
        now,
    )
    .unwrap();
    let community_did = community.payload.did.clone();

    let (speaker_priv, speaker_pk) = keys::generate(Alg::Ed25519).unwrap();
    let speaker_did = DidKey::from_pubkey(&speaker_pk).unwrap();

    let delegation_payload = SpeakerDelegation {
        community_did: community_did.clone(),
        speaker_did: speaker_did.clone(),
        speaker_pubkey: speaker_pk.clone(),
        scope: DelegationScope::Reciter,
        issued_at: now.checked_sub(1.hour()).unwrap(),
        expires_at: Some(now.checked_add(24.hours()).unwrap()),
        revoked_at: None,
    };
    let delegation =
        Signed::build(delegation_payload, &[(community_did.as_str(), &root_priv)]).unwrap();

    // Build N witnesses.
    let mut witnesses = Vec::with_capacity(n_witnesses);
    let mut witness_entries = Vec::with_capacity(n_witnesses);
    for _ in 0..n_witnesses {
        let (wpriv, wpub) = keys::generate(Alg::Ed25519).unwrap();
        let wdid = DidKey::from_pubkey(&wpub).unwrap();
        witness_entries.push(WitnessEntry {
            witness_did: wdid.clone(),
            witness_pubkey: wpub.clone(),
            role: WitnessRole::IntraCommunity,
            valid_from: now.checked_sub(1.hour()).unwrap(),
            valid_until: Some(now.checked_add(24.hours()).unwrap()),
        });
        witnesses.push((wdid, wpriv));
    }
    let witness_registry_payload = WitnessRegistry {
        community_did: community_did.clone(),
        entries: witness_entries,
        issued_at: now,
    };
    let witness_registry = Signed::build(
        witness_registry_payload,
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let _ = speaker_pk;
    Fixture {
        community,
        root_priv,
        speaker_did,
        speaker_priv,
        delegations: vec![delegation],
        witness_registry,
        witnesses,
        now,
    }
}

fn draft_recitation(community_did: &DidKey, speaker_did: &DidKey, recited_at: Timestamp) -> Recitation {
    Recitation {
        community_did: community_did.clone(),
        speaker_did: speaker_did.clone(),
        version: RecitationVersion::current(),
        movement_substrate: "We anchor above ourselves.".into(),
        movement_fallenness: "We know we can harm.".into(),
        movement_recognition: "We recognize the other.".into(),
        movement_common_virtue: CommonVirtueMovement::new(
            "We commit to the virtues this compact relies on.",
            vec![
                Commitment::new("truthfulness", "We tell the truth about what we have done."),
                Commitment::new("restitution", "We make good what we have broken."),
            ],
        ),
        movement_non_ultimacy: "This compact is not ultimate.".into(),
        language_tag: "en".into(),
        translation_en: None,
        recited_at,
    }
}

#[test]
fn happy_path_seals() {
    let f = mk_fixture("Alpha", 3);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);

    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();

    let attestations: Vec<_> = f
        .witnesses
        .iter()
        .map(|(wdid, wpriv)| {
            IntraCommunityRiteRunner::attest(wdid, wpriv, recitation.payload_cid, f.now).unwrap()
        })
        .collect();

    let sealed = runner
        .seal(
            recitation,
            attestations,
            &f.root_priv,
            f.now,
        )
        .expect("intra rite should seal");

    assert_eq!(sealed.signatures.len(), 1);
    assert_eq!(sealed.payload.internal_witnesses.len(), 3);
}

#[test]
fn zero_witnesses_rejected() {
    let f = mk_fixture("Alpha", 0);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let err = runner
        .seal(
            recitation,
            vec![],
            &f.root_priv,
            f.now,
        )
        .unwrap_err();
    match err {
        kc_core::KcError::Archive(msg) => assert!(msg.contains("at least one internal witness")),
        e => panic!("wrong error: {e:?}"),
    }
}

#[test]
fn speaker_without_delegation_rejected() {
    // Build fixture but wipe delegations — speaker has no authorization.
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &[], &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &f.witnesses[0].0,
        &f.witnesses[0].1,
        recitation.payload_cid,
        f.now,
    )
    .unwrap()];
    let err = runner
        .seal(recitation, attestations, &f.root_priv, f.now)
        .unwrap_err();
    match err {
        kc_core::KcError::Archive(msg) => assert!(msg.contains("no active Reciter delegation")),
        e => panic!("wrong error: {e:?}"),
    }
}

#[test]
fn revoked_delegation_rejected() {
    // Revoke delegation at a moment before the recitation time.
    let f = mk_fixture("Alpha", 1);
    let revoked_at = f.now.checked_sub(1.minute()).unwrap();

    let revoked = SpeakerDelegation {
        revoked_at: Some(revoked_at),
        ..f.delegations[0].payload.clone()
    };
    let sig = Signed::build(revoked, &[(f.community.payload.did.as_str(), &f.root_priv)]).unwrap();
    let delegations = vec![sig];

    let runner = IntraCommunityRiteRunner::new(&f.community, &delegations, &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &f.witnesses[0].0,
        &f.witnesses[0].1,
        recitation.payload_cid,
        f.now,
    )
    .unwrap()];
    assert!(matches!(
        runner.seal(recitation, attestations, &f.root_priv, f.now),
        Err(kc_core::KcError::Archive(_))
    ));
}

#[test]
fn tampered_recitation_fails_signature() {
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let mut recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    // Flip a byte in the signature.
    recitation.signatures[0].signature[0] ^= 0x01;
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &f.witnesses[0].0,
        &f.witnesses[0].1,
        recitation.payload_cid,
        f.now,
    )
    .unwrap()];
    let err = runner
        .seal(recitation, attestations, &f.root_priv, f.now)
        .unwrap_err();
    assert!(matches!(err, kc_core::KcError::SignatureInvalid));
}

#[test]
fn attestation_referencing_wrong_cid_rejected() {
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let bogus_cid = Cid::of_bytes(b"not the recitation");
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &f.witnesses[0].0,
        &f.witnesses[0].1,
        bogus_cid,
        f.now,
    )
    .unwrap()];
    let err = runner
        .seal(recitation, attestations, &f.root_priv, f.now)
        .unwrap_err();
    match err {
        kc_core::KcError::Archive(msg) => assert!(msg.contains("recitation_cid")),
        e => panic!("wrong error: {e:?}"),
    }
}

#[test]
fn non_registered_witness_rejected() {
    let f = mk_fixture("Alpha", 1);
    // Create an outsider witness not in the registry.
    let (outsider_priv, outsider_pk) = keys::generate(Alg::Ed25519).unwrap();
    let outsider_did = DidKey::from_pubkey(&outsider_pk).unwrap();
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &outsider_did,
        &outsider_priv,
        recitation.payload_cid,
        f.now,
    )
    .unwrap()];
    let err = runner
        .seal(recitation, attestations, &f.root_priv, f.now)
        .unwrap_err();
    match err {
        kc_core::KcError::Archive(msg) => assert!(msg.contains("IntraCommunity role")),
        e => panic!("wrong error: {e:?}"),
    }
}

#[test]
fn sealed_record_roundtrips_through_cbor() {
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &f.speaker_did,
        &f.speaker_priv,
        draft_recitation(&f.community.payload.did, &f.speaker_did, f.now),
    )
    .unwrap();
    let attestations = vec![IntraCommunityRiteRunner::attest(
        &f.witnesses[0].0,
        &f.witnesses[0].1,
        recitation.payload_cid,
        f.now,
    )
    .unwrap()];
    let sealed = runner
        .seal(recitation, attestations, &f.root_priv, f.now)
        .unwrap();
    let bytes = kc_core::canonical::to_canonical_cbor(&sealed).unwrap();
    let decoded: Signed<IntraCommunityRiteRecord> =
        kc_core::canonical::from_canonical_cbor(&bytes).unwrap();
    assert_eq!(sealed, decoded);
}

#[test]
fn a_rite_missing_a_movement_is_refused() {
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let mut draft = draft_recitation(&f.community.payload.did, &f.speaker_did, f.now);
    draft.movement_common_virtue.commitments.clear();
    let rec = IntraCommunityRiteRunner::recite(&f.speaker_did, &f.speaker_priv, draft).unwrap();
    let att =
        IntraCommunityRiteRunner::attest(&f.witnesses[0].0, &f.witnesses[0].1, rec.payload_cid, f.now)
            .unwrap();
    let err = runner
        .seal(rec, vec![att], &f.root_priv, f.now)
        .expect_err("a rite missing Movement 4 must not seal");
    assert!(format!("{err}").contains("missing a movement"), "unexpected error: {err}");
}

#[test]
fn a_rite_reciting_nothing_is_refused() {
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let mut draft = draft_recitation(&f.community.payload.did, &f.speaker_did, f.now);
    draft.movement_substrate = String::new();
    draft.movement_fallenness = String::new();
    draft.movement_recognition = String::new();
    draft.movement_common_virtue = CommonVirtueMovement::new("", vec![]);
    draft.movement_non_ultimacy = String::new();
    let rec = IntraCommunityRiteRunner::recite(&f.speaker_did, &f.speaker_priv, draft).unwrap();
    let att =
        IntraCommunityRiteRunner::attest(&f.witnesses[0].0, &f.witnesses[0].1, rec.payload_cid, f.now)
            .unwrap();
    assert!(
        runner.seal(rec, vec![att], &f.root_priv, f.now).is_err(),
        "a rite reciting nothing at all must not seal"
    );
}

#[test]
fn the_sealed_version_is_the_recitation_version() {
    // `version_of_rite` is derived rather than supplied, so a sealed record cannot
    // disagree with the recitation it holds. Before this, `seal` took the version as
    // a parameter and nothing compared the two.
    let f = mk_fixture("Alpha", 1);
    let runner = IntraCommunityRiteRunner::new(&f.community, &f.delegations, &f.witness_registry);
    let mut draft = draft_recitation(&f.community.payload.did, &f.speaker_did, f.now);
    draft.version = RecitationVersion("kc-rite-v-unreleased".into());
    let rec = IntraCommunityRiteRunner::recite(&f.speaker_did, &f.speaker_priv, draft).unwrap();
    let att =
        IntraCommunityRiteRunner::attest(&f.witnesses[0].0, &f.witnesses[0].1, rec.payload_cid, f.now)
            .unwrap();
    let sealed = runner.seal(rec, vec![att], &f.root_priv, f.now).expect("should seal");
    assert_eq!(
        sealed.payload.version_of_rite,
        sealed.payload.recitation.payload.version
    );
    assert_eq!(sealed.payload.version_of_rite.0, "kc-rite-v-unreleased");
}
