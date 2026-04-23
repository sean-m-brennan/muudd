//! Projection types returned by `lookup`.

use serde::{Deserialize, Serialize};

use kc_ceremony::inter::AntiSpiralFeature;
use kc_core::{Cid, DidKey, Timestamp};
use kc_identity::{DelegationScope, WitnessRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecognitionStanding {
    /// Community DID absent from archive at `as_of`.
    Unknown,
    /// `CommunityIdentityPublished` present; no inter ceremony yet.
    Identified,
    /// ≥1 sealed `InterCommunityWitness` including this community.
    InCompact,
    /// Inter record(s) exist but only non-admissions; no seal.
    Denied,
}

impl RecognitionStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Identified => "Identified",
            Self::InCompact => "InCompact",
            Self::Denied => "Denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntraRiteSummary {
    pub record_cid: Cid,
    pub sealed_at: Timestamp,
    pub speaker_did: DidKey,
    pub witness_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterAdmissionSummary {
    pub record_cid: Cid,
    pub witnessed_at: Timestamp,
    pub counterparties: Vec<DidKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterNonAdmissionSummary {
    pub record_cid: Cid,
    pub decided_at: Timestamp,
    pub counterparties: Vec<DidKey>,
    pub rejections: Vec<RejectionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectionSummary {
    pub rejecting_community: DidKey,
    pub presenting_community: DidKey,
    pub feature: AntiSpiralFeature,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationSummary {
    pub record_cid: Cid,
    pub subject_did: DidKey,
    pub scope: DelegationScope,
    pub active_from: Timestamp,
    pub active_until: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessSummary {
    pub record_cid: Cid,
    pub member_did: DidKey,
    pub role: WitnessRole,
    pub active_from: Timestamp,
    pub active_until: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationView {
    pub community_did: DidKey,
    pub as_of: Timestamp,
    pub standing: RecognitionStanding,
    pub identity_cid: Option<Cid>,
    pub intra_rites: Vec<IntraRiteSummary>,
    pub inter_admissions: Vec<InterAdmissionSummary>,
    pub inter_nonadmissions: Vec<InterNonAdmissionSummary>,
    pub active_delegations: Vec<DelegationSummary>,
    pub active_witnesses: Vec<WitnessSummary>,
}
