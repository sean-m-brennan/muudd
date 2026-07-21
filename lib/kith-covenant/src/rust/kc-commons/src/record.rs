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

//! The §7 commons payloads and their classifying enums.

use kc_conduct::DecisionClass;
use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// The five compact-level commons (commons_federation.md §"The five space
/// commons"). Each is non-excludable at the margin, rivalrous under stress, and
/// weaponizable by withholding or pollution, so each is held by the compact
/// rather than owned by any single member community.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommonsKind {
    /// Atmosphere generation, water recycling, thermal regulation.
    LifeSupport,
    /// Electromagnetic spectrum and relays for intra-system communication.
    Communication,
    /// Ephemerides, conjunction warnings, debris tracking.
    Navigation,
    /// The accumulated knowledge of the compact ("inheritance of the future").
    KnowledgeArchive,
    /// Distress beacons, rescue vessels, medical facilities.
    Rescue,
}

impl CommonsKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LifeSupport => "LifeSupport",
            Self::Communication => "Communication",
            Self::Navigation => "Navigation",
            Self::KnowledgeArchive => "KnowledgeArchive",
            Self::Rescue => "Rescue",
        }
    }

    pub fn all() -> [CommonsKind; 5] {
        [
            Self::LifeSupport,
            Self::Communication,
            Self::Navigation,
            Self::KnowledgeArchive,
            Self::Rescue,
        ]
    }
}

/// Registration of a compact-level commons. Ostrom principle 1 (clearly defined
/// boundaries: who the appropriators are and what the resource is) and principle
/// 2 (proportional provision). A compact-level act: **second-class**, co-signed
/// by the registering communities under a distinct-community quorum. The commons
/// is thereafter referenced by this record's archive CID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonsRegistration {
    pub kind: CommonsKind,
    pub name: String,
    pub description: String,
    /// The member communities with defined rights to draw on this commons
    /// (Ostrom's boundary). Usage and sanction records are meaningful only for a
    /// community in this set.
    pub appropriators: Vec<DidKey>,
    /// The rule tying benefit to contribution (principle 2), free-form for the
    /// reader; not machine-interpreted in v1.
    pub provision_rule: String,
    /// The registering communities (each co-signs). Must be a non-empty subset of
    /// `appropriators` and satisfy the second-class quorum.
    pub registered_by: Vec<DidKey>,
    pub registered_at: Timestamp,
}

/// A community's self-reported accounting of what it drew from and returned to a
/// commons over a period. First-class, self-signed; feeds free-rider detection
/// (Ostrom principle 4 monitoring). `consumed`/`contributed` are dimensionless
/// units in the commons' own terms (bandwidth-hours, rescue-standby-hours,
/// archive-bytes, …); v1 interprets them only through the free-rider ratio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageRecord {
    /// The registered commons this reports on (its `CommonsRegistered` CID).
    pub commons_cid: Cid,
    pub community: DidKey,
    pub consumed: u64,
    pub contributed: u64,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub recorded_at: Timestamp,
}

/// The graduated-sanction ladder for a commons (Ostrom principle 5). Distinct
/// from the §5 dispute ladder: this tracks one community's abuse of a shared
/// resource, not a conflict between two communities. `None` is the resting state
/// and the de-escalation floor (restoration path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SanctionLevel {
    /// No sanction in force. The resting state and the de-escalation floor.
    None,
    /// Heightened monitoring (first-offense-tolerant: the lightest response).
    Monitoring,
    /// A formal warning on the record.
    Warning,
    /// Restricted drawdown rights until restoration. Second-class.
    Restriction,
    /// Excluded from the commons (subject to restoration). Second-class.
    Exclusion,
}

impl SanctionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Monitoring => "Monitoring",
            Self::Warning => "Warning",
            Self::Restriction => "Restriction",
            Self::Exclusion => "Exclusion",
        }
    }
}

/// Ordinal rank for the legal-progression check (higher = more severe).
pub fn sanction_rank(level: SanctionLevel) -> u8 {
    use SanctionLevel::*;
    match level {
        None => 0,
        Monitoring => 1,
        Warning => 2,
        Restriction => 3,
        Exclusion => 4,
    }
}

/// The two levels that impose compact-wide cost and so require a
/// distinct-community quorum and a non-unilateral imposer set — the commons
/// analogue of [`kc_escalation::is_second_class_rung`] (commons_federation.md
/// §"Light-lag-honest decision classes"; §"Community-applied sanctions").
pub fn is_second_class_level(level: SanctionLevel) -> bool {
    matches!(level, SanctionLevel::Restriction | SanctionLevel::Exclusion)
}

/// One step of a community along (or back down) a commons' graduated-sanction
/// ladder. Append-only: the current level of a (commons, subject) pair is the
/// `to_level` of its latest sanction. Each record names the level it leaves and
/// the level it enters, so the archive can reject out-of-order or skipping moves;
/// de-escalation (a lower `to_level`) is the always-legal restoration move.
/// Co-signed by the `imposers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonsSanction {
    /// The registered commons this concerns (its `CommonsRegistered` CID).
    pub commons_cid: Cid,
    pub subject_community: DidKey,
    pub from_level: SanctionLevel,
    pub to_level: SanctionLevel,
    /// The specific abuse cited (overdraw, pollution, withholding-as-leverage).
    pub basis: String,
    /// Prior archive records this cites (usage records, conduct, prior sanctions).
    pub references: Vec<Cid>,
    /// The communities imposing (each co-signs). For a second-class level these
    /// are the distinct-community quorum; a first-class level (monitoring,
    /// warning, or any de-escalation) may be moved by a single appropriator.
    pub imposers: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub decided_at: Timestamp,
}
