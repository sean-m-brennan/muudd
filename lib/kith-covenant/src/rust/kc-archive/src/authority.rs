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
//!
//! That last clause was true only of delegations until **D32**. The archive's four
//! registry steps resolved a DID out of its CID-keyed entry index and took the first
//! match, which is hash order, so a superseded roster could seat a retired key. Both
//! halves now share one definition, `store::latest_rosters`, over the log in append
//! order.

use std::collections::{BTreeMap, BTreeSet};

use kc_ceremony::inter::{ExaminerRegistry, InterCommunityWitnessRecord, SponsorshipRecord};
use kc_commons::{is_second_class_level, sanction_rank, CommonsSanction, SanctionLevel, UsageRecord};
use kc_conduct::{ActivityAttestation, AttestedFunction, DecisionClass};
use kc_constitution::{
    AmendmentKind, CompactIncapableAmendment, FastCadenceAmendment, ForbearanceChange,
    MinimumContentAmendment,
};
use kc_core::{Cid, DidKey, KcError, KcResult, Timestamp};
use kc_escalation::{
    dispute_ladder_rank, is_second_class_rung, Capability, Designation, DesignationTier,
    DesignationUrgency, InjunctionChallenge, InterimInjunction, LadderTransition, ReEntry,
    ReexaminationConclusion, ReexaminationOutcome, RestitutionCompletion,
};
use kc_identity::DelegationScope;
use kc_mediation::{ArgumentRecord, ArgumentRole, MediationDecision};
use kc_observer::{
    ObserverRegistry, ObserverReport, ReportResponse, SourceReveal, WhistleblowerReport,
};
use kc_provenance::{EventActionInvoked, EventReport};

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

/// Below this active-corps size the §8 pool-share (no-majority) rule does not
/// bite: small compacts must be free to staff the observer corps even when one
/// community necessarily supplies a majority (liminal_institutions.md "The
/// mediator pool": for small compacts every community must contribute).
pub const DEFAULT_OBSERVER_POOL_FLOOR: usize = 5;

/// Maximum length in seconds of a single observer nomination window (§8). No
/// open-ended observer standing — a permanent observer corps is the capture the
/// rotation rule exists to prevent (jester_and_dissent.md "Rotation and
/// diversity"). One year.
pub const DEFAULT_OBSERVER_MAX_TERM_SECONDS: i64 = 365 * 24 * 3600;

/// Minimum gap in seconds an observer must sit out between a term that has ended
/// and a re-nomination to active service (§8 rotation-period exclusion,
/// jester_and_dissent.md "the observer's removal and rotation-period exclusion").
/// Ninety days.
pub const DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS: i64 = 90 * 24 * 3600;

/// Structured-skepticism hold (capture move 4): a *qualifying* manufactured event
/// may not drive a recorded action until this window elapses from its report,
/// buying the ambient community time to develop its own interpretation before
/// consequences attach — depriving the manufacturer of the loudest-interpreter
/// advantage (capture_resistance.md move 4; liminal_institutions.md
/// "Manufactured-event resistance"). Seven days.
pub const DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS: i64 = 7 * 24 * 3600;

/// Distinct non-originator corroborations that release a qualifying event from the
/// structured-skepticism hold before its window elapses: once the observable facts
/// are independently confirmed, the delay has served its purpose.
pub const DEFAULT_STRUCTURED_SKEPTICISM_MIN_CORROBORATORS: usize = 2;

/// The earliest a sealed decision's proceedings may open on their own terms (§4): the
/// near end of the whitepaper's "25 to 30 year mark (one generational cycle)". Years are
/// 365 days here, as everywhere in this tier.
pub const MIN_DECISION_SEAL_SECONDS: i64 = 25 * 365 * 24 * 3600;

/// The latest (§4): the far end of the same range. A seal is bounded above as well as
/// below because a deterministic reveal that a panel could set a century out is not a
/// deferral of publication but the end of it.
pub const MAX_DECISION_SEAL_SECONDS: i64 = 30 * 365 * 24 * 3600;

/// Distinct arrivals at `Exclusion` before a commons escalation may be found (§7,
/// condition 2). Two: a community that returns to exclusion after being restored is not
/// failing to understand the rule, which is the distinction `commons_federation.md`
/// draws when it warns that disproportionate response "produces defiance rather than
/// compliance". One offence, however grave, is what the ladder is for.
pub const DEFAULT_COMMONS_ESCALATION_CYCLES: usize = 2;

/// The same count for a life-critical commons (§7, condition 4). One: life support and
/// rescue are what the compact keeps running when it pauses everything else, and a
/// habitat cannot wait out a second cycle to find out whether the first meant anything.
pub const DEFAULT_LIFE_CRITICAL_ESCALATION_CYCLES: usize = 1;

/// How long a subject must have stood at `Exclusion` before a commons escalation may be
/// found (§7, condition 3). One year: restoration is always available from exclusion,
/// and the finding is that it was available and not taken, which cannot be said of a
/// community still in the middle of taking it.
pub const DEFAULT_COMMONS_EXCLUSION_DWELL_SECONDS: i64 = 365 * 24 * 3600;

/// The shortest hold a mediator pool may impose (§4). One hour: the doctrine calibrates
/// the interval to the event class and puts the floor of that range at "hours to days"
/// for most events, and a hold measured in seconds is not a delay, it is a formality
/// that lets a pool appear to have invoked the protocol without invoking it.
pub const MIN_INVOKED_SKEPTICISM_HOLD_SECONDS: i64 = 3600;

/// The longest hold a mediator pool may impose (§4). Six months, the far end of the
/// doctrine's "weeks or months" for high-charge events of major interpretive
/// consequence. There is a ceiling at all because an unbounded hold is not skepticism
/// but suppression, which is the very use the pool's own review exists to catch — and a
/// bound a rule enforces catches it without waiting for anyone to notice.
pub const MAX_INVOKED_SKEPTICISM_HOLD_SECONDS: i64 = 182 * 24 * 3600;

/// The three limits every roster is checked against, carried as one value.
///
/// [`ObserverCaps`], [`MediatorCaps`] and [`ExaminerCaps`] are deliberately distinct
/// types — separately tunable policies, and a caller that swapped two of them should
/// not compile. But all three share exactly this triple, and it is the triple
/// [`check_roster_caps`] enforces, so the shared *rule* gets one parameter rather
/// than three positional numbers that each caller re-spells in the same order.
///
/// The projection is one-way on purpose: a `Caps` yields its `RosterLimits`, and
/// nothing turns a `RosterLimits` back into a `Caps`. `ExaminerCaps::panel_min` has
/// no counterpart here because it is not a roster-cap rule at all; it is checked
/// where an admission is authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RosterLimits {
    /// Active-roster size at or above which the no-majority pool-share rule bites.
    pub pool_floor: usize,
    /// Maximum length (seconds) of a single term.
    pub max_term_seconds: i64,
    /// Minimum sit-out (seconds) between a prior term's end and a re-nomination.
    pub rotation_period_seconds: i64,
}

/// §8 observer-corps caps: the capture-resistance limits on the observer pool
/// (pool-share domination, term length, rotation gap). A [`Default`] carries the
/// `DEFAULT_OBSERVER_*` policy constants; callers may tighten or relax them
/// (e.g. a small ad-hoc compact lowering the floor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObserverCaps {
    /// Active-corps size at or above which the no-majority pool-share rule bites.
    pub pool_floor: usize,
    /// Maximum length (seconds) of a single observer nomination window.
    pub max_term_seconds: i64,
    /// Minimum sit-out (seconds) between a prior term's end and a re-nomination.
    pub rotation_period_seconds: i64,
}

impl Default for ObserverCaps {
    fn default() -> Self {
        Self {
            pool_floor: DEFAULT_OBSERVER_POOL_FLOOR,
            max_term_seconds: DEFAULT_OBSERVER_MAX_TERM_SECONDS,
            rotation_period_seconds: DEFAULT_OBSERVER_ROTATION_PERIOD_SECONDS,
        }
    }
}

impl ObserverCaps {
    /// The three roster limits, for [`check_roster_caps`].
    pub fn limits(&self) -> RosterLimits {
        RosterLimits {
            pool_floor: self.pool_floor,
            max_term_seconds: self.max_term_seconds,
            rotation_period_seconds: self.rotation_period_seconds,
        }
    }
}

/// Below this active-pool size the §2 examining pool-share rule does not bite, for
/// the same reason as the §8 observer floor: a small compact must be able to staff
/// the examining function even when one community necessarily supplies a majority.
pub const DEFAULT_EXAMINER_POOL_FLOOR: usize = 5;

/// Maximum length in seconds of a single examining term (§2). Shorter than the
/// observer term: `capture_resistance.md` §7 calls a standing admissions cadre "the
/// single most capturable body in the compact", and the gate is at its weakest
/// exactly where the entering party is least known. Half a year.
pub const DEFAULT_EXAMINER_MAX_TERM_SECONDS: i64 = 182 * 24 * 3600;

/// Minimum sit-out in seconds between an examining term that has ended and a
/// re-nomination to the pool (§2 rotation, mirroring §8). Ninety days.
pub const DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS: i64 = 90 * 24 * 3600;

/// Minimum number of distinct seated examiners that must witness an admission into a
/// standing compact (§2 cross-community composition). Three, as for a mediation
/// panel: `capture_resistance.md` §7 asks that the credentialing function be rotated
/// and composed "like the mediator pool it resembles".
pub const DEFAULT_EXAMINER_PANEL_MIN: usize = 3;

/// Below this active-pool size the §4 mediator pool-share rule does not bite, for the
/// same reason as the §8 observer and §2 examiner floors. Set at **9** rather than 5:
/// `liminal_institutions.md` puts the minimum viable pool at roughly 3*n* so that a
/// three-person non-party panel exists for any bilateral dispute, and below that a
/// small compact cannot staff mediation at all without one community supplying a
/// majority. The whitepaper lists the viable band as an open question; this is a floor,
/// not an answer to it.
pub const DEFAULT_MEDIATOR_POOL_FLOOR: usize = 9;

/// Maximum length in seconds of a single mediator nomination window (§4). Two years.
///
/// The doctrine is unusually direct here: mediators serve "with term limits that ensure
/// return to the home community. The structural feature that matters is that mediators
/// *must return home*. **A permanent mediator class is the first step toward an
/// inquisitorial caste**" (`liminal_institutions.md` "Discernment training"). Longer
/// than the examiner term, because the same passage wants a pool "populated by
/// apprenticeship" and depth of judgment takes time to acquire; bounded regardless,
/// because the caste is what the bound exists to prevent.
pub const DEFAULT_MEDIATOR_MAX_TERM_SECONDS: i64 = 730 * 24 * 3600;

/// Minimum sit-out in seconds between a mediator term that has ended and a
/// re-nomination to the pool (§4 rotation, mirroring §8 and §2). One year — the
/// "return home" the term limit exists for is not served by a nominal gap.
pub const DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS: i64 = 365 * 24 * 3600;

/// §4 mediator-pool caps: the capture-resistance limits on who may mediate. The same
/// four rules as [`ObserverCaps`] and [`ExaminerCaps`], enforced by the same
/// [`check_roster_caps`], with §4's own policy numbers.
///
/// The mediator pool was the **third** roster and the one wired to nothing: until this
/// existed a community could hold nine of ten seats and staff every panel in the
/// compact, and every mediator could be seated with no end date at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediatorCaps {
    /// Active-pool size at or above which the no-majority pool-share rule bites.
    pub pool_floor: usize,
    /// Maximum length (seconds) of a single mediator nomination window.
    pub max_term_seconds: i64,
    /// Minimum sit-out (seconds) between a prior term's end and a re-nomination.
    pub rotation_period_seconds: i64,
}

impl Default for MediatorCaps {
    fn default() -> Self {
        Self {
            pool_floor: DEFAULT_MEDIATOR_POOL_FLOOR,
            max_term_seconds: DEFAULT_MEDIATOR_MAX_TERM_SECONDS,
            rotation_period_seconds: DEFAULT_MEDIATOR_ROTATION_PERIOD_SECONDS,
        }
    }
}

impl MediatorCaps {
    /// The three roster limits, for [`check_roster_caps`].
    pub fn limits(&self) -> RosterLimits {
        RosterLimits {
            pool_floor: self.pool_floor,
            max_term_seconds: self.max_term_seconds,
            rotation_period_seconds: self.rotation_period_seconds,
        }
    }
}

/// §2 examining-pool caps: the capture-resistance limits on who may conduct
/// anti-spiral-feature credentialing. Structurally the same as [`ObserverCaps`] and
/// enforced by the same rules ([`check_roster_caps`]), but a distinct type — the two
/// are separately tunable policies, and a caller that swapped them should not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExaminerCaps {
    /// Active-pool size at or above which the no-majority pool-share rule bites.
    pub pool_floor: usize,
    /// Maximum length (seconds) of a single examining term.
    pub max_term_seconds: i64,
    /// Minimum sit-out (seconds) between a prior term's end and a re-nomination.
    pub rotation_period_seconds: i64,
    /// Distinct seated examiners an admission into a standing compact must carry.
    pub panel_min: usize,
}

impl Default for ExaminerCaps {
    fn default() -> Self {
        Self {
            pool_floor: DEFAULT_EXAMINER_POOL_FLOOR,
            max_term_seconds: DEFAULT_EXAMINER_MAX_TERM_SECONDS,
            rotation_period_seconds: DEFAULT_EXAMINER_ROTATION_PERIOD_SECONDS,
            panel_min: DEFAULT_EXAMINER_PANEL_MIN,
        }
    }
}

impl ExaminerCaps {
    /// The three roster limits, for [`check_roster_caps`].
    pub fn limits(&self) -> RosterLimits {
        RosterLimits {
            pool_floor: self.pool_floor,
            max_term_seconds: self.max_term_seconds,
            rotation_period_seconds: self.rotation_period_seconds,
        }
    }
}

/// The number of failed-then-re-violated restoration cycles that a *normal*
/// anathema designation must cite (bounded_conflict.md condition 1).
const DESIGNATION_CYCLE_FLOOR: usize = 3;

/// Is `community` inside a probation window at instant `at`?
///
/// The §2 probation predicate, and the *single* representation of the window
/// arithmetic. Two windows count, per `capture_resistance.md` "Probation as the
/// load-bearing record" and `exit_and_succession.md` §"Re-entry after expulsion":
///
/// - a received sponsorship whose stake still rides on this community's conduct
///   (`SponsorshipRecord::in_probation`), and
/// - a completed re-entry before its `probation_until` — a re-entered actor does
///   not return to clean-slate status.
///
/// `kc-reputation::derive_standing` calls this rather than re-deriving it, so the
/// standing a dashboard reports and the window an authority rule enforces cannot
/// drift apart. Precedence (an active designation overriding, probation applying
/// only to an admitted community) stays in reputation, where standing lives; only
/// the window is here, where the authority pass can reach it.
pub fn probationary_at(archive: &InMemoryArchive, community: &DidKey, at: Timestamp) -> bool {
    probationary_at_excluding(archive, community, at, None)
}

/// As [`probationary_at`], ignoring one record: the one currently being authorized.
///
/// A record may not be de-weighted by a probation window that it itself opens. A
/// re-entry's `completed_at` is the exact instant its own post-re-entry window
/// begins, so without this exclusion the petitioner would be dropped from the
/// quorum of the very act it is party to — the record would refuse itself. Mirrors
/// the `self_cid` skip in [`authorize_observer_registry`].
///
/// A sponsorship opens the *entrant's* window while speaking for the *sponsor*, so
/// it is only self-referential when a community vouches for itself, which
/// [`authorize_sponsorship`] refuses on its own grounds.
fn probationary_at_excluding(
    archive: &InMemoryArchive,
    community: &DidKey,
    at: Timestamp,
    exclude: Option<&Cid>,
) -> bool {
    for (cid, entry) in archive.iter() {
        if exclude == Some(cid) {
            continue;
        }
        match entry {
            ArchiveEntry::SponsorshipVouched(s) => {
                if &s.payload.entrant_community == community && s.payload.in_probation(at) {
                    return true;
                }
            }
            ArchiveEntry::ReEntryCompleted(s) => {
                let r = &s.payload;
                if &r.petitioner == community
                    && r.completed_at <= at
                    && at < r.probation_until
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// How many of `dids` carry deliberative weight at `at` — distinct communities, minus
/// those inside a probation window (§2) and those held out of the `Deliberation`
/// office by an in-force injunction (§9).
///
/// The *one* representation of "counting weight rather than signatures", so a
/// component-level bar and the generic quorum cannot disagree about who counts.
/// `exclude` skips one record in the window scan (see
/// [`probationary_at_excluding`]); `barred` drops a community for a reason that is
/// not probation — the subject of its own readmission.
///
/// It exists because the count and the *floor* are two different things and only
/// this one is about weight: a floor that counted signatures while the grade
/// counted weight would let the higher bar be met with weight the doctrine
/// withholds, which is what measurement found in both contraction bars.
fn deliberative_weight<'a>(
    archive: &InMemoryArchive,
    dids: impl IntoIterator<Item = &'a DidKey>,
    at: Timestamp,
    exclude: Option<&Cid>,
    barred: Option<&DidKey>,
) -> ProbationTally {
    let mut tally = ProbationTally::default();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for did in dids {
        if !seen.insert(did.as_str()) {
            continue;
        }
        tally.distinct += 1;
        if Some(did) == barred {
            tally.barred = true;
            continue;
        }
        if probationary_at_excluding(archive, did, at, exclude) {
            tally.probationary += 1;
            continue;
        }
        // The office of deliberation, withheld by an in-force injunction. It belongs
        // here rather than at each bar for the reason the probation filter does: the
        // count and the floor must read the same rule, or the bar that counts
        // signatures becomes the cheaper of the two.
        if capability_revoked(archive, did, Capability::Deliberation, at) {
            tally.without_office += 1;
            continue;
        }
        tally.counted += 1;
    }
    tally
}

/// The result of a [`deliberative_weight`] count: what was presented, what carried
/// weight, and why the difference — an operator has to be able to tell a shortfall
/// of signatures from a shortfall caused by probation.
#[derive(Default)]
struct ProbationTally {
    /// Distinct communities presented.
    distinct: usize,
    /// Distinct communities carrying deliberative weight.
    counted: usize,
    /// How many were dropped for being inside a probation window.
    probationary: usize,
    /// How many were dropped for being held out of the deliberation office by an
    /// in-force injunction (§9).
    without_office: usize,
    /// Whether the `barred` community was among them.
    barred: bool,
}

impl ProbationTally {
    /// The §2 clause naming why a count fell short, or `None` when nothing was
    /// dropped and the shortfall is an ordinary want of signatures.
    fn because(&self) -> Option<String> {
        let mut why: Vec<String> = Vec::new();
        if self.probationary > 0 {
            why.push(format!(
                "{} of {} bound communities are probationary",
                self.probationary, self.distinct
            ));
        }
        if self.without_office > 0 {
            why.push(format!(
                "{} of {} bound communities are held out of the Deliberation office by \
                 an in-force injunction",
                self.without_office, self.distinct
            ));
        }
        if self.barred {
            why.push("the subject does not count toward its own readmission".into());
        }
        (!why.is_empty()).then(|| format!(" ({}; §2)", why.join(", ")))
    }
}

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
        // A community's account of a window in a function it holds is an ordinary
        // act of that community, delegable exactly as a conduct record is.
        ArchiveEntry::ActivityAttested(_) => &[Envoy],
        ArchiveEntry::DisputeFiled(_) => &[Envoy],
        // A party's own case, delegable exactly as the dispute filing is. A panelist's
        // argument never reaches here: `record_authority` returns `None` for it.
        ArchiveEntry::ArgumentFiled(_) => &[Envoy],
        // A party asking for a recusal, consenting to the panel it faces, and a
        // community accepting or being accepted as a protecting power, are all acts a
        // community's envoy may sign for it.
        ArchiveEntry::RecusalRequested(_)
        | ArchiveEntry::PanelConsentRecorded(_)
        | ArchiveEntry::ProtectingPowerDesignated(_) => &[Envoy],
        // A recusal decision is signed by mediators in person, never by a delegate.
        ArchiveEntry::RecusalDecided(_) => &[],
        // §7 commons acts an envoy may sign on the community's behalf.
        ArchiveEntry::CommonsRegistered(_)
        | ArchiveEntry::UsageAccounted(_)
        | ArchiveEntry::SanctionApplied(_)
        | ArchiveEntry::CommonsEscalated(_) => &[Envoy],
        // Opening a sealed usage record is the custodians' act, authorized by the
        // custody it opens rather than by principal binding; this set is unused.
        ArchiveEntry::UsageOpened(_) => &[],
        // §9 constitutional acts a community may co-sign via an envoy.
        ArchiveEntry::ConstitutionRatified(_)
        | ArchiveEntry::CompactIncapableAmended(_)
        | ArchiveEntry::FastCadenceAmended(_)
        | ArchiveEntry::MinimumContentAdopted(_)
        | ArchiveEntry::MinimumContentAmended(_) => &[Envoy],
        // Move 1: a community's reading of its own substrate, and a third party's
        // attestation of an interpreter's independence; an envoy may sign either.
        ArchiveEntry::SubstrateInterpreted(_)
        | ArchiveEntry::InterpreterIndependenceAttested(_) => &[Envoy],
        // Move 4: an event report / corroboration / action a community authors; an
        // envoy may sign on its behalf.
        ArchiveEntry::EventReported(_)
        | ArchiveEntry::EventCorroborated(_)
        | ArchiveEntry::EventActionInvoked(_)
        | ArchiveEntry::RetroactiveInterestFound(_) => &[Envoy],
        // Move 4, the pool's own side: an invocation and a review are signed by
        // mediators in person, never by a community's delegate, exactly as a recusal
        // decision is. Both are authorized by a panel rule, so these sets are unused.
        ArchiveEntry::SkepticismInvoked(_) | ArchiveEntry::SkepticismReviewed(_) => &[],
        // Move 6: a principal witness attestation / output a community authors;
        // an envoy may sign on its behalf.
        ArchiveEntry::PrincipalWitnessed(_) | ArchiveEntry::PrincipalOutputRecorded(_) => &[Envoy],
        // §8. A subject community's response to an observer report; an envoy may
        // sign it. (The other §8 records are authorized by observer/panel rules.)
        ArchiveEntry::ReportResponsePublished(_) => &[Envoy],
        // §2. An examining roster is the nominating community's own act: root only,
        // like the §8 observer registry it mirrors. A member roll joins them: naming
        // who belongs to a community is not a thing a delegated speaker may do.
        ArchiveEntry::ExaminerRegistryUpdated(_)
        | ArchiveEntry::MemberRollPublished(_)
        | ArchiveEntry::ObserverRegistryUpdated(_)
        | ArchiveEntry::ObserverReportPublished(_)
        | ArchiveEntry::WhistleblowerFiled(_)
        | ArchiveEntry::SourceRevealed(_)
        | ArchiveEntry::SourceExculpated(_) => &[],
        // §5 escalation acts an envoy may sign on the community's behalf.
        ArchiveEntry::LadderAdvanced(_)
        | ArchiveEntry::DesignationDecided(_)
        | ArchiveEntry::ReexaminationConcluded(_)
        | ArchiveEntry::RestitutionObligated(_)
        | ArchiveEntry::ReEntryCompleted(_)
        | ArchiveEntry::SubstantiveReEngagementInvoked(_)
        | ArchiveEntry::InjunctionConfirmed(_) => &[Envoy],
        // A charter is signed by the compact's own key and its members' roots in
        // person: constituting a body is not a thing a delegate does.
        ArchiveEntry::CompactChartered(_) => &[],
        ArchiveEntry::IntraCommunityRite(_)
        | ArchiveEntry::SpeakerDelegated(_)
        | ArchiveEntry::WitnessRegistryUpdated(_)
        | ArchiveEntry::MediatorRegistryUpdated(_)
        | ArchiveEntry::CommunityIdentityPublished(_)
        // These are authorized by a panel rule, not principal binding; their
        // allowed-scope sets are unused.
        | ArchiveEntry::MediationDecided(_)
        | ArchiveEntry::DecisionUnsealed(_)
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
        // An attestation speaks for the community that holds the seats, and is
        // first-class: accounting for one's own window is not a compact-level decision.
        ArchiveEntry::ActivityAttested(s) => Some(ra(
            vec![s.payload.attesting_community.clone()],
            s.payload.attested_at,
            false,
        )),
        ArchiveEntry::MediatorRegistryUpdated(s) => {
            Some(ra(vec![s.payload.nominating_community.clone()], s.payload.issued_at, false))
        }
        ArchiveEntry::DisputeFiled(s) => {
            Some(ra(vec![s.payload.complainant.clone()], s.payload.filed_at, false))
        }
        // MediationDecided is authorized by the panel rule (see authorize_decision),
        // and ArgumentFiled by its own role rule (see authorize_argument), which does the
        // party-side principal binding itself because only one of the two roles has a
        // community principal at all.
        ArchiveEntry::MediationDecided(_) | ArchiveEntry::ArgumentFiled(_) => None,
        // Opening a seal is the custodians' act, checked by its own rule below. A
        // usage opening is the same shape one component over.
        ArchiveEntry::DecisionUnsealed(_) | ArchiveEntry::UsageOpened(_) => None,
        // §4.2. A recusal request and a protecting-power designation are the parties'
        // and protectors' own acts; a recusal *decision* is answered by mediators, who
        // are people in the pool rather than communities, so it carries no principal at
        // all. All three are checked by their own rules below.
        // A panel consent joins them: it is the party's own act, self-signed, and its
        // rule does the party-side binding itself (see `authorize_panel_consent`).
        ArchiveEntry::RecusalRequested(_)
        | ArchiveEntry::RecusalDecided(_)
        | ArchiveEntry::PanelConsentRecorded(_)
        | ArchiveEntry::ProtectingPowerDesignated(_) => None,
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
        // Re-examining a designation is the same weight of act as making one, so it
        // carries the same second-class distinct-community floor. Hard-coded rather
        // than read off the record, the fail-safe direction (see §2's note on
        // `ReEntry.decision_class`).
        ArchiveEntry::ReexaminationConcluded(s) => {
            Some(ra(s.payload.examiners.clone(), s.payload.concluded_at, true))
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
        // A charter is constituted by its members: second-class, so the
        // distinct-community quorum arrives generically. The compact's own signature is
        // required by the structural rule rather than counted here, since the compact is
        // not one of the communities constituting it.
        ArchiveEntry::CompactChartered(s) => {
            Some(ra(s.payload.chartered_by.clone(), s.payload.chartered_at, true))
        }
        // The commons escalation finding is always second-class: it is never one
        // community's to make about another, so the distinct-community quorum comes for
        // free with the flag.
        ArchiveEntry::CommonsEscalated(s) => {
            Some(ra(s.payload.movers.clone(), s.payload.escalated_at, true))
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
        // §2. An examining roster is the nominating community speaking for itself:
        // a first-class principal-bound act, capped by `ExaminerCaps`.
        ArchiveEntry::ExaminerRegistryUpdated(s) => {
            Some(ra(vec![s.payload.nominating_community.clone()], s.payload.issued_at, false))
        }
        // §8. A member roll is the community stating its own composition.
        ArchiveEntry::MemberRollPublished(s) => {
            Some(ra(vec![s.payload.community_did.clone()], s.payload.published_at, false))
        }
        // Observer reports and whistleblower records are authorized by the
        // observer / reception-panel rules, not principal binding.
        ArchiveEntry::ObserverReportPublished(_)
        | ArchiveEntry::WhistleblowerFiled(_)
        | ArchiveEntry::SourceRevealed(_)
        | ArchiveEntry::SourceExculpated(_) => None,
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
        ArchiveEntry::MinimumContentAdopted(s) => {
            Some(ra(s.payload.adopters.clone(), s.payload.adopted_at, true))
        }
        ArchiveEntry::MinimumContentAmended(s) => {
            Some(ra(s.payload.deciders.clone(), s.payload.decided_at, true))
        }
        // Move 1. A substrate interpretation is the community speaking for
        // itself: first-class, principal-bound. Interpretive plurality is a
        // detector signal, not an authority refusal.
        ArchiveEntry::SubstrateInterpreted(s) => {
            Some(ra(vec![s.payload.community_did.clone()], s.payload.recorded_at, false))
        }
        // An independence attestation is the attesting party speaking for itself;
        // whether it counts toward plurality (attester != community, != interpreter)
        // is a detector filter in the lens, not an authority refusal.
        ArchiveEntry::InterpreterIndependenceAttested(s) => {
            Some(ra(vec![s.payload.attesting_party.clone()], s.payload.attested_at, false))
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
        // An event action is the acting community speaking for itself; whether the
        // structured-skepticism hold permits it is a structural rule below.
        ArchiveEntry::EventActionInvoked(s) => {
            Some(ra(vec![s.payload.acting_community.clone()], s.payload.invoked_at, false))
        }
        // A retroactive interest finding is the finder speaking for itself; whether
        // it impugns the event (finder != originator) is a detector filter, not an
        // authority refusal.
        ArchiveEntry::RetroactiveInterestFound(s) => {
            Some(ra(vec![s.payload.finder.clone()], s.payload.found_at, false))
        }
        // An invocation and a review are the *pool's* acts, not any community's, and
        // are authorized by their own panel rules (see `authorize_skepticism_*`).
        ArchiveEntry::SkepticismInvoked(_) | ArchiveEntry::SkepticismReviewed(_) => None,
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
        ArchiveEntry::ActivityAttested(s) => dids(s),
        ArchiveEntry::MediatorRegistryUpdated(s) => dids(s),
        ArchiveEntry::DisputeFiled(s) => dids(s),
        ArchiveEntry::ArgumentFiled(s) => dids(s),
        ArchiveEntry::RecusalRequested(s) => dids(s),
        ArchiveEntry::RecusalDecided(s) => dids(s),
        ArchiveEntry::ProtectingPowerDesignated(s) => dids(s),
        ArchiveEntry::PanelConsentRecorded(s) => dids(s),
        ArchiveEntry::MediationDecided(s) => dids(s),
        ArchiveEntry::DecisionUnsealed(s) => dids(s),
        ArchiveEntry::LadderAdvanced(s) => dids(s),
        ArchiveEntry::DesignationDecided(s) => dids(s),
        ArchiveEntry::ReexaminationConcluded(s) => dids(s),
        ArchiveEntry::RestitutionObligated(s) => dids(s),
        ArchiveEntry::RestitutionVerified(s) => dids(s),
        ArchiveEntry::ReEntryCompleted(s) => dids(s),
        ArchiveEntry::SubstantiveReEngagementInvoked(s) => dids(s),
        ArchiveEntry::InjunctionIssued(s) => dids(s),
        ArchiveEntry::InjunctionConfirmed(s) => dids(s),
        ArchiveEntry::InjunctionChallenged(s) => dids(s),
        ArchiveEntry::CommonsRegistered(s) => dids(s),
        ArchiveEntry::UsageAccounted(s) => dids(s),
        ArchiveEntry::UsageOpened(s) => dids(s),
        ArchiveEntry::CommonsEscalated(s) => dids(s),
        ArchiveEntry::CompactChartered(s) => dids(s),
        ArchiveEntry::SanctionApplied(s) => dids(s),
        ArchiveEntry::ExaminerRegistryUpdated(s) => dids(s),
        ArchiveEntry::MemberRollPublished(s) => dids(s),
        ArchiveEntry::ObserverRegistryUpdated(s) => dids(s),
        ArchiveEntry::ObserverReportPublished(s) => dids(s),
        ArchiveEntry::ReportResponsePublished(s) => dids(s),
        ArchiveEntry::WhistleblowerFiled(s) => dids(s),
        ArchiveEntry::SourceRevealed(s) => dids(s),
        ArchiveEntry::SourceExculpated(s) => dids(s),
        ArchiveEntry::ConstitutionRatified(s) => dids(s),
        ArchiveEntry::CompactIncapableAmended(s) => dids(s),
        ArchiveEntry::FastCadenceAmended(s) => dids(s),
        ArchiveEntry::MinimumContentAdopted(s) => dids(s),
        ArchiveEntry::MinimumContentAmended(s) => dids(s),
        ArchiveEntry::SubstrateInterpreted(s) => dids(s),
        ArchiveEntry::InterpreterIndependenceAttested(s) => dids(s),
        ArchiveEntry::EventReported(s) => dids(s),
        ArchiveEntry::EventCorroborated(s) => dids(s),
        ArchiveEntry::SkepticismInvoked(s) => dids(s),
        ArchiveEntry::SkepticismReviewed(s) => dids(s),
        ArchiveEntry::EventActionInvoked(s) => dids(s),
        ArchiveEntry::RetroactiveInterestFound(s) => dids(s),
        ArchiveEntry::PrincipalWitnessed(s) => dids(s),
        ArchiveEntry::PrincipalOutputRecorded(s) => dids(s),
    }
}

/// The nominating community of `mediator_did` if it is an active registered
/// mediator at `at`, else `None`.
pub(crate) fn mediator_home(
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

/// §2 sponsor rule: a community inside its own probation window may not vouch for an
/// entrant.
///
/// This is the withholding with teeth against capture move 7. Without it the
/// admissions chain is self-feeding — a community admitted this second may vouch for
/// the next, so a faction of one grows by one entrant per ceremony and every
/// admission is procedurally clean (`capture_resistance.md` §7, "the vector is slower
/// than any other in this catalogue and harder to see").
///
/// Refusal rather than de-weighting, unlike the quorum domains: a vouch *is* an
/// exercise of deliberative weight in admissions and nothing counts vouches, so a
/// vouch that carried no weight would be an inert record. What a sponsor stakes is
/// standing it has not yet earned.
fn authorize_sponsorship(
    archive: &InMemoryArchive,
    s: &SponsorshipRecord,
) -> KcResult<()> {
    // A vouch is another community's testimony about your operative practice; there
    // is no such thing as vouching for yourself, and the sponsor-stake mechanism
    // (`capture_resistance.md` §7 "Sponsor stake") is empty if the stake and the
    // conduct it rides on belong to the same community.
    if s.sponsor_community == s.entrant_community {
        return Err(KcError::Authority(format!(
            "community {} vouched for itself; a sponsorship stakes another \
             community's standing on the entrant's conduct (§2)",
            s.sponsor_community.as_str()
        )));
    }
    if probationary_at(archive, &s.sponsor_community, s.sponsored_at) {
        return Err(KcError::Authority(format!(
            "sponsor {} is itself probationary at {}; a probationary community carries \
             no deliberative weight in admissions (§2)",
            s.sponsor_community.as_str(),
            s.sponsored_at
        )));
    }
    Ok(())
}

/// §2 admission rule: an entrant joining a standing compact must be admitted by a
/// quorum of communities that are *already in that compact* and carrying deliberative
/// weight.
///
/// `capture_resistance.md` §7: "Admission and readmission validate only with
/// multi-community quorum signatures … No AU-region's faction, and no single sponsor
/// cluster, can admit alone." Before this rule an admission carried no structural
/// requirement at all beyond two participants, and because the entrant is itself a
/// participant and co-signs, **one sitting member plus the entrant sealed an
/// admission** — measured, and kept as
/// `tests::admissions_integrity::measure_faction_of_one_seats_entrants_unopposed`.
///
/// Three things make the rule hold rather than merely read well:
///
/// - **The admitted set is derived, never declared.** A participant counts as sitting
///   only because an earlier record in this archive says so. A field on the record
///   asserting who was already a member would be unenforceable — a lesson from
///   ethne D21/D22, where a self-declared founding boundary could be edited to
///   promote its own grade. The evidence is the source.
/// - **Insertion order is authoritative, not `witnessed_at`.** Priors are the records
///   that precede this one in the append-only log, so authorizing an earlier
///   admission never "sees" a later one and no admission can be retroactively
///   legitimized by one appended afterward. Same choice, for the same reason, as
///   [`current_rung`].
/// - **Compact formation is exempt.** The first record for a `compact_label` has no
///   sitting members to draw a quorum from; its participants are founders, not
///   entrants. All 10 admission records that existed in this tree before the rule
///   were formations, so the exemption is also what keeps it from being a rewrite of
///   the corpus.
///
/// The quorum is stricter than the generic second-class one in two ways: it counts
/// only participants already in the compact (so the entrant cannot supply its own
/// admitting signature) and it drops probationary ones (§2's first withholding —
/// which is what stops each new entrant from immediately amplifying the faction that
/// admitted it).
///
/// *Honest limit:* `compact_label` is a self-declared string, so a mislabelled record
/// reads as the formation of some other compact. It cannot forge this compact's roll
/// and a consumer reading by label never sees it — accountability, not prevention.
fn authorize_inter_witness(
    archive: &InMemoryArchive,
    rec: &InterCommunityWitnessRecord,
    signers: &[&str],
    self_cid: &Cid,
    quorum_threshold: usize,
    caps: ExaminerCaps,
) -> KcResult<()> {
    let mut sitting: BTreeSet<&str> = BTreeSet::new();
    for (cid, e) in archive.iter() {
        if cid == self_cid {
            break; // insertion order: never look past the record being authorized
        }
        if let ArchiveEntry::InterCommunityWitness(s) = e {
            if s.payload.compact_label != rec.compact_label {
                continue;
            }
            for p in &s.payload.participants {
                sitting.insert(p.as_str());
            }
        }
    }
    if sitting.is_empty() {
        return Ok(()); // compact formation: these are founders
    }

    // Sponsor stake, with teeth. Every entrant must arrive vouched for by a sitting
    // member that carries weight.
    //
    // This is what makes the probation withholding load-bearing rather than opt-in.
    // A probation window only opens when somebody stakes standing on an entrant, so
    // without this rule a faction could admit entrant after entrant while never
    // vouching for any of them, and there would be no probationary community to
    // de-weight — measured, and kept as
    // `tests::admissions_integrity::measure_admission_without_a_sponsor_is_refused`
    // (5 of 5 unsponsored admissions were authorized). `capture_resistance.md` §7:
    // "The vouching structure already specified in the inter-community layer is given
    // teeth: a sponsor's standing rides on the entrant's conduct through the
    // probationary period."
    //
    // The vouch must already be in the log (insertion order again), which is also the
    // order `recognition_protocols.md` describes: the sponsor speaks for the candidate
    // before the compact seals the admission.
    for p in &rec.participants {
        if sitting.contains(p.as_str()) {
            continue; // a sitting member is not an entrant and needs no sponsor
        }
        let mut sponsored = false;
        for (cid, e) in archive.iter() {
            if cid == self_cid {
                break;
            }
            if let ArchiveEntry::SponsorshipVouched(s) = e {
                let v = &s.payload;
                if &v.entrant_community == p
                    && sitting.contains(v.sponsor_community.as_str())
                    && !probationary_at_excluding(
                        archive,
                        &v.sponsor_community,
                        v.sponsored_at,
                        Some(self_cid),
                    )
                {
                    sponsored = true;
                    break;
                }
            }
        }
        if !sponsored {
            return Err(KcError::Authority(format!(
                "entrant {} is admitted to '{}' with no sponsor among the sitting \
                 communities carrying weight; a sponsor's standing must ride on the \
                 entrant's conduct (§2 sponsor stake)",
                p.as_str(),
                rec.compact_label
            )));
        }
    }

    let mut admitting: BTreeSet<&str> = BTreeSet::new();
    let mut probationary = 0usize;
    for p in &rec.participants {
        if !sitting.contains(p.as_str()) {
            continue; // an entrant does not admit itself
        }
        if probationary_at_excluding(archive, p, rec.witnessed_at, Some(self_cid)) {
            probationary += 1;
            continue;
        }
        admitting.insert(p.as_str());
    }
    if admitting.len() < quorum_threshold {
        let because = if probationary > 0 {
            format!(
                ", {probationary} sitting participant(s) probationary and so carrying \
                 no weight in admissions"
            )
        } else {
            String::new()
        };
        return Err(KcError::Authority(format!(
            "admission into '{}' has an admitting quorum of {} sitting communities, \
             need >= {} ({} participants, {} sitting{}; §2)",
            rec.compact_label,
            admitting.len(),
            quorum_threshold,
            rec.participants.len(),
            sitting.len(),
            because
        )));
    }

    // The examining panel: the signers who hold an active examining seat, seated by a
    // community that is itself sitting and carrying weight. Not *every* signer must be
    // an examiner — unlike a mediation decision, this record is jointly the entrant's
    // act and the compact's — but enough of them must be, from enough communities.
    let mut panel: BTreeSet<&str> = BTreeSet::new();
    let mut panel_homes: BTreeMap<String, usize> = BTreeMap::new();
    for signer in signers {
        let Some(home) = examiner_home(archive, signer, rec.witnessed_at) else {
            continue;
        };
        if !sitting.contains(home.as_str()) {
            continue; // an entrant's own examiner does not examine the entrant
        }
        if probationary_at_excluding(archive, &home, rec.witnessed_at, Some(self_cid)) {
            continue;
        }
        if panel.insert(*signer) {
            *panel_homes.entry(home.as_str().to_string()).or_insert(0) += 1;
        }
    }
    if panel.len() < caps.panel_min {
        return Err(KcError::Authority(format!(
            "admission into '{}' was examined by {} seated examiner(s), need >= {} \
             (§2 credentialing panel)",
            rec.compact_label,
            panel.len(),
            caps.panel_min
        )));
    }
    // Cross-community composition. Two distinct seating communities always; and once
    // the compact is large enough for the pool-share rule to bite at all, no single
    // community may supply a majority of the panel either — a panel one community
    // dominates is the standing admissions cadre wearing a rotation schedule.
    if panel_homes.len() < 2 {
        return Err(KcError::Authority(format!(
            "admission into '{}' was examined by a panel from a single community, {} \
             (§2 cross-community composition)",
            rec.compact_label,
            panel_homes.keys().next().map(String::as_str).unwrap_or("none")
        )));
    }
    let pool_total: usize =
        active_roster_shares(&rosters_of(archive, RosterKind::Examiner), rec.witnessed_at)
            .values()
            .sum();
    if pool_total >= caps.pool_floor {
        if let Some((community, held)) = panel_homes.iter().max_by_key(|(_, n)| **n) {
            if 2 * held > panel.len() {
                return Err(KcError::Authority(format!(
                    "community {} supplied {}/{} of the panel examining an admission \
                     into '{}', a majority (§2 cross-community composition)",
                    community,
                    held,
                    panel.len(),
                    rec.compact_label
                )));
            }
        }
    }
    Ok(())
}

/// §2 mediator-selection rule: a probationary community may not nominate mediators.
///
/// The second of the doctrine's three withholdings. Seating a mediator is the
/// selection act, and `capture_resistance.md` §7 withholds weight in "mediator
/// selection" for the same reason it withholds it in admissions — otherwise each
/// admitted entrant immediately amplifies the faction that admitted it, here by
/// staffing the pool that decides its disputes.
fn authorize_mediator_registry(
    archive: &InMemoryArchive,
    reg: &kc_mediation::MediatorRegistry,
    self_cid: &Cid,
    caps: MediatorCaps,
) -> KcResult<()> {
    require_office(
        archive,
        &reg.nominating_community,
        Capability::MediatorSeat,
        reg.issued_at,
        "seating mediators",
    )?;
    if probationary_at(archive, &reg.nominating_community, reg.issued_at) {
        return Err(KcError::Authority(format!(
            "community {} is probationary at {}; a probationary community carries no \
             deliberative weight in mediator selection (§2)",
            reg.nominating_community.as_str(),
            reg.issued_at
        )));
    }
    // The encryption key must not be the signing key. §8 custody seals a share of a
    // whistleblower's identity to this key by key agreement; if it is the same key the
    // mediator signs with, the separation the field exists to provide is gone and the
    // registration says otherwise on its face.
    for e in &reg.entries {
        if e.encryption_pubkey.bytes == e.mediator_pubkey.bytes {
            return Err(KcError::Authority(format!(
                "mediator {} is registered with its signing key as its encryption key: \
                 custody sealing would then reuse one key for two primitives",
                e.mediator_did
            )));
        }
    }
    // The four capture-resistance caps §8's observers and §2's examiners already carry.
    // The mediator pool was the third roster and the one wired to nothing, which is how
    // a community could hold nine of ten seats and staff every panel in the compact,
    // and how every mediator could be seated with no end date — "a permanent mediator
    // class is the first step toward an inquisitorial caste"
    // (`liminal_institutions.md`).
    let seats: Vec<Seat> = reg
        .entries
        .iter()
        .map(|en| Seat {
            did: en.mediator_did.as_str(),
            from: en.valid_from,
            until: en.valid_until,
        })
        .collect();
    check_roster_caps(
        archive,
        RosterKind::Mediator,
        &reg.nominating_community,
        reg.issued_at,
        &seats,
        self_cid,
        caps.limits(),
    )
}

/// The encryption key each active registered mediator's §8 custody shares must be
/// sealed to, at `at`. Later registrations from the same community supersede earlier
/// ones, so this follows the same resolution the panel rules use.
///
/// Public because a filer sealing custody and the authority pass checking it must use
/// one definition — the same reason `insider_keys` is public.
pub fn mediator_encryption_keys(
    archive: &InMemoryArchive,
    at: Timestamp,
) -> BTreeMap<String, kc_core::PubKey> {
    // One definition of "the latest roster per community", shared with the archive's own
    // key resolution (`store::latest_rosters`) — this function and `resolve_did` used to
    // carry the rule separately, and only this one had it.
    let latest = crate::store::latest_rosters(
        archive.iter().filter_map(|(_, e)| match e {
            ArchiveEntry::MediatorRegistryUpdated(s) => {
                Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
            }
            _ => None,
        }),
        at,
    );
    let mut out = BTreeMap::new();
    for reg in latest {
        for entry in &reg.entries {
            if entry.active_at(at) {
                out.insert(entry.mediator_did.as_str().to_string(), entry.encryption_pubkey.clone());
            }
        }
    }
    out
}

/// A [`mediator_encryption_keys`] lookup shaped for [`kc_core::ThresholdCustody::seal`].
pub fn mediator_encryption_resolver(
    archive: &InMemoryArchive,
    at: Timestamp,
) -> impl Fn(&DidKey) -> KcResult<kc_core::PubKey> {
    let keys = mediator_encryption_keys(archive, at);
    move |did: &DidKey| {
        keys.get(did.as_str()).cloned().ok_or_else(|| {
            KcError::Key(format!(
                "{did} is not an active registered mediator, so no encryption key resolves \
                 for it: a share cannot be sealed to a custodian the archive cannot place"
            ))
        })
    }
}

/// How one panel names itself in an authority error. The five panels differ only in
/// their wording, so the wording is the parameter and the rule is not.
struct PanelSpec<'a> {
    /// The role phrase preceding a signer: `"panelist"`, `"injunction panelist"`.
    role: &'a str,
    /// How a barred home community is described: `"a party community"`.
    barred_kind: &'a str,
    /// The panel's name in a shortfall message: `"mediation panel"`.
    label: &'a str,
}

/// The mediator-panel rule, shared by all five panels in the tier: every co-signer
/// must be an active registered mediator, none may come from a `barred` community
/// (non-overlap), the panel must span at least `panel_min` of them — and a panelist
/// whose home community is probationary **does not count** toward that floor (§2).
///
/// One implementation, because the probation filter existed in the mediation panel
/// alone and the other four counted seated panelists from probationary communities
/// at full weight. Copying the filter four times would have left the fifth panel —
/// whichever gets written next — to reintroduce the hole; the §2 slice made the same
/// move for the roster caps (`check_roster_caps`).
///
/// The panelist may still **sit**: what is withheld is weight, not participation. It
/// matters because seating-time refusal is not enough — a re-admitted community's
/// earlier nominations are still live, so its mediators are already seated when its
/// probation window opens.
fn check_mediator_panel(
    archive: &InMemoryArchive,
    signers: &[&str],
    at: Timestamp,
    barred: &BTreeSet<String>,
    panel_min: usize,
    spec: PanelSpec<'_>,
) -> KcResult<()> {
    let mut panel: BTreeSet<&str> = BTreeSet::new();
    let mut probationary_panelists = 0usize;
    for signer in signers {
        let home = match mediator_home(archive, signer, at) {
            None => {
                return Err(KcError::Authority(format!(
                    "{} {signer} is not an active registered mediator",
                    spec.role
                )))
            }
            Some(home) => home,
        };
        if barred.contains(home.as_str()) {
            return Err(KcError::Authority(format!(
                "{} {signer} is from {} (non-overlap violated)",
                spec.role, spec.barred_kind
            )));
        }
        if probationary_at(archive, &home, at) {
            probationary_panelists += 1;
            continue;
        }
        panel.insert(signer);
    }
    if panel.len() < panel_min {
        let because = if probationary_panelists > 0 {
            format!(
                " ({probationary_panelists} panelist(s) from probationary communities \
                 carry no weight; §2)"
            )
        } else {
            String::new()
        };
        return Err(KcError::Authority(format!(
            "{} has {} distinct mediators, need >= {}{}",
            spec.label,
            panel.len(),
            panel_min,
            because
        )));
    }
    Ok(())
}

/// The §4 rule for an argument submitted on a dispute (§4 "structured argument
/// submission"). The two roles answer different questions and are held to different
/// rules.
///
/// A **party's** argument is its own case, so only the complainant or the respondent may
/// make one, and its community's envoy may sign for it exactly as for the dispute filing
/// itself. A **panelist's** argument is the position the decision must answer, so the
/// author must be an active registered mediator from neither party community — the panel
/// rule, one record over — and the signature must be the mediator's own: a mediator is a
/// person in a pool rather than a community with envoys, and nobody argues in a
/// mediator's name.
///
/// Note what is deliberately *not* applied here: §2's probation de-weighting. That rule
/// governs whether a panelist counts toward `panel_min`, which is a question about
/// weight; whether someone may *speak* is a different question, and a probationary
/// community's mediator who may sit on a panel may argue before it. The decision's own
/// `panel_min` still does the counting.
fn authorize_argument(
    archive: &InMemoryArchive,
    arg: &ArgumentRecord,
    signers: &[&str],
) -> KcResult<()> {
    let dispute = match archive.get(&arg.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "argument references {} which is not a filed dispute",
                arg.dispute_cid
            )))
        }
    };
    // An argument is *made*. An empty one was not. The content is never read beyond
    // this: no rule can weigh an argument, only record that it was put.
    if arg.position.trim().is_empty() {
        return Err(KcError::Authority(
            "an argument must state a position".into(),
        ));
    }
    match arg.role {
        ArgumentRole::Party => {
            if arg.author != dispute.complainant && arg.author != dispute.respondent {
                return Err(KcError::Authority(format!(
                    "{} is not a party to this dispute, so may not argue as one",
                    arg.author.as_str()
                )));
            }
            let ok = signers.iter().any(|s| {
                signer_authorized_for(
                    archive,
                    &arg.author,
                    s,
                    arg.submitted_at,
                    &[DelegationScope::Envoy],
                )
            });
            if !ok {
                return Err(KcError::Authority(format!(
                    "party argument for {} has no authorized signer",
                    arg.author.as_str()
                )));
            }
        }
        ArgumentRole::Panelist => {
            if signers != [arg.author.as_str()] {
                return Err(KcError::Authority(
                    "a panelist's argument is signed by exactly that panelist".into(),
                ));
            }
            let home = match mediator_home(archive, arg.author.as_str(), arg.submitted_at) {
                None => {
                    return Err(KcError::Authority(format!(
                        "{} is not an active registered mediator, so may not argue as a \
                         panelist",
                        arg.author.as_str()
                    )))
                }
                Some(home) => home,
            };
            if home == dispute.complainant || home == dispute.respondent {
                return Err(KcError::Authority(format!(
                    "panelist {} is from a party community (non-overlap violated)",
                    arg.author.as_str()
                )));
            }
        }
    }
    Ok(())
}

/// The §4 rule for a recusal request: only a party may ask, and only about a mediator.
///
/// Panel *composition* was already enforced — registered, non-party, minimum count —
/// but a party had no way to say "this one, on this dispute" and have it recorded and
/// answered. Nothing here blocks: an unanswered request is visible rather than
/// paralysing, because a request that stopped a mediation by existing would be the
/// delay tactic §4's own structured-skepticism review guards against.
fn authorize_recusal_request(
    archive: &InMemoryArchive,
    req: &kc_mediation::RecusalRequest,
    signers: &[&str],
) -> KcResult<()> {
    let dispute = match archive.get(&req.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "recusal request references {} which is not a filed dispute",
                req.dispute_cid
            )))
        }
    };
    if req.requested_by != dispute.complainant && req.requested_by != dispute.respondent {
        return Err(KcError::Authority(format!(
            "{} is not a party to this dispute, so may not request a recusal in it",
            req.requested_by.as_str()
        )));
    }
    if req.grounds.trim().is_empty() {
        return Err(KcError::Authority(
            "a recusal request must state its grounds".into(),
        ));
    }
    if mediator_home(archive, req.mediator.as_str(), req.requested_at).is_none() {
        return Err(KcError::Authority(format!(
            "{} is not an active registered mediator, so has no seat to step down from",
            req.mediator.as_str()
        )));
    }
    let ok = signers.iter().any(|sg| {
        signer_authorized_for(
            archive,
            &req.requested_by,
            sg,
            req.requested_at,
            &[DelegationScope::Envoy],
        )
    });
    if !ok {
        return Err(KcError::Authority(format!(
            "recusal request for {} has no authorized signer",
            req.requested_by.as_str()
        )));
    }
    Ok(())
}

/// §4: the custody a sealed decision's proceedings are held under.
///
/// ## Custody is the communities', not the pool's, and it has to be
///
/// The whitepaper asks for "keys distributed across the mediator pool so no single
/// custodian can unseal early" and, in the same breath, for a "deterministic reveal at
/// the **25 to 30 year** mark (one generational cycle)". Measured against the tier as
/// built, those two cannot both be had: `DEFAULT_MEDIATOR_MAX_TERM_SECONDS` caps a
/// mediator's seat at two years, because "a permanent mediator class is the first step
/// toward an inquisitorial caste". A share sealed to a mediator's registered encryption
/// key is therefore held by somebody whose key the archive **stops resolving** long
/// before the seal opens — the signature on the opening cannot even be verified, and the
/// deterministic reveal is dead by construction. The same shape as §9's re-examination
/// cadence, which §8's term limits made unreachable.
///
/// So a generational seal is held by what lasts a generation, and in this compact that is
/// the **member communities**: a `CommunityIdentity` resolves at any instant, with no
/// validity window, which is exactly the property a 25-year custody needs. The doctrine's
/// substance — distributed, no single custodian, not the panel's to open — is kept; what
/// changes is which body holds the keys, because the other reading names a custodian who
/// will not be there.
///
/// The cost is stated rather than hidden: a community has no separate encryption key, so
/// a share is sealed to the key it also signs with — one key under two primitives,
/// mitigated only by domain-separated derivation, which is precisely the compromise
/// `MediatorEntry::encryption_pubkey` was added to avoid for §8. It is accepted here
/// because the alternative is not a better key but no reveal at all.
///
/// ## What is checked
///
/// The sharing is **recoverable in principle** (`check_shape`); every custodian is a
/// published member community and neither party (the two with an interest in opening it
/// early or never); each share is sealed to that community's own root key, so naming a
/// custodian and sealing to them cannot come apart — §8's hard-won lesson, in the setting
/// where the key is the DID itself.
///
/// And **the deciding panel may not hold a quorum of the shares**, counted through the
/// panelists' home communities. A panel holding `k` of its own shares has not sealed its
/// proceedings, it has deferred publishing them at its own discretion, and the parties
/// who spoke candidly to it were given a guarantee that does not exist.
fn authorize_sealed_reasoning(
    archive: &InMemoryArchive,
    decision: &MediationDecision,
    dispute: &kc_mediation::DisputeRecord,
    sealed: &kc_mediation::SealedReasoning,
) -> KcResult<()> {
    sealed
        .custody
        .check_shape()
        .map_err(|e| KcError::Authority(format!("sealed decision custody: {e}")))?;

    let span = sealed.reveal_at.as_second() - decision.decided_at.as_second();
    if !(MIN_DECISION_SEAL_SECONDS..=MAX_DECISION_SEAL_SECONDS).contains(&span) {
        return Err(KcError::Authority(format!(
            "a sealed decision opens {span}s after it was decided, outside the \
             generational cycle [{MIN_DECISION_SEAL_SECONDS}s, {MAX_DECISION_SEAL_SECONDS}s] \
             §4 seals for"
        )));
    }

    let parties: BTreeSet<&str> = [dispute.complainant.as_str(), dispute.respondent.as_str()]
        .into_iter()
        .collect();
    for share in &sealed.custody.sealed_shares {
        let holder = &share.holder_did;
        if parties.contains(holder.as_str()) {
            return Err(KcError::Authority(format!(
                "{holder} is a party to this dispute, so may not hold a share of the seal \
                 over its own proceedings"
            )));
        }
        let root = community_root_pubkey(archive, holder).ok_or_else(|| {
            KcError::Authority(format!(
                "the sealed-decision share for {holder} names a custodian that is not a \
                 published member community, so no key of theirs will resolve when the \
                 seal opens"
            ))
        })?;
        if root.bytes != share.recipient_pubkey.0 {
            return Err(KcError::Authority(format!(
                "the sealed-decision share for {holder} is sealed to a key that is not \
                 that community's own, so the custodian cannot open it"
            )));
        }
    }

    // The panel's own homes, which is what "the panel can open this" means once the
    // custodians are communities.
    let mut panel_homes: BTreeSet<String> = BTreeSet::new();
    for m in &decision.panel {
        if let Some(home) = mediator_home(archive, m.as_str(), decision.decided_at) {
            panel_homes.insert(home.as_str().to_string());
        }
    }
    let k = usize::from(sealed.custody.threshold_k);
    let on_panel = sealed
        .custody
        .holders()
        .iter()
        .filter(|h| panel_homes.contains(h.as_str()))
        .count();
    if on_panel >= k {
        return Err(KcError::Authority(format!(
            "{on_panel} of the {k} shares needed to open this seal are held by the \
             deciding panel's own communities, so the panel can unseal its own \
             proceedings: that is a deferral of publication, not a seal"
        )));
    }
    Ok(())
}

/// A community's root public key, if it has published an identity. Communities resolve
/// at any instant — no validity window — which is the property a generational seal needs
/// of whoever holds its shares.
fn community_root_pubkey(archive: &InMemoryArchive, did: &DidKey) -> Option<kc_core::PubKey> {
    archive.iter().find_map(|(_, e)| match e {
        ArchiveEntry::CommunityIdentityPublished(s) if s.payload.did == *did => {
            Some(s.payload.root_pubkey.clone())
        }
        _ => None,
    })
}

/// §4 decision-unsealing rule: references a *sealed* decision, is co-signed by exactly
/// the custodians it names, and **actually opens what the decision sealed**.
///
/// The shape is §8's `SourceReveal`, for the same reason: a rule about who signs is not
/// a guarantee about what is disclosed. The presented shares must recover the sealed
/// text, that text must open the decision's commitment, and `disclosed_reasoning` must be
/// what was recovered — so a quorum of custodians cannot publish a more flattering
/// account than the panel wrote and have the archive carry it as the proceedings.
///
/// Opening before `reveal_at` needs a named cause. After it, the seal has expired on its
/// own terms and the opening is bookkeeping.
fn authorize_decision_unsealed(
    archive: &InMemoryArchive,
    unsealed: &kc_mediation::DecisionUnsealed,
    signers: &[&str],
) -> KcResult<()> {
    let decision = match archive.get(&unsealed.decision_cid) {
        Some(ArchiveEntry::MediationDecided(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "decision unsealing references {} which is not a mediation decision",
                unsealed.decision_cid
            )))
        }
    };
    let sealed = match &decision.sealed {
        Some(s) => s,
        None => {
            return Err(KcError::Authority(
                "decision unsealing references a decision whose proceedings were never \
                 sealed"
                    .into(),
            ))
        }
    };
    let claimed: BTreeSet<&str> = unsealed.custodians.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "an unsealing's custodians must be exactly its co-signers; it names {} and \
             {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    let holders: BTreeSet<String> = sealed
        .custody
        .holders()
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();
    if let Some(stranger) = signers.iter().find(|s| !holders.contains(**s)) {
        return Err(KcError::Authority(format!(
            "{stranger} holds no share of this decision's custody, so has nothing to open"
        )));
    }
    if unsealed.unsealed_at < sealed.reveal_at && unsealed.cause.trim().is_empty() {
        return Err(KcError::Authority(
            "opening a seal before its generational cycle has run must cite a cause".into(),
        ));
    }
    let recovered = sealed.custody.recover(&unsealed.shares).map_err(|e| {
        KcError::Authority(format!("decision unsealing does not open the custody: {e}"))
    })?;
    let text = String::from_utf8(recovered).map_err(|_| {
        KcError::Authority("what the custody recovered is not the sealed reasoning text".into())
    })?;
    let commitment = kc_mediation::SealedReasoning::commitment(&text)
        .map_err(|e| KcError::Authority(format!("sealed reasoning commitment: {e}")))?;
    if commitment != sealed.reasoning_commitment {
        return Err(KcError::Authority(
            "what the custody recovered does not open the decision's reasoning commitment"
                .into(),
        ));
    }
    if unsealed.disclosed_reasoning != text {
        return Err(KcError::Authority(
            "the unsealing discloses a reasoning the custody shares do not recover".into(),
        ));
    }
    Ok(())
}

/// The §4 rule for a party's consent to the panel as constituted.
///
/// The party must be a party, and it must speak for itself: consent signed by anyone
/// else is not consent. The named panel must be one that could actually sit — every
/// member an active registered mediator, none from either party community — so a
/// consent cannot be recorded to a fiction and then counted as the real panel's.
///
/// Withholding must state something, since it is the answer that owes an account; the
/// statement is never read for content, exactly as a recusal's grounds are not.
fn authorize_panel_consent(
    archive: &InMemoryArchive,
    consent: &kc_mediation::PanelConsent,
    signers: &[&str],
) -> KcResult<()> {
    let dispute = match archive.get(&consent.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "panel consent references {} which is not a filed dispute",
                consent.dispute_cid
            )))
        }
    };
    if consent.party != dispute.complainant && consent.party != dispute.respondent {
        return Err(KcError::Authority(format!(
            "{} is not a party to this dispute, so has no consent to give in it",
            consent.party.as_str()
        )));
    }
    if consent.answer == kc_mediation::ConsentAnswer::Withheld && consent.statement.trim().is_empty()
    {
        return Err(KcError::Authority(
            "withheld consent must state something: it is the answer that owes an account".into(),
        ));
    }
    if consent.panel.is_empty() {
        return Err(KcError::Authority(
            "panel consent must name the panel it consents to".into(),
        ));
    }
    // The panel named must be one that could sit. Reuse the panel rule with a floor of
    // one, because what is checked here is that each named mediator is real and
    // non-party — whether the panel is large enough is the *decision's* rule, and
    // consent to a short panel is still a party's own act.
    let barred: BTreeSet<String> = [&dispute.complainant, &dispute.respondent]
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();
    let named: Vec<&str> = consent.panel.iter().map(|d| d.as_str()).collect();
    check_mediator_panel(
        archive,
        &named,
        consent.given_at,
        &barred,
        1,
        PanelSpec {
            role: "consented panelist",
            barred_kind: "a party community",
            label: "consented panel",
        },
    )?;
    let ok = signers.iter().any(|sg| {
        signer_authorized_for(
            archive,
            &consent.party,
            sg,
            consent.given_at,
            &[DelegationScope::Envoy],
        )
    });
    if !ok {
        return Err(KcError::Authority(format!(
            "panel consent for {} has no authorized signer: nobody consents on another's behalf",
            consent.party.as_str()
        )));
    }
    Ok(())
}

/// Whether `party` has consented to exactly `panel` on `dispute_cid`, as the archive
/// stands before `self_cid` in insertion order.
///
/// "Exactly" is the point: a party that accepted three mediators has not accepted a
/// fourth, so the consent is matched against the panel set rather than the dispute. The
/// party's latest answer before the decision governs, since consent can be given and
/// then withdrawn by a later `Withheld` on the same panel.
fn consented_to_panel(
    archive: &InMemoryArchive,
    dispute_cid: &Cid,
    party: &DidKey,
    panel: &BTreeSet<&str>,
    self_cid: &Cid,
) -> bool {
    let self_key = self_cid.to_string();
    let mut latest: Option<kc_mediation::ConsentAnswer> = None;
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::PanelConsentRecorded(s) = e {
            let c = &s.payload;
            if &c.dispute_cid != dispute_cid || c.party.as_str() != party.as_str() {
                continue;
            }
            let named: BTreeSet<&str> = c.panel.iter().map(|d| d.as_str()).collect();
            if &named == panel {
                latest = Some(c.answer);
            }
        }
    }
    latest == Some(kc_mediation::ConsentAnswer::Given)
}

/// The §4 rule for answering a recusal request, where who may sign depends on the
/// answer.
///
/// A **grant** is the named mediator's own act and theirs alone: stepping down is always
/// available to them and nobody else may condition it, which is the same rule every
/// self-signed act in this compact carries. A **decline** must come from registered
/// non-party mediators *other than* the one named — a panelist who could dismiss
/// requests against themselves is precisely the conflict the request exists to raise —
/// and must state reasons, since a refusal to step aside is the answer that owes an
/// account.
fn authorize_recusal_decision(
    archive: &InMemoryArchive,
    dec: &kc_mediation::RecusalDecision,
    signers: &[&str],
) -> KcResult<()> {
    let req = match archive.get(&dec.request_cid) {
        Some(ArchiveEntry::RecusalRequested(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "recusal decision references {} which is not a recusal request",
                dec.request_cid
            )))
        }
    };
    let dispute = match archive.get(&req.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(
                "the recusal request's dispute is not a filed dispute".into(),
            ))
        }
    };
    let claimed: BTreeSet<&str> = dec.deciders.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "a recusal decision's deciders must be exactly its signers; it names {} \
             and {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    match dec.answer {
        kc_mediation::RecusalAnswer::Granted => {
            if signers != [req.mediator.as_str()] {
                return Err(KcError::Authority(
                    "a recusal is granted by exactly the mediator stepping down".into(),
                ));
            }
        }
        kc_mediation::RecusalAnswer::Declined => {
            if dec.reasoning.trim().is_empty() {
                return Err(KcError::Authority(
                    "a declined recusal must state its reasoning".into(),
                ));
            }
            if signers.is_empty() {
                return Err(KcError::Authority(
                    "a recusal decision needs at least one decider".into(),
                ));
            }
            for sg in signers {
                if *sg == req.mediator.as_str() {
                    return Err(KcError::Authority(format!(
                        "mediator {sg} may not answer the request for their own recusal"
                    )));
                }
                let home = match mediator_home(archive, sg, dec.decided_at) {
                    None => {
                        return Err(KcError::Authority(format!(
                            "{sg} is not an active registered mediator, so may not \
                             answer a recusal request"
                        )))
                    }
                    Some(home) => home,
                };
                if home == dispute.complainant || home == dispute.respondent {
                    return Err(KcError::Authority(format!(
                        "mediator {sg} is from a party community (non-overlap violated)"
                    )));
                }
            }
        }
    }
    Ok(())
}

/// The §4 rule for a protecting-power designation: the Geneva analogue, and neutrality
/// is the whole of it.
///
/// `liminal_institutions.md`: "It is not an arbitrator; **it is a witness with
/// standing**." So this gates nothing — no decision waits on a protector and no panel is
/// invalid without one. What is enforced is that the protector is genuinely neutral (not
/// a party, and not itself seating the panel's mediators, or the witness and the judge
/// are one body) and that both sides consented: standing for another community is a duty
/// and is never conscripted, and a protector the disputant did not accept is a watcher
/// imposed on them.
fn authorize_protecting_power(
    archive: &InMemoryArchive,
    pp: &kc_mediation::ProtectingPower,
    signers: &[&str],
) -> KcResult<()> {
    let dispute = match archive.get(&pp.dispute_cid) {
        Some(ArchiveEntry::DisputeFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "protecting-power designation references {} which is not a filed dispute",
                pp.dispute_cid
            )))
        }
    };
    if pp.disputant != dispute.complainant && pp.disputant != dispute.respondent {
        return Err(KcError::Authority(format!(
            "{} is not a party to this dispute, so has no interests to protect in it",
            pp.disputant.as_str()
        )));
    }
    if pp.protector == dispute.complainant || pp.protector == dispute.respondent {
        return Err(KcError::Authority(format!(
            "{} is a party to this dispute and cannot stand neutral in it",
            pp.protector.as_str()
        )));
    }
    // The protector must not also be seating the pool this dispute's panel is drawn
    // from: a community that both watches the process and supplies its judges is not
    // the neutral third the Geneva analogue describes.
    let shares = active_roster_shares(
        &rosters_of(archive, RosterKind::Mediator),
        pp.designated_at,
    );
    if shares.contains_key(pp.protector.as_str()) {
        return Err(KcError::Authority(format!(
            "{} seats mediators and so cannot also stand as protecting power: a \
             witness with standing is not one of the judges",
            pp.protector.as_str()
        )));
    }
    // Both consent. Neither may be conscripted, and neither may be imposed.
    for principal in [&pp.disputant, &pp.protector] {
        let ok = signers.iter().any(|sg| {
            signer_authorized_for(
                archive,
                principal,
                sg,
                pp.designated_at,
                &[DelegationScope::Envoy],
            )
        });
        if !ok {
            return Err(KcError::Authority(format!(
                "protecting-power designation is not signed for {}; both the disputant \
                 and the protector must accept it",
                principal.as_str()
            )));
        }
    }
    Ok(())
}

/// The attested-null rule (§4 and §8, erosion remediation item 5): a community may only
/// account for a window in a function it actually held, and the account must be about
/// that window.
///
/// The empty `acts` list is deliberately legal, and is the whole point: a signed null is
/// the difference between "no disputes were filed" and "we stopped convening", which are
/// the same silence to every relative measure. What the rule refuses is an account of a
/// window that is not one (`window_end` at or before `window_start`), an account from a
/// community that seated nobody in that function (nothing to account for, and nobody to
/// hold to it), and acts cited from outside the window they are offered as evidence of.
fn authorize_activity_attestation(
    archive: &InMemoryArchive,
    att: &ActivityAttestation,
) -> KcResult<()> {
    if att.window_end <= att.window_start {
        return Err(KcError::Authority(format!(
            "activity attestation window ends at {}s, at or before its start {}s",
            att.window_end.as_second(),
            att.window_start.as_second()
        )));
    }
    // A null owes an account. `acts` empty and `statement` empty together is a record
    // that says nothing at all, which is the silence this record exists to end.
    if att.acts.is_empty() && att.statement.trim().is_empty() {
        return Err(KcError::Authority(
            "an attested null must state its account: an empty attestation is the silence \
             it exists to replace"
                .into(),
        ));
    }
    // The obligation belongs to a community that holds seats. Read at `window_end`,
    // because the window is what is being accounted for.
    let seated = holds_function(archive, &att.attesting_community, att.function, att.window_end);
    if !seated {
        return Err(KcError::Authority(format!(
            "{} seated no {} holders at the end of the window, so has no account to give",
            att.attesting_community.as_str(),
            att.function.as_str()
        )));
    }
    // Every cited act must fall inside the window, and be an act of the function.
    for cid in &att.acts {
        let at = match (att.function, archive.get(cid)) {
            (AttestedFunction::Mediation, Some(ArchiveEntry::MediationDecided(s))) => {
                s.payload.decided_at
            }
            (AttestedFunction::Mediation, Some(ArchiveEntry::DisputeFiled(s))) => {
                s.payload.filed_at
            }
            (AttestedFunction::Observation, Some(ArchiveEntry::ObserverReportPublished(s))) => {
                s.payload.reported_at
            }
            _ => {
                return Err(KcError::Authority(format!(
                    "attested act {cid} is not a {} act in this archive",
                    att.function.as_str()
                )))
            }
        };
        if at < att.window_start || at > att.window_end {
            return Err(KcError::Authority(format!(
                "attested act {cid} falls outside the window it is offered for"
            )));
        }
    }
    Ok(())
}

/// Whether `community` had at least one active holder of `function` at `at`. Reads the
/// same latest-issued-wins roster resolution (`store::latest_rosters`) that key
/// resolution and the pool lenses share, so "seated" means one thing in the crate.
fn holds_function(
    archive: &InMemoryArchive,
    community: &DidKey,
    function: AttestedFunction,
    at: Timestamp,
) -> bool {
    match function {
        AttestedFunction::Mediation => crate::store::latest_rosters(
            archive.iter().filter_map(|(_, e)| match e {
                ArchiveEntry::MediatorRegistryUpdated(s) => {
                    Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
                }
                _ => None,
            }),
            at,
        )
        .into_iter()
        .any(|r| {
            r.nominating_community.as_str() == community.as_str()
                && r.entries.iter().any(|e| e.active_at(at))
        }),
        AttestedFunction::Observation => crate::store::latest_rosters(
            archive.iter().filter_map(|(_, e)| match e {
                ArchiveEntry::ObserverRegistryUpdated(s) => {
                    Some((&s.payload.nominating_community, s.payload.issued_at, &s.payload))
                }
                _ => None,
            }),
            at,
        )
        .into_iter()
        .any(|r| {
            r.nominating_community.as_str() == community.as_str()
                && r.entries.iter().any(|e| e.active_at(at))
        }),
    }
}

/// The §5 rule for a re-examination conclusion (erosion remediation item 5, finding K3):
/// the attested null of the severest act the compact can take.
///
/// Two of these rules exist so the record cannot become an evasion of the obligation it
/// records. It may not be dated **before** the deadline it answers, or a compact would
/// confirm at designation time and buy permanent silence. And a `Confirmed` outcome must
/// name the next date, later than its own, or one confirmation would discharge a
/// generational obligation forever. A `Reversed` outcome requires the reversal itself to
/// be recorded by the path that already exists, so the archive keeps one definition of a
/// reversed designation rather than two that can disagree.
fn authorize_reexamination(
    archive: &InMemoryArchive,
    rc: &ReexaminationConclusion,
) -> KcResult<()> {
    let designation = match archive.get(&rc.designation_cid) {
        Some(ArchiveEntry::DesignationDecided(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "re-examination references {} which is not a designation",
                rc.designation_cid
            )))
        }
    };
    let Some(due) = designation.reexamine_after else {
        return Err(KcError::Authority(format!(
            "designation {} mandated no re-examination, so none can conclude one",
            rc.designation_cid
        )));
    };
    if rc.concluded_at < due {
        return Err(KcError::Authority(format!(
            "re-examination concluded at {}s, before the mandated date {}s: a \
             re-examination cannot be held in advance of the thing it examines",
            rc.concluded_at.as_second(),
            due.as_second()
        )));
    }
    if rc.reasoning.trim().is_empty() {
        return Err(KcError::Authority(
            "a re-examination must publish its reasoning".into(),
        ));
    }
    match rc.outcome {
        ReexaminationOutcome::Confirmed => match rc.next_reexamine_after {
            None => {
                return Err(KcError::Authority(
                    "a confirmed designation must set the next re-examination date: a \
                     confirmation renews the obligation, it does not discharge it"
                        .into(),
                ))
            }
            Some(next) if next <= rc.concluded_at => {
                return Err(KcError::Authority(format!(
                    "the next re-examination date {}s is not after this conclusion {}s",
                    next.as_second(),
                    rc.concluded_at.as_second()
                )))
            }
            Some(_) => {}
        },
        ReexaminationOutcome::Reversed => {
            if !designation_reversed(archive, &rc.designation_cid, rc.concluded_at) {
                return Err(KcError::Authority(format!(
                    "re-examination reports {} reversed, but no lifting record or \
                     re-entry answers it",
                    rc.designation_cid
                )));
            }
        }
    }
    Ok(())
}

/// Whether a designation has been reversed as of `at`, by the two paths the reputation
/// lookup already defines: a `DesignationLifted` conduct record naming it, or a completed
/// re-entry answering it. One definition, read by the authority pass and the erosion lens
/// alike.
fn designation_reversed(archive: &InMemoryArchive, designation_cid: &Cid, at: Timestamp) -> bool {
    archive.iter().any(|(_, e)| match e {
        ArchiveEntry::ConductAttested(s) => {
            s.payload.kind == kc_conduct::ConductKind::DesignationLifted
                && s.payload.recorded_at <= at
                && s.payload.references.iter().any(|r| r == designation_cid)
        }
        ArchiveEntry::ReEntryCompleted(s) => {
            s.payload.completed_at <= at && &s.payload.designation_cid == designation_cid
        }
        _ => false,
    })
}

/// The one refusal erosion remediation carries (the rest is reported, not enforced): a
/// designation whose mandated re-examination has come and gone unanswered may not be
/// **cited** by a further escalation act.
///
/// The most severe act the compact can take cannot rest on an obligation the compact has
/// itself let lapse. There is an act here to hang a refusal on, which is why this is a
/// gate while the silent-function and unstaffed-function readings are lenses: a compact
/// that has merely stopped acting offers nothing to refuse.
fn no_stale_designation_cited<'a>(
    archive: &InMemoryArchive,
    cited: impl Iterator<Item = &'a Cid>,
    at: Timestamp,
) -> KcResult<()> {
    for cid in cited {
        let Some(ArchiveEntry::DesignationDecided(s)) = archive.get(cid) else { continue };
        let Some(due) = s.payload.reexamine_after else { continue };
        if due > at || designation_reversed(archive, cid, at) {
            continue;
        }
        if latest_reexamination(archive, cid, at).is_none() {
            return Err(KcError::Authority(format!(
                "cited designation {cid} was due for re-examination at {}s and none is \
                 recorded: a further escalation may not rest on a lapsed obligation",
                due.as_second()
            )));
        }
    }
    Ok(())
}

/// The latest re-examination conclusion for a designation at or before `at`, by
/// `concluded_at`. Insertion order breaks ties, so the archive's own order is
/// authoritative exactly as it is for §4's argument answering.
fn latest_reexamination<'a>(
    archive: &'a InMemoryArchive,
    designation_cid: &Cid,
    at: Timestamp,
) -> Option<&'a ReexaminationConclusion> {
    let mut best: Option<&ReexaminationConclusion> = None;
    for (_, e) in archive.iter() {
        if let ArchiveEntry::ReexaminationConcluded(s) = e {
            let rc = &s.payload;
            if &rc.designation_cid != designation_cid || rc.concluded_at > at {
                continue;
            }
            if best.is_none_or(|b| rc.concluded_at >= b.concluded_at) {
                best = Some(rc);
            }
        }
    }
    best
}

/// The §4 panel rule for a mediation decision: every co-signer must be an active
/// registered mediator; the panel must span at least `panel_min` of them; no
/// panelist may come from a party community (non-overlap); and a panelist whose home
/// community is probationary does not count toward `panel_min` (§2).
fn authorize_decision(
    archive: &InMemoryArchive,
    decision: &MediationDecision,
    signers: &[&str],
    panel_min: usize,
    self_cid: &Cid,
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
    // §4 "Decision publication: public decisions with **mandatory** reasoning trail;
    // sealed decisions with threshold-reveal" (protocol-components.md). The field
    // existed and nothing read it, so a panel could publish a decision that said nothing
    // about why. A sealed decision satisfies the trail differently — the reasoning
    // exists and is recoverable, it is simply not public yet — and must then carry no
    // cleartext reasoning at all, so a decision has exactly one account of itself.
    match &decision.sealed {
        None => {
            if decision.reasoning.trim().is_empty() {
                return Err(KcError::Authority(
                    "a mediation decision must state its reasoning (§4's reasoning trail is \
                     mandatory) or seal it"
                        .into(),
                ));
            }
        }
        Some(sealed) => {
            if !decision.reasoning.trim().is_empty() {
                return Err(KcError::Authority(
                    "a sealed decision carries no cleartext reasoning: a public summary \
                     beside a sealed trail is a second account of the same decision, and \
                     nothing could ever hold the two to each other"
                        .into(),
                ));
            }
            authorize_sealed_reasoning(archive, decision, dispute, sealed)?;
        }
    }
    // The `panel` field is **derived, not chosen**: it must be exactly the co-signers.
    // Before this rule nothing read it, so it could name the two *party* communities —
    // precisely whom non-overlap excludes — while three legitimate non-party mediators
    // signed, and the record was authorized. A field a reader sees without the envelope
    // to hand must not be able to contradict the rule that actually ran.
    let claimed: BTreeSet<&str> = decision.panel.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "a mediation decision's panel must be exactly its co-signers; it names {} \
             and {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    // The deliberation the decision rests on (§4 "Deliberation support"). Every cited
    // record must be an argument on *this* dispute, so a decision can neither cite
    // another dispute's deliberation nor point at an unrelated record.
    let mut argued_by: BTreeSet<&str> = BTreeSet::new();
    for cid in &decision.arguments {
        let arg = match archive.get(cid) {
            Some(ArchiveEntry::ArgumentFiled(s)) => &s.payload,
            _ => {
                return Err(KcError::Authority(format!(
                    "mediation decision cites {cid} which is not a filed argument"
                )))
            }
        };
        if arg.dispute_cid != decision.dispute_cid {
            return Err(KcError::Authority(format!(
                "mediation decision cites {cid}, an argument on another dispute"
            )));
        }
        if arg.role == ArgumentRole::Panelist {
            argued_by.insert(arg.author.as_str());
        }
    }
    // **Every co-signing panelist must have argued.** Otherwise three signatures ride
    // on one author's text and the panel is a rubber stamp: the same arithmetic as the
    // witness rules, which count the people present rather than the signatures
    // collected.
    if let Some(silent) = signers.iter().find(|s| !argued_by.contains(**s)) {
        return Err(KcError::Authority(format!(
            "panelist {silent} co-signs a decision without having argued it"
        )));
    }
    // **Every party argument already in the log must be answered.** A panel may not
    // decide while ignoring a case put to it. Insertion order is authoritative, as it
    // is for a dispute's current rung, so a party filing *after* the decision cannot
    // retroactively invalidate it.
    let cited: BTreeSet<String> = decision.arguments.iter().map(|c| c.to_string()).collect();
    let self_key = self_cid.to_string();
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::ArgumentFiled(s) = e {
            if s.payload.dispute_cid == decision.dispute_cid
                && s.payload.role == ArgumentRole::Party
                && !cited.contains(&cid.to_string())
            {
                return Err(KcError::Authority(format!(
                    "mediation decision does not answer the party argument {cid} filed \
                     before it"
                )));
            }
        }
    }
    // **A settlement is a claim about the parties, not about the panel.** `Settled`
    // says the two of them reached a mediated settlement, so both must have consented
    // to the panel that records it. Nothing else waits on consent: requiring it to
    // decide at all would hand either party a veto over its own dispute, which is
    // exactly what the escalation ladder exists to route around. A party that refuses
    // to settle blocks nothing — the panel upholds, dismisses or refers instead.
    if decision.outcome == kc_mediation::DecisionOutcome::Settled {
        for party in [&dispute.complainant, &dispute.respondent] {
            if !consented_to_panel(archive, &decision.dispute_cid, party, &actual, self_cid) {
                return Err(KcError::Authority(format!(
                    "a settled outcome needs {}'s consent to this panel, and the archive \
                     holds none: a settlement the parties did not consent to is not a \
                     settlement",
                    party.as_str()
                )));
            }
        }
    }
    let barred: BTreeSet<String> = [&dispute.complainant, &dispute.respondent]
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();
    check_mediator_panel(
        archive,
        signers,
        decision.decided_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "panelist",
            barred_kind: "a party community",
            label: "mediation panel",
        },
    )
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
        // Weight, not signatures (§2) — the commons second-class floor's reasoning,
        // one component over.
        let tally = deliberative_weight(
            archive,
            transition.initiators.iter(),
            transition.transitioned_at,
            None,
            None,
        );
        let floor = quorum_threshold.max(2);
        if tally.counted < floor {
            return Err(KcError::Authority(format!(
                "{:?} needs >= {} distinct initiating communities, got {}{}",
                transition.to_rung,
                floor,
                tally.counted,
                tally.because().unwrap_or_default()
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
            if d.reversal_conditions.as_ref().is_none_or(|s| s.trim().is_empty()) {
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

    // Erosion remediation's one refusal: no lapsed designation may be cited in support
    // of a new one.
    no_stale_designation_cited(
        archive,
        d.cited_acts.iter().chain(d.failed_restoration_cycles.iter()),
        d.designated_at,
    )?;

    // Condition 2: compact-wide deliberation.
    //
    // A cycle's *published reasoning* is checked here for both urgencies, because it is
    // the same requirement §4's decisions carry and the record had it as decoration:
    // `criteria`, `cited_acts` and `reversal_conditions` were all enforced non-empty
    // while the deliberation's own reasoning was not, so a designation could name its
    // representatives and say nothing about what they concluded. "Designation is
    // published and named ... other parties must be able to see the designation, the
    // reasons for it, and the conduct that triggered it" (bounded_conflict.md).
    if let Some(c) = d.deliberation_cycles.iter().find(|c| c.reasoning.trim().is_empty()) {
        return Err(KcError::Authority(format!(
            "deliberation cycle {} must publish its reasoning (condition 2)",
            c.cycle_index
        )));
    }
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
            // §2: probationary deliberators carry no weight in designation
            // procedures, so they do not count toward it.
            let distinct: BTreeSet<&str> = d
                .deliberators
                .iter()
                .filter(|x| !probationary_at(archive, x, d.designated_at))
                .map(|x| x.as_str())
                .collect();
            if distinct.len() < quorum_threshold + 1 {
                return Err(KcError::Authority(format!(
                    "expedited designation needs >= {} distinct communities carrying \
                     deliberative weight, got {} (§2 excludes probationary deliberators)",
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

    // The expedited path's covenant-facing floor.
    //
    // Condition 1 is relaxed above for the emergency path, and condition 3 checks only
    // that `criteria` is non-empty and `cited_acts` is non-empty. The CIDs themselves are
    // never resolved, so without this the emergency route is the one place a designation
    // may be assembled out of records describing what a community does inside itself, or
    // out of CIDs naming no archive record at all.
    //
    // natural_law.md ("The interior of a member community") holds that the compact forms
    // no verdict on a member's internal morality until that morality touches the covenant.
    // The normal path carries that structurally, because condition 1's restitution chains
    // resolve through `RestitutionVerified` and `ConductAttested`, which are compact-facing
    // by construction. This is the expedited path's equivalent: the emergency route may
    // shorten the pattern, and may not dispense with a covenant-facing act altogether.
    if matches!(d.urgency, DesignationUrgency::Expedited)
        && !cites_adverse_conduct(archive, &d.subject_community, &d.cited_acts, d.designated_at)
    {
        return Err(KcError::Authority(
            "expedited designation must cite at least one attested adverse act by the \
             subject: the covenant-facing floor that the relaxed condition 1 leaves"
                .into(),
        ));
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

/// True if any entry in `cited` resolves to an attested adverse `ConductRecord`
/// naming `subject` and recorded no later than the designation itself. The
/// expedited path's covenant-facing floor; see `authorize_designation`.
///
/// Note (design limit, the same one `sustained_bad_faith` carries): this asks that
/// a covenant-facing act be cited, not that the *pattern* be one. A single adverse
/// record satisfies it, which is what "expedited" means. The gravity of the act is
/// not machine-checked and remains the deliberators' judgment.
fn cites_adverse_conduct(
    archive: &InMemoryArchive,
    subject: &DidKey,
    cited: &[Cid],
    at: Timestamp,
) -> bool {
    cited.iter().any(|cid| match archive.get(cid) {
        Some(ArchiveEntry::ConductAttested(s)) => {
            &s.payload.subject_community == subject
                && s.payload.kind.is_adverse()
                && s.payload.recorded_at <= at
        }
        _ => false,
    })
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
/// community (non-overlap), and the panel spans at least `panel_min` — counting
/// weight, so a probationary home does not verify a restitution (§2).
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
    let barred: BTreeSet<String> = [&obligation.subject_community, &obligation.beneficiary]
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();
    check_mediator_panel(
        archive,
        signers,
        completion.verified_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "restitution verifier",
            barred_kind: "a party community",
            label: "restitution verifier panel",
        },
    )
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

/// The commons registered at `commons_cid`, or an error if it does not resolve to a
/// `CommonsRegistered` entry. The registration itself, rather than only its appropriator
/// set, because a sealed usage record needs the draw allowance too.
fn commons_registration<'a>(
    archive: &'a InMemoryArchive,
    commons_cid: &Cid,
    context: &str,
) -> KcResult<&'a kc_commons::CommonsRegistration> {
    match archive.get(commons_cid) {
        Some(ArchiveEntry::CommonsRegistered(s)) => Ok(&s.payload),
        _ => Err(KcError::Authority(format!(
            "{context} references {commons_cid} which is not a registered commons"
        ))),
    }
}

/// §7 usage rule: a usage record must report on a registered commons, and the
/// reporting community must be one of that commons' appropriators (a usage claim
/// from a non-appropriator is meaningless). Principal binding is already enforced
/// generically before this runs.
///
/// A record that accounts under the **surveillance threshold** rather than in public is
/// held to more: see [`authorize_sealed_usage`].
fn authorize_usage(archive: &InMemoryArchive, u: &UsageRecord) -> KcResult<()> {
    let reg = commons_registration(archive, &u.commons_cid, "usage record")?;
    if !reg
        .appropriators
        .iter()
        .any(|a| a.as_str() == u.community.as_str())
    {
        return Err(KcError::Authority(format!(
            "usage community {} is not an appropriator of the commons",
            u.community.as_str()
        )));
    }
    if let Some(sealed) = &u.sealed {
        authorize_sealed_usage(u, reg, sealed)?;
    }
    Ok(())
}

/// §7 sealed-usage rule: what a community must show when it accounts under the
/// surveillance threshold instead of in public.
///
/// Four things, and the first is the one that keeps this from being a way to disappear.
///
/// **There must be something to prove.** A commons with no `draw_allowance` states no
/// machine-readable cap, so a sealed record against it would commit figures and
/// demonstrate nothing about them, which is worse than publishing: the compact would
/// hold a number it cannot see and cannot bound. Sealing is available exactly where the
/// commons has said what "within bounds" means.
///
/// **The proof must verify** against the commons' allowance and this record's own
/// commitment. It cannot be lifted from another record, another commons or another
/// bound; the bit commitments recombine to one commitment and no other.
///
/// **The cleartext fields must be zero.** A record carries its figures once. Otherwise a
/// community could publish a flattering number beside a sealed real one, and no rule
/// could ever hold the two to each other, which is the same reason a sealed §4 decision
/// carries no public summary.
///
/// **The custody must not be the reporter's own.** Every custodian is a fellow
/// appropriator of that commons and none is the reporting community: a community holding
/// a quorum of the shares over its own figures has sealed nothing from anyone who
/// matters, and the appropriator set is the right custodian pool because those are the
/// parties the accounting is owed to.
fn authorize_sealed_usage(
    u: &UsageRecord,
    reg: &kc_commons::CommonsRegistration,
    sealed: &kc_commons::SealedUsage,
) -> KcResult<()> {
    let allowance = reg.draw_allowance.ok_or_else(|| {
        KcError::Authority(
            "this commons states no draw allowance, so a sealed usage record would commit \
             figures and prove nothing about them"
                .into(),
        )
    })?;
    if u.consumed != 0 || u.contributed != 0 {
        return Err(KcError::Authority(
            "a sealed usage record carries no cleartext figures: a public number beside a \
             sealed one is a second account of the same draw, and nothing could hold the \
             two to each other"
                .into(),
        ));
    }
    sealed
        .within_allowance
        .verify(&sealed.consumed_commitment, allowance)
        .map_err(|e| {
            KcError::Authority(format!(
                "sealed usage does not prove its draw is within the {allowance} allowance: {e}"
            ))
        })?;
    sealed
        .custody
        .check_shape()
        .map_err(|e| KcError::Authority(format!("sealed usage custody: {e}")))?;
    let appropriators: BTreeSet<&str> = reg.appropriators.iter().map(|d| d.as_str()).collect();
    for holder in sealed.custody.holders() {
        if holder.as_str() == u.community.as_str() {
            return Err(KcError::Authority(format!(
                "{} holds a share of the custody over its own usage figures",
                holder.as_str()
            )));
        }
        if !appropriators.contains(holder.as_str()) {
            return Err(KcError::Authority(format!(
                "{} is not an appropriator of this commons, so has no standing to hold a \
                 share of its usage figures",
                holder.as_str()
            )));
        }
    }
    Ok(())
}

/// §7 usage-opening rule: a custodian quorum recovering a sealed record's figures.
///
/// Mirrors §4's `authorize_decision_unsealed`. The custodians are derived from the
/// co-signers, each must actually hold a share, and the presented shares must recover
/// figures that **open both commitments** — so a quorum cannot publish a number the
/// community never committed to. A cause is required, since opening is an act somebody
/// should have to account for even though it needs no waiting period.
fn authorize_usage_opened(
    archive: &InMemoryArchive,
    op: &kc_commons::UsageOpened,
    signers: &[&str],
) -> KcResult<()> {
    let usage = match archive.get(&op.usage_cid) {
        Some(ArchiveEntry::UsageAccounted(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "usage opening references {} which is not a usage record",
                op.usage_cid
            )))
        }
    };
    let sealed = match &usage.sealed {
        Some(s) => s,
        None => {
            return Err(KcError::Authority(
                "usage opening references a record whose figures were never sealed".into(),
            ))
        }
    };
    if op.cause.trim().is_empty() {
        return Err(KcError::Authority(
            "opening a sealed usage record must cite a cause".into(),
        ));
    }
    let claimed: BTreeSet<&str> = op.custodians.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "a usage opening's custodians must be exactly its co-signers; it names {} and \
             {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    let holders: BTreeSet<String> = sealed
        .custody
        .holders()
        .iter()
        .map(|d| d.as_str().to_string())
        .collect();
    if let Some(stranger) = signers.iter().find(|s| !holders.contains(**s)) {
        return Err(KcError::Authority(format!(
            "{stranger} holds no share of this record's custody, so has nothing to open"
        )));
    }
    let plaintext = sealed.custody.recover(&op.shares).map_err(|e| {
        KcError::Authority(format!("usage opening does not open the custody: {e}"))
    })?;
    let figures = kc_commons::SealedUsage::figures_from(&plaintext).map_err(|e| {
        KcError::Authority(format!("what the custody recovered is not usage figures: {e}"))
    })?;
    let consumed_blinding = kc_core::Blinding::from_bytes(&figures.consumed_blinding)
        .map_err(|e| KcError::Authority(format!("recovered consumed blinding: {e}")))?;
    let contributed_blinding = kc_core::Blinding::from_bytes(&figures.contributed_blinding)
        .map_err(|e| KcError::Authority(format!("recovered contributed blinding: {e}")))?;
    if !sealed
        .consumed_commitment
        .opens_to(figures.consumed, &consumed_blinding)
        || !sealed
            .contributed_commitment
            .opens_to(figures.contributed, &contributed_blinding)
    {
        return Err(KcError::Authority(
            "the recovered figures do not open the record's own commitments".into(),
        ));
    }
    if op.consumed != figures.consumed || op.contributed != figures.contributed {
        return Err(KcError::Authority(
            "the opening discloses figures the custody shares do not recover".into(),
        ));
    }
    Ok(())
}

/// §7 compact-charter rule (Ostrom principle 8): what constitutes a compact.
///
/// A community's identity is self-signed, because a community exists by its own act. A
/// compact does not: it is constituted by its members, so the charter needs a quorum of
/// them (generic, via the second-class flag) **and** the compact's own signature.
///
/// The compact's own signature is not ceremony. The DID is derived from the key on the
/// record, and without a signature proving somebody holds that key, a quorum could
/// charter a compact around a key nobody has, and every later act signed by it would rest
/// on an assumption the archive never checked. It is also what stops a compact being
/// chartered around a key its members hold jointly and no compact-level body holds at
/// all.
fn authorize_compact_charter(
    archive: &InMemoryArchive,
    c: &kc_identity::CompactCharter,
    signers: &[&str],
) -> KcResult<()> {
    if c.label.trim().is_empty() {
        return Err(KcError::Authority("a compact charter must be named".into()));
    }
    if c.members.is_empty() {
        return Err(KcError::Authority(
            "a compact is constituted by its members and this charter names none".into(),
        ));
    }
    let derived = DidKey::from_pubkey(&c.root_pubkey)
        .map_err(|e| KcError::Authority(format!("compact charter key: {e}")))?;
    if derived != c.did {
        return Err(KcError::Authority(
            "a compact charter's DID must be derived from the key it carries".into(),
        ));
    }
    if !signers.contains(&c.did.as_str()) {
        return Err(KcError::Authority(format!(
            "the compact {} did not sign its own charter, so nothing shows the key it \
             will act under is held by anyone",
            c.did.as_str()
        )));
    }
    let members: BTreeSet<&str> = c.members.iter().map(|d| d.as_str()).collect();
    for ch in &c.chartered_by {
        if !members.contains(ch.as_str()) {
            return Err(KcError::Authority(format!(
                "{} charters this compact without being one of its members",
                ch.as_str()
            )));
        }
    }
    // Every member must be a community the archive knows, or the compact is constituted
    // by names rather than by communities.
    for m in &c.members {
        if community_root_pubkey(archive, m).is_none() {
            return Err(KcError::Authority(format!(
                "{} is named a member of this compact but has published no community \
                 identity",
                m.as_str()
            )));
        }
    }
    Ok(())
}

/// The compact chartered at some point in the archive that has `community` as a member,
/// if any. Used by the subsidiarity rule to find the body a community answers through.
fn compact_of(archive: &InMemoryArchive, community: &DidKey) -> Option<DidKey> {
    archive.iter().find_map(|(_, e)| match e {
        ArchiveEntry::CompactChartered(s)
            if s.payload.members.iter().any(|m| m == community) =>
        {
            Some(s.payload.did.clone())
        }
        _ => None,
    })
}

/// §7 **subsidiarity**: the inter-compact level may not reach past a compact to its
/// member communities.
///
/// This is what makes nesting real rather than decorative. Ostrom's principle 8 is not
/// "there is a higher level" but that each level governs at its own scale, and the scale
/// of an inter-compact commons is compacts. A federation that could sanction a member
/// community directly would be governing at the compact's scale while answering to
/// nobody at it, which is exactly the sovereignty drift `liminal_institutions.md` warns
/// about, arriving one level up. Measured before the rule: nothing stopped it, because
/// nothing distinguished the levels at all.
///
/// So on a `Compact`-level commons, the subject of a sanction or an escalation must be
/// one of its appropriator compacts. A compact answers for its members; its members
/// answer to it.
fn check_subsidiarity(
    archive: &InMemoryArchive,
    reg: &kc_commons::CommonsRegistration,
    subject: &DidKey,
    what: &str,
) -> KcResult<()> {
    if reg.level != kc_commons::AppropriatorLevel::Compact {
        return Ok(());
    }
    let is_appropriator = reg.appropriators.iter().any(|a| a == subject);
    if is_appropriator {
        return Ok(());
    }
    if let Some(compact) = compact_of(archive, subject) {
        return Err(KcError::Authority(format!(
            "{what} names the member community {}, but this commons is held at the \
             inter-compact level: it reaches {} only through its compact {}",
            subject.as_str(),
            subject.as_str(),
            compact.as_str()
        )));
    }
    Err(KcError::Authority(format!(
        "{what} names {}, which is not an appropriator compact of this inter-compact \
         commons",
        subject.as_str()
    )))
}

/// §7 commons-escalation rule: the four conditions under which a compact may find that
/// a community's abuse of a commons has passed beyond graduated sanctions.
///
/// This is the threshold `commons_federation.md` leaves open ("the trigger threshold for
/// commons-specific violations needs specification"), specified in the shape §5's own
/// four-condition anathema test already carries, so the two read as one family. The
/// distinct-community quorum arrives generically: the record is always second-class.
///
/// Insertion order is authoritative throughout, as it is for a dispute's rung and a
/// commons' current level, so a sanction appended after the finding cannot retroactively
/// make it valid or invalid.
fn authorize_commons_escalation(
    archive: &InMemoryArchive,
    e: &kc_commons::CommonsEscalation,
    self_cid: &Cid,
) -> KcResult<()> {
    let reg = commons_registration(archive, &e.commons_cid, "commons escalation")?;
    check_subsidiarity(archive, reg, &e.subject_community, "a commons escalation")?;
    if !reg
        .appropriators
        .iter()
        .any(|a| a.as_str() == e.subject_community.as_str())
    {
        return Err(KcError::Authority(format!(
            "commons escalation subject {} is not an appropriator of the commons",
            e.subject_community.as_str()
        )));
    }
    if e.basis.trim().is_empty() {
        return Err(KcError::Authority(
            "a commons escalation must state the pattern it finds".into(),
        ));
    }

    // Condition 3's citation half, borrowed from §5: a finding this heavy cites the
    // specific acts it rests on, and each must be a sanction on this pair.
    if e.cited_sanctions.is_empty() {
        return Err(KcError::Authority(
            "a commons escalation must cite the sanctions it rests on".into(),
        ));
    }
    for cid in &e.cited_sanctions {
        match archive.get(cid) {
            Some(ArchiveEntry::SanctionApplied(s))
                if s.payload.commons_cid == e.commons_cid
                    && s.payload.subject_community == e.subject_community => {}
            Some(ArchiveEntry::SanctionApplied(_)) => {
                return Err(KcError::Authority(format!(
                    "commons escalation cites {cid}, a sanction on another commons or another \
                     community"
                )))
            }
            _ => {
                return Err(KcError::Authority(format!(
                    "commons escalation cites {cid} which is not a commons sanction"
                )))
            }
        }
    }

    // Walk this pair's ladder history up to the finding, counting arrivals at Exclusion
    // and remembering when the standing one began.
    let self_key = self_cid.to_string();
    let mut level = SanctionLevel::None;
    let mut arrivals = 0usize;
    let mut excluded_since: Option<Timestamp> = None;
    for (cid, entry) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        if let ArchiveEntry::SanctionApplied(s) = entry {
            let sanc = &s.payload;
            if sanc.commons_cid != e.commons_cid
                || sanc.subject_community != e.subject_community
            {
                continue;
            }
            if sanc.to_level == SanctionLevel::Exclusion && level != SanctionLevel::Exclusion {
                arrivals += 1;
                excluded_since = Some(sanc.decided_at);
            }
            if sanc.to_level != SanctionLevel::Exclusion {
                excluded_since = None;
            }
            level = sanc.to_level;
        }
    }

    // Condition 1: the ladder is exhausted.
    if level != SanctionLevel::Exclusion {
        return Err(KcError::Authority(format!(
            "commons escalation needs the subject at Exclusion and it is at {}: graduated \
             sanctions are the answer until they have demonstrably failed",
            level.as_str()
        )));
    }

    // Conditions 2 and 4: defiance rather than compliance, counted lower for a commons
    // the compact cannot pause.
    let required = if kc_commons::is_life_critical(reg.kind) {
        DEFAULT_LIFE_CRITICAL_ESCALATION_CYCLES
    } else {
        DEFAULT_COMMONS_ESCALATION_CYCLES
    };
    if arrivals < required {
        return Err(KcError::Authority(format!(
            "commons escalation needs {required} arrivals at Exclusion on a {} commons and \
             the subject has {arrivals}: one offence is what the ladder is for",
            reg.kind.as_str()
        )));
    }

    // Condition 3: restoration was available and not taken.
    let since = excluded_since.ok_or_else(|| {
        KcError::Authority("the subject's exclusion has no recorded beginning".into())
    })?;
    let dwelt = e.escalated_at.as_second() - since.as_second();
    if dwelt < DEFAULT_COMMONS_EXCLUSION_DWELL_SECONDS {
        return Err(KcError::Authority(format!(
            "commons escalation needs {}s at Exclusion and the subject has stood {dwelt}s: \
             restoration is available from exclusion, and a community in the middle of \
             taking it has not refused it",
            DEFAULT_COMMONS_EXCLUSION_DWELL_SECONDS
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
    let reg = commons_registration(archive, &s.commons_cid, "commons sanction")?;
    check_subsidiarity(archive, reg, &s.subject_community, "a commons sanction")?;
    if !reg
        .appropriators
        .iter()
        .any(|a| a.as_str() == s.subject_community.as_str())
    {
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
        // Weight, not signatures (§2): the `max(2)` floor is *higher* than the generic
        // quorum wherever a compact configures one below it, so it needs the same
        // filter or it becomes the cheaper of the two routes.
        let tally = deliberative_weight(archive, s.imposers.iter(), s.decided_at, None, None);
        let floor = quorum_threshold.max(2);
        if tally.counted < floor {
            return Err(KcError::Authority(format!(
                "{:?} needs >= {} distinct imposing communities, got {}{}",
                s.to_level,
                floor,
                tally.counted,
                tally.because().unwrap_or_default()
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

/// Which kind of service roster: the §8 observer corps or the §2 examining pool.
///
/// The two are the same structure — a nominating community, an `issued_at` that
/// supersedes its own earlier rosters, and a list of seats with validity windows —
/// so the capture-resistance caps over them are computed once, here, rather than
/// restated per component. `capture_resistance.md` §7 asks for exactly that: the
/// credentialing function "rotated, witnessed, and monitored like the mediator pool
/// it resembles".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RosterKind {
    /// §8 observer corps (`jester_and_dissent.md` "Rotation and diversity").
    Observer,
    /// §2 examining pool (`capture_resistance.md` §7 "Credentialing panels by lot
    /// and rotation").
    Examiner,
    /// §4 mediator pool (`liminal_institutions.md` "The mediator pool" and
    /// "Discernment training").
    Mediator,
}

impl RosterKind {
    /// The word for one servant of this roster, used in refusal messages.
    fn role(self) -> &'static str {
        match self {
            Self::Observer => "observer",
            Self::Examiner => "examiner",
            Self::Mediator => "mediator",
        }
    }

    /// The protocol section whose rule a refusal is citing.
    fn section(self) -> &'static str {
        match self {
            Self::Observer => "§8",
            Self::Examiner => "§2",
            Self::Mediator => "§4",
        }
    }
}

/// One seat in a roster: who serves, and over what window.
struct Seat<'a> {
    did: &'a str,
    from: Timestamp,
    until: Option<Timestamp>,
}

impl Seat<'_> {
    fn active_at(&self, t: Timestamp) -> bool {
        self.from <= t && self.until.is_none_or(|end| t < end)
    }
}

/// One roster record, normalized across kinds.
struct Roster<'a> {
    cid: &'a Cid,
    community: &'a DidKey,
    issued_at: Timestamp,
    seats: Vec<Seat<'a>>,
}

/// Every roster of `kind` in the archive, in insertion order, normalized. The only
/// per-kind code in the caps machinery.
fn rosters_of(archive: &InMemoryArchive, kind: RosterKind) -> Vec<Roster<'_>> {
    let mut out = Vec::new();
    for (cid, e) in archive.iter() {
        match (kind, e) {
            (RosterKind::Observer, ArchiveEntry::ObserverRegistryUpdated(s)) => {
                out.push(Roster {
                    cid,
                    community: &s.payload.nominating_community,
                    issued_at: s.payload.issued_at,
                    seats: s
                        .payload
                        .entries
                        .iter()
                        .map(|en| Seat {
                            did: en.observer_did.as_str(),
                            from: en.valid_from,
                            until: en.valid_until,
                        })
                        .collect(),
                });
            }
            (RosterKind::Examiner, ArchiveEntry::ExaminerRegistryUpdated(s)) => {
                out.push(Roster {
                    cid,
                    community: &s.payload.nominating_community,
                    issued_at: s.payload.issued_at,
                    seats: s
                        .payload
                        .entries
                        .iter()
                        .map(|en| Seat {
                            did: en.examiner_did.as_str(),
                            from: en.valid_from,
                            until: en.valid_until,
                        })
                        .collect(),
                });
            }
            (RosterKind::Mediator, ArchiveEntry::MediatorRegistryUpdated(s)) => {
                out.push(Roster {
                    cid,
                    community: &s.payload.nominating_community,
                    issued_at: s.payload.issued_at,
                    seats: s
                        .payload
                        .entries
                        .iter()
                        .map(|en| Seat {
                            did: en.mediator_did.as_str(),
                            from: en.valid_from,
                            until: en.valid_until,
                        })
                        .collect(),
                });
            }
            _ => {}
        }
    }
    out
}

/// The active roster as of `at`, per seating community, respecting superseding (only
/// the latest roster per nominating community with `issued_at <= at` contributes its
/// seats). The pool total is the sum; one community's share is its entry.
fn active_roster_shares(rosters: &[Roster<'_>], at: Timestamp) -> BTreeMap<String, usize> {
    let mut latest: BTreeMap<&str, &Roster> = BTreeMap::new();
    for r in rosters {
        if r.issued_at.as_second() <= at.as_second() {
            let key = r.community.as_str();
            let supersede = latest
                .get(key)
                .is_none_or(|cur| r.issued_at.as_second() >= cur.issued_at.as_second());
            if supersede {
                latest.insert(key, r);
            }
        }
    }
    latest
        .values()
        .map(|r| {
            (
                r.community.as_str().to_string(),
                r.seats.iter().filter(|s| s.active_at(at)).count(),
            )
        })
        .collect()
}

/// The bounded windows `did` has served in rosters of this kind other than
/// `self_cid`. Superseding is irrelevant: a term served is a term served.
fn prior_service_windows(
    rosters: &[Roster<'_>],
    did: &str,
    self_cid: &Cid,
) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for r in rosters {
        if r.cid == self_cid {
            continue;
        }
        for seat in &r.seats {
            if seat.did == did {
                if let Some(until) = seat.until {
                    out.push((seat.from.as_second(), until.as_second()));
                }
            }
        }
    }
    out
}

/// The rotation caps on a service roster, stated once for both kinds that have them:
///
/// - **Term limit.** Every seat must set `valid_until`, and the window may not exceed
///   `max_term_seconds`. No open-ended standing — a permanent corps is exactly the
///   capture the rotation rule prevents.
/// - **Rotation gap.** A re-nomination must begin at least `rotation_period_seconds`
///   after the most recent *prior* term that had already ended at or before this
///   window's start: the servant sits out.
/// - **Continuous-service union.** The per-window term limit is not enough on its
///   own: overlapping re-nominations (a new window opened before the prior one ends)
///   never trip the rotation-gap rule, so a community could chain them to keep one
///   servant seated indefinitely. Each window is therefore merged with every prior
///   window from which it is not separated by a full rotation sit-out (overlap or a
///   sub-`rotation_period` gap counts as one continuous term); the merged span
///   containing this window may not exceed `max_term_seconds`.
/// - **Pool-share.** Once the compact-wide active roster reaches `pool_floor`, no
///   single nominating community may hold a majority of it. Below the floor a small
///   compact may staff it freely (`liminal_institutions.md` "The mediator pool": for
///   small compacts every community must contribute).
fn check_roster_caps(
    archive: &InMemoryArchive,
    kind: RosterKind,
    community: &DidKey,
    issued_at: Timestamp,
    seats: &[Seat<'_>],
    self_cid: &Cid,
    limits: RosterLimits,
) -> KcResult<()> {
    let RosterLimits { pool_floor, max_term_seconds, rotation_period_seconds } = limits;
    let (role, section) = (kind.role(), kind.section());
    let rosters = rosters_of(archive, kind);

    for seat in seats {
        let valid_until = seat.until.ok_or_else(|| {
            KcError::Authority(format!(
                "{role} {} has an open-ended term; a bounded valid_until is required \
                 ({section} rotation)",
                seat.did
            ))
        })?;
        let term = valid_until.as_second() - seat.from.as_second();
        if term > max_term_seconds {
            return Err(KcError::Authority(format!(
                "{role} {} term {}s exceeds the maximum {}s ({section} term limit)",
                seat.did, term, max_term_seconds
            )));
        }
        let prior_windows = prior_service_windows(&rosters, seat.did, self_cid);
        let from = seat.from.as_second();
        let until = valid_until.as_second();

        // Rotation gap: the latest prior term that ended at or before this window's
        // start. A re-nomination must sit out a full rotation period after it.
        let prior_end = prior_windows
            .iter()
            .filter(|&&(_, pu)| pu <= from)
            .map(|&(_, pu)| pu)
            .max();
        if let Some(pe) = prior_end {
            let gap = from - pe;
            if gap < rotation_period_seconds {
                return Err(KcError::Authority(format!(
                    "{role} {} re-nominated after a {}s gap, below the rotation period \
                     {}s ({section} rotation)",
                    seat.did, gap, rotation_period_seconds
                )));
            }
        }

        // Continuous-service union: merge this window with every prior window not
        // separated from the chain by a full rotation sit-out, then bound the merged
        // span that contains this window by the term limit. This catches
        // overlap-extension, which the rotation-gap rule (ended terms only) cannot see.
        let mut windows = prior_windows;
        windows.push((from, until));
        windows.sort_unstable();
        let (mut cs, mut ce) = windows[0];
        for &(s, e) in windows.iter().skip(1) {
            if s - ce < rotation_period_seconds {
                ce = ce.max(e); // connected: extend the continuous-service component
            } else if cs <= from && until <= ce {
                break; // this window's component closed before reaching it
            } else {
                cs = s; // genuine sit-out: start a new component
                ce = e;
            }
        }
        if cs <= from && until <= ce {
            let span = ce - cs;
            if span > max_term_seconds {
                return Err(KcError::Authority(format!(
                    "{role} {} continuous service {}s (overlapping windows unioned) \
                     exceeds the maximum {}s ({section} term limit)",
                    seat.did, span, max_term_seconds
                )));
            }
        }
    }

    let shares = active_roster_shares(&rosters, issued_at);
    let total: usize = shares.values().sum();
    let this_share = shares.get(community.as_str()).copied().unwrap_or(0);
    if total >= pool_floor && 2 * this_share > total {
        return Err(KcError::Authority(format!(
            "community {} would hold {}/{} of the active {role} pool, a majority \
             ({section} pool-share)",
            community.as_str(),
            this_share,
            total
        )));
    }
    Ok(())
}

/// §8 observer-registry caps (capture resistance), enforced when a roster is
/// admitted. The four rules — bounded term, rotation gap, continuous-service union,
/// no-majority pool share — are stated in [`check_roster_caps`], which §2's examining
/// pool shares rather than copies. Only the policy numbers are §8's own.
fn authorize_observer_registry(
    archive: &InMemoryArchive,
    reg: &ObserverRegistry,
    self_cid: &Cid,
    caps: ObserverCaps,
) -> KcResult<()> {
    require_office(
        archive,
        &reg.nominating_community,
        Capability::ObserverSeat,
        reg.issued_at,
        "nominating observers",
    )?;
    let seats: Vec<Seat> = reg
        .entries
        .iter()
        .map(|en| Seat {
            did: en.observer_did.as_str(),
            from: en.valid_from,
            until: en.valid_until,
        })
        .collect();
    check_roster_caps(
        archive,
        RosterKind::Observer,
        &reg.nominating_community,
        reg.issued_at,
        &seats,
        self_cid,
        caps.limits(),
    )
}

/// §2 examining-pool caps: the same four rules as the §8 observer corps, applied to
/// the communities that conduct anti-spiral-feature credentialing.
///
/// `capture_resistance.md` §7: "The examiners who conduct the anti-spiral-feature
/// examination are drawn by lot from member communities, serve fixed terms, publish
/// their reasoning, and rotate out, with cross-community composition requirements. A
/// standing admissions cadre would be the single most capturable body in the compact,
/// for the same reasons a standing mediator class would be."
///
/// Drawing by lot is not enforced (see `kc_ceremony::inter::examiner`); fixed terms,
/// rotation, and composition are.
fn authorize_examiner_registry(
    archive: &InMemoryArchive,
    reg: &ExaminerRegistry,
    self_cid: &Cid,
    caps: ExaminerCaps,
) -> KcResult<()> {
    // §2's first withholding reaches the examining pool too: seating examiners is
    // a compact function, and a probationary community does not staff it.
    require_office(
        archive,
        &reg.nominating_community,
        Capability::ExaminerSeat,
        reg.issued_at,
        "seating examiners",
    )?;
    if probationary_at(archive, &reg.nominating_community, reg.issued_at) {
        return Err(KcError::Authority(format!(
            "community {} is probationary at {}; a probationary community does not \
             staff the examining pool (§2)",
            reg.nominating_community.as_str(),
            reg.issued_at
        )));
    }
    let seats: Vec<Seat> = reg
        .entries
        .iter()
        .map(|en| Seat {
            did: en.examiner_did.as_str(),
            from: en.valid_from,
            until: en.valid_until,
        })
        .collect();
    check_roster_caps(
        archive,
        RosterKind::Examiner,
        &reg.nominating_community,
        reg.issued_at,
        &seats,
        self_cid,
        caps.limits(),
    )
}

/// The community that seated `examiner_did`, if it holds an active examining seat at
/// `at`. The §2 twin of [`mediator_home`].
fn examiner_home(archive: &InMemoryArchive, examiner_did: &str, at: Timestamp) -> Option<DidKey> {
    for r in rosters_of(archive, RosterKind::Examiner) {
        for seat in &r.seats {
            if seat.did == examiner_did && seat.active_at(at) {
                return Some(r.community.clone());
            }
        }
    }
    None
}

/// Whether a hold is in effect on `event_cid` as of `at`, and how long it runs.
///
/// Two things can place an event under the structured-skepticism protocol, and §4 needs
/// both:
///
/// - the originating community's own declaration on the [`EventReport`], which is a
///   voluntary submission and carries no capture risk; and
/// - a [`SkepticismInvocation`] by the mediator pool, which is the doctrine's actual
///   assignment of the invocation and the only thing that reaches an event whose
///   originator declared it mundane.
///
/// Either one places the hold, and the window is the longer of them, since a pool that
/// has weighed the event's interpretive charge has said something the default cannot.
///
/// A `Pretextual` review releases the event from **either** kind of hold. That is the
/// point of reviewing the hold rather than the invocation record: the self-declared
/// hold was the one that answered to nobody, and it is reachable here.
fn skepticism_hold_until(
    archive: &InMemoryArchive,
    event_cid: &Cid,
    event: &EventReport,
    at: Timestamp,
) -> Option<i64> {
    let mut released = false;
    let mut window = if event.qualifies() {
        Some(DEFAULT_STRUCTURED_SKEPTICISM_HOLD_SECONDS)
    } else {
        None
    };
    for (_, e) in archive.iter() {
        match e {
            ArchiveEntry::SkepticismInvoked(s) if &s.payload.event_cid == event_cid => {
                if s.payload.invoked_at <= at {
                    window = Some(window.unwrap_or(0).max(s.payload.hold_seconds));
                }
            }
            ArchiveEntry::SkepticismReviewed(s) if &s.payload.event_cid == event_cid => {
                if s.payload.reviewed_at <= at
                    && s.payload.answer == kc_provenance::ReviewAnswer::Pretextual
                {
                    released = true;
                }
            }
            _ => {}
        }
    }
    if released {
        return None;
    }
    window.map(|w| event.reported_at.as_second() + w)
}

/// Move-4 structured-skepticism hold: a formal action premised on an event under the
/// protocol is refused while the event is still within its hold window **and**
/// under-corroborated. Any one of — no hold is in effect, its window has elapsed, or
/// enough distinct communities have corroborated it — clears the action. Corroborators
/// are counted as distinct communities other than the originator (no self-corroboration),
/// as of the action's `invoked_at`.
///
/// What places the hold is [`skepticism_hold_until`]. It used to be the originator's
/// four self-declared booleans alone, which made the protocol's teeth a matter for the
/// party the protocol constrains.
fn authorize_event_action(archive: &InMemoryArchive, action: &EventActionInvoked) -> KcResult<()> {
    let event = match archive.get(&action.event_cid) {
        Some(ArchiveEntry::EventReported(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "event action references {} which is not a reported event",
                action.event_cid
            )))
        }
    };
    let hold_until =
        match skepticism_hold_until(archive, &action.event_cid, event, action.invoked_at) {
            None => return Ok(()),
            Some(h) => h,
        };
    let mut corroborators: BTreeSet<String> = BTreeSet::new();
    for (_, e) in archive.iter() {
        if let ArchiveEntry::EventCorroborated(s) = e {
            let c = &s.payload;
            if c.event_cid == action.event_cid
                && c.corroborated_at <= action.invoked_at
                && c.corroborating_community.as_str() != event.originator.as_str()
            {
                corroborators.insert(c.corroborating_community.as_str().to_string());
            }
        }
    }
    let within_hold = action.invoked_at.as_second() < hold_until;
    if within_hold && corroborators.len() < DEFAULT_STRUCTURED_SKEPTICISM_MIN_CORROBORATORS {
        return Err(KcError::Authority(format!(
            "action on qualifying event {} invoked within the structured-skepticism hold (until {}s) with {} of {} required corroborations (move 4)",
            action.event_cid,
            hold_until,
            corroborators.len(),
            DEFAULT_STRUCTURED_SKEPTICISM_MIN_CORROBORATORS
        )));
    }
    Ok(())
}

/// The event an invocation or review refers to, or an authority error naming what the
/// CID actually resolved to.
fn referenced_event<'a>(
    archive: &'a InMemoryArchive,
    event_cid: &Cid,
    what: &str,
) -> KcResult<&'a EventReport> {
    match archive.get(event_cid) {
        Some(ArchiveEntry::EventReported(s)) => Ok(&s.payload),
        _ => Err(KcError::Authority(format!(
            "{what} references {event_cid} which is not a reported event"
        ))),
    }
}

/// §4: the mediator pool's authority to invoke the structured-skepticism protocol.
///
/// The panel is derived from the co-signers, must be `panel_min` active registered
/// mediators, and may not include one from the **originating community**: placing the
/// pool's hold on one's own event is what the self-declared triggers already are, and
/// routing it through the pool instead would launder a community's own act into the
/// compact's.
///
/// The criteria and the grounds are both required, because the doctrine asks the review
/// to be able to tell a genuine invocation from a pretextual one and "the criteria must
/// be documented specifically". The hold is bounded at both ends: a nominal hold is no
/// protocol, and an unbounded one is suppression.
///
/// The event's declared `aligned_faction` is deliberately **not** barred. That field is
/// the originator's own declaration, so barring it would let an originator name a rival
/// and disqualify that rival's mediators from ever invoking against it.
fn authorize_skepticism_invocation(
    archive: &InMemoryArchive,
    inv: &kc_provenance::SkepticismInvocation,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let event = referenced_event(archive, &inv.event_cid, "skepticism invocation")?;
    let claimed: BTreeSet<&str> = inv.panel.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "a skepticism invocation's panel must be exactly its co-signers; it names {} \
             and {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    if inv.criteria.is_empty() {
        return Err(KcError::Authority(
            "a skepticism invocation must name the criteria it finds met: an invocation \
             on no stated criterion is the pretextual use the pool's review exists to catch"
                .into(),
        ));
    }
    if inv.grounds.trim().is_empty() {
        return Err(KcError::Authority(
            "a skepticism invocation must document its grounds specifically (§4)".into(),
        ));
    }
    if inv.hold_seconds < MIN_INVOKED_SKEPTICISM_HOLD_SECONDS
        || inv.hold_seconds > MAX_INVOKED_SKEPTICISM_HOLD_SECONDS
    {
        return Err(KcError::Authority(format!(
            "an invoked hold of {}s is outside the range the protocol calibrates to \
             [{}s, {}s]: a nominal hold is no delay at all and an unbounded one is \
             suppression",
            inv.hold_seconds,
            MIN_INVOKED_SKEPTICISM_HOLD_SECONDS,
            MAX_INVOKED_SKEPTICISM_HOLD_SECONDS
        )));
    }
    let barred: BTreeSet<String> = [event.originator.as_str().to_string()].into_iter().collect();
    check_mediator_panel(
        archive,
        signers,
        inv.invoked_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "invoking mediator",
            barred_kind: "the originating community",
            label: "invoking panel",
        },
    )
}

/// §4: the mediator pool's internal review of a hold, against pretextual use.
///
/// Two communities are barred from the reviewing panel, for opposite reasons. The
/// **originator** benefits from release, so its mediators may not vote on it. And every
/// mediator who **co-signed an invocation on this event** is barred, because a panel
/// that reviews its own invocation answers the question the review was raised to ask.
/// Without that second bar the record would exist and audit nothing: the invoking panel
/// would simply uphold itself.
///
/// The reasoning is required on both answers. An audit that is legible only when the
/// hold is struck down tells a reader nothing about the holds that stood.
fn authorize_skepticism_review(
    archive: &InMemoryArchive,
    rev: &kc_provenance::SkepticismReview,
    signers: &[&str],
    panel_min: usize,
) -> KcResult<()> {
    let event = referenced_event(archive, &rev.event_cid, "skepticism review")?;
    let claimed: BTreeSet<&str> = rev.reviewers.iter().map(|d| d.as_str()).collect();
    let actual: BTreeSet<&str> = signers.iter().copied().collect();
    if claimed != actual {
        return Err(KcError::Authority(format!(
            "a skepticism review's reviewers must be exactly its co-signers; it names {} \
             and {} signed",
            claimed.len(),
            actual.len()
        )));
    }
    if rev.reasoning.trim().is_empty() {
        return Err(KcError::Authority(
            "a skepticism review must state its reasoning on either answer: an audit \
             legible only when the hold falls tells a reader nothing about the holds \
             that stood"
                .into(),
        ));
    }
    // Every mediator who invoked on this event is disqualified from reviewing it.
    let mut invokers: BTreeSet<String> = BTreeSet::new();
    for (_, e) in archive.iter() {
        if let ArchiveEntry::SkepticismInvoked(s) = e {
            if s.payload.event_cid == rev.event_cid {
                for m in &s.payload.panel {
                    invokers.insert(m.as_str().to_string());
                }
            }
        }
    }
    if let Some(sitting) = signers.iter().find(|sg| invokers.contains(**sg)) {
        return Err(KcError::Authority(format!(
            "{sitting} co-signed the invocation under review, so may not review it"
        )));
    }
    let barred: BTreeSet<String> = [event.originator.as_str().to_string()].into_iter().collect();
    check_mediator_panel(
        archive,
        signers,
        rev.reviewed_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "reviewing mediator",
            barred_kind: "the originating community",
            label: "reviewing panel",
        },
    )
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
/// mediators — counting weight, so a probationary home does not receive a
/// filing (§2).
fn authorize_reception_panel(
    archive: &InMemoryArchive,
    accused: &DidKey,
    signers: &[&str],
    at: Timestamp,
    panel_min: usize,
    what: &str,
) -> KcResult<()> {
    // A reception panel acts before any hearing, on a filing nobody may answer yet, so
    // it is an emergency panel in the sense that matters here.
    require_panel_office(
        archive,
        signers,
        at,
        Capability::EmergencyPanelSeat,
        &format!("sitting on a {what} panel"),
    )?;
    let barred: BTreeSet<String> = [accused.as_str().to_string()].into_iter().collect();
    check_mediator_panel(
        archive,
        signers,
        at,
        &barred,
        panel_min,
        PanelSpec {
            role: &format!("{what} panelist"),
            barred_kind: "the accused community",
            label: &format!("{what} panel"),
        },
    )
}

/// The keys the archive can resolve as **insiders** of `community` at `at`: its root
/// key, every active speaker delegation it has issued, and every member named on any
/// roll it has published.
///
/// Public because a source assembling a filing must use the *same* definition the
/// authority pass will check it against; two implementations of "the insider set"
/// would refuse every honest filing. A source calls this against the archive as it
/// stands, which is exactly the prefix its own filing will be checked against — see
/// [`insider_keys_upto`] for why that matters.
///
/// This is the ring a whistleblower proves membership in. Delegations follow the same
/// supersession rule as `resolve_did` — the record with the greatest `issued_at <= at`
/// wins, so an appended revocation removes a key from the ring. **Member rolls do
/// not supersede**: they are unioned, so a community cannot narrow the suspect set
/// around a source by publishing a shorter roll.
pub fn insider_keys(
    archive: &InMemoryArchive,
    community: &DidKey,
    at: Timestamp,
) -> Vec<(DidKey, kc_core::PubKey)> {
    insider_keys_upto(archive, community, at, None)
}

/// [`insider_keys`], bounded to the records appearing **before `stop_at`** in
/// insertion order.
///
/// The bound is what makes the insider set a property of the log rather than of when
/// it was read. Without it the accused community — the one party with a motive —
/// invalidates any filing against it by appending a single delegation backdated
/// before `filed_at` (the recomputed set no longer equals the offered ring), and
/// narrows the suspect set around a source by backdating a revocation. Both were
/// measured before this rule existed. Insertion order is authoritative for the same
/// reason it is in `current_rung`: authorizing an earlier record must never "see" a
/// later one.
pub fn insider_keys_upto(
    archive: &InMemoryArchive,
    community: &DidKey,
    at: Timestamp,
    stop_at: Option<&Cid>,
) -> Vec<(DidKey, kc_core::PubKey)> {
    let stop = stop_at.map(|c| c.to_string());
    let prefix = || {
        archive.iter().take_while(|(cid, _)| match &stop {
            Some(s) => cid.to_string() != *s,
            None => true,
        })
    };

    let mut out: Vec<(DidKey, kc_core::PubKey)> = Vec::new();
    for (_, e) in prefix() {
        if let ArchiveEntry::CommunityIdentityPublished(s) = e {
            if &s.payload.did == community {
                out.push((s.payload.did.clone(), s.payload.root_pubkey.clone()));
            }
        }
    }
    // Latest delegation per speaker, then keep the ones live at `at`.
    let mut latest: BTreeMap<String, &kc_identity::SpeakerDelegation> = BTreeMap::new();
    for (_, e) in prefix() {
        if let ArchiveEntry::SpeakerDelegated(s) = e {
            let d = &s.payload;
            if &d.community_did != community || d.issued_at > at {
                continue;
            }
            let key = d.speaker_did.as_str().to_string();
            match latest.get(&key) {
                Some(prev)
                    if (prev.issued_at, prev.revoked_at.is_some())
                        >= (d.issued_at, d.revoked_at.is_some()) => {}
                _ => {
                    latest.insert(key, d);
                }
            }
        }
    }
    for d in latest.values() {
        if d.active_at(at) {
            out.push((d.speaker_did.clone(), d.speaker_pubkey.clone()));
        }
    }
    // Every member ever rolled, unioned. No supersession: a later roll that drops a
    // name does not remove that name from the ring. Once named an insider, always
    // one — a departed member keeps the channel they had, and the accused gains no
    // way to shrink the set the day after somebody leaves.
    for (_, e) in prefix() {
        if let ArchiveEntry::MemberRollPublished(s) = e {
            let r = &s.payload;
            if &r.community_did != community || r.published_at > at {
                continue;
            }
            for m in &r.members {
                out.push((m.member_did.clone(), m.member_pubkey.clone()));
            }
        }
    }
    // One key, one ring position. A member may also hold a delegation, and two rolls
    // may name the same member; a duplicated ring position would be a ring signature
    // over a repeated key, which no rule downstream expects.
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    out.retain(|(_, pk)| seen.insert(pk.bytes.clone()));
    out
}

/// §8 member-roll rule: a community's statement of its own composition. Principal
/// binding (root only — see `allowed_scopes`) already establishes *who* may publish
/// one; these are the constraints on what it may say.
fn authorize_member_roll(roll: &kc_identity::MemberRoll) -> KcResult<()> {
    if roll.members.is_empty() {
        return Err(KcError::Authority(
            "a member roll must name at least one member".into(),
        ));
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for m in &roll.members {
        if !seen.insert(m.member_did.as_str()) {
            return Err(KcError::Authority(format!(
                "member roll names {} twice",
                m.member_did
            )));
        }
        // The carried key must be the one the DID resolves to. Otherwise a roll
        // names a real member against a foreign key, putting an outsider in the ring
        // under that member's name — and the ring is what bounds a reveal.
        match m.member_did.to_pubkey() {
            Ok(pk) if pk.bytes == m.member_pubkey.bytes => {}
            Ok(_) => {
                return Err(KcError::Authority(format!(
                    "member roll gives {} a key its DID does not resolve to",
                    m.member_did
                )))
            }
            Err(_) => {
                return Err(KcError::Authority(format!(
                    "member roll names {}, whose key cannot be resolved from its DID",
                    m.member_did
                )))
            }
        }
        if m.member_did == roll.community_did {
            return Err(KcError::Authority(
                "a member roll may not name the community itself as a member: its root \
                 key is already an insider, and a second entry would be a repeated ring \
                 position"
                    .into(),
            ));
        }
    }
    Ok(())
}

/// §8 whistleblower rule: a non-party mediator reception panel of at least
/// `panel_min` vouches the report, **and** the filing carries a source of its own.
///
/// The second half is the one that matters. Without it the archive holds no evidence
/// an insider exists at all — the signers are the panel, so a panel could file
/// manufactured insider complaints against a rival community and both integrity
/// passes would pass them. Here the source's ring signature must verify against the
/// accused community's own resolvable keys, and the attestation must be bound to
/// *this* accusation: same accused, same commitment, same instant, and a digest of the
/// claim text, so an attestation cannot be lifted off one claim and stapled to
/// another. The custody is checked for shape at the same time, because a sharing that
/// could never be recovered is a source protected by loss rather than by threshold.
fn authorize_whistleblower(
    archive: &InMemoryArchive,
    wb: &WhistleblowerReport,
    signers: &[&str],
    panel_min: usize,
    self_cid: &Cid,
) -> KcResult<()> {
    authorize_reception_panel(
        archive,
        &wb.accused_community,
        signers,
        wb.filed_at,
        panel_min,
        "whistleblower reception",
    )?;

    // The attestation is internally valid and closes over the ring it names.
    wb.insider
        .verify()
        .map_err(|e| KcError::Authority(format!("whistleblower insider attestation: {e}")))?;

    // The ring is exactly the accused community's insider key set, as the log stood
    // when this filing was appended. Reading the current tip instead would let the
    // accused break its own accusations after the fact.
    let insiders = insider_keys_upto(archive, &wb.accused_community, wb.filed_at, Some(self_cid));
    if insiders.len() < 2 {
        return Err(KcError::Authority(format!(
            "community {} has {} resolvable keys, too few for an anonymous filing (a ring of one names its signer)",
            wb.accused_community,
            insiders.len()
        )));
    }
    let expected: BTreeSet<Vec<u8>> = insiders.iter().map(|(_, pk)| pk.bytes.clone()).collect();
    let offered: BTreeSet<Vec<u8>> = wb.insider.ring.iter().map(|pk| pk.bytes.clone()).collect();
    if offered != expected {
        return Err(KcError::Authority(format!(
            "whistleblower ring is not {}'s insider key set ({} offered, {} resolvable)",
            wb.accused_community,
            offered.len(),
            expected.len()
        )));
    }

    // The attestation is bound to this filing, not merely to some filing.
    let claim = &wb.insider.payload;
    if claim.accused_community != wb.accused_community {
        return Err(KcError::Authority(
            "whistleblower attestation names a different accused community".into(),
        ));
    }
    if claim.source_commitment != wb.source_commitment {
        return Err(KcError::Authority(
            "whistleblower attestation commits to a different source".into(),
        ));
    }
    if claim.filed_at != wb.filed_at {
        return Err(KcError::Authority(
            "whistleblower attestation was made for a different filing instant".into(),
        ));
    }
    let digest = WhistleblowerReport::claim_digest(&wb.claim)
        .map_err(|e| KcError::Authority(format!("claim digest: {e}")))?;
    if claim.claim_digest != digest {
        return Err(KcError::Authority(
            "whistleblower attestation was made for a different claim".into(),
        ));
    }

    // Custody: recoverable in principle, and held by exactly the panel that vouched.
    wb.custody
        .check_shape()
        .map_err(|e| KcError::Authority(format!("whistleblower source custody: {e}")))?;
    let holders: BTreeSet<String> =
        wb.custody.holders().iter().map(|d| d.as_str().to_string()).collect();
    let panel: BTreeSet<String> =
        wb.reception_panel.iter().map(|d| d.as_str().to_string()).collect();
    if holders != panel {
        return Err(KcError::Authority(format!(
            "source custody is held by {} keys but the reception panel is {}",
            holders.len(),
            panel.len()
        )));
    }
    // The threshold must be a **majority** of the custodians, which is the precise
    // reading of "structurally stronger than individual mediator integrity": no
    // minority of the panel can reconstruct the source, whatever it agrees among
    // itself. Requiring `panel_min` instead would be wrong — for a panel of exactly
    // `panel_min` that is unanimity, so one unavailable panelist would make the source
    // permanently unnameable. The separate rule that a reveal be co-signed by
    // `panel_min` non-party mediators still applies, so both gates hold at once.
    let k = usize::from(wb.custody.threshold_k);
    if 2 * k <= panel.len() {
        return Err(KcError::Authority(format!(
            "source custody threshold {k} is not a majority of the {} custodians, so a minority could name the source",
            panel.len()
        )));
    }
    // Each share must be sealed to the key its named holder is registered under.
    //
    // Naming the panel is not the same as sealing to it. Without this rule a filing
    // could name exactly the reception panel and seal every share to one outsider's
    // key: `check_shape` passed, the holder set matched, and **no** named custodian
    // could open anything — so the reveal path was dead for good with nothing on the
    // record to show it, and one actor held the whole secret. Judged at `filed_at`, so
    // a later re-registration cannot retroactively invalidate an honest filing.
    let registered = mediator_encryption_keys(archive, wb.filed_at);
    for share in &wb.custody.sealed_shares {
        let holder = share.holder_did.as_str();
        match registered.get(holder) {
            Some(pk) if pk.bytes == share.recipient_pubkey.0 => {}
            Some(_) => {
                return Err(KcError::Authority(format!(
                    "the source custody share for {holder} is sealed to a key that is not its \
                     registered encryption key, so that custodian cannot open it"
                )))
            }
            None => {
                return Err(KcError::Authority(format!(
                    "the source custody share for {holder} names a holder with no registered \
                     encryption key at {}",
                    wb.filed_at
                )))
            }
        }
    }
    Ok(())
}

/// §8 source-reveal rule: references a filed whistleblower report, cites a
/// non-empty cause, is authorized by a non-party mediator panel of at least
/// `panel_min` — and **actually opens the filing's commitment**.
///
/// The last clause is what makes the quorum gate mean anything. Before it, `cause` and
/// the panel were checked and `disclosed_source` was not: the commitment on the filing
/// was read by no rule anywhere, so a panel could append any name it liked and the
/// archive would carry it as a disclosure. Now the reveal must present enough opened
/// custody shares to reconstruct the identity, every share is checked against the
/// filing's published commitments, the recovered preimage must hash to the filing's
/// `source_commitment`, and `disclosed_source` must be what was recovered. Naming
/// someone therefore requires the shares — and the someone can only be a member of the
/// ring the source proved membership in.
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
    )?;

    let preimage = wb
        .custody
        .recover_preimage(&rev.shares, &wb.source_commitment)
        .map_err(|e| {
            KcError::Authority(format!(
                "source reveal does not open the filing's commitment: {e}"
            ))
        })?;
    if rev.disclosed_source != preimage.identity {
        return Err(KcError::Authority(
            "source reveal discloses a name the custody shares do not recover".into(),
        ));
    }
    // The disclosed source must be someone the source could have been — judged against
    // the filing's own insider set, bounded by the filing's position in the log, not by
    // whatever the accused has appended since.
    let ring_dids: BTreeSet<String> =
        insider_keys_upto(archive, &wb.accused_community, wb.filed_at, Some(&rev.report_cid))
            .iter()
            .map(|(did, _)| did.as_str().to_string())
            .collect();
    if !ring_dids.contains(&preimage.identity) {
        return Err(KcError::Authority(format!(
            "source reveal names {}, who is not an insider of {}",
            preimage.identity, wb.accused_community
        )));
    }
    Ok(())
}

/// §8 exculpation rule: a member proves a filing's key image is not theirs.
///
/// Unlike every other §8 record this needs no panel — that is the point. A colluding
/// quorum can still name the wrong ring member, and the wronged member's remedy must
/// not run through the body that wronged them. What the archive checks is only that
/// the claim is honest arithmetic: the claimant's key resolves, the proof ties that key
/// to the image being claimed, and the image differs from the one on the filing. An
/// "exculpation" whose image *matches* the filing is a confession, and is refused
/// rather than recorded as a denial.
fn authorize_source_exculpation(
    archive: &InMemoryArchive,
    exc: &kc_observer::SourceExculpation,
    signers: &[&str],
) -> KcResult<()> {
    let wb = match archive.get(&exc.report_cid) {
        Some(ArchiveEntry::WhistleblowerFiled(s)) => &s.payload,
        _ => {
            return Err(KcError::Authority(format!(
                "source exculpation references {} which is not a whistleblower report",
                exc.report_cid
            )))
        }
    };
    if !signers.contains(&exc.claimant_did.as_str()) {
        return Err(KcError::Authority(format!(
            "source exculpation must be signed by its claimant {}",
            exc.claimant_did
        )));
    }
    // Resolve the claimant among the filing's own insider set: being in the ring is
    // both what makes the claimant implicated and where their key comes from. Bounded
    // by the filing's position, so a wrongly-named member's remedy cannot be closed off
    // by the accused appending a record afterwards.
    let insiders =
        insider_keys_upto(archive, &wb.accused_community, wb.filed_at, Some(&exc.report_cid));
    let pubkey = insiders
        .iter()
        .find(|(did, pk)| {
            did == &exc.claimant_did && wb.insider.ring.iter().any(|r| r.bytes == pk.bytes)
        })
        .map(|(_, pk)| pk.clone())
        .ok_or_else(|| {
            KcError::Authority(format!(
                "source exculpation claimant {} is not in the filing's ring, so the filing never implicated them",
                exc.claimant_did
            ))
        })?;
    exc.verify_proof(&pubkey)
        .map_err(|e| KcError::Authority(format!("source exculpation proof: {e}")))?;
    if &exc.claimed_key_image == wb.source_tag() {
        return Err(KcError::Authority(format!(
            "source exculpation by {} reproduces the filing's own key image: it is a confession, not a denial",
            exc.claimant_did
        )));
    }
    Ok(())
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
            // Counting *weight*, not signatures (§2). A bar that counted signatures
            // while the generic quorum counted weight would be the easier of the two
            // whenever a faction had probationary entrants to spare: the ordinary
            // quorum met by standing members, the higher bar topped up with weight
            // the doctrine withholds. Measured — that is exactly what happened.
            let tally =
                deliberative_weight(archive, a.deciders.iter(), a.decided_at, None, None);
            if tally.counted < quorum_threshold + 1 {
                return Err(KcError::Authority(format!(
                    "contracting the compact-incapable list needs >= {} distinct communities, got {}{}",
                    quorum_threshold + 1,
                    tally.counted,
                    tally.because().unwrap_or_default()
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
        // Weight, not signatures — see the compact-incapable contraction bar (§2).
        let tally = deliberative_weight(archive, a.deciders.iter(), a.decided_at, None, None);
        if tally.counted < quorum_threshold + 1 {
            return Err(KcError::Authority(format!(
                "contracting the structured-fast cadence needs >= {} distinct communities, got {}{}",
                quorum_threshold + 1,
                tally.counted,
                tally.because().unwrap_or_default()
            )));
        }
    }
    Ok(())
}

/// The forbearance keys in the adopted minimum content as of `self_cid` (exclusive):
/// the adoption's own forbearances plus every `Broaden` minus every `Narrow` among
/// the amendments preceding it in insertion order.
///
/// No genesis-original set is computed here, and that absence is the rule. The
/// compact-incapable list keeps its founding matters immutable; the minimum content
/// does not distinguish a founding forbearance from a later one, because the
/// protection the doctrine asks for is the asymmetric bar and not a lock
/// (natural_law.md "Interaction with the deference instruments").
fn current_forbearance_keys(archive: &InMemoryArchive, self_cid: &Cid) -> BTreeSet<String> {
    let self_key = self_cid.to_string();
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            break;
        }
        match e {
            ArchiveEntry::MinimumContentAdopted(s) => {
                for f in &s.payload.forbearances {
                    keys.insert(f.key.clone());
                }
            }
            ArchiveEntry::MinimumContentAmended(s) => match s.payload.change {
                ForbearanceChange::Broaden => {
                    keys.insert(s.payload.forbearance.key.clone());
                }
                ForbearanceChange::Narrow => {
                    keys.remove(&s.payload.forbearance.key);
                }
            },
            _ => {}
        }
    }
    keys
}

/// True if an adoption of the minimum content exists at or before `self_cid` in
/// insertion order.
fn adoption_present_before(archive: &InMemoryArchive, self_cid: &Cid) -> bool {
    let self_key = self_cid.to_string();
    for (cid, e) in archive.iter() {
        if cid.to_string() == self_key {
            return false;
        }
        if matches!(e, ArchiveEntry::MinimumContentAdopted(_)) {
            return true;
        }
    }
    false
}

/// §9 minimum-content adoption rule: the adoption is a **singleton** — a second
/// `MinimumContentAdopted` is refused, and it must name at least one forbearance.
/// The distinct-community quorum on the adopters is enforced generically.
///
/// Deliberately independent of `ConstitutionRatified`. The compact's ground at its
/// boundary is not a matter belonging to a member, so it does not wait on the
/// artifact that lists those matters; requiring a prior genesis would make the ground
/// hostage to an unrelated ratification.
fn authorize_minimum_adoption(archive: &InMemoryArchive, self_cid: &Cid) -> KcResult<()> {
    if adoption_present_before(archive, self_cid) {
        return Err(KcError::Authority(
            "the minimum content is already adopted; adoption is a singleton".into(),
        ));
    }
    match archive.get(self_cid) {
        Some(ArchiveEntry::MinimumContentAdopted(s)) if s.payload.forbearances.is_empty() => {
            Err(KcError::Authority(
                "an adoption of the minimum content names no forbearance".into(),
            ))
        }
        _ => Ok(()),
    }
}

/// §9 minimum-content amendment rule (self-binding, asymmetric): requires a prior
/// adoption; a `Broaden` must name a forbearance not already in force; a `Narrow`
/// must name one that is in force and carries the higher contraction bar
/// (>= `quorum_threshold + 1` distinct deciders). The normal quorum is enforced
/// generically before this runs.
///
/// The one structural difference from [`authorize_incapable_amendment`]: there is no
/// genesis-immutability branch. A founding forbearance is contractible at the higher
/// bar like any other, which is what the doctrine specifies — the argument for
/// narrowing what is owed at the frontier is "always available and always pragmatic",
/// so it gets a bar, not a refusal.
fn authorize_minimum_amendment(
    archive: &InMemoryArchive,
    a: &MinimumContentAmendment,
    self_cid: &Cid,
    quorum_threshold: usize,
) -> KcResult<()> {
    if !adoption_present_before(archive, self_cid) {
        return Err(KcError::Authority(
            "minimum-content amendment before the minimum content is adopted".into(),
        ));
    }
    let current = current_forbearance_keys(archive, self_cid);
    match a.change {
        ForbearanceChange::Broaden => {
            if current.contains(&a.forbearance.key) {
                return Err(KcError::Authority(format!(
                    "forbearance {:?} is already in force",
                    a.forbearance.key
                )));
            }
        }
        ForbearanceChange::Narrow => {
            if !current.contains(&a.forbearance.key) {
                return Err(KcError::Authority(format!(
                    "forbearance {:?} is not in force, cannot narrow",
                    a.forbearance.key
                )));
            }
            // Weight, not signatures — see the compact-incapable contraction bar (§2).
            let tally = deliberative_weight(archive, a.deciders.iter(), a.decided_at, None, None);
            if tally.counted < quorum_threshold + 1 {
                return Err(KcError::Authority(format!(
                    "narrowing the minimum content needs >= {} distinct communities, got {}{}",
                    quorum_threshold + 1,
                    tally.counted,
                    tally.because().unwrap_or_default()
                )));
            }
        }
    }
    Ok(())
}

/// Refuse a panel when any co-signer's home community is held out of `capability`.
/// Signers whose home does not resolve are left to the panel rule itself, which
/// refuses them for not being active registered mediators.
fn require_panel_office(
    archive: &InMemoryArchive,
    signers: &[&str],
    at: Timestamp,
    capability: Capability,
    act: &str,
) -> KcResult<()> {
    for signer in signers {
        if let Some(home) = mediator_home(archive, signer, at) {
            require_office(archive, &home, capability, at, act)?;
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
    require_panel_office(
        archive,
        signers,
        inj.issued_at,
        Capability::EmergencyPanelSeat,
        "issuing an interim injunction",
    )?;
    let barred = injunction_barred_communities(archive, &inj.subject_community, inj.dispute_cid.as_ref());
    check_mediator_panel(
        archive,
        signers,
        inj.issued_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "injunction panelist",
            barred_kind: "a party community",
            label: "injunction panel",
        },
    )
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
    // The different-panel safeguard runs first, and on every signer: sitting twice is
    // a fault of the challenge's composition, not a want of weight, so it is refused
    // before anything is counted.
    for signer in signers {
        if issuing.contains(signer) {
            return Err(KcError::Authority(format!(
                "challenge panelist {signer} also sat on the issuing panel (different-panel safeguard violated)"
            )));
        }
    }
    check_mediator_panel(
        archive,
        signers,
        chal.decided_at,
        &barred,
        panel_min,
        PanelSpec {
            role: "challenge panelist",
            barred_kind: "a party community",
            label: "challenge panel",
        },
    )
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
        if !injunction_in_force(archive, cid, inj, at) {
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

/// Is the injunction at `cid` in force as of `at` — issued by then, not overturned by
/// a ruled challenge, and either inside its window or confirmed before it lapsed?
///
/// One reading, shared by every rule that asks. `kc_reputation`'s projection derives
/// the same status for the dashboard; a third copy here, per caller, is how two rules
/// come to disagree about whether a member is under an injunction.
fn injunction_in_force(
    archive: &InMemoryArchive,
    cid: &Cid,
    inj: &InterimInjunction,
    at: Timestamp,
) -> bool {
    if inj.issued_at.as_second() > at.as_second() {
        return false;
    }
    let overturned = archive.iter().any(|(_, e)| match e {
        ArchiveEntry::InjunctionChallenged(s) => {
            s.payload.injunction_cid == *cid
                && s.payload.overturned
                && s.payload.decided_at.as_second() <= at.as_second()
        }
        _ => false,
    });
    if overturned {
        return false;
    }
    // Confirmation keeps an injunction in force past its window, but only once it has
    // happened: a confirmation dated after `at` says nothing about the compact's state
    // at `at`.
    let confirmed = archive.iter().any(|(_, e)| match e {
        ArchiveEntry::InjunctionConfirmed(s) => {
            s.payload.injunction_cid == *cid
                && s.payload.confirmed_at.as_second() <= at.as_second()
                && s.payload.confirmed_at.as_second() <= inj.expires_at.as_second()
        }
        _ => false,
    });
    at.as_second() <= inj.expires_at.as_second() || confirmed
}

/// Is `capability` withheld from `community` by an injunction in force as of `at`?
///
/// The archive-side twin of `kc_reputation::has_capability`, which this crate cannot
/// call (the dependency runs the other way). An in-force injunction is the only thing
/// that withholds a capability, so this is the whole trigger: covenant-facing by
/// construction, and never a reading of a member's interior.
fn capability_revoked(
    archive: &InMemoryArchive,
    community: &DidKey,
    capability: Capability,
    at: Timestamp,
) -> bool {
    for (cid, e) in archive.iter() {
        let inj = match e {
            ArchiveEntry::InjunctionIssued(s) if &s.payload.subject_community == community => {
                &s.payload
            }
            _ => continue,
        };
        if inj.revoked_capabilities.contains(&capability)
            && injunction_in_force(archive, cid, inj, at)
        {
            return true;
        }
    }
    false
}

/// Refuse `act` when `community` is held out of `capability` by an in-force injunction.
/// One message shape for all five offices, so a refusal always names the office, the
/// act it blocked, and the community held out.
fn require_office(
    archive: &InMemoryArchive,
    community: &DidKey,
    capability: Capability,
    at: Timestamp,
    act: &str,
) -> KcResult<()> {
    if capability_revoked(archive, community, capability, at) {
        return Err(KcError::Authority(format!(
            "community {} is held out of the {} office by an injunction in force at {}; \
             {} is refused (its services are untouched)",
            community.as_str(),
            capability.as_str(),
            at,
            act
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

/// Check one record's authority with the default roster caps. `Ok(())` if authorized;
/// `KcError::Authority` naming the first violation otherwise. See
/// [`authorize_with_caps`] to supply custom §8 observer, §2 examining or §4 mediator
/// limits.
pub fn authorize(
    archive: &InMemoryArchive,
    entry: &ArchiveEntry,
    quorum_threshold: usize,
    panel_min: usize,
) -> KcResult<()> {
    authorize_with_caps(
        archive,
        entry,
        quorum_threshold,
        panel_min,
        ObserverCaps::default(),
        ExaminerCaps::default(),
        MediatorCaps::default(),
    )
}

/// Check one record's authority, gating each registry record with its roster's caps
/// (`observer_caps` for §8, `examiner_caps` for §2, `mediator_caps` for §4 — all three
/// enforced by the one [`check_roster_caps`]). `Ok(())` if authorized;
/// `KcError::Authority` naming the first violation otherwise.
pub fn authorize_with_caps(
    archive: &InMemoryArchive,
    entry: &ArchiveEntry,
    quorum_threshold: usize,
    panel_min: usize,
    observer_caps: ObserverCaps,
    examiner_caps: ExaminerCaps,
    mediator_caps: MediatorCaps,
) -> KcResult<()> {
    // Mediation decisions are authorized by the panel rule, not principal binding.
    if let ArchiveEntry::MediationDecided(s) = entry {
        let signers = signer_dids(entry);
        let self_cid = entry.cid()?;
        return authorize_decision(archive, &s.payload, &signers, panel_min, &self_cid);
    }
    // Opening a seal is authorized by the custody it opens, not by principal binding.
    //
    // These two dispatch **here**, before the generic pass, and not in the structural
    // match further down. `record_authority` returns `None` for both (no community
    // speaks for them), and a `None` there returns `Ok(())` immediately — so a rule
    // placed in that match would exist, compile, and never run. It happened: the usage
    // rule sat there authorizing everything until the refusal tests caught it.
    if let ArchiveEntry::DecisionUnsealed(s) = entry {
        let signers = signer_dids(entry);
        return authorize_decision_unsealed(archive, &s.payload, &signers);
    }
    if let ArchiveEntry::UsageOpened(s) = entry {
        let signers = signer_dids(entry);
        return authorize_usage_opened(archive, &s.payload, &signers);
    }
    // An argument is authorized by its own role rule (§4), not principal binding: a
    // panelist is a person in the pool rather than a community.
    if let ArchiveEntry::ArgumentFiled(s) = entry {
        let signers = signer_dids(entry);
        return authorize_argument(archive, &s.payload, &signers);
    }
    // §4.2's three records are each authorized by their own rule.
    if let ArchiveEntry::RecusalRequested(s) = entry {
        let signers = signer_dids(entry);
        return authorize_recusal_request(archive, &s.payload, &signers);
    }
    if let ArchiveEntry::RecusalDecided(s) = entry {
        let signers = signer_dids(entry);
        return authorize_recusal_decision(archive, &s.payload, &signers);
    }
    if let ArchiveEntry::ProtectingPowerDesignated(s) = entry {
        let signers = signer_dids(entry);
        return authorize_protecting_power(archive, &s.payload, &signers);
    }
    if let ArchiveEntry::PanelConsentRecorded(s) = entry {
        let signers = signer_dids(entry);
        return authorize_panel_consent(archive, &s.payload, &signers);
    }
    // Move 4's pool side: an invocation and its review are panel acts, not any one
    // community's, so they are authorized by their panel rules rather than by
    // principal binding.
    if let ArchiveEntry::SkepticismInvoked(s) = entry {
        let signers = signer_dids(entry);
        return authorize_skepticism_invocation(archive, &s.payload, &signers, panel_min);
    }
    if let ArchiveEntry::SkepticismReviewed(s) = entry {
        let signers = signer_dids(entry);
        return authorize_skepticism_review(archive, &s.payload, &signers, panel_min);
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
        let self_cid = entry.cid()?;
        return authorize_whistleblower(archive, &s.payload, &signers, panel_min, &self_cid);
    }
    if let ArchiveEntry::SourceRevealed(s) = entry {
        let signers = signer_dids(entry);
        return authorize_source_reveal(archive, &s.payload, &signers, panel_min);
    }
    // An exculpation is authorized by its own arithmetic, not by any panel: the
    // remedy for being wrongly named must not depend on the body that named you.
    if let ArchiveEntry::SourceExculpated(s) = entry {
        let signers = signer_dids(entry);
        return authorize_source_exculpation(archive, &s.payload, &signers);
    }

    let auth = match record_authority(entry) {
        Some(a) => a,
        None => return Ok(()),
    };
    let allowed = allowed_scopes(entry);
    let signers = signer_dids(entry);

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
    }

    // §2 probation de-weighting, in **every** domain where a multi-community quorum
    // is what validates the act: a probationary community's principal binding still
    // holds — it may speak — but it carries no deliberative weight toward the quorum.
    // Principal binding above is deliberately unchanged; only the count below moves.
    //
    // The gate is `second_class`, not a list of record types. `capture_resistance.md`
    // §7 enumerates admissions, mediator selection and designation procedures, but its
    // reason is general — *"otherwise each admitted entrant immediately amplifies the
    // faction that admitted it"* — and second-class is precisely the class of acts
    // whose cost is compact-wide. Stating the rule as the class rather than as a list
    // is also what keeps it true: a second-class record added later inherits the
    // withholding instead of quietly escaping it.
    if auth.second_class {
        // Exclude this record from the window scan: a re-entry opens the
        // petitioner's own probation window at its `completed_at`, and a record may
        // not be de-weighted by a window it itself opens.
        //
        // *This exclusion is currently shadowed here* and no test can isolate it: the
        // only record that opens a window for one of its own principals is a re-entry,
        // whose petitioner the `subject` rule below already drops before the probation
        // check runs. Measured — mutating it away fails nothing. It is kept because it
        // is the helper's contract rather than this caller's convenience: a
        // second-class record added later that opens a window for a principal it does
        // *not* name as its subject would otherwise refuse itself, which is the defect
        // §2 found the hard way. Stated rather than left as an untestable rule.
        let self_cid = entry.cid()?;
        // The subject of a readmission does not carry weight toward it. "No single
        // sponsor cluster can admit alone" (`capture_resistance.md` §7) is emptiest
        // when the cluster is the returning community itself.
        let subject = match entry {
            ArchiveEntry::ReEntryCompleted(s) => Some(&s.payload.petitioner),
            _ => None,
        };
        let tally = deliberative_weight(
            archive,
            auth.principals.iter(),
            auth.at,
            Some(&self_cid),
            subject,
        );
        if tally.counted < quorum_threshold {
            // Name the reason precisely. A shortfall of signatures, a shortfall caused
            // by probation, and a shortfall caused by the subject not counting toward
            // its own readmission are three different problems with three different
            // fixes, and the operator has to be able to tell which one this is.
            return Err(KcError::Authority(format!(
                "second-class decision requires >= {quorum_threshold} distinct \
                 communities, got {}{}",
                tally.counted,
                tally.because().unwrap_or_default()
            )));
        }
    }

    // Component-specific structural rules (§5), after principal binding + quorum.
    match entry {
        // §2 admissions integrity.
        ArchiveEntry::InterCommunityWitness(s) => {
            let self_cid = entry.cid()?;
            authorize_inter_witness(
                archive,
                &s.payload,
                &signers,
                &self_cid,
                quorum_threshold,
                examiner_caps,
            )?;
        }
        ArchiveEntry::SponsorshipVouched(s) => {
            authorize_sponsorship(archive, &s.payload)?;
        }
        ArchiveEntry::MediatorRegistryUpdated(s) => {
            let self_cid = entry.cid()?;
            authorize_mediator_registry(archive, &s.payload, &self_cid, mediator_caps)?;
        }
        ArchiveEntry::LadderAdvanced(s) => {
            let self_cid = entry.cid()?;
            authorize_transition(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        ArchiveEntry::DesignationDecided(s) => {
            authorize_designation(archive, &s.payload, quorum_threshold)?;
        }
        ArchiveEntry::ReexaminationConcluded(s) => {
            authorize_reexamination(archive, &s.payload)?;
        }
        // The gate's second site. `LadderTransition` cites a dispute rather than a
        // designation, so it has nothing to hold; the re-engagement rung is the other
        // escalation act that cites evidence, and a designation is citable there.
        ArchiveEntry::SubstantiveReEngagementInvoked(s) => {
            no_stale_designation_cited(
                archive,
                s.payload.decay_evidence.iter(),
                s.payload.invoked_at,
            )?;
        }
        ArchiveEntry::ActivityAttested(s) => {
            authorize_activity_attestation(archive, &s.payload)?;
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
        ArchiveEntry::CommonsEscalated(s) => {
            authorize_commons_escalation(archive, &s.payload, &entry.cid()?)?;
        }
        ArchiveEntry::CompactChartered(s) => {
            authorize_compact_charter(archive, &s.payload, &signer_dids(entry))?;
        }
        ArchiveEntry::SanctionApplied(s) => {
            let self_cid = entry.cid()?;
            authorize_commons_sanction(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        ArchiveEntry::ObserverRegistryUpdated(s) => {
            let self_cid = entry.cid()?;
            authorize_observer_registry(archive, &s.payload, &self_cid, observer_caps)?;
        }
        ArchiveEntry::ExaminerRegistryUpdated(s) => {
            let self_cid = entry.cid()?;
            authorize_examiner_registry(archive, &s.payload, &self_cid, examiner_caps)?;
        }
        ArchiveEntry::MemberRollPublished(s) => {
            authorize_member_roll(&s.payload)?;
        }
        ArchiveEntry::ReportResponsePublished(s) => {
            authorize_report_response(archive, &s.payload)?;
        }
        ArchiveEntry::EventActionInvoked(s) => {
            authorize_event_action(archive, &s.payload)?;
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
        ArchiveEntry::MinimumContentAdopted(_) => {
            let self_cid = entry.cid()?;
            authorize_minimum_adoption(archive, &self_cid)?;
        }
        ArchiveEntry::MinimumContentAmended(s) => {
            let self_cid = entry.cid()?;
            authorize_minimum_amendment(archive, &s.payload, &self_cid, quorum_threshold)?;
        }
        _ => {}
    }
    Ok(())
}

/// Check authority for every record in the archive, fail-fast. Pair with
/// `verify_all` (crypto integrity); this pass assumes signatures already verify.
/// `quorum_threshold` gates second-class decisions; `panel_min` gates mediation
/// panels (§4); the three caps types gate their rosters' limits (pool-share, term,
/// rotation, continuous service).
pub fn check_authority(
    archive: &InMemoryArchive,
    quorum_threshold: usize,
    panel_min: usize,
    observer_caps: ObserverCaps,
    examiner_caps: ExaminerCaps,
    mediator_caps: MediatorCaps,
) -> KcResult<()> {
    for (_, entry) in archive.iter() {
        authorize_with_caps(
            archive,
            entry,
            quorum_threshold,
            panel_min,
            observer_caps,
            examiner_caps,
            mediator_caps,
        )?;
    }
    Ok(())
}

/// Every authority violation in the archive, as `(cid, reason)`, rather than
/// failing on the first.
pub fn authority_violations(
    archive: &InMemoryArchive,
    quorum_threshold: usize,
    panel_min: usize,
    observer_caps: ObserverCaps,
    examiner_caps: ExaminerCaps,
    mediator_caps: MediatorCaps,
) -> Vec<(String, String)> {
    let mut out = vec![];
    for (cid, entry) in archive.iter() {
        if let Err(e) = authorize_with_caps(
            archive,
            entry,
            quorum_threshold,
            panel_min,
            observer_caps,
            examiner_caps,
            mediator_caps,
        ) {
            out.push((cid.to_string(), e.to_string()));
        }
    }
    out
}
