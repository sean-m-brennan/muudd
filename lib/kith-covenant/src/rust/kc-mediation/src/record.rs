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

//! The §4 mediation payloads and their classifying enum.

use kc_conduct::LadderRung;
use kc_core::{Cid, DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// One nominated mediator: a key a community vouches as competent to mediate,
/// valid within a window. The nominating community is the enclosing
/// [`MediatorRegistry`], so the non-overlap rule (no mediator from a party
/// community) can always determine a mediator's home community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatorEntry {
    pub mediator_did: DidKey,
    pub mediator_pubkey: PubKey,
    /// The key §8 custody shares are sealed to, distinct from the signing key above.
    ///
    /// A mediator seated on a whistleblower reception panel holds a share of the
    /// source's identity, and sealing that share is a key agreement. Doing it against
    /// the signing key would put one key under two primitives; this is the separate key,
    /// and the authority pass refuses a registration where the two are equal. It is
    /// still an Ed25519 key — the Edwards-form agreement path is unchanged and no new
    /// algorithm enters the compact — but it never signs anything.
    pub encryption_pubkey: PubKey,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

impl MediatorEntry {
    /// Is this mediator nomination live at instant `t`?
    pub fn active_at(&self, t: Timestamp) -> bool {
        self.valid_from <= t && self.valid_until.is_none_or(|end| t < end)
    }
}

/// A community's roster of nominated mediators. Signed by the community root.
/// Registries for the same `nominating_community` supersede by latest
/// `issued_at`; the compact-wide pool is the union across all communities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediatorRegistry {
    pub nominating_community: DidKey,
    pub entries: Vec<MediatorEntry>,
    pub issued_at: Timestamp,
}

/// A dispute filed by one community against another, classified by the
/// escalation-ladder rung it is entering (kc-conduct::LadderRung; the full
/// ladder state machine is §5). Signed by the complainant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeRecord {
    pub complainant: DidKey,
    pub respondent: DidKey,
    pub rung: LadderRung,
    pub summary: String,
    /// Prior archive records this dispute draws on (conduct, prior disputes).
    pub references: Vec<Cid>,
    pub filed_at: Timestamp,
}

/// How a mediation resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DecisionOutcome {
    /// The complaint was sustained.
    Upheld,
    /// The complaint was rejected.
    Dismissed,
    /// The parties reached a mediated settlement.
    Settled,
    /// Referred onward (e.g. to the next rung or the designation track).
    Referred,
}

impl DecisionOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upheld => "Upheld",
            Self::Dismissed => "Dismissed",
            Self::Settled => "Settled",
            Self::Referred => "Referred",
        }
    }
}

/// A panel's decision on a dispute (referenced by CID). Co-signed by the panel
/// mediators; the archive authority layer enforces that the panel is >= 3
/// registered mediators, none from a party community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediationDecision {
    pub dispute_cid: Cid,
    /// The mediator DIDs on the deciding panel (each co-signs the envelope).
    pub panel: Vec<DidKey>,
    pub outcome: DecisionOutcome,
    pub reasoning: String,
    pub decided_at: Timestamp,
}
