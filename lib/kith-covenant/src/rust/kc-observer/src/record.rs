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

//! The §8 observer / jester payloads.

use kc_core::{
    Cid, DidKey, DlEqProof, KcResult, KeyImage, PrivKey, PubKey, RevealedShare, RingSigned,
    ThresholdCustody, Timestamp,
};
use serde::{Deserialize, Serialize};

/// One nominated observer: a key a community vouches as fit to travel, observe,
/// and report, valid within a window, carrying the **declared perspective** it
/// reports from (kept explicit so downstream readers discount partiality rather
/// than treat a report as neutral). The nominating community is the enclosing
/// [`ObserverRegistry`], so the non-party rule (an observer may not report on its
/// own community) can always determine an observer's home.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverEntry {
    pub observer_did: DidKey,
    pub observer_pubkey: PubKey,
    pub declared_perspective: String,
    pub valid_from: Timestamp,
    pub valid_until: Option<Timestamp>,
}

impl ObserverEntry {
    /// Is this observer nomination live at instant `t`?
    pub fn active_at(&self, t: Timestamp) -> bool {
        self.valid_from <= t && self.valid_until.is_none_or(|end| t < end)
    }
}

/// A community's roster of nominated observers. Signed by the community root.
/// Registries for the same `nominating_community` supersede by latest
/// `issued_at`; the compact-wide corps is the union across all communities
/// (mirrors the mediator/witness registries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverRegistry {
    pub nominating_community: DidKey,
    pub entries: Vec<ObserverEntry>,
    pub issued_at: Timestamp,
}

/// An attributed public report on a community's conduct, signed by an active
/// registered observer whose home community is **not** the subject
/// (cross-community journalism, non-overlap). Publication is irrevocable; the
/// subject answers with an appended [`ReportResponse`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObserverReport {
    /// The community observed.
    pub subject_community: DidKey,
    /// The reporting observer (resolves via the observer registry).
    pub observer_did: DidKey,
    /// The observer's declared standpoint, restated on the report for readers who
    /// see the report without the registry to hand.
    pub declared_perspective: String,
    pub summary: String,
    /// Prior archive records this report draws on (conduct, usage, prior reports).
    pub references: Vec<Cid>,
    pub observed_at: Timestamp,
    pub reported_at: Timestamp,
}

/// A subject community's public response to an [`ObserverReport`] (referenced by
/// CID), signed by and attributed to that community. The response is appended to
/// the record; it never suppresses the report (publication is irrevocable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportResponse {
    pub report_cid: Cid,
    pub responding_community: DidKey,
    pub response: String,
    pub responded_at: Timestamp,
}

/// What a source ring-signs: the assertion "I am an insider of this community, and
/// this filing is mine."
///
/// Every field is bound into the attestation for a reason. `accused_community` and
/// `filed_at` pin it to one filing against one community. `claim_digest` pins it to
/// the **accusation text**, so a reception panel cannot lift a source's attestation
/// off the claim they made and staple it to one they did not. `source_commitment`
/// ties the anonymous attestation to the identity held in threshold custody, so the
/// eventual reveal is a reveal *of this source*.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsiderClaim {
    pub accused_community: DidKey,
    pub source_commitment: Cid,
    /// `Cid::of(&claim)` for the report's claim text.
    pub claim_digest: Cid,
    pub filed_at: Timestamp,
}

/// An insider complaint that a community is violating the common virtues, filed
/// across the AT/kith-covenant boundary. Co-signed by a mediator **reception
/// panel** that vouches it received a credible insider report.
///
/// The source is protected twice over, by construction rather than by promise:
///
/// - `insider` is a **ring signature** over the accused community's own resolvable
///   keys. It proves an insider filed this — without which the archive holds no
///   evidence a source exists at all, and a panel could manufacture complaints
///   against a rival — while saying nothing about which insider.
/// - `custody` holds the source's identity under **k-of-n threshold encryption**
///   keyed to the reception panel. A minority of the panel cannot reconstruct it
///   early, which is what §8's "structurally stronger than individual mediator
///   integrity" requires; and the commitment is what a [`SourceReveal`] must open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhistleblowerReport {
    pub accused_community: DidKey,
    /// A commitment to the source's identity: `Cid::of(&SourcePreimage)`. The archive
    /// never holds the preimage in the clear — `custody` holds it encrypted, and a
    /// [`SourceReveal`] must produce a preimage that opens *this* commitment.
    pub source_commitment: Cid,
    pub claim: String,
    /// Prior archive records the claim cites, if any.
    pub references: Vec<Cid>,
    /// The mediator reception panel that received and vouched the report (each
    /// co-signs). The authority pass requires it be non-party active mediators.
    pub reception_panel: Vec<DidKey>,
    pub filed_at: Timestamp,
    /// The source's own anonymous attestation that it is an insider of the accused
    /// community. The ring is the community's resolvable keys at `filed_at`.
    pub insider: RingSigned<InsiderClaim>,
    /// The source identity, sealed so that a quorum of `reception_panel` — and only a
    /// quorum — can recover it.
    pub custody: ThresholdCustody,
}

impl WhistleblowerReport {
    /// The digest a filing's [`InsiderClaim`] must carry for `claim`.
    pub fn claim_digest(claim: &str) -> KcResult<Cid> {
        Cid::of(&claim.to_string())
    }

    /// The source's step: ring-sign an [`InsiderClaim`] as member `signer_index` of
    /// `ring`. Produced by the source alone, so the panel never holds the key that
    /// made it.
    pub fn attest(
        accused_community: DidKey,
        source_commitment: Cid,
        claim: &str,
        filed_at: Timestamp,
        ring: Vec<PubKey>,
        signer_index: usize,
        signer_priv: &PrivKey,
    ) -> KcResult<RingSigned<InsiderClaim>> {
        let payload = InsiderClaim {
            accused_community,
            source_commitment,
            claim_digest: Self::claim_digest(claim)?,
            filed_at,
        };
        RingSigned::build(payload, ring, signer_index, signer_priv)
    }

    /// The source's anonymous handle for this filing: the ring signature's key image.
    /// Equal across every filing by the same insider, and tied to no identity.
    pub fn source_tag(&self) -> &KeyImage {
        &self.insider.signature.key_image
    }
}

/// A disclosure of a whistleblower's source, gated on a mediator-panel quorum
/// citing named cause (jester_and_dissent.md "Procedural safeguards": the source
/// is protected unless a panel, for cause, reveals it). References the
/// [`WhistleblowerReport`] by CID.
///
/// The quorum is no longer only a rule about who signs the record: `shares` must
/// carry enough opened custody shares to actually recover the identity, and what they
/// recover must open the filing's commitment. A panel cannot name whoever it likes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReveal {
    pub report_cid: Cid,
    /// The mediator panel authorizing the reveal (each co-signs).
    pub panel: Vec<DidKey>,
    /// The named cause for the reveal (must be non-empty).
    pub cause: String,
    /// The disclosed source identity (the commitment preimage's identity), revealed
    /// only now. Kept in the clear so the archive is readable without doing crypto;
    /// the authority pass requires it match what `shares` actually recover.
    pub disclosed_source: String,
    /// Custody shares opened by their holders, at least `threshold_k` of them.
    pub shares: Vec<RevealedShare>,
    pub revealed_at: Timestamp,
}

/// A member's proof that a whistleblower filing was **not** theirs.
///
/// Threshold custody bounds who can be named to the filing's own ring, but a
/// colluding quorum could still name the wrong ring member. Key images are
/// deterministic in the signer's key, so the named member computes their own,
/// publishes it with a proof that it is genuinely theirs, and anyone can see it
/// differs from the image on the filing. The member clears themselves without
/// surrendering their key and without needing anybody's permission — the one remedy
/// here that does not route through a panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExculpation {
    /// The [`WhistleblowerReport`] whose authorship is denied.
    pub report_cid: Cid,
    /// The member denying it (signs the record).
    pub claimant_did: DidKey,
    /// The claimant's own key image.
    pub claimed_key_image: KeyImage,
    /// Proof that `claimed_key_image` belongs to the claimant's key.
    pub proof: DlEqProof,
    pub exculpated_at: Timestamp,
}

/// The challenge context for an exculpation proof, so a proof made for one filing
/// cannot be replayed against another.
pub fn exculpation_context(report_cid: &Cid) -> Vec<u8> {
    let mut ctx = b"kc:observer:exculpation:v1".to_vec();
    ctx.extend_from_slice(report_cid.as_bytes());
    ctx
}

impl SourceExculpation {
    /// Build an exculpation for `report_cid` from the claimant's own key.
    pub fn prove(
        report_cid: Cid,
        claimant_did: DidKey,
        claimant_priv: &PrivKey,
        exculpated_at: Timestamp,
    ) -> KcResult<Self> {
        let (claimed_key_image, proof) =
            DlEqProof::prove(claimant_priv, &exculpation_context(&report_cid))?;
        Ok(Self { report_cid, claimant_did, claimed_key_image, proof, exculpated_at })
    }

    /// Check the proof binds `claimant_pubkey` to the claimed key image.
    pub fn verify_proof(&self, claimant_pubkey: &PubKey) -> KcResult<()> {
        self.proof.verify(
            claimant_pubkey,
            &self.claimed_key_image,
            &exculpation_context(&self.report_cid),
        )
    }
}
