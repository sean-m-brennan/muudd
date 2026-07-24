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

//! The pluggable substrate-provider interface: the source of the emergent
//! evidence a founding ratifies (and, later, the ongoing input a cohesion
//! monitor reads). `autonomous_trust` is *one* provider (`ethne-at`, a separate
//! adapter crate); the [`crate::manual::ManualFounding`] provider needs no
//! substrate at all. Ethne never depends on any particular provider — that is
//! what keeps kith-covenant free of an autonomous_trust dependency.

use en_charter::{Commitment, FoundingProposal, Substrate};
use en_core::{DidKey, KcResult, Timestamp};
use serde::{Deserialize, Serialize};

/// A candidate member the provider surfaces from the substrate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberCandidate {
    pub did: DidKey,
    /// An earned-reputation reading, normalized to `[0.0, 1.0]`. A manual
    /// convener asserts full reputation; an emergent provider computes it.
    pub reputation: f64,
    /// The guardian this member answers to, if it is a derived (machine) party.
    pub guardian: Option<DidKey>,
}

/// The **soft** emergent boundary a provider proposes for a founding: a candidate
/// roll and (optionally) a substrate it could infer. "Soft" because membership at
/// the edge is uncertain — the polity draws the boundary it will actually ratify.
/// Emergence proposes; the polity ratifies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundingCandidate {
    pub members: Vec<MemberCandidate>,
    /// A substrate the provider could infer. `None` for a purely emergent group
    /// with no articulated ground — which (design open question Q4) must anchor
    /// to a substrate before it can charter at all.
    pub substrate: Option<Substrate>,
}

impl FoundingCandidate {
    /// Draw a deliberate [`FoundingProposal`] from this soft candidate — the
    /// "ratify emergence, do not replace it" translation. The convener supplies
    /// the polity DID, the substrate the polity commits to (it may adopt
    /// `self.substrate` or override it), and the founding commitments (the
    /// provider never dictates what the polity stands for). The roll is trimmed
    /// to candidates whose reputation is at least `min_reputation`.
    ///
    /// The result is still only a *proposal*: it carries no authority until the
    /// members ratify and co-sign it (`FoundingProposal::ratify`).
    pub fn propose(
        &self,
        polity: DidKey,
        substrate: Substrate,
        founding_commitments: Vec<Commitment>,
        min_reputation: f64,
        proposed_at: Timestamp,
    ) -> FoundingProposal {
        let proposed_members = self
            .members
            .iter()
            .filter(|m| m.reputation >= min_reputation)
            .map(|m| m.did.clone())
            .collect();
        FoundingProposal {
            polity,
            substrate,
            founding_commitments,
            proposed_members,
            proposed_at,
        }
    }
}

/// A source of the emergent structure ethne crystallizes into a polity. Kept
/// deliberately narrow. The change/anomaly stream a cohesion monitor will consume
/// is a later slice; v1 covers the founding path and a point reputation reading.
pub trait SubstrateProvider {
    /// The current candidate founding boundary read from the substrate.
    fn founding_candidate(&self) -> KcResult<FoundingCandidate>;

    /// A point earned-reputation reading for one member, if known.
    fn member_reputation(&self, member: &DidKey) -> Option<f64>;
}
