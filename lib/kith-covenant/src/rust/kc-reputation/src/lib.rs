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
    adversarial_principal, ceremony_cost_decay, ceremony_cost_decay_all, collusive_cohorts,
    compact_incapable, composition_graph, constitution_state, deliberation_decay,
    deliberative_capacity, designation_consensus, enforcement_convergence,
    enforcement_convergence_all,
    enforcement_direction, enforcement_direction_all, erosion_alert, erosion_alert_with,
    feature_concern_profile, free_rider_signals, in_structured_fast, interpretive_plurality,
    mediator_pool_concentration, minimum_forbearance, observer_dashboard,
    observer_pool_concentration, overdue_reexaminations,
    principal_witness_chain, rejection_pattern, silent_functions, unanswered_grievances,
    unstaffed_functions,
    sponsor_stake, structured_skepticism, substrate_deference, CeremonyCostDecay,
    CohortCollusionReport, CohortDivergence, CompositionGraph, ConstitutionState, CrossWitness,
    DeliberationDecay, DeliberationReading, DeliberativeCapacity, DesignationConsensus,
    EnforcementConvergence, EnforcementDirection,
    ErosionAlert, ErosionSeverity, ErosionThresholds, EventProvenance, FastDeviation,
    CONCRETE_BEARINGS,
    FeatureConcernProfile, FreeRiderReport, InterpretivePluralityReport, LineagePoint,
    MediatorPoolConcentration, MediatorPoolShare, ObserverDashboard,
    ObserverPoolConcentration, ObserverPoolShare, OutputDivergence,
    OverdueReaffirmation, OverdueReexamination, PrincipalValidationReport, RejectionEvent,
    RejectionPattern, RitePoint, SilentFunction,
    ThinningDeliberation, UnansweredGrievance, UnstaffedFunction,
    ServingMediator, ServingObserver, SkepticismReport, SponseeOutcome, SponsorEdge, SponsorStakeReport,
    SubstrateDeferenceReport, UsageTally, WitnessChainReport, WitnessLink,
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
