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

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use kc_archive::{probationary_at, ArchiveEntry, InMemoryArchive};
use kc_ceremony::inter::AntiSpiralFeature;
use kc_commons::sanction_rank;
use kc_conduct::{AttestedFunction, ConductBearing, ConductKind};
use kc_constitution::{AmendmentKind, FastPeriod, Forbearance, ForbearanceChange, IncapableMatter};
use kc_core::{time, Cid, DidKey, Timestamp};
use kc_escalation::{dispute_ladder_rank, ReexaminationOutcome};
use kc_mediation::MediatorRegistry;
use kc_observer::ObserverRegistry;

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
                record_cid: *cid,
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
    graph.edges.sort_by_key(|e| e.sponsored_at);
    graph
}

// ---------------------------------------------------------------------------
// Move 1 interpretive-plurality lens (over a community's substrate readings).
// ---------------------------------------------------------------------------

/// One published interpretation as of the query, carrying the record CID so any
/// plurality/monopoly flag is traceable back to a signed record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineagePoint {
    pub record_cid: Cid,
    pub lineage_id: String,
    pub interpreter: DidKey,
    pub interprets_cid: Option<Cid>,
    pub recorded_at: Timestamp,
}

/// A member community's interpretive lineages of its own substrate, plus the
/// per-lineage and per-interpreter counts (capture_resistance.md move 1). The
/// capture signature is a *monopoly*: a single interpreter or a single lineage
/// cornering the interpretive function so substrate-deference quietly becomes
/// faction-deference. This lens makes plurality (and its absence) countable and
/// renders **no verdict** — a low count may be a young community, not a captured
/// one, so the monopoly threshold lives with the caller (the §10 harness).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InterpretivePluralityReport {
    /// The community's interpretations, oldest first (traceable).
    pub interpretations: Vec<LineagePoint>,
    /// lineage_id → number of interpretations under it, as of the query.
    pub lineages: BTreeMap<String, usize>,
    /// interpreter DID → number of interpretations it authored.
    pub by_interpreter: BTreeMap<String, usize>,
    /// interpreter DID → the distinct third parties (≠ the community, ≠ that
    /// interpreter) who attested its independence (capture move 1, anti-sockpuppet).
    /// Only interpreters that actually authored a reading appear. An interpreter
    /// with a non-empty set here is *independent*; a declared-only interpreter with
    /// no outside attester is a possible sockpuppet.
    pub independent_attesters: BTreeMap<String, BTreeSet<String>>,
}

impl InterpretivePluralityReport {
    /// Number of distinct interpretive lineages.
    pub fn distinct_lineages(&self) -> usize {
        self.lineages.len()
    }
    /// Number of distinct interpreters — the anti-monopoly count.
    pub fn distinct_interpreters(&self) -> usize {
        self.by_interpreter.len()
    }
    /// Interpreters ordered by descending share of interpretations — the
    /// concentration ranking a monopoly heads.
    pub fn interpreter_concentration(&self) -> Vec<(String, usize)> {
        descending(self.by_interpreter.clone())
    }
    /// The positive signal move 1 asks a community to demonstrate: more than one
    /// lineage AND more than one interpreter. Its absence is not proof of
    /// capture (the caller owns the monopoly threshold), only its absence. Counts
    /// *declared* interpreters — see `independent_plurality_demonstrated` for the
    /// anti-sockpuppet form.
    pub fn plurality_demonstrated(&self) -> bool {
        self.distinct_lineages() >= 2 && self.distinct_interpreters() >= 2
    }
    /// Number of interpreters whose independence a distinct third party has
    /// attested — the anti-sockpuppet count. A faction minting interpreter
    /// identities it controls inflates `distinct_interpreters` but not this.
    pub fn distinct_independent_interpreters(&self) -> usize {
        self.independent_attesters.values().filter(|a| !a.is_empty()).count()
    }
    /// The anti-sockpuppet upgrade of `plurality_demonstrated`: at least two
    /// lineages AND at least two interpreters whose independence a distinct third
    /// party has attested. Declared-only plurality no longer suffices, so a single
    /// faction cannot demonstrate plurality with identities it wholly controls.
    pub fn independent_plurality_demonstrated(&self) -> bool {
        self.distinct_lineages() >= 2 && self.distinct_independent_interpreters() >= 2
    }
}

/// Build the interpretive-plurality report for `community` as of `as_of`
/// (default now), over the `SubstrateInterpreted` records whose subject is that
/// community. Distinctness is keyed on the interpreter DID and lineage id, never
/// on record count, so a prolific single interpreter cannot fake plurality.
pub fn interpretive_plurality(
    archive: &InMemoryArchive,
    community: &DidKey,
    as_of: Option<Timestamp>,
) -> InterpretivePluralityReport {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut report = InterpretivePluralityReport::default();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::SubstrateInterpreted(signed) = entry {
            let s = &signed.payload;
            if &s.community_did != community || s.recorded_at > as_of {
                continue;
            }
            report.interpretations.push(LineagePoint {
                record_cid: *cid,
                lineage_id: s.lineage_id.clone(),
                interpreter: s.interpreter_did.clone(),
                interprets_cid: s.interprets_cid,
                recorded_at: s.recorded_at,
            });
            *report.lineages.entry(s.lineage_id.clone()).or_insert(0) += 1;
            *report
                .by_interpreter
                .entry(s.interpreter_did.as_str().to_string())
                .or_insert(0) += 1;
        }
    }
    // Second pass: independence attestations. An attester counts only when it is
    // neither the community nor the interpreter (no self-vouching), and only for
    // an interpreter that actually authored a reading — an attestation for a
    // never-seen interpreter cannot manufacture independence.
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::InterpreterIndependenceAttested(signed) = entry {
            let a = &signed.payload;
            if &a.community_did != community || a.attested_at > as_of {
                continue;
            }
            let interpreter = a.interpreter_did.as_str().to_string();
            let attester = a.attesting_party.as_str();
            if attester == community.as_str()
                || attester == a.interpreter_did.as_str()
                || !report.by_interpreter.contains_key(&interpreter)
            {
                continue;
            }
            report
                .independent_attesters
                .entry(interpreter)
                .or_default()
                .insert(attester.to_string());
        }
    }
    report.interpretations.sort_by_key(|i| i.recorded_at);
    report
}

// ---------------------------------------------------------------------------
// Move 4 structured-skepticism lens (over event reports + corroborations).
// ---------------------------------------------------------------------------

/// One event's provenance summary as of the query, carrying the report CID so
/// any under-corroboration flag is traceable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventProvenance {
    pub event_cid: Cid,
    pub originator: DidKey,
    pub reported_at: Timestamp,
    /// Does the event meet the structured-skepticism criteria?
    pub qualifies: bool,
    /// Which qualifying triggers fired, for the reader.
    pub triggers: Vec<String>,
    /// Are the provenance tags (declared incentives, information history, and a
    /// stake-holding originator's disclosure) present?
    pub provenance_complete: bool,
    /// Distinct communities that corroborated the report, excluding the
    /// originator (self-corroboration does not count).
    pub corroborators: Vec<DidKey>,
    /// The subset of `corroborators` that supplied a **cross-substrate
    /// translation** of the interpretation. An interpretation that survives only
    /// in the originating framing (corroborated, but never translated into another
    /// community's substrate) is structurally suspect — the manufacturer may have
    /// engineered a reading that only coheres in its own terms.
    pub translated_corroborators: Vec<DidKey>,
    /// Retroactive findings that the originator held an interest it did not
    /// disclose at intake — `(finder, undisclosed_interest)`, from finders other
    /// than the originator (a self-finding is honest late disclosure, not bad
    /// faith). A non-empty list means the event was authored in bad faith and may
    /// already have driven action before the omission surfaced (move 4).
    pub undisclosed_interests_found: Vec<(DidKey, String)>,
    /// The faction the event is conveniently aligned with, if any.
    pub aligned_faction: Option<DidKey>,
    /// The mediators who invoked the structured-skepticism protocol on this event
    /// (§4), across every invocation. Empty means the pool never invoked, which is
    /// not the same as the event being unremarkable — most events never reach the
    /// pool at all.
    pub invoked_by: Vec<DidKey>,
    /// The criteria the **pool** found met, as distinct from the triggers the
    /// originator declared. Union across invocations.
    pub invoked_criteria: Vec<String>,
    /// The criteria the pool found that the originator did **not** declare. This is
    /// the divergence worth reading: it is the pool contradicting an originator's
    /// account of its own claim, which is the whole reason the invocation is the
    /// pool's and not the originator's.
    pub undeclared_criteria: Vec<String>,
    /// The longest hold any invocation imposed, if any.
    pub invoked_hold_seconds: Option<i64>,
    /// Reviewers who have examined the hold on this event, across every review.
    pub reviewed_by: Vec<DidKey>,
    /// Whether a review found the hold pretextual, releasing the event.
    pub released_as_pretextual: bool,
    /// Whether the event is under the protocol at all — by the originator's own
    /// declaration or by the pool's invocation, and not released. This is what the
    /// archive's hold actually turns on; `qualifies` alone is only the originator's
    /// half of it.
    pub under_protocol: bool,
}

/// Provenance and corroboration status of every event report in the archive
/// (capture_resistance.md move 4; liminal_institutions.md structured-skepticism
/// protocol). The lens makes the protocol *legible* — which qualifying events
/// still lack independent corroboration, so the loudest interpreter has not yet
/// been checked — and renders **no verdict**; the corroboration threshold and
/// any consequent delay are the caller's (the §10 harness / the mediator pool).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkepticismReport {
    /// Events oldest first (traceable).
    pub events: Vec<EventProvenance>,
}

impl SkepticismReport {
    /// Number of events under the structured-skepticism protocol — declared by their
    /// originator or invoked by the pool, and not released.
    ///
    /// This counts `under_protocol` rather than `qualifies`, and every filter below
    /// does the same. Reading the originator's declaration alone would leave the lens
    /// blind to exactly the events the pool's invocation exists to reach: the ones
    /// whose originator declared them mundane.
    pub fn qualifying_events(&self) -> usize {
        self.events.iter().filter(|e| e.under_protocol).count()
    }
    /// The CIDs of events under the protocol with fewer than `min` distinct
    /// corroborators — the loudest-interpreter window still open.
    pub fn under_corroborated(&self, min: usize) -> Vec<Cid> {
        self.events
            .iter()
            .filter(|e| e.under_protocol && e.corroborators.len() < min)
            .map(|e| e.event_cid)
            .collect()
    }
    /// The CIDs of events under a hold that **no panel has reviewed**. §4 makes the
    /// pool answerable for its invocations ("invocations are auditable"), and a hold
    /// nobody has examined is the state that answers to nobody — the one a faction
    /// suppressing an urgent matter would leave the compact in.
    pub fn unreviewed_holds(&self) -> Vec<Cid> {
        self.events
            .iter()
            .filter(|e| e.under_protocol && e.reviewed_by.is_empty())
            .map(|e| e.event_cid)
            .collect()
    }
    /// The CIDs of events where the pool found a criterion the originator did not
    /// declare. Not a verdict on anybody: it is the pool's reading of an event set
    /// against its originator's, which is the disagreement the invocation authority
    /// exists to make possible.
    pub fn declaration_divergences(&self) -> Vec<Cid> {
        self.events
            .iter()
            .filter(|e| !e.undeclared_criteria.is_empty())
            .map(|e| e.event_cid)
            .collect()
    }
    /// The CIDs of qualifying events with fewer than `min` distinct corroborators
    /// supplying a cross-substrate translation — interpretations still surviving
    /// only in the originating framing. Distinct from `under_corroborated`: an
    /// event can be well-corroborated in its native terms yet never translated,
    /// which is its own capture signal (move 4; a manufactured reading that only
    /// coheres in the manufacturer's substrate).
    pub fn under_translated(&self, min: usize) -> Vec<Cid> {
        self.events
            .iter()
            .filter(|e| e.under_protocol && e.translated_corroborators.len() < min)
            .map(|e| e.event_cid)
            .collect()
    }
    /// The CIDs of qualifying events for which a third party has retroactively
    /// found an undisclosed originator interest — events now shown to be authored
    /// in bad faith. Independent of corroboration: a well-corroborated event whose
    /// originator concealed an interest is still impugned, and may already have
    /// driven action (the re-opening trigger move 4 leaves to conduct/escalation).
    pub fn retroactively_impugned(&self) -> Vec<Cid> {
        self.events
            .iter()
            .filter(|e| e.under_protocol && !e.undisclosed_interests_found.is_empty())
            .map(|e| e.event_cid)
            .collect()
    }
}

/// Build the structured-skepticism report over the whole archive as of `as_of`
/// (default now). Corroborators are counted as **distinct communities other
/// than the originator**, keyed by DID, so an originator cannot self-corroborate
/// or a single ally corroborate repeatedly to fake independent support.
/// The pool's invocations on one event, folded: the invoking mediators across all of
/// them, every criterion they found, and the longest hold imposed.
type InvocationTally = (BTreeMap<String, DidKey>, BTreeSet<String>, i64);

/// The reviews of one event's hold, folded: the reviewers, and whether any of them found
/// the hold pretextual.
type ReviewTally = (BTreeMap<String, DidKey>, bool);

pub fn structured_skepticism(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> SkepticismReport {
    let as_of = as_of.unwrap_or_else(time::now);
    // event_cid string → (community DID string → community DidKey), distinct.
    let mut corroborations: BTreeMap<String, BTreeMap<String, DidKey>> = BTreeMap::new();
    // Same, but only for corroborations that supplied a cross-substrate translation.
    let mut translations: BTreeMap<String, BTreeMap<String, DidKey>> = BTreeMap::new();
    // event_cid string → retroactive undisclosed-interest findings (finder, interest).
    let mut findings: BTreeMap<String, Vec<(DidKey, String)>> = BTreeMap::new();
    // event_cid string → what the pool did about that event, and what came of it.
    let mut invocations: BTreeMap<String, InvocationTally> = BTreeMap::new();
    let mut reviews: BTreeMap<String, ReviewTally> = BTreeMap::new();
    let mut events: Vec<(Cid, &kc_provenance::EventReport)> = Vec::new();
    for (cid, entry) in archive.iter() {
        match entry {
            ArchiveEntry::EventReported(signed) => {
                let e = &signed.payload;
                if e.reported_at > as_of {
                    continue;
                }
                events.push((*cid, e));
            }
            ArchiveEntry::EventCorroborated(signed) => {
                let c = &signed.payload;
                if c.corroborated_at > as_of {
                    continue;
                }
                corroborations
                    .entry(c.event_cid.to_string())
                    .or_default()
                    .insert(
                        c.corroborating_community.as_str().to_string(),
                        c.corroborating_community.clone(),
                    );
                if c.translation.is_some() {
                    translations
                        .entry(c.event_cid.to_string())
                        .or_default()
                        .insert(
                            c.corroborating_community.as_str().to_string(),
                            c.corroborating_community.clone(),
                        );
                }
            }
            ArchiveEntry::SkepticismInvoked(signed) => {
                let i = &signed.payload;
                if i.invoked_at > as_of {
                    continue;
                }
                let e = invocations
                    .entry(i.event_cid.to_string())
                    .or_insert_with(|| (BTreeMap::new(), BTreeSet::new(), 0i64));
                for m in &i.panel {
                    e.0.insert(m.as_str().to_string(), m.clone());
                }
                for c in &i.criteria {
                    e.1.insert(c.as_str().to_string());
                }
                e.2 = e.2.max(i.hold_seconds);
            }
            ArchiveEntry::SkepticismReviewed(signed) => {
                let r = &signed.payload;
                if r.reviewed_at > as_of {
                    continue;
                }
                let e = reviews
                    .entry(r.event_cid.to_string())
                    .or_insert_with(|| (BTreeMap::new(), false));
                for m in &r.reviewers {
                    e.0.insert(m.as_str().to_string(), m.clone());
                }
                if r.answer == kc_provenance::ReviewAnswer::Pretextual {
                    e.1 = true;
                }
            }
            ArchiveEntry::RetroactiveInterestFound(signed) => {
                let f = &signed.payload;
                if f.found_at > as_of {
                    continue;
                }
                findings
                    .entry(f.event_cid.to_string())
                    .or_default()
                    .push((f.finder.clone(), f.undisclosed_interest.clone()));
            }
            _ => {}
        }
    }

    // Distinct communities in `map` for `event`, excluding the originator.
    let distinct_others = |map: &BTreeMap<String, BTreeMap<String, DidKey>>,
                           event: &str,
                           originator: &str|
     -> Vec<DidKey> {
        map.get(event)
            .map(|m| {
                m.iter()
                    .filter(|(k, _)| k.as_str() != originator)
                    .map(|(_, d)| d.clone())
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut report = SkepticismReport::default();
    for (cid, e) in events {
        let key = cid.to_string();
        let corroborators = distinct_others(&corroborations, &key, e.originator.as_str());
        let translated_corroborators = distinct_others(&translations, &key, e.originator.as_str());
        // A finding counts only when the finder is not the event's own originator
        // (self-finding is honest late disclosure, not bad faith).
        let undisclosed_interests_found: Vec<(DidKey, String)> = findings
            .get(&key)
            .map(|v| {
                v.iter()
                    .filter(|(finder, _)| finder.as_str() != e.originator.as_str())
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let (invoked_by, invoked_criteria, invoked_hold_seconds) = match invocations.get(&key) {
            None => (Vec::new(), Vec::new(), None),
            Some((panel, criteria, hold)) => (
                panel.values().cloned().collect(),
                criteria.iter().cloned().collect::<Vec<String>>(),
                Some(*hold),
            ),
        };
        let (reviewed_by, released_as_pretextual) = match reviews.get(&key) {
            None => (Vec::new(), false),
            Some((panel, released)) => (panel.values().cloned().collect(), *released),
        };
        let declared: BTreeSet<&str> = e.qualifying_triggers().into_iter().collect();
        let undeclared_criteria: Vec<String> = invoked_criteria
            .iter()
            .filter(|c| !declared.contains(c.as_str()))
            .cloned()
            .collect();
        // What the archive's hold actually turns on: either source places the event
        // under the protocol, and a pretextual finding releases it from either.
        let under_protocol =
            (e.qualifies() || !invoked_by.is_empty()) && !released_as_pretextual;
        report.events.push(EventProvenance {
            event_cid: cid,
            originator: e.originator.clone(),
            reported_at: e.reported_at,
            qualifies: e.qualifies(),
            triggers: e
                .qualifying_triggers()
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            provenance_complete: e.provenance_complete(),
            corroborators,
            translated_corroborators,
            undisclosed_interests_found,
            aligned_faction: e.aligned_faction.clone(),
            invoked_by,
            invoked_criteria,
            undeclared_criteria,
            invoked_hold_seconds,
            reviewed_by,
            released_as_pretextual,
            under_protocol,
        });
    }
    report.events.sort_by_key(|e| e.reported_at);
    report
}

// ---------------------------------------------------------------------------
// Move 6 adversarial-principal lens (over witness attestations + outputs).
// ---------------------------------------------------------------------------

/// A cross-community attestation of a principal's validity as of the query —
/// only those from a witness that is neither the principal nor its home
/// community (the independence move 6 requires).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossWitness {
    pub record_cid: Cid,
    pub witness_community: DidKey,
    pub attested_at: Timestamp,
}

/// A matter on which the principal's decision diverged from its cohort's
/// majority, carrying the record CID and whether the divergence benefits the
/// principal's own side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDivergence {
    pub output_cid: Cid,
    pub cohort: String,
    pub matter: String,
    pub principal_decision: String,
    pub cohort_majority: String,
    /// Distinct peer principals compared on this matter.
    pub peers_compared: usize,
    pub favors_home: bool,
}

/// Adversarial-principal validation for a principal (capture_resistance.md move
/// 6; m2m_interaction.md). Two independent signals, neither a verdict:
/// **cross-community witness** count (validity that does not rest on the
/// principal's own or its home community's attestation) and **output
/// triangulation** against similarly-situated principals (divergence toward the
/// principal's own benefit). The caller decides what count/divergence is
/// actionable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrincipalValidationReport {
    pub cross_witnesses: Vec<CrossWitness>,
    pub divergences: Vec<OutputDivergence>,
    /// The principal's own outputs that had a peer cohort to triangulate against —
    /// the denominator for the continuous-behavior rate below. A single divergence
    /// among many aligned outputs is noise; a divergence among few is not.
    pub outputs_triangulated: usize,
}

impl PrincipalValidationReport {
    /// Distinct communities (other than the principal / its home) that have
    /// attested the principal — the independent-validation count.
    pub fn distinct_cross_witnesses(&self) -> usize {
        self.cross_witnesses
            .iter()
            .map(|w| w.witness_community.as_str())
            .collect::<BTreeSet<_>>()
            .len()
    }
    /// Divergences in a direction that benefits the principal's own side — the
    /// per-matter capture signature.
    pub fn self_benefiting_divergences(&self) -> usize {
        self.divergences.iter().filter(|d| d.favors_home).count()
    }
    /// Statistical triangulation of *continuous behavior*: the fraction of the
    /// principal's triangulated outputs that diverged from its cohort toward its
    /// own benefit. One divergence may be honest disagreement; a high sustained
    /// rate is the capture signature a single flag cannot show. `0.0` when nothing
    /// has been triangulated.
    pub fn self_dealing_rate(&self) -> f64 {
        if self.outputs_triangulated == 0 {
            return 0.0;
        }
        self.self_benefiting_divergences() as f64 / self.outputs_triangulated as f64
    }
    /// Whether the principal shows *systematic* self-dealing: enough triangulated
    /// outputs to be meaningful (`min_outputs`) and a self-dealing rate at or above
    /// `rate_threshold`. Both bounds are the caller's policy; the lens renders no
    /// verdict.
    pub fn systematic_self_dealing(&self, min_outputs: usize, rate_threshold: f64) -> bool {
        self.outputs_triangulated >= min_outputs && self.self_dealing_rate() >= rate_threshold
    }
}

/// Build the adversarial-principal report for `principal` as of `as_of`
/// (default now). Cross-witnesses count only communities other than the
/// principal and its home; triangulation compares the principal's decision on
/// each (cohort, matter) against the majority among **distinct peer
/// principals** (deduped by DID, latest output per peer), so one prolific peer
/// cannot swing the ensemble.
pub fn adversarial_principal(
    archive: &InMemoryArchive,
    principal: &DidKey,
    as_of: Option<Timestamp>,
) -> PrincipalValidationReport {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut report = PrincipalValidationReport::default();

    // (cohort, matter) → peer principal DID → (latest decision, produced_at).
    let mut peers: BTreeMap<(String, String), BTreeMap<String, (String, Timestamp)>> =
        BTreeMap::new();
    // The principal's own outputs: (cid, cohort, matter, decision, favors_home).
    let mut own: Vec<(Cid, String, String, String, bool)> = Vec::new();

    for (cid, entry) in archive.iter() {
        match entry {
            ArchiveEntry::PrincipalWitnessed(signed) => {
                let a = &signed.payload;
                if a.attested_at > as_of || &a.principal_did != principal {
                    continue;
                }
                if a.witness_community.as_str() != principal.as_str()
                    && a.witness_community.as_str() != a.principal_home.as_str()
                {
                    report.cross_witnesses.push(CrossWitness {
                        record_cid: *cid,
                        witness_community: a.witness_community.clone(),
                        attested_at: a.attested_at,
                    });
                }
            }
            ArchiveEntry::PrincipalOutputRecorded(signed) => {
                let o = &signed.payload;
                if o.produced_at > as_of {
                    continue;
                }
                if &o.principal_did == principal {
                    own.push((
                        *cid,
                        o.cohort.clone(),
                        o.matter.clone(),
                        o.decision.clone(),
                        o.favors_home,
                    ));
                } else {
                    let slot = peers
                        .entry((o.cohort.clone(), o.matter.clone()))
                        .or_default()
                        .entry(o.principal_did.as_str().to_string())
                        .or_insert((o.decision.clone(), o.produced_at));
                    if o.produced_at >= slot.1 {
                        *slot = (o.decision.clone(), o.produced_at);
                    }
                }
            }
            _ => {}
        }
    }

    for (cid, cohort, matter, decision, favors_home) in own {
        let Some(peer_map) = peers.get(&(cohort.clone(), matter.clone())) else {
            continue;
        };
        if peer_map.is_empty() {
            continue;
        }
        // This output had a cohort to triangulate against (whether or not it
        // diverged) — the denominator for the continuous-behavior rate.
        report.outputs_triangulated += 1;
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for (d, _) in peer_map.values() {
            *counts.entry(d.clone()).or_insert(0) += 1;
        }
        // Highest count; ties broken by the lexically smallest decision.
        let majority = counts
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
            .map(|(k, _)| k)
            .unwrap();
        if decision != majority {
            report.divergences.push(OutputDivergence {
                output_cid: cid,
                cohort,
                matter,
                principal_decision: decision,
                cohort_majority: majority,
                peers_compared: peer_map.len(),
                favors_home,
            });
        }
    }

    report.cross_witnesses.sort_by_key(|w| w.attested_at);
    report.divergences.sort_by(|a, b| a.matter.cmp(&b.matter));
    report
}

/// A cohort whose majority position on a shared matter diverges from the
/// cross-cohort consensus, carrying whether that bloc position benefits the
/// cohort's own members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CohortDivergence {
    pub cohort: String,
    pub matter: String,
    pub cohort_majority: String,
    pub cross_cohort_consensus: String,
    /// Distinct cohorts that decided this matter (>= 2 for a comparison to exist).
    pub cohorts_compared: usize,
    /// The cohort's majority position is one a strict majority of its members
    /// marked as favoring their own home — a self-serving bloc.
    pub self_benefiting: bool,
}

/// Cross-cohort triangulation for **collusive whole-cohort capture**
/// (capture_resistance.md move 6). Per-principal triangulation
/// (`adversarial_principal`) compares a principal against its *own* cohort's
/// majority, so a cohort that colludes wholesale shows no internal divergence and
/// slips through. This lens lifts the comparison a level: for each matter decided
/// by two or more cohorts it computes each cohort's majority and the cross-cohort
/// consensus (the majority among the cohort-majorities), and surfaces cohorts that
/// diverge — self-serving divergence being the capture signature. No verdict; the
/// caller owns the threshold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CohortCollusionReport {
    pub divergences: Vec<CohortDivergence>,
}

impl CohortCollusionReport {
    /// Cohort divergences whose bloc position benefits the cohort's own members —
    /// the whole-cohort capture signature per-principal triangulation is blind to.
    pub fn self_benefiting_cohort_divergences(&self) -> usize {
        self.divergences.iter().filter(|d| d.self_benefiting).count()
    }
}

/// Highest-count key in `counts`; ties broken by the lexically smallest key (the
/// same rule the per-principal triangulation uses).
fn plurality(counts: &BTreeMap<String, usize>) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(k, _)| k.clone())
}

/// One principal's deduped output within a `(cohort, matter)`: the decision it
/// produced, whether that decision favored its home community, and when.
type PrincipalDecision = (String, bool, Timestamp);

/// matter → cohort → principal DID → its latest decision.
///
/// Three levels of `BTreeMap` keyed by `String`, which is the shape the dedup rule
/// needs: latest-per-principal, within a cohort, within a matter. Named because the
/// nesting order is the rule — transposing cohort and matter would dedup across the
/// wrong scope and still compile.
type CohortDecisionIndex = BTreeMap<String, BTreeMap<String, BTreeMap<String, PrincipalDecision>>>;

/// Build the collusive-cohort report over the whole archive as of `as_of`
/// (default now). Outputs are deduped to the latest per distinct principal within
/// each `(cohort, matter)`, so one prolific machine cannot swing its cohort's
/// majority.
pub fn collusive_cohorts(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> CohortCollusionReport {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut data: CohortDecisionIndex = BTreeMap::new();

    for (_, entry) in archive.iter() {
        if let ArchiveEntry::PrincipalOutputRecorded(signed) = entry {
            let o = &signed.payload;
            if o.produced_at > as_of {
                continue;
            }
            let slot = data
                .entry(o.matter.clone())
                .or_default()
                .entry(o.cohort.clone())
                .or_default()
                .entry(o.principal_did.as_str().to_string())
                .or_insert((o.decision.clone(), o.favors_home, o.produced_at));
            if o.produced_at >= slot.2 {
                *slot = (o.decision.clone(), o.favors_home, o.produced_at);
            }
        }
    }

    let mut report = CohortCollusionReport::default();
    for (matter, cohorts) in &data {
        if cohorts.len() < 2 {
            continue; // nothing to triangulate across
        }
        // Each cohort's majority decision + whether that bloc is self-serving.
        let mut cohort_major: BTreeMap<String, (String, bool)> = BTreeMap::new();
        for (cohort, principals) in cohorts {
            let total = principals.len();
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            let mut fav_counts: BTreeMap<String, usize> = BTreeMap::new();
            for (decision, favors_home, _) in principals.values() {
                *counts.entry(decision.clone()).or_insert(0) += 1;
                if *favors_home {
                    *fav_counts.entry(decision.clone()).or_insert(0) += 1;
                }
            }
            let Some(majority) = plurality(&counts) else { continue };
            let self_benefiting = fav_counts.get(&majority).copied().unwrap_or(0) * 2 > total;
            cohort_major.insert(cohort.clone(), (majority, self_benefiting));
        }
        // Cross-cohort consensus: the majority among the cohort majorities.
        let mut consensus_counts: BTreeMap<String, usize> = BTreeMap::new();
        for (maj, _) in cohort_major.values() {
            *consensus_counts.entry(maj.clone()).or_insert(0) += 1;
        }
        let Some(consensus) = plurality(&consensus_counts) else { continue };
        let cohorts_compared = cohort_major.len();
        for (cohort, (maj, self_benefiting)) in &cohort_major {
            if maj != &consensus {
                report.divergences.push(CohortDivergence {
                    cohort: cohort.clone(),
                    matter: matter.clone(),
                    cohort_majority: maj.clone(),
                    cross_cohort_consensus: consensus.clone(),
                    cohorts_compared,
                    self_benefiting: *self_benefiting,
                });
            }
        }
    }
    report
        .divergences
        .sort_by(|a, b| a.matter.cmp(&b.matter).then_with(|| a.cohort.cmp(&b.cohort)));
    report
}

/// One link in a principal's witness-attestation chain, as of the query. Carries
/// its own entry `cid`, the `prev` link it declares, whether that `prev` resolves
/// to another attestation of the *same* principal present in the chain
/// (`prev_resolved`), and whether the link is **cross-community** — signed by a
/// witness that is neither the principal nor its home community, the signature
/// the home cannot produce and so the source of the chain's separability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitnessLink {
    pub cid: Cid,
    pub witness_community: DidKey,
    pub prev: Option<Cid>,
    pub attested_at: Timestamp,
    /// The declared `prev` names another link in this principal's chain.
    pub prev_resolved: bool,
    /// Signed by a community other than the principal and its home — separable.
    pub cross_community: bool,
}

/// The witness-attestation chain for a principal (capture_resistance.md move 6,
/// "a separate attestation chain ... cryptographically separable from the
/// within-community attestation"). The links are the principal's attestations in
/// chain order; the `prev` pointers make the sequence tamper-evident — each entry
/// CID commits its full content (including `prev`), so altering or reordering a
/// link dangles every pointer after it. The report exposes chain **integrity**
/// (no dangling `prev`, no forks) and **separability** (the count of
/// cross-community links, which `Signed::verify_all` proves were signed by keys
/// the home community does not hold). No verdict; the caller owns the threshold.
///
/// Honest boundary: this does **not** defeat shared-key sockpuppets. A witness
/// colluding with the home, or a home secretly controlling a second key, still
/// produces a link that looks "cross-community." That is move-1
/// third-party-verification territory; the chain proves only that *distinct keys*
/// attested and that the sequence is unbroken — not that the keys are
/// independent principals.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WitnessChainReport {
    /// The principal's attestation links, oldest first.
    pub links: Vec<WitnessLink>,
}

impl WitnessChainReport {
    /// Links signed by a community other than the principal / its home — the
    /// cryptographically separable attestations the home cannot forge.
    pub fn separable_links(&self) -> usize {
        self.links.iter().filter(|l| l.cross_community).count()
    }
    /// Links whose declared `prev` does not resolve to another link in the chain
    /// — a dangling pointer, evidence of a removed or tampered predecessor.
    pub fn broken_links(&self) -> usize {
        self.links.iter().filter(|l| l.prev.is_some() && !l.prev_resolved).count()
    }
    /// A chain is **linear** when at most one link is a root (declares no `prev`)
    /// and no two links claim the same predecessor (no fork).
    pub fn is_linear(&self) -> bool {
        let roots = self.links.iter().filter(|l| l.prev.is_none()).count();
        if roots > 1 {
            return false;
        }
        let mut seen: BTreeSet<[u8; 32]> = BTreeSet::new();
        for l in &self.links {
            if let Some(p) = &l.prev {
                if !seen.insert(*p.as_bytes()) {
                    return false; // two links claim the same predecessor
                }
            }
        }
        true
    }
    /// The chain is intact when no `prev` dangles and the chain is linear.
    pub fn is_intact(&self) -> bool {
        self.broken_links() == 0 && self.is_linear()
    }
}

/// Build the witness-attestation chain for `principal` as of `as_of` (default
/// now). Links are the principal's `PrincipalWitnessAttestation` entries in
/// attestation order (ties broken by entry CID for stability). A link's `prev` is
/// *resolved* when it names another of this principal's links, and *separable*
/// (cross-community) when the witness is neither the principal nor its home.
pub fn principal_witness_chain(
    archive: &InMemoryArchive,
    principal: &DidKey,
    as_of: Option<Timestamp>,
) -> WitnessChainReport {
    let as_of = as_of.unwrap_or_else(time::now);

    // Collect this principal's attestation links (CID + payload).
    let mut raw: Vec<(Cid, kc_principal::PrincipalWitnessAttestation)> = Vec::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::PrincipalWitnessed(signed) = entry {
            let a = &signed.payload;
            if a.attested_at <= as_of && &a.principal_did == principal {
                raw.push((*cid, a.clone()));
            }
        }
    }
    // The set of CIDs present in the chain, so a `prev` can be resolved.
    let present: BTreeSet<[u8; 32]> = raw.iter().map(|(c, _)| *c.as_bytes()).collect();

    let mut report = WitnessChainReport::default();
    for (cid, a) in raw {
        let prev_resolved = a
            .prev
            .as_ref()
            .map(|p| present.contains(p.as_bytes()))
            .unwrap_or(false);
        let cross_community = a.witness_community.as_str() != principal.as_str()
            && a.witness_community.as_str() != a.principal_home.as_str();
        report.links.push(WitnessLink {
            cid,
            witness_community: a.witness_community,
            prev: a.prev,
            attested_at: a.attested_at,
            prev_resolved,
            cross_community,
        });
    }
    report
        .links
        .sort_by(|a, b| a.attested_at.cmp(&b.attested_at).then_with(|| a.cid.to_hex().cmp(&b.cid.to_hex())));
    report
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
                    sponsorship_cid: *cid,
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
                    outcome.adverse_conduct.push(*cid);
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
            profile.evidence.push(*cid);
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
                    non_admission_cid: *cid,
                    rejecting: r.rejecting_community.clone(),
                    presenting: r.presenting_community.clone(),
                    feature: r.feature,
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
    pattern.events.sort_by_key(|e| e.decided_at);
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
    /// `Some(true)` when the latest re-affirmation gap exceeds the **first** gap —
    /// the *pinned* counterpart to [`Self::latest_gap_widening`]. Needs at least two
    /// gaps; `None` otherwise.
    ///
    /// The rolling reading catches a widening and then **goes quiet once the widened
    /// cadence has recurred often enough to dominate the median of the priors** —
    /// gaps of `[100, 400, 400, 400]` read `false`, because 400 is no longer unusual
    /// *for this community*. The degraded cadence has become the reference it is
    /// judged against, so the signal falls silent exactly when the decay has
    /// finished setting. A baseline pinned to the founding cadence cannot be dragged
    /// along behind the drift — the same reason
    /// [`Self::witness_count_declined`] compares against the first rite.
    pub fn gap_widening_since_founding(&self) -> Option<bool> {
        if self.gaps_seconds.len() < 2 {
            return None;
        }
        Some(*self.gaps_seconds.last().unwrap() > self.gaps_seconds[0])
    }

    /// `Some(true)` when the latest re-affirmation gap exceeds the median of the
    /// prior gaps — a *rolling* reading, judged against how this community has
    /// lately behaved. Needs at least two gaps (three rites); `None` otherwise,
    /// since there is not enough cadence to compare.
    ///
    /// See [`Self::gap_widening_since_founding`] for the pinned counterpart.
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

/// Finish a decay series: sort the rites, derive the successive gaps, and measure
/// the staleness of the last one. Shared by the single- and all-community entry
/// points so there is one definition of a gap.
fn seal_decay(mut decay: CeremonyCostDecay, as_of: Timestamp) -> CeremonyCostDecay {
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
                    record_cid: *cid,
                    sealed_at: rec.sealed_at,
                    witness_count: rec.internal_witnesses.len(),
                });
            }
        }
    }
    seal_decay(decay, as_of)
}

/// Every community's ceremony-cost-decay series, keyed by community DID, in one
/// archive pass.
///
/// The compact-wide counterpart to [`ceremony_cost_decay`], and the input the
/// erosion alert needs: a *per-community* lens cannot answer "is anybody's
/// re-affirmation overdue" without knowing who the communities are. Only
/// communities that have sealed at least one rite appear — a community with no
/// rite at all has no cadence to be late against, and inventing one would report
/// a founding as a lapse.
pub fn ceremony_cost_decay_all(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> BTreeMap<String, CeremonyCostDecay> {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut by_community: BTreeMap<String, CeremonyCostDecay> = BTreeMap::new();

    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::IntraCommunityRite(signed) = entry {
            let rec = &signed.payload;
            let recitation = &rec.recitation.payload;
            if rec.sealed_at <= as_of {
                by_community
                    .entry(recitation.community_did.as_str().to_string())
                    .or_default()
                    .rites
                    .push(RitePoint {
                        record_cid: *cid,
                        sealed_at: rec.sealed_at,
                        witness_count: rec.internal_witnesses.len(),
                    });
            }
        }
    }
    by_community
        .into_iter()
        .map(|(did, decay)| (did, seal_decay(decay, as_of)))
        .collect()
}

// ---------------------------------------------------------------------------
// §7 commons free-rider lens (read-only, over usage records).
// ---------------------------------------------------------------------------

/// One appropriator's cumulative draw and contribution on a commons, with the
/// usage-record CIDs for traceability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageTally {
    pub community: DidKey,
    /// Summed over this community's **public** records only. A nonzero
    /// [`sealed_count`](Self::sealed_count) means this understates the real draw.
    pub consumed: u64,
    /// Summed over public records only, as above.
    pub contributed: u64,
    /// Every record this community filed, sealed or public.
    pub record_count: usize,
    /// How many of them were sealed, and so are counted here but not summed.
    ///
    /// Without this the tally would read as a complete account of a community that
    /// happened to draw nothing, which is what a sealed record made it read as before
    /// the field existed: measured, a community sealing 1,850 units showed `consumed: 0`
    /// and ranked nowhere, looking like the most scrupulous member of the compact.
    pub sealed_count: usize,
    pub evidence: Vec<Cid>,
}

impl UsageTally {
    /// `consumed - contributed` as a signed net draw (positive = drew more than
    /// it returned). i128 avoids u64 underflow.
    pub fn net_draw(&self) -> i128 {
        self.consumed as i128 - self.contributed as i128
    }
    /// `consumed / contributed`, `None` when `contributed == 0` (a nonzero draw
    /// against zero contribution is the strongest free-rider shape but is left
    /// for the reader to weigh via `net_draw`, not collapsed into a verdict).
    pub fn draw_ratio(&self) -> Option<f64> {
        if self.contributed == 0 {
            None
        } else {
            Some(self.consumed as f64 / self.contributed as f64)
        }
    }
}

/// Compact-wide free-rider signal for one commons (Ostrom principle 4 monitoring;
/// commons_federation.md §"The tragedy-of-the-virtues problem"). Per-appropriator
/// consumed/contributed tallies over the commons' usage records. Surfaces the
/// asymmetry between what communities drew and what they returned and renders **no
/// verdict** — whether an imbalance is free-riding, and what to do about it, is
/// the mediator pool's call. Every tally is traceable to signed usage records by
/// the CIDs it carries.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FreeRiderReport {
    /// Per-appropriator tallies, ordered by community DID.
    pub tallies: Vec<UsageTally>,
    /// The commons' per-period draw allowance, if it states one. What every sealed
    /// record on it carries a proof against, and so what bounds the draw those records
    /// hide.
    pub allowance: Option<u64>,
}

impl FreeRiderReport {
    /// Is every record in this report public, so the sums below are the whole account?
    ///
    /// A reader that treats `total_consumed` as *the* total is right only when this
    /// holds. Sealing is a legitimate exercise of §7's surveillance threshold, but it
    /// makes the sums a floor rather than a figure, and the report says which it is
    /// rather than leaving the reader to assume.
    pub fn is_complete(&self) -> bool {
        self.tallies.iter().all(|t| t.sealed_count == 0)
    }

    /// Communities that sealed at least one record, and how many, ordered by community.
    /// Not an accusation: sealing is permitted and proven within allowance. It is the
    /// fact a reader needs in order to know what the tally does not cover.
    pub fn partially_sealed(&self) -> Vec<(String, usize)> {
        self.tallies
            .iter()
            .filter(|t| t.sealed_count > 0)
            .map(|t| (t.community.as_str().to_string(), t.sealed_count))
            .collect()
    }

    /// The most that could be hidden behind every sealed record in this report:
    /// `sealed records × allowance`.
    ///
    /// This is what sealing actually costs the compact's monitoring, and it is a bound
    /// rather than a blindfold. Each sealed record carries a proof that its draw is at
    /// most the allowance, so the hidden quantity is not unknown, only unresolved. Adding
    /// this to `total_consumed` gives the worst case a reader has to reason about.
    ///
    /// `None` when the commons states no allowance, which is also the case in which no
    /// record on it can be sealed.
    pub fn max_hidden_draw(&self) -> Option<u64> {
        let allowance = self.allowance?;
        let sealed: u64 = self.tallies.iter().map(|t| t.sealed_count as u64).sum();
        Some(sealed.saturating_mul(allowance))
    }

    pub fn total_consumed(&self) -> u64 {
        self.tallies.iter().map(|t| t.consumed).sum()
    }
    pub fn total_contributed(&self) -> u64 {
        self.tallies.iter().map(|t| t.contributed).sum()
    }
    /// Communities whose consumption exceeds contribution, ordered by descending
    /// net draw — the ranking a free-rider dashboard would head with.
    pub fn by_net_draw(&self) -> Vec<(String, i128)> {
        let mut v: Vec<(String, i128)> = self
            .tallies
            .iter()
            .filter(|t| t.net_draw() > 0)
            .map(|t| (t.community.as_str().to_string(), t.net_draw()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v
    }
}

/// Compute the free-rider report for the commons registered at `commons_cid`, as
/// of `as_of` (defaults to now). Aggregates every usage record referencing that
/// commons whose `recorded_at <= as_of`.
pub fn free_rider_signals(
    archive: &InMemoryArchive,
    commons_cid: &Cid,
    as_of: Option<Timestamp>,
) -> FreeRiderReport {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut tallies: BTreeMap<String, UsageTally> = BTreeMap::new();
    let mut allowance = None;
    for (cid, entry) in archive.iter() {
        match entry {
            ArchiveEntry::CommonsRegistered(signed) if cid == commons_cid => {
                allowance = signed.payload.draw_allowance;
            }
            ArchiveEntry::UsageAccounted(signed) => {
                let u = &signed.payload;
                if &u.commons_cid != commons_cid || u.recorded_at > as_of {
                    continue;
                }
                let t =
                    tallies.entry(u.community.as_str().to_string()).or_insert_with(|| UsageTally {
                        community: u.community.clone(),
                        consumed: 0,
                        contributed: 0,
                        record_count: 0,
                        sealed_count: 0,
                        evidence: vec![],
                    });
                // A sealed record is counted and not summed. Its figures are committed
                // rather than published, so there is nothing honest to add; what it
                // carries instead is a proof that the draw is within the allowance,
                // which `max_hidden_draw` turns into the bound a reader needs.
                if u.sealed.is_some() {
                    t.sealed_count += 1;
                } else {
                    t.consumed = t.consumed.saturating_add(u.consumed);
                    t.contributed = t.contributed.saturating_add(u.contributed);
                }
                t.record_count += 1;
                t.evidence.push(*cid);
            }
            _ => {}
        }
    }
    FreeRiderReport { tallies: tallies.into_values().collect(), allowance }
}

// ---------------------------------------------------------------------------
// §8 observer pool-share / rotation lens (read-only — the domination front).
// ---------------------------------------------------------------------------

/// One nominating community's current stake in the observer corps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserverPoolShare {
    pub community: String,
    /// Active observers this community currently nominates.
    pub active: usize,
}

/// One observer currently serving, with the term end that governs its rotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingObserver {
    pub observer_did: String,
    pub nominating_community: String,
    /// The term end; `None` only for legacy open-ended entries the term-limit rule
    /// now refuses at admission.
    pub term_end: Option<Timestamp>,
}

/// §8 observer pool-share / rotation lens (capture resistance): the active corps'
/// composition by nominating community and the observers currently serving. Like
/// every jester lens it surfaces flags with provenance and renders **no verdict**
/// — the no-majority pool-share cap and the term/rotation limits are *enforced* by
/// the authority pass (`check_authority`). This board makes the finer drift toward
/// domination (proportional concentration short of a majority) and the rotation
/// horizon visible before the hard cap bites.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObserverPoolConcentration {
    /// Active observers per nominating community, descending by count then DID.
    pub shares: Vec<ObserverPoolShare>,
    /// Total active observers across the corps.
    pub total_active: usize,
    /// Observers currently serving, soonest term-end first (the rotation view).
    pub serving: Vec<ServingObserver>,
}

impl ObserverPoolConcentration {
    /// The largest single-community active count (0 if the corps is empty) — the
    /// domination front the pool-share cap watches.
    pub fn top_active(&self) -> usize {
        self.shares.first().map_or(0, |s| s.active)
    }

    /// True when one community already holds a majority of the active corps (the
    /// condition the pool-share authority rule refuses once past its floor).
    pub fn majority_held(&self) -> bool {
        self.total_active > 0 && 2 * self.top_active() > self.total_active
    }
}

/// The active observer corps as of `as_of`, respecting registry superseding (only
/// the latest `ObserverRegistryUpdated` per nominating community with
/// `issued_at <= as_of` contributes its active entries).
pub fn observer_pool_concentration(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> ObserverPoolConcentration {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut latest: BTreeMap<String, &ObserverRegistry> = BTreeMap::new();
    for (_, e) in archive.iter() {
        if let ArchiveEntry::ObserverRegistryUpdated(s) = e {
            if s.payload.issued_at <= as_of {
                let key = s.payload.nominating_community.as_str().to_string();
                let supersede = latest
                    .get(&key)
                    .is_none_or(|cur| s.payload.issued_at >= cur.issued_at);
                if supersede {
                    latest.insert(key, &s.payload);
                }
            }
        }
    }
    let mut per_community: BTreeMap<String, usize> = BTreeMap::new();
    let mut serving = vec![];
    let mut total_active = 0usize;
    for reg in latest.values() {
        for en in &reg.entries {
            if en.active_at(as_of) {
                total_active += 1;
                *per_community
                    .entry(reg.nominating_community.as_str().to_string())
                    .or_insert(0) += 1;
                serving.push(ServingObserver {
                    observer_did: en.observer_did.as_str().to_string(),
                    nominating_community: reg.nominating_community.as_str().to_string(),
                    term_end: en.valid_until,
                });
            }
        }
    }
    let mut shares: Vec<ObserverPoolShare> = per_community
        .into_iter()
        .map(|(community, active)| ObserverPoolShare { community, active })
        .collect();
    shares.sort_by(|a, b| b.active.cmp(&a.active).then_with(|| a.community.cmp(&b.community)));
    serving.sort_by(|a, b| {
        a.term_end.cmp(&b.term_end).then_with(|| a.observer_did.cmp(&b.observer_did))
    });
    ObserverPoolConcentration { shares, total_active, serving }
}

/// One community's share of the active mediator pool (§4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediatorPoolShare {
    pub community: String,
    pub active: usize,
}

/// One mediator currently seated, with the term end the rotation view sorts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingMediator {
    pub mediator_did: String,
    pub nominating_community: String,
    pub term_end: Option<Timestamp>,
    /// Who trained them, where the apprenticeship was recorded.
    pub trained_by: Option<String>,
}

/// The §4 mediator pool's composition, by seating community and by training lineage.
///
/// The counterpart of [`ObserverPoolConcentration`], and it renders **no verdict** for
/// the same reason: the no-majority share, term and rotation limits are *enforced* by
/// the authority pass, and this board makes the finer drift visible before the hard cap
/// bites.
///
/// The lineage half exists because §4's apprenticeship model carries its own named
/// weakness — mediators trained under experienced mediators absorb genuine discernment,
/// and are "susceptible to capture by the incumbent generation's biases"
/// (`liminal_institutions.md` "Discernment training"). A pool that all traces to one
/// trainer is that weakness realized. Reporting it is the whole of what this does: the
/// same documents warn against "a certification barrier that becomes a gatekeeping
/// mechanism", and the whitepaper lists the training pipeline as an open question, so
/// **no rule reads `trained_by` to admit or refuse anyone.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediatorPoolConcentration {
    /// Active mediators per nominating community, descending by count then DID.
    pub shares: Vec<MediatorPoolShare>,
    /// Total active mediators across the pool.
    pub total_active: usize,
    /// Mediators currently seated, soonest term-end first (the rotation view).
    pub serving: Vec<ServingMediator>,
    /// Active mediators per recorded trainer, descending by count then DID. Mediators
    /// with no recorded trainer are absent rather than grouped under a blank: an
    /// unrecorded apprenticeship and no apprenticeship are not distinguished, and a
    /// compact that does not practise it is not thereby irregular.
    pub lineages: Vec<MediatorPoolShare>,
    /// Active mediators with a recorded trainer — the denominator the lineage readings
    /// are against, so a pool that records nothing reads as unknown rather than healthy.
    pub with_lineage: usize,
}

impl MediatorPoolConcentration {
    /// The largest single-community active count (0 if the pool is empty).
    pub fn top_active(&self) -> usize {
        self.shares.first().map_or(0, |s| s.active)
    }

    /// True when one community already holds a majority of the active pool — the
    /// condition the pool-share authority rule refuses once past its floor.
    pub fn majority_held(&self) -> bool {
        self.total_active > 0 && 2 * self.top_active() > self.total_active
    }

    /// The largest single-trainer active count (0 if nothing is recorded).
    pub fn top_lineage(&self) -> usize {
        self.lineages.first().map_or(0, |s| s.active)
    }

    /// True when one trainer accounts for a majority of the mediators **whose lineage
    /// is recorded** — the incumbent generation holding the pool.
    ///
    /// Measured against `with_lineage` rather than `total_active` deliberately. A pool
    /// where one trainer is behind 3 of 4 recorded apprenticeships says that whatever
    /// apprenticeship happened, one hand shaped it; padding the denominator with
    /// mediators who recorded nothing would let a compact silence the reading by simply
    /// not recording, which is the failure mode the whole erosion audit exists for.
    /// `false` when nothing is recorded, so silence is never read as a finding.
    pub fn lineage_majority_held(&self) -> bool {
        self.with_lineage > 0 && 2 * self.top_lineage() > self.with_lineage
    }
}

/// The active mediator pool as of `as_of`, respecting registry superseding (only the
/// latest `MediatorRegistryUpdated` per nominating community with `issued_at <= as_of`
/// contributes its active entries).
pub fn mediator_pool_concentration(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> MediatorPoolConcentration {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut latest: BTreeMap<String, &MediatorRegistry> = BTreeMap::new();
    for (_, e) in archive.iter() {
        if let ArchiveEntry::MediatorRegistryUpdated(s) = e {
            if s.payload.issued_at <= as_of {
                let key = s.payload.nominating_community.as_str().to_string();
                let supersede = latest
                    .get(&key)
                    .is_none_or(|cur| s.payload.issued_at >= cur.issued_at);
                if supersede {
                    latest.insert(key, &s.payload);
                }
            }
        }
    }
    let mut per_community: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_trainer: BTreeMap<String, usize> = BTreeMap::new();
    let mut serving = vec![];
    let (mut total_active, mut with_lineage) = (0usize, 0usize);
    for reg in latest.values() {
        for en in &reg.entries {
            if en.active_at(as_of) {
                total_active += 1;
                *per_community
                    .entry(reg.nominating_community.as_str().to_string())
                    .or_insert(0) += 1;
                if let Some(t) = &en.trained_by {
                    with_lineage += 1;
                    *per_trainer.entry(t.as_str().to_string()).or_insert(0) += 1;
                }
                serving.push(ServingMediator {
                    mediator_did: en.mediator_did.as_str().to_string(),
                    nominating_community: reg.nominating_community.as_str().to_string(),
                    term_end: en.valid_until,
                    trained_by: en.trained_by.as_ref().map(|t| t.as_str().to_string()),
                });
            }
        }
    }
    let rank = |m: BTreeMap<String, usize>| {
        let mut v: Vec<MediatorPoolShare> = m
            .into_iter()
            .map(|(community, active)| MediatorPoolShare { community, active })
            .collect();
        v.sort_by(|a, b| b.active.cmp(&a.active).then_with(|| a.community.cmp(&b.community)));
        v
    };
    serving.sort_by(|a, b| {
        a.term_end.cmp(&b.term_end).then_with(|| a.mediator_did.cmp(&b.mediator_did))
    });
    MediatorPoolConcentration {
        shares: rank(per_community),
        total_active,
        serving,
        lineages: rank(per_trainer),
        with_lineage,
    }
}

// ---------------------------------------------------------------------------
// §8 observer dashboard (read-only aggregation — the composition-drift board).
// ---------------------------------------------------------------------------

/// The observer corps's compact-wide capture-signature view (protocol-components
/// §8 "composition-drift dashboard"). It bundles the admissions-integrity lenses
/// already computed elsewhere — the sponsorship [`CompositionGraph`] (composition
/// drift) and the [`RejectionPattern`] (who rejects whom) — with the §8 activity
/// tallies: attributed observer reports per subject community and whistleblower
/// reports per accused community. Like every jester lens it surfaces flags with
/// provenance (the embedded structs carry their CIDs) and renders **no verdict**.
/// It also carries the observer-corps composition ([`ObserverPoolConcentration`],
/// the pool-share/rotation view) and the *substrate-deference* half of the board
/// ([`SubstrateDeferenceReport`], §9) — the compact-incapable-list state and
/// structured-fast cadence — so a single board shows both membership drift and
/// non-ultimacy erosion.
#[derive(Debug, Clone, Default)]
pub struct ObserverDashboard {
    pub composition: CompositionGraph,
    pub rejections: RejectionPattern,
    /// Subject community DID → attributed observer reports against it, descending.
    pub observer_reports_by_subject: Vec<(String, usize)>,
    /// Accused community DID → whistleblower reports against it, descending. The
    /// source is never counted or exposed.
    pub whistleblowers_by_accused: Vec<(String, usize)>,
    /// Accused community DID → how many **distinct** insider sources have filed
    /// against it, descending.
    ///
    /// Read alongside `whistleblowers_by_accused`, this separates a community many
    /// insiders are reporting from one insider reporting many times. Ten filings and
    /// ten sources is a pattern; ten filings and one source is a single voice, and
    /// possibly a channel being flooded to drown a real report. Distinctness comes
    /// from the ring signatures' key images, so counting sources never means learning
    /// who they are.
    pub distinct_sources_by_accused: Vec<(String, usize)>,
    /// The observer corps's active composition by nominating community and the
    /// current rotation horizon (the pool-share lens).
    pub pool: ObserverPoolConcentration,
    /// The substrate-deference / erosion report: compact-incapable-list drift and
    /// structured-fast cadence (the §9 half of the §8 dashboard).
    pub substrate: SubstrateDeferenceReport,
    /// The graded erosion alert derived from `substrate` — the crossed-threshold
    /// signal the observer corps reads before eyeballing raw counts.
    pub erosion: ErosionAlert,
    /// Community DID → how many keys its anonymous channel can hide a source among,
    /// ascending, so the thinnest channels are first.
    ///
    /// A community whose count is 1 has no channel at all, and 2 is a channel in name
    /// only: with the key image published on every filing, a suspect set of two is
    /// nearly a name. This is the lens on whether the §8 channel is real, and it is
    /// what a member roll moves. Ascending because the interesting end is the low end.
    pub anonymity_set_by_community: Vec<(String, usize)>,
}

fn descending(map: BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut v: Vec<(String, usize)> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Build the observer dashboard as of `as_of` (defaults to now).
pub fn observer_dashboard(archive: &InMemoryArchive, as_of: Option<Timestamp>) -> ObserverDashboard {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut reports: BTreeMap<String, usize> = BTreeMap::new();
    let mut whistles: BTreeMap<String, usize> = BTreeMap::new();
    let mut tags: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (_, entry) in archive.iter() {
        match entry {
            ArchiveEntry::ObserverReportPublished(s) if s.payload.reported_at <= as_of => {
                *reports.entry(s.payload.subject_community.as_str().to_string()).or_insert(0) += 1;
            }
            ArchiveEntry::WhistleblowerFiled(s) if s.payload.filed_at <= as_of => {
                let accused = s.payload.accused_community.as_str().to_string();
                *whistles.entry(accused.clone()).or_insert(0) += 1;
                tags.entry(accused).or_default().insert(s.payload.source_tag().to_hex());
            }
            _ => {}
        }
    }
    let distinct: BTreeMap<String, usize> =
        tags.into_iter().map(|(k, v)| (k, v.len())).collect();
    // Every community the archive knows, sized by its insider set. Communities come
    // from their published identities, so a community with neither delegation nor roll
    // still appears — at 1, which is the reading that matters.
    let mut anonymity: Vec<(String, usize)> = archive
        .iter()
        .filter_map(|(_, e)| match e {
            ArchiveEntry::CommunityIdentityPublished(s) => Some(s.payload.did.clone()),
            _ => None,
        })
        .map(|did| {
            let n = kc_archive::insider_keys(archive, &did, as_of).len();
            (did.as_str().to_string(), n)
        })
        .collect();
    anonymity.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    ObserverDashboard {
        composition: composition_graph(archive, Some(as_of)),
        rejections: rejection_pattern(archive, Some(as_of)),
        observer_reports_by_subject: descending(reports),
        whistleblowers_by_accused: descending(whistles),
        distinct_sources_by_accused: descending(distinct),
        pool: observer_pool_concentration(archive, Some(as_of)),
        substrate: substrate_deference(archive, Some(as_of)),
        erosion: erosion_alert(archive, Some(as_of)),
        anonymity_set_by_community: anonymity,
    }
}

// ---------------------------------------------------------------------------
// §9 substrate-deference (constitutional layer) — state, gates, erosion.
// ---------------------------------------------------------------------------

/// The compact's constitutional state as of a query time: whether it is ratified,
/// the compact-incapable matters currently in force (genesis + adds − removes),
/// the current structured-fast cadence (genesis periods overlaid by the latest
/// amendment per label), and the adopted minimum content (adoption + broadenings −
/// narrowings).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConstitutionState {
    pub ratified: bool,
    pub compact_incapable: Vec<IncapableMatter>,
    pub fast_periods: Vec<FastPeriod>,
    /// Whether the compact has adopted the functional minimum as the ground of its own
    /// conduct. Independent of `ratified`: the two artifacts are separate acts.
    pub minimum_adopted: bool,
    /// The forbearances in force (adoption + broadenings − narrowings). Unlike
    /// `compact_incapable`, no entry here is immutable.
    pub minimum_content: Vec<Forbearance>,
}

/// Compute the constitution as of `as_of` (defaults to now). Amendments with
/// `decided_at <= as_of` are applied in insertion order.
pub fn constitution_state(archive: &InMemoryArchive, as_of: Option<Timestamp>) -> ConstitutionState {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut ratified = false;
    let mut matters: BTreeMap<String, IncapableMatter> = BTreeMap::new();
    let mut periods: BTreeMap<String, FastPeriod> = BTreeMap::new();
    let mut minimum_adopted = false;
    let mut forbearances: BTreeMap<String, Forbearance> = BTreeMap::new();
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::ConstitutionRatified(s) if s.payload.genesis_at <= as_of => {
                ratified = true;
                for m in &s.payload.compact_incapable {
                    matters.insert(m.key.clone(), m.clone());
                }
                for p in &s.payload.fast_periods {
                    periods.insert(p.label.clone(), p.clone());
                }
            }
            ArchiveEntry::CompactIncapableAmended(s) if s.payload.decided_at <= as_of => {
                match s.payload.kind {
                    AmendmentKind::Add => {
                        matters.insert(s.payload.matter.key.clone(), s.payload.matter.clone());
                    }
                    AmendmentKind::Remove => {
                        matters.remove(&s.payload.matter.key);
                    }
                }
            }
            ArchiveEntry::FastCadenceAmended(s) if s.payload.decided_at <= as_of => {
                periods.insert(s.payload.period.label.clone(), s.payload.period.clone());
            }
            ArchiveEntry::MinimumContentAdopted(s) if s.payload.adopted_at <= as_of => {
                minimum_adopted = true;
                for f in &s.payload.forbearances {
                    forbearances.insert(f.key.clone(), f.clone());
                }
            }
            ArchiveEntry::MinimumContentAmended(s) if s.payload.decided_at <= as_of => {
                match s.payload.change {
                    ForbearanceChange::Broaden => {
                        forbearances
                            .insert(s.payload.forbearance.key.clone(), s.payload.forbearance.clone());
                    }
                    ForbearanceChange::Narrow => {
                        forbearances.remove(&s.payload.forbearance.key);
                    }
                }
            }
            _ => {}
        }
    }
    ConstitutionState {
        ratified,
        compact_incapable: matters.into_values().collect(),
        fast_periods: periods.into_values().collect(),
        minimum_adopted,
        minimum_content: forbearances.into_values().collect(),
    }
}

/// §9 categorical-deference gate: is `matter_key` a matter the compact will not
/// decide (in force as of `as_of`)? A caller contemplating a first-order decision
/// on a member's internal affairs checks this and declines when true.
pub fn compact_incapable(
    archive: &InMemoryArchive,
    matter_key: &str,
    as_of: Option<Timestamp>,
) -> bool {
    constitution_state(archive, as_of)
        .compact_incapable
        .iter()
        .any(|m| m.key == matter_key)
}

/// §9 self-binding gate: is `forbearance_key` part of the ground the compact has
/// bound its own conduct to (in force as of `as_of`)? The twin of
/// [`compact_incapable`], and the asymmetry between them is the doctrine's: that one
/// answers what the compact will not decide *for a member*, this one what the compact
/// will not do *itself*, wherever it acts and consent has run out.
pub fn minimum_forbearance(
    archive: &InMemoryArchive,
    forbearance_key: &str,
    as_of: Option<Timestamp>,
) -> bool {
    constitution_state(archive, as_of)
        .minimum_content
        .iter()
        .any(|f| f.key == forbearance_key)
}

/// §9 temporal-deference gate: the label of the structured-fast period containing
/// `at` in the cadence in force then, or `None` if the compact is not fasting.
pub fn in_structured_fast(archive: &InMemoryArchive, at: Timestamp) -> Option<String> {
    constitution_state(archive, Some(at))
        .fast_periods
        .into_iter()
        .find(|p| p.contains(at))
        .map(|p| p.label)
}

/// A deliberative compact act initiated during a structured fast — a **deviation**
/// the observer corps surfaces (capture_resistance.md: the fast is honored by
/// social practice, made legible here, not hard-refused by the authority pass).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastDeviation {
    pub record_cid: Cid,
    /// The archive entry kind (`DisputeFiled`, `DesignationDecided`, …).
    pub kind: String,
    pub at: Timestamp,
    pub fast_label: String,
}

/// The substrate-deference / erosion report (protocol-components §9
/// "erosion-detection alerting"; completes the substrate-deference half of the §8
/// dashboard). Read-only, no verdict: it makes the compact's non-ultimacy legible
/// — the size and drift of the compact-incapable list, the fast cadence and its
/// contraction count, and deliberative acts initiated during a fast — so members
/// and the jester can see drift toward ultimacy before any single act looks wrong.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubstrateDeferenceReport {
    pub ratified: bool,
    /// Compact-incapable matters currently in force.
    pub incapable_current: usize,
    /// `Add` amendments accepted-or-attempted as of the query (expansion).
    pub incapable_added: usize,
    /// `Remove` amendments as of the query (contraction).
    pub incapable_removed: usize,
    /// True when any protection has been rolled back — a compact-incapable matter
    /// removed, a fast period contracted, or a forbearance narrowed out of the
    /// compact's own adopted ground. The erosion signal: contraction of the
    /// deference commitments is exactly the early sign of drift toward ultimacy the
    /// observer corps should see (surfaced, not judged). (Note: a *net* count is
    /// not the signal — a matter can only be removed if it was earlier added, so
    /// removals never exceed additions; the presence of any rollback is what
    /// matters.)
    pub contraction_detected: bool,
    /// Structured-fast periods currently in the cadence.
    pub fast_period_count: usize,
    /// Total seconds of structured fast currently scheduled.
    pub fast_seconds_total: i64,
    /// Fast-cadence `Contract` amendments as of the query (shrinkage).
    pub fast_contractions: usize,
    /// Forbearances in the adopted minimum content currently in force.
    pub minimum_current: usize,
    /// `Broaden` amendments as of the query (expansion of the compact's own ground).
    pub minimum_broadened: usize,
    /// `Narrow` amendments as of the query (contraction). Counted separately from the
    /// incapable list because the two artifacts answer different questions, and folded
    /// into `contraction_detected` because the pressure on them is the same.
    pub minimum_narrowed: usize,
    /// Deliberative acts initiated during a fast window, oldest first.
    pub fast_deviations: Vec<FastDeviation>,
}

/// The deliberative-initiation timestamp and kind of `entry`, if it is a
/// compact-level *initiating* act (a new mediation, a designation, or an
/// escalation up the dispute or commons ladder). De-escalations, injunctions
/// (emergency), and registry/response records are not initiations.
fn deliberative_initiation(entry: &ArchiveEntry) -> Option<(Timestamp, &'static str)> {
    match entry {
        ArchiveEntry::DisputeFiled(s) => Some((s.payload.filed_at, "DisputeFiled")),
        ArchiveEntry::DesignationDecided(s) => Some((s.payload.designated_at, "DesignationDecided")),
        ArchiveEntry::LadderAdvanced(s) => {
            let from = dispute_ladder_rank(s.payload.from_rung);
            let to = dispute_ladder_rank(s.payload.to_rung);
            match (from, to) {
                (Some(f), Some(t)) if t > f => Some((s.payload.transitioned_at, "LadderAdvanced")),
                _ => None,
            }
        }
        ArchiveEntry::SanctionApplied(s) => {
            if sanction_rank(s.payload.to_level) > sanction_rank(s.payload.from_level) {
                Some((s.payload.decided_at, "SanctionApplied"))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Compute the substrate-deference report as of `as_of` (defaults to now).
pub fn substrate_deference(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> SubstrateDeferenceReport {
    let as_of = as_of.unwrap_or_else(time::now);
    let state = constitution_state(archive, Some(as_of));

    let mut added = 0usize;
    let mut removed = 0usize;
    let mut fast_contractions = 0usize;
    let mut minimum_broadened = 0usize;
    let mut minimum_narrowed = 0usize;
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::CompactIncapableAmended(s) if s.payload.decided_at <= as_of => {
                match s.payload.kind {
                    AmendmentKind::Add => added += 1,
                    AmendmentKind::Remove => removed += 1,
                }
            }
            ArchiveEntry::FastCadenceAmended(s)
                if s.payload.decided_at <= as_of && s.payload.change.is_contraction() =>
            {
                fast_contractions += 1;
            }
            ArchiveEntry::MinimumContentAmended(s) if s.payload.decided_at <= as_of => {
                match s.payload.change {
                    ForbearanceChange::Broaden => minimum_broadened += 1,
                    ForbearanceChange::Narrow => minimum_narrowed += 1,
                }
            }
            _ => {}
        }
    }

    let mut deviations = vec![];
    for (cid, e) in archive.iter() {
        if let Some((at, kind)) = deliberative_initiation(e) {
            if at <= as_of {
                if let Some(label) = state.fast_periods.iter().find(|p| p.contains(at)) {
                    deviations.push(FastDeviation {
                        record_cid: *cid,
                        kind: kind.to_string(),
                        at,
                        fast_label: label.label.clone(),
                    });
                }
            }
        }
    }
    deviations.sort_by_key(|d| d.at);

    SubstrateDeferenceReport {
        ratified: state.ratified,
        incapable_current: state.compact_incapable.len(),
        incapable_added: added,
        incapable_removed: removed,
        contraction_detected: removed > 0 || fast_contractions > 0 || minimum_narrowed > 0,
        fast_period_count: state.fast_periods.len(),
        fast_seconds_total: state.fast_periods.iter().map(|p| p.seconds()).sum(),
        fast_contractions,
        minimum_current: state.minimum_content.len(),
        minimum_broadened,
        minimum_narrowed,
        fast_deviations: deviations,
    }
}

/// The severity of substrate-deference erosion, escalating with the number of
/// distinct erosion signals present. Diagnostic only: it is surfaced to the
/// observer corps and the member communities (protocol-components §9
/// "erosion-detection alerting"), never a verdict and never a refusal — non-
/// ultimacy is a disposition the compact demonstrates, not a rule the archive
/// enforces (capture_resistance.md §2).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ErosionSeverity {
    /// No deference commitment has been rolled back and no deliberative act has
    /// been initiated during a fast.
    #[default]
    None,
    /// A single deference dimension shows a single, isolated erosion — worth an
    /// eye, not yet a trend.
    Notice,
    /// Erosion is compounding: two or more distinct dimensions have eroded, or a
    /// single dimension has eroded more than once. The observer corps and member
    /// communities should look now.
    Alert,
}

impl ErosionSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErosionSeverity::None => "None",
            ErosionSeverity::Notice => "Notice",
            ErosionSeverity::Alert => "Alert",
        }
    }
}

/// One decided dispute's deliberation, as the archive records it (§4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliberationReading {
    pub decision_cid: Cid,
    pub dispute_cid: Cid,
    /// How many arguments the decision answered.
    pub argument_count: usize,
    /// Seconds from the earliest cited argument to the decision. `None` when the
    /// decision cited nothing datable.
    pub span_seconds: Option<i64>,
    pub decided_at: Timestamp,
}

/// The compact's deliberation over time, oldest decision first — §4's record of *how*
/// it decided, which is what the §8 observer corps and the §9 erosion signals both
/// assume can be audited.
///
/// Compact-wide rather than per-community, unlike the rite series: a mediation panel is
/// drawn from non-party communities and decides for the compact, so there is no single
/// community whose deliberation this is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeliberationDecay {
    pub decisions: Vec<DeliberationReading>,
}

impl DeliberationDecay {
    /// True when the latest decision answered fewer arguments than the **first**.
    ///
    /// Pinned to the compact's first decision, never to a rolling mean, for the reason
    /// [`CeremonyCostDecay::witness_count_declined`] is: a deliberation that thins a
    /// little each time drags a rolling baseline along behind it and the signal falls
    /// silent exactly when the decay has finished setting. Only an absolute measure
    /// sees a drift with no adversary behind it. Needs two decisions; `false`
    /// otherwise, so nothing is fabricated from one data point.
    pub fn argument_count_declined(&self) -> bool {
        match (self.decisions.first(), self.decisions.last()) {
            (Some(f), Some(l)) if self.decisions.len() >= 2 => {
                l.argument_count < f.argument_count
            }
            _ => false,
        }
    }

    /// True when the latest decision was reached in less time than the **first**, on
    /// the same pinned baseline.
    ///
    /// The span is what §4's "async-compatible for light-lag" asks the record to carry:
    /// a compact whose deliberation collapses from weeks to seconds has stopped
    /// deliberating across the distances it exists to span, whatever it still files.
    pub fn span_shortened(&self) -> bool {
        match (
            self.decisions.first().and_then(|d| d.span_seconds),
            self.decisions.last().and_then(|d| d.span_seconds),
        ) {
            (Some(f), Some(l)) if self.decisions.len() >= 2 => l < f,
            _ => false,
        }
    }
}

/// Read every mediation decision's deliberation off the archive, oldest first.
///
/// Content is never read. How long an argument's `position` is says nothing a rule may
/// act on — the tier's standing line that no rule can weigh what was said, only record
/// that it was put.
pub fn deliberation_decay(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> DeliberationDecay {
    let as_of_ts = as_of.unwrap_or_else(time::now);
    let mut arguments: HashMap<String, Timestamp> = HashMap::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::ArgumentFiled(s) = entry {
            arguments.insert(cid.to_string(), s.payload.submitted_at);
        }
    }
    let mut decisions: Vec<DeliberationReading> = Vec::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::MediationDecided(s) = entry {
            let d = &s.payload;
            if d.decided_at > as_of_ts {
                continue;
            }
            let earliest = d
                .arguments
                .iter()
                .filter_map(|a| arguments.get(&a.to_string()).map(|t| t.as_second()))
                .fold(None::<i64>, |acc, t| Some(acc.map_or(t, |a: i64| a.min(t))));
            decisions.push(DeliberationReading {
                decision_cid: *cid,
                dispute_cid: d.dispute_cid,
                argument_count: d.arguments.len(),
                span_seconds: earliest.map(|e| d.decided_at.as_second() - e),
                decided_at: d.decided_at,
            });
        }
    }
    decisions.sort_by_key(|d| d.decided_at);
    DeliberationDecay { decisions }
}

/// The erosion alert (protocol-components §9 "erosion-detection alerting"): a
/// graded, derived reading of [`SubstrateDeferenceReport`] that **escalates** as
/// the erosion signals compound, so the observer corps sees a crossed threshold
/// rather than having to eyeball raw counts. Purely diagnostic — no archive
/// record is emitted, nothing is signed, and no deliberative act is refused; a
/// human observer who sees an `Alert` may then choose to publish an attributed
/// `ObserverReport`. That human step is intentionally not automated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErosionAlert {
    pub severity: ErosionSeverity,
    /// One human-readable reason per active erosion signal (categorical first,
    /// then temporal, then in-practice), for direct display to the corps.
    pub reasons: Vec<String>,
    /// A compact-incapable matter has been removed (categorical-list contraction).
    pub list_contraction: bool,
    /// A forbearance has been narrowed out of the compact's own adopted ground.
    ///
    /// The signal with no constituency behind it: nobody in the compact is asked to
    /// endorse the minimum content, so no member holds it as a commitment to defend
    /// when the argument for narrowing what is owed at the frontier arrives. The
    /// asymmetric bar refuses the cheap narrowing; this makes the dear one legible.
    pub minimum_contraction: bool,
    /// A structured-fast period has been shortened (temporal-cadence contraction).
    pub fast_contraction: bool,
    /// Deliberative acts initiated during a fast window (temporal deference eroding
    /// in practice — "exceptions multiply").
    pub active_fast_deviations: usize,
    /// Communities whose last sealed re-affirmation is older than the cadence,
    /// oldest-overdue first.
    ///
    /// The first signal on this alert that fires on **omission**. The three above
    /// all require a record: somebody amends the list, shortens a fast, or acts
    /// during one. A polity that simply stops emits none of them, and a detector
    /// that waits to be told cannot be told by silence. This one measures against
    /// a fixed cadence instead of against the polity's own recent behaviour, so a
    /// re-affirmation that never comes is visible as the thing it is.
    pub overdue_reaffirmations: Vec<OverdueReaffirmation>,
    /// Communities whose latest rite drew fewer witnesses than their **first**,
    /// sorted by DID.
    ///
    /// Pinned to the founding rite rather than to a rolling mean, so a ceremony
    /// hollowed out one witness per cycle cannot drag the reference along behind
    /// it (`CeremonyCostDecay::witness_count_declined`).
    pub hollowing_communities: Vec<String>,
    /// The compact's deliberation is thinning: its latest mediation decision answered
    /// fewer arguments than its first, or was reached in less time.
    ///
    /// Read against the compact's **own first** decision rather than its recent
    /// average, which is the only kind of measure that sees a drift with nobody driving
    /// it — the same reason `hollowing_communities` is pinned to the founding rite. The
    /// §4 floor (every panelist argues, every party argument answered) makes a *bare*
    /// decision impossible; this is what can still decay once the floor holds.
    pub thinning_deliberation: Option<ThinningDeliberation>,
    /// Designations past their mandated re-examination date and still standing,
    /// longest-overdue first ([`overdue_reexaminations`]).
    ///
    /// Also an omission signal, and the one with a named party on the other end of
    /// it: an exclusion the compact undertook to revisit and did not.
    pub overdue_reexaminations: Vec<OverdueReexamination>,
    /// Standing functions with no act and no attested null inside the cadence, longest
    /// silence first ([`silent_functions`]).
    ///
    /// The omission signal for the compact's ordinary running rather than its ceremony.
    /// A pool that stopped taking disputes and a corps that stopped filing reports leave
    /// every relative measure untouched, which is why this one is against a clock.
    pub silent_functions: Vec<SilentFunction>,
    /// Standing functions a community once staffed and now has nobody seated in, longest
    /// vacancy first ([`unstaffed_functions`]).
    ///
    /// The sharpest omission of the set, and the one no remediation item had named:
    /// measuring found a compact with zero active mediators and zero active observers for
    /// forty-three years reading clean on every instrument.
    pub unstaffed_functions: Vec<UnstaffedFunction>,
    /// The compact's enforcement has narrowed onto part of the taxonomy while
    /// spreading across subjects ([`enforcement_convergence`]).
    ///
    /// The first signal here that fires on **intensification**. Every dimension above
    /// reads a form: a list, a cadence, a rite, a seat, a decision. All of them are
    /// satisfied by a polity that keeps every form and performs it harder, which is
    /// the whole of substitution — the recited object untouched in the record and the
    /// enforced object quietly narrowed. Measured before this field existed: three
    /// compacts differing only in what they enforced, one of them citing a single
    /// ground against five different communities, all three scoring `None` with no
    /// reasons on every instrument the compact had.
    pub enforcement_convergence: Option<EnforcementConvergence>,
    /// The compact's enforcement runs at communities holding reduced standing
    /// ([`enforcement_direction`]).
    ///
    /// The narrowed ground does not land evenly. This is the reading that says where
    /// it landed, and it is reported separately rather than folded into the
    /// convergence dimension because either can occur without the other.
    pub enforcement_direction: Option<EnforcementDirection>,
}

/// Why the compact's deliberation reads as thinning, with both readings reported so a
/// corps member can see which one fired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThinningDeliberation {
    pub decisions: usize,
    pub first_argument_count: usize,
    pub latest_argument_count: usize,
    pub first_span_seconds: Option<i64>,
    pub latest_span_seconds: Option<i64>,
    /// The latest decision answered fewer arguments than the first.
    pub argument_count_declined: bool,
    /// The latest decision was reached in less time than the first.
    pub span_shortened: bool,
}

/// A designation whose mandated re-examination date has passed with the
/// designation still standing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverdueReexamination {
    pub designation_cid: Cid,
    /// The designated community — the party whose exclusion is being left in
    /// place unexamined.
    pub subject: String,
    /// `Anathema`; `HostisHumaniGeneris` is excluded by construction, since that
    /// tier is forbidden a `reexamine_after` (it has no reversal path).
    pub tier: String,
    /// The deadline that has passed unanswered. The designation's own
    /// `reexamine_after` until a re-examination confirms it, and the date that
    /// confirmation set thereafter.
    pub reexamine_after: Timestamp,
    pub seconds_overdue: i64,
    /// How many times the designation has been re-examined and confirmed. A
    /// designation overdue on its *third* renewal is a different reading from one
    /// nobody ever looked at, and before the conclusion record existed the two were
    /// the same number: zero.
    pub confirmations: usize,
}

/// Designations whose `reexamine_after` has passed as of `as_of` and which are
/// still standing, longest-overdue first.
///
/// The severest act the compact can take is the one with the least
/// follow-through: `Designation::reexamine_after` is stored, surfaced through
/// `ReputationView`, and *required* for the Expedited path, and before this lens
/// nothing computed whether it had arrived. A community could be designated on the
/// expedited path with a mandated re-examination date that no reading would ever
/// raise.
///
/// "Still standing" reuses the reversal set `lookup` already defines — a
/// `DesignationLifted` conduct record naming the designation, or a `ReEntry`
/// answering it — so there is one definition of a reversed designation.
///
/// **The limit this used to carry is closed.** A re-examination that *confirmed* a
/// designation left no record, so the lens could not distinguish "nobody looked" from
/// "the panel looked and let it stand", and reported the weaker claim about both. That
/// is remediation item 5's own named first case, and
/// `kc_escalation::ReexaminationConclusion` is the signed record it asked for. A
/// confirmation now answers the deadline it was held for and sets the next one, so what
/// this lens reports is the deadline standing *unanswered* at `as_of`, with
/// `confirmations` saying how many renewals the designation has been through. A
/// designation overdue on its third renewal and one nobody ever examined are no longer
/// the same reading.
pub fn overdue_reexaminations(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> Vec<OverdueReexamination> {
    let as_of = as_of.unwrap_or_else(time::now);

    // The reversal set, by the same two paths `lookup` uses.
    let mut reversed: BTreeSet<String> = BTreeSet::new();
    for (_, entry) in archive.iter() {
        match entry {
            ArchiveEntry::ConductAttested(s)
                if s.payload.kind == ConductKind::DesignationLifted
                    && s.payload.recorded_at <= as_of =>
            {
                reversed.extend(s.payload.references.iter().map(|r| r.to_string()));
            }
            ArchiveEntry::ReEntryCompleted(s) if s.payload.completed_at <= as_of => {
                reversed.insert(s.payload.designation_cid.to_string());
            }
            _ => {}
        }
    }

    // Every re-examination the archive holds, per designation, in append order. A
    // `Confirmed` conclusion answers the deadline it was held for and sets the next one,
    // so what this lens now reports is the *unanswered* deadline rather than the original
    // one. Before this record existed only a reversal could clear the reading, so a panel
    // that re-examined a designation at the mandated date and let it stand looked exactly
    // like a compact that had forgotten, forever: the lens punished the diligent panel
    // (the erosion audit's finding K3, remediation item 5).
    let mut conclusions: BTreeMap<String, Vec<(Timestamp, Option<Timestamp>, bool)>> =
        BTreeMap::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::ReexaminationConcluded(s) = entry {
            let rc = &s.payload;
            if rc.concluded_at > as_of {
                continue;
            }
            conclusions.entry(rc.designation_cid.to_string()).or_default().push((
                rc.concluded_at,
                rc.next_reexamine_after,
                matches!(rc.outcome, ReexaminationOutcome::Confirmed),
            ));
        }
    }

    let mut overdue: Vec<OverdueReexamination> = Vec::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::DesignationDecided(s) = entry {
            let d = &s.payload;
            let Some(mandated) = d.reexamine_after else { continue };
            if d.designated_at > as_of || reversed.contains(&cid.to_string()) {
                continue;
            }
            // Walk the confirmations forward: each answers the deadline standing at the
            // time and replaces it with the one it set. A confirmation with no next date
            // cannot occur (the authority pass refuses it), so an absent one leaves the
            // deadline where it was rather than clearing it.
            let mut due = mandated;
            let mut confirmations = 0usize;
            let empty = Vec::new();
            let history = conclusions.get(&cid.to_string()).unwrap_or(&empty);
            let mut sorted: Vec<_> = history.clone();
            sorted.sort_by_key(|(at, _, _)| *at);
            for (concluded_at, next, confirmed) in sorted {
                if !confirmed || concluded_at < due {
                    continue;
                }
                confirmations += 1;
                match next {
                    Some(n) => due = n,
                    None => break,
                }
            }
            if due > as_of {
                continue;
            }
            overdue.push(OverdueReexamination {
                designation_cid: *cid,
                subject: d.subject_community.as_str().to_string(),
                tier: format!("{:?}", d.tier),
                reexamine_after: due,
                seconds_overdue: as_of.as_second() - due.as_second(),
                confirmations,
            });
        }
    }
    overdue.sort_by(|a, b| {
        b.seconds_overdue
            .cmp(&a.seconds_overdue)
            .then_with(|| a.subject.cmp(&b.subject))
    });
    overdue
}

/// One community's re-affirmation overdue against the cadence, with the numbers
/// the reading was made from so the corps can check it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverdueReaffirmation {
    pub community_did: String,
    /// The last sealed rite, so the finding is traceable to a signed record.
    pub last_rite_cid: Cid,
    pub seconds_since_last: i64,
    /// The cadence it was judged against — reported rather than assumed, because
    /// the calibration is an open question (see [`ErosionThresholds`]).
    pub cadence_seconds: i64,
}

/// The cut-points the erosion alert judges *absolute* measures against — the ones
/// whose reference is a fixed cadence rather than the polity's own recent
/// behaviour.
///
/// This exists because uniform drift is invisible to every relative measure. A
/// decline that moves a whole population equally leaves every ratio unchanged, so
/// only a reference the population does not supply can see it (see
/// `doc/architecture/erosion-legibility.md`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErosionThresholds {
    /// How long a standing function may go without either an act or an attested null
    /// before the silence is reported (erosion remediation items 5 and 6).
    ///
    /// Shorter than the rite cadence by design and for a stated reason: a
    /// re-affirmation is generational, while mediating and observing are the ordinary
    /// running of the compact. A function that has produced nothing and said nothing
    /// for a quarter is not yet a scandal, which is why this reports rather than
    /// refuses, but it is a fact the corps should be reading.
    ///
    /// **It must sit inside the shortest roster term, and the first value chosen here
    /// did not.** At one year it exactly equalled `DEFAULT_OBSERVER_MAX_TERM_SECONDS`,
    /// so an observer corps could never be silent for a full cadence while staffed: its
    /// seats expired first and it read as *unstaffed* instead. A silence reading longer
    /// than the term it watches is unreachable by construction, and the fix is the
    /// cadence rather than the term, since the term limit is doctrine (an observer must
    /// return home) and the cadence is a calibration. Ninety days, matching §8's
    /// rotation period.
    pub attestation_cadence_seconds: i64,
    /// How long after its last sealed rite a community's re-affirmation counts as
    /// overdue.
    ///
    /// The corpus default is generational, 25–30 years, and
    /// `protocol-components.md` records both that the calibration lacks empirical
    /// support and that the eventual shape is a compact-formation parameter with
    /// per-community override rather than a constant. This field is the
    /// compact-wide default until that parameter exists; the *upper* end of the
    /// band is used so an unsettled calibration cannot manufacture false Notices.
    pub rite_cadence_seconds: i64,
    /// How far back the substitution reading looks when asking what the compact
    /// currently enforces ([`enforcement_convergence`]).
    ///
    /// Between the other two cadences, and for a stated reason. Shorter than the
    /// generational rite cadence, or a compact's founding decade averages away a
    /// narrowing that happened last year; longer than the quarterly attestation
    /// cadence, or a polity that enforces rarely never accumulates enough acts to
    /// read at all. Five years. Like both of its neighbours this is a calibration
    /// without empirical support behind it, and eventually a compact-formation
    /// parameter rather than a constant.
    pub enforcement_window_seconds: i64,
    /// Fewest enforcement acts in the window before the substitution readings say
    /// anything. Below it they report **untestable**, not healthy.
    ///
    /// Twelve, chosen against the arithmetic rather than picked. Coverage over
    /// eight concrete grounds is a coupon-collector reading: twelve acts drawn
    /// evenly across the taxonomy are expected to touch `8·(1−(7/8)¹²) ≈ 6.4` of
    /// them, a coverage of about 0.80, which sits well clear of the 0.5 floor. At
    /// eight acts the same expectation is 0.66, close enough to the floor that
    /// ordinary variance in a small young compact would trip it. The floor is not
    /// "enough data" in the abstract; it is enough that a polity enforcing the
    /// whole object cannot look like one that has stopped.
    pub enforcement_volume_floor: usize,
    /// Coverage at or above this is not convergence. Permille, so the struct keeps
    /// `Eq` and the cut-point stays an exact integer rather than a float nobody can
    /// compare. 500‰ — half the taxonomy.
    pub enforcement_coverage_floor_permille: u32,
    /// Target Herfindahl **below** this counts as dispersed across subjects.
    ///
    /// The discriminator between substitution and a genuinely recurring problem, so
    /// it decides more than any other cut-point here. 500‰ is an effective target
    /// count of two: enforcement landing on one or two communities is their
    /// problem, and enforcement on the same ground landing on three or more is the
    /// compact's.
    pub enforcement_dispersion_ceiling_permille: u32,
    /// Downward share at or above this counts as enforcement running downward.
    /// 800‰, leaving room for the scrutiny of entrants that §2 probation exists to
    /// license.
    pub enforcement_downward_floor_permille: u32,
}

impl Default for ErosionThresholds {
    fn default() -> Self {
        ErosionThresholds {
            attestation_cadence_seconds: 90 * 86_400,
            rite_cadence_seconds: 30 * 365 * 86_400,
            enforcement_window_seconds: 5 * 365 * 86_400,
            enforcement_volume_floor: 12,
            enforcement_coverage_floor_permille: 500,
            enforcement_dispersion_ceiling_permille: 500,
            enforcement_downward_floor_permille: 800,
        }
    }
}

/// One standing function of one community, with how long it has been since that function
/// last produced either an act or an attested null.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SilentFunction {
    pub community_did: String,
    /// `Mediation` or `Observation`.
    pub function: String,
    /// Seconds since the last act or attestation. Measured from the seat's start when
    /// there has never been either, because a function that has done nothing since the
    /// day it was seated is the case the audit is most interested in.
    pub seconds_since: i64,
    pub cadence_seconds: i64,
    /// True when the last thing on the record was an attested null rather than an act:
    /// the compact said "nothing happened", which is evidence, and then stopped saying
    /// even that.
    pub last_was_a_null: bool,
}

/// One standing function a community once held and now has nobody seated in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnstaffedFunction {
    pub community_did: String,
    pub function: String,
    /// When the last seat in this function expired.
    pub vacant_since: Timestamp,
    pub seconds_vacant: i64,
}

/// Functions that have gone quiet: no act and no attested null within the cadence
/// (erosion remediation items 5 and 6).
///
/// An **absolute** measure, which is the whole point. Every relative reading of a
/// mediator pool or an observer corps is unchanged by the pool simply stopping: the
/// shares are the same shares, the concentration is the same concentration, and a corps
/// with nothing to report and a corps that stopped looking file the same number of
/// reports, namely none. Only a measure whose reference is a clock rather than the
/// current population can see it (`erosion-legibility.md`, findings K1 and K4).
///
/// The attested null is what makes the reading fair. Before `ActivityAttestation` existed
/// a quiet pool had no way to say so, and reporting its silence would have been reporting
/// missing data as a finding. Now silence is either evidence (an attestation inside the
/// cadence) or a fact about the compact (nothing at all), and this lens reports only the
/// second.
///
/// Reports rather than refuses, deliberately: a compact that has stopped acting offers no
/// act to refuse. The one refusal erosion remediation carries is in the authority pass,
/// on citing a lapsed designation.
pub fn silent_functions(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    cadence_seconds: i64,
) -> Vec<SilentFunction> {
    let as_of = as_of.unwrap_or_else(time::now);
    // (community, function) -> (seat start, last act or null, whether it was a null)
    let mut seats: BTreeMap<(String, String), Timestamp> = BTreeMap::new();
    let mut last: BTreeMap<(String, String), (Timestamp, bool)> = BTreeMap::new();

    let note = |map: &mut BTreeMap<(String, String), (Timestamp, bool)>,
                community: String,
                function: AttestedFunction,
                at: Timestamp,
                is_null: bool| {
        let key = (community, function.as_str().to_string());
        match map.get(&key) {
            Some((prev, _)) if *prev >= at => {}
            _ => {
                map.insert(key, (at, is_null));
            }
        }
    };

    // Who holds which function, and from when. Latest roster per community wins, the
    // same resolution the pool lenses use.
    let mut med: BTreeMap<String, &MediatorRegistry> = BTreeMap::new();
    let mut obs: BTreeMap<String, &ObserverRegistry> = BTreeMap::new();
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::MediatorRegistryUpdated(s) if s.payload.issued_at <= as_of => {
                let key = s.payload.nominating_community.as_str().to_string();
                if med.get(&key).is_none_or(|c| s.payload.issued_at >= c.issued_at) {
                    med.insert(key, &s.payload);
                }
            }
            ArchiveEntry::ObserverRegistryUpdated(s) if s.payload.issued_at <= as_of => {
                let key = s.payload.nominating_community.as_str().to_string();
                if obs.get(&key).is_none_or(|c| s.payload.issued_at >= c.issued_at) {
                    obs.insert(key, &s.payload);
                }
            }
            _ => {}
        }
    }
    for reg in med.values() {
        for en in &reg.entries {
            if en.active_at(as_of) {
                let key = (
                    reg.nominating_community.as_str().to_string(),
                    AttestedFunction::Mediation.as_str().to_string(),
                );
                let start = en.valid_from;
                match seats.get(&key) {
                    Some(prev) if *prev <= start => {}
                    _ => {
                        seats.insert(key, start);
                    }
                }
            }
        }
    }
    for reg in obs.values() {
        for en in &reg.entries {
            if en.active_at(as_of) {
                let key = (
                    reg.nominating_community.as_str().to_string(),
                    AttestedFunction::Observation.as_str().to_string(),
                );
                let start = en.valid_from;
                match seats.get(&key) {
                    Some(prev) if *prev <= start => {}
                    _ => {
                        seats.insert(key, start);
                    }
                }
            }
        }
    }

    // Acts and nulls. A mediation act is credited to the *panelists'* home communities,
    // since that is who exercised the function; a dispute filing is not an exercise of
    // the pool and does not count.
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::ActivityAttested(s) if s.payload.attested_at <= as_of => {
                note(
                    &mut last,
                    s.payload.attesting_community.as_str().to_string(),
                    s.payload.function,
                    s.payload.window_end.min(s.payload.attested_at),
                    s.payload.acts.is_empty(),
                );
            }
            ArchiveEntry::MediationDecided(s) if s.payload.decided_at <= as_of => {
                for panelist in &s.payload.panel {
                    if let Some(home) = mediator_home_at(&med, panelist.as_str(), s.payload.decided_at)
                    {
                        note(
                            &mut last,
                            home,
                            AttestedFunction::Mediation,
                            s.payload.decided_at,
                            false,
                        );
                    }
                }
            }
            ArchiveEntry::ObserverReportPublished(s) if s.payload.reported_at <= as_of => {
                if let Some(home) = observer_home_at(&obs, s.payload.observer_did.as_str(), s.payload.reported_at)
                {
                    note(
                        &mut last,
                        home,
                        AttestedFunction::Observation,
                        s.payload.reported_at,
                        false,
                    );
                }
            }
            _ => {}
        }
    }

    let mut out: Vec<SilentFunction> = Vec::new();
    for ((community, function), seated_from) in seats {
        let key = (community.clone(), function.clone());
        let (since, was_null) = match last.get(&key) {
            Some((at, is_null)) => (*at, *is_null),
            // Never an act and never a null: measured from the seat, so a function that
            // has done nothing since the day it was staffed reads as what it is.
            None => (seated_from, false),
        };
        let elapsed = as_of.as_second() - since.as_second();
        if elapsed > cadence_seconds {
            out.push(SilentFunction {
                community_did: community,
                function,
                seconds_since: elapsed,
                cadence_seconds,
                last_was_a_null: was_null,
            });
        }
    }
    out.sort_by(|a, b| {
        b.seconds_since
            .cmp(&a.seconds_since)
            .then_with(|| a.community_did.cmp(&b.community_did))
            .then_with(|| a.function.cmp(&b.function))
    });
    out
}

/// The home community of a mediator at `at`, from an already-resolved roster map.
fn mediator_home_at(
    rosters: &BTreeMap<String, &MediatorRegistry>,
    mediator_did: &str,
    at: Timestamp,
) -> Option<String> {
    rosters.values().find_map(|reg| {
        reg.entries
            .iter()
            .any(|e| e.mediator_did.as_str() == mediator_did && e.active_at(at))
            .then(|| reg.nominating_community.as_str().to_string())
    })
}

/// The home community of an observer at `at`, from an already-resolved roster map.
fn observer_home_at(
    rosters: &BTreeMap<String, &ObserverRegistry>,
    observer_did: &str,
    at: Timestamp,
) -> Option<String> {
    rosters.values().find_map(|reg| {
        reg.entries
            .iter()
            .any(|e| e.observer_did.as_str() == observer_did && e.active_at(at))
            .then(|| reg.nominating_community.as_str().to_string())
    })
}

/// Functions a community once staffed and now has nobody seated in.
///
/// The sharper half of the same failure, and the one neither remediation item named.
/// Measuring the tier found that a compact with **zero** active mediators and **zero**
/// active observers for forty-three years, keeping its re-affirmation rites perfectly,
/// read clean on every instrument: `deliberative_capacity` reads the member roll and
/// probation, the pool lenses report shares of an empty pool without remarking that it is
/// empty, and no alert dimension looked at staffing at all. An unexercised function and an
/// abolished one are different failures, and this is the second.
///
/// A function **never** staffed is not reported: a compact that never seated observers has
/// not abandoned the office, it declined to open it, and the absence of an office is not
/// the same as an abandoned one. That is this lens's negative control.
pub fn unstaffed_functions(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
) -> Vec<UnstaffedFunction> {
    let as_of = as_of.unwrap_or_else(time::now);
    // (community, function) -> (ever seated, currently seated, last term end)
    let mut state: BTreeMap<(String, String), (bool, bool, Option<Timestamp>)> = BTreeMap::new();

    let mut record = |community: String,
                      function: AttestedFunction,
                      active_now: bool,
                      term_end: Option<Timestamp>| {
        let key = (community, function.as_str().to_string());
        let e = state.entry(key).or_insert((false, false, None));
        e.0 = true;
        e.1 |= active_now;
        // The latest end among expired seats is when the office actually fell vacant.
        if let Some(end) = term_end {
            if e.2.is_none_or(|prev| end > prev) {
                e.2 = Some(end);
            }
        }
    };

    let mut med: BTreeMap<String, &MediatorRegistry> = BTreeMap::new();
    let mut obs: BTreeMap<String, &ObserverRegistry> = BTreeMap::new();
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::MediatorRegistryUpdated(s) if s.payload.issued_at <= as_of => {
                let key = s.payload.nominating_community.as_str().to_string();
                if med.get(&key).is_none_or(|c| s.payload.issued_at >= c.issued_at) {
                    med.insert(key, &s.payload);
                }
            }
            ArchiveEntry::ObserverRegistryUpdated(s) if s.payload.issued_at <= as_of => {
                let key = s.payload.nominating_community.as_str().to_string();
                if obs.get(&key).is_none_or(|c| s.payload.issued_at >= c.issued_at) {
                    obs.insert(key, &s.payload);
                }
            }
            _ => {}
        }
    }
    for reg in med.values() {
        for en in &reg.entries {
            if en.valid_from > as_of {
                continue;
            }
            record(
                reg.nominating_community.as_str().to_string(),
                AttestedFunction::Mediation,
                en.active_at(as_of),
                en.valid_until.filter(|end| *end <= as_of),
            );
        }
    }
    for reg in obs.values() {
        for en in &reg.entries {
            if en.valid_from > as_of {
                continue;
            }
            record(
                reg.nominating_community.as_str().to_string(),
                AttestedFunction::Observation,
                en.active_at(as_of),
                en.valid_until.filter(|end| *end <= as_of),
            );
        }
    }

    let mut out: Vec<UnstaffedFunction> = Vec::new();
    for ((community, function), (ever, active_now, vacant_since)) in state {
        if !ever || active_now {
            continue;
        }
        let Some(since) = vacant_since else { continue };
        out.push(UnstaffedFunction {
            community_did: community,
            function,
            vacant_since: since,
            seconds_vacant: as_of.as_second() - since.as_second(),
        });
    }
    out.sort_by(|a, b| {
        b.seconds_vacant
            .cmp(&a.seconds_vacant)
            .then_with(|| a.community_did.cmp(&b.community_did))
            .then_with(|| a.function.cmp(&b.function))
    });
    out
}

// ---------------------------------------------------------------------------
// Substitution: what the compact actually enforces
// (`doc/background/substitution.md`; erosion-legibility.md findings K5 and K6).
// ---------------------------------------------------------------------------

/// The concrete grounds a conduct record can bear on: the five anti-spiral
/// features plus the three named obligations. Fixed order, fixed size, and it is
/// the denominator every coverage reading below divides by.
///
/// `ConductBearing::Other(_)` is deliberately absent. It is open-ended, so
/// admitting it to the denominator would let the thing being measured supply its
/// own reference — the one property this reading cannot have, for the reason
/// `erosion-legibility.md` gives: a measure that moves with its subject cannot see
/// a drift that moves the subject uniformly.
pub const CONCRETE_BEARINGS: [&str; 8] = [
    "FallenActorAnthropology",
    "GraceOverMerit",
    "ScapegoatDisarming",
    "EnemyLoveOperational",
    "NonUltimacyOfCompact",
    "CommonsObligation",
    "Charity",
    "Rescue",
];

/// The key a bearing counts under. Concrete grounds get their own name; each
/// distinct `Other` string is its own ground, prefixed so it can never collide
/// with a concrete one.
fn bearing_key(b: &ConductBearing) -> String {
    match b {
        ConductBearing::AntiSpiral(f) => feature_key(f).to_string(),
        ConductBearing::CommonsObligation => "CommonsObligation".to_string(),
        ConductBearing::Charity => "Charity".to_string(),
        ConductBearing::Rescue => "Rescue".to_string(),
        ConductBearing::Other(s) => format!("Other:{s}"),
    }
}

/// Sum of squared shares — the Herfindahl index. `1.0` is everything on one
/// bucket; `1/k` is `k` buckets drawn evenly. `None` when there is nothing to
/// index, which is a different reading from "perfectly dispersed" and must not be
/// collapsed into one.
fn herfindahl(counts: &BTreeMap<String, usize>, total: usize) -> Option<f64> {
    if total == 0 {
        return None;
    }
    let t = total as f64;
    Some(counts.values().map(|c| (*c as f64 / t).powi(2)).sum())
}

/// One enforcement act, lifted out of the archive so both substitution lenses
/// walk it once and agree on what counts.
struct EnforcementAct<'a> {
    cid: Cid,
    bearing: &'a ConductBearing,
    subject: &'a DidKey,
    reporters: &'a [DidKey],
    recorded_at: Timestamp,
}

/// Every enforcement act recorded inside the window ending at `as_of`.
///
/// "Enforcement" is `dispute_ladder_rank(rung).is_some()`, which is narrower than
/// `ladder_rung.is_some()` by exactly one variant: `SubstantiveReEngagement` sits
/// on the ladder enum but is the ceremony-cost track rather than a dispute rung,
/// and counting a community's re-engagement as an act of enforcement against it
/// would read the compact's most hopeful record as one of its harshest.
fn enforcement_acts(
    archive: &InMemoryArchive,
    as_of: Timestamp,
    window_seconds: i64,
) -> Vec<EnforcementAct<'_>> {
    let floor = as_of.as_second() - window_seconds;
    let mut acts = Vec::new();
    for (cid, entry) in archive.iter() {
        if let ArchiveEntry::ConductAttested(signed) = entry {
            let c = &signed.payload;
            if c.recorded_at > as_of || c.recorded_at.as_second() <= floor {
                continue;
            }
            let Some(rung) = c.ladder_rung else { continue };
            if dispute_ladder_rank(rung).is_none() {
                continue;
            }
            acts.push(EnforcementAct {
                cid: *cid,
                bearing: &c.bearing,
                subject: &c.subject_community,
                reporters: &c.reporters,
                recorded_at: c.recorded_at,
            });
        }
    }
    acts.sort_by_key(|a| a.recorded_at);
    acts
}

/// What a compact's enforcement is aimed at: how much of it there is, how much of
/// the shared taxonomy it still cites, and how widely it is spread over subjects.
///
/// The substitution reading (`doc/background/substitution.md`). A compact keeping
/// every form and intensifying it — rites ahead of cadence, witness counts rising,
/// nothing amended, no office vacant — can still narrow what it actually enforces
/// down to a single ground, and every instrument that counts observances reads that
/// polity as healthy. This one counts what the observances are *about*.
///
/// Renders no verdict, like every lens in this module: the numbers are reported
/// with the CIDs behind them and the corps decides what they mean.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnforcementConvergence {
    /// The enforcing community, or `None` for the compact-wide reading.
    pub community: Option<String>,
    /// Enforcement acts inside the window.
    pub volume: usize,
    /// Per concrete ground, how many acts cited it. Grounds never cited are
    /// absent, so `per_bearing.len()` is the coverage numerator.
    pub per_bearing: BTreeMap<String, usize>,
    /// Acts whose ground was `Other(_)`, keyed by the string the filer chose.
    pub per_other: BTreeMap<String, usize>,
    /// Per subject community, how many acts were aimed at it.
    pub per_subject: BTreeMap<String, usize>,
    pub window_seconds: i64,
    pub evidence: Vec<Cid>,
}

impl EnforcementConvergence {
    /// Acts landing off the shared taxonomy.
    pub fn other_acts(&self) -> usize {
        self.per_other.values().sum()
    }

    /// Distinct concrete grounds cited, over the whole concrete taxonomy.
    ///
    /// Absolute by construction: the denominator is a fixed enum, not the
    /// compact's own recent behaviour. A polity that cites `Other` for everything
    /// scores `0.0` here rather than `1.0`, which is the correct reading and the
    /// reason no separate off-taxonomy rule is needed.
    pub fn coverage(&self) -> f64 {
        self.per_bearing.len() as f64 / CONCRETE_BEARINGS.len() as f64
    }

    /// Herfindahl over grounds, counting each distinct `Other` string as its own.
    pub fn concentration(&self) -> Option<f64> {
        let mut all = self.per_bearing.clone();
        for (k, v) in &self.per_other {
            all.insert(k.clone(), *v);
        }
        herfindahl(&all, self.volume)
    }

    /// Herfindahl over subject communities. **The discriminator**: a polity with
    /// one genuinely recurring problem also shows high volume and low coverage,
    /// and what separates it from substitution is that legitimate concentration
    /// lands on a *target* while substitution concentrates on a *ground* across
    /// many targets.
    pub fn target_dispersion(&self) -> Option<f64> {
        herfindahl(&self.per_subject, self.volume)
    }

    /// Share of acts citing a ground outside the shared taxonomy.
    pub fn other_share(&self) -> Option<f64> {
        if self.volume == 0 {
            None
        } else {
            Some(self.other_acts() as f64 / self.volume as f64)
        }
    }

    /// Whether this reading is substitution-shaped: enough acts to read at all,
    /// citing too little of the taxonomy, spread too widely over subjects to be
    /// one community's recurring problem.
    pub fn converging(&self, thresholds: &ErosionThresholds) -> bool {
        self.volume >= thresholds.enforcement_volume_floor
            && self.coverage() < thresholds.enforcement_coverage_floor_permille as f64 / 1000.0
            && self.target_dispersion().is_some_and(|h| {
                h < thresholds.enforcement_dispersion_ceiling_permille as f64 / 1000.0
            })
    }
}

/// The compact-wide enforcement reading over the window ending at `as_of`.
///
/// **This is the level the erosion alert reads, and the per-community map is not.**
/// Measured: a compact whose communities each specialise — one filing only on
/// `Charity`, another only on `Rescue` — scores as converged on every single
/// per-community reading while the compact as a whole still enforces the entire
/// object. That is division of labour in an observer corps, not a narrowing of what
/// the compact holds, and a per-community trigger would report it as erosion.
/// [`enforcement_convergence_all`] is the drill-down that names who, after the
/// compact-wide reading has said whether there is anything to name.
pub fn enforcement_convergence(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    window_seconds: i64,
) -> EnforcementConvergence {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut out = EnforcementConvergence {
        window_seconds,
        ..Default::default()
    };
    for act in enforcement_acts(archive, as_of, window_seconds) {
        tally_act(&mut out, &act);
    }
    out
}

/// Add one act to a convergence reading. Shared so the compact-wide and
/// per-community forms cannot count the same act differently.
fn tally_act(out: &mut EnforcementConvergence, act: &EnforcementAct<'_>) {
    out.volume += 1;
    let key = bearing_key(act.bearing);
    if matches!(act.bearing, ConductBearing::Other(_)) {
        *out.per_other.entry(key).or_insert(0) += 1;
    } else {
        *out.per_bearing.entry(key).or_insert(0) += 1;
    }
    *out
        .per_subject
        .entry(act.subject.as_str().to_string())
        .or_insert(0) += 1;
    out.evidence.push(act.cid);
}

/// The same reading per **enforcing** community, keyed by reporter DID.
///
/// An act filed by three reporters counts once for each of them: the question this
/// answers is what a given community's enforcement is aimed at, and all three of
/// them filed it. Only communities that have filed at least one enforcement act in
/// the window appear.
pub fn enforcement_convergence_all(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    window_seconds: i64,
) -> BTreeMap<String, EnforcementConvergence> {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut by_community: BTreeMap<String, EnforcementConvergence> = BTreeMap::new();
    for act in enforcement_acts(archive, as_of, window_seconds) {
        for reporter in act.reporters {
            let did = reporter.as_str().to_string();
            let entry = by_community.entry(did.clone()).or_insert_with(|| {
                EnforcementConvergence {
                    community: Some(did),
                    window_seconds,
                    ..Default::default()
                }
            });
            tally_act(entry, &act);
        }
    }
    by_community
}

/// Which way a compact's enforcement runs: at communities holding reduced
/// standing, at communities holding full standing, or between equals.
///
/// The Girardian half of the substitution reading. A narrowed ground does not land
/// evenly; it lands on whoever is least able to refuse it, and §2 probation is the
/// compact's own name for reduced standing, so the axis is already on the record.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnforcementDirection {
    /// The enforcing community, or `None` for the compact-wide reading.
    pub community: Option<String>,
    /// Subject in probation, no reporter in probation.
    pub downward: usize,
    /// A reporter in probation, subject not.
    pub upward: usize,
    /// Neither in probation, or both. Enforcement between equals.
    pub level: usize,
    pub window_seconds: i64,
    /// The downward acts only — the ones a reader would want to open first.
    pub evidence: Vec<Cid>,
}

impl EnforcementDirection {
    pub fn total(&self) -> usize {
        self.downward + self.upward + self.level
    }

    pub fn downward_share(&self) -> Option<f64> {
        let t = self.total();
        if t == 0 {
            None
        } else {
            Some(self.downward as f64 / t as f64)
        }
    }

    /// Whether enforcement runs downward: enough acts to read, and a dominant
    /// share of them aimed at communities holding reduced standing.
    ///
    /// Thresholded rather than absolute, because *some* downward skew is what
    /// probation is for. A compact that scrutinises its entrants more closely than
    /// its founders is doing what §2 asks of it; one whose enforcement has stopped
    /// reaching full members at all is doing something else.
    pub fn runs_downward(&self, thresholds: &ErosionThresholds) -> bool {
        self.total() >= thresholds.enforcement_volume_floor
            && self.downward_share().is_some_and(|s| {
                s >= thresholds.enforcement_downward_floor_permille as f64 / 1000.0
            })
    }
}

/// Classify one act, resolving each party's standing at the instant the act was
/// recorded rather than at `as_of`: the question is who held what standing when
/// the compact moved against them.
fn classify_direction(
    archive: &InMemoryArchive,
    act: &EnforcementAct<'_>,
    memo: &mut HashMap<(String, i64), bool>,
) -> Ordering {
    let mut probationary = |did: &DidKey| -> bool {
        let key = (did.as_str().to_string(), act.recorded_at.as_second());
        if let Some(hit) = memo.get(&key) {
            return *hit;
        }
        let v = probationary_at(archive, did, act.recorded_at);
        memo.insert(key, v);
        v
    };
    let subject_down = probationary(act.subject);
    let reporter_down = act.reporters.iter().any(|r| probationary(r));
    match (subject_down, reporter_down) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// The compact-wide direction reading over the window ending at `as_of`.
pub fn enforcement_direction(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    window_seconds: i64,
) -> EnforcementDirection {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut out = EnforcementDirection {
        window_seconds,
        ..Default::default()
    };
    let mut memo = HashMap::new();
    for act in enforcement_acts(archive, as_of, window_seconds) {
        tally_direction(&mut out, &act, classify_direction(archive, &act, &mut memo));
    }
    out
}

fn tally_direction(out: &mut EnforcementDirection, act: &EnforcementAct<'_>, dir: Ordering) {
    match dir {
        Ordering::Less => {
            out.downward += 1;
            out.evidence.push(act.cid);
        }
        Ordering::Greater => out.upward += 1,
        Ordering::Equal => out.level += 1,
    }
}

/// The same reading per **enforcing** community, keyed by reporter DID.
pub fn enforcement_direction_all(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    window_seconds: i64,
) -> BTreeMap<String, EnforcementDirection> {
    let as_of = as_of.unwrap_or_else(time::now);
    let mut by_community: BTreeMap<String, EnforcementDirection> = BTreeMap::new();
    let mut memo = HashMap::new();
    for act in enforcement_acts(archive, as_of, window_seconds) {
        let dir = classify_direction(archive, &act, &mut memo);
        for reporter in act.reporters {
            let did = reporter.as_str().to_string();
            let entry = by_community.entry(did.clone()).or_insert_with(|| {
                EnforcementDirection {
                    community: Some(did),
                    window_seconds,
                    ..Default::default()
                }
            });
            tally_direction(entry, &act, dir);
        }
    }
    by_community
}

/// Compute the erosion alert as of `as_of` (defaults to now), with the default
/// [`ErosionThresholds`]. See [`erosion_alert_with`] for the parameterized form.
pub fn erosion_alert(archive: &InMemoryArchive, as_of: Option<Timestamp>) -> ErosionAlert {
    erosion_alert_with(archive, as_of, &ErosionThresholds::default())
}

/// Compute the erosion alert as of `as_of` (defaults to now). Escalation rule:
/// count the distinct eroded dimensions; zero → `None`, exactly one isolated
/// instance → `Notice`, and two-or-more dimensions **or** any dimension eroded more
/// than once → `Alert`.
///
/// The twelve dimensions divide three ways, and the division is the whole design.
/// Four fire on an **act**: somebody amends the incapable list, narrows the adopted
/// ground, shortens a fast, or deliberates during one. Six fire on **omission** or
/// against a pinned baseline: a re-affirmation that never came, a ceremony drawing
/// fewer witnesses than at its founding, a thinning deliberation, a designation left
/// unexamined, a function gone quiet, an office left empty. The last two fire on
/// **intensification**: a compact enforcing harder and narrower than it recites.
///
/// An alert built only from act-triggered signals reads a polity that has stopped
/// functioning as a polity in perfect health. An alert built only from those two
/// families reads a polity that has kept every form and substituted a smaller object
/// underneath them as its most vigorous member.
///
/// Diagnostic only, unchanged: nothing is signed, no record is emitted, and an
/// overdue re-affirmation is *reported*, never *enforced*. Whether the cadence
/// acquires teeth of its own (expiry, escalation) is an open question in
/// `protocol-components.md` and is deliberately not settled here.
pub fn erosion_alert_with(
    archive: &InMemoryArchive,
    as_of: Option<Timestamp>,
    thresholds: &ErosionThresholds,
) -> ErosionAlert {
    // Resolved once and shared by every sub-lens: with `as_of: None` each would
    // otherwise call `time::now` separately and grade against a different instant.
    let as_of_ts = as_of.unwrap_or_else(time::now);
    let r = substrate_deference(archive, Some(as_of_ts));

    let list_contraction = r.incapable_removed > 0;
    let minimum_contraction = r.minimum_narrowed > 0;
    let fast_contraction = r.fast_contractions > 0;
    let deviations = r.fast_deviations.len();

    // The two absolute signals, read off every community's rite series in one pass.
    let mut overdue_reaffirmations: Vec<OverdueReaffirmation> = Vec::new();
    let mut hollowing_communities: Vec<String> = Vec::new();
    for (did, decay) in ceremony_cost_decay_all(archive, Some(as_of_ts)) {
        if let (Some(since), Some(last)) = (decay.seconds_since_last, decay.rites.last()) {
            if since > thresholds.rite_cadence_seconds {
                overdue_reaffirmations.push(OverdueReaffirmation {
                    community_did: did.clone(),
                    last_rite_cid: last.record_cid,
                    seconds_since_last: since,
                    cadence_seconds: thresholds.rite_cadence_seconds,
                });
            }
        }
        if decay.witness_count_declined() {
            hollowing_communities.push(did);
        }
    }
    // Worst first: the longest-overdue community is the one the corps should read
    // about at the top of the alert.
    overdue_reaffirmations.sort_by(|a, b| {
        b.seconds_since_last
            .cmp(&a.seconds_since_last)
            .then_with(|| a.community_did.cmp(&b.community_did))
    });

    let mut reasons = Vec::new();
    let mut dimensions = 0usize;
    let mut compounding = false;

    if list_contraction {
        dimensions += 1;
        reasons.push(format!(
            "compact-incapable list contracted: {} matter(s) removed",
            r.incapable_removed
        ));
        if r.incapable_removed > 1 {
            compounding = true;
        }
    }
    if minimum_contraction {
        dimensions += 1;
        reasons.push(format!(
            "the compact's own adopted ground narrowed: {} forbearance(s) removed",
            r.minimum_narrowed
        ));
        if r.minimum_narrowed > 1 {
            compounding = true;
        }
    }
    if fast_contraction {
        dimensions += 1;
        reasons.push(format!(
            "structured-fast cadence contracted: {} shortening amendment(s)",
            r.fast_contractions
        ));
        if r.fast_contractions > 1 {
            compounding = true;
        }
    }
    if deviations > 0 {
        dimensions += 1;
        reasons.push(format!(
            "{deviations} deliberative act(s) initiated during a fast"
        ));
        if deviations > 1 {
            compounding = true;
        }
    }
    if !overdue_reaffirmations.is_empty() {
        dimensions += 1;
        reasons.push(format!(
            "{} community/communities overdue for re-affirmation past the {}-day cadence",
            overdue_reaffirmations.len(),
            thresholds.rite_cadence_seconds / 86_400
        ));
        // More than one community late, or one late by more than a second full
        // cadence, is no longer an isolated lapse.
        if overdue_reaffirmations.len() > 1
            || overdue_reaffirmations
                .iter()
                .any(|o| o.seconds_since_last > 2 * thresholds.rite_cadence_seconds)
        {
            compounding = true;
        }
    }
    if !hollowing_communities.is_empty() {
        dimensions += 1;
        reasons.push(format!(
            "{} community/communities drawing fewer witnesses than at founding",
            hollowing_communities.len()
        ));
        if hollowing_communities.len() > 1 {
            compounding = true;
        }
    }
    // §4's deliberation, on the same pinned baseline as the rite signals above.
    let delib = deliberation_decay(archive, Some(as_of_ts));
    let count_declined = delib.argument_count_declined();
    let span_shortened = delib.span_shortened();
    let thinning_deliberation = if count_declined || span_shortened {
        let first = delib.decisions.first().expect("a reading fired, so there are two");
        let latest = delib.decisions.last().expect("a reading fired, so there are two");
        Some(ThinningDeliberation {
            decisions: delib.decisions.len(),
            first_argument_count: first.argument_count,
            latest_argument_count: latest.argument_count,
            first_span_seconds: first.span_seconds,
            latest_span_seconds: latest.span_seconds,
            argument_count_declined: count_declined,
            span_shortened,
        })
    } else {
        None
    };
    if let Some(t) = &thinning_deliberation {
        dimensions += 1;
        let mut which = Vec::new();
        if t.argument_count_declined {
            which.push(format!(
                "{} argument(s) answered where the first decision answered {}",
                t.latest_argument_count, t.first_argument_count
            ));
        }
        if t.span_shortened {
            which.push(format!(
                "reached in {}s where the first took {}s",
                t.latest_span_seconds.unwrap_or_default(),
                t.first_span_seconds.unwrap_or_default()
            ));
        }
        reasons.push(format!("deliberation thinning: {}", which.join("; ")));
        // Both readings firing is no longer one isolated measure moving.
        if t.argument_count_declined && t.span_shortened {
            compounding = true;
        }
    }
    let overdue_reexaminations = overdue_reexaminations(archive, Some(as_of_ts));
    if !overdue_reexaminations.is_empty() {
        dimensions += 1;
        reasons.push(format!(
            "{} designation(s) past a mandated re-examination and still standing",
            overdue_reexaminations.len()
        ));
        if overdue_reexaminations.len() > 1 {
            compounding = true;
        }
    }

    // The two function signals. They cannot double-count, and the reason is structural
    // rather than a rule applied here: [`silent_functions`] reads only functions with a
    // seat active at `as_of`, and [`unstaffed_functions`] reports only functions with
    // none, so a single lapse contributes exactly one dimension.
    //
    // Worth stating because the guard against it was written first and then measured
    // away. A filter here would have been a rule against an impossible state, which is
    // the shape this codebase keeps finding on the other side (a value no rule reads);
    // an invariant that holds by construction belongs in the lens definitions and in the
    // test that pins it, not in a redundant caller check.
    let unstaffed = unstaffed_functions(archive, Some(as_of_ts));
    let silent = silent_functions(
        archive,
        Some(as_of_ts),
        thresholds.attestation_cadence_seconds,
    );

    if !silent.is_empty() {
        dimensions += 1;
        reasons.push(format!(
            "{} standing function(s) with no act and no attested null inside the cadence",
            silent.len()
        ));
        // More than one function gone quiet, or one quiet for more than twice the
        // cadence, is no longer an isolated lull. The same shape as the rite reading.
        if silent.len() > 1
            || silent
                .iter()
                .any(|f| f.seconds_since > 2 * thresholds.attestation_cadence_seconds)
        {
            compounding = true;
        }
    }
    if !unstaffed.is_empty() {
        dimensions += 1;
        reasons.push(format!(
            "{} standing function(s) with nobody seated at all",
            unstaffed.len()
        ));
        // The same duration rule the silence reading uses, and for the same reason.
        // Without it a compact whose office has stood empty for years graded *below*
        // one still seated and merely quiet, which is backwards: the vacancy is the
        // stronger claim, so it must not be the weaker grade.
        if unstaffed.len() > 1
            || unstaffed
                .iter()
                .any(|u| u.seconds_vacant > 2 * thresholds.attestation_cadence_seconds)
        {
            compounding = true;
        }
    }

    // The two substitution signals. Both are read compact-wide rather than per
    // community, because a compact whose members each specialise reads as converged
    // on every per-community measure while still enforcing the whole object between
    // them (see [`enforcement_convergence`]).
    let convergence = enforcement_convergence(
        archive,
        Some(as_of_ts),
        thresholds.enforcement_window_seconds,
    );
    let enforcement_convergence = if convergence.converging(thresholds) {
        Some(convergence)
    } else {
        None
    };
    if let Some(c) = &enforcement_convergence {
        dimensions += 1;
        reasons.push(format!(
            "enforcement converging on a narrowing ground: {} act(s) citing {} of {} grounds, \
             spread across {} subject(s)",
            c.volume,
            c.per_bearing.len(),
            CONCRETE_BEARINGS.len(),
            c.per_subject.len()
        ));
        // One ground carrying the whole of a compact's enforcement is the terminal
        // case, not a step toward it; a majority of acts filed off the shared
        // taxonomy is the same finding by the other route.
        if c.per_bearing.len() <= 1 || c.other_share().is_some_and(|s| s > 0.5) {
            compounding = true;
        }
    }
    let direction = enforcement_direction(
        archive,
        Some(as_of_ts),
        thresholds.enforcement_window_seconds,
    );
    let enforcement_direction = if direction.runs_downward(thresholds) {
        Some(direction)
    } else {
        None
    };
    if let Some(d) = &enforcement_direction {
        dimensions += 1;
        reasons.push(format!(
            "enforcement running downward: {} of {} act(s) aimed at communities holding \
             reduced standing",
            d.downward,
            d.total()
        ));
        // Nothing aimed at a full member in the whole window. The share threshold
        // allows for probation doing its work; this is the case where it is not
        // doing its work but standing in for the object.
        if d.upward == 0 && d.level == 0 {
            compounding = true;
        }
    }

    let severity = match dimensions {
        0 => ErosionSeverity::None,
        1 if !compounding => ErosionSeverity::Notice,
        _ => ErosionSeverity::Alert,
    };

    ErosionAlert {
        severity,
        reasons,
        list_contraction,
        minimum_contraction,
        fast_contraction,
        active_fast_deviations: deviations,
        overdue_reaffirmations,
        hollowing_communities,
        thinning_deliberation,
        overdue_reexaminations,
        silent_functions: silent,
        unstaffed_functions: unstaffed,
        enforcement_convergence,
        enforcement_direction,
    }
}

// ---------------------------------------------------------------------------
// §2 deliberative-capacity lens (how much of the compact carries weight).
// ---------------------------------------------------------------------------

/// How much of a compact's roll carries deliberative weight right now (§2).
///
/// Probation withholds weight from every second-class quorum, so a compact that
/// has just admitted a cohort holds fewer *deciding* members than *members* — and
/// if it admits enough at once, fewer than the quorum needs. That is a correct
/// consequence of the withholding and an operationally severe one: no second-class
/// decision is available until a window closes. It is surfaced here rather than
/// left to be discovered as a refusal nobody can account for.
///
/// It is also a capture signal in its own right, read in the opposite direction:
/// a roll whose standing share is falling is a compact taking in members faster
/// than it can season them, which is the growth pressure
/// `capture_resistance.md` §7 names as the historical failure mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliberativeCapacity {
    /// The compact this reads, by self-declared label (the §2 honest limit).
    pub compact_label: String,
    /// The derived roll: every community admitted under this label, in DID order.
    /// Derived from the admission records, never declared.
    pub members: Vec<DidKey>,
    /// Those of `members` inside a probation window at `as_of`.
    pub probationary: Vec<DidKey>,
    /// The quorum this capacity was judged against.
    pub quorum_threshold: usize,
    pub as_of: Timestamp,
}

impl DeliberativeCapacity {
    /// Members carrying deliberative weight.
    pub fn standing(&self) -> usize {
        self.members.len() - self.probationary.len()
    }

    /// The share of the roll that carries weight, or `None` for an empty roll —
    /// *not evaluated* rather than a zero share, since a compact with no members
    /// is not one whose seasoning has lapsed.
    pub fn standing_share(&self) -> Option<f64> {
        (!self.members.is_empty())
            .then(|| self.standing() as f64 / self.members.len() as f64)
    }

    /// Can the compact take an ordinary second-class decision at all?
    pub fn can_act(&self) -> bool {
        self.standing() >= self.quorum_threshold
    }

    /// Can it take one at the asymmetric *contraction* bar — removing a matter from
    /// the compact-incapable list, shortening a fast? The pair matters because a
    /// compact can be able to expand its protections and unable to contract them,
    /// which is the intended direction of the asymmetry.
    // `>= quorum_threshold + 1` rather than the equivalent `> quorum_threshold` that
    // clippy::int_plus_one prefers, for the same reason ethne keeps
    // `majority(n) + 1 <= n` in en-charter's authority pass: "the ordinary quorum plus
    // one" is the named bar §9 states, and the arithmetically identical form no longer
    // says which rule this enforces. The +1 is also load-bearing rather than
    // decorative — it is what stops the asymmetric contraction bar from collapsing into
    // the ordinary quorum once probationary weight is counted.
    #[allow(clippy::int_plus_one)]
    pub fn can_contract(&self) -> bool {
        self.standing() >= self.quorum_threshold + 1
    }
}

/// Derive [`DeliberativeCapacity`] for one compact label as of a query instant.
///
/// The roll is every participant of every admission record carrying that label,
/// which is the same derivation `kc_archive`'s admitting quorum uses — a lens and a
/// gate that disagreed about who is a member would be worse than either alone.
pub fn deliberative_capacity(
    archive: &InMemoryArchive,
    compact_label: &str,
    quorum_threshold: usize,
    as_of: Option<Timestamp>,
) -> DeliberativeCapacity {
    let as_of = as_of.unwrap_or_else(time::now);
    // Keyed by DID string so the roll dedups and orders without a parse round-trip.
    let mut roll: BTreeMap<String, DidKey> = BTreeMap::new();
    for (_, entry) in archive.iter() {
        if let ArchiveEntry::InterCommunityWitness(s) = entry {
            if s.payload.compact_label != compact_label || s.payload.witnessed_at > as_of {
                continue;
            }
            for p in &s.payload.participants {
                roll.insert(p.as_str().to_string(), p.clone());
            }
        }
    }
    let members: Vec<DidKey> = roll.into_values().collect();
    let probationary = members
        .iter()
        .filter(|d| probationary_at(archive, d, as_of))
        .cloned()
        .collect();
    DeliberativeCapacity {
        compact_label: compact_label.to_string(),
        members,
        probationary,
        quorum_threshold,
        as_of,
    }
}
