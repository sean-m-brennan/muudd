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

//! The §9 constitutional payloads.

use kc_conduct::DecisionClass;
use kc_core::{DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// A matter the compact will not decide (categorical deference). `key` is the
/// stable identifier amendments match on; `description` is for the reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncapableMatter {
    pub key: String,
    pub description: String,
}

/// A structured-fast window: a period during which the compact's deliberative and
/// adjudicative functions pause (life-critical commons continue). Half-open
/// `[start, end)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPeriod {
    pub label: String,
    pub start: Timestamp,
    pub end: Timestamp,
}

impl FastPeriod {
    /// Is instant `t` inside this fast window?
    pub fn contains(&self, t: Timestamp) -> bool {
        self.start <= t && t < self.end
    }
    /// Duration in seconds (clamped at zero).
    pub fn seconds(&self) -> i64 {
        (self.end.as_second() - self.start.as_second()).max(0)
    }
}

/// The founding constitutional artifact: the initial compact-incapable list and
/// structured-fast cadence, ratified by the founding communities. There is
/// **exactly one** genesis per archive (a second is refused), and its matters are
/// **genesis-immutable** — protected by the same immutability as the recognition
/// statement's five movements, so they can never be removed (only added to).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstitutionGenesis {
    pub compact_incapable: Vec<IncapableMatter>,
    pub fast_periods: Vec<FastPeriod>,
    /// The founding communities (each co-signs); a distinct-community quorum.
    pub ratifiers: Vec<DidKey>,
    pub genesis_at: Timestamp,
}

/// Whether an amendment expands or contracts a deference commitment. Expansion is
/// the easy direction; contraction carries the higher bar (the asymmetric capture
/// risk).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AmendmentKind {
    /// Add a matter to the compact-incapable list (expansion — easy).
    Add,
    /// Remove a matter from the list (contraction — hard; never a genesis matter).
    Remove,
}

impl AmendmentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "Add",
            Self::Remove => "Remove",
        }
    }
    /// Contraction (the capture-risk direction) carries the higher quorum bar.
    pub fn is_contraction(&self) -> bool {
        matches!(self, Self::Remove)
    }
}

/// A second-class amendment to the compact-incapable list. Co-signed by the
/// deciders. `Add` requires the normal distinct-community quorum; `Remove`
/// requires the higher contraction bar and is refused entirely for a
/// genesis-original matter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactIncapableAmendment {
    pub kind: AmendmentKind,
    pub matter: IncapableMatter,
    pub rationale: String,
    pub deciders: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub decided_at: Timestamp,
}

/// Whether a fast-cadence amendment lengthens/adds (expansion) or shortens/removes
/// (contraction) a fast period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FastChange {
    /// Add or lengthen a fast period (expansion — easy).
    Extend,
    /// Shorten or remove a fast period (contraction — hard).
    Contract,
}

impl FastChange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Extend => "Extend",
            Self::Contract => "Contract",
        }
    }
    pub fn is_contraction(&self) -> bool {
        matches!(self, Self::Contract)
    }
}

/// A second-class amendment to the structured-fast cadence. The resulting
/// `period` is the fast window as it stands after the change, matched by `label`.
/// `Contract` carries the higher quorum bar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastCadenceAmendment {
    pub change: FastChange,
    pub period: FastPeriod,
    pub rationale: String,
    pub deciders: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub decided_at: Timestamp,
}
