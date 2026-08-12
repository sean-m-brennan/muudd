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
use kc_core::{
    keys, time, Alg, Cid, DidKey, KcError, KcResult, PrivKey, PubKey, Signed, Timestamp,
};
use kc_mediation::{
    ArgumentRecord, ArgumentRole, ConsentAnswer, DecisionOutcome, DisputeRecord,
    MediationDecision, MediatorEntry, MediatorRegistry, PanelConsent,
};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};

use crate::authority::{
    authorize, authorize_with_caps, check_authority, ExaminerCaps, MediatorCaps, ObserverCaps,
    DEFAULT_PANEL_MIN, DEFAULT_QUORUM_THRESHOLD, DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS,
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

/// A community's DID and the private key that speaks for it, as the fixtures below
/// hand them back. The pair is the unit every fixture deals in, so naming it keeps
/// their return types readable rather than a run of alternating `DidKey`/`PrivKey`.
type Keypair = (DidKey, PrivKey);

/// One seat in the non-party mediator pool: the community, the key that speaks for
/// it, and the key of the mediator it seated. Three separate fixtures hand back a
/// `Vec` of these, which is why it is named once here.
type PoolSeat = (DidKey, PrivKey, PrivKey);

/// What [`mediation_fixture`] returns: complainant `a` and its key, respondent `b` and
/// its key, the filed dispute, and one mediator per non-party community.
type MediationFixture = (DidKey, PrivKey, DidKey, PrivKey, Cid, Vec<PoolSeat>);

/// What the §8 observer `fixture` returns: subject `S` and its key, the reporting
/// community's single observer, and the non-party mediator pool.
type ObserverFixture = (DidKey, PrivKey, Keypair, Vec<PoolSeat>);

/// What `wb_fixture` returns: accused `S` and its key, its delegated insiders as
/// `(did, key, pubkey)` (the resolvable key set the ring is drawn over), and the
/// non-party mediator pool.
type WhistleblowerFixture = (DidKey, PrivKey, Vec<(DidKey, PrivKey, PubKey)>, Vec<PoolSeat>);

/// What the §9 constitution `fixture` returns: the standing compact, then the
/// entrants who are inside a probation window.
type ProbationFixture = (Vec<Keypair>, Vec<Keypair>);

/// What the `attested_nulls` `designated` fixture returns: the designation, its subject,
/// and the three communities whose keys the higher quorum needs.
type DesignatedFixture = (Cid, DidKey, Keypair, Keypair, Keypair);

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
        ExaminerCaps::default(),
        MediatorCaps::default(),
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

/// A mediator term for fixtures: one year, inside §4's two-year maximum.
const FIXTURE_MEDIATOR_TERM: i64 = 365 * 24 * 3600;

fn add_mediators(
    archive: &mut InMemoryArchive,
    community: &DidKey,
    root_priv: &PrivKey,
    n: usize,
    at: Timestamp,
) -> Vec<(DidKey, PrivKey, PrivKey)> {
    let mut entries = vec![];
    let mut out = vec![];
    for _ in 0..n {
        let (mpriv, mpub) = keys::generate(Alg::Ed25519).unwrap();
        let mdid = DidKey::from_pubkey(&mpub).unwrap();
        // A second keypair, registered as the encryption key and never used to sign:
        // §8 custody seals a share of a source's identity to it by key agreement.
        let (epriv, epub) = keys::generate(Alg::Ed25519).unwrap();
        entries.push(MediatorEntry {
            mediator_did: mdid.clone(),
            mediator_pubkey: mpub,
            encryption_pubkey: epub,
            valid_from: at,
            // A bounded term, well inside §4's maximum. Every fixture here used to seat
            // `None` — a permanent mediator class, which is exactly what the term limit
            // exists to prevent.
            valid_until: Some(ts(at.as_second() + FIXTURE_MEDIATOR_TERM)),
            trained_by: None,
        });
        out.push((mdid, mpriv, epriv));
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

/// File one argument on `dispute_cid` and return its CID.
fn argue(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    author: &DidKey,
    signers: &[(&str, &PrivKey)],
    role: ArgumentRole,
    at: Timestamp,
) -> Cid {
    let a = Signed::build(
        ArgumentRecord {
            dispute_cid,
            author: author.clone(),
            role,
            position: "as we read it".into(),
            evidence: vec![],
            submitted_at: at,
        },
        signers,
    )
    .unwrap();
    archive.append(ArchiveEntry::ArgumentFiled(a)).unwrap()
}

/// Every panelist files their own argument, one second apart ending at `at`, and the
/// CIDs are returned in panel order. A decision must cite one per co-signer, so this is
/// what "a panel that deliberated" looks like in a fixture.
fn panel_arguments(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    at: Timestamp,
) -> Vec<Cid> {
    let n = panel.len() as i64;
    panel
        .iter()
        .enumerate()
        .map(|(i, (d, p))| {
            let when = ts(at.as_second() - n + i as i64);
            argue(archive, dispute_cid, d, &[(d.as_str(), *p)], ArgumentRole::Panelist, when)
        })
        .collect()
}

fn decide(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    at: Timestamp,
) -> Cid {
    let arguments = panel_arguments(archive, dispute_cid, panel, at);
    decide_citing(archive, dispute_cid, panel, arguments, at)
}

/// As [`decide`], with the citation supplied — for the cases where what the decision
/// claims about the deliberation is the thing under test.
///
/// The outcome is `Upheld`, which is the neutral one for tests about panel composition
/// and deliberation. It read `Settled` until §4.1 gave that word a rule, at which point
/// every fixture here was recording a settlement neither party had agreed to. Tests
/// about settlement use [`decide_settled`], which obtains the consent first.
fn decide_citing(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    arguments: Vec<Cid>,
    at: Timestamp,
) -> Cid {
    decide_citing_outcome(archive, dispute_cid, panel, arguments, DecisionOutcome::Upheld, at)
}

fn decide_citing_outcome(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    panel: &[(&DidKey, &PrivKey)],
    arguments: Vec<Cid>,
    outcome: DecisionOutcome,
    at: Timestamp,
) -> Cid {
    let panel_dids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
    let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
    let dec = Signed::build(
        MediationDecision {
            dispute_cid,
            panel: panel_dids,
            outcome,
            reasoning: "agreed".into(),
            sealed: None,
            arguments,
            decided_at: at,
        },
        &pairs,
    )
    .unwrap();
    archive.append(ArchiveEntry::MediationDecided(dec)).unwrap()
}

/// A party's consent (or withholding) to `panel` on `dispute_cid`.
fn consent(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    party: &DidKey,
    party_priv: &PrivKey,
    panel: &[(&DidKey, &PrivKey)],
    answer: ConsentAnswer,
    at: Timestamp,
) -> Cid {
    let c = Signed::build(
        PanelConsent {
            dispute_cid,
            party: party.clone(),
            panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            answer,
            statement: "as constituted".into(),
            given_at: at,
        },
        &[(party.as_str(), party_priv)],
    )
    .unwrap();
    archive
        .append(ArchiveEntry::PanelConsentRecorded(c))
        .unwrap()
}

/// Both parties consent to `panel`, then the panel records a settlement.
fn decide_settled(
    archive: &mut InMemoryArchive,
    dispute_cid: Cid,
    parties: (&DidKey, &PrivKey, &DidKey, &PrivKey),
    panel: &[(&DidKey, &PrivKey)],
    at: Timestamp,
) -> Cid {
    let (ca, ca_priv, cb, cb_priv) = parties;
    consent(archive, dispute_cid, ca, ca_priv, panel, ConsentAnswer::Given, at);
    consent(archive, dispute_cid, cb, cb_priv, panel, ConsentAnswer::Given, at);
    let arguments = panel_arguments(archive, dispute_cid, panel, at);
    decide_citing_outcome(archive, dispute_cid, panel, arguments, DecisionOutcome::Settled, at)
}

/// Complainant A, respondent B, and three non-party communities C/D/E each with
/// one mediator. See [`MediationFixture`] for what the elements are; the doc here used
/// to list five and the type has six, having omitted the respondent's key.
fn mediation_fixture(a: &mut InMemoryArchive) -> MediationFixture {
    let (ca, ca_priv) = community(a, "A", ts(10));
    let (cb, cb_priv) = community(a, "B", ts(10));
    let (cc, cc_priv) = community(a, "C", ts(10));
    let (cd, cd_priv) = community(a, "D", ts(10));
    let (ce, ce_priv) = community(a, "E", ts(10));
    let mut pool = vec![];
    pool.extend(add_mediators(a, &cc, &cc_priv, 1, ts(10)));
    pool.extend(add_mediators(a, &cd, &cd_priv, 1, ts(10)));
    pool.extend(add_mediators(a, &ce, &ce_priv, 1, ts(10)));
    let dispute = file_dispute(a, &ca, &ca_priv, &cb, ts(100));
    (ca, ca_priv, cb, cb_priv, dispute, pool)
}

/// §2's second withholding: a probationary community carries no deliberative weight
/// in *mediator selection*, so it may not seat mediators — otherwise each admitted
/// entrant immediately staffs the pool that will judge its own disputes.
#[test]
fn probationary_community_may_not_nominate_mediators() {
    let mut a = InMemoryArchive::new();
    let (sponsor, sponsor_priv) = community(&mut a, "Sponsor", ts(10));
    let (entrant, entrant_priv) = community(&mut a, "Entrant", ts(10));
    // Probation window [100, 10_000).
    let vouch = Signed::build(
        kc_ceremony::inter::SponsorshipRecord {
            sponsor_community: sponsor.clone(),
            entrant_community: entrant.clone(),
            admission_cid: None,
            probation_until: ts(10_000),
            statement: "vouched".into(),
            sponsored_at: ts(100),
        },
        &[(sponsor.as_str(), &sponsor_priv)],
    )
    .unwrap();
    a.append(ArchiveEntry::SponsorshipVouched(vouch)).unwrap();

    // Inside the window: refused. The registry record is the selection act.
    add_mediators(&mut a, &entrant, &entrant_priv, 1, ts(500));
    let inside = a
        .iter()
        .filter(|(_, e)| matches!(e, ArchiveEntry::MediatorRegistryUpdated(_)))
        .map(|(c, _)| *c)
        .last()
        .unwrap();
    let err = authorize(&a, a.get(&inside).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect_err("a probationary community does not select mediators");
    assert!(err.to_string().contains("mediator selection"), "got: {err}");

    // NEGATIVE CONTROL: once the window has closed, the same act is authorized.
    add_mediators(&mut a, &entrant, &entrant_priv, 1, ts(20_000));
    let after = a
        .iter()
        .filter(|(_, e)| matches!(e, ArchiveEntry::MediatorRegistryUpdated(_)))
        .map(|(c, _)| *c)
        .last()
        .unwrap();
    authorize(&a, a.get(&after).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("after probation the community selects mediators like any member");
}

/// §2: a panelist whose home community is probationary may sit on a mediation panel
/// but does not count toward `panel_min`. This is the rule that closes the re-entry
/// route, where a re-admitted community's earlier nominations are still live and so
/// its mediators are already seated when its new probation window opens.
#[test]
fn probationary_home_panelist_does_not_count_toward_panel_min() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
    let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
    let dec = decide(&mut a, dispute, &panel, ts(200));
    // The full 3-panel is authorized before any probation window exists.
    authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("a valid non-party 3-panel is authorized");

    // Now one panelist's home community enters probation across the decision.
    let (sponsor, sponsor_priv) = community(&mut a, "Sponsor", ts(10));
    let home = crate::authority::mediator_home(&a, pool[0].0.as_str(), ts(200))
        .expect("the panelist has a seating community");
    let vouch = Signed::build(
        kc_ceremony::inter::SponsorshipRecord {
            sponsor_community: sponsor.clone(),
            entrant_community: home,
            admission_cid: None,
            probation_until: ts(10_000),
            statement: "vouched".into(),
            sponsored_at: ts(100),
        },
        &[(sponsor.as_str(), &sponsor_priv)],
    )
    .unwrap();
    a.append(ArchiveEntry::SponsorshipVouched(vouch)).unwrap();

    let err = authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect_err("2 of 3 panelists carry weight, below panel_min");
    let msg = err.to_string();
    assert!(
        msg.contains("2 distinct mediators") && msg.contains("carry no weight"),
        "the refusal must say which panelists were discounted, got: {msg}"
    );
    // NEGATIVE CONTROL: the same panel is authorized at a panel_min of 2.
    authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, 2)
        .expect("the two weight-carrying panelists satisfy a panel_min of 2");
}

#[test]
fn mediation_dispute_and_valid_panel_are_authorized() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
    // The dispute record itself (complainant root-signed) is authorized.
    authorize(&a, a.get(&dispute).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("filed dispute is authorized");
    // A 3-mediator, all-non-party panel decides.
    let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
    let dec = decide(&mut a, dispute, &panel, ts(200));
    authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
        .expect("valid non-party 3-panel is authorized");
}

#[test]
fn mediation_panel_below_minimum_is_rejected() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
    let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
    let dec = decide(&mut a, dispute, &panel, ts(200));
    assert!(authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn mediation_party_mediator_violates_non_overlap() {
    let mut a = InMemoryArchive::new();
    let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
    // The complainant A also nominates a mediator and seats it on the panel.
    let a_meds = add_mediators(&mut a, &ca, &ca_priv, 1, ts(10));
    let mut panel: Vec<(&DidKey, &PrivKey)> =
        pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
    panel.push((&a_meds[0].0, &a_meds[0].1));
    let dec = decide(&mut a, dispute, &panel, ts(200));
    assert!(authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN).is_err());
}

#[test]
fn mediation_unregistered_panelist_is_rejected() {
    let mut a = InMemoryArchive::new();
    let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
    let (stray_did, stray_priv) = {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        (DidKey::from_pubkey(&pk).unwrap(), sk)
    };
    let mut panel: Vec<(&DidKey, &PrivKey)> =
        pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
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

    /// A designation could name its representatives and say nothing about what they
    /// concluded. `criteria`, `cited_acts` and `reversal_conditions` were all enforced
    /// non-empty while the deliberation's own reasoning was not — the same decoration
    /// §4's decisions carried, one section over. "Other parties must be able to see the
    /// designation, the reasons for it, and the conduct that triggered it"
    /// (bounded_conflict.md).
    #[test]
    fn a_deliberation_cycle_must_publish_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (s, (p, pp), (qc, qp), cited) = designation_fixture(&mut a);
        let mut silent = cycle(1, &p, 1100 + W, 1200 + W);
        silent.reasoning = "  ".into();
        let dz = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Normal,
            &[(&p, &pp), (&qc, &qp)], cited,
            vec![cycle(0, &p, 1000, 1100), silent],
            "three graver failed restorations", Some("changed conduct over probation".into()),
            Some(ts(99_999)), ts(2000));
        let err = authorize(&a, a.get(&dz).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        assert!(
            err.to_string().contains("must publish its reasoning"),
            "got: {err}"
        );
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

    /// §2's third withholding: probationary communities carry no deliberative weight
    /// in *designation procedures*. An expedited designation trades the two-cycle
    /// cadence for a higher immediate quorum, so it is exactly where a faction that
    /// had just seated its entrants would want to count them.
    #[test]
    fn probationary_deliberators_do_not_count_toward_a_designation_quorum() {
        let mut a = InMemoryArchive::new();
        let (s, _sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (qc, qp) = community(&mut a, "Q", ts(1));
        let (r, rp) = community(&mut a, "R", ts(1));

        // Three deliberators satisfy the expedited quorum (> DEFAULT_QUORUM_THRESHOLD).
        let dz = designation(&mut a, &s, DesignationTier::Anathema, DesignationUrgency::Expedited,
            &[(&p, &pp), (&qc, &qp), (&r, &rp)], vec![], vec![cycle(0, &p, 1000, 1100)],
            "acute harm", Some("rev".into()), Some(ts(5000)), ts(2000));
        ok(&a, &dz);

        // R is then vouched into a probation window spanning the designation.
        let vouch = Signed::build(
            kc_ceremony::inter::SponsorshipRecord {
                sponsor_community: p.clone(),
                entrant_community: r.clone(),
                admission_cid: None,
                probation_until: ts(10_000),
                statement: "vouched".into(),
                sponsored_at: ts(100),
            },
            &[(p.as_str(), &pp)],
        )
        .unwrap();
        a.append(ArchiveEntry::SponsorshipVouched(vouch)).unwrap();

        let err = authorize(&a, a.get(&dz).unwrap(), Q, P)
            .expect_err("a probationary deliberator does not count");
        let msg = err.to_string();
        assert!(
            msg.contains("deliberative weight") && msg.contains("§2"),
            "the refusal must name the §2 exclusion, got: {msg}"
        );

        // NEGATIVE CONTROL: a fourth, unencumbered deliberator restores the quorum.
        let (t, tp) = community(&mut a, "T", ts(1));
        let restored = designation(&mut a, &s, DesignationTier::Anathema,
            DesignationUrgency::Expedited,
            &[(&p, &pp), (&qc, &qp), (&r, &rp), (&t, &tp)], vec![],
            vec![cycle(0, &p, 1000, 1100)],
            "acute harm", Some("rev".into()), Some(ts(5000)), ts(2000));
        ok(&a, &restored);
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
        // Two communities OTHER than the petitioner make up the admitting quorum.
        // (§2: the petitioner does not count toward its own readmission. This
        // fixture used to read `[(&s, &sp), (&p, &pp)]` — petitioner plus one — which
        // is a single community admitting itself.)
        let re = reentry(&mut a, dz, &s, &sp, rec, ws, &[(&p, &pp), (&w1, &w1p)], true);
        ok(&a, &re);
    }

    /// §2: readmission is an admission, so "no single sponsor cluster can admit
    /// alone" applies to it — and the emptiest such cluster is the returning
    /// community itself. Petitioner-plus-one is the arithmetic that used to pass.
    #[test]
    fn reentry_quorum_does_not_count_the_petitioner() {
        let mut a = InMemoryArchive::new();
        let (s, sp) = community(&mut a, "S", ts(1));
        let (p, pp) = community(&mut a, "P", ts(1));
        let (w1, w1p) = community(&mut a, "W1", ts(1));
        let (w2, w2p) = community(&mut a, "W2", ts(1));
        let dz = min_designation(&mut a, &s, DesignationTier::Anathema);
        let rec = recitation_for(&s, &sp, true);
        let ws = vec![witness_att(&w1, &w1p, true), witness_att(&w2, &w2p, true)];
        let re = reentry(&mut a, dz, &s, &sp, rec, ws, &[(&s, &sp), (&p, &pp)], true);
        let err = authorize(&a, a.get(&re).unwrap(), Q, P).expect_err("subject cannot self-admit");
        assert!(
            err.to_string().contains("own readmission"),
            "the refusal must name the subject exclusion, got: {err}"
        );
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
    fn injunction_fixture(a: &mut InMemoryArchive) -> (DidKey, Vec<(DidKey, PrivKey, PrivKey)>) {
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
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100 + NINETY_DAYS);
        ok(&a, &inj);
    }

    /// §2 reaches the emergency panel too. Before the probation filter was shared,
    /// this panel counted a seated mediator from a probationary community at full
    /// weight — so the one panel that can suspend a community's capabilities without
    /// a hearing was the cheapest to staff with unseasoned weight.
    #[test]
    fn probationary_home_does_not_count_toward_an_injunction_panel() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100 + NINETY_DAYS);
        ok(&a, &inj);

        // One panelist's home community enters probation across the issuance.
        let (sponsor, sponsor_priv) = community(&mut a, "Sponsor", ts(1));
        let home = crate::authority::mediator_home(&a, pool[0].0.as_str(), ts(100))
            .expect("the panelist has a seating community");
        let vouch = Signed::build(
            kc_ceremony::inter::SponsorshipRecord {
                sponsor_community: sponsor.clone(),
                entrant_community: home,
                admission_cid: None,
                probation_until: ts(10_000),
                statement: "vouched".into(),
                sponsored_at: ts(50),
            },
            &[(sponsor.as_str(), &sponsor_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::SponsorshipVouched(vouch)).unwrap();

        let err = authorize(&a, a.get(&inj).unwrap(), Q, P)
            .expect_err("2 of 3 panelists carry weight, below panel_min");
        let msg = err.to_string();
        assert!(
            msg.contains("injunction panel has 2 distinct mediators")
                && msg.contains("carry no weight"),
            "the refusal must name the discounted panelist, got: {msg}"
        );
        // The panelist still *sits* — what is withheld is weight, not participation:
        // at a panel_min of 2 the same record is authorized.
        authorize(&a, a.get(&inj).unwrap(), Q, 2)
            .expect("the two weight-carrying panelists satisfy a panel_min of 2");
    }

    #[test]
    fn injunction_below_panel_minimum_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        // Only two non-party mediators — below panel_min.
        let small: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
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
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let inj = issue(&mut a, &s, None, vec![Capability::Trade], &panel, 100, 100);
        bad(&a, &inj);
    }

    #[test]
    fn injunction_challenge_by_different_panel_ok_same_panel_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, pool) = injunction_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
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
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
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
    use kc_core::{KeyImage, RevealedShare, SourcePreimage, ThresholdCustody};
    use kc_observer::{
        ObserverEntry, ObserverRegistry, ObserverReport, ReportResponse, SourceExculpation,
        SourceReveal, WhistleblowerReport,
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

    /// Give `community` `n` delegated speakers, so it has a resolvable insider set
    /// wide enough to hide a whistleblower in. Returns (did, priv, pub) per speaker.
    #[allow(clippy::type_complexity)]
    fn add_insiders(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        n: usize,
        at: Timestamp,
    ) -> Vec<(DidKey, PrivKey, PubKey)> {
        let mut out = vec![];
        for _ in 0..n {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            let did = DidKey::from_pubkey(&pk).unwrap();
            a.append(ArchiveEntry::SpeakerDelegated(grant(
                community, root_priv, &did, &pk, at, None,
            )))
            .unwrap();
            out.push((did, sk, pk));
        }
        out
    }

    /// The ring the authority pass will expect for `accused` at `at`: its root key,
    /// every live delegation, and every member on every roll it has published (unioned,
    /// never superseded). Built here from the same archive the rule reads, so a test
    /// that gets this wrong is testing the rule and not the helper.
    fn expected_ring(a: &InMemoryArchive, accused: &DidKey, at: Timestamp) -> Vec<PubKey> {
        let mut ring = vec![];
        for (_, e) in a.iter() {
            if let ArchiveEntry::CommunityIdentityPublished(s) = e {
                if &s.payload.did == accused {
                    ring.push(s.payload.root_pubkey.clone());
                }
            }
        }
        for (_, e) in a.iter() {
            if let ArchiveEntry::SpeakerDelegated(s) = e {
                if &s.payload.community_did == accused && s.payload.active_at(at) {
                    ring.push(s.payload.speaker_pubkey.clone());
                }
            }
        }
        for (_, e) in a.iter() {
            if let ArchiveEntry::MemberRollPublished(s) = e {
                if &s.payload.community_did == accused && s.payload.published_at <= at {
                    for m in &s.payload.members {
                        ring.push(m.member_pubkey.clone());
                    }
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        ring.retain(|pk| seen.insert(pk.bytes.clone()));
        ring
    }

    /// The parts of a filing a test may want to vary.
    struct Filing {
        wb: WhistleblowerReport,
        /// The source's own DID, as committed to in custody.
        source_did: DidKey,
    }

    /// Build a well-formed filing: a real ring attestation over `accused`'s insider
    /// set, signed by `source_index` of that ring, with the source identity held
    /// k-of-n by the reception panel.
    // Eight axes because the §8 rejection cases need each one independently wrong: a
    // ring drawn over the wrong community, a source outside the insider set, a pubkey
    // that does not match the signing key, a panel that is not the registered
    // reception panel, the claim text, a threshold above or below the custodians
    // actually sealed to, and the instant that decides which member roll the ring is
    // drawn from. This builds the *well-formed* filing so each test can corrupt one
    // thing; a params struct would only relocate the same eight.
    #[allow(clippy::too_many_arguments)]
    fn build_filing(
        a: &InMemoryArchive,
        accused: &DidKey,
        source_priv: &PrivKey,
        source_pub: &PubKey,
        panel: &[(&DidKey, &PrivKey)],
        claim: &str,
        threshold: usize,
        at: Timestamp,
    ) -> Filing {
        let ring = expected_ring(a, accused, at);
        let source_index = ring
            .iter()
            .position(|pk| pk.bytes == source_pub.bytes)
            .expect("the source must be in its community's insider set");
        let source_did = DidKey::from_pubkey(source_pub).unwrap();
        let preimage = SourcePreimage::new(source_did.as_str());
        let commitment = preimage.commitment().unwrap();
        // Sealed to each panelist's *registered encryption key*, resolved from the
        // archive — the production path. Naming a custodian is not sealing to them, so
        // a filing that gets this wrong is refused by the authority pass.
        let holders: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let custody = ThresholdCustody::seal(
            &kc_core::canonical::to_canonical_cbor(&preimage).unwrap(),
            &holders,
            crate::mediator_encryption_resolver(a, at),
            threshold,
        )
        .unwrap();
        let insider = WhistleblowerReport::attest(
            accused.clone(),
            commitment,
            claim,
            at,
            ring,
            source_index,
            source_priv,
        )
        .unwrap();
        Filing {
            wb: WhistleblowerReport {
                accused_community: accused.clone(),
                source_commitment: commitment,
                claim: claim.into(),
                references: vec![],
                reception_panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
                filed_at: at,
                insider,
                custody,
            },
            source_did,
        }
    }

    /// The §8 resolution, but falling back to a holder's own `did:key` when it is not a
    /// registered mediator. Only for fixtures that deliberately name an unregistered
    /// holder: the rule under test is then the authority pass's, and `seal` must not
    /// refuse first or the test would pass for the wrong reason.
    fn lenient_enc(
        a: &InMemoryArchive,
        at: Timestamp,
    ) -> impl Fn(&DidKey) -> kc_core::KcResult<PubKey> + '_ {
        let strict = crate::mediator_encryption_resolver(a, at);
        move |did: &DidKey| strict(did).or_else(|_| did.to_pubkey())
    }

    fn append_filing(
        a: &mut InMemoryArchive,
        wb: WhistleblowerReport,
        panel: &[(&DidKey, &PrivKey)],
    ) -> Cid {
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let signed = Signed::build(wb, &pairs).unwrap();
        a.append(ArchiveEntry::WhistleblowerFiled(signed)).unwrap()
    }

    fn reveal_with(
        a: &mut InMemoryArchive,
        report_cid: Cid,
        panel: &[(&DidKey, &PrivKey)],
        cause: &str,
        disclosed_source: &str,
        shares: Vec<RevealedShare>,
        at: Timestamp,
    ) -> Cid {
        let ids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let rev = Signed::build(
            SourceReveal {
                report_cid,
                panel: ids,
                cause: cause.into(),
                disclosed_source: disclosed_source.into(),
                shares,
                revealed_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::SourceRevealed(rev)).unwrap()
    }

    /// Open every panelist's custody share.
    /// Each custodian opens its own share with its **encryption** key, which is not the
    /// key it signs with: `pool` carries `(did, signing, encryption)`. Passing the
    /// signing key here is now refused by name, which is the point of the change.
    fn open_shares(
        wb: &WhistleblowerReport,
        pool: &[(DidKey, PrivKey, PrivKey)],
    ) -> Vec<RevealedShare> {
        pool.iter()
            .map(|(d, _sign, enc)| wb.custody.open_share(d, enc).unwrap())
            .collect()
    }

    /// Subject S, a reporting community R with one observer, plus a three-mediator
    /// non-party pool C/D/E. Returns (s, s_priv, obs, pool).
    fn fixture(a: &mut InMemoryArchive) -> ObserverFixture {
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

    /// The whistleblower fixture: accused S with two delegated insiders (so its
    /// resolvable key set is three deep), plus the non-party mediator pool.
    fn wb_fixture(a: &mut InMemoryArchive) -> WhistleblowerFixture {
        let (s, s_priv, _obs, pool) = fixture(a);
        let insiders = add_insiders(a, &s, &s_priv, 2, ts(10));
        (s, s_priv, insiders, pool)
    }

    #[test]
    fn whistleblower_with_insider_attestation_and_custody_authorized() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
    }

    // --- custody must be sealed to keys its named holders control ---------------
    //
    // Measured before these rules existed: a filing naming exactly the reception panel
    // but sealing every share to one outsider's key was `verify`-clean and **authorized**,
    // with 0 of 3 named custodians able to open a share. The lawful reveal path was dead
    // for good, one actor held the whole secret, and nothing on the record showed it.

    #[test]
    fn an_honestly_sealed_filing_authorizes_and_every_custodian_opens_its_own_share() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        // Every custodian opens with its registered *encryption* key, not its signing key.
        for (did, _sign, enc) in &pool {
            f.wb.custody.open_share(did, enc).expect("its own share opens");
        }
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
    }

    /// Build a filing whose custody is sealed by `resolve` instead of honestly.
    fn filing_sealed_by(
        a: &InMemoryArchive,
        accused: &DidKey,
        source: (&PrivKey, &PubKey),
        panel: &[(&DidKey, &PrivKey)],
        resolve: impl Fn(&DidKey) -> kc_core::KcResult<PubKey>,
        at: Timestamp,
    ) -> WhistleblowerReport {
        let ring = expected_ring(a, accused, at);
        let idx = ring.iter().position(|pk| pk.bytes == source.1.bytes).unwrap();
        let source_did = DidKey::from_pubkey(source.1).unwrap();
        let preimage = SourcePreimage::new(source_did.as_str());
        let commitment = preimage.commitment().unwrap();
        let holders: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let custody = ThresholdCustody::seal(
            &kc_core::canonical::to_canonical_cbor(&preimage).unwrap(),
            &holders,
            resolve,
            P,
        )
        .unwrap();
        let insider = WhistleblowerReport::attest(
            accused.clone(), commitment, "coerced labor", at, ring, idx, source.0,
        )
        .unwrap();
        WhistleblowerReport {
            accused_community: accused.clone(),
            source_commitment: commitment,
            claim: "coerced labor".into(),
            references: vec![],
            reception_panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            filed_at: at,
            insider,
            custody,
        }
    }

    #[test]
    fn custody_sealed_to_a_substituted_key_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let (att_sk, att_pk) = keys::generate(Alg::Ed25519).unwrap();
        let wb = filing_sealed_by(
            &a, &s, (&insiders[0].1, &insiders[0].2), &panel,
            |_| Ok(att_pk.clone()), ts(100),
        );
        // The measurement, preserved: no named custodian can open, and one key opens all.
        assert_eq!(
            pool.iter().filter(|(d, _, e)| wb.custody.open_share(d, e).is_ok()).count(),
            0
        );
        assert_eq!(
            pool.iter().filter(|(d, _, _)| wb.custody.open_share(d, &att_sk).is_ok()).count(),
            pool.len()
        );
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn custody_sealed_to_another_panelists_key_is_rejected() {
        // Subtler than an outsider: every key belongs to a real custodian, just the
        // wrong one, so the holder set and the key set both look right.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let registered = crate::mediator_encryption_keys(&a, ts(100));
        let rotated: Vec<PubKey> = pool
            .iter()
            .map(|(d, _, _)| registered.get(d.as_str()).unwrap().clone())
            .collect();
        let order: Vec<String> = pool.iter().map(|(d, _, _)| d.as_str().to_string()).collect();
        let wb = filing_sealed_by(
            &a, &s, (&insiders[0].1, &insiders[0].2), &panel,
            |did| {
                let i = order.iter().position(|o| o == did.as_str()).unwrap();
                Ok(rotated[(i + 1) % rotated.len()].clone())
            },
            ts(100),
        );
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn custody_naming_a_holder_with_no_registered_encryption_key_is_rejected() {
        // Reaching this rule takes care, and the care is the point. The reception-panel
        // rule checks the *signers*, while the custody rule checks the *holders*, and
        // `holders == reception_panel` is checked between them — so a holder who is not
        // a registered mediator is normally caught earlier, as a signer. The gap it
        // leaves is a `reception_panel` naming somebody who never signs: the three real
        // mediators sign (meeting `panel_min`), the panel lists a fourth who does not,
        // and custody is sealed to that fourth. Then, and only then, is this the first
        // rule to fire.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let (stranger, _stranger_priv) = community(&mut a, "Stranger", ts(10));
        let signers: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let mut listed = signers.clone();
        listed.push((&stranger, &insiders[0].1)); // key unused: it never signs
        let wb = filing_sealed_by(
            &a, &s, (&insiders[0].1, &insiders[0].2), &listed,
            lenient_enc(&a, ts(100)), ts(100),
        );
        let cid = append_filing(&mut a, wb, &signers);
        bad(&a, &cid);
    }

    #[test]
    fn a_re_registration_after_a_filing_does_not_invalidate_it() {
        // The custody rule reads the registry at `filed_at`, so a community rotating its
        // mediator's encryption key later cannot retroactively break an honest filing —
        // the same log-order principle the ring is bound by.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
        // C re-registers its mediator under a fresh encryption key, dated after filing.
        let (cf, cf_priv) = community(&mut a, "Rotator", ts(10));
        add_mediators(&mut a, &cf, &cf_priv, 1, ts(300));
        ok(&a, &cid);
    }

    #[test]
    fn a_mediator_registered_with_its_signing_key_as_its_encryption_key_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "C", ts(10));
        let (_mpriv, mpub) = keys::generate(Alg::Ed25519).unwrap();
        let mdid = DidKey::from_pubkey(&mpub).unwrap();
        let reg = Signed::build(
            MediatorRegistry {
                nominating_community: c.clone(),
                entries: vec![MediatorEntry {
                    mediator_did: mdid,
                    mediator_pubkey: mpub.clone(),
                    encryption_pubkey: mpub,
                    valid_from: ts(10),
                    valid_until: None,
                    trained_by: None,
                }],
                issued_at: ts(10),
            },
            &[(c.as_str(), &c_priv)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::MediatorRegistryUpdated(reg)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn whistleblower_panel_from_accused_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, insiders, pool) = wb_fixture(&mut a);
        // The accused S also nominates a mediator and seats it on the reception panel.
        let s_meds = add_mediators(&mut a, &s, &s_priv, 1, ts(10));
        let mut panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        panel.push((&s_meds[0].0, &s_meds[0].1));
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn whistleblower_panel_below_minimum_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", 2, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        bad(&a, &cid);
    }

    /// **The manufactured filing.** Before this slice a reception panel could file a
    /// whistleblower report with a made-up `source_commitment` and no source
    /// involvement whatsoever, and it passed. The archive held no evidence an insider
    /// existed, so a non-party panel could invent insider complaints against a rival.
    #[test]
    fn a_panel_cannot_manufacture_a_filing_without_a_source() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // A ring the panel assembles from its own keys, signed by a panelist: the
        // shape of a filing with no insider behind it.
        let ring: Vec<PubKey> = panel.iter().map(|(d, _)| d.to_pubkey().unwrap()).collect();
        let insider = WhistleblowerReport::attest(
            s.clone(),
            Cid::of_bytes(b"commit"),
            "coerced labor",
            ts(100),
            ring,
            0,
            panel[0].1,
        )
        .unwrap();
        let holders: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let custody =
            ThresholdCustody::seal(b"whatever", &holders, lenient_enc(&a, ts(100)), P).unwrap();
        let wb = WhistleblowerReport {
            accused_community: s,
            source_commitment: Cid::of_bytes(b"commit"),
            claim: "coerced labor".into(),
            references: vec![],
            reception_panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            filed_at: ts(100),
            insider,
            custody,
        };
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
        let _ = insiders;
    }

    #[test]
    fn a_ring_missing_an_insider_key_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        // Narrow the ring by dropping a member who is not the signer. A smaller ring
        // is a narrower suspect set — with the source still validly hidden inside it —
        // so the rule insists on the community's whole resolvable key set.
        let signer = insiders[0].2.clone();
        let mut ring = f.wb.insider.ring.clone();
        let drop_at = ring.iter().position(|pk| pk.bytes != signer.bytes).unwrap();
        ring.remove(drop_at);
        assert!(ring.len() >= 2, "the narrowed ring must still be signable");
        let index = ring.iter().position(|pk| pk.bytes == signer.bytes).unwrap();
        let insider = WhistleblowerReport::attest(
            s.clone(),
            f.wb.source_commitment,
            &f.wb.claim,
            ts(100),
            ring,
            index,
            &insiders[0].1,
        )
        .unwrap();
        // The narrowed attestation is internally valid; only the ring-set rule catches it.
        insider.verify().unwrap();
        let wb = WhistleblowerReport { insider, ..f.wb };
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn a_ring_padded_with_an_outsider_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let (_, outsider_pub) = keys::generate(Alg::Ed25519).unwrap();
        let mut ring = f.wb.insider.ring.clone();
        ring.push(outsider_pub);
        let index = ring
            .iter()
            .position(|pk| pk.bytes == insiders[0].2.bytes)
            .unwrap();
        let insider = WhistleblowerReport::attest(
            s.clone(),
            f.wb.source_commitment,
            &f.wb.claim,
            ts(100),
            ring,
            index,
            &insiders[0].1,
        )
        .unwrap();
        let wb = WhistleblowerReport { insider, ..f.wb };
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn a_community_with_one_key_has_no_anonymous_channel() {
        // A ring of one names its signer. Rather than hand a source a signed
        // confession while telling them they are anonymous, the filing is refused.
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // S has only its root key: a ring of one, which `attest` will not even build.
        let ring = expected_ring(&a, &s, ts(100));
        assert_eq!(ring.len(), 1);
        assert!(WhistleblowerReport::attest(
            s.clone(),
            Cid::of_bytes(b"c"),
            "claim",
            ts(100),
            ring,
            0,
            &s_priv,
        )
        .is_err());
        // And a filing that pads the ring to two with a foreign key is refused by
        // the ring-set rule, so there is no way around it.
        let (_, foreign) = keys::generate(Alg::Ed25519).unwrap();
        let mut ring = expected_ring(&a, &s, ts(100));
        ring.push(foreign);
        let insider = WhistleblowerReport::attest(
            s.clone(),
            Cid::of_bytes(b"c"),
            "claim",
            ts(100),
            ring,
            0,
            &s_priv,
        )
        .unwrap();
        let holders: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let custody = ThresholdCustody::seal(b"x", &holders, lenient_enc(&a, ts(100)), P).unwrap();
        let wb = WhistleblowerReport {
            accused_community: s,
            source_commitment: Cid::of_bytes(b"c"),
            claim: "claim".into(),
            references: vec![],
            reception_panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            filed_at: ts(100),
            insider,
            custody,
        };
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn an_attestation_cannot_be_moved_to_another_claim() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "the rota is coerced", P, ts(100),
        );
        // The panel keeps the source's attestation but files a graver accusation.
        let swapped =
            WhistleblowerReport { claim: "they are poisoning the water".into(), ..f.wb };
        let cid = append_filing(&mut a, swapped, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn a_custody_threshold_a_minority_could_meet_is_rejected() {
        // 2-of-4 lets any two panelists name the source between them. The rule wants a
        // majority — the smallest threshold no minority can reach.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, mut pool) = wb_fixture(&mut a);
        let (cf, cf_priv) = community(&mut a, "F", ts(10));
        pool.extend(add_mediators(&mut a, &cf, &cf_priv, 1, ts(10)));
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        assert_eq!(panel.len(), 4);
        let minority = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", 2, ts(100),
        );
        let cid = append_filing(&mut a, minority.wb, &panel);
        bad(&a, &cid);
        // Three of four is a majority, and passes.
        let majority = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", 3, ts(100),
        );
        let cid = append_filing(&mut a, majority.wb, &panel);
        ok(&a, &cid);
    }

    #[test]
    fn custody_held_by_someone_other_than_the_panel_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        // Seal the shares to a fourth party instead of the third panelist: the record
        // would claim a quorum gate that the actual key holders do not match.
        let (_, stranger_pub) = keys::generate(Alg::Ed25519).unwrap();
        let stranger_did = DidKey::from_pubkey(&stranger_pub).unwrap();
        let mut holders: Vec<DidKey> = panel.iter().take(2).map(|(d, _)| (*d).clone()).collect();
        holders.push(stranger_did);
        let _ = stranger_pub;
        let custody = ThresholdCustody::seal(b"x", &holders, lenient_enc(&a, ts(100)), P).unwrap();
        let wb = WhistleblowerReport { custody, ..f.wb };
        let cid = append_filing(&mut a, wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn source_reveal_requires_cause_and_valid_panel() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let shares = open_shares(&f.wb, &pool);
        let source = f.source_did.as_str().to_string();
        let wb = append_filing(&mut a, f.wb, &panel);
        // Empty cause: rejected.
        let no_cause =
            reveal_with(&mut a, wb, &panel, "", &source, shares.clone(), ts(200));
        bad(&a, &no_cause);
        // For cause, by a non-party panel, opening the commitment: authorized.
        let good = reveal_with(
            &mut a,
            wb,
            &panel,
            "source recanted; claim contested",
            &source,
            shares,
            ts(300),
        );
        ok(&a, &good);
    }

    /// **The invented disclosure.** Before this slice `source_commitment` was read by
    /// no rule anywhere: `authorize_source_reveal` checked the reference, the cause,
    /// and the panel, and never hashed `disclosed_source`. A panel could name anyone.
    #[test]
    fn a_reveal_cannot_name_someone_the_shares_do_not_recover() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let shares = open_shares(&f.wb, &pool);
        // The other insider, who had nothing to do with the filing.
        let framed = insiders[1].0.as_str().to_string();
        let wb = append_filing(&mut a, f.wb, &panel);
        let framing = reveal_with(&mut a, wb, &panel, "for cause", &framed, shares, ts(200));
        bad(&a, &framing);
    }

    #[test]
    fn a_reveal_naming_an_arbitrary_string_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let shares = open_shares(&f.wb, &pool);
        let wb = append_filing(&mut a, f.wb, &panel);
        let invented = reveal_with(
            &mut a,
            wb,
            &panel,
            "for cause",
            "did:key:zSomeoneWeDislike",
            shares,
            ts(200),
        );
        bad(&a, &invented);
    }

    #[test]
    fn a_reveal_one_share_short_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // Threshold 3 of 3, so two shares must not suffice.
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", 3, ts(100),
        );
        let shares = open_shares(&f.wb, &pool);
        let source = f.source_did.as_str().to_string();
        let wb = append_filing(&mut a, f.wb, &panel);
        let short = reveal_with(
            &mut a,
            wb,
            &panel,
            "for cause",
            &source,
            shares[..2].to_vec(),
            ts(200),
        );
        bad(&a, &short);
        // All three: authorized.
        let full =
            reveal_with(&mut a, wb, &panel, "for cause", &source, shares, ts(300));
        ok(&a, &full);
    }

    #[test]
    fn a_reveal_with_a_tampered_share_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let mut shares = open_shares(&f.wb, &pool);
        shares[1].value.0[0] ^= 0x01;
        let source = f.source_did.as_str().to_string();
        let wb = append_filing(&mut a, f.wb, &panel);
        let cid = reveal_with(&mut a, wb, &panel, "for cause", &source, shares, ts(200));
        bad(&a, &cid);
    }

    #[test]
    fn a_reveal_repeating_one_share_is_rejected() {
        // Otherwise a single panelist reaches the threshold alone.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let shares = open_shares(&f.wb, &pool);
        let doubled = vec![shares[0], shares[0], shares[0]];
        let source = f.source_did.as_str().to_string();
        let wb = append_filing(&mut a, f.wb, &panel);
        let cid = reveal_with(&mut a, wb, &panel, "for cause", &source, doubled, ts(200));
        bad(&a, &cid);
    }

    // --- §8 exculpation: answering a reveal without a panel's leave ---------

    fn exculpate(
        a: &mut InMemoryArchive,
        report_cid: Cid,
        claimant: (&DidKey, &PrivKey),
        at: Timestamp,
    ) -> Cid {
        let exc =
            SourceExculpation::prove(report_cid, claimant.0.clone(), claimant.1, at).unwrap();
        let signed = Signed::build(exc, &[(claimant.0.as_str(), claimant.1)]).unwrap();
        a.append(ArchiveEntry::SourceExculpated(signed)).unwrap()
    }

    #[test]
    fn a_wrongly_named_member_can_clear_itself() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let wb = append_filing(&mut a, f.wb, &panel);
        // insiders[1] did not file; its key image differs from the filing's.
        let cid = exculpate(&mut a, wb, (&insiders[1].0, &insiders[1].1), ts(400));
        ok(&a, &cid);
    }

    #[test]
    fn the_true_source_cannot_disown_its_own_filing() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let wb = append_filing(&mut a, f.wb, &panel);
        // An honest proof from the source reproduces the filing's key image, so the
        // record is a confession and is refused as an exculpation.
        let cid = exculpate(&mut a, wb, (&insiders[0].0, &insiders[0].1), ts(400));
        bad(&a, &cid);
    }

    #[test]
    fn an_exculpation_with_a_forged_proof_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let wb = append_filing(&mut a, f.wb, &panel);
        let mut exc = SourceExculpation::prove(
            wb,
            insiders[1].0.clone(),
            &insiders[1].1,
            ts(400),
        )
        .unwrap();
        exc.proof.response.0[0] ^= 0x01;
        let signed =
            Signed::build(exc, &[(insiders[1].0.as_str(), &insiders[1].1)]).unwrap();
        let cid = a.append(ArchiveEntry::SourceExculpated(signed)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn an_exculpation_borrowing_another_members_image_is_rejected() {
        // Claiming an image you cannot prove is yours would let the true source point
        // at a bystander's image and walk away.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let wb = append_filing(&mut a, f.wb, &panel);
        let mut exc = SourceExculpation::prove(
            wb,
            insiders[0].0.clone(),
            &insiders[0].1,
            ts(400),
        )
        .unwrap();
        // The source swaps in the innocent member's key image.
        exc.claimed_key_image = KeyImage::of(&insiders[1].1).unwrap();
        let signed =
            Signed::build(exc, &[(insiders[0].0.as_str(), &insiders[0].1)]).unwrap();
        let cid = a.append(ArchiveEntry::SourceExculpated(signed)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn an_exculpation_by_a_non_ring_member_is_rejected() {
        // Nobody outside the ring was ever implicated, so there is nothing to clear.
        let mut a = InMemoryArchive::new();
        let (s, _s_priv, insiders, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders[0].1, &insiders[0].2, &panel, "coerced labor", P, ts(100),
        );
        let wb = append_filing(&mut a, f.wb, &panel);
        let cid = exculpate(&mut a, wb, (&pool[0].0, &pool[0].1), ts(400));
        bad(&a, &cid);
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

    // --- the member roll, and the insider set as a property of the log -----------
    //
    // Four measurements taken before any of this existed, kept as the regression
    // tests for it:
    //   M1 a bare community's insider set was 1 — the hole the roll closes;
    //   M2 the whole corpus's anonymity set was 3 (root + 2 delegates);
    //   M3 one delegation backdated *after* a filing invalidated that filing;
    //   M4 one backdated revocation shrank the suspect set 3 → 2 around a source.

    fn member_keys(n: usize) -> Vec<(DidKey, PrivKey, PubKey)> {
        (0..n)
            .map(|_| {
                let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
                (DidKey::from_pubkey(&pk).unwrap(), sk, pk)
            })
            .collect()
    }

    fn roll_of(
        community: &DidKey,
        members: &[(DidKey, PrivKey, PubKey)],
        as_of: Timestamp,
        published_at: Timestamp,
    ) -> kc_identity::MemberRoll {
        kc_identity::MemberRoll {
            community_did: community.clone(),
            members: members
                .iter()
                .map(|(d, _, pk)| kc_identity::RollMember {
                    member_did: d.clone(),
                    member_pubkey: pk.clone(),
                })
                .collect(),
            as_of,
            published_at,
        }
    }

    fn append_roll(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        roll: kc_identity::MemberRoll,
    ) -> Cid {
        let signed = Signed::build(roll, &[(community.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::MemberRollPublished(signed)).unwrap()
    }

    /// M1, preserved: with no delegation and no roll, a community still has a ring of
    /// one and therefore no anonymous channel. The roll widens the set; it does not
    /// manufacture one out of nothing.
    fn insiders(a: &InMemoryArchive, c: &DidKey, at: Timestamp) -> usize {
        crate::insider_keys(a, c, at).len()
    }

    #[test]
    fn m1_a_community_with_no_delegation_and_no_roll_still_has_a_ring_of_one() {
        let mut a = InMemoryArchive::new();
        let (c, _c_priv) = community(&mut a, "Bare", ts(10));
        assert_eq!(insiders(&a, &c, ts(100)), 1);
    }

    #[test]
    fn a_member_roll_widens_the_insider_set() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        // M2: root + 2 delegates was the corpus's whole anonymity set.
        add_insiders(&mut a, &c, &c_priv, 2, ts(10));
        assert_eq!(insiders(&a, &c, ts(100)), 3);
        let members = member_keys(5);
        append_roll(&mut a, &c, &c_priv, roll_of(&c, &members, ts(20), ts(30)));
        assert_eq!(insiders(&a, &c, ts(100)), 8);
    }

    #[test]
    fn a_rolled_member_with_no_delegation_can_file() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        let members = member_keys(4);
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members, ts(20), ts(30)));
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // The source holds no delegation of any kind — only a place on the roll.
        let f = build_filing(
            &a, &s, &members[2].1, &members[2].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
    }

    #[test]
    fn a_roll_only_ever_widens_the_set_a_later_roll_cannot_drop_a_member() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(4);
        append_roll(&mut a, &c, &c_priv, roll_of(&c, &members, ts(20), ts(30)));
        assert_eq!(insiders(&a, &c, ts(100)), 5);
        // A later, shorter roll: the community drops three members. Union-only, so the
        // suspect set does not shrink and whoever just left keeps their channel.
        append_roll(&mut a, &c, &c_priv, roll_of(&c, &members[..1], ts(40), ts(50)));
        assert_eq!(insiders(&a, &c, ts(100)), 5);
    }

    #[test]
    fn a_departed_member_can_still_file() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        let members = member_keys(4);
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members, ts(20), ts(30)));
        // The community re-rolls without members[3], who has left.
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members[..3], ts(40), ts(50)));
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &members[3].1, &members[3].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
    }

    #[test]
    fn a_member_who_also_holds_a_delegation_takes_one_ring_position() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(3);
        append_roll(&mut a, &c, &c_priv, roll_of(&c, &members, ts(20), ts(30)));
        assert_eq!(insiders(&a, &c, ts(100)), 4);
        // members[1] is also made a speaker. The set must not gain a position.
        a.append(ArchiveEntry::SpeakerDelegated(grant(
            &c, &c_priv, &members[1].0, &members[1].2, ts(40), None,
        )))
        .unwrap();
        assert_eq!(insiders(&a, &c, ts(100)), 4);
    }

    #[test]
    fn m3_a_delegation_appended_after_a_filing_cannot_invalidate_it() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, insiders_v, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders_v[0].1, &insiders_v[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
        // The accused backdates a delegation to before the filing instant. Insertion
        // order is authoritative, so the filing's ring is untouched.
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        a.append(ArchiveEntry::SpeakerDelegated(grant(
            &s, &s_priv, &did, &pk, ts(50), None,
        )))
        .unwrap();
        ok(&a, &cid);
    }

    #[test]
    fn m4_a_backdated_revocation_cannot_narrow_a_filed_rings_suspect_set() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, insiders_v, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders_v[0].1, &insiders_v[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
        // The accused revokes the non-source insider, backdated before the filing.
        a.append(ArchiveEntry::SpeakerDelegated(grant(
            &s,
            &s_priv,
            &insiders_v[1].0,
            &insiders_v[1].2,
            ts(10),
            Some(ts(60)),
        )))
        .unwrap();
        ok(&a, &cid);
    }

    #[test]
    fn a_roll_appended_after_a_filing_does_not_widen_that_filing_ring() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, insiders_v, pool) = wb_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &insiders_v[0].1, &insiders_v[0].2, &panel, "coerced labor", P, ts(100),
        );
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
        // The mirror of M3: appending a roll afterwards must not move the ring either,
        // in the widening direction any more than the narrowing one.
        let members = member_keys(3);
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members, ts(20), ts(30)));
        ok(&a, &cid);
    }

    #[test]
    fn a_filing_omitting_a_rolled_member_from_its_ring_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        let members = member_keys(4);
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members, ts(20), ts(30)));
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let mut f = build_filing(
            &a, &s, &members[0].1, &members[0].2, &panel, "coerced labor", P, ts(100),
        );
        // A narrowed ring: drop the last member, keeping the source in it.
        let mut ring = f.wb.insider.ring.clone();
        ring.pop();
        let idx = ring.iter().position(|pk| pk.bytes == members[0].2.bytes).unwrap();
        f.wb.insider = WhistleblowerReport::attest(
            s.clone(),
            f.wb.source_commitment,
            &f.wb.claim,
            ts(100),
            ring,
            idx,
            &members[0].1,
        )
        .unwrap();
        let cid = append_filing(&mut a, f.wb, &panel);
        bad(&a, &cid);
    }

    #[test]
    fn an_empty_member_roll_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let cid = append_roll(&mut a, &c, &c_priv, roll_of(&c, &[], ts(20), ts(30)));
        bad(&a, &cid);
    }

    #[test]
    fn a_roll_naming_the_same_member_twice_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(2);
        let mut roll = roll_of(&c, &members, ts(20), ts(30));
        roll.members.push(roll.members[0].clone());
        let cid = append_roll(&mut a, &c, &c_priv, roll);
        bad(&a, &cid);
    }

    #[test]
    fn a_roll_giving_a_member_a_foreign_key_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(2);
        let (_sk, outsider) = keys::generate(Alg::Ed25519).unwrap();
        let mut roll = roll_of(&c, &members, ts(20), ts(30));
        // The member's own DID, an outsider's key: without the identity rule this puts
        // a non-member in the ring under a member's name.
        roll.members[0].member_pubkey = outsider;
        let cid = append_roll(&mut a, &c, &c_priv, roll);
        bad(&a, &cid);
    }

    #[test]
    fn a_roll_naming_the_community_itself_is_rejected() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(1);
        let mut roll = roll_of(&c, &members, ts(20), ts(30));
        roll.members.push(kc_identity::RollMember {
            member_did: c.clone(),
            member_pubkey: c.to_pubkey().unwrap(),
        });
        let cid = append_roll(&mut a, &c, &c_priv, roll);
        bad(&a, &cid);
    }

    #[test]
    fn only_the_community_itself_may_publish_its_roll() {
        let mut a = InMemoryArchive::new();
        let (c, _c_priv) = community(&mut a, "Polity", ts(10));
        let (other, other_priv) = community(&mut a, "Other", ts(10));
        let members = member_keys(2);
        let signed =
            Signed::build(roll_of(&c, &members, ts(20), ts(30)), &[(other.as_str(), &other_priv)])
                .unwrap();
        let cid = a.append(ArchiveEntry::MemberRollPublished(signed)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn an_envoy_may_not_publish_a_member_roll() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let (esk, epk) = keys::generate(Alg::Ed25519).unwrap();
        let envoy = DidKey::from_pubkey(&epk).unwrap();
        a.append(ArchiveEntry::SpeakerDelegated(
            Signed::build(
                SpeakerDelegation {
                    community_did: c.clone(),
                    speaker_did: envoy.clone(),
                    speaker_pubkey: epk,
                    scope: DelegationScope::Envoy,
                    issued_at: ts(10),
                    expires_at: None,
                    revoked_at: None,
                },
                &[(c.as_str(), &c_priv)],
            )
            .unwrap(),
        ))
        .unwrap();
        let members = member_keys(2);
        let signed =
            Signed::build(roll_of(&c, &members, ts(20), ts(30)), &[(envoy.as_str(), &esk)])
                .unwrap();
        let cid = a.append(ArchiveEntry::MemberRollPublished(signed)).unwrap();
        bad(&a, &cid);
    }

    #[test]
    fn a_tampered_roll_fails_verification() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "Polity", ts(10));
        let members = member_keys(2);
        let cid = append_roll(&mut a, &c, &c_priv, roll_of(&c, &members, ts(20), ts(30)));
        assert!(a.verify(&cid).is_ok());
        let tampered = match a.get(&cid).unwrap() {
            ArchiveEntry::MemberRollPublished(s) => {
                let mut s = s.clone();
                s.payload.members.pop();
                ArchiveEntry::MemberRollPublished(s)
            }
            _ => unreachable!(),
        };
        let mut b = InMemoryArchive::new();
        community(&mut b, "Polity", ts(10));
        // Appending the tampered record: its CID no longer matches its signature.
        let bad_cid = b.append(tampered).unwrap();
        assert!(b.verify(&bad_cid).is_err());
    }

    #[test]
    fn a_reveal_may_name_a_rolled_member() {
        let mut a = InMemoryArchive::new();
        let (s, s_priv, _obs, pool) = fixture(&mut a);
        let members = member_keys(4);
        append_roll(&mut a, &s, &s_priv, roll_of(&s, &members, ts(20), ts(30)));
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let f = build_filing(
            &a, &s, &members[1].1, &members[1].2, &panel, "coerced labor", P, ts(100),
        );
        let source_did = f.source_did.clone();
        let shares = open_shares(&f.wb, &pool[..P]);
        let cid = append_filing(&mut a, f.wb, &panel);
        ok(&a, &cid);
        let rev = reveal_with(
            &mut a, cid, &panel, "credible retaliation", source_did.as_str(), shares, ts(200),
        );
        ok(&a, &rev);
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
            event_cid: *event_cid,
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
            event_cid: *event_cid,
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

/// §2 admissions integrity — the enforcement half (`capture_resistance.md` §7).
///
/// The compact's answer to admissions capture had been "make it legible": the
/// composition-drift dashboard, sponsor stake, and harness scenario 7 all *see* a
/// faction seating entrants, and nothing refused it. These are the gate's cases.
///
/// The two `measure_*` tests are the slice's measurement kept as its regression
/// test: each was authored green against the pre-gate code, and each records in its
/// assertion message the number it measured.
mod admissions_integrity {
    use super::*;
    use kc_ceremony::inter::{
        ExaminerEntry, ExaminerRegistry, InterCommunityWitnessRecord, SponsorshipRecord,
    };
    use kc_ceremony::intra::IntraCommunityRiteRecord;

    const YEAR: i64 = 365 * 24 * 3600;

    /// A full credentialing round over N participants: each presents one claim per
    /// required feature, and every other participant accepts all of them. This
    /// generalizes the hand-rolled two-party loop in
    /// `full_two_community_ceremony_verifies_and_roundtrips`, which is what an
    /// *admission* needs — a compact formation has two founders, an admission has
    /// the sitting members plus the entrant.
    fn credential(
        label: &str,
        members: &[(&Community, &Signed<IntraCommunityRiteRecord>)],
        now: Timestamp,
    ) -> Signed<InterCommunityWitnessRecord> {
        let participants: Vec<ParticipantContext> = members
            .iter()
            .map(|(c, rite)| ParticipantContext {
                community: &c.signed_identity,
                intra_record: rite,
                cross_boundary_signer_did: c.envoy_did.clone(),
                cross_boundary_signer_priv: &c.envoy_priv,
            })
            .collect();
        let runner = InterCredentialingRunner::new(label.into(), participants);

        let mut claims = Vec::new();
        for (c, _) in members {
            for feature in REQUIRED_FEATURES {
                claims.push(
                    InterCredentialingRunner::present_claim(
                        &c.envoy_did,
                        &c.envoy_priv,
                        c.signed_identity.payload.did.clone(),
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
            for (c, _) in members {
                if c.signed_identity.payload.did == claim.payload.community_did {
                    continue;
                }
                acceptances.push(
                    InterCredentialingRunner::accept(
                        &c.envoy_did,
                        &c.envoy_priv,
                        c.signed_identity.payload.did.clone(),
                        claim.payload_cid,
                        true,
                        None,
                        now,
                    )
                    .unwrap(),
                );
            }
        }
        match runner.seal(claims, acceptances, vec![], now).unwrap() {
            InterCredentialingOutcome::Sealed(s) => s,
            InterCredentialingOutcome::NonAdmission(_) => panic!("expected an admission"),
        }
    }

    fn vouch(
        archive: &mut InMemoryArchive,
        sponsor: &Community,
        entrant: &DidKey,
        sponsored_at: Timestamp,
        probation_until: Timestamp,
    ) -> Cid {
        let sponsor_did = sponsor.signed_identity.payload.did.clone();
        let signed = Signed::build(
            SponsorshipRecord {
                sponsor_community: sponsor_did.clone(),
                entrant_community: entrant.clone(),
                admission_cid: None,
                probation_until,
                statement: "operative practice observed".into(),
                sponsored_at,
            },
            &[(sponsor_did.as_str(), &sponsor.root_priv)],
        )
        .unwrap();
        archive.append(ArchiveEntry::SponsorshipVouched(signed)).unwrap()
    }

    /// Seat this community's cross-boundary signer as its examiner over
    /// `[valid_from, valid_until)`.
    ///
    /// The examiner at a credentialing *is* the community's cross-boundary signer:
    /// the runner co-signs the witness record with exactly one signer per
    /// participant, and principal binding requires that signature to be the
    /// community's own root or `Envoy`. So §2's caps constrain who a community may
    /// send to examine an entrant, and for how long it may keep sending them — which
    /// is the bite the admitting quorum does not have.
    fn seat_examiner(
        archive: &mut InMemoryArchive,
        c: &Community,
        valid_from: Timestamp,
        valid_until: Option<Timestamp>,
        issued_at: Timestamp,
    ) -> KcResult<Cid> {
        let did = c.signed_identity.payload.did.clone();
        let signed = Signed::build(
            ExaminerRegistry {
                nominating_community: did.clone(),
                entries: vec![ExaminerEntry {
                    examiner_did: c.envoy_did.clone(),
                    examiner_pubkey: c.envoy_delegation.payload.speaker_pubkey.clone(),
                    valid_from,
                    valid_until,
                }],
                issued_at,
            },
            &[(did.as_str(), &c.root_priv)],
        )
        .unwrap();
        archive.append(ArchiveEntry::ExaminerRegistryUpdated(signed))
    }

    /// A community seeded into the archive with its intra rite sealed and appended —
    /// everything a credentialing participant needs.
    fn member(archive: &mut InMemoryArchive, name: &str, now: Timestamp)
        -> (Community, Signed<IntraCommunityRiteRecord>)
    {
        let c = mk_community(name, now);
        seed_archive(archive, &c);
        let rite = run_intra_rite(&c, now);
        archive.append(ArchiveEntry::IntraCommunityRite(rite.clone())).unwrap();
        (c, rite)
    }

    /// MEASUREMENT (step 1 of the §2 slice), kept as the regression test.
    ///
    /// Capture move 7 with nothing in its way: a faction of one co-founds a compact
    /// with its first aligned entrant, then seats a stream of further entrants where
    /// every co-signer other than the faction is itself inside its sponsorship
    /// probation window. **Measured against the pre-gate code: the faction seated 5
    /// of 5 attempted entrants, `verify_all` green and `check_authority` green at a
    /// quorum threshold of 3** — every admission procedurally clean, which is
    /// exactly what `capture_resistance.md` §7 says makes this vector hard to see.
    #[test]
    fn measure_faction_of_one_seats_entrants_unopposed() {
        let now = time::now();
        let probation_until = now.checked_add((YEAR / 3600).hours()).unwrap();
        let mut archive = InMemoryArchive::new();
        let (faction, faction_rite) = member(&mut archive, "Faction", now);

        let mut seated: Vec<(Community, Signed<IntraCommunityRiteRecord>)> = Vec::new();
        let mut refused: Option<String> = None;
        for i in 0..5 {
            let (entrant, rite) = member(&mut archive, &format!("Entrant-{i}"), now);
            let mut members: Vec<(&Community, &Signed<IntraCommunityRiteRecord>)> =
                vec![(&faction, &faction_rite)];
            members.extend(seated.iter().map(|(c, r)| (c, r)));
            members.push((&entrant, &rite));

            let witness = credential("captured-compact", &members, now);
            // The sponsor stakes its standing first, opening the entrant's probation:
            // vouch, then seal. §2 counts only a vouch already in the log.
            vouch(
                &mut archive,
                &faction,
                &entrant.signed_identity.payload.did,
                now,
                probation_until,
            );
            let cid = archive
                .append(ArchiveEntry::InterCommunityWitness(witness))
                .unwrap();
            if let Err(e) = authorize(&archive, archive.get(&cid).unwrap(), 3, DEFAULT_PANEL_MIN) {
                refused = Some(e.to_string());
                break;
            }
            seated.push((entrant, rite));
        }

        verify_all(&archive).expect("every record is crypto-intact either way");
        assert_eq!(
            seated.len(),
            1,
            "a faction of one must seat only its co-founder; measured 5 of 5 seated \
             before the §2 gate existed. Refusal: {refused:?}"
        );
        let reason = refused.expect("the second admission must be refused");
        assert!(
            reason.contains("admitting"),
            "the refusal must name the admitting quorum, got: {reason}"
        );
    }

    /// A standing compact of three founders, each sending a seated examiner. The
    /// fixture the examining-panel rules are exercised against.
    fn standing_compact(
        archive: &mut InMemoryArchive,
        now: Timestamp,
        seat: bool,
    ) -> Vec<(Community, Signed<IntraCommunityRiteRecord>)> {
        let term_end = now.checked_add((90 * 24).hours()).unwrap();
        let founders: Vec<(Community, Signed<IntraCommunityRiteRecord>)> = ["A", "B", "C"]
            .iter()
            .map(|n| member(archive, n, now))
            .collect();
        let refs: Vec<(&Community, &Signed<IntraCommunityRiteRecord>)> =
            founders.iter().map(|(c, r)| (c, r)).collect();
        let formation = credential("standing-compact", &refs, now);
        archive
            .append(ArchiveEntry::InterCommunityWitness(formation))
            .unwrap();
        if seat {
            for (c, _) in &founders {
                seat_examiner(archive, c, now, Some(term_end), now).unwrap();
            }
        }
        founders
    }

    /// One entrant presented to a standing compact by all of its members, vouched
    /// for beforehand by `sponsor` when one is given.
    ///
    /// The vouch precedes the admission, which is both the order
    /// `recognition_protocols.md` describes and the only order §2 counts: the sponsor
    /// speaks for the candidate, then the compact seals the admission.
    fn admit_entrant(
        archive: &mut InMemoryArchive,
        founders: &[(Community, Signed<IntraCommunityRiteRecord>)],
        sponsor: Option<&Community>,
        name: &str,
        now: Timestamp,
    ) -> Cid {
        let (entrant, rite) = member(archive, name, now);
        if let Some(sp) = sponsor {
            vouch(
                archive,
                sp,
                &entrant.signed_identity.payload.did,
                now,
                now.checked_add((YEAR / 3600).hours()).unwrap(),
            );
        }
        let mut refs: Vec<(&Community, &Signed<IntraCommunityRiteRecord>)> =
            founders.iter().map(|(c, r)| (c, r)).collect();
        refs.push((&entrant, &rite));
        let witness = credential("standing-compact", &refs, now);
        archive
            .append(ArchiveEntry::InterCommunityWitness(witness))
            .unwrap()
    }

    /// §2: an admission into a standing compact must be examined by a panel of seated
    /// examiners, not merely co-signed. Before the rule, the credentialing runner
    /// imposed a headcount of two participants and nothing else — there was no term,
    /// no rotation, and no cap on how much of the examining function one community
    /// could hold.
    #[test]
    fn admission_without_a_seated_panel_is_refused() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let founders = standing_compact(&mut archive, now, false);
        let cid = admit_entrant(&mut archive, &founders, Some(&founders[0].0), "Entrant", now);
        let err = authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect_err("an unexamined admission must be refused");
        assert!(
            err.to_string().contains("credentialing panel"),
            "expected the panel rule, got: {err}"
        );
    }

    /// NEGATIVE CONTROL: the same admission, examined by a seated panel spanning
    /// three communities, is authorized. The gate refuses unexamined admissions, not
    /// admissions.
    #[test]
    fn admission_examined_by_a_seated_panel_is_authorized() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let founders = standing_compact(&mut archive, now, true);
        let cid = admit_entrant(&mut archive, &founders, Some(&founders[0].0), "Entrant", now);
        authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect("a properly examined admission is authorized");
        verify_all(&archive).expect("and it verifies");
    }

    /// NEGATIVE CONTROL for the formation exemption: a compact's own founding record
    /// has no sitting members to draw a quorum or a panel from, and is authorized.
    /// All 10 admission records that existed before this rule were formations.
    #[test]
    fn compact_formation_is_exempt() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let (a, a_rite) = member(&mut archive, "A", now);
        let (b, b_rite) = member(&mut archive, "B", now);
        let formation = credential("new-compact", &[(&a, &a_rite), (&b, &b_rite)], now);
        let cid = archive
            .append(ArchiveEntry::InterCommunityWitness(formation))
            .unwrap();
        authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect("compact formation is exempt: these are founders, not entrants");
    }

    /// §2 cross-community composition: a panel drawn from one community is the
    /// standing admissions cadre wearing a rotation schedule.
    #[test]
    fn single_community_examining_panel_is_refused() {
        let now = time::now();
        let term_end = now.checked_add((90 * 24).hours()).unwrap();
        let mut archive = InMemoryArchive::new();
        let founders = standing_compact(&mut archive, now, false);
        // Only A seats an examiner, so A alone examines.
        seat_examiner(&mut archive, &founders[0].0, now, Some(term_end), now).unwrap();
        let cid = admit_entrant(&mut archive, &founders, Some(&founders[0].0), "Entrant", now);
        // A panel_min of 1 is satisfied, so composition is the only rule left to
        // fire — otherwise the panel-size refusal would mask it.
        let caps = ExaminerCaps { panel_min: 1, ..ExaminerCaps::default() };
        let err = authorize_with_caps(
            &archive,
            archive.get(&cid).unwrap(),
            2,
            DEFAULT_PANEL_MIN,
            ObserverCaps::default(),
            caps,
            MediatorCaps::default(),
        )
        .expect_err("a single-community panel must be refused");
        assert!(
            err.to_string().contains("single community"),
            "expected the composition rule, got: {err}"
        );
    }

    /// NEGATIVE CONTROL 5 (§2 term limit, shared with §8): an open-ended examining
    /// seat is refused; the same roster with a bounded term is authorized. This is
    /// what proves the shared caps checker is wired to the examiner path and not
    /// merely present in the file.
    #[test]
    fn open_ended_examining_term_is_refused_bounded_is_not() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let (a, _) = member(&mut archive, "A", now);

        let open = seat_examiner(&mut archive, &a, now, None, now).unwrap();
        let err = authorize(&archive, archive.get(&open).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect_err("no open-ended examining standing");
        assert!(err.to_string().contains("open-ended"), "got: {err}");

        let bounded = seat_examiner(
            &mut archive,
            &a,
            now,
            Some(now.checked_add((90 * 24).hours()).unwrap()),
            now,
        )
        .unwrap();
        authorize(&archive, archive.get(&bounded).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect("a bounded term is fine");
    }

    /// NEGATIVE CONTROL 6 (§2 continuous-service union, shared with §8): overlapping
    /// re-nominations never trip the rotation-gap rule, so a community could chain
    /// them to keep one examiner seated indefinitely. A copy of the §8 rules that had
    /// dropped the union step would pass this test — which is why the two components
    /// share one implementation.
    #[test]
    fn overlap_chained_examining_terms_are_refused() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let (a, _) = member(&mut archive, "A", now);
        let day = |d: i64| now.checked_add((d * 24).hours()).unwrap();

        // Three 150-day terms, each inside the 182-day limit on its own, and each
        // opened before every earlier one had ended. No prior term has *ended* by the
        // third window's start, so the rotation-gap rule (ended terms only) stays
        // silent — which is precisely the evasion the union rule exists to catch.
        // Unioned, the continuous service runs 270 days.
        seat_examiner(&mut archive, &a, now, Some(day(150)), now).unwrap();
        seat_examiner(&mut archive, &a, day(100), Some(day(250)), day(100)).unwrap();
        let third = seat_examiner(&mut archive, &a, day(120), Some(day(270)), day(120)).unwrap();

        let err = authorize(&archive, archive.get(&third).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect_err("chained overlapping terms must be refused");
        let msg = err.to_string();
        assert!(
            msg.contains("continuous service") && msg.contains("§2"),
            "expected the §2 union rule, got: {msg}"
        );
    }

    /// §2's first withholding reaches the examining pool: a probationary community
    /// does not staff the body that examines the next entrant.
    #[test]
    fn probationary_community_may_not_seat_examiners() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let (sponsor, _) = member(&mut archive, "Sponsor", now);
        let (entrant, _) = member(&mut archive, "Entrant", now);
        vouch(
            &mut archive,
            &sponsor,
            &entrant.signed_identity.payload.did,
            now,
            now.checked_add((YEAR / 3600).hours()).unwrap(),
        );
        let cid = seat_examiner(
            &mut archive,
            &entrant,
            now,
            Some(now.checked_add((90 * 24).hours()).unwrap()),
            now,
        )
        .unwrap();
        let err = authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect_err("a probationary community does not staff the examining pool");
        assert!(err.to_string().contains("probationary"), "got: {err}");
    }

    /// MEASUREMENT: can a faction evade the probation withholding by simply never
    /// vouching? Probation only opens when someone stakes standing on the entrant, and
    /// nothing so far requires an admission to carry a sponsorship at all — so three
    /// aligned communities that seat examiners and skip the vouch would face no
    /// probationary de-weighting of anything.
    #[test]
    fn measure_admission_without_a_sponsor_is_refused() {
        let now = time::now();
        let mut archive = InMemoryArchive::new();
        let founders = standing_compact(&mut archive, now, true);

        let mut seated = 0usize;
        let mut refused: Option<String> = None;
        for i in 0..5 {
            // No `vouch` anywhere: no sponsorship, so no probation window opens.
            let cid = admit_entrant(&mut archive, &founders, None, &format!("Entrant-{i}"), now);
            match authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN) {
                Ok(()) => seated += 1,
                Err(e) => {
                    refused = Some(e.to_string());
                    break;
                }
            }
        }
        assert_eq!(
            seated, 0,
            "an unsponsored admission must be refused; measured 5 of 5 seated before \
             the sponsorship requirement existed. Refusal: {refused:?}"
        );
        let reason = refused.expect("the first unsponsored admission must be refused");
        assert!(
            reason.contains("sponsor"),
            "the refusal must name the missing sponsorship, got: {reason}"
        );
    }

    /// MEASUREMENT (step 1), kept as the regression test: an entrant admitted this
    /// second may vouch for the next. **Measured against the pre-gate code:
    /// authorized.** The chain this opens is the patient-sponsorship one — today's
    /// entrant is tomorrow's sponsor, and probation withheld nothing.
    #[test]
    fn measure_probationary_community_may_not_sponsor() {
        let now = time::now();
        let probation_until = now.checked_add((YEAR / 3600).hours()).unwrap();
        let mut archive = InMemoryArchive::new();
        let (sponsor, _) = member(&mut archive, "Sitting-Member", now);
        let (entrant, _) = member(&mut archive, "Entrant", now);
        let (next, _) = member(&mut archive, "Next-Entrant", now);

        // The entrant is inside its probation window from `now`.
        vouch(
            &mut archive,
            &sponsor,
            &entrant.signed_identity.payload.did,
            now,
            probation_until,
        );
        // ...and immediately vouches for the next entrant.
        let cid = vouch(
            &mut archive,
            &entrant,
            &next.signed_identity.payload.did,
            now,
            probation_until,
        );

        let err = authorize(&archive, archive.get(&cid).unwrap(), 2, DEFAULT_PANEL_MIN)
            .expect_err("a probationary community carries no weight in admissions (§2)");
        assert!(
            err.to_string().contains("probation"),
            "the refusal must name probation, got: {err}"
        );
    }
}

/// §2 probation reach (the purposive reading): probationary weight counts toward
/// **no** second-class quorum, at the ordinary bar or at the asymmetric contraction
/// bar, and first-class local voice is untouched.
///
/// Every one of these was measured GREEN before the rule existed. Three entrants,
/// each inside a probation window, chartered and contracted the compact-incapable
/// list on their own — the list of matters that may not be compacted away, upstream
/// of every other protection in the tier.
mod probation_reach {
    use super::*;
    use kc_ceremony::inter::SponsorshipRecord;
    use kc_commons::{CommonsKind, CommonsRegistration, CommonsSanction, SanctionLevel};
    use kc_constitution::{
        AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
        FastChange, FastPeriod, IncapableMatter,
    };
    use kc_escalation::LadderTransition;

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const YEAR: i64 = 365 * 24 * 3600;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    /// Refused, *and* refused for the §2 reason — a refusal on other grounds would
    /// pass this test while proving nothing about probation.
    fn refused_for_probation(a: &InMemoryArchive, cid: &Cid) -> String {
        let err = authorize(a, a.get(cid).unwrap(), Q, P)
            .expect_err("expected a refusal");
        let msg = err.to_string();
        assert!(
            msg.contains("probationary") && msg.contains("§2"),
            "the refusal must name probation as the reason, got: {msg}"
        );
        msg
    }

    fn matter(k: &str) -> IncapableMatter {
        IncapableMatter { key: k.into(), description: k.into() }
    }

    type Party<'a> = (&'a DidKey, &'a PrivKey);

    /// A compact of four standing communities with a ratified constitution, plus
    /// three entrants each inside a probation window from ts(100) for a year.
    fn fixture(a: &mut InMemoryArchive) -> ProbationFixture {
        let standing: Vec<(DidKey, PrivKey)> =
            ["A", "B", "C", "D"].iter().map(|n| community(a, n, ts(10))).collect();
        let dids: Vec<DidKey> = standing.iter().map(|(d, _)| d.clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            standing.iter().map(|(d, p)| (d.as_str(), p)).collect();
        let g = Signed::build(
            ConstitutionGenesis {
                compact_incapable: vec![matter("genesis_m")],
                fast_periods: vec![FastPeriod {
                    label: "annual".into(),
                    start: ts(1000),
                    end: ts(2000),
                    recurrence: None,
                }],
                ratifiers: dids,
                genesis_at: ts(20),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::ConstitutionRatified(g)).unwrap();

        let entrants: Vec<(DidKey, PrivKey)> =
            ["E1", "E2", "E3"].iter().map(|n| community(a, n, ts(90))).collect();
        let (sponsor, sponsor_priv) = (standing[0].0.clone(), standing[0].1.clone());
        for (e, _) in &entrants {
            let s = Signed::build(
                SponsorshipRecord {
                    sponsor_community: sponsor.clone(),
                    entrant_community: e.clone(),
                    admission_cid: None,
                    probation_until: ts(100 + YEAR),
                    statement: "operative practice observed".into(),
                    sponsored_at: ts(100),
                },
                &[(sponsor.as_str(), &sponsor_priv)],
            )
            .unwrap();
            a.append(ArchiveEntry::SponsorshipVouched(s)).unwrap();
        }
        (standing, entrants)
    }

    fn amend(
        a: &mut InMemoryArchive,
        kind: AmendmentKind,
        key: &str,
        deciders: &[Party<'_>],
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

    fn party(p: &(DidKey, PrivKey)) -> Party<'_> {
        (&p.0, &p.1)
    }

    #[allow(clippy::too_many_arguments)]
    fn sanction_at(
        a: &mut InMemoryArchive,
        commons_cid: Cid,
        subject: &DidKey,
        from: SanctionLevel,
        to: SanctionLevel,
        imposers: &[Party<'_>],
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
                basis: "overdraw".into(),
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

    fn advance_at(
        a: &mut InMemoryArchive,
        dispute: Cid,
        inits: &[Party<'_>],
        at: Timestamp,
    ) -> Cid {
        let initiators: Vec<DidKey> = inits.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> = inits.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let t = Signed::build(
            LadderTransition {
                dispute_cid: dispute,
                from_rung: LadderRung::LimitedSanctions,
                to_rung: LadderRung::CollectiveSanctions,
                initiators,
                decision_class: DecisionClass::SecondClass,
                reasoning: "x".into(),
                transitioned_at: at,
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::LadderAdvanced(t)).unwrap()
    }

    #[test]
    fn a_probationary_quorum_cannot_amend_the_compact_incapable_list() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        // Measured GREEN before the rule: two communities admitted this year,
        // amending the compact's constitutional list between them.
        let bad = amend(
            &mut a,
            AmendmentKind::Add,
            "m_x",
            &[party(&entrants[0]), party(&entrants[1])],
            ts(200),
        );
        let msg = refused_for_probation(&a, &bad);
        assert!(msg.contains("got 0"), "no weight at all was presented: {msg}");
        // Control: the same act on standing weight is authorized.
        let good = amend(
            &mut a,
            AmendmentKind::Add,
            "m_ok",
            &[party(&standing[0]), party(&standing[1])],
            ts(210),
        );
        ok(&a, &good);
    }

    /// `deliberative_weight` counts communities, not signatures — the property the
    /// four component bars used to get for free from their own `BTreeSet` and now
    /// inherit from the shared counter. One community signing twice is one community.
    #[test]
    fn one_community_signing_twice_carries_one_vote() {
        let mut a = InMemoryArchive::new();
        let (standing, _entrants) = fixture(&mut a);
        let s0 = party(&standing[0]);
        let bad = amend(&mut a, AmendmentKind::Add, "m_dup", &[s0, s0], ts(200));
        let err = authorize(&a, a.get(&bad).unwrap(), Q, P)
            .expect_err("one community twice is not a quorum of two");
        assert!(err.to_string().contains("got 1"), "got: {err}");
        let good = amend(
            &mut a,
            AmendmentKind::Add,
            "m_two",
            &[s0, party(&standing[1])],
            ts(210),
        );
        ok(&a, &good);
    }

    /// THE FINDING. Extending only the generic quorum leaves this green: the ordinary
    /// bar is met by the standing members and the *higher* contraction bar is topped
    /// up with probationary weight, so the +1 collapses to the ordinary quorum.
    #[test]
    fn probationary_weight_cannot_reach_the_incapable_contraction_bar() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let all: Vec<Party<'_>> = standing.iter().map(party).collect();
        // Two matters, and the control runs first: a *refused* amendment still moves
        // the derived list the next one reads, so a control after the attempt would
        // fail for "not in force" — refused for the wrong reason, proving nothing.
        for key in ["m_ctl", "m_adv"] {
            let added = amend(&mut a, AmendmentKind::Add, key, &all[..Q], ts(300));
            ok(&a, &added);
        }
        // Control: Q+1 standing communities contract, which is what the bar means.
        let good = amend(&mut a, AmendmentKind::Remove, "m_ctl", &all[..Q + 1], ts(400));
        ok(&a, &good);

        // The attempt: Q standing (enough for the ordinary quorum) + every entrant
        // (enough for the raw distinct count the bar used to make).
        let mut deciders: Vec<Party<'_>> = all[..Q].to_vec();
        deciders.extend(entrants.iter().map(party));
        let bad = amend(&mut a, AmendmentKind::Remove, "m_adv", &deciders, ts(500));
        let msg = refused_for_probation(&a, &bad);
        assert!(
            msg.contains(&format!("got {Q}")),
            "only the standing members carry weight: {msg}"
        );
    }

    #[test]
    fn probationary_weight_cannot_reach_the_fast_cadence_contraction_bar() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let all: Vec<Party<'_>> = standing.iter().map(party).collect();
        fn contract(
            a: &mut InMemoryArchive,
            deciders: &[Party<'_>],
            at: Timestamp,
        ) -> Cid {
            let dids: Vec<DidKey> = deciders.iter().map(|(d, _)| (*d).clone()).collect();
            let pairs: Vec<(&str, &PrivKey)> =
                deciders.iter().map(|(d, p)| (d.as_str(), *p)).collect();
            let am = Signed::build(
                FastCadenceAmendment {
                    change: FastChange::Contract,
                    period: FastPeriod {
                        label: "annual".into(),
                        start: ts(1000),
                        end: ts(1500),
                        recurrence: None,
                    },
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
        let mut deciders: Vec<Party<'_>> = all[..Q].to_vec();
        deciders.extend(entrants.iter().map(party));
        let bad = contract(&mut a, &deciders, ts(600));
        refused_for_probation(&a, &bad);
        let good = contract(&mut a, &all[..Q + 1], ts(700));
        ok(&a, &good);
    }

    /// Two subjects on one commons, so the control's ladder and the adversarial
    /// ladder cannot interfere: a refused record still sets the level a later
    /// sanction reads, and a control refused for "leaves the wrong level" would
    /// prove nothing.
    #[test]
    fn a_second_class_commons_sanction_needs_standing_imposers() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let (subj1, subj1_priv) = community(&mut a, "S1", ts(10));
        let (subj2, subj2_priv) = community(&mut a, "S2", ts(10));
        let appropriators = vec![
            standing[0].0.clone(),
            standing[1].0.clone(),
            subj1.clone(),
            subj2.clone(),
        ];
        let reg = Signed::build(
            CommonsRegistration {
                kind: CommonsKind::Communication,
                name: "relay".into(),
                description: "a compact-level commons".into(),
                appropriators: appropriators.clone(),
                provision_rule: "provision scales with draw".into(),
                registered_by: appropriators,
                registered_at: ts(200),
            },
            &[
                (standing[0].0.as_str(), &standing[0].1),
                (standing[1].0.as_str(), &standing[1].1),
                (subj1.as_str(), &subj1_priv),
                (subj2.as_str(), &subj2_priv),
            ],
        )
        .unwrap();
        let commons_cid = a.append(ArchiveEntry::CommonsRegistered(reg)).unwrap();
        let s0 = party(&standing[0]);
        let s1 = party(&standing[1]);
        for subject in [&subj1, &subj2] {
            let m = sanction_at(
                &mut a,
                commons_cid,
                subject,
                SanctionLevel::None,
                SanctionLevel::Monitoring,
                &[s0],
                DecisionClass::FirstClass,
                ts(300),
            );
            ok(&a, &m);
            let w = sanction_at(
                &mut a,
                commons_cid,
                subject,
                SanctionLevel::Monitoring,
                SanctionLevel::Warning,
                &[s0],
                DecisionClass::FirstClass,
                ts(400),
            );
            ok(&a, &w);
        }
        // Control first: standing imposers reach Restriction on subject 1.
        let good = sanction_at(
            &mut a,
            commons_cid,
            &subj1,
            SanctionLevel::Warning,
            SanctionLevel::Restriction,
            &[s0, s1],
            DecisionClass::SecondClass,
            ts(500),
        );
        ok(&a, &good);
        // The same act on subject 2, carried by entrants, is refused.
        let bad = sanction_at(
            &mut a,
            commons_cid,
            &subj2,
            SanctionLevel::Warning,
            SanctionLevel::Restriction,
            &[party(&entrants[0]), party(&entrants[1])],
            DecisionClass::SecondClass,
            ts(600),
        );
        refused_for_probation(&a, &bad);
    }

    #[test]
    fn a_second_class_rung_needs_standing_initiators() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        // Two disputes: a refused transition still sets the rung the next one reads,
        // so the control needs its own ladder rather than a later step on this one.
        let file = |name: &str, a: &mut InMemoryArchive| {
            let (resp, _resp_priv) = community(a, name, ts(10));
            let d = Signed::build(
                DisputeRecord {
                    complainant: standing[0].0.clone(),
                    respondent: resp.clone(),
                    rung: LadderRung::LimitedSanctions,
                    summary: "s".into(),
                    references: vec![],
                    filed_at: ts(200),
                },
                &[(standing[0].0.as_str(), &standing[0].1)],
            )
            .unwrap();
            a.append(ArchiveEntry::DisputeFiled(d)).unwrap()
        };
        let dispute = file("Respondent-adv", &mut a);
        let control_dispute = file("Respondent-ctl", &mut a);
        let good = advance_at(
            &mut a,
            control_dispute,
            &[party(&standing[0]), party(&standing[1])],
            ts(300),
        );
        ok(&a, &good);
        let bad = advance_at(&mut a, dispute, &[party(&entrants[0]), party(&entrants[1])], ts(400));
        refused_for_probation(&a, &bad);
    }

    /// The commons floor is `max(quorum, 2)`, so at a quorum of 1 it is *higher* than
    /// the generic one — and a higher bar met with weight the generic bar rejects is
    /// the contraction-bar finding again, one component over. At quorum 2 the generic
    /// layer already refuses this, which is why the test has to configure the quorum
    /// where the component rule is the only thing standing.
    #[test]
    fn the_commons_second_class_floor_counts_weight_at_a_quorum_of_one() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let (subj, subj_priv) = community(&mut a, "S", ts(10));
        let appropriators = vec![standing[0].0.clone(), subj.clone()];
        let reg = Signed::build(
            CommonsRegistration {
                kind: CommonsKind::Communication,
                name: "relay".into(),
                description: "a commons".into(),
                appropriators: appropriators.clone(),
                provision_rule: "provision scales with draw".into(),
                registered_by: appropriators,
                registered_at: ts(200),
            },
            &[(standing[0].0.as_str(), &standing[0].1), (subj.as_str(), &subj_priv)],
        )
        .unwrap();
        let commons_cid = a.append(ArchiveEntry::CommonsRegistered(reg)).unwrap();
        let s0 = party(&standing[0]);
        let m = sanction_at(
            &mut a, commons_cid, &subj, SanctionLevel::None, SanctionLevel::Monitoring,
            &[s0], DecisionClass::FirstClass, ts(300),
        );
        authorize(&a, a.get(&m).unwrap(), 1, P).expect("first-class, one imposer");
        let w = sanction_at(
            &mut a, commons_cid, &subj, SanctionLevel::Monitoring, SanctionLevel::Warning,
            &[s0], DecisionClass::FirstClass, ts(400),
        );
        authorize(&a, a.get(&w).unwrap(), 1, P).expect("first-class, one imposer");
        // One standing imposer meets the generic quorum of 1; the entrant is what
        // would carry it over the floor of 2.
        let bad = sanction_at(
            &mut a, commons_cid, &subj, SanctionLevel::Warning, SanctionLevel::Restriction,
            &[s0, party(&entrants[0])], DecisionClass::SecondClass, ts(500),
        );
        let err = authorize(&a, a.get(&bad).unwrap(), 1, P)
            .expect_err("the floor of 2 is not reachable on one standing imposer");
        let msg = err.to_string();
        assert!(
            msg.contains("probationary") && msg.contains("got 1"),
            "the refusal must name probation and the weight it counted, got: {msg}"
        );
    }

    /// The same shape for the second-class ladder rung, whose floor is also
    /// `max(quorum, 2)`.
    #[test]
    fn the_second_class_rung_floor_counts_weight_at_a_quorum_of_one() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let (resp, _resp_priv) = community(&mut a, "Respondent", ts(10));
        let d = Signed::build(
            DisputeRecord {
                complainant: standing[0].0.clone(),
                respondent: resp.clone(),
                rung: LadderRung::LimitedSanctions,
                summary: "s".into(),
                references: vec![],
                filed_at: ts(200),
            },
            &[(standing[0].0.as_str(), &standing[0].1)],
        )
        .unwrap();
        let dispute = a.append(ArchiveEntry::DisputeFiled(d)).unwrap();
        let bad = advance_at(
            &mut a,
            dispute,
            &[party(&standing[0]), party(&entrants[0])],
            ts(300),
        );
        let err = authorize(&a, a.get(&bad).unwrap(), 1, P)
            .expect_err("the floor of 2 is not reachable on one standing initiator");
        assert!(err.to_string().contains("probationary"), "got: {err}");
    }

    /// The boundary the doctrine draws: probation withholds *compact-wide* weight and
    /// nothing else. A first-class act — a local body acting inside its own envelope —
    /// is untouched, which is why the withholding is not a year of silence.
    #[test]
    fn a_first_class_act_is_untouched_by_probation() {
        let mut a = InMemoryArchive::new();
        let (standing, entrants) = fixture(&mut a);
        let (subj, subj_priv) = community(&mut a, "S", ts(10));
        let appropriators = vec![standing[0].0.clone(), entrants[0].0.clone(), subj.clone()];
        let reg = Signed::build(
            CommonsRegistration {
                kind: CommonsKind::Communication,
                name: "relay".into(),
                description: "a commons".into(),
                appropriators: appropriators.clone(),
                provision_rule: "provision scales with draw".into(),
                registered_by: appropriators,
                registered_at: ts(200),
            },
            &[
                (standing[0].0.as_str(), &standing[0].1),
                (entrants[0].0.as_str(), &entrants[0].1),
                (subj.as_str(), &subj_priv),
            ],
        )
        .unwrap();
        let commons_cid = a.append(ArchiveEntry::CommonsRegistered(reg)).unwrap();
        // A probationary community, acting alone, applies a first-class sanction.
        let s = Signed::build(
            CommonsSanction {
                commons_cid,
                subject_community: subj.clone(),
                from_level: SanctionLevel::None,
                to_level: SanctionLevel::Monitoring,
                basis: "overdraw".into(),
                references: vec![],
                imposers: vec![entrants[0].0.clone()],
                decision_class: DecisionClass::FirstClass,
                decided_at: ts(300),
            },
            &[(entrants[0].0.as_str(), &entrants[0].1)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::SanctionApplied(s)).unwrap();
        ok(&a, &cid);
    }
}

// --- D32: append order is the resolution order ------------------------------
//
// Neither of these two properties had a test before the archive mechanism was
// extracted into `kc-log`, and that is why both were broken. Nothing in this suite
// appended two registry rosters for one community, or two identities for one DID, so
// nothing ever asked which of them key resolution would pick — and the answer was
// whichever the CID-keyed entry index happened to yield.

mod append_order_resolution {
    use super::*;
    use kc_core::SuccessorLink;

    /// `community_identity` documents "the most recent" publication and must mean the
    /// last one appended.
    ///
    /// Before D32 it read `entries.values().rev()` — descending CID-string order, which
    /// is unrelated to publication order. Measured over 200 trials, it returned the
    /// **earlier** of two publications **104** times. Run over enough independent key
    /// pairs here that hash order cannot pass by luck: at 40 trials the old code had
    /// about a 1-in-10^12 chance of surviving.
    #[test]
    fn community_identity_returns_the_last_published_not_the_lowest_hashed() {
        let now = time::now();
        let mut wrong = 0;
        let trials = 40;

        for t in 0..trials {
            let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
            let did = DidKey::from_pubkey(&root_pub).unwrap();
            let first = CommunityIdentity::new_self_signed(
                format!("c{t}"),
                &root_priv,
                root_pub,
                now,
            )
            .unwrap();

            // A second publication for the SAME did — the did is derived from
            // `root_pubkey`, so it must carry the same root key — this one recording a
            // rotation. The successor chain is what a caller asking for "the most
            // recent" identity is after.
            let (next_priv, next_pub) = keys::generate(Alg::Ed25519).unwrap();
            let link = Signed::build(
                SuccessorLink {
                    previous_did: did.clone(),
                    new_did: DidKey::from_pubkey(&next_pub).unwrap(),
                    new_pubkey: next_pub,
                    rotated_at: now,
                },
                &[(did.as_str(), &next_priv)],
            )
            .unwrap();
            let mut later_payload = first.payload.clone();
            later_payload.successor_chain = vec![link];
            let later = Signed::build(later_payload, &[(did.as_str(), &root_priv)]).unwrap();

            let mut a = InMemoryArchive::new();
            a.append(ArchiveEntry::CommunityIdentityPublished(first)).unwrap();
            a.append(ArchiveEntry::CommunityIdentityPublished(later)).unwrap();
            assert_eq!(a.len(), 2, "two distinct publications, both retained");

            if a.community_identity(&did).unwrap().payload.successor_chain.is_empty() {
                wrong += 1;
            }
        }

        assert_eq!(wrong, 0, "returned the earlier publication in {wrong}/{trials} trials");
    }

    /// A *superseded* witness key must not verify a signature.
    ///
    /// The precondition is itself part of the finding: only `CommunityIdentity` binds a
    /// DID to its pubkey (`CommunityIdentity::verify_self_signed`). Registry entries do
    /// not, so one roster can name a witness DID with key K1 and a later roster the same
    /// DID with K2 — and both entries can be inside their validity windows at once,
    /// because a roster supersedes as a whole rather than retracting entries. Before D32
    /// `resolve_did` took the first match out of the CID-keyed index, so the retired K1
    /// still resolved, and still verified, about half the time.
    #[test]
    fn a_superseded_witness_key_no_longer_verifies_and_the_current_one_does() {
        let now = time::now();
        let c = mk_community("Alpha", now);
        let community_did = c.signed_identity.payload.did.clone();
        let mut a = InMemoryArchive::new();
        seed_archive(&mut a, &c);

        // The witness rotates: a second roster, issued an hour later, names the same
        // witness DID with a new key. `mk_community`'s roster runs to now+24h, so at
        // now+2h BOTH entries for this DID are inside their windows and only
        // latest-issued-wins can separate them.
        let (new_witness_priv, new_witness_pub) = keys::generate(Alg::Ed25519).unwrap();
        let rotated = Signed::build(
            WitnessRegistry {
                community_did: community_did.clone(),
                entries: vec![WitnessEntry {
                    witness_did: c.witness_did.clone(),
                    witness_pubkey: new_witness_pub.clone(),
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                }],
                issued_at: now.checked_add(1.hour()).unwrap(),
            },
            &[(community_did.as_str(), &c.root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::WitnessRegistryUpdated(rotated)).unwrap();

        let at = now.checked_add(2.hours()).unwrap();

        // Signed by the RETIRED key: resolution yields the current key, so the
        // signature does not verify.
        let stale = delegate_signed_conduct(
            &community_did,
            &c.witness_did,
            &c.witness_priv,
            at,
            "signed by the retired witness key",
        );
        let stale_cid = a.append(ArchiveEntry::ConductAttested(stale)).unwrap();
        assert!(
            matches!(a.verify(&stale_cid), Err(KcError::SignatureInvalid)),
            "a retired witness key must not verify"
        );

        // Signed by the CURRENT key: it does. Without this half the rule could be
        // "refuse everything" and the test above would still pass.
        let fresh = delegate_signed_conduct(
            &community_did,
            &c.witness_did,
            &new_witness_priv,
            at,
            "signed by the current witness key",
        );
        let fresh_cid = a.append(ArchiveEntry::ConductAttested(fresh)).unwrap();
        a.verify(&fresh_cid).expect("the current witness key must verify");
    }

    /// Supersession is per nominating community: one community rotating its roster must
    /// not blind resolution to another community's witnesses.
    #[test]
    fn one_communitys_rotation_does_not_hide_anothers_roster() {
        let now = time::now();
        let alpha = mk_community("Alpha", now);
        let beta = mk_community("Beta", now);
        let mut a = InMemoryArchive::new();
        seed_archive(&mut a, &alpha);
        seed_archive(&mut a, &beta);

        // Alpha rotates; Beta does not.
        let (alpha_new_priv, alpha_new_pub) = keys::generate(Alg::Ed25519).unwrap();
        let alpha_did = alpha.signed_identity.payload.did.clone();
        let rotated = Signed::build(
            WitnessRegistry {
                community_did: alpha_did.clone(),
                entries: vec![WitnessEntry {
                    witness_did: alpha.witness_did.clone(),
                    witness_pubkey: alpha_new_pub,
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                }],
                issued_at: now.checked_add(1.hour()).unwrap(),
            },
            &[(alpha_did.as_str(), &alpha.root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::WitnessRegistryUpdated(rotated)).unwrap();

        let at = now.checked_add(2.hours()).unwrap();
        let beta_did = beta.signed_identity.payload.did.clone();

        // Beta's original witness key still resolves and verifies.
        let beta_rec = delegate_signed_conduct(
            &beta_did,
            &beta.witness_did,
            &beta.witness_priv,
            at,
            "beta's witness, unrotated",
        );
        let cid = a.append(ArchiveEntry::ConductAttested(beta_rec)).unwrap();
        a.verify(&cid).expect("beta's roster is untouched by alpha's rotation");

        // And Alpha's new one does too.
        let alpha_rec = delegate_signed_conduct(
            &alpha_did,
            &alpha.witness_did,
            &alpha_new_priv,
            at,
            "alpha's rotated witness",
        );
        let cid = a.append(ArchiveEntry::ConductAttested(alpha_rec)).unwrap();
        a.verify(&cid).expect("alpha's current roster resolves");
    }

    /// A roster issued in the future must not resolve keys in the present.
    #[test]
    fn a_roster_not_yet_issued_does_not_resolve() {
        let now = time::now();
        let c = mk_community("Alpha", now);
        let community_did = c.signed_identity.payload.did.clone();
        let mut a = InMemoryArchive::new();
        seed_archive(&mut a, &c);

        // A roster issued well after the record we are about to verify.
        let (future_priv, future_pub) = keys::generate(Alg::Ed25519).unwrap();
        let future_did = DidKey::from_pubkey(&future_pub).unwrap();
        let ahead = Signed::build(
            WitnessRegistry {
                community_did: community_did.clone(),
                entries: vec![WitnessEntry {
                    witness_did: future_did.clone(),
                    witness_pubkey: future_pub,
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: None,
                }],
                issued_at: now.checked_add(10.hours()).unwrap(),
            },
            &[(community_did.as_str(), &c.root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::WitnessRegistryUpdated(ahead)).unwrap();

        let at = now.checked_add(1.hour()).unwrap();
        let rec = delegate_signed_conduct(
            &community_did,
            &future_did,
            &future_priv,
            at,
            "signed by a witness seated by a roster not yet issued",
        );
        let cid = a.append(ArchiveEntry::ConductAttested(rec)).unwrap();
        assert!(
            matches!(a.verify(&cid), Err(KcError::Key(_))),
            "a roster issued later than the record must not seat its signer"
        );
    }
}

/// §4.1 — the deliberation record, and the two fields that were decoration until it
/// arrived.
///
/// The measurement this slice began from: a paneled decision could carry an **empty
/// `reasoning`** and be authorized, though §4's own specification calls the reasoning
/// trail *mandatory*; and its **`panel` field was read by no rule at all**, so it could
/// name the two *party* communities — precisely whom non-overlap excludes — while three
/// legitimate non-party mediators signed, and the record was still authorized. Nothing
/// recorded *how* a panel decided, which is what the §8 observer corps and the §9
/// erosion signals both assume can be audited. These tests are that measurement.
mod deliberation {
    use super::*;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect("expected authorized");
    }
    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        let msg = err.to_string();
        assert!(msg.contains(needle), "expected {needle:?}, got: {msg}");
    }

    /// A decision that says nothing about why. §4: "public decisions with **mandatory**
    /// reasoning trail" — a requirement the record carried and no rule enforced.
    #[test]
    fn a_decision_must_state_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let arguments = panel_arguments(&mut a, dispute, &panel, ts(200));
        let panel_dids: Vec<DidKey> = panel.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let dec = Signed::build(
            MediationDecision {
                dispute_cid: dispute,
                panel: panel_dids,
                outcome: DecisionOutcome::Upheld,
                reasoning: "   ".into(), // whitespace is not a reason
                sealed: None,
                arguments,
                decided_at: ts(200),
            },
            &pairs,
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::MediationDecided(dec)).unwrap();
        refused(&a, &cid, "must state its reasoning");
    }

    /// The `panel` field is **derived, not chosen**. This is the sharp one: before the
    /// rule, the field an auditor reads to check non-overlap could name the two parties
    /// while three proper non-party mediators signed, and nothing objected. An unread
    /// field that can contradict the rule that did run is worse than an absent one.
    #[test]
    fn a_decision_s_panel_must_be_its_co_signers() {
        let mut a = InMemoryArchive::new();
        let (ca, _ca_priv, cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let arguments = panel_arguments(&mut a, dispute, &panel, ts(200));
        let pairs: Vec<(&str, &PrivKey)> =
            panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let dec = Signed::build(
            MediationDecision {
                dispute_cid: dispute,
                // The two PARTY communities, and only two of them.
                panel: vec![ca.clone(), cb.clone()],
                outcome: DecisionOutcome::Upheld,
                reasoning: "we concur".into(),
                sealed: None,
                arguments,
                decided_at: ts(200),
            },
            &pairs,
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::MediationDecided(dec)).unwrap();
        refused(&a, &cid, "panel must be exactly its co-signers");
    }

    /// Every co-signer must have argued. Three signatures riding on one author's text
    /// is a rubber stamp: the witness rules' arithmetic, which counts the people
    /// present rather than the signatures collected.
    #[test]
    fn a_co_signer_who_did_not_argue_is_refused() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // Only the first panelist files anything.
        let one = vec![argue(
            &mut a,
            dispute,
            panel[0].0,
            &[(panel[0].0.as_str(), panel[0].1)],
            ArgumentRole::Panelist,
            ts(190),
        )];
        let cid = decide_citing(&mut a, dispute, &panel, one, ts(200));
        refused(&a, &cid, "co-signs a decision without having argued it");

        // With all three arguing, the identical panel and outcome go through.
        let mut b = InMemoryArchive::new();
        let (_, _, _, _cb_priv, dispute2, pool2) = mediation_fixture(&mut b);
        let panel2: Vec<(&DidKey, &PrivKey)> = pool2.iter().map(|(d, p, _)| (d, p)).collect();
        let cid2 = decide(&mut b, dispute2, &panel2, ts(200));
        ok(&b, &cid2);
    }

    /// A decision may not cite another dispute's deliberation, nor a record that is not
    /// an argument at all.
    #[test]
    fn a_decision_cites_arguments_on_its_own_dispute() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // A second dispute between the same parties, argued properly.
        let other = file_dispute(&mut a, &ca, &ca_priv, &cb, ts(150));
        let borrowed = panel_arguments(&mut a, other, &panel, ts(190));
        let cid = decide_citing(&mut a, dispute, &panel, borrowed, ts(200));
        refused(&a, &cid, "an argument on another dispute");

        // And citing the dispute record itself is not citing an argument.
        let mut b = InMemoryArchive::new();
        let (_, _, _, _cb_priv, dispute2, pool2) = mediation_fixture(&mut b);
        let panel2: Vec<(&DidKey, &PrivKey)> = pool2.iter().map(|(d, p, _)| (d, p)).collect();
        let cid2 = decide_citing(&mut b, dispute2, &panel2, vec![dispute2], ts(200));
        refused(&b, &cid2, "not a filed argument");
    }

    /// A panel may not decide while ignoring a case a party put to it. Insertion order
    /// is authoritative, as it is for a dispute's current rung, so a party filing
    /// *after* the decision cannot retroactively invalidate it.
    #[test]
    fn a_party_argument_filed_before_the_decision_must_be_answered() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // The respondent answers. Its own root key signs, as for the dispute filing.
        let answer = argue(
            &mut a,
            dispute,
            &cb,
            &[(cb.as_str(), &cb_priv)],
            ArgumentRole::Party,
            ts(180),
        );
        let panelists = panel_arguments(&mut a, dispute, &panel, ts(190));
        let deaf = decide_citing(&mut a, dispute, &panel, panelists.clone(), ts(200));
        refused(&a, &deaf, "does not answer the party argument");

        // Answering it is enough; nothing requires the panel to agree with it.
        let mut cited = panelists;
        cited.push(answer);
        let heard = decide_citing(&mut a, dispute, &panel, cited, ts(300));
        ok(&a, &heard);
    }

    /// A party argument filed *after* a decision leaves that decision standing. The
    /// alternative would let a losing party invalidate a concluded mediation by filing
    /// one more record.
    #[test]
    fn a_party_argument_filed_after_the_decision_does_not_unmake_it() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let cid = decide(&mut a, dispute, &panel, ts(200));
        ok(&a, &cid);
        argue(
            &mut a,
            dispute,
            &cb,
            &[(cb.as_str(), &cb_priv)],
            ArgumentRole::Party,
            ts(400),
        );
        ok(&a, &cid);
    }

    /// Only a party may argue as one, and only an active non-party mediator as a
    /// panelist — the panel rule, one record over.
    #[test]
    fn each_role_is_held_to_its_own_rule() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();

        // A mediator is not a party, so may not make a party's case.
        let cid = argue(
            &mut a,
            dispute,
            panel[0].0,
            &[(panel[0].0.as_str(), panel[0].1)],
            ArgumentRole::Party,
            ts(180),
        );
        refused(&a, &cid, "is not a party to this dispute");

        // And a party community is not a panelist, however it labels itself.
        let cid2 = argue(
            &mut a,
            dispute,
            &ca,
            &[(ca.as_str(), &ca_priv)],
            ArgumentRole::Panelist,
            ts(181),
        );
        refused(&a, &cid2, "not an active registered mediator");

        // A panelist's argument is signed by that panelist alone: nobody argues in a
        // mediator's name.
        let arg = Signed::build(
            ArgumentRecord {
                dispute_cid: dispute,
                author: panel[0].0.clone(),
                role: ArgumentRole::Panelist,
                position: "as we read it".into(),
                evidence: vec![],
                submitted_at: ts(182),
            },
            &[(panel[1].0.as_str(), panel[1].1)],
        )
        .unwrap();
        let cid3 = a.append(ArchiveEntry::ArgumentFiled(arg)).unwrap();
        refused(&a, &cid3, "signed by exactly that panelist");
    }

    /// An argument is *made*. An empty one was not.
    #[test]
    fn an_argument_must_state_a_position() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let arg = Signed::build(
            ArgumentRecord {
                dispute_cid: dispute,
                author: pool[0].0.clone(),
                role: ArgumentRole::Panelist,
                position: "  ".into(),
                evidence: vec![],
                submitted_at: ts(180),
            },
            &[(pool[0].0.as_str(), &pool[0].1)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::ArgumentFiled(arg)).unwrap();
        refused(&a, &cid, "must state a position");
    }
}

/// §4.2 — the mediator pool was the third roster and the one wired to nothing.
///
/// The measurement this slice began from, run against the pre-slice code: a community
/// could hold **9 of 10** mediator seats and its registry authorized; an A-v-B panel of
/// three drawn **entirely from that community** authorized; and every mediator could be
/// seated with **no end date at all**. `check_roster_caps` already carried all four
/// rules and §2's examiners and §8's observers both shared it — mediators were the ones
/// it never reached.
mod mediator_caps {
    use super::*;
    use crate::authority::{
        DEFAULT_MEDIATOR_MAX_TERM_SECONDS, DEFAULT_MEDIATOR_POOL_FLOOR,
        DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
    };

    /// Seat `n` mediators for `community` over an explicit window.
    fn seat(
        a: &mut InMemoryArchive,
        community: &DidKey,
        root_priv: &PrivKey,
        n: usize,
        from: i64,
        until: Option<i64>,
        issued_at: i64,
    ) -> Cid {
        let mut entries = vec![];
        for _ in 0..n {
            let (_mpriv, mpub) = keys::generate(Alg::Ed25519).unwrap();
            let mdid = DidKey::from_pubkey(&mpub).unwrap();
            let (_epriv, epub) = keys::generate(Alg::Ed25519).unwrap();
            entries.push(MediatorEntry {
                mediator_did: mdid,
                mediator_pubkey: mpub,
                encryption_pubkey: epub,
                valid_from: ts(from),
                valid_until: until.map(ts),
                trained_by: None,
            });
        }
        let reg = Signed::build(
            MediatorRegistry {
                nominating_community: community.clone(),
                entries,
                issued_at: ts(issued_at),
            },
            &[(community.as_str(), root_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::MediatorRegistryUpdated(reg)).unwrap()
    }

    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }
    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect("expected authorized");
    }

    const YEAR: i64 = 365 * 24 * 3600;

    /// Row 3 of the measured table: a seat with no end is a permanent mediator class,
    /// "the first step toward an inquisitorial caste" (`liminal_institutions.md`).
    #[test]
    fn a_mediator_seat_must_have_an_end() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "C", ts(10));
        let cid = seat(&mut a, &c, &c_priv, 1, 10, None, 10);
        refused(&a, &cid, "open-ended term");

        // Bounded, and the identical registration goes through.
        let mut b = InMemoryArchive::new();
        let (c2, c2_priv) = community(&mut b, "C", ts(10));
        let cid2 = seat(&mut b, &c2, &c2_priv, 1, 10, Some(10 + YEAR), 10);
        ok(&b, &cid2);
    }

    /// And the term is bounded in length, not merely closed: mediators "must return
    /// home", which a fifty-year term does not accomplish.
    #[test]
    fn a_mediator_term_is_bounded_in_length() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "C", ts(10));
        let over = DEFAULT_MEDIATOR_MAX_TERM_SECONDS + 1;
        let cid = seat(&mut a, &c, &c_priv, 1, 10, Some(10 + over), 10);
        refused(&a, &cid, "exceeds the maximum");
    }

    /// Rows 1 and 2: one community holding a majority of the pool staffs every panel in
    /// the compact, which is what the no-majority share exists to prevent.
    #[test]
    fn no_community_may_hold_a_majority_of_the_mediator_pool() {
        let mut a = InMemoryArchive::new();
        let (cc, cc_priv) = community(&mut a, "C", ts(10));
        let (cd, cd_priv) = community(&mut a, "D", ts(10));
        // D seats 4, then C tries for 9 of 13.
        seat(&mut a, &cd, &cd_priv, 4, 10, Some(10 + YEAR), 10);
        let cid = seat(&mut a, &cc, &cc_priv, 9, 10, Some(10 + YEAR), 10);
        refused(&a, &cid, "a majority");

        // Four of eight is not a majority, and is allowed.
        let mut b = InMemoryArchive::new();
        let (c2, c2_priv) = community(&mut b, "C", ts(10));
        let (d2, d2_priv) = community(&mut b, "D", ts(10));
        seat(&mut b, &d2, &d2_priv, 5, 10, Some(10 + YEAR), 10);
        let cid2 = seat(&mut b, &c2, &c2_priv, 4, 10, Some(10 + YEAR), 10);
        ok(&b, &cid2);
    }

    /// The floor: a compact too small to staff mediation any other way is not refused
    /// for it. `liminal_institutions.md` puts the minimum viable pool near 3n, and the
    /// whitepaper lists the viable band as an open question.
    #[test]
    fn the_pool_share_rule_does_not_bite_a_small_compact() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "C", ts(10));
        let cid = seat(&mut a, &c, &c_priv, DEFAULT_MEDIATOR_POOL_FLOOR - 1, 10, Some(10 + YEAR), 10);
        ok(&a, &cid);
    }

    /// Rotation: a mediator whose term ends must sit out before returning, or the
    /// "return home" is nominal.
    #[test]
    fn a_returning_mediator_must_sit_out_a_rotation() {
        let mut a = InMemoryArchive::new();
        let (c, c_priv) = community(&mut a, "C", ts(10));
        // One mediator, seated twice, with a gap far shorter than the rotation period.
        let (_mpriv, mpub) = keys::generate(Alg::Ed25519).unwrap();
        let mdid = DidKey::from_pubkey(&mpub).unwrap();
        let (_epriv, epub) = keys::generate(Alg::Ed25519).unwrap();
        let entry = |from: i64, until: i64| MediatorEntry {
            mediator_did: mdid.clone(),
            mediator_pubkey: mpub.clone(),
            encryption_pubkey: epub.clone(),
            valid_from: ts(from),
            valid_until: Some(ts(until)),
            trained_by: None,
        };
        let build = |a: &mut InMemoryArchive, e: MediatorEntry, at: i64| {
            let reg = Signed::build(
                MediatorRegistry {
                    nominating_community: c.clone(),
                    entries: vec![e],
                    issued_at: ts(at),
                },
                &[(c.as_str(), &c_priv)],
            )
            .unwrap();
            a.append(ArchiveEntry::MediatorRegistryUpdated(reg)).unwrap()
        };
        let first = build(&mut a, entry(100, 100 + YEAR), 100);
        ok(&a, &first);
        let too_soon = build(&mut a, entry(100 + YEAR + 10, 100 + 2 * YEAR), 100 + YEAR + 10);
        refused(&a, &too_soon, "below the rotation period");

        // A full sit-out is accepted.
        let mut b = InMemoryArchive::new();
        let (c3, c3_priv) = community(&mut b, "C", ts(10));
        let build3 = |a: &mut InMemoryArchive, from: i64, until: i64| {
            let reg = Signed::build(
                MediatorRegistry {
                    nominating_community: c3.clone(),
                    entries: vec![MediatorEntry {
                        mediator_did: mdid.clone(),
                        mediator_pubkey: mpub.clone(),
                        encryption_pubkey: epub.clone(),
                        valid_from: ts(from),
                        valid_until: Some(ts(until)),
                        trained_by: None,
                    }],
                    issued_at: ts(from),
                },
                &[(c3.as_str(), &c3_priv)],
            )
            .unwrap();
            a.append(ArchiveEntry::MediatorRegistryUpdated(reg)).unwrap()
        };
        build3(&mut b, 100, 100 + YEAR);
        let after = build3(
            &mut b,
            100 + YEAR + DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
            100 + 2 * YEAR + DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
        );
        ok(&b, &after);
    }
}

/// §4.2 — the three records §4 specifies and the tier did not carry: a recusal a party
/// can actually request, a protecting power, and the training lineage behind a
/// mediator.
mod recusal_and_protection {
    use super::*;
    use kc_mediation::{ProtectingPower, RecusalAnswer, RecusalDecision, RecusalRequest};

    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }
    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect("expected authorized");
    }

    fn request(
        a: &mut InMemoryArchive,
        dispute: Cid,
        by: &DidKey,
        by_priv: &PrivKey,
        mediator: &DidKey,
        grounds: &str,
        at: i64,
    ) -> Cid {
        let r = Signed::build(
            RecusalRequest {
                dispute_cid: dispute,
                requested_by: by.clone(),
                mediator: mediator.clone(),
                grounds: grounds.into(),
                requested_at: ts(at),
            },
            &[(by.as_str(), by_priv)],
        )
        .unwrap();
        a.append(ArchiveEntry::RecusalRequested(r)).unwrap()
    }

    fn answer(
        a: &mut InMemoryArchive,
        request_cid: Cid,
        ans: RecusalAnswer,
        deciders: &[(&DidKey, &PrivKey)],
        reasoning: &str,
        at: i64,
    ) -> Cid {
        let dids: Vec<DidKey> = deciders.iter().map(|(d, _)| (*d).clone()).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            deciders.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let d = Signed::build(
            RecusalDecision {
                request_cid,
                answer: ans,
                deciders: dids,
                reasoning: reasoning.into(),
                decided_at: ts(at),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::RecusalDecided(d)).unwrap()
    }

    /// A party may ask, and be answered. Nothing here blocks the mediation.
    #[test]
    fn a_party_may_request_a_recusal_and_the_mediator_may_grant_it() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let req = request(&mut a, dispute, &ca, &ca_priv, &pool[0].0, "prior dealings", 150);
        ok(&a, &req);

        // The grant is the mediator's own act.
        let granted = answer(
            &mut a,
            req,
            RecusalAnswer::Granted,
            &[(&pool[0].0, &pool[0].1)],
            "",
            160,
        );
        ok(&a, &granted);

        // The dispute still decides: a request neither blocks nor unseats by itself.
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let dec = decide(&mut a, dispute, &panel, ts(200));
        ok(&a, &dec);
    }

    /// Only a party may ask, only about a seated mediator, and only with grounds.
    #[test]
    fn a_recusal_request_is_a_party_s_act_about_a_seated_mediator() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let (cc, cc_priv) = community(&mut a, "Stranger", ts(10));

        let outsider = request(&mut a, dispute, &cc, &cc_priv, &pool[0].0, "meddling", 150);
        refused(&a, &outsider, "is not a party to this dispute");

        let (_p, pub_) = keys::generate(Alg::Ed25519).unwrap();
        let unseated = DidKey::from_pubkey(&pub_).unwrap();
        let nobody = request(&mut a, dispute, &ca, &ca_priv, &unseated, "grounds", 151);
        refused(&a, &nobody, "no seat to step down from");

        let silent = request(&mut a, dispute, &ca, &ca_priv, &pool[0].0, "   ", 152);
        refused(&a, &silent, "must state its grounds");
    }

    /// The asymmetry that is the point of the record: a grant is the named mediator's
    /// own act, and a decline may never be signed by the mediator whose own recusal is
    /// in question.
    #[test]
    fn a_mediator_may_not_answer_the_request_for_their_own_recusal() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let req = request(&mut a, dispute, &ca, &ca_priv, &pool[0].0, "prior dealings", 150);

        // Declining one's own recusal.
        let self_declined = answer(
            &mut a,
            req,
            RecusalAnswer::Declined,
            &[(&pool[0].0, &pool[0].1)],
            "I am fine",
            160,
        );
        refused(&a, &self_declined, "may not answer the request for their own recusal");

        // Someone else granting it for them.
        let other_granted = answer(
            &mut a,
            req,
            RecusalAnswer::Granted,
            &[(&pool[1].0, &pool[1].1)],
            "",
            161,
        );
        refused(&a, &other_granted, "granted by exactly the mediator stepping down");

        // A decline by the others, with reasons, is authorized.
        let declined = answer(
            &mut a,
            req,
            RecusalAnswer::Declined,
            &[(&pool[1].0, &pool[1].1), (&pool[2].0, &pool[2].1)],
            "the dealings named are not with this party",
            162,
        );
        ok(&a, &declined);
    }

    /// A refusal to step aside is the answer that owes an account.
    #[test]
    fn a_declined_recusal_must_state_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let req = request(&mut a, dispute, &ca, &ca_priv, &pool[0].0, "prior dealings", 150);
        let silent = answer(
            &mut a,
            req,
            RecusalAnswer::Declined,
            &[(&pool[1].0, &pool[1].1)],
            "  ",
            160,
        );
        refused(&a, &silent, "must state its reasoning");
    }

    fn protect(
        a: &mut InMemoryArchive,
        dispute: Cid,
        disputant: (&DidKey, &PrivKey),
        protector: (&DidKey, &PrivKey),
        at: i64,
    ) -> Cid {
        let pp = Signed::build(
            ProtectingPower {
                dispute_cid: dispute,
                disputant: disputant.0.clone(),
                protector: protector.0.clone(),
                statement: "we stand for them".into(),
                designated_at: ts(at),
            },
            &[
                (disputant.0.as_str(), disputant.1),
                (protector.0.as_str(), protector.1),
            ],
        )
        .unwrap();
        a.append(ArchiveEntry::ProtectingPowerDesignated(pp)).unwrap()
    }

    /// A neutral community may stand for a disputant, and gates nothing by doing so.
    #[test]
    fn a_neutral_community_may_stand_as_protecting_power() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        // A community that seats no mediators: neutral in both senses.
        let (cn, cn_priv) = community(&mut a, "Neutral", ts(10));
        let cid = protect(&mut a, dispute, (&ca, &ca_priv), (&cn, &cn_priv), 150);
        ok(&a, &cid);

        // "A witness with standing", not an arbitrator: the mediation decides exactly
        // as it would have.
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let dec = decide(&mut a, dispute, &panel, ts(200));
        ok(&a, &dec);
    }

    /// Neutrality is the whole of the rule: not a party, and not one of the judges.
    #[test]
    fn a_protecting_power_must_be_neutral_in_both_senses() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cb_priv, dispute, _pool) = mediation_fixture(&mut a);

        // The other party cannot stand for its opponent.
        let party = protect(&mut a, dispute, (&ca, &ca_priv), (&cb, &cb_priv), 150);
        refused(&a, &party, "cannot stand neutral in it");

        // Nor can a community that seats the mediators who will judge the dispute: the
        // witness and the judge would be one body. Built here rather than from the
        // shared fixture, which keeps its seating communities' keys to itself.
        let mut b = InMemoryArchive::new();
        let (pa, pa_priv) = community(&mut b, "A", ts(10));
        let (pb, _pb_priv) = community(&mut b, "B", ts(10));
        let (seating, seat_priv) = community(&mut b, "Seating", ts(10));
        add_mediators(&mut b, &seating, &seat_priv, 1, ts(10));
        let d2 = file_dispute(&mut b, &pa, &pa_priv, &pb, ts(100));
        let judge = protect(&mut b, d2, (&pa, &pa_priv), (&seating, &seat_priv), 151);
        refused(&b, &judge, "seats mediators and so cannot also stand");
    }

    /// Both sides consent: a duty is never conscripted, and a watcher is never imposed.
    #[test]
    fn a_protecting_power_needs_both_signatures() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, _pool) = mediation_fixture(&mut a);
        let (cn, _cn_priv) = community(&mut a, "Neutral", ts(10));
        // Only the disputant signs: the protector was volunteered by someone else.
        let pp = Signed::build(
            ProtectingPower {
                dispute_cid: dispute,
                disputant: ca.clone(),
                protector: cn.clone(),
                statement: "they will stand for us".into(),
                designated_at: ts(150),
            },
            &[(ca.as_str(), &ca_priv)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::ProtectingPowerDesignated(pp)).unwrap();
        refused(&a, &cid, "both the disputant and the protector must accept it");
    }
}

// --- Erosion remediation 5 and 6: the attested null and its two rules ---------
//
// `ActivityAttestation` and `ReexaminationConclusion` are records whose whole purpose
// is to say that nothing happened, so the rules that matter are the ones stopping
// them from becoming a way to *say* nothing happened without having looked. See
// `doc/architecture/erosion-legibility.md`, remediation items 5 and 6.

mod attested_nulls {
    use super::*;
    use kc_conduct::{ActivityAttestation, AttestedFunction};
    use kc_escalation::{
        DeliberationCycle, Designation, DesignationTier, DesignationUrgency,
        ReexaminationConclusion, ReexaminationOutcome,
    };
    use kc_observer::{ObserverEntry, ObserverRegistry};

    const YEAR: i64 = 365 * 24 * 3600;

    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }
    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect("expected authorized");
    }

    /// Seat one observer for `community` on a bounded term. `add_observers` is private
    /// to the §8 module, and this needs only the one seat.
    fn seat_observer(a: &mut InMemoryArchive, community: &DidKey, priv_: &PrivKey, at: i64) {
        let (_opriv, opub) = keys::generate(Alg::Ed25519).unwrap();
        let odid = DidKey::from_pubkey(&opub).unwrap();
        let reg = Signed::build(
            ObserverRegistry {
                nominating_community: community.clone(),
                entries: vec![ObserverEntry {
                    observer_did: odid,
                    observer_pubkey: opub,
                    declared_perspective: "outsider".into(),
                    valid_from: ts(at),
                    valid_until: Some(ts(at + 100 * 24 * 3600)),
                }],
                issued_at: ts(at),
            },
            &[(community.as_str(), priv_)],
        )
        .unwrap();
        a.append(ArchiveEntry::ObserverRegistryUpdated(reg)).unwrap();
    }

    // The erosion-5/6 cases turn on exactly these axes: which function was attested,
    // the window (which decides whether consecutive attestations leave a gap), whether
    // `acts` is empty (the attested null, which reads identically to neglect on any
    // act-triggered instrument), the statement, and the instant. Each test varies one.
    #[allow(clippy::too_many_arguments)]
    fn attestation(
        a: &mut InMemoryArchive,
        community: &DidKey,
        priv_: &PrivKey,
        function: AttestedFunction,
        window: (i64, i64),
        acts: Vec<Cid>,
        statement: &str,
        at: i64,
    ) -> Cid {
        let att = Signed::build(
            ActivityAttestation {
                function,
                attesting_community: community.clone(),
                window_start: ts(window.0),
                window_end: ts(window.1),
                acts,
                statement: statement.into(),
                attested_at: ts(at),
            },
            &[(community.as_str(), priv_)],
        )
        .unwrap();
        a.append(ArchiveEntry::ActivityAttested(att)).unwrap()
    }

    /// An expedited designation with a mandated re-examination date, plus the two
    /// deliberating communities the higher quorum needs.
    fn designated(
        a: &mut InMemoryArchive,
        due: i64,
        at: i64,
    ) -> DesignatedFixture {
        let (s, _sp) = community(a, "S", ts(1));
        let (p, pp) = community(a, "P", ts(1));
        let (q, qp) = community(a, "Q", ts(1));
        let (r, rp) = community(a, "R", ts(1));
        let dz = Signed::build(
            Designation {
                subject_community: s.clone(),
                tier: DesignationTier::Anathema,
                urgency: DesignationUrgency::Expedited,
                deliberators: vec![p.clone(), q.clone(), r.clone()],
                criteria: "the named pattern".into(),
                cited_acts: vec![Cid::of_bytes(b"act")],
                failed_restoration_cycles: vec![],
                deliberation_cycles: vec![DeliberationCycle {
                    cycle_index: 0,
                    representatives: vec![p.clone()],
                    opened_at: ts(at - 10),
                    concluded_at: ts(at - 5),
                    reasoning: "r".into(),
                }],
                reversal_conditions: Some("changed conduct".into()),
                reexamine_after: Some(ts(due)),
                decision_class: DecisionClass::SecondClass,
                designated_at: ts(at),
            },
            &[(p.as_str(), &pp), (q.as_str(), &qp), (r.as_str(), &rp)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::DesignationDecided(dz)).unwrap();
        (cid, s, (p, pp), (q, qp), (r, rp))
    }

    fn conclusion(
        a: &mut InMemoryArchive,
        designation_cid: Cid,
        examiners: &[(&DidKey, &PrivKey)],
        outcome: ReexaminationOutcome,
        reasoning: &str,
        next: Option<i64>,
        at: i64,
    ) -> Cid {
        let pairs: Vec<(&str, &PrivKey)> =
            examiners.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let rc = Signed::build(
            ReexaminationConclusion {
                designation_cid,
                outcome,
                examiners: examiners.iter().map(|(d, _)| (*d).clone()).collect(),
                reasoning: reasoning.into(),
                next_reexamine_after: next.map(ts),
                decision_class: DecisionClass::SecondClass,
                concluded_at: ts(at),
            },
            &pairs,
        )
        .unwrap();
        a.append(ArchiveEntry::ReexaminationConcluded(rc)).unwrap()
    }

    // --- the re-examination conclusion ---------------------------------------

    #[test]
    fn a_re_examination_is_recorded_and_renews_the_obligation() {
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Confirmed,
            "the pattern that named them has not changed",
            Some(1000 + 10 * YEAR),
            1010,
        );
        ok(&a, &cid);
    }

    #[test]
    fn a_re_examination_may_not_be_held_before_the_date_it_answers() {
        // The evasion this rule exists for: confirm at designation time and buy
        // permanent silence from the erosion lens.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Confirmed,
            "nothing has changed since ten minutes ago",
            Some(9000),
            110,
        );
        refused(&a, &cid, "before the mandated date");
    }

    #[test]
    fn a_confirmation_must_set_the_next_date() {
        // The other evasion: one confirmation discharging a generational obligation.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Confirmed,
            "it stands",
            None,
            1010,
        );
        refused(&a, &cid, "must set the next re-examination date");

        // And the next date must actually be in the future of the conclusion.
        let mut b = InMemoryArchive::new();
        let (bdz, _bs, (bp, bpp), (bq, bqp), _br) = designated(&mut b, 1000, 100);
        let bcid = conclusion(
            &mut b,
            bdz,
            &[(&bp, &bpp), (&bq, &bqp)],
            ReexaminationOutcome::Confirmed,
            "it stands",
            Some(900),
            1010,
        );
        refused(&b, &bcid, "is not after this conclusion");
    }

    #[test]
    fn a_re_examination_must_publish_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Confirmed,
            "   ",
            Some(9000),
            1010,
        );
        refused(&a, &cid, "must publish its reasoning");
    }

    #[test]
    fn reporting_a_reversal_needs_the_reversal_itself() {
        // One definition of "reversed", which the erosion audit credits the lens for.
        // A conclusion cannot become a second, cheaper way to lift a designation.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Reversed,
            "we consider it lifted",
            None,
            1010,
        );
        refused(&a, &cid, "no lifting record or re-entry answers it");
    }

    #[test]
    fn a_re_examination_carries_the_designation_quorum() {
        // Re-examining the severest act is the same weight of act as taking it, so it
        // inherits the second-class distinct-community floor rather than being a thing
        // one community can conclude alone.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), _q, _r) = designated(&mut a, 1000, 100);
        let cid = conclusion(
            &mut a,
            dz,
            &[(&p, &pp)],
            ReexaminationOutcome::Confirmed,
            "it stands",
            Some(9000),
            1010,
        );
        refused(&a, &cid, "second-class decision requires");
    }

    // --- the gate: erosion remediation's one refusal -------------------------

    #[test]
    fn a_lapsed_designation_may_not_be_cited_by_a_new_one() {
        // The one refusal this slice carries. The most severe act the compact can take
        // may not rest on an obligation the compact has itself let lapse.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), (r, rp)) = designated(&mut a, 1000, 100);
        let (s2, _s2p) = community(&mut a, "S2", ts(1));
        let cited = Signed::build(
            Designation {
                subject_community: s2.clone(),
                tier: DesignationTier::Anathema,
                urgency: DesignationUrgency::Expedited,
                deliberators: vec![p.clone(), q.clone(), r.clone()],
                criteria: "consorting with the designated".into(),
                cited_acts: vec![dz],
                failed_restoration_cycles: vec![],
                deliberation_cycles: vec![DeliberationCycle {
                    cycle_index: 0,
                    representatives: vec![p.clone()],
                    opened_at: ts(4990),
                    concluded_at: ts(4995),
                    reasoning: "r".into(),
                }],
                reversal_conditions: Some("changed conduct".into()),
                reexamine_after: Some(ts(20_000)),
                decision_class: DecisionClass::SecondClass,
                designated_at: ts(5000),
            },
            &[(p.as_str(), &pp), (q.as_str(), &qp), (r.as_str(), &rp)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::DesignationDecided(cited)).unwrap();
        refused(&a, &cid, "a further escalation may not rest on a lapsed obligation");
    }

    #[test]
    fn a_designation_cited_before_its_date_passes_the_gate() {
        // Negative control, predicted first: the gate must fire on *lapse*, not on
        // citation. A designation still inside its window is ordinary evidence.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), (r, rp)) = designated(&mut a, 100_000, 100);
        let (s2, _s2p) = community(&mut a, "S2", ts(1));
        let cited = Signed::build(
            Designation {
                subject_community: s2.clone(),
                tier: DesignationTier::Anathema,
                urgency: DesignationUrgency::Expedited,
                deliberators: vec![p.clone(), q.clone(), r.clone()],
                criteria: "consorting with the designated".into(),
                cited_acts: vec![dz],
                failed_restoration_cycles: vec![],
                deliberation_cycles: vec![DeliberationCycle {
                    cycle_index: 0,
                    representatives: vec![p.clone()],
                    opened_at: ts(4990),
                    concluded_at: ts(4995),
                    reasoning: "r".into(),
                }],
                reversal_conditions: Some("changed conduct".into()),
                reexamine_after: Some(ts(200_000)),
                decision_class: DecisionClass::SecondClass,
                designated_at: ts(5000),
            },
            &[(p.as_str(), &pp), (q.as_str(), &qp), (r.as_str(), &rp)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::DesignationDecided(cited)).unwrap();
        ok(&a, &cid);
    }

    #[test]
    fn a_re_examined_designation_may_be_cited_again() {
        // The gate is about follow-through, not about age: once the obligation is
        // answered, the citation is good again. Without this the fix would make a
        // diligently-maintained designation permanently uncitable.
        let mut a = InMemoryArchive::new();
        let (dz, _s, (p, pp), (q, qp), (r, rp)) = designated(&mut a, 1000, 100);
        conclusion(
            &mut a,
            dz,
            &[(&p, &pp), (&q, &qp)],
            ReexaminationOutcome::Confirmed,
            "the pattern has not changed",
            Some(100_000),
            1010,
        );
        let (s2, _s2p) = community(&mut a, "S2", ts(1));
        let cited = Signed::build(
            Designation {
                subject_community: s2.clone(),
                tier: DesignationTier::Anathema,
                urgency: DesignationUrgency::Expedited,
                deliberators: vec![p.clone(), q.clone(), r.clone()],
                criteria: "consorting with the designated".into(),
                cited_acts: vec![dz],
                failed_restoration_cycles: vec![],
                deliberation_cycles: vec![DeliberationCycle {
                    cycle_index: 0,
                    representatives: vec![p.clone()],
                    opened_at: ts(4990),
                    concluded_at: ts(4995),
                    reasoning: "r".into(),
                }],
                reversal_conditions: Some("changed conduct".into()),
                reexamine_after: Some(ts(200_000)),
                decision_class: DecisionClass::SecondClass,
                designated_at: ts(5000),
            },
            &[(p.as_str(), &pp), (q.as_str(), &qp), (r.as_str(), &rp)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::DesignationDecided(cited)).unwrap();
        ok(&a, &cid);
    }

    // --- the activity attestation -------------------------------------------

    #[test]
    fn a_community_holding_seats_may_attest_a_null_window() {
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        add_mediators(&mut a, &c, &cp, 1, ts(100));
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Mediation,
            (100, 200),
            vec![],
            "no disputes were filed in this window",
            200,
        );
        ok(&a, &cid);
    }

    #[test]
    fn a_community_seating_nobody_has_no_account_to_give() {
        // The obligation belongs to whoever holds the seats. A community that seated
        // nobody cannot manufacture a clean record of a function it does not perform.
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Mediation,
            (100, 200),
            vec![],
            "nothing to report",
            200,
        );
        refused(&a, &cid, "has no account to give");
    }

    #[test]
    fn an_empty_attestation_must_still_state_its_account() {
        // A null owes an explanation, because "no disputes were filed" and "we stopped
        // convening" are both nulls and are not the same thing.
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        add_mediators(&mut a, &c, &cp, 1, ts(100));
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Mediation,
            (100, 200),
            vec![],
            "  ",
            200,
        );
        refused(&a, &cid, "must state its account");
    }

    #[test]
    fn a_window_must_be_a_window() {
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        add_mediators(&mut a, &c, &cp, 1, ts(100));
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Mediation,
            (200, 200),
            vec![],
            "an instant",
            200,
        );
        refused(&a, &cid, "at or before its start");
    }

    #[test]
    fn a_cited_act_must_fall_inside_the_window() {
        // An attestation is evidence about a window, so it may not borrow an act from
        // outside the window it is offered for.
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        add_mediators(&mut a, &c, &cp, 1, ts(100));
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let dispute = Signed::build(
            DisputeRecord {
                complainant: x.clone(),
                respondent: y.clone(),
                rung: LadderRung::MediatedArbitration,
                summary: "s".into(),
                references: vec![],
                filed_at: ts(5000),
            },
            &[(x.as_str(), &xp)],
        )
        .unwrap();
        let dispute_cid = a.append(ArchiveEntry::DisputeFiled(dispute)).unwrap();
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Mediation,
            (100, 200),
            vec![dispute_cid],
            "we handled this",
            200,
        );
        refused(&a, &cid, "falls outside the window");
    }

    #[test]
    fn a_cited_act_must_belong_to_the_attested_function() {
        // An observation attestation may not cite a mediation act: the two functions
        // are separately accountable, which is the whole reason the enum exists.
        let mut a = InMemoryArchive::new();
        let (c, cp) = community(&mut a, "C", ts(10));
        seat_observer(&mut a, &c, &cp, 100);
        add_mediators(&mut a, &c, &cp, 1, ts(100));
        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, _yp) = community(&mut a, "Y", ts(10));
        let dispute = Signed::build(
            DisputeRecord {
                complainant: x.clone(),
                respondent: y.clone(),
                rung: LadderRung::MediatedArbitration,
                summary: "s".into(),
                references: vec![],
                filed_at: ts(150),
            },
            &[(x.as_str(), &xp)],
        )
        .unwrap();
        let dispute_cid = a.append(ArchiveEntry::DisputeFiled(dispute)).unwrap();
        let cid = attestation(
            &mut a,
            &c,
            &cp,
            AttestedFunction::Observation,
            (100, 200),
            vec![dispute_cid],
            "we observed this",
            200,
        );
        refused(&a, &cid, "is not a Observation act");
    }
}

/// §4.1 — consent to the panel as constituted.
///
/// The measurement this part began from: a panel recorded that the parties **settled** a
/// dispute neither party had ever agreed to settle, and it was authorized. Consent
/// existed in the tier only where it was structural (a protecting power is co-signed)
/// or where it was refused (a recusal request, answered); the ordinary case — this
/// panel, these mediators, may decide my dispute — was inferred from a party having
/// filed an argument, which is a different act.
///
/// Consent gates the settlement and nothing else, deliberately: a consent precondition
/// on deciding at all would hand either party a veto over its own dispute.
mod panel_consent {
    use super::*;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect("expected authorized");
    }
    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }

    /// THE MEASUREMENT, now the rule: a settlement neither party consented to is refused.
    #[test]
    fn a_settlement_no_party_consented_to_is_refused() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let args = panel_arguments(&mut a, dispute, &panel, ts(200));
        let dec = decide_citing_outcome(
            &mut a, dispute, &panel, args, DecisionOutcome::Settled, ts(200),
        );
        refused(&a, &dec, "is not a settlement");

        // NEGATIVE CONTROL: the identical decision, with both consents on the record,
        // is authorized. The consent is what changed, not the panel or the arguments.
        let mut b = InMemoryArchive::new();
        let (ca2, ca2_priv, cb2, cb2_priv, dispute2, pool2) = mediation_fixture(&mut b);
        let panel2: Vec<(&DidKey, &PrivKey)> = pool2.iter().map(|(d, p, _)| (d, p)).collect();
        let dec2 = decide_settled(
            &mut b,
            dispute2,
            (&ca2, &ca2_priv, &cb2, &cb2_priv),
            &panel2,
            ts(200),
        );
        ok(&b, &dec2);
        let _ = (ca, ca_priv, cb, cb_priv);
    }

    /// One party's consent is not both parties'. A settlement is a claim about the two
    /// of them.
    #[test]
    fn one_partys_consent_does_not_settle_for_the_other() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        consent(&mut a, dispute, &ca, &ca_priv, &panel, ConsentAnswer::Given, ts(150));
        let args = panel_arguments(&mut a, dispute, &panel, ts(200));
        let dec = decide_citing_outcome(
            &mut a, dispute, &panel, args, DecisionOutcome::Settled, ts(200),
        );
        let err = authorize(&a, a.get(&dec).unwrap(), DEFAULT_QUORUM_THRESHOLD, DEFAULT_PANEL_MIN)
            .expect_err("the respondent never consented");
        assert!(
            err.to_string().contains(cb.as_str()),
            "the refusal must name the party whose consent is missing, got: {err}"
        );
    }

    /// **Consent does not carry across a change to the panel.** A party that accepted
    /// three mediators has not accepted a fourth, so a settlement recorded by a panel
    /// other than the consented one has no consent behind it.
    #[test]
    fn consent_does_not_carry_to_a_differently_constituted_panel() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let (cf, cf_priv) = community(&mut a, "F", ts(10));
        let extra = add_mediators(&mut a, &cf, &cf_priv, 1, ts(10));

        let three: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        consent(&mut a, dispute, &ca, &ca_priv, &three, ConsentAnswer::Given, ts(150));
        consent(&mut a, dispute, &cb, &cb_priv, &three, ConsentAnswer::Given, ts(150));

        // A fourth mediator joins, and that panel records the settlement.
        let mut four = three.clone();
        four.push((&extra[0].0, &extra[0].1));
        let args = panel_arguments(&mut a, dispute, &four, ts(200));
        let dec = decide_citing_outcome(
            &mut a, dispute, &four, args, DecisionOutcome::Settled, ts(200),
        );
        refused(&a, &dec, "is not a settlement");

        // NEGATIVE CONTROL: the same four-mediator panel, consented to as four, settles.
        let dec_ok = decide_settled(
            &mut a, dispute, (&ca, &ca_priv, &cb, &cb_priv), &four, ts(300),
        );
        ok(&a, &dec_ok);
    }

    /// Consent is withdrawable: the party's latest answer on that panel governs.
    #[test]
    fn a_later_withholding_supersedes_an_earlier_consent() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        consent(&mut a, dispute, &ca, &ca_priv, &panel, ConsentAnswer::Given, ts(150));
        consent(&mut a, dispute, &cb, &cb_priv, &panel, ConsentAnswer::Given, ts(150));
        consent(&mut a, dispute, &cb, &cb_priv, &panel, ConsentAnswer::Withheld, ts(160));
        let args = panel_arguments(&mut a, dispute, &panel, ts(200));
        let dec = decide_citing_outcome(
            &mut a, dispute, &panel, args, DecisionOutcome::Settled, ts(200),
        );
        refused(&a, &dec, "is not a settlement");
    }

    /// **Nothing but a settlement waits on consent.** A party that refuses to settle
    /// does not thereby block its own dispute: the panel upholds, dismisses or refers.
    #[test]
    fn withheld_consent_does_not_block_the_other_three_outcomes() {
        for outcome in [
            DecisionOutcome::Upheld,
            DecisionOutcome::Dismissed,
            DecisionOutcome::Referred,
        ] {
            let mut a = InMemoryArchive::new();
            let (ca, ca_priv, cb, cb_priv, dispute, pool) = mediation_fixture(&mut a);
            let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
            consent(&mut a, dispute, &ca, &ca_priv, &panel, ConsentAnswer::Withheld, ts(150));
            consent(&mut a, dispute, &cb, &cb_priv, &panel, ConsentAnswer::Withheld, ts(150));
            let args = panel_arguments(&mut a, dispute, &panel, ts(200));
            let dec = decide_citing_outcome(&mut a, dispute, &panel, args, outcome, ts(200));
            ok(&a, &dec);
        }
    }

    /// A consent is the party's own act. Nobody consents on another's behalf, and a
    /// non-party has no consent to give at all.
    #[test]
    fn only_a_party_speaking_for_itself_may_consent() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let (outsider, outsider_priv) = community(&mut a, "Outsider", ts(10));

        // A non-party purporting to consent.
        let cid = consent(
            &mut a, dispute, &outsider, &outsider_priv, &panel, ConsentAnswer::Given, ts(150),
        );
        refused(&a, &cid, "is not a party to this dispute");

        // A party's consent signed by somebody else. The payload names the party; the
        // envelope does not carry a key that speaks for it.
        let forged = Signed::build(
            PanelConsent {
                dispute_cid: dispute,
                party: ca.clone(),
                panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
                answer: ConsentAnswer::Given,
                statement: "on their behalf".into(),
                given_at: ts(150),
            },
            &[(outsider.as_str(), &outsider_priv)],
        )
        .unwrap();
        let cid = a
            .append(ArchiveEntry::PanelConsentRecorded(forged))
            .unwrap();
        refused(&a, &cid, "nobody consents on another's behalf");

        // NEGATIVE CONTROL: the party signing for itself is authorized.
        let mine = consent(
            &mut a, dispute, &ca, &ca_priv, &panel, ConsentAnswer::Given, ts(150),
        );
        ok(&a, &mine);
    }

    /// Consent may not be recorded to a panel that could not sit — a party community's
    /// own mediator, or somebody who is not a registered mediator at all. Otherwise a
    /// party could consent to a fiction and have it counted as the real panel's.
    #[test]
    fn consent_to_a_panel_that_could_not_sit_is_refused() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        // The complainant's own mediator.
        let own = add_mediators(&mut a, &ca, &ca_priv, 1, ts(10));
        let mut panel: Vec<(&DidKey, &PrivKey)> =
            pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        panel.push((&own[0].0, &own[0].1));
        let cid = consent(
            &mut a, dispute, &ca, &ca_priv, &panel, ConsentAnswer::Given, ts(150),
        );
        refused(&a, &cid, "non-overlap violated");

        // Somebody who holds no mediator seat.
        let (stray_did, stray_priv) = {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            (DidKey::from_pubkey(&pk).unwrap(), sk)
        };
        let mut panel2: Vec<(&DidKey, &PrivKey)> =
            pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        panel2.push((&stray_did, &stray_priv));
        let cid = consent(
            &mut a, dispute, &ca, &ca_priv, &panel2, ConsentAnswer::Given, ts(150),
        );
        refused(&a, &cid, "is not an active registered mediator");
    }

    /// Withholding is the answer that owes an account.
    #[test]
    fn withheld_consent_must_state_something() {
        let mut a = InMemoryArchive::new();
        let (ca, ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let c = Signed::build(
            PanelConsent {
                dispute_cid: dispute,
                party: ca.clone(),
                panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
                answer: ConsentAnswer::Withheld,
                statement: "   ".into(),
                given_at: ts(150),
            },
            &[(ca.as_str(), &ca_priv)],
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::PanelConsentRecorded(c)).unwrap();
        refused(&a, &cid, "owes an account");
    }
}

/// §4.1 — the mediator pool's authority over the structured-skepticism protocol.
///
/// The measurement this part began from, in two directions. **The protocol had no teeth
/// against a manufacturer:** whether an event qualified was four booleans the
/// *originating community* set on its own report, so a manufacturer that declared its
/// own event mundane was never held, and a full non-party mediator pool sitting in the
/// same archive could do nothing about it. **And a hold answered to nobody:** the
/// mirror case, a community placing its own event beyond action, with no record any
/// reviewer could author. §4 assigns the invocation to the pool and requires the pool
/// to review it against pretextual use; neither was a record.
mod skepticism_invocation {
    use super::*;
    use kc_provenance::{
        EventActionInvoked, EventCorroboration, EventReport, ReviewAnswer, SkepticismCriterion,
        SkepticismInvocation, SkepticismReview,
    };

    use crate::authority::{
        MAX_INVOKED_SKEPTICISM_HOLD_SECONDS, MIN_INVOKED_SKEPTICISM_HOLD_SECONDS,
    };

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const DAY: i64 = 24 * 3600;

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), Q, P).expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }

    /// The originator, three non-party communities each seating one mediator, and one
    /// event. `declared` is what the originator says about its own claim; the facts of
    /// the event are the same either way.
    fn fixture(
        a: &mut InMemoryArchive,
        declared: bool,
    ) -> (DidKey, PrivKey, Cid, Vec<(DidKey, PrivKey, PrivKey)>) {
        let (r, rp) = community(a, "Originator", ts(10));
        let (cc, cc_priv) = community(a, "C", ts(10));
        let (cd, cd_priv) = community(a, "D", ts(10));
        let (ce, ce_priv) = community(a, "E", ts(10));
        let mut pool = vec![];
        pool.extend(add_mediators(a, &cc, &cc_priv, 1, ts(10)));
        pool.extend(add_mediators(a, &cd, &cd_priv, 1, ts(10)));
        pool.extend(add_mediators(a, &ce, &ce_priv, 1, ts(10)));
        let ev = report(a, &r, &rp, declared, ts(1000));
        (r, rp, ev, pool)
    }

    fn report(
        a: &mut InMemoryArchive,
        originator: &DidKey,
        root_priv: &PrivKey,
        declared_qualifying: bool,
        at: Timestamp,
    ) -> Cid {
        let rec = EventReport {
            originator: originator.clone(),
            summary: "a sudden, faction-convenient omen witnessed by an interested party".into(),
            // The facts of this event meet every criterion the doctrine names. What the
            // record carries is what its originator chose to declare about them.
            high_interpretive_charge: declared_qualifying,
            sudden_onset: declared_qualifying,
            originator_holds_stake: declared_qualifying,
            aligned_faction: None,
            declared_incentives: "none that we see".into(),
            information_history: "first-hand".into(),
            disclosed_interests: vec![],
            onset_at: at,
            reported_at: at,
        };
        let signed = Signed::build(rec, &[(originator.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::EventReported(signed)).unwrap()
    }

    fn act(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        acting: &DidKey,
        root_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let rec = EventActionInvoked {
            event_cid: *event_cid,
            acting_community: acting.clone(),
            action: "designate the rival community".into(),
            invoked_at: at,
        };
        let signed = Signed::build(rec, &[(acting.as_str(), root_priv)]).unwrap();
        a.append(ArchiveEntry::EventActionInvoked(signed)).unwrap()
    }

    fn invoke_with(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        panel: &[(&DidKey, &PrivKey)],
        criteria: Vec<SkepticismCriterion>,
        grounds: &str,
        hold_seconds: i64,
        at: Timestamp,
    ) -> Cid {
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let rec = SkepticismInvocation {
            event_cid: *event_cid,
            panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            criteria,
            grounds: grounds.into(),
            hold_seconds,
            invoked_at: at,
        };
        let signed = Signed::build(rec, &pairs).unwrap();
        a.append(ArchiveEntry::SkepticismInvoked(signed)).unwrap()
    }

    fn invoke(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        panel: &[(&DidKey, &PrivKey)],
        hold_seconds: i64,
        at: Timestamp,
    ) -> Cid {
        invoke_with(
            a,
            event_cid,
            panel,
            vec![
                SkepticismCriterion::SuddenOnset,
                SkepticismCriterion::OriginatorHoldsStake,
            ],
            "onset within the hour of the allocation vote, and the witness stands to gain",
            hold_seconds,
            at,
        )
    }

    fn review(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        panel: &[(&DidKey, &PrivKey)],
        answer: ReviewAnswer,
        reasoning: &str,
        at: Timestamp,
    ) -> Cid {
        let pairs: Vec<(&str, &PrivKey)> = panel.iter().map(|(d, p)| (d.as_str(), *p)).collect();
        let rec = SkepticismReview {
            event_cid: *event_cid,
            reviewers: panel.iter().map(|(d, _)| (*d).clone()).collect(),
            answer,
            reasoning: reasoning.into(),
            reviewed_at: at,
        };
        let signed = Signed::build(rec, &pairs).unwrap();
        a.append(ArchiveEntry::SkepticismReviewed(signed)).unwrap()
    }

    fn corroborate(
        a: &mut InMemoryArchive,
        event_cid: &Cid,
        who: &DidKey,
        who_priv: &PrivKey,
        at: Timestamp,
    ) -> Cid {
        let rec = EventCorroboration {
            event_cid: *event_cid,
            corroborating_community: who.clone(),
            translation: None,
            corroborated_at: at,
        };
        let signed = Signed::build(rec, &[(who.as_str(), who_priv)]).unwrap();
        a.append(ArchiveEntry::EventCorroborated(signed)).unwrap()
    }

    /// THE MEASUREMENT, now the rule. An originator declares its own event mundane and
    /// the self-declared hold does not reach it — but the pool's invocation does, and
    /// the action premised on it is refused.
    #[test]
    fn the_pool_can_hold_an_event_its_originator_declared_mundane() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, false);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();

        // Without an invocation, this is exactly what used to be authorized.
        let before = act(&mut a, &ev, &r, &rp, ts(1000 + 3600));
        ok(&a, &before);

        let inv = invoke(&mut a, &ev, &panel, 14 * DAY, ts(1000 + 60));
        ok(&a, &inv);

        let after = act(&mut a, &ev, &r, &rp, ts(1000 + 2 * 3600));
        refused(&a, &after, "structured-skepticism hold");
    }

    /// THE OTHER HALF, now the rule: a hold placed by the originator's own declaration
    /// is reviewable, and a `Pretextual` finding releases it. The review is against the
    /// **hold**, not against an invocation record, which is what lets it reach the
    /// self-declared case at all.
    #[test]
    fn a_pretextual_review_releases_a_self_declared_hold() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, true);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();

        let held = act(&mut a, &ev, &r, &rp, ts(1000 + 3600));
        refused(&a, &held, "structured-skepticism hold");

        let rev = review(
            &mut a,
            &ev,
            &panel,
            ReviewAnswer::Pretextual,
            "the originator flagged its own rescue request to keep the compact from answering it",
            ts(1000 + 1800),
        );
        ok(&a, &rev);

        let released = act(&mut a, &ev, &r, &rp, ts(1000 + 7200));
        ok(&a, &released);
    }

    /// An `Upheld` review is an audit record and nothing more: the hold it examined
    /// stands exactly as it did.
    #[test]
    fn an_upheld_review_leaves_the_hold_standing() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, true);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let rev = review(
            &mut a,
            &ev,
            &panel,
            ReviewAnswer::Upheld,
            "the criteria are met on the record and the invocation is not a tactic",
            ts(1000 + 1800),
        );
        ok(&a, &rev);
        let still_held = act(&mut a, &ev, &r, &rp, ts(1000 + 7200));
        refused(&a, &still_held, "structured-skepticism hold");
    }

    /// **A panel may not review its own invocation.** Without this the review record
    /// would exist and audit nothing: the invoking panel would uphold itself, or
    /// release its own hold when it suited.
    #[test]
    fn the_invoking_panel_may_not_review_its_own_invocation() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let (cf, cf_priv) = community(&mut a, "F", ts(10));
        let (cg, cg_priv) = community(&mut a, "G", ts(10));
        let (ch, ch_priv) = community(&mut a, "H", ts(10));
        let mut others = vec![];
        others.extend(add_mediators(&mut a, &cf, &cf_priv, 1, ts(10)));
        others.extend(add_mediators(&mut a, &cg, &cg_priv, 1, ts(10)));
        others.extend(add_mediators(&mut a, &ch, &ch_priv, 1, ts(10)));

        let invoking: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        invoke(&mut a, &ev, &invoking, 14 * DAY, ts(1000 + 60));

        let self_review = review(
            &mut a,
            &ev,
            &invoking,
            ReviewAnswer::Upheld,
            "we reviewed ourselves and found ourselves sound",
            ts(1000 + 120),
        );
        refused(&a, &self_review, "co-signed the invocation under review");

        // Even one invoker sitting on the reviewing panel is enough to disqualify it.
        let mixed: Vec<(&DidKey, &PrivKey)> = vec![
            (&pool[0].0, &pool[0].1),
            (&others[0].0, &others[0].1),
            (&others[1].0, &others[1].1),
        ];
        let mixed_review = review(
            &mut a,
            &ev,
            &mixed,
            ReviewAnswer::Upheld,
            "one of us invoked it",
            ts(1000 + 130),
        );
        refused(&a, &mixed_review, "co-signed the invocation under review");

        // NEGATIVE CONTROL: an entirely separate panel reviews it, and is authorized.
        let independent: Vec<(&DidKey, &PrivKey)> =
            others.iter().map(|(d, p, _)| (d, p)).collect();
        let clean = review(
            &mut a,
            &ev,
            &independent,
            ReviewAnswer::Upheld,
            "the criteria are met and the invocation is not a tactic",
            ts(1000 + 140),
        );
        ok(&a, &clean);
    }

    /// The originating community may neither invoke the pool's hold on its own event
    /// (that is what its own declaration already is) nor sit in review of a hold it
    /// benefits from releasing.
    #[test]
    fn the_originators_own_mediators_may_neither_invoke_nor_review() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, true);
        let own = add_mediators(&mut a, &r, &rp, 1, ts(10));

        let mut panel: Vec<(&DidKey, &PrivKey)> =
            pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        panel.push((&own[0].0, &own[0].1));

        let inv = invoke(&mut a, &ev, &panel, 14 * DAY, ts(1000 + 60));
        refused(&a, &inv, "non-overlap violated");

        // NEGATIVE CONTROL: the same three-mediator panel without the originator's own.
        let clean: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let inv_ok = invoke(&mut a, &ev, &clean, 14 * DAY, ts(1000 + 80));
        ok(&a, &inv_ok);

        // The review half, in its own archive: signing an invocation disqualifies a
        // mediator from reviewing it, which would otherwise mask the rule under test.
        let mut b = InMemoryArchive::new();
        let (r2, rp2, ev2, pool2) = fixture(&mut b, true);
        let own2 = add_mediators(&mut b, &r2, &rp2, 1, ts(10));
        let mut panel2: Vec<(&DidKey, &PrivKey)> =
            pool2.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        panel2.push((&own2[0].0, &own2[0].1));
        let rev = review(
            &mut b,
            &ev2,
            &panel2,
            ReviewAnswer::Pretextual,
            "release our own event",
            ts(1000 + 70),
        );
        refused(&b, &rev, "non-overlap violated");
    }

    /// The invoking panel is derived from the co-signers, so a record cannot name one
    /// panel while another signs — the rule `MediationDecision.panel` already carries.
    #[test]
    fn an_invocations_panel_must_be_its_co_signers() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let (cf, cf_priv) = community(&mut a, "F", ts(10));
        let stranger = add_mediators(&mut a, &cf, &cf_priv, 1, ts(10));

        let signers: Vec<(&str, &PrivKey)> =
            pool.iter().map(|(d, p, _)| (d.as_str(), p)).collect();
        let rec = SkepticismInvocation {
            event_cid: ev,
            // Names a mediator who did not sign.
            panel: vec![
                pool[0].0.clone(),
                pool[1].0.clone(),
                stranger[0].0.clone(),
            ],
            criteria: vec![SkepticismCriterion::SuddenOnset],
            grounds: "onset within the hour of the vote".into(),
            hold_seconds: 14 * DAY,
            invoked_at: ts(1060),
        };
        let signed = Signed::build(rec, &signers).unwrap();
        let cid = a.append(ArchiveEntry::SkepticismInvoked(signed)).unwrap();
        refused(&a, &cid, "must be exactly its co-signers");
    }

    /// An invocation must document criteria and grounds, and a review must give
    /// reasoning on either answer.
    #[test]
    fn an_invocation_documents_its_criteria_and_a_review_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();

        let no_criteria = invoke_with(&mut a, &ev, &panel, vec![], "it feels wrong", 14 * DAY, ts(1060));
        refused(&a, &no_criteria, "must name the criteria");

        let no_grounds = invoke_with(
            &mut a,
            &ev,
            &panel,
            vec![SkepticismCriterion::SuddenOnset],
            "   ",
            14 * DAY,
            ts(1061),
        );
        refused(&a, &no_grounds, "document its grounds specifically");

        // A review that strikes a hold down without saying why, and one that sustains
        // it without saying why, are refused alike.
        for answer in [ReviewAnswer::Pretextual, ReviewAnswer::Upheld] {
            let silent = review(&mut a, &ev, &panel, answer, "  ", ts(1062));
            refused(&a, &silent, "must state its reasoning on either answer");
        }
    }

    /// The hold a pool imposes is bounded at both ends. A nominal hold is no delay, and
    /// an unbounded one is the suppression the review exists to catch — caught here by
    /// a rule rather than by somebody noticing.
    #[test]
    fn an_invoked_hold_is_bounded_at_both_ends() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();

        let nominal = invoke(&mut a, &ev, &panel, MIN_INVOKED_SKEPTICISM_HOLD_SECONDS - 1, ts(1060));
        refused(&a, &nominal, "outside the range the protocol calibrates to");

        let forever = invoke(&mut a, &ev, &panel, MAX_INVOKED_SKEPTICISM_HOLD_SECONDS + 1, ts(1061));
        refused(&a, &forever, "outside the range the protocol calibrates to");

        // NEGATIVE CONTROLS: both endpoints themselves are inside the range.
        let floor = invoke(&mut a, &ev, &panel, MIN_INVOKED_SKEPTICISM_HOLD_SECONDS, ts(1062));
        ok(&a, &floor);
        let ceiling = invoke(&mut a, &ev, &panel, MAX_INVOKED_SKEPTICISM_HOLD_SECONDS, ts(1063));
        ok(&a, &ceiling);
    }

    /// **The interval scales with interpretive charge.** A pool that weighs an event as
    /// high-charge and imposes months holds it past the seven-day default; the window is
    /// the longer of the two, never the shorter.
    #[test]
    fn an_invoked_hold_extends_the_default_window_and_never_shortens_it() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, true);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        invoke(&mut a, &ev, &panel, 90 * DAY, ts(1000 + 60));

        // Past the seven-day default, still inside the pool's ninety days.
        let inside = act(&mut a, &ev, &r, &rp, ts(1000 + 30 * DAY));
        refused(&a, &inside, "structured-skepticism hold");
        // Past the pool's window.
        let outside = act(&mut a, &ev, &r, &rp, ts(1000 + 91 * DAY));
        ok(&a, &outside);

        // A shorter invoked hold on a self-declared event cannot cut the default short.
        let mut b = InMemoryArchive::new();
        let (r2, rp2, ev2, pool2) = fixture(&mut b, true);
        let panel2: Vec<(&DidKey, &PrivKey)> = pool2.iter().map(|(d, p, _)| (d, p)).collect();
        invoke(&mut b, &ev2, &panel2, MIN_INVOKED_SKEPTICISM_HOLD_SECONDS, ts(1000 + 60));
        let still = act(&mut b, &ev2, &r2, &rp2, ts(1000 + 2 * DAY));
        refused(&b, &still, "structured-skepticism hold");
    }

    /// Corroboration still releases an invoked hold. The pool's invocation imposes the
    /// delay the protocol asks for; it is not a licence to hold an event the ambient
    /// community has independently confirmed.
    #[test]
    fn corroboration_releases_an_invoked_hold_as_it_releases_a_declared_one() {
        let mut a = InMemoryArchive::new();
        let (r, rp, ev, pool) = fixture(&mut a, false);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        invoke(&mut a, &ev, &panel, 90 * DAY, ts(1000 + 60));

        let (x, xp) = community(&mut a, "X", ts(10));
        let (y, yp) = community(&mut a, "Y", ts(10));
        corroborate(&mut a, &ev, &x, &xp, ts(1100));
        corroborate(&mut a, &ev, &y, &yp, ts(1200));

        let action = act(&mut a, &ev, &r, &rp, ts(1300));
        ok(&a, &action);
    }

    /// An invocation must reference an event that exists, and so must a review.
    #[test]
    fn an_invocation_and_a_review_must_reference_a_reported_event() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        // Point at a mediator registry instead of an event.
        let not_an_event = a
            .iter()
            .filter(|(_, e)| matches!(e, ArchiveEntry::MediatorRegistryUpdated(_)))
            .map(|(c, _)| *c)
            .next()
            .unwrap();
        let inv = invoke(&mut a, &not_an_event, &panel, 14 * DAY, ts(1060));
        refused(&a, &inv, "is not a reported event");
        let rev = review(&mut a, &not_an_event, &panel, ReviewAnswer::Upheld, "n/a", ts(1061));
        refused(&a, &rev, "is not a reported event");
        let _ = ev;
    }

    /// Both panels carry the ordinary §4 panel rule: registered, active, and at least
    /// `panel_min` of them.
    #[test]
    fn both_panels_need_the_mediator_minimum() {
        let mut a = InMemoryArchive::new();
        let (_r, _rp, ev, pool) = fixture(&mut a, false);
        let two: Vec<(&DidKey, &PrivKey)> = pool.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        let inv = invoke(&mut a, &ev, &two, 14 * DAY, ts(1060));
        refused(&a, &inv, "invoking panel");

        // The review, in its own archive for the same reason as above.
        let mut b = InMemoryArchive::new();
        let (_r2, _rp2, ev2, pool2) = fixture(&mut b, false);
        let two2: Vec<(&DidKey, &PrivKey)> =
            pool2.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        let rev = review(&mut b, &ev2, &two2, ReviewAnswer::Upheld, "two of us", ts(1061));
        refused(&b, &rev, "reviewing panel");
    }
}

/// §4.1 — sealed decisions with threshold-reveal.
///
/// The measurement this part began from: a decision had to publish its reasoning in the
/// clear or be refused. There was no third state, so the sealed-mediation-proceedings
/// exception the whitepaper describes — "threshold encryption for sealed mediation
/// proceedings, with keys distributed across the mediator pool so no single custodian
/// can unseal early", opening at the 25-to-30-year mark — could not be recorded at all.
///
/// What is sealed is the reasoning. Dispute, panel and outcome stay public, so every
/// rule already built on a decision keeps running over a sealed one.
mod sealed_decisions {
    use super::*;
    use kc_core::{RevealedShare, ThresholdCustody};
    use kc_mediation::{DecisionUnsealed, SealedReasoning};

    use crate::authority::{MAX_DECISION_SEAL_SECONDS, MIN_DECISION_SEAL_SECONDS};

    const Q: usize = DEFAULT_QUORUM_THRESHOLD;
    const P: usize = DEFAULT_PANEL_MIN;
    const TEXT: &str = "the respondent's account of the boundary is the older one, and the \
                        complainant knew it";

    fn ok(a: &InMemoryArchive, cid: &Cid) {
        verify_all(a).expect("signatures valid");
        authorize(a, a.get(cid).unwrap(), Q, P).expect("expected authorized");
    }
    fn refused(a: &InMemoryArchive, cid: &Cid, needle: &str) {
        verify_all(a).expect("signatures valid");
        let err = authorize(a, a.get(cid).unwrap(), Q, P).expect_err("expected a refusal");
        assert!(err.to_string().contains(needle), "expected {needle:?}, got: {err}");
    }

    /// A dispute, a deciding panel of three, and three further communities to hold the
    /// shares.
    ///
    /// **A seal outlives everyone who set it**, which is the constraint that decides who
    /// holds the shares. `add_mediators` seats a one-year term and §4 caps any term at
    /// two, while the seal opens at twenty-five: a mediator's key stops resolving long
    /// before, so a share sealed to one could never be opened, and the signature on the
    /// opening could not even be verified. Communities resolve at any instant, so
    /// custodians are communities and the panel's homes are what the panel-quorum bar
    /// counts.
    #[allow(clippy::type_complexity)]
    fn fixture(
        a: &mut InMemoryArchive,
    ) -> (Cid, DidKey, DidKey, Vec<(DidKey, PrivKey, PrivKey)>, Vec<(DidKey, PrivKey)>) {
        let (ca, _ca_priv, cb, _cb_priv, dispute, panel) = mediation_fixture(a);
        let holders = vec![
            community(a, "F", ts(10)),
            community(a, "G", ts(10)),
            community(a, "H", ts(10)),
        ];
        (dispute, ca, cb, panel, holders)
    }

    /// The home communities of the mediators in `mediation_fixture`'s pool, which is what
    /// the panel-quorum bar counts once custodians are communities.
    fn panel_homes(
        a: &InMemoryArchive,
        pool: &[(DidKey, PrivKey, PrivKey)],
        at: Timestamp,
    ) -> Vec<DidKey> {
        pool.iter()
            .map(|(d, _, _)| {
                crate::authority::mediator_home(a, d.as_str(), at).expect("seated somewhere")
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn seal_decision(
        a: &mut InMemoryArchive,
        dispute: Cid,
        panel: &[(&DidKey, &PrivKey)],
        custodians: &[&DidKey],
        threshold: usize,
        text: &str,
        reveal_after: i64,
        cleartext_reasoning: &str,
        at: Timestamp,
    ) -> Cid {
        let arguments = panel_arguments(a, dispute, panel, at);
        let holders: Vec<DidKey> = custodians.iter().map(|d| (*d).clone()).collect();
        let custody = ThresholdCustody::seal(
            text.as_bytes(),
            &holders,
            kc_core::didkey_encryption_resolver,
            threshold,
        )
        .expect("seal the proceedings");
        let dec = Signed::build(
            MediationDecision {
                dispute_cid: dispute,
                panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
                outcome: DecisionOutcome::Upheld,
                reasoning: cleartext_reasoning.into(),
                sealed: Some(SealedReasoning {
                    reasoning_commitment: SealedReasoning::commitment(text).unwrap(),
                    custody,
                    reveal_at: ts(at.as_second() + reveal_after),
                }),
                arguments,
                decided_at: at,
            },
            &panel.iter().map(|(d, p)| (d.as_str(), *p)).collect::<Vec<_>>(),
        )
        .unwrap();
        a.append(ArchiveEntry::MediationDecided(dec)).unwrap()
    }

    /// Open `k` shares as their holders and return them.
    ///
    /// The key here is the holder's **encryption** key, not its signing key: a share is
    /// sealed by key agreement against the separate `encryption_pubkey` a mediator is
    /// registered under, which is the whole reason `MediatorEntry` carries two.
    fn open_shares(
        a: &InMemoryArchive,
        decision: &Cid,
        holders: &[(&DidKey, &PrivKey)],
    ) -> Vec<RevealedShare> {
        let sealed = match a.get(decision).unwrap() {
            ArchiveEntry::MediationDecided(s) => s.payload.sealed.as_ref().unwrap(),
            _ => panic!("not a decision"),
        };
        holders
            .iter()
            .map(|(d, p)| sealed.custody.open_share(d, p).expect("open own share"))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn unseal(
        a: &mut InMemoryArchive,
        decision: Cid,
        custodians: &[(&DidKey, &PrivKey)],
        shares: Vec<RevealedShare>,
        disclosed: &str,
        cause: &str,
        at: Timestamp,
    ) -> Cid {
        let rec = DecisionUnsealed {
            decision_cid: decision,
            custodians: custodians.iter().map(|(d, _)| (*d).clone()).collect(),
            cause: cause.into(),
            disclosed_reasoning: disclosed.into(),
            shares,
            unsealed_at: at,
        };
        let signed = Signed::build(
            rec,
            &custodians.iter().map(|(d, p)| (d.as_str(), *p)).collect::<Vec<_>>(),
        )
        .unwrap();
        a.append(ArchiveEntry::DecisionUnsealed(signed)).unwrap()
    }

    /// THE MEASUREMENT, now the rule: a decision can seal its proceedings, and the seal
    /// opens on a custodial threshold that recovers exactly what the panel wrote.
    #[test]
    fn a_panel_can_seal_its_proceedings_and_a_custodian_quorum_opens_them() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();

        let dec = seal_decision(
            &mut a,
            dispute,
            &deciding,
            &custodians,
            2,
            TEXT,
            MIN_DECISION_SEAL_SECONDS,
            "",
            ts(200),
        );
        ok(&a, &dec);

        // Two of the three custodians open their shares and publish the proceedings,
        // a generation later — when no mediator who decided it still holds a seat.
        let openers: Vec<(&DidKey, &PrivKey)> = holders.iter().take(2).map(|(d, p)| (d, p)).collect();
        let shares = open_shares(&a, &dec, &openers);
        let cid = unseal(
            &mut a,
            dec,
            &openers,
            shares,
            TEXT,
            "",
            ts(200 + MIN_DECISION_SEAL_SECONDS + 1),
        );
        ok(&a, &cid);
    }

    /// **The deciding panel may not hold a quorum of its own shares**, counted through
    /// the panelists' home communities. A panel that can unseal itself has deferred
    /// publication at its own discretion rather than sealed anything, and the parties who
    /// spoke candidly to it were given a guarantee that does not exist.
    #[test]
    fn a_panel_holding_a_quorum_of_its_own_shares_has_not_sealed_anything() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let homes = panel_homes(&a, &panel, ts(200));

        // Custody keyed to the panel's own communities: 2 of 3, all three theirs.
        let own: Vec<&DidKey> = homes.iter().collect();
        let dec = seal_decision(
            &mut a, dispute, &deciding, &own, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "", ts(200),
        );
        refused(&a, &dec, "the panel can unseal its own");

        // One panel home among the custodians is fine at k = 2: it takes two to open.
        let mixed: Vec<&DidKey> = vec![&homes[0], &holders[0].0, &holders[1].0];
        let dec_ok = seal_decision(
            &mut a, dispute, &deciding, &mixed, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "", ts(300),
        );
        ok(&a, &dec_ok);

        // Two panel homes at k = 2 is not: those two alone can open it.
        let two_own: Vec<&DidKey> = vec![&homes[0], &homes[1], &holders[0].0];
        let dec_bad = seal_decision(
            &mut a, dispute, &deciding, &two_own, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(400),
        );
        refused(&a, &dec_bad, "the panel can unseal its own");
    }

    /// Neither party may hold a share of the seal over its own proceedings: they are
    /// exactly who would want it opened early, or never.
    #[test]
    fn a_party_may_not_hold_a_share_of_its_own_dispute() {
        let mut a = InMemoryArchive::new();
        let (dispute, ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let with_party: Vec<&DidKey> = vec![&ca, &holders[0].0, &holders[1].0];
        let dec = seal_decision(
            &mut a, dispute, &deciding, &with_party, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(200),
        );
        refused(&a, &dec, "is a party to this dispute");
    }

    /// A custodian must be a published member community — something whose key still
    /// resolves when the seal opens. This is the rule that keeps the deterministic reveal
    /// from being dead on arrival.
    #[test]
    fn a_custodian_must_be_a_community_that_will_still_resolve() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        // A mediator: a seat §4 caps at two years, against a seal that runs twenty-five.
        let mediator = &panel[0].0;
        let with_mediator: Vec<&DidKey> = vec![mediator, &holders[0].0, &holders[1].0];
        let dec = seal_decision(
            &mut a, dispute, &deciding, &with_mediator, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(200),
        );
        refused(&a, &dec, "not a published member community");
    }

    /// A sealed decision carries no cleartext reasoning: one decision, one account of
    /// itself, and nothing that could contradict the sealed text with nobody able to
    /// check.
    #[test]
    fn a_sealed_decision_carries_no_public_summary() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();
        let dec = seal_decision(
            &mut a,
            dispute,
            &deciding,
            &custodians,
            2,
            TEXT,
            MIN_DECISION_SEAL_SECONDS,
            "in short, the complainant lost",
            ts(200),
        );
        refused(&a, &dec, "a sealed decision carries no cleartext reasoning");
    }

    /// An unsealed decision still owes its reasoning trail, and the refusal now names
    /// the alternative.
    #[test]
    fn an_unsealed_decision_still_owes_its_reasoning() {
        let mut a = InMemoryArchive::new();
        let (_ca, _ca_priv, _cb, _cb_priv, dispute, pool) = mediation_fixture(&mut a);
        let panel: Vec<(&DidKey, &PrivKey)> = pool.iter().map(|(d, p, _)| (d, p)).collect();
        let args = panel_arguments(&mut a, dispute, &panel, ts(200));
        let dec = Signed::build(
            MediationDecision {
                dispute_cid: dispute,
                panel: panel.iter().map(|(d, _)| (*d).clone()).collect(),
                outcome: DecisionOutcome::Upheld,
                reasoning: String::new(),
                sealed: None,
                arguments: args,
                decided_at: ts(200),
            },
            &panel.iter().map(|(d, p)| (d.as_str(), *p)).collect::<Vec<_>>(),
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::MediationDecided(dec)).unwrap();
        refused(&a, &cid, "mandatory) or seal it");
    }

    /// The generational cycle is a bound, not a number the record carries and nobody
    /// reads. A seal that opens next year is not a seal; one that opens in a century is
    /// not a deferral of publication but the end of it.
    #[test]
    fn the_seal_opens_within_a_generational_cycle() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();

        let early = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT,
            MIN_DECISION_SEAL_SECONDS - 1, "", ts(200),
        );
        refused(&a, &early, "outside the generational cycle");

        let late = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT,
            MAX_DECISION_SEAL_SECONDS + 1, "", ts(300),
        );
        refused(&a, &late, "outside the generational cycle");

        // NEGATIVE CONTROLS: both endpoints are inside.
        let at_min = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT,
            MIN_DECISION_SEAL_SECONDS, "", ts(400),
        );
        ok(&a, &at_min);
        let at_max = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT,
            MAX_DECISION_SEAL_SECONDS, "", ts(500),
        );
        ok(&a, &at_max);
    }

    /// **An unsealing must open what was sealed.** A custodian quorum cannot publish a
    /// more flattering account than the panel wrote: the shares must recover the text,
    /// and the text must open the commitment.
    #[test]
    fn an_unsealing_cannot_publish_a_reasoning_the_panel_never_wrote() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();
        let dec = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(200),
        );
        let openers: Vec<(&DidKey, &PrivKey)> = holders.iter().take(2).map(|(d, p)| (d, p)).collect();
        let shares = open_shares(&a, &dec, &openers);
        let after = ts(200 + MIN_DECISION_SEAL_SECONDS + 1);

        let lie = unseal(
            &mut a,
            dec,
            &openers,
            shares.clone(),
            "the panel found unanimously for the complainant",
            "",
            after,
        );
        refused(&a, &lie, "shares do not recover");

        // One share short of the threshold recovers nothing at all.
        let one: Vec<(&DidKey, &PrivKey)> = holders.iter().take(1).map(|(d, p)| (d, p)).collect();
        let short = open_shares(&a, &dec, &one);
        let starved = unseal(&mut a, dec, &one, short, TEXT, "", after);
        refused(&a, &starved, "does not open the custody");

        // NEGATIVE CONTROL: the true text with a real quorum is authorized.
        let honest = unseal(&mut a, dec, &openers, shares, TEXT, "", after);
        ok(&a, &honest);
    }

    /// Opening the seal early needs a named cause; after the cycle has run it does not.
    #[test]
    fn opening_a_seal_early_must_cite_a_cause() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();
        let dec = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(200),
        );
        let openers: Vec<(&DidKey, &PrivKey)> = holders.iter().take(2).map(|(d, p)| (d, p)).collect();
        let shares = open_shares(&a, &dec, &openers);

        let bare = unseal(&mut a, dec, &openers, shares.clone(), TEXT, "  ", ts(1_000_000));
        refused(&a, &bare, "must cite a cause");

        let for_cause = unseal(
            &mut a,
            dec,
            &openers,
            shares,
            TEXT,
            "the designation proceedings turn on what this panel found",
            ts(1_000_001),
        );
        ok(&a, &for_cause);
    }

    /// Only the seal's own custodians may open it, and the custodian list is derived
    /// from the co-signers as every panel field in this tier is.
    #[test]
    fn only_the_named_custodians_may_open_a_seal() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        // Custody held by two communities only.
        let custodians: Vec<&DidKey> = holders.iter().take(2).map(|(d, _)| d).collect();
        let dec = seal_decision(
            &mut a, dispute, &deciding, &custodians, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "",
            ts(200),
        );
        ok(&a, &dec);
        let openers: Vec<(&DidKey, &PrivKey)> = holders.iter().take(2).map(|(d, p)| (d, p)).collect();
        let shares = open_shares(&a, &dec, &openers);
        let after = ts(200 + MIN_DECISION_SEAL_SECONDS + 1);

        // A third community, holding no share, joins the signers.
        let mut intruding = openers.clone();
        intruding.push((&holders[2].0, &holders[2].1));
        let cid = unseal(&mut a, dec, &intruding, shares.clone(), TEXT, "", after);
        refused(&a, &cid, "holds no share of this decision's custody");

        // Naming custodians who did not sign.
        let rec = DecisionUnsealed {
            decision_cid: dec,
            custodians: vec![holders[0].0.clone(), holders[2].0.clone()],
            cause: String::new(),
            disclosed_reasoning: TEXT.into(),
            shares,
            unsealed_at: after,
        };
        let signed = Signed::build(
            rec,
            &openers.iter().map(|(d, p)| (d.as_str(), *p)).collect::<Vec<_>>(),
        )
        .unwrap();
        let cid = a.append(ArchiveEntry::DecisionUnsealed(signed)).unwrap();
        refused(&a, &cid, "must be exactly its co-signers");
    }

    /// An unsealing must reference a decision, and one that was actually sealed.
    #[test]
    fn unsealing_needs_a_sealed_decision_to_reference() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let deciding: Vec<(&DidKey, &PrivKey)> = panel.iter().map(|(d, p, _)| (d, p)).collect();
        let openers: Vec<(&DidKey, &PrivKey)> = holders.iter().take(2).map(|(d, p)| (d, p)).collect();

        let plain = decide(&mut a, dispute, &deciding, ts(200));
        let cid = unseal(&mut a, plain, &openers, vec![], TEXT, "", ts(300));
        refused(&a, &cid, "whose proceedings were never sealed");

        let cid = unseal(&mut a, dispute, &openers, vec![], TEXT, "", ts(300));
        refused(&a, &cid, "is not a mediation decision");
    }

    /// Every rule already built on a decision keeps running over a sealed one: the panel
    /// is still non-party, still `panel_min`, and still has to have argued.
    #[test]
    fn sealing_suspends_none_of_the_other_decision_rules() {
        let mut a = InMemoryArchive::new();
        let (dispute, _ca, _cb, panel, holders) = fixture(&mut a);
        let custodians: Vec<&DidKey> = holders.iter().map(|(d, _)| d).collect();
        let two: Vec<(&DidKey, &PrivKey)> = panel.iter().take(2).map(|(d, p, _)| (d, p)).collect();
        let dec = seal_decision(
            &mut a, dispute, &two, &custodians, 2, TEXT, MIN_DECISION_SEAL_SECONDS, "", ts(200),
        );
        refused(&a, &dec, "mediation panel");
    }
}
