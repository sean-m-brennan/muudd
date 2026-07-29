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

//! Signing algorithm dispatch.
//!
//! MVP uses Ed25519 only, chosen on trustworthiness grounds: well-understood
//! elliptic-curve construction, no known backdoors, long track record. NIST
//! lattice-based PQC schemes (ML-DSA/Dilithium) are deferred pending community
//! confidence in their provenance and implementation maturity.
//!
//! `Alg` is an enum with a single variant today so signed payloads carry an
//! algorithm tag and the dispatch shape is preserved for a future second
//! variant (likely a hash-based PQC scheme like SLH-DSA if one is adopted).

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::error::{KcError, KcResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Alg {
    Ed25519,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PubKey {
    pub alg: Alg,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivKey {
    pub alg: Alg,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

impl std::fmt::Debug for PrivKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrivKey")
            .field("alg", &self.alg)
            .field("bytes", &format_args!("<{} bytes redacted>", self.bytes.len()))
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    pub alg: Alg,
    #[serde(with = "serde_bytes")]
    pub bytes: Vec<u8>,
}

pub fn generate(alg: Alg) -> KcResult<(PrivKey, PubKey)> {
    match alg {
        Alg::Ed25519 => {
            let mut csprng = OsRng;
            let sk = SigningKey::generate(&mut csprng);
            let pk = sk.verifying_key();
            Ok((
                PrivKey { alg, bytes: sk.to_bytes().to_vec() },
                PubKey { alg, bytes: pk.to_bytes().to_vec() },
            ))
        }
    }
}

/// The public key belonging to a private key.
///
/// Needed wherever a holder must find *itself* in a published list of keys — a ring
/// member locating its own position, a share holder matching its sealed share —
/// without the caller having to carry the public half alongside the private one.
pub fn public_of(priv_key: &PrivKey) -> KcResult<PubKey> {
    match priv_key.alg {
        Alg::Ed25519 => {
            let sk_bytes: [u8; 32] = priv_key.bytes.as_slice().try_into().map_err(|_| {
                KcError::Key(format!(
                    "ed25519 secret key must be 32 bytes, got {}",
                    priv_key.bytes.len()
                ))
            })?;
            let sk = SigningKey::from_bytes(&sk_bytes);
            Ok(PubKey { alg: Alg::Ed25519, bytes: sk.verifying_key().to_bytes().to_vec() })
        }
    }
}

pub fn sign(priv_key: &PrivKey, message: &[u8]) -> KcResult<Signature> {
    match priv_key.alg {
        Alg::Ed25519 => {
            let sk_bytes: [u8; 32] = priv_key.bytes.as_slice().try_into().map_err(|_| {
                KcError::Key(format!(
                    "ed25519 secret key must be 32 bytes, got {}",
                    priv_key.bytes.len()
                ))
            })?;
            let sk = SigningKey::from_bytes(&sk_bytes);
            let sig = sk.sign(message);
            Ok(Signature { alg: Alg::Ed25519, bytes: sig.to_bytes().to_vec() })
        }
    }
}

pub fn verify(pub_key: &PubKey, message: &[u8], signature: &Signature) -> KcResult<()> {
    if pub_key.alg != signature.alg {
        return Err(KcError::UnsupportedAlg(format!(
            "key alg {:?} does not match signature alg {:?}",
            pub_key.alg, signature.alg
        )));
    }
    match pub_key.alg {
        Alg::Ed25519 => {
            let pk_bytes: [u8; 32] = pub_key.bytes.as_slice().try_into().map_err(|_| {
                KcError::Key(format!(
                    "ed25519 public key must be 32 bytes, got {}",
                    pub_key.bytes.len()
                ))
            })?;
            let pk = VerifyingKey::from_bytes(&pk_bytes)
                .map_err(|e| KcError::Key(format!("ed25519 pk: {e}")))?;
            let sig_bytes: [u8; 64] = signature.bytes.as_slice().try_into().map_err(|_| {
                KcError::Key(format!(
                    "ed25519 signature must be 64 bytes, got {}",
                    signature.bytes.len()
                ))
            })?;
            let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);
            pk.verify(message, &sig).map_err(|_| KcError::SignatureInvalid)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_roundtrip() {
        let (sk, pk) = generate(Alg::Ed25519).unwrap();
        let msg = b"kith-covenant test message";
        let sig = sign(&sk, msg).unwrap();
        verify(&pk, msg, &sig).expect("valid signature should verify");
    }

    #[test]
    fn tampered_message_fails() {
        let (sk, pk) = generate(Alg::Ed25519).unwrap();
        let sig = sign(&sk, b"original").unwrap();
        assert!(matches!(
            verify(&pk, b"tampered", &sig),
            Err(KcError::SignatureInvalid)
        ));
    }

    #[test]
    fn wrong_public_key_fails() {
        let (sk1, _) = generate(Alg::Ed25519).unwrap();
        let (_, pk2) = generate(Alg::Ed25519).unwrap();
        let sig = sign(&sk1, b"msg").unwrap();
        assert!(matches!(
            verify(&pk2, b"msg", &sig),
            Err(KcError::SignatureInvalid)
        ));
    }

    #[test]
    fn privkey_debug_redacts_bytes() {
        let (sk, _) = generate(Alg::Ed25519).unwrap();
        let dbg = format!("{sk:?}");
        assert!(dbg.contains("redacted"));
        assert!(!dbg.contains(&format!("{:?}", sk.bytes)));
    }

    #[test]
    fn public_of_recovers_the_generated_public_key() {
        let (sk, pk) = generate(Alg::Ed25519).unwrap();
        assert_eq!(public_of(&sk).unwrap(), pk);
        let (other_sk, _) = generate(Alg::Ed25519).unwrap();
        assert_ne!(public_of(&other_sk).unwrap(), pk);
    }

    #[test]
    fn pubkey_roundtrips_through_canonical_cbor() {
        use crate::canonical::{from_canonical_cbor, to_canonical_cbor};
        let (_, pk) = generate(Alg::Ed25519).unwrap();
        let bytes = to_canonical_cbor(&pk).unwrap();
        let decoded: PubKey = from_canonical_cbor(&bytes).unwrap();
        assert_eq!(pk, decoded);
    }
}
