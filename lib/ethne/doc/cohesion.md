*Previous: [The rite](transmission.md)*

# Watching for erosion

Communities rarely announce that they have stopped being communities. They keep
the forms and lose the substance, and the two come apart slowly enough that
nobody inside notices the day it happened.

The failure mode with a villain is the easy one. Somebody captures the offices,
somebody manufactures a crisis, somebody steers the admissions gate, and every
one of those leaves a trace because somebody had to act. The hard case has no
adversary at all. Attendance thins. Ceremonies get shorter. Members stop
re-affirming and nobody raises it. The guardians of the machines drift away from
their consoles. Nothing is decided, so nothing is recorded, and a polity that
has quietly stopped can look identical to one with nothing to report.

This chapter is the machinery for seeing that. It has three parts: a record of
what a monitor actually observed, a set of graded measures computed over those
observations and over the signed records of the polity itself, and the loop a
host runs to keep both current.

One rule governs the whole of it, and it recurs in several places below. A
measure that can only fire when somebody acts cannot see a polity that has
stopped acting. Absolute measures see sincere drift; act-triggered ones do not.

## What a monitor saw

Although it looks like a log line, an `Observed` record is evidence. It carries
the readings of one observer at one instant. It is perspectival evidence in the
same sense that the observer reports of the compact tier are, and three
properties follow from that.

It asserts nothing about the polity. No majority is involved, because an
observation is not a decision. And the observer need not be a member, since a
monitor that had to sit inside the roll could be silenced by expulsion.

The single rule is that it be signed by exactly the observer it names, which is
the same rule that keeps anybody from putting an observation in somebody else's
mouth.

A window read model turns those records back into a picture over a span,
reporting a per-member reading count and, for each member, an unattended span.
That last figure distinguishes *never observed*, meaning not evaluated, from
*observed but never attended*, meaning evaluated and found absent. Collapsing
the two would make a monitor that never ran look like a polity nobody is
watching over.

Two rules make the window sound.

The unattended span is clamped at the pruning horizon of the window, so that
pruning may only ever weaken a signal and never strengthen one. That clamp is
load-bearing rather than precautionary. The last-attested figure is a maximum
over the stamps in the window, so dropping the interval holding the freshest one
would start the absence earlier and make it *longer*, which is pruning
manufacturing an alarm.

And an attendance stamp may not postdate the observation reporting it, since a
monitor cannot have seen an attestation that had not yet happened. That rule is
what makes the clamp sound in the first place.

A trends model derives three things from the signed records of the polity alone,
with no provider involved: a widening affirmation gap, judged against the median
of prior gaps; a thinning induction witness turnout, never inferred from a
single point; and a roll churn rate. It borrows its shape from the ceremony-cost
analysis of the compact tier, hedges included.

## Forgetting, carefully

Observations accumulate forever and a log that only grows is a log that
eventually cannot be carried.

Exactly one entry class may be pruned, and it is the observation, for a precise
reason. An observation decides nothing, so dropping one cannot rewrite what the
polity resolved. Being inert is what makes it forgettable, and one predicate
that matches nothing else is read by the receipt, the authority check, and the
compaction alike.

An `ObservationsPruned` record names a horizon and carries a receipt, being how
many records went and a digest over their identifiers. The receipt exists
because a concern crossing to the compact tier rests on observations, and
evidence must not be able to vanish without a signature.

Two routes authorize a pruning, and the asymmetry between them is the whole
security of the mechanism. One effective charter keeper among the deciders
suffices, which is the delegated power that office otherwise has no record to
exercise, and only beyond a settling period of thirty days. The roll may prune
anything at all at a majority.

Destroying evidence is the move a captured party makes, so the recent record
stays out of the reach of any single custodian. That thirty-day constant is law
rather than a tunable threshold, since a floor that the party it constrains
could lower is not a floor.

Three further rules apply. A horizon must advance. The receipt is verified while
the dropped observations are still present and treated as beyond checking once
compaction has run, which is an honest limit rather than a rule that would flag
every historical pruning. And applying prunings returns a *new* archive rather
than mutating one.

*When to prune is a read model rather than a daemon.* A recommendation answers
whether to prune, to what horizon, and who may authorize it, and returns nothing
at all when there is nothing worth doing. It signs nothing, and the host feeds
the horizon back through the ordinary authoring path, because this tier never
reads a clock and the loop, like the storage backend, belongs to the host.

The safety floor is not the host to reconstruct, and it has two halves. A live
measure span, defaulting to one day, protects the span that a live measure reads
back over. A minimum witness count, defaulting to three, protects the witness
count it also needs. Both are required, because the clamp above preserves the
span and nothing preserves the count. Without the second, the same absence at
the same horizon grades as a concern at a dense observation cadence and as a
notice at a sparse one, which is a measure reporting the cadence rather than the
polity.

The count is per member, since the watcher set is, so a thinly watched member
can pin the horizon. That is the correct answer rather than a deadlock, because
the roll may still prune anything by majority. What is refused is *recommending*
it.

The retention default is the same settling period, which is the smallest
retention that both preserves every observation-based measure and leaves the
single-custodian route open. A *shorter* retention keeps more history and is
therefore more expensive to authorize rather than less, which reads backward
until the reason is stated. And a minimum dropped count, defaulting to one
hundred, exists because a pruning record is itself unprunable: pruning eagerly
at one leaves the log nearly three times larger than pruning at one hundred.

The recommendation reports every open route, cheapest first, and can only ever
propose a horizon the authority check would accept.

## The graded measures

A cohesion report grades strain into three levels, being none, notice, and
concern, computed over the chartered roll against the current candidate from the
provider. There are more than a dozen components, and the interesting part of
each is not what it measures but what it is allowed to conclude.

Firstly, *attrition, reputation decay, and floor breaches* compare the roll the
polity ratified against what the substrate currently shows. An opaque boundary
caps attrition at notice, since a provider that has stopped reporting a boundary
is not evidence that the boundary has shrunk.

Second, *guardians not present* is capped at notice outright. A snapshot cannot
tell a night shift from an abandoned polity, so the signal may warn and may
never escalate on its own.

Third, *transmission decay*, being members lapsed on re-affirming the substrate,
carries no such cap and may reach concern, because duration is already inside
the measure.

Lastly, *lost guardian edges* carries no cap either, for a different reason. It
compares what the polity ratified against what is observed, and only over
members the provider still sees, so a vanished member is attrition rather than
answerability collapsing.

Four measures exist only over time. A *widening affirmation gap* and a *thinning
witness turnout* are both capped at notice, because a trend is a leading
indicator whose consequence the affirmation component can already escalate, and
grading both would count one decay twice. *Roll churn* is a rate rather than a
trend and carries a full pair of thresholds.

*Sustained absence* is the one signal permitted past the attendance cap. A
guardian unattended across the recorded window for at least a configured span,
and seen by at least a configured number of *distinct observers*, reaches
concern.

Counting watchers rather than filings is the whole of that rule. Counting
observations instead would let one key file three times and clear a floor whose
entire justification is that somebody has been watching, so a stranger with one
keypair could manufacture the one escalation this component is trusted with
alone. Counting distinct observers is arithmetic rather than a judgement about
who they are.

Whether the watchers are independent *of each other* is a separate lens,
reported beside the escalation it qualifies and read by no component. That
asymmetry is deliberate. Gating an escalation on the monitor being vouched for
would let a captured polity vouch for nobody and thereby become unwatchable, and
an unvouched monitor is the ordinary state of an honest new one.

The two conditions do two different jobs, and the sentence is worth keeping: the
span is the absence, and the count is the evidence that somebody was watching
while it lasted. What lifts the snapshot cap is therefore a record rather than a
looser threshold.

*The unrated share* fixed a conflation the other measures rested on. Both
reputation measures are now computed over the members the provider actually
rated and say nothing when nobody is rated. Before that, an unrated roll was
indistinguishable from a failing one, and a freshly founded polity graded at
concern for being new. The unrated share is itself graded, capped at notice by
construction for the same reason attendance is, since a young cohort and a
provider that has stopped rating look identical. It exists so the two reputation
measures cannot go dark silently.

*A withdrawn substrate anchor* is unlike everything above it. Every other
measure grades how far something has drifted. This one grades a single signed
act by the one key entitled to make it, so it reaches concern alone and takes no
threshold at all. It is event-triggered, and it cannot decay back, since there
is no withdrawal of a withdrawal.

*The cost of the rite* is what no earlier measure could reach. A cheapening
ceremony is the latest completed rite being shorter than the first, measured
against the first rather than the median, because a uniform drift moves its own
recent history. A cheapening renewal is its sibling, split out because an
induction happens once per member and a renewal recurs, so one series would call
the difference between two members a drift. Attrition within a rite is the
polity that turned out for the naming of the substrate and had drifted away by
the submission of the polity itself. All three are capped at notice like the
other trends.

Abandoned rites are reported and deliberately not graded, since nothing in the
doctrine says how long a catechumenate may take.

And *nobody holding the substrate* reaches concern alone with no threshold,
because a polity that has expelled its way there can take no act at all: every
majority is a majority of the members who hold it.

The read models the report needs travel in one inputs structure of optionals,
which puts *not evaluated* into the type rather than into a convention. The
report never decides a split, which is a judgement the tier deliberately refuses
to make on its own.

## Where the log lives

Durable storage exists for a log that can be compacted, and the whole crate
exists for one branch: same generation means append a segment, and a changed
generation means rewrite in full.

Although the interface is one trait, two backends implement it: a file store
with one segment envelope per line, where an append is a write and a sync and a
full rewrite goes through a temporary file, a sync, and a rename that the
operating system makes atomic; and an optional database backend storing the same
segments as rows, so the two are differentially testable. The database earns its
place on the rewrite, which is one transaction rather than a temporary-file
dance.

The store owns the cursor, because the hazard it guards is exactly what happens
when a caller keeps one. A host that appends past a compaction loses the pruning
record and keeps the observations it dropped, and both verification passes still
report that the reloaded log is fine. That is the failure mode this tier fears
most, being silent rather than loud.

A torn tail left by an interrupted write is reported rather than skipped, and
recovery truncates it as a deliberate act, since silently dropping an
unparseable last line would discard a signed entry without saying so.

There is no clock in any of it. The loop stays with the host.

## The loop

A host cycle answers what a host owes on a cadence and when. A tick returns the
actions due, being persisting, a graded report, raising a concern when erosion
reaches that level, and a pruning recommendation when the observation log is
due.

It assembles *every* read model the grade can use, since a missing input makes a
component report nothing rather than complain. And the report now says so
itself. It names the readings a grade was not given, the measures that therefore
went quiet, and whether any of them could have escalated. That is what finally
distinguishes an erosion level of none from nobody having looked.

Closing that hazard properly meant pinning the grading path to omit nothing, so
that a caller off the sanctioned path is at least legible rather than silently
partial.

The tick decides and computes; the host signs and does the input and output. It
takes the current instant as a parameter like every other cadence in this tier,
so a year of host behavior is unit-testable in microseconds, and it authors
nothing, because raising a concern and pruning both need keys the host holds.

It covers the cadenced duties and only those. Membership, induction, office
conferral, and envoy authorization are deliberate acts of the polity rather than
scheduled ones.

A runnable reference host mirrors the demo of the compact tier, and its single
clock read is the only one in the tier outside a test. It runs a simulated
timeline by default so that it terminates and can be verified, with a follow
mode for the real loop.

It runs three monitors, rotated, and the reason is a finding rather than a
preference. The retention floor counts distinct watchers per member, so the
single monitor the example used to hold never cleared it, and its entire
compaction path was dead: four hundred observations to zero compactions, while
the restart section printed a compaction count as though it had run one. Writing
the mirror of that host in Python is what found it, which is the practical form
of a rule worth stating plainly: a demo is not a test.

## Further reading

- [Design decisions D1 to D34](design.md): D16 for observations and the
  time-based measures, D17 and D19 for pruning and the recommendation, D20 for the
  store and the host cycle, D28 for watchers rather than filings, and D31 for the
  omitted-readings report.
- [Erosion legibility](../../kith-covenant/doc/architecture/erosion-legibility.md):
  the same failure with no adversary, worked at the compact scale, and where the
  absolute-measure rule comes from.
- [What crosses to the compact](uplift.md): what a concern-level grade actually
  emits, and what it deliberately does not.

---

*Next: [What crosses to the compact](uplift.md)*
