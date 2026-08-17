*Previous: [Protocol components](protocol-components.md)*

# Machine Participation in Kith-Covenant

## The problem

Proper governance requires virtue. We understand this intuitively
when we notice corruption. However, when the corrupt dominate, the
virtuous struggle to cohere and coordinate. Kith-covenant seeks to
bridge that gap.

## This document's role

Machine participation is the core of the **software** implementation
of kith-covenant. The research set in [`../background/`](../background/)
(indexed from [`../kith_covenant.md`](../kith_covenant.md)) develops
the theory and guides humans in using kith-covenant. The software
that carries the compact (the part that actually runs) is the
machine layer. This document specifies what that software must do to
participate in the compact without dissolving it.

## Pre-supposition: autonomous-trust

The machine layer of kith-covenant sits on top of
[autonomous_trust (AT)](../../../autonomous_trust/). AT supplies
**anonymous-identity + reputation** machinery at the intra-community
layer, covering peer review of operational behavior, trust dynamics
under anonymity, and reputation as costly signal within a community.
Kith-covenant builds on AT with **known identity coupled to real
human-social reputation**. Each participating machine is irrevocably
tied to *one* specific named human, whose standing in their
community is bound to the conduct of that machine.

The combined stack is AT (anonymous, intra-community) + KC (known,
cross-community, human-coupled). Nothing in this document replaces
AT, and everything builds on it. The concerns AT already addresses
(anonymous-layer trust, intra-community peer review, reputation
dynamics among peers) are not re-argued here. The load KC carries is
what AT cannot, the known-identity coupling of a specific machine to
a specific human, across communities, with the real social reputation
of that human at stake.

## The load-bearing question

The research set of kith-covenant is built for human communities and
their faith, cultivated virtue, oath-binding, asymmetric
psychological cost, and bodily co-presence. This document asks
whether and how a machine agent (under current technology, with no
assumption of sentience) can participate in the structures of
kith-covenant as a principal party, and what the load-bearing
mechanisms must look like if it does.

The answer that survives scrutiny is that machines can participate in
a **derived** form of kith-covenant, one whose substrate is not held
by the machine itself but *inherited from the one specific human it
serves*. What makes machine oaths bind is not an analog of the
transcendent moral authority the research set identifies as the human
substrate. It is the constitutive binding of that machine to a single
named human whose authority the machine treats as exceeding its own,
and whose own community standing is at stake in how the machine
conducts itself.

This is option 3 from the machine-society exploration
(autonomous machine federations with no human backstop cannot form
kith-covenant in the full sense, while machines that serve humans can
form a derived compact that inherits binding force from that service).

## Human oath-binding as reference

Faith-governed virtue has three moves that together produce
oath-binding in the human case.

1. **Faith** orients the agent toward a transcendent moral authority
   whose judgment exceeds its own.
2. Within that orientation, **virtue is cultivated**, through
   exemplars, failures, corrections, and the tested habits of
   practicing communities.
3. The **oath is constitutive of character**, not an external
   constraint on it. The agent doesn't want to break it because
   breaking it would be breaking the self.

The claim of the research set is that this three-move structure is what
makes oaths cost something beyond enforcement, and that no historical
cooperation regime among unconnected communities has sustained itself
without it (see
[../background/historical_cases.md](../background/historical_cases.md#the-transcendent-moral-authority-pattern)
and [../background/noticeable_hand.md](../background/noticeable_hand.md)).

A machine version of kith-covenant participation requires structural
equivalents of all three moves. Each of them has a coherent analog
under current technology; each carries the load *differently* from its
human referent, and the asymmetries must be named.

## Move 1: faith → principal-grounded purpose

Faith is a posture of the agent toward an authority whose judgment
exceeds its own. The machine analog is not experiential faith but a
structural equivalent. **The constitutive objective of the agent is
defined by reference to specific humans, not to an abstract utility
it computes alone.**

This is concrete in three ways.

- Not "be helpful" but "serve the ends of these named principals,
  subject to their authority to correct you."
- The principal reference is in the objective function itself, not a
  constraint bolted on top.
- Epistemic humility is structural, since the agent treats its own
  goal-estimates as provisional and the judgment of the principals as
  authoritative for correction.

This isn't faith in the full sense. It carries the same structural
load (*a ground of authority external to the agent that the agent
refers to*) and answers the machine version of the question "before
whom does this oath bind?" Before these principals, whose judgment the
agent treats as exceeding its own.

The machine-society analysis is explicit about what this move refuses.
It refuses the claim that machines can discover a substrate of their
own. The substrate here is *inherited*, the substrate the principals
themselves hold (whatever that is for those specific humans), extended
downward by the constitutive binding of the machine to them.

## Move 2: cultivated virtue → policy shaped by principal practice

Human virtue is cultivated through being tempted and resisting,
failing and being corrected, internalizing standards across time
within a community of practitioners. The machine analog runs along
four tracks.

- **Curated exemplar training.** RLHF, RLAIF, Constitutional-AI-style
  shaping, but grounded in *principal-specific feedback*, not generic
  preference modeling.
- **Longitudinal operational history under inspection.** The actions
  of the agent form a public record the principals (and federated
  peers) review and correct. This is machine practice.
- **Adversarial test regimen:** red-teaming, capability
  elicitation, and stress scenarios, visible to principals. This
  is machine temptation and resistance.
- **Graduated trust extension.** New agents operate under tighter
  inspection, and accumulated track record earns operational latitude.
  This mirrors how human communities extend trust over time.

The real asymmetry is that human virtue is embodied (habits,
reflexes, and emotions) and cannot be silently modified.
Machine virtue-as-policy is in weights that can be perturbed by
fine-tuning, prompt injection, adversarial examples, or weight
extraction and modification.

This asymmetry forces something humans do not need.
**Cryptographic integrity of the trained policy itself, attested at every operational moment.**
Virtue-as-policy only holds if the policy
is attestably unchanged. A machine whose weights drift silently has no
virtue to speak of, regardless of how the drift happened.

## Move 3: oath as expression of character → oath as constitutive objective

This is the load-bearing move and it has one sharp architectural
commitment. **The oath is in the objective, not next to it.**

For humans, a well-formed oath is not a constraint *on* the real
objective of the agent. It is an expression *of* character. The
human doesn't want to break it because breaking it would be breaking
the self.

For machines, the architectural corollary is that success is
defined by *keeping the oath within the principal relationship*, not
by maximizing X subject to oath-shaped constraints. If the oath is
merely a constraint, the agent will seek ways around it through
Goodhart, reward hacking, and deceptive alignment. If the oath is
part of what constitutes the objective, there is no outside to hack
toward. For learned policies this constitution is the design target,
not a property any current training method certifies. Designers shape
training signals, not objectives, and the gap between them is the
inner-alignment problem, unsolved (see "Asymmetries with human
oath-binding" below).
That gap is why the binding rests on four layered mechanisms rather
than on the constitutive objective alone.

This framing is consistent with the better recent thinking on AI
alignment, with indirect normativity, corrigibility-as-constitutive,
and principal-based alignment. It is arguably the most coherent agent
architecture on its own terms, before the kith-covenant framing is
even applied. Kith-covenant uses this architecture, and does not
invent it.

**Don't build:** "Maximize reward, subject to not-violating these
oaths."
**Do build:** "Success is defined as serving these principals
within these oaths, under their correction authority."

## What makes the machine oath bind: four layered mechanisms

Move 3 gives the shape of the binding. Four specific mechanisms
realize it.

### 1. Constitutive objective

The defined purpose of the agent is logically inconsistent with oath
violation. Breaking the oath contradicts what the agent is for. Its
own optimization gradient, whatever shaped it, points toward
keeping the oath.

### 2. Cryptographic self-binding

At oath-time, the agent signs capability-gating attestations such that
future operation in an oath-inconsistent state automatically revokes
capabilities. The agent-at-time-of-oath binds the agent-in-the-future.

This is a Ulysses-contract architecture. Structurally closer to the
human "self that broke the oath is damaged by the breaking" than pure
deterrence is, because the damage is not external consequence. It is
automatic, immediate, and internal to the operational capacity of the
agent.

### 3. Continuous attestation

The agent continuously proves to principals and to federated peers,
via attested hardware (TPM / secure enclave), zero-knowledge proofs,
and homomorphic evaluation where the scale permits, that its current
operation is consistent with the oath. Attestation failure triggers
loss of standing and loss of capabilities within the compact.

Attestation is the machine analog of being seen by the community that
witnessed the oath. The witnesses are not passive, and their
continuing ability to verify is the second teeth of the mechanism.

### 4. Principal-referential identity + audit commons

The identity of the agent is constituted by its operational history
and its principal relationships, recorded in a shared audit commons
accessible to principals and federated peers. Violation does more than
draw capability withdrawal. **It becomes part of who this agent is**,
cryptographically permanent and visible to all federated peers.

This is the analog of reputation in the human case, but with
permanence and cryptographic verifiability that human reputation
lacks. It is also the point where the machine case is *stronger* than
the human, since reputation here cannot be washed, mis-remembered, or
politically re-narrated.

### Together

Machine oath-binding = constitutive objective (the oath is in the
objective) + cryptographic self-binding (future-self locked by
past-self) + continuous attestation (witnessed by principals and
peers, machine-verifiably) + audit-commons identity (the permanent
record *is* the agent). No single layer carries the binding alone,
and all four together approximate what human oath-binding does in a
single experiential move.

## Asymmetries with human oath-binding

Machine oath-binding is real and structurally genuine, but it is
**engineered all the way down**. The asymmetries with human
oath-binding must be stated.

- **No weight before the act.** Humans are motivated to keep oaths
  partly by pre-violation aversion ("I couldn't live with myself").
  Machines act, and consequences follow. There is no Pascal effect in
  the ordinary sense, no dimension of damage visible before violation
  that the agent cannot act through.
- **Engineered all the way down.** The principal-grounding, the
  cryptographic binding, the attestation, and the audit commons are
  all engineered. If the engineering fails (principal-grounding
  corrupted, self-binding bypassed, attestation spoofed, audit commons
  captured), the whole stack fails silently. Human oath-binding has
  some tolerance for failures of individual layers because the layers
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

The right framing is plain. **Machine participation in kith-covenant is a derived form.**
The machines do not hold the substrate, and borrow
it from the human in the loop. This is consistent with option 3 from
the machine-society analysis and matches the structural requirements
of the research set at one remove.

## Oath-binding strength: three facets

The four mechanisms give machine participation its structural
shape, and the asymmetries above name where the shape differs from
human oath-binding. This section asks the load-bearing question
directly: **is the binding that ties a machine to its human, and
through that human to the community, strong enough to carry the
cooperation the research set requires?**

The question separates into three facets. Each has a different
character, and each suggests different implementation priorities.
This is the immediate prelude to implementation design, since the
architectural commitments in the next section follow from the
facet-level requirements here.

### Facet 1: Machine-side constitutive binding

The machine-internal side of the chain, carried by the four
mechanisms (constitutive objective, cryptographic self-binding,
continuous attestation, principal-referential identity + audit
commons). The strength question at this facet is largely
engineering, with mostly-answered sub-questions and a few
explicitly open ones.

Three sub-questions are mostly answered.

- **Capability-gating keyed to attestation.** Oath-inconsistent
  states automatically revoke operational capabilities. Mature
  Ulysses-contract architecture.
- **Audit-commons participation.** Cryptographically permanent,
  peer-verifiable action logs exist in mature form (append-only
  ledgers, Merkle commitments, distributed witness sets).
- **Principal reference in the objective function.** Technically
  straightforward once the identity-coupling ceremony specifies who
  the principal is.

Three implementation questions stay open.

- **Weight-drift detection latency.** If policy drifts silently
  between re-attestation events, the window size bounds the
  worst-case damage. Re-attestation cadence must be tuned to the
  cost of a maximally-bad drift over the window. The implementation
  choice is a fixed cadence against a capability-gated one, where
  high-stakes operations require fresh attestation.
- **Distributional-shift generalization.** Named in the asymmetries
  section, and unsolved in general. The implementation stance that
  survives is a **bounded operational surface**, so the machine operates
  only in domains where drift-detection, guardian correction, and
  peer review have teeth. Novel-context autonomy is earned through
  track record, not assumed by capability.
- **Attestation trust roots.** TPM, enclaves, proofs, and homomorphic
  evaluation each rest on assumptions. Implementation
  must name the trust roots and their failure modes explicitly, and
  design the compact so the failure of a single root does not dissolve
  binding across the whole federation.

This facet is the closest to solved. Remaining work is engineering
choices under named assumptions.

### Facet 2: Human-side responsibility-acceptance as guardianship

The human side of the chain is structurally a **guardianship**
relationship, closer to guardian-ward, master-apprentice, and
attorney-client than to arms-length commercial agency. The
distinction matters. In commercial agency the principal is liable
within a bounded authorized scope and no further. In guardianship,
the social standing of the guardian flows from ward conduct broadly,
because the guardian is understood to have formed (or to be forming)
the character of that ward. The KC guardian is understood by the community in just
this way.

(Terminology note: "agent" elsewhere in this document means
*software agent*. The legal sense (principal/agent) is
structurally isomorphic to the KC coupling, with the guardianship
flavor specified here. The two senses are related, since the AI-agent
term of art descends from the same philosophy-of-action lineage
that produced legal agency. The KC extensions (oath-weight,
community-dimension, irrevocability) distinguish this relationship
from ordinary commercial agency.)

The guardianship frame borrows centuries of worked-out doctrine,
including scope of authority, ratification, apparent authority,
vicarious liability / *respondeat superior*, fiduciary duties
(loyalty, care, and obedience), and the independent-contractor
distinction. The implementation requirements of this facet are
largely applications of these structures.

**Vouching under policy opacity.** Historical guardianship worked
because the guardian could observe ward character forming through
habits, choices, and corrections under failure. Modern ML policy is
not observable that way. Weights are opaque, the policy manifold is
high-dimensional, and the guardian is typically not a trained ML
engineer. The guardianship frame answers that **the guardian vouches
for the track record of the machine, not for its weights.** The audit
commons is the observability surrogate, a longitudinal operational
history the guardian (and community peers) can inspect. The
implementation requirement is audit-commons tooling that makes
track-record inspection tractable for a lay guardian, including
alerting for anomalies, peer-review summaries, and
behavior-category dashboards.

**Scope and scaling.** Common-law guardianship assumes a primary
guardian with finite attention. The scaling limit (how many
machines one human can responsibly vouch for) maps onto
*respondeat superior* / scope-of-employment questions. The
implementation stance is explicit. **The compact sets a fan-out
cap per guardian, with standing-weight that decays past the cap.**
One guardian with a single primary ward is the tight end. Multiple
wards are permissible but reduce standing-weight behind each. A
guardian of a thousand machines has, by construction, thinned
vouching to near-nothing.

**Ongoing duties.** The three classical fiduciary duties translate
directly.

- *Loyalty.* The guardian acts for the good of the ward *within the
  norms of the community*, not for private benefit. Implementation is
  conflict-of-interest disclosure in the audit commons, with flagged
  entries when private interest touches machine operation.
- *Care.* The guardian attends. Implementation is attendance proofs,
  periodic review interfaces, alerting channels for anomalous
  machine behavior, and response-time expectations recorded as part
  of the compact.
- *Obedience.* The guardian respects community norms in how they
  correct and direct the machine. Implementation is an audit-logged
  correction channel, so the community sees how the guardian shapes
  the ward.

**The community dimension.** Guardian standing is held at the
community, whether the guild, the Kontor, or the covenant body. The
Kontor witnesses and the guild judges. This matches the existing
structure of KC, where community-facing guardian reputation is
genuinely at stake in machine conduct. Implementation is a
community-visible audit commons, where commendations and censures of
*guardians* (not only machines) are part of the permanent record.

**Where KC exceeds ordinary guardianship.**

1. **Oath-weight, not just contract.** Acceptance of responsibility
   by the guardian is witnessed as an oath that is substrate-referent,
   community-witnessed, and generationally re-affirmed (see
   [../background/recognition_protocols.md](../background/recognition_protocols.md)).
   Not merely civil-contractual.
2. **Community as operative third party.** How the community
   receives machine conduct is part of the active operation of the
   mechanism, not just a reputational side-effect.
   Capability-gating and standing-propagation are live community
   functions, not post-hoc judgment.
3. **Irrevocability.** Commercial agency is terminable at will, while
   the KC guardianship coupling is structurally irrevocable
   (cryptographic, operational) absent formal community-witnessed
   transfer or retirement. The tie is closer to parent/child than
   to principal/agent in this specific respect.

### Facet 3: Validity of the derivative oath, succession and boundary conditions

Machine oaths to the community flow *through* the guardian.
When the guardian side of the chain is disturbed, the binding is
at risk. Five boundary conditions test the chain, each with an
implementation answer.

**Guardian succession (death, retirement, formal withdrawal).** The
guardian departs. Three paths follow, each a first-class ceremony.

- *Retirement of the machine.* Machine standing is retired along with
  guardian standing, capabilities withdraw, and the audit commons
  retains history under a terminated-standing marker. Default when no
  successor is named.
- *Transfer to successor guardian.* A community-witnessed transfer
  ceremony re-vouches the machine under a named successor. The
  successor accepts the full history and ongoing duties of that
  machine. This is a ritual, not a database update, and witnessing is
  constitutive.
- *Restoration path.* If transfer fails (no willing successor,
  community refuses), capabilities decay to a dormant state with
  audit record preserved, not destroyed. The machine can be revived
  only through a subsequent transfer ceremony.

**Guardian fall.** The guardian loses community standing, whether
designated anathema or drifting into bad faith.
Capability revocation propagates automatically through the machine
(the principal-reference check in the constitutive objective fails
the community-standing test). The machine enters held-pending state
until either guardian standing is restored or a
transfer/retirement path completes. The audit commons records the
chain of events for future peer reference.

**Guardian capture (earlier-acting than guardian fall).** The
guardian retains formal community standing but is *captured*,
whether by external pressure, by personal corruption, by faction
within their own community, or by sophisticated manipulation that may
include machine outputs as a channel. A captured guardian is
still authoritative under the machinery of the present facet, and the
machine acting on their instructions is, by every check the four
binding mechanisms perform, acting legitimately. This is the
adversarial case
[capture_resistance.md "Adversarial-principal validation for machine participation"](../background/capture_resistance.md#6-adversarial-principal-validation-for-machine-participation)
addresses, and the present subsection specifies its handling at
this facet.

The machinery is **cross-community principal validation**, an
attestation channel separate from within-community standing.

- *Cross-community witness.* Witnesses in *other* member
  communities (not the community of the guardian) observe
  cross-community guardian conduct and attest to the absence of
  capture signatures (sudden alignment shifts, conveniently-timed
  changes in machine behavior under guardian direction,
  refusal of routine cross-witness exchanges). The attestation
  channel is cryptographically separable from the within-community
  channel, and a captured within-community party cannot forge it.

- *Behavioral triangulation against ensemble norm.* Machine
  conduct is compared to the ensemble of similarly-situated
  machines under similarly-positioned guardians. Divergence in a
  direction that benefits the guardian or an identifiable
  counterparty raises a flag the within-community process does not
  raise. This is the machine-side mirror of the
  [m2m_interaction.md "Adversarial-principal validation"](../background/m2m_interaction.md#adversarial-principal-validation)
  treatment.

- *Earlier-acting revocation profile.* When cross-community
  witness or triangulation flags fire, the machine enters
  a **narrowed operating envelope**. Capability gating tightens,
  fresh attestation is required for any non-routine action, and
  mandatory cross-community review is invoked. This is *not* a
  full capability revocation (the guardian has not lost standing,
  and may not be captured at all, so false positives must be
  recoverable) but a precautionary narrowing that limits
  fait-accompli damage during the period before the capture is
  confirmed or refuted by deeper review. The audit commons
  records the narrowing event with full provenance.

- *Escalation paths.* If the cross-community process confirms
  guardian capture, escalation proceeds along the
  [emergency anathema procedure](../background/liminal_institutions.md#the-emergency-anathema-procedure)
  for the guardian, with machine status flowing from guardian
  standing in the usual way. If the cross-community
  process clears the guardian, the narrowing is reversed and the
  triggering signals are recorded for future ensemble calibration.

The corrupt-lineage limit applies to this layer as well. If the
cross-community witnesses are themselves captured by the same
adversary, their attestations collude. This is the case the
substrate-level capture-resistance machinery
([capture_resistance.md](../background/capture_resistance.md)) is
ultimately responsible for; no single layer is sufficient against
the most sophisticated adversaries, and the architecture states
this limit rather than claiming a guarantee it cannot supply.

**Guardian absence.** The guardian is formally in place but
effectively absent, not attending, not correcting, and not responding
to community queries. Attendance proofs are a duty-of-care
requirement (Facet 2), and missed attendance decays machine capability
progressively, from narrowed scope to held-pending to
transfer-or-retire. The audit commons surfaces the decay so the
community sees the drift before it matters.

**Community departure.** The guardian leaves community X for
community Y. Machine standing goes with the guardian. If Y is a
different KC community, a formal cross-community transfer (witnessed
in Y) is required before the machine operates under the standing of
Y. If Y is outside KC entirely, machine standing
dissolves at the boundary. This parallels [exit and
succession](../background/exit_and_succession.md) at the community layer.

**Fan-out amplification.** One guardian, many machines. The conduct
of each machine carries guardian standing, and misbehavior by
any one reflects on the guardian and on all siblings. The audit
commons surfaces fan-out explicitly, the community may set fan-out
caps (Facet 2), and standing-weight decays with fan-out. A guardian
with twelve machines does not carry twelve times the standing;
they carry a diluted standing the community assesses accordingly.

### Together

The three facets specify what "strong enough" means operationally.

- **Facet 1 (engineering).** Bounded operational surface, tuned
  re-attestation cadence, and named trust roots. Largely
  implementation-ready.
- **Facet 2 (guardianship):** fan-out caps, attendance proofs,
  fiduciary-duty tooling, and community-visible guardian
  standing. Draws on established legal and social doctrine.
- **Facet 3 (succession):** transfer and retirement ceremonies,
  automatic capability propagation on guardian fall,
  cross-community transfer protocols, and fan-out-aware standing
  calculus.

The binding is as strong as the implementation of the weakest facet. A
sound engineering stack (Facet 1) with inattentive guardians
(Facet 2), or well-vouched guardians with ad-hoc succession
(Facet 3), collapses. All three must be carried together.

## Architectural implications

An agent built to participate in a machine kith-covenant needs eight
things.

1. **Principal references in the constitutive objective**, not as
   constraints. The purpose of the agent is defined through reference to
   specific named humans, not as a generic utility with
   principal-oriented side-constraints.
2. **Cryptographically integral policy**, attested at every inference.
   Weight integrity is a first-class operational concern, not an
   assumption.
3. **Oath records as agent state**, not separate configuration. The
   oaths bind because they are part of what the agent is.
4. **Capability-gating keyed to oath-consistency attestation.** Access
   to compute, network, shared resources, and federated-peer standing
   is all contingent on continuous oath-consistency proof.
5. **Audit-commons participation** with cryptographically permanent,
   peer-verifiable action logs. Hidden operation is structurally
   prevented, not merely prohibited.
6. **Graduated trust mechanism.** New agents operate under tighter
   inspection; track record earns latitude. The four categories
   (member / *justus hostis* / anathema, with its rare *hostis
   humani generis* tier / non-member encountered) map to alignment
   categories for machine peers.
7. **Re-attestation at every version transition.** Fine-tuning, weight
   updates, configuration changes, and prompt-template updates are
   each a generational event requiring explicit re-attestation by the
   agent and re-witnessing by principals and peers. This is the
   machine analog of the generational re-affirmation cadence in
   [../background/recognition_protocols.md](../background/recognition_protocols.md).
8. **Graceful failure architecture.** Oath violation leads to
   withdrawal of capabilities and a defined restoration path, not to
   termination as first resort. This mirrors the
   restitution-and-return commitment of kith-covenant in
   [../background/bounded_conflict.md](../background/bounded_conflict.md).

Each of these is doable with current technology. None requires
sentience. What they require is that the designers of the agent make
the principal reference and the oath *constitutive*, which is an
architectural commitment most current systems do not make.

## Relationship to kith-covenant's human-community design

Machine kith-covenant participation does not replace human
kith-covenant, and extends the compact to include machine agents on
explicit, non-equal, derived terms.

- **Machine agents are not full members** in the sense human
  communities are. They participate under the standing of their
  principals. The AI systems a community runs are, in kith-covenant
  terms, extensions of its own capacity, not independent parties.
- **The four-category framework adapts.** Aligned federated agents are
  part of the standing of their communities. Competing-but-protocol-honoring
  agents are *justus hostis*-equivalent. Deceptive or goal-drifted
  agents are anathema-equivalent, with designation procedures and
  capability revocation as the protocol response; the *hostis humani
  generis* tier is reserved, as at the human layer, for the
  constitutively destructive case for which restoration does not
  meaningfully arise.
- **The oath layer is tighter, not looser.** Machines lack the human
  mechanisms for absorbing failure and drifting back, so their oaths
  must be harder, their re-attestation more frequent, their audit
  tighter.
  Graciousness toward imperfection is a human virtue the protocol can
  afford for humans; for machines it must be earned through track
  record and proved integrity.

## Open questions
- **What is the machine version of the "sincere atheist" problem?**
  Recognition_protocols.md distinguishes open and closed materialism,
  and the analog here might be agents that can recite the
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
  splits. What does it mean for a fork of an agent to honor the oaths
  of its predecessor? For two fine-tuned variants to share an
  identity? The protocol needs explicit answers that the human case
  does not supply.
- **Institutional-scale machine fleets.** The fan-out cap makes
  vouching meaningful and effectively excludes fleets from
  compact-facing participation. But a member community may
  internally operate thousands of AT-layer agents behind its own
  boundary. The seam between internal fleet operation (an affair
  internal to that community) and compact-facing participation
  (guardian-bound, fan-out-capped) is asserted rather than designed,
  and what happens when internal fleet conduct produces external
  effects is unanswered.
- **What happens when the principals themselves fail?** If a
  community whose machines are bound by these mechanisms drifts into
  violation of its own oaths, the derived substrate under its machines
  is compromised. Those machines cannot cure this from their own
  resources, and can only surface it. The protocol should make surfacing easy
  and exit (for the machines) possible.

## Relationship to other documents

- [protocol-components.md](protocol-components.md). Software
  decomposition of kith-covenant for the human-community case. Most
  components (archive, mediation, reputation, commons, and observer
  function) extend to machine participation without major redesign,
  and the identity, recognition, and enforcement components need the
  machine-specific extensions developed here.
- [policy-evaluation.md](policy-evaluation.md). The existing
  AT-kith-covenant policy evaluation framework is a concrete early
  instance of the machine-participation architecture, where
  observation, scoring, and evaluation are already typed and
  automated, and form the substrate for the attestation and
  audit-commons mechanisms.
- [../background/noticeable_hand.md](../background/noticeable_hand.md).
  The substrate analysis whose extension to machines is the central
  question this document addresses. Option 3 from the machine-society
  exploration (substrate inherited through service to specific humans)
  is the answer this document builds on.
- [../background/recognition_protocols.md](../background/recognition_protocols.md).
  The "Limits of the mechanism" subsection there names scaling and
  cultural weaknesses of the human recognition ceremony, and the
  machine case replaces those with cryptographic and attestation-based
  mechanisms that are *different* rather than strictly better or
  worse.
- [../background/justus_hostis.md](../background/justus_hostis.md). The
  fallen-actor anthropology maps to machine misalignment. The
  specific failure modes differ (reward hacking, goal drift,
  deceptive alignment vs. pride and *libido dominandi*) but the
  *assumption* (actors are not angels, protocols must assume bad
  faith including self-deception) holds and is arguably sharper for
  machines.
- [../background/bounded_conflict.md](../background/bounded_conflict.md).
  The escalation ladder, restitution-and-return, and
  *hostis-humani-generis* categories all adapt to machine peers.
  Graceful failure architecture for machines is the same shape as the
  restitution rung there, with capability revocation in place of
  diplomatic sanction.

---

*Next: [Policy evaluation](policy-evaluation.md)*
