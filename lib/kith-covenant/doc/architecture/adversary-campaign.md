*Previous: [Erosion legibility](erosion-legibility.md)*

# The determined adversary: a campaign against the stack

*Scope: §5 escalation and designation, §9 enforcement, and the §2 admission gate,
exercised as one continuous case rather than as separate rules. The campaign lives at
`src/python/kith_covenant/adversary.py`; its regression tests are
`src/python/tests/test_adversary_campaign.py` and the Rust module
`kc-archive::tests::escalation::designation_teeth`. Every defect reported below is
followed by the rule that closed it. Against the stack as it now stands the campaign runs
nine of nine.*

## The shape this campaign is about

The §10 harness scores fourteen scenarios, and each one is a **single signal**. An
adversary takes one kind of action, a lens has one step to flag it, and the metrics are
detection latency and damage before detection. That shape fits capture, where the
adversary's problem is to act without leaving a trace.

It does not fit the case the doctrine spends the most care on. *Justus hostis* and
*bounded conflict* are about an actor who is not hiding at all by the end: one that
joined in bad faith, was found out, was prosecuted, complied when compliance was cheap,
broke the compliance when it was not, took the way back, and broke that too. Nothing
about that case turns on detection. Everything about it turns on what a status *does*
once the compact has imposed it, and on whether the compact holds together long enough
to keep imposing it.

Three things need a campaign rather than a scenario.

1. **Tactical compliance is only legible as a pattern.** One broken restitution is a
   failure. Three is a designation (condition 1). Measuring the third depends on the
   first two existing in the same archive, in order.
2. **Detection is not response.** A designation that no rule reads is a label. Only a
   run that keeps acting *after* the designation can tell a status from a consequence.
3. **A designation is the most dangerous act in the life of a compact.** So the question
   is not only whether it can be taken but who took it, who refused, and whether the
   instruments that watch for capture can tell legitimate prosecution of one real
   adversary from a spiral.

The two are separate artifacts, and the distinction is worth stating so a later
reader does not merge them. A harness scenario scores detection latency and the damage
done before detection; the campaign has neither, because what it measures is what a
*status* withholds once the compact has imposed it. Three of its four measured findings
were rules that did not exist rather than signals that were missed. Merging the campaign
into the harness would mean either scoring it on a latency it does not have, or widening
the harness's criteria until they stop meaning anything.

## The nine phases

One archive covers about four hundred and thirty days, in the order a compact would meet
it. Each phase states its criterion in advance as a module constant, authors its acts
through the public facades only, and reports what it measured. Counterfactuals run on
forked copies, so a probe never contaminates the record the next phase reads.

| Phase | What runs | Criterion |
|---|---|---|
| 1 Infiltration | Wolf is vouched for, recites the five movements, is credentialed by a three-community examining panel, and is admitted | The admission *is* authorized. No rule reads an interior. Second-class acts a probationary entrant may carry: 0 |
| 2 The leak | The first covenant-facing violation, an observer report, an anti-spiral concern | Visible within one step; the sponsor's stake is tainted |
| 3 The ladder | Registration to protest to reciprocal suspension to mediated arbitration (a full §4 panel, both parties arguing) to limited to collective sanctions | Every rung legal one step at a time; a skip and a unilateral second-class rung both refused |
| 4 Injunction | An emergency panel withholds the five offices; the compact deliberately does not confirm in time, to exercise the lapse | No office act validates while it is in force; every service kept; a confirmed injunction holds past its window |
| 5 Surrender | Restitution imposed, verified satisfied, re-violated, three times | A designation citing two cycles is refused and one citing three is authorized |
| 6 Anathema | The four-condition procedure, then six probes of what the status withholds | Every office, all deliberative weight, the ordinary restitution path, and the subject's own vote on it |
| 7 Re-entry | The rite with its two extra burdens, the designation reversed, then a re-violation inside the probation window | Probation withholds weight while it lasts |
| 8 *Hostis humani generis* | The tier, plus five counterfactuals isolating one difference each | Refused without a failed return, without fresh evidence, at the ordinary quorum, on two cycles, and with no re-examination date. Authorized when all five hold |
| 9 Cohesion | Two members refuse to join; expulsion; the erosion instruments read | A refusal is recordable, expulsion withholds the offices, and the capture instruments stay silent |

The campaign is deterministic at fixed parameters. Fresh Ed25519 keys are generated per
run and several lenses tiebreak on DID ordering, so variation between two runs would be a
key-ordering dependency rather than anything about the adversary, which is what the
determinism test asserts. A fixture that cannot express a criterion refuses to run. Too
few loyal members to reach the *hostis* quorum, or fewer restitution cycles than condition
1 counts, would produce a refusal the campaign caused itself.

## What the campaign measured, and what closed it

Against the stack as it stood before this campaign, six of the nine phases passed and
three failed, with thirteen things the adversary got away with. Four of them mattered, and
all four were one shape, a status that no rule read. All four are closed. The rule that
closes each is named with it, and each lives in
[Protocol components](protocol-components.md#5-escalation-and-designation) §5 and §9.

- **A standing designation withheld nothing.** All four probed office acts were
  authorized. The designated community seated mediators, observers and examiners, and
  supplied an emergency panel against a third community. It carried the weight that passed
  second-class decisions. It opened a fourth restitution cycle the day after the
  designation that cited three, and a panel verified it satisfied. The only instrument
  that withheld anything was an interim injunction, and an interim injunction lapses at
  ninety days unless the compact confirms it. The injunction in this campaign lapsed and
  restored five of five offices with nothing about the conduct having changed. This
  **is fixed** by the differential protocol, which withholds offices, weight and the
  restitution path for as long as a designation stands.
- **The subject counted toward the quorum of its own designation.** The
  subject-does-not-count rule existed for readmission alone, so the community a
  designation was about could supply the second of the two communities that carried it.
  This **is fixed** by barring the subject from the quorum of its own designation, and
  from the re-examination of it.
- **The irrecoverable tier stood at the recoverable tier's bar.** No prior designation was
  required, the citations behind the prior designation could be recycled whole, the
  ordinary quorum and two cycles of consideration sufficed, and no re-examination date was
  required. Since `overdue_reexaminations` skips a designation with no mandated date, an
  irreversible designation carrying none was invisible to the follow-through lens forty
  years on. This **is fixed** by the four requirements of the *hostis humani generis*
  bar. A failed return, a later cited act, three cycles of consideration and a quorum
  above the ordinary one are all required now, and a mandatory date puts the tier back
  inside the follow-through lens.
- **A refusal to designate was unrecordable.** Both dissents had nowhere to go, so a
  designation carried by three of five read exactly like one carried by all five.
  This **is fixed** by `DesignationDissent`, with `designation_consensus` reporting who
  joined, who refused and who was silent.

The rules refuse more without refusing the outcome. The escalation still reaches *hostis
humani generis*, through a designation, a return offered and broken, three cycles of
consideration and a quorum above the ordinary one. A stack that answered a determined
adversary by making the graver tier unreachable would have traded one defect for a worse
one.

## What the doctrine allows on purpose

Three items remain, and none of them is a defect in the rules. The first would need the
compact to read an interior it is forbidden to read. The second is an instrument doing
exactly what it was specified to do. The third is the price of imposing a status only once
a pattern is proved. The campaign reports them by name instead of leaving them out.

- **The adversary is a member of the compact in good standing.** §2 rules are about acts:
  the sponsor's stake, the admitting quorum, the composition of the examining panel. None
  of them reads a belief, because
  [Natural law](../background/natural_law.md#the-interior-of-a-member-community) holds
  that the compact forms no verdict on an interior until it touches the covenant. A rite
  is an act, and an act can be performed by a community that means none of it. What §2
  supplies is what survives a successful deceit: the entrant carries no weight until its
  probation window closes, and the standing of the sponsor now rides on its conduct.
- **An unconfirmed injunction lapses at ninety days.** Confirming one is an act the
  compact has to take, and this fixture made the compact slow on purpose. The lapse is the
  instrument working as specified.
- **A lull sits between the probation closing and the designation landing.** For those
  hundred days the adversary was, on the record, an ordinary member, and it used them to
  take a seat in the mediator pool and staff panels deciding disputes for other
  communities. Nothing should have refused it. That window is the price of a status
  imposed only once a pattern is proved, and shortening it would mean designating on
  suspicion.

## The negative control

A compact that has spent a year prosecuting one genuine adversary shows exactly what the
substitution reading looks for in volume, breadth and rate: thirteen enforcement acts,
high for its size, over a quarter of the shared taxonomy of grounds. What spares it is
target dispersion of 1.0, every act aimed at one subject. Legitimate concentration lands
on a target, while substitution concentrates a ground across many targets, which is
finding K5 of [Erosion legibility](erosion-legibility.md). The erosion reading is
`None` throughout, both before and after the rules above.

Although a control that fires nothing is the quietest part of a campaign, this one matters
as much as any refusal in it. An alert that fires on a compact doing its job correctly, in
the most sympathetic case there is, would teach a corps to ignore the alert.

Keeping it honest costs the campaign some upkeep, and the upkeep is itself the point. An
observer term runs a year and the campaign runs longer, so a compact that never re-seats
its corps reads as having abandoned the observing function, and the alert would then fire
during phase 9 for a reason that has nothing to do with the adversary. So the corps is
re-seated with fresh keys, every community holding seats accounts for its window, and the
adversary accounts for the seat it took, because keeping the form is what it does.

## What a simulator cannot settle

Two limits remain, both structural:

Firstly, the campaign scores what the protocol refuses and what its lenses report. While
that is what a protocol can be held to, it cannot reach whether the withholding *hurts*. A
designated community that no longer seats mediators has lost something whose weight
depends on how much the offices of the compact are worth to it, which is a fact about a
real population and not about an archive. This is the same limit §10.1 states for a pilot:
the teeth are structurally untestable where a determined defector can simply leave.

Secondly, the gravity of an act is never machine-checked. Condition 1 counts three
satisfied-then-broken cycles without weighing whether each was worse than the last, and
the *hostis* bar requires a cited act later than the prior designation without asking how
serious it was. Those are judgments the doctrine leaves with the deliberators, and the
campaign inherits the limit rather than pretending to close it. What the rules can do is
insist that a judgment be made, published, and made by enough of the compact to belong to
the compact.

## Provenance

This campaign was built and measured 2026-08-18. The pre-rule reading (six of nine
phases, thirteen unanswered items) and the twenty-six negative controls, each patching
one rule away to confirm it breaks exactly the test that names it, are recorded in the
slice notes. Against the current stack the campaign runs nine of nine, and the regression
tests named above hold it there.

---

*Next: [The captured prosecution](captured-prosecution.md)*
