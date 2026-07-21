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

//! The §8 observer / jester payloads.

use kc_core::{Cid, DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// One nominated observer: a key a community vouches as fit to travel, observe,
/// and report, valid within a window, carrying the **declared perspective** it
/// reports from (kept explicit so downstream readers discount partiality rather
/// than treat a report as neutral). The nominating community is the enclosing
/// [`ObserverRegistry`], so the non-party rule (an observer may not report on its
/// own community) can always determine an observer's home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverEntry {
    pub observer_did: DidKey,
    pub observer_pubkey: PubKey,
    pub declared_perspective: String,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

impl ObserverEntry {
    /// Is this observer nomination live at instant `t`?
    pub fn active_at(&self, t: Timestamp) -> bool {
        self.valid_from <= t && self.valid_until.map_or(true, |end| t < end)
    }
}

/// A community's roster of nominated observers. Signed by the community root.
/// Registries for the same `nominating_community` supersede by latest
/// `issued_at`; the compact-wide corps is the union across all communities
/// (mirrors the mediator/witness registries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverRegistry {
    pub nominating_community: DidKey,
    pub entries: Vec<ObserverEntry>,
    pub issued_at: Timestamp,
}

/// An attributed public report on a community's conduct, signed by an active
/// registered observer whose home community is **not** the subject
/// (cross-community journalism, non-overlap). Publication is irrevocable; the
/// subject answers with an appended [`ReportResponse`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverReport {
    /// The community observed.
    pub subject_community: DidKey,
    /// The reporting observer (resolves via the observer registry).
    pub observer_did: DidKey,
    /// The observer's declared standpoint, restated on the report for readers who
    /// see the report without the registry to hand.
    pub declared_perspective: String,
    pub summary: String,
    /// Prior archive records this report draws on (conduct, usage, prior reports).
    pub references: Vec<Cid>,
    pub observed_at: Timestamp,
    pub reported_at: Timestamp,
}

/// A subject community's public response to an [`ObserverReport`] (referenced by
/// CID), signed by and attributed to that community. The response is appended to
/// the record; it never suppresses the report (publication is irrevocable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportResponse {
    pub report_cid: Cid,
    pub responding_community: DidKey,
    pub response: String,
    pub responded_at: Timestamp,
}

/// An insider complaint that a community is violating the common virtues, filed
/// across the AT/kith-covenant boundary. Co-signed by a mediator **reception
/// panel** that vouches it received a credible insider report. The source
/// identity is **never stored**: `source_commitment` is a hash (a `Cid`) the
/// panel holds the preimage of out-of-band, so the archive carries only the
/// commitment. Cryptographic *anonymity* (ring signatures) and threshold
/// *encryption* of the source are deferred; v1 protects the source by not
/// recording it and gating any reveal behind a quorum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhistleblowerReport {
    pub accused_community: DidKey,
    /// A commitment to the source's identity (hash of identity + nonce). The
    /// archive never holds the preimage; a [`SourceReveal`] may later disclose it.
    pub source_commitment: Cid,
    pub claim: String,
    /// Prior archive records the claim cites, if any.
    pub references: Vec<Cid>,
    /// The mediator reception panel that received and vouched the report (each
    /// co-signs). The authority pass requires it be non-party active mediators.
    pub reception_panel: Vec<DidKey>,
    pub filed_at: Timestamp,
}

/// A disclosure of a whistleblower's source, gated on a mediator-panel quorum
/// citing named cause (jester_and_dissent.md "Procedural safeguards": the source
/// is protected unless a panel, for cause, reveals it). References the
/// [`WhistleblowerReport`] by CID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReveal {
    pub report_cid: Cid,
    /// The mediator panel authorizing the reveal (each co-signs).
    pub panel: Vec<DidKey>,
    /// The named cause for the reveal (must be non-empty).
    pub cause: String,
    /// The disclosed source identity (the commitment preimage), revealed only now.
    pub disclosed_source: String,
    pub revealed_at: Timestamp,
}
