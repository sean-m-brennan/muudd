*Previous: [Speaking without being identified](anonymous_communication.md)*

# Trusting the metal

Every guarantee in the previous chapters rests on an assumption nobody has
checked. The routing is honest if the software is honest. The software is honest
if the compiler was honest. The compiler was honest if the machine that ran it
was honest, and that machine is a pile of silicon fabricated on another
continent by parties nobody in the community has met.

That regress has no bottom, and pretending otherwise is the most common form of
security theater. There is no configuration of open licenses, reproducible
builds and signed releases that terminates in certainty. What we can build
instead is a way to make the regress expensive, visible, and shallow enough that
attacking it costs more than it returns.

Three things do that work. Reviewability makes the software checkable by the
people running it, which is a much stronger requirement than a license
permitting them to read it. Supply chain tracking makes the physical parts
accountable, link by link, from wafer to pole. And reputation at risk puts a
named party behind each claim, so that a false one costs somebody something.
None of the three delivers proof. Together they change the question from "is
this trustworthy" to "who would have to be lying, and what would it cost them",
which is a question a community can actually answer.

## Open in the deepest sense

An open license is necessary and nowhere near sufficient. Most software the
community would rely on is nominally open and practically opaque, running to
millions of lines, with dependencies numbering in the thousands, a build system
nobody understands, and a binary nobody has ever confirmed corresponds to the
source.

Reviewability is a stronger property, and it has four parts.

The source must be readable at human scale. This is the constraint that produced
the small-process architecture and the unikernel image, not from aesthetic
preference but from the observation that a volunteer with an evening can review
a few hundred lines and cannot review a hundred thousand. Every line the node
runs has to be a line somebody in the community could plausibly read.

The dependency graph must be shallow and named. A component with four hundred
transitive dependencies has four hundred parties who can change what the node
does, and the community has met none of them. Our rule is that dependencies are
few, vendored, pinned by content hash, and individually justified.

The build must be reproducible. Given the same source, any two people on
different machines must produce byte-identical output. Without this, reviewing
the source proves nothing about the binary, since the compilation step is an
unaudited transformation in the middle. With it, any community member can build
the image independently and confirm that the published artifact is what the
published source produces.

Lastly, the toolchain must be bootstrappable. Reproducibility from a compiler
binary somebody handed you is reproducibility conditioned on that binary, which
is precisely the classic attack. A chain tracing back to a small auditable seed
compiler closes that gap, at real cost in effort, and we treat it as a goal
rather than a solved problem.

## Following the parts

Software can at least in principle be read. Hardware cannot, and the community
has no way to inspect a die. What it can do is track provenance and reduce how
much it has to trust any single link.

The chain has four links, and each gets a different treatment.

Silicon is the link nobody can verify and everybody depends on. A backdoored
radio chip defeats every other measure in this document, and no realistic
community project will detect one. Three mitigations are available, none
complete. Prefer parts with multiple independent vendors, so that compromising
the supply means compromising several competitors at once. Prefer old, dull,
high-volume parts over new specialized ones, since a widely deployed commodity
component is a poor place to hide something that must stay hidden. And assume
compromise anyway, which is what the end-to-end encryption and the small trusted
computing base are for. If a radio is lying, the traffic it carries is still
ciphertext, and the node still cannot be turned into a general-purpose computer.

Boards and assemblies are more tractable. They are inspectable with a magnifier
and a multimeter, the schematics for commodity gear are frequently available,
and an unexpected component on a board is a thing a careful man can find. This
is where a community with one competent electronics person gets real value.

Firmware is where the practical fight is. The radios ship with vendor firmware
the community did not write and mostly cannot read, and it runs beneath
everything else on the node. We replace it where open replacements exist,
constrain it where they do not by treating the radio as an untrusted peripheral
rather than a trusted component, and record which firmware version is on which
unit so that a later disclosure can be acted on rather than discovered too late.

Integration is the link the community owns outright, and it is where
accountability actually attaches. Somebody bought the parts, assembled the node,
flashed the image, and mounted it on the pole. That man is known, is present,
and can be asked. The record of who did what to which unit, kept honestly, is
the single most valuable supply chain artifact the community will ever have, and
it costs nothing but discipline.

## Reputation at risk

The measures above establish what can be checked. Almost everything important
cannot be, and for the remainder we rely on the oldest mechanism there is, being
a named party whose standing depends on having told the truth.

This is a different kind of reputation from the one Part II describes, and
conflating the two causes confusion. Peer reputation in AutonomousTrust is
computed by machines about machines from observed protocol behavior, and it is
continuous, automatic and anonymous by default. Infrastructure reputation is
held by people about people, concerns claims rather than conduct, and is neither
automatic nor anonymous. They answer different questions. One asks whether this
peer is behaving well right now, where the other asks whether the man who says
this node contains what he says it contains was telling the truth.

We keep the mechanism deliberately simple, because complicated accountability
schemes are the ones that get gamed.

Every claim has an author. The bill of materials for a node, the firmware
versions on it, the build hash of its image, and the assertion that the image
matches its published source are each signed by a person, not by an organization
and not by a process. Anonymous infrastructure claims are worth nothing here,
and that asymmetry against the previous chapter is intentional, since dissent
needs anonymity and vouching does not.

Claims are checkable after the fact, even where they are not checkable in
advance. Nobody verifies a build hash before trusting a node, and anybody can
verify it later, which is the property making the signature costly to falsify.
The threat that keeps an author honest is not detection today but detection
whenever somebody eventually looks.

Being wrong is survivable and lying is not. A man who signed a claim that turned
out to be mistaken corrects it and continues. A man who signed a claim he knew
to be false has done the one thing this arrangement exists to punish, and the
community that treats the two identically will shortly have nobody willing to
sign anything.

A community can operate this on paper and many should, and the machinery in the
upper tiers is exactly the machinery for it at scale. The conduct records in
Part IV are append-only, attributable and cryptographically verifiable, which is
what an infrastructure claim wants to be, and a community running the full stack
gets the ledger for free.

## What this does not achieve

We state this plainly, so nobody mistakes the posture for a guarantee.

A determined state adversary with access to the fabrication process defeats all
of this, and no community network will ever be able to say otherwise. A
sufficiently patient supply chain attack on a commodity radio will succeed. A
compromised community member who signs false claims will be believed for a
while.

What the arrangement achieves is proportion. Attacks that used to cost nothing,
such as shipping a modified image or swapping a part, now require sustained
deception by a named man who will eventually be caught. Attacks that remain
cheap are confined to layers where the damage is bounded by encryption and by
the small trusted computing base. And the community knows which is which, which
is the difference between an informed risk and a comfortable illusion.

## Further reading

- [What runs on a node](software_architecture.md): the small-process and
  unikernel commitments that make review feasible in the first place.
- [The security model](../lib/autonomous_trust/doc/security.md) and
  [security hardening](../lib/autonomous_trust/doc/architecture/security-hardening.md):
  the threat model and containment properties one tier up.
- [The dual implementation](../lib/autonomous_trust/doc/architecture/native-ffi-dual-implementation.md)
  and [correctness proofs](../lib/autonomous_trust/doc/correctness-proofs.md):
  formal verification of the runtime, which is the strongest form reviewability
  takes anywhere in this project.

---

*Next: [The optional stack](the-stack.md)*
