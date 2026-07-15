*[Sustainable Space](../../doc/sustainable_space.md) > [MUUDD](muudd.md) > Decentralization Momentum (Preparedness Framing)*

# Infrastructure Independence: The Case for Decentralized Communications

A survey of recent events and trends, from grid failures and disaster response to digital financial control and Big Tech censorship, that validate the core premises of MUUDD and AutonomousTrust. The common thread: centralized infrastructure of all kinds is a single point of failure, and communities that depend on it are at the mercy of forces they cannot influence.

---

## Disaster Preparedness and Grid Vulnerability

### Hurricane Helene (September 2024)

Hurricane Helene struck western North Carolina as a Category 4 storm, killing over 230 people and cutting power to 5.9 million customers across 10 states. In the Appalachian community of Swannanoa, NC, all cellular and internet infrastructure was destroyed. Meshtastic LoRa mesh devices were deployed in North Carolina and Tennessee after the storm wiped out first responder communication systems, enabling text-based coordination when nothing else worked. Amateur radio (VHF handy talkies) proved critical for household-to-household contact.

Following the storm, communities began formally evaluating Meshtastic as permanent emergency infrastructure. The city of Monticello, Florida considered a formal Meshtastic emergency communication system proposal. The Salisbury Emergency Radio Network (SERN) was established as a community-based mesh communication network for emergency preparedness [^hp].

[^hp]: Heartland Emergency Preparedness, "Building a Community Meshtastic Network: Step-by-Step Guide for Emergency Preparedness", https://heartlandemergencypreparedness.com/2025/08/25/building-a-community-meshtastic-network-step-by-step-guide-for-emergency-preparedness/

### The April 2025 Iberian Blackout

On April 28, 2025, a massive power blackout cut electricity to 47 million people across Spain and Portugal for approximately 10 hours, the largest European blackout in over 20 years. With no electricity or cell service, families with Meshtastic devices stayed connected and coordinated emergency help. The event became a significant catalyst for Meshtastic adoption worldwide and reignited global discussion about "black sky" events [^ib].

[^ib]: Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout

### EMP and Grid Fragility

The vulnerability of the U.S. power grid is not hypothetical. MITRE has published research calling EMP "the dangerous but overlooked threat" [^mitre]. Fox News reported extensively on experts warning that "EMPs, cyber threats and AI could cripple the US" power grid, with a nuclear EMP event capable of wiping out satellite ground stations, financial markets, healthcare systems, transportation networks, and military command and control.

Texas remains the emblematic case for grid fragility:

- Winter Storm Fern (January 2026) forced the DOE to issue an emergency order for backup generation to prevent blackouts.
- A 2025 audit revealed ongoing problems with winterization, years after the deadly 2021 freeze.
- AI data center growth is straining grid capacity, with ERCOT warning of rising demand from AI and crypto facilities.
- Texas Governor Abbott's Executive Order GA-49 tasked TDEM with assessing EMP threats to critical infrastructure. Senate Bill 75 created the Texas Grid Security Commission to address vulnerabilities across energy, communications, emergency services, and transportation.

The Texas Public Policy Foundation concluded that "Texas Has Strengthened Its Grid, But It Is Still Vulnerable to the Next Winter Storm" [^tppf]. These concerns directly feed interest in communications that do not depend on the grid.

[^mitre]: MITRE, "Electromagnetic Pulse: The Dangerous but Overlooked Threat", https://www.mitre.org/news-insights/impact-story/electromagnetic-pulse-dangerous-overlooked-threat
[^tppf]: Texas Public Policy Foundation, https://www.texaspolicy.com/texas-has-strengthened-its-grid-but-it-is-still-vulnerable-to-the-next-winter-storm/

---

## The Mesh Networking Surge

### Meshtastic

The largest open-source mesh movement. Meshtastic runs on 100+ distinct hardware variants using LoRa (Long Range) radio to create encrypted, decentralized mesh networks that operate without cell towers, internet, or power infrastructure. By 2025, nodes were deployed across all seven continents. A certified hardware partner program launched as its first sustainable revenue stream, and the project was featured at Embedded World 2026 as a serious business [^tiot].

The user base spans hikers, ham radio operators, preppers, emergency responders, ranchers, and vehicle fleet trackers. Prepper-oriented publications have embraced it: Suburban Survival Blog published "Meshtastic and LoRa: The Off-Grid Comms Lifeline for Preppers" [^ssb]; Ham Radio 360 asked "Unveiling the Meshtastic Revolution: A Prepper's Ultimate Companion?" [^hr360]; SpecFive markets devices specifically to the preparedness community [^sf].

[^tiot]: Talking IoT, "Meshtastic at Embedded World 2026: How an Open-Source Mesh Network Became a Serious Business", https://talkingiot.io/meshtastic-at-embedded-world-2026-how-an-open-source-mesh-network-became-a-serious-business/
[^ssb]: Suburban Survival Blog, https://suburbansurvivalblog.com/meshtastic-lora-the-off-grid-communication-lifeline-every-prepper-needs/
[^hr360]: Ham Radio 360, http://hamradio360.com/unveiling-the-meshtastic-revolution-a-preppers-ultimate-companion/
[^sf]: SpecFive, https://specfive.com/blogs/articles/the-best-radio-for-prepper

### MeshCore

Developed by Scott Powell in late 2024, inspired partly by New Zealand's Cyclone Gabrielle devastation. A lightweight C++ library offering structured routing (vs. Meshtastic's broadcast flooding), store-and-forward mechanisms, and encrypted direct messages. Works on Heltec, RAK Wireless, and other LoRa hardware.

### Reticulum

A transport-agnostic mesh protocol seeing development in native mobile UIs, federation with other public-key protocols (Nostr, Waku), and deployment in humanitarian and educational infrastructure. Positioned as a more flexible long-term solution for larger meshes.

### goTenna

Professional-grade mobile mesh networking used by 100,000+ customers across 49 countries, including U.S. and European military forces, California Fire, Google, and the City of New York. The Air Force awarded a \$15 million contract for the Pro X3 next-generation tactical mesh device. CivTAK/ATAK (Android Team Awareness Kit), originally military software, has civilian adoption for tactical mapping and coordination and integrates with goTenna mesh.

### Bitchat

Announced July 2025 by Jack Dorsey. A peer-to-peer encrypted messaging app using Bluetooth Low Energy mesh networking plus the Nostr protocol for global reach. Saw massive real-world adoption during crises: 70,000 downloads in one week in Madagascar, nearly 50,000 in a single day in Nepal during protests (the PM resigned after a counterproductive social media ban), and spikes in Uganda and Iran during internet blackouts (January 2026).

### Market Growth

The wireless mesh network market was valued at \$12.36 billion in 2024 and is projected to reach \$24-31 billion by 2030-2032 (12% CAGR) [^nst].

[^nst]: Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/

### Ham Radio Resurgence

Over 750,000 licensed U.S. operators as of 2024-2025, with growing Technician license enrollment among the preparedness community. ARES (Amateur Radio Emergency Service) and RACES (Radio Amateur Civil Emergency Service) provide organized emergency communications frameworks. The prepper community maintains the "3-3-3 Radio Plan" (monitor channel 3 every 3 hours for 3 minutes) and comprehensive SHTF frequency lists across FRS, GMRS, MURS, CB, and ham bands.

---

## Rural Self-Reliance and the Failure of Federal Broadband

### BEAD Program Failures

The \$42.5 billion Broadband Equity, Access, and Deployment (BEAD) program has been widely criticized. By early 2025, only three states (Louisiana, Delaware, Nevada) had final proposals approved and no connections had been completed. Senator John Thune (R-SD) called BEAD "a sad story of government inefficiency and progressivism run amok," arguing the Biden administration "loaded up the program with requirements Congress never envisioned" [^thune]. Eleven of 13 Republican Senate Commerce Committee members signed a letter opposing NTIA's fiber-only mandates. Under Trump (2025), BEAD approvals were rescinded and states were required to reopen bidding, but Texas saw its allocation cut from \$3.3 billion to \$1.3 billion.

The Mackinac Center for Public Policy published "More trouble ahead for government-funded internet" [^mac]. Citizens Against Government Waste published "BEAD: Broadband or Bust" [^cagw].

[^thune]: Sen. John Thune, "Broadband Blunders Leave Americans Disconnected", https://www.thune.senate.gov/public/index.cfm/2024/9/broadband-blunders-leave-americans-disconnected
[^mac]: Mackinac Center for Public Policy, https://www.mackinac.org/blog/2024/more-trouble-ahead-for-government-funded-internet
[^cagw]: Citizens Against Government Waste, https://www.cagw.org/bead-broadband-or-bust/

### Rural Mesh Is Filling the Gap

In rural Wyoming, a community of ranchers established a 30-node Meshtastic network spanning over 2,000 square miles for property monitoring and coordination. LoRa technology enables farm-specific applications: soil moisture monitoring, cattle management, equipment status tracking, and worker coordination -- all without cellular infrastructure. NDSU Agriculture Extension published a guide on "Basics of LoRa Technology for Crop and Livestock Management" [^ndsu].

[^ndsu]: NDSU Agriculture Extension, https://www.ndsu.edu/agriculture/extension/publications/basics-lora-technology-crop-and-livestock-management

---

## Digital Financial Control

### CBDCs: Programmable Government Money

Conservative and libertarian opposition to Central Bank Digital Currencies (CBDCs) has become a major policy priority. The core concern: unlike physical cash or even current bank deposits, a CBDC *could* be programmed to expire after a set period, be restricted to approved merchants, enforce automatic tax withholding, implement geographic restrictions, or be frozen without judicial process.

The Heritage Foundation called CBDCs "another governmental control tactic" threatening "the privacy and civil liberties of American citizens." Chief Economist E.J. Antoni stated: "Central bank digital currencies give faceless bureaucrats the ultimate control over how you live your life" [^hf]. The Cato Institute published "CBDC Spells Doom for Financial Privacy," with Nicholas Anthony's book *Digital Currency or Digital Control?* arguing that the centralized ledger architecture is inherently a surveillance tool [^cato]. The Mises Institute connected CBDCs to the Canadian trucker convoy bank account freezes of 2022 as a preview of what becomes routine when financial infrastructure is centralized.

[^hf]: Heritage Foundation, https://www.heritage.org/markets-and-finance/commentary/are-congressional-republicans-about-greenlight-cbdc
[^cato]: Cato Institute, "CBDC Spells Doom for Financial Privacy", https://www.cato.org/free-society/fall-2024/cbdc-spells-doom-financial-privacy

### Legislative Response

- **CBDC Anti-Surveillance State Act (H.R. 5403):** Passed the House in May 2024. Sponsored by Rep. Tom Emmer (R-MN). Prohibits the Federal Reserve from issuing a CBDC directly to individuals. A strengthened version passed 219-210 in July 2025, codifying Trump's executive order.
- **Sen. Ted Cruz (R-TX)** introduced companion Senate legislation: "A CBDC could give the federal government the ability to spy on every single financial transaction made by every single American."
- **Trump Executive Order (January 2025):** Directed federal agencies to cease any CBDC research, development, or pilot programs. Reversed Biden's Executive Order 14067.
- **Florida SB 7054 (May 2023):** First state to ban CBDCs, signed by DeSantis. Model for approximately 20+ states that introduced similar legislation through 2024-2025.
- **South Dakota:** Governor Noem vetoed a UCC update specifically because it included language that could facilitate CBDC adoption.

### Operation Choke Point 2.0

A coordinated federal effort to de-bank crypto and other disfavored industries through financial services pressure. The House Financial Services Committee documented at least 30 digital asset entities losing banking access between 2022-2024, with FDIC letters to approximately 24 banks demanding they delay crypto services. Marc Andreessen discussed this on Joe Rogan's podcast, making "debanking" a mainstream concern. The original Operation Choke Point under Obama targeted gun sellers, payday lenders, and other legal businesses [^ocp].

[^ocp]: https://www.opchokepoint2.org/

### Off-Grid Financial Tools

The parallel economy movement is producing tangible alternatives. MeshtBank enables off-grid digital currency exchange using Meshtastic messaging protocols -- accounts, balances, and transactions without internet [^hack]. PublicSq operates a "patriot economy" marketplace connecting consumers with businesses aligned with conservative values. Multiple state-level "right to pay with cash" bills reflect concerns that CBDC-driven elimination of cash would remove the last anonymous payment method.

[^hack]: Hackaday, "Off-Grid, Small-Scale Payment System", https://hackaday.com/2025/12/05/off-grid-small-scale-payment-system/

---

## Age Verification as Stealth Digital Identity

A wave of age verification legislation is creating what critics call a universal digital ID system under child-safety justification. To verify that *anyone* is not a minor, *everyone* must prove their identity.

### The Legislative Wave

- **Louisiana** (Jan 2023): First state to require government ID for adult sites via LA Wallet. Template for ~20 other states.
- **Texas HB 1181** (2023): Supreme Court largely upheld in *Free Speech Coalition v. Paxton* (2025).
- **KOSA** (Kids Online Safety Act): Passed Senate 91-3, July 2024. State AG enforcement powers concerned both left and right -- conservatives worried about scope creep, libertarians about the surveillance architecture.
- **UK Online Safety Act** (Oct 2023): Age verification plus the Section 122 "spy clause" enabling demands to scan private messages.
- **Australia** (Nov 2024): Banned under-16s from social media with mandatory platform-level verification.
- **EU eIDAS 2.0**: Requires member states to offer digital identity wallets by 2026.

### The Infrastructure Is the Policy

The conservative critique mirrors the CBDC concern: once the infrastructure for universal digital identity exists, mission creep is inevitable. Age verification today, content filtering tomorrow, political speech controls after that. Every verification mechanism -- government ID upload, facial age estimation, third-party verification services, device-level signals, digital identity wallets -- creates either a direct identity trail or requires trust in a centralized authority.

South Korea tried a real-name internet registration system and repealed it after massive data breaches. China's is the system critics warn the West is building incrementally.

---

## Big Tech Censorship and Platform Dependence

### Documented Censorship

The Senate Commerce Committee investigation (April 2024) found that "Big Tech Weaponizes Terms of Service to Silence the Right," documenting how platforms used "a roadmap written by left-wing activist groups" to target conservative organizations [^scc].

[^scc]: Senate Commerce Committee, "Big Tech Weaponizes Terms of Service to Silence the Right", https://www.commerce.senate.gov/2024/4/senate-commerce-investigation-reveals-how-big-tech-weaponizes-terms-of-service-to-silence-the-right

Religious content has been a particular target:

- Grace Fellowship Church had a Facebook Nativity play announcement removed as "spam"
- YouTube suspended Michael Savage for sharing a video about Christmas and persecuted Christians
- YouTube added fact-checking labels to a Hickory Grove Presbyterian Church sermon about abortion
- Google's Blogger deplatformed Catholic blog Messa in Latino for discussing homosexuality
- Apple banned an app by Living Hope Ministries after complaints
- Google banned an Idaho church's app under hate speech policies
- Meta deplatformed pro-life users; Google refused ads for abortion pill reversal

Sources: National Catholic Register [^ncr], NAPA Legal Institute [^napa], First Liberty Institute [^fl].

[^ncr]: National Catholic Register, https://www.ncregister.com/news/big-tech-s-big-bias-religious-conservatives-face-scary-pattern-of-censorship
[^napa]: NAPA Legal Institute, https://www.napalegalinstitute.org/post/how-big-tech-targets-faith-groups-for-censorship
[^fl]: First Liberty Institute, https://firstliberty.org/news/big-tech-isnt-neutral/

### Infrastructure Independence Response

The Parler shutdown by AWS in January 2021 was the watershed moment. Rumble CEO Chris Pavlovski responded by building independent cloud infrastructure: "When we saw that happen, we took it upon ourselves to make sure that could never happen to a platform like Rumble." Rumble launched a cloud service in 2024 and now hosts Truth Social's infrastructure [^fb].

Gab, founded by Andrew Torba, expanded its own data center to "preserve a parallel Christian society on the internet for generations to come," running on forked Mastodon (decentralized social network software). In April 2025, Gab restricted UK access over Online Safety Act compliance concerns.

Alliance Defending Freedom launched a new Center for Free Speech in December 2024 specifically to address corporate censorship of religious and conservative speech. As of August 2025, ADF had been involved in 83 Supreme Court cases [^adf].

[^fb]: Fox Business, https://www.foxbusiness.com/politics/rumble-launches-effort-combat-existential-threat-video-platform-free-big-tech
[^adf]: Alliance Defending Freedom, https://adflegal.org/press-release/adf-submits-comment-to-ftc-outlining-extensive-examples-of-big-tech-censorship/

---

## Surveillance State Concerns

### FISA Section 702 Reauthorization (April 2024)

FISA reauthorization became a major flashpoint. Trump posted on Truth Social calling on Republicans to "KILL FISA." Nineteen conservative "privacy hawks" initially revolted against Republican leadership, blocking a procedural vote. A bipartisan warrant-requirement amendment failed on a 212-212 tie. Speaker Johnson reduced reauthorization from five to two years as a compromise. The bill passed 60-34 in the Senate [^fisa].

[^fisa]: NPR, https://www.npr.org/2024/04/20/1246076114/senate-passes-reauthorization-surveillance-program-fisa

### DOGE and Government Data Centralization

The Department of Government Efficiency (DOGE), established January 2025, raised alarm by seeking sweeping access to sensitive government databases. DOGE raced to build a centralized database combining Social Security, tax, and medical data. Whistleblower disclosures alleged attempts to exfiltrate and alter data [^npr].

[^npr]: NPR, "DOGE Is Trying to Access All of It", https://www.npr.org/2025/03/11/nx-s1-5305054/doge-elon-musk-security-data-information-privacy

### The Three Pillars Argument

Conservative and libertarian critics increasingly present digital control as a triad: control the money (CBDC), control the identity (digital ID/age verification), control the communications (platform censorship + surveillance). Each pillar reinforces the others. The argument: resistance to any one pillar requires building decentralized alternatives across all three domains -- financial, identity, and communications.

---

## Community Cohesion and Parallel Institutions

### Homeschool and Microschool Networks

Over 3.1 million U.S. students were homeschooled as of 2023, up from 2.5 million in 2019. Microschools, learning pods, and homeschool co-ops are expanding nationwide. Heath Brown's research documents this as "parallel politics" -- local and national associations filling social and political voids, creating "tight-knit and politically active" communities [^par]. The Foundation for Economic Education published "The Rise of Homeschooling Was Broad and Bipartisan" [^fee].

These communities coordinate through email lists, private messaging groups, and co-op scheduling tools, increasingly seeking platforms they control rather than Big Tech services that might moderate their educational or religious content.

[^par]: Heath Brown, "Homeschooling the Right: How Conservative Education Activism Erodes the State", Journals of OpenEdition, https://journals.openedition.org/ejas/17223
[^fee]: Foundation for Economic Education, https://fee.org/articles/the-rise-of-homeschooling-was-broad-and-bipartisan/

### Homesteading and Local Food Networks

Homesteading participation grew significantly in 2024-2025, driven by grocery costs and supply chain anxiety. Simple homesteading practices can reduce grocery bills by up to 25% annually. Community barter systems, seed-sharing networks, and tool-lending cooperatives use local communications to coordinate outside mainstream platforms. MeshtBank's off-grid digital currency exchange demonstrates how mesh infrastructure can support local economic activity without internet dependency.

### Church Communities

Churches represent perhaps the most natural unit of community mesh networking -- geographically proximate, socially bonded, with shared values and regular gathering patterns. When platforms suppress religious content, the need for independent communications infrastructure becomes existential rather than merely convenient.

---

## Scaling Challenges

The fundamental technical bottleneck for mesh networking is throughput scalability. Multi-hop transmission severely limits per-node throughput as networks grow. Most existing solutions struggle beyond a few dozen nodes without degraded performance. At 100+ nodes, hierarchical architectures with layered routing become necessary.

Additional challenges include spectrum regulation (LoRa ISM band power and duty-cycle limits vary by country), achieving sufficient node density in sparse rural areas, and building sustainable revenue models for open-source projects.

---

## Alignment with MUUDD and AutonomousTrust

These converging trends validate MUUDD's core design premises from a preparedness and community-sovereignty perspective. The common thread across every topic above is **infrastructure dependence as vulnerability** -- whether the infrastructure is the power grid, the internet, the financial system, or Big Tech platforms.

### The threat model

MUUDD was designed for a world where centralized infrastructure is a single point of failure. That world is here:

- **Grid failure**: Helene, the Iberian blackout, Texas winter storms, EMP vulnerability
- **Communications failure**: Cell/internet outages during disasters, government shutdowns (296 in 2024), platform censorship
- **Financial control**: CBDCs, debanking (Operation Choke Point), account freezes (Canadian truckers)
- **Identity control**: Age verification as universal digital ID, eIDAS wallets, social credit architecture
- **Surveillance**: FISA 702, DOGE data centralization, corporate data harvesting

### What MUUDD provides that existing tools lack

Current mesh solutions (Meshtastic, MeshCore, Reticulum, goTenna, Bitchat) address pieces of the problem. None address the whole:

| Gap in Current Ecosystem | MUUDD/AT Answer |
|---|---|
| No identity or reputation system | AT's cryptographic identity + Byzantine reputation scoring |
| No defense against infiltration (Sybil attacks) | Consensus-based admission (PoW/PoS/PoA) |
| No community governance framework | Kith-Covenant's distributed governance protocols |
| Scaling past ~100 nodes degrades | AT's hierarchical trust domains mapping to tiered routing |
| No secure fleet management | AT Phase 3: decentralized Paxos-based updates replacing SSH |
| Single-technology approaches | MUUDD spans physical (radio), transport (DTN), trust (AT), and social (Kith-Covenant) layers |
| Vulnerable to mandatory digital ID | AT's anonymous-first architecture with behavioral reputation |
| No off-grid economic exchange | AT trust framework + mesh transport enables MeshtBank-style systems |
| Crisis tool only, not daily infrastructure | AT reputation + Kith-Covenant governance enable persistent community networks |

### The three pillars, addressed

The conservative/libertarian "three pillars" framework -- free speech, free transactions, free movement -- maps directly onto MUUDD's layered architecture:

1. **Communications sovereignty**: Mesh-routed, user-owned physical layer (radio) + encrypted DTN transport + AT trust evaluation = communications that no ISP, platform, or government agency can shut down
2. **Identity sovereignty**: AT's anonymous-first design with behavioral trust replaces government-issued credentials, certificate authorities, and age-verification infrastructure with peer-evaluated reputation
3. **Financial sovereignty**: AT's trust framework over mesh transport provides the infrastructure for decentralized economic exchange without centralized financial intermediaries

### Community as the unit of resilience

MUUDD's design recognizes what the preparedness community already knows: the individual is not the unit of survival -- the community is. Meshtastic gives a family a radio. MUUDD gives a community an infrastructure: communications, trust evaluation, governance, and the ability to interoperate with other communities at scale, from a rural county to an orbital habitat.

---

## Sources

- Heartland Emergency Preparedness, "Building a Community Meshtastic Network", https://heartlandemergencypreparedness.com/2025/08/25/building-a-community-meshtastic-network-step-by-step-guide-for-emergency-preparedness/
- Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout
- MITRE, "Electromagnetic Pulse: The Dangerous but Overlooked Threat", https://www.mitre.org/news-insights/impact-story/electromagnetic-pulse-dangerous-overlooked-threat
- Texas Public Policy Foundation, "Texas Has Strengthened Its Grid, But It Is Still Vulnerable", https://www.texaspolicy.com/texas-has-strengthened-its-grid-but-it-is-still-vulnerable-to-the-next-winter-storm/
- Talking IoT, "Meshtastic at Embedded World 2026", https://talkingiot.io/meshtastic-at-embedded-world-2026-how-an-open-source-mesh-network-became-a-serious-business/
- Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/
- Sen. John Thune, "Broadband Blunders Leave Americans Disconnected", https://www.thune.senate.gov/public/index.cfm/2024/9/broadband-blunders-leave-americans-disconnected
- Mackinac Center for Public Policy, "More Trouble Ahead for Government-Funded Internet", https://www.mackinac.org/blog/2024/more-trouble-ahead-for-government-funded-internet
- NDSU Agriculture Extension, "Basics of LoRa Technology for Crop and Livestock Management", https://www.ndsu.edu/agriculture/extension/publications/basics-lora-technology-crop-and-livestock-management
- Heritage Foundation on CBDCs, https://www.heritage.org/markets-and-finance/commentary/are-congressional-republicans-about-greenlight-cbdc
- Cato Institute, "CBDC Spells Doom for Financial Privacy", https://www.cato.org/free-society/fall-2024/cbdc-spells-doom-financial-privacy
- Operation Choke Point 2.0, https://www.opchokepoint2.org/
- Hackaday, "Off-Grid, Small-Scale Payment System", https://hackaday.com/2025/12/05/off-grid-small-scale-payment-system/
- Senate Commerce Committee, "Big Tech Weaponizes Terms of Service", https://www.commerce.senate.gov/2024/4/senate-commerce-investigation-reveals-how-big-tech-weaponizes-terms-of-service-to-silence-the-right
- National Catholic Register, "Big Tech's Big Bias", https://www.ncregister.com/news/big-tech-s-big-bias-religious-conservatives-face-scary-pattern-of-censorship
- Fox Business, "Rumble Launches Effort to Combat Existential Threat", https://www.foxbusiness.com/politics/rumble-launches-effort-combat-existential-threat-video-platform-free-big-tech
- NPR, "Senate Passes FISA Reauthorization", https://www.npr.org/2024/04/20/1246076114/senate-passes-reauthorization-surveillance-program-fisa
- NPR, "DOGE Is Trying to Access All of It", https://www.npr.org/2025/03/11/nx-s1-5305054/doge-elon-musk-security-data-information-privacy
- Heath Brown, "Homeschooling the Right", https://journals.openedition.org/ejas/17223
- Foundation for Economic Education, "The Rise of Homeschooling", https://fee.org/articles/the-rise-of-homeschooling-was-broad-and-bipartisan/
- Alliance Defending Freedom, https://adflegal.org/press-release/adf-submits-comment-to-ftc-outlining-extensive-examples-of-big-tech-censorship/
- Freedom House, *Freedom on the Net 2025*, https://freedomhouse.org/report/freedom-net/2025/uncertain-future-global-internet
- NAPA Legal Institute, https://www.napalegalinstitute.org/post/how-big-tech-targets-faith-groups-for-censorship
- First Liberty Institute, https://firstliberty.org/news/big-tech-isnt-neutral/
- Citizens Against Government Waste, https://www.cagw.org/bead-broadband-or-bust/
