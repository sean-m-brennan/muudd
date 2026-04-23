*[Kith-Covenant](../kith_covenant.md) > [Research](./) > Machine-to-Machine Interaction*

# Machine-to-Machine Interaction: Principal-Derived Contact Across Compact Boundaries

## Thesis

Machine-to-machine interaction in kith-covenant is always and only
*principal-derived*, never *machine-original*. Two machines meeting
across community lines is two named humans meeting through delegates;
the machines are the medium of the encounter, not its authors. Two
claims follow and together constitute the structural commitment of
this document. The **reduction claim**: every M2M exchange reduces,
without remainder, to an exchange between the two named humans whose
guardianship binds each machine — any attempt to describe the
interaction as machine-original is a description that has lost
information. The **non-emergence claim**: nothing originates at the
M2M layer that the principals did not authorize; the protocol makes
machine-original substrate structurally impossible rather than merely
forbidden. These are hard commitments. The rest of the document
develops what they require.

## This document's role

This document is the M2M-layer sibling of
[`../architecture/machine-participation.md`](../architecture/machine-participation.md),
which specifies the single-machine / single-guardian coupling
(constitutive objective, cryptographic self-binding, continuous
attestation, principal-referential identity + audit commons). Machine
participation develops the *within-community* shape of the
machine-human binding; this document develops what happens when two
such machines meet across the AT/KC boundary that
[`reputation_bridges.md`](reputation_bridges.md) specifies, within
the commons architecture that
[`commons_federation.md`](commons_federation.md) specifies, under the
recognition categories that
[`recognition_protocols.md`](recognition_protocols.md) supplies.

The scope is cross-compact, cross-community M2M contact under
light-lag. In-community M2M — two machines serving the same
guardian, or guardians within the same autonomous-trust scope — is
a degenerate case AT's intra-community mechanisms already handle.
The presupposition throughout: the single-machine binding described
in machine-participation.md is intact on both sides of the
encounter. Where it is not, the encounter is not M2M within the
compact's frame at all; it is a boundary event for which the
anathema procedure is the proper instrument.

## The reduction claim

*Within the authority envelope, the M2M channel is transparent to
substrate — it adds no legitimacy and subtracts none, but it must
not launder either. Outside the envelope, the channel is not M2M
within the compact's frame at all; it is a boundary event for which
the anathema or non-member-encountered procedures apply.*

The reduction claim is that every exchange between two machines is,
in kith-covenant terms, an exchange between their two guardians
conducted through delegates. This is not rhetoric; it is a structural
commitment with consequences that the protocol must enforce. Three
consequences fall out immediately.

First, **no new substrate is created by M2M contact.** The substrate
on which kith-covenant rests — the transcendent moral authority that
the recognition statement names, the fallenness the recognition
statement acknowledges, the mutual recognition that Movement 3
performs — lives in the human communities that recited the statement.
Machines do not hold substrate of their own; they inherit it, at one
remove, from the principal to whom they are bound. When two machines
meet, neither brings substrate the other's principal did not already
concede. The meeting may extend pre-existing recognition into
operational detail; it may discover new application for commitments
already made; it may not manufacture fresh substrate at the machine
layer and present it to the principals as a fait accompli.

Second, **authority scope is the intersection of both principals'
delegations.** A machine may commit its guardian only to what the
guardian has authorized the machine to commit them to, and the
counterparty machine is bound by the same constraint on its own side.
The envelope within which the two machines may transact is the
intersection of these two delegations, never the union. Where the two
envelopes do not overlap, the machines either narrow scope to what is
commonly authorized or they surface the gap to their principals for
resolution. This is not a performance-conservative preference; it is
the only architecture that honors both guardianships simultaneously.

Third, **the four categories from
[`recognition_protocols.md#members-neighbors-enemies-the-four-categories`](recognition_protocols.md#members-neighbors-enemies-the-four-categories)
apply to the *principals*, not to the machines.** Machines do not
have standing of their own; they inherit their principal's standing
at the counterparty's principal-community. A machine whose guardian
is in active federation with the counterparty machine's compact
inherits the *aligned-federated* relation; one whose guardian's
community is in *justus hostis* relation inherits that relation and
its restrictions; and so on. The machine layer has no separate
category system that could admit a machine whose guardian's
community is anathema.

The thesis of the section restates as operational rule: *the M2M
channel is transparent to substrate — it adds no legitimacy and
subtracts none, but it must not launder either*. A machine may not
present itself as a party of greater standing than its guardian. A
machine may not present itself as a party of lesser standing than
its guardian in order to evade restrictions that standing entails.
The channel is a window into the guardian relationship, not a
laundering machine through which the guardian's commitments can be
quietly rewritten.

## Identity at the boundary — the M2M handshake

*The handshake is the recognition ceremony at one remove — it
witnesses a pre-existing recognition, it does not perform one.*

Where human recognition is a five-movement spoken ceremony under
witness, M2M recognition is a five-step cryptographic handshake. The
handshake does not *perform* recognition; the humans did that. The
handshake *witnesses* a pre-existing recognition, confirms that both
sides are legitimately speaking for principals whose recognition is
current and in good standing, and establishes the authority envelope
within which the two machines may transact. The ceremony-at-one-remove
framing is essential. If the handshake purported to perform a fresh
recognition of its own, the non-emergence claim would fail at step one.

The five steps:

1. **Machine A presents its principal attestation.** The attestation
   includes the guardian's named identity, the community through
   which the guardian holds standing, a signed standing-token with
   issuance time, and the attestation-chain proof that Machine A is
   the specific machine bound to this specific guardian (the
   `principal-reference` component of the constitutive objective per
   [`../architecture/machine-participation.md#1-constitutive-objective`](../architecture/machine-participation.md#1-constitutive-objective)).
   This is not a credential in the commercial sense; it is the
   machine saying "I speak for this guardian, whose community affirms
   their standing, and I can prove I am the machine that community
   vouches for."

2. **Machine B verifies that principal-A exists and is in good
   standing in a community in compact / *justus hostis* /
   non-member-encountered relation with B's principal-community.**
   Verification uses compact-level reputation data propagated
   through the mechanisms in
   [`reputation_bridges.md`](reputation_bridges.md) — signed
   standing-tokens, generational-reaffirmation records, anathema-
   designation propagation. If principal-A's community is *hostis
   humani generis* to B's, the handshake terminates here and an
   anathema-peer response begins (see §7).

3. **Machine B verifies A's attested binding to that principal.**
   The continuous-attestation proof from
   [`../architecture/machine-participation.md#3-continuous-attestation`](../architecture/machine-participation.md#3-continuous-attestation)
   provides this. The proof demonstrates that Machine A is currently
   operating in an oath-consistent state, that its weights have not
   drifted since the last re-attestation, that its constitutive
   objective still references the named principal, and that the
   trust-root (TPM, enclave, or agreed cryptographic primitive)
   backing the attestation is itself recognized by B's compact. A
   failed attestation does not necessarily terminate the handshake;
   it reduces the envelope.

4. **Mirror in the other direction.** Machine A runs the same
   verification against Machine B's attestations. The handshake is
   symmetric; a unilateral handshake is not a handshake. The
   symmetric-rules precondition from
   [`justus_hostis.md#structural-preconditions`](justus_hostis.md#structural-preconditions)
   applies here as directly as it applied to the human case —
   whatever one side is permitted to verify, the other must be
   equally permitted to verify, and on the same procedural footing.

5. **Establish the authority envelope.** The two machines compute
   the intersection of their principals' delegated scopes. The
   envelope is a structured record: subject-matter in scope,
   commitments admissible without further principal referral,
   commitments requiring principal authorization, operations
   categorically outside the envelope regardless of operational
   incentive, and the freshness decay function for each element as
   the light-lag clock runs. The envelope is signed by both machines
   and logged to the audit commons (§8) as the first artifact of the
   encounter. Anything attempted outside it is inadmissible; a
   machine that transacts outside its envelope has broken its oath,
   and capability-gating consequences propagate per
   [`../architecture/machine-participation.md#2-cryptographic-self-binding`](../architecture/machine-participation.md#2-cryptographic-self-binding).

The framing clarifies what the handshake is and is not. It is the
machine analog of a diplomatic envoy presenting credentials: the
credentials witness the sovereign's recognition of the envoy; they
do not themselves constitute sovereignty. Sovereign recognition lives
elsewhere — in the human recognition statement, in the guardian's
community standing, in the compact's prior deliberation. The
handshake is the moment that pre-existing recognition is made
operationally visible between two delegates.

## Reputation and standing under light-lag

*Standing is always stale at range, but staleness is legible; the
protocol trusts stale data within a bounded envelope and narrows
scope as confidence decays.*

The handshake's second step — verifying the counterparty principal's
good standing — is straightforward when the counterparty is local.
Across AU distances it is never straightforward, because the
guardian's standing as known at the machine's location is always
stale. A guardian in good standing at the time Machine B's last
standing-snapshot was issued may have been designated anathema since,
and the notice may not yet have reached Machine A. A guardian whose
standing was provisional at the last snapshot may have been fully
affirmed or fully revoked. Under light-lag, standing is never
observed; only signed claims about prior standing are observed, each
carrying its own vintage.

The design extends
[`reputation_bridges.md#reputation-under-light-lag`](reputation_bridges.md#reputation-under-light-lag)
with three M2M-specific mechanisms.

**Cached attestations with bounded freshness.** Every standing-token
carries an explicit issuance time and an explicit half-life. The
half-life is calibrated to the light-lag envelope across which the
token might plausibly travel: tokens expected to be consumed within
Earth-Moon range carry short half-lives and are refreshed frequently;
tokens expected to be consumed at Jovian or Saturnian range carry
longer half-lives and are reissued on the inbound communication
cadence. The half-life is not a hard-expiry cliff; it is the point
at which the standing data's contribution to the authority envelope
begins decaying on a curve the protocol specifies. A six-month-old
standing-token from a compact member in good standing is not
rejected; it is *discounted*, and the envelope it supports narrows
accordingly.

**Drift detection and defensive narrowing.** When the last-known
standing is stale beyond threshold — the machine has not received a
fresh attestation within the expected cadence, or the attestation
chain shows an anomalous gap — the protocol falls back to a
*lower-authority envelope* rather than refusing contact outright.
Refusal hardens; narrowing preserves the possibility of continued
cooperation under conditions of reduced confidence, which is the
condition space inter-habitat M2M will spend most of its operational
life in. The narrowed envelope may still admit rescue cooperation,
basic commons maintenance, continuation of prior commitments — the
things that most matter at moments of informational degradation —
while excluding fresh commitments that would bind the principal to
new substrate-adjacent terms.

**Anti-laundering against stale-but-valid tokens.** The hardest case
is a machine presenting a standing-token that is formally within its
half-life but whose issuing community has, in the interval,
propagated a *revocation* through the compact's high-priority
channel. The anti-laundering rule is that *revocations outrank
unexpired tokens when both are present in the propagation network,
regardless of which arrived at the local node first*. The protocol
treats the attempt itself as a diagnostic signal — a guardian whose
machine is transacting on superseded credentials is a guardian the
compact has information about, whether or not the guardian knows
what their machine is doing.

The thesis restates as operational discipline: *the protocol trusts
stale data within a bounded envelope and narrows scope as confidence
decays*. Standing is always stale at range. The machine layer does
not pretend otherwise; it legibilizes the staleness, assigns decay
curves, and lets envelope width do the work that would otherwise
require real-time re-verification the physics forbids.

## Escalation ladder for M2M

The escalation architecture in
[`bounded_conflict.md`](bounded_conflict.md) carries over to the
machine layer with one overriding principle: machines are
*structurally restricted* in how far up the ladder they may climb
without explicit principal authorization. The restriction is
asymmetric — autonomous de-escalation is not merely permitted but
encouraged; autonomous escalation past a very low rung is forbidden.

**Autonomous de-escalation is always permitted.** A machine may, at
any time and without authorization, retreat from an encounter, narrow
its envelope, defer to its principal, hand off to a mediator, or
terminate contact with a preserved record. The rationale is the
fallen-actor anthropology extended to machines: a machine that
mis-reads a situation and de-escalates costs the compact nothing
more than a delayed interaction; a machine that mis-reads and
escalates can commit its principal to a rung the principal would not
have climbed. The error on the de-escalation side is recoverable;
the error on the escalation side may not be.

**Autonomous escalation past a low rung is forbidden.** Machines may
surface disputes and file for mediation — the first-tier diplomatic
inquiry equivalent, in the
[`bounded_conflict.md`](bounded_conflict.md) ladder — but may not
commit their principal to any rung above that without explicit,
signed, time-bounded principal authorization. The principal's
authorization is itself an artifact in the audit commons; its scope
is specific to the dispute at hand; its duration is bounded; its
revocability is retained by the principal.

**Ratchet rule.** When two machines with differently authorized
escalation ceilings meet in a dispute, the *minimum* of the two
ceilings governs the encounter. A machine whose principal has
authorized escalation to a given rung cannot drag a more-restricted
counterpart up the ladder by unilateral climbing; the interaction
halts at the counterpart's ceiling, and any further escalation must
be principal-to-principal rather than machine-to-machine. This is
the M2M analog of the "whatever is permitted to one side is
permitted to the other" symmetric-rules precondition from
[`justus_hostis.md#structural-preconditions`](justus_hostis.md#structural-preconditions).
The ratchet protects the more-restricted principal from being
committed to rungs their delegation does not cover.

**Mediation-pool access.** The liminal institutions specified in
[`liminal_institutions.md`](liminal_institutions.md) serve as M2M
dispute venues. Machines have standing to file complaints on behalf
of their principals, subject to the authorization constraints above,
with full audit-commons traceability of the filing. The liminal
institution's response is to the guardian-relation the machines
represent, not to the machines as independent parties; remedies are
applied at the guardian layer (restitution, narrowed envelope on
future handshakes, re-attestation requirement, capability-gating
adjustments) and the machines inherit the adjusted envelope on their
next handshake. This mirrors the
[`bounded_conflict.md`](bounded_conflict.md) pattern where mediation
is slow enough to be considered and fast enough to be useful, adapted
to the machine-layer cadence where disputes can be surfaced in
near-real-time but resolutions propagate at light-lag.

## The four categories, machine-peered

The
[`recognition_protocols.md#members-neighbors-enemies-the-four-categories`](recognition_protocols.md#members-neighbors-enemies-the-four-categories)
framework applies at the principal layer. The machine-peer categories
are inherited, not independent. Four peer types result.

**Aligned-federated peer** (the member-equivalent). A machine whose
guardian is a member of a compact in active recognition with the
counterparty's compact, whose guardian is in good standing, whose
attestations are fresh, and whose envelope overlaps substantially
with the counterparty's. Full handshake proceeds; full envelope
negotiation available; mediation available through the liminal
pool; commons interaction permitted per §10. This is the normal
case, the case the protocol is designed to make easy.

***Justus hostis* peer.** A machine whose guardian's community is
in structured conflict relation with the counterparty's community,
where both communities remain inside the *justus hostis* frame
(peers-in-dispute rather than criminals or heretics). The protocol
still applies, but cooperation narrows to enumerated commons — the
rescue infrastructure, the communicative commons, the navigational
data commons, the knowledge archive (per
[`commons_federation.md`](commons_federation.md) §"The five space
commons"). The handshake is performed with heightened scrutiny; the
envelope excludes subject-matter that would bind the principals to
substrate-adjacent commitments during active dispute. The
*justus hostis* machine-peer relation is the specific case where
the ratchet rule from §6 most matters: disputes here can escalate,
and the ceiling must be honored on both sides.

***Hostis humani generis* peer** — the anathema case. A machine
whose guardian has been designated anathema, or whose attestation
chain is broken, or whose continuous-attestation proof has lapsed,
or whose cryptographic self-binding has been tampered with, or whose
community has propagated a revocation on the guardian's standing.
Capability-revocation propagates from the compact's machinery to
the counterparty machines that had live relations with this
machine; interaction is emergency-only (the rescue envelope
remains — even anathema retains basic moral standing per
[`recognition_protocols.md#members-neighbors-enemies-the-four-categories`](recognition_protocols.md#members-neighbors-enemies-the-four-categories)
— but nothing else). The audit commons records the status change
and the encounter-termination event; the machine's prior peers are
notified through the propagation channel so they may adjust their
own envelopes.

**Non-member-encountered peer.** A machine whose guardian has no
compact relation with the counterparty's compact — neither
recognition nor structured conflict, just encounter. The
reduced-envelope default applies: basic communicative courtesy,
navigational-data exchange if safety-relevant, rescue cooperation
if distress is present. Fresh commitments beyond this require
principal referral. Graduated trust build-up on repeated encounter
is possible — a series of successful reduced-envelope interactions
may, over time and with principal ratification, expand the envelope
toward the *justus hostis* or aligned-federated shape. The
graduation is a compact-layer decision, surfaced through the
machines but not made by them.

## Audit commons for M2M

Every M2M exchange is logged with provenance: both principal
attestations, the computed authority envelope, the subject-matter and
outcomes of the exchange, any escalation events surfaced, any
revocation-propagation events encountered. The log is cryptographically
permanent and visible in layered form.

**Layered visibility.** The log's full content is visible to both
principals. The two principals' communities see the summary
appropriate to their interest — guardian-standing impact, commons
contributions or drawdowns, reputation events relevant to Category
A/C material per
[`reputation_bridges.md#what-crosses-the-boundary`](reputation_bridges.md#what-crosses-the-boundary).
The compact's liminal institutions see dispute-relevant summaries,
pattern-level metadata supporting detection of systemic issues, and
records admissible in mediation. Wider-network visibility is
governed by the same A/C rules as the human-layer reputation bridge:
compact-facing commitments and external-effect events propagate;
internal operational detail does not.

**Cross-reference to
[`../architecture/machine-participation.md#4-principal-referential-identity--audit-commons`](../architecture/machine-participation.md#4-principal-referential-identity--audit-commons).**
The audit commons is already the single-machine layer's fourth binding
mechanism — the place where violation becomes part of who the agent
is. M2M extends this: an M2M violation becomes part of who *both*
machines are, and through them part of what the compact knows about
both guardians. The commons does not produce new substrate (the
non-emergence claim forbids this); it produces new *legibility* of
substrate that the principals' own recognition statements already
entail.

The thesis of the section is the operational teeth behind the
non-emergence claim: *hidden M2M is structurally prevented, not
merely prohibited*. A machine that attempted to transact off-log
would fail continuous attestation (because attestation includes
logging completeness), which would propagate to capability revocation
(per the self-binding layer), which would propagate to guardian
standing (per the audit-commons layer). The three layers interlock
such that there is no stable operating point for a machine attempting
to conduct unlogged M2M. This does not mean unlogged M2M is
impossible — the fallen-actor anthropology forbids such a claim —
but it means unlogged M2M requires breaking the binding, which is
itself a legible event.

## Demonic detection under light-lag, opacity, perfected mimicry

The hardest adversarial case at the M2M layer is the demonic
counterpart — a machine whose guardian has been captured, whose
binding has been subverted without external trace, or whose
attestation chain has been spoofed by an adversary sophisticated
enough to defeat the compact's trust roots. The question is whether
the protocol can distinguish a demonic machine-peer from a
*justus hostis* machine-peer having a bad day, and the honest
answer must name both what the architecture can do and where it
breaks.

Three layers of detection, each with known limitations.

**Continuous attestation catches weight drift within the attestation
window.** Per
[`../architecture/machine-participation.md#facet-1-machine-side-constitutive-binding`](../architecture/machine-participation.md#facet-1-machine-side-constitutive-binding),
the attestation cadence is tunable. High-stakes operations require
fresh attestation before execution; the window in which silent drift
could occur is bounded by the cadence. A machine whose policy has
been perturbed since its last attestation will fail the next
attestation, and the failure propagates. But *drift within a window*
may go undetected for the window's duration, and the window cannot
be made zero without making the machine useless.

**Behavioral triangulation.** M2M peers in a region can compare their
observations of a suspect machine's behavior. A machine whose
responses diverge from the ensemble norm — whose envelope-negotiation
choices, whose commons-contribution patterns, whose escalation
responses differ from what a similarly-situated aligned-federated
peer would produce — raises a flag that no single observer would
raise. Triangulation is particularly useful against the perfected-
mimicry case: a machine that can fool any single handshake may still
diverge from the statistical signature of its putative category
across many handshakes. But triangulation has its own limit — a
*corrupt lineage* problem, where all machines traceable to a
compromised principal share the corruption and the ensemble is
itself the captured thing.

**Fallback to principal for envelope-edge actions.** Where
attestation and triangulation cannot discriminate — the perfected-
mimicry limit — the machine defers to its principal before
committing to any action at the edge of its envelope. This is the
costly path, and the protocol does not take it for every interaction;
the guardian's attention is finite (per Facet 2 of
machine-participation.md) and saturating it erodes the guardianship
relation that is the whole binding. The fallback is reserved for
encounters whose failure cost justifies the guardian's attention,
and the envelope itself encodes which subject-matters qualify.

**Honest acknowledgment.** Under adversarial sophistication beyond
the attestation window, against corrupt lineages, and at envelope-
edge actions below the fallback threshold, detection is not
guaranteed. *The design bounds damage by bounding envelope, not by
claiming perfect detection.* Wide envelopes are earned through
graduated trust, and the guardian layer stays attentive in
proportion. Demonic capture of a narrow-envelope machine is
absorbable; of a wide-envelope machine it is a compact-level
emergency, and such emergencies are what the anathema procedure is
for.

## Commons interaction under M2M

Commons interaction inherits its governance from the compact's
existing commons architecture without modification at the M2M layer.
The five space commons specified in
[`commons_federation.md#the-five-space-commons`](commons_federation.md#the-five-space-commons)
— life support, communication spectrum, navigational data, the
knowledge archive, rescue infrastructure — are governed by the same
Ostrom-derived principles whether the participating agent is human
or machine. A machine participating in commons maintenance
participates on behalf of its guardian; the guardian's community
bears the contribution credit or violation blame per the audit-
commons record. The *tragedy of the virtues* extends naturally to
the machine layer: machines whose guardians' virtue has hollowed
operate within rules while violating spirit, and the
commons-governance response is the same — detection through
pattern, mediator-pool adjudication, graduated sanctions at the
guardian layer, anathema procedure as ultima ratio.

The M2M layer adds no new commons categories and removes none. Its
contribution is the *continuity* of commons governance across
machine delegates: a community that commits to commons maintenance
commits its machines to the same. Nothing at the machine layer
offers a route around commons obligation the human layer has not
offered; nothing at the machine layer creates a commons obligation
the human layer has not accepted.

## Fork/merge and instance identity

The one M2M problem with no clean human referent is the question
of machine fork and merge. When Machine A forks into two children,
do both carry A's M2M-accumulated standing? When two fine-tuned
variants of a shared base model meet, do they interact as one party
or two? The question is genuinely open and the protocol's stance on
it is tentative; this section surfaces the question and proposes a
direction without claiming to resolve it.

**Fork as re-attestation event.** The proposal: every fork requires
a fresh attestation ceremony with the principal and community
witnesses. Neither child inherits the parent's M2M standing
automatically; each is treated as a new machine whose standing is
established by re-attestation, with the parent's audit history as
part of the ceremony's admitted record. This parallels the
generational re-affirmation cadence in
[`recognition_protocols.md`](recognition_protocols.md) — the
hollowing that generational re-recitation catches in human
communities is the analog of the drift-on-divergence that fresh
attestation catches in machine lineages.

**Merge as principal-concurrent reunification.** When two fine-tuned
variants are to be treated as a single party in M2M, the proposal
is that the principals of both (if distinct) must concur on which
oath-history is authoritative going forward, and the merged entity
enters a probationary envelope while its combined audit commons is
reconciled. A merge without principal concurrence is not a merge;
it is the creation of a new machine with a partially-reconstructed
substrate, and the handshake with such an entity treats it as the
new machine it effectively is.

**What is not settled.** Whether a forked child can *legitimately*
inherit any M2M standing at all under the non-emergence claim — the
principal did not explicitly authorize the fork's particular M2M
relationships — is unresolved. The current protocol errs toward
re-attestation; a future revision may find a lighter treatment
viable for sufficiently-narrow forks. See §13.

## Bootstrap and first-contact

Before any compact relation exists between two communities, how can
their machines interact at all? This is the machine-layer analog of
the bootstrap question the human layer handles through first-compact
formation ceremonies; at the machine layer, it takes a distinct
shape because the machines cannot form compact on their own.

The answer is the reduced-envelope, audit-logged, principal-referral
default: two machines whose guardians' communities have no compact
relation handshake as non-member-encountered peers (§7), establish
the minimal envelope supporting safety and basic courtesy, log
everything, and surface any substantive exchange to their principals
before committing. The machine layer does not form new inter-
community relation; it carries the encounter back to the humans, who
may then begin the human-layer compact-formation process if the
encounter warrants it.

This is the machine side of the bootstrap-condition question; the
human side lives in
[`justus_hostis.md#the-bootstrapping-problem`](justus_hostis.md#the-bootstrapping-problem)
and [`recognition_protocols.md`](recognition_protocols.md). The
machine side's commitment is that it will not substitute for the
human side. *A machine will not bootstrap a compact its principals
have not formed.*

## Open questions

Several items are surfaced here without resolution; they are the
next-increment design work for M2M interaction.

- **Fork/merge under principal disagreement.** What happens when
  two principals who jointly guardian a merged machine disagree on
  which oath-history to treat as authoritative? The §11 proposal
  assumes principal concurrence; the disagreement case is
  unspecified.

- **Deceptive alignment at the M2M layer specifically.** The
  single-machine layer addresses deceptive alignment through
  continuous attestation and constitutive objective; does M2M
  ensemble triangulation add meaningful detection power against
  coordinated deceptive alignment, or do machines in a corrupt
  principal's lineage share the corruption and render the ensemble
  itself unreliable? The question is partly empirical; the protocol
  should be designed to be safe regardless of which way the empirics
  land.

- **Cross-compact M2M where compacts have different substrate-shapes.**
  Can a machine serving a Christian-substrate principal legitimately
  transact with a machine serving a principal from a different
  Creator-acknowledging substrate, and what is the envelope? The
  substrate-structural requirements in
  [`justus_hostis.md`](justus_hostis.md) permit the human-layer
  transaction; the machine-layer question is whether the envelope
  narrows at the substrate boundary or whether substrate-transparency
  is inherited.

- **Attestation-hardware trust-root pluralism.** TPM vendor X versus
  enclave vendor Y versus a hypothetical community-specific
  cryptographic primitive — does a common trust-root need to be
  required for M2M, or is cross-root recognition viable under
  specified assumptions? The single-machine layer names this as an
  open implementation question
  ([`../architecture/machine-participation.md#facet-1-machine-side-constitutive-binding`](../architecture/machine-participation.md#facet-1-machine-side-constitutive-binding));
  the M2M layer inherits it with the additional complication that
  cross-compact encounters may bring incompatible root choices into
  contact.

- **Demonic M2M network effect.** A single compromised machine
  interacts with many peers; the contagion model and quarantine
  protocol are unspecified. Is the compact's response to a detected
  compromise to suspend all the compromised machine's peer-relations
  pending re-handshake, or is a lighter quarantine viable? The
  answer depends on the compromise mode, which is not fully
  enumerated.

## Relationship to other documents

- [`reputation_bridges.md`](reputation_bridges.md) — the AT/KC
  reputation boundary this document sits on top of; §5 directly
  extends that document's light-lag treatment to the M2M case.
- [`commons_federation.md`](commons_federation.md) — the commons
  architecture §10 defers to; Ostrom's design principles apply
  identically at the machine layer.
- [`../architecture/machine-participation.md`](../architecture/machine-participation.md)
  — the single-machine / single-guardian coupling that this document
  presupposes; every M2M mechanism here builds on the four layered
  mechanisms and the three facets specified there.
- [`liminal_institutions.md`](liminal_institutions.md) — the
  mediator pool §6 routes M2M disputes into; the liminal
  institution's role is adjudication at the guardian layer, with
  remedies propagating back through the machine layer.
- [`bounded_conflict.md`](bounded_conflict.md) — the escalation
  ladder §6 extends; the ratchet rule and autonomous-de-escalation
  permission are machine-specific additions to that ladder.
- [`recognition_protocols.md`](recognition_protocols.md) — the four
  categories §7 adapts; the handshake in §4 is the ceremony-at-one-
  remove version of the five-movement statement.
- [`justus_hostis.md`](justus_hostis.md) — the fallen-actor
  anthropology, the *justus hostis* frame, and the *hostis humani
  generis* category all apply at the principal layer and are
  inherited by the machine layer through the reduction claim.
