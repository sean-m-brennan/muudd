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

//! What a host running a polity has to do on a cadence, and when (design **D20**).
//!
//! # The one rule this crate is built around
//!
//! **`tick` decides and computes; the host signs and does I/O.**
//!
//! Every cadence in this tier is a query-time parameter — `stale_members(cadence,
//! as_of)`, `Thresholds`, `cohesion_report(as_of)`, `prune_recommendation(as_of)` —
//! because no library here calls `now()`. That discipline is what makes the tier's
//! judgements reproducible and testable at any moment, real or simulated, and D20
//! keeps it: [`HostCycle::tick`] takes `as_of` like everything else. Exactly one
//! place in ethne reads a clock, and it is not a library — it is the
//! `polity_host` binary that drives this.
//!
//! `tick` also **authors nothing**. Signing keys belong to the host, and the two
//! actions that would need them — raising a kith-covenant Concern, and pruning the
//! observation log — are returned as *recommendations over a computed report*. That
//! is the same posture `prune_recommendation` already takes, and the same principle
//! the tier is built on: emergence proposes, the polity ratifies. Housekeeping and
//! alarm-raising are no exception.
//!
//! # What it covers, and what it deliberately does not
//!
//! The cadenced duties, and only those: persist, grade cohesion, notice that the
//! grade has reached `Concern`, and notice that the observation log is due for
//! pruning. Membership, induction, office conferral and envoy authorization are
//! **deliberate acts of the polity**, not scheduled ones, and nothing here initiates
//! them.

use en_archive::Archive;
use en_charter::{
    affirmation_state, charter_state, decay_trends, guardian_state, observation_window,
    prune_recommendation, CharterEntry, PruneRecommendation, RetentionPolicy,
};
use en_cohesion::{cohesion_report, CohesionInputs, CohesionReport, ErosionLevel, Thresholds};
use en_core::{DidKey, Timestamp};
use en_substrate::FoundingCandidate;

/// How often a host attends to each duty, and on what terms.
///
/// The cadences are policy, so they live here rather than in any record — a polity's
/// charter does not say how often its host wakes up.
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// The polity this host runs.
    pub polity: DidKey,
    /// Where each cohesion signal tips into `Notice` and into `Concern`.
    pub thresholds: Thresholds,
    /// How much observation history to keep, and the floors that protect a live
    /// measure from being clipped (D19).
    pub retention: RetentionPolicy,
    /// Whether the substrate's boundary is opaque, which caps attrition at `Notice`.
    pub opaque_boundary: bool,
    /// How often to grade cohesion. Defaults to the attendance cadence, the
    /// fastest-moving input the grade has.
    pub cohesion_cadence_seconds: i64,
    /// How often to ask whether the observation log is due for pruning. Daily by
    /// default: housekeeping is not urgent, and `min_dropped` already guards churn.
    pub retention_cadence_seconds: i64,
    /// How often to persist. Zero — every tick — by default, because the log is
    /// evidence and the store already writes nothing when nothing has changed.
    pub save_cadence_seconds: i64,
    /// How far back to read the observation record when grading. Defaults to the
    /// retention span: everything the polity has chosen to keep, and no more, since
    /// beyond that there is nothing to read anyway.
    pub observation_window_seconds: i64,
}

impl HostConfig {
    /// A configuration with the tier's own defaults for `polity`.
    pub fn new(polity: DidKey) -> Self {
        let thresholds = Thresholds::default();
        let retention = RetentionPolicy::default();
        HostConfig {
            polity,
            opaque_boundary: false,
            cohesion_cadence_seconds: thresholds.attendance_cadence_seconds,
            retention_cadence_seconds: 86_400,
            save_cadence_seconds: 0,
            observation_window_seconds: retention.retain_seconds,
            thresholds,
            retention,
        }
    }
}

/// Something the host should do now. Returned, never performed.
#[derive(Debug, Clone)]
pub enum DueAction {
    /// Persist the log. The store decides whether that is an append or a rewrite.
    Save,
    /// The polity's cohesion as of this tick, below `Concern`.
    Cohesion(CohesionReport),
    /// Cohesion has reached `Concern` — the level that crosses to kith-covenant.
    ///
    /// The host raises it with `en_uplift::coherence_concern`, which needs the
    /// observer's signing key. That is why this is a report and not a record.
    RaiseConcern(CohesionReport),
    /// The observation log is due for pruning, on these terms (D19). The host feeds
    /// `horizon` to `prune_observations`; the polity still ratifies its own
    /// housekeeping.
    Prune(PruneRecommendation),
}

impl DueAction {
    /// A short, stable name — for logging, and so a host can count what it did.
    pub fn kind(&self) -> &'static str {
        match self {
            DueAction::Save => "Save",
            DueAction::Cohesion(_) => "Cohesion",
            DueAction::RaiseConcern(_) => "RaiseConcern",
            DueAction::Prune(_) => "Prune",
        }
    }
}

/// A host's cadence state: what it last attended to, and when.
///
/// Holds no clock and no archive — both arrive at [`tick`](HostCycle::tick), so the
/// same cycle can be driven over a simulated timeline in a test and a real one in
/// production, with identical results.
#[derive(Debug, Clone)]
pub struct HostCycle {
    config: HostConfig,
    last_save: Option<Timestamp>,
    last_cohesion: Option<Timestamp>,
    last_retention: Option<Timestamp>,
}

impl HostCycle {
    pub fn new(config: HostConfig) -> Self {
        HostCycle { config, last_save: None, last_cohesion: None, last_retention: None }
    }

    pub fn config(&self) -> &HostConfig {
        &self.config
    }

    /// What is due at `as_of`.
    ///
    /// Everything is due on the first tick. A duty is marked attended when it is
    /// *computed*, not when the host acts on the result — so ignoring a returned
    /// action does not make the host spin on it; the next cadence will recompute.
    pub fn tick(
        &mut self,
        archive: &Archive<CharterEntry>,
        candidate: &FoundingCandidate,
        as_of: Timestamp,
    ) -> Vec<DueAction> {
        let mut due = Vec::new();

        if elapsed(self.last_save, self.config.save_cadence_seconds, as_of) {
            self.last_save = Some(as_of);
            due.push(DueAction::Save);
        }

        // Nothing below is meaningful for a polity that does not exist yet or has
        // ended. Persisting still is: the log is what remains of it.
        let state = charter_state(archive, &self.config.polity, as_of);
        if !state.chartered || state.dissolved {
            return due;
        }

        if elapsed(self.last_cohesion, self.config.cohesion_cadence_seconds, as_of) {
            self.last_cohesion = Some(as_of);
            let report = self.grade(archive, candidate, as_of);
            due.push(if report.erosion == ErosionLevel::Concern {
                DueAction::RaiseConcern(report)
            } else {
                DueAction::Cohesion(report)
            });
        }

        if elapsed(self.last_retention, self.config.retention_cadence_seconds, as_of) {
            self.last_retention = Some(as_of);
            if let Some(rec) =
                prune_recommendation(archive, &self.config.polity, &self.config.retention, as_of)
            {
                due.push(DueAction::Prune(rec));
            }
        }

        due
    }

    /// Grade cohesion with **every** read-model the archive can supply.
    ///
    /// Assembling these is the part a host would otherwise have to know to do, and
    /// omitting one is silent: each missing input makes its component report nothing
    /// rather than complain, which is right for a library and wrong for a host that
    /// simply forgot.
    pub fn grade(
        &self,
        archive: &Archive<CharterEntry>,
        candidate: &FoundingCandidate,
        as_of: Timestamp,
    ) -> CohesionReport {
        let polity = &self.config.polity;
        let state = charter_state(archive, polity, as_of);
        let guardians = guardian_state(archive, polity, as_of);
        let affirmations = affirmation_state(archive, polity, as_of);
        let trends = decay_trends(archive, polity, as_of);
        let since = Timestamp::from_second(
            as_of.as_second().saturating_sub(self.config.observation_window_seconds),
        )
        .unwrap_or(as_of);
        let window = observation_window(archive, polity, since, as_of);

        cohesion_report(
            &state.members,
            candidate,
            self.config.opaque_boundary,
            &CohesionInputs {
                guardians: Some(&guardians),
                affirmations: Some(&affirmations),
                trends: Some(&trends),
                window: Some(&window),
            },
            as_of,
            &self.config.thresholds,
        )
    }
}

/// Whether `cadence` seconds have passed since `last`. Never attended ⇒ due.
///
/// A cadence of zero means every tick. A clock that goes backwards simply reports
/// nothing due until it catches up, which is the conservative direction: a host with
/// a bad clock under-acts rather than grading the same moment repeatedly.
fn elapsed(last: Option<Timestamp>, cadence: i64, as_of: Timestamp) -> bool {
    match last {
        None => true,
        Some(t) => as_of.as_second() - t.as_second() >= cadence,
    }
}

#[cfg(test)]
mod tests;
