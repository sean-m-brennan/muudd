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
    InterCredentialingOutcome, InterCredentialingRunner, ParticipantContext, REQUIRED_FEATURES,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, Signed, Timestamp};
use kc_identity::{
    CommunityIdentity, DelegationScope, SpeakerDelegation, WitnessEntry, WitnessRegistry,
    WitnessRole,
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
    m.add_function(wrap_pyfunction!(generate_ed25519_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(cid_of_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(run_two_community_ceremony, m)?)?;
    Ok(())
}
