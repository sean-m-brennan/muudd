*Previous: [Protocol timeouts at deep-space latency](../docs/space-protocol-timeouts.md)*

# Infrastructure independence: the case for decentralized communications

A survey of recent events that bear on why we are building MUUDD and
AutonomousTrust. The thread running through all of it is that centralized
infrastructure of every kind is a single point of failure, and a community that
depends on it is at the mercy of forces it cannot influence. That holds whether
the infrastructure is the power grid, the internet, the financial system or a
Big Tech platform.

---

## The grid, and what happens when it stops

### Hurricane Helene, September 2024

Helene struck western North Carolina as a Category 4 storm, killing over 230
people and cutting power to 5.9 million customers across 10 states. In the
Appalachian community of Swannanoa, every piece of cellular and internet
infrastructure was destroyed. Meshtastic LoRa mesh devices were deployed across
North Carolina and Tennessee after the storm wiped out first-responder
communications, and text coordination worked when nothing else did. Amateur
radio on VHF handy talkies carried household-to-household contact.

Communities began evaluating Meshtastic as permanent emergency infrastructure
afterward rather than as a stopgap. Monticello, Florida took up a formal
Meshtastic emergency communication proposal, and the Salisbury Emergency Radio
Network was established as a community-based mesh for emergency preparedness
[^hp].

[^hp]: Heartland Emergency Preparedness, "Building a Community Meshtastic Network: Step-by-Step Guide for Emergency Preparedness", https://heartlandemergencypreparedness.com/2025/08/25/building-a-community-meshtastic-network-step-by-step-guide-for-emergency-preparedness/

### The Iberian blackout, April 2025

On 28 April 2025 a power failure cut electricity to 47 million people across
Spain and Portugal for roughly ten hours, the largest European blackout in over
twenty years. With no electricity and no cell service, families holding
Meshtastic devices stayed in contact and coordinated help. The event did more
for mesh adoption than any amount of advocacy had, and it reopened the public
argument about "black sky" events [^ib].

[^ib]: Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout

### EMP and grid fragility

The vulnerability of the U.S. grid is not hypothetical. MITRE has published
research calling EMP "the dangerous but overlooked threat" [^mitre], and experts
have warned at length that EMPs, cyber attack and AI-driven demand could between
them cripple the grid. A nuclear EMP event would take out satellite ground
stations, financial markets, healthcare systems, transportation and military
command and control together.

Texas is the emblematic case, and the record there is not reassuring.

- Winter Storm Fern in January 2026 forced the DOE to issue an emergency order for backup generation to head off blackouts.
- A 2025 audit found winterization problems still outstanding, years after the deadly 2021 freeze.
- AI data center growth is straining capacity, and ERCOT has warned about rising demand from AI and crypto facilities.
- Governor Abbott's Executive Order GA-49 tasked TDEM with assessing EMP threats to critical infrastructure, and Senate Bill 75 created the Texas Grid Security Commission covering energy, communications, emergency services and transportation.

The Texas Public Policy Foundation concluded that Texas has strengthened its
grid and remains vulnerable to the next winter storm [^tppf]. Concerns of that
shape feed directly into interest in communications that do not depend on the
grid at all.

[^mitre]: MITRE, "Electromagnetic Pulse: The Dangerous but Overlooked Threat", https://www.mitre.org/news-insights/impact-story/electromagnetic-pulse-dangerous-overlooked-threat
[^tppf]: Texas Public Policy Foundation, https://www.texaspolicy.com/texas-has-strengthened-its-grid-but-it-is-still-vulnerable-to-the-next-winter-storm/

---

## Control of the network

### Shutdowns at record levels

Government-imposed internet shutdowns rose from 78 in 2016 to 296 in 2024 across
54 countries, a near-280% increase, and at least 244 more followed in 2025.
India led for the seventh consecutive year with 84; Myanmar had 85 disruptions,
76 of them by the military. The 2024 total cost the global economy an estimated
$7.69 billion across roughly 88,000 hours of outage.

Freedom House recorded global internet freedom declining for the fifteenth
consecutive year in 2025, with people in at least 57 of 72 countries arrested
for online expression, a record [^fh].

[^fh]: Freedom House, *Freedom on the Net 2025: The Uncertain Future of the Global Internet*, https://freedomhouse.org/report/freedom-net/2025/uncertain-future-global-internet

### Net neutrality collapse in the U.S.

The FCC reinstated net neutrality rules in May 2024, and in January 2025 the
Sixth Circuit struck them down, holding that classifying ISPs as
telecommunications services was inconsistent with the Telecommunications Act of
1996. The question now rests with Congress rather than the FCC. State laws in
California, Washington and Oregon remain, and absent federal rules an ISP has
broad power to shape what its customers can reach.

### Platform bans and platform exodus

The U.S. TikTok ban, upheld by the Supreme Court in January 2025, triggered mass
protest, with creators demonstrating at the Capitol and millions migrating to
the Chinese app Xiaohongshu under the hashtag "#tiktokrefugee". The ACLU
condemned the ruling, and observers warned the ban would embolden censorship
abroad and cost the U.S. its standing as a free-speech champion [^fpf].

The exodus from Twitter/X after Elon Musk's acquisition drove sharp growth in
decentralized alternatives. Bluesky grew 930% year over year in 2024 and reached
26.44 million users by January 2025, taking on more than 700,000 in the week
after the 2024 election alone. Mastodon reached nearly 9 million by late 2024.
The decentralized social network market is projected to compound at 23.6% to
$101.2 billion by 2035.

[^fpf]: Freedom of the Press Foundation, "Banning TikTok Enables Online Censorship", https://freedom.press/issues/tiktok-ban-weakens-first-amendment/

### The UN Cybercrime Treaty

The UN General Assembly adopted the Convention Against Cybercrime on 24 December
2024, and 72 nations signed at a ceremony in Hanoi in October 2025. The EFF,
Human Rights Watch and the Foundation for Defense of Democracies each call it a
Trojan horse for suppressing dissent, one that obligates states to establish
broad electronic surveillance powers with no adequate human-rights safeguard.
Its definition of cybercrime is any crime committed using technology, and it
defers to individual countries on rights protections [^fdd].

[^fdd]: Foundation for Defense of Democracies, "The UN Cybercrime Treaty: A Trojan Horse for Suppressing Dissent", https://www.fdd.org/analysis/2025/10/23/the-un-cybercrime-treaty-a-trojan-horse-for-suppressing-dissent/

### Platform censorship, and the response to it

A Senate Commerce Committee investigation in April 2024 found that big platforms
weaponize terms of service against the right, working from what it described as
a roadmap written by left-wing activist groups [^scc].

Religious content has been a particular target. Grace Fellowship Church had a
Facebook Nativity play announcement removed as spam. YouTube suspended Michael
Savage over a video about Christmas and persecuted Christians, and attached
fact-checking labels to a Hickory Grove Presbyterian sermon on abortion. Google's
Blogger deplatformed the Catholic blog Messa in Latino over a discussion of
homosexuality, and Google banned an Idaho church's app under its hate speech
policy. Apple banned an app by Living Hope Ministries after complaints. Meta
deplatformed pro-life users, and Google refused ads for abortion pill reversal
[^ncr] [^napa] [^fl].

The AWS shutdown of Parler in January 2021 was the watershed. Rumble's CEO Chris
Pavlovski answered it by building independent cloud infrastructure, on the
reasoning that when they saw that happen they took it on themselves to make sure
it could never happen to a platform like Rumble. Rumble launched a cloud service
in 2024 and now hosts Truth Social's infrastructure [^fb]. Gab, founded by
Andrew Torba, expanded its own data center to preserve a parallel Christian
society on the internet for generations to come, running on forked Mastodon, and
in April 2025 restricted UK access over Online Safety Act compliance. Alliance
Defending Freedom opened a Center for Free Speech in December 2024 for exactly
this class of case, and by August 2025 had been involved in 83 Supreme Court
cases [^adf].

[^scc]: Senate Commerce Committee, "Big Tech Weaponizes Terms of Service to Silence the Right", https://www.commerce.senate.gov/2024/4/senate-commerce-investigation-reveals-how-big-tech-weaponizes-terms-of-service-to-silence-the-right
[^ncr]: National Catholic Register, https://www.ncregister.com/news/big-tech-s-big-bias-religious-conservatives-face-scary-pattern-of-censorship
[^napa]: NAPA Legal Institute, https://www.napalegalinstitute.org/post/how-big-tech-targets-faith-groups-for-censorship
[^fl]: First Liberty Institute, https://firstliberty.org/news/big-tech-isnt-neutral/
[^fb]: Fox Business, https://www.foxbusiness.com/politics/rumble-launches-effort-combat-existential-threat-video-platform-free-big-tech
[^adf]: Alliance Defending Freedom, https://adflegal.org/press-release/adf-submits-comment-to-ftc-outlining-extensive-examples-of-big-tech-censorship/

---

## Control of identity

A wave of age verification legislation across Western democracies is building
what critics call a universal digital ID under child-safety justification. The
mechanism is simple enough to state. To verify that *anyone* is not a minor,
*everyone* must prove who they are. The EFF puts it as requiring citizens to
show their papers to go online.

### The legislative wave

- **Louisiana** (Jan 2023): first state to require government ID, via LA Wallet, for adult sites. Template for roughly 20 others.
- **Texas HB 1181** (2023): largely upheld by the Supreme Court in *Free Speech Coalition v. Paxton* (2025), setting broad precedent.
- **KOSA**, the Kids Online Safety Act: passed the Senate 91-3 in July 2024. It mandates no verification explicitly, and compliance without it is not obviously possible. State attorney-general enforcement powers drew alarm from both directions, conservatives over scope creep and libertarians over the surveillance architecture, and over "harmful to minors" being weaponized against LGBTQ+ content.
- **UK Online Safety Act** (Oct 2023): Ofcom-enforced verification for pornography sites, plus the Section 122 "spy clause" enabling demands to scan private messages.
- **Australia** (Nov 2024): under-16s banned from social media entirely, with platform-level verification and no parental consent override.
- **EU eIDAS 2.0**: member states must offer digital identity wallets by 2026. Critics read this as the infrastructure that makes universal online ID inevitable, with age verification the politically palatable way in.
- **France**: piloting a "double anonymity" protocol, which still requires trusting two intermediaries and still creates correlation risk.

### Every mechanism has the same shape

| Method | Privacy concern |
|---|---|
| Government ID upload | Direct identity linkage to browsing activity |
| Third-party verification (Yoti and similar) | The third party knows both who you are and what you reach |
| Facial age estimation | Normalizes facial recognition; racial bias documented |
| Device-level age signals | OS vendors become identity gatekeepers |
| Open banking checks | Financial identity exposed |
| Digital identity wallets (eIDAS) | Builds the universal ID infrastructure outright |
| Zero-knowledge proofs | Still requires trusting an intermediary |

### The infrastructure is the policy

Once universal digital ID exists for age checks, extending it to content
filtering, political speech control, tax enforcement or immigration status is
technically trivial and politically tempting. Critics draw the parallel to
China's real-name registration and Russia's sovereign internet law. South Korea
built a real-name system and repealed it after mass data breaches, which is the
cautionary case rather than the theoretical one.

The EFF, ACLU, Center for Democracy & Technology, Fight for the Future, Access
Now, the Open Rights Group and Article 19 have all objected. Industry answered
with direct action: Pornhub blocked whole states rather than comply, and traffic
in Louisiana fell 80%. VPN use spiked in every affected state, which is the laws
pushing users toward circumvention rather than toward compliance.

---

## Control of money

### CBDCs as programmable government money

Opposition to Central Bank Digital Currencies has become a policy priority on
the right, and the objection is structural rather than partisan whatever its
framing. Unlike cash, or even a current bank deposit, a CBDC *could* be
programmed to expire on a date, to work only at approved merchants, to withhold
tax automatically, to stop at a geographic boundary, or to freeze with no
judicial process at all.

The Heritage Foundation calls CBDCs another governmental control tactic that
threatens the privacy and civil liberties of American citizens, and its chief
economist E.J. Antoni holds that they give faceless bureaucrats the ultimate
control over how you live your life [^hf]. The Cato Institute published "CBDC
Spells Doom for Financial Privacy", and Nicholas Anthony's *Digital Currency or
Digital Control?* argues that a centralized ledger is inherently a surveillance
tool [^cato]. The Mises Institute reads the Canadian trucker convoy account
freezes of 2022 as a preview of what becomes routine once financial
infrastructure is centralized.

[^hf]: Heritage Foundation, https://www.heritage.org/markets-and-finance/commentary/are-congressional-republicans-about-greenlight-cbdc
[^cato]: Cato Institute, "CBDC Spells Doom for Financial Privacy", https://www.cato.org/free-society/fall-2024/cbdc-spells-doom-financial-privacy

### The legislative response

- **CBDC Anti-Surveillance State Act (H.R. 5403):** passed the House in May 2024, sponsored by Rep. Tom Emmer. It bars the Federal Reserve from issuing a CBDC directly to individuals. A strengthened version passed 219-210 in July 2025, codifying the executive order.
- **Sen. Ted Cruz** introduced companion Senate legislation, on the argument that a CBDC could give the federal government the ability to spy on every single financial transaction made by every single American.
- **Trump Executive Order (January 2025):** directed federal agencies to cease CBDC research, development and pilots, reversing Executive Order 14067.
- **Florida SB 7054 (May 2023):** first state ban, model for the 20-plus states that introduced similar bills through 2024 and 2025.
- **South Dakota:** Governor Noem vetoed a UCC update specifically over language that could facilitate CBDC adoption.

### Operation Choke Point 2.0

A coordinated federal effort to de-bank crypto and other disfavored industries
through pressure on financial services. The House Financial Services Committee
documented at least 30 digital asset entities losing banking access between 2022
and 2024, with FDIC letters to roughly 24 banks demanding they delay crypto
services. Marc Andreessen discussing it on the Joe Rogan podcast made
"debanking" a mainstream concern. The original Operation Choke Point under Obama
targeted gun sellers, payday lenders and other legal businesses [^ocp].

[^ocp]: https://www.opchokepoint2.org/

### Off-grid financial tools

The parallel economy is producing working alternatives rather than manifestos.
MeshtBank runs off-grid digital currency exchange over Meshtastic messaging,
with accounts, balances and transactions and no internet anywhere in the loop
[^hack]. PublicSq operates a marketplace connecting consumers with businesses
aligned to conservative values. State-level right-to-pay-with-cash bills reflect
the concern that eliminating cash removes the last anonymous payment method
there is.

[^hack]: Hackaday, "Off-Grid, Small-Scale Payment System", https://hackaday.com/2025/12/05/off-grid-small-scale-payment-system/

---

## Surveillance, and the three pillars

### FISA Section 702, April 2024

Reauthorization became a flashpoint. Trump called publicly on Republicans to
kill FISA. Nineteen privacy hawks revolted against Republican leadership and
blocked a procedural vote. A bipartisan warrant-requirement amendment failed on
a 212-212 tie. Speaker Johnson cut reauthorization from five years to two as a
compromise, and the bill passed the Senate 60-34 [^fisa].

### DOGE and government data centralization

The Department of Government Efficiency, established 20 January 2025, drew alarm
by seeking sweeping access to government databases holding personal information
on millions of Americans, and raced to build a centralized store combining
Social Security, tax and medical data. Whistleblower disclosures alleged
attempts to exfiltrate and alter data. MIT Technology Review warned that the
tech takeover threatens the safety and stability of critical data [^mit] [^npr].

[^fisa]: NPR, https://www.npr.org/2024/04/20/1246076114/senate-passes-reauthorization-surveillance-program-fisa
[^mit]: MIT Technology Review, "DOGE's Tech Takeover Threatens the Safety and Stability of Our Critical Data", https://www.technologyreview.com/2025/04/14/1114988/doges-tech-takeover-threatens-the-safety-and-stability-of-our-critical-data/
[^npr]: NPR, "DOGE Is Trying to Access All of It", https://www.npr.org/2025/03/11/nx-s1-5305054/doge-elon-musk-security-data-information-privacy

### Leaving the platforms

Awareness of Google's data collection practices, sharpened by the 2024 breaches,
accelerated the "deGoogle" movement toward Proton, Tuta and Freespoke.
Decentralized identity management built on blockchain and zero-knowledge proofs
is picking up adoption as the privacy-first alternative to surveillance
capitalism.

### The three pillars

Critics increasingly present digital control as a triad. Control the money
through a central bank digital currency, control identity through digital ID and
age verification, control communications through platform censorship and
surveillance. Each pillar holds the others up, which is why resisting any one of
them means building decentralized alternatives across all three.

---

## The mesh surge

Against that backdrop, interest in decentralized communications infrastructure
has gone from hobbyist experiment to a commercially viable ecosystem with
real-world crisis validation behind it.

**Meshtastic** is the largest open-source mesh movement. It runs on more than
100 distinct hardware variants using LoRa radio to build encrypted decentralized
meshes that need no cell tower, no internet and no power infrastructure, and by
2025 nodes were deployed on all seven continents. A certified hardware partner
program launched as its first sustainable revenue stream, and the project was
featured at Embedded World 2026 as a serious business [^tiot]. Its user base
spans hikers, ham operators, preppers, emergency responders, ranchers and fleet
trackers, and the preparedness press has embraced it: Suburban Survival Blog on
the off-grid comms lifeline [^ssb], Ham Radio 360 on the prepper's ultimate
companion [^hr360], SpecFive marketing hardware to the community directly [^sf].

**MeshCore** was developed by Scott Powell in late 2024, prompted partly by New
Zealand's Cyclone Gabrielle. It is a lightweight C++ library with structured
routing rather than Meshtastic's broadcast flooding, plus store-and-forward and
encrypted direct messages, running on Heltec, RAK Wireless and other LoRa
hardware. A post about it on Mastodon trended within the hour, which is the
decentralized social and mesh communities turning out to be the same people.

**Reticulum** is a transport-agnostic mesh protocol, with native mobile UIs in
development, federation with other public-key protocols including Nostr and
Waku, and deployment in humanitarian and educational infrastructure. It is
positioned as the more flexible long-term answer for larger meshes.

**goTenna** sells professional-grade mobile mesh to more than 100,000 customers
across 49 countries, including U.S. and European military forces, California
Fire, Google and the City of New York. The Air Force awarded a $15 million
contract for the Pro X3 tactical device. CivTAK/ATAK, originally military
software, has civilian adoption for tactical mapping and integrates with goTenna
mesh.

**Bitchat** was announced in July 2025 by Jack Dorsey, a peer-to-peer encrypted
messenger over Bluetooth Low Energy mesh plus Nostr for global reach. Beta hit
its 10,000-user TestFlight limit almost at once.

**NYC Mesh** is a community-owned wireless network providing affordable
resilient internet independent of commercial providers, working with the city on
NYCHA buildings to close the digital divide.

**Ham radio** has not gone anywhere either. Over 750,000 licensed U.S. operators
as of 2024-2025, with Technician enrollment growing in the preparedness
community. ARES and RACES supply the organized emergency frameworks, and the
prepper community maintains the 3-3-3 radio plan and frequency lists across FRS,
GMRS, MURS, CB and the ham bands.

The wireless mesh market was valued at $12.36 billion in 2024 and is projected
to reach $24-31 billion by 2030-2032, compounding at 12%, on demand for
resilient self-healing architectures across smart cities, industrial IoT and
next-generation wireless [^nst].

[^tiot]: Talking IoT, "Meshtastic at Embedded World 2026: How an Open-Source Mesh Network Became a Serious Business", https://talkingiot.io/meshtastic-at-embedded-world-2026-how-an-open-source-mesh-network-became-a-serious-business/
[^ssb]: Suburban Survival Blog, https://suburbansurvivalblog.com/meshtastic-lora-the-off-grid-communication-lifeline-every-prepper-needs/
[^hr360]: Ham Radio 360, http://hamradio360.com/unveiling-the-meshtastic-revolution-a-preppers-ultimate-companion/
[^sf]: SpecFive, https://specfive.com/blogs/articles/the-best-radio-for-prepper
[^nst]: Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/

### Adoption under actual censorship

The strongest evidence that this is past hobbyist enthusiasm comes from the
download spikes. People living under a shutdown reach for these tools.

- **Madagascar**, September 2025: 70,000 Bitchat downloads in one week during protests
- **Nepal**, September 2025: nearly 50,000 in a single day during the protests that ended with the prime minister's resignation, fueled partly by a counterproductive social media ban
- **Uganda and Iran**, January 2026: Bitchat spikes during internet blackouts
- **Myanmar**: activists on Bridgefy BLE mesh working around military shutdowns
- **Spain and Portugal**, April 2025: Meshtastic proving itself through the Iberian blackout

### Rural mesh, where federal broadband did not arrive

The $42.5 billion Broadband Equity, Access, and Deployment program has drawn
criticism from every direction. By early 2025 only three states, Louisiana,
Delaware and Nevada, had final proposals approved, and no connections had been
completed at all. Senator John Thune called BEAD a sad story of government
inefficiency and progressivism run amok, arguing the previous administration had
loaded the program with requirements Congress never envisioned [^thune]. Eleven
of 13 Republican Senate Commerce Committee members signed a letter opposing the
NTIA fiber-only mandates. Under the 2025 administration, approvals were
rescinded and states required to reopen bidding, and Texas saw its allocation
cut from $3.3 billion to $1.3 billion. The Mackinac Center [^mac] and Citizens
Against Government Waste [^cagw] have both published on it.

Mesh is filling the gap without waiting. In rural Wyoming, a community of
ranchers built a 30-node Meshtastic network spanning over 2,000 square miles for
property monitoring and coordination. LoRa carries soil moisture monitoring,
cattle management, equipment status and worker coordination with no cellular
infrastructure anywhere, and NDSU Agriculture Extension has published a guide on
it [^ndsu].

[^thune]: Sen. John Thune, "Broadband Blunders Leave Americans Disconnected", https://www.thune.senate.gov/public/index.cfm/2024/9/broadband-blunders-leave-americans-disconnected
[^mac]: Mackinac Center for Public Policy, https://www.mackinac.org/blog/2024/more-trouble-ahead-for-government-funded-internet
[^cagw]: Citizens Against Government Waste, https://www.cagw.org/bead-broadband-or-bust/
[^ndsu]: NDSU Agriculture Extension, https://www.ndsu.edu/agriculture/extension/publications/basics-lora-technology-crop-and-livestock-management

### What still does not scale

The bottleneck is throughput. Multi-hop transmission cuts per-node throughput
hard as a network grows, and most existing solutions degrade past a few dozen
nodes. At 100 and above, hierarchical architectures with layered routing become
necessary, and the proposals on the table run to multi-tier designs with 3D
beamforming and heterogeneous relays to hold hop count constant per tier.

The social constraints are as real as the technical ones: spectrum regulation,
where LoRa ISM band power and duty-cycle limits vary by country; node density,
which is hardest to achieve exactly where a mesh would help most; and revenue
models that keep an open-source project alive.

---

## Parallel institutions

### Homeschool and microschool networks

Over 3.1 million U.S. students were homeschooled as of 2023, up from 2.5 million
in 2019, and microschools, learning pods and co-ops are expanding nationwide.
Heath Brown's research documents this as parallel politics, with local and
national associations filling social and political voids and producing
tight-knit, politically active communities [^par]. The Foundation for Economic
Education has written on how broad and bipartisan the rise has been [^fee].
These communities coordinate over email lists, private messaging groups and
co-op scheduling tools, and increasingly want platforms they control rather than
services that might moderate their educational or religious content.

### Homesteading and local food

Homesteading participation grew markedly through 2024 and 2025 on grocery costs
and supply chain anxiety, and simple practices can cut a grocery bill by up to a
quarter annually. Barter systems, seed-sharing networks and tool-lending
cooperatives all coordinate locally, outside the mainstream platforms.
MeshtBank shows how mesh infrastructure supports local economic activity with no
internet dependency at all.

### Churches

A church is very nearly the natural unit of community mesh networking:
geographically proximate, socially bonded, shared values, regular gathering.
When platforms suppress religious content the need for independent
communications stops being a convenience and becomes existential.

[^par]: Heath Brown, "Homeschooling the Right: How Conservative Education Activism Erodes the State", Journals of OpenEdition, https://journals.openedition.org/ejas/17223
[^fee]: Foundation for Economic Education, https://fee.org/articles/the-rise-of-homeschooling-was-broad-and-bipartisan/

---

## What this means for MUUDD and AutonomousTrust

These trends validate the premises we designed against, and they expose gaps in
the current mesh ecosystem that we are positioned to fill.

### The threat model is materializing

We built MUUDD for a world in which centralized infrastructure is a liability
rather than a convenience. The design predates the trend, and the world it
anticipated has arrived.

- **Grid failure**: Helene, the Iberian blackout, Texas winter storms, EMP exposure
- **Access control from above**: 296 shutdowns in 2024, net neutrality collapse, platform bans
- **Identity control from above**: age verification as stealth digital ID, eIDAS wallets, DOGE data centralization
- **Financial control from above**: CBDCs, debanking through Operation Choke Point, the Canadian convoy account freezes
- **Speech control from above**: the UN Cybercrime Treaty, DSA enforcement, state AG powers under KOSA
- **Surveillance from above**: FISA 702, corporate data harvesting, government database consolidation

### What the current ecosystem does not do

The ecosystem is fragmented. Meshtastic serves hardware hobbyists, Bitchat
serves protest messaging, Reticulum serves protocol purists, community meshes
serve local broadband. Each addresses a piece. None addresses the whole.

| Gap | The MUUDD/AT answer |
|---|---|
| No identity or reputation system | AT's cryptographic identity plus Byzantine reputation scoring |
| No defense against infiltration (Sybil attacks) | Consensus-based admission (PoW/PoS/PoA) |
| No governance framework | Kith-Covenant's distributed governance protocols |
| Scaling past ~100 nodes degrades | AT's hierarchical trust domains mapping to tiered routing |
| No secure fleet management | AT Phase 3: decentralized Paxos-based updates replacing SSH |
| Single-technology approaches | MUUDD spans physical (radio), transport (DTN), trust (AT) and social (Kith-Covenant) layers |
| Vulnerable to mandatory digital ID | AT's anonymous-first architecture, with behavioral reputation in place of government credentials |
| No off-grid economic exchange | AT's trust framework over mesh transport, which is what a MeshtBank-style system needs |
| Crisis tool only, never daily infrastructure | AT reputation plus Kith-Covenant governance, which is what makes a community network persist |

### AT is the missing trust layer

No current mesh project has an answer to the identity problem that the
age-verification laws exploit. AT's anonymous-first design earns trust through
behavior rather than conferring it on presentation of a government credential,
which counters the centralized identity infrastructure being built under
child-safety justification directly rather than obliquely. The four-level trust
gradient — refuse communications, communicate only, offer services, share data —
meters access finely with no external identity authority anywhere in it.

### The three pillars, answered

The free speech, free transactions, free movement triad maps onto our layers
without forcing.

1. **Communications sovereignty.** A mesh-routed user-owned physical layer, encrypted DTN transport, and AT trust evaluation together make communications that no ISP, platform or agency can switch off.
2. **Identity sovereignty.** AT's anonymous-first design with behavioral trust replaces government credentials, certificate authorities and age-verification infrastructure with peer-evaluated reputation.
3. **Financial sovereignty.** AT's trust framework over mesh transport is the infrastructure decentralized exchange needs, with no centralized intermediary in it.

### From crisis tool to daily infrastructure

Mesh adoption today is spike-driven, on blackouts and protests. AT's reputation
system and Kith-Covenant's governance are what could carry a community network
between the spikes, which is the whole distance between an emergency tool and
something people use on an ordinary Tuesday.

### From terrestrial to orbital

No other mesh project carries DTN integration or orbital-scale design. Our
architecture spans rural Appalachian hilltop relays through to asteroid-belt
inter-habitat links, on one trust and governance model at every scale.

### The community is the unit

What the preparedness community already knows is the thing the design turns on.
The individual is not the unit of survival. The community is. Meshtastic gives a
family a radio. MUUDD gives a community an infrastructure — communications,
trust evaluation, governance, and the ability to deal with other communities at
scale, from a rural county to an orbital habitat.

---

## Sources

- Freedom House, *Freedom on the Net 2025*, https://freedomhouse.org/report/freedom-net/2025/uncertain-future-global-internet
- Internet Society, "Internet Shutdowns Policy Brief", https://www.internetsociety.org/resources/policybriefs/2025/internet-shutdowns/
- Surfshark, "Internet Shutdowns 2025", https://surfshark.com/research/study/internet-shutdowns-2025
- Freedom of the Press Foundation, "Banning TikTok Enables Online Censorship", https://freedom.press/issues/tiktok-ban-weakens-first-amendment/
- DWT, "Sixth Circuit Invalidates FCC's 2024 Net Neutrality Order", https://www.dwt.com/blogs/broadband-advisor/2025/01/6th-circuit-fcc-net-neutrality-order
- Foundation for Defense of Democracies, "UN Cybercrime Treaty", https://www.fdd.org/analysis/2025/10/23/the-un-cybercrime-treaty-a-trojan-horse-for-suppressing-dissent/
- EFF, "UN Cybercrime Treaty Still Flawed", https://www.eff.org/deeplinks/2024/12/still-flawed-and-lacking-safeguards-un-cybercrime-treaty-goes-un-general-assembly
- Senate Commerce Committee, "Big Tech Weaponizes Terms of Service", https://www.commerce.senate.gov/2024/4/senate-commerce-investigation-reveals-how-big-tech-weaponizes-terms-of-service-to-silence-the-right
- National Catholic Register, "Big Tech's Big Bias", https://www.ncregister.com/news/big-tech-s-big-bias-religious-conservatives-face-scary-pattern-of-censorship
- NAPA Legal Institute, https://www.napalegalinstitute.org/post/how-big-tech-targets-faith-groups-for-censorship
- First Liberty Institute, https://firstliberty.org/news/big-tech-isnt-neutral/
- Fox Business, "Rumble Launches Effort to Combat Existential Threat", https://www.foxbusiness.com/politics/rumble-launches-effort-combat-existential-threat-video-platform-free-big-tech
- Alliance Defending Freedom, https://adflegal.org/press-release/adf-submits-comment-to-ftc-outlining-extensive-examples-of-big-tech-censorship/
- Heritage Foundation on CBDCs, https://www.heritage.org/markets-and-finance/commentary/are-congressional-republicans-about-greenlight-cbdc
- Cato Institute, "CBDC Spells Doom for Financial Privacy", https://www.cato.org/free-society/fall-2024/cbdc-spells-doom-financial-privacy
- Operation Choke Point 2.0, https://www.opchokepoint2.org/
- Hackaday, "Off-Grid, Small-Scale Payment System", https://hackaday.com/2025/12/05/off-grid-small-scale-payment-system/
- NPR, "Senate Passes FISA Reauthorization", https://www.npr.org/2024/04/20/1246076114/senate-passes-reauthorization-surveillance-program-fisa
- NPR, "DOGE Is Trying to Access All of It", https://www.npr.org/2025/03/11/nx-s1-5305054/doge-elon-musk-security-data-information-privacy
- MIT Technology Review, "DOGE's Tech Takeover", https://www.technologyreview.com/2025/04/14/1114988/doges-tech-takeover-threatens-the-safety-and-stability-of-our-critical-data/
- Heartland Emergency Preparedness, "Building a Community Meshtastic Network", https://heartlandemergencypreparedness.com/2025/08/25/building-a-community-meshtastic-network-step-by-step-guide-for-emergency-preparedness/
- Wikipedia, "2025 Iberian Peninsula Blackout", https://en.wikipedia.org/wiki/2025_Iberian_Peninsula_blackout
- MITRE, "Electromagnetic Pulse: The Dangerous but Overlooked Threat", https://www.mitre.org/news-insights/impact-story/electromagnetic-pulse-dangerous-overlooked-threat
- Texas Public Policy Foundation, "Texas Has Strengthened Its Grid, But It Is Still Vulnerable", https://www.texaspolicy.com/texas-has-strengthened-its-grid-but-it-is-still-vulnerable-to-the-next-winter-storm/
- Talking IoT, "Meshtastic at Embedded World 2026", https://talkingiot.io/meshtastic-at-embedded-world-2026-how-an-open-source-mesh-network-became-a-serious-business/
- Suburban Survival Blog, https://suburbansurvivalblog.com/meshtastic-lora-the-off-grid-communication-lifeline-every-prepper-needs/
- Ham Radio 360, http://hamradio360.com/unveiling-the-meshtastic-revolution-a-preppers-ultimate-companion/
- SpecFive, https://specfive.com/blogs/articles/the-best-radio-for-prepper
- Newstrail, "Wireless Mesh Network Market", https://www.newstrail.com/wireless-mesh-network-market/
- CNBC, "Jack Dorsey Launches WhatsApp Rival Built on Bluetooth", https://www.cnbc.com/2025/07/07/jack-dorsey-whatsapp-bluetooth.html
- Sen. John Thune, "Broadband Blunders Leave Americans Disconnected", https://www.thune.senate.gov/public/index.cfm/2024/9/broadband-blunders-leave-americans-disconnected
- Mackinac Center for Public Policy, "More Trouble Ahead for Government-Funded Internet", https://www.mackinac.org/blog/2024/more-trouble-ahead-for-government-funded-internet
- Citizens Against Government Waste, "BEAD: Broadband or Bust", https://www.cagw.org/bead-broadband-or-bust/
- NDSU Agriculture Extension, "Basics of LoRa Technology for Crop and Livestock Management", https://www.ndsu.edu/agriculture/extension/publications/basics-lora-technology-crop-and-livestock-management
- Heath Brown, "Homeschooling the Right", https://journals.openedition.org/ejas/17223
- Foundation for Economic Education, "The Rise of Homeschooling", https://fee.org/articles/the-rise-of-homeschooling-was-broad-and-bipartisan/

---

*Next: [Process architecture](../lib/autonomous_trust/doc/architecture/process-architecture.md)*
