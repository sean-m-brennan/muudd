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

use crate::lookup::lookup;
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

fn publish_delegation(
    archive: &mut InMemoryArchive,
    community_did: &DidKey,
    community_priv: &PrivKey,
    speaker_did: &DidKey,
    speaker_pub: &PubKey,
    scope: DelegationScope,
    issued_at: Timestamp,
    expires_at: Option<Timestamp>,
    revoked_at: Option<Timestamp>,
) -> Cid {
    let d = SpeakerDelegation {
        community_did: community_did.clone(),
        speaker_did: speaker_did.clone(),
        speaker_pubkey: speaker_pub.clone(),
        scope,
        issued_at,
        expires_at,
        revoked_at,
    };
    let signed = Signed::build(d, &[(community_did.as_str(), community_priv)])
        .expect("sign del");
    archive
        .append(ArchiveEntry::SpeakerDelegated(signed))
        .expect("append")
}

fn publish_witness_registry(
    archive: &mut InMemoryArchive,
    community_did: &DidKey,
    community_priv: &PrivKey,
    entries: Vec<WitnessEntry>,
    issued_at: Timestamp,
) -> Cid {
    let reg = WitnessRegistry {
        community_did: community_did.clone(),
        entries,
        issued_at,
    };
    let signed = Signed::build(reg, &[(community_did.as_str(), community_priv)])
        .expect("sign reg");
    archive
        .append(ArchiveEntry::WitnessRegistryUpdated(signed))
        .expect("append")
}

#[test]
fn identified_standing_with_only_identity() {
    let (root_priv, root_pub, did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    let id_cid = publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));

    let view = lookup(&archive, &did, Some(t(200)));
    assert_eq!(view.standing, RecognitionStanding::Identified);
    assert_eq!(view.identity_cid, Some(id_cid));
}

#[test]
fn identity_filtered_out_when_as_of_is_before_created_at() {
    let (root_priv, root_pub, did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(1_000));

    let view = lookup(&archive, &did, Some(t(500)));
    assert_eq!(view.standing, RecognitionStanding::Unknown);
    assert!(view.identity_cid.is_none());
}

#[test]
fn unknown_standing_for_missing_did() {
    let (_, _, stranger_did) = fresh_id();
    let archive = InMemoryArchive::new();
    let view: ReputationView = lookup(&archive, &stranger_did, Some(t(1_000_000)));
    assert_eq!(view.community_did, stranger_did);
    assert_eq!(view.as_of, t(1_000_000));
    assert_eq!(view.standing, RecognitionStanding::Unknown);
    assert!(view.identity_cid.is_none());
    assert!(view.intra_rites.is_empty());
    assert!(view.inter_admissions.is_empty());
    assert!(view.inter_nonadmissions.is_empty());
    assert!(view.active_delegations.is_empty());
    assert!(view.active_witnesses.is_empty());
}

#[test]
fn intra_rites_accumulated_in_ascending_sealed_at_order() {
    let (root_priv, root_pub, did) = fresh_id();
    let (speaker_priv, _, speaker_did) = fresh_id();

    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));
    let cid_b = publish_intra_rite(&mut archive, &did, &root_priv, &speaker_did, &speaker_priv, 2, t(300));
    let cid_a = publish_intra_rite(&mut archive, &did, &root_priv, &speaker_did, &speaker_priv, 3, t(200));

    let view = lookup(&archive, &did, Some(t(400)));
    // Append-order is [b(300), a(200)], but we want ascending-by-sealed_at.
    assert_eq!(view.intra_rites.len(), 2);
    assert_eq!(view.intra_rites[0].record_cid, cid_a);
    assert_eq!(view.intra_rites[0].sealed_at, t(200));
    assert_eq!(view.intra_rites[0].witness_count, 3);
    assert_eq!(view.intra_rites[1].record_cid, cid_b);
    assert_eq!(view.intra_rites[1].sealed_at, t(300));
    assert_eq!(view.intra_rites[1].witness_count, 2);
    assert_eq!(view.intra_rites[0].speaker_did, speaker_did);
}

#[test]
fn intra_rite_filtered_out_by_as_of() {
    let (root_priv, root_pub, did) = fresh_id();
    let (speaker_priv, _, speaker_did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));
    publish_intra_rite(&mut archive, &did, &root_priv, &speaker_did, &speaker_priv, 1, t(500));

    let view = lookup(&archive, &did, Some(t(400)));
    assert!(view.intra_rites.is_empty());
}

#[test]
fn incompact_standing_and_counterparty_exclusion() {
    let (alpha_priv, alpha_pub, alpha) = fresh_id();
    let (beta_priv, beta_pub, beta) = fresh_id();
    let (gamma_priv, gamma_pub, gamma) = fresh_id();

    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &alpha, &alpha_pub, &alpha_priv, t(100));
    publish_identity(&mut archive, "Beta", &beta, &beta_pub, &beta_priv, t(100));
    publish_identity(&mut archive, "Gamma", &gamma, &gamma_pub, &gamma_priv, t(100));
    let wcid = publish_inter_admission(
        &mut archive,
        &[(&alpha, &alpha_priv), (&beta, &beta_priv), (&gamma, &gamma_priv)],
        t(500),
    );

    let view = lookup(&archive, &alpha, Some(t(600)));
    assert_eq!(view.standing, RecognitionStanding::InCompact);
    assert_eq!(view.inter_admissions.len(), 1);
    assert_eq!(view.inter_admissions[0].record_cid, wcid);
    assert_eq!(view.inter_admissions[0].witnessed_at, t(500));
    // Counterparties are [Beta, Gamma] (not Alpha itself), order preserved.
    assert_eq!(view.inter_admissions[0].counterparties, vec![beta.clone(), gamma.clone()]);
}

#[test]
fn inter_admission_filtered_out_by_as_of() {
    let (alpha_priv, alpha_pub, alpha) = fresh_id();
    let (beta_priv, beta_pub, beta) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &alpha, &alpha_pub, &alpha_priv, t(100));
    publish_identity(&mut archive, "Beta", &beta, &beta_pub, &beta_priv, t(100));
    publish_inter_admission(&mut archive, &[(&alpha, &alpha_priv), (&beta, &beta_priv)], t(500));

    let view = lookup(&archive, &alpha, Some(t(400)));
    assert_eq!(view.standing, RecognitionStanding::Identified);
    assert!(view.inter_admissions.is_empty());
}

#[test]
fn denied_standing_with_only_nonadmission() {
    let (alpha_priv, alpha_pub, alpha) = fresh_id();
    let (beta_priv, beta_pub, beta) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &alpha, &alpha_pub, &alpha_priv, t(100));
    publish_identity(&mut archive, "Beta", &beta, &beta_pub, &beta_priv, t(100));
    let ncid = publish_inter_nonadmission(
        &mut archive,
        &[(&alpha, &alpha_priv), (&beta, &beta_priv)],
        &beta, // rejecting
        &alpha, // presenting
        t(500),
    );

    let view = lookup(&archive, &alpha, Some(t(600)));
    assert_eq!(view.standing, RecognitionStanding::Denied);
    assert_eq!(view.inter_nonadmissions.len(), 1);
    assert_eq!(view.inter_nonadmissions[0].record_cid, ncid);
    assert_eq!(view.inter_nonadmissions[0].decided_at, t(500));
    assert_eq!(view.inter_nonadmissions[0].counterparties, vec![beta.clone()]);
    assert_eq!(view.inter_nonadmissions[0].rejections.len(), 1);
    assert_eq!(view.inter_nonadmissions[0].rejections[0].rejecting_community, beta);
    assert_eq!(view.inter_nonadmissions[0].rejections[0].presenting_community, alpha);
}

#[test]
fn admission_supersedes_nonadmission() {
    let (alpha_priv, alpha_pub, alpha) = fresh_id();
    let (beta_priv, beta_pub, beta) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &alpha, &alpha_pub, &alpha_priv, t(100));
    publish_identity(&mut archive, "Beta", &beta, &beta_pub, &beta_priv, t(100));
    publish_inter_nonadmission(
        &mut archive,
        &[(&alpha, &alpha_priv), (&beta, &beta_priv)],
        &beta, &alpha, t(500),
    );
    publish_inter_admission(
        &mut archive,
        &[(&alpha, &alpha_priv), (&beta, &beta_priv)],
        t(800),
    );

    let view = lookup(&archive, &alpha, Some(t(1_000)));
    assert_eq!(view.standing, RecognitionStanding::InCompact);
    assert_eq!(view.inter_admissions.len(), 1);
    assert_eq!(view.inter_nonadmissions.len(), 1);
}

#[test]
fn active_delegations_populated_and_revocations_excluded() {
    let (root_priv, root_pub, did) = fresh_id();
    let (_, speaker_pub, speaker_did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));
    let active_cid = publish_delegation(
        &mut archive, &did, &root_priv, &speaker_did, &speaker_pub,
        DelegationScope::Reciter, t(200), None, None,
    );
    let (_, other_pub, other_did) = fresh_id();
    let _revoked_cid = publish_delegation(
        &mut archive, &did, &root_priv, &other_did, &other_pub,
        DelegationScope::Envoy, t(300), None, Some(t(400)),
    );

    let view_now = lookup(&archive, &did, Some(t(500)));
    assert_eq!(view_now.active_delegations.len(), 1);
    assert_eq!(view_now.active_delegations[0].record_cid, active_cid);
    assert_eq!(view_now.active_delegations[0].subject_did, speaker_did);
    assert_eq!(view_now.active_delegations[0].scope, DelegationScope::Reciter);

    // Before revocation, both are active.
    let view_before = lookup(&archive, &did, Some(t(350)));
    assert_eq!(view_before.active_delegations.len(), 2);
}

#[test]
fn active_witnesses_respect_validity_window_and_registry_supersession() {
    let (root_priv, root_pub, did) = fresh_id();
    let (_, w1_pub, w1_did) = fresh_id();
    let (_, w2_pub, w2_did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));

    // Registry v1: W1 intra-community, valid 200..400.
    let _reg1 = publish_witness_registry(
        &mut archive, &did, &root_priv,
        vec![WitnessEntry {
            witness_did: w1_did.clone(),
            witness_pubkey: w1_pub.clone(),
            role: WitnessRole::IntraCommunity,
            valid_from: t(200),
            valid_until: Some(t(400)),
        }],
        t(200),
    );
    // Registry v2: both W1 (still intra, now no expiry) and W2 (cross-boundary).
    let _reg2 = publish_witness_registry(
        &mut archive, &did, &root_priv,
        vec![
            WitnessEntry {
                witness_did: w1_did.clone(),
                witness_pubkey: w1_pub,
                role: WitnessRole::IntraCommunity,
                valid_from: t(500),
                valid_until: None,
            },
            WitnessEntry {
                witness_did: w2_did.clone(),
                witness_pubkey: w2_pub,
                role: WitnessRole::CrossBoundary,
                valid_from: t(500),
                valid_until: None,
            },
        ],
        t(500),
    );

    // At t=300: v2 not yet issued, so use v1; W1 active.
    let view_300 = lookup(&archive, &did, Some(t(300)));
    assert_eq!(view_300.active_witnesses.len(), 1);
    assert_eq!(view_300.active_witnesses[0].member_did, w1_did);

    // At t=450: v2 not yet issued; v1 expired (valid_until = 400). Zero active.
    let view_450 = lookup(&archive, &did, Some(t(450)));
    assert!(view_450.active_witnesses.is_empty());

    // At t=600: v2 issued; both W1 and W2 active.
    let view_600 = lookup(&archive, &did, Some(t(600)));
    assert_eq!(view_600.active_witnesses.len(), 2);
}

#[test]
fn as_of_none_resolves_to_now_and_is_stable() {
    let (root_priv, root_pub, did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));

    let before = kc_core::time::now();
    let view = lookup(&archive, &did, None);
    let after = kc_core::time::now();

    assert!(view.as_of >= before && view.as_of <= after);
    assert_eq!(view.standing, RecognitionStanding::Identified);
}
