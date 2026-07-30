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

//! Linkable ring signatures over the **existing** Ed25519 identity keys.
//!
//! A ring signature proves that *someone in a named set* signed, without saying
//! which member. `kith-covenant` needs it for one thing (§8): a whistleblower must
//! prove they are an insider of the community they are reporting without the filing
//! being attributable to them. Protection by omission — storing only a hash of the
//! source — proves nothing at all about whether a source exists, which is the
//! failure this module closes.
//!
//! The scheme is LSAG (a linkable spontaneous anonymous group signature): a ring of
//! Schnorr challenge/response pairs that closes on itself, plus a **key image**
//! `I = x·H` that is deterministic in the signer's secret key. The key image is a
//! stable anonymous handle: two filings by the same insider carry the same image, so
//! one source flooding the channel is legible as one source, while nothing about the
//! image reveals *which* ring member produced it.
//!
//! ## Why hand-rolled
//!
//! MVP crypto here is Ed25519-only on trustworthiness grounds (see [`crate::keys`]),
//! and no published ring-signature crate operates on Ed25519 identity keys: the
//! available implementations are either on the wrong curve entirely or on Ristretto,
//! which would force every community to carry a *second* keypair purely to be a
//! possible whistleblower. Working directly on the Ed25519 keys the compact already
//! has is what makes an anonymous filing possible without a key-distribution
//! ceremony. The cost is that this is **unaudited** code: it is exercised by
//! sign/verify tests across every ring position, tamper and reorder rejections, and
//! property tests over ring sizes, but it has had no external review.
//!
//! ## The second generator
//!
//! Linkability needs a second generator `H` whose discrete log with respect to the
//! Ed25519 basepoint `B` is **unknown**. This is not a formality. If `H = h·B` for a
//! known `h`, then anyone can compute `h·P = h·x·B = x·H = I` for every ring member's
//! public key `P` and simply read off which one matches the published image — the
//! signature would be transparent rather than anonymous. `h_point` therefore
//! derives `H` by try-and-increment over a domain-separated Blake3 hash, taking the
//! first candidate that decompresses to a curve point, then clearing torsion with
//! `mul_by_cofactor`. Nothing-up-my-sleeve: the input is a fixed ASCII string.

use std::sync::OnceLock;

use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use ed25519_dalek::hazmat::ExpandedSecretKey;
use serde::{Deserialize, Serialize};

use crate::canonical;
use crate::cid::Cid;
use crate::error::{KcError, KcResult};
use crate::keys::{Alg, PrivKey, PubKey};

/// Domain-separation tags. Every hash in this module is prefixed with exactly one
/// of these, so a digest computed for one purpose can never be reused as another.
const DOMAIN_H: &[u8] = b"kc:ring:generator-H:v1";
const DOMAIN_CHALLENGE: &[u8] = b"kc:ring:challenge:v1";
const DOMAIN_DLEQ: &[u8] = b"kc:ring:dleq-challenge:v1";

/// The second generator `H`: a curve point with unknown discrete log relative to the
/// Ed25519 basepoint. Derived once per process from a fixed ASCII domain string by
/// try-and-increment, then cofactor-cleared so it lies in the prime-order subgroup.
fn h_point() -> &'static EdwardsPoint {
    static H: OnceLock<EdwardsPoint> = OnceLock::new();
    H.get_or_init(|| {
        for counter in 0u32..1000 {
            let mut hasher = blake3::Hasher::new();
            hasher.update(DOMAIN_H);
            hasher.update(&counter.to_be_bytes());
            let candidate = CompressedEdwardsY(*hasher.finalize().as_bytes());
            if let Some(point) = candidate.decompress() {
                let cleared = point.mul_by_cofactor();
                if cleared != EdwardsPoint::default() {
                    return cleared;
                }
            }
        }
        // Unreachable in practice: roughly half of all 32-byte strings decompress,
        // so 1000 misses has probability about 2^-1000.
        unreachable!("no valid curve point in 1000 candidates")
    })
}

/// Hash arbitrary framed input to a scalar. Blake3 in XOF mode gives 64 bytes, which
/// `from_bytes_mod_order_wide` reduces with negligible bias.
fn hash_to_scalar(domain: &[u8], parts: &[&[u8]]) -> Scalar {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    for part in parts {
        // Length-prefix every part so distinct part boundaries cannot collide.
        hasher.update(&(part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    let mut wide = [0u8; 64];
    hasher.finalize_xof().fill(&mut wide);
    Scalar::from_bytes_mod_order_wide(&wide)
}

/// A uniformly random scalar, drawn from the OS CSPRNG.
fn random_scalar() -> Scalar {
    use rand::RngCore;
    let mut wide = [0u8; 64];
    rand::rngs::OsRng.fill_bytes(&mut wide);
    Scalar::from_bytes_mod_order_wide(&wide)
}

/// The secret scalar behind an Ed25519 signing key.
///
/// An Ed25519 private key is a 32-byte *seed*, not a scalar; the scalar is the
/// clamped first half of `SHA-512(seed)`. `ExpandedSecretKey` performs exactly that
/// derivation, so the scalar recovered here is the same one that produces the key's
/// ordinary Ed25519 signatures — which is what lets a ring signature and a normal
/// signature share one identity key.
pub(crate) fn secret_scalar(priv_key: &PrivKey) -> KcResult<Scalar> {
    if priv_key.alg != Alg::Ed25519 {
        return Err(KcError::UnsupportedAlg(format!(
            "ring signatures require Ed25519, got {:?}",
            priv_key.alg
        )));
    }
    let seed: [u8; 32] = priv_key.bytes.as_slice().try_into().map_err(|_| {
        KcError::Key(format!(
            "ed25519 secret key must be 32 bytes, got {}",
            priv_key.bytes.len()
        ))
    })?;
    Ok(ExpandedSecretKey::from(&seed).scalar)
}

/// Decompress a public key to a curve point.
pub(crate) fn public_point(pub_key: &PubKey) -> KcResult<EdwardsPoint> {
    if pub_key.alg != Alg::Ed25519 {
        return Err(KcError::UnsupportedAlg(format!(
            "ring signatures require Ed25519, got {:?}",
            pub_key.alg
        )));
    }
    let bytes: [u8; 32] = pub_key.bytes.as_slice().try_into().map_err(|_| {
        KcError::Key(format!(
            "ed25519 public key must be 32 bytes, got {}",
            pub_key.bytes.len()
        ))
    })?;
    CompressedEdwardsY(bytes)
        .decompress()
        .ok_or_else(|| KcError::Key("ed25519 public key is not a curve point".into()))
}

/// A 32-byte compressed curve point or scalar, serialized as a CBOR byte string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bytes32(#[serde(with = "serde_bytes_array")] pub [u8; 32]);

impl Bytes32 {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

/// `serde_bytes` handles `Vec<u8>` and `&[u8]`; fixed arrays need a shim so they
/// encode as CBOR byte strings rather than arrays of integers.
mod serde_bytes_array {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(v).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let buf: serde_bytes::ByteBuf = Deserialize::deserialize(d)?;
        let v = buf.into_vec();
        v.try_into()
            .map_err(|v: Vec<u8>| serde::de::Error::invalid_length(v.len(), &"32 bytes"))
    }
}

/// `I = x·H`: a value determined by the signer's secret key alone, revealing nothing
/// about which ring member holds it.
///
/// Two signatures carrying the same key image came from the same key. This is the
/// property the observer dashboard counts (distinct sources per accused community)
/// and the property a falsely-named member disproves (see [`DlEqProof`]).
///
/// The image is *global*, not per-ring: one insider filing against two different
/// communities is linkable across those filings. That is deliberate — flood
/// detection needs it — but it is a real disclosure and is documented as such.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyImage(pub Bytes32);

impl KeyImage {
    /// The key image of a signing key.
    pub fn of(priv_key: &PrivKey) -> KcResult<Self> {
        let x = secret_scalar(priv_key)?;
        Ok(Self(Bytes32((x * h_point()).compress().to_bytes())))
    }

    pub fn to_hex(&self) -> String {
        self.0.to_hex()
    }

    fn point(&self) -> KcResult<EdwardsPoint> {
        CompressedEdwardsY(self.0 .0)
            .decompress()
            .ok_or_else(|| KcError::Key("key image is not a curve point".into()))
    }
}

/// An LSAG signature: the seed challenge, one response per ring member, and the
/// signer's key image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingSignature {
    /// The challenge for ring position 0; the verifier walks the ring from here.
    pub c0: Bytes32,
    /// One response scalar per ring member, in ring order.
    pub responses: Vec<Bytes32>,
    pub key_image: KeyImage,
}

/// A payload signed by *one of* a named set of keys.
///
/// Deliberately shaped like [`crate::Signed`]: the payload travels with the CID of
/// its canonical-CBOR encoding, and [`RingSigned::verify`] re-serializes, re-computes
/// the CID, checks it against the frozen one, and only then checks the signature. The
/// difference is what `signatures` becomes — a ring plus one anonymous proof, rather
/// than a list of attributed ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RingSigned<T> {
    pub payload: T,
    pub payload_cid: Cid,
    /// The set the signer proves membership in, in a fixed order. The order is bound
    /// into the challenge chain, so a reordered ring fails verification.
    pub ring: Vec<PubKey>,
    pub signature: RingSignature,
}

/// The challenge for the next ring position. Binding the payload CID, the whole ring
/// in order, and the key image means a signature cannot be lifted onto another
/// payload, replayed against a different ring, or paired with another key image.
fn ring_challenge(
    payload_cid: &Cid,
    ring: &[PubKey],
    key_image: &KeyImage,
    l: &EdwardsPoint,
    r: &EdwardsPoint,
) -> Scalar {
    let mut ring_bytes: Vec<u8> = Vec::with_capacity(ring.len() * 32);
    for member in ring {
        ring_bytes.extend_from_slice(&member.bytes);
    }
    let l_bytes = l.compress().to_bytes();
    let r_bytes = r.compress().to_bytes();
    hash_to_scalar(
        DOMAIN_CHALLENGE,
        &[
            payload_cid.as_bytes(),
            &ring_bytes,
            &key_image.0 .0,
            &l_bytes,
            &r_bytes,
        ],
    )
}

impl<T: Serialize> RingSigned<T> {
    /// Ring-sign `payload` as ring member `signer_index`.
    ///
    /// Fails if the ring has fewer than two members. A ring of one is not a ring: it
    /// names its signer exactly, and producing one would hand a source a signed
    /// confession while telling them they were anonymous.
    pub fn build(
        payload: T,
        ring: Vec<PubKey>,
        signer_index: usize,
        signer_priv: &PrivKey,
    ) -> KcResult<Self> {
        if ring.len() < 2 {
            return Err(KcError::Key(format!(
                "a ring needs at least 2 members to hide a signer, got {}",
                ring.len()
            )));
        }
        if signer_index >= ring.len() {
            return Err(KcError::Key(format!(
                "signer index {} is outside a ring of {}",
                signer_index,
                ring.len()
            )));
        }
        let x = secret_scalar(signer_priv)?;
        let points: Vec<EdwardsPoint> =
            ring.iter().map(public_point).collect::<KcResult<_>>()?;
        // The signer's own public key must be the one at `signer_index`, or the
        // closing step produces a response that does not verify.
        if EdwardsPoint::mul_base(&x) != points[signer_index] {
            return Err(KcError::Key(
                "signer's key does not match its position in the ring".into(),
            ));
        }

        let bytes = canonical::to_canonical_cbor(&payload)?;
        let payload_cid = Cid::of_bytes(&bytes);
        let h = h_point();
        let key_image = KeyImage(Bytes32((x * h).compress().to_bytes()));
        let image_point = x * h;

        let n = ring.len();
        let mut challenges = vec![Scalar::ZERO; n];
        let mut responses = vec![Scalar::ZERO; n];

        // Start at the signer with a random nonce, committing to both generators.
        let alpha = random_scalar();
        let next = (signer_index + 1) % n;
        challenges[next] = ring_challenge(
            &payload_cid,
            &ring,
            &key_image,
            &(EdwardsPoint::mul_base(&alpha)),
            &(alpha * h),
        );

        // Walk the rest of the ring with random responses, forging each link.
        let mut i = next;
        while i != signer_index {
            responses[i] = random_scalar();
            let l = EdwardsPoint::mul_base(&responses[i]) + challenges[i] * points[i];
            let r = responses[i] * h + challenges[i] * image_point;
            let nxt = (i + 1) % n;
            challenges[nxt] = ring_challenge(&payload_cid, &ring, &key_image, &l, &r);
            i = nxt;
        }

        // Close the ring: the one step that needs the secret key.
        responses[signer_index] = alpha - challenges[signer_index] * x;

        Ok(Self {
            payload,
            payload_cid,
            ring,
            signature: RingSignature {
                c0: Bytes32(challenges[0].to_bytes()),
                responses: responses.iter().map(|r| Bytes32(r.to_bytes())).collect(),
                key_image,
            },
        })
    }

    pub fn canonical_payload_bytes(&self) -> KcResult<Vec<u8>> {
        canonical::to_canonical_cbor(&self.payload)
    }

    /// Verify the CID matches the re-serialized payload and the ring closes.
    ///
    /// Returns `Ok(())` without revealing anything about which member signed — the
    /// verifier does the same work for every position.
    pub fn verify(&self) -> KcResult<()> {
        let bytes = self.canonical_payload_bytes()?;
        let actual_cid = Cid::of_bytes(&bytes);
        if actual_cid != self.payload_cid {
            return Err(KcError::CidMismatch {
                expected: self.payload_cid.to_string(),
                actual: actual_cid.to_string(),
            });
        }
        let n = self.ring.len();
        if n < 2 {
            return Err(KcError::Key(format!(
                "a ring needs at least 2 members to hide a signer, got {n}"
            )));
        }
        if self.signature.responses.len() != n {
            return Err(KcError::Key(format!(
                "ring has {n} members but {} responses",
                self.signature.responses.len()
            )));
        }
        let points: Vec<EdwardsPoint> =
            self.ring.iter().map(public_point).collect::<KcResult<_>>()?;
        let image_point = self.signature.key_image.point()?;
        let h = h_point();

        let mut c = scalar_from_canonical(&self.signature.c0)?;
        let c0 = c;
        // Zipped rather than indexed: both lengths were checked equal to `n`
        // above, so this is the same walk without two bounds checks per step.
        for (response, point) in self.signature.responses.iter().zip(points.iter()) {
            let s = scalar_from_canonical(response)?;
            let l = EdwardsPoint::mul_base(&s) + c * point;
            let r = s * h + c * image_point;
            c = ring_challenge(
                &self.payload_cid,
                &self.ring,
                &self.signature.key_image,
                &l,
                &r,
            );
        }
        // The walk must return to where it started.
        if c == c0 {
            Ok(())
        } else {
            Err(KcError::SignatureInvalid)
        }
    }

    /// The ring member public keys, for callers checking the ring against a roster.
    pub fn ring_keys(&self) -> &[PubKey] {
        &self.ring
    }
}

/// Reject non-canonical scalar encodings rather than silently reducing them: an
/// unreduced scalar would give a second valid encoding of the same signature.
fn scalar_from_canonical(b: &Bytes32) -> KcResult<Scalar> {
    Option::from(Scalar::from_canonical_bytes(b.0))
        .ok_or_else(|| KcError::Key("scalar is not canonically encoded".into()))
}

/// A Chaum–Pedersen proof that one secret `x` satisfies **both** `P = x·B` and
/// `I = x·H`, disclosing nothing about `x`.
///
/// This is how a member named by a source reveal clears themselves. Key images are
/// deterministic, so the member computes their own, publishes it with this proof that
/// it is genuinely theirs, and the archive compares it to the image on the filing. A
/// mismatch means the filing was not theirs — proved without surrendering the key,
/// and without needing anyone's permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DlEqProof {
    /// Commitment `r·B`.
    pub commit_base: Bytes32,
    /// Commitment `r·H`.
    pub commit_h: Bytes32,
    /// Response `z = r + c·x`.
    pub response: Bytes32,
}

fn dleq_challenge(
    context: &[u8],
    p: &EdwardsPoint,
    i: &EdwardsPoint,
    commit_base: &EdwardsPoint,
    commit_h: &EdwardsPoint,
) -> Scalar {
    hash_to_scalar(
        DOMAIN_DLEQ,
        &[
            context,
            &p.compress().to_bytes(),
            &i.compress().to_bytes(),
            &commit_base.compress().to_bytes(),
            &commit_h.compress().to_bytes(),
        ],
    )
}

impl DlEqProof {
    /// Prove that `priv_key`'s scalar links its public key to its key image.
    /// `context` is bound into the challenge so a proof cannot be replayed onto
    /// another record.
    pub fn prove(priv_key: &PrivKey, context: &[u8]) -> KcResult<(KeyImage, Self)> {
        let x = secret_scalar(priv_key)?;
        let h = h_point();
        let p = EdwardsPoint::mul_base(&x);
        let image = x * h;
        let r = random_scalar();
        let commit_base = EdwardsPoint::mul_base(&r);
        let commit_h = r * h;
        let c = dleq_challenge(context, &p, &image, &commit_base, &commit_h);
        let z = r + c * x;
        Ok((
            KeyImage(Bytes32(image.compress().to_bytes())),
            Self {
                commit_base: Bytes32(commit_base.compress().to_bytes()),
                commit_h: Bytes32(commit_h.compress().to_bytes()),
                response: Bytes32(z.to_bytes()),
            },
        ))
    }

    /// Check the proof ties `pub_key` to `key_image`.
    pub fn verify(&self, pub_key: &PubKey, key_image: &KeyImage, context: &[u8]) -> KcResult<()> {
        let p = public_point(pub_key)?;
        let image = key_image.point()?;
        let commit_base = CompressedEdwardsY(self.commit_base.0)
            .decompress()
            .ok_or_else(|| KcError::Key("dleq commitment is not a curve point".into()))?;
        let commit_h = CompressedEdwardsY(self.commit_h.0)
            .decompress()
            .ok_or_else(|| KcError::Key("dleq commitment is not a curve point".into()))?;
        let z = scalar_from_canonical(&self.response)?;
        let h = h_point();
        let c = dleq_challenge(context, &p, &image, &commit_base, &commit_h);
        if EdwardsPoint::mul_base(&z) != commit_base + c * p {
            return Err(KcError::SignatureInvalid);
        }
        if z * h != commit_h + c * image {
            return Err(KcError::SignatureInvalid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    struct Claim {
        text: String,
        n: u64,
    }

    fn claim(text: &str) -> Claim {
        Claim { text: text.into(), n: 7 }
    }

    fn ring_of(n: usize) -> (Vec<PrivKey>, Vec<PubKey>) {
        let mut privs = Vec::new();
        let mut pubs = Vec::new();
        for _ in 0..n {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            privs.push(sk);
            pubs.push(pk);
        }
        (privs, pubs)
    }

    #[test]
    fn h_point_is_in_prime_order_subgroup() {
        let h = h_point();
        assert!(h.is_torsion_free());
        assert_ne!(h, &EdwardsPoint::default());
        // Stable across calls (it is memoized, but also deterministic).
        assert_eq!(h, h_point());
    }

    #[test]
    fn secret_scalar_matches_the_ed25519_public_key() {
        // The scalar recovered from the seed must be the one behind the published
        // public key, or ring membership checks would reject honest signers.
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let x = secret_scalar(&sk).unwrap();
        assert_eq!(EdwardsPoint::mul_base(&x), public_point(&pk).unwrap());
    }

    #[test]
    fn every_ring_position_can_sign_and_verify() {
        for n in 2..=8usize {
            let (privs, pubs) = ring_of(n);
            for (i, priv_key) in privs.iter().enumerate() {
                let signed =
                    RingSigned::build(claim("insider"), pubs.clone(), i, priv_key).unwrap();
                signed
                    .verify()
                    .unwrap_or_else(|e| panic!("ring {n} position {i} failed: {e}"));
            }
        }
    }

    #[test]
    fn ring_of_one_is_refused() {
        let (privs, pubs) = ring_of(1);
        let err = RingSigned::build(claim("alone"), pubs, 0, &privs[0]).unwrap_err();
        assert!(matches!(err, KcError::Key(_)));
    }

    #[test]
    fn signer_index_must_match_the_signers_key() {
        let (privs, pubs) = ring_of(3);
        // Claim position 1 while holding position 0's key.
        let err = RingSigned::build(claim("mismatch"), pubs, 1, &privs[0]).unwrap_err();
        assert!(matches!(err, KcError::Key(_)));
    }

    #[test]
    fn signer_index_out_of_range_is_refused() {
        let (privs, pubs) = ring_of(3);
        let err = RingSigned::build(claim("oob"), pubs, 5, &privs[0]).unwrap_err();
        assert!(matches!(err, KcError::Key(_)));
    }

    #[test]
    fn non_member_cannot_sign() {
        let (_privs, pubs) = ring_of(4);
        let (outsider, _) = keys::generate(Alg::Ed25519).unwrap();
        // An outsider has no valid position, so every index it claims is refused.
        for i in 0..4 {
            assert!(RingSigned::build(claim("outsider"), pubs.clone(), i, &outsider).is_err());
        }
    }

    #[test]
    fn tampered_payload_fails_the_cid_check() {
        let (privs, pubs) = ring_of(3);
        let mut signed = RingSigned::build(claim("original"), pubs, 2, &privs[2]).unwrap();
        signed.payload.text = "rewritten".into();
        assert!(matches!(signed.verify(), Err(KcError::CidMismatch { .. })));
    }

    #[test]
    fn tampered_payload_with_recomputed_cid_fails_the_ring() {
        // Recomputing the CID gets past the first check; the challenge chain is
        // bound to the CID, so the ring no longer closes.
        let (privs, pubs) = ring_of(3);
        let mut signed = RingSigned::build(claim("original"), pubs, 1, &privs[1]).unwrap();
        signed.payload.text = "rewritten".into();
        signed.payload_cid = Cid::of(&signed.payload).unwrap();
        assert!(matches!(signed.verify(), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn reordered_ring_fails() {
        let (privs, pubs) = ring_of(4);
        let mut signed = RingSigned::build(claim("ordered"), pubs, 0, &privs[0]).unwrap();
        signed.ring.swap(1, 3);
        assert!(matches!(signed.verify(), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn substituted_ring_member_fails() {
        let (privs, pubs) = ring_of(3);
        let (_, outsider_pub) = keys::generate(Alg::Ed25519).unwrap();
        let mut signed = RingSigned::build(claim("ring"), pubs, 0, &privs[0]).unwrap();
        signed.ring[2] = outsider_pub;
        assert!(matches!(signed.verify(), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn tampered_response_fails() {
        let (privs, pubs) = ring_of(3);
        let mut signed = RingSigned::build(claim("responses"), pubs, 2, &privs[2]).unwrap();
        signed.signature.responses[0].0[0] ^= 0x01;
        assert!(matches!(
            signed.verify(),
            Err(KcError::SignatureInvalid) | Err(KcError::Key(_))
        ));
    }

    #[test]
    fn substituted_key_image_fails() {
        let (privs, pubs) = ring_of(3);
        let (other, _) = keys::generate(Alg::Ed25519).unwrap();
        let mut signed = RingSigned::build(claim("image"), pubs, 1, &privs[1]).unwrap();
        signed.signature.key_image = KeyImage::of(&other).unwrap();
        assert!(matches!(signed.verify(), Err(KcError::SignatureInvalid)));
    }

    #[test]
    fn wrong_response_count_fails() {
        let (privs, pubs) = ring_of(3);
        let mut signed = RingSigned::build(claim("count"), pubs, 0, &privs[0]).unwrap();
        signed.signature.responses.pop();
        assert!(matches!(signed.verify(), Err(KcError::Key(_))));
    }

    #[test]
    fn key_image_is_deterministic_and_key_specific() {
        let (a_priv, _) = keys::generate(Alg::Ed25519).unwrap();
        let (b_priv, _) = keys::generate(Alg::Ed25519).unwrap();
        assert_eq!(KeyImage::of(&a_priv).unwrap(), KeyImage::of(&a_priv).unwrap());
        assert_ne!(KeyImage::of(&a_priv).unwrap(), KeyImage::of(&b_priv).unwrap());
    }

    #[test]
    fn same_signer_links_across_payloads_and_rings() {
        // The linkability property: one insider filing twice is one key image, even
        // with a different ring and a different payload.
        let (privs, pubs) = ring_of(3);
        let (_extra_priv, extra_pub) = keys::generate(Alg::Ed25519).unwrap();
        let first = RingSigned::build(claim("first"), pubs.clone(), 1, &privs[1]).unwrap();
        let mut other_ring = pubs.clone();
        other_ring[0] = extra_pub;
        let second = RingSigned::build(claim("second"), other_ring, 1, &privs[1]).unwrap();
        first.verify().unwrap();
        second.verify().unwrap();
        assert_eq!(first.signature.key_image, second.signature.key_image);
        assert_eq!(first.signature.key_image, KeyImage::of(&privs[1]).unwrap());
    }

    #[test]
    fn different_signers_in_one_ring_do_not_link() {
        let (privs, pubs) = ring_of(3);
        let a = RingSigned::build(claim("same ring"), pubs.clone(), 0, &privs[0]).unwrap();
        let b = RingSigned::build(claim("same ring"), pubs, 2, &privs[2]).unwrap();
        assert_ne!(a.signature.key_image, b.signature.key_image);
    }

    #[test]
    fn signature_shape_is_independent_of_signer_position() {
        // Anonymity, structurally: two signatures over the same payload and ring by
        // different members are indistinguishable in shape. Nothing in the encoding
        // says where the signer sat.
        let (privs, pubs) = ring_of(5);
        let a = RingSigned::build(claim("who signed"), pubs.clone(), 0, &privs[0]).unwrap();
        let b = RingSigned::build(claim("who signed"), pubs.clone(), 4, &privs[4]).unwrap();
        assert_eq!(a.signature.responses.len(), b.signature.responses.len());
        let a_bytes = canonical::to_canonical_cbor(&a).unwrap();
        let b_bytes = canonical::to_canonical_cbor(&b).unwrap();
        assert_eq!(a_bytes.len(), b_bytes.len());
        // And no response is a fixed marker of the signer's slot.
        assert_ne!(a.signature.responses[0], b.signature.responses[0]);
    }

    #[test]
    fn roundtrips_through_canonical_cbor() {
        let (privs, pubs) = ring_of(4);
        let signed = RingSigned::build(claim("wire"), pubs, 3, &privs[3]).unwrap();
        let bytes = canonical::to_canonical_cbor(&signed).unwrap();
        let decoded: RingSigned<Claim> = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(decoded, signed);
        decoded.verify().unwrap();
    }

    #[test]
    fn non_canonical_scalar_is_refused() {
        let (privs, pubs) = ring_of(3);
        let mut signed = RingSigned::build(claim("canon"), pubs, 0, &privs[0]).unwrap();
        // All-ones is larger than the group order, so it has no canonical reading.
        signed.signature.responses[1] = Bytes32([0xff; 32]);
        assert!(matches!(signed.verify(), Err(KcError::Key(_))));
    }

    #[test]
    fn dleq_proof_verifies_and_binds_its_key() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let (other_sk, other_pk) = keys::generate(Alg::Ed25519).unwrap();
        let (image, proof) = DlEqProof::prove(&sk, b"ctx").unwrap();
        assert_eq!(image, KeyImage::of(&sk).unwrap());
        proof.verify(&pk, &image, b"ctx").unwrap();
        // Wrong public key, wrong image, wrong context: all refused.
        assert!(proof.verify(&other_pk, &image, b"ctx").is_err());
        assert!(proof
            .verify(&pk, &KeyImage::of(&other_sk).unwrap(), b"ctx")
            .is_err());
        assert!(proof.verify(&pk, &image, b"other ctx").is_err());
    }

    #[test]
    fn tampered_dleq_proof_fails() {
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let (image, proof) = DlEqProof::prove(&sk, b"ctx").unwrap();
        let mut bad = proof.clone();
        bad.response.0[0] ^= 0x01;
        assert!(bad.verify(&pk, &image, b"ctx").is_err());
        let mut bad2 = proof;
        bad2.commit_h.0[0] ^= 0x01;
        assert!(bad2.verify(&pk, &image, b"ctx").is_err());
    }

    #[test]
    fn dleq_cannot_claim_someone_elses_image() {
        // The exculpation gate: you cannot produce a valid proof for an image that
        // is not yours, so you cannot disown a filing by claiming a different image.
        let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        let (victim_sk, _) = keys::generate(Alg::Ed25519).unwrap();
        let (_image, proof) = DlEqProof::prove(&sk, b"ctx").unwrap();
        let victim_image = KeyImage::of(&victim_sk).unwrap();
        assert!(proof.verify(&pk, &victim_image, b"ctx").is_err());
    }
}
