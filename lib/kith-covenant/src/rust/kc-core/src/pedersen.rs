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

//! Committed values and proofs that a committed value is within a public bound.
//!
//! §7 asks for a **surveillance threshold**: "configurable disclosure levels for internal
//! health indicators", and for a community to be able to show its draw on a commons is
//! within its allowance without publishing the draw itself. A community that must publish
//! every figure to account honestly is disclosing its internal capacity, its consumption
//! patterns and its reserves to every other member, which is exactly what the threshold
//! exists to bound.
//!
//! Two pieces:
//!
//! - [`Commitment`], a Pedersen commitment `C = v·B + r·H`. Hiding, because `r` is
//!   uniform and `H` has unknown discrete log relative to `B`; binding, because opening
//!   it two ways would yield that discrete log.
//! - [`BoundProof`], which shows a committed `v` satisfies `0 <= v <= bound` for a public
//!   `bound`, and nothing else about `v`.
//!
//! ## Construction
//!
//! The bound proof is bit decomposition with an OR-proof per bit, the standard
//! pre-Bulletproofs construction. For a value in `[0, 2^N)`:
//!
//! 1. Commit to each bit: `C_i = b_i·B + r_i·H`, with the blindings chosen so that
//!    `Σ 2^i·C_i` is *exactly* `C`. The verifier checks that identity directly, so no
//!    separate proof of the linear relation is needed.
//! 2. Prove each `C_i` opens to 0 or 1, without saying which. Either `C_i = r_i·H` (the
//!    bit is 0) or `C_i - B = r_i·H` (the bit is 1), so this is a 1-of-2 proof of
//!    knowledge of a discrete log with respect to `H`: the honest branch is run for
//!    real, the other is simulated backwards from a chosen challenge, and Fiat-Shamir
//!    over both commitments fixes the split. This is the same Σ-protocol shape
//!    [`crate::DlEqProof`] already carries, one composition further.
//!
//! Then `v <= bound` is two range proofs: `v ∈ [0, 2^N)` and `bound - v ∈ [0, 2^N)`. The
//! second needs no new commitment, because `(bound - v)·B + (-r)·H` is exactly
//! `bound·B - C`, which the verifier computes itself. Both being in range forces
//! `v <= bound`: the two values sum to `bound` and each is below `2^N`, and `2^(N+1)` is
//! vastly below the group order, so neither can have wrapped.
//!
//! ## What this is not
//!
//! It is not succinct. A proof is [`RANGE_BITS`] OR-proofs per side and runs to about
//! **15 KB**, with verification linear in the bit width. Five 32-byte scalars and points
//! per bit, times forty bits, times two sides. That is affordable for periodic usage
//! accounting on a replicated archive and would not be for anything high-volume, and the
//! figure is asserted in the tests so it cannot drift unnoticed.
//!
//! It is also **more hand-rolled cryptography**, on a tree that already carries a ring
//! signature, Feldman sharing, a key agreement and a Chaum-Pedersen proof, none of it
//! externally reviewed (`TODO.md`, "An external review of the hand-rolled crypto"). The
//! construction here is textbook and reuses this crate's existing generator and hashing
//! rather than introducing new machinery, but it widens the surface that review has to
//! cover, and the tracker says so.

use curve25519_dalek::edwards::{CompressedEdwardsY, EdwardsPoint};
use curve25519_dalek::scalar::Scalar;
use serde::{Deserialize, Serialize};

use crate::error::{KcError, KcResult};
use crate::ring::{h_point, hash_to_scalar, random_scalar, Bytes32};

const DOMAIN_BIT_CHALLENGE: &[u8] = b"kc:pedersen:bit-challenge:v1";

/// The width of a range proof, in bits: values up to `2^40 - 1`, about 1.1e12.
///
/// Wide enough for commons accounting in any of the units the five commons are measured
/// in (bandwidth-hours, rescue-standby-hours, archive-bytes) with several orders of
/// headroom, and narrow enough that a proof stays around 15 KB. A bound at or above
/// `2^40` is refused rather than silently proven against a wrapped value.
pub const RANGE_BITS: usize = 40;

/// A Pedersen commitment `C = v·B + r·H` to a `u64` value.
///
/// Hiding and binding, and *homomorphic*: the sum of two commitments commits to the sum
/// of their values. The archive stores only this; the value and its blinding stay with
/// whoever made it, or go under threshold custody so a quorum can recover them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment(pub Bytes32);

impl Commitment {
    /// Commit to `value` under `blinding`.
    pub fn new(value: u64, blinding: &Blinding) -> Self {
        let point = EdwardsPoint::mul_base(&Scalar::from(value)) + blinding.0 * h_point();
        Self(Bytes32(point.compress().to_bytes()))
    }

    /// Does this commitment open to `value` under `blinding`?
    ///
    /// What an opened usage record is checked against: a quorum that recovers the
    /// figures must recover the ones actually committed to.
    pub fn opens_to(&self, value: u64, blinding: &Blinding) -> bool {
        Self::new(value, blinding) == *self
    }

    fn point(&self) -> KcResult<EdwardsPoint> {
        decompress(&self.0, "commitment")
    }
}

/// The blinding factor of a [`Commitment`]: a uniform scalar, without which the
/// commitment reveals the value to anyone who can guess it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blinding(#[serde(with = "scalar_serde")] Scalar);

impl Blinding {
    /// A fresh blinding from the OS CSPRNG.
    pub fn random() -> Self {
        Self(random_scalar())
    }

    /// The 32-byte canonical encoding, for sealing under threshold custody alongside the
    /// value it blinds.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Recover a blinding from its encoding. Refuses a non-canonical scalar.
    pub fn from_bytes(b: &[u8; 32]) -> KcResult<Self> {
        Option::<Scalar>::from(Scalar::from_canonical_bytes(*b))
            .map(Self)
            .ok_or_else(|| KcError::Key("blinding is not a canonical scalar".into()))
    }
}

mod scalar_serde {
    use curve25519_dalek::scalar::Scalar;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Scalar, s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(&v.to_bytes()).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Scalar, D::Error> {
        let buf: serde_bytes::ByteBuf = Deserialize::deserialize(d)?;
        let arr: [u8; 32] = buf
            .into_vec()
            .try_into()
            .map_err(|_| serde::de::Error::custom("scalar must be 32 bytes"))?;
        Option::<Scalar>::from(Scalar::from_canonical_bytes(arr))
            .ok_or_else(|| serde::de::Error::custom("scalar is not canonical"))
    }
}

/// One bit's commitment and its 0-or-1 proof.
///
/// **Both challenges are published**, and the verifier's check is that they sum to the
/// Fiat-Shamir hash. Publishing only one and deriving the other does not work: the hash
/// is taken over the nonce commitments, which the verifier can only reconstruct from the
/// challenges, so deriving `c1` from the hash would require the hash before it could be
/// computed. Carrying both costs 32 bytes a bit and makes the check acyclic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitProof {
    /// `C_i`, the commitment to this bit.
    pub commitment: Bytes32,
    #[serde(with = "scalar_serde")]
    c0: Scalar,
    #[serde(with = "scalar_serde")]
    c1: Scalar,
    #[serde(with = "scalar_serde")]
    s0: Scalar,
    #[serde(with = "scalar_serde")]
    s1: Scalar,
}

/// A proof that a committed value lies in `[0, 2^RANGE_BITS)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RangeProof {
    pub bits: Vec<BitProof>,
}

/// A proof that a committed value `v` satisfies `0 <= v <= bound`, revealing nothing
/// else about `v`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundProof {
    /// `v ∈ [0, 2^RANGE_BITS)`.
    pub low: RangeProof,
    /// `bound - v ∈ [0, 2^RANGE_BITS)`, over the commitment `bound·B - C` that the
    /// verifier computes for itself.
    pub high: RangeProof,
}

impl BoundProof {
    /// Prove that `value`, committed as `commitment` under `blinding`, is at most
    /// `bound`.
    ///
    /// Errors rather than proving anything if the value exceeds the bound, if the bound
    /// is too large for the fixed width, or if the commitment does not actually open to
    /// the value given: a caller that has lost track of its own opening should find out
    /// here rather than publish a proof that cannot verify.
    pub fn prove(
        value: u64,
        blinding: &Blinding,
        commitment: &Commitment,
        bound: u64,
    ) -> KcResult<Self> {
        if bound >= (1u64 << RANGE_BITS) {
            return Err(KcError::Key(format!(
                "a bound of {bound} is at or above the {RANGE_BITS}-bit proof width"
            )));
        }
        if value > bound {
            return Err(KcError::Key(
                "cannot prove a value is within a bound it exceeds".into(),
            ));
        }
        if !commitment.opens_to(value, blinding) {
            return Err(KcError::Key(
                "the commitment does not open to the value offered for proof".into(),
            ));
        }
        let low = RangeProof::prove(value, &blinding.0)?;
        // `bound·B - C` commits to `bound - value` under blinding `-r`, so the high side
        // needs no commitment of its own and the verifier reconstructs it.
        let high = RangeProof::prove(bound - value, &(-blinding.0))?;
        Ok(Self { low, high })
    }

    /// Verify against the published commitment and the public bound.
    pub fn verify(&self, commitment: &Commitment, bound: u64) -> KcResult<()> {
        if bound >= (1u64 << RANGE_BITS) {
            return Err(KcError::Key(format!(
                "a bound of {bound} is at or above the {RANGE_BITS}-bit proof width"
            )));
        }
        let c = commitment.point()?;
        self.low.verify(&c)?;
        let complement = EdwardsPoint::mul_base(&Scalar::from(bound)) - c;
        self.high.verify(&complement)?;
        Ok(())
    }
}

impl RangeProof {
    fn prove(value: u64, blinding: &Scalar) -> KcResult<Self> {
        if RANGE_BITS < 64 && value >= (1u64 << RANGE_BITS) {
            return Err(KcError::Key(format!(
                "value does not fit in {RANGE_BITS} bits"
            )));
        }
        // Blindings for all but the top bit are uniform; the top one is solved for, so
        // that `Σ 2^i·r_i` is exactly the commitment's own blinding and the verifier's
        // recombination check needs no extra proof.
        let mut blindings: Vec<Scalar> = (0..RANGE_BITS - 1).map(|_| random_scalar()).collect();
        let two = Scalar::from(2u64);
        let mut partial = Scalar::ZERO;
        let mut power = Scalar::ONE;
        for r in &blindings {
            partial += power * r;
            power *= two;
        }
        // `power` is now 2^(RANGE_BITS-1), the weight of the bit being solved for.
        let top_inv = Option::<Scalar>::from(power.invert())
            .ok_or_else(|| KcError::Key("2^(N-1) is not invertible".into()))?;
        blindings.push((blinding - partial) * top_inv);

        let mut bits = Vec::with_capacity(RANGE_BITS);
        for (i, r) in blindings.iter().enumerate() {
            let bit = (value >> i) & 1 == 1;
            bits.push(BitProof::prove(bit, r)?);
        }
        Ok(Self { bits })
    }

    fn verify(&self, commitment: &EdwardsPoint) -> KcResult<()> {
        if self.bits.len() != RANGE_BITS {
            return Err(KcError::Key(format!(
                "a range proof carries {RANGE_BITS} bit proofs, got {}",
                self.bits.len()
            )));
        }
        let two = Scalar::from(2u64);
        let mut power = Scalar::ONE;
        let mut recombined = EdwardsPoint::default();
        for b in &self.bits {
            recombined += power * b.verify()?;
            power *= two;
        }
        if &recombined != commitment {
            return Err(KcError::Key(
                "the bit commitments do not recombine to the committed value".into(),
            ));
        }
        Ok(())
    }
}

impl BitProof {
    /// The 1-of-2 proof. The honest branch runs forwards; the other is simulated
    /// backwards from a freely chosen challenge, and only a real opening can make the
    /// two challenges sum to the Fiat-Shamir hash over both nonce commitments.
    fn prove(bit: bool, blinding: &Scalar) -> KcResult<Self> {
        let h = *h_point();
        let b = EdwardsPoint::mul_base(&Scalar::ONE);
        let commitment = if bit { b + blinding * h } else { blinding * h };
        // P_0 = C (opens to 0), P_1 = C - B (opens to 1). The prover knows the discrete
        // log of P_bit with respect to H, and of the other, nothing.
        let p0 = commitment;
        let p1 = commitment - b;

        let k = random_scalar();
        let c_sim = random_scalar();
        let s_sim = random_scalar();
        if bit {
            // Branch 0 is simulated, branch 1 is real.
            let t0 = s_sim * h - c_sim * p0;
            let t1 = k * h;
            let c = challenge(&commitment, &t0, &t1);
            let c1 = c - c_sim;
            Ok(Self {
                commitment: Bytes32(commitment.compress().to_bytes()),
                c0: c_sim,
                c1,
                s0: s_sim,
                s1: k + c1 * blinding,
            })
        } else {
            // Branch 1 is simulated, branch 0 is real.
            let t0 = k * h;
            let t1 = s_sim * h - c_sim * p1;
            let c = challenge(&commitment, &t0, &t1);
            let c0 = c - c_sim;
            Ok(Self {
                commitment: Bytes32(commitment.compress().to_bytes()),
                c0,
                c1: c_sim,
                s0: k + c0 * blinding,
                s1: s_sim,
            })
        }
    }

    /// Returns the verified commitment point, so the caller recombines the same point
    /// the proof was checked against rather than decompressing it a second time.
    fn verify(&self) -> KcResult<EdwardsPoint> {
        let h = *h_point();
        let commitment = decompress(&self.commitment, "bit commitment")?;
        let p0 = commitment;
        let p1 = commitment - EdwardsPoint::mul_base(&Scalar::ONE);
        let t0 = self.s0 * h - self.c0 * p0;
        let t1 = self.s1 * h - self.c1 * p1;
        if self.c0 + self.c1 != challenge(&commitment, &t0, &t1) {
            return Err(KcError::Key(
                "bit proof does not open to 0 or 1: the challenge split does not sum".into(),
            ));
        }
        Ok(commitment)
    }
}

fn challenge(commitment: &EdwardsPoint, t0: &EdwardsPoint, t1: &EdwardsPoint) -> Scalar {
    hash_to_scalar(
        DOMAIN_BIT_CHALLENGE,
        &[
            &commitment.compress().to_bytes(),
            &t0.compress().to_bytes(),
            &t1.compress().to_bytes(),
        ],
    )
}

/// Decompress a point from untrusted input, refusing anything off the prime-order
/// subgroup.
///
/// The torsion check matters here in a way it does not for a public key: a bit
/// commitment carrying a low-order component would still recombine and still pass the
/// per-bit proof, while committing to a value the proof does not constrain.
fn decompress(b: &Bytes32, what: &str) -> KcResult<EdwardsPoint> {
    let p = CompressedEdwardsY(b.0)
        .decompress()
        .ok_or_else(|| KcError::Key(format!("{what} is not a curve point")))?;
    if !p.is_torsion_free() {
        return Err(KcError::Key(format!(
            "{what} is not in the prime-order subgroup"
        )));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical;

    fn commit(v: u64) -> (Commitment, Blinding) {
        let r = Blinding::random();
        (Commitment::new(v, &r), r)
    }

    #[test]
    fn a_commitment_opens_only_to_what_it_committed() {
        let (c, r) = commit(4242);
        assert!(c.opens_to(4242, &r));
        assert!(!c.opens_to(4243, &r));
        assert!(!c.opens_to(4242, &Blinding::random()));
    }

    #[test]
    fn the_same_value_commits_differently_every_time() {
        // Hiding: without this, a commitment to a known small value would be a lookup.
        let (a, _) = commit(7);
        let (b, _) = commit(7);
        assert_ne!(a, b);
    }

    #[test]
    fn a_value_within_its_bound_proves_and_verifies() {
        for (v, bound) in [(0u64, 0u64), (0, 100), (50, 100), (100, 100), (1, 1)] {
            let (c, r) = commit(v);
            let p = BoundProof::prove(v, &r, &c, bound).expect("prove");
            p.verify(&c, bound).unwrap_or_else(|e| panic!("{v} <= {bound}: {e}"));
        }
    }

    #[test]
    fn a_large_value_near_the_width_limit_still_proves() {
        let bound = (1u64 << RANGE_BITS) - 1;
        let (c, r) = commit(bound - 1);
        let p = BoundProof::prove(bound - 1, &r, &c, bound).expect("prove");
        p.verify(&c, bound).expect("verify");
    }

    /// NEGATIVE CONTROL, and the property the whole construction exists for.
    #[test]
    fn a_value_over_its_bound_cannot_be_proven() {
        let (c, r) = commit(101);
        let err = BoundProof::prove(101, &r, &c, 100).expect_err("101 is not within 100");
        assert!(err.to_string().contains("exceeds"), "got: {err}");
    }

    /// The prover cannot get around the refusal above by proving a smaller value and
    /// attaching the proof to the real commitment: the recombination check ties the
    /// proof to one commitment and no other.
    #[test]
    fn a_proof_does_not_transfer_to_another_commitment() {
        let (honest, r) = commit(50);
        let proof = BoundProof::prove(50, &r, &honest, 100).expect("prove");
        let (other, _) = commit(50);
        assert!(proof.verify(&other, 100).is_err(), "a proof must not transfer");
        // Even the same value with a different blinding is a different commitment.
        assert_ne!(honest, other);
    }

    /// A proof is bound to the bound it was made for. Verifying against a *larger*
    /// bound must fail too: the high side proves `bound - v` for one specific bound.
    #[test]
    fn a_proof_does_not_transfer_to_another_bound() {
        let (c, r) = commit(50);
        let proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        assert!(proof.verify(&c, 99).is_err(), "a tighter bound must not verify");
        assert!(proof.verify(&c, 101).is_err(), "a looser bound must not verify");
        proof.verify(&c, 100).expect("its own bound verifies");
    }

    /// Tampering with any single bit commitment breaks the recombination.
    #[test]
    fn a_tampered_bit_commitment_is_caught() {
        let (c, r) = commit(50);
        let mut proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        let (decoy, _) = commit(1);
        proof.low.bits[3].commitment = decoy.0;
        assert!(proof.verify(&c, 100).is_err(), "a substituted bit must be caught");
    }

    /// Tampering with a response breaks the challenge split, which is the per-bit
    /// soundness check rather than the recombination one.
    #[test]
    fn a_tampered_response_is_caught() {
        let (c, r) = commit(50);
        let mut proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        proof.low.bits[0].s0 += Scalar::ONE;
        let err = proof.verify(&c, 100).expect_err("a forged response must be caught");
        assert!(err.to_string().contains("does not sum"), "got: {err}");
    }

    /// A proof must carry exactly the fixed width. A short one cannot slip through by
    /// recombining over fewer bits.
    #[test]
    fn a_truncated_proof_is_refused() {
        let (c, r) = commit(50);
        let mut proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        proof.low.bits.truncate(RANGE_BITS - 1);
        let err = proof.verify(&c, 100).expect_err("a short proof must be refused");
        assert!(err.to_string().contains("bit proofs"), "got: {err}");
    }

    /// A commitment whose declared opening is wrong is caught at proving time, so a
    /// caller that has lost its blinding finds out before publishing.
    #[test]
    fn proving_against_a_mismatched_opening_is_refused() {
        let (c, _) = commit(50);
        let err = BoundProof::prove(50, &Blinding::random(), &c, 100)
            .expect_err("a wrong blinding must be caught");
        assert!(err.to_string().contains("does not open"), "got: {err}");
    }

    /// The fixed width is a real limit and is refused rather than wrapped around, which
    /// is what would break soundness if a bound near the group order were allowed.
    #[test]
    fn a_bound_beyond_the_proof_width_is_refused() {
        let (c, r) = commit(1);
        let too_big = 1u64 << RANGE_BITS;
        assert!(BoundProof::prove(1, &r, &c, too_big).is_err());
        let honest = BoundProof::prove(1, &r, &c, 100).expect("prove");
        assert!(honest.verify(&c, too_big).is_err(), "verification refuses it too");
    }

    /// A point off the prime-order subgroup is refused. A low-order component would
    /// recombine and pass the per-bit proof while committing to something the proof does
    /// not constrain.
    #[test]
    fn a_bit_commitment_off_the_subgroup_is_refused() {
        use curve25519_dalek::constants::EIGHT_TORSION;
        let (c, r) = commit(50);
        let mut proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        let tainted = decompress(&proof.low.bits[0].commitment, "t").unwrap() + EIGHT_TORSION[1];
        proof.low.bits[0].commitment = Bytes32(tainted.compress().to_bytes());
        let err = proof.verify(&c, 100).expect_err("torsion must be refused");
        assert!(err.to_string().contains("prime-order subgroup"), "got: {err}");
    }

    #[test]
    fn commitment_and_proof_roundtrip_through_cbor() {
        let (c, r) = commit(50);
        let proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        let bytes = canonical::to_canonical_cbor(&proof).unwrap();
        let back: BoundProof = canonical::from_canonical_cbor(&bytes).unwrap();
        assert_eq!(proof, back);
        back.verify(&c, 100).expect("a decoded proof still verifies");

        let cb = canonical::to_canonical_cbor(&c).unwrap();
        let c_back: Commitment = canonical::from_canonical_cbor(&cb).unwrap();
        assert_eq!(c, c_back);
        // The blinding travels separately, under custody, and roundtrips on its own.
        let rb = canonical::to_canonical_cbor(&r).unwrap();
        let r_back: Blinding = canonical::from_canonical_cbor(&rb).unwrap();
        assert!(c_back.opens_to(50, &r_back));
    }

    /// The size the archive pays per sealed record, asserted so it cannot drift
    /// unnoticed into something a compact would not want to replicate.
    ///
    /// Five scalars-or-points per bit (the bit commitment, both challenges, both
    /// responses), forty bits, two sides: about 15 KB. Written as ~10 KB first, which is
    /// what the proof would cost if a single challenge could be published, and this
    /// assertion is what caught the estimate after the OR-proof was corrected to carry
    /// both.
    #[test]
    fn a_bound_proof_costs_about_fifteen_kilobytes() {
        let (c, r) = commit(50);
        let proof = BoundProof::prove(50, &r, &c, 100).expect("prove");
        let size = canonical::to_canonical_cbor(&proof).unwrap().len();
        assert!(
            (13_000..18_000).contains(&size),
            "a bound proof should be about 15 KB, got {size}"
        );
    }
}
