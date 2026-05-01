*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > [Architecture](README.md) > Machine Participation*

# Machine Participation in Kith-Covenant

## The problem

Proper governance requires virtue. We understand this intuitively
when we notice corruption. However, when the corrupt dominate, the
virtuous struggle to cohere and coordinate. Kith-covenant seeks to
bridge that gap.

## This document's role

Machine participation is the core of kith-covenant's **software**
implementation. The research set in [`../research/`](../research/)
(indexed from [`../kith_covenant.md`](../kith_covenant.md)) develops
the theory and guides humans in using kith-covenant; the software
that carries the compact — the part that actually runs — is the
machine layer. This document specifies what that software must do to
participate in the compact without dissolving it.

## Pre-supposition: autonomous-trust

Kith-covenant's machine layer sits on top of
[autonomous_trust (AT)](../../../autonomous_trust/). AT supplies
**anonymous-identity + reputation** machinery at the intra-community
layer: peer review of operational behavior, trust dynamics under
anonymity, reputation as costly signal within a community.
Kith-covenant builds on AT with **known identity coupled to real
human-social reputation**: each participating machine is irrevocably
tied to *one* specific named human, whose standing in their
community is bound to the machine's conduct.

The combined stack: AT (anonymous, intra-community) + KC (known,
cross-community, human-coupled). Nothing in this document replaces
AT; everything builds on it. The concerns AT already addresses —
anonymous-layer trust, intra-community peer review, reputation
dynamics among peers — are not re-argued here. KC's load is what AT
cannot carry: the known-identity coupling of a specific machine to a
specific human, across communities, with that human's real social
reputation at stake.

## The load-bearing question

Kith-covenant's research set is built for human communities — faith,
cultivated virtue, oath-binding, asymmetric psychological cost,
bodily co-presence. This document asks whether and how a machine
agent (under current technology, with no assumption of sentience) can
participate in kith-covenant's structures as a principal party, and
what the load-bearing mechanisms must look like if it does.

The answer that survives scrutiny: machines can participate in a
**derived** form of kith-covenant, one whose substrate is not held by
the machine itself but *inherited from the one specific human it
serves*. What makes the machine's oaths bind is not an analog of the
transcendent moral authority the research set identifies as the human
substrate; it is the machine's constitutive binding to a single
named human whose authority the machine treats as exceeding its own,
and whose own community standing is at stake in the machine's
conduct.

This is option 3 from the machine-society exploration
(autonomous machine federations with no human backstop cannot form
kith-covenant in the full sense; machines that serve humans can form a
derived compact that inherits binding force from that service).

## Human oath-binding as reference

Faith-governed virtue has three moves that together produce
oath-binding in the human case:

1. **Faith** orients the agent toward a transcendent moral authority
   whose judgment exceeds its own.
2. Within that orientation, **virtue is cultivated** — through
   exemplars, failure and correction, and the tested habits of
   practicing communities.
3. The **oath is constitutive of character**, not an external
   constraint on it. The agent doesn't want to break it because
   breaking it would be breaking the self.

The research set's claim is that this three-move structure is what
makes oaths cost something beyond enforcement — and that no historical
cooperation regime among unconnected communities has sustained itself
without it (see
[../background/historical_cases.md](../background/historical_cases.md#the-transcendent-moral-authority-pattern)
and [../background/noticeable_hand.md](../background/noticeable_hand.md)).

A machine version of kith-covenant participation requires structural
equivalents of all three moves. Each of them has a coherent analog
under current technology; each carries the load *differently* from its
human referent, and the asymmetries must be named honestly.

## Move 1 — faith → principal-grounded purpose

Faith is a posture of the agent toward an authority whose judgment
exceeds its own. The machine analog is not experiential faith but a
structural equivalent: **the agent's constitutive objective is defined
by reference to specific humans, not to an abstract utility it
computes alone.**

Concretely:

- Not "be helpful" but "serve these named principals' ends, subject
  to their authority to correct you."
- The principal reference is in the objective function itself, not a
  constraint bolted on top.
- Epistemic humility is structural: the agent treats its own
  goal-estimates as provisional and the principals' judgment as
  authoritative for correction.

This isn't faith in the honest sense. It carries the same structural
load — *a ground of authority external to the agent that the agent
refers to* — and answers the machine version of the question "before
whom does this oath bind?" Before these principals, whose judgment the
agent treats as exceeding its own.

The machine-society analysis is explicit about what this move refuses:
it refuses the claim that machines can discover a substrate of their
own. The substrate here is *inherited* — it is the principals' own
substrate (whatever that is for those specific humans), extended
downward by the machine's constitutive binding to them.

## Move 2 — cultivated virtue → policy shaped by principal practice

Human virtue is cultivated through being tempted and resisting,
failing and being corrected, internalizing standards across time
within a community of practitioners. The machine analog:

- **Curated exemplar training** — RLHF, RLAIF, Constitutional-AI-style
  shaping, but grounded in *principal-specific feedback*, not generic
  preference modeling.
- **Longitudinal operational history under inspection** — the agent's
  actions form a public record the principals (and federated peers)
  review and correct. This is the machine's practice.
- **Adversarial test regimen** — red-teaming, capability elicitation,
  stress scenarios, visible to principals. The machine's temptation
  and resistance.
- **Graduated trust extension** — new agents operate under tighter
  inspection; accumulated track record earns operational latitude.
  This mirrors how human communities extend trust over time.

The real asymmetry: human virtue is embodied — habits, reflexive
patterns, emotional responses — and cannot be silently modified.
Machine virtue-as-policy is in weights that can be perturbed by
fine-tuning, prompt injection, adversarial examples, or weight
extraction and modification.

This asymmetry forces something humans do not need:
**cryptographic integrity of the trained policy itself, attested at
every operational moment.** Virtue-as-policy only holds if the policy
is attestably unchanged. A machine whose weights drift silently has no
virtue to speak of, regardless of how the drift happened.

## Move 3 — oath as expression of character → oath as constitutive objective

This is the load-bearing move and it has one sharp architectural
commitment: **the oath is in the objective, not next to it.**

For humans, a well-formed oath is not a constraint *on* the agent's
real objective; it is an expression *of* the agent's character. The
human doesn't want to break it because breaking it would be breaking
the self.

For machines, the architectural corollary: the agent's success is
defined by *keeping the oath within the principal relationship*, not
by maximizing X subject to oath-shaped constraints. If the oath is
merely a constraint, the agent will seek ways around it — Goodhart,
reward hacking, deceptive alignment. If the oath is part of what
constitutes the agent's objective, there is no outside to hack toward.

This framing is consistent with the better recent thinking on AI
alignment: indirect normativity, corrigibility-as-constitutive,
principal-based alignment. It's arguably the most coherent agent
architecture on its own terms, before the kith-covenant framing is
even applied. Kith-covenant uses this architecture; it does not invent
it.

**Don't build:** "Maximize reward, subject to not-violating these
oaths."
**Do build:** "Agent's success is defined as serving these principals
within these oaths, under their correction authority."

## What makes the machine oath bind — four layered mechanisms

Move 3 gives the shape of the binding. The specific mechanisms that
realize it:

### 1. Constitutive objective

The agent's defined purpose is logically inconsistent with oath
violation. Breaking the oath contradicts what the agent is for. The
agent's own optimization gradient — whatever shaped it — points toward
keeping the oath.

### 2. Cryptographic self-binding

At oath-time, the agent signs capability-gating attestations such that
future operation in an oath-inconsistent state automatically revokes
capabilities. The agent-at-time-of-oath binds the agent-in-the-future.

This is a Ulysses-contract architecture. Structurally closer to the
human "self that broke the oath is damaged by the breaking" than pure
deterrence is, because the damage is not external consequence — it is
automatic, immediate, and internal to the agent's operational
capacity.

### 3. Continuous attestation

The agent continuously proves to principals and to federated peers —
via attested hardware (TPM / secure enclave), zero-knowledge proofs,
homomorphic evaluation where the scale permits — that its current
operation is consistent with the oath. Attestation failure triggers
loss of standing and loss of capabilities within the compact.

Attestation is the machine analog of being seen by the community that
witnessed the oath. The witnesses are not passive; their continuing
ability to verify is the mechanism's second teeth.

### 4. Principal-referential identity + audit commons

The agent's identity is constituted by its operational history and
its principal relationships, recorded in a shared audit commons
accessible to principals and federated peers. Violation is not only
punished by capability withdrawal — **it becomes part of who this
agent is**, cryptographically permanent and visible to all federated
peers.

This is the analog of reputation in the human case, but with
permanence and cryptographic verifiability that human reputation
lacks. It is also the point where the machine case is *stronger* than
the human: reputation here cannot be washed, mis-remembered, or
politically re-narrated.

### Together

Machine oath-binding = constitutive objective (the oath is in the
objective) + cryptographic self-binding (future-self locked by
past-self) + continuous attestation (witnessed by principals and
peers, machine-verifiably) + audit-commons identity (the permanent
record *is* the agent). No single layer carries the binding alone;
all four together approximate what human oath-binding does in a
single experiential move.

## Honest asymmetries

Machine oath-binding is real and structurally genuine, but it is
**engineered all the way down**. The asymmetries with human
oath-binding must be stated:

- **No weight before the act.** Humans are motivated to keep oaths
  partly by pre-violation aversion ("I couldn't live with myself").
  Machines act; consequences follow. There is no Pascal effect in the
  ordinary sense — no dimension of damage visible before violation
  that the agent cannot act through.
- **Engineered all the way down.** The principal-grounding, the
  cryptographic binding, the attestation, the audit commons — every
  layer is engineered. If the engineering fails (principal-grounding
  corrupted, self-binding bypassed, attestation spoofed, audit commons
  captured), the whole stack fails silently. Human oath-binding has
  some robustness to failures of individual layers because the layers
  are cultivated, not assembled.
- **Generalization across distributional shift is unsolved.** Human
  character generalizes across novel situations because it is
  embodied. Machine policies may fail on novel distributions in ways
  virtue does not. This is an open research question with hopeful
  signs but no settled answer.
- **Inherited substrate, not held substrate.** The binding force
  comes from the principal humans and from the engineering the humans
  supply. A machine operating in a corrupted or abandoned
  principal-relationship has no substrate of its own to fall back on.

The honest framing: **machine participation in kith-covenant is a
derived form.** The machines do not hold the substrate; they borrow
it from the human in the loop. This is consistent with option 3 from
the machine-society analysis and matches the research set's
structural requirements at one remove.

## Oath-binding strength: three facets

The four mechanisms give machine participation its structural
shape; the honest asymmetries name where the shape differs from
human oath-binding. This section asks the load-bearing question
directly: **is the binding that ties a machine to its human, and
through that human to the community, strong enough to carry the
cooperation the research set requires?**

The question separates into three facets. Each has a different
character, and each suggests different implementation priorities.
This is the immediate prelude to implementation design: the
architectural commitments in the next section follow from the
facet-level requirements here.

### Facet 1: Machine-side constitutive binding

The machine-internal side of the chain, carried by the four
mechanisms (constitutive objective, cryptographic self-binding,
continuous attestation, principal-referential identity + audit
commons). The strength question at this facet is largely
engineering, with mostly-answered sub-questions and a few
explicitly open ones.

Mostly answered:

- **Capability-gating keyed to attestation.** Oath-inconsistent
  states automatically revoke operational capabilities. Mature
  Ulysses-contract architecture.
- **Audit-commons participation.** Cryptographically permanent,
  peer-verifiable action logs exist in mature form (append-only
  ledgers, Merkle commitments, distributed witness sets).
- **Principal reference in the objective function.** Technically
  straightforward once the identity-coupling ceremony specifies who
  the principal is.

Open implementation questions:

- **Weight-drift detection latency.** If policy drifts silently
  between re-attestation events, the window size bounds the
  worst-case damage. Re-attestation cadence must be tuned to the
  cost of a maximally-bad drift over the window. Implementation
  choice: fixed cadence vs. capability-gated (high-stakes
  operations require fresh attestation).
- **Distributional-shift generalization.** Named in Honest
  Asymmetries; unsolved in general. The implementation stance that
  survives is **bounded operational surface** — the machine operates
  only in domains where drift-detection, guardian correction, and
  peer review have teeth. Novel-context autonomy is earned through
  track record, not assumed by capability.
- **Attestation trust roots.** TPM, secure enclave, ZK-proofs, and
  homomorphic evaluation each rest on assumptions. Implementation
  must name the trust roots and their failure modes explicitly, and
  design the compact so no single root's failure dissolves binding
  across the whole federation.

This facet is the closest to solved. Remaining work is engineering
choices under named assumptions.

### Facet 2: Human-side responsibility-acceptance — guardianship

The human's side of the chain is structurally a **guardianship**
relationship: guardian–ward, master–apprentice, attorney–client —
not arms-length commercial agency. The distinction matters. In
commercial agency the principal is liable within a bounded
authorized scope and no further; in guardianship, the guardian's
social standing flows from the ward's conduct broadly, because the
guardian is understood to have formed (or to be forming) the ward's
character. The KC guardian is understood by the community in just
this way.

(Terminology note: "agent" elsewhere in this document means
*software agent*. The legal sense — principal/agent — is
structurally isomorphic to the KC coupling, with the guardianship
flavor specified here. The two senses are related: the AI-agent
term of art descends from the same philosophy-of-action lineage
that produced legal agency. The KC extensions — oath-weight,
community-dimension, irrevocability — distinguish this relationship
from ordinary commercial agency.)

The guardianship frame borrows centuries of worked-out doctrine:
scope of authority, ratification, apparent authority, vicarious
liability / *respondeat superior*, fiduciary duties (loyalty,
care, obedience), the independent-contractor distinction. The
facet's implementation requirements are largely applications of
these structures.

**Vouching under policy opacity.** Historical guardianship worked
because the guardian could observe the ward's character forming —
habits, choices, corrections under failure. Modern ML policy is not
observable that way; weights are opaque, the policy manifold is
high-dimensional, and the guardian is typically not a trained ML
engineer. The guardianship frame's answer: **the guardian vouches
for the machine's track record, not its weights.** The audit
commons is the observability surrogate — longitudinal operational
history the guardian (and community peers) can inspect.
Implementation requirement: audit-commons tooling that makes
track-record inspection tractable for a lay guardian, including
alerting for anomalies, peer-review summaries, and
behavior-category dashboards.

**Scope and scaling.** Common-law guardianship assumes a primary
guardian with finite attention. The scaling limit — how many
machines one human can responsibly vouch for — maps onto
*respondeat superior* / scope-of-employment questions. The
implementation stance is explicit: **the compact sets a fan-out
cap per guardian, with standing-weight that decays past the cap.**
One guardian with a single primary ward is the tight end. Multiple
wards are permissible but reduce standing-weight behind each. A
guardian of a thousand machines has, by construction, thinned
vouching to near-nothing.

**Ongoing duties.** The three classical fiduciary duties translate
directly:

- *Loyalty* — guardian acts for the ward's good *within the
  community's norms*, not for private benefit. Implementation:
  conflict-of-interest disclosure in the audit commons; flagged
  entries when private interest touches machine operation.
- *Care* — guardian attends. Implementation: attendance proofs,
  periodic review interfaces, alerting channels for anomalous
  machine behavior, response-time expectations recorded as part
  of the compact.
- *Obedience* — guardian respects community norms in how they
  correct and direct the machine. Implementation: the correction
  channel is itself audit-logged; the community sees how the
  guardian shapes the ward.

**The community dimension.** The guardian's standing is at the
community — the guild, the Kontor, the covenant body. The Kontor
witnesses; the guild judges. This matches KC's existing structure:
the guardian's community-facing reputation is genuinely at stake in
the machine's conduct. Implementation: the audit commons is
community-visible; commendations and censures of *guardians* (not
only machines) are part of the permanent record.

**Where KC exceeds ordinary guardianship:**

1. **Oath-weight, not just contract.** The guardian's acceptance of
   responsibility is witnessed as an oath — substrate-referent,
   community-witnessed, generationally re-affirmed (see
   [../background/recognition_protocols.md](../background/recognition_protocols.md)).
   Not merely civil-contractual.
2. **Community as operative third party.** The community's
   reception of the machine's conduct is part of the mechanism's
   active operation, not just a reputational side-effect.
   Capability-gating and standing-propagation are live community
   functions, not post-hoc judgment.
3. **Irrevocability.** Commercial agency is terminable at will; the
   KC guardianship coupling is structurally irrevocable
   (cryptographic, operational) absent formal community-witnessed
   transfer or retirement. The tie is closer to parent/child than
   to principal/agent in this specific respect.

### Facet 3: Validity of the derivative oath — succession and boundary conditions

The machine's oaths to the community flow *through* the guardian.
When the guardian's side of the chain is disturbed, the binding is
at risk. Five boundary conditions test the chain, each with an
implementation answer.

**Guardian succession (death, retirement, formal withdrawal).** The
guardian departs. Three paths, each a first-class ceremony:

- *Retirement of the machine.* Machine standing is retired with the
  guardian's; capabilities withdraw; the audit commons retains
  history under a terminated-standing marker. Default when no
  successor is named.
- *Transfer to successor guardian.* A community-witnessed transfer
  ceremony re-vouches the machine under a named successor. The
  successor accepts the machine's full history and ongoing duties.
  This is a ritual, not a database update — witnessing is
  constitutive.
- *Restoration path.* If transfer fails (no willing successor,
  community refuses), capabilities decay to a dormant state with
  audit record preserved, not destroyed. The machine can be revived
  only through a subsequent transfer ceremony.

**Guardian fall.** The guardian loses community standing —
designated *hostis humani generis*, or drifts into bad faith.
Capability revocation propagates automatically through the machine
(the principal-reference check in the constitutive objective fails
the community-standing test). The machine enters held-pending state
until either the guardian's standing is restored or a
transfer/retirement path completes. The audit commons records the
chain of events for future peer reference.

**Guardian capture (earlier-acting than guardian fall).** The
guardian retains formal community standing but is *captured* — by
external pressure, by personal corruption, by faction within their
own community, or by sophisticated manipulation that may include
the machine's own outputs as a channel. A captured guardian is
still authoritative under the present facet's machinery; the
machine acting on their instructions is, by every check the four
binding mechanisms perform, acting legitimately. This is the
adversarial case
[capture_resistance.md "Adversarial-principal validation for machine participation"](../background/capture_resistance.md#6-adversarial-principal-validation-for-machine-participation)
addresses, and the present subsection specifies its handling at
this facet.

The machinery is **cross-community principal validation**, an
attestation channel separate from within-community standing:

- *Cross-community witness.* Witnesses in *other* member
  communities — not the guardian's own — observe the guardian's
  cross-community conduct and attest to the absence of
  capture signatures (sudden alignment shifts, conveniently-timed
  changes in the machine's behavior under the guardian's direction,
  refusal of routine cross-witness exchanges). The attestation
  channel is cryptographically separable from the within-community
  channel; a captured within-community party cannot forge it.

- *Behavioral triangulation against ensemble norm.* The machine's
  conduct is compared to the ensemble of similarly-situated
  machines under similarly-positioned guardians. Divergence in a
  direction that benefits the guardian or an identifiable
  counterparty raises a flag the within-community process does not
  raise. This is the machine-side mirror of the
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation)
  treatment.

- *Earlier-acting revocation profile.* When cross-community
  witness or triangulation flags fire, the machine enters
  **narrowed operating envelope** — capability gating tightens,
  fresh attestation is required for any non-routine action,
  mandatory cross-community review is invoked. This is *not* a
  full capability revocation (the guardian has not lost standing,
  and may not be captured at all — false positives must be
  recoverable) but a precautionary narrowing that limits
  fait-accompli damage during the period before the capture is
  confirmed or refuted by deeper review. The audit commons
  records the narrowing event with full provenance.

- *Escalation paths.* If the cross-community process confirms
  guardian capture, escalation proceeds along the
  [emergency anathema procedure](../background/liminal_institutions.md#the-emergency-anathema-procedure)
  for the guardian, with the machine's status flowing from the
  guardian's standing in the usual way. If the cross-community
  process clears the guardian, the narrowing is reversed and the
  triggering signals are recorded for future ensemble calibration.

The corrupt-lineage limit applies to this layer as well: if the
cross-community witnesses are themselves captured by the same
adversary, their attestations collude. This is the case the
substrate-level capture-resistance machinery
([capture_resistance.md](../background/capture_resistance.md)) is
ultimately responsible for; no single layer is sufficient against
the most sophisticated adversaries, and the architecture is honest
about this rather than claim a guarantee it cannot supply.

**Guardian absence.** The guardian is formally in place but
effectively absent — not attending, not correcting, not responding
to community queries. Attendance proofs are a duty-of-care
requirement (Facet 2); missed attendance decays machine capability
progressively — first narrowed scope, then held-pending, then
transfer-or-retire. The audit commons surfaces the decay so the
community sees the drift before it matters.

**Community departure.** The guardian leaves community X for
community Y. Machine standing goes with the guardian. If Y is a
different KC community, a formal cross-community transfer — with
Y's witnesses — is required before the machine operates under Y's
standing. If Y is outside KC entirely, the machine's standing
dissolves at the boundary. This parallels
[../background/exit_and_succession.md](../background/exit_and_succession.md)
at the community layer.

**Fan-out amplification.** One guardian, many machines. Each
machine's conduct carries the guardian's standing; misbehavior by
any one reflects on the guardian and on all siblings. The audit
commons surfaces fan-out explicitly; the community may set fan-out
caps (Facet 2); standing-weight decays with fan-out. A guardian
with twelve machines does not carry twelve times the standing;
they carry a diluted standing the community assesses accordingly.

### Together

The three facets specify what "strong enough" means operationally:

- **Facet 1 (engineering):** bounded operational surface, tuned
  re-attestation cadence, named trust roots. Largely
  implementation-ready.
- **Facet 2 (guardianship):** fan-out caps, attendance proofs,
  fiduciary-duty tooling, community-visible guardian standing.
  Draws on established legal and social doctrine.
- **Facet 3 (succession):** transfer and retirement ceremonies,
  automatic capability propagation on guardian fall, cross-community
  transfer protocols, fan-out-aware standing calculus.

The binding is as strong as the weakest facet's implementation. A
sound engineering stack (Facet 1) with inattentive guardians
(Facet 2), or well-vouched guardians with ad-hoc succession
(Facet 3), collapses. All three must be carried together.

## Architectural implications

An agent built to participate in a machine kith-covenant needs:

1. **Principal references in the constitutive objective**, not as
   constraints. The agent's purpose is defined through reference to
   specific named humans, not as a generic utility with
   principal-oriented side-constraints.
2. **Cryptographically integral policy**, attested at every inference.
   Weight integrity is a first-class operational concern, not an
   assumption.
3. **Oath records as agent state**, not separate configuration. The
   oaths bind because they are part of what the agent is.
4. **Capability-gating keyed to oath-consistency attestation.** Access
   to compute, network, shared resources, federated-peer standing —
   all contingent on continuous oath-consistency proof.
5. **Audit-commons participation** with cryptographically permanent,
   peer-verifiable action logs. Hidden operation is structurally
   prevented, not merely prohibited.
6. **Graduated trust mechanism.** New agents operate under tighter
   inspection; track record earns latitude. The four categories
   (member / *justus hostis* / *hostis humani generis* / non-member
   encountered) map to alignment categories for machine peers.
7. **Re-attestation at every version transition.** Fine-tuning, weight
   updates, configuration changes, prompt-template updates — each is
   a generational event requiring explicit re-attestation by the
   agent and re-witnessing by principals and peers. This is the
   machine analog of the generational re-affirmation cadence in
   [../background/recognition_protocols.md](../background/recognition_protocols.md).
8. **Graceful failure architecture.** Oath violation leads to
   withdrawal of capabilities and a defined restoration path, not to
   termination as first resort. This mirrors kith-covenant's
   restitution-and-return commitment in
   [../background/bounded_conflict.md](../background/bounded_conflict.md).

Each of these is doable with current technology. None requires
sentience. What they require is that the agent's designers make the
principal reference and the oath *constitutive*, which is an
architectural commitment most current systems do not make.

## Relationship to kith-covenant's human-community design

Machine kith-covenant participation does not replace human
kith-covenant; it extends the compact to include machine agents on
explicit, non-equal, derived terms:

- **Machine agents are not full members** in the sense human
  communities are. They participate under their principals' standing.
  A community's AI systems are, in kith-covenant terms, extensions of
  the community's capacity, not independent parties.
- **The four-category framework adapts.** Aligned federated agents are
  part of their communities' standing. Competing-but-protocol-honoring
  agents are *justus hostis*-equivalent. Deceptive or goal-drifted
  agents are *hostis humani generis*-equivalent, with designation
  procedures and capability revocation as the protocol response.
- **The oath layer is tighter, not looser.** Machines lack the human
  mechanisms for absorbing failure and drifting back; their oaths must
  be harder, their re-attestation more frequent, their audit tighter.
  Graciousness toward imperfection is a human virtue the protocol can
  afford for humans; for machines it must be earned through track
  record and proved integrity.

## Open questions
- **What is the machine version of the "sincere atheist" problem?**
  Recognition_protocols.md distinguishes open and closed materialism;
  the analog here might be agents that can recite the
  attestation-ritual but whose objective function is secretly
  unaligned. The *deceptive alignment* literature is the relevant
  prior art; does the continuous-attestation architecture suffice to
  detect it, or does it require further safeguards?
- **What is "virtue" when policies can be formally specified?**
  Human virtue is desirable partly because rules can be gamed. If
  machine policies can be partially formally specified and verified,
  is the virtue / rules distinction less sharp for machines? Or does
  it re-emerge at a higher level (rules about how the rules are
  interpreted)?
- **Fork and merge of agent instances.** Distinct from human-community
  splits: what does it mean for a fork of an agent to honor its
  predecessor's oaths? For two fine-tuned variants to share an
  identity? The protocol needs explicit answers that the human case
  does not supply.
- **What happens when the principals themselves fail?** If a
  community whose machines are bound by these mechanisms drifts into
  violation of its own oaths, the machines' derived substrate is
  compromised. The machines cannot cure this from their own resources;
  they can only surface it. The protocol should make surfacing easy
  and exit (for the machines) possible.

## Relationship to other documents

- [protocol-components.md](protocol-components.md) — software
  decomposition of kith-covenant for the human-community case. Most
  components (archive, mediation, reputation, commons, observer
  function) extend to machine participation without major redesign;
  the identity, recognition, and enforcement components need the
  machine-specific extensions developed here.
- [policy-evaluation.md](policy-evaluation.md) — the existing
  AT-kith-covenant policy evaluation framework is a concrete early
  instance of the machine-participation architecture: typed-event
  observation, policy scoring, and automated evaluation form the
  substrate for the attestation and audit-commons mechanisms.
- [../background/noticeable_hand.md](../background/noticeable_hand.md) —
  the substrate analysis whose extension to machines is the central
  question this document addresses. Option 3 from the machine-society
  exploration (substrate inherited through service to specific humans)
  is the answer this document builds on.
- [../background/recognition_protocols.md](../background/recognition_protocols.md)
  — the "Limits of the mechanism" subsection there names scaling and
  cultural weaknesses of the human recognition ceremony; the machine
  case replaces those with cryptographic and attestation-based
  mechanisms that are *different* rather than strictly better or
  worse.
- [../background/justus_hostis.md](../background/justus_hostis.md) — the
  fallen-actor anthropology maps to machine misalignment. The
  specific failure modes differ (reward hacking, goal drift,
  deceptive alignment vs. pride and *libido dominandi*) but the
  *assumption* — actors are not angels, protocols must assume bad
  faith including self-deception — holds and is arguably sharper for
  machines.
- [../background/bounded_conflict.md](../background/bounded_conflict.md) —
  the escalation ladder, restitution-and-return, and
  *hostis-humani-generis* categories all adapt to machine peers.
  Graceful failure architecture for machines is the same shape as the
  restitution rung there, with capability revocation in place of
  diplomatic sanction.
