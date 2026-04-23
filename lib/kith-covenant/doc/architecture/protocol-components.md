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
> three-component stack end-to-end — build two communities with real
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

- **Community identity** — cryptographic keypair for the community as
  party, distinct from any individual's key. Must survive leadership
  change within the community; requires a "successor key" mechanism
  so internal rotation does not cost compact standing.
- **Designated speakers** — signed delegations from the community key
  authorizing specific persons to recite, file, or commit on the
  community's behalf. Revocable.
- **Witness registry** — identities of individuals qualified to
  witness recognition events, with rotation schedule and continuity
  constraints (per `recognition_protocols.md` "witness continuity").
- **Quantum-resistant primitives** — given the generational time
  horizons, identity keys should not rest on schemes vulnerable to
  medium-term cryptographic attack.

### 2. Recognition ceremony (two layers)

Per `recognition_protocols.md` "Two layers: intra-community
authorization and inter-community grammar," the recognition
mechanism is split. The software must support both layers and
must not conflate them.

- **Intra-community ceremony runner** — drives each community's own
  five-movement rite as performed on that community's envoys and
  representative machines, collects internal witness attestations,
  seals the intra-community record in the community's own terms. The
  community's own members are the witnesses here; the asymmetric-
  psychological-cost mechanism operates at this layer and requires
  bodily co-presence for the reasons given in `recognition_protocols.md`.
- **Inter-community credentialing runner** — accepts the sealed
  intra-community record as credential, drives the compact-level
  sequential presentation (each community performs its own rite in
  turn under cross-community witness), produces the anti-spiral-
  feature examination exchange (per `overview.md` §5b item (e) and
  §3b.i), and seals the cross-boundary witness record.
- **In-person / remote bridge** — intra-community rites require
  bodily co-presence (the cost mechanism demands it); cross-boundary
  witness may tolerate hybrid forms at interplanetary scale.
  Working design decision: intra-community authorization in person
  always; compact-formation cross-boundary witness in person when
  physically reachable; cross-boundary witness of intra-community
  generational re-affirmation cryptographically bound + video, with
  in-person confirmation at the nearest practical opportunity.
- **Recitation archive** — content-addressed, tamper-evident,
  publicly verifiable at both layers; each community's
  intra-community rite is sealed separately and referenced into the
  inter-community record that the compact holds. Referenced by
  reputation primitives (§6).
- **Re-affirmation scheduler** — tracks generational cadence per
  community; surfaces overdue or imminent intra-community
  re-affirmations; signals cross-boundary witnesses when a member
  community's re-affirmation event is scheduled.
- **Versioning** — Movement 4 content is each community's own and
  versioned per community; the inter-community layer tracks which
  version of each community's rite was witnessed at each admission
  and re-affirmation.

### 3. Archive / commons of memory

- **Distributed append-only archive** of compact-level events:
  recognitions, re-affirmations, mediations, designations,
  restorations, exits, reconstitutions.
- **Replication** — archive content replicated across member
  communities; no single community's failure loses the record.
- **Sealing** — threshold encryption for sealed mediation proceedings,
  with keys distributed across the mediator pool so no single custodian
  can unseal early. Deterministic reveal at the 25–30 year mark.
- **Access control** — public records vs. sealed records vs.
  restricted records (e.g., reputation details that cross the
  surveillance threshold).

### 4. Mediation

- **Mediator pool directory** — identities, rotations, recusals,
  lineage of training.
- **Case intake** — disputes filed between communities, classified
  by escalation-ladder rung.
- **Deliberation support** — async-compatible for light-lag; structured
  argument submission; evidence packaging; recusal and consent
  tracking.
- **Interim-injunction issuance** — protocol-level "stop" order with
  enforcement linkage (§9).
- **Decision publication** — public decisions with mandatory reasoning
  trail; sealed decisions with threshold-reveal.

### 5. Escalation and designation

- **Ladder state machine** per dispute — virtue-discussion → formal
  complaint → mediation → graduated sanctions → *hostis humani generis*
  designation. Timed transitions where the research specifies them.
- **Designation procedure** — four-condition test (per
  `bounded_conflict.md` §"Handling the *hostis humani generis*"),
  quorum rules, review period, emergency-variant handling.
- **Restitution tracking** — restoration obligations, verification of
  completion.
- **Re-entry procedure** — examination, witnessing, re-recitation
  binding on re-admitted party.

### 6. Reputation bridges

- **Reputation ledger** — what is on the record across the compact:
  recitation history, completed mediations, designations (active and
  lifted), restoration completions.
- **Selective-disclosure primitives** — zero-knowledge proofs for
  narrow claims (e.g., "this community has no active designation
  against it") without exposing full history.
- **Decay model** — not pure exponential; step function for dramatic
  violations plus slow decay for minor. See
  `reputation_bridges.md` open question on half-life modeling.
- **Recitation history as foundational datum** — compact-anchored
  reputation root, portable across mediators and disputes.
- **Surveillance threshold** — configurable disclosure levels for
  internal health indicators; large/powerful communities disclose
  more by default.

### 7. Commons governance

- **Commons registry** — shared resources (knowledge archive, rescue
  capacity, communication relays, navigation beacons) with Ostrom-style
  boundary definitions.
- **Usage accounting** — consumption, contribution, free-rider
  detection.
- **Graduated-sanctions ledger** — progression from monitoring to
  warning to restriction to exclusion, with restoration paths.
- **Light-lag-tolerant consensus** — CRDT-like where semantics permit;
  BFT-style where they do not. Eventually-consistent governance is
  the baseline assumption (per `commons_federation.md`).
- **Nested-enterprise support** — hierarchical commons (community-level
  → regional → compact-wide) with appropriate authority at each level.

### 8. Jester and observer function

- **Observer corps registry** — declared-perspective reporters,
  rotation, cross-community composition requirements to prevent
  narrative capture by the dominant member.
- **Report publication** with attribution (public reporting) alongside
  whistleblower channel with source protection.
- **Threshold-held source records** — whistleblower identity
  recoverable only with mediator-panel quorum and only for named
  cause. Source protection must be structurally stronger than
  individual mediator integrity.
- **Archive access** — observer reports form a durable check on the
  mediator pool; archive must make observer corpus retrievable across
  generations.

### 9. Enforcement model

This is the load-bearing gap the research docs repeatedly punt on.
Three candidate mechanisms, layerable:

- **Reputation + directory consequences.** Ignored injunction →
  automatic restricted-member status until mediated. Software-enforced
  at the directory level: reputation lookups surface the violation; the
  injunction and its defiance are published to the archive
  automatically.
- **Capability-based access.** Compact-gated resources — rescue
  coordination endpoints, shared navigation, reputation-dependent
  trade, commons withdrawals — are revocable at protocol level when a
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

### 10. Bootstrapping and test harness

- **Simulation environment** for adversarial testing: captured-mediator
  scenarios, hollow-recitation drift, coordinated collusion, narrative
  capture of the observer corps, protocol-layer replay attacks.
- **Earth-pilot configuration** — small-scale deployment mode that can
  actually run with real communities. Necessary for the test-cases gap
  (§D.5 in skeptical_critique).
- **Integration harness** against `autonomous_trust` for live-AT
  message traffic observation, per
  [policy-evaluation.md](policy-evaluation.md).
- **Falsification criteria** — what observable outcomes in pilot
  deployments would count as kith-covenant failing? This needs explicit
  design, not implicit.

## Key architectural decisions

1. **Distributed substrate.** Libp2p-style peer-to-peer with
   content-addressed storage. No distinguished server; no mechanism
   that becomes a single point of failure or a sovereign-in-disguise.
2. **Cryptographic identity.** Decentralized identifiers (DID-style)
   with community-level key hierarchies. Quantum-resistant primitives
   at the root layer.
3. **Consensus model.** Compact-wide facts (recognition, designation,
   re-entry) are rare but need durability; most mediations are local
   to a panel. Probably *not* a single global chain — more likely
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

## Suggested incremental build order

The minimum useful slice, in order:

1. **Identity + recognition + archive** (components 1, 2, 3). The
   compact does not exist until recognition is real and verifiable.
   Testable with two communities in a room.
2. **Reputation lookup** (component 6, minimal slice). Once recognition
   events exist in the archive, downstream reputation claims become
   meaningful.
3. **Integration seam with autonomous_trust** (§5 above). Enables a
   member community to operate its AT instance with kith-covenant
   presence.
4. **Mediation state machine + escalation ladder** (components 4, 5).
   Needed before disputes are filed.
5. **Enforcement model** (component 9). Should not be deferred
   further; decisions here shape 1–4 in retrospect.
6. **Commons registry** (component 7) and **observer function**
   (component 8). Layerable once the core protocol runs.
7. **Test harness and adversarial simulation** (component 10) —
   continuous, not sequenced. Needs to exist from step 1.

## Open design questions

Several hard decisions remain unresolved and are flagged here rather
than buried in subsystem docs:

- **Enforcement model specifics.** Which of the three candidates in §9
  is primary? The research docs prefer transparency-as-teeth;
  capability-based access is the strongest at the code layer.
- **In-person requirement vs. interplanetary scale.** The hybrid
  proposal in §2 is working but unvalidated. The 3–20 minute
  Earth-Mars light-lag may be tolerable; multi-hour delays to outer
  system are not.
- **Re-affirmation cadence calibration.** The 25–30 year default lacks
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
  threshold-cryptography libraries, CRDT libraries — which stacks are
  mature enough to build on, which need to be built or forked?

## Relationship to other documents

- [policy-evaluation.md](policy-evaluation.md) — existing architecture
  doc covering the AT/kith-covenant evaluation seam; the integration
  point for §5 above.
- [../research/](../research/) — the research set this document
  operationalizes. Each component above corresponds to one or more
  research docs' design commitments.
- [../kith_covenant.md](../kith_covenant.md) — parent index.
- [../background/skeptical_critique.md](../background/skeptical_critique.md)
  §D — the "Operational Gaps" section that flagged these items; this
  document is the working response to that section.
