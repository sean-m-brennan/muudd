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
/// adjudicative functions pause (life-critical commons continue). The first
/// occurrence is the half-open window `[start, end)`; with `recurrence` set it
/// repeats on that cadence (a calendar rhythm — an annual fast, the Iroquois
/// Condolence period — rather than a one-off).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPeriod {
    pub label: String,
    pub start: Timestamp,
    pub end: Timestamp,
    /// Recurrence cycle in seconds. `Some(cycle)` with `cycle >= end - start`
    /// repeats the fast every `cycle` seconds from `start`, each occurrence lasting
    /// `end - start`. `None` — or a cycle shorter than the occurrence (occurrences
    /// may not overlap) — is a single window. Absent in older archives → `None`.
    #[serde(default)]
    pub recurrence: Option<i64>,
}

impl FastPeriod {
    /// Is instant `t` inside this fast — the first window, or any later occurrence
    /// when the fast recurs?
    pub fn contains(&self, t: Timestamp) -> bool {
        let (start, end, now) = (self.start.as_second(), self.end.as_second(), t.as_second());
        if now < start {
            return false;
        }
        let duration = end - start;
        match self.recurrence {
            // A valid recurrence repeats the occurrence every `cycle` seconds; the
            // fast is active when the phase within the current cycle is inside it.
            Some(cycle) if cycle > 0 && cycle >= duration && duration > 0 => {
                (now - start) % cycle < duration
            }
            // No recurrence (or a degenerate one): the single window `[start, end)`.
            _ => now < end,
        }
    }
    /// Duration of one occurrence in seconds (clamped at zero).
    pub fn seconds(&self) -> i64 {
        (self.end.as_second() - self.start.as_second()).max(0)
    }
    /// Does this fast repeat on a calendar cadence (a valid recurrence), rather
    /// than occurring once?
    pub fn is_recurring(&self) -> bool {
        matches!(self.recurrence, Some(cycle) if cycle > 0 && cycle >= self.seconds() && self.seconds() > 0)
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

/// Where in the compact's conduct a forbearance operates (natural_law.md "The four
/// sites"). Recorded so a reader can see which part of the boundary a contraction
/// reaches; the amendment rule is the same at every site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MinimumSite {
    /// Duties owed to a non-member met in trade, transit, rescue, or disaster.
    NonMemberDuties,
    /// The standard the *hostis humani generis* designation rests on.
    HostisHumaniGeneris,
    /// What binds a restricted member that names no transcendent authority.
    RestrictedMember,
    /// The refusals governing what the compact does to those outside it.
    BoundaryRefusal,
}

impl MinimumSite {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NonMemberDuties => "NonMemberDuties",
            Self::HostisHumaniGeneris => "HostisHumaniGeneris",
            Self::RestrictedMember => "RestrictedMember",
            Self::BoundaryRefusal => "BoundaryRefusal",
        }
    }
}

/// One forbearance in the adopted minimum content: something the compact will not do
/// at its boundary, whatever anyone there has consented to. `key` is the stable
/// identifier amendments match on; `description` is for the reader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Forbearance {
    pub key: String,
    pub description: String,
    pub site: MinimumSite,
}

/// The compact's adoption of the functional minimum as the ground of its own conduct
/// where consent has run out (natural_law.md "The ground the compact claims for
/// itself"), quorum-ratified. There is **exactly one** adoption per archive; a second
/// is refused.
///
/// This is **not** an entry on the compact-incapable list and must never be filed as
/// one. That list names matters belonging to a *member* that the compact will not
/// decide; these forbearances restrain the compact's *own* conduct, which is the one
/// thing it has always been free to bind. "Recognizing a standard and holding a writ
/// are different things, and only the second would be a matter belonging to a member."
///
/// Unlike the genesis compact-incapable matters, a genesis-original forbearance is
/// **not** immutable: contracting the minimum content carries the asymmetric higher
/// bar and nothing more. The doctrine asks for a bar rather than a lock, because the
/// pressure it defends against is an argument that is "always available and always
/// pragmatic" rather than an argument that is never legitimate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimumContentAdoption {
    pub forbearances: Vec<Forbearance>,
    /// The adopting communities (each co-signs); a distinct-community quorum.
    pub adopters: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub adopted_at: Timestamp,
}

/// Whether an amendment to the minimum content adds a forbearance or removes one.
/// Narrowing is the capture-risk direction and carries the higher bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ForbearanceChange {
    /// Add a forbearance to the adopted ground (expansion — easy).
    Broaden,
    /// Remove one (contraction — hard, at every site and including genesis-original
    /// forbearances).
    Narrow,
}

impl ForbearanceChange {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Broaden => "Broaden",
            Self::Narrow => "Narrow",
        }
    }
    /// Contraction (the capture-risk direction) carries the higher quorum bar.
    pub fn is_contraction(&self) -> bool {
        matches!(self, Self::Narrow)
    }
}

/// A second-class amendment to the adopted minimum content. Co-signed by the
/// deciders. `Broaden` requires the normal distinct-community quorum; `Narrow`
/// requires the higher contraction bar.
///
/// The bar is doing work member conviction cannot: nobody in the compact is asked to
/// endorse this ground, so "when the argument for narrowing the forbearance owed at
/// the frontier arrives, no member holds the minimum content as its own commitment to
/// defend" (natural_law.md, limit 4). The protection is procedural for that reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinimumContentAmendment {
    pub change: ForbearanceChange,
    pub forbearance: Forbearance,
    pub rationale: String,
    pub deciders: Vec<DidKey>,
    pub decision_class: DecisionClass,
    pub decided_at: Timestamp,
}
