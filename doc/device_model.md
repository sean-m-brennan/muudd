*Previous: [Nodes and radios](hardware_design.md)*

# What a person carries

The previous chapter described infrastructure. Boxes on poles, owned by the
community, sited deliberately, running for years. This chapter describes the
other half of the network, which is the phone in a pocket.

The two halves could not be more different. A relay is chosen, mounted, and
maintained by people who understand it. A handset is whatever its owner happened
to buy, running whatever the manufacturer ships, updated on a schedule set
elsewhere, carried everywhere, and quite possibly compromised. The network has
no say in any of that and cannot acquire one.

So the device model starts from an uncomfortable admission. The network does not
trust the devices its own members carry, and it should not. Not because the
members are suspect, but because their hardware sits outside the control of the
community in every respect that matters. A phone is a general-purpose computer
built by a foreign supply chain, running an operating system its owner cannot
audit, in constant contact with services that log everything. Treating that as a
trusted participant would be a category error.

What follows from the admission is the whole design of the edge: devices connect
at one hop, get a bounded amount of reach, earn more through demonstrated
behavior, and never become part of the infrastructure. The relay is a citizen of
the network. The handset is a guest.

## What the network can assume about a device

Almost nothing, and the four assumptions worth stating are mostly negative.

Firstly, devices are *heterogeneous*. Phones, tablets, laptops, an ancient
desktop in a back room, an embedded sensor on a water tank. Different
architectures, different operating systems, different lifespans. Nothing can be
required that is not available everywhere, which in practice means a standard
network interface, a standard cryptographic suite, and nothing else.

Second, devices are *mobile*. A handset moves between relays several times a day
and leaves the network entirely for hours at a stretch. Any design that treats
the attachment point of a device as stable is wrong within the hour, and any
design that requires a device to be reachable is wrong overnight. Sessions have
to survive reattachment, and messages have to survive the recipient being gone.

Third, devices sit *at the edge*, one hop deep. A handset talks to the nearest
relay and to nothing else directly. It does not route for anybody, does not
carry traffic on behalf of another device, and does not participate in the mesh
proper. This is partly capability, since battery life and radio power make a
phone a poor relay, and mostly trust. A device the network cannot vouch for is a
device the network will not route through.

Lastly, devices are *replaceable and disposable*, as the network sees them.
Losing one is an ordinary event, not an incident. Nothing that matters may be
stored only on a handset, and nothing that matters may depend on a particular
handset being present.

## The network is suspicious of the user

Conventional network security worries about protecting the user from the
network. Captive portals, certificate warnings, and the whole apparatus of
transport security exist because the user is presumed honest and the intervening
infrastructure is presumed hostile.

MUUDD inverts the concern without abandoning it. The infrastructure is still
presumed potentially hostile, and the traffic is end-to-end encrypted
accordingly, so a relay forwards ciphertext it cannot read. On top of that, the
network is also suspicious of the device asking to connect, and it stays
suspicious for a while.

A device arrives with no standing. It is admitted to the extent that a member
vouches for it, which is a deliberate act by a person with something to lose,
and its reach expands only as its behavior warrants. That progression is not a
policy somebody wrote; it is the natural consequence of the trust model in Part
II, where a peer starts at zero and earns access along a gradient rather than
passing through a single gate. A device that behaves ordinarily accumulates
standing over days. A device that floods, probes, or misroutes loses it faster
than it gained it.

The practical shape of that gradient at the edge runs roughly as follows. At the
lowest standing a device gets local traffic only, meaning it can reach other
devices and services within its own community and nothing beyond. Above that it
gets forwarding across the mesh to other communities on the same network. Above
that it gets egress toward the outside world, which is the most expensive and
most abusable capability the network has. And at each level the amount is
metered, because bandwidth on a solar-powered relay is a genuinely scarce
resource and a single misbehaving device can exhaust it.

Two properties of this arrangement deserve emphasis. It is enforced at the
relay, locally, with no consultation of any central authority, so it keeps
working when the network is partitioned. And it degrades rather than gating: a
device that loses standing loses reach progressively rather than being
disconnected, which matters because most standing loss is a malfunction rather
than an attack, and disconnecting a neighbor over a misbehaving application is a
good way to lose a neighbor.

## Reaching the wider net

Although the mesh can carry traffic to the public internet, the relationship
between the two is deliberately awkward.

Access is *indirect*. There is no expectation that a device on the mesh has a
route to arbitrary internet destinations, and frequently it does not. Egress
exists at gateways, gateways are few, their capacity is shared among everyone
behind them, and the whole architecture is built on the premise that they may go
away. A design that assumed transparent internet access would fail the first
time that premise was exercised, which is precisely the case it exists for.

Naming is *local*. Global DNS is a hierarchy rooted in servers the community
does not run, reachable only through a gateway, and subject to interference at
several levels. The network does not depend on it. Instead each community
maintains a namespace of its own, holding short names for the services and
people on the local mesh, resolved locally, meaningful within the community and
nowhere else. Two communities may both have a name for their clinic, and the
names need not agree, because a global namespace is a global authority and the
entire project is an argument against having one.

Routes to the outside are handled as *bookmarks* rather than as a general
capability. A community maintains a set of known, named external destinations it
has decided are worth reaching, along with how to reach them, and those are what
a device gets when it asks for egress. Although this looks restrictive next to
unrestricted browsing, it buys three things. The scarce bandwidth at a gateway
goes to things somebody decided were worth it. The set of outside parties who
can observe traffic from the community stays small and known. And the network
keeps functioning unchanged when the general internet is unreachable, because
nothing important depended on it.

Although this arrangement resembles a corporate allowlist in mechanism, it
differs in who holds the list. The community writes it, the community can see
it, and any member can argue about it. That distinction is the whole point.

## What a device is for

Given all of the above, the honest summary is that a device on this network can
do less than a device on a commercial one, and the things it can do are things
that keep working when the commercial one stops.

It can reach the people and services in its own community, at full speed, with
no gateway involved, with the traffic never leaving the valley. It can message
somebody two communities over, with delivery that tolerates the recipient being
offline for a day. It can reach a known external destination when a gateway is
up and the community allotted it capacity. And it can do all of this without an
account at any company, without a subscription, and without a record of it
accumulating anywhere outside the community.

That is a smaller offering than a phone plan and a different one. It is also the
offering that survives the phone plan being switched off.

## Further reading

- [Trust as a live value](../lib/autonomous_trust/doc/concept.md) and
  [Standing turned into capability](../lib/autonomous_trust/doc/architecture/trust-tiers.md):
  the trust gradient that the edge admission policy is an instance of.
- [Speaking without being identified](anonymous_communication.md): what the
  device model gives up on privacy, and how some of it is bought back.
- [The rural network study](rural-network.md): the household endpoint bill of
  materials, and the trust-based admission section.

---

*Next: [What runs on a node](software_architecture.md)*
