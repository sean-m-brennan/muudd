*Previous: [High-trust computing whitepaper](../../autonomous_trust/doc/whitepaper/HighTrust.md)*

# The crates, and what each one holds

The Rust workspace and the Python surface over it. This is the reference
inventory. It says what lives where, which rule is enforced by which crate, and
why each boundary sits where it does. The narrative treatment of the same
material is chapters 17 through 22, beginning at [from cooperation to
community](concept.md), and the reasoning behind every rule below is recorded as
a numbered decision in [design.md](design.md).

## `en-core`

The base crate re-exports the identity, signing and archive primitives that the
compact tier holds neutrally, and nothing else. Depending on the neutral
base alone is decision D1, and it is what lets this tier run standalone.

## `en-archive`

A re-export of the append-only, content-addressed log, generic over the entry
type. This is the neutral archive *mechanism*, shared with the compact tier one
level up rather than forked from it (D32, closing a deferral in D1). Although
both are needed, verification and identifier resolution are domain concerns
rather than archive concerns, and they live in `en-charter`.

Persistence lives here too (D17). A versioned dump and load, plus an incremental
dump since a cursor and a corresponding append. So a host holding a cursor
appends what is *new* rather than re-serializing the whole log, which is what a
steadily appended observation record needs.

A cursor carries the archive *generation* and is refused across a compaction
(D20). The store is strictly append-only with no removal path, and compaction is
a charter-level operation returning a new archive.

## `en-charter`

The founding primitive and the keystone of the tier, `en-charter` holds sixteen
record kinds, the ratify-not-replace interface, the authority rules, and the
read models that derive state from the log.

### Founding and the charter

`Substrate`, `Commitment`, `FoundingProposal`, `PolityCharter`,
`CommitmentAmendment`, the roll-lifecycle `MembershipChange` over admit, depart
and expel, and the terminal `CharterDissolution`.

A `FoundingProposal` carries no authority of its own. Only ratification plus
co-signing yields a charter. Signatures verify against self-resolving
identities. The `charter_state` model derives the live roll, being the founding
roll plus admissions less departures, the commitments in force, and any
dissolution with its successors.

### Authority rules

Ratification is genesis-singleton and by majority. The substrate and the
founding commitments are genesis-immutable. Amendment is asymmetric, adding at a
majority and removing at a majority plus one.

The membership lifecycle is asymmetric. Firstly, admit at a majority plus the
consent of the joiner. Second, depart self-signed. Lastly, expel at a majority
plus one. All three sit under a self-govern floor.

Dissolution at a majority of the roll, and it is terminal, with no charter event
following it. The named successors are checked three ways (D25). Duplicates are
refused, since a fan-out nobody counts is a count that misleads. Self-succession
is refused, since a polity that survives its own dissolution has not dissolved.
And a successor may not come from the roll, since a member is a person or a
machine rather than a polity. That last is the third of the closure family,
alongside the guardian loop of D15 and the substrate anchor of D23. Naming *no*
successor is valid, because that is the difference between a fission and a
death.

### Internal offices, and their use

`OfficeChange` over `Office { Convener, Inductor, EnvoyAuthorizer,
CharterKeeper, RendezvousOperator, Moderator, Other }`. Confer at a majority plus the consent of the holder,
and in a machine-majority polity never on a derived member with no guardian
(D15). Resign self-signed. Revoke at a majority with the holder excluded.

The roll confers, and offices never confer offices, so the founding roll
bootstraps the first ones with no regress. A holding is effective only while its
holder is on the roll.

An office is not merely held but *used* (D14). Where an inductor is effectively
held, one of its holders other than the inductee must witness an induction, and
the roll may proceed without them only at a majority plus one. That is a
*defeasible* gate, since an office may raise the bar for the rite and may never
block it.

An envoy authorizer holds a *delegated power*, where the signature of one holder
stands in for the majority of the roll on an `EnvoyAuthorization` over
`EnvoyKind { Authorize, Resign, Revoke }`. This is how the polity says who may
speak for it beyond itself, in its own vocabulary, with the envoy always
co-signing rather than self-appointing and the key checked to derive the
identifier. Because the majority route always stays open, filling that office
costs the roll nothing.

Convener and charter keeper gate nothing, and self-signed acts are never gated.
The `office_state` model derives who holds what, and `envoy_state` derives the
standings in force, effective only while the envoy is on the roll and unexpired.

### Content policy (D37)

`ContentPolicy` carries what an author must meet to post in the polity's name
(`operator_bound_required`, `min_tier`, `daily_post_cap`) and what each label
does, over `LabelAction { Allow, Warn, Hide }`. `ContentPolicyAmendment`
(`CharterEntry::ContentPolicyAmended`) replaces it whole: a majority tightens it,
and a loosening, as `ContentPolicy::is_loosening_of` decides, needs a majority
plus one. The authority pass measures each amendment against the policy accepted
so far. `content_policy_state` derives the policy in force from accepted
amendments only, and the moderators: inducted holders of `Moderator`, who label
content alone. The policy artifact, the labels and the verdict belong to the
consumer (Agora's `ethne_ffi`).

### Stele profile (D42)

`SteleProfile` carries what "safe enough" means to whoever follows the polity in
Stele: `replicas`, `tier`, `require` and `ignore` (`scanner:channel` checks),
`severity_floor` (`SeverityFloor { Low, Medium, High }`), `onset_days`,
`half_life_days` and `team` (`TeamCoupling { Show, Worst }`). `check_shape` holds
Stele's floors. `SteleProfileAmendment` (`CharterEntry::SteleProfileAmended`)
replaces it whole under D37's rule: a majority tightens it, and a loosening, as
`SteleProfile::is_loosening_of` decides, needs a majority plus one.
`stele_profile_state` derives the profile in force from accepted amendments only,
with `set_by` and `seq` (the count of accepted amendments). A polity that never
amended one is under `SteleProfile::default()`, Stele's own `stele-default`.

### The chartered node-to-guardian edge

A `Derivation` (D15) says which members are derived parties and which human each
answers to, ratified at founding with every named guardian co-signing, because
answerability is accepted and never assigned.

It moves afterward by `GuardianChange` records, in three forms. Designate takes
a majority plus the consent of the guardian. Rotate needs the *incoming*
guardian consenting. Release comes either from the departing guardian alone or
from a majority recording that the human is gone. A guardian of none is an
honest chartered state rather than a missing value, and a release clears the
edge while keeping the designation, since a machine does not become human by
losing its human. No machine may guard a machine, and nothing may guard itself.
The `guardian_state` model derives the effective picture and answers whether the
polity is machine-majority, which is what conditions the office rule above.

### A claimed edge carries what showed it

A `GuardianChange` that asserts an edge presents the signed `Observation` in
which the substrate reported it, or declares `Unbacked { reason }`. The evidence
is *carried* rather than cited by identifier, because pruning under D17 would
dangle a citation and make authority a property of when you looked.

Founding derivations are gated the same way (D27), and there the asymmetry
inverts: unbacked is permitted and honest but earns nothing, because at founding
a self-report would *buy* the anchor exemption of D23. The observer may be
neither the polity nor anyone on its roll. A founder signing a real observation
about its own cluster is the same closure shape D23 refuses for an anchor and
D25 for a successor, reaching the *evidence* one level out. A named guardian
observing the edge it is about to accept does count. It is not on the roll, it
co-signs either way, and what it asserts by observing is its own answerability,
since interest disqualifies praise rather than an admission (D24).

### Transmission, and the rite

The `Affirmation` record over `AffirmationKind { Induction, ReAffirmation }`.
Admission puts someone on the roll; *induction* is the distinct step by which
they acquire the substrate, witnessed by a majority co-signing alongside the
inductee, with the ratifiers of a charter inducted by ratifying it.
*Re-affirmation* is self-signed renewal by an already-inducted member, which
since D34 carries a full re-recitation of the five movements and is refused
without one.

The rite has movements (D29). A `RiteMovementRecorded` entry carries one
movement of the rite of one member, over `Movement { Substrate { names },
Fallenness, Recognition, CommonVirtue { commits_to }, NonUltimacy { names_anchor
} }`. Those are the five movements of the recognition doctrine one tier up, in a
document that places them at exactly this tier. Each variant carries only what
its own movement can be held to, so a rule cannot check the wrong field.

The inductee recites and signs, everyone attending co-signs, and attendees must
already hold the substrate. Movements are *cited, not carried*, deliberately the
opposite of the guardian evidence above, and for the reason that rule exists. A
citation is unsafe only when what it points at can be pruned, and the prunable
predicate matches nothing but an observation. Citing is also what puts the rite
in the log *as it happens*.

Order is semantic, since movement one opens a rite and re-opening abandons an
unfinished attempt so it cannot deadlock a member. Duration is never required,
because the doctrine asks that the movements happen rather than that they take a
week.

What *is* checked is coherence with the charter, never content. Movement one
must name the chartered substrate. Movement four must commit to exactly the
commitments in force. Movement five must name the chartered anchor, and nothing
at all where the charter is bare. Fallenness and recognition are presence-only,
since no rule can read contrition. A witness counts only if present for every
movement: the completion cites the five identifiers of its rite in order and may
name only the intersection of their attendance, because five records with
disjoint signers is a rite nobody attended.

D34 extends the rite to renewal, where it was previously not merely absent but
refused. A movement now declares which ceremony it belongs to, and the claim is
checked against the state of the member rather than inferred from it. So a
renewal must recite its own rite, an induction cannot pass one off as its own,
and one re-recitation completes one affirmation. On a renewal the attendance is
*recorded rather than required*. It must be exactly who was present throughout,
so a member can neither over-report nor under-report it, and it may be nobody at
all. The signer stays the member alone, because renewal is precisely the act no
one else may condition. A reciter may not attend their own ceremony, which only
bites on renewals, since an inductee could never attend theirs. The coherence
checks bind renewals too, so a polity that amends its commitments and never
re-recites becomes visible at its next renewal.

### The substrate is required of whoever governs

`seasoned_members` and `deciding_weight` are the one representation every
deliberative threshold reads. An un-inducted member may not hold an office,
refused at conferral *and* dropped from the effective holders, lapsing and
reviving as the roll rule does, and may not attend a rite.

The denominator is the *inducted* roll rather than the whole roll, because
padding the roll while carrying no weight would make admission a way to freeze a
polity, which is the worse hazard. The machine-majority question still reads the
whole roll, because it asks what the polity is made of rather than who decides.

The `affirmation_state` model reports the induction status and last affirmation
of each current member, and `stale_members(cadence_seconds, as_of)` names the
un-inducted and the overdue, so a community that stops re-affirming shows
erosion. The freshness cadence is a query parameter rather than a charter term.
The `rite_state` model reports each rite with its movements, its *span*, being
the cost of the ceremony in time, who attended throughout, how many attendees it
lost, and which rites were begun and never finished.

### What a monitor saw

`Observed(Signed<Observation>)` (D16) carries the readings of one observer at
one instant. It is perspectival evidence, in the sense the observer reports of
the compact tier are. So it asserts nothing about the polity, and no majority is
involved, since an observation is not a decision. The observer need not be a
member either, since a monitor that had to sit inside the roll could be silenced
by expulsion. The one rule is that it be signed by exactly the observer it
names.

The `observation_window(since, as_of)` model reads those back as a window with a
span, a per-member reading count, and a per-member `unattended_span`. That last
distinguishes *never observed*, meaning not evaluated, from *observed but never
attended*. It is clamped at the `pruned_before` horizon of the window (D17), so
that pruning may only ever weaken a signal and never strengthen one.

That clamp is load-bearing rather than precautionary: `last_attested` is a
maximum over the stamps of the window, so dropping the interval holding the
freshest one would start the absence earlier and make it *longer*, which is
pruning manufacturing a concern. Relatedly, an attendance stamp may not postdate
the observation reporting it, since a monitor cannot have seen an attestation
that had not yet happened, and that rule is what makes the clamp sound.

The `decay_trends` model derives three things from the signed records of the
polity alone, with no provider involved. A widening affirmation gap, judged
against the *median* of prior gaps. A thinning induction witness turnout, never
inferred from a single point. And a roll-churn rate. It borrows the shape of the
ceremony-cost analysis one tier up, hedges included.

### The observation record can be pruned, and only it

An observation decides nothing, so dropping one cannot rewrite what the polity
resolved, which is what makes observations the one inert and therefore the one
prunable entry class (D17).

`ObservationsPruned(Signed<ObservationPruning>)` names a horizon and carries a
receipt, being how many records went plus a digest over their identifiers. The
receipt exists because a concern crossing to the compact tier rests on
observations, and evidence must not be able to vanish without a signature.

One effective charter keeper among the deciders suffices, which is the delegated
power that office otherwise has no record to exercise, but only beyond
`PRUNE_SETTLING_SECONDS` (thirty days), while the roll may prune anything at a
majority. Destroying evidence is the move a captured party makes, so the recent
record stays out of the reach of any single custodian. That constant is law
rather than a tunable threshold, since a floor the party it constrains could
lower is not a floor.

A horizon must advance. The receipt is verified while the dropped observations
are still present and treated as beyond checking once compaction has run, an
honest limit rather than a rule that would flag every historical pruning.
Applying prunings returns a *new* archive without them, and can never reach a
governing record, because the single `is_prunable` predicate that the receipt,
the authority check and the compaction all read matches nothing but an
observation.

### When to prune is a read model, not a daemon

`prune_recommendation(archive, polity, &RetentionPolicy, as_of)` (D19) answers
whether to prune, to what horizon, and who may authorize it, returning nothing
when there is nothing worth doing. It signs nothing, with the host feeding the
horizon back to the ordinary authoring path, because this tier never reads a
clock and the loop, like the storage backend, belongs to the host.

Although the host owns the loop, the safety floor is not the host to
reconstruct, and it has two halves. `live_measure_seconds` (default one day)
protects the *span* a live measure reads back over, and `min_witness_watchers`
(default three) protects the *witness count* it also needs. Both are required,
because the clamp of D17 preserves the span and nothing preserves the count.
Without the second, the same absence at the same horizon grades as a concern at
a dense observation cadence and as a notice at a sparse one. The count is per
member, since the watcher set is, so a thinly watched member can pin the
horizon. That is the correct answer rather than a deadlock, because the roll may
still prune anything by majority; what is refused is *recommending* it.

`retain_seconds` defaults to `PRUNE_SETTLING_SECONDS`, the smallest retention
that both preserves every observation-based measure and leaves the
single-custodian route open, a *shorter* retention keeping more history and
therefore being more expensive to authorize rather than less. `min_dropped`
(default one hundred) exists because a pruning record is itself unprunable:
pruning eagerly at one leaves the log 2.9 times larger than at one hundred. The
recommendation reports every open route, cheapest first, and can only ever
propose a horizon the authority check accepts.

### A founding says what it excluded

`FoundingProposal` and `PolityCharter` carry a `BoundaryAttestation` (D21,
resolving Q1): the `candidate_size` the boundary showed, and a reasoned
`Exclusion` for everyone it showed that the roll does not name. Without it the
genesis quorum takes its denominator from the roll of the proposal itself. A
faction of one then charters a thirty-node cluster as a polity of itself on a
single signature, with the other twenty-nine not merely outvoted but
*unrecorded*.

Genesis authority gates the accounting only. The numbers must add up, exclusions
may not repeat, and nobody may be excluded and enrolled at once. Nothing refuses
a charter for being narrow, because a genuinely separate group of three inside a
thirty-node cluster is indistinguishable from a faction capturing it without a
substantive judgment no rule can make. What the rule removes is the silence,
not the discretion.

Since the core of a charter has no amendment path, the attestation is
genesis-immutable by construction. `founding_legitimacy` reads the grade back
off the charter *alone*, with no provider and no cohesion input, so anyone
holding it can re-run the judgment forever. It reports `consent_share`, being
ratifiers over everyone the boundary *showed* rather than over the roll the
founders *chose*: the faction of one reads 0.033. Its `FoundingBasis`, over
corroborated, narrowed and self-attested, is deliberately unordered and
deliberately not an erosion level, since a founding is not eroding and these are
not comparable harms. This is accountability rather than prevention: the
attestation is the signed *claim* of the proposer.

### The claim names its author

The `Substrate` arm *carries* its evidence (D22, resolving Q5), as `Substrate {
readings: Vec<Signed<SubstrateReading>> }`. Each reading names a reader, is
signed by that reader alone, and is bound to the founding it is offered for.
Without that binding, an honest account of one cluster replays into somebody
else's founding while the signature still verifies. An unsigned label cannot do
this work: with one, flipping a field promotes the bare assertion of a convener
to a corroborated reading with no provider consulted.

The rule with teeth is identity rather than arithmetic: the roll together with
the exclusions must equal the members of some signed reading *as a set*, where
an accounting rule supplies both sides of its own sum.

`founding_legitimacy` carries `readers` and `corroborating_readers`,
deliberately not a count of readings filed, since a reader who signed a
*different* boundary corroborates nothing, and the gap between the two numbers
is a reader contradicting the founding on its own charter, permanently. That
count is a *lens and never a gate*, because an identifier costs nothing and a
captured reader can mint its own corroborators. This is the stance the compact
tier takes on its first capture move, with independence attestation as the
successor.

### The excluded answer back

`BoundaryDissented`, over `DissentKind { Unnamed, ReasonRejected }`, is
perspectival evidence in the exact shape of an observation. It is signed by the
dissenter alone, with no deciders and no majority. The dissenter need not be a
member, since one required to sit inside the roll could be silenced by never
being admitted.

It is the only thing in the tier that can contradict an understated
`candidate_size`, which is precisely what the accounting rule cannot reach,
since the archive holds no provider and the excluded are the only witnesses.
Authority checks coherence with the record contested and never its truth, so a
dissenter rejecting a reason must actually be among the exclusions and one
claiming to be unnamed must be in neither the founding roll nor the exclusions.

`founding_dissent` is a separate read model from `founding_legitimacy` on
purpose, since dissent accrues over time and folding it in would destroy the
charter-alone property. A dissent may be filed *after dissolution*, because
otherwise founders silence the excluded by winding the polity up, and it is
unprunable for free, since the prunable predicate matches only the observations
D17 makes forgettable precisely because they are inert.

### The substrate has to mean something

Non-ultimacy is the whole reason the tier has a substrate, so four rules hold at
genesis (D23). The substrate must be *articulated*, with name and statement
non-empty, which is presence and never content, since no rule can read a
tradition. An anchor may be neither the polity nor anyone on its roll, a ground
inside what it grounds being no ground, which is the guardian loop rule one
level out. A roll of nothing but unguarded derived parties *must* name an
anchor. The trigger is deliberately narrower than machine-majority, because the
statement of a human roll *is* its unkeyed anchor, and a guarded machine already
has a consenting human above it. And a present anchor *co-signs the founding*
alongside the ratifiers and guardians, because answerability is accepted and
never assigned.

`founding_legitimacy` carries `grounding` over anchored, guarded, held and bare,
descriptive and unordered like the basis. It lives there rather than in an
accruing read model on exactly the charter-alone test, since a genesis-immutable
substrate cannot accrue and the answer is the same forever. Bare is legal, and
it is reported so that *permitted* is not mistaken for *grounded*.

`SubstrateDisavowed` is the signed withdrawal of the named anchor, read by the
separate and accruing `GroundStanding`. It *asserts and cannot un-charter*:
nothing rewrites the substrate, reconstructs authority state or dissolves
anything, since a withdrawal with that power would be a unilateral kill switch
in the hands of every anchor. A disowned polity keeps governing itself, on a
ground that disowns it, legibly. It shares two deliberate properties with
dissent, being permitted after dissolution and unprunable for free. It is
*stricter* in one. Only the key the charter names may file it, where a dissent
must admit anyone, because an omitted party is by definition absent from the
record.

### Independence attestation

A signed reading is attributable and does not distinguish one reader from five
faces of the same actor (D24, generalized by D28). One party minting five
identifiers and signing five identical readings grades as corroborated, the
highest basis the tier awards. The cost is thirty-two microseconds a voice,
being five hundred readers in 16.2 milliseconds.

`IndependenceAttestation { polity, subject, role, … }` over `AttestedRole {
BoundaryReader, Observer }` carries the account of a third party about a named
subject, on the shape of the first capture move of the compact tier. One entry
covers both roles rather than one per role, with one set of stake buckets, one
survives-dissolution rule, and a coherence gate parameterized by role. The
parameter is needed because each role is checkable against a different part of
the log. A boundary reader is fixed at genesis, while *observers accrue*, so
that half reads the log up to the filing.

Three rules apply: sole signer, postdating the charter, and the subject
*actually appearing* in the log as what it is claimed to be. The third is a
*gate* where the compact tier keeps it a lens. It sits on the line the dissent
arm draws, that coherence with the record spoken about is checkable and the
truth of the claim never is.

Two consequences are worth naming. A monitor must speak before it can be vouched
for, since pre-registering a witness is the sockpuppet move with a longer setup.
And a key may hold both roles while a voucher for one is not a voucher for the
other. Vouching is for a party *in a capacity*, and what an outsider can speak
to is a relationship rather than an identity in the abstract.

Who *counts* is stricter than in the compact tier. An attestation from the
polity, from the subject itself, or from anyone on the founding roll does not
credit independence, since the founders are precisely the interested party. It
is *accepted and reported separately* rather than refused, because destroying a
claim somebody signed serves nobody where merely not crediting it costs nothing.
The `Controlled` arm lets somebody say the lens is being fooled, and counts
*from anyone*: interest disqualifies praise rather than warning, so a founder
breaking ranks is heard.

Four read models expose it. `independent_corroboration` counts readers who
*both* agree with the ratified boundary *and* carry a disinterested voucher, so
a chorus of five hundred scores zero. `observer_independence` is its counterpart
for monitors. `independent_observation` also reports the *unvouched*, so the
absence of vouching is visible rather than silently read as its presence. And
`independent_grounding` is how many founding edges graded as guarded rest on a
monitor somebody was willing to be named for. Each is separate and accruing
rather than a field on `founding_legitimacy`, on the charter-alone property of
that model.

An attestation is itself unverifiable and does not pretend otherwise. It moves
the cost of faking corroboration from one key to a second party willing to be
*named* for it. A lens, never a gate.

### What outlives a dissolution

Three entry classes do, which is enough to name the rule rather than the
exceptions: *evidence about the founding* outlives the polity because the
legitimacy of the founding does, where governing acts die with it.

## `en-substrate`

The pluggable `SubstrateProvider` interface, being the source of the emergent
boundary a founding ratifies (`FoundingCandidate` to `propose` to
`FoundingProposal`).

The guardian edges *travel with the boundary* (D15): a candidate the substrate
reads as a derived party becomes a proposed `Derivation`, so which human answers
for which machine is ratified rather than asserted.

The candidate says what the provider *knows* rather than merely what it saw
(D18). A `reputation: Option<f64>` distinguishes *unrated* from *rated low*. A
`derived: bool` carries the reading of the substrate about *kind*, independently
of the guardian edge, so a machine with nobody behind it is ratified as the
unguarded derivation it is. Before that it proposed silently as a human. An
unrated candidate does not satisfy a demanded reputation floor, since a positive
floor is a positive assertion and an absent reading cannot support it, and a
floor of zero demands nothing.

Change is streamed rather than diffed (D16). `SubstrateChange` deltas cover a
member appearing or vanishing, a reputation or guardian-edge move, an advancing
attendance stamp, and a boundary going opaque. They drain through a
`ChangeStream` trait kept deliberately *separate* from `SubstrateProvider`,
because an empty stream and no stream are different claims, and a convener with
a list should not have to fake one.

The provenance of the boundary travels with the candidate (D21). A
`BoundarySource` says whether a boundary is an independent reading or the word
of the convener. It rides on `FoundingCandidate` rather than being stamped at
`propose`, because a manual-founding candidate has exactly the same *shape* as
one from a trust service, and only the provider knows which it is.

`propose` records whoever the reputation floor trims, with the reason it
actually used. It distinguishes a reputation of 0.21 below a floor of 0.30 from
an unrated candidate against a floor of 0.30 demanding a reading, which is the
semantics of D18 said where an excluded party can read it. And
`propose_selecting` narrows to a named subset for one stated reason, so an
honest small founding is a first-class act rather than a floor it is not.

`ManualFounding` ships as a convener-driven provider needing no substrate
service, and consistently no stream. Such a polity still charters, and the
charter simply never pretends it was corroborated: the same honest chartered
state as the missing guardian of an unguarded machine.

That is a fact rather than a label (D22, resolving Q5). `FoundingCandidate`
carries no source field at all, because the evidence *is* the source: `propose`
takes `Vec<Signed<SubstrateReading>>`, empty meaning an assertion by the
convener. A separate label would leave a claim and its evidence as two things
that could disagree, which is the defect D22 closes.
`FoundingCandidate::reading()` yields the *unsigned* reading, since the provider
holds no keys and gains none, signing being the business of the host exactly as
D20 settled for every other authored act. `ManualFounding` is therefore
convener-asserted because it holds no reader key. A convener who genuinely
obtains a signed reading matching their roll has real corroboration and is
welcome to it; what is not free is *claiming* one.

The guardian edges travel with their backing too (D27): `build` takes an
optional `Signed<Observation>` so a live provider can present what it actually
saw, and falls back to `Unbacked { reason }` when no monitor has run. The
provider does not mint backing the rule would reject, so an honest bootstrap
proposal is one the founders must ground rather than one that fails late.

## `en-at`

The autonomous-trust provider (D5 to D8). An `AtEventSource` seam, being a mock
plus an `at-ffi`-gated embed and attach coupling, is folded into an
`AtSubstrateProvider`. That provider maps three things: the gateway-reputation
subtree of the trust substrate to the candidate roll, the verified operator to
the guardian edge of each member, and the attended-now stamp to when that
guardian was last confirmed present.

It implements `ChangeStream` by diffing each folded event against the view it
already holds, since the provider is the one place the prior reading is in hand.
It emits an attendance delta only when the stamp actually *advances*, so a
repeated identical stamp is not mistaken for renewed presence.

The default build links no trust substrate: the independence of the compact tier
from that substrate holds with the adapter present.

## `en-cohesion`

The coherence monitor, being a pure read model over the neutral base alone.

`cohesion_report` grades strain into an `ErosionLevel` over none, notice and
concern, computed off the chartered roll against the current candidate of the
provider. Although the list is long, each component is narrow. They are
attrition, reputation decay, floor breaches, guardians not present, transmission
decay (members lapsed on re-affirming the substrate, read off the affirmation
state), and lost guardian edges (a chartered edge the substrate no longer
shows). An opaque boundary caps attrition at notice.

Absent guardians are capped at notice outright, since a snapshot cannot tell a
night shift from an abandoned polity, so the signal may warn and may never
escalate. Transmission decay carries no such cap and may reach concern, because
duration is already inside that measure. Nor do lost guardian edges, for a
different reason. They compare what the polity *ratified* against what is
observed, and only over members the provider still sees, so a vanished member is
attrition rather than answerability collapsing.

D16 adds the four measures that only exist over time. A widening affirmation gap
and a thinning witness turnout are both capped at notice, because a trend is a
leading indicator whose consequence the affirmation component can already
escalate, and grading both would count one decay twice. Roll churn is a rate
rather than a trend, and carries a full notice and concern pair. And *sustained
absence* is the one signal permitted past the attendance cap: a guardian
unattended across the recorded window for at least `sustained_absence_seconds`,
*and* seen by at least `sustained_absence_watchers` **distinct observers**,
reaches concern.

Watchers, not filings (D28). Counting *observations* instead would let one key
file three times and clear a floor whose entire justification is that somebody
has been watching, so a stranger with one keypair could manufacture the
escalation this component alone is trusted with. Counting distinct observers is
arithmetic rather than a judgment about who they are. Whether the watchers are
independent *of each other* is the watchers lens, over distinct, vouched and
contested, reported beside the escalation it qualifies and read by no component.
That asymmetry is deliberate: gating an escalation on the monitor being vouched
for would let a captured polity vouch for nobody and thereby become unwatchable,
and an unvouched monitor is the ordinary state of an honest new one. The two
conditions do two different jobs: *the span is the absence, and the count is the
evidence that somebody was watching while it lasted*. What lifts the snapshot
cap is therefore a record rather than a looser threshold.

D18 adds the eleventh component and fixes a conflation the other ten rested on:
both reputation measures are computed over the members the provider actually
*rated*, reported as `rated_members`, and say nothing when nobody is rated.
Before this, an unrated roll was indistinguishable from a failing one and a
freshly founded polity graded at concern for being new. The unrated share itself
is graded, capped at notice by construction for the attendance reason, since a
young cohort and a provider that has stopped rating are the same picture. It
exists so the two reputation components cannot go *silently* dark.

D23 adds the twelfth, and it is unlike the other eleven: whether the chartered
substrate anchor has withdrawn. Every other measure grades how far something has
drifted; this one grades a single signed act by the one key entitled to make it,
so it reaches concern alone and takes no threshold at all. It is
event-triggered, and it cannot decay back, since there is no withdrawal of a
withdrawal.

D29 adds the cost of the rite, which no earlier measure could reach. A
*cheapening ceremony* is the latest completed rite being shorter than the first.
Although the median is the usual baseline, this one is measured against the
first rather than the median, because a uniform drift moves its own recent
history. D34 splits this by ceremony and adds a *cheapening renewal* as its
sibling: an induction happens once per member and a renewal recurs, so one
series would call the difference between two members a drift. *Attrition within
a rite* is the polity that turned out for the naming of the substrate and had
drifted away by the submission of the polity itself. Both are capped at notice
like the other trends. Abandoned rites are reported and deliberately *not*
graded, since nothing in the doctrine says how long a catechumenate may take.
And `nobody_holds_the_substrate` reaches concern alone with no threshold: a
polity that has expelled its way there can take no act at all, since every
majority is a majority of the members who hold it.

The read models of the report travel in one `CohesionInputs` struct of
`Option<&T>`, which puts *not evaluated* in the type rather than in a
convention. The report never decides a split.

## `en-uplift`

The upward interface to the compact tier (D4, D9, D10) and the *one* crate
holding the confined record dependencies of that tier (`kc-conduct` and
`kc-identity`, D10 and D14).

`community_identity` hands a chartered polity off as a self-signed
`CommunityIdentity`. `coherence_concern` emits the `Concern` of the compact
tier, under a neutral coherence-erosion bearing, only at concern-level erosion.

`fission` emits a ratified split of N successor identities, not fewer than two
(D25), a signed `SuccessorLink` to each, index-aligned, and the
`CharterDissolution` of the predecessor naming them all. So a three-way split is
one act rather than two chained fissions recording an intermediate polity nobody
lived in. It carries the four guards of fusion in the other direction: arity,
pairwise-distinct successors, no collision with the predecessor, and a verified
self-signature on *every* successor. Absent the last, a link is signed under the
root of the predecessor to an identity whose own signature does not verify.

`fusion` emits the mirror ratified merger, being N predecessors not fewer than
two into one merged `CommunityIdentity`, with a `SuccessorLink` and a
`CharterDissolution` naming the merged polity as sole successor per predecessor.

`envoy_delegation` turns an *effective* envoy standing into the
`SpeakerDelegation` of the compact tier, scoped as an envoy, the fourth border
artifact and the only one an office produces, taking the derived `EnvoyState`
rather than the archive.

`member_roll` (D26) publishes the roll from `charter_state` under the root key
of the polity as the `MemberRoll` of the compact tier, being border artifact
five and the feed for the whistleblower ring one tier up.

`intra_rite` (D35) is border artifact six. It carries the five-movement ceremony
of the polity as the records the intra rite of the compact takes: a `Reciter`
delegation naming the polity's `Inductor`, a witness registry naming the members
present for every movement, and the recitation, which crosses unsigned for the
reciter and the witnesses to seal.

`rendezvous_roster` (D36, module `rendezvous`) is border artifact seven and the
one that crosses to the substrate rather than to the compact: the members holding
`RendezvousOperator` with a live `RelayEndpoint`, from `rendezvous_state`, written
as AutonomousTrust's own signed `at-relay-roster` file under the root key, each relay
pinned to its operator's `did:key`. A relay whose `RelayEndpoint` lists `areas` is also
an area hub (D38), and the file then carries an `areas` object naming each hub's
geohash areas under its own hint; with no hub the object is left out and the file is
byte for byte the D36 one. It adds no compact-tier crate, only `serde_json`, `sha2` and
`hex`.

The module `stele` (D41) holds border artifacts nine and ten, which cross to Stele:
`stele_team_roster` (the root-signed `stele-team` file listing members' AT nodes with
each node's consent, the release officers from `release_state`, and the guardian edges
from `guardian_state`) and `stele_release_authorization` (one accepted release, signed
by its officer as a `stele-release` file). `consent_message` gives the bytes a member's
AT node signs. Both files are byte-pinned. Like `rendezvous`, it needs only
`serde_json` and `hex`.

The same module holds border artifact eleven (D42): `stele_profile`, the profile
the roll last decided, as the root-signed `stele-profile` file with its `seq` and
`set_by`, exported only from accepted amendments. It too is byte-pinned.

The module `compact` (D40) holds what the polity signs as a member of a compact
once it is in one, being border artifact eight: `anti_spiral_claims`,
`feature_acceptance` for a verdict on another participant's claims,
`sponsorship`, `examiner_registry`, `anathema` and `lift`. Each returns its
payload unsigned, because the key that signs it is on another device. Each
except the examining roster checks the acts the roll has in force and refuses a
record none covers, through `mandate` or, where a Join or an Admit act will do,
through `CompactState::covers` directly. The examining roster is the root's to
sign and needs no act.

## `en-store`

Durable storage for a log that can be *compacted* (D20), being the backend D17
and D19 both left owed.

An `ArchiveStore` trait generic over the entry type, so this crate needs no
`en-charter` dependency. `FileStore` holds one segment envelope per line, where
an append is a write and an `fsync`, and a full rewrite goes through a temporary
file, an `fsync` and a `rename`, which POSIX makes atomic. `SqliteStore` sits
behind a `sqlite` feature and stores *the same* segments as rows, so the two are
differentially testable. SQLite earns its place on the rewrite, which is one
transaction rather than a temporary-file dance.

The whole crate exists for one branch: same generation means append a segment,
and a changed generation means rewrite in full.

The store owns the cursor, because the hazard it guards is exactly what happens
when a caller keeps one. A host that appends past a compaction loses the
`ObservationsPruned` record and keeps the observations it dropped, and both
verification passes still say the reloaded log is fine.

A torn tail left by an interrupted write is *reported*, never skipped, and
`recover()` truncates it as a deliberate act, because silently dropping an
unparseable last line would discard a signed entry without saying so. No clock:
the loop stays with the host.

## `en-host`

What a host does on a cadence, and when (D20). `HostCycle::tick(archive,
candidate, as_of)` returns the `DueAction`s owed. Those are persist, a graded
`CohesionReport`, a raised concern when erosion reaches that level, and a
`PruneRecommendation` when the observation log is due. It assembles *every* read
model the grade can use, since a missing input makes its component report
nothing rather than complain.

Since D31 the report says so itself. `omitted_readings` names the readings a
grade was not given, `silent_measures()` the measures that therefore went quiet,
and `blind_to_concern()` whether any of them could have escalated. So an erosion
level of none and *nobody looked* are finally distinguishable. That closes the
hazard properly rather than in two tests of this crate: `grade` is pinned to
omit nothing, and a caller off this path is at least legible.

`tick` decides and computes; the host signs and does the input and output. It
takes `as_of` like every other cadence in this tier, so a year of host behavior
is unit-testable in microseconds, and it authors nothing, because raising a
concern and pruning both need keys the host holds. It covers the cadenced duties
and only those: membership, induction, office conferral and envoy authorization
are deliberate acts of the polity rather than scheduled ones.

## `src/rust/examples/polity_host`

A runnable reference host, a binary that nothing depends on, mirroring the
two-communities demo of the compact tier. Its single clock read is the only one
in ethne outside a test, apart from a binding convenience in the Python
extension where an absent timestamp means now.

It runs a simulated timeline by default so that it terminates and can be
verified, with a follow mode for the real loop.

Three monitors, rotated (D31). The retention floor of D19 counts distinct
watchers per member since D28. The single monitor this example used to hold
never cleared it, so its whole compaction path was dead: four hundred
observations to zero compactions, while the restart section printed a compaction
count as though it had run one. The Python mirror of this file is what found
that.

## `en-pyo3` and `src/python/ethne/`

The Python surface (D2). An `ethne._core` extension module built by maturin, and
a Pythonic package over it in the six parts of the tier.

`ethne.polity` holds the charter log, keys, and the five derived read models.

`ethne.authoring` holds a `Signer` plus one call per charter record. Found and
amend. Admit, depart, expel. Confer, resign, revoke an office. Designate,
rotate, release a guardian edge. Authorize, resign, revoke an envoy. Induct and
re-affirm. Record an observation, prune the observation log, and dissolve.

`ethne.monitor` holds a `FoundingCandidate`, `manual_founding`, and
`cohesion_report`. The `as_of` argument fixes the moment guardian attendance is
judged fresh against, and the call takes the decay-trends and observation-window
read models. It also holds `sign_observation`, the pre-archive counterpart of
`record_observation`, since the evidence of a founding derivation must be signed
before the charter that will carry it exists.

`ethne.host` (D31) binds `HostConfig`, `DueAction` and `HostCycle` whole, so a
Python host drives the same cadence a Rust one does rather than reimplementing
it, and `HostCycle.grade` is the only call that supplies every reading.

`ethne.uplift` holds the artifacts that cross to the compact tier, as JSON a
consumer on the far side appends verbatim.

`ethne.at` (D33) holds the autonomous-trust substrate itself. That is an
`AtProvider` folding the event stream of the substrate into a candidate, the
`SubstrateChange` stream a host observes it through, and `did_from_pubkey`,
being the mapping from a trust-substrate signing key to a member identifier that
nothing exposed before.

The ratify-not-replace discipline survives the crossing: `propose_charter`
returns only JSON, and a charter exists only once `ratify_charter` has a
majority of the roll co-sign it.

`ethne.at` takes *wire fields* rather than an assembled peer. A
`guardian_pubkey` becomes a guardian edge, and an `operator_attested_at` an
attendance stamp, only when `operator_bound` is set. That reduction is why the
fold crosses the seam rather than being left to the caller. Hand-rolled, it read
an unguarded machine as guarded and attended. The live couplings
(`AtProvider.attached`, `.embedded`) exist only in an `at-ffi` build and are
*absent* rather than present-and-raising otherwise; ask `AT_FFI` or
`at_ffi_available()`.

---

*Next: [Design decisions D1 to D40](design.md)*
