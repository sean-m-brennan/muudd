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

use std::collections::BTreeMap;

use kc_core::{Cid, DidKey, KcError, KcResult, PubKey, Timestamp};
use kc_identity::CommunityIdentity;

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

#[derive(Debug, Default, Clone)]
pub struct InMemoryArchive {
    order: Vec<Cid>,
    entries: BTreeMap<String, ArchiveEntry>,
}

impl InMemoryArchive {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append-only iterator in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&Cid, &ArchiveEntry)> {
        self.order
            .iter()
            .map(|cid| (cid, self.entries.get(&cid.to_string()).expect("index invariant")))
    }

    /// Resolve a DID to a public key by scanning the archive. Scope hints
    /// (`at` time) constrain which delegations/witness entries are active.
    fn resolve_did(&self, did: &str, at: Timestamp) -> Option<PubKey> {
        // 1. Community root identity always resolves (no time window).
        for entry in self.entries.values() {
            if let ArchiveEntry::CommunityIdentityPublished(s) = entry {
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
            .entries
            .values()
            .filter_map(|e| match e {
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
        // 3. Witness registry entry active in its validity window.
        for entry in self.entries.values() {
            if let ArchiveEntry::WitnessRegistryUpdated(s) = entry {
                for e in &s.payload.entries {
                    if e.witness_did.as_str() == did
                        && e.valid_from <= at
                        && e.valid_until.map_or(true, |end| at < end)
                    {
                        return Some(e.witness_pubkey.clone());
                    }
                }
            }
        }
        // 4. Mediator registry entry active in its validity window (§4). Lets a
        //    panel mediator's co-signature on a decision resolve.
        for entry in self.entries.values() {
            if let ArchiveEntry::MediatorRegistryUpdated(s) = entry {
                for e in &s.payload.entries {
                    if e.mediator_did.as_str() == did && e.active_at(at) {
                        return Some(e.mediator_pubkey.clone());
                    }
                }
            }
        }
        None
    }

    /// Look up the most recent self-certified CommunityIdentity for a community DID.
    pub fn community_identity(&self, did: &DidKey) -> Option<&kc_core::Signed<CommunityIdentity>> {
        self.entries.values().rev().find_map(|e| match e {
            ArchiveEntry::CommunityIdentityPublished(s) if s.payload.did == *did => Some(s),
            _ => None,
        })
    }
}

impl Archive for InMemoryArchive {
    fn append(&mut self, entry: ArchiveEntry) -> KcResult<Cid> {
        let cid = entry.cid()?;
        let key = cid.to_string();
        if self.entries.contains_key(&key) {
            return Err(KcError::Archive(format!("duplicate entry: {cid}")));
        }
        self.entries.insert(key, entry);
        self.order.push(cid);
        Ok(cid)
    }

    fn get(&self, cid: &Cid) -> Option<&ArchiveEntry> {
        self.entries.get(&cid.to_string())
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
            ArchiveEntry::MediatorRegistryUpdated(s) => {
                let at = s.payload.issued_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::DisputeFiled(s) => {
                let at = s.payload.filed_at;
                s.verify(|did: &str| self.resolve_did(did, at))
            }
            ArchiveEntry::MediationDecided(s) => {
                let at = s.payload.decided_at;
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
        }
    }

    fn len(&self) -> usize {
        self.order.len()
    }
}
