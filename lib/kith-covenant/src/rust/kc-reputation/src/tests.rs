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

// ---------------------------------------------------------------------------
// Conduct + sponsorship fixtures and capture-signal tests.
// ---------------------------------------------------------------------------

use kc_ceremony::inter::SponsorshipRecord;
use kc_conduct::{
    ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity,
};

use crate::analytics::{composition_graph, feature_concern_profile, sponsor_stake};

#[allow(clippy::too_many_arguments)]
fn publish_sponsorship(
    archive: &mut InMemoryArchive,
    sponsor_did: &DidKey,
    sponsor_priv: &PrivKey,
    entrant_did: &DidKey,
    probation_until: Timestamp,
    sponsored_at: Timestamp,
) -> Cid {
    let rec = SponsorshipRecord {
        sponsor_community: sponsor_did.clone(),
        entrant_community: entrant_did.clone(),
        admission_cid: None,
        probation_until,
        statement: "fixture vouch".into(),
        sponsored_at,
    };
    let signed =
        Signed::build(rec, &[(sponsor_did.as_str(), sponsor_priv)]).expect("sign sponsorship");
    archive
        .append(ArchiveEntry::SponsorshipVouched(signed))
        .expect("append")
}

#[allow(clippy::too_many_arguments)]
fn publish_conduct(
    archive: &mut InMemoryArchive,
    subject_did: &DidKey,
    reporter_did: &DidKey,
    reporter_priv: &PrivKey,
    perspective: Option<&str>,
    kind: ConductKind,
    bearing: ConductBearing,
    recorded_at: Timestamp,
) -> Cid {
    let rec = ConductRecord {
        subject_community: subject_did.clone(),
        reporters: vec![reporter_did.clone()],
        reporter_perspective: perspective.map(|s| s.to_string()),
        kind,
        bearing,
        severity: Severity::Minor,
        ladder_rung: if matches!(kind, ConductKind::Concern | ConductKind::Commendation) {
            None
        } else {
            Some(LadderRung::MediatedArbitration)
        },
        decision_class: DecisionClass::FirstClass,
        references: vec![],
        narrative: "fixture conduct".into(),
        observed_at: recorded_at,
        recorded_at,
    };
    let signed =
        Signed::build(rec, &[(reporter_did.as_str(), reporter_priv)]).expect("sign conduct");
    archive
        .append(ArchiveEntry::ConductAttested(signed))
        .expect("append")
}

#[test]
fn conduct_projects_onto_subject_only() {
    let (_, _, subject) = fresh_id();
    let (_, _, other) = fresh_id();
    let (rep_priv, _rep_pub, reporter) = fresh_id();
    let mut archive = InMemoryArchive::new();

    let cid = publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, Some("charter-originalist"),
        ConductKind::Concern, ConductBearing::AntiSpiral(AntiSpiralFeature::NonUltimacyOfCompact),
        t(500),
    );

    let subj_view = lookup(&archive, &subject, Some(t(1000)));
    assert_eq!(subj_view.conduct.len(), 1);
    assert_eq!(subj_view.conduct[0].record_cid, cid);
    assert_eq!(subj_view.conduct[0].kind, ConductKind::Concern);

    // The conduct is not attributed to an unrelated community.
    let other_view = lookup(&archive, &other, Some(t(1000)));
    assert!(other_view.conduct.is_empty());
}

#[test]
fn conduct_respects_as_of_horizon() {
    let (_, _, subject) = fresh_id();
    let (rep_priv, _, reporter) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, None,
        ConductKind::Violation, ConductBearing::CommonsObligation, t(900),
    );
    // Queried before the record's recorded_at: not yet visible.
    assert!(lookup(&archive, &subject, Some(t(800))).conduct.is_empty());
    assert_eq!(lookup(&archive, &subject, Some(t(1000))).conduct.len(), 1);
}

#[test]
fn sponsorship_projects_to_both_endpoints() {
    let (sp_priv, _, sponsor) = fresh_id();
    let (_, _, entrant) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &entrant, t(2000), t(100));

    let sponsor_view = lookup(&archive, &sponsor, Some(t(500)));
    assert_eq!(sponsor_view.sponsorships_given.len(), 1);
    assert!(sponsor_view.sponsorships_received.is_empty());

    let entrant_view = lookup(&archive, &entrant, Some(t(500)));
    assert_eq!(entrant_view.sponsorships_received.len(), 1);
    assert!(entrant_view.sponsorships_given.is_empty());
}

#[test]
fn composition_graph_counts_out_degree_and_ranks_concentration() {
    let (sp_priv, _, sponsor) = fresh_id();
    let (_, _, e1) = fresh_id();
    let (_, _, e2) = fresh_id();
    let (other_priv, _, other_sponsor) = fresh_id();
    let (_, _, e3) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &e1, t(2000), t(100));
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &e2, t(2000), t(200));
    publish_sponsorship(&mut archive, &other_sponsor, &other_priv, &e3, t(2000), t(300));

    let graph = composition_graph(&archive, Some(t(1000)));
    assert_eq!(graph.edges.len(), 3);
    assert_eq!(graph.out_degree.get(sponsor.as_str()), Some(&2));
    let ranked = graph.by_concentration();
    assert_eq!(ranked[0].0, sponsor.as_str().to_string());
    assert_eq!(ranked[0].1, 2);
}

#[test]
fn sponsor_stake_flags_in_probation_misconduct_only() {
    let (sp_priv, _, sponsor) = fresh_id();
    let (_, _, entrant) = fresh_id();
    let (rep_priv, _, reporter) = fresh_id();
    let mut archive = InMemoryArchive::new();
    // Probation window [100, 1000).
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &entrant, t(1000), t(100));
    // Adverse conduct inside probation -> counts.
    publish_conduct(
        &mut archive, &entrant, &reporter, &rep_priv, Some("observer-A"),
        ConductKind::Violation, ConductBearing::Charity, t(500),
    );
    // Adverse conduct AFTER probation -> does not count against the sponsor.
    publish_conduct(
        &mut archive, &entrant, &reporter, &rep_priv, Some("observer-A"),
        ConductKind::Violation, ConductBearing::Charity, t(1500),
    );
    // A commendation inside probation -> not adverse, does not count.
    publish_conduct(
        &mut archive, &entrant, &reporter, &rep_priv, None,
        ConductKind::Commendation, ConductBearing::Charity, t(600),
    );

    let report = sponsor_stake(&archive, &sponsor, Some(t(2000)));
    assert_eq!(report.total_sponsees(), 1);
    assert_eq!(report.tainted_sponsees(), 1);
    assert_eq!(report.sponsees[0].adverse_conduct.len(), 1);
    assert_eq!(report.taint_rate(), Some(1.0));
}

#[test]
fn sponsor_stake_taint_rate_none_for_no_sponsees() {
    let (_, _, sponsor) = fresh_id();
    let archive = InMemoryArchive::new();
    let report = sponsor_stake(&archive, &sponsor, Some(t(2000)));
    assert_eq!(report.total_sponsees(), 0);
    assert_eq!(report.taint_rate(), None);
}

#[test]
fn feature_concern_profile_buckets_by_anti_spiral_feature() {
    let (_, _, subject) = fresh_id();
    let (rep_priv, _, reporter) = fresh_id();
    let (rep2_priv, _, reporter2) = fresh_id();
    let mut archive = InMemoryArchive::new();
    // Two concerns on NonUltimacy, one on EnemyLove, from two perspectives.
    publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, Some("observer-A"),
        ConductKind::Concern, ConductBearing::AntiSpiral(AntiSpiralFeature::NonUltimacyOfCompact),
        t(100),
    );
    publish_conduct(
        &mut archive, &subject, &reporter2, &rep2_priv, Some("observer-B"),
        ConductKind::Concern, ConductBearing::AntiSpiral(AntiSpiralFeature::NonUltimacyOfCompact),
        t(200),
    );
    publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, Some("observer-A"),
        ConductKind::Violation, ConductBearing::AntiSpiral(AntiSpiralFeature::EnemyLoveOperational),
        t(300),
    );
    // A commendation must NOT enter the adverse profile.
    publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, None,
        ConductKind::Commendation, ConductBearing::AntiSpiral(AntiSpiralFeature::GraceOverMerit),
        t(400),
    );
    // A non-feature adverse record.
    publish_conduct(
        &mut archive, &subject, &reporter, &rep_priv, Some("observer-A"),
        ConductKind::Concern, ConductBearing::Rescue, t(500),
    );

    let profile = feature_concern_profile(&archive, &subject, Some(t(1000)));
    assert_eq!(profile.per_feature.get("NonUltimacyOfCompact"), Some(&2));
    assert_eq!(profile.per_feature.get("EnemyLoveOperational"), Some(&1));
    assert_eq!(profile.per_feature.get("GraceOverMerit"), None, "commendation excluded");
    assert_eq!(profile.non_feature_adverse, 1);
    assert_eq!(profile.evidence.len(), 4);
    assert_eq!(profile.perspectives, vec!["observer-A".to_string(), "observer-B".to_string()]);
}

// --- v0 jester lenses -----------------------------------------------------

use crate::analytics::{ceremony_cost_decay, rejection_pattern};

/// Non-admission fixture that lets each test pin the (rejecting, presenting,
/// feature) triples directly.
fn publish_inter_nonadmission_multi(
    archive: &mut InMemoryArchive,
    participants: &[(&DidKey, &PrivKey)],
    rejections: Vec<(DidKey, DidKey, AntiSpiralFeature)>,
    decided_at: Timestamp,
) -> Cid {
    let dids: Vec<DidKey> = participants.iter().map(|(d, _)| (*d).clone()).collect();
    let record = NonAdmissionRecord {
        compact_label: "fixture-compact".into(),
        participants: dids,
        rejections: rejections
            .into_iter()
            .map(|(rej, pres, feat)| RejectionDetail {
                rejecting_community: rej,
                presenting_community: pres,
                feature: feat,
                rationale: "fixture rationale".into(),
            })
            .collect(),
        decided_at,
    };
    let (first_did, first_priv) = participants[0];
    let signed =
        Signed::build(record, &[(first_did.as_str(), first_priv)]).expect("sign non-adm");
    archive
        .append(ArchiveEntry::InterCommunityNonAdmission(signed))
        .expect("append")
}

#[test]
fn rejection_pattern_tallies_by_rejector_presenting_and_feature() {
    let (a_priv, _, a) = fresh_id();
    let (_, _, b) = fresh_id();
    let (_, _, c) = fresh_id();
    let (_, _, d) = fresh_id();
    let mut archive = InMemoryArchive::new();
    let parts = [(&a, &a_priv)];
    // A rejects B and C on FallenActor; D rejects B on EnemyLove (later).
    publish_inter_nonadmission_multi(
        &mut archive, &parts,
        vec![(a.clone(), b.clone(), AntiSpiralFeature::FallenActorAnthropology)],
        t(100),
    );
    publish_inter_nonadmission_multi(
        &mut archive, &parts,
        vec![(a.clone(), c.clone(), AntiSpiralFeature::FallenActorAnthropology)],
        t(200),
    );
    publish_inter_nonadmission_multi(
        &mut archive, &parts,
        vec![(d.clone(), b.clone(), AntiSpiralFeature::EnemyLoveOperational)],
        t(300),
    );

    let p = rejection_pattern(&archive, Some(t(1000)));
    assert_eq!(p.total_rejections(), 3);
    assert_eq!(p.by_rejector.get(a.as_str()), Some(&2));
    assert_eq!(p.by_rejector.get(d.as_str()), Some(&1));
    assert_eq!(p.by_presenting.get(b.as_str()), Some(&2));
    assert_eq!(p.by_presenting.get(c.as_str()), Some(&1));
    assert_eq!(p.by_feature.get("FallenActorAnthropology"), Some(&2));
    assert_eq!(p.by_feature.get("EnemyLoveOperational"), Some(&1));
    // Concentration ranking heads with the busiest rejector.
    assert_eq!(p.by_concentration().first().unwrap().0, a.as_str().to_string());
    assert_eq!(p.perspectives.len(), 2, "two distinct rejecting communities");
    // events are oldest-first and traceable.
    assert_eq!(p.events.len(), 3);
    assert!(p.events[0].decided_at <= p.events[1].decided_at);
}

#[test]
fn rejection_pattern_respects_as_of() {
    let (a_priv, _, a) = fresh_id();
    let (_, _, b) = fresh_id();
    let mut archive = InMemoryArchive::new();
    let parts = [(&a, &a_priv)];
    publish_inter_nonadmission_multi(
        &mut archive, &parts,
        vec![(a.clone(), b.clone(), AntiSpiralFeature::FallenActorAnthropology)],
        t(100),
    );
    publish_inter_nonadmission_multi(
        &mut archive, &parts,
        vec![(a.clone(), b.clone(), AntiSpiralFeature::EnemyLoveOperational)],
        t(300),
    );
    // as_of before the second decision excludes it.
    let p = rejection_pattern(&archive, Some(t(250)));
    assert_eq!(p.total_rejections(), 1);
    assert_eq!(p.by_feature.get("EnemyLoveOperational"), None);
}

#[test]
fn rejection_pattern_empty_archive_is_inert() {
    let archive = InMemoryArchive::new();
    let p = rejection_pattern(&archive, Some(t(1000)));
    assert_eq!(p.total_rejections(), 0);
    assert!(p.by_rejector.is_empty());
    assert!(p.perspectives.is_empty());
    assert!(p.by_concentration().is_empty());
}

#[test]
fn ceremony_cost_decay_tracks_witness_decline_and_widening_gaps() {
    let (root_priv, _, comm) = fresh_id();
    let (sp_priv, _, speaker) = fresh_id();
    let mut archive = InMemoryArchive::new();
    // Three rites: witnesses 5 -> 4 -> 2; sealed at 100, 200, 400 (gaps 100, 200).
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 5, t(100));
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 4, t(200));
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 2, t(400));

    let decay = ceremony_cost_decay(&archive, &comm, Some(t(1000)));
    assert_eq!(decay.rite_count(), 3);
    assert_eq!(decay.first_witness_count(), Some(5));
    assert_eq!(decay.last_witness_count(), Some(2));
    assert_eq!(decay.min_witness_count(), Some(2));
    assert_eq!(decay.max_witness_count(), Some(5));
    assert!(decay.witness_count_declined());
    assert_eq!(decay.gaps_seconds, vec![100, 200]);
    assert_eq!(decay.latest_gap_widening(), Some(true));
    assert_eq!(decay.seconds_since_last, Some(600));
    // rites carry their CIDs and are oldest-first.
    assert!(decay.rites[0].sealed_at < decay.rites[2].sealed_at);
}

#[test]
fn ceremony_cost_decay_single_rite_has_no_trend() {
    let (root_priv, _, comm) = fresh_id();
    let (sp_priv, _, speaker) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 3, t(100));

    let decay = ceremony_cost_decay(&archive, &comm, Some(t(500)));
    assert_eq!(decay.rite_count(), 1);
    assert!(decay.gaps_seconds.is_empty());
    assert!(!decay.witness_count_declined(), "no decline from one point");
    assert_eq!(decay.latest_gap_widening(), None);
    assert_eq!(decay.seconds_since_last, Some(400));
}

#[test]
fn ceremony_cost_decay_empty_is_inert() {
    let (_, _, comm) = fresh_id();
    let archive = InMemoryArchive::new();
    let decay = ceremony_cost_decay(&archive, &comm, Some(t(500)));
    assert_eq!(decay.rite_count(), 0);
    assert_eq!(decay.first_witness_count(), None);
    assert_eq!(decay.latest_gap_widening(), None);
    assert_eq!(decay.seconds_since_last, None);
}

#[test]
fn ceremony_cost_decay_respects_as_of() {
    let (root_priv, _, comm) = fresh_id();
    let (sp_priv, _, speaker) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 5, t(100));
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 4, t(200));
    publish_intra_rite(&mut archive, &comm, &root_priv, &speaker, &sp_priv, 2, t(400));

    // as_of before the third rite: only the first two count.
    let decay = ceremony_cost_decay(&archive, &comm, Some(t(250)));
    assert_eq!(decay.rite_count(), 2);
    assert_eq!(decay.gaps_seconds, vec![100]);
    assert!(decay.witness_count_declined(), "4 < 5");
    assert_eq!(decay.latest_gap_widening(), None, "needs >= 2 gaps");
    assert_eq!(decay.seconds_since_last, Some(50));
}

// --- Probationary / Anathema standings ------------------------------------

#[allow(clippy::too_many_arguments)]
fn publish_conduct_with_refs(
    archive: &mut InMemoryArchive,
    subject_did: &DidKey,
    reporter_did: &DidKey,
    reporter_priv: &PrivKey,
    kind: ConductKind,
    references: Vec<Cid>,
    recorded_at: Timestamp,
) -> Cid {
    let rec = ConductRecord {
        subject_community: subject_did.clone(),
        reporters: vec![reporter_did.clone()],
        reporter_perspective: None,
        kind,
        bearing: ConductBearing::Other("designation".into()),
        severity: Severity::Grave,
        ladder_rung: None,
        decision_class: DecisionClass::SecondClass,
        references,
        narrative: "fixture designation".into(),
        observed_at: recorded_at,
        recorded_at,
    };
    let signed =
        Signed::build(rec, &[(reporter_did.as_str(), reporter_priv)]).expect("sign conduct");
    archive
        .append(ArchiveEntry::ConductAttested(signed))
        .expect("append")
}

/// Publish an admitted subject (identity + inter admission with a counterparty)
/// and return its (priv, pub, did) plus the archive-mutating side effects.
fn seed_admitted_subject(archive: &mut InMemoryArchive) -> (PrivKey, PubKey, DidKey) {
    let (subj_priv, subj_pub, subj) = fresh_id();
    let (cp_priv, _, cp) = fresh_id();
    publish_identity(archive, "Subject", &subj, &subj_pub, &subj_priv, t(10));
    publish_inter_admission(archive, &[(&subj, &subj_priv), (&cp, &cp_priv)], t(20));
    (subj_priv, subj_pub, subj)
}

#[test]
fn anathema_standing_from_active_designation_overrides_admission() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (rep_priv, _, rep) = fresh_id();
    publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationAnathema, vec![], t(30),
    );
    let view = lookup(&archive, &subj, Some(t(1000)));
    assert_eq!(view.standing, RecognitionStanding::Anathema);
}

#[test]
fn anathema_lifted_reverts_to_member() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (rep_priv, _, rep) = fresh_id();
    let anath_cid = publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationAnathema, vec![], t(30),
    );
    publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationLifted, vec![anath_cid], t(40),
    );
    let view = lookup(&archive, &subj, Some(t(1000)));
    assert_eq!(view.standing, RecognitionStanding::InCompact);
}

#[test]
fn anathema_lift_is_time_filtered() {
    // The lift is dated after as_of, so at as_of the designation still stands.
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (rep_priv, _, rep) = fresh_id();
    let anath_cid = publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationAnathema, vec![], t(30),
    );
    publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationLifted, vec![anath_cid], t(100),
    );
    let view = lookup(&archive, &subj, Some(t(50)));
    assert_eq!(view.standing, RecognitionStanding::Anathema);
}

#[test]
fn probationary_standing_within_probation_window() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (sp_priv, _, sponsor) = fresh_id();
    // Probation window [100, 1000).
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &subj, t(1000), t(100));
    let view = lookup(&archive, &subj, Some(t(500)));
    assert_eq!(view.standing, RecognitionStanding::Probationary);
}

#[test]
fn probation_elapsed_is_plain_incompact() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (sp_priv, _, sponsor) = fresh_id();
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &subj, t(1000), t(100));
    // as_of == probation_until: the window is half-open, so probation has ended.
    let view = lookup(&archive, &subj, Some(t(1000)));
    assert_eq!(view.standing, RecognitionStanding::InCompact);
}

#[test]
fn anathema_overrides_probation() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (sp_priv, _, sponsor) = fresh_id();
    let (rep_priv, _, rep) = fresh_id();
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &subj, t(1000), t(100));
    publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationAnathema, vec![], t(200),
    );
    let view = lookup(&archive, &subj, Some(t(500)));
    assert_eq!(view.standing, RecognitionStanding::Anathema);
}

#[test]
fn appended_revocation_supersedes_grant_in_active_delegations() {
    let (root_priv, root_pub, did) = fresh_id();
    let (_, sp_pub, sp_did) = fresh_id();
    let mut archive = InMemoryArchive::new();
    publish_identity(&mut archive, "Alpha", &did, &root_pub, &root_priv, t(100));
    publish_delegation(
        &mut archive, &did, &root_priv, &sp_did, &sp_pub,
        DelegationScope::Reciter, t(200), None, None,
    );
    // Before revocation: the speaker is active.
    let before = lookup(&archive, &did, Some(t(300)));
    assert_eq!(before.active_delegations.len(), 1);

    // Superseding revocation for the SAME speaker (later issued_at, revoked_at).
    publish_delegation(
        &mut archive, &did, &root_priv, &sp_did, &sp_pub,
        DelegationScope::Reciter, t(400), None, Some(t(400)),
    );
    let after = lookup(&archive, &did, Some(t(500)));
    assert!(
        after.active_delegations.is_empty(),
        "revoked speaker must not appear in active_delegations"
    );
    // Non-retroactive: at t(300) the earlier grant is still the winning record.
    let mid = lookup(&archive, &did, Some(t(300)));
    assert_eq!(mid.active_delegations.len(), 1);

    // A re-grant reactivates.
    publish_delegation(
        &mut archive, &did, &root_priv, &sp_did, &sp_pub,
        DelegationScope::Reciter, t(600), None, None,
    );
    let regranted = lookup(&archive, &did, Some(t(700)));
    assert_eq!(regranted.active_delegations.len(), 1);
    assert_eq!(regranted.active_delegations[0].subject_did, sp_did);
}

// --- §4 dispute projection --------------------------------------------------

use kc_mediation::{DecisionOutcome, DisputeRecord, MediationDecision};

fn publish_dispute(
    archive: &mut InMemoryArchive,
    complainant: &DidKey,
    comp_priv: &PrivKey,
    respondent: &DidKey,
    filed_at: Timestamp,
) -> Cid {
    let d = DisputeRecord {
        complainant: complainant.clone(),
        respondent: respondent.clone(),
        rung: LadderRung::MediatedArbitration,
        summary: "fixture dispute".into(),
        references: vec![],
        filed_at,
    };
    let signed = Signed::build(d, &[(complainant.as_str(), comp_priv)]).expect("sign dispute");
    archive.append(ArchiveEntry::DisputeFiled(signed)).expect("append")
}

fn publish_decision(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    outcome: DecisionOutcome,
    decided_at: Timestamp,
) -> Cid {
    let panel_dids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
    let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
    let dec = MediationDecision {
        dispute_cid,
        panel: panel_dids,
        outcome,
        reasoning: "fixture".into(),
        decided_at,
    };
    let signed = Signed::build(dec, &pairs).expect("sign decision");
    archive.append(ArchiveEntry::MediationDecided(signed)).expect("append")
}

#[test]
fn disputes_project_to_both_parties_with_outcome() {
    let (a_priv, _, a) = fresh_id();
    let (_, _, b) = fresh_id();
    let (m1_priv, _, m1) = fresh_id();
    let (m2_priv, _, m2) = fresh_id();
    let (m3_priv, _, m3) = fresh_id();
    let mut archive = InMemoryArchive::new();
    let dispute = publish_dispute(&mut archive, &a, &a_priv, &b, t(100));

    // Before any decision: open for both parties.
    let av = lookup(&archive, &a, Some(t(1000)));
    assert_eq!(av.disputes_filed.len(), 1);
    assert!(av.disputes_against.is_empty());
    assert_eq!(av.disputes_filed[0].decided_outcome, None);
    let bv = lookup(&archive, &b, Some(t(1000)));
    assert_eq!(bv.disputes_against.len(), 1);
    assert_eq!(bv.disputes_against[0].decided_outcome, None);

    // After a decision: outcome shows for both, and respects as_of.
    publish_decision(
        &mut archive, dispute,
        &[(&m1, &m1_priv), (&m2, &m2_priv), (&m3, &m3_priv)],
        DecisionOutcome::Settled, t(500),
    );
    let before = lookup(&archive, &a, Some(t(400)));
    assert_eq!(before.disputes_filed[0].decided_outcome, None, "decision after as_of");
    let after = lookup(&archive, &a, Some(t(1000)));
    assert_eq!(after.disputes_filed[0].decided_outcome, Some("Settled".to_string()));
}

// --- §5 escalation projection & standing ------------------------------------

mod escalation_projection {
    use super::*;
    use kc_conduct::{DecisionClass, LadderRung};
    use kc_escalation::{
        Designation, DesignationTier, DesignationUrgency, LadderTransition, ReEntry,
        ReEntryWitnessAttestation, RestitutionCompletion, RestitutionObligation,
    };
    use kc_mediation::DisputeRecord;

    /// A community that is Identified + InCompact (identity + one admission).
    fn member(archive: &mut InMemoryArchive, name: &str) -> (DidKey, PrivKey) {
        let (sk, pk, did) = fresh_id();
        publish_identity(archive, name, &did, &pk, &sk, t(10));
        let (_ck, cpk, cdid) = fresh_id();
        // counterpart identity so the admission has a second participant
        let cpriv = {
            let (csk, _, _) = fresh_id();
            csk
        };
        let _ = (cpk, cpriv);
        publish_inter_admission(archive, &[(&did, &sk), (&cdid, &sk)], t(20));
        (did, sk)
    }

    fn append_designation(
        archive: &mut InMemoryArchive,
        subject: &DidKey,
        signer: &PrivKey,
        tier: DesignationTier,
        at: Timestamp,
    ) -> Cid {
        let dz = Designation {
            subject_community: subject.clone(),
            tier,
            urgency: DesignationUrgency::Normal,
            deliberators: vec![subject.clone()],
            criteria: "c".into(),
            cited_acts: vec![],
            failed_restoration_cycles: vec![],
            deliberation_cycles: vec![],
            reversal_conditions: match tier {
                DesignationTier::Anathema => Some("rev".into()),
                DesignationTier::HostisHumaniGeneris => None,
            },
            reexamine_after: None,
            decision_class: DecisionClass::SecondClass,
            designated_at: at,
        };
        let signed = Signed::build(dz, &[(subject.as_str(), signer)]).unwrap();
        archive.append(ArchiveEntry::DesignationDecided(signed)).unwrap()
    }

    fn append_reentry(
        archive: &mut InMemoryArchive,
        designation_cid: Cid,
        subject: &DidKey,
        signer: &PrivKey,
        probation_until: Timestamp,
        completed_at: Timestamp,
    ) -> Cid {
        let recitation = Recitation {
            community_did: subject.clone(),
            speaker_did: subject.clone(),
            version: RecitationVersion::current(),
            movement_substrate: "a".into(),
            movement_fallenness: "b".into(),
            movement_recognition: "c".into(),
            movement_common_virtue: "d".into(),
            movement_non_ultimacy: "e".into(),
            language_tag: "en".into(),
            translation_en: None,
            recited_at: completed_at,
        };
        let signed_rec = Signed::build(recitation, &[(subject.as_str(), signer)]).unwrap();
        let att = ReEntryWitnessAttestation {
            witness_did: subject.clone(),
            recitation_cid: signed_rec.payload_cid,
            sincerity_affirmed: true,
            attested_at: completed_at,
        };
        let signed_att = Signed::build(att, &[(subject.as_str(), signer)]).unwrap();
        let re = ReEntry {
            designation_cid,
            petitioner: subject.clone(),
            recitation: signed_rec,
            conduct_accounting: "acct".into(),
            public_acknowledgement: "wrong".into(),
            witnesses: vec![signed_att],
            probation_until,
            deliberators: vec![subject.clone()],
            decision_class: DecisionClass::SecondClass,
            completed_at,
        };
        let signed = Signed::build(re, &[(subject.as_str(), signer)]).unwrap();
        archive.append(ArchiveEntry::ReEntryCompleted(signed)).unwrap()
    }

    #[test]
    fn new_designation_projects_anathema_standing() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        append_designation(&mut a, &s, &sp, DesignationTier::Anathema, t(100));
        let v = lookup(&a, &s, Some(t(200)));
        assert_eq!(v.standing, RecognitionStanding::Anathema);
        assert_eq!(v.designations.len(), 1);
        assert!(!v.designations[0].reversed);
        assert_eq!(v.designations[0].tier, "Anathema");
    }

    #[test]
    fn reentry_reverses_designation_to_probationary_then_member() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let dz = append_designation(&mut a, &s, &sp, DesignationTier::Anathema, t(100));
        append_reentry(&mut a, dz, &s, &sp, t(5000), t(2000));
        // Within the probation window: reversed, and standing is Probationary.
        let during = lookup(&a, &s, Some(t(3000)));
        assert!(during.designations[0].reversed);
        assert_eq!(during.reentries.len(), 1);
        assert_eq!(during.standing, RecognitionStanding::Probationary);
        // After probation: a plain member again.
        let after = lookup(&a, &s, Some(t(9000)));
        assert_eq!(after.standing, RecognitionStanding::InCompact);
    }

    #[test]
    fn ladder_transition_sets_current_rung() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let (r, _rp) = member(&mut a, "R");
        // S files a dispute against R at MediatedArbitration.
        let dispute = DisputeRecord {
            complainant: s.clone(),
            respondent: r.clone(),
            rung: LadderRung::MediatedArbitration,
            summary: "d".into(),
            references: vec![],
            filed_at: t(100),
        };
        let dsigned = Signed::build(dispute, &[(s.as_str(), &sp)]).unwrap();
        let dcid = a.append(ArchiveEntry::DisputeFiled(dsigned)).unwrap();
        // Escalate one step.
        let tr = LadderTransition {
            dispute_cid: dcid,
            from_rung: LadderRung::MediatedArbitration,
            to_rung: LadderRung::LimitedSanctions,
            initiators: vec![s.clone()],
            decision_class: DecisionClass::FirstClass,
            reasoning: "x".into(),
            transitioned_at: t(200),
        };
        let tsigned = Signed::build(tr, &[(s.as_str(), &sp)]).unwrap();
        a.append(ArchiveEntry::LadderAdvanced(tsigned)).unwrap();
        let v = lookup(&a, &s, Some(t(300)));
        assert_eq!(v.disputes_filed.len(), 1);
        assert_eq!(v.disputes_filed[0].rung, LadderRung::MediatedArbitration);
        assert_eq!(v.disputes_filed[0].current_rung, LadderRung::LimitedSanctions);
    }

    #[test]
    fn restitution_obligation_joins_its_verification() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let (b, _bp) = member(&mut a, "B");
        let ob = RestitutionObligation {
            subject_community: s.clone(),
            beneficiary: b.clone(),
            dispute_cid: None,
            terms: "restore".into(),
            imposed_by: vec![b.clone()],
            due_by: None,
            decision_class: DecisionClass::FirstClass,
            imposed_at: t(100),
        };
        let obsigned = Signed::build(ob, &[(s.as_str(), &sp)]).unwrap();
        let ob_cid = a.append(ArchiveEntry::RestitutionObligated(obsigned)).unwrap();
        let done = RestitutionCompletion {
            obligation_cid: ob_cid,
            verifiers: vec![b.clone()],
            satisfied: true,
            narrative: "ok".into(),
            verified_at: t(200),
        };
        let dsigned = Signed::build(done, &[(s.as_str(), &sp)]).unwrap();
        a.append(ArchiveEntry::RestitutionVerified(dsigned)).unwrap();
        let v = lookup(&a, &s, Some(t(300)));
        assert_eq!(v.restitutions.len(), 1);
        assert_eq!(v.restitutions[0].satisfied, Some(true));
        // Unverified obligations project satisfied = None.
        let early = lookup(&a, &s, Some(t(150)));
        assert_eq!(early.restitutions[0].satisfied, None);
    }
}
