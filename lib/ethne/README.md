# Ethne

**Ethne** (Greek ἔθνη, "peoples") is the **polity tier**: it turns a set of
cooperating agents into a *community*, being a bounded, self-governing entity
that stands for a substrate, acts with one voice, and persists across the
turnover of its members.

It sits between a trust substrate and a federation. Below it, a substrate like
[autonomous-trust](../autonomous_trust/README.md) produces emergent trust among
nodes but no community, since its groups and gateway subtrees are topological
clusters rather than covenant-bearing entities with a membership roll and a
standing. Above it, [kith-covenant](../kith-covenant/README.md) governs
relations *between* communities and presumes each one is already coherent,
persistent, and able to act as a unit. Ethne is the tier that produces the
entity the compact presumes, out of whatever cooperation the substrate supplies.

```
   autonomous-trust        ethne                     kith-covenant
   "who cooperates?"  ──►  "what is a community,    ──►  "how do communities
                            and what keeps it one?"      treat one another?"
```

The governing discipline is **ratify emergence, do not replace it**. Ethne reads
the emergent structure and offers it back to the members for deliberate
ratification into a charter. Emergence proposes, the polity ratifies, and the
compact recognizes.

Although the three stack cleanly, ethne is a separate tier by design, so that
kith-covenant depends on neither ethne nor autonomous-trust. Ethne emits only
artifacts the compact already understands, and autonomous-trust is one pluggable
substrate provider among others.

## Quick start

The build needs a per-component **conda** env, consistent with the rest of
continuous-space. `environment.yaml` pins `python==3.13.*`, `rust`, `rust-src`,
`maturin`, and `pytest` from conda-forge, and `run-tests.sh` errors early with
an instruction if any of them is missing.

```bash
cd lib/muudd/lib/ethne
conda env create -f environment.yaml   # first time only
conda activate ethne

# Rust tier (en-pyo3 is a cdylib with no test harness of its own)
cargo test --workspace --exclude en-pyo3   # 437 tests
cargo test -p en-store --features sqlite   # 29, +4 over the default 25

# Python surface
maturin develop                            # builds en-pyo3 into ethne._core
pytest src/python/tests/                   # 233 tests
python src/python/examples/polity_demo.py  # a polity's life, end to end
python src/python/examples/polity_host.py  # the reference host: the loop, and the one clock

./run-tests.sh                             # all of the above in order
```

## Documentation

Ethne is Part III of a continuous narrative that runs from the physical mesh
layer below it up through the compact tier above. The whole chain is entered
from [the MUUDD README](../../README.md). The six chapters here are the ethne
section of it, and each links to the next at its foot.

1. [From cooperation to community](doc/concept.md): the gap this tier fills, the
   decoupling rule, and the discipline that governs everything else.
2. [Founding a polity](doc/charter.md): the charter, the roll, offices, what a
   founding says it excluded, and what survives a dissolution.
3. [Which human answers for which machine](doc/guardianship.md): derivations,
   guardian edges, and the evidence an asserted edge must carry.
4. [The rite](doc/transmission.md): induction against admission, the five
   movements, renewal, and the substrate required of whoever governs.
5. [Watching for erosion](doc/cohesion.md): observations, pruning, the graded
   measures, the store, and the host loop.
6. [Crossing to the compact](doc/uplift.md): the six border artifacts, the
   relay roster that crosses to the substrate instead, and what deliberately
   does not cross.

The reference corpus is appendix C of the same chain: [the
crates](doc/crates.md), the numbered [design decisions D1 to
D36](doc/design.md), and [build and
toolchain](doc/architecture/build-and-toolchain.md).

Outstanding scope is in [TODO.md](TODO.md).

## The decoupling, checked rather than claimed

Every core, monitor and host crate (`en-core`, `en-charter`, `en-substrate`,
`en-at`, `en-cohesion`, `en-store`, `en-host`) depends only on the neutral
`kc-core`, meaning no autonomous-trust and no kith-covenant governance crate.
The two record couplings needed to speak the compact's own vocabulary
(`kc-conduct` for the concern, `kc-identity` for the envoy delegation) are
confined to `en-uplift` and go no further. Ethne never depends on `kc-archive`,
since it emits serialized signed payloads that a consumer on the far side
appends.

The property is machine-checked with `cargo tree` rather than asserted, and it
is re-checked whenever a crate is added rather than assumed to have held. The
one C dependency in the tree, behind the optional SQLite feature of `en-store`,
is off by default and absent from the default workspace graph, which is also
`cargo tree`-checked.

## License

Apache-2.0.
