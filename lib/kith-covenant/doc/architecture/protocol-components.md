*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > [Architecture](README.md) > Protocol Components*

# Protocol Components and Operational Gaps

This document decomposes kith-covenant's software scope: what must be
built to make the social structures described in the research docs
actually runnable. It is the operational companion to the research set
and the working answer to the operational gaps the research documents
flag — enforcement, the mediator training pipeline, in-person recognition
at interplanetary scale, the re-affirmation cadence, and the absence of
pilots and falsification criteria. Those gaps, with what is already built
against each and the criteria by which each would be judged to fail, are
carried here directly: see §10 and "Open design questions" below.

> **MVP implementation status (2026-04-22).** Components **§1 Identity and
> standing**, **§2 Recognition ceremony**, and **§3 Archive / commons of
> memory** have an MVP Rust implementation in the `src/rust/` workspace
> (`kc-core`, `kc-identity`, `kc-ceremony`, `kc-archive`) with Python
> bindings (`kc-pyo3`). The `src/rust/examples/two_communities_demo` binary and
> `src/python/examples/two_communities_demo.py` both exercise the full
> three-component stack end-to-end: build two communities with real
> Ed25519 keys, run each community's intra-rite, run the inter-community
> credentialing ceremony with mutual acceptance of the five anti-spiral
> feature claims, archive every record, verify, roundtrip through JSON,
> and re-verify. See the repo [README](../../README.md) for quickstart.
>
> **§6 Reputation lookup** also has a minimal slice implemented
> (2026-04-22): the `kc-reputation` crate provides standing lookup and
> multi-community aggregation backed by the archive. See `src/rust/kc-reputation/`.
>
> **Capture-signal record types and detectors** are implemented
> (2026-07-16): the `kc-conduct` crate carries the `ConductRecord` payload
> (seeding §§5-6), including the jester's pre-indictment `Concern` kind;
> `SponsorshipRecord` lives in `kc-ceremony::inter` (§2/§7 vouching). Both
> are archived record kinds (`ConductAttested`, `SponsorshipVouched`),
> crypto-verified and projected by `kc-reputation`. Three detectors
> (`composition_graph`, `sponsor_stake`, `feature_concern_profile`) surface
> the composition-drift, sponsor-stake, and per-feature-concern capture
> signals as flags with provenance — no verdicts. Two further v0 read-only
> jester lenses (2026-07-17) sit alongside them: `rejection_pattern`
> (compact-wide admissions-integrity tallies over non-admission records) and
> `ceremony_cost_decay` (per-community witness-count and re-affirmation-gap
> trend over intra rites). Both are descriptive projections for a future
> capture-signal dashboard; neither renders a verdict. Quorum /
> decision-class is *recorded* (`decision_class`, multi-signature envelope)
> but the distinct-community quorum *check* is deferred to §5. See
> `src/rust/kc-conduct/` and `kc-reputation/src/analytics.rs`.
>
> **Standing + authoring primitives (2026-07-17).** `RecognitionStanding`
> gained `Probationary` (admitted and within an active received-sponsorship
> probation window) and `Anathema` (an active `DesignationAnathema` not
> superseded by a later `DesignationLifted`; overrides all other standings).
> Precedence: Anathema > Probationary > InCompact > Denied > Identified >
> Unknown. The Python authoring surface can now build the records that were
> previously constructable only inside the Rust ceremonies:
> `record_non_admission` (feeds the rejection-pattern lens) and
> `run_intra_rite` (re-affirmations, feeding ceremony-cost-decay), plus
> `designate_anathema` / `lift_designation` convenience over `record_conduct`.
>
> **Delegation revocation (2026-07-17).** Revocation is append-only: a
> superseding `SpeakerDelegation` tombstone (later `issued_at`, `revoked_at`
> set). Resolution is now latest-`issued_at`-wins per `speaker_did` in both
> `resolve_did` (kc-archive, governs signature verification) and the
> `active_delegations` projection (kc-reputation), so a tombstone overrides an
> earlier live grant; a re-grant with a still-later `issued_at` reactivates.
> Non-retroactive (records dated before the tombstone still verify).
> Authorable via `revoke_delegation`.
>
> **§5 authority enforcement (2026-07-17).** `check_authority(archive,
> quorum_threshold=2)` (kc-archive `authority` module) is a pass distinct from
> the cryptographic `verify_all`. It enforces (1) **principal binding** — every
> principal a record speaks for must carry a signature from its own root or an
> active `SpeakerDelegation` whose `community_did` is that principal;
> (2) **scope-appropriateness** — a delegate's scope must be allowed for the
> record type (Envoy/CrossBoundaryWitness for inter records, Envoy-or-root for
> sponsorship/conduct, root-only for intra-rite seal / delegation / registry);
> (3) **quorum** — a second-class decision (`DecisionClass::SecondClass`, e.g.
> `DesignationAnathema`) needs >= `quorum_threshold` distinct principal
> communities. Revocation/expiry are honored. `authority_violations` returns all
> offending records instead of failing fast. This upgrades the earlier
> "recorded, enforcement deferred" quorum note above: the count is now enforced
> by `check_authority` (still a separate pass from crypto verification).
>
> **§4 Mediation (v1, 2026-07-17).** The `kc-mediation` crate implements a
> bounded first slice: a **mediator pool** as per-community `MediatorRegistry`
> records (the pool is the union across the archive; latest `valid_from` per
> nominating community supersedes, mirroring the witness registry), **dispute
> intake** (`DisputeRecord`, signed by the complainant, carrying the
> `LadderRung` it reaches for), and **paneled decisions** (`MediationDecision`,
> co-signed by the panel). The research's structural rules are enforced through
> the §5 authority pass rather than a separate mechanism: a mediation decision
> is authorized only when every co-signer is an active registered mediator, the
> panel spans at least `panel_min` of them (default 3; a small compact may lower
> it for an ad-hoc pie-powder panel), and **no panelist comes from either
> party's community** (non-overlap). `check_authority` and `authority_violations`
> gained the `panel_min` parameter; `resolve_did` resolves mediator keys from the
> registry so a panel co-signature verifies without a separately published
> identity. Disputes project onto both parties in `kc-reputation`
> (`disputes_filed` / `disputes_against`, joined with the decision outcome).
> Authorable from Python (`register_mediators`, `file_dispute`,
> `decide_mediation`); `src/python/examples/mediation_demo.py` walks a valid
> panel plus the two panels the protocol refuses. **Deferred to later slices:**
> deliberation / argument-submission records, interim injunction (ties to §9),
> the structured-skepticism protocol, protecting-powers, recusal-request
> workflow, mediator training lineage, the capture-resistance pool-share
> threshold (no single community may dominate the pool), and the full escalation
> ladder state machine (§5). See `src/rust/kc-mediation/`.
>
> **§7 Commons governance (v1, 2026-07-21).** The `kc-commons` crate implements
> a bounded first slice applying Ostrom's design principles to the five space
> commons: a **commons registry** (`CommonsRegistration` — a `CommonsKind`, the
> appropriator set that is Ostrom's clear boundary, and a provision rule; a
> **second-class** act requiring a distinct-community quorum, every registrar an
> appropriator), **usage accounting** (`UsageRecord` — a community's first-class,
> self-signed consumed/contributed report), and the **graduated-sanction ledger**
> (`CommonsSanction` — an append-only `SanctionLevel` ladder Monitoring → Warning
> → Restriction → Exclusion, mirroring the §5 dispute ladder: a move leaves the
> pair's current level and either escalates one step or de-escalates to any lower
> level as the restoration path; the second-class levels Restriction/Exclusion
> carry the distinct-community quorum). The structural rules are enforced through
> the §5 authority pass (`authorize_commons_registration`, `authorize_usage`,
> `authorize_commons_sanction` with a `current_sanction_level` helper), so
> **light-lag-honest decision-class enforcement** (§3/§7) is realized here as the
> existing multi-community quorum check. `kc-reputation` projects a community's
> current per-commons sanction level (`CommonsSanctionSummary`, restored commons
> dropping out) and a `commons_level` gate, and carries a read-only
> `free_rider_signals` lens (per-appropriator consumed/contributed tallies and a
> net-draw ranking — the tragedy-of-the-virtues made countable, no verdict).
> Authorable from Python (`register_commons`, `record_usage`, `apply_sanction`,
> `commons_level`, `free_rider_signals`);
> `src/python/examples/commons_demo.py` walks a registration, usage, a graduated
> escalation with restoration, and the three moves the protocol refuses.
> **Deferred to later slices:** the full CRDT/BFT consensus mechanics and the
> signature-latency window, nested inter-compact enterprises, threshold-sealed
> usage, ZK selective-disclosure of usage, the commons-specific escalation
> trigger (the "commons equivalent of the anathema designation" open question),
> and the constitutional reclassification machinery (a §9 substrate-deference
> concern). See `src/rust/kc-commons/`.
>
> **§8 Observer / jester corps (v1, 2026-07-21).** The `kc-observer` crate
> promotes the read-only capture lenses into a corps with standing. An
> **observer-corps registry** (`ObserverRegistry` / `ObserverEntry` — nominated
> observers carrying a **declared perspective**, superseding per community like
> the mediator/witness registries; `resolve_did` gained an observer arm) backs
> **attributed public reporting**: an `ObserverReport` is signed by a registered
> observer whose home community is **not** the subject (cross-community
> journalism, non-overlap), and the subject answers with an appended
> `ReportResponse` that never suppresses it (the Type-4-heterodoxy visibility
> mechanism the escalation ladder cannot reach). The **whistleblower channel**
> (`WhistleblowerReport`) is filed by a non-party mediator **reception panel** and
> stores only a `source_commitment` — the source identity is never recorded; a
> `SourceReveal` discloses it solely on a mediator-panel quorum citing named
> cause. `kc-reputation` projects a community's `observer_reports` (with
> responses) and `whistleblower_reports_against` (never the source, only whether a
> reveal has occurred), and an `observer_dashboard` aggregator bundles the
> sponsorship-composition and rejection lenses with per-community report tallies
> — the **composition-drift dashboard**. Authorable from Python
> (`register_observers`, `publish_observer_report`, `respond_to_report`,
> `file_whistleblower`, `reveal_source`, `observer_dashboard`);
> `src/python/examples/observer_demo.py` walks a report with response, a
> whistleblower with a for-cause reveal, the dashboard, and the three moves the
> protocol refuses. **Deferred to later slices (MVP crypto is Ed25519-only):**
> ring-signature source *anonymity* (prove insider membership without revealing
> which member), threshold-*encrypted* source records, the **substrate-deference**
> half of the §8 dashboard (needs §9's compact-incapable list and structured-fast
> state), observer pool-share and rotation caps, and secure out-of-band
> whistleblower transport. See `src/rust/kc-observer/`.
>
> **§9 Substrate-deference test (v1, 2026-07-21).** The `kc-constitution` crate
> operationalizes non-ultimacy as a versioned, signed constitutional layer (the
> post-hoc enforcement candidates — reputation/directory, capability-based, and
> transparency — are already realized by the §5 authority pass, the §9 injunction
> capability-gate, and the append-only archive respectively). `ConstitutionGenesis`
> founds the **compact-incapable list** (matters the compact will not decide — a
> member's internal theology, governance, recognition-statement content) and the
> **structured-fast cadence** (`FastPeriod` windows during which deliberation
> pauses); it is quorum-ratified and a **singleton** per archive, and its matters
> are **genesis-immutable**. `CompactIncapableAmendment` and
> `FastCadenceAmendment` are second-class and **asymmetric**: expansion (Add /
> Extend) uses the normal quorum, contraction (Remove / Contract) requires the
> higher bar (`quorum+1`), and a genesis-original matter can never be removed —
> the asymmetry that reflects the capture risk. `kc-reputation` exposes the
> `constitution_state` projection, the `compact_incapable` (categorical) and
> `in_structured_fast` (temporal) **deference gates**, and the
> `substrate_deference` **erosion report** — the list's size and add/remove drift
> with a contraction flag, the fast cadence and its contraction count, and
> deliberative acts initiated during a fast (**fast deviations**). Per the
> research the fast is honored by practice and made *legible* here, not
> hard-refused; the erosion report is the substrate-deference half of the §8
> dashboard the observer slice deferred. Authorable from Python
> (`ratify_constitution`, `amend_compact_incapable`, `amend_fast_cadence`,
> `constitution_state`, `compact_incapable`, `in_structured_fast`,
> `substrate_deference`); `src/python/examples/constitution_demo.py` walks
> ratification, the gates, an expansion, an erosion report, and the moves the
> protocol refuses (second genesis, removing a genesis matter, contracting below
> the higher bar). **Deferred to later slices:** recurring/calendar fast schedules
> (v1 uses explicit windows), automatic escalation on detected erosion, and any
> hard-refusal of deliberative acts during a fast. See `src/rust/kc-constitution/`.
>
> **§10 Capture-resistance test harness (v1, 2026-07-21).** §10 is a *harness*,
> not a protocol component with records, so it is realized as a Python module
> (`src/python/kith_covenant/harness.py`) driving the real public API — the honest
> generalization of the hand-authored refusal cases, which are themselves Python.
> `run_harness()` runs the **seven capture-resistance scenarios** and scores each
> against **falsification criteria stated in advance** (module constants for
> detection latency and damage bounds — the §10 requirement that failure be
> defined explicitly, not implicitly). The four scenarios whose detectors are
> built are exercised end-to-end on fresh, time-evolving archives and scored
> PASS/FAIL: **move 2** compact-incapable erosion (via `substrate_deference`'s
> contraction flag), **move 3** ceremony hollowing across cycles (via
> `ceremony_cost_decay`), **move 5** light-lag fait-accompli by forged quorum (a
> single faction's second-class act, rejected by `check_authority`; `verify_all`
> is the crypto backstop), and **move 7** admissions capture by patient
> sponsorship (via `composition_graph` concentration). The three whose detectors
> are deferred past this MVP — **move 1** faction-monopolized interpretation,
> **move 4** manufactured-event injection, **move 6** principal capture — are
> reported as **unsupported**, each naming the missing capability, so coverage
> gaps are surfaced rather than silently omitted. `detection latency` and `damage`
> are the primary metrics per scenario. `src/python/examples/capture_harness_demo.py`
> prints the scoreboard. **Deferred to later slices:** the Earth-pilot deployment
> configuration and the live-`autonomous_trust` integration harness (both need a
> real deployment, not a simulator); continuous/background scenario execution;
> and the three unsupported scenarios, which unblock as their detectors are built.
>
> **§10 detectors for moves 1, 4, 6 (v2, 2026-07-21).** The three deferred
> detectors are now built, and their scenarios are scored PASS on fresh,
> time-evolving archives through the same public API — the harness now reports
> **7 supported, 7/7 passed**. **Move 1** faction-monopolized interpretation: the
> new `kc_ceremony::lineage::SubstrateInterpretation` record carries a community's
> attested readings of its own substrate (lineage id + interpreter), and the
> `interpretive_plurality` lens surfaces a single-interpreter/single-lineage
> monopoly while clearing a community that demonstrates distinct lineages and
> interpreters. **Move 4** manufactured-event injection: the new `kc-provenance`
> crate carries `EventReport` (provenance-tagged at intake — originator, declared
> incentives, information history, stake disclosure, and the structured-skepticism
> qualifying triggers) and `EventCorroboration`; the `structured_skepticism` lens
> flags a qualifying event as under-corroborated the moment it lands (before it
> can drive action) and clears it once distinct non-originator communities
> corroborate. **Move 6** principal capture: the new `kc-principal` crate carries
> `PrincipalWitnessAttestation` (cross-community validity attestation) and
> `PrincipalOutput` (for triangulation); the `adversarial_principal` lens counts
> cross-community witnesses (excluding the principal and its home) and flags an
> output diverging from the cohort majority toward the principal's own benefit.
> All three follow the established jester-lens contract — flags with provenance,
> no verdict; thresholds live with the caller. **Deferred to later slices:**
> anti-sockpuppet proof of interpreter independence (move 1); enforced structured
> delay, cross-substrate translation semantics, and the retroactive bad-faith
> finding (move 4); the cryptographically separable witness chain, statistical
> triangulation of continuous behavior, and collusive whole-cohort capture
> (move 6). Records are Ed25519-signed, first-class, principal-bound.
>
> **All ten components now have an MVP v1 slice** (uncommitted on
> `feature/research`), and all seven capture-resistance scenarios have a built,
> scored detector. What remains is not whole components but the parts each
> slice explicitly deferred (catalogued per §-paragraph above and in §11) plus the
> two deployment-dependent §10 items. MVP crypto is Ed25519-only;
> `Alg` is an enum whose shape preserves a future non-lattice PQC slot. The
> capture-resistance responsibilities carried inside §§1-10 and catalogued in §11
> are realized by the slices above: interpretive-plurality evidence (§1/§2),
> manufactured-event provenance and structured-skepticism (§3/§4), and
> adversarial-principal validation (§1/§6) are the detectors the §10 harness now
> exercises end-to-end.

Capture resistance is a cross-cutting architectural concern, not a
component: each component below carries capture-resistance
responsibilities as part of its ordinary specification, and §11
catalogues the unified concern, routes the seven strengthening moves of
[capture_resistance.md](../background/capture_resistance.md) to their
component homes, and names the implementation work that follows.

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

## Design constraint: the bootstrapping problem

The framework protects preconditions it cannot itself generate: the
recognition ceremony requires communities that *already* hold the
substrate. This is developed in
[justus_hostis.md](../background/justus_hostis.md) §"The bootstrapping
problem", where the compact is cast as a protector and connector of
communities that already meet the preconditions rather than a generator
of them. The problem is shared by every cooperation framework that
requires pre-existing trust, Ostrom's commons cases included. It is a
standing constraint on the whole decomposition below: the software cannot
manufacture the character, the substrate, or the trust it depends on, only
protect and connect what exists. Where a component appears to demand that
the compact create trust — the mediator pool before there is a cadre, a
pilot before there is a compact — the constraint is the reason the design
reaches for existing communities and existing ties rather than generating
them from nothing.

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
- **Cross-community principal-witness attestation channel**: a
  separate attestation chain
  for principals (human guardians of derived-substrate machine
  participants and, by extension, designated speakers for human
  members) carried by witnesses in *other* member communities than
  the principal's own. Cryptographically separable from the
  within-community attestation; supports the
  adversarial-principal validation specified in
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation)
  and
  [machine-participation.md "Guardian capture"](machine-participation.md#facet-3-validity-of-the-derivative-oath-succession-and-boundary-conditions).
- **Interpretive-plurality evidence at recognition**: the
  inter-community
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
  *Design state:* the implemented ceremony (`kc-ceremony`) models
  presence as a witness co-signature over a recitation CID plus a
  timestamp — there is no location, liveness, or bodily-attendance field,
  and a single internal witness satisfies the intra rite. Co-presence is
  therefore not enforced at the code layer; a CID co-signature can be
  produced anywhere. Whether co-presence should be given a code-layer
  attestation (itself spoofable) or left a social requirement audited
  through the ceremony-cost proxy (§6) is an open decision, tracked in
  "Open design questions" below.
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
- **Ceremony-cost proxy tracking**:
  recorded proxies for asymmetric psychological cost across
  re-affirmation cycles: preparation time and attention,
  recitant articulation of what is being committed and forgone,
  witness reports of comportment, content of the
  anti-spiral-feature examination exchange, substantive vs. rote
  responses to challenge questions. Surfaces decay across cycles
  per [recognition_protocols.md "Tracking ceremony cost"](../background/recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles).
- **Substantive re-engagement rung**:
  a heavier re-affirmation cycle invoked when proxy decay is
  detected, with extended preparation, more demanding examination,
  and reasserted cost. This is a designated state between ordinary
  re-affirmation and anathema designation.
- **Credentialing-panel rotation**: the examiners who conduct the
  anti-spiral-feature examination are drawn by lot from member
  communities, serve fixed terms, publish reasoning, and rotate
  out, with cross-community composition requirements, per
  [capture_resistance.md "Admissions integrity"](../background/capture_resistance.md#7-admissions-integrity-the-gate-as-capture-surface).
- **Admission as second-class decision**: admission and readmission
  records validate only with multi-community quorum signatures
  under the decision-class machinery (§3, §7).
- **Sponsor-stake tracking**: vouching records tie a sponsor's
  standing to the entrant's conduct through the probationary
  period; sponsorship patterns that repeatedly precede misconduct
  are surfaced as capture signatures (§6, §8).
- **Probationary-status enforcement**: probationary members carry
  no deliberative weight in admissions, mediator selection, or
  designation procedures until the period completes.

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
- **Manufactured-event provenance tagging**: records advancing
  interpretations of high-charge
  events carry originator identity, declared incentives,
  information-history of the interpretation's emergence, and
  stake-disclosure entries; used by the structured-skepticism
  protocol per [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).
- **Decision-class metadata**:
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
- **Structured-skepticism protocol invocation**: mediator-pool
  authority to invoke the protocol on
  events meeting qualifying criteria (high interpretive charge,
  faction-aligned interest, sudden onset, originating from
  stake-holding witnesses). Imposes structural delay,
  multi-witness corroboration, cross-substrate translation, and
  stake-disclosure requirements. Mediator-pool internal review
  guards the invocation against pretextual use to suppress
  legitimate urgent matters. Specified in detail in
  [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).

### 5. Escalation and designation

- **Ladder state machine** per dispute, implementing the canonical
  ladder of
  [bounded_conflict.md "Protocol sketch"](../background/bounded_conflict.md#protocol-sketch):
  dispute registration → registered protest → reciprocal suspension
  of specified courtesies → mediated arbitration → limited material
  sanctions → compact-authorized collective sanctions → expulsion.
  Anathema designation and virtue-violation enforcement are separate
  tracks, not rungs. Timed transitions where the research specifies
  them.
- **Designation procedure**: four-condition test (per
  [bounded_conflict.md "Handling the anathema"](../background/bounded_conflict.md#handling-the-anathema)),
  quorum rules, review period, emergency-variant handling. The
  *hostis humani generis* tier within anathema carries its higher
  evidentiary threshold.
- **Restitution tracking**: restoration obligations, verification of
  completion.
- **Re-entry procedure**: examination, witnessing, re-recitation
  binding on re-admitted party.
- **Substantive-re-engagement rung**:
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
- **Principal-output triangulation data flows**: comparison of a
  machine's
  conduct against the ensemble of similarly-situated machines
  under similarly-positioned principals. Divergence flags entered
  in the reputation ledger for cross-community review per
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation).
- **Ceremony-cost decay surfacing**: reputation queries surface
  proxy-decay state
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
- **Decision-class enforcement**:
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

> **Constitutional standing.** The
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
- **Capture-signature monitoring**:
  the observer corps maintains active monitoring for the legible
  signs of early-stage capture: substrate-deference erosion (shrinking
  compact-incapable list, shortening structured-fast period),
  ceremony-cost proxy decay across multiple member communities,
  manufactured-event patterns, principal-output triangulation
  divergence. Monitoring outputs are reported to the audit commons
  on a continuous cadence rather than only when specific reports are filed.
- **Substrate-deference reporting**:
  observer corps maintains the public dashboard tracking
  compact-incapable list state and structured-fast cadence
  observance, making the substrate-deference test's operational
  practice continuously legible to the compact's members.
- **Composition-drift dashboard**: the observer corps maintains an
  admissions dashboard alongside the substrate-deference dashboard:
  who sponsored, who examined, which admissions clustered by
  region, lineage, or sponsor. Clustering in directions that
  benefit an identifiable faction is flagged before the faction
  reaches deliberative weight.

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

**Substrate-deference test as ongoing constitutional operation.**
Beyond the post-hoc enforcement
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
  capture of the observer corps, protocol-layer replay attacks. The seed
  already exists as negative-path coverage the protocol must refuse: the
  `kc-core` payload-tamper test, the `kc-archive` §5 authority tests
  (wrong-scope delegate, cross-community forgery, quorum and panel
  failures), the Python `test_authority.py` / `test_mediation.py`
  (panel-too-small, party-mediator, unregistered-mediator) /
  `test_standing.py` (declining rites, revoked delegation), and the
  `capture_signals_demo.py` / `mediation_demo.py` demos, which construct
  archives the protocol must reject. The harness generalizes these
  hand-authored refusal cases into a time-evolving simulator rather than a
  fixed pass/fail set.
- **Earth-pilot configuration**: small-scale deployment mode that can
  actually run with real communities. Necessary for the test-cases gap.
  First compacts should be among Earth-bound communities with existing
  ties and cheap exit — denominational networks, diaspora federations,
  tribal confederations, ecological commons, cooperative-firm networks —
  generalized to space only after Earth has been the harder first test.
- **Integration harness** against `autonomous_trust` for live-AT
  message traffic observation, per
  [policy-evaluation.md](policy-evaluation.md).
- **Capture-resistance scenario simulation**: adversarial test
  categories specifically targeting
  the seven strengthening moves: faction-monopolizing-interpretation
  within a member community (move 1); compact drift toward
  ultimacy via compact-incapable-list erosion (move 2); ceremony
  hollowing across simulated generational cycles (move 3);
  manufactured-event injection with timed loudest-interpreter
  pressure (move 4); light-lag fait-accompli attack with forged
  quorum signatures (move 5); principal-capture by external
  manipulation including AI-mediated influence operations (move
  6); admissions capture by patient sponsorship of aligned
  entrants toward the collusion threshold (move 7). Each scenario
  is run continuously in the simulation environment, with
  detection latency and damage bounds as the primary metrics.

**Falsification criteria.** What observable outcomes would count as
kith-covenant failing must be stated in advance, per component and at the
whole-compact level — explicit design, not implicit. This is the
least-built of the operational gaps and the precondition for any pilot.

The top-level criterion is **retention despite exit** under weak forcing,
per [bounded_conflict.md "Earth first"](../background/bounded_conflict.md#earth-first-the-harder-test):
whether communities keep choosing the compact's recognition, mediation,
and commons when leaving is cheap and the state system is available. A
compact that holds when exit is free has demonstrated that it binds
voluntarily. Earth is therefore the harder first test, not a rehearsal
for space.

The standing caveat is that **"the teeth" — sanction and enforcement
effectiveness — are structurally untestable on Earth**, where a determined
defector simply leaves. An Earth pilot that fails to demonstrate
enforcement has not thereby falsified the framework; it has reached the
edge of what Earth can test, and pilots must not be scored on it.

Per-component criteria:

- **Injunction enforcement (§9).** Success: defiance of an interim
  injunction is published unforgeably and in a form that downstream
  reputation and capability consequences can act on, without a sovereign.
  Failure: an order that can be ignored with no archived trace — a paper
  order.
- **Mediator pool (§4).** Success: panels stay non-overlapping and
  un-captured as the pool rotates through cases, and disputing communities
  keep accepting panel decisions when they could walk. Failure tripwire:
  the emergence of a permanent mediator class — the inquisitorial caste the
  term limits exist to prevent.
- **In-person recognition vs. light-lag (§2).** Success: a remote
  generational re-affirmation still carries real cost — treated as
  consequential, not a formality. Failure: remote rites drift into
  formality while in-person ones stay costly, so the filter does its work
  only where it is least needed.
- **Re-affirmation cadence (§2, §5).** Success: the cadence catches
  substrate drift before it reaches anathema without the rite decaying into
  an empty ritual. A cadence that flags nothing and one that flags
  constantly are both failures.

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

**The seven strengthening moves and their component homes.**

| Move | Focus | Primary component(s) | Specification |
|---|---|---|---|
| 1. Interpretive plurality | Member-substrate health | §1 (identity), §2 (recognition) | [recognition_protocols.md "Inter-community layer"](../background/recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing) item 5 |
| 2. Substrate-deference test | Compact-level non-ultimacy | §9 (enforcement), §8 (jester monitoring) | [justus_hostis.md "Non-ultimacy"](../background/justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient) feature 5 |
| 3. Ceremony cost tracking | Ceremony erosion detection | §2 (recognition), §3 (archive), §5 (escalation), §6 (reputation) | [recognition_protocols.md "Tracking ceremony cost"](../background/recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles) |
| 4. Manufactured-event resistance | Interpretation-dominance prevention | §3 (archive), §4 (mediation) | [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol) |
| 5. Light-lag-honest decision classes | Fait-accompli prevention | §3 (archive), §7 (commons) | [commons_federation.md "Light-lag-honest decision classes"](../background/commons_federation.md#light-lag-honest-decision-classes) |
| 6. Adversarial-principal validation | Principal-capture detection | §1 (identity), §6 (reputation) | [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation), [machine-participation.md "Guardian capture"](machine-participation.md#facet-3-validity-of-the-derivative-oath-succession-and-boundary-conditions) |
| 7. Admissions integrity | Composition-capture prevention | §2 (recognition), §8 (observer dashboard), §10 (test harness) | [capture_resistance.md "Admissions integrity"](../background/capture_resistance.md#7-admissions-integrity-the-gate-as-capture-surface) |

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
- Credentialing-panel rotation, sponsor-stake tracking, and
  probationary-status enforcement (§2).
- Composition-drift dashboard maintained by observer corps (§8).
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
capture-resistance responsibilities specified inside §§1-10 and
unified in §11.

1. **Identity + recognition + archive** (components 1, 2, 3). The
   compact does not exist until recognition is real and verifiable.
   Testable with two communities in a room. **(CR)** Identity
   includes the cross-community principal-witness channel (§1);
   recognition records the interpretive-plurality evidence and
   carries the admissions-integrity machinery (panel rotation,
   sponsor stake, probationary status) (§2);
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
   with the substrate-deference dashboard, the composition-drift
   dashboard, and capture-signature monitoring as standing
   competencies, not optional features.
7. **Capture-resistance test harness** (component 10, expanded). The
   seven adversarial scenarios (faction-monopolizing-interpretation,
   compact drift toward ultimacy, ceremony hollowing, manufactured-event
   injection, light-lag fait-accompli, principal-capture, admissions
   capture) must
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
  `recognition_protocols.md` open questions, which argues for a
  compact-formation parameter with per-community override rather than a
  constant. A second, prior decision: is the cadence *enforced* (an
  overdue or hollowing re-affirmation does something on its own — expiry,
  escalation) or *observed only* (the `ceremony_cost_decay` lens surfaces
  decay and the jester and mediators act, but nothing triggers
  automatically)? The lens is built; the consequence is not.
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
- [../background/](../background/): the research set this document
  operationalizes. Each component above corresponds to one or more
  research docs' design commitments.
- [../kith_covenant.md](../kith_covenant.md): parent index.
