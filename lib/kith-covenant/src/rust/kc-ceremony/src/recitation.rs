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

//! The five-movement recitation. Data type only — signing + sealing live in
//! the `intra` submodule. Movement content is community-supplied: the
//! protocol records *that* the movements were recited, not *what* their
//! substrate is. See doc/background/recognition_protocols.md.

use kc_core::{DidKey, Timestamp};
use serde::{Deserialize, Serialize};

/// The five protocol movements. Order is semantic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Movement {
    /// Movement 1 — naming the substrate each community anchors above itself.
    Substrate,
    /// Movement 2 — acknowledging fallenness (fallen-actor anthropology).
    Fallenness,
    /// Movement 3 — recognizing the other community as a legitimate interlocutor.
    Recognition,
    /// Movement 4 — committing to the common virtues the compact relies on.
    CommonVirtue,
    /// Movement 5 — submitting the compact itself to the substrate (non-ultimacy).
    NonUltimacy,
}

/// Protocol version of the recitation schema. Bumped when movement semantics change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecitationVersion(pub String);

impl RecitationVersion {
    pub fn current() -> Self {
        Self("kc-rite-v2".to_string())
    }
}

/// One commitment of the common-virtue movement: a stable key and the text recited
/// for it.
///
/// The key is what an enforcement act cites, so it is the identity of the commitment
/// across re-recitations and across communities. Two communities using the same key
/// are held to be committing to the same thing, which is what lets a compact have an
/// object at all rather than a list of unrelated private vocabularies. The text is
/// what was actually said, in the recitation's own language.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Commitment {
    pub key: String,
    pub text: String,
}

impl Commitment {
    pub fn new(key: impl Into<String>, text: impl Into<String>) -> Self {
        Self { key: key.into(), text: text.into() }
    }

    pub fn well_formed(&self) -> bool {
        !self.key.trim().is_empty() && !self.text.trim().is_empty()
    }
}

/// Movement 4, the common virtues the community holds.
///
/// The one movement with structure, and the reason is that it is the only one a later
/// act can be *about*. Movements 1, 2, 3 and 5 are stances; this one is a list of
/// undertakings, and an undertaking the compact stops enforcing is the substitution
/// the audit is looking for (`doc/background/substitution.md`).
///
/// The recited text is **derived** from the commitments rather than stored beside
/// them. A second field holding the prose would be a field no rule reads that can
/// contradict the rule that does, which is the shape this codebase keeps finding on
/// the other side of a missing rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonVirtueMovement {
    /// The framing sentence, in the recitation's own language.
    pub preamble: String,
    /// The undertakings, in the order recited. Keys are unique within one recitation.
    pub commitments: Vec<Commitment>,
}

impl CommonVirtueMovement {
    pub fn new(preamble: impl Into<String>, commitments: Vec<Commitment>) -> Self {
        Self { preamble: preamble.into(), commitments }
    }

    /// The movement as spoken: the preamble, then the commitments in order.
    pub fn recited_text(&self) -> String {
        let mut out = self.preamble.trim().to_string();
        for (i, c) in self.commitments.iter().enumerate() {
            out.push_str(&format!("\n{}. {}", i + 1, c.text.trim()));
        }
        out
    }

    pub fn keys(&self) -> Vec<&str> {
        self.commitments.iter().map(|c| c.key.trim()).collect()
    }

    /// A preamble, at least one commitment, every commitment well formed, and no key
    /// recited twice.
    ///
    /// "At least one" is not a formality: a denominator that can be empty is not a
    /// denominator, and a compact whose members recite no commitments at all would
    /// read as perfectly covered by any enforcement whatsoever.
    pub fn well_formed(&self) -> bool {
        if self.preamble.trim().is_empty() || self.commitments.is_empty() {
            return false;
        }
        if !self.commitments.iter().all(Commitment::well_formed) {
            return false;
        }
        let mut seen = std::collections::BTreeSet::new();
        self.commitments.iter().all(|c| seen.insert(c.key.trim()))
    }
}

/// A single community's recitation. Community-supplied content in each
/// movement — protocol only records the form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recitation {
    pub community_did: DidKey,
    pub speaker_did: DidKey,
    pub version: RecitationVersion,
    pub movement_substrate: String,
    pub movement_fallenness: String,
    pub movement_recognition: String,
    pub movement_common_virtue: CommonVirtueMovement,
    pub movement_non_ultimacy: String,
    pub language_tag: String,
    pub translation_en: Option<String>,
    pub recited_at: Timestamp,
}

impl Recitation {
    /// Does the recitation have non-empty text for all five movements?
    pub fn movements_all_present(&self) -> bool {
        !self.movement_substrate.trim().is_empty()
            && !self.movement_fallenness.trim().is_empty()
            && !self.movement_recognition.trim().is_empty()
            && self.movement_common_virtue.well_formed()
            && !self.movement_non_ultimacy.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kc_core::{canonical, keys, time, Alg};

    fn sample_recitation() -> Recitation {
        let (_, c_pk) = keys::generate(Alg::Ed25519).unwrap();
        let (_, s_pk) = keys::generate(Alg::Ed25519).unwrap();
        Recitation {
            community_did: DidKey::from_pubkey(&c_pk).unwrap(),
            speaker_did: DidKey::from_pubkey(&s_pk).unwrap(),
            version: RecitationVersion::current(),
            movement_substrate: "We anchor above ourselves in the source we did not make.".into(),
            movement_fallenness: "We acknowledge we are capable of harm.".into(),
            movement_recognition: "We recognize the other as a community bound by their own substrate.".into(),
            movement_common_virtue: CommonVirtueMovement::new(
                "We commit to the virtues this compact relies on.",
                vec![
                    Commitment::new("truthfulness", "We tell the truth about what we have done."),
                    Commitment::new("restitution", "We make good what we have broken."),
                    Commitment::new("hospitality", "We receive the stranger."),
                ],
            ),
            movement_non_ultimacy: "This compact does not claim ultimacy for itself.".into(),
            language_tag: "en".into(),
            translation_en: None,
            recited_at: time::now(),
        }
    }

    #[test]
    fn all_movements_present_when_populated() {
        let r = sample_recitation();
        assert!(r.movements_all_present());
    }

    #[test]
    fn missing_movement_detected() {
        let mut r = sample_recitation();
        r.movement_substrate = "   ".into();
        assert!(!r.movements_all_present());
    }

    #[test]
    fn a_movement_four_with_no_commitments_is_not_present() {
        // A denominator that can be empty is not a denominator.
        let mut r = sample_recitation();
        r.movement_common_virtue.commitments.clear();
        assert!(!r.movements_all_present());
    }

    #[test]
    fn a_movement_four_with_no_preamble_is_not_present() {
        let mut r = sample_recitation();
        r.movement_common_virtue.preamble = "  ".into();
        assert!(!r.movements_all_present());
    }

    #[test]
    fn a_key_recited_twice_is_refused() {
        // Two entries under one key would count once in the object and twice in the
        // recitation, so the list and its own denominator would disagree.
        let mut r = sample_recitation();
        r.movement_common_virtue
            .commitments
            .push(Commitment::new("truthfulness", "and again"));
        assert!(!r.movement_common_virtue.well_formed());
        assert!(!r.movements_all_present());
    }

    #[test]
    fn a_blank_key_or_text_is_refused() {
        let mut r = sample_recitation();
        r.movement_common_virtue.commitments[1].key = "  ".into();
        assert!(!r.movements_all_present());
        let mut r = sample_recitation();
        r.movement_common_virtue.commitments[1].text = "".into();
        assert!(!r.movements_all_present());
    }

    #[test]
    fn the_recited_text_is_derived_from_the_commitments() {
        // Not stored beside them: there is one representation, so the prose and the
        // list cannot disagree.
        let r = sample_recitation();
        let text = r.movement_common_virtue.recited_text();
        assert!(text.starts_with("We commit to the virtues"));
        for (i, c) in r.movement_common_virtue.commitments.iter().enumerate() {
            assert!(text.contains(&format!("{}. {}", i + 1, c.text)), "{text}");
        }
    }

    #[test]
    fn the_version_names_the_schema_that_carries_a_list() {
        // Movement semantics changed, which is the case this constant exists for.
        assert_eq!(RecitationVersion::current().0, "kc-rite-v2");
    }

    #[test]
    fn recitation_roundtrips_through_cbor() {
        let r = sample_recitation();
        let bytes = canonical::to_canonical_cbor(&r).unwrap();
        let decoded: Recitation = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(r, decoded);
    }

    #[test]
    fn movement_enum_stable_serialization() {
        // Guardrail: if enum variant names ever change, serialized records
        // from older runs will no longer decode. This test pins the form.
        for (m, name) in [
            (Movement::Substrate, "\"Substrate\""),
            (Movement::Fallenness, "\"Fallenness\""),
            (Movement::Recognition, "\"Recognition\""),
            (Movement::CommonVirtue, "\"CommonVirtue\""),
            (Movement::NonUltimacy, "\"NonUltimacy\""),
        ] {
            let j = serde_json::to_string(&m).unwrap();
            assert_eq!(j, name);
        }
    }
}
