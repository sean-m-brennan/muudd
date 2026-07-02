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

//! End-to-end: run the full two-community ceremony via kc-ceremony's actual
//! runners, then use kc-reputation::lookup to project standing. This covers
//! the "real signatures, real runners" path not covered by the fixture-based
//! unit tests.

use kc_archive::{verify_all, Archive, ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::{
    InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext, REQUIRED_FEATURES,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_core::{keys, time, Alg, DidKey, KcError, PrivKey, Signed};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};
use kc_reputation::{lookup, RecognitionStanding};

use jiff::ToSpan;

// ── local helpers (mirrors kc-pyo3 private helpers, stripped of PyResult) ──

struct CommunityBundle {
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

fn mk_community(name: &str) -> Result<CommunityBundle, KcError> {
    let now = time::now();

    let (root_priv, root_pub) = keys::generate(Alg::Ed25519)?;
    let signed_identity =
        CommunityIdentity::new_self_signed(name.into(), &root_priv, root_pub, now)?;
    let community_did = signed_identity.payload.did.clone();

    let (speaker_priv, speaker_pk) = keys::generate(Alg::Ed25519)?;
    let speaker_did = DidKey::from_pubkey(&speaker_pk)?;
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
    )?;

    let (envoy_priv, envoy_pk) = keys::generate(Alg::Ed25519)?;
    let envoy_did = DidKey::from_pubkey(&envoy_pk)?;
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
    )?;

    let (witness_priv, witness_pk) = keys::generate(Alg::Ed25519)?;
    let witness_did = DidKey::from_pubkey(&witness_pk)?;
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
    )?;

    Ok(CommunityBundle {
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
    })
}

fn seed_archive(archive: &mut InMemoryArchive, c: &CommunityBundle) -> Result<(), KcError> {
    archive.append(ArchiveEntry::CommunityIdentityPublished(
        c.signed_identity.clone(),
    ))?;
    archive.append(ArchiveEntry::SpeakerDelegated(c.speaker_delegation.clone()))?;
    archive.append(ArchiveEntry::SpeakerDelegated(c.envoy_delegation.clone()))?;
    archive.append(ArchiveEntry::WitnessRegistryUpdated(
        c.witness_registry.clone(),
    ))?;
    Ok(())
}

fn run_intra_rite(c: &CommunityBundle) -> Result<Signed<IntraCommunityRiteRecord>, KcError> {
    let now = time::now();
    let delegations = vec![c.speaker_delegation.clone()];
    let runner =
        IntraCommunityRiteRunner::new(&c.signed_identity, &delegations, &c.witness_registry);

    let recitation = IntraCommunityRiteRunner::recite(
        &c.speaker_did,
        &c.speaker_priv,
        Recitation {
            community_did: c.signed_identity.payload.did.clone(),
            speaker_did: c.speaker_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: "We are anchored above self.".into(),
            movement_fallenness: "We acknowledge our fallenness.".into(),
            movement_recognition: "We recognize the other.".into(),
            movement_common_virtue: "We share common virtue.".into(),
            movement_non_ultimacy: "The compact is not ultimate.".into(),
            language_tag: "en".into(),
            translation_en: None,
            recited_at: now,
        },
    )?;

    let attestation = IntraCommunityRiteRunner::attest(
        &c.witness_did,
        &c.witness_priv,
        recitation.payload_cid,
        now,
    )?;

    runner.seal(
        recitation,
        vec![attestation],
        &c.root_priv,
        RecitationVersion::current(),
        now,
    )
}

/// Build an archive by running a full two-community ceremony, mirroring the
/// flow in `kc_pyo3::run_two_community_ceremony` but stripped of Python
/// boilerplate. Returns an `InMemoryArchive` populated with:
///   - 2 CommunityIdentityPublished entries
///   - at least 1 IntraCommunityRite per community (sealed rite)
///   - 1 InterCommunityWitness entry (the admission)
///   - delegations + registries for both communities
fn build_two_community_archive() -> InMemoryArchive {
    let alpha = mk_community("Alpha").expect("mk_community Alpha");
    let beta = mk_community("Beta").expect("mk_community Beta");

    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &alpha).expect("seed alpha");
    seed_archive(&mut archive, &beta).expect("seed beta");

    let alpha_rite = run_intra_rite(&alpha).expect("intra rite alpha");
    let beta_rite = run_intra_rite(&beta).expect("intra rite beta");
    archive
        .append(ArchiveEntry::IntraCommunityRite(alpha_rite.clone()))
        .expect("append alpha rite");
    archive
        .append(ArchiveEntry::IntraCommunityRite(beta_rite.clone()))
        .expect("append beta rite");

    let now = time::now();

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
        InterCredentialingRunner::new("two-community-compact".into(), participants);

    // Each community presents one claim per required feature.
    let mut claims = Vec::new();
    for (community_did, envoy_did, envoy_priv) in [
        (
            alpha.signed_identity.payload.did.clone(),
            &alpha.envoy_did,
            &alpha.envoy_priv,
        ),
        (
            beta.signed_identity.payload.did.clone(),
            &beta.envoy_did,
            &beta.envoy_priv,
        ),
    ] {
        for feature in REQUIRED_FEATURES {
            let claim = InterCredentialingRunner::present_claim(
                envoy_did,
                envoy_priv,
                community_did.clone(),
                feature,
                format!("{feature:?} evidenced by our sealed rite and practice."),
                vec![],
                now,
            )
            .expect("present_claim");
            claims.push(claim);
        }
    }

    // The other community accepts every claim.
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
        let acceptance = InterCredentialingRunner::accept(
            signer_did,
            signer_priv,
            accepter_did,
            claim.payload_cid,
            true,
            None,
            now,
        )
        .expect("accept");
        acceptances.push(acceptance);
    }

    let outcome = inter_runner
        .seal(claims, acceptances, vec![], now)
        .expect("inter_runner.seal");

    let witness_record = match outcome {
        InterCredentialingOutcome::Sealed(s) => s,
        InterCredentialingOutcome::NonAdmission(_) => {
            panic!("expected Sealed but got NonAdmission");
        }
    };
    archive
        .append(ArchiveEntry::InterCommunityWitness(witness_record))
        .expect("append witness record");

    archive
}

#[test]
fn end_to_end_two_community_ceremony_projects_incompact_for_both() {
    let archive = build_two_community_archive();

    // Sanity: archive verifies end-to-end.
    verify_all(&archive).expect("archive verifies");

    // Extract the two community DIDs from the archive.
    let mut community_dids = Vec::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::CommunityIdentityPublished(signed) = entry {
            community_dids.push(signed.payload.did.clone());
        }
    }
    assert_eq!(community_dids.len(), 2);

    for did in &community_dids {
        let view = lookup(&archive, did, None);
        assert_eq!(
            view.standing,
            RecognitionStanding::InCompact,
            "community {did} should be InCompact after a successful two-party inter ceremony"
        );
        assert_eq!(view.inter_admissions.len(), 1, "one admission record");
        assert_eq!(
            view.inter_admissions[0].counterparties.len(),
            1,
            "one counterparty"
        );
        assert!(!view.intra_rites.is_empty(), "at least one sealed intra rite");
    }
}
