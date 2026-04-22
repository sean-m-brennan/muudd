//! Inter-community credentialing runner.
//!
//! Consumes only *completed*, *sealed* intra-community rite records — the
//! two-layer rule (see doc/architecture/protocol-components.md §2) forbids
//! this runner from reaching into the intra layer.
//!
//! `seal()` enforces:
//! - ≥2 participants.
//! - Each participant submits exactly one claim per anti-spiral feature.
//! - Every other participant submits exactly one acceptance verdict per claim.
//! - If all verdicts are `accepted: true` → `Sealed(InterCommunityWitnessRecord)`.
//! - Any `accepted: false` verdict → `NonAdmission(NonAdmissionRecord)`.
//! - The terminal record is co-signed by every participant's cross-boundary signer.

use kc_core::{Cid, DidKey, KcError, KcResult, PrivKey, Signed, Timestamp};
use kc_identity::CommunityIdentity;

use crate::intra::IntraCommunityRiteRecord;

use super::anti_spiral::{
    AntiSpiralFeature, AntiSpiralFeatureClaim, EvidenceRef, FeatureAcceptance, REQUIRED_FEATURES,
};
use super::witness_record::{
    ExaminationExchange, InterCommunityWitnessRecord, NonAdmissionRecord, RejectionDetail,
    SequencedRite,
};

/// A participant in an inter-community ceremony. Carries both the identity
/// context (community + sealed intra rite) and the cross-boundary signer that
/// will co-sign the terminal inter record.
pub struct ParticipantContext<'a> {
    pub community: &'a Signed<CommunityIdentity>,
    pub intra_record: &'a Signed<IntraCommunityRiteRecord>,
    pub cross_boundary_signer_did: DidKey,
    pub cross_boundary_signer_priv: &'a PrivKey,
}

pub struct InterCredentialingRunner<'a> {
    pub compact_label: String,
    pub participants: Vec<ParticipantContext<'a>>,
}

#[derive(Debug)]
pub enum InterCredentialingOutcome {
    Sealed(Signed<InterCommunityWitnessRecord>),
    NonAdmission(Signed<NonAdmissionRecord>),
}

impl<'a> InterCredentialingRunner<'a> {
    pub fn new(compact_label: String, participants: Vec<ParticipantContext<'a>>) -> Self {
        Self { compact_label, participants }
    }

    /// Ergonomic wrapper to build a signed claim.
    pub fn present_claim(
        signer_did: &DidKey,
        signer_priv: &PrivKey,
        community_did: DidKey,
        feature: AntiSpiralFeature,
        claim_text: String,
        evidence_refs: Vec<EvidenceRef>,
        claimed_at: Timestamp,
    ) -> KcResult<Signed<AntiSpiralFeatureClaim>> {
        let payload = AntiSpiralFeatureClaim {
            community_did,
            feature,
            claim_text,
            evidence_refs,
            claimed_at,
        };
        Signed::build(payload, &[(signer_did.as_str(), signer_priv)])
    }

    /// Ergonomic wrapper to build a signed acceptance verdict.
    pub fn accept(
        signer_did: &DidKey,
        signer_priv: &PrivKey,
        accepting_community_did: DidKey,
        claim_cid: Cid,
        accepted: bool,
        rationale: Option<String>,
        decided_at: Timestamp,
    ) -> KcResult<Signed<FeatureAcceptance>> {
        let payload = FeatureAcceptance {
            accepting_community_did,
            claim_cid,
            accepted,
            rationale,
            decided_at,
        };
        Signed::build(payload, &[(signer_did.as_str(), signer_priv)])
    }

    pub fn seal(
        &self,
        claims: Vec<Signed<AntiSpiralFeatureClaim>>,
        acceptances: Vec<Signed<FeatureAcceptance>>,
        examination_exchanges: Vec<ExaminationExchange>,
        witnessed_at: Timestamp,
    ) -> KcResult<InterCredentialingOutcome> {
        if self.participants.len() < 2 {
            return Err(KcError::Archive(
                "inter credentialing requires at least 2 participants".into(),
            ));
        }

        let participant_dids: Vec<DidKey> = self
            .participants
            .iter()
            .map(|p| p.community.payload.did.clone())
            .collect();

        // Each participant must present exactly one claim per required feature.
        for pdid in &participant_dids {
            for feature in REQUIRED_FEATURES {
                let n = claims
                    .iter()
                    .filter(|c| c.payload.community_did == *pdid && c.payload.feature == feature)
                    .count();
                if n != 1 {
                    return Err(KcError::Archive(format!(
                        "participant {pdid} must present exactly one claim for {feature:?}, got {n}"
                    )));
                }
            }
        }

        // Cross-verify acceptances: for every claim, every other participant
        // must have submitted exactly one acceptance.
        let mut rejections: Vec<RejectionDetail> = Vec::new();
        for claim in &claims {
            let presenting = &claim.payload.community_did;
            for accepter in &participant_dids {
                if accepter == presenting {
                    continue;
                }
                let matched: Vec<&Signed<FeatureAcceptance>> = acceptances
                    .iter()
                    .filter(|a| {
                        a.payload.accepting_community_did == *accepter
                            && a.payload.claim_cid == claim.payload_cid
                    })
                    .collect();
                if matched.len() != 1 {
                    return Err(KcError::Archive(format!(
                        "acceptance missing or duplicated: accepter {accepter} on claim by {presenting} for {:?} (got {} verdicts)",
                        claim.payload.feature,
                        matched.len()
                    )));
                }
                let verdict = matched[0];
                if !verdict.payload.accepted {
                    rejections.push(RejectionDetail {
                        rejecting_community: accepter.clone(),
                        presenting_community: presenting.clone(),
                        feature: claim.payload.feature,
                        rationale: verdict
                            .payload
                            .rationale
                            .clone()
                            .unwrap_or_else(|| "(no rationale provided)".into()),
                    });
                }
            }
        }

        // Build signer list owned-then-borrowed to satisfy Signed::build.
        let owned_dids: Vec<String> = self
            .participants
            .iter()
            .map(|p| p.cross_boundary_signer_did.as_str().to_string())
            .collect();
        let signers: Vec<(&str, &PrivKey)> = owned_dids
            .iter()
            .zip(self.participants.iter())
            .map(|(d, p)| (d.as_str(), p.cross_boundary_signer_priv))
            .collect();

        if !rejections.is_empty() {
            let record = NonAdmissionRecord {
                compact_label: self.compact_label.clone(),
                participants: participant_dids,
                rejections,
                decided_at: witnessed_at,
            };
            let signed = Signed::build(record, &signers)?;
            return Ok(InterCredentialingOutcome::NonAdmission(signed));
        }

        let sequenced_rites: Vec<SequencedRite> = self
            .participants
            .iter()
            .enumerate()
            .map(|(idx, p)| SequencedRite {
                reciting_community: p.community.payload.did.clone(),
                intra_record_cid: p.intra_record.payload_cid,
                sequence_index: idx as u32,
            })
            .collect();

        let record = InterCommunityWitnessRecord {
            compact_label: self.compact_label.clone(),
            participants: participant_dids,
            sequenced_rites,
            claims,
            acceptances,
            examination_exchanges,
            witnessed_at,
        };
        let signed = Signed::build(record, &signers)?;
        Ok(InterCredentialingOutcome::Sealed(signed))
    }
}
