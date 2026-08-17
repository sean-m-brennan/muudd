*Previous: [Who this is for](audience_profile.md)*

# Nodes and radios

A MUUDD node is a box on a pole. Inside it are one or more radios, a small
router, a battery, and a charge controller; outside it are an antenna and a
solar panel. It costs somewhere between two hundred dollars for a household
endpoint and five thousand for a ridgetop relay, it draws less power than a
light bulb, and it is expected to run for years without anyone visiting.

That last expectation drives everything else. A device that needs regular
attention is a device that will be abandoned, because the person who has to
climb the hill in February is a volunteer with a job. So the design is not
optimized for throughput, or for cost, or for elegance. It is optimized for the
number of times per year somebody has to go look at it, and the target is zero.

Three consequences follow immediately. The power budget has to close on solar in
the worst month of the year, with days of battery reserve, which caps the whole
node at tens of watts and rules out anything with a fan. The radios have to be
commodity parts on unlicensed spectrum, purchasable by anyone and replaceable
from a shelf, which rules out anything requiring a license or a contract. And
the network has to keep working when any given node is dead, which rules out any
arrangement where one box is load-bearing for everybody else.

This chapter covers the physical layer at the level of principle: what the tiers
of node are, how the radio choice gets made, and where the power comes from. The
numbers below are illustrative. A complete deployment for one Appalachian
county, priced to the connector and sized against real terrain, is [the rural
network study](rural-network.md).

## Four tiers of node

Nodes come in four kinds, distinguished by where they sit and what they are for
rather than by any difference in software.

**Ridgetop relays** form the backbone. Placed on summits with clear line of
sight to several valleys and to each other, they carry aggregated traffic over
long directional links. A ridgetop relay is the most expensive node in the
network, around five thousand dollars, most of which is mast, solar array, and
battery rather than radio. It typically carries two backhaul radios pointed at
neighboring ridges, one sector antenna serving the valley below, and a
low-bandwidth long-range radio for monitoring and emergencies.

**Valley relays** extend coverage down into the hollows where people actually
live. They take backhaul from a ridgetop node and distribute access to the
households around them, at roughly a third the cost and half the power draw of a
ridgetop. Where grid power is available at the site, it supplements or replaces
the solar array.

**Household endpoints** are the cheapest part of the network by two orders of
magnitude: an outdoor radio aimed at the nearest relay, an indoor router, and a
cable between them. Two hundred dollars where line of sight to a relay exists,
and up to six hundred where the terrain forces a longer-wavelength radio.

**Gateways** connect the mesh to the wider internet, and they are the tier the
design deliberately refuses to depend on. A gateway is an ordinary relay with an
uplink bolted on, whether fiber where fiber reaches, cable where it does not, or
a satellite dish on a ridgetop that already has the unobstructed sky view such
service requires. Several gateways are better than one, not for bandwidth but
because the network's whole claim is that it survives their loss.

## Choosing a radio

There is no single right radio, and the ones that look best on a specification
sheet are usually wrong. The governing trade is between frequency and
penetration, and it runs in exactly one direction: lower frequencies bend around
obstacles and pass through foliage, higher frequencies carry more data and need
to see where they are going.

That trade produces a rough ordering, and a real deployment uses several points
on it at once.

Firstly, at the low end, sub-gigahertz long-range modulation gets through almost
anything. Chirp spread-spectrum radios in the 900 MHz band achieve receiver
sensitivities down near negative one hundred forty decibel-milliwatts, which
buys links through forest and around ridges where nothing else works at all. The
cost is throughput measured in kilobits rather than megabits. This is useless
for carrying traffic and invaluable for everything else: network monitoring,
telemetry from the solar systems, emergency text when the backbone is down, and
a heartbeat that says a relay is still alive.

Second, television whitespace occupies the sweet spot for terrain. The vacant
broadcast channels between 470 and 698 MHz combine sub-gigahertz propagation
with usable broadband, five to twenty-five megabits per channel, over ten to
fifteen kilometers without line of sight. In rural country the band is largely
empty, with five to ten vacant channels typical, because there are few
broadcasters to protect. The costs are equipment price, which runs an order of
magnitude above commodity gear, and a regulatory obligation to query a
geolocation database before transmitting. For valley-to-ridgetop links where the
ridge is in the way, nothing else comes close.

Third, mid-band shared spectrum around 3.5 GHz sits between the two, with more
bandwidth than whitespace and better penetration than 5 GHz, at the price of
registering with a coordination service and yielding to incumbents. It suits
serving a cluster of households from a valley relay with partial line of sight.

Fourth, commodity WiFi at 2.4 and 5 GHz delivers by far the most throughput per
dollar, and it needs to see the other end. Within a valley community, from a
relay to the houses around it, over distances under a couple of kilometers, it
is the obvious answer and it is cheap.

Lastly, dedicated point-to-point equipment at 5.8 GHz carries the backbone.
Ridgetop to ridgetop, with proper antenna alignment and clearance, links of
twenty to fifty kilometers at hundreds of megabits are routine, and the
regulations specifically permit the high-gain directional antennas that make
them possible.

One technology is deliberately excluded. Amateur radio mesh, which repurposes
ham spectrum for packet networking with modified commodity hardware, is
attractive on range and cost and is fundamentally incompatible with this design:
Part 97 prohibits encrypting amateur transmissions. A network whose entire
premise is that operators cannot read the traffic they carry cannot run on
spectrum where concealing the content is illegal. Amateur mesh remains useful
alongside the network for emergency coordination and public information, and it
cannot carry MUUDD traffic.

## Spectrum, and why unlicensed

Every band above is unlicensed, and that is a design commitment rather than a
budget decision.

A license is a permission, and a permission is a thing that can be withdrawn. It
also attaches to a licensee, which means someone has to be the party of record,
which means there is a single point at which the whole network can be
administratively disabled. Unlicensed operation under Part 15 has neither
property. The rules constrain power and require that a device accept
interference, and within those constraints nobody has to approve anything.

The constraints are real and they shape the design. Effective radiated power
caps at four watts in the ISM bands, and the antenna gain rules trade transmit
power against directionality above six decibels of gain, which is why the
backbone uses tight directional links rather than shouting. The point-to-point
exception in the upper 5 GHz band relaxes that trade specifically for fixed
links, and it is the regulatory basis for the long ridgetop hops the backbone
depends on. Whitespace carries a database obligation, which is a dependency on
an outside service and therefore a genuine weakness, mitigated only by the fact
that the query result is cacheable and the channel assignment is stable.

## Power

The energy budget is the constraint that decides whether a site is viable, and
it is worked backwards from the worst month.

A ridgetop relay drawing seventy watts continuously needs about seventeen
hundred watt-hours a day. At a latitude around thirty-nine degrees north,
December yields roughly three peak sun hours; at eighty percent system
efficiency that requires about seven hundred watts of panel, so two
four-hundred-watt panels give a modest margin for cloudy days. Three days of
autonomy through an ice storm needs on the order of four hundred amp-hours at
twelve volts, which a two-hundred amp-hour lithium iron phosphate bank at
twenty-four volts supplies at eighty percent depth of discharge.

Those numbers move with latitude, weather, and load, and the method does not.
Size the panel against the darkest month rather than the annual average, size
the battery against the longest expected outage rather than a single night, and
pick the chemistry that tolerates cold and deep cycling. Adding a satellite
uplink to a ridgetop nearly doubles the load, which nearly doubles both the
panel array and the battery bank, and that cost is properly attributed to the
uplink rather than to the relay.

Lightning protection and grounding are not optional at these sites and are
frequently the difference between a node that lasts a decade and one that lasts
a season. Surge suppressors on every radio line, a proper ground rod, and heavy
copper to it are a few hundred dollars against a node that costs thousands.

## Repairability

A node that cannot be fixed by the people who own it is a node owned by whoever
can fix it. Three rules follow.

Every part is a catalog part. No custom silicon, no potted assemblies, no
firmware that only the vendor can flash. When a radio fails, the replacement is
ordered from the same shelf the original came from, or a different one, since
the network does not care which vendor's box is on the pole as long as it speaks
the same protocols.

Failures are local. A dead radio takes out one link. A dead relay takes out the
households behind it and reroutes everyone else. No failure anywhere takes out
the network, which is what permits repair to happen on a volunteer's schedule
rather than an emergency one.

Diagnosis happens remotely. The long-range low-bandwidth radio on every node
exists so that a failing site can be characterized from a warm room before
anyone drives out: battery voltage, panel current, radio link states, uptime.
Climbing the hill with the right part is the whole difference between one trip
and three.

## The same design, off Earth

Although the discussion above is terrestrial, the constraints that produce this
design are the constraints of space habitation with the difficulty turned up.

Power is again the binding budget, and again solar, with worse thermal
management and no convenient ground. Repairability is again the governing
requirement, and much harder, since the parts shelf is whatever was lifted or
can be made in situ, which is why the wider project treats recursive
bootstrapping as a first-class concern rather than an afterthought. Unattended
operation is again the target, and now genuinely mandatory rather than merely
economical.

What changes is the propagation problem. Between habitats there is no terrain,
no foliage, and no weather, and instead there is distance measured in
light-minutes. The radios get easier and the protocols get much harder, which is
the subject of [the timeout catalogue](../docs/space-protocol-timeouts.md) and,
further up the stack, of everything the upper tiers do about decisions that
cannot wait for a round trip.

## Further reading

- [The rural network study](rural-network.md): terrain analysis, RF propagation
  arithmetic, the full bill of materials for each tier, network topology, and
  cost against fiber.
- [Gateway uplinks](architecture/gateway-uplinks.md): modeling internet egress
  for the mesh.
- [Simulating a community mesh network](../docs/network_simulation.md): the tools
  used to size a deployment before buying anything.
- [Terrain to asteroid belt](../docs/system-simulation.md): the same simulation
  extended off Earth.

---

*Next: [What a person carries](device_model.md)*
