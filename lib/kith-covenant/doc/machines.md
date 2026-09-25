*Previous: [Protected dissent](jester.md)*

# Machines as delegates

Some of the parties here are software, being instrument controllers, navigation
aides, communications mediators, and eventually general-capability agents. A
compact that governs communities across light-lag will be operated substantially
by machines, because a decision that has to wait forty minutes for a human is
not a decision anybody can operate on.

Our stance on that fact is one sentence. **Machines never originate substrate.**
A machine inherits it from a named human principal, who inherits it from a
community.

Two claims follow, and both are stronger than they look.

*The reduction claim.* Every machine-to-machine exchange reduces, without
remainder, to an exchange between two named humans whose guardianship binds each
machine.

*The non-emergence claim.* There is no machine-native legitimacy, and there is
no machine-to-machine cooperation without a human on each end. Machines do not
form compacts. They do not admit each other. They do not recognize each other in
the sense this book has been using, because recognition is an act performed
before something one holds above oneself, and a machine holds nothing.

The tier below supplies the two names. A chartered polity records which of its
members are derived parties and which human answers for each, ratified at
founding with every named guardian co-signing, because answerability is accepted
and never assigned.

## What derivation actually means

Three facets of human participation have machine analogs, and reading the
mapping wrong is the most common error here.

*Faith* becomes principal-grounded purpose. A machine does not believe anything.
Its objectives are grounded in what its principal authorized, and the grounding
is a fact about the delegation rather than about the machine.

*Virtue* becomes policy shaped by the practice of the principal. A machine has
no dispositions. It has behavior that a principal shaped, and whether that
behavior expresses a virtue is a question about the principal.

*Oath-as-character* becomes oath as constitutive objective. A human who swears
an oath is changed by it in a way that persists whether or not anyone is
watching. A machine that is bound by an oath has that oath compiled into what it
is optimizing, which is a different kind of thing and must not be confused with
the first.

Those asymmetries are the reason the machine layer cannot simply reuse the human
layer with different nouns.

## The handshake and the envelope

When two machines meet, five steps run.

Firstly, each presents an attestation. Second, each verifies the standing of the
other principal, in both directions. Third, each proves continuous binding to
its principal, being a cryptographic attestation that its objectives, weights,
and trust roots have not drifted since the last re-attestation. Fourth, the
protocol computes the *authority envelope*. Lastly, the signed envelope becomes
the first artifact written to the audit commons for that encounter.

The envelope is the *intersection* of what each principal has authorized, never
the union. Anything outside it is inadmissible by definition, which is what
gives the machine layer its clearest boundary. No question of what a machine may
do goes unanswered by an object both principals signed.

*The ratchet rule.* When two machines meet with different escalation ceilings,
the lower governs. A more-authorized agent cannot drag a less-authorized one up
the ladder. Without this, delegation becomes a laundering mechanism, since a
principal who wants a thing done past their own ceiling need only find a
counterparty whose ceiling is higher.

*Fork and merge.* A machine that forks does not inherit the standing of its
parent, and both children require fresh attestation. A merge requires the
principals to concur on which oath-history is authoritative. Both rules exist
because standing that survives a fork is standing that can be multiplied for
free.

## Standing under light-lag

Reputation crossing a compact boundary is stale by construction, and we treat
staleness as a quantity rather than as a binary.

Standing tokens carry an explicit issuance time, a half-life, and a decay curve.
Stale data is *discounted* rather than rejected. A six-month-old token is still
useful for routine operations, less useful for high-stakes commitments, and
never enough to justify a substantive decision in a fast-moving situation.

Revocations override unexpired tokens regardless of which arrived first, because
the alternative lets an adversary win by controlling delivery order.

The behavior under uncertainty is *defensive narrowing*. A machine that cannot
establish current standing narrows its envelope rather than falling back to a
default, since a default in either direction is exploitable and a narrower
envelope is merely inconvenient.

## Crossing the boundary

Three categories of data cross between the trust substrate inside a community
and the compact outside it, and how we categorize them is the interface.

*Compact-facing commitments* always cross. The recognition statement, subsequent
compact-level behavior, adherence to the common virtues, contributions to the
commons, and response to mediation.

*Aggregated internal health* crosses with consent, by zero-knowledge proof.
Population stability, resource adequacy, the rate of dispute resolution, and
continuity of generational re-affirmation. The compact learns that internal
health is above threshold without learning the underlying state.

*Specific violation reports* cross when external effects occur. Internal conduct
producing external harm, and whistleblower attestations that are anonymous by
ring signature while verifiably from insiders.

What deliberately does not cross is the reputation score itself. Earned
reputation is a measurement and conferred standing is an adjudication, and
wiring one to the other numerically is a category error rather than an
integration.

## Detection, and where it stops

We are honest about how far the compact can see into a machine that is
misrepresenting itself, and the honesty is the point of the section. Three
mechanisms work, within limits.

*Continuous attestation* catches drift within the attestation window. It cannot
catch anything that happens between windows, and the window cannot be closed
arbitrarily far because attestation costs round trips that light-lag makes
expensive.

*Behavioral triangulation* compares a machine against the ensemble of
similarly-situated peers and flags divergence, particularly divergence in
directions that benefit the principal. It cannot catch a whole ensemble that is
wrong together.

*Envelope-edge fallback* returns to the principal for anything at the boundary
of what was authorized. It cannot help where the harmful act is comfortably
inside the envelope.

Past those, against a sufficiently capable adversary with perfected mimicry, the
protocol acknowledges its limit. Detection is bounded by *constraining the
envelope* rather than by any claim of perfect discrimination, and what remains
is the substrate-level capture machinery and the jester.

That is not a comfortable place to end a chapter, and stating it is preferable
to the alternative. A protocol that claimed to detect arbitrary deception would
be making exactly the kind of promise the anthropology of this project forbids.

## Bootstrap, and first contact

Machines carry a first-contact encounter *back to their principals* rather than
authorizing anything new.

Two machines meeting with no compact relation between their communities may
exchange courtesy, safety-relevant data, and distress response, which is the
reduced envelope of the non-member-encountered category applied at the machine
layer. Anything beyond that is referred.

This is a scoped answer rather than a solved problem, and it is one of the
standing holes we track. A compact must be formed among communities that already
meet its preconditions. The compact does not generate the preconditions, and
neither do the machines that operate it.

## Further reading

- [Machine-to-machine interaction](background/m2m_interaction.md): the reduction
  and non-emergence claims, the handshake and envelope, the ratchet rule, and
  detection bounded by envelope.
- [Machine participation](architecture/machine-participation.md): the three
  facets mapped, cryptographic self-binding, continuous attestation, and the
  asymmetries with human oath-binding.
- [Reputation bridges](background/reputation_bridges.md): the interface with the
  trust substrate, the four categories of data, and reputation under light-lag.
- [Which human answers for which machine](../../ethne/doc/guardianship.md): where
  the two names in the reduction claim actually come from.

---

*Next: [Capture resistance](capture.md)*
