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

//! Projection types returned by `lookup`.

use serde::{Deserialize, Serialize};

use kc_ceremony::inter::AntiSpiralFeature;
use kc_conduct::{ConductBearing, ConductKind, DecisionClass, LadderRung, Severity};
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
    /// Admitted, but within an active received-sponsorship probation window
    /// (a sponsor's standing is staked on this community's conduct until
    /// `probation_until`). A refinement of `InCompact`.
    Probationary,
    /// Inter record(s) exist but only non-admissions; no seal.
    Denied,
    /// An active `DesignationAnathema` stands against this community that has
    /// not been superseded by a later `DesignationLifted`. Overrides all other
    /// standings.
    Anathema,
}

impl RecognitionStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Identified => "Identified",
            Self::InCompact => "InCompact",
            Self::Probationary => "Probationary",
            Self::Denied => "Denied",
            Self::Anathema => "Anathema",
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

/// One conduct record projected for the subject community, in record-time order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConductSummary {
    pub record_cid: Cid,
    pub kind: ConductKind,
    pub bearing: ConductBearing,
    pub severity: Severity,
    pub ladder_rung: Option<LadderRung>,
    pub decision_class: DecisionClass,
    pub reporters: Vec<DidKey>,
    pub reporter_perspective: Option<String>,
    pub references: Vec<Cid>,
    pub recorded_at: Timestamp,
}

/// One sponsorship edge touching this community (as sponsor or as entrant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SponsorshipSummary {
    pub record_cid: Cid,
    pub sponsor_community: DidKey,
    pub entrant_community: DidKey,
    pub admission_cid: Option<Cid>,
    pub probation_until: Timestamp,
    pub sponsored_at: Timestamp,
}

/// One dispute this community is party to (§4), with the mediation outcome if a
/// panel has decided it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeSummary {
    pub record_cid: Cid,
    pub complainant: DidKey,
    pub respondent: DidKey,
    pub rung: LadderRung,
    /// The dispute's current rung: the latest ladder transition's target, or the
    /// filed rung if it has not yet moved (§5).
    pub current_rung: LadderRung,
    pub summary: String,
    pub filed_at: Timestamp,
    /// `None` while open; the `DecisionOutcome` name once decided.
    pub decided_outcome: Option<String>,
}

/// One anathema designation naming this community as subject (§5), with the
/// structured evidence of the four-condition test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignationSummary {
    pub record_cid: Cid,
    /// `Anathema` or `HostisHumaniGeneris`.
    pub tier: String,
    /// `Normal` or `Expedited`.
    pub urgency: String,
    pub criteria: String,
    /// `None` for the hostis-humani-generis tier (no reversal path).
    pub reversal_conditions: Option<String>,
    pub reexamine_after: Option<Timestamp>,
    /// `true` once a `DesignationLifted` or a re-entry has reversed it.
    pub reversed: bool,
    pub designated_at: Timestamp,
}

/// One re-entry this community completed from an anathema designation (§5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReEntrySummary {
    pub record_cid: Cid,
    pub designation_cid: Cid,
    pub probation_until: Timestamp,
    pub completed_at: Timestamp,
}

/// One restitution obligation on this community (§5), joined with its
/// verification outcome if a verifier panel has ruled on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestitutionSummary {
    pub record_cid: Cid,
    pub beneficiary: DidKey,
    pub terms: String,
    /// `None` while unverified; `Some(true/false)` once a panel verifies.
    pub satisfied: Option<bool>,
    pub imposed_at: Timestamp,
}

/// One deliberate invocation of the substantive-re-engagement rung against this
/// community (§5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstantiveReEngagementSummary {
    pub record_cid: Cid,
    pub required_reaffirmation_by: Timestamp,
    pub invoked_at: Timestamp,
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
    /// Conduct records naming this community as subject, oldest first.
    pub conduct: Vec<ConductSummary>,
    /// Sponsorships this community gave (it is the sponsor).
    pub sponsorships_given: Vec<SponsorshipSummary>,
    /// Sponsorships this community received (it is the entrant).
    pub sponsorships_received: Vec<SponsorshipSummary>,
    /// Disputes this community filed (it is the complainant), oldest first.
    pub disputes_filed: Vec<DisputeSummary>,
    /// Disputes filed against this community (it is the respondent), oldest first.
    pub disputes_against: Vec<DisputeSummary>,
    /// Anathema designations naming this community, oldest first (§5).
    pub designations: Vec<DesignationSummary>,
    /// Re-entries this community completed, oldest first (§5).
    pub reentries: Vec<ReEntrySummary>,
    /// Restitution obligations on this community, oldest first (§5).
    pub restitutions: Vec<RestitutionSummary>,
    /// Substantive-re-engagement invocations against this community (§5).
    pub substantive_reengagements: Vec<SubstantiveReEngagementSummary>,
}
