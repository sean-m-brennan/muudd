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

//! Intra-community rite runner.
//!
//! Enforces the intra-layer invariants before sealing:
//! 1. Speaker is authorized — holds an active `Reciter` delegation.
//! 2. Recitation signature verifies against the speaker's delegated key.
//! 3. All five movements are recited. A rite is the whole recitation or it is
//!    not the rite, and Movement 4 is what a polity is measured against later.
//! 4. At least one internal witness.
//! 5. Every witness is in the community's active witness registry with
//!    `IntraCommunity` role at their attestation time.
//! 6. Every attestation references the actual recitation CID.
//! 7. Every witness attestation signature verifies.
//! 8. The sealed record is signed by the community's root key.
//!
//! `version_of_rite` is **derived** from the recitation being sealed rather
//! than supplied, so the record cannot disagree with the recitation it holds.
//!
//! Two-layer rule: this module must not be used to build inter-community
//! records. The `inter::` runner consumes only completed, sealed
//! `Signed<IntraCommunityRiteRecord>` values.

use kc_core::{Cid, DidKey, KcError, KcResult, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessRegistry, WitnessRole,
};

use crate::recitation::Recitation;

use super::record::{InternalWitnessAttestation, IntraCommunityRiteRecord};

pub struct IntraCommunityRiteRunner<'a> {
    pub community: &'a Signed<CommunityIdentity>,
    pub delegations: &'a [Signed<SpeakerDelegation>],
    pub witness_registry: &'a Signed<WitnessRegistry>,
}

impl<'a> IntraCommunityRiteRunner<'a> {
    pub fn new(
        community: &'a Signed<CommunityIdentity>,
        delegations: &'a [Signed<SpeakerDelegation>],
        witness_registry: &'a Signed<WitnessRegistry>,
    ) -> Self {
        Self { community, delegations, witness_registry }
    }

    /// Speaker produces a signed recitation. Ergonomic wrapper around
    /// `Signed::build`.
    pub fn recite(
        speaker_did: &DidKey,
        speaker_priv: &PrivKey,
        draft: Recitation,
    ) -> KcResult<Signed<Recitation>> {
        Signed::build(draft, &[(speaker_did.as_str(), speaker_priv)])
    }

    /// Internal witness produces a signed attestation referring to a
    /// specific recitation CID.
    pub fn attest(
        witness_did: &DidKey,
        witness_priv: &PrivKey,
        recitation_cid: Cid,
        attested_at: Timestamp,
    ) -> KcResult<Signed<InternalWitnessAttestation>> {
        let attestation = InternalWitnessAttestation {
            witness_did: witness_did.clone(),
            recitation_cid,
            attested_at,
        };
        Signed::build(attestation, &[(witness_did.as_str(), witness_priv)])
    }

    /// Validate inputs and seal into a `Signed<IntraCommunityRiteRecord>`
    /// signed by the community root.
    pub fn seal(
        &self,
        recitation: Signed<Recitation>,
        attestations: Vec<Signed<InternalWitnessAttestation>>,
        community_root_priv: &PrivKey,
        sealed_at: Timestamp,
    ) -> KcResult<Signed<IntraCommunityRiteRecord>> {
        let community_did = &self.community.payload.did;

        // 1+2. Speaker delegation + recitation signature.
        let speaker_did = recitation.payload.speaker_did.clone();
        let delegation = self
            .delegations
            .iter()
            .find(|d| {
                d.payload.speaker_did == speaker_did
                    && d.payload.scope == DelegationScope::Reciter
                    && d.payload.community_did == *community_did
                    && d.payload.active_at(recitation.payload.recited_at)
            })
            .ok_or_else(|| {
                KcError::Archive(format!(
                    "no active Reciter delegation for speaker {speaker_did} in community {community_did} at recitation time"
                ))
            })?;

        let speaker_pk = delegation.payload.speaker_pubkey.clone();
        let speaker_did_str = speaker_did.as_str().to_string();
        recitation.verify(move |did: &str| {
            if did == speaker_did_str {
                Some(speaker_pk.clone())
            } else {
                None
            }
        })?;

        // Recitation must declare the same community it's being sealed in.
        if recitation.payload.community_did != *community_did {
            return Err(KcError::Archive(format!(
                "recitation community_did {} does not match sealing community {}",
                recitation.payload.community_did, community_did
            )));
        }

        // 3. All five movements are recited.
        if !recitation.payload.movements_all_present() {
            return Err(KcError::Archive(
                "recitation is missing a movement: the rite is all five or it is not the rite"
                    .into(),
            ));
        }

        // 4. At least one internal witness.
        if attestations.is_empty() {
            return Err(KcError::Archive(
                "intra-community rite requires at least one internal witness".into(),
            ));
        }

        // 4b. At least one witness who is not the speaker.
        //
        // A registry may legitimately seat the speaker: in a small community the person
        // who recites this rite attends everyone else's. What it may not do is let that
        // stand as the witnessing of their own. Measured before this rule, on the ethne
        // border where it first appeared: a rite whose only internal witness was its own
        // speaker sealed and archived clean.
        if attestations
            .iter()
            .all(|a| a.payload.witness_did == recitation.payload.speaker_did)
        {
            return Err(KcError::Archive(
                "intra-community rite is witnessed only by its own speaker: an attestation \
                 to one's own recitation witnesses nothing"
                    .into(),
            ));
        }

        // 5+6+7. Per-witness checks.
        for a in &attestations {
            if a.payload.recitation_cid != recitation.payload_cid {
                return Err(KcError::Archive(format!(
                    "attestation recitation_cid {} does not match recitation CID {}",
                    a.payload.recitation_cid, recitation.payload_cid
                )));
            }
            let witness_did = &a.payload.witness_did;
            if !self.witness_registry.payload.active_at(
                witness_did,
                WitnessRole::IntraCommunity,
                a.payload.attested_at,
            ) {
                return Err(KcError::Archive(format!(
                    "witness {witness_did} is not active with IntraCommunity role at attestation time"
                )));
            }
            let witness_pk = self
                .witness_registry
                .payload
                .entries
                .iter()
                .find(|e| &e.witness_did == witness_did)
                .map(|e| e.witness_pubkey.clone())
                .ok_or_else(|| {
                    KcError::Archive(format!(
                        "witness {witness_did} not found in registry (internal consistency error)"
                    ))
                })?;
            let wdid_str = witness_did.as_str().to_string();
            a.verify(move |did: &str| {
                if did == wdid_str {
                    Some(witness_pk.clone())
                } else {
                    None
                }
            })?;
        }

        // 8. Seal. The version is the recitation's own, never a second opinion
        // about it.
        let record = IntraCommunityRiteRecord {
            version_of_rite: recitation.payload.version.clone(),
            recitation,
            internal_witnesses: attestations,
            sealed_at,
        };
        Signed::build(record, &[(community_did.as_str(), community_root_priv)])
    }
}
