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
        Self("kc-rite-v1".to_string())
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
    pub movement_common_virtue: String,
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
            && !self.movement_common_virtue.trim().is_empty()
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
            movement_common_virtue: "We commit to truthfulness, restitution, hospitality.".into(),
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
