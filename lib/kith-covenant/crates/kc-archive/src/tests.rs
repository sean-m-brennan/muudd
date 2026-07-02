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
use kc_core::{keys, time, Alg, DidKey, KcError, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
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
