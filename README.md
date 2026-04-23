# Mesh-routed User-owned User-controlled Dynamic Domains (MUUDD)

Decentralized, trust-based communications infrastructure for building resilient community networks independent of centralized internet and surveillance systems — combining mesh networking, high-trust cooperative computing (AutonomousTrust), and shared civil protocols (Kith-Covenant) to enable candid, secure person-to-person communication.

### Architecture

- [MUUDD Communications Architecture](doc/architecture/README.md)

### Development environment

Each component (`muudd`, `lib/autonomous_trust`, `lib/kith-covenant`) has its own
`environment.yaml` conda spec. To work on the stack end-to-end in a single env,
compose them with:

```bash
./compose-environment.py                      # writes master-environ.yaml
conda env create -f master-environ.yaml       # or `conda env update -f ...`
conda activate sustainable-space-master
```

The composer merges channels + dependencies across the three files, reports any
conflicts (e.g. disagreeing version pins) to stderr and exits non-zero. The
generated `master-environ.yaml` is a derived artifact and is gitignored — edit
the per-component `environment.yaml` files, not the merged output.

### Research

See [MUUDD research documentation](doc/muudd.md) for the full research index.

- [MUUDD: Biblically-aligned Networks](doc/muudd.md)
- [Decentralized Mesh Network for Rural Appalachian Connectivity](doc/rural-network.md)
- [Simulating a Community Mesh Network: Open-Source Tools and Integration](docs/network_simulation.md)
- [MUUDD System Simulation: Terrain to Asteroid Belt](docs/system-simulation.md)

