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

//! Intra-community rite record: the sealed artifact produced by an
//! intra-community ceremony. Combines the signed recitation, internal
//! witness attestations, and the community's seal.

use kc_core::{Cid, DidKey, Signed, Timestamp};
use serde::{Deserialize, Serialize};

use crate::recitation::{Recitation, RecitationVersion};

/// An internal witness's attestation that they observed a specific recitation
/// (referenced by CID). Signed by the witness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalWitnessAttestation {
    pub witness_did: DidKey,
    pub recitation_cid: Cid,
    pub attested_at: Timestamp,
}

/// The sealed output of an intra-community rite. Wrapped in
/// `Signed<IntraCommunityRiteRecord>` by the community's root key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntraCommunityRiteRecord {
    pub recitation: Signed<Recitation>,
    pub internal_witnesses: Vec<Signed<InternalWitnessAttestation>>,
    pub version_of_rite: RecitationVersion,
    pub sealed_at: Timestamp,
}
