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
set with soft boundaries, a per-member earned-reputation reading, a per-member
guardian edge with the liveness stamp of when that guardian was last verified
present, and a change/anomaly stream (**built 2026-07-27 — D16**: `SubstrateChange`
deltas behind a *separate* `ChangeStream` trait, so a provider with no notion of
change is not made to fake one). The shipped `ManualFounding` provider needs no substrate
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

**Built 2026-07-27 (D15).** The relation is structural: a `Derivation` on the
charter says which members are derived (machine) parties and which human each
answers to, ratified at founding (with every named guardian co-signing) and moved
afterwards by `GuardianChange` records. From that one fact both halves follow — a
machine-majority polity refuses an office to a machine with nobody behind it, and a
chartered edge the substrate stops showing is graded as coherence erosion.

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
  **BUILT — attached first, then embedded once AT grew a lifecycle to bind
  (2026-07-30).** *Attached* went in over AT's flat `app_events.h` ABI. *Embedded*
  could not follow, for a structural reason rather than an effort one: the flat ABI
  covered the event stream but not the lifecycle, and both routes into the
  lifecycle are unsafe from Rust. `at_node_init`/`at_node_start` take a
  caller-allocated `at_node_t`, which embeds `logger_t` and `queue_t` **by value** —
  mirroring it means reproducing two opaque C layouts, exactly the
  silent-corruption hazard the flat ABI exists to avoid. `run_autonomous_trust`
  looks flat enough to call directly and is not trustworthy: its header says it
  "blocks until the daemon exits" and returns 0/non-zero, while `node.c` assigns
  the return value to `daemon_pid` and carries on, and the header declares
  `capabilities` as `void *` where the definition takes `capability_t *`. Two
  disagreements between a declaration and its own use, in the one function
  embedding needs. So the missing piece was named as an AT-side flat lifecycle ABI
  rather than faked with a type that always fails — the dead-field pattern D23 and
  KC's `source_commitment` both turned out to be — and then **built there**:
  `app_node.h`, an opaque heap handle, `at_app_node_start` /
  `at_app_node_alive` / `at_app_node_pid` / `at_app_node_stop`.

  `EmbeddedAtNode` composes rather than duplicates: `at_app_node_start` binds and
  assigns `q_in` itself, so the reader inside it is an `AttachedAtDaemon` built with
  `open_existing`, and `Drop` closes the reader before stopping the node it borrows
  from. Three properties of that ABI matter to a Rust caller and are pinned on
  AT's side, because finishing this found all three broken: the arguments are
  actually *used* (a config aliasing the node it initialised was being zeroed, so
  every queue name silently became an AT default and `generate_config` did
  nothing); the strings are **copied**, so a `CString` temporary may drop while the
  node lives (`node.h` keeps the caller's pointers and reads `app_name` at
  shutdown); and a handle means the event stream is *reachable* — a failed inbound
  bind now fails the start and reaps the daemon instead of logging and continuing.
  See autonomous_trust's `doc/architecture/app-peer-carrier.md`, "Owning the daemon
  from another language". **Not** covered in-sandbox: a live start/poll/stop, since
  no daemon can run here (AT's ISSUES.md §2.1.1); what the 40 `at-ffi` tests cover
  is the decoded-event contract over real sockets plus every argument guard.
- **D26 — the two carrier streams are joined here, by uuid, and `None` stays
  honest (2026-07-30).** AT emits peer identity and peer reputation as *separate*
  messages (its own decision, to avoid coupling two independent processes), so
  `en-at` owns the join. Four rules, in `decode::PeerJoin`: the join key is the
  **uuid** while the ethne identity is the **signing key**, so a reputation for a
  uuid never yet observed is *held* rather than dropped — AT's two producers have
  no ordering guarantee; neither message kind may clobber the other's fields;
  `reputation: None` covers both "AT says unrated" and "not heard yet", which is
  faithful because **D18** defines `None` as *no reading at all* and its hazard is
  confusing `None` with *rated low* (AT's 0.2 `PREREP_NEUTRAL`) — that distinction
  is preserved; and an attendance stamp of `0.0`, or any stamp on a node without
  the verified operator bound, becomes `None`. The join is **not** feature-gated:
  the FFI is what needs a C library, the join is where the reasoning lives, so its
  rules are unit-tested in the default build.
- **D7 — the AT boundary is the gateway-reputation subtree (resolves Q7).** The
  candidate roll is the enumerated subtree member-roster produced by the AT
  identity process's roster capability (requestor-side BFS, privacy-opt-out-aware,
  arbitrary depth). The top gateway is the boundary; a private boundary is an
  intentional opaque edge, not an incomplete roll. This reuses the capability
  landed 2026-07-23 rather than inventing a new boundary notion.
  **CORRECTION (2026-07-29, on building the AT carrier):** the subtree roster
  cannot supply a *key-identified* member. Its entries are
  `{uuid, nickname, address}` (`_roster_add_member`) — no signing key — so it
  cannot feed `AtPeer.signing_pubkey`, and a member with no key has no
  `did:key` and cannot be charted. The live feed's boundary is therefore this
  node's **admitted peers** (`protocol.peers[]`, which are `public_identity_t`
  and do carry `signature.public`). D7's subtree remains the right boundary
  *notion* and is still reachable offline via a synthesized/captured view
  (`MockEventSource`), but a live roll is one-hop-scoped until the roster
  carries keys — which is an AT wire change, not an adapter change. The
  opaque-edge signal (`BoundaryPrivate`) is consequently not emitted live
  either: it is only meaningful alongside the subtree view.
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
  holders per office are allowed. Use-time enforcement — v1 was conferral +
  derivation (`office_state`) + a `holds` query, and nothing consulted them — is
  **built as of 2026-07-27; see D14**. The machine-majority "offices held by
  guardians" constraint **is now an authority rule, as of D15** — the obstacle was
  that the charter log recorded no guardians (they were read from the substrate
  provider only), and D15 removes it by chartering the node→guardian edge. In a
  machine-majority polity a conferral refuses a derived member with no guardian.
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
  overdue — so a community whose members stop re-affirming shows erosion.
  **Built 2026-07-27:** `en-cohesion` grades that list as a fifth erosion
  component, calling `stale_members` rather than restating the rule, and — unlike
  the absent-guardian component — it may reach `Concern`, because duration is
  already inside the measure: a member a whole cadence past re-affirming is not
  merely away from the console. An absent `AffirmationState` means *not evaluated*,
  never *all fresh*. The
  cadence is a **query-time** parameter (like `en-cohesion`'s thresholds), not a
  charter term, so no `PolityCharter` change. Roll-witnessing was chosen over
  office-gated induction, and the `Inductor` office was a named role only —
  **until D14 gave it a defeasible presiding requirement over the rite
  (2026-07-27), which does not displace the witnessing rule but adds to it**.

- **D14 — offices bite at use time, in two different ways (locked 2026-07-27).**
  D12 built offices as conferral plus derivation; nothing consulted the derivation,
  so an office was a label. Two of the four now carry authority into the records
  themselves, and deliberately *not* by the same mechanism:
  - The **`Inductor` presides** — a **defeasible gate**. Where the office is
    effectively held, one of its holders *other than the inductee* must be among an
    induction's witnesses; the roll may still induct without them, but only at a
    majority **+ 1** (D11's asymmetric-bar move). So an officer can raise the bar
    for the rite and can never block it. The cost is stated rather than hidden:
    conferring an `Inductor` *does* raise the roll's own bar for inducting without
    them, majority → majority + 1. The zero-cost alternative — recording who
    presided without requiring it — was considered and declined as no teeth at all.
  - The **`EnvoyAuthorizer` holds a delegated power** — an **alternative route**,
    not a gate. One holder's signature stands in for the roll's majority on an
    `EnvoyAuthorization`; because the majority route always stays open, filling the
    office costs the roll nothing and lets the officer stall nothing. The officer
    may not be the envoy on that route, so the office is never a unilateral
    self-appointment as the polity's voice; the envoy always co-signs (a duty is
    not conscripted). What use-time enforcement means concretely: a record taking
    the officer route must name someone who **actually held the office at that
    point in the log**, which `reconstruct`'s per-index office state already knew.
  `Convener` gates nothing, and gating the roll's own deliberations (admission,
  expulsion, amendment, conferral) on one would put an office in the path of every
  sovereign act, which D12 exists to refuse. **The `CharterKeeper` had no record to
  gate until D17 gave it one** — custody of the log is what that office is, and
  pruning the observation record is the act it now holds a delegated power over
  (bounded by a settling horizon, since destroying evidence is not like authorizing
  an envoy). Self-signed acts —
  re-affirmation, voluntary departure, resignation — are never gated: they are
  precisely the acts no one else may condition.

  The envoy record is the substance of the second half. `EnvoyAuthorization`
  (`EnvoyKind { Authorize, Resign, Revoke }`, mirroring `OfficeKind` for the same
  reasons) names who may speak for the polity, in **ethne's own vocabulary** — the
  record carries no KC delegation scope, so `en-charter` stays `kc-core`-only. A
  standing is effective only while its envoy is on the roll (lapsing and reviving
  as an office holding does) and until its own `expires_at`; the record also carries
  the envoy's public key, checked to derive their DID, because the KC-side artifact
  names its speaker by key. `en-uplift::envoy_delegation` turns an *effective*
  standing into KC's `SpeakerDelegation` scoped `Envoy` — the fourth border
  artifact, and the only one an office produces. It takes the derived `EnvoyState`,
  not the archive, so the border crate still never walks a log. `kc-identity` becomes
  a *direct* `en-uplift` dependency, confined exactly as `kc-conduct` is — and it
  **widens the build graph not at all**: `kc-conduct → kc-ceremony → kc-identity`
  already put it in the tree, so D14 only makes an existing edge explicit.

- **D15 — the node→guardian edge is chartered, not merely observed (locked
  2026-07-27).** The relation KC's integration analysis calls "asserted in prose,
  absent from both codebases' types" now has a type on the ethne side. A
  `Derivation { member, guardian: Option<DidKey> }` marks a member a derived
  (machine) party; founding derivations live in the `PolityCharter` and later ones
  in `GuardianChange` records (`Designate` / `Rotate` / `Release`). `guardian: None`
  is a **real chartered state** — a machine with nobody behind it — and not a
  missing value; recording it honestly is what gives the office rule something to
  refuse.
  - **Ratified, not read.** The provider *observes* the edge
    (`MemberCandidate.guardian`); `FoundingCandidate::propose` now carries it into
    the proposal instead of discarding it, and only ratification makes it a fact.
    That is the same move the roll itself undergoes, so the guardian edge obeys
    ratify-don't-replace rather than becoming a provider assertion the polity is
    stuck with.
  - **Consent, everywhere.** A named guardian co-signs the founding, a
    `Designate`, and a `Rotate` (the *incoming* one) — answerability for a machine
    is accepted, never assigned, which is KC's "irrevocably tied to one specific
    named human" read as a *bilateral* tie. Symmetrically nobody is held to it: a
    `Release` needs either the departing guardian's own signature or a majority of
    the roll recording that the human is simply gone. A `Release` clears the *edge*
    and keeps the designation — a machine does not become human by losing its human.
  - **No machine guards a machine**, at founding or on rotation, and nothing guards
    itself; otherwise the human-above-machine anchor closes into a loop.
  - **The shape follows the tier's grain, not the charter's roll.** `members` stays
    `Vec<DidKey>`. Designation is a separate record with a separate derivation, as
    D11's admission, D12's offices, D13's induction and D14's envoy standing all
    are — this tier already distinguishes "on the roll" from "has property X", and
    restructuring the roll into typed members would have rippled through every
    signature that takes one for a guarantee the provider reading cannot give anyway.
  - **What follows.** `guardian_state` derives the effective picture (designations
    lapse while their member is off the roll, exactly as office holdings do) and
    answers `is_machine_majority(roll_len)`. The office rule (D12) refuses a
    conferral on an unguarded derived member **where the polity is machine-majority
    at that point in the log** — a mostly human polity is untouched, because the
    rule exists for the polity that substantially *is* its guardian assembly, not
    because machines are second class. `en-cohesion` gains a sixth component: a
    chartered edge the provider no longer shows, counted only over members it still
    observes, so an absent member is attrition and an opaque boundary cannot
    masquerade as answerability collapsing.
  - **Two honest limits.** (1) A machine that was never designated reads as human;
    the designation is only as good as the provider's reading, which is **Q5**, not
    something a founding can settle. (2) **CLOSED by D18.** `MemberCandidate` could not
    *express* "derived but unguarded" — a `guardian: None` reading meant human, or
    machine with nobody, or a provider that cannot tell — so `propose` ratified only
    *guarded* machines and the unguarded state had to be designated deliberately
    after founding. `MemberCandidate::derived` now carries the substrate's reading of
    *kind* independently of the edge, and `propose` filters on it.

- **D16 — what was seen is recorded, so that duration can be judged (locked
  2026-07-27).** Every measure before this one read a *snapshot*, which is why the
  attendance component had to be capped: a night shift, a weekend and an abandoned
  polity are the same picture. D16 gives the polity a memory of its own observation
  and grades four things that only exist over time.
  - **The stream.** `SubstrateChange` deltas — appearance, vanishing, reputation
    move, guardian-edge move, an advancing attendance stamp, boundary opacity —
    drained through a `ChangeStream` trait kept **separate** from `SubstrateProvider`,
    because `ManualFounding` is a convener with a list and *an empty stream and no
    stream are different claims*. Deltas rather than a diff of two candidates: the
    provider is the one place the prior reading is already in hand, and two equal
    snapshots either side of a flap look like no change at all.
  - **The record.** `CharterEntry::Observed(Signed<Observation>)` carries what one
    monitor saw at one instant — the readings in full, not the deltas, because a
    duration measure needs the stamps and a self-describing record is what makes it
    evidence. An observation **asserts nothing about the polity**: it is perspectival,
    in the sense KC's observer reports are, so there are no deciders, no majority, and
    the observer **need not be a member** — a monitor that had to be inside the roll
    could be silenced by expulsion. The one rule is that it must be signed by exactly
    the observer it names: nobody may put an observation in another's mouth. (This is
    the entry `entry.rs` anticipated when it said a later slice would fold cohesion
    records in. It also cost `CharterEntry` its `Eq` derive — an observation carries
    float reputations, and a total equality over those would be a lie.)
  - **Sustained absence, the exception that proves the cap.** A guardian unattended
    across the recorded window for at least `sustained_absence_seconds`, *and* seen by
    at least `sustained_absence_readings` observations, reaches `Concern`. Two
    conditions doing two different jobs: **the span is the absence; the count is the
    evidence that somebody was watching while it lasted.** One stale reading with a
    long silence behind it says nothing about the polity — only that nobody looked.
    The snapshot cap is unchanged and still absolute; what lifts it is not a looser
    threshold but a record. A member the window never saw is *not evaluated*, never
    assumed absent.
  - **Three trends that needed no stream at all.** Gap-widening, witness-decline and
    roll churn are derivable from the polity's own signed records — `Affirmation`
    carries its instant and its witnesses, `MembershipChange` its instant — so
    `decay_trends` reads them with no provider involved, borrowing the shape of
    `kc_reputation::analytics::ceremony_cost_decay` one tier up, hedges included: a
    witness decline needs two inductions (never a trend from one point) and a widening
    gap is judged against the **median** of prior gaps, so one aberrant interval does
    not read as decay. The two booleans **warn but never escalate** — a trend is a
    leading indicator, and the lapse it leads to is already escalatable by the
    affirmation component, so grading both to `Concern` would count one decay twice.
    Churn, being a rate rather than a trend, carries a full pair.
  - **`CohesionInputs`.** With four charter-side read-models the report's positional
    signature became four adjacent references of the same shape; they now live in one
    struct of `Option<&T>` with `Default`, which also puts *not evaluated* in the type
    instead of in a convention repeated by hand at every component.

- **D17 — the log may forget, but only by saying so (locked 2026-07-27).** D16 left
  exactly one thing owed: nothing pruned the observation record, and nothing persisted
  it incrementally. Both are measurable rather than theoretical. One observation is
  1.7 KB of JSON for a five-member polity and 5.6 KB at twenty-five, so at the
  attendance cadence a twenty-five-member polity writes roughly **557 MiB a year** —
  D16's "no change ⇒ no entry" bounds that below the poll rate, but an attendance stamp
  advances on every re-announce, so an attended fleet sits near it. And `en-archive`
  had no persistence at all: it existed only in the Python extension module, as a
  whole-log re-serialization, so a Rust consumer could not save a log and a Python one
  saving per observation paid for the whole archive every time.
  - **Why observations, and only observations.** `reconstruct` matches an observation
    and does nothing with it: *an observation changes no state.* Every other record
    here governs something, so dropping one would rewrite what the polity decided;
    dropping an observation cannot. **Observations are the one inert entry class,
    which is exactly what makes them the one prunable class** — the argument is
    structural, not a matter of taste about which data is precious.
  - **A recorded act, not a sweep.** `ObservationPruning` names a horizon and carries a
    receipt — how many records went, and a digest over their CIDs. A Concern crossing
    to kith-covenant rests on observations, so evidence that could vanish silently
    would make D16's reading count — the *evidence somebody was watching* — impossible
    to falsify, and leave an auditor unable to tell "nobody looked" from "we deleted
    it". The receipt is checked **conditionally**: while the observations are still
    present the record must say truthfully what it drops, but once compaction has run
    they are gone, and a rule that recomputed regardless would report every historical
    pruning as a violation. So the log verifies what it still can and treats an applied
    pruning as beyond its power to check rather than as a lie. The receipt keeps its
    force for anyone holding an earlier copy.
  - **The CharterKeeper's teeth, bounded.** One effective `CharterKeeper` among the
    deciders suffices — the delegated-power shape D14 gave the `EnvoyAuthorizer` — but
    **only beyond `PRUNE_SETTLING_SECONDS` (30 days)**; the roll may prune anything at a
    majority. The asymmetry is the point: destroying evidence is precisely the move a
    captured party makes, so the recent record stays out of any single custodian's
    reach while the majority route keeps the office from stalling anything. That
    constant is deliberately **law rather than a threshold** — every other cadence here
    is a query-time parameter because it is policy, but a floor the party it constrains
    could lower is not a floor.
  - **Pruning may only ever weaken a signal.** The window carries the `pruned_before`
    horizon and `unattended_span` clamps to it, because behind the horizon the log
    holds no evidence and no absence may be claimed across it. This is not a
    precaution. `last_attested` is a *max* over the window's stamps, so dropping the
    interval that held the freshest one lowers that max, starts the absence earlier and
    makes it **longer** — pruning manufacturing a Concern. Building the slice surfaced
    that the clamp is only sound given something D16 never enforced: **an attendance
    stamp may not postdate the observation reporting it.** A monitor cannot have seen
    an attestation that had not happened yet, and with every stamp preceding its own
    look, a dropped observation can only have carried stamps from before the horizon.
    That rule is now part of the observation record's own authority pass, and D17's
    invariant rests on it.
  - **Compaction, not mutation.** `apply_prunings` returns a *new* archive. `en-archive`
    stays append-only with no removal path, because giving the neutral mechanism one
    would hand it to every entry class that ever lives there; the knowledge that only
    observations may go belongs with the domain. A single predicate, `is_prunable`, is
    what the authoring receipt, the authority check and the compaction all read, so
    they cannot disagree about what was dropped — and it is where the guarantee lives
    that a pruning can never reach a governing record.
  - **Persistence.** A versioned dump/load moves down into `en-archive`, generic over
    the entry type as the rest of that crate is, plus `dump_json_from(start)` /
    `append_json` so a host holding a cursor appends what is new instead of rewriting
    the log. The Python surface now emits the envelope and reads either it or the bare
    array earlier builds wrote.

- **D18 — the candidate says what the provider *knows*, not just what it saw
  (locked 2026-07-28).** `MemberCandidate` was lossy about its own provider's
  epistemic state, and three separate findings turned out to be that one defect:
  `guardian: None` conflated human / machine-with-nobody / cannot-tell (D15's second
  honest limit), `attested_at: None` conflated nobody-there / cannot-confirm, and a
  bare `reputation: f64` conflated **unrated** with **rated low**. D16 had already
  made this move one layer up — `CohesionInputs` became a struct of `Option<&T>` so
  that "not evaluated" lives *in the type* rather than in a convention
  hand-implemented three times — and D18 gives `MemberCandidate`'s own fields the
  same treatment.
  - **Why the reputation conflation was a live hazard, not a tidiness complaint.**
    autonomous_trust returns its `PREREP_NEUTRAL` (0.2) both for a peer it knows
    nothing about *and* as a score a peer can genuinely earn, while `en-cohesion`'s
    `reputation_floor` (0.3) means *this member is failing*. Measured before the
    change: a four-member roll of unrated members graded **`Concern`** — the level
    that crosses to kith-covenant — on day one, purely for being new. Unknown and
    failing were literally indistinguishable: the same input, the same grade.
  - **`reputation: Option<f64>`.** `None` is *unrated*. Both reputation components
    are computed over the members that carry a reading, with `rated_members` as the
    denominator, and report nothing when nobody is rated — the same "no reading means
    not evaluated, never healthy" shape the transmission and guardian components
    already used.
  - **An eleventh component: the unrated share, capped at `Notice` by
    construction.** There is deliberately no `unrated_concern`, exactly as there is
    no `unattended_concern`, and for the same kind of reason: a cohort too young to
    have been rated and a provider that has stopped rating are the same picture from
    a snapshot. What it exists to prevent is the two components above going
    *silently* dark — they report nothing when nobody is rated, and something has to
    say that they are not reporting.
  - **`derived: bool`, independent of the guardian edge.** `propose` filters on it
    instead of inferring derivation from `guardian.is_some()`, so a machine the
    substrate reads as having nobody behind it is ratified as the unguarded
    `Derivation { guardian: None }` the charter has always understood. **No
    en-charter change was needed** — the charter could already express the state; only
    the candidate could not propose it. That makes D15's machine-majority office rule
    non-vacuous *at founding* rather than only after a deliberate designation.
  - **An unrated candidate does not satisfy a demanded floor.** A positive
    `min_reputation` is a positive assertion and `None` cannot support it; a floor of
    `0.0` demands nothing and admits everyone. Behaviour-preserving at both ends: a
    provider that used to collapse "unknown" into a low number saw those members
    trimmed by any floor above it, and still does.
  - **The AT mapping is structural, which was a finding rather than a choice.** In
    AT's model there is no such thing as a human peer — a peer *is* a node — so every
    AT candidate is `derived: true` and what varies is `operator_bound`, i.e. whether
    the guardian edge is there. A node with no operator is precisely the unguarded
    machine; before D18 it proposed silently as a human member.
  - **The honest limit that remains.** `reputation: None` is not yet reachable from a
    live node: AT collapses "no bilateral history" into `PREREP_NEUTRAL` at its own
    API boundary, and "exactly 0.2" is not a safe inference because a peer can earn
    it. Exposing the distinction is a named **AT-side prerequisite**, sequenced the
    way Q9 sequenced the operator signal — build the AT half first. The type is honest
    now; the feed is not yet there.

- **D19 — a retention policy, because a horizon chosen by hand is chosen blind
  (locked 2026-07-28).** D17 left exactly one thing owed: it gave the polity the *act*
  of pruning and nothing that says when. A host wanting to prune had to invent a
  horizon, and getting it wrong is silent in both directions — too aggressive and it
  clips the evidence a kith-covenant Concern rests on, too timid and the log grows
  without bound. Measured on the marginal bytes an observation adds to the serialized
  log, with readings across the whole roll: **3.4 KB at five members, 4.6 KB at ten,
  8.0 KB at twenty-five**, so at the attendance cadence a twenty-five-member polity
  writes **~803 MiB a year** and an attended fleet sits near that rate, because a
  stamp advances on every re-announce.
  - **"Scheduler" is the wrong name for what was owed.** This tier's libraries never
    call `now()` — every cadence is a query-time parameter (`stale_members(cadence,
    as_of)`, `Thresholds`, `cohesion_report(as_of)`) — so the loop and the clock
    belong to the host exactly as the storage backend does. What does *not* belong to
    the host is which history a live measure still needs and which authorization route
    a given horizon leaves open. That is tier knowledge a host cannot be expected to
    reconstruct, so the deliverable is a **pure read-model**:
    `prune_recommendation(archive, polity, &RetentionPolicy, as_of)`. It signs nothing
    and appends nothing; the host feeds the horizon it returns to D17's ordinary
    authoring path. *Emergence proposes; the polity ratifies* applies to the polity's
    own housekeeping too.
  - **The recommendation can only ever propose what the authority pass accepts.**
    That is the whole reason it exists. Five conditions, each deferring to the rule
    that already owns it rather than restating it: the polity must be chartered and
    undissolved; the horizon is floored twice (below); it must **advance** past
    `effective_prune_horizon`, since D17 rejects a receding one; and at least
    `min_dropped` observations must actually fall before it. Any failure yields
    `None`. Swept across retentions spanning the settling boundary, every horizon it
    named was accepted by `check_authority` on the cheapest route it named.
  - **The safety floor is a duration *and* a count — and the second half was a
    finding, not a design.** The slice was specified with one floor,
    `live_measure_seconds`, on the reasoning that the longest observation-based
    measure is `sustained_absence_seconds` (one day). Measurement refuted it. A
    sustained absence is graded on **two** conditions, and D17's clamp only protects
    one: the span survives, because the window clamps it to the pruned horizon, but
    the **witness count** does not. The same absence, the same 24-hour horizon, graded
    `Concern` at a dense observation cadence and `Notice` at a sparse one — four
    readings falling to two, one short of `sustained_absence_readings`, with nothing
    saying that evidence rather than conduct is what changed. So `min_witness_readings`
    (default 3, mirroring the threshold it protects) holds the horizon back to leave
    that many readings of **every watched member** standing. Per member, because
    `readings_for` is per member: a member the monitors mention rarely has thin
    evidence, and thinning it further is exactly the clip this read-model exists to
    refuse. A thinly-watched member can therefore *pin* the horizon indefinitely.
    That is the correct answer and not a deadlock — D17 always leaves the roll free to
    prune anything by majority. What is refused is *recommending* it.
  - **Age-only retention, and the default is the cheap one by coincidence worth
    asserting.** `retain_seconds` defaults to `PRUNE_SETTLING_SECONDS` (30 days): the
    smallest value that simultaneously preserves every observation-based measure and
    keeps the **single-`CharterKeeper`** route open, since that route requires
    `horizon <= pruned_at - PRUNE_SETTLING_SECONDS`. A *shorter* retention keeps more
    history and is therefore more expensive to authorize, not less — which is
    counter-intuitive enough to be a test rather than a comment. It also happens to be
    the cheap one to store: a keeper-authorized pruning is ~2.9 KB with its one
    signature, where a roll-majority one carries a majority of them (5.8 KB at five
    members, 20.2 KB at twenty-five). At the default the log is bounded — 8,640
    observations retained, **28 MiB at five members and 66 MiB at twenty-five** — so no
    count- or size-based retention rule is needed and none is built.
  - **`min_dropped` exists because a pruning record is itself unprunable.**
    Observations are the one entry class a polity may drop; the records that drop them
    are governing records and stay forever. Run as a control — the same simulated year
    through a host loop waking daily — pruning at `min_dropped=1` authored **335
    permanent pruning records and left a 1.27 MB log**, against **13 records and 435 KB**
    at the default of 100. Eager housekeeping made the log **2.9× larger**. The default
    keeps the permanent residue under ~3 MiB/year even at the fastest realistic
    observation rate, and stays small enough to fire promptly on a slow cadence, where
    a large floor would let the log run well past its retention target before anything
    triggered.
  - **The recommendation names its authorization route**, so the host knows whom to
    ask: `PruneRoute::CharterKeeper { holders }` — from `office_state`, already
    intersected with the roll, so a departed keeper is never offered — and/or
    `PruneRoute::RollMajority { need }`. Both are reported when both are open, because
    the officer route is an *alternative* to the roll and never a gate on it.
  - **Not graded in `en-cohesion`.** An unpruned log is a housekeeping fact, not
    evidence the polity is losing coherence; the eleven components stay about the
    polity's own life. The **accepted risk**, recorded rather than mitigated: a polity
    that leaves `CharterKeeper` vacant and never prunes accumulates silently, and only
    whoever runs the loop will see it.
  - **What remains the host's**, unchanged from D17: a storage backend (file/SQLite)
    and the loop itself. D19 closes the horizon question, not those.


- **D20 — storage, and the loop (locked 2026-07-28).** D17 and D19 each closed with
  the same deferral: no storage backend ships, and nothing runs the loop. Taking it
  on turned up that the first half was not a gap but a **defect**. D17 shipped
  incremental saving (keep a cursor, write the segment past it) and compaction
  (`apply_prunings` returns a new archive with observations removed), and *those two
  do not compose*. Measured on the real surface: a host saving per observation and
  then pruning reloaded **12 entries instead of 6, with 7 pruned observations
  resurrected**.
  - **Why it was invisible, which is the part that matters.** The reloaded log passed
    `verify_all` *and* `check_authority` — the pruning receipt re-verified, because
    every CID it named was back. And the mechanism was a documented convenience:
    `dump_json_from` treated an index past the end as "already current" and returned
    an empty segment, so after a compaction (cursor 12, new length 6) the save wrote
    **nothing** and the `ObservationsPruned` record never reached storage at all. With
    it went the window's `pruned_before`, so `unattended_span` read **300 days instead
    of 230**. A storage bug *lengthening* a measured absence is precisely the
    Concern-manufacturing failure D17's clamp exists to prevent, re-entering by a
    route the clamp does not cover.
  - **Generations.** An index is only meaningful within an append-only run, because
    compaction removes entries from the *middle* — post-compaction index N is a
    different entry. So the archive carries a `generation`, advanced by
    `Archive::next_generation` (one caller: `apply_prunings`) and never by an append,
    and a cursor is `(generation, index)`. `dump_json_since` refuses a cursor from
    another generation or past the end; `append_json` refuses a segment whose
    generation or start position is not the next piece of *this* log. Envelope version
    2 carries both. Merging two independent logs into one view is a genuinely
    different operation and now says so — `merge_json`, which the KC-side
    several-communities reader wants and persistence never does.
  - **`en-store`: the branch is one line, and that is the crate.** *Same generation ⇒
    append a segment; generation changed ⇒ rewrite in full.* The store owns the
    cursor, because the whole hazard is what happens when a caller keeps one. An
    `ArchiveStore` trait generic over the entry type (as the rest of the archive
    machinery is, D1 — so the crate needs no `en-charter` dependency at all), with
    `FileStore` (one segment per line; append is write+fsync, rewrite is
    temp+fsync+rename, which POSIX makes atomic) and `SqliteStore` behind a `sqlite`
    feature. Both persist *the same segment envelopes*, so they are differentially
    testable and neither can drift into its own idea of a stored log; SQLite earns its
    place on the rewrite, which is one transaction rather than a temp-file dance.
  - **A torn tail is named, not swallowed.** A crash mid-append leaves a partial final
    line. Loading reports it; `recover()` truncates it as a deliberate, reported act.
    Silently skipping an unparseable last line would discard a signed entry without
    saying so — the same shape of failure as the bug above. The log may forget, but
    only by saying so.
  - **`en-host`: `tick` decides and computes; the host signs and does I/O.**
    `HostCycle::tick(archive, candidate, as_of)` returns the `DueAction`s owed —
    persist, a graded `CohesionReport`, a `RaiseConcern` when it reaches that level, a
    `PruneRecommendation` when the log is due. It takes `as_of` like every other
    cadence in this tier, so a year of host behaviour runs in microseconds at times
    chosen to land on cadence boundaries, and it **authors nothing**: raising a
    Concern and pruning both need signing keys the host holds. Emergence proposes and
    the polity ratifies, applied to the host's own duties. It covers the cadenced
    duties and only those — membership, induction, office conferral and envoy
    authorization are deliberate acts, and nothing schedules them.
  - **One clock, and it is not a library.** `src/rust/examples/polity_host` is a
    `[[bin]]`, mirroring KC's `two_communities_demo`; nothing depends on it. Its
    single `time::now()` is the only clock read in ethne outside a test, apart from
    `en-pyo3`'s pre-existing `*_at=None ⇒ now` binding convenience. It runs a
    simulated timeline by default so it terminates and can be verified, and
    `--follow` is the real loop.
  - **Measured.** With the whole log at 5,000 entries an append writes **67 bytes**
    against a rewrite's **63,954** — 955×. The measurement also caught a real defect
    in the claim "each save costs what is new": `dump_json_from_to` reached the tail
    with `iter().skip(start)`, walking and index-looking-up every skipped entry, so an
    append to a 5,000-entry log burned **~5.4 ms of pure CPU**. Slicing the order
    index instead (`iter_from`) makes it flat at **~14 µs**, independent of log size.
  - **The honest limits.** `SqliteStore` is ethne's first C dependency
    (`rusqlite/bundled`); it is off by default and `cargo tree` confirms the default
    workspace graph is unchanged and still `kc-core`-only. Concurrent writers are a
    host error — two stores over one file fail loudly rather than interleaving, but
    nothing *locks*. And `en-host` is Rust-side only: a Python host gets `FileStore`
    and the read-models, not the cycle.

- **D21 — a founding says what it excluded (locked 2026-07-28, resolving Q1).**
  The genesis authority rule asked for a majority of `p.members.len()` — the
  *proposal's own roll* — and `FoundingCandidate::propose` discarded the candidate
  it derived that roll from. The charter had no referent outside itself. Measured
  against a thirty-node substrate: a faction of one chartered the cluster as a
  polity of itself, on one signature, passing both `verify_all` and
  `check_authority`; the exclusion of the other twenty-nine was not merely
  unpunished but **unrecorded**. The majority bar was vacuous because the faction
  picked its own denominator.

  The resolution splits the problem in two, along the seam the tier already uses
  elsewhere — *the log may forget, but only by saying so; a founding may exclude,
  but only by saying so*:

  - **The accounting is gated.** `FoundingProposal` and `PolityCharter` carry a
    `BoundaryAttestation`: the `candidate_size` the boundary showed, and an
    `Exclusion { member, reason }` for everyone it showed that the roll does not
    name. Genesis authority refuses a charter whose numbers do not add up
    (`candidate_size == members + exclusions`), whose exclusions repeat, or that
    excludes someone it also enrolls. Because the charter's core has no amendment
    path, the attestation is genesis-immutable by construction.
  - **The judgement is only graded.** Nothing refuses a charter for being *narrow*.
    A genuinely separate group of three inside a thirty-node cluster is
    indistinguishable from a faction capturing it without a substantive judgement
    about the tradition — the same shape as the attendance cap's inability to tell a
    night shift from an abandoned polity (D8/Q9). What the rule removes is the
    silence, not the discretion.

  `founding_legitimacy(charter)` is a read-model over the charter **alone** — no
  provider, no cohesion input — so anyone holding the charter can re-run it forever.
  It reports `consent_share`, **the single number Q1 asks for**: ratifiers over
  everyone the boundary showed, rather than over the roll the founders chose. Its
  `FoundingBasis` (`Corroborated` / `Narrowed` / `SelfAttested`) is deliberately
  **not** ordered and **not** an `ErosionLevel` — a founding is not eroding, and
  these are not comparable harms.

  `BoundarySource` rides on `FoundingCandidate`, not on `propose`. The provider is
  the only party who knows whether a boundary is an independent reading or a
  convener's word, and a `ManualFounding` candidate has exactly the same *shape* as
  a trust service's, so the distinction cannot be inferred downstream. This is D18's
  move applied to the roll — put the epistemic state in the type — and the codebase
  already drew the line informally: `ManualFounding` gives asserted members
  `Some(1.0)` rather than `None` precisely because *a convener vouching for a roll is
  the reading here*. A convener-asserted founding still charters; the charter simply
  never pretends it was corroborated, which is the same honest chartered state as an
  unguarded machine's `guardian: None` (D15).

  `propose` now records whoever the reputation floor trims, with the reason it
  actually used — distinguishing "reputation 0.21 below the floor 0.30" from
  "unrated; a floor of 0.30 demands a reading" (D18's semantics, stated where an
  excluded party can read them). `propose_selecting` is new, for the *deliberate*
  narrowing: if the only ergonomic path were the floor, an honest small founding
  would be dressed up as a floor it is not, or done by hand with an attestation the
  convener writes themselves.

  *Judgement call, flagged:* `FoundingLegitimacy` carries the exclusion **list**,
  not only the count, though a Rust caller holding the charter could read it off the
  attestation. A grade reporting "29 excluded" without their names would reinstate,
  one level up, the exact silence D21 removes — and it is the only way the excluded,
  or anyone auditing for them, can read the reason off a surface that is not Rust.

  **Measured**, against a thirty-node substrate — a founding narrowed honestly, and
  the same narrowing asserted on the convener's own word:

  | faction | signed | boundary saw | named excluded | `consent_share` | basis |
  |--------:|-------:|-------------:|---------------:|----------------:|-------|
  | 30 | 16 | 30 |  0 | 0.533 | `Corroborated` |
  |  5 |  3 | 30 | 25 | 0.100 | `Narrowed` |
  |  3 |  2 | 30 | 27 | 0.067 | `Narrowed` |
  |  1 |  1 | 30 | 29 | 0.033 | `Narrowed` |

  Before, every row read `chartered` and nothing more.

  **The honest limits.**
  - This is **accountability, not prevention**, and it does not pretend otherwise.
    The attestation is the proposer's *signed claim*. Nothing in the archive can
    check it against the substrate, because the archive has no provider — a
    hand-built proposal can still understate `candidate_size`. What changes is that
    doing so is now an explicit false statement rather than an omission: the
    distinction between negligence and fraud, and what makes a later audit possible
    at all.
  - That residue is **Q5** (a captured provider feeding a gerrymandered boundary),
    which this slice does not close and names as adjacent.
  - The soft-boundary problem is real but bounded here: Q7 resolved the AT boundary
    as the enumerated subtree roster, which is crisp, so "who did the substrate
    show" has a definite answer on the one provider that exists.
  - Adding a required field to `PolityCharter` **breaks deserialization of existing
    dumps**. Acceptable — pre-release, uncommitted, and genesis is a singleton so
    there are no long-lived logs — but it is a real format break, alongside D20's
    envelope v2.
  - Nothing new crosses to KC. D4 holds: a founding defect is not conduct, and the
    border stays at four artifacts.

- **D22 — a boundary says who read it, and the excluded answer back (locked
  2026-07-28, resolving Q5).** D21 recorded *that* a boundary came from a reading.
  It could not say **whose** — `BoundarySource::Substrate` was an unsigned label the
  proposer supplied, so it was evidence of nothing. Measured, and worse than Q5's
  wording: flipping one public field promoted a convener's bare assertion to
  `Substrate`, and the grade with it to `Corroborated`, with no provider ever
  consulted and both verification passes green. **D21's own locked decision #2 —
  that a convener-asserted boundary is labelled so permanently — was unenforced.**
  And since D21's rule checked only that the *numbers* added up, against a
  self-report, any internally consistent fabrication passed.

  Three moves, each in the tier's existing grain:

  - **A — the reading is attributable (authority).** The `Substrate` arm now
    *carries* its evidence: `Substrate { readings: Vec<Signed<SubstrateReading>> }`,
    each reading naming a `reader` and signed by that reader alone (D16's "nobody
    may put an observation in somebody else's mouth"), bound to the founding it is
    offered for (without which an honest reader's account of one cluster replays
    into somebody else's founding while the signature still verifies). Nesting a
    signed envelope is an established idiom in the corpus — `kc-ceremony` and
    `kc-escalation` both carry a `Signed<Recitation>` — and `didkey_resolver`
    recovers the reader's key from its DID, so none of this needs a registry.

    **The rule with teeth is identity, not arithmetic:** the roll ∪ exclusions must
    equal some signed reading's members *as a set*. D21 supplied both sides of its
    own sum; here the right-hand side is signed by somebody else.

    Key custody is unchanged from D20 — the provider holds no keys and gains none
    ("`tick` decides and computes; the host signs and does I/O").
    `FoundingCandidate::reading()` yields an *unsigned* reading and whoever runs the
    provider signs it. **No `en-at` change.**

  - **B — corroboration means readers who agree (a lens, never a gate).**
    `FoundingLegitimacy` gains `readers` and `corroborating_readers` — deliberately
    *not* a count of readings filed, since a reader who signed a different boundary
    corroborates nothing, and the gap between the two numbers is a reader
    contradicting the founding on its own charter, permanently. `Corroborated` now
    requires at least one agreeing attributable reading, where before the label
    alone sufficed. This stays inside `founding_legitimacy`, so D21's property
    survives: the grade still derives **from the charter alone, forever**.

  - **C — the excluded answer back.** An eleventh entry,
    `BoundaryDissented(Signed<BoundaryDissent>)`, over
    `DissentKind { Unnamed, ReasonRejected }`. Perspectival evidence in D16's exact
    shape: signed by the dissenter alone, no deciders, no majority, and the
    dissenter need not be a member — one required to sit inside the roll could be
    silenced by never being admitted. **This is the only thing in the tier that can
    contradict an understated `candidate_size`**, which is precisely what D21
    recorded as beyond its reach: the archive holds no provider, so the excluded are
    the only available witnesses.

    Authority checks coherence with the record contested, never truth: a
    `ReasonRejected` dissenter must actually appear in the charter's exclusions (you
    may reject the reason given for *you*), and an `Unnamed` dissenter must appear in
    neither the founding roll nor the exclusions (a record that names them
    contradicts the claim outright). `founding_dissent` is a **separate** read-model
    from `founding_legitimacy` on purpose — dissent lives in the archive and accrues
    over time, so folding it in would destroy the charter-alone property and improve
    neither.

    Two deliberate departures. A dissent may be filed **after dissolution** — the
    only entry class that may, where nine others are blocked — because otherwise
    founders silence the parties they excluded by winding the polity up, which is
    D17's "destroying evidence is the move a captured party makes" in a different
    costume; a founding's legitimacy outlives its polity in any case, since KC holds
    a `CommunityIdentity` derived from that charter. And it is **unprunable for
    free**: `is_prunable` matches only observations, which D17 made the one
    forgettable class precisely because they are inert. A dissent is the opposite of
    inert.

  *The finding that changed the design.* The plan kept a `source` field on
  `FoundingCandidate` for `propose` to copy across. Once `propose` takes the signed
  readings, that field **should not exist**: the evidence *is* the source, empty
  readings meaning a convener's assertion. Keeping both would have left a claim and
  its evidence as two things that could disagree — the very defect D22 exists to
  close, reintroduced one level down. So the sweep across the tier became a
  *deletion* of `source:` lines rather than an edit of them, and `ManualFounding` is
  `ConvenerAsserted` **because it holds no reader key**, not because it is labelled
  so. A convener who genuinely obtains a signed reading matching their roll has real
  corroboration and is welcome to it; what is no longer free is *claiming* one.

  **The honest limits.**
  - Still **accountability, not prevention**. A captured reader that signs a
    gerrymandered boundary still charters one. What changes is that the lie is
    attributable to a named key forever, and that the parties it omitted can
    contradict it on the record — D21's negligence-versus-fraud distinction taken
    one step further, from an unsigned claim to a signed one with an author.
  - **Corroboration is sockpuppet-vulnerable by construction.** `did:key` costs
    nothing, so a captured reader can mint its own corroborators and the count
    cannot tell them from independent ones. Stated rather than engineered around,
    following kith-covenant's capture-move-1 precedent exactly, where a declared
    `interpreter_did` was likewise "just a name the community minted" and the
    plurality count was kept a lens with independence attested separately.
    **Independence attestation was the named successor to D22, and is built as D24.**
  - A dissent asserts; it does not adjudicate. Nothing decides whether a dissenter
    tells the truth, and nothing should — that is the "replace emergence" move the
    tier refuses.
  - **Second format break in two slices**, and the first against a *committed*
    format: `BoundarySource` changes shape and the attestation gains readings. Still
    pre-release and genesis is a singleton, so no long-lived logs — but the honest
    tally is now D20 (envelope v2), D21, D22.
  - Nothing new crosses to KC. D4 holds; the border stays at four artifacts.

- **D23 — the ground a polity stands on (locked 2026-07-28, resolving Q4).**
  Non-ultimacy is the tier's whole reason for having a substrate, and nothing checked
  it. **`substrate.anchor` was read by no rule anywhere** — the only consumers were a
  pyo3 constructor and a getter, and nothing in the corpus had ever set it to `Some`.
  Measured before the slice, four foundings passed both verification passes: a polity
  whose entire ground was two empty strings; a fresh cluster of three machines with
  nobody behind any of them; a polity naming **itself** as its anchor; and one naming
  a member of its own roll. The last two matter most, because Q4 was framed as an
  *absence* problem and they are a **closure** problem: a rule demanding only presence
  is defeated by one assignment. The asymmetry that made the case is in the corpus
  already — the guardian rule refuses an edge that "closes into a loop", and the
  substrate anchor that comment is *about* could point at the polity itself.

  **Four authority rules at genesis, all additive.**
  1. **Articulated** — name and statement both non-empty, for every polity.
  2. **Non-closure** — an anchor may be neither the polity nor anyone on its roll.
     A ground inside what it grounds is no ground.
  3. **Grounded** — a roll of nothing but derived parties, not one of them guarded,
     must name an anchor. The trigger is deliberately *not* machine-majority: the
     statement is the anchor for a human roll, unkeyed on purpose, and a guarded
     machine already has a consenting human above it, which is D15's model of
     legitimate machine participation. The key is demanded exactly where nobody
     present could hold a tradition and nobody stands above one.
  4. **Consent** — a present anchor **co-signs the founding**, joining the ratifiers
     and the named guardians. Answerability is accepted, never assigned; you may not
     conscript what you claim to stand under. This reached the Python surface as a new
     `anchor=` parameter on `ratify_charter`, since the founding path had no way to
     supply the ground's consent at all.

  **Graded, not only gated.** `founding_legitimacy` gains `grounding`:
  `Anchored` (a keyed party outside the roll consented — the only grounding anybody
  signed), `Guarded` (guardian edges doing the work), `Held` (no derived party at all:
  a human community holding an unkeyed tradition) and `Bare` (articulated and nothing
  else — unguarded machines on a mixed roll). Descriptive and unordered, like
  `FoundingBasis`. `Bare` is permitted; it is reported so that *permitted* is not
  mistaken for *grounded*. It belongs on that read-model rather than a new one on
  exactly the test D22 applied to dissent: the substrate is genesis-immutable and sits
  on the genesis charter, so grounding cannot accrue and the answer is the same
  forever.

  **And the ground may answer back.** Once an anchor consents it can change its mind,
  which is the question D15 answered for guardians with `GuardianChange`. A twelfth
  entry, `SubstrateDisavowed`, is the ground's own signed withdrawal — and it gets a
  deliberately weaker answer than a guardian's: it **asserts and cannot un-charter**.
  Nothing rewrites the substrate, reconstructs any authority state, or dissolves
  anything, because a withdrawal with that power would be a unilateral kill switch in
  every anchor's hands — a worse hole than the one this slice closed. A disowned
  polity keeps governing itself, on a ground that disowns it, legibly. The read-model
  is `GroundStanding` (separate, because *this* does accrue), and `en-cohesion` reads
  it as a twelfth erosion component that reaches **Concern** alone: unlike every other
  measure there it grades a single signed act rather than a drift, so it takes no
  threshold — event-triggered, like kith-covenant's `contraction_detected`.

  Rules the disavowal shares with D22's dissent, for D22's reasons: signed by exactly
  its filer, **permitted after dissolution** (the second and only other such class —
  otherwise founders silence their ground by winding up), and **unprunable for free**,
  since `is_prunable` matches only observations. Where it is *stricter*: a dissent must
  admit any filer, because an omitted party is by definition absent from the record,
  whereas the ground is named on the charter — so only that key may withdraw.

  **The honest limits.**
  - **Presence, never content.** No rule can read a tradition, and this one does not
    pretend to. What it refuses is the blank, which is not a tradition anyone could
    have been mistaken about. A polity may still articulate a ground it does not mean.
  - **A consenting anchor is not a *trustworthy* anchor.** The rule buys that somebody
    holding a key agreed to stand there and can be named for it — the D22 stance
    exactly: accountability, not prevention.
  - **A disavowal asserts.** Nothing adjudicates whether the ground is right to
    withdraw, and a ground that withdrew cannot un-say it; there is no amendment record
    for a substrate and this slice does not invent one. Such a polity's honest option
    is the one every polity has: to re-charter, deliberately, as the different polity
    it now is.
  - Nothing new crosses to KC. D4 holds; the border stays at four artifacts — the
    Concern travels on the existing one.

- **D24 — whether a founding's readers were their own principals (locked 2026-07-29,
  closing the residue D22 named as its own successor).** D22 made a reading
  *attributable*; it could not tell one reader from five faces of the same actor.
  Measured: one party minted five `did:key`s, signed five identical readings of the same
  boundary, and the charter graded **`Corroborated`** — the highest basis the tier awards
  — with both verification passes green and no dissent anywhere. And the chorus is
  cheap in a way worth stating numerically: **500 readers minted and signed in 16.2 ms,
  32 µs a voice.** "`did:key` costs nothing" is not a figure of speech, so a count of
  distinct signing readers cannot mean what it appears to mean.

  kith-covenant reached the identical problem for capture move 1 and answered it with
  `InterpreterIndependenceAttestation` plus a lens that counts *vouching* rather than
  self-declaration. The shape transfers directly.

  **The record.** A thirteenth entry, `ReaderIndependenceAttested`, over
  `IndependenceKind { Independent, Controlled }`: a third party's account of a named
  reader of a named founding. Three authority rules — signed by **exactly** the attesting
  party (nobody vouches in another's name), it must postdate the charter, and the reader
  must **actually appear among the founding's readings**. That last is a *gate* where KC
  keeps it a lens, on the line D22's dissent arm already draws: coherence with the record
  spoken about is checkable, the truth of the claim never is. It needed `Reconstructed`
  to carry the founding's readers, exactly as D23 needed it to carry the anchor — the
  subject of the claim is *in* the record even though the claim is not.

  **Who counts, and who is merely recorded.** An attestation from the polity, from the
  reader itself, or from **anyone on the founding roll** does not count toward
  independence: the founders are precisely the interested party, so a founder vouching
  for the founding's own reader is the sockpuppet move one step out. This is stricter
  than KC, which excludes only the community and the interpreter, and it is affordable
  for the same reason D23's rules were — the roll is on the charter. But the record is
  **accepted and reported separately** as an interested attestation rather than refused:
  destroying a claim somebody signed their name to serves nobody, where merely not
  crediting it costs nothing. Gate the signature; grade the counting.

  **The number D22 could not produce.** `independent_corroboration` is the conjunction of
  two different questions: readers who *both* agree with the ratified boundary *and*
  carry a disinterested voucher. A chorus of five hundred minted readers scores `0`. It
  is a separate, accruing read-model rather than a field on `founding_legitimacy`, on the
  test D22 stated and D23 reused: that read-model derives from the genesis charter alone
  and must give the same answer forever, and attestations arrive afterwards.

  **And the lens can be told it is being fooled.** The `Controlled` arm lets a party
  assert a named reader is *not* independent — one enum on the same record rather than a
  new entry class, following D22's dissent and D23's disavowal in letting whoever is
  contradicted speak. It counts **from anyone**, deliberately: interest disqualifies
  *praise*, not *warning*, and a founder breaking ranks to say "that reader is one of
  ours" is the most informative possible source for the claim. A contested reader is
  reported as contested, never as disqualified; nothing here adjudicates.

  **A category, not a third exception.** Permitting this after dissolution makes three
  such classes, and three is enough to name the rule: records that are **evidence about
  the founding** — dissent (D22), disavowal (D23), independence (D24) — outlive the
  polity, because the founding's legitimacy does (kith-covenant holds a
  `CommunityIdentity` derived from that charter), where **governing acts** die with it.
  All three are likewise unprunable for free, since `is_prunable` matches only
  observations.

  **The honest limits.**
  - **An attestation is itself unverifiable, and this does not pretend otherwise.** It
    does not prove independence. It moves the cost of faking corroboration from one key
    (32 µs) to a second party willing to be **named** for the claim, permanently. That is
    strictly more than nothing and strictly less than proof.
  - **A lens, never a gate.** Nothing refuses a founding for having no attestations,
    because a young honest founding has none and refusing it would be refusing newness.
    This is precisely where kith-covenant left move 1.
  - **No `en-cohesion` component, deliberately.** Independence is a founding-time
    *evidentiary* property, not strain in a living polity; D23's disavowal earned a
    component because a ground withdrawing is an ongoing condition. Recording the
    omission as a decision rather than leaving it to look like an oversight.
  - **The sockpuppet can recurse**, and nothing here stops it: an actor with two
    identities can have one vouch for the other, and only the *roll* filter and the
    absence of a further attester make that visible. What the tier now has is a named
    party to point at when it turns out badly, which is the whole of what it claimed.
  - Nothing new crosses to KC. D4 holds; the border stays at four artifacts.

- **D25 — N-ary fission, and the successor rules nothing enforced (locked 2026-07-29).**
  The last item on the tier's own still-open list was arity: `fusion` takes N ≥ 2
  predecessors while `fission` was pinned at two by its types (`successor_a` /
  `successor_b`, `links: [_; 2]`). A three-way split therefore had to be two chained
  fissions, which recorded a history nobody lived — an intermediate polity that existed
  only to be dissolved again.

  **The measured hole was larger than the arity.** Fusion carries four guards; **fission
  carried none.** Measured, all three passing before this slice: a "split" naming the
  **same successor twice**; a split naming the **predecessor itself** as its own
  successor; and — the serious one — a successor whose **own self-signature does not
  verify**, for which fission signed a `SuccessorLink` under the predecessor's root
  anyway. That last had the polity's root key vouching for a forgery.

  **And the archive half was open too**, which matters more: `authority.rs` never
  mentioned `successors` at all, so a hand-built `CharterDissolution` could name its own
  polity, or one successor twice, and pass `check_authority`. `en-uplift` only *emits*
  records; the authority pass is what a consumer verifies, so guards that live only in
  the emitter are guards a hand-built record walks around.

  So: `fission` becomes N-ary and inherits fusion's four guards in the other direction
  (arity ≥ 2, pairwise-distinct successors, no collision with the predecessor,
  `verify_self_signed` on **every** successor), and the `CharterDissolved` arm gains
  three rules of its own — no duplicate successor, no self-successor, and no successor
  that is on the dissolving polity's own roll. That last is the **third** of a family
  now: *a machine cannot guard a machine* (D15), *a ground inside what it grounds is no
  ground* (D23), and a successor is a polity rather than somebody on the roll. Naming
  **no** successor stays valid — that is the difference between a fission and a death,
  and the tier records both.

  **The honest limits.**
  - **These guards are structural, and that is all they are.** Nothing here can tell a
    legitimate split from a faction dividing the spoils. That judgement is not the
    tier's: the split is a *ratified* act (D4/Q2 — the monitor never decides one), and
    this records it rather than approving it.
  - **The successor DIDs are not checked for being chartered polities**, because at
    fission time they need not be in this archive at all — the successors' own foundings
    are their business, and a KC-side consumer holds their identities. What is checked is
    everything the record can answer for itself.
  - An API break, and the second this month: `FissionOutcome`'s fields change shape.
    `links` was `[_; 2]`, so it could not have survived the generalization in any case.
    Tally is now D20 (envelope v2), D21, D22, D25.


## Open questions (round 2)

- **Q9 — AT operator-attended signal (D8 prerequisite). RESOLVED 2026-07-24.**
  AT must expose, per node and to peers, *which nodes have a human behind them*,
  so `ethne-at` can derive the guardian edge. Resolutions:
  **(a) semantics → both.** Two signals: a durable **operator-bound** attestation
  (the node has a human guardian) and a live **attended-now** freshness (a verified
  operator session is active). ethne's guardian edge uses *bound*; *attended-now*
  feeds cohesion/liveness later. **Both halves are now built (2026-07-27):** AT
  carries `operator_attested_at` (consumer-pull, no keepalive) and `en-at` folds it
  onto each `MemberCandidate`, where `en-cohesion` grades it as a fourth erosion
  component — capped at `Notice`, never a Concern, because a point-in-time snapshot
  cannot tell a night shift from an abandoned polity.
  **(b) verifiability & placement → credential-hash in the signed Identity.** The
  signal carries the operator's chain-verifiable `zta_credential_hash` (+ issuer)
  inside the *signed* Identity payload — tamper-evident, non-forgeable, reusing the
  ZTA machinery and propagating with peer admission. Not self-asserted; not a
  plaintext envelope bool.
  **(c) sequencing → AT signal first.** Build and verify the AT operator-attended
  capability (C + Python + wire + conformance), mirroring the roster capability,
  *then* scaffold `ethne-at` consuming both roster + signal.

## Open questions

- **Q1 — the ratification quorum. RESOLVED 2026-07-28 → gate the accounting, grade
  the judgement (see D21).** A founding may still narrow the cluster it came from,
  but never *silently*: the charter carries the boundary it claims to ratify —
  `candidate_size` plus a reasoned `Exclusion` for everyone the boundary showed and
  the roll does not name — and, the charter's core having no amendment path, carries
  it permanently. Authority refuses a charter whose boundary does not add up;
  nothing refuses one merely for being narrow, because a genuinely separate group of
  three inside a thirty-node cluster is indistinguishable from a faction capturing
  it without a substantive judgement no rule can make. The number Q1 asks for —
  what share of the cluster actually consented — is `founding_legitimacy`'s
  `consent_share`, derivable by anyone holding the charter, forever: the faction of
  one that used to pass a "majority" at a single signature now reads **0.033**.
  Accountability, not prevention; the residue (a captured provider feeding a
  gerrymandered boundary) is **Q5**.
- **Q2 — when is a schism a fission? RESOLVED 2026-07-24 → ratified, never
  auto-verdicted.** The cohesion monitor (`en-cohesion`) does **not** decide a
  fission — algorithmically splitting a polity would be exactly the "replace
  emergence" move the tier exists to refuse. What the monitor does is grade strain
  into an `ErosionLevel` (`None` / `Notice` / `Concern`, mirroring KC's own
  erosion alert one tier down) off two snapshots: the chartered roll vs. the
  provider's current candidate (attrition, reputation decay, floor breaches, absent
  guardians, transmission decay — members lapsed on re-affirming the substrate,
  read off `en-charter`'s `AffirmationState` — lost guardian edges (D15: a
  chartered node→guardian edge the substrate no longer shows, over members it still
  observes), a widening affirmation gap, a thinning induction witness turnout, roll
  churn, sustained absence, the **unrated share** (D18: observed members the
  provider has no reputation reading for), and an opaque boundary capping attrition
  at `Notice`).
  Five of those are
  capped at `Notice`: the opaque boundary caps *attrition*; the absent-guardian
  component is capped outright, because a *snapshot* of absence is ambiguous by
  nature; the unrated component likewise, because a young cohort and a provider that
  has stopped rating are the same picture (D18); and the two decay trends
  (gap-widening, witness-decline) are capped
  because a trend is a leading indicator whose consequence is already gradeable by
  the affirmation component (D16). **Sustained** absence is the one signal that
  passes the attendance cap, and only on the strength of a record: an unattended
  span long enough *and* witnessed by enough archived observations reaches
  `Concern`. At `Concern`, `en-uplift` emits
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
- **Q4 — substrate for a purely emergent group. RESOLVED 2026-07-28 → it must
  anchor, and the anchor must consent (see D23).** The leaning held: a polity with
  nothing above itself is exactly the failure KC's non-ultimacy feature guards
  against, and it charters no longer. A roll of nothing but machines answering to
  nobody must name a substrate anchor outside itself, and that key co-signs the
  founding. A human roll needs no key — its statement *is* the anchor, unkeyed on
  purpose — but every polity must articulate one, and none may name itself or one of
  its own members as its ground. What the tier grades on top of what it gates is
  `grounding`, so a bare-but-legal ground is visible rather than merely permitted;
  and if the ground later withdraws, it says so on the record and the monitor reads a
  Concern. **With this the round-2 ledger is empty: every open question Q1–Q9 is
  resolved.**
- **Q5 — provider trust. RESOLVED 2026-07-28 → attribute the reading, grade the
  agreement, and let the excluded answer (see D22).** Nothing *stops* a captured
  provider, and D22 does not pretend to: what it removes is anonymity. A boundary
  claiming a substrate reading must carry that reading, signed by the reader it
  names and bound to this founding, and the roll plus its exclusions must match what
  that signature shows — identity, where D21 had only arithmetic against the
  proposer's own report. A gerrymandered boundary is therefore attributable to a
  named key, permanently; `founding_legitimacy` reports which readers *agree* with
  what was ratified and which contradict it; and an eleventh entry lets the excluded
  say, unprunably and even after dissolution, that the accounting omitted them —
  the one claim the archive could never check for itself, since it holds no
  provider. Accountability, not prevention. The residue is **sockpuppet
  corroboration** (`did:key` is free), left as a stated limit on a lens rather than
  an authority rule, exactly as kith-covenant left capture move 1; independence
  attestation is the named successor.
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

Decisions D1–D25 locked; **every open question Q1–Q9 resolved**. Build order (steps 1–5
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
   diff, staying `kc-core`-only. `en-uplift` (the crate holding the confined
   KC-record deps, D10) emits the four border artifacts: the `CommunityIdentity`
   handoff, the `Concern` conduct record on `Concern`-level erosion (neutral
   `Other("coherence-erosion")` bearing), a ratified `fission` bundle (two
   successor identities + two signed `SuccessorLink`s + the predecessor's
   `CharterDissolution`, a new `en-charter` charter-lifecycle record), and — since
   D14 — the envoy `SpeakerDelegation` for whoever the polity has authorized to
   speak for it. `cargo tree`
   confirms all core + monitor crates remain `kc-core`-only and that both
   KC-record couplings (`kc-conduct`, and `kc-identity` since D14) stop at
   `en-uplift`.

Deferred (later slices): the full intra/inter ceremony rites for envoy recognition
(KC's job once it holds the `CommunityIdentity`) and the multi-movement
choreography of the intra-community rite (v1 records the rite's completion, not its
movements); the at-ffi decoder that would fill these fields from a live node, and
the AT-side signal it needs before `MemberCandidate::reputation: None` is reachable
at all (AT collapses "no history" into `PREREP_NEUTRAL` at its own API boundary —
see D18); formerly-deferred
richer decay analytics (gap-widening / witness-decline trends, as
in KC's full `ceremony_cost_decay`);
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
