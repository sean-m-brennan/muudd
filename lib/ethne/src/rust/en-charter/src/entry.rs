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
//! founding boundary, the substrate anchor's withdrawal, third-party accounts of
//! whether a reader of the founding was independent, and dissolution. That is every act the tier records, so this **is** the
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
    Affirmation, BoundaryDissent, CharterDissolution, CommitmentAmendment, ContentPolicyAmendment, EnvoyAuthorization,
    GuardianChange, GuardianEvidence, MembershipChange, Observation, ObservationPruning,
    OfficeChange, PolityCharter, RelayEndpoint, RiteMovement,
    IndependenceAttestation, SubstrateDisavowal,
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
    /// One movement of one member's induction rite — the rite as it happens, rather
    /// than only its completion (design **D29**).
    RiteMovementRecorded(Signed<RiteMovement>),
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
    /// A third party's account of whether a named party behind the polity — a
    /// founding boundary reader, or since **D28** a monitor whose observations it is
    /// graded on — is an independent principal (design **D24**). Evidence about the
    /// polity, like `BoundaryDissented` and `SubstrateDisavowed`: it changes no state,
    /// survives dissolution, and is counted by a lens rather than enforced by a rule.
    IndependenceAttested(Signed<IndependenceAttestation>),
    /// A rendezvous operator's own statement of where its relay listens, or its
    /// withdrawal (design **D36**). A governing act's shadow: the office is the
    /// roll's decision, and this only says where the decided service runs.
    RelayEndpointDeclared(Signed<RelayEndpoint>),
    /// The roll replacing the polity's content policy (design **D37**): a majority
    /// to tighten it, more than a majority to loosen it.
    ContentPolicyAmended(Signed<ContentPolicyAmendment>),
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
            CharterEntry::RiteMovementRecorded(s) => &s.signatures,
            CharterEntry::Observed(s) => &s.signatures,
            CharterEntry::BoundaryDissented(s) => &s.signatures,
            CharterEntry::SubstrateDisavowed(s) => &s.signatures,
            CharterEntry::IndependenceAttested(s) => &s.signatures,
            CharterEntry::RelayEndpointDeclared(s) => &s.signatures,
            CharterEntry::ContentPolicyAmended(s) => &s.signatures,
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
            CharterEntry::RiteMovementRecorded(s) => &s.payload.polity,
            CharterEntry::Observed(s) => &s.payload.polity,
            CharterEntry::BoundaryDissented(s) => &s.payload.polity,
            CharterEntry::SubstrateDisavowed(s) => &s.payload.polity,
            CharterEntry::IndependenceAttested(s) => &s.payload.polity,
            CharterEntry::RelayEndpointDeclared(s) => &s.payload.polity,
            CharterEntry::ContentPolicyAmended(s) => &s.payload.polity,
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
            CharterEntry::RiteMovementRecorded(_) => "RiteMovementRecorded",
            CharterEntry::Observed(_) => "Observed",
            CharterEntry::BoundaryDissented(_) => "BoundaryDissented",
            CharterEntry::SubstrateDisavowed(_) => "SubstrateDisavowed",
            CharterEntry::IndependenceAttested(_) => "IndependenceAttested",
            CharterEntry::RelayEndpointDeclared(_) => "RelayEndpointDeclared",
            CharterEntry::ContentPolicyAmended(_) => "ContentPolicyAmended",
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
        // A charter carries two kinds of evidence signed by somebody other than its
        // ratifiers, and both are verified here (D27).
        //
        // **D22's boundary readings.** The authority pass requires a reading's signer
        // set to equal the reader it names — but that is the signer *label*, and
        // nothing was ever checking the signature itself. Measured: a reading signed
        // with an attacker's key and labelled with a victim's DID passed `verify_all`
        // and `check_authority` both. The comment on the `GuardianChanged` arm below
        // asserted this was already handled; it was not. D22 exists because "a reading
        // nobody signed cannot be told apart from a reading nobody took", and an
        // unverified signature is exactly a reading nobody signed.
        //
        // **D27's founding guardian evidence**, for the same reason the
        // `GuardianChanged` arm verifies its own: otherwise the grounded rule and the
        // `Guarded` grade read a forgery.
        CharterEntry::PolityChartered(s) => {
            s.verify(didkey_resolver)?;
            for rd in s.payload.attestation.source.readings() {
                rd.verify(didkey_resolver)?;
            }
            for d in &s.payload.derivations {
                if let Some(GuardianEvidence::Observed(ob)) = &d.evidence {
                    ob.verify(didkey_resolver)?;
                }
            }
            Ok(())
        }
        CharterEntry::CommitmentAmended(s) => s.verify(didkey_resolver),
        CharterEntry::MembershipChanged(s) => s.verify(didkey_resolver),
        CharterEntry::OfficeChanged(s) => s.verify(didkey_resolver),
        // The carried substrate evidence is signed by its observer, so it is
        // verified here alongside the record's own signatures — otherwise a record
        // could carry an observation nobody actually signed and still pass the crypto
        // pass, leaving the authority rule reading a forgery. (D22's carried readings
        // are checked the same way since D27 — when this comment was first written
        // they were not, which is what D27's measurement caught.)
        CharterEntry::GuardianChanged(s) => {
            s.verify(didkey_resolver)?;
            if let Some(GuardianEvidence::Observed(ob)) = &s.payload.evidence {
                ob.verify(didkey_resolver)?;
            }
            Ok(())
        }
        CharterEntry::EnvoyAuthorized(s) => s.verify(didkey_resolver),
        CharterEntry::MemberAffirmed(s) => s.verify(didkey_resolver),
        CharterEntry::RiteMovementRecorded(s) => s.verify(didkey_resolver),
        CharterEntry::Observed(s) => s.verify(didkey_resolver),
        CharterEntry::BoundaryDissented(s) => s.verify(didkey_resolver),
        CharterEntry::SubstrateDisavowed(s) => s.verify(didkey_resolver),
        CharterEntry::IndependenceAttested(s) => s.verify(didkey_resolver),
        CharterEntry::RelayEndpointDeclared(s) => s.verify(didkey_resolver),
        CharterEntry::ContentPolicyAmended(s) => s.verify(didkey_resolver),
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
