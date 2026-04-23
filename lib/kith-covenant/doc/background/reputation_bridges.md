*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > Reputation Bridges*

# Reputation Bridges: Interface Between Autonomous Trust and Kith-Covenant

## Thesis

Kith-covenant begins where autonomous_trust ends. But the boundary
between the intra-community trust system (AT) and the inter-community
cooperation protocol (kith-covenant) is not a wall. Some information
about a community's internal conduct must cross the boundary for the
compact to function: a community's good standing in the compact depends
in part on its internal health, and a community's internal trust
calculations must account for its external compact obligations. The
question is not *whether* information crosses, but *what*, *how*, and
*at whose discretion*.

The medieval *lex mercatoria* solved this problem for commerce. A
merchant's reputation, built in one jurisdiction, traveled with him
to other jurisdictions through the network of fairs, Kontore, and
consular courts. The reputation was non-state (no sovereign issued it),
cross-jurisdictional (it was recognized wherever the lex mercatoria
operated), and carried by the network itself (the infrastructure of
fairs and trading posts was the medium through which reputation
propagated). The system worked for centuries — and then failed, not
because the reputation mechanism broke but because the infrastructure
it traveled on was absorbed by states.

Kith-covenant's reputation bridge must solve the same problem under
harder constraints: across communities more different from each other
than medieval merchants were (merchants shared a trade and a substrate;
compact members share only the substrate), under the privacy
requirements that the autonomous_trust architecture exists to protect,
across AU-distance light-lag that makes real-time reputation
impossible, and with the cryptographic mechanisms that make selective
disclosure possible where medieval merchants had only word-of-mouth.

The central design question is not "what reputation data should cross?"
but **"what reputation data *can* cross without destroying the
autonomy that autonomous_trust exists to protect?"**

---

## The *lex mercatoria* as model

### What made it work

The medieval law merchant ([historical_cases.md](historical_cases.md)
§"Lex mercatoria") succeeded because four structural features
coincided:

1. **Small-world network.** The number of merchants who participated
   in long-distance trade was small enough that reputation could
   propagate through personal acquaintance. A merchant cheating in
   Bruges would be known in Venice within a few trading seasons,
   because the same people (or their apprentices) staffed both ends
   of the trade routes.

2. **Repeated interaction.** The fair cycle was annual; merchants
   returned to the same fairs year after year. The shadow of the
   future was long and *visible* — everyone knew they would meet again.

3. **Shared substrate.** The merchants were, with few exceptions,
   Christians operating within a shared oath-framework. An oath sworn
   before God at a pie-powder court was not merely a legal formality;
   it was a commitment within a framework that both parties
   acknowledged as ultimate.

4. **Institutional infrastructure.** The Kontore, the fair courts, and
   the consular courts provided the physical and procedural
   infrastructure through which reputation was recorded, transmitted,
   and adjudicated. Reputation traveled on this infrastructure the
   way water flows through an irrigation system.

### What broke it

The lex mercatoria's reputation infrastructure did not fail from
within. It was absorbed by the state. As national courts expanded
their jurisdiction (14th–16th centuries), the separate merchant-law
jurisdiction was folded into the common law or civil law. The
reputation that had been carried by the merchant network was
replaced by the credit that was carried by state-backed institutions
(banks, registries, courts of law).

The absorption was not forcible; it was *convenient*. State-backed
institutions offered more reliable enforcement, wider jurisdiction,
and integration with other state services. Merchants migrated to the
state-backed system voluntarily — and in doing so, surrendered the
non-state character of their reputation infrastructure.

### The modern analogue question

The historical question from
[historical_cases.md](historical_cases.md): "In a world where most
infrastructure is captured by state actors, what is reputation
portable on?"

The lex mercatoria's reputation was portable on a *network* — the
network of merchants, fairs, and Kontore. When the network was
absorbed by the state, portability was lost. Kith-covenant's
reputation must be portable on a network that *cannot be absorbed*
— which means the network must be decentralized, cryptographically
secured, and not dependent on any single community's infrastructure.

This is, in essence, the design problem that autonomous_trust solves
at the intra-community level. AT's gossip protocol propagates
reputation through a decentralized peer-to-peer network that no
single node controls. The reputation bridge is the extension of
this principle to the inter-community level: reputation data that
crosses the AT/kith-covenant boundary is carried by a decentralized
inter-community network, cryptographically secured, and not dependent
on any single community's infrastructure.

---

## What crosses the boundary

Not all internal reputation data should cross the AT/kith-covenant
boundary. The boundary exists to protect the autonomy that
autonomous_trust is designed to serve. A community's internal trust
calculations — who trusts whom within the community, what reputation
scores individuals have earned, what internal disputes are underway —
are AT's scope, not kith-covenant's.

What *does* cross falls into four categories:

### Category A: Compact-facing commitments (public, always crosses)

The recognition statement and its subsequent behavioral record. This
is the foundation of inter-community reputation: a community made the
five-movement recognition commitment, and its behavior since then
either confirms or undermines that commitment. The recognition-ceremony
record (cryptographically signed by the recitant, witnesses, and the
compact) is the founding datum. Subsequent compact-level behavior —
participation in deliberation, adherence to the common virtues,
response to mediation, treatment of shared commons — is the ongoing
record.

This data is public by design. The recognition ceremony is a public
act; the compact's proceedings are public acts. No privacy concern
arises because the data is already public.

### Category B: Aggregated internal health indicators (community-level, crosses with consent)

Community-level summaries that reveal vitality without exposing
individual data. Examples:

- **Population stability.** Is the community's population growing,
  stable, or declining? A community in demographic collapse is a
  community whose compact commitments may become unreliable — not
  because of bad faith but because of diminishing capacity.
- **Resource adequacy.** Is the community able to meet its commons
  contributions? A community under resource stress may need
  assistance rather than sanction.
- **Dispute-resolution activity.** Not the content of internal
  disputes, but the *rate* — a proxy for internal health. A community
  with an abnormally high internal dispute rate may be experiencing
  governance stress that could spill over into inter-community
  conduct.
- **Generational transmission.** Is the community maintaining its
  recognition-ceremony re-affirmation cadence? A community that
  skips a generational re-affirmation is a community whose
  commitment may be hollowing.

These indicators are *aggregated* — they reveal community-level
patterns without exposing individual data. They cross the boundary
with the community's consent (the community chooses what to report
and at what level of aggregation) but the compact may require
certain minimum disclosures as a condition of membership. The
tension between privacy and transparency is real and not fully
resolvable; the design must specify the minimum disclosure that
makes compact governance viable.

### Category C: Specific violation reports (crosses when external effects occur)

When a community's internal conduct produces external effects —
harm to other communities, violation of commons obligations,
behavior that triggers the escalation ladder — specific information
about the violation crosses the boundary. This is not surveillance;
it is complaint-driven. The information crosses because another
community has been harmed and has standing to raise the issue through
the compact's mediation mechanisms.

The whistleblower case — an individual within a community reporting
conduct that the community's leadership has not acknowledged — is the
hardest case. This is developed in
[jester_and_dissent.md](jester_and_dissent.md); the reputation-bridge
implication is that the cryptographic mechanisms for anonymous
attestation (see §"Cryptographic mechanisms" below) must be available
for this purpose.

### Category D: Everything else (does not cross)

Individual reputation scores, internal trust relationships, details
of internal disputes, personal data about community members, internal
governance procedures, internal religious practices, internal
cultural norms — none of this crosses the boundary unless it falls
into categories A–C. The autonomous_trust architecture exists
precisely to protect this data; the reputation bridge must not
undermine that protection.

---

## Costly signaling across cultural differences

The open question from
[tacit_coordination.md](tacit_coordination.md): "Costly signaling
assumes that costliness is *legible to all parties*. Across deeply
different cultures, what is costly to one party may be cheap to
another. How does kith-covenant design costly signals that are
credibly costly across the cultural diversity of its membership?"

Three types of signal appear to be universally costly — costly
regardless of cultural context:

### 1. Physical resource commitment

Dedicating material resources to the compact's shared infrastructure —
contributing to the maintenance of communication relays, staffing
rescue stations, sharing life-support capacity in emergencies — is
costly to any community regardless of its cultural values. Resources
are finite; dedicating them to the compact means not using them for
internal purposes. The costliness is *physical* and therefore
culturally universal.

**Proposal:** Physical resource commitment to shared infrastructure is
the baseline costly signal in kith-covenant. A community that
consistently contributes to the compact's commons (communication,
navigation, rescue) demonstrates costly commitment that is legible
to all other communities regardless of cultural difference.

### 2. Time and presence commitment

Participation in compact ceremonies, mediation service, mediator
training, and deliberative processes requires time — time that the
participating community members could spend on internal affairs. Time
commitment is culturally variable (some cultures value leisure, others
value industry), but it is broadly legible as a cost: the person is
*here*, serving the compact, rather than *there*, serving their
community.

**Proposal:** Participation cadence — the frequency and quality of a
community's engagement with compact processes — is a costly signal.
A community that sends its best people to mediation service, that
participates actively in deliberation, that shows up for recognition
ceremonies, is signaling costly commitment through time and presence.

### 3. Knowledge sharing

Opening archives, sharing technical knowledge, teaching craft — is
costly in any tradition that values its heritage. Knowledge is a
competitive advantage; sharing it reduces that advantage. The
costliness is informational and universally legible: the community
is giving away something valuable.

**Proposal:** Contribution to the knowledge commons is a costly
signal. A community that actively contributes to the shared archive —
scientific data, engineering records, cultural works — demonstrates
costly commitment to the compact's "inheritance of the future" virtue.

### Cross-cultural vocabulary

The three types of costly signal — resources, time, knowledge — form
a **cross-cultural vocabulary** for reputation. A community's
reputation in the compact is built through consistent costly signaling
across all three types. The specific form of the signal may vary
(one community contributes rescue capacity; another contributes
navigational data; a third contributes mediator training), but the
costliness is universally legible.

---

## Cryptographic mechanisms for selective disclosure

The privacy/transparency tension in the reputation bridge can be
partially resolved by cryptographic mechanisms that allow selective
disclosure — revealing *properties* of internal state without
revealing the state itself.

### Zero-knowledge proofs

A community can prove a property of its internal state without
revealing the state. Example: "Our internal dispute-resolution rate
is below threshold X" can be proven without revealing what the
disputes are or who the parties are. Zero-knowledge proofs are
computationally expensive but mathematically sound; they are the
strongest privacy-preserving mechanism available.

**Application to reputation bridges:** Category B indicators
(aggregated internal health) can be transmitted as zero-knowledge
proofs rather than raw data. The compact receives verified claims
about community health without accessing the underlying data.

### Commitment schemes

A community can publicly commit to a claim (by publishing a
cryptographic hash of the claim) and reveal the claim later when
conditions warrant. This creates a verifiable record of prior
assertions: if a community claims it is in good health and later
turns out to have been concealing a crisis, the commitment scheme
provides evidence of the misrepresentation.

**Application to reputation bridges:** Communities can make periodic
commitments about their internal health indicators. If the indicators
later prove false (because the community's conduct reveals the
falsity), the commitment record becomes evidence in the compact's
mediation process.

### Ring signatures and anonymous attestation

An individual within a community can make an attestation that is
verifiably from *some* member of the community without revealing
*which* member. This is the cryptographic substrate for the
whistleblower function developed in
[jester_and_dissent.md](jester_and_dissent.md).

**Application to reputation bridges:** Whistleblower reports — claims
that a community's internal conduct violates common virtues — can be
cryptographically attributed to the community (verifying that the
report comes from an insider) without identifying the reporter. The
liminal institution that receives the report
([liminal_institutions.md](liminal_institutions.md)) can assess its
credibility without exposing the reporter to retaliation.

---

## Reputation under light-lag

At AU distances, reputation data is always stale at any non-local
point. The distributed-monitoring model from
[commons_federation.md](commons_federation.md) applies: reputation
propagates through the compact's communication network on a delayed
basis, achieving eventual consistency rather than real-time
consistency.

### Reputation half-life

A concept specific to kith-covenant: the **reputation half-life** is
the period after which reputation data is no longer actionable without
refreshing. A community's reputation built over years of consistent
costly signaling has a long half-life — it takes a long time to decay.
A community's reputation built recently or based on a single dramatic
event has a short half-life — it decays quickly without reinforcement.

The half-life concept serves two purposes:

1. **It prevents stale data from driving decisions.** A community's
   reputation from ten years ago may not reflect its current state.
   The compact's governance mechanisms should weight recent data
   more heavily than old data, with the weighting function calibrated
   to the type of data (compact-facing behavior decays slowly;
   aggregated health indicators decay faster).

2. **It rewards sustained commitment.** A community that has
   consistently signaled costly commitment for decades has a deeper
   reputation reserve than one that has signaled for months. This
   creates a structural incentive for long-term commitment — exactly
   the "shadow of the future" that
   [tacit_coordination.md](tacit_coordination.md) identifies as the
   cooperation-sustaining mechanism.

### Stale-data scenarios

When communication is delayed by hours (AU distances), a community's
reputation as known at a distant point may be significantly out of
date. The most dangerous stale-data scenario: a community has been
designated anathema, but the designation has not yet
propagated to all compact members. A community that is unaware of
the designation may continue to cooperate with the designated
community, undermining the compact's collective response.

The mitigation: designation decisions are propagated with highest
priority on the compact's communication network, and the interim-
injunction mechanism ([liminal_institutions.md](liminal_institutions.md))
provides rapid local response while the full decision propagates.

---

## The portability problem

When a community exits a compact
([exit_and_succession.md](exit_and_succession.md)), what reputation
comes with it? When a community joins a new compact, what reputation
precedes it?

### Reputation upon exit

A departing community's compact-facing record (Category A) is a
permanent record — it is part of the compact's archive and cannot be
erased by departure. The departing community takes with it the
*fact* of its record, but the record itself remains in the compact's
custody (the knowledge-commons principle).

The danger: **reputation laundering**. A community with a bad record
exits the compact specifically to shed that record, then seeks
admission to a new compact without disclosing its history. The
mitigation: the compact's public archive (maintained by its liminal
institutions) is accessible to other compacts. A community seeking
admission to a new compact can be asked to authorize the new compact
to review its previous compact's records. Refusal to authorize such
review is itself a signal.

### Reputation upon entry

A new community seeking admission to a compact has no compact-facing
record. The recognition ceremony
([recognition_protocols.md](recognition_protocols.md)) serves as the
initial reputation event — the community stakes its reputation on the
five-movement commitment, and the witnesses assess sincerity. Beyond
the ceremony, the new community enters a **probationary period**
during which its compact-facing behavior is closely observed and its
reputation builds from zero.

The duration of the probationary period should be calibrated to the
generational re-affirmation cadence — roughly 25–30 years, long
enough for the community to demonstrate sustained commitment through
at least one generation of leadership transition.

### Reputation capture

The inverse of reputation laundering: a community is *trapped* in
a compact by the threat of reputation loss. If exit means losing the
reputation built over decades of costly signaling, then the exit
cost may be prohibitively high — making the compact a prison
disguised as a cooperative.

The mitigation: the community's compact-facing record is a factual
record of its behavior, not a reward that the compact grants or
withholds. A community that exits in good standing (voluntary
withdrawal per the proper procedure) retains its record as a
positive signal to future compacts. Only a community that exits
under designation (expulsion) or that exits to evade designation
loses the reputational benefit of its record.

---

## Reputation as commons

The aggregate reputation data of the compact — the collective record
of all communities' compact-facing behavior — is itself a commons.
It is non-excludable (all compact members can access it) and
depletable by pollution (false reputation, strategic
reputation-poisoning, slander).

**False reputation** — the deliberate inflation or deflation of a
community's reputation through fabricated reports, strategic
misrepresentation, or coordinated smear campaigns — is the
pollution threat. The commons-governance framework from
[commons_federation.md](commons_federation.md) applies: monitoring
(the mediator pool detects reputation fraud), graduated sanctions
(false reputation is a virtue violation subject to the escalation
ladder), and conflict resolution (reputation disputes are adjudicated
by the liminal institutions).

The specific form of reputation fraud to watch for:

- **Strategic inflation:** a community inflates its costly-signaling
  record by reporting contributions it did not make or inflating
  the value of contributions it did make.
- **Strategic deflation:** a community deflates another community's
  reputation through fabricated complaints or distorted accounts of
  the other community's behavior.
- **Coalition fraud:** multiple communities coordinate to inflate
  each other's reputations or deflate a rival's — the
  reputation-commons equivalent of a price-fixing cartel.

The detection mechanism for all three: cross-verification through
the mediator pool, which has access to the compact's records and
can compare a community's claims against the independently
maintained record.

---

## Implementation interface

The reputation bridge has a software counterpart in the
autonomous_trust evaluation architecture. The current interface
(as documented in the system-simulation roadmap) includes:

- A **KithCovenantObserver** (AT `Process` plugin) that subscribes to
  reputation and negotiation event streams within AT and produces
  compact-facing summaries.
- A **KithCovenantPolicy** data class representing the compact's
  behavioral rules as they interface with AT's internal operations.
- A **KithCovenantEvaluator** test harness that scores policy
  effectiveness against a fitness function.

The reputation bridge extends this interface. The observer produces
the Category B aggregated indicators (population stability, resource
adequacy, dispute-resolution rate, generational-transmission health).
The policy includes the disclosure rules (what minimum disclosures
are required for compact membership). The evaluator tests whether the
disclosure level is sufficient for compact governance without being
so invasive as to undermine autonomy.

The implementation detail is deferred to the software layer; this
document establishes the conceptual architecture that the software
must instantiate.

---

## Open questions

- Can a community's **recognition-statement recitation history** serve
  as the foundational reputation datum — the "credit record" equivalent
  that starts the reputation bridge? The recitation record is public,
  cryptographically signed, and verifiable; it may be the natural
  foundation on which all other reputation data builds.

- What is the **threshold** at which aggregated internal health
  indicators cross from "useful compact information" to "surveillance
  apparatus"? Is the threshold the same for all communities, or does
  it vary with community size and power? (A large, powerful community
  disclosing its health indicators is reassuring; a small, vulnerable
  community disclosing the same may be exposing itself to coercion.)

- How does the compact handle **reputation information about non-member
  communities it encounters**? The four-category framework applies:
  non-members encountered have no compact-facing record, but the
  compact may have observational data. Is it legitimate to build a
  reputation profile of a community without its consent, and if so,
  what constraints apply?

- The **zero-knowledge proof** mechanism is computationally expensive.
  At what scale does the computation become prohibitive, and are there
  lighter-weight privacy-preserving mechanisms that are sufficient
  for most purposes? (Aggregation may be sufficient for most
  indicators, with ZK proofs reserved for disputed claims.)

- Is **reputation half-life** the right metaphor? Exponential decay
  assumes a constant rate of information aging, but reputation may
  age non-linearly — a single dramatic violation may destroy decades
  of accumulated reputation in an instant. What is the right decay
  model?

## Relationship to other documents

- [historical_cases.md](historical_cases.md) — the lex mercatoria as
  the primary historical model for non-state cross-jurisdictional
  reputation.
- [tacit_coordination.md](tacit_coordination.md) — costly signaling
  across cultures; reputation as a costly signal that travels across
  community boundaries.
- [recognition_protocols.md](recognition_protocols.md) — the
  recognition statement as the foundational reputation event; the
  four categories as reputation boundaries.
- [liminal_institutions.md](liminal_institutions.md) — liminal
  institutions as reputation arbiters and custodians of the compact's
  reputation commons.
- [commons_federation.md](commons_federation.md) — reputation data
  as a commons; commons-governance mechanisms applied to reputation
  fraud.
- [bounded_conflict.md](bounded_conflict.md) — the escalation ladder
  as the enforcement mechanism for reputation violations.
- [justus_hostis.md](justus_hostis.md) — the fallen-actor
  anthropology: reputation fraud is a predictable behavior of fallen
  actors, not an edge case.
- [noticeable_hand.md](noticeable_hand.md) — the sacred substrate as
  the oath-framework that makes reputation commitments binding.
- [exit_and_succession.md](exit_and_succession.md) —
  reputation portability upon exit.
- [jester_and_dissent.md](jester_and_dissent.md) — the
  whistleblower function and anonymous attestation.
- [../social_framework.md](../social_framework.md) — parent document.
- [../../../../autonomous_trust/doc/autonomous_trust.md](../../../../autonomous_trust/doc/autonomous_trust.md)
  — the AT architecture that the reputation bridge interfaces with.

## References

*To be added — anchor works include Milgrom, North, and Weingast,*
"The Role of Institutions in the Revival of Trade" *(1990) on the lex
mercatoria's reputation mechanism; Greif,* Institutions and the Path
to the Modern Economy *(2006) on medieval commercial reputation;
Goldsmith and Wu,* Who Controls the Internet? *(2006) on
infrastructure capture; Narayanan et al.,* Bitcoin and Cryptocurrency
Technologies *(2016) on cryptographic reputation mechanisms; Chaum,*
"Security Without Identification" *(1985) on anonymous credentials;
Ostrom,* Governing the Commons *(1990) on commons monitoring; and
the autonomous_trust documentation on gossip protocols, Byzantine
reputation scoring, and the trust-level gradient.*
