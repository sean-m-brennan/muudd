*Previous: [Objections](background/objections.md)*

# The crates, and what each one holds

The Rust workspace is fifteen crates plus a demo. This is the reference
inventory. It says what lives where, which rule each crate enforces, and what
measuring each slice actually found. The readable path through the same material
is Part IV of the MUUDD narrative, beginning at [cooperation without a
sovereign](concept.md), and the component decomposition it implements is
[Protocol components](architecture/protocol-components.md).

## `kc-core`

Shared primitives and the `Alg` crypto enum. It is neutral, holding no
governance records and depending on no other governance crate. The polity tier
one level down depends on this crate and on `kc-log`, and on nothing else here.

## `kc-log`

The append-only, content-addressed log, generic over the entry type. This is the
archive *mechanism* rather than the archive, with generation-safe cursors and
the versioned dump and segment format.

It is neutral in the same sense `kc-core` is, holding no governance records and
depending on no other governance crate, which is what lets the polity tier build
its own archive on it rather than forking one.

## `kc-identity`

Ed25519 `did:key` identities, delegations, and witnesses.

It also holds `CompactCharter`, which gives a compact an identifier of its
own, co-signed by a quorum of its members and by its own key. The asymmetry
with a community is deliberate. A community identity is self-signed because
a community exists by its own act, and a compact is constituted by its
members instead.

## `kc-ceremony`

The two-layer recognition ceremony, the five-movement recitation, and
sponsorship records.

## `kc-conduct`

Attributable, append-only conduct records, including the pre-indictment
`Concern` the jester raises. These seed the escalation and reputation
components.

It also carries the *attested null*, being an `ActivityAttestation`: the signed
account by a community of one window in a standing function it holds, whose
empty act list is the point of the record. Silence is then either evidence or a
fact about the compact, where before it was missing data and a quiet pool could
not be told from a stopped one.

## `kc-mediation`

The mediation slice holds per-community mediator registries whose union is the
pool, dispute records, argument records, recusal requests and their answers,
protecting-power designations, and paneled decisions. The panel rule, being
registered, non-party, and at or above the minimum, is enforced by the authority
pass of the archive.

### Argument records

An `ArgumentRecord` is the structured argument submission and evidence packaging
the specification calls for: append-only, self-signed, and held to a different
rule per role.

Firstly, a `Party` argument is the case of a party, and only the complainant or
the respondent may make one. Second, a `Panelist` argument requires an active
registered mediator from neither party community, and that mediator's own
signature, since nobody argues in a mediator's name.

A decision cites the arguments it answers. The pass requires one cited panelist
argument per co-signer, because three signatures on one author's text is a
rubber stamp rather than a panel. It also requires that every party argument
already in the log is answered, because a panel may not decide while ignoring a
case put to it. Insertion order is authoritative, so a later filing cannot
unmake a concluded mediation.

The record also made two existing fields mean something. The specification calls
for a *mandatory* reasoning trail and nothing enforced it, so a decision could
publish an empty `reasoning`. And `MediationDecision.panel` was read by no rule
at all: it could name the two *party* communities, precisely whom non-overlap
excludes, while three legitimate non-party mediators signed, and the record was
authorized. Both are now checked, the panel field against the co-signers it
claims to report.

### The pool is capped

`MediatorCaps` puts mediator seats through the same `check_roster_caps` that the
examiner caps and the observer caps read: bounded terms, a sit-out between them,
the continuous-service union, and no single community holding a majority of the
active pool once the pool is past a floor.

It was the third roster and the one wired to nothing, so a community could hold
nine of ten seats and staff every panel in the compact. Although that was the
larger hole, it was not the only one. A mediator could be seated with
`valid_until: None`, a seat with no end, which is a permanent mediator class and
the first step toward an inquisitorial caste. The whole fixture suite was
seating mediators that way, which is what repairing it cost.

### Recusal, protecting powers, and lineage

These three complete the pool directory the specification asks for.

A party names one panelist on one dispute and states grounds. The request is
recorded and answered, granted only by that mediator stepping down and never
refused by that same mediator.

A `ProtectingPower` is the Geneva analogue, being a non-party community standing
for a disputant, co-signed by both, since the duty is never conscripted and a
protector a disputant did not accept is a watcher imposed on them. And
`MediatorEntry.trained_by` records the apprenticeship.

None of the three gates anything. The mediation decides exactly as it would
have, and the lineage is read only by the pool board, because the doctrine that
asks for the record also warns against a certification barrier that becomes a
gatekeeping mechanism.

### Panel consent

`PanelConsent` is consent to the panel as constituted: self-signed, because
nobody consents on another's behalf, and naming the panel in full, so that
consent to three mediators is not consent to a fourth.

Objection had a record and structural consent had a record. The ordinary case,
being that this panel with these mediators may decide my dispute, was inferred
from the party having filed an argument, which is a different act.

It gates one thing and deliberately no more, since a consent precondition on
deciding would hand either party a veto over its own dispute. A `Settled`
outcome is a claim that *the parties* settled, so both must have consented to
the panel recording it. Before the rule, a panel could record a settlement
neither party had agreed to, and every fixture in the test suite was doing
exactly that.

### Sealed decisions

`SealedReasoning` on a decision plus a `DecisionUnsealed` that opens it, over
the same `ThresholdCustody` the observer component holds a source identity
under.

What is sealed is the reasoning. The dispute, the panel and the outcome stay
public, so a compact can see that a matter was decided, by whom, and how it came
out, and every rule above keeps running. What waits is why.

An unsealing must present enough opened shares to recover the text. That text
must open the commitment the decision published, so a custodian quorum cannot
publish a reasoning the panel never wrote.

The custodians are *member communities rather than mediators*, and that was a
finding rather than a preference. The whitepaper asks for the keys to sit with
the mediator pool *and* for a deterministic reveal at twenty-five years. A
mediator seat is capped at two years. So a share sealed to a mediator is held by
a key the archive stops resolving two decades before the seal opens, which
leaves the reveal dead by construction and the signature on the opening
unverifiable. Community identities resolve at any instant.

The substance of the doctrine is kept by arithmetic: no party may hold a share,
and no quorum of shares may sit with the communities of the deciding panel,
because a panel that can unseal itself has deferred publication at its own
discretion rather than sealed anything.

## `kc-provenance`

The manufactured-event records of the fourth capture move, and the authority the
mediation component holds over them.

An `EventReport` carries provenance tags and the qualifying triggers its
originator declared. A `SkepticismInvocation` is the *mediator pool* placing an
event under the structured-skepticism protocol on its own initiative, and a
`SkepticismReview` is the internal review of the pool against pretextual use.

The hold always had teeth, since an action premised on a qualifying,
under-corroborated event is refused rather than merely flagged. But *qualifying*
was four booleans the originating community set on its own report, so the teeth
rested on the word of the party the protocol exists to constrain. Measuring it
showed both halves. An originator that declared its own event mundane was never
held, and a full non-party mediator pool sitting in the same archive could do
nothing about it. Either source now places the hold, and the window is the
longer of them.

The review runs against the *hold* rather than the invocation record, which is
what lets it reach the self-declared case, being the one that answered to
nobody, where a community could put its own urgent matter beyond action with no
reviewer anywhere on the record. A `Pretextual` finding releases the event from
a hold of either origin. Mediators who co-signed an invocation may not review
it, the originating community may neither invoke nor review, and reasoning is
required on both answers, because an audit legible only when the hold falls says
nothing about the holds that stood.

## `kc-commons`

The commons slice holds the five compact-level commons, Ostrom-style boundary
definitions, usage accounting, and the graduated-sanction ladder from monitoring
to exclusion with the heavier levels quorum-gated.

*A commons may state a `draw_allowance`*, which is the machine-readable half of
a provision rule and the only part of Ostrom's proportionality a rule can act
on. Before it, the written cap bound nothing: a record drawing ten times it was
authorized exactly like one that honored it, and the two were indistinguishable
to the archive.

*Usage may be sealed.* The specification asks for a surveillance threshold with
configurable disclosure levels, and a community that must publish every figure
to account honestly is disclosing its capacity, its consumption pattern and its
reserves to every other member. A sealed record commits its figures, publishes a
zero-knowledge proof that the draw is within the allowance, and puts the figures
themselves under threshold custody keyed to fellow appropriators, a quorum of
whom can open them for cause. The reporter may not be one of its own custodians.

Sealing is not a way to disappear, and measuring is what showed it would have
been. `free_rider_signals` summed the figures, so a community sealing 1,850
units read as having consumed zero and ranked nowhere. That reads not as absent,
but as apparently the most scrupulous member of the compact. The lens now counts sealed
records, reports whether the accounting is complete, and turns each proof into a
bound on what is hidden, so its totals are honestly a floor rather than
dishonestly a figure.

*`CommonsEscalation` is the commons equivalent of the anathema designation*,
which the research corpus lists under open questions. Four conditions hold it,
in the shape the four-condition test carries. The subject is at exclusion. It
has arrived there more than once. It has stood there long enough that
restoration was available and not taken. And one arrival suffices on a
life-critical commons, which the compact keeps running when it pauses everything
else. It gates nothing itself; a designation may cite it, so the ladder is
untouched and the threshold lives in the refusal to record the finding.

*The inter-compact level is Ostrom's eighth principle made concrete.* A
registration carries its appropriator level, and at the compact level the
appropriators are whole compacts. The rule that makes nesting real rather than
decorative is subsidiarity: the higher level may not reach past a compact to its
member communities. A federation that could discipline a community directly
would be governing at the scale of the compact while answering to nobody at it.

## `kc-archive`

The content-addressed, cryptographically verifiable audit commons: the typed
record enum, identifier-to-key resolution, verification, and the authority pass,
over a `kc-log` archive.

## `kc-escalation`

The ladder state machine through to expulsion and the substantive-re-engagement
rung, the four-condition anathema test, restitution obligation and verification,
and re-entry.

Two additions are worth naming. The expedited path carries a *covenant-facing
floor*: an expedited designation must cite an attested adverse act by the
subject, since relaxing the first condition otherwise leaves the emergency route
able to rest on records about the internal life of a community, or on
identifiers naming nothing. And the *recorded conclusion of a mandated
re-examination* makes a confirmation answer its deadline and set the next one,
while a lapsed designation may not be cited by a further escalation act.

## `kc-reputation`

The reputation-bridge slice, being lookup plus standing over Unknown,
Identified, InCompact, Probationary, Denied and Anathema, together with the
capture-signal detectors and the jester lenses.

The lenses cover the rejection pattern, ceremony-cost decay, and the composition
graph, plus interpretive plurality, structured skepticism, and the adversarial
principal. The erosion alert carries sixteen signals, seven of them absolute,
including silent and unstaffed standing functions, a grievance the compact filed
and did not pursue, enforcement landing on the communities that raised matters,
and a compact enforcing a shrinking part of the object its own members recited.

Records are authorable from Python, with append-only delegation revocation
and authority enforcement covering principal binding, scope, and quorum.

## `kc-observer`

The observer-corps registry with enforced term, rotation, and pool-share
caps; attributed cross-community reports with an irrevocable right of
response; and the whistleblower channel.

A source proves insider standing by ring signature, and the reception panel
holds its identity k-of-n. Each share is sealed to the registered *encryption*
key of a mediator, which is distinct from its signing key and checked against
it, so custody cannot be named for one party and sealed to another. A no-panel
exculpation exists for anyone wrongly named.

The corps also owes an account of each window it holds seats for, including an
account of having seen nothing, so a corps that stopped looking is
distinguishable from one with nothing to see.

The ring spans the root key of a community, its live delegations, and its
published member roll, which the polity tier feeds. It is computed over the log
*preceding* each filing, so no later record can invalidate a filing or narrow
the suspect set around its source. The construction is in
[The whistleblower source](architecture/whistleblower-crypto.md).

## `kc-constitution`

The compact-incapable list as a genesis-immutable constitutional artifact with
asymmetric amendment, expansion being easy and contraction harder; the
structured-fast cadence; and the graded erosion alerting the reputation
component reads.

## `kc-pyo3` and `src/python/kith_covenant/`

The PyO3 bindings, built into `kith_covenant._core` by `maturin develop`, and
the Python package that wraps them.

The package holds `ceremony.py` over the bindings, and the autonomous-trust seam
in `interface.py`, `policy.py` and `reputation.py`, which imports with or
without the compiled extension.

## The harness

`harness.py` scores fourteen scenarios against falsification criteria stated in
advance. The ten are the seven strengthening moves, with the seventh counted
twice for who a faction can seat and what a seated entrant can carry, plus the
two with no adversary at all, being the quiet polity and the abandoned office.

It runs continuously over a parameterized space with seeded search, directed
sweeps and shrinking. That apparatus was added for a specific reason: running
the harness two hundred times proved the scoring function was a constant,
returning the same answer regardless of input, which meant it had been measuring
nothing.

## The source-protection feature

`source-protection` is a default-on feature on `kc-core`, so that the polity
tier stops linking the observer cryptography and its dependency graph drops from
sixty-one crates to fifty-three.

One toolchain hazard is worth recording, because it cost real time.
`default-features = false` is *silently ignored* on a member crate whose
dependency is declared with `workspace = true`.

---

*Next: [Protocol components](architecture/protocol-components.md)*
