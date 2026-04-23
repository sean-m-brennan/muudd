//! Read-side projection over the kith-covenant archive. No scoring, no
//! mutations — just structured facts plus a small categorical standings
//! enum derived from those facts.
//!
//! See `docs/superpowers/specs/2026-04-22-reputation-lookup-design.md`.

pub mod lookup;
pub mod view;

pub use lookup::lookup;
pub use view::{
    DelegationSummary, InterAdmissionSummary, InterNonAdmissionSummary,
    IntraRiteSummary, RecognitionStanding, RejectionSummary, ReputationView,
    WitnessSummary,
};

#[cfg(test)]
mod tests;
