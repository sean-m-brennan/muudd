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

//! Read-side projection over the kith-covenant archive. No scoring, no
//! mutations — just structured facts plus a small categorical standings
//! enum derived from those facts.
//!
//! See `docs/superpowers/specs/2026-04-22-reputation-lookup-design.md`.

pub mod analytics;
pub mod lookup;
pub mod view;

pub use analytics::{
    ceremony_cost_decay, composition_graph, feature_concern_profile, rejection_pattern,
    sponsor_stake, CeremonyCostDecay, CompositionGraph, FeatureConcernProfile, RejectionEvent,
    RejectionPattern, RitePoint, SponseeOutcome, SponsorEdge, SponsorStakeReport,
};
pub use lookup::lookup;
pub use view::{
    ConductSummary, DelegationSummary, DesignationSummary, DisputeSummary, InterAdmissionSummary,
    InterNonAdmissionSummary, IntraRiteSummary, ReEntrySummary, RecognitionStanding,
    RejectionSummary, ReputationView, RestitutionSummary, SponsorshipSummary,
    SubstantiveReEngagementSummary, WitnessSummary,
};

#[cfg(test)]
mod tests;
