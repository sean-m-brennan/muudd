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

use en_archive::{Archive, Cursor};
use en_charter::{
    affirmation_state, apply_prunings, authority_violations, charter_state, check_authority,
    decay_trends, envoy_state, guardian_state, observation_window, office_state,
    founding_dissent as founding_dissent_rs, founding_legitimacy as founding_legitimacy_rs, prunable_observations,
    independent_corroboration as independent_corroboration_rs,
    reader_independence as reader_independence_rs, substrate_disavowal as substrate_disavowal_rs,
    prune_recommendation as prune_recommendation_rs, pruning_digest,
    verify_all, Affirmation,
    AffirmationKind, AffirmationState, AmendmentKind, CharterDissolution, CharterEntry, Commitment,
    CommitmentAmendment, DecayTrends, Derivation, EnvoyAuthorization, EnvoyKind, EnvoyState,
    FoundingProposal, GuardianChange, GuardianKind, GuardianState, MemberReading, MembershipChange,
    MembershipKind, Observation, ObservationPruning, ObservationWindow, Office, OfficeChange,
    OfficeKind, OfficeState, PolityCharter, PruneRecommendation, PruneRoute, RetentionPolicy,
    Substrate,
};
use en_charter::{
    BoundaryAttestation, BoundaryDissent, BoundarySource, DissentKind, Exclusion, FoundingBasis,
    FoundingDissent, FoundingLegitimacy, GroundStanding, IndependenceKind,
    IndependentCorroboration, ReaderIndependence, ReaderIndependenceAttestation,
    SubstrateDisavowal, SubstrateGrounding, SubstrateReading,
};
use en_store::{ArchiveStore, FileStore, SaveOutcome};
use en_cohesion::{
    cohesion_report as cohesion_report_rs, CohesionInputs, CohesionReport, ErosionLevel, Thresholds,
};
use en_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, PubKey, Signed, Timestamp};
use en_substrate::{FoundingCandidate, ManualFounding, MemberCandidate, SubstrateProvider};
use en_uplift::FusionPredecessor;
use kc_core::CommunityIdentity;

fn en_err_to_py(e: KcError) -> PyErr {
    PyRuntimeError::new_err(format!("ethne: {e}"))
}

/// A dump that will not load is a problem with the *value* passed in — malformed
/// JSON, an unreadable version, a duplicated entry — so it surfaces as `ValueError`
/// rather than as the `RuntimeError` an internal archive fault would raise.
fn load_err_to_py(e: KcError) -> PyErr {
    PyValueError::new_err(format!("ethne: {e}"))
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

/// Render an affirmation kind for the trend read-model. (Nothing else renders one —
/// every other path only ever constructs them from a string.)
fn affirmation_kind_name(k: &AffirmationKind) -> String {
    match k {
        AffirmationKind::Induction => "Induction".into(),
        AffirmationKind::ReAffirmation => "ReAffirmation".into(),
    }
}

/// Render a membership kind, likewise for the churn series.
fn membership_kind_name(k: &MembershipKind) -> String {
    match k {
        MembershipKind::Admit => "Admit".into(),
        MembershipKind::DepartVoluntary => "DepartVoluntary".into(),
        MembershipKind::Expel => "Expel".into(),
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

/// Parse signed readings handed in as JSON — what `sign_reading` produced.
fn readings_from(readings: Vec<String>) -> PyResult<Vec<Signed<SubstrateReading>>> {
    readings
        .iter()
        .map(|r| serde_json::from_str(r).map_err(json_err_to_py))
        .collect()
}

/// Sign `candidate`'s reading of `polity_did` as `reader_did` — the host's act, not
/// the provider's (design **D22**).
///
/// A substrate provider holds no keys and gains none: signing belongs to whoever
/// runs it, exactly as D20 settled for every other authored act. The result is JSON,
/// to be passed to `propose_from_candidate(readings=[...])`.
///
/// This is what makes a boundary *attributable*. Before D22 a charter could say a
/// reading stood behind it without naming who read — an unsigned self-report, so
/// flipping one field promoted a bare assertion to a corroborated one. A reader's
/// key is recovered from its DID, so none of this needs a registry.
#[pyfunction]
#[pyo3(signature = (candidate, polity_did, reader_did, reader_priv, read_at=None, opaque_boundary=false))]
fn sign_reading(
    candidate: &PyFoundingCandidate,
    polity_did: &str,
    reader_did: &str,
    reader_priv: &[u8],
    read_at: Option<i64>,
    opaque_boundary: bool,
) -> PyResult<String> {
    let signers = Signers::only(reader_did, reader_priv)?;
    let mut reading = candidate.inner.reading(
        parse_did(polity_did)?,
        parse_did(reader_did)?,
        ts_or_now(read_at)?,
    );
    reading.opaque_boundary = opaque_boundary;
    let signed = Signed::build(reading, &signers.pairs()).map_err(en_err_to_py)?;
    serde_json::to_string(&signed).map_err(json_err_to_py)
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
    derivations=Vec::new(),
    exclusions=Vec::new(),
    proposed_at=None,
))]
#[allow(clippy::too_many_arguments)]
fn propose_charter(
    polity_did: &str,
    substrate_name: String,
    substrate_statement: String,
    member_dids: Vec<String>,
    substrate_anchor: Option<String>,
    commitments: Vec<(String, String)>,
    derivations: Vec<(String, Option<String>)>,
    exclusions: Vec<(String, String)>,
    proposed_at: Option<i64>,
) -> PyResult<String> {
    let anchor = substrate_anchor.as_deref().map(parse_did).transpose()?;
    let members = parse_dids(&member_dids)?;
    // The roll a convener names *is* the boundary they saw, so this path is
    // `ConvenerAsserted` by construction (D21) — an accurate claim, not a weaker one
    // smuggled in as a default. A convener who saw more than they are chartering
    // says so by passing `exclusions`.
    let attestation = if exclusions.is_empty() {
        BoundaryAttestation::convener_asserted(members.len())
    } else {
        BoundaryAttestation::convener_asserted_excluding(
            members.len(),
            exclusions
                .into_iter()
                .map(|(did, reason)| Ok(Exclusion { member: parse_did(&did)?, reason }))
                .collect::<PyResult<Vec<_>>>()?,
        )
    };
    let proposal = FoundingProposal {
        polity: parse_did(polity_did)?,
        substrate: Substrate {
            name: substrate_name,
            statement: substrate_statement,
            anchor,
        },
        founding_commitments: commitments_from(commitments),
        proposed_members: members,
        proposed_derivations: derivations_from(derivations)?,
        attestation,
        proposed_at: ts_or_now(proposed_at)?,
    };
    serde_json::to_string(&proposal).map_err(json_err_to_py)
}

/// Parse `(member_did, guardian_did | None)` pairs into founding derivations —
/// which members are derived (machine) parties and the human each answers to. A
/// `None` guardian is a machine chartered with nobody behind it.
fn derivations_from(pairs: Vec<(String, Option<String>)>) -> PyResult<Vec<Derivation>> {
    pairs
        .into_iter()
        .map(|(m, g)| {
            Ok(Derivation {
                member: parse_did(&m)?,
                guardian: g.as_deref().map(parse_did).transpose()?,
            })
        })
        .collect()
}

/// Generate an Ed25519 keypair, returning `(priv, pub, did)`.
#[pyfunction]
fn generate_ed25519_keypair(
    py: Python<'_>,
) -> PyResult<(Bound<'_, PyBytes>, Bound<'_, PyBytes>, String)> {
    let (sk, pk) = keys::generate(Alg::Ed25519).map_err(en_err_to_py)?;
    let did = DidKey::from_pubkey(&pk).map_err(en_err_to_py)?;
    Ok((
        PyBytes::new(py, &sk.bytes),
        PyBytes::new(py, &pk.bytes),
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
    /// Candidate members as
    /// `(did, reputation | None, guardian_did | None, attested_at_seconds | None,
    /// derived)`.
    ///
    /// `reputation` is `None` when the provider has no reading at all — *unrated*,
    /// which is not *rated low* (D18). The fourth element is when that guardian was
    /// last verified present; `None` is "not attended", and whether a stamp is still
    /// fresh is judged by `cohesion_report` against its cadence, not here. `derived`
    /// is whether the substrate reads the member as a machine party, independent of
    /// the guardian edge — `derived` with no guardian is the unguarded machine.
    #[getter]
    fn members(&self) -> Vec<(String, Option<f64>, Option<String>, Option<i64>, bool)> {
        self.inner
            .members
            .iter()
            .map(|m| {
                (
                    m.did.to_string(),
                    m.reputation,
                    m.guardian.as_ref().map(|g| g.to_string()),
                    m.attested_at.map(|t| t.as_second()),
                    m.derived,
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
            .and_then(|m| m.reputation))
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

/// Build a candidate directly from **observed** members — the shape any substrate
/// provider reduces to.
///
/// Observed, and so the boundary is `Substrate`-sourced (design **D21**): this is
/// the stand-in for a reading. A convener asserting a roll on their own word wants
/// `manual_founding`, which labels the boundary `ConvenerAsserted` and says so on
/// the charter forever.
#[pyfunction]
#[pyo3(signature = (members, substrate_name=None, substrate_statement=None, substrate_anchor=None))]
fn founding_candidate(
    members: Vec<(String, Option<f64>, Option<String>, Option<i64>, bool)>,
    substrate_name: Option<String>,
    substrate_statement: Option<String>,
    substrate_anchor: Option<String>,
) -> PyResult<PyFoundingCandidate> {
    let mut out = Vec::with_capacity(members.len());
    for (did, reputation, guardian, attested_at, derived) in members {
        out.push(MemberCandidate {
            did: parse_did(&did)?,
            reputation,
            derived,
            guardian: guardian.as_deref().map(parse_did).transpose()?,
            attested_at: attested_at.map(ts).transpose()?,
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
    readings=Vec::new(),
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
    readings: Vec<String>,
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
        readings_from(readings)?,
        ts_or_now(proposed_at)?,
    );
    serde_json::to_string(&proposal).map_err(json_err_to_py)
}

/// Draw a proposal that keeps exactly `keep`, for one stated `reason` — the
/// **deliberate** narrowing, as against the reputation floor
/// `propose_from_candidate` applies.
///
/// This exists so an honest small founding is a first-class act. A group of five
/// inside a thirty-node cluster may have every right to charter alone; if the only
/// ergonomic path were the floor, that founding would be dressed up as a floor it is
/// not. Here every member the candidate showed and the roll does not name is
/// recorded with the convener's own reason, so the boundary the charter claims to
/// ratify is the one it actually saw.
///
/// A DID in `keep` this candidate never showed is ignored — the attestation
/// describes *this* boundary, and cannot vouch for anyone outside it.
#[pyfunction]
#[pyo3(signature = (
    candidate,
    polity_did,
    keep,
    reason,
    substrate_name=None,
    substrate_statement=None,
    substrate_anchor=None,
    commitments=Vec::new(),
    readings=Vec::new(),
    proposed_at=None,
))]
#[allow(clippy::too_many_arguments)]
fn select_from_candidate(
    candidate: &PyFoundingCandidate,
    polity_did: &str,
    keep: Vec<String>,
    reason: &str,
    substrate_name: Option<String>,
    substrate_statement: Option<String>,
    substrate_anchor: Option<String>,
    commitments: Vec<(String, String)>,
    readings: Vec<String>,
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
    let keep: Vec<DidKey> = keep.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
    let proposal = candidate.inner.propose_selecting(
        parse_did(polity_did)?,
        substrate,
        commitments_from(commitments),
        &keep,
        reason,
        readings_from(readings)?,
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

    /// How many observed members the provider actually rated — the denominator of
    /// both reputation components (D18). `observed - rated_members` is the unrated
    /// count; zero means the reputation components said nothing rather than
    /// reporting health.
    #[getter]
    fn rated_members(&self) -> usize {
        self.inner.rated_members
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

    /// How many observed members carry a guardian edge at all — the denominator
    /// of `unattended`. A fall here is the guardian edge itself eroding.
    #[getter]
    fn guardian_members(&self) -> usize {
        self.inner.guardian_members
    }

    /// How many of those guardians are not verified present right now. Never
    /// escalates past `"Notice"` on its own: a snapshot cannot tell a night shift
    /// from an abandoned polity.
    #[getter]
    fn unattended(&self) -> usize {
        self.inner.unattended
    }

    /// How many chartered members the affirmation reading covered — the
    /// denominator of `stale_affirmations`, and 0 when none was supplied.
    #[getter]
    fn affirming_members(&self) -> usize {
        self.inner.affirming_members
    }

    /// How many of those have let their affirmation of the substrate lapse. Unlike
    /// absent guardians this CAN reach `"Concern"`: duration is already inside the
    /// measure, so a lapse is not ambiguous the way a quiet console is.
    #[getter]
    fn stale_affirmations(&self) -> usize {
        self.inner.stale_affirmations
    }

    /// Chartered guardian edges checkable here: designated members with a guardian
    /// who are still observed. 0 when no guardian reading was supplied.
    #[getter]
    fn chartered_guardian_members(&self) -> usize {
        self.inner.chartered_guardian_members
    }

    /// How many of those the provider no longer sees carrying a guardian at all —
    /// the charter says a human answers for this machine, and the substrate
    /// disagrees.
    #[getter]
    fn guardian_edges_lost(&self) -> usize {
        self.inner.guardian_edges_lost
    }

    /// Whether the latest affirmation gap exceeds the median of the prior ones.
    /// `None` with too short a series to compare. A *rolling* reading.
    #[getter]
    fn latest_gap_widening(&self) -> Option<bool> {
        self.inner.latest_gap_widening
    }

    /// Whether the latest affirmation gap exceeds the **first** one — the same
    /// question against a baseline the drift cannot move. Stays true after a
    /// settled slip, where the rolling reading above goes quiet.
    #[getter]
    fn gap_widening_since_founding(&self) -> Option<bool> {
        self.inner.gap_widening_since_founding
    }

    /// Whether the most recent induction drew fewer witnesses than the first.
    #[getter]
    fn witness_count_declined(&self) -> bool {
        self.inner.witness_count_declined
    }

    /// Roll changes per member over the churn window. `None` when unmeasured.
    #[getter]
    fn churn_rate(&self) -> Option<f64> {
        self.inner.churn_rate
    }

    /// Members unattended long enough, and watched closely enough, for the absence
    /// to be sustained rather than merely current. The one absence signal that
    /// escalates.
    #[getter]
    fn sustained_absences(&self) -> usize {
        self.inner.sustained_absences
    }

    /// Whether the polity's chartered substrate anchor has withdrawn (design
    /// **D23**). `False` both when no ground reading was supplied and when the ground
    /// still stands — safe only because a polity with no anchor cannot be disavowed in
    /// the first place.
    ///
    /// Not a ratio, and not comparable to the rest of this report: it is one signed
    /// act by the one key entitled to perform it, and it reaches `Concern` alone.
    #[getter]
    fn ground_disavowed(&self) -> bool {
        self.inner.ground_disavowed
    }

    /// `"None"`, `"Notice"`, or `"Concern"`.
    #[getter]
    fn erosion(&self) -> &'static str {
        erosion_name(self.inner.erosion)
    }
}

/// Grade the strain between a chartered roll and what the substrate currently
/// shows. Thresholds default to `en-cohesion`'s; override any of them by keyword.
/// Pass `affirmations` (an `AffirmationState`) to include transmission decay —
/// members who have stopped re-affirming the substrate; omitted, that component
/// reports nothing rather than false health. `as_of` (epoch seconds, default now)
/// is the moment both freshness measures are judged against. The monitor never
/// decides a split — a `Concern` is an early warning, and absent guardians never
/// raise one, though lapsed affirmations can.
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
    attendance_cadence_seconds=None,
    unattended_notice=None,
    unrated_notice=None,
    affirmation_cadence_seconds=None,
    affirmation_stale_notice=None,
    affirmation_stale_concern=None,
    guardian_lost_notice=None,
    guardian_lost_concern=None,
    churn_window_seconds=None,
    churn_notice=None,
    churn_concern=None,
    sustained_absence_seconds=None,
    sustained_absence_readings=None,
    guardians=None,
    affirmations=None,
    trends=None,
    window=None,
    ground=None,
    as_of=None,
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
    attendance_cadence_seconds: Option<i64>,
    unattended_notice: Option<f64>,
    unrated_notice: Option<f64>,
    affirmation_cadence_seconds: Option<i64>,
    affirmation_stale_notice: Option<f64>,
    affirmation_stale_concern: Option<f64>,
    guardian_lost_notice: Option<f64>,
    guardian_lost_concern: Option<f64>,
    churn_window_seconds: Option<i64>,
    churn_notice: Option<f64>,
    churn_concern: Option<f64>,
    sustained_absence_seconds: Option<i64>,
    sustained_absence_readings: Option<usize>,
    guardians: Option<&PyGuardianState>,
    affirmations: Option<&PyAffirmationState>,
    trends: Option<&PyDecayTrends>,
    window: Option<&PyObservationWindow>,
    ground: Option<&PyGroundStanding>,
    as_of: Option<i64>,
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
        attendance_cadence_seconds: attendance_cadence_seconds
            .unwrap_or(d.attendance_cadence_seconds),
        unattended_notice: unattended_notice.unwrap_or(d.unattended_notice),
        unrated_notice: unrated_notice.unwrap_or(d.unrated_notice),
        affirmation_cadence_seconds: affirmation_cadence_seconds
            .unwrap_or(d.affirmation_cadence_seconds),
        affirmation_stale_notice: affirmation_stale_notice
            .unwrap_or(d.affirmation_stale_notice),
        affirmation_stale_concern: affirmation_stale_concern
            .unwrap_or(d.affirmation_stale_concern),
        guardian_lost_notice: guardian_lost_notice.unwrap_or(d.guardian_lost_notice),
        guardian_lost_concern: guardian_lost_concern.unwrap_or(d.guardian_lost_concern),
        churn_window_seconds: churn_window_seconds.unwrap_or(d.churn_window_seconds),
        churn_notice: churn_notice.unwrap_or(d.churn_notice),
        churn_concern: churn_concern.unwrap_or(d.churn_concern),
        sustained_absence_seconds: sustained_absence_seconds
            .unwrap_or(d.sustained_absence_seconds),
        sustained_absence_readings: sustained_absence_readings
            .unwrap_or(d.sustained_absence_readings),
    };
    // No affirmation state supplied → an empty one, which the monitor reads as
    // "not evaluated" rather than "everyone is current".
    // An omitted read-model stays `None`, which the monitor reads as "not
    // evaluated" rather than "everyone is fine".
    let inputs = CohesionInputs {
        guardians: guardians.map(|g| &g.inner),
        affirmations: affirmations.map(|a| &a.inner),
        trends: trends.map(|t| &t.inner),
        window: window.map(|w| &w.inner),
        ground: ground.map(|g| &g.inner),
    };
    let inner = cohesion_report_rs(
        &parse_dids(&chartered_roll)?,
        &candidate.inner,
        opaque_boundary,
        &inputs,
        ts_or_now(as_of)?,
        &thresholds,
    );
    Ok(PyCohesionReport { inner })
}

// ---- Uplift: the interface to kith-covenant ---------------------------------

/// The bundle a fission emits, each artifact as JSON a KC-side consumer appends
/// verbatim: the successor identities, one successor link per successor
/// (index-aligned with them), and the predecessor's dissolution naming them all.
///
/// **N-ary since design D25**, so this is the exact mirror of `FusionOutcome`.
#[pyclass(name = "FissionOutcome", get_all)]
#[derive(Clone)]
struct PyFissionOutcome {
    successors: Vec<String>,
    links: Vec<String>,
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

    /// The whole log as a versioned JSON envelope — full inspection from Python
    /// without a wrapper class per record, and the format `load_json` reads.
    fn dump_json(&self) -> PyResult<String> {
        self.inner.dump_json_to_string().map_err(en_err_to_py)
    }

    /// Which generation this log is in — advanced by every compaction, never by an
    /// append. A cursor is only valid within one (D20).
    #[getter]
    fn generation(&self) -> u64 {
        self.inner.generation()
    }

    /// A cursor at the current end of the log: what to record once everything
    /// present has been persisted.
    fn cursor(&self) -> PyCursor {
        PyCursor { inner: self.inner.cursor() }
    }

    /// The entries after `cursor`, as a compact versioned envelope.
    ///
    /// The **incremental** dump, for a host persisting a log it keeps appending to:
    /// hold the `cursor()` you last wrote, pass it here, and append the result to
    /// your own storage. Saving then costs what is new rather than the whole log
    /// each time — which matters for the observation log, where a polity's monitors
    /// append steadily. A cursor already at the end yields an empty segment.
    ///
    /// **Raises if the cursor is from another generation, or past the end**, both of
    /// which mean the log was compacted underneath you: appending from there would
    /// keep the observations a pruning dropped while losing the pruning record
    /// itself. Rewrite in full instead — or let `FileStore` do it, which is what it
    /// is for.
    fn dump_json_since(&self, cursor: &PyCursor) -> PyResult<String> {
        self.inner.dump_json_since(cursor.inner).map_err(en_err_to_py)
    }

    /// Apply a segment from `dump_json_since`, returning how many entries were
    /// added. Strict about generation and position, so a segment that is not the
    /// next piece of *this* log raises rather than producing an archive that never
    /// existed. To combine two independent logs into one view, use `merge_json`.
    fn append_json(&mut self, s: &str) -> PyResult<usize> {
        self.inner.append_json(s).map_err(load_err_to_py)
    }

    /// Fold another log's entries into this one, ignoring generation and position —
    /// a combined *view* over independent logs, not persistence. Duplicate CIDs
    /// still raise.
    fn merge_json(&mut self, s: &str) -> PyResult<usize> {
        self.inner.merge_json(s).map_err(load_err_to_py)
    }

    /// Rebuild an archive from `dump_json` output, preserving append order. Reads
    /// the versioned envelope, and still reads the bare JSON array earlier builds
    /// emitted.
    #[classmethod]
    fn load_json(_cls: &Bound<'_, PyType>, s: &str) -> PyResult<Self> {
        Ok(Self { inner: Archive::load_json_from_str(s).map_err(load_err_to_py)? })
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
    ///
    /// Where the proposal carries derivations (D15), every **named guardian** must
    /// also co-sign, because answerability for a machine is accepted and never
    /// assigned: pass them as `guardian_dids` / `guardian_privs`.
    ///
    /// Where its substrate names an **anchor** (D23), that key co-signs too, for the
    /// same reason one level up: you may not conscript what you claim to stand under.
    /// Pass it as `anchor_did` / `anchor_priv`.
    #[pyo3(signature = (
        proposal_json, ratifier_dids, ratifier_privs,
        guardian_dids=Vec::new(), guardian_privs=Vec::new(),
        anchor_did=None, anchor_priv=None, founded_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn ratify_charter(
        &mut self,
        proposal_json: &str,
        ratifier_dids: Vec<String>,
        ratifier_privs: Vec<Vec<u8>>,
        guardian_dids: Vec<String>,
        guardian_privs: Vec<Vec<u8>>,
        anchor_did: Option<String>,
        anchor_priv: Option<Vec<u8>>,
        founded_at: Option<i64>,
    ) -> PyResult<String> {
        let proposal: FoundingProposal =
            serde_json::from_str(proposal_json).map_err(json_err_to_py)?;
        let mut signers = Signers::new(ratifier_dids, ratifier_privs)?;
        let ratifiers = parse_dids(&signers.dids)?;
        for (did, priv_) in guardian_dids.iter().zip(guardian_privs.iter()) {
            signers.consent(did, priv_)?;
        }
        if guardian_dids.len() != guardian_privs.len() {
            return Err(PyValueError::new_err(
                "guardian_dids and guardian_privs must be the same length",
            ));
        }
        match (&anchor_did, &anchor_priv) {
            (Some(did), Some(priv_)) => signers.consent(did, priv_)?,
            (None, None) => {}
            _ => {
                return Err(PyValueError::new_err(
                    "anchor_did and anchor_priv must be given together",
                ))
            }
        }
        let charter = proposal
            .ratify(ratifiers, ts_or_now(founded_at)?)
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

    // ---- Observation (what a monitor saw, so duration can be measured) -----

    /// Record what a monitor saw at one instant. `readings` are
    /// `(member_did, reputation, guardian_did | None, attested_at | None)` tuples —
    /// the observed reading per member, with the attendance stamp carried through
    /// unreduced.
    ///
    /// An observation asserts nothing about the polity: it is one party's
    /// perspectival evidence, signed by whoever looked, and the observer need not be
    /// a member. What it makes possible is *duration* — see `observation_window` and
    /// the sustained-absence measure, which is the one absence signal that escalates.
    #[pyo3(signature = (
        polity_did, observer_did, observer_priv, readings,
        opaque_boundary=false, observed_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn record_observation(
        &mut self,
        polity_did: &str,
        observer_did: &str,
        observer_priv: &[u8],
        readings: Vec<(String, f64, Option<String>, Option<i64>)>,
        opaque_boundary: bool,
        observed_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(observer_did, observer_priv)?;
        let readings = readings
            .into_iter()
            .map(|(member, reputation, guardian, attested_at)| {
                Ok(MemberReading {
                    member: parse_did(&member)?,
                    reputation,
                    guardian: guardian.as_deref().map(parse_did).transpose()?,
                    attested_at: attested_at.map(ts).transpose()?,
                })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let record = Observation {
            polity: parse_did(polity_did)?,
            observer: parse_did(observer_did)?,
            observed_at: ts_or_now(observed_at)?,
            readings,
            opaque_boundary,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::Observed(signed))
    }

    /// Drop the observations of `polity_did` recorded before `horizon`, leaving a
    /// signed record of the fact and a receipt for what went.
    ///
    /// Observations are the one entry class a polity may drop: they decide nothing,
    /// so removing one cannot rewrite what the polity resolved. Two routes, as with
    /// an envoy authorization: an effective `CharterKeeper` among the deciders
    /// suffices **beyond the settling horizon** (30 days — the custody that office
    /// has), and a majority of the roll may prune anything. Recent evidence is
    /// deliberately out of any single custodian's reach, because destroying it is
    /// the move a captured party makes.
    ///
    /// This authors the record; call `apply_prunings` to actually drop the entries.
    /// The receipt is computed here from what the log currently holds.
    #[pyo3(signature = (
        polity_did, horizon, decider_dids, decider_privs, rationale="", pruned_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn prune_observations(
        &mut self,
        polity_did: &str,
        horizon: i64,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: &str,
        pruned_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(decider_dids, decider_privs)?;
        let polity = parse_did(polity_did)?;
        let horizon = ts(horizon)?;
        let dropped = prunable_observations(&self.inner, &polity, horizon);
        let digest = pruning_digest(&dropped).map_err(en_err_to_py)?;
        let record = ObservationPruning {
            polity,
            horizon,
            dropped: dropped.len(),
            digest,
            rationale: rationale.to_string(),
            deciders: parse_dids(&signers.dids)?,
            pruned_at: ts_or_now(pruned_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::ObservationsPruned(signed))
    }

    /// Apply `polity_did`'s ratified prunings, returning a **new** archive without
    /// the observations they drop. Nothing else is touched: every governing record
    /// is copied through, so the polity's decided state is unchanged.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn apply_prunings(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<Self> {
        let inner = apply_prunings(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?)
            .map_err(en_err_to_py)?;
        Ok(Self { inner })
    }

    /// What pruning this observation log would take right now, or `None` when there
    /// is nothing worth doing.
    ///
    /// Advisory: it signs nothing and appends nothing. Feed the `horizon` it returns
    /// to `prune_observations` — the polity still ratifies its own housekeeping. What
    /// it saves the host is the part a host cannot be expected to reconstruct: **it
    /// can only ever propose something `check_authority` accepts**, and it will not
    /// clip the history a live measure reads back over.
    ///
    /// Every argument defaults to `DEFAULT_RETENTION`, i.e. to the Rust core's own
    /// policy. `retain_seconds` is how much history to keep (30 days by default — the
    /// smallest value that both preserves every observation-based measure and keeps
    /// the single-`CharterKeeper` route open); `min_dropped` is how many observations
    /// must have aged out before a pruning is worth its own permanent record; and the
    /// two safety floors the horizon may never cross, whatever retention asks for —
    /// `live_measure_seconds` for the *span* a live measure reads back over, and
    /// `min_witness_readings` for the *witness count* it also needs. Both are
    /// required: a sustained absence is graded on two conditions, and a duration only
    /// protects one of them.
    #[pyo3(signature = (
        polity_did, retain_seconds=None, min_dropped=None, live_measure_seconds=None,
        min_witness_readings=None, as_of=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn prune_recommendation(
        &self,
        polity_did: &str,
        retain_seconds: Option<i64>,
        min_dropped: Option<usize>,
        live_measure_seconds: Option<i64>,
        min_witness_readings: Option<usize>,
        as_of: Option<i64>,
    ) -> PyResult<Option<PyPruneRecommendation>> {
        let d = RetentionPolicy::default();
        let policy = RetentionPolicy {
            retain_seconds: retain_seconds.unwrap_or(d.retain_seconds),
            min_dropped: min_dropped.unwrap_or(d.min_dropped),
            live_measure_seconds: live_measure_seconds.unwrap_or(d.live_measure_seconds),
            min_witness_readings: min_witness_readings.unwrap_or(d.min_witness_readings),
        };
        Ok(prune_recommendation_rs(
            &self.inner,
            &parse_did(polity_did)?,
            &policy,
            ts_or_now(as_of)?,
        )
        .map(|inner| PyPruneRecommendation { inner }))
    }

    // ---- The guardian edge (which members are machines, and whose) ---------

    /// Designate a member a **derived** (machine) party, optionally naming the human
    /// it answers to. A majority of the live roll decides, and the named guardian
    /// co-signs: answerability is accepted, never assigned. Passing no guardian
    /// charters a machine with nobody behind it — an honest state, and the one the
    /// machine-majority office rule refuses to give an office to.
    #[pyo3(signature = (
        polity_did, member_did, decider_dids, decider_privs,
        guardian_did=None, guardian_priv=None, rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn designate_derived(
        &mut self,
        polity_did: &str,
        member_did: &str,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        guardian_did: Option<&str>,
        guardian_priv: Option<&[u8]>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(decider_dids, decider_privs)?;
        let deciders = parse_dids(&signers.dids)?;
        let guardian = match (guardian_did, guardian_priv) {
            (Some(did), Some(key)) => {
                signers.consent(did, key)?;
                Some(parse_did(did)?)
            }
            (None, None) => None,
            _ => {
                return Err(PyValueError::new_err(
                    "a named guardian must supply both guardian_did and guardian_priv (consent)",
                ))
            }
        };
        let record = GuardianChange {
            polity: parse_did(polity_did)?,
            kind: GuardianKind::Designate,
            member: parse_did(member_did)?,
            guardian,
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::GuardianChanged(signed))
    }

    /// Point a designated member at a (new) guardian — its operator changed, or a
    /// machine chartered with nobody behind it acquires one. A majority plus the
    /// **incoming** guardian's consent; the outgoing one is released, never held.
    #[pyo3(signature = (
        polity_did, member_did, guardian_did, guardian_priv, decider_dids, decider_privs,
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn rotate_guardian(
        &mut self,
        polity_did: &str,
        member_did: &str,
        guardian_did: &str,
        guardian_priv: &[u8],
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(decider_dids, decider_privs)?;
        let deciders = parse_dids(&signers.dids)?;
        signers.consent(guardian_did, guardian_priv)?;
        let record = GuardianChange {
            polity: parse_did(polity_did)?,
            kind: GuardianKind::Rotate,
            member: parse_did(member_did)?,
            guardian: Some(parse_did(guardian_did)?),
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::GuardianChanged(signed))
    }

    /// Record that a member's guardian edge is gone: the machine now answers to
    /// nobody. Two routes — pass the departing guardian's own key to have them step
    /// away, or a majority of the roll to record that the human is simply no longer
    /// there. The designation itself survives; a machine does not become human by
    /// losing its human.
    #[pyo3(signature = (
        polity_did, member_did, guardian_did=None, guardian_priv=None,
        decider_dids=Vec::new(), decider_privs=Vec::new(),
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn release_guardian(
        &mut self,
        polity_did: &str,
        member_did: &str,
        guardian_did: Option<&str>,
        guardian_priv: Option<&[u8]>,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let (signers, deciders) = match (guardian_did, guardian_priv) {
            (Some(did), Some(key)) => (Signers::only(did, key)?, vec![]),
            (None, None) => {
                let s = Signers::new(decider_dids, decider_privs)?;
                let d = parse_dids(&s.dids)?;
                (s, d)
            }
            _ => {
                return Err(PyValueError::new_err(
                    "the departing guardian must supply both guardian_did and guardian_priv",
                ))
            }
        };
        let record = GuardianChange {
            polity: parse_did(polity_did)?,
            kind: GuardianKind::Release,
            member: parse_did(member_did)?,
            guardian: None,
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::GuardianChanged(signed))
    }

    // ---- Envoy standing (who may speak for the polity) --------------------

    /// Authorize `envoy_did` to speak for the polity beyond it. Two routes, and
    /// the authority pass accepts either: a single `EnvoyAuthorizer` (the office's
    /// delegated power), or a majority of the roll. The envoy always co-signs —
    /// speaking for the polity is a duty, never conscripted. `expires_at` bounds
    /// the standing, and travels with the KC-side delegation.
    ///
    /// The envoy's public key is derived from their DID, so the record's key and
    /// DID can never disagree through this path.
    #[pyo3(signature = (
        polity_did, envoy_did, envoy_priv, authorizer_dids, authorizer_privs,
        rationale=String::new(), expires_at=None, decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn authorize_envoy(
        &mut self,
        polity_did: &str,
        envoy_did: &str,
        envoy_priv: &[u8],
        authorizer_dids: Vec<String>,
        authorizer_privs: Vec<Vec<u8>>,
        rationale: String,
        expires_at: Option<i64>,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let mut signers = Signers::new(authorizer_dids, authorizer_privs)?;
        let authorizers = parse_dids(&signers.dids)?;
        signers.consent(envoy_did, envoy_priv)?;
        let envoy = parse_did(envoy_did)?;
        let envoy_pubkey = envoy.to_pubkey().map_err(en_err_to_py)?;
        let record = EnvoyAuthorization {
            polity: parse_did(polity_did)?,
            kind: EnvoyKind::Authorize,
            envoy,
            envoy_pubkey,
            rationale,
            authorizers,
            expires_at: expires_at.map(ts).transpose()?,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::EnvoyAuthorized(signed))
    }

    /// The envoy stands down: self-signed, no authorizers.
    #[pyo3(signature = (polity_did, envoy_did, envoy_priv, rationale=String::new(), decided_at=None))]
    fn resign_envoy(
        &mut self,
        polity_did: &str,
        envoy_did: &str,
        envoy_priv: &[u8],
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(envoy_did, envoy_priv)?;
        let envoy = parse_did(envoy_did)?;
        let envoy_pubkey = envoy.to_pubkey().map_err(en_err_to_py)?;
        let record = EnvoyAuthorization {
            polity: parse_did(polity_did)?,
            kind: EnvoyKind::Resign,
            envoy,
            envoy_pubkey,
            rationale,
            authorizers: vec![],
            expires_at: None,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::EnvoyAuthorized(signed))
    }

    /// The polity withdraws the standing — the same two routes as authorizing it,
    /// with the envoy excluded from the authorizers.
    #[pyo3(signature = (
        polity_did, envoy_did, authorizer_dids, authorizer_privs,
        rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn revoke_envoy(
        &mut self,
        polity_did: &str,
        envoy_did: &str,
        authorizer_dids: Vec<String>,
        authorizer_privs: Vec<Vec<u8>>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::new(authorizer_dids, authorizer_privs)?;
        let envoy = parse_did(envoy_did)?;
        let envoy_pubkey = envoy.to_pubkey().map_err(en_err_to_py)?;
        let record = EnvoyAuthorization {
            polity: parse_did(polity_did)?,
            kind: EnvoyKind::Revoke,
            envoy,
            envoy_pubkey,
            rationale,
            authorizers: parse_dids(&signers.dids)?,
            expires_at: None,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::EnvoyAuthorized(signed))
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

    /// What this polity's founding rests on (**D21**, resolving **Q1**), or `None`
    /// if it was never chartered.
    ///
    /// Reads the genesis charter and nothing else — no provider, no query time,
    /// because none of what it reports can change after founding. The charter's core
    /// has no amendment path, so **anyone holding the log can re-derive this
    /// forever**, which is the point of recording the boundary rather than merely
    /// checking it once.
    ///
    /// `consent_share` is the number Q1 asks for: ratifiers as a share of everyone
    /// the boundary showed, rather than of the roll the founders chose themselves.
    fn founding_legitimacy(&self, polity_did: &str) -> PyResult<Option<PyFoundingLegitimacy>> {
        Ok(founding_legitimacy_rs(&self.inner, &parse_did(polity_did)?)
            .map(|inner| PyFoundingLegitimacy { inner }))
    }

    /// What the excluded have said back about `polity_did`'s founding, as of
    /// `as_of` (design **D22**).
    ///
    /// Kept **separate from `founding_legitimacy` on purpose**: that read-model's
    /// value is that it derives from the charter alone and gives the same answer
    /// forever, while dissent lives in the archive and accrues over time. Folding
    /// them would destroy the first property and improve neither.
    ///
    /// Reports dissent from a **dissolved** polity too — dissolution is the one
    /// thing that does not close this record, since otherwise founders silence the
    /// parties they excluded simply by winding the polity up.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn founding_dissent(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyFoundingDissent> {
        let d = founding_dissent_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyFoundingDissent { inner: d })
    }

    /// File `dissenter_did`'s dissent from `polity_did`'s founding boundary.
    ///
    /// `kind` is `"Unnamed"` (the accounting never mentioned me — the claim that
    /// contradicts an understated `candidate_size`, which nothing else in the tier
    /// can reach) or `"ReasonRejected"` (I am named as excluded and reject the
    /// reason given).
    ///
    /// Perspectival evidence, exactly as an observation is: signed by the dissenter
    /// alone, no deciders, no majority, and the dissenter need not be a member — one
    /// required to sit inside the roll could be silenced by never being admitted. A
    /// dissent **asserts; it does not adjudicate**, and nothing here decides whether
    /// it is true.
    #[pyo3(signature = (polity_did, dissenter_did, dissenter_priv, kind, rationale="", dissented_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn dissent_boundary(
        &mut self,
        polity_did: &str,
        dissenter_did: &str,
        dissenter_priv: &[u8],
        kind: &str,
        rationale: &str,
        dissented_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(dissenter_did, dissenter_priv)?;
        let kind = match kind {
            "Unnamed" => DissentKind::Unnamed,
            "ReasonRejected" => DissentKind::ReasonRejected,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown dissent kind {other:?} (expected Unnamed or ReasonRejected)"
                )))
            }
        };
        let record = BoundaryDissent {
            polity: parse_did(polity_did)?,
            dissenter: parse_did(dissenter_did)?,
            kind,
            rationale: rationale.to_string(),
            dissented_at: ts_or_now(dissented_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::BoundaryDissented(signed))
    }

    /// How `polity_did`'s chartered ground stands as of `as_of`: whether the substrate
    /// anchor that consented at founding has since withdrawn (design **D23**).
    ///
    /// Kept **separate from `founding_legitimacy`** on the same test D22 applied to
    /// dissent. `grounding` — what the charter committed to — lives there, because a
    /// genesis-immutable substrate cannot change and the answer is the same forever.
    /// Whether the ground still stands there lives here, because that can change.
    ///
    /// Reports withdrawals from a **dissolved** polity too: the second entry class a
    /// dissolution does not close, since otherwise founders silence their ground by
    /// winding the polity up.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn substrate_disavowal(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyGroundStanding> {
        let g = substrate_disavowal_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyGroundStanding { inner: g })
    }

    /// File the chartered substrate anchor's withdrawal from `polity_did` (design
    /// **D23**).
    ///
    /// Only the anchor the charter names may do this, and it must sign alone — unlike
    /// a dissent, which has to admit any filer because an omitted party is by
    /// definition absent from the record. The ground is *on* the record.
    ///
    /// **This asserts; it does not un-charter.** The substrate is genesis-immutable,
    /// so nothing here voids the founding, dissolves the polity, or stops it
    /// governing itself — giving a withdrawal that power would hand any anchor a
    /// unilateral kill switch. What it produces is a legible fact, which
    /// `cohesion_report` reads as a Concern.
    #[pyo3(signature = (polity_did, anchor_did, anchor_priv, rationale="", disavowed_at=None))]
    fn disavow_substrate(
        &mut self,
        polity_did: &str,
        anchor_did: &str,
        anchor_priv: &[u8],
        rationale: &str,
        disavowed_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(anchor_did, anchor_priv)?;
        let record = SubstrateDisavowal {
            polity: parse_did(polity_did)?,
            disavower: parse_did(anchor_did)?,
            rationale: rationale.to_string(),
            disavowed_at: ts_or_now(disavowed_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::SubstrateDisavowed(signed))
    }

    /// Who has vouched for (or challenged) `polity_did`'s founding readers, as of
    /// `as_of` (design **D24**).
    ///
    /// D22 made a boundary's reading *attributable* but could not tell one reader from
    /// five faces of the same actor: measured, one party minted five `did:key`s, signed
    /// five identical readings, and the charter graded `Corroborated` — 32 µs a voice.
    /// So the count of distinct signing readers cannot mean what it appears to mean,
    /// and this is where the vouching for it lives.
    ///
    /// Attesters are sorted by **stake**: `attesters` counts, while the polity itself,
    /// the reader itself, and anyone on the founding roll land in
    /// `interested_attesters` — recorded, because somebody signed their name to it, and
    /// not credited, because a founder vouching for the founding's own reader is the
    /// sockpuppet move one step out.
    ///
    /// A **lens, never a gate**: a young honest founding has no attestations, and
    /// nothing here refuses one for it.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn reader_independence(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyReaderIndependence> {
        let r = reader_independence_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyReaderIndependence { inner: r })
    }

    /// Which of `polity_did`'s corroborating readers are independently vouched for, as
    /// of `as_of` — **the upgrade of D22's corroboration count** (design **D24**).
    ///
    /// The conjunction of two different questions: a reader who agrees with the ratified
    /// boundary but has no outside voucher might be five faces of the convener, and a
    /// vouched-for reader who read a *different* boundary corroborates nothing. Only
    /// both together mean what "corroborated" appears to mean, so a chorus of five
    /// hundred minted readers scores `0` here.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn independent_corroboration(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyIndependentCorroboration> {
        let c =
            independent_corroboration_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyIndependentCorroboration { inner: c })
    }

    /// File a third party's account of whether `reader_did` — a reader of
    /// `polity_did`'s founding — is an independent principal (design **D24**).
    ///
    /// `kind` is `"Independent"` (vouching) or `"Controlled"` (the counter-claim, so the
    /// lens can be contradicted by somebody who thinks it is being fooled).
    ///
    /// The attester signs alone, and `reader_did` must actually appear among the
    /// founding's readings — coherence with the record spoken about, never its truth.
    /// Whether the attester is disinterested is the *lens'* concern, not this call's: an
    /// interested attestation is accepted and reported separately rather than refused.
    ///
    /// **What this buys, stated plainly:** it does not prove independence and nothing
    /// adjudicates the claim. It moves the cost of faking corroboration from one key to
    /// a second party willing to be *named* for it, permanently.
    #[pyo3(signature = (polity_did, reader_did, attester_did, attester_priv, kind="Independent", basis="", attested_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn attest_reader_independence(
        &mut self,
        polity_did: &str,
        reader_did: &str,
        attester_did: &str,
        attester_priv: &[u8],
        kind: &str,
        basis: &str,
        attested_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(attester_did, attester_priv)?;
        let kind = match kind {
            "Independent" => IndependenceKind::Independent,
            "Controlled" => IndependenceKind::Controlled,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown independence kind {other:?} (expected Independent or Controlled)"
                )))
            }
        };
        let record = ReaderIndependenceAttestation {
            polity: parse_did(polity_did)?,
            reader: parse_did(reader_did)?,
            attesting_party: parse_did(attester_did)?,
            kind,
            basis: basis.to_string(),
            attested_at: ts_or_now(attested_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::ReaderIndependenceAttested(signed))
    }

    /// The polity's decay trends as of `as_of` — the affirmation series and its
    /// gaps, induction witness counts, and roll changes. Read from the polity's own
    /// signed records, never from a provider.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn decay_trends(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyDecayTrends> {
        let t = decay_trends(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyDecayTrends { inner: t })
    }

    /// What this polity's monitors recorded seeing in `[since, as_of]`. The window is
    /// bounded at both ends on purpose: "has anyone been present today" must not be
    /// answered out of last year's evidence.
    #[pyo3(signature = (polity_did, since, as_of=None))]
    fn observation_window(
        &self,
        polity_did: &str,
        since: i64,
        as_of: Option<i64>,
    ) -> PyResult<PyObservationWindow> {
        let w = observation_window(
            &self.inner,
            &parse_did(polity_did)?,
            ts(since)?,
            ts_or_now(as_of)?,
        );
        Ok(PyObservationWindow { inner: w })
    }

    /// Which members are derived (machine) parties and the human each answers to as
    /// of `as_of` — founding derivations plus every change, intersected with the
    /// live roll, so a designation lapses while its member is off the roll.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn guardian_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyGuardianState> {
        let st = guardian_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyGuardianState { inner: st })
    }

    /// Who may speak for the polity as of `as_of` — standings intersected with the
    /// live roll and with their own expiry, so one lapses while its envoy is off
    /// the roll and ends when it expires.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn envoy_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyEnvoyState> {
        let st = envoy_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyEnvoyState { inner: st })
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

    /// Emit KC's envoy `SpeakerDelegation` (JSON) for `envoy_did` — the border
    /// artifact an *office* produces. The standing is read from this log as of
    /// `as_of`, so an envoy who has resigned, been revoked, left the roll, or
    /// whose authorization has expired yields an error rather than a delegation.
    #[pyo3(signature = (polity_did, envoy_did, root_priv, root_pub, issued_at=None, as_of=None))]
    #[allow(clippy::too_many_arguments)]
    fn envoy_delegation(
        &self,
        polity_did: &str,
        envoy_did: &str,
        root_priv: &[u8],
        root_pub: &[u8],
        issued_at: Option<i64>,
        as_of: Option<i64>,
    ) -> PyResult<String> {
        let polity = parse_did(polity_did)?;
        let charter = self.charter_of(&polity)?;
        let envoys = envoy_state(&self.inner, &polity, ts_or_now(as_of)?);
        let signed = en_uplift::envoy_delegation(
            &charter,
            &envoys,
            &parse_did(envoy_did)?,
            &priv_from_bytes(root_priv),
            pub_from_bytes(root_pub),
            ts_or_now(issued_at)?,
        )
        .map_err(en_err_to_py)?;
        serde_json::to_string(&signed).map_err(json_err_to_py)
    }

    /// Emit a **ratified** split into N (≥ 2) successors: the community has already
    /// chartered them (pass their identities as a list of JSON strings); this signs,
    /// under the predecessor's root, a successor link to each plus the predecessor's
    /// dissolution naming them all. Nothing is appended here — to put the dissolution
    /// in this log and have it pass the authority rules, the deciders must co-sign it
    /// through `dissolve_charter`.
    ///
    /// **N-ary since design D25**, mirroring `fusion`. The guards are fusion's in the
    /// other direction: at least two successors, no successor named twice, no successor
    /// that is the predecessor itself, and every successor's own self-signature
    /// verified — without which the predecessor's root key would sign a link to
    /// whatever it was handed.
    #[pyo3(signature = (
        polity_did, successor_jsons, root_priv,
        decider_dids=Vec::new(), rationale=String::new(), at=None,
    ))]
    fn fission(
        &self,
        polity_did: &str,
        successor_jsons: Vec<String>,
        root_priv: &[u8],
        decider_dids: Vec<String>,
        rationale: String,
        at: Option<i64>,
    ) -> PyResult<PyFissionOutcome> {
        let charter = self.charter_of(&parse_did(polity_did)?)?;
        let successors = successor_jsons
            .iter()
            .map(|j| parse_identity(j))
            .collect::<PyResult<Vec<_>>>()?;
        let outcome = en_uplift::fission(
            &charter,
            successors,
            parse_dids(&decider_dids)?,
            &priv_from_bytes(root_priv),
            rationale,
            ts_or_now(at)?,
        )
        .map_err(en_err_to_py)?;
        Ok(PyFissionOutcome {
            successors: outcome
                .successors
                .iter()
                .map(to_json)
                .collect::<PyResult<Vec<_>>>()?,
            links: outcome.links.iter().map(to_json).collect::<PyResult<Vec<_>>>()?,
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

/// The polity's decay trends — shapes over time, read from its own records.
#[pyclass(name = "DecayTrends")]
struct PyDecayTrends {
    inner: DecayTrends,
}

#[pymethods]
impl PyDecayTrends {
    /// Every affirmation as `(at_secs, witness_count, kind)`, oldest first.
    #[getter]
    fn affirmations(&self) -> Vec<(i64, usize, String)> {
        self.inner
            .affirmations
            .iter()
            .map(|p| {
                (
                    p.at.as_second(),
                    p.witness_count,
                    affirmation_kind_name(&p.kind),
                )
            })
            .collect()
    }

    /// Successive affirmation intervals in seconds.
    #[getter]
    fn gaps_seconds(&self) -> Vec<i64> {
        self.inner.gaps_seconds.clone()
    }

    /// Seconds from the last affirmation to the query time.
    #[getter]
    fn seconds_since_last(&self) -> Option<i64> {
        self.inner.seconds_since_last
    }

    /// Roll changes as `(at_secs, kind)`, oldest first.
    #[getter]
    fn membership_changes(&self) -> Vec<(i64, String)> {
        self.inner
            .membership_changes
            .iter()
            .map(|c| (c.at.as_second(), membership_kind_name(&c.kind)))
            .collect()
    }

    /// Whether the latest affirmation gap exceeds the **median** of the prior gaps.
    /// `None` with fewer than two gaps — not enough cadence to compare, which is not
    /// the same as "no".
    fn latest_gap_widening(&self) -> Option<bool> {
        self.inner.latest_gap_widening()
    }

    /// Whether the latest affirmation gap exceeds the **first** one. Stays true
    /// after a settled slip, where the median reading above goes quiet.
    fn gap_widening_since_founding(&self) -> Option<bool> {
        self.inner.gap_widening_since_founding()
    }

    /// Whether the most recent induction drew fewer witnesses than the first.
    /// `False` with fewer than two inductions: no trend from one point.
    fn witness_count_declined(&self) -> bool {
        self.inner.witness_count_declined()
    }

    /// Roll changes per member over `window_seconds` ending at `as_of`.
    #[pyo3(signature = (window_seconds, roll_len, as_of=None))]
    fn churn_rate(
        &self,
        window_seconds: i64,
        roll_len: usize,
        as_of: Option<i64>,
    ) -> PyResult<Option<f64>> {
        Ok(self.inner.churn_rate(window_seconds, roll_len, ts_or_now(as_of)?))
    }
}

/// What the polity's monitors recorded seeing, over an interval.
#[pyclass(name = "ObservationWindow")]
struct PyObservationWindow {
    inner: ObservationWindow,
}

#[pymethods]
impl PyObservationWindow {
    /// Every observation as `(at_secs, observer_did, member_count, opaque_boundary)`.
    #[getter]
    fn observations(&self) -> Vec<(i64, String, usize, bool)> {
        self.inner
            .observations
            .iter()
            .map(|o| {
                (
                    o.at.as_second(),
                    o.observer.to_string(),
                    o.readings.len(),
                    o.opaque_boundary,
                )
            })
            .collect()
    }

    /// How many observations the window holds.
    fn reading_count(&self) -> usize {
        self.inner.reading_count()
    }

    /// How many of them actually saw `did` — the per-member evidence of watching.
    fn readings_for(&self, did: &str) -> PyResult<usize> {
        Ok(self.inner.readings_for(&parse_did(did)?))
    }

    /// Seconds from the first observation to the last; `None` with fewer than two.
    fn span_seconds(&self) -> Option<i64> {
        self.inner.span_seconds()
    }

    /// The latest attendance stamp recorded anywhere in the window for `did`.
    fn last_attested(&self, did: &str) -> PyResult<Option<i64>> {
        Ok(self.inner.last_attested(&parse_did(did)?).map(|t| t.as_second()))
    }

    /// How long `did` has gone unattended as of `as_of`. `None` for a member the
    /// window never saw — not evaluated, since nobody watching is not evidence.
    ///
    /// Clamped at `pruned_before`: no absence is claimed across an interval whose
    /// evidence was pruned away, so pruning can only ever shorten this.
    #[pyo3(signature = (did, as_of=None))]
    fn unattended_span(&self, did: &str, as_of: Option<i64>) -> PyResult<Option<i64>> {
        Ok(self
            .inner
            .unattended_span(&parse_did(did)?, ts_or_now(as_of)?))
    }

    /// The pruning horizon biting inside this window, if one does: observations
    /// before it were dropped, so the window holds no evidence about that interval.
    /// `None` when nothing was pruned, or when the pruning is older than the
    /// window's own start and so took nothing that was asked for.
    #[getter]
    fn pruned_before(&self) -> Option<i64> {
        self.inner.pruned_before.map(|t| t.as_second())
    }
}

/// The chartered node→guardian picture at a point in time: which members are
/// derived (machine) parties and whom each answers to.
#[pyclass(name = "GuardianState")]
struct PyGuardianState {
    inner: GuardianState,
}

#[pymethods]
impl PyGuardianState {
    /// Every designation in force as `(member_did, guardian_did | None)`.
    #[getter]
    fn derivations(&self) -> Vec<(String, Option<String>)> {
        self.inner
            .derivations
            .iter()
            .map(|d| (d.member.to_string(), d.guardian.as_ref().map(|g| g.to_string())))
            .collect()
    }

    /// Whether `did` is a designated derived (machine) party.
    fn is_derived(&self, did: &str) -> PyResult<bool> {
        Ok(self.inner.is_derived(&parse_did(did)?))
    }

    /// The human `did` answers to. Raises nothing for an undesignated member: it
    /// returns `None`, as does a designated machine with no guardian — use
    /// `is_derived` to tell the two apart.
    fn guardian_of(&self, did: &str) -> PyResult<Option<String>> {
        Ok(self
            .inner
            .guardian_of(&parse_did(did)?)
            .and_then(|g| g.as_ref().map(|g| g.to_string())))
    }

    /// Derived members that do have a guardian — the edges the substrate is
    /// expected to keep showing.
    fn guarded_members(&self) -> Vec<String> {
        self.inner.guarded_members().into_iter().map(|d| d.to_string()).collect()
    }

    /// Derived members answering to nobody.
    fn unguarded_members(&self) -> Vec<String> {
        self.inner.unguarded_members().into_iter().map(|d| d.to_string()).collect()
    }

    /// Whether derived members are at least a majority of a roll of `roll_len` —
    /// the condition under which every office must trace to a human.
    fn is_machine_majority(&self, roll_len: usize) -> bool {
        self.inner.is_machine_majority(roll_len)
    }
}

/// Effective envoy standings at a point in time — the read-model `en-uplift`
/// turns into KC's envoy delegation.
#[pyclass(name = "EnvoyState")]
struct PyEnvoyState {
    inner: EnvoyState,
}

#[pymethods]
impl PyEnvoyState {
    /// Every standing in force as
    /// `(envoy_did, expires_at_secs | None, authorized_at_secs)`.
    #[getter]
    fn standings(&self) -> Vec<(String, Option<i64>, i64)> {
        self.inner
            .standings
            .iter()
            .map(|h| {
                (
                    h.envoy.to_string(),
                    h.expires_at.map(|t| t.as_second()),
                    h.authorized_at.as_second(),
                )
            })
            .collect()
    }

    /// Whether `did` may currently speak for the polity.
    fn is_active(&self, did: &str) -> PyResult<bool> {
        Ok(self.inner.is_active(&parse_did(did)?))
    }

    /// Everyone who may currently speak for the polity.
    fn envoys(&self) -> Vec<String> {
        self.inner.envoys().into_iter().map(|d| d.to_string()).collect()
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

/// A pruning the polity could run now: what to drop, and who may authorize it.
///
/// Read-only, and advisory — `Archive.prune_recommendation` derives it, nothing
/// signs it. The routes are reported cheapest first; both appear when both are open,
/// because the officer route is an *alternative* to the roll and never a gate on it.
#[pyclass(name = "PruneRecommendation")]
struct PyPruneRecommendation {
    inner: PruneRecommendation,
}

#[pymethods]
impl PyPruneRecommendation {
    /// Observations strictly before this epoch second are dropped. Guaranteed to
    /// advance past any horizon already in force, and never to reach within
    /// `live_measure_seconds` of the query time.
    #[getter]
    fn horizon(&self) -> i64 {
        self.inner.horizon.as_second()
    }

    /// How many observations it would drop.
    #[getter]
    fn dropped(&self) -> usize {
        self.inner.dropped
    }

    /// Seconds of history the polity keeps if it acts on this.
    #[getter]
    fn seconds_retained(&self) -> i64 {
        self.inner.seconds_retained
    }

    /// The open authorization routes by name, cheapest first: `"CharterKeeper"`
    /// and/or `"RollMajority"`.
    #[getter]
    fn routes(&self) -> Vec<String> {
        self.inner
            .routes
            .iter()
            .map(|r| match r {
                PruneRoute::CharterKeeper { .. } => "CharterKeeper".to_string(),
                PruneRoute::RollMajority { .. } => "RollMajority".to_string(),
            })
            .collect()
    }

    /// The effective `CharterKeeper` holders, any **one** of whom may authorize this
    /// alone. Empty when that route is closed — either the horizon is too recent for
    /// the office's custody, or nobody on the roll holds it.
    #[getter]
    fn keeper_holders(&self) -> Vec<String> {
        self.inner
            .routes
            .iter()
            .find_map(|r| match r {
                PruneRoute::CharterKeeper { holders } => {
                    Some(holders.iter().map(|d| d.to_string()).collect())
                }
                _ => None,
            })
            .unwrap_or_default()
    }

    /// How many of the current roll must co-sign, on the route that is always open.
    #[getter]
    fn roll_majority_need(&self) -> usize {
        self.inner
            .routes
            .iter()
            .find_map(|r| match r {
                PruneRoute::RollMajority { need } => Some(*need),
                _ => None,
            })
            .unwrap_or(0)
    }

    /// Whether a single `CharterKeeper` can authorize this without the roll.
    #[getter]
    fn keeper_may_authorize(&self) -> bool {
        self.inner.keeper_may_authorize()
    }

    fn __repr__(&self) -> String {
        format!(
            "PruneRecommendation(horizon={}, dropped={}, seconds_retained={}, routes={:?})",
            self.inner.horizon.as_second(),
            self.inner.dropped,
            self.inner.seconds_retained,
            self.routes(),
        )
    }
}

/// Where a host's storage has caught up to: an index **and the generation it was
/// taken in** (D20).
///
/// The generation is what makes a cursor safe. An index alone is meaningless across
/// a compaction — entries go from the middle, so the same index is a different
/// entry, and an index past the new end used to read as "already current". Both are
/// refusals now.
#[pyclass(name = "Cursor")]
#[derive(Clone)]
struct PyCursor {
    inner: Cursor,
}

#[pymethods]
impl PyCursor {
    #[new]
    fn new(generation: u64, index: usize) -> Self {
        PyCursor { inner: Cursor { generation, index } }
    }

    #[getter]
    fn generation(&self) -> u64 {
        self.inner.generation
    }

    #[getter]
    fn index(&self) -> usize {
        self.inner.index
    }

    fn __eq__(&self, other: &PyCursor) -> bool {
        self.inner == other.inner
    }

    fn __repr__(&self) -> String {
        format!("Cursor(generation={}, index={})", self.inner.generation, self.inner.index)
    }
}

/// What a `FileStore.save` actually did.
#[pyclass(name = "SaveOutcome")]
struct PySaveOutcome {
    inner: SaveOutcome,
}

#[pymethods]
impl PySaveOutcome {
    /// Entries newly written — the whole log, when `rewrote`.
    #[getter]
    fn appended(&self) -> usize {
        self.inner.appended
    }

    /// Whether this was a full rewrite rather than an append. True exactly when the
    /// log had been compacted since the last save.
    #[getter]
    fn rewrote(&self) -> bool {
        self.inner.rewrote
    }

    /// Bytes written.
    #[getter]
    fn bytes(&self) -> usize {
        self.inner.bytes
    }

    /// Whether anything reached storage at all.
    #[getter]
    fn wrote_anything(&self) -> bool {
        self.inner.wrote_anything()
    }

    fn __repr__(&self) -> String {
        format!(
            "SaveOutcome(appended={}, rewrote={}, bytes={})",
            self.inner.appended, self.inner.rewrote, self.inner.bytes
        )
    }
}

/// A charter log persisted to a single file, one segment per line.
///
/// Use this rather than hand-rolling persistence, because the one branch that
/// matters is easy to get wrong and silent when you do: **same generation ⇒ append a
/// segment; generation changed ⇒ rewrite in full.** A host that appends past a
/// compaction keeps the observations the pruning dropped and loses the pruning
/// record, and every verification pass still says the reloaded log is fine.
///
/// The store holds the cursor, so you cannot hold a stale one.
///
///     store = FileStore("polity.log")
///     archive = store.load()          # absent file ⇒ empty log, not an error
///     ...
///     store.save(archive)             # append, or rewrite after a pruning
#[pyclass(name = "FileStore")]
struct PyFileStore {
    inner: FileStore,
}

#[pymethods]
impl PyFileStore {
    #[new]
    fn new(path: &str) -> Self {
        PyFileStore { inner: FileStore::new(path) }
    }

    /// Where this store writes.
    #[getter]
    fn path(&self) -> String {
        self.inner.path().display().to_string()
    }

    /// What the store believes it has persisted, if it has looked.
    fn cursor(&self) -> Option<PyCursor> {
        self.inner.cursor().map(|inner| PyCursor { inner })
    }

    /// Read the whole log back, at the generation it was stored in. An absent file
    /// is an empty log, so a first run needs no special case.
    fn load(&mut self) -> PyResult<PyArchive> {
        let inner: Archive<CharterEntry> =
            ArchiveStore::<CharterEntry>::load(&mut self.inner).map_err(load_err_to_py)?;
        Ok(PyArchive { inner })
    }

    /// Persist whatever is not yet stored, appending or rewriting as the log's
    /// generation requires.
    fn save(&mut self, archive: &PyArchive) -> PyResult<PySaveOutcome> {
        let inner = ArchiveStore::<CharterEntry>::save(&mut self.inner, &archive.inner)
            .map_err(en_err_to_py)?;
        Ok(PySaveOutcome { inner })
    }

    /// Drop an unfinished final segment left by an interrupted write, returning the
    /// bytes discarded.
    ///
    /// Loading refuses to do this quietly — silently dropping an unparseable last
    /// line would discard a signed entry without saying so. The log may forget, but
    /// only by saying so.
    fn recover(&mut self) -> PyResult<usize> {
        self.inner.recover().map_err(en_err_to_py)
    }
}

/// What a polity's founding rests on, derived from its charter alone (D21).
/// What the excluded said back about a founding — one party's assertion each, and
/// never a verdict (design **D22**).
#[pyclass(name = "FoundingDissent")]
#[derive(Clone)]
struct PyFoundingDissent {
    inner: FoundingDissent,
}

#[pymethods]
impl PyFoundingDissent {
    /// Every dissent filed, in append order, as
    /// `(dissenter, kind, rationale, dissented_at)`.
    #[getter]
    fn entries(&self) -> Vec<(String, &'static str, String, i64)> {
        self.inner
            .entries
            .iter()
            .map(|d| {
                (
                    d.dissenter.as_str().to_string(),
                    dissent_kind_name(d.kind),
                    d.rationale.clone(),
                    d.dissented_at.as_second(),
                )
            })
            .collect()
    }

    /// Dissenters claiming the founding accounting never mentioned them.
    ///
    /// **The answer to the limit D21 recorded as beyond its reach**: the archive
    /// holds no provider, so nothing can check `candidate_size` against what the
    /// substrate showed — but the people it omitted can say so, and this is where
    /// that shows up.
    #[getter]
    fn unnamed(&self) -> Vec<String> {
        self.inner.unnamed().iter().map(|d| d.dissenter.as_str().to_string()).collect()
    }

    /// Dissenters named as excluded who reject the reason given.
    #[getter]
    fn reasons_rejected(&self) -> Vec<String> {
        self.inner.reasons_rejected().iter().map(|d| d.dissenter.as_str().to_string()).collect()
    }

    /// Distinct dissenters, however many records each filed.
    #[getter]
    fn dissenters(&self) -> Vec<String> {
        self.inner.dissenters().iter().map(|d| (*d).to_string()).collect()
    }

    fn __len__(&self) -> usize {
        self.inner.entries.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "FoundingDissent(entries={}, unnamed={}, reasons_rejected={})",
            self.inner.entries.len(),
            self.inner.unnamed().len(),
            self.inner.reasons_rejected().len()
        )
    }
}

#[pyclass(name = "GroundStanding")]
#[derive(Clone)]
struct PyGroundStanding {
    inner: GroundStanding,
}

#[pymethods]
impl PyGroundStanding {
    /// Every withdrawal filed, in append order, as
    /// `(disavower, rationale, disavowed_at)`.
    #[getter]
    fn entries(&self) -> Vec<(String, String, i64)> {
        self.inner
            .entries
            .iter()
            .map(|d| {
                (
                    d.disavower.as_str().to_string(),
                    d.rationale.clone(),
                    d.disavowed_at.as_second(),
                )
            })
            .collect()
    }

    /// Whether the chartered ground has withdrawn at all.
    ///
    /// **The whole reading.** There is no "how much" and deliberately no recovery
    /// path: a ground that withdrew cannot un-say it, because the substrate has no
    /// amendment record and this tier does not invent one. Such a polity has the
    /// option every polity has — to re-charter, deliberately, as the different polity
    /// it now is.
    #[getter]
    fn disavowed(&self) -> bool {
        self.inner.disavowed()
    }

    /// When the ground first withdrew, if it has. The FIRST, not the latest: a ground
    /// that repeats itself has not withdrawn more recently, it has withdrawn for
    /// longer.
    #[getter]
    fn first_disavowed_at(&self) -> Option<i64> {
        self.inner.first_disavowed_at().map(|t| t.as_second())
    }

    fn __len__(&self) -> usize {
        self.inner.entries.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "GroundStanding(disavowed={}, entries={})",
            self.inner.disavowed(),
            self.inner.entries.len()
        )
    }
}

#[pyclass(name = "ReaderIndependence")]
#[derive(Clone)]
struct PyReaderIndependence {
    inner: ReaderIndependence,
}

fn map_out(m: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>)
    -> Vec<(String, Vec<String>)> {
    m.iter().map(|(k, v)| (k.clone(), v.iter().cloned().collect())).collect()
}

#[pymethods]
impl PyReaderIndependence {
    /// reader → the distinct **disinterested** parties who vouched for it. The ones
    /// that count.
    #[getter]
    fn attesters(&self) -> Vec<(String, Vec<String>)> {
        map_out(&self.inner.attesters)
    }

    /// reader → parties who vouched but have a stake: the polity, the reader itself, or
    /// a founding member. Recorded rather than discarded, and reported rather than
    /// quietly folded in.
    #[getter]
    fn interested_attesters(&self) -> Vec<(String, Vec<String>)> {
        map_out(&self.inner.interested_attesters)
    }

    /// reader → parties asserting it is **not** independent. Counted from anyone,
    /// deliberately: interest disqualifies praise, not warning, and a founder breaking
    /// ranks is the most informative possible source for the claim.
    #[getter]
    fn contested(&self) -> Vec<(String, Vec<String>)> {
        map_out(&self.inner.contested)
    }

    /// Readers with at least one disinterested voucher.
    #[getter]
    fn independent_readers(&self) -> Vec<String> {
        self.inner.independent_readers().iter().map(|r| (*r).to_string()).collect()
    }

    fn is_attested(&self, reader_did: &str) -> bool {
        self.inner.is_attested(reader_did)
    }

    fn is_contested(&self, reader_did: &str) -> bool {
        self.inner.is_contested(reader_did)
    }

    fn __repr__(&self) -> String {
        format!(
            "ReaderIndependence(independent={}, interested={}, contested={})",
            self.inner.independent_readers().len(),
            self.inner.interested_attesters.len(),
            self.inner.contested.len()
        )
    }
}

#[pyclass(name = "IndependentCorroboration")]
#[derive(Clone)]
struct PyIndependentCorroboration {
    inner: IndependentCorroboration,
}

#[pymethods]
impl PyIndependentCorroboration {
    /// Readers who **both** agree with the ratified boundary and carry a disinterested
    /// voucher — the number D22 could not produce.
    #[getter]
    fn readers(&self) -> Vec<String> {
        self.inner.readers.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Of those, the ones somebody has also claimed are controlled — so "independently
    /// corroborated" cannot quietly mean "and nobody objected".
    #[getter]
    fn contested(&self) -> Vec<String> {
        self.inner.contested.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Those with a voucher and no counter-claim.
    #[getter]
    fn uncontested(&self) -> Vec<String> {
        self.inner.uncontested().iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Whether any independently-vouched reader agrees with what was ratified.
    #[getter]
    fn any(&self) -> bool {
        self.inner.any()
    }

    fn __len__(&self) -> usize {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!(
            "IndependentCorroboration(readers={}, contested={})",
            self.inner.count(),
            self.inner.contested.len()
        )
    }
}

fn grounding_name(g: SubstrateGrounding) -> &'static str {
    match g {
        SubstrateGrounding::Anchored => "Anchored",
        SubstrateGrounding::Guarded => "Guarded",
        SubstrateGrounding::Held => "Held",
        SubstrateGrounding::Bare => "Bare",
    }
}

fn dissent_kind_name(k: DissentKind) -> &'static str {
    match k {
        DissentKind::Unnamed => "Unnamed",
        DissentKind::ReasonRejected => "ReasonRejected",
    }
}

#[pyclass(name = "FoundingLegitimacy")]
struct PyFoundingLegitimacy {
    inner: FoundingLegitimacy,
}

#[pymethods]
impl PyFoundingLegitimacy {
    /// `"Substrate"` (an independent reading) or `"ConvenerAsserted"` (the
    /// convener's own word, with nobody else attesting the boundary).
    #[getter]
    fn source(&self) -> &'static str {
        match self.inner.source {
            BoundarySource::Substrate { .. } => "Substrate",
            BoundarySource::ConvenerAsserted => "ConvenerAsserted",
        }
    }

    /// How many members the boundary showed.
    #[getter]
    fn candidate_size(&self) -> usize {
        self.inner.candidate_size
    }

    /// How many the charter names.
    #[getter]
    fn roll_size(&self) -> usize {
        self.inner.roll_size
    }

    /// How many it showed that the charter leaves out.
    #[getter]
    fn excluded(&self) -> usize {
        self.inner.excluded
    }

    /// Exclusions carrying no reason — named, but unjustified.
    /// Every reader who signed a reading behind this boundary (design **D22**).
    /// Empty for a convener's assertion.
    #[getter]
    fn readers(&self) -> Vec<String> {
        self.inner.readers.iter().map(|d| d.as_str().to_string()).collect()
    }

    /// The readers whose signed reading actually **agrees** with the boundary the
    /// charter ratified.
    ///
    /// Not `len(readers)`: a reader who signed a different boundary corroborates
    /// nothing, and the gap between the two is a reader contradicting the founding
    /// on the record. **The limit is stated, not engineered around** — `did:key`
    /// costs nothing, so a captured reader can mint its own corroborators and this
    /// cannot tell them apart. A lens, never a gate.
    #[getter]
    fn corroborating_readers(&self) -> Vec<String> {
        self.inner.corroborating_readers.iter().map(|d| d.as_str().to_string()).collect()
    }

    /// Readers who signed a reading that does **not** match what was ratified.
    #[getter]
    fn dissenting_readers(&self) -> Vec<String> {
        self.inner.dissenting_readers().iter().map(|d| d.as_str().to_string()).collect()
    }

    /// Who the boundary showed that the roll does not name, each with the reason
    /// given, as `(did, reason)`. The names, not just the count — a report that said
    /// only "29 excluded" would reinstate the silence D21 removes.
    #[getter]
    fn exclusions(&self) -> Vec<(String, String)> {
        self.inner
            .exclusions
            .iter()
            .map(|x| (x.member.as_str().to_string(), x.reason.clone()))
            .collect()
    }

    #[getter]
    fn unreasoned_exclusions(&self) -> usize {
        self.inner.unreasoned_exclusions
    }

    /// What stands above the polity (design **D23**): `"Anchored"` (a keyed party
    /// outside the roll consented — the only grounding somebody signed), `"Guarded"`
    /// (no anchor, but human guardians stand above the machines), `"Held"` (no derived
    /// party at all: a human community holding an unkeyed tradition), or `"Bare"`
    /// (articulated and nothing else — unguarded machines on a mixed roll).
    ///
    /// Descriptive, not ordered. `"Bare"` is permitted, and reported so that
    /// "permitted" is not mistaken for "grounded".
    #[getter]
    fn grounding(&self) -> &'static str {
        grounding_name(self.inner.grounding)
    }

    /// The anchor that co-signed the founding, if any — the key that consented to
    /// standing above this polity, and the only key a later withdrawal may be.
    #[getter]
    fn anchor(&self) -> Option<String> {
        self.inner.anchor.as_ref().map(|d| d.as_str().to_string())
    }

    #[getter]
    fn ratifiers(&self) -> usize {
        self.inner.ratifiers
    }

    /// **The number Q1 asks for**: ratifiers as a share of everyone the boundary
    /// showed. One node chartering a thirty-node cluster scores `0.033`.
    #[getter]
    fn consent_share(&self) -> f64 {
        self.inner.consent_share
    }

    /// `"Corroborated"` / `"Narrowed"` / `"SelfAttested"`. **Descriptive, and
    /// deliberately not ordered** — a founding is not eroding, and these are not
    /// comparable harms. A narrow founding may be entirely legitimate; a
    /// self-attested one may be the only kind available.
    #[getter]
    fn basis(&self) -> &'static str {
        match self.inner.basis {
            FoundingBasis::Corroborated => "Corroborated",
            FoundingBasis::Narrowed => "Narrowed",
            FoundingBasis::SelfAttested => "SelfAttested",
        }
    }

    /// Whether an independent reading stands behind the boundary.
    #[getter]
    fn corroborated(&self) -> bool {
        self.inner.corroborated()
    }

    /// Whether the charter names fewer than the boundary showed.
    #[getter]
    fn narrowed(&self) -> bool {
        self.inner.narrowed()
    }

    /// Whether every exclusion carries a reason.
    #[getter]
    fn fully_reasoned(&self) -> bool {
        self.inner.fully_reasoned()
    }

    fn __repr__(&self) -> String {
        format!(
            "FoundingLegitimacy(basis={}, grounding={}, candidate_size={}, roll_size={}, \
             excluded={}, ratifiers={}, consent_share={:.3})",
            self.basis(),
            self.grounding(),
            self.inner.candidate_size,
            self.inner.roll_size,
            self.inner.excluded,
            self.inner.ratifiers,
            self.inner.consent_share,
        )
    }
}

/// `en-charter`'s own retention defaults, so the Python side never carries a second
/// copy of them.
fn default_retention(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let d = RetentionPolicy::default();
    let out = PyDict::new(py);
    out.set_item("retain_seconds", d.retain_seconds)?;
    out.set_item("min_dropped", d.min_dropped)?;
    out.set_item("live_measure_seconds", d.live_measure_seconds)?;
    out.set_item("min_witness_readings", d.min_witness_readings)?;
    Ok(out)
}

/// `en-cohesion`'s own threshold defaults, so the Python side never carries a
/// second copy of them.
fn default_thresholds(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let d = Thresholds::default();
    let out = PyDict::new(py);
    out.set_item("reputation_floor", d.reputation_floor)?;
    out.set_item("attrition_notice", d.attrition_notice)?;
    out.set_item("attrition_concern", d.attrition_concern)?;
    out.set_item("mean_reputation_notice", d.mean_reputation_notice)?;
    out.set_item("mean_reputation_concern", d.mean_reputation_concern)?;
    out.set_item("breach_notice", d.breach_notice)?;
    out.set_item("breach_concern", d.breach_concern)?;
    out.set_item("attendance_cadence_seconds", d.attendance_cadence_seconds)?;
    out.set_item("unattended_notice", d.unattended_notice)?;
    out.set_item("unrated_notice", d.unrated_notice)?;
    out.set_item("affirmation_cadence_seconds", d.affirmation_cadence_seconds)?;
    out.set_item("affirmation_stale_notice", d.affirmation_stale_notice)?;
    out.set_item("affirmation_stale_concern", d.affirmation_stale_concern)?;
    out.set_item("guardian_lost_notice", d.guardian_lost_notice)?;
    out.set_item("guardian_lost_concern", d.guardian_lost_concern)?;
    out.set_item("churn_window_seconds", d.churn_window_seconds)?;
    out.set_item("churn_notice", d.churn_notice)?;
    out.set_item("churn_concern", d.churn_concern)?;
    out.set_item("sustained_absence_seconds", d.sustained_absence_seconds)?;
    out.set_item("sustained_absence_readings", d.sustained_absence_readings)?;
    Ok(out)
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("ALG_ED25519", "Ed25519")?;
    m.add("DEFAULT_THRESHOLDS", default_thresholds(m.py())?)?;
    m.add("DEFAULT_RETENTION", default_retention(m.py())?)?;
    m.add_class::<PyArchive>()?;
    m.add_class::<PyCharterState>()?;
    m.add_class::<PyOfficeState>()?;
    m.add_class::<PyDecayTrends>()?;
    m.add_class::<PyObservationWindow>()?;
    m.add_class::<PyPruneRecommendation>()?;
    m.add_class::<PyFoundingLegitimacy>()?;
    m.add_class::<PyGroundStanding>()?;
    m.add_class::<PyReaderIndependence>()?;
    m.add_class::<PyIndependentCorroboration>()?;
    m.add_class::<PyCursor>()?;
    m.add_class::<PySaveOutcome>()?;
    m.add_class::<PyFileStore>()?;
    m.add_class::<PyGuardianState>()?;
    m.add_class::<PyEnvoyState>()?;
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
    m.add_function(wrap_pyfunction!(select_from_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(sign_reading, m)?)?;
    m.add_function(wrap_pyfunction!(cohesion_report, m)?)?;
    m.add_function(wrap_pyfunction!(coherence_concern, m)?)?;
    Ok(())
}
