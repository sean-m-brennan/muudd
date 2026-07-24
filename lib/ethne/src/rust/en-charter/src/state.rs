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

//! Derived charter state: the substrate, roll, in-force commitments, and office
//! holdings of a polity as of a point in time.

use en_archive::Archive;
use en_core::{DidKey, Timestamp};

use crate::entry::CharterEntry;
use crate::record::{
    AffirmationKind, AmendmentKind, Commitment, MembershipKind, Office, OfficeKind, Substrate,
};

/// The state of a polity's charter as of a query time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharterState {
    /// True once the polity has been founded (a genesis charter exists).
    pub chartered: bool,
    pub substrate: Option<Substrate>,
    /// The current membership roll = the founding roll + admissions − departures
    /// (voluntary or by expulsion), applied in order up to `as_of`.
    pub members: Vec<DidKey>,
    /// Commitments in force = founding commitments + Adds − Removes (≤ `as_of`).
    pub commitments: Vec<Commitment>,
    pub founded_at: Option<Timestamp>,
    /// True once the polity has been dissolved (a dissolution record ≤ `as_of`
    /// exists). A dissolved polity has gone quiet — its line continues, if at
    /// all, through `successors`.
    pub dissolved: bool,
    /// The successor polities named by the dissolution (two for a fission).
    pub successors: Vec<DidKey>,
    pub dissolved_at: Option<Timestamp>,
}

/// Compute a polity's charter state as of `as_of`. The genesis charter fixes the
/// substrate, founding roll, and founding commitments; commitment amendments and
/// membership changes up to `as_of` are then applied in insertion order (Add
/// appends / Remove drops a commitment by key; Admit appends / Depart|Expel drops
/// a member); finally a dissolution winds the polity up.
pub fn charter_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> CharterState {
    let mut state = CharterState::default();

    // 1. Genesis: the first charter for this polity (a singleton in a valid archive).
    for (_cid, e) in archive.iter() {
        if let CharterEntry::PolityChartered(s) = e {
            if &s.payload.polity == polity && s.payload.founded_at <= as_of {
                state.chartered = true;
                state.substrate = Some(s.payload.substrate.clone());
                state.members = s.payload.members.clone();
                state.commitments = s.payload.founding_commitments.clone();
                state.founded_at = Some(s.payload.founded_at);
                break;
            }
        }
    }
    if !state.chartered {
        return state;
    }

    // 2. Apply commitment amendments in insertion order, up to `as_of`.
    for (_cid, e) in archive.iter() {
        if let CharterEntry::CommitmentAmended(s) = e {
            let a = &s.payload;
            if &a.polity != polity || a.decided_at > as_of {
                continue;
            }
            match a.kind {
                AmendmentKind::Add => {
                    if !state.commitments.iter().any(|c| c.key == a.commitment.key) {
                        state.commitments.push(a.commitment.clone());
                    }
                }
                AmendmentKind::Remove => {
                    state.commitments.retain(|c| c.key != a.commitment.key);
                }
            }
        }
    }

    // 3. Apply membership changes in insertion order, up to `as_of`.
    for (_cid, e) in archive.iter() {
        if let CharterEntry::MembershipChanged(s) = e {
            let m = &s.payload;
            if &m.polity != polity || m.decided_at > as_of {
                continue;
            }
            match m.kind {
                MembershipKind::Admit => {
                    if !state.members.contains(&m.member) {
                        state.members.push(m.member.clone());
                    }
                }
                MembershipKind::DepartVoluntary | MembershipKind::Expel => {
                    state.members.retain(|d| d != &m.member);
                }
            }
        }
    }

    // 4. A dissolution ≤ `as_of` winds the polity up and records its successors.
    for (_cid, e) in archive.iter() {
        if let CharterEntry::CharterDissolved(s) = e {
            let d = &s.payload;
            if &d.polity == polity && d.dissolved_at <= as_of {
                state.dissolved = true;
                state.successors = d.successors.clone();
                state.dissolved_at = Some(d.dissolved_at);
                break;
            }
        }
    }

    state
}

/// One (office, holder) pair currently in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeHolding {
    pub office: Office,
    pub holder: DidKey,
}

/// The offices a polity's members hold as of a query time — the *effective*
/// holdings (a conferral whose holder has left the roll is not included; see
/// [`office_state`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OfficeState {
    pub holdings: Vec<OfficeHolding>,
}

impl OfficeState {
    /// Whether `who` currently holds `office`.
    pub fn holds(&self, who: &DidKey, office: &Office) -> bool {
        self.holdings
            .iter()
            .any(|h| &h.holder == who && &h.office == office)
    }

    /// Everyone currently holding `office` (a role may have several holders).
    pub fn holders(&self, office: &Office) -> Vec<&DidKey> {
        self.holdings
            .iter()
            .filter(|h| &h.office == office)
            .map(|h| &h.holder)
            .collect()
    }

    /// Every office `who` currently holds.
    pub fn offices_of(&self, who: &DidKey) -> Vec<&Office> {
        self.holdings
            .iter()
            .filter(|h| &h.holder == who)
            .map(|h| &h.office)
            .collect()
    }
}

/// One current member's affirmation status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAffirmation {
    pub member: DidKey,
    /// True once the member has acquired the substrate — a founding member (by
    /// ratifying the charter) or one with an `Induction` record.
    pub inducted: bool,
    /// When the member last affirmed the substrate (a founding member's seed is
    /// `founded_at`); `None` if never inducted.
    pub last_affirmed_at: Option<Timestamp>,
}

/// The affirmation status of a polity's current members as of a query time — the
/// "transmission" read-model (design §5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AffirmationState {
    pub entries: Vec<MemberAffirmation>,
}

impl AffirmationState {
    /// Whether `who` has been inducted into the substrate.
    pub fn is_inducted(&self, who: &DidKey) -> bool {
        self.entries.iter().any(|e| &e.member == who && e.inducted)
    }

    /// When `who` last affirmed the substrate, if ever.
    pub fn last_affirmed(&self, who: &DidKey) -> Option<Timestamp> {
        self.entries
            .iter()
            .find(|e| &e.member == who)
            .and_then(|e| e.last_affirmed_at)
    }

    /// Current members whose affirmation has gone stale — never inducted, or last
    /// affirmed more than `cadence_seconds` before `as_of`. This is the charter-
    /// side erosion signal (the analogue of KC's ceremony-cost decay).
    pub fn stale_members(&self, cadence_seconds: i64, as_of: Timestamp) -> Vec<DidKey> {
        self.entries
            .iter()
            .filter(|e| match e.last_affirmed_at {
                Some(last) if e.inducted => as_of.as_second() - last.as_second() > cadence_seconds,
                _ => true,
            })
            .map(|e| e.member.clone())
            .collect()
    }
}

/// Compute the affirmation status of a polity's current members as of `as_of`.
/// Founding members are seeded as inducted at `founded_at` (ratifying the charter
/// is their induction); every `MemberAffirmed` record ≤ `as_of` then sets the
/// induction flag (on `Induction`) and advances `last_affirmed_at`. Only current
/// members ([`charter_state`]) are reported.
pub fn affirmation_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> AffirmationState {
    // Genesis: founding roll + founding instant.
    let mut founding: Vec<DidKey> = Vec::new();
    let mut founded_at: Option<Timestamp> = None;
    for (_cid, e) in archive.iter() {
        if let CharterEntry::PolityChartered(s) = e {
            if &s.payload.polity == polity && s.payload.founded_at <= as_of {
                founding = s.payload.members.clone();
                founded_at = Some(s.payload.founded_at);
                break;
            }
        }
    }

    // Seed each current member: founding members are inducted at `founded_at`.
    let members = charter_state(archive, polity, as_of).members;
    let mut entries: Vec<MemberAffirmation> = members
        .iter()
        .map(|m| {
            let is_founding = founding.contains(m);
            MemberAffirmation {
                member: m.clone(),
                inducted: is_founding,
                last_affirmed_at: if is_founding { founded_at } else { None },
            }
        })
        .collect();

    // Apply affirmations in order, up to `as_of`.
    for (_cid, e) in archive.iter() {
        if let CharterEntry::MemberAffirmed(s) = e {
            let aff = &s.payload;
            if &aff.polity != polity || aff.affirmed_at > as_of {
                continue;
            }
            if let Some(entry) = entries.iter_mut().find(|x| x.member == aff.member) {
                if aff.kind == AffirmationKind::Induction {
                    entry.inducted = true;
                }
                entry.last_affirmed_at = Some(match entry.last_affirmed_at {
                    Some(prev) if prev >= aff.affirmed_at => prev,
                    _ => aff.affirmed_at,
                });
            }
        }
    }

    AffirmationState { entries }
}

/// Compute a polity's **effective** office holdings as of `as_of`. Office changes
/// are applied in insertion order (Confer adds an `(office, holder)` pair; Resign
/// and Revoke drop it); the result is then intersected with the current
/// membership roll ([`charter_state`]), so an office conferred on a member who has
/// since left the roll is not reported — it lapses while they are away and revives
/// only if they return. Holdings are returned sorted by `(office, holder)` for a
/// deterministic order.
pub fn office_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> OfficeState {
    // Raw grants: Confer adds, Resign/Revoke removes the matching pair.
    let mut raw: Vec<OfficeHolding> = Vec::new();
    for (_cid, e) in archive.iter() {
        if let CharterEntry::OfficeChanged(s) = e {
            let o = &s.payload;
            if &o.polity != polity || o.decided_at > as_of {
                continue;
            }
            let matches = |h: &OfficeHolding| h.office == o.office && h.holder == o.holder;
            match o.kind {
                OfficeKind::Confer => {
                    if !raw.iter().any(matches) {
                        raw.push(OfficeHolding { office: o.office.clone(), holder: o.holder.clone() });
                    }
                }
                OfficeKind::Resign | OfficeKind::Revoke => {
                    raw.retain(|h| !matches(h));
                }
            }
        }
    }

    // Effective = raw grants whose holder is still on the current roll.
    let members = charter_state(archive, polity, as_of).members;
    raw.retain(|h| members.contains(&h.holder));
    raw.sort_by(|a, b| (&a.office, a.holder.as_str()).cmp(&(&b.office, b.holder.as_str())));

    OfficeState { holdings: raw }
}
