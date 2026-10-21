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

//! A community's **member roll**: the individual key-holders it names as its own.
//!
//! kith-covenant models communities and the speakers they delegate. That is enough
//! to run ceremony and enough to bind a community to its acts, but it is not enough
//! for the §8 whistleblower channel, whose ring must be wide enough to hide a source
//! in — and an insider holding no delegation was not in it at all.
//!
//! The roll is the polity tier's read-model crossing the boundary: `ethne` charters
//! membership and `en-uplift` publishes the roll under the polity's root key. Nothing
//! here depends on that tier; a roll is an ordinary community-signed record, and a
//! deployment with no polity tier can publish one by hand.
//!
//! Two properties are load-bearing and enforced at the archive layer rather than here:
//!
//! - **A roll only ever widens the insider set.** Rolls are unioned, never superseded,
//!   so a community cannot shrink the suspect set around a source by publishing a
//!   narrower roll. Once named an insider, always an insider. The cost is that a
//!   departed member stays in the ring — which is the direction to err in, since the
//!   alternative hands the accused a lever for exposing whoever just left.
//! - **A member carries its own key.** `member_pubkey` must be what `member_did`
//!   resolves to, so a roll cannot name a real member against a foreign key and put an
//!   outsider in the ring under a member's name.

use kc_core::{DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// One member of a community: an identity and the key that speaks for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollMember {
    pub member_did: DidKey,
    pub member_pubkey: PubKey,
}

/// A community's statement of who its members are, signed by its root key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberRoll {
    pub community_did: DidKey,
    pub members: Vec<RollMember>,
    /// The instant the roll was read from whatever models membership upstream (for a
    /// polity, its charter). Provenance, not authority: this tier cannot check it.
    pub as_of: Timestamp,
    pub published_at: Timestamp,
}

impl MemberRoll {
    /// Does this roll name `did`?
    pub fn names(&self, did: &DidKey) -> bool {
        self.members.iter().any(|m| &m.member_did == did)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{keys, Alg};

    fn member() -> RollMember {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        RollMember { member_did: DidKey::from_pubkey(&pk).unwrap(), member_pubkey: pk }
    }

    fn roll(members: Vec<RollMember>) -> MemberRoll {
        MemberRoll {
            community_did: member().member_did,
            members,
            as_of: Timestamp::from_second(10).unwrap(),
            published_at: Timestamp::from_second(20).unwrap(),
        }
    }

    #[test]
    fn a_roll_names_its_members_and_no_one_else() {
        let m = member();
        let other = member();
        let r = roll(vec![m.clone()]);
        assert!(r.names(&m.member_did));
        assert!(!r.names(&other.member_did));
    }

    #[test]
    fn a_members_carried_key_is_the_one_its_did_resolves_to() {
        let m = member();
        assert_eq!(m.member_did.to_pubkey().unwrap().bytes, m.member_pubkey.bytes);
    }
}
