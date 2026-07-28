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

use en_core::{Alg, DidKey, KcResult, PubKey, Timestamp};

/// A decoded AT peer as observed within the gateway-reputation subtree (the
/// boundary decision **D7** draws for the polity). Identity is the peer's
/// ed25519 *signing* public key — the same key that becomes its `did:key` on the
/// ethne/kith-covenant side — so no external registry is needed.
#[derive(Clone, Debug, PartialEq)]
pub struct AtPeer {
    /// 32-byte ed25519 signing public key (AT `public_identity_t.signature`).
    pub signing_pubkey: Vec<u8>,
    /// Earned reputation on AT's own absolute `[0.0, 1.0]` scale, or `None` when AT
    /// has nothing to rate this peer on (design **D18**).
    ///
    /// AT's scale needs no rescaling: it is anchored by fixed constants
    /// (`PREREP_NEUTRAL` 0.2, `COMM_CUTOFF` 0.1, tier floors at .50/.65/.80/.90) and
    /// is not normalized within the peer population, so the number crosses the seam
    /// as-is.
    ///
    /// **`None` is not yet reachable from a live node, and that is an AT-side gap
    /// rather than a decoder one.** AT returns `PREREP_NEUTRAL` both for a peer it
    /// has no history for and as a score a peer can genuinely earn, so it collapses
    /// "unrated" into a number at its own API boundary; "exactly 0.2" is not a safe
    /// inference. Exposing the distinction (a "has bilateral history" signal
    /// alongside the score) is a named AT-side prerequisite, sequenced the way Q9
    /// sequenced the operator signal: build the AT half first. Until then the
    /// at-ffi decoder reports `Some(score)` for every peer.
    pub reputation: Option<f64>,
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
    /// When a human guardian was last verified *present* at this node's console —
    /// AT's live **attended-now** signal, the counterpart to the durable
    /// `operator_bound` (AT `public_identity_t.operator_attested_at`). `None` for
    /// "not attended", which is what AT reports both when nobody is there and
    /// when it could not confirm; the decoder maps AT's `0.0` stamp to `None`.
    ///
    /// The two signals answer different questions and must not be read for one
    /// another: a node can be `operator_bound` with nobody at the keyboard for a
    /// week, and that is a normal state, not an error. This stamp is only
    /// meaningful against a clock — it says nothing on its own about whether a
    /// human is there *now*, which is why freshness is graded at query time by
    /// the cohesion monitor rather than reduced to a bool here.
    pub operator_attested_at: Option<Timestamp>,
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

    /// When this node's guardian was last verified present, or `None` for not
    /// attended. Gated on `operator_bound` for the same reason
    /// [`guardian_did`](Self::guardian_did) is: an attendance stamp on a node
    /// whose operator credential never verified attests to nothing.
    pub fn attended_at(&self) -> Option<Timestamp> {
        if self.operator_bound {
            self.operator_attested_at
        } else {
            None
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
