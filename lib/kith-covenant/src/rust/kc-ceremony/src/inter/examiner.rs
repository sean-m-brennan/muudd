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

//! Examining pool: the roster of examiners a member community seats to conduct
//! anti-spiral-feature credentialing (capture_resistance.md §7 "Credentialing panels
//! by lot and rotation"; protocol-components §2).
//!
//! Deliberately the same shape as the §4 mediator pool
//! (`kc_mediation::MediatorRegistry`) and the §8 observer corps
//! (`kc_observer::ObserverRegistry`), because the doctrine says so: the credentialing
//! function is to be "rotated, witnessed, and monitored like the mediator pool it
//! resembles", and "a standing admissions cadre would be the single most capturable
//! body in the compact, for the same reasons a standing mediator class would be."
//!
//! Terms are bounded and rotation is enforced in `kc_archive::authority`
//! (`ExaminerCaps`), which shares one implementation of those rules with §8 rather
//! than restating them.
//!
//! *Not* enforced here: "drawn by lot". A verifiable lot needs a randomness beacon
//! the compact does not have; what the code enforces is bounded terms, rotation
//! sit-outs, and no-majority pool share. Selection fairness remains a social
//! requirement, audited through the composition-drift dashboard.

use kc_core::{DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// One examiner's seat in a nominating community's roster.
///
/// `valid_until` is `Option` for structural symmetry with the mediator and observer
/// entries, but §2's caps refuse an open-ended examining term — a permanent examiner
/// is the standing admissions cadre the rotation rule exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExaminerEntry {
    pub examiner_did: DidKey,
    pub examiner_pubkey: PubKey,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

impl ExaminerEntry {
    /// Is this examiner seated at instant `t`?
    pub fn active_at(&self, t: Timestamp) -> bool {
        self.valid_from <= t && self.valid_until.is_none_or(|end| t < end)
    }
}

/// A community's roster of seated examiners. Signed by the community root.
///
/// Registries for the same `nominating_community` supersede by latest `issued_at`;
/// the compact-wide pool is the union across all communities, exactly as for the
/// mediator and observer rosters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExaminerRegistry {
    pub nominating_community: DidKey,
    pub entries: Vec<ExaminerEntry>,
    pub issued_at: Timestamp,
}
