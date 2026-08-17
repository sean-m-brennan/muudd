*Previous: [When communities disagree](disputes.md)*

# The five commons

Some things cannot be divided, and five of them decide whether anybody survives.

*Life support.* Air, water, and the systems that recycle them. *Communications.*
The relays and spectrum everyone shares. *Navigation.* Beacons, ephemerides, and
the data that keeps vehicles from hitting each other. *Knowledge.* What has been
learned about keeping people alive out there. *Rescue.* The standing obligation
to answer distress.

These are governed as thin-grammar commons in the Geneva style, meaning
cooperation is required across the boundary of the compact rather than only
inside it. The reason is not generosity. It is that the cost of defection is
physical, immediate, and legible to everyone. Nobody wants to poison the well
they drink from, and in a habitat the well is the air.

The governance is patterned on Ostrom's design principles for common-pool
resources, adapted in two directions the terrestrial cases never had to handle.

*For light-lag*, monitoring is distributed and reports are eventually consistent
rather than real-time. A commons whose governance required a synchronous view
would be ungovernable at interplanetary distance.

*For the fait accompli*, decisions are split into classes by their reversibility.
Operational choices are valid locally within light-lag. Compact-level decisions
require quorum signatures from communities at multiple distances, which no
faction confined to one region can satisfy alone.

## Accounting, and what a written cap is worth

A commons may state a draw allowance. That is the machine-readable half of a
provision rule, and it is the only part of proportionality that a rule can act
on.

Before it existed, the written cap bound nothing. A record drawing ten times the
stated cap was authorized exactly like one that honored it, and the two were
indistinguishable to the archive.

*Usage may be sealed*, and the reason is a real tension rather than a
convenience. The specification asks for a surveillance threshold with
configurable disclosure levels, and a community that must publish every figure in
order to account honestly is disclosing its capacity, its consumption pattern,
and its reserves to every other member.

A sealed record commits its figures, publishes a zero-knowledge proof that the
draw is within the allowance, and puts the figures themselves under threshold
custody keyed to fellow appropriators, a quorum of whom can open them for cause.
The reporter may not be one of its own custodians.

*Sealing is not a way to disappear*, and measuring is what showed it would have
been. The free-rider lens summed the published figures, so a community sealing
1,850 units read as having consumed zero and ranked nowhere: not absent, but
apparently the most scrupulous member of the compact. The lens now counts sealed
records, reports whether the accounting is complete, and turns each proof into a
bound on what is hidden, so its totals are honestly a floor rather than
dishonestly a figure.

## Graduated sanctions

The sanction ladder runs from monitoring through to exclusion, with the heavier
levels quorum-gated so that no single party can impose them.

Graduated is the operative word, and it is Ostrom's finding rather than a
preference. Sanctions that jump straight to the maximum are sanctions that get
withheld, because nobody will impose them for a first offense, so the rule
becomes dead. Sanctions that start small get used, and getting used is what makes
them credible.

*The commons equivalent of anathema* exists as a separate finding, which the
research corpus had listed under open questions. Four conditions hold it, in the
shape the four-condition designation test carries. The subject is at exclusion.
It has arrived there more than once, which is defiance rather than compliance. It
has stood there long enough that restoration was available and not taken. And one
arrival suffices on a life-critical commons, which the compact keeps running when
it pauses everything else.

The finding gates nothing by itself. A designation may cite it, so the ladder is
untouched and the threshold lives in the refusal to record the finding rather
than in a new power.

## Nesting, and the limit that makes it real

Ostrom's eighth principle is nested enterprises, and here it is made concrete
rather than gestured at.

A compact gets an identifier of its own, co-signed by a quorum of its members and
by its own key. The asymmetry with a community is deliberate and exact: a
community identity is self-signed because a community exists by its own act, and
a compact is constituted by its members instead.

A commons registration carries its appropriator level, and at the compact level
the appropriators are whole compacts.

The rule that makes nesting real rather than decorative is *subsidiarity*: the
higher level may not reach past a compact to its member communities. A federation
that could discipline a community directly would be governing at the scale of the
compact while answering to nobody at that scale.

## The tragedy of the virtues

One problem here has no clean answer and is named rather than solved.

The classic tragedy of the commons is a problem of self-interest, and the
machinery above addresses it. The tragedy of the *virtues* is the opposite
problem. A community that draws less than its allowance, contributes more than
its share, and answers every distress call is subsidizing communities that do
not, and it has no procedural recourse that does not make it the kind of body
that keeps accounts against its neighbors.

The compact makes the asymmetry legible and declines to correct it
automatically, on the same reasoning that keeps it from judging the interior of a
member. A mechanism that penalized generosity-by-comparison would be a mechanism
for punishing the members it most needs.

## Further reading

- [Commons federation](background/commons_federation.md): Ostrom's eight
  principles mapped across, the five commons developed in full, monitoring
  without a panopticon, and the tragedy of the virtues.
- [Bounded conflict](background/bounded_conflict.md): the never-target categories
  the commons inherit from.
- [Protocol components](architecture/protocol-components.md): commons governance
  as built, including the sanction ladder and the inter-compact level.

---

*Next: [Protected dissent](jester.md)*
