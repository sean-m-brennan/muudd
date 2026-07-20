# Kith-Covenant

**Kith-Covenant** (KC) is a protocol for cooperation among communities that do not share ends and cannot appeal to any authority standing above all of them. It sits on top of [autonomous-trust](../autonomous_trust/README.md), which handles anonymous reputation *inside* a single community. KC handles the other scope: known identity, tied to a specific named human whose social standing is at stake, binding one community to another across a boundary neither side rules.

The problem is the oldest one in governance. The earliest surviving parity treaty between two foreign powers (Hittite and Egyptian, 1259 BCE) already has its shape: how do you make an agreement hold between sides with different gods, different languages, and different loyalties? Bounded functional cooperation converges across cultures without much trouble, and always has. Commerce, maritime rescue, envoy protection, and wounded-combatant treatment work cross-culturally on a thin shared grammar. Full compacts, the ones governing a broad slice of community life rather than one function, have a much worse record, and they fail in two distinct ways. Without a foundation above the parties, oaths do not bind and short-term interest overrides them. With a foundation that becomes the compact's own object of devotion, the system collapses into loyalty cascades that end in purges. Robespierre's Terror and the Cultural Revolution are not accidents; they are the predictable failure mode of virtue-oriented communities that lack specific structural brakes. The Salem witch trials are the harder case: a community that held the brakes in doctrine, saw them defeated in practice, and then had its spiral terminated from within, followed by public repentance and restitution.

KC's answer rests on two rules and a filter:

1. **Each community anchors to whatever it holds above itself.** The compact stays neutral on the content. A Christian community names the Trinity, a Muslim community names Allah, a Haudenosaunee community names *Shöñgwa'ya'di:söñ*, a Buddhist community names the Dharma. The compact requires that each party *have* such a ground, not that they share one.
2. **The compact itself never becomes the ultimate.** An oath sworn before an authority the swearer treats as exceeding the compact costs something real in a dimension compact-internal enforcement cannot reach. The compact is structurally forbidden the role of that authority.
3. **A community qualifies for full membership only if its own tradition carries the five anti-spiral features** that historically block the loyalty-cascade failure: a universal fallen-actor anthropology (everyone can fail, including the most virtuous), grace over merit (virtue is received, not a status to compete for), a scapegoat-disarming mechanism, enemy-love as an operational norm, and non-ultimacy of the political community. These are present across mature religious traditions and absent in the ideological systems (Jacobin, Maoist, pure-tribalist) that produced the purges.

Recognition runs in two layers, because the historical record says joint creedal ceremonies across unshared substrates do not work. The **intra-community layer** is where the real binding happens: each community performs its own authorization rite, in its own tradition, on the envoys and machines it sends out. Speaking a commitment in front of one's own people is costly, and it works in a language the community can read for sincerity. The **inter-community layer** is a thin procedural grammar in the tradition of Kadesh, merchant law, and the Geneva framework: credentialed envoys, vouching by existing members, and an audit commons recording conduct over time. Each side keeps its own substrate; the compact speaks a thin form every anti-spiral-competent substrate can render into.

## Quick start

The project builds against a per-component **conda** env, consistent with the rest of sustainable-space. `environment.yaml` pins `python==3.13.*`, `rust`, `rust-src`, `maturin`, and `pytest` from conda-forge.

```bash
cd lib/muudd/lib/kith-covenant
conda env create -f environment.yaml     # first time only
conda activate kith-covenant
./run-tests.sh                           # cargo tests + Rust demo + PyO3 build + pytest + Python demo
```

To run the pieces individually:

```bash
# Rust core
cargo test --workspace --exclude kc-pyo3     # 85 unit and property tests
cargo run -p two_communities_demo            # end-to-end ceremony with audit trail

# Python bindings (conda env active)
maturin develop                              # builds kc-pyo3 into kith_covenant._core
python src/python/examples/two_communities_demo.py
pytest src/python/tests/
```

The bindings expose a thin Pythonic surface; see `src/python/kith_covenant/ceremony.py`. The autonomous-trust seam (`interface.py`, `policy.py`) imports cleanly whether or not the bindings are compiled.

## Start here

New to Kith-Covenant? Read these in order. Each goes one level deeper than the last.

1. **[Summary](doc/summary.md)**: the problem, the answer, and the technical shape in about 500 plain-language words.
2. **[Orientation](doc/orientation.md)**: a guided walk through the whole design. Each section names a facet, says what it is for, and points to the background document that argues it. This is the table of contents with reasoning attached.
3. **[Overview](doc/overview.md)**: the full standalone argument, with the four threads (inversion, substrate, fallen-actor anthropology, mutual vulnerability), the two-layer recognition model, the machine layer, and the seven open holes named with their disposition.

For everything in one document (executive summary, argument, software architecture, historical appendices, and bibliography), read the [whitepaper](doc/whitepaper/whitepaper.md).

## Relation to autonomous-trust

Autonomous-trust sits underneath Kith-Covenant, and nothing in KC replaces it. AT's scope is intra-community: anonymous identity, peer admission by vote, reputation consensus among peers who share one trust boundary. That anthropology is right for the inside of a working community. Its *scope* is wrong for what happens across community lines, where anonymity is the failure mode rather than the protection. What has to cross a boundary is not a reputation score but a binding that couples a specific named machine to a specific named human whose community standing rides on the machine's conduct.

Machines participate only as delegates. A machine never originates substrate; it inherits it from one named human guardian, who inherits it from a community. Every machine-to-machine exchange reduces to an exchange between the two guardians: every machine act stays inside an envelope the guardians pre-authorized, and the guardians answer for what happens inside it. There is no machine-native legitimacy, and there is no AI-to-AI cooperation without a human on each end. The interface (what crosses the boundary, on whose authority, with what protections) is specified in [`doc/background/reputation_bridges.md`](doc/background/reputation_bridges.md).

## Implementation status

The research set is mature and the software is an MVP. The Rust implementation covers **Components 1-4 and 6** of [`doc/architecture/protocol-components.md`](doc/architecture/protocol-components.md), the ten-component decomposition of the protocol:


| #          | Component                                                         | Status                                                                                               |
| ---------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 1          | Identity and standing                                             | Implemented: Ed25519 self-certifying`did:key`, delegations, witnesses                                |
| 2          | Recognition ceremony                                              | Implemented: two-layer split (intra rite plus inter credentialing with 5-feature anti-spiral claims) |
| 3          | Archive / commons                                                 | Implemented: in-memory, content-addressed, JSON dump/load, cryptographic verification                |
| 4          | Mediation                                                         | Implemented (v1): mediator pool (per-community registries; union = pool), dispute intake, and paneled decisions, with the research's structural rules enforced through the §5 authority pass — every panelist an active registered mediator, at least a configurable minimum (default 3) per panel, and non-overlap with either party's community (`kc-mediation`). Deferred: deliberation/argument records, interim injunction (§9), the capture-resistance pool-share threshold, and the full escalation-ladder state machine (§5) |
| 6          | Reputation bridges                                                | Lookup plus standing (Unknown/Identified/InCompact/Probationary/Denied/Anathema), capture-signal detectors, and v0 read-only jester lenses (rejection-pattern, ceremony-cost-decay) (`kc-reputation`); conduct/sponsorship/non-admission/intra-rite records authorable from Python, with append-only delegation revocation and §5 authority enforcement (principal-binding + scope + quorum, `check_authority`) (`kc-conduct`, `kc-ceremony`, `kc-identity`); dispute records project onto both parties with their mediation outcome |
| 5, 7-10    | Escalation, commons, jester, enforcement, test harness            | Post-MVP (§5 escalation and §8 jester have their archived record types seeded by `kc-conduct`)       |

Three things the MVP deliberately is not:

- **Not replicated, not transported.** The archive is single-host and in-memory. Threshold-sealing, libp2p transport, live re-affirmation scheduling, and adversarial-test harnesses are post-MVP.
- **Not in-person.** The intra-rite data model is software-only. The research corpus commits to bodily co-presence for the actual rite; the MVP encodes the protocol's shape, not the embodied practice.
- **Ed25519-only.** The `Alg` enum is a single-variant structure kept open for a future non-lattice PQC addition (such as SLH-DSA/SPHINCS+) if one is accepted. NIST lattice PQC (ML-DSA/Dilithium) is deferred pending community confidence in its provenance.

## Development setup

Dependencies come from conda, not pip or venv. `environment.yaml` provides `cargo`/`rustc` (conda `rust`), `maturin`, and `pytest`; `run-tests.sh` errors early with an instruction if any is missing.

The Rust workspace is eight crates plus a demo:

- `kc-core`: shared primitives and the `Alg` crypto enum.
- `kc-identity`: Ed25519 `did:key` identities, delegations, witnesses.
- `kc-ceremony`: the two-layer recognition ceremony, five-movement recitation, and sponsorship (vouching) records.
- `kc-conduct`: attributable, append-only conduct records (incl. the jester's pre-indictment `Concern`), seeding the escalation and reputation-ledger components.
- `kc-mediation`: the §4 mediation slice — per-community mediator registries (union = the pool), dispute records, and paneled decisions; the panel rule (registered, non-party, ≥ minimum) is enforced by the archive's authority pass.
- `kc-archive`: the content-addressed, cryptographically verifiable audit commons.
- `kc-reputation`: the reputation-bridge slice (lookup and standing), the capture-signal detectors, and the v0 read-only jester lenses (rejection-pattern and ceremony-cost-decay) (`analytics`).
- `kc-pyo3`: the PyO3 bindings, built into `kith_covenant._core` by `maturin develop`.

The Python package under `src/python/kith_covenant/` wraps those bindings (`ceremony.py`) and carries the autonomous-trust seam (`interface.py`, `policy.py`, `reputation.py`), which imports with or without the compiled `_core`.

## Documentation


| Document                                                     | Description                                                                                                         |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| [doc/summary.md](doc/summary.md)                             | Plain-language summary (start here)                                                                                 |
| [doc/orientation.md](doc/orientation.md)                     | Guided walk through the design, with the reasoning behind each part                                                 |
| [doc/overview.md](doc/overview.md)                           | The full standalone argument                                                                                        |
| [doc/whitepaper/whitepaper.md](doc/whitepaper/whitepaper.md) | Consolidated synthesis in a single document                                                                         |
| [doc/kith_covenant.md](doc/kith_covenant.md)                 | Map of the whole research and architecture set                                                                      |
| [doc/architecture/](doc/architecture/README.md)              | Software decomposition: protocol components, machine participation, policy evaluation                               |
| [doc/background/](doc/kith_covenant.md)                      | The research corpus: historical cases,*justus hostis*, recognition protocols, commons, capture resistance, and more |

## License

Apache License 2.0.
