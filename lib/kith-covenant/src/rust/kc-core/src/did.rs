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

//! `did:key` method — minimal Ed25519 implementation.
//!
//! Form: `did:key:z<base58btc(multicodec_prefix || raw_pubkey)>` per the W3C
//! did:key method spec. Ed25519 multicodec is 0xed, varint-encoded as
//! `[0xed, 0x01]`, so every Ed25519 did:key string begins with `did:key:z6Mk`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{KcError, KcResult};
use crate::keys::{Alg, PubKey};

/// Varint encoding of multicodec 0xed (ed25519-pub): two bytes.
const ED25519_MULTICODEC_VARINT: [u8; 2] = [0xed, 0x01];

const DID_KEY_PREFIX: &str = "did:key:z";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DidKey(String);

impl DidKey {
    pub fn from_pubkey(pk: &PubKey) -> KcResult<Self> {
        match pk.alg {
            Alg::Ed25519 => {
                if pk.bytes.len() != 32 {
                    return Err(KcError::InvalidDid(format!(
                        "ed25519 public key must be 32 bytes, got {}",
                        pk.bytes.len()
                    )));
                }
                let mut raw = Vec::with_capacity(2 + 32);
                raw.extend_from_slice(&ED25519_MULTICODEC_VARINT);
                raw.extend_from_slice(&pk.bytes);
                let encoded = bs58::encode(&raw)
                    .with_alphabet(bs58::Alphabet::BITCOIN)
                    .into_string();
                Ok(DidKey(format!("{DID_KEY_PREFIX}{encoded}")))
            }
        }
    }

    pub fn to_pubkey(&self) -> KcResult<PubKey> {
        let body = self.0.strip_prefix(DID_KEY_PREFIX).ok_or_else(|| {
            KcError::InvalidDid(format!(
                "expected '{DID_KEY_PREFIX}' prefix, got: {}",
                self.0
            ))
        })?;
        let raw = bs58::decode(body)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_vec()
            .map_err(|e| KcError::InvalidDid(format!("base58: {e}")))?;
        if raw.len() < 2 {
            return Err(KcError::InvalidDid(format!(
                "did:key payload too short: {} bytes",
                raw.len()
            )));
        }
        if raw[0..2] != ED25519_MULTICODEC_VARINT {
            return Err(KcError::InvalidDid(format!(
                "unsupported multicodec prefix: {:02x?} (only ed25519 0xed supported)",
                &raw[0..2]
            )));
        }
        let key_bytes = raw[2..].to_vec();
        if key_bytes.len() != 32 {
            return Err(KcError::InvalidDid(format!(
                "ed25519 did:key must carry 32-byte key, got {}",
                key_bytes.len()
            )));
        }
        Ok(PubKey {
            alg: Alg::Ed25519,
            bytes: key_bytes,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for DidKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for DidKey {
    type Err = KcError;

    fn from_str(s: &str) -> KcResult<Self> {
        if !s.starts_with(DID_KEY_PREFIX) {
            return Err(KcError::InvalidDid(format!(
                "expected '{DID_KEY_PREFIX}' prefix, got: {s}"
            )));
        }
        let candidate = DidKey(s.to_string());
        // Round-trip decode to validate payload.
        candidate.to_pubkey()?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    #[test]
    fn ed25519_roundtrip() {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        let decoded = did.to_pubkey().unwrap();
        assert_eq!(pk, decoded);
    }

    #[test]
    #[allow(non_snake_case)]
    fn ed25519_did_starts_with_z6Mk() {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        assert!(
            did.as_str().starts_with("did:key:z6Mk"),
            "unexpected did:key form: {did}"
        );
    }

    #[test]
    fn from_str_accepts_valid() {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        let s = did.as_str().to_string();
        let parsed: DidKey = s.parse().unwrap();
        assert_eq!(did, parsed);
    }

    #[test]
    fn from_str_rejects_bad_prefix() {
        let e = "did:example:123".parse::<DidKey>().unwrap_err();
        assert!(matches!(e, KcError::InvalidDid(_)));
    }

    #[test]
    fn from_str_rejects_bad_base58() {
        let e = "did:key:z!!!not-base58!!!".parse::<DidKey>().unwrap_err();
        assert!(matches!(e, KcError::InvalidDid(_)));
    }

    #[test]
    fn from_str_rejects_wrong_multicodec() {
        // Construct a did:key with multicodec 0x00 (identity) instead of ed25519.
        let fake = {
            let mut raw = vec![0x00, 0x01];
            raw.extend_from_slice(&[0u8; 32]);
            bs58::encode(&raw)
                .with_alphabet(bs58::Alphabet::BITCOIN)
                .into_string()
        };
        let s = format!("did:key:z{fake}");
        let e = s.parse::<DidKey>().unwrap_err();
        assert!(matches!(e, KcError::InvalidDid(msg) if msg.contains("multicodec")));
    }

    #[test]
    fn distinct_keys_have_distinct_dids() {
        let (_, pk1) = keys::generate(Alg::Ed25519).unwrap();
        let (_, pk2) = keys::generate(Alg::Ed25519).unwrap();
        let d1 = DidKey::from_pubkey(&pk1).unwrap();
        let d2 = DidKey::from_pubkey(&pk2).unwrap();
        assert_ne!(d1, d2);
    }

    #[test]
    fn serde_transparent_as_string() {
        // `serde(transparent)` means the DidKey encodes as the inner string, not as a struct.
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        let cbor = crate::canonical::to_canonical_cbor(&did).unwrap();
        let decoded: DidKey = crate::canonical::from_canonical_cbor(&cbor).unwrap();
        assert_eq!(did, decoded);

        // A plain String with the same content must also decode as a DidKey.
        let s = did.as_str().to_string();
        let cbor_s = crate::canonical::to_canonical_cbor(&s).unwrap();
        let decoded2: DidKey = crate::canonical::from_canonical_cbor(&cbor_s).unwrap();
        assert_eq!(did, decoded2);
    }
}
