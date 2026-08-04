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

//! Reputation-lookup function. Single-pass projection over an
//! `InMemoryArchive`.

use std::collections::{HashMap, HashSet};

use kc_archive::{probationary_at, ArchiveEntry, InMemoryArchive};
use kc_commons::{CommonsSanction, SanctionLevel};
use kc_conduct::{ConductKind, LadderRung};
use kc_core::{time, Cid, DidKey, Timestamp};
use kc_identity::SpeakerDelegation;

use crate::view::{
    CommonsSanctionSummary, ConductSummary, DelegationSummary, DesignationSummary, DisputeSummary,
    InjunctionSummary, InterAdmissionSummary, InterNonAdmissionSummary, IntraRiteSummary,
    ObserverReportSummary, RecognitionStanding, ReEntrySummary, RejectionSummary,
    ReportResponseSummary, ReputationView, RestitutionSummary, SponsorshipSummary,
    SubstantiveReEngagementSummary, WhistleblowerSummary, WitnessSummary,
};

pub fn lookup(
    archive: &InMemoryArchive,
    community_did: &DidKey,
    as_of: Option<Timestamp>,
) -> ReputationView {
    let as_of = as_of.unwrap_or_else(time::now);

    let mut view = ReputationView {
        community_did: community_did.clone(),
        as_of,
        standing: RecognitionStanding::Unknown,
        identity_cid: None,
        intra_rites: vec![],
        inter_admissions: vec![],
        inter_nonadmissions: vec![],
        active_delegations: vec![],
        active_witnesses: vec![],
        conduct: vec![],
        sponsorships_given: vec![],
        sponsorships_received: vec![],
        disputes_filed: vec![],
        disputes_against: vec![],
        designations: vec![],
        reentries: vec![],
        restitutions: vec![],
        substantive_reengagements: vec![],
        injunctions: vec![],
        revoked_capabilities: vec![],
        commons_sanctions: vec![],
        observer_reports: vec![],
        whistleblower_reports_against: vec![],
    };

    // Latest CommunityIdentityPublished with created_at <= as_of wins.
    let mut best_identity_at: Option<Timestamp> = None;

    for (cid, entry) in archive.iter() {
        match entry {
            ArchiveEntry::CommunityIdentityPublished(signed) => {
                let ci = &signed.payload;
                if &ci.did == community_did
                    && ci.created_at <= as_of
                    && best_identity_at.is_none_or(|prev| ci.created_at >= prev)
                {
                    view.identity_cid = Some(*cid);
                    best_identity_at = Some(ci.created_at);
                }
            }
            ArchiveEntry::IntraCommunityRite(signed) => {
                let rec = &signed.payload;
                let recitation = &rec.recitation.payload;
                if &recitation.community_did == community_did && rec.sealed_at <= as_of {
                    view.intra_rites.push(IntraRiteSummary {
                        record_cid: *cid,
                        sealed_at: rec.sealed_at,
                        speaker_did: recitation.speaker_did.clone(),
                        witness_count: rec.internal_witnesses.len(),
                    });
                }
            }
            ArchiveEntry::InterCommunityWitness(signed) => {
                let rec = &signed.payload;
                if rec.participants.iter().any(|d| d == community_did)
                    && rec.witnessed_at <= as_of
                {
                    let counterparties: Vec<DidKey> = rec
                        .participants
                        .iter()
                        .filter(|d| *d != community_did)
                        .cloned()
                        .collect();
                    view.inter_admissions.push(InterAdmissionSummary {
                        record_cid: *cid,
                        witnessed_at: rec.witnessed_at,
                        counterparties,
                    });
                }
            }
            ArchiveEntry::InterCommunityNonAdmission(signed) => {
                let rec = &signed.payload;
                if rec.participants.iter().any(|d| d == community_did)
                    && rec.decided_at <= as_of
                {
                    let counterparties: Vec<DidKey> = rec
                        .participants
                        .iter()
                        .filter(|d| *d != community_did)
                        .cloned()
                        .collect();
                    let rejections: Vec<RejectionSummary> = rec
                        .rejections
                        .iter()
                        .filter(|r| {
                            &r.rejecting_community == community_did
                                || &r.presenting_community == community_did
                        })
                        .map(|r| RejectionSummary {
                            rejecting_community: r.rejecting_community.clone(),
                            presenting_community: r.presenting_community.clone(),
                            feature: r.feature,
                            rationale: r.rationale.clone(),
                        })
                        .collect();
                    view.inter_nonadmissions.push(InterNonAdmissionSummary {
                        record_cid: *cid,
                        decided_at: rec.decided_at,
                        counterparties,
                        rejections,
                    });
                }
            }
            // Speaker delegations are supersession-resolved after the loop.
            ArchiveEntry::SpeakerDelegated(_) => {}
            ArchiveEntry::SponsorshipVouched(signed) => {
                let s = &signed.payload;
                if s.sponsored_at <= as_of {
                    let summary = SponsorshipSummary {
                        record_cid: *cid,
                        sponsor_community: s.sponsor_community.clone(),
                        entrant_community: s.entrant_community.clone(),
                        admission_cid: s.admission_cid,
                        probation_until: s.probation_until,
                        sponsored_at: s.sponsored_at,
                    };
                    if &s.sponsor_community == community_did {
                        view.sponsorships_given.push(summary.clone());
                    }
                    if &s.entrant_community == community_did {
                        view.sponsorships_received.push(summary);
                    }
                }
            }
            ArchiveEntry::ConductAttested(signed) => {
                let c = &signed.payload;
                if &c.subject_community == community_did && c.recorded_at <= as_of {
                    view.conduct.push(ConductSummary {
                        record_cid: *cid,
                        kind: c.kind,
                        bearing: c.bearing.clone(),
                        severity: c.severity,
                        ladder_rung: c.ladder_rung,
                        decision_class: c.decision_class,
                        reporters: c.reporters.clone(),
                        reporter_perspective: c.reporter_perspective.clone(),
                        references: c.references.clone(),
                        recorded_at: c.recorded_at,
                    });
                }
            }
            ArchiveEntry::DesignationDecided(signed) => {
                let d = &signed.payload;
                if &d.subject_community == community_did && d.designated_at <= as_of {
                    view.designations.push(DesignationSummary {
                        record_cid: *cid,
                        tier: d.tier.as_str().to_string(),
                        urgency: d.urgency.as_str().to_string(),
                        criteria: d.criteria.clone(),
                        reversal_conditions: d.reversal_conditions.clone(),
                        reexamine_after: d.reexamine_after,
                        reversed: false, // set by the reversal join below
                        designated_at: d.designated_at,
                    });
                }
            }
            ArchiveEntry::ReEntryCompleted(signed) => {
                let r = &signed.payload;
                if &r.petitioner == community_did && r.completed_at <= as_of {
                    view.reentries.push(ReEntrySummary {
                        record_cid: *cid,
                        designation_cid: r.designation_cid,
                        probation_until: r.probation_until,
                        completed_at: r.completed_at,
                    });
                }
            }
            ArchiveEntry::RestitutionObligated(signed) => {
                let o = &signed.payload;
                if &o.subject_community == community_did && o.imposed_at <= as_of {
                    view.restitutions.push(RestitutionSummary {
                        record_cid: *cid,
                        beneficiary: o.beneficiary.clone(),
                        terms: o.terms.clone(),
                        satisfied: None, // set by the verification join below
                        imposed_at: o.imposed_at,
                    });
                }
            }
            ArchiveEntry::SubstantiveReEngagementInvoked(signed) => {
                let s = &signed.payload;
                if &s.subject_community == community_did && s.invoked_at <= as_of {
                    view.substantive_reengagements.push(SubstantiveReEngagementSummary {
                        record_cid: *cid,
                        required_reaffirmation_by: s.required_reaffirmation_by,
                        invoked_at: s.invoked_at,
                    });
                }
            }
            ArchiveEntry::InjunctionIssued(signed) => {
                let inj = &signed.payload;
                if &inj.subject_community == community_did && inj.issued_at <= as_of {
                    view.injunctions.push(InjunctionSummary {
                        record_cid: *cid,
                        harm: inj.harm.clone(),
                        protective_measures: inj.protective_measures.clone(),
                        revoked_capabilities: inj
                            .revoked_capabilities
                            .iter()
                            .map(|c| c.as_str().to_string())
                            .collect(),
                        status: String::new(), // derived by the status join below
                        issued_at: inj.issued_at,
                        expires_at: inj.expires_at,
                    });
                }
            }
            _ => {}
        }
    }

    // Pre-scan: find the latest WitnessRegistryUpdated whose issued_at ≤ as_of.
    // Registry entries supersede each other; only the newest-in-window applies.
    let latest_registry = archive
        .iter()
        .filter_map(|(cid, entry)| match entry {
            ArchiveEntry::WitnessRegistryUpdated(signed)
                if &signed.payload.community_did == community_did
                    && signed.payload.issued_at <= as_of =>
            {
                Some((cid, &signed.payload))
            }
            _ => None,
        })
        .max_by_key(|(_, reg)| reg.issued_at);

    if let Some((reg_cid, reg)) = latest_registry {
        for e in &reg.entries {
            let valid_now = e.valid_from <= as_of
                && e.valid_until.is_none_or(|end| as_of < end);
            if valid_now {
                view.active_witnesses.push(WitnessSummary {
                    record_cid: *reg_cid,
                    member_did: e.witness_did.clone(),
                    role: e.role,
                    active_from: e.valid_from,
                    active_until: e.valid_until,
                });
            }
        }
    }

    // Speaker-delegation supersession: per speaker_did the record with the
    // greatest issued_at (<= as_of) wins, so an appended revocation overrides an
    // earlier grant (on an issued_at tie a revoking record wins). Only include a
    // speaker whose winning record is active at as_of.
    let mut latest_deleg: HashMap<String, (&Cid, &SpeakerDelegation)> = HashMap::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::SpeakerDelegated(signed) = entry {
            let d = &signed.payload;
            if &d.community_did == community_did && d.issued_at <= as_of {
                let key = d.speaker_did.as_str().to_string();
                let supersedes = match latest_deleg.get(&key) {
                    Some((_, prev)) => {
                        (d.issued_at, d.revoked_at.is_some())
                            > (prev.issued_at, prev.revoked_at.is_some())
                    }
                    None => true,
                };
                if supersedes {
                    latest_deleg.insert(key, (cid, d));
                }
            }
        }
    }
    for (_, (cid, d)) in latest_deleg {
        if d.active_at(as_of) {
            view.active_delegations.push(DelegationSummary {
                record_cid: *cid,
                subject_did: d.speaker_did.clone(),
                scope: d.scope,
                active_from: d.issued_at,
                active_until: d.expires_at,
            });
        }
    }
    view
        .active_delegations
        .sort_by(|a, b| a.subject_did.as_str().cmp(b.subject_did.as_str()));

    // §4 disputes this community is party to, joined with any mediation outcome.
    let mut decisions: HashMap<String, String> = HashMap::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::MediationDecided(s) = entry {
            if s.payload.decided_at <= as_of {
                decisions.insert(
                    s.payload.dispute_cid.to_string(),
                    s.payload.outcome.as_str().to_string(),
                );
            }
        }
    }
    // §5 current rung per dispute: the `to_rung` of the latest ladder transition
    // (transitioned_at <= as_of), else the filed rung.
    let mut ladder: HashMap<String, (Timestamp, LadderRung)> = HashMap::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::LadderAdvanced(s) = entry {
            let t = &s.payload;
            if t.transitioned_at <= as_of {
                let key = t.dispute_cid.to_string();
                let supersede = ladder.get(&key).is_none_or(|(at, _)| t.transitioned_at >= *at);
                if supersede {
                    ladder.insert(key, (t.transitioned_at, t.to_rung));
                }
            }
        }
    }
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::DisputeFiled(s) = entry {
            let d = &s.payload;
            if d.filed_at <= as_of
                && (&d.complainant == community_did || &d.respondent == community_did)
            {
                let current_rung = ladder
                    .get(&cid.to_string())
                    .map_or(d.rung, |(_, r)| *r);
                let summary = DisputeSummary {
                    record_cid: *cid,
                    complainant: d.complainant.clone(),
                    respondent: d.respondent.clone(),
                    rung: d.rung,
                    current_rung,
                    summary: d.summary.clone(),
                    filed_at: d.filed_at,
                    decided_outcome: decisions.get(&cid.to_string()).cloned(),
                };
                if &d.complainant == community_did {
                    view.disputes_filed.push(summary.clone());
                }
                if &d.respondent == community_did {
                    view.disputes_against.push(summary);
                }
            }
        }
    }
    view.disputes_filed.sort_by_key(|s| s.filed_at);
    view.disputes_against.sort_by_key(|s| s.filed_at);

    // §5 restitution verification join: obligation_cid → satisfied.
    let mut verified: HashMap<String, bool> = HashMap::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::RestitutionVerified(s) = entry {
            if s.payload.verified_at <= as_of {
                verified.insert(s.payload.obligation_cid.to_string(), s.payload.satisfied);
            }
        }
    }
    for r in &mut view.restitutions {
        r.satisfied = verified.get(&r.record_cid.to_string()).copied();
    }

    // §5 designation reversal set: any designation whose CID is referenced by a
    // DesignationLifted conduct record, or answered by one of this community's
    // re-entries, is reversed.
    let mut reversed: HashSet<String> = view
        .conduct
        .iter()
        .filter(|c| c.kind == ConductKind::DesignationLifted)
        .flat_map(|c| c.references.iter().map(|r| r.to_string()))
        .collect();
    for re in &view.reentries {
        reversed.insert(re.designation_cid.to_string());
    }
    for d in &mut view.designations {
        d.reversed = reversed.contains(&d.record_cid.to_string());
    }

    // §9 injunction status derivation. For each injunction naming this community,
    // scan confirmations and overturning challenges (visible as of `as_of`):
    //   Overturned  — a challenge ruled to overturn it (terminal, wins over all).
    //   Confirmed   — a compact-quorum confirmation arrived before it expired.
    //   Active      — issued, still within its window, unconfirmed.
    //   Lapsed      — the window passed with no confirmation.
    // Capabilities are revoked while Active or Confirmed (not Overturned/Lapsed);
    // their union (sorted) is the community's currently-revoked set.
    for inj in &mut view.injunctions {
        let cid_s = inj.record_cid.to_string();
        let overturned = archive.iter().any(|(_, e)| match e {
            ArchiveEntry::InjunctionChallenged(s) => {
                s.payload.injunction_cid.to_string() == cid_s
                    && s.payload.overturned
                    && s.payload.decided_at <= as_of
            }
            _ => false,
        });
        let confirmed = archive.iter().any(|(_, e)| match e {
            ArchiveEntry::InjunctionConfirmed(s) => {
                s.payload.injunction_cid.to_string() == cid_s
                    && s.payload.confirmed_at <= as_of
                    && s.payload.confirmed_at <= inj.expires_at
            }
            _ => false,
        });
        inj.status = if overturned {
            "Overturned"
        } else if confirmed {
            "Confirmed"
        } else if as_of <= inj.expires_at {
            "Active"
        } else {
            "Lapsed"
        }
        .to_string();
    }
    let mut revoked: HashSet<String> = HashSet::new();
    for inj in &view.injunctions {
        if inj.status == "Active" || inj.status == "Confirmed" {
            for cap in &inj.revoked_capabilities {
                revoked.insert(cap.clone());
            }
        }
    }
    view.revoked_capabilities = revoked.into_iter().collect();
    view.revoked_capabilities.sort();

    // §7 commons sanctions naming this community as subject. The current level per
    // commons is the `to_level` of the latest sanction (decided_at <= as_of;
    // insertion order breaks a tie), joined with the commons' kind. Only commons
    // whose current level is not `None` are surfaced (a restored pair is at rest).
    let mut commons_kind: HashMap<String, String> = HashMap::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::CommonsRegistered(s) = entry {
            commons_kind.insert(cid.to_string(), s.payload.kind.as_str().to_string());
        }
    }
    let mut latest_sanction: HashMap<String, (Timestamp, &Cid, &CommonsSanction)> = HashMap::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::SanctionApplied(s) = entry {
            let p = &s.payload;
            if &p.subject_community == community_did && p.decided_at <= as_of {
                let key = p.commons_cid.to_string();
                let supersede = latest_sanction.get(&key).is_none_or(|(at, _, _)| p.decided_at >= *at);
                if supersede {
                    latest_sanction.insert(key, (p.decided_at, cid, p));
                }
            }
        }
    }
    for (commons_key, (_, cid, p)) in latest_sanction {
        if matches!(p.to_level, SanctionLevel::None) {
            continue;
        }
        view.commons_sanctions.push(CommonsSanctionSummary {
            record_cid: *cid,
            commons_cid: p.commons_cid,
            kind: commons_kind.get(&commons_key).cloned().unwrap_or_else(|| "Unknown".into()),
            current_level: p.to_level.as_str().to_string(),
            basis: p.basis.clone(),
            decided_at: p.decided_at,
        });
    }
    view.commons_sanctions.sort_by_key(|s| s.decided_at);

    // §8 observer reports naming this community as subject, joined with the
    // responses appended to each (a report_cid -> its responses map, in
    // record-time order).
    let mut responses: HashMap<String, Vec<ReportResponseSummary>> = HashMap::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::ReportResponsePublished(s) = entry {
            let r = &s.payload;
            if r.responded_at <= as_of {
                responses.entry(r.report_cid.to_string()).or_default().push(ReportResponseSummary {
                    record_cid: *cid,
                    response: r.response.clone(),
                    responded_at: r.responded_at,
                });
            }
        }
    }
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::ObserverReportPublished(s) = entry {
            let rep = &s.payload;
            if &rep.subject_community == community_did && rep.reported_at <= as_of {
                let mut resp =
                    responses.get(&cid.to_string()).cloned().unwrap_or_default();
                resp.sort_by_key(|r| r.responded_at);
                view.observer_reports.push(ObserverReportSummary {
                    record_cid: *cid,
                    observer_did: rep.observer_did.clone(),
                    declared_perspective: rep.declared_perspective.clone(),
                    summary: rep.summary.clone(),
                    observed_at: rep.observed_at,
                    reported_at: rep.reported_at,
                    responses: resp,
                });
            }
        }
    }
    view.observer_reports.sort_by_key(|s| s.reported_at);

    // §8 whistleblower reports naming this community as accused. The source is
    // never projected — only whether a for-cause reveal has since disclosed it.
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::WhistleblowerFiled(s) = entry {
            let wb = &s.payload;
            if &wb.accused_community == community_did && wb.filed_at <= as_of {
                let revealed = archive.iter().any(|(_, e)| match e {
                    ArchiveEntry::SourceRevealed(r) => {
                        r.payload.report_cid == *cid && r.payload.revealed_at <= as_of
                    }
                    _ => false,
                });
                let exculpations = archive
                    .iter()
                    .filter(|(_, e)| match e {
                        ArchiveEntry::SourceExculpated(x) => {
                            x.payload.report_cid == *cid && x.payload.exculpated_at <= as_of
                        }
                        _ => false,
                    })
                    .count();
                view.whistleblower_reports_against.push(WhistleblowerSummary {
                    record_cid: *cid,
                    claim: wb.claim.clone(),
                    filed_at: wb.filed_at,
                    source_revealed: revealed,
                    source_tag: wb.source_tag().to_hex(),
                    exculpations,
                });
            }
        }
    }
    view.whistleblower_reports_against.sort_by_key(|s| s.filed_at);

    view.intra_rites.sort_by_key(|s| s.sealed_at);
    view.conduct.sort_by_key(|s| s.recorded_at);
    view.sponsorships_given.sort_by_key(|s| s.sponsored_at);
    view.sponsorships_received.sort_by_key(|s| s.sponsored_at);
    view.designations.sort_by_key(|s| s.designated_at);
    view.reentries.sort_by_key(|s| s.completed_at);
    view.restitutions.sort_by_key(|s| s.imposed_at);
    view.substantive_reengagements.sort_by_key(|s| s.invoked_at);
    view.injunctions.sort_by_key(|s| s.issued_at);
    view.standing = derive_standing(&view, archive);
    view
}

/// §9 capability gate: `true` unless `capability` is currently revoked from
/// `community_did` by an active or confirmed interim injunction, as of `as_of`.
/// Membership implies all capabilities; an injunction is the only thing that
/// removes one. The `capability` argument is a `Capability` name (see
/// `Capability::as_str`); an unknown name is never in the revoked set, so it
/// simply returns `true`.
pub fn has_capability(
    archive: &InMemoryArchive,
    community_did: &DidKey,
    capability: &str,
    as_of: Option<Timestamp>,
) -> bool {
    let view = lookup(archive, community_did, as_of);
    !view.revoked_capabilities.iter().any(|c| c == capability)
}

/// §7 commons gate: the graduated-sanction level standing against `community_did`
/// on the commons registered at `commons_cid`, as of `as_of` — the `to_level` of
/// the latest `SanctionApplied` for the pair, or [`SanctionLevel::None`] if none
/// stands. A caller enforcing a restriction checks whether this is
/// `Restriction`/`Exclusion`; drawdown is unrestricted at `None`/`Monitoring`/
/// `Warning`. Independent of the commons registration itself resolving — an
/// orphaned sanction still reports its level.
pub fn commons_level(
    archive: &InMemoryArchive,
    community_did: &DidKey,
    commons_cid: &Cid,
    as_of: Option<Timestamp>,
) -> SanctionLevel {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut level = SanctionLevel::None;
    let mut best: Option<Timestamp> = None;
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::SanctionApplied(s) = entry {
            let p = &s.payload;
            if &p.subject_community == community_did
                && &p.commons_cid == commons_cid
                && p.decided_at <= as_of
                && best.is_none_or(|b| p.decided_at >= b)
            {
                best = Some(p.decided_at);
                level = p.to_level;
            }
        }
    }
    level
}

fn derive_standing(view: &ReputationView, archive: &InMemoryArchive) -> RecognitionStanding {
    // Anathema overrides all: an active DesignationAnathema against this
    // community that no later DesignationLifted has superseded (by referencing
    // its CID). view.conduct is already filtered to recorded_at <= as_of.
    let lifted: HashSet<String> = view
        .conduct
        .iter()
        .filter(|c| c.kind == ConductKind::DesignationLifted)
        .flat_map(|c| c.references.iter().map(|r| r.to_string()))
        .collect();
    let active_anathema = view.conduct.iter().any(|c| {
        c.kind == ConductKind::DesignationAnathema && !lifted.contains(&c.record_cid.to_string())
    });
    // New-style (§5) designations carry a precomputed `reversed` flag (set by the
    // lift ∪ re-entry reversal join in `lookup`).
    let active_new_designation = view.designations.iter().any(|d| !d.reversed);
    if active_anathema || active_new_designation {
        return RecognitionStanding::Anathema;
    }

    let has_identity = view.identity_cid.is_some();
    let has_admission = !view.inter_admissions.is_empty();
    let has_nonadmission = !view.inter_nonadmissions.is_empty();
    if !(has_identity || has_admission || has_nonadmission) {
        return RecognitionStanding::Unknown;
    }
    if has_admission {
        // Probationary when inside a received-sponsorship or post-re-entry
        // probation window; otherwise a plain member. The window arithmetic is
        // `kc_archive::probationary_at` — the same predicate the §2 authority rules
        // enforce, so what a dashboard reports and what the gate refuses cannot
        // drift apart. Only the *precedence* is decided here: an active designation
        // has already returned above, and probation applies to an admitted
        // community.
        return if probationary_at(archive, &view.community_did, view.as_of) {
            RecognitionStanding::Probationary
        } else {
            RecognitionStanding::InCompact
        };
    }
    if has_nonadmission {
        return RecognitionStanding::Denied;
    }
    RecognitionStanding::Identified
}
