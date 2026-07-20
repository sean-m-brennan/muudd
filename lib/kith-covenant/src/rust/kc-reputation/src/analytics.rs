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

//! Capture-signal detectors over the audit commons.
//!
//! These are the jester's lenses (capture_resistance.md §§2,3,7). Each is an
//! **openly partisan** projection that surfaces *flags with provenance* — CIDs,
//! counts, edges — and renders **no verdict**. Adjudication is the mediator
//! pool's job (jester_and_dissent.md, adversarial-information model:
//! advocacy layer vs. adjudication layer). Every number a detector returns can be traced back to signed
//! records in the archive by the CIDs it carries.
//!
//! The detectors read the archive directly rather than a `ReputationView` so
//! they can range over the whole compact (all communities, all sponsors) in a
//! single pass.

use std::collections::BTreeMap;

use kc_archive::{ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::AntiSpiralFeature;
use kc_conduct::ConductBearing;
use kc_core::{time, Cid, DidKey, Timestamp};

/// One sponsor→entrant edge as of the query time, carrying the record CID so
/// any flag drawn from the graph is traceable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SponsorEdge {
    pub record_cid: Cid,
    pub sponsor: DidKey,
    pub entrant: DidKey,
    pub sponsored_at: Timestamp,
    pub probation_until: Timestamp,
}

/// The sponsorship graph, plus a derived per-sponsor out-degree. Composition
/// drift shows up as concentration: a small set of sponsors accounting for a
/// growing share of admissions (capture_resistance.md §7). This detector does
/// not decide *whether* concentration is capture; it makes it countable.
#[derive(Debug, Clone, Default)]
pub struct CompositionGraph {
    pub edges: Vec<SponsorEdge>,
    /// Sponsor DID → number of entrants it sponsored (as-of the query).
    pub out_degree: BTreeMap<String, usize>,
}

impl CompositionGraph {
    /// Sponsors ordered by descending out-degree — the concentration ranking a
    /// composition-drift dashboard would head with.
    pub fn by_concentration(&self) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> =
            self.out_degree.iter().map(|(k, n)| (k.clone(), *n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}

/// Build the sponsorship graph over the whole archive, as of `as_of`
/// (defaults to now).
pub fn composition_graph(archive: &InMemoryArchive, as_of: Option<Timestamp>) -> CompositionGraph {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut graph = CompositionGraph::default();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::SponsorshipVouched(signed) = entry {
            let s = &signed.payload;
            if s.sponsored_at > as_of {
                continue;
            }
            graph.edges.push(SponsorEdge {
                record_cid: cid.clone(),
                sponsor: s.sponsor_community.clone(),
                entrant: s.entrant_community.clone(),
                sponsored_at: s.sponsored_at,
                probation_until: s.probation_until,
            });
            *graph
                .out_degree
                .entry(s.sponsor_community.as_str().to_string())
                .or_insert(0) += 1;
        }
    }
    graph.edges.sort_by(|a, b| a.sponsored_at.cmp(&b.sponsored_at));
    graph
}

/// One entrant a sponsor vouched for, with the count of adverse conduct
/// attributed to that entrant *inside its probationary window*.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SponseeOutcome {
    pub entrant: DidKey,
    pub sponsorship_cid: Cid,
    pub sponsored_at: Timestamp,
    pub probation_until: Timestamp,
    /// CIDs of adverse (`Concern`/`Violation`/`DesignationAnathema`) conduct
    /// records against the entrant recorded within its probation window
    /// `[sponsored_at, probation_until)`.
    pub adverse_conduct: Vec<Cid>,
}

/// The sponsor-stake report: "sponsorship that repeatedly precedes misconduct
/// is itself a capture signature" (capture_resistance.md §7). The report ties a
/// sponsor to the in-probation conduct of everyone it vouched for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SponsorStakeReport {
    pub sponsor: DidKey,
    pub sponsees: Vec<SponseeOutcome>,
}

impl SponsorStakeReport {
    pub fn total_sponsees(&self) -> usize {
        self.sponsees.len()
    }
    /// Sponsees who accrued at least one adverse conduct record in probation.
    pub fn tainted_sponsees(&self) -> usize {
        self.sponsees.iter().filter(|s| !s.adverse_conduct.is_empty()).count()
    }
    /// Fraction of sponsees that turned adverse in probation, in `[0, 1]`.
    /// `None` when the sponsor has vouched for no one (avoids a 0/0 verdict).
    pub fn taint_rate(&self) -> Option<f64> {
        let n = self.total_sponsees();
        if n == 0 {
            None
        } else {
            Some(self.tainted_sponsees() as f64 / n as f64)
        }
    }
}

/// Compute the sponsor-stake report for `sponsor` as of `as_of`.
pub fn sponsor_stake(
    archive: &InMemoryArchive,
    sponsor: &DidKey,
    as_of: Option<Timestamp>,
) -> SponsorStakeReport {
    let as_of = as_of.unwrap_or_else(time::now);

    // Collect this sponsor's edges.
    let mut sponsees: Vec<SponseeOutcome> = vec![];
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::SponsorshipVouched(signed) = entry {
            let s = &signed.payload;
            if &s.sponsor_community == sponsor && s.sponsored_at <= as_of {
                sponsees.push(SponseeOutcome {
                    entrant: s.entrant_community.clone(),
                    sponsorship_cid: cid.clone(),
                    sponsored_at: s.sponsored_at,
                    probation_until: s.probation_until,
                    adverse_conduct: vec![],
                });
            }
        }
    }

    // For each sponsee, find adverse conduct recorded within its probation
    // window (and not after the query time).
    for outcome in &mut sponsees {
        for (cid, entry) in archive.iter() {
            if let ArchiveEntry::ConductAttested(signed) = entry {
                let c = &signed.payload;
                let in_window = c.recorded_at <= as_of
                    && c.recorded_at >= outcome.sponsored_at
                    && c.recorded_at < outcome.probation_until;
                if c.subject_community == outcome.entrant && c.kind.is_adverse() && in_window {
                    outcome.adverse_conduct.push(cid.clone());
                }
            }
        }
    }

    sponsees.sort_by(|a, b| a.entrant.as_str().cmp(b.entrant.as_str()));
    SponsorStakeReport { sponsor: sponsor.clone(), sponsees }
}

/// Adverse-conduct counts for one community, broken out by which anti-spiral
/// feature the conduct bore on. The behavioral shadow of a hollow or absent
/// substrate: a community that professes the five features but keeps failing
/// (say) `NonUltimacyOfCompact` and `EnemyLoveOperational` in observable
/// conduct — exactly what the substrate-deference test predicts of a community
/// whose substrate is declared, not held.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureConcernProfile {
    /// Per-feature count of adverse conduct records bearing on that feature.
    pub per_feature: BTreeMap<String, usize>,
    /// Adverse records whose bearing is not one of the five features.
    pub non_feature_adverse: usize,
    /// CIDs of every adverse record counted, for traceability.
    pub evidence: Vec<Cid>,
    /// Distinct declared reporter perspectives among the counted records, so a
    /// reader can tell a broad signal from one partisan drumbeat.
    pub perspectives: Vec<String>,
}

fn feature_key(f: &AntiSpiralFeature) -> &'static str {
    match f {
        AntiSpiralFeature::FallenActorAnthropology => "FallenActorAnthropology",
        AntiSpiralFeature::GraceOverMerit => "GraceOverMerit",
        AntiSpiralFeature::ScapegoatDisarming => "ScapegoatDisarming",
        AntiSpiralFeature::EnemyLoveOperational => "EnemyLoveOperational",
        AntiSpiralFeature::NonUltimacyOfCompact => "NonUltimacyOfCompact",
    }
}

/// Compute the per-feature adverse-conduct profile for `community` as of `as_of`.
pub fn feature_concern_profile(
    archive: &InMemoryArchive,
    community: &DidKey,
    as_of: Option<Timestamp>,
) -> FeatureConcernProfile {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut profile = FeatureConcernProfile::default();
    let mut perspectives: std::collections::BTreeSet<String> = Default::default();

    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::ConductAttested(signed) = entry {
            let c = &signed.payload;
            if &c.subject_community != community || c.recorded_at > as_of || !c.kind.is_adverse() {
                continue;
            }
            match &c.bearing {
                ConductBearing::AntiSpiral(f) => {
                    *profile.per_feature.entry(feature_key(f).to_string()).or_insert(0) += 1;
                }
                _ => profile.non_feature_adverse += 1,
            }
            profile.evidence.push(cid.clone());
            if let Some(p) = &c.reporter_perspective {
                perspectives.insert(p.clone());
            }
        }
    }
    profile.perspectives = perspectives.into_iter().collect();
    profile
}

// ---------------------------------------------------------------------------
// v0 jester lenses (read-only, over the existing projection).
//
// Two dashboard-facing views the jester's advocacy layer would head with. Like
// the detectors above, each surfaces flags with provenance and renders no
// verdict: adjudication is the mediator pool's job.
// ---------------------------------------------------------------------------

/// One standing rejection lifted out of a non-admission record, carrying the
/// record CID so any count drawn from it is traceable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectionEvent {
    pub non_admission_cid: Cid,
    pub rejecting: DidKey,
    pub presenting: DidKey,
    pub feature: AntiSpiralFeature,
    pub decided_at: Timestamp,
}

/// Compact-wide rejection tallies. Admissions integrity is the credentialing
/// panel's exposure (capture_resistance.md §7: the panel is the most capturable
/// object), so who rejects whom, and on which anti-spiral feature, is worth
/// making countable. Concentration is surfaced here, not judged.
#[derive(Debug, Clone, Default)]
pub struct RejectionPattern {
    /// Every rejection across the compact, oldest first.
    pub events: Vec<RejectionEvent>,
    /// Rejecting community DID → rejections it issued.
    pub by_rejector: BTreeMap<String, usize>,
    /// Presenting community DID → times it was rejected.
    pub by_presenting: BTreeMap<String, usize>,
    /// Anti-spiral feature → rejections citing it as grounds.
    pub by_feature: BTreeMap<String, usize>,
    /// Distinct rejecting communities, so a reader can tell a broad pattern
    /// from one community's drumbeat.
    pub perspectives: Vec<String>,
}

impl RejectionPattern {
    pub fn total_rejections(&self) -> usize {
        self.events.len()
    }
    /// Rejecting communities ordered by descending rejection count — the
    /// concentration ranking an admissions-integrity dashboard would head with.
    pub fn by_concentration(&self) -> Vec<(String, usize)> {
        let mut v: Vec<(String, usize)> =
            self.by_rejector.iter().map(|(k, n)| (k.clone(), *n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}

/// Build the compact-wide rejection pattern as of `as_of` (defaults to now).
pub fn rejection_pattern(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> RejectionPattern {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut pattern = RejectionPattern::default();
    let mut perspectives: std::collections::BTreeSet<String> = Default::default();

    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::InterCommunityNonAdmission(signed) = entry {
            let rec = &signed.payload;
            if rec.decided_at > as_of {
                continue;
            }
            for r in &rec.rejections {
                pattern.events.push(RejectionEvent {
                    non_admission_cid: cid.clone(),
                    rejecting: r.rejecting_community.clone(),
                    presenting: r.presenting_community.clone(),
                    feature: r.feature.clone(),
                    decided_at: rec.decided_at,
                });
                *pattern
                    .by_rejector
                    .entry(r.rejecting_community.as_str().to_string())
                    .or_insert(0) += 1;
                *pattern
                    .by_presenting
                    .entry(r.presenting_community.as_str().to_string())
                    .or_insert(0) += 1;
                *pattern
                    .by_feature
                    .entry(feature_key(&r.feature).to_string())
                    .or_insert(0) += 1;
                perspectives.insert(r.rejecting_community.as_str().to_string());
            }
        }
    }
    pattern.events.sort_by(|a, b| a.decided_at.cmp(&b.decided_at));
    pattern.perspectives = perspectives.into_iter().collect();
    pattern
}

/// One intra rite reduced to its cost proxies, with the record CID for
/// traceability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RitePoint {
    pub record_cid: Cid,
    pub sealed_at: Timestamp,
    pub witness_count: usize,
}

/// Per-community view of recognition-ceremony cost over time. The corpus makes
/// ceremony cost load-bearing (in-person, witnessed, repeated); this lens makes
/// the erosion of that cost visible as a trend: declining witness
/// participation, lengthening re-affirmation gaps, an overdue re-affirmation.
/// It reports the series and mechanical deltas and renders no verdict on
/// whether the decay is capture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CeremonyCostDecay {
    /// This community's sealed intra rites, oldest first.
    pub rites: Vec<RitePoint>,
    /// Successive `sealed_at` deltas in seconds (len == rites.len() - 1 when
    /// there are rites, else empty).
    pub gaps_seconds: Vec<i64>,
    /// Seconds from the last rite to `as_of` (staleness). `None` with no rites.
    pub seconds_since_last: Option<i64>,
}

impl CeremonyCostDecay {
    pub fn rite_count(&self) -> usize {
        self.rites.len()
    }
    pub fn first_witness_count(&self) -> Option<usize> {
        self.rites.first().map(|r| r.witness_count)
    }
    pub fn last_witness_count(&self) -> Option<usize> {
        self.rites.last().map(|r| r.witness_count)
    }
    pub fn min_witness_count(&self) -> Option<usize> {
        self.rites.iter().map(|r| r.witness_count).min()
    }
    pub fn max_witness_count(&self) -> Option<usize> {
        self.rites.iter().map(|r| r.witness_count).max()
    }
    /// True when the most recent rite drew fewer witnesses than the first.
    /// Meaningful only with at least two rites; `false` otherwise so no signal
    /// is fabricated from a single data point.
    pub fn witness_count_declined(&self) -> bool {
        match (self.first_witness_count(), self.last_witness_count()) {
            (Some(f), Some(l)) if self.rites.len() >= 2 => l < f,
            _ => false,
        }
    }
    /// `Some(true)` when the latest re-affirmation gap exceeds the median of the
    /// prior gaps. Needs at least two gaps (three rites); `None` otherwise,
    /// since there is not enough cadence to compare.
    pub fn latest_gap_widening(&self) -> Option<bool> {
        if self.gaps_seconds.len() < 2 {
            return None;
        }
        let last = *self.gaps_seconds.last().unwrap();
        let mut prior: Vec<i64> = self.gaps_seconds[..self.gaps_seconds.len() - 1].to_vec();
        prior.sort_unstable();
        let m = prior.len();
        let median = if m % 2 == 1 {
            prior[m / 2] as f64
        } else {
            (prior[m / 2 - 1] + prior[m / 2]) as f64 / 2.0
        };
        Some((last as f64) > median)
    }
}

/// Compute the ceremony-cost-decay series for `community` as of `as_of`.
pub fn ceremony_cost_decay(
    archive: &InMemoryArchive,
    community: &DidKey,
    as_of: Option<Timestamp>,
) -> CeremonyCostDecay {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut decay = CeremonyCostDecay::default();

    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::IntraCommunityRite(signed) = entry {
            let rec = &signed.payload;
            let recitation = &rec.recitation.payload;
            if &recitation.community_did == community && rec.sealed_at <= as_of {
                decay.rites.push(RitePoint {
                    record_cid: cid.clone(),
                    sealed_at: rec.sealed_at,
                    witness_count: rec.internal_witnesses.len(),
                });
            }
        }
    }
    decay.rites.sort_by_key(|r| r.sealed_at);
    for w in decay.rites.windows(2) {
        decay
            .gaps_seconds
            .push(w[1].sealed_at.as_second() - w[0].sealed_at.as_second());
    }
    decay.seconds_since_last = decay
        .rites
        .last()
        .map(|r| as_of.as_second() - r.sealed_at.as_second());
    decay
}
