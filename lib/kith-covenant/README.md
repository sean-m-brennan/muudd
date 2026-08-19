# Kith-Covenant

**Kith-Covenant** (KC) is a protocol for cooperation among communities that do
not share ends and cannot appeal to any authority standing above all of them. It
sits on top of [ethne](../ethne/README.md), the polity tier that turns
cooperating agents into a coherent chartered community, which may in turn be fed
by [autonomous-trust](../autonomous_trust/README.md). That substrate handles
anonymous reputation *inside* a single community. This tier handles the other
scope: known identity, tied to a specific named human whose social standing is
at stake, binding one community to another across a boundary neither side rules.

The problem is the oldest one in governance. The earliest surviving parity
treaty between two foreign powers, Hittite and Egyptian in 1259 BCE, already has
its shape. How do you make an agreement hold between sides with different gods,
different languages, and different loyalties?

The record splits sharply. Bounded functional cooperation converges across
cultures without much trouble, and always has. Commerce, maritime rescue, envoy
protection, and the treatment of wounded combatants all work cross-culturally on
a thin shared grammar. Although the thin cases are encouraging, full compacts
have a much worse record. Those are the ones governing a broad slice of
community life rather than one function, and they fail in two distinct ways.
Without a foundation above the parties, oaths do not bind and short-term
interest overrides them. With a foundation that becomes the object of devotion
for the compact itself, the system collapses into loyalty cascades that end in
purges.

The answer rests on two rules and a filter.

1. **Each community anchors to whatever it holds above itself.** The compact
   stays neutral on content. A Christian community names the Trinity, a
   Muslim community names Allah, a Haudenosaunee community names
   *Shöñgwa'ya'di:söñ*, and a Buddhist community names the Dharma. The compact
   requires that each party *have* such a ground, not that they share one.
2. **The compact itself never becomes the ultimate.** An oath sworn before an
   authority the swearer treats as exceeding the compact costs something real in
   a dimension that compact-internal enforcement cannot reach.
3. **A community qualifies for full membership only if its own tradition carries
   the five anti-spiral features** that historically block the loyalty-cascade
   failure: everyone can fail including the most virtuous, virtue is received
   rather than earned, victim-selection as communal hygiene is refused, moral
   obligation extends to strangers, and the political community is not the final
   thing.

Recognition runs in two layers, because the historical record says joint creedal
ceremonies across unshared substrates do not work. Firstly, the intra-community
layer is where the real binding happens, with each community performing its own
authorization rite in its own tradition. Second, the inter-community layer is a
thin procedural grammar in the tradition of Kadesh, merchant law, and the Geneva
framework, holding credentialed envoys, vouching by existing members, and an
audit commons recording conduct over time.

## Quick start

The project builds against a per-component **conda** env, consistent with the
rest of sustainable-space. `environment.yaml` pins `python==3.13.*`, `rust`,
`rust-src`, `maturin`, and `pytest` from conda-forge.

```bash
cd lib/muudd/lib/kith-covenant
conda env create -f environment.yaml     # first time only
conda activate kith-covenant
./run-tests.sh                           # cargo tests + Rust demo + PyO3 build + pytest + Python demo
```

To run the pieces individually:

```bash
# Rust core
cargo test --workspace --exclude kc-pyo3     # 379 unit and property tests
cargo run -p two_communities_demo            # end-to-end ceremony with audit trail

# Python bindings (conda env active)
maturin develop                              # builds kc-pyo3 into kith_covenant._core
python src/python/examples/two_communities_demo.py
pytest src/python/tests/
```

The bindings expose a thin Pythonic surface; see
`src/python/kith_covenant/ceremony.py`. The autonomous-trust seam
(`interface.py`, `policy.py`) imports cleanly whether or not the bindings are
compiled.

## Documentation

Kith-Covenant is Part IV of a continuous narrative that runs from the physical
mesh layer up through the trust and polity tiers below it. The whole chain is
entered from [the MUUDD README](../../README.md). The ten chapters here are the
KC section of it, and each links to the next at its foot.

1. [Cooperation without a sovereign](doc/concept.md): the ancient problem, the
   inversion argument, and the two rules with their filter.
2. [The ground each community stands on](doc/substrate.md): the substrate
   finding, the five anti-spiral features, and what they actually buy.
3. [Two layers of recognition](doc/recognition.md): the intra rite, the inter
   grammar, and the admissions gate.
4. [Four categories of standing](doc/standing.md): member, *justus hostis*,
   anathema, non-member encountered, and what an outsider is owed.
5. [When communities disagree](doc/disputes.md): the ladder, the mediator pool,
   sealed decisions, and the third place.
6. [The five commons](doc/commons.md): Ostrom adapted for light-lag, sealed
   usage, graduated sanctions, and nesting.
7. [Protected dissent](doc/jester.md): the observer corps, attributed reports,
   and the whistleblower channel.
8. [Machines as delegates](doc/machines.md): the reduction claim, the authority
   envelope, and where detection stops.
9. [Capture resistance](doc/capture.md): the seven moves, the failure with no
   adversary, and the harness.
10. [What only humans can do](doc/limits.md): the split, the structural limit,
    and the seven holes.

The reference corpus is appendix D of the same chain, indexed at
[doc/kith_covenant.md](doc/kith_covenant.md) and beginning at [justus
hostis](doc/background/justus_hostis.md). For everything in one standalone
document, read the [whitepaper](doc/whitepaper/whitepaper.md).

Implementation detail, being the crates and what each one holds, is in
[doc/crates.md](doc/crates.md). Outstanding scope is in [`TODO.md`](TODO.md).

## Relation to the tiers below

Autonomous-trust sits underneath Kith-Covenant, and nothing here replaces it.
Its scope is intra-community, covering anonymous identity, peer admission by
vote, and reputation consensus among peers who share one trust boundary. That
anthropology is right for the inside of a working community. Its *scope* is
wrong for what happens across community lines, where anonymity is the failure
mode rather than the protection. What has to cross a boundary is not a
reputation score but a binding that couples a specific named machine to a
specific named human whose community standing rides on the conduct of that
machine.

Between the two sits [ethne](../ethne/README.md), the polity tier. The trust
substrate produces *emergent* cooperation among nodes, and not a bounded,
coherent, persistent community, which is the party this tier already presumes as
its atomic actor. Ethne turns the cooperation into that party by *ratifying* the
emergent boundary into a signed charter rather than imposing one.

Kith-Covenant depends on neither. Ethne speaks to it only through the public
vocabulary this tier already has. It hands over a standard `CommunityIdentity`,
surfaces a coherence breakdown as the existing `Concern`, publishes a
`MemberRoll` that feeds the whistleblower ring, and presents a community split
as new communities plus the old one going quiet. This tier sees communities and
evidence. It never learns the lower tiers exist.

Machines participate only as delegates. A machine never originates substrate; it
inherits it from one named human guardian, who inherits it from a community.
Every machine-to-machine exchange reduces to an exchange between the two
guardians. Every machine act stays inside an envelope the guardians
pre-authorized, and the guardians answer for what happens inside it. There is no
machine-native legitimacy, and no machine-to-machine cooperation without a human
on each end.

## The ten components

The Rust implementation covers all ten components of the decomposition in
[`doc/architecture/protocol-components.md`](doc/architecture/protocol-components.md).
Per-component detail is in the sections of that spec, and crate-level detail is
in [`doc/crates.md`](doc/crates.md).

| # | Component | What it provides |
|---|---|---|
| 1 | Identity and standing | Ed25519 self-certifying `did:key`, delegations, witnesses |
| 2 | Recognition ceremony | The two-layer split, plus admissions integrity: probation withholds weight, an admitting quorum of sitting non-probationary members, a sponsor per entrant, and term-limited rotated pool-share-capped examining seats |
| 3 | Archive and commons | In-memory, content-addressed, JSON dump and load, cryptographic verification |
| 4 | Mediation | The mediator pool with term, rotation and pool-share caps; dispute intake, argument and deliberation records, recusal, protecting powers, training lineage, panel consent, and sealed reasoning under community custody |
| 5 | Escalation and designation | The ladder through to expulsion, the four-condition anathema test, the covenant-facing floor on the expedited path, restitution and verification, re-entry, and the recorded conclusion of a mandated re-examination |
| 6 | Reputation bridges | Lookup and standing, the capture-signal detectors, and the jester lenses, including an erosion alert with sixteen signals, seven of them absolute and one graded against the compact's own recited object |
| 7 | Commons governance | The five compact-level commons, usage accounting public or sealed, the graduated ladder from monitoring to exclusion, the commons equivalent of anathema, and the inter-compact level with its subsidiarity limit |
| 8 | Jester and observer function | Observer-corps registry with enforced caps, attributed reports with an irrevocable right of response, and a whistleblower channel whose source proves insider standing by ring signature over the log preceding each filing |
| 9 | Enforcement and non-ultimacy | The compact-incapable list as a genesis-immutable artifact with asymmetric amendment, the structured-fast cadence, interim injunctions with capability gating, and graded erosion alerting |
| 10 | Bootstrapping and test harness | Fourteen scenarios scored against falsification criteria stated in advance, run over a parameterized space with seeded search, directed sweeps, and shrinking |

Three things the implementation deliberately is not.

*Not replicated, not transported.* The archive is single-host and in-memory, and
the protocol is specified without reference to how records travel between hosts.

*Not in-person.* The intra-rite data model is software-only. The research corpus
commits to bodily co-presence for the actual rite, and the code encodes the
shape of the protocol rather than the embodied practice.

*Ed25519-only.* The algorithm enum is a single-variant structure kept open for a
future non-lattice post-quantum addition if one is accepted. NIST lattice
post-quantum signatures are excluded pending community confidence in their
provenance.

## Development setup

Dependencies come from conda rather than from pip or venv. `environment.yaml`
provides `cargo` and `rustc` through the conda `rust` package, plus `maturin`
and `pytest`, and `run-tests.sh` errors early with an instruction if any is
missing.

The Rust workspace is fifteen crates plus a demo, and
[`doc/crates.md`](doc/crates.md) covers what each one holds. The Python package
under `src/python/kith_covenant/` wraps the bindings and carries the
autonomous-trust seam, which imports with or without the compiled extension.

## License

Apache License 2.0.
