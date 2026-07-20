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

//! Sponsorship record: an existing member community vouches for an entrant,
//! staking its own standing on the entrant's conduct through a probationary
//! window (capture_resistance.md §7 "Admissions integrity"; protocol-components
//! §2 "Sponsor-stake tracking").
//!
//! This is the graph edge (sponsor → entrant) that the composition-drift and
//! sponsor-stake detectors project over. Wrapped in `Signed<SponsorshipRecord>`
//! by the sponsoring community (root key or a delegated `Envoy`). Keys are
//! resolved `as_of` [`SponsorshipRecord::sponsored_at`].

use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SponsorshipRecord {
    /// The existing member doing the vouching, whose standing is now staked.
    pub sponsor_community: DidKey,
    /// The candidate being vouched for.
    pub entrant_community: DidKey,
    /// The inter-community admission this sponsorship supports, if already
    /// sealed. `None` when the vouch precedes the admission ceremony.
    pub admission_cid: Option<Cid>,
    /// The sponsor's stake rides on the entrant's conduct until this instant.
    /// Adverse conduct by the entrant before `probation_until` reflects on the
    /// sponsor (sponsor-stake detector).
    pub probation_until: Timestamp,
    /// What the sponsor attests on the entrant's behalf.
    pub statement: String,
    pub sponsored_at: Timestamp,
}

impl SponsorshipRecord {
    /// Is the probationary window still open at instant `t`?
    pub fn in_probation(&self, t: Timestamp) -> bool {
        t >= self.sponsored_at && t < self.probation_until
    }
}
