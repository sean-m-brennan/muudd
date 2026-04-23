//! Unit tests for kc-reputation. Fixture helpers below construct archives
//! from hand-signed entries; for the end-to-end integration test using the
//! ceremony runners see `tests/integration.rs`.

#![allow(dead_code, unused_imports)]

use kc_archive::{ArchiveEntry, InMemoryArchive};
use kc_archive::store::Archive;
use kc_ceremony::inter::{
    AntiSpiralFeature, InterCommunityWitnessRecord, NonAdmissionRecord, RejectionDetail,
};
use kc_ceremony::intra::{InternalWitnessAttestation, IntraCommunityRiteRecord};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_core::{keys, Alg, Cid, DidKey, PrivKey, PubKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};

// TODO T5: enable once lookup module is populated
// use crate::lookup;
use crate::view::{RecognitionStanding, ReputationView};

/// (priv_key, pub_key, did) for a freshly generated Ed25519 identity.
fn fresh_id() -> (PrivKey, PubKey, DidKey) {
    let (sk, pk) = keys::generate(Alg::Ed25519).expect("keygen");
    let did = DidKey::from_pubkey(&pk).expect("did");
    (sk, pk, did)
}

fn t(secs: i64) -> Timestamp {
    jiff::Timestamp::from_second(secs).expect("timestamp")
}

fn publish_identity(
    archive: &mut InMemoryArchive,
    name: &str,
    did: &DidKey,
    root_pub: &PubKey,
    root_priv: &PrivKey,
    created_at: Timestamp,
) -> Cid {
    let ci = CommunityIdentity {
        name: name.into(),
        did: did.clone(),
        root_pubkey: root_pub.clone(),
        successor_chain: vec![],
        created_at,
    };
    let signed = Signed::build(ci, &[(did.as_str(), root_priv)]).expect("sign identity");
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(signed))
        .expect("append")
}

fn publish_intra_rite(
    archive: &mut InMemoryArchive,
    community_did: &DidKey,
    root_priv: &PrivKey,
    speaker_did: &DidKey,
    speaker_priv: &PrivKey,
    witness_count: usize,
    sealed_at: Timestamp,
) -> Cid {
    let recitation = Recitation {
        community_did: community_did.clone(),
        speaker_did: speaker_did.clone(),
        version: RecitationVersion::current(),
        movement_substrate: "fixture substrate".into(),
        movement_fallenness: "fixture fallenness".into(),
        movement_recognition: "fixture recognition".into(),
        movement_common_virtue: "fixture virtue".into(),
        movement_non_ultimacy: "fixture non-ultimacy".into(),
        language_tag: "en".into(),
        translation_en: None,
        recited_at: sealed_at,
    };
    let signed_recitation =
        Signed::build(recitation, &[(speaker_did.as_str(), speaker_priv)])
            .expect("sign recitation");
    // For fixtures, synthesize internal-witness attestations as empty vector
    // padded to the requested count. The unit tests inspect `.len()` only;
    // real attestations are exercised in the integration test.
    let mut attestations = Vec::with_capacity(witness_count);
    for i in 0..witness_count {
        let (wsk, wpk) = keys::generate(Alg::Ed25519).expect("kg");
        let wdid = DidKey::from_pubkey(&wpk).expect("wdid");
        let att = InternalWitnessAttestation {
            witness_did: wdid.clone(),
            recitation_cid: signed_recitation.payload_cid,
            attested_at: sealed_at,
        };
        let signed_att = Signed::build(att, &[(wdid.as_str(), &wsk)]).expect("sign att");
        attestations.push(signed_att);
        let _ = (i, wpk); // silence unused
    }
    let record = IntraCommunityRiteRecord {
        recitation: signed_recitation,
        internal_witnesses: attestations,
        version_of_rite: RecitationVersion::current(),
        sealed_at,
    };
    let signed =
        Signed::build(record, &[(community_did.as_str(), root_priv)]).expect("sign rite");
    archive
        .append(ArchiveEntry::IntraCommunityRite(signed))
        .expect("append")
}

fn publish_inter_admission(
    archive: &mut InMemoryArchive,
    participants: &[(&DidKey, &PrivKey)], // (community_did, its_root_priv)
    witnessed_at: Timestamp,
) -> Cid {
    let dids: Vec<DidKey> = participants.iter().map(|(d, _)| (*d).clone()).collect();
    let record = InterCommunityWitnessRecord {
        compact_label: "fixture-compact".into(),
        participants: dids.clone(),
        sequenced_rites: vec![],
        claims: vec![],
        acceptances: vec![],
        examination_exchanges: vec![],
        witnessed_at,
    };
    // Sign with the first participant's root key. The MVP runner co-signs
    // with every cross-boundary signer; for fixtures the single signature
    // is sufficient because the unit tests don't invoke verify_all().
    let (first_did, first_priv) = participants[0];
    let signed =
        Signed::build(record, &[(first_did.as_str(), first_priv)]).expect("sign witness");
    archive
        .append(ArchiveEntry::InterCommunityWitness(signed))
        .expect("append")
}

fn publish_inter_nonadmission(
    archive: &mut InMemoryArchive,
    participants: &[(&DidKey, &PrivKey)],
    rejecting_did: &DidKey,
    presenting_did: &DidKey,
    decided_at: Timestamp,
) -> Cid {
    let dids: Vec<DidKey> = participants.iter().map(|(d, _)| (*d).clone()).collect();
    let record = NonAdmissionRecord {
        compact_label: "fixture-compact".into(),
        participants: dids.clone(),
        rejections: vec![RejectionDetail {
            rejecting_community: rejecting_did.clone(),
            presenting_community: presenting_did.clone(),
            feature: AntiSpiralFeature::FallenActorAnthropology,
            rationale: "fixture rationale".into(),
        }],
        decided_at,
    };
    let (first_did, first_priv) = participants[0];
    let signed =
        Signed::build(record, &[(first_did.as_str(), first_priv)]).expect("sign non-adm");
    archive
        .append(ArchiveEntry::InterCommunityNonAdmission(signed))
        .expect("append")
}
