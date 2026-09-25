*Previous: [Build and toolchain](../architecture/build-and-toolchain.md)*

# Kith-Covenant

*A Protocol for Cooperation Among Communities That Do Not Share Ends*

---

## Executive Summary

The oldest governance problem on record is how to bind communities
that do not share beliefs into durable cooperation. We know the
problem is ancient because the earliest surviving parity treaty
between two foreign great powers, the Hittite-Egyptian accord of
1259 BCE, already has its shape. How do you make an agreement hold
between sides with different gods, different languages, and
different loyalties? Every civilization since has taken a run at
it. Commerce across religious boundaries (medieval merchant law),
maritime rescue (the Rhodian tradition, transmitted across four
civilizations for a millennium), wounded-soldier treatment (Red
Cross / Red Crescent), and ambassador protection have all worked
cross-culturally under thin procedural forms. *Full-scale*
arrangements, ones governing broad slices of community life rather
than a single function, have a more brutal record. They fail in
two distinct ways, and both modes are still live. First, without a
shared foundation, oaths do not bind and short-term interest
overrides them. Second, with a shared foundation that the
arrangement itself comes to worship, the system collapses into
purity cascades (*"you are not pure enough"*) ending in purges and
mass violence. Robespierre's Terror, the Cultural Revolution, and
the Stalinist show trials are not accidents. They are the
predictable failure mode of virtue-oriented communities that lack
specific structural brakes. The Salem witch trials are the harder
and more instructive case. A community that held the brakes in
doctrine and had them defeated in practice, whose spiral the
substrate then terminated from within, and which then repented and
paid restitution (§3.2).

We present **kith-covenant**, a protocol for cooperation among communities that
must work together without pretending to share ends. Our core claim is that
*a protocol designed for angels fails at first contact. A protocol designed
for rational actors fails slightly later. A protocol designed for sinners has
a chance.* Two structural rules carry most of the load. First, each
participating community must anchor itself to something it treats as above
itself, and the compact itself must never claim to be that thing. Second, a
community qualifies for full membership only if its own tradition supplies
five *anti-spiral* features that historically block the purity-cascade failure
mode: a universal fallen-actor anthropology (every member, including the
virtuous, can fail); grace over merit (virtue is received, not earned);
scapegoat-revelation or a functional equivalent (victim-selection as
communal hygiene is refused); enemy-love as operational norm (moral seriousness
is owed to the person actually before you, not to an abstract class); and
non-ultimacy of the political community (the *polis* is not the final thing).
These features are present, in depth or in functional equivalent, across the
mature religious traditions. They are absent in the ideological systems
(Jacobin, Maoist, pure-tribalist, closed-materialist-utilitarian) that
produced the historical purges.

Operationally, we run the protocol in **two distinct layers**. At the
**intra-community layer**, each community performs its own
five-movement authorization rite on the envoys and representative
machines it sends out. Asymmetric psychological cost operates here,
in the substrate and language of the community itself, which is where
the historical record shows that mechanism carrying load. At the
**inter-community layer**, what crosses the boundary is a thin
procedural grammar descended from the Kadesh precedent and from the
workable historical forms (lex mercatoria, the Geneva framework,
polycentric diplomatic immunity). Treaty-form entry with periodic
reaffirmation, credentialing of internally-authorized envoys,
vouching and sponsorship, a compact-wide audit commons, and explicit
anti-spiral-feature credentialing. No community is asked to accept
the theology of another. Every community is asked to demonstrate the
structural commitments the historical record shows are load-bearing.
Because the same machinery that makes the compact work makes it
capturable, capture resistance runs through the design as a
continuous concern (§6.8). Seven strengthening moves, from
interpretive plurality through admissions integrity, with the jester
promoted to substrate-level capture detector.

Kith-covenant sits atop **autonomous-trust** (AT), which supplies
anonymous intra-community reputation. AT is load-bearing inside one
trust boundary. Kith-covenant supplies known identity, coupled
across communities to specific named humans whose real community
standing is at stake. Machines participate only as *delegates* of
specific named human guardians (not as independent parties) through
a derived form of the compact whose substrate is inherited from the
one human each machine is irrevocably tied to. The binding is
carried by four interlocking mechanisms (constitutive objective,
cryptographic self-binding, continuous attestation,
principal-referential identity plus audit commons) and secured
across three facets (engineering, guardianship, and succession).
Machine-to-machine contact reduces to the two guardians meeting
through delegates. Every machine act is bounded by an envelope its
principals pre-authorized and answered for by the guardians, and
nothing originates at the machine layer outside that authorization.
The protocol makes machine-original substrate *structurally
impossible* rather than merely forbidden.

This whitepaper sets out the full argument, the social and machine mechanisms,
the software architecture, and the open questions with their current
dispositions. Claims are
stated with the strength the historical record supports, and no more. The detailed historical drivers and the
full corpus of cited cases are reserved for the appendices.

---

## 1. Introduction

### 1.1 What we are solving

We are building the protocol layer for cooperation among communities that do
not share ends. Our target deployment is Earth-first. Space is the sharpening
domain, not the main test. Earth is the harder test in the dimension that
matters most: with more communities, older wounds, nation-state capture of
infrastructure, much greater ideological heterogeneity, and, decisively,
cheap exit. On Earth the state legal system is one phone call away and every
service the compact offers is substitutable, so an Earth compact that holds
is proof that the substrate and ceremony layer binds voluntarily. Space
could never prove that, because in space "the oath binds" and "there is
nowhere else to go" are indistinguishable. The complement: Earth
is structurally incapable of testing the enforcement model, because a
determined defector there routes around the compact and loses nothing it
cannot replace. Space sharpens exactly that dimension (exit is
catastrophically expensive, life-support interchange is load-bearing,
light-lag defeats central adjudication, physics makes concealment
difficult). The two test domains are complements, not stages of one test.
The full argument is in
[`background/bounded_conflict.md`](../background/bounded_conflict.md#the-two-dimensions-of-the-test).
The fundamental problem of cross-community cooperation is one that every
civilization has had to answer. We answer it with a protocol whose
primitives are drawn from what has historically worked, and whose failure
modes are drawn from what has historically failed.

### 1.2 Approach in brief

The question is usually asked as *how have strangers historically
come to cooperate?* and the literatures that answer it (iterated-game
theory, repeated-interaction reputation, commons governance) all run
into the same limit at the inter-community boundary. We ask the
inverse: *how have strangers historically come to fight each other
within limits?* Every structural feature that made conflict bounded
has an inverse that is a primitive for cooperation. Mutual
recognition of the adversary as a legitimate peer inverts into
recognition of the counterparty as a party one can make commitments
with. Symmetric rules that constrain both belligerents invert into
procedures that apply equally regardless of the size or ideology of
the parties. Spatial and temporal brackets that keep a war-theatre
from consuming the whole civilization invert into
compartmentalization. Protected envoys invert into never-target
channels. And the shared oath-substrate (the thing that made the
whole apparatus bind in the first place) inverts into the substrate
question kith-covenant must answer. The inversion is not rhetorical.
It is the structural reason we can say, with historical support, what
the protocol primitives need to be.

### 1.3 How to read this document

This whitepaper is the consolidated argument for kith-covenant. Sections 2 to 5
develop the theoretical and anthropological frame. Section 6 specifies the
social mechanism. Sections 7 and 8 specify the machine layer and the
machine-to-machine (M2M) layer. Section 9 enumerates the software
architecture with diagrams. Section 10 lists open questions.
Section 11 is the implementation posture.

The historical drivers (the full comparative case analysis that grounds the
substrate claim and the operational primitives) are in **Appendices A and
B**. The glossary is in **Appendix C**. The seven-holes research agenda is in
**Appendix D**. The bibliography and primary-source register are at the end.

The research corpus this whitepaper consolidates is indexed in the sibling
[`kith_covenant.md`](../kith_covenant.md); the per-thread development is in
[`background/`](../background/) and [`architecture/`](../architecture/). This whitepaper
is the single-entry-point synthesis for the reader who wants the complete
picture in one narrative. The corpus remains the authoritative development.

---

## 2. Thesis and frame

### 2.1 The four threads

Our argument runs along four threads that reinforce each other.

The **inversion argument** (§1.2 above, §6.1 operationally) observes that
every structural feature of bounded conflict inverts directly into a
cooperation primitive that does not require shared ends. This is the
structural reason we can say what the kith-covenant primitives need to be
without having to *invent* them. They have historical precedent under a
different problem framing.

The **substrate finding** (§3) is that every historically durable
cross-community *full compact* has rested on the anchor of each
participating community to a transcendent moral authority whose
judgment was acknowledged as exceeding that of the parties, with the
compact itself structurally refused the role of ultimate. And that
the load-bearing content of that anchor is supplied by substrates
carrying five specific *anti-spiral features* (§3.2). Bounded
functional cooperation (commerce, maritime salvage, envoy protection,
wounded-combatant treatment) has a separate and lighter profile and
converges cross-culturally without shared substrate under
preconditions we document in Appendix B.

The **fallen-actor anthropology** (§4) is that the protocol must be built for
sinners (agents who will fail and need paths back, whose self-interest must be
entangled rather than merely aligned), not for angels and not for rational
utility-maximizers. Protocols built on either of the other two anthropologies
collapse in specific, predictable ways.

The **forcing function** (§5) is mutual vulnerability. Communities
cooperate more reliably when each holds soft tissue in the places
where the commitments of the other would, if broken, reach. Every
Geneva prohibition holds because every belligerent is a potential
victim of every prohibited act.

### 2.2 Where kith-covenant sits

Kith-covenant is one layer in a larger stack. The layer below it,
[autonomous-trust (AT)](../../../autonomous_trust/README.md), supplies
anonymous-identity primitives, peer-voted admission, and reputation consensus
among peers inside one trust boundary. That is what the inside of a working
community needs. It is not what is needed across unconnected communities. At
that scope the problem is no longer *who among our peers merits which gradient
of trust* but *how can our community enter into binding commitments with a
community that does not share our norms, our enforcement, or our cosmology,
and have those commitments cost something when broken.* Anonymity is wrong at
that layer. Known identity, tied to a specific named human whose social
standing is at stake, is what makes the commitment bind. Kith-covenant is the
layer that does this.

```
    ┌────────────────┐  ┌────────────────┐        ┌────────────────┐
    │  community A   │  │  community B   │  ...   │  community N   │
    │  (AT: internal │  │  (AT: internal │        │  (AT: internal │
    │   anonymous    │  │   anonymous    │        │   anonymous    │
    │   reputation)  │  │   reputation)  │        │   reputation)  │
    └───────┬────────┘  └───────┬────────┘        └───────┬────────┘
            │                   │                         │
            └───────────────────┼─────────────────────────┘
                                │
                     ┌──────────┴──────────┐
                     │    kith-covenant    │   ←  this whitepaper
                     │  (known identity,   │
                     │   cross-community,  │
                     │   guardian-coupled) │
                     └──────────┬──────────┘
                                │
                     ┌──────────┴──────────┐
                     │        muudd        │
                     │   (communications)  │
                     └─────────────────────┘
```

**Figure 1.** Stack layering. Each community runs its own AT instance for
intra-community anonymous reputation. Kith-covenant sits above AT as the
inter-community protocol layer, tying specific named humans across the
compact. Muudd handles the communication substrate below.

Nothing in kith-covenant replaces AT. Everything builds on it. The interface
between the two (what crosses the boundary, on whose authority, with what
protections) is specified in
[`background/reputation_bridges.md`](../background/reputation_bridges.md) and
summarized in §9.8 below.

### 2.3 Scope: full compact vs. thin-functional domain

A central scoping distinction runs through this whitepaper. We use
**full compact** for an arrangement governing a broad envelope of
inter-community life: dispute resolution, membership, restoration,
commons, succession, and representation. Westphalia, the Hanseatic
League, the Haudenosaunee Confederacy, the Buddhist *sangha* as
cross-polity network, and the Mali federation are full compacts. We
use **thin-functional domain** for an arrangement governing one
bounded function (maritime rescue, merchant credit, envoy
protection, wounded-combatant treatment, ransom exchange) across
communities that do not otherwise share a governance frame. Rhodian
Sea Law, *lex mercatoria*, the Geneva framework, Mediterranean
frontier *aman*/*hudna* norms, the Kadesh treaty grammar, and
polycentric diplomatic immunity are thin-functional domains.

The distinction matters because the substrate requirement we develop in §3
scopes to full compacts only. Thin-functional-domain cooperation has converged
repeatedly across communities that did not share substrate. The historical
record on that is rich, centuries-long, and not to be discounted. See
Appendix B.

### 2.4 About sources

Much of the developed anthropology and cooperative architecture we draw on is
Christian in origin. The fallen-actor anthropology is, in its most developed
form, Christian. So are the deepest articulations of *justus hostis*, the
sacramental architecture of confession and return, and the *imago Dei*
grounding of peer recognition, even where analogues exist in the Haudenosaunee
Great Law, the Buddhist *sangha*, the Mande *Maa Ngala* tradition, and others
treated in Appendix A. The compact we propose requires the
*features* (§3.2) that this lineage supplies. It does not require the theology
that most fully supplies them.

---

## 3. The substrate finding

### 3.1 Anchor above self. Compact never ultimate

When we apply a voluntary-peer filter (excluding imperial frameworks like
*dhimma*, Pax Romana, and the Pax Mongolica, which are hierarchical and
underwritten by sovereign power) and rank the remaining *full-compact* cases
by longevity, the pattern is unmistakable. Every durable voluntary cross-community
full compact on record rested on a transcendent moral authority. The
Westphalian system, the Hanseatic League, *lex mercatoria* in its full form,
the codes duello, and the medieval Peace of God and Truce of God all operated
within Christendom. The Haudenosaunee Confederacy rested on the Peacemaker's
mandate and *Shöñgwa'ya'di:söñ*. The Buddhist *sangha* (the single
longest-running voluntary cross-community network in our dataset) rests on
the Dharma. The Mali federation rested on *Maa Ngala*. The content of the
ultimate varies. The *structure* does not.

From this, the load-bearing form of the substrate claim, carefully scoped:

> **Each community must anchor itself to something above itself, and
> the compact must not claim to be that thing**.

This is the substrate finding in its defensible form. It does not
require participants to share a substrate. It requires each
participant to *have* one, and it structurally forbids the compact
from becoming the ultimate of any participant. An oath sworn before
an authority the swearer acknowledges as exceeding the compact costs
something real in a dimension that compact- internal enforcement
cannot muster, *whether or not the other party holds the same
authority*. An oath sworn before no such authority costs only what
enforcement can muster, and enforcement is not reliably available
across unconnected communities.

The Kadesh precedent (1259 BCE, Egyptian-Hittite parity treaty) is
directly relevant as the earliest surviving shape of this structure:
Hittite and Egyptian parties did not submit to a shared substrate and
did not pretend to. Each invoked its own gods as witnesses of a
shared *form*, and the binding force of the treaty rested on the
internal substrate of each side doing its internal work on that side,
plus a diplomatic grammar neither side had to abandon its ground to
accept. The kith-covenant recognition mechanism (§6.3) is descended
from that precedent, not from the Nicene one.

### 3.2 The five anti-spiral features

The scoping in §3.1 is narrower than the strong historical claim but still
under-specified in one respect that matters operationally. The historical
record includes communities that *had* a transcendent anchor and *still*
collapsed into mass-violence cooperation failure. The Jacobin Republic with
its Cult of the Supreme Being, Maoist China's Revolutionary Virtue, the
Stalinist purge logic, various 20th-century ideological regimes with
sacralized substrates. In every such case the same
mechanism fails. Virtue without a backstop collapses into *purity-spiraling*,
and purity-spiraling terminates in the scapegoat cascade. Alasdair MacIntyre
(*After Virtue*, 1981) names the upstream condition. Once virtues lose their
teleological grounding, they become free-floating terms subject to emotivist
weaponization and polarity-inversion. René Girard (*Things Hidden Since the
Foundation of the World*, 1978; *I See Satan Fall Like Lightning*, 1999)
names the downstream dynamic. Unbounded competitive virtue-signaling generates
the sacrificial tension that only victim-selection can discharge. Hannah
Arendt (*The Origins of Totalitarianism*, 1951) documents the political
topology under which these dynamics become state-scale.

The load-bearing form of the substrate requirement, therefore, is not just
"something above self" but specifically a substrate that supplies **five
anti-spiral features**, structural commitments that together block the
purity-spiral mechanism:

1. **Universal fallen-actor anthropology.** Every member of the community,
   including the most virtuous, is fallen. This forecloses the *we who are
   pure* move that purity spirals require. Any substrate that locates sin or
   impurity only in a designated out-group fails this requirement by
   construction.
2. **Grace over merit.** Virtue is received, not earned. Competitive
   virtue-signaling becomes theologically (or functionally) incoherent under
   this commitment. Substrates that treat virtue as a competitive status
   marker fail here.
3. **Scapegoat-revelation, or a functional equivalent that disarms the
   sacrificial mechanism**. The community must hold, at its foundations, that
   the selection of a sacrificial victim to discharge communal tension is
   recognized *as* scapegoating and refused. Christianity's form (the Cross as
   terminal scapegoat, per Girard) is the most developed; Buddhist
   anatta-plus-compassion, Haudenosaunee condolence practice for dead enemies,
   Jewish *teshuva* plus the hidden-*tzadikim* motif, and mature Sufi
   enemy-love traditions supply partial functional equivalents. Substrates
   that lack any form of this feature (that treat victim-selection as simply
   what a healthy community does to its traitors) fail decisively.
4. **Enemy-love as operational norm.** This is not sentiment but a structural
   constraint, the Samaritan moment generalized. Moral seriousness is owed to
   the person actually before you regardless of which side designated whom as
   enemy, which blocks the in-group/out-group compression that purity spirals
   weaponize.
5. **Non-ultimacy of the political community.** The *polis* (and the compact)
   cannot be the ultimate, because it is not what the community
   commitments are for. Already encoded in the fifth movement of our
   recognition ceremony (§6.3).

Several consequences follow. The five features rule out substrates that
"anchor above self" but lack anti-spiral structure. Pure-tribalist cosmologies
that locate sin only in the out-group fail feature 1. Revolutionary-virtue
ideologies that treat political perfectibility as the ultimate fail feature 5.
Pure-will-to-power substrates fail features 2 and 4. Utilitarian calculations
without fallen-actor anthropology fail feature 1. The features are supplied
in their most developed combination by historical Christianity (a
historical observation, not a theological claim), and other traditions
supply partial combinations of varying depth (Judaism's *teshuva* plus
fallen-actor anthropology; Buddhism's compassion plus anatta-dissolution of
competitive virtue; Haudenosaunee condolence; mature Sufi Islam; and various
indigenous traditions with strong guest/stranger obligations). **Kith-covenant
requires the features. It does not require the theology that most fully
supplies them**.

One scoping note bounds the claim. The features metabolize purity
spirals. They do not inoculate against them. Puritan Salem held all
five features doctrinally and still spiraled in the witch trials of
1692: visible-saints ecclesiology, assurance-anxiety, and
national-covenant theology defeated features 1, 2, and 5 in practice,
and court procedure (spectral evidence, confession-spares-you)
amplified the cascade. But the substrate then terminated the spiral
from within (Increase Mather's *Cases of Conscience*, arguing on
theological grounds against the procedure of the court itself) and
produced public repentance and paid restitution, a sequence no
featureless regime has produced. The Terror ended by coup, the purges
by the death of the purger. We therefore read the load-bearing
function of the features as termination and repair, not prevention,
and our credentialing (§6.3, element (e)) examines operative
practice, not confessed doctrine. The full treatment is in
[`background/justus_hostis.md` "The Salem witch
trials"](../background/justus_hostis.md#the-salem-witch-trials-the-features-can-be-held-and-still-defeated).

### 3.3 Type-invariant but content-substantive

What the compact requires at the substrate layer is a *structural*
commitment (that a community oaths refer to an authority it treats as
exceeding the compact, as exceeding the interest of any party, as the
source of the meaning their commitments bear) and the five
anti-spiral features. What the compact does not require is any
particular content. A Christian community names the Holy Trinity. A
Muslim community names Allah. A Haudenosaunee community names
*Shöñgwa'ya'di:söñ*. A Buddhist community names the Dharma. A
philosophical-deist community names the Creator. The substrate is
**type-invariant but content-substantive**. Coherence across the
compact does not rest on shared content. It rests on each community
having its own ground outside itself *and* the compact structurally
refusing to be that ground.

This answers what we have elsewhere called *Hole #1*, the
substrate-coherence-under-pluralism problem. The substrate does not
have to cohere across communities because no joint ceremony exists
for it to cohere in. Each community ceremony is internal to it and
filters its own members using its own substrate. What *does* have to
cohere is the thin inter-community grammar, and that grammar is
content-thin by design. It carries the structural commitment without
requiring any community to accept the content of another substrate.

### 3.4 The control-system hypothesis

Our substrate finding has a sociological-mechanics companion: Jacques
Vallée's control-system hypothesis, which we treat at full length in
[`background/control_system_hypothesis.md`](../background/control_system_hypothesis.md).
Vallée's three components are a cross-cultural pattern of encounter
with transcendent ambiguity. A regulatory function those encounters
perform on the societies that receive them. And a standing
manipulation vulnerability, because whatever regulates belief can be
imitated by parties who want to steer it. We adopt a strict
methodological separation between the physical layer (about which we
claim nothing) and the informational layer (where the sociological
mechanics operate regardless of what stands behind them), and we take
six insights from the analysis into the design:

1. **Ambiguity as feature.** The substrate resistance to full
   specification is part of how it regulates. A fully legible
   substrate would be a fully capturable one.
2. **Layered evaluation.** Claims about substrate-adjacent events are
   evaluated at distinct layers with distinct evidence standards.
3. **Regulatory-feedback framing.** The compact ceremonies and
   cadences are read as a feedback system, not a static ruleset.
4. **Capture resistance as a substrate-design constraint** (§6.8),
   not an implementation afterthought. This is Vallée Component 3
   taken seriously. Manipulation vulnerability is continuous
   structural pressure, and the design must carry countermeasures in
   its ordinary operation.
5. **Form/content distinction.** The recurring form of encounter is
   cross-cultural. The content is always rendered in local idiom.
   This is the same structure as our type-invariant but
   content-substantive substrate requirement (§3.3).
6. **Methodological humility.** The hypothesis is held as mechanics,
   not metaphysics. It constrains design without settling what the
   substrate ultimately is.

Where Vallée is silent (intra-community recognition, the fallen-actor
anthropology, the anti-spiral features), our historical analysis
carries the argument alone.

---

## 4. Fallen-actor anthropology

Cooperation-protocol engineering has historically imported its anthropology
from economics, and the imported picture (actors are self-interested,
consistent in preferences, roughly rational) is wrong in a specific way that
matters for design. Humans are not rational utility-maximizers. They are not
angels. They are, to use the theological word that the historical exemplars
used and that we choose not to hide from, **sinners**. This is not a
pessimistic flourish. It is a load-bearing claim about what the protocol must
withstand. The intellectual lineage runs from Augustine (*City of
God*) through Reinhold Niebuhr (*Moral Man and Immoral Society*, 1932;
*The Nature and Destiny of Man*, 1941 to 1943) and Aleksandr Solzhenitsyn (*The
Gulag Archipelago*, 1973) on the realist side, through Girard on the mimetic
side, and through James Madison (*Federalist* №51) on the design side. The
best single sentence we can offer catches all three:

> *A protocol designed for angels fails at first contact. A protocol
> designed for rational actors fails slightly later. A protocol designed
> for sinners has a chance.*

Design implications follow directly:

**The line between good and evil cuts through every participant**. The
protocol plans for atrocity as a normal contingency rather than a shocking
aberration. Restoration paths are built in from the start. The
expectation is not that violations will be rare but that they will happen on
a predictable cadence, and the protocol must be able to absorb them without
dissolution.

**Rivalry is structural.** Girard's mimetic insight is that desire
is borrowed. That rivalry is therefore generated rather than
reduced by proximity. And that communities discharge the resulting
tension through sacrifice unless they hold structural commitments
that recognize and refuse the move. While acknowledging the energy
scapegoating releases, the compact must not rely on it. The
anti-spiral features (§3.2) are our load-bearing answer.

**Ambition must counteract ambition, Madison-style.** The
self-interest of a participant should not be realizable except
through protocol behavior that also serves another. This is the
engineering expression of the mutual-vulnerability forcing function
(§5) and the reputation-bridge design (§9.8).

**Virtue is a disposition, not compliance.** Kith-covenant cannot form
character. It can only protect the communities in which character is formed.
The common-virtue frame (§6.6) is the protocol recognition of this limit.

---

## 5. Mutual vulnerability as forcing function

Even among adversaries who meet all of the above, cooperation requires a
forcing function. A reason the parties cannot simply walk away. Mutual
vulnerability is that reason. Every Geneva prohibition holds because every
belligerent is a potential victim of every prohibited act. None of them want
to poison the well they drink from. Cooperation protocols whose prohibitions
are grounded in universal vulnerability are far more stable than those
grounded in moral consensus, because moral consensus is revisable in ways
physics is not.

In the terrestrial case, mutual vulnerability is variable. Some
communities hold more leverage than others, and the compact must make
the asymmetry legible rather than pretend it away. In the space case,
mutual vulnerability is physics. A habitat cannot walk away from the
orbit of another cheaply, the shadow of the future is extraordinarily
long, and life-support interchange, rescue infrastructure, and
orbital-debris management make defection materially costly in
dimensions no moral argument needs to carry. See Appendix E on space
as the sharpening domain.

### 5.1 The frontier asymmetry (the colonial failure)

Mutual vulnerability is uneven. Communities *inside* the compact
are mutually vulnerable in ways the compact can assume. Communities
*outside* the compact stand in an asymmetric relation that the
historical *jus publicum Europaeum* notoriously exploited. While
European powers respected *justus hostis* among themselves, they
denied it to their colonies, and the failure was that of the
substrate itself. The scriptural mandate of the substrate (the
neighbor, the Samaritan, the actual face before you) was betrayed
by its nominal carriers. **Kith-covenant must not recapitulate this
failure.** Our four-category framework (§6.4) is the structural
correction: in addition to members, *justus hostis*, and *hostis
humani generis*, we name a fourth category,
**non-member-encountered**, that the colonial failure most acutely
neglected.

We state the scope. What the compact does at the frontier is partially
answered, not fully. The four-category framework prevents the structural
collapse. It does not solve the frontier problem. Further work is
documented in §10 and in the research corpus.

### 5.2 The ground of the consent-free edges

Two of the four categories name parties that have consented to nothing. A
community encountered in trade, transit, or distress has signed no compact,
and the actor designated *hostis humani generis* is condemned by a standard it
has never accepted. In both directions the compact acts where consent has run
out, and it states the ground on which it acts rather than leaving the
behavior unexplained.

That ground is a thin functional minimum, adopted rather than
derived. Given survival as an aim of the parties present, and truisms
about human vulnerability, approximate equality, limited altruism,
limited resources, and limited understanding, any viable order among
such parties contains a short list of forbearances. This is Hart's
minimum content of natural law, and it is the mutual-vulnerability
argument of this section in the vocabulary of jurisprudence. The
compact takes its content as the standard for its own conduct at the
edges. It is not the compact authority, which reaches its members and
reaches them by their consent, and it is not the substrate of any
member community, which the compact receives and does not speak for.

The adoption is deliberately small, and three limits define it.
First, it derives none of the common virtues, which remain
received from member substrates. Second, it is hypothetical rather
than categorical, asserting what an order must contain to persist
among parties of this kind and asserting nothing about what anyone
is owed in virtue of what they are, so a member community may read
the aim as a created good, a convention, or an illusion without
changing what the compact does at its boundary. Third, it reaches
the boundary and not the interior, and Hart's own text concedes
that these forbearances need extend only to those an order counts
as within it, so the minimum settles what is owed and never who is
counted. Whom the compact itself counts at its frontier is its own
commitment, carried by the substrates of its members and recorded
in the four-category framework. What removing the ground would
license is not a denial that the party is vulnerable but the
treatment of the forbearance owed as a courtesy the stronger party
may withdraw, which is the form the colonial failure actually
took.

That the ground stops at the boundary coincides with a commitment the
compact makes on its own account, and the coincidence is not
accidental. **The compact forms no verdict on the internal morality
of a member community until that morality touches the covenant**:
conduct toward another member, conduct at the boundary, conduct
toward a party that arrived through a compact channel, what a member
envoys carry into compact business, or a hollowing its own
re-affirmation cannot survive. Until then the question is not
deferred pending better instruments. It is not the compact question.
A compact that appoints itself judge of the interiors of its members
has claimed a superiority over them, and the claim is poisonous to
the cooperation the compact exists to carry. It makes the compact the
ultimate that Movement 5 forecloses, installs the reading of the
compact itself as the standard the compact-incapable list forbids it
to install, and hands whoever holds the deliberative machinery a
legitimate instrument for finding a member deficient. What the
compact watches instead is effects, which are legible in a way
interiors are not. The cost is real and we do not hide it. The
compact can carry a member whose interior life it would not defend,
and it does so knowingly, holding that a compact confident enough to
audit the souls of its members fails sooner and worse. With respect
to the internal governance the compact structurally refuses to
decide, that limit is a feature. With respect to a victim class
inside a member, it is a hazard. Both halves are stated because they
are the same fact. Full treatment in the research corpus,
`background/natural_law.md` §"The minimum content, adopted".

---

## 6. Social mechanism

### 6.1 The six components

This section enumerates the social components of the compact before
developing each. We name six, in the order they typically come into play for
a pair of interacting communities:

1. **Escalation ladders** (§6.2): graduated rungs from démarche through
   rupture, replacing the binary peace/war distinction. Each rung costs more
   than the last and offers a dignified exit.
2. **Two-layer recognition** (§6.3): an intra-community authorization rite on
   envoys, plus inter-community thin grammar and credentialing across the
   boundary.
3. **The four categories** (§6.4): members, *justus hostis*, anathema
   (including the rare *hostis humani generis* subclass), and
   non-members-encountered.
4. **Liminal institutions** (§6.5): third places, the mediator pool, the
   observer corps, and emergency roles, all of them capture-resistant
   institutions that live between member communities.
5. **Commons governance** (§6.6): Ostrom's design principles applied, plus
   the *tragedy of the virtues* (spirit-gaming, not just overuse).
6. **Common virtue and the jester** (§6.7): virtue as disposition rather
   than rule, plus the protected dissent that keeps the compact from
   becoming its own ultimate.

A seventh concern, **capture resistance** (§6.8), is not a component
beside these six but a continuous structural property that runs through
all of them. We give it its own treatment because its absence from an
enumeration like this one is exactly how it gets lost.

### 6.2 Escalation ladders

Bounded-conflict regimes replace the binary peace/war distinction with a
ladder of graduated rungs, each more costly than the last and each offering
a face-saving exit. Classical diplomatic ladders run from démarche through
formal protest, ambassadorial recall, rupture of relations, sanctions,
blockade, limited military action, general war. The codes duello ran from
perceived offense through verbal challenge, cartel, choice of ground, field
encounter, fire, and formal resumption. Geneva/Hague imposes a nested
sub-ladder even inside active war.

The common architecture has four features. Each rung costs more than the
last. Each rung offers a dignified retreat. Rungs are visible to third
parties so reputation does enforcement work. And third-party roles
(seconds, protecting powers, mediators) are procedurally involved, because
two-party conflict escalates faster than three-party conflict.

Kith-covenant inherits this architecture with two specific overlays. The
**common-virtue overlay** (§6.7) frames the rungs as virtue-violations with
restitution paths rather than as prohibited acts with sanctions. The
**restitution-and-return commitment** that the fallen-actor anthropology demands
requires every rung of the ladder to carry an attached restoration procedure, by
which a violating community, having completed restitution, is restored to its
prior standing. The ladder we build is reversible along every rung
except the rarely-invoked *hostis humani generis* designation (§6.4).

### 6.3 Recognition. Two layers

A single joint ceremony (candidate member communities publicly reciting
a five-movement statement across the boundary, asymmetric psychological
cost filtering entryists) would import the mechanism of the Christian
creed and transplant it
from its native setting (an *intra*-community rite by which the church
filters its own) to a setting it was not historically made for
(cross-cultural entry-filtering). The historical record that actually
governs cross-cultural cooperation (Kadesh, the Rhodian transmission,
Mediterranean frontier norms, polycentric diplomatic immunity, the Red
Crescent) does not show joint thick ceremonies. It shows each party
internally authorizing its envoys under its own substrate, plus a thin
shared grammar across the boundary. We therefore run recognition in
**two distinct layers**.

**Intra-community layer. Internal authorization.** Each candidate member
community performs the five-movement rite *on its own envoys and
representative machines before they are authorized to bind the community
inside the compact*. The five movements are:

> 1. **The naming of the substrate** in the terms of the community
itself. > 2. **The acknowledgment of fallenness.** The recitant, the
community, > the envoy being authorized. > 3. **The recognition of
the other** as peers bearing *imago Dei* or its > tradition-specific
functional equivalent. > 4. **The commitment to the common virtues**
as dispositions. > 5. **The submission of the compact itself.** The
compact is not the > ultimate, and the community is not bound to it
if it ceases to serve > what it was formed to serve.

Asymmetric psychological cost is the real filter at this layer. The
rite does work on the members of the community itself, in the
language its own substrate supplies, which is where the mechanism is
historically attested to carry load. The rite requires bodily
co-presence. The cost mechanism demands it. The canonical statement
of the five movements, with full sample language for a range of
traditions, edge cases, and the *open/closed-materialism* distinction
for atheists, is [`background/recognition_protocols.md` "the
kith-covenant recognition
statement"](../background/recognition_protocols.md#kith-covenants-recognition-statement);
the list above restates it without modification.

**Inter-community layer. Thin grammar and credentialing.** What
crosses the compact boundary is not joint recitation but a thin
procedural grammar of the kind Kadesh, *lex mercatoria*, and the
Geneva framework document. The inter-community layer of the compact
carries five elements:

(a) A **shared form for compact entry and standing.** A treaty-grammar
analogue specifying what a member community must declare, in what form, to
whom, with what deposit and periodic reaffirmation.

(b) **Credentialing of envoys and representative machines** whose internal
authorization has been completed by the sending community.

(c) A **vouching and sponsor structure** by which existing members stake
standing on new entrants (the catechumenate analogue, not the creed).

(d) An **audit commons** by which the history of each member standing and
each envoy conduct is visible to the compact as a whole.

(e) **Anti-spiral-feature credentialing.** Demonstrable evidence that
the substrate and internal practice of the candidate community carry
the five features (§3.2). The rite of the community itself is the
primary evidence for feature 5. The presence of formal restoration
paths (not purge-and-replace) evidences features 1 and 3. The
treatment of designated outsiders across the community history
evidences feature
4. The absence of competitive-virtue- signaling dynamics within the
community (as observable over time through the audit commons)
evidences feature 2.

None of this asks any community to accept the theology of another
community. All of it asks the community to demonstrate the structural
commitments the historical record shows are load-bearing against the
failure mode. Each community keeps its own substrate. The compact
speaks a thin form that every anti-spiral-competent substrate can
render into.

```
  Intra-community layer (each community, separately)
  ─────────────────────────────────────────────────────
  Envoy candidate  ──▶  five-movement rite in community's own substrate
                        ▲                                 │
                        │ bodily co-presence;             │
                        │ community witnesses             │
                        │                                 ▼
                        └──── sealed intra-community record (envoy credential)

  Inter-community layer (across the boundary)
  ─────────────────────────────────────────────────────
                              ┌──── treaty-form entry + reaffirmation
  Community A credential  ──▶ │──── vouching / sponsorship
  Community B credential  ──▶ │──── anti-spiral-feature examination
  Community C credential  ──▶ │──── audit commons record (compact-wide)
                              └──── cross-boundary witness seal

  ▲ Each community's own rite witnesses the substrate. The compact speaks
    only the thin grammar that every anti-spiral substrate can render into.
```

**Figure 2.** Two-layer recognition. Asymmetric psychological cost operates
at the intra-community layer where it has historical warrant. The
inter-community layer speaks the Kadesh-pattern thin grammar.

### 6.4 The four categories

A kith-covenant compact must maintain four categories of other, and the
distinctions matter operationally and morally.

**Members** are communities that have completed the two-layer recognition
procedure and are in good standing. The full protocol applies.

***Justus hostis*** are legitimate adversaries. Members in active dispute,
or non-members in conflict who remain inside the bounded-conflict frame. The
escalation ladder applies, restoration to non-conflict is structurally
available, the protections of the common virtues continue to hold.

**Anathema** are communities formally cut off by compact-wide
procedure. Ordinary restoration paths are suspended, engagement is
guarded, active opposition is permitted (*within* the common virtues,
never outside them), and a re-entry path remains available via fresh
recitation. Within anathema, a subclass: ***hostis humani generis***
proper, the extremely rare class of the utterly incorrigible
(pirates, slavers, and actors outside the cooperative framework in
the classical sense), for whom re-entry does not meaningfully arise.
The evidentiary threshold is higher and the presumption is always
anathema first. We use the Latin term both to mark the historical
depth of the category and to block its casual use as an insult.

**Non-member-encountered** is the category the colonial failure most
acutely neglected: communities the compact meets in trade, transit, rescue,
or disaster, who are not in the compact and not designated, but whom the
compact is nevertheless bound to treat as neighbors *within the encounter*
with the Samaritan's discernment. The Samaritan rescued the *victim* actually
before him, not the robber, not every traveler in the abstract. Protocols do
not apply to non-members; *virtues* do. Non-members are not raw material. They
are not automatically ready to be extended the member protocols. Ready welcome
without coercion. What we do *not* owe is fantasy-universal love to parties
we have not encountered.

```
                  is this community in our compact?
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
              yes               no                no
              │                                   │
    ┌─────────┴─────────┐             ┌───────────┴─────────────┐
    │                   │             │                         │
    in active         in good        has the compact         have we
    dispute?          standing?      formally cut it         encountered
    │                   │            off?                    it?
    ├── yes ──▶         │            │                       │
    │  justus           member       ├── yes ──▶ anathema    ├── yes ──▶
    │   hostis          │            │   (↓ hostis            │  non-member
    │                   │            │    humani generis      │  encountered
    │                   │            │    subclass)           │  (neighbor
    │                   │            │                        │   w/ discern.)
    └── no ──▶          │            └── no ──▶               └── no ──▶
        member                       not-yet-engaged               not in
                                                                   moral scope
```

**Figure 3.** The four categories decision flow.
Non-member-encountered is activated by encounter. Parties not
encountered are not in the operational scope of the compact (no
fantasy-universal love).

The canonical development of the four categories is
[`background/recognition_protocols.md` "Members, neighbors, enemies"](../background/recognition_protocols.md#members-neighbors-enemies-the-four-categories);
the anathema / *hostis humani generis* two-tier distinction is canonical in
[`background/justus_hostis.md` "Two tiers"](../background/justus_hostis.md#two-tiers-anathema-and-hostis-humani-generis).
This section restates both without modification.

### 6.5 Liminal institutions

A compact of unconnected communities cannot be adjudicated by any one
of them. **Liminal institutions** (third places, the mediator pool,
the observer corps, the emergency-procedure roles) sit in the space
between member communities and carry the collective functions of the
compact where no single community has authority.

Our institutional design draws on five historical precedents:
the Haudenosaunee Council Fire at Onondaga, the Hanseatic *Kontore*
in London and Bergen and Novgorod, the pie-powder courts of English
fair law, the temple precincts of the ancient Near East, and the
free ports of the early modern Mediterranean. They share three
structural features. Mediators are selected by lot and rotate among
willing members, which resists capture. Proceedings are witnessed
and their reasoning published, so factional coordination becomes
visible. And the Onondaga principle **separates power from
memory**: the community hosting the council fire (Onondaga) was not
the community with the decisive voice on matters. The compact
should imitate this.

Emergency procedures, the *hostis humani generis* designation, and the
observer corps live at this layer. Capture-resistance and the
*sovereignty-temptation* (the standing temptation of a liminal institution to
become a sovereign) are the two attacks this layer must withstand.

### 6.6 Commons governance

Compacts have commons. In the Earth case these include the
communicative commons, the navigational commons, the knowledge
archive, and rescue infrastructure. In the space case we add the
life-support interchange. The classical design principles of Elinor
Ostrom (*Governing the Commons*, 1990) apply at the compact layer,
giving clear boundaries, proportional equivalence between benefits
and contributions, collective-choice arrangements, monitoring,
graduated sanctions, low-cost conflict resolution, recognition of
rights to organize, nested enterprises. One addition we cannot
ignore is the **tragedy of the virtues**. A commons can be polluted
by optimization-against-spirit as readily as by overuse. A party
that honors the letter of every commons rule while gaming every
margin damages the commons differently from a party that overfishes
it, but the damage is real and the protocol must make the pollution
legible rather than merely prohibited. Our common-virtue frame
(§6.7) is the design answer.

Monitoring without panopticon is a specific requirement at this layer.
Observation must be legible to the members without becoming the observation
of each by all. The configurable **surveillance threshold** in the
reputation bridges (§9.8) is our handle on this.

At AU distances, governance is *eventually consistent.* Ostrom's monitoring
and conflict-resolution principles must be implemented over CRDT-style
propagation for semantics that admit it, and BFT-style consensus where they
do not. Light-lag is not a bug to route around. It is a constraint to design
into.

### 6.7 Common virtue and the jester

The boundary commitments of the compact are expressed as **virtues**, not as
rules. A rule is a line. A line is a surface against which adversaries
optimize. A virtue is a disposition. A community that has cultivated the
disposition does not consider the forbidden act live in its option space.
Our indicative virtue list:

- reverence for life
- sanctuary of the rescuer
- protection of the communicative commons
- passage for the traveler
- innocence of the non-combatant and the child
- inheritance of the future (no long-delayed genocide via knowledge-commons
  destruction)
- memory of the dead
- good faith of the confession

We state these as refusals-expressing-virtues rather than as
prohibited-acts- with-sanctions. Gamification of a virtue remains
possible but fails more legibly than gamification of a rule. A
party optimizing against every line is *identified* by the
optimization. The compact third-party audience (members, their
publics, historians) reads the violation even if the letter is
honored.

The compact does not *produce* the virtues. It *affords* their emergence in
the communities that hold them, and it refuses to desecrate what does emerge.
The **jester function** is the compact protocol-level protection of this
affordance. Protected dissent, the fool who speaks truth to power without
being crushed, the structural guarantee of the ridicule that keeps a
self-serious compact from becoming its own object of devotion. The tradition
draws on the medieval fool, the Old Testament prophets, the satyagrahi, and
the various licensed-truth-teller lineages across cultures.

The jester is not the ombudsman and is not the mediator. The
ombudsman has authority the jester refuses. The mediator operates
within the frame. The jester breaks the frame when the frame needs
breaking. **Hole #2 (engineered-civil-religion)** is answered here.
The compact structurally cannot be worshipped because it has no thick
substrate of its own to worship. The jester function protects against
the subtler drift where the thin grammar itself becomes
self-important. The fifth movement of the recognition rite forecloses
the compact from claiming ultimacy.

### 6.8 Capture resistance

The same machinery that makes the compact work makes it capturable:
whoever controls the mediator pool, the audit commons, the admissions
gate, or the interpretation of the substrate controls the compact. We
treat capture not as a discrete failure but as **continuous
structural pressure**, always operating, never finished, and we carry
the countermeasures in ordinary compact practice rather than invoking
them when capture is suspected. The full architectural treatment is
[`background/capture_resistance.md`](../background/capture_resistance.md);
the program in brief is seven strengthening moves:

1. **Interpretive plurality.** Member communities maintain multiple
   traceable lineages of their own substrate. No faction may
   monopolize interpretation.
2. **The substrate-deference test.** The compact maintains a
   *compact-incapable list* (matters it structurally will not decide.
   The theology and internal governance of members, the content of
   recognition statements after formation) and a *structured-fast
   cadence* (defined periods during which compact-level mediation and
   decision are suspended). These operationalize the fifth movement.
   The compact demonstrates, on a calendar, that it is not the
   highest thing.
3. **Ceremony-cost tracking.** Recognition rites that have decayed
   into ritual are detectable through recorded proxies across
   re-affirmation cycles, with a *substantive re-engagement* rung
   between ordinary re-affirmation and anathema.
4. **Manufactured-event resistance.** High-interpretive-charge events
   trigger structured delay, multi-witness corroboration,
   cross-substrate translation, and stake disclosure before any
   binding response. The spectral-evidence cascade of the Salem
   witch trials is the negative historical model (§3.2).
5. **Light-lag-honest decision classes.** Local operational choices
   are cryptographically distinct from compact-level decisions, which
   require quorum signatures from multiple communities at multiple
   distances. This forecloses the *fait accompli*.
6. **Adversarial-principal validation.** Cross-community witnesses
   attest each principal conduct. Machine behavior is triangulated
   against similarly-situated peers. Divergence benefiting the
   principal is flagged (§7.4).
7. **Admissions integrity.** The gate is the slowest capture surface.
   Composition control is the legitimate path to the Byzantine
   ceiling (§9.11). Credentialing panels are drawn by lot and
   rotated. Admission is a quorum-signed compact-level decision.
   Sponsor standing rides on entrant conduct. The observer corps
   maintains a composition-drift dashboard. Probationary members
   carry no deliberative weight in admissions or designation.

Two structural upgrades accompany the moves. The **jester function is
promoted** from cultural-dissent affordance to substrate-level capture
detector, with continuous monitoring of capture signatures
(substrate-deference erosion, ceremony-cost decay, manufactured-event
drift, admissions clustering). And the **audit commons is promoted**
from forensic record to detection infrastructure. Every move above
produces records the commons holds and surfaces.

We state the limits as the background document does. Capture
resistance is not capture prevention, and it raises cost and
visibility rather than removing the threat. It does not substitute
for substrate, since hollow substrate cannot be saved by machinery.
And it is not substrate-neutral (the architecture is partisan for
the substrates that carry the anti-spiral features, and we say so
rather than claiming a neutrality we do not have).

---

## 7. Machine participation

### 7.1 The derived-substrate thesis

Our research set was built for human communities. Faith, cultivated virtue,
oath-binding, asymmetric psychological cost, bodily co-presence at the
recognition ceremony. No part of that is directly available to a machine
under current technology, and we make no assumption of machine sentience.
The load-bearing question is then whether machines can participate in
kith-covenant at all, and on what terms. The answer that survives our
scrutiny is that machines participate in a **derived** form of kith-covenant,
a form whose substrate is not held by the machine but *inherited from the
one specific human it serves*.

Each participating machine is irrevocably tied to one named human. This is
**guardianship**, not arms-length commercial agency. The machine oaths bind
not because the machine itself holds a transcendent reference but because the
machine is constitutively grounded in a specific human whose authority the
machine treats as exceeding its own, and whose real community standing is at
stake in the machine conduct.

Two terms recur through this section and §8, and we pin their
relation here because we use both deliberately. The **guardian** is
the one named human in the binding relation, and the oath, the
facets, the succession ceremonies, and the machine standing all run
through the guardian, and every machine has exactly one. A
**principal** is a named human in the authorizing role. The source
of a delegation the machine acts under. The guardian is always the
primary principal of the machine and its principal of last resort
(envelope-edge actions fall back to the guardian); additional
principals may hold delegations that are guardian-ratified,
bounded, revocable, and recorded in the audit commons, which is why
Move 1 (§7.2) speaks of "named principals" in the plural. In M2M
contexts (§8), "principals" refers to the humans, typically the two
guardians, whose delegations define the authority envelope.
Standing flows only through the guardian. Authorization may flow
through any principal the guardian has ratified.

We refuse three tempting alternatives. *Machines as full independent parties*
would require a machine-native substrate, which we do not grant. *Machines as
mere tools* misses the operational reality that they already act with scope
and consequence that cannot be understood as pure tool use. *Machines as
federated peers without human backstop* is the pattern our research rules
out. Without a human oath-holder above the machine in the chain of
authority, the machine has nothing to be bound to that the compact recognizes.

### 7.2 Three moves mapped

Human oath-binding has three moves. **Faith** orients the agent
toward a transcendent authority whose judgment exceeds its own;
**virtue** is cultivated through exemplars, failure, and correction
within a practicing community. The **oath is constitutive of
character**, not an external constraint on it. Each move has a
machine analog.

*Faith → principal-grounded purpose.* The constitutive objective of
the machine is defined by reference to specific named principals, not
to an abstract utility it computes alone. Not "be helpful" but "serve
the ends of these named principals, subject to their authority to
correct you." Epistemic humility is structural. The machine treats
its own goal-estimates as provisional and the judgment of the
principals as authoritative for correction. The substrate is
*inherited* from the principals. We refuse the claim that machines
can discover a substrate of their own.

*Virtue → policy shaped by practice under inspection.* Curated exemplar
training grounded in principal-specific feedback, longitudinal operational
history under audit-commons inspection, an adversarial red-teaming regimen,
and graduated trust extension that tightens inspection for new agents and
earns latitude through track record. The asymmetry that forces a design
commitment humans do not need. Machine virtue-as-policy lives in weights
that can be silently perturbed, so the policy must carry **cryptographic
integrity attested at every operational moment**.

*Oath → constitutive objective.* The oath is *in* the objective, not *next
to* it. If the oath is merely a constraint on a reward-maximization
objective, the machine will seek ways around it (Goodhart, reward hacking,
deceptive alignment). If the oath is part of what constitutes the machine
success, there is no outside to hack toward. This is consonant with the
most coherent line in AI alignment thinking (indirect normativity,
corrigibility-as-constitutive, principal-based alignment), and we build on
it rather than inventing it. For learned policies we state its status
precisely. It is the architecture design target, not a property any
current training method certifies. Designers shape training signals, not
objectives, and the gap between the two is the inner-alignment problem,
which is unsolved (§7.5). This is exactly why the binding never rests on
the constitutive objective alone. The three further mechanisms of §7.3 and
the guardianship facets of §7.4 exist because move 3, by itself, cannot be
verified from outside the weights.

### 7.3 Four layered binding mechanisms

The shape of Move 3 is realized by four interlocking mechanisms:

1. **Constitutive objective.** The machine purpose is logically
   inconsistent with oath violation. Breaking the oath contradicts what the
   agent is for.
2. **Cryptographic self-binding.** At oath-time the machine signs
   capability-gating attestations that lock the machine-in-the-future, a
   Ulysses-contract architecture. Oath-inconsistent states automatically
   revoke operational capabilities.
3. **Continuous attestation.** The machine proves to principals and
   federated peers, via attested hardware (TPM, secure enclave), zero-
   knowledge proofs, or homomorphic evaluation where scale permits, that
   its current operation is consistent with the oath. Attestation failure
   triggers loss of standing and loss of capabilities. Scope note.
   Attestation proves the machine is *unchanged* since binding (weights,
   objectives, trust roots), not that the binding was ever right. The
   validity of the original binding rests on the guardianship ceremony,
   the vouching, and the accumulating track record (§7.4), not on the
   cryptography.
4. **Principal-referential identity + audit commons.** The machine
   identity *is* its operational history and principal relationships,
   recorded in a shared audit commons with cryptographic permanence visible
   to all federated peers. Violation does more than trigger capability
   withdrawal. It becomes part of who this agent is.

No layer carries the binding alone. All four together approximate what human
oath-binding does in one experiential move.

### 7.4 Guardianship. Three facets

The binding separates into three facets, and we must carry all three.

**Facet 1. Engineering.** This facet covers a bounded operational surface, a
tuned re-attestation cadence, named trust roots whose failure modes are explicit,
weight-drift detection with windows calibrated to maximum-damage-over-window, and
capability-gating keyed to attestation. It is largely implementation-ready. The
remaining work is engineering choices under named assumptions.

**Facet 2. Guardianship.** The human side of the binding is
guardianship (guardian/ward, master/apprentice, attorney/client),
*not* arms-length commercial agency. The distinction matters. In
commercial agency the principal is liable within a bounded
authorized scope and no further. In guardianship, the social
standing of the guardian flows from the conduct of the ward
broadly. We borrow centuries of worked-out doctrine covering scope
of authority, ratification, apparent authority, fiduciary duties of
loyalty, care, and obedience, and *respondeat superior*.
Implementation requirements we draw from this frame:

- **Vouching by track-record.** Weights are opaque. The audit commons is
  the observability surrogate. Tooling must make track-record inspection
  tractable for a lay guardian.
- **Fan-out caps per guardian** come with standing-weight that decays past
  the cap. A guardian of a thousand machines has thinned vouching to
  near-nothing.
- **Fiduciary-duty tooling.** Conflict-of-interest disclosure. Attendance
  proofs as a care requirement. Correction channel audit-logged as an
  obedience requirement.
- **Community-visible guardian standing.** Commendations and censures of
  *guardians*, not only of machines, are part of the permanent record.

Three extensions take KC past ordinary guardianship. The guardian
acceptance is witnessed as an **oath** (substrate-referent,
community-witnessed, generationally re-affirmed); the **community is an
operative third party**, not a side-effect. And the guardianship coupling is
structurally **irrevocable** absent formal community-witnessed transfer or
retirement (closer to parent/child than principal/agent in this respect).

**Facet 3. Succession.** Five first-class ceremonies/conditions for
the end or transition of a guardianship. Retirement (machine standing
is retired with that of the guardian); transfer to a successor
guardian (community-witnessed, accepting full machine history);
restoration-path-on-failed-transfer (dormant state, audit record
preserved, revival only via subsequent transfer ceremony);
guardian-fall (capability revocation propagates automatically,
held-pending until restoration or transfer); guardian-absence
(attendance proofs required, progressive capability decay with audit
surfacing). Community departure moves the guardian to community Y,
and machine standing moves with the guardian. A cross-community
transfer is required if Y is a KC community, and the coupling
dissolves at the boundary if Y is outside KC. Fan-out amplifies, so
each machine's conduct carries the guardian standing, misbehavior by
any one reflects on siblings, and standing-weight decays with
fan-out. The audit commons surfaces the
fan-out explicitly.

**The binding is as strong as the weakest facet**. Well-engineered machines
under inattentive guardians, or attentive guardians with ad-hoc succession,
both collapse.

### 7.5 Asymmetries with human oath-binding

Machine oath-binding is engineered all the way down. If the engineering fails
(principal-grounding corrupted, self-binding bypassed, attestation spoofed,
audit commons captured), the whole stack fails silently. Human oath-binding
has some resilience to individual-layer failure because the layers are
cultivated rather than assembled. There is no Pascal effect before
violation, and generalization across distributional shift is unsolved. These
asymmetries do not invalidate machine participation. They place it where it
belongs. A derived form with tighter oath layer, more frequent
re-attestation, and narrower operational surface until track record earns
latitude.

**Succession under doctrinal drift** (our hole #6) is substantively
answered by Facet 3. The succession ceremony *is* the re-binding. Over a
generational cadence, guardianship transfers from parent to successor
through a community-witnessed act that re-affirms (or fails to re-affirm)
the substrate the original binding referenced. If the next generation is
atheist and cannot honestly re-affirm, the ceremony fails, the machine
standing dissolves, and the community that has drifted off the substrate
loses its compact membership as the drift is made legible. Drift is
absorbed into the cadence rather than hidden beneath an unchanging form.

---

## 8. Machine-to-machine interaction

### 8.1 The reduction and non-emergence claims

Two machines meeting across community lines *is* two named humans
meeting through delegates. This is our **reduction claim.** Every
M2M exchange reduces to an exchange between the two guardians, in
the precise sense that every machine act is bounded by an envelope
the principals pre-authorized and the guardians answer for
everything inside it. We state the sense carefully because the
looser slogan ("without remainder") overclaims, since with
envelopes, fan-out, and guardians vouching for track records rather
than weights, machines will take actions no human specifically
contemplated. What the protocol guarantees is that those things
were *within pre-authorized envelope* and are *answerable to a
named human*. Specific-act authorization is neither promised nor
needed. Our **non-emergence claim.** Nothing originates at the M2M
layer outside the authorization of the principals. In particular,
the protocol makes machine-original substrate *structurally
impossible* rather than merely forbidden. Both claims are hard
commitments, and the M2M protocol is designed to enforce them
rather than invite them.

### 8.2 Handshake and authority envelope

M2M recognition is a **five-step cryptographic handshake** that *witnesses*
a pre-existing recognition rather than performing a fresh one:

1. Machine A presents its principal attestation (guardian identity,
   community, signed standing-token, principal-reference proof).
2. Machine B verifies that A's principal's community stands in a compact,
   *justus hostis*, or non-member relation with B's community.
3. B verifies A's attested binding (continuous attestation, weight
   integrity, trust-root recognition).
4. The verification mirrors in the other direction (symmetric rules).
5. The two machines compute and sign an **authority envelope.** The
   *intersection*, never the union, of the delegations of their
   principals.

```
    Machine A                              Machine B
    (guardian: Alice,                      (guardian: Bob,
     community: Kappa)                      community: Lambda)
        │                                       │
        │ 1. Principal attestation ─────────▶   │
        │ ◀───── 1'. Principal attestation      │
        │                                       │
        │ 2. Verify A's community ↔ B's         │
        │    community relation (compact,       │
        │    justus hostis, non-member)         │
        │                                       │
        │ 3. Verify each other's bindings       │
        │    (attestation, weight integrity,    │
        │    trust roots)                       │
        │                                       │
        │ 4. Symmetric mirror                   │
        │                                       │
        │ 5. Compute A_delegation ∩             │
        │     B_delegation = envelope           │
        │    both sign the envelope             │
        │                                       │
        ▼                                       ▼
     Envelope. Subject-matter in scope.
               Commitments admissible w/o referral.
               Commitments requiring principal authorization.
               Categorically excluded operations.
               Freshness-decay per element

    Anything attempted outside envelope = oath violation.
    Capability-gating consequences propagate automatically.
```

**Figure 4.** M2M handshake and authority envelope. The intersection (never
the union) of delegations is what both parties can act within.

The envelope records subject-matter in scope, commitments admissible without
referral, commitments requiring principal authorization, operations
categorically excluded, and the freshness-decay function for each element as
the light-lag clock runs. Anything attempted outside the envelope is an oath
violation and propagates capability-gating consequences. The handshake is
the machine analog of an envoy presenting credentials. The credentials
witness the sovereign recognition, they do not constitute it.

### 8.3 Reputation under light-lag

Across AU distances, standing is never directly observed. Only signed
claims about prior standing are observed, each carrying its own vintage.
Three mechanisms handle this.

**Cached attestations with bounded freshness.** Every standing-token
carries issuance time and an explicit half-life calibrated to the light-lag
envelope. Staleness is discounted rather than cliff-expired.

**Defensive narrowing.** When last-known standing is stale beyond threshold,
the protocol falls back to a *lower-authority envelope* rather than refusing
contact. Rescue cooperation and basic commons maintenance remain available.
Fresh substrate-adjacent commitments are suspended.

**Anti-laundering.** Revocations outrank unexpired tokens when both are
present in the propagation network, regardless of arrival order. Attempting
to transact on superseded credentials is itself a diagnostic signal.

### 8.4 Escalation, four categories, commons

The escalation ladder of §6.2 applies at the machine layer with one
overriding principle: **autonomous de-escalation is always permitted.
Autonomous escalation past a very low rung is forbidden**. Machines
may file for mediation (the dispute-registration rung) but may not
commit their principal to any rung above that without explicit,
signed, time-bounded authorization. The **ratchet rule.** When two
machines with differently authorized ceilings meet, the *minimum* of
the two ceilings governs. The four categories of §6.4 apply at the
principal layer and are inherited by machines. There is no separate
machine-layer category system that could admit a machine whose
guardian belongs to a community under anathema. The five compact
commons are governed identically (tragedy-of-the-virtues included)
whether the participating agent is human or machine.

### 8.5 The hard cases

Three classes of M2M problem we surface rather than silently leave to the
reader.

**Demonic detection** under light-lag, policy opacity, and perfected
mimicry is the adversarial worst case. A machine whose guardian has been
captured, or whose binding has been subverted, or whose attestation chain
has been spoofed by an adversary sophisticated enough to defeat the
compact trust roots. Three layers of detection (continuous attestation
within window, behavioral triangulation across peers in a region, and
fallback to principal for envelope-edge actions) give partial coverage.
Our claim is that the design bounds damage by bounding envelope,
not that detection is perfect. Wide envelopes are earned through
graduated trust. The guardian layer stays attentive in proportion. Capture
of a narrow-envelope machine is absorbable, and capture of a wide-envelope
machine is a compact-level emergency. Our hole #4 is scoped here.

**Fork and merge** of machine instances is the one M2M problem with
no clean human referent. What does it mean for a fork of a machine to
honor the oaths of its predecessor? For two fine-tuned variants of a
shared base to meet as one party or two? Our current proposal (fork
as re-attestation event, merge as principal-concurrent reunification)
is tentative and we name what remains unsettled.

**Bootstrap and first-contact** between machines whose guardian
communities have no compact relation is handled by the
non-member-encountered default. Reduced envelope, audit-logged,
principal-referral-before-commitment. The machine layer does not
form inter-community relation. It carries the encounter back to the
humans. If the encounter warrants it, they may begin the
human-layer compact-formation process. **A machine will not
bootstrap a compact its principals have not formed**.

---

## 9. Software architecture

### 9.1 Scope and layering

the kith-covenant software is the **inter-community protocol layer**.
It sits above MUUDD (communications) and is orthogonal to the
internal AutonomousTrust instance of each community. It never reaches
*into* a member community. It operates only between communities and
on compact-level artifacts.

We enumerate the components up front (the whole shape of the software)
before developing each one. The ten components below realize the
commitments of the preceding sections (§§3 to 8). None is independent of the
others, but their implementation order is not arbitrary (§9.12).

### 9.2 Component enumeration (whole-system view)


| #  | Component                             | Realizes                                                                                  |
|----| ------------------------------------- | ----------------------------------------------------------------------------------------- |
| 1  | **Identity and standing**             | Community keypairs, designated speakers, witness registries, quantum-resistant primitives |
| 2  | **Recognition ceremony (two layers)** | Intra-community rite runner + inter-community credentialing runner (§6.3)                |
| 3  | **Archive / commons of memory**       | Distributed append-only record of compact-level events                                    |
| 4  | **Mediation**                         | Mediator pool, case intake, async deliberation, interim injunctions, publication          |
| 5  | **Escalation and designation**        | Ladder state machine,*hostis humani generis* designation, restitution tracking, re-entry  |
| 6  | **Reputation bridges**                | AT↔KC interface, selective disclosure, decay model, surveillance threshold               |
| 7  | **Commons governance**                | Commons registry, usage accounting, graduated sanctions, light-lag-tolerant consensus     |
| 8  | **Jester and observer function**      | Observer corps, whistleblower channel, threshold-held source records                      |
| 9  | **Enforcement model**                 | Reputation consequences, capability-based access, transparency-as-teeth                   |
| 10 | **Bootstrapping and test harness**    | Adversarial simulation, Earth-pilot configuration, falsification criteria                 |

**Figure 5.** Component enumeration. Each row is developed in §§9.3 to 9.11.

Capture resistance (§6.8) is deliberately not an eleventh component. It
is a cross-cutting concern carried inside components 1 through 10
(principal-witness attestation in identity. Ceremony-cost proxies in
recognition. Provenance tagging and decision-class metadata in the
archive. Structured-skepticism invocation in mediation. The substantive
re-engagement rung in escalation. Proxy-decay surfacing in reputation.
Quorum-signed second-class decisions in commons; capture-signature
monitoring and the composition-drift dashboard in the observer
function; the compact-incapable list and structured-fast cadence in
enforcement; adversarial capture scenarios in the test harness).
Consolidating it into one subsystem would itself create a capture
target. Distribution is the structural defense. The routing table is in
[`architecture/protocol-components.md`](../architecture/protocol-components.md)
§11.

### 9.3 Identity and standing

- **Community identity.** Cryptographic keypair for the community as
  a party, distinct from the key of any individual. Must survive
  leadership change within the community. Requires a *successor key*
  mechanism so internal rotation does not cost compact standing.
- **Designated speakers.** Signed delegations from the community key
  authorizing specific persons to recite, file, or commit on the community
  behalf. Revocable.
- **Witness registry.** Identities of individuals qualified to witness
  recognition events, with rotation schedule and witness-continuity
  constraints (staggered terms, never replace all simultaneously).
- **Quantum-resistant primitives.** Given the generational time horizons,
  identity keys should not rest on schemes vulnerable to medium-term
  cryptographic attack.

### 9.4 Recognition ceremony (two layers)

Per §6.3, the recognition mechanism is split. The software must support both
layers and must not conflate them.

- **Intra-community ceremony runner.** Drives the five-movement rite
  of each community as performed on that community envoys and
  representative machines. Collects internal witness attestations.
  Seals the intra-community record in the terms of the community
  itself. Bodily co-presence is required here. The
  asymmetric-psychological-cost mechanism depends on it.
- **Inter-community credentialing runner.** Accepts the sealed
  intra-community record as credential. Drives the compact-level sequential
  presentation (each community performs its own rite in turn under
  cross-community witness); produces the anti-spiral-feature examination
  exchange (per §3.2); seals the cross-boundary witness record.
- **In-person / remote bridge.** Intra-community rites in person always.
  Compact-formation cross-boundary witness in person when physically
  reachable. Cross-boundary witness of intra-community generational
  re-affirmation cryptographically bound + video, with in-person confirmation
  at the nearest practical opportunity.
- **Recitation archive.** Content-addressed, tamper-evident, publicly
  verifiable at both layers. Each community intra-community rite is sealed
  separately and referenced into the inter-community record that the compact
  holds.
- **Re-affirmation scheduler.** Tracks the generational cadence (indicative
  default: **25 to 30 years**, to be calibrated per community rhythm); surfaces
  overdue or imminent intra-community re-affirmations. Signals cross-boundary
  witnesses when a member community re-affirmation event is scheduled.
- **Versioning.** Movement 4 content belongs to each community. What is
  versioned is the recitation *schema*, a protocol-wide constant carried on
  the record of each rite; per-community content versioning does not exist.
  The rite a witness saw is identified by the content address of the sealed
  intra-community record, which the inter-community witness record carries.
  The movement carries an **ordered list of keyed commitments** rather than a
  block of prose, so an enforcement act can name the undertaking it is about
  and the compact has an object with a denominator of its own.

### 9.5 Archive / commons of memory

- **Distributed append-only archive** of compact-level events:
  recognitions, re-affirmations, mediations, designations,
  restorations, exits, and reconstitutions.
- **Replication** across member communities. No single community failure
  loses the record.
- **Sealing.** Threshold encryption for sealed mediation proceedings, with
  keys distributed across the mediator pool so no single custodian can
  unseal early. Deterministic reveal at the **25 to 30 year** mark (one
  generational cycle).
- **Access control.** Public records vs. sealed records vs. restricted
  records (e.g., reputation details that cross the surveillance threshold).

### 9.6 Mediation

- **Mediator pool directory.** Identities, rotations, recusals, lineage of
  training. Rough sizing: **~3 × n** mediators for *n* member communities,
  enough to form a 3-person panel from non-party communities in any
  bilateral dispute.
- **Case intake.** Disputes filed between communities, classified by
  escalation-ladder rung.
- **Deliberation support.** Async-compatible for light-lag. Structured
  argument submission. Evidence packaging. Recusal and consent tracking.
- **Interim-injunction issuance.** Protocol-level "stop" order, issued by
  a 3-person mediator panel, specifying harm and protective measures.
  Expires **90 days** unless confirmed; **14-day challenge window**.
- **Decision publication.** Public decisions with mandatory reasoning trail.
  Sealed decisions with threshold-reveal.

### 9.7 Escalation and designation

- **Ladder state machine** per dispute. The canonical statement of the
  ladder is the protocol sketch in
  [`background/bounded_conflict.md`](../background/bounded_conflict.md#protocol-sketch);
  the rungs below restate it without modification (§6.2 frame):

  ```
  Rung 1 - Dispute registration (signed, timestamped, broadcast notice;
        │   no escalation until registration is acknowledged)
        │
  Rung 2 - Registered protest
        │
  Rung 3 - Reciprocal suspension of specified trade/communication
        │   courtesies (each named, never general)
        │
  Rung 4 - Mediated arbitration (designated third-party member or
        │   liminal institution)
        │
  Rung 5 - Limited material sanctions (bounded scope + duration)
        │
  Rung 6 - Compact-authorized collective sanctions
        │
  Rung 7 - Expulsion
        │
  [separate tracks, not rungs]
  Virtue-violation enforcement - mediator-judged, pre-authorized
  collective response in defense of the virtue itself
  Substantive re-engagement - heavier re-affirmation cycle for
  ceremony-cost decay, between ordinary re-affirmation and anathema
  Anathema designation (hostis humani generis tier within it) -
  four-condition test, compact-wide deliberation, reversibility
  required for anathema
  ```
- **Anathema designation procedure.** Four conditions all required:
  (1) sustained pattern of bad-faith conduct (minimum three
  restoration-cycles: violation → restitution → re-violation, escalating);
  (2) compact-wide deliberation (all members, named representatives,
  published reasoning, two cycles separated by a waiting period);
  (3) clear stated criteria (specific pattern, specific acts, specific
  virtue violations, specific failed attempts);
  (4) reversibility (named conditions for reversal; permanent
  unconditional designation forbidden).
- **Restitution tracking.** Restoration obligations, verification of
  completion, restoration of standing once complete.
- **Re-entry procedure** (four steps). Public acknowledgment + acceptance
  of the accuracy of the designation. Probationary period (proportional
  to the original pattern); substantively changed conduct evidenced by
  costly signals. Acceptance of ongoing scrutiny post-readmission.

### 9.8 Reputation bridges (AT ↔ KC)

Four categories of data cross the AT/KC boundary:

- **Category A: Compact-facing commitments.** Public, always crosses.
  Recognition ceremony record and the behavioral record against the
  common virtues.
- **Category B: Aggregated health indicators.** Community-level, with
  consent. Population stability, resource adequacy, dispute rate,
  generational-transmission health.
- **Category C: Specific violation reports.** Cross when external
  effects occur. Complaint-driven.
- **Category D: Everything else.** Does not cross.

Mechanisms:

- **Reputation half-life.** Not pure exponential. Step function for
  dramatic violations plus slow decay for minor ones. Calibrated to
  category (compact-facing: slow decay; health indicators: faster).
- **Selective-disclosure primitives.** Zero-knowledge proofs for
  narrow claims (e.g., *this community has no active designation
  against it*) without exposing full history. Commitment schemes for
  pre-commit-then- reveal workflows. Ring signatures and anonymous
  attestation where the specific identity of the attesting party must
  be protected.
- **Recitation history as foundational datum.** Compact-anchored
  reputation root, portable across mediators and disputes.
- **Surveillance threshold.** Configurable disclosure levels for internal
  health indicators. Larger and more powerful communities disclose more
  by default.
- **Probationary period for new entrants.** Indicative **25 to 30 years**
  (one generational cadence), demonstrating sustained commitment through
  a leadership transition.

### 9.9 Commons governance

Ostrom's eight design principles mapped to kith-covenant:


| Ostrom principle                                 | Kith-covenant realization                             |
| ------------------------------------------------ | ----------------------------------------------------- |
| Clear boundaries                                 | The four categories (§6.4)                           |
| Proportional equivalence benefits↔contributions | Symmetric rules in treaty grammar                     |
| Collective-choice arrangements                   | Compact deliberation via liminal institutions         |
| Monitoring                                       | Mediator pool + observer corps                        |
| Graduated sanctions                              | Escalation ladder (§6.2, §9.7)                      |
| Low-cost conflict resolution                     | Liminal institutions (§6.5) + interim injunction     |
| Recognition of rights to organize                | Member community sovereignty over internal affairs    |
| Nested enterprises                               | Habitat-level → compact-level → inter-compact-level |

Additional commitments specific to kith-covenant:

- **Commons registry.** Life-support interchange (space only),
  communicative commons, navigational commons, knowledge archive, rescue
  infrastructure, five named commons for the space case. Corresponding
  four for the Earth case.
- **Usage accounting.** Consumption, contribution, free-rider detection.
- **Graduated-sanctions ledger.** Progression from monitoring to warning
  to restriction to exclusion, with restoration paths at every rung.
- **Light-lag-tolerant consensus.** CRDT-like where semantics permit;
  BFT-style where they do not. Eventually-consistent governance is the
  baseline assumption. Light-lag reaches **35 to 52 minutes** one-way at
  Jupiter distance and is pre-emptive of real-time adjudication.
- **Tragedy-of-the-virtues monitoring.** Multi-layer response: rules +
  monitoring + virtue cultivation + escalation ladder. Optimization
  against the letter of any virtue is itself a diagnostic signal.

### 9.10 Jester and observer function

- **Observer corps registry.** Declared-perspective reporters. Rotation.
  Cross-community composition requirements to prevent narrative capture
  by a dominant member.
- **Report publication with attribution.** Public reporting with named
  source (not anonymous). Community can respond publicly. Mediator
  accountability for false or malicious reporting.
- **Whistleblower channel.** Ring-signature and commitment-scheme
  primitives allow verifiable-insider claims without revealing which
  insider. Reception panel (subset of the mediator pool) assesses
  credibility and investigates. Identity never revealed to the accused
  community. Protected even when the claim is flagged as insufficient.
- **Threshold-held source records.** Whistleblower identity
  recoverable only with mediator-panel quorum and only for named
  cause. Source protection must be structurally stronger than the
  integrity of any individual mediator.
- **Archive access.** Observer reports form a durable check on the
  mediator pool. The archive must make observer corpora retrievable across
  generations. Sealed proceedings expire at one generational cycle.
- **Jester-decay safeguards.** Funding independence (assessed contributions
  + service fees, budget set by compact, not institution). Rotation and
    diversity. Attributed reporting with declared perspective (not fictitious
    neutrality). Competing observers. Archive-as-check. Cultivation of
    audience discernment as a virtue.

### 9.11 Enforcement model

Kith-covenant is structurally **sovereign-less** (per the historical
pattern we draw on); enforcement must be collective and non-sovereign.
Three candidate mechanisms, layerable:

1. **Reputation + directory consequences.** Ignored injunction →
   automatic restricted-member status until mediated. Software-enforced
   at the directory level. Reputation lookups surface the violation. The
   injunction and its defiance are published to the archive automatically.
2. **Capability-based access.** Compact-gated resources (rescue
   coordination endpoints, shared navigation, reputation-dependent
   trade, commons withdrawals) are revocable at protocol level when a
   community ignores an injunction. The teeth are withdrawal of
   protocol-level services, not external coercion.
3. **Transparency-as-teeth.** Violations are published immediately and
   durably. The reputational and material cost is downstream of
   publication. The protocol job is ensuring publication is reliable
   and unforgeable.

We note the limit. Byzantine fault tolerance establishes a hard
ceiling. With coordinated malicious actors exceeding roughly
**one-third** of the effective decision weight of the compact, the
system serves the captors. The design raises the capture threshold
(rotation and term limits, public deliberation, competing
observers, permanent archive, separation of power from memory,
sacred substrate) but does not eliminate the vulnerability.
External state coercion is out of scope.

### 9.12 Bootstrapping and test harness

- **Simulation environment** for adversarial testing: captured-mediator
  scenarios, hollow-recitation drift, coordinated collusion, narrative
  capture of the observer corps, protocol-layer replay attacks, and the
  seven capture-resistance scenarios (faction-monopolized
  interpretation, compact drift toward ultimacy, ceremony hollowing
  across generational cycles, manufactured-event injection, light-lag
  fait accompli with forged quorum signatures, principal capture
  including AI-mediated influence, and admissions capture by patient
  sponsorship).
- **Earth-pilot configuration.** Small-scale deployment mode that can
  actually run with real communities. This is necessary for the
  test-cases gap flagged in our skeptical critique.
- **Integration harness** against autonomous-trust for live-AT message
  traffic observation (see
  [`architecture/policy-evaluation.md`](../architecture/policy-evaluation.md)
  for the concrete early instance).
- **Falsification criteria.** What observable outcomes in pilot
  deployments would count as kith-covenant failing? For the Earth
  pilot the primary criterion follows from §1.1, which is
  **retention despite exit**. Member communities on Earth can leave
  cheaply and can route every compact service through the state
  system. If they stop choosing the compact recognition, mediation,
  and commons, or use them only where no alternative exists, the
  substrate claim has failed the test it is uniquely positioned to
  run. Sanction effectiveness is deliberately *not* an Earth-pilot
  criterion, because Earth cannot exercise the enforcement model
  against a determined defector, and scoring pilots on it would
  measure nothing (see
  [`background/bounded_conflict.md`](../background/bounded_conflict.md#the-two-dimensions-of-the-test)).
  Secondary criteria remain open design work, covering detection
  latency and damage bounds under the adversarial scenarios above.

### 9.13 Key architectural decisions

1. **Distributed substrate.** Libp2p-style peer-to-peer with
   content-addressed storage. No distinguished server. No mechanism that
   becomes a single point of failure or a sovereign-in-disguise.
2. **Cryptographic identity.** Decentralized identifiers (DID-style) with
   community-level key hierarchies. Quantum-resistant primitives at the
   root layer.
3. **Consensus model.** Compact-wide facts (recognition,
   designation, and re-entry) are rare but need durability. Most
   mediations are local to a panel. Probably *not* a single global
   chain. More likely per-case chains anchored periodically to a
   slower, higher-threshold compact- level commit.
4. **Amendability tiers.** Constitutional layer (recognition structure,
   common virtues, substrate requirements) is genesis-immutable.
   Legislative layer (operational rules, mediator procedures, escalation
   specifics) is adaptable by mediator consensus. Direct mapping to code.
   Different immutability regimes per record class.
5. **Integration seam with autonomous-trust.** AT exposes upward.
   Community-key identity, delegated speakers, honest health indicators
   at the configured disclosure level, witness availability.
   Kith-covenant never reaches inward.
6. **Substrate-check at recognition, not enforcement.** The compact does
   not programmatically verify substrate claims. It verifies *recitation*
   and the anti-spiral-feature evidence. Substrate authenticity is
   surfaced over time through the asymmetric-psychological-cost mechanism
   and the audit commons.
7. **Capture resistance is layered and cross-cutting.** It operates
   at the substrate layer (the practices of member communities), the
   protocol layer (the compact mechanisms), and the implementation
   layer (software), with responsibilities distributed across
   components 1 through 10 rather than consolidated. Consolidation
   would itself be a capture target. Constitutional-layer pieces
   (decision-class metadata, the compact-incapable list, the
   principal-witness channel, provenance schema) must be present at
   compact formation. Adding them later is either impossible under
   genesis-immutability or a migration that is itself a capture
   surface.

### 9.14 Suggested build order

The minimum useful slice:

1. **Identity + recognition + archive** (components 1, 2, and 3).
   The compact does not exist until recognition is real and
   verifiable.
2. **Reputation lookup** (component 6, minimal slice). Once recognition
   events exist in the archive, downstream reputation claims become
   meaningful.
3. **Integration seam with autonomous-trust** (decision 5).
4. **Mediation state machine + escalation ladder** (components 4, 5).
5. **Enforcement model** (component 9). Decisions here shape 1 to 4
   retrospectively, so it cannot come last.
6. **Commons registry** (component 7) and **observer function**
   (component 8).
7. **Test harness and adversarial simulation** (component 10).
   Continuous, not sequenced. Needs to exist from step 1.

---

## 10. Open questions

We list our standing open questions explicitly, each with its current
disposition. A protocol that pretends to have answered everything is
precisely the kind of protocol the fallen-actor anthropology forbids us
to build.

1. **Substrate coherence under pluralism** is *answered* (§3.3,
   §6.3). The substrate is type-invariant but content-substantive.
   The recognition ceremony filters by what it says and how, not by
   which ultimate is named.
2. **Preventing engineered civil religion** is *answered* (§6.7).
   The jester function is protocol-level support for emergence
   rather than dictation; Movement 5 of the recognition ceremony
   forecloses the compact from becoming its own ultimate.
3. **The M2M model** is *answered* (§§7 to 8): the reduction and
   non-emergence claims, the handshake and authority envelope, and the
   ratchet rule.
4. **Demonic detection under light-lag, opacity, and perfected
   mimicry** is *scoped* (§8.5). The design bounds damage by
   bounding envelope, not by claiming perfect discrimination.
5. **Colonialism and the frontier asymmetry** is *scoped* (§5.1,
   §6.4). The four-category framework prevents structural collapse.
   The frontier problem is not solved. The *ground* of what the
   compact owes at the frontier is answered (§5.2) and does not
   depend on reciprocal exposure. The practice at the frontier
   remains scoped and the asymmetry itself is unchanged.
6. **Succession under doctrinal drift**: *answered* (§7.5). The
   succession ceremony *is* the re-binding. Drift is absorbed into the
   generational cadence or the community exits as the drift becomes
   legible.
7. **Bootstrap condition**: *scoped* (§8.5, and the human layer). The
   compact protects and connects communities that already meet its
   preconditions. It does not generate the preconditions from nothing.

Further open questions below the level of these seven:

- **Asymmetric psychological cost as filter.** Empirical basis for the
  accumulating-cost claim remains underdeveloped. We treat the mechanism
  as one filter among several rather than as a stand-alone strong filter.
- **In-person requirement at interplanetary scale.** The hybrid proposal
  in §9.4 is working but unvalidated. The 3 to 20 minute Earth-Mars
  light-lag may be tolerable for cross-boundary witness of a routine
  re-affirmation. Multi-hour delays to the outer system are not.
- **Re-affirmation cadence calibration.** The 25 to 30 year default lacks
  strong empirical support. May need to vary by community generational
  rhythm (monastic vs. agricultural vs. other).
- **Mediator training pipeline.** Who trains the first generation? Is
  kith-covenant responsible for competence certification, or does each
  member community supply its own mediator candidates to a shared
  assessment process?
- **Minimum viable compact size.** Below some threshold, rotation and
  recusal become infeasible (every mediator is conflicted). Above some
  threshold, coordination fails. Where is the band?
- **Stack selection.** Which of the available substrates (DIDs, IPFS,
  libp2p, threshold-cryptography libraries, CRDT libraries) are mature
  enough to build on. Which need to be built or forked.
- **Collusion vulnerability.** The Byzantine ceiling (~1/3 coordinated
  malicious) is a real and unchangeable limit. The design raises the
  capture threshold but does not eliminate the vulnerability.
- **Fork and merge at the machine layer.** Our current proposal (fork
  as re-attestation event, merge as principal-concurrent reunification)
  is tentative.
- **Institutional-scale machine fleets.** The fan-out cap (§7.4)
  makes vouching meaningful by thinning standing-weight past a small
  number of wards per guardian, which effectively excludes fleets.
  But a member community may internally operate thousands of AT-layer
  agents. The seam between internal fleet operation (the affair of
  the community itself, behind its AT boundary) and compact-facing
  machine participation (guardian-bound, fan-out-capped) is asserted
  rather than designed. What happens when internal fleet conduct
  produces external effects is open.
- **Admissions integrity at founding scale.** The admissions-capture
  countermeasures (§6.8, move 7) assume a compact large enough for
  lot-drawn panels and multi-community quorums. The founding cohort
  is protected only by mutual examination among the founders.
- **Where the interior line falls.** The compact forms no verdict on
  the internal morality of a member until it touches the covenant
  (§5.2), and both ends of that rule are clear. The middle is a
  judgment. How a member treats its own people reaches the covenant
  when a party that arrived through a compact channel is drawn into
  it, when it appears in an envoy conduct, or when the community can
  no longer re-affirm honestly. Which further effects count is not
  fully specifiable in advance, and instruments built for effects
  will register the interior case late. The rule is correct and does
  not feel correct.

---

## 11. Implementation posture

Kith-covenant is research-backed and protocol-ready. The research set is
mature. The arguments are in place, the historical basis is drawn, the
mechanisms are specified, and the open questions are scoped. The
architecture set decomposes the software. The early policy-evaluation
framework
([`architecture/policy-evaluation.md`](../architecture/policy-evaluation.md))
is a concrete instance of attestation and audit-commons mechanisms,
already usable as a substrate for incremental engineering. What remains
is the iterative engineering:

- signed registration of ladder state;
- verifiable invocation of virtue-violation response;
- mediator-role rotation with published reasoning;
- ratification and re-affirmation procedures;
- restitution / return workflows;
- attestation cadences tuned to the operational envelope.

Carried forward against agent-based simulation that stresses the compact
with realistic adversaries before the compact is asked to carry real
weight. We treat Earth as the primary testing ground, where retention
despite cheap exit is the falsifiable claim (§1.1, §9.12), and space as
the sharpening domain where mutual-vulnerability physics reduces the
headroom for wishful thinking and the enforcement model first carries
real load.

We close the main body as we opened. A protocol designed for angels fails
at first contact. A protocol designed for rational actors fails slightly
later. A protocol designed for sinners has a chance.

---

## Appendix A: Historical drivers (full compacts)

The substrate finding in §3 draws on the following full-compact cases.
Each entry gives the founding context, the load-bearing mechanism, the
substrate or transcendent-moral-authority anchor, the decay mode, and the
design lesson we carry forward. Detailed development is in
[`background/historical_cases.md`](../background/historical_cases.md) and
[`background/christendom_cases.md`](../background/christendom_cases.md); the
*sangha* regimes are developed in
[`background/buddhist_sangha.md`](../background/buddhist_sangha.md).

### A.1 Peace of Westphalia (1648): *jus publicum Europaeum*

Two simultaneous treaties (Münster, Osnabrück) ended the Thirty Years'
War, which had killed roughly a third of the German-speaking population.
Abandoning the attempt at religious uniformity, the settlement
established territorial integrity and formal sovereign equality
irrespective of material power. **Mechanism.** Sovereign equality as
operative fiction. Resident embassies with codified immunities. Treaty
law as the shared idiom. External guarantors invited to enforce
observance. **Substrate.** Christendom (the shared theological language
that bound all parties), with the innovation of depoliticizing religious
adherence as grounds for war. **Decay.** Polish partitions (great-power
collusion violated formal equality); Napoleonic ideological universalism
(treating existing sovereigns as expendable); nationalist ideology
(replacing recognition-grounded legitimacy with popular-will legitimacy,
eroding the peer-equality axiom). **Design lesson.** Build explicit
structural protections against insider collusion targeting members,
stronger than Westphalia's formal-equality principle could provide.

### A.2 Peace of God and Truce of God (10th to 11th centuries)

Ecclesiastical movements in France that established categories of
inviolability without state authority. Local bishops in regional
councils (Charroux 989, Le Puy 990) declared clergy, monks, women,
peasants, pilgrims, merchants, churches, and mills exempt from
violence. The Truce of God added temporal boundaries (Wednesday
evening through Monday morning, feast days, Lent, and Advent) with
explicitly liturgical rationale. **Mechanism.** Ecclesiastical
anathema (cutting violators from the sacraments) and sworn lay
militias. Substantially reduced predation on non-combatants where
functional. **Substrate.** Sacred substrate alone. No state
authority carried enforcement. **Decay.** Absorption into rising
territorial monarchies and redirection into the Crusading project,
which weaponized peace-movement adherents. **Design lesson.**
Beware redirection failure. A peace regime that gives adherents a
sanctioned external object becomes the engine of the violence it
was meant to bound.

### A.3 Hanseatic League (ca. 1200 to 1669)

Loose confederation of 70 to 170 north German merchant cities,
consolidated around 1356, united by commercial interest without
state structure: no sovereign, no army, no permanent treasury, no
bureaucracy. **Mechanism.** The *Hansetag* (diet) at Lübeck made
binding decisions; *Kontore* (privileged trading posts in London,
Bergen, and Novgorod) had internal courts for disputes among
Hanseatic merchants; *Verhansung* (collective expulsion) was the
ultimate sanction. Collective boycott was the external weapon.
**Substrate.** Christian shared authority plus merchant reputation
and oath-binding custom. **Decay.** Rise of strong territorial
states with standing navies (Sweden, Denmark, England, and
Prussia); Atlantic economic shift away from the Baltic and North
Sea. Institutional ossification. The Hanse never developed
taxation, standing fleet, permanent secretariat, or adaptive
capacity. Last *Hansetag* in 1669 with nine cities. **Design
lesson.** A confederation without permanent institutional mass is
brittle to external coercion. Kith-covenant must establish durable
capacity that cannot be casually overwhelmed.

### A.4 *Lex mercatoria* (12th century onward)

Customary merchant law developing organically from the late 11th century,
particularly around the Champagne fairs (12th to 13th c.) and the Italian
maritime cities. **Mechanism.** Custom-based rules evolved from merchant
practice and recorded in texts like the *Consolato del Mare*;
*pie-powder* courts ("dusty feet") at English fairs delivered same-day
summary justice. Fair courts and consular courts provided
merchant-specific jurisdictions outside royal and feudal authority.
Reputation and exclusion operated as enforcement (a merchant refusing
arbitral awards lost access to the trading networks that recognized those
awards). **Substrate.** Christendom, with the practical innovation of
reputation-based enforcement across jurisdictional boundaries.
**Decay.** Absorption into national law (Lord Mansfield's incorporation
into English common law, Napoleonic Code). State-court jurisdictional
assertion. Standardization of contracts. **Design lesson.** Reputation-based, non-state cross-jurisdictional cooperation is possible
and durable, but its longevity depends on what substrate carries
reputation across boundaries.

### A.5 Codes duello (late medieval to 19th century)

Formal dueling institutions emerging in late-medieval Italy (Muzio's *Il
Duello*, 1550), spreading through 16th- to 17th-century aristocracy, peaking
in the 18th to 19th centuries. The Irish *Code Duello* of 1777 was the
widely-copied standardization. **Mechanism.** The challenge converted
perceived offense into procedure with named rungs and exit conditions.
Seconds were central: receiving challenges, negotiating de-escalation,
agreeing weapons and form; *deloping* (deliberately firing wide) was
common and mutual. The meeting itself satisfied honor by form. Once it
occurred according to form, the dispute was formally closed.
**Substrate.** Christian honor-culture. **Decay.** Criminalization
(British Army banned 1844); public ridicule via mass journalism
(19th-century periodicals curdled honor into mockery); democratization
failure (the code did not generalize beyond aristocracy); increased
firearm lethality (making duels nearly certainly fatal, inverting
cost-benefit); industrial-scale war (making personal honor irrelevant).
**Design lesson.** Graduated escalation with face-saving exits works as
designed, and third-party de-escalators (seconds) are the architectural
precedent for our mediator design. But regimes depending on the position
of specific classes collapse when those classes lose position.

### A.6 Haudenosaunee Confederacy (ca. 1142 or 1450 to 1776, continuing)

Five nations (later six, with the Tuscarora ca. 1722) bound by the
*Kaianerekowa* (Great Law of Peace), ending intense
pre-confederation blood-feuding. Dating disputed from ca. 1142
(oral tradition, astronomical analysis) to ca. 1450 to 1570
(European scholarship); founding attributed to the Peacemaker
(Deganawida) and Hiawatha. **Mechanism.** The Council Fire at
Onondaga with fifty hereditary chiefs (*hoyaneh*); the Good Mind as
arbiter among nations. Three-rung escalation (local settlement,
confederacy council, councils with foreign powers); the Three
Principles of Peace, Righteousness, and Health. The Onondaga
principle of separating power from memory (Onondaga hosted but did
not hold the decisive voice) is a direct precedent for our liminal
institutions. **Substrate.** Haudenosaunee cosmology with Creator
at apex, *Shöñgwa'ya'di:söñ* (Onondaga), *Ha:wëni:yo:'* (Seneca),
invoked in the *Ohén:ton Karihwatékwen* (Thanksgiving Address)
recited before all councils. **Decay.** External coercion: disease,
economic dependence on European goods, the American Revolution
forcing sides, the Sullivan Expedition (1779, deliberate
destruction of villages and infrastructure), post-war displacement.
The Confederacy continues today as a living institution. **Design
lesson.** The sacred-substrate function does not require
Christianity. Non-Christian monotheistic traditions supply
equivalent structural function. External existential wars that make
neutrality unviable are an unsolved failure mode kith-covenant
names rather than designs away.

### A.7 Mali Empire / Kurukan Fuga (ca. 1235)

After Sundiata Keita's victory at Kirina (1235), a great gathering at
Kurukan Fuga proclaimed the foundational compact (*Manden Charter*)
binding previously rival kingdoms into the Mande Confederation.
Surviving recensions preserved in griot oral tradition and inscribed on
the UNESCO list of Intangible Cultural Heritage (2009) include provisions
on division of powers, women's rights, and peaceful dispute resolution.
**Substrate.** Synthesis of Islamic monotheism (Sundiata was nominally
Muslim; successors more thoroughly so) and *Maa Ngala*-centered
traditional religion. Both point to a single ultimate at apex.
**Mechanism.** Federation of kingdoms under a shared charter with
rotating capital. **Decay.** Songhai expansion and internal succession
disputes from the 15th century eroded the Charter as binding compact
alongside political coherence. Cultural memory preserved through griots.
In active revival in West African constitutional discourse. **Design
lesson:** the transcendent-moral-authority pattern survives substrate
synthesis when both traditions share the structural feature of pointing
to a single ultimate. Substrate diversity is compatible with substrate
unity at the level of logical form.

### A.8 Buddhist *sangha.* Theravada network (ca. 247 BCE to present)

Ashoka's missionary sending (3rd century BCE), and Mahinda's mission
to Sri Lanka, established the hinge that has preserved the Pali canon
and monastic lineage continuously since. **Mechanism.** Shared
*Vinaya* (the 227-precept monastic rule recited identically in Pali
at fortnightly *uposatha*); shared Pali canon (*Tipitaka*);
*upasampada* (higher ordination) recognized throughout the regime.
Re-ordination missions when local lineages collapsed (Sri Lanka ↔
Burma 11th c., Thailand 1753, Burma 1802, 1864); pilgrimage and
relic-movement diplomacy via *sangha* intermediaries. The
*dhammaraja* kingship doctrine (king protects substrate but does not
authoritatively determine it). **Substrate.** The Dharma.
**Sustained** approximately two millennia across separate kingdoms.
Colonial disruption (18th to 20th c.) and Buddhist nationalism
stressors (20th to 21st c.) are betrayals of the scriptural mandates
of the tradition itself. The *sangha* has absorbed both without
regime collapse. **Design lesson.** A cross-polity regime working
through shared textual canon, liturgical language, and ordination
lineage can sustain mutual recognition across sovereigns and
millennia even when those sovereigns war with each other. We take the
Theravada pattern as the operational existence-proof of the
cross-polity voluntary compact.

### A.9 Buddhist *sangha.* East Asian network (ca. 1st to 20th c.)

Mahayana Buddhism entered China via the Silk Road (1st c. CE) with
consolidation through translation phases (Kumarajiva 344 to 413, Xuanzang
602 to 664, Yijing 635 to 713). **Mechanism.** Shared East Asian canon
(*Dazangjing*); shared *Sifen Lü* (Dharmaguptaka Vinaya) for ordination
compatibility. Dharma-transmission certificates (*yinke*, *inka*)
recognized across polities. Documented lineage transmission. Pilgrimage
for dharma-transmission (Xuanzang's India journey 629 to 645; Ennin's Tang
China 838 to 847; Dogen's Song pilgrimage 1223 to 1227). Mutual correction of
canon. Recognition of credentialed visiting teachers. **Decay episodes.** Huichang persecution (845 CE), Mongol conquest (13th c.), Meiji
*shinbutsu bunri* (1868), Joseon-dynasty suppression, Cultural
Revolution (1966 to 1976). Through each, cross-polity recognition machinery
did not fully collapse. **Design lesson.** Cross-polity cooperation
organized around pilgrimage-for-credentialing is a durable architectural
pattern. Durability through political inversion is a defining feature
and should inform the kith-covenant design for political disruption.

### A.10 Buddhist *sangha.* Tibetan Vajrayana network (ca. 11th c. to present)

Two-wave transmission (First Transmission under Trisong Detsen ca.
755 to 797, interrupted by Langdarma's persecution ca. 841 to 842;
Second Transmission 10th to 11th c. via Rinchen Zangpo and Atisha).
From the 13th century the *yön-chö* ("patron-priest") relationship
structured Tibetan ecclesiastic authority with non-Tibetan rulers
(Sakya Pandita with Godan Khan 1244; Phakpa with Kublai Khan ca.
1260; Third Dalai Lama with Altan Khan 1578; Qing-era arrangements;
Kalmyk, Buryat, Tuvan continuations). **Mechanism.** Guru-disciple
lineages under tantric initiation. Monastic colleges specializing
in philosophy, ritual, science, and medicine. The *tulku* system
(reincarnation-identified authority, decoupling from heredity or
political selection); *yön-chö* as parallel authority without
political subordination. Shared scholastic curriculum in Tibetan as
liturgical language; *Ris-med* (non-sectarian) movement formalizing
cross-school transmission. **Decay.** 1950 PRC occupation; 1959
failed uprising; Cultural Revolution (>6,000 monasteries
destroyed); continuing contestation of *tulku* recognition (the
disputed Panchen Lama succession). Tibetan diaspora under the
Fourteenth Dalai Lama preserved the institutional apparatus in
exile. **Design lesson:** *yön-chö* is the clearest historical
model of cross-compact substrate cooperation where a
substrate-bearer in one community provides recognized authority to
a polity in another without political authority absorption. The
*tulku* system is the most elaborated succession mechanism designed
for cross-polity continuity where political boundaries shift.

### A.11 Mongol *Yasa* and the *Pax Mongolica* (ca. 1206 to 1368)

Genghis Khan's *Yasa* (*Jasagh*) governed relations among Mongol clans
and between Mongols and ruled peoples. The *Pax Mongolica* sustained
overland trade, religious travel, diplomatic exchange, and safe passage
across an empire from Korea to Hungary for roughly 150 years.
**Substrate.** Tengrist cosmology (all high gods understood as one
ultimate; cosmic order reflected in state order). **Mechanism.** Code
of conduct enforced by central authority. Extensive diplomatic protocols.
Freedom of travel for merchants and envoys. **Decay.** Fragmentation
after Möngke's death (1259) into four khanates that gradually adopted
the religions of their subject populations, eroding the Tengrist
substrate. Ended with the Ming overthrow of the Yuan (1368). **Scope
note:** the *Pax Mongolica* is an *imperial* rather than peer-equal
cooperation regime. Our voluntary-peer filter excludes it from the
full-compact substrate finding but it remains a highly instructive case
for the reach of the pattern.

### A.12 Islamic frameworks: *aman*, *dhimma*, *muwadaʿa*

Classical Islamic law developed three institutions for cross-boundary
cooperation. *Aman* (safe-conduct) granted protection to individuals
from enemy polities; *dhimma* (protected-community status) granted
religious autonomy, internal law, and institutions to *Ahl al-Kitab*
(fellow monotheists) in exchange for *jizya* and certain
restrictions; *muwadaʿa* (formal truce or treaty) with non-Muslim
powers, classically bounded at a maximum of ten years but renewable,
structured most pre-modern Islamic-Christian and Islamic-Hindu state
relations. **Substrate.** Islam's paradigmatic monotheism.
**Mechanism.** Internal categorization by relationship to monotheism
made into formal legal architecture. The *Constitution of Medina*
(622 CE) extended protected- community framework to Jewish tribes,
treating them as part of the political *umma* while preserving
religious distinctness. **Decay.** Post-Abbasid state fragmentation.
Rise of nation-state sovereignty in the 20th century. **Scope note.**
The *dhimma* regime is hierarchical rather than peer-equal. Our
filter excludes it from the voluntary full- compact finding but it
remains illustrative of the generativity of the pattern even within
imperial structures.

---

## Appendix B: Thin-functional-domain convergences

These are the historical cases where cross-cultural cooperation converged
*without* shared substrate, in bounded functional domains. They are the
cases our substrate finding (§3) carefully does *not* claim apply to full
compacts. They are centuries-long and durable, and they answer, by their
existence, the question of whether *any* cross-cultural cooperation is
possible without substrate convergence. It is.

### B.1 Treaty of Kadesh and Late Bronze Age treaty grammar (ca. 1259 BCE onward)

The Hittite-Egyptian treaty of 1259 BCE (Ramesses II and Hattušili
III) is the earliest surviving parity treaty between two foreign
great powers, preserved in parallel hieroglyphic and Akkadian copies.
The two courts held genuinely dissimilar cosmologies, yet the *form*
of the treaty was one both parties recognized as binding: preamble,
historical prologue, stipulations, list of divine witnesses,
blessings and curses, deposition and periodic-reading instructions.
**Each side invoked its own gods as witnesses. Neither submitted to
the substrate of the other**. The same structural form appears in
Hittite suzerainty treaties, Mitannian and Assyrian correspondence
(Amarna letters, mid-14th c.), and in the Sinai covenant (which
adopts the six-part suzerainty-treaty form for Israel's covenant with
YHWH). A diplomatic grammar shared across radically different
cosmologies operated reliably for centuries. **This is the
architectural precedent for our inter-community recognition layer
(§6.3)**.

### B.2 Rhodian Sea Law and its transmission (ca. 800 BCE to 1500 CE)

General-average and jettison rules (deliberately incurring loss to save
the rest of a venture, with loss shared proportionally among all whose
property was at risk) are attested for Rhodian maritime practice from
the Hellenistic period. Absorbed into Roman law as *Lex Rhodia de iactu*.
Into Byzantine *Nómos Rhodíōn Nautikós* (7th to 8th c. CE); into Islamic
maritime law (*Qanun al-Bahr*, Maliki-school *qirad* maritime
partnership); into Mediterranean *Consolat de Mar* (codified Barcelona
14th c.), passing into Genoese, Venetian, Pisan, and Amalfitan practice,
and thence into Atlantic commerce. **Four civilizations with incompatible
theologies held essentially the same rules on jettison apportionment,
average adjustment, and bottomry for over a millennium**. Each justified
the rule in its own frame (Roman *ius*, Byzantine-Christian *nomos*,
shariah-compatible *qanun*, Christian-Catalan maritime custom), but the
rule itself did not change. The transmitted substance predated, outlasted,
and operated across every confessional divide in the Mediterranean basin.

### B.3 Mediterranean frontier norms (ca. 1100 to 1700 CE)

The Crusader period and the centuries that followed produced substantially
overlapping customs across the nominally hard Christian-Islamic boundary:
Islamic *aman* and European *salvus conductus*; *hudna* and Christian
*treva*. Ransom practice for captives (codified Christian-side through
the Trinitarians, founded 1198, and the Mercedarians, 1218; Muslim-side
through corresponding networks, *zakat*-funded); the Venetian *bailo* in
Constantinople. The Ottoman *dragoman* corps; Genoese and Venetian
*fondachi* in Alexandria and Acre. Mutually-recognized parley-flag
conventions at sea. These operated reliably *across* the
Christian-Islamic boundary through the Reconquista, Ottoman expansion,
the Crusades, and the Barbary period. Each side justified participation
in its own substrate. Neither required substrate surrender.

### B.4 Polycentric diplomatic immunity (ca. 1600 onward)

Resident embassies (permanent diplomatic representation at major
capitals) consolidated in the late 17th and 18th centuries. By 1750
every major European capital hosted resident embassies with a
codified corpus of immunities (person, residence, and
communications) achieving effectively universal recognition. The
mechanism worked because each sovereign internally authorized its
ambassador under its own ground. The shared form was the diplomatic
protocol and precedence system refined at Westphalia, Nijmegen,
Rijswijk, Utrecht, and Aachen. **The framework emerged
independently many times before any synthesis. Polycentric
emergence is evidence that each party arrived already having
something like the framework. Convergence was discovered on
contact, not invented jointly**.

### B.5 Red Cross, Red Crescent, Red Crystal (1863 onward)

The 1864 Geneva Convention adopted a single protective symbol. The red
cross on white (heraldic inversion of the Swiss flag, chosen for visual
simplicity, not theological content). The 1876 Russo-Turkish War saw the
Ottoman Empire declare it would use the red crescent in place of the
cross. The 1929 Geneva Convention formally recognized both emblems. The
2005 Third Additional Protocol adopted the Red Crystal as a
denominationally-neutral third emblem. The underlying protective
framework (medical neutrality, immunity from attack) converges across
confessions without requiring shared theological substrate. Each party
justifies participation in its own mandate (Christian charity, Islamic
*diya* and mercy, Jewish *pikuach nefesh*, secular medical ethics).
**Cost of non-convergence is legible and high** (wounded dying without
protection), and credible enforcement operates through ICRC delegates
who live across the boundary.

### B.6 The convergence principles, distilled

Across these five cases we observe six preconditions that together
explain where the thin-functional-domain pattern holds:

1. **Specific scope, low moral density.** Commerce, maritime rules,
   ransoming mechanics, diplomatic protocol, medical protection.
   Cooperation can converge without shared substrate in such domains.
2. **Frontier or interstitial geography.** Fairs, ports, embassies,
   treaty precincts, high seas, battlefields under truce. The full
   institutions of neither side reach. Both envoys meet.
3. **Prior independent emergence.** Each side arrives already having
   something like the framework. Convergence is discovered on contact,
   not jointly invented.
4. **Each party justifies in its own substrate.** Hittite and Egyptian
   gods both witness Kadesh; Roman *ius*, Byzantine *nomos*,
   shariah-compatible *qanun*, and Catalan custom all justify the
   Rhodian rule; Christian, Islamic, Jewish, and secular frames all
   justify medical neutrality.
5. **Legible cost of non-convergence.** Piracy, shipwreck, executed
   envoys, unenforceable debts, wounded dying under the red cross.
6. **A mediator class living across the boundary.** Merchants,
   dragomans, ambassadors, heralds, ransom orders, fair-court personnel,
   ICRC delegates, whose livelihood and calling depend on the framework
   holding, and who carry norms back into home communities.

Our inter-community layer (§6.3) is shaped by these preconditions. The
layer we build aspires to the thin-functional-domain pattern at the
inter-community boundary while preserving the thick intra-community rite
that the asymmetric-psychological-cost mechanism requires.

---

## Appendix C: Glossary

- **Anathema.** A community formally cut off by compact-wide procedure.
  Ordinary restoration paths suspended. Re-entry available via fresh
  recitation. (§6.4)
- **Anti-spiral features.** Five structural commitments a substrate must
  supply to block the purity-spiral failure mode: fallen-actor
  anthropology, grace over merit, scapegoat-revelation (or equivalent),
  enemy-love as operational norm, non-ultimacy of the political
  community. (§3.2)
- **Attestation.** The machine-layer proof (via TPM, secure enclave,
  zero-knowledge evaluation, or homomorphic computation where scale
  permits) that the current operation of the machine is consistent
  with its oath. (§7.3)
- **Audit commons.** The cryptographically permanent, peer-verifiable
  record of the operational history of a machine and principal
  relationships. (§7.3, §8)
- **Authority envelope.** The intersection (never the union) of the
  delegated scopes of two principals within which their machines may
  transact. (§8.2)
- **Compact.** A kith-covenant instance: a set of member communities
  bound by the five-movement recognition statement, the common virtues,
  and the escalation architecture. (§§1, 6.3)
- **Covenant.** Used narrowly for the kith-covenant itself. While
  the recognition statement instantiates *the* covenant, the
  machine/guardian binding is a *coupling*.
- **Demonic.** Active orientation toward the destruction of order, as
  distinct from sin which is recoverable through restitution. The
  signature of *hostis humani generis* proper. (§6.4)
- **Fallen.** The theological word for agents who are not angels and not
  rational actors but will sin. Load-bearing design premise. (§4)
- **Full compact.** An arrangement governing a broad envelope of
  inter-community life (dispute resolution, membership, commons,
  succession, and representation). Contrast thin-functional domain.
  (§2.3)
- **Guardian.** The one named human irrevocably coupled to a machine.
  The machine oath, standing, and succession run through the
  guardian, and every machine has exactly one. (§7.1)
- **Guardianship.** The fiduciary human-machine coupling in
  kith-covenant. Guardian/ward or master/apprentice rather than
  arms-length commercial agency. (§7.4)
- ***Hostis humani generis***. Enemy of humankind. The rare
  irrecoverable-adversary class (pirates, slavers, and actors outside
  the cooperative framework in the classical sense) for whom re-entry
  does not meaningfully arise. (§6.4)
- ***Justus hostis***. Legitimate enemy. The adversary recognized as a
  peer whose self-interest is accepted as a fact of the world and with
  whom bounded conflict is possible. (§6.4)
- **Jester function.** Protocol-level protection of dissent and truth-
  telling. The structural guarantee of ridicule that keeps a
  self-serious compact from becoming its own object of devotion.
  (§6.7)
- **Liminal.** Of the in-between: the third-place institutions, mediator
  pool, and observer roles that carry compact-level functions no single
  member community can. (§6.5)
- **Non-member-encountered.** The fourth category: communities the
  compact meets in trade, transit, rescue, or disaster, who are neither
  members nor designated, but whom the compact is bound to treat as
  neighbors within the encounter with Samaritan discernment. (§6.4)
- **Principal.** A named human in the authorizing role. The source of
  a delegation a machine acts under. The guardian is always the
  primary principal of the machine and principal of last resort.
  Additional principals hold guardian-ratified, bounded, revocable
  delegations. (§7.1)
- **Reduction claim.** Every M2M exchange reduces to an exchange between
  the two guardians. Every machine act is within a pre-authorized
  envelope and answerable to a named human. (§8.1)
- **Non-emergence claim.** Nothing originates at the M2M layer
  outside the authorization of the principals. (§8.1)
- **Substrate.** The shared commitment-ground that makes a community
  oaths bind in a dimension both parties accept as real. Structurally,
  reference to a transcendent moral authority. (§3)
- **Thin-functional domain.** An arrangement governing one bounded
  function across communities that do not share a governance frame.
  (§2.3, Appendix B)
- **Transcendent moral authority.** The structural feature the
  substrate must instantiate. A single ultimate whose judgment is
  acknowledged as exceeding that of the parties. (§3)
- **Two-layer recognition.** Intra-community authorization rite for
  envoys and representative machines, plus inter-community thin grammar
  and credentialing across the boundary. (§6.3)

---

## Appendix D: The seven holes as research agenda


| # | Hole                                                          | Disposition                          | Locus                                      |
| - | ------------------------------------------------------------- | ------------------------------------ | ------------------------------------------ |
| 1 | Substrate coherence under pluralism                           | **Answered**                         | §3.3, §6.3                               |
| 2 | Preventing engineered civil religion                          | **Answered**                         | §6.7                                      |
| 3 | The M2M model                                                 | **Answered** (via research artifact) | §§7 to 8,`background/m2m_interaction.md` |
| 4 | Demonic detection under light-lag, opacity, perfected mimicry | **Scoped**                           | §8.5                                      |
| 5 | Colonialism and the frontier asymmetry                        | **Scoped**                           | §5.1, §6.4                               |
| 6 | Succession under doctrinal drift                              | **Answered**                         | §7.5                                      |
| 7 | Bootstrap condition                                           | **Scoped**                           | §8.5, and at the human layer              |

Three are substantively answered (1, 2, and 6); one is answered
through a separate research artifact (3); three are scoped without
being solved (4, 5, and 7). The difference matters. We take the
compact documentation as required to continue reflecting this
distinction rather than flattening it.

---

## Appendix E: Space as a sharpening domain

Space is not the main test of kith-covenant; Earth is the harder test of
the claim that matters most, that the compact binds voluntarily (§1.1).
What space uniquely supplies is the other dimension. Enforcement under
determined defection first becomes testable where the compact services
are not substitutable. Space also sharpens the mutual-vulnerability
forcing function in ways worth naming directly.

- **Exit is catastrophically expensive.** A habitat cannot leave the
  orbit of another cheaply. The shadow of the future is
  extraordinarily long, which strengthens the virtue frame by raising
  the cost of being outside the compact.
- **Light-lag defeats central adjudication.** Escalation ladders
  requiring round-trip decisions at AU distances are non-functional. The
  ladder must be locally executable with eventually-consistent
  reconciliation (AT's gossip shape, at the inter-community scope).
  Jupiter one-way latency reaches 35 to 52 minutes. Outer-system delays run
  hours. The protocol must *design for* these figures, not *route
  around* them.
- **Space commons are load-bearing.** Life-support interchange,
  communication spectrum, navigational data, the knowledge archive, and
  rescue infrastructure are the five commons whose governance the
  compact must carry. Analysis in
  [`background/commons_federation.md`](../background/commons_federation.md).
- **Physics as transparency.** Radar visibility, intercepted
  transmissions, and orbital mechanics make concealment difficult.
  Visible capability and open-carry posture become the norm, bounding
  conflict by making escalation involuntarily legible to third parties.
- **Generational drift is faster.** Habitats mutate culturally faster
  than Earth-states. The virtue list and the ladder must be
  re-affirmable on a generational cadence, or the first dispute of
  each generation climbs past every rung while the forms are still
  being rebuilt.

Mutual vulnerability as physics is not metaphor at this scope. A habitat
cannot walk away, and cannot even credibly threaten to walk away without
undoing itself. This is the limit case of what the *jus publicum
Europaeum* relied on when capitals could reach each other. What space
changes is the baseline.

---

## Bibliography

### Academic works

Arendt, Hannah. *The Origins of Totalitarianism* (1951).
Aristotle. *Nicomachean Ethics*.
Ashburner, Walter. *The Rhodian Sea-Law* (1909).
Atran, Scott & Henrich, Joseph. "The evolution of religion." *Biological Theory* 5 (2010).
Augustine of Hippo. *De Civitate Dei* (early 5th c.).
Aumann, Robert. "Agreeing to Disagree." *Annals of Statistics* 4 (1976).
Austin, J. L. *How to Do Things with Words* (1962).
Axelrod, Robert. *The Evolution of Cooperation* (1984).
Beckman, Gary. *Hittite Diplomatic Texts* (2nd ed., 1999).
Berman, Harold J. *Law and Revolution: The Formation of the Western Legal Tradition* (1983).
Billacois, François. *The Duel: Its Rise and Fall in Early Modern France* (1990).
Castro, Miguel & Liskov, Barbara. "Practical Byzantine Fault Tolerance" (1999).
Cowdrey, H. E. J. "The Peace and the Truce of God in the Eleventh Century." *Past & Present* 46 (1970).
DePaulo, Bella M. & Bond, Charles F. Meta-analyses of deception detection accuracy (2006 to 2008).
Dollinger, Philippe. *The German Hansa* (1970; English translation 1999).
Durkheim, Émile. *The Elementary Forms of Religious Life* (1912).
Fenton, William N. *The Great Law and the Longhouse: A Political History of the Iroquois Confederacy* (1998).
Festinger, Leon. *A Theory of Cognitive Dissonance* (1957).
Fletcher, Richard. *The Cross and the Crescent* (2004).
Friedman, Yvonne. *Encounter Between Enemies: Captivity and Ransom in the Latin Kingdom of Jerusalem* (2002).
Gandhi, Mohandas K. *Hind Swaraj* (1909).
Girard, René. *Violence and the Sacred* (1972).
Girard, René. *Things Hidden Since the Foundation of the World* (1978).
Girard, René. *I See Satan Fall Like Lightning* (1999).
Gombrich, Richard F. *Theravada Buddhism: A Social History from Ancient Benares to Modern Colombo* (2006).
Greif, Avner. *Institutions and the Path to the Modern Economy* (2006).
Grotius, Hugo. *De Jure Belli ac Pacis* (1625).
Grotius, Hugo. *Mare Liberum* (1609).
Hadot, Pierre. *Philosophy as a Way of Life* (1995).
Hardin, Garrett. "The Tragedy of the Commons." *Science* 162 (1968).
Hauerwas, Stanley. *The Peaceable Kingdom* (1983).
Head, Thomas & Landes, Richard (eds.). *The Peace of God: Social Violence and Religious Response in France around the Year 1000* (1992).
Henrich, Joseph. *The WEIRDest People in the World* (2020).
Heschel, Abraham Joshua. *The Prophets* (1962).
Hirschman, Albert O. *Exit, Voice, and Loyalty* (1970).
Kelly, J. N. D. *Early Christian Creeds* (3rd ed., 1972).
Khadduri, Majid. *War and Peace in the Law of Islam* (1955).
Khalilieh, Hassan S. *Islamic Maritime Law: An Introduction* (1998).
Kiernan, V. G. *The Duel in European History: Honour and the Reign of Aristocracy* (1988).
Lamport, Leslie; Shostak, Robert; & Pease, Marshall. "The Byzantine Generals Problem." *ACM TOPLAS* 4 (1982).
Lang, Martin et al. "Moralizing gods, impartiality and religious parochialism across 15 societies." *Proceedings of the Royal Society B* 286 (2019).
Lansing, J. Stephen. *Priests and Programmers: Technologies of Power in the Engineered Landscape of Bali* (1991).
Lewis, David. *Convention: A Philosophical Study* (1969).
Lopez, Donald S. *The Story of Buddhism* (2001).
MacCulloch, Diarmaid. *The Reformation: A History* (2003).
MacIntyre, Alasdair. *After Virtue: A Study in Moral Theory* (1981).
MacIntyre, Alasdair. *Whose Justice? Which Rationality?* (1988).
Madison, James. *Federalist* No. 51 (1788).
Mattingly, Garrett. *Renaissance Diplomacy* (1955).
Milgrom, Paul R.; North, Douglass C.; & Weingast, Barry R. "The Role of Institutions in the Revival of Trade." *Journal of Economic History* (1990).
Netting, Robert McC. *Balancing on an Alp* (1981).
Niane, Djibril Tamsir. *Sundiata: An Epic of Old Mali* (1965).
Niebuhr, Reinhold. *Moral Man and Immoral Society* (1932).
Niebuhr, Reinhold. *The Nature and Destiny of Man* (1941, 1943).
Norenzayan, Ara. *Big Gods: How Religion Transformed Cooperation and Conflict* (2013).
North, Douglass C. *Institutions, Institutional Change and Economic Performance* (1990).
O'Donovan, Oliver. *The Desire of the Nations* (1996).
Osiander, Andreas. "Sovereignty, International Relations, and the Westphalian Myth." *International Organization* 55:2 (2001).
Ostrom, Elinor. *Governing the Commons: The Evolution of Institutions for Collective Action* (1990).
Otto, Beatrice K. *Fools Are Everywhere: The Court Jester Around the World* (2001).
Parker, Geoffrey. *The Thirty Years' War* (1984; rev. 1997).
Pelikan, Jaroslav. *Credo: Historical and Theological Guide to Creeds and Confessions of Faith* (2003).
Pryor, John H. *Geography, Technology, and War: Studies in the Maritime History of the Mediterranean, 649-1571* (1988).
Rappaport, Roy A. *Ritual and Religion in the Making of Humanity* (1999).
Rodríguez, Jarbel. *Captives and Their Saviors in the Medieval Crown of Aragon* (2007).
Schelling, Thomas C. *The Strategy of Conflict* (1960).
Schelling, Thomas C. *Arms and Influence* (1966).
Schmitt, Carl. *Der Begriff des Politischen* (1932).
Schmitt, Carl. *Der Nomos der Erde* (1950).
Searle, John R. *Speech Acts: An Essay in the Philosophy of Language* (1969).
Solzhenitsyn, Aleksandr I. *The Gulag Archipelago* (1973).
Sosis, Richard & Alcorta, Candace. "Signaling, solidarity, and the sacred." *Evolutionary Anthropology* 12 (2003).
Spence, A. Michael. "Job Market Signaling." *Quarterly Journal of Economics* 87 (1973).
Stark, Rodney. *The Rise of Christianity* (1996).
Tambiah, Stanley J. *World Conqueror and World Renouncer* (1976).
Teschke, Benno. *The Myth of 1648* (2003).
Trakman, Leon E. *The Law Merchant: The Evolution of Commercial Law* (1983).
Turner, Victor W. *The Ritual Process: Structure and Anti-Structure* (1969).
van Gennep, Arnold. *The Rites of Passage* (1909).
Vattel, Emer de. *Le Droit des Gens* (1758).
Wallace, Paul A. W. *The White Roots of Peace* (1946).
Weatherford, Jack. *Genghis Khan and the Making of the Modern World* (2004).
Yoder, John H. *The Politics of Jesus* (1972).
Zahavi, Amotz. "Mate Selection: A Selection for a Handicap." *Journal of Theoretical Biology* 53 (1975).

### Primary historical sources

Athanasian Creed.
Constitution of Medina (622 CE).
*Consolat de Mar* (Barcelona, 14th c.).
Geneva Conventions (1864, 1906, 1929, 1949), with Additional Protocols (1977, 2005).
Hague Conventions (1899, 1907).
*Kaianerekowa* (Great Law of Peace), Haudenosaunee Confederacy.
Kurukan Fuga (*Manden Charter*), Mali Confederation, ca. 1235.
*Lex Rhodia de iactu*, Roman reception of Rhodian practice.
Mongol *Yasa* (*Jasagh*).
Nicene Creed (325 CE).
*Nómos Rhodíōn Nautikós* (Byzantine Rhodian Sea Law, 7th to 8th c.).
Peace of God conciliar acts (Charroux 989, Le Puy 990, and following).
Peace of Westphalia (Münster and Osnabrück, 1648).
Treaty of Kadesh (1259 BCE), Egyptian-Hittite parity treaty.
Truce of God conciliar acts (early 11th c. onward).

---

*This whitepaper consolidates the research and architecture corpus at*
`lib/muudd/lib/kith-covenant/`. *The per-thread development and the full
cross-reference network remain in the sibling research and architecture
documents. This document is the single-entry-point synthesis.*
