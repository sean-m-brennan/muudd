# Ethne

**Ethne** (Greek ἔθνη, "peoples") is the **polity tier**: it turns a set of cooperating agents into a *community* — a bounded, self-governing entity that stands for a substrate, acts with one voice, and persists across the turnover of its members.

It sits between a trust substrate and a federation. Below it, a substrate like [autonomous-trust](../autonomous_trust/README.md) produces *emergent* trust among nodes but no community: its groups and gateway subtrees are topological clusters, not covenant-bearing entities with a membership roll and a standing. Above it, [kith-covenant](../kith-covenant/README.md) governs relations *between* communities and presumes each one is already coherent, persistent, and able to act as a unit. Ethne is the tier that produces the entity KC presumes, out of whatever cooperation the substrate supplies.

```
   autonomous-trust        ethne                     kith-covenant
   "who cooperates?"  ──►  "what is a community —  ──►  "how do communities
                            and what keeps it one?"      treat one another?"
```

The governing discipline is **ratify emergence, do not replace it**: ethne reads the emergent structure and offers it back to the members for deliberate ratification into a charter. Emergence proposes; the polity ratifies; KC recognizes.

Ethne is a **separate tier by design**, so that kith-covenant depends on neither ethne nor autonomous-trust: ethne emits only KC-native artifacts (a `CommunityIdentity`, recognition records, and a `Concern` on a coherence breakdown), and autonomous-trust is one *pluggable* substrate provider among others.

## Status

Design decisions D1–D13 are locked and Q2/Q3/Q6/Q7/Q8/Q9 resolved; see **[doc/design.md](doc/design.md)** for the full architecture: the gap it fills, the decoupling rule, the five responsibilities (founding & charter, substrate articulation, internal offices, cohesion monitoring, longevity), what it reuses from KC vs. what is new, the machine-guardian model, the lifecycle, and the open questions.

Built so far — the Rust workspace and the Python surface over it (D2):

- **`en-core`** — base crate; re-exports the neutral `kc-core` primitives ethne builds on (D1: kc-core only).
- **`en-archive`** — append-only, content-addressed log, generic over the entry type (the neutral archive mechanism, forked thin from `kc-archive`; verification/DID-resolution are domain concerns for later slices).
- **`en-charter`** — the polity **founding** primitive (keystone). Records (`Substrate`, `Commitment`, `FoundingProposal`, `PolityCharter`, `CommitmentAmendment`, the roll-lifecycle `MembershipChange` (`Admit` / `DepartVoluntary` / `Expel`), and the terminal `CharterDissolution`), the *ratify-not-replace* API (a `FoundingProposal` carries no authority; only `ratify` + co-signing yields a charter), signature verification (`did:key` self-resolves), state derivation (`charter_state` — the *live* roll = founding + admits − departures, plus commitments, dissolution + successors), and the authority rules: genesis-singleton, majority ratification, genesis-immutable substrate + founding commitments, asymmetric amendment (add at a majority, remove at a majority + 1), an asymmetric membership lifecycle (Admit at a majority + the joiner's consent, DepartVoluntary self-signed, Expel at a majority + 1 — all under a self-govern floor), an internal-office lifecycle (`OfficeChange` over `Office { Convener, Inductor, EnvoyAuthorizer, CharterKeeper, Other }` — Confer at a majority + the holder's consent, Resign self-signed, Revoke at a majority with the holder excluded; the roll confers, offices never confer offices, so the founding roll bootstraps the first ones with no regress; a holding is effective only while its holder is on the roll), majority-of-roll dissolution, and terminal-dissolution (no charter event after a polity is dissolved). `office_state` derives who holds what. **Transmission** (the `Affirmation` record over `AffirmationKind { Induction, ReAffirmation }`): admission puts someone on the roll, while *induction* is the distinct step by which they acquire the substrate — witnessed by a majority of the roll co-signing alongside the inductee, with founding members inducted by ratifying the charter — and *re-affirmation* is self-signed renewal by an already-inducted member. `affirmation_state` reports each current member's induction status and last affirmation, and `stale_members(cadence_seconds, as_of)` names the un-inducted and the overdue, so a community that stops re-affirming shows erosion (the freshness cadence is a query parameter, not a charter term).
- **`en-substrate`** — the pluggable `SubstrateProvider` interface: the source of the emergent boundary a founding ratifies (`FoundingCandidate` → `propose` → `FoundingProposal`). Ships `ManualFounding`, a convener-driven provider needing no substrate service.
- **`en-at`** — the `autonomous_trust` provider (D5–D8): an `AtEventSource` seam (mock + `at-ffi`-gated embed/attach coupling) folded into an `AtSubstrateProvider` that maps AT's gateway-reputation subtree to the candidate roll and the verified operator to each member's guardian edge. The default build links no AT — KC ⟂ AT holds with the adapter present.
- **`en-cohesion`** — the polity's **coherence monitor** (a pure `kc-core`-only read-model). `cohesion_report` grades strain into an `ErosionLevel` (`None` / `Notice` / `Concern`) off the chartered roll vs. the provider's current candidate — attrition, reputation decay, floor breaches, an opaque boundary capping attrition at `Notice`. It never decides a split (see D4/Q2).
- **`en-uplift`** — the **upward interface** to kith-covenant (D4/D9/D10) and the *one* crate holding the confined `kc-conduct` dependency. `community_identity` hands a chartered polity off as a self-signed `CommunityIdentity`; `coherence_concern` emits KC's `Concern` (neutral `Other("coherence-erosion")` bearing) only at `Concern`-level erosion; `fission` emits a ratified split — two successor identities, a signed `SuccessorLink` to each, and the predecessor's `CharterDissolution`; `fusion` emits the mirror ratified merger — N (≥ 2) predecessors → one merged `CommunityIdentity`, with a `SuccessorLink` and a `CharterDissolution` (naming the merged polity as sole successor) per predecessor.

- **`en-pyo3`** + **`src/python/ethne/`** — the **Python surface** (D2): an `ethne._core` extension module built by maturin, and a Pythonic package over it in the tier's own four parts — `ethne.polity` (the `Archive` charter log, keys, and the three derived read-models), `ethne.authoring` (a `Signer` plus one call per charter record: found, amend, admit / depart / expel, confer / resign / revoke an office, induct / re-affirm, dissolve), `ethne.monitor` (a `FoundingCandidate`, `manual_founding`, and `cohesion_report`), and `ethne.uplift` (the three artifacts that cross to kith-covenant, as JSON a KC-side consumer appends verbatim). The ratify-not-replace discipline survives the crossing: `propose_charter` returns only JSON, and a charter exists only once `ratify_charter` has a majority of the roll co-sign it. `en-at` has no bindings yet — its real coupling is behind the `at-ffi` gate — but Python can drive the same path through `founding_candidate`.

```bash
cd lib/muudd/lib/ethne

# Rust tier (en-pyo3 is a cdylib with no test harness of its own)
cargo test --workspace --exclude en-pyo3   # 73 tests (archive + charter + substrate + at + cohesion + uplift)

# Python surface
maturin develop                            # builds en-pyo3 into ethne._core
pytest src/python/tests/                   # 56 tests
python src/python/examples/polity_demo.py  # a polity's life, end to end

./run-tests.sh                             # all of the above in order
```

The build needs a per-component **conda** env, consistent with the rest of sustainable-space: `environment.yaml` pins `python==3.13.*`, `rust`, `rust-src`, `maturin`, and `pytest` from conda-forge (`conda env create -f environment.yaml && conda activate ethne`). `run-tests.sh` errors early with an instruction if any of them is missing.

The decoupling is concrete and machine-checked (`cargo tree`): every core and monitor crate (`en-core`, `en-charter`, `en-substrate`, `en-at`, `en-cohesion`) depends only on the neutral `kc-core` — no `autonomous_trust`, no kith-covenant governance crate. The single `kc-conduct` coupling needed to speak KC's `Concern` is confined to `en-uplift` and goes no further; ethne never depends on `kc-archive` (it emits serialized `Signed<T>` payloads a KC-side consumer appends). `en-substrate`'s end-to-end test founds a polity through the manual provider with neither AT nor KC present.

Next, the deferred slices: the multi-movement choreography of the intra-community rite (v1 records the rite's completion, not its movements), the intra/inter ceremony rites for envoy recognition, use-time office enforcement, wiring affirmation staleness into `en-cohesion` / an `en-uplift` `Concern`, and `en-at` bindings. (Membership admission / departure / expulsion — D11 — fusion, the N-ary mirror of fission, internal offices — D12 — longevity & continuity — D13, induction / re-affirmation plus the freshness query — and the PyO3 / Python surface — D2 — are now built.)

## License

Apache-2.0.
