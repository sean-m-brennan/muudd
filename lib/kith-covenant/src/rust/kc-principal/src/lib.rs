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

//! Adversarial-principal validation (capture-resistance move 6).
//!
//! Machines are derived-substrate parties whose binding flows from their named
//! human principal (the guardianship model). But the *principal* can themselves
//! be captured — by their community's drift, external pressure, personal
//! corruption, or manipulation through the machine's own outputs, including an
//! **AI-mediated influence operation** (capture_resistance.md move 6;
//! m2m_interaction.md §"Adversarial-principal validation"). A machine acting on
//! a captured principal's instructions is, by the existing protocol, acting
//! legitimately — so principal-validation must be a separate concern with its
//! own requirements.
//!
//! The strengthening move has two halves, and this crate carries a record for
//! each:
//! - [`PrincipalWitnessAttestation`] — principal validity must not rest on the
//!   principal's own attestation or their own community's. A **cross-community**
//!   witness (from neither the principal nor its home community) attests
//!   separately.
//! - [`PrincipalOutput`] — the machine's behavior is **triangulated** against
//!   similarly-situated machines under similarly-positioned principals. Conduct
//!   diverging from the ensemble in a direction that benefits the principal's
//!   own side is flagged.
//!
//! Both are *data only*. Which attestations count as cross-community and which
//! outputs diverge toward self-benefit is a **detector** signal (kc-reputation
//! `adversarial_principal`) — flags with provenance, no verdict.

use kc_core::{DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// A cross-community attestation of a principal's validity. Wrapped in
/// `Signed<PrincipalWitnessAttestation>` by the attesting (`witness_community`)
/// community. The detector counts it toward independent validation only when the
/// witness is neither the principal nor its home community.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalWitnessAttestation {
    pub principal_did: DidKey,
    /// The principal's own community.
    pub principal_home: DidKey,
    /// The community making the attestation (self-signed by it).
    pub witness_community: DidKey,
    pub basis: String,
    pub attested_at: Timestamp,
}

/// One machine output, recorded for triangulation. Wrapped in
/// `Signed<PrincipalOutput>` by the principal's home community. Similarly-
/// situated outputs share a `cohort` and a `matter`; a principal whose
/// `decision` diverges from its cohort's on a matter, in a direction that
/// benefits its own side (`favors_home`), is the capture signature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalOutput {
    pub principal_did: DidKey,
    pub principal_home: DidKey,
    /// The similarly-situated class of principals this output is compared within.
    pub cohort: String,
    /// The shared decision point peers in the cohort also answer.
    pub matter: String,
    /// The answer, compared across the cohort.
    pub decision: String,
    /// Declared: does this answer benefit the principal's own side?
    pub favors_home: bool,
    pub produced_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{canonical, keys, time, Alg};

    fn did() -> DidKey {
        let (_, pk) = keys::generate(Alg::Ed25519).unwrap();
        DidKey::from_pubkey(&pk).unwrap()
    }

    #[test]
    fn attestation_roundtrips_through_cbor() {
        let a = PrincipalWitnessAttestation {
            principal_did: did(),
            principal_home: did(),
            witness_community: did(),
            basis: "observed continuity of instruction".into(),
            attested_at: time::now(),
        };
        let bytes = canonical::to_canonical_cbor(&a).unwrap();
        let decoded: PrincipalWitnessAttestation =
            canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(a, decoded);
    }

    #[test]
    fn output_roundtrips_through_cbor() {
        let o = PrincipalOutput {
            principal_did: did(),
            principal_home: did(),
            cohort: "admissions-review".into(),
            matter: "M1".into(),
            decision: "admit".into(),
            favors_home: false,
            produced_at: time::now(),
        };
        let bytes = canonical::to_canonical_cbor(&o).unwrap();
        let decoded: PrincipalOutput = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(o, decoded);
    }
}
