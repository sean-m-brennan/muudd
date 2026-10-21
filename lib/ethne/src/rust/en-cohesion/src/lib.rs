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

//! `en-cohesion` — the polity's **coherence monitor** (a pure read-model).
//!
//! A chartered polity can erode: members drift out of the observed boundary,
//! earned reputation decays, the edge goes opaque, the humans answerable for its
//! machines stop being present, the substrate stops being re-affirmed by the
//! members who carry it, the human a chartered machine answers to is no longer
//! there at all — and, as *trends* rather than states, the substrate carried less
//! and less often, fewer turning out to receive new members, the boundary redrawn
//! faster and faster. This crate reads that strain
//! off two snapshots — the **chartered roll** (from `en-charter`'s
//! `charter_state`) and the **current candidate** the substrate provider surfaces
//! (`SubstrateProvider::founding_candidate`) — and grades it into a
//! [`CohesionReport`]. It adds no trait method and no archive entry: it is a lens
//! over state the other crates already produce, so it stays `kc-core`-only and
//! depends on no particular provider.
//!
//! # Ratify, do not replace (design open question Q2)
//!
//! The monitor **does not decide a fission**. Algorithmically splitting a polity
//! would be exactly the "replace emergence" move the tier exists to refuse. What
//! the monitor does is raise a graded [`ErosionLevel`] — up to
//! [`ErosionLevel::Concern`], an *early warning* — which the upper tier
//! ([`en-uplift`](../en_uplift/index.html)) can surface to kith-covenant as a
//! Concern. A fission is a **deliberate re-charter**: the community ratifies two
//! successor foundings, and only then is the predecessor dissolved. `Concern` may
//! name the strain that precedes a ratified split, but the split itself is
//! ratified, never auto-verdicted.
//!
//! The three erosion levels mirror kith-covenant's own graded erosion alert
//! (`None` / `Notice` / `Alert`), one tier down.

use std::collections::{BTreeMap, BTreeSet};

use en_charter::{
    AffirmationKind, AffirmationState, DecayTrends, GroundStanding, GuardianState,
    IndependentObservation, ObservationWindow, Rite,
};
use en_core::{DidKey, Timestamp};
use en_substrate::FoundingCandidate;

/// How strained a polity's cohesion is, graded. Ordered `None < Notice <
/// Concern` (the derived order follows declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ErosionLevel {
    /// Cohesion within normal bounds.
    #[default]
    None,
    /// Early strain worth watching; not yet a Concern.
    Notice,
    /// Strain the upper tier should surface as a kith-covenant Concern.
    Concern,
}

/// The cut-points that turn raw cohesion measures into an [`ErosionLevel`]. Each
/// `*_notice` bound raises at least a Notice; each `*_concern` bound raises a
/// Concern. See [`Thresholds::default`] for the v1 values.
#[derive(Debug, Clone, PartialEq)]
pub struct Thresholds {
    /// A member whose earned reputation is below this counts as a floor breach.
    pub reputation_floor: f64,
    /// Attrition (fraction of the chartered roll no longer observed) at or above
    /// this raises a Notice; at or above `attrition_concern`, a Concern.
    pub attrition_notice: f64,
    pub attrition_concern: f64,
    /// Mean observed reputation at or below this raises a Notice; at or below
    /// `mean_reputation_concern`, a Concern.
    pub mean_reputation_notice: f64,
    pub mean_reputation_concern: f64,
    /// Fraction of observed members in floor breach at or above this raises a
    /// Notice; at or above `breach_concern`, a Concern.
    pub breach_notice: f64,
    pub breach_concern: f64,
    /// How long a guardian's verified presence still counts as *present*. An
    /// attendance stamp older than this (or absent) makes the member unattended.
    /// A cadence, not a cut-point: it says what "now" means for a signal whose
    /// whole content is freshness.
    pub attendance_cadence_seconds: i64,
    /// Fraction of guardian-bearing members currently unattended at or above
    /// which a Notice is raised.
    ///
    /// There is deliberately **no `unattended_concern` counterpart.** Absence of
    /// humans at consoles is ambiguous by nature — a night shift, a weekend and
    /// an abandoned polity look identical in a point-in-time snapshot — so *this*
    /// component is capped at Notice by construction and cannot be tuned into a
    /// Concern. Left escalatable, a fleet whose operators simply went home would
    /// cross to kith-covenant every night.
    ///
    /// **The escape hatch this comment used to promise now exists** (D16): where the
    /// polity's monitors have *recorded* what they saw, `sustained_absence_seconds`
    /// and `sustained_absence_watchers` grade the same absence over an interval, and
    /// that measure may escalate — because a duration is exactly what a snapshot
    /// lacked. The cap here stands unchanged: a snapshot never escalates.
    pub unattended_notice: f64,
    /// Fraction of observed members the provider has **no reputation reading** for,
    /// at or above which a Notice is raised.
    ///
    /// As with `unattended_notice` there is deliberately **no `unrated_concern`
    /// counterpart** — this component is capped at Notice by construction (D18). A
    /// cohort too young to have been rated and a provider that has stopped rating
    /// look identical from a snapshot, so the reading may warn and must not
    /// escalate. Its job is to keep the two reputation components from going
    /// *silently* dark: they report nothing when nobody is rated, and something has
    /// to say that they are not reporting.
    ///
    /// Half the observed roll unrated, matching the attendance share's looseness:
    /// being unrated is a normal early state, not a fault.
    pub unrated_notice: f64,
    /// How long a member's affirmation of the substrate stays current. Past this,
    /// their transmission has lapsed (`en_charter::AffirmationState::stale_members`
    /// owns the rule; this is the cadence handed to it).
    pub affirmation_cadence_seconds: i64,
    /// Fraction of members whose affirmation has lapsed at or above which a Notice
    /// is raised; at or above `affirmation_stale_concern`, a Concern.
    ///
    /// Unlike the attendance component this one **does** escalate, and the
    /// difference is not a policy preference: duration is already inside the
    /// measure. A guardian away from the console might be at lunch, but a member
    /// who has not re-affirmed the substrate in a whole cadence is not — the
    /// transmission that keeps a polity a polity has actually lapsed.
    pub affirmation_stale_notice: f64,
    pub affirmation_stale_concern: f64,
    /// Fraction of chartered guardian edges the substrate no longer shows, at or
    /// above which a Notice is raised; at or above `guardian_lost_concern`, a
    /// Concern.
    ///
    /// This one escalates too, and for a reason unlike either of the others: it
    /// compares what the polity **ratified** against what is **observed**, and only
    /// over members the provider still sees. A member who has vanished entirely is
    /// attrition, never a lost edge — so an opaque boundary cannot masquerade as
    /// guardian loss, and no cap is needed. What remains is a credential that has
    /// stopped verifying for a member in plain view: a machine the charter says
    /// answers to a human, which no longer does.
    pub guardian_lost_notice: f64,
    pub guardian_lost_concern: f64,
    /// The window over which roll churn is counted, and the rate (changes per
    /// member over that window) at which a Notice, then a Concern, is raised.
    /// Unlike the two trend booleans below, churn is a continuous measure, so it
    /// gets a full pair.
    pub churn_window_seconds: i64,
    pub churn_notice: f64,
    pub churn_concern: f64,
    /// How long a guardian must have been continuously unattended, across recorded
    /// observations, before the absence is *sustained* rather than merely current.
    /// A day by default: long enough to have outlasted any shift.
    pub sustained_absence_seconds: i64,
    /// How many **distinct observers** must have seen that member across the span.
    ///
    /// **This is not "N absences".** The span is the absence; the count is the
    /// evidence that somebody was *watching* while it lasted. A span computed from a
    /// single stale reading with a long silence behind it says nothing about the
    /// polity — only that nobody looked. The same span witnessed repeatedly does.
    ///
    /// **Watchers, not filings** (design **D28**). Until this was measured the
    /// component read `readings_for`, so one key filing three times satisfied a floor
    /// whose whole justification was "somebody was watching" — and a stranger with
    /// one keypair could manufacture the `Concern` that crosses to kith-covenant.
    /// Counting distinct observers is arithmetic, not a judgement about who they are;
    /// whether they are independent *of each other* is the separate lens
    /// `observer_independence` grades, and deliberately not a gate.
    pub sustained_absence_watchers: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            reputation_floor: 0.3,
            attrition_notice: 0.2,
            attrition_concern: 0.4,
            mean_reputation_notice: 0.5,
            mean_reputation_concern: 0.35,
            breach_notice: 0.2,
            breach_concern: 0.4,
            // Five minutes: long enough that a guardian who stepped away from the
            // keyboard for a moment still counts as present, short enough that
            // "attended" keeps meaning attended *now*. AT's own attestation is
            // pulled on demand and accepted within a two-minute window, so this
            // sits comfortably above the pull's own tolerance.
            attendance_cadence_seconds: 300,
            // Half the guardian-bearing members unattended before it is worth
            // noting. Unattended is a normal state, not a fault, so this is
            // deliberately looser than the reputation-breach share.
            unattended_notice: 0.5,
            unrated_notice: 0.5,
            // 180 days — the re-affirmation cadence the design and the demo
            // already use for this lens standalone.
            affirmation_cadence_seconds: 180 * 24 * 60 * 60,
            // A quarter of the roll lapsed is worth noting; half of it means the
            // substrate is no longer being carried by the people who hold it.
            affirmation_stale_notice: 0.25,
            affirmation_stale_concern: 0.5,
            guardian_lost_notice: 0.25,
            guardian_lost_concern: 0.5,
            // Thirty days, and a roll that turns over a fifth of itself inside one
            // is worth noticing; half of itself, worth a Concern.
            churn_window_seconds: 30 * 86_400,
            churn_notice: 0.2,
            churn_concern: 0.5,
            sustained_absence_seconds: 86_400,
            sustained_absence_watchers: 3,
        }
    }
}

/// A graded read of a polity's cohesion at one moment. Every field is derived;
/// nothing here changes state.
#[derive(Debug, Clone, PartialEq)]
pub struct CohesionReport {
    /// Fraction of the chartered roll no longer present in the observed
    /// candidate, in `[0.0, 1.0]`.
    pub attrition: f64,
    /// How many still-observed members are below the reputation floor.
    pub reputation_floor_breaches: usize,
    /// Mean earned reputation over the still-observed chartered members (`0.0`
    /// when none remain observed).
    pub mean_reputation: f64,
    /// Whether the observed boundary is (partly) an intentional opaque edge. When
    /// true, apparent attrition is ambiguous — a member may have gone private
    /// rather than left — so attrition alone is not allowed to escalate past a
    /// Notice.
    pub opaque_boundary: bool,
    /// Chartered members still present in the candidate.
    pub observed: usize,
    /// How many of those the provider actually **rated** — the denominator of both
    /// reputation components (D18).
    ///
    /// Reported in its own right because the difference `observed - rated_members`
    /// is the unrated count, and because a *fall* here is the provider's own reach
    /// eroding, which no ratio over it could show. Zero means the reputation
    /// components said nothing rather than reporting health.
    pub rated_members: usize,
    /// Size of the chartered roll.
    pub chartered: usize,
    /// How many still-observed chartered members carry a guardian edge at all —
    /// the denominator of the attendance measure. Reported in its own right
    /// because a *fall* here is the guardian edge itself eroding, which no ratio
    /// over it could show.
    pub guardian_members: usize,
    /// How many of those guardian-bearing members have nobody verified present:
    /// their attendance stamp is older than the cadence, or there is none.
    pub unattended: usize,
    /// How many chartered members the supplied affirmation state actually covers —
    /// the denominator of `stale_affirmations`, and `0` when no transmission
    /// reading was supplied at all (in which case the component says nothing
    /// rather than reporting false health).
    pub affirming_members: usize,
    /// How many of those have let their affirmation lapse: never inducted, or last
    /// affirmed longer ago than the cadence.
    pub stale_affirmations: usize,
    /// How many chartered guardian edges are checkable here: members the charter
    /// designates derived **with** a guardian who are still observed. `0` when no
    /// guardian reading was supplied, in which case the component says nothing.
    pub chartered_guardian_members: usize,
    /// How many of those the provider no longer sees carrying a guardian at all —
    /// the charter says a human answers for this machine, and the substrate
    /// disagrees.
    pub guardian_edges_lost: usize,
    /// Whether the polity's latest affirmation gap exceeds the median of its prior
    /// gaps — the substrate being carried *less often*. `None` with fewer than two
    /// gaps to compare, which is not the same as `Some(false)`.
    ///
    /// A *rolling* reading: the median rises with the gaps, so this one asks whether
    /// the newest gap is unusual for the polity as it now behaves.
    pub latest_gap_widening: Option<bool>,
    /// Whether the latest affirmation gap exceeds the **first** one — the same
    /// question asked against a baseline the drift cannot move.
    ///
    /// `latest_gap_widening` goes quiet once the widened cadence has recurred often
    /// enough to dominate the median of the priors — it falls silent exactly when
    /// the decay has finished setting. This component is here because a polity's own
    /// recent behaviour is the one reference a uniform drift is guaranteed to have
    /// already shifted.
    pub gap_widening_since_founding: Option<bool>,
    /// Whether the most recent induction drew fewer witnesses than the first — the
    /// community turning out less to receive its new members. `false` with fewer
    /// than two inductions, so no trend is invented from one.
    pub witness_count_declined: bool,
    /// Roll changes per member across `churn_window_seconds` — how fast the
    /// boundary is being redrawn. `None` when unmeasured.
    pub churn_rate: Option<f64>,
    /// Chartered members whose guardian has been unattended across the recorded
    /// window for at least `sustained_absence_seconds`, seen by at least
    /// `sustained_absence_watchers` **distinct observers** (D28). Unlike the snapshot attendance
    /// count, this one escalates: nobody has been there all day, and we were
    /// watching.
    pub sustained_absences: usize,
    /// **How much of the watching behind this report anyone has vouched for**
    /// (design **D28**) — distinct monitors seen in the window, and how many of them
    /// carry a disinterested independence attestation.
    ///
    /// A **lens, never a gate**, and the asymmetry is deliberate in a way worth
    /// stating: the erosion level above does not move by one step on account of these
    /// numbers. Gating an escalation on the monitor being vouched for would hand a
    /// captured polity the simplest possible defence — vouch for nobody, and become
    /// unwatchable — which is a worse hole than the one being reported. And a monitor
    /// nobody has vouched for is the normal state of an honest new one, so refusing it
    /// would be refusing newness, the line **D24** drew for readers.
    ///
    /// What it buys is that a reader of this report can tell a `Concern` raised by
    /// three independent monitors from one raised by three keys, where before D28 the
    /// second could not even be distinguished from *one key filing three times*.
    /// `None` when no `observers` reading was supplied.
    pub watchers: Option<WatcherStanding>,
    /// How many of the polity's rites this reading covers — the denominator of the
    /// three measures below, and `0` when no rite reading was supplied (in which case
    /// they say nothing rather than reporting health).
    pub rites: usize,
    /// Whether the polity's most recent completed rite ran **shorter** than its first
    /// — the ceremony costing less than it used to. `None` with fewer than two
    /// completed rites, which is not the same as `Some(false)`.
    ///
    /// This is the measure v1 could not take at all: an induction carried a single
    /// instant, so a rite held over a week and one performed in a second left
    /// identical records. Capped at `Notice` like the other trends — a cheapening
    /// ceremony is a leading indicator, and what it leads to is already gradeable.
    pub rite_span_shortening: Option<bool>,

    /// The same measure for **renewals** (design **D34**): whether the polity's most
    /// recent completed re-recitation ran shorter than its first. `None` with fewer than
    /// two, which is not `Some(false)`.
    ///
    /// Kept separate from `rite_span_shortening` because an induction and a renewal are
    /// different ceremonies: one series would compare a member's reception against
    /// another's renewal and read the difference as drift. This is also the measure that
    /// did not exist before D34, and the one that matters most over a polity's life —
    /// induction happens once per member, the renewal recurs forever, and it was the
    /// recurring one that carried no cost at all.
    pub renewal_span_shortening: Option<bool>,
    /// Rites begun and never completed, as of this reading. Not graded: nothing in the
    /// doctrine says how long a catechumenate may take, so an open rite is reported
    /// and left to a reader who knows the polity's own cadence.
    pub rites_abandoned: usize,
    /// Attendees lost between a rite's first movement and its last, summed over
    /// completed rites — the polity that turned out for the naming of the substrate
    /// and had drifted away by the submission of the polity itself.
    pub rite_attrition: usize,
    /// Whether **nobody** on the roll holds the substrate. Reached only by a polity
    /// expelling its way there, since the founding seeds its ratifiers — and terminal
    /// while it lasts, because every majority is of the members who hold it. Graded
    /// straight to `Concern`, event-style, with no threshold: like a ground's
    /// withdrawal this is a state, not a drift.
    pub nobody_holds_the_substrate: bool,
    /// Whether the polity's chartered substrate anchor has withdrawn (design
    /// **D23**). `false` both when no ground reading was supplied and when the ground
    /// still stands, which is safe *here* only because the alternative — a polity
    /// with no anchor at all — cannot be disavowed in the first place.
    ///
    /// Not a ratio, and not comparable to the rest of this report: it is one signed
    /// act by the one key entitled to perform it.
    pub ground_disavowed: bool,
    /// **Which optional readings this grade was not given** (design **D31**), by the
    /// field names of [`CohesionInputs`], in that order.
    ///
    /// Every optional input here reports *nothing* rather than health when it is
    /// absent, which is right for a library and useless to a reader who cannot tell
    /// silence from health. Until this field existed the only thing standing between a
    /// forgetful caller and a false clean bill was having gone through `en-host`, whose
    /// `HostCycle::grade` assembles all of them. A caller off that path inherited the
    /// hazard whole, and the Python side had no `en-host` at all.
    ///
    /// So the answer travels with the grade rather than with the call site. It is the
    /// same move **D28** made for `unvouched`: absence made visible instead of read as
    /// presence. And it is a **lens, never a gate** — a partial grade stays legitimate,
    /// because a young polity has held no rites and a caller may deliberately want one
    /// component. What changes is that the report can say so.
    ///
    /// See [`silent_measures`](Self::silent_measures) for what went quiet, and
    /// [`blind_to_concern`](Self::blind_to_concern) for whether any of it could have
    /// escalated.
    pub omitted_readings: Vec<&'static str>,
    /// The graded erosion this report carries.
    pub erosion: ErosionLevel,
}

impl CohesionReport {
    /// The measures that said nothing because their reading was absent (**D31**).
    ///
    /// Derived from [`omitted_readings`](Self::omitted_readings) over a fixed table, so
    /// a reader gets the consequence rather than the cause: `guardians` missing is a
    /// fact about the call, `guardian_edges_lost` reporting nothing is a fact about the
    /// grade, and it is the second one that matters when the erosion reads `None`.
    pub fn silent_measures(&self) -> Vec<&'static str> {
        self.omitted_readings.iter().flat_map(|input| silent_for(input)).copied().collect()
    }

    /// Whether any measure that **can reach `Concern`** was silent (**D31**).
    ///
    /// `erosion == ErosionLevel::None && blind_to_concern()` is the honest reading of
    /// *no news, and I was not looking* — a distinction no other field here can draw,
    /// since a component with nothing to say and a component with nothing to say it
    /// *from* both contribute [`ErosionLevel::None`] to the maximum.
    ///
    /// Which inputs qualify is a property of the levels below, not a judgement:
    /// `guardians`, `affirmations`, `trends` and `window` each carry a component with a
    /// `Concern` step, and `ground` reaches `Concern` alone. `rites` is capped at
    /// `Notice`, and `observers` feeds the `watchers` lens that no level reads, so
    /// neither of those blinds this report to anything it could have escalated on.
    pub fn blind_to_concern(&self) -> bool {
        self.omitted_readings.iter().any(|input| CONCERN_CAPABLE_INPUTS.contains(input))
    }
}

/// The inputs carrying at least one component with a `Concern` step. See
/// [`CohesionReport::blind_to_concern`].
const CONCERN_CAPABLE_INPUTS: [&str; 5] =
    ["guardians", "affirmations", "trends", "window", "ground"];

/// What goes quiet when a given input is absent. The other half of
/// [`CohesionReport::silent_measures`], kept beside the constant above so the two
/// tables are read together.
fn silent_for(input: &str) -> &'static [&'static str] {
    match input {
        "guardians" => &["guardian_edges_lost"],
        // Two of them, and the second is why an absent transmission reading is worse
        // than it looks: a polity where nobody holds the substrate can take no
        // second-class act of its own, and this is the only thing that says so.
        "affirmations" => &["stale_affirmations", "nobody_holds_the_substrate"],
        "trends" => &["gap_widening", "witness_count_declined", "churn_rate"],
        // The window is the only input that is a memory rather than a state, so losing
        // it costs both the one absence measure permitted to escalate and D28's lens
        // over who was watching while it lasted.
        "window" => &["sustained_absences", "watchers"],
        "ground" => &["ground_disavowed"],
        "rites" => &["rite_span_shortening", "renewal_span_shortening", "rite_attrition"],
        "observers" => &["watchers.vouched", "watchers.contested"],
        _ => &[],
    }
}

fn grade(value: f64, notice: f64, concern: f64) -> ErosionLevel {
    if value >= concern {
        ErosionLevel::Concern
    } else if value >= notice {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    }
}

/// Grade a *low-is-bad* measure (reputation): at or below `concern` → Concern, at
/// or below `notice` → Notice.
fn grade_low(value: f64, notice: f64, concern: f64) -> ErosionLevel {
    if value <= concern {
        ErosionLevel::Concern
    } else if value <= notice {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    }
}

/// Whether a guardian counts as present: a stamp within `cadence_seconds` of
/// `as_of`. No stamp at all is not attended — "never seen" and "seen too long
/// ago" are the same answer to the question being asked. Delta arithmetic in
/// whole seconds, matching `en_charter::AffirmationState::stale_members`.
fn is_attended(attested_at: Option<Timestamp>, as_of: Timestamp, cadence_seconds: i64) -> bool {
    match attested_at {
        Some(at) => as_of.as_second() - at.as_second() <= cadence_seconds,
        None => false,
    }
}

/// The charter-side read-models a report may consult, each optional because each
/// answers a question that may simply not have been asked yet.
///
/// **`None` means *not evaluated*, never *healthy*.** That distinction is the whole
/// reason these are `Option`s rather than defaulted values: a component with no
/// reading reports nothing, where a component reading an empty state would report
/// zero-of-zero and look like health. Bundling them also keeps the report's
/// signature honest — four adjacent references of different types are transposable
/// in a way named fields are not.
///
/// Every field is derived from the polity's own signed log
/// (`en_charter::{guardian_state, affirmation_state, decay_trends}`), never from a
/// provider: the provider's side of the comparison is the `candidate`.
/// **Who was watching, and who will say so** (design **D28**) — the standing of the
/// monitors whose observations a report's duration measures rest on.
///
/// Three numbers rather than a grade, because there is no defensible ordering of
/// them: a polity watched by one vouched-for monitor and one watched by six unvouched
/// ones are differently legible, not differently healthy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WatcherStanding {
    /// Distinct observers in the window. **Distinct** is the whole point: before D28
    /// this was a count of filings, so one key filing three times cleared a floor
    /// whose justification was that somebody had been watching.
    pub distinct: usize,
    /// How many of them carry a disinterested independence attestation — somebody
    /// outside the polity, outside the roll, and not the monitor itself, willing to be
    /// named for the claim that this monitor is its own principal.
    pub vouched: usize,
    /// How many of them somebody has claimed are *controlled*. Counted from anyone,
    /// including a founder: interest disqualifies praise, not a warning (**D24**).
    pub contested: usize,
}

impl WatcherStanding {
    /// Watchers nobody has vouched for. Not a fault — the ordinary state of an honest
    /// new monitor, and of every monitor in a polity whose neighbours write no
    /// attestations.
    pub fn unvouched(&self) -> usize {
        self.distinct.saturating_sub(self.vouched)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CohesionInputs<'a> {
    /// Which members are derived (machine) parties and whom they answer to (D15).
    pub guardians: Option<&'a GuardianState>,
    /// Who is still actively re-affirming the substrate (D13).
    pub affirmations: Option<&'a AffirmationState>,
    /// The polity's decay trends — affirmation cadence, witness turnout, roll churn
    /// (D16).
    pub trends: Option<&'a DecayTrends>,
    /// What the polity's monitors recorded seeing, over an interval (D16). The one
    /// input that is not a state but a memory, and the only one that can turn an
    /// ambiguous snapshot reading into a duration.
    pub window: Option<&'a ObservationWindow>,
    /// Whether the polity's chartered ground has withdrawn (D23). `None` when no
    /// reading was supplied, in which case the component says nothing rather than
    /// reporting false health — the same discipline every other optional input here
    /// follows.
    pub ground: Option<&'a GroundStanding>,
    /// The polity's rites — movements, spans, attendance (D29). `None` when no rite
    /// reading was supplied, in which case the rite measures say nothing.
    pub rites: Option<&'a [Rite]>,
    /// Who has vouched for the polity's monitors (D28). Feeds the `watchers` lens
    /// and nothing else: no component reads it, and no level moves on it.
    pub observers: Option<&'a IndependentObservation>,
}

impl CohesionInputs<'_> {
    /// Which of these readings are absent, by field name and in declaration order
    /// (**D31**). Empty when every one was supplied, which is what `en-host`'s
    /// `HostCycle::grade` guarantees and what its own test now asserts directly.
    ///
    /// The names are stable: they are what
    /// [`CohesionReport::omitted_readings`] carries and what a Python caller reads back
    /// through the binding, so they are part of the interface rather than a debug
    /// convenience.
    pub fn omitted(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.guardians.is_none() {
            out.push("guardians");
        }
        if self.affirmations.is_none() {
            out.push("affirmations");
        }
        if self.trends.is_none() {
            out.push("trends");
        }
        if self.window.is_none() {
            out.push("window");
        }
        if self.ground.is_none() {
            out.push("ground");
        }
        if self.rites.is_none() {
            out.push("rites");
        }
        if self.observers.is_none() {
            out.push("observers");
        }
        out
    }
}

/// Read a polity's cohesion by comparing its `chartered_roll` against the
/// `candidate` the substrate provider currently surfaces. `chartered_roll` is the
/// polity's **live** roll — `charter_state(...).members`, which already reflects
/// admissions and departures — so a member who formally departed is simply absent
/// from the roll and is *not* counted as attrition, while a still-enrolled member
/// the provider no longer observes *is*. `opaque_boundary` is the provider's
/// report of whether the observed
/// edge is (partly) an intentional opaque one (e.g. `AtSubstrateProvider::
/// has_private_boundary`); a provider that cannot tell should pass `false`.
/// `inputs` carries the charter-side read-models — the chartered guardian edges,
/// the transmission state, the decay trends — each optional, and each omitted one
/// reporting *nothing* rather than pretending to health (see [`CohesionInputs`]).
/// `as_of` is the moment the read is taken, against which both the attendance
/// stamps and the affirmation cadence are judged — passed in, never read off the
/// clock here, so a report is reproducible.
///
/// The erosion level is the strongest of the component signals — attrition, mean
/// reputation, floor-breach share, unrated share, unattended share, lapsed
/// affirmations, lost guardian edges, a widening affirmation gap (read both rolling
/// and against the founding cadence), a declining witness turnout, roll churn, and
/// sustained absence —
/// with two caps: under an opaque boundary the attrition component cannot exceed
/// `Notice`, because a disappearance behind an opaque edge cannot be attributed to
/// departure; and the attendance component can never exceed `Notice` at all (see
/// [`Thresholds::unattended_notice`]), because a snapshot cannot tell a night
/// shift from an abandoned polity; and the unrated component is likewise capped at
/// `Notice` (see [`Thresholds::unrated_notice`]), because an unrated member is not
/// a failing one. Lapsed affirmations are *not* capped — see
/// [`Thresholds::affirmation_stale_notice`]. Nor are lost guardian edges — see
/// [`Thresholds::guardian_lost_notice`] for why that needs no cap. The two trend
/// booleans *are* capped at `Notice`, by construction: a trend is not yet a lapse,
/// and the lapse it points at is already escalatable on its own. **Sustained
/// absence is the exception that proves the attendance cap's rule**: the same
/// absence, given a duration and a witness, does escalate.
#[allow(clippy::too_many_arguments)]
pub fn cohesion_report(
    chartered_roll: &[DidKey],
    candidate: &FoundingCandidate,
    opaque_boundary: bool,
    inputs: &CohesionInputs<'_>,
    as_of: Timestamp,
    thresholds: &Thresholds,
) -> CohesionReport {
    let no_guardians = GuardianState::default();
    let no_affirmations = AffirmationState::default();
    let guardians = inputs.guardians.unwrap_or(&no_guardians);
    let affirmations = inputs.affirmations.unwrap_or(&no_affirmations);
    // Each currently observed candidate member's readings, by DID: earned
    // reputation, whether it carries a guardian edge at all, and when that
    // guardian was last verified present.
    let observed_members: BTreeMap<&str, (Option<f64>, bool, Option<Timestamp>)> = candidate
        .members
        .iter()
        .map(|m| {
            (
                m.did.as_str(),
                (m.reputation, m.guardian.is_some(), m.attested_at),
            )
        })
        .collect();

    // The chartered members still present in the candidate, counted; and separately
    // the reputation readings among them. The two are NOT the same number — a
    // present member the provider cannot rate is observed but unrated (D18) — so
    // `present` is the attrition denominator and `present_rep.len()` the reputation
    // one. Conflating them is what let an unrated cohort read as a failing one.
    let mut present = 0usize;
    let mut present_rep: Vec<f64> = Vec::new();
    // Of those, the ones with a guardian edge — the attendance denominator — and
    // how many of them have nobody verified present.
    let mut guardian_members = 0usize;
    let mut unattended = 0usize;
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for did in chartered_roll {
        // Guard against a duplicated roll entry double-counting.
        if !seen.insert(did.as_str()) {
            continue;
        }
        if let Some((rep, has_guardian, attested_at)) = observed_members.get(did.as_str()) {
            present += 1;
            if let Some(r) = rep {
                present_rep.push(*r);
            }
            if *has_guardian {
                guardian_members += 1;
                if !is_attended(*attested_at, as_of, thresholds.attendance_cadence_seconds) {
                    unattended += 1;
                }
            }
        }
    }

    let chartered = seen.len();
    let observed = present;
    let rated_members = present_rep.len();
    let attrition = if chartered == 0 {
        0.0
    } else {
        (chartered - observed) as f64 / chartered as f64
    };
    let mean_reputation = if present_rep.is_empty() {
        0.0
    } else {
        present_rep.iter().sum::<f64>() / present_rep.len() as f64
    };
    let reputation_floor_breaches = present_rep
        .iter()
        .filter(|r| **r < thresholds.reputation_floor)
        .count();
    // Denominator is the RATED members, not the observed ones: a member nobody could
    // rate is not evidence of a breach.
    let breach_share = if rated_members == 0 {
        0.0
    } else {
        reputation_floor_breaches as f64 / rated_members as f64
    };

    let attrition_level = {
        let raw = grade(attrition, thresholds.attrition_notice, thresholds.attrition_concern);
        if opaque_boundary {
            raw.min(ErosionLevel::Notice)
        } else {
            raw
        }
    };
    // Both reputation components speak only over members that carry a reading. No
    // reading at all is "not evaluated", NEVER "failing" (D18) — the same shape as
    // the transmission and guardian components below, and the reason a freshly
    // founded polity no longer grades at Concern for being new. An empty or wholly
    // unrated polity's erosion is carried by attrition and by the unrated component,
    // not by a spurious 0.0 mean.
    let mean_level = if rated_members == 0 {
        ErosionLevel::None
    } else {
        grade_low(
            mean_reputation,
            thresholds.mean_reputation_notice,
            thresholds.mean_reputation_concern,
        )
    };
    let breach_level = grade(breach_share, thresholds.breach_notice, thresholds.breach_concern);
    // The unrated share (D18). Capped at Notice BY CONSTRUCTION — there is
    // deliberately no `unrated_concern`, exactly as there is no
    // `unattended_concern`, and for the same reason: a young cohort nobody has had
    // occasion to rate and a provider that has stopped rating are indistinguishable
    // in a snapshot. What this component exists to prevent is the two components
    // above going *silently* dark: without it, a provider that stops rating turns
    // mean reputation and the breach share off and says nothing about it.
    let unrated_level = if observed == 0 {
        ErosionLevel::None
    } else {
        let unrated_share = (observed - rated_members) as f64 / observed as f64;
        if unrated_share >= thresholds.unrated_notice {
            ErosionLevel::Notice
        } else {
            ErosionLevel::None
        }
    };
    // Attendance speaks only where there are guardians to be present: a member
    // with no human behind it is neither attended nor unattended, so a polity of
    // pure machines does not sit permanently at Notice.
    //
    // Capped at Notice by construction rather than by a threshold — there is no
    // bound that could raise this to Concern. See `Thresholds::unattended_notice`.
    let attendance_level = if guardian_members == 0 {
        ErosionLevel::None
    } else {
        let unattended_share = unattended as f64 / guardian_members as f64;
        if unattended_share >= thresholds.unattended_notice {
            ErosionLevel::Notice
        } else {
            ErosionLevel::None
        }
    };

    // Transmission decay. The staleness rule itself belongs to en-charter's lens —
    // never restated here — so there is one definition of a lapsed affirmation.
    // Intersected with the roll so a caller who hands us a roll and a state that
    // disagree cannot inflate the count.
    let stale_affirmations = affirmations
        .stale_members(thresholds.affirmation_cadence_seconds, as_of)
        .iter()
        .filter(|d| seen.contains(d.as_str()))
        .count();
    let affirming_members = affirmations
        .entries
        .iter()
        .filter(|e| seen.contains(e.member.as_str()))
        .count();
    // No transmission reading at all is "not evaluated", NOT "all fresh": zero
    // stale out of zero members must never read as health.
    let affirmation_level = if affirming_members == 0 {
        ErosionLevel::None
    } else {
        grade(
            stale_affirmations as f64 / affirming_members as f64,
            thresholds.affirmation_stale_notice,
            thresholds.affirmation_stale_concern,
        )
    };

    // The chartered guardian edge against the observed one. Only members the
    // provider STILL SEES are counted: an unobserved member is already attrition,
    // and counting them here would let an opaque boundary read as guardian loss.
    let mut chartered_guardian_members = 0usize;
    let mut guardian_edges_lost = 0usize;
    for member in guardians.guarded_members() {
        if !seen.contains(member.as_str()) {
            continue;
        }
        if let Some((_, has_guardian, _)) = observed_members.get(member.as_str()) {
            chartered_guardian_members += 1;
            if !has_guardian {
                guardian_edges_lost += 1;
            }
        }
    }
    // As with transmission: no reading is "not evaluated", not "all present".
    let guardian_level = if chartered_guardian_members == 0 {
        ErosionLevel::None
    } else {
        grade(
            guardian_edges_lost as f64 / chartered_guardian_members as f64,
            thresholds.guardian_lost_notice,
            thresholds.guardian_lost_concern,
        )
    };

    // Decay trends (D16). These are *shapes over time*, read from the polity's own
    // records rather than from the provider, and they answer a different question
    // from the state components above: not "has transmission lapsed" but "is it
    // lapsing".
    //
    // The two booleans are capped at Notice BY CONSTRUCTION, with no threshold that
    // could raise them. A widening gap or a thinning turnout is a *leading
    // indicator*; the lapse it leads to is already gradeable to Concern by the
    // affirmation component, so letting the trend escalate as well would count the
    // same decay twice — and would let a polity that is still inside its cadence
    // cross to kith-covenant on the strength of a derivative.
    let latest_gap_widening = inputs.trends.and_then(|t| t.latest_gap_widening());
    let gap_widening_since_founding =
        inputs.trends.and_then(|t| t.gap_widening_since_founding());
    let witness_count_declined = inputs.trends.is_some_and(|t| t.witness_count_declined());
    // Either reading raises the same Notice. They are not redundant: the rolling
    // one fires on a gap that is abrupt, the pinned one on a cadence that has
    // slipped since founding without any single gap ever looking abrupt.
    let gap_level = if latest_gap_widening == Some(true)
        || gap_widening_since_founding == Some(true)
    {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    };
    let witness_level = if witness_count_declined {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    };
    // Churn is continuous, so unlike the two booleans it carries a full pair: a
    // boundary redrawn fast enough is not a leading indicator of anything, it is
    // the boundary failing to hold.
    let churn_rate = inputs
        .trends
        .and_then(|t| t.churn_rate(thresholds.churn_window_seconds, chartered, as_of));
    let churn_level = match churn_rate {
        Some(rate) => grade(rate, thresholds.churn_notice, thresholds.churn_concern),
        None => ErosionLevel::None,
    };

    // Sustained absence (D16) — the one absence signal permitted to escalate,
    // because it is the only one that knows how long. Two conditions, and they do
    // different jobs: the span IS the absence, and the watcher count is evidence
    // that somebody was watching while it lasted. Either alone is defeasible; a
    // stale reading with silence behind it proves only that nobody looked.
    //
    // **Watchers, not filings** (D28). This read `readings_for` until it was
    // measured, and one key filing three times cleared the floor — so the escalation
    // this component alone is trusted with could be manufactured by a stranger for
    // one keypair. Nothing here asks whether the watchers are independent; that is
    // graded, not gated, because a polity that refuses to vouch for anyone must not
    // thereby become unwatchable.
    let sustained_absences = match inputs.window {
        None => 0,
        Some(window) => chartered_roll
            .iter()
            .filter(|did| {
                window.watchers_for(did) >= thresholds.sustained_absence_watchers
                    && window
                        .unattended_span(did, as_of)
                        .is_some_and(|span| span >= thresholds.sustained_absence_seconds)
            })
            .count(),
    };
    let sustained_level = if sustained_absences > 0 {
        ErosionLevel::Concern
    } else {
        ErosionLevel::None
    };

    // Who was doing the watching (D28). Reported beside the escalation it qualifies,
    // and read by no component: see `CohesionReport::watchers` for why this one may
    // never become a gate.
    let watchers = inputs.window.map(|window| {
        let seen: BTreeSet<&str> =
            window.observations.iter().map(|o| o.observer.as_str()).collect();
        let vouched = inputs.observers.map_or(0, |ind| {
            seen.iter()
                .filter(|w| ind.observers.iter().any(|o| o.as_str() == **w))
                .count()
        });
        let contested = inputs.observers.map_or(0, |ind| {
            seen.iter()
                .filter(|w| ind.contested.iter().any(|o| o.as_str() == **w))
                .count()
        });
        WatcherStanding { distinct: seen.len(), vouched, contested }
    });

    // D23 — the ground has withdrawn.
    //
    // The one component that grades a single act rather than a quantity, and it goes
    // straight to Concern. Every other measure here asks how far something has
    // drifted, because drift is what they can see; this one is a signed statement by
    // the only key entitled to make it, saying the polity is standing on a ground
    // that disowns it. There is nothing gradual to grade and no threshold that could
    // tune it, so there is no threshold — like kith-covenant's
    // `contraction_detected`, it is event-triggered, not a ratio.
    //
    // It cannot decay back to None: a disavowal is unprunable and there is no
    // withdrawal of a withdrawal. A polity whose ground has gone stays legibly
    // unmoored until it deliberately re-charters, which is the honest reading.
    let ground_disavowed = inputs.ground.is_some_and(GroundStanding::disavowed);
    let ground_level = if ground_disavowed {
        ErosionLevel::Concern
    } else {
        ErosionLevel::None
    };

    // D29 — the rite's own cost. `span_seconds` exists because the tier records the
    // movements now: the completion alone carried one instant and could not be
    // compared with anything.
    let all_rites = inputs.rites.unwrap_or(&[]);
    let rites = all_rites.len();
    let completed: Vec<&Rite> = all_rites.iter().filter(|r| r.completed_at.is_some()).collect();
    let rites_abandoned = rites - completed.len();
    let rite_attrition: usize = completed.iter().map(|r| r.attrition()).sum();
    // Against the FIRST rite, not the median of the priors: a ceremony that thins
    // uniformly moves its own recent history, which is the failure
    // `gap_widening_since_founding` exists to catch one measure over.
    //
    // **D34 splits the comparison by ceremony.** Inductions and renewals are not
    // comparable as ceremonies — one happens once per member and the other recurs — so a
    // single series would measure one member's reception against another's renewal and
    // call the difference drift. Each is now compared against the first of its own kind.
    let shortening_within = |kind: AffirmationKind| -> Option<bool> {
        let of_kind: Vec<&&Rite> = completed.iter().filter(|r| r.kind == kind).collect();
        if of_kind.len() < 2 {
            return None;
        }
        match (
            of_kind.first()?.span_seconds(),
            of_kind.last()?.span_seconds(),
        ) {
            (Some(a), Some(b)) => Some(b < a),
            _ => None,
        }
    };
    let rite_span_shortening = shortening_within(AffirmationKind::Induction);
    // The recurring ceremony's own cost, which before D34 did not exist to measure: a
    // renewal carried no movements, so a polity could hold its transmission measure green
    // forever without performing the rite once.
    let renewal_span_shortening = shortening_within(AffirmationKind::ReAffirmation);
    let rite_level = if rite_span_shortening == Some(true)
        || renewal_span_shortening == Some(true)
        || rite_attrition > 0
    {
        ErosionLevel::Notice
    } else {
        ErosionLevel::None
    };
    // Nobody holding the substrate is not a drift, so it takes no threshold: a polity
    // in that state can take no second-class act of its own, and saying so is more use
    // than a ratio.
    let nobody_holds_the_substrate = inputs
        .affirmations
        .is_some_and(|a| !a.entries.is_empty() && a.entries.iter().all(|e| !e.inducted));
    let substrate_level = if nobody_holds_the_substrate {
        ErosionLevel::Concern
    } else {
        ErosionLevel::None
    };

    let erosion = attrition_level
        .max(mean_level)
        .max(breach_level)
        .max(unrated_level)
        .max(attendance_level)
        .max(affirmation_level)
        .max(guardian_level)
        .max(gap_level)
        .max(witness_level)
        .max(churn_level)
        .max(sustained_level)
        .max(rite_level)
        .max(substrate_level)
        .max(ground_level);

    CohesionReport {
        attrition,
        reputation_floor_breaches,
        mean_reputation,
        opaque_boundary,
        observed,
        rated_members,
        chartered,
        guardian_members,
        unattended,
        affirming_members,
        stale_affirmations,
        chartered_guardian_members,
        guardian_edges_lost,
        latest_gap_widening,
        gap_widening_since_founding,
        witness_count_declined,
        churn_rate,
        sustained_absences,
        watchers,
        rites,
        rite_span_shortening,
        renewal_span_shortening,
        rites_abandoned,
        rite_attrition,
        nobody_holds_the_substrate,
        ground_disavowed,
        // D31 — what this grade was not given. Taken from the inputs rather than
        // reconstructed from the measures above, because several of them are
        // indistinguishable from a genuine zero: `sustained_absences == 0` is either
        // nobody absent or nothing read, and only the call knows which.
        omitted_readings: inputs.omitted(),
        erosion,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use en_charter::{DisavowalEntry, MemberAffirmation, MovementPoint, Substrate};
    use en_core::{keys, Alg, DidKey};
    use en_substrate::{FoundingCandidate, MemberCandidate};

    fn did() -> DidKey {
        let (_sk, pk) = keys::generate(Alg::Ed25519).unwrap();
        DidKey::from_pubkey(&pk).unwrap()
    }

    fn substrate() -> Substrate {
        Substrate { name: "tradition".into(), statement: "answers to X".into(), anchor: None }
    }

    /// An affirmation state: `(member, inducted, last_affirmed_at)`.
    fn affirmed(
        entries: &[(DidKey, bool, Option<Timestamp>)],
    ) -> AffirmationState {
        AffirmationState {
            entries: entries
                .iter()
                .map(|(m, inducted, last)| MemberAffirmation {
                    member: m.clone(),
                    inducted: *inducted,
                    last_affirmed_at: *last,
                })
                .collect(),
        }
    }

    /// A fixed instant, so freshness assertions never touch the wall clock.
    fn at(secs: i64) -> Timestamp {
        Timestamp::from_second(1_721_800_000 + secs).unwrap()
    }

    /// A candidate whose members carry guardian edges and attendance stamps:
    /// `(did, reputation, guardian, attested_at)`. `guardian: None` is a member
    /// with no human behind it at all.
    fn guarded(
        members: &[(DidKey, f64, Option<DidKey>, Option<Timestamp>)],
    ) -> FoundingCandidate {
        FoundingCandidate {
            members: members
                .iter()
                .map(|(d, r, g, a)| MemberCandidate {
                    did: d.clone(),
                    reputation: Some(*r),
                    derived: g.is_some(),
                    guardian: g.clone(),
                    attested_at: *a,
                })
                .collect(),
            substrate: Some(substrate()),
        }
    }

    /// A candidate whose members may be **unrated**: `(did, Option<reputation>)`.
    /// The one thing `guarded` cannot express, and the whole point of D18.
    fn rated(members: &[(DidKey, Option<f64>)]) -> FoundingCandidate {
        FoundingCandidate {
            members: members
                .iter()
                .map(|(d, r)| MemberCandidate {
                    did: d.clone(),
                    reputation: *r,
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
        }
    }

    /// A chartered guardian read-model: `(member, guardian)`. `None` marks a
    /// designated machine the charter says answers to nobody, which the
    /// guardian-edge component ignores — there is no edge there to lose.
    fn chartered(edges: &[(DidKey, Option<DidKey>)]) -> GuardianState {
        GuardianState {
            derivations: edges
                .iter()
                .map(|(m, g)| en_charter::DerivedEdge { member: m.clone(), guardian: g.clone() })
                .collect(),
        }
    }

    /// A trends read-model built directly, so the grader can be tested without a
    /// log: `gaps` are the successive affirmation intervals and `witnesses` the
    /// per-induction turnout, oldest first.
    fn trends(gaps: &[i64], witnesses: &[usize], churn: &[i64]) -> DecayTrends {
        let mut at_secs = 0i64;
        let mut affirmations: Vec<en_charter::AffirmationPoint> = Vec::new();
        // One affirmation point per gap, plus the first — inductions consume the
        // witness counts in order, and any remainder are re-affirmations.
        for (i, gap) in std::iter::once(&0).chain(gaps.iter()).enumerate() {
            at_secs += gap;
            let (kind, witness_count) = match witnesses.get(i) {
                Some(w) => (en_charter::AffirmationKind::Induction, *w),
                None => (en_charter::AffirmationKind::ReAffirmation, 0),
            };
            affirmations.push(en_charter::AffirmationPoint {
                record_cid: en_core::Cid::of(&at_secs.to_be_bytes()).unwrap(),
                at: at(at_secs),
                witness_count,
                kind,
            });
        }
        DecayTrends {
            gaps_seconds: gaps.to_vec(),
            seconds_since_last: Some(0),
            membership_changes: churn
                .iter()
                .map(|c| en_charter::ChurnPoint {
                    record_cid: en_core::Cid::of(&c.to_be_bytes()).unwrap(),
                    at: at(*c),
                    kind: en_charter::MembershipKind::Admit,
                })
                .collect(),
            affirmations,
        }
    }

    /// One point in a hand-built observation window: when it was filed, and who was
    /// seen with which attendance stamp. Named so the three helpers below state the
    /// same shape once instead of three times.
    type WindowPoint = (i64, Vec<(DidKey, Option<i64>)>);

    /// An observation window built directly: each entry is one observation at
    /// `(at, [(member, attendance stamp)])`.
    /// A window whose every filing is by a *different* observer — the ordinary case,
    /// and since D28 the one the sustained-absence grade needs.
    fn window(points: &[WindowPoint]) -> ObservationWindow {
        window_by(points, &[])
    }

    /// `window`, but with every filing signed by the same `solo` observer when one is
    /// given: one party's word, however many times it is repeated.
    fn window_by(points: &[WindowPoint], solo: &[DidKey]) -> ObservationWindow {
        let mut w = window_inner(points);
        if let Some(one) = solo.first() {
            for o in &mut w.observations {
                o.observer = one.clone();
            }
        }
        w
    }

    fn window_inner(points: &[WindowPoint]) -> ObservationWindow {
        ObservationWindow {
            observations: points
                .iter()
                .map(|(when, readings)| en_charter::ObservationPoint {
                    record_cid: en_core::Cid::of(&when.to_be_bytes()).unwrap(),
                    observer: did(),
                    at: at(*when),
                    readings: readings
                        .iter()
                        .map(|(m, stamp)| en_charter::MemberReading {
                            member: m.clone(),
                            reputation: 0.9,
                            guardian: Some(did()),
                            attested_at: stamp.map(at),
                        })
                        .collect(),
                    opaque_boundary: false,
                })
                .collect(),
            pruned_before: None,
        }
    }

    fn candidate(members: &[(DidKey, f64)]) -> FoundingCandidate {
        FoundingCandidate {
            members: members
                .iter()
                .map(|(d, r)| MemberCandidate {
                    did: d.clone(),
                    reputation: Some(*r),
                    derived: false,
                    guardian: None,
                    attested_at: None,
                })
                .collect(),
            substrate: Some(substrate()),
        }
    }

    #[test]
    fn healthy_roll_is_none() {
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.chartered, 5);
        assert_eq!(r.observed, 5);
        assert_eq!(r.attrition, 0.0);
        assert_eq!(r.reputation_floor_breaches, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn mild_attrition_is_notice() {
        // 5 chartered, 4 still observed → attrition 0.2 (== notice, < concern).
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(4).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.observed, 4);
        assert!((r.attrition - 0.2).abs() < 1e-9);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn heavy_attrition_is_concern() {
        // 5 chartered, 2 still observed → attrition 0.6 (>= concern 0.4).
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(2).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert!((r.attrition - 0.6).abs() < 1e-9);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn opaque_boundary_caps_attrition_at_notice() {
        // The same 0.6 attrition, but behind an opaque edge → capped at Notice,
        // because the disappearance cannot be attributed to departure.
        let roll: Vec<DidKey> = (0..5).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().take(2).map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(&roll, &candidate(&members), true, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert!(r.opaque_boundary);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn low_mean_reputation_is_concern() {
        // Full roll observed, but reputation has decayed below the concern mean.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.3)).collect();
        let r = cohesion_report(&roll, &candidate(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.attrition, 0.0);
        assert!(r.mean_reputation <= Thresholds::default().mean_reputation_concern);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn floor_breaches_counted_and_graded() {
        // 4 observed, 2 below the 0.3 floor → breach share 0.5 (>= concern 0.4),
        // but the mean (≈0.5) stays above its concern bound, so the breach share
        // is what carries the Concern.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = vec![
            (roll[0].clone(), 0.8),
            (roll[1].clone(), 0.8),
            (roll[2].clone(), 0.1),
            (roll[3].clone(), 0.1),
        ];
        let r = cohesion_report(&roll, &candidate(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.reputation_floor_breaches, 2);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn empty_chartered_roll_is_none() {
        let r = cohesion_report(&[], &candidate(&[]), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.chartered, 0);
        assert_eq!(r.observed, 0);
        assert_eq!(r.attrition, 0.0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    // --- attendance: the live half of the operator signal (design.md Q9a) ------

    #[test]
    fn guardians_present_is_none() {
        let g = did();
        let members: Vec<_> = (0..4)
            .map(|_| (did(), 0.9, Some(g.clone()), Some(at(0))))
            .collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        // Read one minute later: every stamp is still inside the cadence.
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(60), &Thresholds::default());
        assert_eq!(r.guardian_members, 4);
        assert_eq!(r.unattended, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn stale_attendance_stamps_count_as_unattended() {
        let g = did();
        let members: Vec<_> = (0..4)
            .map(|_| (did(), 0.9, Some(g.clone()), Some(at(0))))
            .collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        // An hour later every stamp has aged out of the 300s cadence.
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(3600), &Thresholds::default());
        assert_eq!(r.unattended, 4);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn the_cadence_boundary_is_inclusive() {
        // Exactly at the cadence still counts as present; one second past does not.
        let g = did();
        let d = did();
        let members = [(d.clone(), 0.9, Some(g), Some(at(0)))];
        let roll = vec![d];
        let t = Thresholds::default();

        let edge = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(300), &t);
        assert_eq!(edge.unattended, 0);

        let past = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(301), &t);
        assert_eq!(past.unattended, 1);
    }

    #[test]
    fn a_never_attended_guardian_member_is_unattended() {
        // No stamp at all is not attended: "never seen" and "seen too long ago"
        // answer the same question the same way.
        let g = did();
        let d = did();
        let members = [(d.clone(), 0.9, Some(g), None)];
        let roll = vec![d];
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.guardian_members, 1);
        assert_eq!(r.unattended, 1);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn members_without_a_guardian_are_outside_the_measure() {
        // A machine with no human behind it is neither attended nor unattended,
        // so a polity of pure machines does not rest at Notice.
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.guardian_members, 0);
        assert_eq!(r.unattended, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn the_denominator_is_only_the_guardian_bearing_members() {
        // Two of four members have guardians; one of those two is away. That is
        // half the *guardian-bearing* members (>= 0.5), not a quarter of the roll.
        let g = did();
        let a = did();
        let b = did();
        let c = did();
        let e = did();
        let members = [
            (a.clone(), 0.9, Some(g.clone()), Some(at(0))),
            (b.clone(), 0.9, Some(g), None),
            (c.clone(), 0.9, None, None),
            (e.clone(), 0.9, None, None),
        ];
        let roll = vec![a, b, c, e];
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.guardian_members, 2);
        assert_eq!(r.unattended, 1);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn total_absence_of_guardians_never_reaches_concern() {
        // THE load-bearing case. Every guardian away is the worst this signal can
        // report, and it must still stop at Notice: a snapshot cannot tell a night
        // shift from an abandoned polity, and a Concern would cross to
        // kith-covenant every evening. The cap is structural — no threshold can
        // loosen it, which is why there is no `unattended_concern`.
        let g = did();
        let members: Vec<_> = (0..8)
            .map(|_| (did(), 0.95, Some(g.clone()), None))
            .collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());

        assert_eq!(r.guardian_members, 8);
        assert_eq!(r.unattended, 8);
        assert_eq!(r.erosion, ErosionLevel::Notice);
        assert_ne!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn attendance_does_not_mask_a_concern_from_another_component() {
        // The components combine by max, so a capped attendance signal must not
        // pull a real Concern down. Here attendance is fine and reputation is not.
        let g = did();
        let members: Vec<_> = (0..4)
            .map(|_| (did(), 0.3, Some(g.clone()), Some(at(0))))
            .collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.unattended, 0);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn attendance_is_ignored_for_members_off_the_roll() {
        // The walk is over the roll, so an unattended peer the polity never
        // chartered cannot drag its cohesion down.
        let g = did();
        let mine = did();
        let stranger = did();
        let members = [
            (mine.clone(), 0.9, Some(g.clone()), Some(at(0))),
            (stranger, 0.9, Some(g), None),
        ];
        let roll = vec![mine];
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.guardian_members, 1);
        assert_eq!(r.unattended, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn an_unobserved_member_is_attrition_not_absence() {
        // A chartered member the provider no longer sees at all is not counted as
        // an unattended guardian — it is gone, which attrition already says.
        let g = did();
        let here = did();
        let gone = did();
        let members = [(here.clone(), 0.9, Some(g), Some(at(0)))];
        let roll = vec![here, gone];
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(r.chartered, 2);
        assert_eq!(r.observed, 1);
        assert_eq!(r.guardian_members, 1);
        assert_eq!(r.unattended, 0);
        assert!(r.attrition > 0.0);
    }

    #[test]
    fn the_unattended_notice_bound_is_configurable() {
        // The Notice cut-point can be tuned (only the Concern cap is structural).
        let g = did();
        let a = did();
        let b = did();
        let members = [
            (a.clone(), 0.9, Some(g.clone()), Some(at(0))),
            (b.clone(), 0.9, Some(g), None),
        ];
        let roll = vec![a, b];
        // Half unattended: raises a Notice under the default 0.5 bound...
        let strict = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &Thresholds::default());
        assert_eq!(strict.erosion, ErosionLevel::Notice);
        // ...and stays silent when the deployment tolerates more absence.
        let loose = Thresholds {
            unattended_notice: 0.9,
            ..Thresholds::default()
        };
        let r = cohesion_report(&roll, &guarded(&members), false, &CohesionInputs::default(), at(0), &loose);
        assert_eq!(r.unattended, 1);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    // --- transmission decay: is the substrate still being re-affirmed? ---------
    // The staleness rule is en-charter's (AffirmationState::stale_members); these
    // pin how it GRADES, including that — unlike attendance — it may escalate.

    const CADENCE: i64 = 180 * 24 * 60 * 60;

    #[test]
    fn a_currently_affirming_roll_is_none() {
        let members: Vec<_> = (0..4).map(|_| did()).collect();
        let roll = members.clone();
        let state = affirmed(
            &members
                .iter()
                .map(|d| (d.clone(), true, Some(at(0))))
                .collect::<Vec<_>>(),
        );
        // A day later: everyone is well inside the 180-day cadence.
        let r = cohesion_report(
            &roll,
            &candidate(&roll.iter().map(|d| (d.clone(), 0.9)).collect::<Vec<_>>()),
            false,
            &CohesionInputs { affirmations: Some(&state), ..Default::default() },
            at(86_400),
            &Thresholds::default(),
        );
        assert_eq!(r.affirming_members, 4);
        assert_eq!(r.stale_affirmations, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn lapsed_affirmations_reach_concern() {
        // THE load-bearing case, and the opposite of the attendance cap: half the
        // roll past the cadence IS a Concern, because a member who has not
        // re-affirmed in 180 days is not "at lunch" — the transmission has lapsed.
        let members: Vec<_> = (0..4).map(|_| did()).collect();
        let roll = members.clone();
        let state = affirmed(&[
            (members[0].clone(), true, Some(at(0))),
            (members[1].clone(), true, Some(at(0))),
            (members[2].clone(), true, Some(at(CADENCE * 2))),
            (members[3].clone(), true, Some(at(CADENCE * 2))),
        ]);
        // Read at 2 cadences + a day: the first two have lapsed, the last two not.
        let r = cohesion_report(
            &roll,
            &candidate(&roll.iter().map(|d| (d.clone(), 0.9)).collect::<Vec<_>>()),
            false,
            &CohesionInputs { affirmations: Some(&state), ..Default::default() },
            at(CADENCE * 2 + 86_400),
            &Thresholds::default(),
        );
        assert_eq!(r.stale_affirmations, 2);
        assert_eq!(r.erosion, ErosionLevel::Concern);
        assert_ne!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn a_quarter_lapsed_is_only_a_notice() {
        let members: Vec<_> = (0..4).map(|_| did()).collect();
        let roll = members.clone();
        let mut entries: Vec<_> = members
            .iter()
            .map(|d| (d.clone(), true, Some(at(CADENCE * 2))))
            .collect();
        entries[0] = (members[0].clone(), true, Some(at(0))); // the lapsed one
        let r = cohesion_report(
            &roll,
            &candidate(&roll.iter().map(|d| (d.clone(), 0.9)).collect::<Vec<_>>()),
            false,
            &CohesionInputs { affirmations: Some(&affirmed(&entries)), ..Default::default() },
            at(CADENCE * 2 + 86_400),
            &Thresholds::default(),
        );
        assert_eq!(r.stale_affirmations, 1); // 0.25 == notice, < concern
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn a_never_inducted_member_counts_as_lapsed() {
        // en-charter's rule: never inducted is stale, full stop — a member who
        // never acquired the substrate cannot have kept it current.
        let d = did();
        let roll = vec![d.clone()];
        let r = cohesion_report(
            &roll,
            &candidate(&[(d.clone(), 0.9)]),
            false,
            &CohesionInputs { affirmations: Some(&affirmed(&[(d, false, None)])), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.affirming_members, 1);
        assert_eq!(r.stale_affirmations, 1);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn the_affirmation_cadence_boundary_is_exclusive() {
        // Note the deliberate asymmetry with attendance: en-charter's staleness is
        // a STRICT `>`, so exactly-at-the-cadence is still current, whereas the
        // attendance cadence uses `<=`. Each is pinned in its own test so a change
        // to either rule cannot pass silently.
        let d = did();
        let roll = vec![d.clone()];
        let state = affirmed(&[(d.clone(), true, Some(at(0)))]);
        let cand = candidate(&[(d, 0.9)]);
        let t = Thresholds::default();

        let edge = cohesion_report(&roll, &cand, false, &CohesionInputs { affirmations: Some(&state), ..Default::default() }, at(CADENCE), &t);
        assert_eq!(edge.stale_affirmations, 0);

        let past = cohesion_report(&roll, &cand, false, &CohesionInputs { affirmations: Some(&state), ..Default::default() }, at(CADENCE + 1), &t);
        assert_eq!(past.stale_affirmations, 1);
    }

    #[test]
    fn no_affirmation_reading_is_not_evaluated_not_healthy() {
        // An empty state must not read as "nobody is stale". The component simply
        // says nothing, which is the honest answer when no archive was consulted.
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, _)| d.clone()).collect();
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs::default(),
            at(CADENCE * 10),
            &Thresholds::default(),
        );
        assert_eq!(r.affirming_members, 0);
        assert_eq!(r.stale_affirmations, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn affirmations_off_the_roll_are_ignored() {
        // A stale entry for someone the polity does not charter cannot drag its
        // cohesion down, and cannot pad the denominator either.
        let mine = did();
        let stranger = did();
        let roll = vec![mine.clone()];
        let r = cohesion_report(
            &roll,
            &candidate(&[(mine.clone(), 0.9)]),
            false,
            &CohesionInputs {
                affirmations: Some(&affirmed(&[
                (mine, true, Some(at(CADENCE * 2))),
                (stranger, false, None),
            ])),
                ..Default::default()
            },
            at(CADENCE * 2 + 1),
            &Thresholds::default(),
        );
        assert_eq!(r.affirming_members, 1);
        assert_eq!(r.stale_affirmations, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn transmission_and_attendance_are_independent() {
        // Two different liveness questions: is a human at the console, and is the
        // substrate still being carried. Either may speak while the other is silent.
        let g = did();
        let a = did();
        let b = did();
        let roll = vec![a.clone(), b.clone()];
        let cand = guarded(&[
            (a.clone(), 0.9, Some(g.clone()), Some(at(CADENCE * 2))),
            (b.clone(), 0.9, Some(g), Some(at(CADENCE * 2))),
        ]);
        let read_at = at(CADENCE * 2);
        let t = Thresholds::default();

        // Guardians present, transmission lapsed -> Concern from affirmations only.
        let lapsed = affirmed(&[(a.clone(), true, Some(at(0))), (b.clone(), true, Some(at(0)))]);
        let r = cohesion_report(&roll, &cand, false, &CohesionInputs { affirmations: Some(&lapsed), ..Default::default() }, read_at, &t);
        assert_eq!(r.unattended, 0);
        assert_eq!(r.stale_affirmations, 2);
        assert_eq!(r.erosion, ErosionLevel::Concern);

        // Transmission current, guardians away -> Notice from attendance only.
        let current = affirmed(&[
            (a.clone(), true, Some(read_at)),
            (b.clone(), true, Some(read_at)),
        ]);
        let away = guarded(&[
            (a, 0.9, Some(did()), None),
            (b, 0.9, Some(did()), None),
        ]);
        let r2 = cohesion_report(&roll, &away, false, &CohesionInputs { affirmations: Some(&current), ..Default::default() }, read_at, &t);
        assert_eq!(r2.stale_affirmations, 0);
        assert_eq!(r2.unattended, 2);
        assert_eq!(r2.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn the_affirmation_bounds_are_configurable() {
        let members: Vec<_> = (0..4).map(|_| did()).collect();
        let roll = members.clone();
        let state = affirmed(
            &members
                .iter()
                .map(|d| (d.clone(), true, Some(at(0))))
                .collect::<Vec<_>>(),
        );
        let cand = candidate(&roll.iter().map(|d| (d.clone(), 0.9)).collect::<Vec<_>>());
        let read_at = at(CADENCE * 2);

        // All four lapsed under the default cadence...
        let strict = cohesion_report(&roll, &cand, false, &CohesionInputs { affirmations: Some(&state), ..Default::default() }, read_at, &Thresholds::default());
        assert_eq!(strict.stale_affirmations, 4);
        assert_eq!(strict.erosion, ErosionLevel::Concern);

        // ...and none of them under a cadence the deployment sets wider.
        let lax = Thresholds {
            affirmation_cadence_seconds: CADENCE * 10,
            ..Thresholds::default()
        };
        let r = cohesion_report(&roll, &cand, false, &CohesionInputs { affirmations: Some(&state), ..Default::default() }, read_at, &lax);
        assert_eq!(r.stale_affirmations, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    // --- D15: the chartered guardian edge against the observed one -------------

    #[test]
    fn a_chartered_guardian_the_substrate_no_longer_shows_is_erosion() {
        let machines: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let human = did();
        let roll = machines.clone();
        let edges = chartered(
            &machines.iter().map(|m| (m.clone(), Some(human.clone()))).collect::<Vec<_>>(),
        );
        // All four observed; two of them no longer carry a guardian.
        let obs = guarded(&[
            (machines[0].clone(), 0.9, Some(human.clone()), Some(at(0))),
            (machines[1].clone(), 0.9, Some(human.clone()), Some(at(0))),
            (machines[2].clone(), 0.9, None, None),
            (machines[3].clone(), 0.9, None, None),
        ]);
        let r = cohesion_report(&roll, &obs, false, &CohesionInputs { guardians: Some(&edges), ..Default::default() }, at(0), &Thresholds::default());
        assert_eq!(r.chartered_guardian_members, 4);
        assert_eq!(r.guardian_edges_lost, 2);
        // 0.5 — at the concern bound. Unlike attendance, this one escalates.
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn one_lost_edge_of_four_is_a_notice() {
        let machines: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let human = did();
        let edges = chartered(
            &machines.iter().map(|m| (m.clone(), Some(human.clone()))).collect::<Vec<_>>(),
        );
        let mut obs: Vec<(DidKey, f64, Option<DidKey>, Option<Timestamp>)> = machines
            .iter()
            .map(|m| (m.clone(), 0.9, Some(human.clone()), Some(at(0))))
            .collect();
        obs[0].2 = None;
        let r = cohesion_report(
            &machines,
            &guarded(&obs),
            false,
            &CohesionInputs { guardians: Some(&edges), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.guardian_edges_lost, 1); // 0.25 == notice, < concern
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn an_unobserved_member_is_attrition_and_never_a_lost_edge() {
        // The load-bearing restriction: only members the provider STILL SEES can
        // lose an edge. Otherwise a member who vanished — or went behind an opaque
        // boundary — would be counted twice, once as attrition and once as guardian
        // loss, and an opaque edge could masquerade as answerability collapsing.
        let machines: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let human = did();
        let edges = chartered(
            &machines.iter().map(|m| (m.clone(), Some(human.clone()))).collect::<Vec<_>>(),
        );
        // Only two of the four are observed at all; both still carry their guardian.
        let obs = guarded(&[
            (machines[0].clone(), 0.9, Some(human.clone()), Some(at(0))),
            (machines[1].clone(), 0.9, Some(human.clone()), Some(at(0))),
        ]);
        let r = cohesion_report(&machines, &obs, false, &CohesionInputs { guardians: Some(&edges), ..Default::default() }, at(0), &Thresholds::default());
        assert_eq!(r.chartered_guardian_members, 2, "the denominator is the observed, not the roll");
        assert_eq!(r.guardian_edges_lost, 0);
        assert_eq!(r.attrition, 0.5);
    }

    #[test]
    fn no_guardian_reading_is_not_evaluated_rather_than_healthy() {
        let machines: Vec<DidKey> = (0..2).map(|_| did()).collect();
        let obs = guarded(&[
            (machines[0].clone(), 0.9, None, None),
            (machines[1].clone(), 0.9, None, None),
        ]);
        // Nobody is chartered as derived, so there is no edge to have lost — even
        // though not one observed member carries a guardian.
        let r = cohesion_report(
            &machines,
            &obs,
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.chartered_guardian_members, 0);
        assert_eq!(r.guardian_edges_lost, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn a_machine_chartered_with_nobody_behind_it_has_no_edge_to_lose() {
        // `guardian: None` in the charter is an honest state, not a defect the
        // monitor re-reports: the office rule is what answers for it (D15).
        let machine = did();
        let roll = vec![machine.clone()];
        let r = cohesion_report(
            &roll,
            &guarded(&[(machine.clone(), 0.9, None, None)]),
            false,
            &CohesionInputs {
                guardians: Some(&chartered(&[(machine, None)])),
                ..Default::default()
            },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.chartered_guardian_members, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn the_guardian_edge_component_is_independent_of_attendance() {
        // An edge present but unattended is a Notice (capped); an edge lost with
        // everyone else present is a Concern. Neither borrows the other's level.
        let machines: Vec<DidKey> = (0..2).map(|_| did()).collect();
        let human = did();
        let edges = chartered(&[
            (machines[0].clone(), Some(human.clone())),
            (machines[1].clone(), Some(human.clone())),
        ]);

        // Both edges intact, nobody at the console.
        let unattended = guarded(&[
            (machines[0].clone(), 0.9, Some(human.clone()), None),
            (machines[1].clone(), 0.9, Some(human.clone()), None),
        ]);
        let r = cohesion_report(&machines, &unattended, false, &CohesionInputs { guardians: Some(&edges), ..Default::default() }, at(0), &Thresholds::default());
        assert_eq!(r.unattended, 2);
        assert_eq!(r.guardian_edges_lost, 0);
        assert_eq!(r.erosion, ErosionLevel::Notice, "attendance alone stays capped");

        // Both edges gone, and attendance therefore says nothing at all.
        let lost = guarded(&[
            (machines[0].clone(), 0.9, None, None),
            (machines[1].clone(), 0.9, None, None),
        ]);
        let r = cohesion_report(&machines, &lost, false, &CohesionInputs { guardians: Some(&edges), ..Default::default() }, at(0), &Thresholds::default());
        assert_eq!(r.guardian_members, 0, "no observed edges, so no attendance denominator");
        assert_eq!(r.guardian_edges_lost, 2);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    // --- D16: decay trends, graded ---------------------------------------------

    // --- D18: unrated members are not failing members ------------------------

    #[test]
    fn an_unrated_cohort_is_not_a_failing_cohort() {
        // THE regression this slice exists for (finding E3). Before D18 a provider
        // that could not rate anyone had to report SOME number, and any number low
        // enough to mean "we don't know" sat below the 0.3 failure floor — so a
        // freshly founded polity graded at Concern, the level that crosses to
        // kith-covenant, purely for being new.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let unrated: Vec<(DidKey, Option<f64>)> =
            roll.iter().map(|d| (d.clone(), None)).collect();
        let r = cohesion_report(
            &roll,
            &rated(&unrated),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.observed, 4, "all four are present");
        assert_eq!(r.rated_members, 0, "none of them is rated");
        assert_eq!(r.reputation_floor_breaches, 0, "an unrated member breaches nothing");
        // Notice from the unrated component alone — never Concern.
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn the_same_roll_actually_rated_low_still_reaches_concern() {
        // The other half of the regression: D18 must separate *unknown* from
        // *failing*, not soften *failing*. Same roll, same size, readings present
        // and bad.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let low: Vec<(DidKey, Option<f64>)> =
            roll.iter().map(|d| (d.clone(), Some(0.2))).collect();
        let r = cohesion_report(
            &roll,
            &rated(&low),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rated_members, 4);
        assert_eq!(r.reputation_floor_breaches, 4);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn the_unrated_component_never_escalates() {
        // Capped at Notice BY CONSTRUCTION: there is no threshold that could raise
        // it, so even a wholly unrated roll stops at Notice. A young cohort and a
        // provider that has stopped rating are indistinguishable here, which is
        // exactly why this may warn and must not cross to KC.
        let roll: Vec<DidKey> = (0..8).map(|_| did()).collect();
        let unrated: Vec<(DidKey, Option<f64>)> =
            roll.iter().map(|d| (d.clone(), None)).collect();
        let r = cohesion_report(
            &roll,
            &rated(&unrated),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn a_partly_rated_roll_grades_only_the_rated_half() {
        // Two rated members, both healthy; two unrated. The mean and the breach
        // share must be computed over the RATED two — counting the unrated in the
        // denominator would dilute a real breach share, and counting them as
        // breaches would invent one.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let mixed = vec![
            (roll[0].clone(), Some(0.9)),
            (roll[1].clone(), Some(0.9)),
            (roll[2].clone(), None),
            (roll[3].clone(), None),
        ];
        let r = cohesion_report(
            &roll,
            &rated(&mixed),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.observed, 4);
        assert_eq!(r.rated_members, 2);
        assert!((r.mean_reputation - 0.9).abs() < 1e-9, "mean over the rated two");
        assert_eq!(r.reputation_floor_breaches, 0);
        // Half unrated meets `unrated_notice` (0.5) exactly, so: Notice, not None.
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn one_unrated_member_in_four_says_nothing() {
        // Negative control on the unrated component: below the share it is silent,
        // and the rated three still grade normally.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let mostly = vec![
            (roll[0].clone(), Some(0.9)),
            (roll[1].clone(), Some(0.9)),
            (roll[2].clone(), Some(0.9)),
            (roll[3].clone(), None),
        ];
        let r = cohesion_report(
            &roll,
            &rated(&mostly),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rated_members, 3);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn an_unrated_member_is_still_observed_not_attrition() {
        // The two axes must not bleed: being unrated says nothing about presence.
        // A polity whose members are all present but unrated has ZERO attrition.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let unrated: Vec<(DidKey, Option<f64>)> =
            roll.iter().map(|d| (d.clone(), None)).collect();
        let r = cohesion_report(
            &roll,
            &rated(&unrated),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.attrition, 0.0);
        assert_eq!(r.observed, 4);
    }

    #[test]
    fn a_widening_affirmation_gap_warns_but_never_escalates() {
        // Capped at Notice by construction: the lapse a widening gap points at is
        // already escalatable by the affirmation component, so escalating here too
        // would grade the same decay twice.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let widening = trends(&[100, 100, 10_000], &[], &[]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { trends: Some(&widening), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.latest_gap_widening, Some(true));
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn a_steady_cadence_and_too_short_a_series_both_read_as_no_signal() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let report = |t: &DecayTrends| {
            cohesion_report(
                &roll,
                &candidate(&members),
                false,
                &CohesionInputs { trends: Some(t), ..Default::default() },
                at(0),
                &Thresholds::default(),
            )
        };
        assert_eq!(report(&trends(&[100, 100, 100], &[], &[])).erosion, ErosionLevel::None);
        // One gap cannot be compared to a median of nothing: unknown, not healthy —
        // and unknown must not raise anything either.
        let r = report(&trends(&[100], &[], &[]));
        assert_eq!(r.latest_gap_widening, None);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn a_cadence_that_slipped_and_stayed_there_still_raises_a_notice() {
        // The shifting-baseline case: gaps 100, 400, 400, 400. The rolling median
        // has absorbed the drift, so `latest_gap_widening` is Some(false) — the
        // polity's own recent behaviour has become the reference. Pinned to the
        // founding cadence the slip is still there, and the component still warns.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let settled = trends(&[100, 400, 400, 400], &[], &[]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { trends: Some(&settled), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.latest_gap_widening, Some(false), "the rolling reading is silent");
        assert_eq!(r.gap_widening_since_founding, Some(true));
        assert_eq!(r.erosion, ErosionLevel::Notice, "still capped at Notice");
    }

    #[test]
    fn the_pinned_gap_component_invents_no_signal() {
        // Negative controls: a held cadence is not flagged, and one gap says
        // nothing rather than reporting health.
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let report = |t: &DecayTrends| {
            cohesion_report(
                &roll,
                &candidate(&members),
                false,
                &CohesionInputs { trends: Some(t), ..Default::default() },
                at(0),
                &Thresholds::default(),
            )
        };
        let steady = report(&trends(&[100, 100, 100], &[], &[]));
        assert_eq!(steady.gap_widening_since_founding, Some(false));
        assert_eq!(steady.erosion, ErosionLevel::None);

        let single = report(&trends(&[100], &[], &[]));
        assert_eq!(single.gap_widening_since_founding, None);
        assert_eq!(single.erosion, ErosionLevel::None);
    }

    /// D29 — a rite fixture: `(recited_at, attendees)` per movement, drawn from one
    /// pool so the *same* people attend across movements. (Minting fresh DIDs per
    /// movement made every attendee read as lost, which is the measure working.)
    fn rite(member: &DidKey, spans: &[(i64, usize)], completed: bool) -> Rite {
        rite_of(member, spans, completed, AffirmationKind::Induction)
    }

    fn rite_of(
        member: &DidKey,
        spans: &[(i64, usize)],
        completed: bool,
        kind: AffirmationKind,
    ) -> Rite {
        let pool: Vec<DidKey> =
            (0..spans.iter().map(|(_, n)| *n).max().unwrap_or(0)).map(|_| did()).collect();
        let movements = spans
            .iter()
            .map(|(at, present)| MovementPoint {
                order: 1,
                attending: pool[..*present].to_vec(),
                recited_at: at_ts(*at),
            })
            .collect();
        Rite {
            member: member.clone(),
            kind,
            movements,
            completed_at: completed.then(|| at_ts(spans.last().map(|s| s.0).unwrap_or(0))),
        }
    }

    fn at_ts(secs: i64) -> Timestamp {
        Timestamp::from_second(secs).unwrap()
    }

    #[test]
    fn a_cheapening_rite_warns_but_never_escalates() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // A rite held over a week, then one performed in five seconds.
        let rites = vec![
            rite(&roll[0], &[(0, 3), (604_800, 3)], true),
            rite(&roll[1], &[(1_000_000, 3), (1_000_005, 3)], true),
        ];
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { rites: Some(&rites), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rites, 2);
        assert_eq!(r.rite_span_shortening, Some(true));
        assert_eq!(r.erosion, ErosionLevel::Notice, "a cheapening ceremony warns");

        // One rite is not a trend, and a lengthening one is not a signal.
        let single = vec![rite(&roll[0], &[(0, 3), (5, 3)], true)];
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { rites: Some(&single), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rite_span_shortening, None, "no trend from one rite");
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    /// **D34** — the recurring ceremony gets its own cheapening measure, kept separate
    /// from the induction's because the two are not comparable as ceremonies: one
    /// happens once per member, the other forever.
    #[test]
    fn a_cheapening_renewal_is_measured_within_renewals() {
        let roll: Vec<DidKey> = (0..3).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let grade = |rites: &[Rite]| {
            cohesion_report(
                &roll,
                &candidate(&members),
                false,
                &CohesionInputs { rites: Some(rites), ..Default::default() },
                at(0),
                &Thresholds::default(),
            )
        };

        // A week-long first renewal, then a five-second one.
        let thinning = vec![
            rite_of(&roll[0], &[(0, 2), (604_800, 2)], true, AffirmationKind::ReAffirmation),
            rite_of(
                &roll[0],
                &[(2_000_000, 2), (2_000_005, 2)],
                true,
                AffirmationKind::ReAffirmation,
            ),
        ];
        let r = grade(&thinning);
        assert_eq!(r.renewal_span_shortening, Some(true));
        assert_eq!(r.rite_span_shortening, None, "no induction series to read");
        assert_eq!(r.erosion, ErosionLevel::Notice, "capped at Notice, like D29's trends");

        // The two series do not contaminate each other: a long renewal beside a
        // shortening induction leaves each measure reading its own kind.
        let mixed = vec![
            rite(&roll[0], &[(0, 2), (604_800, 2)], true),
            rite(&roll[1], &[(1_000_000, 2), (1_000_005, 2)], true),
            rite_of(&roll[2], &[(5, 2), (10, 2)], true, AffirmationKind::ReAffirmation),
            rite_of(
                &roll[2],
                &[(2_000_000, 2), (2_604_800, 2)],
                true,
                AffirmationKind::ReAffirmation,
            ),
        ];
        let r = grade(&mixed);
        assert_eq!(r.rite_span_shortening, Some(true), "the inductions thinned");
        assert_eq!(r.renewal_span_shortening, Some(false), "the renewals lengthened");
    }

    /// The uniform-drift lesson, one measure over: graded against the FIRST renewal, a
    /// ceremony that thinned once and stayed thin is still visible. Measured against its
    /// own recent history it would go quiet, because the cheapened ceremony has become
    /// what normal looks like — which is exactly the failure
    /// `gap_widening_since_founding` exists to catch one measure over. (A *monotone*
    /// decline is caught either way; the plateau is where the baselines diverge, so the
    /// plateau is what this fixture is.)
    #[test]
    fn a_renewal_that_thinned_once_and_stayed_thin_is_still_visible() {
        let roll: Vec<DidKey> = (0..2).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // A week-long first renewal, then three at a couple of minutes each.
        let rites: Vec<Rite> = [604_800_i64, 120, 120, 120]
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let start = 1_000_000 * (i as i64 + 1);
                rite_of(
                    &roll[0],
                    &[(start, 2), (start + span, 2)],
                    true,
                    AffirmationKind::ReAffirmation,
                )
            })
            .collect();
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { rites: Some(&rites), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.renewal_span_shortening, Some(true));
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn attendees_lost_mid_rite_are_reported_and_an_open_rite_is_not_graded() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // A completed rite that four attended and three finished, plus one abandoned.
        let mut lost = rite(&roll[0], &[(0, 4), (10, 4)], true);
        lost.movements[1].attending.truncate(3);
        let rites = vec![lost, rite(&roll[1], &[(20, 3)], false)];
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { rites: Some(&rites), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rite_attrition, 1);
        assert_eq!(r.rites_abandoned, 1);
        assert_eq!(r.erosion, ErosionLevel::Notice, "attrition warns; abandonment does not");

        // With no rite reading the measures say nothing rather than reporting health.
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.rites, 0);
        assert_eq!(r.rite_span_shortening, None);
        assert_eq!(r.rite_attrition, 0);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn nobody_holding_the_substrate_reaches_concern_alone() {
        let roll: Vec<DidKey> = (0..3).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // Everyone present, well-regarded, and holding nothing.
        let none = affirmed(&roll.iter().map(|d| (d.clone(), false, None)).collect::<Vec<_>>());
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { affirmations: Some(&none), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert!(r.nobody_holds_the_substrate);
        assert_eq!(r.erosion, ErosionLevel::Concern);

        // One inducted member is enough for the polity to be able to act.
        let mut some: Vec<(DidKey, bool, Option<Timestamp>)> =
            roll.iter().map(|d| (d.clone(), false, None)).collect();
        some[0] = (roll[0].clone(), true, Some(at_ts(0)));
        let st = affirmed(&some);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { affirmations: Some(&st), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert!(!r.nobody_holds_the_substrate);

        // An empty reading is *not evaluated*, not "nobody holds it".
        let empty = affirmed(&[]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { affirmations: Some(&empty), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert!(!r.nobody_holds_the_substrate);
    }

    #[test]
    fn a_thinning_witness_turnout_warns_but_never_escalates() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // Two inductions, 5 witnesses then 2.
        let thinning = trends(&[100], &[5, 2], &[]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { trends: Some(&thinning), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert!(r.witness_count_declined);
        assert_eq!(r.erosion, ErosionLevel::Notice);

        // A single induction is not a trend.
        let one = trends(&[], &[5], &[]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { trends: Some(&one), ..Default::default() },
            at(0),
            &Thresholds::default(),
        );
        assert!(!r.witness_count_declined);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn churn_carries_a_full_pair_because_it_is_a_rate_not_a_trend() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let report = |churn: &[i64]| {
            cohesion_report(
                &roll,
                &candidate(&members),
                false,
                &CohesionInputs { trends: Some(&trends(&[], &[], churn)), ..Default::default() },
                at(0),
                &Thresholds::default(),
            )
        };
        // One change over a roll of four: 0.25, at the notice bound.
        let r = report(&[0]);
        assert_eq!(r.churn_rate, Some(0.25));
        assert_eq!(r.erosion, ErosionLevel::Notice);
        // Two: 0.5, the concern bound. A boundary redrawn this fast is not a leading
        // indicator of anything — it is the boundary failing to hold.
        assert_eq!(report(&[0, 0]).erosion, ErosionLevel::Concern);
        // None at all: measured, and zero.
        let quiet = report(&[]);
        assert_eq!(quiet.churn_rate, Some(0.0));
        assert_eq!(quiet.erosion, ErosionLevel::None);
    }

    #[test]
    fn churn_outside_the_window_does_not_count() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        // Two changes, both long before the window opens.
        let old = trends(&[], &[], &[0, 0]);
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs { trends: Some(&old), ..Default::default() },
            at(100 * 86_400),
            &Thresholds::default(),
        );
        assert_eq!(r.churn_rate, Some(0.0));
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn omitting_the_trends_is_not_evaluated_rather_than_healthy() {
        let roll: Vec<DidKey> = (0..4).map(|_| did()).collect();
        let members: Vec<(DidKey, f64)> = roll.iter().map(|d| (d.clone(), 0.9)).collect();
        let r = cohesion_report(
            &roll,
            &candidate(&members),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(r.latest_gap_widening, None);
        assert!(!r.witness_count_declined);
        assert_eq!(r.churn_rate, None, "no reading is not a rate of zero");
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    // --- D16: sustained absence, the exception that proves the cap ------------

    const DAY: i64 = 86_400;

    #[test]
    fn absence_that_outlasts_a_day_and_was_watched_escalates() {
        // The escape hatch the attendance cap has promised since it was written: the
        // same absence, given a duration and a witness, does reach Concern.
        let m = did();
        let roll = vec![m.clone()];
        let seen = window(&[
            (0, vec![(m.clone(), Some(0))]),
            (DAY / 2, vec![(m.clone(), None)]),
            (DAY, vec![(m.clone(), None)]),
        ]);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs { window: Some(&seen), ..Default::default() },
            at(DAY + 1),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 1);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn one_stale_reading_with_silence_behind_it_escalates_nothing() {
        // The load-bearing case for the reading count. The span is identical to the
        // test above; the difference is that nobody was watching, which is evidence
        // about the monitor and not about the polity.
        let m = did();
        let roll = vec![m.clone()];
        let barely = window(&[(0, vec![(m.clone(), Some(0))])]);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs { window: Some(&barely), ..Default::default() },
            at(10 * DAY),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 0);
        // The snapshot component still speaks, and is still capped.
        assert_eq!(r.unattended, 1);
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    #[test]
    fn an_absence_shorter_than_the_span_stays_a_notice() {
        // A night shift, watched attentively: three readings, but only an evening.
        let m = did();
        let roll = vec![m.clone()];
        let evening = window(&[
            (0, vec![(m.clone(), Some(0))]),
            (3_600, vec![(m.clone(), None)]),
            (7_200, vec![(m.clone(), None)]),
        ]);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs { window: Some(&evening), ..Default::default() },
            at(7_200),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 0);
        assert_eq!(r.erosion, ErosionLevel::Notice, "still only the snapshot signal");
    }

    #[test]
    fn a_member_the_window_never_saw_is_not_a_sustained_absence() {
        // Not evaluated, never assumed absent: nobody watching is not evidence.
        let seen = did();
        let unseen = did();
        let roll = vec![seen.clone(), unseen.clone()];
        let w = window(&[
            (0, vec![(seen.clone(), Some(0))]),
            (DAY / 2, vec![(seen.clone(), None)]),
            (DAY, vec![(seen.clone(), None)]),
        ]);
        let r = cohesion_report(
            &roll,
            &guarded(&[
                (seen.clone(), 0.9, Some(did()), Some(at(0))),
                (unseen.clone(), 0.9, Some(did()), Some(at(DAY + 1))),
            ]),
            false,
            &CohesionInputs { window: Some(&w), ..Default::default() },
            at(DAY + 1),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 1, "only the member actually watched");
    }

    #[test]
    fn a_returning_guardian_ends_the_span() {
        let m = did();
        let roll = vec![m.clone()];
        let returned = window(&[
            (0, vec![(m.clone(), Some(0))]),
            (DAY / 2, vec![(m.clone(), None)]),
            (DAY, vec![(m.clone(), Some(DAY))]),
        ]);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(DAY)))]),
            false,
            &CohesionInputs { window: Some(&returned), ..Default::default() },
            at(DAY + 60),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 0);
        assert_eq!(r.erosion, ErosionLevel::None, "attended again, so nothing is raised");
    }

    #[test]
    fn omitting_the_window_is_not_evaluated() {
        let m = did();
        let roll = vec![m.clone()];
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), None)]),
            false,
            &CohesionInputs::default(),
            at(10 * DAY),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 0);
        // Without a record of observation, an absence can never be more than the
        // capped snapshot reading — which is the whole reason D16 exists.
        assert_eq!(r.erosion, ErosionLevel::Notice);
    }

    fn withdrawn(at_secs: i64) -> GroundStanding {
        GroundStanding {
            entries: vec![DisavowalEntry {
                disavower: did(),
                rationale: "I do not stand above these people".into(),
                disavowed_at: at(at_secs),
            }],
        }
    }

    #[test]
    fn a_withdrawn_ground_is_a_concern_on_its_own() {
        // Everything else about this polity is healthy: full reputation, nobody gone,
        // no lapsed affirmations. The ground has withdrawn, and that alone reaches the
        // level that crosses to kith-covenant — because unlike every other component
        // here there is nothing gradual to grade.
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let ground = withdrawn(500);
        let r = cohesion_report(
            &roll,
            &guarded(&members),
            false,
            &CohesionInputs { ground: Some(&ground), ..CohesionInputs::default() },
            at(600),
            &Thresholds::default(),
        );
        assert!(r.ground_disavowed);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn a_ground_still_standing_says_nothing() {
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let standing = GroundStanding::default();
        let r = cohesion_report(
            &roll,
            &guarded(&members),
            false,
            &CohesionInputs { ground: Some(&standing), ..CohesionInputs::default() },
            at(600),
            &Thresholds::default(),
        );
        assert!(!r.ground_disavowed);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn an_unsupplied_ground_reading_says_nothing_either() {
        // The same discipline every optional input here follows: a host that did not
        // ask gets silence, not false health and not a complaint.
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let r = cohesion_report(
            &roll,
            &guarded(&members),
            false,
            &CohesionInputs::default(),
            at(600),
            &Thresholds::default(),
        );
        assert!(!r.ground_disavowed);
        assert_eq!(r.erosion, ErosionLevel::None);
    }

    #[test]
    fn a_withdrawal_does_not_decay_back_to_none() {
        // There is no withdrawal of a withdrawal, and a disavowal is unprunable, so
        // an arbitrarily old one still reads. A polity whose ground has gone stays
        // legibly unmoored until it deliberately re-charters.
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let ground = withdrawn(1);
        let r = cohesion_report(
            &roll,
            &guarded(&members),
            false,
            &CohesionInputs { ground: Some(&ground), ..CohesionInputs::default() },
            at(50_000_000),
            &Thresholds::default(),
        );
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }

    #[test]
    fn the_ground_component_adds_no_threshold() {
        // It is event-triggered, not a ratio: there is no knob a captured roll could
        // tune to make a withdrawal read quieter.
        let t = Thresholds::default();
        let members: Vec<_> = (0..4).map(|_| (did(), 0.9, None, None)).collect();
        let roll: Vec<DidKey> = members.iter().map(|(d, ..)| d.clone()).collect();
        let ground = withdrawn(500);
        let inputs = CohesionInputs { ground: Some(&ground), ..CohesionInputs::default() };
        // Every threshold at its most permissive still cannot silence it.
        let permissive = Thresholds {
            attrition_notice: 1.0,
            attrition_concern: 1.0,
            ..t
        };
        let r = cohesion_report(&roll, &guarded(&members), false, &inputs, at(600), &permissive);
        assert_eq!(r.erosion, ErosionLevel::Concern);
    }


    // ---- D28: the watcher, not the filing --------------------------------------

    #[test]
    fn one_key_filing_three_times_does_not_escalate() {
        // **THE MEASURED HOLE.** This component is the only absence signal permitted
        // past the snapshot cap, and it reached `Concern` — the grade that crosses to
        // kith-covenant as a conduct record — on a count of FILINGS. A stranger with
        // one keypair, no relationship to the polity at all, could raise it.
        let m = did();
        let roll = vec![m.clone()];
        let solo = did();
        let points = [
            (0, vec![(m.clone(), Some(0))]),
            (DAY / 2, vec![(m.clone(), None)]),
            (DAY, vec![(m.clone(), None)]),
        ];
        let seen = window_by(&points, &[solo]);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs { window: Some(&seen), ..Default::default() },
            at(DAY + 1),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 0, "three filings, one watcher");
        assert_eq!(r.erosion, ErosionLevel::Notice, "the snapshot signal still speaks");
        assert_eq!(r.watchers.unwrap().distinct, 1);

        // The control: the same three filings from three keys DO escalate, so the
        // assertion above is about the count and not about the fixture.
        let seen = window(&points);
        let r = cohesion_report(
            &roll,
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs { window: Some(&seen), ..Default::default() },
            at(DAY + 1),
            &Thresholds::default(),
        );
        assert_eq!(r.sustained_absences, 1);
        assert_eq!(r.erosion, ErosionLevel::Concern);
        assert_eq!(r.watchers.unwrap().distinct, 3);
    }

    #[test]
    fn vouching_moves_the_lens_and_never_the_level() {
        // The locked decision, as a test. Gating an escalation on the monitor being
        // vouched for would let a captured polity vouch for nobody and thereby become
        // unwatchable, so the numbers move and the grade does not.
        let m = did();
        let roll = vec![m.clone()];
        let points = [
            (0, vec![(m.clone(), Some(0))]),
            (DAY / 2, vec![(m.clone(), None)]),
            (DAY, vec![(m.clone(), None)]),
        ];
        let seen = window(&points);
        let vouched = seen.observations[0].observer.clone();
        let ind = IndependentObservation {
            observers: vec![vouched],
            contested: vec![],
            unvouched: vec![],
        };
        let read = |observers| {
            cohesion_report(
                &roll,
                &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
                false,
                &CohesionInputs { window: Some(&seen), observers, ..Default::default() },
                at(DAY + 1),
                &Thresholds::default(),
            )
        };
        let bare = read(None);
        let with = read(Some(&ind));
        assert_eq!(bare.watchers.unwrap().vouched, 0);
        assert_eq!(with.watchers.unwrap().vouched, 1);
        assert_eq!(with.watchers.unwrap().unvouched(), 2);
        assert_eq!(bare.erosion, with.erosion, "the lens must not move the level");
        assert_eq!(bare.sustained_absences, with.sustained_absences);
    }

    // ---- D31: a grade says what it was not given -----------------------------

    /// Every optional reading, so the omission list has a zero to be measured against.
    // One parameter per optional reading, and that arity is the point: D31 measures
    // what a grade was *not* given, so this fixture has to supply every optional
    // input for the omission list to have a zero to be measured against. The obvious
    // collapse is `CohesionInputs`, which the body already builds from these — but
    // then a reading added to `CohesionInputs` would flow through untouched and go
    // silently unmeasured, which is the D31 failure itself. Keeping one parameter per
    // reading means adding one breaks this signature and someone has to look.
    #[allow(clippy::too_many_arguments)]
    fn every_reading(
        m: &DidKey,
        seen: &ObservationWindow,
        ground: &GroundStanding,
        rites: &[Rite],
        guardians: &GuardianState,
        affirmations: &AffirmationState,
        trends: &DecayTrends,
        observers: &IndependentObservation,
    ) -> CohesionReport {
        cohesion_report(
            &[m.clone()],
            &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
            false,
            &CohesionInputs {
                guardians: Some(guardians),
                affirmations: Some(affirmations),
                trends: Some(trends),
                window: Some(seen),
                ground: Some(ground),
                rites: Some(rites),
                observers: Some(observers),
            },
            at(0),
            &Thresholds::default(),
        )
    }

    #[test]
    fn a_grade_given_everything_omits_nothing() {
        // The negative control, and the invariant `en-host` is now pinned to: a lens
        // that fires on a complete call is worse than no lens.
        let m = did();
        let seen = window(&[(0, vec![(m.clone(), Some(0))])]);
        let g = GroundStanding { entries: vec![] };
        let rites = vec![rite(&m, &[(0, 2), (10, 2)], true)];
        let guardians = chartered(&[(m.clone(), Some(did()))]);
        let affirmations = affirmed(&[(m.clone(), true, Some(at(0)))]);
        let tr = trends(&[10], &[2], &[]);
        let ind =
            IndependentObservation { observers: vec![], contested: vec![], unvouched: vec![] };
        let r = every_reading(&m, &seen, &g, &rites, &guardians, &affirmations, &tr, &ind);
        assert!(r.omitted_readings.is_empty(), "{:?}", r.omitted_readings);
        assert!(r.silent_measures().is_empty());
        assert!(!r.blind_to_concern());
    }

    #[test]
    fn a_grade_given_nothing_names_all_seven_readings() {
        let m = did();
        let r = cohesion_report(
            &[m.clone()],
            &guarded(&[(m.clone(), 0.9, None, None)]),
            false,
            &CohesionInputs::default(),
            at(0),
            &Thresholds::default(),
        );
        assert_eq!(
            r.omitted_readings,
            ["guardians", "affirmations", "trends", "window", "ground", "rites", "observers"],
            "declaration order, and the names are interface"
        );
        assert!(r.blind_to_concern());
        // The consequence, not the cause: what a reader of the grade needs.
        assert!(r.silent_measures().contains(&"sustained_absences"));
        assert!(r.silent_measures().contains(&"nobody_holds_the_substrate"));
        assert_eq!(r.erosion, ErosionLevel::None, "and it still reads clean");
    }

    #[test]
    fn the_omission_that_hides_a_withdrawn_ground_says_so() {
        // The measured hole, as a regression. A polity whose chartered ground has
        // withdrawn grades `Concern` when the reading is supplied and `None` when it is
        // not, and until D31 nothing in the second report distinguished it from health.
        let m = did();
        let withdrawn_ground = withdrawn(0);
        let read = |ground| {
            cohesion_report(
                &[m.clone()],
                &guarded(&[(m.clone(), 0.9, None, None)]),
                false,
                &CohesionInputs { ground, ..Default::default() },
                at(DAY),
                &Thresholds::default(),
            )
        };
        let told = read(Some(&withdrawn_ground));
        let not_told = read(None);
        assert_eq!(told.erosion, ErosionLevel::Concern);
        assert_eq!(not_told.erosion, ErosionLevel::None);
        // Legibility, not prevention: the Concern is still lost. What is bought is that
        // the second report can no longer be read as a clean bill of health.
        assert!(not_told.omitted_readings.contains(&"ground"));
        assert!(not_told.silent_measures().contains(&"ground_disavowed"));
        assert!(not_told.blind_to_concern());
        assert!(!told.omitted_readings.contains(&"ground"));
    }

    #[test]
    fn only_a_concern_capable_reading_blinds_the_grade_to_one() {
        // The split is a property of the levels, not a judgement, so it is asserted
        // rather than described: rites are capped at `Notice` and the watcher lens is
        // read by no component at all, so neither absence hides an escalation.
        let m = did();
        let seen = window(&[(0, vec![(m.clone(), Some(0))])]);
        let g = GroundStanding { entries: vec![] };
        let rites = vec![rite(&m, &[(0, 2), (10, 2)], true)];
        let guardians = chartered(&[(m.clone(), Some(did()))]);
        let affirmations = affirmed(&[(m.clone(), true, Some(at(0)))]);
        let tr = trends(&[10], &[2], &[]);
        let ind =
            IndependentObservation { observers: vec![], contested: vec![], unvouched: vec![] };
        let full = CohesionInputs {
            guardians: Some(&guardians),
            affirmations: Some(&affirmations),
            trends: Some(&tr),
            window: Some(&seen),
            ground: Some(&g),
            rites: Some(&rites),
            observers: Some(&ind),
        };
        let grade_with = |inputs: &CohesionInputs| {
            cohesion_report(
                &[m.clone()],
                &guarded(&[(m.clone(), 0.9, Some(did()), Some(at(0)))]),
                false,
                inputs,
                at(0),
                &Thresholds::default(),
            )
        };

        let no_rites = grade_with(&CohesionInputs { rites: None, ..full.clone() });
        assert_eq!(no_rites.omitted_readings, ["rites"]);
        assert_eq!(
            no_rites.silent_measures(),
            ["rite_span_shortening", "renewal_span_shortening", "rite_attrition"]
        );
        assert!(!no_rites.blind_to_concern(), "rite erosion is capped at Notice");

        let no_observers = grade_with(&CohesionInputs { observers: None, ..full.clone() });
        assert_eq!(no_observers.silent_measures(), ["watchers.vouched", "watchers.contested"]);
        assert!(!no_observers.blind_to_concern(), "D28's lens is read by no level");

        let no_window = grade_with(&CohesionInputs { window: None, ..full.clone() });
        assert!(no_window.blind_to_concern(), "the one absence measure that escalates");

        for one in ["guardians", "affirmations", "trends", "ground"] {
            let mut inputs = full.clone();
            match one {
                "guardians" => inputs.guardians = None,
                "affirmations" => inputs.affirmations = None,
                "trends" => inputs.trends = None,
                _ => inputs.ground = None,
            }
            assert!(grade_with(&inputs).blind_to_concern(), "{one} carries a Concern step");
        }
    }
}
