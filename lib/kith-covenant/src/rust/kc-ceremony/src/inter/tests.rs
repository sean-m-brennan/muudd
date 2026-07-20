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

//! Inter-community credentialing integration tests.

use jiff::ToSpan;
use kc_core::{keys, time, Alg, DidKey, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};

use crate::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use crate::recitation::{Recitation, RecitationVersion};

use super::anti_spiral::{AntiSpiralFeature, REQUIRED_FEATURES};
use super::runner::{InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext};

struct Community {
    community: Signed<CommunityIdentity>,
    envoy_did: DidKey,
    envoy_priv: PrivKey,
    intra_record: Signed<IntraCommunityRiteRecord>,
}

fn mk_community(name: &str, now: Timestamp) -> Community {
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519).unwrap();
    let community =
        CommunityIdentity::new_self_signed(name.into(), &root_priv, root_pub, now).unwrap();
    let community_did = community.payload.did.clone();

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

    let (witness_priv, witness_pk) = keys::generate(Alg::Ed25519).unwrap();
    let witness_did = DidKey::from_pubkey(&witness_pk).unwrap();
    let witness_registry = Signed::build(
        WitnessRegistry {
            community_did: community_did.clone(),
            entries: vec![WitnessEntry {
                witness_did: witness_did.clone(),
                witness_pubkey: witness_pk,
                role: WitnessRole::IntraCommunity,
                valid_from: now.checked_sub(1.hour()).unwrap(),
                valid_until: Some(now.checked_add(24.hours()).unwrap()),
            }],
            issued_at: now,
        },
        &[(community_did.as_str(), &root_priv)],
    )
    .unwrap();

    let delegations = vec![speaker_delegation];
    let runner = IntraCommunityRiteRunner::new(&community, &delegations, &witness_registry);
    let recitation = IntraCommunityRiteRunner::recite(
        &speaker_did,
        &speaker_priv,
        Recitation {
            community_did: community_did.clone(),
            speaker_did: speaker_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: format!("{name} anchors above itself."),
            movement_fallenness: "We are capable of harm.".into(),
            movement_recognition: "We recognize the other.".into(),
            movement_common_virtue: "We commit to truthfulness and restitution.".into(),
            movement_non_ultimacy: "This compact is not ultimate.".into(),
            language_tag: "en".into(),
            translation_en: None,
            recited_at: now,
        },
    )
    .unwrap();
    let attestation = IntraCommunityRiteRunner::attest(
        &witness_did,
        &witness_priv,
        recitation.payload_cid,
        now,
    )
    .unwrap();
    let intra_record = runner
        .seal(
            recitation,
            vec![attestation],
            &root_priv,
            RecitationVersion::current(),
            now,
        )
        .unwrap();

    Community { community, envoy_did, envoy_priv, intra_record }
}

fn claims_for(community_did: &DidKey, envoy_did: &DidKey, envoy_priv: &PrivKey, now: Timestamp) -> Vec<Signed<super::AntiSpiralFeatureClaim>> {
    REQUIRED_FEATURES
        .iter()
        .copied()
        .map(|feature| {
            InterCredentialingRunner::present_claim(
                envoy_did,
                envoy_priv,
                community_did.clone(),
                feature,
                format!("{feature:?} evidenced."),
                vec![],
                now,
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn happy_path_seals_admission() {
    let now = time::now();
    let alpha = mk_community("Alpha", now);
    let beta = mk_community("Beta", now);

    let participants = vec![
        ParticipantContext {
            community: &alpha.community,
            intra_record: &alpha.intra_record,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.community,
            intra_record: &beta.intra_record,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let runner = InterCredentialingRunner::new("alpha-beta-compact".into(), participants);

    let mut claims = claims_for(&alpha.community.payload.did, &alpha.envoy_did, &alpha.envoy_priv, now);
    claims.extend(claims_for(&beta.community.payload.did, &beta.envoy_did, &beta.envoy_priv, now));

    // Each community accepts the other's 5 claims.
    let mut acceptances = Vec::new();
    for claim in &claims {
        let accepter = if claim.payload.community_did == alpha.community.payload.did {
            (&beta.envoy_did, &beta.envoy_priv, beta.community.payload.did.clone())
        } else {
            (&alpha.envoy_did, &alpha.envoy_priv, alpha.community.payload.did.clone())
        };
        acceptances.push(
            InterCredentialingRunner::accept(
                accepter.0,
                accepter.1,
                accepter.2,
                claim.payload_cid,
                true,
                None,
                now,
            )
            .unwrap(),
        );
    }

    let out = runner.seal(claims, acceptances, vec![], now).unwrap();
    match out {
        InterCredentialingOutcome::Sealed(signed) => {
            assert_eq!(signed.payload.participants.len(), 2);
            assert_eq!(signed.payload.claims.len(), 10);
            assert_eq!(signed.payload.acceptances.len(), 10);
            assert_eq!(signed.signatures.len(), 2);
            assert_eq!(signed.payload.sequenced_rites.len(), 2);
            assert_eq!(signed.payload.sequenced_rites[0].sequence_index, 0);
            assert_eq!(signed.payload.sequenced_rites[1].sequence_index, 1);
        }
        InterCredentialingOutcome::NonAdmission(_) => panic!("expected Sealed, got NonAdmission"),
    }
}

#[test]
fn rejection_yields_non_admission() {
    let now = time::now();
    let alpha = mk_community("Alpha", now);
    let beta = mk_community("Beta", now);

    let participants = vec![
        ParticipantContext {
            community: &alpha.community,
            intra_record: &alpha.intra_record,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.community,
            intra_record: &beta.intra_record,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let runner = InterCredentialingRunner::new("alpha-beta-compact".into(), participants);

    let mut claims = claims_for(&alpha.community.payload.did, &alpha.envoy_did, &alpha.envoy_priv, now);
    claims.extend(claims_for(&beta.community.payload.did, &beta.envoy_did, &beta.envoy_priv, now));

    // Beta rejects Alpha's NonUltimacyOfCompact claim; all others accepted.
    let mut acceptances = Vec::new();
    for claim in &claims {
        let accepter = if claim.payload.community_did == alpha.community.payload.did {
            (&beta.envoy_did, &beta.envoy_priv, beta.community.payload.did.clone())
        } else {
            (&alpha.envoy_did, &alpha.envoy_priv, alpha.community.payload.did.clone())
        };
        let accepted = !(claim.payload.community_did == alpha.community.payload.did
            && claim.payload.feature == AntiSpiralFeature::NonUltimacyOfCompact);
        acceptances.push(
            InterCredentialingRunner::accept(
                accepter.0,
                accepter.1,
                accepter.2,
                claim.payload_cid,
                accepted,
                if accepted {
                    None
                } else {
                    Some("Alpha's non-ultimacy claim not convincing yet.".into())
                },
                now,
            )
            .unwrap(),
        );
    }

    let out = runner.seal(claims, acceptances, vec![], now).unwrap();
    match out {
        InterCredentialingOutcome::Sealed(_) => panic!("expected NonAdmission, got Sealed"),
        InterCredentialingOutcome::NonAdmission(signed) => {
            assert_eq!(signed.payload.rejections.len(), 1);
            assert_eq!(signed.payload.rejections[0].feature, AntiSpiralFeature::NonUltimacyOfCompact);
            assert_eq!(
                signed.payload.rejections[0].presenting_community,
                alpha.community.payload.did
            );
            assert_eq!(signed.signatures.len(), 2);
        }
    }
}

#[test]
fn missing_claim_blocks_seal() {
    let now = time::now();
    let alpha = mk_community("Alpha", now);
    let beta = mk_community("Beta", now);

    let participants = vec![
        ParticipantContext {
            community: &alpha.community,
            intra_record: &alpha.intra_record,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.community,
            intra_record: &beta.intra_record,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let runner = InterCredentialingRunner::new("alpha-beta-compact".into(), participants);

    // Alpha submits only 4 features; misses NonUltimacyOfCompact.
    let alpha_partial: Vec<_> = REQUIRED_FEATURES
        .iter()
        .copied()
        .filter(|f| *f != AntiSpiralFeature::NonUltimacyOfCompact)
        .map(|feature| {
            InterCredentialingRunner::present_claim(
                &alpha.envoy_did,
                &alpha.envoy_priv,
                alpha.community.payload.did.clone(),
                feature,
                "evidence".into(),
                vec![],
                now,
            )
            .unwrap()
        })
        .collect();
    let mut claims = alpha_partial;
    claims.extend(claims_for(&beta.community.payload.did, &beta.envoy_did, &beta.envoy_priv, now));

    let err = runner.seal(claims, vec![], vec![], now).unwrap_err();
    match err {
        kc_core::KcError::Archive(msg) => {
            assert!(msg.contains("exactly one claim"));
            assert!(msg.contains("NonUltimacyOfCompact"));
        }
        e => panic!("wrong error: {e:?}"),
    }
}

#[test]
fn single_participant_rejected() {
    let now = time::now();
    let solo = mk_community("Solo", now);
    let participants = vec![ParticipantContext {
        community: &solo.community,
        intra_record: &solo.intra_record,
        cross_boundary_signer_did: solo.envoy_did.clone(),
        cross_boundary_signer_priv: &solo.envoy_priv,
    }];
    let runner = InterCredentialingRunner::new("solo-compact".into(), participants);
    assert!(matches!(
        runner.seal(vec![], vec![], vec![], now),
        Err(kc_core::KcError::Archive(_))
    ));
}

#[test]
fn missing_acceptance_blocks_seal() {
    let now = time::now();
    let alpha = mk_community("Alpha", now);
    let beta = mk_community("Beta", now);
    let participants = vec![
        ParticipantContext {
            community: &alpha.community,
            intra_record: &alpha.intra_record,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.community,
            intra_record: &beta.intra_record,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let runner = InterCredentialingRunner::new("alpha-beta-compact".into(), participants);
    let mut claims = claims_for(&alpha.community.payload.did, &alpha.envoy_did, &alpha.envoy_priv, now);
    claims.extend(claims_for(&beta.community.payload.did, &beta.envoy_did, &beta.envoy_priv, now));
    // No acceptances at all.
    let err = runner.seal(claims, vec![], vec![], now).unwrap_err();
    assert!(matches!(err, kc_core::KcError::Archive(_)));
}

