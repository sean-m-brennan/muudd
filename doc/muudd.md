*Previous: [MUUDD](../README.md)*

# Why user-owned networks

Communication is the first thing a hostile power takes. Not the roads, not the
banks, not the food supply: the ability of ordinary people to say true things to
each other and be believed. Everything else follows from that, which is why
every serious attempt at control begins there, and why the counter-move has to
begin there too.

The tools available today are all borrowed. Phone service, messaging, mail, and
the platforms where public conversation happens are owned by parties who can
revoke access on their own judgement, and who log everything meanwhile. That
arrangement is tolerable while those parties remain indifferent. It stops being
tolerable the moment they do not, and the transition takes no notice at all: the
same handset, the same app, the same wires, and a different answer to the
question of who may speak.

MUUDD is the alternative built from the bottom. Solar-powered radios on
ridgetops and rooftops, each one owned by the household or the community that
installed it, passing traffic to each other directly. Nobody bills for it.
Nobody can switch it off from a distance. It survives a grid failure, a
carrier's decision, and a storm that takes out the fiber, because it does not
depend on any of the three.

What follows is the argument for building it, which is not primarily a technical
argument, and then the design that argument produces.

## Biblically-aligned networks, or software as God intended

As of this writing, our dominant culture, and thus indeed at least the
Anglophone world at large, is in a deep spiritual crisis, a tribulation of
unprecedented reach. There is a sense of oppression and disconcertedness that is
not yet demonstrable physically, yet is on the cusp of manifesting: "what rough
beast, its hour come round at last, Slouches towards" [^1] us? There is a moral
void in this age of atomization. Although believers yearn for fellowship,
corruption and deceit abound among neighbors and permeate institutions, leading
to universal distrust. Entropy has the upper hand.

So war is upon us. Currently just primarily in spirit, but bleeding into the
kinetic soon enough, war is always a test of a people's ability to cooperate and
coordinate. The Cloud People have their cloud computing: powerful, complex (too
complex), tightly interconnected (precariously), centralized and thoroughly
monitored. Too monitored. It is quite easy to fall from the cloud with an impure
thought. But believers already know: "all have sinned and fall short of the
glory of God" [^2]. We're already fallen, already in the dirt toiling for a
living. All the instruments and tools of ordering our society for the Good have
been taken from us (though they stare us in the face). So we need new tools to
open lines of communication, to speak the forbidden Truth, to trust again in men
of goodwill, and to shun both the traitor and those who curse God.

### God created the World to train the Faithful [^3]

We have a duty to God and to our posterity to resist what is coming, indeed what
is already here. But the wicked are in our midst, and as we've already seen they
will happily guide us to worldly doom. Amongst us is courage and anger and
disgust aplenty, but also an utter inability to organize or even recognize a
valid leader. The cypherpunks were right about private communications being an
antidote to tyranny, but they largely hold to a different and dying faith, civic
humanism. Thus, they failed in their crusade to overthrow an immoral Big Mother
government *because* they sought to replace it with amoral chaos. We can take
the digital tools with which they were trying to achieve (self-defeating)
anarchy and apply them more completely, in ways that build social order instead
of further isolating the "sovereign individual".

We know collectivism is bad (observe the 20th century), but so is individualism
(vis. The 21st). Neither nation nor citizen are fundamental. We must walk a fine
line (narrow path!) between the two as a society. Whether it is implicit or
explicit, long-term cooperation and peace requires a shared faith, which creates
a synthetic tribe. (Doubtful? How's that multiculturalism working out?) Sharing
faith requires the ability to communicate candidly about this world and the
next, which we are no longer allowed to do. Christianity is a grassroots faith
(the faith, not the religion), sharing the living gospel person-to-person in
ongoing interactions. The powers-that-be believe that Christianity is no longer
a threat because the various institutions of the religion have been subverted,
but the faith itself is yet alive, despite everything. Greater tribulation is
coming, however.

What we need is to be able to identify friend and foe, to trust others, not
blindly but tangibly. We need to do this anonymously, at least at first, due to
the high social and physical cost of self-revelation. We should also then be
able to build new institutions that are both decentralized and founded on strong
personal bonds, making them harder to attack externally and internally. All this
must not rely on the rather fragile and highly monitored internet or electrical
grid.

We already have a blueprint for this social organization from the Old Testament:
only a hierarchy of tribal elders, **patriarchy**, can judge a man, because they
both know him and have an interest in not just his welfare, but also that of the
wider tribe. But Christ brought the sword to cleave kin apart if they
disbelieve, and he wields it still. That does not change the optimality of the
patriarchal structure though, the tragic caveat being that blood kin are not
necessarily spiritual kin.

How do we implement such an order in the age of unlimited immigration, gay
rights and feminism? The solution consists of several tiers of organization:
hardware, software and human.

### Hardware

*All* institutions are failing, including those that provide critical
infrastructure. Rather than seek to merely patch what exists, we should extend
current communications and power networks, while utilizing what exists.

That principle sets three hardware constraints, and the design follows from
them. Firstly, a node has to run without the grid, because the grid is one of
the institutions in question: solar panel, battery bank sized for several days
of winter cloud, and a total draw small enough that the arithmetic closes at
seventy watts rather than seven hundred. Second, the radios have to be
unlicensed, commodity parts anyone can buy and replace without asking anybody's
permission, which rules out spectrum that requires a license and rules in the
Part 15 bands, television whitespace, and the sub-gigahertz long-range links.
Lastly, a node has to be repairable by the people who own it, with hand tools
and a parts list, because a device that must be shipped somewhere for service is
a device somebody else controls.

What comes out of those constraints is a tiered arrangement: relays on ridgetops
with line of sight to several valleys, relays down in the hollows where people
actually live, and an endpoint at each household. Gateways to the wider internet
exist where they are convenient and are deliberately not load-bearing, since the
network's whole value is that it keeps working when they go away.

The full treatment is [chapter 3](hardware_design.md), and a complete worked
deployment for one Appalachian county, priced to the connector, is [the rural
network study](rural-network.md).

### Software

Hardware that moves bits is not yet a network anyone can use. The software layer
has to do three things the hardware cannot.

It has to route without a map. No node in a mesh knows the whole topology, and
any node that thought it did would be wrong within the minute, since links come
and go with the weather, the foliage, and the battery state of a relay two
ridges over. Routing is therefore local, situational, and continuously revised,
which is an epistemological constraint before it is an engineering one.

It has to be small enough to audit. A node running a general-purpose operating
system carries millions of lines nobody in the community has read, and every one
of them is a place for something to hide. The answer is a unikernel: the
application and only the parts of an operating system it actually calls,
compiled into a single image with no shell, no package manager, and no way in
that the design did not put there. Around that sit small task-specific processes
in the Unix tradition, composable and individually reviewable.

It has to fail and recover without an administrator. Nobody is driving to a
ridgetop in February. A node that loses its neighbors has to keep working on its
own, and rejoin cleanly when the link comes back, with no human in the loop and
no state that a partition can corrupt.

That is [chapter 5](software_architecture.md). Two further concerns get their
own chapters: routing that does not reveal who is talking to whom ([chapter
6](anonymous_communication.md)), and knowing that the silicon and the supply
chain behind a node are what they claim to be ([chapter
7](verifiable_infrastructure.md)).

Everything above that is optional, and is the subject of Parts II through IV.

### Human

The hardware and the software are the easy tiers. A network of honest neighbors
needs neither, and a network of liars is not saved by either.

The human tier is where the essay's argument lands, and it decomposes into three
questions that have to be answered in order.

Firstly, who is trustworthy? Not who holds a valid credential, since credentials
are issued by institutions and the institutions are the problem. The question is
who has behaved well, observably, over time, in the specific dealings that
actually happened. That question can be answered by machines watching each
other, which is what **AutonomousTrust** does, and it can be answered
anonymously, which matters exactly as much as the essay says it does:
self-revelation is costly, and a man who must identify himself before he can
participate will not participate.

Second, what is a community? Trust among individuals is not yet a body that can
act. A tribe has a boundary, a memory, offices, and a way of bringing the young
in, and it persists as its members die and are born. The elders of the Old
Testament model are not a reputation score; they are an institution with a roll
and a charter, however unwritten. **Ethne** is the tier that turns cooperation
into that institution, and it does so by ratifying the bonds that already formed
rather than by imposing a structure from outside, because a tribe assembled by
decree is not a tribe.

Lastly, how do such communities deal with one another? Blood kin are not
necessarily spiritual kin, and spiritual kin are scattered across communities
that will not agree on much. Two congregations, two counties, two habitats: they
need to trade, to rescue each other, to keep communications open, and to settle
disputes without either submitting to the other or pretending to a shared creed
they do not have. **Kith-Covenant** is the civil protocol for that, and its
central rule is the one the essay already stated: each party answers to
something above itself, and the arrangement between them is never permitted to
become that thing.

None of the three is required to send a message. All three are required to build
something that outlives the men who built it.

[^1]: W. B. Yeats, "The Second Coming", written in 1919 just after the flu
    pandemic and at the beginning of many years of warfare in his native Ireland.
    https://en.wikipedia.org/wiki/The_Second_Coming_(poem)

[^2]: Romans 3:23

[^3]: Paraphrasing Frank Herbert's *Dune*

## To infinity

### Space

Why? Because it's there. More importantly, expansion into space provides a new
and infinite frontier, an antidote to the current civilizational stagnation we
see today. Providing an escape hatch for young men of adventurous spirit,
because it's there.

But our current tech is unsustainable, our supply chains tenuous and fragile.
Our technology, specifically anything going into space, must be eminently
repairable with basic tools and resources through recursive bootstrapping.

Space is also the setting that makes the human tier unavoidable rather than
merely desirable. On Earth a bad neighbor can be left behind; between habitats
separated by light-lag there is nowhere to walk to, no authority close enough to
appeal to, and no way to federate under one sovereign without either irrelevance
or tyranny. Whatever holds people together out there has to be voluntary, has to
survive a twenty-minute round trip, and has to bind parties who share nothing
but the vacuum. That is the design target the upper tiers are aimed at, and
Earth is where it gets tested, because a compact that binds where exit is cheap
has proven something a compact enforced by isolation never can.

## Further reading

- [The rural network study](rural-network.md): a complete deployment for one
  Appalachian county, from terrain analysis through bill of materials to
  ownership model.
- [The case for decentralized communications](decentralization_momentum.md):
  shutdowns, censorship, digital identity, and the rise of mesh networking from
  2024 onward.
- [Infrastructure independence](decentralization_momentum_alt.md): the same case
  framed around preparedness, disaster recovery, and community self-reliance.
- [Simulating a community mesh network](../docs/network_simulation.md) and
  [terrain to asteroid belt](../docs/system-simulation.md): the simulation
  framework.
- [Space protocol timeouts](../docs/space-protocol-timeouts.md): what breaks in
  ordinary protocols at interplanetary latency.
- [AutonomousTrust reference](../lib/autonomous_trust/doc/autonomous_trust.md),
  [Kith-Covenant reference](../lib/kith-covenant/doc/kith_covenant.md): the two
  research corpora behind the upper tiers.

---

*Next: [Who this is for](audience_profile.md)*
