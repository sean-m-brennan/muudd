//! Speaker delegation: a community authorizes a specific key-holder to act
//! in a named scope. Signed by the community's root key. Verification is
//! done at the archive layer (Phase 5) where the community's root pubkey
//! can be resolved by community DID.

use kc_core::{DidKey, PubKey, Timestamp};
use serde::{Deserialize, Serialize};

/// What this delegation authorizes its holder to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DelegationScope {
    /// Recite the community's rite (speak the movements).
    Reciter,
    /// Represent the community in inter-community ceremony.
    Envoy,
    /// Act as an internal witness to intra-community rites.
    InternalWitness,
    /// Act as a cross-boundary witness across community lines.
    CrossBoundaryWitness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakerDelegation {
    pub community_did: DidKey,
    pub speaker_did: DidKey,
    pub speaker_pubkey: PubKey,
    pub scope: DelegationScope,
    pub issued_at: Timestamp,
    pub expires_at: Option<Timestamp>,
    pub revoked_at: Option<Timestamp>,
}

impl SpeakerDelegation {
    /// Is the delegation live at instant `t`? (issued, not expired, not revoked)
    pub fn active_at(&self, t: Timestamp) -> bool {
        if t < self.issued_at {
            return false;
        }
        if let Some(exp) = self.expires_at {
            if t > exp {
                return false;
            }
        }
        if let Some(rev) = self.revoked_at {
            if t >= rev {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::ToSpan;
    use kc_core::{keys, time, Alg};

    fn sample_delegation(
        scope: DelegationScope,
        issued: Timestamp,
        expires: Option<Timestamp>,
        revoked: Option<Timestamp>,
    ) -> SpeakerDelegation {
        let (_, community_pk) = keys::generate(Alg::Ed25519).unwrap();
        let (_, speaker_pk) = keys::generate(Alg::Ed25519).unwrap();
        let community_did = DidKey::from_pubkey(&community_pk).unwrap();
        let speaker_did = DidKey::from_pubkey(&speaker_pk).unwrap();
        SpeakerDelegation {
            community_did,
            speaker_did,
            speaker_pubkey: speaker_pk,
            scope,
            issued_at: issued,
            expires_at: expires,
            revoked_at: revoked,
        }
    }

    #[test]
    fn active_in_window() {
        let t = time::now();
        let d = sample_delegation(DelegationScope::Reciter, t, None, None);
        assert!(d.active_at(t));
        assert!(d.active_at(t.checked_add(1.hour()).unwrap()));
    }

    #[test]
    fn inactive_before_issue() {
        let t = time::now();
        let d = sample_delegation(DelegationScope::Reciter, t, None, None);
        assert!(!d.active_at(t.checked_sub(1.second()).unwrap()));
    }

    #[test]
    fn inactive_after_expiry() {
        let t = time::now();
        let exp = t.checked_add(1.hour()).unwrap();
        let d = sample_delegation(DelegationScope::Reciter, t, Some(exp), None);
        assert!(d.active_at(exp));
        assert!(!d.active_at(exp.checked_add(1.second()).unwrap()));
    }

    #[test]
    fn revocation_takes_effect_immediately() {
        let t = time::now();
        let rev = t.checked_add(30.minutes()).unwrap();
        let d = sample_delegation(DelegationScope::Envoy, t, None, Some(rev));
        assert!(d.active_at(rev.checked_sub(1.second()).unwrap()));
        assert!(!d.active_at(rev));
    }

    #[test]
    fn scopes_are_distinct() {
        let t = time::now();
        let r = sample_delegation(DelegationScope::Reciter, t, None, None);
        let e = SpeakerDelegation {
            scope: DelegationScope::Envoy,
            ..r.clone()
        };
        assert_ne!(r.scope, e.scope);
    }
}
