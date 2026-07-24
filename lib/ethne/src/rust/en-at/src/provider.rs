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

//! `ethne-at` — the autonomous_trust [`SubstrateProvider`] (decision **D5**).
//!
//! It folds the [`AtEventSource`] stream into a live view of the observed
//! gateway-reputation subtree (the boundary of decision **D7**) and presents
//! that view as a substrate ethne can charter a polity from. Each member's
//! guardian edge is its verified operator (the operator-attended signal of
//! decision **D8**).
//!
//! This is the "ratify emergence, do not replace it" seam on the AT side: the
//! provider only *reports* what AT observes — a candidate roll with soft edges —
//! and never itself confers standing. Nothing here depends on kith-covenant's
//! governance crates, so KC ⟂ AT and KC ⟂ ethne hold (the real AT C linkage is
//! opt-in behind the `at-ffi` feature).

use std::collections::HashMap;

use en_charter::Substrate;
use en_core::{DidKey, KcResult};
use en_substrate::{FoundingCandidate, MemberCandidate, SubstrateProvider};

use crate::event::{signing_key_did, AtEvent};
use crate::source::AtEventSource;

/// One member's folded reading.
#[derive(Clone, Debug, PartialEq)]
struct MemberView {
    reputation: f64,
    guardian: Option<DidKey>,
}

/// The autonomous_trust substrate provider. Call [`refresh`](Self::refresh) to
/// fold newly-arrived events into the view, then read it through the
/// [`SubstrateProvider`] methods. Generic over the [`AtEventSource`] so the same
/// logic runs against the mock (tests / offline founding) and the `at-ffi`
/// coupling modes.
pub struct AtSubstrateProvider<S: AtEventSource> {
    source: S,
    /// Keyed by member `did:key`. The roll is sorted by DID when emitted, so the
    /// candidate order is deterministic regardless of event arrival order.
    view: HashMap<DidKey, MemberView>,
    /// A substrate the deployment articulates for this AT cluster, if any. A
    /// purely emergent group has `None` (design open question Q4: it must anchor
    /// to a substrate before it can actually charter).
    substrate: Option<Substrate>,
    /// Whether any observed boundary edge is an intentional private/opaque one
    /// (informational; privacy is not incompleteness).
    private_boundary: bool,
}

impl<S: AtEventSource> AtSubstrateProvider<S> {
    /// A provider over `source` with no articulated substrate (pure emergent).
    pub fn new(source: S) -> Self {
        Self {
            source,
            view: HashMap::new(),
            substrate: None,
            private_boundary: false,
        }
    }

    /// A provider that also carries an articulated substrate to offer the polity.
    pub fn with_substrate(source: S, substrate: Substrate) -> Self {
        Self {
            source,
            view: HashMap::new(),
            substrate: Some(substrate),
            private_boundary: false,
        }
    }

    /// Poll the event source and fold everything it returns into the view. Must
    /// be called before reading the [`SubstrateProvider`] surface to see the
    /// latest observations. Returns the number of events applied.
    pub fn refresh(&mut self) -> KcResult<usize> {
        let events = self.source.poll()?;
        let n = events.len();
        for event in events {
            self.apply(event)?;
        }
        Ok(n)
    }

    fn apply(&mut self, event: AtEvent) -> KcResult<()> {
        match event {
            AtEvent::PeerObserved(peer) => {
                let did = peer.did()?;
                let guardian = peer.guardian_did()?;
                self.view.insert(
                    did,
                    MemberView {
                        reputation: peer.reputation,
                        guardian,
                    },
                );
            }
            AtEvent::PeerDeparted { signing_pubkey } => {
                let did = signing_key_did(&signing_pubkey)?;
                self.view.remove(&did);
            }
            AtEvent::BoundaryPrivate => {
                self.private_boundary = true;
            }
        }
        Ok(())
    }

    /// Whether an intentional private/opaque boundary edge has been observed
    /// (an AT gateway opted out of disclosure). The members behind it are simply
    /// absent from the roll — not counted as unreachable/incomplete.
    pub fn has_private_boundary(&self) -> bool {
        self.private_boundary
    }

    /// The number of members currently in the observed subtree view.
    pub fn observed_len(&self) -> usize {
        self.view.len()
    }
}

impl<S: AtEventSource> SubstrateProvider for AtSubstrateProvider<S> {
    fn founding_candidate(&self) -> KcResult<FoundingCandidate> {
        let mut members: Vec<MemberCandidate> = self
            .view
            .iter()
            .map(|(did, mv)| MemberCandidate {
                did: did.clone(),
                reputation: mv.reputation,
                guardian: mv.guardian.clone(),
            })
            .collect();
        // Deterministic order (HashMap iteration is unordered): sort by DID.
        members.sort_by(|a, b| a.did.as_str().cmp(b.did.as_str()));
        Ok(FoundingCandidate {
            members,
            substrate: self.substrate.clone(),
        })
    }

    fn member_reputation(&self, member: &DidKey) -> Option<f64> {
        self.view.get(member).map(|mv| mv.reputation)
    }
}
