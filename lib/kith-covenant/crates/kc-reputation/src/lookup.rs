//! Reputation-lookup function. Single-pass projection over an
//! `InMemoryArchive`.

use kc_archive::{ArchiveEntry, InMemoryArchive};
use kc_core::{time, DidKey, Timestamp};

use crate::view::{
    DelegationSummary,
    InterAdmissionSummary, InterNonAdmissionSummary, IntraRiteSummary,
    RecognitionStanding, RejectionSummary, ReputationView, WitnessSummary,
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
    };

    // Latest CommunityIdentityPublished with created_at <= as_of wins.
    let mut best_identity_at: Option<Timestamp> = None;

    for (cid, entry) in archive.iter() {
        match entry {
            ArchiveEntry::CommunityIdentityPublished(signed) => {
                let ci = &signed.payload;
                if &ci.did == community_did && ci.created_at <= as_of {
                    if best_identity_at.map_or(true, |prev| ci.created_at >= prev) {
                        view.identity_cid = Some(cid.clone());
                        best_identity_at = Some(ci.created_at);
                    }
                }
            }
            ArchiveEntry::IntraCommunityRite(signed) => {
                let rec = &signed.payload;
                let recitation = &rec.recitation.payload;
                if &recitation.community_did == community_did && rec.sealed_at <= as_of {
                    view.intra_rites.push(IntraRiteSummary {
                        record_cid: cid.clone(),
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
                        record_cid: cid.clone(),
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
                            feature: r.feature.clone(),
                            rationale: r.rationale.clone(),
                        })
                        .collect();
                    view.inter_nonadmissions.push(InterNonAdmissionSummary {
                        record_cid: cid.clone(),
                        decided_at: rec.decided_at,
                        counterparties,
                        rejections,
                    });
                }
            }
            ArchiveEntry::SpeakerDelegated(signed) => {
                let d = &signed.payload;
                if &d.community_did == community_did && d.active_at(as_of) {
                    view.active_delegations.push(DelegationSummary {
                        record_cid: cid.clone(),
                        subject_did: d.speaker_did.clone(),
                        scope: d.scope,
                        active_from: d.issued_at,
                        active_until: d.expires_at,
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
                && e.valid_until.map_or(true, |end| as_of < end);
            if valid_now {
                view.active_witnesses.push(WitnessSummary {
                    record_cid: reg_cid.clone(),
                    member_did: e.witness_did.clone(),
                    role: e.role,
                    active_from: e.valid_from,
                    active_until: e.valid_until,
                });
            }
        }
    }

    view.intra_rites.sort_by_key(|s| s.sealed_at);
    view.standing = derive_standing(&view);
    view
}

fn derive_standing(view: &ReputationView) -> RecognitionStanding {
    let has_identity = view.identity_cid.is_some();
    let has_admission = !view.inter_admissions.is_empty();
    let has_nonadmission = !view.inter_nonadmissions.is_empty();
    if !(has_identity || has_admission || has_nonadmission) {
        return RecognitionStanding::Unknown;
    }
    if has_admission {
        return RecognitionStanding::InCompact;
    }
    if has_nonadmission {
        return RecognitionStanding::Denied;
    }
    RecognitionStanding::Identified
}
