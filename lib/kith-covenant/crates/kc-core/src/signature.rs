//! Signed envelope: payload + CID of canonical-CBOR payload + signatures.
//!
//! `Signed<T>` is the base shape for everything that travels through the
//! archive. Verification re-serializes the payload (canonical CBOR is
//! deterministic), re-computes the CID, checks it matches the frozen
//! `payload_cid`, then verifies every signature against the frozen bytes.

use serde::{Deserialize, Serialize};

use crate::canonical;
use crate::cid::Cid;
use crate::error::{KcError, KcResult};
use crate::keys::{self, Alg, PrivKey, PubKey, Signature};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureEntry {
    pub signer_did: String,
    pub alg: Alg,
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signed<T> {
    pub payload: T,
    pub payload_cid: Cid,
    pub signatures: Vec<SignatureEntry>,
}

impl<T: Serialize> Signed<T> {
    /// Build a signed envelope. Each signer is `(did_string, secret_key)`.
    pub fn build(payload: T, signers: &[(&str, &PrivKey)]) -> KcResult<Self> {
        let bytes = canonical::to_canonical_cbor(&payload)?;
        let cid = Cid::of_bytes(&bytes);
        let mut signatures = Vec::with_capacity(signers.len());
        for (did, sk) in signers {
            let sig = keys::sign(sk, &bytes)?;
            signatures.push(SignatureEntry {
                signer_did: (*did).to_string(),
                alg: sig.alg,
                signature: sig.bytes,
            });
        }
        Ok(Self { payload, payload_cid: cid, signatures })
    }

    pub fn canonical_payload_bytes(&self) -> KcResult<Vec<u8>> {
        canonical::to_canonical_cbor(&self.payload)
    }

    /// Verify the CID matches the re-serialized payload and every signature
    /// verifies against it. The resolver maps `signer_did` → `PubKey`;
    /// unknown signers fail with `KcError::Key`.
    pub fn verify<F>(&self, resolver: F) -> KcResult<()>
    where
        F: Fn(&str) -> Option<PubKey>,
    {
        let bytes = self.canonical_payload_bytes()?;
        let actual_cid = Cid::of_bytes(&bytes);
        if actual_cid != self.payload_cid {
            return Err(KcError::CidMismatch {
                expected: self.payload_cid.to_string(),
                actual: actual_cid.to_string(),
            });
        }
        if self.signatures.is_empty() {
            return Err(KcError::Key("no signatures to verify".into()));
        }
        for entry in &self.signatures {
            let pk = resolver(&entry.signer_did)
                .ok_or_else(|| KcError::Key(format!("no pubkey for {}", entry.signer_did)))?;
            let sig = Signature { alg: entry.alg, bytes: entry.signature.clone() };
            keys::verify(&pk, &bytes, &sig)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_resolver(entries: Vec<(String, PubKey)>) -> impl Fn(&str) -> Option<PubKey> {
        let map: BTreeMap<String, PubKey> = entries.into_iter().collect();
        move |did: &str| map.get(did).cloned()
    }

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Msg {
        text: String,
        counter: u64,
    }

    #[test]
    fn roundtrip_verify_ok() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6test".to_string();
        let signed = Signed::build(
            Msg { text: "hello".into(), counter: 7 },
            &[(&did, &sk)],
        )
        .unwrap();
        let r = make_resolver(vec![(did.clone(), pk)]);
        signed.verify(r).expect("freshly built signed should verify");
    }

    #[test]
    fn tampered_signature_fails() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6tamper".to_string();
        let mut signed = Signed::build(
            Msg { text: "hello".into(), counter: 1 },
            &[(&did, &sk)],
        )
        .unwrap();
        // Flip a signature byte.
        signed.signatures[0].signature[0] ^= 0x01;
        let r = make_resolver(vec![(did, pk)]);
        assert!(matches!(signed.verify(r), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn tampered_payload_cid_mismatch() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6cidmismatch".to_string();
        let mut signed = Signed::build(
            Msg { text: "hello".into(), counter: 1 },
            &[(&did, &sk)],
        )
        .unwrap();
        // Mutate payload without re-computing CID: CID now mismatches.
        signed.payload.text = "goodbye".into();
        let r = make_resolver(vec![(did, pk)]);
        assert!(matches!(signed.verify(r), Err(KcError::CidMismatch { .. })));
    }

    #[test]
    fn tampered_payload_and_cid_still_fails_signature() {
        // If an attacker mutates the payload AND recomputes the CID to match,
        // CID check passes but the signature was over the original bytes and
        // must fail.
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6sigonly".to_string();
        let mut signed = Signed::build(
            Msg { text: "hello".into(), counter: 1 },
            &[(&did, &sk)],
        )
        .unwrap();
        signed.payload.text = "rewritten".into();
        signed.payload_cid = Cid::of(&signed.payload).unwrap();
        let r = make_resolver(vec![(did, pk)]);
        assert!(matches!(signed.verify(r), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn unknown_signer_fails() {
        let (sk, _pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6unknown".to_string();
        let signed = Signed::build(
            Msg { text: "hi".into(), counter: 0 },
            &[(&did, &sk)],
        )
        .unwrap();
        let r = make_resolver(vec![]);
        assert!(matches!(signed.verify(r), Err(KcError::Key(_))));
    }

    #[test]
    fn empty_signatures_fail() {
        // Build a Signed manually with no signatures — verify must reject.
        let payload = Msg { text: "x".into(), counter: 0 };
        let cid = Cid::of(&payload).unwrap();
        let signed: Signed<Msg> = Signed {
            payload,
            payload_cid: cid,
            signatures: vec![],
        };
        assert!(matches!(signed.verify(|_| None), Err(KcError::Key(_))));
    }

    #[test]
    fn multi_signer_all_verified() {
        let (sk1, pk1) = keys::generate(Alg::Ed25519).unwrap();
        let (sk2, pk2) = keys::generate(Alg::Ed25519).unwrap();
        let d1 = "did:key:z6multi1".to_string();
        let d2 = "did:key:z6multi2".to_string();
        let signed = Signed::build(
            Msg { text: "co-signed".into(), counter: 42 },
            &[(&d1, &sk1), (&d2, &sk2)],
        )
        .unwrap();
        assert_eq!(signed.signatures.len(), 2);
        let r = make_resolver(vec![(d1, pk1), (d2, pk2)]);
        signed.verify(r).unwrap();
    }

    #[test]
    fn signed_roundtrips_through_cbor() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let did = "did:key:z6cbor".to_string();
        let signed = Signed::build(
            Msg { text: "wire".into(), counter: 99 },
            &[(&did, &sk)],
        )
        .unwrap();
        let bytes = canonical::to_canonical_cbor(&signed).unwrap();
        let decoded: Signed<Msg> =
            canonical::from_canonical_cbor(&bytes).unwrap();
        let r = make_resolver(vec![(did, pk)]);
        decoded.verify(r).unwrap();
    }
}
