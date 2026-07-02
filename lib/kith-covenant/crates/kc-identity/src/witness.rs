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

//! Witness registry: the community's declaration of who may witness at its
//! intra-community rite and who may cross the boundary as a witness at
//! inter-community ceremony. Signed by the community root.

use kc_core::{DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WitnessRole {
    /// Member of this community witnessing its own rite.
    IntraCommunity,
    /// Authorized to attend and countersign across community boundaries.
    CrossBoundary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessEntry {
    pub witness_did: DidKey,
    pub witness_pubkey: PubKey,
    pub role: WitnessRole,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessRegistry {
    pub community_did: DidKey,
    pub entries: Vec<WitnessEntry>,
    pub issued_at: Timestamp,
}

impl WitnessRegistry {
    /// Is `did` an active witness of `role` at time `t` in this registry?
    pub fn active_at(&self, did: &DidKey, role: WitnessRole, t: Timestamp) -> bool {
        self.entries.iter().any(|e| {
            &e.witness_did == did
                && e.role == role
                && e.valid_from <= t
                && e.valid_until.map_or(true, |end| t < end)
        })
    }

    /// All active witnesses of `role` at time `t`.
    pub fn active_of(&self, role: WitnessRole, t: Timestamp) -> Vec<&WitnessEntry> {
        self.entries
            .iter()
            .filter(|e| {
                e.role == role
                    && e.valid_from <= t
                    && e.valid_until.map_or(true, |end| t < end)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::ToSpan;
    use kc_core::{keys, time, Alg};

    fn mk_entry(role: WitnessRole, valid_from: Timestamp, valid_until: Option<Timestamp>) -> WitnessEntry {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        WitnessEntry {
            witness_did: DidKey::from_pubkey(&pk).unwrap(),
            witness_pubkey: pk,
            role,
            valid_from,
            valid_until,
        }
    }

    #[test]
    fn active_only_when_role_matches() {
        let (_, community_pk) = keys::generate(Alg::Ed25519).unwrap();
        let community_did = DidKey::from_pubkey(&community_pk).unwrap();
        let t = time::now();

        let intra = mk_entry(WitnessRole::IntraCommunity, t, None);
        let cross = mk_entry(WitnessRole::CrossBoundary, t, None);
        let intra_did = intra.witness_did.clone();
        let cross_did = cross.witness_did.clone();

        let reg = WitnessRegistry {
            community_did,
            entries: vec![intra, cross],
            issued_at: t,
        };

        assert!(reg.active_at(&intra_did, WitnessRole::IntraCommunity, t));
        assert!(!reg.active_at(&intra_did, WitnessRole::CrossBoundary, t));
        assert!(reg.active_at(&cross_did, WitnessRole::CrossBoundary, t));
    }

    #[test]
    fn inactive_outside_validity_window() {
        let (_, community_pk) = keys::generate(Alg::Ed25519).unwrap();
        let community_did = DidKey::from_pubkey(&community_pk).unwrap();
        let t0 = time::now();
        let t_mid = t0.checked_add(1.hour()).unwrap();
        let t_end = t0.checked_add(2.hours()).unwrap();

        let entry = mk_entry(WitnessRole::IntraCommunity, t_mid, Some(t_end));
        let did = entry.witness_did.clone();

        let reg = WitnessRegistry {
            community_did,
            entries: vec![entry],
            issued_at: t0,
        };

        assert!(!reg.active_at(&did, WitnessRole::IntraCommunity, t0));
        assert!(reg.active_at(&did, WitnessRole::IntraCommunity, t_mid));
        assert!(!reg.active_at(&did, WitnessRole::IntraCommunity, t_end));
    }

    #[test]
    fn active_of_returns_matching_subset() {
        let (_, community_pk) = keys::generate(Alg::Ed25519).unwrap();
        let community_did = DidKey::from_pubkey(&community_pk).unwrap();
        let t = time::now();

        let entries = vec![
            mk_entry(WitnessRole::IntraCommunity, t, None),
            mk_entry(WitnessRole::IntraCommunity, t, None),
            mk_entry(WitnessRole::CrossBoundary, t, None),
        ];

        let reg = WitnessRegistry {
            community_did,
            entries,
            issued_at: t,
        };

        assert_eq!(reg.active_of(WitnessRole::IntraCommunity, t).len(), 2);
        assert_eq!(reg.active_of(WitnessRole::CrossBoundary, t).len(), 1);
    }
}
