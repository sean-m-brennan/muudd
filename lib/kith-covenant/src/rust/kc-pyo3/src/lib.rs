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

use std::collections::BTreeMap;

use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};

use jiff::ToSpan;
use kc_archive::{
    verify_all, Archive, ArchiveEntry, ExaminerCaps, InMemoryArchive, MediatorCaps, ObserverCaps,
    DEFAULT_EXAMINER_MAX_TERM_SECONDS, DEFAULT_EXAMINER_PANEL_MIN, DEFAULT_EXAMINER_POOL_FLOOR,
    DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS, DEFAULT_MEDIATOR_MAX_TERM_SECONDS,
    DEFAULT_MEDIATOR_POOL_FLOOR, DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
    DEFAULT_OBSERVER_MAX_TERM_SECONDS, DEFAULT_OBSERVER_POOL_FLOOR,
    DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS,
};
use kc_ceremony::inter::{
    AntiSpiralFeature, ExaminerEntry, ExaminerRegistry, InterCommunityWitnessRecord,
    InterCredentialingOutcome, InterCredentialingRunner, NonAdmissionRecord,
    ParticipantContext, RejectionDetail, SponsorshipRecord, REQUIRED_FEATURES,
};
use kc_conduct::{
    ActivityAttestation, AttestedFunction, ConductBearing, ConductKind, ConductRecord,
    DecisionClass, LadderRung, Severity,
};
use kc_ceremony::intra::{IntraCommunityRiteRecord, IntraCommunityRiteRunner};
use kc_ceremony::lineage::{InterpreterIndependenceAttestation, SubstrateInterpretation};
use kc_ceremony::recitation::{Recitation, RecitationVersion};
use kc_principal::{PrincipalOutput, PrincipalWitnessAttestation};
use kc_provenance::{
    EventActionInvoked, EventCorroboration, EventReport, ReviewAnswer,
    RetroactiveInterestFinding, SkepticismCriterion, SkepticismInvocation, SkepticismReview,
};
use kc_core::{
    keys, time, Alg, Blinding, BoundProof, Cid, Commitment, DidKey, KcError, PrivKey, PubKey,
    RevealedShare, RingSigned, Signed, SourcePreimage, ThresholdCustody, Timestamp,
};
use kc_identity::{
    CommunityIdentity, CompactCharter, DelegationScope, MemberRoll, RollMember, SpeakerDelegation,
    WitnessEntry, WitnessRegistry, WitnessRole,
};
use kc_mediation::{
    ArgumentRecord, ArgumentRole, ConsentAnswer, DecisionOutcome, DecisionUnsealed,
    DisputeRecord, MediationDecision, MediatorEntry, MediatorRegistry, PanelConsent,
    ProtectingPower, RecusalAnswer, RecusalDecision, RecusalRequest, SealedReasoning,
};
use kc_commons::{
    AppropriatorLevel, CommonsEscalation, CommonsKind, CommonsRegistration, CommonsSanction,
    SanctionLevel, SealedUsage, UsageFigures, UsageOpened, UsageRecord,
};
use kc_observer::{
    InsiderClaim, ObserverEntry, ObserverRegistry, ObserverReport, ReportResponse,
    SourceExculpation, SourceReveal, WhistleblowerReport,
};
use kc_constitution::{
    AmendmentKind, CompactIncapableAmendment, ConstitutionGenesis, FastCadenceAmendment,
    FastChange, FastPeriod, Forbearance, ForbearanceChange, IncapableMatter,
    MinimumContentAdoption, MinimumContentAmendment, MinimumSite,
};
use kc_escalation::{
    Capability, DeliberationCycle, Designation, DesignationTier, DesignationUrgency,
    InjunctionChallenge, InjunctionConfirmation, InterimInjunction, LadderTransition, ReEntry,
    ReEntryWitnessAttestation, ReexaminationConclusion, ReexaminationOutcome,
    RestitutionCompletion, RestitutionObligation, SubstantiveReEngagement,
};

/// One overdue re-examination as the Python seam carries it: `(designation_cid,
/// subject, tier, reexamine_after, seconds_overdue, confirmations)`.
///
/// Named because the row crosses the seam twice, once from the archive reader and once
/// as a field of the erosion alert, and the two have to agree element for element.
/// `confirmations` counts renewals, so a designation overdue on its third re-examination
/// is not the same reading as one nobody ever examined.
type OverdueReexaminationRow = (String, String, String, i64, i64, usize);

/// One silent standing function: `(community_did, function, seconds_since,
/// cadence_seconds, last_was_a_null)`. Same two readers as above.
type SilentFunctionRow = (String, String, i64, i64, bool);

/// The thinning-deliberation reading: `(decisions, first_argument_count,
/// latest_argument_count, first_span_seconds, latest_span_seconds,
/// argument_count_declined, span_shortened)`, absent when deliberation is not thinning.
type ThinningDeliberation = (usize, usize, usize, Option<i64>, Option<i64>, bool, bool);

/// Co-signers resolved for `Signed::build`: the DIDs to name in the record, and the
/// owned `(did_string, priv_key)` pairs a caller borrows to sign with.
type Cosigners = (Vec<DidKey>, Vec<(String, PrivKey)>);

/// One manufactured-event reading as the Python seam carries it: `(event_cid,
/// originator, reported_at, qualifies, triggers, provenance_complete, corroborators,
/// translated_corroborators, undisclosed_interests_found, aligned_faction)`, where
/// `undisclosed_interests_found` is a list of `(finder_did, interest)`.
///
/// Ten elements, four of them collections, three of those `Vec<String>` — which is
/// exactly the shape where a comment naming the order is the only thing keeping the
/// producer and the Python reader in agreement. Named so the order has somewhere to
/// live besides a doc comment above an anonymous tuple.
type SkepticismEventRow = (
    String,
    String,
    i64,
    bool,
    Vec<String>,
    bool,
    Vec<String>,
    Vec<String>,
    Vec<(String, String)>,
    Option<String>,
);

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
        PyBytes::new(py, &sk.bytes),
        PyBytes::new(py, &pk.bytes),
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
    /// The §8 observer-corps caps (pool-share, term, rotation) gate observer
    /// registries, and the §2 examiner caps do the same for the examining pool
    /// plus the panel that must examine an admission into a standing compact.
    /// A separate pass from `verify_all` (crypto); raises on the first violation.
    /// Assumes signatures already verify.
    #[pyo3(signature = (
        quorum_threshold=2,
        panel_min=3,
        observer_pool_floor=DEFAULT_OBSERVER_POOL_FLOOR,
        observer_max_term_seconds=DEFAULT_OBSERVER_MAX_TERM_SECONDS,
        observer_rotation_period_seconds=DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS,
        examiner_pool_floor=DEFAULT_EXAMINER_POOL_FLOOR,
        examiner_max_term_seconds=DEFAULT_EXAMINER_MAX_TERM_SECONDS,
        examiner_rotation_period_seconds=DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS,
        examiner_panel_min=DEFAULT_EXAMINER_PANEL_MIN,
        mediator_pool_floor=DEFAULT_MEDIATOR_POOL_FLOOR,
        mediator_max_term_seconds=DEFAULT_MEDIATOR_MAX_TERM_SECONDS,
        mediator_rotation_period_seconds=DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn check_authority(
        &self,
        quorum_threshold: usize,
        panel_min: usize,
        observer_pool_floor: usize,
        observer_max_term_seconds: i64,
        observer_rotation_period_seconds: i64,
        examiner_pool_floor: usize,
        examiner_max_term_seconds: i64,
        examiner_rotation_period_seconds: i64,
        examiner_panel_min: usize,
        mediator_pool_floor: usize,
        mediator_max_term_seconds: i64,
        mediator_rotation_period_seconds: i64,
    ) -> PyResult<()> {
        let caps = ObserverCaps {
            pool_floor: observer_pool_floor,
            max_term_seconds: observer_max_term_seconds,
            rotation_period_seconds: observer_rotation_period_seconds,
        };
        let ex_caps = ExaminerCaps {
            pool_floor: examiner_pool_floor,
            max_term_seconds: examiner_max_term_seconds,
            rotation_period_seconds: examiner_rotation_period_seconds,
            panel_min: examiner_panel_min,
        };
        let med_caps = MediatorCaps {
            pool_floor: mediator_pool_floor,
            max_term_seconds: mediator_max_term_seconds,
            rotation_period_seconds: mediator_rotation_period_seconds,
        };
        kc_archive::check_authority(
            &self.inner, quorum_threshold, panel_min, caps, ex_caps, med_caps,
        )
            .map_err(kc_err_to_py)
    }

    /// Every authority violation as `(cid, reason)`, instead of raising.
    #[pyo3(signature = (
        quorum_threshold=2,
        panel_min=3,
        observer_pool_floor=DEFAULT_OBSERVER_POOL_FLOOR,
        observer_max_term_seconds=DEFAULT_OBSERVER_MAX_TERM_SECONDS,
        observer_rotation_period_seconds=DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS,
        examiner_pool_floor=DEFAULT_EXAMINER_POOL_FLOOR,
        examiner_max_term_seconds=DEFAULT_EXAMINER_MAX_TERM_SECONDS,
        examiner_rotation_period_seconds=DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS,
        examiner_panel_min=DEFAULT_EXAMINER_PANEL_MIN,
        mediator_pool_floor=DEFAULT_MEDIATOR_POOL_FLOOR,
        mediator_max_term_seconds=DEFAULT_MEDIATOR_MAX_TERM_SECONDS,
        mediator_rotation_period_seconds=DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn authority_violations(
        &self,
        quorum_threshold: usize,
        panel_min: usize,
        observer_pool_floor: usize,
        observer_max_term_seconds: i64,
        observer_rotation_period_seconds: i64,
        examiner_pool_floor: usize,
        examiner_max_term_seconds: i64,
        examiner_rotation_period_seconds: i64,
        examiner_panel_min: usize,
        mediator_pool_floor: usize,
        mediator_max_term_seconds: i64,
        mediator_rotation_period_seconds: i64,
    ) -> Vec<(String, String)> {
        let caps = ObserverCaps {
            pool_floor: observer_pool_floor,
            max_term_seconds: observer_max_term_seconds,
            rotation_period_seconds: observer_rotation_period_seconds,
        };
        let ex_caps = ExaminerCaps {
            pool_floor: examiner_pool_floor,
            max_term_seconds: examiner_max_term_seconds,
            rotation_period_seconds: examiner_rotation_period_seconds,
            panel_min: examiner_panel_min,
        };
        let med_caps = MediatorCaps {
            pool_floor: mediator_pool_floor,
            max_term_seconds: mediator_max_term_seconds,
            rotation_period_seconds: mediator_rotation_period_seconds,
        };
        kc_archive::authority_violations(
            &self.inner, quorum_threshold, panel_min, caps, ex_caps, med_caps,
        )
    }

    /// §2: is `community_did` inside a probation window at `at` (a received
    /// sponsorship still riding on its conduct, or a completed re-entry before its
    /// window closes)? The predicate the §2 authority rules enforce, exposed so a
    /// caller can see *why* an act would be refused before authoring it.
    fn probationary_at(&self, community_did: &str, at: i64) -> PyResult<bool> {
        use std::str::FromStr as _;
        let did = DidKey::from_str(community_did).map_err(kc_err_to_py)?;
        Ok(kc_archive::probationary_at(&self.inner, &did, ts(at)?))
    }

    // ---- §4 Mediation authoring ------------------------------------------

    /// Publish a community's mediator roster (§4). `mediators` is a list of
    /// `(pubkey_bytes, did, encryption_pubkey_bytes)` the community nominates; they
    /// become active over `[valid_from, valid_until)`. Signed by the community root.
    /// Returns the CID.
    ///
    /// `valid_until` is **required** by the §4 caps: a seat with no end is a permanent
    /// mediator class, "the first step toward an inquisitorial caste". The pass also
    /// bounds the term's length, requires a sit-out before re-nomination, and refuses a
    /// roster that would give one community a majority of the active pool — the same
    /// four rules §8's observers and §2's examiners carry.
    ///
    /// `trained_by` optionally records each mediator's apprenticeship, by DID and in
    /// the same order as `mediators`. It **gates nothing** and is read only by
    /// `mediator_pool_concentration`.
    #[pyo3(signature = (community_priv, community_did, mediators, valid_from, valid_until=None, trained_by=None))]
    fn register_mediators(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        mediators: Vec<(Vec<u8>, String, Vec<u8>)>,
        valid_from: i64,
        valid_until: Option<i64>,
        trained_by: Option<Vec<Option<String>>>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let vf = ts(valid_from)?;
        let vu = valid_until.map(ts).transpose()?;
        if let Some(t) = &trained_by {
            if t.len() != mediators.len() {
                return Err(PyValueError::new_err(
                    "trained_by must be the same length as mediators",
                ));
            }
        }
        let mut entries = Vec::with_capacity(mediators.len());
        for (i, (pk, did, enc)) in mediators.iter().enumerate() {
            let trainer = match trained_by.as_ref().and_then(|t| t[i].as_deref()) {
                Some(d) => Some(parse_did(d)?),
                None => None,
            };
            entries.push(MediatorEntry {
                mediator_did: parse_did(did)?,
                mediator_pubkey: pub_from_bytes(pk),
                encryption_pubkey: pub_from_bytes(enc),
                valid_from: vf,
                valid_until: vu,
                trained_by: trainer,
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

    /// Publish a community's member roll (§8). `members` is a list of
    /// `(pubkey_bytes, did)`; each key must be the one its DID resolves to. Signed by
    /// the community root — naming who belongs is not delegable. Returns the CID.
    ///
    /// The roll widens the insider ring a whistleblower hides in, and it only ever
    /// widens it: rolls are unioned rather than superseded, so publishing a shorter
    /// roll later does not remove anyone. A departed member keeps the channel they
    /// had. `as_of` records the instant the roll was read upstream (a polity charter),
    /// and is provenance only — this tier cannot check it.
    #[pyo3(signature = (community_priv, community_did, members, as_of, published_at=None))]
    fn publish_member_roll(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        members: Vec<(Vec<u8>, String)>,
        as_of: i64,
        published_at: Option<i64>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let as_of = ts(as_of)?;
        let published_at = match published_at {
            Some(p) => ts(p)?,
            None => as_of,
        };
        let mut entries = Vec::with_capacity(members.len());
        for (pk, did) in &members {
            entries.push(RollMember {
                member_did: parse_did(did)?,
                member_pubkey: pub_from_bytes(pk),
            });
        }
        let roll = MemberRoll { community_did: cdid, members: entries, as_of, published_at };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(roll, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::MemberRollPublished(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Seat a community's examiners (§2). `examiners` is a list of
    /// `(pubkey_bytes, did)` the community seats to conduct anti-spiral-feature
    /// credentialing; they hold seats over `[valid_from, valid_until)`. Signed by the
    /// community root. Returns the CID.
    ///
    /// `valid_until=None` is accepted by the authoring call and refused by the
    /// authority pass: §2 allows no open-ended examining standing. A probationary
    /// community may not seat examiners at all.
    #[pyo3(signature = (community_priv, community_did, examiners, valid_from, valid_until=None))]
    fn register_examiners(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        examiners: Vec<(Vec<u8>, String)>,
        valid_from: i64,
        valid_until: Option<i64>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let vf = ts(valid_from)?;
        let vu = valid_until.map(ts).transpose()?;
        let mut entries = Vec::with_capacity(examiners.len());
        for (pk, did) in &examiners {
            entries.push(ExaminerEntry {
                examiner_did: parse_did(did)?,
                examiner_pubkey: pub_from_bytes(pk),
                valid_from: vf,
                valid_until: vu,
            });
        }
        let reg = ExaminerRegistry { nominating_community: cdid, entries, issued_at: vf };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(reg, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ExaminerRegistryUpdated(signed))
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

    /// Request that a named mediator step down from a dispute (§4), signed by the
    /// requesting party. Returns the CID, which the answer references.
    ///
    /// Nothing blocks on this: an unanswered request is visible rather than paralysing,
    /// because a request that halted a mediation by existing would be the delay tactic
    /// §4's own review guards against.
    #[pyo3(signature = (party_priv, party_did, dispute_cid, mediator_did, grounds, requested_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn request_recusal(
        &mut self,
        party_priv: &[u8],
        party_did: &str,
        dispute_cid: &str,
        mediator_did: &str,
        grounds: String,
        requested_at: Option<i64>,
    ) -> PyResult<String> {
        let req = RecusalRequest {
            dispute_cid: parse_cid(dispute_cid)?,
            requested_by: parse_did(party_did)?,
            mediator: parse_did(mediator_did)?,
            grounds,
            requested_at: parse_as_of(requested_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(party_priv);
        let signed = Signed::build(req, &[(party_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::RecusalRequested(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Answer a recusal request (§4). `answer` is `"Granted"` or `"Declined"`.
    ///
    /// Who may sign depends on the answer, and that asymmetry is the rule. A **grant**
    /// is signed by exactly the mediator stepping down — nobody else may condition it.
    /// A **decline** must come from registered non-party mediators *other than* the one
    /// named, and must state its reasoning: a panelist who could dismiss requests
    /// against themselves is the conflict the request exists to raise.
    #[pyo3(signature = (decider_privs, decider_dids, request_cid, answer, reasoning=String::new(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn decide_recusal(
        &mut self,
        decider_privs: Vec<Vec<u8>>,
        decider_dids: Vec<String>,
        request_cid: &str,
        answer: &str,
        reasoning: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        if decider_privs.len() != decider_dids.len() {
            return Err(PyValueError::new_err(
                "decider_privs and decider_dids must be the same length",
            ));
        }
        let deciders: Vec<DidKey> =
            decider_dids.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
        let dec = RecusalDecision {
            request_cid: parse_cid(request_cid)?,
            answer: parse_recusal_answer(answer)?,
            deciders,
            reasoning,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let sks: Vec<PrivKey> = decider_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            decider_dids.iter().map(|s| s.as_str()).zip(sks.iter()).collect();
        let signed = Signed::build(dec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::RecusalDecided(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Designate a neutral community as protecting power for one disputant (§4),
    /// co-signed by the disputant and the protector.
    ///
    /// The Geneva analogue: "it is not an arbitrator; it is a witness with standing".
    /// So it gates nothing — no decision waits on it. What the pass enforces is
    /// neutrality (the protector is neither party and seats no mediators) and that both
    /// sides consented, since standing for another community is a duty and is never
    /// conscripted.
    #[pyo3(signature = (disputant_priv, disputant_did, protector_priv, protector_did, dispute_cid, statement=String::new(), designated_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn designate_protecting_power(
        &mut self,
        disputant_priv: &[u8],
        disputant_did: &str,
        protector_priv: &[u8],
        protector_did: &str,
        dispute_cid: &str,
        statement: String,
        designated_at: Option<i64>,
    ) -> PyResult<String> {
        let pp = ProtectingPower {
            dispute_cid: parse_cid(dispute_cid)?,
            disputant: parse_did(disputant_did)?,
            protector: parse_did(protector_did)?,
            statement,
            designated_at: parse_as_of(designated_at)?.unwrap_or_else(time::now),
        };
        let dsk = priv_from_bytes(disputant_priv);
        let psk = priv_from_bytes(protector_priv);
        let signed = Signed::build(pp, &[(disputant_did, &dsk), (protector_did, &psk)])
            .map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ProtectingPowerDesignated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// File one argument on a dispute (§4), signed by its author. `role` is
    /// `"Party"` or `"Panelist"`.
    ///
    /// A party's own case may be signed by its envoy, exactly as the dispute filing
    /// may. A panelist's argument must be signed by that panelist alone and the author
    /// must be an active registered mediator from neither party community — nobody
    /// argues in a mediator's name. Returns the CID, which the decision cites.
    #[pyo3(signature = (author_priv, author_did, dispute_cid, role, position, evidence=Vec::new(), submitted_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn file_argument(
        &mut self,
        author_priv: &[u8],
        author_did: &str,
        dispute_cid: &str,
        role: &str,
        position: String,
        evidence: Vec<String>,
        submitted_at: Option<i64>,
    ) -> PyResult<String> {
        let arg = ArgumentRecord {
            dispute_cid: parse_cid(dispute_cid)?,
            author: parse_did(author_did)?,
            role: parse_argument_role(role)?,
            position,
            evidence: evidence.iter().map(|c| parse_cid(c)).collect::<PyResult<_>>()?,
            submitted_at: parse_as_of(submitted_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(author_priv);
        let signed = Signed::build(arg, &[(author_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ArgumentFiled(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Record a mediation decision (§4), co-signed by the panel. The archive's
    /// authority pass enforces >= panel_min registered, non-party mediators, a
    /// non-empty `reasoning` (§4's trail is mandatory), a `panel` equal to the
    /// co-signers, one cited `Panelist` argument per co-signer, and that every party
    /// argument already in the log is answered. `outcome` is a DecisionOutcome name.
    /// A `Settled` outcome additionally needs both parties' recorded consent to this
    /// exact panel (see `record_panel_consent`).
    ///
    /// To **seal** the proceedings instead of publishing them (§4), leave `reasoning`
    /// empty and pass the text as `seal_reasoning` with `seal_custodian_dids`: the
    /// reasoning is sealed under k-of-n custody keyed to those **member communities**,
    /// and opens at `seal_reveal_at` (default: one generational cycle from the
    /// decision). Custodians are communities rather than mediators because a mediator's
    /// seat is capped at two years and a seal runs for twenty-five: a share held by a key
    /// the archive stops resolving can never be opened. The authority pass refuses a
    /// party as custodian, and refuses a custodian set of which the deciding panel's own
    /// communities hold a quorum — a panel that can unseal itself has sealed nothing.
    /// Returns the CID.
    #[pyo3(signature = (dispute_cid, panel_privs, panel_dids, outcome, reasoning, arguments, decided_at=None, seal_reasoning=None, seal_custodian_dids=None, seal_threshold=None, seal_reveal_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn decide_mediation(
        &mut self,
        dispute_cid: &str,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        outcome: &str,
        reasoning: String,
        arguments: Vec<String>,
        decided_at: Option<i64>,
        seal_reasoning: Option<String>,
        seal_custodian_dids: Option<Vec<String>>,
        seal_threshold: Option<usize>,
        seal_reveal_at: Option<i64>,
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
        let decided = parse_as_of(decided_at)?.unwrap_or_else(time::now);
        // Sealing, if asked for. The caller supplies custodian DIDs and never their
        // keys: each share is sealed to the encryption key its holder is *registered*
        // under, resolved from this archive, exactly as §8 custody is.
        let sealed = match seal_custodian_dids {
            None => None,
            Some(custodians) => {
                if !reasoning.trim().is_empty() {
                    return Err(PyValueError::new_err(
                        "a sealed decision carries no cleartext reasoning: pass the text \
                         as `seal_reasoning` instead",
                    ));
                }
                let text = seal_reasoning.ok_or_else(|| {
                    PyValueError::new_err(
                        "sealing a decision needs the reasoning text as `seal_reasoning`",
                    )
                })?;
                let holders: Vec<DidKey> =
                    custodians.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
                let k = seal_threshold.unwrap_or(holders.len() / 2 + 1);
                // Custodians are member communities, whose keys still resolve when the
                // seal opens a generation later; a mediator's would not.
                let custody = ThresholdCustody::seal(
                    text.as_bytes(),
                    &holders,
                    kc_core::didkey_encryption_resolver,
                    k,
                )
                .map_err(kc_err_to_py)?;
                let reveal_at = match parse_as_of(seal_reveal_at)? {
                    Some(t) => t,
                    // One generational cycle from the decision, at the near end of the
                    // range §4 seals for.
                    None => ts(decided.as_second()
                        + kc_archive::MIN_DECISION_SEAL_SECONDS)?,
                };
                Some(SealedReasoning {
                    reasoning_commitment: SealedReasoning::commitment(&text)
                        .map_err(kc_err_to_py)?,
                    custody,
                    reveal_at,
                })
            }
        };
        let dec = MediationDecision {
            dispute_cid: parse_cid(dispute_cid)?,
            panel,
            outcome: parse_outcome(outcome)?,
            reasoning,
            sealed,
            arguments: arguments.iter().map(|c| parse_cid(c)).collect::<PyResult<_>>()?,
            decided_at: decided,
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

    /// Record a party's consent (or withholding) to the panel as constituted (§4).
    ///
    /// Self-signed: nobody consents on another's behalf. `answer` is a ConsentAnswer
    /// name (`Given` / `Withheld`); a withholding must state something. Consent gates a
    /// `Settled` outcome and nothing else — requiring it to decide at all would hand
    /// either party a veto over its own dispute. Returns the CID.
    #[pyo3(signature = (party_priv, party_did, dispute_cid, panel_dids, answer, statement=String::new(), given_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn record_panel_consent(
        &mut self,
        party_priv: &[u8],
        party_did: &str,
        dispute_cid: &str,
        panel_dids: Vec<String>,
        answer: &str,
        statement: String,
        given_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = PanelConsent {
            dispute_cid: parse_cid(dispute_cid)?,
            party: parse_did(party_did)?,
            panel: panel_dids.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?,
            answer: parse_consent_answer(answer)?,
            statement,
            given_at: parse_as_of(given_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(party_priv);
        let signed = Signed::build(rec, &[(party_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::PanelConsentRecorded(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// One custodian opens its own share of a sealed decision's proceedings (§4),
    /// returning it as JSON to be gathered into an `unseal_decision` call. A share is
    /// useless alone: it takes the seal's threshold of them to read anything.
    fn open_decision_share(
        &self,
        decision_cid: &str,
        holder_priv: Vec<u8>,
        holder_did: &str,
    ) -> PyResult<String> {
        let cid = parse_cid(decision_cid)?;
        let sealed = match self.inner.get(&cid) {
            Some(ArchiveEntry::MediationDecided(s)) => s.payload.sealed.as_ref(),
            _ => None,
        }
        .ok_or_else(|| {
            PyValueError::new_err(format!("{decision_cid} is not a sealed mediation decision"))
        })?;
        let share = sealed
            .custody
            .open_share(&parse_did(holder_did)?, &priv_from_bytes(&holder_priv))
            .map_err(kc_err_to_py)?;
        serde_json::to_string(&share).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Open a sealed decision's proceedings (§4), co-signed by the custodians whose
    /// `shares` are presented. The authority pass requires the shares to actually
    /// recover the sealed text and that text to open the decision's commitment, so an
    /// unsealing cannot publish a reasoning the panel never wrote. Opening before the
    /// seal's `reveal_at` must cite a cause. Returns the CID.
    #[pyo3(signature = (custodian_privs, custodian_dids, decision_cid, disclosed_reasoning, shares, cause=String::new(), unsealed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn unseal_decision(
        &mut self,
        custodian_privs: Vec<Vec<u8>>,
        custodian_dids: Vec<String>,
        decision_cid: &str,
        disclosed_reasoning: String,
        shares: Vec<String>,
        cause: String,
        unsealed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&custodian_privs, &custodian_dids)?;
        let shares: Vec<RevealedShare> = shares
            .iter()
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| PyValueError::new_err(format!("custody share: {e}")))
            })
            .collect::<PyResult<_>>()?;
        let rec = DecisionUnsealed {
            decision_cid: parse_cid(decision_cid)?,
            custodians: ids,
            cause,
            disclosed_reasoning,
            shares,
            unsealed_at: parse_as_of(unsealed_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> =
            pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::DecisionUnsealed(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// The mediator pool invokes the structured-skepticism protocol on an event (§4),
    /// co-signed by the invoking panel.
    ///
    /// This is what reaches an event whose originator declared it mundane: the hold no
    /// longer turns only on `report_event`'s self-declared triggers. `criteria` are
    /// SkepticismCriterion names, and the panel must document its grounds. The authority
    /// pass refuses a panel containing the originating community's own mediators, and
    /// bounds `hold_seconds` to the range the protocol calibrates to. Returns the CID.
    #[pyo3(signature = (panel_privs, panel_dids, event_cid, criteria, grounds, hold_seconds, invoked_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn invoke_skepticism(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        event_cid: &str,
        criteria: Vec<String>,
        grounds: String,
        hold_seconds: i64,
        invoked_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let rec = SkepticismInvocation {
            event_cid: parse_cid(event_cid)?,
            panel: ids,
            criteria: criteria
                .iter()
                .map(|c| parse_skepticism_criterion(c))
                .collect::<PyResult<_>>()?,
            grounds,
            hold_seconds,
            invoked_at: parse_as_of(invoked_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> =
            pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SkepticismInvoked(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// The pool's internal review of a hold, against pretextual use (§4), co-signed by
    /// the reviewing mediators.
    ///
    /// The review is against the **hold**, so it reaches a self-declared hold as well as
    /// an invoked one — the self-declared case being the one that used to answer to
    /// nobody. `answer` is a ReviewAnswer name; `Pretextual` releases the event.
    /// Mediators who co-signed an invocation on this event may not review it, and
    /// reasoning is required on either answer. Returns the CID.
    #[pyo3(signature = (panel_privs, panel_dids, event_cid, answer, reasoning, reviewed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn review_skepticism(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        event_cid: &str,
        answer: &str,
        reasoning: String,
        reviewed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let rec = SkepticismReview {
            event_cid: parse_cid(event_cid)?,
            reviewers: ids,
            answer: parse_review_answer(answer)?,
            reasoning,
            reviewed_at: parse_as_of(reviewed_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> =
            pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SkepticismReviewed(signed))
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

    // ---- §9 Interim injunction authoring (capability-gating) -------------

    /// Issue an interim injunction against a community (§9), co-signed by the
    /// emergency mediator panel. `revoked_capabilities` are Capability names
    /// suspended while the injunction is in force. `expires_at` defaults to
    /// `issued_at + 90 days`. The authority pass enforces a non-party mediator
    /// panel. Returns the injunction's CID (referenced by confirmations and
    /// challenges).
    #[pyo3(signature = (panel_privs, panel_dids, subject_did, harm, protective_measures, revoked_capabilities, dispute_cid=None, issued_at=None, expires_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn issue_injunction(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        subject_did: &str,
        harm: String,
        protective_measures: String,
        revoked_capabilities: Vec<String>,
        dispute_cid: Option<String>,
        issued_at: Option<i64>,
        expires_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let revoked: Vec<Capability> =
            revoked_capabilities.iter().map(|c| parse_capability(c)).collect::<PyResult<_>>()?;
        let issued = parse_as_of(issued_at)?.unwrap_or_else(time::now);
        let expires = match parse_as_of(expires_at)? {
            Some(t) => t,
            None => ts(issued.as_second() + 90 * 24 * 3600)?,
        };
        let inj = InterimInjunction {
            subject_community: parse_did(subject_did)?,
            dispute_cid: dispute_cid.as_deref().map(parse_cid).transpose()?,
            harm,
            protective_measures,
            revoked_capabilities: revoked,
            panel: ids,
            issued_at: issued,
            expires_at: expires,
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(inj, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InjunctionIssued(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Confirm an injunction (§9): the full compact ratifying it, co-signed by a
    /// distinct-community quorum. A confirmation that arrives before the
    /// injunction's expiry lets it persist past its window. Returns the CID.
    #[pyo3(signature = (confirmer_privs, confirmer_dids, injunction_cid, confirmed_at=None))]
    fn confirm_injunction(
        &mut self,
        confirmer_privs: Vec<Vec<u8>>,
        confirmer_dids: Vec<String>,
        injunction_cid: &str,
        confirmed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&confirmer_privs, &confirmer_dids)?;
        let c = InjunctionConfirmation {
            injunction_cid: parse_cid(injunction_cid)?,
            confirmers: ids,
            confirmed_at: parse_as_of(confirmed_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(c, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InjunctionConfirmed(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Challenge an injunction (§9), heard by a *different* mediator panel than
    /// the one that issued it (the authority pass enforces disjointness and the
    /// non-party rule). `overturned=True` ends the injunction. Returns the CID.
    #[pyo3(signature = (panel_privs, panel_dids, injunction_cid, overturned, reasoning=String::new(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn challenge_injunction(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        injunction_cid: &str,
        overturned: bool,
        reasoning: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let ch = InjunctionChallenge {
            injunction_cid: parse_cid(injunction_cid)?,
            panel: ids,
            overturned,
            reasoning,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(ch, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InjunctionChallenged(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// §9 capability gate: `True` unless `capability` is currently revoked from
    /// `did` by an active or confirmed injunction, as of `as_of` (default now).
    #[pyo3(signature = (did, capability, as_of=None))]
    fn has_capability(&self, did: &str, capability: &str, as_of: Option<i64>) -> PyResult<bool> {
        let did_key = parse_did(did)?;
        // Validate the name so a typo fails loudly rather than silently passing.
        let _ = parse_capability(capability)?;
        let as_of_ts = parse_as_of(as_of)?;
        Ok(kc_reputation::has_capability(&self.inner, &did_key, capability, as_of_ts))
    }

    // ---- §7 Commons governance authoring ---------------------------------

    /// Register a compact-level commons (§7), co-signed by the registering
    /// communities. `kind` is a CommonsKind name; `appropriators` are the member
    /// DIDs with draw rights (Ostrom's boundary). A second-class act: the
    /// authority pass requires a distinct-community quorum, and every registrant
    /// must itself be an appropriator. Returns the commons' CID (referenced by
    /// usage and sanction records).
    ///
    /// `draw_allowance` is `provision_rule`'s machine-readable half: the per-period cap
    /// on one appropriator's draw, in the commons' own units. Without it the stated rule
    /// binds nothing and the commons cannot host sealed usage, since there would be no
    /// bound to prove against.
    ///
    /// `level` is an AppropriatorLevel name. At `"Compact"` the appropriators are whole
    /// compacts rather than communities, and subsidiarity applies: sanctions and
    /// escalations on the commons name a compact, never one of its member communities.
    #[pyo3(signature = (registrar_privs, registrar_dids, kind, name, description, appropriators, provision_rule=String::new(), registered_at=None, draw_allowance=None, level="Community"))]
    #[allow(clippy::too_many_arguments)]
    fn register_commons(
        &mut self,
        registrar_privs: Vec<Vec<u8>>,
        registrar_dids: Vec<String>,
        kind: &str,
        name: String,
        description: String,
        appropriators: Vec<String>,
        provision_rule: String,
        registered_at: Option<i64>,
        draw_allowance: Option<u64>,
        level: &str,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&registrar_privs, &registrar_dids)?;
        let appropriator_dids: Vec<DidKey> =
            appropriators.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
        let reg = CommonsRegistration {
            kind: parse_commons_kind(kind)?,
            name,
            description,
            appropriators: appropriator_dids,
            level: parse_appropriator_level(level)?,
            provision_rule,
            draw_allowance,
            registered_by: ids,
            registered_at: parse_as_of(registered_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(reg, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::CommonsRegistered(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Record a community's self-reported usage of a commons (§7), signed by that
    /// community. First-class; the authority pass requires the reporter be an
    /// appropriator of the referenced commons. Returns the CID.
    ///
    /// **To account under the surveillance threshold** rather than in public, pass
    /// `seal_custodian_dids`: the figures are committed instead of published, a proof
    /// that the draw is within the commons' `draw_allowance` goes on the record, and the
    /// figures themselves are sealed under k-of-n custody keyed to those communities.
    /// The custodians must be fellow appropriators and may not include the reporter, so
    /// a community cannot hold a quorum over its own account. A quorum opens them later
    /// with `open_usage`.
    #[pyo3(signature = (community_priv, community_did, commons_cid, consumed, contributed, period_start=None, period_end=None, recorded_at=None, seal_custodian_dids=None, seal_threshold=None))]
    #[allow(clippy::too_many_arguments)]
    fn record_usage(
        &mut self,
        community_priv: Vec<u8>,
        community_did: &str,
        commons_cid: &str,
        consumed: u64,
        contributed: u64,
        period_start: Option<i64>,
        period_end: Option<i64>,
        recorded_at: Option<i64>,
        seal_custodian_dids: Option<Vec<String>>,
        seal_threshold: Option<usize>,
    ) -> PyResult<String> {
        let at = parse_as_of(recorded_at)?.unwrap_or_else(time::now);
        let cid = parse_cid(commons_cid)?;
        // Sealing, if asked for. The figures are committed, the draw is proven within
        // the commons' allowance, and the figures themselves go under custody keyed to
        // fellow appropriators, so the reporter cannot open its own account.
        let sealed = match seal_custodian_dids {
            None => None,
            Some(custodians) => {
                let allowance = match self.inner.get(&cid) {
                    Some(ArchiveEntry::CommonsRegistered(s)) => s.payload.draw_allowance,
                    _ => None,
                }
                .ok_or_else(|| {
                    PyValueError::new_err(
                        "sealing usage needs the commons to state a `draw_allowance`: \
                         there would otherwise be no bound to prove against",
                    )
                })?;
                let consumed_blinding = Blinding::random();
                let contributed_blinding = Blinding::random();
                let consumed_commitment = Commitment::new(consumed, &consumed_blinding);
                let contributed_commitment = Commitment::new(contributed, &contributed_blinding);
                let within_allowance = BoundProof::prove(
                    consumed,
                    &consumed_blinding,
                    &consumed_commitment,
                    allowance,
                )
                .map_err(kc_err_to_py)?;
                let figures = UsageFigures {
                    consumed,
                    contributed,
                    consumed_blinding: consumed_blinding.to_bytes(),
                    contributed_blinding: contributed_blinding.to_bytes(),
                };
                let holders: Vec<DidKey> =
                    custodians.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
                let k = seal_threshold.unwrap_or(holders.len() / 2 + 1);
                let custody = ThresholdCustody::seal(
                    &SealedUsage::figure_bytes(&figures).map_err(kc_err_to_py)?,
                    &holders,
                    kc_core::didkey_encryption_resolver,
                    k,
                )
                .map_err(kc_err_to_py)?;
                Some(SealedUsage {
                    consumed_commitment,
                    contributed_commitment,
                    within_allowance,
                    custody,
                })
            }
        };
        let public = sealed.is_none();
        let u = UsageRecord {
            commons_cid: cid,
            community: parse_did(community_did)?,
            consumed: if public { consumed } else { 0 },
            contributed: if public { contributed } else { 0 },
            sealed,
            period_start: parse_as_of(period_start)?.unwrap_or(at),
            period_end: parse_as_of(period_end)?.unwrap_or(at),
            recorded_at: at,
        };
        let key = priv_from_bytes(&community_priv);
        let signed = Signed::build(u, &[(community_did, &key)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::UsageAccounted(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Charter a compact (§7, Ostrom principle 8): give it a DID of its own, co-signed
    /// by the compact's key and a quorum of its member communities.
    ///
    /// A community's identity is self-signed because a community exists by its own act;
    /// a compact does not, so its charter is constituted by its members. The compact's
    /// own signature is required too, or the DID would bind to a key nobody has shown
    /// they hold. Returns the CID.
    #[pyo3(signature = (compact_priv, compact_pub, label, members, charterer_privs, charterer_dids, chartered_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn charter_compact(
        &mut self,
        compact_priv: Vec<u8>,
        compact_pub: Vec<u8>,
        label: String,
        members: Vec<String>,
        charterer_privs: Vec<Vec<u8>>,
        charterer_dids: Vec<String>,
        chartered_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&charterer_privs, &charterer_dids)?;
        let member_dids: Vec<DidKey> =
            members.iter().map(|d| parse_did(d)).collect::<PyResult<_>>()?;
        let chartered: Vec<(DidKey, &PrivKey)> = ids
            .iter()
            .cloned()
            .zip(pairs_owned.iter().map(|(_, k)| k))
            .collect();
        let signed = CompactCharter::new_signed(
            label,
            &priv_from_bytes(&compact_priv),
            pub_from_bytes(&compact_pub),
            member_dids,
            &chartered,
            parse_as_of(chartered_at)?.unwrap_or_else(time::now),
        )
        .map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::CompactChartered(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Find that a community's abuse of a commons has passed beyond graduated sanctions
    /// (§7), co-signed by the finding communities. Second-class.
    ///
    /// This is the commons equivalent of the anathema designation, which
    /// `commons_federation.md` left open. The authority pass enforces the four
    /// conditions: the subject is at `Exclusion`, has arrived there more than once
    /// (defiance rather than compliance, and once suffices on a life-critical commons),
    /// has stood there long enough that restoration was available and not taken, and the
    /// finding cites real sanctions on this pair. It gates nothing by itself: it is what
    /// a §5 designation may cite as its commons-side basis. Returns the CID.
    #[pyo3(signature = (mover_privs, mover_dids, commons_cid, subject_did, basis, cited_sanctions, escalated_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn escalate_commons(
        &mut self,
        mover_privs: Vec<Vec<u8>>,
        mover_dids: Vec<String>,
        commons_cid: &str,
        subject_did: &str,
        basis: String,
        cited_sanctions: Vec<String>,
        escalated_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&mover_privs, &mover_dids)?;
        let rec = CommonsEscalation {
            commons_cid: parse_cid(commons_cid)?,
            subject_community: parse_did(subject_did)?,
            basis,
            cited_sanctions: cited_sanctions
                .iter()
                .map(|c| parse_cid(c))
                .collect::<PyResult<_>>()?,
            movers: ids,
            decision_class: DecisionClass::SecondClass,
            escalated_at: parse_as_of(escalated_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> =
            pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::CommonsEscalated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// One custodian opens its own share of a sealed usage record (§7), returning it as
    /// JSON to be gathered into an `open_usage` call. A share is useless alone.
    fn open_usage_share(
        &self,
        usage_cid: &str,
        holder_priv: Vec<u8>,
        holder_did: &str,
    ) -> PyResult<String> {
        let cid = parse_cid(usage_cid)?;
        let sealed = match self.inner.get(&cid) {
            Some(ArchiveEntry::UsageAccounted(s)) => s.payload.sealed.as_ref(),
            _ => None,
        }
        .ok_or_else(|| PyValueError::new_err(format!("{usage_cid} is not a sealed usage record")))?;
        let share = sealed
            .custody
            .open_share(&parse_did(holder_did)?, &priv_from_bytes(&holder_priv))
            .map_err(kc_err_to_py)?;
        serde_json::to_string(&share).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Open a sealed usage record's figures (§7), co-signed by the custodians whose
    /// `shares` are presented and citing a cause.
    ///
    /// The authority pass requires the shares to recover figures that open the record's
    /// own commitments, so a custodian quorum cannot publish a number the community
    /// never committed to. Returns the CID.
    #[pyo3(signature = (custodian_privs, custodian_dids, usage_cid, consumed, contributed, shares, cause, opened_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn open_usage(
        &mut self,
        custodian_privs: Vec<Vec<u8>>,
        custodian_dids: Vec<String>,
        usage_cid: &str,
        consumed: u64,
        contributed: u64,
        shares: Vec<String>,
        cause: String,
        opened_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&custodian_privs, &custodian_dids)?;
        let shares: Vec<RevealedShare> = shares
            .iter()
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| PyValueError::new_err(format!("custody share: {e}")))
            })
            .collect::<PyResult<_>>()?;
        let rec = UsageOpened {
            usage_cid: parse_cid(usage_cid)?,
            custodians: ids,
            cause,
            consumed,
            contributed,
            shares,
            opened_at: parse_as_of(opened_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> =
            pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::UsageOpened(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Apply a graduated-sanction step against a community on a commons (§7),
    /// co-signed by the imposing communities. `from_level`/`to_level` are
    /// SanctionLevel names; the authority pass requires the move to leave the
    /// pair's current level and either escalate one step or de-escalate, and the
    /// second-class levels (Restriction, Exclusion) to carry a distinct-community
    /// quorum. Returns the CID.
    #[pyo3(signature = (imposer_privs, imposer_dids, commons_cid, subject_did, from_level, to_level, basis=String::new(), references=Vec::new(), decision_class="FirstClass".to_string(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn apply_sanction(
        &mut self,
        imposer_privs: Vec<Vec<u8>>,
        imposer_dids: Vec<String>,
        commons_cid: &str,
        subject_did: &str,
        from_level: &str,
        to_level: &str,
        basis: String,
        references: Vec<String>,
        decision_class: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&imposer_privs, &imposer_dids)?;
        let refs: Vec<Cid> = references.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let s = CommonsSanction {
            commons_cid: parse_cid(commons_cid)?,
            subject_community: parse_did(subject_did)?,
            from_level: parse_sanction_level(from_level)?,
            to_level: parse_sanction_level(to_level)?,
            basis,
            references: refs,
            imposers: ids,
            decision_class: parse_decision_class(&decision_class)?,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(s, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SanctionApplied(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// §7 commons gate: the current SanctionLevel name standing against `did` on
    /// the commons at `commons_cid`, as of `as_of` (default now). `None` when no
    /// sanction stands.
    #[pyo3(signature = (did, commons_cid, as_of=None))]
    fn commons_level(&self, did: &str, commons_cid: &str, as_of: Option<i64>) -> PyResult<String> {
        let did_key = parse_did(did)?;
        let cid = parse_cid(commons_cid)?;
        let as_of_ts = parse_as_of(as_of)?;
        let level = kc_reputation::commons_level(&self.inner, &did_key, &cid, as_of_ts);
        Ok(level.as_str().to_string())
    }

    /// §7 free-rider lens over a commons's usage records as of `as_of`:
    /// per-appropriator consumed/contributed tallies plus the net-draw ranking.
    #[pyo3(signature = (commons_cid, as_of=None))]
    fn free_rider_signals(&self, commons_cid: &str, as_of: Option<i64>) -> PyResult<PyFreeRiderReport> {
        let cid = parse_cid(commons_cid)?;
        let as_of_ts = parse_as_of(as_of)?;
        let report = kc_reputation::free_rider_signals(&self.inner, &cid, as_of_ts);
        Ok(PyFreeRiderReport::from(&report))
    }

    // ---- §8 Observer / jester corps authoring ----------------------------

    /// Publish `community`'s roster of nominated observers (§8), signed by the
    /// community root. Each observer is `(pubkey, did, declared_perspective)`;
    /// the compact-wide corps is the union across communities (latest registry
    /// per community supersedes). Returns the CID.
    #[pyo3(signature = (community_priv, community_did, observers, valid_from, valid_until=None))]
    fn register_observers(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        observers: Vec<(Vec<u8>, String, String)>,
        valid_from: i64,
        valid_until: Option<i64>,
    ) -> PyResult<String> {
        let cdid = parse_did(community_did)?;
        let vf = ts(valid_from)?;
        let vu = valid_until.map(ts).transpose()?;
        let mut entries = Vec::with_capacity(observers.len());
        for (pk, did, perspective) in &observers {
            entries.push(ObserverEntry {
                observer_did: parse_did(did)?,
                observer_pubkey: pub_from_bytes(pk),
                declared_perspective: perspective.clone(),
                valid_from: vf,
                valid_until: vu,
            });
        }
        let reg = ObserverRegistry { nominating_community: cdid, entries, issued_at: vf };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(reg, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ObserverRegistryUpdated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Publish an attributed observer report on `subject_did` (§8), signed by the
    /// observer. The authority pass requires the observer be a registered,
    /// non-party member of the corps. Returns the report CID.
    #[pyo3(signature = (observer_priv, observer_did, subject_did, declared_perspective, summary, references=Vec::new(), observed_at=None, reported_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn publish_observer_report(
        &mut self,
        observer_priv: &[u8],
        observer_did: &str,
        subject_did: &str,
        declared_perspective: String,
        summary: String,
        references: Vec<String>,
        observed_at: Option<i64>,
        reported_at: Option<i64>,
    ) -> PyResult<String> {
        let at = parse_as_of(reported_at)?.unwrap_or_else(time::now);
        let refs: Vec<Cid> = references.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let rep = ObserverReport {
            subject_community: parse_did(subject_did)?,
            observer_did: parse_did(observer_did)?,
            declared_perspective,
            summary,
            references: refs,
            observed_at: parse_as_of(observed_at)?.unwrap_or(at),
            reported_at: at,
        };
        let sk = priv_from_bytes(observer_priv);
        let signed = Signed::build(rep, &[(observer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ObserverReportPublished(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a subject community's public response to an observer report (§8),
    /// signed by that community. The response never suppresses the report.
    #[pyo3(signature = (community_priv, community_did, report_cid, response, responded_at=None))]
    fn respond_to_report(
        &mut self,
        community_priv: &[u8],
        community_did: &str,
        report_cid: &str,
        response: String,
        responded_at: Option<i64>,
    ) -> PyResult<String> {
        let r = ReportResponse {
            report_cid: parse_cid(report_cid)?,
            responding_community: parse_did(community_did)?,
            response,
            responded_at: parse_as_of(responded_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(community_priv);
        let signed = Signed::build(r, &[(community_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ReportResponsePublished(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// The keys the archive resolves as insiders of `community_did` at `at`: its root
    /// key plus every live speaker delegation. This is the ring a whistleblower proves
    /// membership in; returned as `(did, pubkey_bytes)` so a caller can find its own
    /// position. Exposed because the source assembles its attestation against exactly
    /// the set the authority pass will check.
    #[pyo3(signature = (community_did, at=None))]
    fn insider_ring(&self, community_did: &str, at: Option<i64>) -> PyResult<Vec<(String, Vec<u8>)>> {
        let when = parse_as_of(at)?.unwrap_or_else(time::now);
        Ok(kc_archive::insider_keys(&self.inner, &parse_did(community_did)?, when)
            .into_iter()
            .map(|(did, pk)| (did.into_string(), pk.bytes))
            .collect())
    }

    /// **The source's step.** Ring-sign an insider attestation for a filing against
    /// `accused_did`, and commit to the source's identity.
    ///
    /// Returns the out-of-band packet the source hands the reception panel: the
    /// anonymous attestation plus the commitment preimage. The panel cannot forge the
    /// attestation, and the source never hands over its key. Fails if the source's key
    /// is not in the accused community's insider set, or if that set is too small to
    /// hide anyone.
    #[pyo3(signature = (source_priv, accused_did, claim, filed_at=None))]
    fn attest_insider(
        &self,
        source_priv: Vec<u8>,
        accused_did: &str,
        claim: &str,
        filed_at: Option<i64>,
    ) -> PyResult<String> {
        let at = parse_as_of(filed_at)?.unwrap_or_else(time::now);
        let accused = parse_did(accused_did)?;
        let sk = priv_from_bytes(&source_priv);
        let ring_pairs = kc_archive::insider_keys(&self.inner, &accused, at);
        if ring_pairs.len() < 2 {
            return Err(PyValueError::new_err(format!(
                "{accused_did} has {} resolvable key(s): a ring of one names its signer, so it has no anonymous whistleblower channel. Delegate a speaker first.",
                ring_pairs.len()
            )));
        }
        let ring: Vec<PubKey> = ring_pairs.iter().map(|(_, pk)| pk.clone()).collect();
        // Find the source's own position from its private key alone, so the caller
        // never has to state which ring member it is.
        let own_pub = keys::public_of(&sk).map_err(kc_err_to_py)?;
        let (index, source_did) = ring_pairs
            .iter()
            .enumerate()
            .find(|(_, (_, pk))| pk.bytes == own_pub.bytes)
            .map(|(i, (did, _))| (i, did.clone()))
            .ok_or_else(|| {
                PyValueError::new_err(format!(
                    "this key is not an insider of {accused_did}: only a community's root key or a live speaker delegation can file"
                ))
            })?;
        let preimage = SourcePreimage::new(source_did.as_str());
        let commitment = preimage.commitment().map_err(kc_err_to_py)?;
        let insider =
            WhistleblowerReport::attest(accused, commitment, claim, at, ring, index, &sk)
                .map_err(kc_err_to_py)?;
        let packet = serde_json::json!({
            "insider": insider,
            "preimage": preimage,
            "filed_at": at.as_second(),
            "claim": claim,
        });
        serde_json::to_string(&packet).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// **The panel's step.** File a whistleblower report against `accused_did` (§8),
    /// co-signed by a mediator reception panel, from the source's `attestation` packet.
    ///
    /// The panel seals the source identity under k-of-n custody keyed to its own
    /// members (`threshold` defaults to a majority of the panel), so no minority of the
    /// panel can name the source later. The authority pass requires a non-party
    /// reception panel, an attestation from a real insider bound to this exact claim,
    /// and custody held by exactly this panel. Returns the report CID.
    #[pyo3(signature = (panel_privs, panel_dids, accused_did, claim, attestation, threshold=None, references=Vec::new()))]
    #[allow(clippy::too_many_arguments)]
    fn file_whistleblower(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        accused_did: &str,
        claim: String,
        attestation: &str,
        threshold: Option<usize>,
        references: Vec<String>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let refs: Vec<Cid> = references.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?;
        let packet: serde_json::Value = serde_json::from_str(attestation)
            .map_err(|e| PyValueError::new_err(format!("attestation packet: {e}")))?;
        let insider: RingSigned<InsiderClaim> =
            serde_json::from_value(packet["insider"].clone())
                .map_err(|e| PyValueError::new_err(format!("attestation: {e}")))?;
        let preimage: SourcePreimage = serde_json::from_value(packet["preimage"].clone())
            .map_err(|e| PyValueError::new_err(format!("attestation preimage: {e}")))?;
        let filed_at = insider.payload.filed_at;

        // The caller supplies no keys. Each share is sealed to the encryption key its
        // holder is *registered* under, resolved from this archive — so a filing cannot
        // name the panel and seal the shares elsewhere, which used to be authorized and
        // left no custodian able to open anything.
        let holders: Vec<DidKey> = ids.to_vec();
        // A majority by default: the smallest quorum that cannot be met by a minority.
        let k = threshold.unwrap_or(holders.len() / 2 + 1);
        let bytes = kc_core::canonical::to_canonical_cbor(&preimage).map_err(kc_err_to_py)?;
        let custody = ThresholdCustody::seal(
            &bytes,
            &holders,
            kc_archive::mediator_encryption_resolver(&self.inner, filed_at),
            k,
        )
        .map_err(kc_err_to_py)?;

        let wb = WhistleblowerReport {
            accused_community: parse_did(accused_did)?,
            source_commitment: preimage.commitment().map_err(kc_err_to_py)?,
            claim,
            references: refs,
            reception_panel: ids,
            filed_at,
            insider,
            custody,
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(wb, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::WhistleblowerFiled(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// One panelist opens its own custody share of a filed report's source (§8),
    /// returning it as JSON to be gathered into a [`reveal_source`] call. A share is
    /// useless alone: it takes the filing's threshold of them to name anybody.
    fn open_source_share(
        &self,
        report_cid: &str,
        holder_priv: Vec<u8>,
        holder_did: &str,
    ) -> PyResult<String> {
        let cid = parse_cid(report_cid)?;
        let wb = match self.inner.get(&cid) {
            Some(ArchiveEntry::WhistleblowerFiled(s)) => &s.payload,
            _ => {
                return Err(PyValueError::new_err(format!(
                    "{report_cid} is not a whistleblower report"
                )))
            }
        };
        let share = wb
            .custody
            .open_share(&parse_did(holder_did)?, &priv_from_bytes(&holder_priv))
            .map_err(kc_err_to_py)?;
        serde_json::to_string(&share).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Disclose a whistleblower's source (§8), co-signed by a mediator panel, citing a
    /// non-empty cause, and carrying enough opened custody `shares` to actually
    /// recover the identity. The authority pass checks that they do: a reveal cannot
    /// name someone the shares do not produce. Returns the CID.
    #[pyo3(signature = (panel_privs, panel_dids, report_cid, cause, disclosed_source, shares, revealed_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn reveal_source(
        &mut self,
        panel_privs: Vec<Vec<u8>>,
        panel_dids: Vec<String>,
        report_cid: &str,
        cause: String,
        disclosed_source: String,
        shares: Vec<String>,
        revealed_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&panel_privs, &panel_dids)?;
        let shares: Vec<RevealedShare> = shares
            .iter()
            .map(|s| {
                serde_json::from_str(s)
                    .map_err(|e| PyValueError::new_err(format!("custody share: {e}")))
            })
            .collect::<PyResult<_>>()?;
        let rev = SourceReveal {
            report_cid: parse_cid(report_cid)?,
            panel: ids,
            cause,
            disclosed_source,
            shares,
            revealed_at: parse_as_of(revealed_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(rev, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SourceRevealed(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Publish a proof that a whistleblower filing's key image is **not** the
    /// claimant's (§8) — the answer available to a member a reveal named wrongly.
    ///
    /// Needs nothing but the claimant's own key: no panel, no quorum, no permission.
    /// The archive refuses it if the proof is bad, if the claimant was never in the
    /// filing's ring, or if the claimed image *matches* the filing (which would make
    /// the record a confession rather than a denial).
    #[pyo3(signature = (claimant_priv, claimant_did, report_cid, exculpated_at=None))]
    fn exculpate_source(
        &mut self,
        claimant_priv: Vec<u8>,
        claimant_did: &str,
        report_cid: &str,
        exculpated_at: Option<i64>,
    ) -> PyResult<String> {
        let sk = priv_from_bytes(&claimant_priv);
        let did = parse_did(claimant_did)?;
        let exc = SourceExculpation::prove(
            parse_cid(report_cid)?,
            did.clone(),
            &sk,
            parse_as_of(exculpated_at)?.unwrap_or_else(time::now),
        )
        .map_err(kc_err_to_py)?;
        let signed = Signed::build(exc, &[(did.as_str(), &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SourceExculpated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// §8 observer dashboard as of `as_of`: the composition-drift board bundling
    /// the sponsorship graph, the rejection pattern, and per-community observer /
    /// whistleblower report tallies.
    #[pyo3(signature = (as_of=None))]
    fn observer_dashboard(&self, as_of: Option<i64>) -> PyResult<PyObserverDashboard> {
        let as_of_ts = parse_as_of(as_of)?;
        let d = kc_reputation::observer_dashboard(&self.inner, as_of_ts);
        Ok(PyObserverDashboard::from(&d))
    }

    /// §4 mediator pool-share / rotation / lineage lens as of `as_of`: the active
    /// pool's composition by seating community **and by training lineage** (no verdict
    /// — the caps are enforced by `check_authority`).
    ///
    /// The lineage half is the only reader of `trained_by`. §4's apprenticeship model
    /// carries its own named weakness — mediators trained under experienced mediators
    /// are "susceptible to capture by the incumbent generation's biases" — and a pool
    /// that all traces to one trainer is that weakness realized. It is reported and
    /// never gated: the source documents warn against a certification barrier becoming
    /// a gatekeeping mechanism.
    #[pyo3(signature = (as_of=None))]
    fn mediator_pool_concentration(
        &self,
        as_of: Option<i64>,
    ) -> PyResult<PyMediatorPoolConcentration> {
        let as_of_ts = parse_as_of(as_of)?;
        let p = kc_reputation::mediator_pool_concentration(&self.inner, as_of_ts);
        Ok(PyMediatorPoolConcentration::from(&p))
    }

    /// §8 observer pool-share / rotation lens as of `as_of`: the active corps'
    /// composition by nominating community and the observers currently serving
    /// (no verdict — the caps are enforced by `check_authority`).
    #[pyo3(signature = (as_of=None))]
    fn observer_pool_concentration(
        &self,
        as_of: Option<i64>,
    ) -> PyResult<PyObserverPoolConcentration> {
        let as_of_ts = parse_as_of(as_of)?;
        let p = kc_reputation::observer_pool_concentration(&self.inner, as_of_ts);
        Ok(PyObserverPoolConcentration::from(&p))
    }

    // ---- §9 Substrate-deference (constitution) authoring -----------------

    /// Ratify the founding constitution (§9): the initial compact-incapable list
    /// (`incapable` = list of (key, description)) and structured-fast cadence
    /// (`fast_periods` = list of (label, start, end, recurrence); `recurrence` is
    /// an optional cycle in seconds for a repeating fast, else None), co-signed by
    /// the founding communities. Second-class (quorum); a singleton. Returns the CID.
    #[pyo3(signature = (ratifier_privs, ratifier_dids, incapable, fast_periods, genesis_at=None))]
    fn ratify_constitution(
        &mut self,
        ratifier_privs: Vec<Vec<u8>>,
        ratifier_dids: Vec<String>,
        incapable: Vec<(String, String)>,
        fast_periods: Vec<(String, i64, i64, Option<i64>)>,
        genesis_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&ratifier_privs, &ratifier_dids)?;
        let matters = incapable
            .into_iter()
            .map(|(key, description)| IncapableMatter { key, description })
            .collect();
        let periods = fast_periods
            .into_iter()
            .map(|(label, start, end, recurrence)| {
                Ok(FastPeriod { label, start: ts(start)?, end: ts(end)?, recurrence })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let g = ConstitutionGenesis {
            compact_incapable: matters,
            fast_periods: periods,
            ratifiers: ids,
            genesis_at: parse_as_of(genesis_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(g, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ConstitutionRatified(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Amend the compact-incapable list (§9). `kind` is "Add" or "Remove";
    /// second-class. Adds use the normal quorum; Removes require the higher
    /// contraction bar and may not name a genesis-original matter. Returns the CID.
    #[pyo3(signature = (decider_privs, decider_dids, kind, matter_key, matter_description=String::new(), rationale=String::new(), decision_class="SecondClass".to_string(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn amend_compact_incapable(
        &mut self,
        decider_privs: Vec<Vec<u8>>,
        decider_dids: Vec<String>,
        kind: &str,
        matter_key: String,
        matter_description: String,
        rationale: String,
        decision_class: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&decider_privs, &decider_dids)?;
        let a = CompactIncapableAmendment {
            kind: parse_amendment_kind(kind)?,
            matter: IncapableMatter { key: matter_key, description: matter_description },
            rationale,
            deciders: ids,
            decision_class: parse_decision_class(&decision_class)?,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(a, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::CompactIncapableAmended(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Amend the structured-fast cadence (§9). `change` is "Extend" or "Contract"
    /// (Contract carries the higher bar). `period` is the resulting (label, start,
    /// end); `recurrence` is an optional cycle in seconds for a repeating fast.
    /// Second-class. Returns the CID.
    #[pyo3(signature = (decider_privs, decider_dids, change, label, start, end, rationale=String::new(), decision_class="SecondClass".to_string(), decided_at=None, recurrence=None))]
    #[allow(clippy::too_many_arguments)]
    fn amend_fast_cadence(
        &mut self,
        decider_privs: Vec<Vec<u8>>,
        decider_dids: Vec<String>,
        change: &str,
        label: String,
        start: i64,
        end: i64,
        rationale: String,
        decision_class: String,
        decided_at: Option<i64>,
        recurrence: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&decider_privs, &decider_dids)?;
        let a = FastCadenceAmendment {
            change: parse_fast_change(change)?,
            period: FastPeriod { label, start: ts(start)?, end: ts(end)?, recurrence },
            rationale,
            deciders: ids,
            decision_class: parse_decision_class(&decision_class)?,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(a, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::FastCadenceAmended(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Adopt the functional minimum as the ground of the compact's own conduct (§9).
    /// `forbearances` is a list of (key, description, site) where site is one of
    /// "NonMemberDuties", "HostisHumaniGeneris", "RestrictedMember",
    /// "BoundaryRefusal". Co-signed by the adopting communities; second-class, and a
    /// singleton. Returns the CID.
    ///
    /// This is **not** an entry on the compact-incapable list and must not be filed as
    /// one: that list names matters belonging to a member, this one binds the compact.
    #[pyo3(signature = (adopter_privs, adopter_dids, forbearances, decision_class="SecondClass".to_string(), adopted_at=None))]
    fn adopt_minimum_content(
        &mut self,
        adopter_privs: Vec<Vec<u8>>,
        adopter_dids: Vec<String>,
        forbearances: Vec<(String, String, String)>,
        decision_class: String,
        adopted_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&adopter_privs, &adopter_dids)?;
        let list = forbearances
            .into_iter()
            .map(|(key, description, site)| {
                Ok(Forbearance { key, description, site: parse_minimum_site(&site)? })
            })
            .collect::<PyResult<Vec<_>>>()?;
        let a = MinimumContentAdoption {
            forbearances: list,
            adopters: ids,
            decision_class: parse_decision_class(&decision_class)?,
            adopted_at: parse_as_of(adopted_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(a, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::MinimumContentAdopted(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Amend the adopted minimum content (§9). `change` is "Broaden" or "Narrow";
    /// second-class. Broadenings use the normal quorum; narrowings carry the higher
    /// contraction bar, at every site and including a founding forbearance — the
    /// doctrine asks for a bar there, not a lock. Returns the CID.
    #[pyo3(signature = (decider_privs, decider_dids, change, forbearance_key, description=String::new(), site="NonMemberDuties".to_string(), rationale=String::new(), decision_class="SecondClass".to_string(), decided_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn amend_minimum_content(
        &mut self,
        decider_privs: Vec<Vec<u8>>,
        decider_dids: Vec<String>,
        change: &str,
        forbearance_key: String,
        description: String,
        site: String,
        rationale: String,
        decision_class: String,
        decided_at: Option<i64>,
    ) -> PyResult<String> {
        let (ids, pairs_owned) = cosigners(&decider_privs, &decider_dids)?;
        let a = MinimumContentAmendment {
            change: parse_forbearance_change(change)?,
            forbearance: Forbearance {
                key: forbearance_key,
                description,
                site: parse_minimum_site(&site)?,
            },
            rationale,
            deciders: ids,
            decision_class: parse_decision_class(&decision_class)?,
            decided_at: parse_as_of(decided_at)?.unwrap_or_else(time::now),
        };
        let pairs: Vec<(&str, &PrivKey)> = pairs_owned.iter().map(|(x, k)| (x.as_str(), k)).collect();
        let signed = Signed::build(a, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::MinimumContentAmended(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// §9 self-binding gate: is `forbearance_key` part of the ground the compact has
    /// bound its own conduct to, as of `as_of`? The twin of `compact_incapable`, and
    /// the two never answer each other's question.
    #[pyo3(signature = (forbearance_key, as_of=None))]
    fn minimum_forbearance(&self, forbearance_key: &str, as_of: Option<i64>) -> PyResult<bool> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(kc_reputation::minimum_forbearance(&self.inner, forbearance_key, as_of_ts))
    }

    /// §9 constitutional state as of `as_of`: whether ratified, the in-force
    /// compact-incapable matters, and the current structured-fast cadence.
    #[pyo3(signature = (as_of=None))]
    fn constitution_state(&self, as_of: Option<i64>) -> PyResult<PyConstitutionState> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(PyConstitutionState::from(&kc_reputation::constitution_state(&self.inner, as_of_ts)))
    }

    /// §9 categorical-deference gate: is `matter_key` a matter the compact will
    /// not decide, as of `as_of`?
    #[pyo3(signature = (matter_key, as_of=None))]
    fn compact_incapable(&self, matter_key: &str, as_of: Option<i64>) -> PyResult<bool> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(kc_reputation::compact_incapable(&self.inner, matter_key, as_of_ts))
    }

    /// §9 temporal-deference gate: the label of the structured-fast period
    /// containing `at`, or None if the compact is not fasting then.
    fn in_structured_fast(&self, at: i64) -> PyResult<Option<String>> {
        Ok(kc_reputation::in_structured_fast(&self.inner, ts(at)?))
    }

    /// §9 substrate-deference / erosion report as of `as_of`: the size and drift
    /// of the compact-incapable list, the fast cadence and its contraction count,
    /// and deliberative acts initiated during a fast.
    #[pyo3(signature = (as_of=None))]
    fn substrate_deference(&self, as_of: Option<i64>) -> PyResult<PySubstrateDeferenceReport> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(PySubstrateDeferenceReport::from(&kc_reputation::substrate_deference(&self.inner, as_of_ts)))
    }

    /// §9 erosion alert: a graded, derived reading of the substrate-deference
    /// report plus the two absolute ceremony signals (severity None/Notice/Alert
    /// with reasons), escalating as erosion signals compound. Diagnostic only —
    /// surfaced to the observer corps.
    ///
    /// `rite_cadence_seconds` overrides the generational re-affirmation cadence an
    /// overdue rite is judged against (default 30 years); the calibration is an
    /// open question, so it is a parameter rather than a constant.
    #[pyo3(signature = (
        as_of=None,
        rite_cadence_seconds=None,
        attestation_cadence_seconds=None,
        enforcement_window_seconds=None,
        enforcement_volume_floor=None,
        enforcement_coverage_floor_permille=None,
        enforcement_dispersion_ceiling_permille=None,
        enforcement_downward_floor_permille=None,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn erosion_alert(
        &self,
        as_of: Option<i64>,
        rite_cadence_seconds: Option<i64>,
        attestation_cadence_seconds: Option<i64>,
        enforcement_window_seconds: Option<i64>,
        enforcement_volume_floor: Option<usize>,
        enforcement_coverage_floor_permille: Option<u32>,
        enforcement_dispersion_ceiling_permille: Option<u32>,
        enforcement_downward_floor_permille: Option<u32>,
    ) -> PyResult<PyErosionAlert> {
        let as_of_ts = parse_as_of(as_of)?;
        let mut thresholds = kc_reputation::ErosionThresholds::default();
        if let Some(c) = rite_cadence_seconds {
            thresholds.rite_cadence_seconds = c;
        }
        if let Some(c) = attestation_cadence_seconds {
            thresholds.attestation_cadence_seconds = c;
        }
        if let Some(w) = enforcement_window_seconds {
            thresholds.enforcement_window_seconds = w;
        }
        if let Some(v) = enforcement_volume_floor {
            thresholds.enforcement_volume_floor = v;
        }
        if let Some(p) = enforcement_coverage_floor_permille {
            thresholds.enforcement_coverage_floor_permille = p;
        }
        if let Some(p) = enforcement_dispersion_ceiling_permille {
            thresholds.enforcement_dispersion_ceiling_permille = p;
        }
        if let Some(p) = enforcement_downward_floor_permille {
            thresholds.enforcement_downward_floor_permille = p;
        }
        Ok(PyErosionAlert::from(&kc_reputation::erosion_alert_with(
            &self.inner,
            as_of_ts,
            &thresholds,
        )))
    }

    /// Substitution lens — what the compact's enforcement is aimed at over the
    /// window ending at `as_of`: how many acts, how much of the shared taxonomy
    /// they still cite, and how widely they are spread over subject communities.
    ///
    /// Compact-wide. A polity keeping every form and intensifying it can still
    /// narrow what it enforces to a single ground, and every instrument that counts
    /// observances reads that polity as healthy; this counts what the observances
    /// are *about*. No verdict — the cut-points are applied by `erosion_alert`.
    #[pyo3(signature = (as_of=None, window_seconds=None))]
    fn enforcement_convergence(
        &self,
        as_of: Option<i64>,
        window_seconds: Option<i64>,
    ) -> PyResult<PyEnforcementConvergence> {
        let as_of_ts = parse_as_of(as_of)?;
        let w = window_seconds
            .unwrap_or_else(|| kc_reputation::ErosionThresholds::default().enforcement_window_seconds);
        Ok(PyEnforcementConvergence::from(
            &kc_reputation::enforcement_convergence(&self.inner, as_of_ts, w),
        ))
    }

    /// The same reading per **enforcing** community, keyed by reporter DID.
    ///
    /// The drill-down that names who, and deliberately not what the alert reads: a
    /// compact whose communities each specialise scores as converged on every
    /// per-community reading while still enforcing the whole object between them.
    #[pyo3(signature = (as_of=None, window_seconds=None))]
    fn enforcement_convergence_all(
        &self,
        as_of: Option<i64>,
        window_seconds: Option<i64>,
    ) -> PyResult<BTreeMap<String, PyEnforcementConvergence>> {
        let as_of_ts = parse_as_of(as_of)?;
        let w = window_seconds
            .unwrap_or_else(|| kc_reputation::ErosionThresholds::default().enforcement_window_seconds);
        Ok(kc_reputation::enforcement_convergence_all(&self.inner, as_of_ts, w)
            .iter()
            .map(|(k, v)| (k.clone(), PyEnforcementConvergence::from(v)))
            .collect())
    }

    /// Substitution lens — which way the compact's enforcement runs: at
    /// communities holding reduced standing (§2 probation), at communities holding
    /// full standing, or between equals. Standing is resolved at the instant each
    /// act was recorded, not at `as_of`.
    #[pyo3(signature = (as_of=None, window_seconds=None))]
    fn enforcement_direction(
        &self,
        as_of: Option<i64>,
        window_seconds: Option<i64>,
    ) -> PyResult<PyEnforcementDirection> {
        let as_of_ts = parse_as_of(as_of)?;
        let w = window_seconds
            .unwrap_or_else(|| kc_reputation::ErosionThresholds::default().enforcement_window_seconds);
        Ok(PyEnforcementDirection::from(
            &kc_reputation::enforcement_direction(&self.inner, as_of_ts, w),
        ))
    }

    /// The same reading per **enforcing** community, keyed by reporter DID.
    #[pyo3(signature = (as_of=None, window_seconds=None))]
    fn enforcement_direction_all(
        &self,
        as_of: Option<i64>,
        window_seconds: Option<i64>,
    ) -> PyResult<BTreeMap<String, PyEnforcementDirection>> {
        let as_of_ts = parse_as_of(as_of)?;
        let w = window_seconds
            .unwrap_or_else(|| kc_reputation::ErosionThresholds::default().enforcement_window_seconds);
        Ok(kc_reputation::enforcement_direction_all(&self.inner, as_of_ts, w)
            .iter()
            .map(|(k, v)| (k.clone(), PyEnforcementDirection::from(v)))
            .collect())
    }

    /// §5 designation follow-through: designations whose mandated
    /// `reexamine_after` has passed and which are still standing, longest-overdue
    /// first, as (designation_cid, subject, tier, reexamine_after,
    /// seconds_overdue). Read-only and no verdict — an exclusion the compact
    /// undertook to revisit and has not.
    #[pyo3(signature = (as_of=None))]
    fn overdue_reexaminations(
        &self,
        as_of: Option<i64>,
    ) -> PyResult<Vec<OverdueReexaminationRow>> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(kc_reputation::overdue_reexaminations(&self.inner, as_of_ts)
            .iter()
            .map(|o| {
                (
                    o.designation_cid.to_string(),
                    o.subject.clone(),
                    o.tier.clone(),
                    o.reexamine_after.as_second(),
                    o.seconds_overdue,
                    o.confirmations,
                )
            })
            .collect())
    }

    /// Standing functions with no act and no attested null inside `cadence_seconds`, as
    /// (community_did, function, seconds_since, cadence_seconds, last_was_a_null),
    /// longest silence first. An absolute measure: every relative reading of a pool or a
    /// corps is unchanged by its simply stopping.
    #[pyo3(signature = (as_of=None, cadence_seconds=None))]
    fn silent_functions(
        &self,
        as_of: Option<i64>,
        cadence_seconds: Option<i64>,
    ) -> PyResult<Vec<SilentFunctionRow>> {
        let as_of_ts = parse_as_of(as_of)?;
        let cadence = cadence_seconds
            .unwrap_or_else(|| kc_reputation::ErosionThresholds::default().attestation_cadence_seconds);
        Ok(kc_reputation::silent_functions(&self.inner, as_of_ts, cadence)
            .iter()
            .map(|f| {
                (
                    f.community_did.clone(),
                    f.function.clone(),
                    f.seconds_since,
                    f.cadence_seconds,
                    f.last_was_a_null,
                )
            })
            .collect())
    }

    /// Functions a community once staffed and now has nobody seated in, as
    /// (community_did, function, vacant_since, seconds_vacant), longest vacancy first. A
    /// function never staffed is absent from the reading.
    #[pyo3(signature = (as_of=None))]
    fn unstaffed_functions(
        &self,
        as_of: Option<i64>,
    ) -> PyResult<Vec<(String, String, i64, i64)>> {
        let as_of_ts = parse_as_of(as_of)?;
        Ok(kc_reputation::unstaffed_functions(&self.inner, as_of_ts)
            .iter()
            .map(|u| {
                (
                    u.community_did.clone(),
                    u.function.clone(),
                    u.vacant_since.as_second(),
                    u.seconds_vacant,
                )
            })
            .collect())
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

    /// How much of `compact_label`'s roll carries deliberative weight as of
    /// `as_of` (§2): the roll, who is probationary, and whether an ordinary
    /// second-class decision — or one at the contraction bar — is available at all.
    #[pyo3(signature = (compact_label, quorum_threshold=2, as_of=None))]
    fn deliberative_capacity(
        &self,
        compact_label: &str,
        quorum_threshold: usize,
        as_of: Option<i64>,
    ) -> PyResult<PyDeliberativeCapacity> {
        let as_of_ts = parse_as_of(as_of)?;
        let c = kc_reputation::deliberative_capacity(
            &self.inner,
            compact_label,
            quorum_threshold,
            as_of_ts,
        );
        Ok(PyDeliberativeCapacity::from(&c))
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

    /// Move 1 jester lens — interpretive-plurality report for `community` as of
    /// `as_of`: its substrate readings, the per-lineage and per-interpreter
    /// counts, and the interpreter-concentration ranking a monopoly heads. No
    /// verdict — the monopoly threshold is the caller's.
    #[pyo3(signature = (community, as_of=None))]
    fn interpretive_plurality(
        &self,
        community: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyInterpretivePluralityReport> {
        let did = parse_did(community)?;
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::interpretive_plurality(&self.inner, &did, as_of_ts);
        Ok(PyInterpretivePluralityReport::from(&r))
    }

    /// Move 4 jester lens — structured-skepticism report over the whole archive
    /// as of `as_of`: every event report's provenance, qualifying triggers, and
    /// distinct-community corroborators (excluding the originator). No verdict —
    /// the corroboration threshold is the caller's.
    #[pyo3(signature = (as_of=None))]
    fn structured_skepticism(&self, as_of: Option<i64>) -> PyResult<PySkepticismReport> {
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::structured_skepticism(&self.inner, as_of_ts);
        Ok(PySkepticismReport::from(&r))
    }

    /// Move 6 jester lens — adversarial-principal report for `principal` as of
    /// `as_of`: cross-community witness attestations (excluding the principal
    /// and its home) and output divergences from the cohort majority (with
    /// whether each benefits the principal's own side). No verdict.
    #[pyo3(signature = (principal, as_of=None))]
    fn adversarial_principal(
        &self,
        principal: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyPrincipalValidationReport> {
        let did = parse_did(principal)?;
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::adversarial_principal(&self.inner, &did, as_of_ts);
        Ok(PyPrincipalValidationReport::from(&r))
    }

    /// Move 6 jester lens — collusive whole-cohort report as of `as_of`:
    /// cross-cohort triangulation surfacing cohorts whose majority on a shared
    /// matter diverges from the cross-cohort consensus (self-serving divergence
    /// being the whole-cohort capture signature per-principal triangulation
    /// misses). No verdict.
    #[pyo3(signature = (as_of=None))]
    fn collusive_cohorts(&self, as_of: Option<i64>) -> PyResult<PyCohortCollusionReport> {
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::collusive_cohorts(&self.inner, as_of_ts);
        Ok(PyCohortCollusionReport::from(&r))
    }

    /// Move 6 jester lens — the witness-attestation chain for `principal` as of
    /// `as_of`: the principal's attestation links oldest-first, with chain
    /// integrity (dangling `prev`, forks) and separability (the count of
    /// cross-community links the home community cannot forge). No verdict.
    #[pyo3(signature = (principal, as_of=None))]
    fn principal_witness_chain(
        &self,
        principal: &str,
        as_of: Option<i64>,
    ) -> PyResult<PyWitnessChainReport> {
        let did = parse_did(principal)?;
        let as_of_ts = parse_as_of(as_of)?;
        let r = kc_reputation::principal_witness_chain(&self.inner, &did, as_of_ts);
        Ok(PyWitnessChainReport::from(&r))
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

    /// Append a signed substrate interpretation (capture move 1): one point on a
    /// traceable interpretive lineage. The envelope is signed by `signer_priv`/
    /// `signer_did` (the community root, or a key it delegated); the payload
    /// attributes the reading to `community_did`. `interpreter_did` is the
    /// internal author (the plurality unit). `interprets_cid` optionally links
    /// the rite/recitation it reads. Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, community_did, lineage_id, interpreter_did, divergence=String::new(), interprets_cid=None, references=Vec::new(), recorded_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_interpretation(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        community_did: &str,
        lineage_id: String,
        interpreter_did: &str,
        divergence: String,
        interprets_cid: Option<String>,
        references: Vec<String>,
        recorded_at: Option<i64>,
    ) -> PyResult<String> {
        let community = parse_did(community_did)?;
        let interpreter = parse_did(interpreter_did)?;
        let interprets = interprets_cid.as_deref().map(parse_cid).transpose()?;
        let refs: Vec<Cid> = references.iter().map(|c| parse_cid(c)).collect::<PyResult<_>>()?;
        let rec = SubstrateInterpretation {
            community_did: community,
            lineage_id,
            interpreter_did: interpreter,
            interprets_cid: interprets,
            divergence,
            references: refs,
            recorded_at: parse_as_of(recorded_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::SubstrateInterpreted(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed interpreter-independence attestation (capture move 1,
    /// anti-sockpuppet). The envelope is signed by `signer_priv`/`signer_did`; the
    /// payload attributes the vouching to `attesting_party_did`. The
    /// `interpretive_plurality` lens counts it toward `interpreter_did`'s
    /// independence only when the attester is neither `community_did` nor the
    /// interpreter. Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, attesting_party_did, community_did, interpreter_did, basis=String::new(), attested_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn attest_interpreter_independence(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        attesting_party_did: &str,
        community_did: &str,
        interpreter_did: &str,
        basis: String,
        attested_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = InterpreterIndependenceAttestation {
            community_did: parse_did(community_did)?,
            interpreter_did: parse_did(interpreter_did)?,
            attesting_party: parse_did(attesting_party_did)?,
            basis,
            attested_at: parse_as_of(attested_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InterpreterIndependenceAttested(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed event report (capture move 4), provenance-tagged at
    /// intake. Signed by `signer_priv`/`signer_did`; attributed to
    /// `originator_did`. The booleans are the structured-skepticism qualifying
    /// triggers; `aligned_faction_did` is the conveniently-aligned faction (or
    /// None). Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, originator_did, summary, high_interpretive_charge=false, sudden_onset=false, originator_holds_stake=false, aligned_faction_did=None, declared_incentives=String::new(), information_history=String::new(), disclosed_interests=Vec::new(), onset_at=None, reported_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_event(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        originator_did: &str,
        summary: String,
        high_interpretive_charge: bool,
        sudden_onset: bool,
        originator_holds_stake: bool,
        aligned_faction_did: Option<String>,
        declared_incentives: String,
        information_history: String,
        disclosed_interests: Vec<String>,
        onset_at: Option<i64>,
        reported_at: Option<i64>,
    ) -> PyResult<String> {
        let originator = parse_did(originator_did)?;
        let aligned = aligned_faction_did.as_deref().map(parse_did).transpose()?;
        let reported = parse_as_of(reported_at)?.unwrap_or_else(time::now);
        let onset = parse_as_of(onset_at)?.unwrap_or(reported);
        let rec = EventReport {
            originator,
            summary,
            high_interpretive_charge,
            sudden_onset,
            originator_holds_stake,
            aligned_faction: aligned,
            declared_incentives,
            information_history,
            disclosed_interests,
            onset_at: onset,
            reported_at: reported,
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::EventReported(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed corroboration of an event report by another community
    /// (capture move 4). Signed by `signer_priv`/`signer_did`; attributed to
    /// `community_did`. `event_cid` is the `EventReported` entry CID. Returns the
    /// corroboration entry CID.
    #[pyo3(signature = (signer_priv, signer_did, community_did, event_cid, translation=None, corroborated_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_corroboration(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        community_did: &str,
        event_cid: &str,
        translation: Option<String>,
        corroborated_at: Option<i64>,
    ) -> PyResult<String> {
        let community = parse_did(community_did)?;
        let rec = EventCorroboration {
            event_cid: parse_cid(event_cid)?,
            corroborating_community: community,
            translation,
            corroborated_at: parse_as_of(corroborated_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::EventCorroborated(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed formal action premised on an event report (capture move 4).
    /// Signed by `signer_priv`/`signer_did`; attributed to `acting_community_did`.
    /// `event_cid` is the `EventReported` entry CID. The archive's authority pass
    /// refuses this while the cited *qualifying* event is still within its
    /// structured-skepticism hold and under-corroborated. Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, acting_community_did, event_cid, action=String::new(), invoked_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn invoke_event_action(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        acting_community_did: &str,
        event_cid: &str,
        action: String,
        invoked_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = EventActionInvoked {
            event_cid: parse_cid(event_cid)?,
            acting_community: parse_did(acting_community_did)?,
            action,
            invoked_at: parse_as_of(invoked_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::EventActionInvoked(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed retroactive interest finding (capture move 4): a third
    /// party discloses an interest the originator of `event_cid` held at intake but
    /// did not declare. Signed by `signer_priv`/`signer_did`; attributed to
    /// `finder_did`. The `structured_skepticism` lens marks the event impugned
    /// (unless the finder is the event's own originator). Returns the entry CID.
    #[pyo3(signature = (signer_priv, signer_did, finder_did, event_cid, undisclosed_interest=String::new(), found_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn find_undisclosed_interest(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        finder_did: &str,
        event_cid: &str,
        undisclosed_interest: String,
        found_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = RetroactiveInterestFinding {
            event_cid: parse_cid(event_cid)?,
            finder: parse_did(finder_did)?,
            undisclosed_interest,
            found_at: parse_as_of(found_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::RetroactiveInterestFound(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed cross-community principal-witness attestation (capture
    /// move 6). Signed by `signer_priv`/`signer_did`; attributed to
    /// `witness_community_did`. `principal_home_did` is the principal's own
    /// community. Returns the entry CID.
    /// `prev` optionally names the previous attestation in this principal's
    /// chain (its entry CID), making continuous attestation a tamper-evident
    /// hash-linked chain; omit it for the chain root.
    #[pyo3(signature = (signer_priv, signer_did, witness_community_did, principal_did, principal_home_did, basis=String::new(), prev=None, attested_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_principal_witness(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        witness_community_did: &str,
        principal_did: &str,
        principal_home_did: &str,
        basis: String,
        prev: Option<&str>,
        attested_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = PrincipalWitnessAttestation {
            principal_did: parse_did(principal_did)?,
            principal_home: parse_did(principal_home_did)?,
            witness_community: parse_did(witness_community_did)?,
            basis,
            prev: prev.map(parse_cid).transpose()?,
            attested_at: parse_as_of(attested_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::PrincipalWitnessed(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed principal output for triangulation (capture move 6).
    /// Signed by `signer_priv`/`signer_did`; the output is attributed to
    /// `principal_home_did` (the home community recording its principal's
    /// output). `cohort`/`matter` scope the comparison; `favors_home` declares
    /// whether the decision benefits the principal's own side. Returns the CID.
    #[pyo3(signature = (signer_priv, signer_did, principal_did, principal_home_did, cohort, matter, decision, favors_home=false, produced_at=None))]
    #[allow(clippy::too_many_arguments)]
    fn append_principal_output(
        &mut self,
        signer_priv: &[u8],
        signer_did: &str,
        principal_did: &str,
        principal_home_did: &str,
        cohort: String,
        matter: String,
        decision: String,
        favors_home: bool,
        produced_at: Option<i64>,
    ) -> PyResult<String> {
        let rec = PrincipalOutput {
            principal_did: parse_did(principal_did)?,
            principal_home: parse_did(principal_home_did)?,
            cohort,
            matter,
            decision,
            favors_home,
            produced_at: parse_as_of(produced_at)?.unwrap_or_else(time::now),
        };
        let sk = priv_from_bytes(signer_priv);
        let signed = Signed::build(rec, &[(signer_did, &sk)]).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::PrincipalOutputRecorded(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append a signed inter-community non-admission record (at least one feature
    /// claim was rejected). Signed by the `signer_privs[i]`/`signer_dids[i]`
    /// pairs (participant root keys, or keys they have delegated). `rejections`
    /// is a list of `(rejecting_did, presenting_did, feature, rationale)`. The
    /// Append a signed inter-community admission record (§2). `signers` are the
    /// cross-boundary signers, `participants` the communities the record speaks for
    /// — the sitting members and the entrant together, as the ceremony seals them.
    ///
    /// The first such record for a `compact_label` is that compact's *formation* and
    /// is exempt from the §2 admission rules. Every later one is an admission into a
    /// standing compact and must carry an admitting quorum of sitting, non-probationary
    /// members plus an examining panel of seated examiners spanning at least two
    /// communities — see `check_authority`.
    ///
    /// This authors the record directly, as `record_non_admission` does, rather than
    /// driving the full claim/acceptance ceremony (`two_community_credentialing`).
    #[pyo3(signature = (signer_privs, signer_dids, participants, compact_label, witnessed_at=None))]
    fn append_admission(
        &mut self,
        signer_privs: Vec<Vec<u8>>,
        signer_dids: Vec<String>,
        participants: Vec<String>,
        compact_label: String,
        witnessed_at: Option<i64>,
    ) -> PyResult<String> {
        if signer_privs.len() != signer_dids.len() {
            return Err(PyValueError::new_err(
                "signer_privs and signer_dids must be the same length",
            ));
        }
        if signer_privs.is_empty() {
            return Err(PyValueError::new_err("at least one signer is required"));
        }
        let witnessed = parse_as_of(witnessed_at)?.unwrap_or_else(time::now);
        let parts: Vec<DidKey> =
            participants.iter().map(|p| parse_did(p)).collect::<PyResult<_>>()?;
        let rec = InterCommunityWitnessRecord {
            compact_label,
            participants: parts,
            sequenced_rites: vec![],
            claims: vec![],
            acceptances: vec![],
            examination_exchanges: vec![],
            witnessed_at: witnessed,
        };
        let sks: Vec<PrivKey> = signer_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let pairs: Vec<(&str, &PrivKey)> =
            signer_dids.iter().map(|s| s.as_str()).zip(sks.iter()).collect();
        let signed = Signed::build(rec, &pairs).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::InterCommunityWitness(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

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

    /// Append a community's account of one window in a standing function it holds. An
    /// empty `acts` is the attested null and is the point of the record.
    #[allow(clippy::too_many_arguments)]
    fn append_activity_attestation(
        &mut self,
        signer_privs: Vec<Vec<u8>>,
        signer_dids: Vec<String>,
        attesting_did: &str,
        function: &str,
        window_start: i64,
        window_end: i64,
        acts: Vec<String>,
        statement: String,
        attested_at: i64,
    ) -> PyResult<String> {
        if signer_privs.len() != signer_dids.len() || signer_privs.is_empty() {
            return Err(PyValueError::new_err(
                "signer_privs and signer_dids must be non-empty and equal length",
            ));
        }
        let att = ActivityAttestation {
            function: parse_attested_function(function)?,
            attesting_community: parse_did(attesting_did)?,
            window_start: ts(window_start)?,
            window_end: ts(window_end)?,
            acts: acts.iter().map(|s| parse_cid(s)).collect::<PyResult<_>>()?,
            statement,
            attested_at: ts(attested_at)?,
        };
        let sks: Vec<PrivKey> = signer_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let signers: Vec<(&str, &PrivKey)> = signer_dids
            .iter()
            .zip(sks.iter())
            .map(|(d, sk)| (d.as_str(), sk))
            .collect();
        let signed = Signed::build(att, &signers).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ActivityAttested(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }

    /// Append the conclusion of a designation's mandated re-examination (§5). `outcome` is
    /// "Confirmed" or "Reversed"; a confirmation must set `next_reexamine_after`, since it
    /// renews the obligation rather than discharging it.
    #[allow(clippy::too_many_arguments)]
    fn append_reexamination(
        &mut self,
        signer_privs: Vec<Vec<u8>>,
        signer_dids: Vec<String>,
        examiner_dids: Vec<String>,
        designation_cid: &str,
        outcome: &str,
        reasoning: String,
        next_reexamine_after: Option<i64>,
        concluded_at: i64,
    ) -> PyResult<String> {
        if signer_privs.len() != signer_dids.len() || signer_privs.is_empty() {
            return Err(PyValueError::new_err(
                "signer_privs and signer_dids must be non-empty and equal length",
            ));
        }
        if examiner_dids.is_empty() {
            return Err(PyValueError::new_err("examiner_dids must be non-empty"));
        }
        let rc = ReexaminationConclusion {
            designation_cid: parse_cid(designation_cid)?,
            outcome: parse_reexamination_outcome(outcome)?,
            examiners: examiner_dids
                .iter()
                .map(|d| parse_did(d))
                .collect::<PyResult<_>>()?,
            reasoning,
            next_reexamine_after: next_reexamine_after.map(ts).transpose()?,
            decision_class: kc_conduct::DecisionClass::SecondClass,
            concluded_at: ts(concluded_at)?,
        };
        let sks: Vec<PrivKey> = signer_privs.iter().map(|b| priv_from_bytes(b)).collect();
        let signers: Vec<(&str, &PrivKey)> = signer_dids
            .iter()
            .zip(sks.iter())
            .map(|(d, sk)| (d.as_str(), sk))
            .collect();
        let signed = Signed::build(rc, &signers).map_err(kc_err_to_py)?;
        self.inner
            .append(ArchiveEntry::ReexaminationConcluded(signed))
            .map(|c| c.to_string())
            .map_err(kc_err_to_py)
    }
}

fn parse_attested_function(s: &str) -> PyResult<AttestedFunction> {
    match s {
        "Mediation" => Ok(AttestedFunction::Mediation),
        "Observation" => Ok(AttestedFunction::Observation),
        other => Err(PyValueError::new_err(format!("unknown attested function: {other}"))),
    }
}

fn parse_reexamination_outcome(s: &str) -> PyResult<ReexaminationOutcome> {
    match s {
        "Confirmed" => Ok(ReexaminationOutcome::Confirmed),
        "Reversed" => Ok(ReexaminationOutcome::Reversed),
        other => Err(PyValueError::new_err(format!("unknown re-examination outcome: {other}"))),
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

fn parse_recusal_answer(s: &str) -> PyResult<RecusalAnswer> {
    match s {
        "Granted" => Ok(RecusalAnswer::Granted),
        "Declined" => Ok(RecusalAnswer::Declined),
        _ => Err(PyValueError::new_err(format!(
            "unknown recusal answer '{s}' (expected Granted or Declined)"
        ))),
    }
}

fn parse_argument_role(s: &str) -> PyResult<ArgumentRole> {
    match s {
        "Party" => Ok(ArgumentRole::Party),
        "Panelist" => Ok(ArgumentRole::Panelist),
        _ => Err(PyValueError::new_err(format!(
            "unknown argument role '{s}' (expected Party or Panelist)"
        ))),
    }
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

fn parse_appropriator_level(s: &str) -> PyResult<AppropriatorLevel> {
    Ok(match s {
        "Community" => AppropriatorLevel::Community,
        "Compact" => AppropriatorLevel::Compact,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown appropriator level: {other}"
            )))
        }
    })
}

fn parse_consent_answer(s: &str) -> PyResult<ConsentAnswer> {
    Ok(match s {
        "Given" => ConsentAnswer::Given,
        "Withheld" => ConsentAnswer::Withheld,
        other => return Err(PyValueError::new_err(format!("unknown consent answer: {other}"))),
    })
}

fn parse_review_answer(s: &str) -> PyResult<ReviewAnswer> {
    Ok(match s {
        "Upheld" => ReviewAnswer::Upheld,
        "Pretextual" => ReviewAnswer::Pretextual,
        other => return Err(PyValueError::new_err(format!("unknown review answer: {other}"))),
    })
}

fn parse_skepticism_criterion(s: &str) -> PyResult<SkepticismCriterion> {
    Ok(match s {
        "HighInterpretiveCharge" => SkepticismCriterion::HighInterpretiveCharge,
        "SuddenOnset" => SkepticismCriterion::SuddenOnset,
        "OriginatorHoldsStake" => SkepticismCriterion::OriginatorHoldsStake,
        "AlignedFaction" => SkepticismCriterion::AlignedFaction,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown skepticism criterion: {other}"
            )))
        }
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

fn parse_minimum_site(s: &str) -> PyResult<MinimumSite> {
    Ok(match s {
        "NonMemberDuties" => MinimumSite::NonMemberDuties,
        "HostisHumaniGeneris" => MinimumSite::HostisHumaniGeneris,
        "RestrictedMember" => MinimumSite::RestrictedMember,
        "BoundaryRefusal" => MinimumSite::BoundaryRefusal,
        other => return Err(PyValueError::new_err(format!("unknown minimum-content site: {other}"))),
    })
}

fn parse_forbearance_change(s: &str) -> PyResult<ForbearanceChange> {
    Ok(match s {
        "Broaden" => ForbearanceChange::Broaden,
        "Narrow" => ForbearanceChange::Narrow,
        other => {
            return Err(PyValueError::new_err(format!(
                "unknown forbearance change: {other}"
            )))
        }
    })
}

fn parse_capability(s: &str) -> PyResult<Capability> {
    Ok(match s {
        "RescueCoordination" => Capability::RescueCoordination,
        "Navigation" => Capability::Navigation,
        "Trade" => Capability::Trade,
        "CommonsWithdrawal" => Capability::CommonsWithdrawal,
        "Communication" => Capability::Communication,
        // The offices — the roles that select who is sanctioned, revocable apart from
        // every relation of exchange above.
        "MediatorSeat" => Capability::MediatorSeat,
        "ObserverSeat" => Capability::ObserverSeat,
        "ExaminerSeat" => Capability::ExaminerSeat,
        "Deliberation" => Capability::Deliberation,
        "EmergencyPanelSeat" => Capability::EmergencyPanelSeat,
        other => return Err(PyValueError::new_err(format!("unknown capability: {other}"))),
    })
}

fn parse_commons_kind(s: &str) -> PyResult<CommonsKind> {
    Ok(match s {
        "LifeSupport" => CommonsKind::LifeSupport,
        "Communication" => CommonsKind::Communication,
        "Navigation" => CommonsKind::Navigation,
        "KnowledgeArchive" => CommonsKind::KnowledgeArchive,
        "Rescue" => CommonsKind::Rescue,
        other => return Err(PyValueError::new_err(format!("unknown commons kind: {other}"))),
    })
}

fn parse_amendment_kind(s: &str) -> PyResult<AmendmentKind> {
    Ok(match s {
        "Add" => AmendmentKind::Add,
        "Remove" => AmendmentKind::Remove,
        other => return Err(PyValueError::new_err(format!("unknown amendment kind: {other}"))),
    })
}

fn parse_fast_change(s: &str) -> PyResult<FastChange> {
    Ok(match s {
        "Extend" => FastChange::Extend,
        "Contract" => FastChange::Contract,
        other => return Err(PyValueError::new_err(format!("unknown fast change: {other}"))),
    })
}

fn parse_sanction_level(s: &str) -> PyResult<SanctionLevel> {
    Ok(match s {
        "None" => SanctionLevel::None,
        "Monitoring" => SanctionLevel::Monitoring,
        "Warning" => SanctionLevel::Warning,
        "Restriction" => SanctionLevel::Restriction,
        "Exclusion" => SanctionLevel::Exclusion,
        other => return Err(PyValueError::new_err(format!("unknown sanction level: {other}"))),
    })
}

/// Validate a multi-signer set and prepare it: returns the parsed principal DIDs
/// and owned `(did_string, priv_key)` pairs a caller can borrow for `Signed::build`.
fn cosigners(privs: &[Vec<u8>], dids: &[String]) -> PyResult<Cosigners> {
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

#[pyclass(skip_from_py_object, name = "IntraRiteSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "InterAdmissionSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "RejectionSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "InterNonAdmissionSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "DelegationSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "WitnessSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "ConductSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "SponsorshipSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "DisputeSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "DesignationSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "ReEntrySummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "RestitutionSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "SubstantiveReEngagementSummary", get_all)]
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

#[pyclass(skip_from_py_object, name = "InjunctionSummary", get_all)]
#[derive(Clone)]
struct PyInjunctionSummary {
    record_cid: String,
    harm: String,
    protective_measures: String,
    revoked_capabilities: Vec<String>,
    status: String,
    issued_at: i64,
    expires_at: i64,
}

impl From<&rep::InjunctionSummary> for PyInjunctionSummary {
    fn from(s: &rep::InjunctionSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            harm: s.harm.clone(),
            protective_measures: s.protective_measures.clone(),
            revoked_capabilities: s.revoked_capabilities.clone(),
            status: s.status.clone(),
            issued_at: s.issued_at.as_second(),
            expires_at: s.expires_at.as_second(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "CommonsSanctionSummary", get_all)]
#[derive(Clone)]
struct PyCommonsSanctionSummary {
    record_cid: String,
    commons_cid: String,
    kind: String,
    current_level: String,
    basis: String,
    decided_at: i64,
}

impl From<&rep::CommonsSanctionSummary> for PyCommonsSanctionSummary {
    fn from(s: &rep::CommonsSanctionSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            commons_cid: s.commons_cid.to_string(),
            kind: s.kind.clone(),
            current_level: s.current_level.clone(),
            basis: s.basis.clone(),
            decided_at: s.decided_at.as_second(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "ReportResponseSummary", get_all)]
#[derive(Clone)]
struct PyReportResponseSummary {
    record_cid: String,
    response: String,
    responded_at: i64,
}

impl From<&rep::ReportResponseSummary> for PyReportResponseSummary {
    fn from(s: &rep::ReportResponseSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            response: s.response.clone(),
            responded_at: s.responded_at.as_second(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "ObserverReportSummary", get_all)]
#[derive(Clone)]
struct PyObserverReportSummary {
    record_cid: String,
    observer_did: String,
    declared_perspective: String,
    summary: String,
    observed_at: i64,
    reported_at: i64,
    responses: Vec<PyReportResponseSummary>,
}

impl From<&rep::ObserverReportSummary> for PyObserverReportSummary {
    fn from(s: &rep::ObserverReportSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            observer_did: s.observer_did.to_string(),
            declared_perspective: s.declared_perspective.clone(),
            summary: s.summary.clone(),
            observed_at: s.observed_at.as_second(),
            reported_at: s.reported_at.as_second(),
            responses: s.responses.iter().map(PyReportResponseSummary::from).collect(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "WhistleblowerSummary", get_all)]
#[derive(Clone)]
struct PyWhistleblowerSummary {
    record_cid: String,
    claim: String,
    filed_at: i64,
    source_revealed: bool,
    /// The filing's anonymous source tag (ring-signature key image, hex).
    source_tag: String,
    /// Published proofs that this filing's key image is not the claimant's.
    exculpations: usize,
}

impl From<&rep::WhistleblowerSummary> for PyWhistleblowerSummary {
    fn from(s: &rep::WhistleblowerSummary) -> Self {
        Self {
            record_cid: s.record_cid.to_string(),
            claim: s.claim.clone(),
            filed_at: s.filed_at.as_second(),
            source_tag: s.source_tag.clone(),
            exculpations: s.exculpations,
            source_revealed: s.source_revealed,
        }
    }
}

#[pyclass(skip_from_py_object, name = "ReputationView", get_all)]
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
    injunctions: Vec<PyInjunctionSummary>,
    revoked_capabilities: Vec<String>,
    commons_sanctions: Vec<PyCommonsSanctionSummary>,
    observer_reports: Vec<PyObserverReportSummary>,
    whistleblower_reports_against: Vec<PyWhistleblowerSummary>,
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
            injunctions: v.injunctions.iter().map(PyInjunctionSummary::from).collect(),
            revoked_capabilities: v.revoked_capabilities.clone(),
            commons_sanctions: v
                .commons_sanctions
                .iter()
                .map(PyCommonsSanctionSummary::from)
                .collect(),
            observer_reports: v
                .observer_reports
                .iter()
                .map(PyObserverReportSummary::from)
                .collect(),
            whistleblower_reports_against: v
                .whistleblower_reports_against
                .iter()
                .map(PyWhistleblowerSummary::from)
                .collect(),
        }
    }
}

// ---- Capture-signal analytics bindings -------------------------------------

#[pyclass(skip_from_py_object, name = "SponseeOutcome", get_all)]
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

#[pyclass(skip_from_py_object, name = "SponsorStakeReport", get_all)]
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

#[pyclass(skip_from_py_object, name = "CompositionGraph", get_all)]
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

#[pyclass(skip_from_py_object, name = "DeliberativeCapacity", get_all)]
#[derive(Clone)]
struct PyDeliberativeCapacity {
    compact_label: String,
    /// The derived roll, in DID order.
    members: Vec<String>,
    /// Those inside a probation window, carrying no second-class weight (§2).
    probationary: Vec<String>,
    /// Members that do carry weight.
    standing: usize,
    /// Standing over roll, or `None` for an empty roll (not evaluated).
    standing_share: Option<f64>,
    quorum_threshold: usize,
    /// Whether an ordinary second-class decision is available at all.
    can_act: bool,
    /// Whether one is available at the asymmetric contraction bar.
    can_contract: bool,
    as_of: i64,
}

impl From<&rep::DeliberativeCapacity> for PyDeliberativeCapacity {
    fn from(c: &rep::DeliberativeCapacity) -> Self {
        Self {
            compact_label: c.compact_label.clone(),
            members: c.members.iter().map(|d| d.to_string()).collect(),
            probationary: c.probationary.iter().map(|d| d.to_string()).collect(),
            standing: c.standing(),
            standing_share: c.standing_share(),
            quorum_threshold: c.quorum_threshold,
            can_act: c.can_act(),
            can_contract: c.can_contract(),
            as_of: c.as_of.as_second(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "FreeRiderReport", get_all)]
#[derive(Clone)]
struct PyFreeRiderReport {
    /// (community_did, consumed, contributed, record_count, sealed_count, [usage_cids]).
    /// `consumed` and `contributed` are summed over that community's **public** records
    /// only; a nonzero `sealed_count` means they understate the real draw.
    tallies: Vec<(String, u64, u64, usize, usize, Vec<String>)>,
    total_consumed: u64,
    total_contributed: u64,
    /// (community_did, net_draw) for communities that drew more than they
    /// returned, descending by net draw.
    by_net_draw: Vec<(String, i128)>,
    /// Whether every record here is public, so the totals are the whole account rather
    /// than a floor.
    is_complete: bool,
    /// (community_did, sealed_record_count) for communities that sealed at least one
    /// record. Not an accusation: sealing is permitted and proven within allowance.
    partially_sealed: Vec<(String, usize)>,
    /// The most the sealed records could be hiding: `sealed × allowance`. Each carries a
    /// proof that its draw is within the allowance, so the hidden quantity is bounded
    /// rather than unknown. `None` when the commons states no allowance.
    max_hidden_draw: Option<u64>,
    /// The commons' per-period draw allowance, if it states one.
    allowance: Option<u64>,
}

impl From<&rep::FreeRiderReport> for PyFreeRiderReport {
    fn from(r: &rep::FreeRiderReport) -> Self {
        Self {
            tallies: r
                .tallies
                .iter()
                .map(|t| {
                    (
                        t.community.to_string(),
                        t.consumed,
                        t.contributed,
                        t.record_count,
                        t.sealed_count,
                        t.evidence.iter().map(|c| c.to_string()).collect(),
                    )
                })
                .collect(),
            total_consumed: r.total_consumed(),
            total_contributed: r.total_contributed(),
            by_net_draw: r.by_net_draw(),
            is_complete: r.is_complete(),
            partially_sealed: r.partially_sealed(),
            max_hidden_draw: r.max_hidden_draw(),
            allowance: r.allowance,
        }
    }
}

#[pyclass(skip_from_py_object, name = "ObserverPoolConcentration", get_all)]
#[derive(Clone)]
struct PyObserverPoolConcentration {
    /// (community_did, active_observers) descending — the domination front.
    shares: Vec<(String, usize)>,
    total_active: usize,
    /// (observer_did, nominating_community, term_end) soonest term-end first.
    serving: Vec<(String, String, Option<i64>)>,
    /// Largest single-community active count.
    top_active: usize,
    /// True when one community already holds a majority of the corps (the
    /// condition the pool-share authority rule refuses once past its floor).
    majority_held: bool,
}

#[pyclass(skip_from_py_object, name = "MediatorPoolConcentration", get_all)]
#[derive(Clone)]
struct PyMediatorPoolConcentration {
    /// (community_did, active_mediators) descending — the domination front.
    shares: Vec<(String, usize)>,
    total_active: usize,
    /// (mediator_did, nominating_community, term_end, trained_by) soonest term-end
    /// first.
    serving: Vec<(String, String, Option<i64>, Option<String>)>,
    /// Largest single-community active count.
    top_active: usize,
    /// True when one community already holds a majority of the pool (the condition the
    /// pool-share authority rule refuses once past its floor).
    majority_held: bool,
    /// (trainer_did, active_mediators_trained) descending. Mediators with no recorded
    /// trainer are absent rather than grouped under a blank.
    lineages: Vec<(String, usize)>,
    /// Active mediators whose lineage is recorded — the denominator the lineage
    /// readings are against, so a pool that records nothing reads as unknown rather
    /// than healthy.
    with_lineage: usize,
    /// Largest single-trainer active count.
    top_lineage: usize,
    /// True when one trainer accounts for a majority of the mediators **whose lineage
    /// is recorded** — the incumbent generation holding the pool. `False` when nothing
    /// is recorded, so silence is never read as a finding.
    lineage_majority_held: bool,
}

impl From<&rep::MediatorPoolConcentration> for PyMediatorPoolConcentration {
    fn from(p: &rep::MediatorPoolConcentration) -> Self {
        Self {
            shares: p.shares.iter().map(|s| (s.community.clone(), s.active)).collect(),
            total_active: p.total_active,
            serving: p
                .serving
                .iter()
                .map(|s| {
                    (
                        s.mediator_did.clone(),
                        s.nominating_community.clone(),
                        s.term_end.map(|t| t.as_second()),
                        s.trained_by.clone(),
                    )
                })
                .collect(),
            top_active: p.top_active(),
            majority_held: p.majority_held(),
            lineages: p.lineages.iter().map(|s| (s.community.clone(), s.active)).collect(),
            with_lineage: p.with_lineage,
            top_lineage: p.top_lineage(),
            lineage_majority_held: p.lineage_majority_held(),
        }
    }
}

impl From<&rep::ObserverPoolConcentration> for PyObserverPoolConcentration {
    fn from(p: &rep::ObserverPoolConcentration) -> Self {
        Self {
            shares: p.shares.iter().map(|s| (s.community.clone(), s.active)).collect(),
            total_active: p.total_active,
            serving: p
                .serving
                .iter()
                .map(|s| {
                    (
                        s.observer_did.clone(),
                        s.nominating_community.clone(),
                        s.term_end.map(|t| t.as_second()),
                    )
                })
                .collect(),
            top_active: p.top_active(),
            majority_held: p.majority_held(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "ObserverDashboard", get_all)]
#[derive(Clone)]
struct PyObserverDashboard {
    composition: PyCompositionGraph,
    rejections: PyRejectionPattern,
    /// (subject_did, observer_report_count), descending.
    observer_reports_by_subject: Vec<(String, usize)>,
    /// (accused_did, whistleblower_count), descending. Never the source.
    whistleblowers_by_accused: Vec<(String, usize)>,
    /// (accused_did, distinct_source_count), descending — how many separate insiders
    /// filed, never who. One source filing ten times reads as one.
    distinct_sources_by_accused: Vec<(String, usize)>,
    /// The observer-corps composition (pool-share / rotation view).
    pool: PyObserverPoolConcentration,
    /// The substrate-deference / erosion report (§9 half of the board).
    substrate: PySubstrateDeferenceReport,
    /// The graded erosion alert derived from `substrate` (the crossed-threshold signal).
    erosion: PyErosionAlert,
    /// (community_did, insider_set_size), **ascending** — the thinnest anonymous
    /// channels first. 1 means no channel at all; 2 is a channel in name only.
    anonymity_set_by_community: Vec<(String, usize)>,
}

impl From<&rep::ObserverDashboard> for PyObserverDashboard {
    fn from(d: &rep::ObserverDashboard) -> Self {
        Self {
            composition: PyCompositionGraph::from(&d.composition),
            rejections: PyRejectionPattern::from(&d.rejections),
            observer_reports_by_subject: d.observer_reports_by_subject.clone(),
            whistleblowers_by_accused: d.whistleblowers_by_accused.clone(),
            distinct_sources_by_accused: d.distinct_sources_by_accused.clone(),
            pool: PyObserverPoolConcentration::from(&d.pool),
            substrate: PySubstrateDeferenceReport::from(&d.substrate),
            erosion: PyErosionAlert::from(&d.erosion),
            anonymity_set_by_community: d.anonymity_set_by_community.clone(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "ConstitutionState", get_all)]
#[derive(Clone)]
struct PyConstitutionState {
    ratified: bool,
    /// (key, description) of each in-force compact-incapable matter.
    compact_incapable: Vec<(String, String)>,
    /// (label, start, end, recurrence) of each structured-fast period; `recurrence`
    /// is the cycle in seconds for a repeating fast, else None.
    fast_periods: Vec<(String, i64, i64, Option<i64>)>,
    /// Whether the compact has adopted the functional minimum as the ground of its own
    /// conduct. A separate act from ratification, so a separate flag.
    minimum_adopted: bool,
    /// (key, description, site) of each in-force forbearance. Never a compact-incapable
    /// matter: this list binds the compact, that one names matters belonging to members.
    minimum_content: Vec<(String, String, String)>,
}

impl From<&rep::ConstitutionState> for PyConstitutionState {
    fn from(s: &rep::ConstitutionState) -> Self {
        Self {
            ratified: s.ratified,
            compact_incapable: s
                .compact_incapable
                .iter()
                .map(|m| (m.key.clone(), m.description.clone()))
                .collect(),
            fast_periods: s
                .fast_periods
                .iter()
                .map(|p| (p.label.clone(), p.start.as_second(), p.end.as_second(), p.recurrence))
                .collect(),
            minimum_adopted: s.minimum_adopted,
            minimum_content: s
                .minimum_content
                .iter()
                .map(|f| (f.key.clone(), f.description.clone(), f.site.as_str().to_string()))
                .collect(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "SubstrateDeferenceReport", get_all)]
#[derive(Clone)]
struct PySubstrateDeferenceReport {
    ratified: bool,
    incapable_current: usize,
    incapable_added: usize,
    incapable_removed: usize,
    contraction_detected: bool,
    fast_period_count: usize,
    fast_seconds_total: i64,
    fast_contractions: usize,
    /// Forbearances in the adopted minimum content currently in force.
    minimum_current: usize,
    /// `Broaden` amendments as of the query (the compact widening its own ground).
    minimum_broadened: usize,
    /// `Narrow` amendments as of the query. Folded into `contraction_detected`.
    minimum_narrowed: usize,
    /// (record_cid, kind, at, fast_label) for each deliberative act during a fast.
    fast_deviations: Vec<(String, String, i64, String)>,
}

impl From<&rep::SubstrateDeferenceReport> for PySubstrateDeferenceReport {
    fn from(r: &rep::SubstrateDeferenceReport) -> Self {
        Self {
            ratified: r.ratified,
            incapable_current: r.incapable_current,
            incapable_added: r.incapable_added,
            incapable_removed: r.incapable_removed,
            contraction_detected: r.contraction_detected,
            fast_period_count: r.fast_period_count,
            fast_seconds_total: r.fast_seconds_total,
            fast_contractions: r.fast_contractions,
            minimum_current: r.minimum_current,
            minimum_broadened: r.minimum_broadened,
            minimum_narrowed: r.minimum_narrowed,
            fast_deviations: r
                .fast_deviations
                .iter()
                .map(|d| (d.record_cid.to_string(), d.kind.clone(), d.at.as_second(), d.fast_label.clone()))
                .collect(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "ErosionAlert", get_all)]
#[derive(Clone)]
struct PyErosionAlert {
    /// "None" | "Notice" | "Alert" — escalating with the number of erosion signals.
    severity: String,
    /// One human-readable reason per active erosion signal.
    reasons: Vec<String>,
    list_contraction: bool,
    /// A forbearance has been narrowed out of the compact's own adopted ground — the
    /// signal with no constituency inside the compact to raise it.
    minimum_contraction: bool,
    fast_contraction: bool,
    active_fast_deviations: usize,
    /// (community_did, last_rite_cid, seconds_since_last, cadence_seconds) for each
    /// community whose re-affirmation is overdue, longest-overdue first. The one
    /// signal here that fires on omission rather than on an act.
    overdue_reaffirmations: Vec<(String, String, i64, i64)>,
    /// Communities whose latest rite drew fewer witnesses than their first.
    hollowing_communities: Vec<String>,
    /// `(decisions, first_argument_count, latest_argument_count, first_span_seconds |
    /// None, latest_span_seconds | None, argument_count_declined, span_shortened)` when
    /// the compact's deliberation is thinning, else `None`.
    ///
    /// Read against the compact's own **first** decision, never its recent average: a
    /// deliberation that thins a little each time drags a rolling baseline behind it,
    /// and only an absolute measure sees a drift with nobody driving it.
    thinning_deliberation: Option<ThinningDeliberation>,
    /// (designation_cid, subject, tier, reexamine_after, seconds_overdue, confirmations)
    /// for each designation past its mandated re-examination and still standing,
    /// longest-overdue first. `reexamine_after` is the deadline standing *unanswered*:
    /// the designation's own until a re-examination confirms it, then the date that
    /// confirmation set. `confirmations` counts the renewals, so a designation overdue on
    /// its third is no longer the same reading as one nobody ever examined.
    overdue_reexaminations: Vec<OverdueReexaminationRow>,
    /// (community_did, function, seconds_since, cadence_seconds, last_was_a_null) for
    /// each standing function with no act and no attested null inside the cadence,
    /// longest silence first.
    silent_functions: Vec<SilentFunctionRow>,
    /// (community_did, function, vacant_since, seconds_vacant) for each function a
    /// community once staffed and now has nobody seated in, longest vacancy first. A
    /// function never staffed is absent: declining to open an office is not abandoning
    /// one.
    unstaffed_functions: Vec<(String, String, i64, i64)>,
    /// The compact's enforcement has narrowed onto part of the shared taxonomy
    /// while spreading across subjects, else `None`.
    ///
    /// The first signal here that fires on **intensification** rather than on an
    /// act or an omission. Every other field on this alert is satisfied by a polity
    /// that keeps each form and performs it harder, which is the whole of
    /// substitution: the recited object untouched in the record and the enforced
    /// object quietly narrowed.
    enforcement_convergence: Option<PyEnforcementConvergence>,
    /// The compact's enforcement runs at communities holding reduced standing,
    /// else `None`. Where the narrowed ground landed.
    enforcement_direction: Option<PyEnforcementDirection>,
}

#[pyclass(skip_from_py_object, name = "EnforcementConvergence", get_all)]
#[derive(Clone)]
struct PyEnforcementConvergence {
    /// The enforcing community, or `None` for the compact-wide reading.
    community: Option<String>,
    /// Enforcement acts inside the window.
    volume: usize,
    /// Concrete ground → acts citing it. Grounds never cited are absent.
    per_bearing: BTreeMap<String, usize>,
    /// `Other(_)` ground string → acts citing it.
    per_other: BTreeMap<String, usize>,
    /// Subject community → acts aimed at it.
    per_subject: BTreeMap<String, usize>,
    window_seconds: i64,
    evidence: Vec<String>,
    /// Distinct concrete grounds cited over the whole concrete taxonomy.
    ///
    /// Absolute by construction: the denominator is a fixed enum, never the
    /// compact's own recent behaviour. `Other(_)` is outside it, so a polity filing
    /// everything off-taxonomy scores `0.0` here, which is the correct reading and
    /// the reason no separate off-taxonomy rule exists.
    coverage: f64,
    /// Herfindahl over grounds; `None` when there is nothing to index.
    concentration: Option<f64>,
    /// Herfindahl over subject communities. **The discriminator**: a polity with
    /// one genuinely recurring problem also shows high volume and low coverage, and
    /// what separates it is that legitimate concentration lands on a *target* while
    /// substitution concentrates on a *ground* across many targets.
    target_dispersion: Option<f64>,
    /// Share of acts citing a ground outside the shared taxonomy.
    other_share: Option<f64>,
}

impl From<&rep::EnforcementConvergence> for PyEnforcementConvergence {
    fn from(c: &rep::EnforcementConvergence) -> Self {
        Self {
            community: c.community.clone(),
            volume: c.volume,
            per_bearing: c.per_bearing.clone(),
            per_other: c.per_other.clone(),
            per_subject: c.per_subject.clone(),
            window_seconds: c.window_seconds,
            evidence: c.evidence.iter().map(|c| c.to_string()).collect(),
            coverage: c.coverage(),
            concentration: c.concentration(),
            target_dispersion: c.target_dispersion(),
            other_share: c.other_share(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "EnforcementDirection", get_all)]
#[derive(Clone)]
struct PyEnforcementDirection {
    /// The enforcing community, or `None` for the compact-wide reading.
    community: Option<String>,
    /// Subject in probation, no reporter in probation.
    downward: usize,
    /// A reporter in probation, subject not.
    upward: usize,
    /// Neither in probation, or both. Enforcement between equals.
    level: usize,
    window_seconds: i64,
    /// The downward acts only.
    evidence: Vec<String>,
    total: usize,
    downward_share: Option<f64>,
}

impl From<&rep::EnforcementDirection> for PyEnforcementDirection {
    fn from(d: &rep::EnforcementDirection) -> Self {
        Self {
            community: d.community.clone(),
            downward: d.downward,
            upward: d.upward,
            level: d.level,
            window_seconds: d.window_seconds,
            evidence: d.evidence.iter().map(|c| c.to_string()).collect(),
            total: d.total(),
            downward_share: d.downward_share(),
        }
    }
}

impl From<&rep::ErosionAlert> for PyErosionAlert {
    fn from(a: &rep::ErosionAlert) -> Self {
        Self {
            severity: a.severity.as_str().to_string(),
            reasons: a.reasons.clone(),
            list_contraction: a.list_contraction,
            minimum_contraction: a.minimum_contraction,
            fast_contraction: a.fast_contraction,
            active_fast_deviations: a.active_fast_deviations,
            overdue_reaffirmations: a
                .overdue_reaffirmations
                .iter()
                .map(|o| {
                    (
                        o.community_did.clone(),
                        o.last_rite_cid.to_string(),
                        o.seconds_since_last,
                        o.cadence_seconds,
                    )
                })
                .collect(),
            hollowing_communities: a.hollowing_communities.clone(),
            thinning_deliberation: a.thinning_deliberation.as_ref().map(|t| {
                (
                    t.decisions,
                    t.first_argument_count,
                    t.latest_argument_count,
                    t.first_span_seconds,
                    t.latest_span_seconds,
                    t.argument_count_declined,
                    t.span_shortened,
                )
            }),
            overdue_reexaminations: a
                .overdue_reexaminations
                .iter()
                .map(|o| {
                    (
                        o.designation_cid.to_string(),
                        o.subject.clone(),
                        o.tier.clone(),
                        o.reexamine_after.as_second(),
                        o.seconds_overdue,
                        o.confirmations,
                    )
                })
                .collect(),
            silent_functions: a
                .silent_functions
                .iter()
                .map(|f| {
                    (
                        f.community_did.clone(),
                        f.function.clone(),
                        f.seconds_since,
                        f.cadence_seconds,
                        f.last_was_a_null,
                    )
                })
                .collect(),
            unstaffed_functions: a
                .unstaffed_functions
                .iter()
                .map(|u| {
                    (
                        u.community_did.clone(),
                        u.function.clone(),
                        u.vacant_since.as_second(),
                        u.seconds_vacant,
                    )
                })
                .collect(),
            enforcement_convergence: a
                .enforcement_convergence
                .as_ref()
                .map(PyEnforcementConvergence::from),
            enforcement_direction: a
                .enforcement_direction
                .as_ref()
                .map(PyEnforcementDirection::from),
        }
    }
}

#[pyclass(skip_from_py_object, name = "FeatureConcernProfile", get_all)]
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

#[pyclass(skip_from_py_object, name = "RejectionPattern", get_all)]
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

#[pyclass(skip_from_py_object, name = "CeremonyCostDecay", get_all)]
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
    /// Rolling: the latest gap against the median of the priors.
    latest_gap_widening: Option<bool>,
    /// Pinned: the latest gap against the FIRST one. Stays true after a settled
    /// drift, where the rolling reading goes quiet.
    gap_widening_since_founding: Option<bool>,
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
            gap_widening_since_founding: d.gap_widening_since_founding(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "InterpretivePluralityReport", get_all)]
#[derive(Clone)]
struct PyInterpretivePluralityReport {
    /// (record_cid, lineage_id, interpreter, recorded_at), oldest first.
    interpretations: Vec<(String, String, String, i64)>,
    /// (lineage_id, count).
    lineages: Vec<(String, usize)>,
    /// (interpreter_did, count) descending — the concentration ranking.
    interpreter_concentration: Vec<(String, usize)>,
    distinct_lineages: usize,
    distinct_interpreters: usize,
    plurality_demonstrated: bool,
    /// (interpreter_did, [attester_did, ...]) — distinct third parties who
    /// attested each interpreter's independence (move 1 anti-sockpuppet).
    independent_attesters: Vec<(String, Vec<String>)>,
    distinct_independent_interpreters: usize,
    independent_plurality_demonstrated: bool,
}

impl From<&rep::InterpretivePluralityReport> for PyInterpretivePluralityReport {
    fn from(r: &rep::InterpretivePluralityReport) -> Self {
        Self {
            interpretations: r
                .interpretations
                .iter()
                .map(|p| {
                    (
                        p.record_cid.to_string(),
                        p.lineage_id.clone(),
                        p.interpreter.to_string(),
                        p.recorded_at.as_second(),
                    )
                })
                .collect(),
            lineages: r.lineages.iter().map(|(k, n)| (k.clone(), *n)).collect(),
            interpreter_concentration: r.interpreter_concentration(),
            distinct_lineages: r.distinct_lineages(),
            distinct_interpreters: r.distinct_interpreters(),
            plurality_demonstrated: r.plurality_demonstrated(),
            independent_attesters: r
                .independent_attesters
                .iter()
                .map(|(k, atts)| (k.clone(), atts.iter().cloned().collect()))
                .collect(),
            distinct_independent_interpreters: r.distinct_independent_interpreters(),
            independent_plurality_demonstrated: r.independent_plurality_demonstrated(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "SkepticismReport", get_all)]
#[derive(Clone)]
struct PySkepticismReport {
    /// Oldest first.
    events: Vec<SkepticismEventRow>,
    qualifying_events: usize,
}

impl From<&rep::SkepticismReport> for PySkepticismReport {
    fn from(r: &rep::SkepticismReport) -> Self {
        Self {
            events: r
                .events
                .iter()
                .map(|e| {
                    (
                        e.event_cid.to_string(),
                        e.originator.to_string(),
                        e.reported_at.as_second(),
                        e.qualifies,
                        e.triggers.clone(),
                        e.provenance_complete,
                        e.corroborators.iter().map(|d| d.to_string()).collect(),
                        e.translated_corroborators.iter().map(|d| d.to_string()).collect(),
                        e.undisclosed_interests_found
                            .iter()
                            .map(|(f, i)| (f.to_string(), i.clone()))
                            .collect(),
                        e.aligned_faction.as_ref().map(|d| d.to_string()),
                    )
                })
                .collect(),
            qualifying_events: r.qualifying_events(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "PrincipalValidationReport", get_all)]
#[derive(Clone)]
struct PyPrincipalValidationReport {
    /// (record_cid, witness_community, attested_at), oldest first.
    cross_witnesses: Vec<(String, String, i64)>,
    /// (output_cid, cohort, matter, principal_decision, cohort_majority,
    /// peers_compared, favors_home).
    #[allow(clippy::type_complexity)]
    divergences: Vec<(String, String, String, String, String, usize, bool)>,
    distinct_cross_witnesses: usize,
    self_benefiting_divergences: usize,
    /// The principal's outputs that had a peer cohort to triangulate against — the
    /// denominator for `self_dealing_rate`.
    outputs_triangulated: usize,
    /// Fraction of triangulated outputs that self-dealt (continuous-behavior
    /// signal); `0.0` when nothing was triangulated. Use with a minimum sample and
    /// a rate threshold to judge systematic self-dealing (both caller policy).
    self_dealing_rate: f64,
}

impl From<&rep::PrincipalValidationReport> for PyPrincipalValidationReport {
    fn from(r: &rep::PrincipalValidationReport) -> Self {
        Self {
            cross_witnesses: r
                .cross_witnesses
                .iter()
                .map(|w| {
                    (
                        w.record_cid.to_string(),
                        w.witness_community.to_string(),
                        w.attested_at.as_second(),
                    )
                })
                .collect(),
            divergences: r
                .divergences
                .iter()
                .map(|d| {
                    (
                        d.output_cid.to_string(),
                        d.cohort.clone(),
                        d.matter.clone(),
                        d.principal_decision.clone(),
                        d.cohort_majority.clone(),
                        d.peers_compared,
                        d.favors_home,
                    )
                })
                .collect(),
            distinct_cross_witnesses: r.distinct_cross_witnesses(),
            self_benefiting_divergences: r.self_benefiting_divergences(),
            outputs_triangulated: r.outputs_triangulated,
            self_dealing_rate: r.self_dealing_rate(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "CohortCollusionReport", get_all)]
#[derive(Clone)]
struct PyCohortCollusionReport {
    /// (cohort, matter, cohort_majority, cross_cohort_consensus, cohorts_compared,
    /// self_benefiting), sorted by (matter, cohort).
    #[allow(clippy::type_complexity)]
    divergences: Vec<(String, String, String, String, usize, bool)>,
    self_benefiting_cohort_divergences: usize,
}

impl From<&rep::CohortCollusionReport> for PyCohortCollusionReport {
    fn from(r: &rep::CohortCollusionReport) -> Self {
        Self {
            divergences: r
                .divergences
                .iter()
                .map(|d| {
                    (
                        d.cohort.clone(),
                        d.matter.clone(),
                        d.cohort_majority.clone(),
                        d.cross_cohort_consensus.clone(),
                        d.cohorts_compared,
                        d.self_benefiting,
                    )
                })
                .collect(),
            self_benefiting_cohort_divergences: r.self_benefiting_cohort_divergences(),
        }
    }
}

#[pyclass(skip_from_py_object, name = "WitnessChainReport", get_all)]
#[derive(Clone)]
struct PyWitnessChainReport {
    /// (cid, witness_community, prev|None, attested_at, prev_resolved,
    /// cross_community), oldest first.
    #[allow(clippy::type_complexity)]
    links: Vec<(String, String, Option<String>, i64, bool, bool)>,
    /// Count of cross-community (separable) links the home cannot forge.
    separable_links: usize,
    /// Count of links whose declared `prev` dangles (no matching link).
    broken_links: usize,
    /// At most one root and no forks.
    is_linear: bool,
    /// No dangling `prev` and linear.
    is_intact: bool,
}

impl From<&rep::WitnessChainReport> for PyWitnessChainReport {
    fn from(r: &rep::WitnessChainReport) -> Self {
        Self {
            links: r
                .links
                .iter()
                .map(|l| {
                    (
                        l.cid.to_string(),
                        l.witness_community.to_string(),
                        l.prev.as_ref().map(|c| c.to_string()),
                        l.attested_at.as_second(),
                        l.prev_resolved,
                        l.cross_community,
                    )
                })
                .collect(),
            separable_links: r.separable_links(),
            broken_links: r.broken_links(),
            is_linear: r.is_linear(),
            is_intact: r.is_intact(),
        }
    }
}

// `gil_used = true` states what this module required before the pyo3 0.28 default
// flipped. Until 0.28 a module opted *in* to free-threaded support with
// `gil_used = false`; from 0.28 modules allow it by default and must opt out. The
// bump would therefore have acquired a free-threaded claim silently, and nothing
// here can test it: this sandbox's CPython 3.13.7 is a GIL build
// (`Py_GIL_DISABLED=0`), so a free-threaded interpreter never loads the module.
//
// The claim is not obviously safe either. The seam hands out archive handles, and
// under a free-threaded interpreter two Python threads reach one of those with no
// GIL serializing them. Declaring the status quo keeps the contract the tests
// actually exercise; lifting it is a separate change that needs a free-threaded
// interpreter to verify.
#[pymodule(gil_used = true)]
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
    m.add_class::<PyInjunctionSummary>()?;
    m.add_class::<PyCommonsSanctionSummary>()?;
    m.add_class::<PyFreeRiderReport>()?;
    m.add_class::<PyObserverReportSummary>()?;
    m.add_class::<PyReportResponseSummary>()?;
    m.add_class::<PyWhistleblowerSummary>()?;
    m.add_class::<PyMediatorPoolConcentration>()?;
    m.add_class::<PyObserverPoolConcentration>()?;
    m.add_class::<PyObserverDashboard>()?;
    m.add_class::<PyConstitutionState>()?;
    m.add_class::<PySubstrateDeferenceReport>()?;
    m.add_class::<PyErosionAlert>()?;
    m.add_class::<PyEnforcementConvergence>()?;
    m.add_class::<PyEnforcementDirection>()?;
    m.add_class::<PySponseeOutcome>()?;
    m.add_class::<PySponsorStakeReport>()?;
    m.add_class::<PyCompositionGraph>()?;
    m.add_class::<PyDeliberativeCapacity>()?;
    m.add_class::<PyFeatureConcernProfile>()?;
    m.add_class::<PyRejectionPattern>()?;
    m.add_class::<PyCeremonyCostDecay>()?;
    m.add_class::<PyInterpretivePluralityReport>()?;
    m.add_class::<PySkepticismReport>()?;
    m.add_class::<PyPrincipalValidationReport>()?;
    m.add_class::<PyCohortCollusionReport>()?;
    m.add_class::<PyWitnessChainReport>()?;
    m.add_class::<PyCaptureScenario>()?;
    m.add_function(wrap_pyfunction!(generate_ed25519_keypair, m)?)?;
    m.add_function(wrap_pyfunction!(cid_of_bytes, m)?)?;
    m.add_function(wrap_pyfunction!(now_seconds, m)?)?;
    m.add_function(wrap_pyfunction!(run_two_community_ceremony, m)?)?;
    m.add_function(wrap_pyfunction!(demo_capture_scenario, m)?)?;
    Ok(())
}
