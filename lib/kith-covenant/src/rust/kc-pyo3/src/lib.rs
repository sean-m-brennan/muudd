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

//! PyO3 bindings — high-level façade over the kith-covenant Rust stack.
//!
//! Exposed surface is deliberately narrow for MVP:
//! - Key primitives: `generate_ed25519_keypair`, `cid_of_bytes`.
//! - Archive class: `PyArchive` with `verify_all`, `dump_json`, `load_json`,
//!   `__len__`, `cids`, `entries`.
//! - One-shot ceremony runner: `run_two_community_ceremony` returning a
//!   `PyCeremonyResult` carrying DIDs, CIDs, and the full JSON archive.
//!
//! Fine-grained per-type wrappers (PyCommunityIdentity, PyRecitation, etc.)
//! are intentionally omitted — `PyArchive.dump_json` gives Python full
//! inspection via JSON without requiring one `#[pyclass]` per Rust struct.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};

use jiff::ToSpan;
use kc_archive::{verify_all, Archive, ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::{
    AntiSpiralFeature, InterCredentialingOutcome, InterCredentialingRunner, NonAdmissionRecord,
    ParticipantContext, RejectionDetail, SponsorshipRecord, REQUIRED_FEATURES,
};
use kc_conduct::{
    ConductBearing, ConductKind, ConductRecord, DecisionClass, LadderRung, Severity,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, PubKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
};
use kc_mediation::{
    DecisionOutcome, DisputeRecord, MediationDecision, MediatorEntry, MediatorRegistry,
};
use kc_escalation::{
    DeliberationCycle, Designation, DesignationTier, DesignationUrgency, LadderTransition, ReEntry,
    ReEntryWitnessAttestation, RestitutionCompletion, RestitutionObligation,
    SubstantiveReEngagement,
};

fn kc_err_to_py(e: KcError) -> PyErr {
    PyRuntimeError::new_err(format!("kc: {e}"))
}

#[pyfunction]
fn generate_ed25519_keypair(
    py: Python<'_>,
) -> PyResult<(Bound<'_, PyBytes>, Bound<'_, PyBytes>, String)> {
    let (sk, pk) = keys::generate(Alg::Ed25519).map_err(kc_err_to_py)?;
    let did = DidKey::from_pubkey(&pk).map_err(kc_err_to_py)?;
    Ok((
        PyBytes::new_bound(py, &sk.bytes),
        PyBytes::new_bound(py, &pk.bytes),
        did.into_string(),
    ))
}

#[pyfunction]
fn cid_of_bytes(bytes: Vec<u8>) -> String {
    Cid::of_bytes(&bytes).to_string()
}

/// Current time as whole seconds since the Unix epoch, matching the archive's
/// `Timestamp` resolution. Convenience for authoring record timestamps.
#[pyfunction]
fn now_seconds() -> i64 {
    time::now().as_second()
}

#[pyclass]
struct PyArchive {
    inner: InMemoryArchive,
}

#[pymethods]
impl PyArchive {
    #[new]
    fn new() -> Self {
        Self {
            inner: InMemoryArchive::new(),
        }
    }

    #[classmethod]
    fn load_json(_cls: &Bound<'_, PyType>, s: &str) -> PyResult<Self> {
        InMemoryArchive::load_json_from_str(s)
            .map(|inner| Self { inner })
            .map_err(kc_err_to_py)
    }

    fn dump_json(&self) -> PyResult<String> {
        self.inner.dump_json_to_string().map_err(kc_err_to_py)
    }

    fn verify_all(&self) -> PyResult<()> {
        verify_all(&self.inner).map_err(kc_err_to_py)
    }

    /// Enforce §5 authority: each principal a record speaks for must carry a
    /// signature from its own root or an active, scope-appropriate delegate, and
    /// second-class decisions need >= `quorum_threshold` distinct communities.
    /// A separate pass from `verify_all` (crypto); raises on the first
    /// violation. Assumes signatures already verify.
    #[pyo3(signature = (quorum_threshold=2, panel_min=3))]
    fn check_authority(&self, quorum_threshold: usize, panel_min: usize) -> PyResult<()> {
        kc_archive::check_authority(&self.inner, quorum_threshold, panel_min).map_err(kc_err_to_py)
    }

    /// Every authority violation as `(cid, reason)`, instead of raising.
    #[pyo3(signature = (quorum_threshold=2, panel_min=3))]
    fn authority_violations(
        &self,
        quorum_threshold: usize,
        panel_min: usize,
    ) -> Vec<(String, String)> {
        kc_archive::authority_violations(&self.inner, quorum_threshold, panel_min)
    }

    // ---- §4 Mediation authoring ------------------------------------------

    /// Publish a community's mediator roster (§4). `mediators` is a list of
    /// `(pubkey_bytes, did)` the community nominates; they become active over
    /// `[valid_from, valid_until)`. Signed by the community root. Returns the CID.
    #[pyo3(signature = (community_priv, community_did, mediators, valid_from, valid_until=None))]
    fn register_mediators(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        mediators: Vec<(Vec<u8>, String)>,
        valid_from: i64,
        valid_until: Option<i64>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let vf = ts(valid_from)?;
        let vu = valid_until.map(ts).transpose()?;
        let mut entries = Vec::with_capacity(mediators.len());
        for (pk, did) in &mediators {
            entries.push(MediatorEntry {
                mediator_did: parse_did(did)?,
                mediator_pubkey: pub_from_bytes(pk),
                valid_from: vf,
                valid_until: vu,
            });
        }
        let reg = MediatorRegistry { nominating_community: cdid, entries, issued_at: vf };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(reg, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::MediatorRegistryUpdated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// File a dispute (§4), signed by the complainant. `rung` is a LadderRung
    /// name. Returns the dispute's entry CID (used to reference it in a decision).
    #[pyo3(signature = (complainant_priv, complainant_did, respondent_did, rung, summary=String::new(), references=Vec::new(), filed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn file_dispute(
        &mut self,
        complainant_priv: &[u8],
        complainant_did: &str,
        respondent_did: &str,
        rung: &str,
        summary: String,
        references: Vec<String>,
        filed_at: Option<i64>,
    ) -> PyResult<String> {
        let refs: Vec<Cid> =
            references.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let d = DisputeRecord {
            complainant: parse_did(complainant_did)?,
            respondent: parse_did(respondent_did)?,
            rung: parse_ladder_rung(rung)?,
            summary,
            references: refs,
            filed_at: parse_as_of(filed_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(complainant_priv);
        let signed = Signed::build(d, &[(complainant_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::DisputeFiled(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Record a mediation decision (§4), co-signed by the panel. The archive's
    /// authority pass enforces >= panel_min registered, non-party mediators.
    /// `outcome` is a DecisionOutcome name. Returns the CID.
    #[pyo3(signature = (dispute_cid, panel_privs, panel_dids, outcome, reasoning=String::new(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn decide_mediation(
        &mut self,
        dispute_cid: &str,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        outcome: &str,
        reasoning: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        if panel_privs.len() != panel_dids.len() {
            return Err(PyValueError::new_err(
                "panel_privs and panel_dids must be the same length",
            ));
        }
        if panel_privs.is_empty() {
            return Err(PyValueError::new_err("a panel needs at least one mediator"));
        }
        let panel: Vec<DidKey> =
            panel_dids.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
        let dec = MediationDecision {
            dispute_cid: parse_cid(dispute_cid)?,
            panel,
            outcome: parse_outcome(outcome)?,
            reasoning,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let sks: Vec<PrivKey> = panel_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            panel_dids.iter().map(|s| s.as_str()).zip(sks.iter()).collect();
        let signed = Signed::build(dec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::MediationDecided(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    // ---- §5 Escalation & designation authoring ---------------------------

    /// Advance (or de-escalate) a dispute along the ladder (§5), co-signed by the
    /// initiating communities. `from_rung`/`to_rung` are LadderRung names; the
    /// authority pass enforces that the move is legal from the dispute's current
    /// rung and that second-class rungs carry a non-unilateral quorum. Returns the CID.
    #[pyo3(signature = (initiator_privs, initiator_dids, dispute_cid, from_rung, to_rung, decision_class, reasoning=String::new(), transitioned_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn file_ladder_transition(
        &mut self,
        initiator_privs: Vec<Vec<u8>>,
        initiator_dids: Vec<String>,
        dispute_cid: &str,
        from_rung: &str,
        to_rung: &str,
        decision_class: &str,
        reasoning: String,
        transitioned_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&initiator_privs, &initiator_dids)?;
        let t = LadderTransition {
            dispute_cid: parse_cid(dispute_cid)?,
            from_rung: parse_ladder_rung(from_rung)?,
            to_rung: parse_ladder_rung(to_rung)?,
            initiators: ids,
            decision_class: parse_decision_class(decision_class)?,
            reasoning,
            transitioned_at: parse_as_of(transitioned_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(d, k)| (d.as_str(), k)).collect();
        let signed = Signed::build(t, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::LadderAdvanced(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Designate a community anathema (§5), co-signed by the deliberating quorum.
    /// `tier` is a DesignationTier name, `urgency` a DesignationUrgency name.
    /// `deliberation_cycles` is a list of `(cycle_index, [rep_did], opened_at,
    /// concluded_at, reasoning)`. The authority pass enforces the four-condition
    /// test. Returns the CID.
    #[pyo3(signature = (deliberator_privs, deliberator_dids, subject_did, tier, urgency, criteria, cited_acts, failed_restoration_cycles, deliberation_cycles, reversal_conditions=None, reexamine_after=None, designated_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn decide_designation(
        &mut self,
        deliberator_privs: Vec<Vec<u8>>,
        deliberator_dids: Vec<String>,
        subject_did: &str,
        tier: &str,
        urgency: &str,
        criteria: String,
        cited_acts: Vec<String>,
        failed_restoration_cycles: Vec<String>,
        deliberation_cycles: Vec<(u32, Vec<String>, i64, i64, String)>,
        reversal_conditions: Option<String>,
        reexamine_after: Option<i64>,
        designated_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&deliberator_privs, &deliberator_dids)?;
        let cited: Vec<Cid> = cited_acts.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let cycles_in: Vec<Cid> =
            failed_restoration_cycles.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let mut cycles = Vec::with_capacity(deliberation_cycles.len());
        for (idx, reps, opened, concluded, reasoning) in deliberation_cycles {
            let representatives: Vec<DidKey> =
                reps.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
            cycles.push(DeliberationCycle {
                cycle_index: idx,
                representatives,
                opened_at: ts(opened)?,
                concluded_at: ts(concluded)?,
                reasoning,
            });
        }
        let d = Designation {
            subject_community: parse_did(subject_did)?,
            tier: parse_designation_tier(tier)?,
            urgency: parse_designation_urgency(urgency)?,
            deliberators: ids,
            criteria,
            cited_acts: cited,
            failed_restoration_cycles: cycles_in,
            deliberation_cycles: cycles,
            reversal_conditions,
            reexamine_after: parse_as_of(reexamine_after)?,
            decision_class: DecisionClass::SecondClass,
            designated_at: parse_as_of(designated_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(d, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::DesignationDecided(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Impose a restitution obligation (§5), co-signed by the imposing communities.
    /// Returns the obligation's CID (referenced by a later verification).
    #[pyo3(signature = (imposer_privs, imposer_dids, subject_did, beneficiary_did, terms, dispute_cid=None, decision_class="FirstClass".to_string(), due_by=None, imposed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn impose_restitution(
        &mut self,
        imposer_privs: Vec<Vec<u8>>,
        imposer_dids: Vec<String>,
        subject_did: &str,
        beneficiary_did: &str,
        terms: String,
        dispute_cid: Option<String>,
        decision_class: String,
        due_by: Option<i64>,
        imposed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&imposer_privs, &imposer_dids)?;
        let o = RestitutionObligation {
            subject_community: parse_did(subject_did)?,
            beneficiary: parse_did(beneficiary_did)?,
            dispute_cid: dispute_cid.as_deref().map(parse_cid).transpose()?,
            terms,
            imposed_by: ids,
            due_by: parse_as_of(due_by)?,
            decision_class: parse_decision_class(&decision_class)?,
            imposed_at: parse_as_of(imposed_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(o, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::RestitutionObligated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Verify a restitution obligation (§5), co-signed by a verifier panel. The
    /// authority pass enforces a non-party mediator panel. Returns the CID.
    #[pyo3(signature = (verifier_privs, verifier_dids, obligation_cid, satisfied, narrative=String::new(), verified_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn verify_restitution(
        &mut self,
        verifier_privs: Vec<Vec<u8>>,
        verifier_dids: Vec<String>,
        obligation_cid: &str,
        satisfied: bool,
        narrative: String,
        verified_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&verifier_privs, &verifier_dids)?;
        let done = RestitutionCompletion {
            obligation_cid: parse_cid(obligation_cid)?,
            verifiers: ids,
            satisfied,
            narrative,
            verified_at: parse_as_of(verified_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(done, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::RestitutionVerified(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Complete a re-entry from an anathema designation (§5). The petitioner's
    /// fresh five-movement recitation and each witness attestation are built and
    /// signed here; the re-entry itself is co-signed by the admitting quorum.
    /// `movements` is (substrate, fallenness, recognition, common_virtue,
    /// non_ultimacy); `witnesses` is a list of `(priv, did, sincerity_affirmed)`.
    #[pyo3(signature = (deliberator_privs, deliberator_dids, designation_cid, petitioner_priv, petitioner_did, movements, witnesses, conduct_accounting, public_acknowledgement, probation_until, completed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn complete_reentry(
        &mut self,
        deliberator_privs: Vec<Vec<u8>>,
        deliberator_dids: Vec<String>,
        designation_cid: &str,
        petitioner_priv: &[u8],
        petitioner_did: &str,
        movements: (String, String, String, String, String),
        witnesses: Vec<(Vec<u8>, String, bool)>,
        conduct_accounting: String,
        public_acknowledgement: String,
        probation_until: i64,
        completed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&deliberator_privs, &deliberator_dids)?;
        let at = parse_as_of(completed_at)?.unwrap_or_else(time::now);
        let pet_did = parse_did(petitioner_did)?;
        let pet_sk = priv_from_bytes(petitioner_priv);
        let recitation = Recitation {
            community_did: pet_did.clone(),
            speaker_did: pet_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: movements.0,
            movement_fallenness: movements.1,
            movement_recognition: movements.2,
            movement_common_virtue: movements.3,
            movement_non_ultimacy: movements.4,
            language_tag: "en".into(),
            translation_en: None,
            recited_at: at,
        };
        let signed_rec =
            Signed::build(recitation, &[(petitioner_did, &pet_sk)]).map_err(kc_err_to_py)?;
        let rec_cid = signed_rec.payload_cid;
        let mut atts = Vec::with_capacity(witnesses.len());
        for (wpriv, wdid, affirmed) in &witnesses {
            let wsk = priv_from_bytes(wpriv);
            let att = ReEntryWitnessAttestation {
                witness_did: parse_did(wdid)?,
                recitation_cid: rec_cid,
                sincerity_affirmed: *affirmed,
                attested_at: at,
            };
            atts.push(Signed::build(att, &[(wdid.as_str(), &wsk)]).map_err(kc_err_to_py)?);
        }
        let re = ReEntry {
            designation_cid: parse_cid(designation_cid)?,
            petitioner: pet_did,
            recitation: signed_rec,
            conduct_accounting,
            public_acknowledgement,
            witnesses: atts,
            probation_until: ts(probation_until)?,
            deliberators: ids,
            decision_class: DecisionClass::SecondClass,
            completed_at: at,
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(re, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ReEntryCompleted(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Invoke the substantive-re-engagement rung against a member (§5), a
    /// deliberate act referencing decay evidence. Co-signed by the invoking
    /// communities. Returns the CID.
    #[pyo3(signature = (invoker_privs, invoker_dids, subject_did, decay_evidence, required_reaffirmation_by, decision_class="FirstClass".to_string(), invoked_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn invoke_substantive_reengagement(
        &mut self,
        invoker_privs: Vec<Vec<u8>>,
        invoker_dids: Vec<String>,
        subject_did: &str,
        decay_evidence: Vec<String>,
        required_reaffirmation_by: i64,
        decision_class: String,
        invoked_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&invoker_privs, &invoker_dids)?;
        let evidence: Vec<Cid> =
            decay_evidence.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let s = SubstantiveReEngagement {
            subject_community: parse_did(subject_did)?,
            decay_evidence: evidence,
            invoked_by: ids,
            required_reaffirmation_by: ts(required_reaffirmation_by)?,
            decision_class: parse_decision_class(&decision_class)?,
            invoked_at: parse_as_of(invoked_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(s, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SubstantiveReEngagementInvoked(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    fn cids(&self) -> Vec<String> {
        self.inner.iter().map(|(cid, _)| cid.to_string()).collect()
    }

    fn entries(&self) -> Vec<(String, String)> {
        self.inner
            .iter()
            .map(|(cid, e)| (e.kind().to_string(), cid.to_string()))
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Project this archive's entries into a ReputationView for the given
    /// community DID. `as_of` defaults to "now" when None.
    #[pyo3(signature = (did, as_of=None))]
    fn reputation_of(
        &self,
        did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyReputationView> {
        use std::str::FromStr as _;
        let did_key = DidKey::from_str(did).map_err(|e: KcError| {
            PyValueError::new_err(format!("invalid did: {e}"))
        })?;
        let as_of_ts = as_of
            .map(|s| {
                Timestamp::from_second(s)
                    .map_err(|e| PyValueError::new_err(format!("invalid as_of: {e}")))
            })
            .transpose()?;
        let view = kc_reputation::lookup(&self.inner, &did_key, as_of_ts);
        Ok(PyReputationView::from(&view))
    }

    /// Sponsorship graph over the whole archive as of `as_of` (default now):
    /// edges plus the per-sponsor concentration ranking (composition drift).
    #[pyo3(signature = (as_of=None))]
    fn composition_graph(&self, as_of: Option<i64>) -> PyResult<PyCompositionGraph> {
        let as_of_ts = parse_as_of(as_of)?;
        let g = kc_reputation::composition_graph(&self.inner, as_of_ts);
        Ok(PyCompositionGraph::from(&g))
    }

    /// Sponsor-stake report for `sponsor` as of `as_of`: for each sponsee, the
    /// adverse conduct recorded during its probation window.
    #[pyo3(signature = (sponsor, as_of=None))]
    fn sponsor_stake(&self, sponsor: &str, as_of: Option<i64>) -> PyResult<PySponsorStakeReport> {
        let did = parse_did(sponsor)?;
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::sponsor_stake(&self.inner, &did, as_of_ts);
        Ok(PySponsorStakeReport::from(&r))
    }

    /// Per-anti-spiral-feature adverse-conduct profile for `community` as of
    /// `as_of`: the behavioral shadow of a hollow or absent substrate.
    #[pyo3(signature = (community, as_of=None))]
    fn feature_concern_profile(
        &self,
        community: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyFeatureConcernProfile> {
        let did = parse_did(community)?;
        let as_of_ts = parse_as_of(as_of)?;
        let p = kc_reputation::feature_concern_profile(&self.inner, &did, as_of_ts);
        Ok(PyFeatureConcernProfile::from(&p))
    }

    /// v0 jester lens — compact-wide rejection pattern as of `as_of`: rejections
    /// tallied by rejecting community, presenting community, and anti-spiral
    /// feature, with the rejecting-community concentration ranking.
    #[pyo3(signature = (as_of=None))]
    fn rejection_pattern(&self, as_of: Option<i64>) -> PyResult<PyRejectionPattern> {
        let as_of_ts = parse_as_of(as_of)?;
        let p = kc_reputation::rejection_pattern(&self.inner, as_of_ts);
        Ok(PyRejectionPattern::from(&p))
    }

    /// v0 jester lens — ceremony-cost-decay series for `community` as of
    /// `as_of`: the witness-count and re-affirmation-gap trend over its intra
    /// rites, with staleness since the last rite.
    #[pyo3(signature = (community, as_of=None))]
    fn ceremony_cost_decay(
        &self,
        community: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyCeremonyCostDecay> {
        let did = parse_did(community)?;
        let as_of_ts = parse_as_of(as_of)?;
        let d = kc_reputation::ceremony_cost_decay(&self.inner, &did, as_of_ts);
        Ok(PyCeremonyCostDecay::from(&d))
    }

    // ---- Authoring (mutating) --------------------------------------------
    //
    // Signing happens here in Rust; the caller holds the private-key bytes
    // (from `generate_ed25519_keypair`). Records do not auto-verify on append
    // — the signer's identity must already be published — so call `verify_all`
    // after authoring a batch.

    /// Publish a self-signed community identity so records signed by this
    /// community's root key resolve during verification. `root_priv`/`root_pub`
    /// are the raw Ed25519 key bytes. Returns `(did, identity_cid)`.
    #[pyo3(signature = (name, root_priv, root_pub, created_at=None))]
    fn publish_identity(
        &mut self,
        name: &str,
        root_priv: &[u8],
        root_pub: &[u8],
        created_at: Option<i64>,
    ) -> PyResult<(String, String)> {
        let sk = priv_from_bytes(root_priv);
        let pk = pub_from_bytes(root_pub);
        let now = parse_as_of(created_at)?.unwrap_or_else(time::now);
        let signed = CommunityIdentity::new_self_signed(name.to_string(), &sk, pk, now)
            .map_err(kc_err_to_py)?;
        let did = signed.payload.did.to_string();
        let cid = self
            .inner
            .append(ArchiveEntry::CommunityIdentityPublished(signed))
            .map_err(kc_err_to_py)?;
        Ok((did, cid.to_string()))
    }

    /// Publish a `SpeakerDelegation`: the community (`community_did`, signing
    /// with `community_priv`) authorizes the key `speaker_pub` / `speaker_did`
    /// to act in `scope`. Signed by the community root key, so its identity must
    /// already be published. Once appended and active, records signed by
    /// `speaker_did` resolve during verification. `issued_at`/`expires_at` are
    /// Unix seconds; `expires_at=None` means open-ended (until revoked). The
    /// delegation must be active at each record's recorded_at/sponsored_at.
    /// Returns the entry CID.
    #[pyo3(signature = (community_priv, community_did, speaker_pub, speaker_did, scope, issued_at, expires_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn delegate_speaker(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        speaker_pub: &[u8],
        speaker_did: &str,
        scope: &str,
        issued_at: i64,
        expires_at: Option<i64>,
    ) -> PyResult<String> {
        let deleg = SpeakerDelegation {
            community_did: parse_did(community_did)?,
            speaker_did: parse_did(speaker_did)?,
            speaker_pubkey: pub_from_bytes(speaker_pub),
            scope: parse_delegation_scope(scope)?,
            issued_at: ts(issued_at)?,
            expires_at: expires_at.map(ts).transpose()?,
            revoked_at: None,
        };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(deleg, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SpeakerDelegated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Revoke a prior delegation to `speaker_did` by appending a superseding
    /// `SpeakerDelegation` tombstone: a later `issued_at` with `revoked_at` set,
    /// copying the pubkey/scope/expiry from the latest existing delegation.
    /// Signed by the community root key. Because resolution is latest-issued-at
    /// wins, records signed by the delegate and dated at/after `revoked_at` stop
    /// resolving, and the delegation drops out of `active_delegations`. Errors
    /// if no delegation exists for the speaker. `issued_at` defaults to now,
    /// `revoked_at` defaults to `issued_at`. Returns the entry CID.
    #[pyo3(signature = (community_priv, community_did, speaker_did, issued_at=None, revoked_at=None))]
    fn revoke_delegation(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        speaker_did: &str,
        issued_at: Option<i64>,
        revoked_at: Option<i64>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let sdid = parse_did(speaker_did)?;
        let (pubkey, scope, expires) = {
            let existing = self
                .inner
                .iter()
                .filter_map(|(_, e)| match e {
                    ArchiveEntry::SpeakerDelegated(s)
                        if s.payload.speaker_did == sdid
                            && s.payload.community_did == cdid =>
                    {
                        Some(&s.payload)
                    }
                    _ => None,
                })
                .max_by_key(|d| d.issued_at);
            match existing {
                Some(d) => (d.speaker_pubkey.clone(), d.scope, d.expires_at),
                None => {
                    return Err(PyValueError::new_err(
                        "no delegation to revoke for this speaker",
                    ))
                }
            }
        };
        let issued = parse_as_of(issued_at)?.unwrap_or_else(time::now);
        let revoked = parse_as_of(revoked_at)?.unwrap_or(issued);
        let rec = SpeakerDelegation {
            community_did: cdid,
            speaker_did: sdid,
            speaker_pubkey: pubkey,
            scope,
            issued_at: issued,
            expires_at: expires,
            revoked_at: Some(revoked),
        };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(rec, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SpeakerDelegated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed sponsorship (sponsor vouches for entrant). The envelope
    /// is signed by `signer_did` (the sponsor community's root key, or a key it
    /// has delegated); the payload attributes the vouch to `sponsor_did`. For a
    /// root-signed vouch pass `signer_did == sponsor_did`. Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, sponsor_did, entrant_did, sponsored_at, probation_until, statement=String::new(), admission_cid=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_sponsorship(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        sponsor_did: &str,
        entrant_did: &str,
        sponsored_at: i64,
        probation_until: i64,
        statement: String,
        admission_cid: Option<String>,
    ) -> PyResult<String> {
        let sponsor = parse_did(sponsor_did)?;
        let entrant = parse_did(entrant_did)?;
        let admission = admission_cid.as_deref().map(parse_cid).transpose()?;
        let rec = SponsorshipRecord {
            sponsor_community: sponsor,
            entrant_community: entrant,
            admission_cid: admission,
            probation_until: ts(probation_until)?,
            statement,
            sponsored_at: ts(sponsored_at)?,
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SponsorshipVouched(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed inter-community non-admission record (at least one feature
    /// claim was rejected). Signed by the `signer_privs[i]`/`signer_dids[i]`
    /// pairs (participant root keys, or keys they have delegated). `rejections`
    /// is a list of `(rejecting_did, presenting_did, feature, rationale)`. The
    /// signer identities must already be published. Returns the entry CID.
    #[pyo3(signature = (signer_privs, signer_dids, participants, rejections, compact_label=String::new(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_non_admission(
        &mut self,
        signer_privs: Vec<Vec<u8>>,
        signer_dids: Vec<String>,
        participants: Vec<String>,
        rejections: Vec<(String, String, String, String)>,
        compact_label: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        if signer_privs.len() != signer_dids.len() {
            return Err(PyValueError::new_err(
                "signer_privs and signer_dids must be the same length",
            ));
        }
        if signer_privs.is_empty() {
            return Err(PyValueError::new_err("at least one signer is required"));
        }
        let decided = parse_as_of(decided_at)?.unwrap_or_else(time::now);
        let parts: Vec<DidKey> =
            participants.iter().map(|p| parse_did(p)).collect::<PyResult<_>>()?;
        let mut rej: Vec<RejectionDetail> = Vec::with_capacity(rejections.len());
        for (rejecting, presenting, feature, rationale) in &rejections {
            rej.push(RejectionDetail {
                rejecting_community: parse_did(rejecting)?,
                presenting_community: parse_did(presenting)?,
                feature: parse_feature(feature)?,
                rationale: rationale.clone(),
            });
        }
        let rec = NonAdmissionRecord {
            compact_label,
            participants: parts,
            rejections: rej,
            decided_at: decided,
        };
        let sks: Vec<PrivKey> = signer_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            signer_dids.iter().map(|s| s.as_str()).zip(sks.iter()).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InterCommunityNonAdmission(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Author and append a full intra-community rite (a re-affirmation) for
    /// `community_did`, signing with its root key `community_priv`. Provisions a
    /// fresh Reciter delegation and a witness registry of `witness_count`
    /// internal witnesses (both anchored one hour before `sealed_at`), then runs
    /// the real `IntraCommunityRiteRunner` so the record is valid to the runner,
    /// not merely to `verify_all`. The community identity must already be
    /// published. `movements` is the five recitation movements; `witness_count`
    /// must be >= 1. Returns the rite entry CID.
    #[pyo3(signature = (community_priv, community_did, movements, witness_count, sealed_at))]
    fn append_intra_rite(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        movements: Vec<String>,
        witness_count: usize,
        sealed_at: i64,
    ) -> PyResult<String> {
        if movements.len() != 5 {
            return Err(PyValueError::new_err(format!(
                "movements must have 5 entries, got {}",
                movements.len()
            )));
        }
        if witness_count < 1 {
            return Err(PyValueError::new_err("witness_count must be >= 1"));
        }
        let did = parse_did(community_did)?;
        let sealed = ts(sealed_at)?;
        let root_sk = priv_from_bytes(community_priv);
        let identity = self
            .inner
            .community_identity(&did)
            .ok_or_else(|| PyValueError::new_err("community identity not published"))?
            .clone();
        let window_from = sealed
            .checked_sub(1.hour())
            .map_err(|e| PyValueError::new_err(format!("sealed_at too early: {e}")))?;

        // Reciter delegation, active at sealed_at.
        let (reciter_priv, reciter_pub) = keys::generate(Alg::Ed25519).map_err(kc_err_to_py)?;
        let reciter_did = DidKey::from_pubkey(&reciter_pub).map_err(kc_err_to_py)?;
        let reciter_delegation = Signed::build(
            SpeakerDelegation {
                community_did: did.clone(),
                speaker_did: reciter_did.clone(),
                speaker_pubkey: reciter_pub,
                scope: DelegationScope::Reciter,
                issued_at: window_from,
                expires_at: None,
                revoked_at: None,
            },
            &[(community_did, &root_sk)],
        )
        .map_err(kc_err_to_py)?;

        // Witness registry of `witness_count` internal witnesses.
        let mut witness_keys: Vec<(DidKey, PrivKey)> = Vec::with_capacity(witness_count);
        let mut entries = Vec::with_capacity(witness_count);
        for _ in 0..witness_count {
            let (wsk, wpk) = keys::generate(Alg::Ed25519).map_err(kc_err_to_py)?;
            let wdid = DidKey::from_pubkey(&wpk).map_err(kc_err_to_py)?;
            entries.push(WitnessEntry {
                witness_did: wdid.clone(),
                witness_pubkey: wpk,
                role: WitnessRole::IntraCommunity,
                valid_from: window_from,
                valid_until: None,
            });
            witness_keys.push((wdid, wsk));
        }
        let witness_registry = Signed::build(
            WitnessRegistry {
                community_did: did.clone(),
                entries,
                issued_at: window_from,
            },
            &[(community_did, &root_sk)],
        )
        .map_err(kc_err_to_py)?;

        self.inner
            .append(ArchiveEntry::SpeakerDelegated(reciter_delegation.clone()))
            .map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::WitnessRegistryUpdated(witness_registry.clone()))
            .map_err(kc_err_to_py)?;

        // Run the real rite.
        let delegations = vec![reciter_delegation];
        let runner = IntraCommunityRiteRunner::new(&identity, &delegations, &witness_registry);
        let [sub, fal, rec, vir, ult] = <[String; 5]>::try_from(movements).unwrap();
        let recitation = IntraCommunityRiteRunner::recite(
            &reciter_did,
            &reciter_priv,
            Recitation {
                community_did: did.clone(),
                speaker_did: reciter_did.clone(),
                version: RecitationVersion::current(),
                movement_substrate: sub,
                movement_fallenness: fal,
                movement_recognition: rec,
                movement_common_virtue: vir,
                movement_non_ultimacy: ult,
                language_tag: "en".into(),
                translation_en: None,
                recited_at: sealed,
            },
        )
        .map_err(kc_err_to_py)?;
        let mut attestations = Vec::with_capacity(witness_count);
        for (wdid, wsk) in &witness_keys {
            attestations.push(
                IntraCommunityRiteRunner::attest(wdid, wsk, recitation.payload_cid, sealed)
                    .map_err(kc_err_to_py)?,
            );
        }
        let rite = runner
            .seal(recitation, attestations, &root_sk, RecitationVersion::current(), sealed)
            .map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::IntraCommunityRite(rite))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed conduct record. The envelope is signed by the
    /// `signer_privs[i]` / `signer_dids[i]` pairs (community root keys or keys
    /// they have delegated — a multi-community quorum for second-class kinds);
    /// the payload attributes the report to `reporter_dids` (the communities).
    /// For a root-signed record pass `signer_dids == reporter_dids`.
    /// `bearing` is one of AntiSpiral/CommonsObligation/Charity/Rescue/Other;
    /// AntiSpiral requires `feature`, Other requires `bearing_detail`.
    /// Returns the entry CID.
    #[pyo3(signature = (
        signer_privs, signer_dids, reporter_dids, subject_did, kind, bearing,
        recorded_at, observed_at,
        feature=None, bearing_detail=None, severity=String::from("Minor"),
        ladder_rung=None, decision_class=String::from("FirstClass"),
        references=Vec::new(), narrative=String::new(), perspective=None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn append_conduct(
        &mut self,
        signer_privs: Vec<Vec<u8>>,
        signer_dids: Vec<String>,
        reporter_dids: Vec<String>,
        subject_did: &str,
        kind: &str,
        bearing: &str,
        recorded_at: i64,
        observed_at: i64,
        feature: Option<String>,
        bearing_detail: Option<String>,
        severity: String,
        ladder_rung: Option<String>,
        decision_class: String,
        references: Vec<String>,
        narrative: String,
        perspective: Option<String>,
    ) -> PyResult<String> {
        if signer_privs.len() != signer_dids.len() || signer_privs.is_empty() {
            return Err(PyValueError::new_err(
                "signer_privs and signer_dids must be non-empty and equal length",
            ));
        }
        if reporter_dids.is_empty() {
            return Err(PyValueError::new_err("reporter_dids must be non-empty"));
        }
        let subject = parse_did(subject_did)?;
        let reporters: Vec<DidKey> = reporter_dids
            .iter()
            .map(|d| parse_did(d))
            .collect::<PyResult<_>>()?;
        let refs: Vec<Cid> = references
            .iter()
            .map(|s| parse_cid(s))
            .collect::<PyResult<_>>()?;
        let rec = ConductRecord {
            subject_community: subject,
            reporters,
            reporter_perspective: perspective,
            kind: parse_conduct_kind(kind)?,
            bearing: parse_bearing(bearing, feature.as_deref(), bearing_detail.as_deref())?,
            severity: parse_severity(&severity)?,
            ladder_rung: ladder_rung.as_deref().map(parse_ladder_rung).transpose()?,
            decision_class: parse_decision_class(&decision_class)?,
            references: refs,
            narrative,
            observed_at: ts(observed_at)?,
            recorded_at: ts(recorded_at)?,
        };
        let sks: Vec<PrivKey> = signer_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let signers: Vec<(&str, &PrivKey)> = signer_dids
            .iter()
            .zip(sks.iter())
            .map(|(d, sk)| (d.as_str(), sk))
            .collect();
        let signed = Signed::build(rec, &signers).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ConductAttested(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }
}

fn priv_from_bytes(b: &[u8]) -> PrivKey {
    PrivKey { alg: Alg::Ed25519, bytes: b.to_vec() }
}

fn parse_cid(s: &str) -> PyResult<Cid> {
    let hex = s.strip_prefix("b3:").unwrap_or(s);
    Cid::from_hex(hex).map_err(|e| PyValueError::new_err(format!("bad cid '{s}': {e}")))
}

fn pub_from_bytes(b: &[u8]) -> PubKey {
    PubKey { alg: Alg::Ed25519, bytes: b.to_vec() }
}

fn ts(secs: i64) -> PyResult<Timestamp> {
    Timestamp::from_second(secs)
        .map_err(|e| PyValueError::new_err(format!("invalid timestamp {secs}: {e}")))
}

fn parse_conduct_kind(s: &str) -> PyResult<ConductKind> {
    Ok(match s {
        "Commendation" => ConductKind::Commendation,
        "Concern" => ConductKind::Concern,
        "Violation" => ConductKind::Violation,
        "DesignationAnathema" => ConductKind::DesignationAnathema,
        "DesignationLifted" => ConductKind::DesignationLifted,
        "RestorationCompleted" => ConductKind::RestorationCompleted,
        other => return Err(PyValueError::new_err(format!("unknown conduct kind: {other}"))),
    })
}

fn parse_feature(s: &str) -> PyResult<AntiSpiralFeature> {
    Ok(match s {
        "FallenActorAnthropology" => AntiSpiralFeature::FallenActorAnthropology,
        "GraceOverMerit" => AntiSpiralFeature::GraceOverMerit,
        "ScapegoatDisarming" => AntiSpiralFeature::ScapegoatDisarming,
        "EnemyLoveOperational" => AntiSpiralFeature::EnemyLoveOperational,
        "NonUltimacyOfCompact" => AntiSpiralFeature::NonUltimacyOfCompact,
        other => return Err(PyValueError::new_err(format!("unknown anti-spiral feature: {other}"))),
    })
}

fn parse_bearing(
    kind: &str,
    feature: Option<&str>,
    detail: Option<&str>,
) -> PyResult<ConductBearing> {
    Ok(match kind {
        "AntiSpiral" => {
            let f = feature.ok_or_else(|| {
                PyValueError::new_err("bearing 'AntiSpiral' requires a `feature`")
            })?;
            ConductBearing::AntiSpiral(parse_feature(f)?)
        }
        "CommonsObligation" => ConductBearing::CommonsObligation,
        "Charity" => ConductBearing::Charity,
        "Rescue" => ConductBearing::Rescue,
        "Other" => ConductBearing::Other(
            detail
                .ok_or_else(|| PyValueError::new_err("bearing 'Other' requires `bearing_detail`"))?
                .to_string(),
        ),
        other => return Err(PyValueError::new_err(format!("unknown bearing: {other}"))),
    })
}

fn parse_severity(s: &str) -> PyResult<Severity> {
    Ok(match s {
        "Minor" => Severity::Minor,
        "Serious" => Severity::Serious,
        "Grave" => Severity::Grave,
        other => return Err(PyValueError::new_err(format!("unknown severity: {other}"))),
    })
}

fn parse_ladder_rung(s: &str) -> PyResult<LadderRung> {
    Ok(match s {
        "DisputeRegistered" => LadderRung::DisputeRegistered,
        "RegisteredProtest" => LadderRung::RegisteredProtest,
        "ReciprocalSuspension" => LadderRung::ReciprocalSuspension,
        "MediatedArbitration" => LadderRung::MediatedArbitration,
        "LimitedSanctions" => LadderRung::LimitedSanctions,
        "CollectiveSanctions" => LadderRung::CollectiveSanctions,
        "Expulsion" => LadderRung::Expulsion,
        "SubstantiveReEngagement" => LadderRung::SubstantiveReEngagement,
        other => return Err(PyValueError::new_err(format!("unknown ladder rung: {other}"))),
    })
}

fn parse_outcome(s: &str) -> PyResult<DecisionOutcome> {
    Ok(match s {
        "Upheld" => DecisionOutcome::Upheld,
        "Dismissed" => DecisionOutcome::Dismissed,
        "Settled" => DecisionOutcome::Settled,
        "Referred" => DecisionOutcome::Referred,
        other => return Err(PyValueError::new_err(format!("unknown decision outcome: {other}"))),
    })
}

fn parse_decision_class(s: &str) -> PyResult<DecisionClass> {
    Ok(match s {
        "FirstClass" => DecisionClass::FirstClass,
        "SecondClass" => DecisionClass::SecondClass,
        other => return Err(PyValueError::new_err(format!("unknown decision class: {other}"))),
    })
}

fn parse_designation_tier(s: &str) -> PyResult<DesignationTier> {
    Ok(match s {
        "Anathema" => DesignationTier::Anathema,
        "HostisHumaniGeneris" => DesignationTier::HostisHumaniGeneris,
        other => return Err(PyValueError::new_err(format!("unknown designation tier: {other}"))),
    })
}

fn parse_designation_urgency(s: &str) -> PyResult<DesignationUrgency> {
    Ok(match s {
        "Normal" => DesignationUrgency::Normal,
        "Expedited" => DesignationUrgency::Expedited,
        other => return Err(PyValueError::new_err(format!("unknown designation urgency: {other}"))),
    })
}

/// Validate a multi-signer set and prepare it: returns the parsed principal DIDs
/// and owned `(did_string, priv_key)` pairs a caller can borrow for `Signed::build`.
fn cosigners(privs: &[Vec<u8>], dids: &[String]) -> PyResult<(Vec<DidKey>, Vec<(String, PrivKey)>)> {
    if privs.len() != dids.len() {
        return Err(PyValueError::new_err("signer privs and dids must be the same length"));
    }
    if privs.is_empty() {
        return Err(PyValueError::new_err("need at least one signer"));
    }
    let ids: Vec<DidKey> = dids.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
    let pairs: Vec<(String, PrivKey)> = dids
        .iter()
        .zip(privs.iter())
        .map(|(d, p)| (d.clone(), priv_from_bytes(p)))
        .collect();
    Ok((ids, pairs))
}

fn parse_delegation_scope(s: &str) -> PyResult<DelegationScope> {
    Ok(match s {
        "Reciter" => DelegationScope::Reciter,
        "Envoy" => DelegationScope::Envoy,
        "InternalWitness" => DelegationScope::InternalWitness,
        "CrossBoundaryWitness" => DelegationScope::CrossBoundaryWitness,
        other => return Err(PyValueError::new_err(format!("unknown delegation scope: {other}"))),
    })
}

fn parse_did(did: &str) -> PyResult<DidKey> {
    use std::str::FromStr as _;
    DidKey::from_str(did).map_err(|e: KcError| PyValueError::new_err(format!("invalid did: {e}")))
}

fn parse_as_of(as_of: Option<i64>) -> PyResult<Option<Timestamp>> {
    as_of
        .map(|s| {
            Timestamp::from_second(s)
                .map_err(|e| PyValueError::new_err(format!("invalid as_of: {e}")))
        })
        .transpose()
}

#[pyclass]
struct PyCeremonyResult {
    #[pyo3(get)]
    alpha_did: String,
    #[pyo3(get)]
    beta_did: String,
    #[pyo3(get)]
    alpha_identity_cid: String,
    #[pyo3(get)]
    beta_identity_cid: String,
    #[pyo3(get)]
    alpha_intra_cid: String,
    #[pyo3(get)]
    beta_intra_cid: String,
    #[pyo3(get)]
    witness_cid: String,
    #[pyo3(get)]
    archive_size: usize,
    #[pyo3(get)]
    archive_json: String,
    #[pyo3(get)]
    compact_label: String,
}

struct CommunityBundle {
    signed_identity: Signed<CommunityIdentity>,
    root_priv: PrivKey,
    speaker_did: DidKey,
    speaker_priv: PrivKey,
    envoy_did: DidKey,
    envoy_priv: PrivKey,
    witness_did: DidKey,
    witness_priv: PrivKey,
    speaker_delegation: Signed<SpeakerDelegation>,
    envoy_delegation: Signed<SpeakerDelegation>,
    witness_registry: Signed<WitnessRegistry>,
}

fn mk_community(name: &str, now: Timestamp) -> Result<CommunityBundle, KcError> {
    let (root_priv, root_pub) = keys::generate(Alg::Ed25519)?;
    let signed_identity =
        CommunityIdentity::new_self_signed(name.into(), &root_priv, root_pub, now)?;
    let community_did = signed_identity.payload.did.clone();

    let (speaker_priv, speaker_pk) = keys::generate(Alg::Ed25519)?;
    let speaker_did = DidKey::from_pubkey(&speaker_pk)?;
    let speaker_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: speaker_did.clone(),
            speaker_pubkey: speaker_pk,
            scope: DelegationScope::Reciter,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )?;

    let (envoy_priv, envoy_pk) = keys::generate(Alg::Ed25519)?;
    let envoy_did = DidKey::from_pubkey(&envoy_pk)?;
    let envoy_delegation = Signed::build(
        SpeakerDelegation {
            community_did: community_did.clone(),
            speaker_did: envoy_did.clone(),
            speaker_pubkey: envoy_pk.clone(),
            scope: DelegationScope::Envoy,
            issued_at: now.checked_sub(1.hour()).unwrap(),
            expires_at: Some(now.checked_add(24.hours()).unwrap()),
            revoked_at: None,
        },
        &[(community_did.as_str(), &root_priv)],
    )?;

    let (witness_priv, witness_pk) = keys::generate(Alg::Ed25519)?;
    let witness_did = DidKey::from_pubkey(&witness_pk)?;

    let witness_registry = Signed::build(
        WitnessRegistry {
            community_did: community_did.clone(),
            entries: vec![
                WitnessEntry {
                    witness_did: witness_did.clone(),
                    witness_pubkey: witness_pk,
                    role: WitnessRole::IntraCommunity,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
                WitnessEntry {
                    witness_did: envoy_did.clone(),
                    witness_pubkey: envoy_pk,
                    role: WitnessRole::CrossBoundary,
                    valid_from: now.checked_sub(1.hour()).unwrap(),
                    valid_until: Some(now.checked_add(24.hours()).unwrap()),
                },
            ],
            issued_at: now,
        },
        &[(community_did.as_str(), &root_priv)],
    )?;

    Ok(CommunityBundle {
        signed_identity,
        root_priv,
        speaker_did,
        speaker_priv,
        envoy_did,
        envoy_priv,
        witness_did,
        witness_priv,
        speaker_delegation,
        envoy_delegation,
        witness_registry,
    })
}

fn seed_archive(archive: &mut InMemoryArchive, c: &CommunityBundle) -> Result<(), KcError> {
    archive.append(ArchiveEntry::CommunityIdentityPublished(
        c.signed_identity.clone(),
    ))?;
    archive.append(ArchiveEntry::SpeakerDelegated(c.speaker_delegation.clone()))?;
    archive.append(ArchiveEntry::SpeakerDelegated(c.envoy_delegation.clone()))?;
    archive.append(ArchiveEntry::WitnessRegistryUpdated(
        c.witness_registry.clone(),
    ))?;
    Ok(())
}

fn run_intra_rite(
    c: &CommunityBundle,
    movements: [String; 5],
    now: Timestamp,
) -> Result<Signed<IntraCommunityRiteRecord>, KcError> {
    let delegations = vec![c.speaker_delegation.clone()];
    let runner =
        IntraCommunityRiteRunner::new(&c.signed_identity, &delegations, &c.witness_registry);
    let [sub, fal, rec, vir, ult] = movements;
    let recitation = IntraCommunityRiteRunner::recite(
        &c.speaker_did,
        &c.speaker_priv,
        Recitation {
            community_did: c.signed_identity.payload.did.clone(),
            speaker_did: c.speaker_did.clone(),
            version: RecitationVersion::current(),
            movement_substrate: sub,
            movement_fallenness: fal,
            movement_recognition: rec,
            movement_common_virtue: vir,
            movement_non_ultimacy: ult,
            language_tag: "en".into(),
            translation_en: None,
            recited_at: now,
        },
    )?;
    let attestation = IntraCommunityRiteRunner::attest(
        &c.witness_did,
        &c.witness_priv,
        recitation.payload_cid,
        now,
    )?;
    runner.seal(
        recitation,
        vec![attestation],
        &c.root_priv,
        RecitationVersion::current(),
        now,
    )
}

#[pyfunction]
#[pyo3(signature = (alpha_name, alpha_movements, beta_name, beta_movements, compact_label = String::from("two-community-compact")))]
fn run_two_community_ceremony(
    alpha_name: &str,
    alpha_movements: Vec<String>,
    beta_name: &str,
    beta_movements: Vec<String>,
    compact_label: String,
) -> PyResult<PyCeremonyResult> {
    if alpha_movements.len() != 5 {
        return Err(PyValueError::new_err(format!(
            "alpha_movements must have 5 entries, got {}",
            alpha_movements.len()
        )));
    }
    if beta_movements.len() != 5 {
        return Err(PyValueError::new_err(format!(
            "beta_movements must have 5 entries, got {}",
            beta_movements.len()
        )));
    }
    let alpha_arr: [String; 5] = alpha_movements.try_into().unwrap();
    let beta_arr: [String; 5] = beta_movements.try_into().unwrap();

    let now = time::now();
    let alpha = mk_community(alpha_name, now).map_err(kc_err_to_py)?;
    let beta = mk_community(beta_name, now).map_err(kc_err_to_py)?;

    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &alpha).map_err(kc_err_to_py)?;
    seed_archive(&mut archive, &beta).map_err(kc_err_to_py)?;

    let alpha_rite = run_intra_rite(&alpha, alpha_arr, now).map_err(kc_err_to_py)?;
    let beta_rite = run_intra_rite(&beta, beta_arr, now).map_err(kc_err_to_py)?;
    let alpha_intra_cid = archive
        .append(ArchiveEntry::IntraCommunityRite(alpha_rite.clone()))
        .map_err(kc_err_to_py)?;
    let beta_intra_cid = archive
        .append(ArchiveEntry::IntraCommunityRite(beta_rite.clone()))
        .map_err(kc_err_to_py)?;

    let participants = vec![
        ParticipantContext {
            community: &alpha.signed_identity,
            intra_record: &alpha_rite,
            cross_boundary_signer_did: alpha.envoy_did.clone(),
            cross_boundary_signer_priv: &alpha.envoy_priv,
        },
        ParticipantContext {
            community: &beta.signed_identity,
            intra_record: &beta_rite,
            cross_boundary_signer_did: beta.envoy_did.clone(),
            cross_boundary_signer_priv: &beta.envoy_priv,
        },
    ];
    let inter_runner = InterCredentialingRunner::new(compact_label.clone(), participants);

    let mut claims = Vec::new();
    for (community_did, envoy_did, envoy_priv) in [
        (
            alpha.signed_identity.payload.did.clone(),
            &alpha.envoy_did,
            &alpha.envoy_priv,
        ),
        (
            beta.signed_identity.payload.did.clone(),
            &beta.envoy_did,
            &beta.envoy_priv,
        ),
    ] {
        for feature in REQUIRED_FEATURES {
            let claim = InterCredentialingRunner::present_claim(
                envoy_did,
                envoy_priv,
                community_did.clone(),
                feature,
                format!("{feature:?} evidenced by our sealed rite and practice."),
                vec![],
                now,
            )
            .map_err(kc_err_to_py)?;
            claims.push(claim);
        }
    }

    let mut acceptances = Vec::new();
    for claim in &claims {
        let (signer_did, signer_priv, accepter_did) =
            if claim.payload.community_did == alpha.signed_identity.payload.did {
                (
                    &beta.envoy_did,
                    &beta.envoy_priv,
                    beta.signed_identity.payload.did.clone(),
                )
            } else {
                (
                    &alpha.envoy_did,
                    &alpha.envoy_priv,
                    alpha.signed_identity.payload.did.clone(),
                )
            };
        let acceptance = InterCredentialingRunner::accept(
            signer_did,
            signer_priv,
            accepter_did,
            claim.payload_cid,
            true,
            None,
            now,
        )
        .map_err(kc_err_to_py)?;
        acceptances.push(acceptance);
    }

    let outcome = inter_runner
        .seal(claims, acceptances, vec![], now)
        .map_err(kc_err_to_py)?;
    let witness_record = match outcome {
        InterCredentialingOutcome::Sealed(s) => s,
        InterCredentialingOutcome::NonAdmission(_) => {
            return Err(PyRuntimeError::new_err(
                "expected admission but inter ceremony produced non-admission",
            ));
        }
    };
    let witness_cid = archive
        .append(ArchiveEntry::InterCommunityWitness(witness_record.clone()))
        .map_err(kc_err_to_py)?;

    verify_all(&archive).map_err(kc_err_to_py)?;
    let archive_json = archive.dump_json_to_string().map_err(kc_err_to_py)?;
    let archive_size = archive.len();

    Ok(PyCeremonyResult {
        alpha_did: alpha.signed_identity.payload.did.to_string(),
        beta_did: beta.signed_identity.payload.did.to_string(),
        alpha_identity_cid: alpha.signed_identity.payload_cid.to_string(),
        beta_identity_cid: beta.signed_identity.payload_cid.to_string(),
        alpha_intra_cid: alpha_intra_cid.to_string(),
        beta_intra_cid: beta_intra_cid.to_string(),
        witness_cid: witness_cid.to_string(),
        archive_size,
        archive_json,
        compact_label,
    })
}

/// Result of [`demo_capture_scenario`]: the archive JSON plus the DIDs a caller
/// needs to query the projection and analytics.
#[pyclass]
struct PyCaptureScenario {
    #[pyo3(get)]
    hub_did: String,
    #[pyo3(get)]
    drifting_entrant_did: String,
    #[pyo3(get)]
    healthy_entrant_did: String,
    #[pyo3(get)]
    archive_json: String,
    #[pyo3(get)]
    archive_size: usize,
}

/// Build a small, fully-signed archive exhibiting the two capture signals this
/// slice detects, so callers (and tests) have something concrete to project
/// over. Mirrors the `run_two_community_ceremony` idiom.
///
/// Shape: a hub community sponsors two entrants (composition concentration).
/// The hub then files two `Concern` records against one entrant, both bearing
/// on anti-spiral features (`NonUltimacyOfCompact`, `EnemyLoveOperational`) and
/// both inside that entrant's probation window — the behavioral shadow of a
/// hollow substrate. The other entrant receives only a `Commendation`.
#[pyfunction]
fn demo_capture_scenario() -> PyResult<PyCaptureScenario> {
    let now = time::now();
    let hub = mk_community("Hub", now).map_err(kc_err_to_py)?;
    let drifting = mk_community("Drifting", now).map_err(kc_err_to_py)?;
    let healthy = mk_community("Healthy", now).map_err(kc_err_to_py)?;

    let mut archive = InMemoryArchive::new();
    seed_archive(&mut archive, &hub).map_err(kc_err_to_py)?;
    seed_archive(&mut archive, &drifting).map_err(kc_err_to_py)?;
    seed_archive(&mut archive, &healthy).map_err(kc_err_to_py)?;

    let hub_did = hub.signed_identity.payload.did.clone();
    let drifting_did = drifting.signed_identity.payload.did.clone();
    let healthy_did = healthy.signed_identity.payload.did.clone();

    // jiff Timestamp arithmetic takes hours-or-smaller units, not calendar days.
    let sponsored_at = now.checked_sub((30 * 24).hours()).unwrap();
    let probation_until = now.checked_add((300 * 24).hours()).unwrap();

    // Hub sponsors both entrants (out-degree 2 -> concentration).
    for entrant in [&drifting_did, &healthy_did] {
        let rec = SponsorshipRecord {
            sponsor_community: hub_did.clone(),
            entrant_community: entrant.clone(),
            admission_cid: None,
            probation_until,
            statement: "vouched by the hub".into(),
            sponsored_at,
        };
        let signed =
            Signed::build(rec, &[(hub_did.as_str(), &hub.root_priv)]).map_err(kc_err_to_py)?;
        archive
            .append(ArchiveEntry::SponsorshipVouched(signed))
            .map_err(kc_err_to_py)?;
    }

    // Two Concerns against the drifting entrant, in probation, on features.
    let recorded_at = now.checked_sub(24.hours()).unwrap();
    for feature in [
        AntiSpiralFeature::NonUltimacyOfCompact,
        AntiSpiralFeature::EnemyLoveOperational,
    ] {
        let rec = ConductRecord {
            subject_community: drifting_did.clone(),
            reporters: vec![hub_did.clone()],
            reporter_perspective: Some("charter-originalist observer".into()),
            kind: ConductKind::Concern,
            bearing: ConductBearing::AntiSpiral(feature),
            severity: Severity::Minor,
            ladder_rung: None,
            decision_class: DecisionClass::FirstClass,
            references: vec![],
            narrative: format!("{feature:?}: decided a matter its charter reserves"),
            observed_at: recorded_at,
            recorded_at,
        };
        let signed =
            Signed::build(rec, &[(hub_did.as_str(), &hub.root_priv)]).map_err(kc_err_to_py)?;
        archive
            .append(ArchiveEntry::ConductAttested(signed))
            .map_err(kc_err_to_py)?;
    }

    // A commendation for the healthy entrant (not adverse).
    let commend = ConductRecord {
        subject_community: healthy_did.clone(),
        reporters: vec![hub_did.clone()],
        reporter_perspective: None,
        kind: ConductKind::Commendation,
        bearing: ConductBearing::AntiSpiral(AntiSpiralFeature::GraceOverMerit),
        severity: Severity::Minor,
        ladder_rung: None,
        decision_class: DecisionClass::FirstClass,
        references: vec![],
        narrative: "restored a lapsed member rather than expelling".into(),
        observed_at: recorded_at,
        recorded_at,
    };
    let signed =
        Signed::build(commend, &[(hub_did.as_str(), &hub.root_priv)]).map_err(kc_err_to_py)?;
    archive
        .append(ArchiveEntry::ConductAttested(signed))
        .map_err(kc_err_to_py)?;

    verify_all(&archive).map_err(kc_err_to_py)?;
    let archive_json = archive.dump_json_to_string().map_err(kc_err_to_py)?;
    let archive_size = archive.len();

    Ok(PyCaptureScenario {
        hub_did: hub_did.to_string(),
        drifting_entrant_did: drifting_did.to_string(),
        healthy_entrant_did: healthy_did.to_string(),
        archive_json,
        archive_size,
    })
}

// ---- Reputation lookup bindings --------------------------------------------

use kc_reputation as rep;

#[pyclass(name = "IntraRiteSummary", get_all)]
#[derive(Clone)]
struct PyIntraRiteSummary {
    record_cid: String,
    sealed_at: i64,
    speaker_did: String,
    witness_count: usize,
}

impl From<&rep::IntraRiteSummary> for PyIntraRiteSummary {
    fn from(s: &rep::IntraRiteSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            sealed_at: s.sealed_at.as_second(),
            speaker_did: s.speaker_did.to_string(),
            witness_count: s.witness_count,
        }
    }
}

#[pyclass(name = "InterAdmissionSummary", get_all)]
#[derive(Clone)]
struct PyInterAdmissionSummary {
    record_cid: String,
    witnessed_at: i64,
    counterparties: Vec<String>,
}

impl From<&rep::InterAdmissionSummary> for PyInterAdmissionSummary {
    fn from(s: &rep::InterAdmissionSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            witnessed_at: s.witnessed_at.as_second(),
            counterparties: s.counterparties.iter().map(|d| d.to_string()).collect(),
        }
    }
}

#[pyclass(name = "RejectionSummary", get_all)]
#[derive(Clone)]
struct PyRejectionSummary {
    rejecting_community: String,
    presenting_community: String,
    feature: String,
    rationale: String,
}

impl From<&rep::RejectionSummary> for PyRejectionSummary {
    fn from(s: &rep::RejectionSummary) -> Self {
        Self {
            rejecting_community: s.rejecting_community.to_string(),
            presenting_community: s.presenting_community.to_string(),
            feature: format!("{:?}", s.feature),
            rationale: s.rationale.clone(),
        }
    }
}

#[pyclass(name = "InterNonAdmissionSummary", get_all)]
#[derive(Clone)]
struct PyInterNonAdmissionSummary {
    record_cid: String,
    decided_at: i64,
    counterparties: Vec<String>,
    rejections: Vec<PyRejectionSummary>,
}

impl From<&rep::InterNonAdmissionSummary> for PyInterNonAdmissionSummary {
    fn from(s: &rep::InterNonAdmissionSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            decided_at: s.decided_at.as_second(),
            counterparties: s.counterparties.iter().map(|d| d.to_string()).collect(),
            rejections: s.rejections.iter().map(PyRejectionSummary::from).collect(),
        }
    }
}

#[pyclass(name = "DelegationSummary", get_all)]
#[derive(Clone)]
struct PyDelegationSummary {
    record_cid: String,
    subject_did: String,
    scope: String,
    active_from: i64,
    active_until: Option<i64>,
}

impl From<&rep::DelegationSummary> for PyDelegationSummary {
    fn from(s: &rep::DelegationSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            subject_did: s.subject_did.to_string(),
            scope: format!("{:?}", s.scope),
            active_from: s.active_from.as_second(),
            active_until: s.active_until.map(|t| t.as_second()),
        }
    }
}

#[pyclass(name = "WitnessSummary", get_all)]
#[derive(Clone)]
struct PyWitnessSummary {
    record_cid: String,
    member_did: String,
    role: String,
    active_from: i64,
    active_until: Option<i64>,
}

impl From<&rep::WitnessSummary> for PyWitnessSummary {
    fn from(s: &rep::WitnessSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            member_did: s.member_did.to_string(),
            role: format!("{:?}", s.role),
            active_from: s.active_from.as_second(),
            active_until: s.active_until.map(|t| t.as_second()),
        }
    }
}

#[pyclass(name = "ConductSummary", get_all)]
#[derive(Clone)]
struct PyConductSummary {
    record_cid: String,
    kind: String,
    bearing: String,
    severity: String,
    ladder_rung: Option<String>,
    decision_class: String,
    reporters: Vec<String>,
    reporter_perspective: Option<String>,
    references: Vec<String>,
    recorded_at: i64,
}

impl From<&rep::ConductSummary> for PyConductSummary {
    fn from(s: &rep::ConductSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            kind: s.kind.as_str().to_string(),
            bearing: format!("{:?}", s.bearing),
            severity: format!("{:?}", s.severity),
            ladder_rung: s.ladder_rung.map(|r| format!("{r:?}")),
            decision_class: format!("{:?}", s.decision_class),
            reporters: s.reporters.iter().map(|d| d.to_string()).collect(),
            reporter_perspective: s.reporter_perspective.clone(),
            references: s.references.iter().map(|c| c.to_string()).collect(),
            recorded_at: s.recorded_at.as_second(),
        }
    }
}

#[pyclass(name = "SponsorshipSummary", get_all)]
#[derive(Clone)]
struct PySponsorshipSummary {
    record_cid: String,
    sponsor_community: String,
    entrant_community: String,
    admission_cid: Option<String>,
    probation_until: i64,
    sponsored_at: i64,
}

impl From<&rep::SponsorshipSummary> for PySponsorshipSummary {
    fn from(s: &rep::SponsorshipSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            sponsor_community: s.sponsor_community.to_string(),
            entrant_community: s.entrant_community.to_string(),
            admission_cid: s.admission_cid.as_ref().map(|c| c.to_string()),
            probation_until: s.probation_until.as_second(),
            sponsored_at: s.sponsored_at.as_second(),
        }
    }
}

#[pyclass(name = "DisputeSummary", get_all)]
#[derive(Clone)]
struct PyDisputeSummary {
    record_cid: String,
    complainant: String,
    respondent: String,
    rung: String,
    /// The dispute's current rung after any ladder transitions (§5).
    current_rung: String,
    summary: String,
    filed_at: i64,
    /// `None` while open; the `DecisionOutcome` name once a panel has decided.
    decided_outcome: Option<String>,
}

impl From<&rep::DisputeSummary> for PyDisputeSummary {
    fn from(s: &rep::DisputeSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            complainant: s.complainant.to_string(),
            respondent: s.respondent.to_string(),
            rung: format!("{:?}", s.rung),
            current_rung: format!("{:?}", s.current_rung),
            summary: s.summary.clone(),
            filed_at: s.filed_at.as_second(),
            decided_outcome: s.decided_outcome.clone(),
        }
    }
}

#[pyclass(name = "DesignationSummary", get_all)]
#[derive(Clone)]
struct PyDesignationSummary {
    record_cid: String,
    tier: String,
    urgency: String,
    criteria: String,
    reversal_conditions: Option<String>,
    reexamine_after: Option<i64>,
    reversed: bool,
    designated_at: i64,
}

impl From<&rep::DesignationSummary> for PyDesignationSummary {
    fn from(s: &rep::DesignationSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            tier: s.tier.clone(),
            urgency: s.urgency.clone(),
            criteria: s.criteria.clone(),
            reversal_conditions: s.reversal_conditions.clone(),
            reexamine_after: s.reexamine_after.map(|t| t.as_second()),
            reversed: s.reversed,
            designated_at: s.designated_at.as_second(),
        }
    }
}

#[pyclass(name = "ReEntrySummary", get_all)]
#[derive(Clone)]
struct PyReEntrySummary {
    record_cid: String,
    designation_cid: String,
    probation_until: i64,
    completed_at: i64,
}

impl From<&rep::ReEntrySummary> for PyReEntrySummary {
    fn from(s: &rep::ReEntrySummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            designation_cid: s.designation_cid.to_string(),
            probation_until: s.probation_until.as_second(),
            completed_at: s.completed_at.as_second(),
        }
    }
}

#[pyclass(name = "RestitutionSummary", get_all)]
#[derive(Clone)]
struct PyRestitutionSummary {
    record_cid: String,
    beneficiary: String,
    terms: String,
    satisfied: Option<bool>,
    imposed_at: i64,
}

impl From<&rep::RestitutionSummary> for PyRestitutionSummary {
    fn from(s: &rep::RestitutionSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            beneficiary: s.beneficiary.to_string(),
            terms: s.terms.clone(),
            satisfied: s.satisfied,
            imposed_at: s.imposed_at.as_second(),
        }
    }
}

#[pyclass(name = "SubstantiveReEngagementSummary", get_all)]
#[derive(Clone)]
struct PySubstantiveReEngagementSummary {
    record_cid: String,
    required_reaffirmation_by: i64,
    invoked_at: i64,
}

impl From<&rep::SubstantiveReEngagementSummary> for PySubstantiveReEngagementSummary {
    fn from(s: &rep::SubstantiveReEngagementSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            required_reaffirmation_by: s.required_reaffirmation_by.as_second(),
            invoked_at: s.invoked_at.as_second(),
        }
    }
}

#[pyclass(name = "ReputationView", get_all)]
#[derive(Clone)]
struct PyReputationView {
    community_did: String,
    as_of: i64,
    standing: String,
    identity_cid: Option<String>,
    intra_rites: Vec<PyIntraRiteSummary>,
    inter_admissions: Vec<PyInterAdmissionSummary>,
    inter_nonadmissions: Vec<PyInterNonAdmissionSummary>,
    active_delegations: Vec<PyDelegationSummary>,
    active_witnesses: Vec<PyWitnessSummary>,
    conduct: Vec<PyConductSummary>,
    sponsorships_given: Vec<PySponsorshipSummary>,
    sponsorships_received: Vec<PySponsorshipSummary>,
    disputes_filed: Vec<PyDisputeSummary>,
    disputes_against: Vec<PyDisputeSummary>,
    designations: Vec<PyDesignationSummary>,
    reentries: Vec<PyReEntrySummary>,
    restitutions: Vec<PyRestitutionSummary>,
    substantive_reengagements: Vec<PySubstantiveReEngagementSummary>,
}

impl From<&rep::ReputationView> for PyReputationView {
    fn from(v: &rep::ReputationView) -> Self {
        Self {
            community_did: v.community_did.to_string(),
            as_of: v.as_of.as_second(),
            standing: v.standing.as_str().to_string(),
            identity_cid: v.identity_cid.as_ref().map(|c| c.to_string()),
            intra_rites: v.intra_rites.iter().map(PyIntraRiteSummary::from).collect(),
            inter_admissions: v
                .inter_admissions
                .iter()
                .map(PyInterAdmissionSummary::from)
                .collect(),
            inter_nonadmissions: v
                .inter_nonadmissions
                .iter()
                .map(PyInterNonAdmissionSummary::from)
                .collect(),
            active_delegations: v
                .active_delegations
                .iter()
                .map(PyDelegationSummary::from)
                .collect(),
            active_witnesses: v
                .active_witnesses
                .iter()
                .map(PyWitnessSummary::from)
                .collect(),
            conduct: v.conduct.iter().map(PyConductSummary::from).collect(),
            sponsorships_given: v
                .sponsorships_given
                .iter()
                .map(PySponsorshipSummary::from)
                .collect(),
            sponsorships_received: v
                .sponsorships_received
                .iter()
                .map(PySponsorshipSummary::from)
                .collect(),
            disputes_filed: v.disputes_filed.iter().map(PyDisputeSummary::from).collect(),
            disputes_against: v
                .disputes_against
                .iter()
                .map(PyDisputeSummary::from)
                .collect(),
            designations: v.designations.iter().map(PyDesignationSummary::from).collect(),
            reentries: v.reentries.iter().map(PyReEntrySummary::from).collect(),
            restitutions: v.restitutions.iter().map(PyRestitutionSummary::from).collect(),
            substantive_reengagements: v
                .substantive_reengagements
                .iter()
                .map(PySubstantiveReEngagementSummary::from)
                .collect(),
        }
    }
}

// ---- Capture-signal analytics bindings -------------------------------------

#[pyclass(name = "SponseeOutcome", get_all)]
#[derive(Clone)]
struct PySponseeOutcome {
    entrant: String,
    sponsorship_cid: String,
    sponsored_at: i64,
    probation_until: i64,
    adverse_conduct: Vec<String>,
}

impl From<&rep::SponseeOutcome> for PySponseeOutcome {
    fn from(s: &rep::SponseeOutcome) -> Self {
        Self {
            entrant: s.entrant.to_string(),
            sponsorship_cid: s.sponsorship_cid.to_string(),
            sponsored_at: s.sponsored_at.as_second(),
            probation_until: s.probation_until.as_second(),
            adverse_conduct: s.adverse_conduct.iter().map(|c| c.to_string()).collect(),
        }
    }
}

#[pyclass(name = "SponsorStakeReport", get_all)]
#[derive(Clone)]
struct PySponsorStakeReport {
    sponsor: String,
    sponsees: Vec<PySponseeOutcome>,
    total_sponsees: usize,
    tainted_sponsees: usize,
    taint_rate: Option<f64>,
}

impl From<&rep::SponsorStakeReport> for PySponsorStakeReport {
    fn from(r: &rep::SponsorStakeReport) -> Self {
        Self {
            sponsor: r.sponsor.to_string(),
            sponsees: r.sponsees.iter().map(PySponseeOutcome::from).collect(),
            total_sponsees: r.total_sponsees(),
            tainted_sponsees: r.tainted_sponsees(),
            taint_rate: r.taint_rate(),
        }
    }
}

#[pyclass(name = "CompositionGraph", get_all)]
#[derive(Clone)]
struct PyCompositionGraph {
    /// (record_cid, sponsor, entrant, sponsored_at, probation_until)
    edges: Vec<(String, String, String, i64, i64)>,
    /// (sponsor_did, out_degree) descending by out-degree — the concentration ranking.
    by_concentration: Vec<(String, usize)>,
}

impl From<&rep::CompositionGraph> for PyCompositionGraph {
    fn from(g: &rep::CompositionGraph) -> Self {
        Self {
            edges: g
                .edges
                .iter()
                .map(|e| {
                    (
                        e.record_cid.to_string(),
                        e.sponsor.to_string(),
                        e.entrant.to_string(),
                        e.sponsored_at.as_second(),
                        e.probation_until.as_second(),
                    )
                })
                .collect(),
            by_concentration: g.by_concentration(),
        }
    }
}

#[pyclass(name = "FeatureConcernProfile", get_all)]
#[derive(Clone)]
struct PyFeatureConcernProfile {
    /// (anti_spiral_feature, adverse_count), feature name as in the Rust enum.
    per_feature: Vec<(String, usize)>,
    non_feature_adverse: usize,
    evidence: Vec<String>,
    perspectives: Vec<String>,
}

impl From<&rep::FeatureConcernProfile> for PyFeatureConcernProfile {
    fn from(p: &rep::FeatureConcernProfile) -> Self {
        Self {
            per_feature: p.per_feature.iter().map(|(k, n)| (k.clone(), *n)).collect(),
            non_feature_adverse: p.non_feature_adverse,
            evidence: p.evidence.iter().map(|c| c.to_string()).collect(),
            perspectives: p.perspectives.clone(),
        }
    }
}

// ---- v0 jester lenses ------------------------------------------------------

#[pyclass(name = "RejectionPattern", get_all)]
#[derive(Clone)]
struct PyRejectionPattern {
    /// (non_admission_cid, rejecting, presenting, feature, decided_at), oldest first.
    events: Vec<(String, String, String, String, i64)>,
    /// (rejecting_did, count)
    by_rejector: Vec<(String, usize)>,
    /// (presenting_did, count)
    by_presenting: Vec<(String, usize)>,
    /// (anti_spiral_feature, count), feature name as in the Rust enum.
    by_feature: Vec<(String, usize)>,
    /// Rejecting communities ordered by descending rejection count.
    by_concentration: Vec<(String, usize)>,
    total_rejections: usize,
    /// Distinct rejecting communities (breadth of source).
    perspectives: Vec<String>,
}

impl From<&rep::RejectionPattern> for PyRejectionPattern {
    fn from(p: &rep::RejectionPattern) -> Self {
        Self {
            events: p
                .events
                .iter()
                .map(|e| {
                    (
                        e.non_admission_cid.to_string(),
                        e.rejecting.to_string(),
                        e.presenting.to_string(),
                        format!("{:?}", e.feature),
                        e.decided_at.as_second(),
                    )
                })
                .collect(),
            by_rejector: p.by_rejector.iter().map(|(k, n)| (k.clone(), *n)).collect(),
            by_presenting: p.by_presenting.iter().map(|(k, n)| (k.clone(), *n)).collect(),
            by_feature: p.by_feature.iter().map(|(k, n)| (k.clone(), *n)).collect(),
            by_concentration: p.by_concentration(),
            total_rejections: p.total_rejections(),
            perspectives: p.perspectives.clone(),
        }
    }
}

#[pyclass(name = "CeremonyCostDecay", get_all)]
#[derive(Clone)]
struct PyCeremonyCostDecay {
    /// (record_cid, sealed_at, witness_count), oldest first.
    rites: Vec<(String, i64, usize)>,
    /// Successive sealed_at deltas in seconds.
    gaps_seconds: Vec<i64>,
    seconds_since_last: Option<i64>,
    rite_count: usize,
    first_witness_count: Option<usize>,
    last_witness_count: Option<usize>,
    min_witness_count: Option<usize>,
    max_witness_count: Option<usize>,
    witness_count_declined: bool,
    latest_gap_widening: Option<bool>,
}

impl From<&rep::CeremonyCostDecay> for PyCeremonyCostDecay {
    fn from(d: &rep::CeremonyCostDecay) -> Self {
        Self {
            rites: d
                .rites
                .iter()
                .map(|r| (r.record_cid.to_string(), r.sealed_at.as_second(), r.witness_count))
                .collect(),
            gaps_seconds: d.gaps_seconds.clone(),
            seconds_since_last: d.seconds_since_last,
            rite_count: d.rite_count(),
            first_witness_count: d.first_witness_count(),
            last_witness_count: d.last_witness_count(),
            min_witness_count: d.min_witness_count(),
            max_witness_count: d.max_witness_count(),
            witness_count_declined: d.witness_count_declined(),
            latest_gap_widening: d.latest_gap_widening(),
        }
    }
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("ALG_ED25519", "Ed25519")?;
    m.add_class::<PyArchive>()?;
    m.add_class::<PyCeremonyResult>()?;
    m.add_class::<PyReputationView>()?;
    m.add_class::<PyIntraRiteSummary>()?;
    m.add_class::<PyInterAdmissionSummary>()?;
    m.add_class::<PyInterNonAdmissionSummary>()?;
    m.add_class::<PyRejectionSummary>()?;
    m.add_class::<PyDelegationSummary>()?;
    m.add_class::<PyWitnessSummary>()?;
    m.add_class::<PyConductSummary>()?;
    m.add_class::<PySponsorshipSummary>()?;
    m.add_class::<PyDisputeSummary>()?;
    m.add_class::<PyDesignationSummary>()?;
    m.add_class::<PyReEntrySummary>()?;
    m.add_class::<PyRestitutionSummary>()?;
    m.add_class::<PySubstantiveReEngagementSummary>()?;
    m.add_class::<PySponseeOutcome>()?;
    m.add_class::<PySponsorStakeReport>()?;
    m.add_class::<PyCompositionGraph>()?;
    m.add_class::<PyFeatureConcernProfile>()?;
    m.add_class::<PyRejectionPattern>()?;
    m.add_class::<PyCeremonyCostDecay>()?;
    m.add_class::<PyCaptureScenario>()?;
    m.add_function(wrap_pyfunction!(generate_ed25519_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(cid_of_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(now_seconds, m)?)?;
    m.add_function(wrap_pyfunction!(run_two_community_ceremony, m)?)?;
    m.add_function(wrap_pyfunction!(demo_capture_scenario, m)?)?;
    Ok(())
}
