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

//! Derived charter state: the substrate, roll, in-force commitments, office
//! holdings, guardian edges, and envoy standings of a polity as of a point in time,
//! plus the decay trends visible across its whole log.

use std::collections::{BTreeSet, HashMap};

use en_archive::Archive;
use en_core::{Cid, DidKey, KcResult, PubKey, Timestamp};

use crate::entry::CharterEntry;
use crate::record::{
    AffirmationKind, AmendmentKind, BoundarySource, Commitment, Derivation, DissentKind, EnvoyKind,
    Exclusion, GuardianKind,
    MemberReading, MembershipKind, Office, OfficeKind, Substrate,
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

/// Which members are derived (machine) parties and the human each answers to, as
/// of a query time — the *effective* designations only (see [`guardian_state`]).
/// This is the read-model the machine-majority office rule and `en-cohesion`'s
/// guardian-edge component both read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuardianState {
    pub derivations: Vec<Derivation>,
}

impl GuardianState {
    /// Whether `who` is a designated derived (machine) party.
    pub fn is_derived(&self, who: &DidKey) -> bool {
        self.derivations.iter().any(|d| &d.member == who)
    }

    /// The human `who` answers to: `Some(None)` is a *designated* member whose
    /// guardian edge is absent — a machine with nobody behind it — as against
    /// `None`, which means `who` is not a derived party at all.
    pub fn guardian_of(&self, who: &DidKey) -> Option<&Option<DidKey>> {
        self.derivations.iter().find(|d| &d.member == who).map(|d| &d.guardian)
    }

    /// Derived members that do have a guardian — the edges the substrate is
    /// expected to keep showing.
    pub fn guarded_members(&self) -> Vec<&DidKey> {
        self.derivations
            .iter()
            .filter(|d| d.guardian.is_some())
            .map(|d| &d.member)
            .collect()
    }

    /// Derived members with no guardian: chartered machines answering to nobody.
    pub fn unguarded_members(&self) -> Vec<&DidKey> {
        self.derivations
            .iter()
            .filter(|d| d.guardian.is_none())
            .map(|d| &d.member)
            .collect()
    }

    /// Whether derived members are at least a majority of a roll of `roll_len` —
    /// the condition under which every office must trace to a human (D15).
    pub fn is_machine_majority(&self, roll_len: usize) -> bool {
        roll_len > 0 && self.derivations.len() >= crate::majority(roll_len)
    }
}

/// One envoy standing currently in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvoyHolding {
    pub envoy: DidKey,
    /// The envoy's public key, as the authorization recorded it — the KC-side
    /// delegation names its speaker by key.
    pub envoy_pubkey: PubKey,
    /// When the standing lapses of its own accord, if ever.
    pub expires_at: Option<Timestamp>,
    pub authorized_at: Timestamp,
}

/// Who may speak for the polity beyond it, as of a query time — the *effective*
/// standings only (see [`envoy_state`]). This is the read-model `en-uplift` turns
/// into kith-covenant's envoy delegation, which is why it carries the key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvoyState {
    pub standings: Vec<EnvoyHolding>,
}

impl EnvoyState {
    /// Whether `who` may currently speak for the polity.
    pub fn is_active(&self, who: &DidKey) -> bool {
        self.standings.iter().any(|h| &h.envoy == who)
    }

    /// The standing `who` holds, if any — the shape `en-uplift` needs (the key and
    /// the expiry travel to KC with the delegation).
    pub fn standing(&self, who: &DidKey) -> Option<&EnvoyHolding> {
        self.standings.iter().find(|h| &h.envoy == who)
    }

    /// Everyone who may currently speak for the polity (a polity may have several).
    pub fn envoys(&self) -> Vec<&DidKey> {
        self.standings.iter().map(|h| &h.envoy).collect()
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

/// Who may speak for the polity beyond it as of `as_of` — raw standings
/// (`Authorize` − `Resign`/`Revoke`) intersected with the current roll and with
/// the standings that have not expired.
///
/// The roll intersection mirrors [`office_state`]: a standing lapses while its
/// envoy is off the roll and revives if they return, because speaking for a
/// polity one has left is exactly what should not be possible. Expiry is checked
/// here rather than in the authority pass, since a lapsed standing was still
/// validly granted.
pub fn envoy_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> EnvoyState {
    let mut raw: Vec<EnvoyHolding> = Vec::new();
    for (_cid, e) in archive.iter() {
        if let CharterEntry::EnvoyAuthorized(s) = e {
            let ea = &s.payload;
            if &ea.polity != polity || ea.decided_at > as_of {
                continue;
            }
            match ea.kind {
                EnvoyKind::Authorize => {
                    if !raw.iter().any(|h| h.envoy == ea.envoy) {
                        raw.push(EnvoyHolding {
                            envoy: ea.envoy.clone(),
                            envoy_pubkey: ea.envoy_pubkey.clone(),
                            expires_at: ea.expires_at,
                            authorized_at: ea.decided_at,
                        });
                    }
                }
                EnvoyKind::Resign | EnvoyKind::Revoke => {
                    raw.retain(|h| h.envoy != ea.envoy);
                }
            }
        }
    }

    // Effective = granted, the envoy still on the roll, and not yet expired.
    let members = charter_state(archive, polity, as_of).members;
    raw.retain(|h| {
        members.contains(&h.envoy) && h.expires_at.map(|exp| as_of <= exp).unwrap_or(true)
    });
    raw.sort_by(|a, b| a.envoy.as_str().cmp(b.envoy.as_str()));

    EnvoyState { standings: raw }
}

/// Which members are derived parties and whom they answer to as of `as_of` — the
/// charter's founding derivations plus every `GuardianChange` in order, intersected
/// with the current roll.
///
/// The roll intersection mirrors [`office_state`] and [`envoy_state`]: a
/// designation lapses while its member is off the roll and revives if they return.
/// A `Release` clears the *edge* but keeps the designation — a machine does not
/// become human by losing its human, and that distinction is exactly what the
/// office rule and the cohesion monitor read.
pub fn guardian_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> GuardianState {
    let mut raw: Vec<Derivation> = Vec::new();
    for (_cid, e) in archive.iter() {
        match e {
            CharterEntry::PolityChartered(s)
                if &s.payload.polity == polity && s.payload.founded_at <= as_of =>
            {
                for d in &s.payload.derivations {
                    if !raw.iter().any(|h| h.member == d.member) {
                        raw.push(d.clone());
                    }
                }
            }
            CharterEntry::GuardianChanged(s) if &s.payload.polity == polity => {
                let gc = &s.payload;
                if gc.decided_at > as_of {
                    continue;
                }
                match gc.kind {
                    GuardianKind::Designate => {
                        if !raw.iter().any(|d| d.member == gc.member) {
                            raw.push(Derivation {
                                member: gc.member.clone(),
                                guardian: gc.guardian.clone(),
                            });
                        }
                    }
                    GuardianKind::Rotate => {
                        for d in raw.iter_mut().filter(|d| d.member == gc.member) {
                            d.guardian = gc.guardian.clone();
                        }
                    }
                    GuardianKind::Release => {
                        for d in raw.iter_mut().filter(|d| d.member == gc.member) {
                            d.guardian = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Effective = designated, with the member still on the roll.
    let members = charter_state(archive, polity, as_of).members;
    raw.retain(|d| members.contains(&d.member));
    raw.sort_by(|a, b| a.member.as_str().cmp(b.member.as_str()));

    GuardianState { derivations: raw }
}

/// One affirmation in the polity's series, for trend analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffirmationPoint {
    pub record_cid: Cid,
    pub at: Timestamp,
    /// Witnesses who received the inductee. `0` for a re-affirmation, which is
    /// self-signed by design (D13) — so witness trends read inductions only.
    pub witness_count: usize,
    pub kind: AffirmationKind,
}

/// One roll change in the polity's series.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnPoint {
    pub record_cid: Cid,
    pub at: Timestamp,
    pub kind: MembershipKind,
}

/// The polity's **decay trends**: not a state but a shape over time — is the
/// substrate being re-affirmed less often, is the community turning out less to
/// receive new members, is the roll churning?
///
/// This borrows the shape of kith-covenant's
/// `kc_reputation::analytics::ceremony_cost_decay`, one tier up, including its two
/// hedges: a witness decline needs at least two points (never fabricate a trend
/// from a single reading), and a widening gap is judged against the **median** of
/// the prior gaps rather than the previous one, so a single long interval does not
/// read as decay.
///
/// Nothing here is derived from a substrate provider — every point comes from the
/// polity's own signed records, so trends are as auditable as the acts they count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecayTrends {
    /// Every affirmation up to `as_of`, oldest first.
    pub affirmations: Vec<AffirmationPoint>,
    /// Successive `at` deltas in seconds across `affirmations`
    /// (`len == affirmations.len() - 1`, empty with fewer than two).
    pub gaps_seconds: Vec<i64>,
    /// Seconds from the last affirmation to `as_of`. `None` with no affirmations.
    pub seconds_since_last: Option<i64>,
    /// Every roll change up to `as_of`, oldest first.
    pub membership_changes: Vec<ChurnPoint>,
}

impl DecayTrends {
    /// The inductions only — the rite that actually has witnesses.
    pub fn inductions(&self) -> Vec<&AffirmationPoint> {
        self.affirmations
            .iter()
            .filter(|p| p.kind == AffirmationKind::Induction)
            .collect()
    }

    /// Whether the most recent induction drew fewer witnesses than the first.
    /// Needs at least two inductions; `false` otherwise, so no trend is invented
    /// from one data point.
    pub fn witness_count_declined(&self) -> bool {
        let inductions = self.inductions();
        match (inductions.first(), inductions.last()) {
            (Some(f), Some(l)) if inductions.len() >= 2 => l.witness_count < f.witness_count,
            _ => false,
        }
    }

    /// `Some(true)` when the latest affirmation gap exceeds the **first** gap — the
    /// *pinned* counterpart to [`Self::latest_gap_widening`]. Needs at least two
    /// gaps; `None` otherwise.
    ///
    /// The two answer different questions and both are worth asking. The rolling
    /// reading asks whether *this* gap is unusual for this polity as it now behaves.
    /// This one asks whether the cadence has slipped since the polity began.
    ///
    /// The rolling reading catches the widening and then **goes quiet once the
    /// widened cadence has recurred often enough to dominate the median of the
    /// priors** — gaps of `[100, 400, 400, 400]` read `false`, because 400 is no
    /// longer unusual *for this polity*. That is shifting-baseline syndrome: the
    /// degraded cadence has become the reference it is judged against, and the
    /// signal falls silent exactly when the decay has finished setting. A baseline
    /// pinned to the founding cadence cannot be dragged along behind the drift,
    /// which is why [`Self::witness_count_declined`] is built the same way.
    pub fn gap_widening_since_founding(&self) -> Option<bool> {
        if self.gaps_seconds.len() < 2 {
            return None;
        }
        Some(*self.gaps_seconds.last().unwrap() > self.gaps_seconds[0])
    }

    /// `Some(true)` when the latest affirmation gap exceeds the **median** of the
    /// prior gaps — a *rolling* reading, judged against how this polity has lately
    /// behaved. Needs at least two gaps (three affirmations); `None` otherwise,
    /// since there is not yet enough cadence to compare against.
    ///
    /// See [`Self::gap_widening_since_founding`] for the pinned counterpart and why
    /// this reading alone is not enough.
    pub fn latest_gap_widening(&self) -> Option<bool> {
        if self.gaps_seconds.len() < 2 {
            return None;
        }
        let last = *self.gaps_seconds.last().unwrap();
        let mut prior: Vec<i64> = self.gaps_seconds[..self.gaps_seconds.len() - 1].to_vec();
        prior.sort_unstable();
        let m = prior.len();
        let median = if m % 2 == 1 {
            prior[m / 2] as f64
        } else {
            (prior[m / 2 - 1] + prior[m / 2]) as f64 / 2.0
        };
        Some((last as f64) > median)
    }

    /// Roll changes per member per `window_seconds` ending at `as_of` — the rate at
    /// which the boundary is being redrawn. `None` when the window is empty or the
    /// roll is, since a rate over nothing is not zero but unmeasured.
    pub fn churn_rate(
        &self,
        window_seconds: i64,
        roll_len: usize,
        as_of: Timestamp,
    ) -> Option<f64> {
        if window_seconds <= 0 || roll_len == 0 {
            return None;
        }
        let since = as_of.as_second() - window_seconds;
        let n = self
            .membership_changes
            .iter()
            .filter(|c| c.at.as_second() > since)
            .count();
        Some(n as f64 / roll_len as f64)
    }
}

/// The polity's decay trends as of `as_of`, read from its own signed records.
pub fn decay_trends(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> DecayTrends {
    let mut trends = DecayTrends::default();
    for (cid, e) in archive.iter() {
        match e {
            CharterEntry::MemberAffirmed(s)
                if &s.payload.polity == polity && s.payload.affirmed_at <= as_of =>
            {
                trends.affirmations.push(AffirmationPoint {
                    record_cid: *cid,
                    at: s.payload.affirmed_at,
                    witness_count: s.payload.witnesses.len(),
                    kind: s.payload.kind,
                });
            }
            CharterEntry::MembershipChanged(s)
                if &s.payload.polity == polity && s.payload.decided_at <= as_of =>
            {
                trends.membership_changes.push(ChurnPoint {
                    record_cid: *cid,
                    at: s.payload.decided_at,
                    kind: s.payload.kind,
                });
            }
            _ => {}
        }
    }
    trends.affirmations.sort_by_key(|p| p.at);
    trends.membership_changes.sort_by_key(|c| c.at);
    for w in trends.affirmations.windows(2) {
        trends
            .gaps_seconds
            .push(w[1].at.as_second() - w[0].at.as_second());
    }
    trends.seconds_since_last = trends
        .affirmations
        .last()
        .map(|p| as_of.as_second() - p.at.as_second());
    trends
}

/// One recorded observation, with who took it.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationPoint {
    pub record_cid: Cid,
    pub observer: DidKey,
    pub at: Timestamp,
    pub readings: Vec<MemberReading>,
    pub opaque_boundary: bool,
}

/// **What the polity's monitors saw, over an interval** (design **D16**) — the
/// read-model that lets a measure speak about duration.
///
/// A snapshot can say a guardian is absent; only this can say for how long, and only
/// this can say whether anyone was even watching while they were. Both matter: the
/// span is the absence, and the reading count is the evidence that the absence was
/// observed rather than merely unrecorded.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObservationWindow {
    /// The observations inside the window, oldest first.
    pub observations: Vec<ObservationPoint>,
    /// The pruning horizon biting inside this window, if one does (design **D17**):
    /// observations before it were dropped, so the window holds no evidence about
    /// that interval and must not speak as though it does. `None` when nothing was
    /// pruned, or when the pruning is older than the window's own `since` and so
    /// took nothing the caller asked about.
    pub pruned_before: Option<Timestamp>,
}

impl ObservationWindow {
    /// How many observations the window holds.
    pub fn reading_count(&self) -> usize {
        self.observations.len()
    }

    /// How many of them actually saw `who` — the per-member evidence of watching,
    /// which is what a duration measure should be held to. A monitor that observed
    /// ten times but saw this member once has watched them once.
    pub fn readings_for(&self, who: &DidKey) -> usize {
        self.observations
            .iter()
            .filter(|o| o.readings.iter().any(|r| &r.member == who))
            .count()
    }

    /// Seconds from the first observation to the last. `None` with fewer than two.
    pub fn span_seconds(&self) -> Option<i64> {
        match (self.observations.first(), self.observations.last()) {
            (Some(f), Some(l)) if self.observations.len() >= 2 => {
                Some(l.at.as_second() - f.at.as_second())
            }
            _ => None,
        }
    }

    /// The latest attendance stamp recorded for `who` anywhere in the window.
    pub fn last_attested(&self, who: &DidKey) -> Option<Timestamp> {
        self.observations
            .iter()
            .flat_map(|o| o.readings.iter())
            .filter(|r| &r.member == who)
            .filter_map(|r| r.attested_at)
            .max()
    }

    /// How long `who` has gone unattended, as of `as_of`.
    ///
    /// Three cases, deliberately distinguished: a member the window never saw
    /// returns `None` — *not evaluated*, since nobody was watching them. A member
    /// seen but never with a stamp is unattended since the window first saw them,
    /// because that is the whole interval over which a human failed to appear. A
    /// member seen with a stamp is unattended since that stamp.
    ///
    /// **Clamped at [`pruned_before`]** (design **D17**), which pins the invariant
    /// that *pruning may only ever weaken a signal, never strengthen one*. The clamp
    /// is not a precaution: [`last_attested`] is a `max` over the window's stamps, so
    /// if the pruned interval held a higher stamp than anything surviving, dropping it
    /// lowers that max, moves the start of the absence *earlier*, and lengthens the
    /// span — pruning manufacturing a Concern. Behind the horizon the log holds no
    /// evidence either way, and no absence may be claimed across it.
    ///
    /// [`pruned_before`]: ObservationWindow::pruned_before
    /// [`last_attested`]: ObservationWindow::last_attested
    pub fn unattended_span(&self, who: &DidKey, as_of: Timestamp) -> Option<i64> {
        let first_seen = self
            .observations
            .iter()
            .find(|o| o.readings.iter().any(|r| &r.member == who))
            .map(|o| o.at)?;
        let mut since = self.last_attested(who).unwrap_or(first_seen).as_second();
        if let Some(horizon) = self.pruned_before {
            since = since.max(horizon.as_second());
        }
        Some(as_of.as_second() - since)
    }
}

/// The observations of `polity` recorded in `[since, as_of]`, oldest first.
///
/// `since` bounds the window explicitly rather than defaulting to the whole log: a
/// monitor asking "has anyone been present today" must not be answered from last
/// year's evidence.
pub fn observation_window(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    since: Timestamp,
    as_of: Timestamp,
) -> ObservationWindow {
    let mut observations: Vec<ObservationPoint> = Vec::new();
    for (cid, e) in archive.iter() {
        if let CharterEntry::Observed(s) = e {
            let ob = &s.payload;
            if &ob.polity != polity || ob.observed_at < since || ob.observed_at > as_of {
                continue;
            }
            observations.push(ObservationPoint {
                record_cid: *cid,
                observer: ob.observer.clone(),
                at: ob.observed_at,
                readings: ob.readings.clone(),
                opaque_boundary: ob.opaque_boundary,
            });
        }
    }
    observations.sort_by_key(|o| o.at);
    // A pruning older than the window's own start took nothing the caller asked
    // about, so it is not a truncation of *this* window.
    let pruned_before = effective_prune_horizon(archive, polity, as_of).filter(|h| *h > since);
    ObservationWindow { observations, pruned_before }
}

/// The furthest-forward observation-pruning horizon in force for `polity` as of
/// `as_of`, or `None` if its observation log has never been pruned.
pub fn effective_prune_horizon(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> Option<Timestamp> {
    archive
        .iter()
        .filter_map(|(_, e)| match e {
            CharterEntry::ObservationsPruned(s)
                if &s.payload.polity == polity && s.payload.pruned_at <= as_of =>
            {
                Some(s.payload.horizon)
            }
            _ => None,
        })
        .max()
}

/// Whether `entry` is an observation of `polity` that a pruning to `horizon` drops.
///
/// **The single definition of what a pruning takes.** The authoring side builds its
/// receipt from it, the authority pass checks the receipt against it, and
/// [`apply_prunings`] removes exactly what it matches — so the three can never
/// disagree about what was dropped. It is also where the guarantee lives that a
/// pruning cannot reach a governing record: nothing but
/// [`CharterEntry::Observed`] can satisfy it.
pub fn is_prunable(entry: &CharterEntry, polity: &DidKey, horizon: Timestamp) -> bool {
    match entry {
        CharterEntry::Observed(s) => {
            &s.payload.polity == polity && s.payload.observed_at < horizon
        }
        // Everything else, and deliberately so. Worth naming the two cases that
        // look inert and are not: a `BoundaryDissented` (D22) and a
        // `SubstrateDisavowed` (D23) both fall here and are therefore **unprunable
        // for free**. D17 made observations the one forgettable class precisely
        // because they are inert — dropping one cannot rewrite what the polity
        // resolved. These two are the opposite of inert: one is the excluded party's
        // only mark on a record they had no hand in, the other the ground's only way
        // to say it does not stand there. A pruning route to either would hand the
        // founders the erasure both records exist to prevent.
        _ => false,
    }
}

/// The observations of `polity` a pruning to `horizon` would drop, in append
/// order — the input to [`pruning_digest`](crate::pruning_digest).
pub fn prunable_observations(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    horizon: Timestamp,
) -> Vec<Cid> {
    archive
        .iter()
        .filter(|(_, e)| is_prunable(e, polity, horizon))
        .map(|(cid, _)| *cid)
        .collect()
}

/// Apply `polity`'s ratified prunings, returning a **new** archive without the
/// observations they drop (design **D17**).
///
/// This is compaction, and it is deliberately not a `retain` on `en-archive`. The
/// archive is the neutral append-only *mechanism*; giving it a removal path would
/// hand one to every entry class that will ever live in it. Here the knowledge that
/// only observations may go is where the domain knowledge belongs, and the function
/// **cannot drop a governing record, because it only ever matches
/// [`CharterEntry::Observed`]** — every other entry is copied through untouched.
///
/// CIDs are content addresses, so rebuilding preserves them: the surviving entries
/// keep the identities they had, and the pruning records stay in the log to say what
/// went.
pub fn apply_prunings(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> KcResult<Archive<CharterEntry>> {
    let horizon = effective_prune_horizon(archive, polity, as_of);
    // A *new generation*, not merely a new archive: entries are being removed, so
    // every cursor a host holds into the old one is now meaningless and must say so
    // (D20). Without this the host's next incremental save silently writes nothing
    // and its storage keeps the observations this pruning just dropped.
    let mut out = Archive::next_generation(archive);
    for (_, e) in archive.iter() {
        if horizon.is_some_and(|h| is_prunable(e, polity, h)) {
            continue;
        }
        out.append(e.clone())?;
    }
    Ok(out)
}


// ---------------------------------------------------------------------------
// D19 — the retention policy over the observation log.
// ---------------------------------------------------------------------------

/// How long a polity keeps its observations, and how much must have aged out
/// before dropping them is worth a record (design **D19**).
///
/// A policy, never a schedule. This tier's libraries do not call `now()` — every
/// cadence is a query-time parameter — so the loop and the clock stay with the host,
/// exactly as the storage backend does. What is *not* the host's to know is which
/// history a live measure still depends on and which authorization route a given
/// horizon leaves open; that is what [`prune_recommendation`] answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// How much observation history to keep, counting back from the query time.
    ///
    /// Defaults to [`crate::PRUNE_SETTLING_SECONDS`] (30 days), which is the
    /// smallest value that does two things at once: it keeps every
    /// observation-based measure intact (the longest is `en_cohesion`'s
    /// `sustained_absence_seconds`, one day), and it keeps the **single
    /// `CharterKeeper`** route open, since that route requires a horizon at least
    /// `PRUNE_SETTLING_SECONDS` old. The safe default is therefore also the cheap
    /// one to authorize — and, as it happens, the cheap one to store: a
    /// keeper-authorized pruning carries one signature where a roll-majority one
    /// carries a majority of them.
    ///
    /// A shorter retention is permitted and forces the roll route. It can never clip
    /// a live measure, because [`prune_recommendation`] floors the horizon at
    /// *both* `live_measure_seconds` and `min_witness_readings` regardless of what is
    /// asked for — so the horizon may end up keeping more than was requested, never
    /// less.
    pub retain_seconds: i64,
    /// How many observations must have aged past the horizon before pruning them is
    /// worth doing.
    ///
    /// **This exists because a pruning record is itself unprunable.** Observations
    /// are the one entry class a polity may drop (see [`is_prunable`]); the pruning
    /// records that drop them are governing records and stay forever. Measured: a
    /// keeper-authorized pruning is ~2.9 KB regardless of roll size, against
    /// 3.4–8.0 KB for one observation — so pruning *per observation* would trade a
    /// reclaimable few kilobytes for a permanent three, which is worse the longer
    /// the polity runs, not better.
    ///
    /// The default of 100 keeps that permanent residue under about 3 MiB/year even
    /// at the fastest realistic observation rate (25 members on a 300-second
    /// attendance cadence, ~105k observations/year), against ~66 MiB retained — and
    /// stays small enough to fire promptly on a slow cadence, where a large floor
    /// would let the log run well past its retention target before anything
    /// triggered.
    pub min_dropped: usize,
    /// The longest span any live measure reads back over — one of the two floors the
    /// horizon can never cross, whatever `retain_seconds` asks for. It protects the
    /// *span* half of a sustained absence; `min_witness_readings` protects the other.
    ///
    /// Defaults to one day, matching `en_cohesion`'s `sustained_absence_seconds`.
    /// It is a *separate* knob from `retain_seconds` because the two are different
    /// claims: one is how much history the polity wants to keep, the other is how
    /// much its own grading still needs. Collapsing them would let a caller tune
    /// away a safety property while believing they were only tuning storage.
    pub live_measure_seconds: i64,
    /// How many observations *of each watched member* must survive — the second half
    /// of the safety floor, and the half a duration cannot supply.
    ///
    /// A sustained absence is graded on two conditions, and `live_measure_seconds`
    /// only protects one of them. The span survives a prune (D17 clamps it to the
    /// horizon), but the **witness count** does not: prune to a 24-hour horizon on a
    /// polity observed twice a day and `readings_for` falls from four to two, so a
    /// `Concern` silently becomes a `Notice` with no signal that evidence was what
    /// changed. Measured, not assumed — the same absence, the same horizon, graded
    /// `Concern` at a dense cadence and `Notice` at a sparse one.
    ///
    /// So the floor is a duration **and** a count. Defaults to 3, matching
    /// `en_cohesion`'s `sustained_absence_readings`. The count is per member, not per
    /// log, because that is what `readings_for` counts: a member the monitors mention
    /// rarely has thin evidence, and thinning it further is exactly the clip this
    /// read-model exists to refuse.
    pub min_witness_readings: usize,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy {
            retain_seconds: crate::PRUNE_SETTLING_SECONDS,
            min_dropped: 100,
            live_measure_seconds: 86_400,
            min_witness_readings: 3,
        }
    }
}

/// Who can authorize a recommended pruning. Both are reported when both are open —
/// the officer route is an *alternative* to the roll, never a gate on it (D17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PruneRoute {
    /// One of these effective `CharterKeeper` holders may authorize it alone. Only
    /// offered when the horizon is at least `PRUNE_SETTLING_SECONDS` old, because
    /// that is the condition D17's authority pass applies.
    CharterKeeper { holders: Vec<DidKey> },
    /// This many members of the current roll must co-sign.
    RollMajority { need: usize },
}

/// A pruning the polity could run now: what to drop, and who may authorize it.
///
/// Advisory only. It signs nothing and appends nothing; the host feeds `horizon`
/// to the ordinary authoring path. "Emergence proposes; the polity ratifies" holds
/// for the polity's own housekeeping too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneRecommendation {
    /// Observations strictly before this are dropped. Guaranteed to advance past any
    /// horizon already in force, and never to reach within
    /// `live_measure_seconds` of `as_of`.
    pub horizon: Timestamp,
    /// How many observations it would drop, at or above `min_dropped`.
    pub dropped: usize,
    /// Seconds of history the polity keeps if it acts on this.
    pub seconds_retained: i64,
    /// Every authorization route open at this horizon, cheapest first.
    pub routes: Vec<PruneRoute>,
}

impl PruneRecommendation {
    /// Whether a single `CharterKeeper` can authorize this without the roll.
    pub fn keeper_may_authorize(&self) -> bool {
        self.routes
            .iter()
            .any(|r| matches!(r, PruneRoute::CharterKeeper { .. }))
    }
}

/// The latest horizon that still leaves `min_readings` observations of **every**
/// watched member standing, or `None` when some member has too few to spare.
///
/// This is the half of the safety floor a duration cannot supply. `readings_for`
/// counts observations mentioning a member, so the binding constraint is whichever
/// member is mentioned most sparsely: the horizon may not advance past their
/// `min_readings`-th most recent observation. A polity whose monitors cover the
/// whole roll every time is unaffected — the duration floor binds first — while one
/// with a thinly-watched member is pinned, which is the correct answer and not a
/// deadlock: D17 always leaves the roll free to prune anything by majority. What is
/// refused is *recommending* it.
fn witness_floor(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
    min_readings: usize,
) -> Option<Timestamp> {
    if min_readings == 0 {
        return Some(as_of);
    }
    let mut per_member: HashMap<&DidKey, Vec<i64>> = HashMap::new();
    for (_, entry) in archive.iter() {
        if let CharterEntry::Observed(s) = entry {
            if &s.payload.polity != polity || s.payload.observed_at > as_of {
                continue;
            }
            for reading in &s.payload.readings {
                per_member
                    .entry(&reading.member)
                    .or_default()
                    .push(s.payload.observed_at.as_second());
            }
        }
    }
    let mut floor = as_of.as_second();
    for (_, mut times) in per_member {
        if times.len() < min_readings {
            return None; // nothing may go: this member has no evidence to spare
        }
        times.sort_unstable();
        floor = floor.min(times[times.len() - min_readings]);
    }
    Timestamp::from_second(floor).ok()
}

/// What pruning `polity`'s observation log would take right now under `policy`, or
/// `None` when there is nothing worth doing (design **D19**).
///
/// **The recommendation can only ever propose something the authority pass would
/// accept**, which is the point of it existing at all — a host choosing a horizon by
/// hand can silently clip the evidence a kith-covenant Concern rests on. Five
/// conditions, each deferring to the rule that already owns it rather than
/// restating it:
///
/// 1. The polity must be chartered and not dissolved (nothing to keep otherwise).
/// 2. The horizon is floored at `as_of - live_measure_seconds`, so a graded **span**
///    can never be shortened by acting on this. A `retain_seconds` shorter than that
///    floor is honoured *up to* it and no further.
/// 3. It is floored again at the *witness floor*, so a graded **witness count** cannot
///    be shortened either. Both floors are needed: a sustained absence is two
///    conditions, and a duration only protects one of them.
/// 4. The horizon must **advance** past [`effective_prune_horizon`] — D17 rejects a
///    receding one, which would claim to drop what an earlier pruning already did.
/// 5. At least `min_dropped` observations must actually fall before it.
///
/// The routes come from the live office holdings ([`office_state`], already
/// intersected with the roll, so a departed keeper is not offered) and from
/// [`majority`](crate::majority) over the current roll.
pub fn prune_recommendation(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    policy: &RetentionPolicy,
    as_of: Timestamp,
) -> Option<PruneRecommendation> {
    let state = charter_state(archive, polity, as_of);
    if !state.chartered || state.dissolved {
        return None;
    }

    // The floor is not a preference: whatever retention is asked for, the horizon
    // stops short of the history the polity's own grading still reads. Two floors,
    // because a sustained absence is graded on a span *and* on a witness count.
    let retain = policy.retain_seconds.max(policy.live_measure_seconds);
    let by_duration = Timestamp::from_second(as_of.as_second().checked_sub(retain)?).ok()?;
    let by_witness = witness_floor(archive, polity, as_of, policy.min_witness_readings)?;
    let horizon = by_duration.min(by_witness);

    if let Some(prev) = effective_prune_horizon(archive, polity, as_of) {
        if horizon <= prev {
            return None;
        }
    }

    let dropped = prunable_observations(archive, polity, horizon).len();
    if dropped < policy.min_dropped {
        return None;
    }

    let mut routes = Vec::new();
    // The officer route, on exactly D17's condition.
    if horizon.as_second() <= as_of.as_second() - crate::PRUNE_SETTLING_SECONDS {
        let holders: Vec<DidKey> = office_state(archive, polity, as_of)
            .holders(&Office::CharterKeeper)
            .into_iter()
            .cloned()
            .collect();
        if !holders.is_empty() {
            routes.push(PruneRoute::CharterKeeper { holders });
        }
    }
    routes.push(PruneRoute::RollMajority {
        need: crate::majority(state.members.len()),
    });

    Some(PruneRecommendation {
        horizon,
        dropped,
        seconds_retained: as_of.as_second() - horizon.as_second(),
        routes,
    })
}


// ---------------------------------------------------------------------------
// D21 — how a founding stands, read off the charter alone.
// ---------------------------------------------------------------------------

/// How a charter's boundary came to be what it is. **Descriptive, deliberately not
/// ordered, and deliberately not `en-cohesion`'s `ErosionLevel`** —
/// a founding is not *eroding*, and these are not comparable harms. A narrow
/// founding may be entirely legitimate; a self-attested one may be the only kind
/// available. What the tier refuses to do is pretend it cannot tell them apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoundingBasis {
    /// An independent reading, and a majority of what it showed ratified.
    Corroborated,
    /// An independent reading, but the ratifiers are a minority of what it showed.
    Narrowed,
    /// The convener's own assertion is the only account of this boundary.
    SelfAttested,
}

/// What stands above a polity, read off its charter alone (design **D23**,
/// resolving open question **Q4**).
///
/// Like [`FoundingBasis`], descriptive and deliberately **not** ordered: these are
/// different kinds of ground, not degrees of one. `Bare` is not a violation — a
/// human community answering to something with no key is the ordinary case, and
/// D23's authority rules permit it. What the tier refuses is to be unable to say
/// which of these it is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubstrateGrounding {
    /// A keyed party outside the roll consented to standing above this polity: the
    /// strongest grounding available here, because it is the only one somebody
    /// signed.
    Anchored,
    /// No anchor, but human guardians stand above the machines on the roll — D15's
    /// derived-party edges doing the grounding.
    Guarded,
    /// No derived party on the roll at all — a human community holding an
    /// articulated tradition. Nobody keyed above it, and none need be: what such a
    /// community defers to may have no key, and usually does not.
    Held,
    /// Articulated, and nothing else: no anchor, unguarded machines on the roll, and
    /// no guardian edge anywhere. Reachable only on a *mixed* roll, since a wholly
    /// machine roll answering to nobody is refused outright — so somebody here can
    /// hold the tradition, and nothing says anybody does. Reported so that
    /// "permitted" is not mistaken for "grounded".
    Bare,
}

/// What a polity's founding rests on, derived from its charter alone (design
/// **D21**, resolving open question **Q1**).
///
/// Every field comes from the genesis charter, which has no amendment path — so
/// **anyone holding the charter can re-derive this forever**, which is the whole
/// point of recording the attestation rather than merely checking it once.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundingLegitimacy {
    /// Whether an independent reading stands behind the boundary, or only a
    /// convener's word.
    pub source: BoundarySource,
    /// How many the boundary showed.
    pub candidate_size: usize,
    /// How many the charter names.
    pub roll_size: usize,
    /// How many it showed that the charter leaves out.
    pub excluded: usize,
    /// **Who** it left out, each with the reason given. A grade that reports
    /// twenty-nine excluded but not their names would reinstate, one level up, the
    /// exact silence D21 exists to remove — so the report carries them, even though
    /// a Rust caller holding the charter could read them off the attestation
    /// directly. It is the only way the excluded, or anyone auditing on their
    /// behalf, can read the reason off a surface that is not Rust.
    pub exclusions: Vec<Exclusion>,
    /// Exclusions carrying an empty reason: named, but unjustified. Accounting
    /// without justification is still accounting, and this is what says so.
    pub unreasoned_exclusions: usize,
    /// Every reader who signed a reading behind this boundary, in record order
    /// (design **D22**). Empty for a convener's assertion.
    pub readers: Vec<DidKey>,
    /// The readers whose signed reading actually **agrees** with the boundary the
    /// charter ratified (design **D22**).
    ///
    /// Deliberately not `readers.len()`: a reader who signed a *different* boundary
    /// corroborates nothing, and the gap between these two numbers is a reader
    /// disagreeing with the founding on the record.
    ///
    /// **The limit, stated rather than engineered around:** `did:key` costs
    /// nothing, so a captured reader can mint its own corroborators and this count
    /// cannot tell them from independent ones. It is therefore a lens and never a
    /// gate — the same stance kith-covenant took on capture move 1, where a declared
    /// `interpreter_did` was likewise just a name the faction minted. Independence
    /// attestation is the named successor to D22.
    pub corroborating_readers: Vec<DidKey>,
    /// How many actually signed.
    pub ratifiers: usize,
    /// **The number Q1 asks for**: ratifiers as a share of everyone the boundary
    /// showed, rather than of the roll the founders chose. One node chartering a
    /// thirty-node cluster scores `0.033`, where the old majority-of-roll bar it
    /// satisfied reported nothing at all.
    pub consent_share: f64,
    pub basis: FoundingBasis,
    /// What stands above the polity (design **D23**). Belongs on *this* read-model
    /// rather than a separate one for the reason stated at [`FoundingDissent`]: the
    /// substrate is genesis-immutable and sits on the genesis charter, so grounding
    /// cannot accrue and anyone holding the charter re-derives the same answer
    /// forever. Disavowal, which *does* accrue, is kept out — see
    /// [`substrate_disavowal`].
    pub grounding: SubstrateGrounding,
    /// The anchor that co-signed the founding, if any — the key that consented to
    /// standing above this polity. Carried so a reader can name it without parsing
    /// the substrate, and because it is who a later disavowal must be.
    pub anchor: Option<DidKey>,
}

impl FoundingLegitimacy {
    /// Whether an independent reading that **agrees with the ratified boundary**
    /// stands behind it.
    ///
    /// Before D22 this asked only whether the charter carried the `Substrate`
    /// label, which the proposer supplied and nobody signed.
    pub fn corroborated(&self) -> bool {
        !self.corroborating_readers.is_empty()
    }

    /// Readers who signed a reading that does **not** match the ratified boundary.
    /// A non-empty answer is a reader contradicting the founding, in the charter,
    /// permanently.
    pub fn dissenting_readers(&self) -> Vec<&DidKey> {
        self.readers
            .iter()
            .filter(|r| !self.corroborating_readers.contains(r))
            .collect()
    }

    /// Whether the charter names fewer than the boundary showed.
    pub fn narrowed(&self) -> bool {
        self.excluded > 0
    }

    /// Whether every exclusion carries a reason.
    pub fn fully_reasoned(&self) -> bool {
        self.unreasoned_exclusions == 0
    }
}

/// How `polity`'s founding stands, or `None` if it was never chartered.
///
/// Reads the genesis charter and nothing else — no provider, no cohesion input, no
/// query time, because none of what it reports can change after founding.
pub fn founding_legitimacy(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
) -> Option<FoundingLegitimacy> {
    let charter = archive.iter().find_map(|(_, e)| match e {
        CharterEntry::PolityChartered(s) if &s.payload.polity == polity => Some(&s.payload),
        _ => None,
    })?;

    let att = &charter.attestation;
    let candidate_size = att.candidate_size;
    let ratifiers = charter.ratifiers.len();
    let consent_share = if candidate_size == 0 {
        0.0
    } else {
        ratifiers as f64 / candidate_size as f64
    };
    let readers: Vec<DidKey> =
        att.source.readings().iter().map(|r| r.payload.reader.clone()).collect();
    let corroborating_readers = att.corroborating_readers(&charter.members);
    let basis = if !att.source.is_read() {
        FoundingBasis::SelfAttested
    } else if corroborating_readers.is_empty() {
        // `Substrate` with no reading that matches what was ratified. The label is
        // no longer enough on its own (D22): a reading that shows a different
        // boundary is not corroboration of this one.
        FoundingBasis::Narrowed
    } else if ratifiers >= crate::majority(candidate_size) {
        // A majority of what was seen is the bar the old rule *claimed* to apply
        // while actually applying it to the roll. Here it classifies rather than
        // gates, which is the D21 split: gate the accounting, grade the judgement.
        FoundingBasis::Corroborated
    } else {
        FoundingBasis::Narrowed
    };

    // D23 — what stands above it. Ordered by strength of what somebody actually
    // did: a consenting key first, then guardian edges, then a roll that could hold
    // the tradition itself, then nothing but the sentence.
    let anchor = charter.substrate.anchor.clone();
    let derived_on_roll = charter
        .derivations
        .iter()
        .any(|d| charter.members.iter().any(|m| m == &d.member));
    let grounding = if anchor.is_some() {
        SubstrateGrounding::Anchored
    } else if charter.derivations.iter().any(|d| d.guardian.is_some()) {
        SubstrateGrounding::Guarded
    } else if !derived_on_roll {
        SubstrateGrounding::Held
    } else {
        SubstrateGrounding::Bare
    };

    Some(FoundingLegitimacy {
        source: att.source.clone(),
        readers,
        corroborating_readers,
        candidate_size,
        roll_size: charter.members.len(),
        excluded: att.exclusions.len(),
        exclusions: att.exclusions.clone(),
        unreasoned_exclusions: att.exclusions.iter().filter(|e| e.reason.trim().is_empty()).count(),
        ratifiers,
        consent_share,
        basis,
        grounding,
        anchor,
    })
}

/// One party's dissent from a founding boundary, as recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DissentEntry {
    pub dissenter: DidKey,
    pub kind: DissentKind,
    pub rationale: String,
    pub dissented_at: Timestamp,
}

/// What the excluded have said back about a founding (design **D22**).
///
/// Kept **separate from [`founding_legitimacy`] on purpose.** That read-model's
/// whole value is that it derives from the charter *alone* and so gives the same
/// answer forever to anyone holding it; dissent lives in the archive and accrues
/// over time. Folding the two would destroy the first one's property in order to
/// improve neither. Two read-models, two honest claims.
///
/// Nothing here adjudicates. A dissent is one party's assertion, and the tier
/// records that it was made — no more.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoundingDissent {
    /// Every dissent filed against this founding, in append order.
    pub entries: Vec<DissentEntry>,
}

impl FoundingDissent {
    /// Dissenters claiming the founding accounting never mentioned them.
    ///
    /// **This is the answer to the limit D21 recorded as beyond its reach.** The
    /// archive holds no provider, so nothing can check `candidate_size` against
    /// what the substrate showed — but the people it omitted can say so, and this
    /// is where that shows up.
    pub fn unnamed(&self) -> Vec<&DissentEntry> {
        self.entries.iter().filter(|d| d.kind == DissentKind::Unnamed).collect()
    }

    /// Dissenters who were named as excluded and reject the reason given.
    pub fn reasons_rejected(&self) -> Vec<&DissentEntry> {
        self.entries.iter().filter(|d| d.kind == DissentKind::ReasonRejected).collect()
    }

    /// Distinct dissenters, however many records each filed.
    pub fn dissenters(&self) -> BTreeSet<&str> {
        self.entries.iter().map(|d| d.dissenter.as_str()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Read back the dissent filed against `polity`'s founding, up to `as_of`.
///
/// Reports dissent from a **dissolved** polity too: that is the one entry class a
/// dissolution does not close, so a read-model that dropped it after winding-up
/// would hand founders exactly the silence the record exists to deny them.
pub fn founding_dissent(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> FoundingDissent {
    let entries = archive
        .iter()
        .filter_map(|(_, e)| match e {
            CharterEntry::BoundaryDissented(s)
                if &s.payload.polity == polity && s.payload.dissented_at <= as_of =>
            {
                Some(DissentEntry {
                    dissenter: s.payload.dissenter.clone(),
                    kind: s.payload.kind,
                    rationale: s.payload.rationale.clone(),
                    dissented_at: s.payload.dissented_at,
                })
            }
            _ => None,
        })
        .collect();
    FoundingDissent { entries }
}

// ---------------------------------------------------------------------------
// D23 — whether the ground still stands there.
// ---------------------------------------------------------------------------

/// One withdrawal by a polity's chartered substrate anchor, as recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisavowalEntry {
    pub disavower: DidKey,
    pub rationale: String,
    pub disavowed_at: Timestamp,
}

/// How a polity's chartered ground stands: whether the anchor that consented at
/// founding has since withdrawn (design **D23**).
///
/// Named for the *standing* rather than the act because the record shape
/// [`crate::SubstrateDisavowal`] is the act — the same split D22 made between
/// `BoundaryDissent` (one filing) and `FoundingDissent` (what has accrued).
///
/// Kept **separate from [`founding_legitimacy`]**, and on exactly the test D22
/// applied to dissent: that read-model derives from the genesis charter alone and so
/// gives the same answer forever, while this accrues over time. `grounding` — what
/// the charter *committed to* — belongs there, because a genesis-immutable substrate
/// cannot change. Whether the ground still stands there belongs here, because that
/// can.
///
/// Nothing here adjudicates, and nothing here un-charters. A disavowal is the
/// ground's own assertion, and the tier records that it was made.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroundStanding {
    /// Every withdrawal filed for this polity, in append order. More than one is
    /// possible and is not a contradiction — a ground may say it twice.
    pub entries: Vec<DisavowalEntry>,
}

impl GroundStanding {
    /// Whether the chartered ground has withdrawn at all.
    ///
    /// **This is the whole reading.** There is no "how much" here and deliberately
    /// no recovery path: a ground that withdrew and later changed its mind cannot
    /// un-say it, because the substrate has no amendment record and this tier does
    /// not invent one. What such a polity has is the option every polity has — to
    /// re-charter, deliberately, as the different polity it now is.
    pub fn disavowed(&self) -> bool {
        !self.entries.is_empty()
    }

    /// When the ground first withdrew, if it has.
    pub fn first_disavowed_at(&self) -> Option<Timestamp> {
        self.entries.iter().map(|d| d.disavowed_at).min()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Read back the withdrawals filed against `polity`'s substrate, up to `as_of`.
///
/// Reports them for a **dissolved** polity too, for the reason
/// [`founding_dissent`] does: this is the second entry class a dissolution does not
/// close, and a read-model that dropped it after winding-up would hand founders the
/// silence the record exists to deny them.
pub fn substrate_disavowal(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> GroundStanding {
    let entries = archive
        .iter()
        .filter_map(|(_, e)| match e {
            CharterEntry::SubstrateDisavowed(s)
                if &s.payload.polity == polity && s.payload.disavowed_at <= as_of =>
            {
                Some(DisavowalEntry {
                    disavower: s.payload.disavower.clone(),
                    rationale: s.payload.rationale.clone(),
                    disavowed_at: s.payload.disavowed_at,
                })
            }
            _ => None,
        })
        .collect();
    GroundStanding { entries }
}
