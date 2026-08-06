# Kith-Covenant Protocol Architecture

Technical architecture documentation for the Kith-Covenant social protocol evaluation framework.

## Contents

- [Protocol Components and Operational Gaps](protocol-components.md): software decomposition realizing the research-set commitments; component-by-component scope; architectural decisions; incremental build order
- [Remaining work](../../TODO.md): the backlog, kept outside the reference documents. Everything the protocol text describes as in scope but does not yet do, per component, each tagged in-sandbox or deployment-gated, with links to spec, source docs, and code
- [Erosion Legibility, the Uniform-Drift Audit](erosion-legibility.md): classification of every erosion component, reputation projection, and capture lens as relative / event-triggered / absolute; why uniform drift (a sincere, uncoordinated decline where nobody defects) is invisible to the first two kinds; findings E1-E2 and K1-K4 with the four wired remedies, and the two further measures the audit names but does not carry (attested nulls, an observer reporting cadence)
- [Machine Participation](machine-participation.md): how machine agents under current technology (no sentience assumed) can participate in kith-covenant as derived-substrate parties bound through their constitutive service to specific humans; the three moves of faith-governed virtue mapped; four layered binding mechanisms; the asymmetries with human oath-binding
- [Build, Toolchain, and Verification](build-and-toolchain.md): the build environment and its two non-optional facts; the MSRV floor and how a declared version came to mean nothing; the MSRV-aware resolver; lint policy and the pyo3 migration; cargo feature unification across the two workspaces; the bug class both integrity passes are blind to by construction; test hazards and the negative-control discipline
- [Policy Evaluation](policy-evaluation.md): AT-Kith-Covenant interface, fitness function, and evaluation framework
- [AutonomousTrust Integration](autonomous-trust-integration.md): the plan for melding AT (intra-community, earned/emergent, machine-tempo reputation) with KC (inter-community, conferred/covenantal, deliberative-tempo governance), hosted in KC importing the AT package; disconnect analysis (ontology, trust-vs-standing, tempo, guardian axis, enforcement, lifecycle, transport), the meld proposals M1-M8, the five concrete first couplings, phased path, and open questions
