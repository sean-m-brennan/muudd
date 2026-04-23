# Kith-Covenant

- shared civil protocols, a social techne for distributed large scale cooperation
- formerly Politē, from the Greek _πολιτεία (politeía)_ - lit. organizational structure and governance

## Repository layout

This repo contains both the research corpus (under `doc/`) and an MVP Rust
implementation of Components 1–3 from the architecture (identity, recognition,
archive).

```
.
├── doc/
│   ├── whitepaper.md              Consolidated whitepaper (~16k words)
│   ├── overview.md                Standalone narrative anchor
│   ├── summary.md                 Short précis
│   ├── kith_covenant.md           Index to the research corpus
│   ├── research/                  10 research documents
│   └── architecture/              Software decomposition + design docs
├── crates/                        Rust workspace (MVP, Components 1–3)
│   ├── kc-core/                   Canonical CBOR, Ed25519, Cid, Signed<T>, DidKey
│   ├── kc-identity/               CommunityIdentity, delegations, witnesses, keystore
│   ├── kc-ceremony/               intra/ + inter/ (two-layer strict separation)
│   ├── kc-archive/                In-memory archive, JSON dump/load, verify
│   └── kc-pyo3/                   Python bindings (via maturin)
├── examples/
│   └── two_communities_demo/      Rust end-to-end demo binary
└── src/python/
    ├── kith_covenant/             Python package (bindings + AT seam)
    └── examples/                  Python mirror of the Rust demo
```

## Quickstart

The project uses a per-component conda env (consistent with the rest of
sustainable-space) — `environment.yaml` pins `python==3.12.*`, `rust`,
`rust-src`, `maturin`, and `pytest` from conda-forge.

```bash
cd lib/muudd/lib/kith-covenant
conda env create -f environment.yaml     # first time only
conda activate kith-covenant
./run-tests.sh                           # cargo tests + Rust demo + PyO3 build + pytest + Python demo
```

Or run individual pieces:

```bash
# Rust only
cargo test --workspace --exclude kc-pyo3     # 72 unit + property tests
cargo run -p two_communities_demo            # end-to-end ceremony with audit trail

# Python bindings (after conda env active)
maturin develop                              # builds kc-pyo3 → kith_covenant._core
python src/python/examples/two_communities_demo.py
pytest src/python/tests/
```

The bindings expose a thin Pythonic surface — see `src/python/kith_covenant/ceremony.py`.
The AT seam (`interface.py`, `policy.py`) imports cleanly with or without the
bindings compiled.

## MVP scope

The Rust implementation covers **Components 1–3** of
[doc/architecture/protocol-components.md](doc/architecture/protocol-components.md):

| # | Component               | MVP status |
|---|-------------------------|------------|
| 1 | Identity and standing   | implemented (Ed25519 self-certifying did:key, delegations, witnesses) |
| 2 | Recognition ceremony    | implemented (two-layer split: intra rite + inter credentialing with 5-feature anti-spiral claims) |
| 3 | Archive / commons       | implemented (in-memory, content-addressed, JSON dump/load, cryptographic verification) |
| 6 | Reputation bridges | minimal slice (lookup + standing, 2026-04-22) — `kc-reputation` crate |
| 4, 5, 7–10 | Mediation, escalation, commons, jester, enforcement, test harness | out of scope for MVP |

**Crypto.** Ed25519-only for MVP. The `Alg` enum is a single-variant structure
preserved for future non-lattice PQC additions (e.g. SLH-DSA/SPHINCS+) if and
when one is accepted. NIST lattice PQC (ML-DSA/Dilithium) is deferred pending
community confidence in its provenance.

**Not replicated, not transported.** The archive is single-host in-memory.
Threshold-sealing, libp2p transport, live re-affirmation scheduling,
adversarial-test harnesses, and Components 4–10 are post-MVP.

**Not in-person.** The intra-rite data model is software-only; the research
corpus commits to bodily co-presence for the actual rite. The MVP encodes the
protocol shape, not the embodied practice.

## Architecture

For technical architecture documentation, see [doc/architecture/](doc/architecture/README.md).
Key references:
- [`doc/overview.md`](doc/overview.md) — standalone narrative
- [`doc/whitepaper.md`](doc/whitepaper.md) — consolidated synthesis
- [`doc/architecture/protocol-components.md`](doc/architecture/protocol-components.md) — ten-component decomposition (MVP covers §§1–3)

## Introduction

Possibility: ML model for protocols?

Concept: "open-carry" - participants are visibly heavily armed, not as threat or intimidation, but to indicate preparedness, determination, and seriousness. Akin to "speak softly but carry a big stick." Enforcement capability is visible; concealment is neither expected nor rewarded.

Aristocracy of virtue
