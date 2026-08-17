*Previous: [From cooperation to community](concept.md)*

# Founding a polity

A founding is the moment a group of cooperators stops being a group and becomes
a body. Concretely it is one signed document, the charter, that says four
things: who the members are, what the community holds above itself, what it
commits to, and how those commitments may later change.

Everything about how that document comes into being follows from the discipline
of the previous chapter. The trust substrate proposes a boundary, drawn from
whatever cooperation actually happened. The members ratify it, or ratify
something narrower, by signing. A proposal carries no authority whatever on its
own; only ratification plus co-signing yields a charter.

That leaves an obvious hole, and closing it honestly is most of what this
chapter is about. If the founders choose the roll, what stops three people
inside a thirty-node cluster from chartering the whole cluster as a polity of
themselves, with the other twenty-seven not merely outvoted but unrecorded?
Nothing can prevent it, because a genuinely separate group of three inside a
cluster of thirty is indistinguishable from a faction capturing it without a
substantive judgement no rule can make. What the tier does instead is remove the
silence: a founding must say what it excluded and why, that statement is
permanent and immutable, and anyone reading the charter later can compute for
themselves how much of the cluster consented.

## The founding act

Three record kinds carry it. A `Substrate` names what the polity stands under. A
`Commitment` states what it binds itself to. A `FoundingProposal` gathers a
proposed roll, a substrate, commitments, and everything below into one offer.

Ratifying that proposal, with a majority of the proposed roll co-signing, yields
a `PolityCharter`. Signatures verify against self-resolving identities, so no
registry is consulted and no authority issues anything.

From the charter and the records that follow it, a read model derives the live
state, being the current roll, the commitments in force, and any dissolution
with its named successors. Nothing is stored twice, and nothing is edited: the
log is append-only and the state is a function of it.

## The roll, and how it moves

Three authority rules govern membership, and their asymmetry is deliberate.

Firstly, *admission* takes a majority of the roll plus the consent of the
joiner, because nobody is conscripted into a community.

Second, *voluntary departure* is self-signed and takes nothing else, because a
community that can refuse an exit is a prison.

Lastly, *expulsion* takes a majority plus one, and sits under a self-governance
floor. The extra vote is the whole point: adding a member and removing one are
not symmetric acts, and a bare majority that can expel is a bare majority that
can manufacture itself a supermajority one departure at a time.

## Amendment, and what cannot be amended

The core of a charter has no amendment path at all. The substrate and the
founding commitments are genesis-immutable, which is the community-scoped form
of the same rule the compact tier applies to itself.

Although the core is fixed, the rest is not. Beyond the core, amendment is
asymmetric. A commitment is added at a majority and removed at a majority plus
one. Contraction is harder than expansion, everywhere in this stack, for one
reason: the argument for dropping a commitment is always available, and what
answers an argument that is always available is an asymmetry rather than a
prohibition.

## Offices, and the difference between holding and using

A polity confers roles on its own members through `OfficeChange` records over
five offices, being convener, inductor, envoy authorizer, charter keeper, and a
catch-all for anything a particular community needs.

Conferral takes a majority plus the consent of the holder. Resignation is
self-signed. Revocation takes a majority with the holder excluded from the
count, since a holder who could vote on their own removal holds the office
against the roll rather than from it.

Two structural rules keep the office system from regressing. The roll confers,
and offices never confer offices, so the founding roll bootstraps the first
officeholders with no regress and no chicken-and-egg. And a holding is effective
only while its holder is on the roll, so an expulsion or a departure lapses
every office that member held, and a readmission revives them.

Although five offices exist, an office is not merely held but *used*, and only
two of the five actually do anything.

Where an *inductor* is effectively held, one of its holders other than the
inductee must witness an induction, and the roll may proceed without that
witness only at a majority plus one. This is a defeasible gate rather than a
hard one, and the distinction is the design. An office may raise the bar for a
rite and may never block it outright, because an office that could block the
rite could strangle the community by simply declining to show up.

An *envoy authorizer* holds a delegated power. One holder signing an
`EnvoyAuthorization` stands in for the majority of the roll, which is how the
polity says who may speak for it beyond itself, in its own vocabulary rather
than the vocabulary of the compact. The envoy always co-signs rather than
self-appointing, and the key is checked to derive the identifier. Because the
majority route always stays open, filling that office costs the roll nothing: it
buys convenience and surrenders no authority.

Convener and charter keeper gate nothing in the ordinary path, and self-signed
acts are never gated by any office.

## What a founding excluded

Every proposal and every charter carries a `BoundaryAttestation`, holding the
candidate size the boundary actually showed and a reasoned exclusion for
everyone it showed whom the roll does not name.

Without it, the genesis quorum takes its denominator from the roll the proposal
itself declares, so a faction of one charters a thirty-node cluster as a polity
of itself on a single signature.

Genesis authority gates the accounting and only the accounting. The numbers must
add up, exclusions may not repeat, and nobody may be excluded and enrolled at
once. Nothing refuses a charter for being narrow. What the rule removes is the
silence, not the discretion.

Because the core of a charter has no amendment path, the attestation is
genesis-immutable by construction. A read model computes the founding legitimacy
from the charter alone, with no provider consulted and no cohesion input, so
anyone holding the charter can re-run the judgement forever and get the same
answer. It reports a consent share, being ratifiers over everyone the boundary
showed rather than over the roll the founders chose, and the faction of one
described above reads 0.033.

It also reports a founding basis, being corroborated, narrowed, or
self-attested. That basis is deliberately unordered and deliberately not an
erosion level, since a founding is not eroding and these are not comparable
harms. The whole construction is accountability rather than prevention: the
attestation is a signed claim by the proposer, and the point is that the claim
exists and is permanent.

## The claim names its author

A label saying where a boundary came from would be worthless, because flipping a
field would promote a bare assertion by a convener into a corroborated reading
with no provider consulted anywhere.

So the substrate arm carries its evidence rather than describing it. It holds
signed readings, each naming a reader, signed by that reader alone, and bound to
the founding it is offered for. The binding matters: without it, an honest
account of one cluster replays into somebody else's founding with the signature
still verifying.

The rule with teeth is identity rather than arithmetic. The roll together with
the exclusions must equal the members of some signed reading *as a set*, where a
merely arithmetic rule would let the proposer supply both sides of its own sum.

The legitimacy model reports both the readers and the corroborating readers, and
deliberately not a count of readings filed, since a reader who signed a
different boundary corroborates nothing. The gap between the two numbers is a
reader contradicting the founding, permanently, on its own charter.

That count is a lens and never a gate, because identifiers cost nothing and a
captured reader can mint its own corroborators. Which brings up the successor
mechanism below.

## Independence, attested but never enforced

A signed reading is attributable and does not distinguish one reader from five
faces of the same actor. One party minting five identifiers and signing five
identical readings grades as corroborated, the highest basis the tier awards, at
a cost of about thirty-two microseconds a voice.

An `IndependenceAttestation` is a third party account of a named subject in a
named capacity, being either boundary reader or observer. One record covers both
roles, with one set of stake buckets and one survival rule, and with a coherence
gate parameterized by role, because each is checkable against a different part
of the log. A boundary reader is fixed at genesis, where observers accrue, so
that half reads the log up to the moment of filing.

Three rules apply, being signature, sequence, and appearance. The attestation
has a sole signer. It postdates the charter. And the subject must actually
appear in the log as the thing it is claimed to be, which is a gate here where
the compact tier keeps the equivalent a lens, drawn on the line that coherence
with the record spoken about is checkable while the truth of a claim never is.

Two consequences are worth naming. A monitor must speak before it can be vouched
for, since pre-registering a witness is the sockpuppet move with a longer setup.
And a key may hold both roles while a voucher for one is not a voucher for the
other, because vouching is for a party in a capacity, and what an outsider can
speak to is a relationship rather than an identity in the abstract.

Who counts is stricter here than in the compact tier. An attestation from the
polity, from the subject itself, or from anyone on the founding roll does not
credit independence, since the founders are precisely the interested party. It
is accepted and reported separately rather than refused, because destroying a
claim somebody signed serves nobody where merely not crediting it costs nothing.
A `Controlled` arm lets somebody say the lens is being fooled, and it counts
from anyone at all: interest disqualifies praise rather than warning, so a
founder breaking ranks is heard.

The read models report independent corroboration, meaning readers who both agree
with the ratified boundary and carry a disinterested voucher, so a chorus of
five hundred sockpuppets scores zero. They report observer independence,
independent observation including the explicitly unvouched, so that an absence
of vouching is visible rather than silently read as its presence, and
independent grounding, being how many founding edges rest on a monitor somebody
was willing to be named for.

An attestation is itself unverifiable and does not pretend otherwise. It moves
the cost of faking corroboration from one key to a second party willing to be
named for it. A lens, never a gate.

## The excluded answer back

The accounting rule above cannot reach an understated candidate size, since the
archive holds no provider and the excluded are the only witnesses.

So `BoundaryDissented` exists, in two forms. A dissenter says either that they
were never named at all or that the reason recorded for their exclusion is
rejected. It is signed by the dissenter alone, involves no deciders and no
majority, and the dissenter need not be a member, since one required to sit
inside the roll could be silenced by never being admitted.

Authority checks coherence with the record contested and never its truth, so a
dissenter rejecting a reason must actually be among the exclusions, and one
claiming to be unnamed must be in neither the founding roll nor the exclusions.

Dissent is a separate read model from founding legitimacy on purpose, since
dissent accrues over time and folding it in would destroy the charter-alone
property. A dissent may be filed after dissolution, because otherwise founders
silence the excluded by winding the polity up.

## The substrate has to mean something

Non-ultimacy is the whole reason this tier has a substrate at all, so four rules
hold at genesis.

Firstly, the substrate must be *articulated*, with a non-empty name and
statement. This is presence and never content, since no rule can read a
tradition.

Second, an anchor may be neither the polity nor anyone on its roll, since a
ground inside what it grounds is no ground. This is the same closure the
guardian rules apply one level in and the succession rules apply one level out.

Third, a roll of nothing but unguarded machine parties *must* name an anchor.
The trigger is deliberately narrower than machine majority, because the
statement of a human roll is its own unkeyed anchor, and a guarded machine
already has a consenting human above it.

Lastly, a present anchor co-signs the founding alongside the ratifiers and the
guardians, because answerability is accepted and never assigned.

The legitimacy model carries a grounding, being anchored, guarded, held, or
bare, descriptive and unordered like the basis. It lives on the charter-alone
model precisely because a genesis-immutable substrate cannot accrue, so the
answer is the same forever. Bare is legal, and it is reported so that permitted
is not mistaken for grounded.

*The ground may answer back.* A `SubstrateDisavowed` is the signed withdrawal of
the named anchor, read by a separate accruing model. It asserts and it cannot
un-charter: nothing rewrites the substrate, reconstructs authority state, or
dissolves anything, since a withdrawal with that power would be a unilateral
kill switch in the hands of every anchor. A disowned polity keeps governing
itself, on a ground that disowns it, legibly. It shares two properties with
dissent, being permitted after dissolution and unprunable, and it is stricter in
one: only the key the charter names may file it, where a dissent must admit
anyone, because an omitted party is by definition absent from the record.

## Dissolution, and what survives it

Dissolution takes a majority of the roll and is terminal. No charter event
follows it.

The named successors of a dissolution are checked three ways. Duplicates are
refused, since a fan-out nobody counts is a count that misleads. Self-succession
is refused, since a polity that survives its own dissolution has not dissolved.
And a successor may not be drawn from the roll, since a member is a person or a
machine rather than a polity.

Naming no successor at all is valid, because that is the difference between a
fission and a death.

Three entry classes outlive a dissolution, which is enough to name the rule
rather than the exceptions. Evidence about the founding outlives the polity
because the legitimacy of the founding does, where governing acts die with it.

## Further reading

- [Design decisions D1 to D34](design.md): particularly D21 and D22 on the
  boundary attestation and its evidence, D23 on the substrate rules, D24 and D28
  on independence attestation, and D25 on succession.
- [The crates](crates.md): the record kinds, the read models, and where each rule
  is enforced.
- [Which human answers for which machine](guardianship.md): the derivation
  records the founding ratifies alongside the roll.

---

*Next: [Which human answers for which machine](guardianship.md)*
