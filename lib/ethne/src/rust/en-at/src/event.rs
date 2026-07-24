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

//! Decoded autonomous_trust events — the currency of the [`crate::AtEventSource`]
//! seam (decision D5). The C linkage tees AT→app messages into these neutral
//! Rust values, so the adapter's fold logic is pure and testable without a live
//! daemon.

use en_core::{Alg, DidKey, KcResult, PubKey};

/// A decoded AT peer as observed within the gateway-reputation subtree (the
/// boundary decision **D7** draws for the polity). Identity is the peer's
/// ed25519 *signing* public key — the same key that becomes its `did:key` on the
/// ethne/kith-covenant side — so no external registry is needed.
#[derive(Clone, Debug, PartialEq)]
pub struct AtPeer {
    /// 32-byte ed25519 signing public key (AT `public_identity_t.signature`).
    pub signing_pubkey: Vec<u8>,
    /// Earned reputation, normalized to `[0.0, 1.0]` by the source (AT's
    /// Paxos-committed per-peer reputation, rescaled).
    pub reputation: f64,
    /// AT topology rank (one-hop reachability via gateways); informational for
    /// v1, carried so a later cohesion slice can read it.
    pub rank: i32,
    /// The verified operator-attended signal (decision **D8**): true iff AT
    /// classified this node as having a human guardian (operator credential
    /// verified against the distinct operator anchor). Never the peer's own
    /// unverified claim — AT neutralizes that at admission.
    pub operator_bound: bool,
    /// The guardian operator's ed25519 signing key, present iff `operator_bound`.
    /// Becomes the member's guardian `did:key` — the human-above-machine anchor
    /// the substrate's guardian ground requires.
    pub guardian_pubkey: Option<Vec<u8>>,
}

/// The `did:key` for an ed25519 signing public key — the one mapping between an
/// AT identity and its ethne/kith-covenant DID (did:key embeds the key, so no
/// registry is involved).
pub fn signing_key_did(bytes: &[u8]) -> KcResult<DidKey> {
    DidKey::from_pubkey(&PubKey {
        alg: Alg::Ed25519,
        bytes: bytes.to_vec(),
    })
}

fn ed25519_did(bytes: &[u8]) -> KcResult<DidKey> {
    signing_key_did(bytes)
}

impl AtPeer {
    /// This peer's `did:key` (derived from its signing key).
    pub fn did(&self) -> KcResult<DidKey> {
        ed25519_did(&self.signing_pubkey)
    }

    /// The guardian's `did:key`, when this node is operator-attended (D8).
    /// `Ok(None)` when there is no human behind the node; `Err` only if a
    /// present guardian key is malformed.
    pub fn guardian_did(&self) -> KcResult<Option<DidKey>> {
        match (&self.guardian_pubkey, self.operator_bound) {
            (Some(gk), true) => ed25519_did(gk).map(Some),
            // A guardian key without the verified bound is ignored — the signal
            // is only trustworthy when AT set operator_bound.
            _ => Ok(None),
        }
    }
}

/// A decoded event teed from the AT→app stream. The adapter folds a sequence of
/// these into its live view of the observed subtree.
#[derive(Clone, Debug, PartialEq)]
pub enum AtEvent {
    /// A peer is present in the observed subtree (upsert its reading).
    PeerObserved(AtPeer),
    /// A peer left the observed subtree, keyed by its signing key.
    PeerDeparted { signing_pubkey: Vec<u8> },
    /// The observed subtree boundary is (partly) an intentional private/opaque
    /// edge — a private gateway opted out of disclosure (AT_ROSTER_PRIVATE).
    /// Informational: privacy is not incompleteness, and the members behind an
    /// opaque edge are simply absent from the roll, not "unknown/failed".
    BoundaryPrivate,
}
