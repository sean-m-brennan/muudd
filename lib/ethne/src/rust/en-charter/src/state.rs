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

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use en_archive::Archive;
use en_core::{Cid, DidKey, KcResult, PubKey, Timestamp};

use crate::entry::CharterEntry;
use crate::record::{
    AffirmationKind, AmendmentKind, AttestedRole, BoundarySource, Commitment, DerivedEdge,
    DissentKind, EnvoyKind, Exclusion, GuardianEvidence, GuardianKind, IndependenceKind,
    MemberReading, MembershipKind, Movement, Office, OfficeKind, Substrate,
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
///
/// Holds [`DerivedEdge`] rather than [`crate::Derivation`] since **D27**: a derivation is one
/// record's claim *and the evidence offered for it*, while this is the state left
/// after folding the charter with every later change, where that evidence has no
/// meaning. Keeping `Derivation` here would have put an always-`None` `evidence`
/// field in a read-model — a field present and read by nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuardianState {
    pub derivations: Vec<DerivedEdge>,
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
///
/// The charter's **ratifiers** are seeded as inducted at `founded_at`, because
/// ratifying *is* their induction (D13) — and only they. A founding member who did
/// not sign the charter is on the roll and un-inducted, which is an honest state the
/// tier already has a name for: the founding needs only a majority, so the rest of
/// the roll consented to nothing and recited nothing. Seeding the whole founding roll
/// (as this did until **D29**) handed the substrate to whoever was listed, dated to an
/// instant they were absent from — measured, a roll of 7 ratified by 4 reported 7
/// inducted.
///
/// Every `MemberAffirmed` record ≤ `as_of` then sets the induction flag (on
/// `Induction`) and advances `last_affirmed_at`. Only current members
/// ([`charter_state`]) are reported.
pub fn affirmation_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> AffirmationState {
    // Genesis: the ratifiers (not the founding roll) + the founding instant.
    let mut founding: Vec<DidKey> = Vec::new();
    let mut founded_at: Option<Timestamp> = None;
    for (_cid, e) in archive.iter() {
        if let CharterEntry::PolityChartered(s) = e {
            if &s.payload.polity == polity && s.payload.founded_at <= as_of {
                founding = s.payload.ratifiers.clone();
                founded_at = Some(s.payload.founded_at);
                break;
            }
        }
    }

    // Seed each current member: ratifiers are inducted at `founded_at`.
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

/// One relay the polity runs, as its roster will list it (design **D36**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayListing {
    /// The operator, whose `did:key` is the relay's signing key.
    pub member: DidKey,
    pub at_uuid: String,
    pub host: String,
    pub port: u16,
    pub declared_at: Timestamp,
    /// The geohash areas the relay is a hub for (D38); empty for a plain relay.
    pub areas: Vec<String>,
}

/// The relays a polity runs as of a query time, one per operator, sorted by
/// member DID.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RendezvousState {
    pub relays: Vec<RelayListing>,
}

/// Which relays the polity runs as of `as_of` (design **D36**): each member's
/// latest [`RelayEndpoint`](crate::RelayEndpoint) declared by then, unless it was a
/// withdrawal, kept only while that member holds
/// [`Office::RendezvousOperator`] in [`office_state`] at `as_of`. So revoking the
/// office, resigning it, or leaving the roll drops the relay with no further act,
/// and a dissolved polity runs none.
///
/// **Only declarations the authority pass accepts are read.** This read-model runs
/// [`authority_violations`](crate::authority_violations) itself rather than
/// trusting that the host did, because the roster is what an outsider will trust:
/// a declaration in somebody else's name, one by a member who never held the
/// office, or one by a holder who has not received the substrate (D29's rule,
/// which [`office_state`] alone does not apply) is simply not there.
pub fn rendezvous_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> RendezvousState {
    if charter_state(archive, polity, as_of).dissolved {
        return RendezvousState::default();
    }
    let refused: Vec<Cid> =
        crate::authority_violations(archive).into_iter().map(|(cid, _)| cid).collect();
    let offices = office_state(archive, polity, as_of);
    let mut latest: BTreeMap<String, &crate::RelayEndpoint> = BTreeMap::new();
    for (cid, e) in archive.iter() {
        let CharterEntry::RelayEndpointDeclared(s) = e else { continue };
        let d = &s.payload;
        if &d.polity != polity || d.declared_at > as_of || refused.contains(cid) {
            continue;
        }
        // Insertion order is the tiebreak for equal instants: the later entry wins.
        match latest.get(d.member.as_str()) {
            Some(prev) if prev.declared_at > d.declared_at => {}
            _ => {
                latest.insert(d.member.as_str().to_string(), d);
            }
        }
    }
    let relays = latest
        .into_values()
        .filter(|d| !d.withdrawn && offices.holds(&d.member, &Office::RendezvousOperator))
        .map(|d| RelayListing {
            member: d.member.clone(),
            at_uuid: d.at_uuid.clone(),
            host: d.host.clone(),
            port: d.port,
            declared_at: d.declared_at,
            areas: d.areas.clone(),
        })
        .collect();
    RendezvousState { relays }
}

/// The compact acts a polity's roll has decided and that are in force (design
/// **D40**): its envoy's mandate at kith-covenant's border.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactState {
    /// Accepted acts decided by `as_of` and not lapsed, in log order, each with the
    /// entry that recorded it.
    pub acts: Vec<(Cid, crate::CompactAct)>,
}

impl CompactState {
    /// Whether an act in force lets the envoy do `kind` in `compact` towards
    /// `counterparty`.
    pub fn covers(&self, kind: crate::CompactActKind, compact: &str, counterparty: &DidKey) -> bool {
        self.acts.iter().any(|(_, a)| {
            a.kind == kind && a.compact == compact && a.counterparties.contains(counterparty)
        })
    }

    /// The compacts the roll has decided to found or join.
    pub fn compacts(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .acts
            .iter()
            .filter(|(_, a)| a.kind == crate::CompactActKind::Join)
            .map(|(_, a)| a.compact.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// The compact acts in force for `polity` as of `as_of` (design **D40**). Only acts
/// the authority pass accepts are read, as for [`content_policy_state`], so a
/// severe act passed at a bare majority is simply not there. A dissolved polity has
/// no mandate.
pub fn compact_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> CompactState {
    if charter_state(archive, polity, as_of).dissolved {
        return CompactState::default();
    }
    let refused: Vec<Cid> =
        crate::authority_violations(archive).into_iter().map(|(cid, _)| cid).collect();
    let mut out = CompactState::default();
    for (cid, e) in archive.iter() {
        let CharterEntry::CompactActDecided(s) = e else { continue };
        let a = &s.payload;
        if &a.polity != polity || refused.contains(cid) || !a.in_force_at(as_of) {
            continue;
        }
        out.acts.push((*cid, a.clone()));
    }
    out
}

/// The content policy a polity is under as of a query time, and who may label
/// content in its name (design **D37**).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentPolicyState {
    /// The policy in force: the latest accepted amendment decided by `as_of`, or the
    /// empty policy when there is none.
    pub policy: crate::ContentPolicy,
    /// The amendment that set it, `None` for the empty policy. A policy artifact's
    /// sequence is read from here, so a reader can tell a newer policy from an
    /// older one without the log.
    pub set_by: Option<Cid>,
    /// Holders of [`Office::Moderator`] who hold the substrate (D29), sorted by DID.
    pub moderators: Vec<DidKey>,
}

/// The content policy and moderators of a polity as of `as_of` (design **D37**).
///
/// **Only amendments the authority pass accepts are read**, for the reason
/// [`rendezvous_state`] gives: this is what a reader outside the polity will be
/// shown, so a loosening passed at a bare majority is simply not there. A dissolved
/// polity moderates nothing and has no moderators.
///
/// A moderator must hold the office ([`office_state`]) **and** have been inducted
/// ([`affirmation_state`]), because labelling in the polity's name is the polity
/// acting through a member — D29's rule for every office.
pub fn content_policy_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> ContentPolicyState {
    if charter_state(archive, polity, as_of).dissolved {
        return ContentPolicyState::default();
    }
    let refused: Vec<Cid> =
        crate::authority_violations(archive).into_iter().map(|(cid, _)| cid).collect();
    let mut out = ContentPolicyState::default();
    let mut latest: Option<Timestamp> = None;
    for (cid, e) in archive.iter() {
        let CharterEntry::ContentPolicyAmended(s) = e else { continue };
        let a = &s.payload;
        if &a.polity != polity || a.decided_at > as_of || refused.contains(cid) {
            continue;
        }
        // Insertion order is the tiebreak for equal instants: the later entry wins.
        if latest.is_none_or(|t| a.decided_at >= t) {
            latest = Some(a.decided_at);
            out.policy = a.policy.clone();
            out.set_by = Some(*cid);
        }
    }
    let inducted: BTreeSet<String> = affirmation_state(archive, polity, as_of)
        .entries
        .iter()
        .filter(|m| m.inducted)
        .map(|m| m.member.as_str().to_string())
        .collect();
    let mut moderators: Vec<DidKey> = office_state(archive, polity, as_of)
        .holders(&Office::Moderator)
        .into_iter()
        .filter(|d| inducted.contains(d.as_str()))
        .cloned()
        .collect();
    moderators.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    moderators.dedup();
    out.moderators = moderators;
    out
}

/// One release the polity authorized (design **D41**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseListing {
    /// The record's CID in the charter log.
    pub cid: Cid,
    pub officer: DidKey,
    pub repo: String,
    pub url: String,
    pub commit: String,
    pub built_by: Option<DidKey>,
    pub authorized_at: Timestamp,
}

/// Who may release for the polity as of a query time, and every release it has
/// authorized by then (design **D41**).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReleaseState {
    /// The members holding [`Office::ReleaseOfficer`] who also hold the substrate
    /// (D29), sorted by DID: the officers whose authorizations the polity stands
    /// behind from now on.
    pub officers: Vec<DidKey>,
    /// Every accepted authorization made by `as_of`, in log order.
    pub releases: Vec<ReleaseListing>,
}

impl ReleaseState {
    /// The authorizations of `commit` of the repository at `url`.
    pub fn of(&self, url: &str, commit: &str) -> Vec<&ReleaseListing> {
        self.releases.iter().filter(|r| r.url == url && r.commit == commit).collect()
    }
}

/// Who may release for the polity as of `as_of`, and what it has released
/// (design **D41**).
///
/// **Releases are never withdrawn.** A release stays listed after its officer
/// resigns, is revoked or leaves the roll, and after the polity dissolves, because
/// answerability for shipped code is not something a later act can take back: a
/// polity that could unlist a release by revoking its officer could unlist any
/// finding against it. Only the *officers* list moves with the office.
///
/// **Only authorizations the authority pass accepts are read**, as for
/// [`rendezvous_state`], so one signed by a member who never held the office, or
/// held it without the substrate, is simply not there.
pub fn release_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> ReleaseState {
    let refused: Vec<Cid> =
        crate::authority_violations(archive).into_iter().map(|(cid, _)| cid).collect();
    let mut releases = Vec::new();
    for (cid, e) in archive.iter() {
        let CharterEntry::ReleaseAuthorized(s) = e else { continue };
        let r = &s.payload;
        if &r.polity != polity || r.authorized_at > as_of || refused.contains(cid) {
            continue;
        }
        releases.push(ReleaseListing {
            cid: *cid,
            officer: r.officer.clone(),
            repo: r.repo.clone(),
            url: r.url.clone(),
            commit: r.commit.clone(),
            built_by: r.built_by.clone(),
            authorized_at: r.authorized_at,
        });
    }
    let mut officers = Vec::new();
    if !charter_state(archive, polity, as_of).dissolved {
        let offices = office_state(archive, polity, as_of);
        let affirmed = affirmation_state(archive, polity, as_of);
        officers = offices
            .holders(&Office::ReleaseOfficer)
            .into_iter()
            .filter(|h| affirmed.is_inducted(h))
            .cloned()
            .collect();
        officers.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        officers.dedup();
    }
    ReleaseState { officers, releases }
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
    let mut raw: Vec<DerivedEdge> = Vec::new();
    for (_cid, e) in archive.iter() {
        match e {
            CharterEntry::PolityChartered(s)
                if &s.payload.polity == polity && s.payload.founded_at <= as_of =>
            {
                for d in &s.payload.derivations {
                    if !raw.iter().any(|h| h.member == d.member) {
                        raw.push(d.edge());
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
                            raw.push(DerivedEdge {
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
/// span is the absence, and the **watcher** count is the evidence that the absence was
/// observed rather than merely unrecorded. Watchers, not filings — see
/// [`Self::watchers_for`] for why the distinction is load-bearing (**D28**).
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

    /// How many **filings** in the window actually saw `who`. An honest count of
    /// records, and deliberately not what any grade reads — see [`Self::watchers_for`].
    pub fn readings_for(&self, who: &DidKey) -> usize {
        self.observations
            .iter()
            .filter(|o| o.readings.iter().any(|r| &r.member == who))
            .count()
    }

    /// How many **distinct observers** saw `who` — the per-member evidence of
    /// watching, and what a duration measure is held to (design **D28**).
    ///
    /// **D19 made this count a safety floor on the reasoning that it "is evidence
    /// that somebody was watching", and the sentence was false as written.** Measured:
    /// one key, filing three times, made `readings_for` read `3` and drove
    /// `en_cohesion`'s sustained-absence component to `Concern` — a `Concern` that
    /// crosses to kith-covenant as a conduct record, manufactured by a stranger for
    /// the price of one keypair. Three filings by one actor are one watcher, and that
    /// is arithmetic rather than a judgement about who anyone is.
    ///
    /// A monitor that observed ten times but saw this member once has watched them
    /// once, which is why this is per member and not per log.
    ///
    /// Nothing here asks whether the watchers are *independent* of each other or of
    /// the polity — minting a second key is still cheap. That is the separate lens
    /// [`observer_independence`] grades, and it is deliberately not a gate.
    pub fn watchers_for(&self, who: &DidKey) -> usize {
        self.observations
            .iter()
            .filter(|o| o.readings.iter().any(|r| &r.member == who))
            .map(|o| o.observer.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }

    /// The distinct observers who saw `who`, oldest-first order discarded.
    pub fn watchers_of(&self, who: &DidKey) -> BTreeSet<&str> {
        self.observations
            .iter()
            .filter(|o| o.readings.iter().any(|r| &r.member == who))
            .map(|o| o.observer.as_str())
            .collect()
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
    /// *both* `live_measure_seconds` and `min_witness_watchers` regardless of what is
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
    /// *span* half of a sustained absence; `min_witness_watchers` protects the other.
    ///
    /// Defaults to one day, matching `en_cohesion`'s `sustained_absence_seconds`.
    /// It is a *separate* knob from `retain_seconds` because the two are different
    /// claims: one is how much history the polity wants to keep, the other is how
    /// much its own grading still needs. Collapsing them would let a caller tune
    /// away a safety property while believing they were only tuning storage.
    pub live_measure_seconds: i64,
    /// How many **distinct observers** *of each watched member* must survive — the
    /// second half of the safety floor, and the half a duration cannot supply.
    ///
    /// A sustained absence is graded on two conditions, and `live_measure_seconds`
    /// only protects one of them. The span survives a prune (D17 clamps it to the
    /// horizon), but the **witness count** does not: prune to a 24-hour horizon on a
    /// polity observed twice a day and the count falls from four to two, so a
    /// `Concern` silently becomes a `Notice` with no signal that evidence was what
    /// changed. Measured, not assumed — the same absence, the same horizon, graded
    /// `Concern` at a dense cadence and `Notice` at a sparse one.
    ///
    /// So the floor is a duration **and** a count. Defaults to 3, matching
    /// `en_cohesion`'s `sustained_absence_watchers`. **Watchers, not filings, since
    /// D28** — the floor tracks whatever the grade reads, or it protects a number
    /// nothing uses. The count is per member, not per log: a member the monitors
    /// mention rarely has thin evidence, and thinning it further is exactly the clip
    /// this read-model exists to refuse.
    pub min_witness_watchers: usize,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy {
            retain_seconds: crate::PRUNE_SETTLING_SECONDS,
            min_dropped: 100,
            live_measure_seconds: 86_400,
            min_witness_watchers: 3,
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

/// The latest horizon that still leaves `min_watchers` **distinct observers** of
/// **every** watched member standing, or `None` when some member has too few to spare.
///
/// This is the half of the safety floor a duration cannot supply, and **the floor has
/// to preserve whatever the grade reads** — which is D19's own finding, recurring one
/// decision later. D19 protected a count of filings because that is what the
/// sustained-absence component read; **D28** found that count was the wrong quantity
/// and moved the component to distinct watchers, so this moves with it. A floor that
/// protects a number nothing grades is not a floor.
///
/// The binding constraint is whichever member is *watched* most sparsely: the horizon
/// may not advance past the point where their `min_watchers`-th distinct observer
/// falls out of the retained window. Note this is strictly stronger than counting
/// filings — a member seen ten times by one monitor has one watcher, so a policy
/// asking for three pins the horizon completely. That is the correct answer and not a
/// deadlock: D17 always leaves the roll free to prune anything by majority. What is
/// refused is *recommending* it.
fn witness_floor(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
    min_watchers: usize,
) -> Option<Timestamp> {
    if min_watchers == 0 {
        return Some(as_of);
    }
    // (when, who) per member, so the walk below can count observers rather than rows.
    let mut per_member: HashMap<&DidKey, Vec<(i64, &str)>> = HashMap::new();
    for (_, entry) in archive.iter() {
        if let CharterEntry::Observed(s) = entry {
            if &s.payload.polity != polity || s.payload.observed_at > as_of {
                continue;
            }
            for reading in &s.payload.readings {
                per_member.entry(&reading.member).or_default().push((
                    s.payload.observed_at.as_second(),
                    s.payload.observer.as_str(),
                ));
            }
        }
    }
    let mut floor = as_of.as_second();
    for (_, mut seen) in per_member {
        // Newest first, accumulating distinct observers: the horizon may fall no
        // earlier than the moment the `min_watchers`-th of them was reached, because
        // everything at or after that instant is what survives the prune.
        seen.sort_unstable_by_key(|(at, _)| Reverse(*at));
        let mut observers: BTreeSet<&str> = BTreeSet::new();
        let mut pinned = None;
        for (at, who) in seen {
            observers.insert(who);
            if observers.len() >= min_watchers {
                pinned = Some(at);
                break;
            }
        }
        // Nothing may go: this member has too few watchers to spare one.
        let pinned = pinned?;
        floor = floor.min(pinned);
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
    let by_witness = witness_floor(archive, polity, as_of, policy.min_witness_watchers)?;
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
    // `Guarded` requires an edge an observation actually **backed**, taken by
    // somebody outside the polity (D27). A self-declared `Unbacked` edge is honest and
    // permitted, but grading it as ground would let a polity raise its own grade by
    // writing a sentence — and a founder's own observation of the cluster would let it
    // do the same in two steps. Same predicate as the authority rule, via
    // `Derivation::is_backed`, so the grade and the gate cannot drift.
    let grounding = if anchor.is_some() {
        SubstrateGrounding::Anchored
    } else if charter.derivations.iter().any(|d| {
        d.is_backed(&charter.polity, |o| charter.members.iter().any(|m| m.as_str() == o))
    }) {
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

// ---------------------------------------------------------------------------
// D24 — whether a founding's readers were their own principals.
// ---------------------------------------------------------------------------

/// Who has vouched for (or challenged) a founding's readers, and whose vouching
/// counts (design **D24**, closing the residue **D22** named as its successor).
///
/// Kept **separate from [`founding_legitimacy`]** on the test D22 stated and D23
/// reused: that read-model derives from the genesis charter alone, so anyone holding
/// the charter re-derives it forever. Attestations arrive afterwards and accrue, so
/// they cannot live there without destroying the property.
///
/// **A lens, never a gate.** Nothing refuses a founding for having no attestations,
/// and a young honest founding has none — refusing it would be refusing newness. This
/// is exactly where kith-covenant left the same problem for capture move 1.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReaderIndependence {
    /// reader → the distinct disinterested parties who vouched for it. These are the
    /// ones that **count**: neither the polity, nor the reader itself, nor anyone on
    /// the founding roll.
    pub attesters: BTreeMap<String, BTreeSet<String>>,
    /// reader → parties who vouched but have a stake: the polity, the reader itself,
    /// or a founding member.
    ///
    /// Recorded rather than discarded, and reported rather than quietly folded in. A
    /// founder vouching for the founding's own reader is the sockpuppet move one step
    /// out, so it cannot count; but it is still a claim somebody signed their name to,
    /// and destroying it would serve nobody.
    pub interested_attesters: BTreeMap<String, BTreeSet<String>>,
    /// reader → parties asserting it is **not** independent. The counter-claim, so the
    /// lens can be contradicted by somebody who thinks it is being fooled. Never
    /// adjudicated: a contested reader is reported as contested, not as disqualified.
    pub contested: BTreeMap<String, BTreeSet<String>>,
}

impl ReaderIndependence {
    /// Readers with at least one disinterested voucher.
    pub fn independent_readers(&self) -> Vec<&str> {
        self.attesters
            .iter()
            .filter(|(_, a)| !a.is_empty())
            .map(|(r, _)| r.as_str())
            .collect()
    }

    /// Whether `reader` has any disinterested voucher.
    pub fn is_attested(&self, reader: &str) -> bool {
        self.attesters.get(reader).is_some_and(|a| !a.is_empty())
    }

    /// Whether anyone has claimed `reader` is controlled.
    pub fn is_contested(&self, reader: &str) -> bool {
        self.contested.get(reader).is_some_and(|a| !a.is_empty())
    }
}

/// What a founding's corroboration is worth once independence is taken into account
/// (design **D24**) — the reading that answers D22's residue.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndependentCorroboration {
    /// Readers who **both** agree with the ratified boundary and carry a
    /// disinterested voucher. **The number D22 could not produce**: a chorus of five
    /// hundred minted readers scores `0` here, because none of them has an outsider
    /// willing to be named for it.
    pub readers: Vec<DidKey>,
    /// Of those, the ones somebody has also claimed are controlled. Reported so that
    /// "independently corroborated" cannot quietly mean "and nobody objected".
    pub contested: Vec<DidKey>,
}

impl IndependentCorroboration {
    /// Whether any independently-vouched reader agrees with what was ratified.
    pub fn any(&self) -> bool {
        !self.readers.is_empty()
    }

    /// How many.
    pub fn count(&self) -> usize {
        self.readers.len()
    }

    /// The uncontested ones — those with a voucher and no counter-claim.
    pub fn uncontested(&self) -> Vec<&DidKey> {
        self.readers.iter().filter(|r| !self.contested.contains(r)).collect()
    }
}

/// Read back who has vouched for `polity`'s parties in one `role`, up to `as_of`
/// (design **D24**, generalized to observers by **D28**).
///
/// Attesters are sorted into three buckets by *stake*, which is the whole point: an
/// attestation from the polity, from the subject itself, or from a founding member
/// cannot count toward independence, because those are precisely the parties a
/// sockpuppet chorus would be serving. The founding roll is used rather than the live
/// one — a member admitted years later had no hand in what is being vouched for, and
/// for an observer the founders remain the interested party whoever joined since.
///
/// The buckets are **per role**. A key that both read the boundary and later filed
/// observations is two subjects here, because a voucher speaks to a relationship
/// rather than to an identity in the abstract: "I know this party is not the
/// convener's creature" is a different claim from "I know this monitor is nobody's
/// creature", and only one of them may have been made.
///
/// Reports attestations for a **dissolved** polity too: the third entry class a
/// dissolution does not close, and by now a category rather than an exception —
/// evidence about a founding outlives the polity, because the founding's legitimacy
/// does.
pub fn independence(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    role: AttestedRole,
    as_of: Timestamp,
) -> ReaderIndependence {
    let mut out = ReaderIndependence::default();
    let charter = archive.iter().find_map(|(_, e)| match e {
        CharterEntry::PolityChartered(s) if &s.payload.polity == polity => Some(&s.payload),
        _ => None,
    });
    let Some(charter) = charter else {
        return out;
    };
    let roll: BTreeSet<&str> = charter.members.iter().map(DidKey::as_str).collect();

    for (_, e) in archive.iter() {
        let CharterEntry::IndependenceAttested(s) = e else {
            continue;
        };
        let a = &s.payload;
        if &a.polity != polity || a.role != role || a.attested_at > as_of {
            continue;
        }
        let subject = a.subject.as_str().to_string();
        let attester = a.attesting_party.as_str().to_string();
        match a.kind {
            IndependenceKind::Controlled => {
                // A counter-claim counts however interested its author is. Somebody
                // saying "that one is ours" is worth recording from anyone — including
                // a founder breaking ranks, who is the most informative possible
                // source for it.
                out.contested.entry(subject).or_default().insert(attester);
            }
            IndependenceKind::Independent => {
                let interested = attester == polity.as_str()
                    || attester == a.subject.as_str()
                    || roll.contains(attester.as_str());
                if interested {
                    out.interested_attesters.entry(subject).or_default().insert(attester);
                } else {
                    out.attesters.entry(subject).or_default().insert(attester);
                }
            }
        }
    }
    out
}

/// [`independence`] over the founding's boundary readers (design **D24**).
pub fn reader_independence(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> ReaderIndependence {
    independence(archive, polity, AttestedRole::BoundaryReader, as_of)
}

/// [`independence`] over the monitors that have observed this polity (design **D28**).
///
/// Read alongside [`ObservationWindow::watchers_for`], which counts watchers without
/// asking who they are. This says who has been vouched for; nothing anywhere refuses
/// an unvouched monitor, deliberately — a polity that simply declines to vouch for
/// anybody must not thereby become unwatchable.
pub fn observer_independence(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> ReaderIndependence {
    independence(archive, polity, AttestedRole::Observer, as_of)
}

/// Which of `polity`'s corroborating readers are independently vouched for, up to
/// `as_of` — **the upgrade of D22's corroboration count** (design **D24**).
///
/// Agreement and independence are two different questions and this is the conjunction
/// of them: a reader who agrees with the ratified boundary but has no outside voucher
/// might be five faces of the convener, and a vouched-for reader who read a *different*
/// boundary corroborates nothing. Only both together mean what "corroborated" appears
/// to mean.
pub fn independent_corroboration(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> IndependentCorroboration {
    let charter = archive.iter().find_map(|(_, e)| match e {
        CharterEntry::PolityChartered(s) if &s.payload.polity == polity => Some(&s.payload),
        _ => None,
    });
    let Some(charter) = charter else {
        return IndependentCorroboration::default();
    };
    let independence = reader_independence(archive, polity, as_of);
    let readers: Vec<DidKey> = charter
        .attestation
        .corroborating_readers(&charter.members)
        .into_iter()
        .filter(|r| independence.is_attested(r.as_str()))
        .collect();
    let contested = readers
        .iter()
        .filter(|r| independence.is_contested(r.as_str()))
        .cloned()
        .collect();
    IndependentCorroboration { readers, contested }
}

/// **What a polity's watching is worth once independence is taken into account**
/// (design **D28**) — the observer counterpart of [`IndependentCorroboration`].
///
/// Read against a member's `watchers_for`: that count says how many distinct parties
/// spoke, this says how many of them somebody outside was willing to be named for.
/// A polity watched by five monitors that are five faces of one actor reads `5` there
/// and `0` here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndependentObservation {
    /// Monitors that have observed this polity **and** carry a disinterested voucher.
    pub observers: Vec<DidKey>,
    /// Of those, the ones somebody has also claimed are controlled — so
    /// "independently observed" cannot quietly mean "and nobody objected".
    pub contested: Vec<DidKey>,
    /// Monitors that have observed and carry no disinterested voucher at all.
    ///
    /// **Not a fault, and the reason none of this is a gate.** A monitor nobody has
    /// vouched for is the normal state of a new one, and of every honest one in a
    /// polity whose neighbours do not write attestations. Reported so that the absence
    /// of vouching is visible rather than silently read as its presence.
    pub unvouched: Vec<DidKey>,
}

impl IndependentObservation {
    /// How many monitors carry a disinterested voucher.
    pub fn count(&self) -> usize {
        self.observers.len()
    }

    /// Whether any does.
    pub fn any(&self) -> bool {
        !self.observers.is_empty()
    }

    /// The vouched-for monitors nobody has contradicted.
    pub fn uncontested(&self) -> Vec<&DidKey> {
        self.observers.iter().filter(|o| !self.contested.contains(o)).collect()
    }
}

/// Which of the monitors that have observed `polity` are independently vouched for,
/// up to `as_of` (design **D28**).
///
/// The set of monitors is taken from the log rather than from any registry, because
/// there is no registry: **anybody may observe a polity**, which is D16's deliberate
/// choice — a monitor that had to sit inside the roll could be silenced by expulsion.
/// The cost of that openness is that a monitor may also be nobody, and this is the
/// lens that says which.
pub fn independent_observation(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> IndependentObservation {
    let mut out = IndependentObservation::default();
    let seen = observer_independence(archive, polity, as_of);
    // Keyed by DID string because `DidKey` is not `Ord`; the value carries the key
    // itself so the report can hand back real identities rather than strings.
    let mut monitors: BTreeMap<&str, &DidKey> = BTreeMap::new();
    for (_, e) in archive.iter() {
        match e {
            CharterEntry::Observed(s)
                if &s.payload.polity == polity && s.payload.observed_at <= as_of =>
            {
                monitors.insert(s.payload.observer.as_str(), &s.payload.observer);
            }
            // A monitor whose observation was carried as guardian evidence has
            // observed this polity as surely as one that filed its own entry — the
            // same equivalence the authority pass's `observers` ledger draws.
            CharterEntry::PolityChartered(s) if &s.payload.polity == polity => {
                for d in &s.payload.derivations {
                    if let Some(GuardianEvidence::Observed(ob)) = &d.evidence {
                        monitors
                            .insert(ob.payload.observer.as_str(), &ob.payload.observer);
                    }
                }
            }
            CharterEntry::GuardianChanged(s) if &s.payload.polity == polity => {
                if let Some(GuardianEvidence::Observed(ob)) = &s.payload.evidence {
                    monitors.insert(ob.payload.observer.as_str(), &ob.payload.observer);
                }
            }
            _ => {}
        }
    }
    for (who, did) in monitors {
        if seen.is_attested(who) {
            out.observers.push(did.clone());
            if seen.is_contested(who) {
                out.contested.push(did.clone());
            }
        } else {
            out.unvouched.push(did.clone());
        }
    }
    out
}

/// **What the polity's chartered ground rests on, once the monitor is looked at**
/// (design **D28**).
///
/// D27 made a founding guardian edge carry the observation that showed it, and made
/// the observer be somebody outside the polity. It did not — deliberately — ask *who*
/// that outsider was, because requiring a vouched-for monitor at founding would refuse
/// newness, which is the line **D24** drew and this slice keeps.
///
/// So the grade stays where it is and this reports the residue: of the founding edges
/// that earned `Guarded`, how many rest on a monitor somebody outside has been willing
/// to be named for. A separate accruing read-model rather than a field on
/// [`FoundingLegitimacy`], on that model's charter-alone property — attestations
/// arrive afterwards, and folding them in would mean the same charter graded
/// differently on different days.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IndependentGrounding {
    /// Founding-derived members whose guardian edge is backed by a vouched-for monitor.
    pub vouched_edges: Vec<DidKey>,
    /// Founding-derived members whose backing monitor carries no disinterested
    /// voucher. `Guarded` all the same — this is what the grade does not say.
    pub unvouched_edges: Vec<DidKey>,
    /// Members among `vouched_edges` whose monitor somebody has called controlled.
    pub contested_edges: Vec<DidKey>,
}

impl IndependentGrounding {
    /// Whether any chartered edge rests on a vouched-for monitor.
    pub fn any(&self) -> bool {
        !self.vouched_edges.is_empty()
    }
}

/// Read back [`IndependentGrounding`] for `polity` as of `as_of` (design **D28**).
pub fn independent_grounding(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> IndependentGrounding {
    let mut out = IndependentGrounding::default();
    let charter = archive.iter().find_map(|(_, e)| match e {
        CharterEntry::PolityChartered(s) if &s.payload.polity == polity => Some(&s.payload),
        _ => None,
    });
    let Some(charter) = charter else {
        return out;
    };
    let seen = observer_independence(archive, polity, as_of);
    let on_roll = |o: &str| charter.members.iter().any(|m| m.as_str() == o);
    for d in &charter.derivations {
        if !d.is_backed(&charter.polity, on_roll) {
            continue;
        }
        let Some(GuardianEvidence::Observed(ob)) = &d.evidence else {
            continue;
        };
        let who = ob.payload.observer.as_str();
        if seen.is_attested(who) {
            out.vouched_edges.push(d.member.clone());
            if seen.is_contested(who) {
                out.contested_edges.push(d.member.clone());
            }
        } else {
            out.unvouched_edges.push(d.member.clone());
        }
    }
    out
}

// --- D29: the rite as a process, not only its completion ---------------------

/// One movement of one rite, as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementPoint {
    /// 1–5, the movement's place in the sequence.
    pub order: u8,
    /// Who was present, and so co-signed.
    pub attending: Vec<DidKey>,
    pub recited_at: Timestamp,
}

/// One member's rite, complete or in progress (design **D29**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rite {
    pub member: DidKey,
    /// Which ceremony this rite is (design **D34**): a first acquisition of the
    /// substrate, or its periodic renewal. Taken from the opening movement, which every
    /// later movement of the rite must agree with.
    ///
    /// Kept distinct because the two are not comparable as ceremonies: an induction
    /// happens once per member and a renewal recurs, so a cheapening measure that mixed
    /// them would compare one member's reception against another's renewal.
    pub kind: AffirmationKind,
    /// The movements recited so far, in order.
    pub movements: Vec<MovementPoint>,
    /// When the completing affirmation of this rite's own kind was recorded, or `None`
    /// while the rite is
    /// open — which includes a rite that was **abandoned**: this tier does not
    /// decide when an unfinished rite has been given up on, because nothing in the
    /// doctrine says how long a catechumenate may take.
    pub completed_at: Option<Timestamp>,
}

impl Rite {
    /// Whether all five movements have been recited.
    pub fn all_movements_recited(&self) -> bool {
        self.movements.len() == Movement::COUNT as usize
    }

    /// Seconds from the first movement to the last, or `None` for an empty rite.
    ///
    /// This is the rite's **cost in time**, and the measure v1 could not express at
    /// all: an induction carried one instant, so a ceremony held over a week and one
    /// performed in a second produced identical records.
    pub fn span_seconds(&self) -> Option<i64> {
        match (self.movements.first(), self.movements.last()) {
            (Some(a), Some(b)) => Some(b.recited_at.as_second() - a.recited_at.as_second()),
            _ => None,
        }
    }

    /// The members who attended **every** movement — the only ones the completion may
    /// count as witnesses. An intersection rather than a union, because five movements
    /// signed by five disjoint groups is a rite nobody was at.
    pub fn attended_throughout(&self) -> Vec<DidKey> {
        let mut iter = self.movements.iter();
        let Some(first) = iter.next() else {
            return Vec::new();
        };
        let mut keep: Vec<DidKey> = first.attending.clone();
        for m in iter {
            keep.retain(|w| m.attending.contains(w));
        }
        keep
    }

    /// How many attendees the rite **lost** between its first movement and its last.
    ///
    /// Not a trend across rites but a fact within one: the polity that turned out for
    /// the naming of the substrate and had drifted away by the submission of the
    /// polity itself performed the whole rite and attended half of it.
    pub fn attrition(&self) -> usize {
        let first = self.movements.first().map(|m| m.attending.len()).unwrap_or(0);
        first.saturating_sub(self.attended_throughout().len())
    }
}

/// Every rite this polity has performed or begun, oldest first (design **D29**).
///
/// A member's rite is the run of movements since their last movement 1, because
/// opening a rite abandons any attempt in progress. So a member who was started on
/// twice appears twice: the abandoned attempt is part of the record, and a polity
/// that keeps beginning rites it does not finish is saying something about itself.
pub fn rite_state(
    archive: &Archive<CharterEntry>,
    polity: &DidKey,
    as_of: Timestamp,
) -> Vec<Rite> {
    let mut rites: Vec<Rite> = Vec::new();
    for (_cid, e) in archive.iter() {
        match e {
            CharterEntry::RiteMovementRecorded(s) => {
                let m = &s.payload;
                if &m.polity != polity || m.recited_at > as_of {
                    continue;
                }
                let point = MovementPoint {
                    order: m.movement.order(),
                    attending: m.attending.clone(),
                    recited_at: m.recited_at,
                };
                // Movement 1 opens a rite; anything else extends this member's most
                // recent open one.
                let open = rites
                    .iter_mut()
                    .rev()
                    .find(|r| r.member == m.member && r.completed_at.is_none());
                match (point.order, open) {
                    (1, _) | (_, None) => rites.push(Rite {
                        member: m.member.clone(),
                        kind: m.kind,
                        movements: vec![point],
                        completed_at: None,
                    }),
                    (_, Some(r)) => r.movements.push(point),
                }
            }
            CharterEntry::MemberAffirmed(s) => {
                let af = &s.payload;
                if &af.polity != polity || af.affirmed_at > as_of {
                    continue;
                }
                // **D34**: a renewal completes its own re-recitation exactly as an
                // induction completes its rite. Matching on kind is what keeps a
                // completed renewal from reading as an abandoned rite forever, which is
                // the state the measurement found a faithful re-recitation stuck in.
                if let Some(r) = rites.iter_mut().rev().find(|r| {
                    r.member == af.member && r.completed_at.is_none() && r.kind == af.kind
                }) {
                    r.completed_at = Some(af.affirmed_at);
                }
            }
            _ => {}
        }
    }
    rites
}
