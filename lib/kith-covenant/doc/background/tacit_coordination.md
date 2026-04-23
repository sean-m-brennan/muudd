*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > Tacit Coordination*

# Tacit Coordination: Cooperation Without Communication

## Thesis

Most of kith-covenant's design assumes that explicit communication between
participants is available — that disputes can be registered, mediators
contacted, virtues articulated, ladders climbed and descended through
exchange of messages. There are conditions under which this assumption
fails: light-lag at AU distances; deliberate jamming or signal denial;
cultural rupture across generations; protocol-substrate compromise;
adversarial information warfare. In such conditions kith-covenant needs a
*fallback layer* in which participants can still coordinate without
communicating, on the basis of mutual interest, shared salience, and
the iterated structure of repeated interaction.

This fallback layer is the subject of the formal literature on **tacit
coordination** — Schelling's focal points, Axelrod's tournaments on
iterated cooperation, the folk theorem of repeated games, costly
signaling theory, and the common-knowledge framework. The literature
gives kith-covenant three things: (a) an account of how cooperation can
emerge among self-interested actors without explicit agreement, which
is encouraging; (b) a precise account of the conditions under which it
emerges, which constrains kith-covenant's design space; and (c) a clear
account of how it fails, which prevents kith-covenant from over-relying on it.

The cardinal lesson is that tacit coordination is *real but narrow*.
It can carry load when explicit protocol cannot, but it cannot
substitute for the virtue substrate
([justus_hostis.md](justus_hostis.md#the-anthropology-the-substrate-must-assume),
[bounded_conflict.md](bounded_conflict.md#common-virtue-not-non-negotiable-rules))
that kith-covenant's main design rests on. A kith-covenant that treats the tacit
layer as primary will fail in exactly the situations the explicit
substrate was designed to address.

## Schelling's focal points

Thomas Schelling's *The Strategy of Conflict* (1960) introduced the
concept of the **focal point** (sometimes "Schelling point"): in a
coordination problem with multiple equilibria, players converge on the
equilibrium that is *salient* — that stands out for some shared reason
not encoded in the formal structure of the game.

Schelling's classic examples:

- *"Where will you meet a stranger in New York City if you have no way
  to communicate?"* The most common answer in Schelling's informal
  experiments was *Grand Central Terminal at noon*. Nothing in the
  formal structure of the city makes this place-and-time the
  equilibrium; its salience comes from outside the game — from shared
  cultural reference and from being the obvious choice given a shared
  imagination of "the obvious place."
- *Splitting $100, where neither side gets the money unless both
  propose the same split.* Most pairs converge on $50/$50, even though
  every other split is also a Nash equilibrium. The 50/50 split is
  *focal* because of fairness intuition, the salience of equal
  division, and the shared knowledge that the other player has the
  same intuition.
- *Naval mine-sweeping coordination.* When two units are independently
  assigned overlapping search areas, they converge on dividing the
  area at the obvious geographic feature — a river, a coastline — even
  when nothing in their orders specifies this.

Three structural features make a focal point work:

1. **Salience to all parties.** Each party must recognize the focal
   feature as obvious; this requires shared cultural, perceptual, or
   physical reference.
2. **Mutual knowledge of mutual knowledge.** Each party must believe
   the other party recognizes the same feature as obvious — and must
   believe the other party believes the same about them, recursively.
3. **No competing salience of comparable strength.** If two features
   are equally salient, the coordination problem is not solved.

Focal points generalize to all cooperation problems where the
coordination question is "which equilibrium do we converge on?" rather
than "do we cooperate at all?". They explain a great deal of
spontaneous human coordination — from queuing conventions to right-vs-
left side traffic to the convergence of standards.

### Application and limits for kith-covenant

Focal points can supply tacit coordination when kith-covenant's explicit
protocol is unavailable. Examples in the kith-covenant context:

- *Distress signal frequencies and protocols.* If protocol traffic is
  cut, vessels in distress and potential rescuers can converge on
  legacy distress procedures (analogues of Mayday/SOS, COSPAS-SARSAT
  beacons) as the focal fallback. Kith-Covenant should specify and protect
  these fallbacks even though they sit "outside" the main protocol.
- *Customary boundaries.* In the absence of negotiated borders, parties
  can converge on physical features (orbital lanes, established
  transit corridors) as focal coordination points. The legal-historical
  parallel is freedom of the seas crystallizing around obvious
  features rather than detailed treaty.
- *Rendezvous and meeting points.* For diplomatic contact when normal
  channels fail, designating durable physical or orbital "always-here"
  meeting points functions as a focal point even when no message can
  be sent.

The limit is sharp: focal points only function where salience is
shared. *Salience is culture-relative* (see "Common knowledge and
culture-relative salience" below). A kith-covenant spanning communities that
have drifted apart culturally — across generations, across deep
ideological divides, across the asteroid belt — cannot rely on focal
points without first having established what the focal points *are*.
This is itself an explicit-protocol task, which means that even the
tacit layer requires some prior explicit work to define its content.

## Iterated cooperation: Axelrod and after

Robert Axelrod's *The Evolution of Cooperation* (1984) is the second
classical reference. Axelrod ran computer tournaments in which
strategies for the iterated Prisoner's Dilemma competed against each
other. The result that became famous: **Anatol Rapoport's
*tit-for-tat* strategy won both tournaments, despite being one of the
simplest strategies submitted.**

Tit-for-tat:

1. On the first move, *cooperate*.
2. On every subsequent move, *do whatever the other player did on the
   previous move*.

Axelrod identified four properties of strategies that did well in the
tournaments:

- **Nice** — never the first to defect. Strategies that defected first
  triggered retaliation cascades they then could not stop.
- **Retaliatory** — punishes defection promptly. Strategies that were
  too forgiving were exploited.
- **Forgiving** — returns to cooperation as soon as the other player
  does. Strategies that retaliated indefinitely (e.g., "grim trigger")
  fell into mutual-defection death spirals.
- **Clear** — the strategy is easy for the other player to read and
  predict. Complicated strategies, even if individually competent,
  scored worse because their counterparties could not learn what to
  expect.

Subsequent work has refined the picture. Axelrod's later work and
others showed:

- *Tit-for-two-tats* (cooperate unless defected against twice in a row)
  is more robust to noise — but can be exploited by alternating
  defectors.
- *Pavlov* (Win-Stay, Lose-Shift) outperforms tit-for-tat in some
  noisy environments because it can recover from mistakes faster.
- *Generous tit-for-tat* (occasionally cooperates after a defection)
  also handles noise better, at the cost of being slightly more
  exploitable.
- In *evolutionary tournaments* with population dynamics, the picture
  is more complex: cooperative strategies can dominate but only after
  enough cooperative strategies are present to find each other; small
  initial cooperative populations can be wiped out before they can
  establish themselves.

The contemporary research has substantially complicated Axelrod's
clean picture without overturning its central claim: **simple,
provocable, forgiving, and clear strategies do well in iterated
interaction across a wide range of conditions**, and complex strategies
generally do not pay off enough to be worth their complexity.

### Application and limits for kith-covenant

The four properties give kith-covenant a design heuristic for behavior at the
tacit layer when explicit protocol breaks down:

- *Default to cooperation*, but
- *retaliate quickly* when defected against, and
- *forgive quickly* when the other party returns to cooperation, and
- *behave legibly* so that other parties can read intent.

These map directly onto kith-covenant's protocol architecture (the dispute
ladder is provocable; the restitution paths are forgiving; the
public-record requirement is clarity-by-design) and provide a
fallback heuristic for participants when the explicit ladder is
unavailable.

The limits are again sharp:

- The *iterated Prisoner's Dilemma* assumes a *binary cooperate/defect
  choice* and *paired interaction*. Real inter-community conflict has
  many degrees of freedom, multiple parties, and unclear attribution.
  Tit-for-tat in pure form does not generalize cleanly.
- The *noise problem* — one mis-perceived defection triggers retaliation
  triggers retaliation — is severe in real-world settings where
  attribution is uncertain and signals can be jammed or spoofed. This
  is exactly the situation kith-covenant is designed for, and the Axelrod
  result must be applied with corresponding care.
- The strategies do well *on average across populations*; they do not
  guarantee good outcomes in individual interactions, and a single
  catastrophic interaction (a violation that cannot be retaliated
  against, or a retaliation that itself triggers existential response)
  can destroy a relationship that the long-run statistics treat as
  successful.

## The shadow of the future and the folk theorem

A general result in repeated-game theory — the **folk theorem** —
states roughly that any individually rational outcome in a one-shot
game can be sustained as an equilibrium in the infinitely-repeated
version, provided the players' discount rates are low enough (i.e.,
they care enough about future payoffs). The popular shorthand:
**cooperation is sustainable when the shadow of the future is long
enough**.

For kith-covenant, this is the formal underpinning of an observation made
throughout the rest of the documentation: *exit costs and continued
interaction are the deepest material support for cooperation*. Where
exit is cheap and future interaction is rare, defection pays.

Three concrete implications:

- **Space habitats have an exceptionally long shadow.** They cannot
  cheaply move; they will be neighbors for centuries; the discount
  rate on future cooperation is unusually low. This strongly favors
  cooperative equilibria, even among purely self-interested actors.
- **Earth communities have a much shorter shadow.** Exit options exist;
  state and corporate actors come and go; cultural turnover happens on
  decade scales. The folk-theorem argument for cooperation is
  correspondingly weaker on Earth, which is one more reason
  ([bounded_conflict.md](bounded_conflict.md#earth-first-the-harder-test))
  Earth is the harder test.
- **The folk theorem is a permission, not a prediction.** It says
  cooperation *can* be sustained, not that it *will* be. The space of
  equilibria is large; coordination on a particular cooperative
  equilibrium still requires the focal-point machinery above.

## Common knowledge and culture-relative salience

A focal point requires not only that both parties recognize the
salient feature, but that each party knows the other recognizes it,
and knows that the other knows, recursively. This is the
**common-knowledge** condition, formalized by Robert Aumann (1976) and
explored in David Lewis's *Convention* (1969).

Three observations matter for kith-covenant:

1. **Salience is culture-relative.** Schelling's New Yorkers
   converged on Grand Central; people from other cultures converge on
   different points (in some informal experiments, the answer varies
   substantially by city of origin). What is "obviously the right
   meeting place" is shared within a cultural community and not across
   communities. **Tacit coordination across communities that lack
   shared culture is much harder than tacit coordination within a
   community.**
2. **Common knowledge can be created, but doing so is itself an
   explicit act.** Public ceremonies, broadcasts, ratifications,
   traditions — these are mechanisms for making something common
   knowledge in the technical sense. *The very act of kith-covenant ratifying
   its compact creates common knowledge of the compact's terms*; this
   is one of the most important things ratification does, beyond the
   merely legal.
3. **Common knowledge is fragile.** A single party defecting from the
   shared assumption is enough to break the common-knowledge condition,
   because each other party must now wonder whether others have also
   defected. This is the *unraveling* dynamic familiar from electronic
   commerce and bank runs: confidence collapses faster than it builds.

The implication is that kith-covenant's protocol layer is not just a
practical convenience over the tacit layer — *the protocol layer is
how common knowledge is established and maintained*. Even where tacit
coordination would suffice in principle, the explicit protocol does
the work of making the conditions for tacit coordination *common
knowledge*. Without that, the tacit layer cannot function reliably
across community boundaries.

## Costly signaling

A final body of theory worth marking. **Costly signaling** —
introduced in evolutionary biology by Amotz Zahavi (1975) and applied
to economics and politics by Michael Spence (1973) and others —
addresses how parties communicate trustworthy information when cheap
talk is unreliable.

The core idea: a signal is credible to the extent that *it would not
be in the signaler's interest to send it if the underlying claim were
false*. Examples:

- A peacock's tail is credible evidence of fitness because an unfit
  peacock could not survive the cost of growing one.
- An expensive education is credible evidence of capability because a
  less capable applicant would find the education unaffordable in
  effort.
- A nation that publicly demolishes a weapon system is making a costly
  signal of disarmament intent — and the public, costly nature of the
  act is what makes it credible where mere statements would not be.

Costly signaling is the formal-theoretical underpinning of kith-covenant's
*public, expensive, reversible* protocol acts. A community that
publicly accepts arbitration, that visibly invests in maintaining the
knowledge commons, that hosts memorial occasions whose interruption
would be a major cultural loss — these are costly signals of compact
membership in good standing. They are credible *because* they are
costly. They cannot be cheaply faked by a community that is
optimizing against the letter of the protocol while violating its
spirit.

The connection to the gamification problem
([bounded_conflict.md](bounded_conflict.md#gamification-and-the-letter-spirit-divergence))
is direct: a community that games the letter has to fake its costly
signals as well as fake its formal compliance, and the costly signals
are by construction harder to fake. The protocol's deepest
anti-gamification mechanism is therefore not better rules but
*better-designed costly signals* — protocol acts that cannot be
performed convincingly by an adversary who does not actually share
the underlying virtue.

## Decay modes for tacit coordination

Tacit coordination has its own characteristic failure modes, distinct
from those of explicit-protocol cooperation. Kith-Covenant must understand
these in order not to lean on tacit coordination where it cannot
support the load.

- **Salience drift.** Focal points migrate over time as cultures
  change. The "obvious meeting place" of a generation ago may not be
  obvious now. Tacit coordination across generations is therefore
  systematically harder than tacit coordination within a generation,
  and kith-covenant's generational-cadence re-affirmation
  ([bounded_conflict.md](bounded_conflict.md#protocol-sketch))
  partially addresses this by re-establishing common knowledge.
- **Noise-induced retaliation cascades.** Tit-for-tat and its
  relatives are catastrophically vulnerable to mis-attributed
  defection in noisy environments. A single jammed or spoofed signal
  can trigger a death spiral. Kith-Covenant's design must include
  *attribution-uncertainty handling* explicitly — a mediator role
  that can pause retaliation pending confirmation, or a default rule
  of forgiveness on first apparent defection.
- **Coordination on bad equilibria.** Tacit coordination locks in the
  *focal* equilibrium, which is not necessarily the *best* equilibrium.
  Once a community has converged on a bad practice (a wasteful
  custom, an unjust convention) tacit coordination tends to *preserve*
  it because deviation is individually costly. Explicit deliberation
  is the mechanism for moving from a bad equilibrium to a better one;
  reliance on tacit coordination tends to ossify whatever equilibrium
  is currently focal.
- **Common-knowledge collapse.** Once a few parties visibly defect,
  common knowledge of cooperation evaporates rapidly because each
  remaining party must now doubt whether others are still
  cooperating. Kith-Covenant's response should be procedural: rapid,
  visible, public re-affirmation by the still-cooperating parties
  to re-establish common knowledge of who is in good standing. This
  is the same mechanism the Hanseatic League used informally and that
  kith-covenant can use deliberately.
- **Adversarial salience-poisoning.** A sophisticated adversary can
  *manipulate salience* to shift focal points in their favor —
  promoting one meeting place over another, one frequency over
  another, one custom over another, in ways that disadvantage
  counterparties. Open-source intelligence, propaganda, and modern
  information warfare all exploit this. Kith-Covenant cannot fully prevent
  this but can resist it by maintaining *plurality of salient
  features* (multiple focal points for the same coordination
  problem) so that capture of any one does not break coordination.

## Application to kith-covenant

The relationship between the tacit layer and the rest of kith-covenant's
design can be summarized:

| Kith-Covenant layer | Tacit-coordination role |
|---|---|
| Common virtue | Makes costly signals legible; without shared virtue, costly signals are merely costly |
| Recognition protocols | Explicit acts that establish common knowledge of membership |
| Escalation ladder | Provides the *retaliatory* and *forgiving* properties that tit-for-tat-style behavior would otherwise have to extemporize |
| Mediator roles | Handle attribution uncertainty, breaking the noise-induced cascade dynamic |
| Periodic re-affirmation | Refreshes common knowledge before salience drift breaks the tacit layer |
| Restitution and return | Operationalizes "forgiveness" so the iterated game's equivalent property survives violations |
| Inviolability of rescue | A focal point so universally salient that it can survive even severe protocol breakdown |

Where the tacit layer carries weight that the explicit protocol cannot:

- **In the gap when communications are lost.** Light-lag, jamming,
  rupture — in all such cases, communities must coordinate on what
  was last established as common knowledge before the gap, plus
  whatever remains universally salient (rescue, transit lanes,
  distress procedures).
- **At the founding of a new compact.** Before explicit protocol can
  be ratified, founding communities are coordinating on focal
  points and costly signals that establish good faith.
- **At the boundary with non-members.** Kith-Covenant's compact does not
  reach non-members, but tacit coordination via focal points — the
  freedom-of-the-seas analogue — can extend cooperation into spaces
  where no compact applies.

Where it cannot:

- **Substituting for explicit virtue.** The tacit layer assumes parties
  will recognize the same focal points and respond similarly to
  iterated interaction; both assumptions fail across deep
  cultural difference. Kith-Covenant's main work is at the explicit-virtue
  layer for exactly this reason.
- **Handling first-time situations.** Tacit coordination relies on
  shared imagination of the obvious choice; novel situations have no
  established focal points. Explicit protocol must do the work of
  establishing the new convention.
- **Supporting restitution and return.** The iterated-cooperation
  literature is largely silent on what happens *after* a major
  defection that cannot be simply retaliated against and forgotten.
  The restitution-and-return architecture
  ([bounded_conflict.md](bounded_conflict.md#protocol-sketch),
  [historical_cases.md](historical_cases.md#the-iroquois-confederacy-haudenosaunee-kaianerekowa))
  is irreducibly explicit work.

## Open questions

- What is the minimum set of focal points kith-covenant should *explicitly
  establish at compact founding* to ensure tacit coordination remains
  available during communication failure? Distress procedures and
  rescue protocols are obvious; what else?
- How does kith-covenant measure the *health* of its common-knowledge
  substrate? Are there observable signals that common knowledge is
  decaying before it visibly collapses?
- The Axelrod tournaments assume paired interaction; real kith-covenant
  scenarios are multi-party. What does the iterated-cooperation
  literature say about *n-party* iterated interactions, and what
  does it say about kith-covenant's design?
- Costly signaling assumes that costliness is *legible to all
  parties*. Across deeply different cultures, what is costly to one
  party may be cheap to another. How does kith-covenant design costly
  signals that are credibly costly across the cultural diversity of
  its membership?
- The Ostrom design principles for commons (see
  [commons_federation.md](commons_federation.md))
  overlap substantially with the conclusions here. What does the
  empirical commons literature add that the formal coordination
  literature does not?

## Relationship to other documents

- [justus_hostis.md](justus_hostis.md) — the virtue substrate that
  this document treats as primary; tacit coordination is the fallback
  when virtue-substrate communication fails. The transcendent-moral-authority
  finding ([historical_cases.md](historical_cases.md#the-transcendent-moral-authority-pattern),
  [noticeable_hand.md](noticeable_hand.md)) deepens the substrate
  question: costly signals work because there is something at stake
  beyond reputation, and the historical pattern suggests the something
  is transcendent. Tacit coordination operates well over a substrate
  that supplies that beyond-reputation stake; less well when it does
  not.
- [bounded_conflict.md](bounded_conflict.md) — the explicit protocol
  whose tacit-layer fallback is treated here. Several mechanisms (the
  ladder's retaliatory/forgiving properties, the mediator's
  attribution role, periodic re-affirmation) are direct applications
  of tacit-coordination theory.
- [historical_cases.md](historical_cases.md) — the historical cases
  are real-world instances of the dynamics formalized here; the
  Hanseatic boycott is a costly signal, the codes duello procedure
  is a focal-point convention, the Iroquois Council Fire is a
  common-knowledge institution.
- [commons_federation.md](commons_federation.md) —
  Ostrom's empirical work on commons governance overlaps substantially
  with the conclusions here.
- [reputation_bridges.md](reputation_bridges.md) —
  reputation as a costly signal that travels across community
  boundaries.
- [../social_framework.md](../social_framework.md) — parent document.

## References

*To be added — anchor works include: Schelling,* The Strategy of
Conflict *(1960) and* Arms and Influence *(1966); Axelrod,* The
Evolution of Cooperation *(1984) and* The Complexity of Cooperation
*(1997); Aumann, "Agreeing to Disagree,"* Annals of Statistics *4
(1976); Lewis,* Convention: A Philosophical Study *(1969); Zahavi,
"Mate Selection — A Selection for a Handicap,"* Journal of Theoretical
Biology *53 (1975); Spence, "Job Market Signaling,"* Quarterly Journal
of Economics *87 (1973); Nowak & Sigmund on Pavlov / Win-Stay-Lose-Shift
("A strategy of win-stay, lose-shift that outperforms tit-for-tat in
the Prisoner's Dilemma game,"* Nature *364, 1993); Ostrom,* Governing
the Commons *(1990); Binmore,* Game Theory and the Social Contract
*(1994, 1998).*
