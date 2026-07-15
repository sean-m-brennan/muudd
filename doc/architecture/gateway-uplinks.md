# Gateway Uplinks

Internet egress modeling for the Appalachian mesh network simulation, grounded in
FCC Broadband Data Collection filings for Braxton County, WV.

## FCC BDC Data (June 2025)

The simulation's gateway placement is driven by real-world broadband availability.
FCC BDC filings as of June 2025 show the following wired service in Braxton County:

| Location | Provider | Technology | Down/Up (Mbps) | Coverage |
|-----------|----------|------------|-----------------|------------------|
| Sutton | Frontier | Fiber (FTTH) | up to 5000/5000 | Address-dependent |
| Gassaway | Frontier | Fiber (FTTH) | up to 7000/7000 | Address-dependent |
| Burnsville | Frontier | Fiber (FTTH) | up to 7000/7000 | Address-dependent |
| Flatwoods | Shentel | Cable (HFC) | up to 1000/35 | ~63% of homes |
| County-wide | Various | All wired | -- | Fiber <1% of homes |

Gateways are valley nodes, not hilltop relays. Fiber-to-the-home
service runs to the valley towns (Sutton, Gassaway, Burnsville) via Frontier, while
the I-79 corridor at Flatwoods has only Shentel cable. This inverts the common
assumption that interstate corridors carry the best broadband infrastructure. For the
mesh network, fiber gateway nodes belong at valley relay sites in the towns, not at
ridgetop relays near the interstate.

Sources: FCC National Broadband Map, BroadbandNow, Frontier local service pages,
Braxton County Digital Equity Plan (Sept 2024).

## GatewayUplink Model

A `GatewayUplink` is a `Configuration` subclass on `PeerInfo` that describes wired
internet egress for a single node (`simulator/peer/peer.py`). Fields:

- **technology** (`str`): `'fiber'`, `'cable'`, `'satellite'`, or `'dsl'`
- **bandwidth_down_mbps** (`float`): download capacity; must be > 0
- **bandwidth_up_mbps** (`float`): upload capacity; must be > 0
- **reliability** (`float`, default 0.99): probability the link is up at any
  given tick, in the range [0.0, 1.0]

All constraints are enforced by `icontract` preconditions, consistent with the
framework's design-by-contract pattern.

The `uplink` field is on `PeerInfo` (the persistent node definition), not on
`PeerConnection` (the transient connectivity snapshot), because internet egress is a
semi-permanent infrastructure attribute rather than per-tick RF state. Nodes without
an uplink are mesh-only relays; their behavior is unchanged.

## Per-Tick Reliability Simulation

During each tick, `Simulator.compute_step()` builds a `GatewayMap`
(`dict[str, GatewayUplink]`) by iterating over peers with an uplink and rolling
against each uplink's reliability:

```
for each peer with an uplink:
    if random() < uplink.reliability:
        include peer in GatewayMap
```

A reliability of 1.0 means the gateway is always available; 0.0 means never.
The `GatewayMap` is returned as part of `compute_step()`'s result tuple and passed
through to `SimState`.

## Aggregate Metrics on SimState

`SimState` carries the `GatewayMap` and four convenience fields computed by the
simulator before state construction, available to any downstream consumer (inspector
dashboard, Kith-Covenant evaluator) without modifying `MetricsCollector`.

| Field | Type | Description |
|-------------------------------|---------|-----------------------------------------------|
| `gateways` | `GatewayMap` | Active gateways this tick (peer UUID to uplink) |
| `gateway_count` | `int` | Number of active gateways |
| `aggregate_uplink_down_mbps` | `float` | Sum of active gateway download bandwidths |
| `aggregate_uplink_up_mbps` | `float` | Sum of active gateway upload bandwidths |
| `per_node_down_mbps` | `float` | Aggregate download / active peer count |

`per_node_down_mbps` is a proxy for per-household bandwidth; the denominator is
the relay node count, not actual households served.

## Appalachian Scenario Gateway Assignments

`create_appalachian_config()` in `simulator/scenarios/appalachian.py` assigns uplinks
to four valley nodes. Bandwidth estimates are conservative (1 Gbps symmetric for
fiber rather than the 5-7 Gbps max advertised) to reflect realistic business-tier
service for a community gateway.

| Node | Technology | Down/Up (Mbps) | Reliability |
|------------------------|------------|-----------------|-------------|
| `sutton_valley_1` | fiber | 1000/1000 | 0.99 |
| `burnsville_valley_1` | fiber | 1000/1000 | 0.99 |
| `gassaway_valley` | fiber | 1000/1000 | 0.99 |
| `flatwoods_valley` | cable | 1000/35 | 0.97 |

When `hilltop_only=True`, all valley nodes are excluded, so the scenario has zero
internet gateways. Hilltop-only mode is for backbone-only testing.

## Files

All paths relative to `autonomous-trust-simulator/autonomous_trust/`.

| File | Role |
|--------------------------------------|----------------------------------------------|
| `simulator/peer/peer.py` | `GatewayUplink` class; `uplink` field on `PeerInfo` |
| `simulator/sim_data.py` | `GatewayMap` type alias; gateway fields on `SimState` |
| `simulator/simulator.py` | Per-tick gateway resolution and metric computation |
| `simulator/scenarios/appalachian.py` | Valley-node uplink assignments |
