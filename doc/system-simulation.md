# MUUDD System Simulation: Terrain to Asteroid Belt

A unified simulation framework for testing the full MUUDD software stack — AutonomousTrust identity/reputation/negotiation over realistic mesh networks, from Appalachian terrain through asteroid-belt inter-habitat scale. This document serves as both a technical reference and a living implementation checklist; steps are designed to be executed one at a time across sessions.

## Motivation

The MUUDD stack comprises three layers, each documented but not yet integrated end-to-end:

1. **Physical layer** — community mesh radio hardware and topology for rural Appalachia ([network.md](network.md))
2. **Trust layer** — the AutonomousTrust (AT) framework for encrypted, reputation-gated cooperative computing (`autonomous_trust/`)
3. **Protocol layer** — Politē group protocols for civil, trustworthy multi-party interaction (`polite/`)

Supporting these are a simulation tool survey ([network_simulation.md](network_simulation.md)) and a red-team evaluation recommending MITRE CALDERA ([eval.md](../eval.md)). The AT simulator already provides a working time-stepped connectivity engine with Docker/iptables network enforcement, but it uses a simplified inverse-square RF model without terrain awareness. What's missing is the integration path: terrain-realistic RF → AT protocol testing → adversarial evaluation → group protocol evaluation → deep-space scale.

This document fills that gap.

## Architecture

```
┌─────────────────────────────────────────────────┐
│              Polite (Group Protocols)            │  ← Phase 4
├─────────────────────────────────────────────────┤
│     AutonomousTrust (Identity / Reputation /     │
│          Negotiation / Network)                  │  ← Phase 2
├─────────────────────────────────────────────────┤
│   Red Team (CALDERA + Attack Scenarios)          │  ← Phase 3
├─────────────────────────────────────────────────┤
│      Router (iptables/tc on Docker network)      │
├─────────────────────────────────────────────────┤
│    Simulator (connectivity matrix per timestep)  │
├─────────────────────────────────────────────────┤
│  Terrain/RF Layer (SPLAT! path-loss matrices)    │  ← Phase 1
├─────────────────────────────────────────────────┤
│  Scenario Configs (Appalachian → Asteroid Belt)  │  ← Phase 1 / Phase 5
└─────────────────────────────────────────────────┘
```

Each layer builds on the one below it. The **Simulator** (`simulator.py`) already computes a per-timestep boolean connectivity `Matrix` via `PeerConnection.can_reach()` and feeds it to the **Router** (`routing.py`), which enforces reachability on Docker bridge networks using iptables DROP rules and tc traffic shaping. The phases below extend this stack downward (terrain-aware RF), upward (AT integration, adversarial testing, group protocols), and outward (deep-space scale).

## Existing Simulator Components

| Component | Path (relative to `autonomous_trust/simulator/`) | Role |
|-----------|------|------|
| `Simulator` | `simulator.py` | `SelectServer`; computes per-timestep connectivity matrix via `peer.can_reach()` |
| `SimConfig` / `SimState` | `sim_data.py` | Scenario definition (peers, times) and state snapshot (positions, `Matrix`) |
| `PeerInfo` / `PeerMovement` | `peer/peer.py` | Peer definitions with antenna, interface, signal strength; Bezier/ellipse movement paths |
| `Router` | `radio/routing.py` | Applies connectivity matrix to Docker containers via iptables + tc CBQ shaping |
| `NetInterface` / `Antenna` | `radio/iface.py` | `SMALL`/`MEDIUM`/`LARGE` bandwidth classes; `DIPOLE`/`YAGI`/`PARABOLIC` antenna gains |
| `create_config()` | `config.py` | Builds scenario configs from identity directories — pattern to follow for new scenarios |
| Dash UI | `dash_components/` | Real-time web visualization of simulation state |
| Docker/K8s orchestration | `../../Tiltfile`, `../../gen_compose.py` (AT root) | Runs real AT processes on bridge network with Router enforcing simulated connectivity |
| `EllipsePath` / `PointPath` | `peer/path.py` | Movement models including de La Hire ellipse construction (reusable for orbital paths) |

**Type definitions** in `sim_data.py`:
```python
Matrix = dict[str, dict[str, bool]]     # Connectivity: peer_id → {peer_id: reachable}
Map = dict[str, Ident]                  # Position snapshot: peer_id → Ident
```

**Current RF model** in `PeerConnection.can_reach()`:
```python
signal_strength = (1.0 / dist**2) * signal + antenna_gain
min_strength = -154 * 10 * log10(rate_bps)
return signal_strength < min_strength  # more negative = stronger
```

This is a basic inverse-square model with antenna gain. Phase 1 replaces it with terrain-aware path loss.

## Phase Dependencies

```
Phase 1 (Terrain RF) ──→ Phase 2 (AT Integration) ──→ Phase 3 (Red Team)
                                                   ──→ Phase 4 (Polite)  [parallel with 3]
                         Phase 2 + 3 + 4 ──────────→ Phase 5 (Space)
```

---

## Phase 1: Terrain-Aware RF Layer

**Objective:** Replace the boolean `can_reach()` with a signal-strength model fed by SPLAT! terrain data. Create the Appalachian scenario.

### Background

The current RF model computes path loss from distance alone. Real Appalachian terrain introduces knife-edge diffraction, Fresnel zone obstruction, and foliage attenuation that can add 20–60 dB beyond free-space loss. SPLAT! [1] uses the Longley-Rice ITM/ITWOM propagation model with SRTM elevation data to compute realistic path loss — the same model the FCC uses for regulatory analysis.

The network design in [network.md](network.md) identifies specific hilltop and valley sites in Braxton County, WV with worked link budgets at 900 MHz, 2.4 GHz, and 5.8 GHz. SPLAT! output validates those budgets against real terrain.

### Integration Points

1. **`PeerConnection.can_reach()`** in `peer/peer.py` — currently inverse-square + antenna gain vs threshold. Refactor to accept an optional external path-loss value (dB) and fall back to the current model when absent. This preserves backward compatibility with existing scenarios.

2. **`Simulator.compute_step()`** in `simulator.py` — calls `peer.can_reach(other)` for each pair. Pass terrain path-loss value when a path-loss matrix is loaded.

3. **`SimConfig`** in `sim_data.py` — add optional `path_loss_matrix` field of type `SignalMatrix`.

4. **New type** alongside existing `Matrix`:
   ```python
   SignalMatrix = dict[str, dict[str, float]]  # peer_id → {peer_id: path_loss_dB}
   ```
   This carries per-pair signal quality for bandwidth degradation, not just reachability.

5. **`Router`** in `routing.py` — use signal quality from `SignalMatrix` to set tc bandwidth rates proportional to link quality, not just block/allow.

### New Files (to create)

| File | Purpose |
|------|---------|
| `simulator/radio/terrain.py` | SPLAT! batch runner + CSV parser for pre-computed path-loss matrices |
| `simulator/scenarios/__init__.py` | Scenario package init |
| `simulator/scenarios/appalachian.py` | Braxton County node placement: 8 hilltop relays + 12 valley nodes + endpoints from [network.md](network.md) coordinates, stationary `PointData` paths, appropriate antenna/interface assignments |
| `tests/test_terrain_connectivity.py` | Loads sample path-loss CSV, verifies terrain-aware `can_reach` behavior |

### Verification Criteria

- **Backward compatibility:** Existing tests pass unchanged (fallback path when no terrain data is loaded)
- **Terrain-aware connectivity:** New test validates that a pair with high terrain path loss is correctly marked unreachable even at short distance
- **SPLAT! consistency:** Computed path-loss values for Sutton–Burnsville hilltop pair are consistent with [network.md](network.md) link budget (viable at 5.8 GHz with YAGI antennas, marginal at 2.4 GHz)
- **Bandwidth shaping:** Router tc rates degrade proportionally with signal quality

### Steps

- [ ] Download SRTM tiles N38W081, N38W080 for Braxton County
- [x] Create `radio/terrain.py` with SPLAT! batch runner and CSV path-loss parser
- [x] Add `SignalMatrix` type alias to `sim_data.py`
- [x] Add optional `path_loss_matrix: SignalMatrix` field to `SimConfig`
- [x] Refactor `PeerConnection.can_reach()` to accept optional terrain path-loss (dB), falling back to inverse-square when absent
- [x] Update `Simulator.compute_step()` to pass terrain data to `can_reach` when available
- [x] Create `scenarios/__init__.py` package
- [x] Create `scenarios/appalachian.py` with Braxton County node definitions from [network.md](network.md)
- [x] Write `test_terrain_connectivity.py` with sample path-loss CSV fixture
- [ ] Run SPLAT! batch for all node pairs at 900 MHz, 2.4 GHz, 5.8 GHz
- [ ] Validate SPLAT! results against [network.md](network.md) link budgets (Sutton–Burnsville should be viable at 5.8 GHz)
- [x] Update `Router` to read `SignalMatrix` and set tc bandwidth rates proportional to link quality

---

## Phase 2: AutonomousTrust Integration Testing

**Objective:** Run real AT processes over the terrain-aware simulated network. Measure protocol performance under realistic Appalachian mesh conditions.

### Background

The AT framework runs as a pool of cooperating processes — Identity, Reputation, Negotiation, Network — each communicating via multiprocessing queues. The existing Docker/Tilt orchestration (`Tiltfile`, `gen_compose.py`, `run_mission.sh`) already deploys AT nodes on a Docker bridge network with the `Router` enforcing simulated connectivity via iptables. Phase 2 extends this to use the terrain-aware Appalachian scenario from Phase 1 and adds systematic metrics collection.

### Integration Points

1. **Docker/Tilt orchestration** — existing `Tiltfile` + `gen_compose.py` create AT nodes on bridge network. Router already enforces simulator connectivity via iptables. The Appalachian scenario requires a custom compose generator that maps Braxton County nodes to AT identities.

2. **`SimMetadataSource`** in `peer/peer_metadata.py` — bridge between simulator and AT process. Receives `SimState`, updates peer position metadata. This is the mechanism by which AT processes learn their simulated location.

3. **Composition pattern** — follow the launch pattern in `mock.py` `__main__` for combining Dash UI + Simulator + AT processes in a single orchestrated run.

### New Files (to create)

| File | Purpose |
|------|---------|
| `simulator/scenarios/appalachian_compose.py` | Docker-compose generator for Appalachian scenario (extends `gen_compose.py` pattern) |
| `simulator/metrics/__init__.py` | Metrics collection package |
| `simulator/metrics/collector.py` | Collects: identity admission convergence time, reputation score stability, negotiation RTT, bandwidth overhead |
| `tests/c_system/test_appalachian_scenario.py` | System test using testcontainers pattern |

### Metrics

| Metric | Description | Target |
|--------|-------------|--------|
| **Identity convergence** | Time for all nodes to complete mutual identity admission | < 60s for 20 nodes |
| **Reputation stability** | Standard deviation of reputation scores after convergence | σ < 0.05 under non-adversarial conditions |
| **Negotiation RTT** | Round-trip time for a negotiation exchange | Reflects simulated link latency |
| **Bandwidth overhead** | AT protocol traffic as fraction of available link bandwidth | < 15% for steady-state operation |

### Verification Criteria

- All 4 AT process types (Identity, Reputation, Negotiation, Network) initialize successfully across simulated network
- Identity admission completes within 60s for 20 nodes
- Reputation scores converge under non-adversarial conditions
- Metrics output as JSON for downstream analysis

### Steps

- [x] Create `scenarios/appalachian_compose.py` docker-compose generator for Appalachian scenario
- [x] Extend Router to read terrain-aware `SignalMatrix` from Phase 1
- [x] Create test harness script (`test-simulation.sh`) for orchestrated launch
- [x] Implement `metrics/collector.py` with Identity convergence time metric
- [x] Add Reputation convergence metric to collector
- [x] Add Negotiation round-trip time metric to collector
- [x] Add Network bandwidth overhead metric to collector
- [x] Run 8-node hilltop-only scenario end-to-end
- [x] Run 20-node (hilltop + valley) scenario end-to-end
- [x] Document baseline performance numbers in JSON output

---

## Phase 3: Adversarial Testing / Red Team

**Objective:** Integrate MITRE CALDERA [2] for automated attack scenarios against AT over the simulated mesh. Build an automated attack → measure → report cycle.

### Background

The [eval.md](../eval.md) recommendation is CALDERA for adversarial testing: it deploys as a Docker container, uses lightweight "sandcat" agents, and defines attack behavior in YAML adversary profiles. The AT framework's security claims — Sybil resistance via voting, Byzantine fault tolerance via Paxos consensus, NaCl-encrypted channels — need systematic adversarial validation.

### Integration Points

1. **CALDERA container** — deploys on the same Docker bridge network as AT nodes (per [eval.md](../eval.md) recommendation). CALDERA server coordinates attack operations.

2. **Sandcat agents** — deployed as sidecars on selected AT containers. These are CALDERA's lightweight agents that execute attack abilities on host containers.

3. **`Router` extension** — `AttackRouter(Router)` subclass that injects network partitions on a configurable schedule via iptables, simulating link failures and adversarial network control.

4. **Custom adversary profiles** — YAML files defining AT-specific attack sequences using CALDERA's ability/adversary format.

### Attack Scenarios

| # | Attack | Target | Expected AT Response |
|---|--------|--------|---------------------|
| 1 | **Sybil attack** | Spawn multiple fake identities against admission voting | Voting rejects; identity count stays bounded |
| 2 | **Byzantine node** | Provide inconsistent reputation scores to different peers | Paxos consensus detects inconsistency; byzantine node reputation drops |
| 3 | **Reputation gaming** | Cooperate → build reputation → defect | Tit-for-tat recovery; reputation penalty exceeds gained trust |
| 4 | **Network partition** | iptables-based network split via AttackRouter | Store-and-forward recovery after partition heals; state re-converges |
| 5 | **MITM attempt** | Intercept encrypted group channel | Fails — validates NaCl/libsodium encryption correctness |

### New Files (to create)

| File | Purpose |
|------|---------|
| `simulator/redteam/__init__.py` | Red team package init |
| `simulator/redteam/attack_router.py` | `AttackRouter(Router)` with partition scheduling via iptables |
| `simulator/redteam/byzantine_node.py` | Modified AT process that deliberately provides inconsistent reputation data |
| `simulator/redteam/adversary_profiles/sybil.yaml` | CALDERA adversary profile for Sybil attack |
| `simulator/redteam/adversary_profiles/byzantine.yaml` | CALDERA adversary profile for Byzantine behavior |
| `simulator/redteam/adversary_profiles/reputation_gaming.yaml` | CALDERA adversary profile for reputation gaming |
| `simulator/redteam/harness.py` | Orchestrator: launch → attack → measure → report |

### Verification Criteria

- Each attack scenario runs to completion without simulator crashes
- AT detects and recovers from each attack:
  - Sybil identities rejected by voting
  - Byzantine node reputation drops below trust threshold
  - Reputation gamer penalized after defection
  - Partition recovery succeeds (state re-converges)
  - MITM attempt fails (NaCl encryption holds)
- Automated report generated in JSON + human-readable format

### Steps

- [x] Deploy CALDERA server container alongside AT network in docker-compose
- [x] Create CALDERA sandcat agent sidecar configuration for AT containers
- [x] Define Sybil attack adversary profile (`adversary_profiles/sybil.yaml`)
- [x] Define Byzantine node adversary profile (`adversary_profiles/byzantine.yaml`)
- [x] Define reputation gaming adversary profile (`adversary_profiles/reputation_gaming.yaml`)
- [x] Implement `AttackRouter(Router)` with configurable partition scheduling
- [x] Implement `byzantine_node.py` (modified AT `ReputationProcess` with deliberate inconsistency)
- [x] Implement `sybil_attack.py`, `reputation_gaming.py`, `mitm_attack.py` attack scenarios
- [x] Create automated harness (`harness.py`): launch → attack → collect metrics → report
- [x] Generate consolidated red-team report (JSON + human-readable markdown)
- [ ] Run each attack scenario, collect pass/fail and recovery metrics

---

## Phase 4: Polite Protocol Hooks

**Objective:** Define the AT ↔ Polite interface. Create test scaffolding for group protocol evaluation. Design the automated test → implement → test loop that will drive Polite protocol development.

**Dependency note:** Phase 4 can begin in parallel with Phase 3 — both depend on Phase 2 but not on each other.

### Background

Polite (`lib/polite/`) is at the conceptual stage: a Rust project with a Python `initial.py` containing TensorFlow scaffolding for a Transformer encoder, but no actual group protocol implementation. The goal of Phase 4 is not to implement Polite protocols, but to build the interface and evaluation framework so that protocol development can proceed with automated feedback.

### Key Design

**`PoliteObserver`** — an AT `Process` subclass that subscribes to Reputation + Negotiation message streams. It receives:
- Reputation score updates (who trusts whom, how much)
- Negotiation outcomes (successful deals, failed deals, terms)
- Identity admission events (new peers joining, peers departing)
- Group channel metadata (active channels, membership)

**`PolitePolicy`** — a data class representing group behavioral rules. Examples:
- Minimum reputation threshold for negotiation eligibility (e.g., 0.3)
- Defection cool-down period before re-admission to negotiations
- Maximum group size for consensus operations
- Bandwidth allocation priority by reputation rank

Initially hand-crafted; the ML pipeline (Phase 4's final design step) replaces this with learned policies.

**`PoliteEvaluator`** — a test harness that:
1. Accepts a `PolitePolicy`
2. Runs the AT simulation with the policy active (via `PoliteObserver`)
3. Collects outcome metrics: cooperation rate, task throughput, attack resilience
4. Scores the policy against a fitness function

### The Automated Loop (Design Target)

```
┌──────────────────────────────────────────────────────┐
│  1. Define PolitePolicy (hand-crafted → ML-generated)│
│  2. Run AT simulation with policy active              │
│  3. Collect outcome metrics                           │
│  4. Score policy via fitness function                 │
│  5. (Future) Feed scores to ML → generate new policy  │
│  └──→ repeat                                          │
└──────────────────────────────────────────────────────┘
```

This is where `lib/polite/src/initial.py`'s TensorFlow Transformer encoder becomes relevant — it is the placeholder for the policy-generation model that will close the loop in step 5.

### New Files (to create)

| File | Purpose |
|------|---------|
| `lib/polite/python/polite_interface.py` | Abstract interface definitions: events, policy structure, evaluator protocol |
| `lib/polite/python/polite_observer.py` | AT `Process` plugin subscribing to reputation/negotiation streams |
| `lib/polite/python/polite_policy.py` | `PolitePolicy` data class + static example policies |
| `simulator/polite/__init__.py` | Polite evaluation package within simulator |
| `simulator/polite/evaluator.py` | `PoliteEvaluator` test harness |

### Verification Criteria

- `PoliteObserver` successfully receives events during AT simulation run
- Hand-crafted policy measurably changes simulation outcomes vs null policy (no Polite)
- `PoliteEvaluator` produces comparable, reproducible scores across runs
- ML integration point clearly documented for future automated policy generation

### Steps

- [ ] Define `PoliteObserver` abstract interface (events it receives from AT) in `polite_interface.py`
- [ ] Define `PolitePolicy` data class with example static policies in `polite_policy.py`
- [ ] Implement `PoliteObserver` as AT `Process` plugin in `polite_observer.py`
- [ ] Register `PoliteObserver` in `subsystems.cfg.json` as optional process
- [ ] Create `PoliteEvaluator` test harness in `simulator/polite/evaluator.py`
- [ ] Run baseline simulation with null policy (no Polite active)
- [ ] Run simulation with simple hand-crafted policy (e.g., "min reputation 0.3 to negotiate")
- [ ] Compare metrics between null and hand-crafted policy runs
- [ ] Document ML integration point for automated policy generation (spec for Transformer in `initial.py`)
- [ ] Design the automated policy-generation loop (specification only — ML implementation is future work)

---

## Phase 5: Scale to Space

**Objective:** Adapt the simulation from terrestrial mesh to asteroid-belt inter-habitat mesh. Test AT protocols under extreme latency and intermittent connectivity — the ultimate scale target for MUUDD.

### Background

Asteroid belt distances range from 2–4 AU between habitats, producing one-way light times of **16–64 minutes**. Orbital mechanics create intermittent connectivity: sun occultation, asteroid shadowing, and varying distances as habitats follow Keplerian ellipses. Solar flux at 2.7 AU (mid-belt) is approximately **14% of Earth-surface levels**, constraining both power budgets and communication energy.

These conditions break assumptions embedded in most networking protocols: that RTT is sub-second, that links are persistent, and that power is abundant. AT's protocol timeouts, consensus algorithms, and reputation convergence rates all need evaluation under these constraints.

### Key Physics

| Parameter | Terrestrial (Braxton County) | Asteroid Belt |
|-----------|------------------------------|---------------|
| Inter-node distance | 1–30 km | 2–4 AU (300M–600M km) |
| One-way light time | < 0.1 ms | 16–64 minutes |
| Link persistence | Continuous (stationary nodes) | Intermittent (orbital geometry) |
| Available bandwidth | 10 Kbps – 10 Gbps | 1 Kbps – 10 Mbps (distance-dependent) |
| Power | Grid/solar, abundant | Solar at 14% Earth flux, severely constrained |

### Integration Points

1. **`PeerMovement` path system** — use `EllipseData`/`EllipsePath` (already in `peer/path.py` with de La Hire construction) for solar-orbit trajectories. Each habitat follows a Keplerian ellipse around the sun.

2. **`Router`** — extend with `tc netem delay` for light-time-proportional latency injection. Current Router uses iptables for reachability and tc CBQ for bandwidth; add tc netem for delay.

3. **`NetInterface`** — add new interface types:
   - `LASER_COMMS` — high bandwidth (up to 10 Mbps), requires line-of-sight, fails during occultation
   - `DEEP_SPACE` — low bandwidth (1–100 Kbps), high-gain parabolic, tolerates longer ranges but not occultation

4. **AT protocol timeouts** — must be identified and made configurable for minutes-to-hours RTT. Identity admission, reputation voting rounds, and negotiation handshakes all have implicit or explicit timeouts that assume sub-second RTT.

5. **DTN (Delay-Tolerant Networking)** — an adapter wrapping AT's `Network` process to implement store-and-forward bundle protocol semantics. When a peer is unreachable, messages are queued and forwarded when connectivity resumes. This is a design spec initially, not a full implementation.

### New Files (to create)

| File | Purpose |
|------|---------|
| `simulator/scenarios/asteroid_belt.py` | Orbital scenario: 5–10 habitats on Keplerian solar orbits with realistic belt parameters |
| `simulator/radio/space_link.py` | Laser comms LOS check, RF deep-space link budget, light-delay calculation |
| `simulator/radio/delay.py` | Router extension adding `tc netem delay` for light-time injection |

### Verification Criteria

- AT protocols complete (slowly) despite multi-minute RTT
- Partitions from orbital geometry (sun occultation) handled by store-and-forward
- Protocol timeouts documented and made configurable
- Light-delay values consistent with orbital mechanics (distance / c)

### Steps

- [ ] Implement Keplerian orbit model for habitat positions using `EllipseData`/`EllipsePath`
- [ ] Add light-delay calculation to `compute_step` (distance / c per pair)
- [ ] Add `LASER_COMMS` and `DEEP_SPACE` to `NetInterface` enum in `radio/iface.py`
- [ ] Implement `radio/space_link.py` with LOS check and deep-space link budget
- [ ] Extend Router with `tc netem delay` injection in `radio/delay.py`
- [ ] Create `scenarios/asteroid_belt.py` with 5–10 habitats on solar orbits
- [ ] Test AT identity admission under 30-minute RTT
- [ ] Test reputation convergence under intermittent connectivity (sun occultation periods)
- [ ] Identify and catalogue all AT protocol timeouts needing adjustment for space-scale RTT
- [ ] Design DTN bundle protocol adapter for AT Network process (specification only initially)
- [ ] Run full scenario: orbital motion + AT protocols + adversarial nodes

---

## Worked Example: Sutton–Burnsville Link Validation

To illustrate the Phase 1 integration, here is how terrain-aware simulation changes the outcome for a specific link from [network.md](network.md).

**Current model (inverse-square only):**
```
Distance: 18.5 km (Sutton Knob → Burnsville hilltop)
Free-space path loss at 5.8 GHz: 20*log10(18500) + 20*log10(5800e6) + 32.44 = 131.0 dB
With YAGI antennas (12 dBi each): effective loss = 131.0 - 24.0 = 107.0 dB
Link margin with 30 dBm TX: 30 - 107.0 - (-90 receiver sensitivity) = +13 dB → viable
```

**Terrain-aware model (SPLAT! ITM):**
```
SPLAT! computes additional path loss from:
  - Two intervening ridges (knife-edge diffraction): +8–15 dB
  - Fresnel zone obstruction at 5.8 GHz (F1 radius 9.0 m at midpoint): +3–6 dB
  - Foliage on ridge crests (ITM terrain roughness): +2–8 dB
  Total additional loss: +13–29 dB
  Effective link margin: +13 - (13 to 29) = -16 to 0 dB
```

This demonstrates why terrain-aware simulation is essential: a link that appears viable in free-space may be marginal or infeasible with real terrain. The SPLAT! output determines whether this link needs higher-gain antennas, a relay node, or a frequency change.

In the simulator, this translates to a refactored `can_reach()`. **Implementation note:** the current `self.signal` is a linear-scale value used in the inverse-square calculation. The terrain-aware path requires converting to dB scale for compatibility with SPLAT! output. The existing fallback path must preserve the original linear calculation for backward compatibility.

```python
# Phase 1 change to can_reach() — pseudocode showing the interface change.
# Actual implementation must reconcile the existing linear-scale signal
# with SPLAT!'s dB-scale path loss (convert self.signal to dBm first).
def can_reach(self, other: PeerConnection, terrain_loss_db: float | None = None) -> bool:
    if terrain_loss_db is not None:
        # Terrain-aware: all values in dB scale
        signal_dbm = 10 * math.log10(abs(self.signal)) if self.signal != 0 else -200
        effective_signal = signal_dbm - terrain_loss_db + self.antenna.gain + other.antenna.gain
    else:
        # Fall back to inverse-square model (backward compatibility, linear scale)
        dist = self.position.distance_to(other.position)
        effective_signal = (1.0 / dist**2) * self.signal + self.antenna.gain
    min_strength = -154 * 10 * math.log10(self.iface.rate)
    return effective_signal < min_strength
```

## Worked Example: Asteroid Belt Light Delay

To illustrate Phase 5's delay injection, consider two habitats at opposition across the belt.

**Scenario:** Habitat A at 2.2 AU, Habitat B at 3.1 AU, angular separation 47°.

```
Chord distance = sqrt(2.2² + 3.1² - 2·2.2·3.1·cos(47°)) = 2.31 AU
Light time = 2.31 AU × 499.0 s/AU = 1152.7 s ≈ 19.2 minutes one-way
Round-trip time = 38.4 minutes
```

**Router delay injection:**
```bash
# tc netem adds per-packet delay on the Docker veth interface
tc qdisc add dev veth_habitat_b root netem delay 1152700ms
```

**AT protocol impact:**
- Identity admission (4 round trips for challenge-response + voting): **~2.6 hours**
- Reputation update propagation to 5 peers: **~1.6 hours** (pipelined)
- Negotiation (3 round trips): **~1.9 hours**

This makes clear that AT protocol timeouts (currently assuming seconds) must be parameterized, and that DTN store-and-forward is not optional at this scale.

---

## References

1. Magliacane, J. "SPLAT! — An RF Signal Propagation, Loss, and Terrain Analysis Tool." [https://www.qsl.net/kd2bd/splat.html](https://www.qsl.net/kd2bd/splat.html)

2. MITRE. "CALDERA — Automated Adversary Emulation Platform." [https://caldera.mitre.org/](https://caldera.mitre.org/)

3. National Telecommunications and Information Administration. "Irregular Terrain Model (ITM)." [https://its.ntia.gov/software/itm](https://its.ntia.gov/software/itm)

4. Braxton County, WV. "Digital Equity Plan." Referenced in [network.md](network.md).

5. RFC 9171. "Bundle Protocol Version 7." IETF (2022). DTN reference for Phase 5 adapter design.

6. AutonomousTrust documentation. See `lib/autonomous_trust/` README and source.
