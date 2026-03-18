# Simulating a Community Mesh Network: Open-Source Tools and Integration

A survey of open-source software for simulating the decentralized mesh network described in [network.md](network.md) — covering RF propagation modeling, network protocol simulation, terrain data, and practical integration strategies for building a complete simulation pipeline.

## Overview

No single open-source tool can simulate a multi-technology community mesh network end-to-end. The problem spans three distinct layers that each require specialized software:

1. **Terrain-aware RF propagation** — computing realistic path loss between node locations using real elevation data, accounting for knife-edge diffraction, foliage attenuation, and Fresnel zone clearance
2. **Network protocol simulation** — modeling mesh routing, multi-radio coordination, throughput, latency, and packet delivery across the simulated topology
3. **System-level dynamics** — solar energy cycles, node availability, traffic demand patterns, failure resilience, and long-term cost modeling

The practical approach is a **pipeline architecture**: pre-compute pairwise RF path loss externally from real terrain data, feed the resulting link-quality matrix into a network simulator, and use system-level modeling for capacity planning and resilience analysis.

This document surveys the available tools in each category, compares their fitness for the Appalachian mesh network use case, and recommends integration strategies ranging from lightweight (a few Python scripts with SPLAT!) to comprehensive (NS-3 with terrain-aware propagation and multi-technology radio models).

## RF Propagation and Terrain Modeling

These tools compute signal strength, path loss, and coverage areas using real elevation data — the foundation for any realistic simulation.

### SPLAT! (Signal Propagation, Loss, And Terrain)

**SPLAT!** is the most mature open-source RF path analysis tool for terrestrial propagation in the 20 MHz to 20 GHz range [1]. Written in C, licensed under GPLv2, it has been a standard tool in the amateur radio and WISP planning communities for over two decades.

**Capabilities:**
- Uses the **Longley-Rice Irregular Terrain Model (ITM)** and **ITWOM v3.0** — the same propagation models used by the FCC for regulatory analysis [2]
- Ingests **SRTM elevation data** (converted to native SDF format via the included `srtm2sdf` utility)
- Generates color-coded coverage maps, terrain height profiles, path loss graphs, and Fresnel zone analysis
- Handles knife-edge diffraction implicitly through the ITM/ITWOM models
- Fully command-line driven — scriptable for batch analysis of many node pairs and frequency bands

**Limitations:**
- Single-threaded execution; large coverage maps are slow
- No built-in foliage/land-cover database — terrain roughness parameters approximate vegetation effects statistically rather than from measured canopy data
- Output is image maps and text reports, not a programmatic API — integration requires parsing output files
- Linux/Unix only

**Relevance to the Appalachian mesh:** SPLAT! is the most practical starting point. Download SRTM tiles for Braxton County, define candidate node locations (hilltop relays, valley relays, gateways), and generate pairwise path loss reports at each frequency band (900 MHz, 2.4 GHz, 5.8 GHz). The resulting data directly validates or invalidates the link budget calculations in [network.md](network.md).

**Example workflow:**
```bash
# Convert SRTM tiles to SPLAT! format
srtm2sdf N38W081.hgt

# Compute path loss and terrain profile between two sites
splat -t sutton_hilltop.qth -r burnsville_hilltop.qth -f 5800 -o report.png

# Generate coverage map from a hilltop relay
splat -t powell_mtn.qth -L 12.0 -f 900 -o coverage_900mhz.png
```

### Signal Server (W3AXL Fork)

A **multi-threaded evolution** of SPLAT!'s core algorithms, originally the engine behind CloudRF.com (2012–2016), now maintained as a community fork [3]. Written in C/C++ under GPL.

**Advantages over SPLAT!:**
- Multi-threaded — significantly faster for large coverage maps
- Same Longley-Rice ITM propagation model
- Reads SRTM HGT tiles directly (30 m or 90 m resolution)

**Disadvantage:** Less documentation and smaller community than SPLAT!. CloudRF discontinued open-source development in favor of their proprietary SLEIPNIR engine.

### rf-signals (Rust)

A modern Rust implementation of RF planning algorithms created by a working WISP operator (iZones) [4]. Licensed under GPLv2.

**Notable features:**
- Pure Rust implementations of ITM3/Longley-Rice, HATA/COST-231, free-space path loss, and Fresnel zone calculations
- **`terrain-cooker`**: reads LiDAR LAS point clouds and converts them to an efficient binary format for propagation analysis
- **`bracket-heat`**: web-based WISP planning tool for link feasibility
- Designed specifically for real-world WISP deployment planning

**Relevance:** The LiDAR ingestion capability is valuable for Appalachian terrain where USGS 3DEP provides 1 m resolution LIDAR data — capturing tree canopy height that SRTM (30 m, bare-earth radar) misses entirely. This enables more accurate foliage attenuation modeling than any ITM-only tool.

### ITM / Longley-Rice (NTIA Reference Implementation)

The **Irregular Terrain Model** is the foundational propagation algorithm used by SPLAT!, Signal Server, rf-signals, and Radio Mobile [2]. The NTIA (National Telecommunications and Information Administration) provides the reference implementation in Fortran, with C/C++ ports available. It is public domain (US government work) and covers 20 MHz to 20 GHz.

All of the tools above are essentially wrappers around ITM/ITWOM that add terrain data ingestion, visualization, and batch processing. Understanding ITM's parameters — terrain roughness, climate zone, ground conductivity/permittivity, antenna heights, frequency — is essential for accurate simulation regardless of which wrapper tool is used.

### Tool Comparison: RF Propagation

| Tool | Language | License | Multi-thread | LiDAR Support | Scriptable | Maturity |
|------|----------|---------|-------------|---------------|------------|----------|
| SPLAT! | C | GPLv2 | No | No (SRTM only) | Yes (CLI) | High |
| Signal Server | C/C++ | GPL | Yes | No (SRTM only) | Yes (CLI) | Medium |
| rf-signals | Rust | GPLv2 | Yes | Yes (LAS) | Yes (library) | Low–Medium |
| ITM reference | Fortran/C | Public domain | N/A | N/A | Yes (library) | High |

## Network Simulators

These tools model network behavior — routing protocols, packet forwarding, throughput, latency, and failure recovery — on top of a defined topology.

### NS-3 (Network Simulator 3)

**NS-3** is the standard open-source discrete-event network simulator for research and education [5]. Written in C++ with Python bindings, licensed under GPLv2. Actively maintained (v3.47 released February 2026).

**Capabilities relevant to mesh simulation:**
- **802.11s mesh module** with HWMP (Hybrid Wireless Mesh Protocol) — reactive and proactive routing modes, multi-interface mesh points
- **Multi-radio/multi-technology nodes** — combine NR (5G-LENA module), WiFi, and custom radio models in a single simulation
- **LoRaWAN module** (signetlabdei/lorawan [6]) — LoRa PHY with collision modeling, gateway simulation, class A/C devices
- **5G-LENA NR module** [7] — for CBRS 3.5 GHz modeling with shared spectrum
- **Propagation model chaining** — compose Friis, log-distance, Nakagami fading, and custom models
- **Energy framework** — model solar harvesting, battery state, and energy-dependent node behavior

**Integration with terrain data:** NS-3 does not natively ingest SRTM elevation data. The recommended approach is to pre-compute a **pairwise path loss matrix** using SPLAT! or rf-signals, then implement a custom `PropagationLossModel` in NS-3 that returns the pre-computed loss for each transmitter-receiver pair. This is straightforward:

```cpp
class TerrainAwareLossModel : public PropagationLossModel {
    // Load path-loss matrix from CSV (output of SPLAT! batch run)
    // Override DoCalcRxPower() to look up loss by node-pair ID
};
```

**Limitations:**
- Steep learning curve — simulations are coded in C++ or Python, not configured via GUI
- The 802.11s HWMP implementation has known issues with path maintenance (active routes can time out, causing packet drops under some topologies)
- No built-in TVWS radio model — would require a custom PHY/MAC implementation or approximation via parameterized WiFi model
- Large simulations (thousands of nodes) require significant RAM and CPU time

**Best for:** Detailed protocol-level analysis — measuring throughput, latency, jitter, and packet loss under realistic traffic loads and comparing mesh routing strategies (HWMP vs OLSR vs B.A.T.M.A.N.).

### OMNeT++ with INET Framework

**OMNeT++** is a modular discrete-event simulation framework; **INET** provides the protocol models [8]. Written in C++ with NED (Network Description Language) for topology specification. OMNeT++ is free for academic/non-commercial use (Academic Public License); INET is LGPL.

**Capabilities:**
- Scalable physical layer from unit-disc radio to detailed propagation with fading models
- **FLoRa framework** [9] — LoRa simulation within INET, including collision and capture effects, ADR, multi-gateway, and energy consumption
- MANET routing protocol support via `AdhocHost` module
- 3D visualization of terrain, transmission ranges, and connectivity
- Optional detailed signal representation in time/frequency domain

**Limitations:**
- Academic license restricts commercial use (commercial license: OMNEST)
- More complex build system than NS-3
- No built-in TVWS or CBRS models
- Terrain-aware propagation requires the same external pre-computation approach as NS-3

**Best for:** LoRa-specific simulation (FLoRa is more mature than the NS-3 LoRaWAN module) and scenarios where detailed signal-level analysis is needed.

### CORE + EMANE (Emulation)

**CORE** (Common Open Research Emulator) creates network topologies using Linux network namespaces; **EMANE** (Extendable Mobile Ad-hoc Network Emulator) adds realistic wireless physical-layer emulation [10]. CORE is Python (BSD license); EMANE is C++ (BSD license). Both are actively maintained; CORE originated at the US Naval Research Laboratory.

**Capabilities:**
- **Runs real software** — actual routing daemons (Babel, batman-adv, OLSR), real applications, real IP stacks in lightweight containers
- EMANE provides physical-layer accounting: signal propagation, antenna profiles, interference
- Four built-in radio models: RF Pipe, IEEE 802.11abg, TDMA, Bent Pipe
- Pluggable MAC/PHY architecture — custom radio models can be written for LoRa, TVWS, CBRS
- MANET routing via **OSPF v3 MDR** (Mobile Ad-hoc extension) built-in
- GUI for topology design and real-time visualization

**Key advantage:** Because CORE runs real routing daemons and applications (not simulated models), simulation results directly predict deployed-network behavior. You can test the actual batman-adv or Babel firmware that would run on MikroTik/Ubiquiti hardware, rather than a simplified model of it.

**Limitations:**
- EMANE's built-in radio models do not cover LoRa, TVWS, or CBRS — custom plugins are needed
- Resource-intensive: each emulated node runs a real Linux network stack
- Documentation is sparse for advanced EMANE plugin development

**Best for:** Testing real mesh routing software and applications in a realistic wireless environment. The closest thing to a "digital twin" of the deployed network without actual hardware.

### Mininet-WiFi

**Mininet-WiFi** extends the Mininet SDN emulator with WiFi station and access point virtualization using the Linux `mac80211_hwsim` driver [11]. Python, permissive license, actively maintained.

**Capabilities:**
- WiFi mesh and ad-hoc mode via `wmediumd` wireless medium simulation
- Position-based association/disassociation
- SDN (OpenFlow) integration for programmable network control
- Real routing protocols running in network namespaces

**Limitations:** WiFi-only — no LoRa, TVWS, or CBRS models. `wmediumd` provides basic wireless medium simulation but not terrain-aware propagation. Better suited for WiFi-specific mesh experiments than the multi-technology network described in [network.md](network.md).

### Mesh-Specific Simulators

Two lightweight tools focus specifically on mesh network routing:

**MeshNetSimulator** [12] (Rust) — interactive web-based graph visualization for sketching topologies and testing routing strategies. Abstract model without RF physics; useful for rapid routing algorithm prototyping.

**meshnet-lab** [13] (Python) — emulates large mesh topologies using Linux namespaces with real routing protocol binaries (Babel, B.A.T.M.A.N.-adv, OLSR1/2, BMX6/7, Yggdrasil, CJDNS). Includes automated benchmarking with plot generation. No RF modeling — link connectivity is defined topologically. Excellent for **comparing real mesh routing implementations** at scale.

### Tool Comparison: Network Simulation

| Tool | Type | Real Software | Multi-Radio | RF Modeling | LoRa | Ease of Use | Scale |
|------|------|--------------|-------------|-------------|------|-------------|-------|
| NS-3 | Simulation | No | Yes | Pluggable | Module | Low | 1000s nodes |
| OMNeT++/INET | Simulation | No | Yes | Pluggable | FLoRa | Low | 1000s nodes |
| CORE+EMANE | Emulation | Yes | Yes (plugins) | EMANE | Plugin needed | Medium | 100s nodes |
| Mininet-WiFi | Emulation | Yes | WiFi only | wmediumd | No | Medium | 100s nodes |
| meshnet-lab | Emulation | Yes | No | No | No | High | 1000s nodes |
| MeshNetSimulator | Abstract | No | No | No | No | High | 1000s nodes |

## GIS and Terrain Data

### Elevation Data Sources

Realistic RF simulation requires real elevation data for the target deployment area.

**SRTM (Shuttle Radar Topography Mission):** Global DEM at 1 arcsecond (~30 m) and 3 arcsecond (~90 m) resolution. Freely available from OpenTopography, USGS EarthExplorer, or NASA JPL. The standard input format for SPLAT! and Signal Server. For Braxton County, WV (approximately 38.4°N–39.0°N, 80.4°W–80.9°W), tiles N38W081 and N38W080 cover the area.

**USGS 3DEP (3D Elevation Program):** US-specific elevation at 1 m, 10 m, and 30 m resolution [14]. The 1 m data is derived from airborne LiDAR and is available for much of Appalachia. Critical advantage: **LiDAR captures tree canopy height** (first-return elevation) and bare-earth terrain (last-return), enabling direct measurement of foliage obstruction rather than statistical estimation. Accessible via the `py3dep` Python library or OpenTopography REST API.

**OpenTopography:** Unified access portal for both SRTM and 3DEP data. Provides Cloud Optimized GeoTIFF format and a REST API with on-the-fly mosaic subsetting [15].

### GIS Platforms

**GRASS GIS** [16] with the **RaPlaT** (Radio Planning Tool) add-on provides RF coverage calculation directly within a full GIS environment. RaPlaT supports GSM/UMTS/WiFi in the 400 MHz to 2.4 GHz range, uses DEM and land-cover data for propagation prediction, and supports calibration against field measurements. It includes a parallel computation module for batch RF analysis. The full GRASS GIS ecosystem provides viewshed analysis, terrain profiling, and hydrological modeling — useful for siting solar-powered nodes near ridgetop access roads.

**QGIS** [17] offers a broader plugin ecosystem and easier learning curve than GRASS. Its viewshed analysis tools and native support for SRTM/GeoTIFF data make it useful for visual site planning and coverage map overlay. GRASS tools are accessible from the QGIS Processing toolbox, so GRASS-RaPlaT can be used from within QGIS.

## System-Level and Agent-Based Modeling

For higher-level questions — solar energy adequacy, network resilience under node failures, traffic demand patterns, long-term cost projections — general-purpose simulation frameworks offer more flexibility than network simulators.

### SimPy

**SimPy** is a process-based discrete-event simulation framework for Python [18]. MIT license, actively maintained (v4.1.x).

**Capabilities:**
- Shared resources with capacity constraints (model channel contention, gateway bandwidth limits)
- Real-time and as-fast-as-possible execution modes
- Monitoring and statistics collection

**Use case:** Model each mesh node as a SimPy process with a solar charging/discharging cycle, traffic generation based on time-of-day demand patterns, and failure/recovery events. Shared resources represent backbone link capacity and gateway internet bandwidth. This produces utilization curves, queue depths, and availability metrics across seasons and weather scenarios.

The **ns.py** library [19] extends SimPy with network-specific primitives (packet generators, links, switches, schedulers, traffic shapers, monitors), providing a lighter-weight alternative to NS-3 for scenarios where full protocol simulation is unnecessary.

### Mesa

**Mesa** is a general-purpose agent-based modeling framework for Python [20]. Apache 2.0 license, actively maintained (Mesa 3 published in JOSS 2025).

**Capabilities:**
- Agents interact in configurable spaces (grid, continuous, or network graph)
- Built-in data collection and browser-based interactive visualization
- Network topology support — agents on a graph can model mesh nodes

**Use case:** Model the community deployment process itself — household adoption decisions based on neighbor connectivity, cooperative resource allocation, maintenance volunteer scheduling, and network growth dynamics. Mesa's visualization makes this accessible for non-technical stakeholders (community organizers, grant reviewers).

## Integration Strategies

### Strategy 1: Lightweight Planning Pipeline (Recommended Starting Point)

**Tools:** SPLAT! + Python scripts + QGIS
**Effort:** Days to a working prototype
**What it answers:** Can proposed links close? What does the coverage map look like? Where should nodes be placed?

```
SRTM tiles → SPLAT! (batch path loss) → Python (parse, analyze) → QGIS (visualize)
```

1. Download SRTM 30 m tiles for Braxton County
2. Define candidate node locations (lat/lon/height) in a CSV file
3. Run SPLAT! in batch mode for all node pairs at each frequency band (900 MHz, 2.4 GHz, 5.8 GHz)
4. Parse SPLAT! output with Python to build a link-quality matrix
5. Apply link budget calculations from [network.md](network.md) to determine which links are viable
6. Visualize coverage and viable links in QGIS overlaid on terrain and road maps

This pipeline validates the network design in [network.md](network.md) against real terrain data with minimal software development. A Python script of 200–400 lines orchestrates SPLAT! runs and parses results.

### Strategy 2: Full Network Simulation

**Tools:** SPLAT! + NS-3 (or CORE+EMANE)
**Effort:** Weeks to months
**What it answers:** What throughput and latency do households experience? How does the network perform under load? Which routing protocol works best?

```
SRTM tiles → SPLAT! (path loss matrix) → NS-3 custom PropagationLossModel → simulation
```

1. Pre-compute the full path loss matrix as in Strategy 1
2. Implement a custom NS-3 `PropagationLossModel` that returns pre-computed terrain-aware loss for each node pair
3. Configure NS-3 nodes with multiple radio interfaces matching the tier architecture (5 GHz PtP backhaul, TVWS/WiFi access, LoRa monitoring)
4. Apply realistic traffic models (household demand profiles)
5. Run simulations varying: number of nodes, routing protocol (HWMP, OLSR, Babel), failure scenarios, traffic levels

**Alternative with CORE+EMANE:** Replace NS-3 with CORE for emulation using real routing software. EMANE's RF Pipe model can be parameterized with the SPLAT!-derived path loss values. This tests actual batman-adv or Babel firmware behavior rather than a simulation model of it.

### Strategy 3: Comprehensive Digital Twin

**Tools:** rf-signals (with LiDAR) + NS-3 + SimPy + QGIS
**Effort:** Months
**What it answers:** Full system behavior including RF, network protocols, energy systems, and seasonal variation.

```
LiDAR (3DEP) → rf-signals (canopy-aware path loss)
                          ↓
                    NS-3 (network simulation)
                          ↓
                    SimPy (energy/availability model) ←→ Mesa (community adoption model)
                          ↓
                    QGIS (visualization, reporting)
```

1. Use 1 m LiDAR data from USGS 3DEP for canopy-aware propagation via rf-signals
2. Model seasonal foliage variation (leaf-on/leaf-off path loss profiles)
3. Full NS-3 multi-technology simulation with energy-aware node behavior
4. SimPy models solar charging cycles and battery state-of-charge through winter weather scenarios
5. Mesa models community adoption dynamics and network growth
6. QGIS integrates all outputs for planning presentations

This is a research-grade simulation suitable for grant proposals, academic publication, or detailed engineering design. It requires substantial development effort but produces the most accurate predictions.

### Strategy Comparison

| Strategy | Effort | RF Accuracy | Network Accuracy | Energy Model | Custom Code |
|----------|--------|-------------|------------------|-------------|-------------|
| 1: Planning pipeline | Days | High (ITM) | None (link budget only) | None | ~300 lines Python |
| 2: Full network sim | Weeks–months | High (ITM) | High (NS-3/CORE) | Basic | ~1,000–3,000 lines C++/Python |
| 3: Digital twin | Months | Very high (LiDAR) | High | Full seasonal | ~5,000+ lines multi-language |

## Related Open-Source Deployment Tools

While not simulators, these projects provide the actual software stack for deployed community mesh networks and can inform simulation parameters:

**OpenWISP** [21] — network management platform for OpenWrt-based mesh networks. Automates configuration, monitoring, RADIUS authentication, and firmware upgrades. Includes a topology collector/visualizer for mesh routing protocols.

**LibreMesh** [22] — modular OpenWrt-based firmware framework used by community networks worldwide. Provides the routing, address allocation, and network management firmware that would run on deployed MikroTik/Ubiquiti hardware.

**ISEMS (Independent Solar Energy Mesh System)** [23] — monitoring and maintenance software for energy-autonomous solar mesh routers. Directly relevant to the solar-powered hilltop relay architecture in [network.md](network.md).

**Meshtastic** [24] — open-source LoRa mesh communication platform on ESP32/nRF52840 hardware. The reference implementation for the LoRa monitoring layer described in [network.md](network.md).

**disaster.radio** [25] — LoRa mesh network platform with solar-powered nodes and ESP32 hardware. Includes a JavaScript mesh simulator useful for visualizing LoRa mesh behavior.

## Recommendations

For the Appalachian mesh network described in [network.md](network.md), the recommended path is:

1. **Start with Strategy 1** — validate the network design against real Braxton County terrain using SPLAT! and SRTM data. This can be done in a few days and immediately reveals whether proposed hilltop-to-hilltop and valley-to-ridgetop links are viable. Write the Python orchestration and analysis scripts as a reusable tool.

2. **Progress to Strategy 2 using CORE+EMANE** if protocol-level questions need answering (which routing protocol performs best? how does the network handle backbone link failure?). CORE's use of real routing software means results directly predict deployed behavior.

3. **Add SimPy energy modeling** to validate the solar sizing calculations in [network.md](network.md) against historical weather data for Braxton County — particularly winter cloud cover, ice storm duration, and shortest-day solar resource.

4. **Reserve Strategy 3** for grant-funded research phases where the additional accuracy justifies the development effort.

In all strategies, **SPLAT! with SRTM data** is the foundation. It is mature, well-documented, uses the same ITM propagation model that the FCC relies on, and requires no custom code to produce actionable terrain-aware link analysis.

## References

1. Magliacane, J. "SPLAT! — An RF Signal Propagation, Loss, and Terrain Analysis Tool." (2024). [https://www.qsl.net/kd2bd/splat.html](https://www.qsl.net/kd2bd/splat.html)

2. National Telecommunications and Information Administration. "Irregular Terrain Model (ITM)." US Department of Commerce, NTIA/ITS. [https://its.ntia.gov/software/itm](https://its.ntia.gov/software/itm)

3. W3AXL. "Signal Server." Forked from Cloud-RF/Signal-Server. [https://github.com/W3AXL/Signal-Server](https://github.com/W3AXL/Signal-Server)

4. Brackett, H. "rf-signals: RF Planning in Rust." iZones. [https://github.com/thebracket/rf-signals](https://github.com/thebracket/rf-signals)

5. NS-3 Consortium. "NS-3 Network Simulator." (2026). [https://www.nsnam.org/](https://www.nsnam.org/)

6. Magrin, D., Centenaro, M., and Vangelista, L. "Performance Evaluation of LoRa Networks in a Smart City Scenario." *IEEE International Conference on Communications* (2017). Module: [https://github.com/signetlabdei/lorawan](https://github.com/signetlabdei/lorawan)

7. Centre Tecnologic de Telecomunicacions de Catalunya. "5G-LENA NR Module for NS-3." (2026). [https://5g-lena.cttc.es/](https://5g-lena.cttc.es/)

8. Varga, A. "OMNeT++ Discrete Event Simulation System." (2026). [https://omnetpp.org/](https://omnetpp.org/)

9. Slabicki, M., Premsankar, G., and Di Francesco, M. "Adaptive Configuration of LoRa Networks for Dense IoT Deployments." *IEEE/IFIP Network Operations and Management Symposium* (2018). FLoRa: [https://flora.aalto.fi/](https://flora.aalto.fi/)

10. Ahrenholz, J. "Comparison of CORE Network Emulation Platforms." *IEEE Military Communications Conference* (2010). CORE: [https://github.com/coreemu/core](https://github.com/coreemu/core) | EMANE: [https://github.com/adjacentlink/emane](https://github.com/adjacentlink/emane)

11. Fontes, R.R., Afzal, S., Brito, S.H.B., Santos, M.A.S., and Rothenberg, C.E. "Mininet-WiFi: Emulating Software-Defined Wireless Networks." *International Conference on Network and Service Management* (2015). [https://mininet-wifi.github.io/](https://mininet-wifi.github.io/)

12. Warning, M. "MeshNetSimulator." [https://github.com/mwarning/MeshNetSimulator](https://github.com/mwarning/MeshNetSimulator)

13. Warning, M. "meshnet-lab." [https://github.com/mwarning/meshnet-lab](https://github.com/mwarning/meshnet-lab)

14. US Geological Survey. "3D Elevation Program (3DEP)." [https://www.usgs.gov/3d-elevation-program](https://www.usgs.gov/3d-elevation-program) | Python access via py3dep: [https://pypi.org/project/py3dep/](https://pypi.org/project/py3dep/)

15. OpenTopography. "High-Resolution Topography Data and Tools." [https://portal.opentopography.org/](https://portal.opentopography.org/)

16. GRASS Development Team. "GRASS GIS." Open Source Geospatial Foundation (2026). [https://grass.osgeo.org/](https://grass.osgeo.org/) | RaPlaT add-on: Hrovat, A., et al. "GRASS-RaPlaT — Radio Planning Tool for GRASS GIS." *Elektrotehniski Vestnik* 78(1-2), 73–78 (2011).

17. QGIS Development Team. "QGIS Geographic Information System." Open Source Geospatial Foundation (2026). [https://qgis.org/](https://qgis.org/)

18. Team SimPy. "SimPy: Discrete-Event Simulation for Python." (2026). [https://simpy.readthedocs.io/](https://simpy.readthedocs.io/)

19. TL-System. "ns.py: Network Simulator in SimPy." [https://github.com/TL-System/ns.py](https://github.com/TL-System/ns.py)

20. Kazil, J., Masad, D., and Crooks, A. "Mesa 3: Agent-Based Modeling in Python." *Journal of Open Source Software* 10(97), 7668 (2025). [https://mesa.readthedocs.io/](https://mesa.readthedocs.io/)

21. OpenWISP. "Open Source Network Management System." [https://openwisp.org/](https://openwisp.org/)

22. LibreMesh. "Modular Firmware for Community Mesh Networks." [https://libremesh.org/](https://libremesh.org/)

23. Independent Solar Energy Mesh System (ISEMS). [https://isems.netlify.app/](https://isems.netlify.app/)

24. Meshtastic. "Open Source LoRa Mesh Communication Platform." [https://meshtastic.org/](https://meshtastic.org/)

25. disaster.radio. "LoRa Mesh Network for Off-Grid Communication." [https://disaster.radio/](https://disaster.radio/)
