*Previous: [Whistleblower cryptography](whistleblower-crypto.md)*

# Erosion Legibility: the Uniform-Drift Audit

*Audit date: 2026-07-28. Scope: every erosion component, reputation projection, and
capture lens in `kith-covenant` and `ethne`.*

## The failure mode this audit is about

The capture-resistance work (`capture_resistance.md`, the §10 harness) is built
against an **adversary**, someone who wants something, coordinates to get it, and
leaves traces because coordination is a kind of action. All seven capture moves are
of that shape.

A different failure needs no adversary. Every party is sincere; the standard simply
falls, together, for everyone at once. Rites are held a little later each cycle and
nobody is lying about why. Disputes go unfiled because filing feels
disproportionate. The observer corps publishes nothing because nothing looks wrong
to it either. No one defects, because there is nothing to defect from. The drift
*is* the consensus.

Call it uniform drift. Two properties make it hard.

1. **It is invisible to relative measures.** A cascade that moves everyone equally
   leaves every ratio unchanged. Concentration lenses catch a faction; they cannot
   catch a climate.
2. **It is invisible to event-triggered measures.** Apathy is the absence of
   initiation, so zero events reads exactly like zero problems. A detector that
   waits to be told cannot be told by silence.

A polity whose only reference is itself has no invariant against which uniform
drift could appear as drift. Although a relative lens is the right instrument
against a faction, it is the wrong one here. That is not a gap in the implementation; it is
arithmetic. The only instruments that see it are those whose reference point is
**not the current population**, whether a fixed cadence, a fixed threshold, a
pinned historical baseline, or a text no majority may revise.

So every measure in both tiers sorts into three columns, and only the third one
counts here.

## Classification: `en-cohesion` (ethne)

| Component | Class | Reference | Ceiling |
|---|---|---|---|
| `attrition` | ratified-vs-observed | chartered roll | Concern (Notice if opaque) |
| `mean_reputation` | absolute cut-points (.5 / .35) | fixed | Concern |
| `breach_share` | absolute floor (.3) + absolute share | fixed | Concern |
| `unattended` | absolute cadence (300 s) | fixed | **Notice, by construction** |
| `stale_affirmations` | absolute cadence (180 d) | fixed | Concern |
| `guardian_edges_lost` | ratified-vs-observed | charter | Concern |
| `latest_gap_widening` | self-relative | **median of prior gaps, moves** | Notice |
| `witness_count_declined` | self-relative | **first induction, pinned** | Notice |
| `churn_rate` | absolute rate, trailing 30 d window | fixed rate | Concern |
| `sustained_absences` | absolute duration (86 400 s) + 3 readings | fixed | Concern |

Seven of ten are absolute or pinned, so the references in this tier are fixed,
chartered, and pinned. This tier came out of the D13-D17 work in good shape, and `witness_count_declined`
(`ethne/src/rust/en-charter/src/state.rs`) is the pattern the rest of the corpus
should copy: it compares the latest induction against the **first**, so no amount
of gradual thinning can drag the reference along behind it.

### Finding E1: one moving baseline (fixed)

`DecayTrends::latest_gap_widening` (`en-charter/src/state.rs`) compares the newest
affirmation gap against the median of *all* prior gaps. The identical algorithm is
duplicated in
`kc-reputation/src/analytics.rs::CeremonyCostDecay::latest_gap_widening`.

The precise failure mode is **persistence, not monotonicity**, and it is worth
stating carefully, because the obvious guess is wrong. On a strictly rising series
the last gap always exceeds the median of the priors, so the rolling reading *does*
fire. What defeats it is a drift that arrives and then settles. Gaps of
`[100, 400, 400, 400]` read `false`, because by the last gap 400 *is* the median.
The signal catches the widening and then falls silent exactly when the decay has
finished setting, with the polity permanently at four times its founding cadence.
The degraded cadence has become the reference it is judged against.

**The remedy** is `gap_widening_since_founding` alongside it on both structs,
comparing the latest gap against the *first*, the same pinned construction as
`witness_count_declined`. Both readings are kept, because they answer different
questions. The rolling one asks whether this gap is unusual for the polity as it
now behaves, and the pinned one whether the cadence has slipped since the polity
began.
In `en-cohesion` either raises the same Notice-capped `gap_level`; on the KC side
the reading is exposed but not added to the alert, since ceremony lapse is already
gradeable there through the absolute cadence.

### Finding E2: the AT dependency (verified, not normalized)

`mean_reputation` and `breach_share` are the two strongest absolute signals in
this tier, and both read a scalar that arrives from AutonomousTrust as
`MemberCandidate::reputation`. If AT normalized reputation within-population
anywhere upstream, both components would silently become *relative* and the tier
would lose them with no change to any code in `ethne`.

**It does not.** The AT scale is absolute `[0, 1]`, anchored by fixed constants
and documented as such. The anchors are neutral, floor, and ceiling.

- `PREREP_NEUTRAL` 0.2 ("no information"), `COMM_CUTOFF` 0.1 (participation
  floor), slash floor 0.0, and ceiling 1.0, where a peer "must EARN its way up
  toward 1.0".
- `TIER_FLOORS` at 0.50 / 0.65 / 0.80 / 0.90, with `COOP_ENTER` 0.55 and
  `COOP_EXIT` 0.45.
- `_pure_reputation` divides by `total_weight` (the sum of *task weights over the
  transactions of that peer*), never by a peer count.
- `_consensus_reputation` is an EMA over the committed transaction scores of that
  peer, explicitly "a pure function of the chain and `peer_uuid`", with no
  dependence on `self.identity`, `self.reputations`, or any per-node latch. Weighting is
  applied by iterating the EMA `w` times specifically to keep the `[0, 1]` range
  intact.
- The C twin (`src/c/autonomous_trust/reputation/`) mirrors every constant and the
  same `sum / total_weight` denominator.
- A targeted search for the telltale patterns (dividing by `len(reputations)`,
  `max`/`min` over the population, percentile or rank) returns nothing.

Better than neutral, in fact. Because each term is multiplied by the reputation
of the *counterparty*, a uniform decline in submitted scores drives the reputation
of every peer down faster than linearly. Uniform drift is amplified by this
scalar, not hidden by it.

The one soft spot is that `TransactionScore.score` is not range-validated or
clamped anywhere. The `[0, 1]` scale is convention, enforced by the cut-points in
the scoring functions rather than by the type, so an out-of-range score would propagate.

### Finding E3: the two tiers' thresholds are calibrated independently (fixed)

Found while verifying E2, and a live mismatch rather than a hypothetical one.

In AT, `PREREP_NEUTRAL` is **0.2** and means *this peer is unknown to us*. In
`en-cohesion`, `reputation_floor` is **0.3** and means *this member is failing*.
So the "no information" value of AT sits below the failure floor of ethne, and
nothing reconciles them. Measured, with a four-member roll, the readings are
these.

| Cohort reputation | erosion | breaches | mean |
|---|---|---|---|
| 0.20, the unknown-peer value in AT | **Concern** | 4/4 | 0.20 |
| 0.35, above the floor, no breaches | **Concern** | 0/4 | 0.35 |
| 0.90 | None | 0/4 | 0.90 |

A polity chartered from a fresh AT cohort therefore reads `Concern` (the top
level of the tier, the one that surfaces to kith-covenant) on day one, for no reason but
that its members are new. `min_reputation` on the founding proposal defaults to
**0.0**, so nothing filters this out; the `0.5` that appears in `monitor.py` is an
example in a docstring, not a default.

This is the inverse of a principle the codebase is otherwise careful about. The
`CohesionInputs` docs insist that a missing reading means *not evaluated*, never
*healthy*, and here an unknown reputation reads as *unhealthy*. Unknown and failing are
being conflated, in the direction that cries wolf.

Note also that the 0.35 row has **zero** floor breaches yet still grades Concern,
because `mean_reputation_concern` is 0.35 and `grade_low` compares with `<=`. A
polity where no member breaches the floor can sit at Concern on the mean alone.

**Fixed by ethne D18** (2026-07-28), via direction 1, in which the provider
distinguishes unknown from low rather than either threshold being retuned. `MemberCandidate::
reputation` became `Option<f64>`; both reputation components are computed over the
members that carry a reading, with `rated_members` as the denominator, and report
nothing when nobody is rated, the shape an absent `AffirmationState` already used.
An eleventh erosion component grades the unrated share, capped at `Notice` by
construction (a young cohort and a provider that has stopped rating are
indistinguishable in a snapshot), so the reputation components cannot go *silently*
dark. Measured on the same four-member roll, the change reads as follows.

| Cohort | before | after |
|---|---|---|
| unrated | `Concern`, 4/4 breaches | **`Notice`**, 0 breaches, 0 rated |
| rated 0.2 | `Concern`, 4/4 breaches | `Concern`, 4/4 breaches |

Before the change those two rows were *the same input*, and unknown and failing
could not be told apart. They now diverge, which was the whole point. The second
row is deliberately unchanged, because D18 had to separate unknown from failing
without softening failing.

The same slice closed the second honest limit of ethne D15, which was the
identical defect in the guardian field. `MemberCandidate::derived` now carries the
reading of *kind* held by the substrate, independently of the guardian edge. See `ethne/doc/design.md` D18.

**Still open, and named there.** `reputation: None` is not yet reachable from a
live node. AT collapses "no bilateral history" into `PREREP_NEUTRAL` at its own API
boundary, so exposing the distinction is an AT-side prerequisite sequenced the way
Q9 sequenced the operator signal. The type is honest; the feed is not there yet.

## Classification: kith-covenant

### Finding K1: the erosion alert cannot see omission (fixed)

`erosion_alert` (`kc-reputation/src/analytics.rs`) is the only graded erosion
signal in KC. It has three inputs, and all three are **act-triggered**.

- `list_contraction` requires a `CompactIncapableAmended { Remove }` record
- `fast_contraction` requires a `FastCadenceAmended { Contract }` record
- `fast_deviations` requires a deliberative act inside a fast window

There is no absolute cadence anywhere in the KC Rust workspace. `kc-observer`,
`kc-commons`, `kc-mediation`, and `kc-conduct` contain no staleness or deadline
logic at all, and in `kc-escalation` the only temporal bounds are the 90-day
`expires_at` on an injunction and the `reexamine_after` field on a designation.

The consequence, stated plainly, is that **a polity that simply stops (no rites sealed, no
disputes filed, no observer reports published, every cadence lapsed) scores
`ErosionSeverity::None`.** KC detects affirmative rollback and nothing else. Drift
by omission is invisible to it.

This inverts the tiering. `en-cohesion` grades ten components including several
absolute cadences, while KC, the outer and later tier, grades three act-triggered
booleans.

**Fixed.** The alert now grades seven dimensions across two kinds. Three fire on an
act, as before. Four fire on omission or a pinned baseline, namely
`overdue_reaffirmations` (an absolute generational cadence, parameterized; see
K3), `hollowing_communities` (witness count against the *founding* rite),
`overdue_reexaminations` (finding K3, item 2), and `thinning_deliberation`
(§4's argument count and deliberation span against the compact's *first* decision).
`erosion_alert_with` takes an `ErosionThresholds`; `erosion_alert` keeps the old
signature and supplies the default.

The seventh arrived with the argument records of §4, and only became measurable
then, since until a decision cited the deliberation it rested on there was nothing
to read. It is the one signal here that is **compact-wide rather than
per-community**, since a panel is drawn from non-party communities and decides for
the compact, so there is no single community whose deliberation it is. Both of its
readings are pinned to the first decision, and the reason is worth stating
precisely, because measuring it corrected a plausible assumption. **A monotone
decline is caught by any baseline, first or median alike.** The two diverge on a
*plateau*, a deliberation that thinned once and stayed thin, where the cheapened
form has become what the recent history of the compact calls normal. That is
the shape the pinned baseline exists for, and the negative control for it has to be a
plateau or it passes under both readings and proves nothing. The alert also now resolves `as_of` once and shares it with every
sub-lens, where each previously called `time::now` separately when passed `None`.

### Finding K2: the capture detectors are cohort-relative by construction

`collusive_cohorts` measures how far a principal diverges *from the majority of
its cohort*. If the cohort drifts as a body there is no divergence to measure, so
the detector cannot fire, not as a tuning matter but by construction. The same
holds for `composition_graph`, `observer_pool_concentration`, `rejection_pattern`,
`free_rider_signals`, `sponsor_stake`, and `feature_concern_profile`, since all are
concentration or divergence measures, and uniform drift changes neither.

This is not a defect in those lenses. Although they see nothing here, they were
built against the coordinated adversary and they work against it. What they read is
concentration, divergence, and coordination. It is a statement about what the set as a whole
does not cover.

### Finding K3: three absolute instruments already exist and nothing reads them (fixed)

1. **`CeremonyCostDecay::seconds_since_last`** is genuine absolute staleness,
   computed, with no threshold anywhere in the codebase. `witness_count_declined`
   on the same struct is pinned-to-first, the good pattern, and is likewise
   ungraded. `ceremony_cost_decay` is not on `observer_dashboard`, and outside
   tests its only caller is the §10 harness.
2. **`Designation::reexamine_after`** is an absolute deadline, stored, surfaced
   through `ReputationView`, and *mandatory* for the Expedited path
   (`kc-archive/src/authority.rs`). Nothing computes whether it has passed. A
   community may be designated on the expedited path with a required re-examination
   date that no lens will ever raise. The most severe act the compact can take is
   the one with the least follow-through.
3. **The asymmetric amendment ratchet** makes expansion easy and contraction at
   ≥ quorum+1, works as designed, and is the model for generalizing.

The parts are largely present, being computed, stored, and surfaced. They are not
wired to the alarm.

**Fixed.** `ceremony_cost_decay_all` computes the series for every community in one
archive pass (a per-community lens cannot answer "is anybody overdue" without
knowing who the communities are) and the alert reads staleness and pinned witness
decline off it. `overdue_reexaminations` is a new lens over `reexamine_after`,
reusing the reversal set `lookup` already defines (a `DesignationLifted` conduct
record naming the designation, or a `ReEntry` answering it) so there is one
definition of a reversed designation.

Two things are worth recording about the re-examination lens. First, the cadence
calibration is a **documented open question**. `protocol-components.md` records
that the 25-30-year default lacks empirical support, wants a compact-formation
parameter with per-community override, and asks separately whether the cadence is
*enforced* or *observed only*, noting that the lens is built while the consequence
is not. The lens deliberately answers only the observed half, which needs no
decision. Nothing is signed, no record is emitted, and an overdue re-affirmation
is reported, never enforced. The threshold is a parameter rather than a constant for the same
reason, defaulted to the upper end of the band so an unsettled calibration cannot
manufacture false Notices.

Second, a **known limit**. A re-examination that *confirms* a designation leaves
no record in the current model, so the lens cannot distinguish "nobody looked" from
"the panel looked and let it stand." It therefore reports the weaker and honest
claim, that the date passed and the designation still stands. Closing that gap needs a
signed re-examination record, which is the attested-nulls requirement applied
to this one case.

### Finding K4: the harness has no omission scenario (fixed)

All seven §10 scenarios are active-adversary, and in each of them someone does
something detectable. There is no scenario in which the polity does nothing. Consequently
`HarnessReport::all_supported_passed()` can return `True` for a polity that has
ceased to function.

**The remedy** is scenario 8, `quiet-polity`, which is deliberately *not* numbered
as an eighth capture move, because it has no adversary at all. A community is
founded, seals one rite, and never holds another, and every party is sincere. The
scenario asserts that no act-triggered signal fires, since otherwise it is not
measuring what it claims, and scores against a criterion stated in advance. The
alert must reach `MIN_SEVERITY_QUIET` within `MAX_DAMAGE_QUIET` cadences of the
cadence elapsing. Notice rather than Alert, because ceasing to act is not by itself
proof of corruption, only of something no longer being carried. It scores detected,
latency 0, damage 0.

### Finding K5: the erosion alert cannot see intensification (fixed)

K1 found that every dimension of the alert was act-triggered, so a polity that simply
stopped read as healthy. The remedy was six dimensions that fire on omission or on a
pinned baseline. **All ten of the resulting dimensions are satisfied by a polity that
keeps every form and performs each of them harder every year.**

That polity is not hypothetical and not adversarial. It seals its rites ahead of cadence
with witness counts that climb, amends nothing, shortens no fast, deliberates during none,
leaves no office empty and no designation unexamined, and answers more arguments in its
latest mediation than in its first. On every instrument the compact owns it reads current,
staffed, and improving. What changes is only what its enforcement is about. A
taxonomy of eight grounds narrows to one, and that one is applied with rising vigour across
a widening set of members. Nothing in the record marks the change, because nothing was
amended. The recitation is still read in full.

Measured before anything was built. Three compacts of eight communities each, thirty
enforcement acts apiece, identical in every ceremonial and constitutional respect and
differing only in what they enforced: one on a single ground against five members, one
across all eight grounds, one on a single ground against a single member. All three scored
`ErosionSeverity::None` with no reasons, and scored identically on `substrate_deference`,
`collusive_cohorts`, and `observer_pool_concentration`. Nothing the compact owned
distinguished a polity that had swapped its object from one that had not.

**Fixed.** Two dimensions, an eleventh and a twelfth, reading `enforcement_convergence` and
`enforcement_direction` over the conduct records. The first is coverage of the ground
taxonomy across a window, with the spread of subjects alongside it; the second is whether
enforcement lands on members holding reduced standing. Both are counts of what the compact
enforces rather than of how often, how promptly, or how ceremonially it does so. No record changed and no schema
moved, because `ConductRecord` already carried the ground, the subject, the filers, and the
ladder position. The argument for the dimension is
[substitution.md](../background/substitution.md); this finding is the audit-side statement
of it.

Coverage is **absolute** in the sense E1 and K1 established, and by a different route from
the cadences. Those measure against a clock. This measures against a fixed enumeration
written into the protocol, so a compact cannot improve its coverage by enforcing less, and
the open `Other` ground stays outside the denominator for the same reason. A denominator
that grew whenever a filer invented a category would let the compact supply its own
reference, and a compact narrowed to one invented ground would score perfect coverage of a
taxonomy of one.

Two calibrations are worth recording, and both were settled by arithmetic rather than
taste.

The **volume floor is a coupon-collector bound, not a data-sufficiency preference**.
Coverage over eight grounds is expected to be partial at small counts however evenly a
compact enforces. Twelve acts drawn evenly reach about 6.4 grounds, a coverage near 0.80
and clear of the 0.5 floor; eight acts reach about 5.5, a coverage of 0.66, close enough
that ordinary variance in a small young compact would trip the reading. Below the floor the
reading reports **untestable**, which is a different answer from healthy.

And **substantive re-engagement had to be excluded from what counts as enforcement**. It
sits on `LadderRung` and `dispute_ladder_rank` returns `None` for it, since it is the
ceremony-cost track rather than a dispute rung. Counting it would have read the most hopeful
record the compact keeps as one of its harshest.

### Finding K6: coverage alone punishes the compact with a difficult member (fixed)

Stated as its own finding because it is the failure the first design of K5 would have
shipped. A compact with one genuinely recurring offender files repeatedly on the same
ground for the honest reason, so it shows **high volume and low coverage, exactly like a
substituting compact**. A reading built on coverage alone fires on both.

**Fixed** by the dispersion of subjects, which is the discriminator rather than an
embellishment on the reading. Legitimate concentration lands on a *target*; substitution
concentrates on a *ground* across many targets. The negative control is built to that
shape: two compacts identical in volume and identical in coverage, separated only by
whether the enforcement lands on one member or on five, with the flag required on the second
and silence required on the first.

**A second measurement corrected the design itself, and this is the part no plan
anticipated.** The natural level to take the reading at is per community, and it is the
wrong level. Six communities each filing on one ground apiece score as converged on *every*
per-community reading, while the compact between them still enforces six of the eight
grounds. That is division of labour in an observer corps and not a narrowing of what the
compact holds, and a per-community trigger reports it as erosion. The dimension therefore
reads `enforcement_convergence` compact-wide, and `enforcement_convergence_all` is the
drill-down that names who, available only after the compact-wide reading has said there is
anything to name. This is the same shape as the `thinning_deliberation` finding under K1:
the level a signal is taken at is part of the signal.

### Finding K7: every dimension so far is a thing the compact stopped doing (fixed)

K1 divided the instruments into act-triggered and omission-triggered and found that a
polity which merely stopped read as healthy. K5 added intensification and found that a
polity which kept every form and narrowed the object read as healthy too. Both remedies
share an assumption that a third measurement broke, that erosion is something happening to
the compact as a whole.

A compact whose majority is complicit with one member erodes in neither of those ways. It
holds every rite, staffs every office, amends nothing, enforces the whole taxonomy against
everybody else, and declines to move against one. And **declining produces observances**. A
panel sits, arguments are filed and answered, a decision is recorded. So every instrument
that counts observances reads the captured compact as diligent, and the compact that buried
the same evidence four times looked busier than the compact with nothing to bury.

Three archives were measured with the same evidence, differing only in the compact. One
prosecuted it to collective sanctions, one dismissed it four times through panels drawn
from the same three seats, one had nothing to prosecute. All three graded `None` with no
reasons, on the alert and on every convergence, direction, concentration, silence, vacancy
and re-examination reading. Burial and prosecution differed on zero substantive
instruments.

**Fixed** by a thirteenth dimension, `unanswered_grievances`, absolute against a clock
like the K3 instruments and for the same reason. A compact that pursues nothing leaves
shares, concentration and cadence exactly where they were, so a relative measure over a
population that all agrees to look away is unchanged by the looking away. It is the first
dimension with a **named beneficiary**, which is what separates it from the other six
omissions. Those are things the compact stopped doing; this is a thing it declines to do
to one member, and declining is compatible with a full calendar.

The dimension's own design was corrected by an existing control rather than by a plan. The
first version asked whether each adverse *record* had been cited by a dispute, and the
determined-adversary campaign, whose subject was restituted three times and designated
twice, scored six unanswered records against it. A prosecuting compact accumulates a
course of conduct and escalates once rather than filing a dispute per record. Firstly, the
reading asks whether the compact is moving against the member at all, by way of a live
dispute naming it, a restitution imposed on it, or a designation of it. Secondly, a
dispute answers only while it is running, since a filing left dormant past the cadence
would otherwise hold every reading off a member for good.

Beside the count sits the reading a panel cannot produce by accident: how many mediator
seats sat on more than one of the dismissals that closed the same evidence. Independent
panels each reaching the same conclusion and one panel reaching it repeatedly are
different objects, and only the second is a signature. Although that repetition is the
signature, it stays a count rather than a threshold, because a small compact has few
mediators and some repetition follows from arithmetic.

The claim stays inside what K1 to K6 already conceded. No verdict is offered on any
dismissal, since no protocol can tell a wrong one from a right one. What the dimension
guarantees is that a compact declining to prosecute filed evidence can no longer do so
without a reading that says so, which is forensic legibility for the remnant and not
prevention. The full measurement, the two authority defects it turned up, and the member-side
attestation that carries the same fact are in
[captured-prosecution.md](captured-prosecution.md).

### Finding K8: the same capture has a second half, and it moves every instrument the right way (fixed)

K7 read a majority that will not act. The measurement that closed it left a residual
stated in the same document: **the complicit majority can also turn the ladder on the
member that filed the evidence, and when the probe did exactly that, zero instruments
moved.**

It is the harder half, because nothing here is an omission. The compact enforces, at
volume, at a member, on a ground the taxonomy names, through the record types the
protocol provides. Every instrument that counts observances reads it as vigour, and the
one lens that was built to say where enforcement lands could not say it either.
`enforcement_direction` buckets each act by **standing**, calling `probationary_at` on
the parties, and an honest complainant holds full standing throughout. Six acts against
the member that filed the evidence read `downward=0 upward=0 level=6`: enforcement
between equals.

Standing is not the axis a captured compact moves along. It does not need to reduce
anybody first; it already holds the weight to run the accuser up the ladder on its own
signature, which is the seam
[captured-prosecution.md](captured-prosecution.md) measured.

**Fixed** by a fourteenth dimension, `reporter_targeting`, whose axis is the *relation*
between an act and a prior grievance: whether the subject of this act is the community
that raised an earlier one about whoever is bringing this one. The join is on party
identity and time order rather than on citation, because `ConductRecord::references` is
optional and nothing obliges a counter-prosecution to cite what it answers. It reads all
four carriers that record a raised matter, being a conduct record, a dispute, a
non-pursuit attestation and an observer report, since narrowing it to one would leave
the evasion
the §8 whistleblower channel exists to close.

It is the second dimension with a **named beneficiary**, and the two are mirror images:
the thirteenth reads what the compact declines to do to one member, and this reads what
it does to whoever asked.

Three calibrations carry the design, and each answers a way the reading could have been
wrong.

- **Thresholded, not absolute.** Unlike K3's clocks, this fires on a share. Reciprocity
  is ordinary in a live compact, since two communities with a real quarrel file about
  each other, and a compact where every grievance ran one way would be the stranger
  object.
  The volume floor is three rather than the substitution twelve, because that one is a
  coupon-collector reading over eight grounds and this is a ratio over two parties.
- **Compact-wide, for the K5 and K6 reason.** A per-community reading converges on any
  compact with a division of labour, since the community that brings most matters is the
  one most acts are attributed to. `reporter_targeting_all` is the drill-down that names
  who, once the compact-wide reading has said there is something to name, and it credits
  an act to a co-signer only where the grievance named that co-signer.
- **One pair is a quarrel; several is a disposition.** The dimension compounds to
  `Alert` only when more than one pair is implicated. One pair carrying the whole of it
  is a quarrel between two communities, which the compact can see for itself.

The negative control is the determined-adversary campaign, exactly as it was for K5, and
it holds: nine of nine phases, zero of thirteen enforcement acts answering a raised
matter, and the erosion instruments silent through phase 9. The measurement also
corrected the plan that predicted it would be close. **The adversary never files an
adverse record about anybody**, so the campaign is a *silent* control on this axis rather
than a near miss, and the discriminating control had to be built where the shape actually
occurs: the same volume of enforcement, against the same member, by the same communities,
with no prior grievance for it to answer.

The reading's limit is measured rather than assumed. The join names a pair, so a **proxy
prosecution** brought by allies the grievance never named reads zero, and a majority
willing to prosecute its accuser is also willing to choose who signs. It stays as it is,
because the alternatives, anyone in the compact or anyone enforcing after a grievance,
are readings of enforcement rather than of retaliation.

### Finding K9: the denominator is the protocol's, and a compact can narrow what is not in it (fixed)

K5 gave the alert a reading of what enforcement is *about*, over the eight concrete grounds
[`CONCRETE_BEARINGS`] fixes. That denominator is what makes the reading absolute, and it is
also its limit. Eight grounds are coarse. A compact that quietly stops holding *hospitality
to strangers* while keeping *charity to members* moves `Charity` coverage not at all,
because both are the same bucket.

**Measured.** A compact that halved what it recited across three generational rites, with
rising witness counts, nothing amended, no office empty and enforcement spread across the
whole taxonomy, scored a perfect **1.0** on shared-taxonomy coverage at volume 32 and graded
`None` on all fourteen dimensions. Nothing in the audit read Movement 4 at all.

The obstacle was never the lens. Movement 4 carried the common virtue as an opaque block of
text, so there was no way to cite the third commitment of a recited list because there was
no list, and building one is protocol design. It is now a preamble and an ordered list of
commitments, each a key and its recited text, with the spoken form derived from the list so
the prose and the list cannot disagree. A conduct record carries the keys it is about beside
its bearing: the bearing names a shared ground, the keys name an undertaking.

**Fixed** by a fifteenth dimension, `commitment_coverage`, and a sixteenth,
`commitment_contraction`. The fifteenth is the first dimension graded against a denominator
the **compact supplied** rather than one the protocol fixed, and three things keep that from
being the relative measure K1 rules out.

- **Pinned to the first rite of each member.** A compact cannot raise its coverage by
  reciting a shorter list, because the denominator is what it undertook. The union only
  grows, since an admitted community adds its commitments and no departure record exists to
  take any away, so the reading gets harder as a compact ages and cannot be improved by any
  act of the compact.
- **A commitment taken on later counts outside the object.** The same rule that keeps the
  open ground out of the shared denominator: one that grew whenever a filer named something
  new would let the compact supply its own reference.
- **The volume floor scales with the object.** Coverage over `n` commitments is a
  coupon-collector quantity and here `n` varies per compact, so a constant floor would
  report the compact that undertook the most as narrowing forever. Two acts per commitment,
  and below it the answer is untestable rather than healthy.

The sixteenth exists because the fifteenth cannot see the move that makes a narrowing
legal. A compact whose enforcement has narrowed can bring its recitation down to match,
after which the smaller object is the one on the record, and a pinned denominator is by
construction blind to that. `commitment_contraction` reads the pinned baseline against the
latest recitation, the same shape as `hollowing_communities`, so a commitment taken up again
clears while the archive keeps both rites.

**An existing control corrected the design, for the second time in this document.** The
first version counted every enforcement act toward testability, and the determined-adversary
campaign, whose compact prosecutes one real adversary through thirteen acts that name no
commitment because nobody there had adopted the practice, scored coverage `0.0`, read as
narrowing, and graded `Alert` at eight of nine phases. That measures adoption of a citation
habit rather than the object, and it fired on the most sympathetic case there is.
Testability now counts only acts that cite something. Silence about the object is not
evidence about the object in either direction, and what it costs is reported as an uncited
share, which is the same posture `free_rider_signals` takes toward a sealed usage figure.

The evasion that leaves is stated rather than closed: a compact that narrows its object and
stops citing at the same time reads untestable. It reads untestable **with an uncited share
of one**, which is a fact on the record about a compact that has stopped saying what its
enforcement is about.

## What is and is not claimable

No mechanism here closes the hole. Every instrument above still terminates in
someone reading a Concern and caring about it, and a cascade broad enough to move
the whole population also moves the reading of the alert. Institutions do not get
around that, and the remnant who refuses is a matter of grace, not of architecture.

What the compact can claim is narrower and worth building for. It can make drift
**expensive, slow, and legible**, and it can guarantee that a corrupted polity
leaves a record a later or outside reader can reconstruct. That is forensic
legibility for the remnant rather than prevention.

The claim is deliberately stated here, in the architecture, rather than left
implicit, so that a successor generation that wants to drop it has to drop it in
writing.
That is itself one of the ratchets.

## Remediation

Eleven measures answer the findings above, and all eleven are carried.

1. **`ceremony_cost_decay` wired into `erosion_alert` and the dashboard.**
   An absolute staleness cadence plus the pinned witness decline. KC's first
   absolute cadence. (K1, K3)
2. **An overdue-re-examination lens** over `Designation::reexamine_after`. (K3)
3. **A pinned gap baseline** alongside the rolling one in both copies,
   graded in `en-cohesion`. (E1)
4. **§10 scenario 8, the quiet polity.** Detected, latency 0, damage 0
   against a criterion stated in advance. (K4)
5. **Attested nulls**, the null result as a signed act, meaning
   `ActivityAttestation` for the window of a standing function and
   `ReexaminationConclusion` for the K3 case, with
   `silent_functions` reading them against an absolute cadence.
6. **A reporting cadence for both standing functions**, mediation and observation
   alike, since the obligation and the failure are identical on the two sides.
7. **Enforcement coverage over a fixed ground taxonomy**, with the spread of subjects
   beside it, read compact-wide. The eleventh and twelfth alert dimensions, and the first
   two that fire on intensification rather than on an act or an omission. (K5, K6)
8. **§10 scenario 10, the substituted object.** Detected, latency 0, damage 1 against a
   bound of 2, with the recurring-problem control silent. (K5, K6)
9. **The relation between an enforcement act and a prior grievance**, read compact-wide
   over all four carriers of a raised matter. The fourteenth alert dimension, and the
   second with a named beneficiary. (K8)
10. **§10 scenarios 11 and 12, the buried grievance and the answered accuser.** Detected,
    latency 0, damage 1 against a bound of 1 and damage 0 against a bound of 2, each with
    a same-volume control silent. Scenario 11 closes a gap K7 left behind: the thirteenth
    dimension shipped with no §10 scenario at all, so K4's remedy held for the eighth
    dimension and not for the newest one. (K7, K8)
11. **An indexable commitment list in Movement 4**, with the compact's recited object
    pinned to the first rite of each member, read by the fifteenth and sixteenth alert
    dimensions, and **§10 scenario 13, the narrowed object**: detected, latency 0, damage 2
    against a bound of 3, with the shared-taxonomy reading silent at full coverage
    throughout and the whole-object control quiet. (K9)

Measure 2 used to carry the confirm-leaves-no-record limit as a stated limitation.
**It is closed.** A re-examination that confirmed a designation left no trace, so a
panel that looked at the mandated date and let the designation stand read exactly
like a compact that had forgotten, forever. The lens punished the diligent panel,
which is worse than a missed signal. `ReexaminationConclusion` is the attested null of the
severest act the compact can take. A confirmation answers the deadline it was held for
and sets the next one, and two rules keep the record from evading what it records. It
may not be dated before the deadline it answers, and a confirmation must name the
next date. Without the second, one confirmation would discharge a generational obligation
once and for all.

**Measuring for measures 5 and 6 found a third hole neither of them named.** A compact
with **zero** active mediators and **zero** active observers for forty-three years,
keeping its re-affirmation rites perfectly, read clean on every instrument.
`deliberative_capacity` reads the member roll and probation, the pool lenses report
shares of an empty pool without remarking that it is empty, and no alert dimension
looked at staffing at all. An unexercised function and an abolished one are different
failures, so `unstaffed_functions` is the second reading and it carries its own
negative control: a function *never* staffed is absent from it, because declining to
open an office is not abandoning one.

Two calibrations are worth recording, because both were wrong on the first pass and
measuring corrected them.

The **function cadence must sit inside the shortest roster term**. At one year it
exactly equalled the observer term limit in §8, so an observer corps could never be
silent for a full cadence while staffed. Its seats expired first and it read as
unstaffed instead, making the silence reading unreachable for that function by
construction. The fix is the cadence, not the term, since the term limit is doctrine
(an observer must return home) and the cadence is a calibration. Ninety days,
matching the rotation period in §8.

And a **long vacancy compounds on duration exactly as a long silence does**. Graded
without that rule, a compact whose office had stood empty for years scored *below* one
still seated and merely quiet, which is backwards: the vacancy is the stronger claim
and must not carry the weaker grade.

One thing the design expected and did not find: a rule stopping the two function
readings from double-counting. They cannot. `silent_functions` reads only functions with
a seat active at `as_of` and `unstaffed_functions` only functions with none, so a single
lapse is exactly one dimension and the guard written for it was a rule against an
impossible state. The invariant lives in the lens definitions and in the test that pins
it, rather than in a redundant check.

**The one refusal.** Everything above is reported, never enforced, for the reason the
audit itself gives. A compact that has stopped acting offers no act to refuse, and a
gate that fires at silence would punish a quiet compact for a missing form. The
exception is where there *is* an act. A designation whose mandated re-examination has
come and gone unanswered may not be **cited** by a further escalation act, neither a
new designation nor a re-engagement. The most severe act the compact can take cannot
rest on an obligation the compact has itself let lapse. Once the obligation is answered
the citation is good again, so the gate is about follow-through and not about age.

**E2 is closed**: AT's scalar is verified absolute, not population-normalized, so
`mean_reputation` and `breach_share` are genuinely absolute signals. **E3** is
answered in ethne D18, by carrying the epistemic state of the provider across the
seam rather than retuning either threshold, and its AT-side half, and a range check on
`TransactionScore.score`, fall to `autonomous_trust` and are listed in
[`TODO.md`](../../TODO.md). Both findings are verification results and carry no
KC-side code change.

### Negative controls

Every signal carries a negative control: silence *inside* the cadence must not
flag, a community that never sealed a rite is not overdue (a founding is not a
lapse), a designation with no mandated date is never overdue (absence of a promise
is not a broken promise), a held cadence is not a slip, and a single gap reports
nothing rather than health. The function signals add three of their own: a function
never staffed is not abandoned, an active or open-ended term is not a vacancy, and a
designation cited *inside* its window passes the gate, so the refusal fires on lapse
rather than on citation.

Scenario 9's control is the sharpest of them, because it is the whole reason the
attested null exists. A quiet pool may simply have had no disputes, so the scenario
runs a second compact that attests each window as it closes and requires the flag on
the silent compact **and** silence on the attesting one. A cadence signal that fired on
both would be measuring the calendar rather than the compact.

The substitution dimensions carry six of their own, and scenario 10 runs the paired-compact
form for the first two.

1. **The recurring problem.** A compact enforcing the same volume on the same single ground
   against one member must not flag, and the compact spreading that enforcement across five
   must. Same volume, same coverage, and the target dispersion is the only thing between
   them. This is K6, and it is the control the whole dimension turns on.
2. **Specialisation.** Six communities each filing on one ground apiece must not flag,
   although every per-community reading is converged. This is the control that moved the
   trigger from per-community to compact-wide.
3. **Sparsity.** A compact below the volume floor reports untestable rather than firing, so
   a young compact is not reported as a narrowing one.
4. **Decay.** A compact whose enforcement has stopped falls out of the window and reads
   zero volume, which is not zero coverage. It fires the omission dimensions and not this
   one, which is what keeps the third mode distinct from the first.
5. **Direction at full members.** Enforcement aimed at members holding full standing never
   fires the direction dimension at any volume, since some downward skew is what probation
   exists to license.
6. **One novel ground.** A single `Other` filing among a covered taxonomy leaves coverage at
   1.0 and is silent, while a sustained off-taxonomy share scores coverage 0.0 and fires on
   the ordinary reading. The control holds by construction rather than by a rule, since
   `Other` is outside the denominator.

The reporter-targeting dimension carries five, and two of them broke nothing on their
first pass, which is what found the two rules no fixture reached.

1. **The pair the grievance names.** Enforcement against a member that raised a matter
   about *somebody else* must not fire. Removing the join broke no test at all until the
   probe grew an arm for it, so the reading's own limit had been asserted in prose and
   measured nowhere.
2. **One pair or several.** The compounding condition had the same problem: no fixture
   implicated two pairs, so `Alert` was unreachable and removing the rule changed
   nothing. Two pairs at the same share now separate a quarrel from a disposition.
3. **Ordinary reciprocity.** A minority of acts answering a raised matter must not fire,
   or the reading is a count of enforcement rather than of retaliation.
4. **The volume floor.** Three acts is the floor and two is not, so a compact with one
   bad month is not read as a captured one.
5. **The drill-down credit.** A co-signer the grievance never named does not inherit the
   reading, so `reporter_targeting_all` names who rather than everyone present.

The recited-object dimensions carry ten, and one of them broke nothing on its first pass.

1. **The pin.** Reading the object from the latest rite rather than the first breaks four
   tests at once, which is the whole absoluteness argument in one control.
2. **The seated filter.** A community reciting outside the compact contributes nothing to
   the denominator, the same scoping the citation rule takes.
3. **Off-object keys.** A commitment taken on later counts outside the object and never
   into the numerator.
4. **Testability against cited acts.** The control the campaign supplied: counting uncited
   acts reads a compact that has not adopted the practice as one that has abandoned its
   object.
5. **The scaled floor.** A constant floor reports the compact that undertook the most as
   narrowing forever.
6. **The contraction baseline.** Comparing the latest rite against itself finds nothing,
   which is what a pinned baseline is for.
7. **The whole-object control.** A compact still enforcing everything it recited must never
   flag at any volume.
8. **A broadened list is not a contraction.**
9. **The citation resolves.** A key nobody recited is refused, and one recited only outside
   the compact is refused with it.
10. **The citation's time bound.** This is the one that broke nothing at first: the compact
    scoping was doing all the work in the fixture, because the later reciter was also a
    later member. Rewritten so a **seated** member takes the commitment on afterwards, the
    bound is the only thing refusing the earlier citation.

## Provenance

This audit follows from a reading of Girard's account of the scapegoat mechanism
and the observation that the institutional inverse of the mechanism, an exclusion
that is slow, evidenced, non-unanimous, and re-entry-permitting, is what the
mediation → escalation → designation ladder already implements. The gap Girard
leaves is institutional. He has an individual escape and a collective catastrophe,
and nothing between them. Uniform drift is the collective case arriving without an adversary, and the
third column above is the only thing that sees it coming.

The substitution findings come from the other half of the same reading, and the corpus
carries both accounts already without having named them as a pair. Girard supplies
revelation, and the third anti-spiral feature is the countermeasure built from it. Wink
supplies renunciation, and the fifth anti-spiral feature is the countermeasure built from
that. Substitution is the case where neither fires. Revelation has nothing to expose,
since nothing anomalous is on display and every filing is correct on its face. Renunciation
is satisfied, since the compact has claimed no ultimacy and kept every list and every fast
it undertook to keep. What is left is a compact enforcing a smaller object than it recites,
in perfect and sincere good order, with nothing hidden, forged, or coordinated anywhere in
the record. That is why the reading which catches it had to be a count of what the
enforcement is *about* rather than a search for anything wrong with how it was done.

---

*Next: [The determined adversary](adversary-campaign.md)*
