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
//! (conferral / resignation / revocation), which members are derived (machine)
//! parties and the human each answers to, who the polity authorizes to speak for
//! it beyond itself, what its monitors observed, the pruning of that observation
//! record, members' induction / re-affirmation of the substrate, dissent from the
//! founding boundary, and dissolution. That is every act the tier records, so this **is** the
//! ethne-wide entry enum — earlier drafts of this note promised a later slice
//! would fold a founding-slice type together with the rest, and there is nothing
//! left to fold. Cohesion is not among them and never will be: a `CohesionReport`
//! is *derived* from these entries rather than appended alongside them, which is
//! why it is not even `Serialize`. Signature verification is self-contained: every
//! signer is a `did:key`, so its public key is recovered from the DID itself —
//! no registry, and no genesis chicken-and-egg.

use std::collections::BTreeSet;
use std::str::FromStr;

use en_archive::Entry;
use en_core::{DidKey, KcResult, PubKey, Signed};
use serde::{Deserialize, Serialize};

use crate::record::{
    Affirmation, BoundaryDissent, CharterDissolution, CommitmentAmendment, EnvoyAuthorization,
    GuardianChange, MembershipChange, Observation, ObservationPruning, OfficeChange, PolityCharter,
    SubstrateDisavowal,
};

/// An entry in a polity's charter log.
///
/// `PartialEq` but not `Eq`: an [`Observation`] carries reputation readings, which
/// are floats, and claiming a total equality over them would be a lie. Content
/// addressing does not depend on it — a `Cid` is taken over the serialized bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CharterEntry {
    /// The founding of a polity (genesis singleton).
    PolityChartered(Signed<PolityCharter>),
    /// An addition or removal of a non-founding commitment.
    CommitmentAmended(Signed<CommitmentAmendment>),
    /// An admission, voluntary departure, or expulsion changing the roll.
    MembershipChanged(Signed<MembershipChange>),
    /// A conferral, resignation, or revocation of an internal office.
    OfficeChanged(Signed<OfficeChange>),
    /// A designation, rotation, or release of a member's guardian edge.
    GuardianChanged(Signed<GuardianChange>),
    /// An authorization, resignation, or revocation of an envoy's standing to
    /// speak for the polity beyond itself.
    EnvoyAuthorized(Signed<EnvoyAuthorization>),
    /// A member's induction into or re-affirmation of the substrate.
    MemberAffirmed(Signed<Affirmation>),
    /// What one monitor saw at one instant — perspectival evidence, not a decision.
    Observed(Signed<Observation>),
    /// A ratified dropping of observations older than a horizon, with a receipt for
    /// what went. The log may forget, but only by saying so.
    ObservationsPruned(Signed<ObservationPruning>),
    /// One excluded party's answer to the founding boundary — perspectival
    /// evidence, like `Observed`, and the only entry a dissolved polity still
    /// accepts (design **D22**).
    BoundaryDissented(Signed<BoundaryDissent>),
    /// The chartered substrate anchor's own withdrawal — the ground saying it does
    /// not stand above this polity (design **D23**). Like `BoundaryDissented` it
    /// asserts rather than decides, survives dissolution, and changes no
    /// reconstructed state: the substrate is genesis-immutable.
    SubstrateDisavowed(Signed<SubstrateDisavowal>),
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
            CharterEntry::GuardianChanged(s) => &s.signatures,
            CharterEntry::EnvoyAuthorized(s) => &s.signatures,
            CharterEntry::MemberAffirmed(s) => &s.signatures,
            CharterEntry::Observed(s) => &s.signatures,
            CharterEntry::BoundaryDissented(s) => &s.signatures,
            CharterEntry::SubstrateDisavowed(s) => &s.signatures,
            CharterEntry::ObservationsPruned(s) => &s.signatures,
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
            CharterEntry::GuardianChanged(s) => &s.payload.polity,
            CharterEntry::EnvoyAuthorized(s) => &s.payload.polity,
            CharterEntry::MemberAffirmed(s) => &s.payload.polity,
            CharterEntry::Observed(s) => &s.payload.polity,
            CharterEntry::BoundaryDissented(s) => &s.payload.polity,
            CharterEntry::SubstrateDisavowed(s) => &s.payload.polity,
            CharterEntry::ObservationsPruned(s) => &s.payload.polity,
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
            CharterEntry::GuardianChanged(_) => "GuardianChanged",
            CharterEntry::EnvoyAuthorized(_) => "EnvoyAuthorized",
            CharterEntry::MemberAffirmed(_) => "MemberAffirmed",
            CharterEntry::Observed(_) => "Observed",
            CharterEntry::BoundaryDissented(_) => "BoundaryDissented",
            CharterEntry::SubstrateDisavowed(_) => "SubstrateDisavowed",
            CharterEntry::ObservationsPruned(_) => "ObservationsPruned",
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
        CharterEntry::GuardianChanged(s) => s.verify(didkey_resolver),
        CharterEntry::EnvoyAuthorized(s) => s.verify(didkey_resolver),
        CharterEntry::MemberAffirmed(s) => s.verify(didkey_resolver),
        CharterEntry::Observed(s) => s.verify(didkey_resolver),
        CharterEntry::BoundaryDissented(s) => s.verify(didkey_resolver),
        CharterEntry::SubstrateDisavowed(s) => s.verify(didkey_resolver),
        CharterEntry::ObservationsPruned(s) => s.verify(didkey_resolver),
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
