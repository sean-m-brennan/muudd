*Previous: [Trusting the metal](verifiable_infrastructure.md)*

# The optional stack

Everything so far has been about moving bits, covering radios, power budgets,
small programs on nodes with no administrator, and routing that works without a
map. A community that builds all of it has a working network, and a working
network is not yet a working community.

The gap shows up the first time something goes wrong socially rather than
technically. A member floods the relay and will not stop. A neighboring
settlement wants to interconnect and nobody can say who has the authority to
agree. A founder dies and it turns out nothing was written down about who
decides what. A machine acting for somebody takes an action nobody authorized
and there is no record of whose machine it was. None of these is a routing
problem, and no amount of better routing addresses any of them.

Three optional tiers address them, and each answers exactly one question the
tier below it cannot.

**Who may cooperate?** AutonomousTrust answers it by having every node judge its
peers on what they actually did, continuously, with no central authority
consulted.

**What is a community, and what keeps it one?** Ethne answers it by turning a
group that already cooperates into a chartered polity with a roll, offices, and
a way of bringing in the young.

**How do communities treat one another?** Kith-Covenant answers it with a civil
protocol binding parties who share no creed and answer to no common sovereign.

The rest of this book is those three, in that order. This chapter says why they
are three rather than one, what each is actually for, and what a community gets
by installing none, some, or all of them.

## The three questions, in order

The questions have to be answered in this sequence, because each one presupposes
the answer to the last.

Start with a mesh and nothing else. Traffic flows, and the network has no
opinion about who deserves to send it. Among honest neighbors this is fine, and
it is the correct configuration for a small deployment where everyone knows
everyone and nobody is hostile. It stays fine exactly as long as that
description holds.

Add a node that misbehaves, whether compromised, malfunctioning or simply
greedy, and the question of who may cooperate becomes urgent. The conventional
answer is a credential, where some authority issues certificates and holders are
admitted. That answer fails here twice over. It requires an authority, which
this whole project is an argument against, and it cannot see the case it most
needs to see, which is a peer holding a perfectly valid credential that has been
taken over. AutonomousTrust answers instead from behavior, so every peer starts
at zero, earns reach through observed conduct, and loses it the moment conduct
changes. This is the first tier, and it operates entirely between machines with
no person involved.

Now suppose that works, and a set of nodes reliably cooperate. There is still no
community. What exists is a cluster in a graph, with no boundary anybody agreed
to, no memory of decisions, no offices, and no way to persist as members join
and leave and die. A cluster cannot sign an agreement, cannot be a party to
anything, and cannot be held to a commitment made last year by people who have
since departed. Ethne is the tier that turns the cluster into a body, giving it
a charter, a membership roll, internal offices, a stated ground the community
holds itself to, and a rite by which new members actually receive it.

Lastly, suppose a community exists in that stronger sense, and there is another
one over the ridge. They share no history, no founding, and quite possibly no
faith. They cannot merge, cannot submit to each other, and cannot appeal upward,
since there is nobody above both of them. They will nonetheless need each other,
for rescue, for trade, for shared infrastructure, and for the plain fact that a
hostile party near one of them is a problem for both. Kith-Covenant is the
protocol for that relation, and it is the oldest unsolved problem in this
document by about three thousand years.

## Why three tiers and not one

The obvious objection is that this could be a single system, and we deliberately
made it three, for reasons that are structural rather than stylistic.

Each tier has a different anthropology, and mixing them produces incoherence.
AutonomousTrust deals with machines, anonymously, inside one trust boundary,
where anonymity is a protection and identity is not needed. Kith-Covenant deals
with named humans across a boundary neither side rules, where anonymity is the
failure mode rather than the protection, because what has to cross is a man
whose standing is at stake in what he commits. Those two stances are both
correct in their own scope and directly contradictory outside it.

Each tier is separately useful. A mesh with only the trust tier is a network
among machines that copes with bad peers, which is a complete and shippable
thing. A community with only the polity tier is a chartered body that keeps its
own records, which is useful whether or not it interconnects with anybody. The
compact tier assumes only that its parties are coherent communities, and does
not care how they became so.

And each tier can be replaced. Kith-Covenant depends on neither of the tiers
below it and does not know they exist. It sees communities and evidence about
communities, which ethne hands it in the vocabulary the compact already speaks.
Ethne in turn treats the trust substrate as pluggable, with AutonomousTrust as
one provider among possible others, and can be driven by a convener with a list
of names and no trust service at all. We made the couplings one-directional and
thin on purpose, so that a failure or a replacement at one level does not
propagate.

The independence is not merely claimed, since we check it mechanically. The
polity crates depend only on a neutral primitives crate shared with the compact
tier, the two places where polity code speaks the compact's own record types are
confined to a single crate, and the trust-substrate coupling is absent from the
default build entirely.

## What each tier actually gives a community

We state this concretely, because the abstractions above are easy to nod along
to and hard to act on.

From **AutonomousTrust**, a community gets a network that defends itself. Peers
that behave badly lose reach without anybody writing a rule or filing a ticket.
A compromised node is contained by what it had earned rather than by what
somebody remembered to forbid. Admission needs no certificate authority, so it
keeps working when the network is cut off from everything. And the whole
judgment is made at each node from local observation, which is why it survives
a partition that would leave a conventional zero-trust deployment either failing
open or failing closed.

From **ethne**, a community gets an institution rather than a habit. A written
charter says who the members are and what the community commits to. Offices,
conferred by the roll and revocable by it, mean somebody is authorized to speak
and everybody knows who. Every membership change, every office change and every
act of the polity is recorded, append-only and signed. A rite gives a member
what the community stands for, distinct from merely being added to a list. And a
monitor watches for the specific ways such a body decays, meaning attrition,
ceremonies that have become empty, members who stopped re-affirming, and
machines whose responsible human has gone quiet.

From **Kith-Covenant**, a community gets a way to deal with strangers that does
not require becoming them. Recognition works without a shared creed. Four
defined relations name where a community stands toward another, with movement
between them following a named ladder rather than an argument. Five commons are
governed jointly, covering life support, communications, navigation, knowledge
and rescue. Mediators are drawn by lot and rotated out before they accumulate
weight. A protected role exists whose whole job is to say aloud what the
institutions are becoming. And an explicit account of what the compact will not
do is the part that keeps it from turning into the sovereign it exists to avoid.

## Installing none, some, or all

The tiers are cumulative in dependency and not in obligation. Four
configurations are sensible.

A **bare mesh** suits a small deployment among people who already trust each
other, where the software the community has to review is smallest and there is
nothing social to govern. Most first deployments should start here.

A **mesh with trust** suits a network large enough that not everyone knows
everyone, or exposed enough that a hostile or broken node is a realistic
concern. This is the configuration for a county-scale deployment.

A **chartered community** suits a group that intends to persist, that has
decisions worth recording, and that expects to still exist when its founders do
not. The polity tier is worth installing before it is needed, since a charter
written during a crisis is written by whoever is winning.

A **compact of communities** suits the case where two or more chartered
communities have to deal with each other durably. This is the most elaborate
configuration and the one with the least operational experience behind it, and
its proving ground is Earth precisely because the stakes there are recoverable.

## Where this is going

We designed and largely built the whole stack for terrestrial communities, and
we aimed it at permanent habitation off Earth, because space removes every
escape hatch at once. A habitat cannot walk away from a bad neighbor, cannot
appeal to an authority close enough to help, and cannot federate under one
sovereign without either irrelevance or tyranny, since a decision taking forty
minutes to travel is not a decision anyone can be governed by. Whatever holds
people together out there must be voluntary, must tolerate light-lag, and must
bind parties who share nothing but the vacuum.

That is the design target. Earth is where it gets tested, and a compact that
holds where exit is one phone call away has demonstrated something a compact
enforced by isolation never could.

Part II begins one tier up, with the question of who may cooperate at all.

## Further reading

- [Trust as a live value](../lib/autonomous_trust/doc/concept.md): the first
  tier, in one chapter.
- [From cooperation to community](../lib/ethne/doc/concept.md): the second, and
  the precise gap it fills.
- [Cooperation without a sovereign](../lib/kith-covenant/doc/concept.md): the
  third, and the oldest problem in the book.
- [The AutonomousTrust integration](../lib/kith-covenant/doc/architecture/autonomous-trust-integration.md):
  how the tiers are wired together, and what deliberately does not cross.

---

*Next: [Trust as a live value](../lib/autonomous_trust/doc/concept.md)*
