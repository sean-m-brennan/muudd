*Previous: [The five commons](commons.md)*

# Protected dissent

Every safeguard in this book can be captured by sufficient coordination. The
mediator pool, the audit commons, the admissions gate, and the interpretation of
the substrate itself are all things a determined faction can come to control.

The jester is the one role explicitly designed to name what the institutions are
becoming when the institutions themselves cannot.

The tradition is long and cross-cultural. It runs from the medieval fool who
could say to the king what no courtier could, through the prophets of the Hebrew
Bible whose entire office was telling Israel what Israel did not want to hear,
to the satyagrahi and the modern investigative journalist. In each case there is
a licensed truth-teller, protected by convention, whose protection exists
precisely because what they say is unwelcome.

We promote the jester from a cultural affordance to a substrate-level capture
detector, which means giving the role structural protection, institutional
insulation, and technical channels rather than hoping the convention holds. Three properties define the role.

*Institutionally protected.* Interference with an observer is itself a virtue
violation, which puts it inside the escalation machinery rather than outside it.

*Structurally insulated.* Rotation, multiple competing observers, and a
deliberate refusal to let the corps become a single ideological monoculture. An
observer corps that agrees with itself is a faction with a nice name.

*Technically equipped.* Ring signatures for whistleblowers, commitment schemes
that pre-stage evidence, and secure communication routed outside the
infrastructure of the accused community.

## Attributed reports, and the right of response

Observers travel between member communities, observe conditions, and report.

The reports are *attributed* rather than anonymous, and that is deliberate. An
observer is not pretending to neutrality. Naming who saw what lets a reader
weigh the report against everything else known about that observer, and an
unsigned claim gives a reader nothing to weigh it with.

Every report carries an *irrevocable right of response* for the community it
concerns. Irrevocable is the operative word. A right of response that the
compact could withdraw is a right of response held at the discretion of whoever
holds the machinery.

## Accounting for having seen nothing

The corps owes an account of each window it holds seats for, *including an
account of having seen nothing*.

That requirement looks like paperwork and closes a real hole. Without it, a
corps that stopped looking is indistinguishable from a corps with nothing to
see. Both produce no reports. The attested null makes silence into either
evidence or a fact about the compact, where before it was missing data.

This is one instance of a rule running through the whole design, stated plainly
in the erosion analysis. A measure that can only fire when somebody acts cannot
see a compact that has stopped acting, and only absolute measures see sincere
drift.

## The whistleblower channel

The hardest case is a member of a community reporting on that community. The
source has to prove he is an insider, since otherwise any stranger can allege
anything, and has to stay unidentified, since otherwise nobody will speak.

A ring signature does both. The source proves membership in a defined key set
without revealing which member signed.

Four properties of the construction matter.

*The ring spans a community's root key, its live delegations, and its published
member roll.* The roll comes from the polity tier below, and it had to, because
this tier models communities and speakers and holds no member roster. Before the
roll arrived, an insider holding no delegation was not in the ring at all, and a
community that had delegated nobody had no anonymous channel whatever.

*The ring is computed over the log preceding each filing* rather than over the
current archive tip. That distinction is the whole security of the mechanism.
Computing it at verification time against the tip let the accused community, the
one party with a motive, append one backdated delegation and invalidate every
filing against it, or backdate a revocation and narrow the suspect set around a
source after the fact. We measured both, and both worked. Authority must be a
property of the log rather than of the moment you looked.

*Rolls are unioned and never superseded.* Once named an insider, always an
insider, because the alternative hands a community a way to shrink the suspect
set around a member the day after they leave.

*The identity of the source is held k-of-n by the reception panel*, each share
sealed to the registered *encryption* key of a mediator, which is distinct from
the signing key and checked against it. That check closes a hole the item was
originally misnamed for. Shares could be sealed to the key of an outsider, with
no custodian able to open them, and the filing was still recorded as authorized.
The recipient key is now resolved rather than supplied.

## The reception panel

The panel that receives whistleblower reports operates under three rules.

The investigation threshold is *low*, because a channel that demands proof
before it will look is a channel that discovers nothing.

The source is *never revealed*, including the source of a report that turns out
to be false. That is the harder half of the commitment and the one that decides
whether anybody uses the channel twice.

And there is a *no-panel exculpation* for anyone wrongly named, meaning a person
accused in a report can clear themselves without a panel having to convene. An
accusation that can only be answered by a proceeding is an accusation that
punishes by process.

## Four kinds of dissent

The corpus distinguishes them because they need different protection.

*Procedural* dissent says a decision was reached wrongly. *Substantive* dissent
says a decision was wrong. *Prophetic* dissent says the institution has become
something other than what it was for. And *heterodox* dissent says the shared
understanding itself is mistaken.

The first two are ordinary and the machinery of the previous chapters handles
them. The third is what the jester is for. The fourth is the most dangerous to
protect and the most dangerous not to, because a compact that cannot hear it
cannot correct a foundational error, and a compact that hears nothing else has
no foundation left.

## The problem from the other direction

The corpus also names the failure mode of protecting dissent too well, and it is
the inquisitorial caste arriving by a different door.

An observer corps with standing, protection, and a mandate to find problems will
find problems, because that is what it is for and what its members are rewarded
for. The safeguards are the ones we apply everywhere else, being rotation, term
limits, pool-share caps enforced by the same roster machinery the mediators and
the examiners use, and the refusal to let observation become a career.

None of that is sufficient, and the chapter on limits says so.

## Why this role exists at all

The jester exists because every other safeguard here can be captured by
sufficient coordination, and because the jester sees precisely by being rooted
somewhere outside what is being captured. It is the one role in the design that
does not depend on the institutions being sound, and that is exactly why it is
the one role that can say when they are not.

## Further reading

- [Jester and dissent](background/jester_and_dissent.md): the licensed
  truth-teller tradition, cross-community journalism, the four types of dissent,
  and the inquisitorial-caste problem from the complementary direction.
- [Whistleblower cryptography](architecture/whistleblower-crypto.md): the ring
  construction, threshold custody, and the encryption-key binding.
- [Erosion legibility](architecture/erosion-legibility.md): the attested null,
  the empty office, and why only absolute measures see sincere drift.
- [Capture resistance](background/capture_resistance.md): the jester promoted to
  substrate-level detector, and the stated limits.

---

*Next: [Machines as delegates](machines.md)*
