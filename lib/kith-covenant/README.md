# Kith-Covenant

**Kith-Covenant** (KC) is a protocol for cooperation among communities that do not share ends and cannot appeal to any authority standing above all of them. It sits on top of [ethne](../ethne/README.md), the polity tier that turns cooperating agents into a coherent, chartered community, which may in turn be fed by [autonomous-trust](../autonomous_trust/README.md). AT handles anonymous reputation *inside* a single community. KC handles the other scope: known identity, tied to a specific named human whose social standing is at stake, binding one community to another across a boundary neither side rules.

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
cargo test --workspace --exclude kc-pyo3     # 379 unit and property tests
cargo run -p two_communities_demo            # end-to-end ceremony with audit trail

# Python bindings (conda env active)
maturin develop                              # builds kc-pyo3 into kith_covenant._core
python src/python/examples/two_communities_demo.py
pytest src/python/tests/
```

The bindings expose a thin Pythonic surface; see `src/python/kith_covenant/ceremony.py`. The autonomous-trust seam (`interface.py`, `policy.py`) imports cleanly whether or not the bindings are compiled.

## Start here

Read these in order; each goes one level deeper than the last.

1. **[Summary](doc/summary.md)**: the problem, the answer, and the technical shape in about 500 plain-language words.
2. **[Orientation](doc/orientation.md)**: a guided walk through the whole design. Each section names a facet, says what it is for, and points to the background document that argues it. This is the table of contents with reasoning attached.
3. **[Overview](doc/overview.md)**: the full standalone argument, with the four threads (inversion, substrate, fallen-actor anthropology, mutual vulnerability), the two-layer recognition model, the machine layer, and the seven open holes named with their disposition.

For everything in one document (executive summary, argument, software architecture, historical appendices, and bibliography), read the [whitepaper](doc/whitepaper/whitepaper.md).

## Relation to autonomous-trust

Autonomous-trust sits underneath Kith-Covenant, and nothing in KC replaces it. AT's scope is intra-community: anonymous identity, peer admission by vote, reputation consensus among peers who share one trust boundary. That anthropology is right for the inside of a working community. Its *scope* is wrong for what happens across community lines, where anonymity is the failure mode rather than the protection. What has to cross a boundary is not a reputation score but a binding that couples a specific named machine to a specific named human whose community standing rides on the machine's conduct.

Between the two sits **[ethne](../ethne/README.md)**, the polity tier: AT produces *emergent* cooperation among nodes, but not a bounded, coherent, persistent community. That is the party KC already presumes as its atomic actor. Ethne is what turns the cooperation into that party, by *ratifying* the emergent boundary into a signed charter rather than imposing one. KC depends on neither ethne nor AT (KC ⟂ ethne, KC ⟂ AT): ethne speaks to KC only through KC's own public vocabulary. It hands KC a standard `CommunityIdentity`, surfaces a coherence breakdown as KC's existing `Concern`, and presents a community split as two new communities plus the old one going quiet. KC sees communities and evidence; it never learns the lower tiers exist.

Machines participate only as delegates. A machine never originates substrate; it inherits it from one named human guardian, who inherits it from a community. Every machine-to-machine exchange reduces to an exchange between the two guardians: every machine act stays inside an envelope the guardians pre-authorized, and the guardians answer for what happens inside it. There is no machine-native legitimacy, and there is no AI-to-AI cooperation without a human on each end. The interface (what crosses the boundary, on whose authority, with what protections) is specified in [`doc/background/reputation_bridges.md`](doc/background/reputation_bridges.md).

## The ten components

The Rust implementation covers **all ten components** of [`doc/architecture/protocol-components.md`](doc/architecture/protocol-components.md), the ten-component decomposition of the protocol. What each component provides is below; the per-component detail is in the spec's own sections, and anything outstanding is tracked in [`TODO.md`](TODO.md).


| #          | Component                                                         | What it provides                                                                                     |
| ---------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| 1          | Identity and standing                                             | Ed25519 self-certifying`did:key`, delegations, witnesses                                |
| 2          | Recognition ceremony                                              | two-layer split (intra rite plus inter credentialing with 5-feature anti-spiral claims); admissions integrity enforced: probation withholds weight in admissions, mediator selection and designation procedures, an admission into a standing compact needs an admitting quorum of sitting non-probationary members plus a sponsor per entrant, and examining seats are term-limited, rotated and pool-share capped (`ExaminerCaps`, sharing one implementation with the §8 observer caps) |
| 3          | Archive / commons                                                 | in-memory, content-addressed, JSON dump/load, cryptographic verification                |
| 4          | Mediation                                                         | mediator pool (per-community registries; union = pool) with **term, rotation and pool-share caps**, dispute intake, **argument and deliberation records**, **recusal requests answered**, **protecting powers**, **training lineage**, and paneled decisions, with the structural rules enforced through the authority pass: every panelist an active registered mediator, a configurable minimum per panel (default 3), non-overlap with either party's community, a **mandatory** reasoning trail, a `panel` field bound to the co-signers, one cited argument per co-signer, and every party's filed case answered (`kc-mediation`) |
| 5          | Escalation and designation                                        | the ladder state machine through to expulsion and the substantive-re-engagement rung, the four-condition anathema test, restitution obligation and verification, re-entry, and the **recorded conclusion of a designation's mandated re-examination** (a confirmation answers its deadline and sets the next one; a lapsed designation may not be cited by a further escalation act) (`kc-escalation`) |
| 6          | Reputation bridges                                                | lookup plus standing (Unknown/Identified/InCompact/Probationary/Denied/Anathema), the capture-signal detectors and jester lenses (rejection pattern, ceremony-cost decay, composition graph, interpretive plurality, structured skepticism, adversarial principal, erosion alert with nine signals, six of them absolute, including **silent and unstaffed standing functions**) (`kc-reputation`); records authorable from Python with append-only delegation revocation and authority enforcement (principal binding + scope + quorum, `check_authority`) |
| 7          | Commons governance                                                | the five compact-level commons, usage accounting, and the graduated-sanction ladder Monitoring→Exclusion with the heavier levels quorum-gated (`kc-commons`) |
| 8          | Jester and observer function                                      | observer-corps registry with enforced term, rotation and pool-share caps; attributed cross-community reports with an irrevocable right of response; and a whistleblower channel whose source proves insider standing by ring signature and whose identity is held k-of-n by the reception panel (each share sealed to a mediator's registered encryption key, distinct from its signing key, and checked against it, so custody cannot be named for one party and sealed to another), with a no-panel exculpation for anyone wrongly named. The corps also owes an account of each window it holds seats for, **including an account of having seen nothing**, so a corps that stopped looking is distinguishable from one with nothing to see. The ring spans a community's root key, its live delegations and its published member roll (fed by the polity tier), and is computed over the log *preceding* each filing, so no later record can invalidate a filing or narrow the suspect set around its source (`kc-observer`, see [`doc/architecture/whistleblower-crypto.md`](doc/architecture/whistleblower-crypto.md)) |
| 9          | Enforcement and non-ultimacy                                      | the compact-incapable list as a genesis-immutable constitutional artifact with asymmetric amendment (expansion easy, contraction harder), the structured-fast cadence, interim injunctions with capability gating, and graded erosion alerting (`kc-constitution`, `kc-escalation`) |
| 10         | Bootstrapping and test harness                                    | the capture-resistance harness scores ten scenarios (the seven strengthening moves, with move 7 counted twice for who a faction can seat and what a seated entrant can carry, plus the two with no adversary: the quiet polity and the abandoned office) against falsification criteria stated in advance, run continuously over a parameterized space with seeded search, directed sweeps, and shrinking (`harness.py`) |

Three things the implementation deliberately is not:

- **Not replicated, not transported.** The archive is single-host and in-memory, and the protocol is specified without reference to how records travel between hosts.
- **Not in-person.** The intra-rite data model is software-only. The research corpus commits to bodily co-presence for the actual rite; the code encodes the protocol's shape, not the embodied practice.
- **Ed25519-only.** The `Alg` enum is a single-variant structure kept open for a future non-lattice PQC addition (such as SLH-DSA/SPHINCS+) if one is accepted. NIST lattice PQC (ML-DSA/Dilithium) is excluded pending community confidence in its provenance.

## Development setup

Dependencies come from conda, not pip or venv. `environment.yaml` provides `cargo`/`rustc` (conda `rust`), `maturin`, and `pytest`; `run-tests.sh` errors early with an instruction if any is missing.

The Rust workspace is fifteen crates plus a demo. The principal ones:

- `kc-core`: shared primitives and the `Alg` crypto enum.
- `kc-log`: the append-only, content-addressed log, generic over the entry type: the archive *mechanism*, with generation-safe cursors and the versioned JSON dump/segment format. Neutral: no governance records and no dependency on any other `kc-*` crate, so the ethne polity tier builds its own archive on it too (ethne decision D32).
- `kc-identity`: Ed25519 `did:key` identities, delegations, witnesses.
- `kc-ceremony`: the two-layer recognition ceremony, five-movement recitation, and sponsorship (vouching) records.
- `kc-conduct`: attributable, append-only conduct records (incl. the jester's pre-indictment `Concern`), seeding the escalation and reputation-ledger components. It also carries the **attested null** (`ActivityAttestation`): a community's signed account of one window in a standing function it holds, whose empty act list is the point of the record. Silence is then either evidence or a fact about the compact, where before it was missing data and a quiet pool could not be told from a stopped one.
- `kc-mediation`: the §4 mediation slice. Per-community mediator registries (union = the pool), dispute records, argument records, recusal requests and their answers, protecting-power designations, and paneled decisions; the panel rule (registered, non-party, ≥ minimum) is enforced by the archive's authority pass.

  An `ArgumentRecord` is §4's "structured argument submission" and "evidence packaging": append-only, self-signed, and held to a different rule per role. A `Party` argument is a party's own case and only the complainant or respondent may make one, while a `Panelist` argument requires an active registered mediator from neither party community and that mediator's own signature, since nobody argues in a mediator's name. A decision cites the arguments it answers, and the pass requires **one cited panelist argument per co-signer** (three signatures on one author's text is a rubber stamp, not a panel) and that **every party argument already in the log is answered** (a panel may not decide while ignoring a case put to it; insertion order is authoritative, so a later filing cannot unmake a concluded mediation).

  The record also made two existing fields mean something. §4 specifies a *mandatory* reasoning trail and nothing enforced it, so a decision could publish an empty `reasoning`. And `MediationDecision.panel` was read by no rule at all: it could name the two *party* communities, precisely whom non-overlap excludes, while three legitimate non-party mediators signed, and the record was authorized. Both are now checked, the panel field against the co-signers it claims to report.

  **The pool itself is now capped.** `MediatorCaps` puts mediator seats through the same `check_roster_caps` the §2 examiner and §8 observer caps read: bounded terms, a sit-out between them, the continuous-service union, and no single community holding a majority of the active pool once the pool is past a floor. It was the third roster and the one wired to nothing, so a community could hold nine of ten seats and staff every panel in the compact, and a mediator could be seated with `valid_until: None`, a seat with no end, which is a permanent mediator class, "the first step toward an inquisitorial caste". The whole fixture suite was seating mediators that way, which is what repairing it cost.

  **Recusal, protecting powers, and lineage** complete the pool directory §4 specifies. A party names one panelist on one dispute and states grounds; the request is recorded and answered, granted only by the mediator stepping down and never refused by that same mediator. A `ProtectingPower` is the Geneva analogue: a non-party community standing for a disputant, co-signed by both, since the duty is never conscripted and a protector a disputant did not accept is a watcher imposed on them. `MediatorEntry.trained_by` records the apprenticeship. None of the three gates anything: the mediation decides exactly as it would have, and the lineage is read only by the pool board, because the doctrine that asks for the record also warns against "a certification barrier that becomes a gatekeeping mechanism".
  **Consent to the panel as constituted** is `PanelConsent`, self-signed because nobody consents on another's behalf, naming the panel in full so that consent to three mediators is not consent to a fourth. Objection had a record and structural consent had a record; the ordinary case (this panel, these mediators, may decide my dispute) was inferred from the party having filed an argument, which is a different act. It gates one thing and deliberately no more, since a consent precondition on deciding would hand either party a veto over its own dispute: a `Settled` outcome is a claim that *the parties* settled, so both must have consented to the panel recording it. Before the rule, a panel could record a settlement neither party had agreed to, and every §4 fixture in the test suite was doing exactly that.

  **Sealed decisions** are `SealedReasoning` on the decision plus a `DecisionUnsealed` that opens it, over the same `ThresholdCustody` §8 holds a source's identity under. What is sealed is the reasoning: dispute, panel and outcome stay public, so a compact can see that a matter was decided, by whom, and how it came out, and every rule above keeps running. What waits is why. An unsealing must present enough opened shares to recover the text and that text must open the decision's commitment, so a custodian quorum cannot publish a reasoning the panel never wrote.

  The custodians are **member communities, not mediators**, and that was a finding rather than a preference. The whitepaper asks for the keys to sit with the mediator pool *and* for a deterministic reveal at the twenty-five-year mark; §4 caps a mediator's seat at two years, so a share sealed to a mediator is held by a key the archive stops resolving two decades before the seal opens, leaving the reveal dead by construction and the signature on the opening unverifiable. Community identities resolve at any instant. The doctrine's substance is kept by arithmetic: no party may hold a share, and no quorum of shares may sit with the deciding panel's own communities, because a panel that can unseal itself has deferred publication at its own discretion rather than sealed anything.
- `kc-provenance`: the manufactured-event records of capture move 4, and §4's authority over them. `EventReport` carries provenance tags and the originator's own qualifying triggers; `SkepticismInvocation` is the **mediator pool** placing an event under the structured-skepticism protocol on its own initiative, and `SkepticismReview` is the pool's internal review against pretextual use.

  The hold always had teeth, since an action premised on a qualifying, under-corroborated event is refused rather than merely flagged. But *qualifying* was four booleans the originating community set on its own report, so the teeth rested on the word of the party the protocol exists to constrain. Measured: an originator that declared its own event mundane was never held, and a full non-party mediator pool sitting in the same archive could do nothing about it. Either source now places the hold, and the window is the longer of them.

  The review is against the **hold** rather than the invocation record, which is what lets it reach the self-declared case: the one that answered to nobody, where a community could put its own urgent matter beyond action with no reviewer anywhere on the record. A `Pretextual` finding releases the event from a hold of either origin. Mediators who co-signed an invocation may not review it, the originating community may neither invoke nor review, and reasoning is required on both answers, because an audit legible only when the hold falls says nothing about the holds that stood.
- `kc-archive`: the content-addressed, cryptographically verifiable audit commons: the typed record enum, DID-to-key resolution, verification, and the authority pass, over a `kc-log` archive.
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
