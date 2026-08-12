<!--
  Copyright 2026 Sean M. Brennan and contributors
  Licensed under the Apache License, Version 2.0.
-->

# Ethne: the polity tier

**Design document.** The tier's decisions are recorded as D1-D31 below, and the
questions they raised as Q1-Q9. Anything the design specifies and the code does not
carry is in [`TODO.md`](../TODO.md). How the tier is built, which toolchain choices
are deliberate, and the verification discipline the slices are held to are in
[`architecture/build-and-toolchain.md`](architecture/build-and-toolchain.md).

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
   "who cooperates?"        "what is a community,         "how do communities treat
                             and what keeps it one?"       one another?"
```

Kith-covenant's atomic actor is *a community*, and it quietly presumes that
community is already **bounded** (a known membership), **coherent** (it stands
for a substrate and can speak through delegates), **persistent** (it outlives its
current members; the structured fast, re-affirmation cycles, and
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
handle to bind to. It deliberately does **not** build the community's *inside*:
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

Consequence: KC sits on top of ethne, which may be fed by autonomous-trust, rather
than on autonomous-trust directly. KC's README says so.

## The load-bearing principle: ratify emergence, do not replace it

AT and KC embody two different **legitimacy stories**. AT: structure is
legitimate because it is *earned and emergent*, bottom-up from behaviour. KC:
standing is legitimate because it is *conferred and covenantal*, by a recognized
act. Ethne's entire job is to reconcile them, and the way it does so is a single
discipline it must never violate:

> Ethne **reads** the emergent structure (the trust field, the cluster
> boundary, the earned reputations) and offers it back to the members for
> **deliberate ratification** into a charter. Emergence *proposes*; the polity
> *ratifies*; KC *recognizes*.

Impose structure top-down and ethne betrays AT's legitimacy (the community is a
diktat, not a people). Merely relabel the emergent cluster (M1 alone) and it
betrays KC's coherence assumption (the mask has no inside). The interesting
engineering is precisely the ratification step: the moment a set of cooperators
*chooses* to become a bounded thing that stands for something and can act as one.

## Core vocabulary

- **Polity**: the persistent community entity. Identified by a root DID. Not
  equal to its current members; members join and leave it.
- **Charter**: the polity's founding, signed constitutional artifact. The
  ratified membership roll at founding, the articulated substrate, the internal
  offices, and the amendment rules. The community-scoped analogue of KC's
  compact genesis.
- **Substrate**: what the polity holds above itself and defers to (its
  tradition / governance / ground). For a machine-majority polity the substrate
  is *derived* and anchors upward to human guardians.
- **Membership roll**: the hard-edged, signed set produced by ratifying a soft
  emergent boundary. Versioned; admissions and departures are records.
- **Office**: a *conferred* internal role (convener, inductor, envoy-authorizer,
  charter-keeper) held until revoked, distinct from earned reputation.
- **Cohesion**: the measurable degree to which the polity still functions as one
  coherent actor (no schism, no internal capture, substrate not drifting).
- **Founding / fission / fusion / dissolution**: the lifecycle transitions of a
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

### 3. Internal offices: trust → conferred standing, turned inward  *(new)*

KC's integration analysis names the "category change from measurement to
adjudication" (its disconnect §B1): earned reputation is not conferred standing,
and wiring one to the other numerically is a category error. Ethne applies that
change *inward*: earned AT reputation may be *evidence* in conferring an internal
office, but the office is set by a deliberate, signed act and holds until revoked,
it never tracks the reputation number. Neither AT (emergent authority only) nor
KC (inter-community roles only) lets a community confer its *own* internal
offices. Ethne does.

### 4. Cohesion & anti-fragmentation  *(reuse KC observer/jester, scoped inward)*

Ongoing detection of schism, internal capture, and substrate drift *within* one
polity, and telling the world when a "community" has stopped being
one coherent party (so KC no longer treats it as a single actor). This is KC's
observer/jester machinery and ceremony-cost tracking, applied to a single
community rather than the compact. The output that crosses to KC is a plain
`ConductKind::Concern`, so KC needs no new vocabulary.

### 5. Longevity & continuity  *(reuse KC ceremony-cost / re-affirmation)*

The polity's identity is a persistent charter that members join and leave, with:
**transmission** (induction / re-affirmation, how a new member acquires the
substrate; KC's ceremony-cost proxies already model erosion here), **succession**
(offices persist as officeholders change), and **identity persistence** (the
charter, not the member set, is the entity). This is the tier that makes KC's
generational assumptions (structured fast, re-affirmation cycles,
genesis-immutability) coherent in the first place, because it is what creates the
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
| DID identity, signing | `kc-core` | n/a | reuse as-is |
| Append-only content-addressed archive | `kc-log` (neutral, beside `kc-core`) | n/a — shared, not borrowed | `en-archive` re-exports it; `kc-archive` is built on it too (D32) |
| Genesis / immutable-core / asymmetric amendment | `kc-core` | `kc-constitution` shape | **new** `en-charter` (D3) |
| Intra-community induction rite | `kc-core` | `kc-ceremony` five-Movement rite | **new** `en-*` (runs *for members*, not envoys) |
| Ceremony-cost proxies / re-affirmation | `kc-core` | `kc-reputation` decay pattern | **new**, over ethne induction records |
| Cohesion / drift monitoring | `kc-core` | `kc-reputation` observer lenses | **new**, over ethne records |
| Substrate-provider interface (pluggable input) | `kc-core` | n/a | **new** |
| Internal-office conferral | `kc-core` | n/a | **new** (distinct from KC's inter-community roles) |
| Coherence-erosion signal to KC | none (KC-native) | `ConductKind::Concern` | reuse, no new KC vocabulary |

## Interfaces

### Downward: the substrate-provider interface  *(pluggable)*

Ethne defines a narrow `SubstrateProvider` interface (built in `en-substrate`):
the source of the emergent evidence a founding ratifies and the ongoing
behavioural input cohesion monitors. It exposes, roughly: a candidate membership
set with soft boundaries, a per-member earned-reputation reading, a per-member
guardian edge with the liveness stamp of when that guardian was last verified
present, and a change/anomaly stream (**D16**: `SubstrateChange`
deltas behind a *separate* `ChangeStream` trait, so a provider with no notion of
change is not made to fake one). The shipped `ManualFounding` provider needs no substrate
service; an existing-organisation roster importer is another trivial provider.

**`ethne-at`: the autonomous_trust provider, over AT's C interface (decision
D5).** autonomous_trust's *production* implementation is a **C daemon**, not the
Python reference tree: an embedder runs it with `at_node_init` / `at_node_start`
/ `at_node_run` (`src/c/autonomous_trust/node.h`) over a pair of IPC message
queues, and reads AT→app messages with `messaging_recv_on` (`generic_msg_t`,
`message_type_t`, `src/c/autonomous_trust/utilities/message.h`). So `ethne-at` is
a **Rust crate over the C FFI**, not a Python module: ethne stays pure Rust and
needs no PyO3 for the adapter. It *tees the AT→app message stream* and folds it
into a live view: peer-admission and group-membership messages give the candidate
roll and its (soft) boundary; reputation-update messages give the per-member
earned reading and the change/anomaly stream; the `Operator` / node identity
gives the guardian edge for a derived (machine) member. That view *is* a
`SubstrateProvider`.

The C linkage is isolated behind an `AtEventSource` seam (a Rust trait yielding
decoded AT events) so `ethne-at` builds and unit-tests in-sandbox against a
**mock** event source, with the real `extern "C"` binding to
`libautonomous_trust` behind the `at-ffi` cargo feature. (A live daemon links and runs; see D6.) AT's reputation subsystem (`src/c/autonomous_trust/reputation/`,
a Paxos-committed hash-linked history with per-peer reputations and window/
checkpoint roots) is the reputation source; its Merkle-linked history can later
become machine-attested *exhibits* for a KC `Concern`, matching the AT↔KC
integration doc's M4.

### Upward: what ethne hands KC

Ethne emits only KC-native artifacts, so KC stays oblivious to the tier
(decision D4):

- a standard **`CommunityIdentity`** (root DID + delegation graph): the chartered
  polity as a KC-recognizable party;
- **recognition artifacts** for the polity's authorized envoys/machines (produced
  by the induction rite);
- a **`Concern`** when the cohesion monitor finds *soft coherence erosion*, the
  jester pre-indictment flag KC already routes. `Concern` carries **early
  warning**, not a fission verdict (a `Concern` is about conduct; a split is about
  a community's existence: a category `Concern` should not be forced to carry).
- a **fission** is emitted with *no new type at all*: ethne charters the two
  successor polities (two new `CommunityIdentity`s) and records the predecessor's
  charter **dissolution**. KC sees two new communities seek recognition and the
  old one go quiet: entirely within KC's existing vocabulary. **Fusion** is the
  mirror and is built (`en-uplift::fusion`, N-ary ≥ 2 predecessors): the merged
  polity's `CommunityIdentity` plus, per predecessor, a `SuccessorLink` to it and
  that predecessor's charter dissolution naming it as sole successor, so KC sees
  several communities go quiet and one new one seek recognition.

## Machine-majority polities and the guardian assembly

KC's machine-participation model is unambiguous that a machine participates only
in a *derived* form, "irrevocably tied to one specific named human" (the
guardian) and the integration analysis flags (its §D1 / §M2 / question Q2)
that the node→guardian→standing relation is "asserted in prose, absent from both
codebases' types." Ethne is the natural home for that relation to become
structural: for a machine-majority polity, **the polity substantially *is* the
guardian assembly**, the humans whose standing is at stake in the machines'
conduct. The charter's substrate for such a polity is the guardians' ground; the
offices are held by guardians; the membership roll records node→guardian edges.
This gives Facets 1-3 of machine-participation an entity to live in.

**D15** settles the relation, and it is structural: a `Derivation` on the
charter says which members are derived (machine) parties and which human each
answers to, ratified at founding (with every named guardian co-signing) and moved
afterwards by `GuardianChange` records. From that one fact both halves follow: a
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

## Architectural decisions

- **D1, dependencies: the neutral base only.** Ethne depends solely on kith-covenant's
  neutral crates — `kc-core` for identity/crypto and, since **D32**, `kc-log` for the
  append-only log; it does **not** depend on KC's governance crates. Their
  analytics are bound to KC's own `ArchiveEntry` types, so there is little to
  reuse over ethne's own records; depending on them would invert the layering
  (ethne sits below KC in the data flow) and block ethne-standalone use. Ethne
  therefore owns thin `en-*` code that borrows KC's proven designs. *Note:* the
  append-only archive **mechanism** in `kc-archive` is neutral and worth sharing,
  but `kc-archive` hardcoded KC's `ArchiveEntry` enum; v1 forked a thin
  `en-archive` over ethne's own entry type, and extracting a generic entry-agnostic
  archive base that both build on was named here as the one extraction worth doing.
  **D32 built it** as `kc-log`: `en-archive` re-exports it, `kc-archive` is a newtype
  over it, and measuring the fork before removing it turned up a ~20x index cost in
  both tiers and a key-resolution defect in KC. Depending
  on `kc-core` alone still meant linking KC's §8 whistleblower crypto, which the
  polity tier never calls. `kc-core` now gates its `ring` and `custody` modules behind
  a default-on `source-protection` feature, and both root manifests here set
  `default-features = false` on the `kc-core` **workspace** entry: that placement is
  load-bearing, since cargo ignores the flag when it is written on a member crate that
  inherits the dependency with `workspace = true`. The closure of `en-core` drops from
  61 crates to 53. `curve25519-dalek` remains, reached through `ed25519-dalek`, which
  `kc-core` needs for signatures unconditionally.
- **D2, package shape: mirror KC.** A Rust + PyO3 workspace
  (`ethne/src/rust/en-*`, `ethne/src/python/`), same build/test conventions and
  signed-record rigor, its own `environment.yaml`. **Built** (2026-07-24), both
  halves: the `en-pyo3` cdylib (`ethne._core`, via maturin from `pyproject.toml`)
  and the `ethne` package over it, split along the tier's own seams,
  `ethne.polity` / `ethne.authoring` / `ethne.monitor` / `ethne.host` / `ethne.uplift`,
  joined by `ethne.at` at **D33**. Three
  conventions carry the Rust semantics across intact: co-signers arrive as
  parallel DID/key lists which a `Signer` dataclass flattens (a record's consent
  co-signature, the joiner, the office holder, the inductee, is added by the
  binding); every KC-facing artifact crosses as **JSON**, so a KC-side consumer
  appends it verbatim and ethne still needs no `kc-archive` dependency; and the
  authoring surface preserves ratify-not-replace, since a proposal is only JSON
  until a majority of the roll co-signs it. `en-at` was deliberately left unbound here,
  on the reasoning that its real coupling is `at-ffi`-gated and Python could reach the
  same path through `founding_candidate`. **D33 retired that reasoning**: reaching the
  path by hand meant reimplementing the fold, and the reimplementation was measured
  reading an unguarded machine as guarded and attended.
- **D3, charter: fork a thin `en-charter`.** Own the
  genesis-singleton / immutable-core / asymmetric-amendment code in ethne,
  depending only on `kc-core`, borrowing the `kc-constitution` design. Keeps
  `kc-constitution` focused on the compact and honors D1.
- **D4, KC contract: no new KC-facing type.** Ethne emits `CommunityIdentity` +
  recognition artifacts; *soft* coherence erosion → the existing `Concern` flag;
  *hard* fission → charter two successor polities + dissolve the predecessor's
  charter (KC sees new `CommunityIdentity`s + the old one going quiet). This
  preserves KC ⟂ ethne totally. A dedicated coherence/fission record was
  rejected because it would reintroduce a KC→ethne schema dependency.
- **D5: `ethne-at` is a Rust crate over autonomous_trust's C interface.** AT's
  production implementation is a C daemon driven over IPC message queues
  (`node.h` lifecycle + `message.h` `messaging_recv_on`), not the Python
  reference tree. `ethne-at` therefore tees the AT→app message stream through an
  `extern "C"` / `-sys` binding and implements `SubstrateProvider` over the
  resulting view: no PyO3, ethne stays pure Rust. The C linkage sits behind an
  `AtEventSource` trait so the crate builds and tests against a mock, with the
  real `libautonomous_trust` binding feature-gated.
- **D6: `ethne-at` supports both coupling modes (resolves Q6).** The
  `AtEventSource` seam has two concrete implementations from day one: an *embedded*
  source that owns an AT node's lifecycle (`at_node_init`/`at_node_start`) and an
  *attached* source that reads an already-running daemon's `q_out`
  (`messaging_recv_on`). Both are feature-gated behind the real C binding; the
  in-sandbox mock covers both paths' decoded-event contract.
  **BUILT: attached first, then embedded once AT grew a lifecycle to bind
  (2026-07-30).** *Attached* went in over AT's flat `app_events.h` ABI. *Embedded*
  could not follow, for a structural reason rather than an effort one: the flat ABI
  covered the event stream but not the lifecycle, and both routes into the
  lifecycle are unsafe from Rust. `at_node_init`/`at_node_start` take a
  caller-allocated `at_node_t`, which embeds `logger_t` and `queue_t` **by value**:
  mirroring it means reproducing two opaque C layouts, exactly the
  silent-corruption hazard the flat ABI exists to avoid. `run_autonomous_trust`
  looks flat enough to call directly and is not trustworthy: its header says it
  "blocks until the daemon exits" and returns 0/non-zero, while `node.c` assigns
  the return value to `daemon_pid` and carries on, and the header declares
  `capabilities` as `void *` where the definition takes `capability_t *`. Two
  disagreements between a declaration and its own use, in the one function
  embedding needs. So the missing piece was named as an AT-side flat lifecycle ABI
  rather than faked with a type that always fails (the dead-field pattern D23 and
  KC's `source_commitment` both turned out to be) and then **built there**:
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
  shutdown); and a handle means the event stream is *reachable*, a failed inbound
  bind now fails the start and reaps the daemon instead of logging and continuing.
  See autonomous_trust's `doc/architecture/app-peer-carrier.md`, "Owning the daemon
  from another language".
- **D6: the live leg is covered now, and what blocked it was a real bug
  (2026-08-04).** This said a live start/poll/stop was "not covered in-sandbox,
  since no daemon can run here (AT's ISSUES.md §2.1.1)". §2.1.1 turned out not to
  be the sandbox artifact it was filed as: `get_data_dir` told `path_join` its
  destination held 255 bytes where `unix_addr`'s holds 108, so a long
  `$AUTONOMOUS_TRUST_ROOT` overflowed the caller's stack and aborted *this*
  process from inside the FFI call. Short roots hide it, which is exactly why it
  "did not reproduce" off-sandbox. Fixed on AT's side, and
  `a_live_daemon_starts_becomes_ready_and_stops` now starts a real daemon, waits
  for it, reads from it and stops it: 58 `at-ffi` tests.

  Two things the live leg then measured, both of which had been invisible:
  `at_app_node_start` returns in ~2 ms while the daemon's queue is not bound for
  another ~170-210 ms, so a host's first sends silently went nowhere, hence
  `EmbeddedAtNode::ready`/`wait_ready` over AT's new `at_app_node_ready`, and a
  distinct `AT_APP_NOT_READY` from a roster pull instead of an unclassifiable -1.
  And `at_app_node_stop` returned while the daemon was still running, because it
  `waitpid`'d a process that `daemonize`'s double fork had already given to init;
  a full shutdown takes ~5.3 s. Both fixed in AT.

  The assertions are on the lifecycle, not on event content: a solitary node with
  no peers emits nothing, so a count would assert nothing. Content is covered by
  the five end-to-end emitter tests. Still open, and AT's: a stopped daemon leaves
  8 subsystem processes running (ISSUES.md §2.1.3); the test kills the ones it
  caused.
- **D6: the two carrier streams are joined here, by uuid, and `None` stays
  honest (2026-07-30).** (Filed as "D26" until 2026-08-06, which was a
  mislabel: D26 is the member roll as a border artifact. This is the third D6
  entry, the coupling's decoder half.) AT emits peer identity and peer reputation as *separate*
  messages (its own decision, to avoid coupling two independent processes), so
  `en-at` owns the join. Four rules, in `decode::PeerJoin`: the join key is the
  **uuid** while the ethne identity is the **signing key**, so a reputation for a
  uuid never yet observed is *held* rather than dropped, AT's two producers have
  no ordering guarantee; neither message kind may clobber the other's fields;
  `reputation: None` covers both "AT says unrated" and "not heard yet", which is
  faithful because **D18** defines `None` as *no reading at all* and its hazard is
  confusing `None` with *rated low* (AT's 0.2 `PREREP_NEUTRAL`), that distinction
  is preserved; and an attendance stamp of `0.0`, or any stamp on a node without
  the verified operator bound, becomes `None`. The join is **not** feature-gated:
  the FFI is what needs a C library, the join is where the reasoning lives, so its
  rules are unit-tested in the default build.

  The join is exercised end to end over the real transport rather than only in
  unit tests, which needed something on AT's side to play daemon:
  `app_events_test_support.{h,c}` (in `libsrc`, called by nothing in AT itself)
  gives a foreign consumer `at_app_test_emit_peer` and
  `at_app_test_emit_reputation`. Sends need no queue of their own, since
  `at_app_events_open` already calls `messaging_assign`, so a consumer that
  opened a queue can emit *to* it and loop a synthetic event back to itself over
  the real socket. The helpers deliberately do **not** copy the real emitter's
  gating: see
  [`architecture/build-and-toolchain.md`](architecture/build-and-toolchain.md),
  "Verification practice".
- **D7: the AT boundary is the gateway-reputation subtree (resolves Q7).** The
  candidate roll is the enumerated subtree member-roster produced by the AT
  identity process's roster capability (requestor-side BFS, privacy-opt-out-aware,
  arbitrary depth). The top gateway is the boundary; a private boundary is an
  intentional opaque edge, not an incomplete roll. This reuses the capability
  landed 2026-07-23 rather than inventing a new boundary notion.
  **CORRECTION (2026-07-29, on building the AT carrier):** the subtree roster
  cannot supply a *key-identified* member. Its entries are
  `{uuid, nickname, address}` (`_roster_add_member`) (no signing key) so it
  cannot feed `AtPeer.signing_pubkey`, and a member with no key has no
  `did:key` and cannot be charted. The live feed's boundary is therefore this
  node's **admitted peers** (`protocol.peers[]`, which are `public_identity_t`
  and do carry `signature.public`). D7's subtree remains the right boundary
  *notion* and is still reachable offline via a synthesized/captured view
  (`MockEventSource`), but a live roll is one-hop-scoped until the roster
  carries keys, which is an AT wire change, not an adapter change. The
  opaque-edge signal (`BoundaryPrivate`) is consequently not emitted live
  either: it is only meaningful alongside the subtree view.
- **D8: the guardian edge is the `Operator` identity (resolves Q8).** A derived
  (machine) member's guardian is its human `Operator`, carried on the AT event
  stream. This is the human-above-machine anchor the substrate's guardian ground
  requires, and aligns with the AT↔KC integration M2 and machine-participation
  node→guardian→standing. **Prerequisite (see Q9):** D8 assumes AT advertises,
  per node, a verifiable *operator-attended* signal (which nodes have a human
  behind them). That signal does not yet exist on the wire and must be built in AT
  first: the counterpart to the subtree-roster capability that grounds D7.
- **D9: `CommunityIdentity` is a shared `kc-core` primitive (locked
  2026-07-24).** The upward handoff (D4) requires both tiers to speak the *same*
  `CommunityIdentity` type, but a KC-owned identity type would drag KC's graph
  into ethne, and an ethne-owned one would invert the layering. Resolution:
  `CommunityIdentity` + `SuccessorLink` are **extracted down into `kc-core`** (the
  neutral base both tiers already depend on); they need only `kc-core` types and
  have one inherent, self-signing implementation. `kc-identity` re-exports them, so
  KC's own code is unchanged. This keeps the handoff a shared neutral primitive
  without either tier depending on the other.
- **D10: the `kc-conduct` coupling is confined to one crate (locked
  2026-07-24).** Emitting KC's `Concern` (D4) means depending on `kc-conduct` (a
  KC *record* crate, not a governance crate, but still not `kc-core`). Rather
  than relax D1 across ethne, the dependency is confined to a **single upward-
  facing crate, `en-uplift`**, whose whole job is speaking KC's language. Every
  crate below the uplift boundary (`en-core`, `en-charter`, `en-substrate`,
  `en-at`, `en-cohesion`) stays `kc-core`-only, verified by `cargo tree`. The
  Concern uses the neutral `ConductBearing::Other("coherence-erosion")` (**not**
  the covenant-specific `AntiSpiral` bearing) so no covenant semantics leak below
  the boundary. ethne also never depends on `kc-archive`: `en-uplift` *emits*
  serialized `Signed<T>` payloads (CBOR-roundtrippable), and a KC-side consumer
  wraps each into an `ArchiveEntry` and appends it.
- **D11: the membership roll evolves by signed record, asymmetrically (locked
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
  **self-govern floor**: the point at which it could no longer form the
  `majority + 1` its own contraction rules require (so a roll only sheds a member
  from size ≥ 4). The floor binds *both* departure kinds: when a voluntary exit
  would break self-governance, collective viability wins, the leaver waits for a
  replacement to be admitted, or for the polity to dissolve, rather than strand a
  rump that cannot act. This is the charter-side answer to how a roll changes
  legitimately (adjacent to question **Q1** on the founding quorum); it does
  not touch who counts as an *emergent* member on the substrate side. A companion
  hardening makes dissolution terminal: once dissolved, no further charter event
  (amendment, membership change, or a second dissolution) is authorized.
- **D12: internal offices are roll-conferred, three-kind, effective only while
  held by a member (resolves Q3).** Offices (`Office {
  Convener, Inductor, EnvoyAuthorizer, CharterKeeper, Other(String) }`) are
  conferred and revoked by signed `OfficeChange` records. The **Q3 bootstrap** is
  answered by making the *roll* the standing conferring authority: **offices
  never confer offices**, so the founding roll confers the first offices with the
  same authority that ratified the charter, and there is no infinite regress (no
  PolityCharter change). The three kinds mirror the membership asymmetry (D11), for
  the same principles: *Confer* needs a majority of the current roll **and** the
  holder's co-signature (consent, no conscription into duty), on a current member;
  *Resign* is signed by the holder alone (no forced service); *Revoke* needs a
  majority with the holder excluded from the deciders. An office holding is
  **effective only while its holder is on the roll**: it lapses if the holder
  departs and revives if they return; the roll ends it for good by `Revoke`. There
  is no self-govern floor (offices may all be vacant), the bars are symmetric
  (the roll stays sovereign; offices are functional roles under it), and multiple
  holders per office are allowed. Conferral, derivation (`office_state`) and a `holds`
  query are not sufficient on their own, since nothing need consult them; **D14** adds
  the use-time enforcement. The machine-majority "offices held by
  guardians" constraint **is an authority rule, per D15**: the obstacle was
  that the charter log recorded no guardians (they were read from the substrate
  provider only), and D15 removes it by chartering the node→guardian edge. In a
  machine-majority polity a conferral refuses a derived member with no guardian.
- **D13: transmission is roll-witnessed induction plus self-signed
  re-affirmation, with a query-time freshness cadence.** Of
  responsibility #5's three parts, two need no new machinery: **succession** falls
  out of D12 (an office is a permanent role; its holders are transient grants, so
  offices persist across holders by construction), and **identity persistence** is
  already structural (the polity DID is stable across every D11 roll change, the
  charter, not the member set, is the entity). What is new is **transmission**,
  carried by signed `Affirmation` records (`AffirmationKind { Induction,
  ReAffirmation }`). Admission (D11) puts someone on the roll; **induction is the
  distinct step by which they acquire the substrate**, so a member can be on the
  roll yet un-inducted. An induction is **witnessed by the roll** (a majority of
  current members co-sign alongside the inductee (the community receives them, and
  they participate)) while a **re-affirmation is self-signed** by an
  already-inducted member, because renewal is the member's own act. Since **D34** a
  renewal also **carries a full re-recitation of the five movements** and is refused
  without one, with whoever attended recorded and never required — self-signed and
  ceremonially costly are not in tension. The charter's
  **ratifiers** are inducted by ratifying it (dated `founded_at`), so they take no record
  and cannot be inducted again, and only they: **D29** found the seed reading the
  founding *roll*, which handed the substrate to listed members who had signed nothing. Freshness borrows
  `kc-reputation::ceremony_cost_decay`'s staleness shape: `affirmation_state`
  reports each current member's induction status and last affirmation, and
  `stale_members(cadence_seconds, as_of)` names those who are un-inducted or
  overdue, so a community whose members stop re-affirming shows erosion.
  `en-cohesion` grades that list as a fifth erosion
  component, calling `stale_members` rather than restating the rule, and (unlike
  the absent-guardian component) it may reach `Concern`, because duration is
  already inside the measure: a member a whole cadence past re-affirming is not
  merely away from the console. An absent `AffirmationState` means *not evaluated*,
  never *all fresh*. The
  cadence is a **query-time** parameter (like `en-cohesion`'s thresholds), not a
  charter term, so no `PolityCharter` change. Roll-witnessing was chosen over
  office-gated induction, and the `Inductor` office was a named role only,
  **until D14 gave it a defeasible presiding requirement over the rite
  (2026-07-27), which does not displace the witnessing rule but adds to it**.

- **D14: offices bite at use time, in two different ways.**
  D12 built offices as conferral plus derivation; nothing consulted the derivation,
  so an office was a label. Two of the four now carry authority into the records
  themselves, and deliberately *not* by the same mechanism:
  - The **`Inductor` presides**, a **defeasible gate**. Where the office is
    effectively held, one of its holders *other than the inductee* must be among an
    induction's witnesses; the roll may still induct without them, but only at a
    majority **+ 1** (D11's asymmetric-bar move). So an officer can raise the bar
    for the rite and can never block it. The cost is stated rather than hidden:
    conferring an `Inductor` *does* raise the roll's own bar for inducting without
    them, majority → majority + 1. The zero-cost alternative (recording who
    presided without requiring it) was considered and declined as no teeth at all.
  - The **`EnvoyAuthorizer` holds a delegated power**: an **alternative route**,
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
  gate until D17 gave it one**: custody of the log is what that office is, and
  pruning the observation record is the act it now holds a delegated power over
  (bounded by a settling horizon, since destroying evidence is not like authorizing
  an envoy). Self-signed acts
  (re-affirmation, voluntary departure, resignation) are never gated: they are
  precisely the acts no one else may condition.

  The envoy record is the substance of the second half. `EnvoyAuthorization`
  (`EnvoyKind { Authorize, Resign, Revoke }`, mirroring `OfficeKind` for the same
  reasons) names who may speak for the polity, in **ethne's own vocabulary**: the
  record carries no KC delegation scope, so `en-charter` stays `kc-core`-only. A
  standing is effective only while its envoy is on the roll (lapsing and reviving
  as an office holding does) and until its own `expires_at`; the record also carries
  the envoy's public key, checked to derive their DID, because the KC-side artifact
  names its speaker by key. `en-uplift::envoy_delegation` turns an *effective*
  standing into KC's `SpeakerDelegation` scoped `Envoy`, the fourth border
  artifact, and the only one an office produces. It takes the derived `EnvoyState`,
  not the archive, so the border crate still never walks a log. `kc-identity` becomes
  a *direct* `en-uplift` dependency, confined exactly as `kc-conduct` is, and it
  **widens the build graph not at all**: `kc-conduct → kc-ceremony → kc-identity`
  already put it in the tree, so D14 only makes an existing edge explicit.

- **D15: the node→guardian edge is chartered, not merely observed (locked
  2026-07-27).** The relation KC's integration analysis calls "asserted in prose,
  absent from both codebases' types" now has a type on the ethne side. A
  `Derivation { member, guardian: Option<DidKey> }` marks a member a derived
  (machine) party; founding derivations live in the `PolityCharter` and later ones
  in `GuardianChange` records (`Designate` / `Rotate` / `Release`). `guardian: None`
  is a **real chartered state** (a machine with nobody behind it) and not a
  missing value; recording it honestly is what gives the office rule something to
  refuse.
  - **Ratified, not read.** The provider *observes* the edge
    (`MemberCandidate.guardian`); `FoundingCandidate::propose` now carries it into
    the proposal instead of discarding it, and only ratification makes it a fact.
    That is the same move the roll itself undergoes, so the guardian edge obeys
    ratify-don't-replace rather than becoming a provider assertion the polity is
    stuck with.
  - **Consent, everywhere.** A named guardian co-signs the founding, a
    `Designate`, and a `Rotate` (the *incoming* one): answerability for a machine
    is accepted, never assigned, which is KC's "irrevocably tied to one specific
    named human" read as a *bilateral* tie. Symmetrically nobody is held to it: a
    `Release` needs either the departing guardian's own signature or a majority of
    the roll recording that the human is simply gone. A `Release` clears the *edge*
    and keeps the designation: a machine does not become human by losing its human.
  - **No machine guards a machine**, at founding or on rotation, and nothing guards
    itself; otherwise the human-above-machine anchor closes into a loop.
  - **The shape follows the tier's grain, not the charter's roll.** `members` stays
    `Vec<DidKey>`. Designation is a separate record with a separate derivation, as
    D11's admission, D12's offices, D13's induction and D14's envoy standing all
    are: this tier already distinguishes "on the roll" from "has property X", and
    restructuring the roll into typed members would have rippled through every
    signature that takes one for a guarantee the provider reading cannot give anyway.
  - **What follows.** `guardian_state` derives the effective picture (designations
    lapse while their member is off the roll, exactly as office holdings do) and
    answers `is_machine_majority(roll_len)`. The office rule (D12) refuses a
    conferral on an unguarded derived member **where the polity is machine-majority
    at that point in the log**: a mostly human polity is untouched, because the
    rule exists for the polity that substantially *is* its guardian assembly, not
    because machines are second class. `en-cohesion` gains a sixth component: a
    chartered edge the provider no longer shows, counted only over members it still
    observes, so an absent member is attrition and an opaque boundary cannot
    masquerade as answerability collapsing.
  - **Two honest limits.** (1) A machine that was never designated reads as human;
    the designation is only as good as the provider's reading, which is **Q5**, not
    something a founding can settle. (2) **CLOSED by D18.** `MemberCandidate` could not
    *express* "derived but unguarded" (a `guardian: None` reading meant human, or
    machine with nobody, or a provider that cannot tell) so `propose` ratified only
    *guarded* machines and the unguarded state had to be designated deliberately
    after founding. `MemberCandidate::derived` now carries the substrate's reading of
    *kind* independently of the edge, and `propose` filters on it.
  - **The edge now has a live source (2026-07-30).** D15 was built against
    `MockEventSource` and could not have run against a daemon: AT carried *that*
    a human exists and *when* one was last present, never *which* human, so
    `en-at`'s decoder hard-coded `guardian_pubkey: None`, a chartered rule with
    no feed, which is the dead-field shape D23 and KC's `source_commitment` both
    turned out to be. AT's operator-key slice closes it. A node may bind an
    ed25519 operator key with its PIV credential (the signed pre-image names the
    node, so a harvested pair cannot transfer) and AT verifies that binding
    before ever putting the key on its carrier. `en-at` decodes it under a fifth
    join rule (*an all-zero key, or any key on an unbound node, becomes `None`*)
    and an opted-in peer now reaches `MemberCandidate.guardian` off a real
    socket. **What this does not give us:** the binding is opt-in on AT's side
    and deliberately free to decline, so most peers will read `guardian: None`
    and that is a correct reading of an unguarded machine, not a gap.

  - **The guardian now co-signs (2026-08-03).** Naming a guardian was
    only half of D15: the rule demands the named guardian's own signature, and until
    now nothing could produce one. The key deliberately lives in the operator's
    keystore rather than on a polity's authoring host, so `Signed::build`'s
    private-key-per-signer shape left only one route (move the key) which is the
    arrangement that location exists to prevent. Both custody paths now exist. The
    **co-located** one (`en_at::operator_signer`) reads AT's keystore where the
    operator is running the authoring step themselves; it refuses a key file anyone
    else can read, and checks the derived `did:key` against the guardian the substrate
    reported so a wrong key is caught at authoring time rather than surfacing later as
    a signer mismatch. The **detached** one (`en_charter::detached`) sends the payload
    instead: the archive exports canonical bytes, a CID and a legible description; the
    operator signs them with their own key (AT's `--sign-file`); and `assemble`
    verifies each signature before anything is appended. A detached envelope is
    byte-identical to a built one (same CID, same signatures, since ed25519 is
    deterministic) so nothing downstream can tell which route was used. The
    cross-library agreement the whole thing rests on (AT persists a hex 32-byte seed
    via libsodium; `kc_core` signs from that seed via dalek) is pinned as a test
    vector rather than assumed. `kc-core` is untouched, so kith-covenant is unaffected.

    **And the edge must now be backed by what the substrate showed.** D15 grades the
    charter against the provider's reading, so an edge no reading ever showed left the
    measure comparing a ratified claim to nothing, and five tests moved a guardian
    edge with no observation anywhere while asserting the result correct, which is
    `substrate.anchor` (D23) and kith-covenant's `source_commitment` again. A
    `GuardianChange` that *claims* an edge (a `Designate` naming a guardian, or any
    `Rotate`) now carries `GuardianEvidence`: either the signed `Observation` whose
    reading for that member shows that guardian, or an explicit `Unbacked { reason }`.
    A `Release` carries none: it claims the edge is *gone*, and D16 is explicit that a
    member absent from the readings was not observed, which is not evidence of absence.
    Evidence is **carried, not cited**, because D17 lets a polity prune observations by
    a signed act: a citation would leave a record authorized today and unauthorized
    after legitimate housekeeping, making authority a property of when you looked
    rather than of the log. The regression test that pins it is
    `a_cited_observation_would_not_survive_a_legitimate_pruning`. The override is
    self-declared and safe only through an asymmetry (it can never *strengthen* a
    record, only disclose that nothing backed it) which is D23's
    `SubstrateDisavowed` shape, and why the self-declared field D21/D22 found
    unenforceable elsewhere is sound here.

    **What this still does not give us.** The binding remains opt-in on AT's side and
    free to decline, so most peers read `guardian: None` and that is a correct reading
    of an unguarded machine. Evidence is one observer's perspectival reading and never a
    ratified fact (D16): a colluding observer can sign a reading showing any edge it
    likes, so the gate raises the cost from *nothing* to *a signed false observation
    attributable to a named observer*, legibility, not prevention, with independence
    attestation (**D24**'s shape) the named successor. Founding derivations are not
    gated by this rule; `propose` filters on the substrate reading at the emitter and
    D21/D22 gave founding its own carried-evidence discipline; whether founding should
    carry per-edge evidence *as well* is a question this design leaves open, and it is
    recorded as such in [`TODO.md`](../TODO.md).
    Distinct guardian keys remain not-distinct-humans (**D24**); counting them as such
    would want an independence attestation, and nothing here supplies one.

- **D16: what was seen is recorded, so that duration can be judged (locked
  2026-07-27).** Every measure before this one read a *snapshot*, which is why the
  attendance component had to be capped: a night shift, a weekend and an abandoned
  polity are the same picture. D16 gives the polity a memory of its own observation
  and grades four things that only exist over time.
  - **The stream.** `SubstrateChange` deltas (appearance, vanishing, reputation
    move, guardian-edge move, an advancing attendance stamp, boundary opacity)
    drained through a `ChangeStream` trait kept **separate** from `SubstrateProvider`,
    because `ManualFounding` is a convener with a list and *an empty stream and no
    stream are different claims*. Deltas rather than a diff of two candidates: the
    provider is the one place the prior reading is already in hand, and two equal
    snapshots either side of a flap look like no change at all.
  - **The record.** `CharterEntry::Observed(Signed<Observation>)` carries what one
    monitor saw at one instant, the readings in full, not the deltas, because a
    duration measure needs the stamps and a self-describing record is what makes it
    evidence. An observation **asserts nothing about the polity**: it is perspectival,
    in the sense KC's observer reports are, so there are no deciders, no majority, and
    the observer **need not be a member**, a monitor that had to be inside the roll
    could be silenced by expulsion. The one rule is that it must be signed by exactly
    the observer it names: nobody may put an observation in another's mouth. (This is
    the entry `entry.rs` anticipated when it said a later slice would fold cohesion
    records in. It also cost `CharterEntry` its `Eq` derive: an observation carries
    float reputations, and a total equality over those would be a lie.)

    *The other half of that anticipation never happens, and the note in `entry.rs`
    saying it would has been corrected.* `CharterEntry` carries every variant the
    tier needs, so there is no founding-slice type left to fold in. Cohesion was
    never going to join them either: a `CohesionReport` is **derived** from the
    entries rather than appended alongside them, which is why it is not even
    `Serialize`, and why adding a field to it is not a format break.
  - **Sustained absence, the exception that proves the cap.** A guardian unattended
    across the recorded window for at least `sustained_absence_seconds`, *and* seen by
    at least `sustained_absence_watchers` distinct observers (**D28**), reaches
    `Concern`. Two
    conditions doing two different jobs: **the span is the absence; the count is the
    evidence that somebody was watching while it lasted.** One stale reading with a
    long silence behind it says nothing about the polity, only that nobody looked.
    The snapshot cap is unchanged and still absolute; what lifts it is not a looser
    threshold but a record. A member the window never saw is *not evaluated*, never
    assumed absent.
  - **Three trends that needed no stream at all.** Gap-widening, witness-decline and
    roll churn are derivable from the polity's own signed records (`Affirmation`
    carries its instant and its witnesses, `MembershipChange` its instant) so
    `decay_trends` reads them with no provider involved, borrowing the shape of
    `kc_reputation::analytics::ceremony_cost_decay` one tier up, hedges included: a
    witness decline needs two inductions (never a trend from one point) and a widening
    gap is judged against the **median** of prior gaps, so one aberrant interval does
    not read as decay. The two booleans **warn but never escalate**: a trend is a
    leading indicator, and the lapse it leads to is already escalatable by the
    affirmation component, so grading both to `Concern` would count one decay twice.
    Churn, being a rate rather than a trend, carries a full pair.
  - **`CohesionInputs`.** With four charter-side read-models the report's positional
    signature became four adjacent references of the same shape; they now live in one
    struct of `Option<&T>` with `Default`, which also puts *not evaluated* in the type
    instead of in a convention repeated by hand at every component.

- **D17: the log may forget, but only by saying so.** D16 left
  exactly one thing owed: nothing pruned the observation record, and nothing persisted
  it incrementally. Both are measurable rather than theoretical. One observation is
  1.7 KB of JSON for a five-member polity and 5.6 KB at twenty-five, so at the
  attendance cadence a twenty-five-member polity writes roughly **557 MiB a year**:
  D16's "no change ⇒ no entry" bounds that below the poll rate, but an attendance stamp
  advances on every re-announce, so an attended fleet sits near it. And `en-archive`
  had no persistence at all: it existed only in the Python extension module, as a
  whole-log re-serialization, so a Rust consumer could not save a log and a Python one
  saving per observation paid for the whole archive every time.
  - **Why observations, and only observations.** `reconstruct` matches an observation
    and does nothing with it: *an observation changes no state.* Every other record
    here governs something, so dropping one would rewrite what the polity decided;
    dropping an observation cannot. **Observations are the one inert entry class,
    which is exactly what makes them the one prunable class**: the argument is
    structural, not a matter of taste about which data is precious.
  - **A recorded act, not a sweep.** `ObservationPruning` names a horizon and carries a
    receipt: how many records went, and a digest over their CIDs. A Concern crossing
    to kith-covenant rests on observations, so evidence that could vanish silently
    would make D16's reading count (the *evidence somebody was watching*) impossible
    to falsify, and leave an auditor unable to tell "nobody looked" from "we deleted
    it". The receipt is checked **conditionally**: while the observations are still
    present the record must say truthfully what it drops, but once compaction has run
    they are gone, and a rule that recomputed regardless would report every historical
    pruning as a violation. So the log verifies what it still can and treats an applied
    pruning as beyond its power to check rather than as a lie. The receipt keeps its
    force for anyone holding an earlier copy.
  - **The CharterKeeper's teeth, bounded.** One effective `CharterKeeper` among the
    deciders suffices (the delegated-power shape D14 gave the `EnvoyAuthorizer`) but
    **only beyond `PRUNE_SETTLING_SECONDS` (30 days)**; the roll may prune anything at a
    majority. The asymmetry is the point: destroying evidence is precisely the move a
    captured party makes, so the recent record stays out of any single custodian's
    reach while the majority route keeps the office from stalling anything. That
    constant is deliberately **law rather than a threshold**: every other cadence here
    is a query-time parameter because it is policy, but a floor the party it constrains
    could lower is not a floor.
  - **Pruning may only ever weaken a signal.** The window carries the `pruned_before`
    horizon and `unattended_span` clamps to it, because behind the horizon the log
    holds no evidence and no absence may be claimed across it. This is not a
    precaution. `last_attested` is a *max* over the window's stamps, so dropping the
    interval that held the freshest one lowers that max, starts the absence earlier and
    makes it **longer**: pruning manufacturing a Concern. Building the slice surfaced
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
    they cannot disagree about what was dropped, and it is where the guarantee lives
    that a pruning can never reach a governing record.
  - **Persistence.** A versioned dump/load moves down into `en-archive`, generic over
    the entry type as the rest of that crate is, plus `dump_json_from(start)` /
    `append_json` so a host holding a cursor appends what is new instead of rewriting
    the log. The Python surface now emits the envelope and reads either it or the bare
    array earlier builds wrote.

- **D18: the candidate says what the provider *knows*, not just what it saw.**
  `MemberCandidate` was lossy about its own provider's epistemic state, and three
  separate findings turned out to be that one defect: `guardian: None` conflated human /
  machine-with-nobody / cannot-tell (D15's second honest limit), `attested_at: None`
  conflated nobody-there / cannot-confirm, and a bare `reputation: f64` conflated
  **unrated** with **rated low**. D16 had already made this move one layer up
  (`CohesionInputs` became a struct of `Option<&T>` so that "not evaluated" lives *in
  the type* rather than in a convention hand-implemented three times) and D18 gives
  `MemberCandidate`'s own fields the same treatment.
  - **Why the reputation conflation was a live hazard, not a tidiness complaint.**
    autonomous_trust returns its `PREREP_NEUTRAL` (0.2) both for a peer it knows
    nothing about *and* as a score a peer can genuinely earn, while `en-cohesion`'s
    `reputation_floor` (0.3) means *this member is failing*. Absent the distinction,
    a four-member roll of unrated members grades **`Concern`** (the level
    that crosses to kith-covenant) on day one, purely for being new. Unknown and
    failing were literally indistinguishable: the same input, the same grade.
  - **`reputation: Option<f64>`.** `None` is *unrated*. Both reputation components
    are computed over the members that carry a reading, with `rated_members` as the
    denominator, and report nothing when nobody is rated: the same "no reading means
    not evaluated, never healthy" shape the transmission and guardian components
    already used.
  - **An eleventh component: the unrated share, capped at `Notice` by
    construction.** There is deliberately no `unrated_concern`, exactly as there is
    no `unattended_concern`, and for the same kind of reason: a cohort too young to
    have been rated and a provider that has stopped rating are the same picture from
    a snapshot. What it exists to prevent is the two components above going
    *silently* dark: they report nothing when nobody is rated, and something has to
    say that they are not reporting.
  - **`derived: bool`, independent of the guardian edge.** `propose` filters on it
    instead of inferring derivation from `guardian.is_some()`, so a machine the
    substrate reads as having nobody behind it is ratified as the unguarded
    `Derivation { guardian: None }` the charter has always understood. **No
    en-charter change was needed**: the charter could already express the state; only
    the candidate could not propose it. That makes D15's machine-majority office rule
    non-vacuous *at founding* rather than only after a deliberate designation.
  - **An unrated candidate does not satisfy a demanded floor.** A positive
    `min_reputation` is a positive assertion and `None` cannot support it; a floor of
    `0.0` demands nothing and admits everyone. Behaviour-preserving at both ends: a
    provider that used to collapse "unknown" into a low number saw those members
    trimmed by any floor above it, and still does.
  - **The AT mapping is structural, which was a finding rather than a choice.** In
    AT's model there is no such thing as a human peer (a peer *is* a node) so every
    AT candidate is `derived: true` and what varies is `operator_bound`, i.e. whether
    the guardian edge is there. A node with no operator is precisely the unguarded
    machine; before D18 it proposed silently as a human member.
  - **The honest limit that remains.** `reputation: None` is not yet reachable from a
    live node: AT collapses "no bilateral history" into `PREREP_NEUTRAL` at its own
    API boundary, and "exactly 0.2" is not a safe inference because a peer can earn
    it. Exposing the distinction is a named **AT-side prerequisite**, sequenced the
    way Q9 sequenced the operator signal: build the AT half first. The type is honest
    now; the feed is not yet there.

- **D19: a retention policy, because a horizon chosen by hand is chosen blind.** D17
  left exactly one thing owed: it gave the polity the *act* of pruning and nothing that
  says when. A host wanting to prune had to invent a horizon, and getting it wrong is
  silent in both directions: too aggressive and it clips the evidence a kith-covenant
  Concern rests on, too timid and the log grows without bound. Measured on the marginal
  bytes an observation adds to the serialized log, with readings across the whole roll:
  **3.4 KB at five members, 4.6 KB at ten, 8.0 KB at twenty-five**, so at the attendance
  cadence a twenty-five-member polity writes **~803 MiB a year** and an attended fleet
  sits near that rate, because a stamp advances on every re-announce.
  - **"Scheduler" is the wrong name for what was owed.** This tier's libraries never
    call `now()` (every cadence is a query-time parameter (`stale_members(cadence,
    as_of)`, `Thresholds`, `cohesion_report(as_of)`)) so the loop and the clock
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
  - **The safety floor is a duration *and* a count, and the second half was a
    finding, not a design.** The slice was specified with one floor,
    `live_measure_seconds`, on the reasoning that the longest observation-based
    measure is `sustained_absence_seconds` (one day). Measurement refuted it. A
    sustained absence is graded on **two** conditions, and D17's clamp only protects
    one: the span survives, because the window clamps it to the pruned horizon, but
    the **witness count** does not. The same absence, the same 24-hour horizon, graded
    `Concern` at a dense observation cadence and `Notice` at a sparse one: four
    readings falling to two, one short of `sustained_absence_watchers`, with nothing
    saying that evidence rather than conduct is what changed. So `min_witness_watchers`
    (default 3, mirroring the threshold it protects) holds the horizon back to leave
    that many readings of **every watched member** standing. Per member, because
    `readings_for` is per member: a member the monitors mention rarely has thin
    evidence, and thinning it further is exactly the clip this read-model exists to
    refuse. A thinly-watched member can therefore *pin* the horizon indefinitely.
    That is the correct answer and not a deadlock: D17 always leaves the roll free to
    prune anything by majority. What is refused is *recommending* it.
  - **Age-only retention, and the default is the cheap one by coincidence worth
    asserting.** `retain_seconds` defaults to `PRUNE_SETTLING_SECONDS` (30 days): the
    smallest value that simultaneously preserves every observation-based measure and
    keeps the **single-`CharterKeeper`** route open, since that route requires
    `horizon <= pruned_at - PRUNE_SETTLING_SECONDS`. A *shorter* retention keeps more
    history and is therefore more expensive to authorize, not less, which is
    counter-intuitive enough to be a test rather than a comment. It also happens to be
    the cheap one to store: a keeper-authorized pruning is ~2.9 KB with its one
    signature, where a roll-majority one carries a majority of them (5.8 KB at five
    members, 20.2 KB at twenty-five). At the default the log is bounded (8,640
    observations retained, **28 MiB at five members and 66 MiB at twenty-five**) so no
    count- or size-based retention rule is needed and none is built.
  - **`min_dropped` exists because a pruning record is itself unprunable.**
    Observations are the one entry class a polity may drop; the records that drop them
    are governing records and stay forever. Run as a control (the same simulated year
    through a host loop waking daily) pruning at `min_dropped=1` authored **335
    permanent pruning records and left a 1.27 MB log**, against **13 records and 435 KB**
    at the default of 100. Eager housekeeping made the log **2.9× larger**. The default
    keeps the permanent residue under ~3 MiB/year even at the fastest realistic
    observation rate, and stays small enough to fire promptly on a slow cadence, where
    a large floor would let the log run well past its retention target before anything
    triggered.
  - **The recommendation names its authorization route**, so the host knows whom to
    ask: `PruneRoute::CharterKeeper { holders }` (from `office_state`, already
    intersected with the roll, so a departed keeper is never offered) and/or
    `PruneRoute::RollMajority { need }`. Both are reported when both are open, because
    the officer route is an *alternative* to the roll and never a gate on it.
  - **Not graded in `en-cohesion`.** An unpruned log is a housekeeping fact, not
    evidence the polity is losing coherence; the eleven components stay about the
    polity's own life. The **accepted risk**, recorded rather than mitigated: a polity
    that leaves `CharterKeeper` vacant and never prunes accumulates silently, and only
    whoever runs the loop will see it.
  - **What remains the host's**, unchanged from D17: a storage backend (file/SQLite)
    and the loop itself. D19 closes the horizon question, not those.


- **D20: storage, and the loop.** D17 and D19 each closed with
  the same deferral: no storage backend ships, and nothing runs the loop. Taking it
  on turned up that the first half was not a gap but a **defect**. D17 shipped
  incremental saving (keep a cursor, write the segment past it) and compaction
  (`apply_prunings` returns a new archive with observations removed), and *those two
  do not compose*. Measured on the real surface: a host saving per observation and
  then pruning reloaded **12 entries instead of 6, with 7 pruned observations
  resurrected**.
  - **Why it was invisible, which is the part that matters.** The reloaded log passed
    `verify_all` *and* `check_authority`: the pruning receipt re-verified, because
    every CID it named was back. And the mechanism was a documented convenience:
    `dump_json_from` treated an index past the end as "already current" and returned
    an empty segment, so after a compaction (cursor 12, new length 6) the save wrote
    **nothing** and the `ObservationsPruned` record never reached storage at all. With
    it went the window's `pruned_before`, so `unattended_span` read **300 days instead
    of 230**. A storage bug *lengthening* a measured absence is precisely the
    Concern-manufacturing failure D17's clamp exists to prevent, re-entering by a
    route the clamp does not cover.
  - **Generations.** An index is only meaningful within an append-only run, because
    compaction removes entries from the *middle*: post-compaction index N is a
    different entry. So the archive carries a `generation`, advanced by
    `Archive::next_generation` (one caller: `apply_prunings`) and never by an append,
    and a cursor is `(generation, index)`. `dump_json_since` refuses a cursor from
    another generation or past the end; `append_json` refuses a segment whose
    generation or start position is not the next piece of *this* log. Envelope version
    2 carries both. Merging two independent logs into one view is a genuinely
    different operation and now says so: `merge_json`, which the KC-side
    several-communities reader wants and persistence never does.
  - **`en-store`: the branch is one line, and that is the crate.** *Same generation ⇒
    append a segment; generation changed ⇒ rewrite in full.* The store owns the
    cursor, because the whole hazard is what happens when a caller keeps one. An
    `ArchiveStore` trait generic over the entry type (as the rest of the archive
    machinery is, D1, so the crate needs no `en-charter` dependency at all), with
    `FileStore` (one segment per line; append is write+fsync, rewrite is
    temp+fsync+rename, which POSIX makes atomic) and `SqliteStore` behind a `sqlite`
    feature. Both persist *the same segment envelopes*, so they are differentially
    testable and neither can drift into its own idea of a stored log; SQLite earns its
    place on the rewrite, which is one transaction rather than a temp-file dance.
  - **A torn tail is named, not swallowed.** A crash mid-append leaves a partial final
    line. Loading reports it; `recover()` truncates it as a deliberate, reported act.
    Silently skipping an unparseable last line would discard a signed entry without
    saying so: the same shape of failure as the bug above. The log may forget, but
    only by saying so.
  - **`en-host`: `tick` decides and computes; the host signs and does I/O.**
    `HostCycle::tick(archive, candidate, as_of)` returns the `DueAction`s owed,
    persist, a graded `CohesionReport`, a `RaiseConcern` when it reaches that level, a
    `PruneRecommendation` when the log is due. It takes `as_of` like every other
    cadence in this tier, so a year of host behaviour runs in microseconds at times
    chosen to land on cadence boundaries, and it **authors nothing**: raising a
    Concern and pruning both need signing keys the host holds. Emergence proposes and
    the polity ratifies, applied to the host's own duties. It covers the cadenced
    duties and only those: membership, induction, office conferral and envoy
    authorization are deliberate acts, and nothing schedules them.
  - **One clock, and it is not a library.** `src/rust/examples/polity_host` is a
    `[[bin]]`, mirroring KC's `two_communities_demo`; nothing depends on it. Its
    single `time::now()` is the only clock read in ethne outside a test, apart from
    `en-pyo3`'s pre-existing `*_at=None ⇒ now` binding convenience. It runs a
    simulated timeline by default so it terminates and can be verified, and
    `--follow` is the real loop.
  - **Measured.** With the whole log at 5,000 entries an append writes **67 bytes**
    against a rewrite's **63,954**: 955×. The measurement also caught a real defect
    in the claim "each save costs what is new": `dump_json_from_to` reached the tail
    with `iter().skip(start)`, walking and index-looking-up every skipped entry, so an
    append to a 5,000-entry log burned **~5.4 ms of pure CPU**. Slicing the order
    index instead (`iter_from`) makes it flat at **~14 µs**, independent of log size.
  - **The honest limits.** `SqliteStore` is ethne's first C dependency
    (`rusqlite/bundled`); it is off by default and `cargo tree` confirms the default
    workspace graph is unchanged and still `kc-core`-only. Concurrent writers are a
    host error: two stores over one file fail loudly rather than interleaving, but
    nothing *locks*. And `en-host` is Rust-side only: a Python host gets `FileStore`
    and the read-models, not the cycle. (Both are closed: locking by D30's own second
    half, and the binding by **D31**, where measuring the second one found it was never
    about ergonomics.)

- **D21: a founding says what it excluded (resolving Q1).**
  The genesis authority rule asked for a majority of `p.members.len()` (the
  *proposal's own roll*) and `FoundingCandidate::propose` discarded the candidate
  it derived that roll from. The charter had no referent outside itself. Measured
  against a thirty-node substrate: a faction of one chartered the cluster as a
  polity of itself, on one signature, passing both `verify_all` and
  `check_authority`; the exclusion of the other twenty-nine was not merely
  unpunished but **unrecorded**. The majority bar was vacuous because the faction
  picked its own denominator.

  The resolution splits the problem in two, along the seam the tier already uses
  elsewhere: *the log may forget, but only by saying so; a founding may exclude,
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
    about the tradition: the same shape as the attendance cap's inability to tell a
    night shift from an abandoned polity (D8/Q9). What the rule removes is the
    silence, not the discretion.

  `founding_legitimacy(charter)` is a read-model over the charter **alone** (no
  provider, no cohesion input) so anyone holding the charter can re-run it forever.
  It reports `consent_share`, **the single number Q1 asks for**: ratifiers over
  everyone the boundary showed, rather than over the roll the founders chose. Its
  `FoundingBasis` (`Corroborated` / `Narrowed` / `SelfAttested`) is deliberately
  **not** ordered and **not** an `ErosionLevel`: a founding is not eroding, and
  these are not comparable harms.

  `BoundarySource` rides on `FoundingCandidate`, not on `propose`. The provider is
  the only party who knows whether a boundary is an independent reading or a
  convener's word, and a `ManualFounding` candidate has exactly the same *shape* as
  a trust service's, so the distinction cannot be inferred downstream. This is D18's
  move applied to the roll (put the epistemic state in the type) and the codebase
  already drew the line informally: `ManualFounding` gives asserted members
  `Some(1.0)` rather than `None` precisely because *a convener vouching for a roll is
  the reading here*. A convener-asserted founding still charters; the charter simply
  never pretends it was corroborated, which is the same honest chartered state as an
  unguarded machine's `guardian: None` (D15).

  `propose` now records whoever the reputation floor trims, with the reason it
  actually used: distinguishing "reputation 0.21 below the floor 0.30" from
  "unrated; a floor of 0.30 demands a reading" (D18's semantics, stated where an
  excluded party can read them). `propose_selecting` is new, for the *deliberate*
  narrowing: if the only ergonomic path were the floor, an honest small founding
  would be dressed up as a floor it is not, or done by hand with an attestation the
  convener writes themselves.

  *Judgement call, flagged:* `FoundingLegitimacy` carries the exclusion **list**,
  not only the count, though a Rust caller holding the charter could read it off the
  attestation. A grade reporting "29 excluded" without their names would reinstate,
  one level up, the exact silence D21 removes, and it is the only way the excluded,
  or anyone auditing for them, can read the reason off a surface that is not Rust.

  **Measured**, against a thirty-node substrate: a founding narrowed honestly, and
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
    check it against the substrate, because the archive has no provider: a
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
    dumps**. Acceptable (pre-release, uncommitted, and genesis is a singleton so
    there are no long-lived logs) but it is a real format break, alongside D20's
    envelope v2.
  - Nothing new crosses to KC. D4 holds: a founding defect is not conduct, and the
    border stays at four artifacts.

- **D22: a boundary says who read it, and the excluded answer back (locked
  2026-07-28, resolving Q5).** D21 recorded *that* a boundary came from a reading.
  It could not say **whose**: `BoundarySource::Substrate` was an unsigned label the
  proposer supplied, so it was evidence of nothing. Measured, and worse than Q5's
  wording: flipping one public field promoted a convener's bare assertion to
  `Substrate`, and the grade with it to `Corroborated`, with no provider ever
  consulted and both verification passes green. **D21's own decision #2
  (that a convener-asserted boundary is labelled so permanently) was unenforced**,
  and a rule that checks only that the *numbers* add up, against a
  self-report, passes any internally consistent fabrication.

  Three moves, each in the tier's existing grain:

  - **A: the reading is attributable (authority).** The `Substrate` arm now
    *carries* its evidence: `Substrate { readings: Vec<Signed<SubstrateReading>> }`,
    each reading naming a `reader` and signed by that reader alone (D16's "nobody
    may put an observation in somebody else's mouth"), bound to the founding it is
    offered for (without which an honest reader's account of one cluster replays
    into somebody else's founding while the signature still verifies). Nesting a
    signed envelope is an established idiom in the corpus (`kc-ceremony` and
    `kc-escalation` both carry a `Signed<Recitation>`) and `didkey_resolver`
    recovers the reader's key from its DID, so none of this needs a registry.

    **The rule with teeth is identity, not arithmetic:** the roll ∪ exclusions must
    equal some signed reading's members *as a set*. D21 supplied both sides of its
    own sum; here the right-hand side is signed by somebody else.

    Key custody is unchanged from D20: the provider holds no keys and gains none
    ("`tick` decides and computes; the host signs and does I/O").
    `FoundingCandidate::reading()` yields an *unsigned* reading and whoever runs the
    provider signs it. **No `en-at` change.**

  - **B: corroboration means readers who agree (a lens, never a gate).**
    `FoundingLegitimacy` gains `readers` and `corroborating_readers`, deliberately
    *not* a count of readings filed, since a reader who signed a different boundary
    corroborates nothing, and the gap between the two numbers is a reader
    contradicting the founding on its own charter, permanently. `Corroborated` now
    requires at least one agreeing attributable reading, where before the label
    alone sufficed. This stays inside `founding_legitimacy`, so D21's property
    survives: the grade still derives **from the charter alone, forever**.

  - **C: the excluded answer back.** An eleventh entry,
    `BoundaryDissented(Signed<BoundaryDissent>)`, over
    `DissentKind { Unnamed, ReasonRejected }`. Perspectival evidence in D16's exact
    shape: signed by the dissenter alone, no deciders, no majority, and the
    dissenter need not be a member, one required to sit inside the roll could be
    silenced by never being admitted. **This is the only thing in the tier that can
    contradict an understated `candidate_size`**, which is precisely what D21
    recorded as beyond its reach: the archive holds no provider, so the excluded are
    the only available witnesses.

    Authority checks coherence with the record contested, never truth: a
    `ReasonRejected` dissenter must actually appear in the charter's exclusions (you
    may reject the reason given for *you*), and an `Unnamed` dissenter must appear in
    neither the founding roll nor the exclusions (a record that names them
    contradicts the claim outright). `founding_dissent` is a **separate** read-model
    from `founding_legitimacy` on purpose: dissent lives in the archive and accrues
    over time, so folding it in would destroy the charter-alone property and improve
    neither.

    Two deliberate departures. A dissent may be filed **after dissolution** (the
    only entry class that may, where nine others are blocked) because otherwise
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
  its evidence as two things that could disagree: the very defect D22 exists to
  close, reintroduced one level down. So the sweep across the tier became a
  *deletion* of `source:` lines rather than an edit of them, and `ManualFounding` is
  `ConvenerAsserted` **because it holds no reader key**, not because it is labelled
  so. A convener who genuinely obtains a signed reading matching their roll has real
  corroboration and is welcome to it; what is no longer free is *claiming* one.

  **The honest limits.**
  - Still **accountability, not prevention**. A captured reader that signs a
    gerrymandered boundary still charters one. What changes is that the lie is
    attributable to a named key forever, and that the parties it omitted can
    contradict it on the record: D21's negligence-versus-fraud distinction taken
    one step further, from an unsigned claim to a signed one with an author.
  - **Corroboration is sockpuppet-vulnerable by construction.** `did:key` costs
    nothing, so a captured reader can mint its own corroborators and the count
    cannot tell them from independent ones. Stated rather than engineered around,
    following kith-covenant's capture-move-1 precedent exactly, where a declared
    `interpreter_did` was likewise "just a name the community minted" and the
    plurality count was kept a lens with independence attested separately.
    **Independence attestation was the named successor to D22, and is built as D24.**
  - A dissent asserts; it does not adjudicate. Nothing decides whether a dissenter
    tells the truth, and nothing should: that is the "replace emergence" move the
    tier refuses.
  - **Second format break in two slices**, and the first against a *committed*
    format: `BoundarySource` changes shape and the attestation gains readings. Still
    pre-release and genesis is a singleton, so no long-lived logs, but the honest
    tally is now D20 (envelope v2), D21, D22.
  - Nothing new crosses to KC. D4 holds; the border stays at four artifacts.

- **D23: the ground a polity stands on (resolving Q4).**
  Non-ultimacy is the tier's whole reason for having a substrate, and nothing checked
  it. **`substrate.anchor` was read by no rule anywhere**: the only consumers were a
  pyo3 constructor and a getter, and nothing in the corpus had ever set it to `Some`.
  Measured, four foundings passed both verification passes: a polity
  whose entire ground was two empty strings; a fresh cluster of three machines with
  nobody behind any of them; a polity naming **itself** as its anchor; and one naming
  a member of its own roll. The last two matter most, because Q4 was framed as an
  *absence* problem and they are a **closure** problem: a rule demanding only presence
  is defeated by one assignment. The asymmetry that made the case is in the corpus
  already: the guardian rule refuses an edge that "closes into a loop", and the
  substrate anchor that comment is *about* could point at the polity itself.

  **Four authority rules at genesis, all additive.**
  1. **Articulated**: name and statement both non-empty, for every polity.
  2. **Non-closure**: an anchor may be neither the polity nor anyone on its roll.
     A ground inside what it grounds is no ground.
  3. **Grounded**: a roll of nothing but derived parties, not one of them guarded,
     must name an anchor. The trigger is deliberately *not* machine-majority: the
     statement is the anchor for a human roll, unkeyed on purpose, and a guarded
     machine already has a consenting human above it, which is D15's model of
     legitimate machine participation. The key is demanded exactly where nobody
     present could hold a tradition and nobody stands above one.
  4. **Consent**: a present anchor **co-signs the founding**, joining the ratifiers
     and the named guardians. Answerability is accepted, never assigned; you may not
     conscript what you claim to stand under. This reached the Python surface as a new
     `anchor=` parameter on `ratify_charter`, since the founding path had no way to
     supply the ground's consent at all.

  **Graded, not only gated.** `founding_legitimacy` gains `grounding`:
  `Anchored` (a keyed party outside the roll consented, the only grounding anybody
  signed), `Guarded` (guardian edges doing the work), `Held` (no derived party at all:
  a human community holding an unkeyed tradition) and `Bare` (articulated and nothing
  else, unguarded machines on a mixed roll). Descriptive and unordered, like
  `FoundingBasis`. `Bare` is permitted; it is reported so that *permitted* is not
  mistaken for *grounded*. It belongs on that read-model rather than a new one on
  exactly the test D22 applied to dissent: the substrate is genesis-immutable and sits
  on the genesis charter, so grounding cannot accrue and the answer is the same
  forever.

  **And the ground may answer back.** Once an anchor consents it can change its mind,
  which is the question D15 answered for guardians with `GuardianChange`. A twelfth
  entry, `SubstrateDisavowed`, is the ground's own signed withdrawal, and it gets a
  deliberately weaker answer than a guardian's: it **asserts and cannot un-charter**.
  Nothing rewrites the substrate, reconstructs any authority state, or dissolves
  anything, because a withdrawal with that power would be a unilateral kill switch in
  every anchor's hands: a worse hole than the one this slice closed. A disowned
  polity keeps governing itself, on a ground that disowns it, legibly. The read-model
  is `GroundStanding` (separate, because *this* does accrue), and `en-cohesion` reads
  it as a twelfth erosion component that reaches **Concern** alone: unlike every other
  measure there it grades a single signed act rather than a drift, so it takes no
  threshold, event-triggered, like kith-covenant's `contraction_detected`.

  Rules the disavowal shares with D22's dissent, for D22's reasons: signed by exactly
  its filer, **permitted after dissolution** (the second and only other such class,
  otherwise founders silence their ground by winding up), and **unprunable for free**,
  since `is_prunable` matches only observations. Where it is *stricter*: a dissent must
  admit any filer, because an omitted party is by definition absent from the record,
  whereas the ground is named on the charter, so only that key may withdraw.

  **The honest limits.**
  - **Presence, never content.** No rule can read a tradition, and this one does not
    pretend to. What it refuses is the blank, which is not a tradition anyone could
    have been mistaken about. A polity may still articulate a ground it does not mean.
  - **A consenting anchor is not a *trustworthy* anchor.** The rule buys that somebody
    holding a key agreed to stand there and can be named for it: the D22 stance
    exactly: accountability, not prevention.
  - **A disavowal asserts.** Nothing adjudicates whether the ground is right to
    withdraw, and a ground that withdrew cannot un-say it; there is no amendment record
    for a substrate and this slice does not invent one. Such a polity's honest option
    is the one every polity has: to re-charter, deliberately, as the different polity
    it now is.
  - Nothing new crosses to KC. D4 holds; the border stays at four artifacts: the
    Concern travels on the existing one.

- **D24: whether a founding's readers were their own principals (closing the residue D22
  named as its own successor).** D22 made a reading *attributable*; it could not tell
  one reader from five faces of the same actor. Measured: one party minted five
  `did:key`s, signed five identical readings of the same boundary, and the charter
  graded **`Corroborated`** (the highest basis the tier awards) with both verification
  passes green and no dissent anywhere. And the chorus is cheap in a way worth stating
  numerically: **500 readers minted and signed in 16.2 ms, 32 µs a voice.** "`did:key`
  costs nothing" is not a figure of speech, so a count of distinct signing readers
  cannot mean what it appears to mean.

  kith-covenant reached the identical problem for capture move 1 and answered it with
  `InterpreterIndependenceAttestation` plus a lens that counts *vouching* rather than
  self-declaration. The shape transfers directly.

  **The record.** A thirteenth entry, `ReaderIndependenceAttested`, over
  `IndependenceKind { Independent, Controlled }`: a third party's account of a named
  reader of a named founding. Three authority rules: signed by **exactly** the attesting
  party (nobody vouches in another's name), it must postdate the charter, and the reader
  must **actually appear among the founding's readings**. That last is a *gate* where KC
  keeps it a lens, on the line D22's dissent arm already draws: coherence with the record
  spoken about is checkable, the truth of the claim never is. It needed `Reconstructed`
  to carry the founding's readers, exactly as D23 needed it to carry the anchor: the
  subject of the claim is *in* the record even though the claim is not.

  **Who counts, and who is merely recorded.** An attestation from the polity, from the
  reader itself, or from **anyone on the founding roll** does not count toward
  independence: the founders are precisely the interested party, so a founder vouching
  for the founding's own reader is the sockpuppet move one step out. This is stricter
  than KC, which excludes only the community and the interpreter, and it is affordable
  for the same reason D23's rules were: the roll is on the charter. But the record is
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
  assert a named reader is *not* independent: one enum on the same record rather than a
  new entry class, following D22's dissent and D23's disavowal in letting whoever is
  contradicted speak. It counts **from anyone**, deliberately: interest disqualifies
  *praise*, not *warning*, and a founder breaking ranks to say "that reader is one of
  ours" is the most informative possible source for the claim. A contested reader is
  reported as contested, never as disqualified; nothing here adjudicates.

  **A category, not a third exception.** Permitting this after dissolution makes three
  such classes, and three is enough to name the rule: records that are **evidence about
  the founding** (dissent (D22), disavowal (D23), independence (D24)) outlive the
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

- **D25: N-ary fission, and the successor rules nothing enforced.**
  The last item on the tier's own still-open list was arity: `fusion` takes N ≥ 2
  predecessors while `fission` was pinned at two by its types (`successor_a` /
  `successor_b`, `links: [_; 2]`). A three-way split therefore had to be two chained
  fissions, which recorded a history nobody lived: an intermediate polity that existed
  only to be dissolved again.

  **The measured hole was larger than the arity.** Fusion carries four guards; **fission
  carried none.** Measured, all three passing before this slice: a "split" naming the
  **same successor twice**; a split naming the **predecessor itself** as its own
  successor; and (the serious one) a successor whose **own self-signature does not
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
  three rules of its own, no duplicate successor, no self-successor, and no successor
  that is on the dissolving polity's own roll. That last is the **third** of a family
  now: *a machine cannot guard a machine* (D15), *a ground inside what it grounds is no
  ground* (D23), and a successor is a polity rather than somebody on the roll. Naming
  **no** successor stays valid: that is the difference between a fission and a death,
  and the tier records both.

  **The honest limits.**
  - **These guards are structural, and that is all they are.** Nothing here can tell a
    legitimate split from a faction dividing the spoils. That judgement is not the
    tier's: the split is a *ratified* act (D4/Q2, the monitor never decides one), and
    this records it rather than approving it.
  - **The successor DIDs are not checked for being chartered polities**, because at
    fission time they need not be in this archive at all: the successors' own foundings
    are their business, and a KC-side consumer holds their identities. What is checked is
    everything the record can answer for itself.
  - An API break, and the second this month: `FissionOutcome`'s fields change shape.
    `links` was `[_; 2]`, so it could not have survived the generalization in any case.
    Tally is now D20 (envelope v2), D21, D22, D25.

- **D26: the roll as a border artifact, and the ring as a property of the log (locked
  2026-08-03).** kith-covenant's §8 whistleblower channel proves a source is an insider
  with a ring signature over the accused community's key set, and that set was its root
  key plus its live speaker delegations. KC models communities and speakers and has no
  member roster, so an insider holding no delegation was not in the ring, and a
  community that had delegated nobody had no anonymous channel at all. KC recorded this
  as an honest limit and named **this tier** as the feed, since a polity is exactly the
  thing that does model membership. `en-uplift::member_roll` is that feed: border
  artifact **#6**, the polity's `charter_state` roll published under its root key as
  KC's new `MemberRoll`. Every ethne member is a `did:key`, so each member's verifying
  key comes out of its own DID and nothing else has to travel.

  **What measuring first found, and it was the larger half.** KC recomputed the insider
  set at *verification* time against the **current** archive tip, while evaluating it at
  the filing's `filed_at`, and required exact set equality. So the accused community
  (the one party with a motive) could append **one** delegation backdated ahead of
  `filed_at` and every filing against it stopped verifying, or backdate a **revocation**
  and narrow the suspect set around a source after the fact. Both measured: 3 → 4
  invalidated a filing, 3 → 2 narrowed one. A member roll bolted onto that would have
  inherited the defect and added a second lever. KC now computes the ring over the
  records **preceding the filing in insertion order**, which is the principle its own
  §5 `current_rung` already stated ("authorizing an earlier transition never sees a
  later one") and its §8 had not applied. Same failure the guardian-cosign slice
  (**D15**, hours earlier) named on this side of the boundary: authority must be a
  property of the log, not of when you looked.

  **Rolls are unioned, never superseded**: KC's rule, and the one that decides what
  publishing a roll here means. Once named an insider, always an insider, because the
  alternative hands a polity a way to shrink the suspect set around a member the day
  after they left. So ethne's roll shrinks on a departure and KC's ring does not; the
  asymmetry is deliberate and a departed member keeps the channel they had.

  **The honest limits.**
  - **A roll is the polity speaking about itself.** KC checks that the publisher is the
    community named and nothing more; it cannot audit a charter it has no dependency on.
    `as_of` crosses as *provenance*, not authority: the same status `substrate.anchor`
    was found to have had before D23 gave it rules, and named as such here so it does
    not drift into looking load-bearing.
  - **Nothing compels publication.** A polity that never publishes a roll gives its
    members no channel. Union-only bounds what a *published* roll can be walked back to;
    it cannot conjure one.
  - **The tier drops itself from its own roll** (KC refuses a roll naming the community,
    whose root key is already an insider), so a polity of one member and no delegations
    still has no anonymous channel. That is a true reading, not a gap.

- **D27: the founding edge must show what stood behind it.**
  **D15** made every *post-founding* `GuardianChange` carry its evidence: a claimed
  edge presents the signed `Observation` that showed it, or declares
  `Unbacked{reason}`. Founding derivations in the genesis `PolityCharter` carried
  nothing, and the guardian-cosign slice left that open as a question rather than an
  answer. It was a **gate evasion, not a grading gap.**

  **Measured first.** A probe: three machines, nobody behind two of them, **one**
  asserted guardian edge, **no** anchor:

      verify_all      -> green
      check_authority -> green
      founding_legitimacy.grounding -> Guarded     anchor -> None

  **D23**'s grounded rule is `all_derived && none_guarded && anchor.is_none()`, and
  `none_guarded` goes false the moment a single edge is claimed. So one co-signing
  confederate exempted an entire machine cluster from the must-name-a-ground rule, on
  a claim nothing evidenced, while the identical claim, filed one record later in
  the same log, was refused. Consent was never the hole: founding derivations already
  require the named guardian's co-signature. Only the backing was missing.

  A second measurement reshaped the design. **The founding attestation cannot supply
  the evidence:** D22's `SubstrateReading` is guardian-blind (a flat
  `members: Vec<DidKey>`), while D16's `Observation` carries `MemberReading.guardian`.
  There was no read-the-field-that-is-already-there outcome as in D23: the evidence
  has to be *carried*, so `Derivation` gains `evidence: Option<GuardianEvidence>` and
  `GuardianState` gets its own `DerivedEdge { member, guardian }`, since the
  *effective* picture has no business holding per-record evidence. A field present
  and meaningless is the defect class this corpus keeps finding.

  **The fork, and the call: only backed edges count.** `Unbacked` stays permitted and
  honest, but it earns nothing, neither the `Guarded` grade nor D23's anchor
  exemption. `Unbacked` is sound post-founding *only* because it can never strengthen
  a record; at founding that asymmetry **inverts**, since declaring it would buy the
  exemption. That is a strengthening self-report, the exact shape D21 and D22 each
  had to delete. Accepted cost, stated plainly: a bootstrap cluster whose guardians
  are real but unobserved must name an anchor until a monitor has run.

  **THE FINDING: requiring a signed observation was not enough.** A founder signed a
  real, verifying observation of its own cluster's edges, and the charter graded
  `Guarded` with no anchor, both passes green. Evidence closed the *forgery* hole and
  left the *closure* hole wide open. This is the third member of a family: **D23**
  refused a polity naming itself as its ground, **D25** refused a dissolution naming
  a successor already on its own roll, and here the closure reaches the *evidence*
  rather than the claim. `is_backed` now takes the polity and its roll: the observer
  may be neither. The refusal says which shortfall it is looking at (edges observed
  from inside versus edges self-declared) because the remedies differ (have somebody
  outside look, or write down a ground).

  **A judgment call, flagged.** A named **guardian** observing the edge it is about
  to accept *does* count. It is not on the roll (a machine cannot guard a machine),
  it co-signs the founding either way, and what it asserts by observing is its own
  answerability: a burden taken up, not a credit awarded. In that case the keyed
  human outside the roll *is* the ground, which is exactly D23's picture of
  `Guarded`. Interest disqualifies praise, not a warning and not an admission
  (**D24**).

  **The authoring surface.** Founding evidence is *carried*, not resolved by CID as a
  post-founding `GuardianChange` resolves its own, because at founding there is no
  archive yet to hold the observation. Python gains `sign_observation`, the
  pre-archive counterpart of `Archive.record_observation`, and a founding derivation
  is written either as the bare `(member, guardian_or_None)` pair (still correct for
  a machine answering to nobody, which claims nothing) or as
  `(member, guardian, observation_json, unbacked_reason)`. Passing both is refused at
  the boundary; naming a guardian with neither builds fine and is refused by
  `check_authority`, deliberately, so the rule lives in one place and a Python
  negative control can reach it. On the substrate side `build` takes an optional
  `Signed<Observation>` so the live AT path can back its edges, and falls back to
  `Unbacked{reason}` honestly when no monitor has run: the provider does not mint
  backing the rule will reject.


- **D28: the watcher, not the filing.** **D15** and **D27** each
  closed a hole in guardian evidence and each named the same successor for what was
  left: *"evidence remains one observer's perspectival reading, a colluding observer
  can sign a reading showing any edge, so the gate is legibility rather than
  prevention, with D24's independence attestation the named successor."* Measured, the
  residue was both wrongly located and understated.

  **Three probes.**

      1. THE CONFEDERATE AT FOUNDING
         one keypair, on nobody's roll, signs an Observation showing any edge
         verify_all = GREEN   check_authority = GREEN
         grounding  = Guarded, anchor = None, boundary readers = 0
         cost: 271 µs a voice, 500 observer voices in 136 ms

      2. THE NAMED SUCCESSOR CANNOT REACH IT
         filing a reader attestation about that observer:
         REFUSED, "... is not a reader of ...'s founding; there is nothing to attest"

      3. ONE KEY, THREE FILINGS, no relationship to the polity at all
         readings_for = 3, span 4d, unattended 10d
         ⇒ en-cohesion: Concern ⇒ en-uplift emits a kith-covenant conduct record

  So: **the successor did not apply.** D24's coherence gate (deliberately stricter
  than kith-covenant's, requiring the subject to appear in the record) demanded the
  subject be a *reader of the boundary*, and an observer is not one. The record named
  as the fix was about a different party, and its own best feature was what blocked it.

  **A load-bearing sentence was false.** D19 made the witness count a safety floor on
  the reasoning that it "is evidence that somebody was watching". `readings_for` counts
  *observations*. One key filing three times read as three witnesses.

  **And the attack points outward.** The residue's wording described a polity
  flattering itself. The sharper case is a stranger *attacking* one: for the price of
  one keypair, manufacture the `Concern` that crosses the boundary to kith-covenant as
  a conduct record against a community you have never met. That is a worse capability
  than buying your own ground, and nothing in the residue mentioned it.

  **What is a gate, and what is only a lens.** The slice divides exactly there, and
  the division is the whole design.

  - **Distinct watchers is a gate**, because it is not about independence at all. One
    actor filing three times is one watcher; that is arithmetic, not a judgement about
    who anyone is. `ObservationWindow::watchers_for` counts distinct observers,
    `sustained_absence_readings` becomes `sustained_absence_watchers`, and the wrong
    name is renamed because the wrong name is what made D19's sentence false.
  - **D19's floor moves with it**: `min_witness_watchers`, preserving distinct
    watchers per member. This is D19's own finding recurring one decision later: *the
    retention floor must preserve whatever the grade reads*, or it protects a number
    nothing uses. Strictly stronger than before, since a member seen ten times by one
    monitor now has one watcher and pins the horizon outright: the correct answer, and
    not a deadlock, because the roll may still prune anything by majority.
  - **Independence is a lens and never a gate.** Gating an escalation on the monitor
    being vouched for hands a captured polity the simplest defence there is: vouch for
    nobody, become unwatchable. And an unvouched monitor is the ordinary state of an
    honest new one, so refusing it would be refusing newness: D24's line, kept.

  **The record is generalized rather than duplicated.**
  `ReaderIndependenceAttestation` becomes `IndependenceAttestation { polity, subject,
  role, attesting_party, kind, basis, attested_at }` over
  `AttestedRole { BoundaryReader, Observer }`, and the entry becomes
  `IndependenceAttested`. The two roles are the same claim about the same kind of
  party and share one set of counting rules: the three stake buckets, the
  survives-dissolution rule, the unprunable rule. Only the coherence gate differs,
  because each role is checkable against a different part of the log: a boundary
  reader is fixed at genesis, while **observers accrue**, so that half reads the log
  up to the filing. An accepted API break, taken over a fourteenth sibling entry that
  would have written the bucket logic twice: the copy kith-covenant's §2 slice
  deliberately avoided by factoring `check_roster_caps`.

  A consequence worth naming: **a monitor must speak before it can be vouched for.**
  Pre-registering a witness is the sockpuppet move with a longer setup. And a key may
  hold both roles, where **a voucher for one is not a voucher for the other**:
  vouching is for a party *in a capacity*, since what an outsider can speak to is a
  relationship, not an identity in the abstract.

  **The read-models.** `independence(archive, polity, role, as_of)` with
  `reader_independence` / `observer_independence` as thin wrappers;
  `independent_observation`, the observer counterpart of D24's
  `independent_corroboration`, which also reports `unvouched` so that the absence of
  vouching is visible rather than silently read as its presence; and
  `independent_grounding`, which says how many of the founding edges that earned
  `Guarded` rest on a monitor somebody was willing to be named for. That last is a
  separate accruing model rather than a field on `founding_legitimacy`, on that model's
  charter-alone property. `CohesionReport` gains `watchers`
  (`distinct` / `vouched` / `contested`), reported beside the escalation it qualifies
  and read by no component.

  **Also found:** the tests. Switching the grade to distinct watchers broke **thirteen
  green tests** (six in Rust (`en-charter`, `en-host`) and seven in Python
  (`test_monitor`, `test_store`)) every one of which satisfied a floor justified as
  "somebody was watching" using a single monitor filing repeatedly. The fixtures
  (`observed_polity`, en-host's `Fixture`, `_observed_log`, `_absent_machine`,
  `_watched`) each held one `monitor`. This is the shape D27 found and D23 found before
  it: the tests asserted the hole was correct. All rotate over three now, and the
  lone-actor case is a test of its own.

  **The honest limit.** Distinct keys are still cheap: five minted monitors are five
  real watchers and the count says so truthfully. What D28 buys is that a reader of a
  report can tell three independent monitors from three keys, where before it could not
  even tell three keys from *one key filing three times*. Beyond that the lens depends
  on outsiders choosing to write attestations, and nothing compels them.


- **D29: the rite has movements, and the substrate is required of those who govern.**
  D13 left one thing owed: *"the multi-movement choreography of the rite (v1 records its
  completion, not its movements)"*. Measuring it first turned a design item into a
  design item plus two defects.

  **What the doctrine settles.** `recognition_protocols.md` §"The intra-community layer"
  places the five-movement rite at exactly this tier: each community performs it *"on
  its own envoys and representative machines… in the language of the community's own
  substrate"*, and *"the five movements carry their full content at this layer"*. So the
  movements belong to the polity's own rite, and KC's `Recitation` (five movement
  strings, refused when any is empty) is the shape one level up.

  **What v1 recorded.** Measured: an induction carried `{ polity, member, kind,
  witnesses, affirmed_at }`, so a rite held over a week with an officer presiding and one
  performed in a second by a majority signing a blank produced **identical record
  shapes**, both authorized. There was nowhere to put the week, the movements, the
  presiding, or what the inductee received. The only cost the rite carried was turnout.

  **F1: induction was decorative for authority.** `prior.inducted` was read at exactly
  one site: the affirmation rule itself (no double induction, no re-affirmation before
  induction). No other rule in the tier read it. Measured, a roll of three inducted
  founders plus six members who never were, **every pass green**: the un-inducted
  majority conferred `Inductor` on one of its own, amended the polity's commitments,
  expelled a founder, and presided over a newcomer's induction into a substrate none of
  them had received. The lens saw it (`stale_members` 6 of 9): legibility without
  refusal, which is the line kith-covenant's §2 had just crossed one tier up.

  **F2: "inducted by ratifying the charter" was not what happened.** D13's own sentence
  says ratifying is a founding member's induction; the seed read the founding **roll**,
  so a roll of seven ratified by four reported **seven** inducted, the three non-signers
  dated `founded_at`. D19's "somebody was watching" shape again: a load-bearing
  sentence the code did not implement. The two sets are now distinct in the type:
  `founding_members` is the boundary D21's exclusions and dissents are measured against,
  `ratifiers` is the induction seed, and a listed founder who never signed is on the roll
  and un-inducted, an honest state the tier already had a name for.

  **The record.** A fourteenth entry, `RiteMovementRecorded(Signed<RiteMovement>)`, over
  `Movement { Substrate { names }, Fallenness, Recognition, CommonVirtue { commits_to },
  NonUltimacy { names_anchor } }`: each variant carrying only what its own movement can
  be held to, so a rule cannot check the wrong field and a movement that names nothing
  checkable says so in the type. The inductee recites and signs; everyone attending
  co-signs, and **attendees must already hold the substrate**, because the substrate is
  transmitted by those who hold it.

  **Cited, not carried**: deliberately the opposite of D15/D27's rule for guardian
  evidence, and for the reason that rule exists: a citation is unsafe when what it points
  at can be *pruned*, and `is_prunable` matches nothing but an observation. A movement
  cannot vanish, so citing is sound, and citing is what puts the rite in the log *as it
  happens*, which carrying it inline would hide.

  **Order is semantic; duration is not required.** Movement 1 always opens a rite (and
  re-opening abandons any attempt in progress, so an unfinished rite cannot deadlock a
  member forever); every other movement must be the immediate successor of the last, and
  may not predate it. A rite performed in one sitting is legitimate (the doctrine never
  demands a span, only that the movements happen) so duration became a **lens**.

  **Coherence with the charter, never content.** Movement 1 must name the chartered
  substrate; movement 4 must commit to exactly the commitments in force at that instant;
  movement 5 must name the chartered anchor, and names nothing where the charter is
  `Bare` (D23's honest state). Movements 2 and 3 are presence-only: no rule can read
  contrition or recognition, and a gate that pretended to would be the hollowest thing in
  the tier. This is D24/D28's line: coherence with the record is checkable, the truth of
  the claim never is. (Writing the demo tripped this rule against a hardcoded substrate
  name, which is the rule working.)

  **A witness counts only if present for every movement.** The completion cites its
  rite's five CIDs in order and may name as witnesses only the intersection of the
  movements' attendance. Otherwise a "week-long rite" is five records with disjoint
  signers and nobody attended it, which is **D28's arithmetic one record over**: count
  the people present, not the signatures collected.

  **F1's fix: the majority is of the members who hold the substrate.** `seasoned_members`
  and `deciding_weight` are the one representation, read by every deliberative threshold
  in the tier (amendment, admission, expulsion, conferral, revocation, the three
  guardian arms, both envoy routes, pruning, dissolution, induction). An un-inducted
  member may not hold an office (refused at conferral *and* dropped from
  `effective_holders`, lapsing and reviving exactly as the roll rule does) and may not
  attend a rite. `is_machine_majority` deliberately still reads the whole roll: it asks
  what the polity is made of, not who decides.

  **The denominator is the inducted roll, not the whole roll.** If un-inducted members
  padded the denominator while carrying no weight, admitting people would be a way to
  freeze a polity: the mirror hazard, and worse than the one being closed. They are
  simply not counted on either side. The honest consequence is that a polity can expel
  its way to nobody holding the substrate, at which point it can take no act at all;
  `en-cohesion` names that state rather than the authority pass inventing a rescue.

  **What the monitor gained.** `rite_state` reports each rite's movements, span,
  attendance-throughout and attrition, and which rites were begun and never finished.
  `en-cohesion` grades a **cheapening ceremony** (the latest completed rite shorter than
  the first: against the first, not the median, because a uniform drift moves its own
  recent history, which is `gap_widening_since_founding`'s lesson one measure over) and
  **attrition within a rite**, both capped at `Notice` like the other trends; abandoned
  rites are reported and not graded, since nothing in the doctrine says how long a
  catechumenate may take; and `nobody_holds_the_substrate` reaches `Concern` alone,
  event-style with no threshold, like a ground's withdrawal.

  **A consequence worth naming: D14's inductee clause is now unreachable.** "An
  `Inductor` other than the inductee" was written against a case that can no longer be
  constructed, since an office requires the substrate and an inductee by definition lacks
  it. Kept, with the shadowing stated where the rule lives, on the same reasoning as
  kith-covenant's self-CID exclusion: it is the rule's own statement of what it means,
  and a change that allowed re-induction would need it again. Its test now asserts the
  stronger rule that replaced it.

  **The tests asserted the hole, for the third time in three slices.** 33 Rust tests and
  23 Python tests failed on the weight rule alone: almost all of them fixtures that
  meant "a healthy polity" while ratifying two of three, which under D29 leaves a member
  who carries nothing. `charter_seasoned` (Rust) and an all-ratifying `found` fixture
  (Python) say it now. Three needed real rewrites, and one is worth keeping: the ripple
  test asserted that *admission* moves a later quorum. It does not, and should not: what
  moves it is the point at which the new member acquires the substrate.

  **Honest limits.** Nothing measures whether a rite was *meant*; presence and coherence
  are all a rule can reach, and a polity determined to hold empty ceremonies can recite
  five coherent movements in five seconds: the span lens is what makes that legible
  rather than invisible. Re-affirmation kept its bare self-signed form here: the
  doctrine's *"periodic re-affirmation includes a full re-recitation of the five
  movements"* is the compact-tier generational cadence, and extending movements to
  renewal was named as follow-up rather than taken silently. **D34 closes that
  deferral** — and found the tier was not merely omitting the re-recitation but refusing
  it, so a polity performing the doctrine's own ceremony produced five violations while
  one clicking an empty record passed clean.


- **D30: one writer, and a way back for a log two writers forked.**
  D20 left the item in one clause: *"nothing LOCKS a store (two writers fail loudly but
  are not prevented)"*, and assigned it to the host. Measuring it overturned both halves
  of that sentence.

  **The failure is not loud.** Four scenarios, run against the pre-D30 code:

  1. **Two hosts appending.** Both saves returned `Ok`, and storage became
     **unloadable**: *"segment starts at 5 but this log ends at 7"*. `recover()`
     reported **0 bytes**, because the file was not *torn* (every line complete) but
     *forked*. A polity's archive was left permanently unreadable **with no repair path
     in the crate at all**, which is the worst state in the tier: the log is the
     evidence.
  2. **A stale host compacting.** The silent one, and the one that decided the slice.
     Host A appended two entries and fsynced them; host B, which had never seen them,
     pruned and therefore rewrote. B's save returned `Ok(rewrote)`, the reload succeeded
     and was well-formed, `verify_all` and `check_authority` were both green, and A's
     entries were **gone**. Worse, A's cursor still read generation 0 index 7, so A's
     next save reported `Nothing` and never re-wrote them. **This is D20's own failure
     (a compaction quietly losing signed records) reappearing across *writers* instead
     of across *cursors*.**
  3. **Two concurrent rewrites.** Both opened the same `<path>.tmp`, wrote into it, and
     raced the rename. The winner's save returned `Err(rename: ENOENT)` while its 256941
     bytes were what storage held; the loser's returned `Ok(64941)` for a log that was
     gone. **Both outcome reports were wrong, in opposite directions**, and the file
     loaded cleanly so nothing downstream found out.
  4. **The same two patterns on `SqliteStore`: identical outcomes.** SQLite's own file
     locking prevented database corruption and prevented **neither** logical failure.

  The root cause is one sentence: **`save` decided from `self.stored`, an in-memory
  belief about shared storage, and never checked that belief against storage.**

  **Why exclusion moved into the tier, re-litigating D20.** The argument that put the
  cursor in the store puts this there too: the store owns the cursor *because* a caller
  keeping one caused the bug, and "am I the only writer of this file" is the same class
  of belief about shared storage. Scenario 4 is the evidence that it is not an
  operational concern a careful host can discharge: SQLite had real file locking
  underneath and lost the entries anyway. What stays the host's is the **policy** on a
  refusal: retry, back off, or exit. That is the same line D20 drew around the clock:
  the tier decides *what is safe*, the host decides *what to do about it*.

  **Two mechanisms, because neither reaches the other's case.** A **compare-and-swap**
  (`check_not_forked`) reads where storage actually ends immediately before writing and
  refuses if that is not where this store left it, which catches a stale belief even
  when the two writers never overlap in time, where there is nothing for a lock to
  contend for. An **exclusive lock** closes the case the CAS cannot reach: two *fresh*
  stores, neither of which has a belief to check. Both live in `en-store`; the CAS lives
  beside `plan_save` so the two backends cannot drift into separate ideas of when a write
  is safe, which is the reason `plan_save` is shared in the first place.

  **Mechanism, and the traps in it.** The lock is `flock(LOCK_EX|LOCK_NB)` on a
  `<path>.lock` **sidecar, not the store file**: a rewrite's `rename` swaps the inode, so
  a lock on the store file would end up held on a file with no name while the next writer
  locked the new one. `flock` releases when the descriptor closes, **including on process
  death**, so a leftover `.lock` file is not a stale lock and there is nothing to reap.
  Non-blocking, because a library on the host's loop does not block (`en_at::source`
  states the rule for event sources). `FileStore::claim` holds it for a store's lifetime
  so a host discovers a rival at boot rather than at its first save. Storage's end is read
  from the **tail segment's header alone** (`en_archive::segment_bounds`, entries skipped
  via `IgnoredAny`), O(tail), not O(log), the distinction D20 already had to fix once in
  `iter_from`. `SqliteStore` needs none of the sidecar: its check goes inside the same
  `IMMEDIATE` transaction as the write, which is a genuine atomic CAS, and that is where
  that backend earns its place a second time.

  **Reading is never locked**, and this is a decision rather than an omission. `load`'s
  only hazard is reading mid-append, already a *named* error (the torn tail), while
  `rename` gives a reader the old log or the new one and never a mixture. Locking reads
  would let a writer's claim block a reader, and a reader must always be able to read the
  evidence.

  **A repair, because refusing forward does nothing for a log already forked**: from two
  writers before this slice, or from a restored backup, or an `rsync`. `reconcile` lifts
  the orphaned segments' entries out and **re-appends them at the end of the surviving
  log**, then rewrites storage as one base segment. Nothing is forgotten and **no CID
  changes**: `Entry::cid` is `Cid::of(self)` (content-only, not chained) so a receipt
  naming a moved entry still resolves, which is the property that let D17 carry evidence
  rather than cite it. Duplicates are rejected by CID, so a replayed segment moves
  nothing and the repair is idempotent. Dropping the orphans was the alternative, and it
  was refused for the reason `recover` exists: discarding signed, valid records because
  two writers raced is the failure this crate was written to stop.

  **The judgment call, stated rather than buried.** `reconcile` *decides the append order*
  between two writers that were genuinely concurrent, putting the orphan's entries after
  the surviving log's. Neither order is more correct (the writes were concurrent) but a
  state folded in order can differ between them, so the repair reports exactly which CIDs
  it moved.

  **The tests asserted the hole, for the fourth slice running, and this one passed for
  the wrong reason.** Exactly one green test broke:
  `a_second_store_over_the_same_file_cannot_append_from_a_stale_cursor`, whose comment
  claimed two writers *"must fail loudly"* while its assertions were `save(..).unwrap()`
  and `assert!(out.rewrote)`, i.e. it asserted the stale writer's rewrite **succeeding**.
  It looked harmless only because both writers saved the *same* compacted archive, so the
  overwrite replaced the log with identical content and nothing was visibly lost. Give
  them different content, as two live hosts have, and the same code path is scenario 2.
  This is D28's `the_most_sparsely_watched_member` shape again: one green test isolating
  nothing.

  **And the reference host caught a defect in the fix**, which is what a reference host is
  for. `load` on a path that does not exist yet legitimately reports a zero-entry log, and
  the first version of the CAS read that as *storage having been emptied*: refusing a
  polity its founding save, against the crate's own promise that a first run needs no
  special case. Absent storage and an empty generation-0 log are now normalized to the
  same fact, and the confusion it must *not* be (a store that wrote a real log and comes
  back to nothing) keeps its own distinct diagnosis.

  **Honest limits.** (1) `flock` is advisory and POSIX-only; on a non-unix target, or a
  filesystem whose `flock` does not carry across hosts, the CAS carries alone and what is
  lost is the early clean refusal, not the guarantee. (2) The CAS is a **refusal, not
  coordination**: two hosts that genuinely both want to write must still be resolved by
  whoever supervises them; the tier's contribution is that neither can destroy the log
  while they argue. (3) One control did not fire and is reported as such: moving the
  SQLite check outside its transaction broke nothing, because the test's writers act
  sequentially and never contend the window. The `IMMEDIATE` placement rests on reasoning
  about that window rather than on a test that distinguishes it.

- **D31: the cycle crosses to Python, and a grade says what it was not given.**
  D30 closed with one clause still owed: *"`en-host` is Rust-side only: a Python host gets
  `FileStore` and the read-models, not the cycle."* Read as ergonomics that was a small
  item. Measured, it was not.

  **The measured hole.** `_core` exported 48 names and none of them was the cycle, so a
  Python host reimplemented two things: the cadence bookkeeping, which is tedious, and the
  read-model assembly, which fails silently. `cohesion_report` takes seven optional
  readings and each omitted one makes its component report **nothing**, which no reader
  can tell from health. So: a healthy three-member anchored polity, three rotating
  monitors, and its chartered ground files a `SubstrateDisavowal`, which is a `Concern`
  alone with no threshold (D23). Through `HostCycle::grade` that is
  `erosion=Concern, ground_disavowed=true`. Assembled the way ethne's **own Python demo**
  assembled it, one of the seven supplied, it is `erosion=None, ground_disavowed=false`,
  both passes green, nothing amiss anywhere in the report. Omitting `ground` alone loses
  the escalation by itself.

  **This is D23's finding #4 recurring, one level out.** That slice found that a component
  whose input `grade` forgets reports health rather than complaining, and closed it with
  two `en-host` tests pinning `grade`. Which means the safety lived in a *test of one
  crate*, and every caller off that path inherited the hazard whole. The Python side had
  no such path at all.

  **(A) The report carries what it was not given.** `CohesionInputs::omitted()` names the
  absent readings; `CohesionReport::omitted_readings` carries them, so the answer travels
  with the grade rather than with the call site; `silent_measures()` maps them to the
  measures that therefore said nothing, which is the consequence a reader needs rather
  than the cause; and `blind_to_concern()` reports whether any measure that **can reach
  Concern** was silent, so `erosion == None && blind_to_concern()` finally distinguishes
  *no news* from *I was not looking*. This is **D28's `unvouched` move one level down**:
  absence made visible instead of read as presence. And it is a **lens, never a gate**. A
  partial grade stays legitimate, because a young polity has held no rites and a caller
  comparing two readings may deliberately want one component; what changes is that the
  report says so. Which inputs count as Concern-capable is a property of the levels and
  not a judgement: `guardians`, `affirmations` (twice over, since
  `nobody_holds_the_substrate` is the only thing that reports a polity unable to take a
  second-class act of its own), `trends` via churn, `window` via sustained absence, and
  `ground`. `rites` is capped at `Notice` and `observers` feeds a lens no level reads, so
  neither blinds a grade to anything it could have escalated on.

  **(B) The cycle itself crosses.** `en-pyo3` gains `en-host` and binds `HostConfig`,
  `DueAction` and `HostCycle` whole. It adds nothing to the graph: `en-host` reaches only
  crates the binding already had, and its own tree is `kc-core`-only. `tick` performs
  nothing on the Python side either, which is D20's line held rather than restated: a
  convenience that saved a Python host from signing would have put keys inside the
  library on one side of the binding only. `grade` is exposed in its own right, because it
  is the assembly a caller otherwise has to know to perform.

  **(C) A Python reference host, and mirroring it found a second defect.** Writing
  `src/python/examples/polity_host.py` against the Rust one exposed something in the Rust
  one: **both reference hosts held a single monitor, so neither ever pruned.** D19's
  retention floor counts distinct **watchers per member** since D28, and one monitor
  filing every day never clears it. Measured on one log: one monitor gives
  `prune_recommendation` `None`, three give `dropped=370`. So 40 ticks and 400
  observations produced **0 compactions**, while the restart section printed a compaction
  count as though it had exercised one, and the entire `Prune` arm, the rewrite branch and
  the generation check were dead in the demo that exists to show them. D28 rotated three
  monitors through five *test* fixtures and left the examples alone. Both hosts now
  rotate three: three compactions, generation 3, and the round trip still agrees CID for
  CID.

  **The transferable lesson, and it is the same one twice in one slice.** A safety property
  enforced only where it is *assembled* is not enforced; and a demo is not a test, so a
  path a demo silently stopped taking goes unnoticed for three slices. Both were caught by
  building the mirror rather than by reading the code.

  **Honest limits.** Legibility, not prevention: a caller who omits `ground` still loses
  the Concern, and `blind_to_concern` only lets the report be read for what it is. Nothing
  makes a hand-assembled grade complete, because refusing a partial one would break every
  legitimate comparison and buy nothing the report cannot now say. `SqliteStore` stays
  Rust-only, and deliberately: binding it would put `rusqlite/bundled` and a C dependency
  on the Python wheel's default path, against the property `cargo tree` checks. Nothing
  new crosses to KC, and the border stays at four artifacts plus the roll.

- **D32: one archive mechanism, and append order as the only order.**
  D1 forked a thin `en-archive` over ethne's own entry type because `kc-archive`
  hardcodes KC's `ArchiveEntry` enum, and named the generic base both should build on as
  "the one extraction worth doing later". It is now built: `kc-log`, a crate beside
  `kc-core` in kith-covenant's tree, holding the `Entry` trait, `Archive<E>`, `Cursor`,
  and the whole versioned dump/segment envelope. `en-archive` is a re-export of it and
  `kc-archive`'s `InMemoryArchive` is a newtype over it, so the duplication D1 recorded
  is gone. `kc-log` is neutral — a log over `kc-core`'s `Cid`, no governance records, no
  dependency on any `kc-*` governance crate — so D1's rule reads "`kc-core` and `kc-log`,
  both neutral"; `en-core`'s closure is unchanged at 53 crates and still carries no
  `chacha20poly1305`.

  **Measuring the fork first found two defects, and neither was the one the deferral
  described.** D1 framed this as tidiness. It was not.

  *The index cost about twenty times what it should, in both tiers.* Both forks held
  `order: Vec<Cid>` beside `entries: BTreeMap<String, E>` keyed by `cid.to_string()` — so
  every read hex-encoded 32 bytes and heap-allocated a 67-character `String` to find an
  entry whose `Cid` the caller was already holding. Measured through the shipped code,
  5,000 entries walked 62 times (the number of non-test `iter()` sites in KC):
  **211.4 ns per entry** in `en_archive::Archive::iter()` and **212.7 ns** in
  `kc_archive::InMemoryArchive::iter()`, against **9.1 ns** for a `HashMap<Cid, E>`.
  `Cid` was already `Copy + Eq + Hash`, so nothing in `kc-core` had to change. The
  measurement is kept runnable as an `#[ignore]`d test in `kc-log`.

  *KC read hash order where it documented recency, and where supersession was required.*
  `community_identity` was documented "the most recent self-certified CommunityIdentity"
  and read `entries.values().rev()` — descending *CID-string* order, unrelated to when
  anything was published. Over 200 trials with two publications for one DID it returned
  the **earlier** record **104** times. The same index was scanned by `resolve_did`'s four
  registry steps, which took the first match they found there; those rosters supersede by
  latest `issued_at`, and `authority.rs` already claimed the archive resolved keys with
  "the same latest-issued-wins supersession" — true only of delegations. Because only
  `CommunityIdentity` binds a DID to its pubkey, a roster could name a witness DID with a
  new key while the old entry was still inside its window, and the **retired key still
  verified**. Both halves now share one definition, `store::latest_rosters`, which is the
  rule `authority::mediator_encryption_keys` was already applying alone.

  So the fix is structural rather than a patch: the base's index is a `HashMap`, which has
  no order to mistake for one, it is private to `kc-log`, and the only iteration the type
  offers is append-ordered. Supersession is a domain rule and is now written down as one.

  **What the measurement did NOT find, recorded because the prediction was wrong.** The
  slice predicted that KC's existing tests would turn out to have asserted the hole, the
  way D23, D27 and D28 all did. **Not one of KC's 440 tests broke.** Nothing in the suite
  had ever appended two rosters for one community or two identities for one DID, so
  nothing ever asked which one key resolution would pick — which is why a coin-flip
  answer survived. The absence of a test, not a wrong test, was the cover here. Four new
  tests now hold it: most-recent-publication over 40 independent key pairs, a retired
  witness key refused *and* the current one accepted, one community's rotation not
  blinding another's roster, and a roster issued after the record it would seat.

  **Judgment call, stated rather than buried.** `kc-log` also carries `cursor`,
  `generation`, `next_generation`, `iter_from`, `dump_json_since`, `append_json`,
  `merge_json` and `contains`, all of which this tier's observation log uses. KC has no
  compaction and no incremental-persistence caller, so **none of them is forwarded onto
  `InMemoryArchive`**. Exposed API that no rule reads is the dead-surface pattern D23's
  `substrate.anchor` and KC's `source_commitment` both turned out to be; each is one
  `pub fn` away when KC has something to call it.

  **Honest limits.** KC's dump format moves from version 1 to version 2, gaining the
  `generation` and `start` fields a compaction-safe cursor needs. Old dumps still load —
  `read_dump` accepts the v1 envelope and the legacy bare array, asserted in both Rust and
  Python — but a v2 dump will not load in an older build, and that is a one-way move. The
  ordering fix is also narrower than it sounds for step 1: `CommunityIdentity` binds its
  DID to its key, so hash order there was harmless, and only the four registry steps were
  resolving wrongly. Finally, D20's generation protection is now asserted in `kc-log`'s
  own tests and not independently in ethne's: removing the check fails the base's test and
  leaves ethne's suite green.

- **D33: the AT adapter reaches Python, and the fold crosses the seam rather than the
  peer.**
  `en-at` was the one crate D2's Python surface stopped short of, on the stated ground
  that its real coupling is `at-ffi`-gated and that a Python caller could reach the same
  path by hand through `founding_candidate`. Both halves of that reasoning were wrong, and
  measuring is what showed it.

  *The gating premise was wrong.* Almost nothing in `en-at` needs `at-ffi`. The fold, the
  replay source, the change stream, `has_private_boundary`, `observed_len`,
  `founding_candidate`, `member_reputation` and `signing_key_did` are all pure Rust,
  available with the feature off. Only `AttachedAtDaemon` and `EmbeddedAtNode` are gated.
  There was never a dilemma about the crate — only about those two.

  *Reaching the path by hand was not a path.* `ethne._core` exported 43 names and **none**
  turned an ed25519 public key into a `did:key`; the only DID a Python caller could obtain
  came from `generate_ed25519_keypair`, for a key it had just generated. A host holding an
  observed peer's signing key therefore had to reimplement multicodec-plus-multibase
  itself, with no way to tell whether it had got it right — and a wrong prefix or alphabet
  yields a well-formed DID naming nobody.

  *And the hand-built roll was silently wrong.* `AtPeer::guardian_did` and
  `AtPeer::attended_at` both gate on `operator_bound`, because a guardian key or an
  attendance stamp on a node whose operator credential never verified attests to nothing.
  That gate is invisible from outside the crate. Written as a competent developer would
  write it from the wire fields, the Python fold got the member count, the departure, the
  ordering and every reputation right, and diverged on exactly **two fields, both on one
  peer, both toward health**: it reported a `guardian` DID and an `attested_at` stamp for a
  node AT had marked unverified. So it read an **unguarded machine as guarded and
  attended** — defeating the office rule D15 exists for, fabricating D8's
  human-above-machine anchor out of a key AT declined to vouch for, and handing the
  cohesion monitor a freshness stamp with nothing behind it. Nothing in the output looked
  wrong. This is **D31's** finding again, in the one place D31 did not reach: an assembly a
  Python host reimplements fails silently toward health.

  So the surface takes **wire fields, never an assembled peer**. `observe_peer` accepts
  what AT sends and the Rust fold applies the gates; there is deliberately no Python object
  whose `guardian` or `attested_at` a caller can set. The divergence is unrepresentable
  rather than documented. Re-running the same feed through the bound provider: **0
  divergences where the hand-rolled fold had 2.**

  **The `at-ffi` answer, which was the item's actual question.** A caller sees the live
  constructors **absent** — not present-and-raising, which is the dead-field pattern D23's
  `substrate.anchor` and KC's `source_commitment` both turned out to be — plus
  `at_ffi_available()`, a fact to branch on rather than an exception to catch. The flag is
  *read*, not decorative: `ethne.at` branches on it to decide what to export. Both
  directions are asserted by a paired test that flips with the build.

  **Two properties the binding work discovered.** The live couplings hold raw C handles, so
  they are neither `Send` nor `Sync`: an `at-ffi` build's provider is **thread-bound**
  (`#[pyclass(unsendable)]`), applied per-feature so the restriction appears exactly where
  its cause does. And `observe_peer` / `depart_peer` / `boundary_private` are refused by a
  daemon-backed provider, naming what it is rather than what it lacks — injecting an
  observation into a live feed would fabricate a reading AT never made, which is the whole
  class of error this seam exists to prevent.

  **Honest limits.** **No live AT daemon runs in this sandbox, so the gated half is
  compile-, link- and gate-checked only** — it builds against
  `libautonomous_trust.so` and its names appear and disappear with the feature, but no
  `attached`/`embedded` provider has ever folded a real event here. Separately, the guardian
  *identity* remains unreachable from a live node for reasons that are AT's, not this
  binding's: AT carries `operator_bound` and `operator_attested_at` but nothing saying
  *which* human, so a guardian edge from a live feed is still `MockEventSource`-only. The
  Python surface can express it; AT cannot yet supply it.

- **D34: the renewal carries the rite, because the recurring ceremony was the one with
  no measurable cost.**
  D29 built the five movements and named one follow-up: the doctrine's "periodic
  re-affirmation includes a full re-recitation of the five movements" had no record
  shape, so re-affirmation kept its bare self-signed form. Measuring it found the tier
  did not merely omit the re-recitation. **It refused it.** Three behaviours, through the
  running code:

  | what a polity does | authority pass | freshness | erosion |
  |---|---|---|---|
  | members click a bare `reaffirm_member`, 3 cycles, no rite since founding | clean | 0 of 3 stale | `None` |
  | a member performs the full five-movement re-recitation | **5 violations** — "rite movement for an already-inducted member" | still 3 of 3 stale | — |
  | nobody re-affirms at all | clean | 3 of 3 stale | `Concern` |

  So the pass rewarded the empty form and punished obedience to the doctrine. Worse, the
  faithful re-recitation landed in the log as an **abandoned** rite, because the
  completion it needed could never come: an `Induction` was refused for an
  already-inducted member, and a `ReAffirmation` was refused if it cited any movements at
  all. The ceremony did not even refresh the member's `last_affirmed`, so a member stayed
  overdue after performing it.

  This is the third variant of the corpus's recurring shape, and one worse than the
  second. D29's own F1 was legibility without refusal — the line kith-covenant's §2 had
  just crossed. Here there was not even legibility, because **induction is instrumented
  and happens once per member, while the renewal happens forever and carried nothing**:
  no movements, no span, no attendance. A polity could hold its transmission measure
  green indefinitely at zero ceremonial cost.

  *Required, not optional.* A `RiteMovement` now carries `kind: AffirmationKind`, and a
  `ReAffirmation` must cite exactly five `ReAffirmation`-kind movements for that member,
  in order, forming the rite completed at that point — the induction's citation rule,
  reused rather than restated. A bare renewal is refused. The kind is **carried rather
  than inferred** from whether the member happens to be inducted at that log index,
  because a carried claim can be checked against the state: `Induction` requires that
  they do not yet hold the substrate, `ReAffirmation` that they do, so a mismatch is
  refused instead of silently reinterpreted. That is D29's own principle for `Movement` —
  each variant carries only what it can be held to, and what a rule must check is in the
  type.

  *Recorded, not gated.* `witnesses` must equal the rite's `attended_throughout()`
  exactly, so a member can neither over- nor under-report who was there; it may be
  **empty**, and it is never a threshold. The signers stay exactly the member. This is
  D14's standing rule that self-signed acts "are never gated: they are precisely the acts
  no one else may condition" — attendance is a fact about the ceremony, not a condition
  on it.

  *Nobody attends their own ceremony.* An attendee may not be the reciter. At an
  induction this was already impossible, because attendees must hold the substrate and
  the inductee does not; but a renewing member does hold it, so without the rule they
  would attend their own ceremony and inflate its attendance. D28's arithmetic again:
  count the people present, not the signatures collected.

  *One ceremony completes one affirmation.* Nothing else stops a member replaying a
  single re-recitation to reset their freshness every cycle — the induction path is
  protected by the no-double-induction rule, and a renewal repeats by nature. This
  became reachable only because renewals began citing rites at all, and it is the one
  rule in the slice that the design did not anticipate; reading what the repaired tests
  actually asserted is what surfaced it.

  *The coherence checks now bind renewals too*, which is a real gain and not incidental:
  a polity that amends its commitments and never re-recites becomes visible, because its
  next renewal must commit to the amended set. `Rite` carries its kind and `completed_at`
  is set by the completing affirmation of the *matching* kind, so a renewal awaiting its
  `ReAffirmation` is open exactly as an induction is rather than permanently abandoned.
  `en-cohesion`'s ceremony-cheapening lens filters to `Induction`, its meaning unchanged,
  and renewal-cheapening is a sibling component measured **within** renewals: comparing
  an induction's span against a renewal's would be meaningless, since one happens once
  per member and the other recurs. Each is graded against its own **first**, not its
  median — `gap_widening_since_founding`'s lesson, and the plateau is where the two
  baselines diverge. A ceremony that thinned once and stayed thin has made the cheapened
  form into what its own recent history calls normal. Both are capped at `Notice`, like
  D29's other trends.

  The interval stays a query-time parameter. D29 made the affirmation cadence a query
  argument rather than a charter term, and the doctrine's generational cadence is stated
  one tier up, so `stale_members(cadence_seconds, as_of)` already covers "overdue" and no
  `PolityCharter` change was needed.

  **Honest limits.** A polity with no willing attendees can still renew alone, which is
  the price of D14 and is the right price. Nothing checks that a recitation was *meant* —
  the statement text is required to be non-empty and is never read — which is D24 and
  D28's standing line: this tier measures form, and sincerity is not a thing a log can
  hold. And the *authority pass* is what refuses the bare form; the freshness read-model
  is descriptive of the log and still reports a refused renewal's `last_affirmed`, which
  is the tier's standing separation between describing the record and judging it, not
  something this slice introduced (a four-of-five induction citation has always read as
  inducted). A polity that ignores `check_authority` can still tell itself it is fresh.


## Questions and resolutions (round 2)

- **Q9: AT operator-attended signal (D8 prerequisite).**
  AT must expose, per node and to peers, *which nodes have a human behind them*,
  so `ethne-at` can derive the guardian edge. Resolutions:
  **(a) semantics → both.** Two signals: a durable **operator-bound** attestation
  (the node has a human guardian) and a live **attended-now** freshness (a verified
  operator session is active). ethne's guardian edge uses *bound*; *attended-now*
  feeds cohesion/liveness later. **Both halves exist:** AT
  carries `operator_attested_at` (consumer-pull, no keepalive) and `en-at` folds it
  onto each `MemberCandidate`, where `en-cohesion` grades it as a fourth erosion
  component, capped at `Notice`, never a Concern, because a point-in-time snapshot
  cannot tell a night shift from an abandoned polity.
  **(b) verifiability & placement → credential-hash in the signed Identity.** The
  signal carries the operator's chain-verifiable `zta_credential_hash` (+ issuer)
  inside the *signed* Identity payload: tamper-evident, non-forgeable, reusing the
  ZTA machinery and propagating with peer admission. Not self-asserted; not a
  plaintext envelope bool.
  **(c) sequencing → AT signal first.** Build and verify the AT operator-attended
  capability (C + Python + wire + conformance), mirroring the roster capability,
  *then* scaffold `ethne-at` consuming both roster + signal.

## Questions and resolutions

Every question the design raised, Q1-Q8 here and Q9 above, is answered; each entry
records the question and the decision that settled it.

- **Q1: the ratification quorum. Resolved: gate the accounting, grade
  the judgement (see D21).** A founding may still narrow the cluster it came from,
  but never *silently*: the charter carries the boundary it claims to ratify
  (`candidate_size` plus a reasoned `Exclusion` for everyone the boundary showed and
  the roll does not name) and, the charter's core having no amendment path, carries
  it permanently. Authority refuses a charter whose boundary does not add up;
  nothing refuses one merely for being narrow, because a genuinely separate group of
  three inside a thirty-node cluster is indistinguishable from a faction capturing
  it without a substantive judgement no rule can make. The number Q1 asks for
  (what share of the cluster actually consented) is `founding_legitimacy`'s
  `consent_share`, derivable by anyone holding the charter, forever: the faction of
  one that used to pass a "majority" at a single signature now reads **0.033**.
  Accountability, not prevention; the residue (a captured provider feeding a
  gerrymandered boundary) is **Q5**.
- **Q2: when is a schism a fission? Resolved: ratified, never
  auto-verdicted.** The cohesion monitor (`en-cohesion`) does **not** decide a
  fission: algorithmically splitting a polity would be exactly the "replace
  emergence" move the tier exists to refuse. What the monitor does is grade strain
  into an `ErosionLevel` (`None` / `Notice` / `Concern`, mirroring KC's own
  erosion alert one tier down) off two snapshots: the chartered roll vs. the
  provider's current candidate (attrition, reputation decay, floor breaches, absent
  guardians, transmission decay: members lapsed on re-affirming the substrate,
  read off `en-charter`'s `AffirmationState`, lost guardian edges (D15: a
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
  KC's `ConductKind::Concern`, an **early warning**, the strain that *precedes* a
  split, not a verdict. A fission is a **deliberate re-charter**: the community
  ratifies two successor foundings, and only then does `en-uplift::fission` emit
  the two successor `CommunityIdentity`s, a signed `SuccessorLink` to each, and the
  predecessor's charter dissolution. So there is no erosion *threshold* that
  "turns" one community into two: the split is always a ratified act; the monitor
  only tells the members (and KC) that the strain is real.
- **Q3: office conferral bootstrap. Resolved: the roll confers (see
  D12).** The regress is avoided by making the *roll* the standing conferring
  authority rather than any office: offices never confer offices, so the founding
  roll confers the first offices with the same majority authority that ratified the
  charter, and every later conferral/revocation is likewise a roll-majority act.
  Deliberate (a signed `OfficeChange`, holder consenting), never a reputation
  readout.
- **Q4: substrate for a purely emergent group. Resolved: it must
  anchor, and the anchor must consent (see D23).** The leaning held: a polity with
  nothing above itself is exactly the failure KC's non-ultimacy feature guards
  against, and it charters no longer. A roll of nothing but machines answering to
  nobody must name a substrate anchor outside itself, and that key co-signs the
  founding. A human roll needs no key (its statement *is* the anchor, unkeyed on
  purpose) but every polity must articulate one, and none may name itself or one of
  its own members as its ground. What the tier grades on top of what it gates is
  `grounding`, so a bare-but-legal ground is visible rather than merely permitted;
  and if the ground later withdraws, it says so on the record and the monitor reads a
  Concern.
- **Q5: provider trust. Resolved: attribute the reading, grade the
  agreement, and let the excluded answer (see D22).** Nothing *stops* a captured
  provider, and D22 does not pretend to: what it removes is anonymity. A boundary
  claiming a substrate reading must carry that reading, signed by the reader it
  names and bound to this founding, and the roll plus its exclusions must match what
  that signature shows: identity, where D21 had only arithmetic against the
  proposer's own report. A gerrymandered boundary is therefore attributable to a
  named key, permanently; `founding_legitimacy` reports which readers *agree* with
  what was ratified and which contradict it; and an eleventh entry lets the excluded
  say, unprunably and even after dissolution, that the accounting omitted them:
  the one claim the archive could never check for itself, since it holds no
  provider. Accountability, not prevention. The residue is **sockpuppet
  corroboration** (`did:key` is free), left as a stated limit on a lens rather than
  an authority rule, exactly as kith-covenant left capture move 1; independence
  attestation is the named successor.
- **Q6: `ethne-at` coupling mode. Resolved: support both (see D6).**
  Both *embed* (`at_node_init`/`at_node_start`, owning the daemon lifecycle) and
  *attach* (`messaging_recv_on` on an existing `q_out`) are supported from day one,
  behind the `AtEventSource` seam. Embedding is self-contained for tests/tools;
  attaching fits a co-deployed fleet.
- **Q7: what defines the AT boundary. Resolved: gateway-reputation
  subtree (see D7).** The candidate roll is the enumerated subtree member-roster
  (an identity-process capability): arbitrary-depth,
  privacy-opt-out-aware, with the top gateway as the natural boundary. (Rejected:
  `Group` shared-key set, too flat; reputation-floored peer set, fuzzy and
  requestor-local.)
- **Q8: guardian edge source. Resolved: `Operator` identity (see
  D8).** A member's guardian is its human `Operator`. Natural human-above-machine
  anchor; ties to the AT↔KC integration doc's M2/Q2 and machine-participation
  node→guardian→standing.

## Naming

`ethne` (ἔθνη, plural of *ethnos*, "a people / nation / community") names the
tier's product: peoples, as distinct from *kith* (the recognized cross-community
relation KC governs) and from the anonymous nodes AT knows. Rust crate prefix
`en-` (e.g. `en-charter`, `en-polity`, `en-cohesion`), Python package `ethne`.

## How the tier is assembled

The tier is built up in five layers, each of which stands on its own. The order is
part of the design: every layer proves the decoupling it inherits before the next one
is added.

1. The `ethne` workspace (D2), with `en-core` depending on `kc-core`, and
   `en-archive` over the shared `kc-log` archive base (D1, built by D32 — it began as
   a thin fork over ethne's own entry enum).
2. The **`en-charter` / founding object** (D3), the keystone primitive, with the
   ratify-not-replace discipline built into its API.
3. The `SubstrateProvider` interface and a trivial manual-founding
   provider (no AT), which is what proves KC ⟂ AT end-to-end.
4. The **`ethne-at` adapter** as the second provider: the `en-at` crate
   (D5/D6): the `AtEventSource` seam (mock + `at-ffi`-gated embed/attach coupling)
   folded into `AtSubstrateProvider`, mapping the observed gateway-reputation
   subtree (D7) to the candidate roll and the verified operator (D8) to each
   member's guardian edge. It rests on two autonomous_trust capabilities:
   the subtree member-roster (D7) and the operator-attended signal (D8,
   the "which nodes have a human behind them" signal). `cargo tree` confirms the
   default build's only external dep is `kc-core`: KC ⟂ AT holds with the adapter
   present; the real C linkage is opt-in behind `at-ffi`.
5. The upward interface to KC recognition (D4/D9/D10). `CommunityIdentity`
   + `SuccessorLink` extracted to `kc-core` (D9). `en-cohesion` grades coherence
   erosion (`None`/`Notice`/`Concern`) off the chartered-roll vs. current-candidate
   diff, staying `kc-core`-only. `en-uplift` (the crate holding the confined
   KC-record deps, D10) emits the border artifacts: the `CommunityIdentity`
   handoff, the `Concern` conduct record on `Concern`-level erosion (neutral
   `Other("coherence-erosion")` bearing), a ratified `fission` bundle (N successor
   identities + a signed `SuccessorLink` to each + the predecessor's
   `CharterDissolution`, an `en-charter` charter-lifecycle record, all N-ary per
   **D25**), the envoy `SpeakerDelegation` for whoever the polity has authorized to
   speak for it (**D14**), and the `MemberRoll` (**D26**). `cargo tree`
   confirms all core + monitor crates remain `kc-core`-only and that both
   KC-record couplings (`kc-conduct`, and `kc-identity` per D14) stop at
   `en-uplift`.

Scope this design specifies that the code does not carry, together with the slices
that follow from it, is listed in [`TODO.md`](../TODO.md). One boundary is worth
naming here rather than there, because it is a division of labour and not a backlog
item: the intra and inter ceremony rites for envoy recognition are KC's, once KC
holds the `CommunityIdentity` ethne hands it.

Cross-references:
[kith-covenant/doc/architecture/autonomous-trust-integration.md](../../kith-covenant/doc/architecture/autonomous-trust-integration.md),
[kith-covenant/doc/architecture/protocol-components.md](../../kith-covenant/doc/architecture/protocol-components.md),
[kith-covenant/doc/architecture/machine-participation.md](../../kith-covenant/doc/architecture/machine-participation.md).
