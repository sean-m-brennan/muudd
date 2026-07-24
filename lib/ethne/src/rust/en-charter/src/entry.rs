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

//! The charter archive-entry enum, its content-addressing, and signature
//! verification.
//!
//! `CharterEntry` covers the founding, its commitment amendments, the membership
//! roll's lifecycle (admission / departure / expulsion), the internal offices
//! (conferral / resignation / revocation), members' induction / re-affirmation of
//! the substrate, and dissolution; a later slice folds it (and cohesion records)
//! into a single ethne-wide entry enum. Signature verification is self-contained:
//! every
//! signer is a `did:key`, so its public key is recovered from the DID itself —
//! no registry, and no genesis chicken-and-egg.

use std::collections::BTreeSet;
use std::str::FromStr;

use en_archive::Entry;
use en_core::{DidKey, KcResult, PubKey, Signed};
use serde::{Deserialize, Serialize};

use crate::record::{
    Affirmation, CharterDissolution, CommitmentAmendment, MembershipChange, OfficeChange,
    PolityCharter,
};

/// An entry in a polity's charter log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharterEntry {
    /// The founding of a polity (genesis singleton).
    PolityChartered(Signed<PolityCharter>),
    /// An addition or removal of a non-founding commitment.
    CommitmentAmended(Signed<CommitmentAmendment>),
    /// An admission, voluntary departure, or expulsion changing the roll.
    MembershipChanged(Signed<MembershipChange>),
    /// A conferral, resignation, or revocation of an internal office.
    OfficeChanged(Signed<OfficeChange>),
    /// A member's induction into or re-affirmation of the substrate.
    MemberAffirmed(Signed<Affirmation>),
    /// The deliberate winding-up of the polity (terminal; may name successors).
    CharterDissolved(Signed<CharterDissolution>),
}

impl CharterEntry {
    /// The DIDs that signed this entry.
    pub fn signer_dids(&self) -> BTreeSet<String> {
        let sigs = match self {
            CharterEntry::PolityChartered(s) => &s.signatures,
            CharterEntry::CommitmentAmended(s) => &s.signatures,
            CharterEntry::MembershipChanged(s) => &s.signatures,
            CharterEntry::OfficeChanged(s) => &s.signatures,
            CharterEntry::MemberAffirmed(s) => &s.signatures,
            CharterEntry::CharterDissolved(s) => &s.signatures,
        };
        sigs.iter().map(|e| e.signer_did.clone()).collect()
    }

    /// The polity this entry concerns.
    pub fn polity(&self) -> &DidKey {
        match self {
            CharterEntry::PolityChartered(s) => &s.payload.polity,
            CharterEntry::CommitmentAmended(s) => &s.payload.polity,
            CharterEntry::MembershipChanged(s) => &s.payload.polity,
            CharterEntry::OfficeChanged(s) => &s.payload.polity,
            CharterEntry::MemberAffirmed(s) => &s.payload.polity,
            CharterEntry::CharterDissolved(s) => &s.payload.polity,
        }
    }
}

impl Entry for CharterEntry {
    fn kind(&self) -> &'static str {
        match self {
            CharterEntry::PolityChartered(_) => "PolityChartered",
            CharterEntry::CommitmentAmended(_) => "CommitmentAmended",
            CharterEntry::MembershipChanged(_) => "MembershipChanged",
            CharterEntry::OfficeChanged(_) => "OfficeChanged",
            CharterEntry::MemberAffirmed(_) => "MemberAffirmed",
            CharterEntry::CharterDissolved(_) => "CharterDissolved",
        }
    }
}

/// Resolve a `did:key` signer to its public key directly from the DID. Every
/// ethne signer is a `did:key`, so no registry lookup is needed.
pub fn didkey_resolver(did: &str) -> Option<PubKey> {
    DidKey::from_str(did).ok().and_then(|d| d.to_pubkey().ok())
}

/// Verify one entry's signatures (CID match + every signature valid).
pub fn verify_entry(entry: &CharterEntry) -> KcResult<()> {
    match entry {
        CharterEntry::PolityChartered(s) => s.verify(didkey_resolver),
        CharterEntry::CommitmentAmended(s) => s.verify(didkey_resolver),
        CharterEntry::MembershipChanged(s) => s.verify(didkey_resolver),
        CharterEntry::OfficeChanged(s) => s.verify(didkey_resolver),
        CharterEntry::MemberAffirmed(s) => s.verify(didkey_resolver),
        CharterEntry::CharterDissolved(s) => s.verify(didkey_resolver),
    }
}

/// Verify every entry in the archive.
pub fn verify_all(archive: &en_archive::Archive<CharterEntry>) -> KcResult<()> {
    for (_cid, e) in archive.iter() {
        verify_entry(e)?;
    }
    Ok(())
}
