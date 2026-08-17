*Previous: [The crates, and what each one holds](../crates.md)*

# Protocol Components and Operational Gaps

This document decomposes the software scope of kith-covenant.
What must be built to make the social structures described in
the research docs actually runnable. It is the operational
companion to the research set and the working answer to the
operational gaps the research documents flag: enforcement,
training, recognition, cadence, and falsification. Those are the
mediator training pipeline, in-person recognition at
interplanetary scale, the re-affirmation cadence, and the
absence of pilots and falsification criteria. Those gaps, with
the answer of the design to each and the criteria by which each
would be judged to fail, are carried here directly: see §10 and
"Open design questions" below.

The component sections below say what each component is and
does. For the coverage by the implementation of them, see the
component table in the [README](../../README.md). For scope the
protocol describes but the code does not yet carry,
[`TODO.md`](../../TODO.md).

Capture resistance is a cross-cutting architectural concern, not a component.
Each component below carries capture-resistance responsibilities as part of its
ordinary specification, and §11 catalogues the unified concern, routes the seven
strengthening moves of
[capture_resistance.md](../background/capture_resistance.md) to their component
homes, and names the implementation work that follows.

## Purpose and scope

The software of kith-covenant is the **inter-community protocol
layer**. It sits above [muudd](../../../../doc/muudd.md)
(communications) and is orthogonal to the internal
[autonomous_trust](../../../../lib/autonomous_trust/doc/autonomous_trust.md)
instance each community runs. Even where a member community is
the subject of a record, the protocol never reaches *into* it.
It operates only between communities and on compact-level
artifacts.

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

The software must realize the mechanisms the research set
commits to: recognition ceremony, archive, mediation,
escalation, reputation bridges, commons governance, the jester
function, exit, and succession. Everything below is organized
around those commitments.

## Design constraint. The bootstrapping problem

The framework protects preconditions it cannot itself generate. The recognition
ceremony requires communities that *already* hold the substrate. This is
developed in [justus_hostis.md](../background/justus_hostis.md) §"The
bootstrapping problem", where the compact is cast as a protector and connector
of communities that already meet the preconditions rather than a generator of
them. The problem is shared by every cooperation framework that requires
pre-existing trust, Ostrom's commons cases included. It is a standing constraint
on the whole decomposition below. The software cannot manufacture the character,
the substrate, or the trust it depends on, only protect and connect what exists.
Where a component appears to demand that the compact create trust (the mediator
pool before there is a cadre, a pilot before there is a compact), the constraint
is the reason the design reaches for existing communities and existing ties
rather than generating them from nothing.

## Component decomposition

### 1. Identity and standing

- **Community identity.** Cryptographic keypair for the
  community as party, distinct from the key of any individual.
  Must survive leadership change within the community. Requires
  a "successor key" mechanism so internal rotation does not cost
  compact standing.
- **Designated speakers.** Signed delegations from the community
  key authorizing specific persons to recite, file, or commit on
  behalf of the community. Revocable.
- **Witness registry.** Identities of individuals qualified to
  witness recognition events, with rotation schedule and continuity
  constraints (per `recognition_protocols.md` "witness continuity").
- **Quantum-resistant primitives.** Given the generational time
  horizons, identity keys should not rest on schemes vulnerable to
  medium-term cryptographic attack.
- **Cross-community principal-witness attestation channel.** A
  separate attestation chain for principals (human guardians of
  derived-substrate machine participants and, by extension,
  designated speakers for human members) carried by witnesses in
  *other* member communities than the own. Cryptographically
  separable from the within-community attestation. Supports the
  adversarial-principal validation specified in
  [m2m_interaction.md "Adversarial-principal
  validation"](../background/m2m_interaction.md#adversarial-principal-validation)
  and [machine-participation.md "Guardian
  capture"](machine-participation.md#facet-3-validity-of-the-derivative-oath-succession-and-boundary-conditions).
- **Interpretive-plurality evidence at recognition.** The
  inter-community
  credentialing process records evidence of multiple traceable
  interpretive lineages within the candidate community, per
  [recognition_protocols.md "Inter-community layer"](../background/recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing)
  element 5 and
  [capture_resistance.md "Interpretive plurality"](../background/capture_resistance.md#1-interpretive-plurality-as-a-structural-requirement-on-members).

### 2. Recognition ceremony (two layers)

Per `recognition_protocols.md` "Two layers. Intra-community authorization and
inter-community grammar," the recognition mechanism is split. The software must
support both layers and must not conflate them.

- **Intra-community ceremony runner.** Drives the five-movement
  rite of each community as performed on the envoys of that
  community and representative machines, collects internal
  witness attestations, seals the intra-community record in the
  terms of the community itself. The members of the community
  itself are the witnesses here. The asymmetric-
  psychological-cost mechanism operates at this layer and
  requires bodily co-presence for the reasons given in
  `recognition_protocols.md`.
- **Inter-community credentialing runner.** Accepts the sealed
  intra-community record as credential, drives the compact-level
  sequential presentation (each community performs its own rite in
  turn under cross-community witness), produces the anti-spiral-
  feature examination exchange (per `overview.md` §5b item (e) and
  §3b.i), and seals the cross-boundary witness record.
- **In-person / remote bridge.** Intra-community rites require
  bodily co-presence (the cost mechanism demands it). Cross-boundary
  witness may tolerate hybrid forms at interplanetary scale.
  Working design decision: intra-community authorization in person
  always. Compact-formation cross-boundary witness in person when
  physically reachable. Cross-boundary witness of intra-community
  generational re-affirmation cryptographically bound + video, with
  in-person confirmation at the nearest practical opportunity.
  *Design state:* the implemented ceremony (`kc-ceremony`) models
  presence as a witness co-signature over a recitation CID plus a
  timestamp. There is no location, liveness, or bodily-attendance field,
  and a single internal witness satisfies the intra rite. Co-presence is
  therefore not enforced at the code layer. A CID co-signature can be
  produced anywhere. Whether co-presence should be given a code-layer
  attestation (itself spoofable) or left a social requirement audited
  through the ceremony-cost proxy (§6) is an open decision, tracked in
  "Open design questions" below.
- **Recitation archive.** Content-addressed, tamper-evident,
  publicly verifiable at both layers. The intra-community rite
  of each community is sealed separately and referenced into the
  inter-community record that the compact holds. Referenced by
  reputation primitives (§6).
- **Re-affirmation scheduler.** Tracks generational cadence per
  community. Surfaces overdue or imminent intra-community
  re-affirmations. Signals cross-boundary witnesses when the
  re-affirmation event of a member community is scheduled.
- **Versioning.** Movement 4 content belongs to each community.
  What is versioned is the recitation *schema*, one protocol-wide
  constant bumped when movement semantics change, and the record
  of a rite carries it. Per-community content versioning does not
  exist. What identifies the rite a witness saw is the content
  address of the sealed intra-community record, which the
  inter-community witness record carries, so the layer tracks
  which rite was witnessed rather than which version of it.
  The movement itself is opaque, unindexed, and unaddressable,
  which is why no measure can cite a numbered commitment from a
  recited list
  ([substitution.md](../background/substitution.md)).
- **Ceremony-cost proxy tracking.** Recorded proxies for asymmetric psychological cost across
  re-affirmation cycles. Preparation time and attention,
  recitant articulation of what is being committed and forgone,
  witness reports of comportment, content of the
  anti-spiral-feature examination exchange, substantive vs. rote
  responses to challenge questions. Surfaces decay across cycles
  per [recognition_protocols.md "Tracking ceremony cost"](../background/recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles).
- **Substantive re-engagement rung.** A heavier re-affirmation
  cycle invoked when proxy decay is detected, with preparation,
  examination, and cost all reasserted: extended preparation,
  more demanding examination, and cost restored to the founding
  level. This is a designated state between ordinary
  re-affirmation and anathema designation.
- **Credentialing-panel rotation.** The examiners who conduct the
  anti-spiral-feature examination are drawn by lot from member
  communities, serve fixed terms, publish reasoning, and rotate
  out, with cross-community composition requirements, per
  [capture_resistance.md "Admissions integrity"](../background/capture_resistance.md#7-admissions-integrity-the-gate-as-capture-surface).
- **Admission as second-class decision.** Admission and readmission
  records validate only with multi-community quorum signatures
  under the decision-class machinery (§3, §7).
- **Sponsor-stake tracking.** Vouching records tie the standing
  of a sponsor to the conduct of the entrant through the
  probationary period. Sponsorship patterns that repeatedly
  precede misconduct are surfaced as capture signatures (§6,
  §8).
- **Probationary-status enforcement.** Probationary members carry
  no deliberative weight in **any second-class decision** until the
  period completes, admissions, mediator selection and designation
  procedures first, and every other compact-wide act on the same
  reasoning. First-class local voice is untouched.
- **Examiner caps.** Examining seats are bounded in term, rotated, and
  pool-share capped, so no one community can hold a majority of the
  panel pool (`ExaminerCaps`, one implementation shared with the §8
  observer caps rather than a restatement of them).
- **Admission into a standing compact.** Distinct from compact
  formation, and gated on three things at once, an admitting quorum of
  sitting non-probationary members, a sponsor for every entrant, and an
  examining panel spanning at least two communities. Sponsor stake is
  required of every admission, which is what makes probation
  load-bearing rather than opt-in: without it, a faction that seats
  examiners and simply never vouches opens no probation window at all.
- **What probation withholds.** A probationary community may not sponsor
  an entrant (a refusal rather than a de-weighting, since nothing counts
  vouches and a weightless vouch would be an inert record), may not seat
  mediators or examiners, does not count toward `panel_min`, and does
  not count toward a designation or admitting quorum. The withholding is
  written against the second-class *class* rather than a list of record
  types, so a second-class record added later inherits it instead of
  quietly escaping it. A petitioner is not de-weighted by the very
  re-entry record it is party to, since a re-entry's `completed_at` is
  the instant its own probation window opens.
- **The admitted set is derived, never declared.** A participant counts
  as sitting only because an earlier record says so, and priors are read
  in insertion order, so no admission can be retroactively legitimized
  by one appended afterwards. Compact formation is exempt, its
  participants being founders rather than entrants.

Two limits are worth stating plainly. First, "drawn by lot" is
not enforced. A verifiable lot needs a randomness beacon the
compact does not have, so terms, rotation, and composition are
enforced while selection fairness remains social. Second,
`compact_label` is a self-declared string, so a mislabelled
record reads as the formation of some other compact. It cannot
forge the roll of this compact, and a consumer reading by label
never sees it, which makes this accountability rather than
prevention.

### 3. Archive / commons of memory

- **Distributed append-only archive** of compact-level events:
  recognitions, re-affirmations, mediations, designations,
  restorations, exits, reconstitutions.
- **Replication.** Archive content replicated across member
  communities. The failure of no single community loses the
  record.
- **Sealing.** Threshold encryption for sealed mediation proceedings,
  with keys distributed across the mediator pool so no single custodian
  can unseal early. Deterministic reveal at the 25-30 year mark.
- **Access control.** Public records vs. sealed records vs.
  restricted records (e.g., reputation details that cross the
  surveillance threshold).
- **Manufactured-event provenance tagging.** Records advancing
  interpretations of high-charge events carry originator
  identity, declared incentives, information-history of how the
  interpretation emerged, and stake-disclosure entries. Used by
  the structured-skepticism protocol per
  [liminal_institutions.md "Manufactured-event
  resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).
- **Decision-class metadata.** Every compact-level decision record carries explicit
  classification (first-class, valid within light-lag. Second-class,
  requiring multi-community quorum signature) per
  [commons_federation.md "Light-lag-honest decision classes"](../background/commons_federation.md#light-lag-honest-decision-classes).
  The metadata is part of the constitutional layer
  (genesis-immutable as a classification scheme. Individual
  classifications modifiable only through second-class procedure).

### 4. Mediation

- **Mediator pool directory.** Identities, rotations, recusals,
  lineage of training. *All four are built.* Rotation is the same
  `check_roster_caps` the §2 examiners and §8 observers share. Bounded
  term, sit-out between terms, and no single community holding a majority
  of the active pool past a floor. A permanent mediator class is
  the first step toward an inquisitorial caste, and mediators must return
  home. Recusals are `RecusalRequest` and `RecusalDecision`. Lineage is
  `MediatorEntry.trained_by`, reported by the pool board and read by no
  rule.
- **Case intake.** Disputes filed between communities, classified
  by escalation-ladder rung.
- **Deliberation support.** Async-compatible for light-lag.
  Structured argument submission. Evidence packaging. Recusal and
  consent tracking. *Argument submission and evidence packaging
  are built* (`kc-mediation::ArgumentRecord`, one record per
  argument, self-signed, cited by the decision that answers it).
  *Recusal is built.* A party names one panelist on one dispute
  and states grounds, and the request is recorded and answered,
  granted only by the mediator stepping down, since nobody else
  may step down for them, and never answered by that mediator when
  the answer is a refusal. A request neither blocks the mediation
  nor unseats anyone by itself. An unanswered one stays visible.
  *Consent tracking is built.* Consent was recorded where it was
  structural (a protecting power is co-signed by the disputant it
  stands for and the community taking the duty, because standing
  for another community is never conscripted and a protector a
  disputant did not accept is a watcher imposed on them) and where
  it was refused, in an answered recusal request. The ordinary
  case had no record: `kc-mediation::PanelConsent` is a party
  saying that this panel, these mediators, may decide its dispute,
  self-signed because nobody consents on the behalf of another,
  and naming the panel in full so that consent to three mediators
  is not consent to a fourth.

  It gates one thing and deliberately no more. While consent is
  right for a settlement, a consent precondition on deciding at
  all would hand either party a veto over its own dispute, and
  the escalation ladder exists because parties disagree. But a
  `Settled` outcome is a claim that *the parties* reached a
  settlement, so both must have consented to the panel recording
  it. Measured before the rule. A panel recorded a settlement
  neither party had ever agreed to, and it was authorized. A
  party that refuses to settle blocks nothing: the panel
  upholds, dismisses or refers instead.
- **Interim-injunction issuance.** Protocol-level "stop" order with
  enforcement linkage (§9).
- **Decision publication.** Public decisions with mandatory
  reasoning trail. Sealed decisions with threshold-reveal. *The
  reasoning trail is now mandatory in fact*: the field existed
  from the first §4 slice and no rule read it, so a panel could
  publish a decision that said nothing about why. *Sealed
  decisions are built* (`kc-mediation::SealedReasoning` /
  `DecisionUnsealed`). What is sealed is the reasoning. The
  dispute, panel, and outcome stay public, so every rule already
  built on a decision keeps running over a sealed one, and a
  compact can see that a matter was decided, by whom, and how it
  came out. What waits is why. An unsealing must present enough
  opened custody shares to recover the text and that text must
  open the commitment of the decision, so a custodian quorum
  cannot publish a reasoning the panel never wrote.

  **The custodians are member communities, not the mediator
  pool, and they have to be.** The whitepaper asks for keys
  "distributed across the mediator pool so no single custodian
  can unseal early" and, in the same breath, for a deterministic
  reveal at the 25-to-30-year mark. Measured against the tier as
  built, those cannot both be had. §4 caps the seat of a
  mediator at two years, because a permanent mediator class is
  the first step toward an inquisitorial caste, so a share
  sealed to a mediator is held by a key the archive stops
  resolving two decades before the seal opens, so the signature
  on the opening cannot even be verified, and the deterministic
  reveal is dead by construction. A community identity resolves
  at any instant, which is what a generational custody needs.
  The substance of the doctrine is kept. No party may hold a
  share, and no quorum of shares may sit with the communities of
  the deciding panel itself, since a panel that can unseal
  itself has deferred publication at its own discretion rather
  than sealed anything. The cost, stated rather than hidden, is
  that a community has no separate encryption key, so a share
  seals to the key it also signs with, the compromise
  `MediatorEntry::encryption_pubkey` exists to avoid for §8,
  accepted here because the alternative is no reveal.
- **Structured-skepticism protocol invocation.** Mediator-pool
  authority to invoke the protocol on
  events meeting qualifying criteria (high interpretive charge,
  faction-aligned interest, sudden onset, originating from
  stake-holding witnesses). Imposes structural delay,
  multi-witness corroboration, cross-substrate translation, and
  stake-disclosure requirements. Mediator-pool internal review
  guards the invocation against pretextual use to suppress
  legitimate urgent matters. Specified in detail in
  [liminal_institutions.md "Manufactured-event resistance"](../background/liminal_institutions.md#manufactured-event-resistance-the-structured-skepticism-protocol).

  *The authority of the pool is built*
  (`kc-provenance::SkepticismInvocation`). The hold of the
  protocol had teeth from the first move-4 slice, since an
  action premised on a qualifying, under-corroborated event is
  refused, not merely flagged. But *qualifying* was four
  booleans the originating community set on its own report. So
  the teeth rested on the word of the party the protocol exists
  to constrain. Measured before the rule, an originator that
  declared its own event mundane was never held, and a full
  non-party mediator pool sitting in the same archive could do
  nothing about it. The invocation reaches exactly that event.
  It is co-signed by a panel that may not include the mediators
  of the originating community itself, must name the criteria
  the pool finds met and document its grounds, and carries a
  hold bounded at both ends. While the interval scales with
  interpretive charge, a nominal hold is no delay and an
  unbounded one is suppression.

  *The internal review is built* (`SkepticismReview`), and it is
  a review of the **hold** rather than of the invocation record.
  That is what lets it reach the self-declared case, which was
  the one that answered to nobody. A community could put its own
  urgent matter beyond action with no reviewer on the record
  anywhere. A `Pretextual` finding releases the event from a
  hold of either origin. Although an `Upheld` finding changes
  nothing, it exists because auditable invocations must be
  legible whichever way the review comes out. Mediators who
  co-signed the invocation may not review it, and reasoning is
  required on both answers. The `structured_skepticism` lens
  reports the criteria of the pool against the declaration of
  the originator, and flags a hold no panel has reviewed.

### 5. Escalation and designation

- **Ladder state machine** per dispute, implementing the canonical
  ladder of
  [bounded_conflict.md "Protocol sketch"](../background/bounded_conflict.md#protocol-sketch). Dispute registration → registered protest → reciprocal suspension
  of specified courtesies → mediated arbitration → limited material
  sanctions → compact-authorized collective sanctions → expulsion.
  Anathema designation and virtue-violation enforcement are separate
  tracks, not rungs. Timed transitions where the research specifies
  them.
- **Designation procedure.** Four-condition test (per
  [bounded_conflict.md "Handling the anathema"](../background/bounded_conflict.md#handling-the-anathema)),
  quorum rules, review period, emergency-variant handling. The
  *hostis humani generis* tier within anathema carries its higher
  evidentiary threshold.
- **Re-examination follow-through.** The review period is an
  absolute deadline, mandatory on the expedited path, and what
  the panel concluded is now a record of its own
  (`kc-escalation::ReexaminationConclusion`). A confirmation
  answers the deadline it was held for and sets the next one,
  since a confirmation renews the obligation rather than
  discharging it, and it may not be dated before the deadline it
  answers. Until this record existed, a panel that re-examined
  at the mandated date and let the designation stand left no
  trace and read exactly like a compact that had forgotten. The
  erosion lens punished the diligent panel. A designation whose
  re-examination has lapsed may not be **cited** by a further
  escalation act, which is the one refusal the remediation of
  the erosion audit carries (see
  [erosion-legibility.md](erosion-legibility.md)).
- **The covenant-facing floor of the expedited path.** An
  expedited designation must cite at least one attested adverse
  `ConductRecord` naming the subject and recorded no later than
  the designation itself. The normal path carries this
  structurally, since condition 1 resolves three restitution
  chains through `RestitutionVerified` and `ConductAttested`,
  both compact-facing record kinds. The expedited path relaxes
  condition 1 by design. Condition 3 checks only that `criteria`
  and `cited_acts` are non-empty without resolving the CIDs, so
  before this rule the emergency route was the one place a
  designation could be assembled out of records describing what
  a community does inside itself, or out of CIDs naming no
  archive record at all. Measured before the rule. An expedited
  designation citing `Cid::of_bytes(b"act")` was authorized, as
  were all four expedited fixtures in the suite. What the rule
  protects is [natural_law.md "The interior of a member
  community"](../background/natural_law.md#the-interior-of-a-member-community).
  The compact forms no verdict on the internal morality of a
  member until that morality touches the covenant, and the
  emergency route may shorten the pattern without dispensing
  with a covenant-facing act. The *gravity* of the cited act is
  not machine-checked and remains the judgment of the
  deliberators, the same design limit `sustained_bad_faith`
  carries for condition 1's "each more serious than the last".
- **Restitution tracking.** Restoration obligations, verification of
  completion.
- **Re-entry procedure.** Examination, witnessing, re-recitation
  binding on re-admitted party.
- **Substantive-re-engagement rung.** The new ladder rung between ordinary re-affirmation and anathema
  for ceremony-cost decay cases (per §2 above). Triggered by
  proxy-decay surfacing in the audit commons. Applies to a member
  community without designating it as anathema. Resolved by
  successful heavier re-affirmation cycle or by escalation to
  anathema if the heavier cycle also shows decay. State-machine
  position is: ordinary re-affirmation → substantive re-engagement
  → anathema designation.

### 6. Reputation bridges

- **Reputation ledger.** What is on the record across the compact.
  Recitation history, completed mediations, designations (active and
  lifted), restoration completions.
- **Selective-disclosure primitives.** Zero-knowledge proofs for
  narrow claims (e.g., "this community has no active designation
  against it") without exposing full history.
- **Decay model.** Not pure exponential. Step function for dramatic
  violations plus slow decay for minor. See
  `reputation_bridges.md` open question on half-life modeling.
- **Recitation history as foundational datum.** Compact-anchored
  reputation root, portable across mediators and disputes.
- **Surveillance threshold.** Configurable disclosure levels for
  internal health indicators. Large/powerful communities disclose
  more by default.
- **Principal-output triangulation data flows.** Comparison of the
  conduct of a machine against the ensemble of similarly-situated
  machines under similarly-positioned principals. Divergence flags
  entered in the reputation ledger for cross-community review per
  [m2m_interaction.md "Adversarial-principal
  validation"](../background/m2m_interaction.md#adversarial-principal-validation).
- **Ceremony-cost decay surfacing.** Reputation queries surface
  proxy-decay state
  alongside conventional standing data, so peers and observers
  can see ceremony erosion before it has progressed to formal
  re-engagement or anathema.

### 7. Commons governance

- **Commons registry.** Shared resources (knowledge archive, rescue
  capacity, communication relays, navigation beacons) with Ostrom-style
  boundary definitions.
- **Usage accounting.** Consumption, contribution, free-rider
  detection. *Sealed usage is built.* A commons may state a
  `draw_allowance`, which is `provision_rule`'s machine-readable
  half and the only part of Ostrom's proportionality a rule can
  act on. Before it, a record drawing ten times the written cap
  was authorized exactly like one that honoured it, and the two
  were indistinguishable to the archive. A community may then
  account under the §7 surveillance threshold rather than in
  public, committing its figures and publishing a
  **zero-knowledge proof that the draw is within the
  allowance**, with the figures themselves under threshold
  custody keyed to fellow appropriators. A community that must
  publish every figure to account honestly is disclosing its
  capacity, its consumption pattern and its reserves to every
  other member, which is what the threshold exists to bound.

  Sealing is not a way to disappear from the detector, and it took a
  measurement to see that it would have been. `free_rider_signals`
  summed the figures, so a community sealing 1,850 units read as
  `consumed: 0` and ranked nowhere: not merely absent but apparently
  the most scrupulous member of the compact. The lens now counts
  sealed records separately, reports whether the accounting is
  complete, and turns each proof into a bound on what is hidden, so
  the totals are honestly a floor rather than dishonestly a figure.

  The proof is bit-decomposition with a Chaum-Pedersen OR-proof per
  bit, about 15 KB per record, built on the generator and hashing
  `kc-core` already carries for §8. It is more hand-rolled
  cryptography on a tree that has none externally reviewed, and the
  review item in `TODO.md` says so.
- **Graduated-sanctions ledger.** Progression from monitoring to
  warning to restriction to exclusion, with restoration paths.
  *The commons equivalent of the anathema designation is built.*
  `commons_federation.md` lists it under open questions. The
  ladder and the §5 designation track both existed and nothing
  joined them, so a community could be excluded, restored and
  excluded again across years and reach the designation track by
  no commons-specific route. `kc-commons::CommonsEscalation` is
  the threshold, specified in the shape the §5 four-condition
  anathema test carries. The subject is at `Exclusion`, has
  arrived there more than once (defiance rather than compliance,
  which is the distinction the source document draws), has stood
  there long enough that restoration was available and not
  taken, and one arrival suffices on a life-critical commons,
  which the compact keeps running when it pauses everything
  else. The finding gates nothing by itself. It is what a §5
  designation may cite, so the ladder is untouched and the
  threshold lives in the refusal to record it.
- **Light-lag-tolerant consensus.** CRDT-like where semantics permit;
  BFT-style where they do not. Eventually-consistent governance is
  the baseline assumption (per `commons_federation.md`).
- **Nested-enterprise support.** Hierarchical commons
  (community-level → regional → compact-wide) with appropriate
  authority at each level. *The inter-compact level is built*,
  which `commons_federation.md` had left as "a forward pointer".
  Nothing at that level could be said while a compact had no
  identity. The only compact-level name in the tier was a
  self-declared string, so two witnesses could write the same
  label and mean different compacts. `kc-identity::CompactCharter`
  gives a compact a DID of its own, co-signed by a quorum of its
  member communities **and** by the key of the compact itself.
  That asymmetry with `CommunityIdentity`, which is self-signed,
  is the content of "a higher nesting level" as this tier realizes
  it. A community exists by its own act, and a compact is
  constituted by its members. A `CommonsRegistration` then carries
  an appropriator level, and at the compact level its
  appropriators are whole compacts.

  What makes the nesting real rather than decorative is one
  rule: **the inter-compact level may not reach past a compact
  to its member communities.** A sanction or escalation on such
  a commons names a compact, never one of its members. If a
  federation could discipline a member community directly, it
  would be governing at the scale of the compact while answering
  to nobody at it, which is the sovereignty drift
  `liminal_institutions.md` warns about arriving one level up.
  Measured before the rule. Nothing stopped it, because nothing
  distinguished the levels at all.

  The habitat level remains autonomous_trust's, and the compact of
  compacts is not itself a further governance tier here. What is
  built is the identity of the level, its commons, and its limit.
- **Decision-class enforcement.** Cryptographic enforcement of
  the first-class / second-class distinction (per §3 metadata
  above). Second-class decisions produce signed records whose
  validation requires multi-party signature from a quorum
  spanning multiple communities at multiple distances. The
  quorum is tuned so no faction in a single AU-region can
  satisfy it alone. Signatures must arrive within a defined
  latency window. Validation failure is surfaced automatically
  in the audit commons. The classification of which decisions
  fall into which class is a constitutional-layer commitment.
  Reclassification is itself a second-class decision (the
  classification is self-protecting in this specific structural
  sense). *Reclassification is built* and has been since the §9
  slice: `kc-constitution::CompactIncapableAmendment` with
  `AmendmentKind::{Add, Remove}` moves a matter into or out of
  the compact-incapable list, genesis matters excepted, with the
  harder bar on contraction. It was deferred in the §7 slice and
  built in the §9 slice, and the deferral note in `kc-commons`
  outlived the deferral by several slices, from which the
  tracker copied it. The item read as outstanding for as long as
  nobody checked. Specified in detail in [commons_federation.md
  "Light-lag-honest decision
  classes"](../background/commons_federation.md#light-lag-honest-decision-classes).

### 8. Jester and observer function

> **Constitutional standing.** The
> jester / observer function is a **substrate-level capture detector**
> rather than a cultural-dissent affordance. Its permission
> to name what others cannot is what makes early-stage capture
> legible before terminal capture: contraction of substrate-deference,
> decay of ceremony cost, manufactured-event drift, principal-capture
> signatures. The function operates at the substrate layer (continuous
> monitoring of capture signatures) as well as the protocol layer
> (post-hoc reporting). Per
> [capture_resistance.md "Architectural unification"](../background/capture_resistance.md#architectural-unification),
> the function holds a constitutional position, and its implementation is
> not separable into a later phase.

- **Reporting cadence.** A community holding observer seats
  accounts for each window, **including an account of having
  seen nothing** (`kc-conduct::ActivityAttestation`). Absence
  has to be signed for, or a corps that has stopped looking
  cannot be told from a corps with nothing to see. They file the
  same number of reports, namely none. The same record and the
  same cadence serve the §4 mediator pool, since the obligation
  and the failure are identical on the two sides. Read by
  `silent_functions`, and by `unstaffed_functions` for the
  sharper case of a corps that no longer exists at all.
  Reported, never refused.
- **Observer corps registry.** Declared-perspective reporters,
  rotation, cross-community composition requirements to prevent
  narrative capture by the dominant member.
- **Report publication** with attribution (public reporting) alongside
  whistleblower channel with source protection.
- **Threshold-held source records.** Whistleblower identity
  recoverable only with mediator-panel quorum and only for named
  cause. Source protection must be structurally stronger than
  individual mediator integrity.
- **Archive access.** Observer reports form a durable check on the
  mediator pool. Archive must make observer corpus retrievable across
  generations.
- **Capture-signature monitoring.** The observer corps maintains
  active monitoring for the legible signs of early-stage
  capture: substrate-deference erosion (shrinking
  compact-incapable list, shortening structured-fast period),
  ceremony-cost proxy decay across multiple member communities,
  manufactured-event patterns, principal-output triangulation
  divergence. Though a filing is the visible event, monitoring
  outputs are reported to the audit commons on a continuous
  cadence rather than only when specific reports are filed.
- **Substrate-deference reporting.** Observer corps maintains
  the public dashboard tracking compact-incapable list state and
  structured-fast cadence observance, making the operational
  practice of the substrate-deference test continuously legible
  to the compact members.
- **Composition-drift dashboard.** The observer corps maintains an
  admissions dashboard alongside the substrate-deference dashboard.
  Who sponsored, who examined, which admissions clustered by
  region, lineage, or sponsor. Clustering in directions that
  benefit an identifiable faction is flagged before the faction
  reaches deliberative weight.

### 9. Enforcement model

This is the load-bearing gap the research docs repeatedly punt on. Three
candidate mechanisms, layerable:

- **Reputation + directory consequences.** Ignored injunction →
  automatic restricted-member status until mediated. Software-enforced
  at the directory level. Reputation lookups surface the violation. The
  injunction and its defiance are published to the archive
  automatically.
- **Capability-based access.** Compact-gated resources (rescue
  coordination endpoints, shared navigation, reputation-dependent
  trade, commons withdrawals) are revocable at protocol level when a
  community ignores an injunction. The teeth are withdrawal of
  protocol-level services, not external coercion.

  As built, a capability is a **service** or an **office**, and
  the two are revoked independently. The services are the
  relations of exchange just listed. The offices are the roles
  that select who is sanctioned: a mediator-pool seat, an observer
  or jester seat, an examining seat, deliberative weight in a
  second-class decision, a seat on an emergency panel. Without the
  second kind, an injunction could suspend every relation of
  exchange a community held and leave untouched every role by
  which it judged others, which is the option the design wanted
  and could not express. What may trigger the withholding stays
  bounded by the covenant. An in-force injunction is the only
  thing that revokes a capability, and an injunction answers to
  conduct at the boundary. Holding a community out of office for
  what it does at home would be a verdict on the interior of a
  member, which
  [natural_law.md](../background/natural_law.md#the-interior-of-a-member-community)
  forbids the compact to form.
- **Transparency-as-teeth.** Violations are published immediately
  and durably. The reputational and material cost is downstream of
  publication. The job of the protocol is ensuring publication is
  reliable and unforgeable.

Kith-covenant is structurally sovereign-less (per `historical_cases.md` pattern
3), so enforcement must be collective and non-sovereign. The above three are the
realistic options at the code layer. External state coercion is not in scope.

**Substrate-deference test as ongoing constitutional
operation.** Beyond the post-hoc enforcement responses above,
enforcement now also encompasses the **continuous demonstration
of non-ultimacy** by the compact itself. This is enforcement of
the compact constitution against compact drift itself, performed
by the standing operation of the compact rather than by an
external force:

- **Compact-incapable list maintenance.** The constitutional
  list of decisions the compact will not make (the internal
  theology of member communities and governance, content of
  recognition statements after compact-formation, truth-claims
  of the tradition of any member, etc.) is maintained as a
  versioned, signed constitutional artifact. Modifications
  require second-class decision procedure. Expansions are easier
  than contractions procedurally, reflecting the asymmetric
  capture risk.
- **Structured-fast cadence enforcement.** During defined periods
  (the compact analogue of liturgical Lent / sabbatical *shmita*),
  compact-level mediations, sanctions, and decisions are not
  initiated. Life-critical commons proceed. The deliberative and
  adjudicative functions pause. The cadence is part of the
  constitutional layer. Observer corps surfaces deviations.
- **The adopted minimum content as a protected artifact.** The thin
  functional minimum the compact has adopted as the ground of its own
  conduct where consent has run out
  ([natural_law.md](../background/natural_law.md#the-ground-the-compact-claims-for-itself))
  is a second constitutional artifact, adopted once by quorum and
  amendable only by second-class procedure. It is **not** a row on the
  compact-incapable list and may not be filed as one. That list names
  matters belonging to a member, and these forbearances bind the
  compact. Two differences from the list are load-bearing. Narrowing
  the ground carries the same asymmetric higher bar as contracting the
  list, at every one of the four sites. And a *founding* forbearance
  is contractible at that bar rather than immutable, because the
  pressure the doctrine defends against is an argument that is always
  available and always pragmatic, which a bar answers and a lock does
  not.
- **Erosion-detection alerting.** The audit commons surfaces
  trends in any of the three dimensions (exceptions multiplying
  in the compact-incapable list, forbearances narrowed out of
  the adopted ground of the compact itself, the structured-fast
  period contracting under pragmatic argument) to the observer
  corps and to the member communities. The narrowing signal is
  the one with no constituency inside the compact to raise it.
  Because nobody is asked to endorse the minimum content, no
  member holds it as a commitment to defend when the argument
  for narrowing arrives. Even so, the bar on contraction stands.

The substrate-deference test is operational and diagnostic (both
functions are live in the continuous performance of the test).
Specified in [justus_hostis.md "Non-ultimacy of the political
community"](../background/justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient)
and developed in [capture_resistance.md "Substrate-deference
test"](../background/capture_resistance.md#2-the-substrate-deference-test-operationalizing-non-ultimacy).

### 10. Bootstrapping and test harness

- **Simulation environment** for adversarial testing:
  captured-mediator scenarios, hollow-recitation drift,
  coordinated collusion, narrative capture of the observer
  corps, protocol-layer replay attacks. The seed already exists
  as negative-path coverage the protocol must refuse. The
  `kc-core` payload-tamper test, the `kc-archive` §5 authority
  tests (wrong-scope delegate, cross-community forgery, quorum,
  and panel failures), the Python `test_authority.py` /
  `test_mediation.py` (panel-too-small, party-mediator,
  unregistered-mediator) / `test_standing.py` (declining rites,
  revoked delegation), and the `capture_signals_demo.py` /
  `mediation_demo.py` demos, which construct archives the
  protocol must reject. The harness generalizes these
  hand-authored refusal cases into a time-evolving simulator
  rather than a fixed pass/fail set.
- **Earth-pilot configuration.** Small-scale deployment mode that can
  actually run with real communities. Necessary for the test-cases gap.
  First compacts should be among Earth-bound communities with existing
  ties and cheap exit (denominational networks, diaspora federations,
  tribal confederations, ecological commons, cooperative-firm networks)
  generalized to space only after Earth has been the harder first test.
- **Integration harness** against `autonomous_trust` for live-AT
  message traffic observation, per
  [policy-evaluation.md](policy-evaluation.md).
- **Capture-resistance scenario simulation.** Adversarial test categories
  specifically target the seven strengthening moves:
  - faction-monopolizing-interpretation within a member community (move 1)
  - compact drift toward ultimacy via compact-incapable-list erosion (move 2)
  - ceremony hollowing across simulated generational cycles (move 3)
  - manufactured-event injection with timed loudest-interpreter pressure (move 4)
  - light-lag fait-accompli attack with forged quorum signatures (move 5)
  - principal-capture by external manipulation, including AI-mediated influence
    operations (move 6)
  - admissions capture by patient sponsorship of aligned entrants toward the
    collusion threshold (move 7)

  Each scenario is run continuously in the simulation
  environment, with detection latency and damage bounds as the
  primary metrics.

**As built. Continuous execution is parameter variation, not
iteration.** The item read as though a scheduler was the missing
piece, and measuring it found otherwise: `run_harness()` is a
**constant function**, 200 consecutive runs returning
byte-identical latency and damage for every scenario. A loop
around it recomputes the same numbers forever. Finally, what the
harness gained was `HarnessParams` (whose `canonical()`
reproduces the hand-authored case byte-for-byte, as a fixed
point) and three drivers over it: `run_continuous`, taking
seeded draws and accumulating the worst observed latency and
damage per scenario **together with the seed that produced
each**; `sweep` / `sweep_all`, walking one parameter through its
whole declared range so that a moving bound becomes a reported
number; and `shrink_failure`, minimizing a failing parameter set
back toward canonical. There is no daemon and no scheduler. The
calls are bounded and return, because the cadence belongs to the
caller.

That work also corrected the criteria themselves. Several were
not properties of a detector at all but encodings of their
fixture. One damage bound held only because a hand-written
witness sequence declined on its first comparable cycle, so a
flat prefix scored worse with **no detector at fault**, and
another was literally its own detection threshold. "Criteria
stated in advance" therefore became *state the **formula** in
advance*, with the old constants retained as their
canonical-case values and a regression test that restores each
old constant and confirms it now fails. Although a search over
400 randomized rounds plus a full sweep of every declared range
finds no failures, that is meaningful only because the machinery
demonstrably can fail. The suite breaks a criterion on purpose
and checks that the failure is caught, shrunk, and replayable
from its seed. Demo and suite use a fixed seed, so a search can
never redden the build. The honest limit is that a seed
reproduces the *parameters* and not the key material, Ed25519
keys coming from the OS CSPRNG, so key-order independence is a
test rather than an assumption.

**Falsification criteria.** What observable outcomes would count as
kith-covenant failing must be stated in advance, per component and at the
whole-compact level: explicit design, not implicit. This is the least-built of
the operational gaps and the precondition for any pilot.

The top-level criterion is **retention despite exit** under weak
forcing, per [bounded_conflict.md "Earth
first"](../background/bounded_conflict.md#earth-first-the-harder-test).
Whether communities keep choosing the recognition, mediation,
and commons of the compact when leaving is cheap and the state
system is available. If a compact holds when exit is free, it
has demonstrated that it binds voluntarily. Earth is therefore
the harder first test, not a rehearsal for space.

The standing caveat is that **"the teeth" (sanction and enforcement
effectiveness) are structurally untestable on Earth**, where a determined
defector simply leaves. An Earth pilot that fails to demonstrate enforcement has
not thereby falsified the framework. It has reached the edge of what Earth can
test, and pilots must not be scored on it.

Per-component criteria:

- **Injunction enforcement (§9).** Success. Defiance of an interim
  injunction is published unforgeably and in a form that downstream
  reputation and capability consequences can act on, without a sovereign.
  Failure. An order that can be ignored with no archived trace, a paper
  order.
- **Mediator pool (§4).** Success. Panels stay non-overlapping and
  un-captured as the pool rotates through cases, and disputing communities
  keep accepting panel decisions when they could walk. Failure tripwire.
  The emergence of a permanent mediator class, the inquisitorial caste the
  term limits exist to prevent.
- **In-person recognition vs. light-lag (§2).** Success. Even at
  light-lag, a remote generational re-affirmation still carries
  real cost, treated as consequential rather than as a
  formality. Failure. Remote rites drift into formality while
  in-person ones stay costly, so the filter does its work only
  where it is least needed.
- **Re-affirmation cadence (§2, §5).** Success. The cadence catches
  substrate drift before it reaches anathema without the rite decaying into
  an empty ritual. A cadence that flags nothing and one that flags
  constantly are both failures.

### 11. Capture resistance (cross-cutting)

Although it is catalogued here as §11, this is not a separate
component but a unified architectural concern that runs through
§§1-10. The full statement is in
[capture_resistance.md](../background/capture_resistance.md).
The specifications for its individual moves live in their host
research documents. The role of this section is to give the
concern explicit constitutional standing in the software
decomposition, route the responsibilities to components, and
name the implementation work that follows.

**The unified architectural commitment.** Capture resistance is
a **continuous operating concern** of the substrate, the
protocol, and the implementation, woven through the ordinary
practice of the compact rather than invoked only when capture is
suspected. The prior fragmentation (jester role for cultural
dissent. Audit commons for forensic record. Emergency anathema
for crisis response) is unified under this commitment, with the
constitutional position of each fragment upgraded as specified
above (§§1-10 inline).

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
layers: substrate, compact, and implementation. Substrate is the
practices of member communities themselves, meaning interpretive
plurality and ceremony cost. Compact is the mechanisms of
kith-covenant, meaning substrate-deference, manufactured-event
resistance, decision classes, and jester-as-detector.
Implementation is the software: audit commons as detection
infrastructure, cryptographic decision-class enforcement,
attestation cadence tuned to capture-detection windows). Each
layer presupposes the others. Capture-resistance only works as a
layered system.

**What this section requires of the implementation.** These items are
first-class implementation work, with priorities calibrated to the
capture-resistance test harness. Each is carried by the code named beside it:

| Item | Component(s) | Where |
|---|---|---|
| Cross-community principal-witness attestation channel | §1 | `kc-principal` (`PrincipalWitnessAttestation`); `distinct_cross_witnesses` excludes self and home |
| Ceremony-cost proxy tracking and surfacing | §2, §3, §6 | `kc-reputation` (`ceremony_cost_decay`) |
| Substantive-re-engagement state machine rung | §5 | `kc-escalation` (`SubstantiveReEngagement`, the terminal rung of the ladder) |
| Manufactured-event provenance tagging in archive | §3 | `kc-provenance` (`EventReport` carries declared incentives, information history, disclosed interests) |
| Decision-class metadata and cryptographic enforcement | §3, §7 | `DecisionClass` on the record. Enforced in `kc-archive::authority` by counting *distinct communities* verified signatures, so a faction cannot supply a quorum by asserting one |
| Sealed usage with a proof of allowance | §7 | `CommonsRegistration.draw_allowance` gives `provision_rule` a machine-readable half, and `SealedUsage` commits the figures with a bit-decomposition bound proof (`kc-core::pedersen`, ~15 KB) plus threshold custody keyed to fellow appropriators. `free_rider_signals` counts sealed records rather than dropping them, so sealing bounds what a reader cannot see instead of hiding it |
| Commons equivalent of the anathema designation | §7 | `CommonsEscalation` and its four conditions: at `Exclusion`, arrived there more than once, stood there past the restoration window, and a lower bar for life-critical commons. Cites specific sanctions, and gates nothing itself: a §5 designation may cite it |
| Nested inter-compact enterprises | §7 | `kc-identity::CompactCharter` (a compact DID constituted by its members and its own key) plus `AppropriatorLevel` on the registration, and the subsidiarity rule: the higher level names compacts, never their member communities |
| Structured-skepticism protocol invocation | §4 | `SkepticismInvocation` + `SkepticismReview`, plus the hold on `EventActionInvoked`: an action premised on an under-corroborated qualifying event is refused, not merely flagged. Either the declaration of the originator itself or the invocation by the pool places the hold, and a `Pretextual` review releases it from either. The invocation is what reaches an event whose originator declared it mundane, and the review is what makes a hold answerable to somebody. The `structured_skepticism` lens reads both, so a pool-held event is no longer invisible to it |
| Consent to the panel as constituted | §4 | `PanelConsent`, self-signed, naming the panel in full. Gates a `Settled` outcome and nothing else: requiring consent to decide would hand either party a veto, but a settlement the parties did not consent to is not a settlement |
| Sealed decisions with threshold-reveal | §4 | `SealedReasoning` + `DecisionUnsealed` over `kc_core::ThresholdCustody`, the §8 primitive at its second use. Although the mediators are the parties who decide, custodians are **communities**, because the §4 two-year mediator term cannot carry a twenty-five-year seal. No party holds a share and no quorum sits with the communities of the panel itself |
| Mediator-pool rotation and share cap | §4 | `MediatorRegistry` + `MediatorCaps`: bounded terms, rotation gap, continuous-service union, no-majority pool share, the same `check_roster_caps` the §2 examiner and §8 observer caps read. Until it existed the mediator pool was the third roster and the one wired to nothing, so one community could hold nine of ten seats and every mediator could be seated with no end date at all. The *draw* stays social: one panel may still be composed entirely of mediators from a single non-party community whose share of the pool is legal, as with drawing examiners by lot |
| Recusal requested and answered | §4 | `RecusalRequest` + `RecusalDecision`. Grounds are required and never weighed for content, since a rule can record that a ground was stated, not judge it. Granted only by the mediator stepping down. A refusal belongs to the panel, never to that mediator. A declined request must state reasoning. It gates nothing: the mediation decides as it would have |
| Protecting powers | §4 | `ProtectingPower`, the Geneva analogue from [liminal_institutions.md](../background/liminal_institutions.md). A non-party community standing for a disputant, co-signed by both, barred from seating mediators in the same dispute. "It is not an arbitrator. It is a witness with standing", so it gates nothing |
| Mediator training lineage | §4 | `MediatorEntry.trained_by` plus the lineage half of `mediator_pool_concentration`: per-trainer counts against the *recorded* denominator, so a pool that records nothing reads as unknown rather than healthy. Read by no rule, because the same doctrine warns against a certification barrier becoming a gatekeeping mechanism |
| Substrate-deference dashboard maintained by observer corps | §8 | `substrate_deference` + `erosion_alert`, bundled on `observer_dashboard` |
| Attested nulls for the standing functions | §4, §8 | `kc-conduct::ActivityAttestation`: a signed account of one window, whose **empty** act list is the purpose of the record. Silence becomes either evidence or a fact about the compact rather than missing data. Signed by the community that holds the seats, since the obligation is theirs for as long as it does |
| A reporting cadence read against a clock | §4, §8 | `silent_functions` (no act and no attested null inside the cadence) and `unstaffed_functions` (a function once staffed and now empty), both absolute, both on `erosion_alert`. The cadence sits inside the shortest roster term, or the silence reading is unreachable for that function |
| Designation re-examination follow-through | §5 | `kc-escalation::ReexaminationConclusion` plus the gate on citing a lapsed designation. Closes the confirm-leaves-no-record limit the erosion audit stated rather than papered over |
| Compact-incapable list as constitutional artifact | §9 | `kc-constitution` (`ConstitutionGenesis`. Genesis-immutable matters, asymmetric amendment) |
| Structured-fast cadence as protocol-enforced operating mode | §9 | `FastPeriod` + `in_structured_fast`. Hard refusal *during* a fast is a documented non-goal (a fast is a disposition, not a compliance regime) |
| The adopted minimum content as a protected artifact | §9 | `kc-constitution::MinimumContentAdoption` (a singleton, keyed to the four sites) + `MinimumContentAmendment`, whose `Narrow` carries the asymmetric bar and counts *weight*. Deliberately not a row on the compact-incapable list, and deliberately without genesis-immutability: a founding forbearance yields at the higher bar, since the doctrine asks for a bar and not a lock. `minimum_forbearance` is the self-binding gate beside `compact_incapable`, and a narrowing is a contraction on `substrate_deference` and its own dimension on `erosion_alert` |
| Offices distinguished from services in the capability set | §9 | `Capability::offices()` beside `Capability::services()`: `MediatorSeat`, `ObserverSeat`, `ExaminerSeat`, `Deliberation`, `EmergencyPanelSeat`, gated at the three rosters, both emergency panels, and inside `deliberative_weight` itself. Before it, an injunction revoking every capability the compact could express left all three rosters open and every panel seat available. The trigger is an in-force injunction and nothing else, so the withholding is covenant-facing and never a reading of the interior of a member |
| Composition-drift dashboard maintained by observer corps | §8 | `composition_graph` + `rejection_pattern` + `observer_pool_concentration`, bundled on `observer_dashboard` |
| Capture-resistance scenario simulation in test harness | §10 | `harness.py`: ten scenarios scored against criteria stated in advance, run continuously over a parameterized space. Two of the ten have no adversary (the quiet polity, the abandoned office), which is what the omission signals need |
| Sponsor-stake tracking | §2 | `SponsorshipRecord` + sponsor concentration in `composition_graph` |
| Probationary-status enforcement | §2 | A probationary community may not sponsor an entrant, may not seat mediators or examiners, does not count toward `panel_min` on any of the five panels, and carries no weight toward **any second-class quorum**, including the asymmetric contraction bars, which count weight rather than signatures. `kc_archive::probationary_at` is the one predicate both the gate and `RecognitionStanding` read; `deliberative_weight` is the one counter every bar reads. `deliberative_capacity` reports what the compact can still do |
| Credentialing-panel rotation | §2 | `ExaminerRegistry` + `ExaminerCaps`: bounded terms, rotation gap, continuous-service union, no-majority pool share, one implementation shared with the §8 observer caps. An admission into a standing compact must be examined by `panel_min` seated examiners spanning two or more communities. Drawing *by lot* is not enforced |
| Admission as a second-class decision | §2 | An admission into a standing compact needs an admitting quorum of sitting, non-probationary members, so one member plus the co-signature of the entrant itself no longer seals one |
| Sponsor stake required of an admission | §2 | Every entrant must arrive vouched for. Without it probation is opt-in, since a faction that never vouches opens no window to de-weight |

The last two items come from `capture_resistance.md` §7 rather than from the
per-component notes above, and they are what make the two probation rows bite. A
probationary community that carries no weight in admissions needs an admissions
quorum to be excluded from, and a probation window has to be opened by somebody
before there is anything to withhold. Hence the standing hazard in deriving such
a list from other status lists: it reproduces their omissions. The one element
here the protocol specifies and the code does not carry is drawing examiners *by
lot*.

Item 7 of the seven moves (admissions integrity) therefore has both halves: its
detection half (the composition-drift dashboard, sponsor concentration, the
rejection pattern) and its enforcement half. Harness scenario 7 scores it on
refusal rather than legibility: a faction acting alone seats exactly one
community, its co-founder at formation.

Scope this document specifies that the code does not yet carry is listed in
[`TODO.md`](../../TODO.md).

## Architectural decisions

1. **Distributed substrate.** Libp2p-style peer-to-peer with
   content-addressed storage. No distinguished server. No mechanism
   that becomes a single point of failure or a sovereign-in-disguise.
2. **Cryptographic identity.** Decentralized identifiers (DID-style)
   with community-level key hierarchies. Quantum-resistant primitives
   at the root layer.
3. **Consensus model.** Compact-wide facts (recognition, designation,
   re-entry) are rare but need durability. Most mediations are local
   to a panel. Probably *not* a single global chain. More likely
   per-case chains anchored periodically to a slower, higher-threshold
   compact-level commit.
4. **Amendability tiers.** Constitutional layer (recognition structure,
   common virtues, substrate requirements) is genesis-immutable.
   Legislative layer (operational rules, mediator procedures,
   escalation specifics) is adaptable by mediator consensus. Direct
   mapping to code. Different immutability regimes per record class.
5. **Integration seam with autonomous_trust.** AT exposes upward.
   Community-key identity, delegated speakers, honest health
   indicators at the configured disclosure level, witness availability.
   Kith-covenant never reaches inward. See
   [policy-evaluation.md](policy-evaluation.md) for the current seam.
6. **Substrate-check at recognition, not enforcement.** The compact
   does not programmatically verify substrate claims. It verifies
   *recitation*. Substrate authenticity is surfaced over time through
   the asymmetric-psychological-cost mechanism (per
   `recognition_protocols.md`), not by code audit.
7. **Capture resistance is layered and cross-cutting.** Per
   [capture_resistance.md](../background/capture_resistance.md),
   capture resistance operates at the substrate layer (member the
   practices of the communities themselves), the protocol layer
   (the mechanisms of kith-covenant), and the implementation layer
   (software). No single component implements capture resistance.
   Each component carries responsibilities specified inline above
   (§§1-10), and §11 catalogues the unified concern. The
   implementation must refuse the temptation to consolidate
   capture-resistance into a single subsystem. The consolidation
   itself becomes a capture target. Distribution of the concern
   across components is the structural defense.

## Component dependencies

The decomposition has a dependency order, and two of its constraints are
constitutional rather than merely practical.

- **Identity, recognition and archive (components 1, 2, 3) come
  first.** While the compact does not exist until recognition is
  real and verifiable, the smallest thing that can be tested is
  two communities in a room. Identity carries the
  cross-community principal-witness channel (§1). Recognition
  carries the interpretive-plurality evidence and the
  admissions-integrity machinery, meaning panel rotation,
  sponsor stake and probationary status (§2). The archive
  carries decision-class metadata and provenance tags (§3). All
  three are far harder to add afterwards than to design in.
- **Reputation lookup (component 6) depends on recognition events already being in the
  archive**, which is what makes a downstream reputation claim mean anything.
  Reputation surfaces ceremony-cost proxy state. Principal-output triangulation sits a
  layer above it.
- **The autonomous_trust seam (§5) is independent of the governance components.** It is
  what lets a member community operate its AT instance with kith-covenant presence.
- **Mediation and escalation (components 4, 5) precede any dispute being filed.**
  Mediation invokes the structured-skepticism protocol. The escalation ladder carries
  the substantive-re-engagement rung between ordinary re-affirmation and anathema.
- **The enforcement model (component 9) shapes components 1-4 in retrospect**, so its
  decisions belong before them rather than after. Enforcement carries the
  substrate-deference test as an ongoing constitutional operation: the
  compact-incapable list, the structured-fast cadence, and erosion-detection alerting.
  These are constitutional-layer commitments, and they must be settled before a
  compact is instantiated.
- **Commons registry (7) and observer function (8) layer on once the core protocol
  runs.** Commons enforces decision class cryptographically, through a multi-party
  quorum signature for second-class decisions. The observer function is a
  substrate-level capture detector, holding the substrate-deference dashboard, the
  composition-drift dashboard and capture-signature monitoring as standing
  competencies rather than optional features.
- **The capture-resistance harness (component 10) is continuous rather than
  sequenced.** It exercises the implementation from the first component onward and
  accumulates scenarios as the rest arrive. The seven adversarial scenarios are
  faction-monopolizing-interpretation, compact drift toward ultimacy, ceremony
  hollowing, manufactured-event injection, light-lag fait-accompli, principal capture,
  and admissions capture.

Capture resistance is not separable into a later phase. Several of its
responsibilities (decision-class metadata, the principal-witness channel, the
audit-commons provenance schema, and the substrate-deference list as a
genesis-immutable artifact) are constitutional-layer commitments that must be
present at compact formation. Adding them afterwards is either impossible, since
genesis-immutability would be violated, or operationally fraught, since existing
records would need a migration that is itself a capture surface.

## Open design questions

Several hard decisions remain unresolved and are flagged here rather than buried
in subsystem docs:

- **Enforcement model specifics.** Which of the three candidates in §9
  is primary? The research docs prefer transparency-as-teeth.
  Capability-based access is the strongest at the code layer.
- **In-person requirement vs. interplanetary scale.** The hybrid
  proposal in §2 is unvalidated. The 3-20 minute
  Earth-Mars light-lag may be tolerable. Multi-hour delays to outer
  system are not.
- **Re-affirmation cadence calibration.** The 25-30 year default lacks
  empirical support. May need to vary by community generational
  rhythm (monastic vs. agricultural vs. other) per
  `recognition_protocols.md` open questions, which argues for a
  compact-formation parameter with per-community override rather than a
  constant. A second, prior decision: is the cadence *enforced* (an
  overdue or hollowing re-affirmation does something on its own, expiry,
  escalation) or *observed only* (the `ceremony_cost_decay` lens surfaces
  decay and the jester and mediators act, but nothing triggers
  automatically)? The protocol as specified observes.
- **Mediator training pipeline.** Who trains the first generation?
  Is kith-covenant responsible for competence certification, or does
  each member community supply its own mediator candidates to a
  shared assessment process?
- **Minimum viable compact size.** Below some threshold, rotation and
  recusal become infeasible (every mediator is conflicted). Above
  some threshold, coordination fails. Where is the band?
- **Integration with existing infrastructure.** DIDs, IPFS, libp2p,
  threshold-cryptography libraries, CRDT libraries. Which stacks are
  mature enough to build on, which need to be built or forked?

## Relationship to other documents

- [policy-evaluation.md](policy-evaluation.md). Existing architecture
  doc covering the AT/kith-covenant evaluation seam. The integration
  point for §5 above.
- [../background/](../background/). The research set this
  document operationalizes. Each component above corresponds to
  one or more design commitments in the research docs.
- [../kith_covenant.md](../kith_covenant.md). Parent index.

---

*Next: [Machine participation](machine-participation.md)*
