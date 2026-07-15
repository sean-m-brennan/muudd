*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > [Architecture](README.md) > Protocol Components*

# Protocol Components and Operational Gaps

This document decomposes kith-covenant's software scope: what must be
built to make the social structures described in the research docs
actually runnable. It is the operational companion to the research set
and the working answer to §D "Operational Gaps" in
[skeptical_critique.md](../background/skeptical_critique.md).

> **MVP implementation status (2026-04-22).** Components **§1 Identity and
> standing**, **§2 Recognition ceremony**, and **§3 Archive / commons of
> memory** have an MVP Rust implementation in the `crates/` workspace
> (`kc-core`, `kc-identity`, `kc-ceremony`, `kc-archive`) with Python
> bindings (`kc-pyo3`). The `examples/two_communities_demo` binary and
> `src/python/examples/two_communities_demo.py` both exercise the full
> three-component stack end-to-end: build two communities with real
> Ed25519 keys, run each community's intra-rite, run the inter-community
> credentialing ceremony with mutual acceptance of the five anti-spiral
> feature claims, archive every record, verify, roundtrip through JSON,
> and re-verify. See the repo [README](../../README.md) for quickstart.
>
> **§6 Reputation lookup** also has a minimal slice implemented
> (2026-04-22): the `kc-reputation` crate provides standing lookup and
> multi-community aggregation backed by the archive. See `crates/kc-reputation/`.
>
> Components §§4, 5, 7, 8, 9, 10 (mediation, escalation, commons
> governance, jester, enforcement, test harness) are *not* implemented and
> remain as described below. MVP crypto is Ed25519-only; `Alg` is an enum
> whose shape preserves a future non-lattice PQC slot.
>
> **Capture-resistance amendment (2026-04-28).** Following
> [capture_resistance.md](../background/capture_resistance.md), capture
> resistance is introduced as a **cross-cutting architectural concern**
> rather than a single component. Each existing component (§§1-10)
> acquires capture-resistance responsibilities specified inline in the
> sections below, and a new §11 catalogues the unified architectural
> concern and the implementation work it implies. None of the
> capture-resistance responsibilities are implemented in the current
> MVP; their addition does not invalidate the existing
> three-component stack but does extend its eventual scope.

## Purpose and scope

Kith-covenant's software is the **inter-community protocol layer**. It
sits above [muudd](../../../../doc/muudd.md) (communications) and is
orthogonal to each community's internal
[autonomous_trust](../../../../lib/autonomous_trust/doc/autonomous_trust.md)
instance. It never reaches *into* a member community; it operates only
between communities and on compact-level artifacts.

```
  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
  │  community A │     │  community B │ ... │  community N │
  │    (AT)      │     │    (AT)      │     │    (AT)      │
  └──────┬───────┘     └──────┬───────┘     └──────┬───────┘
         │                    │                    │
         └────────────────┬───┴────────────────────┘
                          │
                ┌─────────┴─────────┐
                │   kith-covenant   │   ← this layer
                └─────────┬─────────┘
                          │
                ┌─────────┴─────────┐
                │      muudd        │
                └───────────────────┘
```

The software must realize the mechanisms the research set commits to:
recognition ceremony, archive, mediation, escalation, reputation
bridges, commons governance, the jester function, exit and succession.
Everything below is organized around those commitments.

## Component decomposition

### 1. Identity and standing

- **Community identity**: cryptographic keypair for the community as
  party, distinct from any individual's key. Must survive leadership
  change within the community; requires a "successor key" mechanism
  so internal rotation does not cost compact standing.
- **Designated speakers**: signed delegations from the community key
  authorizing specific persons to recite, file, or commit on the
  community's behalf. Revocable.
- **Witness registry**: identities of individuals qualified to
  witness recognition events, with rotation schedule and continuity
  constraints (per `recognition_protocols.md` "witness continuity").
- **Quantum-resistant primitives**: given the generational time
  horizons, identity keys should not rest on schemes vulnerable to
  medium-term cryptographic attack.
- **Cross-community principal-witness attestation channel**
  (capture-resistance amendment): a separate attestation chain
  for principals (human guardians of derived-substrate machine
  participants and, by extension, designated speakers for human
  members) carried by witnesses in *other* member communities than
  the principal's own. Cryptographically separable from the
  within-community attestation; supports the
  adversarial-principal validation specified in
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation)
  and
  [machine-participation.md "Guardian capture"](machine-participation.md#facet-3-validity-of-the-derivative-oath--succession-and-boundary-conditions).
- **Interpretive-plurality evidence at recognition**
  (capture-resistance amendment): the inter-community
  credentialing process records evidence of multiple traceable
  interpretive lineages within the candidate community, per
  [recognition_protocols.md "Inter-community layer"](../background/recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing)
  element 5 and
  [capture_resistance.md "Interpretive plurality"](../background/capture_resistance.md#1-interpretive-plurality-as-a-structural-requirement-on-members).

### 2. Recognition ceremony (two layers)

Per `recognition_protocols.md` "Two layers: intra-community
authorization and inter-community grammar," the recognition
mechanism is split. The software must support both layers and
must not conflate them.

- **Intra-community ceremony runner**: drives each community's own
  five-movement rite as performed on that community's envoys and
  representative machines, collects internal witness attestations,
  seals the intra-community record in the community's own terms. The
  community's own members are the witnesses here; the asymmetric-
  psychological-cost mechanism operates at this layer and requires
  bodily co-presence for the reasons given in `recognition_protocols.md`.
- **Inter-community credentialing runner**: accepts the sealed
  intra-community record as credential, drives the compact-level
  sequential presentation (each community performs its own rite in
  turn under cross-community witness), produces the anti-spiral-
  feature examination exchange (per `overview.md` §5b item (e) and
  §3b.i), and seals the cross-boundary witness record.
- **In-person / remote bridge**: intra-community rites require
  bodily co-presence (the cost mechanism demands it); cross-boundary
  witness may tolerate hybrid forms at interplanetary scale.
  Working design decision: intra-community authorization in person
  always; compact-formation cross-boundary witness in person when
  physically reachable; cross-boundary witness of intra-community
  generational re-affirmation cryptographically bound + video, with
  in-person confirmation at the nearest practical opportunity.
- **Recitation archive**: content-addressed, tamper-evident,
  publicly verifiable at both layers; each community's
  intra-community rite is sealed separately and referenced into the
  inter-community record that the compact holds. Referenced by
  reputation primitives (§6).
- **Re-affirmation scheduler**: tracks generational cadence per
  community; surfaces overdue or imminent intra-community
  re-affirmations; signals cross-boundary witnesses when a member
  community's re-affirmation event is scheduled.
- **Versioning**: Movement 4 content is each community's own and
  versioned per community; the inter-community layer tracks which
  version of each community's rite was witnessed at each admission
  and re-affirmation.
- **Ceremony-cost proxy tracking** (capture-resistance amendment):
  recorded proxies for asymmetric psychological cost across
  re-affirmation cycles: preparation time and attention,
  recitant articulation of what is being committed and forgone,
  witness reports of comportment, content of the
  anti-spiral-feature examination exchange, substantive vs. rote
  responses to challenge questions. Surfaces decay across cycles
  per [recognition_protocols.md "Tracking ceremony cost"](../background/recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles).
- **Substantive re-engagement rung** (capture-resistance amendment):
  a heavier re-affirmation cycle invoked when proxy decay is
  detected, with extended preparation, more demanding examination,
  and reasserted cost. This is a designated state between ordinary
  re-affirmation and anathema designation.

### 3. Archive / commons of memory

- **Distributed append-only archive** of compact-level events:
  recognitions, re-affirmations, mediations, designations,
  restorations, exits, reconstitutions.
- **Replication**: archive content replicated across member
  communities; no single community's failure loses the record.
- **Sealing**: threshold encryption for sealed mediation proceedings,
  with keys distributed across the mediator pool so no single custodian
  can unseal early. Deterministic reveal at the 25-30 year mark.
- **Access control**: public records vs. sealed records vs.
  restricted records (e.g., reputation details that cross the
  surveillance threshold).
- **Manufactured-event provenance tagging** (capture-resistance
  amendment): records advancing interpretations of high-charge
  events carry originator identity, declared incentives,
  information-history of the interpretation's emergence, and
  stake-disclosure entries; used by the structured-skepticism
  protocol per [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).
- **Decision-class metadata** (capture-resistance amendment):
  every compact-level decision record carries explicit
  classification (first-class, valid within light-lag; second-class,
  requiring multi-community quorum signature) per
  [commons_federation.md "Light-lag-honest decision classes"](../background/commons_federation.md#light-lag-honest-decision-classes).
  The metadata is part of the constitutional layer
  (genesis-immutable as a classification scheme; individual
  classifications modifiable only through second-class procedure).

### 4. Mediation

- **Mediator pool directory**: identities, rotations, recusals,
  lineage of training.
- **Case intake**: disputes filed between communities, classified
  by escalation-ladder rung.
- **Deliberation support**: async-compatible for light-lag; structured
  argument submission; evidence packaging; recusal and consent
  tracking.
- **Interim-injunction issuance**: protocol-level "stop" order with
  enforcement linkage (§9).
- **Decision publication**: public decisions with mandatory reasoning
  trail; sealed decisions with threshold-reveal.
- **Structured-skepticism protocol invocation** (capture-resistance
  amendment): mediator-pool authority to invoke the protocol on
  events meeting qualifying criteria (high interpretive charge,
  faction-aligned interest, sudden onset, originating from
  stake-holding witnesses). Imposes structural delay,
  multi-witness corroboration, cross-substrate translation, and
  stake-disclosure requirements. Mediator-pool internal review
  guards the invocation against pretextual use to suppress
  legitimate urgent matters. Specified in detail in
  [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).

### 5. Escalation and designation

- **Ladder state machine** per dispute: virtue-discussion → formal
  complaint → mediation → graduated sanctions → *hostis humani generis*
  designation. Timed transitions where the research specifies them.
- **Designation procedure**: four-condition test (per
  `bounded_conflict.md` §"Handling the *hostis humani generis*"),
  quorum rules, review period, emergency-variant handling.
- **Restitution tracking**: restoration obligations, verification of
  completion.
- **Re-entry procedure**: examination, witnessing, re-recitation
  binding on re-admitted party.
- **Substantive-re-engagement rung** (capture-resistance amendment):
  the new ladder rung between ordinary re-affirmation and anathema
  for ceremony-cost decay cases (per §2 above). Triggered by
  proxy-decay surfacing in the audit commons; applies to a member
  community without designating it as anathema; resolved by
  successful heavier re-affirmation cycle or by escalation to
  anathema if the heavier cycle also shows decay. State-machine
  position is: ordinary re-affirmation → substantive re-engagement
  → anathema designation.

### 6. Reputation bridges

- **Reputation ledger**: what is on the record across the compact:
  recitation history, completed mediations, designations (active and
  lifted), restoration completions.
- **Selective-disclosure primitives**: zero-knowledge proofs for
  narrow claims (e.g., "this community has no active designation
  against it") without exposing full history.
- **Decay model**: not pure exponential; step function for dramatic
  violations plus slow decay for minor. See
  `reputation_bridges.md` open question on half-life modeling.
- **Recitation history as foundational datum**: compact-anchored
  reputation root, portable across mediators and disputes.
- **Surveillance threshold**: configurable disclosure levels for
  internal health indicators; large/powerful communities disclose
  more by default.
- **Principal-output triangulation data flows**
  (capture-resistance amendment): comparison of a machine's
  conduct against the ensemble of similarly-situated machines
  under similarly-positioned principals. Divergence flags entered
  in the reputation ledger for cross-community review per
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation).
- **Ceremony-cost decay surfacing** (capture-resistance
  amendment): reputation queries surface proxy-decay state
  alongside conventional standing data, so peers and observers
  can see ceremony erosion before it has progressed to formal
  re-engagement or anathema.

### 7. Commons governance

- **Commons registry**: shared resources (knowledge archive, rescue
  capacity, communication relays, navigation beacons) with Ostrom-style
  boundary definitions.
- **Usage accounting**: consumption, contribution, free-rider
  detection.
- **Graduated-sanctions ledger**: progression from monitoring to
  warning to restriction to exclusion, with restoration paths.
- **Light-lag-tolerant consensus**: CRDT-like where semantics permit;
  BFT-style where they do not. Eventually-consistent governance is
  the baseline assumption (per `commons_federation.md`).
- **Nested-enterprise support**: hierarchical commons (community-level
  → regional → compact-wide) with appropriate authority at each level.
- **Decision-class enforcement** (capture-resistance amendment):
  cryptographic enforcement of the first-class / second-class
  distinction (per §3 metadata above). Second-class decisions
  produce signed records whose validation requires multi-party
  signature from a quorum spanning multiple communities at
  multiple distances; the quorum is tuned so no single AU-region's
  faction can satisfy it alone. Signatures must arrive within a
  defined latency window. Validation failure is surfaced
  automatically in the audit commons. The classification of which
  decisions fall into which class is a constitutional-layer
  commitment; reclassification is itself a second-class decision
  (the classification is self-protecting in this specific
  structural sense). Specified in detail in
  [commons_federation.md "Light-lag-honest decision classes"](../background/commons_federation.md#light-lag-honest-decision-classes).

### 8. Jester and observer function

> **Constitutional standing (capture-resistance amendment).** The
> jester / observer function is **promoted from cultural-dissent
> affordance to substrate-level capture detector**. Its permission
> to name what others cannot is what makes early-stage capture
> legible before terminal capture: contraction of substrate-deference,
> decay of ceremony cost, manufactured-event drift, principal-capture
> signatures. The function operates at the substrate layer (continuous
> monitoring of capture signatures) as well as the protocol layer
> (post-hoc reporting). Per
> [capture_resistance.md "Architectural unification"](../background/capture_resistance.md#architectural-unification),
> the function's constitutional position is upgraded; its
> implementation cannot be deferred to "after MVP."

- **Observer corps registry**: declared-perspective reporters,
  rotation, cross-community composition requirements to prevent
  narrative capture by the dominant member.
- **Report publication** with attribution (public reporting) alongside
  whistleblower channel with source protection.
- **Threshold-held source records**: whistleblower identity
  recoverable only with mediator-panel quorum and only for named
  cause. Source protection must be structurally stronger than
  individual mediator integrity.
- **Archive access**: observer reports form a durable check on the
  mediator pool; archive must make observer corpus retrievable across
  generations.
- **Capture-signature monitoring** (capture-resistance amendment):
  the observer corps maintains active monitoring for the legible
  signs of early-stage capture: substrate-deference erosion (shrinking
  compact-incapable list, shortening structured-fast period),
  ceremony-cost proxy decay across multiple member communities,
  manufactured-event patterns, principal-output triangulation
  divergence. Monitoring outputs are reported to the audit commons
  on a continuous cadence rather than only when specific reports are filed.
- **Substrate-deference reporting** (capture-resistance amendment):
  observer corps maintains the public dashboard tracking
  compact-incapable list state and structured-fast cadence
  observance, making the substrate-deference test's operational
  practice continuously legible to the compact's members.

### 9. Enforcement model

This is the load-bearing gap the research docs repeatedly punt on.
Three candidate mechanisms, layerable:

- **Reputation + directory consequences.** Ignored injunction →
  automatic restricted-member status until mediated. Software-enforced
  at the directory level: reputation lookups surface the violation; the
  injunction and its defiance are published to the archive
  automatically.
- **Capability-based access.** Compact-gated resources (rescue
  coordination endpoints, shared navigation, reputation-dependent
  trade, commons withdrawals) are revocable at protocol level when a
  community ignores an injunction. The teeth are withdrawal of
  protocol-level services, not external coercion.
- **Transparency-as-teeth.** Violations are published immediately and
  durably. The reputational and material cost is downstream of
  publication; the protocol's job is ensuring publication is reliable
  and unforgeable.

Kith-covenant is structurally sovereign-less (per `historical_cases.md`
pattern 3), so enforcement must be collective and non-sovereign. The
above three are the realistic options at the code layer. External
state coercion is not in scope.

**Substrate-deference test as ongoing constitutional operation**
(capture-resistance amendment). Beyond the post-hoc enforcement
responses above, enforcement now also encompasses the
**continuous demonstration of non-ultimacy** by the compact itself.
This is enforcement of the compact's own constitution against the
compact's own drift, performed by the compact's standing operation
rather than by an external force:

- **Compact-incapable list maintenance.** The constitutional list
  of decisions the compact will not make (member communities'
  internal theology and governance, content of recognition
  statements after compact-formation, truth-claims of any member's
  tradition, etc.) is maintained as a versioned, signed
  constitutional artifact. Modifications require second-class
  decision procedure; expansions are easier than contractions
  procedurally, reflecting the asymmetric capture risk.
- **Structured-fast cadence enforcement.** During defined periods
  (the compact's analogue of liturgical Lent / sabbatical *shmita*),
  compact-level mediations, sanctions, and decisions are not
  initiated. Life-critical commons proceed; the deliberative and
  adjudicative functions pause. The cadence is part of the
  constitutional layer; observer corps surfaces deviations.
- **Erosion-detection alerting.** The audit commons surfaces
  trends in either dimension (exceptions multiplying in the
  compact-incapable list, the structured-fast period contracting
  under pragmatic argument) to the observer corps and to the
  member communities.

The substrate-deference test is operational and diagnostic
(both functions are live in the test's continuous performance);
specified in
[justus_hostis.md "Non-ultimacy of the political community"](../background/justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient)
and developed in
[capture_resistance.md "Substrate-deference test"](../background/capture_resistance.md#2-the-substrate-deference-test-operationalizing-non-ultimacy).

### 10. Bootstrapping and test harness

- **Simulation environment** for adversarial testing: captured-mediator
  scenarios, hollow-recitation drift, coordinated collusion, narrative
  capture of the observer corps, protocol-layer replay attacks.
- **Earth-pilot configuration**: small-scale deployment mode that can
  actually run with real communities. Necessary for the test-cases gap
  (§D.5 in skeptical_critique).
- **Integration harness** against `autonomous_trust` for live-AT
  message traffic observation, per
  [policy-evaluation.md](policy-evaluation.md).
- **Falsification criteria**: what observable outcomes in pilot
  deployments would count as kith-covenant failing? This needs explicit
  design, not implicit.
- **Capture-resistance scenario simulation** (capture-resistance
  amendment): adversarial test categories specifically targeting
  the six strengthening moves: faction-monopolizing-interpretation
  within a member community (move 1); compact drift toward
  ultimacy via compact-incapable-list erosion (move 2); ceremony
  hollowing across simulated generational cycles (move 3);
  manufactured-event injection with timed loudest-interpreter
  pressure (move 4); light-lag fait-accompli attack with forged
  quorum signatures (move 5); principal-capture by external
  manipulation including AI-mediated influence operations (move
  6). Each scenario is run continuously in the simulation
  environment, with detection latency and damage bounds as the
  primary metrics.

### 11. Capture resistance (cross-cutting)

This is not a separate component but a unified architectural
concern that runs through §§1-10. The full statement is in
[capture_resistance.md](../background/capture_resistance.md);
the specifications for its individual moves live in their host
research documents. The role of this section is to give the
concern explicit constitutional standing in the software
decomposition, route the responsibilities to components, and name
the implementation work that follows.

**The unified architectural commitment.** Capture resistance is a
**continuous operating concern** of the substrate, the protocol,
and the implementation, woven through the compact's ordinary
practice rather than invoked only when capture is suspected. The
prior fragmentation (jester role for cultural dissent; audit
commons for forensic record; emergency anathema for crisis
response) is unified under this commitment, with each fragment's
constitutional position upgraded as specified above (§§1-10
inline).

**The six strengthening moves and their component homes.**

| Move | Focus | Primary component(s) | Specification |
|---|---|---|---|
| 1. Interpretive plurality | Member-substrate health | §1 (identity), §2 (recognition) | [recognition_protocols.md "Inter-community layer"](../background/recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing) item 5 |
| 2. Substrate-deference test | Compact-level non-ultimacy | §9 (enforcement), §8 (jester monitoring) | [justus_hostis.md "Non-ultimacy"](../background/justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient) feature 5 |
| 3. Ceremony cost tracking | Ceremony erosion detection | §2 (recognition), §3 (archive), §5 (escalation), §6 (reputation) | [recognition_protocols.md "Tracking ceremony cost"](../background/recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles) |
| 4. Manufactured-event resistance | Interpretation-dominance prevention | §3 (archive), §4 (mediation) | [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol) |
| 5. Light-lag-honest decision classes | Fait-accompli prevention | §3 (archive), §7 (commons) | [commons_federation.md "Light-lag-honest decision classes"](../background/commons_federation.md#light-lag-honest-decision-classes) |
| 6. Adversarial-principal validation | Principal-capture detection | §1 (identity), §6 (reputation) | [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation), [machine-participation.md "Guardian capture"](machine-participation.md#facet-3-validity-of-the-derivative-oath--succession-and-boundary-conditions) |

**Architectural layering.** Capture resistance operates at three
layers: substrate (member communities' own practices:
interpretive plurality, ceremony cost), compact (kith-covenant's
mechanisms: substrate-deference, manufactured-event resistance,
decision classes, jester-as-detector), and implementation
(software: audit commons as detection infrastructure,
cryptographic decision-class enforcement, attestation cadence
tuned to capture-detection windows). Each layer presupposes
the others; capture-resistance only works as a layered system.

**What this section adds to the implementation backlog.** The
following items become first-class implementation work,
priorities calibrated to the capture-resistance test harness:

- Cross-community principal-witness attestation channel (§1).
- Ceremony-cost proxy tracking and surfacing (§2, §3, §6).
- Substantive-re-engagement state machine rung (§5).
- Manufactured-event provenance tagging in archive (§3).
- Decision-class metadata and cryptographic enforcement (§3, §7).
- Structured-skepticism protocol invocation in mediation (§4).
- Substrate-deference dashboard maintained by observer corps (§8).
- Compact-incapable list as constitutional artifact (§9).
- Structured-fast cadence as protocol-enforced operating mode (§9).
- Capture-resistance scenario simulation in test harness (§10).

None of these is implemented in the current MVP. Their addition
does not invalidate the existing three-component stack but does
materially extend the eventual scope.

## Architectural decisions

1. **Distributed substrate.** Libp2p-style peer-to-peer with
   content-addressed storage. No distinguished server; no mechanism
   that becomes a single point of failure or a sovereign-in-disguise.
2. **Cryptographic identity.** Decentralized identifiers (DID-style)
   with community-level key hierarchies. Quantum-resistant primitives
   at the root layer.
3. **Consensus model.** Compact-wide facts (recognition, designation,
   re-entry) are rare but need durability; most mediations are local
   to a panel. Probably *not* a single global chain: more likely
   per-case chains anchored periodically to a slower, higher-threshold
   compact-level commit.
4. **Amendability tiers.** Constitutional layer (recognition structure,
   common virtues, substrate requirements) is genesis-immutable.
   Legislative layer (operational rules, mediator procedures,
   escalation specifics) is adaptable by mediator consensus. Direct
   mapping to code: different immutability regimes per record class.
5. **Integration seam with autonomous_trust.** AT exposes upward:
   community-key identity, delegated speakers, honest health
   indicators at the configured disclosure level, witness availability.
   Kith-covenant never reaches inward. See
   [policy-evaluation.md](policy-evaluation.md) for the current seam.
6. **Substrate-check at recognition, not enforcement.** The compact
   does not programmatically verify substrate claims; it verifies
   *recitation*. Substrate authenticity is surfaced over time through
   the asymmetric-psychological-cost mechanism (per
   `recognition_protocols.md`), not by code audit.
7. **Capture resistance is layered and cross-cutting.** Per
   [capture_resistance.md](../background/capture_resistance.md),
   capture resistance operates at the substrate layer (member
   communities' own practices), the protocol layer (kith-covenant's
   mechanisms), and the implementation layer (software). No single
   component implements capture resistance; each component carries
   responsibilities specified inline above (§§1-10), and §11
   catalogues the unified concern. The implementation must
   refuse the temptation to consolidate capture-resistance into a
   single subsystem: the consolidation itself becomes a capture
   target. Distribution of the concern across components is the
   structural defense.

## Suggested incremental build order

The minimum useful slice, in order. **Capture-resistance work is
woven through, not appended at the end**: items marked (CR) carry
capture-resistance responsibilities specified in the inline §§1-10
amendments and unified in §11.

1. **Identity + recognition + archive** (components 1, 2, 3). The
   compact does not exist until recognition is real and verifiable.
   Testable with two communities in a room. **(CR)** Identity
   includes the cross-community principal-witness channel (§1);
   recognition records the interpretive-plurality evidence (§2);
   archive carries decision-class metadata and provenance tags
   from the start (§3): adding these later is much harder than
   designing them in.
2. **Reputation lookup** (component 6, minimal slice). Once recognition
   events exist in the archive, downstream reputation claims become
   meaningful. **(CR)** Reputation surfaces ceremony-cost proxy
   state from the start; principal-output triangulation is a
   later layer.
3. **Integration seam with autonomous_trust** (§5 above). Enables a
   member community to operate its AT instance with kith-covenant
   presence.
4. **Mediation state machine + escalation ladder** (components 4, 5).
   Needed before disputes are filed. **(CR)** Mediation includes
   structured-skepticism protocol invocation; escalation ladder
   includes the substantive-re-engagement rung between ordinary
   re-affirmation and anathema.
5. **Enforcement model** (component 9). Should not be deferred
   further; decisions here shape 1-4 in retrospect. **(CR)**
   Enforcement includes the substrate-deference test as ongoing
   constitutional operation: compact-incapable list, structured-fast
   cadence, erosion-detection alerting. These are constitutional-layer
   commitments and must be specified before the compact is
   instantiated, not after.
6. **Commons registry** (component 7) and **observer function**
   (component 8). Layerable once the core protocol runs. **(CR)**
   Commons includes cryptographic decision-class enforcement
   (multi-party quorum signature for second-class decisions);
   observer function is promoted to substrate-level capture detector
   with the substrate-deference dashboard and capture-signature
   monitoring as standing competencies, not optional features.
7. **Capture-resistance test harness** (component 10, expanded). The
   six adversarial scenarios (faction-monopolizing-interpretation,
   compact drift toward ultimacy, ceremony hollowing, manufactured-event
   injection, light-lag fait-accompli, principal-capture) must
   exercise the implementation continuously from the earliest stages.
   This is **continuous, not sequenced**: the test harness exists
   from step 1 and accumulates scenarios as the components are built.

The capture-resistance work cannot be deferred to a "phase 2" because
several of its responsibilities (decision-class metadata, principal-witness
channel, audit-commons provenance schema, substrate-deference list as
genesis-immutable artifact) are constitutional-layer commitments that
must be present at compact-formation. Adding them later is either
impossible (genesis-immutability would be violated) or
operationally fraught (existing records would need migration that
itself becomes a capture surface).

## Open design questions

Several hard decisions remain unresolved and are flagged here rather
than buried in subsystem docs:

- **Enforcement model specifics.** Which of the three candidates in §9
  is primary? The research docs prefer transparency-as-teeth;
  capability-based access is the strongest at the code layer.
- **In-person requirement vs. interplanetary scale.** The hybrid
  proposal in §2 is working but unvalidated. The 3-20 minute
  Earth-Mars light-lag may be tolerable; multi-hour delays to outer
  system are not.
- **Re-affirmation cadence calibration.** The 25-30 year default lacks
  empirical support. May need to vary by community generational
  rhythm (monastic vs. agricultural vs. other) per
  `recognition_protocols.md` open questions.
- **Mediator training pipeline.** Who trains the first generation?
  Is kith-covenant responsible for competence certification, or does
  each member community supply its own mediator candidates to a
  shared assessment process?
- **Minimum viable compact size.** Below some threshold, rotation and
  recusal become infeasible (every mediator is conflicted). Above
  some threshold, coordination fails. Where is the band?
- **Integration with existing infrastructure.** DIDs, IPFS, libp2p,
  threshold-cryptography libraries, CRDT libraries: which stacks are
  mature enough to build on, which need to be built or forked?

## Relationship to other documents

- [policy-evaluation.md](policy-evaluation.md): existing architecture
  doc covering the AT/kith-covenant evaluation seam; the integration
  point for §5 above.
- [../research/](../research/): the research set this document
  operationalizes. Each component above corresponds to one or more
  research docs' design commitments.
- [../kith_covenant.md](../kith_covenant.md): parent index.
- [../background/skeptical_critique.md](../background/skeptical_critique.md)
  §D: the "Operational Gaps" section that flagged these items; this
  document is the working response to that section.
