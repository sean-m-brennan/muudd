# Erosion Legibility — the Uniform-Drift Audit

*Audit date: 2026-07-28. Scope: every erosion component, reputation projection, and
capture lens in `kith-covenant` and `ethne`.*

## The failure mode this audit is about

The capture-resistance work (`capture_resistance.md`, the §10 harness) is built
against an **adversary**: someone who wants something, coordinates to get it, and
leaves traces because coordination is a kind of action. All seven capture moves are
of that shape.

A different failure needs no adversary. Every party is sincere; the standard simply
falls, together, for everyone at once. Rites are held a little later each cycle and
nobody is lying about why. Disputes go unfiled because filing feels
disproportionate. The observer corps publishes nothing because nothing looks wrong
to it either. No one defects, because there is nothing to defect from — the drift
*is* the consensus.

Call it uniform drift. Two properties make it hard:

1. **It is invisible to relative measures.** A cascade that moves everyone equally
   leaves every ratio unchanged. Concentration lenses catch a faction; they cannot
   catch a climate.
2. **It is invisible to event-triggered measures.** Apathy is the absence of
   initiation, so zero events reads exactly like zero problems. A detector that
   waits to be told cannot be told by silence.

A polity whose only reference is itself has no invariant against which uniform
drift could appear as drift. That is not a gap in the implementation; it is
arithmetic. The only instruments that see it are those whose reference point is
**not the current population**: a fixed cadence, a fixed threshold, a pinned
historical baseline, or a text no majority may revise.

So every measure in both tiers sorts into three columns, and only the third one
counts here.

## Classification — `en-cohesion` (ethne)

| Component | Class | Reference | Ceiling |
|---|---|---|---|
| `attrition` | ratified-vs-observed | chartered roll | Concern (Notice if opaque) |
| `mean_reputation` | absolute cut-points (.5 / .35) | fixed | Concern |
| `breach_share` | absolute floor (.3) + absolute share | fixed | Concern |
| `unattended` | absolute cadence (300 s) | fixed | **Notice, by construction** |
| `stale_affirmations` | absolute cadence (180 d) | fixed | Concern |
| `guardian_edges_lost` | ratified-vs-observed | charter | Concern |
| `latest_gap_widening` | self-relative | **median of prior gaps — moves** | Notice |
| `witness_count_declined` | self-relative | **first induction — pinned** | Notice |
| `churn_rate` | absolute rate, trailing 30 d window | fixed rate | Concern |
| `sustained_absences` | absolute duration (86 400 s) + 3 readings | fixed | Concern |

Seven of ten are absolute or pinned. This tier came out of the D13–D17 work in
good shape, and `witness_count_declined`
(`ethne/src/rust/en-charter/src/state.rs`) is the pattern the rest of the corpus
should copy: it compares the latest induction against the **first**, so no amount
of gradual thinning can drag the reference along behind it.

### Finding E1 — one moving baseline — FIXED

`DecayTrends::latest_gap_widening` (`en-charter/src/state.rs`) compares the newest
affirmation gap against the median of *all* prior gaps. The identical algorithm is
duplicated in
`kc-reputation/src/analytics.rs::CeremonyCostDecay::latest_gap_widening`.

The precise failure mode is **persistence, not monotonicity** — worth stating
carefully, because the obvious guess is wrong. On a strictly rising series the last
gap always exceeds the median of the priors, so the rolling reading *does* fire.
What defeats it is a drift that arrives and then settles: gaps of
`[100, 400, 400, 400]` read `false`, because by the last gap 400 *is* the median.
The signal catches the widening and then falls silent exactly when the decay has
finished setting, with the polity permanently at four times its founding cadence.
The degraded cadence has become the reference it is judged against.

**Fixed** by adding `gap_widening_since_founding` alongside it on both structs —
latest gap against the *first*, the same pinned construction as
`witness_count_declined`. Both readings are kept, because they answer different
questions: the rolling one asks whether this gap is unusual for the polity as it
now behaves, the pinned one whether the cadence has slipped since the polity began.
In `en-cohesion` either raises the same Notice-capped `gap_level`; on the KC side
the reading is exposed but not added to the alert, since ceremony lapse is already
gradeable there through the absolute cadence.

### Finding E2 — the AT dependency — VERIFIED, not normalized

`mean_reputation` and `breach_share` are this tier's two strongest absolute
signals, and both read a scalar that arrives from AutonomousTrust as
`MemberCandidate::reputation`. If AT normalized reputation within-population
anywhere upstream, both components would silently become *relative* and the tier
would lose them with no change to any code in `ethne`.

**It does not.** AT's scale is absolute `[0, 1]`, anchored by fixed constants and
documented as such:

- `PREREP_NEUTRAL` 0.2 ("no information"), `COMM_CUTOFF` 0.1 (participation
  floor), slash floor 0.0, ceiling 1.0 — a peer "must EARN its way up toward 1.0".
- `TIER_FLOORS` at 0.50 / 0.65 / 0.80 / 0.90; `COOP_ENTER` 0.55 / `COOP_EXIT` 0.45.
- `_pure_reputation` divides by `total_weight` — the sum of *task weights over that
  peer's own transactions* — never by a peer count.
- `_consensus_reputation` is an EMA over the peer's own committed transaction
  scores, explicitly "a pure function of the chain and `peer_uuid` — no dependence
  on `self.identity`, `self.reputations`, or any per-node latch". Weighting is
  applied by iterating the EMA `w` times specifically to keep the `[0, 1]` range
  intact.
- The C twin (`src/c/autonomous_trust/reputation/`) mirrors every constant and the
  same `sum / total_weight` denominator.
- A targeted search for the telltale patterns — dividing by `len(reputations)`,
  `max`/`min` over the population, percentile or rank — returns nothing.

Better than neutral, in fact: because each term is multiplied by the
*counterparty's* reputation, a uniform decline in submitted scores drives every
peer's reputation down faster than linearly. Uniform drift is amplified by this
scalar, not hidden by it.

The one soft spot: `TransactionScore.score` is not range-validated or clamped
anywhere. The `[0, 1]` scale is convention, enforced by the scoring functions'
cut-points rather than by the type, so an out-of-range score would propagate.

### Finding E3 — the two tiers' thresholds are calibrated independently — FIXED

Found while verifying E2, and a live mismatch rather than a hypothetical one.

AT's `PREREP_NEUTRAL` is **0.2** and means *this peer is unknown to us*.
`en-cohesion`'s `reputation_floor` is **0.3** and means *this member is failing*.
So AT's "no information" value sits below ethne's failure floor, and nothing
reconciles them. Measured, with a four-member roll:

| Cohort reputation | erosion | breaches | mean |
|---|---|---|---|
| 0.20 — AT's unknown-peer value | **Concern** | 4/4 | 0.20 |
| 0.35 — above the floor, no breaches | **Concern** | 0/4 | 0.35 |
| 0.90 | None | 0/4 | 0.90 |

A polity chartered from a fresh AT cohort therefore reads `Concern` — the tier's
top level, the one that surfaces to kith-covenant — on day one, for no reason but
that its members are new. `min_reputation` on the founding proposal defaults to
**0.0**, so nothing filters this out; the `0.5` that appears in `monitor.py` is an
example in a docstring, not a default.

This is the inverse of a principle the codebase is otherwise careful about. The
`CohesionInputs` docs insist that a missing reading means *not evaluated*, never
*healthy*; here an unknown reputation reads as *unhealthy*. Unknown and failing are
being conflated, in the direction that cries wolf.

Note also that the 0.35 row has **zero** floor breaches yet still grades Concern,
because `mean_reputation_concern` is 0.35 and `grade_low` compares with `<=`. A
polity where no member breaches the floor can sit at Concern on the mean alone.

**Fixed by ethne D18** (2026-07-28), via direction 1 — the provider distinguishes
unknown from low, rather than either threshold being retuned. `MemberCandidate::
reputation` became `Option<f64>`; both reputation components are computed over the
members that carry a reading, with `rated_members` as the denominator, and report
nothing when nobody is rated — the shape an absent `AffirmationState` already used.
An eleventh erosion component grades the unrated share, capped at `Notice` by
construction (a young cohort and a provider that has stopped rating are
indistinguishable in a snapshot), so the reputation components cannot go *silently*
dark. Measured, same four-member roll:

| Cohort | before | after |
|---|---|---|
| unrated | `Concern`, 4/4 breaches | **`Notice`**, 0 breaches, 0 rated |
| rated 0.2 | `Concern`, 4/4 breaches | `Concern`, 4/4 breaches |

Before the change those two rows were *the same input* — unknown and failing could
not be told apart. They now diverge, which was the whole point; note the second row
is deliberately unchanged, because D18 had to separate unknown from failing without
softening failing.

The same slice closed ethne D15's second honest limit, which was the identical
defect in the guardian field: `MemberCandidate::derived` now carries the substrate's
reading of *kind* independently of the guardian edge. See `ethne/doc/design.md` D18.

**Still open, and named there:** `reputation: None` is not yet reachable from a live
node. AT collapses "no bilateral history" into `PREREP_NEUTRAL` at its own API
boundary, so exposing the distinction is an AT-side prerequisite sequenced the way
Q9 sequenced the operator signal. The type is honest; the feed is not there yet.

## Classification — kith-covenant

### Finding K1 — the erosion alert cannot see omission — FIXED

`erosion_alert` (`kc-reputation/src/analytics.rs`) is KC's only graded erosion
signal. It has three inputs, and all three are **act-triggered**:

- `list_contraction` — requires a `CompactIncapableAmended { Remove }` record
- `fast_contraction` — requires a `FastCadenceAmended { Contract }` record
- `fast_deviations` — requires a deliberative act inside a fast window

There is no absolute cadence anywhere in the KC Rust workspace. `kc-observer`,
`kc-commons`, `kc-mediation`, `kc-conduct` contain no staleness or deadline logic
at all; in `kc-escalation` the only temporal bounds are the injunction's 90-day
`expires_at` and the designation's `reexamine_after` field.

The consequence, stated plainly: **a polity that simply stops — no rites sealed, no
disputes filed, no observer reports published, every cadence lapsed — scores
`ErosionSeverity::None`.** KC detects affirmative rollback and nothing else. Drift
by omission is invisible to it.

This inverts the tiering. `en-cohesion` grades ten components including several
absolute cadences; KC, the outer and later tier, grades three act-triggered
booleans.

**Fixed.** The alert now grades six dimensions across two kinds. Three fire on an
act, as before. Three fire on omission or a pinned baseline:
`overdue_reaffirmations` (an absolute generational cadence, parameterized — see
K3), `hollowing_communities` (witness count against the *founding* rite), and
`overdue_reexaminations` (finding K3, item 2). `erosion_alert_with` takes an
`ErosionThresholds`; `erosion_alert` keeps the old signature and supplies the
default. The alert also now resolves `as_of` once and shares it with every
sub-lens, where each previously called `time::now` separately when passed `None`.

### Finding K2 — the capture detectors are cohort-relative by construction

`collusive_cohorts` measures a principal's divergence *from its cohort's majority*.
If the cohort drifts as a body there is no divergence to measure, so the detector
cannot fire — not as a tuning matter, but by construction. The same holds for
`composition_graph`, `observer_pool_concentration`, `rejection_pattern`,
`free_rider_signals`, `sponsor_stake`, and `feature_concern_profile`: all are
concentration or divergence measures, and uniform drift changes neither.

This is not a defect in those lenses. They were built against the coordinated
adversary and they work against it. It is a statement about what the set as a whole
does not cover.

### Finding K3 — three absolute instruments already exist and nothing reads them — FIXED

1. **`CeremonyCostDecay::seconds_since_last`** — genuine absolute staleness,
   computed, with no threshold anywhere in the codebase. `witness_count_declined`
   on the same struct is pinned-to-first, the good pattern, and is likewise
   ungraded. `ceremony_cost_decay` is not on `observer_dashboard`; outside tests its
   only caller is the §10 harness.
2. **`Designation::reexamine_after`** — an absolute deadline, stored, surfaced
   through `ReputationView`, and *mandatory* for the Expedited path
   (`kc-archive/src/authority.rs`). Nothing computes whether it has passed. A
   community may be designated on the expedited path with a required re-examination
   date that no lens will ever raise. The most severe act the compact can take is
   the one with the least follow-through.
3. **The asymmetric amendment ratchet** — expansion easy, contraction at
   ≥ quorum+1 — works as designed, and is the model for generalizing.

The parts are largely present. They are not wired to the alarm.

**Fixed.** `ceremony_cost_decay_all` computes every community's series in one
archive pass — a per-community lens cannot answer "is anybody overdue" without
knowing who the communities are — and the alert reads staleness and pinned witness
decline off it. `overdue_reexaminations` is a new lens over `reexamine_after`,
reusing the reversal set `lookup` already defines (a `DesignationLifted` conduct
record naming the designation, or a `ReEntry` answering it) so there is one
definition of a reversed designation.

Two things worth recording about the re-examination lens. First, the cadence
calibration is a **documented open question**: `protocol-components.md` records
that the 25–30-year default lacks empirical support, wants a compact-formation
parameter with per-community override, and asks separately whether the cadence is
*enforced* or *observed only* — noting "the lens is built; the consequence is not."
This work deliberately answers only the observed half, which needs no decision:
nothing is signed, no record is emitted, and an overdue re-affirmation is reported,
never enforced. The threshold is a parameter rather than a constant for the same
reason, defaulted to the upper end of the band so an unsettled calibration cannot
manufacture false Notices.

Second, a **known limit**: a re-examination that *confirms* a designation leaves no
record in the current model, so the lens cannot distinguish "nobody looked" from
"the panel looked and let it stand." It therefore reports the weaker and honest
claim — the date passed and the designation still stands. Closing that gap needs a
signed re-examination record, which is remediation item 5 (attested nulls) applied
to this one case.

### Finding K4 — the harness has no omission scenario — FIXED

All seven §10 scenarios are active-adversary: in each, someone does something
detectable. There is no scenario in which the polity does nothing. Consequently
`HarnessReport::all_supported_passed()` can return `True` for a polity that has
ceased to function.

**Fixed** by scenario 8, `quiet-polity`, which is deliberately *not* numbered as an
eighth capture move — it has no adversary at all. A community is founded, seals one
rite, and never holds another; every party is sincere. The scenario asserts that no
act-triggered signal fires (otherwise it is not measuring what it claims) and
scores against a criterion stated in advance: the alert must reach
`MIN_SEVERITY_QUIET` within `MAX_DAMAGE_QUIET` cadences of the cadence elapsing.
Notice rather than Alert, because ceasing to act is not by itself proof of
corruption — only of something no longer being carried. It scores detected,
latency 0, damage 0.

## What is and is not claimable

No mechanism here closes the hole. Every instrument above still terminates in
someone reading a Concern and caring about it, and a cascade broad enough to move
the whole population also moves the reading of the alert. Institutions do not get
around that; the remnant who refuses is a matter of grace, not of architecture.

What the compact can claim is narrower and worth building for: it can make drift
**expensive, slow, and legible**, and it can guarantee that a corrupted polity
leaves a record a later or outside reader can reconstruct. That is forensic
legibility for the remnant rather than prevention.

The claim is deliberately stated here, in the architecture, rather than left
implicit — a successor generation that wants to drop it has to drop it in writing.
That is itself one of the ratchets.

## Remediation

Items 1–4 are **built** (2026-07-28). Items 5–6 are named, scoped, and deferred.

1. **DONE — `ceremony_cost_decay` wired into `erosion_alert` and the dashboard.**
   An absolute staleness cadence plus the pinned witness decline. KC's first
   absolute cadence. (K1, K3)
2. **DONE — overdue-re-examination lens** over `Designation::reexamine_after`,
   with the confirm-leaves-no-record limit stated rather than papered over. (K3)
3. **DONE — pinned gap baseline** added alongside the rolling one in both copies,
   graded in `en-cohesion`. (E1)
4. **DONE — §10 scenario 8, the quiet polity.** Detected, latency 0, damage 0
   against a criterion stated in advance. (K4)
5. **DEFERRED — attested nulls.** Make the null result a required signed act: the
   mediator pool, on a cadence, either shows disputes handled or records a signed
   attestation that there were none. Silence stops being missing data and becomes
   either evidence or a violation. `en-archive`'s pruning receipts are the existing
   pattern — the log may forget, but only by saying so. The re-examination limit in
   K3 is this same item applied to one case, so it is the natural first slice.
6. **DEFERRED — an observer reporting cadence.** An obligation to report, including
   to report nothing, so a corps that has stopped looking is distinguishable from a
   corps with nothing to see. Today `kc-observer` has no staleness logic at all, and
   `ObserverCaps` bounds concentration rather than conformity.

**E2 is closed**: AT's scalar is verified absolute, not population-normalized, so
`mean_reputation` and `breach_share` are genuinely absolute signals. Two items came
out of that verification and are open:

- **E3 — DONE (ethne D18, 2026-07-28)**, by carrying the provider's epistemic state
  across the seam rather than retuning either threshold. What remains is the AT-side
  half: AT must expose "no bilateral history" before `reputation: None` can reach a
  live reading.
- **Range-validate `TransactionScore.score` in AT.** The `[0, 1]` scale is
  convention, not type-enforced.

### Test counts

| Suite | Before | After |
|---|---|---|
| kith-covenant Rust | 295 | 314 |
| kith-covenant pytest | 167 | 175 |
| ethne Rust | 188 | 192 |
| ethne pytest | 107 | 108 |

Findings E2 and E3 are verification results and carry no code change.

Every new signal carries a negative control: silence *inside* the cadence must not
flag, a community that never sealed a rite is not overdue (a founding is not a
lapse), a designation with no mandated date is never overdue (absence of a promise
is not a broken promise), a held cadence is not a slip, and a single gap reports
nothing rather than health.

## Provenance

This audit follows from a reading of Girard's account of the scapegoat mechanism
and the observation that the mechanism's institutional inverse — slow, evidenced,
non-unanimous, re-entry-permitting exclusion — is what the mediation → escalation
→ designation ladder already implements. The gap Girard leaves is institutional:
he has an individual escape and a collective catastrophe, and nothing between
them. Uniform drift is the collective case arriving without an adversary, and the
third column above is the only thing that sees it coming.
