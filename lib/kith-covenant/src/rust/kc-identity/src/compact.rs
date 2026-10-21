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

//! A compact's own identity, for the level of nesting above the compact.
//!
//! Ostrom's eighth design principle asks that "appropriation, provision, monitoring,
//! enforcement, conflict resolution, and governance activities are organized in multiple
//! layers of nested enterprises", and `commons_federation.md` maps three layers onto this
//! project: the habitat (autonomous_trust), the compact (kith-covenant), and the
//! inter-compact level, which it leaves as "a forward pointer; the mechanisms would be
//! structurally analogous to kith-covenant itself, but at a higher nesting level".
//!
//! Nothing at that higher level can be said until a compact is a thing that can be
//! named. Before this record the only compact-level name in the tier was
//! `InterCommunityWitnessRecord.compact_label`, a self-declared string: two witnesses
//! could write the same label and mean different compacts, or different labels and mean
//! the same one, and no rule could tell.
//!
//! ## Constituted, not self-signed
//!
//! A [`CompactCharter`] is the structural analogue of `CommunityIdentity` one level up,
//! with one deliberate difference. A community's identity is **self-signed**: a community
//! exists by its own act and answers to nobody for existing. A compact does not exist by
//! its own act. It is constituted by its member communities, so the charter is co-signed
//! by a quorum of them *and* by the compact's own key, which proves the key is held by
//! somebody and binds the DID the compact will thereafter sign with.
//!
//! That asymmetry is the whole content of "a higher nesting level" as this tier realizes
//! it. The compact is answerable downward in a way its members are not, and the record
//! shape says so before any rule does.

use kc_core::{DidKey, KcResult, PrivKey, PubKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

/// A compact's identity, constituted by a quorum of its member communities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactCharter {
    /// The compact's name. Unlike `compact_label` this is not the identifier; the DID
    /// is, and two compacts may share a name without sharing anything else.
    pub label: String,
    /// Derived from `root_pubkey`, exactly as a community's DID is.
    pub did: DidKey,
    pub root_pubkey: PubKey,
    /// The member communities this compact is constituted by. An inter-compact commons
    /// reaches its members only through it, never past it.
    pub members: Vec<DidKey>,
    /// The members chartering it, each of whom co-signs alongside the compact's own key.
    /// Must be a non-empty subset of `members` meeting the second-class quorum.
    pub chartered_by: Vec<DidKey>,
    pub chartered_at: Timestamp,
}

impl CompactCharter {
    /// Build a charter and sign it with the compact's own key plus each chartering
    /// member's. `did` is derived from `root_pub`.
    ///
    /// The compact's own signature is not ceremony: without it the charter would bind a
    /// DID to a key nobody has demonstrated holding, and every later act signed by that
    /// key would rest on an assumption the archive never checked.
    pub fn new_signed(
        label: String,
        root_priv: &PrivKey,
        root_pub: PubKey,
        members: Vec<DidKey>,
        chartered_by: &[(DidKey, &PrivKey)],
        chartered_at: Timestamp,
    ) -> KcResult<Signed<Self>> {
        let did = DidKey::from_pubkey(&root_pub)?;
        let payload = CompactCharter {
            label,
            did: did.clone(),
            root_pubkey: root_pub,
            members,
            chartered_by: chartered_by.iter().map(|(d, _)| d.clone()).collect(),
            chartered_at,
        };
        let mut signers: Vec<(&str, &PrivKey)> = vec![(did.as_str(), root_priv)];
        for (d, k) in chartered_by {
            signers.push((d.as_str(), k));
        }
        Signed::build(payload, &signers)
    }
}
