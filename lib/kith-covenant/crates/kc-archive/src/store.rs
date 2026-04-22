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
        for entry in self.entries.values() {
            match entry {
                ArchiveEntry::CommunityIdentityPublished(s) => {
                    if s.payload.did.as_str() == did {
                        return Some(s.payload.root_pubkey.clone());
                    }
                }
                ArchiveEntry::SpeakerDelegated(s) => {
                    let d = &s.payload;
                    if d.speaker_did.as_str() == did && d.active_at(at) {
                        return Some(d.speaker_pubkey.clone());
                    }
                }
                ArchiveEntry::WitnessRegistryUpdated(s) => {
                    for e in &s.payload.entries {
                        if e.witness_did.as_str() == did
                            && e.valid_from <= at
                            && e.valid_until.map_or(true, |end| at < end)
                        {
                            return Some(e.witness_pubkey.clone());
                        }
                    }
                }
                _ => {}
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
        }
    }

    fn len(&self) -> usize {
        self.order.len()
    }
}
