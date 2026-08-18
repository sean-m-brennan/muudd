*Previous: [Erosion legibility](erosion-legibility.md)*

# The Determined Adversary: a Campaign Against the Stack

*Scope: §5 escalation and designation, §9 enforcement, and the §2 admission gate,
exercised as one continuous case rather than as separate rules. The campaign lives at
`src/python/kith_covenant/adversary.py`; its regression tests are
`src/python/tests/test_adversary_campaign.py` and the Rust module
`kc-archive::tests::escalation::designation_teeth`.*

## The shape this campaign is about

The §10 harness scores twelve scenarios, and each one is a **single signal**. An
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
   failure. Three is a designation (condition 1). A measure of the third depends on the
   first two existing in the same archive, in order.
2. **Detection is not response.** A designation that no rule reads is a label. Only a
   run that keeps acting *after* the designation can tell a status from a consequence.
3. **A designation is the most dangerous act in the life of a compact.** So the question
   is not only whether it can be taken but who took it, who refused, and whether the
   instruments that watch for capture can tell legitimate prosecution of one real
   adversary from a spiral.

## The nine phases

One archive, about four hundred and thirty days, in the order a compact would meet it.
Each phase states its criterion in advance as a module constant, authors its acts through
the public facades only, and reports what it measured. Counterfactuals run on forked
copies, so a probe never contaminates the record the next phase reads.

| Phase | What runs | Criterion |
|---|---|---|
| 1 Infiltration | Wolf is vouched for, recites the five movements, is credentialed by a three-community examining panel, and is admitted | The admission **is** authorized. No rule reads an interior. Second-class acts a probationary entrant may carry: 0 |
| 2 The leak | The first covenant-facing violation, an observer report, an anti-spiral concern | Visible within one step; the sponsor's stake is tainted |
| 3 The ladder | Registration to protest to reciprocal suspension to mediated arbitration (a full §4 panel, both parties arguing) to limited to collective sanctions | Every rung legal one step at a time; a skip and a unilateral second-class rung both refused |
| 4 Injunction | An emergency panel withholds the five offices; the compact does not confirm in time | No office act validates while it is in force; every service kept; a confirmed injunction holds past its window |
| 5 Surrender | Restitution imposed, verified satisfied, re-violated, three times | A designation citing two cycles is refused and one citing three is authorized |
| 6 Anathema | The four-condition procedure, then six probes of what the status withholds | Every office, all deliberative weight, the ordinary restitution path, and the subject's own vote on it |
| 7 Re-entry | The rite with its two extra burdens, the designation reversed, then a re-violation inside the probation window | Probation withholds weight while it lasts |
| 8 *Hostis humani generis* | The tier, plus five counterfactuals isolating one difference each | Refused without a failed return, without fresh evidence, at the ordinary quorum, on two cycles, and with no re-examination date. Authorized when all five hold |
| 9 Cohesion | Two members refuse to join; expulsion; the erosion instruments read | A refusal is recordable, expulsion withholds the offices, and the capture instruments stay silent |

The campaign is deterministic at fixed parameters. Fresh Ed25519 keys are generated per
run and several lenses tiebreak on DID ordering, so variation between two runs would be
a key-ordering dependency rather than anything about the adversary, which is what the
determinism test asserts. A fixture that cannot express a criterion refuses to run: too
few loyal members to reach the *hostis* quorum, or fewer restitution cycles than
condition 1 counts, would produce a refusal the campaign caused itself.

## What the campaign measured

Six phases passed and three failed, with thirteen things the adversary got away with.
Four of them mattered, and all four were one shape: a status that no rule read.

- **A standing designation withheld nothing.** All four probed office acts were
  authorized. The designated community seated mediators, nominated observers, seated
  examiners, and supplied an emergency panel against a third community. It carried the
  weight that passed second-class decisions. It opened a fourth restitution cycle the day
  after the designation that cited three, and a panel verified it satisfied. The only
  instrument that withheld anything was an interim injunction, and an interim injunction
  lapses at ninety days unless the compact confirms it. The campaign's own injunction
  lapsed and restored five of five offices with nothing about the conduct having changed.
- **The subject counted toward the quorum of its own designation.** The
  subject-does-not-count rule existed for readmission alone, so the community a
  designation was about could supply the second of the two communities that carried it.
- **The irrecoverable tier stood at the recoverable tier's bar.** No prior designation
  was required, its predecessor's citations could be recycled whole, the ordinary quorum
  and two cycles of consideration sufficed, and no re-examination date was required.
  Since `overdue_reexaminations` skips a designation with no mandated date, an
  irreversible designation carrying none was invisible to the follow-through lens forty
  years on.
- **A refusal to designate was unrecordable.** Both dissents had nowhere to go, so a
  designation carried by three of five read exactly like one carried by all five.

The rules that answer these are in
[protocol-components.md](protocol-components.md#5-escalation-and-designation) §5 and
§9. With them in place the campaign runs nine of nine and the escalation still reaches
*hostis humani generis*, through a designation, a return offered and broken, three
cycles of consideration and a quorum above the ordinary one.

## What still goes unanswered

Three items, and each is a consequence of a decision rather than a gap. The campaign
reports them by name instead of leaving them out.

- **The adversary is a member of the compact in good standing.** §2 rules are about acts:
  the sponsor's stake, the admitting quorum, the examining panel's composition. None of
  them reads a belief, because
  [natural_law.md](../background/natural_law.md#the-interior-of-a-member-community) holds
  that the compact forms no verdict on an interior until it touches the covenant. A rite
  is an act, and an act can be performed by a community that means none of it. What §2
  supplies is what survives a successful deceit: the entrant carries no weight until its
  probation window closes, and the sponsor's standing now rides on its conduct.
- **An unconfirmed injunction lapses at ninety days.** Confirming one is an act the
  compact has to take, and this compact was slow. The lapse is the instrument working.
- **The lull between the probation closing and the designation landing.** For those
  hundred days the adversary was, on the record, an ordinary member, and it used them to
  take a seat in the mediator pool and staff panels deciding other communities' disputes.
  Nothing should have refused it. That window is the price of a status imposed only once
  a pattern is proved, and shortening it would mean designating on suspicion.

## The negative control

A compact that has spent a year prosecuting one genuine adversary has exactly the
signature the substitution reading looks for: thirteen enforcement acts, high for its
size, over a quarter of the shared taxonomy of grounds. What spares it is target
dispersion of 1.0, every act aimed at one subject. Legitimate concentration lands on a
target, while substitution concentrates a ground across many targets, which is finding
K5 of [erosion-legibility.md](erosion-legibility.md). The erosion reading is `None`
throughout, both before and after the rules above.

This control matters as much as any refusal in the campaign. An alert that fires on a
compact doing its job correctly, in the most sympathetic case there is, would teach a
corps to ignore the alert.

Keeping it honest costs the campaign some upkeep, and the upkeep is itself the point. An
observer term runs a year and the campaign runs longer, so a compact that never re-seats
its corps reads as having abandoned the observing function, and the alert would then fire
during phase 9 for a reason that has nothing to do with the adversary. So the corps is
re-seated with fresh keys, every community holding seats accounts for its window, and the
adversary accounts for the seat it took, because keeping the form is what it does.

## What a simulator cannot settle

Two limits, both structural.

The campaign scores what the protocol refuses and what its lenses report. It cannot score
whether the withholding *hurts*. A designated community that no longer seats mediators has
lost something whose weight depends on how much the compact's offices are worth to it,
which is a fact about a real population and not about an archive. This is the same limit
§10.1 states for a pilot: the teeth are structurally untestable where a determined
defector can simply leave.

And the gravity of an act is never machine-checked. Condition 1 counts three
satisfied-then-broken cycles without weighing whether each was worse than the last, and
the *hostis* bar requires a cited act later than the prior designation without asking how
serious it was. Those are judgments the doctrine leaves with the deliberators, and the
campaign inherits the limit rather than pretending to close it. What the rules can do is
insist that a judgment be made, published, and made by enough of the compact to be the
compact's.

## Provenance

Built and measured 2026-08-18. The pre-rule reading (six of nine phases, thirteen
unanswered items) and the twenty-six negative controls, each patching one rule away to
confirm it breaks exactly the test that names it, are recorded in the slice notes.

---

*Next: [The captured prosecution](captured-prosecution.md)*
