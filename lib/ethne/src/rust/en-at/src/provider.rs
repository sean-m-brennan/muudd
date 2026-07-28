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

use en_charter::{BoundarySource, Substrate};
use en_core::{DidKey, KcResult, Timestamp};
use en_substrate::{
    ChangeStream, FoundingCandidate, MemberCandidate, SubstrateChange, SubstrateProvider,
};

use crate::event::{signing_key_did, AtEvent};
use crate::source::AtEventSource;

/// One member's folded reading.
#[derive(Clone, Debug, PartialEq)]
struct MemberView {
    reputation: Option<f64>,
    guardian: Option<DidKey>,
    /// When this member's guardian was last verified present (AT's attended-now
    /// signal), or `None` for not attended. Folded through unreduced — whether a
    /// stamp is still *fresh* is graded downstream against a cadence, not here.
    attested_at: Option<Timestamp>,
}

/// The autonomous_trust substrate provider. Call [`refresh`](Self::refresh) to
/// fold newly-arrived events into the view, then read it through the
/// [`SubstrateProvider`] methods. Generic over the [`AtEventSource`] so the same
/// logic runs against the mock (tests / offline founding) and the `at-ffi`
/// coupling modes.
pub struct AtSubstrateProvider<S: AtEventSource> {
    source: S,
    /// Changes observed while folding events, drained by
    /// [`ChangeStream::changes`]. Accumulated here rather than diffed by the caller
    /// because `view` is the only place the prior reading is already in hand.
    pending: Vec<SubstrateChange>,
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
            pending: Vec::new(),
            view: HashMap::new(),
            substrate: None,
            private_boundary: false,
        }
    }

    /// A provider that also carries an articulated substrate to offer the polity.
    pub fn with_substrate(source: S, substrate: Substrate) -> Self {
        Self {
            source,
            pending: Vec::new(),
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
                let attested_at = peer.attended_at();
                let next = MemberView {
                    reputation: peer.reputation,
                    guardian,
                    attested_at,
                };
                // Every delta is taken against the prior view, which is what makes
                // a flap visible: two equal snapshots either side of one would not
                // be.
                match self.view.get(&did) {
                    None => self.pending.push(SubstrateChange::MemberAppeared(MemberCandidate {
                        did: did.clone(),
                        reputation: next.reputation,
                        derived: true,
                        guardian: next.guardian.clone(),
                        attested_at: next.attested_at,
                    })),
                    Some(prior) => {
                        if prior.reputation != next.reputation {
                            self.pending.push(SubstrateChange::ReputationChanged {
                                member: did.clone(),
                                from: prior.reputation,
                                to: next.reputation,
                            });
                        }
                        if prior.guardian != next.guardian {
                            self.pending.push(SubstrateChange::GuardianEdgeChanged {
                                member: did.clone(),
                                guardian: next.guardian.clone(),
                            });
                        }
                    }
                }
                // A re-attestation is reported whenever the stamp moves forward,
                // including for a member seen for the first time: the stamp is the
                // datum a duration measure needs, and it is never reduced to a bool
                // here.
                if let Some(stamp) = next.attested_at {
                    let advanced = self
                        .view
                        .get(&did)
                        .and_then(|p| p.attested_at)
                        .map(|prior| stamp > prior)
                        .unwrap_or(true);
                    if advanced {
                        self.pending.push(SubstrateChange::AttendanceStamped {
                            member: did.clone(),
                            attested_at: stamp,
                        });
                    }
                }
                self.view.insert(did, next);
            }
            AtEvent::PeerDeparted { signing_pubkey } => {
                let did = signing_key_did(&signing_pubkey)?;
                if self.view.remove(&did).is_some() {
                    self.pending.push(SubstrateChange::MemberVanished(did));
                }
            }
            AtEvent::BoundaryPrivate => {
                if !self.private_boundary {
                    self.pending
                        .push(SubstrateChange::BoundaryOpacityChanged { opaque: true });
                }
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

    /// The event source, mutably — for feeding a `MockEventSource` (tests, replay
    /// of a captured AT view) or driving an attached one. The provider owns its
    /// source, so this is the only way to reach it after construction.
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
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
                // Every AT peer IS a node, so derivation here is structural rather
                // than a per-peer reading: there is no such thing as a human peer in
                // AT's model, only a node with or without a verified human behind it.
                // What varies is the guardian edge (`operator_bound`), and a node
                // without one is exactly the unguarded machine D15's office rule
                // exists for — which the founding could not express until
                // `MemberCandidate::derived` did (D18). Before that, a node with no
                // operator silently proposed as a human member.
                derived: true,
                guardian: mv.guardian.clone(),
                attested_at: mv.attested_at,
            })
            .collect();
        // Deterministic order (HashMap iteration is unordered): sort by DID.
        members.sort_by(|a, b| a.did.as_str().cmp(b.did.as_str()));
        Ok(FoundingCandidate {
            members,
            substrate: self.substrate.clone(),
            // An independent reading: this is the AT subtree as the trust service
            // actually observed it, not a roll anyone typed out (design **D21**).
            source: BoundarySource::Substrate,
        })
    }

    fn member_reputation(&self, member: &DidKey) -> Option<f64> {
        // Flattened deliberately: this accessor's `None` means "no reading, for any
        // reason" — peer unknown, or known and unrated. See the trait's doc.
        self.view.get(member).and_then(|mv| mv.reputation)
    }
}

impl<S: AtEventSource> ChangeStream for AtSubstrateProvider<S> {
    /// Fold whatever AT has queued, then drain the deltas that fold produced. The
    /// two steps are one call on purpose: a caller that polled without folding, or
    /// folded without draining, would silently lose movements — and a lost movement
    /// is indistinguishable from a quiet substrate.
    fn changes(&mut self) -> KcResult<Vec<SubstrateChange>> {
        self.refresh()?;
        Ok(std::mem::take(&mut self.pending))
    }
}
