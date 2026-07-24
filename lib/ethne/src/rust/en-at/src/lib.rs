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

//! # ethne-at — the autonomous_trust substrate provider
//!
//! `en-at` adapts a running (or replayed) autonomous_trust view into an ethne
//! [`en_substrate::SubstrateProvider`], so a cooperating AT cluster can be
//! offered to its members for ratification into a bounded, substrate-anchored
//! polity ("ratify emergence, do not replace it").
//!
//! Locked design decisions this crate realizes:
//! - **D5** — it is a *Rust crate over AT's C interface*, not PyO3: ethne stays
//!   pure Rust. All C contact is isolated behind the [`AtEventSource`] seam.
//! - **D6** — both coupling modes (embed an AT node / attach to a running
//!   daemon's out-queue) are [`AtEventSource`] implementations, in the
//!   `at-ffi`-gated [`source::ffi`] module.
//! - **D7** — the candidate roll is the observed *gateway-reputation subtree*
//!   (AT's subtree member-roster capability); a private gateway is an opaque
//!   boundary edge, not incompleteness.
//! - **D8** — a member's guardian edge is its verified **operator** (the
//!   operator-attended signal), the human-above-machine anchor.
//!
//! Decoupling: the default build depends only on `en-*` (→ `kc-core`); it does
//! NOT link autonomous_trust. So `KC ⟂ AT` and `KC ⟂ ethne` are preserved even
//! with this adapter present — the real AT linkage is opt-in via `at-ffi`.

pub mod event;
pub mod provider;
pub mod source;

pub use event::{signing_key_did, AtEvent, AtPeer};
pub use provider::AtSubstrateProvider;
pub use source::{AtEventSource, MockEventSource};

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::Substrate;
    use en_core::{keys, Alg, Timestamp};
    use en_substrate::SubstrateProvider;

    /// A distinct ed25519 signing key per seed (deterministic across a run is
    /// not required — we only need distinct valid keys).
    fn key() -> Vec<u8> {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        pk.bytes
    }

    fn machine(rep: f64) -> (Vec<u8>, AtPeer) {
        let k = key();
        (
            k.clone(),
            AtPeer {
                signing_pubkey: k,
                reputation: rep,
                rank: 0,
                operator_bound: false,
                guardian_pubkey: None,
            },
        )
    }

    fn operator_attended(rep: f64) -> (Vec<u8>, Vec<u8>, AtPeer) {
        let node = key();
        let guardian = key();
        (
            node.clone(),
            guardian.clone(),
            AtPeer {
                signing_pubkey: node,
                reputation: rep,
                rank: 5,
                operator_bound: true,
                guardian_pubkey: Some(guardian),
            },
        )
    }

    #[test]
    fn fold_builds_founding_candidate_with_guardian_edges() {
        let (_mk, m) = machine(0.9);
        let (nk, gk, op) = operator_attended(0.8);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::PeerObserved(m), AtEvent::PeerObserved(op)]);

        let mut provider = AtSubstrateProvider::new(src);
        assert_eq!(provider.refresh().unwrap(), 2);
        assert_eq!(provider.observed_len(), 2);

        let fc = provider.founding_candidate().unwrap();
        assert_eq!(fc.members.len(), 2);
        assert!(fc.substrate.is_none()); // pure emergent (Q4)

        // the operator-attended node carries a guardian edge = the operator DID;
        // the plain machine carries none (D8).
        let node_did = signing_key_did(&nk).unwrap();
        let guardian_did = signing_key_did(&gk).unwrap();
        let op_member = fc.members.iter().find(|m| m.did == node_did).unwrap();
        assert_eq!(op_member.guardian.as_ref(), Some(&guardian_did));
        assert!(fc
            .members
            .iter()
            .any(|m| m.did != node_did && m.guardian.is_none()));
    }

    #[test]
    fn departed_peer_leaves_the_roll() {
        let (mk, m) = machine(0.7);
        let (_ok, other) = machine(0.8);
        // Observe two, then depart the first — folded in arrival order within a
        // single refresh, so the departed peer is gone and the other remains.
        let mut src = MockEventSource::new();
        src.extend([
            AtEvent::PeerObserved(m),
            AtEvent::PeerObserved(other),
            AtEvent::PeerDeparted {
                signing_pubkey: mk.clone(),
            },
        ]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        assert_eq!(provider.observed_len(), 1);
        assert!(provider
            .member_reputation(&signing_key_did(&mk).unwrap())
            .is_none());
    }

    #[test]
    fn reputation_filter_trims_the_proposal() {
        // Two above the floor, one below — propose() must drop the low one.
        let (_a, a) = machine(0.9);
        let (_b, b) = machine(0.6);
        let (_c, c) = machine(0.2);
        let mut src = MockEventSource::new();
        src.extend([
            AtEvent::PeerObserved(a),
            AtEvent::PeerObserved(b),
            AtEvent::PeerObserved(c),
        ]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();

        let fc = provider.founding_candidate().unwrap();
        assert_eq!(fc.members.len(), 3);

        // Build a proposal with a reputation floor of 0.5 -> drops the 0.2 member.
        let (_psk, ppk) = keys::generate(Alg::Ed25519).unwrap();
        let polity = signing_key_did(&ppk.bytes).unwrap();
        let substrate = Substrate {
            name: "Test Polity".into(),
            statement: "a cohering AT cluster".into(),
            anchor: None,
        };
        let proposal = fc.propose(
            polity,
            substrate,
            vec![],
            0.5,
            Timestamp::from_second(0).unwrap(),
        );
        assert_eq!(proposal.proposed_members.len(), 2);
    }

    #[test]
    fn private_boundary_is_recorded_not_a_failure() {
        let (_mk, m) = machine(0.9);
        let mut src = MockEventSource::new();
        src.extend([AtEvent::BoundaryPrivate, AtEvent::PeerObserved(m)]);
        let mut provider = AtSubstrateProvider::new(src);
        provider.refresh().unwrap();
        assert!(provider.has_private_boundary());
        // the visible member is still in the roll — privacy != incompleteness
        assert_eq!(provider.observed_len(), 1);
    }
}
