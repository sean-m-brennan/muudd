*Previous: [Watching for erosion](cohesion.md)*

# What crosses to the compact

The compact tier does not know this tier exists, and keeping it that way is the
point of the whole arrangement. It sees communities and evidence about
communities. It never learns where either came from.

That is achievable only if the border is narrow and speaks the vocabulary of the
compact rather than a vocabulary of its own. Although the border carries real
weight, it is narrow. Six artifacts cross it, all of them record types the
compact already understands, and nothing else does. There is no adapter
protocol, no negotiation, and no handshake. A polity emits a signed payload and
something on the other side appends it.

The confinement is structural as well as conventional. Exactly one crate in this
tier speaks the record types of the compact, and every other crate depends only
on the neutral base. That property is checked mechanically rather than asserted,
and it is checked again whenever a crate is added.

## The six artifacts

*A community identity.* A chartered polity is handed off as a self-signed
community identity, being a root identifier and its delegation graph.
Self-signed is the correct shape here, because a community exists by its own
act. A compact is constituted by its members instead, and is co-signed
accordingly, which is a distinction the tier above draws and this one respects.

*A concern.* When the cohesion monitor grades erosion at the concern level, and
only then, this tier emits the pre-indictment conduct flag the compact already
routes, under a neutral bearing that names coherence erosion.

What it carries is early warning, and deliberately not a verdict about fission.
A concern is about conduct. A split is about whether a community still exists,
which is a category a conduct record should not be made to carry. The tier
declines to decide a split at all, which is not modesty but the recognition that
no measure can make that judgement and a measure pretending to would be worse
than none.

*A fission.* A ratified split emits no new type whatsoever. The tier charters
the successor polities, emitting some number of new community identities not
fewer than two, a signed successor link to each, index-aligned, and the
dissolution of the predecessor charter naming them all.

Making it N-ary rather than binary matters. A three-way split is one act rather
than two chained fissions recording an intermediate polity nobody ever lived in.
Four guards apply, being arity, distinctness, and verification. The successors
must number at least two, must be pairwise distinct, and must not collide with
the predecessor. And *every* successor must carry a verifying self-signature,
absent which a link is signed under the root of the predecessor to an identity
whose own signature does not verify.

From the far side, the compact sees several new communities seeking recognition
and an old one going quiet. That is entirely within vocabulary it already has.

*A fusion.* The mirror, being some number of predecessors not fewer than two
merging into one community identity, with a successor link and a charter
dissolution per predecessor naming the merged polity as sole successor. The
compact sees several communities go quiet and one new one seek recognition.

*An envoy delegation.* An effective envoy standing becomes the speaker
delegation of the compact, scoped as an envoy. This is the only border artifact
an office produces, and it takes the derived envoy state rather than the
archive, which keeps the crate that speaks compact vocabulary from having to
read the log.

*A member roll.* The published roll of the polity, under its root key.

## Why the roll had to cross

The last artifact is the youngest and the reason for it is worth following,
because it is a case of one tier honestly recording a limit and naming the other
as the fix.

The whistleblower channel of the compact tier proves that a source is an insider
by a ring signature over the key set of the accused community. That set was the
root key plus the live speaker delegations. The compact models communities and
speakers and has no member roster at all, so an insider holding no delegation
was not in the ring, and a community that had delegated nobody had no anonymous
channel whatever.

The compact recorded that as an honest limit and named this tier as the feed,
since a polity is exactly the thing that does model membership. Every member
here carries a self-resolving identifier, so each verifying key comes out of the
identifier itself and nothing else has to travel.

*What measuring first found was the larger half.* The compact recomputed the
insider set at verification time against the *current* archive tip, while
evaluating it as of the filing time, and required exact set equality. So the
accused community, the one party with a motive, held two levers. Backdate one
delegation ahead of the filing, and every filing against it stopped verifying.
Backdate a revocation, and the suspect set around a source narrowed after the
fact. Both were measured. Going from three keys to four invalidated a filing,
and going from three to two narrowed one.

A member roll bolted onto that would have inherited the defect and added a
second lever to it. The ring is now computed over the records preceding the
filing in insertion order. That is the principle the escalation ladder of the
compact already stated, that authorizing an earlier transition never sees a
later one, and it is the principle the observer channel had not applied.

It is the same failure the guardian co-signature rules name on this side of the
boundary, and the sentence generalizes past both: authority must be a property
of the log rather than of when you looked.

*Rolls are unioned and never superseded*, which is the rule of the compact tier
and the one that decides what publishing a roll here means. Once named an
insider, always an insider, because the alternative hands a polity a way to
shrink the suspect set around a member the day after they left. So the roll here
shrinks on a departure and the ring there does not. The asymmetry is deliberate,
and a departed member keeps the channel they had.

One honest limit remains, and it is stated rather than papered over. A roll is
the polity speaking about itself, and the compact checks that the publisher is
who it claims to be rather than that the roll is true.

## What deliberately does not cross

Naming the omissions is as useful as naming the artifacts.

Firstly, no observation crosses. The evidence a monitor gathered stays on this
side, and what goes over is a graded conclusion with a bearing. The compact tier
has observer machinery of its own, and needs no second stream of raw readings
from underneath it.

Second, no reputation number crosses. Earned reputation is a measurement, and
conferred standing is an adjudication, and wiring one to the other numerically
is the category error this tier was built to avoid rather than to commit at its
own border.

Third, no office crosses except through an envoy delegation. Internal offices
are internal, and the compact has no business knowing who convenes a member
community or who keeps its charter.

Lastly, no archive dependency crosses. This tier emits serialized signed
payloads that a consumer on the far side appends, and it never depends on the
archive crate of the compact tier. That keeps the coupling to record shapes
rather than to storage.

## Where Part III ends

A polity now exists in the strong sense. It has a boundary that somebody
ratified and that says what it excluded. It has a ground it stands under and a
record of who withdrew if anybody did. It has offices conferred by its roll and
lapsing with it. It has a rite by which members actually receive what it stands
for, and a measure of whether that rite is getting cheaper. It has a monitor
that can tell the difference between quiet and stopped. And it can hand all of
that to a stranger as six artifacts in a vocabulary the stranger already speaks.

What it does not have is any relation to that stranger. Two chartered polities
on opposite sides of a ridge, or of an orbit, share no history, no founding, and
quite possibly no ground. They cannot merge, cannot submit to one another, and
cannot appeal upward, since nobody stands above both. They will nonetheless need
each other.

That is the subject of Part IV, and it is the oldest unsolved problem in this
book.

## Further reading

- [Design decisions D1 to D34](design.md): D4, D9 and D10 for the upward
  interface and its confinement, D14 for the envoy delegation, D25 for N-ary
  fission and fusion, and D26 for the roll.
- [Protected dissent](../../kith-covenant/doc/jester.md) and
  [whistleblower cryptography](../../kith-covenant/doc/architecture/whistleblower-crypto.md):
  the ring the roll feeds, and what it now computes over.
- [The AutonomousTrust integration](../../kith-covenant/doc/architecture/autonomous-trust-integration.md):
  the boundary from the far side, including what the compact tier deliberately
  refuses to learn.

---

*Next: [Cooperation without a sovereign](../../kith-covenant/doc/concept.md)*
