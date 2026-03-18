# Decentralized Mesh Network for Rural Appalachian Connectivity

A technical framework for deploying community-owned mesh radio networks in mountainous, low-density terrain where traditional broadband economics fail — designed as the physical layer complement to the MUUDD software stack and AutonomousTrust protocol.

## Overview

Conventional broadband deployment is an economic impossibility in much of rural Appalachia. The Braxton County, West Virginia Digital Equity Plan [1] illustrates the problem starkly: 12,505 people spread across 516 square miles (population density of 24/sq mi), with 23.6% of households lacking broadband access and 3,465 un- or underserved locations. The estimated cost to build fiber to these locations is **\$38.2 million** for 477 route-miles — working out to **\$46,000–\$78,000 per demand point**. Per capita income is $21,948, and 34.4% of the population lives below 150% of the federal poverty line.

The revenue arithmetic is fatal. Consider a representative 13-mile fiber route serving 6 households: at \$70/month with a 30% take rate (industry average for rural), annual revenue is \$5,040. Against construction costs exceeding \$500,000, payback never arrives. Even with BEAD subsidies covering construction, the ongoing maintenance cost of fiber plant in terrain subject to ice storms, landslides, and flooding undermines sustainability.

**Community mesh networking** offers a fundamentally different economic model:

- **Capital cost per household: \$200–\$1,500** (vs \$46,000–\$78,000 for fiber)
- **User-owned equipment** eliminates subscription dependency
- **Distributed maintenance** by trained community members replaces utility-scale operations
- **Incremental deployment** — each new node extends coverage without central planning
- **Resilient topology** — no single point of failure, unlike fiber trunk lines

This document specifies the radio hardware, network topology, and deployment economics for connecting communities across the ridge-and-valley terrain of central Appalachia. It is designed to integrate with the MUUDD trust-based communications stack: AutonomousTrust provides the identity, encryption, and reputation layer; this document provides the physical transport beneath it.

## Terrain Analysis: Central Appalachian Geography

### Ridge-and-Valley Topology

Braxton County sits in the Allegheny Plateau subregion of the central Appalachian Mountains, characterized by deeply dissected terrain with narrow valleys (locally called **hollows** or "hollers") carved by tributaries of the Elk River and Little Kanawha River. Ridgeline elevations range from 1,800 to 2,800 ft (550–850 m), while valley floors sit at 800–1,200 ft (240–370 m), producing typical ridge-to-valley relief of **600–1,600 ft (180–490 m)** [2].

The terrain creates a distinctive connectivity challenge:

- **Line of sight between valleys is rare.** Two households on opposite sides of a ridge may be 1 mile apart horizontally but completely radio-shadowed by 500+ ft of intervening rock and forest.
- **Valley bottoms channel along waterways.** Populations cluster along Elk River (Sutton, Gassaway, Flatwoods), Holly River, and Birch River, forming linear settlement patterns 0.5–2 miles wide.
- **Ridgetops provide extraordinary vantage points.** A single relay at 2,400 ft elevation can achieve line-of-sight to multiple valley communities 5–15 miles distant.
- **Saddles and gaps** (low points in ridgelines) serve as natural relay sites connecting adjacent drainage basins with minimal tower height requirements.

### Foliage and Seasonal Variation

Central West Virginia is approximately 80% forested [3], predominantly mixed mesophytic hardwood (oak, hickory, maple, tulip poplar) with some evergreen (hemlock, white pine, spruce at higher elevations). This creates two distinct propagation regimes:

- **Leaf-on (May–October):** Dense canopy attenuates radio signals significantly. Hardwood foliage adds 10–20 dB of additional path loss at 2.4 GHz and 5–12 dB at 900 MHz over bare-tree conditions [4].
- **Leaf-off (November–April):** Signal propagation improves markedly, but ice loading on antennas and solar panels creates mechanical stress. Bare branches still scatter signals at higher frequencies.

Network design must accommodate the **worst-case leaf-on scenario** while benefiting from improved winter propagation as margin.

### Weather and Environmental Factors

Braxton County receives approximately 44 inches (1,120 mm) of precipitation annually, distributed relatively evenly across seasons. Key environmental challenges for radio infrastructure:

- **Ice storms** occur 2–5 times per winter, depositing 0.25–1.0 inches of radial ice on exposed structures. Tower and antenna systems must withstand 0.5-inch radial ice loading per TIA-222 [5].
- **Wind** loading: sustained 60 mph with 80 mph gusts for design purposes.
- **Lightning** exposure is elevated on ridgetop sites. All hilltop installations require proper grounding, surge suppression on all cable entries, and galvanic isolation of antenna feeds.
- **Temperature range:** -15°F to 100°F (-26°C to 38°C). Electronics enclosures require ventilation for summer heat and battery chemistry must tolerate winter cold.

## RF Propagation in Mountainous Terrain

### Fresnel Zone Clearance

Radio links require not just geometric line-of-sight but clearance of the **first Fresnel zone** — the ellipsoidal region around the direct path where reflected signals could destructively interfere. The radius of the first Fresnel zone at the midpoint of a link is:

    r₁ = 17.3 × √(d / 4f)

where r₁ is in meters, d is path length in kilometers, and f is frequency in GHz [6]. For a 10 km link:

| Frequency | First Fresnel Zone Radius (midpoint) |
|-----------|--------------------------------------|
| 900 MHz   | 28.9 m (94.8 ft)                     |
| 2.4 GHz   | 17.7 m (58.1 ft)                     |
| 5.8 GHz   | 11.4 m (37.4 ft)                     |

At 900 MHz, a 10 km link requires nearly **95 ft of clearance** above any midpoint obstruction — a critical consideration when ridges rise 500–1,600 ft above valley floors. This is precisely why hilltop relay placement is essential: links that merely graze a ridgeline suffer severe diffraction loss even without direct obstruction.

### Knife-Edge Diffraction

When a radio path is partially obstructed by a ridge that can be approximated as a sharp edge, the signal diffracts over the obstacle with calculable loss. The ITU-R P.526 method [7] models this using the dimensionless Fresnel-Kirchhoff parameter ν:

    ν = h × √(2(d₁ + d₂) / (λ × d₁ × d₂))

where h is the height of the obstruction above the direct path (m), d₁ and d₂ are distances from transmitter and receiver to the obstruction (m), and λ is wavelength (m). The diffraction loss J(ν) in dB is approximately:

    J(ν) ≈ 6.9 + 20 × log₁₀(√((ν - 0.1)² + 1) + ν - 0.1)    for ν > -0.7

For a ridge extending 50 m above the direct path at the midpoint of a 10 km link:

| Frequency | ν    | Diffraction Loss |
|-----------|------|------------------|
| 900 MHz   | 2.74 | 16.5 dB          |
| 2.4 GHz   | 4.47 | 20.9 dB          |
| 5.8 GHz   | 6.96 | 24.8 dB          |

**Lower frequencies diffract significantly better around obstacles.** A 900 MHz link loses 16.5 dB over a 50 m ridge obstruction versus 24.8 dB at 5.8 GHz — an 8.3 dB advantage that can mean the difference between a functional link and no link at all.

### Foliage Attenuation

ITU-R P.833 [4] provides models for attenuation through vegetation. For a signal passing through 100 m of dense deciduous forest canopy (leaf-on conditions):

| Frequency | Specific Attenuation | Total (100 m) |
|-----------|----------------------|---------------|
| 900 MHz   | 0.10–0.20 dB/m       | 10–20 dB      |
| 2.4 GHz   | 0.15–0.40 dB/m       | 15–40 dB      |
| 5.8 GHz   | 0.25–0.50 dB/m       | 25–50 dB      |

These figures reinforce two design principles: (1) **elevate antennas above the tree canopy** wherever possible, and (2) **use sub-GHz frequencies** for non-line-of-sight (NLOS) paths through forested terrain.

### Rain Fade

At frequencies below 10 GHz, rain fade is modest and rarely a limiting factor. At 5.8 GHz in heavy rain (50 mm/hr, a severe thunderstorm), specific attenuation is approximately 0.06 dB/km [8] — producing only 0.6 dB of loss over a 10 km path. Rain fade is not a significant design constraint for the frequency bands considered here.

### Worked Example: Sutton to Burnsville Link

Consider a backbone link from a hilltop relay above **Sutton** (38.66°N, 80.71°W, ridgetop elevation ~2,200 ft) to a relay above **Burnsville** (38.85°N, 80.66°W, ridgetop elevation ~2,400 ft), a path distance of approximately 22 km (13.7 mi).

**Path profile:** The terrain between these points crosses two intermediate ridges with saddle elevations of approximately 2,000 ft and 2,100 ft. With relay antennas at 40 ft (12 m) above ground on the terminal ridgetops, the worst-case intermediate ridge at 2,100 ft produces an obstruction approximately 15 m below the direct path — providing positive Fresnel zone clearance at all frequencies.

**Link budget at 5.8 GHz (PtP backhaul):**

- Free-space path loss (22 km, 5.8 GHz): 138.1 dB
- Tx power: 30 dBm (1 W, FCC Part 15 limit)
- Antenna gain (each end, 30 dBi dish): +60 dB total
- Cable/connector losses: -3 dB
- Foliage margin (antennas above canopy): 0 dB
- Rain fade margin: -1.3 dB
- **Received signal: -52.4 dBm**
- Receiver sensitivity (MCS9, 80 MHz channel): -65 dBm
- **Link margin: 12.6 dB** — adequate for reliable operation

At this margin, the link supports **450+ Mbps aggregate throughput** using modern 802.11ac PtP equipment, sufficient to serve as a backbone segment for hundreds of households.

## Radio Technologies for Ridge-and-Valley Terrain

### Technology Comparison

| Technology | Frequency | Range (LOS) | Range (NLOS) | Throughput | Cost/Unit | License | NLOS Rating |
|------------|-----------|-------------|--------------|------------|-----------|---------|-------------|
| LoRa/Meshtastic | 902–928 MHz | 15–25 km | 3–8 km | 0.3–50 kbps | \$35–80 | Part 15 ISM | Excellent |
| TV White Space | 470–698 MHz | 15–30 km | 5–15 km | 5–25 Mbps | \$500–2,000 | Part 15 Subpart H | Excellent |
| WiFi Mesh 2.4 GHz | 2.4 GHz | 3–8 km | 0.5–2 km | 50–300 Mbps | \$50–200 | Part 15 ISM | Moderate |
| WiFi Mesh 5 GHz | 5 GHz | 5–15 km | 0.3–1 km | 100–900 Mbps | \$80–300 | Part 15 U-NII | Poor |
| PtP Backhaul 5 GHz | 5.8 GHz | 20–50 km | N/A (LOS req.) | 450–2,000 Mbps | \$200–800 | Part 15 U-NII | None |
| CBRS 3.5 GHz | 3.55–3.7 GHz | 5–15 km | 1–5 km | 50–400 Mbps | \$300–1,500 | GAA/PAL | Moderate |
| AREDN (Amateur) | 2.4/5 GHz | 5–30 km | 0.5–2 km | 50–300 Mbps | \$50–200 | Part 97 | Poor |

### Sub-GHz 900 MHz (LoRa / Meshtastic)

**LoRa** (Long Range) modulation at 902–928 MHz ISM band offers the best non-line-of-sight penetration of any license-free technology. The chirp spread-spectrum modulation trades bandwidth for extreme receiver sensitivity (-130 to -148 dBm), enabling links through forests and around ridges where higher-frequency signals fail entirely [9].

**Meshtastic** is an open-source mesh networking firmware for LoRa hardware that provides automatic message routing, store-and-forward capability, and basic encrypted messaging. Devices like the Heltec V3 (\$25), LILYGO T-Beam (\$35), and RAK WisBlock (\$40–80) form a low-cost mesh for text messaging and telemetry.

**Limitations:** Throughput is extremely low (0.3–50 kbps depending on spreading factor). This is unsuitable for broadband internet but valuable for:
- Network management and monitoring backhaul
- Emergency/disaster communications
- Text messaging when higher-bandwidth links fail
- Sensor telemetry (weather stations, solar system monitoring)

### TV White Space (TVWS) 470–698 MHz

TVWS exploits unused television broadcast channels in the VHF/UHF band, combining the excellent propagation characteristics of sub-GHz frequencies with usable broadband throughput (5–25 Mbps per channel). The FCC Part 15 Subpart H rules [10] require devices to query a geo-location database to identify available channels at their location, preventing interference with active TV broadcasts.

**Advantages for Appalachian terrain:**
- Sub-GHz propagation diffracts around ridges and penetrates foliage far better than WiFi bands
- Ranges of 10–15 km NLOS are achievable with modest antenna heights
- Power limit of 4 W EIRP (fixed) provides substantial link budget

**Challenges:**
- Equipment cost is higher (\$500–2,000 per unit) than commodity WiFi
- Available channels vary by location — rural areas typically have 3–8 usable channels
- Throughput per channel is modest (5–16 Mbps with 6 MHz channels using 802.11af or proprietary wideband)
- Equipment ecosystem is smaller: Microsoft Airband partners (Adaptrum, 6Harmonics), Radwin, and some Cambium products

TVWS is the **strongest candidate for valley-to-ridgetop access links** where direct line-of-sight is unavailable and WiFi frequencies cannot penetrate.

### 2.4/5 GHz WiFi Mesh

Commodity WiFi equipment (Ubiquiti, MikroTik, GL.iNet, TP-Link) provides the highest throughput-per-dollar for line-of-sight and near-line-of-sight links. Dual-band mesh nodes can use 5 GHz for high-bandwidth backbone links and 2.4 GHz for wider-area client access.

**Recommended platforms:**
- **Ubiquiti UniFi Mesh / airMAX:** Widely deployed in community networks, excellent management interface, \$80–300/unit
- **MikroTik RouterBOARD:** Extremely cost-effective, powerful routing capabilities, steeper learning curve, \$50–200/unit
- **GL.iNet travel routers:** Low-cost OpenWrt-based nodes suitable for household endpoints, \$30–80/unit

**In Appalachian terrain**, WiFi mesh is best suited for:
- Short-range links within valley communities (< 2 km)
- Household distribution from a local relay
- Intra-community mesh where nodes have reasonable line-of-sight

### 5 GHz Point-to-Point Backhaul

For ridgetop-to-ridgetop backbone links with clear line-of-sight, dedicated PtP equipment delivers the highest throughput:

- **Ubiquiti airFiber 5XHD:** Up to 1 Gbps, 200+ km range, \$500/pair
- **Cambium ePMP Force 400C:** 700+ Mbps, integrated antenna, \$400/unit
- **MikroTik Wireless Wire Dish:** 2 Gbps (60 GHz) with 5 GHz failover, \$600/pair
- **Ubiquiti airMAX LiteBeam 5AC Gen2:** 450 Mbps, excellent for 10–30 km links, \$100/unit

These form the high-capacity backbone connecting ridgetop relays. Link distances of 20–50 km are routine with proper antenna alignment and Fresnel zone clearance.

### CBRS 3.5 GHz

The Citizens Broadband Radio Service at 3.55–3.7 GHz provides 150 MHz of shared spectrum under a three-tier access model. The **General Authorized Access (GAA)** tier requires no license — only an SAS (Spectrum Access System) registration — making it accessible to community networks [11].

**Advantages:**
- 150 MHz bandwidth supports high throughput (50–400 Mbps)
- Better NLOS propagation than 5 GHz (though worse than sub-GHz)
- Growing equipment ecosystem from Baicells, BLiNQ, Cambium, and others

**Challenges:**
- Equipment cost is higher than WiFi (\$300–1,500/unit)
- GAA users must yield to Priority Access License (PAL) holders and incumbent military radar
- Requires SAS connectivity for channel assignment
- 3.5 GHz propagation is intermediate — better than 5 GHz but substantially worse than TVWS through foliage

CBRS is best suited as a **mid-range access technology** for serving clusters of households from a valley relay with partial line-of-sight.

### HF/VHF Amateur Radio Data (AREDN)

The **Amateur Radio Emergency Data Network (AREDN)** [12] repurposes Part 97 amateur radio spectrum for TCP/IP mesh networking using modified commodity WiFi hardware (Ubiquiti, MikroTik, TP-Link with AREDN firmware). It provides mesh routing, DHCP, DNS, and web services over ham radio frequencies.

**Critical limitation for MUUDD:** Part 97 regulations **prohibit encryption** of amateur radio transmissions (47 CFR §97.113). This is fundamentally incompatible with the MUUDD/AutonomousTrust trust model, which requires end-to-end encrypted channels. AREDN can serve as an emergency fallback or community coordination tool but **cannot carry MUUDD encrypted traffic**.

AREDN nodes may still be deployed alongside the mesh network for:
- Emergency communications when primary network fails
- Community coordination and public information
- Network monitoring and diagnostics (non-sensitive data)

## Spectrum and Regulatory Framework

### FCC Part 15 Power Limits

All unlicensed radio operations in the US are governed by FCC Part 15 rules. Key limits for community mesh deployment [13]:

| Band | Max Conducted Power | Max EIRP | Antenna Gain Rule |
|------|--------------------|---------|--------------------|
| 902–928 MHz ISM | 1 W (30 dBm) | 4 W (36 dBm) | 6 dBi max for hopping systems |
| 2.4 GHz ISM | 1 W (30 dBm) | 4 W (36 dBm) | Reduce power 1 dB per 3 dB antenna gain above 6 dBi |
| 5.15–5.25 GHz U-NII-1 | 50 mW (17 dBm) | 200 mW (23 dBm) | Indoor use only |
| 5.25–5.35 GHz U-NII-2A | 250 mW (24 dBm) | 1 W (30 dBm) | DFS required |
| 5.47–5.725 GHz U-NII-2C | 250 mW (24 dBm) | 1 W (30 dBm) | DFS required |
| 5.725–5.85 GHz U-NII-3 | 1 W (30 dBm) | — | PtP: reduce 1 dB per dB above 23 dBi antenna |
| TVWS (VHF/UHF) | — | 4 W fixed / 100 mW portable | Geo-location database required |

**Point-to-point exception (5.725–5.85 GHz):** For fixed PtP links, the FCC allows higher EIRP by reducing the power-reduction ratio — enabling high-gain directional antennas with up to **53 dBm EIRP** for long-distance backbone links. This is the regulatory basis for 20–50 km ridgetop backbone connections.

### TVWS Database Requirement

TVWS devices must query an FCC-approved geo-location database before transmitting, to verify that the selected channel is not in use by licensed TV broadcasters or other protected services. Approved databases are operated by Google, Microsoft, Broadcom (formerly Nominet US), and others [10]. Fixed devices report their GPS coordinates; the database returns a list of available channels and maximum permitted power levels.

For rural West Virginia, TVWS availability is excellent — most areas have 5–10 vacant TV channels due to low broadcaster density.

### CBRS Spectrum Access System

CBRS GAA operation requires registration with a Spectrum Access System (SAS) operated by approved administrators (Google, Federated Wireless, CommScope, Amdocs) [11]. The SAS dynamically assigns channels and power levels to avoid interference with incumbent military radar (primarily Navy shipborne radar along the coasts) and PAL holders.

In inland West Virginia, incumbent military radar interference is minimal, and PAL auction prices in rural counties are typically low or uncontested — GAA access is effectively unrestricted.

### Part 97 Amateur Radio Constraints

As noted above, amateur radio frequencies cannot carry encrypted traffic. Additionally, Part 97 prohibits communication "in which the station licensee or control operator has a pecuniary interest" (47 CFR §97.113(a)(3)), which would preclude any paid-service model. Amateur radio mesh (AREDN) is useful for emergency backup but is **not suitable as the primary MUUDD transport**.

### Experimental License Options

For novel deployments or higher power levels, the FCC offers experimental licenses (Part 5) with processing times of 30–60 days. A community network could obtain an experimental license for:
- Higher-power TVWS operation during initial deployment
- Testing novel modulation or frequency-sharing schemes
- Demonstrating viability for a future rulemaking petition

## Node Architecture

### Tier 1: Hilltop Relay Node

Hilltop relays form the high-capacity backbone of the network. Placed on ridgetops or summited peaks with clear line-of-sight to multiple valleys and adjacent ridges, these nodes carry aggregated traffic from valley communities to internet gateways.

**Bill of Materials:**

| Component | Specification | Est. Cost |
|-----------|--------------|-----------|
| Backhaul radio (×2) | Ubiquiti airMAX LiteBeam 5AC Gen2, 23 dBi | \$200 |
| Access radio | Ubiquiti airMAX Rocket 5AC Prism + 120° sector antenna | \$350 |
| TVWS radio | Adaptrum ACRS2 or equivalent, for NLOS valley links | \$1,200 |
| Router/switch | MikroTik hEX PoE (RB960PGS) | \$80 |
| LoRa module | RAK WisBlock or Heltec V3 (network monitoring) | \$50 |
| Enclosure | NEMA 4X weatherproof box, 24"×20"×10" | \$150 |
| Mast/tower | 40 ft guyed steel mast or self-supporting pole | \$800 |
| Solar panel | 400 W monocrystalline (×2 panels) | \$500 |
| Battery bank | 200 Ah LiFePO4 (12V or 24V system) | \$800 |
| Charge controller | Victron SmartSolar MPPT 100/30 | \$180 |
| PoE injectors | Passive/802.3af PoE for radio power | \$60 |
| Lightning protection | PolyPhaser or Transtector surge suppressors (×4) | \$200 |
| Grounding system | Ground rod, #6 copper, cadweld connections | \$150 |
| Cable/connectors | Shielded outdoor CAT6, LMR-400 coax, weatherproof connectors | \$200 |
| **Total per node** | | **\$4,920** |

**Solar Sizing Worked Example (38.7°N latitude, Braxton County):**

Solar resource at 38.7°N latitude yields approximately **3.0 peak sun hours per day in December** (worst month) and 5.5 hours in June [14]. System load:

- Backhaul radios (×2): 12 W each = 24 W
- Access radio + antenna: 20 W
- TVWS radio: 15 W
- Router/switch: 10 W
- LoRa module: 1 W
- **Total continuous load: 70 W → 1,680 Wh/day**

Panel sizing (December worst case, 3.0 peak sun hours, 80% system efficiency):

    Panel capacity = 1,680 / (3.0 × 0.80) = 700 W

Two 400 W panels (800 W total) provide 14% margin over the minimum 700 W requirement — adequate for cloudy-day buffering.

Battery sizing (3 days autonomy for extended cloud cover / ice storms):

    Battery capacity = 1,680 × 3 / 12V = 420 Ah at 12V

A 200 Ah LiFePO4 battery at 24V (= 400 Ah equivalent at 12V) provides approximately 2.8 days of autonomy with 80% depth of discharge — acceptable with the conservative panel sizing providing daily recharge margin.

### Tier 2: Valley Relay Node

Valley relays extend coverage from ridgetop nodes down into hollows and along river valleys where populations concentrate. They receive backhaul from a hilltop relay (via TVWS or 5 GHz directional) and distribute access to surrounding households.

**Bill of Materials:**

| Component | Specification | Est. Cost |
|-----------|--------------|-----------|
| Uplink radio | Ubiquiti LiteBeam 5AC Gen2 or TVWS CPE (directional, aimed at hilltop) | \$100–1,200 |
| Access radio | Ubiquiti airMAX LiteAP AC (120° sector) or omni | \$200 |
| LoRa module | Meshtastic node for monitoring/emergency | \$40 |
| Router | MikroTik hAP ac³ | \$70 |
| Enclosure | NEMA 4X weatherproof, 16"×14"×8" | \$80 |
| Mast | 20 ft roof/wall mount or free-standing pole | \$200 |
| Solar panel | 200 W monocrystalline | \$150 |
| Battery | 100 Ah LiFePO4 (12V) | \$400 |
| Charge controller | Victron SmartSolar MPPT 75/15 | \$100 |
| Surge protection | PolyPhaser lightning suppressor (×2) | \$100 |
| Cable/connectors | Outdoor CAT6, mounting hardware | \$100 |
| **Total per node** | | **\$1,540–\$2,640** |

Valley relay power budget is approximately 35 W continuous (840 Wh/day). A single 200 W panel at 3.0 peak sun hours yields 480 Wh (with losses), supplementing the 100 Ah battery for approximately 1.7 days autonomy. Grid power (if available at the relay site) can replace or supplement solar.

### Tier 3: Household Endpoint

The household endpoint connects a home to the mesh network. It consists of an outdoor radio unit aimed at the nearest relay and an indoor WiFi access point or Ethernet connection.

**Bill of Materials:**

| Component | Specification | Est. Cost |
|-----------|--------------|-----------|
| Outdoor CPE | Ubiquiti NanoStation 5AC Loco (directional) or TVWS CPE | \$60–500 |
| Indoor router | GL.iNet GL-MT3000 (OpenWrt) or similar | \$70 |
| PoE adapter | Passive PoE injector | \$10 |
| Mounting hardware | Wall/pole mount bracket, cable glands | \$30 |
| Ethernet cable | Outdoor-rated CAT6, 25–100 ft | \$30 |
| **Total per household** | | **\$200–\$640** |

For households with direct line-of-sight to a relay, a \$60 Ubiquiti NanoStation provides 100+ Mbps connectivity. For NLOS situations requiring TVWS, the cost rises to \$400–640 but remains two orders of magnitude below fiber construction cost.

### Tier 4: Gateway Node

Gateway nodes provide the mesh network's connection to the broader internet. Two gateway variants serve different deployment contexts.

#### Variant A: Fiber Gateway

Located where fiber, cable, or high-capacity cellular service is available — typically in or near towns (Sutton, Flatwoods, Burnsville).

**Bill of Materials (additional to Hilltop or Valley Relay):**

| Component | Specification | Est. Cost |
|-----------|--------------|-----------|
| Fiber ONT or cable modem | ISP-provided or purchased | \$0–200 |
| Enterprise router | MikroTik CCR2004-1G-12S+2XS or similar | \$400 |
| UPS | 1500 VA online UPS, 2-hour runtime | \$300 |
| **Additional cost per gateway** | | **\$700–\$900** |

Fiber gateways provide the highest bandwidth (100–1,000 Mbps symmetrical) but are constrained to locations with existing terrestrial broadband infrastructure.

#### Variant B: LEO Satellite Gateway (Starlink)

LEO satellite service — particularly Starlink — enables **any hilltop relay to double as its own internet gateway**, eliminating the requirement to route all traffic to town-based fiber POPs [21]. Hilltop sites are ideal for satellite: they have unobstructed sky view above ~25° elevation angle, which Starlink requires for constellation access.

**Bill of Materials (additional to Tier 1 Hilltop Relay):**

| Component | Specification | Est. Cost |
|-----------|--------------|-----------|
| Starlink kit | Standard dish + router + mounting hardware | \$599 |
| Ethernet adapter | Starlink Ethernet adapter for router bypass | \$25 |
| Enlarged solar array | Additional 2× 400 W panels (total 4× 400 W = 1,600 W) | \$500 |
| Enlarged battery bank | Additional 200 Ah LiFePO4 (total 400 Ah at 24V) | \$800 |
| Upgraded charge controller | Victron SmartSolar MPPT 150/45 | \$300 |
| **Additional cost per satellite gateway** | | **\$2,224** |

**Solar sizing impact:** The Starlink dish draws 50–75 W continuously (varies by generation; 60 W average assumed). Adding this to the Tier 1 base load:

- Base Tier 1 load: 70 W
- Starlink dish: 60 W
- **Total continuous load: 130 W → 3,120 Wh/day**

Panel sizing (December worst case, 3.0 peak sun hours, 80% efficiency):

    Panel capacity = 3,120 / (3.0 × 0.80) = 1,300 W

Four 400 W panels (1,600 W total) provide 23% margin. Battery sizing for 3 days autonomy:

    Battery capacity = 3,120 × 3 / 24V = 390 Ah at 24V

A 400 Ah LiFePO4 bank at 24V (doubled from base Tier 1) provides 3.1 days autonomy at 80% DoD.

**Starlink service tiers relevant to community mesh:**

| Tier | Download | Upload | Latency | Monthly Cost | Deprioritization |
|------|----------|--------|---------|-------------|-----------------|
| Residential | 50–200 Mbps | 10–25 Mbps | 20–40 ms | \$120/mo | Yes, during congestion |
| Business | 40–220 Mbps | 8–25 Mbps | 20–40 ms | \$120–250/mo | Reduced |
| Business Priority | 40–220 Mbps | 8–25 Mbps | 20–40 ms | \$250–500/mo | Priority data allotment |

For a hilltop relay serving 50–100 households, Starlink Residential (50–200 Mbps shared) provides 0.5–4 Mbps per household at peak — marginal but functional for basic broadband. Distributing multiple Starlink gateways across the backbone (one per 2–3 hilltop relays) improves per-household bandwidth by spreading load.

**Risks of satellite-dependent gateways:**
- **Single vendor dependency:** SpaceX controls pricing, terms of service, and capacity allocation. Price increases or policy changes affect the entire network.
- **Fair-use throttling:** Residential plans deprioritize heavy users during congestion. A shared gateway with dozens of households may trigger this. Business Priority mitigates this at higher cost.
- **Weather sensitivity:** Heavy rain and wet snow degrade LEO satellite links (though less than GEO satellite). Starlink's phased-array antenna can compensate for moderate weather, but severe storms cause outages.
- **Sky view at valley sites:** Valley relays (Tier 2) may have adequate sky view for Starlink if the valley is wide enough and the dish can see above surrounding ridges at ~25° elevation. Narrow hollows may not qualify.

A gateway node is essentially a hilltop or valley relay with an internet uplink added. The fiber and satellite variants can coexist — fiber gateways at town locations provide high-bandwidth, high-reliability anchors, while Starlink gateways at remote hilltops provide distributed redundancy. Multiple gateways of either type ensure that if one uplink fails, traffic reroutes through alternative gateways.

## Network Topology for Ridge-and-Valley Terrain

### Ridgetop Backbone Chain

The primary network architecture exploits the ridge system as a natural backbone corridor. Hilltop relays placed along major ridgelines form a **chain topology** with redundant cross-links:

```
    [Gateway-Flatwoods]          ☆ = Starlink uplink
          |  (fiber)
    [Hilltop-1: Powell Mtn ☆]---[Hilltop-2: Otter Creek Ridge]
          |         \                    |
    [Valley-Sutton]  [Valley-Centralia] [Valley-Burnsville ☆]
       |    \            |                |      \
     [HH] [HH]        [HH]            [HH]    [HH]
```

Each hilltop relay connects to at least **two other hilltop relays**, creating loop redundancy in the backbone. If any single hilltop node fails, traffic reroutes through the alternate backbone path.

With Starlink-equipped hilltop relays, the topology shifts from a **hub-and-spoke model** (all traffic funnels to town-based fiber POPs) to a **distributed gateway model** where multiple ridgetop nodes independently connect to the internet. This reduces backbone hop count (valley traffic exits via the nearest Starlink hilltop rather than traversing the full backbone chain), improves aggregate bandwidth (each Starlink dish adds 50–200 Mbps of independent uplink capacity), and eliminates the fiber gateway as a single point of failure for entire regions of the network.

### Valley Finger Extensions

From each hilltop relay, **valley relays extend coverage down into hollows** along a tree topology:

1. Hilltop relay provides a high-elevation anchor point
2. First valley relay, placed partway down the slope or at a valley intersection, receives signal from the hilltop
3. Additional valley relays extend along the valley floor, each relaying from the previous node
4. Household endpoints connect to the nearest valley relay

Valley chains should be limited to **3–4 hops** to maintain acceptable latency and throughput. Each hop adds 2–5 ms latency and reduces aggregate throughput by approximately 50% (in half-duplex mesh mode) or minimally (with dedicated backhaul radio).

### Saddle and Gap Relay Placement

**Saddles** (low points in ridgelines) are high-value relay sites because they:
- Provide line-of-sight into valleys on **both** sides of the ridge
- Require lower mast heights than mid-ridge locations
- Often have road access (roads frequently cross ridges at saddle points)
- Can serve as junction nodes connecting valley communities on opposite sides of a ridge

### Minimum Node Density

For the Braxton County service area (516 sq mi, ~3,465 underserved locations):

- **Ridgetop backbone:** Major ridges in the county span approximately 80 linear miles. At 5 km (3.1 mi) spacing, this requires **~26 hilltop relays**.
- **Valley relays:** With approximately 15 major hollows and valley systems, averaging 3 relays per valley, this requires **~45 valley relays**.
- **Gateways:** 3–4 gateway nodes at fiber-connected locations (Flatwoods/I-79 corridor, Sutton, Gassaway, Burnsville).
- **Household endpoints:** One per connected household. At 70% deployment of 3,465 underserved locations: **~2,425 endpoints**.

### Worked Example: Sutton–Burnsville–Flatwoods Triangle

The three largest population centers in Braxton County form a triangle approximately 15–20 km per side:

- **Flatwoods** (I-79 corridor): Gateway node with fiber internet access. Population cluster along US-19.
- **Sutton** (county seat, Elk River): Population cluster along Elk River. 10 km south of Flatwoods.
- **Burnsville** (Little Kanawha River): Population cluster in narrow valley. 22 km northeast of Sutton.

**Estimated deployment for this triangle (fiber gateway model):**

| Node Type | Count | Unit Cost | Subtotal |
|-----------|-------|-----------|----------|
| Hilltop relay | 8 | \$4,920 | \$39,360 |
| Valley relay | 12 | \$2,000 (avg) | \$24,000 |
| Fiber gateway (additional) | 2 | \$800 | \$1,600 |
| Household endpoint | 400 | \$350 (avg) | \$140,000 |
| **Total** | | | **\$204,960** |

**Estimated deployment for this triangle (Starlink gateway model):**

| Node Type | Count | Unit Cost | Subtotal |
|-----------|-------|-----------|----------|
| Hilltop relay (base) | 5 | \$4,920 | \$24,600 |
| Hilltop relay + Starlink | 3 | \$7,144 | \$21,432 |
| Valley relay | 12 | \$2,000 (avg) | \$24,000 |
| Household endpoint | 400 | \$350 (avg) | \$140,000 |
| **Total** | | | **\$210,032** |

Both models cover approximately 400 households at **\$512–\$525 per household** — compared to the fiber estimate of \$46,000–\$78,000 per demand point. The Starlink model costs slightly more in hardware but eliminates the dependency on fiber POPs in towns and provides 3 independent internet gateways with distributed load. The cost advantage over fiber remains **88:1 to 150:1**.

## Equipment and Cost Analysis

### Full County Deployment Estimate

Scaling from the triangle example to the full Braxton County service area. Two models are presented: fiber-only gateways (requiring proximity to existing broadband infrastructure) and a hybrid model using Starlink at select hilltop relays for distributed internet access.

**Model A: Fiber Gateway**

| Component | Quantity | Unit Cost | Total |
|-----------|----------|-----------|-------|
| Hilltop relay nodes | 26 | \$4,920 | \$127,920 |
| Valley relay nodes | 45 | \$2,000 | \$90,000 |
| Fiber gateway nodes (additional equipment) | 4 | \$800 | \$3,200 |
| Household endpoints | 2,425 | \$350 | \$848,750 |
| Installation labor (professional, hilltop) | 26 | \$2,000 | \$52,000 |
| Installation labor (community, valley) | 45 | \$500 | \$22,500 |
| Spare equipment pool (10%) | — | — | \$114,237 |
| Network management tools/software | — | — | \$5,000 |
| Training program (community maintainers) | — | — | \$15,000 |
| **Total infrastructure** | | | **\$1,278,607** |

**Model B: Hybrid Fiber + Starlink Gateway**

| Component | Quantity | Unit Cost | Total |
|-----------|----------|-----------|-------|
| Hilltop relay nodes (base) | 18 | \$4,920 | \$88,560 |
| Hilltop relay nodes + Starlink | 8 | \$7,144 | \$57,152 |
| Valley relay nodes | 45 | \$2,000 | \$90,000 |
| Fiber gateway nodes (additional equipment) | 2 | \$800 | \$1,600 |
| Household endpoints | 2,425 | \$350 | \$848,750 |
| Installation labor (professional, hilltop) | 26 | \$2,000 | \$52,000 |
| Installation labor (community, valley) | 45 | \$500 | \$22,500 |
| Spare equipment pool (10%) | — | — | \$121,056 |
| Network management tools/software | — | — | \$5,000 |
| Training program (community maintainers) | — | — | \$15,000 |
| **Total infrastructure** | | | **\$1,301,618** |

The hybrid model costs approximately **\$23,000 more** in hardware (1.8% increase) but provides 10 independent internet gateways (8 Starlink + 2 fiber) versus 4 fiber-only gateways — a 2.5× improvement in gateway redundancy and aggregate uplink bandwidth. With 8 Starlink dishes each providing 50–200 Mbps, the network has 400–1,600 Mbps of aggregate internet capacity distributed across the ridgetop backbone.

### Cost Comparison: Fiber vs Mesh

| Metric | Fiber (from DEP [1]) | Mesh (Fiber GW) | Mesh (Hybrid GW) |
|--------|----------------------|------------------|-------------------|
| Total infrastructure cost | \$38,200,000 | \$1,278,607 | \$1,301,618 |
| Cost per demand point | \$46,000–\$78,000 | \$527 | \$537 |
| Ongoing monthly cost (per HH) | \$70/mo | \$5–15/mo | \$5–15/mo |
| Construction timeline | 3–5 years | 6–12 months | 6–12 months |
| Gateway redundancy | N/A (single trunk) | 4 fiber POPs | 10 (2 fiber + 8 satellite) |
| Aggregate uplink bandwidth | 100–1,000 Mbps | 300–1,000 Mbps | 700–2,600 Mbps |
| Typical throughput (per HH) | 100–1,000 Mbps | 25–100 Mbps | 25–150 Mbps |
| Scalability | Requires new construction | Add nodes incrementally | Add nodes incrementally |
| Community ownership possible | Rarely (capital-intensive) | Yes (core model) | Yes (core model) |

**Both mesh models cost approximately 3% of the fiber buildout** while providing broadband-class service (25+ Mbps) sufficient for video conferencing, telehealth, remote education, and general internet use. The hybrid Starlink model provides significantly higher aggregate uplink bandwidth and gateway redundancy for a negligible increase in capital cost.

### Monthly Operating Costs

Shared infrastructure costs (hilltop relays, gateways, management) distributed across connected households:

**Model A: Fiber Gateway Only**

| Expense | Monthly Cost |
|---------|-------------|
| Internet gateway service (3 × \$100/mo business fiber) | \$300 |
| Equipment replacement fund (5% annual of infrastructure) | \$4,328 |
| Insurance (equipment and liability) | \$500 |
| Domain/hosting for management portal | \$20 |
| **Total monthly shared cost** | **\$5,148** |
| **Per household (2,425 connected)** | **\$2.12/mo** |

**Model B: Hybrid Fiber + Starlink Gateway**

| Expense | Monthly Cost |
|---------|-------------|
| Internet gateway service (2 × \$100/mo business fiber) | \$200 |
| Starlink service (8 × \$120/mo residential) | \$960 |
| Equipment replacement fund (5% annual of infrastructure) | \$4,422 |
| Insurance (equipment and liability) | \$500 |
| Domain/hosting for management portal | \$20 |
| **Total monthly shared cost** | **\$6,102** |
| **Per household (2,425 connected)** | **\$2.52/mo** |

The hybrid model's operating cost is \$0.40/mo higher per household — from \$2.12 to \$2.52 — while providing 2.5× more gateway redundancy and roughly double the aggregate internet bandwidth. Upgrading select Starlink subscriptions to Business Priority (\$250–500/mo) for higher-traffic hilltop relays would increase per-household cost to approximately \$3.00–3.50/mo, still well below \$5.

Even with a \$10–15/month cooperative membership fee (to build reserves for major repairs, expansion, and community maintainer stipends), the cost is **80–90% below commercial ISP pricing** — critical for a community where per capita income is \$21,948.

## Community Ownership Model

### Ownership Structure

The mesh network operates on a **hybrid ownership model**:

1. **Household-owned endpoints:** Each household purchases (or receives via subsidy) their own endpoint equipment. They own it, maintain it, and benefit from it directly. Cost: \$200–640 one-time.

2. **Cooperative-owned infrastructure:** Hilltop relays, valley relays, and gateway connections are owned and maintained by a community cooperative (organized as a 501(c)(12) telephone cooperative or similar structure). Members pay modest monthly dues (\$10–15) to fund maintenance, insurance, and reserves.

3. **Distributed maintenance:** Trained community members perform routine maintenance (equipment checks, antenna alignment, firmware updates) on local infrastructure. The cooperative coordinates training and provides replacement equipment.

### Precedent Networks

Several community mesh networks demonstrate the viability of this model:

**Guifi.net** (Catalonia, Spain) [15]: The world's largest community network with over 40,000 active nodes. Operates as a commons: the network infrastructure is collectively owned under a commons license, while individual connections are user-owned. Guifi.net demonstrates that community mesh networks can scale to tens of thousands of nodes and persist for over two decades (founded 2004).

**NYC Mesh** (New York City) [16]: An all-volunteer community network serving thousands of connections in Manhattan, Brooklyn, and surrounding boroughs. Operates on a suggested monthly donation model (\$20–60/mo). Demonstrates the community organizing and volunteer maintenance model in practice.

**Detroit Community Technology Project** [17]: Focuses on digital equity in underserved Detroit neighborhoods. Combines mesh networking with digital literacy training — a model directly applicable to Appalachian communities where digital skills gaps compound connectivity gaps.

**Mountain Area Information Network (MAIN)** (western North Carolina) [18]: An Appalachian-specific precedent. MAIN has operated community internet infrastructure in the mountains of western NC since 1995, demonstrating that community-owned networks can serve mountainous rural terrain for decades.

**Althea** [19]: A blockchain-incentivized mesh networking protocol where nodes earn payment for forwarding traffic. While the cryptocurrency element may not suit all communities, Althea's technical architecture — automatic payment channels between mesh nodes based on bandwidth forwarded — demonstrates a self-sustaining economic model for mesh infrastructure.

### Funding Sources

Initial deployment can leverage multiple funding streams:

- **BEAD (Broadband Equity, Access, and Deployment) program:** WV allocated \$1.21 billion. Community mesh could serve as a "last-mile" complement to BEAD-funded fiber middle-mile — mesh extends from fiber POPs into areas where fiber construction costs are prohibitive.
- **ARC (Appalachian Regional Commission)** grants for broadband and economic development
- **USDA ReConnect Program** loans and grants for rural broadband
- **FCC Emergency Connectivity Fund** and E-Rate for schools/libraries as anchor institutions
- **State of WV broadband office** funding and technical assistance
- **Foundation grants** (e.g., Internet Society, Mozilla Foundation, Ford Foundation community technology programs)

## Integration with MUUDD Software Stack

### Physical-to-Logical Layer Mapping

The mesh network provides the physical transport layer over which the MUUDD software stack operates. The mapping between network tiers and AutonomousTrust's hierarchy is natural:

| Network Tier | AutonomousTrust Role | Function |
|--------------|---------------------|----------|
| Gateway node (fiber or Starlink) | **Root trust anchor** | Connects to external trust networks; validates inter-community identity |
| Hilltop relay (with or without Starlink) | **Domain authority** | Manages trust for a geographic region; aggregates reputation data |
| Valley relay | **Local authority** | Manages trust for a valley community; relays identity assertions |
| Household endpoint | **Peer node** | End-user identity; participates in local trust web |

In the hybrid gateway model, Starlink-equipped hilltop relays serve as both domain authority and root trust anchor. This dual role aligns naturally: the node already aggregates traffic for its region, so adding trust anchoring at the same point minimizes protocol overhead.

### Trust-Based Network Admission

AutonomousTrust's identity and reputation protocol provides a critical security function for the mesh network: **preventing rogue nodes from joining the network or intercepting traffic**.

When a new node joins the network:
1. The node generates a cryptographic identity (public/private key pair) through AutonomousTrust
2. An existing trusted node (community maintainer) vouches for the new node through the trust protocol
3. The new node receives a signed certificate that the network recognizes
4. Mesh routing protocols only forward traffic through nodes with valid trust credentials
5. Misbehaving nodes (e.g., traffic manipulation, excessive resource consumption) accumulate negative reputation scores and can be automatically isolated

This model eliminates the need for a centralized authentication server — trust is distributed across the network, matching the physical architecture.

### Encrypted Communications

All MUUDD traffic is end-to-end encrypted via AutonomousTrust's channel protocol. This means:
- **Relay nodes cannot read traffic** they forward (only encrypted ciphertext transits intermediate hops)
- **Network operators cannot surveil users** (a critical property for community trust)
- **Part 97 amateur spectrum cannot be used** (encryption prohibition), reinforcing the design choice to use Part 15 unlicensed spectrum

### Resilience and Partition Tolerance

The mesh topology combined with AutonomousTrust's store-and-forward capability provides resilience against network partitions:
- If a hilltop relay fails, AutonomousTrust queues messages for delivery when connectivity is restored
- The LoRa/Meshtastic layer provides a low-bandwidth emergency channel that can relay critical messages even when the primary 5 GHz backbone is down
- Local trust relationships (within a valley community) continue to function even if the community is temporarily isolated from the broader network

## Deployment Strategy

### Phase 1: Ridgetop Backbone and Gateway (Months 1–3)

**Objective:** Establish the high-capacity backbone connecting 3–4 ridgetop sites with internet gateway access.

**Activities:**
1. Site survey of ridgetop locations — verify line-of-sight, access roads, property ownership/easements, and sky view for Starlink
2. Deploy 3–4 hilltop relay nodes along the Flatwoods–Sutton–Burnsville corridor, with Starlink dishes on 2–3 of these
3. Install 1–2 fiber gateway nodes at broadband-connected locations (Flatwoods I-79 corridor, Sutton) if available
4. Establish PtP backbone links between hilltop relays
5. Validate backbone throughput, gateway failover routing, and multi-path internet egress (fiber + Starlink)
6. Deploy LoRa/Meshtastic monitoring layer across all backbone sites

**Milestone:** End-to-end internet connectivity verified from all hilltop sites via both fiber and Starlink gateways. Backbone throughput > 200 Mbps per link.

### Phase 2: Valley Relays to Population Clusters (Months 3–6)

**Objective:** Extend coverage from ridgetop backbone down into the three primary valley communities.

**Activities:**
1. Deploy 8–12 valley relay nodes in Sutton, Flatwoods, and Burnsville population areas
2. Install TVWS links for NLOS valley-to-ridgetop connections where direct 5 GHz is blocked
3. Validate coverage area using signal mapping (drive/walk tests with spectrum analyzer)
4. Recruit and begin training 6–8 community maintainers (2–3 per valley community)
5. Establish cooperative organizational structure (bylaws, membership, bank account)

**Milestone:** Coverage available to 200+ households across three communities. Community cooperative formally organized.

### Phase 3: Household Endpoints and Training (Months 6–12)

**Objective:** Connect individual households and build community capacity for self-sustaining operation.

**Activities:**
1. Install household endpoints for first wave of subscribers (target: 100–200 in first 3 months)
2. Conduct digital literacy workshops (basic internet skills, network troubleshooting, security)
3. Train community maintainers on equipment installation, antenna alignment, firmware updates, and basic troubleshooting
4. Deploy additional valley relays to extend coverage to outlying hollows based on demand
5. Document procedures, create maintenance guides, establish parts inventory
6. Transition daily operations to community cooperative management

**Milestone:** 400+ households connected. Cooperative self-managing with trained community maintainers. Monthly operating costs covered by membership fees.

### Phase 4: Expansion and Optimization (Year 2+)

**Ongoing activities:**
- Extend network to additional communities within and beyond Braxton County
- Upgrade backbone capacity as throughput demands grow
- Integrate AutonomousTrust software stack for identity management and encrypted communications
- Explore inter-community mesh links to adjacent county networks
- Evaluate emerging technologies (WiFi 6E/7, improved TVWS equipment, competing LEO constellations such as Amazon Kuiper)
- Share deployment playbook with other Appalachian communities

## References

1. Braxton County Broadband Committee. "Braxton County Digital Equity Plan." Braxton County, WV (2023). Developed under the West Virginia Office of Broadband Digital Equity Act planning process.

2. Cardwell, D.H., Erwin, R.B., and Woodward, H.P. "Geologic Map of West Virginia." *West Virginia Geological and Economic Survey* (1968). 1:250,000 scale.

3. Morin, R.S., Cook, G.W., Barnett, C.J., Butler, B.J., Crocker, S.J., et al. "West Virginia Forests: 2013." *USDA Forest Service Resource Bulletin NRS-105* (2016). [doi:10.2737/NRS-RB-105](https://doi.org/10.2737/NRS-RB-105)

4. International Telecommunication Union. "Attenuation in Vegetation." *ITU-R Recommendation P.833-10* (2021). Provides models for estimating signal attenuation through different types of vegetation at various frequencies.

5. Telecommunications Industry Association. "Structural Standard for Antenna Supporting Structures, Antennas, and Small Wind Turbine Support Structures." *TIA-222-H* (2017). Defines ice and wind loading criteria for communications structures.

6. Seybold, J.S. *Introduction to RF Propagation*. John Wiley & Sons (2005). Comprehensive treatment of radio wave propagation including Fresnel zone analysis and terrain effects.

7. International Telecommunication Union. "Propagation by Diffraction." *ITU-R Recommendation P.526-15* (2019). Methods for calculating diffraction loss over terrain obstacles including knife-edge and rounded-hill models.

8. International Telecommunication Union. "Specific Attenuation Model for Rain for Use in Prediction Methods." *ITU-R Recommendation P.838-3* (2005). Rain attenuation coefficients by frequency and polarization.

9. Semtech Corporation. "LoRa and LoRaWAN: A Technical Overview." *Semtech Technical Paper AN1200.22* (2020). [https://www.semtech.com/lora](https://www.semtech.com/lora)

10. Federal Communications Commission. "Unlicensed Operation in the TV Broadcast Bands." *47 CFR Part 15, Subpart H* (2010, amended 2020). Rules governing TV White Space device operation, database requirements, and power limits.

11. Federal Communications Commission. "Citizens Broadband Radio Service." *47 CFR Part 96* (2015, amended 2020). Three-tier spectrum sharing framework for 3.55–3.7 GHz band including GAA and PAL access tiers.

12. Amateur Radio Emergency Data Network (AREDN). "AREDN Documentation." (2024). [https://www.arednmesh.org/](https://www.arednmesh.org/)

13. Federal Communications Commission. "Radio Frequency Devices." *47 CFR Part 15* (2024). General rules for unlicensed radio operation including power limits and spurious emission standards.

14. National Renewable Energy Laboratory. "PVWatts Calculator." NREL (2024). Solar resource data and photovoltaic system performance estimates by location. [https://pvwatts.nrel.gov/](https://pvwatts.nrel.gov/)

15. Baig, R., Roca, R., Freitag, F., and Navarro, L. "Guifi.net, a Crowdsourced Network Infrastructure Held in Common." *Computer Networks* 90, 150–165 (2015). [doi:10.1016/j.comnet.2015.07.009](https://doi.org/10.1016/j.comnet.2015.07.009)

16. NYC Mesh. "NYC Mesh Community Network." (2024). [https://www.nycmesh.net/](https://www.nycmesh.net/)

17. Detroit Community Technology Project. "Digital Stewards Program." Allied Media Projects (2024). [https://detroitcommunitytech.org/](https://detroitcommunitytech.org/)

18. Mountain Area Information Network (MAIN). "About MAIN." (2024). Community internet service provider serving western North Carolina since 1995. [https://main.nc.us/](https://main.nc.us/)

19. Althea Network. "Althea: Incentivized Mesh Networking." (2024). [https://www.althea.net/](https://www.althea.net/)

20. Flickenger, R., Okay, S., Pietrosemoli, E., Zennaro, M., and Fonda, C. "Very Long Distance Wi-Fi Networks." *Proceedings of the 2nd ACM SIGCOMM Workshop on Networked Systems for Developing Regions* (2008). [doi:10.1145/1397705.1397707](https://doi.org/10.1145/1397705.1397707)

21. Michel, F., Trevisan, M., Giordano, D., and Bonaventure, O. "A First Look at Starlink Performance." *ACM Internet Measurement Conference* (2022). [doi:10.1145/3517745.3561416](https://doi.org/10.1145/3517745.3561416)
