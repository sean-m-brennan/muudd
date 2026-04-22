//! Inter-community ceremony records.
//!
//! Two terminal artifact types:
//! - [`InterCommunityWitnessRecord`] — admission; all participants accepted
//!   all of each other's feature claims.
//! - [`NonAdmissionRecord`] — refusal; at least one claim was rejected.
//!
//! Both are sealed with one signature per participant (cross-boundary signer).

use kc_core::{Cid, DidKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

use super::anti_spiral::{AntiSpiralFeature, AntiSpiralFeatureClaim, FeatureAcceptance};

/// One community's sealed intra rite embedded at a specific order position
/// in the inter ceremony.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequencedRite {
    pub reciting_community: DidKey,
    pub intra_record_cid: Cid,
    pub sequence_index: u32,
}

/// Free-form question-and-response exchange during the examination phase.
/// Optional component; MVP may leave empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExaminationExchange {
    pub question_from: DidKey,
    pub question_text: String,
    pub response_from: DidKey,
    pub response_text: String,
    pub exchanged_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterCommunityWitnessRecord {
    pub compact_label: String,
    pub participants: Vec<DidKey>,
    pub sequenced_rites: Vec<SequencedRite>,
    pub claims: Vec<Signed<AntiSpiralFeatureClaim>>,
    pub acceptances: Vec<Signed<FeatureAcceptance>>,
    pub examination_exchanges: Vec<ExaminationExchange>,
    pub witnessed_at: Timestamp,
}

/// Recorded when the inter ceremony produces at least one standing rejection.
/// Structurally parallel to the admission record so both forms can sit in
/// the archive side-by-side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonAdmissionRecord {
    pub compact_label: String,
    pub participants: Vec<DidKey>,
    pub rejections: Vec<RejectionDetail>,
    pub decided_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectionDetail {
    pub rejecting_community: DidKey,
    pub presenting_community: DidKey,
    pub feature: AntiSpiralFeature,
    pub rationale: String,
}
