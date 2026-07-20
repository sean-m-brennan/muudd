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

//! Typed archive entries. One variant per record kind that the protocol
//! produces. Every entry carries a `Signed<...>` envelope; the archive stores
//! the enum and indexes by `Cid`.

use kc_ceremony::inter::{InterCommunityWitnessRecord, NonAdmissionRecord, SponsorshipRecord};
use kc_ceremony::intra::IntraCommunityRiteRecord;
use kc_conduct::ConductRecord;
use kc_core::{Cid, KcResult, Signed};
use kc_escalation::{
    Designation, LadderTransition, ReEntry, RestitutionCompletion, RestitutionObligation,
    SubstantiveReEngagement,
};
use kc_identity::{CommunityIdentity, SpeakerDelegation, WitnessRegistry};
use kc_mediation::{DisputeRecord, MediationDecision, MediatorRegistry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveEntry {
    CommunityIdentityPublished(Signed<CommunityIdentity>),
    SpeakerDelegated(Signed<SpeakerDelegation>),
    WitnessRegistryUpdated(Signed<WitnessRegistry>),
    IntraCommunityRite(Signed<IntraCommunityRiteRecord>),
    InterCommunityWitness(Signed<InterCommunityWitnessRecord>),
    InterCommunityNonAdmission(Signed<NonAdmissionRecord>),
    /// An existing member vouches for an entrant (capture_resistance.md §7).
    SponsorshipVouched(Signed<SponsorshipRecord>),
    /// An attributable observation about a community's conduct, including the
    /// jester's pre-indictment `Concern` (kc-conduct).
    ConductAttested(Signed<ConductRecord>),
    /// A community's roster of nominated mediators (§4). The pool is the union
    /// across communities; supersedes by latest `issued_at` per community.
    MediatorRegistryUpdated(Signed<MediatorRegistry>),
    /// A dispute filed by one community against another (§4).
    DisputeFiled(Signed<DisputeRecord>),
    /// A mediation panel's decision on a dispute (§4), co-signed by the panel.
    MediationDecided(Signed<MediationDecision>),
    /// A dispute's advance (or de-escalation) along the escalation ladder (§5).
    LadderAdvanced(Signed<LadderTransition>),
    /// A community's designation as anathema (§5), co-signed by the quorum.
    DesignationDecided(Signed<Designation>),
    /// A restitution obligation imposed on a community (§5).
    RestitutionObligated(Signed<RestitutionObligation>),
    /// A verification that a restitution obligation was met (§5), co-signed by a
    /// non-party mediator panel.
    RestitutionVerified(Signed<RestitutionCompletion>),
    /// A community's re-entry from an anathema designation (§5), co-signed by the
    /// admitting quorum.
    ReEntryCompleted(Signed<ReEntry>),
    /// A deliberate invocation of the substantive-re-engagement rung (§5).
    SubstantiveReEngagementInvoked(Signed<SubstantiveReEngagement>),
}

impl ArchiveEntry {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CommunityIdentityPublished(_) => "CommunityIdentityPublished",
            Self::SpeakerDelegated(_) => "SpeakerDelegated",
            Self::WitnessRegistryUpdated(_) => "WitnessRegistryUpdated",
            Self::IntraCommunityRite(_) => "IntraCommunityRite",
            Self::InterCommunityWitness(_) => "InterCommunityWitness",
            Self::InterCommunityNonAdmission(_) => "InterCommunityNonAdmission",
            Self::SponsorshipVouched(_) => "SponsorshipVouched",
            Self::ConductAttested(_) => "ConductAttested",
            Self::MediatorRegistryUpdated(_) => "MediatorRegistryUpdated",
            Self::DisputeFiled(_) => "DisputeFiled",
            Self::MediationDecided(_) => "MediationDecided",
            Self::LadderAdvanced(_) => "LadderAdvanced",
            Self::DesignationDecided(_) => "DesignationDecided",
            Self::RestitutionObligated(_) => "RestitutionObligated",
            Self::RestitutionVerified(_) => "RestitutionVerified",
            Self::ReEntryCompleted(_) => "ReEntryCompleted",
            Self::SubstantiveReEngagementInvoked(_) => "SubstantiveReEngagementInvoked",
        }
    }

    /// Hash the entry itself for archive-level addressing. (This is distinct
    /// from any nested `Signed<T>.payload_cid`, which addresses the payload.)
    pub fn cid(&self) -> KcResult<Cid> {
        Cid::of(self)
    }
}
