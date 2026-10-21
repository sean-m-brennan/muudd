*Previous: [A rural deployment, worked in full](../rural-network.md)*

# Gateway uplinks

Internet egress modeling for the Appalachian mesh network simulation, grounded
in FCC Broadband Data Collection filings for Braxton County, West Virginia.

## Where the wires actually are

Gateway placement in the simulation is driven by real broadband availability
rather than by assumption. Filings as of June 2025 show the following wired
service in Braxton County.

| Location | Provider | Technology | Down/Up (Mbps) | Coverage |
|-----------|----------|------------|-----------------|------------------|
| Sutton | Frontier | Fiber (FTTH) | up to 5000/5000 | Address-dependent |
| Gassaway | Frontier | Fiber (FTTH) | up to 7000/7000 | Address-dependent |
| Burnsville | Frontier | Fiber (FTTH) | up to 7000/7000 | Address-dependent |
| Flatwoods | Shentel | Cable (HFC) | up to 1000/35 | ~63% of homes |
| County-wide | Various | All wired | -- | Fiber <1% of homes |

Gateways are valley nodes, not hilltop relays. Fiber-to-the-home service runs to
the valley towns (Sutton, Gassaway, Burnsville) via Frontier, while the I-79
corridor at Flatwoods has only Shentel cable. This inverts the common assumption
that interstate corridors carry the best broadband infrastructure. For the mesh
network, fiber gateway nodes belong at valley relay sites in the towns, not at
ridgetop relays near the interstate.

Sources are the FCC National Broadband Map, BroadbandNow, Frontier local service
pages, and the Braxton County Digital Equity Plan of September 2024.

## The uplink model

A `GatewayUplink` is a `Configuration` subclass on `PeerInfo` describing wired
internet egress for a single node, in `simulator/peer/peer.py`. It carries four
fields.

- `technology`, a string over `'fiber'`, `'cable'`, `'satellite'` and `'dsl'`
- `bandwidth_down_mbps`, a float download capacity, required positive
- `bandwidth_up_mbps`, a float upload capacity, required positive
- `reliability`, a float defaulting to 0.99, being the probability the link is up
  at any given tick, bounded to the unit interval

All four constraints are enforced by `icontract` preconditions, consistent with
the design-by-contract pattern used across the framework.

The `uplink` field is on `PeerInfo` (the persistent node definition), not on
`PeerConnection` (the transient connectivity snapshot), because internet egress
is a semi-permanent infrastructure attribute rather than per-tick RF state.
Nodes without an uplink are mesh-only relays, and their behavior is unchanged.

## Reliability, rolled per tick

During each tick, `Simulator.compute_step()` builds a `GatewayMap`, being a
dictionary from node identifier to uplink, by iterating over peers that have an
uplink and rolling against the reliability of each.

```
for each peer with an uplink:
    if random() < uplink.reliability:
        include peer in GatewayMap
```

A reliability of 1.0 means the gateway is always available, and 0.0 means never.
The map is returned as part of the result tuple and passed through to
`SimState`.

## Aggregate metrics

`SimState` carries the gateway map and four convenience fields, computed by the
simulator before state construction. Although a consumer could derive them, they
are precomputed so that the inspector dashboard, the compact evaluator, and
anything else downstream can read them without modifying the metrics collector.

| Field | Type | Description |
|-------------------------------|---------|-----------------------------------------------|
| `gateways` | `GatewayMap` | Active gateways this tick (peer UUID to uplink) |
| `gateway_count` | `int` | Number of active gateways |
| `aggregate_uplink_down_mbps` | `float` | Sum of active gateway download bandwidths |
| `aggregate_uplink_up_mbps` | `float` | Sum of active gateway upload bandwidths |
| `per_node_down_mbps` | `float` | Aggregate download / active peer count |

The per-node figure is a proxy for per-household bandwidth. Its denominator is
the relay node count rather than the households actually served.

## Gateway assignments in the scenario

The scenario builder in `simulator/scenarios/appalachian.py` assigns uplinks to
four valley nodes. Bandwidth estimates are deliberately conservative, being one
gigabit symmetric for fiber rather than the five to seven advertised, to reflect
realistic business-tier service for a community gateway.

| Node | Technology | Down/Up (Mbps) | Reliability |
|------------------------|------------|-----------------|-------------|
| `sutton_valley_1` | fiber | 1000/1000 | 0.99 |
| `burnsville_valley_1` | fiber | 1000/1000 | 0.99 |
| `gassaway_valley` | fiber | 1000/1000 | 0.99 |
| `flatwoods_valley` | cable | 1000/35 | 0.97 |

When the hilltop-only flag is set, all valley nodes are excluded, so the
scenario has no internet gateways at all. That mode exists for testing the
backbone by itself.

## Where the code lives

All paths are relative to `autonomous-trust-simulator/autonomous_trust/`.

| File | Role |
|--------------------------------------|----------------------------------------------|
| `simulator/peer/peer.py` | `GatewayUplink` class; `uplink` field on `PeerInfo` |
| `simulator/sim_data.py` | `GatewayMap` type alias; gateway fields on `SimState` |
| `simulator/simulator.py` | Per-tick gateway resolution and metric computation |
| `simulator/scenarios/appalachian.py` | Valley-node uplink assignments |

---

*Next: [Simulating a community mesh](../../docs/network_simulation.md)*
