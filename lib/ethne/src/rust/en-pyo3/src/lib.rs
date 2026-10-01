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

use std::collections::BTreeMap;

use en_archive::{Archive, Cursor};
use en_charter::{
    affirmation_state, apply_prunings, authority_violations, charter_state, check_authority,
    decay_trends, envoy_state, guardian_state, observation_window, office_state,
    founding_dissent as founding_dissent_rs, founding_legitimacy as founding_legitimacy_rs, prunable_observations,
    independent_corroboration as independent_corroboration_rs,
    independent_grounding as independent_grounding_rs,
    independent_observation as independent_observation_rs,
    observer_independence as observer_independence_rs,
    reader_independence as reader_independence_rs, substrate_disavowal as substrate_disavowal_rs,
    prune_recommendation as prune_recommendation_rs, pruning_digest,
    verify_all, Affirmation,
    AffirmationKind, AffirmationState, AmendmentKind, CharterDissolution, CharterEntry, Commitment,
    CommitmentAmendment, DecayTrends, Derivation, EnvoyAuthorization, EnvoyKind, EnvoyState,
    FoundingProposal, GuardianChange, GuardianEvidence, GuardianKind, GuardianState,
    MemberReading, MembershipChange,
    MembershipKind, Observation, ObservationPruning, ObservationWindow, Office, OfficeChange,
    OfficeKind, OfficeState, PolityCharter, PruneRecommendation, PruneRoute, RetentionPolicy,
    Substrate, Movement, RiteMovement, rite_state, Rite,
    rendezvous_state, RelayEndpoint,
    content_policy_state, ContentPolicy, ContentPolicyAmendment,
    compact_state, CompactAct, CompactActKind,
};
use en_charter::{
    BoundaryAttestation, BoundaryDissent, BoundarySource, DissentKind, Exclusion, FoundingBasis,
    FoundingDissent, FoundingLegitimacy, GroundStanding, IndependenceKind,
    AttestedRole, IndependenceAttestation, IndependentCorroboration, IndependentGrounding,
    IndependentObservation, ReaderIndependence,
    SubstrateDisavowal, SubstrateGrounding, SubstrateReading,
};
use en_store::{ArchiveStore, FileStore, Reconciled, SaveOutcome};
use en_cohesion::{
    cohesion_report as cohesion_report_rs, CohesionInputs, CohesionReport, ErosionLevel, Thresholds,
};
use en_host::{DueAction, HostConfig, HostCycle};
use en_core::{keys, time, Alg, Cid, DidKey, KcError, PrivKey, PubKey, Signed, Timestamp};
use en_at::{signing_key_did, AtEvent, AtPeer, AtSubstrateProvider, MockEventSource};
#[cfg(feature = "at-ffi")]
use en_at::{AtLogLevel, AttachedAtDaemon, EmbeddedAtNode};
use en_substrate::{
    ChangeStream, FoundingCandidate, FoundingTerms, ManualFounding, MemberCandidate,
    SubstrateChange, SubstrateProvider,
};
use en_uplift::FusionPredecessor;
use kc_core::CommunityIdentity;

/// One member as the Python seam carries it:
/// `(did, reputation, guardian_did, attested_at, derived)`.
///
/// Named because it crosses the seam in **both** directions — it is the `members`
/// getter's row and `founding_candidate`'s parameter row, and the two have to agree
/// element for element. A bare tuple repeated in two signatures is a contract nothing
/// states; this is that contract.
///
/// `reputation` is `None` for an unrated member (D18: the provider's epistemic state,
/// not a low score), and `derived` with no `guardian_did` is the unguarded machine.
type MemberRow = (String, Option<f64>, Option<String>, Option<i64>, bool);

/// One induction rite as the Python seam carries it: `(member, kind, movements,
/// began_at, completed_at, attendance, expected_movements)`.
type RiteRow = (
    String,
    String,
    usize,
    Option<i64>,
    Option<i64>,
    Vec<String>,
    usize,
);

/// Resolve a caller's evidence arguments into the record's carried evidence (D15).
///
/// The Python surface takes a **citation** — the CID of an observation already in the
/// charter — and this copies it into the record, so the caller gets the ergonomics of
/// a reference while the record gets the durability of a copy. That matters because
/// D17 lets a polity prune observations: a record holding a CID would be
/// un-authorized by legitimate housekeeping, while a record holding the copy is not.
///
/// `unbacked_reason` is the declared override, for bootstrap and offline authoring.
/// Exactly one of the two may be given.
fn resolve_evidence(
    archive: &Archive<CharterEntry>,
    evidence_cid: Option<&str>,
    unbacked_reason: Option<String>,
) -> PyResult<Option<GuardianEvidence>> {
    match (evidence_cid, unbacked_reason) {
        (Some(_), Some(_)) => Err(PyValueError::new_err(
            "pass either evidence_cid or unbacked_reason, not both",
        )),
        (None, None) => Ok(None),
        (None, Some(reason)) => {
            if reason.trim().is_empty() {
                return Err(PyValueError::new_err(
                    "unbacked_reason may not be empty: an unexplained override is what \
                     the rule exists to prevent",
                ));
            }
            Ok(Some(GuardianEvidence::Unbacked { reason }))
        }
        (Some(cid_str), None) => {
            let cid = parse_cid(cid_str)?;
            match archive.get(&cid) {
                Some(CharterEntry::Observed(signed)) => {
                    Ok(Some(GuardianEvidence::Observed(Box::new(signed.clone()))))
                }
                Some(other) => Err(PyValueError::new_err(format!(
                    "evidence_cid {cid_str} is a {}, not an observation",
                    en_archive::Entry::kind(other)
                ))),
                None => Err(PyValueError::new_err(format!(
                    "evidence_cid {cid_str} is not in this charter"
                ))),
            }
        }
    }
}


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

fn parse_cids(cids: &[String]) -> PyResult<Vec<Cid>> {
    cids.iter().map(|c| parse_cid(c)).collect()
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

/// The named offices match by name; anything else is a polity-specific
/// `Office::Other`.
fn parse_office(s: &str) -> Office {
    match s {
        "Convener" => Office::Convener,
        "Inductor" => Office::Inductor,
        "EnvoyAuthorizer" => Office::EnvoyAuthorizer,
        "CharterKeeper" => Office::CharterKeeper,
        "RendezvousOperator" => Office::RendezvousOperator,
        "Moderator" => Office::Moderator,
        other => Office::Other(other.to_string()),
    }
}

/// Parse an affirmation kind from its variant name — which ceremony a rite movement
/// belongs to (**D34**).
fn affirmation_kind(name: &str) -> PyResult<AffirmationKind> {
    match name {
        "Induction" => Ok(AffirmationKind::Induction),
        "ReAffirmation" => Ok(AffirmationKind::ReAffirmation),
        other => Err(PyValueError::new_err(format!(
            "unknown affirmation kind '{other}': expected Induction or ReAffirmation"
        ))),
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
        Office::RendezvousOperator => "RendezvousOperator".into(),
        Office::Moderator => "Moderator".into(),
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

/// Sign arbitrary exported payload bytes with an ed25519 key (D15's detached path).
///
/// The counterpart to `pending_guardian_change`: whoever holds the key signs the bytes
/// it exported and hands back the signature. Deliberately a free function rather than
/// an `Archive` method — the signer is not the archive's host, and in the arrangement
/// this exists for it is a different process on a different machine.
#[pyfunction]
fn sign_detached(signer_priv: &[u8], payload_bytes: &[u8]) -> PyResult<Vec<u8>> {
    let sk = priv_from_bytes(signer_priv);
    keys::sign(&sk, payload_bytes)
        .map(|s| s.bytes)
        .map_err(en_err_to_py)
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
    derivations: Vec<PyDerivation>,
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

/// One founding derivation as a caller writes it: either the bare pair, or the
/// pair plus the evidence for its edge (**D27**).
///
/// Untagged, tried in order, so `(member, None)` keeps working for the case that
/// needs no evidence — a machine chartered with nobody behind it, which claims
/// nothing. A caller naming a guardian passes the 4-tuple.
#[derive(FromPyObject)]
enum PyDerivation {
    /// `(member_did, guardian_did | None)`
    Bare(String, Option<String>),
    /// `(member_did, guardian_did | None, observation_json | None, unbacked_reason | None)`
    Evidenced(String, Option<String>, Option<String>, Option<String>),
}

/// Parse caller-written derivations into founding records — which members are
/// derived (machine) parties, the human each answers to, and what showed it
/// (**D27**).
///
/// Evidence is passed here rather than resolved by CID as the post-founding path
/// does, because at founding there is no archive yet to hold the observation: sign
/// it with `sign_observation` and hand in the JSON, exactly as D22's boundary
/// readings work. Pass `unbacked_reason` instead to declare that nothing backed the
/// edge — honest, permitted, and (unlike post-founding) not enough to count as
/// guarding, so a wholly machine roll must still name an anchor.
///
/// A guardian named with neither argument builds fine and is refused by
/// `check_authority`, deliberately: the rule lives in one place, and a Python
/// negative control has to be able to reach it.
fn derivations_from(pairs: Vec<PyDerivation>) -> PyResult<Vec<Derivation>> {
    pairs
        .into_iter()
        .map(|d| {
            let (m, g, ob_json, reason) = match d {
                PyDerivation::Bare(m, g) => (m, g, None, None),
                PyDerivation::Evidenced(m, g, ob, r) => (m, g, ob, r),
            };
            let guardian = g.as_deref().map(parse_did).transpose()?;
            let evidence = match (ob_json.as_deref(), reason) {
                (Some(_), Some(_)) => {
                    return Err(PyValueError::new_err(
                        "pass either an observation or an unbacked_reason for a founding \
                         derivation, not both",
                    ))
                }
                (Some(json), None) => {
                    let ob: Signed<Observation> =
                        serde_json::from_str(json).map_err(json_err_to_py)?;
                    Some(GuardianEvidence::Observed(Box::new(ob)))
                }
                (None, Some(reason)) => Some(GuardianEvidence::Unbacked { reason }),
                (None, None) => None,
            };
            Ok(Derivation { member: parse_did(&m)?, guardian, evidence })
        })
        .collect()
}

/// Sign an observation of `polity_did` as `observer_did`, returning JSON — the
/// pre-archive counterpart of `Archive.record_observation` (**D27**).
///
/// A founding derivation's evidence has to be signed before the charter that will
/// carry it exists, so it cannot be resolved out of the archive by CID the way a
/// later `guardian_change` resolves its own. `readings` are
/// `(member_did, reputation, guardian_did | None, attested_at | None)` tuples, as
/// there. Pass the result to `propose_charter(derivations=[...])`.
#[pyfunction]
#[pyo3(signature = (
    polity_did, observer_did, observer_priv, readings,
    opaque_boundary=false, observed_at=None,
))]
fn sign_observation(
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
    serde_json::to_string(&signed).map_err(json_err_to_py)
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
#[pyclass(skip_from_py_object, name = "FoundingCandidate")]
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
    fn members(&self) -> Vec<MemberRow> {
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
    members: Vec<MemberRow>,
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

/// Parse an observation handed in as JSON — what `sign_observation` produced.
///
/// Backs the founding guardian edges it actually shows (**D27**); every other
/// claimed edge is recorded as unbacked, saying so.
fn observation_from(json: Option<String>) -> PyResult<Option<Signed<Observation>>> {
    json.as_deref()
        .map(|j| serde_json::from_str(j).map_err(json_err_to_py))
        .transpose()
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
    observation=None,
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
    observation: Option<String>,
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
        FoundingTerms {
            polity: parse_did(polity_did)?,
            substrate,
            founding_commitments: commitments_from(commitments),
            readings: readings_from(readings)?,
            observation: observation_from(observation)?,
            proposed_at: ts_or_now(proposed_at)?,
        },
        min_reputation,
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
    observation=None,
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
    observation: Option<String>,
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
        FoundingTerms {
            polity: parse_did(polity_did)?,
            substrate,
            founding_commitments: commitments_from(commitments),
            readings: readings_from(readings)?,
            observation: observation_from(observation)?,
            proposed_at: ts_or_now(proposed_at)?,
        },
        &keep,
        reason,
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
#[pyclass(skip_from_py_object, name = "CohesionReport")]
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

    /// **Who was watching, and who will say so** (design **D28**) —
    /// `(distinct, vouched, contested)`, or `None` when no window was supplied.
    ///
    /// A **lens, never a gate**: the erosion level does not move on these numbers.
    /// Gating an escalation on the monitor being vouched for would let a captured
    /// polity vouch for nobody and become unwatchable. What it buys is that a reader
    /// can tell a `Concern` raised by three independent monitors from one raised by
    /// three keys — or, before D28, by one key filing three times.
    #[getter]
    fn watchers(&self) -> Option<(usize, usize, usize)> {
        self.inner.watchers.map(|w| (w.distinct, w.vouched, w.contested))
    }

    /// How many rites this reading covers (**D29**); `0` when none was supplied.
    #[getter]
    fn rites(&self) -> usize {
        self.inner.rites
    }

    /// Whether the polity's most recent completed rite ran shorter than its first —
    /// the ceremony costing less than it used to. `None` with fewer than two completed
    /// rites, which is not `False`. Capped at `Notice` like the other trends.
    #[getter]
    fn rite_span_shortening(&self) -> Option<bool> {
        self.inner.rite_span_shortening
    }

    /// Rites begun and never completed. Reported, not graded: nothing says how long a
    /// catechumenate may take.
    #[getter]
    fn rites_abandoned(&self) -> usize {
        self.inner.rites_abandoned
    }

    /// Attendees lost between a rite's first movement and its last, over completed
    /// rites — the polity that turned out for the naming of the substrate and had
    /// drifted away by the submission of the polity itself.
    #[getter]
    fn rite_attrition(&self) -> usize {
        self.inner.rite_attrition
    }

    /// Whether nobody on the roll holds the substrate. Reaches `Concern` alone: every
    /// majority is of the members who hold it, so the polity can take no act of its own
    /// while this is true.
    #[getter]
    fn nobody_holds_the_substrate(&self) -> bool {
        self.inner.nobody_holds_the_substrate
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

    /// **Which optional readings this grade was not given** (design **D31**), by the
    /// keyword names `cohesion_report` takes.
    ///
    /// Every optional input reports *nothing* rather than health when absent, which is
    /// useless to a reader who cannot tell silence from health. Empty here means the
    /// grade saw everything, which is what `HostCycle.grade` guarantees.
    #[getter]
    fn omitted_readings(&self) -> Vec<&'static str> {
        self.inner.omitted_readings.clone()
    }

    /// The measures that therefore said nothing: the consequence rather than the cause.
    #[getter]
    fn silent_measures(&self) -> Vec<&'static str> {
        self.inner.silent_measures()
    }

    /// Whether any measure that **can reach `Concern`** was silent.
    ///
    /// `report.erosion == "None" and report.blind_to_concern` is *no news, and I was not
    /// looking* — a distinction nothing else here can draw, since a component with
    /// nothing to say and a component with nothing to say it *from* both contribute
    /// nothing to the level. `rites` and `observers` never set this: rite erosion is
    /// capped at `Notice`, and the watcher lens is read by no level at all.
    #[getter]
    fn blind_to_concern(&self) -> bool {
        self.inner.blind_to_concern()
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
    sustained_absence_watchers=None,
    guardians=None,
    affirmations=None,
    trends=None,
    window=None,
    ground=None,
    observers=None,
    rites=None,
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
    sustained_absence_watchers: Option<usize>,
    guardians: Option<&PyGuardianState>,
    affirmations: Option<&PyAffirmationState>,
    trends: Option<&PyDecayTrends>,
    window: Option<&PyObservationWindow>,
    ground: Option<&PyGroundStanding>,
    observers: Option<&PyIndependentObservation>,
    rites: Option<&PyRiteState>,
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
        sustained_absence_watchers: sustained_absence_watchers
            .unwrap_or(d.sustained_absence_watchers),
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
        observers: observers.map(|o| &o.inner),
        rites: rites.map(|r| r.inner.as_slice()),
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

// ---- The host's cadence (D20), bound whole (D31) -----------------------------

/// How often a host attends to each duty, and on what terms (design **D20**).
///
/// The cadences are policy rather than charter: a polity's charter does not say how
/// often its host wakes up. Threshold and retention overrides are the same flattened
/// keywords `cohesion_report` and `Archive.prune_recommendation` take, because a host
/// configured one way and a hand call configured another would be two answers to one
/// question.
///
///     config = HostConfig(polity_did, cohesion_cadence_seconds=3600)
///     cycle = HostCycle(config)
#[pyclass(skip_from_py_object, name = "HostConfig")]
#[derive(Clone)]
struct PyHostConfig {
    inner: HostConfig,
}

#[pymethods]
impl PyHostConfig {
    #[new]
    #[pyo3(signature = (
        polity_did,
        opaque_boundary=false,
        cohesion_cadence_seconds=None,
        retention_cadence_seconds=None,
        save_cadence_seconds=None,
        observation_window_seconds=None,
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
        sustained_absence_watchers=None,
        retain_seconds=None,
        min_dropped=None,
        live_measure_seconds=None,
        min_witness_watchers=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        polity_did: &str,
        opaque_boundary: bool,
        cohesion_cadence_seconds: Option<i64>,
        retention_cadence_seconds: Option<i64>,
        save_cadence_seconds: Option<i64>,
        observation_window_seconds: Option<i64>,
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
        sustained_absence_watchers: Option<usize>,
        retain_seconds: Option<i64>,
        min_dropped: Option<usize>,
        live_measure_seconds: Option<i64>,
        min_witness_watchers: Option<usize>,
    ) -> PyResult<Self> {
        // `HostConfig::new` derives two of its cadences from the thresholds and the
        // retention policy, so those are settled first and the derived defaults are
        // read back off the result rather than restated here.
        let d = Thresholds::default();
        let dr = RetentionPolicy::default();
        let mut inner = HostConfig::new(parse_did(polity_did)?);
        inner.thresholds = Thresholds {
            reputation_floor: reputation_floor.unwrap_or(d.reputation_floor),
            attrition_notice: attrition_notice.unwrap_or(d.attrition_notice),
            attrition_concern: attrition_concern.unwrap_or(d.attrition_concern),
            mean_reputation_notice: mean_reputation_notice.unwrap_or(d.mean_reputation_notice),
            mean_reputation_concern: mean_reputation_concern
                .unwrap_or(d.mean_reputation_concern),
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
            sustained_absence_watchers: sustained_absence_watchers
                .unwrap_or(d.sustained_absence_watchers),
        };
        inner.retention = RetentionPolicy {
            retain_seconds: retain_seconds.unwrap_or(dr.retain_seconds),
            min_dropped: min_dropped.unwrap_or(dr.min_dropped),
            live_measure_seconds: live_measure_seconds.unwrap_or(dr.live_measure_seconds),
            min_witness_watchers: min_witness_watchers.unwrap_or(dr.min_witness_watchers),
        };
        inner.opaque_boundary = opaque_boundary;
        // The two derived defaults track whatever the caller just set: the grade runs at
        // the attendance cadence (its fastest-moving input) and the window spans the
        // retention (everything the polity chose to keep, and no more).
        inner.cohesion_cadence_seconds =
            cohesion_cadence_seconds.unwrap_or(inner.thresholds.attendance_cadence_seconds);
        inner.observation_window_seconds =
            observation_window_seconds.unwrap_or(inner.retention.retain_seconds);
        if let Some(s) = retention_cadence_seconds {
            inner.retention_cadence_seconds = s;
        }
        if let Some(s) = save_cadence_seconds {
            inner.save_cadence_seconds = s;
        }
        Ok(PyHostConfig { inner })
    }

    /// The polity this host runs.
    #[getter]
    fn polity(&self) -> String {
        self.inner.polity.as_str().to_string()
    }

    /// Whether the substrate's boundary is opaque, which caps attrition at `Notice`.
    #[getter]
    fn opaque_boundary(&self) -> bool {
        self.inner.opaque_boundary
    }

    #[getter]
    fn cohesion_cadence_seconds(&self) -> i64 {
        self.inner.cohesion_cadence_seconds
    }

    #[getter]
    fn retention_cadence_seconds(&self) -> i64 {
        self.inner.retention_cadence_seconds
    }

    #[getter]
    fn save_cadence_seconds(&self) -> i64 {
        self.inner.save_cadence_seconds
    }

    #[getter]
    fn observation_window_seconds(&self) -> i64 {
        self.inner.observation_window_seconds
    }

    /// What the thresholds resolved to, over the same keys as `DEFAULT_THRESHOLDS`.
    #[getter]
    fn thresholds<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        thresholds_dict(py, &self.inner.thresholds)
    }

    /// What the retention policy resolved to, over the same keys as
    /// `DEFAULT_RETENTION`.
    #[getter]
    fn retention<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        retention_dict(py, &self.inner.retention)
    }

    fn __repr__(&self) -> String {
        format!(
            "HostConfig(polity={}, cohesion_cadence_seconds={}, \
             retention_cadence_seconds={}, save_cadence_seconds={}, \
             observation_window_seconds={})",
            self.inner.polity.as_str(),
            self.inner.cohesion_cadence_seconds,
            self.inner.retention_cadence_seconds,
            self.inner.save_cadence_seconds,
            self.inner.observation_window_seconds,
        )
    }
}

/// Something the host should do now. **Returned, never performed.**
///
/// `kind` is `"Save"`, `"Cohesion"`, `"RaiseConcern"` or `"Prune"`. `report` carries the
/// graded `CohesionReport` for the middle two and `recommendation` the
/// `PruneRecommendation` for the last; exactly one of them is set, and neither is for a
/// `Save`.
///
/// Raising a Concern and pruning both need keys, and the keys are the host's: the
/// polity still ratifies its own housekeeping, and alarm-raising is signed by whoever
/// raises it. So this is a report over a computed state and not a record.
#[pyclass(skip_from_py_object, name = "DueAction")]
#[derive(Clone)]
struct PyDueAction {
    /// A short, stable name: `"Save"`, `"Cohesion"`, `"RaiseConcern"`, `"Prune"`.
    #[pyo3(get)]
    kind: &'static str,
    /// The grade, for `"Cohesion"` and `"RaiseConcern"`.
    #[pyo3(get)]
    report: Option<PyCohesionReport>,
    /// The terms, for `"Prune"`.
    #[pyo3(get)]
    recommendation: Option<PyPruneRecommendation>,
}

#[pymethods]
impl PyDueAction {
    fn __repr__(&self) -> String {
        format!("DueAction({})", self.kind)
    }
}

impl PyDueAction {
    fn from_rs(action: DueAction) -> Self {
        let kind = action.kind();
        match action {
            DueAction::Save => PyDueAction { kind, report: None, recommendation: None },
            DueAction::Cohesion(inner) | DueAction::RaiseConcern(inner) => PyDueAction {
                kind,
                report: Some(PyCohesionReport { inner }),
                recommendation: None,
            },
            DueAction::Prune(inner) => PyDueAction {
                kind,
                report: None,
                recommendation: Some(PyPruneRecommendation { inner }),
            },
        }
    }
}

/// A host's cadence state: what it last attended to, and when (design **D20**).
///
/// **`tick` decides and computes; the host signs and does I/O.** Nothing here writes a
/// store, signs a record or reads a clock: `as_of` arrives like every other cadence in
/// this tier, so the same cycle drives a simulated timeline in a test and a real one in
/// production with identical results.
///
///     cycle = HostCycle(HostConfig(polity_did))
///     for action in cycle.tick(archive, candidate, as_of=now):
///         if action.kind == "Save":
///             store.save(archive)
///         elif action.kind == "RaiseConcern":
///             record = coherence_concern(action.report, polity_did, observer)
///
/// `grade` is exposed in its own right because it is the assembly a caller otherwise
/// has to know to perform: it supplies **every** read-model the archive can offer, and
/// an omitted one makes its component report nothing rather than complain. Since
/// **D31** a report says which readings it was given, so a hand-assembled grade is at
/// least legible; this is how you avoid needing that.
#[pyclass(name = "HostCycle")]
struct PyHostCycle {
    inner: HostCycle,
}

#[pymethods]
impl PyHostCycle {
    #[new]
    fn new(config: &PyHostConfig) -> Self {
        PyHostCycle { inner: HostCycle::new(config.inner.clone()) }
    }

    /// What is due at `as_of` (epoch seconds, default now).
    ///
    /// Everything is due on the first tick. A duty is marked attended when it is
    /// *computed*, not when the host acts on the result, so ignoring a returned action
    /// does not make the host spin on it; the next cadence recomputes. A polity that is
    /// not yet chartered or already dissolved gets `Save` and nothing else, since the
    /// log is what remains of it.
    #[pyo3(signature = (archive, candidate, as_of=None))]
    fn tick(
        &mut self,
        archive: &PyArchive,
        candidate: &PyFoundingCandidate,
        as_of: Option<i64>,
    ) -> PyResult<Vec<PyDueAction>> {
        let due = self.inner.tick(&archive.inner, &candidate.inner, ts_or_now(as_of)?);
        Ok(due.into_iter().map(PyDueAction::from_rs).collect())
    }

    /// Grade cohesion with **every** read-model the archive can supply.
    ///
    /// The safe form of `cohesion_report`: assembling the seven readings is the part a
    /// host would otherwise have to know to do, and omitting one is silent in the
    /// direction of health. The returned report's `omitted_readings` is empty, and its
    /// own test asserts so.
    #[pyo3(signature = (archive, candidate, as_of=None))]
    fn grade(
        &self,
        archive: &PyArchive,
        candidate: &PyFoundingCandidate,
        as_of: Option<i64>,
    ) -> PyResult<PyCohesionReport> {
        let inner = self.inner.grade(&archive.inner, &candidate.inner, ts_or_now(as_of)?);
        Ok(PyCohesionReport { inner })
    }

    /// The configuration this cycle runs on.
    #[getter]
    fn config(&self) -> PyHostConfig {
        PyHostConfig { inner: self.inner.config().clone() }
    }

    fn __repr__(&self) -> String {
        format!("HostCycle(polity={})", self.inner.config().polity.as_str())
    }
}

// ---- Uplift: the interface to kith-covenant ---------------------------------

/// The bundle a fission emits, each artifact as JSON a KC-side consumer appends
/// verbatim: the successor identities, one successor link per successor
/// (index-aligned with them), and the predecessor's dissolution naming them all.
///
/// **N-ary since design D25**, so this is the exact mirror of `FusionOutcome`.
#[pyclass(skip_from_py_object, name = "FissionOutcome", get_all)]
#[derive(Clone)]
struct PyFissionOutcome {
    successors: Vec<String>,
    links: Vec<String>,
    dissolution: String,
}

/// The bundle a fusion emits: the merged identity, plus one successor link and
/// one dissolution per predecessor, index-aligned with the input list.
#[pyclass(skip_from_py_object, name = "FusionOutcome", get_all)]
#[derive(Clone)]
struct PyFusionOutcome {
    fused: String,
    links: Vec<String>,
    dissolutions: Vec<String>,
}

/// What a polity hands KC so KC can hold its rite: the authorizations and the draft,
/// never the signatures.
///
/// `recitation` is **unsigned** on purpose. Sealing it needs the reciter's key and
/// every witness's, and this seam holds nobody's key but the polity root's, so
/// whoever runs the ceremony signs. Each field is JSON a KC-side consumer appends
/// or signs verbatim.
#[pyclass(skip_from_py_object, name = "RiteHandoff", get_all)]
#[derive(Clone)]
struct PyRiteHandoff {
    reciter: String,
    witnesses: String,
    recitation: String,
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
    /// Guardian changes built but not yet signed, keyed by the CID of the bytes the
    /// signers are being asked to sign (D15's detached path).
    ///
    /// Process state, deliberately: it holds only what is *in flight*, and the
    /// exchange is resumable without it — `pending_guardian_change` is a pure
    /// function of its arguments, so re-running it with the same explicit
    /// `decided_at` reproduces the same CID. Pass `decided_at` rather than letting it
    /// default to now if the exchange may outlive the process.
    pending: BTreeMap<String, GuardianChange>,
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

    /// A relay declaration or withdrawal, signed by the member alone (D36).
    #[allow(clippy::too_many_arguments)]
    fn relay_endpoint(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        at_uuid: &str,
        host: &str,
        port: u16,
        declared_at: Option<i64>,
        withdrawn: bool,
        areas: Vec<String>,
    ) -> PyResult<String> {
        let signers = Signers::only(member_did, member_priv)?;
        let record = RelayEndpoint {
            polity: parse_did(polity_did)?,
            member: parse_did(member_did)?,
            at_uuid: at_uuid.to_string(),
            host: host.to_string(),
            port,
            declared_at: ts_or_now(declared_at)?,
            withdrawn,
            areas,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::RelayEndpointDeclared(signed))
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
        Self { inner: Archive::new(), pending: BTreeMap::new() }
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
        Ok(Self {
            inner: Archive::load_json_from_str(s).map_err(load_err_to_py)?,
            pending: BTreeMap::new(),
        })
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

    // ---- Content policy (D37) ---------------------------------------------

    /// Replace the polity's content policy whole (design **D37**). `policy_json`
    /// is a `ContentPolicy` as JSON: `{"operator_bound_required", "min_tier",
    /// "daily_post_cap", "labels": {label: "Allow"|"Warn"|"Hide"}}`. A majority
    /// tightens; loosening anything needs a majority + 1. Returns the entry's CID.
    #[pyo3(signature = (
        polity_did, policy_json, rationale=String::new(),
        decider_dids=Vec::new(), decider_privs=Vec::new(), decided_at=None,
    ))]
    fn amend_content_policy(
        &mut self,
        polity_did: &str,
        policy_json: &str,
        rationale: String,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let policy: ContentPolicy = serde_json::from_str(policy_json).map_err(json_err_to_py)?;
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = ContentPolicyAmendment {
            polity: parse_did(polity_did)?,
            policy,
            rationale,
            deciders: parse_dids(&signers.dids)?,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::ContentPolicyAmended(signed))
    }

    /// The polity's content policy as of `as_of` (design **D37**):
    /// `(policy_json, set_by_cid_or_None, moderator_dids)`. Only accepted
    /// amendments are read; moderators are inducted holders of `Moderator`.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn content_policy_state(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<(String, Option<String>, Vec<String>)> {
        let st = content_policy_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        let policy = serde_json::to_string(&st.policy).map_err(json_err_to_py)?;
        Ok((
            policy,
            st.set_by.map(|c| c.to_string()),
            st.moderators.iter().map(|d| d.as_str().to_string()).collect(),
        ))
    }

    // ---- Compact acts (D40) ------------------------------------------------

    /// Record the roll's decision about a kith-covenant compact (design **D40**).
    /// `kind` is one of `Join`, `Admit`, `Sponsor`, `Refuse`, `Anathematize`,
    /// `Lift`. A majority decides; `Refuse` and `Anathematize` need a majority + 1.
    /// A sponsorship names `probation_days`. Returns the entry's CID.
    #[pyo3(signature = (
        polity_did, kind, compact, counterparty_dids, decider_dids, decider_privs,
        probation_days=None, rationale=String::new(), decided_at=None, expires_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn decide_compact_act(
        &mut self,
        polity_did: &str,
        kind: &str,
        compact: String,
        counterparty_dids: Vec<String>,
        decider_dids: Vec<String>,
        decider_privs: Vec<Vec<u8>>,
        probation_days: Option<u32>,
        rationale: String,
        decided_at: Option<i64>,
        expires_at: Option<i64>,
    ) -> PyResult<String> {
        let kind: CompactActKind =
            serde_json::from_value(serde_json::Value::String(kind.into())).map_err(json_err_to_py)?;
        let signers = Signers::new(decider_dids, decider_privs)?;
        let record = CompactAct {
            polity: parse_did(polity_did)?,
            kind,
            compact,
            counterparties: parse_dids(&counterparty_dids)?,
            probation_days,
            rationale,
            deciders: parse_dids(&signers.dids)?,
            decided_at: ts_or_now(decided_at)?,
            expires_at: expires_at.map(|t| ts_or_now(Some(t))).transpose()?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::CompactActDecided(signed))
    }

    /// The compact acts in force as of `as_of` (design **D40**), as JSON: a list of
    /// `{"cid", "act"}` in log order. Only acts the authority pass accepts are read.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn compact_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<String> {
        let st = compact_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        let list: Vec<serde_json::Value> = st
            .acts
            .iter()
            .map(|(cid, a)| {
                Ok(serde_json::json!({
                    "cid": cid.to_string(),
                    "act": serde_json::to_value(a).map_err(json_err_to_py)?,
                }))
            })
            .collect::<PyResult<_>>()?;
        serde_json::to_string(&list).map_err(json_err_to_py)
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
        Ok(Self { inner, pending: BTreeMap::new() })
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
    /// `min_witness_watchers` for the *witness count* it also needs. Both are
    /// required: a sustained absence is graded on two conditions, and a duration only
    /// protects one of them.
    #[pyo3(signature = (
        polity_did, retain_seconds=None, min_dropped=None, live_measure_seconds=None,
        min_witness_watchers=None, as_of=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn prune_recommendation(
        &self,
        polity_did: &str,
        retain_seconds: Option<i64>,
        min_dropped: Option<usize>,
        live_measure_seconds: Option<i64>,
        min_witness_watchers: Option<usize>,
        as_of: Option<i64>,
    ) -> PyResult<Option<PyPruneRecommendation>> {
        let d = RetentionPolicy::default();
        let policy = RetentionPolicy {
            retain_seconds: retain_seconds.unwrap_or(d.retain_seconds),
            min_dropped: min_dropped.unwrap_or(d.min_dropped),
            live_measure_seconds: live_measure_seconds.unwrap_or(d.live_measure_seconds),
            min_witness_watchers: min_witness_watchers.unwrap_or(d.min_witness_watchers),
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
        guardian_did=None, guardian_priv=None, evidence_cid=None, unbacked_reason=None,
        rationale=String::new(), decided_at=None,
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
        evidence_cid: Option<&str>,
        unbacked_reason: Option<String>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let evidence = resolve_evidence(&self.inner, evidence_cid, unbacked_reason)?;
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
            evidence,
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
        evidence_cid=None, unbacked_reason=None, rationale=String::new(), decided_at=None,
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
        evidence_cid: Option<&str>,
        unbacked_reason: Option<String>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let evidence = resolve_evidence(&self.inner, evidence_cid, unbacked_reason)?;
        let mut signers = Signers::new(decider_dids, decider_privs)?;
        let deciders = parse_dids(&signers.dids)?;
        signers.consent(guardian_did, guardian_priv)?;
        let record = GuardianChange {
            polity: parse_did(polity_did)?,
            kind: GuardianKind::Rotate,
            member: parse_did(member_did)?,
            guardian: Some(parse_did(guardian_did)?),
            evidence,
            rationale,
            deciders,
            decided_at: ts_or_now(decided_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::GuardianChanged(signed))
    }

    /// **Phase 1 of detached signing (D15):** build a guardian change without signing
    /// it, and return what the signers need — `(cid, payload_bytes, description)`.
    ///
    /// Use this when the guardian's key is not on this host, which is the arrangement
    /// AT's keystore location assumes: one key per operator, living with the operator.
    /// The bytes travel to the guardian, they sign in their own process, and
    /// [`append_guardian_change_signed`] puts the signatures back on. The private key
    /// never enters this process.
    ///
    /// **Show the description to whoever is signing.** Asking a human to sign opaque
    /// bytes teaches them to sign anything.
    ///
    /// Pass an explicit `decided_at` if the exchange may outlive this process: the
    /// pending record is held in memory, and re-running this call reproduces the same
    /// CID only if every argument (including the timestamp) is the same.
    #[pyo3(signature = (
        polity_did, member_did, kind, decider_dids, guardian_did=None,
        evidence_cid=None, unbacked_reason=None, rationale=String::new(), decided_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn pending_guardian_change(
        &mut self,
        polity_did: &str,
        member_did: &str,
        kind: &str,
        decider_dids: Vec<String>,
        guardian_did: Option<&str>,
        evidence_cid: Option<&str>,
        unbacked_reason: Option<String>,
        rationale: String,
        decided_at: Option<i64>,
    ) -> PyResult<(String, Vec<u8>, String)> {
        let kind = match kind {
            "Designate" => GuardianKind::Designate,
            "Rotate" => GuardianKind::Rotate,
            "Release" => GuardianKind::Release,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown guardian change kind {other:?}; expected Designate, Rotate or Release"
                )))
            }
        };
        let evidence = resolve_evidence(&self.inner, evidence_cid, unbacked_reason)?;
        let record = GuardianChange {
            polity: parse_did(polity_did)?,
            kind,
            member: parse_did(member_did)?,
            guardian: guardian_did.map(parse_did).transpose()?,
            evidence,
            rationale,
            deciders: parse_dids(&decider_dids)?,
            decided_at: ts_or_now(decided_at)?,
        };
        let description = en_charter::describe_guardian_change(&record);
        let p = en_charter::pending(record, description).map_err(en_err_to_py)?;
        let cid = p.cid.to_string();
        self.pending.insert(cid.clone(), p.payload);
        Ok((cid, p.bytes, p.description))
    }

    /// **Phase 2 of detached signing (D15):** attach the collected signatures to a
    /// pending guardian change and append it.
    ///
    /// `signatures` is a list of `(signer_did, signature_bytes)`. Every signature is
    /// verified against the pending payload *before* the record is appended, so a
    /// wrong key, a stale exchange or a corrupted signature fails here with a reason
    /// rather than surfacing later as an unauthorized record.
    fn append_guardian_change_signed(
        &mut self,
        cid: &str,
        signatures: Vec<(String, Vec<u8>)>,
    ) -> PyResult<String> {
        // Cloned rather than removed: a failed assembly is usually a signature the
        // caller will fix and retry, and dropping the pending record would make them
        // rebuild it — risking a different `decided_at` and so a different CID.
        let record = self.pending.get(cid).cloned().ok_or_else(|| {
            PyValueError::new_err(format!(
                "no pending guardian change {cid} in this process; call \
                 pending_guardian_change first (and pass an explicit decided_at if the \
                 exchange spans a restart)"
            ))
        })?;
        let signed = en_charter::assemble(record, &signatures).map_err(en_err_to_py)?;
        let appended = self.append(CharterEntry::GuardianChanged(signed))?;
        self.pending.remove(cid);
        Ok(appended)
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
            // A release claims the edge is *gone*; carrying evidence of one would
            // argue the opposite, so the rule forbids it and this never sets it.
            evidence: None,
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

    // ---- Transmission: the rite, then induction and re-affirmation ---------

    /// Recite one movement of a member's induction rite (**D29**).
    ///
    /// `movement` is one of `"Substrate"`, `"Fallenness"`, `"Recognition"`,
    /// `"CommonVirtue"`, `"NonUltimacy"`; order is semantic, so a rite opens with
    /// `Substrate` and each later movement must follow its predecessor. `statement` is
    /// the polity's own words and is required but never read. What *is* checked is
    /// coherence with the charter: `names` must be the chartered substrate's name,
    /// `commits_to` the commitments in force, `names_anchor` the chartered anchor.
    /// Everyone attending co-signs alongside the inductee, and attendees must already
    /// hold the substrate. Returns the movement's CID, which the induction cites.
    #[pyo3(signature = (
        polity_did, member_did, member_priv, movement, statement, attending_dids,
        attending_privs, names=None, commits_to=Vec::new(), names_anchor=None,
        recited_at=None, kind="Induction",
    ))]
    #[allow(clippy::too_many_arguments)]
    fn recite_movement(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        movement: &str,
        statement: String,
        attending_dids: Vec<String>,
        attending_privs: Vec<Vec<u8>>,
        names: Option<String>,
        commits_to: Vec<String>,
        names_anchor: Option<String>,
        recited_at: Option<i64>,
        kind: &str,
    ) -> PyResult<String> {
        let mut signers = Signers::new(attending_dids, attending_privs)?;
        let attending = parse_dids(&signers.dids)?;
        signers.consent(member_did, member_priv)?;
        let movement = match movement {
            "Substrate" => Movement::Substrate {
                names: names.ok_or_else(|| {
                    PyValueError::new_err("the Substrate movement must name the substrate")
                })?,
            },
            "Fallenness" => Movement::Fallenness,
            "Recognition" => Movement::Recognition,
            "CommonVirtue" => Movement::CommonVirtue { commits_to },
            "NonUltimacy" => Movement::NonUltimacy {
                names_anchor: names_anchor.as_deref().map(parse_did).transpose()?,
            },
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown movement '{other}': expected Substrate, Fallenness, Recognition, \
                     CommonVirtue or NonUltimacy"
                )))
            }
        };
        let record = RiteMovement {
            polity: parse_did(polity_did)?,
            member: parse_did(member_did)?,
            kind: affirmation_kind(kind)?,
            movement,
            statement,
            attending,
            recited_at: ts_or_now(recited_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::RiteMovementRecorded(signed))
    }

    /// Complete a member's induction into the substrate — the step admission does
    /// *not* perform.
    ///
    /// Since **D29** an induction completes a *rite*: `movement_cids` must be the five
    /// [`Self::recite_movement`] records in order, the witnesses must be members who
    /// attended every one of them, and they must number a majority of the members who
    /// already hold the substrate. The charter's ratifiers need no induction —
    /// ratifying was theirs.
    #[pyo3(signature = (
        polity_did, member_did, member_priv, witness_dids, witness_privs, movement_cids,
        affirmed_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn induct_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        witness_dids: Vec<String>,
        witness_privs: Vec<Vec<u8>>,
        movement_cids: Vec<String>,
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
            movements: parse_cids(&movement_cids)?,
            affirmed_at: ts_or_now(affirmed_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::MemberAffirmed(signed))
    }

    /// Renew an inducted member's affirmation, on the **full re-recitation of the five
    /// movements** the doctrine asks of it (**D34**).
    ///
    /// `movement_cids` must be the five `recite_movement` records of this member's own
    /// re-recitation (`kind="ReAffirmation"`), in order, and `witness_dids` exactly those
    /// who attended every one of them — which may be nobody. Attendance is recorded,
    /// never required: the renewal is signed by the member alone, because it is precisely
    /// the act no one else may condition.
    ///
    /// The freshness of these is what `AffirmationState.stale_members` reads. Before D34
    /// this took no re-recitation at all, which made the recurring ceremony the only one
    /// with no cost — see D34 for what that measured.
    #[pyo3(signature = (
        polity_did, member_did, member_priv, movement_cids, witness_dids=Vec::new(),
        affirmed_at=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn reaffirm_member(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        movement_cids: Vec<String>,
        witness_dids: Vec<String>,
        affirmed_at: Option<i64>,
    ) -> PyResult<String> {
        let signers = Signers::only(member_did, member_priv)?;
        let record = Affirmation {
            polity: parse_did(polity_did)?,
            member: parse_did(member_did)?,
            kind: AffirmationKind::ReAffirmation,
            witnesses: parse_dids(&witness_dids)?,
            movements: parse_cids(&movement_cids)?,
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
        self.attest_independence(
            polity_did,
            reader_did,
            attester_did,
            attester_priv,
            "BoundaryReader",
            kind,
            basis,
            attested_at,
        )
    }

    /// Who has vouched for `polity_did`'s **monitors**, as of `as_of` (**D28**).
    ///
    /// The observer counterpart of `reader_independence`, same three buckets and the
    /// same stake rule. Read it beside `ObservationWindow.watchers_for`, which counts
    /// watchers without asking who they are.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn observer_independence(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyReaderIndependence> {
        let r = observer_independence_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyReaderIndependence { inner: r })
    }

    /// Which of the monitors that have observed `polity_did` are independently
    /// vouched for, as of `as_of` (design **D28**).
    ///
    /// A polity watched by five monitors that are five faces of one actor reads five
    /// distinct watchers and `0` here. Nothing refuses an unvouched monitor — a
    /// polity that simply declines to vouch for anybody must not become unwatchable.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn independent_observation(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyIndependentObservation> {
        let o = independent_observation_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyIndependentObservation { inner: o })
    }

    /// Of the founding edges that earned `Guarded`, which rest on a monitor somebody
    /// outside has vouched for (design **D28**).
    ///
    /// D27 required the founding observer to be outside the polity and deliberately
    /// did not ask *who* — requiring a vouched-for monitor at founding would refuse
    /// newness. This reports the residue rather than changing the grade, and it is a
    /// separate accruing model so `founding_legitimacy` keeps its charter-alone
    /// property.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn independent_grounding(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyIndependentGrounding> {
        let g = independent_grounding_rs(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyIndependentGrounding { inner: g })
    }

    /// File a third party's account of whether `subject_did` — a party behind
    /// `polity_did` in the named `role` — is an independent principal (**D24**,
    /// generalized to observers by **D28**).
    ///
    /// `role` is `"BoundaryReader"` or `"Observer"`; `kind` is `"Independent"`
    /// (vouching) or `"Controlled"` (the counter-claim). The attester signs alone, and
    /// the subject must actually appear in the record in that role — among the
    /// founding's readings for a reader, among the observations filed so far for a
    /// monitor. A monitor must therefore have **spoken before it can be vouched for**:
    /// pre-registering a witness is the sockpuppet move with a longer setup.
    #[pyo3(signature = (polity_did, subject_did, attester_did, attester_priv, role="BoundaryReader", kind="Independent", basis="", attested_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn attest_independence(
        &mut self,
        polity_did: &str,
        subject_did: &str,
        attester_did: &str,
        attester_priv: &[u8],
        role: &str,
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
        let role = match role {
            "BoundaryReader" => AttestedRole::BoundaryReader,
            "Observer" => AttestedRole::Observer,
            other => {
                return Err(PyValueError::new_err(format!(
                    "unknown attested role {other:?} (expected BoundaryReader or Observer)"
                )))
            }
        };
        let record = IndependenceAttestation {
            polity: parse_did(polity_did)?,
            subject: parse_did(subject_did)?,
            role,
            attesting_party: parse_did(attester_did)?,
            kind,
            basis: basis.to_string(),
            attested_at: ts_or_now(attested_at)?,
        };
        let signed = Signed::build(record, &signers.pairs()).map_err(en_err_to_py)?;
        self.append(CharterEntry::IndependenceAttested(signed))
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

    /// The polity's rites as of `as_of` — the movements recited, each rite's span, who
    /// attended throughout, and which rites were begun and never finished (**D29**).
    #[pyo3(signature = (polity_did, as_of=None))]
    fn rite_state(&self, polity_did: &str, as_of: Option<i64>) -> PyResult<PyRiteState> {
        let st = rite_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(PyRiteState { inner: st })
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

    /// A rendezvous operator states where its relay listens (design **D36**).
    /// Signed by the member alone; the authority pass refuses it unless the member
    /// holds `RendezvousOperator`. `at_uuid` is the member node's AT identity uuid,
    /// lowercase and hyphenated. `areas` (D38) makes the relay an area hub for
    /// those geohash areas. Returns the entry's CID.
    #[pyo3(signature = (polity_did, member_did, member_priv, at_uuid, host, port, declared_at=None, areas=None))]
    #[allow(clippy::too_many_arguments)]
    fn declare_relay_endpoint(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        at_uuid: &str,
        host: &str,
        port: u16,
        declared_at: Option<i64>,
        areas: Option<Vec<String>>,
    ) -> PyResult<String> {
        self.relay_endpoint(polity_did, member_did, member_priv, at_uuid, host, port, declared_at,
                            false, areas.unwrap_or_default())
    }

    /// The member takes back its relay declaration. Needs no office, so a revoked
    /// operator can still withdraw. Returns the entry's CID.
    #[pyo3(signature = (polity_did, member_did, member_priv, at_uuid, declared_at=None))]
    fn withdraw_relay_endpoint(
        &mut self,
        polity_did: &str,
        member_did: &str,
        member_priv: &[u8],
        at_uuid: &str,
        declared_at: Option<i64>,
    ) -> PyResult<String> {
        self.relay_endpoint(polity_did, member_did, member_priv, at_uuid, "", 0, declared_at, true,
                            vec![])
    }

    /// The relays the polity runs as of `as_of` (design **D36**): a list of
    /// `(member_did, at_uuid, host, port, declared_at, areas)`, one per operator who
    /// holds `RendezvousOperator` then and has a live declaration, sorted by member
    /// DID; `areas` (D38) is empty for a relay that is no hub.
    #[pyo3(signature = (polity_did, as_of=None))]
    fn rendezvous_state(
        &self,
        polity_did: &str,
        as_of: Option<i64>,
    ) -> PyResult<Vec<(String, String, String, u16, i64, Vec<String>)>> {
        let st = rendezvous_state(&self.inner, &parse_did(polity_did)?, ts_or_now(as_of)?);
        Ok(st
            .relays
            .into_iter()
            .map(|r| {
                (r.member.as_str().to_string(), r.at_uuid, r.host, r.port, r.declared_at.as_second(),
                 r.areas)
            })
            .collect())
    }

    /// Emit the polity's relay roster as AT's signed `at-relay-roster` file
    /// (border artifact #7, design **D36**), read from this log as of `as_of`.
    /// Returns the file's exact text, for the host to write where its AT nodes
    /// read rosters. `seq` must exceed every seq published before; the seconds of
    /// `as_of` are a natural choice. An empty roster is valid and replaces the
    /// previous one.
    #[pyo3(signature = (polity_did, root_priv, root_pub, seq, as_of=None))]
    fn rendezvous_roster(
        &self,
        polity_did: &str,
        root_priv: &[u8],
        root_pub: &[u8],
        seq: u64,
        as_of: Option<i64>,
    ) -> PyResult<String> {
        let polity = parse_did(polity_did)?;
        let charter = self.charter_of(&polity)?;
        let state = rendezvous_state(&self.inner, &polity, ts_or_now(as_of)?);
        en_uplift::rendezvous_roster(
            &charter,
            &state,
            &priv_from_bytes(root_priv),
            &pub_from_bytes(root_pub),
            seq,
        )
        .map_err(en_err_to_py)
    }

    /// Emit KC's `MemberRoll` (JSON) for this polity — the border artifact that gives
    /// it a real anonymous whistleblower channel. The roll is read from this log as of
    /// `as_of` (the founding roll plus admissions minus departures), so it names
    /// exactly who is a member then; each member's verifying key comes from its own
    /// `did:key`.
    ///
    /// KC **unions** rolls rather than superseding them, so publishing a later,
    /// shorter roll removes nobody from its insider ring: a member who has left keeps
    /// the channel they had. Republish whenever the roll grows.
    #[pyo3(signature = (polity_did, root_priv, root_pub, as_of=None, published_at=None))]
    fn member_roll(
        &self,
        polity_did: &str,
        root_priv: &[u8],
        root_pub: &[u8],
        as_of: Option<i64>,
        published_at: Option<i64>,
    ) -> PyResult<String> {
        let polity = parse_did(polity_did)?;
        let charter = self.charter_of(&polity)?;
        let as_of = ts_or_now(as_of)?;
        let state = charter_state(&self.inner, &polity, as_of);
        let published_at = match published_at {
            Some(p) => ts(p)?,
            None => as_of,
        };
        let signed = en_uplift::member_roll(
            &charter,
            &state.members,
            &priv_from_bytes(root_priv),
            pub_from_bytes(root_pub),
            as_of,
            published_at,
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

    /// Emit the records KC's intra-community rite is held to: the reciter
    /// delegation, the witness registry, and the drafted recitation (each JSON).
    ///
    /// The polity's **Inductor** recites — the office that presides over the polity's
    /// own rite is the office that carries one across the border, and KC's `Envoy`
    /// scope is for representation and deliberately not this. The **attendees**
    /// witness: the members present for every movement of the polity's own rite, so
    /// the KC record answers to the people who were there rather than to whoever holds
    /// a key. The five movements are drafted from the charter, and the polity's
    /// in-force commitment keys travel into Movement 4 verbatim.
    ///
    /// A rite is not mintable from nothing. This is an error unless a rite of the
    /// polity's stands complete as of `at`, unless `inductor_did` holds the Inductor
    /// office and has been inducted, and unless the polity has commitments in force.
    ///
    /// Nothing is signed here but the two authorizations, under the polity root. Feed
    /// `recitation` to KC's `IntraCommunityRiteRunner`: the reciter signs it, each
    /// witness signs its own attestation, and the runner seals.
    #[pyo3(signature = (polity_did, inductor_did, root_priv, root_pub, at=None))]
    fn intra_rite(
        &self,
        polity_did: &str,
        inductor_did: &str,
        root_priv: &[u8],
        root_pub: &[u8],
        at: Option<i64>,
    ) -> PyResult<PyRiteHandoff> {
        let polity = parse_did(polity_did)?;
        let charter = self.charter_of(&polity)?;
        let at = ts_or_now(at)?;
        let state = charter_state(&self.inner, &polity, at);
        let offices = office_state(&self.inner, &polity, at);
        let affirmations = affirmation_state(&self.inner, &polity, at);
        let rites = rite_state(&self.inner, &polity, at);
        let handoff = en_uplift::intra_rite(
            &charter,
            &state.commitments,
            &offices,
            &affirmations,
            &rites,
            &parse_did(inductor_did)?,
            &priv_from_bytes(root_priv),
            pub_from_bytes(root_pub),
            at,
        )
        .map_err(en_err_to_py)?;
        Ok(PyRiteHandoff {
            reciter: to_json(&handoff.reciter)?,
            witnesses: to_json(&handoff.witnesses)?,
            recitation: to_json(&handoff.recitation)?,
        })
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
#[pyclass(skip_from_py_object, name = "CharterState", get_all)]
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

    /// How many **filings** in the window saw `did`. An honest count of records, and
    /// deliberately not what any grade reads — see `watchers_for`.
    fn readings_for(&self, did: &str) -> PyResult<usize> {
        Ok(self.inner.readings_for(&parse_did(did)?))
    }

    /// How many **distinct observers** saw `did` — the per-member evidence of
    /// watching, and what the sustained-absence grade is held to (**D28**).
    ///
    /// Three filings by one key are one watcher. Before D28 the grade read
    /// `readings_for`, so a stranger with one keypair could manufacture the `Concern`
    /// that crosses to kith-covenant.
    fn watchers_for(&self, did: &str) -> PyResult<usize> {
        Ok(self.inner.watchers_for(&parse_did(did)?))
    }

    /// The distinct observers that saw `did`.
    fn watchers_of(&self, did: &str) -> PyResult<Vec<String>> {
        Ok(self
            .inner
            .watchers_of(&parse_did(did)?)
            .into_iter()
            .map(str::to_string)
            .collect())
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

#[pyclass(skip_from_py_object, name = "RiteState")]
#[derive(Clone)]
struct PyRiteState {
    inner: Vec<Rite>,
}

#[pymethods]
impl PyRiteState {
    /// Every rite, oldest first, as
    /// `(member, kind, movements_recited, span_seconds | None, completed_at | None,
    /// attended_throughout, attrition)`.
    ///
    /// `kind` is `"Induction"` or `"ReAffirmation"` (**D34**). It is carried because the
    /// two are not comparable as ceremonies — one happens once per member and the other
    /// recurs — so a caller measuring a rite's cost has to know which it is reading.
    #[getter]
    fn rites(
        &self,
    ) -> Vec<RiteRow> {
        self.inner
            .iter()
            .map(|r| {
                (
                    r.member.to_string(),
                    affirmation_kind_name(&r.kind),
                    r.movements.len(),
                    r.span_seconds(),
                    r.completed_at.map(|t| t.as_second()),
                    r.attended_throughout().iter().map(|w| w.to_string()).collect(),
                    r.attrition(),
                )
            })
            .collect()
    }

    /// Rites begun and never completed.
    #[getter]
    fn abandoned(&self) -> usize {
        self.inner.iter().filter(|r| r.completed_at.is_none()).count()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
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
#[pyclass(skip_from_py_object, name = "PruneRecommendation")]
#[derive(Clone)]
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
#[pyclass(skip_from_py_object, name = "Cursor")]
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

/// What a `FileStore.reconcile` did to a forked log (design **D30**).
#[pyclass(name = "Reconciled")]
struct PyReconciled {
    inner: Reconciled,
}

#[pymethods]
impl PyReconciled {
    /// Whether the log was forked at all. A healthy store reports False.
    #[getter]
    fn was_forked(&self) -> bool {
        self.inner.was_forked()
    }

    /// Trailing segments that would not apply where they claimed to.
    #[getter]
    fn segments_orphaned(&self) -> usize {
        self.inner.segments_orphaned
    }

    /// Entries lifted out of those segments and re-appended at the end of the log.
    #[getter]
    fn entries_moved(&self) -> usize {
        self.inner.entries_moved
    }

    /// Entries in those segments the log already held, by CID, and so were already
    /// safe.
    #[getter]
    fn entries_already_present(&self) -> usize {
        self.inner.entries_already_present
    }

    /// The CIDs that moved, in the order they were re-appended.
    #[getter]
    fn cids_moved(&self) -> Vec<String> {
        self.inner.cids_moved.clone()
    }

    /// Bytes written by the rewrite that leaves storage as a single base segment.
    #[getter]
    fn bytes(&self) -> usize {
        self.inner.bytes
    }

    fn __repr__(&self) -> String {
        format!(
            "Reconciled(segments_orphaned={}, entries_moved={}, \
             entries_already_present={}, bytes={})",
            self.inner.segments_orphaned,
            self.inner.entries_moved,
            self.inner.entries_already_present,
            self.inner.bytes
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
///
/// **Only one writer** (D30). Every `save` takes an exclusive lock and checks that
/// storage is still where this store left it, raising rather than writing if it is
/// not; `claim()` moves that discovery to startup. What used to happen without it was
/// silent: a second writer's compaction discarded the first's already-persisted
/// entries and every call returned successfully.
///
///     store = FileStore("polity.log")
///     store.claim()                   # raises if another host is writing
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
        Ok(PyArchive { inner, pending: BTreeMap::new() })
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

    /// Take the write lock and hold it for this store's life, so a second writer is
    /// refused **now** rather than at its first save.
    ///
    /// A host should call this at startup: a supervisor that restarted a host while the
    /// old one is still alive is the ordinary way two writers arrive on one log, and a
    /// refusal at boot is one a supervisor can act on. Writes are guarded either way.
    ///
    /// Idempotent. Released when the store is garbage-collected or the process dies, so
    /// there is nothing to clean up and no stale lock to reap.
    fn claim(&mut self) -> PyResult<()> {
        self.inner.claim().map_err(en_err_to_py)
    }

    /// Whether this store holds a lifetime write lock.
    #[getter]
    fn is_claimed(&self) -> bool {
        self.inner.is_claimed()
    }

    /// Give up a `claim()`, leaving individual writes still guarded — for a process
    /// that is done writing but still reading, and does not want a second host to wait
    /// on its exit.
    fn release(&mut self) {
        self.inner.release();
    }

    /// Where **storage** says the log ends, read from the tail alone rather than by
    /// loading it.
    ///
    /// `None` when storage is absent or empty. Compare it with `cursor()` to ask
    /// whether anything has moved underneath this store; `save` does exactly that, and
    /// refuses when the two disagree.
    fn stored_end(&self) -> PyResult<Option<PyCursor>> {
        Ok(self.inner.stored_end().map_err(en_err_to_py)?.map(|inner| PyCursor { inner }))
    }

    /// Repair a log that two writers forked, and report what the repair did.
    ///
    /// Storage can hold a segment sequence that will not fold back together, because
    /// two writers each appended their own continuation of the same log. Loading such a
    /// log raises — correctly — and `recover()` cannot help: the file is not *torn* but
    /// *forked*, every line complete. This lifts the orphaned entries out, re-appends
    /// them at the end of the surviving log, and rewrites storage as one segment.
    ///
    /// **Nothing is forgotten and no CID changes**, entries being content-addressed, so
    /// a receipt naming any of them still resolves. What the repair does decide is the
    /// order between two writers that were concurrent, and it reports which CIDs moved.
    ///
    /// Safe on a healthy log: `was_forked` is False and nothing moves.
    fn reconcile(&mut self) -> PyResult<PyReconciled> {
        let inner = ArchiveStore::<CharterEntry>::reconcile(&mut self.inner)
            .map_err(en_err_to_py)?;
        Ok(PyReconciled { inner })
    }
}

/// What a polity's founding rests on, derived from its charter alone (D21).
/// What the excluded said back about a founding — one party's assertion each, and
/// never a verdict (design **D22**).
#[pyclass(skip_from_py_object, name = "FoundingDissent")]
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

#[pyclass(skip_from_py_object, name = "GroundStanding")]
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

#[pyclass(skip_from_py_object, name = "ReaderIndependence")]
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

#[pyclass(skip_from_py_object, name = "IndependentCorroboration")]
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

#[pyclass(skip_from_py_object, name = "IndependentObservation")]
#[derive(Clone)]
struct PyIndependentObservation {
    inner: IndependentObservation,
}

#[pymethods]
impl PyIndependentObservation {
    /// Monitors that have observed this polity **and** carry a disinterested voucher.
    #[getter]
    fn observers(&self) -> Vec<String> {
        self.inner.observers.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Of those, the ones somebody has also claimed are controlled.
    #[getter]
    fn contested(&self) -> Vec<String> {
        self.inner.contested.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Monitors that have observed and carry no disinterested voucher at all.
    ///
    /// **Not a fault**, and the reason none of this is a gate: an unvouched monitor is
    /// the normal state of an honest new one. Reported so the absence of vouching is
    /// visible rather than silently read as its presence.
    #[getter]
    fn unvouched(&self) -> Vec<String> {
        self.inner.unvouched.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// The vouched-for monitors nobody has contradicted.
    #[getter]
    fn uncontested(&self) -> Vec<String> {
        self.inner.uncontested().iter().map(|r| r.as_str().to_string()).collect()
    }

    #[getter]
    fn any(&self) -> bool {
        self.inner.any()
    }

    fn __len__(&self) -> usize {
        self.inner.count()
    }

    fn __repr__(&self) -> String {
        format!(
            "IndependentObservation(vouched={}, unvouched={}, contested={})",
            self.inner.count(),
            self.inner.unvouched.len(),
            self.inner.contested.len()
        )
    }
}

#[pyclass(skip_from_py_object, name = "IndependentGrounding")]
#[derive(Clone)]
struct PyIndependentGrounding {
    inner: IndependentGrounding,
}

#[pymethods]
impl PyIndependentGrounding {
    /// Founding-derived members whose guardian edge is backed by a vouched-for monitor.
    #[getter]
    fn vouched_edges(&self) -> Vec<String> {
        self.inner.vouched_edges.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Founding-derived members whose backing monitor carries no disinterested
    /// voucher. `Guarded` all the same — this is what the grade does not say.
    #[getter]
    fn unvouched_edges(&self) -> Vec<String> {
        self.inner.unvouched_edges.iter().map(|r| r.as_str().to_string()).collect()
    }

    /// Members whose backing monitor somebody has called controlled.
    #[getter]
    fn contested_edges(&self) -> Vec<String> {
        self.inner.contested_edges.iter().map(|r| r.as_str().to_string()).collect()
    }

    #[getter]
    fn any(&self) -> bool {
        self.inner.any()
    }

    fn __repr__(&self) -> String {
        format!(
            "IndependentGrounding(vouched={}, unvouched={}, contested={})",
            self.inner.vouched_edges.len(),
            self.inner.unvouched_edges.len(),
            self.inner.contested_edges.len()
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
    retention_dict(py, &RetentionPolicy::default())
}

/// A retention policy as a dict, so `HostConfig` can report what it resolved to over
/// the same key names the defaults use (**D31**).
fn retention_dict<'py>(
    py: Python<'py>,
    d: &RetentionPolicy,
) -> PyResult<Bound<'py, PyDict>> {
    let out = PyDict::new(py);
    out.set_item("retain_seconds", d.retain_seconds)?;
    out.set_item("min_dropped", d.min_dropped)?;
    out.set_item("live_measure_seconds", d.live_measure_seconds)?;
    out.set_item("min_witness_watchers", d.min_witness_watchers)?;
    Ok(out)
}

/// `en-cohesion`'s own threshold defaults, so the Python side never carries a
/// second copy of them.
fn default_thresholds(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    thresholds_dict(py, &Thresholds::default())
}

/// Thresholds as a dict, over the same key names the defaults use (**D31**).
fn thresholds_dict<'py>(py: Python<'py>, d: &Thresholds) -> PyResult<Bound<'py, PyDict>> {
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
    out.set_item("sustained_absence_watchers", d.sustained_absence_watchers)?;
    Ok(out)
}

// ---- The autonomous_trust substrate adapter (decision D33) -------------------
//
// **Why the FOLD crosses this seam and not the peer type.** The adapter's readings could
// already be reached by hand: a Python host received AT's wire fields and built the roll
// itself through `founding_candidate(members=[...])`. Measuring that reimplementation
// against the Rust fold over one mixed feed found it right about the member count, the
// departure, the ordering and every reputation — and wrong on exactly two fields, both on
// the same peer, both toward health: it reported a `guardian` DID and an `attested_at`
// stamp for a node whose `operator_bound` was FALSE.
//
// Those two fields are gated in Rust (`AtPeer::guardian_did`, `AtPeer::attended_at`)
// because an attendance stamp on a node whose operator credential never verified attests
// to nothing. The gate is invisible from outside, so the hand-built roll read an
// **unguarded machine as guarded and attended** — defeating the office rule D15 exists
// for, fabricating D8's human-above-machine anchor out of a key AT declined to vouch for,
// and handing the cohesion monitor a freshness stamp with nothing behind it. Nothing in
// the output looked wrong. That is the D31 shape exactly.
//
// So this surface takes **wire fields**, never an assembled peer: `observe_peer` accepts
// what AT sends and the Rust fold applies the gates. There is deliberately no Python
// object whose `guardian` or `attested_at` a caller can set, which makes the measured
// divergence unrepresentable rather than merely documented.

/// One movement of the observed substrate — the delta stream design **D16** reads.
///
/// `kind` is a short stable name and the payload getters are `None` where they do not
/// apply, the same shape as [`PyDueAction`]:
///
/// | `kind` | carries |
/// |---|---|
/// | `"MemberAppeared"` | `member`, `to_reputation`, `guardian`, `attested_at`, `derived` |
/// | `"MemberVanished"` | `member` |
/// | `"ReputationChanged"` | `member`, `from_reputation`, `to_reputation` |
/// | `"GuardianEdgeChanged"` | `member`, `guardian` (`None` = the edge went away) |
/// | `"AttendanceStamped"` | `member`, `attested_at` |
/// | `"BoundaryOpacityChanged"` | `opaque` |
#[pyclass(skip_from_py_object, name = "SubstrateChange")]
#[derive(Clone)]
struct PySubstrateChange {
    #[pyo3(get)]
    kind: &'static str,
    #[pyo3(get)]
    member: Option<String>,
    #[pyo3(get)]
    from_reputation: Option<f64>,
    #[pyo3(get)]
    to_reputation: Option<f64>,
    #[pyo3(get)]
    guardian: Option<String>,
    #[pyo3(get)]
    attested_at: Option<i64>,
    #[pyo3(get)]
    derived: Option<bool>,
    #[pyo3(get)]
    opaque: Option<bool>,
}

#[pymethods]
impl PySubstrateChange {
    fn __repr__(&self) -> String {
        match &self.member {
            Some(m) => format!("SubstrateChange({}, {m})", self.kind),
            None => format!("SubstrateChange({})", self.kind),
        }
    }
}

impl PySubstrateChange {
    fn blank(kind: &'static str) -> Self {
        PySubstrateChange {
            kind,
            member: None,
            from_reputation: None,
            to_reputation: None,
            guardian: None,
            attested_at: None,
            derived: None,
            opaque: None,
        }
    }

    fn from_rs(change: SubstrateChange) -> Self {
        match change {
            SubstrateChange::MemberAppeared(m) => PySubstrateChange {
                member: Some(m.did.to_string()),
                to_reputation: m.reputation,
                guardian: m.guardian.as_ref().map(|g| g.to_string()),
                attested_at: m.attested_at.map(|t| t.as_second()),
                derived: Some(m.derived),
                ..Self::blank("MemberAppeared")
            },
            SubstrateChange::MemberVanished(did) => PySubstrateChange {
                member: Some(did.to_string()),
                ..Self::blank("MemberVanished")
            },
            SubstrateChange::ReputationChanged { member, from, to } => PySubstrateChange {
                member: Some(member.to_string()),
                from_reputation: from,
                to_reputation: to,
                ..Self::blank("ReputationChanged")
            },
            SubstrateChange::GuardianEdgeChanged { member, guardian } => PySubstrateChange {
                member: Some(member.to_string()),
                guardian: guardian.as_ref().map(|g| g.to_string()),
                ..Self::blank("GuardianEdgeChanged")
            },
            SubstrateChange::AttendanceStamped { member, attested_at } => PySubstrateChange {
                member: Some(member.to_string()),
                attested_at: Some(attested_at.as_second()),
                ..Self::blank("AttendanceStamped")
            },
            SubstrateChange::BoundaryOpacityChanged { opaque } => PySubstrateChange {
                opaque: Some(opaque),
                ..Self::blank("BoundaryOpacityChanged")
            },
        }
    }
}

/// Which event source an [`PyAtProvider`] is folding. One Python class either way, so a
/// caller's code does not change shape when a deployment swaps a replayed view for a live
/// daemon — only the constructor does.
enum AtBacking {
    Mock(AtSubstrateProvider<MockEventSource>),
    #[cfg(feature = "at-ffi")]
    Attached(AtSubstrateProvider<AttachedAtDaemon>),
    #[cfg(feature = "at-ffi")]
    Embedded(AtSubstrateProvider<EmbeddedAtNode>),
}

/// Dispatch a `&mut`/`&` expression over whichever source is behind the provider.
macro_rules! on_backing {
    ($self:expr, |$p:ident| $body:expr) => {
        match &$self.inner {
            AtBacking::Mock($p) => $body,
            #[cfg(feature = "at-ffi")]
            AtBacking::Attached($p) => $body,
            #[cfg(feature = "at-ffi")]
            AtBacking::Embedded($p) => $body,
        }
    };
}
macro_rules! on_backing_mut {
    ($self:expr, |$p:ident| $body:expr) => {
        match &mut $self.inner {
            AtBacking::Mock($p) => $body,
            #[cfg(feature = "at-ffi")]
            AtBacking::Attached($p) => $body,
            #[cfg(feature = "at-ffi")]
            AtBacking::Embedded($p) => $body,
        }
    };
}

/// The autonomous_trust substrate provider (decisions **D5**–**D8**): it folds an AT
/// event stream into the observed gateway-reputation subtree and presents it as a
/// substrate a polity can be chartered from.
///
/// A bare `AtProvider()` folds a **replayed** view — the events you hand it — which is
/// what offline founding and tests use, and needs no autonomous_trust linkage at all.
/// The live couplings (`attached`, `attached_existing`, `embedded`) exist only in an
/// `at-ffi` build; ask [`at_ffi_available`] rather than catching an exception.
///
///     p = AtProvider()
///     p.observe_peer(node_pubkey, reputation=0.83, operator_bound=True,
///                    guardian_pubkey=op_pubkey, operator_attested_at=now)
///     for change in p.changes():          # folds, then drains
///         ...
///     candidate = p.founding_candidate()
///
/// The provider only *reports* what AT observes and never confers standing: a candidate
/// is input to a proposal ("ratify emergence, do not replace it").
///
/// # Threading
///
/// In an `at-ffi` build this class is **thread-bound**: it may only be used from the
/// thread that created it, and PyO3 raises if another touches it. That is not a
/// conservative choice, it is the truth about the object — `AttachedAtDaemon` and
/// `EmbeddedAtNode` hold raw C handles and are neither `Send` nor `Sync`, so a provider
/// that could be handed to another thread would be lying about the daemon. The default
/// build has no such handles and no such restriction, which is why the attribute is
/// applied per-feature rather than always: a restriction appears exactly where the thing
/// causing it does.
#[cfg_attr(feature = "at-ffi", pyclass(name = "AtProvider", unsendable))]
#[cfg_attr(not(feature = "at-ffi"), pyclass(name = "AtProvider"))]
struct PyAtProvider {
    inner: AtBacking,
}

#[pymethods]
impl PyAtProvider {
    /// A provider over a replayed event feed, with no articulated substrate.
    #[new]
    #[pyo3(signature = (substrate_name=None, substrate_statement=None, substrate_anchor=None))]
    fn new(
        substrate_name: Option<String>,
        substrate_statement: Option<String>,
        substrate_anchor: Option<String>,
    ) -> PyResult<Self> {
        let substrate = substrate_from(substrate_name, substrate_statement, substrate_anchor)?;
        let src = MockEventSource::new();
        Ok(PyAtProvider {
            inner: AtBacking::Mock(match substrate {
                Some(s) => AtSubstrateProvider::with_substrate(src, s),
                None => AtSubstrateProvider::new(src),
            }),
        })
    }

    /// Queue an observation of a peer in the subtree, **as AT reports it**.
    ///
    /// Every argument is a wire field, and the reductions stay on this side of the seam:
    /// `guardian_pubkey` becomes a guardian DID and `operator_attested_at` an attendance
    /// stamp **only when `operator_bound` is true**, because a guardian key or an
    /// attendance stamp on a node whose operator credential never verified attests to
    /// nothing (**D8**). Passing them with `operator_bound=False` is not an error and not
    /// ignored input — it is exactly what AT sends for an unverified node, and the roll
    /// must read it as the unguarded machine it is.
    #[pyo3(signature = (signing_pubkey, reputation=None, rank=0, operator_bound=false,
                        guardian_pubkey=None, operator_attested_at=None))]
    fn observe_peer(
        &mut self,
        signing_pubkey: Vec<u8>,
        reputation: Option<f64>,
        rank: i32,
        operator_bound: bool,
        guardian_pubkey: Option<Vec<u8>>,
        operator_attested_at: Option<i64>,
    ) -> PyResult<()> {
        let peer = AtPeer {
            signing_pubkey,
            reputation,
            rank,
            operator_bound,
            guardian_pubkey,
            operator_attested_at: operator_attested_at.map(ts).transpose()?,
        };
        self.queue(AtEvent::PeerObserved(peer), "observe_peer")
    }

    /// Queue a peer's departure from the observed subtree.
    ///
    /// Only a replayed or synthesized view can express this: nothing in AT ever removes an
    /// entry from its peer table, so a live feed is upsert-only and grades absence by
    /// staleness instead (which is what `cohesion_report` already does).
    fn depart_peer(&mut self, signing_pubkey: Vec<u8>) -> PyResult<()> {
        self.queue(AtEvent::PeerDeparted { signing_pubkey }, "depart_peer")
    }

    /// Queue notice that part of the observed boundary is an intentional private edge.
    /// Informational: privacy is not incompleteness, and the members behind an opaque edge
    /// are simply absent from the roll rather than counted as unreachable.
    fn boundary_private(&mut self) -> PyResult<()> {
        self.queue(AtEvent::BoundaryPrivate, "boundary_private")
    }

    /// Poll the source and fold everything it returns into the view, returning how many
    /// events were applied. Call before reading unless you are using `changes()`, which
    /// folds as part of its contract.
    fn refresh(&mut self) -> PyResult<usize> {
        on_backing_mut!(self, |p| p.refresh()).map_err(en_err_to_py)
    }

    /// Fold whatever is queued, then drain the movements that fold produced.
    ///
    /// The two steps are one call deliberately: a caller that polled without folding, or
    /// folded without draining, would silently lose movements — and a lost movement is
    /// indistinguishable from a quiet substrate. Draining is not repeatable; the same
    /// change is never reported twice.
    fn changes(&mut self) -> PyResult<Vec<PySubstrateChange>> {
        let changes = on_backing_mut!(self, |p| p.changes()).map_err(en_err_to_py)?;
        Ok(changes.into_iter().map(PySubstrateChange::from_rs).collect())
    }

    /// The candidate roll as the substrate currently reads it, ordered by DID.
    fn founding_candidate(&self) -> PyResult<PyFoundingCandidate> {
        let inner = on_backing!(self, |p| p.founding_candidate()).map_err(en_err_to_py)?;
        Ok(PyFoundingCandidate { inner })
    }

    /// A member's earned reputation, or `None` for no reading — which is *unrated*, not
    /// *rated low* (**D18**), and also covers a peer this provider has never seen.
    fn member_reputation(&self, did: &str) -> PyResult<Option<f64>> {
        let d = parse_did(did)?;
        Ok(on_backing!(self, |p| p.member_reputation(&d)))
    }

    /// Whether an intentional private/opaque boundary edge has been observed.
    fn has_private_boundary(&self) -> bool {
        on_backing!(self, |p| p.has_private_boundary())
    }

    /// How many members are currently in the observed subtree view.
    fn observed_len(&self) -> usize {
        on_backing!(self, |p| p.observed_len())
    }

    fn __len__(&self) -> usize {
        self.observed_len()
    }

    fn __repr__(&self) -> String {
        let kind = match &self.inner {
            AtBacking::Mock(_) => "replayed",
            #[cfg(feature = "at-ffi")]
            AtBacking::Attached(_) => "attached",
            #[cfg(feature = "at-ffi")]
            AtBacking::Embedded(_) => "embedded",
        };
        format!("AtProvider({kind}, {} observed)", self.observed_len())
    }

    // --- the at-ffi half: present only in a build that linked autonomous_trust -----

    /// Attach to a running autonomous_trust daemon by opening its queues (**D6**).
    #[cfg(feature = "at-ffi")]
    #[staticmethod]
    #[pyo3(signature = (q_in, q_out=None, substrate_name=None, substrate_statement=None,
                        substrate_anchor=None))]
    fn attached(
        q_in: &str,
        q_out: Option<&str>,
        substrate_name: Option<String>,
        substrate_statement: Option<String>,
        substrate_anchor: Option<String>,
    ) -> PyResult<Self> {
        let src = AttachedAtDaemon::open(q_in, q_out).map_err(en_err_to_py)?;
        let substrate = substrate_from(substrate_name, substrate_statement, substrate_anchor)?;
        Ok(PyAtProvider {
            inner: AtBacking::Attached(match substrate {
                Some(s) => AtSubstrateProvider::with_substrate(src, s),
                None => AtSubstrateProvider::new(src),
            }),
        })
    }

    /// Attach using the queue this process has already been assigned (**D6**).
    #[cfg(feature = "at-ffi")]
    #[staticmethod]
    #[pyo3(signature = (q_out=None, substrate_name=None, substrate_statement=None,
                        substrate_anchor=None))]
    fn attached_existing(
        q_out: Option<&str>,
        substrate_name: Option<String>,
        substrate_statement: Option<String>,
        substrate_anchor: Option<String>,
    ) -> PyResult<Self> {
        let src = AttachedAtDaemon::open_existing(q_out).map_err(en_err_to_py)?;
        let substrate = substrate_from(substrate_name, substrate_statement, substrate_anchor)?;
        Ok(PyAtProvider {
            inner: AtBacking::Attached(match substrate {
                Some(s) => AtSubstrateProvider::with_substrate(src, s),
                None => AtSubstrateProvider::new(src),
            }),
        })
    }

    /// Start an embedded autonomous_trust node and fold its out-queue (**D6**).
    ///
    /// **Name a `log_file` under any harness that captures stderr through a pipe.** With
    /// none, `autonomous_trust.c` sets `NO_STDERR_REDIRECT` on the daemon *and* passes the
    /// flag to every subsystem process it forks, so all of them keep this process's stderr
    /// deliberately — and a harness reading that pipe to EOF never gets one while any
    /// child holds the write end. Measured during the Rust leg: a `timeout 60` run held
    /// the shell for ten minutes. `None` is the terminal-friendly default, and pytest is
    /// not a terminal.
    #[cfg(feature = "at-ffi")]
    #[staticmethod]
    #[pyo3(signature = (app_name, q_in, q_out, log_level=2, generate_config=false,
                        log_file=None, substrate_name=None, substrate_statement=None,
                        substrate_anchor=None))]
    #[allow(clippy::too_many_arguments)]
    fn embedded(
        app_name: &str,
        q_in: &str,
        q_out: &str,
        log_level: i32,
        generate_config: bool,
        log_file: Option<&str>,
        substrate_name: Option<String>,
        substrate_statement: Option<String>,
        substrate_anchor: Option<String>,
    ) -> PyResult<Self> {
        let level = at_log_level(log_level)?;
        let src = EmbeddedAtNode::start_logging_to(
            app_name,
            q_in,
            q_out,
            level,
            generate_config,
            log_file,
        )
        .map_err(en_err_to_py)?;
        let substrate = substrate_from(substrate_name, substrate_statement, substrate_anchor)?;
        Ok(PyAtProvider {
            inner: AtBacking::Embedded(match substrate {
                Some(s) => AtSubstrateProvider::with_substrate(src, s),
                None => AtSubstrateProvider::new(src),
            }),
        })
    }

    /// Ask the daemon for a full roster pull — the only emission that can carry an
    /// *unrated* peer, since every change-driven one is rated by construction.
    #[cfg(feature = "at-ffi")]
    fn request_roster(&mut self) -> PyResult<()> {
        match &mut self.inner {
            AtBacking::Attached(p) => p.source_mut().request_roster().map_err(en_err_to_py),
            AtBacking::Embedded(p) => p.source_mut().request_roster().map_err(en_err_to_py),
            AtBacking::Mock(_) => Err(not_daemon_backed("request_roster")),
        }
    }

    /// How many reputation readings the coupling is currently holding.
    #[cfg(feature = "at-ffi")]
    fn held_reputations(&self) -> PyResult<usize> {
        match &self.inner {
            AtBacking::Attached(p) => Ok(p.source().held_reputations()),
            AtBacking::Embedded(p) => Ok(p.source().held_reputations()),
            AtBacking::Mock(_) => Err(not_daemon_backed("held_reputations")),
        }
    }

    /// Whether the embedded node's process is still alive.
    #[cfg(feature = "at-ffi")]
    fn alive(&mut self) -> PyResult<bool> {
        match &mut self.inner {
            AtBacking::Embedded(p) => Ok(p.source_mut().alive()),
            _ => Err(not_daemon_backed("alive")),
        }
    }

    /// Whether the embedded node has finished coming up.
    #[cfg(feature = "at-ffi")]
    fn ready(&mut self) -> PyResult<bool> {
        match &mut self.inner {
            AtBacking::Embedded(p) => Ok(p.source_mut().ready()),
            _ => Err(not_daemon_backed("ready")),
        }
    }

    /// Block until the embedded node is ready, or `timeout_seconds` elapses.
    #[cfg(feature = "at-ffi")]
    fn wait_ready(&mut self, timeout_seconds: f64) -> PyResult<bool> {
        if !(timeout_seconds.is_finite() && timeout_seconds >= 0.0) {
            return Err(PyValueError::new_err(
                "timeout_seconds must be a non-negative, finite number",
            ));
        }
        match &mut self.inner {
            AtBacking::Embedded(p) => Ok(p
                .source_mut()
                .wait_ready(std::time::Duration::from_secs_f64(timeout_seconds))),
            _ => Err(not_daemon_backed("wait_ready")),
        }
    }

    /// The embedded node's process id.
    #[cfg(feature = "at-ffi")]
    fn pid(&self) -> PyResult<i32> {
        match &self.inner {
            AtBacking::Embedded(p) => Ok(p.source().pid()),
            _ => Err(not_daemon_backed("pid")),
        }
    }
}

impl PyAtProvider {
    /// Queue an event onto a replayed feed.
    ///
    /// Only the replayed backing accepts one, and the refusal is the honest answer rather
    /// than a limitation: a live coupling's events come *from* the daemon, so injecting an
    /// observation into it would fabricate a reading AT never made — which is the whole
    /// class of mistake this seam exists to prevent.
    fn queue(&mut self, event: AtEvent, method: &str) -> PyResult<()> {
        // Named in the refusal below, which only exists in an at-ffi build.
        let _ = method;
        match &mut self.inner {
            AtBacking::Mock(p) => {
                p.source_mut().push(event);
                Ok(())
            }
            #[cfg(feature = "at-ffi")]
            _ => {
                let _ = event;
                Err(PyRuntimeError::new_err(format!(
                    "{method}() needs a replayed-feed provider; this one folds a live \
                     autonomous_trust feed, whose observations come from the daemon"
                )))
            }
        }
    }
}

/// Why a daemon-only method refused: the provider is folding a replayed feed. A caller
/// error about a real method, as against a method that could only ever fail.
#[cfg(feature = "at-ffi")]
fn not_daemon_backed(method: &str) -> PyErr {
    PyRuntimeError::new_err(format!(
        "{method}() needs a daemon-backed provider; this one is folding a replayed feed \
         (build it with AtProvider.attached(), .attached_existing() or .embedded())"
    ))
}

/// AT's log level, by its wire number (1 Debug, 2 Info, 3 Warning, 4 Error, 5 Critical).
#[cfg(feature = "at-ffi")]
fn at_log_level(level: i32) -> PyResult<AtLogLevel> {
    match level {
        1 => Ok(AtLogLevel::Debug),
        2 => Ok(AtLogLevel::Info),
        3 => Ok(AtLogLevel::Warning),
        4 => Ok(AtLogLevel::Error),
        5 => Ok(AtLogLevel::Critical),
        other => Err(PyValueError::new_err(format!(
            "log_level must be 1..=5 (Debug..Critical), got {other}"
        ))),
    }
}

/// Whether this build linked autonomous_trust — i.e. whether the live coupling
/// constructors `AtProvider.attached`, `.attached_existing` and `.embedded` exist.
///
/// **Ask this rather than catching an exception.** In a default build those constructors
/// are absent, not present-and-failing; the replayed-feed provider is fully available
/// either way, so most callers never need to ask at all.
#[pyfunction]
fn at_ffi_available() -> bool {
    cfg!(feature = "at-ffi")
}

/// The `did:key` for a raw 32-byte Ed25519 **public** key.
///
/// The one mapping between an autonomous_trust identity and its ethne/kith-covenant DID:
/// a peer's AT signing key *is* its member identity, and `did:key` embeds the key, so no
/// registry is involved. Until D33 there was no route here at all — the only DID a Python
/// caller could obtain came from `generate_ed25519_keypair`, for a key it had just made —
/// so a host holding an observed peer's key had to reimplement multicodec-plus-multibase
/// itself and had no way to know if it had got it right.
#[pyfunction]
fn did_from_ed25519_pubkey(pubkey: Vec<u8>) -> PyResult<String> {
    Ok(signing_key_did(&pubkey).map_err(en_err_to_py)?.into_string())
}

// `gil_used = true` states what this module required before the pyo3 0.28 default
// flipped. Until 0.28 a module opted *in* to free-threaded support with
// `gil_used = false`; from 0.28 modules allow it by default and must opt out. The
// bump would therefore have acquired a free-threaded claim silently, and nothing
// here can test it: this sandbox's CPython 3.13.7 is a GIL build
// (`Py_GIL_DISABLED=0`), so a free-threaded interpreter never loads the module.
//
// The claim is not obviously safe either. The seam hands out `FileStore`/`ArchiveStore` handles, and
// under a free-threaded interpreter two Python threads reach one of those with no
// GIL serializing them. Declaring the status quo keeps the contract the tests
// actually exercise; lifting it is a separate change that needs a free-threaded
// interpreter to verify.
#[pymodule(gil_used = true)]
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
    m.add_class::<PyIndependentObservation>()?;
    m.add_class::<PyIndependentGrounding>()?;
    m.add_class::<PyIndependentCorroboration>()?;
    m.add_class::<PyCursor>()?;
    m.add_class::<PySaveOutcome>()?;
    m.add_class::<PyReconciled>()?;
    m.add_class::<PyFileStore>()?;
    m.add_class::<PyGuardianState>()?;
    m.add_class::<PyRiteState>()?;
    m.add_class::<PyEnvoyState>()?;
    m.add_class::<PyAffirmationState>()?;
    m.add_class::<PyFoundingCandidate>()?;
    m.add_class::<PyCohesionReport>()?;
    m.add_class::<PyHostConfig>()?;
    m.add_class::<PyDueAction>()?;
    m.add_class::<PyHostCycle>()?;
    m.add_class::<PyRiteHandoff>()?;
    m.add_class::<PyFissionOutcome>()?;
    m.add_class::<PyFusionOutcome>()?;
    m.add_class::<PyAtProvider>()?;
    m.add_class::<PySubstrateChange>()?;
    m.add_function(wrap_pyfunction!(generate_ed25519_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(cid_of_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(now_seconds, m)?)?;
    m.add_function(wrap_pyfunction!(propose_charter, m)?)?;
    m.add_function(wrap_pyfunction!(founding_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(manual_founding, m)?)?;
    m.add_function(wrap_pyfunction!(propose_from_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(select_from_candidate, m)?)?;
    m.add_function(wrap_pyfunction!(sign_reading, m)?)?;
    m.add_function(wrap_pyfunction!(sign_observation, m)?)?;
    m.add_function(wrap_pyfunction!(sign_detached, m)?)?;
    m.add_function(wrap_pyfunction!(cohesion_report, m)?)?;
    m.add_function(wrap_pyfunction!(coherence_concern, m)?)?;
    m.add_function(wrap_pyfunction!(did_from_ed25519_pubkey, m)?)?;
    m.add_function(wrap_pyfunction!(at_ffi_available, m)?)?;
    Ok(())
}
