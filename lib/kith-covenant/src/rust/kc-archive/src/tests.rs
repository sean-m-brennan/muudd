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

//! Integration tests for the archive: build a two-community ceremony from
//! the ground up, append every record, verify, roundtrip through JSON,
//! re-verify after reload.

use jiff::ToSpan;
use kc_ceremony::inter::{
    InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext, REQUIRED_FEATURES,
};
use kc_ceremony::intra::IntraCommunityRiteRunner;
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_conduct::{ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity};
use kc_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, Signed, Timestamp};
use kc_mediation::{
    DecisionOutcome, DisputeRecord, MediationDecision, MediatorEntry, MediatorRegistry,
};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};

use crate::authority::{
    authorize, check_authority, ObserverCaps, DEFAULT_PANEL_MIN, DEFAULT_QUORUM_THRESHOLD,
    DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS,
};
use crate::entry::ArchiveEntry;
use crate::store::{Archive, InMemoryArchive};
use crate::verify::verify_all;

struct Community {
    signed_identity: Signed<CommunityIdentity>,
    root_priv: PrivKey,
    speaker_did: DidKey,
    speaker_priv: PrivKey,
    envoy_did: DidKey,
    envoy_priv: PrivKey,
    witness_did: DidKey,
    witness_priv: PrivKey,
    speaker_delegation: Signed<SpeakerDelegation>,
    envoy_delegation: Signed<SpeakerDelegation>,
    witness_registry: Signed<WitnessRegistry>,
}

fn mk_community(name: &str, now: Timestamp) -> Community {
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
    let signed_identity =
        CommunityIdentity::new_self_signed(name.into(), &root_priv, root_pub, now).unwrap();
    let community_did = signed_identity.payload.did.clone();

    let (speaker_priv, speaker_pk) = keys::generate(Alg::Ed25519).unwrap();
    let speaker_did = DidKey::from_pubkey(&speaker_pk).unwrap();
    let speaker_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: speaker_did.clone(),
            speaker_pubkey: speaker_pk,
            scope: DelegationScope::Reciter,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let (envoy_priv, envoy_pk) = keys::generate(Alg::Ed25519).unwrap();
    let envoy_did = DidKey::from_pubkey(&envoy_pk).unwrap();
    let envoy_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: envoy_did.clone(),
            speaker_pubkey: envoy_pk.clone(),
            scope: DelegationScope::Envoy,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let (witness_priv, witness_pk) = keys::generate(Alg::Ed25519).unwrap();
    let witness_did = DidKey::from_pubkey(&witness_pk).unwrap();

    // Witness registry includes the internal witness AND the envoy (as CrossBoundary).
    let witness_registry = Signed::build(
        WitnessRegistry {
            community_did: community_did.clone(),
            entries: vec![
                WitnessEntry {
                    witness_did: witness_did.clone(),
                    witness_pubkey: witness_pk,
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
                WitnessEntry {
                    witness_did: envoy_did.clone(),
                    witness_pubkey: envoy_pk,
                    role: WitnessRole::CrossBoundary,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
            ],
            issued_at: now,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    Community {
        signed_identity,
        root_priv,
        speaker_did,
        speaker_priv,
        envoy_did,
        envoy_priv,
        witness_did,
        witness_priv,
        speaker_delegation,
        envoy_delegation,
        witness_registry,
    }
}

fn seed_archive(archive: &mut InMemoryArchive, c: &Community) {
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(
            c.signed_identity.clone(),
        ))
        .unwrap();
    archive
        .append(ArchiveEntry::SpeakerDelegated(c.speaker_delegation.clone()))
        .unwrap();
    archive
        .append(ArchiveEntry::SpeakerDelegated(c.envoy_delegation.clone()))
        .unwrap();
    archive
        .append(ArchiveEntry::WitnessRegistryUpdated(c.witness_registry.clone()))
        .unwrap();
}

fn run_intra_rite(c: &Community, now: Timestamp) -> Signed<kc_ceremony::intra::IntraCommunityRiteRecord> {
    let delegations = vec![c.speaker_delegation.clone()];
    let runner = IntraCommunityRiteRunner::new(&c.signed_identity, &delegations, &c.witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &c.speaker_did,
        &c.speaker_priv,
        Recitation {
            community_did: c.signed_identity.payload.did.clone(),
            speaker_did: c.speaker_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: "substrate".into(),
            movement_fallenness: "fallenness".into(),
            movement_recognition: "recognition".into(),
            movement_common_virtue: "common virtue".into(),
            movement_non_ultimacy: "non-ultimacy".into(),
            language_tag: "en".into(),
            translation_en: None,
            recited_at: now,
        },
    )
    .unwrap();
    let attestation =
        IntraCommunityRiteRunner::attest(&c.witness_did, &c.witness_priv, recitation.payload_cid, now)
            .unwrap();
    runner
        .seal(
            recitation,
            vec![attestation],
            &c.root_priv,
            RecitationVersion::current(),
            now,
        )
        .unwrap()
}

#[test]
fn single_community_identity_verifies() {
    let now = time::now();
    let c = mk_community("Alpha", now);
    let mut archive = InMemoryArchive::new();
    let cid = archive
        .append(ArchiveEntry::CommunityIdentityPublished(c.signed_identity.clone()))
        .unwrap();
    archive.verify(&cid).expect("community identity must verify");
}

#[test]
fn delegation_verifies_against_community() {
    let now = time::now();
    let c = mk_community("Alpha", now);
    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &c);
    verify_all(&archive).expect("seeded archive must verify end-to-end");
}

#[test]
fn delegation_without_prior_community_fails_to_verify() {
    let c = mk_community("Alpha", time::now());
    let mut archive = InMemoryArchive::new();
    let cid = archive
        .append(ArchiveEntry::SpeakerDelegated(c.speaker_delegation.clone()))
        .unwrap();
    // Community identity was never appended — resolver can't find the root pubkey.
    assert!(matches!(
        archive.verify(&cid),
        Err(KcError::Key(_))
    ));
}

#[test]
fn tampered_delegation_fails_to_verify() {
    let now = time::now();
    let c = mk_community("Alpha", now);
    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &c);

    // Tamper the delegation's signature byte before a fresh append.
    let mut evil = c.speaker_delegation.clone();
    evil.signatures[0].signature[0] ^= 0x01;
    // Its CID differs from the already-appended legit one, so append succeeds.
    let cid = archive.append(ArchiveEntry::SpeakerDelegated(evil)).unwrap();
    assert!(matches!(archive.verify(&cid), Err(KcError::SignatureInvalid)));
}

#[test]
fn full_two_community_ceremony_verifies_and_roundtrips() {
    let now = time::now();
    let alpha = mk_community("Alpha", now);
    let beta = mk_community("Beta", now);

    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &alpha);
    seed_archive(&mut archive, &beta);

    let alpha_rite = run_intra_rite(&alpha, now);
    let beta_rite = run_intra_rite(&beta, now);
    archive
        .append(ArchiveEntry::IntraCommunityRite(alpha_rite.clone()))
        .unwrap();
    archive
        .append(ArchiveEntry::IntraCommunityRite(beta_rite.clone()))
        .unwrap();

    let participants = vec![
        ParticipantContext {
            community: &alpha.signed_identity,
            intra_record: &alpha_rite,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.signed_identity,
            intra_record: &beta_rite,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let inter_runner =
        InterCredentialingRunner::new("alpha-beta-compact".into(), participants);

    let mut claims = Vec::new();
    for (community_did, envoy_did, envoy_priv) in [
        (
            &alpha.signed_identity.payload.did,
            &alpha.envoy_did,
            &alpha.envoy_priv,
        ),
        (
            &beta.signed_identity.payload.did,
            &beta.envoy_did,
            &beta.envoy_priv,
        ),
    ] {
        for feature in REQUIRED_FEATURES {
            claims.push(
                InterCredentialingRunner::present_claim(
                    envoy_did,
                    envoy_priv,
                    community_did.clone(),
                    feature,
                    format!("{feature:?} evidenced."),
                    vec![],
                    now,
                )
                .unwrap(),
            );
        }
    }

    let mut acceptances = Vec::new();
    for claim in &claims {
        let (signer_did, signer_priv, accepter_did) =
            if claim.payload.community_did == alpha.signed_identity.payload.did {
                (
                    &beta.envoy_did,
                    &beta.envoy_priv,
                    beta.signed_identity.payload.did.clone(),
                )
            } else {
                (
                    &alpha.envoy_did,
                    &alpha.envoy_priv,
                    alpha.signed_identity.payload.did.clone(),
                )
            };
        acceptances.push(
            InterCredentialingRunner::accept(
                signer_did,
                signer_priv,
                accepter_did,
                claim.payload_cid,
                true,
                None,
                now,
            )
            .unwrap(),
        );
    }

    let outcome = inter_runner.seal(claims, acceptances, vec![], now).unwrap();
    let witness_record = match outcome {
        InterCredentialingOutcome::Sealed(s) => s,
        InterCredentialingOutcome::NonAdmission(_) => panic!("expected admission"),
    };
    archive
        .append(ArchiveEntry::InterCommunityWitness(witness_record))
        .unwrap();

    // Verify every record is crypto-valid against archive state.
    verify_all(&archive).expect("complete archive must verify end-to-end");
    // The real ceremony (envoys co-signing inter records, roots sealing their
    // own rites/delegations) must also pass authority enforcement.
    check_authority(
        &archive,
        DEFAULT_QUORUM_THRESHOLD,
        DEFAULT_PANEL_MIN,
        ObserverCaps::default(),
    )
    .expect("complete ceremony archive must be authorized");

    // Roundtrip through JSON and re-verify.
    let json = archive.dump_json_to_string().unwrap();
    assert!(json.contains("\"version\""));
    let reloaded = InMemoryArchive::load_json_from_str(&json).unwrap();
    assert_eq!(reloaded.len(), archive.len());
    verify_all(&reloaded).expect("reloaded archive must verify identically");
}

#[test]
fn duplicate_append_rejected() {
    let now = time::now();
    let c = mk_community("Alpha", now);
    let mut archive = InMemoryArchive::new();
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(c.signed_identity.clone()))
        .unwrap();
    let err = archive
        .append(ArchiveEntry::CommunityIdentityPublished(c.signed_identity))
        .unwrap_err();
    assert!(matches!(err, KcError::Archive(msg) if msg.contains("duplicate")));
}

#[test]
fn verify_missing_cid_errors() {
    let archive = InMemoryArchive::new();
    let bogus = kc_core::Cid::of_bytes(b"not in archive");
    assert!(matches!(
        archive.verify(&bogus),
        Err(KcError::Archive(_))
    ));
}

// Task 16: property-based tamper detection across signature bytes.
// For any byte position × any bit, flipping the bit in the signature must
// break self-signed verification.
mod tamper_props {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn flipping_any_signature_bit_breaks_self_signed(
            byte_idx: usize,
            bit in 0u8..8,
        ) {
            let now = time::now();
            let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
            let mut signed = CommunityIdentity::new_self_signed(
                "proptest".into(), &root_priv, root_pub, now,
            ).unwrap();
            let sig_len = signed.signatures[0].signature.len();
            let idx = byte_idx % sig_len;
            signed.signatures[0].signature[idx] ^= 1 << bit;
            prop_assert!(CommunityIdentity::verify_self_signed(&signed).is_err());
        }

        #[test]
        fn flipping_any_payload_cid_byte_breaks_verification(
            byte_idx: usize,
            bit in 0u8..8,
        ) {
            let now = time::now();
            let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
            let mut signed = CommunityIdentity::new_self_signed(
                "proptest".into(), &root_priv, root_pub, now,
            ).unwrap();
            let cid_bytes = *signed.payload_cid.as_bytes();
            let idx = byte_idx % cid_bytes.len();
            let mut mutated = cid_bytes;
            mutated[idx] ^= 1 << bit;
            // Reconstruct a Cid from the mutated bytes via hex roundtrip.
            signed.payload_cid = kc_core::Cid::from_hex(&hex::encode(mutated)).unwrap();
            prop_assert!(CommunityIdentity::verify_self_signed(&signed).is_err());
        }

        #[test]
        fn flipping_any_signature_entry_byte_breaks_verification(
            entry_idx in 0usize..1,
            byte_kind in 0u8..3,
            byte_offset: usize,
            bit in 0u8..8,
        ) {
            // Targeted byte-flip within the signature envelope's mutable surfaces:
            // - kind 0: flip bit inside signatures[entry_idx].signature
            // - kind 1: flip bit inside signatures[entry_idx].signer_did (as bytes)
            // - kind 2: flip bit in payload_cid hash bytes
            let now = time::now();
            let c = mk_community("Alpha", now);
            let mut signed = c.signed_identity.clone();
            match byte_kind {
                0 => {
                    let sig = &mut signed.signatures[entry_idx].signature;
                    let i = byte_offset % sig.len();
                    sig[i] ^= 1 << bit;
                }
                1 => {
                    let did = &mut signed.signatures[entry_idx].signer_did;
                    if did.is_empty() { return Ok(()); }
                    // Safe byte mutation: work on bytes, then pass through utf8 check.
                    let mut bytes = did.as_bytes().to_vec();
                    let i = byte_offset % bytes.len();
                    bytes[i] ^= 1 << bit;
                    // If the mutation broke UTF-8, just skip.
                    match String::from_utf8(bytes) {
                        Ok(s) => *did = s,
                        Err(_) => return Ok(()),
                    }
                }
                _ => {
                    let cid_bytes = *signed.payload_cid.as_bytes();
                    let i = byte_offset % cid_bytes.len();
                    let mut m = cid_bytes;
                    m[i] ^= 1 << bit;
                    signed.payload_cid = kc_core::Cid::from_hex(&hex::encode(m)).unwrap();
                }
            }
            prop_assert!(CommunityIdentity::verify_self_signed(&signed).is_err());
        }
    }
}

// --- Delegation revocation (append-only tombstone) --------------------------

/// Helper: a conduct record signed by `signer_did`/`signer_priv`, attributed to
/// `community`, at `at`.
fn delegate_signed_conduct(
    community: &DidKey,
    signer_did: &DidKey,
    signer_priv: &PrivKey,
    at: Timestamp,
    note: &str,
) -> Signed<ConductRecord> {
    Signed::build(
        ConductRecord {
            subject_community: community.clone(),
            reporters: vec![community.clone()],
            reporter_perspective: None,
            kind: ConductKind::Concern,
            bearing: ConductBearing::Charity,
            severity: Severity::Minor,
            ladder_rung: None,
            decision_class: DecisionClass::FirstClass,
            references: vec![],
            narrative: note.into(),
            observed_at: at,
            recorded_at: at,
        },
        &[(signer_did.as_str(), signer_priv)],
    )
    .unwrap()
}

fn grant(
    community: &DidKey,
    root_priv: &PrivKey,
    speaker_did: &DidKey,
    speaker_pub: &kc_core::PubKey,
    issued: Timestamp,
    revoked: Option<Timestamp>,
) -> Signed<SpeakerDelegation> {
    Signed::build(
        SpeakerDelegation {
            community_did: community.clone(),
            speaker_did: speaker_did.clone(),
            speaker_pubkey: speaker_pub.clone(),
            scope: DelegationScope::Reciter,
            issued_at: issued,
            expires_at: None,
            revoked_at: revoked,
        },
        &[(community.as_str(), root_priv)],
    )
    .unwrap()
}

#[test]
fn appended_revocation_supersedes_grant_in_verification() {
    let ts = |s: i64| Timestamp::from_second(s).unwrap();
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
    let identity =
        CommunityIdentity::new_self_signed("Comm".into(), &root_priv, root_pub, ts(10)).unwrap();
    let community = identity.payload.did.clone();
    let (deleg_priv, deleg_pub) = keys::generate(Alg::Ed25519).unwrap();
    let deleg_did = DidKey::from_pubkey(&deleg_pub).unwrap();

    let mut archive = InMemoryArchive::new();
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(identity))
        .unwrap();
    archive
        .append(ArchiveEntry::SpeakerDelegated(grant(
            &community, &root_priv, &deleg_did, &deleg_pub, ts(100), None,
        )))
        .unwrap();

    // Pre-revocation delegate-signed record verifies.
    let pre_cid = archive
        .append(ArchiveEntry::ConductAttested(delegate_signed_conduct(
            &community, &deleg_did, &deleg_priv, ts(150), "pre",
        )))
        .unwrap();
    assert!(archive.verify(&pre_cid).is_ok());

    // Append a revocation tombstone (later issued_at, revoked_at set).
    archive
        .append(ArchiveEntry::SpeakerDelegated(grant(
            &community, &root_priv, &deleg_did, &deleg_pub, ts(200), Some(ts(200)),
        )))
        .unwrap();

    // A delegate-signed record dated after revocation must fail.
    let post_cid = archive
        .append(ArchiveEntry::ConductAttested(delegate_signed_conduct(
            &community, &deleg_did, &deleg_priv, ts(250), "post",
        )))
        .unwrap();
    assert!(
        archive.verify(&post_cid).is_err(),
        "post-revocation delegate record must fail verification"
    );
    // Non-retroactive: the pre-revocation record still verifies.
    assert!(archive.verify(&pre_cid).is_ok());

    // A re-grant (later issued_at, not revoked) reactivates the key.
    archive
        .append(ArchiveEntry::SpeakerDelegated(grant(
            &community, &root_priv, &deleg_did, &deleg_pub, ts(300), None,
        )))
        .unwrap();
    let after_cid = archive
        .append(ArchiveEntry::ConductAttested(delegate_signed_conduct(
            &community, &deleg_did, &deleg_priv, ts(350), "after",
        )))
        .unwrap();
    assert!(
        archive.verify(&after_cid).is_ok(),
        "a re-grant after revocation must reactivate the key"
    );
}

// --- Authority enforcement (§5) --------------------------------------------

fn ts(s: i64) -> Timestamp {
    Timestamp::from_second(s).unwrap()
}

fn community(archive: &mut InMemoryArchive, name: &str, at: Timestamp) -> (DidKey, PrivKey) {
    let (priv_, pub_) = keys::generate(Alg::Ed25519).unwrap();
    let id = CommunityIdentity::new_self_signed(name.into(), &priv_, pub_, at).unwrap();
    let did = id.payload.did.clone();
    archive
        .append(ArchiveEntry::CommunityIdentityPublished(id))
        .unwrap();
    (did, priv_)
}

fn add_delegate(
    archive: &mut InMemoryArchive,
    community: &DidKey,
    root_priv: &PrivKey,
    scope: DelegationScope,
    issued: Timestamp,
    revoked: Option<Timestamp>,
) -> (DidKey, PrivKey) {
    let (dpriv, dpub) = keys::generate(Alg::Ed25519).unwrap();
    let ddid = DidKey::from_pubkey(&dpub).unwrap();
    let deleg = Signed::build(
        SpeakerDelegation {
            community_did: community.clone(),
            speaker_did: ddid.clone(),
            speaker_pubkey: dpub,
            scope,
            issued_at: issued,
            expires_at: None,
            revoked_at: revoked,
        },
        &[(community.as_str(), root_priv)],
    )
    .unwrap();
    archive.append(ArchiveEntry::SpeakerDelegated(deleg)).unwrap();
    (ddid, dpriv)
}

fn conduct(
    reporters: Vec<DidKey>,
    signers: &[(&DidKey, &PrivKey)],
    dc: DecisionClass,
    at: Timestamp,
) -> Signed<ConductRecord> {
    let pairs: Vec<(&str, &PrivKey)> =
        signers.iter().map(|(d, p)| (d.as_str(), *p)).collect();
    Signed::build(
        ConductRecord {
            subject_community: reporters[0].clone(),
            reporters,
            reporter_perspective: None,
            kind: ConductKind::Concern,
            bearing: ConductBearing::Charity,
            severity: Severity::Minor,
            ladder_rung: None,
            decision_class: dc,
            references: vec![],
            narrative: "x".into(),
            observed_at: at,
            recorded_at: at,
        },
        &pairs,
    )
    .unwrap()
}

#[test]
fn authority_root_and_correct_scope_delegate_pass() {
    let mut a = InMemoryArchive::new();
    let (x, x_priv) = community(&mut a, "X", ts(10));
    // Root signs conduct for itself.
    let c1 = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone()], &[(&x, &x_priv)], DecisionClass::FirstClass, ts(100),
        )))
        .unwrap();
    authorize(&a, a.get(&c1).unwrap(), 2, DEFAULT_PANEL_MIN).expect("root-for-self is authorized");
    // An Envoy delegate of X signs conduct for X.
    let (env, env_priv) = add_delegate(&mut a, &x, &x_priv, DelegationScope::Envoy, ts(50), None);
    let c2 = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone()], &[(&env, &env_priv)], DecisionClass::FirstClass, ts(100),
        )))
        .unwrap();
    authorize(&a, a.get(&c2).unwrap(), 2, DEFAULT_PANEL_MIN).expect("scoped delegate is authorized");
}

#[test]
fn authority_rejects_cross_community_delegate() {
    let mut a = InMemoryArchive::new();
    let (x, x_priv) = community(&mut a, "X", ts(10));
    let (y, _y_priv) = community(&mut a, "Y", ts(10));
    let (env, env_priv) = add_delegate(&mut a, &x, &x_priv, DelegationScope::Envoy, ts(50), None);
    // X's envoy signs a record attributed to Y.
    let c = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![y.clone()], &[(&env, &env_priv)], DecisionClass::FirstClass, ts(100),
        )))
        .unwrap();
    assert!(authorize(&a, a.get(&c).unwrap(), 2, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn authority_rejects_wrong_scope() {
    let mut a = InMemoryArchive::new();
    let (x, x_priv) = community(&mut a, "X", ts(10));
    // A Reciter delegate signs conduct, which allows only Envoy (or root).
    let (rec, rec_priv) = add_delegate(&mut a, &x, &x_priv, DelegationScope::Reciter, ts(50), None);
    let c = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone()], &[(&rec, &rec_priv)], DecisionClass::FirstClass, ts(100),
        )))
        .unwrap();
    assert!(authorize(&a, a.get(&c).unwrap(), 2, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn authority_rejects_revoked_delegate() {
    let mut a = InMemoryArchive::new();
    let (x, x_priv) = community(&mut a, "X", ts(10));
    let (env, env_priv) =
        add_delegate(&mut a, &x, &x_priv, DelegationScope::Envoy, ts(50), Some(ts(80)));
    // Record dated after revocation.
    let c = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone()], &[(&env, &env_priv)], DecisionClass::FirstClass, ts(100),
        )))
        .unwrap();
    assert!(authorize(&a, a.get(&c).unwrap(), 2, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn authority_second_class_requires_quorum() {
    let mut a = InMemoryArchive::new();
    let (x, x_priv) = community(&mut a, "X", ts(10));
    let (y, y_priv) = community(&mut a, "Y", ts(10));
    // Single community, second-class → fails the quorum.
    let c1 = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone()], &[(&x, &x_priv)], DecisionClass::SecondClass, ts(100),
        )))
        .unwrap();
    assert!(authorize(&a, a.get(&c1).unwrap(), 2, DEFAULT_PANEL_MIN).is_err());
    // Two distinct communities co-sign → passes.
    let c2 = a
        .append(ArchiveEntry::ConductAttested(conduct(
            vec![x.clone(), y.clone()],
            &[(&x, &x_priv), (&y, &y_priv)],
            DecisionClass::SecondClass,
            ts(100),
        )))
        .unwrap();
    authorize(&a, a.get(&c2).unwrap(), 2, DEFAULT_PANEL_MIN).expect("two-community quorum authorized");
}

// --- §4 mediation authority -------------------------------------------------

fn add_mediators(
    archive: &mut InMemoryArchive,
    community: &DidKey,
    root_priv: &PrivKey,
    n: usize,
    at: Timestamp,
) -> Vec<(DidKey, PrivKey)> {
    let mut entries = vec![];
    let mut out = vec![];
    for _ in 0..n {
        let (mpriv, mpub) = keys::generate(Alg::Ed25519).unwrap();
        let mdid = DidKey::from_pubkey(&mpub).unwrap();
        entries.push(MediatorEntry {
            mediator_did: mdid.clone(),
            mediator_pubkey: mpub,
            valid_from: at,
            valid_until: None,
        });
        out.push((mdid, mpriv));
    }
    let reg = Signed::build(
        MediatorRegistry { nominating_community: community.clone(), entries, issued_at: at },
        &[(community.as_str(), root_priv)],
    )
    .unwrap();
    archive
        .append(ArchiveEntry::MediatorRegistryUpdated(reg))
        .unwrap();
    out
}

fn file_dispute(
    archive: &mut InMemoryArchive,
    complainant: &DidKey,
    comp_priv: &PrivKey,
    respondent: &DidKey,
    at: Timestamp,
) -> Cid {
    let d = Signed::build(
        DisputeRecord {
            complainant: complainant.clone(),
            respondent: respondent.clone(),
            rung: LadderRung::MediatedArbitration,
            summary: "boundary".into(),
            references: vec![],
            filed_at: at,
        },
        &[(complainant.as_str(), comp_priv)],
    )
    .unwrap();
    archive.append(ArchiveEntry::DisputeFiled(d)).unwrap()
}

fn decide(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    at: Timestamp,
) -> Cid {
    let panel_dids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
    let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
    let dec = Signed::build(
        MediationDecision {
            dispute_cid,
            panel: panel_dids,
            outcome: DecisionOutcome::Settled,
            reasoning: "agreed".into(),
            decided_at: at,
        },
        &pairs,
    )
    .unwrap();
    archive.append(ArchiveEntry::MediationDecided(dec)).unwrap()
}

/// Complainant A, respondent B, and three non-party communities C/D/E each with
/// one mediator. Returns (a, a_priv, b, dispute_cid, [mC, mD, mE]).
fn mediation_fixture(
    a: &mut InMemoryArchive,
) -> (DidKey, PrivKey, DidKey, Cid, Vec<(DidKey, PrivKey)>) {
    let (ca, ca_priv) = community(a, "A", ts(10));
    let (cb, _cb_priv) = community(a, "B", ts(10));
    let (cc, cc_priv) = community(a, "C", ts(10));
    let (cd, cd_priv) = community(a, "D", ts(10));
    let (ce, ce_priv) = community(a, "E", ts(10));
    let mut pool = vec![];
    pool.extend(add_mediators(a, &cc, &cc_priv, 1, ts(10)));
    pool.extend(add_mediators(a, &cd, &cd_priv, 1, ts(10)));
    pool.extend(add_mediators(a, &ce, &ce_priv, 1, ts(10)));
    let dispute = file_dispute(a, &ca, &ca_priv, &cb, ts(100));
    (ca, ca_priv, cb, dispute, pool)
}

#[test]
fn mediation_dispute_and_valid_panel_are_authorized() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, dispute, pool) = mediation_fixture(&mut a);
    // The dispute record itself (complainant root-signed) is authorized.
    authorize(&a, a.get(&dispute).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("filed dispute is authorized");
    // A 3-mediator, all-non-party panel decides.
    let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
    let dec = decide(&mut a, dispute, &panel, ts(200));
    authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("valid non-party 3-panel is authorized");
}

#[test]
fn mediation_panel_below_minimum_is_rejected() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, dispute, pool) = mediation_fixture(&mut a);
    let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p)| (d, p)).collect();
    let dec = decide(&mut a, dispute, &panel, ts(200));
    assert!(authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn mediation_party_mediator_violates_non_overlap() {
    let mut a = InMemoryArchive::new();
    let (ca, ca_priv, _cb, dispute, pool) = mediation_fixture(&mut a);
    // The complainant A also nominates a mediator and seats it on the panel.
    let a_meds = add_mediators(&mut a, &ca, &ca_priv, 1, ts(10));
    let mut panel: Vec<(&DidKey, &PrivKey)> =
        pool.iter().take(2).map(|(d, p)| (d, p)).collect();
    panel.push((&a_meds[0].0, &a_meds[0].1));
    let dec = decide(&mut a, dispute, &panel, ts(200));
    assert!(authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn mediation_unregistered_panelist_is_rejected() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, dispute, pool) = mediation_fixture(&mut a);
    let (stray_did, stray_priv) = {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        (DidKey::from_pubkey(&pk).unwrap(), sk)
    };
    let mut panel: Vec<(&DidKey, &PrivKey)> =
        pool.iter().take(2).map(|(d, p)| (d, p)).collect();
    panel.push((&stray_did, &stray_priv));
    let dec = decide(&mut a, dispute, &panel, ts(200));
    assert!(authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN).is_err());
}

// --- §5 escalation and designation authority --------------------------------

mod escalation {
    use super::*;
    use crate::authority::DEFAULT_WAITING_PERIOD_SECONDS;
    use kc_escalation::{
        DeliberationCycle, Designation, DesignationTier, DesignationUrgency, LadderTransition,
        ReEntry, ReEntryWitnessAttestation, RestitutionCompletion, RestitutionObligation,
    };

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;

    fn file_dispute_at(
        a: &mut InMemoryArchive,
        comp: &DidKey,
        comp_priv: &PrivKey,
        resp: &DidKey,
        rung: LadderRung,
        at: Timestamp,
    ) -> Cid {
        let d = Signed::build(
            DisputeRecord {
                complainant: comp.clone(),
                respondent: resp.clone(),
                rung,
                summary: "s".into(),
                references: vec![],
                filed_at: at,
            },
            &[(comp.as_str(), comp_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::DisputeFiled(d)).unwrap()
    }

    fn transition(
        a: &mut InMemoryArchive,
        dispute: &Cid,
        from: LadderRung,
        to: LadderRung,
        inits: &[(&DidKey, &PrivKey)],
        dc: DecisionClass,
        at: Timestamp,
    ) -> Cid {
        let initiators = inits.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = inits.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let t = Signed::build(
            LadderTransition {
                dispute_cid: *dispute,
                from_rung: from,
                to_rung: to,
                initiators,
                decision_class: dc,
                reasoning: "x".into(),
                transitioned_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::LadderAdvanced(t)).unwrap()
    }

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    #[test]
    fn ladder_escalate_one_step_then_deescalate() {
        let mut a = InMemoryArchive::new();
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::MediatedArbitration, ts(100));
        // Escalate exactly one step (arbitration -> limited sanctions).
        let up = transition(&mut a, &d, LadderRung::MediatedArbitration, LadderRung::LimitedSanctions,
            &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
        ok(&a, &up);
        // De-escalate to any lower rung (reversibility).
        let down = transition(&mut a, &d, LadderRung::LimitedSanctions, LadderRung::RegisteredProtest,
            &[(&x, &xp)], DecisionClass::FirstClass, ts(300));
        ok(&a, &down);
    }

    #[test]
    fn ladder_rejects_skipping_a_rung() {
        let mut a = InMemoryArchive::new();
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::DisputeRegistered, ts(100));
        // 0 -> 2 skips a rung.
        let skip = transition(&mut a, &d, LadderRung::DisputeRegistered, LadderRung::ReciprocalSuspension,
            &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
        bad(&a, &skip);
    }

    #[test]
    fn ladder_rejects_wrong_from_rung() {
        let mut a = InMemoryArchive::new();
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::MediatedArbitration, ts(100));
        // The dispute is at MediatedArbitration; a transition claiming to leave
        // RegisteredProtest does not match the current rung.
        let wrong = transition(&mut a, &d, LadderRung::RegisteredProtest, LadderRung::LimitedSanctions,
            &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
        bad(&a, &wrong);
    }

    #[test]
    fn ladder_rejects_substantive_reengagement_as_a_dispute_rung() {
        let mut a = InMemoryArchive::new();
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::MediatedArbitration, ts(100));
        let bad_rung = transition(&mut a, &d, LadderRung::MediatedArbitration,
            LadderRung::SubstantiveReEngagement, &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
        bad(&a, &bad_rung);
    }

    #[test]
    fn ladder_second_class_rung_needs_quorum_and_second_class_label() {
        // Unilateral, first-class label -> rejected. (Own archive so the invalid
        // transition does not pollute the dispute's derived current rung.)
        {
            let mut a = InMemoryArchive::new();
            let (x, xp) = community(&mut a, "X", ts(10));
            let (y, _yp) = community(&mut a, "Y", ts(10));
            let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::LimitedSanctions, ts(100));
            let uni = transition(&mut a, &d, LadderRung::LimitedSanctions,
                LadderRung::CollectiveSanctions, &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
            bad(&a, &uni);
        }
        // Two distinct communities co-sign, labeled second-class -> authorized.
        {
            let mut a = InMemoryArchive::new();
            let (x, xp) = community(&mut a, "X", ts(10));
            let (y, yp) = community(&mut a, "Y", ts(10));
            let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::LimitedSanctions, ts(100));
            let quorum = transition(&mut a, &d, LadderRung::LimitedSanctions,
                LadderRung::CollectiveSanctions, &[(&x, &xp), (&y, &yp)],
                DecisionClass::SecondClass, ts(200));
            ok(&a, &quorum);
        }
    }

    #[test]
    fn ladder_rejects_transition_on_closed_dispute() {
        let mut a = InMemoryArchive::new();
        let (x, xp, y, dispute, _pool) = {
            let (x, xp) = community(&mut a, "X", ts(10));
            let (y, _yp) = community(&mut a, "Y", ts(10));
            let d = file_dispute_at(&mut a, &x, &xp, &y, LadderRung::MediatedArbitration, ts(100));
            (x, xp, y, d, ())
        };
        let _ = y;
        // A settled mediation decision closes the dispute.
        let (c, cp) = community(&mut a, "C", ts(10));
        let (dd, ddp) = community(&mut a, "D", ts(10));
        let (e, ep) = community(&mut a, "E", ts(10));
        let mc = add_mediators(&mut a, &c, &cp, 1, ts(10));
        let md = add_mediators(&mut a, &dd, &ddp, 1, ts(10));
        let me = add_mediators(&mut a, &e, &ep, 1, ts(10));
        let panel = [(&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1), (&me[0].0, &me[0].1)];
        let _dec = decide(&mut a, dispute, &panel, ts(150));
        let after = transition(&mut a, &dispute, LadderRung::MediatedArbitration,
            LadderRung::LimitedSanctions, &[(&x, &xp)], DecisionClass::FirstClass, ts(200));
        bad(&a, &after);
    }

    // ---- designation four-condition test ----

    /// Append a satisfied-then-re-violated restitution cycle for `subject`; return
    /// the completion CID a designation cites in `failed_restoration_cycles`.
    fn bad_faith_cycle(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        imposer: &DidKey,
        imposer_priv: &PrivKey,
        base: i64,
    ) -> Cid {
        let ob = Signed::build(
            RestitutionObligation {
                subject_community: subject.clone(),
                beneficiary: imposer.clone(),
                dispute_cid: None,
                terms: "restore".into(),
                imposed_by: vec![imposer.clone()],
                due_by: None,
                decision_class: DecisionClass::FirstClass,
                imposed_at: ts(base),
            },
            &[(imposer.as_str(), imposer_priv)],
        )
        .unwrap();
        let ob_cid = a.append(ArchiveEntry::RestitutionObligated(ob)).unwrap();
        let done = Signed::build(
            RestitutionCompletion {
                obligation_cid: ob_cid,
                verifiers: vec![imposer.clone()],
                satisfied: true,
                narrative: "done".into(),
                verified_at: ts(base + 1),
            },
            &[(imposer.as_str(), imposer_priv)],
        )
        .unwrap();
        let done_cid = a.append(ArchiveEntry::RestitutionVerified(done)).unwrap();
        // Later re-violation: an adverse conduct record about the subject.
        a.append(ArchiveEntry::ConductAttested(conduct(
            vec![subject.clone()],
            &[(imposer, imposer_priv)],
            DecisionClass::FirstClass,
            ts(base + 2),
        )))
        .unwrap();
        done_cid
    }

    fn cycle(idx: u32, rep: &DidKey, opened: i64, concluded: i64) -> DeliberationCycle {
        DeliberationCycle {
            cycle_index: idx,
            representatives: vec![rep.clone()],
            opened_at: ts(opened),
            concluded_at: ts(concluded),
            reasoning: "r".into(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn designation(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        tier: DesignationTier,
        urgency: DesignationUrgency,
        deliberators: &[(&DidKey, &PrivKey)],
        cited: Vec<Cid>,
        cycles: Vec<DeliberationCycle>,
        criteria: &str,
        reversal: Option<String>,
        reexamine: Option<Timestamp>,
        at: Timestamp,
    ) -> Cid {
        let delibs = deliberators.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            deliberators.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let dz = Signed::build(
            Designation {
                subject_community: subject.clone(),
                tier,
                urgency,
                deliberators: delibs,
                criteria: criteria.into(),
                cited_acts: vec![Cid::of_bytes(b"act")],
                failed_restoration_cycles: cited,
                deliberation_cycles: cycles,
                reversal_conditions: reversal,
                reexamine_after: reexamine,
                decision_class: DecisionClass::SecondClass,
                designated_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::DesignationDecided(dz)).unwrap()
    }

    const W: i64 = DEFAULT_WAITING_PERIOD_SECONDS;

    /// Subject S, two deliberating communities P/Q, and three bad-faith cycles.
    fn designation_fixture(
        a: &mut InMemoryArchive,
    ) -> (DidKey, (DidKey, PrivKey), (DidKey, PrivKey), Vec<Cid>) {
        let (s, _sp) = community(a, "S", ts(1));
        let (p, pp) = community(a, "P", ts(1));
        let (qc, qp) = community(a, "Q", ts(1));
        let cycles = vec![
            bad_faith_cycle(a, &s, &p, &pp, 100),
            bad_faith_cycle(a, &s, &p, &pp, 200),
            bad_faith_cycle(a, &s, &p, &pp, 300),
        ];
        (s, (p, pp), (qc, qp), cycles)
    }

    #[test]
    fn designation_normal_valid_passes_all_four_conditions() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), cited) = designation_fixture(&mut a);
        let dz = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited,
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)],
            "three graver failed restorations", Some("changed conduct over probation".into()),
            Some(ts(99_999)), ts(2000));
        ok(&a, &dz);
    }

    #[test]
    fn designation_condition1_needs_three_cycles() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), mut cited) = designation_fixture(&mut a);
        cited.truncate(2); // only two cycles cited
        let dz = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited,
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)],
            "criteria", Some("rev".into()), None, ts(2000));
        bad(&a, &dz);
    }

    #[test]
    fn designation_condition2_needs_two_separated_cycles() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), cited) = designation_fixture(&mut a);
        // Only one cycle.
        let one = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited.clone(), vec![cycle(0, &p, 1000, 1100)],
            "criteria", Some("rev".into()), None, ts(2000));
        bad(&a, &one);
        // Two cycles, but too close together.
        let close = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited, vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1150, 1200)],
            "criteria", Some("rev".into()), None, ts(2000));
        bad(&a, &close);
    }

    #[test]
    fn designation_condition3_needs_criteria() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), cited) = designation_fixture(&mut a);
        let empty = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited, vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)],
            "   ", Some("rev".into()), None, ts(2000));
        bad(&a, &empty);
    }

    #[test]
    fn designation_condition4_reversal_rules_by_tier() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), cited) = designation_fixture(&mut a);
        // Anathema without reversal conditions -> rejected.
        let no_rev = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited.clone(),
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)],
            "criteria", None, None, ts(2000));
        bad(&a, &no_rev);
        // Hostis humani generis WITH a reversal path -> rejected.
        let hostis_rev = designation(&mut a, &s, DesignationTier::HostisHumaniGeneris,
            DesignationUrgency::Normal, &[(&p, &pp), (&qc, &qp)], cited,
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)],
            "criteria", Some("rev".into()), None, ts(2000));
        bad(&a, &hostis_rev);
    }

    #[test]
    fn designation_expedited_relaxes_cycles_but_needs_higher_quorum_and_reexamine() {
        let mut a = InMemoryArchive::new();
        let (s, _sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (qc, qp) = community(&mut a, "Q", ts(1));
        let (r, rp) = community(&mut a, "R", ts(1));
        // Expedited Anathema: 1 cycle, no restitution cycles cited, 3 deliberators
        // (> quorum_threshold), mandatory re-examination -> authorized.
        let ok_exp = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Expedited,
            &[(&p, &pp), (&qc, &qp), (&r, &rp)], vec![], vec![cycle(0, &p, 1000, 1100)],
            "acute harm", Some("rev".into()), Some(ts(5000)), ts(2000));
        ok(&a, &ok_exp);
        // Expedited but only two deliberators -> below the higher quorum.
        let low_q = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Expedited,
            &[(&p, &pp), (&qc, &qp)], vec![], vec![cycle(0, &p, 1000, 1100)],
            "acute harm", Some("rev".into()), Some(ts(5000)), ts(2000));
        bad(&a, &low_q);
        // The hostis tier may never be expedited.
        let hostis_exp = designation(&mut a, &s, DesignationTier::HostisHumaniGeneris,
            DesignationUrgency::Expedited, &[(&p, &pp), (&qc, &qp), (&r, &rp)], vec![],
            vec![cycle(0, &p, 1000, 1100)], "acute harm", None, Some(ts(5000)), ts(2000));
        bad(&a, &hostis_exp);
    }

    // ---- restitution verifier panel ----

    fn obligation(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        beneficiary: &DidKey,
        imposer: &DidKey,
        imposer_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let ob = Signed::build(
            RestitutionObligation {
                subject_community: subject.clone(),
                beneficiary: beneficiary.clone(),
                dispute_cid: None,
                terms: "t".into(),
                imposed_by: vec![imposer.clone()],
                due_by: None,
                decision_class: DecisionClass::FirstClass,
                imposed_at: at,
            },
            &[(imposer.as_str(), imposer_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::RestitutionObligated(ob)).unwrap()
    }

    fn completion(
        a: &mut InMemoryArchive,
        obligation_cid: Cid,
        panel: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let verifiers = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let done = Signed::build(
            RestitutionCompletion {
                obligation_cid,
                verifiers,
                satisfied: true,
                narrative: "ok".into(),
                verified_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::RestitutionVerified(done)).unwrap()
    }

    #[test]
    fn restitution_completion_valid_non_party_panel_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, _sp) = community(&mut a, "S", ts(1));
        let (b, bp) = community(&mut a, "B", ts(1));
        let (c, cp) = community(&mut a, "C", ts(1));
        let (dd, ddp) = community(&mut a, "D", ts(1));
        let (e, ep) = community(&mut a, "E", ts(1));
        let mc = add_mediators(&mut a, &c, &cp, 1, ts(1));
        let md = add_mediators(&mut a, &dd, &ddp, 1, ts(1));
        let me = add_mediators(&mut a, &e, &ep, 1, ts(1));
        let ob = obligation(&mut a, &s, &b, &b, &bp, ts(100));
        let panel = [(&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1), (&me[0].0, &me[0].1)];
        let done = completion(&mut a, ob, &panel, ts(200));
        ok(&a, &done);
    }

    #[test]
    fn restitution_completion_party_verifier_and_small_panel_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (b, bp) = community(&mut a, "B", ts(1));
        let (c, cp) = community(&mut a, "C", ts(1));
        let (dd, ddp) = community(&mut a, "D", ts(1));
        let mc = add_mediators(&mut a, &c, &cp, 1, ts(1));
        let md = add_mediators(&mut a, &dd, &ddp, 1, ts(1));
        // The subject nominates a mediator and slips it onto the panel.
        let sm = add_mediators(&mut a, &s, &sp, 1, ts(1));
        let ob = obligation(&mut a, &s, &b, &b, &bp, ts(100));
        let party_panel = [(&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1), (&sm[0].0, &sm[0].1)];
        let done = completion(&mut a, ob, &party_panel, ts(200));
        bad(&a, &done); // party-community verifier
        // Only two verifiers -> below panel_min.
        let small = completion(&mut a, ob, &[(&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1)], ts(200));
        bad(&a, &small);
    }

    // ---- re-entry ----

    fn recitation_for(did: &DidKey, priv_: &PrivKey, complete: bool) -> Signed<Recitation> {
        let blank = |s: &str| if complete { s.to_string() } else { String::new() };
        Signed::build(
            Recitation {
                community_did: did.clone(),
                speaker_did: did.clone(),
                version: RecitationVersion::current(),
                movement_substrate: blank("a"),
                movement_fallenness: "b".into(),
                movement_recognition: "c".into(),
                movement_common_virtue: "d".into(),
                movement_non_ultimacy: "e".into(),
                language_tag: "en".into(),
                translation_en: None,
                recited_at: ts(3000),
            },
            &[(did.as_str(), priv_)],
        )
        .unwrap()
    }

    fn witness_att(w: &DidKey, wp: &PrivKey, affirmed: bool) -> Signed<ReEntryWitnessAttestation> {
        Signed::build(
            ReEntryWitnessAttestation {
                witness_did: w.clone(),
                recitation_cid: Cid::of_bytes(b"rec"),
                sincerity_affirmed: affirmed,
                attested_at: ts(3001),
            },
            &[(w.as_str(), wp)],
        )
        .unwrap()
    }

    /// A minimal anathema designation (structurally trusted here; the re-entry
    /// test authorizes the re-entry, not the designation).
    fn min_designation(a: &mut InMemoryArchive, subject: &DidKey, tier: DesignationTier) -> Cid {
        // Signed by a throwaway key: append does not verify, and the re-entry
        // tests authorize the re-entry, not this designation.
        let (throwaway, _) = keys::generate(Alg::Ed25519).unwrap();
        let dz = Signed::build(
            Designation {
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
                designated_at: ts(500),
            },
            &[(subject.as_str(), &throwaway)],
        )
        .unwrap();
        a.append(ArchiveEntry::DesignationDecided(dz)).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn reentry(
        a: &mut InMemoryArchive,
        designation_cid: Cid,
        petitioner: &DidKey,
        pet_priv: &PrivKey,
        recitation: Signed<Recitation>,
        witnesses: Vec<Signed<ReEntryWitnessAttestation>>,
        deliberators: &[(&DidKey, &PrivKey)],
        acknowledge: bool,
    ) -> Cid {
        let delibs = deliberators.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            deliberators.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let _ = (petitioner, pet_priv);
        let r = Signed::build(
            ReEntry {
                designation_cid,
                petitioner: petitioner.clone(),
                recitation,
                conduct_accounting: "we accept what we did".into(),
                public_acknowledgement: if acknowledge { "it was wrong".into() } else { String::new() },
                witnesses,
                probation_until: ts(90_000),
                deliberators: delibs,
                decision_class: DecisionClass::SecondClass,
                completed_at: ts(4000),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::ReEntryCompleted(r)).unwrap()
    }

    #[test]
    fn reentry_valid_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (w1, w1p) = community(&mut a, "W1", ts(1));
        let (w2, w2p) = community(&mut a, "W2", ts(1));
        let dz = min_designation(&mut a, &s, DesignationTier::Anathema);
        let rec = recitation_for(&s, &sp, true);
        let ws = vec![witness_att(&w1, &w1p, true), witness_att(&w2, &w2p, true)];
        // Petitioner co-signs with a second community to meet the second-class quorum.
        let re = reentry(&mut a, dz, &s, &sp, rec, ws, &[(&s, &sp), (&p, &pp)], true);
        ok(&a, &re);
    }

    #[test]
    fn reentry_against_hostis_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (w1, w1p) = community(&mut a, "W1", ts(1));
        let (w2, w2p) = community(&mut a, "W2", ts(1));
        let dz = min_designation(&mut a, &s, DesignationTier::HostisHumaniGeneris);
        let rec = recitation_for(&s, &sp, true);
        let ws = vec![witness_att(&w1, &w1p, true), witness_att(&w2, &w2p, true)];
        let re = reentry(&mut a, dz, &s, &sp, rec, ws, &[(&s, &sp), (&p, &pp)], true);
        bad(&a, &re);
    }

    #[test]
    fn reentry_missing_movement_or_too_few_witnesses_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (w1, w1p) = community(&mut a, "W1", ts(1));
        let (w2, w2p) = community(&mut a, "W2", ts(1));
        let dz = min_designation(&mut a, &s, DesignationTier::Anathema);
        // Incomplete recitation.
        let bad_rec = recitation_for(&s, &sp, false);
        let ws = vec![witness_att(&w1, &w1p, true), witness_att(&w2, &w2p, true)];
        let re1 = reentry(&mut a, dz, &s, &sp, bad_rec, ws, &[(&s, &sp), (&p, &pp)], true);
        bad(&a, &re1);
        // Only one sincerity-affirmed witness (the other did not affirm).
        let rec = recitation_for(&s, &sp, true);
        let ws2 = vec![witness_att(&w1, &w1p, true), witness_att(&w2, &w2p, false)];
        let re2 = reentry(&mut a, dz, &s, &sp, rec, ws2, &[(&s, &sp), (&p, &pp)], true);
        bad(&a, &re2);
    }
}

// --- §9 interim injunction authority ----------------------------------------

mod injunction {
    use super::*;
    use crate::authority::DEFAULT_WAITING_PERIOD_SECONDS;
    use kc_escalation::{
        Capability, DeliberationCycle, Designation, DesignationTier, DesignationUrgency,
        InjunctionChallenge, InjunctionConfirmation, InterimInjunction, RestitutionCompletion,
        RestitutionObligation,
    };

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const NINETY_DAYS: i64 = 90 * 24 * 3600;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    fn issue(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        dispute_cid: Option<Cid>,
        revoked: Vec<Capability>,
        panel: &[(&DidKey, &PrivKey)],
        issued: i64,
        expires: i64,
    ) -> Cid {
        let panel_dids = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let inj = Signed::build(
            InterimInjunction {
                subject_community: subject.clone(),
                dispute_cid,
                harm: "ongoing interference".into(),
                protective_measures: "suspend the named capabilities".into(),
                revoked_capabilities: revoked,
                panel: panel_dids,
                issued_at: ts(issued),
                expires_at: ts(expires),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::InjunctionIssued(inj)).unwrap()
    }

    fn confirm(
        a: &mut InMemoryArchive,
        injunction_cid: Cid,
        confirmers: &[(&DidKey, &PrivKey)],
        at: i64,
    ) -> Cid {
        let dids = confirmers.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            confirmers.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let c = Signed::build(
            InjunctionConfirmation { injunction_cid, confirmers: dids, confirmed_at: ts(at) },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::InjunctionConfirmed(c)).unwrap()
    }

    fn challenge(
        a: &mut InMemoryArchive,
        injunction_cid: Cid,
        panel: &[(&DidKey, &PrivKey)],
        overturned: bool,
        at: i64,
    ) -> Cid {
        let panel_dids = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let ch = Signed::build(
            InjunctionChallenge {
                injunction_cid,
                panel: panel_dids,
                overturned,
                reasoning: "the harm was not ongoing".into(),
                decided_at: ts(at),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::InjunctionChallenged(ch)).unwrap()
    }

    /// Subject S plus three non-party communities C/D/E each with one mediator.
    /// Returns (s, [mC, mD, mE]).
    fn injunction_fixture(a: &mut InMemoryArchive) -> (DidKey, Vec<(DidKey, PrivKey)>) {
        let (s, _sp) = community(a, "S", ts(1));
        let (c, cp) = community(a, "C", ts(1));
        let (d, dp) = community(a, "D", ts(1));
        let (e, ep) = community(a, "E", ts(1));
        let mut pool = vec![];
        pool.extend(add_mediators(a, &c, &cp, 1, ts(1)));
        pool.extend(add_mediators(a, &d, &dp, 1, ts(1)));
        pool.extend(add_mediators(a, &e, &ep, 1, ts(1)));
        (s, pool)
    }

    #[test]
    fn injunction_valid_emergency_panel_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100 + NINETY_DAYS);
        ok(&a, &inj);
    }

    #[test]
    fn injunction_below_panel_minimum_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        // Only two non-party mediators — below panel_min.
        let small: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p)| (d, p)).collect();
        let inj_small = issue(&mut a, &s, None, vec![Capability::Trade], &small, 100, 100 + NINETY_DAYS);
        bad(&a, &inj_small);
    }

    #[test]
    fn injunction_party_mediator_violates_non_overlap() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (c, cp) = community(&mut a, "C", ts(1));
        let (d, dp) = community(&mut a, "D", ts(1));
        let mc = add_mediators(&mut a, &c, &cp, 1, ts(1));
        let md = add_mediators(&mut a, &d, &dp, 1, ts(1));
        // The subject nominates a mediator and seats it on its own injunction panel.
        let sm = add_mediators(&mut a, &s, &sp, 1, ts(1));
        let panel = [(&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1), (&sm[0].0, &sm[0].1)];
        let inj = issue(&mut a, &s, None, vec![Capability::Navigation], &panel, 100, 100 + NINETY_DAYS);
        bad(&a, &inj);
    }

    #[test]
    fn injunction_zero_window_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100);
        bad(&a, &inj);
    }

    #[test]
    fn injunction_challenge_by_different_panel_ok_same_panel_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100 + NINETY_DAYS);
        ok(&a, &inj);
        // A challenge co-signed by an issuing panelist violates the different-panel rule.
        let (f, fp) = community(&mut a, "F", ts(1));
        let (g, gp) = community(&mut a, "G", ts(1));
        let mf = add_mediators(&mut a, &f, &fp, 1, ts(1));
        let mg = add_mediators(&mut a, &g, &gp, 1, ts(1));
        let same = [(&pool[0].0, &pool[0].1), (&mf[0].0, &mf[0].1), (&mg[0].0, &mg[0].1)];
        let ch_same = challenge(&mut a, inj, &same, true, 200);
        bad(&a, &ch_same);
        // A wholly disjoint, non-party panel is authorized.
        let (h, hp) = community(&mut a, "H", ts(1));
        let mh = add_mediators(&mut a, &h, &hp, 1, ts(1));
        let diff = [(&mf[0].0, &mf[0].1), (&mg[0].0, &mg[0].1), (&mh[0].0, &mh[0].1)];
        let ch_diff = challenge(&mut a, inj, &diff, true, 200);
        ok(&a, &ch_diff);
    }

    #[test]
    fn injunction_confirmation_needs_distinct_quorum() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100 + NINETY_DAYS);
        // Two distinct communities confirm (second-class quorum).
        let (m, mp) = community(&mut a, "M", ts(1));
        let (n, np) = community(&mut a, "N", ts(1));
        let good = confirm(&mut a, inj, &[(&m, &mp), (&n, &np)], 200);
        ok(&a, &good);
        // A single community cannot meet the quorum.
        let solo = confirm(&mut a, inj, &[(&m, &mp)], 200);
        bad(&a, &solo);
    }

    // ---- designation safeguard #1 (issuing panel barred from designation) ----

    fn bad_faith_cycle(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        imposer: &DidKey,
        imposer_priv: &PrivKey,
        base: i64,
    ) -> Cid {
        let ob = Signed::build(
            RestitutionObligation {
                subject_community: subject.clone(),
                beneficiary: imposer.clone(),
                dispute_cid: None,
                terms: "restore".into(),
                imposed_by: vec![imposer.clone()],
                due_by: None,
                decision_class: DecisionClass::FirstClass,
                imposed_at: ts(base),
            },
            &[(imposer.as_str(), imposer_priv)],
        )
        .unwrap();
        let ob_cid = a.append(ArchiveEntry::RestitutionObligated(ob)).unwrap();
        let done = Signed::build(
            RestitutionCompletion {
                obligation_cid: ob_cid,
                verifiers: vec![imposer.clone()],
                satisfied: true,
                narrative: "done".into(),
                verified_at: ts(base + 1),
            },
            &[(imposer.as_str(), imposer_priv)],
        )
        .unwrap();
        let done_cid = a.append(ArchiveEntry::RestitutionVerified(done)).unwrap();
        a.append(ArchiveEntry::ConductAttested(conduct(
            vec![subject.clone()],
            &[(imposer, imposer_priv)],
            DecisionClass::FirstClass,
            ts(base + 2),
        )))
        .unwrap();
        done_cid
    }

    fn cycle(idx: u32, rep: &DidKey, opened: i64, concluded: i64) -> DeliberationCycle {
        DeliberationCycle {
            cycle_index: idx,
            representatives: vec![rep.clone()],
            opened_at: ts(opened),
            concluded_at: ts(concluded),
            reasoning: "r".into(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn designation(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        deliberators: &[(&DidKey, &PrivKey)],
        cited: Vec<Cid>,
        cycles: Vec<DeliberationCycle>,
        at: Timestamp,
    ) -> Cid {
        let delibs = deliberators.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            deliberators.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let dz = Signed::build(
            Designation {
                subject_community: subject.clone(),
                tier: DesignationTier::Anathema,
                urgency: DesignationUrgency::Normal,
                deliberators: delibs,
                criteria: "three graver failed restorations".into(),
                cited_acts: vec![Cid::of_bytes(b"act")],
                failed_restoration_cycles: cited,
                deliberation_cycles: cycles,
                reversal_conditions: Some("changed conduct over probation".into()),
                reexamine_after: Some(ts(99_999)),
                decision_class: DecisionClass::SecondClass,
                designated_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::DesignationDecided(dz)).unwrap()
    }

    const W: i64 = DEFAULT_WAITING_PERIOD_SECONDS;

    #[test]
    fn designation_blocked_when_deliberator_sat_on_active_injunction_panel() {
        let mut a = InMemoryArchive::new();
        let (s, _sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (qc, qp) = community(&mut a, "Q", ts(1));
        // Two further communities supply the rest of the injunction panel.
        let (c, cp) = community(&mut a, "C", ts(1));
        let (d, dp) = community(&mut a, "D", ts(1));
        // P — a would-be designation deliberator — also nominates a mediator.
        let mp = add_mediators(&mut a, &p, &pp, 1, ts(1));
        let mc = add_mediators(&mut a, &c, &cp, 1, ts(1));
        let md = add_mediators(&mut a, &d, &dp, 1, ts(1));
        let cycles = vec![
            bad_faith_cycle(&mut a, &s, &p, &pp, 100),
            bad_faith_cycle(&mut a, &s, &p, &pp, 200),
            bad_faith_cycle(&mut a, &s, &p, &pp, 300),
        ];
        let panel = [(&mp[0].0, &mp[0].1), (&mc[0].0, &mc[0].1), (&md[0].0, &md[0].1)];
        // Active injunction against S, panel includes P's mediator.
        let _inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 400, 400 + NINETY_DAYS);
        // Designation deliberated by P (overlaps the issuing panel) and Q -> blocked.
        let dz = designation(&mut a, &s, &[(&p, &pp), (&qc, &qp)], cycles.clone(),
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)], ts(2000));
        bad(&a, &dz);

        // Once the injunction is overturned by a different panel before the
        // designation, P is no longer on an *active* panel and it passes.
        let (e, ep) = community(&mut a, "E", ts(1));
        let (f, fp) = community(&mut a, "F", ts(1));
        let (g, gp) = community(&mut a, "G", ts(1));
        let me = add_mediators(&mut a, &e, &ep, 1, ts(1));
        let mf = add_mediators(&mut a, &f, &fp, 1, ts(1));
        let mg = add_mediators(&mut a, &g, &gp, 1, ts(1));
        let diff = [(&me[0].0, &me[0].1), (&mf[0].0, &mf[0].1), (&mg[0].0, &mg[0].1)];
        let _ch = challenge(&mut a, _inj, &diff, true, 500);
        // ts(2001) so this designation differs from the blocked one above (distinct CID).
        let dz2 = designation(&mut a, &s, &[(&p, &pp), (&qc, &qp)], cycles,
            vec![cycle(0, &p, 1000, 1100), cycle(1, &p, 1100 + W, 1200 + W)], ts(2001));
        ok(&a, &dz2);
    }
}

// --- §7 commons governance authority ----------------------------------------

mod commons {
    use super::*;
    use kc_commons::{CommonsKind, CommonsRegistration, CommonsSanction, SanctionLevel, UsageRecord};

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    /// Register a commons whose appropriators are exactly `parties`, co-signed by
    /// all of them (so the second-class quorum is satisfied whenever len >= 2).
    fn register(
        a: &mut InMemoryArchive,
        kind: CommonsKind,
        parties: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let dids: Vec<DidKey> = parties.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = parties.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let reg = Signed::build(
            CommonsRegistration {
                kind,
                name: "shared resource".into(),
                description: "a compact-level commons".into(),
                appropriators: dids.clone(),
                provision_rule: "provision scales with draw".into(),
                registered_by: dids,
                registered_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::CommonsRegistered(reg)).unwrap()
    }

    fn usage(
        a: &mut InMemoryArchive,
        commons_cid: Cid,
        community: &DidKey,
        priv_: &PrivKey,
        consumed: u64,
        contributed: u64,
        at: Timestamp,
    ) -> Cid {
        let u = Signed::build(
            UsageRecord {
                commons_cid,
                community: community.clone(),
                consumed,
                contributed,
                period_start: at,
                period_end: at,
                recorded_at: at,
            },
            &[(community.as_str(), priv_)],
        )
        .unwrap();
        a.append(ArchiveEntry::UsageAccounted(u)).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn sanction(
        a: &mut InMemoryArchive,
        commons_cid: Cid,
        subject: &DidKey,
        from: SanctionLevel,
        to: SanctionLevel,
        imposers: &[(&DidKey, &PrivKey)],
        dc: DecisionClass,
        at: Timestamp,
    ) -> Cid {
        let ids: Vec<DidKey> = imposers.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = imposers.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let s = Signed::build(
            CommonsSanction {
                commons_cid,
                subject_community: subject.clone(),
                from_level: from,
                to_level: to,
                basis: "overdraw beyond the provision rule".into(),
                references: vec![],
                imposers: ids,
                decision_class: dc,
                decided_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::SanctionApplied(s)).unwrap()
    }

    /// Three appropriators A, B, C of one commons; B is the misbehaving subject.
    /// Returns (a, a_priv, b, c, c_priv, commons_cid).
    #[allow(clippy::type_complexity)]
    fn fixture(
        a: &mut InMemoryArchive,
    ) -> (DidKey, PrivKey, DidKey, DidKey, PrivKey, Cid) {
        let (ca, ca_priv) = community(a, "A", ts(10));
        let (cb, _cb_priv) = community(a, "B", ts(10));
        let (cc, cc_priv) = community(a, "C", ts(10));
        let commons = register(
            a,
            CommonsKind::Communication,
            &[(&ca, &ca_priv), (&cb, &_cb_priv), (&cc, &cc_priv)],
            ts(50),
        );
        (ca, ca_priv, cb, cc, cc_priv, commons)
    }

    #[test]
    fn register_usage_and_graduated_sanction_are_authorized() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cc, cc_priv, commons) = fixture(&mut a);
        ok(&a, &commons);
        // Graduated escalation on B, imposed by A and C (two distinct communities).
        let s1 = sanction(&mut a, commons, &cb, SanctionLevel::None, SanctionLevel::Monitoring,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(100));
        ok(&a, &s1);
        let s2 = sanction(&mut a, commons, &cb, SanctionLevel::Monitoring, SanctionLevel::Warning,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(200));
        ok(&a, &s2);
        let s3 = sanction(&mut a, commons, &cb, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[(&ca, &ca_priv), (&cc, &cc_priv)], DecisionClass::SecondClass, ts(300));
        ok(&a, &s3);
    }

    #[test]
    fn usage_by_appropriator_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        let (cb, cb_priv) = community(&mut a, "B", ts(10));
        let commons = register(&mut a, CommonsKind::Navigation,
            &[(&ca, &ca_priv), (&cb, &cb_priv)], ts(50));
        let u = usage(&mut a, commons, &cb, &cb_priv, 100, 40, ts(100));
        ok(&a, &u);
    }

    #[test]
    fn registration_below_quorum_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        // A single-community registration is second-class and fails the quorum.
        let solo = register(&mut a, CommonsKind::LifeSupport, &[(&ca, &ca_priv)], ts(50));
        bad(&a, &solo);
    }

    #[test]
    fn registrant_must_be_an_appropriator() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        let (cb, cb_priv) = community(&mut a, "B", ts(10));
        let (cc, cc_priv) = community(&mut a, "C", ts(10));
        // Registered_by = {A, C} but appropriators = {A, B}: C is not an appropriator.
        let reg = Signed::build(
            CommonsRegistration {
                kind: CommonsKind::Rescue,
                name: "rescue net".into(),
                description: "d".into(),
                appropriators: vec![ca.clone(), cb.clone()],
                provision_rule: "r".into(),
                registered_by: vec![ca.clone(), cc.clone()],
                registered_at: ts(50),
            },
            &[(ca.as_str(), &ca_priv), (cc.as_str(), &cc_priv)],
        )
        .unwrap();
        let _ = cb_priv;
        let cid = a.append(ArchiveEntry::CommonsRegistered(reg)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn usage_by_non_appropriator_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        let (cb, cb_priv) = community(&mut a, "B", ts(10));
        let (cx, cx_priv) = community(&mut a, "X", ts(10));
        let commons = register(&mut a, CommonsKind::Communication,
            &[(&ca, &ca_priv), (&cb, &cb_priv)], ts(50));
        // X is not an appropriator of the commons.
        let u = usage(&mut a, commons, &cx, &cx_priv, 10, 0, ts(100));
        bad(&a, &u);
    }

    #[test]
    fn usage_on_unregistered_commons_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        // A CID that does not resolve to a CommonsRegistered entry.
        let u = usage(&mut a, Cid::of_bytes(b"nope"), &ca, &ca_priv, 10, 0, ts(100));
        bad(&a, &u);
    }

    #[test]
    fn sanction_rejects_skipping_a_level() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cc, cc_priv, commons) = fixture(&mut a);
        // None -> Restriction skips Monitoring and Warning.
        let s = sanction(&mut a, commons, &cb, SanctionLevel::None, SanctionLevel::Restriction,
            &[(&ca, &ca_priv), (&cc, &cc_priv)], DecisionClass::SecondClass, ts(100));
        bad(&a, &s);
    }

    #[test]
    fn sanction_rejects_wrong_from_level() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, _cc, _cc_priv, commons) = fixture(&mut a);
        // The pair is at None, but this claims to leave Warning.
        let s = sanction(&mut a, commons, &cb, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(100));
        bad(&a, &s);
    }

    #[test]
    fn sanction_second_class_level_needs_quorum_and_label() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cc, cc_priv, commons) = fixture(&mut a);
        let s1 = sanction(&mut a, commons, &cb, SanctionLevel::None, SanctionLevel::Monitoring,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(100));
        ok(&a, &s1);
        let s2 = sanction(&mut a, commons, &cb, SanctionLevel::Monitoring, SanctionLevel::Warning,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(200));
        ok(&a, &s2);
        // Restriction by a single community, labeled second-class: fails the quorum.
        let solo = sanction(&mut a, commons, &cb, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[(&ca, &ca_priv)], DecisionClass::SecondClass, ts(300));
        bad(&a, &solo);
        // Restriction labeled first-class: fails the class cross-check.
        let mislabeled = sanction(&mut a, commons, &cb, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[(&ca, &ca_priv), (&cc, &cc_priv)], DecisionClass::FirstClass, ts(301));
        bad(&a, &mislabeled);
    }

    #[test]
    fn sanction_deescalation_is_first_class_and_legal() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cc, cc_priv, commons) = fixture(&mut a);
        let s1 = sanction(&mut a, commons, &cb, SanctionLevel::None, SanctionLevel::Monitoring,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(100));
        ok(&a, &s1);
        let s2 = sanction(&mut a, commons, &cb, SanctionLevel::Monitoring, SanctionLevel::Warning,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(200));
        ok(&a, &s2);
        let s3 = sanction(&mut a, commons, &cb, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[(&ca, &ca_priv), (&cc, &cc_priv)], DecisionClass::SecondClass, ts(300));
        ok(&a, &s3);
        // Restoration: Restriction -> None in a single first-class de-escalation.
        let s4 = sanction(&mut a, commons, &cb, SanctionLevel::Restriction, SanctionLevel::None,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(400));
        ok(&a, &s4);
    }

    #[test]
    fn sanction_on_non_appropriator_subject_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv) = community(&mut a, "A", ts(10));
        let (cb, cb_priv) = community(&mut a, "B", ts(10));
        let (cx, _cx_priv) = community(&mut a, "X", ts(10));
        let commons = register(&mut a, CommonsKind::Communication,
            &[(&ca, &ca_priv), (&cb, &cb_priv)], ts(50));
        // X is not an appropriator, so it cannot be sanctioned on this commons.
        let s = sanction(&mut a, commons, &cx, SanctionLevel::None, SanctionLevel::Monitoring,
            &[(&ca, &ca_priv)], DecisionClass::FirstClass, ts(100));
        bad(&a, &s);
    }
}

// --- §8 observer / jester authority -----------------------------------------

mod observer {
    use super::*;
    use kc_core::PubKey;
    use kc_observer::{
        ObserverEntry, ObserverRegistry, ObserverReport, ReportResponse, SourceReveal,
        WhistleblowerReport,
    };

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const DAY: i64 = 24 * 3600;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    /// Register `n` observers nominated by `community`, each on a bounded
    /// `[at, at + 100 days)` term (well within the default term limit), returned as
    /// (did, priv).
    fn add_observers(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        n: usize,
        at: Timestamp,
    ) -> Vec<(DidKey, PrivKey)> {
        add_observers_term(a, community, root_priv, n, at, ts(at.as_second() + 100 * DAY))
    }

    /// Like [`add_observers`] but with an explicit `valid_until`, for exercising
    /// the §8 term-limit and rotation rules.
    fn add_observers_term(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        n: usize,
        at: Timestamp,
        until: Timestamp,
    ) -> Vec<(DidKey, PrivKey)> {
        let mut entries = vec![];
        let mut out = vec![];
        for _ in 0..n {
            let (opriv, opub) = keys::generate(Alg::Ed25519).unwrap();
            let odid = DidKey::from_pubkey(&opub).unwrap();
            entries.push(ObserverEntry {
                observer_did: odid.clone(),
                observer_pubkey: opub,
                declared_perspective: "labor-rights".into(),
                valid_from: at,
                valid_until: Some(until),
            });
            out.push((odid, opriv));
        }
        let reg = Signed::build(
            ObserverRegistry { nominating_community: community.clone(), entries, issued_at: at },
            &[(community.as_str(), root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::ObserverRegistryUpdated(reg)).unwrap();
        out
    }

    /// Append a roster for `community` with explicit per-observer windows (reusing
    /// the given observer keys), for rotation-gap tests. Returns the registry CID.
    fn append_roster(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        observers: &[(DidKey, PubKey)],
        valid_from: Timestamp,
        valid_until: Option<Timestamp>,
        issued_at: Timestamp,
    ) -> Cid {
        let entries = observers
            .iter()
            .map(|(od, opk)| ObserverEntry {
                observer_did: od.clone(),
                observer_pubkey: opk.clone(),
                declared_perspective: "labor-rights".into(),
                valid_from,
                valid_until,
            })
            .collect();
        let reg = Signed::build(
            ObserverRegistry { nominating_community: community.clone(), entries, issued_at },
            &[(community.as_str(), root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::ObserverRegistryUpdated(reg)).unwrap()
    }

    fn report(
        a: &mut InMemoryArchive,
        subject: &DidKey,
        observer: &(DidKey, PrivKey),
        at: Timestamp,
    ) -> Cid {
        let rep = Signed::build(
            ObserverReport {
                subject_community: subject.clone(),
                observer_did: observer.0.clone(),
                declared_perspective: "labor-rights".into(),
                summary: "authoritarian internal hierarchy".into(),
                references: vec![],
                observed_at: at,
                reported_at: at,
            },
            &[(observer.0.as_str(), &observer.1)],
        )
        .unwrap();
        a.append(ArchiveEntry::ObserverReportPublished(rep)).unwrap()
    }

    fn respond(
        a: &mut InMemoryArchive,
        report_cid: Cid,
        responder: &DidKey,
        resp_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let r = Signed::build(
            ReportResponse {
                report_cid,
                responding_community: responder.clone(),
                response: "our council is by consent".into(),
                responded_at: at,
            },
            &[(responder.as_str(), resp_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::ReportResponsePublished(r)).unwrap()
    }

    fn whistleblow(
        a: &mut InMemoryArchive,
        accused: &DidKey,
        panel: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let ids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let wb = Signed::build(
            WhistleblowerReport {
                accused_community: accused.clone(),
                source_commitment: Cid::of_bytes(b"commit"),
                claim: "coerced labor".into(),
                references: vec![],
                reception_panel: ids,
                filed_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::WhistleblowerFiled(wb)).unwrap()
    }

    fn reveal(
        a: &mut InMemoryArchive,
        report_cid: Cid,
        panel: &[(&DidKey, &PrivKey)],
        cause: &str,
        at: Timestamp,
    ) -> Cid {
        let ids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let rev = Signed::build(
            SourceReveal {
                report_cid,
                panel: ids,
                cause: cause.into(),
                disclosed_source: "did:key:zMember".into(),
                revealed_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::SourceRevealed(rev)).unwrap()
    }

    /// Subject S, a reporting community R with one observer, plus a three-mediator
    /// non-party pool C/D/E. Returns (s, s_priv, obs, pool).
    #[allow(clippy::type_complexity)]
    fn fixture(
        a: &mut InMemoryArchive,
    ) -> (DidKey, PrivKey, (DidKey, PrivKey), Vec<(DidKey, PrivKey)>) {
        let (s, s_priv) = community(a, "S", ts(10));
        let (r, r_priv) = community(a, "R", ts(10));
        let obs = add_observers(a, &r, &r_priv, 1, ts(10)).pop().unwrap();
        let (cc, cc_priv) = community(a, "C", ts(10));
        let (cd, cd_priv) = community(a, "D", ts(10));
        let (ce, ce_priv) = community(a, "E", ts(10));
        let mut pool = vec![];
        pool.extend(add_mediators(a, &cc, &cc_priv, 1, ts(10)));
        pool.extend(add_mediators(a, &cd, &cd_priv, 1, ts(10)));
        pool.extend(add_mediators(a, &ce, &ce_priv, 1, ts(10)));
        (s, s_priv, obs, pool)
    }

    #[test]
    fn non_party_observer_report_and_subject_response_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, obs, _pool) = fixture(&mut a);
        let rep = report(&mut a, &s, &obs, ts(100));
        ok(&a, &rep);
        let resp = respond(&mut a, rep, &s, &s_priv, ts(200));
        ok(&a, &resp);
    }

    #[test]
    fn observer_reporting_on_own_community_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = add_observers(&mut a, &r, &r_priv, 1, ts(10)).pop().unwrap();
        // The observer's home community R is the subject: non-overlap violated.
        let rep = report(&mut a, &r, &obs, ts(100));
        bad(&a, &rep);
    }

    #[test]
    fn unregistered_observer_report_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv) = community(&mut a, "S", ts(10));
        let (od, opriv) = {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            (DidKey::from_pubkey(&pk).unwrap(), sk)
        };
        let rep = report(&mut a, &s, &(od, opriv), ts(100));
        bad(&a, &rep);
    }

    #[test]
    fn only_subject_may_respond_to_a_report() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, obs, _pool) = fixture(&mut a);
        let (other, other_priv) = community(&mut a, "Other", ts(10));
        let rep = report(&mut a, &s, &obs, ts(100));
        let resp = respond(&mut a, rep, &other, &other_priv, ts(200));
        bad(&a, &resp);
    }

    #[test]
    fn whistleblower_non_party_panel_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, _obs, pool) = fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let wb = whistleblow(&mut a, &s, &panel, ts(100));
        ok(&a, &wb);
    }

    #[test]
    fn whistleblower_panel_from_accused_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        // The accused S also nominates a mediator and seats it on the reception panel.
        let s_meds = add_mediators(&mut a, &s, &s_priv, 1, ts(10));
        let mut panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p)| (d, p)).collect();
        panel.push((&s_meds[0].0, &s_meds[0].1));
        let wb = whistleblow(&mut a, &s, &panel, ts(100));
        bad(&a, &wb);
    }

    #[test]
    fn whistleblower_panel_below_minimum_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, _obs, pool) = fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p)| (d, p)).collect();
        let wb = whistleblow(&mut a, &s, &panel, ts(100));
        bad(&a, &wb);
    }

    #[test]
    fn source_reveal_requires_cause_and_valid_panel() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, _obs, pool) = fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p)| (d, p)).collect();
        let wb = whistleblow(&mut a, &s, &panel, ts(100));
        // Empty cause: rejected.
        let no_cause = reveal(&mut a, wb, &panel, "", ts(200));
        bad(&a, &no_cause);
        // For cause, by a non-party panel: authorized.
        let good = reveal(&mut a, wb, &panel, "source recanted; claim contested", ts(300));
        ok(&a, &good);
    }

    // --- §8 observer-corps caps (pool-share + term + rotation) -------------

    /// Generate `n` fresh observer keypairs as (did, pubkey).
    fn observer_keys(n: usize) -> Vec<(DidKey, PubKey)> {
        (0..n)
            .map(|_| {
                let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
                (DidKey::from_pubkey(&pk).unwrap(), pk)
            })
            .collect()
    }

    #[test]
    fn open_ended_observer_term_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        // No valid_until: the §8 term limit refuses open-ended observer standing.
        let reg = append_roster(&mut a, &r, &r_priv, &obs, ts(10), None, ts(10));
        bad(&a, &reg);
    }

    #[test]
    fn overlong_observer_term_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        // A 400-day term exceeds the 365-day default maximum.
        let reg = append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(10 + 400 * DAY)), ts(10));
        bad(&a, &reg);
    }

    #[test]
    fn bounded_observer_term_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        let reg =
            append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(10 + 100 * DAY)), ts(10));
        ok(&a, &reg);
    }

    #[test]
    fn renomination_within_rotation_period_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        // First term ends at day 100.
        append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(100 * DAY)), ts(10));
        // Re-nominated only 30 days later — inside the 90-day rotation period.
        let reg2 =
            append_roster(&mut a, &r, &r_priv, &obs, ts(130 * DAY), Some(ts(200 * DAY)), ts(130 * DAY));
        bad(&a, &reg2);
    }

    #[test]
    fn renomination_after_rotation_period_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(100 * DAY)), ts(10));
        // Re-nominated 100 days after the prior term ended — clears the 90-day gap.
        let reg2 =
            append_roster(&mut a, &r, &r_priv, &obs, ts(200 * DAY), Some(ts(260 * DAY)), ts(200 * DAY));
        ok(&a, &reg2);
    }

    #[test]
    fn overlapping_renomination_exceeding_term_in_union_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        // First window: 350 days — under the 365-day per-window cap on its own.
        append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(350 * DAY)), ts(10));
        // A second window opened before the first ends (overlap) — the rotation-gap
        // rule never sees it, and each window alone is under the cap, but their
        // continuous-service union (day 0 → day 640) exceeds 365 days.
        let reg2 = append_roster(
            &mut a,
            &r,
            &r_priv,
            &obs,
            ts(300 * DAY),
            Some(ts(640 * DAY)),
            ts(300 * DAY),
        );
        bad(&a, &reg2);
    }

    #[test]
    fn overlapping_renomination_within_term_in_union_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (r, r_priv) = community(&mut a, "R", ts(10));
        let obs = observer_keys(1);
        // Two overlapping windows whose union (day 0 → day 200) stays under the cap:
        // contiguous extension is permitted, only over-the-limit chaining is refused.
        append_roster(&mut a, &r, &r_priv, &obs, ts(10), Some(ts(100 * DAY)), ts(10));
        let reg2 = append_roster(
            &mut a,
            &r,
            &r_priv,
            &obs,
            ts(50 * DAY),
            Some(ts(200 * DAY)),
            ts(50 * DAY),
        );
        ok(&a, &reg2);
    }

    #[test]
    fn single_community_majority_of_corps_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (aa, aa_priv) = community(&mut a, "A", ts(10));
        let (bb, bb_priv) = community(&mut a, "B", ts(10));
        // B seats 2; then A seats 3 → a corps of 5 in which A holds 3 (a majority).
        add_observers_term(&mut a, &bb, &bb_priv, 2, ts(10), ts(100 * DAY));
        let a_obs = observer_keys(3);
        let reg = append_roster(&mut a, &aa, &aa_priv, &a_obs, ts(10), Some(ts(100 * DAY)), ts(20));
        bad(&a, &reg);
    }

    #[test]
    fn small_corps_below_floor_tolerates_majority() {
        let mut a = InMemoryArchive::new();
        let (aa, aa_priv) = community(&mut a, "A", ts(10));
        let (bb, bb_priv) = community(&mut a, "B", ts(10));
        // Corps of 4 (< floor 5): A's 3/4 majority is tolerated (small compact).
        add_observers_term(&mut a, &bb, &bb_priv, 1, ts(10), ts(100 * DAY));
        let a_obs = observer_keys(3);
        let reg = append_roster(&mut a, &aa, &aa_priv, &a_obs, ts(10), Some(ts(100 * DAY)), ts(20));
        ok(&a, &reg);
    }
}

// --- §9 substrate-deference (constitution) authority ------------------------

mod constitution {
    use super::*;
    use kc_constitution::{
        AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
        FastChange, FastPeriod, IncapableMatter,
    };

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    fn matter(k: &str) -> IncapableMatter {
        IncapableMatter { key: k.into(), description: k.into() }
    }

    fn ratify(
        a: &mut InMemoryArchive,
        matters: &[&str],
        ratifiers: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let dids: Vec<DidKey> = ratifiers.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = ratifiers.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let g = Signed::build(
            ConstitutionGenesis {
                compact_incapable: matters.iter().map(|k| matter(k)).collect(),
                fast_periods: vec![FastPeriod { label: "annual".into(), start: ts(1000), end: ts(2000) , recurrence: None }],
                ratifiers: dids,
                genesis_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::ConstitutionRatified(g)).unwrap()
    }

    fn amend_incapable(
        a: &mut InMemoryArchive,
        kind: AmendmentKind,
        key: &str,
        deciders: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let dids: Vec<DidKey> = deciders.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = deciders.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let am = Signed::build(
            CompactIncapableAmendment {
                kind,
                matter: matter(key),
                rationale: "r".into(),
                deciders: dids,
                decision_class: DecisionClass::SecondClass,
                decided_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::CompactIncapableAmended(am)).unwrap()
    }

    fn amend_fast(
        a: &mut InMemoryArchive,
        change: FastChange,
        deciders: &[(&DidKey, &PrivKey)],
        at: Timestamp,
    ) -> Cid {
        let dids: Vec<DidKey> = deciders.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = deciders.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let am = Signed::build(
            FastCadenceAmendment {
                change,
                period: FastPeriod { label: "annual".into(), start: ts(1000), end: ts(1500) , recurrence: None },
                rationale: "r".into(),
                deciders: dids,
                decision_class: DecisionClass::SecondClass,
                decided_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::FastCadenceAmended(am)).unwrap()
    }

    /// Four communities A/B/C/D (enough for the contraction bar of quorum+1).
    fn communities(a: &mut InMemoryArchive) -> Vec<(DidKey, PrivKey)> {
        ["A", "B", "C", "D"].iter().map(|n| community(a, n, ts(10))).collect()
    }

    #[test]
    fn genesis_quorum_and_singleton() {
        // Quorum: a single ratifier fails (isolated archive, since an unauthorized
        // genesis still counts against the singleton check for any later one).
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        let solo = ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1)], ts(50));
        bad(&a, &solo);

        // Fresh archive: two ratifiers authorized; a second genesis is refused.
        let mut b = InMemoryArchive::new();
        let cb = communities(&mut b);
        let g = ratify(&mut b, &["internal-theology"],
            &[(&cb[0].0, &cb[0].1), (&cb[1].0, &cb[1].1)], ts(60));
        ok(&b, &g);
        let g2 = ratify(&mut b, &["x"], &[(&cb[0].0, &cb[0].1), (&cb[1].0, &cb[1].1)], ts(70));
        bad(&b, &g2);
    }

    #[test]
    fn add_matter_then_reject_duplicate() {
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(60));
        let add = amend_incapable(&mut a, AmendmentKind::Add, "aesthetics",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(100));
        ok(&a, &add);
        // Adding the genesis matter again (already in force) is rejected.
        let dup = amend_incapable(&mut a, AmendmentKind::Add, "internal-theology",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(110));
        bad(&a, &dup);
    }

    #[test]
    fn genesis_matter_is_immutable() {
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(60));
        // Even at the high bar, a genesis-original matter cannot be removed.
        let rm = amend_incapable(&mut a, AmendmentKind::Remove, "internal-theology",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1), (&c[2].0, &c[2].1)], ts(100));
        bad(&a, &rm);
    }

    #[test]
    fn remove_added_matter_needs_higher_bar() {
        // Normal quorum (2) fails the contraction bar. (Isolated: an unauthorized
        // Remove still shifts the walked list state for a later record.)
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(60));
        amend_incapable(&mut a, AmendmentKind::Add, "aesthetics",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(100));
        let rm2 = amend_incapable(&mut a, AmendmentKind::Remove, "aesthetics",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(200));
        bad(&a, &rm2);

        // Fresh archive: Remove at the higher bar (3 distinct) is authorized.
        let mut b = InMemoryArchive::new();
        let cb = communities(&mut b);
        ratify(&mut b, &["internal-theology"], &[(&cb[0].0, &cb[0].1), (&cb[1].0, &cb[1].1)], ts(60));
        amend_incapable(&mut b, AmendmentKind::Add, "aesthetics",
            &[(&cb[0].0, &cb[0].1), (&cb[1].0, &cb[1].1)], ts(100));
        let rm3 = amend_incapable(&mut b, AmendmentKind::Remove, "aesthetics",
            &[(&cb[0].0, &cb[0].1), (&cb[1].0, &cb[1].1), (&cb[2].0, &cb[2].1)], ts(300));
        ok(&b, &rm3);
    }

    #[test]
    fn remove_absent_and_amend_before_genesis_rejected() {
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        // Amendment before any genesis.
        let early = amend_incapable(&mut a, AmendmentKind::Add, "x",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(40));
        bad(&a, &early);
        ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(60));
        // Removing a matter that is not in force.
        let rm = amend_incapable(&mut a, AmendmentKind::Remove, "never-added",
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1), (&c[2].0, &c[2].1)], ts(100));
        bad(&a, &rm);
    }

    #[test]
    fn fast_contract_needs_higher_bar_extend_does_not() {
        let mut a = InMemoryArchive::new();
        let c = communities(&mut a);
        ratify(&mut a, &["internal-theology"], &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(60));
        // Extend at normal quorum (2): authorized.
        let ext = amend_fast(&mut a, FastChange::Extend, &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(100));
        ok(&a, &ext);
        // Contract at normal quorum (2): fails the higher bar.
        let con2 = amend_fast(&mut a, FastChange::Contract, &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1)], ts(200));
        bad(&a, &con2);
        // Contract at the higher bar (3): authorized.
        let con3 = amend_fast(&mut a, FastChange::Contract,
            &[(&c[0].0, &c[0].1), (&c[1].0, &c[1].1), (&c[2].0, &c[2].1)], ts(300));
        ok(&a, &con3);
    }
}

// --- Move 4 structured-skepticism hold (EventActionInvoked) -----------------

mod event_action {
    use super::*;
    use kc_provenance::{EventActionInvoked, EventCorroboration, EventReport};

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const HOLD: i64 = DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn bad(a: &InMemoryArchive, cid: &Cid) {
        assert!(authorize(a, a.get(cid).unwrap(), Q, P).is_err(), "expected rejection");
    }

    /// Append a signed event report by `originator`. `qualifying` toggles the
    /// high-interpretive-charge trigger; a non-qualifying report fires none.
    fn report_event(
        a: &mut InMemoryArchive,
        originator: &DidKey,
        root_priv: &PrivKey,
        qualifying: bool,
        reported_at: Timestamp,
    ) -> Cid {
        let rec = EventReport {
            originator: originator.clone(),
            summary: "an ambiguous transcendent event".into(),
            high_interpretive_charge: qualifying,
            sudden_onset: false,
            originator_holds_stake: false,
            aligned_faction: None,
            declared_incentives: "surface stability".into(),
            information_history: "first-hand".into(),
            disclosed_interests: vec![],
            onset_at: reported_at,
            reported_at,
        };
        let signed = Signed::build(rec, &[(originator.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::EventReported(signed)).unwrap()
    }

    fn corroborate(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        community: &DidKey,
        root_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let rec = EventCorroboration {
            event_cid: event_cid.clone(),
            corroborating_community: community.clone(),
            translation: None,
            corroborated_at: at,
        };
        let signed = Signed::build(rec, &[(community.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::EventCorroborated(signed)).unwrap()
    }

    fn invoke_action(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        acting: &DidKey,
        root_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let rec = EventActionInvoked {
            event_cid: event_cid.clone(),
            acting_community: acting.clone(),
            action: "escalate to the mediator pool".into(),
            invoked_at: at,
        };
        let signed = Signed::build(rec, &[(acting.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::EventActionInvoked(signed)).unwrap()
    }

    #[test]
    fn action_on_qualifying_event_within_hold_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let ev = report_event(&mut a, &r, &rp, true, ts(1000));
        // Acted on one hour later — inside the hold, no independent corroboration.
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1000 + 3600));
        bad(&a, &act);
    }

    #[test]
    fn action_after_hold_elapses_is_authorized() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let ev = report_event(&mut a, &r, &rp, true, ts(1000));
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1000 + HOLD + 1));
        ok(&a, &act);
    }

    #[test]
    fn sufficient_corroboration_releases_the_hold_early() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let (b, bp) = community(&mut a, "B", ts(10));
        let (c, cp) = community(&mut a, "C", ts(10));
        let ev = report_event(&mut a, &r, &rp, true, ts(1000));
        corroborate(&mut a, &ev, &b, &bp, ts(1100));
        corroborate(&mut a, &ev, &c, &cp, ts(1200));
        // Still inside the hold, but two distinct communities corroborated.
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1300));
        ok(&a, &act);
    }

    #[test]
    fn one_corroborator_is_insufficient_within_hold() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let (b, bp) = community(&mut a, "B", ts(10));
        let ev = report_event(&mut a, &r, &rp, true, ts(1000));
        corroborate(&mut a, &ev, &b, &bp, ts(1100));
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1300));
        bad(&a, &act);
    }

    #[test]
    fn originator_self_corroboration_does_not_release_the_hold() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let ev = report_event(&mut a, &r, &rp, true, ts(1000));
        // The originator "corroborating" itself is not independent support.
        corroborate(&mut a, &ev, &r, &rp, ts(1100));
        corroborate(&mut a, &ev, &r, &rp, ts(1200));
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1300));
        bad(&a, &act);
    }

    #[test]
    fn action_on_non_qualifying_event_is_unconstrained() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        // No triggers → structured skepticism does not apply; act immediately.
        let ev = report_event(&mut a, &r, &rp, false, ts(1000));
        let act = invoke_action(&mut a, &ev, &r, &rp, ts(1000 + 3600));
        ok(&a, &act);
    }

    #[test]
    fn action_on_unknown_event_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (r, rp) = community(&mut a, "R", ts(10));
        let bogus = Cid::of_bytes(b"not an event");
        let act = invoke_action(&mut a, &bogus, &r, &rp, ts(2000));
        bad(&a, &act);
    }
}
