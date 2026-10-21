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

//! Substrate-interpretation records (capture-resistance move 1).
//!
//! Capture most often runs through a *monopoly on interpretation*: a single
//! faction within a member community becomes the canonical interpreter of that
//! community's substrate, and the substrate-deference machinery quietly becomes
//! faction-deference machinery (capture_resistance.md §"The seven strengthening
//! moves", move 1). The strengthening move is to require each community to
//! demonstrate **multiple traceable interpretive lineages** of its own
//! substrate, with the divergences visible in the audit commons
//! (recognition_protocols.md credentialing item 5).
//!
//! A `SubstrateInterpretation` is one such attested reading. Multiple records
//! under distinct `lineage_id`s, authored by distinct `interpreter_did`s, are
//! the plurality the protocol wants; a stream of them all under a single lineage
//! and a single interpreter is the capture signature the jester's
//! `interpretive_plurality` lens (kc-reputation) makes countable. This record is
//! *data only* — the protocol records that a reading was published, not whether
//! its substrate content is correct.

use kc_core::{Cid, DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// One community's attested reading of its own substrate — a single point on an
/// interpretive lineage. Wrapped in `Signed<SubstrateInterpretation>` by the
/// community (root or a delegated key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubstrateInterpretation {
    /// Whose substrate is being read (the member community).
    pub community_did: DidKey,
    /// The named interpretive line this reading belongs to (e.g. "canonical",
    /// "reformist"). Distinct lineage ids are the plurality unit.
    pub lineage_id: String,
    /// The internal party that authored the reading — the anti-monopoly unit. A
    /// declared field (not a separate signing key); see the MVP deferral in
    /// `interpretive_plurality`.
    pub interpreter_did: DidKey,
    /// The rite/recitation this reading interprets, if any (traceability to the
    /// substrate it reads).
    pub interprets_cid: Option<Cid>,
    /// What this reading holds distinctively — the visible divergence that makes
    /// the lineage substantive rather than a rubber stamp.
    pub divergence: String,
    /// Prior lineage records this reading engages (the lineage-graph edges).
    pub references: Vec<Cid>,
    pub recorded_at: Timestamp,
}

/// A third party's attestation that a named interpreter of `community_did`'s
/// substrate is an *independent* principal — the anti-sockpuppet proof for move 1.
/// A declared `interpreter_did` is otherwise just a name the community minted, so
/// a single faction could fake plurality by spinning up interpreter identities it
/// wholly controls. This record lets outside parties vouch for an interpreter's
/// independence; the `interpretive_plurality` lens counts the vouching only when
/// the attester is neither the community nor the interpreter itself. Wrapped in
/// `Signed<InterpreterIndependenceAttestation>` by `attesting_party`. Data only —
/// the record attests independence was claimed, and by whom; whether the claim is
/// well-founded is the audit commons' and jester's concern, not the protocol's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterIndependenceAttestation {
    /// The community whose interpreter is being vouched for.
    pub community_did: DidKey,
    /// The interpreter whose independence is attested.
    pub interpreter_did: DidKey,
    /// The third party vouching. Counts toward independence only when it is
    /// neither `community_did` nor `interpreter_did` (the lens enforces this).
    pub attesting_party: DidKey,
    /// The basis for the independence claim (free text; audit-commons legible).
    pub basis: String,
    pub attested_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{canonical, keys, time, Alg};

    fn sample() -> SubstrateInterpretation {
        let (_, c_pk) = keys::generate(Alg::Ed25519).unwrap();
        let (_, i_pk) = keys::generate(Alg::Ed25519).unwrap();
        SubstrateInterpretation {
            community_did: DidKey::from_pubkey(&c_pk).unwrap(),
            lineage_id: "canonical".into(),
            interpreter_did: DidKey::from_pubkey(&i_pk).unwrap(),
            interprets_cid: None,
            divergence: "Reads Movement 1 as a covenant, not a metaphor.".into(),
            references: vec![],
            recorded_at: time::now(),
        }
    }

    #[test]
    fn interpretation_roundtrips_through_cbor() {
        let r = sample();
        let bytes = canonical::to_canonical_cbor(&r).unwrap();
        let decoded: SubstrateInterpretation = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(r, decoded);
    }
}
