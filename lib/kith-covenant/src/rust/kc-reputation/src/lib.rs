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
    adversarial_principal, ceremony_cost_decay, collusive_cohorts, compact_incapable,
    composition_graph, constitution_state, erosion_alert, feature_concern_profile,
    free_rider_signals, in_structured_fast, interpretive_plurality, observer_dashboard,
    observer_pool_concentration, principal_witness_chain, rejection_pattern, sponsor_stake,
    structured_skepticism, substrate_deference, CeremonyCostDecay, CohortCollusionReport,
    CohortDivergence, CompositionGraph, ConstitutionState, CrossWitness, ErosionAlert,
    ErosionSeverity, EventProvenance, FastDeviation,
    FeatureConcernProfile, FreeRiderReport, InterpretivePluralityReport, LineagePoint,
    ObserverDashboard, ObserverPoolConcentration, ObserverPoolShare, OutputDivergence,
    PrincipalValidationReport, RejectionEvent, RejectionPattern, RitePoint, ServingObserver,
    SkepticismReport, SponseeOutcome, SponsorEdge, SponsorStakeReport, SubstrateDeferenceReport,
    UsageTally, WitnessChainReport, WitnessLink,
};
pub use lookup::{commons_level, has_capability, lookup};
pub use view::{
    CommonsSanctionSummary, ConductSummary, DelegationSummary, DesignationSummary, DisputeSummary,
    InjunctionSummary, InterAdmissionSummary, InterNonAdmissionSummary, IntraRiteSummary,
    ObserverReportSummary, ReEntrySummary, RecognitionStanding, RejectionSummary,
    ReportResponseSummary, ReputationView, RestitutionSummary, SponsorshipSummary,
    SubstantiveReEngagementSummary, WhistleblowerSummary, WitnessSummary,
};

#[cfg(test)]
mod tests;
