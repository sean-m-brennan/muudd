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

//! Community identity record. Self-signed by the community's root key.
//!
//! The community's DID (did:key form) is derived from its root public key,
//! so the identity is fully self-certifying: verification needs only the
//! embedded `root_pubkey` and the envelope's signature.
//!
//! MVP note: successor chain is defined but not exercised in the demo.
//! did:key tightly couples DID to key material, so rotating the root key
//! produces a new DID linked to the previous via a `SuccessorLink` signed
//! by the prior root. Community-stable-DID across rotations would require
//! a different DID method (did:web, did:ion) — post-MVP.

use kc_core::{DidKey, KcError, KcResult, PrivKey, PubKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommunityIdentity {
    pub name: String,
    pub did: DidKey,
    pub root_pubkey: PubKey,
    pub successor_chain: Vec<Signed<SuccessorLink>>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessorLink {
    pub previous_did: DidKey,
    pub new_did: DidKey,
    pub new_pubkey: PubKey,
    pub rotated_at: Timestamp,
}

impl CommunityIdentity {
    /// Create a self-signed `Signed<CommunityIdentity>` with an empty
    /// successor chain. `did` is derived from `root_pub`.
    pub fn new_self_signed(
        name: String,
        root_priv: &PrivKey,
        root_pub: PubKey,
        created_at: Timestamp,
    ) -> KcResult<Signed<Self>> {
        let did = DidKey::from_pubkey(&root_pub)?;
        let payload = CommunityIdentity {
            name,
            did: did.clone(),
            root_pubkey: root_pub,
            successor_chain: Vec::new(),
            created_at,
        };
        let did_str = did.into_string();
        Signed::build(payload, &[(did_str.as_str(), root_priv)])
    }

    /// Verify a `Signed<CommunityIdentity>` is self-signed by its embedded
    /// `root_pubkey`. Also checks the DID matches the pubkey.
    pub fn verify_self_signed(signed: &Signed<Self>) -> KcResult<()> {
        let declared_did = &signed.payload.did;
        let derived_did = DidKey::from_pubkey(&signed.payload.root_pubkey)?;
        if &derived_did != declared_did {
            return Err(KcError::InvalidDid(format!(
                "declared DID {declared_did} does not match pubkey-derived DID {derived_did}"
            )));
        }
        let embedded = signed.payload.root_pubkey.clone();
        let self_did = declared_did.as_str().to_string();
        signed.verify(move |did: &str| {
            if did == self_did {
                Some(embedded.clone())
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{keys, time, Alg, Cid};

    fn make_community(name: &str) -> (Signed<CommunityIdentity>, PrivKey, PubKey) {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let signed = CommunityIdentity::new_self_signed(
            name.to_string(),
            &sk,
            pk.clone(),
            time::now(),
        )
        .unwrap();
        (signed, sk, pk)
    }

    #[test]
    fn self_signed_verifies() {
        let (signed, _, _) = make_community("Alpha");
        CommunityIdentity::verify_self_signed(&signed).unwrap();
    }

    #[test]
    fn tampered_name_breaks_cid() {
        let (mut signed, _, _) = make_community("Alpha");
        signed.payload.name = "NotAlpha".into();
        // The CID was computed over the original payload; changing the payload
        // without recomputing must fail CID check.
        assert!(matches!(
            CommunityIdentity::verify_self_signed(&signed),
            Err(KcError::CidMismatch { .. })
        ));
    }

    #[test]
    fn tampered_root_pubkey_breaks_did_derivation() {
        let (mut signed, _, _) = make_community("Alpha");
        let (_, other_pk) = keys::generate(Alg::Ed25519).unwrap();
        signed.payload.root_pubkey = other_pk;
        signed.payload_cid = Cid::of(&signed.payload).unwrap();
        // Payload's DID no longer matches its root_pubkey-derived DID.
        assert!(matches!(
            CommunityIdentity::verify_self_signed(&signed),
            Err(KcError::InvalidDid(_))
        ));
    }

    #[test]
    fn distinct_communities_have_distinct_dids() {
        let (a, _, _) = make_community("Alpha");
        let (b, _, _) = make_community("Beta");
        assert_ne!(a.payload.did, b.payload.did);
    }

    #[test]
    fn did_matches_pubkey() {
        let (signed, _, pk) = make_community("Gamma");
        assert_eq!(
            signed.payload.did,
            DidKey::from_pubkey(&pk).unwrap()
        );
    }
}
