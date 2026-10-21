# Mesh-routed User-owned User-controlled Dynamic Domains (MUUDD)

MUUDD is a communications network that belongs to the people using it. Radios on
ridgetops and rooftops, powered by solar panels and owned by the households they
serve, carry traffic to each other directly instead of routing everything
through a carrier's equipment. No company can price the network out of a county,
no outage upstream takes it down, and no operator sits in a position to read
what passes through.

That is the physical layer, and it is the whole of what MUUDD proper is:
hardware that moves bits without asking permission. Everything above it is
optional.

Three further tiers can be layered on top, and each answers a question the layer
below it cannot. Firstly, *AutonomousTrust* asks who may cooperate, and answers
by having each node judge its peers on observed behavior rather than on a
credential. Second, *ethne* asks what a community is, and answers by turning a
group that already cooperates into a chartered polity that persists as its
members turn over. Lastly, *Kith-Covenant* asks how communities treat one
another, and answers with a civil protocol that binds parties who share no creed
and answer to no common sovereign.

Although the three are built to stack, none of them is required. A network can
run with none, with one, or with all three, and the physical layer neither knows
nor cares which.

## The reading path

The documentation is one continuous narrative. Start at chapter 1 and follow the
`Next` link at the foot of each chapter, or jump in wherever the subject
interests you. Every chapter opens with a plain-language lead section that
stands on its own, so reading only the leads gives the whole argument at low
resolution.

The machine-readable order is [`doc/narrative-order.json`](doc/narrative-
order.json).

### Part I: MUUDD, the physical layer

1. [Why user-owned networks](doc/muudd.md)
2. [Who this is for](doc/audience_profile.md)
3. [Nodes and radios](doc/hardware_design.md)
4. [What a person carries](doc/device_model.md)
5. [What runs on a node](doc/software_architecture.md)
6. [Speaking without being identified](doc/anonymous_communication.md)
7. [Trusting the metal](doc/verifiable_infrastructure.md)
8. [The optional stack](doc/the-stack.md)

### Part II: AutonomousTrust, who may cooperate

9. [Trust as a live value](lib/autonomous_trust/doc/concept.md)
10. [How a node is built](lib/autonomous_trust/doc/architecture/overview.md)
11. [Becoming a peer](lib/autonomous_trust/doc/architecture/identity-protocol.md)
12. [First contact](lib/autonomous_trust/doc/architecture/first-contact.md)
13. [Getting work done](lib/autonomous_trust/doc/architecture/negotiation.md)
14. [Earning standing](lib/autonomous_trust/doc/architecture/reputation.md)
15. [Standing turned into capability](lib/autonomous_trust/doc/architecture/trust-tiers.md)
16. [The human behind the machine](lib/autonomous_trust/doc/architecture/operator-attended.md)
17. [A worked scenario](lib/autonomous_trust/doc/example-application.md)

### Part III: Ethne, what a community is

18. [From cooperation to community](lib/ethne/doc/concept.md)
19. [Founding a polity](lib/ethne/doc/charter.md)
20. [Which human answers for which machine](lib/ethne/doc/guardianship.md)
21. [The rite](lib/ethne/doc/transmission.md)
22. [Watching for erosion](lib/ethne/doc/cohesion.md)
23. [Crossing to the compact](lib/ethne/doc/uplift.md)

### Part IV: Kith-Covenant, how communities treat one another

24. [Cooperation without a sovereign](lib/kith-covenant/doc/concept.md)
25. [The ground each community stands on](lib/kith-covenant/doc/substrate.md)
26. [Two layers of recognition](lib/kith-covenant/doc/recognition.md)
27. [Four categories of standing](lib/kith-covenant/doc/standing.md)
28. [When communities disagree](lib/kith-covenant/doc/disputes.md)
29. [The five commons](lib/kith-covenant/doc/commons.md)
30. [Protected dissent](lib/kith-covenant/doc/jester.md)
31. [Machines as delegates](lib/kith-covenant/doc/machines.md)
32. [Capture resistance](lib/kith-covenant/doc/capture.md)
33. [What only humans can do](lib/kith-covenant/doc/limits.md)

### Appendices

The reference corpus, chained after chapter 33 and linked from the chapters that
draw on it.

- **A**: [MUUDD reference](doc/rural-network.md). A rural deployment worked in
  full, gateway uplinks, the simulation framework, and the case for
  decentralization.
- **B**: [AutonomousTrust reference](lib/autonomous_trust/doc/architecture/process-architecture.md).
  Process architecture through to the whitepapers, thirty-seven documents.
- **C**: [Ethne reference](lib/ethne/doc/crates.md). The crates, the design
  decisions D1 to D40, and the toolchain.
- **D**: [Kith-Covenant reference](lib/kith-covenant/doc/background/justus_hostis.md).
  Twenty-two background studies, the crates, nine architecture documents,
  and the whitepaper.

## Repository layout

```
muudd/
├── doc/                    Part I chapters and appendix A
├── docs/                   simulation and protocol reference
├── src/                    physical-layer tooling
└── lib/
    ├── autonomous_trust/   Part II, and appendix B (git submodule)
    ├── ethne/              Part III, and appendix C
    └── kith-covenant/      Part IV, and appendix D
```

## Development environment

Each component (`muudd`, `lib/autonomous_trust`, `lib/ethne`, `lib/kith-
covenant`) carries its own `environment.yml` conda specification. To work on
the whole stack in a single environment, compose them:

```bash
./compose-environment.py                      # writes master-environ.yml
conda env create -f master-environ.yml       # or `conda env update -f ...`
conda activate continuous-space-master
```

The composer merges channels and dependencies across the component files,
reports any conflict (disagreeing version pins, for instance) to stderr, and
exits non-zero. The generated `master-environ.yml` is a derived artifact and is
gitignored. Edit the per-component `environment.yml` files, never the merged
output.

Clone with submodules, since `lib/autonomous_trust` is one:

```bash
git submodule update --init --recursive
```

## License

Apache License 2.0.
