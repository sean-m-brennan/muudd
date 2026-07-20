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

use kc_conduct::DecisionClass;
use kc_core::{Cid, DidKey, KcError, KcResult, Timestamp};
use kc_escalation::{
    dispute_ladder_rank, is_second_class_rung, Designation, DesignationTier, DesignationUrgency,
    LadderTransition, ReEntry, RestitutionCompletion,
};
use kc_identity::DelegationScope;
use kc_mediation::MediationDecision;

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
        // §5 escalation acts an envoy may sign on the community's behalf.
        ArchiveEntry::LadderAdvanced(_)
        | ArchiveEntry::DesignationDecided(_)
        | ArchiveEntry::RestitutionObligated(_)
        | ArchiveEntry::ReEntryCompleted(_)
        | ArchiveEntry::SubstantiveReEngagementInvoked(_) => &[Envoy],
        ArchiveEntry::IntraCommunityRite(_)
        | ArchiveEntry::SpeakerDelegated(_)
        | ArchiveEntry::WitnessRegistryUpdated(_)
        | ArchiveEntry::MediatorRegistryUpdated(_)
        | ArchiveEntry::CommunityIdentityPublished(_)
        // MediationDecided and RestitutionVerified are authorized by a panel
        // rule, not principal binding; their allowed-scope sets are unused.
        | ArchiveEntry::MediationDecided(_)
        | ArchiveEntry::RestitutionVerified(_) => &[],
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
