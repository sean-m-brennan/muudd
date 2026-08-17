*Previous: [What a person carries](device_model.md)*

# What runs on a node

A relay on a ridgetop has no keyboard, no screen, and nobody within five miles
of it. Whatever software it runs has to start by itself, recover by itself, and
keep running through a winter without an administrator forming an opinion about
it. That is a harsher requirement than it sounds, and it is met by making the
software small rather than by making it clever.

The whole node software is a handful of small programs and a network stack,
built into an image with no shell, no package manager, and no general-purpose
operating system underneath. It cannot be logged into, because there is nothing
to log in to. It cannot be reconfigured remotely into something else, because it
has no mechanism for becoming something else. What it can do is forward packets,
decide which packets to forward, report its own health, and restart cleanly.

The four sections below cover the four commitments that produce that result: an
architecture of small composable processes, an operating system with nothing in
it that is not used, routing that works without a map, and an explicit position
on what a node can and cannot know.

## Small pieces, composed

The design follows the Unix tradition, and follows it further than most modern
systems do.

Each process does one thing. The routing daemon routes. The trust evaluator
scores peers. The telemetry reporter reports. None of them is a framework, none
of them is extensible, and none of them knows much about the others. They
communicate over well-defined interfaces carrying self-describing text rather
than through shared memory or a common object model, which costs a little
performance and buys the ability to inspect any interaction on the wire and to
replace either end without touching the other.

The reason is auditability before it is elegance. A community that owns its
infrastructure has to be able to review it, and review is a human activity with
human limits. A single ten-thousand-line program is not reviewable by a
volunteer in an evening. Six programs of a few hundred lines each, with the
interfaces between them written down and observable, are. The claim that this
software is open in the deepest sense stays empty unless somebody can actually
read it, and that is a constraint on size rather than on licensing.

Composability follows from the same commitment. A node that needs a capability
somebody has not written yet gets a new small process rather than a modification
to an existing one, which means the reviewed parts stay reviewed. It also means
a deployment can leave out what it does not need, and the parts it left out are
genuinely absent rather than merely disabled.

## A unikernel, not an operating system

A conventional node runs an application on top of a general-purpose operating
system: a kernel, a userland, an init system, a package manager, a shell,
libraries for a hundred things the application does not do, and a mechanism for
installing more. Most of that exists so a human can administer the machine, and
this machine has no administrator.

A unikernel discards it. The application and exactly the operating system
functions it calls are compiled together into a single bootable image. There is
one address space, one scheduler if any, and no facility for running a second
program, because there is no second program. The image boots in a fraction of a
second, occupies a few megabytes, and is byte-for-byte reproducible from source.

Three properties come out of this, and each answers one of the requirements on a
node.

Firstly, the attack surface is what the application actually uses and nothing
else. There is no shell to obtain, no package manager to abuse, no unused
network service listening, and no way to persist a change across a reboot, since
the image is immutable and any state that matters is written elsewhere
deliberately. An attacker who achieves code execution has achieved it inside a
program with no way to become another program.

Second, the failure mode is a clean restart. There is very little state inside a
running node that a fault can corrupt, so the response to almost any fault is to
reboot into the identical known image, which takes under a second and requires
nobody's attention. Self-sufficiency in the field is largely this: a system
whose answer to confusion is to become new again.

Lastly, verification is tractable. A small image built reproducibly from
reviewable source can be checked to be the image it claims to be, which is the
foundation the next chapters build on. A general-purpose distribution assembled
from thousands of packages cannot be checked in any meaningful sense by the
people running it.

The cost is real and worth naming. Unikernel tooling is less mature than
conventional tooling, debugging is harder without a shell to poke around in, and
the discipline of building everything into the image means a change that would
be a package install elsewhere is a rebuild and a redeploy here. That trade is
accepted because the alternative is a general-purpose computer sitting
unsupervised on a hill.

## Routing without a map

Mesh routing is where the philosophical commitments behind this architecture
become concrete engineering.

No node knows the topology. Links appear and disappear with the weather, the
foliage of the season, the battery state of a relay two ridges over, and whether
a tree came down. A node knows its own neighbors, knows what they have recently
told it, and knows how well its own recent traffic actually got through.
Everything else is inference, and the inference is stale.

Routing therefore proceeds from local information toward local decisions. A node
selects the next hop for a packet from the neighbors it can currently reach,
weighted by what it has observed about them, and it revises that continuously.
It does not compute a path to the destination, because it cannot; it makes one
choice and hands the problem to somebody closer.

Three properties of the mesh come from doing it this way.

Firstly, routing is *decentralized* in the strong sense, meaning no node exists
whose failure changes the ability of any other node to decide. A relay dying
removes its links and nothing else. There is no route server, no controller, and
no consensus about paths to be lost.

Second, routing is *socially regulated*, which is the part that distinguishes
this from a conventional mesh protocol. The weight a node gives a neighbor is
not purely a link-quality metric. It incorporates the standing of that neighbor,
accumulated from observed behavior over time. A peer that has dropped traffic,
replayed it, or flooded the medium becomes progressively less attractive as a
next hop, without anybody issuing a rule about it. Although this is the first
place in the stack where an upper tier reaches down and changes how the physical
layer behaves, it remains optional. A mesh with no trust layer routes on link
quality alone, and works fine among honest neighbors.

Lastly, routing *re-enters automatically* after a failure. A node that has been
isolated, whether by a dead relay, a partition, or its own reboot, rediscovers
its neighbors and resumes participating with no human involvement and no
distinguished rejoin procedure. It has no global state to reconcile, which is
exactly why this is easy: reconciliation is only hard for systems that thought
they knew something.

## What a node can know

Underneath all of the above is a position on knowledge that is worth stating
directly, because it explains several design decisions that otherwise look like
limitations.

Knowledge in this system is *local*. A node knows what it has observed and what
its neighbors have told it. It has no view of the whole and no way to obtain
one.

Knowledge is *situational*. What a node knows is knowledge about its own
position, and it does not transfer. When a neighbor reports link quality, the
report is about the radio, the antenna, and the terrain at that neighbor, rather
than a fact about the link in the abstract.

Knowledge is *perishable*. Everything a node believes was true at some past
moment, and the moment recedes. Nothing is cached indefinitely, everything
decays, and a stale belief is treated as weaker evidence rather than as fact.

Refusing global state is not a concession to the difficulty of maintaining it.
It is a rejection of the thing itself. A network component that held
authoritative global state would be the single most valuable thing in the system
to capture, and its existence would make every other decentralization measure
decorative. The same reasoning runs the whole length of this stack: it is why
trust is computed per node rather than issued centrally, why communities are
chartered by their own members rather than registered somewhere, and why the
compact between communities is structurally forbidden from becoming a sovereign.
Each tier refuses the same temptation at its own scale.

The consequence to accept honestly is that every decision in this system is made
on partial, aging, locally held information, and some of those decisions will be
wrong. The architecture is built for that: decisions are cheap, revisable, and
local, so a wrong one costs a retransmission rather than an outage.

## Further reading

- [Verifiable infrastructure](verifiable_infrastructure.md): what reviewability
  and reproducible builds are worth if the silicon underneath is untrustworthy.
- [Simulating a community mesh network](../docs/network_simulation.md): how
  routing behavior is exercised before deployment.
- [How a node is built](../lib/autonomous_trust/doc/architecture/overview.md):
  the same small-process discipline applied one tier up, where the trust
  evaluation actually happens.
- [Networking](../lib/autonomous_trust/doc/architecture/networking.md): the
  channel and message-routing layer that sits above this one.

---

*Next: [Speaking without being identified](anonymous_communication.md)*
