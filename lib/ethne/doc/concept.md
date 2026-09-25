*Previous: [A worked scenario](../../autonomous_trust/doc/example-application.md)*

# From cooperation to community

Nine machines that reliably deal with each other are not a community, they are a
cluster in a graph, and the difference shows up the first time anybody asks the
cluster to do something a cluster cannot do. Ask who its members are and no
answer comes back that anyone agreed to, only a boundary that emerged and keeps
moving. Ask what it stands for and there is nothing to report, because behavior
is not a commitment. Ask who may speak for it and nobody can, since speaking for
a group requires that the group have conferred that on somebody. Ask it to sign
an agreement and be held to that agreement next year, and the request is
incoherent, because the members will have turned over and the new ones never
agreed to anything.

*Ethne*, from the Greek for peoples, is the tier we built to close that gap. It
turns a set of cooperating agents into a polity, meaning a bounded,
self-governing community that stands for something, acts with one voice, and
persists across the turnover of its members.

One discipline governs the whole tier, and everything below elaborates it.

> Ethne reads the emergent structure, being the trust field, the cluster
> boundary, the earned reputations, and offers it back to the members for
> deliberate ratification into a charter. Emergence proposes, the polity
> ratifies, and the compact recognizes.

We could have imposed a structure from the top, which would have betrayed what
the trust layer is for, since a community handed down is a diktat rather than a
people. We could have relabelled the emergent cluster, which betrays what the
compact assumes, since the mask has no inside. The interesting engineering sits
in the ratification step, the moment a set of cooperators chooses to become a
bounded thing that stands for something and can act as one.

## The gap, precisely

The tier above this one takes a community as its atomic actor, and it quietly
presumes four things about that actor. It is *bounded*, meaning it has a known
membership. It is *coherent*, meaning it stands for a substrate and can speak
through delegates. It is *persistent*, meaning it outlives its current members,
without which periodic re-affirmation and immutable founding commitments make no
sense whatever. And it is *agentive*, meaning it can consent, be sponsored, and
be held responsible as a unit.

A trust substrate produces none of the four, because its groups and gateway
subtrees are emergent topological clusters rather than named, root-anchored,
covenant-bearing entities.

An earlier proposal tried to close this by mapping a trust group onto a
community identifier and making the gateway the external face of the community,
which is an adapter and does what adapters do. It gives the compact a handle to
bind to, and it quite deliberately does not build the inside of the community,
punting on which cluster, with what boundary, standing for what. A relabelled
cluster wearing a community mask is not a coherent, governed, durable polity. We
build the inside instead, which answers what that proposal punted, since the
boundary and the substrate an adapter binds are the ones our charter names.

## Reasons for separate tiers

The three tiers are built to stack, but the compact must not require the trust
substrate, and ideally must not require this tier either. We separated them
because the coherence and longevity of a community are a concern in their own
right, quite independent of whichever substrate happens to produce the
cooperation underneath.

The dependency directions, where an arrow means depends on:

```
   AT ──► ethne-at adapter ──► ethne ──► kc-core (identity, crypto, archive)
                                              ▲
   kith-covenant governance crates ───────────┘

   KC ⟂ AT        KC ⟂ ethne        ethne ⟂ AT
```

We build on the neutral primitives of the compact tier, being identity, signing,
and the append-only content-addressed log, and we reuse the community-scoped
machinery of that tier wherever it already exists. We do not depend on the trust
substrate at all.

That substrate is one pluggable provider, reached through an adapter that
depends on both. Swap it for a convener-driven provider, or for an importer over
the roster of an existing organization, and the tier still works, which is what
we mean by calling the substrate optional.

The compact depends on neither of us. We speak to it only through vocabulary it
already has, emitting a standard community identity plus recognition artifacts
it already understands, and surfacing a coherence breakdown as the
pre-indictment concern flag it already routes. The compact sees communities and
evidence about communities, and it never learns that either lower tier exists,
which is what makes the decoupling total.

We check that independence mechanically rather than asserting it. Every core,
monitor and host crate depends only on the neutral base, with no trust-substrate
dependency and no governance crate from the compact tier, and the two places
where this tier speaks the record types of the compact are confined to one crate
and go no further. Our end-to-end test founds a polity through the manual
provider with neither neighbor present.

## Core vocabulary

The following six terms are key in Part III.

A *polity* is the persistent community entity, identified by a root identifier,
and it is not equal to its current members, who merely join and leave it.

A *charter* is the founding signed artifact of a polity, holding the ratified
membership roll at founding, the articulated substrate, the internal offices,
and the amendment rules.

A *substrate* is whatever the polity holds above itself and defers to, being its
tradition, its governance, or its ground. For a polity that is mostly machines
the substrate is derived, and it anchors upward to human guardians.

A *membership roll* is the hard-edged signed set we produce by ratifying a soft
emergent boundary. It is versioned, and admissions and departures are recorded
as entries rather than edited in place.

An *office* is a conferred internal role, being convener, inductor, envoy
authorizer, or charter keeper, held until revoked and quite distinct from earned
reputation.

*Cohesion* is the measurable degree to which the polity still functions as one
coherent actor, meaning no schism, no internal capture, and no drifting
substrate.

## Five responsibilities

Firstly, *founding and charter* is the phase transition from emergent cluster to
chartered polity, an act in which a set of agents mutually and publicly commit
to a membership roll and a substrate, producing a signed charter with a genesis.
This is the keystone of the tier and the one genuinely new object in it.

Second, *substrate articulation* turns implicit emergent norms into an explicit
stance the polity defers to. The trust layer has behavior and the compact needs
a community that stands for something, so this is where standing for something
gets constituted and recorded.

Third, *internal offices* apply a category change inward. Earned reputation is
not conferred standing, and wiring one to the other numerically is a plain
category error, so we allow earned reputation as evidence in conferring an
office while the office itself is set by a deliberate signed act and holds until
revoked, never tracking a number. Neither neighboring tier lets a community
confer its own internal offices, since one has only emergent authority and the
other only inter-community roles.

Fourth, *cohesion and anti-fragmentation* is ongoing detection of schism,
capture, or drift within one polity, and telling the world when a community has
stopped being one coherent party.

Lastly, *longevity and continuity* covers transmission, meaning how a new member
acquires the substrate; succession, meaning offices persisting as officeholders
change; and identity persistence, meaning that the charter rather than the
member set is the entity. This tier is what makes the generational assumptions
of the compact coherent in the first place, because it is what creates the
persistent entity those assumptions are about.

## The life of a polity

```
   founding ──► operation ──► re-affirmation cycles ──► (schism | merger) ──► dissolution
      │            │                  │                        │
   ratify a     confer/rotate     transmit substrate       fission verdict → KC Concern;
   charter      offices; run       to new members;          a real split becomes two
   from the     cohesion           detect erosion           chartered polities (each
   substrate    monitor                                     re-recognized by KC)
```

We treat fission and fusion as first-class, because an emergent split must be
able to become two polities, each separately charterable and separately
recognizable by the compact, and two polities must be able to merge under a new
charter. That is exactly the boundary the compact cannot see on its own and the
trust substrate cannot decide on its own.

## The rest of Part III

The next chapter builds the charter, being the founding act, the roll, what a
founding says it excluded, and the offices a polity confers on itself. The one
after it settles which human answers for which machine, which is the relation
the compact asserts in prose and neither neighboring tier holds in its types.
Then comes the rite, being how a member actually receives the substrate rather
than merely appearing on a list, followed by the monitor that watches a polity
for the specific ways such bodies decay. The last chapter covers what crosses
the border into the compact, which is six kinds of artifact and nothing else.

## Further reading

- [Design decisions D1 to D35](design.md): the full record, including the
  reasoning behind every rule the next five chapters state.
- [The crates](crates.md): what each crate holds, and the checked dependency
  boundary.
- [The AutonomousTrust integration](../../kith-covenant/doc/architecture/autonomous-trust-integration.md):
  the analysis this tier answers, including the adapter proposal it completes.
- [Build and toolchain](architecture/build-and-toolchain.md): how the tier is
  built and the verification discipline behind the decisions.

---

*Next: [Founding a polity](charter.md)*
