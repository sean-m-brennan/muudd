*Previous: [What runs on a node](software_architecture.md)*

# Speaking without being identified

Encryption hides what was said. It does not hide who said it, to whom, or when,
and in a small network those three facts are frequently the whole of what an
adversary wanted. Knowing that a particular household exchanged messages with a
particular neighbor at two in the morning is often more useful than knowing the
contents, and no amount of cipher strength conceals it.

This is an awkward chapter, because the community network makes the problem
worse rather than better. A message on the public internet hides in the traffic
of billions. A message on a mesh of four hundred people hides in the traffic of
four hundred people, most of whom are known to each other, several of whom
operate the relays it passed through. The anonymity of crowds is unavailable
here, since there is no crowd.

The design therefore has to buy anonymity deliberately, and it has to be honest
about the price. What follows sets out why anonymity is needed at all in a
network built on earned reputation, what the mesh makes hard, which techniques
transfer from the wider internet and which do not, and where the project has an
answer against where it has only a question. That last division matters. Several
of the questions below are open, and stating them as open is more useful than
pretending otherwise.

## Why a trust network needs anonymity

The apparent contradiction is worth facing first. The entire stack is built on
knowing who did what. Peers earn standing through observed behavior, communities
keep a membership roll, and the compact between communities credentials named
envoys whose personal reputation is at stake. Anonymity looks like the enemy of
all of it.

It is not, and the reason is that identity and anonymity serve different
functions that happen to run over the same wire.

Reputation exists to allocate scarce things. Bandwidth on a solar-powered relay,
compute on a small node, a place on a membership roll, the authority to speak
for a community. Those decisions require knowing who is asking, because the
whole content of the decision is a judgement about the asker.

Anonymity exists to protect speech that costs the speaker. Reporting that a
respected member is stealing from the commons. Dissenting from a decision the
community has already made. Asking a question that reveals ignorance, illness,
or doubt. In each case the speaker has something true to say and a well-founded
fear of what saying it will cost, and in each case the community is better off
hearing it.

A network that offers only the first function silences the second. Worse, it
silences it exactly when the second matters most, since the pressure not to
speak rises with the seriousness of what there is to say. A community where
nobody can report misconduct without being identified is a community where
misconduct goes unreported, and it will discover this at the worst possible
time.

The stack therefore needs both, and needs them cleanly separated. The upper
tiers say this explicitly and build for it: the observer and whistleblower
machinery in Part IV lets a source prove membership in a community without
revealing which member it is. That machinery assumes a transport that does not
undo the guarantee, which is what this chapter is about.

## What the mesh makes hard

Four properties of a community mesh work against anonymity, and all four are
consequences of things the design wanted for other reasons.

Firstly, the anonymity set is small. Techniques that work on the public internet
rely on a large population of indistinguishable users; a mesh of a few hundred
provides a set small enough that traffic analysis over a few days narrows almost
any flow to a handful of candidates.

Second, the topology is knowable and stable. Relays sit in fixed places, links
follow terrain, and the graph does not change much. An observer who knows the
topology, and everyone does, can infer a great deal about a path from very
little about a packet.

Third, the relays are neighbors. On the public internet the intermediate hops
belong to parties with no interest in any particular user. Here they belong to
people who know the parties personally and may care intensely about what passes
between them. The operator is not a disinterested carrier.

Lastly, capacity is scarce. Cover traffic, padding, and multi-hop redundancy are
the standard defenses, and each of them multiplies bandwidth on links that are
already the binding constraint and on nodes running off a battery. A defense the
network cannot afford to run is not a defense.

## What transfers, and what does not

Onion routing wraps a message in successive layers of encryption, one per hop,
so each relay learns only its predecessor and successor. The construction
transfers to a mesh without modification and buys less here than it does
elsewhere, because its guarantee scales with the number of plausible paths and
the mesh has few. On a network where most pairs of endpoints have two or three
viable routes, an observer at any relay excludes most of the graph immediately.

Mix networks batch messages, hold them, reorder them, and release them together,
which defeats timing correlation at the cost of latency. This transfers better
than onion routing does, because the mesh already tolerates delay, already
store-and-forwards across partitions, and already has no expectation of
interactive response for most traffic. Although a mix adds seconds or minutes,
the classes of message that most need anonymity are precisely the ones where
minutes are acceptable. A whistleblower report is not a phone call.

Cover traffic is the technique the mesh can least afford and most needs.
Constant padding between every pair of relays would defeat nearly all traffic
analysis and would also consume the link budget the network exists to provide. A
partial answer is to pad only the low-bandwidth long-range channel, which
already runs continuously for monitoring, carries kilobits rather than megabits,
and is exactly the right size for the short high-consequence messages anonymity
is for.

Dead drops sidestep the routing problem entirely. A message deposited at a
location on the mesh and collected later by whoever knows to look breaks the
link between sender and receiver in time rather than in space, and the mesh is
well suited to holding such things, since store-and-forward is already how it
works.

## Where anonymity meets standing

The unresolved question is not how to hide a sender. It is how a network that
allocates resources by reputation handles a participant that has deliberately
suppressed its identity.

The tension is exact. Anonymous traffic cannot be charged against anybody's
standing, so it cannot be metered by the mechanism the rest of the network uses
to prevent abuse. An anonymity service that anyone may use without cost is a
flooding tool. An anonymity service that requires standing to use is not
anonymous.

Three partial answers exist, and none is complete.

The first is to bound the channel rather than the user. The anonymous path gets
a small fixed share of capacity, community-wide, allocated first-come. Flooding
it degrades only the anonymous channel and harms the flooder along with everyone
else, and the ordinary network is unaffected. This is cheap and it lets a
determined attacker deny the anonymous channel to everybody, which is a
meaningful attack when the channel exists for reporting misconduct.

The second is to prove membership without proving identity. A sender
demonstrates cryptographically that it is one of the members of a known group,
without revealing which, so the traffic is chargeable against the group and
refusable from outsiders. This is the ring-signature construction the observer
machinery in Part IV uses, and it is the strongest answer available. It requires
a defined group with published keys, which is to say it requires the community
tier, so it is unavailable to a bare mesh with no upper tiers installed.

The third is to make the transition explicit. A participant may act anonymously,
and may later choose to attach a claim to an identity, and the network records
that a transition happened without being able to retroactively deanonymize what
came before. This is useful for the common case where a source speaks
anonymously first and is willing to be named once the matter is taken seriously.

## Open questions

Stated plainly, since the project does not have answers to them yet.

How much cover traffic is enough on a network this small, and can the low-rate
monitoring channel carry the whole burden? What does a mix strategy look like
when the batch is drawn from four hundred participants rather than four million?
How does an anonymous channel behave across a partition, when the mix that was
holding a message is on the other side of a downed relay? Can a bare mesh, with
no community tier and therefore no group to sign for, offer anything better than
a bounded shared channel? And how should the network handle the case where the
anonymity set is one, meaning traffic analysis has succeeded and the sender does
not know it?

The last one may be the most important, since a false belief in anonymity is
worse than a known absence of it.

## Further reading

- [Protected dissent](../lib/kith-covenant/doc/jester.md) and
  [whistleblower cryptography](../lib/kith-covenant/doc/architecture/whistleblower-crypto.md):
  the ring-signature machinery, and the reception panel that holds a source
  identity under threshold custody.
- [Becoming a peer](../lib/autonomous_trust/doc/architecture/identity-protocol.md):
  anonymous participation at the trust tier, and what it does and does not
  conceal.
- [Simulating a community mesh network](../docs/network_simulation.md): the
  framework in which traffic-analysis resistance would be measured.

---

*Next: [Trusting the metal](verifiable_infrastructure.md)*
