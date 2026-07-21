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

//! Authority enforcement (protocol component §5), a pass distinct from the
//! cryptographic `verify_all`.
//!
//! `verify_all` proves each record is intact and every signature resolves to a
//! real key. It does NOT prove the signer was *authorized* to speak for the
//! community the record is attributed to. `check_authority` closes that:
//!
//! 1. **Principal binding.** Every principal community a record speaks for must
//!    carry a signature from a key authorized for it — the community's own root
//!    key, or an active `SpeakerDelegation` whose `community_did` is that
//!    community.
//! 2. **Scope-appropriateness.** A delegate's `scope` must be allowed for the
//!    record type (a Reciter cannot sign an inter-community refusal). Root is
//!    always allowed for its own principal.
//! 3. **Quorum.** A second-class decision (`DecisionClass::SecondClass`, e.g. a
//!    `DesignationAnathema`) must be backed by at least `quorum_threshold`
//!    distinct principal communities.
//!
//! Revocation and expiry are honored via `SpeakerDelegation::active_at` with the
//! same latest-issued-wins supersession the archive uses for key resolution.

use std::collections::BTreeSet;

use kc_commons::{is_second_class_level, sanction_rank, CommonsSanction, SanctionLevel, UsageRecord};
use kc_conduct::DecisionClass;
use kc_constitution::{AmendmentKind, CompactIncapableAmendment, FastCadenceAmendment};
use kc_core::{Cid, DidKey, KcError, KcResult, Timestamp};
use kc_escalation::{
    dispute_ladder_rank, is_second_class_rung, Designation, DesignationTier, DesignationUrgency,
    InjunctionChallenge, InterimInjunction, LadderTransition, ReEntry, RestitutionCompletion,
};
use kc_identity::DelegationScope;
use kc_mediation::MediationDecision;
use kc_observer::{ObserverReport, ReportResponse, SourceReveal, WhistleblowerReport};

use crate::entry::ArchiveEntry;
use crate::store::{Archive, InMemoryArchive};

/// Default distinct-community quorum for second-class decisions.
pub const DEFAULT_QUORUM_THRESHOLD: usize = 2;

/// Default minimum mediation-panel size (§4). Research: three-person panels
/// drawn from non-party communities (liminal_institutions.md "The mediator
/// pool"). Small compacts may lower this for ad-hoc panels.
pub const DEFAULT_PANEL_MIN: usize = 3;

/// Minimum gap between the two deliberation cycles a normal anathema designation
/// requires (bounded_conflict.md §"Handling the anathema", condition 2: "at least
/// two cycles of consideration separated by a defined waiting period"). A policy
/// constant of the escalation component.
pub const DEFAULT_WAITING_PERIOD_SECONDS: i64 = 7 * 24 * 3600;

/// Minimum sincerity-affirmed witnesses for a re-entry (§5). The re-entry
/// sincerity bar is more demanding than first admission's single internal witness
/// (exit_and_succession.md §"Re-entry after expulsion").
pub const DEFAULT_REENTRY_WITNESS_MIN: usize = 2;

/// The number of failed-then-re-violated restoration cycles that a *normal*
/// anathema designation must cite (bounded_conflict.md condition 1).
const DESIGNATION_CYCLE_FLOOR: usize = 3;

/// The delegate scopes allowed to sign a given record type. Root (a community
/// signing for itself) is always authorized regardless of this set; an empty
/// slice therefore means "root only".
fn allowed_scopes(entry: &ArchiveEntry) -> &'static [DelegationScope] {
    use DelegationScope::*;
    match entry {
        ArchiveEntry::InterCommunityWitness(_) | ArchiveEntry::InterCommunityNonAdmission(_) => {
            &[Envoy, CrossBoundaryWitness]
        }
        ArchiveEntry::SponsorshipVouched(_) => &[Envoy],
        ArchiveEntry::ConductAttested(_) => &[Envoy],
        ArchiveEntry::DisputeFiled(_) => &[Envoy],
        // §7 commons acts an envoy may sign on the community's behalf.
        ArchiveEntry::CommonsRegistered(_)
        | ArchiveEntry::UsageAccounted(_)
        | ArchiveEntry::SanctionApplied(_) => &[Envoy],
        // §9 constitutional acts a community may co-sign via an envoy.
        ArchiveEntry::ConstitutionRatified(_)
        | ArchiveEntry::CompactIncapableAmended(_)
        | ArchiveEntry::FastCadenceAmended(_) => &[Envoy],
        // Move 1: a community's reading of its own substrate; an envoy may sign.
        ArchiveEntry::SubstrateInterpreted(_) => &[Envoy],
        // Move 4: an event report / corroboration a community authors; an envoy
        // may sign on its behalf.
        ArchiveEntry::EventReported(_) | ArchiveEntry::EventCorroborated(_) => &[Envoy],
        // Move 6: a principal witness attestation / output a community authors;
        // an envoy may sign on its behalf.
        ArchiveEntry::PrincipalWitnessed(_) | ArchiveEntry::PrincipalOutputRecorded(_) => &[Envoy],
        // §8. A subject community's response to an observer report; an envoy may
        // sign it. (The other §8 records are authorized by observer/panel rules.)
        ArchiveEntry::ReportResponsePublished(_) => &[Envoy],
        ArchiveEntry::ObserverRegistryUpdated(_)
        | ArchiveEntry::ObserverReportPublished(_)
        | ArchiveEntry::WhistleblowerFiled(_)
        | ArchiveEntry::SourceRevealed(_) => &[],
        // §5 escalation acts an envoy may sign on the community's behalf.
        ArchiveEntry::LadderAdvanced(_)
        | ArchiveEntry::DesignationDecided(_)
        | ArchiveEntry::RestitutionObligated(_)
        | ArchiveEntry::ReEntryCompleted(_)
        | ArchiveEntry::SubstantiveReEngagementInvoked(_)
        | ArchiveEntry::InjunctionConfirmed(_) => &[Envoy],
        ArchiveEntry::IntraCommunityRite(_)
        | ArchiveEntry::SpeakerDelegated(_)
        | ArchiveEntry::WitnessRegistryUpdated(_)
        | ArchiveEntry::MediatorRegistryUpdated(_)
        | ArchiveEntry::CommunityIdentityPublished(_)
        // These are authorized by a panel rule, not principal binding; their
        // allowed-scope sets are unused.
        | ArchiveEntry::MediationDecided(_)
        | ArchiveEntry::RestitutionVerified(_)
        | ArchiveEntry::InjunctionIssued(_)
        | ArchiveEntry::InjunctionChallenged(_) => &[],
    }
}

struct RecordAuthority {
    principals: Vec<DidKey>,
    at: Timestamp,
    second_class: bool,
}

/// The principal communities a record speaks for, the instant its signatures are
/// resolved, and whether it is a second-class decision. `None` for records that
/// carry no delegable authority (a self-signed community identity).
fn record_authority(entry: &ArchiveEntry) -> Option<RecordAuthority> {
    let ra = |principals, at, second_class| RecordAuthority { principals, at, second_class };
    match entry {
        ArchiveEntry::CommunityIdentityPublished(_) => None,
        ArchiveEntry::SpeakerDelegated(s) => {
            Some(ra(vec![s.payload.community_did.clone()], s.payload.issued_at, false))
        }
        ArchiveEntry::WitnessRegistryUpdated(s) => {
            Some(ra(vec![s.payload.community_did.clone()], s.payload.issued_at, false))
        }
        ArchiveEntry::IntraCommunityRite(s) => Some(ra(
            vec![s.payload.recitation.payload.community_did.clone()],
            s.payload.sealed_at,
            false,
        )),
        ArchiveEntry::InterCommunityWitness(s) => {
            Some(ra(s.payload.participants.clone(), s.payload.witnessed_at, false))
        }
        ArchiveEntry::InterCommunityNonAdmission(s) => {
            Some(ra(s.payload.participants.clone(), s.payload.decided_at, false))
        }
        ArchiveEntry::SponsorshipVouched(s) => {
            Some(ra(vec![s.payload.sponsor_community.clone()], s.payload.sponsored_at, false))
        }
        ArchiveEntry::ConductAttested(s) => Some(ra(
            s.payload.reporters.clone(),
            s.payload.recorded_at,
            matches!(s.payload.decision_class, DecisionClass::SecondClass),
        )),
        ArchiveEntry::MediatorRegistryUpdated(s) => {
            Some(ra(vec![s.payload.nominating_community.clone()], s.payload.issued_at, false))
        }
        ArchiveEntry::DisputeFiled(s) => {
            Some(ra(vec![s.payload.complainant.clone()], s.payload.filed_at, false))
        }
        // MediationDecided is authorized by the panel rule (see authorize_decision).
        ArchiveEntry::MediationDecided(_) => None,
        // §5. A second-class ladder rung (collective sanctions, expulsion) gets
        // the distinct-community quorum for free via the `second_class` flag.
        ArchiveEntry::LadderAdvanced(s) => Some(ra(
            s.payload.initiators.clone(),
            s.payload.transitioned_at,
            is_second_class_rung(s.payload.to_rung),
        )),
        ArchiveEntry::DesignationDecided(s) => {
            Some(ra(s.payload.deliberators.clone(), s.payload.designated_at, true))
        }
        ArchiveEntry::RestitutionObligated(s) => Some(ra(
            s.payload.imposed_by.clone(),
            s.payload.imposed_at,
            matches!(s.payload.decision_class, DecisionClass::SecondClass),
        )),
        // RestitutionVerified is authorized by the verifier-panel rule.
        ArchiveEntry::RestitutionVerified(_) => None,
        ArchiveEntry::ReEntryCompleted(s) => {
            Some(ra(s.payload.deliberators.clone(), s.payload.completed_at, true))
        }
        ArchiveEntry::SubstantiveReEngagementInvoked(s) => Some(ra(
            s.payload.invoked_by.clone(),
            s.payload.invoked_at,
            matches!(s.payload.decision_class, DecisionClass::SecondClass),
        )),
        // §9. Injunction confirmation is the full compact speaking: second-class,
        // so it gets the distinct-community quorum for free.
        ArchiveEntry::InjunctionConfirmed(s) => {
            Some(ra(s.payload.confirmers.clone(), s.payload.confirmed_at, true))
        }
        // Issuance and challenge are authorized by the emergency-panel rule.
        ArchiveEntry::InjunctionIssued(_) | ArchiveEntry::InjunctionChallenged(_) => None,
        // §7. A commons registration is a compact-level act: second-class, so the
        // registering communities get the distinct-community quorum for free.
        ArchiveEntry::CommonsRegistered(s) => {
            Some(ra(s.payload.registered_by.clone(), s.payload.registered_at, true))
        }
        // A usage record is the reporting community speaking for itself: first-class.
        ArchiveEntry::UsageAccounted(s) => {
            Some(ra(vec![s.payload.community.clone()], s.payload.recorded_at, false))
        }
        // A commons sanction's heavier levels (restriction, exclusion) are
        // second-class; the flag is set from the record's own decision class and
        // cross-checked against `to_level` in the structural rule.
        ArchiveEntry::SanctionApplied(s) => Some(ra(
            s.payload.imposers.clone(),
            s.payload.decided_at,
            matches!(s.payload.decision_class, DecisionClass::SecondClass),
        )),
        // §8. An observer registry is the nominating community speaking for
        // itself; a report response is the subject community speaking. Both are
        // first-class principal-bound acts.
        ArchiveEntry::ObserverRegistryUpdated(s) => {
            Some(ra(vec![s.payload.nominating_community.clone()], s.payload.issued_at, false))
        }
        ArchiveEntry::ReportResponsePublished(s) => {
            Some(ra(vec![s.payload.responding_community.clone()], s.payload.responded_at, false))
        }
        // Observer reports and whistleblower records are authorized by the
        // observer / reception-panel rules, not principal binding.
        ArchiveEntry::ObserverReportPublished(_)
        | ArchiveEntry::WhistleblowerFiled(_)
        | ArchiveEntry::SourceRevealed(_) => None,
        // §9. Genesis ratification and constitutional amendments are compact-level
        // second-class acts: the ratifiers / deciders get the distinct-community
        // quorum for free; the higher contraction bar is checked structurally.
        ArchiveEntry::ConstitutionRatified(s) => {
            Some(ra(s.payload.ratifiers.clone(), s.payload.genesis_at, true))
        }
        ArchiveEntry::CompactIncapableAmended(s) => {
            Some(ra(s.payload.deciders.clone(), s.payload.decided_at, true))
        }
        ArchiveEntry::FastCadenceAmended(s) => {
            Some(ra(s.payload.deciders.clone(), s.payload.decided_at, true))
        }
        // Move 1. A substrate interpretation is the community speaking for
        // itself: first-class, principal-bound. Interpretive plurality is a
        // detector signal, not an authority refusal.
        ArchiveEntry::SubstrateInterpreted(s) => {
            Some(ra(vec![s.payload.community_did.clone()], s.payload.recorded_at, false))
        }
        // Move 4. An event report is the originating community speaking; a
        // corroboration is the corroborating community speaking. Both are
        // first-class principal-bound acts; corroboration sufficiency is a
        // detector signal, not an authority refusal.
        ArchiveEntry::EventReported(s) => {
            Some(ra(vec![s.payload.originator.clone()], s.payload.reported_at, false))
        }
        ArchiveEntry::EventCorroborated(s) => Some(ra(
            vec![s.payload.corroborating_community.clone()],
            s.payload.corroborated_at,
            false,
        )),
        // Move 6. A principal witness attestation is the attesting community
        // speaking for itself (cross-community independence is a detector
        // filter, not an authority refusal); a principal output is the home
        // community recording its principal's output. Both first-class.
        ArchiveEntry::PrincipalWitnessed(s) => Some(ra(
            vec![s.payload.witness_community.clone()],
            s.payload.attested_at,
            false,
        )),
        ArchiveEntry::PrincipalOutputRecorded(s) => {
            Some(ra(vec![s.payload.principal_home.clone()], s.payload.produced_at, false))
        }
    }
}

/// The `signer_did`s on a record's envelope.
fn signer_dids(entry: &ArchiveEntry) -> Vec<&str> {
    fn dids<T>(s: &kc_core::Signed<T>) -> Vec<&str> {
        s.signatures.iter().map(|e| e.signer_did.as_str()).collect()
    }
    match entry {
        ArchiveEntry::CommunityIdentityPublished(s) => dids(s),
        ArchiveEntry::SpeakerDelegated(s) => dids(s),
        ArchiveEntry::WitnessRegistryUpdated(s) => dids(s),
        ArchiveEntry::IntraCommunityRite(s) => dids(s),
        ArchiveEntry::InterCommunityWitness(s) => dids(s),
        ArchiveEntry::InterCommunityNonAdmission(s) => dids(s),
        ArchiveEntry::SponsorshipVouched(s) => dids(s),
        ArchiveEntry::ConductAttested(s) => dids(s),
        ArchiveEntry::MediatorRegistryUpdated(s) => dids(s),
        ArchiveEntry::DisputeFiled(s) => dids(s),
        ArchiveEntry::MediationDecided(s) => dids(s),
        ArchiveEntry::LadderAdvanced(s) => dids(s),
        ArchiveEntry::DesignationDecided(s) => dids(s),
        ArchiveEntry::RestitutionObligated(s) => dids(s),
        ArchiveEntry::RestitutionVerified(s) => dids(s),
        ArchiveEntry::ReEntryCompleted(s) => dids(s),
        ArchiveEntry::SubstantiveReEngagementInvoked(s) => dids(s),
        ArchiveEntry::InjunctionIssued(s) => dids(s),
        ArchiveEntry::InjunctionConfirmed(s) => dids(s),
        ArchiveEntry::InjunctionChallenged(s) => dids(s),
        ArchiveEntry::CommonsRegistered(s) => dids(s),
        ArchiveEntry::UsageAccounted(s) => dids(s),
        ArchiveEntry::SanctionApplied(s) => dids(s),
        ArchiveEntry::ObserverRegistryUpdated(s) => dids(s),
        ArchiveEntry::ObserverReportPublished(s) => dids(s),
        ArchiveEntry::ReportResponsePublished(s) => dids(s),
        ArchiveEntry::WhistleblowerFiled(s) => dids(s),
        ArchiveEntry::SourceRevealed(s) => dids(s),
        ArchiveEntry::ConstitutionRatified(s) => dids(s),
        ArchiveEntry::CompactIncapableAmended(s) => dids(s),
        ArchiveEntry::FastCadenceAmended(s) => dids(s),
        ArchiveEntry::SubstrateInterpreted(s) => dids(s),
        ArchiveEntry::EventReported(s) => dids(s),
        ArchiveEntry::EventCorroborated(s) => dids(s),
        ArchiveEntry::PrincipalWitnessed(s) => dids(s),
        ArchiveEntry::PrincipalOutputRecorded(s) => dids(s),
    }
}

/// The nominating community of `mediator_did` if it is an active registered
/// mediator at `at`, else `None`.
fn mediator_home(
    archive: &InMemoryArchive,
    mediator_did: &str,
    at: Timestamp,
) -> Option<DidKey> {
    for (_, e) in archive.iter() {
        if let ArchiveEntry::MediatorRegistryUpdated(s) = e {
            for entry in &s.payload.entries {
                if entry.mediator_did.as_str() == mediator_did && entry.active_at(at) {
                    return Some(s.payload.nominating_community.clone());
                }
            }
        }
    }
    None
}

/// The §4 panel rule for a mediation decision: every co-signer must be an active
/// registered mediator; the panel must span at least `panel_min` of them; and no
/// panelist may come from a party community (non-overlap).
fn authorize_decision(
    archive: &InMemoryArchive,
    decision: &MediationDecision,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let dispute = match archive.get(&decision.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "mediation decision references {} which is not a filed dispute",
                decision.dispute_cid
            )))
        }
    };
    let at = decision.decided_at;
    let mut panel: BTreeSet<String> = BTreeSet::new();
    for signer in signers {
        match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "panelist {signer} is not an active registered mediator"
                )))
            }
            Some(home) => {
                if home == dispute.complainant || home == dispute.respondent {
                    return Err(KcError::Authority(format!(
                        "panelist {signer} is from a party community (non-overlap violated)"
                    )));
                }
                panel.insert((*signer).to_string());
            }
        }
    }
    if panel.len() < panel_min {
        return Err(KcError::Authority(format!(
            "mediation panel has {} distinct mediators, need >= {}",
            panel.len(),
            panel_min
        )));
    }
    Ok(())
}

/// The dispute's rung as established by the transitions that precede
/// `self_cid` in the append-only log: the `to_rung` of the last `LadderAdvanced`
/// for this dispute appearing before `self_cid` in insertion order, else the rung
/// it was filed at. (Insertion order — not timestamp — is authoritative here, so
/// authorizing an earlier transition never "sees" a later one.)
fn current_rung(
    archive: &InMemoryArchive,
    dispute_cid: &Cid,
    self_cid: &Cid,
    filed_rung: kc_conduct::LadderRung,
) -> kc_conduct::LadderRung {
    let self_key = self_cid.to_string();
    let mut cur = filed_rung;
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::LadderAdvanced(s) = e {
            if &s.payload.dispute_cid == dispute_cid {
                cur = s.payload.to_rung;
            }
        }
    }
    cur
}

/// True if a mediation decision has closed the dispute (upheld / dismissed /
/// settled). A `Referred` outcome moves it onward and does not close it.
fn dispute_is_closed(archive: &InMemoryArchive, dispute_cid: &Cid) -> bool {
    use kc_mediation::DecisionOutcome::*;
    archive.iter().any(|(_, e)| match e {
        ArchiveEntry::MediationDecided(s) => {
            &s.payload.dispute_cid == dispute_cid
                && matches!(s.payload.outcome, Upheld | Dismissed | Settled)
        }
        _ => false,
    })
}

/// §5 legal-transition rule: a ladder transition must reference a filed dispute,
/// leave that dispute's current rung, and either escalate exactly one step or
/// de-escalate to any lower rung; a second-class rung (collective sanctions,
/// expulsion) must be labeled second-class and moved by at least two distinct
/// communities (non-unilateral). Principal binding + the distinct-community
/// quorum are already enforced generically before this runs.
fn authorize_transition(
    archive: &InMemoryArchive,
    transition: &LadderTransition,
    self_cid: &Cid,
    quorum_threshold: usize,
) -> KcResult<()> {
    let dispute = match archive.get(&transition.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "ladder transition references {} which is not a filed dispute",
                transition.dispute_cid
            )))
        }
    };

    // SubstantiveReEngagement is not a dispute rung; reject it on the ladder.
    let from_rank = dispute_ladder_rank(transition.from_rung).ok_or_else(|| {
        KcError::Authority(format!("{:?} is not a dispute-ladder rung", transition.from_rung))
    })?;
    let to_rank = dispute_ladder_rank(transition.to_rung).ok_or_else(|| {
        KcError::Authority(format!("{:?} is not a dispute-ladder rung", transition.to_rung))
    })?;

    let current = current_rung(archive, &transition.dispute_cid, self_cid, dispute.rung);
    if transition.from_rung != current {
        return Err(KcError::Authority(format!(
            "ladder transition leaves {:?} but the dispute is at {:?}",
            transition.from_rung, current
        )));
    }

    let legal = to_rank == from_rank + 1 || to_rank < from_rank;
    if !legal {
        return Err(KcError::Authority(format!(
            "illegal ladder move {:?} -> {:?} (escalate one step or de-escalate)",
            transition.from_rung, transition.to_rung
        )));
    }

    if dispute_is_closed(archive, &transition.dispute_cid) {
        return Err(KcError::Authority(format!(
            "dispute {} is closed by a mediation decision",
            transition.dispute_cid
        )));
    }

    if is_second_class_rung(transition.to_rung) {
        if !matches!(transition.decision_class, DecisionClass::SecondClass) {
            return Err(KcError::Authority(format!(
                "{:?} is a second-class rung but the transition is labeled first-class",
                transition.to_rung
            )));
        }
        let distinct: BTreeSet<&str> = transition.initiators.iter().map(|d| d.as_str()).collect();
        let floor = quorum_threshold.max(2);
        if distinct.len() < floor {
            return Err(KcError::Authority(format!(
                "{:?} needs >= {} distinct initiating communities, got {}",
                transition.to_rung,
                floor,
                distinct.len()
            )));
        }
    }
    Ok(())
}

/// §5 four-condition anathema test (bounded_conflict.md §"Handling the anathema").
/// The distinct-community quorum (condition 2's "not unilateral, not simple
/// majority") is enforced generically via the second-class flag before this runs.
fn authorize_designation(
    archive: &InMemoryArchive,
    d: &Designation,
    quorum_threshold: usize,
) -> KcResult<()> {
    // Condition 4: reversal conditions present iff the tier is Anathema.
    match d.tier {
        DesignationTier::Anathema => {
            if d.reversal_conditions.as_ref().map_or(true, |s| s.trim().is_empty()) {
                return Err(KcError::Authority(
                    "anathema designation must state reversal conditions (condition 4)".into(),
                ));
            }
        }
        DesignationTier::HostisHumaniGeneris => {
            if d.reversal_conditions.is_some() {
                return Err(KcError::Authority(
                    "hostis humani generis has no reversal path (condition 4)".into(),
                ));
            }
        }
    }

    // Condition 3: clear stated criteria citing specific acts.
    if d.criteria.trim().is_empty() || d.cited_acts.is_empty() {
        return Err(KcError::Authority(
            "designation must state criteria and cite specific acts (condition 3)".into(),
        ));
    }

    // Condition 2: compact-wide deliberation.
    match d.urgency {
        DesignationUrgency::Normal => {
            if d.deliberation_cycles.len() < 2 {
                return Err(KcError::Authority(
                    "normal designation needs >= 2 deliberation cycles (condition 2)".into(),
                ));
            }
            if d.deliberation_cycles.iter().any(|c| c.representatives.is_empty()) {
                return Err(KcError::Authority(
                    "each deliberation cycle needs named representatives (condition 2)".into(),
                ));
            }
            for pair in d.deliberation_cycles.windows(2) {
                let gap = pair[1].opened_at.as_second() - pair[0].concluded_at.as_second();
                if gap < DEFAULT_WAITING_PERIOD_SECONDS {
                    return Err(KcError::Authority(
                        "deliberation cycles must be separated by the waiting period (condition 2)"
                            .into(),
                    ));
                }
            }
        }
        DesignationUrgency::Expedited => {
            if matches!(d.tier, DesignationTier::HostisHumaniGeneris) {
                return Err(KcError::Authority(
                    "the hostis humani generis tier may not be expedited".into(),
                ));
            }
            if d.deliberation_cycles.is_empty()
                || d.deliberation_cycles.iter().any(|c| c.representatives.is_empty())
            {
                return Err(KcError::Authority(
                    "expedited designation needs a deliberation cycle with named reps".into(),
                ));
            }
            if d.reexamine_after.is_none() {
                return Err(KcError::Authority(
                    "expedited designation must set a re-examination deadline".into(),
                ));
            }
            // A higher immediate quorum stands in for the collapsed cadence.
            let distinct: BTreeSet<&str> = d.deliberators.iter().map(|x| x.as_str()).collect();
            if distinct.len() < quorum_threshold + 1 {
                return Err(KcError::Authority(format!(
                    "expedited designation needs >= {} distinct communities, got {}",
                    quorum_threshold + 1,
                    distinct.len()
                )));
            }
        }
    }

    // Condition 1: sustained bad faith — >= 3 completed-then-re-violated cycles.
    // Relaxed only for the expedited (emergency) path, whose safeguards are the
    // higher immediate quorum and the mandatory re-examination above.
    if matches!(d.urgency, DesignationUrgency::Normal)
        && !sustained_bad_faith(archive, &d.subject_community, &d.failed_restoration_cycles)
    {
        return Err(KcError::Authority(format!(
            "designation lacks >= {} completed-then-re-violated restoration cycles (condition 1)",
            DESIGNATION_CYCLE_FLOOR
        )));
    }

    // Anti-weaponization safeguard #1 (liminal_institutions.md): a community that
    // supplied a mediator to an in-force emergency injunction against this subject
    // may not also deliberate the permanent designation.
    let injunction_panel =
        active_injunction_panel_communities(archive, &d.subject_community, d.designated_at);
    for deliberator in &d.deliberators {
        if injunction_panel.contains(deliberator.as_str()) {
            return Err(KcError::Authority(format!(
                "deliberator {} sat on an active injunction panel against the subject \
                 (safeguard #1: issuing panel barred from the designation panel)",
                deliberator.as_str()
            )));
        }
    }
    Ok(())
}

/// Number of cited restitution cycles that were satisfied and then re-violated
/// by the subject; true if it meets the condition-1 floor. Mirrors
/// `authorize_decision`'s cross-archive reads. Note (design limit): the
/// "each more serious than the last" gradation in the research is not machine-
/// checked here — only the satisfied-then-re-violated chain is.
fn sustained_bad_faith(archive: &InMemoryArchive, subject: &DidKey, cited: &[Cid]) -> bool {
    let mut cycles = 0usize;
    for cid in cited {
        let completion = match archive.get(cid) {
            Some(ArchiveEntry::RestitutionVerified(s)) => &s.payload,
            _ => continue,
        };
        if !completion.satisfied {
            continue;
        }
        let obligation = match archive.get(&completion.obligation_cid) {
            Some(ArchiveEntry::RestitutionObligated(s)) => &s.payload,
            _ => continue,
        };
        if &obligation.subject_community != subject {
            continue;
        }
        let reviolated = archive.iter().any(|(_, e)| match e {
            ArchiveEntry::ConductAttested(s) => {
                &s.payload.subject_community == subject
                    && s.payload.kind.is_adverse()
                    && s.payload.recorded_at > completion.verified_at
            }
            _ => false,
        });
        if reviolated {
            cycles += 1;
        }
    }
    cycles >= DESIGNATION_CYCLE_FLOOR
}

/// §5 verifier-panel rule for a restitution completion: every co-signer is an
/// active registered mediator, none from the obligation's subject or beneficiary
/// community (non-overlap), and the panel spans at least `panel_min`.
fn authorize_restitution_completion(
    archive: &InMemoryArchive,
    completion: &RestitutionCompletion,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let obligation = match archive.get(&completion.obligation_cid) {
        Some(ArchiveEntry::RestitutionObligated(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "restitution completion references {} which is not an obligation",
                completion.obligation_cid
            )))
        }
    };
    let at = completion.verified_at;
    let mut panel: BTreeSet<String> = BTreeSet::new();
    for signer in signers {
        match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "restitution verifier {signer} is not an active registered mediator"
                )))
            }
            Some(home) => {
                if home == obligation.subject_community || home == obligation.beneficiary {
                    return Err(KcError::Authority(format!(
                        "restitution verifier {signer} is from a party community (non-overlap violated)"
                    )));
                }
                panel.insert((*signer).to_string());
            }
        }
    }
    if panel.len() < panel_min {
        return Err(KcError::Authority(format!(
            "restitution verifier panel has {} distinct mediators, need >= {}",
            panel.len(),
            panel_min
        )));
    }
    Ok(())
}

/// §5 re-entry rule: the petition answers an `Anathema` designation (never
/// Hostis), carries the petitioner's own complete five-movement recitation, a
/// non-empty conduct accounting and public acknowledgement, and a quorum of
/// sincerity-affirmed witnesses.
fn authorize_reentry(archive: &InMemoryArchive, r: &ReEntry) -> KcResult<()> {
    let designation = match archive.get(&r.designation_cid) {
        Some(ArchiveEntry::DesignationDecided(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "re-entry references {} which is not a designation",
                r.designation_cid
            )))
        }
    };
    if matches!(designation.tier, DesignationTier::HostisHumaniGeneris) {
        return Err(KcError::Authority(
            "the hostis humani generis tier has no re-entry path".into(),
        ));
    }
    if r.recitation.payload.community_did != r.petitioner {
        return Err(KcError::Authority("re-entry recitation is not the petitioner's".into()));
    }
    if !r.recitation.payload.movements_all_present() {
        return Err(KcError::Authority("re-entry recitation is missing a movement".into()));
    }
    if r.conduct_accounting.trim().is_empty() || r.public_acknowledgement.trim().is_empty() {
        return Err(KcError::Authority(
            "re-entry must account for the conduct and publicly acknowledge it".into(),
        ));
    }
    let affirmed = r.witnesses.iter().filter(|w| w.payload.sincerity_affirmed).count();
    if affirmed < DEFAULT_REENTRY_WITNESS_MIN {
        return Err(KcError::Authority(format!(
            "re-entry needs >= {} sincerity-affirmed witnesses, got {}",
            DEFAULT_REENTRY_WITNESS_MIN, affirmed
        )));
    }
    Ok(())
}

/// §7 commons registration rule: the boundary must be well-formed — a non-empty
/// appropriator set (Ostrom principle 1), and every registering community must
/// itself be an appropriator (you cannot register a commons you have no standing
/// in). The distinct-community quorum is enforced generically via the
/// second-class flag before this runs.
fn authorize_commons_registration(reg: &kc_commons::CommonsRegistration) -> KcResult<()> {
    if reg.appropriators.is_empty() {
        return Err(KcError::Authority(
            "commons registration must name at least one appropriator (Ostrom principle 1)".into(),
        ));
    }
    if reg.registered_by.is_empty() {
        return Err(KcError::Authority(
            "commons registration must name its registering communities".into(),
        ));
    }
    let appropriators: BTreeSet<&str> = reg.appropriators.iter().map(|d| d.as_str()).collect();
    for r in &reg.registered_by {
        if !appropriators.contains(r.as_str()) {
            return Err(KcError::Authority(format!(
                "registering community {} is not an appropriator of the commons",
                r.as_str()
            )));
        }
    }
    Ok(())
}

/// The appropriators of the commons registered at `commons_cid`, or an error if
/// it does not resolve to a `CommonsRegistered` entry.
fn commons_appropriators<'a>(
    archive: &'a InMemoryArchive,
    commons_cid: &Cid,
    context: &str,
) -> KcResult<BTreeSet<&'a str>> {
    match archive.get(commons_cid) {
        Some(ArchiveEntry::CommonsRegistered(s)) => {
            Ok(s.payload.appropriators.iter().map(|d| d.as_str()).collect())
        }
        _ => Err(KcError::Authority(format!(
            "{context} references {commons_cid} which is not a registered commons"
        ))),
    }
}

/// §7 usage rule: a usage record must report on a registered commons, and the
/// reporting community must be one of that commons' appropriators (a usage claim
/// from a non-appropriator is meaningless). Principal binding is already enforced
/// generically before this runs.
fn authorize_usage(archive: &InMemoryArchive, u: &UsageRecord) -> KcResult<()> {
    let appropriators = commons_appropriators(archive, &u.commons_cid, "usage record")?;
    if !appropriators.contains(u.community.as_str()) {
        return Err(KcError::Authority(format!(
            "usage community {} is not an appropriator of the commons",
            u.community.as_str()
        )));
    }
    Ok(())
}

/// The current sanction level of a (commons, subject) pair as established by the
/// sanctions preceding `self_cid` in the append-only log: the `to_level` of the
/// last `SanctionApplied` for this pair appearing before `self_cid` in insertion
/// order, else `None` (the resting state). Mirrors `current_rung`; insertion
/// order — not timestamp — is authoritative, so authorizing an earlier sanction
/// never "sees" a later one.
fn current_sanction_level(
    archive: &InMemoryArchive,
    commons_cid: &Cid,
    subject: &DidKey,
    self_cid: &Cid,
) -> SanctionLevel {
    let self_key = self_cid.to_string();
    let mut cur = SanctionLevel::None;
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::SanctionApplied(s) = e {
            if &s.payload.commons_cid == commons_cid && &s.payload.subject_community == subject {
                cur = s.payload.to_level;
            }
        }
    }
    cur
}

/// §7 graduated-sanction rule (Ostrom principle 5): a sanction must reference a
/// registered commons, name a subject that is an appropriator of it, leave that
/// (commons, subject) pair's current level, and either escalate exactly one step
/// or de-escalate to any lower level (the restoration path). A second-class level
/// (restriction, expulsion from the commons) must be labeled second-class and
/// moved by at least two distinct communities (non-unilateral,
/// commons_federation.md §"Community-applied sanctions"). Principal binding + the
/// distinct-community quorum are already enforced generically before this runs.
fn authorize_commons_sanction(
    archive: &InMemoryArchive,
    s: &CommonsSanction,
    self_cid: &Cid,
    quorum_threshold: usize,
) -> KcResult<()> {
    let appropriators = commons_appropriators(archive, &s.commons_cid, "commons sanction")?;
    if !appropriators.contains(s.subject_community.as_str()) {
        return Err(KcError::Authority(format!(
            "sanction subject {} is not an appropriator of the commons",
            s.subject_community.as_str()
        )));
    }

    let current = current_sanction_level(archive, &s.commons_cid, &s.subject_community, self_cid);
    if s.from_level != current {
        return Err(KcError::Authority(format!(
            "commons sanction leaves {:?} but the pair is at {:?}",
            s.from_level, current
        )));
    }

    let from_rank = sanction_rank(s.from_level);
    let to_rank = sanction_rank(s.to_level);
    let legal = to_rank == from_rank + 1 || to_rank < from_rank;
    if !legal {
        return Err(KcError::Authority(format!(
            "illegal commons-sanction move {:?} -> {:?} (escalate one step or de-escalate)",
            s.from_level, s.to_level
        )));
    }

    if is_second_class_level(s.to_level) {
        if !matches!(s.decision_class, DecisionClass::SecondClass) {
            return Err(KcError::Authority(format!(
                "{:?} is a second-class sanction but the record is labeled first-class",
                s.to_level
            )));
        }
        let distinct: BTreeSet<&str> = s.imposers.iter().map(|d| d.as_str()).collect();
        let floor = quorum_threshold.max(2);
        if distinct.len() < floor {
            return Err(KcError::Authority(format!(
                "{:?} needs >= {} distinct imposing communities, got {}",
                s.to_level,
                floor,
                distinct.len()
            )));
        }
    } else if matches!(s.decision_class, DecisionClass::SecondClass) {
        return Err(KcError::Authority(format!(
            "{:?} is a first-class sanction but the record is labeled second-class",
            s.to_level
        )));
    }
    Ok(())
}

/// The nominating community of `observer_did` if it is an active registered
/// observer at `at`, else `None`. Mirrors `mediator_home` (§8).
fn observer_home(
    archive: &InMemoryArchive,
    observer_did: &str,
    at: Timestamp,
) -> Option<DidKey> {
    for (_, e) in archive.iter() {
        if let ArchiveEntry::ObserverRegistryUpdated(s) = e {
            for entry in &s.payload.entries {
                if entry.observer_did.as_str() == observer_did && entry.active_at(at) {
                    return Some(s.payload.nominating_community.clone());
                }
            }
        }
    }
    None
}

/// §8 observer-report rule: the report must be signed by its named observer, who
/// must be an active registered observer whose home community is **not** the
/// subject (cross-community journalism, non-overlap — an observer does not report
/// on its own community).
fn authorize_observer_report(
    archive: &InMemoryArchive,
    report: &ObserverReport,
    signers: &[&str],
) -> KcResult<()> {
    if !signers.iter().any(|s| *s == report.observer_did.as_str()) {
        return Err(KcError::Authority(
            "observer report must be signed by its named observer".into(),
        ));
    }
    match observer_home(archive, report.observer_did.as_str(), report.reported_at) {
        None => Err(KcError::Authority(format!(
            "reporter {} is not an active registered observer",
            report.observer_did.as_str()
        ))),
        Some(home) => {
            if home == report.subject_community {
                Err(KcError::Authority(
                    "an observer may not report on its own community (non-overlap violated)".into(),
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// §8 report-response rule: only the report's subject community may respond, and
/// the referenced record must be an observer report. Principal binding (that the
/// responder actually signed) is enforced generically before this runs.
fn authorize_report_response(archive: &InMemoryArchive, resp: &ReportResponse) -> KcResult<()> {
    let report = match archive.get(&resp.report_cid) {
        Some(ArchiveEntry::ObserverReportPublished(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "report response references {} which is not an observer report",
                resp.report_cid
            )))
        }
    };
    if resp.responding_community != report.subject_community {
        return Err(KcError::Authority(
            "only the observer report's subject community may respond".into(),
        ));
    }
    Ok(())
}

/// §8 mediator reception-panel rule shared by whistleblower filing and source
/// reveal: every co-signer is an active registered mediator, none from
/// `accused` (non-overlap), and the panel spans at least `panel_min` distinct
/// mediators.
fn authorize_reception_panel(
    archive: &InMemoryArchive,
    accused: &DidKey,
    signers: &[&str],
    at: Timestamp,
    panel_min: usize,
    what: &str,
) -> KcResult<()> {
    let mut panel: BTreeSet<String> = BTreeSet::new();
    for signer in signers {
        match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "{what} panelist {signer} is not an active registered mediator"
                )))
            }
            Some(home) => {
                if &home == accused {
                    return Err(KcError::Authority(format!(
                        "{what} panelist {signer} is from the accused community (non-overlap violated)"
                    )));
                }
                panel.insert((*signer).to_string());
            }
        }
    }
    if panel.len() < panel_min {
        return Err(KcError::Authority(format!(
            "{what} panel has {} distinct mediators, need >= {}",
            panel.len(),
            panel_min
        )));
    }
    Ok(())
}

/// §8 whistleblower rule: a non-party mediator reception panel of at least
/// `panel_min` vouches the report. The source is a commitment, not recorded here,
/// so there is nothing further to check about it at the archive layer.
fn authorize_whistleblower(
    archive: &InMemoryArchive,
    wb: &WhistleblowerReport,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    authorize_reception_panel(
        archive,
        &wb.accused_community,
        signers,
        wb.filed_at,
        panel_min,
        "whistleblower reception",
    )
}

/// §8 source-reveal rule: references a filed whistleblower report, cites a
/// non-empty cause, and is authorized by a non-party mediator panel of at least
/// `panel_min` (the source is disclosed only for cause, by quorum).
fn authorize_source_reveal(
    archive: &InMemoryArchive,
    rev: &SourceReveal,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let wb = match archive.get(&rev.report_cid) {
        Some(ArchiveEntry::WhistleblowerFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "source reveal references {} which is not a whistleblower report",
                rev.report_cid
            )))
        }
    };
    if rev.cause.trim().is_empty() {
        return Err(KcError::Authority("source reveal must cite a cause".into()));
    }
    authorize_reception_panel(
        archive,
        &wb.accused_community,
        signers,
        rev.revealed_at,
        panel_min,
        "source reveal",
    )
}

/// The genesis-original compact-incapable keys (from the singleton
/// `ConstitutionRatified`, if one exists). These are genesis-immutable: never
/// removable.
fn genesis_incapable_keys(archive: &InMemoryArchive) -> BTreeSet<String> {
    for (_, e) in archive.iter() {
        if let ArchiveEntry::ConstitutionRatified(s) = e {
            return s.payload.compact_incapable.iter().map(|m| m.key.clone()).collect();
        }
    }
    BTreeSet::new()
}

/// The compact-incapable keys in force just before `self_cid` in insertion order:
/// the genesis matters plus every `Add` minus every `Remove` among the
/// amendments preceding `self_cid`. Mirrors `current_rung` / `current_sanction_level`.
fn current_incapable_keys(archive: &InMemoryArchive, self_cid: &Cid) -> BTreeSet<String> {
    let self_key = self_cid.to_string();
    let mut keys = genesis_incapable_keys(archive);
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::CompactIncapableAmended(s) = e {
            match s.payload.kind {
                AmendmentKind::Add => {
                    keys.insert(s.payload.matter.key.clone());
                }
                AmendmentKind::Remove => {
                    keys.remove(&s.payload.matter.key);
                }
            }
        }
    }
    keys
}

/// §9 genesis rule: the constitution is a **singleton** — a second
/// `ConstitutionRatified` appearing after the first (in insertion order) is
/// refused. The ratifier quorum is enforced generically via the second-class flag.
fn authorize_genesis(archive: &InMemoryArchive, self_cid: &Cid) -> KcResult<()> {
    let self_key = self_cid.to_string();
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if matches!(e, ArchiveEntry::ConstitutionRatified(_)) {
            return Err(KcError::Authority(
                "the constitution is already ratified; genesis is a singleton".into(),
            ));
        }
    }
    Ok(())
}

/// True if a genesis exists at or before `self_cid` in insertion order.
fn genesis_present_before(archive: &InMemoryArchive, self_cid: &Cid) -> bool {
    let self_key = self_cid.to_string();
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            return false;
        }
        if matches!(e, ArchiveEntry::ConstitutionRatified(_)) {
            return true;
        }
    }
    false
}

/// §9 compact-incapable amendment rule (categorical deference, asymmetric):
/// requires a prior genesis; an `Add` must name a matter not already in force; a
/// `Remove` must name one that is in force, may **not** name a genesis-original
/// matter (genesis-immutable), and carries the higher contraction bar
/// (>= `quorum_threshold + 1` distinct deciders). The normal quorum is enforced
/// generically before this runs.
fn authorize_incapable_amendment(
    archive: &InMemoryArchive,
    a: &CompactIncapableAmendment,
    self_cid: &Cid,
    quorum_threshold: usize,
) -> KcResult<()> {
    if !genesis_present_before(archive, self_cid) {
        return Err(KcError::Authority(
            "compact-incapable amendment before the constitution is ratified".into(),
        ));
    }
    let current = current_incapable_keys(archive, self_cid);
    match a.kind {
        AmendmentKind::Add => {
            if current.contains(&a.matter.key) {
                return Err(KcError::Authority(format!(
                    "compact-incapable matter {:?} is already in force",
                    a.matter.key
                )));
            }
        }
        AmendmentKind::Remove => {
            if !current.contains(&a.matter.key) {
                return Err(KcError::Authority(format!(
                    "compact-incapable matter {:?} is not in force, cannot remove",
                    a.matter.key
                )));
            }
            if genesis_incapable_keys(archive).contains(&a.matter.key) {
                return Err(KcError::Authority(format!(
                    "compact-incapable matter {:?} is genesis-immutable and cannot be removed",
                    a.matter.key
                )));
            }
            let distinct: BTreeSet<&str> = a.deciders.iter().map(|d| d.as_str()).collect();
            if distinct.len() < quorum_threshold + 1 {
                return Err(KcError::Authority(format!(
                    "contracting the compact-incapable list needs >= {} distinct communities, got {}",
                    quorum_threshold + 1,
                    distinct.len()
                )));
            }
        }
    }
    Ok(())
}

/// §9 fast-cadence amendment rule: requires a prior genesis; a `Contract`
/// (shortening or removing a fast) carries the higher bar (>= `quorum_threshold +
/// 1` distinct deciders). `Extend` uses the normal quorum enforced generically.
fn authorize_fast_amendment(
    archive: &InMemoryArchive,
    a: &FastCadenceAmendment,
    self_cid: &Cid,
    quorum_threshold: usize,
) -> KcResult<()> {
    if !genesis_present_before(archive, self_cid) {
        return Err(KcError::Authority(
            "fast-cadence amendment before the constitution is ratified".into(),
        ));
    }
    if a.change.is_contraction() {
        let distinct: BTreeSet<&str> = a.deciders.iter().map(|d| d.as_str()).collect();
        if distinct.len() < quorum_threshold + 1 {
            return Err(KcError::Authority(format!(
                "contracting the structured-fast cadence needs >= {} distinct communities, got {}",
                quorum_threshold + 1,
                distinct.len()
            )));
        }
    }
    Ok(())
}

/// The communities that may not sit on an injunction's panel: the subject, plus
/// (if the injunction arises from a dispute) that dispute's parties.
fn injunction_barred_communities(
    archive: &InMemoryArchive,
    subject: &DidKey,
    dispute_cid: Option<&Cid>,
) -> BTreeSet<String> {
    let mut barred: BTreeSet<String> = BTreeSet::new();
    barred.insert(subject.as_str().to_string());
    if let Some(cid) = dispute_cid {
        if let Some(ArchiveEntry::DisputeFiled(s)) = archive.get(cid) {
            barred.insert(s.payload.complainant.as_str().to_string());
            barred.insert(s.payload.respondent.as_str().to_string());
        }
    }
    barred
}

/// §9 emergency-panel rule for issuing an interim injunction: every co-signer is
/// an active registered mediator, none from a barred community (the subject, and
/// the dispute parties if it arises from a dispute), and the panel spans at least
/// `panel_min` distinct mediators. The protective window must be non-empty.
fn authorize_injunction(
    archive: &InMemoryArchive,
    inj: &InterimInjunction,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    if inj.expires_at.as_second() <= inj.issued_at.as_second() {
        return Err(KcError::Authority(
            "injunction must expire after it is issued".into(),
        ));
    }
    let barred = injunction_barred_communities(archive, &inj.subject_community, inj.dispute_cid.as_ref());
    let at = inj.issued_at;
    let mut panel: BTreeSet<String> = BTreeSet::new();
    for signer in signers {
        match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "injunction panelist {signer} is not an active registered mediator"
                )))
            }
            Some(home) => {
                if barred.contains(home.as_str()) {
                    return Err(KcError::Authority(format!(
                        "injunction panelist {signer} is from a party community (non-overlap violated)"
                    )));
                }
                panel.insert((*signer).to_string());
            }
        }
    }
    if panel.len() < panel_min {
        return Err(KcError::Authority(format!(
            "injunction panel has {} distinct mediators, need >= {}",
            panel.len(),
            panel_min
        )));
    }
    Ok(())
}

/// §9 challenge rule: an injunction challenge is heard by a *different* panel than
/// the one that issued it. Every co-signer is an active registered mediator, none
/// from a barred community, the panel spans at least `panel_min`, and no panelist
/// sat on the issuing panel (the "different panel" safeguard).
fn authorize_injunction_challenge(
    archive: &InMemoryArchive,
    chal: &InjunctionChallenge,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let injunction = match archive.get(&chal.injunction_cid) {
        Some(ArchiveEntry::InjunctionIssued(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "injunction challenge references {} which is not an issued injunction",
                chal.injunction_cid
            )))
        }
    };
    let issuing: BTreeSet<&str> = injunction.panel.iter().map(|d| d.as_str()).collect();
    let barred = injunction_barred_communities(
        archive,
        &injunction.subject_community,
        injunction.dispute_cid.as_ref(),
    );
    let at = chal.decided_at;
    let mut panel: BTreeSet<String> = BTreeSet::new();
    for signer in signers {
        if issuing.contains(signer) {
            return Err(KcError::Authority(format!(
                "challenge panelist {signer} also sat on the issuing panel (different-panel safeguard violated)"
            )));
        }
        match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "challenge panelist {signer} is not an active registered mediator"
                )))
            }
            Some(home) => {
                if barred.contains(home.as_str()) {
                    return Err(KcError::Authority(format!(
                        "challenge panelist {signer} is from a party community (non-overlap violated)"
                    )));
                }
                panel.insert((*signer).to_string());
            }
        }
    }
    if panel.len() < panel_min {
        return Err(KcError::Authority(format!(
            "challenge panel has {} distinct mediators, need >= {}",
            panel.len(),
            panel_min
        )));
    }
    Ok(())
}

/// Home communities that supplied a mediator to an injunction panel still in force
/// against `subject` as of `at` (issued and within its window or confirmed, and not
/// overturned). Used for designation safeguard #1: the community that drove an
/// emergency injunction may not also deliberate the permanent designation.
fn active_injunction_panel_communities(
    archive: &InMemoryArchive,
    subject: &DidKey,
    at: Timestamp,
) -> BTreeSet<String> {
    let mut communities: BTreeSet<String> = BTreeSet::new();
    for (cid, e) in archive.iter() {
        let inj = match e {
            ArchiveEntry::InjunctionIssued(s) if &s.payload.subject_community == subject => {
                &s.payload
            }
            _ => continue,
        };
        if inj.issued_at.as_second() > at.as_second() {
            continue;
        }
        // Overturned by a ruled challenge decided on or before `at`?
        let overturned = archive.iter().any(|(_, e2)| match e2 {
            ArchiveEntry::InjunctionChallenged(s) => {
                s.payload.injunction_cid == *cid
                    && s.payload.overturned
                    && s.payload.decided_at.as_second() <= at.as_second()
            }
            _ => false,
        });
        if overturned {
            continue;
        }
        // In force: within the window, or confirmed before it lapsed.
        let confirmed = archive.iter().any(|(_, e2)| match e2 {
            ArchiveEntry::InjunctionConfirmed(s) => {
                s.payload.injunction_cid == *cid
                    && s.payload.confirmed_at.as_second() <= inj.expires_at.as_second()
            }
            _ => false,
        });
        let in_window = at.as_second() <= inj.expires_at.as_second();
        if !(in_window || confirmed) {
            continue;
        }
        for panelist in &inj.panel {
            if let Some(home) = mediator_home(archive, panelist.as_str(), inj.issued_at) {
                communities.insert(home.as_str().to_string());
            }
        }
    }
    communities
}

/// If `signer_did` is an active delegate at `at`, its (community_did, scope).
/// Uses latest-issued-wins supersession so revocation/expiry are honored.
fn delegate_of(
    archive: &InMemoryArchive,
    signer_did: &str,
    at: Timestamp,
) -> Option<(DidKey, DelegationScope)> {
    let latest = archive
        .iter()
        .filter_map(|(_, e)| match e {
            ArchiveEntry::SpeakerDelegated(s)
                if s.payload.speaker_did.as_str() == signer_did && s.payload.issued_at <= at =>
            {
                Some(&s.payload)
            }
            _ => None,
        })
        .max_by_key(|d| (d.issued_at, d.revoked_at.is_some()));
    latest.and_then(|d| {
        if d.active_at(at) {
            Some((d.community_did.clone(), d.scope))
        } else {
            None
        }
    })
}

fn signer_authorized_for(
    archive: &InMemoryArchive,
    principal: &DidKey,
    signer_did: &str,
    at: Timestamp,
    allowed: &[DelegationScope],
) -> bool {
    // Root: a community signing for itself is always authorized.
    if signer_did == principal.as_str() {
        return true;
    }
    match delegate_of(archive, signer_did, at) {
        Some((community, scope)) => &community == principal && allowed.contains(&scope),
        None => false,
    }
}

/// Check one record's authority. `Ok(())` if authorized; `KcError::Authority`
/// naming the first violation otherwise.
pub fn authorize(
    archive: &InMemoryArchive,
    entry: &ArchiveEntry,
    quorum_threshold: usize,
    panel_min: usize,
) -> KcResult<()> {
    // Mediation decisions are authorized by the panel rule, not principal binding.
    if let ArchiveEntry::MediationDecided(s) = entry {
        let signers = signer_dids(entry);
        return authorize_decision(archive, &s.payload, &signers, panel_min);
    }
    // Restitution completions are authorized by the verifier-panel rule (§5).
    if let ArchiveEntry::RestitutionVerified(s) = entry {
        let signers = signer_dids(entry);
        return authorize_restitution_completion(archive, &s.payload, &signers, panel_min);
    }
    // §9. Injunction issuance and challenge are authorized by an emergency-panel
    // rule, not principal binding (like a mediation decision).
    if let ArchiveEntry::InjunctionIssued(s) = entry {
        let signers = signer_dids(entry);
        return authorize_injunction(archive, &s.payload, &signers, panel_min);
    }
    if let ArchiveEntry::InjunctionChallenged(s) = entry {
        let signers = signer_dids(entry);
        return authorize_injunction_challenge(archive, &s.payload, &signers, panel_min);
    }
    // §8. Observer reports and whistleblower records are authorized by the
    // observer / reception-panel rules, not principal binding.
    if let ArchiveEntry::ObserverReportPublished(s) = entry {
        let signers = signer_dids(entry);
        return authorize_observer_report(archive, &s.payload, &signers);
    }
    if let ArchiveEntry::WhistleblowerFiled(s) = entry {
        let signers = signer_dids(entry);
        return authorize_whistleblower(archive, &s.payload, &signers, panel_min);
    }
    if let ArchiveEntry::SourceRevealed(s) = entry {
        let signers = signer_dids(entry);
        return authorize_source_reveal(archive, &s.payload, &signers, panel_min);
    }

    let auth = match record_authority(entry) {
        Some(a) => a,
        None => return Ok(()),
    };
    let allowed = allowed_scopes(entry);
    let signers = signer_dids(entry);

    let mut backed: BTreeSet<String> = BTreeSet::new();
    for principal in &auth.principals {
        let ok = signers
            .iter()
            .any(|s| signer_authorized_for(archive, principal, s, auth.at, allowed));
        if !ok {
            return Err(KcError::Authority(format!(
                "principal {} has no authorized signer (allowed delegate scopes: {:?})",
                principal.as_str(),
                allowed
            )));
        }
        backed.insert(principal.as_str().to_string());
    }

    if auth.second_class && backed.len() < quorum_threshold {
        return Err(KcError::Authority(format!(
            "second-class decision requires >= {} distinct communities, got {}",
            quorum_threshold,
            backed.len()
        )));
    }

    // Component-specific structural rules (§5), after principal binding + quorum.
    match entry {
        ArchiveEntry::LadderAdvanced(s) => {
            let self_cid = entry.cid()?;
            authorize_transition(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        ArchiveEntry::DesignationDecided(s) => {
            authorize_designation(archive, &s.payload, quorum_threshold)?;
        }
        ArchiveEntry::ReEntryCompleted(s) => {
            authorize_reentry(archive, &s.payload)?;
        }
        ArchiveEntry::CommonsRegistered(s) => {
            authorize_commons_registration(&s.payload)?;
        }
        ArchiveEntry::UsageAccounted(s) => {
            authorize_usage(archive, &s.payload)?;
        }
        ArchiveEntry::SanctionApplied(s) => {
            let self_cid = entry.cid()?;
            authorize_commons_sanction(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        ArchiveEntry::ReportResponsePublished(s) => {
            authorize_report_response(archive, &s.payload)?;
        }
        ArchiveEntry::ConstitutionRatified(_) => {
            let self_cid = entry.cid()?;
            authorize_genesis(archive, &self_cid)?;
        }
        ArchiveEntry::CompactIncapableAmended(s) => {
            let self_cid = entry.cid()?;
            authorize_incapable_amendment(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        ArchiveEntry::FastCadenceAmended(s) => {
            let self_cid = entry.cid()?;
            authorize_fast_amendment(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        _ => {}
    }
    Ok(())
}

/// Check authority for every record in the archive, fail-fast. Pair with
/// `verify_all` (crypto integrity); this pass assumes signatures already verify.
/// `quorum_threshold` gates second-class decisions; `panel_min` gates mediation
/// panels (§4).
pub fn check_authority(
    archive: &InMemoryArchive,
    quorum_threshold: usize,
    panel_min: usize,
) -> KcResult<()> {
    for (_, entry) in archive.iter() {
        authorize(archive, entry, quorum_threshold, panel_min)?;
    }
    Ok(())
}

/// Every authority violation in the archive, as `(cid, reason)`, rather than
/// failing on the first.
pub fn authority_violations(
    archive: &InMemoryArchive,
    quorum_threshold: usize,
    panel_min: usize,
) -> Vec<(String, String)> {
    let mut out = vec![];
    for (cid, entry) in archive.iter() {
        if let Err(e) = authorize(archive, entry, quorum_threshold, panel_min) {
            out.push((cid.to_string(), e.to_string()));
        }
    }
    out
}
