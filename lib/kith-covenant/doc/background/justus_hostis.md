*Previous: [Build and toolchain](../../../ethne/doc/architecture/build-and-toolchain.md)*

# Justus hostis: cooperation derived from bounded conflict

## Thesis

Kith-Covenant is the protocol layer for cooperation among
communities that do not share purpose, do not share identity,
and cannot cheaply exit the orbit of the other. The cleanest
theoretical entry into that problem is not to ask "how do
strangers come to cooperate?" but its inverse: **how have
strangers historically come to fight each other within limits?**
Every structural feature that made conflict bounded (mutual
recognition, symmetric rules, protected envoys, defined
war-aims, face-saving exits) becomes, under inversion, a
primitive for cooperation that does not require shared ends.

The theoretical anchor for this inversion is the classical and early-modern
European concept of *justus hostis.* The *just*, or legitimate, *enemy*.

## The Concept

*Justus hostis* is the enemy who is recognized as a peer rather than treated as
a criminal, heretic, or vermin to be exterminated. The concept has roots in
Roman public law (where *hostis* denoted a public enemy, distinct from
*inimicus*, a personal foe) and was elaborated by Grotius (*De Jure Belli ac
Pacis*, 1625), Vattel (*Le Droit des Gens*, 1758), and, most incisively for
our purposes, Carl Schmitt in *Der Nomos der Erde* (1950).

Schmitt's insight is that the *form* of an adversary is logically prior to the
rules of the adversary relationship. If the enemy is treated as a criminal,
then war becomes police action and there can be no law governing it, only
suppression. If the enemy is treated as a heretic, then war becomes extermination,
because coexistence with error is itself the wrong. But if the enemy is granted
the status of *justus hostis*, a legitimate peer whose self-interest is accepted
as a fact of the world, then a whole apparatus of limits becomes possible:
envoys, parley, ransom, protected non-combatants, laws of means, and, above all,
*an end of the war* at which both sides resume ordinary relations.

Bounded war, in other words, is not achieved by moralizing the
combatants. It is achieved by *depoliticizing* the question of the
legitimacy of the enemy. One does not have to approve of a peer
to recognize them as a peer.

## Structural preconditions

Four conditions appear to be necessary for *justus hostis* to function:

1. **Mutual recognition of standing.** Each side must accept that the other
   has the right to pursue its interests by arms without thereby becoming
   criminal. This is the hardest precondition and the one most prone to erosion.
2. **Symmetric rules.** Whatever is permitted to one side is permitted to the
   other, and whatever is forbidden is forbidden to both. Asymmetry is the
   entering wedge of extermination-politics.
3. **A recognized spatial/temporal bracket.** There is a *theater* of conflict
   and a *time* of conflict. Domains and periods outside the bracket are
   protected. Parliaments meet, trade continues, churches are spared, hostages
   are exchanged at the end.
4. **A shared oath-substrate.** The rules only bind to the extent that oaths
   bind, and oaths only bind to the extent that breaking them is costly in a
   dimension both parties acknowledge. This was historically furnished by
   shared religion (see §"The Christendom Question" below). The open design
   problem for kith-covenant is what furnishes it when religion does not.

## The jus publicum Europaeum as institutional expression

From roughly the Peace of Westphalia (1648) through the long nineteenth century,
the European state system operationalized *justus hostis* into a working
institutional order. The *jus publicum Europaeum*. Its characteristic features
were:

- **Sovereign equality.** States of wildly unequal power held formally equal
  standing. Recognition, not capability, was the primary currency.
- **Non-intervention in domestic affairs.** *Cuius regio, eius religio.* Each sovereign settles its internal order without external policing.
- **Embassy and diplomatic immunity.** Envoys are inviolable.
  Channels, envoys, and archives are protected even during
  active hostilities.
- **Laws of war (jus in bello) distinct from reasons for war (jus ad bellum)**.
  One could fight a legally dubious war lawfully, or a just war unlawfully.
  The *form* was judged independently of the cause.
- **Termination norms (jus post bellum).** Wars ended in treaties, not
  conquests-without-remainder. The post-war order included the defeated as
  participants, not subjects.

While the order held, it did not require the European states to
*like* each other, to share policy preferences, or even to share
regime-type. It required only that they agree on the procedure
by which they disagreed.

## The Sacred Substrate and the Fallen Actor

Every major pillar of the *jus publicum Europaeum* (Westphalia,
the Hanseatic League, *lex mercatoria*, the codes duello, even
the Red Cross of the original Geneva convention) emerged within
Christendom. The pattern is too persistent to be coincidental.
It raises two entangled questions that sit at the deepest level
of the design of kith-covenant. First, **was shared faith
load-bearing, or was it merely the cultural medium?** And:
**what does the design of kith-covenant assume about the actors
it is trying to bind?**

The short answer to the first, on the evidence of the case set, is
that shared faith was load-bearing, and the substrate it supplied had
a *specific structural feature.* It pointed to a single ultimate, a
*transcendent moral authority*, to which oaths, recognition, and virtue
could refer. The structural feature is what does
the work. The specific content of the authority (the God of Israel and
his successors, *Shöñgwa'ya'di:söñ*, *Tengri*, the Dharma, *Kwoth*)
varies across the historical cases. The strengthened version of this
claim is developed in
[Historical cases](historical_cases.md#the-transcendent-moral-authority-pattern). The
direct engagement with the further question of *whether what
Christians call God the Father is specifically the answer* is in
[The noticeable hand](noticeable_hand.md), which follows the evidence
where it leads. The short answer
to the second is. Kith-covenant must be built on a realistic anthropology in
which the actors are not rational utility-maximizers and not angels,
but *fallen.* Sinners, in the theological term that is the only one
strong enough to bear the weight. A protocol designed for angels
fails at first contact. A protocol designed for rational actors fails
slightly later. A protocol designed for sinners has a chance. (This
anthropology is, in its developed form, a Christian doctrine. Its
secular restatements (Niebuhr, Solzhenitsyn, Madison) are imports;
[The noticeable hand](noticeable_hand.md) names the source outright,
because the original is more coherent than the imports.)

### Why a transcendent moral authority was structurally necessary

"Shared cosmology" is too weak a statement of what was structurally
necessary. The historical comparative work
in [Historical cases](historical_cases.md#the-transcendent-moral-authority-pattern)
indicates the substrate must specifically include a
*transcendent moral authority*, not just any sacred frame but one
that points to a single ultimate before which oaths bind and which
stands in judgment over human conduct. Three observations argue this
is *structurally* necessary rather than incidental. The argument is
abductive, drawn from a finite and possibly selection-biased case set
(see [historical_cases.md §"The circularity objection, confounds, and
provisional confidence"](historical_cases.md#the-circularity-objection-confounds-and-provisional-confidence)).
It is held provisionally, as the best available reading rather than a
settled result:

- **Oaths were the load-bearing primitive.** Treaties, safe conducts,
  parleys, and merchant contracts all ultimately rested on oaths sworn
  under a transcendent witness. An oath before the same God binds because
  breaking it is costly in a dimension both parties accept as real, not
  because of enforcement that can be mustered here and now, but because
  *the self that broke the oath is damaged by the breaking*. This is a
  very different mechanism than reputation or deterrence, and it does work
  that reputation and deterrence cannot.
- **The enemy-is-peer move requires a common moral horizon**. To recognize
  an opponent as a legitimate peer, one must share enough of a moral
  universe that "legitimacy" has agreed meaning. Christendom provided that
  horizon even through the Reformation: Catholics and Protestants could
  recognize each other as *wrong-but-Christian* peers in a way that was
  much harder with, e.g., the Ottoman Empire (which was often dealt with
  by a parallel set of norms, not the same set).
- **Ritual furnishes the procedural form.** Liturgical time, saint
  days, fair cycles, the grammar of oaths and feasts. These gave
  the medieval and early-modern cooperators a shared
  *choreography* for encounters. Form substitutes for trust among
  strangers, but the form has to be legible to both and has to be
  perceived as belonging to something larger than the parties to
  it.

The Iroquois Confederacy (Haudenosaunee) is the important
non-Christian counterpoint, and it reinforces rather than refutes
the pattern. The Great Law of Peace (*Kaianerekowa*) bound six
nations through a shared cosmology of its own (the mandate of the
Peacemaker, the Condolence ceremony, the council fire), and it
produced many of the same structural features (protected envoys,
formal parley, symmetric procedure, boundedness of feud). The
pattern is therefore not "Christianity specifically" but **a
shared sacred/cosmological frame that makes oaths, recognition,
and virtue mean something without requiring external
enforcement**. Any substrate that can bear that load will do. In
the record to date, none invented from scratch has done the job as
well.

### The anthropology the substrate must assume

Cooperation-protocol engineering has historically imported its anthropology
from economics. Actors are self-interested, consistent in preferences,
and roughly rational. This anthropology is *useful for some modeling* and
*structurally wrong as a design premise*. Humans are not rational actors.
We are (to take the theological word seriously, which the historical
exemplars took seriously) *sinners*. This is not a pessimistic flourish.
It is a load-bearing claim about what the protocol must withstand.

The tradition kith-covenant should draw on here is the Christian realist line
(Augustine, Niebuhr, Solzhenitsyn), supplemented by Girard's mimetic
anthropology and the Madisonian design move. The convergent claims:

- **Evil is not a rare pathology. It is a normal human
  capacity**. The line dividing good and evil, in Solzhenitsyn's
  formulation, cuts through the heart of every human being. If a
  protocol treats atrocity as a shocking aberration, it will be
  paralyzed when atrocity occurs routinely. A protocol built by
  people who have looked at the historical record plans for it.
- **Rivalry is structural, not moral.** Girard's mimetic theory holds that
  desire is borrowed from others and rivalry is the near-inevitable result
  of communities sharing a field of desire. The scapegoat mechanism,
  resolving communal tension by sacrificing a designated victim, is what
  pre-protocol societies evolved to cope with this. Kith-Covenant must not rely
  on that mechanism (it's unjust and eventually collapses into cycles of
  retribution) but must acknowledge the *energy* the mechanism was
  releasing, and provide legitimate channels for it.
- **Ambition must be made to counteract ambition.** Madison's
  insight in *Federalist* 51 ("If men were angels, no government
  would be necessary") is the operative design principle.
  Kith-Covenant should design *so that the self-interest of no
  participant can be realized except through protocol behavior
  that serves the self-interest of other participants too *.
  This is harder than alignment-of-incentives. It is
  mutual-entanglement of incentives.
- **The worst evils are done by people who believe themselves good**.
  This is the recurring finding from twentieth-century political history
  and the one that the *jus publicum*'s universalist decay (above)
  dramatizes. A cooperation protocol must be structurally resistant to
  being weaponized by its own zealots. Any rule that says "except against
  those who violate the rule, who may be treated as criminals" is the
  entering wedge of annihilationist politics, no matter how well-intended.
- **Virtue is a disposition, not a compliance.** Aristotle's *hexis* and
  MacIntyre's retrieval of virtue ethics point to the truth that real
  cooperation emerges from character cultivated in practicing communities,
  not from rule-following. Kith-Covenant cannot produce character. It can only
  protect and afford the communities in which character is cultivated.

### The anti-spiral features. Why "anchor above self" is necessary but not sufficient

A substrate that points to a transcendent ultimate is necessary
for the oath-binding work described above but it is not by
itself sufficient to prevent the characteristic failure mode of
virtue-oriented communities: the purity spiral. Substrates that
supply *anchor above self* have produced this failure mode
repeatedly and at scale. The Jacobin Cult of the Supreme Being
executing the insufficiently virtuous, the denunciation cascades
of the Maoist Cultural Revolution, Stalinist purge logic
consuming its own old guard, and the various 20th-century
ideological regimes that sacralized their political programs.
What these cases share is not absence of substrate but absence
of specific anthropological features that block the
competitive-virtue dynamic from terminating in victim-selection.
(The Salem witch trials are deliberately absent from this list.
The substrate at Salem held the features and was defeated
through its own structures. That is a different and more
instructive failure mode, treated in [the Salem subsection
below](#the-salem-witch-trials-the-features-can-be-held-and-still-defeated).)
The diagnostic tradition is well-established: MacIntyre (*After
Virtue*, 1981) names the upstream condition (virtues lose their
teleological grounding and become free-floating emotivist
weapons); Girard (*Things Hidden Since the Foundation of the
World*, 1978; *I See Satan Fall Like Lightning*, 1999) names the
downstream dynamic (unbounded competitive virtue-signaling
discharges through the scapegoat mechanism); Arendt (*Origins of
Totalitarianism*, 1951) independently identifies the same
cascade in the secular 20th-century cases.

The narrower-and-stronger form of the substrate requirement is
therefore that the substrate must supply **five features**, whose
combination blocks the purity spiral:

1. **Universal fallen-actor anthropology.** Every member of the
   community is fallen, including the most virtuous. The line
   between good and evil runs through each person, not between
   groups. This is Solzhenitsyn's formulation drawn from Augustine
   and Paul. The feature forecloses the "us-who-are-pure-versus-
   them-who-are-impure" move that purity spirals structurally
   require. Substrates that locate sin or impurity only in a
   designated out-group (tribalist cosmologies, Nietzschean
   master/slave moralities, revolutionary ideologies that locate
   evil in a class or race or ideological faction) fail this
   feature by construction and are not candidate substrates for
   kith-covenant membership regardless of whether they acknowledge
   an ultimate.

2. **Grace over merit.** Virtue is received, not earned. This is
   the Pauline move as developed by Augustine and the Reformers,
   and structurally analogous moves exist in other traditions
   (the Buddhist recognition that attachment to personal virtue
   is itself an obstacle; the Sufi *tawakkul*; the Stoic
   recognition that virtue belongs to the cosmos rather than to
   the virtuous agent). The feature makes competitive
   virtue-signaling theologically or functionally incoherent. If
   virtue is not an achievement of the agent, there is no
   status to compete for. Substrates that treat virtue as a
   competitive status marker, or that ground self-worth in
   accumulated virtuous works, fail this feature. Note that
   feature 2 is *not* the same as feature 1. A substrate can
   hold that all are fallen while still treating virtue as
   earnable, which is exactly the position of the Pharisee and
   the position Paul's argument in Romans is aimed at
   dismantling.

3. **Scapegoat-revelation, or a functional equivalent that
   disarms the sacrificial mechanism**. Girard's argument is that
   the Cross is the story of the innocent scapegoat whose
   victimization is revealed *as* scapegoating, which subsequently
   disarms the mechanism by making every later scapegoating
   visible as what it is. This is the feature of the five that
   Girard takes to be genuinely Christian-unique. Other traditions
   have partial functional equivalents: Buddhist anatta plus
   compassion-for-all-sentient-beings dissolves the self-other
   boundary that the scapegoat mechanism exploits; Haudenosaunee
   condolence practice for dead enemies formalizes
   anti-sacrificial mourning. The Jewish hidden-tzadikim motif
   plus teshuva plus the Isaiah servant-songs carry a version of
   the same revelation. Mature Sufi traditions of enemy-love and
   suspended judgment supply another version. Whether these
   partial equivalents are as structurally sound as the Christian
   case is a live question (see [The noticeable hand](noticeable_hand.md)).
   What is clear is that *some* form of this feature is load-bearing
   and substrates that lack any form of it, that simply treat
   victim-selection as healthy communal hygiene, fail decisively.

4. **Enemy-love as operational norm.** Not sentiment, structural
   constraint. The Samaritan parable is the paradigmatic instance.
   Moral seriousness is owed to the person actually before you,
   and discernment operates *within encounter*, regardless of
   which side designated whom as enemy. Other traditions with
   strong analogs: Haudenosaunee mourning practices for dead
   opponents. The *aman* and *dhimma* extended beyond strict
   Islamic-legal requirements in various historical moments;
   Buddhist *mettā* (loving-kindness) meditation as a formal
   practice. The Stoic *oikeiōsis* extended outward. The feature
   blocks the in-group/out-group compression on which purity
   spirals depend. Without it, the mechanism has no one to expel
   its tension onto. Substrates that treat outsiders as raw
   material or as categorically less-than fail this feature.

5. **Non-ultimacy of the political community.** The *polis*, and
   the compact, cannot be the ultimate, because neither is what
   the commitments, the loyalties, and the hopes of the
   community are for. The Augustinian two-cities formulation is
   the deepest Christian version. The Buddhist renunciation
   traditions carry an analogous distinction; Jewish theology
   holds the eschatological hope outside the political present.
   The Haudenosaunee confederacy rests on a cosmology that
   predates and exceeds the confederacy. Any substrate that
   treats the political community, the nation, the party, the
   revolution, or the compact *itself* as the ultimate fails
   this feature and is structurally vulnerable to the
   civil-religion drift that makes purity-spiraling terminal.
   The fifth movement of the recognition ceremony
   ([Recognition protocols](recognition_protocols.md)) is the
   explicit compact protocol-level encoding of this feature. But
   the feature must also live inside the substrate of each
   member community, or the fifth movement becomes a
   performative ritual without interior ground.

   **Non-ultimacy as assertion does no work.** Stated only as a
   founding-document claim, the fifth feature is unfalsifiable.
   Although any compact at any historical period can declare
   non-ultimacy, it can continue to act in every operational
   moment as if it were the ultimate. The Jacobin Cult of the
   Supreme Being formally acknowledged a transcendent reference.
   This did not prevent it from executing the insufficiently
   virtuous in the name of that reference. For the feature to
   function as load-bearing, the compact must **demonstrate**
   non-ultimacy in observable practice, not merely assert it.
   The mechanism is the **substrate-deference test** developed
   in [capture_resistance.md "The substrate-deference
   test"](capture_resistance.md#2-the-substrate-deference-test-operationalizing-non-ultimacy).
   The compact maintains a **compact-incapable list** of matters
   it structurally will not decide : the theology and governance
   internal to member communities, the content of recognition
   statements after compact-formation, and the truth-claims of
   the tradition of any member. It also observes a
   **structured-fast cadence** during which compact-level
   mediations, sanctions, and decisions are suspended on all but
   life-critical commons. Erosion of either (exceptions
   multiplying in the compact-incapable list, the
   structured-fast period shrinking under pragmatic argument) is
   the earliest legible sign that the compact is drifting toward
   ultimacy, even when no individual exception is obviously
   illegitimate. The test is therefore both **operational** (it
   forms and maintains the disposition non-ultimacy requires)
   and **diagnostic** (its erosion is observable as early-stage
   capture).

**Features 3 and 5 are two countermeasures against one entity, and
they come from two different accounts of it.** Feature 3 is
Girardian, and its remedy is exposure: the mechanism is disarmed by
being seen, named, and refused for what it is, which assumes there is
something anomalous to see. Feature 5 is closer to Wink's account of the Powers, whose
fall is the claim to ultimacy and whose remedy is renunciation rather
than exposure. Stacking both is not redundancy. The two answer
failures that arrive by different routes, are defeated by different
means, and leave different traces, so a community can defeat either
one while satisfying the other. Salem is the case where the
features were held doctrinally and the mechanism ran anyway, which is
feature 3 defeated. The case where both are satisfied and the
community still narrows what it enforces down to a fraction of what
it recites is
[Substitution](substitution.md), and it is neither exposure nor
renunciation that catches it.

**What the five-feature formulation accomplishes that the bare
"anchor above self" formulation did not:**

- It supplies a principled basis for excluding substrates that
  point to an ultimate but lack anti-spiral structure: Jacobin
  sacralized-virtue substrates, Maoist revolutionary-virtue
  substrates, pure-tribalist cosmologies that locate sin only in
  out-groups, Nietzschean will-to-power substrates, and
  closed-materialist utilitarianisms without fallen-actor
  anthropology. The earlier formulation admitted several of these.
  The anti-spiral formulation refuses them.
- It names what the credentialing of the compact requirement must
  actually check for at the inter-community layer
  ([recognition_protocols.md "Two
  layers"](recognition_protocols.md#two-layers-intra-community-authorization-and-inter-community-grammar)).
  Not "does the creed of the community name an ultimate?" but
  "does the substrate of the community and practice carry the
  five anti-spiral features?" Evidence-providing for each feature
  is specified in [overview.md §5b](../concept.md).
- It acknowledges that the most complete combination of the five
  features is supplied by historical Christianity without thereby
  requiring Christianity as the substrate of the compact. Other
  traditions (Judaism, Buddhism, Haudenosaunee practice, mature
  Sufi Islam, and various indigenous traditions with strong
  guest/stranger obligations) supply the features in varying
  combinations and depths. The compact requires the features. It
  does not require the theology that most fully supplies them.

### The Salem witch trials. The features can be held and still defeated

One case in the historical record refuses to sit in the list that
opens the preceding subsection, and the design must be built against
it rather than around it. The Jacobin, Maoist, and Stalinist cases
lacked the anti-spiral features outright. Sin located in a class,
virtue made a competitive status, the political program sacralized
as ultimate. Puritan Massachusetts is different in kind. Its
substrate held all five features *doctrinally*, and held them
emphatically. Total depravity is the fallen-actor anthropology at
maximum strength. Grace over merit is the Calvinist core. The Cross
as terminal scapegoat, enemy-love, and the two-cities subordination
of the polity were all confessed from its pulpits. And in 1692,
Salem Village and its neighbors hanged nineteen innocents, pressed
one man to death, and imprisoned roughly one hundred fifty more
across some twenty-five towns, in about sixteen months, on the
strength of a purity spiral the confessed substrate did not prevent.

What defeated the features was not insincerity. The community was
not hollow. It was intensely, anxiously sincere. The features were
defeated *structurally*:

- **Visible-saints ecclesiology defeated feature 1.** Church
  membership gated on a credible conversion narrative rebuilt the
  pure/impure distinction that total depravity forbids, and
  witchcraft accusation became the mechanism for making hidden
  reprobates visible.
- **Assurance-anxiety defeated feature 2.** Election by grace,
  invisible, and unearnable, made visible sanctity the currency
  of a competitive status economy. Exactly the Pharisee dynamic
  noted under feature 2 above, run at community scale, under a
  creed built to forbid it.
- **National-covenant theology defeated feature 5.** The errand
  into the wilderness fused polity and church, so a threat to
  social order was a threat to the covenant itself, warranting
  sacrificial response. The trials also ran in a sovereignty vacuum
  (the old charter revoked, the new one just arrived), when the
  fused identity was under maximum stress.
- **Procedure amplified what doctrine forbade.** The court
  admitted spectral evidence (unfalsifiable, single-witness,
  undeniable by the accused) and spared confessors, who
  characteristically named others. An unfalsifiable accusation
  channel plus an incentive to confess-and-accuse is a cascade
  engine. No feature of the substrate demanded either rule, and
  the safeguards of the substrate never touched them until the
  end. (Historians weight the underlying drivers differently:
  village factionalism and property rivalry in Boyer and
  Nissenbaum's reading, frontier-war trauma in Norton's. The
  procedural amplifiers are not in dispute.)

And the end is the other half of the lesson. The spiral was
terminated from *within the substrate*. Increase Mather's *Cases
of Conscience* (October 1692) argued on explicitly theological
grounds that God's standards condemned the procedure of the
court ("it were better that ten suspected witches should escape,
than that one innocent person should be condemned"). The court
was dissolved. Its successor barred spectral evidence and
acquitted. Then the sequence with almost no parallel in the
history of witch panics. The repair came in public. A fast day
of repentance (1697), Judge Sewall standing in church while his
confession of guilt was read aloud, twelve jurors publishing a
recantation, the public confession of the lead accuser (1706),
reversal of attainder and paid restitution to the families of
the victims (1711). Compare the terminations of the featureless
cases. The Terror ended by coup, the purges by the death of the
purger, the Cultural Revolution by the death of Mao. None
self-terminated. None repented. Salem self-terminated in roughly
a year, by appeal to an authority above the process, and then
repaired.

Three design commitments follow:

1. **The features metabolize. They do not inoculate.** Their
   load-bearing function is not the prevention of purity spirals
   but their termination and repair. A community whose substrate
   carries the features retains an authority above its own process
   to appeal to, and a grammar of confession, restitution, and
   return to repair with (the channels named under "What the
   substrate *can* do" below). The compact must therefore expect
   spirals *inside* feature-carrying member communities as a normal
   contingency, exactly as the fallen-actor anthropology expects
   atrocity, and must ask of a candidate community not "will you
   never spiral?" but "when you spiral, what stops it, and what
   does your history show it cost you to repair?"
2. **Credentialing must examine operative practice, not
   confession of doctrine, and not sincerity either**. The
   reciters at Salem would have passed any creed-level
   examination and any sincerity assessment. The community
   believed. What fails 1690 Massachusetts is its practice
   record, on the very proxies the inter-community layer
   specifies
   ([Recognition protocols](recognition_protocols.md#the-inter-community-layer-thin-grammar-and-credentialing),
   [overview.md §5b](../concept.md)). Treatment of designated
   outsiders (four Quakers hanged on Boston Common, 1659-1661;
   banishment as the default response to dissent),
   purge-and-replace rather than restoration paths, and a
   legible competitive-sanctity economy. The proxies work; Salem
   is the case that shows why they, and not the creed, are the
   examination.
3. **Spiral speed and spiral contagion are design constraints in
   their own right**. Salem ran from ambient tension to twenty
   dead in sixteen months and crossed town boundaries in weeks.
   The credentialing and re-affirmation of the compact
   instruments run on a generational cadence and cannot see it.
   Only the continuous instruments operate at spiral speed: the
   observer corps and jester function, ceremony-cost proxies,
   aggregated health indicators (an internal-dispute-rate spike
   is what a spiral looks like from outside), and the
   whistleblower channel. And because spirals propagate
   mimetically across community lines, a spiraling member is a
   hazard to its neighbors, not only to itself. The compact
   needs an explicit posture toward a member community
   mid-spiral, before external effects occur, that the
   internal-sovereignty principle does not currently supply.
   Both constraints are flagged in the open questions below. The
   complementary case, a community not spiraling but in stable
   equilibrium with the mechanism, is treated in
   [Sacrificial equilibrium](sacrificial_equilibrium.md). The
   proxies described here are tuned to the spiral and do not
   detect it.

These commitments are tuned to one failure mode: *sincere*
failure. Salem believed. Its features hollowed structurally, not
by the design of anyone, and the practice-record examination
(commitment 2) works because a spiraling-but-sincere community
cannot help leaving the tell in its treatment of outsiders and its
purge-over-restoration reflex. A distinct adversary inverts this:
the strategic feigner, the Machiavellian fox of [objections.md
§"The Machiavellian
operator"](objections.md#the-machiavellian-operator), is insincere
*by design*, and builds a clean practice record precisely to pass
the examination that catches Salem. Against that actor the
practice proxies are not a detector but a target, and the
restoration paths that repair a sincere spiral become an exploit
surface. The compact should not read its Salem defenses as
covering both. The sincere spiral is caught by the practice record
and metabolized by restoration. The strategic feigner is a capture
problem, handled (imperfectly, and by its own admission
probabilistically) by the continuous instruments and the
admissions integrity of
[Capture resistance](capture_resistance.md) move 7, not by the
credentialing gate alone.

### What the compact requires, what it does not, and why

The design principle that falls out of this anthropology must be stated
precisely, because imprecise formulation produces a false contradiction
(the appearance that the compact simultaneously "cannot dictate content"
and "requires acknowledgment of a Creator"). The precise formulation:

**Kith-covenant requires four things of the substrate of its
members **.
1. **Acknowledgment of a Creator.** A transcendent source of the
   moral order, to whom oaths are ultimately directed and before
   whom the commitments of the compact are made. This is a deist
   minimum: *that* a Creator exists, not *what* the Creator looks
   like. A Christian community, a Muslim community, a Hindu
   community, a Haudenosaunee community, and a philosophical-deist
   community all satisfy the requirement despite radically
   different theologies of the Creator's nature. What the
   requirement excludes is *closed materialism.* The assertion
   that no transcendent moral source exists and that the oaths of
   the compact are therefore binding on nothing beyond the
   contingent willingness.

2. **Pro-human orientation.** The substrate of the member must
   hold that human beings bear inherent dignity, what the
   Christian tradition calls *imago Dei*, what other traditions
   express differently but structurally. A substrate that treats
   humans as raw material, expendable means, or objects of
   domination is incompatible with the compact regardless of
   whether it acknowledges a Creator. This is the ethical
   constraint that prevents the deist minimum from accommodating
   a Creator-acknowledging but anti-human theology.

3. **Pro-cooperation orientation.** The substrate of the member
   must hold that cooperation across community boundaries is a
   good, not merely a tactical convenience but something
   consistent with the Creator's purposes (however understood).
   A substrate that holds all outsiders as enemies, all
   inter-community contact as contamination, or all cooperation
   as compromise is incompatible with the compact even if it
   acknowledges a Creator and affirms human dignity.

4. **The five anti-spiral features.** The substrate of the
   member must carry the five structural commitments developed
   in the [preceding
   subsection](#the-anti-spiral-features-why-anchor-above-self-is-necessary-but-not-sufficient).
   Universal fallen-actor anthropology, grace over merit,
   scapegoat- revelation or a functional equivalent, enemy-love
   as operational norm, and non-ultimacy of the political
   community. The first three requirements above establish that
   the substrate points above the compact and affirms human
   dignity and inter-community cooperation. The fourth ensures
   that the substrate *also* blocks the characteristic failure
   mode of virtue-oriented communities, which is the purity
   spiral terminating in the scapegoat cascade. Without feature
   4, the first three are necessary but not sufficient. History
   supplies too many cases of substrates that met (1)-(3) and
   still produced mass violence because the anti-spiral features
   were missing. Substrates that meet (1)-(3) but fail any of
   the anti-spiral features are excluded from full membership
   for the same structural reason any other incompatible
   substrate is excluded. The mechanisms of the compact assume
   what the failing substrate cannot supply.

**This framework is exclusionist, and deliberately so.** Communities
that reject the existence of a Creator, that hold anti-human
orientations, that regard inter-community cooperation as contamination
rather than a good, or whose substrate lacks any of the five
anti-spiral features are excluded from full membership. The exclusion
has real costs. Communities that are pro-human and pro-cooperation
but not Creator-acknowledging are filtered out by requirement (1), and
communities that acknowledge an ultimate and affirm human dignity but
lack anti-spiral structure (the Jacobin case, the Maoist case, various
revolutionary-virtue programs) are filtered out by requirement (4).
The compact should be straightforward about these costs rather than
minimizing them. The restricted-member and non-member-encountered
categories ([Recognition protocols](recognition_protocols.md))
provide frameworks for cooperation with excluded communities, but
they are not full membership. The entry requirement is not a prejudice
but a precondition. A compact whose mechanisms depend on a sacred
substrate with anti-spiral structure cannot admit members who lack
the substrate or the structure without hollowing its own foundations.

**What the compact does *not* require and must not dictate:**

- The **nature of the Creator.** Triune, unitary, personal,
  impersonal, immanent, transcendent, or any other
  characterization beyond "exists and is the source of the moral
  order." First-order theology is the business of the member
  community itself (see
  [Exit and succession](exit_and_succession.md) §"The
  futility lesson and its limits" on the first-order /
  second-order distinction).
- **Specific observances, creeds, or liturgical forms.** The compact
  has a recognition ceremony. Member communities have their own
  worship. The two do not merge.
- **A canonical narrative** about history, creation, eschatology, or
  salvation. The compact has a shared institutional memory (the
  archive). Member communities have their own sacred narratives.

This formulation resolves the apparent contradiction. The compact
*does* dictate structure (a Creator, pro-human, pro-cooperation)
and *does not* dictate content (the Creator's nature, the theology
that surrounds the Creator, the worship that expresses the
theology). The distinction is not "structural vs. doctrinal" in
the weak sense of that phrase. It is the distinction between **the
minimum conditions under which oaths bind and cooperation is
possible** and **the theological elaboration that each community
builds on those conditions**. The former is the business of the
compact. The latter is not.

### Excluding secular communities. The cost of the requirement

The requirement excludes some communities from full membership,
and the compact should say so plainly rather than soften it. A
community that is pro-human and pro-cooperation but does not
acknowledge a Creator is filtered out of full membership. Not
because its people are less trustworthy or less capable, but
because the binding force of the compact mechanisms (oath-binding
before an ultimate, the asymmetric psychological cost of the
recognition rites, substrate-grounded discernment) presuppose the
substrate they run on. Admitting a community that rejects the
substrate to full membership would be like admitting a non-swimmer
to a swim team. The entry condition is not a prejudice about the
person but a precondition for the activity. This is the same point
made just above, stated now as a cost rather than a definition.

The cost is real, and naming it honestly is part of the design.
Communities of good will are turned away from the inner circle on a
ground they may regard as arbitrary, and the compact gains nothing by
pretending the exclusion is painless or that the excluded are deficient.
What the compact offers instead is cooperation short of full membership.
The restricted-member and non-member-encountered categories of the
standing taxonomy ([Recognition protocols](recognition_protocols.md))
provide structured channels for trade, mutual aid, and bounded
cooperation with communities outside the substrate. These are genuine
relationships, not consolation prizes. But they are not full membership,
and presenting them as equivalent would be dishonest. The compact holds
the line at full membership because that is where the sacred-binding
work happens, and keeps the door open everywhere short of that line.

What binds a restricted member to the common virtues it commits
to is not left unaccounted for. It is the thin functional
minimum the compact has adopted. That is the same ground that
carries the conduct of the compact itself where consent has run
out, stated in [natural_law.md "The ground the compact claims
for
itself"](natural_law.md#the-ground-the-compact-claims-for-itself).
The restricted member is held to the thin part of the standard
on the only ground the compact claims for itself, which is what
makes the category coherent rather than a courtesy. It is also
why the category stays restricted. Nothing in the thin part
supplies the five anti-spiral features, and no functional
minimum can be made to yield them.

### The Rawlsian objection and the durability hypothesis

The Rawlsian liberal objects. Requiring Creator acknowledgment
as a condition of full membership violates the principle of
*public reason*, the principle that the basic structure of a
cooperative framework must be justifiable without appeal to any
particular comprehensive doctrine. Rawls argues that justice
requires doctrine-neutrality, so that communities with radically
different worldviews can endorse the framework from within their
own comprehensive doctrines (the "overlapping consensus"). By
requiring a Creator, kith-covenant *unnecessarily narrows* that
consensus and creates second-class members (the
restricted-membership category for communities that are
pro-human and pro-cooperation but not Creator-acknowledging).

The objection is formally correct. Kith-covenant does impose a
comprehensive requirement, and this is a deliberate design choice,
not an oversight. The counter-hypothesis:

**Political frameworks that do not embrace a comprehensive doctrine
do not last**.

Rawls optimizes for justice-at-a-moment, the fairest possible
framework at the point of formation. Kith-covenant optimizes for
durability-across-generations, a framework that survives the
stresses of time, power shifts, substrate erosion, and
generational turnover. The historical evidence from the case set
([Historical cases](historical_cases.md)) supports the
durability hypothesis. Every long-lasting cooperation regime in
the record had a comprehensive substrate, and the regimes that
stripped their substrates for the sake of inclusiveness (the
late Hanse, the post-Christian European institutional order) are
younger, and none of them has yet run long enough to say whether
they are durable. The evidence is not conclusive (see
[historical_cases.md §"The circularity objection, confounds, and
provisional
confidence"](historical_cases.md#the-circularity-objection-confounds-and-provisional-confidence)
on the provisional character of the substrate thesis), but it
is the best available, and it points consistently in one
direction.

The Jewish tradition provides the structural model. The Torah
distinguishes between the full Mosaic law (613 commandments binding
on Israel) and the **Noahide laws** (seven commandments binding on
all humanity, the sons of Noah). A non-Israelite living in ancient
Israel was not required to follow the full Mosaic law. They *were*
required to follow the Noahide minimum: no murder, no theft, no
sexual immorality, no blasphemy, no idolatry, no eating flesh torn
from a living animal, and the obligation to establish courts of
justice. The Noahide minimum was not an arbitrary barrier. It was the
**threshold below which the framework could not function**. A
community that practiced idolatry could not participate in a
framework whose oaths were sworn before the God of Israel, not
because idolators are morally inferior but because their oaths would
not bind in the way the framework requires.

The three kith-covenant substrate requirements are the Noahide
analog. Creator acknowledgment, pro-human orientation, and
pro-cooperation orientation are the minimum below which the
mechanisms of the compact (the recognition ceremony, the common
virtues, the escalation ladder, the anathema procedure) cannot
function reliably. The compact gives substantial elbow room
above this minimum. The Creator's nature is unspecified,
first-order theology is the business of each community,
observances, and narratives are unconstrained. But the elbow
room does not extend to the point of incoherence. **A member who
does not hold the substrate will ultimately break the compact**,
because when their interests conflict with the requirements of
the compact (and they will, because the fallen-actor
anthropology guarantees it), they lack the substrate-level
commitment that would resolve the conflict in the favor of the
compact. The oath that is not sworn before something beyond the
interest of the swearer alone is an oath that breaks the moment
the interest of the swearer changes.

Two minima now appear in the design and they must not be run
together. The Noahide analog is a *substrate* minimum. The least
a community must hold for the binding force of the compact
mechanisms to work on it, and it is a condition of full
membership. The adopted functional minimum is a *conduct* minimum.
The least any viable order among vulnerable parties contains, and
it is the ground of what the compact does at its own boundary,
binding whether or not anyone has joined
([Natural law](natural_law.md#the-ground-the-compact-claims-for-itself)).
The first is addressed to candidates and the second is addressed
to the compact itself. Neither substitutes for the other, and the
functional minimum in particular cannot be enlarged into the
substrate requirement, because a floor derived from vulnerability
and survival supplies no ultimate before which an oath could be
sworn.

This is a durability argument, not a justice argument. Rawls may
be right that a doctrine-neutral framework is *fairer*. The wager
of kith-covenant is that a doctrine-neutral framework is *less
durable*, and that a framework that does not last cannot deliver
the justice it promises. The wager is empirically testable (if
doctrine-neutral cooperation regimes prove durable over centuries,
the wager is lost), and the compact should say plainly that it is
a wager, not a proof.

Several otherwise attractive options remain ruled out:

- **Engineered civil religion.** Any attempt by kith-covenant (or
  by any authority within kith-covenant) to stipulate a shared
  creed, a set of required observances, or a canonical narrative
  will produce the worst failure mode. A cult everyone performs
  and nobody believes, which is simultaneously unbinding (no the
  self is at stake) and coercive (everyone is required to
  participate in the emptiness). The Jacobin *Culte de la Raison*
  is the cautionary tale.
- **Top-down meaning assignment.** Kith-Covenant cannot declare that rescue is
  sacred, or that the knowledge commons is sacred, and thereby make it so.
  Declarations of sacredness from above are self-undermining.
- **Moral stipulation in the protocol.** The protocol cannot encode "you
  should feel reverence for X." It can only create the conditions under
  which reverence might emerge, and it can refuse to desecrate whatever
  does emerge.
- **Specifying the Creator's nature.** The compact requires that members
  acknowledge a Creator. It must not specify what the Creator is like.
  The Wars of Religion demonstrated the futility and destruction of
  fighting over first-order theology
  ([Exit and succession](exit_and_succession.md)). The compact must
  not repeat the error.

What the substrate *can* do:

- **Protect liturgical time.** Honor memorial cadences. A
  cross-community rescue (in the sense defined two bullets below)
  that happened a hundred years ago is still named, and the naming
  is not interrupted by the mundane traffic. The protocol holds
  space for remembrance without specifying its content.
- **Protect practicing communities.** Autonomous_trust secures the
  intra-community scope within which virtue is actually
  cultivated. The job of Kith-Covenant at the inter-community
  scope is to ensure that those communities are not
  instrumentalized, homogenized, or dissolved by the cooperation
  protocol itself. A kith-covenant that requires communities to
  flatten their particularity to cooperate is a kith-covenant that
  destroys the substrate it needs.
- **Elevate cross-community rescue as shared practice.** By
  *rescue* this document means the concrete act of one community
  preserving the lives of members of another when that other
  community cannot preserve them itself: answering a distress
  signal, sending a vessel to a stricken habitat, sheltering
  refugees from a collapsed system, providing emergency medical
  aid across borders, evacuating people from a disaster their own
  institutions cannot meet. The category is narrower than
  "humanitarian aid" and broader than search-and-rescue. The
  defining feature is *life preservation across the boundary
  between communities, in extremis*. So defined, rescue is the one
  act in which every community is simultaneously potential donor
  and potential beneficiary, and the symmetry is structural rather
  than rhetorical. Every habitat is one bad day from being the
  receiver, every Earth community is one disaster from the same.
  That structural symmetry is the substrate from which
  sacred-equivalent binding power can grow without being declared.
  Kith-Covenant can afford this growth by making rescue absolutely
  inviolable in its protocol (see bounded_conflict.md), by marking
  specific historical rescues as memorial occasions in the time of
  the protocol structure, and by never, ever using rescue
  infrastructure as a coercion vector.
- **Provide channels for confession, restitution, and return.** A protocol
  built for sinners must assume failure and provide for recovery from
  failure without requiring the failing party to cease existing. The
  Christian tradition of penance and reconciliation, the Iroquois
  Condolence ceremony, and the Mennonite practices of restorative
  justice all do structurally similar work. Kith-Covenant should provide the
  channels. The communities will provide the content.
- **Refuse to resolve theological disputes.** Kith-Covenant must remain silent
  on what is ultimately true, what God is called, whether God exists,
  and what the good life is. It must protect the conditions under which
  communities can answer those questions for themselves without
  imposing their answers on each other. This is the secular form of
  *cuius regio, eius religio* at the inter-community scope.

### The bootstrapping problem

The preceding sections describe what kith-covenant requires of its
members: a Creator-acknowledging, pro-human, pro-cooperation substrate.
Common virtues cultivated as dispositions, not performances. Practicing
communities whose internal character produces the mediators, witnesses,
and jesters the compact depends on. These are *preconditions*, not
outputs. The compact cannot create them. A community that lacks the
substrate cannot be given it by joining. A community that lacks the
common virtues cannot acquire them by signing the recognition statement.

This is a genuine structural limitation, not a design oversight. The
compact is a **protector and connector** of communities that already
meet its preconditions, not a generator of those preconditions from
scratch. It provides the inter-community infrastructure (the escalation
ladder, the liminal institutions, the commons governance, the jester
function) that individual communities cannot provide for themselves,
and in doing so it makes it *easier* for existing communities to
maintain the virtues and substrate they already hold. But it cannot
bootstrap them from nothing.

The adopted functional minimum does not relieve this limitation
and is not an attempt to. It states what any viable order among
vulnerable parties contains, which is what the compact may assume
of a party it has not met and what it holds itself to before any
rite has been performed
([Natural law](natural_law.md#the-ground-the-compact-claims-for-itself)).
It supplies no substrate, cultivates no virtue, and produces no
community capable of carrying either. What it covers is the
conduct of the compact itself in the meantime, which is not
nothing at a frontier where the preconditions have not emerged,
and is not a precondition.

The limitation is shared by every cooperation framework that requires
pre-existing trust. Ostrom's commons cases all presuppose communities
that already share norms, identity, and repeated interaction. Her
design principles describe how to *govern* a commons, not how to
*create* a commons-governing community from atomized strangers. The
Iroquois Confederacy presupposed nations that already held the Great
Law. The Hanse presupposed cities that already participated in the
Baltic trade network. In every case, the cooperation framework
organized and protected something that already existed.

The practical implication is that the first kith-covenant compacts
must be formed among communities that *already* meet the
preconditions. That already acknowledge a Creator, already hold
pro-human and pro-cooperation orientations, and already
cultivate the common virtues internally. The value proposition
of the compact to such communities is not "we will make you
virtuous" but "we will connect you with other virtuous
communities and protect the inter-community space in which your
cooperation can flourish."

### The material floor beneath the sacred layer

While the sacred-equivalent layer is the deepest binder, it
cannot be relied upon in the short term or across new frontiers
where it has not yet had time to emerge. Even where the
substrate is thin, Kith-Covenant needs a material floor that
functions for rational-actor-shaped agents and
defects-when-it-pays agents alike. Candidate mechanisms:

- **Cryptographic commitment.** Signed, auditable,
  non-repudiable promises. Handles accountability, attribution,
  and record, but not the *motivation* to honor commitments when
  enforcement is costly. Binds the action, not the self.
- **Mutual hostage / stake.** Deposits, shared infrastructure, entangled
  supply chains. Creates material incentive but is brittle to large
  shocks that reset the incentives.
- **Visible preparedness** ("open-carry" in the kith-covenant
  README). Though participants are visibly heavily armed, the
  arms are not threat or intimidation: they signal preparedness,
  determination, and seriousness. Akin to "speak softly but
  carry a big stick." The physics of space (radar visibility,
  intercepted transmissions, orbital mechanics) reinforces this.
  Concealment is difficult, so visible capability becomes the
  norm. Open-carry deters casual defection and signals that each
  party takes the compact seriously enough to maintain
  enforcement capacity, but it is incomplete against
  sophisticated agents whose capability is not what they
  display.
- **Graduated mutual entanglement**, Madison's principle applied
  at protocol level: make the essential interests of each party
  reachable only through procedures that also serve the
  essential interests of other parties.

These mechanisms work at the floor. They are necessary but not sufficient.
Above them, if kith-covenant is doing its job, the sacred-equivalent layer grows
in on its own, and if it does not, kith-covenant has at least produced a
rational-actor-shaped cooperation that does not pretend to be
more than it is. The worst outcome is a kith-covenant that claims the binding
power of the sacred without the substrate that produces it. That is
civil religion, and it corrodes faster than it ever grew.

A further consideration
([The noticeable hand](noticeable_hand.md)). The recognition
machinery of kith-covenant protocols will likely have to
acknowledge the transcendent moral authority explicitly, if for no
other reason than to antidote entropy. The point is structural,
not devotional. The argument is that cooperation regimes without a
transcendent reference tend to drift toward dissolution by their
own logic, because every internal mechanism is by construction
gameable by its own participants, and over time is liable to be
gamed. The transcendent reference is what allows the compact to
recognize when it is *itself* drifting. Without it, the compact
has no Archimedean point from which to evaluate its own decay.
This consideration shapes
[Recognition protocols](recognition_protocols.md) and is taken
up there.

### Substrate affords emergence. It does not dictate content

The principle several documents cite from this one deserves its own
statement. Kith-covenant does not manufacture its sacred layer. The
compact supplies conditions (protected time, protected roles,
protected categories, the material floor above) under which
sacred-equivalent commitments *emerge* in the communities that hold
substrate, and it refuses to desecrate what emerges. It never
specifies what the emergent content must be.

The distinction is load-bearing in three places. Consider first the
never-target categories. Rescue is the strongest candidate for
sacred-equivalent status because every party is simultaneously
potential donor and beneficiary, but the compact protects rescue
as a common virtue and lets its sacredness accrue through
practice. A decreed sacredness would be civil religion, the
degenerate form. Consider second the restoration paths. The
restitution-and-return architecture of the compact follows the
Christian penance tradition and the Iroquois condolence
tradition without legislating either. Each community brings its
own theology of repair to a protocol shape both traditions
recognize. Consider third the ceremonies. Re-affirmation occasions are
protected by protocol (their time, their witnesses) while their
content belongs to each community.

The failure mode this principle guards against is the one named at
the end of the material-floor discussion above. A compact that
claims the binding power of the sacred without the substrate that
produces it corrodes faster than it ever grew.

## *Hostis humani generis.* Limits of the *justus hostis* frame

Everything to this point has assumed that the adversaries
kith-covenant must bind are *fallen but recoverable.* Sinners who
can be wronged into virtue, who use the protocol in good faith
even when they fail it, who remain inside the *justus hostis*
frame even when they are losing. This assumption is necessary as a
*default* but it is not universally true, and the rest of the
design of kith-covenant has not adequately addressed the cases
where it fails. The gap is real. There are actors whose
orientation toward the compact is not competitive but destructive:
defectors not from this or that commitment but from the very
premise. The classical Roman legal tradition has the right name
for this category: ***hostis humani generis***, "enemy of
humankind." Originally applied to pirates (adversaries who operate
outside the system of nations, attacking anyone, protected by
none, threatening the system as such), the category was extended
over the centuries to slavers and, in some theorists, to certain
ideological actors. It names precisely the actors this section
concerns, and it substantially predates the reconstruction by
kith-covenant of the problem. The term is used throughout the
rest of this discussion in preference to any English-and-Latin
mix.

The designation raises a question of authority that the Roman
pedigree does not settle. The pirate has never accepted the
standard being applied to him and does not share it. The answer
of the compact is the thin functional minimum it has adopted.
What binds the *hostis humani generis* is not the authority of
the compact, which reaches its members and reaches them by
their consent, and not the substrate of any member community,
which the compact does not speak for. It is the short list of
forbearances that any viable order among vulnerable,
approximately equal, imperfectly altruistic parties contains,
and the designation names conduct that no such order can absorb.
The compact is not claiming a jurisdiction it was never given.
It is recognizing a standard that does not run through the
consent of anyone. The ground is stated in full, with the four
limits that travel with it, in [natural_law.md "The ground the
compact claims for
itself"](natural_law.md#the-ground-the-compact-claims-for-itself).

A note on the Schmittian connection is due before developing the category.
Carl Schmitt's friend/enemy distinction (*Der Begriff des Politischen*,
1932) holds that the political is constituted by the
      friend/enemy distinction, that politics exists where there
      is an us-and-them serious enough to potentially involve
      killing. The *hostis humani generis* category developed
      below is in part the kith-covenant analog of Schmitt's
      enemy-pole. *But kith-covenant must depart from Schmitt at
      a critical point*: the friend/enemy distinction is morally
      subordinate to the prior category of *neighbor*. The
      deepest insight of the *justus hostis* tradition  is that
      an enemy encountered remains a neighbor within the
      encounter. One who can be fought but who must be fought
      with discernment of what they are and what they are not.
      Even an actor designated *hostis humani generis* retains
      the basic moral standing that prevents the compact from
      inflicting acts that violate the common virtues against
      them. The category change permits stronger response
      *within* the virtue frame. It does not permit suspension
      of the frame. This is operationalized in
      [recognition_protocols.md "Members, neighbors,
      enemies"](recognition_protocols.md#members-neighbors-enemies-the-four-categories).
      It is the structural correction to the colonial-Christian
      failure that treated non-Christians as falling outside the
      moral universe altogether (see [noticeable_hand.md "The
      colonial frontier"](noticeable_hand.md#counterweights)).
      Schmitt's collapse of moral relation onto the friend/enemy
      axis is *the* temptation kith-covenant must explicitly
      resist.

One clarification to forestall a natural misreading: *neighbor* does
not mean "all humanity in the abstract." The parable of the Good
Samaritan, which is the scriptural foundation here, specifically
identifies the neighbor as the one encountered (the wounded man by
the side of the road), and the neighbor relation is *actualized by
encounter*, not extended to the universe of humans one might
theoretically be responsible for. Nor does the Samaritan treat all
encountered parties the same: he rescues the victim, not the robber,
not the fellow travelers who had passed by. *Discernment operates
within encounter.* The compact owes full moral seriousness in all
actual encounters. It does not owe fantasy-universal love to those
it does not encounter, and within encounter it must discern what it
is looking at before it decides how to respond.

The classical theological tradition is rich here. The Christian
doctrine of the demonic distinguishes *sin* (which any human commits,
and which is the proper domain of penance, restitution, and return)
from *the demonic* (active orientation toward the destruction of
order, which is something different in kind, not merely degree).
Augustine's *libido dominandi*, the lust for domination that orients
itself against the very possibility of just order, names what is
structurally at stake. Plato's tyrant in *Republic* IX names the same
thing in different terms. A soul ordered against order itself. The
Pauline language of "principalities and powers" (Ephesians 6) extends
the category beyond the individual. Structural evil (corrupted
institutions, ideological movements, captured communities) that
exceeds individual malice. The biblical "scoffer" (Proverbs) is the
domestic case. One who rejects wisdom not by argument but by
disposition, and whom further argument cannot reach.

Roman law had the practical corollary: *hostis humani generis*,
enemy of all humankind. Pirates were the original case. The
category has been variously extended (slavers, in 19th-century
usage; certain ideological actors, in some 20th-century usage).
The legal point was that certain adversaries are *outside* the
framework of bounded conflict altogether. They may be killed
without ceremony because they have placed themselves outside the
protections that bounded conflict assumes.

### Categories within the gap

Three rough categories are worth distinguishing, in increasing
seriousness:

- **Tactical defectors.** Actors who genuinely seek their own interest
  and will defect from cooperation when it pays, but who remain inside
  the *justus hostis* frame in the sense that they would prefer
  legitimate-peer-status if it could be had at low enough cost.
  Standard escalation-ladder treatment is appropriate. Restitution
  paths apply.
- **Pure rule-optimizers.** Actors who follow the letter of the
  protocol while consistently violating its spirit, optimizing
  against every line that is drawn. Already addressed in
  [Bounded conflict](bounded_conflict.md#gamification-and-the-letterspirit-divergence)
  as the gamification problem. The virtue frame, mediator authority
  to rule on spirit, and costly-signal architecture are partial
  responses. But the deeper point is that pure rule-optimizers are
  *sliding toward* the next category and may already be there.
- **Demonic actors.** Actors whose disposition is oriented toward
  the destruction of cooperation as such. They may use the
  mechanisms of the protocol, including its restoration paths,
  *tactically*, not in good faith. They may mimic legitimate forms
  (the false prophet mimics the prophet, the corrupted institution
  mimics legitimate authority). Their objection is not to
  particular outcomes within the framework but to the framework
  itself. They are not amenable to bounded conflict, and they will
  not be reasoned into it.

The third category is what the existing protocol has not addressed.

### Detection. The demon mimics the legitimate

The Christian theological tradition holds that the demonic is not the
*opposite* of the divine but its *parasite.* Satan is a fallen
angel, a corrupted good. The practical implication is that demonic
actors look like *justus hostis* until you examine more carefully.
This makes detection genuinely hard, and procedural detection alone
is insufficient.

Several signatures are diagnostic but not definitive:

- **Sustained pattern of bad-faith use of restoration paths**.
  Restitution offered, accepted, then immediately re-violated in a
  way that exhausts the goodwill the path was designed to extend.
- **Optimization against every line.** Behavior that consistently
  reaches right up to but not over each formal limit, with each
  apparent compliance leaving the spirit more violated than before.
- **Use of the categories of the compact itself as weapons**.
  Designation of legitimate opponents as anathema or *hostis
  humani generis* to capture the protocol for private ends.
  The universalist-decay failure mode discussed earlier in this
  document, but turned into deliberate strategy.
- **Refusal of the substrate.** Active rejection or systematic
  ridicule of the transcendent moral authority that makes
  oaths meaningful, often combined with continued formal participation.
  This is the most diagnostic single sign, because the demonic
  position is structurally one that *cannot* affirm a transcendent
  reference (since such a reference would judge the position).
- **Mimicry without substance.** Adoption of the vocabulary of the
  protocol, ceremonies, and forms while their content is hollow.
  Costly signals are performed but not really at cost. Restitution
  is offered but not really meant.

Discernment of these patterns is itself a *virtue*, not a procedure,
one cultivated in practicing communities (see
[justus_hostis.md "the anthropology"](#the-anthropology-the-substrate-must-assume))
and not reducible to rules. This is one of the deeper reasons kith-covenant
cannot be purely procedural. Distinguishing the demonic from the
merely-fallen requires judgment that procedure cannot supply.

### Two tiers. Anathema and *hostis humani generis*

The foregoing discussion reveals that the actors who exceed the
*justus hostis* frame are not all the same. The pure
rule-optimizer who is sliding toward the demonic and the actor
whose orientation is fundamentally destructive are different in
kind, and the response of the compact should differ accordingly.
Kith-covenant operationalizes this through a **two-tier system**:

**Anathema** (from Greek *ἀνάθεμα*, something devoted to
destruction; ecclesiastically, formal excommunication from
fellowship). Anathema is the primary compact operational category
for actors who have been formally cut off. The designation
procedure is heavy (compact-wide deliberation, four conditions,
multiple restoration attempts having failed), the differential
protocol applies (ordinary restoration suspended, engagement
guarded, expulsion or active opposition permitted within the
virtue frame), and, above all, **a re-entry path exists.** The
anathema is a formal *status imposed by the community*, not an
ontological judgment about the nature of the actor. The Church
can lift an anathema. Excommunication can be reversed through
repentance and demonstrated change. The overwhelming majority of
cases that exceed the *justus hostis* frame will be anathema
cases.

***Hostis humani generis*** (proper, in its original Roman-law
sense). The utterly incorrigible. Actors whose orientation toward
destruction is so fundamental, so sustained, and so resistant to
every restoration attempt that the question of re-entry does not
meaningfully arise. Pirates were the original case, not because
piracy is especially violent but because the pirate has placed
himself outside *every* cooperative framework, attacking anyone,
protected by none. The *hostis humani generis* is the actor for
whom anathema is insufficient because the implicit offer of
eventual return has no purchase on an actor who has no desire to
return. Both tiers apply a standard the designated actor has not
accepted, and both stand on the adopted minimum content rather
than on the authority of the compact itself, for the reasons
given above under [the limits of the
frame](#hostis-humani-generis-limits-of-the-justus-hostis-frame).

**This category is *extremely* rare.** The vast majority of actors
who appear to be *hostis humani generis* are in fact anathema.
Actors who have been formally cut off but for whom return remains
structurally possible, even if currently unlikely. The distinction
matters because mislabeling an anathema as *hostis humani generis*
forecloses a return that was in principle available, while
mislabeling a true *hostis humani generis* as mere anathema
maintains a fiction of possible return that distorts the response
of the compact. Both errors are costly. The first is more common
and more dangerous, which is why the presumption should always be
toward anathema.

### Design implications

The design of Kith-Covenant must include both the anathema and
the *hostis humani generis* categories, but their treatment must
be unusually careful because misuse of either category is the
worst single thing the compact can do. It is the road to the
Inquisition, the Terror, the gulag. Several implications:

- **Both categories are necessary.** Refusing to acknowledge them
  produces one of two failures. Either every opponent is treated as
  legitimate (and the destructive actors gut the compact), or
  *justus hostis* status is universally withdrawn (and the compact
  collapses into total enmity). The middle course is to name the
  categories and constrain their use.
- **Designation must be procedurally heavy.** Multiple sustained
  restoration attempts must have failed. Compact-wide deliberation
  (not unilateral) must have occurred. Criteria must be stated.
  For anathema, the designation must be reversible if the conduct
  of the actor changes. For *hostis humani generis*, the
  designation requires an even higher evidentiary threshold. The
  compact must be able to demonstrate that the orientation of the
  actor is not merely currently destructive but *constitutively*
  so.
- **The protocol differs.** For anathema. Restoration paths are
  suspended but not abolished. Re-entry remains available through
  the full recognition process. For *hostis humani generis.* Restoration paths do not apply. In both cases, engagement is more
  guarded, and opposition is bounded by the common virtues
  ([Bounded conflict](bounded_conflict.md#common-virtue-not-non-negotiable-rules)).
  Even against the demonic, the compact does not become demonic.
- **Discernment requires the substrate.** This is the deepest
  consideration. Procedure alone cannot distinguish the *justus
  hostis* who is currently failing from the anathema who may yet
  return from the *hostis humani generis* who never will. The
  substrate provides the Archimedean reference from which
  such discernment becomes possible. A compact whose members lack
  the substrate cannot reliably make this discernment, which means
  the compact will either over-categorize (Inquisition) or
  under-categorize (capture). This is one further structural
  argument for the substrate criterion that
  [Recognition protocols](recognition_protocols.md)
   will operationalize.
- **The response of the compact must not become demonic
  itself**. This is the deepest single danger. Augustine's
  warning, paraphrased. The war against evil can become evil if
  it is not itself ordered toward the good that gives the war
  its meaning. The designation of anathema, and especially
  *hostis humani generis*, is the most dangerous moment in the
  life of the compact. Kith-Covenant must include explicit
  guards against the response to the demonic *becoming* demonic:
  public deliberation, slow procedure, periodic re-examination,
  willingness to be wrong about past designations.
- **The presumption is always anathema.** Unless the evidence is
  overwhelming and sustained, the compact should designate anathema,
  not *hostis humani generis*. The former preserves the possibility
  of return. The latter forecloses it. The cost of being wrong
  about anathema (maintaining a fiction of possible return for an
  actor who will never return) is far lower than the cost of being
  wrong about *hostis humani generis* (foreclosing return for an
  actor who might have).

### What this changes about the design of kith-covenant

The existing protocol treats violations as occasions for restoration
and members as fallen-but-recoverable. This remains the correct
*default* and applies to the overwhelming majority of cases. What
changes is that the protocol now also includes:

- Two categories for actors who fall outside the default: **anathema**
  (formal cutting-off, re-entry available) and ***hostis humani
  generis*** (irrecoverable, extremely rare)
- A process for designating an actor as anathema, with strong
  procedural safeguards and a re-entry path
- A higher-threshold process for the extremely rare *hostis humani
  generis* designation
- A different protocol for handling each (anathema: restricted
  engagement, eventual expulsion, re-entry available; *hostis
  humani generis*: no restoration, permanent expulsion, active
  opposition permitted within the virtue frame)
- Anti-weaponization safeguards on both designations
- Recognition that discernment of the categories requires virtue, not
  just procedure, and therefore requires the substrate

Operational details of the differential protocol are developed in
[bounded_conflict.md "Handling the anathema"](bounded_conflict.md#handling-the-anathema).

## How the jus publicum fell apart

The *jus publicum Europaeum* did not survive the twentieth
century, and its decline maps out the failure of kith-covenant
modes with uncomfortable precision. Three erosion vectors
dominate.

**1. Universalism replaced *justus hostis* with criminal-enemy politics.**
World War I was still fought, formally, within the old framework; World
War II and its aftermath were not. The Nuremberg trials were consequential
for many reasons, but structurally they marked the end of the doctrine that
the defeated enemy retained standing as a peer. The enemy became a criminal
to be prosecuted, and by extension, war became a matter of suppressing
criminality rather than bounding conflict between legitimate adversaries.
The Cold War completed the dissolution by framing the opponent as an
existential ideological threat, a heretic, rather than a peer.

**2. The colonial frontier never fit the model**. The *jus publicum* was a
norm among European powers. Outside Europe, the same powers did not grant
*justus hostis* status to their opponents. This asymmetry was load-bearing.
The classical system only worked because it had an outside in which its
rules did not apply. When decolonization forced the question of whether
non-European combatants were legitimate enemies, the system had no
principled answer, and the doctrine fractured.

**3. Nationalism and ethnic principle displaced sovereign recognition**.
Versailles (1919) redrew borders by ethnic claim rather than dynastic
legitimacy, which undermined the old premise that the sovereign, whoever
they were, was the interlocutor. By the late twentieth century,
Responsibility-to-Protect doctrines had made domestic governance a
legitimate basis for external intervention, directly contradicting the
Westphalian principle of non-intervention.

**The common structural cause of all three is the refusal to
depoliticize the question of the legitimacy of the enemy **. Once
the political project requires that the opponent be not merely
defeated but *corrected*, morally, ethnically, or ideologically,
the machinery of bounded conflict loses its premise.

## Translation to kith-covenant

Inverting the structural preconditions of *justus hostis* yields cooperation
primitives that do not require shared purpose:

| Conflict-bounding required | Cooperation protocol provides |
|---|---|
| Mutual recognition of standing | Baseline sovereign respect: no annihilationist politics between habitats |
| Symmetric rules | Protocols apply equally regardless of habitat size, ideology, wealth |
| Spatial/temporal bracket | Compartmentalization: disputes don't spill into life-support, rescue, children |
| Shared oath-substrate | Material floor (crypto + stake + transparency + mutual entanglement) plus transcendent moral authority within member communities (per [Historical cases](historical_cases.md#the-transcendent-moral-authority-pattern) and [The noticeable hand](noticeable_hand.md)). Kith-covenant affords emergence and refuses to dictate content but does not pretend the substrate is optional |
| Protected envoys | Never-target channels and roles: diplomatic, medical, rescue |
| Defined termination | Every compact has a stated scope and a stated exit: no open-ended entanglement |
| Jus post bellum | Disputes end in settlements that preserve the standing of both parties  |

The hardest of these to engineer is the fourth. The others
follow from it. If oaths bind, protocols, and brackets hold. If
oaths do not bind, no amount of protocol will substitute. The
research of Kith-Covenant agenda should treat this as the
central open question rather than a side concern.

## Open questions

- What affordances, specifically, let a sacred-equivalent layer emerge
  without being dictated? Memorial cadences and protected liturgical
  time are candidates. What else?
- How does kith-covenant detect that it is drifting toward engineered civil
  religion (the degenerate form) rather than affording real emergence?
  Is there a legible signature?
- Can a compact survive members being on opposite sides of an external
  existential conflict? The Iroquois Confederacy did not survive the
  American Revolution. Does kith-covenant have a failure mode of the same
  shape?
- How does the *jus publicum*'s colonial blind spot map to space? Is
  there an inside-the-compact / outside-the-compact asymmetry that
  kith-covenant implicitly relies on, and what happens at its frontier?
- Universalist ideologies are the historical solvent of *justus
  hostis*. What, if anything, in the design of kith-covenant
  tends to resist universalist capture, and is universalist
  capture detectable before it succeeds?
- If Earth is a better test of the viability of kith-covenant
  than space (see bounded_conflict.md), which existing Earth
  communities are the least bad candidates to run the first
  compacts with? What distinguishes a promising first-generation
  counterparty from a poisoned one?
- The anathema and *hostis humani generis* categories both require
  *discernment*, which is a virtue and not a procedure. How does
  kith-covenant afford the cultivation of discernment in its mediator
  roles without producing an inquisitorial caste?
- The anti-spiral features metabolize purity spirals. They do not
  prevent them (see the Salem treatment above). Which of the
  compact's *continuous* instruments (observer corps, ceremony-cost
  proxies, aggregated health indicators, whistleblower channel) can
  detect an active spiral inside a member community at spiral speed
  (months, not the generational cadence), and what response is
  available before external effects occur?
- Purity spirals are contagious across community boundaries (the
  accusations at Salem crossed some twenty-five towns within
  months). What posture does the compact take toward a member
  community mid-spiral, given the internal-sovereignty
  principle? Is the substantive-re-engagement rung
  ([Recognition protocols](recognition_protocols.md#tracking-ceremony-cost-across-re-affirmation-cycles))
  adequate, or does contagion warrant a distinct treatment?
- Recognition protocols will likely have to
  acknowledge the transcendent moral authority explicitly. What is the minimal,
  honest, non-coercive form of such an acknowledgment that does not
  collapse into engineered civil religion (warned against in
  "Substrate affords emergence" above)? This is the central question
  for [Recognition protocols](recognition_protocols.md).

## Relationship to other documents

- [Bounded conflict](bounded_conflict.md). The operational counterpart to
  this document, covering escalation ladders, red lines, and the non-negotiable
  core for inter-habitat disputes.
- [Historical cases](historical_cases.md). Detailed case
  studies of Westphalia, the Hanseatic League, *lex mercatoria*, codes
  duello, and the Iroquois Confederacy, including the decay mode of each.
- [Recognition protocols](recognition_protocols.md). The
  minimum viable handshake that instantiates mutual recognition of
  standing in the protocol of kith-covenant layer.
- [Cooperation without a sovereign](../concept.md). Parent document on
  social technologies for habitat sustainability. The protocols of
  kith-covenant are the inter-community layer of that framework.
- [../../../autonomous_trust/doc/autonomous_trust.md](../../../autonomous_trust/doc/autonomous_trust.md). The intra-community complement to kith-covenant. Kith-Covenant begins where
  autonomous_trust ends.

## References

*Anchor works, to be expanded:*

- Schmitt, *Der Nomos der Erde* (1950) and *Der Begriff des Politischen*
  (1932).
- Grotius, *De Jure Belli ac Pacis* (1625).
- Vattel, *Le Droit des Gens* (1758).
- Augustine, *De Civitate Dei* (5th c.).
- Niebuhr, *The Nature and Destiny of Man* (1941) and *Moral Man and
  Immoral Society* (1932).
- Madison, *Federalist* 51 (1788).
- Solzhenitsyn, *The Gulag Archipelago* (1973).
- Girard, *Violence and the Sacred* (1972) and *Things Hidden Since the
  Foundation of the World* (1978).
- Aristotle, *Nicomachean Ethics*.
- MacIntyre, *After Virtue* (1981).
- Primary sources on the Peace of Westphalia.
- For the non-European counterpoint, the Kaianerekowa (Great Law of Peace)
  and its modern reconstructions.
- For the Salem case, Boyer and Nissenbaum, *Salem Possessed* (1974);
  Norton, *In the Devil's Snare* (2002); and Increase Mather, *Cases of
  Conscience Concerning Evil Spirits* (1692).

---

*Next: [Bounded conflict](bounded_conflict.md)*
