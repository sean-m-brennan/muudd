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

//! PyO3 bindings — the Python surface over the ethne tier (decision D2).
//!
//! Shape mirrors kith-covenant's `kc-pyo3`: one `Archive` class carrying the
//! authoring methods and the derived read-models, a handful of module-level
//! functions for keys and for the substrate/monitor/uplift entry points, and
//! `dump_json` for full inspection rather than one `#[pyclass]` per record.
//!
//! Conventions across the surface:
//! - **Timestamps** are `i64` epoch seconds (the archive's own resolution).
//! - **Keys** are raw 32-byte Ed25519 secrets/publics; DIDs are `did:key` strings.
//! - **Co-signers** arrive as parallel lists (`decider_dids` / `decider_privs`),
//!   and the consent co-signature a record needs (the joiner, the office holder,
//!   the inductee) is added by the binding, de-duplicated against those lists.
//! - **Enums** are variant-name strings. `Office` is the exception: the four named
//!   offices match by name and **any other string becomes `Office::Other(s)`** — so
//!   `office="quartermaster"` works, at the cost that a polity cannot express
//!   `Other("Convener")` through this surface.
//! - **KC-facing artifacts** (`CommunityIdentity`, `SuccessorLink`, `ConductRecord`)
//!   cross as JSON strings, which a KC-side consumer appends verbatim.

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyType};

use en_archive::Archive;
use en_charter::{
    affirmation_state, authority_violations, charter_state, check_authority, office_state,
    verify_all, Affirmation, AffirmationKind, AffirmationState, AmendmentKind, CharterDissolution,
    CharterEntry, Commitment, CommitmentAmendment, FoundingProposal, MembershipChange,
    MembershipKind, Office, OfficeChange, OfficeKind, OfficeState, PolityCharter, Substrate,
};
use en_cohesion::{cohesion_report as cohesion_report_rs, CohesionReport, ErosionLevel, Thresholds};
use en_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, PubKey, Signed, Timestamp};
use en_substrate::{FoundingCandidate, ManualFounding, MemberCandidate, SubstrateProvider};
use en_uplift::FusionPredecessor;
use kc_core::CommunityIdentity;

fn en_err_to_py(e: KcError) -> PyErr {
    PyRuntimeError::new_err(format!("ethne: {e}"))
}

fn json_err_to_py(e: serde_json::Error) -> PyErr {
    PyValueError::new_err(format!("json: {e}"))
}

fn parse_did(did: &str) -> PyResult<DidKey> {
    use std::str::FromStr as _;
    DidKey::from_str(did).map_err(|e: KcError| PyValueError::new_err(format!("invalid did: {e}")))
}

fn parse_dids(dids: &[String]) -> PyResult<Vec<DidKey>> {
    dids.iter().map(|d| parse_did(d)).collect()
}

fn parse_cid(s: &str) -> PyResult<Cid> {
    let hex = s.strip_prefix("b3:").unwrap_or(s);
    Cid::from_hex(hex).map_err(|e| PyValueError::new_err(format!("bad cid '{s}': {e}")))
}

fn priv_from_bytes(b: &[u8]) -> PrivKey {
    PrivKey { alg: Alg::Ed25519, bytes: b.to_vec() }
}

fn pub_from_bytes(b: &[u8]) -> PubKey {
    PubKey { alg: Alg::Ed25519, bytes: b.to_vec() }
}

fn ts(secs: i64) -> PyResult<Timestamp> {
    Timestamp::from_second(secs)
        .map_err(|e| PyValueError::new_err(format!("invalid timestamp {secs}: {e}")))
}

/// `at` or, when omitted, now — matching `kc-pyo3`'s optional-timestamp idiom.
fn ts_or_now(at: Option<i64>) -> PyResult<Timestamp> {
    match at {
        Some(s) => ts(s),
        None => Ok(time::now()),
    }
}

/// A record's signer set: DIDs paired with the keys that sign for them. Built
/// from the parallel lists the Python surface takes, then extended with whatever
/// consent co-signature the record needs.
struct Signers {
    dids: Vec<String>,
    keys: Vec<PrivKey>,
}

impl Signers {
    fn new(dids: Vec<String>, privs: Vec<Vec<u8>>) -> PyResult<Self> {
        if dids.len() != privs.len() {
            return Err(PyValueError::new_err(format!(
                "signer lists must be the same length: {} dids, {} keys",
                dids.len(),
                privs.len()
            )));
        }
        // Validate each DID here so a typo surfaces as a ValueError rather than a
        // later verification failure.
        for d in &dids {
            parse_did(d)?;
        }
        let keys = privs.iter().map(|p| priv_from_bytes(p)).collect();
        Ok(Self { dids, keys })
    }

    /// Add a consent co-signature (the joiner, the office holder, the inductee),
    /// unless that DID already signs — overlap with the deciders is legitimate,
    /// but a doubled signature is not.
    fn consent(&mut self, did: &str, priv_bytes: &[u8]) -> PyResult<()> {
        parse_did(did)?;
        if !self.dids.iter().any(|d| d == did) {
            self.dids.push(did.to_string());
            self.keys.push(priv_from_bytes(priv_bytes));
        }
        Ok(())
    }

    /// Only a consent signature, for the self-signed records (voluntary
    /// departure, resignation, re-affirmation).
    fn only(did: &str, priv_bytes: &[u8]) -> PyResult<Self> {
        let mut s = Self { dids: Vec::new(), keys: Vec::new() };
        s.consent(did, priv_bytes)?;
        Ok(s)
    }

    fn pairs(&self) -> Vec<(&str, &PrivKey)> {
        self.dids
            .iter()
            .map(|d| d.as_str())
            .zip(self.keys.iter())
            .collect()
    }
}

fn parse_amendment_kind(s: &str) -> PyResult<AmendmentKind> {
    match s {
        "Add" => Ok(AmendmentKind::Add),
        "Remove" => Ok(AmendmentKind::Remove),
        other => Err(PyValueError::new_err(format!(
            "unknown amendment kind '{other}' (expected Add or Remove)"
        ))),
    }
}

/// The four named offices match by name; anything else is a polity-specific
/// `Office::Other`.
fn parse_office(s: &str) -> Office {
    match s {
        "Convener" => Office::Convener,
        "Inductor" => Office::Inductor,
        "EnvoyAuthorizer" => Office::EnvoyAuthorizer,
        "CharterKeeper" => Office::CharterKeeper,
        other => Office::Other(other.to_string()),
    }
}

fn office_name(o: &Office) -> String {
    match o {
        Office::Convener => "Convener".into(),
        Office::Inductor => "Inductor".into(),
        Office::EnvoyAuthorizer => "EnvoyAuthorizer".into(),
        Office::CharterKeeper => "CharterKeeper".into(),
        Office::Other(s) => s.clone(),
    }
}

fn commitments_from(pairs: Vec<(String, String)>) -> Vec<Commitment> {
    pairs
        .into_iter()
        .map(|(key, description)| Commitment { key, description })
        .collect()
}

/// Build a founding **proposal** — the emergent boundary offered back to the
/// members. It carries no authority: only `Archive.ratify_charter` (a majority of
/// the roll co-signing) turns it into a charter. Returned as JSON.
#[pyfunction]
#[pyo3(signature = (
    polity_did,
    substrate_name,
    substrate_statement,
    member_dids,
    substrate_anchor=None,
    commitments=Vec::new(),
    proposed_at=None,
))]
fn propose_charter(
    polity_did: &str,
    substrate_name: String,
    substrate_statement: String,
    member_dids: Vec<String>,
    substrate_anchor: Option<String>,
    commitments: Vec<(String, String)>,
    proposed_at: Option<i64>,
) -> PyResult<String> {
    let anchor = substrate_anchor.as_deref().map(parse_did).transpose()?;
    let proposal = FoundingProposal {
        polity: parse_did(polity_did)?,
        substrate: Substrate {
            name: substrate_name,
            statement: substrate_statement,
            anchor,
        },
        founding_commitments: commitments_from(commitments),
        proposed_members: parse_dids(&member_dids)?,
        proposed_at: ts_or_now(proposed_at)?,
    };
    serde_json::to_string(&proposal).map_err(json_err_to_py)
}

/// Generate an Ed25519 keypair, returning `(priv, pub, did)`.
#[pyfunction]
fn generate_ed25519_keypair(
    py: Python<'_>,
) -> PyResult<(Bound<'_, PyBytes>, Bound<'_, PyBytes>, String)> {
    let (sk, pk) = keys::generate(Alg::Ed25519).map_err(en_err_to_py)?;
    let did = DidKey::from_pubkey(&pk).map_err(en_err_to_py)?;
    Ok((
        PyBytes::new_bound(py, &sk.bytes),
        PyBytes::new_bound(py, &pk.bytes),
        did.into_string(),
    ))
}

/// Blake3 content address of a byte string, in `b3:<hex>` form.
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

// ---- Substrate: the emergent boundary a founding ratifies -------------------

/// What a substrate provider currently reads: the candidate roll (each member
/// with an earned reputation and an optional guardian edge) and, optionally, the
/// substrate the provider observes. A candidate is *input to a proposal*, never
/// an authority in itself.
#[pyclass(name = "FoundingCandidate")]
#[derive(Clone)]
struct PyFoundingCandidate {
    inner: FoundingCandidate,
}

#[pymethods]
impl PyFoundingCandidate {
    /// Candidate members as `(did, reputation, guardian_did | None)`.
    #[getter]
    fn members(&self) -> Vec<(String, f64, Option<String>)> {
        self.inner
            .members
            .iter()
            .map(|m| {
                (
                    m.did.to_string(),
                    m.reputation,
                    m.guardian.as_ref().map(|g| g.to_string()),
                )
            })
            .collect()
    }

    #[getter]
    fn substrate_name(&self) -> Option<String> {
        self.inner.substrate.as_ref().map(|s| s.name.clone())
    }

    #[getter]
    fn substrate_statement(&self) -> Option<String> {
        self.inner.substrate.as_ref().map(|s| s.statement.clone())
    }

    #[getter]
    fn substrate_anchor(&self) -> Option<String> {
        self.inner
            .substrate
            .as_ref()
            .and_then(|s| s.anchor.as_ref())
            .map(|d| d.to_string())
    }

    /// A member's reputation as this candidate reads it, if known.
    fn member_reputation(&self, did: &str) -> PyResult<Option<f64>> {
        let d = parse_did(did)?;
        Ok(self
            .inner
            .members
            .iter()
            .find(|m| m.did == d)
            .map(|m| m.reputation))
    }
}

fn substrate_from(
    name: Option<String>,
    statement: Option<String>,
    anchor: Option<String>,
) -> PyResult<Option<Substrate>> {
    match (name, statement) {
        (Some(name), Some(statement)) => Ok(Some(Substrate {
            name,
            statement,
            anchor: anchor.as_deref().map(parse_did).transpose()?,
        })),
        (None, None) => Ok(None),
        _ => Err(PyValueError::new_err(
            "a substrate needs both a name and a statement",
        )),
    }
}

/// Build a candidate directly from observed members — the shape any substrate
/// provider reduces to.
#[pyfunction]
#[pyo3(signature = (members, substrate_name=None, substrate_statement=None, substrate_anchor=None))]
fn founding_candidate(
    members: Vec<(String, f64, Option<String>)>,
    substrate_name: Option<String>,
    substrate_statement: Option<String>,
    substrate_anchor: Option<String>,
) -> PyResult<PyFoundingCandidate> {
    let mut out = Vec::with_capacity(members.len());
    for (did, reputation, guardian) in members {
        out.push(MemberCandidate {
            did: parse_did(&did)?,
            reputation,
            guardian: guardian.as_deref().map(parse_did).transpose()?,
        });
    }
    Ok(PyFoundingCandidate {
        inner: FoundingCandidate {
            members: out,
            substrate: substrate_from(substrate_name, substrate_statement, substrate_anchor)?,
        },
    })
}

/// The convener-driven provider: a polity founded with no substrate service at
/// all. Every asserted member reads at full reputation.
#[pyfunction]
#[pyo3(signature = (member_dids, substrate_name, substrate_statement, substrate_anchor=None))]
fn manual_founding(
    member_dids: Vec<String>,
    substrate_name: String,
    substrate_statement: String,
    substrate_anchor: Option<String>,
) -> PyResult<PyFoundingCandidate> {
    let substrate = substrate_from(Some(substrate_name), Some(substrate_statement), substrate_anchor)?
        .expect("both parts supplied");
    let provider = ManualFounding::new(parse_dids(&member_dids)?, substrate);
    let inner = provider.founding_candidate().map_err(en_err_to_py)?;
    Ok(PyFoundingCandidate { inner })
}

/// Offer a candidate back to its members as a founding proposal, trimming the
/// roll to those at or above `min_reputation`. The substrate defaults to the one
/// the candidate observed; pass a name and statement to override it (the provider
/// never dictates what a polity stands for). Returned as JSON, for
/// `Archive.ratify_charter`.
#[pyfunction]
#[pyo3(signature = (
    candidate,
    polity_did,
    substrate_name=None,
    substrate_statement=None,
    substrate_anchor=None,
    commitments=Vec::new(),
    min_reputation=0.0,
    proposed_at=None,
))]
#[allow(clippy::too_many_arguments)]
fn propose_from_candidate(
    candidate: &PyFoundingCandidate,
    polity_did: &str,
    substrate_name: Option<String>,
    substrate_statement: Option<String>,
    substrate_anchor: Option<String>,
    commitments: Vec<(String, String)>,
    min_reputation: f64,
    proposed_at: Option<i64>,
) -> PyResult<String> {
    let substrate = match substrate_from(substrate_name, substrate_statement, substrate_anchor)? {
        Some(s) => s,
        None => candidate.inner.substrate.clone().ok_or_else(|| {
            PyValueError::new_err(
                "the candidate observed no substrate; pass substrate_name and substrate_statement",
            )
        })?,
    };
    let proposal = candidate.inner.propose(
        parse_did(polity_did)?,
        substrate,
        commitments_from(commitments),
        min_reputation,
        ts_or_now(proposed_at)?,
    );
    serde_json::to_string(&proposal).map_err(json_err_to_py)
}

// ---- Cohesion: the coherence monitor ----------------------------------------

fn erosion_name(e: ErosionLevel) -> &'static str {
    match e {
        ErosionLevel::None => "None",
        ErosionLevel::Notice => "Notice",
        ErosionLevel::Concern => "Concern",
    }
}

/// How far a polity has drifted from the roll it chartered. Holds the Rust
/// report, so `coherence_concern` sees the same values the getters show.
#[pyclass(name = "CohesionReport")]
#[derive(Clone)]
struct PyCohesionReport {
    inner: CohesionReport,
}

#[pymethods]
impl PyCohesionReport {
    /// Share of the chartered roll no longer observed, in `[0, 1]`.
    #[getter]
    fn attrition(&self) -> f64 {
        self.inner.attrition
    }

    #[getter]
    fn reputation_floor_breaches(&self) -> usize {
        self.inner.reputation_floor_breaches
    }

    #[getter]
    fn mean_reputation(&self) -> f64 {
        self.inner.mean_reputation
    }

    /// True when the substrate's edge is partly opaque, which caps the attrition
    /// component at `Notice` (a disappearance behind an opaque edge is not a
    /// departure).
    #[getter]
    fn opaque_boundary(&self) -> bool {
        self.inner.opaque_boundary
    }

    #[getter]
    fn observed(&self) -> usize {
        self.inner.observed
    }

    #[getter]
    fn chartered(&self) -> usize {
        self.inner.chartered
    }

    /// `"None"`, `"Notice"`, or `"Concern"`.
    #[getter]
    fn erosion(&self) -> &'static str {
        erosion_name(self.inner.erosion)
    }
}

/// Grade the strain between a chartered roll and what the substrate currently
/// shows. Thresholds default to `en-cohesion`'s; override any of them by keyword.
/// The monitor never decides a split — a `Concern` is an early warning.
#[pyfunction]
#[pyo3(signature = (
    chartered_roll,
    candidate,
    opaque_boundary=false,
    reputation_floor=None,
    attrition_notice=None,
    attrition_concern=None,
    mean_reputation_notice=None,
    mean_reputation_concern=None,
    breach_notice=None,
    breach_concern=None,
))]
#[allow(clippy::too_many_arguments)]
fn cohesion_report(
    chartered_roll: Vec<String>,
    candidate: &PyFoundingCandidate,
    opaque_boundary: bool,
    reputation_floor: Option<f64>,
    attrition_notice: Option<f64>,
    attrition_concern: Option<f64>,
    mean_reputation_notice: Option<f64>,
    mean_reputation_concern: Option<f64>,
    breach_notice: Option<f64>,
    breach_concern: Option<f64>,
) -> PyResult<PyCohesionReport> {
    let d = Thresholds::default();
    let thresholds = Thresholds {
        reputation_floor: reputation_floor.unwrap_or(d.reputation_floor),
        attrition_notice: attrition_notice.unwrap_or(d.attrition_notice),
        attrition_concern: attrition_concern.unwrap_or(d.attrition_concern),
        mean_reputation_notice: mean_reputation_notice.unwrap_or(d.mean_reputation_notice),
        mean_reputation_concern: mean_reputation_concern.unwrap_or(d.mean_reputation_concern),
        breach_notice: breach_notice.unwrap_or(d.breach_notice),
        breach_concern: breach_concern.unwrap_or(d.breach_concern),
    };
    let inner = cohesion_report_rs(
        &parse_dids(&chartered_roll)?,
        &candidate.inner,
        opaque_boundary,
        &thresholds,
    );
    Ok(PyCohesionReport { inner })
}

// ---- Uplift: the interface to kith-covenant ---------------------------------

/// The bundle a fission emits, each artifact as JSON a KC-side consumer appends
/// verbatim.
#[pyclass(name = "FissionOutcome", get_all)]
#[derive(Clone)]
struct PyFissionOutcome {
    successor_a: String,
    successor_b: String,
    link_a: String,
    link_b: String,
    dissolution: String,
}

/// The bundle a fusion emits: the merged identity, plus one successor link and
/// one dissolution per predecessor, index-aligned with the input list.
#[pyclass(name = "FusionOutcome", get_all)]
#[derive(Clone)]
struct PyFusionOutcome {
    fused: String,
    links: Vec<String>,
    dissolutions: Vec<String>,
}

/// The Concern: if — and only if — `report` has reached `Concern`-level erosion,
/// a signed KC conduct record about the polity, as JSON. Below that level the
/// strain is ethne's own business and nothing crosses the boundary (`None`). The
/// record is filed with an explicit monitor perspective, so KC discounts it as a
/// partial signal rather than a neutral finding.
#[pyfunction]
#[pyo3(signature = (report, subject_did, observer_did, observer_priv, at=None))]
fn coherence_concern(
    report: &PyCohesionReport,
    subject_did: &str,
    observer_did: &str,
    observer_priv: &[u8],
    at: Option<i64>,
) -> PyResult<Option<String>> {
    let record = en_uplift::coherence_concern(
        &report.inner,
        parse_did(subject_did)?,
        parse_did(observer_did)?,
        &priv_from_bytes(observer_priv),
        ts_or_now(at)?,
    )
    .map_err(en_err_to_py)?;
    match record {
        Some(signed) => Ok(Some(serde_json::to_string(&signed).map_err(json_err_to_py)?)),
        None => Ok(None),
    }
}

/// A polity's charter log: an append-only, content-addressed archive of
/// [`CharterEntry`] records, plus the authoring methods that put records into it
/// and the queries that read state back out.
#[pyclass(name = "Archive")]
struct PyArchive {
    inner: Archive<CharterEntry>,
}

impl PyArchive {
    fn append(&mut self, entry: CharterEntry) -> PyResult<String> {
        self.inner
            .append(entry)
            .map(|c| c.to_string())
            .map_err(en_err_to_py)
    }

    /// The genesis charter of `polity`, for the uplift calls that need it.
    fn charter_of(&self, polity: &DidKey) -> PyResult<PolityCharter> {
        self.inner
            .iter()
            .find_map(|(_, e)| match e {
                CharterEntry::PolityChartered(s) if &s.payload.polity == polity => {
                    Some(s.payload.clone())
                }
                _ => None,
            })
            .ok_or_else(|| PyValueError::new_err(format!("no charter for {polity} in this archive")))
    }
}

fn parse_identity(json: &str) -> PyResult<Signed<CommunityIdentity>> {
    serde_json::from_str(json)
        .map_err(|e| PyValueError::new_err(format!("not a signed community identity: {e}")))
}

/// Serialize an emitted artifact for the Python side.
fn to_json<T: serde::Serialize>(value: &T) -> PyResult<String> {
    serde_json::to_string(value).map_err(json_err_to_py)
}

#[pymethods]
impl PyArchive {
    #[new]
    fn new() -> Self {
        Self { inner: Archive::new() }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Entry CIDs in append order.
    fn cids(&self) -> Vec<String> {
        self.inner.iter().map(|(c, _)| c.to_string()).collect()
    }

    /// `(cid, kind)` for every entry, in append order.
    fn entries(&self) -> Vec<(String, String)> {
        use en_archive::Entry as _;
        self.inner
            .iter()
            .map(|(c, e)| (c.to_string(), e.kind().to_string()))
            .collect()
    }

    /// Whether an entry with this CID is present.
    fn contains(&self, cid: &str) -> PyResult<bool> {
        Ok(self.inner.contains(&parse_cid(cid)?))
    }

    /// The whole log as a JSON array of entries — full inspection from Python
    /// without a wrapper class per record.
    fn dump_json(&self) -> PyResult<String> {
        let entries: Vec<&CharterEntry> = self.inner.iter().map(|(_, e)| e).collect();
        serde_json::to_string(&entries).map_err(json_err_to_py)
    }

    /// Rebuild an archive from `dump_json` output, preserving append order.
    #[classmethod]
    fn load_json(_cls: &Bound<'_, PyType>, s: &str) -> PyResult<Self> {
        let entries: Vec<CharterEntry> = serde_json::from_str(s).map_err(json_err_to_py)?;
        let mut inner = Archive::new();
        for e in entries {
            inner.append(e).map_err(en_err_to_py)?;
        }
        Ok(Self { inner })
    }

    /// Crypto-verify every signature in the log. Raises on the first failure.
    fn verify_all(&self) -> PyResult<()> {
        verify_all(&self.inner).map_err(en_err_to_py)
    }

    /// Enforce the charter authority rules (genesis singleton, majority
    /// ratification, genesis-immutable core, asymmetric amendment, the membership
    /// / office / affirmation lifecycles, terminal dissolution). A separate pass
    /// from `verify_all`, which is purely cryptographic; raises on the first
    /// violation.
    fn check_authority(&self) -> PyResult<()> {
        check_authority(&self.inner).map_err(en_err_to_py)
    }

    /// Every authority violation as `(cid, reason)`, instead of raising.
    fn authority_violations(&self) -> Vec<(String, String)> {
        authority_violations(&self.inner)
            .into_iter()
            .map(|(c, m)| (c.to_string(), m))
            .collect()
    }

    // ---- Founding ---------------------------------------------------------

    /// Ratify a proposal (from `propose_charter` / `propose_from_candidate`) into
    /// the polity's genesis charter. The ratifiers must be a majority of the
    /// proposed roll and must be exactly the signers — emergence proposes, the
    /// polity ratifies. Returns the entry CID.
    #[pyo3(signature = (proposal_json, ratifier_dids, ratifier_privs, founded_at=None))]
    fn ratify_charter(
        &mut self,
        proposal_json: &str,
        ratifier_dids: Vec<String>,
        ratifier_privs: Vec<Vec<u8>>,
        founded_at: Option<i64>,
    ) -> PyResult<String> {
        let proposal: FoundingProposal =
            serde_json::from_str(proposal_json).map_err(json_err_to_py)?;
        let signers = Signers::new(ratifier_dids, ratifier_privs)?;
        let charter = proposal
            .ratify(parse_dids(&signers.dids)?, ts_or_now(founded_at)?)
            .map_err(en_err_to_py)?;
        let signed = Signed::build(charter, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::PolityChartered(signed))
    }

    // ---- Commitment amendment --------------------------------------------

    /// Add or remove a commitment (`kind` is `"Add"` or `"Remove"`). Add needs a
    /// majority of the live roll; Remove needs a majority + 1 and cannot touch a
    /// founding commitment.
    #[pyo3(signature = (
        polity_did, kind, key, description=String::new(), rationale=String::new(),
        decider_dids=Vec::new(), decider_privs=Vec::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn amend_commitment(
        &mut self,
        polity_did: &str,
        kind: &str,
        key: String,
        description: String,
        rationale: String,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = CommitmentAmendment {
            polity: parse_did(polity_did)?,
            kind: parse_amendment_kind(kind)?,
            commitment: Commitment { key, description },
            rationale,
            deciders: parse_dids(&signers.dids)?,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::CommitmentAmended(signed))
    }

    // ---- Membership lifecycle --------------------------------------------

    /// Admit a member: a majority of the live roll decides, and the joiner
    /// co-signs its own consent (no conscription).
    #[pyo3(signature = (
        polity_did, member_did, member_priv, decider_dids, decider_privs,
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn admit_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(decider_dids, decider_privs)?;
        let deciders = parse_dids(&signers.dids)?;
        signers.consent(member_did, member_priv)?;
        let record = MembershipChange {
            polity: parse_did(polity_did)?,
            kind: MembershipKind::Admit,
            member: parse_did(member_did)?,
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MembershipChanged(signed))
    }

    /// A member leaves of their own accord: self-signed, no deciders (the roll
    /// cannot force them to stay, subject only to the self-govern floor).
    #[pyo3(signature = (polity_did, member_did, member_priv, rationale=String::new(), decided_at=None))]
    fn depart_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(member_did, member_priv)?;
        let record = MembershipChange {
            polity: parse_did(polity_did)?,
            kind: MembershipKind::DepartVoluntary,
            member: parse_did(member_did)?,
            rationale,
            deciders: vec![],
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MembershipChanged(signed))
    }

    /// Expel a member: a majority **+ 1** of the live roll, with the target
    /// excluded from the deciders.
    #[pyo3(signature = (
        polity_did, member_did, decider_dids, decider_privs,
        rationale=String::new(), decided_at=None,
    ))]
    fn expel_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = MembershipChange {
            polity: parse_did(polity_did)?,
            kind: MembershipKind::Expel,
            member: parse_did(member_did)?,
            rationale,
            deciders: parse_dids(&signers.dids)?,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MembershipChanged(signed))
    }

    // ---- Office lifecycle ------------------------------------------------

    /// Confer an office: a majority of the live roll decides and the holder
    /// co-signs (no conscription into duty). `office` is one of `Convener`,
    /// `Inductor`, `EnvoyAuthorizer`, `CharterKeeper`, or any other string, which
    /// becomes a polity-specific office.
    #[pyo3(signature = (
        polity_did, office, holder_did, holder_priv, decider_dids, decider_privs,
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn confer_office(
        &mut self,
        polity_did: &str,
        office: &str,
        holder_did: &str,
        holder_priv: &[u8],
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(decider_dids, decider_privs)?;
        let deciders = parse_dids(&signers.dids)?;
        signers.consent(holder_did, holder_priv)?;
        let record = OfficeChange {
            polity: parse_did(polity_did)?,
            kind: OfficeKind::Confer,
            office: parse_office(office),
            holder: parse_did(holder_did)?,
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::OfficeChanged(signed))
    }

    /// The holder lays an office down: self-signed, no deciders.
    #[pyo3(signature = (polity_did, office, holder_did, holder_priv, rationale=String::new(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn resign_office(
        &mut self,
        polity_did: &str,
        office: &str,
        holder_did: &str,
        holder_priv: &[u8],
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(holder_did, holder_priv)?;
        let record = OfficeChange {
            polity: parse_did(polity_did)?,
            kind: OfficeKind::Resign,
            office: parse_office(office),
            holder: parse_did(holder_did)?,
            rationale,
            deciders: vec![],
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::OfficeChanged(signed))
    }

    /// The roll takes an office back: a majority with the holder excluded.
    #[pyo3(signature = (
        polity_did, office, holder_did, decider_dids, decider_privs,
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn revoke_office(
        &mut self,
        polity_did: &str,
        office: &str,
        holder_did: &str,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = OfficeChange {
            polity: parse_did(polity_did)?,
            kind: OfficeKind::Revoke,
            office: parse_office(office),
            holder: parse_did(holder_did)?,
            rationale,
            deciders: parse_dids(&signers.dids)?,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::OfficeChanged(signed))
    }

    // ---- Transmission: induction and re-affirmation -----------------------

    /// Induct a member into the substrate — the step admission does *not* perform.
    /// A majority of the roll witnesses (the community receives them) and the
    /// inductee co-signs (they participate). Founding members need no induction:
    /// ratifying the charter was theirs.
    #[pyo3(signature = (
        polity_did, member_did, member_priv, witness_dids, witness_privs, affirmed_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn induct_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        witness_dids: Vec<String>,
        witness_privs: Vec<Vec<u8>>,
        affirmed_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(witness_dids, witness_privs)?;
        let witnesses = parse_dids(&signers.dids)?;
        signers.consent(member_did, member_priv)?;
        let record = Affirmation {
            polity: parse_did(polity_did)?,
            member: parse_did(member_did)?,
            kind: AffirmationKind::Induction,
            witnesses,
            affirmed_at: ts_or_now(affirmed_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MemberAffirmed(signed))
    }

    /// Renew an inducted member's affirmation: self-signed, unwitnessed. The
    /// freshness of these is what `AffirmationState.stale_members` reads.
    #[pyo3(signature = (polity_did, member_did, member_priv, affirmed_at=None))]
    fn reaffirm_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        affirmed_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(member_did, member_priv)?;
        let record = Affirmation {
            polity: parse_did(polity_did)?,
            member: parse_did(member_did)?,
            kind: AffirmationKind::ReAffirmation,
            witnesses: vec![],
            affirmed_at: ts_or_now(affirmed_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MemberAffirmed(signed))
    }

    // ---- Dissolution ------------------------------------------------------

    /// Wind the polity up, optionally naming successors. Terminal: no charter
    /// event is authorized afterwards. Needs a majority of the live roll.
    #[pyo3(signature = (
        polity_did, decider_dids, decider_privs, successors=Vec::new(),
        rationale=String::new(), dissolved_at=None,
    ))]
    fn dissolve_charter(
        &mut self,
        polity_did: &str,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        successors: Vec<String>,
        rationale: String,
        dissolved_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = CharterDissolution {
            polity: parse_did(polity_did)?,
            successors: parse_dids(&successors)?,
            rationale,
            deciders: parse_dids(&signers.dids)?,
            dissolved_at: ts_or_now(dissolved_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::CharterDissolved(signed))
    }

    // ---- Derived state ----------------------------------------------------

    /// The polity's charter state as of `as_of` (default: now) — the live roll,
    /// commitments in force, substrate, founding, and any dissolution.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn charter_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyCharterState> {
        let st = charter_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyCharterState::from(&st))
    }

    /// Who holds which office as of `as_of` — raw grants intersected with the
    /// live roll, so a holding lapses while its holder is off the roll.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn office_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyOfficeState> {
        let st = office_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyOfficeState { inner: st })
    }

    /// Each current member's induction status and last affirmation as of `as_of`.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn affirmation_state(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyAffirmationState> {
        let st = affirmation_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyAffirmationState { inner: st })
    }

    // ---- Uplift to kith-covenant -----------------------------------------

    /// Hand the chartered polity off as a self-signed `CommunityIdentity` KC can
    /// recognize, returned as JSON. The root keypair must be the polity's own —
    /// its DID is derived from `root_pub`, and a mismatch with the charter is an
    /// error.
    #[pyo3(signature = (polity_did, name, root_priv, root_pub, created_at=None))]
    fn community_identity(
        &self,
        polity_did: &str,
        name: String,
        root_priv: &[u8],
        root_pub: &[u8],
        created_at: Option<i64>,
    ) -> PyResult<String> {
        let charter = self.charter_of(&parse_did(polity_did)?)?;
        let signed = en_uplift::community_identity(
            name,
            &charter,
            &priv_from_bytes(root_priv),
            pub_from_bytes(root_pub),
            ts_or_now(created_at)?,
        )
        .map_err(en_err_to_py)?;
        serde_json::to_string(&signed).map_err(json_err_to_py)
    }

    /// Emit a **ratified** split: the community has already chartered two
    /// successors (pass their identities as JSON); this signs, under the
    /// predecessor's root, a successor link to each plus the predecessor's
    /// dissolution naming both. Nothing is appended here — to put the dissolution
    /// in this log and have it pass the authority rules, the deciders must
    /// co-sign it through `dissolve_charter`.
    #[pyo3(signature = (
        polity_did, successor_a_json, successor_b_json, root_priv,
        decider_dids=Vec::new(), rationale=String::new(), at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn fission(
        &self,
        polity_did: &str,
        successor_a_json: &str,
        successor_b_json: &str,
        root_priv: &[u8],
        decider_dids: Vec<String>,
        rationale: String,
        at: Option<i64>,
    ) -> PyResult<PyFissionOutcome> {
        let charter = self.charter_of(&parse_did(polity_did)?)?;
        let outcome = en_uplift::fission(
            &charter,
            parse_identity(successor_a_json)?,
            parse_identity(successor_b_json)?,
            parse_dids(&decider_dids)?,
            &priv_from_bytes(root_priv),
            rationale,
            ts_or_now(at)?,
        )
        .map_err(en_err_to_py)?;
        Ok(PyFissionOutcome {
            successor_a: to_json(&outcome.successor_a)?,
            successor_b: to_json(&outcome.successor_b)?,
            link_a: to_json(&outcome.links[0])?,
            link_b: to_json(&outcome.links[1])?,
            dissolution: to_json(&outcome.dissolution)?,
        })
    }

    /// Emit a **ratified** merger: N (≥ 2) predecessors chartered in this log
    /// merge into the already-chartered `fused` identity (JSON). Each entry of
    /// `predecessors` is `(polity_did, root_priv, decider_dids)`; the returned
    /// links and dissolutions are index-aligned with it. As with `fission`,
    /// nothing is appended.
    #[pyo3(signature = (predecessors, fused_json, rationale=String::new(), at=None))]
    fn fusion(
        &self,
        predecessors: Vec<(String, Vec<u8>, Vec<String>)>,
        fused_json: &str,
        rationale: String,
        at: Option<i64>,
    ) -> PyResult<PyFusionOutcome> {
        // Own the charters and keys first, then borrow them into the predecessor
        // structs the Rust API takes.
        let mut owned: Vec<(PolityCharter, PrivKey, Vec<DidKey>)> =
            Vec::with_capacity(predecessors.len());
        for (did, root_priv, decider_dids) in &predecessors {
            owned.push((
                self.charter_of(&parse_did(did)?)?,
                priv_from_bytes(root_priv),
                parse_dids(decider_dids)?,
            ));
        }
        let parties: Vec<FusionPredecessor> = owned
            .iter()
            .map(|(charter, root_priv, deciders)| FusionPredecessor {
                charter,
                root_priv,
                deciders: deciders.clone(),
            })
            .collect();
        let outcome = en_uplift::fusion(
            &parties,
            parse_identity(fused_json)?,
            rationale,
            ts_or_now(at)?,
        )
        .map_err(en_err_to_py)?;
        Ok(PyFusionOutcome {
            fused: to_json(&outcome.fused)?,
            links: outcome
                .links
                .iter()
                .map(to_json)
                .collect::<PyResult<Vec<String>>>()?,
            dissolutions: outcome
                .dissolutions
                .iter()
                .map(to_json)
                .collect::<PyResult<Vec<String>>>()?,
        })
    }
}

/// A polity's charter state at a point in time.
#[pyclass(name = "CharterState", get_all)]
#[derive(Clone)]
struct PyCharterState {
    chartered: bool,
    substrate_name: Option<String>,
    substrate_statement: Option<String>,
    substrate_anchor: Option<String>,
    members: Vec<String>,
    /// Commitments in force, as `(key, description)`.
    commitments: Vec<(String, String)>,
    founded_at: Option<i64>,
    dissolved: bool,
    successors: Vec<String>,
    dissolved_at: Option<i64>,
}

impl From<&en_charter::CharterState> for PyCharterState {
    fn from(s: &en_charter::CharterState) -> Self {
        Self {
            chartered: s.chartered,
            substrate_name: s.substrate.as_ref().map(|x| x.name.clone()),
            substrate_statement: s.substrate.as_ref().map(|x| x.statement.clone()),
            substrate_anchor: s
                .substrate
                .as_ref()
                .and_then(|x| x.anchor.as_ref())
                .map(|d| d.to_string()),
            members: s.members.iter().map(|d| d.to_string()).collect(),
            commitments: s
                .commitments
                .iter()
                .map(|c| (c.key.clone(), c.description.clone()))
                .collect(),
            founded_at: s.founded_at.map(|t| t.as_second()),
            dissolved: s.dissolved,
            successors: s.successors.iter().map(|d| d.to_string()).collect(),
            dissolved_at: s.dissolved_at.map(|t| t.as_second()),
        }
    }
}

/// Effective office holdings at a point in time.
#[pyclass(name = "OfficeState")]
struct PyOfficeState {
    inner: OfficeState,
}

#[pymethods]
impl PyOfficeState {
    /// Every effective holding as `(office, holder_did)`.
    #[getter]
    fn holdings(&self) -> Vec<(String, String)> {
        self.inner
            .holdings
            .iter()
            .map(|h| (office_name(&h.office), h.holder.to_string()))
            .collect()
    }

    /// Whether `did` holds `office`.
    fn holds(&self, did: &str, office: &str) -> PyResult<bool> {
        Ok(self.inner.holds(&parse_did(did)?, &parse_office(office)))
    }

    /// Everyone holding `office` (an office may have several holders).
    fn holders(&self, office: &str) -> Vec<String> {
        self.inner
            .holders(&parse_office(office))
            .into_iter()
            .map(|d| d.to_string())
            .collect()
    }

    /// Every office `did` holds.
    fn offices_of(&self, did: &str) -> PyResult<Vec<String>> {
        Ok(self
            .inner
            .offices_of(&parse_did(did)?)
            .into_iter()
            .map(office_name)
            .collect())
    }
}

/// The transmission read-model: induction status and affirmation freshness.
#[pyclass(name = "AffirmationState")]
struct PyAffirmationState {
    inner: AffirmationState,
}

#[pymethods]
impl PyAffirmationState {
    /// Every current member as `(did, inducted, last_affirmed_at | None)`.
    #[getter]
    fn entries(&self) -> Vec<(String, bool, Option<i64>)> {
        self.inner
            .entries
            .iter()
            .map(|e| {
                (
                    e.member.to_string(),
                    e.inducted,
                    e.last_affirmed_at.map(|t| t.as_second()),
                )
            })
            .collect()
    }

    /// Whether `did` has acquired the substrate (a founding member, or one with
    /// an induction record).
    fn is_inducted(&self, did: &str) -> PyResult<bool> {
        Ok(self.inner.is_inducted(&parse_did(did)?))
    }

    /// When `did` last affirmed, if ever.
    fn last_affirmed(&self, did: &str) -> PyResult<Option<i64>> {
        Ok(self
            .inner
            .last_affirmed(&parse_did(did)?)
            .map(|t| t.as_second()))
    }

    /// Members who are un-inducted or whose last affirmation is older than
    /// `cadence_seconds` — the charter-side erosion signal.
    #[pyo3(signature = (cadence_seconds, as_of=None))]
    fn stale_members(&self, cadence_seconds: i64, as_of: Option<i64>) -> PyResult<Vec<String>> {
        Ok(self
            .inner
            .stale_members(cadence_seconds, ts_or_now(as_of)?)
            .into_iter()
            .map(|d| d.to_string())
            .collect())
    }
}

/// `en-cohesion`'s own threshold defaults, so the Python side never carries a
/// second copy of them.
fn default_thresholds(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let d = Thresholds::default();
    let out = PyDict::new_bound(py);
    out.set_item("reputation_floor", d.reputation_floor)?;
    out.set_item("attrition_notice", d.attrition_notice)?;
    out.set_item("attrition_concern", d.attrition_concern)?;
    out.set_item("mean_reputation_notice", d.mean_reputation_notice)?;
    out.set_item("mean_reputation_concern", d.mean_reputation_concern)?;
    out.set_item("breach_notice", d.breach_notice)?;
    out.set_item("breach_concern", d.breach_concern)?;
    Ok(out)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("ALG_ED25519", "Ed25519")?;
    m.add("DEFAULT_THRESHOLDS", default_thresholds(m.py())?)?;
    m.add_class::<PyArchive>()?;
    m.add_class::<PyCharterState>()?;
    m.add_class::<PyOfficeState>()?;
    m.add_class::<PyAffirmationState>()?;
    m.add_class::<PyFoundingCandidate>()?;
    m.add_class::<PyCohesionReport>()?;
    m.add_class::<PyFissionOutcome>()?;
    m.add_class::<PyFusionOutcome>()?;
    m.add_function(wrap_pyfunction!(generate_ed25519_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(cid_of_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(now_seconds, m)?)?;
    m.add_function(wrap_pyfunction!(propose_charter, m)?)?;
    m.add_function(wrap_pyfunction!(founding_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(manual_founding, m)?)?;
    m.add_function(wrap_pyfunction!(propose_from_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(cohesion_report, m)?)?;
    m.add_function(wrap_pyfunction!(coherence_concern, m)?)?;
    Ok(())
}
