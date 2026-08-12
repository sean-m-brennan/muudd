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

// The three lifecycle stamps at the end are the axes under test — issued, expires,
// revoked — and the delegation's whole authority question is which of them are set
// relative to the instant being checked. Collapsing them into a struct would hide the
// combination each test is stating, which is the only thing these tests vary.
#[allow(clippy::too_many_arguments)]
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

/// §2: the standing a dashboard reports and the window the authority pass enforces
/// are one predicate, not two. `derive_standing` calls `kc_archive::probationary_at`;
/// this sweeps instants across both window kinds and both boundaries and pins the
/// equivalence, so a future edit to either side that makes them disagree fails here.
///
/// The one intended asymmetry is precedence, and it is asserted too: an
/// anathematized community is inside the window and does not read `Probationary`.
#[test]
fn probationary_standing_and_the_authority_predicate_agree() {
    let mut archive = InMemoryArchive::new();
    let (_, _, subj) = seed_admitted_subject(&mut archive);
    let (sp_priv, _, sponsor) = fresh_id();
    // Sponsorship window [100, 1000).
    publish_sponsorship(&mut archive, &sponsor, &sp_priv, &subj, t(1000), t(100));

    for at in [0, 99, 100, 101, 500, 999, 1000, 1001, 5000] {
        let standing = lookup(&archive, &subj, Some(t(at))).standing;
        let predicate = kc_archive::probationary_at(&archive, &subj, t(at));
        assert_eq!(
            standing == RecognitionStanding::Probationary,
            predicate,
            "standing {standing:?} and probationary_at {predicate} disagree at t={at}"
        );
    }

    // Precedence is reputation's alone: inside the window, but designated.
    let (rep_priv, _, rep) = fresh_id();
    publish_conduct_with_refs(
        &mut archive, &subj, &rep, &rep_priv,
        ConductKind::DesignationAnathema, vec![], t(200),
    );
    assert_eq!(lookup(&archive, &subj, Some(t(500))).standing, RecognitionStanding::Anathema);
    assert!(
        kc_archive::probationary_at(&archive, &subj, t(500)),
        "the window is a fact about sponsorship; the designation overrides the \
         *standing*, not the window"
    );
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

/// Append one panelist argument on `dispute_cid` and return its CID. The erosion
/// reading is a read-model, so these need no registry behind them.
fn publish_argument(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    author: &DidKey,
    author_priv: &PrivKey,
    submitted_at: Timestamp,
) -> Cid {
    let arg = kc_mediation::ArgumentRecord {
        dispute_cid,
        author: author.clone(),
        role: kc_mediation::ArgumentRole::Panelist,
        position: "fixture".into(),
        evidence: vec![],
        submitted_at,
    };
    let signed = Signed::build(arg, &[(author.as_str(), author_priv)]).expect("sign argument");
    archive.append(ArchiveEntry::ArgumentFiled(signed)).expect("append")
}

/// A decision citing `n` arguments, the earliest `span` seconds before it — the two
/// things the deliberation reading measures.
fn publish_deliberated_decision(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    n: usize,
    span: i64,
    decided_at: Timestamp,
) -> Cid {
    let arguments: Vec<Cid> = (0..n)
        .map(|i| {
            let (d, pk) = panel[i % panel.len()];
            // The first argument sits `span` back; the rest follow it.
            let at = t(decided_at.as_second() - span + i as i64);
            publish_argument(archive, dispute_cid, d, pk, at)
        })
        .collect();
    let panel_dids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
    let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
    let dec = MediationDecision {
        dispute_cid,
        panel: panel_dids,
        outcome: DecisionOutcome::Settled,
        reasoning: "fixture".into(),
        sealed: None,
        arguments,
        decided_at,
    };
    let signed = Signed::build(dec, &pairs).expect("sign decision");
    archive.append(ArchiveEntry::MediationDecided(signed)).expect("append")
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
        sealed: None,
        arguments: vec![],
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

// --- §9 interim injunction projection & capability gating -------------------

mod injunction_projection {
    use super::*;
    use crate::lookup::has_capability;
    use kc_escalation::{
        Capability, InjunctionChallenge, InjunctionConfirmation, InterimInjunction,
    };

    const NINETY_DAYS: i64 = 90 * 24 * 3600;

    /// A community that is Identified + InCompact (identity + one admission).
    fn member(archive: &mut InMemoryArchive, name: &str) -> (DidKey, PrivKey) {
        let (sk, pk, did) = fresh_id();
        publish_identity(archive, name, &did, &pk, &sk, t(10));
        let (_ck, _cpk, cdid) = fresh_id();
        publish_inter_admission(archive, &[(&did, &sk), (&cdid, &sk)], t(20));
        (did, sk)
    }

    /// Projection does not check authority, so the panel is arbitrary; the record
    /// is self-signed by the subject only so `append` accepts the signature.
    fn append_injunction(
        archive: &mut InMemoryArchive,
        subject: &DidKey,
        signer: &PrivKey,
        revoked: Vec<Capability>,
        issued: i64,
        expires: i64,
    ) -> Cid {
        let inj = InterimInjunction {
            subject_community: subject.clone(),
            dispute_cid: None,
            harm: "ongoing interference".into(),
            protective_measures: "suspend the named capabilities".into(),
            revoked_capabilities: revoked,
            panel: vec![subject.clone()],
            issued_at: t(issued),
            expires_at: t(expires),
        };
        let signed = Signed::build(inj, &[(subject.as_str(), signer)]).unwrap();
        archive.append(ArchiveEntry::InjunctionIssued(signed)).unwrap()
    }

    fn append_confirmation(
        archive: &mut InMemoryArchive,
        injunction_cid: Cid,
        subject: &DidKey,
        signer: &PrivKey,
        at: i64,
    ) -> Cid {
        let c = InjunctionConfirmation {
            injunction_cid,
            confirmers: vec![subject.clone()],
            confirmed_at: t(at),
        };
        let signed = Signed::build(c, &[(subject.as_str(), signer)]).unwrap();
        archive.append(ArchiveEntry::InjunctionConfirmed(signed)).unwrap()
    }

    fn append_challenge(
        archive: &mut InMemoryArchive,
        injunction_cid: Cid,
        subject: &DidKey,
        signer: &PrivKey,
        overturned: bool,
        at: i64,
    ) -> Cid {
        let ch = InjunctionChallenge {
            injunction_cid,
            panel: vec![subject.clone()],
            overturned,
            reasoning: "reviewed".into(),
            decided_at: t(at),
        };
        let signed = Signed::build(ch, &[(subject.as_str(), signer)]).unwrap();
        archive.append(ArchiveEntry::InjunctionChallenged(signed)).unwrap()
    }

    #[test]
    fn active_injunction_projects_and_revokes_capabilities() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        append_injunction(&mut a, &s, &sp, vec![Capability::Trade, Capability::Navigation],
            100, 100 + NINETY_DAYS);
        let v = lookup(&a, &s, Some(t(200)));
        assert_eq!(v.injunctions.len(), 1);
        assert_eq!(v.injunctions[0].status, "Active");
        // Union is sorted.
        assert_eq!(v.revoked_capabilities, vec!["Navigation".to_string(), "Trade".to_string()]);
        // The gate reflects it.
        assert!(!has_capability(&a, &s, "Trade", Some(t(200))));
        assert!(!has_capability(&a, &s, "Navigation", Some(t(200))));
        assert!(has_capability(&a, &s, "RescueCoordination", Some(t(200))));
        // Standing is unchanged by an injunction (still a member).
        assert_eq!(v.standing, RecognitionStanding::InCompact);
    }

    #[test]
    fn injunction_lapses_after_window_without_confirmation() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        append_injunction(&mut a, &s, &sp, vec![Capability::Trade], 100, 1000);
        // Within the window it is Active and revokes Trade.
        let during = lookup(&a, &s, Some(t(500)));
        assert_eq!(during.injunctions[0].status, "Active");
        assert!(!has_capability(&a, &s, "Trade", Some(t(500))));
        // Past the window with no confirmation it lapses and Trade is restored.
        let after = lookup(&a, &s, Some(t(2000)));
        assert_eq!(after.injunctions[0].status, "Lapsed");
        assert!(after.revoked_capabilities.is_empty());
        assert!(has_capability(&a, &s, "Trade", Some(t(2000))));
    }

    #[test]
    fn confirmed_injunction_persists_past_expiry() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let inj = append_injunction(&mut a, &s, &sp, vec![Capability::CommonsWithdrawal], 100, 1000);
        append_confirmation(&mut a, inj, &s, &sp, 500); // before expiry
        let after = lookup(&a, &s, Some(t(5000)));
        assert_eq!(after.injunctions[0].status, "Confirmed");
        assert_eq!(after.revoked_capabilities, vec!["CommonsWithdrawal".to_string()]);
        assert!(!has_capability(&a, &s, "CommonsWithdrawal", Some(t(5000))));
    }

    #[test]
    fn confirmation_after_expiry_does_not_confirm() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let inj = append_injunction(&mut a, &s, &sp, vec![Capability::Trade], 100, 1000);
        append_confirmation(&mut a, inj, &s, &sp, 1500); // after expiry — too late
        let after = lookup(&a, &s, Some(t(2000)));
        assert_eq!(after.injunctions[0].status, "Lapsed");
        assert!(has_capability(&a, &s, "Trade", Some(t(2000))));
    }

    #[test]
    fn overturned_injunction_restores_capabilities() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let inj = append_injunction(&mut a, &s, &sp, vec![Capability::Communication],
            100, 100 + NINETY_DAYS);
        append_challenge(&mut a, inj, &s, &sp, true, 300);
        // Before the ruling it was Active and revoked Communication.
        let before = lookup(&a, &s, Some(t(200)));
        assert_eq!(before.injunctions[0].status, "Active");
        assert!(!has_capability(&a, &s, "Communication", Some(t(200))));
        // After the overturning ruling the capability is restored.
        let after = lookup(&a, &s, Some(t(400)));
        assert_eq!(after.injunctions[0].status, "Overturned");
        assert!(after.revoked_capabilities.is_empty());
        assert!(has_capability(&a, &s, "Communication", Some(t(400))));
    }

    #[test]
    fn overturn_wins_over_a_prior_confirmation() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = member(&mut a, "S");
        let inj = append_injunction(&mut a, &s, &sp, vec![Capability::Trade], 100, 1000);
        append_confirmation(&mut a, inj, &s, &sp, 500);
        append_challenge(&mut a, inj, &s, &sp, true, 1500);
        let after = lookup(&a, &s, Some(t(2000)));
        assert_eq!(after.injunctions[0].status, "Overturned");
        assert!(has_capability(&a, &s, "Trade", Some(t(2000))));
    }
}

// --- §7 commons projection --------------------------------------------------

mod commons_projection {
    use super::*;
    use crate::analytics::free_rider_signals;
    use crate::lookup::commons_level;
    use kc_commons::{
        CommonsKind, CommonsRegistration, CommonsSanction, SanctionLevel, UsageRecord,
    };
    use kc_conduct::DecisionClass;

    fn register(
        a: &mut InMemoryArchive,
        kind: CommonsKind,
        appropriators: &[&DidKey],
        priv_: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let dids: Vec<DidKey> = appropriators.iter().map(|d| (*d).clone()).collect();
        let reg = CommonsRegistration {
            kind,
            name: "c".into(),
            description: "d".into(),
            appropriators: dids.clone(),
            provision_rule: "r".into(),
            registered_by: dids,
            registered_at: at,
        };
        // Projection tests do not run the authority pass, so a single signer is fine.
        let signed = Signed::build(reg, &[(appropriators[0].as_str(), priv_)]).unwrap();
        a.append(ArchiveEntry::CommonsRegistered(signed)).unwrap()
    }

    fn usage(
        a: &mut InMemoryArchive,
        commons_cid: Cid,
        who: &DidKey,
        who_priv: &PrivKey,
        consumed: u64,
        contributed: u64,
        at: Timestamp,
    ) -> Cid {
        let u = UsageRecord {
            commons_cid,
            community: who.clone(),
            consumed,
            contributed,
            period_start: at,
            period_end: at,
            recorded_at: at,
        };
        let signed = Signed::build(u, &[(who.as_str(), who_priv)]).unwrap();
        a.append(ArchiveEntry::UsageAccounted(signed)).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn sanction(
        a: &mut InMemoryArchive,
        commons_cid: Cid,
        subject: &DidKey,
        imposer: &DidKey,
        imp_priv: &PrivKey,
        from: SanctionLevel,
        to: SanctionLevel,
        dc: DecisionClass,
        at: Timestamp,
    ) -> Cid {
        let s = CommonsSanction {
            commons_cid,
            subject_community: subject.clone(),
            from_level: from,
            to_level: to,
            basis: "b".into(),
            references: vec![],
            imposers: vec![imposer.clone()],
            decision_class: dc,
            decided_at: at,
        };
        let signed = Signed::build(s, &[(imposer.as_str(), imp_priv)]).unwrap();
        a.append(ArchiveEntry::SanctionApplied(signed)).unwrap()
    }

    #[test]
    fn commons_sanction_projects_current_level_and_gate() {
        let (a_priv, _, ca) = fresh_id();
        let (_, _, cb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let commons = register(&mut arc, CommonsKind::Communication, &[&ca, &cb], &a_priv, t(50));
        sanction(&mut arc, commons, &cb, &ca, &a_priv,
            SanctionLevel::None, SanctionLevel::Monitoring, DecisionClass::FirstClass, t(100));
        sanction(&mut arc, commons, &cb, &ca, &a_priv,
            SanctionLevel::Monitoring, SanctionLevel::Warning, DecisionClass::FirstClass, t(200));
        sanction(&mut arc, commons, &cb, &ca, &a_priv,
            SanctionLevel::Warning, SanctionLevel::Restriction, DecisionClass::SecondClass, t(300));

        // The projection carries the latest (current) level, joined with the kind.
        let view = lookup(&arc, &cb, Some(t(1000)));
        assert_eq!(view.commons_sanctions.len(), 1);
        let cs = &view.commons_sanctions[0];
        assert_eq!(cs.current_level, "Restriction");
        assert_eq!(cs.kind, "Communication");
        assert_eq!(cs.commons_cid, commons);

        // The gate free fn agrees and respects as_of.
        assert_eq!(commons_level(&arc, &cb, &commons, Some(t(1000))), SanctionLevel::Restriction);
        assert_eq!(commons_level(&arc, &cb, &commons, Some(t(150))), SanctionLevel::Monitoring);
        assert_eq!(commons_level(&arc, &cb, &commons, Some(t(10))), SanctionLevel::None);
    }

    #[test]
    fn restored_commons_drops_out_of_projection() {
        let (a_priv, _, ca) = fresh_id();
        let (_, _, cb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let commons = register(&mut arc, CommonsKind::Rescue, &[&ca, &cb], &a_priv, t(50));
        sanction(&mut arc, commons, &cb, &ca, &a_priv,
            SanctionLevel::None, SanctionLevel::Monitoring, DecisionClass::FirstClass, t(100));
        sanction(&mut arc, commons, &cb, &ca, &a_priv,
            SanctionLevel::Monitoring, SanctionLevel::None, DecisionClass::FirstClass, t(200));
        // Back at rest: not surfaced, gate reports None.
        let view = lookup(&arc, &cb, Some(t(1000)));
        assert!(view.commons_sanctions.is_empty());
        assert_eq!(commons_level(&arc, &cb, &commons, Some(t(1000))), SanctionLevel::None);
    }

    #[test]
    fn free_rider_signals_tally_and_net_draw() {
        let (a_priv, _, ca) = fresh_id();
        let (b_priv, _, cb) = fresh_id();
        let (c_priv, _, cc) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let commons = register(&mut arc, CommonsKind::Navigation, &[&ca, &cb, &cc], &a_priv, t(50));
        // B draws heavily and returns little; C contributes more than it draws.
        usage(&mut arc, commons, &cb, &b_priv, 800, 100, t(100));
        usage(&mut arc, commons, &cb, &b_priv, 400, 100, t(200));
        usage(&mut arc, commons, &cc, &c_priv, 100, 500, t(100));

        let report = free_rider_signals(&arc, &commons, Some(t(1000)));
        assert_eq!(report.total_consumed(), 1300);
        assert_eq!(report.total_contributed(), 700);
        // Only B has a positive net draw (1200 consumed - 200 contributed = 1000).
        let ranked = report.by_net_draw();
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].0, cb.as_str());
        assert_eq!(ranked[0].1, 1000);
        // B's tally aggregates its two records.
        let b_tally = report.tallies.iter().find(|t| t.community == cb).unwrap();
        assert_eq!(b_tally.record_count, 2);
        assert_eq!(b_tally.evidence.len(), 2);
        assert_eq!(b_tally.draw_ratio(), Some(1200.0 / 200.0));
    }

    #[test]
    fn free_rider_signals_respects_as_of() {
        let (a_priv, _, ca) = fresh_id();
        let (b_priv, _, cb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let commons = register(&mut arc, CommonsKind::KnowledgeArchive, &[&ca, &cb], &a_priv, t(50));
        usage(&mut arc, commons, &cb, &b_priv, 100, 0, t(500));
        // Before the usage record: nothing tallied.
        let early = free_rider_signals(&arc, &commons, Some(t(100)));
        assert_eq!(early.total_consumed(), 0);
        let late = free_rider_signals(&arc, &commons, Some(t(600)));
        assert_eq!(late.total_consumed(), 100);
    }
}

// --- §8 observer / jester projection ----------------------------------------

mod observer_projection {
    use super::*;
    use crate::analytics::observer_dashboard;
    use kc_core::{KeyImage, SourcePreimage, ThresholdCustody};
    use kc_observer::{
        ObserverEntry, ObserverRegistry, ObserverReport, ReportResponse, SourceExculpation,
        SourceReveal, WhistleblowerReport,
    };

    fn register_observer(a: &mut InMemoryArchive, community: &DidKey, at: Timestamp) -> (DidKey, PrivKey) {
        let (opriv, opub) = keys::generate(Alg::Ed25519).unwrap();
        let odid = DidKey::from_pubkey(&opub).unwrap();
        let reg = ObserverRegistry {
            nominating_community: community.clone(),
            entries: vec![ObserverEntry {
                observer_did: odid.clone(),
                observer_pubkey: opub,
                declared_perspective: "labor-rights".into(),
                valid_from: at,
                valid_until: None,
            }],
            issued_at: at,
        };
        // Projection tests skip the authority pass; sign with the observer's own key.
        let signed = Signed::build(reg, &[(community.as_str(), &opriv)]).unwrap();
        // (registry is not signature-checked here; we just need it in the archive)
        a.append(ArchiveEntry::ObserverRegistryUpdated(signed)).unwrap();
        (odid, opriv)
    }

    fn publish_report(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        obs: &DidKey,
        obs_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let rep = ObserverReport {
            subject_community: subject.clone(),
            observer_did: obs.clone(),
            declared_perspective: "labor-rights".into(),
            summary: "authoritarian internal hierarchy".into(),
            references: vec![],
            observed_at: at,
            reported_at: at,
        };
        let signed = Signed::build(rep, &[(obs.as_str(), obs_priv)]).unwrap();
        a.append(ArchiveEntry::ObserverReportPublished(signed)).unwrap()
    }

    fn respond(a: &mut InMemoryArchive, report_cid: Cid, who: &DidKey, who_priv: &PrivKey, at: Timestamp) -> Cid {
        let r = ReportResponse {
            report_cid,
            responding_community: who.clone(),
            response: "council is by consent".into(),
            responded_at: at,
        };
        let signed = Signed::build(r, &[(who.as_str(), who_priv)]).unwrap();
        a.append(ArchiveEntry::ReportResponsePublished(signed)).unwrap()
    }

    /// A filing by a fresh anonymous source. Projection reads do not run the
    /// authority pass, so the ring here is a plausible two-key set rather than the
    /// accused community's real one; what matters to the projection is that the
    /// attestation carries a genuine key image.
    fn whistleblow(a: &mut InMemoryArchive, accused: &DidKey, panel: &DidKey, panel_priv: &PrivKey, at: Timestamp) -> Cid {
        let (source_priv, source_pub) = keys::generate(Alg::Ed25519).unwrap();
        whistleblow_from(a, accused, panel, panel_priv, &source_priv, &source_pub, at)
    }

    /// A filing by a *named* source key, so a test can file twice from one source.
    fn whistleblow_from(
        a: &mut InMemoryArchive,
        accused: &DidKey,
        panel: &DidKey,
        panel_priv: &PrivKey,
        source_priv: &PrivKey,
        source_pub: &PubKey,
        at: Timestamp,
    ) -> Cid {
        let (_, decoy) = keys::generate(Alg::Ed25519).unwrap();
        let ring = vec![source_pub.clone(), decoy];
        let preimage = SourcePreimage::new(DidKey::from_pubkey(source_pub).unwrap().as_str());
        let commitment = preimage.commitment().unwrap();
        // Projection reads do not run the authority pass, so this seals to the panel's
        // own `did:key` rather than to a registered encryption key.
        let custody = ThresholdCustody::seal(
            &kc_core::canonical::to_canonical_cbor(&preimage).unwrap(),
            &[panel.clone(), panel.clone()],
            kc_core::didkey_encryption_resolver,
            2,
        )
        .unwrap();
        let insider = WhistleblowerReport::attest(
            accused.clone(),
            commitment,
            "coerced labor",
            at,
            ring,
            0,
            source_priv,
        )
        .unwrap();
        let wb = WhistleblowerReport {
            accused_community: accused.clone(),
            source_commitment: commitment,
            claim: "coerced labor".into(),
            references: vec![],
            reception_panel: vec![panel.clone()],
            filed_at: at,
            insider,
            custody,
        };
        let signed = Signed::build(wb, &[(panel.as_str(), panel_priv)]).unwrap();
        a.append(ArchiveEntry::WhistleblowerFiled(signed)).unwrap()
    }

    #[test]
    fn observer_report_projects_onto_subject_with_responses() {
        let (a_priv, _, subject) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let (obs, obs_priv) = register_observer(&mut arc, &subject, t(10));  // registry only
        let rep = publish_report(&mut arc, &subject, &obs, &obs_priv, t(100));
        respond(&mut arc, rep, &subject, &a_priv, t(200));

        let v = lookup(&arc, &subject, Some(t(1000)));
        assert_eq!(v.observer_reports.len(), 1);
        assert_eq!(v.observer_reports[0].declared_perspective, "labor-rights");
        assert_eq!(v.observer_reports[0].responses.len(), 1);
        // A community with no report against it sees none.
        let (_, _, other) = fresh_id();
        assert!(lookup(&arc, &other, Some(t(1000))).observer_reports.is_empty());
    }

    #[test]
    fn observer_report_respects_as_of() {
        let (_, _, subject) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let (obs, obs_priv) = register_observer(&mut arc, &subject, t(10));
        publish_report(&mut arc, &subject, &obs, &obs_priv, t(500));
        assert!(lookup(&arc, &subject, Some(t(100))).observer_reports.is_empty());
        assert_eq!(lookup(&arc, &subject, Some(t(600))).observer_reports.len(), 1);
    }

    #[test]
    fn whistleblower_projects_without_source_and_tracks_reveal() {
        let (p_priv, _, panel) = fresh_id();
        let (_, _, accused) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let wb = whistleblow(&mut arc, &accused, &panel, &p_priv, t(100));
        let v = lookup(&arc, &accused, Some(t(500)));
        assert_eq!(v.whistleblower_reports_against.len(), 1);
        assert!(!v.whistleblower_reports_against[0].source_revealed);

        // A for-cause reveal flips the flag (still no source in the projection).
        let rev = SourceReveal {
            report_cid: wb,
            panel: vec![panel.clone()],
            cause: "recanted".into(),
            disclosed_source: "did:key:zMember".into(),
            shares: vec![],
            revealed_at: t(600),
        };
        let signed = Signed::build(rev, &[(panel.as_str(), &p_priv)]).unwrap();
        arc.append(ArchiveEntry::SourceRevealed(signed)).unwrap();
        let after = lookup(&arc, &accused, Some(t(700)));
        assert!(after.whistleblower_reports_against[0].source_revealed);
    }

    #[test]
    fn observer_dashboard_tallies_reports_and_whistleblowers() {
        let (_, _, s1) = fresh_id();
        let (_, _, s2) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let (obs, obs_priv) = register_observer(&mut arc, &s1, t(10));
        publish_report(&mut arc, &s1, &obs, &obs_priv, t(100));
        publish_report(&mut arc, &s1, &obs, &obs_priv, t(110));
        publish_report(&mut arc, &s2, &obs, &obs_priv, t(120));
        let (p_priv, _, panel) = fresh_id();
        whistleblow(&mut arc, &s2, &panel, &p_priv, t(130));

        let dash = observer_dashboard(&arc, Some(t(1000)));
        // s1 has 2 reports, s2 has 1; ranking descending.
        assert_eq!(dash.observer_reports_by_subject[0], (s1.as_str().to_string(), 2));
        assert_eq!(dash.whistleblowers_by_accused, vec![(s2.as_str().to_string(), 1)]);
        assert_eq!(dash.distinct_sources_by_accused, vec![(s2.as_str().to_string(), 1)]);
    }

    #[test]
    fn the_dashboard_separates_many_sources_from_one_source_filing_often() {
        // The flood case: without distinct-source counting, three filings look the
        // same whether they come from three insiders or from one voice repeated.
        let (_, _, many) = fresh_id();
        let (_, _, flooded) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let (p_priv, _, panel) = fresh_id();
        for i in 0..3 {
            whistleblow(&mut arc, &many, &panel, &p_priv, t(100 + i));
        }
        let (one_priv, one_pub) = keys::generate(Alg::Ed25519).unwrap();
        for i in 0..3 {
            whistleblow_from(&mut arc, &flooded, &panel, &p_priv, &one_priv, &one_pub, t(200 + i));
        }
        let dash = observer_dashboard(&arc, Some(t(1000)));
        let count = |v: &Vec<(String, usize)>, d: &DidKey| {
            v.iter().find(|(k, _)| k == d.as_str()).map(|(_, n)| *n).unwrap_or(0)
        };
        // Same filing count either way...
        assert_eq!(count(&dash.whistleblowers_by_accused, &many), 3);
        assert_eq!(count(&dash.whistleblowers_by_accused, &flooded), 3);
        // ...but three voices against one.
        assert_eq!(count(&dash.distinct_sources_by_accused, &many), 3);
        assert_eq!(count(&dash.distinct_sources_by_accused, &flooded), 1);
    }

    #[test]
    fn a_filings_source_tag_projects_and_exculpations_are_counted() {
        let (p_priv, _, panel) = fresh_id();
        let (_, _, accused) = fresh_id();
        let mut arc = InMemoryArchive::new();
        let (source_priv, source_pub) = keys::generate(Alg::Ed25519).unwrap();
        let wb =
            whistleblow_from(&mut arc, &accused, &panel, &p_priv, &source_priv, &source_pub, t(100));
        let v = lookup(&arc, &accused, Some(t(500)));
        let summary = &v.whistleblower_reports_against[0];
        // The tag is the source's key image — an anonymous handle, not an identity.
        assert_eq!(summary.source_tag, KeyImage::of(&source_priv).unwrap().to_hex());
        assert_eq!(summary.exculpations, 0);

        // A member's published denial is counted, so a contested disclosure is legible.
        let (claim_priv, claim_pub) = keys::generate(Alg::Ed25519).unwrap();
        let claimant = DidKey::from_pubkey(&claim_pub).unwrap();
        let exc = SourceExculpation::prove(wb, claimant.clone(), &claim_priv, t(600)).unwrap();
        let signed = Signed::build(exc, &[(claimant.as_str(), &claim_priv)]).unwrap();
        arc.append(ArchiveEntry::SourceExculpated(signed)).unwrap();
        let after = lookup(&arc, &accused, Some(t(700)));
        assert_eq!(after.whistleblower_reports_against[0].exculpations, 1);
    }

    /// Register `n` observers for `community` on a bounded window from `at`,
    /// signed by `community_priv` (projection reads skip signature checks).
    fn register_n(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        n: usize,
        at: Timestamp,
    ) -> Vec<DidKey> {
        let mut entries = vec![];
        let mut dids = vec![];
        for _ in 0..n {
            let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            let od = DidKey::from_pubkey(&pk).unwrap();
            entries.push(ObserverEntry {
                observer_did: od.clone(),
                observer_pubkey: pk,
                declared_perspective: "labor-rights".into(),
                valid_from: at,
                valid_until: Some(t(at.as_second() + 1000)),
            });
            dids.push(od);
        }
        let reg = ObserverRegistry { nominating_community: community.clone(), entries, issued_at: at };
        let signed = Signed::build(reg, &[(community.as_str(), community_priv)]).unwrap();
        a.append(ArchiveEntry::ObserverRegistryUpdated(signed)).unwrap();
        dids
    }

    #[test]
    fn observer_pool_concentration_ranks_and_flags_majority() {
        use crate::analytics::{observer_pool_concentration, ObserverPoolShare};
        let (a_priv, _, aa) = fresh_id();
        let (b_priv, _, bb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        register_n(&mut arc, &aa, &a_priv, 3, t(10));
        register_n(&mut arc, &bb, &b_priv, 2, t(10));

        let pool = observer_pool_concentration(&arc, Some(t(500)));
        assert_eq!(pool.total_active, 5);
        assert_eq!(
            pool.shares[0],
            ObserverPoolShare { community: aa.as_str().to_string(), active: 3 }
        );
        assert_eq!(pool.top_active(), 3);
        assert!(pool.majority_held()); // 3 of 5 — the domination front the cap watches
        assert_eq!(pool.serving.len(), 5);
    }

    #[test]
    fn observer_dashboard_includes_pool_and_substrate() {
        let (a_priv, _, aa) = fresh_id();
        let mut arc = InMemoryArchive::new();
        register_n(&mut arc, &aa, &a_priv, 2, t(10));

        let dash = observer_dashboard(&arc, Some(t(500)));
        assert_eq!(dash.pool.total_active, 2);
        // No constitution ratified: the substrate half is present but not ratified.
        assert!(!dash.substrate.ratified);
    }

    /// The anonymity-set lens: a community with no roll and no delegation shows the
    /// 1 that means "no anonymous channel", and a roll moves the number. Ascending, so
    /// the community that cannot shelter a source is the first row.
    #[test]
    fn observer_dashboard_shows_the_thinnest_anonymity_sets_first() {
        let (a_priv, a_pub, aa) = fresh_id();
        let (b_priv, b_pub, bb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        publish_identity(&mut arc, "A", &aa, &a_pub, &a_priv, t(10));
        publish_identity(&mut arc, "B", &bb, &b_pub, &b_priv, t(10));

        let dash = observer_dashboard(&arc, Some(t(500)));
        assert_eq!(dash.anonymity_set_by_community.len(), 2);
        assert!(dash.anonymity_set_by_community.iter().all(|(_, n)| *n == 1));

        // B publishes a roll of three members; A publishes nothing.
        let members: Vec<kc_identity::RollMember> = (0..3)
            .map(|_| {
                let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
                kc_identity::RollMember {
                    member_did: DidKey::from_pubkey(&pk).unwrap(),
                    member_pubkey: pk,
                }
            })
            .collect();
        let roll = kc_identity::MemberRoll {
            community_did: bb.clone(),
            members,
            as_of: t(20),
            published_at: t(30),
        };
        let signed = Signed::build(roll, &[(bb.as_str(), &b_priv)]).unwrap();
        arc.append(ArchiveEntry::MemberRollPublished(signed)).unwrap();

        let dash = observer_dashboard(&arc, Some(t(500)));
        // A is still 1 and sorts first; B's channel is now four deep.
        assert_eq!(dash.anonymity_set_by_community[0], (aa.as_str().to_string(), 1));
        assert_eq!(dash.anonymity_set_by_community[1], (bb.as_str().to_string(), 4));
    }
}

// --- §9 substrate-deference (constitution) diagnostic -----------------------

mod constitution_diag {
    use super::*;
    use crate::analytics::{
        compact_incapable, constitution_state, erosion_alert, in_structured_fast,
        substrate_deference, ErosionSeverity,
    };
    use kc_constitution::{
        AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
        FastChange, FastPeriod, IncapableMatter,
    };
    use kc_conduct::DecisionClass;
    use kc_mediation::DisputeRecord;

    fn matter(k: &str) -> IncapableMatter {
        IncapableMatter { key: k.into(), description: k.into() }
    }

    fn ratify(a: &mut InMemoryArchive, keys: &[&str], periods: Vec<FastPeriod>, at: Timestamp) {
        let (priv_, _, did) = fresh_id();
        let g = ConstitutionGenesis {
            compact_incapable: keys.iter().map(|k| matter(k)).collect(),
            fast_periods: periods,
            ratifiers: vec![did.clone()],
            genesis_at: at,
        };
        // Diagnostic tests skip the authority pass; a single signer suffices.
        let signed = Signed::build(g, &[(did.as_str(), &priv_)]).unwrap();
        a.append(ArchiveEntry::ConstitutionRatified(signed)).unwrap();
    }

    fn amend(a: &mut InMemoryArchive, kind: AmendmentKind, key: &str, at: Timestamp) {
        let (priv_, _, did) = fresh_id();
        let am = CompactIncapableAmendment {
            kind, matter: matter(key), rationale: "r".into(),
            deciders: vec![did.clone()], decision_class: DecisionClass::SecondClass, decided_at: at,
        };
        let signed = Signed::build(am, &[(did.as_str(), &priv_)]).unwrap();
        a.append(ArchiveEntry::CompactIncapableAmended(signed)).unwrap();
    }

    fn contract_fast(a: &mut InMemoryArchive, period: FastPeriod, at: Timestamp) {
        let (priv_, _, did) = fresh_id();
        let am = FastCadenceAmendment {
            change: FastChange::Contract, period, rationale: "r".into(),
            deciders: vec![did.clone()], decision_class: DecisionClass::SecondClass, decided_at: at,
        };
        let signed = Signed::build(am, &[(did.as_str(), &priv_)]).unwrap();
        a.append(ArchiveEntry::FastCadenceAmended(signed)).unwrap();
    }

    fn file_dispute(a: &mut InMemoryArchive, at: Timestamp) -> Cid {
        let (priv_, _, did) = fresh_id();
        let (_, _, resp) = fresh_id();
        let d = DisputeRecord {
            complainant: did.clone(), respondent: resp, rung: LadderRung::MediatedArbitration,
            summary: "s".into(), references: vec![], filed_at: at,
        };
        let signed = Signed::build(d, &[(did.as_str(), &priv_)]).unwrap();
        a.append(ArchiveEntry::DisputeFiled(signed)).unwrap()
    }

    #[test]
    fn state_reflects_genesis_and_amendments_over_time() {
        let mut a = InMemoryArchive::new();
        assert!(!constitution_state(&a, Some(t(10))).ratified);
        ratify(&mut a, &["internal-theology"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000) , recurrence: None }], t(50));
        amend(&mut a, AmendmentKind::Add, "aesthetics", t(100));

        // Before the add, only the genesis matter is in force.
        let s70 = constitution_state(&a, Some(t(70)));
        assert!(s70.ratified);
        assert_eq!(s70.compact_incapable.len(), 1);
        // After, both.
        let s200 = constitution_state(&a, Some(t(200)));
        assert_eq!(s200.compact_incapable.len(), 2);
        assert!(compact_incapable(&a, "aesthetics", Some(t(200))));
        assert!(!compact_incapable(&a, "aesthetics", Some(t(70))));
    }

    #[test]
    fn structured_fast_gate() {
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["x"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000) , recurrence: None }], t(50));
        assert_eq!(in_structured_fast(&a, t(1500)), Some("annual".to_string()));
        assert_eq!(in_structured_fast(&a, t(2500)), None);
    }

    #[test]
    fn recurring_fast_gate_repeats() {
        // A 1000s fast recurring every 10_000s — the gate should fire in each cycle.
        let mut a = InMemoryArchive::new();
        ratify(
            &mut a,
            &["x"],
            vec![FastPeriod {
                label: "condolence".into(),
                start: t(1000),
                end: t(2000),
                recurrence: Some(10_000),
            }],
            t(50),
        );
        assert_eq!(in_structured_fast(&a, t(1500)), Some("condolence".to_string()));
        assert_eq!(in_structured_fast(&a, t(2500)), None); // in the gap
        assert_eq!(in_structured_fast(&a, t(11_500)), Some("condolence".to_string())); // next cycle
        assert_eq!(in_structured_fast(&a, t(12_500)), None);
    }

    #[test]
    fn net_contraction_and_fast_deviation_surface() {
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["a", "b"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000) , recurrence: None }], t(50));
        // One add, two removes → net contraction.
        amend(&mut a, AmendmentKind::Add, "c", t(100));
        amend(&mut a, AmendmentKind::Remove, "a", t(200));
        amend(&mut a, AmendmentKind::Remove, "b", t(300));
        // A dispute filed *during* the fast window (still inside after the
        // contraction below) is a deviation.
        file_dispute(&mut a, t(1200));
        // A dispute outside the fast is not.
        file_dispute(&mut a, t(3000));
        contract_fast(&mut a, FastPeriod { label: "annual".into(), start: t(1000), end: t(1500) , recurrence: None }, t(400));

        let r = substrate_deference(&a, Some(t(5000)));
        assert!(r.ratified);
        assert_eq!(r.incapable_added, 1);
        assert_eq!(r.incapable_removed, 2);
        assert!(r.contraction_detected);
        assert_eq!(r.fast_contractions, 1);
        assert_eq!(r.fast_deviations.len(), 1);
        assert_eq!(r.fast_deviations[0].kind, "DisputeFiled");
        assert_eq!(r.fast_deviations[0].fast_label, "annual");
    }

    #[test]
    fn erosion_alert_none_when_no_rollback() {
        // Expansion (an Add) and honored fasts are not erosion.
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["a"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000), recurrence: None }], t(50));
        amend(&mut a, AmendmentKind::Add, "b", t(100));
        let al = erosion_alert(&a, Some(t(5000)));
        assert_eq!(al.severity, ErosionSeverity::None);
        assert!(al.reasons.is_empty());
        assert!(!al.list_contraction && !al.fast_contraction);
        assert_eq!(al.active_fast_deviations, 0);
    }

    #[test]
    fn erosion_alert_notice_on_single_isolated_signal() {
        // A single matter removed, nothing else eroded → Notice.
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["a", "b"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000), recurrence: None }], t(50));
        amend(&mut a, AmendmentKind::Remove, "a", t(200));
        let al = erosion_alert(&a, Some(t(5000)));
        assert_eq!(al.severity, ErosionSeverity::Notice);
        assert_eq!(al.reasons.len(), 1);
        assert!(al.list_contraction && !al.fast_contraction);
    }

    #[test]
    fn erosion_alert_escalates_on_multiple_dimensions() {
        // Two distinct dimensions (list + fast contraction) → Alert.
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["a", "b"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000), recurrence: None }], t(50));
        amend(&mut a, AmendmentKind::Remove, "a", t(200));
        contract_fast(&mut a, FastPeriod { label: "annual".into(), start: t(1000), end: t(1500), recurrence: None }, t(400));
        let al = erosion_alert(&a, Some(t(5000)));
        assert_eq!(al.severity, ErosionSeverity::Alert);
        assert_eq!(al.reasons.len(), 2);
        assert!(al.list_contraction && al.fast_contraction);
    }

    #[test]
    fn erosion_alert_escalates_on_repeated_single_dimension() {
        // One dimension eroded more than once (two removes) → Alert, one reason.
        let mut a = InMemoryArchive::new();
        ratify(&mut a, &["a", "b"], vec![FastPeriod { label: "annual".into(), start: t(1000), end: t(2000), recurrence: None }], t(50));
        amend(&mut a, AmendmentKind::Remove, "a", t(200));
        amend(&mut a, AmendmentKind::Remove, "b", t(300));
        let al = erosion_alert(&a, Some(t(5000)));
        assert_eq!(al.severity, ErosionSeverity::Alert);
        assert_eq!(al.reasons.len(), 1);
        assert!(al.list_contraction);
    }
}

// --- uniform drift: the omission signals on the erosion alert ---------------
//
// See `doc/architecture/erosion-legibility.md`. Every pre-existing signal on the
// alert is act-triggered, so a polity that merely *stops* scored None. These
// cover the two signals whose reference is not the polity's own recent
// behaviour: an absolute cadence, and a baseline pinned to the founding rite.

mod uniform_drift {
    use super::*;
    use crate::analytics::{
        ceremony_cost_decay, ceremony_cost_decay_all, erosion_alert, erosion_alert_with,
        overdue_reexaminations, ErosionSeverity, ErosionThresholds,
    };

    /// The default generational cadence in seconds (30 years).
    const CADENCE: i64 = 30 * 365 * 86_400;

    /// A fresh community with the given `(witness_count, sealed_at)` rites.
    fn community_with_rites(archive: &mut InMemoryArchive, rites: &[(usize, Timestamp)]) -> DidKey {
        let (root_priv, _, comm) = fresh_id();
        let (sp_priv, _, speaker) = fresh_id();
        for (witnesses, at) in rites {
            publish_intra_rite(archive, &comm, &root_priv, &speaker, &sp_priv, *witnesses, *at);
        }
        comm
    }

    #[test]
    fn a_polity_that_merely_stopped_is_no_longer_silent() {
        // The regression this slice exists for. Nothing has been rolled back — no
        // amendment, no shortened fast, no act during a fast — so all three
        // act-triggered signals are inert. The polity simply stopped re-affirming.
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(100))]);
        let al = erosion_alert(&a, Some(t(100 + CADENCE + 1)));
        assert!(!al.list_contraction && !al.fast_contraction);
        assert_eq!(al.active_fast_deviations, 0);
        assert_eq!(al.severity, ErosionSeverity::Notice);
        assert_eq!(al.overdue_reaffirmations.len(), 1);
    }

    #[test]
    fn within_the_cadence_the_alert_stays_quiet() {
        // Negative control: the same archive read on the last second before the
        // cadence elapses must be None, or the signal is only a clock.
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(100))]);
        let al = erosion_alert(&a, Some(t(100 + CADENCE)));
        assert_eq!(al.severity, ErosionSeverity::None);
        assert!(al.overdue_reaffirmations.is_empty());
        assert!(al.reasons.is_empty());
    }

    #[test]
    fn overdue_reaffirmation_carries_its_provenance() {
        // Equal witness counts, so the hollowing dimension stays out of the way.
        let mut a = InMemoryArchive::new();
        let comm = community_with_rites(&mut a, &[(5, t(100)), (5, t(500))]);
        let as_of = t(500 + CADENCE + 7);
        let al = erosion_alert(&a, Some(as_of));

        let o = &al.overdue_reaffirmations[0];
        assert_eq!(o.community_did, comm.as_str());
        assert_eq!(o.seconds_since_last, CADENCE + 7);
        assert_eq!(o.cadence_seconds, CADENCE);
        // The CID resolves to the LAST rite, not the founding one.
        let decay = ceremony_cost_decay(&a, &comm, Some(as_of));
        assert_eq!(o.last_rite_cid, decay.rites.last().unwrap().record_cid);
    }

    #[test]
    fn two_overdue_communities_compound_to_alert() {
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(100))]);
        community_with_rites(&mut a, &[(5, t(200))]);
        let al = erosion_alert(&a, Some(t(200 + CADENCE + 1)));
        assert_eq!(al.overdue_reaffirmations.len(), 2);
        assert_eq!(al.severity, ErosionSeverity::Alert);
        assert_eq!(al.reasons.len(), 1, "one dimension, but compounding");
    }

    #[test]
    fn one_community_overdue_by_two_full_cadences_compounds_alone() {
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(100))]);
        let al = erosion_alert(&a, Some(t(100 + 2 * CADENCE + 1)));
        assert_eq!(al.overdue_reaffirmations.len(), 1);
        assert_eq!(al.severity, ErosionSeverity::Alert);
    }

    #[test]
    fn overdue_communities_are_reported_worst_first() {
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(1_000_000))]); // more recent
        community_with_rites(&mut a, &[(5, t(100))]); // longest overdue
        let al = erosion_alert(&a, Some(t(1_000_000 + CADENCE + 1)));
        assert_eq!(al.overdue_reaffirmations.len(), 2);
        assert!(
            al.overdue_reaffirmations[0].seconds_since_last
                > al.overdue_reaffirmations[1].seconds_since_last
        );
    }

    #[test]
    fn hollowing_is_measured_against_the_founding_rite() {
        // Witnesses 5 -> 4 -> 4. The latest is below the FIRST, which a rolling
        // baseline would have absorbed by now. Read inside the cadence so the
        // hollowing dimension is the only one firing.
        let mut a = InMemoryArchive::new();
        let comm = community_with_rites(&mut a, &[(5, t(100)), (4, t(200)), (4, t(300))]);
        let al = erosion_alert(&a, Some(t(1000)));
        assert_eq!(al.hollowing_communities, vec![comm.as_str().to_string()]);
        assert!(al.overdue_reaffirmations.is_empty());
        assert_eq!(al.severity, ErosionSeverity::Notice);
    }

    /// §4's deliberation, on the same pinned baseline as the rite signals: the latest
    /// decision answered fewer arguments than the compact's **first**.
    #[test]
    fn thinning_deliberation_is_measured_against_the_first_decision() {
        let (a_priv, _, ca) = fresh_id();
        let (_, _, cb) = fresh_id();
        let (m1_priv, _, m1) = fresh_id();
        let mut a = InMemoryArchive::new();
        let panel: Vec<(&DidKey, &PrivKey)> = vec![(&m1, &m1_priv)];

        // Six arguments over a week, then four, and four, and four. The latest is below
        // the FIRST while a rolling baseline has fully absorbed it: the median of the
        // priors is 4, so a median reading calls this normal. The **plateau** is where
        // the two baselines diverge — a monotone decline is caught by either, and it is
        // the ceremony that thinned once and stayed thin that only an absolute measure
        // sees.
        for (n, at) in [(6usize, 1_000i64), (4, 2_000), (4, 3_000), (4, 4_000)] {
            let d = publish_dispute(&mut a, &ca, &a_priv, &cb, t(at - 900));
            publish_deliberated_decision(&mut a, d, &panel, n, 604_800, t(at));
        }
        let al = erosion_alert(&a, Some(t(5_000)));
        let th = al.thinning_deliberation.expect("the reading fired");
        assert!(th.argument_count_declined);
        assert!(!th.span_shortened, "the span held at a week each time");
        assert_eq!((th.first_argument_count, th.latest_argument_count), (6, 4));
        assert_eq!(th.decisions, 4);
        assert_eq!(al.severity, ErosionSeverity::Notice, "one dimension, not compounding");
        assert!(al.reasons.iter().any(|r| r.contains("deliberation thinning")));
    }

    /// The span is the other half: a compact whose deliberation collapses from a week
    /// to a second has stopped deliberating across the distances it exists to span,
    /// whatever it still files.
    #[test]
    fn a_collapsing_deliberation_span_is_read_too() {
        let (a_priv, _, ca) = fresh_id();
        let (_, _, cb) = fresh_id();
        let (m1_priv, _, m1) = fresh_id();
        let mut a = InMemoryArchive::new();
        let panel: Vec<(&DidKey, &PrivKey)> = vec![(&m1, &m1_priv)];

        // Same count each time, but the week becomes two seconds.
        for (span, at) in [(604_800i64, 1_000_000i64), (2, 2_000_000)] {
            let d = publish_dispute(&mut a, &ca, &a_priv, &cb, t(at - 700_000));
            publish_deliberated_decision(&mut a, d, &panel, 3, span, t(at));
        }
        let al = erosion_alert(&a, Some(t(3_000_000)));
        let th = al.thinning_deliberation.expect("the reading fired");
        assert!(th.span_shortened);
        assert!(!th.argument_count_declined, "the count held at 3 each time");
        assert_eq!(al.severity, ErosionSeverity::Notice);

        // Both readings firing is no longer one isolated measure moving.
        let d = publish_dispute(&mut a, &ca, &a_priv, &cb, t(2_500_000));
        publish_deliberated_decision(&mut a, d, &panel, 1, 1, t(2_600_000));
        let al = erosion_alert(&a, Some(t(3_000_000)));
        let th = al.thinning_deliberation.expect("the reading fired");
        assert!(th.span_shortened && th.argument_count_declined);
        assert_eq!(al.severity, ErosionSeverity::Alert, "two readings compound");
    }

    /// Negative control against inventing a trend: one decision is not a series, and a
    /// deliberation that *grew* is not erosion.
    #[test]
    fn a_single_or_deepening_deliberation_is_not_thinning() {
        let (a_priv, _, ca) = fresh_id();
        let (_, _, cb) = fresh_id();
        let (m1_priv, _, m1) = fresh_id();
        let mut a = InMemoryArchive::new();
        let panel: Vec<(&DidKey, &PrivKey)> = vec![(&m1, &m1_priv)];

        let d = publish_dispute(&mut a, &ca, &a_priv, &cb, t(500));
        publish_deliberated_decision(&mut a, d, &panel, 3, 100, t(1_000));
        assert!(
            erosion_alert(&a, Some(t(2_000))).thinning_deliberation.is_none(),
            "one decision is not a series"
        );

        let d2 = publish_dispute(&mut a, &ca, &a_priv, &cb, t(1_500));
        publish_deliberated_decision(&mut a, d2, &panel, 5, 200, t(2_000));
        let al = erosion_alert(&a, Some(t(3_000)));
        assert!(al.thinning_deliberation.is_none(), "more arguments over longer is not decay");
        assert_eq!(al.severity, ErosionSeverity::None);
    }

    #[test]
    fn a_community_that_never_sealed_a_rite_is_not_overdue() {
        // Negative control against inventing a cadence: with no rite there is no
        // re-affirmation history, so nothing is late. A founding is not a lapse.
        let a = InMemoryArchive::new();
        let al = erosion_alert(&a, Some(t(10 * CADENCE)));
        assert_eq!(al.severity, ErosionSeverity::None);
        assert!(al.overdue_reaffirmations.is_empty());
        assert!(al.hollowing_communities.is_empty());
    }

    #[test]
    fn the_cadence_is_a_parameter_not_a_constant() {
        // protocol-components.md records that the 25-30y default lacks empirical
        // support and that the eventual shape is a compact-formation parameter. A
        // tighter cadence must therefore change the reading of the same archive.
        let mut a = InMemoryArchive::new();
        community_with_rites(&mut a, &[(5, t(100))]);
        let as_of = Some(t(100 + 5000));
        assert_eq!(erosion_alert(&a, as_of).severity, ErosionSeverity::None);

        let tight = ErosionThresholds { rite_cadence_seconds: 3600, ..Default::default() };
        let al = erosion_alert_with(&a, as_of, &tight);
        assert_eq!(al.severity, ErosionSeverity::Notice);
        assert_eq!(al.overdue_reaffirmations[0].cadence_seconds, 3600);
    }

    #[test]
    fn the_rolling_gap_reading_goes_quiet_once_the_drift_becomes_the_baseline() {
        // Gaps 100, 400, 400, 400: the community widened its cadence fourfold and
        // stayed there. By the last gap 400 IS the median of the priors, so the
        // rolling reading falls silent exactly when the decay has finished setting.
        // Pinned to the founding cadence, the slip is still plainly there.
        let mut a = InMemoryArchive::new();
        let comm =
            community_with_rites(&mut a, &[(5, t(0)), (5, t(100)), (5, t(500)), (5, t(900)), (5, t(1300))]);
        let decay = ceremony_cost_decay(&a, &comm, Some(t(1400)));

        assert_eq!(decay.gaps_seconds, vec![100, 400, 400, 400]);
        assert_eq!(
            decay.latest_gap_widening(),
            Some(false),
            "the degraded cadence has become its own reference"
        );
        assert_eq!(decay.gap_widening_since_founding(), Some(true));
    }

    #[test]
    fn the_pinned_gap_reading_invents_no_signal() {
        // Negative controls: a held cadence is not flagged, and one gap says
        // nothing rather than reporting health.
        let mut a = InMemoryArchive::new();
        let steady = community_with_rites(&mut a, &[(5, t(0)), (5, t(100)), (5, t(200))]);
        assert_eq!(
            ceremony_cost_decay(&a, &steady, Some(t(300))).gap_widening_since_founding(),
            Some(false)
        );

        let mut b = InMemoryArchive::new();
        let young = community_with_rites(&mut b, &[(5, t(0)), (5, t(100))]);
        assert_eq!(
            ceremony_cost_decay(&b, &young, Some(t(200))).gap_widening_since_founding(),
            None,
            "one gap is not a cadence"
        );
    }

    // --- overdue re-examination (the designation follow-through signal) ------

    /// A designation on `subject` carrying a mandated re-examination date.
    fn designate_due(
        archive: &mut InMemoryArchive,
        subject: &DidKey,
        signer: &PrivKey,
        due: Timestamp,
        at: Timestamp,
    ) -> Cid {
        let dz = kc_escalation::Designation {
            subject_community: subject.clone(),
            tier: kc_escalation::DesignationTier::Anathema,
            urgency: kc_escalation::DesignationUrgency::Expedited,
            deliberators: vec![subject.clone()],
            criteria: "c".into(),
            cited_acts: vec![],
            failed_restoration_cycles: vec![],
            deliberation_cycles: vec![],
            reversal_conditions: Some("rev".into()),
            reexamine_after: Some(due),
            decision_class: DecisionClass::SecondClass,
            designated_at: at,
        };
        let signed = Signed::build(dz, &[(subject.as_str(), signer)]).expect("sign designation");
        archive
            .append(ArchiveEntry::DesignationDecided(signed))
            .expect("append")
    }

    /// A re-entry answering `designation_cid` — one of the two reversal paths.
    fn append_reentry_for(
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
        let signed_rec = Signed::build(recitation, &[(subject.as_str(), signer)]).expect("sign rec");
        let att = kc_escalation::ReEntryWitnessAttestation {
            witness_did: subject.clone(),
            recitation_cid: signed_rec.payload_cid,
            sincerity_affirmed: true,
            attested_at: completed_at,
        };
        let signed_att = Signed::build(att, &[(subject.as_str(), signer)]).expect("sign att");
        let re = kc_escalation::ReEntry {
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
        let signed = Signed::build(re, &[(subject.as_str(), signer)]).expect("sign reentry");
        archive
            .append(ArchiveEntry::ReEntryCompleted(signed))
            .expect("append")
    }

    #[test]
    fn a_designation_left_past_its_re_examination_is_flagged() {
        // The most severe act the compact can take, with the least follow-through:
        // before this lens the date could pass and no reading would raise it.
        let mut a = InMemoryArchive::new();
        let (subj_priv, _, subj) = fresh_id();
        designate_due(&mut a, &subj, &subj_priv, t(1000), t(100));

        let al = erosion_alert(&a, Some(t(1500)));
        assert_eq!(al.severity, ErosionSeverity::Notice);
        let (o,) = (&al.overdue_reexaminations[0],);
        assert_eq!(o.subject, subj.as_str());
        assert_eq!(o.tier, "Anathema");
        assert_eq!(o.seconds_overdue, 500);
    }

    #[test]
    fn a_designation_inside_its_re_examination_window_is_quiet() {
        // Negative control: one second before the date, nothing is owed yet.
        let mut a = InMemoryArchive::new();
        let (subj_priv, _, subj) = fresh_id();
        designate_due(&mut a, &subj, &subj_priv, t(1000), t(100));

        let al = erosion_alert(&a, Some(t(999)));
        assert_eq!(al.severity, ErosionSeverity::None);
        assert!(al.overdue_reexaminations.is_empty());
    }

    #[test]
    fn a_re_entry_clears_the_overdue_re_examination() {
        // Reversal path one: the designated community came back.
        let mut a = InMemoryArchive::new();
        let (subj_priv, _, subj) = fresh_id();
        let cid = designate_due(&mut a, &subj, &subj_priv, t(1000), t(100));
        append_reentry_for(&mut a, cid, &subj, &subj_priv, t(2000), t(1200));

        let al = erosion_alert(&a, Some(t(1500)));
        assert!(al.overdue_reexaminations.is_empty());
        assert_eq!(al.severity, ErosionSeverity::None);
    }

    #[test]
    fn a_designation_lifted_conduct_record_clears_it_too() {
        // Reversal path two: the same set `lookup` uses, so there is one
        // definition of a reversed designation.
        let mut a = InMemoryArchive::new();
        let (subj_priv, subj_pub, subj) = fresh_id();
        publish_identity(&mut a, "Subject", &subj, &subj_pub, &subj_priv, t(10));
        let cid = designate_due(&mut a, &subj, &subj_priv, t(1000), t(100));
        publish_conduct_with_refs(
            &mut a,
            &subj,
            &subj,
            &subj_priv,
            ConductKind::DesignationLifted,
            vec![cid],
            t(1100),
        );

        let al = erosion_alert(&a, Some(t(1500)));
        assert!(al.overdue_reexaminations.is_empty());
    }

    #[test]
    fn a_reversal_recorded_after_the_reading_does_not_clear_it() {
        // The lens is `as_of`-honest: a lift dated later than the read is not yet
        // visible, so the designation still reads as standing.
        let mut a = InMemoryArchive::new();
        let (subj_priv, _, subj) = fresh_id();
        let cid = designate_due(&mut a, &subj, &subj_priv, t(1000), t(100));
        append_reentry_for(&mut a, cid, &subj, &subj_priv, t(9000), t(8000));

        assert_eq!(overdue_reexaminations(&a, Some(t(1500))).len(), 1);
        assert!(overdue_reexaminations(&a, Some(t(8500))).is_empty());
    }

    #[test]
    fn two_standing_designations_compound_to_alert() {
        let mut a = InMemoryArchive::new();
        let (p1, _, s1) = fresh_id();
        let (p2, _, s2) = fresh_id();
        designate_due(&mut a, &s1, &p1, t(1000), t(100));
        designate_due(&mut a, &s2, &p2, t(1200), t(100));

        let al = erosion_alert(&a, Some(t(2000)));
        assert_eq!(al.overdue_reexaminations.len(), 2);
        assert_eq!(al.severity, ErosionSeverity::Alert);
        // Worst first.
        assert!(
            al.overdue_reexaminations[0].seconds_overdue
                > al.overdue_reexaminations[1].seconds_overdue
        );
    }

    #[test]
    fn a_designation_without_a_mandated_date_is_never_overdue() {
        // Negative control: `reexamine_after: None` is the Normal path's default.
        // Absence of a promise is not a broken promise.
        let mut a = InMemoryArchive::new();
        let (subj_priv, _, subj) = fresh_id();
        let dz = kc_escalation::Designation {
            subject_community: subj.clone(),
            tier: kc_escalation::DesignationTier::Anathema,
            urgency: kc_escalation::DesignationUrgency::Normal,
            deliberators: vec![subj.clone()],
            criteria: "c".into(),
            cited_acts: vec![],
            failed_restoration_cycles: vec![],
            deliberation_cycles: vec![],
            reversal_conditions: Some("rev".into()),
            reexamine_after: None,
            decision_class: DecisionClass::SecondClass,
            designated_at: t(100),
        };
        let signed = Signed::build(dz, &[(subj.as_str(), &subj_priv)]).expect("sign");
        a.append(ArchiveEntry::DesignationDecided(signed)).expect("append");

        assert!(overdue_reexaminations(&a, Some(t(10_000))).is_empty());
    }

    #[test]
    fn the_all_communities_pass_agrees_with_the_per_community_lens() {
        let mut a = InMemoryArchive::new();
        let c1 = community_with_rites(&mut a, &[(5, t(100)), (3, t(200))]);
        let c2 = community_with_rites(&mut a, &[(2, t(150))]);
        let as_of = Some(t(1000));

        let all = ceremony_cost_decay_all(&a, as_of);
        assert_eq!(all.len(), 2);
        for c in [&c1, &c2] {
            assert_eq!(
                all.get(c.as_str()).expect("community present"),
                &ceremony_cost_decay(&a, c, as_of)
            );
        }
    }
}

// --- move 1 interpretive-plurality lens ------------------------------------

mod interpretive_plurality_lens {
    use super::*;
    use crate::analytics::interpretive_plurality;
    use kc_ceremony::lineage::{InterpreterIndependenceAttestation, SubstrateInterpretation};

    #[allow(clippy::too_many_arguments)]
    fn publish_interpretation(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        lineage_id: &str,
        interpreter: &DidKey,
        recorded_at: Timestamp,
    ) -> Cid {
        let rec = SubstrateInterpretation {
            community_did: community.clone(),
            lineage_id: lineage_id.into(),
            interpreter_did: interpreter.clone(),
            interprets_cid: None,
            divergence: "fixture reading".into(),
            references: vec![],
            recorded_at,
        };
        let signed =
            Signed::build(rec, &[(community.as_str(), community_priv)]).expect("sign interp");
        a.append(ArchiveEntry::SubstrateInterpreted(signed))
            .expect("append")
    }

    #[test]
    fn monopoly_shows_single_interpreter_and_lineage() {
        let (c_priv, _, community) = fresh_id();
        let (_, _, faction) = fresh_id();
        let mut a = InMemoryArchive::new();
        // One faction, one lineage, three readings — the capture signature.
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &faction, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &faction, t(200));
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &faction, t(300));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.interpretations.len(), 3);
        assert_eq!(r.distinct_lineages(), 1);
        assert_eq!(r.distinct_interpreters(), 1);
        assert!(!r.plurality_demonstrated());
        let ranked = r.interpreter_concentration();
        assert_eq!(ranked[0].0, faction.as_str().to_string());
        assert_eq!(ranked[0].1, 3);
    }

    #[test]
    fn plurality_demonstrated_with_distinct_lineages_and_interpreters() {
        let (c_priv, _, community) = fresh_id();
        let (_, _, i1) = fresh_id();
        let (_, _, i2) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "reformist", &i2, t(200));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.distinct_lineages(), 2);
        assert_eq!(r.distinct_interpreters(), 2);
        assert!(r.plurality_demonstrated());
    }

    #[test]
    fn distinctness_keyed_on_interpreter_not_record_count() {
        // A prolific single interpreter across relabeled lineages is still one
        // interpreter — it cannot fake plurality.
        let (c_priv, _, community) = fresh_id();
        let (_, _, faction) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "a", &faction, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "b", &faction, t(200));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.distinct_lineages(), 2);
        assert_eq!(r.distinct_interpreters(), 1);
        assert!(!r.plurality_demonstrated());
    }

    #[test]
    fn empty_for_community_without_interpretations() {
        let (_, _, stranger) = fresh_id();
        let a = InMemoryArchive::new();
        let r = interpretive_plurality(&a, &stranger, Some(t(1000)));
        assert!(r.interpretations.is_empty());
        assert_eq!(r.distinct_lineages(), 0);
        assert!(!r.plurality_demonstrated());
    }

    #[test]
    fn respects_as_of_cutoff() {
        let (c_priv, _, community) = fresh_id();
        let (_, _, i1) = fresh_id();
        let (_, _, i2) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        // A second lineage arrives later — invisible before its time.
        publish_interpretation(&mut a, &community, &c_priv, "reformist", &i2, t(5000));

        let early = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(early.distinct_lineages(), 1);
        assert!(!early.plurality_demonstrated());
        let late = interpretive_plurality(&a, &community, Some(t(6000)));
        assert_eq!(late.distinct_lineages(), 2);
        assert!(late.plurality_demonstrated());
    }

    #[test]
    fn only_the_subject_communitys_readings_count() {
        let (c_priv, _, community) = fresh_id();
        let (o_priv, _, other) = fresh_id();
        let (_, _, faction) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &faction, t(100));
        publish_interpretation(&mut a, &other, &o_priv, "canonical", &faction, t(200));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.interpretations.len(), 1);
    }

    // --- move 1 anti-sockpuppet: interpreter-independence attestation --------

    fn attest_independence(
        a: &mut InMemoryArchive,
        attester: &DidKey,
        attester_priv: &PrivKey,
        community: &DidKey,
        interpreter: &DidKey,
        at: Timestamp,
    ) -> Cid {
        let rec = InterpreterIndependenceAttestation {
            community_did: community.clone(),
            interpreter_did: interpreter.clone(),
            attesting_party: attester.clone(),
            basis: "distinct standing in the compact".into(),
            attested_at: at,
        };
        let signed =
            Signed::build(rec, &[(attester.as_str(), attester_priv)]).expect("sign attestation");
        a.append(ArchiveEntry::InterpreterIndependenceAttested(signed))
            .expect("append")
    }

    #[test]
    fn declared_plurality_without_attestation_is_not_independent() {
        // Two distinct declared interpreters — but nobody has vouched for their
        // independence, so a sockpuppet ring is indistinguishable from the real
        // thing. Declared plurality holds; independent plurality does not.
        let (c_priv, _, community) = fresh_id();
        let (_, _, i1) = fresh_id();
        let (_, _, i2) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "reformist", &i2, t(200));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert!(r.plurality_demonstrated());
        assert_eq!(r.distinct_independent_interpreters(), 0);
        assert!(!r.independent_plurality_demonstrated());
    }

    #[test]
    fn two_third_party_attested_interpreters_are_independent() {
        let (c_priv, _, community) = fresh_id();
        let (_, _, i1) = fresh_id();
        let (_, _, i2) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "reformist", &i2, t(200));
        // A distinct third party vouches for both interpreters' independence.
        attest_independence(&mut a, &witness, &w_priv, &community, &i1, t(300));
        attest_independence(&mut a, &witness, &w_priv, &community, &i2, t(400));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.distinct_independent_interpreters(), 2);
        assert!(r.independent_plurality_demonstrated());
    }

    #[test]
    fn self_and_interpreter_attestations_do_not_count() {
        let (c_priv, _, community) = fresh_id();
        let (i1_priv, _, i1) = fresh_id();
        let (_, _, i2) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        publish_interpretation(&mut a, &community, &c_priv, "reformist", &i2, t(200));
        // The community vouching for its own interpreter is not independence...
        attest_independence(&mut a, &community, &c_priv, &community, &i1, t(300));
        // ...nor is an interpreter vouching for itself.
        attest_independence(&mut a, &i1, &i1_priv, &community, &i1, t(350));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.distinct_independent_interpreters(), 0);
        assert!(!r.independent_plurality_demonstrated());
    }

    #[test]
    fn attestation_for_a_non_interpreter_is_ignored() {
        // Vouching for someone who never authored a reading cannot manufacture an
        // independent interpreter out of nothing.
        let (c_priv, _, community) = fresh_id();
        let (_, _, i1) = fresh_id();
        let (_, _, ghost) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let mut a = InMemoryArchive::new();
        publish_interpretation(&mut a, &community, &c_priv, "canonical", &i1, t(100));
        attest_independence(&mut a, &witness, &w_priv, &community, &ghost, t(300));

        let r = interpretive_plurality(&a, &community, Some(t(1000)));
        assert_eq!(r.distinct_independent_interpreters(), 0);
    }
}

// --- move 4 structured-skepticism lens -------------------------------------

mod structured_skepticism_lens {
    use super::*;
    use crate::analytics::structured_skepticism;
    use kc_provenance::{
        EventCorroboration, EventReport, ReviewAnswer, RetroactiveInterestFinding,
        SkepticismCriterion, SkepticismInvocation, SkepticismReview,
    };

    /// Three mediators standing in for the pool. The lens reads invocation and review
    /// records without re-checking the panel rules, which the archive's authority pass
    /// enforces, so these need only be well-formed.
    fn panel() -> Vec<(DidKey, PrivKey)> {
        (0..3)
            .map(|_| {
                let (p, _, d) = fresh_id();
                (d, p)
            })
            .collect()
    }

    fn invoke(
        a: &mut InMemoryArchive,
        event_cid: Cid,
        criteria: &[SkepticismCriterion],
        hold_seconds: i64,
        at: Timestamp,
    ) -> Cid {
        let pool = panel();
        let pairs: Vec<(&str, &PrivKey)> = pool.iter().map(|(d, p)| (d.as_str(), p)).collect();
        let rec = SkepticismInvocation {
            event_cid,
            panel: pool.iter().map(|(d, _)| d.clone()).collect(),
            criteria: criteria.to_vec(),
            grounds: "onset within the hour of the allocation vote".into(),
            hold_seconds,
            invoked_at: at,
        };
        let signed = Signed::build(rec, &pairs).expect("sign invocation");
        a.append(ArchiveEntry::SkepticismInvoked(signed)).expect("append")
    }

    fn review(a: &mut InMemoryArchive, event_cid: Cid, answer: ReviewAnswer, at: Timestamp) -> Cid {
        let pool = panel();
        let pairs: Vec<(&str, &PrivKey)> = pool.iter().map(|(d, p)| (d.as_str(), p)).collect();
        let rec = SkepticismReview {
            event_cid,
            reviewers: pool.iter().map(|(d, _)| d.clone()).collect(),
            answer,
            reasoning: "the criteria are met on the record".into(),
            reviewed_at: at,
        };
        let signed = Signed::build(rec, &pairs).expect("sign review");
        a.append(ArchiveEntry::SkepticismReviewed(signed)).expect("append")
    }

    #[allow(clippy::too_many_arguments)]
    fn publish_event(
        a: &mut InMemoryArchive,
        originator: &DidKey,
        priv_: &PrivKey,
        charge: bool,
        sudden: bool,
        stake: bool,
        aligned: Option<DidKey>,
        incentives: &str,
        history: &str,
        disclosed: Vec<String>,
        reported_at: Timestamp,
    ) -> Cid {
        let rec = EventReport {
            originator: originator.clone(),
            summary: "e".into(),
            high_interpretive_charge: charge,
            sudden_onset: sudden,
            originator_holds_stake: stake,
            aligned_faction: aligned,
            declared_incentives: incentives.into(),
            information_history: history.into(),
            disclosed_interests: disclosed,
            onset_at: reported_at,
            reported_at,
        };
        let signed = Signed::build(rec, &[(originator.as_str(), priv_)]).expect("sign event");
        a.append(ArchiveEntry::EventReported(signed)).expect("append")
    }

    fn corroborate(
        a: &mut InMemoryArchive,
        event_cid: Cid,
        community: &DidKey,
        priv_: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        corroborate_with(a, event_cid, community, priv_, at, Some("cross-substrate reading"))
    }

    /// Corroborate an event's facts *without* a cross-substrate translation — the
    /// interpretation stays in the originating framing.
    fn corroborate_native(
        a: &mut InMemoryArchive,
        event_cid: Cid,
        community: &DidKey,
        priv_: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        corroborate_with(a, event_cid, community, priv_, at, None)
    }

    fn corroborate_with(
        a: &mut InMemoryArchive,
        event_cid: Cid,
        community: &DidKey,
        priv_: &PrivKey,
        at: Timestamp,
        translation: Option<&str>,
    ) -> Cid {
        let rec = EventCorroboration {
            event_cid,
            corroborating_community: community.clone(),
            translation: translation.map(|s| s.to_string()),
            corroborated_at: at,
        };
        let signed = Signed::build(rec, &[(community.as_str(), priv_)]).expect("sign corr");
        a.append(ArchiveEntry::EventCorroborated(signed))
            .expect("append")
    }

    // A manufactured event: every qualifying trigger, bare provenance.
    fn manufactured(
        a: &mut InMemoryArchive,
        originator: &DidKey,
        priv_: &PrivKey,
        faction: &DidKey,
        at: Timestamp,
    ) -> Cid {
        publish_event(
            a,
            originator,
            priv_,
            true,
            true,
            true,
            Some(faction.clone()),
            "",
            "",
            vec![],
            at,
        )
    }

    /// The lens reads what the archive's hold reads: **either** source places an event
    /// under the protocol.
    ///
    /// The measurement behind this: an event whose originator declared it mundane was
    /// invisible to every filter here, because they all read the originator's own
    /// declaration. That is exactly the event the pool's invocation exists to reach, so
    /// a lens blind to it would report a clean compact at the moment the pool was
    /// acting on a manufactured one.
    #[test]
    fn a_pool_invocation_brings_an_undeclared_event_under_the_lens() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        // Declared mundane by its originator: nothing here fires.
        let ev = publish_event(
            &mut a, &originator, &o_priv, false, false, false, None, "i", "h", vec![], t(100),
        );
        let before = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(before.qualifying_events(), 0);
        assert!(before.under_corroborated(2).is_empty());
        assert!(!before.events[0].under_protocol);

        // The pool finds two criteria the originator did not declare.
        invoke(
            &mut a,
            ev,
            &[
                SkepticismCriterion::SuddenOnset,
                SkepticismCriterion::OriginatorHoldsStake,
            ],
            14 * 24 * 3600,
            t(200),
        );

        let after = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(after.qualifying_events(), 1);
        assert_eq!(after.under_corroborated(2).len(), 1);
        assert!(after.events[0].under_protocol);
        // The originator still declared nothing, and the divergence is legible.
        assert!(!after.events[0].qualifies);
        assert_eq!(after.events[0].undeclared_criteria.len(), 2);
        assert_eq!(after.declaration_divergences().len(), 1);
        assert_eq!(after.events[0].invoked_hold_seconds, Some(14 * 24 * 3600));
    }

    /// A hold nobody has reviewed is the state §4 makes the pool answerable for, and it
    /// is legible whether the hold was invoked or self-declared.
    #[test]
    fn an_unreviewed_hold_is_reported_and_a_review_clears_the_signal() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        assert_eq!(structured_skepticism(&a, Some(t(1000))).unreviewed_holds().len(), 1);

        review(&mut a, ev, ReviewAnswer::Upheld, t(200));
        let r = structured_skepticism(&a, Some(t(1000)));
        assert!(r.unreviewed_holds().is_empty());
        // Upheld changes nothing else: the event is still under the protocol.
        assert_eq!(r.qualifying_events(), 1);
        assert_eq!(r.events[0].reviewed_by.len(), 3);
    }

    /// A pretextual finding releases the event, from a self-declared hold as much as
    /// from an invoked one, and the lens stops reporting it as held.
    #[test]
    fn a_pretextual_finding_releases_the_event_in_the_lens_too() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        assert_eq!(structured_skepticism(&a, Some(t(1000))).qualifying_events(), 1);

        review(&mut a, ev, ReviewAnswer::Pretextual, t(200));
        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.qualifying_events(), 0);
        assert!(r.events[0].released_as_pretextual);
        assert!(r.under_corroborated(2).is_empty());
        // The originator's declaration is untouched: what changed is the hold.
        assert!(r.events[0].qualifies);
    }

    #[test]
    fn manufactured_event_qualifies_and_is_under_corroborated() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        manufactured(&mut a, &originator, &o_priv, &originator, t(100));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.qualifying_events(), 1);
        assert_eq!(r.events[0].corroborators.len(), 0);
        assert!(!r.events[0].provenance_complete);
        assert_eq!(r.under_corroborated(2).len(), 1);
    }

    #[test]
    fn corroboration_by_distinct_communities_clears_the_event() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let (c_priv, _, c) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        corroborate(&mut a, ev, &b, &b_priv, t(200));
        corroborate(&mut a, ev, &c, &c_priv, t(300));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].corroborators.len(), 2);
        assert!(r.under_corroborated(2).is_empty());
    }

    #[test]
    fn self_and_duplicate_corroboration_do_not_count() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        // The originator corroborating itself: excluded.
        corroborate(&mut a, ev, &originator, &o_priv, t(150));
        // One ally corroborating twice: still one distinct community.
        corroborate(&mut a, ev, &b, &b_priv, t(200));
        corroborate(&mut a, ev, &b, &b_priv, t(250));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].corroborators.len(), 1);
        assert_eq!(r.under_corroborated(2).len(), 1);
    }

    #[test]
    fn mundane_event_does_not_qualify() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        // No triggers → does not qualify, so never under-corroborated.
        publish_event(
            &mut a, &originator, &o_priv, false, false, false, None, "", "", vec![], t(100),
        );
        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.qualifying_events(), 0);
        assert!(r.under_corroborated(2).is_empty());
    }

    #[test]
    fn empty_archive_is_inert() {
        let a = InMemoryArchive::new();
        let r = structured_skepticism(&a, Some(t(1000)));
        assert!(r.events.is_empty());
        assert_eq!(r.qualifying_events(), 0);
        assert!(r.under_corroborated(2).is_empty());
    }

    #[test]
    fn respects_as_of_cutoff() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let (c_priv, _, c) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        // Corroborations arrive later — invisible before their time.
        corroborate(&mut a, ev, &b, &b_priv, t(5000));
        corroborate(&mut a, ev, &c, &c_priv, t(6000));

        // Before the corroborations: still under-corroborated.
        let early = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(early.events[0].corroborators.len(), 0);
        assert_eq!(early.under_corroborated(2).len(), 1);
        // After: cleared. An event dated after as_of is invisible entirely.
        let late = structured_skepticism(&a, Some(t(7000)));
        assert_eq!(late.events[0].corroborators.len(), 2);
        assert!(late.under_corroborated(2).is_empty());
        let before_event = structured_skepticism(&a, Some(t(50)));
        assert!(before_event.events.is_empty());
    }

    // --- move 4 cross-substrate translation signal --------------------------

    #[test]
    fn native_framing_corroboration_is_under_translated() {
        // Two distinct communities corroborate the facts, but only in the
        // originating framing (no translation): well-corroborated, yet the
        // interpretation survives only in the manufacturer's substrate.
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let (c_priv, _, c) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        corroborate_native(&mut a, ev, &b, &b_priv, t(200));
        corroborate_native(&mut a, ev, &c, &c_priv, t(300));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].corroborators.len(), 2);
        assert_eq!(r.events[0].translated_corroborators.len(), 0);
        assert!(r.under_corroborated(2).is_empty());
        // Still flagged by the translation signal.
        assert_eq!(r.under_translated(2).len(), 1);
    }

    #[test]
    fn cross_substrate_translation_clears_the_translation_signal() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let (c_priv, _, c) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        corroborate(&mut a, ev, &b, &b_priv, t(200));
        corroborate(&mut a, ev, &c, &c_priv, t(300));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].translated_corroborators.len(), 2);
        assert!(r.under_translated(2).is_empty());
    }

    #[test]
    fn originator_self_translation_does_not_count() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        // The originator "translating" its own event is not cross-substrate.
        corroborate(&mut a, ev, &originator, &o_priv, t(200));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].translated_corroborators.len(), 0);
        assert_eq!(r.under_translated(1).len(), 1);
    }

    // --- move 4 retroactive bad-faith finding -------------------------------

    fn find_interest(
        a: &mut InMemoryArchive,
        event_cid: Cid,
        finder: &DidKey,
        priv_: &PrivKey,
        interest: &str,
        at: Timestamp,
    ) -> Cid {
        let rec = RetroactiveInterestFinding {
            event_cid,
            finder: finder.clone(),
            undisclosed_interest: interest.into(),
            found_at: at,
        };
        let signed = Signed::build(rec, &[(finder.as_str(), priv_)]).expect("sign finding");
        a.append(ArchiveEntry::RetroactiveInterestFound(signed))
            .expect("append")
    }

    #[test]
    fn third_party_finding_impugns_the_event() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        find_interest(&mut a, ev, &b, &b_priv, "owns the disputed claim", t(500));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert_eq!(r.events[0].undisclosed_interests_found.len(), 1);
        assert_eq!(r.retroactively_impugned(), vec![ev]);
    }

    #[test]
    fn originator_self_finding_is_not_bad_faith() {
        let (o_priv, _, originator) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        // The originator disclosing its own omission late is honest, not bad faith.
        find_interest(&mut a, ev, &originator, &o_priv, "a late self-disclosure", t(500));

        let r = structured_skepticism(&a, Some(t(1000)));
        assert!(r.events[0].undisclosed_interests_found.is_empty());
        assert!(r.retroactively_impugned().is_empty());
    }

    #[test]
    fn finding_respects_as_of_cutoff() {
        let (o_priv, _, originator) = fresh_id();
        let (b_priv, _, b) = fresh_id();
        let mut a = InMemoryArchive::new();
        let ev = manufactured(&mut a, &originator, &o_priv, &originator, t(100));
        find_interest(&mut a, ev, &b, &b_priv, "later revealed", t(5000));

        // Before the finding lands, the event is not yet impugned.
        assert!(structured_skepticism(&a, Some(t(1000))).retroactively_impugned().is_empty());
        assert_eq!(structured_skepticism(&a, Some(t(6000))).retroactively_impugned().len(), 1);
    }
}

// --- move 6 adversarial-principal lens -------------------------------------

mod adversarial_principal_lens {
    use super::*;
    use crate::analytics::adversarial_principal;
    use kc_principal::{PrincipalOutput, PrincipalWitnessAttestation};

    #[allow(clippy::too_many_arguments)]
    fn publish_output(
        a: &mut InMemoryArchive,
        home: &DidKey,
        home_priv: &PrivKey,
        principal: &DidKey,
        cohort: &str,
        matter: &str,
        decision: &str,
        favors_home: bool,
        produced_at: Timestamp,
    ) -> Cid {
        let rec = PrincipalOutput {
            principal_did: principal.clone(),
            principal_home: home.clone(),
            cohort: cohort.into(),
            matter: matter.into(),
            decision: decision.into(),
            favors_home,
            produced_at,
        };
        let signed = Signed::build(rec, &[(home.as_str(), home_priv)]).expect("sign output");
        a.append(ArchiveEntry::PrincipalOutputRecorded(signed))
            .expect("append")
    }

    fn attest(
        a: &mut InMemoryArchive,
        witness: &DidKey,
        witness_priv: &PrivKey,
        principal: &DidKey,
        principal_home: &DidKey,
        at: Timestamp,
    ) -> Cid {
        attest_linked(a, witness, witness_priv, principal, principal_home, at, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn attest_linked(
        a: &mut InMemoryArchive,
        witness: &DidKey,
        witness_priv: &PrivKey,
        principal: &DidKey,
        principal_home: &DidKey,
        at: Timestamp,
        prev: Option<Cid>,
    ) -> Cid {
        let rec = PrincipalWitnessAttestation {
            principal_did: principal.clone(),
            principal_home: principal_home.clone(),
            witness_community: witness.clone(),
            basis: "observed continuity".into(),
            prev,
            attested_at: at,
        };
        let signed = Signed::build(rec, &[(witness.as_str(), witness_priv)]).expect("sign attest");
        a.append(ArchiveEntry::PrincipalWitnessed(signed))
            .expect("append")
    }

    // A home community H, its (possibly captured) principal P0, and three peer
    // homes each running a cohort principal that answers "admit".
    fn cohort_archive() -> (InMemoryArchive, PrivKey, DidKey, DidKey) {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        for (i, ans) in ["admit", "admit", "admit"].iter().enumerate() {
            let (peer_priv, _, peer_home) = fresh_id();
            let (_, _, peer_p) = fresh_id();
            publish_output(
                &mut a, &peer_home, &peer_priv, &peer_p, "admissions-review", "M1", ans,
                false, t(100 + i as i64),
            );
        }
        (a, h_priv, home, p0)
    }

    #[test]
    fn self_benefiting_divergence_from_cohort_is_flagged() {
        let (mut a, h_priv, home, p0) = cohort_archive();
        // The captured principal diverges toward its own side.
        publish_output(&mut a, &home, &h_priv, &p0, "admissions-review", "M1", "reject", true, t(200));

        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(r.divergences.len(), 1);
        let d = &r.divergences[0];
        assert_eq!(d.principal_decision, "reject");
        assert_eq!(d.cohort_majority, "admit");
        assert_eq!(d.peers_compared, 3);
        assert!(d.favors_home);
        assert_eq!(r.self_benefiting_divergences(), 1);
    }

    #[test]
    fn agreeing_with_cohort_is_not_flagged() {
        let (mut a, h_priv, home, p0) = cohort_archive();
        publish_output(&mut a, &home, &h_priv, &p0, "admissions-review", "M1", "admit", false, t(200));

        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert!(r.divergences.is_empty());
        assert_eq!(r.self_benefiting_divergences(), 0);
    }

    #[test]
    fn divergence_not_benefiting_home_is_surfaced_but_not_self_benefiting() {
        let (mut a, h_priv, home, p0) = cohort_archive();
        publish_output(&mut a, &home, &h_priv, &p0, "admissions-review", "M1", "reject", false, t(200));

        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(r.divergences.len(), 1);
        assert_eq!(r.self_benefiting_divergences(), 0);
    }

    #[test]
    fn cross_witness_excludes_principal_and_home() {
        let (h_priv, _, home) = fresh_id();
        let (p0_priv, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let mut a = InMemoryArchive::new();
        // Home self-attesting and the principal self-attesting do not count.
        attest(&mut a, &home, &h_priv, &p0, &home, t(100));
        attest(&mut a, &p0, &p0_priv, &p0, &home, t(150));
        let only_home = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(only_home.distinct_cross_witnesses(), 0);

        // A genuine cross-community witness counts.
        attest(&mut a, &witness, &w_priv, &p0, &home, t(200));
        let with_cross = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(with_cross.distinct_cross_witnesses(), 1);
    }

    #[test]
    fn duplicate_cross_witness_counts_once() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let mut a = InMemoryArchive::new();
        let _ = h_priv;
        attest(&mut a, &witness, &w_priv, &p0, &home, t(100));
        attest(&mut a, &witness, &w_priv, &p0, &home, t(200));
        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(r.cross_witnesses.len(), 2);
        assert_eq!(r.distinct_cross_witnesses(), 1);
    }

    #[test]
    fn no_peers_no_divergence() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        // Only the principal's own output; nothing to triangulate against.
        publish_output(&mut a, &home, &h_priv, &p0, "solo", "M1", "reject", true, t(100));
        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert!(r.divergences.is_empty());
    }

    #[test]
    fn empty_and_stranger_are_inert() {
        let (_, _, stranger) = fresh_id();
        let a = InMemoryArchive::new();
        let r = adversarial_principal(&a, &stranger, Some(t(1000)));
        assert!(r.cross_witnesses.is_empty());
        assert!(r.divergences.is_empty());
        assert_eq!(r.distinct_cross_witnesses(), 0);
        assert_eq!(r.self_benefiting_divergences(), 0);
    }

    #[test]
    fn respects_as_of_cutoff() {
        let (mut a, h_priv, home, p0) = cohort_archive();
        // The divergent output arrives late.
        publish_output(&mut a, &home, &h_priv, &p0, "admissions-review", "M1", "reject", true, t(5000));
        let early = adversarial_principal(&a, &p0, Some(t(1000)));
        assert!(early.divergences.is_empty());
        let late = adversarial_principal(&a, &p0, Some(t(6000)));
        assert_eq!(late.self_benefiting_divergences(), 1);
    }

    // --- move 6 statistical triangulation of continuous behavior ------------

    /// Seat a three-peer cohort answering `ans` on `matter`, plus the principal's
    /// own decision on it. `favors_home` marks self-dealing when it diverges.
    #[allow(clippy::too_many_arguments)]
    fn matter_round(
        a: &mut InMemoryArchive,
        home: &DidKey,
        h_priv: &PrivKey,
        p0: &DidKey,
        matter: &str,
        peer_ans: &str,
        own: &str,
        favors_home: bool,
        base_t: i64,
    ) {
        for i in 0..3 {
            let (peer_priv, _, peer_home) = fresh_id();
            let (_, _, peer_p) = fresh_id();
            publish_output(a, &peer_home, &peer_priv, &peer_p, "cohort", matter, peer_ans,
                false, t(base_t + i));
        }
        publish_output(a, home, h_priv, p0, "cohort", matter, own, favors_home, t(base_t + 10));
    }

    #[test]
    fn self_dealing_rate_aggregates_continuous_behavior() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        // Four triangulated matters; the principal self-deals on three of them.
        matter_round(&mut a, &home, &h_priv, &p0, "M1", "admit", "reject", true, 100);
        matter_round(&mut a, &home, &h_priv, &p0, "M2", "admit", "reject", true, 200);
        matter_round(&mut a, &home, &h_priv, &p0, "M3", "admit", "reject", true, 300);
        matter_round(&mut a, &home, &h_priv, &p0, "M4", "admit", "admit", false, 400);

        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(r.outputs_triangulated, 4);
        assert_eq!(r.self_benefiting_divergences(), 3);
        assert!((r.self_dealing_rate() - 0.75).abs() < 1e-9);
        // Systematic at a 0.5 threshold with >= 4 samples; not at 0.8.
        assert!(r.systematic_self_dealing(4, 0.5));
        assert!(!r.systematic_self_dealing(4, 0.8));
    }

    #[test]
    fn one_divergence_over_many_is_not_systematic() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        matter_round(&mut a, &home, &h_priv, &p0, "M1", "admit", "reject", true, 100);
        for (i, m) in ["M2", "M3", "M4", "M5"].iter().enumerate() {
            matter_round(&mut a, &home, &h_priv, &p0, m, "admit", "admit", false, 200 + i as i64 * 100);
        }
        let r = adversarial_principal(&a, &p0, Some(t(2000)));
        assert_eq!(r.outputs_triangulated, 5);
        assert!((r.self_dealing_rate() - 0.2).abs() < 1e-9);
        // A lone divergence in five aligned outputs is not a systematic pattern.
        assert!(!r.systematic_self_dealing(3, 0.5));
    }

    #[test]
    fn small_sample_is_not_systematic_even_at_full_rate() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        // A single self-dealing output: rate 1.0, but too few samples to call.
        matter_round(&mut a, &home, &h_priv, &p0, "M1", "admit", "reject", true, 100);
        let r = adversarial_principal(&a, &p0, Some(t(1000)));
        assert_eq!(r.outputs_triangulated, 1);
        assert!((r.self_dealing_rate() - 1.0).abs() < 1e-9);
        assert!(!r.systematic_self_dealing(3, 0.5));
    }
}

// --- move 6 collusive whole-cohort capture ---------------------------------

mod collusive_cohort_lens {
    use super::*;
    use crate::analytics::collusive_cohorts;
    use kc_principal::PrincipalOutput;

    /// Seat a cohort of `n` principals (each its own home) all deciding `decision`
    /// on `matter`, each marking `favors_home`.
    #[allow(clippy::too_many_arguments)]
    fn seat_cohort(
        a: &mut InMemoryArchive,
        cohort: &str,
        matter: &str,
        decision: &str,
        favors_home: bool,
        n: usize,
        base_t: i64,
    ) {
        for i in 0..n {
            let (home_priv, _, home) = fresh_id();
            let (_, _, principal) = fresh_id();
            let rec = PrincipalOutput {
                principal_did: principal.clone(),
                principal_home: home.clone(),
                cohort: cohort.into(),
                matter: matter.into(),
                decision: decision.into(),
                favors_home,
                produced_at: t(base_t + i as i64),
            };
            let signed = Signed::build(rec, &[(home.as_str(), &home_priv)]).expect("sign");
            a.append(ArchiveEntry::PrincipalOutputRecorded(signed)).expect("append");
        }
    }

    #[test]
    fn self_serving_cohort_diverging_from_consensus_is_flagged() {
        let mut a = InMemoryArchive::new();
        // Two honest cohorts say "admit"; cohort A colludes on "reject" to its benefit.
        seat_cohort(&mut a, "A", "M1", "reject", true, 3, 100);
        seat_cohort(&mut a, "B", "M1", "admit", false, 3, 200);
        seat_cohort(&mut a, "C", "M1", "admit", false, 3, 300);

        let r = collusive_cohorts(&a, Some(t(1000)));
        assert_eq!(r.divergences.len(), 1);
        let d = &r.divergences[0];
        assert_eq!(d.cohort, "A");
        assert_eq!(d.cohort_majority, "reject");
        assert_eq!(d.cross_cohort_consensus, "admit");
        assert_eq!(d.cohorts_compared, 3);
        assert!(d.self_benefiting);
        assert_eq!(r.self_benefiting_cohort_divergences(), 1);
    }

    #[test]
    fn honest_minority_cohort_diverges_but_not_self_benefiting() {
        let mut a = InMemoryArchive::new();
        // Cohort A diverges, but its members do not mark the position self-serving.
        seat_cohort(&mut a, "A", "M1", "reject", false, 3, 100);
        seat_cohort(&mut a, "B", "M1", "admit", false, 3, 200);
        seat_cohort(&mut a, "C", "M1", "admit", false, 3, 300);

        let r = collusive_cohorts(&a, Some(t(1000)));
        assert_eq!(r.divergences.len(), 1);
        assert!(!r.divergences[0].self_benefiting);
        assert_eq!(r.self_benefiting_cohort_divergences(), 0);
    }

    #[test]
    fn cohorts_at_consensus_are_not_flagged() {
        let mut a = InMemoryArchive::new();
        // Everyone agrees — even a self-serving-looking bloc is not a divergence.
        seat_cohort(&mut a, "A", "M1", "admit", true, 3, 100);
        seat_cohort(&mut a, "B", "M1", "admit", false, 3, 200);

        let r = collusive_cohorts(&a, Some(t(1000)));
        assert!(r.divergences.is_empty());
    }

    #[test]
    fn single_cohort_is_inert() {
        let mut a = InMemoryArchive::new();
        // Only one cohort on the matter: nothing to triangulate across.
        seat_cohort(&mut a, "A", "M1", "reject", true, 3, 100);
        let r = collusive_cohorts(&a, Some(t(1000)));
        assert!(r.divergences.is_empty());
    }

    #[test]
    fn respects_as_of_cutoff() {
        let mut a = InMemoryArchive::new();
        seat_cohort(&mut a, "A", "M1", "reject", true, 3, 100);
        seat_cohort(&mut a, "B", "M1", "admit", false, 3, 5000);
        // Before cohort B lands, only one cohort exists → inert.
        assert!(collusive_cohorts(&a, Some(t(1000))).divergences.is_empty());
        assert_eq!(collusive_cohorts(&a, Some(t(6000))).divergences.len(), 1);
    }
}

mod witness_chain_lens {
    use super::*;
    use crate::analytics::principal_witness_chain;
    use kc_principal::PrincipalWitnessAttestation;

    /// Append one attestation link and return its entry CID.
    #[allow(clippy::too_many_arguments)]
    fn link(
        a: &mut InMemoryArchive,
        witness: &DidKey,
        witness_priv: &PrivKey,
        principal: &DidKey,
        principal_home: &DidKey,
        at: Timestamp,
        prev: Option<Cid>,
    ) -> Cid {
        let rec = PrincipalWitnessAttestation {
            principal_did: principal.clone(),
            principal_home: principal_home.clone(),
            witness_community: witness.clone(),
            basis: "observed continuity".into(),
            prev,
            attested_at: at,
        };
        let signed = Signed::build(rec, &[(witness.as_str(), witness_priv)]).expect("sign");
        a.append(ArchiveEntry::PrincipalWitnessed(signed)).expect("append")
    }

    #[test]
    fn linked_cross_community_chain_is_intact_and_separable() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let _ = h_priv;
        let mut a = InMemoryArchive::new();
        let c1 = link(&mut a, &witness, &w_priv, &p0, &home, t(100), None);
        let _c2 = link(&mut a, &witness, &w_priv, &p0, &home, t(200), Some(c1));
        let r = principal_witness_chain(&a, &p0, Some(t(1000)));
        assert_eq!(r.links.len(), 2);
        assert_eq!(r.separable_links(), 2);
        assert_eq!(r.broken_links(), 0);
        assert!(r.is_linear());
        assert!(r.is_intact());
    }

    #[test]
    fn self_and_home_links_are_not_separable() {
        let (h_priv, _, home) = fresh_id();
        let (p0_priv, _, p0) = fresh_id();
        let mut a = InMemoryArchive::new();
        // Home self-attesting and the principal self-attesting are present but
        // not separable — the home holds both keys.
        let c1 = link(&mut a, &home, &h_priv, &p0, &home, t(100), None);
        let _c2 = link(&mut a, &p0, &p0_priv, &p0, &home, t(200), Some(c1));
        let r = principal_witness_chain(&a, &p0, Some(t(1000)));
        assert_eq!(r.links.len(), 2);
        assert_eq!(r.separable_links(), 0);
        assert!(r.is_intact());
    }

    #[test]
    fn dangling_prev_breaks_the_chain() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let _ = h_priv;
        let mut a = InMemoryArchive::new();
        // A link points at a predecessor that is not in the chain (removed/tampered).
        let bogus = Cid::of_bytes(b"not a real link");
        link(&mut a, &witness, &w_priv, &p0, &home, t(200), Some(bogus));
        let r = principal_witness_chain(&a, &p0, Some(t(1000)));
        assert_eq!(r.broken_links(), 1);
        assert!(!r.is_intact());
        assert!(r.is_linear()); // one root-less link, but no fork
    }

    #[test]
    fn two_links_sharing_a_predecessor_is_a_fork() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let _ = h_priv;
        let mut a = InMemoryArchive::new();
        let root = link(&mut a, &witness, &w_priv, &p0, &home, t(100), None);
        link(&mut a, &witness, &w_priv, &p0, &home, t(200), Some(root));
        link(&mut a, &witness, &w_priv, &p0, &home, t(300), Some(root));
        let r = principal_witness_chain(&a, &p0, Some(t(1000)));
        assert_eq!(r.broken_links(), 0); // both prevs resolve
        assert!(!r.is_linear()); // ...but they fork off the same root
        assert!(!r.is_intact());
    }

    #[test]
    fn respects_as_of_and_stranger_is_empty() {
        let (h_priv, _, home) = fresh_id();
        let (_, _, p0) = fresh_id();
        let (w_priv, _, witness) = fresh_id();
        let (_, _, stranger) = fresh_id();
        let _ = h_priv;
        let mut a = InMemoryArchive::new();
        let c1 = link(&mut a, &witness, &w_priv, &p0, &home, t(100), None);
        link(&mut a, &witness, &w_priv, &p0, &home, t(5000), Some(c1));
        // Before the second link lands, only the root is visible.
        assert_eq!(principal_witness_chain(&a, &p0, Some(t(1000))).links.len(), 1);
        assert_eq!(principal_witness_chain(&a, &p0, Some(t(6000))).links.len(), 2);
        // A principal with no attestations has an empty (intact, inert) chain.
        let empty = principal_witness_chain(&a, &stranger, Some(t(6000)));
        assert!(empty.links.is_empty());
        assert_eq!(empty.separable_links(), 0);
        assert!(empty.is_intact());
    }
}

/// The §2 deliberative-capacity lens: how much of a compact's roll carries
/// second-class weight, and whether the compact can act at all.
mod deliberative_capacity_lens {
    use super::*;
    use crate::analytics::deliberative_capacity;

    const Q: usize = 2;

    /// Found a compact of `n` communities by one admission record, returning their
    /// (priv, did) pairs. The roll is derived from that record, never declared.
    fn compact(archive: &mut InMemoryArchive, n: usize) -> Vec<(PrivKey, DidKey)> {
        let members: Vec<(PrivKey, PubKey, DidKey)> = (0..n).map(|_| fresh_id()).collect();
        for (i, (sk, pk, did)) in members.iter().enumerate() {
            publish_identity(archive, &format!("C{i}"), did, pk, sk, t(10));
        }
        let parts: Vec<(&DidKey, &PrivKey)> =
            members.iter().map(|(sk, _, did)| (did, sk)).collect();
        publish_inter_admission(archive, &parts, t(20));
        members.into_iter().map(|(sk, _, did)| (sk, did)).collect()
    }

    #[test]
    fn a_seasoned_roll_carries_its_whole_weight() {
        let mut a = InMemoryArchive::new();
        let members = compact(&mut a, 4);
        let c = deliberative_capacity(&a, "fixture-compact", Q, Some(t(100)));
        assert_eq!(c.members.len(), 4, "the roll is derived from the admission record");
        assert!(c.probationary.is_empty());
        assert_eq!(c.standing(), 4);
        assert_eq!(c.standing_share(), Some(1.0));
        assert!(c.can_act() && c.can_contract());
        let _ = members;
    }

    #[test]
    fn a_probationary_member_is_on_the_roll_and_carries_nothing() {
        let mut a = InMemoryArchive::new();
        let members = compact(&mut a, 4);
        let (sponsor_priv, sponsor) = (&members[0].0, &members[0].1);
        publish_sponsorship(&mut a, sponsor, sponsor_priv, &members[3].1, t(10_000), t(50));
        let c = deliberative_capacity(&a, "fixture-compact", Q, Some(t(100)));
        assert_eq!(c.members.len(), 4, "probation does not remove anyone from the roll");
        assert_eq!(c.probationary.len(), 1);
        assert_eq!(c.standing(), 3);
        assert_eq!(c.standing_share(), Some(0.75));
        assert!(c.can_act() && c.can_contract());
    }

    /// The honest consequence of the withholding, and the reason the lens exists: a
    /// compact that admits a cohort all at once can lose the ability to take a
    /// second-class decision at all. Better surfaced than discovered as a refusal
    /// nobody can account for.
    #[test]
    fn a_compact_can_admit_its_way_out_of_being_able_to_act() {
        let mut a = InMemoryArchive::new();
        let members = compact(&mut a, 4);
        let (sponsor_priv, sponsor) = (&members[0].0, &members[0].1);
        // Three of four enter probation at once.
        for (_, did) in &members[1..] {
            publish_sponsorship(&mut a, sponsor, sponsor_priv, did, t(10_000), t(50));
        }
        let c = deliberative_capacity(&a, "fixture-compact", Q, Some(t(100)));
        assert_eq!(c.standing(), 1);
        assert_eq!(c.standing_share(), Some(0.25));
        assert!(!c.can_act(), "one standing member is below the quorum");
        assert!(!c.can_contract());

        // And it recovers by the passage of time alone — no act required.
        let after = deliberative_capacity(&a, "fixture-compact", Q, Some(t(20_000)));
        assert_eq!(after.standing(), 4);
        assert!(after.can_act() && after.can_contract());
    }

    /// The contraction bar is the *pair*: a compact can be able to expand its
    /// protections and unable to contract them, which is the direction the asymmetry
    /// is meant to run.
    #[test]
    fn expansion_can_be_available_while_contraction_is_not() {
        let mut a = InMemoryArchive::new();
        let members = compact(&mut a, 3);
        let (sponsor_priv, sponsor) = (&members[0].0, &members[0].1);
        publish_sponsorship(&mut a, sponsor, sponsor_priv, &members[2].1, t(10_000), t(50));
        let c = deliberative_capacity(&a, "fixture-compact", Q, Some(t(100)));
        assert_eq!(c.standing(), 2);
        assert!(c.can_act(), "two standing members meet the ordinary quorum");
        assert!(!c.can_contract(), "but not the quorum + 1 contraction bar");
    }

    #[test]
    fn an_unknown_compact_label_is_not_evaluated_rather_than_empty() {
        let mut a = InMemoryArchive::new();
        compact(&mut a, 3);
        let c = deliberative_capacity(&a, "some-other-compact", Q, Some(t(100)));
        assert!(c.members.is_empty());
        assert_eq!(
            c.standing_share(),
            None,
            "an empty roll is not a compact whose seasoning has lapsed"
        );
        assert!(!c.can_act());
    }

    /// The lens reads the roll as of the query, so an admission after `as_of` is not
    /// counted — the same insertion-order/timestamp discipline the gate keeps.
    #[test]
    fn the_roll_is_read_as_of_the_query() {
        let mut a = InMemoryArchive::new();
        compact(&mut a, 3);
        let c = deliberative_capacity(&a, "fixture-compact", Q, Some(t(15)));
        assert!(c.members.is_empty(), "the admission is witnessed at t(20)");
    }
}

/// §4.2 — the mediator pool's composition, by seating community and by training
/// lineage. A read-only board: the caps themselves are enforced by the authority pass.
mod mediator_pool {
    use super::*;
    use crate::analytics::{mediator_pool_concentration, MediatorPoolShare};
    use kc_mediation::{MediatorEntry, MediatorRegistry};

    const YEAR: i64 = 365 * 24 * 3600;

    /// Seat `n` mediators for `community`, optionally all trained by one `trainer`.
    fn seat(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        n: usize,
        trainer: Option<&DidKey>,
        at: Timestamp,
    ) -> Vec<DidKey> {
        let mut entries = vec![];
        let mut dids = vec![];
        for _ in 0..n {
            let (_p, pubk, did) = fresh_id();
            let (_ep, epub, _ed) = fresh_id();
            entries.push(MediatorEntry {
                mediator_did: did.clone(),
                mediator_pubkey: pubk,
                encryption_pubkey: epub,
                valid_from: at,
                valid_until: Some(t(at.as_second() + YEAR)),
                trained_by: trainer.cloned(),
            });
            dids.push(did);
        }
        let reg = MediatorRegistry {
            nominating_community: community.clone(),
            entries,
            issued_at: at,
        };
        let signed = Signed::build(reg, &[(community.as_str(), community_priv)]).unwrap();
        a.append(ArchiveEntry::MediatorRegistryUpdated(signed)).unwrap();
        dids
    }

    /// As [`seat`], with a trainer per seat — a community's mediators need not all
    /// have come up under the same elder.
    fn seat_mixed(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        trainers: &[Option<&DidKey>],
        at: Timestamp,
    ) {
        let mut entries = vec![];
        for trainer in trainers {
            let (_p, pubk, did) = fresh_id();
            let (_ep, epub, _ed) = fresh_id();
            entries.push(MediatorEntry {
                mediator_did: did,
                mediator_pubkey: pubk,
                encryption_pubkey: epub,
                valid_from: at,
                valid_until: Some(t(at.as_second() + YEAR)),
                trained_by: trainer.map(|d| (*d).clone()),
            });
        }
        let reg = MediatorRegistry {
            nominating_community: community.clone(),
            entries,
            issued_at: at,
        };
        let signed = Signed::build(reg, &[(community.as_str(), community_priv)]).unwrap();
        a.append(ArchiveEntry::MediatorRegistryUpdated(signed)).unwrap();
    }

    #[test]
    fn the_pool_is_ranked_by_seating_community_and_flags_a_majority() {
        let (a_priv, _, aa) = fresh_id();
        let (b_priv, _, bb) = fresh_id();
        let mut arc = InMemoryArchive::new();
        seat(&mut arc, &aa, &a_priv, 3, None, t(10));
        seat(&mut arc, &bb, &b_priv, 2, None, t(10));

        let pool = mediator_pool_concentration(&arc, Some(t(500)));
        assert_eq!(pool.total_active, 5);
        assert_eq!(
            pool.shares[0],
            MediatorPoolShare { community: aa.as_str().to_string(), active: 3 }
        );
        assert_eq!(pool.top_active(), 3);
        assert!(pool.majority_held(), "3 of 5 — the domination front the cap watches");
        assert_eq!(pool.serving.len(), 5);
    }

    /// The lineage half: §4's apprenticeship model is "susceptible to capture by the
    /// incumbent generation's biases", and a pool that all traces to one trainer is
    /// that weakness realized. Reported, never gated.
    #[test]
    fn one_trainer_behind_the_pool_is_reported() {
        let (a_priv, _, aa) = fresh_id();
        let (b_priv, _, bb) = fresh_id();
        let (_tp, _tk, elder) = fresh_id();
        let (_tp2, _tk2, other_elder) = fresh_id();
        let mut arc = InMemoryArchive::new();
        // Three seats each, so no community holds a majority — but one elder trained
        // four of the six. The two measures see different things, which is why both
        // exist: a pool can be perfectly balanced between communities and still be one
        // teacher's pool.
        seat_mixed(&mut arc, &aa, &a_priv, &[Some(&elder), Some(&elder), Some(&other_elder)], t(10));
        seat_mixed(&mut arc, &bb, &b_priv, &[Some(&elder), Some(&elder), Some(&other_elder)], t(10));

        let pool = mediator_pool_concentration(&arc, Some(t(500)));
        assert_eq!(pool.total_active, 6);
        assert!(!pool.majority_held(), "3 and 3 between the communities");
        assert_eq!(pool.with_lineage, 6);
        assert_eq!(pool.top_lineage(), 4);
        assert!(pool.lineage_majority_held(), "one elder trained 4 of the 6");
        assert_eq!(
            pool.lineages[0],
            MediatorPoolShare { community: elder.as_str().to_string(), active: 4 }
        );
    }

    /// Silence is never read as a finding, and never as health either: a pool that
    /// records no apprenticeship reports `with_lineage: 0`, and the lineage majority is
    /// measured against what was recorded rather than against the whole pool — or a
    /// compact could quiet the reading by simply not recording.
    #[test]
    fn an_unrecorded_lineage_is_unknown_rather_than_clean() {
        let (a_priv, _, aa) = fresh_id();
        let (_tp, _tk, elder) = fresh_id();
        let mut arc = InMemoryArchive::new();
        seat(&mut arc, &aa, &a_priv, 4, None, t(10));

        let pool = mediator_pool_concentration(&arc, Some(t(500)));
        assert_eq!(pool.total_active, 4);
        assert_eq!(pool.with_lineage, 0);
        assert!(pool.lineages.is_empty(), "no blank trainer group");
        assert!(!pool.lineage_majority_held(), "nothing recorded is not a finding");

        // Three of four recorded, all under one elder: the reading fires against the
        // recorded three, not against the pool of four.
        let (b_priv, _, bb) = fresh_id();
        seat(&mut arc, &bb, &b_priv, 3, Some(&elder), t(10));
        let pool = mediator_pool_concentration(&arc, Some(t(500)));
        assert_eq!((pool.total_active, pool.with_lineage), (7, 3));
        assert!(pool.lineage_majority_held(), "3 of the 3 recorded, though 3 of 7 seated");
    }

    /// The board respects superseding and term windows, like every other lens here.
    #[test]
    fn only_the_latest_registry_and_only_active_seats_count() {
        let (a_priv, _, aa) = fresh_id();
        let mut arc = InMemoryArchive::new();
        seat(&mut arc, &aa, &a_priv, 4, None, t(10));
        // A later registry from the same community supersedes it with two seats.
        seat(&mut arc, &aa, &a_priv, 2, None, t(100));
        assert_eq!(mediator_pool_concentration(&arc, Some(t(500))).total_active, 2);
        // And past the term end, nobody is serving.
        assert_eq!(
            mediator_pool_concentration(&arc, Some(t(100 + YEAR + 1))).total_active,
            0
        );
    }
}

// --- erosion remediation 5 and 6: attested nulls, a reporting cadence, and the
// --- empty office
//
// The measured table this slice was built from, each row an assertion. Rows 1 and 2
// are the K3 case: before `ReexaminationConclusion` existed, a panel that
// re-examined a designation at the mandated date and confirmed it read *identically*
// to a compact that had forgotten, forever. Rows 3 to 6 are the two function
// signals. See `doc/architecture/erosion-legibility.md`.

mod attested_nulls {
    use super::*;
    use crate::analytics::{
        erosion_alert, erosion_alert_with, overdue_reexaminations, silent_functions,
        unstaffed_functions, ErosionSeverity, ErosionThresholds,
    };
    use kc_conduct::{ActivityAttestation, AttestedFunction, ConductKind, ConductRecord, Severity};
    use kc_escalation::{
        Designation, DesignationTier, DesignationUrgency, ReexaminationConclusion,
        ReexaminationOutcome,
    };
    use kc_mediation::{MediatorEntry, MediatorRegistry};
    use kc_observer::{ObserverEntry, ObserverRegistry};

    const YEAR: i64 = 365 * 86_400;
    /// The default function cadence (one year).
    const CADENCE: i64 = YEAR;

    fn designate_due(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        signer: &PrivKey,
        due: Timestamp,
        at: Timestamp,
    ) -> Cid {
        let dz = Designation {
            subject_community: subject.clone(),
            tier: DesignationTier::Anathema,
            urgency: DesignationUrgency::Expedited,
            deliberators: vec![subject.clone()],
            criteria: "c".into(),
            cited_acts: vec![],
            failed_restoration_cycles: vec![],
            deliberation_cycles: vec![],
            reversal_conditions: Some("rev".into()),
            reexamine_after: Some(due),
            decision_class: kc_conduct::DecisionClass::SecondClass,
            designated_at: at,
        };
        let signed = Signed::build(dz, &[(subject.as_str(), signer)]).expect("sign");
        a.append(ArchiveEntry::DesignationDecided(signed)).expect("append")
    }

    fn conclude(
        a: &mut InMemoryArchive,
        designation_cid: Cid,
        who: &DidKey,
        signer: &PrivKey,
        outcome: ReexaminationOutcome,
        next: Option<Timestamp>,
        at: Timestamp,
    ) -> Cid {
        let rc = ReexaminationConclusion {
            designation_cid,
            outcome,
            examiners: vec![who.clone()],
            reasoning: "the pattern that named them has not changed".into(),
            next_reexamine_after: next,
            decision_class: kc_conduct::DecisionClass::SecondClass,
            concluded_at: at,
        };
        let signed = Signed::build(rc, &[(who.as_str(), signer)]).expect("sign");
        a.append(ArchiveEntry::ReexaminationConcluded(signed)).expect("append")
    }

    /// A conduct record about the designation: the nearest thing a diligent compact
    /// could do *before* the conclusion record existed. Row 2 of the table.
    fn note_about(
        a: &mut InMemoryArchive,
        designation_cid: Cid,
        who: &DidKey,
        signer: &PrivKey,
        at: Timestamp,
    ) {
        let rec = ConductRecord {
            subject_community: who.clone(),
            reporters: vec![who.clone()],
            reporter_perspective: None,
            kind: ConductKind::Concern,
            bearing: kc_conduct::ConductBearing::Other("designation re-examination".into()),
            severity: Severity::Minor,
            ladder_rung: None,
            decision_class: kc_conduct::DecisionClass::FirstClass,
            references: vec![designation_cid],
            narrative: "re-examined at the mandated date; confirmed".into(),
            observed_at: at,
            recorded_at: at,
        };
        let signed = Signed::build(rec, &[(who.as_str(), signer)]).expect("sign");
        a.append(ArchiveEntry::ConductAttested(signed)).expect("append");
    }

    fn seat_mediators(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        from: Timestamp,
        until: Option<Timestamp>,
    ) -> DidKey {
        let (_p, pubk, did) = fresh_id();
        let (_ep, epub, _ed) = fresh_id();
        let reg = MediatorRegistry {
            nominating_community: community.clone(),
            entries: vec![MediatorEntry {
                mediator_did: did.clone(),
                mediator_pubkey: pubk,
                encryption_pubkey: epub,
                valid_from: from,
                valid_until: until,
                trained_by: None,
            }],
            issued_at: from,
        };
        let signed = Signed::build(reg, &[(community.as_str(), community_priv)]).expect("sign");
        a.append(ArchiveEntry::MediatorRegistryUpdated(signed)).expect("append");
        did
    }

    fn seat_observers(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        from: Timestamp,
        until: Option<Timestamp>,
    ) -> DidKey {
        let (_p, pubk, did) = fresh_id();
        let reg = ObserverRegistry {
            nominating_community: community.clone(),
            entries: vec![ObserverEntry {
                observer_did: did.clone(),
                observer_pubkey: pubk,
                declared_perspective: "outsider".into(),
                valid_from: from,
                valid_until: until,
            }],
            issued_at: from,
        };
        let signed = Signed::build(reg, &[(community.as_str(), community_priv)]).expect("sign");
        a.append(ArchiveEntry::ObserverRegistryUpdated(signed)).expect("append");
        did
    }

    fn attest(
        a: &mut InMemoryArchive,
        community: &DidKey,
        community_priv: &PrivKey,
        function: AttestedFunction,
        window: (Timestamp, Timestamp),
        acts: Vec<Cid>,
        at: Timestamp,
    ) -> Cid {
        let att = ActivityAttestation {
            function,
            attesting_community: community.clone(),
            window_start: window.0,
            window_end: window.1,
            acts,
            statement: "no disputes were filed in this window".into(),
            attested_at: at,
        };
        let signed = Signed::build(att, &[(community.as_str(), community_priv)]).expect("sign");
        a.append(ArchiveEntry::ActivityAttested(signed)).expect("append")
    }

    // --- rows 1 and 2: diligence and neglect no longer read the same -----------

    #[test]
    fn a_confirmed_re_examination_answers_its_deadline_and_sets_the_next() {
        // Row 1. The panel looked at the mandated date and let the designation stand.
        // Before this record the reading was "overdue, forever"; the lens punished the
        // panel that did its job.
        let mut a = InMemoryArchive::new();
        let (sp, _, subj) = fresh_id();
        let cid = designate_due(&mut a, &subj, &sp, t(1000), t(100));
        conclude(
            &mut a,
            cid,
            &subj,
            &sp,
            ReexaminationOutcome::Confirmed,
            Some(t(1000 + 10 * YEAR)),
            t(1010),
        );

        // Answered: nothing is owed until the date the confirmation set.
        let al = erosion_alert(&a, Some(t(1500)));
        assert!(al.overdue_reexaminations.is_empty());
        assert_eq!(al.severity, ErosionSeverity::None);

        // And owed again once that date passes, now on its first renewal.
        let later = overdue_reexaminations(&a, Some(t(1000 + 10 * YEAR + 5)));
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].confirmations, 1);
        assert_eq!(later[0].reexamine_after, t(1000 + 10 * YEAR));
        assert_eq!(later[0].seconds_overdue, 5);
    }

    #[test]
    fn a_re_examination_nobody_held_still_reads_as_overdue() {
        // Row 2, and the pair that matters: the same designation, no conclusion
        // record, with a conduct record *about* it — the nearest thing a diligent
        // compact could do before this slice. It reads as what it is: unanswered.
        let mut a = InMemoryArchive::new();
        let (sp, _, subj) = fresh_id();
        let cid = designate_due(&mut a, &subj, &sp, t(1000), t(100));
        note_about(&mut a, cid, &subj, &sp, t(1010));

        let ignored = overdue_reexaminations(&a, Some(t(1500)));
        assert_eq!(ignored.len(), 1);
        assert_eq!(ignored[0].confirmations, 0);

        // The measured row: the two archives now differ, where they used to agree.
        let mut b = InMemoryArchive::new();
        let (bp, _, bsubj) = fresh_id();
        let bcid = designate_due(&mut b, &bsubj, &bp, t(1000), t(100));
        conclude(
            &mut b,
            bcid,
            &bsubj,
            &bp,
            ReexaminationOutcome::Confirmed,
            Some(t(1000 + 10 * YEAR)),
            t(1010),
        );
        assert!(overdue_reexaminations(&b, Some(t(1500))).is_empty());
        assert_ne!(
            overdue_reexaminations(&a, Some(t(1500))).len(),
            overdue_reexaminations(&b, Some(t(1500))).len()
        );
    }

    #[test]
    fn a_confirmation_is_not_a_discharge() {
        // Negative control, stated as a prediction first: one confirmation must not
        // silence the lens for good. The next date it sets is a real deadline, and
        // the renewal count keeps rising.
        let mut a = InMemoryArchive::new();
        let (sp, _, subj) = fresh_id();
        let cid = designate_due(&mut a, &subj, &sp, t(1000), t(100));
        conclude(&mut a, cid, &subj, &sp, ReexaminationOutcome::Confirmed, Some(t(5000)), t(1010));
        conclude(&mut a, cid, &subj, &sp, ReexaminationOutcome::Confirmed, Some(t(9000)), t(5010));

        assert!(overdue_reexaminations(&a, Some(t(8000))).is_empty());
        let owed = overdue_reexaminations(&a, Some(t(9500)));
        assert_eq!(owed.len(), 1);
        assert_eq!(owed[0].confirmations, 2, "both renewals counted");
    }

    // --- rows 3 to 5: the silent function -------------------------------------

    #[test]
    fn a_function_that_stops_acting_and_stops_attesting_is_reported() {
        // Row 3/4. A pool (or corps) that acted once and then went quiet for a
        // generation, with its seats still valid so this is silence and not vacancy.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));

        let silent = silent_functions(&a, Some(t(100 + 3 * YEAR)), CADENCE);
        assert_eq!(silent.len(), 1);
        assert_eq!(silent[0].community_did, comm.as_str());
        assert_eq!(silent[0].function, "Mediation");
        assert!(!silent[0].last_was_a_null, "there was never even a null");
        // Measured from the seat, so a function that has done nothing since the day
        // it was staffed reads as three years quiet rather than as brand new.
        assert_eq!(silent[0].seconds_since, 3 * YEAR);
    }

    #[test]
    fn an_attested_null_is_the_difference_between_quiet_and_stopped() {
        // Row 5, the one the whole record exists for. Two compacts, both with an
        // active pool, both with no disputes: one said so, one did not. Before
        // `ActivityAttestation` the two were the same silence.
        let mut quiet = InMemoryArchive::new();
        let (qp, _, q) = fresh_id();
        seat_mediators(&mut quiet, &q, &qp, t(100), Some(t(100 + 50 * YEAR)));
        attest(
            &mut quiet,
            &q,
            &qp,
            AttestedFunction::Mediation,
            (t(100), t(100 + 2 * YEAR)),
            vec![],
            t(100 + 2 * YEAR),
        );

        let mut stopped = InMemoryArchive::new();
        let (sp, _, s) = fresh_id();
        seat_mediators(&mut stopped, &s, &sp, t(100), Some(t(100 + 50 * YEAR)));

        let as_of = t(100 + 2 * YEAR + 10);
        assert!(
            silent_functions(&quiet, Some(as_of), CADENCE).is_empty(),
            "an attested null inside the cadence is evidence, not silence"
        );
        let flagged = silent_functions(&stopped, Some(as_of), CADENCE);
        assert_eq!(flagged.len(), 1);
        assert!(!flagged[0].last_was_a_null);
    }

    #[test]
    fn a_null_stops_counting_once_it_is_itself_stale() {
        // The null buys one cadence, not immunity. A compact that attested once and
        // then stopped attesting is silent again, and the reading says the last thing
        // on the record was a null.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        attest(
            &mut a,
            &comm,
            &cp,
            AttestedFunction::Mediation,
            (t(100), t(100 + YEAR)),
            vec![],
            t(100 + YEAR),
        );

        assert!(silent_functions(&a, Some(t(100 + YEAR + 10)), CADENCE).is_empty());
        let stale = silent_functions(&a, Some(t(100 + 3 * YEAR)), CADENCE);
        assert_eq!(stale.len(), 1);
        assert!(stale[0].last_was_a_null, "it spoke once, then stopped");
    }

    #[test]
    fn silence_inside_the_cadence_is_not_reported() {
        // Negative control: a pool seated last month has not gone quiet.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        assert!(silent_functions(&a, Some(t(100 + CADENCE)), CADENCE).is_empty());
    }

    // --- row 6: the empty office ----------------------------------------------

    #[test]
    fn a_function_with_nobody_seated_is_reported() {
        // Row 6, the hole neither remediation item named. Measured before the fix: a
        // compact with zero active mediators and zero active observers for forty-three
        // years read clean on every instrument.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + YEAR)));
        seat_observers(&mut a, &comm, &cp, t(100), Some(t(100 + YEAR)));

        let as_of = t(100 + 43 * YEAR);
        let vacant = unstaffed_functions(&a, Some(as_of));
        assert_eq!(vacant.len(), 2);
        assert_eq!(vacant[0].vacant_since, t(100 + YEAR));
        assert_eq!(vacant[0].seconds_vacant, 42 * YEAR);
        let al = erosion_alert(&a, Some(as_of));
        assert_ne!(al.severity, ErosionSeverity::None);
        assert_eq!(al.unstaffed_functions.len(), 2);
    }

    #[test]
    fn a_function_never_staffed_is_not_abandoned() {
        // The negative control for row 6, and a real distinction: declining to open an
        // office is not abandoning one. A compact with no observer corps at all must
        // not read the same as one that let its corps lapse.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        let vacant = unstaffed_functions(&a, Some(t(100 + 3 * YEAR)));
        assert!(
            vacant.is_empty(),
            "mediation is staffed and observation was never opened"
        );
    }

    #[test]
    fn a_seat_still_running_is_not_vacant() {
        // Negative control: an active term is not a vacancy, and an open-ended one
        // (legacy entries carry `valid_until: None`) is never vacant either.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        seat_observers(&mut a, &comm, &cp, t(100), None);
        assert!(unstaffed_functions(&a, Some(t(100 + 3 * YEAR))).is_empty());
    }

    #[test]
    fn one_lapse_is_one_dimension_not_two() {
        // The grading concern this slice designed a guard for, and then measured away.
        // The prediction was that a lapsed roster would fire *both* readings, since an
        // unstaffed function cannot attest, and that counting each would reach Alert on
        // compounding alone. It does not: `silent_functions` reads only functions with an
        // active seat and `unstaffed_functions` only functions with none, so the two are
        // disjoint by construction and the filter written for it was a rule against an
        // impossible state. This test pins the invariant that makes the filter
        // unnecessary, which is the thing worth keeping.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + YEAR)));

        // Read shortly after the seat lapses, so the duration rule is not what is
        // being measured: one *fresh* lapse is one dimension.
        let as_of = t(100 + YEAR + 86_400);
        assert!(
            silent_functions(&a, Some(as_of), CADENCE).is_empty(),
            "a vacant function is not read as a quiet one"
        );
        let al = erosion_alert(&a, Some(as_of));
        assert_eq!(al.unstaffed_functions.len(), 1);
        assert!(al.silent_functions.is_empty());
        assert_eq!(
            al.severity,
            ErosionSeverity::Notice,
            "one lapse is one dimension, not two"
        );

        // And a long-standing vacancy compounds on duration, exactly as a long
        // silence does: the stronger claim must not carry the weaker grade.
        let long_after = erosion_alert(&a, Some(t(100 + 10 * YEAR)));
        assert_eq!(long_after.unstaffed_functions.len(), 1);
        assert_eq!(long_after.severity, ErosionSeverity::Alert);
    }

    #[test]
    fn the_function_cadence_is_a_parameter() {
        // Like the rite cadence, and for the same reason: the calibration is an open
        // question, so it is tunable rather than a constant.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        let as_of = t(100 + 2 * YEAR);
        let loose = ErosionThresholds {
            attestation_cadence_seconds: 10 * YEAR,
            ..Default::default()
        };
        assert!(erosion_alert_with(&a, Some(as_of), &loose).silent_functions.is_empty());
        let tight = ErosionThresholds {
            attestation_cadence_seconds: YEAR,
            ..Default::default()
        };
        assert_eq!(
            erosion_alert_with(&a, Some(as_of), &tight).silent_functions.len(),
            1
        );
    }

    #[test]
    fn an_act_by_the_pool_is_not_silence() {
        // A mediation decision is an exercise of the function, credited to the
        // panelists' home community. The pool that is working does not need to attest
        // that it is working.
        let mut a = InMemoryArchive::new();
        let (cp, _, comm) = fresh_id();
        let med = seat_mediators(&mut a, &comm, &cp, t(100), Some(t(100 + 50 * YEAR)));
        let (pp, _, party_a) = fresh_id();
        let (_qp, _, party_b) = fresh_id();
        let dispute = kc_mediation::DisputeRecord {
            complainant: party_a.clone(),
            respondent: party_b.clone(),
            rung: kc_conduct::LadderRung::MediatedArbitration,
            summary: "s".into(),
            references: vec![],
            filed_at: t(100 + YEAR),
        };
        let signed_d = Signed::build(dispute, &[(party_a.as_str(), &pp)]).expect("sign");
        let dispute_cid = a.append(ArchiveEntry::DisputeFiled(signed_d)).expect("append");
        let dec = kc_mediation::MediationDecision {
            dispute_cid,
            panel: vec![med.clone()],
            outcome: kc_mediation::DecisionOutcome::Upheld,
            reasoning: "r".into(),
            sealed: None,
            arguments: vec![],
            decided_at: t(100 + 2 * YEAR),
        };
        let signed = Signed::build(dec, &[(med.as_str(), &cp)]).expect("sign");
        a.append(ArchiveEntry::MediationDecided(signed)).expect("append");

        // Read inside a cadence of the decision: the function has acted.
        assert!(silent_functions(&a, Some(t(100 + 2 * YEAR + 10)), CADENCE).is_empty());
        // And past it, the silence resumes from the act rather than from the seat.
        let later = silent_functions(&a, Some(t(100 + 5 * YEAR)), CADENCE);
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].seconds_since, 3 * YEAR);
    }
}
