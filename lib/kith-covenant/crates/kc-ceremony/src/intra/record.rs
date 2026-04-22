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
