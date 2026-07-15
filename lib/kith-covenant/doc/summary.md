*[Sustainable Space](../../../../../doc/sustainable_space.md) > [MUUDD](../../../doc/muudd.md) > [Kith-Covenant](kith_covenant.md) > Summary*

# Kith-Covenant in Plain Terms

## The problem, and why it's ancient

Communities that don't share beliefs still need to work together. How to bind them durably is the oldest governance problem on record. The earliest surviving parity treaty between two foreign great powers (Hittite and Egyptian, 1259 BCE) already has the shape: how do you make an agreement hold between sides with different gods, different languages, different loyalties? Every civilization since has taken a run at it. Commerce across religious lines (medieval merchant law), maritime rescue (Rhodian sea law, used across four civilizations for a thousand years), wounded-soldier treatment (Red Cross / Red Crescent), ambassador protection: these domains have worked cross-culturally.

Full-scale arrangements (ones governing broad slices of community life, not just one function) have a worse record. They fail in two distinct ways. Without a shared foundation, oaths don't bind; short-term interest overrides them. *With* a shared foundation that becomes the arrangement's own object of devotion, the system collapses into loyalty cascades ("you're not pure enough"), ending in purges and mass violence. Robespierre's Terror, the Cultural Revolution, Stalinist purges, Salem: these are not accidents; they are the predictable failure mode of virtue-oriented communities that lack specific structural brakes.

## How kith-covenant handles it

Kith-covenant (KC) is a protocol for communities that must cooperate without pretending to share beliefs. Two rules: **each community anchors to whatever it holds above itself** (the compact stays neutral on content), and **the compact itself never becomes the ultimate**. A community qualifies for full membership only if its own tradition carries five features that historically block the loyalty-cascade failure mode: members can fail, including the virtuous ones; virtue is received rather than earned; victim-selection as communal hygiene is refused; moral obligation extends to strangers; and the political community is not the final thing. These features are present across mature religious traditions and absent in the ideological systems (Jacobin, Maoist, pure-tribalist) that historically produced purges.

## Technical specifics

KC runs in **two layers**.

- **Intra-community layer**: each community performs its own authorization rite on the envoys and representative machines it sends out. Psychological-commitment mechanisms operate here: speaking a commitment in front of one's own community is costly, and it works in that community's own language and tradition.
- **Inter-community layer**: thin procedural grammar. Five components: (a) a shared treaty-form for entry and periodic reaffirmation; (b) credentials carried by each community's envoys; (c) vouching by existing members for new entrants; (d) an audit commons recording conduct over time; (e) demonstrable evidence that the candidate's practice actually exhibits the five anti-cascade features.

KC sits on top of **autonomous-trust (AT)**. AT handles anonymous peer-reputation inside a single community; KC handles *named* humans across communities whose personal standing is at stake in what their envoys commit. Machines participate only as delegates of specific named human guardians: there is no machine-native legitimacy and no AI-to-AI cooperation without humans on both ends.

Life-support, rescue, orbital-debris, and communications are governed as **thin-grammar commons** in the Geneva Conventions style: cooperation is required across the compact's boundary, because the cost of defection is physical, immediate, and legible to everyone.

## Why this configuration

KC inherits from what historically worked: Kadesh-style each-side-swears-by-its-own-witnesses, merchant-law credentialing, Geneva-style thin protective frameworks. It refuses what hasn't: joint creedal ceremonies across different substrates (that's an intra-community mechanism being applied at the wrong layer), and virtue-oriented cooperation without structural backstops against the loyalty cascade.

## Where to go from here

- [`overview.md`](overview.md): full standalone argument with citations
- [`kith_covenant.md`](kith_covenant.md): map of the research and architecture set
- [`background/historical_cases.md`](background/historical_cases.md): the historical evidence
- [`background/justus_hostis.md`](background/justus_hostis.md): theoretical anchor
- [`architecture/`](architecture/): software architecture
