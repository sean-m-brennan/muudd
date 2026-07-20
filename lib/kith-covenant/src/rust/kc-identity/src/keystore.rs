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

//! In-memory keystore for the demo. Maps DID string → private key.
//!
//! Not persisted, not encrypted, not secret-preserving beyond `PrivKey`'s
//! redacted Debug impl. For demo use only — a real deployment uses HSMs or
//! OS-level secret stores.

use std::collections::BTreeMap;

use kc_core::{DidKey, PrivKey};

#[derive(Default, Debug)]
pub struct Keystore {
    entries: BTreeMap<String, PrivKey>,
}

impl Keystore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, did: &DidKey, priv_key: PrivKey) {
        self.entries.insert(did.as_str().to_string(), priv_key);
    }

    pub fn get(&self, did: &DidKey) -> Option<&PrivKey> {
        self.entries.get(did.as_str())
    }

    pub fn get_by_str(&self, did: &str) -> Option<&PrivKey> {
        self.entries.get(did)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{keys, Alg};

    #[test]
    fn insert_and_get() {
        let mut ks = Keystore::new();
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        ks.insert(&did, sk.clone());
        assert_eq!(ks.len(), 1);
        assert_eq!(ks.get(&did), Some(&sk));
        assert_eq!(ks.get_by_str(did.as_str()), Some(&sk));
    }

    #[test]
    fn missing_lookup_returns_none() {
        let ks = Keystore::new();
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        assert!(ks.get(&did).is_none());
    }

    #[test]
    fn insert_overwrites() {
        let mut ks = Keystore::new();
        let (sk1, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = DidKey::from_pubkey(&pk).unwrap();
        ks.insert(&did, sk1);

        let (sk2, _) = keys::generate(Alg::Ed25519).unwrap();
        ks.insert(&did, sk2.clone());

        assert_eq!(ks.len(), 1);
        assert_eq!(ks.get(&did), Some(&sk2));
    }
}
