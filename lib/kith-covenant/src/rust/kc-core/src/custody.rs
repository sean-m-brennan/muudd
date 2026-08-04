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

//! Threshold-held secrets: a value that **requires** k of n holders to recover.
//!
//! §8 asks for "threshold-held source records: whistleblower identity recoverable only
//! with mediator-panel quorum and only for named cause. Source protection must be
//! structurally stronger than individual mediator integrity." A panel that merely
//! *agrees* to keep a name until quorum is exactly as strong as its least reliable
//! member. This module replaces the promise with arithmetic: fewer than k shares
//! recover nothing.
//!
//! The construction is Feldman verifiable secret sharing. A random scalar `s` is split
//! by a degree `k-1` polynomial, the coefficients are committed to as curve points on
//! the public record, and the secret to be protected is encrypted under a key derived
//! from `s`. Two properties follow, and the second is the reason for choosing Feldman
//! over plain Shamir:
//!
//! 1. Any k holders recover `s` by Lagrange interpolation, and fewer learn nothing.
//! 2. **The sharing is checkable at the moment it is published.** Each share verifies
//!    against the public commitments on its own, so a dealer who hands out
//!    inconsistent shares is caught immediately, and any single honest holder can
//!    demonstrate that their own share is bad. Under plain Shamir the fraud surfaces
//!    only when a recovery is attempted — possibly years later, at the moment the
//!    record matters most.
//!
//! Shares are sealed to their holders on the record itself, so custody is auditable:
//! anyone can confirm that the n shares went to exactly the n panelists and to nobody
//! else, without being able to open any of them.
//!
//! ## The sealing key, and who it must belong to
//!
//! Naming a custodian and sealing to them are two different things, and for a while
//! only the first was checked. [`ThresholdCustody::seal`] took `(DidKey, PubKey)` pairs
//! with nothing anywhere requiring the two to correspond, so a record could name the
//! right panel and seal every share to one outsider's key: structurally valid, fully
//! authorized, and **no** named custodian able to open anything. The reveal path was
//! then permanently dead with nothing on the record to show it — precisely the
//! late-detection failure that property 2 above rejects plain Shamir for, except worse,
//! because the recipient key was not recorded at all and so nobody but the holder who
//! tried could ever discover it.
//!
//! So the key is now **resolved rather than supplied** (see `seal`), and recorded on
//! each share as `recipient_pubkey` for the authority pass to check against the
//! holder's registration. Both halves are needed: resolution keeps the honest caller
//! from getting it wrong, and the recorded key is what a hand-built record cannot lie
//! about undetected.
//!
//! Sealing performs Diffie–Hellman in Edwards form against that key. Where a holder has
//! no separate encryption key, [`didkey_encryption_resolver`] falls back to its
//! `did:key` — i.e. the key it also signs with, one key serving two primitives, which is
//! a compromise mitigated only by domain-separated derivation. Mediators no longer rely
//! on that: `MediatorEntry` carries an `encryption_pubkey` distinct from its signing key
//! (the authority pass refuses a registration where the two are equal), and
//! `kc_archive::mediator_encryption_resolver` is what the §8 custody path uses.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use curve25519_dalek::edwards::EdwardsPoint;
use curve25519_dalek::scalar::Scalar;
use serde::{Deserialize, Serialize};

use crate::cid::Cid;
use crate::did::DidKey;
use crate::error::{KcError, KcResult};
use crate::keys::{Alg, PrivKey, PubKey};
use crate::ring::Bytes32;

const DOMAIN_RECORD_KEY: &[u8] = b"kc:custody:record-key:v1";
const DOMAIN_SEAL_KEY: &[u8] = b"kc:custody:seal-key:v1";
const AAD_RECORD: &[u8] = b"kc:custody:source-record:v1";
const AAD_SHARE: &[u8] = b"kc:custody:sealed-share:v1";

/// The thing a `source_commitment` commits to.
///
/// Kept as an explicit struct rather than a concatenation so the commitment is
/// `Cid::of(&SourcePreimage)` over canonical CBOR — reproducible from any language
/// binding, which matters because the reveal rule recomputes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePreimage {
    /// The source's identity, as a DID string.
    pub identity: String,
    /// A random nonce, so the commitment cannot be brute-forced by guessing
    /// identities from a small candidate set.
    pub nonce: Bytes32,
}

impl SourcePreimage {
    /// Commit to `identity` under a fresh random nonce.
    pub fn new(identity: impl Into<String>) -> Self {
        use rand::RngCore;
        let mut nonce = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        Self { identity: identity.into(), nonce: Bytes32(nonce) }
    }

    /// The commitment: the CID of this preimage's canonical encoding.
    pub fn commitment(&self) -> KcResult<Cid> {
        Cid::of(self)
    }
}

/// One holder's share, encrypted so only that holder can open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedShare {
    /// Who this share was sealed to. The authority pass checks the set of holders
    /// against the reception panel, so nobody can be quietly added or left out.
    pub holder_did: DidKey,
    /// The share's x-coordinate. Indices start at 1 (0 is the secret itself).
    pub index: u8,
    /// The key this share was actually sealed to. On the record because naming a
    /// custodian is not the same as sealing to them: without this, a share sealed to
    /// somebody else's key is indistinguishable from an honest one until the named
    /// holder tries to open it and fails. The authority pass requires it to be the key
    /// that holder is registered under.
    pub recipient_pubkey: Bytes32,
    /// The dealer's one-time public key for this share's key agreement.
    pub ephemeral_pub: Bytes32,
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
    pub nonce: Bytes32,
}

/// A share a holder has opened and published, so it can be interpolated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevealedShare {
    pub index: u8,
    pub value: Bytes32,
}

/// The public half of a threshold-held secret: the commitments that make the sharing
/// checkable, the sealed shares, and the encrypted payload itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdCustody {
    /// How many shares recovery requires.
    pub threshold_k: u8,
    /// Feldman commitments `C_j = a_j·B` for the polynomial's `k` coefficients.
    /// `commitments[0]` commits to the secret itself.
    pub commitments: Vec<Bytes32>,
    pub sealed_shares: Vec<SealedShare>,
    /// The protected payload, encrypted under a key derived from the secret scalar.
    #[serde(with = "serde_bytes")]
    pub ciphertext: Vec<u8>,
    pub nonce: Bytes32,
}

/// Resolve a holder's sealing key from its own `did:key`.
///
/// For holders that carry no separate encryption key. This is the one-key-two-primitives
/// case: the key returned is the key the holder signs with. Prefer a registry-backed
/// resolver where one exists — `kc_archive::mediator_encryption_resolver` for §8 custody.
pub fn didkey_encryption_resolver(did: &DidKey) -> KcResult<PubKey> {
    did.to_pubkey()
}

fn derive_key(domain: &[u8], material: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(material);
    *hasher.finalize().as_bytes()
}

fn random_bytes<const N: usize>() -> [u8; N] {
    use rand::RngCore;
    let mut out = [0u8; N];
    rand::rngs::OsRng.fill_bytes(&mut out);
    out
}

fn random_scalar() -> Scalar {
    Scalar::from_bytes_mod_order_wide(&random_bytes::<64>())
}

fn aead_seal(key: &[u8; 32], aad: &[u8], plaintext: &[u8]) -> KcResult<(Vec<u8>, Bytes32)> {
    let cipher = ChaCha20Poly1305::new(key.into());
    // A fresh 96-bit nonce per sealing; keys here are single-use anyway.
    let raw = random_bytes::<32>();
    let nonce = Nonce::from_slice(&raw[..12]);
    let ct = cipher
        .encrypt(nonce, Payload { msg: plaintext, aad })
        .map_err(|_| KcError::Key("aead encryption failed".into()))?;
    Ok((ct, Bytes32(raw)))
}

fn aead_open(key: &[u8; 32], aad: &[u8], ct: &[u8], nonce: &Bytes32) -> KcResult<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let n = Nonce::from_slice(&nonce.0[..12]);
    cipher
        .decrypt(n, Payload { msg: ct, aad })
        .map_err(|_| KcError::Key("aead decryption failed".into()))
}

/// The shared secret between a scalar and a public key, as an Edwards-form key
/// agreement. Both sides multiply by the cofactor so a small-order public key cannot
/// force a predictable result.
fn agree(secret: &Scalar, peer: &PubKey) -> KcResult<[u8; 32]> {
    let point = crate::ring::public_point(peer)?;
    Ok((secret * point).mul_by_cofactor().compress().to_bytes())
}

fn expanded_scalar(priv_key: &PrivKey) -> KcResult<Scalar> {
    crate::ring::secret_scalar(priv_key)
}

/// Evaluate the sharing polynomial at `x`.
fn poly_eval(coefficients: &[Scalar], x: Scalar) -> Scalar {
    // Horner, from the highest coefficient down.
    let mut acc = Scalar::ZERO;
    for c in coefficients.iter().rev() {
        acc = acc * x + c;
    }
    acc
}

/// The Lagrange coefficient for `index` within `indices`, evaluated at 0.
fn lagrange_at_zero(index: u8, indices: &[u8]) -> KcResult<Scalar> {
    let xi = Scalar::from(u64::from(index));
    let mut num = Scalar::ONE;
    let mut den = Scalar::ONE;
    for &other in indices {
        if other == index {
            continue;
        }
        let xj = Scalar::from(u64::from(other));
        num *= xj;
        den *= xj - xi;
    }
    if den == Scalar::ZERO {
        return Err(KcError::Key("duplicate share indices".into()));
    }
    Ok(num * den.invert())
}

impl ThresholdCustody {
    /// Seal `plaintext` so that `threshold_k` of `holders` are required to recover it.
    ///
    /// `holders` are DIDs, in share-index order (each gets index `i+1`). The key each
    /// share is sealed to is **resolved**, never supplied: an earlier signature took
    /// `(DidKey, PubKey)` pairs and nothing checked that the two corresponded, so a
    /// caller could name the right custodians and seal to a key none of them held. That
    /// produced a record which read as auditable k-of-n custody held by exactly the
    /// panel, passed every structural check, and left **no** custodian able to open a
    /// share — the lawful reveal path dead, with nothing on the record to show it.
    ///
    /// Use [`didkey_encryption_resolver`] where a holder's key is its own `did:key`, or
    /// a registry-backed resolver (`kc_archive::mediator_encryption_resolver`) where
    /// holders carry a separate encryption key. The resolved key is recorded on each
    /// share, so the pairing is checkable by inspection rather than only by the holder
    /// who tries — which is the same argument that chose Feldman over plain Shamir for
    /// the sharing half, applied to the sealing half.
    pub fn seal(
        plaintext: &[u8],
        holders: &[DidKey],
        resolve: impl Fn(&DidKey) -> KcResult<PubKey>,
        threshold_k: usize,
    ) -> KcResult<Self> {
        let n = holders.len();
        if threshold_k < 2 {
            return Err(KcError::Key(format!(
                "a threshold of {threshold_k} is not a threshold: custody must need at least 2 holders"
            )));
        }
        if threshold_k > n {
            return Err(KcError::Key(format!(
                "threshold {threshold_k} exceeds the {n} holders, so the secret could never be recovered"
            )));
        }
        if n > u8::MAX as usize - 1 {
            return Err(KcError::Key(format!("too many holders: {n}")));
        }

        // The sharing polynomial: a_0 is the secret.
        let coefficients: Vec<Scalar> = (0..threshold_k).map(|_| random_scalar()).collect();
        let commitments: Vec<Bytes32> = coefficients
            .iter()
            .map(|a| Bytes32(EdwardsPoint::mul_base(a).compress().to_bytes()))
            .collect();

        let record_key = derive_key(DOMAIN_RECORD_KEY, coefficients[0].as_bytes());
        let (ciphertext, nonce) = aead_seal(&record_key, AAD_RECORD, plaintext)?;

        let mut sealed_shares = Vec::with_capacity(n);
        for (i, did) in holders.iter().enumerate() {
            let index = u8::try_from(i + 1).map_err(|_| KcError::Key("share index".into()))?;
            let pk = resolve(did)?;
            let value = poly_eval(&coefficients, Scalar::from(u64::from(index)));
            let ephemeral = random_scalar();
            let ephemeral_pub = EdwardsPoint::mul_base(&ephemeral);
            let seal_key = derive_key(DOMAIN_SEAL_KEY, &agree(&ephemeral, &pk)?);
            let (ct, share_nonce) = aead_seal(&seal_key, AAD_SHARE, value.as_bytes())?;
            sealed_shares.push(SealedShare {
                holder_did: did.clone(),
                index,
                recipient_pubkey: Bytes32(
                    pk.bytes
                        .clone()
                        .try_into()
                        .map_err(|_| KcError::Key("recipient key is not 32 bytes".into()))?,
                ),
                ephemeral_pub: Bytes32(ephemeral_pub.compress().to_bytes()),
                ciphertext: ct,
                nonce: share_nonce,
            });
        }

        Ok(Self {
            threshold_k: u8::try_from(threshold_k).map_err(|_| KcError::Key("threshold".into()))?,
            commitments,
            sealed_shares,
            ciphertext,
            nonce,
        })
    }

    /// Structural checks that hold without opening anything: the threshold is a real
    /// threshold, there is one commitment per coefficient, and holders are distinct.
    pub fn check_shape(&self) -> KcResult<()> {
        let k = usize::from(self.threshold_k);
        if k < 2 {
            return Err(KcError::Key(format!(
                "custody threshold {k} does not require a quorum"
            )));
        }
        if self.commitments.len() != k {
            return Err(KcError::Key(format!(
                "custody has {} commitments for a threshold of {k}",
                self.commitments.len()
            )));
        }
        if self.sealed_shares.len() < k {
            return Err(KcError::Key(format!(
                "custody has {} shares but needs {k} to recover",
                self.sealed_shares.len()
            )));
        }
        let mut indices: Vec<u8> = self.sealed_shares.iter().map(|s| s.index).collect();
        indices.sort_unstable();
        let count = indices.len();
        indices.dedup();
        if indices.len() != count {
            return Err(KcError::Key("custody has duplicate share indices".into()));
        }
        if indices.first() == Some(&0) {
            return Err(KcError::Key("share index 0 is the secret itself".into()));
        }
        Ok(())
    }

    /// A holder opens their own share.
    pub fn open_share(&self, holder_did: &DidKey, holder_priv: &PrivKey) -> KcResult<RevealedShare> {
        if holder_priv.alg != Alg::Ed25519 {
            return Err(KcError::UnsupportedAlg("custody requires Ed25519".into()));
        }
        let sealed = self
            .sealed_shares
            .iter()
            .find(|s| &s.holder_did == holder_did)
            .ok_or_else(|| KcError::Key(format!("no share sealed to {holder_did}")))?;
        // Say what is wrong. Without this the caller gets an AEAD failure, which reads
        // as a corrupt record rather than as "this share was sealed to a key you do not
        // hold" — and the difference between those two is the whole defect.
        let offered = crate::keys::public_of(holder_priv)?;
        if offered.bytes != sealed.recipient_pubkey.0 {
            return Err(KcError::Key(format!(
                "the share for {holder_did} was sealed to a different key than the one offered, \
                 so this holder cannot open it"
            )));
        }
        let ephemeral_pub = PubKey {
            alg: Alg::Ed25519,
            bytes: sealed.ephemeral_pub.0.to_vec(),
        };
        let seal_key = derive_key(
            DOMAIN_SEAL_KEY,
            &agree(&expanded_scalar(holder_priv)?, &ephemeral_pub)?,
        );
        let plain = aead_open(&seal_key, AAD_SHARE, &sealed.ciphertext, &sealed.nonce)?;
        let value: [u8; 32] = plain
            .try_into()
            .map_err(|_| KcError::Key("share is not 32 bytes".into()))?;
        Ok(RevealedShare { index: sealed.index, value: Bytes32(value) })
    }

    /// Check one revealed share against the public commitments: `f(i)·B` must equal
    /// `Σ C_j·i^j`. This is what makes the sharing verifiable rather than trusted.
    pub fn verify_share(&self, share: &RevealedShare) -> KcResult<()> {
        let value = share_scalar(share)?;
        let x = Scalar::from(u64::from(share.index));
        let mut expected = EdwardsPoint::default();
        let mut power = Scalar::ONE;
        for c in &self.commitments {
            let point = curve25519_dalek::edwards::CompressedEdwardsY(c.0)
                .decompress()
                .ok_or_else(|| KcError::Key("custody commitment is not a curve point".into()))?;
            expected += power * point;
            power *= x;
        }
        if EdwardsPoint::mul_base(&value) == expected {
            Ok(())
        } else {
            Err(KcError::Key(format!(
                "share {} does not match the published commitments",
                share.index
            )))
        }
    }

    /// Recover the protected payload from `shares`.
    ///
    /// Every share is verified against the commitments first, so a forged share is
    /// refused by name rather than silently producing garbage. Fewer than
    /// `threshold_k` shares is an error, not a partial answer.
    pub fn recover(&self, shares: &[RevealedShare]) -> KcResult<Vec<u8>> {
        self.check_shape()?;
        let k = usize::from(self.threshold_k);
        let mut seen: Vec<u8> = Vec::new();
        for s in shares {
            if seen.contains(&s.index) {
                return Err(KcError::Key(format!("share {} was offered twice", s.index)));
            }
            if !self.sealed_shares.iter().any(|sealed| sealed.index == s.index) {
                return Err(KcError::Key(format!(
                    "share {} is not one of this custody's shares",
                    s.index
                )));
            }
            self.verify_share(s)?;
            seen.push(s.index);
        }
        if seen.len() < k {
            return Err(KcError::Key(format!(
                "recovery needs {k} shares, got {}",
                seen.len()
            )));
        }
        // Interpolate with exactly k shares; extra valid shares are redundant.
        let used: Vec<&RevealedShare> = shares.iter().take(k).collect();
        let indices: Vec<u8> = used.iter().map(|s| s.index).collect();
        let mut secret = Scalar::ZERO;
        for s in &used {
            secret += lagrange_at_zero(s.index, &indices)? * share_scalar(s)?;
        }
        let record_key = derive_key(DOMAIN_RECORD_KEY, secret.as_bytes());
        aead_open(&record_key, AAD_RECORD, &self.ciphertext, &self.nonce)
    }

    /// Recover a [`SourcePreimage`] and confirm it opens `commitment`.
    ///
    /// This is the function that finally gives `source_commitment` a rule that reads
    /// it: a reveal must produce the preimage the filing committed to, and no other.
    pub fn recover_preimage(
        &self,
        shares: &[RevealedShare],
        commitment: &Cid,
    ) -> KcResult<SourcePreimage> {
        let plain = self.recover(shares)?;
        let preimage: SourcePreimage = crate::canonical::from_canonical_cbor(&plain)?;
        let actual = preimage.commitment()?;
        if &actual != commitment {
            return Err(KcError::CidMismatch {
                expected: commitment.to_string(),
                actual: actual.to_string(),
            });
        }
        Ok(preimage)
    }

    /// The DIDs the shares were sealed to.
    pub fn holders(&self) -> Vec<&DidKey> {
        self.sealed_shares.iter().map(|s| &s.holder_did).collect()
    }
}

fn share_scalar(share: &RevealedShare) -> KcResult<Scalar> {
    Option::from(Scalar::from_canonical_bytes(share.value.0))
        .ok_or_else(|| KcError::Key(format!("share {} is not a canonical scalar", share.index)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    fn holders(n: usize) -> (Vec<(DidKey, PubKey)>, Vec<PrivKey>) {
        let mut pairs = Vec::new();
        let mut privs = Vec::new();
        for _ in 0..n {
            let (sk, pk) = keys::generate(Alg::Ed25519).unwrap();
            let did = DidKey::from_pubkey(&pk).unwrap();
            pairs.push((did, pk));
            privs.push(sk);
        }
        (pairs, privs)
    }

    /// Just the DIDs, in share-index order. These holders are `did:key`s, so the
    /// `did:key` resolver returns each one's own key and behaviour is unchanged.
    fn dids(pairs: &[(DidKey, PubKey)]) -> Vec<DidKey> {
        pairs.iter().map(|(d, _)| d.clone()).collect()
    }

    fn open_all(
        custody: &ThresholdCustody,
        pairs: &[(DidKey, PubKey)],
        privs: &[PrivKey],
    ) -> Vec<RevealedShare> {
        pairs
            .iter()
            .zip(privs)
            .map(|((did, _), sk)| custody.open_share(did, sk).unwrap())
            .collect()
    }

    #[test]
    fn round_trips_at_exactly_the_threshold() {
        let (pairs, privs) = holders(5);
        let custody = ThresholdCustody::seal(b"the source", &dids(&pairs), didkey_encryption_resolver, 3).unwrap();
        custody.check_shape().unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        assert_eq!(custody.recover(&shares[..3]).unwrap(), b"the source");
    }

    #[test]
    fn every_k_subset_recovers_the_same_secret() {
        // Correctness of the interpolation, not just of one lucky subset.
        let (pairs, privs) = holders(5);
        let custody = ThresholdCustody::seal(b"threshold-held", &dids(&pairs), didkey_encryption_resolver, 3).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        let mut subsets = 0;
        for i in 0..5 {
            for j in (i + 1)..5 {
                for l in (j + 1)..5 {
                    let subset = [shares[i], shares[j], shares[l]];
                    assert_eq!(custody.recover(&subset).unwrap(), b"threshold-held");
                    subsets += 1;
                }
            }
        }
        assert_eq!(subsets, 10);
    }

    #[test]
    fn one_short_of_the_threshold_recovers_nothing() {
        let (pairs, privs) = holders(5);
        let custody = ThresholdCustody::seal(b"secret", &dids(&pairs), didkey_encryption_resolver, 4).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        let err = custody.recover(&shares[..3]).unwrap_err();
        assert!(matches!(err, KcError::Key(_)), "got {err}");
    }

    #[test]
    fn every_holder_can_open_only_its_own_share() {
        let (pairs, privs) = holders(4);
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        for (i, (did, _)) in pairs.iter().enumerate() {
            let share = custody.open_share(did, &privs[i]).unwrap();
            assert_eq!(share.index, (i + 1) as u8);
            custody.verify_share(&share).unwrap();
            // Another holder's key cannot open this holder's share.
            let other = (i + 1) % pairs.len();
            assert!(custody.open_share(did, &privs[other]).is_err());
        }
    }

    #[test]
    fn a_non_holder_has_no_share() {
        let (pairs, privs) = holders(3);
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let (outsider_sk, outsider_pk) = keys::generate(Alg::Ed25519).unwrap();
        let outsider_did = DidKey::from_pubkey(&outsider_pk).unwrap();
        assert!(custody.open_share(&outsider_did, &outsider_sk).is_err());
        assert!(custody.open_share(&pairs[0].0, &outsider_sk).is_err());
        let _ = privs;
    }

    // --- the recipient key: recorded, and the pairing enforced ------------------
    //
    // Measured before this existed: sealing three shares to the right holder DIDs but to
    // one outsider's key produced a custody where `check_shape` passed, the holder set
    // matched the panel, **0 of 3** named custodians could open a share, the substituted
    // key opened all three and recovered the secret alone, Feldman's own share check
    // passed on the wrongly-opened shares, and the recipient key appeared nowhere on the
    // record. The archive authorized it. These are the regressions.

    #[test]
    fn a_share_records_the_key_it_was_sealed_to() {
        let (pairs, _privs) = holders(3);
        let custody =
            ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        // On the record, so a mis-sealing is visible to any auditor rather than only to
        // the holder who tries and fails. This is the Feldman argument — checkable when
        // published, not at recovery — applied to the sealing half.
        for ((did, pk), share) in pairs.iter().zip(&custody.sealed_shares) {
            assert_eq!(&share.holder_did, did);
            assert_eq!(share.recipient_pubkey.0.to_vec(), pk.bytes);
        }
    }

    #[test]
    fn opening_a_share_sealed_to_another_key_names_the_mismatch() {
        // The holder is named correctly and the share is sealed to somebody else's key.
        let (pairs, privs) = holders(3);
        let (_att_sk, att_pk) = keys::generate(Alg::Ed25519).unwrap();
        let substitute = |_did: &DidKey| Ok(att_pk.clone());
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), substitute, 2).unwrap();

        let err = custody.open_share(&pairs[0].0, &privs[0]).unwrap_err();
        let msg = err.to_string();
        // Not an AEAD failure: an AEAD failure reads as a corrupt record, and the
        // difference between "corrupt" and "sealed to a key you do not hold" is the
        // whole defect.
        assert!(msg.contains("sealed to a different key"), "got {msg}");
        // And the substitution is now visible on the record.
        assert!(custody
            .sealed_shares
            .iter()
            .all(|s| s.recipient_pubkey.0.to_vec() == att_pk.bytes));
    }

    #[test]
    fn seal_refuses_a_holder_the_resolver_cannot_place() {
        let (pairs, _privs) = holders(2);
        let unplaceable = |did: &DidKey| -> KcResult<PubKey> {
            Err(KcError::Key(format!("{did} is not a registered custodian")))
        };
        let err = ThresholdCustody::seal(b"x", &dids(&pairs), unplaceable, 2).unwrap_err();
        assert!(err.to_string().contains("not a registered custodian"));
    }

    #[test]
    fn tampered_share_is_caught_by_the_commitments() {
        let (pairs, privs) = holders(3);
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let mut shares = open_all(&custody, &pairs, &privs);
        shares[0].value.0[0] ^= 0x01;
        assert!(custody.verify_share(&shares[0]).is_err());
        assert!(custody.recover(&shares[..2]).is_err());
    }

    #[test]
    fn a_share_from_another_custody_is_refused() {
        let (pairs, privs) = holders(3);
        let a = ThresholdCustody::seal(b"a", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let b = ThresholdCustody::seal(b"b", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let b_shares = open_all(&b, &pairs, &privs);
        assert!(a.verify_share(&b_shares[0]).is_err());
    }

    #[test]
    fn duplicate_share_does_not_count_twice() {
        // Otherwise one holder could reach the threshold alone.
        let (pairs, privs) = holders(4);
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 3).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        let doubled = [shares[0], shares[0], shares[1]];
        assert!(custody.recover(&doubled).is_err());
    }

    #[test]
    fn degenerate_thresholds_are_refused() {
        let (pairs, _privs) = holders(3);
        // A threshold of 1 is no quorum at all.
        assert!(ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 1).is_err());
        // A threshold above n could never be met.
        assert!(ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 4).is_err());
    }

    #[test]
    fn tampered_commitments_are_detected() {
        let (pairs, privs) = holders(3);
        let mut custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        custody.commitments[1].0[0] ^= 0x01;
        // Either the point no longer decompresses or the share check fails; both are
        // refusals, and neither silently recovers.
        assert!(custody.recover(&shares[..2]).is_err());
    }

    #[test]
    fn tampered_ciphertext_is_refused_by_the_aead() {
        let (pairs, privs) = holders(3);
        let mut custody = ThresholdCustody::seal(b"the source", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        let last = custody.ciphertext.len() - 1;
        custody.ciphertext[last] ^= 0x01;
        assert!(custody.recover(&shares[..2]).is_err());
    }

    #[test]
    fn shape_check_rejects_a_short_commitment_list() {
        let (pairs, _privs) = holders(4);
        let mut custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 3).unwrap();
        custody.commitments.pop();
        assert!(custody.check_shape().is_err());
    }

    #[test]
    fn preimage_recovery_checks_the_commitment() {
        let (pairs, privs) = holders(3);
        let preimage = SourcePreimage::new("did:key:zSource");
        let commitment = preimage.commitment().unwrap();
        let bytes = crate::canonical::to_canonical_cbor(&preimage).unwrap();
        let custody = ThresholdCustody::seal(&bytes, &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let shares = open_all(&custody, &pairs, &privs);
        let recovered = custody.recover_preimage(&shares[..2], &commitment).unwrap();
        assert_eq!(recovered, preimage);
        // A commitment to something else does not open this custody.
        let other = SourcePreimage::new("did:key:zOther").commitment().unwrap();
        assert!(matches!(
            custody.recover_preimage(&shares[..2], &other),
            Err(KcError::CidMismatch { .. })
        ));
    }

    #[test]
    fn commitment_is_nonce_bound() {
        // Two commitments to the same identity differ, so a commitment cannot be
        // recognized by re-hashing a guessed identity.
        let a = SourcePreimage::new("did:key:zSame");
        let b = SourcePreimage::new("did:key:zSame");
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.commitment().unwrap(), b.commitment().unwrap());
    }

    #[test]
    fn custody_roundtrips_through_canonical_cbor() {
        let (pairs, privs) = holders(3);
        let custody = ThresholdCustody::seal(b"wire", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let bytes = crate::canonical::to_canonical_cbor(&custody).unwrap();
        let decoded: ThresholdCustody = crate::canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(decoded, custody);
        let shares = open_all(&decoded, &pairs, &privs);
        assert_eq!(decoded.recover(&shares[..2]).unwrap(), b"wire");
    }

    #[test]
    fn holders_are_the_sealed_set() {
        let (pairs, _privs) = holders(3);
        let custody = ThresholdCustody::seal(b"x", &dids(&pairs), didkey_encryption_resolver, 2).unwrap();
        let got: Vec<&DidKey> = custody.holders();
        let want: Vec<&DidKey> = pairs.iter().map(|(d, _)| d).collect();
        assert_eq!(got, want);
    }
}
