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

//! The [`ConductRecord`] payload and its classifying enums.
//!
//! A `ConductRecord` is wrapped in `Signed<ConductRecord>` by its reporter(s)
//! and appended to the archive. It is never mutated: a withdrawn `Concern`, a
//! lifted designation, or a completed restitution is a *follow-on* record whose
//! `references` point at the prior record's CID (mirroring the supersession
//! pattern used by `WitnessRegistry` and `SpeakerDelegation.revoked_at`).

use kc_ceremony::inter::AntiSpiralFeature;
use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// What kind of conduct observation this record makes.
///
/// The ordering (roughly ascending in gravity) is deliberate but carries no
/// protocol meaning on its own; escalation semantics live in component 5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConductKind {
    /// Positive attestation: the subject acted in a way that evidences a
    /// feature or virtue. Counterweight so the ledger is not purely negative.
    Commendation,
    /// The jester's pre-indictment flag. A signed, attributable observation of
    /// drift that is *not* an accusation and triggers no ladder rung. Legible
    /// in aggregate while individual acts retain plausible deniability.
    Concern,
    /// An adjudicated finding that the subject violated a common virtue or
    /// obligation. Distinct from `Concern`: this one has been through
    /// mediation (component 4) and carries a `ladder_rung`.
    Violation,
    /// The subject has been designated anathema. Second-class decision; the
    /// signatures should span a multi-community quorum (enforcement deferred).
    DesignationAnathema,
    /// A prior `DesignationAnathema` (referenced by CID) has been lifted.
    DesignationLifted,
    /// A restitution obligation (referenced by CID) has been completed and
    /// verified.
    RestorationCompleted,
}

impl ConductKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Commendation => "Commendation",
            Self::Concern => "Concern",
            Self::Violation => "Violation",
            Self::DesignationAnathema => "DesignationAnathema",
            Self::DesignationLifted => "DesignationLifted",
            Self::RestorationCompleted => "RestorationCompleted",
        }
    }

    /// Does this kind count against the subject when summing capture signals?
    /// `Concern` and `Violation` do; commendations and the resolution kinds
    /// (lifted / restored) do not.
    pub fn is_adverse(&self) -> bool {
        matches!(self, Self::Concern | Self::Violation | Self::DesignationAnathema)
    }
}

/// What the conduct bears on. The `AntiSpiral` variant reuses the five-feature
/// enum from the recognition ceremony so a community's adverse conduct can be
/// profiled *per anti-spiral feature* — the behavioral shadow of a hollow or
/// absent substrate (e.g. repeated failures of `NonUltimacyOfCompact` or
/// `EnemyLoveOperational`), which is exactly what the substrate-deference test
/// predicts of a community that professes a substrate it does not hold.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConductBearing {
    /// Bears on one of the five anti-spiral features.
    AntiSpiral(AntiSpiralFeature),
    /// Bears on a commons obligation (rescue capacity, agreed drawdowns, etc.).
    CommonsObligation,
    /// Bears on charity toward members in need or the community's own
    /// vulnerable (bounded_conflict.md §"Charity and material disparity").
    Charity,
    /// Bears on the inviolable cross-community rescue duty.
    Rescue,
    /// Anything else; the string names the category for the reader.
    Other(String),
}

/// Coarse severity. Feeds the reputation decay model, which is a step function
/// for grave violations plus slow decay for minor ones (component 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    Minor,
    Serious,
    Grave,
}

/// Position on the canonical escalation ladder
/// (bounded_conflict.md §"Protocol sketch"). `Some(..)` only for enforcement
/// acts; a bare `Concern` or `Commendation` leaves `ladder_rung` `None`.
/// `SubstantiveReEngagement` is the ceremony-cost-decay track, not a dispute
/// rung, but is carried here so the audit commons holds one conduct timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LadderRung {
    DisputeRegistered,
    RegisteredProtest,
    ReciprocalSuspension,
    MediatedArbitration,
    LimitedSanctions,
    CollectiveSanctions,
    Expulsion,
    SubstantiveReEngagement,
}

/// Light-lag decision class (commons_federation.md §"Light-lag-honest decision
/// classes"). Recorded here so the archive holds the classification; the
/// distinct-community quorum *check* is deferred to the escalation component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecisionClass {
    /// Valid when made by an authorized local body within its envelope.
    FirstClass,
    /// Requires a multi-community quorum signature to validate.
    SecondClass,
}

/// One attributable, signed observation about a community's conduct.
///
/// Wrapped in `Signed<ConductRecord>`; for `SecondClass` kinds the envelope
/// should carry one signature per quorum member. Keys are resolved `as_of`
/// [`ConductRecord::recorded_at`] at the archive layer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConductRecord {
    /// Whom the conduct is attributed to.
    pub subject_community: DidKey,
    /// The filer(s). For `SecondClass` kinds these are the quorum; for a
    /// `Concern` this is typically a single declared-perspective observer.
    pub reporters: Vec<DidKey>,
    /// The reporters' declared standpoint, kept explicit so downstream
    /// analytics can discount partiality rather than treat the report as
    /// neutral (jester_and_dissent.md, adversarial-information model).
    /// `None` for neutral records.
    pub reporter_perspective: Option<String>,
    pub kind: ConductKind,
    pub bearing: ConductBearing,
    pub severity: Severity,
    /// Escalation-ladder position, when this record is an enforcement act.
    pub ladder_rung: Option<LadderRung>,
    pub decision_class: DecisionClass,
    /// Prior conduct records this one responds to (resolution / supersession
    /// chains). E.g. a `DesignationLifted` references the `DesignationAnathema`.
    pub references: Vec<Cid>,
    /// Free-form account for the reader; not machine-interpreted.
    pub narrative: String,
    /// When the conduct was observed.
    pub observed_at: Timestamp,
    /// When this record was written (used for key resolution).
    pub recorded_at: Timestamp,
}

// -------------------------------------------------------------------------
// Attested nulls
// -------------------------------------------------------------------------

/// A standing function of the compact whose exercise is attested on a cadence.
///
/// One enum rather than a record per component, because the requirement is identical on
/// both sides and the erosion audit names them as twins: mediation and observation are
/// the two functions that can quietly stop being performed while every relative measure
/// stays unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AttestedFunction {
    /// The §4 mediator pool: disputes taken, or none to take.
    Mediation,
    /// The §8 observer corps: reports filed, or nothing seen.
    Observation,
}

impl AttestedFunction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mediation => "Mediation",
            Self::Observation => "Observation",
        }
    }
}

/// A community's signed account of what it did in one window while holding a standing
/// function, **including an account of having done nothing**.
///
/// This is the erosion audit's "attested null" (remediation item 5), and the empty `acts`
/// list is the whole point of the record. Without it, silence is missing data: a mediator
/// pool that took no disputes because none arose and a pool that stopped answering read
/// identically, and so do an observer corps with nothing to report and one that stopped
/// looking. With it, silence is either evidence (an attested quiet window) or a violation
/// (a window nobody attested), which is what the audit asks for. `en-archive`'s pruning
/// receipts are the pattern: the log may forget, but only by saying so.
///
/// It is signed by the *nominating community*, not by the individual mediators or
/// observers, because the obligation is the community's for as long as it holds seats.
/// Nothing here is refused: a missing attestation is read by the erosion lenses rather
/// than blocking any act, since a compact that has stopped acting has no act to refuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityAttestation {
    pub function: AttestedFunction,
    /// The community that holds seats in this function and owes the account.
    pub attesting_community: DidKey,
    pub window_start: Timestamp,
    /// The end of the window being accounted for. Must be after `window_start`.
    pub window_end: Timestamp,
    /// The acts performed in the window, by CID. **Legally empty**, and an empty list is
    /// the signed null rather than an omission.
    pub acts: Vec<Cid>,
    /// The account for the reader. Required non-empty when `acts` is empty, because a
    /// null is the one case that owes an explanation: "no disputes were filed" and "we
    /// stopped convening" are both nulls and are not the same thing.
    pub statement: String,
    pub attested_at: Timestamp,
}
