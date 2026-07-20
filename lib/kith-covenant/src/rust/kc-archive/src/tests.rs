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

use crate::authority::{authorize, check_authority, DEFAULT_PANEL_MIN, DEFAULT_QUORUM_THRESHOLD};
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
    check_authority(&archive, DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
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
