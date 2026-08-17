*Previous: [Protocol timeouts at deep-space latency](../docs/space-protocol-timeouts.md)*

# The Case for Decentralized Communications: 2024-2026 Landscape

A survey of recent events and trends that validate the core premises of MUUDD
and AutonomousTrust -- that centralized internet infrastructure is becoming a
liability for civil liberties, and that trust-based, user-owned mesh
communications are not merely desirable but increasingly necessary.

## Government Internet Shutdowns at Record Levels

Government-imposed internet shutdowns surged from 78 in 2016 to 296 in 2024
across 54 countries -- a nearly 280% increase. At least 244 more occurred in
2025. India led for the seventh consecutive year with 84 shutdowns in 2024;
Myanmar had 85 disruptions (76 by the military). These shutdowns cost the global
economy an estimated $7.69 billion in 2024, covering approximately 88,000 hours
of outages.

Freedom House reported that global internet freedom declined for the 15th
consecutive year in 2025, with people in at least 57 of 72 countries arrested
for online expression -- a record high [^fh].

[^fh]: Freedom House, *Freedom on the Net 2025: The Uncertain Future of the Global Internet*, https://freedomhouse.org/report/freedom-net/2025/uncertain-future-global-internet

## Net Neutrality Collapse in the U.S.

The FCC reinstated net neutrality rules in May 2024, but in January 2025 the
Sixth Circuit Court of Appeals struck them down, ruling that the FCC classification of ISPs as telecommunications services was inconsistent with the
Telecommunications Act of 1996. The future of net neutrality now depends on
Congress rather than the FCC, though state-level laws in California, Washington,
and Oregon remain in place. Without federal rules, ISPs have broad power to
shape what consumers can see and access.

## Social Media Censorship and Platform Consolidation

### TikTok Ban

The U.S. TikTok ban (upheld by the Supreme Court in January 2025) triggered mass
user protests, with creators demonstrating at the U.S. Capitol and millions
migrating to the Chinese app Xiaohongshu (RedNote) under the viral hashtag
"#tiktokrefugee." The ACLU condemned the ruling. Experts warned the ban could
embolden censorship abroad and shatter the U.S.'s reputation as a free speech
champion [^fpf].

[^fpf]: Freedom of the Press Foundation, "Banning TikTok Enables Online Censorship", https://freedom.press/issues/tiktok-ban-weakens-first-amendment/

### Mass Exodus from X/Twitter

The exodus from Twitter/X following Elon the Musk acquisition drove massive
growth in decentralized alternatives. Bluesky achieved 930% year-over-year
growth in 2024, reaching 26.44 million users by January 2025 (gaining 700,000+
users in the week after the 2024 U.S. election alone). Mastodon grew to nearly 9
million users by late 2024. The decentralized social network market is projected
to grow at 23.6% CAGR to reach $101.2 billion by 2035.

## DOGE and Government Data Centralization

The Department of Government Efficiency (DOGE), established January 20, 2025
under the Trump administration, raised alarm by seeking sweeping access to
sensitive government databases containing personal information on millions of
Americans. DOGE raced to build a centralized database combining Social Security,
tax, and medical data. Whistleblower disclosures alleged attempts to exfiltrate
and alter data. MIT Technology Review warned this "tech takeover threatens the
safety and stability of our critical data" [^mit].

[^mit]: MIT Technology Review, "DOGE's Tech Takeover Threatens the Safety and Stability of Our Critical Data", https://www.technologyreview.com/2025/04/14/1114988/doges-tech-takeover-threatens-the-safety-and-stability-of-our-critical-data/

## The "deGoogle" Movement and Surveillance Awareness

Growing awareness of the Google data collection practices and data breaches in
2024 accelerated the "deGoogle" movement, with users seeking privacy-focused
alternatives like Proton, Tuta, and Freespoke. Decentralized identity management
using blockchain and zero-knowledge proofs is gaining adoption as a
privacy-first alternative to surveillance capitalism.

## UN Cybercrime Treaty

The UN General Assembly adopted the Convention Against Cybercrime on December
24, 2024, which 72 nations signed at a ceremony in Hanoi in October 2025.
Critics including the EFF, Human Rights Watch, and the Foundation for Defense of
Democracies call it a "Trojan horse for suppressing dissent" that obligates
states to establish broad electronic surveillance powers without adequate human
rights safeguards. Its vague definition of cybercrime -- any crime committed
using technology -- and deference to individual countries on human rights
protections alarm digital rights organizations [^fdd].

[^fdd]: Foundation for Defense of Democracies, "The UN Cybercrime Treaty: A Trojan Horse for Suppressing Dissent", https://www.fdd.org/analysis/2025/10/23/the-un-cybercrime-treaty-a-trojan-horse-for-suppressing-dissent/

## Age Verification as Stealth Digital Identity

Firstly, a wave of age verification legislation across Western democracies is
creating what critics call a universal digital ID system under child-safety
justification. The core problem: to verify that *anyone* is not a minor,
*everyone* must prove their identity. As the EFF has framed it, these laws
require citizens to "show their papers to go online."

### Legislative Wave

- **Louisiana** (Jan 2023): First state to require government ID (via LA Wallet) for adult sites. Became a template for ~20 other states.
- **Texas HB 1181** (2023): Supreme Court largely upheld it in *Free Speech Coalition v. Paxton* (2025), setting broad precedent.
- **KOSA** (Kids Online Safety Act): Passed the U.S. Senate 91-3 in July 2024. Does not explicitly mandate age verification, but critics argue compliance is impossible without it. State attorney general enforcement powers raised alarm that "harmful to minors" would be politically weaponized -- particularly against LGBTQ+ content.
- **UK Online Safety Act** (Oct 2023): Ofcom-enforced age verification for pornography sites, plus the controversial Section 122 "spy clause" enabling demands to scan private messages.
- **Australia** (Nov 2024): Banned under-16s from social media entirely, requiring platform-level age verification with no parental consent override.
- **EU eIDAS 2.0**: Requires member states to offer digital identity wallets by 2026 -- critics see this as the infrastructure that makes universal online ID inevitable, with age verification as the politically palatable entry point.
- **France**: Piloting a "double anonymity" protocol for age verification, but even this still requires trusting two intermediary parties and creates correlation risks.

### Verification Mechanisms -- All Problematic

| Method | Privacy Concern |
|---|---|
| Government ID upload | Direct identity linkage to browsing activity |
| Third-party verification (Yoti, etc.) | Third party knows both identity AND what you access |
| Facial age estimation (AI) | Normalizes facial recognition; racial bias documented |
| Device-level age signals | OS vendors become identity gatekeepers |
| Open banking checks | Financial identity exposed |
| Digital identity wallets (eIDAS) | Builds universal ID infrastructure directly |
| Zero-knowledge proofs | Still requires trusting intermediary parties |

### The Mission Creep Problem

Once universal digital ID infrastructure exists for age checks, expansion to
other purposes -- content filtering, political speech control, tax enforcement,
immigration status -- becomes technically trivial and politically tempting.
Critics draw direct parallels to China's real-name internet registration system
and Russia's sovereign internet law. South Korea tried a real-name system and
repealed it after massive data breaches -- a cautionary precedent.

### Opposition

The EFF, ACLU, Center for Democracy & Technology, Fight for the Future, Access
Now, the Open Rights Group (UK), and Article 19 have all raised alarm. Industry
has responded with direct action: Pornhub blocked access from entire states
rather than comply, dropping traffic 80% in Louisiana. VPN usage spiked in every
affected state, demonstrating that the laws push users toward circumvention
tools rather than achieving compliance.

---

## The Rise of Mesh Networking

Against this backdrop, interest in decentralized communications infrastructure
is surging -- from hobbyist experimentation to commercially viable ecosystems
with real-world crisis validation.

### Market Growth

The wireless mesh network market was valued at \$12.36 billion in 2024 and is
projected to reach \$24-31 billion by 2030-2032 (12% CAGR), driven by demand for
resilient, self-healing network architectures across smart cities, industrial
IoT, and next-generation wireless [^nst].

[^nst]: Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/

### Key Projects and Initiatives

**Meshtastic** is the largest open-source mesh movement. It runs on 100+
distinct hardware variants from manufacturers worldwide, supporting GPS
tracking, text messaging, and telemetry over LoRa radio. A certified hardware
partner program launched as its first sustainable revenue stream. Featured at
Embedded World 2026 as a serious business. User base spans hikers, ham radio
operators, preppers, emergency responders, and vehicle fleet trackers.

**MeshCore** was developed by Scott Powell in late 2024, inspired partly by New
Zealand's Cyclone Gabrielle devastation. It is a lightweight C++ library
offering structured routing (vs. Meshtastic's broadcast flooding),
store-and-forward mechanisms, and encrypted direct messages. A post about it on
Mastodon went trending within an hour -- reflecting the intersection of
decentralized social media and mesh networking communities.

**Reticulum** is a transport-agnostic mesh protocol seeing development in native
mobile UIs, federation with other public-key protocols (Nostr, Waku), and
deployment in humanitarian and educational infrastructure. Positioned as a more
flexible long-term solution for larger meshes.

**Bitchat** was announced in July 2025 by Twitter co-founder Jack Dorsey. A
peer-to-peer encrypted messaging app using Bluetooth Low Energy mesh networking
plus the Nostr protocol for global reach. Beta hit its 10,000-user TestFlight
limit quickly. Real-world adoption surged during political crises: 70,000
downloads in one week in Madagascar (September 2025), nearly 50,000 downloads in
a single day in Nepal during protests, and spikes in Uganda and Iran during
internet blackouts (January 2026).

**NYC Mesh** is a community-owned wireless network in New York City providing
affordable, resilient internet independent of commercial providers. Working with
the city on NYCHA buildings to bridge the digital divide.

**goTenna** provides professional-grade mobile mesh networking used by 100,000+
customers across 49 countries, including U.S. and European military forces,
California Fire, Google, and the City of New York.

### Crisis Validation: The April 2025 Iberian Blackout

On April 28, 2025, a massive power blackout cut electricity to 47 million people
across Spain and Portugal for approximately 10 hours -- the largest European
blackout in over 20 years. With no electricity or cell service, families with
Meshtastic devices stayed connected and coordinated emergency help. This event
became a significant catalyst for Meshtastic adoption and public awareness of
mesh networking [^ib].

[^ib]: Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout

### Protest-Driven Adoption

The most compelling evidence that this is more than hobbyist enthusiasm comes
from protest-driven download spikes. People under actual censorship are actively
reaching for mesh tools:

- **Madagascar** (Sept 2025): 70,000 Bitchat downloads in one week during protests
- **Nepal** (Sept 2025): Nearly 50,000 Bitchat downloads in a single day during protests that led to the PM's resignation -- partly fueled by a counterproductive social media ban
- **Uganda and Iran** (Jan 2026): Bitchat download spikes during internet blackouts
- **Myanmar**: Activists used Bridgefy BLE mesh networks to circumvent military shutdowns
- **Spain/Portugal** (Apr 2025): Meshtastic proved its value during the Iberian blackout

### Scaling Challenges

The fundamental technical bottleneck is throughput scalability. Multi-hop
transmission severely limits per-node throughput as networks grow. Most existing
solutions struggle beyond a few dozen nodes without degraded performance. At
100+ nodes, hierarchical architectures with layered routing become necessary.
Proposed solutions include multi-tier designs with 3D beamforming and
heterogeneous relay nodes to maintain constant hop counts per tier.

Social challenges include spectrum regulation (LoRa ISM band power/duty-cycle
limits), the need for sufficient node density (most useful precisely where
hardest to deploy), and sustainable revenue models.

---

## Alignment with MUUDD and AutonomousTrust

These converging trends validate MUUDD's core design premises and reveal gaps in
the current mesh ecosystem that MUUDD/AT are uniquely positioned to fill.

### The threat model is materializing

Although the design predates the trend, MUUDD was built for a world where
centralized internet infrastructure is a liability for civil liberties. That
world has arrived:

- **Access control from above**: Shutdowns (296 in 2024), net neutrality collapse, platform bans
- **Identity control from above**: Age verification as stealth digital ID, eIDAS wallets, DOGE data centralization
- **Speech control from above**: UN Cybercrime Treaty, DSA enforcement, state AG powers under KOSA
- **Surveillance from above**: Corporate data harvesting, government database consolidation

### Gaps in the current mesh ecosystem

The current ecosystem is fragmented -- Meshtastic for hardware hobbyists,
Bitchat for protest messaging, Reticulum for protocol purists, community meshes
for local broadband. None of them address:

| Gap | MUUDD/AT Answer |
|---|---|
| No identity or reputation system | AT's cryptographic identity + Byzantine reputation scoring |
| Sybil attacks on open meshes | Consensus-based admission (PoW/PoS/PoA) |
| No governance framework | Kith-Covenant distributed governance protocols |
| Scaling past ~100 nodes degrades | AT's hierarchical trust domains mapping to tiered routing |
| No secure fleet management | AT Phase 3: decentralized Paxos-based updates replacing SSH |
| Single-technology approaches | MUUDD spans physical (radio), transport (DTN), trust (AT), and social (Kith-Covenant) layers |
| Vulnerable to stealth digital ID | AT's anonymous-first architecture with behavioral reputation instead of government credentials |

### AT as the missing trust layer

Current mesh projects have no answer to the identity problem that
age-verification laws exploit. AT's anonymous-first design -- where trust is
earned through behavior, not conferred by presenting government-issued
credentials -- directly counters the centralized identity infrastructure being
built under child-safety justification. AT's four-level trust gradient (refuse
comms, communicate only, offer services, full data sharing) provides nuanced
access control without requiring any external identity authority.

### From crisis tool to sustainable infrastructure

Current mesh adoption is spike-driven (blackouts, protests). AT's reputation
system and Kith-Covenant's governance framework could enable the persistent
community networks that make mesh viable long-term -- the transition from
emergency tool to daily infrastructure.

### From terrestrial to space

No other mesh project has DTN integration or orbital-scale design. MUUDD's
architecture spans from rural Appalachian hilltop relays to asteroid-belt
inter-habitat links, with a unified trust and governance model across all
scales.

---

## Sources

- Freedom House, *Freedom on the Net 2025*, https://freedomhouse.org/report/freedom-net/2025/uncertain-future-global-internet
- Internet Society, "Internet Shutdowns Policy Brief", https://www.internetsociety.org/resources/policybriefs/2025/internet-shutdowns/
- Surfshark, "Internet Shutdowns 2025", https://surfshark.com/research/study/internet-shutdowns-2025
- Freedom of the Press Foundation, "Banning TikTok Enables Online Censorship", https://freedom.press/issues/tiktok-ban-weakens-first-amendment/
- MIT Technology Review, "DOGE's Tech Takeover", https://www.technologyreview.com/2025/04/14/1114988/doges-tech-takeover-threatens-the-safety-and-stability-of-our-critical-data/
- Foundation for Defense of Democracies, "UN Cybercrime Treaty", https://www.fdd.org/analysis/2025/10/23/the-un-cybercrime-treaty-a-trojan-horse-for-suppressing-dissent/
- EFF, "UN Cybercrime Treaty Still Flawed", https://www.eff.org/deeplinks/2024/12/still-flawed-and-lacking-safeguards-un-cybercrime-treaty-goes-un-general-assembly
- Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/
- Talking IoT, "Meshtastic at Embedded World 2026", https://talkingiot.io/meshtastic-at-embedded-world-2026-how-an-open-source-mesh-network-became-a-serious-business/
- CNBC, "Jack Dorsey Launches WhatsApp Rival Built on Bluetooth", https://www.cnbc.com/2025/07/07/jack-dorsey-whatsapp-bluetooth.html
- Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout
- NPR, "DOGE Is Trying to Access All of It", https://www.npr.org/2025/03/11/nx-s1-5305054/doge-elon-musk-security-data-information-privacy
- DWT, "Sixth Circuit Invalidates FCC's 2024 Net Neutrality Order", https://www.dwt.com/blogs/broadband-advisor/2025/01/6th-circuit-fcc-net-neutrality-order
- Human Rights Watch, "Joint Statement on the Signing of the UN Convention on Cybercrime", https://www.hrw.org/news/2025/10/24/joint-statement-on-the-signing-of-the-un-convention-on-cybercrime

---

*Next: [Infrastructure independence](decentralization_momentum_alt.md)*
