*Previous: [Why user-owned networks](muudd.md)*

# Who this is for

Every network encodes an assumption about its users, and most of the assumption
is invisible until something built on a different one shows up beside it. The
public internet assumes an enormous population of strangers who will never meet,
whose behavior must therefore be constrained by terms of service and moderated
after the fact, and whose attention is the thing actually being sold. Those
assumptions produce the architecture we have: central identity providers,
central moderation, central billing, and a permanent record of who said what.

MUUDD assumes something else. It assumes a few hundred people who mostly know
each other, who live within radio range, who have reasons beyond convenience to
keep faith with one another, and who will still be there in ten years. That is a
smaller and stranger assumption, and nearly every design decision downstream
follows from it.

This chapter states the assumption plainly, because a reader who does not share
it will find the rest of the design perverse. A system that refuses to scale to
millions, that will not admit a stranger without a voucher, and that expects its
users to govern themselves is not an inferior version of a consumer platform. It
is a different thing aimed at different people, and the people are named here.

## The remnant

The target community is small, deliberate, and morally serious. Its members have
concluded that the institutions they were raised to rely on are failing, and
have decided to build replacements rather than complain or emigrate. They are
not survivalists stockpiling against collapse in isolation, and they are not
activists organizing a protest. They are households that intend to still be
households in a generation, in a place, together.

Four traits matter for the design, and each one buys something a general-purpose
network cannot have.

They are **dedicated**. Participation costs real money and real labor: a mast to
raise, a panel to mount, a radio to aim, an outage to drive out and fix in
February. That cost is a filter, and it is not an unfortunate side effect of
building cheaply. A network that anyone can join for free at zero effort is a
network that anyone can flood, and the flooding is the ordinary case rather than
the attack. Costly entry is the oldest and most reliable signal there is.

They are **moral**, in the specific sense that they hold themselves accountable
to something above their own preferences. This is the load-bearing assumption of
the whole stack, and the upper tiers make it explicit: a community that answers
to nothing beyond itself will eventually discover that its own convenience is
the highest good it can name, and will act accordingly. What that authority is
varies by community, and the protocol takes no position. That there is one is
not optional.

They are **responsible**, which is to say the network is theirs to run. There is
no support number. A relay that goes down stays down until somebody in the
community climbs the hill, and the person who climbs the hill is the person who
benefits from the link. Ownership and obligation land on the same shoulders,
which is the arrangement that has always produced maintained infrastructure and
the one that mass-market service deliberately severs.

Lastly, they are **few**. Not few in aggregate, since there is no limit on how
many such communities exist or how they interconnect, but few within any one
compact. Somewhere between a large extended family and a small village: enough
people that the work is divisible, and few enough that reputation is a real
thing rather than a number on a screen.

## Troop, not herd, and not dust

Three social arrangements are available to a technological society, and only one
of them works.

A **herd** is a mass: many individuals, no internal structure, coordinated only
by whatever moves the whole group at once. Herds are what mass media and mass
politics address, and they are trivially steered, since a signal injected at the
front propagates without resistance. Nothing in a herd knows anything about
anybody, so nothing in a herd can check a claim.

**Dust** is the opposite failure, and it is the present one. Atomized
individuals, each sovereign over nothing much, connected to institutions but not
to each other, transacting through intermediaries who know all of them and whom
none of them know. The isolated individual is easy to service and easy to
dispose of, and he cannot resist anything, because resistance requires two
people who trust each other more than they fear the consequence.

A **troop** is the arrangement in between and the one this design targets: a
group small enough that every member is known to most of the others, with
internal structure that emerged rather than being imposed, and with obligations
that run in every direction rather than upward to a center. A troop can act
together without being told to. It can absorb the loss of any one member. It can
judge one of its own, which is the capacity that neither the herd nor the dust
possesses, and it can do so because the judges know the man and have a stake in
both his welfare and the group's.

Everything about the architecture is an attempt to build machinery that a troop
can use without dissolving into one of the other two. The routing is
peer-to-peer because a hub would recreate the herd. Trust is earned through
observed dealings because a central rater would recreate the herd's
steerability. Communities are chartered with a membership roll and internal
offices because without them a troop is only a crowd that happens to be standing
together, and it will scatter under the first real pressure.

## What the assumption buys, and what it costs

Assuming a troop rather than a market makes several hard problems easy and
several easy problems hard. Both directions are worth stating, since the trades
are not obviously favorable and were not made casually.

Made easy: admission control, since a voucher from an existing member is
available and meaningful. Abuse, since a member who misbehaves is known, is
present, and has more at stake than an account. Governance, since decisions can
be made by people who will live with them. Key distribution, since bodies can
meet in a room. Naming, since a network of a few hundred needs no global
namespace and can use the names people already use.

Made hard: growth, since every new member is a deliberate act by the existing
ones rather than a signup. Reach, since a compact that will not admit strangers
cannot become universal by admitting everybody, and must instead federate with
other compacts, which is the entire problem Part IV exists to solve. Anonymity,
since a network where everyone is known is a network where dissent is expensive,
and the design has to purchase anonymity back deliberately rather than getting
it free from the anonymity of crowds. Succession, since a troop that cannot
induct its young becomes a troop of old men and then nothing.

Those four costs are not incidental. Growth, reach, anonymity, and succession
are precisely the concerns the upper tiers exist to address, and the reader who
wonders why a mesh network project ends up specifying membership rites and
escalation ladders will find the answer here. Choosing the troop makes the
technical problems tractable and moves the hard problems into the social layer.
That is a trade the project makes on purpose, on the grounds that the social
problems are the real ones and the technical difficulty of the alternative was
never the thing standing in the way.

## The space case

Although the profile above is drawn from terrestrial communities, the same shape
is forced by permanent habitation off Earth, and forced much harder.

A habitat crew is a troop whether anybody designed it that way or not. It is
small, because mass costs money to lift. Its members are interdependent to a
degree with no terrestrial analogue, since the same air supply keeps all of them
alive. It cannot expel a member without killing him, cannot admit a stranger
casually, and cannot appeal to an outside authority in any useful time. Exit is
unavailable in both directions.

That is the same profile with the escape hatches welded shut, which makes it the
sharpening case for every mechanism in this stack. A design that works for a
county in Appalachia and a design that works at L5 are the same design under
different amounts of pressure, and the terrestrial version is where the mistakes
are affordable.

## Further reading

- [The rural network study](rural-network.md): the ownership model section, and
  the precedent networks that have made community ownership work.
- [Cooperation without a sovereign](../lib/kith-covenant/doc/concept.md): what
  happens when troops that do not share a creed have to deal with each other.
- [From cooperation to community](../lib/ethne/doc/concept.md): the machinery
  that keeps a troop a troop as its members turn over.

---

*Next: [Nodes and radios](hardware_design.md)*
