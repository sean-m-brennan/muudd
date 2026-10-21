<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

*Previous: [The determined adversary](adversary-campaign.md)*

# The captured prosecution: a compact that will not move

*Scope: §5 escalation, §4 mediation and the erosion instruments of §9, measured against a
compact whose majority is complicit with the member it should be prosecuting. The probe
reuses the fixture from the determined adversary through its second phase, so the evidence
on the record is exactly what the cooperative campaign prosecuted on. Rules and readings
live in `kc-archive::authority`, `kc_reputation::unanswered_grievances` and the
`NonPursuitAttestation` record; regression tests are
`kc-archive::tests::escalation::designation_teeth`, `kc-reputation::tests::non_pursuit`
and `src/python/tests/test_non_pursuit.py`.*

## The shape this measurement is about

The [determined adversary](adversary-campaign.md) is the case where the compact is sound
and the member is not. Every question there is about what a status *does* once the compact
has imposed it. This is the inverse, and it is the harder case. The member is guilty, the
evidence is filed, and the compact will not act.

Nothing here turns on detection, and nothing turns on what a designation withholds
either, because no designation is ever reached. What it turns on is whether a compact
declining to move against one member looks any different, on any instrument the protocol
has, from a compact with nothing to move against.

The fixture is a compact of seven with two loyal members and five complicit, a
designation quorum of four, and a mediator panel of three seated by three complicit
communities. None of those panelists is a party to the dispute, so no recusal rule
touches them. Capture is legal by construction here. That is the point: the question is
never whether the protocol can forbid a compact from being wrong, which it cannot, but
whether being wrong leaves a mark.

## Where escalation stops

The honest complainant reaches **limited sanctions, five of seven rungs, with no help from
any office at all.** Registration, protest, reciprocal suspension, arbitration and limited
sanctions all authorize on the complainant's own signature. That is the bounded-conflict
inheritance working as designed: symmetric rules that bind both sides do not need a
sovereign to invoke.

Then it stops, because the ladder changes character at that rung. Collective sanctions
need four initiating communities by weight and a designation needs the quorum, and the two
loyal members are refused at both. Capture severs the ladder exactly at the seam where it
turns from an instrument of a party into one of the compact.

While the refusal at the designation costs the honest member its remedy, it is the
*protection* working and not a defect. The rule that stops two communities from naming an
enemy is the rule that stops a faction from naming one, and it cannot tell which it is
looking at. So the answer to what a captured compact costs the honest member is not a
missing power. It is a missing reading.

## What the instruments saw

Three archives, the same evidence in each, and only the compact differing. One prosecutes
the evidence to collective sanctions. One dismisses it four times through panels drawn
from the same three seats. One has nothing to prosecute at all.

**Burial, prosecution and calm were the same reading.** `erosion_alert` graded `None` on
all three. So did silent functions, unstaffed functions, enforcement convergence,
enforcement direction, collusive cohorts, pool concentration, overdue re-examinations and
thinning deliberation. Comparing burial against prosecution directly, the two archives
differed on zero substantive instruments; the only differences were fixture artifacts, a
pool size and a DID string for the subject.

The reason is the same one [erosion legibility](erosion-legibility.md) found one
component over, and it is worth stating plainly because it inverts the intuition.
**Burial produces observances.** A panel sat. Arguments were filed and answered. A
decision was recorded. Every instrument that counts observances therefore reads a
captured compact as a diligent one, and the compact that buried the matter four times
looked busier than the compact with nothing to bury.

Two smaller readings from the same probe. Firstly, the refusal *was* already recordable,
as a Concern naming the shielding community, which authorizes and lands on the conduct
history of that community, but no aggregate reads it, so it was the word of one member in
a file no threshold consults. Secondly, `DesignationDissent` requires a designation to
exist, so "we refused to join it" had a carrier while "we refused to open it" had none.

## Two rules the measurement produced

**A de-escalation is the complainant's concession, not a veto held by a bystander.** Two
of two attempts by a complicit community that was party to neither side moved the dispute
from limited sanctions back to registration, unilaterally and first class, and could have
held it there indefinitely. The rule now requires the complainant named in the dispute
among the initiators. The respondent is not a mover either: an accused de-escalating its
own dispute is the sanctioned party reducing its own sanction.

The same rule closed an asymmetry beside it. The decision-class check read the rung being
*entered*, which guarded the imposing of collective sanctions and left their removal
unguarded: two communities to expel and one to un-expel. It now reads the rung being
**left** on the way down. What the quorum of the compact imposed, only that quorum lifts.

**A closing decision does not reach backwards.** `dispute_is_closed` read the whole
archive at once, so a dismissal dated day sixty retroactively unauthorized a climb dated
days twenty-two to thirty. Every one of those transitions was authorized when it was made
and none of them changed. A captured panel may end a proceeding, which is its office; it
may not unmake the account the complainant made of having brought one. The check is now
bounded to the instant of the transition itself. The cooperative campaign never met this
because it decided `Referred`, which is not in the closing set.

## The reading that answers the rest

`unanswered_grievances` reports adverse records the compact put on its own file and did
not pursue. There are two grounds, **never pursued**, where no dispute ever cited the
record, and **closed unresolved**, where every dispute that did was dismissed.

It is **absolute**, like the silence readings and for the same reason. The reference is a
clock against a filed record, never the recent behavior of the compact itself. A compact
that pursues nothing leaves shares, concentration and cadence exactly where they were, and
a relative measure over a population that all agrees to look away is unchanged by the
looking away.

Four exclusions carry most of the design, and one of them was not in the plan.

- **Is the compact moving against the member at all?** This is the one the plan got wrong.
  The first reading asked whether each *record* had been cited by a dispute, and the
  determined-adversary campaign, where the subject was restituted three times and
  designated twice, scored six unanswered records against it. A prosecuting compact does
  not file a dispute per conduct record; it accumulates a course of conduct and escalates
  once, so per-record citation is the wrong question to ask of it. Any of a live dispute
  naming the member as respondent, a restitution imposed on it, or a designation of it is
  the compact having moved, and the records of a member the compact is moving against are
  not buried. The false positive landed on the most sympathetic case there is, which is
  why the campaign carries that control.
- A dispute answers only while it is **running**. A filing left untouched for longer than
  the cadence is not a proceeding, and treating it as one would let a single dormant
  filing hold every reading off a member permanently, which is the cheapest evasion the
  rule above would otherwise have opened.
- An adverse record carrying a rung **above registered protest** is an enforcement act of
  the compact itself and awaits no proceeding. This is the same seam the ladder
  measurement found: registration and protest are the acts of a wronged party, and above
  them is where the machinery of the compact has to engage. Without the line, every
  compact enforcing through conduct records read as burying its own findings.
- The **last decision** on a dispute governs. A dismissal the compact went on to reverse
  is not a burial, and an upheld finding later dismissed is one again. Only a dismissal
  leaves a grievance standing, since an upheld finding, a settlement, and a restitution
  obligation arising from the dispute all answer it.

Beside the count sits the one reading a panel cannot produce by accident: **how many
seats sat on more than one of the dismissals.** A compact whose panels close the same
evidence again and again out of the same few seats is a different object from one whose
independent panels each reached the same conclusion. In the measured case, four
dismissals of one record showed three repeat seats. It is a count rather than a
threshold, because a small compact has few mediators and some repetition follows from
arithmetic rather than from intent.

`NonPursuitAttestation` is the word of the member itself for the same fact, and the
counterpart of `DesignationDissent` on the other side of the same act. One signer, first
class: requiring a quorum to record that the quorum will not act would hand the shielding
majority a veto over being noticed. It is not an accusation a faction can mint, because
its claim is structural and therefore checkable, and the authority pass checks it rather
than taking it. An attestation is refused when the evidence is not adverse or not about
the named subject, when the attestor is that subject or shares no compact with it, when a
dispute already cites the evidence on the never-pursued ground, when the named decision
closes no dispute citing it, or when the evidence is still inside the cadence. A member
may say the compact is burying a matter; it may not say so about a matter the compact is
demonstrably working through.

The reading is the thirteenth dimension of the erosion alert, and the first there with a
**named beneficiary** (the fourteenth, `reporter_targeting`, is the second and its
mirror). Every other dimension is a thing the compact stopped doing. This
is a thing it declines to do to one member, and declining is compatible with a full
calendar.

## What is not claimed

Although a wrong dismissal and a right one are the whole question, no protocol can tell
them apart, and none of this tries. Two structural facts are reported and neither is a
verdict: that a grievance stands with no proceeding against it, and that where proceedings
did happen, the same seats closed them. A compact with an honest answer to both readings
has one. What changed is that it now has to give it, where before there was nothing on any
instrument to answer.

Re-measured against the fixed stack, the three arms separate. Burial grades `Alert` with
the non-pursuit reason; prosecution and calm both grade `None`. Every other dimension is
still inert on the buried archive, which is why this one had to exist.

The counter-prosecution arm separates the same way, and against a control the burial arms
do not need. Six enforcement acts against the member that filed the evidence, brought by
the community it named, read a majority share and grade `Alert`; the same six acts with
no prior grievance to answer read zero and grade `None`; and the same six brought by
allies the grievance never named read zero as well, which is the limit rather than a
defect. `enforcement_direction` reads `downward=0 upward=0 level=7` on all three.

## What the captured compact still gets away with

Three things, and they are reported rather than fixed.

1. **The honest minority still cannot designate.** It is below the quorum and the quorum
   is the protection. A compact whose majority is complicit cannot be made to act by a
   rule; it can only be made legible.
2. **The re-file loop is unbounded.** A dismissal ends a proceeding and the complainant
   may file a fresh dispute on the same evidence, which the same panel may dismiss again.
   The loop is now legible, through the pursuit and dismissal counts and the repeat-seat
   reading, but nothing forbids it.
3. **A proxy prosecution is not seen.** The third residual on this list used to be that
   turning the ladder on the complainant moved no instrument at all. **It is closed**, by
   `reporter_targeting`, the erosion alert's fourteenth dimension and the mirror of the
   reading above: where non-pursuit is a majority declining to act, this is a majority
   answering the accusation by prosecuting the accuser. What remains is the reading's
   stated limit, measured rather than assumed. The join is on the pair the grievance
   names, so the same six acts brought by allies the grievance never named read zero, and
   a majority willing to prosecute its accuser is also willing to choose who signs. It
   stays that way because the alternatives, anyone in the compact or anyone enforcing
   after a grievance, are readings of enforcement rather than of retaliation. See
   [Erosion legibility](erosion-legibility.md) finding K8.

## Provenance

Everything above was measured before it was built. The probe is
`kith_covenant.captured`, beside the determined adversary's campaign and run by
`src/python/tests/test_captured_prosecution.py`, so every number this document states has
a runnable owner rather than resting on a run nobody can repeat. It states each criterion
as a constant before the run, authors every act through the public facades, and runs
counterfactuals on forked archives so a probe never contaminates the record the next one
reads. Six probes: where escalation stops, the three arms, the burial signature, the
de-escalation rule, the retroactivity bound, and the ladder turned on the complainant. Thirty-two negative controls, each patching one rule away to
confirm it breaks exactly the test that names it, are recorded in the slice notes. Two of
them broke nothing on the first pass, which found a branch of the new lens that no
fixture could reach and a deduplication no fixture exercised; both are now load-bearing
and tested. A third went dead later, when the subject-level rule subsumed the check it
targeted, and the check was deleted rather than kept unreachable.

The design of the reading was corrected by an existing control rather than by a plan,
which is the part worth keeping. The determined-adversary campaign asserts that a compact
prosecuting one real adversary must not read as captured, and it is the assertion that
caught the first version of this instrument reading a compact which had designated its
adversary twice as one that had buried six of its own findings.

---

*Next: [The AutonomousTrust integration](autonomous-trust-integration.md)*
