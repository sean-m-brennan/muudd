//! Typed archive entries. One variant per record kind that the protocol
//! produces. Every entry carries a `Signed<...>` envelope; the archive stores
//! the enum and indexes by `Cid`.

use kc_ceremony::inter::{InterCommunityWitnessRecord, NonAdmissionRecord};
use kc_ceremony::intra::IntraCommunityRiteRecord;
use kc_core::{Cid, KcResult, Signed};
use kc_identity::{CommunityIdentity, SpeakerDelegation, WitnessRegistry};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveEntry {
    CommunityIdentityPublished(Signed<CommunityIdentity>),
    SpeakerDelegated(Signed<SpeakerDelegation>),
    WitnessRegistryUpdated(Signed<WitnessRegistry>),
    IntraCommunityRite(Signed<IntraCommunityRiteRecord>),
    InterCommunityWitness(Signed<InterCommunityWitnessRecord>),
    InterCommunityNonAdmission(Signed<NonAdmissionRecord>),
}

impl ArchiveEntry {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CommunityIdentityPublished(_) => "CommunityIdentityPublished",
            Self::SpeakerDelegated(_) => "SpeakerDelegated",
            Self::WitnessRegistryUpdated(_) => "WitnessRegistryUpdated",
            Self::IntraCommunityRite(_) => "IntraCommunityRite",
            Self::InterCommunityWitness(_) => "InterCommunityWitness",
            Self::InterCommunityNonAdmission(_) => "InterCommunityNonAdmission",
        }
    }

    /// Hash the entry itself for archive-level addressing. (This is distinct
    /// from any nested `Signed<T>.payload_cid`, which addresses the payload.)
    pub fn cid(&self) -> KcResult<Cid> {
        Cid::of(self)
    }
}
