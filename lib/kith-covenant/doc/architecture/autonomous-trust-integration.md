# AutonomousTrust integration: melding AT and kith-covenant, hosted in KC

*This document enumerates the joints between the two models and specifies how they
meld. It supersedes the AT-repo notes `AT_KC_MELD.md` (disconnects and meld
proposals) and `kc_support.md` (the five concrete first couplings), and it inverts
their ownership: the meld is **hosted in kith-covenant (KC), which imports the
`autonomous_trust` (AT) package**, rather than in AT. Companion to
[machine-participation.md](machine-participation.md),
[protocol-components.md](protocol-components.md), and
[policy-evaluation.md](policy-evaluation.md).*

## Purpose

AutonomousTrust (AT) and kith-covenant (KC) are designed to stack: AT carries
anonymous, machine-to-machine reputation *inside* a single community; KC carries
known-identity governance *between* communities, among a mix of humans and
machines. KC's own framing states the seam plainly: "AT exposes upward:
community-key identity, delegated speakers, health indicators, witness
availability. Kith-covenant never reaches inward"
([protocol-components.md](protocol-components.md), decision 5).

The intended layering is heavily *documented*, and a handful of the joints are
conceptually unresolved rather than merely unimplemented: the
two models make different assumptions about what an actor is, what "trust" means,
who confers authority, and how a decision is reached. This document enumerates
those disconnects against the code on both sides, then lays out how the
two meld, with KC as the integration host.

The reader's shorthand ("KC is human+machine and structured; AT is m2m and has
no intentional hierarchy") is close but worth sharpening in one place: **AT is
not flat.** It has rank, trust tiers, valuation buckets, group/gateway trees, and
even a Proof-of-Authority agreement mode. The real distinction is that AT's
structure is *earned and emergent* (computed from behavior and topology, shifting
continuously), while KC's is *conferred and covenantal* (set by signed acts of
designation that persist until revoked). That difference is the spine of most of
the disconnects below.

## Ownership: the integration lives in KC and imports AT

The original notes assumed the seam would be built *into* AT: an AT-side
`kith_covenant` shim, observer, and enforcement all living in the AT tree, with
imports resolving from within AT. **That is inverted here.** The runtime
integration is a KC concern: KC imports the `autonomous_trust` package and hosts
the observer, the bidirectional adapter, and the enforcement writer. AT is a
dependency of KC's integration layer, not the other way around.

This is a refinement of the split already recorded in
[policy-evaluation.md](policy-evaluation.md), where the KC *library* owns the
policy schema and event types and the AT simulator imported them (AT→KC). The
schema stays exactly where it is; what moves is the **runtime**, so the
dependency for the live integration now runs KC→AT. Concretely, KC's Python
package splits into two layers with a one-way dependency between them:

| Layer | Location | Owns | Depends on |
|---|---|---|---|
| **`kith_covenant` core** | `src/python/kith_covenant/` | event types (`interface.py`), policy schema (`policy.py`), evaluator protocol, the Rust-backed protocol surface (`_core` via PyO3) | nothing AT-specific |
| **`kith_covenant.integration`** (new) | `src/python/kith_covenant/integration/` | the observer, the bidirectional AT↔KC adapter, the KC→AT enforcement writer, the five-feature enforcement hooks | `kith_covenant` core **and** `autonomous_trust` |

The core library stays AT-free, so the policy schema remains stable and reusable
(hand-crafted and future ML-generated policies still target the same types, see
[policy-evaluation.md](policy-evaluation.md) on why that loop does not violate the
non-emergence claim). Everything that must touch AT lives in
`kith_covenant.integration`, which `import autonomous_trust`. AT already exists
and is imported; where a joint genuinely requires a new AT-side field or hook
(noted per-proposal below), that is a *minimal, exposed* extension in AT: the
decision logic still lives in KC.

The material the bridge is welded from: **both are Ed25519.** AT `Signature` and
KC `Signed<T>` are the same primitive.

## Grounding: the two models side by side

| | **autonomous-trust (AT)** | **kith-covenant (KC)** |
|---|---|---|
| Scope | Cooperation *within* one community | Cooperation *between* communities |
| Stack | Python + C twin; multiprocessing `Process` fabric | Rust workspace (`src/rust/kc-*`) + PyO3 |
| First-class actor | **Node** (a machine process) | **Community** (root `did:key`) |
| Identity | `uuid4` + Ed25519 (`Identity`, `sign.py`/`encrypt.py`); Zooko nickname/petname; optional X.509/PIV via `identity/zta/` | W3C `did:key` from root Ed25519; scoped delegate keys via `SpeakerDelegation` |
| Individuals | *are* the nodes | scoped delegate keys (`Envoy`, `Reciter`, `Witness`, `Mediator`) |
| "Trust" | continuous behavioral **reputation** `[0,1]` (CTFT / pure-rep, EMA, idle-decay) in `reputation/repprocess.py` | discrete adjudicated **standing** (`RecognitionStanding`: Unknown/Identified/InCompact/Probationary/Denied/Anathema) |
| Structure | earned/emergent: `rank`, `tier` (0-4), `valuation`, `Group`, gateway tree | conferred/covenantal: root → delegation scopes; mediator/witness registries |
| Decisions | automatic: leaderless Byzantine Multi-Paxos + quorum co-sign | deliberative: mediation panels, escalation ladder, 7-day deliberation cycles, distinct-community quorum |
| Enforcement | *emergent*: below `COMM_CUTOFF` gateways stop forwarding; slashing floors reputation | *issued*: `InterimInjunction` revokes named `Capability`; `has_capability()` gates |
| Tempo | machine (ms consensus, continuous scoring) | deliberative (7-day gaps, 90-day injunctions, generational re-affirmation) |
| Humans | the **Operator**, "the single human touchpoint," PIV+MFA, a *request-only* node | assumed throughout (oaths, rites; "a specific named human whose social standing is at stake") |
| Machines | all nodes | *derived, non-equal*: guardian-bound delegates, "not full members" |
| Transport | UDP/TCP mesh, protobuf `Message`, multicast | **none**, `InMemoryArchive`; presupposes AT + muudd; libp2p is deployment work |

## Points of disconnect

### A. Ontology and identity

**A1: AT has no "community" object for KC to bind to.**
KC's only first-class actor is the community (a root `did:key`,
`kc-identity/src/community.rs`). AT has no community type. Its nearest analogues
(a `Group` (a shared-symmetric-key membership set, `identity/group.py`) and a
gateway subtree) are *emergent topological clusters*, not named, root-anchored,
covenant-bearing entities with a membership roll and a standing. So when KC says
"AT exposes upward community-key identity," there is today nothing in AT that
*produces* a community-key identity: AT identities are strictly per-node. KC has
an anchor point; AT has nothing to hook onto it.

**A2: The identity namespaces don't meet.**
Both are Ed25519 underneath, but AT identity is `uuid4` + key + Zooko
nickname/petname (plus optional X.509/PIV ZTA credential), while KC identity is a
`did:key` plus a *delegation graph*. AT has no DID, no delegation records, and,
most of all, no notion of one key *speaking for a principal it is not*. KC's whole
attribution model (sign with a delegate key, attribute the record to the
community principal, resolve through `SpeakerDelegation`) has no counterpart in
AT, where a signature simply *is* the node.

**A3: Earned/emergent hierarchy vs conferred/covenantal authority.**
AT can tell you who is *reachable* (`rank`, `effective_rank`), who is *trusted by
track record* (`tier`, `valuation`), and can even run Proof-of-Authority voting
(`algorithms/authority.py`). But every one of those is computed from behavior or
topology and drifts continuously; none is *conferred by an act of designation*
that persists as a status. KC's authority is exactly the opposite: `root`,
`Envoy`, `Mediator`, `Witness`, `sponsor` are conferred by signed
delegation/registry records and hold until revoked, regardless of behavior. There
is no AT surface onto which KC's `DelegationScope` / `MediatorRegistry` /
`WitnessRegistry` can land: AT has no representation of "appointed to
speak/mediate/witness."

**A4: Community-as-party has no AT representative.**
KC treats a *community* as the party to a dispute, sponsorship, or injunction. For
that to ride AT, some AT construct must present a whole group/subtree as a single
addressable principal holding one standing and able to sign KC records. AT's
gateway-reputation-tree (`doc/architecture/gateway-reputation-tree.md`) already
aggregates a subtree's reputation and can answer for its roster (the natural
external face of a community) but it is a reputation aggregator, not a principal
that can hold a KC standing or sign on the community's behalf.

### B. Trust vs standing

**B1: Two different meanings of the word.**
AT "trust" is a probabilistic, continuous, decaying behavioral estimate. KC
"trust" is adjudicated relational *standing*: a discrete status set by ceremony
and judgment. AT reputation collapses and recovers automatically from behavior; KC
standing changes only through recognized acts (admission, sponsorship,
designation, re-entry). Nothing today maps one to the other, and the map is not a
rescaling: it is a **category change from measurement to adjudication.** Wiring
them together naively (e.g. "reputation < X ⇒ Anathema") would be a category
error and exactly the kind of automatic loyalty-cascade KC is built to prevent.

### C. Decision-making and tempo

**C1: Automatic vs deliberative.**
AT reaches collective decisions by leaderless Byzantine Multi-Paxos and quorum
co-signing, no argument, no appeal, machine tempo. KC reaches them by mediation
panels (≥3 non-party mediators), escalation-ladder transitions, deliberation
cycles separated by a 7-day waiting period, and quorums of *distinct
communities*. AT has no dispute intake, no panel, no argument record, no appeal
channel, no waiting period. An AT reputation event that *should* become a KC
dispute has no path to become one; a KC decision has no path back into AT's
automatic machinery.

**C2: Tempo mismatch.**
AT runs at milliseconds; KC runs at days-to-generations (7-day deliberation gaps,
90-day injunctions, generational re-affirmation). Any meld must bridge tempo
*without either side blocking on the other*: machine-tempo detection must
**propose** into human-tempo adjudication, and human-tempo verdicts must **latch**
into machine-tempo enforcement, an event-driven coupling, never a synchronous
call.

### D. The human / guardian axis (the heart of "human+machine" vs "m2m only")

**D1: KC's whole answer to machine participation has no AT anchor.**
[machine-participation.md](machine-participation.md) is unambiguous: a machine
participates only in a *derived* form, "irrevocably tied to one specific named
human" (the **guardian**) whose community standing is at stake in the machine's
conduct. The binding rests on four layered mechanisms (constitutive objective,
cryptographic self-binding, continuous attestation, principal-referential
identity + audit commons). Yet:
- KC has **no guardian type in code**, the guardianship relation, the fan-out
  cap, attestation, and weight-integrity are all doc-only
  ([machine-participation.md](machine-participation.md) Facets 1-3), not types.
- AT has **no place to record a node's guardian.** AT's Operator
  (`operator/activate.py`, "the single human touchpoint") is a *requesting* human
  (a request-only node that consumes capabilities), not a *guardian who owns a
  machine's standing*. AT `Identity` carries no principal/guardian field.

So the load-bearing relation of the entire machine layer (node → guardian →
community standing) is asserted in prose and absent from both codebases' types.
This is precisely the "the two don't quite meet up" the reader identified: AT is
m2m with a thin request-only human touchpoint; KC needs a thick human-as-guardian
who *is* the node's substrate.

### E. The enforcement coupling: one-way, thin, and on the wrong side

**E1: The existing bridge only observes, and only AT→KC.**
The current link (`autonomous_trust/evaluation/kith_covenant/observer.py`,
`is_tee_observer = True`) tees AT protocol messages, maps them to four KC-flavored
events, and scores them against a `KithCovenantPolicy`. It **measures**
compliance; it never **enforces**, and it flows strictly AT→KC. There is no return
path by which a KC `InterimInjunction` sets an AT capability gate, or a KC
`Designation` (anathema) floors an AT reputation. KC's "teeth" (`has_capability()`)
terminate in KC's own archive, not in AT's negotiation-admission or
gateway-forwarding. Under the ownership inversion, this observer's *logic* becomes
a KC-hosted module (`kith_covenant.integration`) that imports AT to tee its bus,
rather than an AT-tree module that imports KC.

**E2: The event vocabulary is AT-shaped and thin.**
The seam (KC's `src/python/kith_covenant/interface.py`, `policy.py`) models only
`PeerAdmitted`, `ReputationUpdate`, `NegotiationStarted`, `NegotiationOutcome`, and
a policy of numeric thresholds (`min_reputation_to_negotiate`,
`defection_cooldown_s`, `max_group_size`, `bandwidth_priority_by_rank`). It has
*no* event for anything KC actually governs: no community admission, sponsorship,
conduct/concern, dispute, injunction, re-entry, or attestation. So even the built
seam can only lift AT's intra-community reputation up to KC as a fitness metric; it
cannot carry KC's inter-community governance acts down to AT.

**E3: The runtime seam isn't stood up on the KC side yet.**
The event and policy modules already live in the KC core library
(`src/python/kith_covenant/{interface,policy}.py`), so KC does not need a shim to
*hold the schema*. What is missing is the KC-side **integration package** that
imports `autonomous_trust`, hosts the observer, and adds the return path. The
existing AT-tree observer imports KC top-level `kith_covenant.*` and its
`test_kith_covenant_*` suites skip when the package does not resolve from within
AT; moving the runtime to KC and importing AT removes that fragility (Phase 0
below).

**E4: No attestation anywhere on the seam.**
KC's machine binding leans on *continuous* attestation (TPM/enclave/ZK,
"cryptographically integral policy, attested at every inference") feeding
capability-gating. AT has ZTA credentials at *admission* and a stubbed `_zkp`
layer, but nothing that continuously attests policy integrity, and no event by
which an attestation failure becomes a KC standing/capability change.

### F. Membership lifecycle

**F1, admission: welcoming-committee vs recognition ceremony.**
AT admission is machine-automatic: broadcast `request_access` → border-guard
`welcoming_committee()` → ZTA gate (`_zta_admit`) → quorum vote → two-phase
provisional→confirmed → group-key handover (`identity/idprocess.py`). No vouching
human, no oath, no substrate check. KC admission is a recognition ceremony: an
intra-community rite (five `Movement`s, witnessed, root-sealed) + an
inter-community anti-spiral feature exchange + *sponsorship* (a named sponsor
stakes standing). AT has no sponsor, no rite, no anti-spiral check; its
"membership" is key-in-group, not covenant standing. There is no hook in AT's
admission path to require or record a KC recognition/sponsorship before admitting
a compact-facing node.

**F2, exclusion and restoration: aligned in spirit, unwired in fact.**
This is the *most* aligned pair. Both believe in reversible exclusion + a
restoration path: AT re-admits via `REASON_REHABILITATE`; KC runs restitution
obligation → verification → re-entry ceremony, with anathema reversible and
*hostis humani generis* not. The five features below mirror KC's rules one-to-one
(never-withhold floor, restoration-path completeness, appeal-above channel). But
nothing yet connects an AT rehabilitation to a KC re-entry, or forbids AT from
permanently excluding a compact-facing node with no KC restoration path. The
alignment is latent, not realized.

### G. The physical seam

**G1: KC has no transport; AT is the substrate.**
KC's archive is `InMemoryArchive`, single-host, append-only, JSON dump/load;
libp2p replication is deployment work (KC's [`TODO.md`](../../TODO.md)). KC's
content-addressing (`Cid`, canonical CBOR,
`Signed<T>`) is transport-ready, but the transport is meant to be supplied from
below: muudd for comms, AT for identity/health/witness availability. How KC
archive entries replicate *over* an AT mesh (which nodes hold the archive, how a
new community bootstraps a view, how partition recovery interacts with
append-only archive semantics) is entirely unspecified. AT's own
partition-recovery and gateway-tree machinery are the obvious substrate, but no
design connects them to archive replication.

## How they meld

The guiding principle: **do not collapse the two models into one.** Keep AT's
earned, emergent, machine-tempo reputation exactly as it is, and keep KC's
conferred, deliberative, human-tempo governance exactly as it is. Meld them at a
**seam of translation** owned by `kith_covenant.integration`, with two orthogonal
axes carried side by side and an event-driven, bidirectional coupling between
them. Every proposal below builds on something that already exists.

### M1: A community-binding object (fixes A1, A4, G1)
Introduce a `CommunityBinding` that maps an AT `Group` / gateway-subtree to a KC
community DID, held by the group's gateway/root node. The gateway becomes the
community's external face: it holds the community's KC root (or an `Envoy`-scoped
delegate) and presents the subtree to KC as one `CommunityIdentity`. Reuse the
**gateway-reputation-tree** as the community's external reputation surface, and
let archive entries replicate along the gateway tree (a first answer to G1). This
gives KC's "upward-exposed community-key identity" a real producer. The binding
record and its resolution live in `kith_covenant.integration`; the only AT-side
need is a stable handle to the gateway node, which AT already exposes.

### M2: A guardian directory built on the Operator (fixes D1)
Promote AT's Operator from *requester* to *guardian*. The Operator is already the
authenticated human touchpoint (PIV + MFA, `operator/activate.py`); let an
operator credential **vouch for and own the standing of** one or more machine
nodes, with the fan-out cap and standing-weight decay from
[machine-participation.md](machine-participation.md) Facet 2. This needs one
minimal AT-side field (an optional `guardian` reference on AT `Identity` (a
KC-facing human key/DID)) exposed for the integration layer to read; the
directory, the fan-out cap, and the weight-decay policy live KC-side. This is the
concrete realization of "machine tied to one named human": AT records the guardian
edge; KC's `SpeakerDelegation` attributes the node's records to the community
*through* that guardian. The guardian-fall / guardian-capture / succession cases
in Facet 3 then have an AT state to act on (narrow envelope → held-pending →
transfer/retire).

### M3: two axes, earned reputation × conferred authority (fixes A3)
Do not translate AT reputation into KC authority. Carry both. AT reputation stays
the behavioral substrate, untouched. Add a **conferred-authority overlay** in
which KC roles arrive as signed credentials attached to AT identities: extend the
existing **ZTA credential** mechanism (`identity/zta/`), which is
X.509/attestation-shaped, to carry a KC role/scope. AT nodes then have `(earned
tier) × (conferred role)`. AT's Proof-of-Authority agreement can *optionally*
consult conferred KC authority instead of only topological rank, which is exactly
the "intentional structuring of the hierarchy" the reader found missing, added
without disturbing the emergent trust computation. The overlay's issuance and
verification are KC-hosted; AT need only carry an opaque credential in its ZTA
slot and offer the optional consult hook.

### M4: A standing projection in both directions, honoring the category change (fixes B1, E1)
Define the explicit, one-way-safe map between AT reputation and KC standing:
- **AT → KC is evidence, never verdict.** A sustained AT anomaly (the
  behaviour-governor's dwell state, `autonomous_trust.behaviour`, already
  human-on-the-loop with `auto_slash=False`) or a `SlashAttestation` produces a KC
  `ConductRecord`, specifically a `ConductKind::Concern`, the jester's
  *pre-indictment* flag: signed, attributable, CID'd, **carrying no ladder rung.**
  AT's Merkle-linked transaction history becomes machine-attested *exhibits* with
  provenance. AT never sets KC standing.
- **KC → AT is enforcement, and authoritative.** A KC `InterimInjunction` /
  `Designation` latches into AT as a capability gate / reputation floor, written by
  the community's KC authority credential. `has_capability(archive, did, cap)`
  becomes an input to AT negotiation-admission and gateway-forwarding for
  compact-facing capabilities. A KC anathema → AT exclusion; a KC re-entry → AT
  `REASON_REHABILITATE` readmit (fixes F2). Because the write is authoritative and
  signed, AT must verify the community authority credential before applying it,
  see open question Q3.

Both directions are implemented in `kith_covenant.integration`: the AT→KC producer
reads AT signals (imported) and appends KC records via the existing PyO3 authoring
surface; the KC→AT writer reads KC archive projections and calls AT enforcement
hooks (M4's KC→AT half is the missing part of the bridge).

### M5: Evolve the observer into a bidirectional adapter, KC-hosted (fixes E1, E2, E3)
Turn the one-way observer into a two-way translator that lives in KC and imports
AT:
1. **Stand up `kith_covenant.integration`** (Phase 0) importing `autonomous_trust`;
   move the observer's logic there. The event/policy schema is already in the KC
   core library, so no shim is needed to hold it.
2. **Widen the event vocabulary** in `interface.py`: add `Sponsorship`,
   `Concern`/`Conduct`, `DisputeProposed`, `InjunctionIssued`/`Lifted`, `ReEntry`,
   `AttestationResult`.
3. **Add the return path**: a KC→AT enforcement writer that consumes KC archive
   projections (`ReputationView`, `has_capability`, `RecognitionStanding`) and
   emits AT reputation/capability writes. This is the missing half of the bridge.

### M6: A dispute/enforcement state machine spanning both tempos (fixes C1, C2)
Define the lifecycle that connects AT reputation events to KC's escalation ladder
and back, as an **event-driven** pipeline (never a synchronous call), which is how
the tempo gap is actually bridged:

```
 machine tempo                    human / deliberative tempo                machine tempo
 ─────────────                    ──────────────────────────                ─────────────
 sustained AT anomaly   ──►  KC Concern (jester flag)
   or SlashAttestation   ──►  mediation panel  ──►  ladder transitions
                                              ──►  InterimInjunction ──► capability gate LATCHED into AT
                                              ──►  Designation/anathema ──► AT exclusion
                              restitution ──► verification ──► re-entry ceremony ──► AT REASON_REHABILITATE readmit
```

The bottom of the ladder is fed at machine tempo; the middle is governed at
deliberative tempo; the return latch re-enters machine tempo. Neither side blocks
on the other. The state machine is a KC-hosted coordinator in
`kith_covenant.integration`.

### M7: Optional KC recognition gate at AT admission (fixes F1)
Extend AT's `welcoming_committee` / `_zta_admit` with an *optional* KC gate: a node
requesting **compact-facing** capabilities must present a KC sponsorship +
recognition (its guardian's vouch + the community's standing). Purely
intra-community nodes keep the current emergent AT admission unchanged. The gate's
decision is a KC call (`has_capability` / `RecognitionStanding` over the archive)
invoked from an AT admission hook; the *policy* lives KC-side, the *hook* is a
minimal AT extension. This is backward-compatible and it honors
[machine-participation.md](machine-participation.md)'s own boundary: "internal
fleet operation is the community's own affair; compact-facing participation is
guardian-bound and fan-out-capped."

### M8: Attestation as a first-class seam event (fixes E4)
Make continuous attestation a real signal: an `AttestationResult` event from AT
(extending the ZTA credential path and the stubbed `_zkp`/hardware attestation)
whose *failure* emits a KC `Concern` and triggers the "earlier-acting revocation
profile" from Facet 3, a **narrowed AT operating envelope** (tighter capability
gating, fresh attestation required for non-routine actions) rather than a full
revocation, so false positives are recoverable. This wires
[machine-participation.md](machine-participation.md)'s Move 2/3 (weight integrity →
capability-gating) to a concrete AT signal, evaluated in `kith_covenant.integration`.

## The five concrete first couplings

These are the smallest realizations of M4 (KC→AT enforcement) and M7 (recognition
at admission): the earliest place the KC→AT return path earns its keep. Each maps
a KC never-target / reversibility / manufactured-event rule down into AT behavior.
Under the ownership inversion, the **policy fields and the compliance metric live
in the KC core library and the KC-hosted observer**, and the **enforcement stage
calls an AT hook** (a minimal exposed extension where AT lacks one) rather than
being patched into AT's internals directly.

**Layering strategy (applies to every feature).** Each feature lands in two stages
so nothing invasive ships unmeasured:

1. **Policy field + observer metric (KC-side, non-invasive).** Extend
   `KithCovenantPolicy` with an opt-in field (default `None`/`False` so
   `null_policy()` stays a no-op and current behavior is unchanged), and have the
   KC observer *measure* whether AT honored it (new counters in `report()`, new
   weight in `compute_fitness`). Testable in the in-process evaluator with no AT
   change.
2. **Enforcement hook (gated).** From the KC→AT writer, gate the corresponding AT
   behavior on the policy flag via an AT-exposed hook (negotiation admission,
   `repprocess.py` slash/exclusion). Invasive, so done second and always behind the
   flag.

The KC library already provides the base policy
(`min_reputation_to_negotiate`, `defection_cooldown_s`, `max_group_size`,
`bandwidth_priority_by_rank`) and the base events (`PeerAdmitted`,
`ReputationUpdate`, `NegotiationStarted`, `NegotiationOutcome`, `EvaluatorResult`);
the fields and events below are additions.

### Feature 4 analogue: never-withhold floor (easiest)
- **Policy:** add `courtesy_floor: frozenset[str] = frozenset()`, capability
  categories (e.g. `"safety"`, `"distress"`, `"relay"`) owed to any peer
  regardless of reputation/cooldown. `strict_policy()` sets
  `{"safety","distress","relay"}`; `null_policy()` leaves it empty.
- **Observer:** in `_on_negotiation_started`, if the task's capability category is
  in `courtesy_floor`, short-circuit the reputation/cooldown checks and count it
  compliant; record a *violation* only when a floor-category request was refused.
  Add `floor_honored` / `floor_denied` to `report()`.
- **Enforcement:** the AT negotiation-admission path that consults
  `min_reputation_to_negotiate` (and the multi-agency Negotiation rank-gate,
  `_bootstrap_gate` in `examples/multi_agency/coordinator.py`) must whitelist floor
  capabilities before applying the reputation cutoff. Mirrors KC's never-target
  categories one-to-one. Lowest risk: the observer metric alone is useful and needs
  no AT change.

### Features 1/3: restoration-path completeness (easy)
- **Policy:** add `restoration_required: bool = True` and
  `max_exclusion_s: float | None = None`. Together they forbid a permanent,
  unconditional exclusion: every exclusion must carry either a finite cooldown or a
  reputation-recovery path.
- **Observer:** track, per excluded/slashed peer, whether a re-entry condition
  exists (a `defection_cooldown_s` was set, or reputation can climb back above
  threshold). Emit a violation on a permanent ban (`defection_cooldown_s is None`
  and no recovery path).
- **Enforcement:** confirm the slash/exclusion path in `repprocess.py` always pairs
  exclusion with a re-entry condition, a slashed peer decays back toward
  `PREREP_NEUTRAL` and can re-earn standing (this already largely holds; the work
  is to make it an *invariant* with a test: no exclusion event without a matching
  re-entry condition). Matches KC's anathema-reversibility rule.

### Feature 3 thin analogue: pile-on damping (moderate)
- **Policy:** add `min_corroborators: int = 1` and `collapse_delay_s: float | None
  = None`.
- **Observer:** track anomaly/negative reports per subject and per reporter; flag a
  "pile-on" violation when a single uncorroborated report drives a reputation
  collapse below `min_reputation_to_negotiate`.
- **Enforcement:** gate the *application* of a collapse-inducing slash on either
  (a) ≥ `min_corroborators` independent reporters, or (b) a structured
  `collapse_delay_s` hold during which corroboration must arrive, else the slash is
  dropped. In the demos this maps to the coordinator's validator path
  (`submit_reading_for_validation` → `COMPROMISE_DETECT`): require corroboration
  from ≥N sources before the anomaly cascades into reputation. Most invasive
  (touches core reputation weighting), so ship the observer metric first and put the
  hook behind the flag. This is KC's manufactured-event-resistance
  ([capture_resistance.md](../background/capture_resistance.md) move 4, structured
  skepticism) pushed down into AT.

### Feature 5: appeal-above channel (moderate)
- **Policy:** add `require_human_confirmation_for_expulsion: bool = False`.
- **Interface:** add an `ExpulsionProposed` event (and its companion
  `FloorRequest`).
- **Observer:** track expulsions and whether each surfaced to a principal
  (`expulsions_surfaced` / `expulsions_autonomous` in `report()`).
- **Enforcement:** when the flag is set, an expulsion decision does not finalize
  autonomously, the peer enters a `pending-expulsion` hold and an
  `ExpulsionProposed` event is emitted to a human-principal channel; the exclusion
  commits only on confirmation (or, with no channel configured, after a timeout,
  preserving today's behavior when the flag is off). In the multi-agency/DoD demos
  the coordinator surfaces the proposal to the dashboard for operator confirmation
  (reusing the event-log / narration overlay path). Realizes "reputation consensus
  is never final for expulsion": this is M4's KC→AT enforcement in miniature.

### Feature 2: not addressed
Feature 2 of the source KC feature list is out of scope here, presumably either
already covered by AT or judged not easily addable; confirm against the source KC
feature list before implementing.

### Cross-cutting work for the five features
- Extend `to_dict`/`from_dict` for every new policy field;
  `test_from_dict_ignores_extra_keys` already guarantees old recordings/policies
  stay loadable.
- Add one observer metric per feature to `report()` and a corresponding weight in
  `compute_fitness` (rebalance `_DEFAULT_WEIGHTS` to keep the sum at 1.0).
- Every field defaults to a no-op, so `null_policy()` stays inert and existing
  behavior is unchanged until a policy opts in.
- Extend the three factories (`null`/`permissive`/`strict`) and their tests to
  cover the new fields; add observer unit tests mirroring the existing
  compliance/violation cases.

## What is already aligned (reuse, don't rebuild)

- **Ed25519 everywhere**: the shared cryptographic base; AT `Signature` and KC
  `Signed<T>` are the same primitive.
- **The KC policy/event schema already in the core library**: `policy.py` and
  `interface.py` exist in `src/python/kith_covenant/`; the runtime just moves to a
  KC integration package that imports AT.
- **The observer + [policy-evaluation.md](policy-evaluation.md)**: the AT→KC
  direction already exists; it needs relocating to KC, widening, and a return path,
  not replacement.
- **The five features**: never-withhold floor, restoration-path completeness,
  pile-on damping, appeal-above channel, are the concrete first couplings; the
  appeal-above channel *is* M4's KC→AT enforcement in miniature.
- **AT ZTA credentials**: the natural carrier for the M3 authority overlay.
- **The gateway-reputation-tree**: the natural community external face (M1) and a
  first answer to archive replication (G1).
- **The Operator + PIV/MFA**: the seed of the guardian (M2).
- **The behaviour-governor (human-on-the-loop, `auto_slash=False`)**: the natural
  producer of KC `Concern` records (M4).
- **AT Merkle transaction history**: machine-attested exhibits for KC proceedings.

## A phased path

- **Phase 0: the seam works end-to-end, KC-hosted.** Create the
  `kith_covenant.integration` package importing `autonomous_trust`; relocate the
  observer logic into it; widen `interface.py`; add the KC→AT enforcement writer as
  a no-op-by-default sink. Low risk; unblocks everything. (The policy/event schema
  already lives in the KC core library, so no shim is needed.)
- **Phase 1: identity.** `CommunityBinding` (M1) + the guardian reference on AT
  `Identity` (M2) + the authority overlay on ZTA credentials (M3).
- **Phase 2: coupling.** Standing projection both ways (M4) + the capability latch
  into AT admission/forwarding. Realize the appeal-above and restoration-path
  features here.
- **Phase 3: governance flow.** The dispute/enforcement state machine (M6),
  recognition-at-admission (M7), and attestation events (M8).

## Open questions (stated, not solved)

- **Q1: the reputation↔standing map.** What AT signal is strong enough to justify
  a KC `Concern`, and what deliberative threshold in KC is needed before a verdict
  latches back into AT? The category change (B1) forbids a numeric formula; it wants
  a designed policy.
- **Q2: fan-out / fleet seam.** [machine-participation.md](machine-participation.md)
  admits this is "asserted rather than designed": a community may run thousands of
  AT nodes behind its boundary, but compact-facing participation is fan-out-capped.
  Where exactly is the boundary drawn in AT terms (per capability? per gateway?),
  and what happens when internal fleet conduct produces external effects?
- **Q3: who signs the KC→AT enforcement, and how does AT trust it?** A KC
  injunction writing an AT capability gate must be verifiable by AT nodes. Does the
  community's KC authority credential chain to something AT already verifies (ZTA
  issuer)? What stops a captured community from writing self-serving enforcement
  into its own subtree?
- **Q4: attestation trust roots.** TPM/enclave/ZK each rest on assumptions
  ([machine-participation.md](machine-participation.md) Facet 1). Which does AT
  adopt, and how is the compact designed so no single root's failure dissolves
  binding across the federation?
- **Q5: archive over an AT mesh.** How do KC's append-only, content-addressed
  archive semantics ride AT's partition-recovery and gateway trees (G1)?

---

*This document supersedes the AT-repo notes `AT_KC_MELD.md` and `kc_support.md`.
Cross-references: [machine-participation.md](machine-participation.md),
[protocol-components.md](protocol-components.md),
[policy-evaluation.md](policy-evaluation.md), and background
[m2m_interaction.md](../background/m2m_interaction.md),
[capture_resistance.md](../background/capture_resistance.md),
[jester_and_dissent.md](../background/jester_and_dissent.md).*
