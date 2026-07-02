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

//! Anti-spiral feature credentialing types.
//!
//! Per doc/overview.md §3b.i + §5b: each participating community presents
//! claims that its substrate supplies the five anti-spiral features. The
//! protocol *records* claims and accepting communities' verdicts; it does
//! not adjudicate substrate truth (architecture decision 6: substrate-check
//! at recognition, not enforcement).

use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// The five anti-spiral features a community's substrate must supply for
/// inter-community admission. See whitepaper §3 and background/noticeable_hand.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AntiSpiralFeature {
    /// Universal fallen-actor anthropology: every community member (and
    /// the community itself) is capable of wrong.
    FallenActorAnthropology,
    /// Grace over merit: restoration is possible; the community is not a
    /// pure meritocracy that generates competitive-virtue spirals.
    GraceOverMerit,
    /// Scapegoat-disarming: the substrate exposes and resists the logic
    /// of purifying-sacrifice of a designated victim.
    ScapegoatDisarming,
    /// Enemy-love as an operational norm, not sentiment; enemies remain
    /// human, not disposable or demonized (while *hostis humani generis*
    /// handling remains bounded).
    EnemyLoveOperational,
    /// Non-ultimacy of the political community: the compact never claims
    /// to be the ultimate thing; something stands above it.
    NonUltimacyOfCompact,
}

/// Kind of evidence a community adduces for a feature claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// The community's own sealed intra-community rite.
    OwnRite,
    /// Documented restoration / reconciliation path (for 1 and 3).
    RestorationPath,
    /// Historical record of community conduct.
    HistoricalRecord,
    /// Reputation attestation (e.g., lifted from AT or external).
    ReputationRecord,
    /// Community's own declaration; no harder evidence adduced.
    Declarative,
}

/// Pointer to evidence. `identifier` may be a CID, URI, or community-internal
/// reference. `description` is free-form context for the witnesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub kind: EvidenceKind,
    pub identifier: String,
    pub description: Option<String>,
}

/// One claim: "our substrate supplies feature X." Signed by the presenting
/// community's envoy (or root key, depending on scope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntiSpiralFeatureClaim {
    pub community_did: DidKey,
    pub feature: AntiSpiralFeature,
    pub claim_text: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub claimed_at: Timestamp,
}

/// One verdict: an accepting community's response to a specific claim.
/// `claim_cid` references the Signed<AntiSpiralFeatureClaim>'s payload CID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureAcceptance {
    pub accepting_community_did: DidKey,
    pub claim_cid: Cid,
    pub accepted: bool,
    pub rationale: Option<String>,
    pub decided_at: Timestamp,
}

/// The five features that must each be presented by each participant.
pub const REQUIRED_FEATURES: [AntiSpiralFeature; 5] = [
    AntiSpiralFeature::FallenActorAnthropology,
    AntiSpiralFeature::GraceOverMerit,
    AntiSpiralFeature::ScapegoatDisarming,
    AntiSpiralFeature::EnemyLoveOperational,
    AntiSpiralFeature::NonUltimacyOfCompact,
];
