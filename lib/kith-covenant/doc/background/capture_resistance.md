*[Sustainable Space](../../../../../../doc/sustainable_space.md) > [MUUDD](../../../../doc/muudd.md) > [Kith-Covenant](../kith_covenant.md) > Capture Resistance*

# Capture resistance: architecting against instrumentalization

## A note on this document's posture

This document treats capture resistance as a **unified architectural concern** of
kith-covenant. The motivation is
[control_system_hypothesis.md](control_system_hypothesis.md)'s Component 3: the
observation, originating in Vallée's *Messengers of Deception* (1979) but
generalized to any anchor-above-self substrate, that the same machinery enabling
long-running cooperation is *structurally* exploitable by bad actors who control
the interpretation of the substrate. Several kith-covenant components carry
specific countermeasures within their own scope (the jester role, the audit
commons, the emergency anathema procedure, the exit paths); the architectural
claim this document adds is that these are pieces of the same problem and must
interlock. It names the seven strengthening moves that follow from that claim
and routes the load-bearing pieces to their host documents, where local
implementation work can proceed.

The posture is operational, not theoretical. Each strengthening move is stated
with enough specificity that an implementor can recognize it in code and a
mediator can recognize it in practice. Open questions are flagged where the
architecture is not yet settled.

## The problem: capture as continuous structural property

Capture is conventionally treated as a discrete failure: a faction takes over
an institution, a sovereign-temptation tips a liminal body into rulership, a
charismatic leader gathers a cult. The discrete framing fits some historical
cases (the early papacy's sovereignty drift; the late EU institutions' partial
capture by their own bureaucracy; cults that crystallize around a single
figure). But Vallée's Component 3 names a more general structural property:
**any anchor-above-self substrate is vulnerable, all the time, to the
instrumentalization of its interpretation.** Capture is not a moment that
happens once; it is a continuous pressure that the substrate must resist
continuously. The discrete failures are what the continuous pressure produces
when resistance fails locally.

The implication for protocol design is sharp. Capture-resistance machinery
must be *always operating*, not invoked only when capture is suspected. Periodic
audits, post-hoc anathema procedures, and reactive jester reports are
necessary but not sufficient. They treat capture as exception. The strengthening
moves below treat capture as the standing condition that the substrate's
operational practice must resist by structural design.

A second implication: capture resistance is not solely a protocol-layer
concern. It operates at the substrate layer (member communities' internal
practices), the protocol layer (kith-covenant's mechanisms), and the
implementation layer (the software that carries the protocol). Effective
capture resistance interlocks across all three. A protocol layer with strong
capture resistance built on substrate communities that have been individually
captured fails; a substrate layer with healthy practice running on a captured
implementation fails; and so on. The architecture below is layered.

## The seven strengthening moves

The first six moves emerge from
[control_system_hypothesis.md](control_system_hypothesis.md)'s analysis of
Component 3 and from the historical pattern of capture in long-running
cooperation regimes; the seventh emerges from the observation that the six
defend everything except the gate through which the compact's own
composition is decided. Each is stated as a structural commitment, then
routed to the host document where its operational specification lives or
will live.

### 1. Interpretive plurality as a structural requirement on members

**The capture vector.** Capture most often runs through a *monopoly on
interpretation*. A single faction within a member community gradually becomes
the canonical interpreter of that community's substrate (its scriptures, its
sacred narratives, its recognition-statement Movement 1), and the
substrate-deference machinery quietly becomes faction-deference machinery.
The compact's compact-level non-ultimacy does not help here, because the
capture has happened *inside* a member community before the protocol layer
ever sees it.

**The strengthening move.** Each member community must demonstrate
**multiple traceable interpretive lineages** of its own substrate, with the
divergences visible to the compact via the audit commons. Communities are
not required to *agree* across their internal lineages (disagreement is
the condition that makes the lineages substantive), but they must
demonstrate that no single faction has cornered the interpretive function.

This is theologically uncomfortable for traditions whose interpretive
authority is structurally unitary (the Roman Catholic magisterium, certain
Reformed confessions, hierarchical Orthodox jurisdictions). It is achievable
in those traditions in modified form: the magisterium has historically
operated under the constraint of conciliar disagreement, the Reformed
tradition has accommodated multiple confessions and theological schools,
and Orthodoxy holds a polycentric structure of autocephalous churches.
What the requirement excludes is communities whose substrate is interpreted
by a single living individual or a single closed body whose deliberations
are not visible to peers within the community.

The Buddhist sangha's tolerance of multiple lineages
([buddhist_sangha.md](buddhist_sangha.md)) and the Christian conciliar
tradition both demonstrate that interpretive plurality is achievable
without dissolving substrate coherence. Centralized clerical hierarchies
are the harder cases; the compact engages them through requirement that
internal conciliar processes be substantive, not nominal.

**Routing.** Specified in
[recognition_protocols.md](recognition_protocols.md). Anti-spiral feature
examination at recognition-formation must include evidence of interpretive
plurality. Audit commons (per
[m2m_interaction.md "Audit commons for M2M"](m2m_interaction.md#audit-commons-for-m2m)
and the human-side analogue in
[reputation_bridges.md](reputation_bridges.md)) records evidence of
plurality over time.

### 2. The substrate-deference test: operationalizing non-ultimacy

This is the move whose rationale most needs careful development, because
it asks the compact to perform a kind of restraint that is easy to assert
and hard to demonstrate.

**The capture vector.** The fifth anti-spiral feature
([justus_hostis.md "non-ultimacy of the political community"](justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient))
holds that the compact must not be ultimate, that it answers to something
above itself. Stated as an assertion, this claim is **unfalsifiable**. Any
compact, in any historical period, can declare non-ultimacy in its founding
documents and continue to act in every operational moment as if it were the
ultimate. The Jacobin Cult of the Supreme Being formally acknowledged a
transcendent reference; this did not prevent it from executing the
insufficiently virtuous in the name of that reference. The Soviet
constitution formally acknowledged international worker solidarity above
the state; this did not prevent the state from acting as ultimate in
practice. *Non-ultimacy as assertion does no work.*

**Why an operational test is required.** For non-ultimacy to function as a
load-bearing feature (for the substrate-above-self framework to actually
prevent the compact from drifting into ultimacy), the compact must
**demonstrate** non-ultimacy, not merely assert it. The demonstration must
be observable, repeatable, and structurally incorporated into the compact's
ordinary operation, not reserved for hypothetical moments of capture.

**What demonstrating non-ultimacy actually means.** A compact that answers
to something above itself will, in observable cases, **restrain itself when
it has the power to act and an interest in acting**. An ultimate body
faces no such restraint by definition; it acts whenever its interest and
capacity coincide, because there is no higher authority to defer to. A
non-ultimate body, in contrast, periodically encounters cases where it
*could* act, *would* benefit from acting, and *chooses not to* because
the matter falls outside what its substrate authorizes it to decide.

The test is therefore: **does the compact, in observable practice,
encounter and honor the limits its own substrate places on it?** If the
answer is yes, non-ultimacy is operational. If the answer is no (if every
matter the compact takes interest in is also a matter the compact decides),
the compact is behaving as ultimate regardless of its founding language,
and capture is either complete or in motion.

**Why this is not the same as procedural restraint.** Constitutional
amendment procedures, supermajority requirements, and judicial review are
all forms of procedural restraint. They constrain *how* a body acts but do
not constrain *whether* the body is the ultimate authority within its
domain. They are, structurally, the body's own self-binding rules, which
can be modified by the body. The substrate-deference test is sharper: it
asks whether there are matters the compact will not decide *even when its
own procedures would permit it to do so*, on the grounds that the matter is
not the compact's to decide. The answer must come from the substrate, not
from the compact's own constitutional layer.

**Two structural forms of demonstration.** The historical models that have
operationalized non-ultimacy take two forms, and kith-covenant should
incorporate both.

*Categorical deference (the "compact-incapable" list).* A structurally
defined class of decisions the compact will not make, even when it
technically could and even when its members would consent to its doing so.
The Catholic magisterium operates under such constraints (certain doctrinal
propositions are not the magisterium's to revise); constitutional
democracies operate under entrenched-clause analogues (certain
constitutional provisions cannot be amended even by unanimous vote). For
kith-covenant, candidate compact-incapable matters include: the content of
member communities' internal recognition statements after compact-formation,
the theology of any member's substrate, the internal governance of any
member, the truth-claims of any member's tradition. The compact has
*interests* in these (a member that drifts toward closed materialism is a
problem for the compact; a member whose internal governance has collapsed
threatens the compact) but the compact's *response* to such interests is
recognition-related (anathema designation, etc.), not first-order decision
on the matter itself. The compact-incapable list must be specified at
compact-formation and protected by the same genesis-immutability that
protects the recognition statement's five movements
([recognition_protocols.md "The capture attempt via recognition itself"](recognition_protocols.md#the-capture-attempt-via-recognition-itself)).

One item that is *not* compact-incapable, and that must not be filed there by
analogy, is the thin functional minimum the compact has adopted as the ground of
its conduct toward parties that have not joined it
([natural_law.md](natural_law.md#the-ground-the-compact-claims-for-itself)). The
compact-incapable list names matters belonging to members that the compact will
not decide. The minimum content decides nothing belonging to a member; it binds
the compact's own conduct, which is the one thing the compact has always been
free to bind, and it grants no writ inside any member. The distinction matters in
the direction of protection rather than permission. Because the minimum content
is a restraint on the compact rather than a deference to members, the pressure on
it runs the same way pressure on the deference categories runs: an argument for
narrowing the forbearance owed at the frontier is always available and always
pragmatic. Contracting the minimum content therefore carries the same asymmetric
*higher* bar this document specifies for contracting the compact-incapable list
(see the probation treatment under move 7 below), and its contraction is a
capture signal on the same terms, observable to members, recordable by the audit
commons, and nameable by the jester.

The bar is a rule and not a statement about one. The adopted ground is its own
constitutional artifact, quorum-adopted once, and a narrowing is refused unless it
carries the contraction quorum, counted as weight so probationary entrants cannot
supply it. One difference from the compact-incapable list is deliberate: the list
keeps its founding matters immutable, and a founding forbearance is contractible at
the higher bar. What the doctrine asks for here is an asymmetry answering an argument
that is always available, which is a bar. A lock would say instead that the argument
is never legitimate, and the compact does not know that in advance.

*Temporal deference (the structured-fast pattern).* Periods of structured
non-action during which the compact's ordinary operations are suspended on
all but life-critical commons. The historical models are the liturgical
calendar (Lent, Advent, Sabbath, Yom Kippur), the agricultural sabbatical
year (*shmita*), and the Iroquois Condolence period during which mourning
nations were not consulted on confederacy business. The structural function
is identical across these forms: an institution that has ceased
self-restraint cannot easily reinstate it; an institution practiced in
periodic non-action retains the *capacity* to defer, which is the active
ingredient in non-ultimacy. For kith-covenant, the analogue is a defined
period (annual, generational, or both) during which compact-level
mediations, sanctions, and decisions are not initiated, only continued
where life-critical interests require it. The mediator pool, the audit
commons, and the rescue infrastructure remain operational; the
deliberative and adjudicative functions pause.

**The diagnostic value of the test.** Beyond its operational function, the
substrate-deference test is **diagnostic of capture**. The earliest sign
that a compact is drifting toward ultimacy is that the deference categories
and periods begin to erode: exceptions multiply, periods shrink, the
compact-incapable list contracts under pressure of pragmatic argument. The
contraction is observable. Members and observers can track it; the audit
commons can record it; the jester function can name it
([jester_and_dissent.md](jester_and_dissent.md)). A compact whose
substrate-deference test is shrinking is a compact in early-stage capture,
even if no individual decision in the contraction is obviously illegitimate.

**The deeper claim.** Non-ultimacy, like virtue more generally, is a
*disposition* rather than a *compliance*
([justus_hostis.md "the anthropology"](justus_hostis.md#the-anthropology-the-substrate-must-assume)).
It cannot be fully encoded in rules. The substrate-deference test is the
operational practice that **forms and maintains** the disposition. A compact
that periodically and structurally encounters the limits of its own
authority *acquires the habit* of recognizing those limits in novel cases
the test did not specifically anticipate. A compact that has never deferred
loses the capacity to recognize when it should, and capture has nowhere it
must stop.

**Routing.** Specified in
[justus_hostis.md "non-ultimacy of the political community"](justus_hostis.md#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient)
as the operational corollary of the fifth anti-spiral feature. The
compact-incapable list and the structured-fast cadence are
constitutional-layer commitments per
[protocol-components.md "Amendability tiers"](../architecture/protocol-components.md#architectural-decisions).

### 3. Ceremony cost as tracked quantity

**The capture vector.** Recognition ceremonies are load-bearing because of
the asymmetric psychological cost they impose on members
([recognition_protocols.md "Asymmetric psychological cost"](recognition_protocols.md#asymmetric-psychological-cost-a-closer-look)).
That cost is what binds the recitant to the substrate the ceremony names:
the body knows; the self-perception updates; the social commitment is
public and accumulating. But ceremonies tend to **ossify into credentials**:
the asymmetric cost decays as the ceremony becomes routine, generations
inherit the form without the cost, and what was meant to bind has become a
procedural formality. After a few generations, recognition events that
once cost the recitant are now passed through without engagement. The
compact has formally functional members whose substrate engagement has
hollowed.

**The strengthening move.** Track ceremony cost as a measurable quantity
across recognition events and re-affirmation cycles. The measurement is
necessarily indirect (psychological cost is not directly observable), but
**proxies are observable**: time and attention given to preparation, public
articulation of what the recitant is committing to and what they are
forgoing by committing, witness reports of recitant comportment, content
of the anti-spiral-feature examination exchange, the recitant's
substantive responses (rather than rote affirmations) to challenge
questions. Decay in the proxies across re-affirmation cycles indicates
ceremony erosion.

When the proxies show consistent decay over multiple cycles within a
member community, the compact's response is not anathema (the community
has not actively rejected the substrate) but **substantive
re-engagement**: a heavier re-affirmation cycle in which the compact's
witnesses examine more demandingly, the recitant prepares longer, and
the cost is reasserted at the level the original compact-formation
required. The Christian sacramental tradition's analogue is the periodic
*mission* preached by visiting clergy to refresh ordinary parish life;
the Buddhist sangha's analogue is the periodic ordination renewal; the
Iroquois analogue is the renewal of the chiefs' titles at each new
generation. Each tradition has independently discovered that ceremony
cost decays without reinforcement, and has built reinforcement into its
ordinary practice.

**Routing.** Specified in
[recognition_protocols.md "Periodic re-affirmation"](recognition_protocols.md#periodic-re-affirmation).
The proxy-tracking mechanism is part of the audit commons
([m2m_interaction.md "Audit commons for M2M"](m2m_interaction.md#audit-commons-for-m2m)
and the human-layer analogue in
[reputation_bridges.md](reputation_bridges.md)). Substantive re-engagement
is a designated rung between ordinary re-affirmation and anathema
designation.

### 4. Manufactured-event resistance protocols

**The capture vector.** Vallée's Component 3 documents specific historical
cases in which bad actors deliberately generated **ambiguous-transcendent
events** to direct interpretation. UFO contactee cults consolidated around
manufactured contactee narratives; intelligence agencies seeded paranormal
material into target populations to shape belief; entrepreneurial religious
movements arranged "spontaneous" miracles. The structural pattern: when an
ambiguous event of high interpretive charge appears, communities with weak
event-evaluation discipline tend to interpret it in whichever direction
the loudest interpreter pushes, and the manufacturer of the event is
reliably the loudest interpreter, having had time to prepare framings that
the ambient community has not.

In KC's setting, the analogue scenarios are concrete:

- A "first contact" event (genuine or fabricated) whose interpretation
  could leverage epistemic deference toward whichever member community
  controls the framing.
- An anomalous-physics or anomalous-energy result whose interpretation
  could shift commons-allocation decisions or compact-level resource
  commitments.
- A rescue-need scenario, manufactured or exaggerated, that exploits
  commons obligations to extract resources beyond what genuine need
  would warrant.
- An ambiguous attack or commons violation, attributed-by-implication
  to a member faction the manufacturer wants to discredit.
- A "rediscovered" historical interpretation of a member community's
  substrate that conveniently supports a current power play within or
  across communities.

**The strengthening move.** A **structured-skepticism protocol** for
events meeting specific criteria: high interpretive charge,
conveniently-aligned with one faction's interests, sudden onset, and
originating from witnesses with stake in the interpretation. The protocol
does not prejudge. It does not assume the event is manufactured. It
imposes structural delay and corroboration requirements that **buy time
for the ambient community to develop interpretation**, depriving the
manufacturer of the loudest-interpreter advantage.

Specific elements:

- *Structured delay before action.* Compact-level decisions in response
  to qualifying events are delayed by a defined interval: long enough
  for member communities to develop independent interpretation, short
  enough that genuine emergencies (rescue, life-critical commons) are
  not blocked. The interval scales with interpretive charge.
- *Multi-witness corroboration requirement.* The interpretation cannot
  be advanced by the originating party alone. At minimum, witnesses
  from multiple member communities must independently corroborate the
  observable facts (separable from any interpretation of them) before
  the interpretation enters compact deliberation.
- *Cross-substrate translation.* The interpretation must be translated
  into multiple member communities' substrates and the translations
  compared. An interpretation that survives only in the originating
  substrate's framing (that fails to render coherently into adjacent
  substrates) is not necessarily wrong but is structurally suspect.
- *Provenance tagging in the audit commons.* The interpretation is
  tagged with its originator's identity and incentives. Future
  reference to the interpretation carries this tag.
- *Stake disclosure.* Parties advancing the interpretation must
  disclose what they stand to gain from each available framing. Failure
  to disclose, when interest is later discovered, becomes evidence of
  bad faith.

The historical model is the canonical Catholic process for evaluating
apparitions, which is famously and deliberately slow: alleged
apparitions go through years or decades of investigation before any
authoritative judgment, and most are never judged at all. The slowness
is the point: the structure denies the loudest-interpreter advantage by
ensuring the interpretation must survive a long process during which
ambient interpretation can develop. The analogous KC process need not be
that slow; the criterion is "long enough that the originating party
cannot dominate the framing", calibrated to the event class.

The negative historical model is the court of the 1692 Salem witch
trials, which is what the absence of every element above looks like
in practice. The
court admitted spectral evidence: unfalsifiable, single-witness
testimony from stake-holding accusers, with no corroboration
requirement, no structured delay, no stake disclosure, and no
provenance record. And it spared confessors, who characteristically
named others, adding a cascade incentive on top of the open channel.
The result ran from first affliction to twenty dead in under a year
and jumped town boundaries in weeks. Every element of the
structured-skepticism protocol is, in effect, a rule the Salem court
lacked, and the episode is the concrete case for why the protocol
must already be standing when the qualifying event arrives: no
community mid-cascade has ever adopted one. (The fuller Salem
treatment, including how the community's own substrate eventually
terminated the cascade, is in
[justus_hostis.md "The Salem witch trials"](justus_hostis.md#the-salem-witch-trials-the-features-can-be-held-and-still-defeated).)

**Routing.** Specified in
[liminal_institutions.md](liminal_institutions.md) as part of the
mediator pool's standing competency. The structured-skepticism protocol
is invoked by the mediator pool on its own initiative when qualifying
criteria are met, with mediator-pool-internal review checking that the
invocation itself is not used as a tactic to suppress legitimate urgent
matters.

### 5. Light-lag-honest decision classes

**The capture vector.** At AU-scale distances, a faction within the
compact can complete a capture before the wider compact has the
information needed to recognize it. By the time the report of an action
arrives at the rest of the network, the fait accompli is in place:
treaties signed, commons reallocated, decisions executed. The
existing treatment in
[commons_federation.md "Light-lag and eventually-consistent commons"](commons_federation.md#light-lag-and-eventually-consistent-commons)
addresses this as a *governance latency* problem and adopts eventual
consistency. Vallée's framework reframes the same physical fact as a
**capture vulnerability**: eventual consistency means the wider compact
might *eventually* recognize that capture has occurred, but cannot
*prevent* it within the operational window the captor has.

**The strengthening move.** Formalize **which classes of decision can be
made within light-lag and which structurally cannot**, with cryptographic
enforcement of the distinction. Decisions of the first class (local
operational choices, commons drawdowns within pre-agreed thresholds,
mediator-pool internal procedure) are valid when made by an authorized
local body acting within its envelope. Decisions of the second class
(new mediations begun, sanctions imposed, designations made, compact-level
amendments) produce **protocol artifacts that fail to validate without
inputs from the wider network**. A faction acting unilaterally in the
second class is not merely producing an outcome that the wider network
will later disapprove of; it is producing an outcome that the protocol
itself flags as illegitimate the moment validation is attempted.

The cryptographic mechanism is straightforward: second-class decisions
produce signed records whose validation requires multi-party signature
from a quorum spanning multiple communities at multiple distances, with
the quorum requirement tuned so that no single AU-region's faction can
satisfy it alone. The signatures must arrive within a defined latency
window of the decision; signatures arriving later are accepted only if
the decision has been re-issued within the window. This mechanism makes
second-class decisions *impossible to fait-accompli* without faking
signatures from communities the captor does not control, which is the
protocol-level violation the audit commons surfaces immediately.

The classification of decision types (which class which decision falls
into) is a constitutional-layer commitment, set at compact-formation,
modifiable only through the same heavyweight procedure that the
recognition statement itself requires. A faction attempting to capture
the compact by reclassifying decisions from second to first class is
attempting an obvious protocol amendment, which is itself a second-class
decision.

**Routing.** Specified in
[commons_federation.md "Light-lag and eventually-consistent commons"](commons_federation.md#light-lag-and-eventually-consistent-commons)
as a refinement of the existing eventual-consistency treatment.
Implemented in the protocol's signature-and-quorum mechanism per
[protocol-components.md](../architecture/protocol-components.md).

### 6. Adversarial-principal validation for machine participation

**The capture vector.**
[machine-participation.md](../architecture/machine-participation.md)
specifies machines as derived-substrate parties whose binding flows from
their named human principal, the guardianship model. The binding's
strength depends on the principal being a faithful guardian whose own
community standing is at stake. But Vallée's Component 3 says **the
principal can themselves be captured**: by their community's drift, by
external pressure, by personal corruption, by manipulation through the
machine's own outputs (a captured principal feeding the machine instructions
that further the capture). A machine acting on a captured principal's
instructions is, by the existing protocol, acting legitimately. The
guardianship model treats principal-loyalty as a primitive without asking
who validates the principal.

**The strengthening move.** Principal validity must not rest solely on the
principal's own attestation or on the principal's own community's
attestation. **Continuous attestation must be cross-witnessed by parties
with no shared stake in the principal's current interest.** The technical
machinery already exists in the audit commons and the federated-peer
attestation framework; what is missing is the explicit specification that
principal-validation is a separate concern from machine-attestation, with
its own requirements.

Specific elements:

- *Cross-community principal witness.* The principal's continued good
  standing in their own community is necessary but not sufficient. A
  separate attestation chain (from witnesses in *other* member
  communities who have observed the principal's conduct in
  cross-community interactions) must agree that the principal's
  current operation does not show capture signatures. The attestation
  is cryptographically separable from the within-community attestation,
  so the within-community party cannot forge it.
- *Principal-output triangulation.* The machine's behavior is
  triangulated against the behavior of similarly-situated machines under
  similarly-positioned principals. A machine whose conduct begins to
  diverge from the ensemble in a direction that benefits its principal
  *or its principal's identifiable counterparty* is flagged. This is
  the machine-layer analogue of the manufactured-event resistance
  protocol (item 4): the structural skepticism is built into ongoing
  machine operation, not invoked only on suspicion.
- *Principal-fall propagation, expanded.*
  [machine-participation.md "Guardian fall"](../architecture/machine-participation.md#facet-3-validity-of-the-derivative-oath-succession-and-boundary-conditions)
  treats the principal's loss of standing as triggering capability
  revocation in their bound machines. The expansion: principal *capture*
  (which may not yet have produced loss of standing) must trigger a
  narrower, earlier-acting form of revocation (capability narrowing,
  fresh attestation requirement, mandatory cross-community review)
  before the capture has progressed to overt failure. The detection
  mechanisms are the cross-community witness and the triangulation
  above.
- *Demonic-principal handling.* The most adversarial case is a principal
  who has been captured deliberately by a sophisticated adversary
  (faction within their own community, external state actor, religious
  manipulator, AI-mediated influence operation) and whose machines are
  consequently directed in service of the capture. The
  [m2m_interaction.md "Demonic detection under light-lag"](m2m_interaction.md#demonic-detection-under-light-lag-opacity-perfected-mimicry)
  treatment acknowledges that detection is not guaranteed against
  sophisticated adversaries; the present strengthening is that
  detection is at least *attempted* by the cross-community machinery,
  not deferred to the principal's own community.

The technical analogue is multi-party computation with adversarial
honesty assumptions: validity emerges from the agreement of parties who
do not share the captor's incentive. The social analogue is the
apostolic-succession move where authority requires external attestation
rather than self-claim, and the conciliar move where teaching authority
requires multi-bishop agreement.

**Routing.** Specified in
[m2m_interaction.md "Demonic detection under light-lag"](m2m_interaction.md#demonic-detection-under-light-lag-opacity-perfected-mimicry)
and
[machine-participation.md](../architecture/machine-participation.md)'s
Facet 2 (guardianship) and Facet 3 (succession). The cross-community
witness is a new attestation channel in
[protocol-components.md](../architecture/protocol-components.md)'s
identity-and-standing component.

### 7. Admissions integrity: the gate as capture surface

**The capture vector.** The six moves above defend interpretation,
ceremony, events, decisions, and principals. None of them defends the
gate. Whoever shapes anti-spiral-feature credentialing shapes compact
composition over time, and composition control is the *legitimate*
path to the collusion threshold: a faction that steers admissions
does not need to corrupt any sitting member to assemble, entrant by
entrant, the coordinated fraction beyond which
([bounded_conflict.md "The collusion vulnerability"](bounded_conflict.md#the-collusion-vulnerability))
the institutional architecture serves the captors. The vector is
slower than any other in this catalogue and harder to see, because
every individual admission is procedurally clean. It is aggravated by
the new-entrant evidence problem: credentialing examines operative
practice, not creed
([justus_hostis.md "The Salem witch trials"](justus_hostis.md#the-salem-witch-trials-the-features-can-be-held-and-still-defeated)),
but a community without a long record offers little practice to
examine, so the gate is at its weakest exactly where the entering
party is least known.

**The strengthening move.** Admission is treated as a compact-level
decision with the same structural protections as the other
second-class decisions, and the credentialing function is rotated,
witnessed, and monitored like the mediator pool it resembles.

Specific elements:

- *Credentialing panels by lot and rotation.* The examiners who
  conduct the anti-spiral-feature examination are drawn by lot from
  member communities, serve fixed terms, publish their reasoning,
  and rotate out, with cross-community composition requirements. A
  standing admissions cadre would be the single most capturable
  body in the compact, for the same reasons a standing mediator
  class would be.
- *Admission as a second-class decision.* Admission and readmission
  validate only with multi-community quorum signatures under the
  decision-class machinery (move 5). No AU-region's faction, and no
  single sponsor cluster, can admit alone.
- *Sponsor stake.* The vouching structure already specified in the
  inter-community layer
  ([recognition_protocols.md "Inter-community layer"](recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing))
  is given teeth: a sponsor's standing rides on the entrant's
  conduct through the probationary period, recorded in the audit
  commons. Sponsorship that repeatedly precedes misconduct is
  itself a capture signature.
- *Composition-drift monitoring.* The observer corps maintains an
  admissions dashboard alongside the substrate-deference dashboard:
  who sponsored, who examined, which admissions clustered by
  region, lineage, or sponsor. Clustering in directions that
  benefit an identifiable faction is flagged before the faction
  reaches deliberative weight, while the compact can still respond.
- *Probation as the load-bearing record.* For entrants without a
  long practice record, the probationary period is where the record
  accrues. Probationary members do not carry deliberative weight in
  **any second-class decision** until the period completes:
  admissions, mediator selection and designation procedures are
  where it bites first and hardest, but the withholding is not
  limited to them, because the reason is not: otherwise each
  admitted entrant immediately amplifies the faction that admitted
  it, and a faction one community short of the quorum tops itself up
  with the entrant it just vouched for. What the entrant keeps is
  its first-class voice (a local body acting inside its own
  envelope) so probation is a season without compact-wide weight
  rather than a year of silence. Where a second-class act carries an
  asymmetric *higher* bar (contracting the compact-incapable list,
  shortening a fast), that bar counts weight too; a floor that
  counted signatures where the ordinary quorum counted weight would
  be the cheaper of the two routes, which is the opposite of what an
  asymmetry is for.

The historical analogue is the catechumenate, which was never only a
teaching period: it was the church's protection against entryism,
a span of years in which the candidate's practice became examinable
before the candidate's voice carried weight. The failure analogue is
the institution that lowers its gate under growth pressure and
discovers, a generation later, that its electorate was assembled by
its opponents.

**Routing.** The vouching and examination machinery is specified in
[recognition_protocols.md "Inter-community layer"](recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing);
the panel-rotation and quorum requirements route to
[protocol-components.md](../architecture/protocol-components.md)'s
recognition component (§2) and its capture-resistance test harness
(§10); the composition-drift dashboard routes to the observer corps
(§8). The probationary period is specified in the whitepaper's
reputation-bridge section.

## Architectural unification

The seven moves above are not seven independent features. They are seven
expressions of a single architectural commitment: **capture resistance is
a continuous operating concern of the substrate, the protocol, and the
implementation, woven through the compact's ordinary practice rather
than invoked only when capture is suspected.**

The commitment has implications for how the existing components are
understood:

- The **jester function**
  ([jester_and_dissent.md](jester_and_dissent.md)) is not a cultural
  affordance for protected dissent; it is a **substrate-level capture
  detector**. The jester's permission to name what others cannot
  is what makes early-stage capture (the contraction of deference,
  the decay of ceremony cost, the manufactured-event drift) legible
  before terminal capture. Promoting the jester function from
  cultural-dissent affordance to capture-resistance-substrate
  reframes its constitutional position.
- The **audit commons**
  ([m2m_interaction.md "Audit commons for M2M"](m2m_interaction.md#audit-commons-for-m2m))
  is not a forensic tool for post-hoc accountability; it is the
  **shared substrate for capture detection** across all seven moves. Every
  strengthening move above produces records the audit commons holds and
  surfaces. Promoting it from "permanent record" to "detection
  infrastructure" gives it constitutional standing.
- The **emergency anathema procedure**
  ([liminal_institutions.md "The emergency anathema procedure"](liminal_institutions.md#the-emergency-anathema-procedure))
  is not a last resort; it is the **escalation outcome** for captured
  members and machines. Its existence is what makes the gentler
  capture-resistance machinery operationally credible: without an
  endpoint, the gentler steps degrade into theater.
- The **liminal institutions' capture-resistance design principles**
  ([liminal_institutions.md "Capture resistance and the sovereignty temptation"](liminal_institutions.md#capture-resistance-and-the-sovereignty-temptation))
  apply to the compact itself, not just to its mediator pool and
  fire-keepers. Separation of power and memory, term limits, no
  self-appointment, budget control: the existing principles are the
  compact-internal version of what the substrate-deference test is
  the compact-external version of.

The unification produces a layered picture:

| Layer | Capture-resistance machinery |
|---|---|
| **Substrate (each member community)** | Interpretive plurality (move 1); ceremony cost (move 3) inherited and renewed in member's own practice |
| **Compact (kith-covenant protocol)** | Substrate-deference test (move 2); manufactured-event resistance (move 4); light-lag decision classes (move 5); admissions integrity (move 7); jester promoted to substrate-level role |
| **Implementation (software)** | Audit commons as detection infrastructure; cryptographic decision-class enforcement (move 5); adversarial-principal validation (move 6); admissions-panel rotation and composition-drift dashboards (move 7); attestation cadence tuned to capture-detection windows |

Each layer's machinery presupposes the layers above and below it.
Capture-resistance at the protocol layer assumes substrate-layer health
(if every member community is internally captured, the protocol cannot
recover); substrate-layer health is not enough without protocol-layer
machinery (uncoordinated communities cannot resist coordinated capture);
and both fail without implementation-layer enforcement (a captured
implementation can fake the protocol artifacts that the substrate
relies on).

## Where capture resistance lives in the protocol stack

In the protocol-components decomposition
([protocol-components.md](../architecture/protocol-components.md)),
capture resistance is a
**cross-cutting architectural concern**, not a single component. Each
component carries explicit capture-resistance responsibilities:

- *§1 Identity and standing* carries the cross-community principal-witness
  attestation channel (move 6) and the visible interpretive-plurality
  evidence at recognition (move 1).
- *§2 Recognition ceremony* carries the ceremony-cost proxy tracking and
  substantive re-engagement rung (move 3), and the admissions-integrity
  machinery (move 7).
- *§3 Archive / commons of memory* carries the manufactured-event
  provenance-tagging and stake-disclosure records (move 4) and the
  decision-class metadata that feeds light-lag enforcement (move 5).
- *§4 Mediation* carries the structured-skepticism protocol invocation
  authority (move 4).
- *§5 Escalation and designation* carries the substantive-re-engagement
  rung between ordinary re-affirmation and anathema (move 3).
- *§6 Reputation bridges* carries the principal-output triangulation
  data flows (move 6) and the ceremony-cost decay surfacing (move 3).
- *§7 Commons governance* carries the decision-class classification and
  cryptographic enforcement (move 5).
- *§8 Jester and observer function* is promoted from cultural-dissent
  affordance to substrate-level capture detector, with constitutional
  standing.
- *§9 Enforcement model* includes the substrate-deference test as
  an ongoing constitutional operation, not only the post-hoc
  enforcement responses.
- *§10 Bootstrapping and test harness* includes adversarial
  capture-scenario simulation (faction within member, principal
  capture, manufactured-event injection, light-lag fait-accompli
  attack, admissions capture by patient sponsorship, feigned
  substrate-deference by a strategic operator whose practice record is
  built to pass move 7, and legitimate-machinery composition capture
  driven patiently toward the Byzantine threshold) as standing
  test categories.

A new explicit element in the architecture is the **capture-resistance
test harness**, which exercises all seven strengthening moves under
adversarial scenarios. This is named in the
protocol-stack section below, and is the implementation
priority that follows from this document.

## What capture resistance does not do

Three limits.

**Capture resistance is not capture prevention.** No machinery yet
designed prevents capture absolutely. Sophisticated adversaries with
sufficient resources, time, and willingness can defeat any specific
countermeasure. What capture resistance achieves is *raising the cost
and visibility of capture* such that capture attempts are detected
earlier, addressed sooner, and contained more locally. The architecture
bounds damage; it does not eliminate the possibility of damage.

**Capture resistance does not substitute for substrate.** The strengthening
moves all presuppose that member communities have a substrate worth
defending: that interpretive plurality is genuine plurality within a
real tradition, that ceremony cost reflects genuine commitment, that
the substrate-deference test is performed before something the compact
genuinely takes itself to be answerable to. A compact built on hollow
substrate cannot be saved by capture-resistance machinery; the
machinery's signals will be technically valid but substantively empty,
and the compact will collapse on its own internal contradictions
regardless of how well the protocol is engineered. This is the point of
[justus_hostis.md "the bootstrapping problem"](justus_hostis.md#the-bootstrapping-problem):
the compact is a protector of substrate, not a generator of it.

**Capture resistance is not substrate-neutral.** The architecture
assumes that capture is a problem, that there is something worth
preserving against the captor's interpretation. A philosophical position
that holds substrates are arbitrary and any captor's interpretation is
as valid as any other will see capture resistance as nothing more than
*incumbency protection*. The compact does not hold this position; the
compact holds that some substrates carry the anti-spiral features and
some do not, that the compact's substrate carries them, and that
preserving the substrate against capture is therefore a substantive
moral commitment, not merely a procedural one. The architecture is in
this sense partisan, and the compact should say so rather than claim
a neutrality it does not have.

## Open questions

- **What is the proper cadence for the structured-fast deference
  period?** Annual, generational, both? The historical models span
  weekly (Sabbath) to annual (liturgical year) to multi-year (sabbatical
  *shmita*) to once-per-mourning-event (Iroquois Condolence). Which of
  these or what combination is appropriate for a multi-AU compact whose
  member communities have varying internal cadences?

- **How is the compact-incapable list amended over time?** The
  argument above treats it as constitutional-layer / genesis-immutable.
  But novel situations may genuinely require additions or refinements
  (e.g., AI capabilities not foreseen at compact-formation that need
  explicit designation as compact-incapable or compact-capable). What
  procedure can amend the list without becoming the entering wedge for
  capture-by-amendment?

- **How does the structured-skepticism protocol distinguish genuine
  emergencies from manufactured ones?** The protocol's structural delay
  serves capture-resistance but threatens emergency response. The
  rescue-infrastructure exception is named (life-critical commons
  proceed without delay) but the boundary is not always clear. A
  manufactured rescue-need scenario specifically exploits this
  ambiguity. What procedural protections distinguish it from a real
  one?

- **Can the cross-community principal-witness attestation be itself
  captured?** Move 6 routes principal-validity through witnesses in
  other communities, but if those communities are themselves captured
  (the corrupt-lineage problem named in
  [m2m_interaction.md "Demonic detection"](m2m_interaction.md#demonic-detection-under-light-lag-opacity-perfected-mimicry)),
  the witness chain is captured too. How does the compact handle the
  case where capture has spread across multiple member communities
  before being detected anywhere?

- **What are the failure modes of the substrate-deference test
  itself?** The test could become routinized (the structured-fast period
  becomes a vacation), captured (the compact-incapable list contracts
  under pressure), or weaponized (a faction invokes it to prevent
  legitimate compact action). Each failure mode requires its own
  mitigation, which itself requires capture-resistance, a regress the
  architecture must terminate somewhere.

- **What is the minimum viable capture-resistance configuration?** The
  architecture presents seven strengthening moves as a unified system,
  but a small first compact may not have the resources to implement all
  seven immediately. Which subset is the load-bearing minimum, and what
  failure modes do partial implementations admit?

- **How does admissions integrity (move 7) work at founding scale?**
  A compact of three communities cannot rotate credentialing panels
  that exclude every interested party, and every founding member is
  by definition admitted without a compact-side record. The move's
  machinery assumes a compact large enough for lot-drawn panels and
  meaningful quorums; the founding cohort is protected only by the
  mutual examination of the founders themselves. Where the viability
  threshold sits is the same open band as minimum viable compact
  size.

- **How do member communities with structurally unitary interpretive
  authority (Roman Catholic, certain Orthodox jurisdictions, certain
  Reformed confessions) satisfy the interpretive-plurality requirement
  (move 1) without being asked to abandon their tradition's
  ecclesiology?** The argument above sketches that conciliar and
  polycentric structures provide the plurality the requirement needs;
  developing this in detail for specific traditions is outstanding work.

## Relationship to other documents

- [control_system_hypothesis.md](control_system_hypothesis.md): the
  parent analysis. Vallée's Component 3 is the specific claim this
  document operationalizes; the seven strengthening moves are the
  unified architectural response to "manipulation vulnerabilities are a
  first-order design constraint."
- [justus_hostis.md](justus_hostis.md): the anti-spiral features
  (especially non-ultimacy of the political community) that this
  document operationalizes through the substrate-deference test.
- [recognition_protocols.md](recognition_protocols.md): the
  intra-community ceremony and cross-community grammar that move 3
  (ceremony cost) and move 1 (interpretive plurality) extend.
- [liminal_institutions.md](liminal_institutions.md): the
  capture-resistance design principles and emergency anathema procedure
  that the present unification builds on. Move 4
  (manufactured-event resistance) lives operationally here.
- [commons_federation.md](commons_federation.md): the
  light-lag and eventually-consistent commons treatment that move 5
  (decision classes) extends with cryptographic enforcement.
- [m2m_interaction.md](m2m_interaction.md): the audit commons,
  demonic-detection-under-light-lag treatment, and machine-layer
  reputation flows that move 6 (adversarial-principal validation) extends.
- [jester_and_dissent.md](jester_and_dissent.md): the licensed
  truth-teller role that the present unification promotes from
  cultural-dissent affordance to substrate-level capture detector.
- [exit_and_succession.md](exit_and_succession.md): exit modes
  including capture-driven exit; the present unification's failure
  modes connect here when capture-resistance proves insufficient.
- [reputation_bridges.md](reputation_bridges.md): reputation flows
  that carry ceremony-cost-decay signals and triangulation data
  between member communities.
- [../architecture/protocol-components.md](../architecture/protocol-components.md):
  the software-component decomposition that this document amends with
  capture-resistance as a cross-cutting concern.
- [../architecture/machine-participation.md](../architecture/machine-participation.md):
  the guardianship model that move 6 extends with adversarial-principal
  validation.

## References

*Anchor works to be expanded.*

The substrate-deference test draws structurally on:
- The Catholic understanding of the magisterium's competence and
  incompetence (Vatican I's *Pastor Aeternus*; Vatican II's *Lumen
  Gentium*; *Catechism of the Catholic Church* on the conditions of
  infallible teaching).
- Constitutional entrenched-clause theory: Schmitt, *Verfassungslehre*
  (1928); Ackerman, *We the People* (1991); Roznai, *Unconstitutional
  Constitutional Amendments* (2017).
- The Jewish *shmita* / sabbatical year tradition (Leviticus 25;
  Deuteronomy 15) and the broader Hebrew-Bible Sabbath structure.
- The Iroquois Condolence ceremony as a structured-non-action precedent.

Manufactured-event resistance draws on:
- The Catholic process for evaluating apparitions: *Normae Sacrae
  Congregationis pro Doctrina Fidei de modo procedendi in iudicandis
  praesumptis apparitionibus ac revelationibus* (1978).
- Vallée, *Messengers of Deception* (1979): the documentation of
  deliberate-manufacture cases that motivates the protocol.
- Intelligence-community analytical tradeoff literature on deception
  detection and adversarial provenance assessment.

Adversarial-principal validation draws on:
- The apostolic-succession tradition (multi-bishop ordination
  requirement) as a historical model of cross-witness validation of
  authority.
- Multi-party computation theory (Yao, 1982; Goldreich, *Foundations
  of Cryptography* vol. 2, 2004).
- The deceptive-alignment literature in AI safety: Hubinger et al.,
  "Risks from Learned Optimization" (2019); Carlsmith, "Is Power-Seeking
  AI an Existential Risk?" (2022).

See also the cross-references already in
[control_system_hypothesis.md "References"](control_system_hypothesis.md#references)
for the broader Vallée bibliography.
