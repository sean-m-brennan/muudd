//! Projection types returned by `lookup`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecognitionStanding {
    /// Community DID absent from archive at `as_of`.
    Unknown,
    /// `CommunityIdentityPublished` present; no inter ceremony yet.
    Identified,
    /// ≥1 sealed `InterCommunityWitness` including this community.
    InCompact,
    /// Inter record(s) exist but only non-admissions; no seal.
    Denied,
}

impl RecognitionStanding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Identified => "Identified",
            Self::InCompact => "InCompact",
            Self::Denied => "Denied",
        }
    }
}
