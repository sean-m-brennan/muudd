*Previous: [Founding a polity](charter.md)*

# Which human answers for which machine

The compact tier is unambiguous that a machine participates only in a derived
form, irrevocably tied to one specific named human. That human is the guardian,
and every machine act is ultimately an act the guardian answers for.

For a long time that relation was asserted in prose and absent from the types of
every codebase involved. The compact said machines are delegates and had no
record of whose delegate. The trust substrate knew whether a node had an
operator and had no notion of a community for that operator to answer to. The
claim was load-bearing and nothing carried it.

This tier is the natural home for the relation, and here it becomes structural.
The charter itself records which members are derived parties, meaning machines,
and which human each one answers to. Those edges are ratified at founding, with
every named guardian co-signing, and they move afterward only by signed records.

The reason the guardian co-signs, at founding and at every subsequent change, is
one sentence that recurs through this tier: answerability is accepted and never
assigned. A community cannot vote a person into responsibility for a machine.
The person has to agree.

Two consequences follow immediately, and both matter more than they look. A
polity that is mostly machines refuses an office to a machine with nobody behind
it, since a body governed by parties nobody answers for is governed by nobody.
And a chartered edge that the substrate stops showing is graded as coherence
erosion, because responsibility quietly evaporating is exactly the failure this
record exists to make visible.

## The derivation, and how an edge moves

A `Derivation` on the charter says which members are derived parties and which
human each answers to. It is ratified at founding with every named guardian
co-signing.

Afterward, edges move by `GuardianChange` records in three forms.

Firstly, *designate* takes a majority plus the consent of the guardian, which is
the ordinary case of a community assigning a machine to somebody willing to hold
it.

Second, *rotate* requires the incoming guardian to consent. Note who does not
have to consent: the outgoing one. A guardian handing off cannot block the
handoff by refusing to sign, and the person taking on the answerability must
agree, which is the same rule stated from the other side.

Lastly, *release* happens either by the departing guardian alone or by a
majority recording that the human is gone. Both routes exist because both
situations happen, and neither can be made to wait on the other.

An edge with no guardian is an honest chartered state rather than a missing
value. A release clears the edge and keeps the designation, since a machine does
not become human by losing its human. The distinction is exactly the one the
cohesion monitor needs, because an unguarded machine and an unrecorded machine
are different facts about a polity.

Two closure rules hold everywhere. No machine may guard a machine, and nothing
may guard itself. Both are the same rule the substrate anchor obeys one level
out and the succession rules obey one level further: a ground inside what it
grounds is no ground.

A derived read model answers the effective picture and answers whether the
polity is machine-majority, which is what conditions the office rule above. That
question reads the whole roll rather than the inducted part of it, because it
asks what the polity is made of rather than who decides in it.

## A claimed edge carries what showed it

An edge asserted in a charter is a claim, and a claim with nothing behind it is
worth what it costs to make.

So a guardian change that asserts an edge presents the signed observation in
which the substrate actually reported it, or declares itself unbacked with a
stated reason.

The evidence is *carried* rather than cited by content identifier, and the
reason is specific. Observations are the one prunable entry class in this tier,
so a citation could dangle, and authority would become a property of when you
happened to look. Carrying the evidence makes the record self-contained forever.

Founding derivations are gated the same way, and there the asymmetry inverts. An
unbacked declaration is permitted and honest, and it earns nothing, because at
founding a self-report would *buy* the exemption the substrate rules grant to a
guarded machine.

The observer may be neither the polity nor anyone on its roll. A founder signing
a real observation about its own cluster is the same closure shape refused for
an anchor and for a successor, reaching the evidence one level out.

A named guardian observing the edge it is about to accept does count, and the
reason is worth following. That guardian is not on the roll, it co-signs either
way, and what it asserts by observing is its own answerability. Interest
disqualifies praise rather than an admission.

## What the substrate hands over

The provider interface supplies a candidate boundary, and the guardian edges
travel with it rather than being asserted afterward. A candidate the substrate
reads as a derived party becomes a proposed derivation, so which human answers
for which machine is ratified rather than claimed.

Two fields on a candidate carry distinctions that were previously collapsed.

A *reputation* reading is optional, which distinguishes unrated, rated low, and
rated well. An unrated candidate does not satisfy a demanded reputation floor,
since a positive floor is a positive assertion and an absent reading cannot
support one, and a floor of zero demands nothing. The exclusions record which
distinction actually applied, so an excluded party can read the difference
between falling below a floor and having no reading at all.

A *derived* flag carries the reading of the substrate about kind, independently
of the guardian edge. Before this, a machine with nobody behind it proposed
silently as a human, which is precisely the case the whole chapter exists to
make visible. Now it is ratified as the unguarded derivation it is.

The provider also supplies an optional observation at build time, so a live
provider presents what it actually saw and falls back to an unbacked declaration
with a reason when no monitor has run. The provider never mints backing that the
rule would reject, so an honest bootstrap proposal is one the founders must
ground rather than one that fails late.

## The trust-substrate adapter

Although the adapter is small, it carries the whole coupling. It maps three
things.

The gateway reputation subtree becomes the candidate roll. The verified operator
of a node becomes the guardian edge for that member. And the attended-now stamp
becomes the moment that guardian was last confirmed present.

That third mapping is where the two signals from Part II land, and keeping them
apart is as important here as it was there. Whether a node has a human at all is
the guardian edge, which is durable. Whether a human was recently at the console
is the attendance stamp, which is live and meaningless the moment it ages. The
cohesion monitor reads them differently, and the next chapter but one explains
why a stale stamp may warn and may not escalate on its own.

The adapter implements the change stream by diffing each folded event against
the view it already holds, since the provider is the one place the prior reading
is in hand. It emits an attendance delta only when the stamp actually advances,
so a repeated identical stamp is not mistaken for renewed presence.

The default build links no trust substrate at all, so the independence of the
compact tier from that substrate holds with the adapter present.

## Crossing into Python

The Python surface exposes the adapter directly, and one detail of how is
load-bearing.

The binding takes wire fields rather than an assembled peer object. A guardian
public key becomes a guardian edge, and an operator timestamp becomes an
attendance stamp, only when the operator-bound flag is set.

That reduction is precisely why the fold crosses the seam rather than being left
to the caller. Hand-rolled, it read an unguarded machine as guarded and
attended, which is the same conflation described above arriving through a
different door. The lesson generalizes past this binding: a fold that silently
succeeds toward the healthy answer is worse than one that fails, because nothing
anywhere reports the mistake.

The live couplings are absent rather than present-and-raising in a build without
the trust substrate, and the binding exposes a flag so a caller can ask which
build it has.

## The guardian assembly

For a polity that is mostly machines, the polity substantially *is* the guardian
assembly, meaning the humans whose standing is at stake in the conduct of the
machines.

The substrate of such a polity is the ground those guardians stand on. Its
offices are held by guardians, since a machine with nobody behind it may not
hold one. Its membership roll records the node-to-guardian edges. And the whole
apparatus of the compact tier for machine participation finally has an entity to
live in, rather than being a claim about a relation nothing recorded.

That is the answer to a question the compact tier asks and cannot answer for
itself. When two machines meet across a compact boundary, the exchange reduces
without remainder to an exchange between two named humans. This tier is where
those two names come from.

## Further reading

- [Design decisions D1 to D34](design.md): D15 for the chartered edge, D18 for
  what the provider knows against what it saw, and D27 for the backing an asserted
  edge must carry.
- [The human behind the machine](../../autonomous_trust/doc/architecture/operator-attended.md):
  the two signals underneath, and why one is durable and the other live.
- [Machines as delegates](../../kith-covenant/doc/machines.md): what the compact
  tier does with a guardian, including the authority envelope and the ratchet
  rule.
- [Watching for erosion](cohesion.md): the two measures that read guardian edges,
  and the cap that separates a night shift from an abandoned polity.

---

*Next: [The rite](transmission.md)*
