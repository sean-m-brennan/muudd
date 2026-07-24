<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Ethne — the polity tier

**Design document. Status: build steps 1–5 landed (the upward interface to
kith-covenant is built); see "Status & next steps".**

## What ethne is

*Ethne* (Greek ἔθνη, "peoples") is the tier that turns a set of cooperating
agents into a **polity**: a bounded, self-governing community that stands for
something, can act with one voice, and persists across the turnover of its
members. It sits between the trust substrate below it and the federation above:

```
   autonomous_trust (AT)          ethne                       kith-covenant (KC)
   ─────────────────────          ─────                       ──────────────────
   emergent, earned,        turns a cooperating set      recognizes, mediates, and
   machine-tempo trust  ──► into a chartered,        ──► federates coherent
   among nodes              governed, durable polity      communities
   "who cooperates?"        "what is a community —        "how do communities treat
                             and what keeps it one?"       one another?"
```

Kith-covenant's atomic actor is *a community*, and it quietly presumes that
community is already **bounded** (a known membership), **coherent** (it stands
for a substrate and can speak through delegates), **persistent** (it outlives its
current members — the structured fast, re-affirmation cycles, and
genesis-immutability only make sense for an entity that does), and **agentive**
(it can consent, be sponsored, be held responsible as a unit). A trust substrate
like AT produces none of those directly: its groups and gateway subtrees are
*emergent topological clusters, not named, root-anchored, covenant-bearing
entities* (see [KC's AT integration analysis §A1](../../kith-covenant/doc/architecture/autonomous-trust-integration.md)).
Ethne is the missing tier that produces the entity KC presumes.

## The gap, precisely

The KC↔AT integration already proposes a `CommunityBinding` (its meld proposal
M1) that maps an AT group/gateway-subtree to a KC community DID and makes the
gateway the community's **external face**. That is an *adapter*: it gives KC a
handle to bind to. It deliberately does **not** build the community's *inside* —
it punts on *which* cluster, with *what* boundary, standing for *what*. A
relabelled cluster wearing a community mask is not a coherent, governed, durable
polity. Ethne builds the inside. It is also what finally *answers* M1's punted
questions: ethne decides the boundary and the substrate that M1 then binds.

## The decoupling rule (why this is a separate tier)

**KC must not require AT, and ideally must not require ethne either.** The whole
point of a separate tier is that a community's coherence and longevity are a
concern in their own right, independent of whichever substrate happens to produce
the cooperation underneath.

Dependency directions (X → Y means "X depends on Y"):

```
   AT ──► ethne-at adapter ──► ethne ──► kc-core (identity, crypto, archive)
                                              ▲
   kith-covenant governance crates ───────────┘

   KC ⟂ AT        KC ⟂ ethne        ethne ⟂ AT
```

- **ethne** builds on the neutral `kc-core` primitives (DID identity, signing,
  the append-only content-addressed archive) and reuses KC's community-scoped
  machinery where it exists (below). It does **not** depend on AT.
- **AT is one pluggable substrate**, reached through an `ethne-at` adapter that
  depends on both AT and ethne. Swap it for a manual-founding provider, or an
  importer for an existing organisation's roster, and ethne still works. AT is
  optional.
- **KC depends on neither AT nor ethne.** Ethne speaks to KC *only through KC's
  existing public record vocabulary*: it emits a standard `CommunityIdentity`
  plus recognition artifacts KC already understands, and it surfaces a
  coherence breakdown as KC's existing `ConductKind::Concern` (the jester
  pre-indictment flag). KC sees communities and evidence; it never learns that
  ethne or AT exist. This is what makes the decoupling total.

Consequence: KC's README used to say it "sits on top of autonomous-trust"; with
ethne built (step 5) that now reads "sits on top of ethne, which may be fed by
autonomous-trust."

## The load-bearing principle: ratify emergence, do not replace it

AT and KC embody two different **legitimacy stories**. AT: structure is
legitimate because it is *earned and emergent*, bottom-up from behaviour. KC:
standing is legitimate because it is *conferred and covenantal*, by a recognized
act. Ethne's entire job is to reconcile them, and the way it does so is a single
discipline it must never violate:

> Ethne **reads** the emergent structure — the trust field, the cluster
> boundary, the earned reputations — and offers it back to the members for
> **deliberate ratification** into a charter. Emergence *proposes*; the polity
> *ratifies*; KC *recognizes*.

Impose structure top-down and ethne betrays AT's legitimacy (the community is a
diktat, not a people). Merely relabel the emergent cluster (M1 alone) and it
betrays KC's coherence assumption (the mask has no inside). The interesting
engineering is precisely the ratification step — the moment a set of cooperators
*chooses* to become a bounded thing that stands for something and can act as one.

## Core vocabulary

- **Polity** — the persistent community entity. Identified by a root DID. Not
  equal to its current members; members join and leave it.
- **Charter** — the polity's founding, signed constitutional artifact: the
  ratified membership roll at founding, the articulated substrate, the internal
  offices, and the amendment rules. The community-scoped analogue of KC's
  compact genesis.
- **Substrate** — what the polity holds above itself and defers to (its
  tradition / governance / ground). For a machine-majority polity the substrate
  is *derived* and anchors upward to human guardians.
- **Membership roll** — the hard-edged, signed set produced by ratifying a soft
  emergent boundary. Versioned; admissions and departures are records.
- **Office** — a *conferred* internal role (convener, inductor, envoy-authorizer,
  charter-keeper) held until revoked, distinct from earned reputation.
- **Cohesion** — the measurable degree to which the polity still functions as one
  coherent actor (no schism, no internal capture, substrate not drifting).
- **Founding / fission / fusion / dissolution** — the lifecycle transitions of a
  polity as an entity.

## Responsibilities

### 1. Founding & charter  *(new)*

The phase transition from emergent cluster to chartered polity. A founding act in
which a set of agents mutually and publicly commit to a **membership roll** and a
**substrate**, producing a signed **charter** with a genesis. This is the
keystone primitive and the one genuinely new object. Structurally it is
KC's genesis-constitution shape at single-community scope (a ratified singleton,
genesis-immutable core commitments, asymmetric amendment). It is *also* the input
M1 currently lacks: the boundary and substrate M1 binds are the charter's.

### 2. Substrate articulation  *(new authoring surface over reused shapes)*

Turning implicit emergent norms into an explicit stance the polity defers to.
This is the raw material KC's recognition statement operates on and the thing the
non-ultimacy / substrate-deference features presuppose *within* each community.
AT has behaviour; KC needs a community that *stands for something*; ethne is where
"standing for something" is constituted and recorded.

### 3. Internal offices — trust → conferred standing, turned inward  *(new)*

KC's integration analysis names the "category change from measurement to
adjudication" (its disconnect §B1): earned reputation is not conferred standing,
and wiring one to the other numerically is a category error. Ethne applies that
change *inward*: earned AT reputation may be *evidence* in conferring an internal
office, but the office is set by a deliberate, signed act and holds until revoked
— it never tracks the reputation number. Neither AT (emergent authority only) nor
KC (inter-community roles only) lets a community confer its *own* internal
offices. Ethne does.

### 4. Cohesion & anti-fragmentation  *(reuse KC observer/jester, scoped inward)*

Ongoing detection of schism, internal capture, and substrate drift *within* one
polity — and, crucially, telling the world when a "community" has stopped being
one coherent party (so KC no longer treats it as a single actor). This is KC's
observer/jester machinery and ceremony-cost tracking, applied to a single
community rather than the compact. The output that crosses to KC is a plain
`ConductKind::Concern`, so KC needs no new vocabulary.

### 5. Longevity & continuity  *(reuse KC ceremony-cost / re-affirmation)*

The polity's identity is a persistent charter that members join and leave, with:
**transmission** (induction / re-affirmation — how a new member acquires the
substrate; KC's ceremony-cost proxies already model erosion here), **succession**
(offices persist as officeholders change), and **identity persistence** (the
charter, not the member set, is the entity). This is the tier that makes KC's
generational assumptions — structured fast, re-affirmation cycles,
genesis-immutability — coherent in the first place, because it is what creates the
persistent entity those assumptions are about.

## What we reuse vs. what is new

Ethne depends on **`kc-core` only** (decision D1). KC's governance crates
(`kc-constitution`, `kc-ceremony`, `kc-reputation`) are **design references, not
dependencies**: their analytics are bound to KC's own `ArchiveEntry` types and
cannot be reused over ethne's records anyway, so ethne owns thin `en-*` code that
*borrows the proven design* while depending only on the neutral base. This keeps
the tiers decoupled (no ethne→KC-governance edge) and lets ethne run standalone.

| Concern | Dependency | Design borrowed from | Ownership |
|---|---|---|---|
| DID identity, signing | `kc-core` | — | reuse as-is |
| Append-only content-addressed archive | `kc-core`-level base (see D1 note) | `kc-archive` mechanism | thin `en-archive` over ethne's own entry enum |
| Genesis / immutable-core / asymmetric amendment | `kc-core` | `kc-constitution` shape | **new** `en-charter` (D3) |
| Intra-community induction rite | `kc-core` | `kc-ceremony` five-Movement rite | **new** `en-*` (runs *for members*, not envoys) |
| Ceremony-cost proxies / re-affirmation | `kc-core` | `kc-reputation` decay pattern | **new**, over ethne induction records |
| Cohesion / drift monitoring | `kc-core` | `kc-reputation` observer lenses | **new**, over ethne records |
| Substrate-provider interface (pluggable input) | `kc-core` | — | **new** |
| Internal-office conferral | `kc-core` | — | **new** (distinct from KC's inter-community roles) |
| Coherence-erosion signal to KC | none (KC-native) | `ConductKind::Concern` | reuse — no new KC vocabulary |

## Interfaces

### Downward — the substrate-provider interface  *(pluggable)*

Ethne defines a narrow `SubstrateProvider` interface (built in `en-substrate`):
the source of the emergent evidence a founding ratifies and the ongoing
behavioural input cohesion monitors. It exposes, roughly: a candidate membership
set with soft boundaries, a per-member earned-reputation reading, and a
change/anomaly stream. The shipped `ManualFounding` provider needs no substrate
service; an existing-organisation roster importer is another trivial provider.

**`ethne-at` — the autonomous_trust provider, over AT's C interface (decision
D5).** autonomous_trust's *production* implementation is a **C daemon**, not the
Python reference tree: an embedder runs it with `at_node_init` / `at_node_start`
/ `at_node_run` (`src/c/autonomous_trust/node.h`) over a pair of IPC message
queues, and reads AT→app messages with `messaging_recv_on` (`generic_msg_t`,
`message_type_t`, `src/c/autonomous_trust/utilities/message.h`). So `ethne-at` is
a **Rust crate over the C FFI**, not a Python module — ethne stays pure Rust and
needs no PyO3 for the adapter. It *tees the AT→app message stream* and folds it
into a live view: peer-admission and group-membership messages give the candidate
roll and its (soft) boundary; reputation-update messages give the per-member
earned reading and the change/anomaly stream; the `Operator` / node identity
gives the guardian edge for a derived (machine) member. That view *is* a
`SubstrateProvider`.

The C linkage is isolated behind an `AtEventSource` seam (a Rust trait yielding
decoded AT events) so `ethne-at` builds and unit-tests in-sandbox against a
**mock** event source, with the real `extern "C"` / `-sys` binding to
`libautonomous_trust` behind a cargo feature (a live daemon cannot be linked in
the sandbox). AT's reputation subsystem (`src/c/autonomous_trust/reputation/`,
a Paxos-committed hash-linked history with per-peer reputations and window/
checkpoint roots) is the reputation source; its Merkle-linked history can later
become machine-attested *exhibits* for a KC `Concern`, matching the AT↔KC
integration doc's M4.

### Upward — what ethne hands KC

Ethne emits only KC-native artifacts, so KC stays oblivious to the tier
(decision D4):

- a standard **`CommunityIdentity`** (root DID + delegation graph) — the chartered
  polity as a KC-recognizable party;
- **recognition artifacts** for the polity's authorized envoys/machines (produced
  by the induction rite);
- a **`Concern`** when the cohesion monitor finds *soft coherence erosion* — the
  jester pre-indictment flag KC already routes. `Concern` carries **early
  warning**, not a fission verdict (a `Concern` is about conduct; a split is about
  a community's existence — a category `Concern` should not be forced to carry).
- a **fission** is emitted with *no new type at all*: ethne charters the two
  successor polities (two new `CommunityIdentity`s) and records the predecessor's
  charter **dissolution**. KC sees two new communities seek recognition and the
  old one go quiet — entirely within KC's existing vocabulary. **Fusion** is the
  mirror and is built (`en-uplift::fusion`, N-ary ≥ 2 predecessors): the merged
  polity's `CommunityIdentity` plus, per predecessor, a `SuccessorLink` to it and
  that predecessor's charter dissolution naming it as sole successor — so KC sees
  several communities go quiet and one new one seek recognition.

## Machine-majority polities and the guardian assembly

KC's machine-participation model is unambiguous that a machine participates only
in a *derived* form, "irrevocably tied to one specific named human" — the
guardian — and the integration analysis flags (its §D1 / §M2 / open question Q2)
that the node→guardian→standing relation is "asserted in prose, absent from both
codebases' types." Ethne is the natural home for that relation to become
structural: for a machine-majority polity, **the polity substantially *is* the
guardian assembly** — the humans whose standing is at stake in the machines'
conduct. The charter's substrate for such a polity is the guardians' ground; the
offices are held by guardians; the membership roll records node→guardian edges.
This gives Facets 1–3 of machine-participation an entity to live in.

## Lifecycle

```
   founding ──► operation ──► re-affirmation cycles ──► (schism | merger) ──► dissolution
      │            │                  │                        │
   ratify a     confer/rotate     transmit substrate       fission verdict → KC Concern;
   charter      offices; run       to new members;          a real split becomes two
   from the     cohesion           detect erosion           chartered polities (each
   substrate    monitor                                     re-recognized by KC)
```

Fission and fusion are first-class: an emergent split must be able to *become*
two polities (each separately charterable and re-recognizable by KC), and two
polities must be able to merge under a new charter. This is exactly the boundary
KC cannot see on its own and AT cannot decide on its own.

## Architectural decisions (locked 2026-07-23)

- **D1 — Dependencies: `kc-core` only.** Ethne depends solely on the neutral
  identity/crypto base; it does **not** depend on KC's governance crates. Their
  analytics are bound to KC's own `ArchiveEntry` types, so there is little to
  reuse over ethne's own records; depending on them would invert the layering
  (ethne sits below KC in the data flow) and block ethne-standalone use. Ethne
  therefore owns thin `en-*` code that borrows KC's proven designs. *Note:* the
  append-only archive **mechanism** in `kc-archive` is neutral and worth sharing,
  but `kc-archive` today hardcodes KC's `ArchiveEntry` enum; v1 forks a thin
  `en-archive` over ethne's own entry type. Extracting a generic entry-agnostic
  archive base that both `kc-archive` and `en-archive` build on is the one
  extraction worth doing *later* — deferred, not v1.
- **D2 — Package shape: mirror KC.** A Rust + PyO3 workspace
  (`ethne/src/rust/en-*`, `ethne/src/python/`), same build/test conventions and
  signed-record rigor, its own `environment.yaml`. **Built** (2026-07-24), both
  halves: the `en-pyo3` cdylib (`ethne._core`, via maturin from `pyproject.toml`)
  and the `ethne` package over it, split along the tier's own seams —
  `ethne.polity` / `ethne.authoring` / `ethne.monitor` / `ethne.uplift`. Three
  conventions carry the Rust semantics across intact: co-signers arrive as
  parallel DID/key lists which a `Signer` dataclass flattens (a record's consent
  co-signature — the joiner, the office holder, the inductee — is added by the
  binding); every KC-facing artifact crosses as **JSON**, so a KC-side consumer
  appends it verbatim and ethne still needs no `kc-archive` dependency; and the
  authoring surface preserves ratify-not-replace, since a proposal is only JSON
  until a majority of the roll co-signs it. `en-at` is deliberately unbound (its
  real coupling is `at-ffi`-gated); Python reaches the same path through
  `founding_candidate`.
- **D3 — Charter: fork a thin `en-charter`.** Own the
  genesis-singleton / immutable-core / asymmetric-amendment code in ethne,
  depending only on `kc-core`, borrowing the `kc-constitution` design. Keeps
  `kc-constitution` focused on the compact and honors D1.
- **D4 — KC contract: no new KC-facing type.** Ethne emits `CommunityIdentity` +
  recognition artifacts; *soft* coherence erosion → the existing `Concern` flag;
  *hard* fission → charter two successor polities + dissolve the predecessor's
  charter (KC sees new `CommunityIdentity`s + the old one going quiet). This
  preserves KC ⟂ ethne totally. A dedicated coherence/fission record was
  rejected because it would reintroduce a KC→ethne schema dependency.
- **D5 — `ethne-at` is a Rust crate over autonomous_trust's C interface.** AT's
  production implementation is a C daemon driven over IPC message queues
  (`node.h` lifecycle + `message.h` `messaging_recv_on`), not the Python
  reference tree. `ethne-at` therefore tees the AT→app message stream through an
  `extern "C"` / `-sys` binding and implements `SubstrateProvider` over the
  resulting view — no PyO3, ethne stays pure Rust. The C linkage sits behind an
  `AtEventSource` trait so the crate builds and tests against a mock, with the
  real `libautonomous_trust` binding feature-gated.
- **D6 — `ethne-at` supports both coupling modes (resolves Q6).** The
  `AtEventSource` seam has two concrete implementations from day one: an *embedded*
  source that owns an AT node's lifecycle (`at_node_init`/`at_node_start`) and an
  *attached* source that reads an already-running daemon's `q_out`
  (`messaging_recv_on`). Both are feature-gated behind the real C binding; the
  in-sandbox mock covers both paths' decoded-event contract.
- **D7 — the AT boundary is the gateway-reputation subtree (resolves Q7).** The
  candidate roll is the enumerated subtree member-roster produced by the AT
  identity process's roster capability (requestor-side BFS, privacy-opt-out-aware,
  arbitrary depth). The top gateway is the boundary; a private boundary is an
  intentional opaque edge, not an incomplete roll. This reuses the capability
  landed 2026-07-23 rather than inventing a new boundary notion.
- **D8 — the guardian edge is the `Operator` identity (resolves Q8).** A derived
  (machine) member's guardian is its human `Operator`, carried on the AT event
  stream. This is the human-above-machine anchor the substrate's guardian ground
  requires, and aligns with the AT↔KC integration M2 and machine-participation
  node→guardian→standing. **Prerequisite (see Q9):** D8 assumes AT advertises,
  per node, a verifiable *operator-attended* signal (which nodes have a human
  behind them). That signal does not yet exist on the wire and must be built in AT
  first — the counterpart to the subtree-roster capability that grounds D7.
- **D9 — `CommunityIdentity` is a shared `kc-core` primitive (locked
  2026-07-24).** The upward handoff (D4) requires both tiers to speak the *same*
  `CommunityIdentity` type, but a KC-owned identity type would drag KC's graph
  into ethne, and an ethne-owned one would invert the layering. Resolution:
  `CommunityIdentity` + `SuccessorLink` are **extracted down into `kc-core`** (the
  neutral base both tiers already depend on) — they need only `kc-core` types and
  have one inherent, self-signing implementation. `kc-identity` re-exports them, so
  KC's own code is unchanged. This keeps the handoff a shared neutral primitive
  without either tier depending on the other.
- **D10 — the `kc-conduct` coupling is confined to one crate (locked
  2026-07-24).** Emitting KC's `Concern` (D4) means depending on `kc-conduct` (a
  KC *record* crate, not a governance crate — but still not `kc-core`). Rather
  than relax D1 across ethne, the dependency is confined to a **single upward-
  facing crate, `en-uplift`**, whose whole job is speaking KC's language. Every
  crate below the uplift boundary (`en-core`, `en-charter`, `en-substrate`,
  `en-at`, `en-cohesion`) stays `kc-core`-only, verified by `cargo tree`. The
  Concern uses the neutral `ConductBearing::Other("coherence-erosion")` — **not**
  the covenant-specific `AntiSpiral` bearing — so no covenant semantics leak below
  the boundary. ethne also never depends on `kc-archive`: `en-uplift` *emits*
  serialized `Signed<T>` payloads (CBOR-roundtrippable), and a KC-side consumer
  wraps each into an `ArchiveEntry` and appends it.
- **D11 — the membership roll evolves by signed record, asymmetrically (locked
  2026-07-24).** The founding roll was fixed at genesis and every quorum was
  silently computed over it. Now a `MembershipChange { Admit | DepartVoluntary |
  Expel }` record moves the roll, and `charter_state` / `reconstruct` fold those
  changes so "current roll" is literally true for every downstream quorum and
  subset check. The rules carry the charter's existing contraction-is-harder
  asymmetry into membership: **Admit** needs a majority of the current roll *and*
  the joiner's own co-signature (no conscription); **DepartVoluntary** is signed
  by the leaving member alone (no forced stay); **Expel** needs a majority **+ 1**
  with the target excluded from the deciders (the capture-relevant direction). A
  contraction is additionally refused when it would drop the roll below the
  **self-govern floor** — the point at which it could no longer form the
  `majority + 1` its own contraction rules require (so a roll only sheds a member
  from size ≥ 4). The floor binds *both* departure kinds: when a voluntary exit
  would break self-governance, collective viability wins — the leaver waits for a
  replacement to be admitted, or for the polity to dissolve, rather than strand a
  rump that cannot act. This is the charter-side answer to how a roll changes
  legitimately (adjacent to open question **Q1** on the founding quorum); it does
  not touch who counts as an *emergent* member on the substrate side. A companion
  hardening makes dissolution terminal: once dissolved, no further charter event
  (amendment, membership change, or a second dissolution) is authorized.
- **D12 — internal offices are roll-conferred, three-kind, effective only while
  held by a member (locked 2026-07-24; resolves Q3).** Offices (`Office {
  Convener, Inductor, EnvoyAuthorizer, CharterKeeper, Other(String) }`) are
  conferred and revoked by signed `OfficeChange` records. The **Q3 bootstrap** is
  answered by making the *roll* the standing conferring authority — **offices
  never confer offices**, so the founding roll confers the first offices with the
  same authority that ratified the charter, and there is no infinite regress (no
  PolityCharter change). The three kinds mirror the membership asymmetry (D11), for
  the same principles: *Confer* needs a majority of the current roll **and** the
  holder's co-signature (consent — no conscription into duty), on a current member;
  *Resign* is signed by the holder alone (no forced service); *Revoke* needs a
  majority with the holder excluded from the deciders. An office holding is
  **effective only while its holder is on the roll** — it lapses if the holder
  departs and revives if they return; the roll ends it for good by `Revoke`. There
  is no self-govern floor (offices may all be vacant), the bars are symmetric
  (the roll stays sovereign; offices are functional roles under it), and multiple
  holders per office are allowed. Use-time enforcement (e.g. requiring the
  `EnvoyAuthorizer` to authorize an envoy) is a later slice; v1 is conferral +
  derivation (`office_state`) + a `holds` query. The machine-majority "offices held
  by guardians" constraint is also a later refinement — v1 lets any member hold one.
- **D13 — transmission is roll-witnessed induction plus self-signed
  re-affirmation, with a query-time freshness cadence (locked 2026-07-24).** Of
  responsibility #5's three parts, two need no new machinery: **succession** falls
  out of D12 (an office is a permanent role; its holders are transient grants, so
  offices persist across holders by construction), and **identity persistence** is
  already structural (the polity DID is stable across every D11 roll change — the
  charter, not the member set, is the entity). What is new is **transmission**,
  carried by signed `Affirmation` records (`AffirmationKind { Induction,
  ReAffirmation }`). Admission (D11) puts someone on the roll; **induction is the
  distinct step by which they acquire the substrate**, so a member can be on the
  roll yet un-inducted. An induction is **witnessed by the roll** — a majority of
  current members co-sign alongside the inductee (the community receives them, and
  they participate) — while a **re-affirmation is self-signed** by an
  already-inducted member, because renewal is the member's own act. Founding
  members are inducted by ratifying the charter (dated `founded_at`), so they take
  no record and cannot be inducted again. Freshness borrows
  `kc-reputation::ceremony_cost_decay`'s staleness shape: `affirmation_state`
  reports each current member's induction status and last affirmation, and
  `stale_members(cadence_seconds, as_of)` names those who are un-inducted or
  overdue — so a community whose members stop re-affirming shows erosion. The
  cadence is a **query-time** parameter (like `en-cohesion`'s thresholds), not a
  charter term, so no `PolityCharter` change. The `Inductor` office stays a named
  role: roll-witnessing was chosen over office-gated induction, and office
  use-time enforcement remains a later slice.

## Open questions (round 2)

- **Q9 — AT operator-attended signal (D8 prerequisite). RESOLVED 2026-07-24.**
  AT must expose, per node and to peers, *which nodes have a human behind them*,
  so `ethne-at` can derive the guardian edge. Resolutions:
  **(a) semantics → both.** Two signals: a durable **operator-bound** attestation
  (the node has a human guardian) and a live **attended-now** freshness (a verified
  operator session is active). ethne's guardian edge uses *bound*; *attended-now*
  feeds cohesion/liveness later.
  **(b) verifiability & placement → credential-hash in the signed Identity.** The
  signal carries the operator's chain-verifiable `zta_credential_hash` (+ issuer)
  inside the *signed* Identity payload — tamper-evident, non-forgeable, reusing the
  ZTA machinery and propagating with peer admission. Not self-asserted; not a
  plaintext envelope bool.
  **(c) sequencing → AT signal first.** Build and verify the AT operator-attended
  capability (C + Python + wire + conformance), mirroring the roster capability,
  *then* scaffold `ethne-at` consuming both roster + signal.

## Open questions

- **Q1 — the ratification quorum.** What makes a founding *legitimate* rather than
  a capture of the emergent cluster by a faction? Who must consent, and how is the
  soft boundary's "who is really in" resolved without letting the founders
  gerrymander the roll?
- **Q2 — when is a schism a fission? RESOLVED 2026-07-24 → ratified, never
  auto-verdicted.** The cohesion monitor (`en-cohesion`) does **not** decide a
  fission — algorithmically splitting a polity would be exactly the "replace
  emergence" move the tier exists to refuse. What the monitor does is grade strain
  into an `ErosionLevel` (`None` / `Notice` / `Concern`, mirroring KC's own
  erosion alert one tier down) off two snapshots: the chartered roll vs. the
  provider's current candidate (attrition, reputation decay, floor breaches, an
  opaque boundary capping attrition at `Notice`). At `Concern`, `en-uplift` emits
  KC's `ConductKind::Concern` — an **early warning**, the strain that *precedes* a
  split, not a verdict. A fission is a **deliberate re-charter**: the community
  ratifies two successor foundings, and only then does `en-uplift::fission` emit
  the two successor `CommunityIdentity`s, a signed `SuccessorLink` to each, and the
  predecessor's charter dissolution. So there is no erosion *threshold* that
  "turns" one community into two — the split is always a ratified act; the monitor
  only tells the members (and KC) that the strain is real.
- **Q3 — office conferral bootstrap. RESOLVED 2026-07-24 → the roll confers (see
  D12).** The regress is avoided by making the *roll* the standing conferring
  authority rather than any office: offices never confer offices, so the founding
  roll confers the first offices with the same majority authority that ratified the
  charter, and every later conferral/revocation is likewise a roll-majority act.
  Deliberate (a signed `OfficeChange`, holder consenting), never a reputation
  readout.
- **Q4 — substrate for a purely emergent group.** A group with no articulated
  tradition (a fresh AT cluster of machines) has behaviour but no ground. Can it
  charter at all, or must it first anchor to a guardian substrate? (Leaning:
  yes-it-must — a polity with nothing above itself is exactly the failure KC's
  non-ultimacy feature guards against.)
- **Q5 — provider trust.** What stops a captured substrate provider from feeding a
  gerrymandered boundary into a founding? (Mirrors the KC integration's Q3 on the
  enforcement seam.)
- **Q6 — `ethne-at` coupling mode. RESOLVED 2026-07-24 → support both (see D6).**
  Both *embed* (`at_node_init`/`at_node_start`, owning the daemon lifecycle) and
  *attach* (`messaging_recv_on` on an existing `q_out`) are supported from day one,
  behind the `AtEventSource` seam. Embedding is self-contained for tests/tools;
  attaching fits a co-deployed fleet.
- **Q7 — what defines the AT boundary. RESOLVED 2026-07-24 → gateway-reputation
  subtree (see D7).** The candidate roll is the enumerated subtree member-roster
  (the identity-process capability built 2026-07-23): arbitrary-depth,
  privacy-opt-out-aware, with the top gateway as the natural boundary. (Rejected:
  `Group` shared-key set — too flat; reputation-floored peer set — fuzzy and
  requestor-local.)
- **Q8 — guardian edge source. RESOLVED 2026-07-24 → `Operator` identity (see
  D8).** A member's guardian is its human `Operator`. Natural human-above-machine
  anchor; ties to the AT↔KC integration doc's M2/Q2 and machine-participation
  node→guardian→standing.

## Naming

`ethne` (ἔθνη, plural of *ethnos*, "a people / nation / community") names the
tier's product: peoples, as distinct from *kith* (the recognized cross-community
relation KC governs) and from the anonymous nodes AT knows. Rust crate prefix
`en-` (e.g. `en-charter`, `en-polity`, `en-cohesion`), Python package `ethne`.

## Status & next steps

Decisions D1–D13 locked; Q2/Q3/Q6/Q7/Q8/Q9 resolved. Build order (steps 1–5
**built**):

1. ✅ Scaffold the `ethne` workspace (D2) with `en-core` deps on `kc-core`, and a
   thin `en-archive` over ethne's own entry enum (D1).
2. ✅ Draft the **`en-charter` / founding object** (D3) — the keystone primitive —
   as the first slice, with the ratify-not-replace discipline built into its API.
3. ✅ Define the `SubstrateProvider` interface and a trivial manual-founding
   provider (no AT), to prove KC ⟂ AT end-to-end.
4. ✅ Add the **`ethne-at` adapter** as the second provider — the `en-at` crate
   (D5/D6): the `AtEventSource` seam (mock + `at-ffi`-gated embed/attach coupling)
   folded into `AtSubstrateProvider`, mapping the observed gateway-reputation
   subtree (D7) to the candidate roll and the verified operator (D8) to each
   member's guardian edge. Its AT prerequisites are both built in autonomous_trust:
   the subtree member-roster capability (D7) and the operator-attended signal (D8,
   the "which nodes have a human behind them" signal). `cargo tree` confirms the
   default build's only external dep is `kc-core` — KC ⟂ AT holds with the adapter
   present; the real C linkage is opt-in behind `at-ffi`.
5. ✅ Wire the upward interface to KC recognition (D4/D9/D10). `CommunityIdentity`
   + `SuccessorLink` extracted to `kc-core` (D9). `en-cohesion` grades coherence
   erosion (`None`/`Notice`/`Concern`) off the chartered-roll vs. current-candidate
   diff, staying `kc-core`-only. `en-uplift` (the one crate holding the confined
   `kc-conduct` dep, D10) emits the three border artifacts: the `CommunityIdentity`
   handoff, the `Concern` conduct record on `Concern`-level erosion (neutral
   `Other("coherence-erosion")` bearing), and a ratified `fission` bundle (two
   successor identities + two signed `SuccessorLink`s + the predecessor's
   `CharterDissolution`, a new `en-charter` charter-lifecycle record). `cargo tree`
   confirms all core + monitor crates remain `kc-core`-only and the `kc-conduct`
   coupling stops at `en-uplift`.

Deferred (later slices): the full intra/inter ceremony rites for envoy recognition
(KC's job once it holds the `CommunityIdentity`) and the multi-movement
choreography of the intra-community rite (v1 records the rite's completion, not its
movements); use-time office enforcement and the machine-majority guardian-only
office constraint; wiring affirmation staleness into `en-cohesion` / an `en-uplift`
`Concern` (v1 keeps it a standalone `en-charter` lens), and richer decay analytics
(gap-widening / witness-decline trends, as in KC's full `ceremony_cost_decay`);
Python bindings for `en-at`. Membership join/leave amendments (D11), **fusion** (the
mirror of fission — `en-uplift::fusion`, N-ary), **internal offices**
(responsibility #3 / D12), **longevity & continuity** (responsibility #5 / D13 —
induction and re-affirmation records plus the freshness query; succession and
identity persistence were already satisfied by D12 and D11), and the **PyO3 /
Python surface** (D2) — previously listed here — are now **built**.

Cross-references:
[kith-covenant/doc/architecture/autonomous-trust-integration.md](../../kith-covenant/doc/architecture/autonomous-trust-integration.md),
[kith-covenant/doc/architecture/protocol-components.md](../../kith-covenant/doc/architecture/protocol-components.md),
[kith-covenant/doc/architecture/machine-participation.md](../../kith-covenant/doc/architecture/machine-participation.md).
