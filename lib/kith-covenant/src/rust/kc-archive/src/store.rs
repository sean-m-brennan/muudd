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

//! In-memory archive. Append-only, indexed by entry CID. Verification walks
//! the archive's own state to resolve signer DIDs → public keys.
//!
//! The append-only mechanism itself lives in [`kc_log`] (ethne decision **D32**): this
//! crate holds the *domain* half — which records exist, how a signer's key is found,
//! and what verification means — over a log it no longer implements twice.

use std::collections::BTreeMap;

use kc_core::{Cid, DidKey, KcError, KcResult, PubKey, Timestamp};
use kc_identity::CommunityIdentity;
use kc_log::Archive as Log;

use crate::entry::ArchiveEntry;

pub trait Archive {
    fn append(&mut self, entry: ArchiveEntry) -> KcResult<Cid>;
    fn get(&self, cid: &Cid) -> Option<&ArchiveEntry>;
    fn verify(&self, cid: &Cid) -> KcResult<()>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The archive kith-covenant runs on: [`kc_log::Archive`] over [`ArchiveEntry`], plus
/// this crate's DID resolution and verification.
///
/// Only the surface kith-covenant has a caller for is forwarded. `kc-log` also carries
/// generation-safe cursors, incremental segments, `iter_from`, `contains` and
/// `merge_json`, all of which the ethne polity tier uses for its observation log and
/// none of which kith-covenant has a compaction or incremental-persistence caller for
/// yet. Exposing them here would be API that no rule reads — the same dead-surface
/// pattern as ethne's D23 `substrate.anchor` and this protocol's own
/// `source_commitment`. They are one `pub fn` away when there is something to call them.
#[derive(Debug, Default, Clone)]
pub struct InMemoryArchive {
    log: Log<ArchiveEntry>,
}

impl InMemoryArchive {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append-only iterator in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Cid, &ArchiveEntry)> {
        self.log.iter()
    }

    /// The underlying log, for this crate's own persistence forwarding only.
    pub(crate) fn log(&self) -> &Log<ArchiveEntry> {
        &self.log
    }

    /// Wrap a log loaded from a dump. Crate-private for the same reason: the log is an
    /// implementation detail, not part of this crate's surface.
    pub(crate) fn from_log(log: Log<ArchiveEntry>) -> Self {
        Self { log }
    }

    /// Resolve a DID to a public key by scanning the archive. Scope hints
    /// (`at` time) constrain which delegations/witness entries are active.
    ///
    /// **Every step reads the log in append order** and, where the record kind
    /// supersedes, applies supersession explicitly. Before D32 the four registry steps
    /// scanned the entry index instead and took the first match they found there —
    /// which, the index being keyed by CID, was hash order. Two registry records naming
    /// one DID with different keys therefore resolved to an arbitrary one of them, so a
    /// *superseded* key could still verify a signature; and `authority.rs`'s claim that
    /// the archive resolves keys with "the same latest-issued-wins supersession" was
    /// true only of step 2.
    fn resolve_did(&self, did: &str, at: Timestamp) -> Option<PubKey> {
        // 1. Community root identity always resolves (no time window). Order is
        //    immaterial here and only here: `CommunityIdentity` derives its DID from
        //    its `root_pubkey` and `verify_self_signed` enforces the match, so every
        //    record answering to this DID carries the same key.
        for (_, entry) in self.iter() {
            if let ArchiveEntry::CommunityIdentityPublished(s) = entry {
                if s.payload.did.as_str() == did {
                    return Some(s.payload.root_pubkey.clone());
                }
            }
        }
        // 1b. A chartered compact's own key, on the same terms as a community root and
        //     for the same reason: the charter derives the DID from the key it carries
        //     and the authority pass requires the compact's own signature on it, so
        //     every record answering to this DID carries the same key. Without this step
        //     a compact could be chartered and then never sign anything, since nothing
        //     would resolve its key.
        for (_, entry) in self.iter() {
            if let ArchiveEntry::CompactChartered(s) = entry {
                if s.payload.did.as_str() == did {
                    return Some(s.payload.root_pubkey.clone());
                }
            }
        }
        // 2. Speaker delegation: the record with the greatest `issued_at <= at`
        //    wins (supersession), so an appended revocation — a superseding
        //    record with `revoked_at` set and a later `issued_at` — overrides an
        //    earlier grant. On an `issued_at` tie a revoking record wins. If the
        //    winning record is inactive (revoked/expired) we fall through: the
        //    same key may separately hold a witness role.
        let latest = self
            .iter()
            .filter_map(|(_, e)| match e {
                ArchiveEntry::SpeakerDelegated(s)
                    if s.payload.speaker_did.as_str() == did
                        && s.payload.issued_at <= at =>
                {
                    Some(&s.payload)
                }
                _ => None,
            })
            .max_by_key(|d| (d.issued_at, d.revoked_at.is_some()));
        if let Some(d) = latest {
            if d.active_at(at) {
                return Some(d.speaker_pubkey.clone());
            }
        }
        // 3. Witness registry entry active in its validity window, from the latest
        //    roster each community had issued by `at`.
        for reg in latest_rosters(self.iter().filter_map(|(_, e)| match e {
            ArchiveEntry::WitnessRegistryUpdated(s) => {
                Some((&s.payload.community_did, s.payload.issued_at, &s.payload))
            }
            _ => None,
        }), at) {
            for e in &reg.entries {
                if e.witness_did.as_str() == did
                    && e.valid_from <= at
                    && e.valid_until.is_none_or(|end| at < end)
                {
                    return Some(e.witness_pubkey.clone());
                }
            }
        }
        // 4. Mediator registry entry active in its validity window (§4). Lets a
        //    panel mediator's co-signature on a decision resolve.
        for reg in latest_rosters(self.iter().filter_map(|(_, e)| match e {
            ArchiveEntry::MediatorRegistryUpdated(s) => {
                Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
            }
            _ => None,
        }), at) {
            for e in &reg.entries {
                if e.mediator_did.as_str() == did && e.active_at(at) {
                    return Some(e.mediator_pubkey.clone());
                }
            }
        }
        // 5. Examiner registry entry active in its validity window (§2). Lets a
        //    seated examiner's signature on a credentialing record resolve.
        for reg in latest_rosters(self.iter().filter_map(|(_, e)| match e {
            ArchiveEntry::ExaminerRegistryUpdated(s) => {
                Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
            }
            _ => None,
        }), at) {
            for e in &reg.entries {
                if e.examiner_did.as_str() == did && e.active_at(at) {
                    return Some(e.examiner_pubkey.clone());
                }
            }
        }
        // 6. Observer registry entry active in its validity window (§8). Lets a
        //    registered observer's signature on a report resolve.
        for reg in latest_rosters(self.iter().filter_map(|(_, e)| match e {
            ArchiveEntry::ObserverRegistryUpdated(s) => {
                Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
            }
            _ => None,
        }), at) {
            for e in &reg.entries {
                if e.observer_did.as_str() == did && e.active_at(at) {
                    return Some(e.observer_pubkey.clone());
                }
            }
        }
        None
    }

    /// Look up the most recent self-certified CommunityIdentity for a community DID.
    ///
    /// "Most recent" means **last in append order**. It used to mean `.values().rev()`
    /// over the CID-keyed entry index, which is descending *hash* order and unrelated
    /// to when anything was published: measured over 200 trials with two publications
    /// for one DID, that returned the earlier record 104 times. See `store.rs` in
    /// `kc-log` for why the index is no longer reachable to be read that way.
    pub fn community_identity(&self, did: &DidKey) -> Option<&kc_core::Signed<CommunityIdentity>> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ArchiveEntry::CommunityIdentityPublished(s) if s.payload.did == *did => Some(s),
                _ => None,
            })
            .last()
    }
}

/// The latest roster each nominating community had issued by `at`, in append order.
///
/// One definition for all four registry kinds (§2 examiners, §4 mediators, §8
/// observers, and the witness registry), and the same rule
/// [`crate::authority::mediator_encryption_keys`] already applied to the mediator pool
/// — "later registrations from the same community supersede earlier ones" — so key
/// resolution and the panel rules cannot drift apart. On an `issued_at` tie the later
/// append wins, which is the only tiebreak the log can offer.
pub(crate) fn latest_rosters<'a, R: 'a>(
    records: impl Iterator<Item = (&'a DidKey, Timestamp, &'a R)>,
    at: Timestamp,
) -> Vec<&'a R> {
    let mut latest: BTreeMap<&str, (Timestamp, &'a R)> = BTreeMap::new();
    for (community, issued_at, reg) in records {
        if issued_at > at {
            continue;
        }
        match latest.get(community.as_str()) {
            Some((prev, _)) if *prev > issued_at => {}
            _ => {
                latest.insert(community.as_str(), (issued_at, reg));
            }
        }
    }
    latest.into_values().map(|(_, reg)| reg).collect()
}

impl Archive for InMemoryArchive {
    fn append(&mut self, entry: ArchiveEntry) -> KcResult<Cid> {
        self.log.append(entry)
    }

    fn get(&self, cid: &Cid) -> Option<&ArchiveEntry> {
        self.log.get(cid)
    }

    fn verify(&self, cid: &Cid) -> KcResult<()> {
        let entry = self
            .get(cid)
            .ok_or_else(|| KcError::Archive(format!("not in archive: {cid}")))?;
        match entry {
            ArchiveEntry::CommunityIdentityPublished(s) => {
                CommunityIdentity::verify_self_signed(s)
            }
            ArchiveEntry::SpeakerDelegated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::WitnessRegistryUpdated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MemberRollPublished(s) => {
                let at = s.payload.published_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::IntraCommunityRite(s) => {
                let at = s.payload.sealed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InterCommunityWitness(s) => {
                let at = s.payload.witnessed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InterCommunityNonAdmission(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SponsorshipVouched(s) => {
                let at = s.payload.sponsored_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ConductAttested(s) => {
                let at = s.payload.recorded_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ActivityAttested(s) => {
                let at = s.payload.attested_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ReexaminationConcluded(s) => {
                let at = s.payload.concluded_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::DesignationDissented(s) => {
                let at = s.payload.filed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::NonPursuitAttested(s) => {
                let at = s.payload.filed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MediatorRegistryUpdated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::DisputeFiled(s) => {
                let at = s.payload.filed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ArgumentFiled(s) => {
                let at = s.payload.submitted_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::RecusalRequested(s) => {
                let at = s.payload.requested_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::RecusalDecided(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ProtectingPowerDesignated(s) => {
                let at = s.payload.designated_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::PanelConsentRecorded(s) => {
                let at = s.payload.given_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SkepticismInvoked(s) => {
                let at = s.payload.invoked_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SkepticismReviewed(s) => {
                let at = s.payload.reviewed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MediationDecided(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::DecisionUnsealed(s) => {
                let at = s.payload.unsealed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::LadderAdvanced(s) => {
                let at = s.payload.transitioned_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::DesignationDecided(s) => {
                let at = s.payload.designated_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::RestitutionObligated(s) => {
                let at = s.payload.imposed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::RestitutionVerified(s) => {
                let at = s.payload.verified_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ReEntryCompleted(s) => {
                let at = s.payload.completed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SubstantiveReEngagementInvoked(s) => {
                let at = s.payload.invoked_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InjunctionIssued(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InjunctionConfirmed(s) => {
                let at = s.payload.confirmed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InjunctionChallenged(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::CommonsRegistered(s) => {
                let at = s.payload.registered_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::UsageAccounted(s) => {
                let at = s.payload.recorded_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::UsageOpened(s) => {
                let at = s.payload.opened_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::CommonsEscalated(s) => {
                let at = s.payload.escalated_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::CompactChartered(s) => {
                let at = s.payload.chartered_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SanctionApplied(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ExaminerRegistryUpdated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ObserverRegistryUpdated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ObserverReportPublished(s) => {
                let at = s.payload.reported_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ReportResponsePublished(s) => {
                let at = s.payload.responded_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::WhistleblowerFiled(s) => {
                let at = s.payload.filed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SourceRevealed(s) => {
                let at = s.payload.revealed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SourceExculpated(s) => {
                let at = s.payload.exculpated_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::ConstitutionRatified(s) => {
                let at = s.payload.genesis_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::CompactIncapableAmended(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::FastCadenceAmended(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MinimumContentAdopted(s) => {
                let at = s.payload.adopted_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MinimumContentAmended(s) => {
                let at = s.payload.decided_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::SubstrateInterpreted(s) => {
                let at = s.payload.recorded_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::InterpreterIndependenceAttested(s) => {
                let at = s.payload.attested_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::EventReported(s) => {
                let at = s.payload.reported_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::EventCorroborated(s) => {
                let at = s.payload.corroborated_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::EventActionInvoked(s) => {
                let at = s.payload.invoked_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::RetroactiveInterestFound(s) => {
                let at = s.payload.found_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::PrincipalWitnessed(s) => {
                let at = s.payload.attested_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::PrincipalOutputRecorded(s) => {
                let at = s.payload.produced_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
        }
    }

    fn len(&self) -> usize {
        self.log.len()
    }
}
