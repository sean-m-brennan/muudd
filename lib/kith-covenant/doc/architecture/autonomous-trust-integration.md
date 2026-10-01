*Previous: [The captured prosecution](captured-prosecution.md)*

# AutonomousTrust integration: melding AT and kith-covenant, hosted in KC

*This document enumerates the joints between the two models and specifies how they
meld. It supersedes the AT-repo notes `AT_KC_MELD.md` (disconnects and meld
proposals) and `kc_support.md` (the five concrete first couplings), and it inverts
their ownership, so that the meld is **hosted in kith-covenant (KC), which imports the
`autonomous_trust` (AT) package**, rather than in AT. Companion to
[Machine participation](machine-participation.md),
[Protocol components](protocol-components.md), and
[Policy evaluation architecture](policy-evaluation.md).*

## Purpose

AutonomousTrust (AT) and kith-covenant (KC) are designed to stack. AT carries
anonymous, machine-to-machine reputation *inside* a single community, and KC carries
known-identity governance *between* communities, among a mix of humans, machines,
and guardians. The framing KC uses states the seam plainly. "AT exposes upward
community-key identity, delegated speakers, health indicators, and witness
availability. Kith-covenant never reaches inward"
([Protocol components](protocol-components.md), decision 5).

The intended layering is heavily *documented*, and a handful of the joints are
conceptually unresolved rather than merely unimplemented. The
two models make different assumptions about what an actor is, what "trust" means,
who confers authority, and how a decision is reached. This document enumerates
those disconnects against the code on both sides, then lays out how the
two meld, with KC as the integration host.

The shorthand a reader reaches for, that KC is human and machine and structured
while AT is machine-to-machine with no intentional hierarchy, is close but worth
sharpening in one place. **AT is not flat.** It has ranks, tiers, valuations,
groups, and gateway trees, and it can even run a Proof-of-Authority agreement
mode. The real distinction is that
structure in AT is *earned and emergent* (computed from behavior and topology,
shifting continuously), while structure in KC is *conferred and covenantal* (set by
signed acts of designation that persist until revoked). That difference is the
spine of most of the disconnects below.

## Ownership: the integration lives in KC and imports AT

The original notes assumed the seam would be built *into* AT, with an AT-side
`kith_covenant` shim, observer, and enforcement all living in the AT tree, and with
imports resolving from within AT. **That is inverted here.** The runtime
integration is a KC concern. KC imports the `autonomous_trust` package and hosts
the observer, the bidirectional adapter, and the enforcement writer. AT is a
dependency of the integration layer in KC, not the other way around.

This is a refinement of the split already recorded in
[Policy evaluation architecture](policy-evaluation.md), where the KC *library* owns the
policy schema and event types and the AT simulator imported them (AT→KC). The
schema stays exactly where it is, and what moves is the **runtime**, so the
dependency for the live integration now runs KC→AT. Concretely, the Python package
of KC splits into two layers with a one-way dependency between them:

| Layer | Location | Owns | Depends on |
|---|---|---|---|
| **`kith_covenant` core** | `src/python/kith_covenant/` | event types (`interface.py`), policy schema (`policy.py`), evaluator protocol, the Rust-backed protocol surface (`_core` via PyO3) | nothing AT-specific |
| **`kith_covenant.integration`** (new) | `src/python/kith_covenant/integration/` | the observer, the bidirectional AT↔KC adapter, the KC→AT enforcement writer, the five-feature enforcement hooks | `kith_covenant` core **and** `autonomous_trust` |

The core library stays AT-free, so the policy schema remains stable and reusable
(hand-crafted and future ML-generated policies still target the same types, see
[Policy evaluation architecture](policy-evaluation.md) on why that loop does not violate the
non-emergence claim). Everything that must touch AT lives in
`kith_covenant.integration`, which `import autonomous_trust`. AT already exists
and is imported. If a joint genuinely requires a new AT-side field or hook
(noted per-proposal below), that is a *minimal, exposed* extension in AT, and the
decision logic still lives in KC.

The material the bridge is welded from is one primitive. **Both are Ed25519.** AT
`Signature` and KC `Signed<T>` are the same thing.

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

**A1. AT has no "community" object for KC to bind to.**
The only first-class actor in KC is the community (a root `did:key`,
`kc-identity/src/community.rs`). AT has no community type. Its nearest analogs
(a `Group` (a shared-symmetric-key membership set, `identity/group.py`) and a
gateway subtree) are *emergent topological clusters*, not named, root-anchored,
covenant-bearing entities with a membership roll and a standing. So when KC says
"AT exposes upward community-key identity," there is today nothing in AT that
*produces* a community-key identity, since AT identities are strictly per-node. KC
has an anchor point, and AT has nothing to hook onto it.

**A2. The identity namespaces don't meet.**
Although both are Ed25519 underneath, AT identity is `uuid4` + key + Zooko
nickname/petname (plus optional X.509/PIV ZTA credential), while KC identity is a
`did:key` plus a *delegation graph*. AT has no DID, no delegation records, and,
most of all, no notion of one key *speaking for a principal it is not*. The whole attribution
model of KC (sign with a delegate key, attribute the record to the
community principal, resolve through `SpeakerDelegation`) has no counterpart in
AT, where a signature simply *is* the node.

**A3. Earned/emergent hierarchy vs conferred/covenantal authority.**
AT can tell you who is *reachable* (`rank`, `effective_rank`), who is *trusted by
track record* (`tier`, `valuation`), and can even run Proof-of-Authority voting
(`algorithms/authority.py`). But rank, tier, and valuation are computed from behavior or
topology and drift continuously, and none is *conferred by an act of designation*
that persists as a status. The authority KC confers is exactly the opposite. The
roles of root, envoy, mediator, witness, and sponsor are conferred by signed
delegation/registry records and hold until revoked, regardless of behavior. There
is no AT surface onto which the `DelegationScope`, `MediatorRegistry` and
`WitnessRegistry` of KC can land, since AT has no representation of "appointed to
speak/mediate/witness."

**A4. Community-as-party has no AT representative.**
KC treats a *community* as the party to a dispute, sponsorship, or injunction. For
that to ride AT, some AT construct must present a whole group/subtree as a single
addressable principal holding one standing and able to sign KC records. The AT
gateway-reputation-tree (`doc/architecture/gateway-reputation-tree.md`) already
aggregates subtree reputation and can answer for its roster (the natural
external face of a community) but it is a reputation aggregator, not a principal
that can hold a KC standing or sign on behalf of the community.

### B. Trust vs standing

**B1. Two different meanings of the word.**
AT "trust" is a probabilistic, continuous, decaying behavioral estimate. KC
"trust" is adjudicated relational *standing*, a discrete status set by ceremony
and judgment. While AT reputation collapses and recovers automatically from behavior, KC
standing changes only through recognized acts (admission, sponsorship,
designation, and re-entry). Nothing today maps one to the other, and the map is not a
rescaling. It is a **category change from measurement to adjudication.** Wiring
them together naively (e.g. "reputation < X ⇒ Anathema") would be a category
error and exactly the kind of automatic loyalty-cascade KC is built to prevent.

### C. Decision-making and tempo

**C1. Automatic vs deliberative.**
AT reaches collective decisions by leaderless Byzantine Multi-Paxos and quorum
co-signing, no argument, no appeal, machine tempo. KC reaches them by mediation
panels (≥3 non-party mediators), escalation-ladder transitions, deliberation
cycles separated by a 7-day waiting period, and quorums of *distinct
communities*. AT has no dispute intake, no panel, no argument record, no appeal
channel, no waiting period. An AT reputation event that *should* become a KC
dispute has no path to become one, and a KC decision has no path back into the AT
automatic machinery.

**C2. Tempo mismatch.**
While AT runs at milliseconds, KC runs at days-to-generations (7-day deliberation
gaps, 90-day injunctions, and generational re-affirmation). Any meld must bridge tempo
*without either side blocking on the other*, so machine-tempo detection must
**propose** into human-tempo adjudication, and human-tempo verdicts must **latch**
into machine-tempo enforcement, an event-driven coupling, never a synchronous
call.

### D. The human / guardian axis (the heart of "human+machine" vs "m2m only")

**D1. The whole answer KC gives to machine participation has no AT anchor.**
[Machine participation](machine-participation.md) is unambiguous that a machine
participates only in a *derived* form, "irrevocably tied to one specific named
human" (the **guardian**) whose community standing is at stake in the machine's
conduct. The binding rests on four layered mechanisms (constitutive objective,
cryptographic self-binding, continuous attestation, principal-referential
identity + audit commons). Yet:
- KC has **no guardian type in code**, the guardianship relation, the fan-out
  cap, attestation, and weight-integrity are all doc-only
  ([Machine participation](machine-participation.md) Facets 1-3), not types.
- AT has **no place to record the guardian of a node.** The AT Operator
  (`operator/activate.py`, "the single human touchpoint") is a *requesting* human
  (a request-only node that consumes capabilities), not a *guardian who owns the
  standing of a machine*. AT `Identity` carries no principal/guardian field.

So the load-bearing relation of the entire machine layer (node → guardian →
community standing) is asserted in prose and absent from the types on both sides.
This is precisely where the two do not quite meet up. AT is machine-to-machine
with a thin request-only human touchpoint, and KC needs a thick human-as-guardian
who *is* the node's substrate.

### E. The enforcement coupling: one-way, thin, and on the wrong side

**E1. The existing bridge only observes, and only AT→KC.**
The current link (`autonomous_trust/evaluation/kith_covenant/observer.py`,
`is_tee_observer = True`) tees AT protocol messages, maps them to four KC-flavored
events, and scores them against a `KithCovenantPolicy`. It **measures**
compliance. It never **enforces**, and it flows strictly AT→KC. There is no return
path by which a KC `InterimInjunction` sets an AT capability gate, or a KC
`Designation` (anathema) floors an AT reputation. The "teeth" of KC (`has_capability()`)
terminate in the KC archive itself, not in the negotiation-admission of AT or
gateway-forwarding. Under the ownership inversion, the *logic* of this observer becomes
a KC-hosted module (`kith_covenant.integration`) that imports AT to tee its bus,
rather than an AT-tree module that imports KC.

**E2. The event vocabulary is AT-shaped and thin.**
The seam (`src/python/kith_covenant/interface.py` and `policy.py` in KC) models only
`PeerAdmitted`, `ReputationUpdate`, `NegotiationStarted`, `NegotiationOutcome`, and
a policy of numeric thresholds (`min_reputation_to_negotiate`,
`defection_cooldown_s`, `max_group_size`, `bandwidth_priority_by_rank`). It has
*no* event for anything KC actually governs, with no community admission, sponsorship,
conduct/concern, dispute, injunction, re-entry, or attestation. So even the built
seam can only lift the intra-community reputation of AT up to KC as a fitness metric, and it
cannot carry the inter-community governance acts of KC down to AT.

**E3. The runtime seam isn't stood up on the KC side yet.**
The event and policy modules already live in the KC core library
(`src/python/kith_covenant/{interface,policy}.py`), so KC does not need a shim to
*hold the schema*. What is missing is the KC-side **integration package** that
imports `autonomous_trust`, hosts the observer, and adds the return path. The
existing AT-tree observer imports KC top-level `kith_covenant.*` and its
`test_kith_covenant_*` suites skip when the package does not resolve from within
AT, and moving the runtime to KC and importing AT removes that fragility (Phase 0
below).

**E4. No attestation anywhere on the seam.**
The machine binding in KC leans on *continuous* attestation (TPM/enclave/ZK,
"cryptographically integral policy, attested at every inference") feeding
capability-gating. While AT has ZTA credentials at *admission* and a stubbed `_zkp`
layer, nothing continuously attests policy integrity, and no event carries an
attestation failure into a KC standing/capability change.

### F. Membership lifecycle

**F1, admission. Welcoming-committee vs recognition ceremony.**
AT admission is machine-automatic, running broadcast `request_access` → border-guard
`welcoming_committee()` → ZTA gate (`_zta_admit`) → quorum vote → two-phase
provisional→confirmed → group-key handover (`identity/idprocess.py`). No vouching
human, no oath, no substrate check. KC admission is a recognition ceremony, with an
intra-community rite (five `Movement`s, witnessed, root-sealed) + an
inter-community anti-spiral feature exchange + *sponsorship* (a named sponsor
stakes standing). AT has no sponsor, no rite, and no anti-spiral check, and its
"membership" is key-in-group, not covenant standing. There is no hook in the AT
admission path to require or record a KC recognition/sponsorship before admitting
a compact-facing node.

**F2, exclusion and restoration. Aligned in spirit, unwired in fact.**
This is the *most* aligned pair. Both believe in reversible exclusion + a
restoration path. AT re-admits via `REASON_REHABILITATE`, and KC runs restitution
obligation → verification → re-entry ceremony, with anathema reversible and
*hostis humani generis* not. Although the five features below mirror the rules of
KC one-to-one (never-withhold floor, restoration-path completeness, appeal-above channel),
nothing yet connects an AT rehabilitation to a KC re-entry, or forbids AT from
permanently excluding a compact-facing node with no KC restoration path. The
alignment is latent, not realized.

### G. The physical seam

**G1. KC has no transport; AT is the substrate.**
The KC archive is `InMemoryArchive`, single-host, append-only, and JSON dump/load.
libp2p replication is deployment work (the [`TODO.md`](../../TODO.md) of KC). The
content-addressing (`Cid`, canonical CBOR,
`Signed<T>`) is transport-ready, but the transport is meant to be supplied from
below, with muudd for comms and AT for identity, health, and witness
availability. How KC
archive entries replicate *over* an AT mesh (which nodes hold the archive, how a
new community bootstraps a view, how partition recovery interacts with
append-only archive semantics) is entirely unspecified. The AT
partition-recovery and gateway-tree machinery are the obvious substrate, but no
design connects them to archive replication.

## How they meld

The guiding principle is stated once. **Do not collapse the two models into one.** Keep the
earned, emergent, machine-tempo reputation of AT exactly as it is, and keep the
conferred, deliberative, human-tempo governance of KC exactly as it is. Meld them at a
**seam of translation** owned by `kith_covenant.integration`, with two orthogonal
axes carried side by side and an event-driven, bidirectional coupling between
them. Every proposal below builds on something that already exists.

### M1: A community-binding object (fixes A1, A4, G1)
Introduce a `CommunityBinding` that maps an AT `Group` / gateway-subtree to a KC
community DID, held by the gateway/root node of the group. The gateway becomes the
external face of that community. It holds the KC root (or an `Envoy`-scoped
delegate) and presents the subtree to KC as one `CommunityIdentity`. Reuse the
**gateway-reputation-tree** as the external reputation surface, and
let archive entries replicate along the gateway tree (a first answer to G1). This
gives the "upward-exposed community-key identity" of KC a real producer. The binding
record and its resolution live in `kith_covenant.integration`. The only AT-side
need is a stable handle to the gateway node, which AT already exposes.

### M2: A guardian directory built on the Operator (fixes D1)
Promote the AT Operator from *requester* to *guardian*. The Operator is already the
authenticated human touchpoint (PIV + MFA, `operator/activate.py`). Let an
operator credential **vouch for and own the standing of** one or more machine
nodes, with the fan-out cap and standing-weight decay from
[Machine participation](machine-participation.md) Facet 2. This needs one
minimal AT-side field (an optional `guardian` reference on AT `Identity` (a
KC-facing human key/DID)) exposed for the integration layer to read. The
directory, the fan-out cap, and the weight-decay policy live KC-side. This is the
concrete realization of "machine tied to one named human". AT records the guardian
edge, and `SpeakerDelegation` in KC attributes those records to the community
*through* that guardian. The guardian-fall / guardian-capture / succession cases
in Facet 3 then have an AT state to act on (narrow envelope → held-pending →
transfer/retire).

### M3: two axes, earned reputation × conferred authority (fixes A3)
Do not translate AT reputation into KC authority. Carry both. AT reputation stays
the behavioral substrate, untouched. Add a **conferred-authority overlay** in
which KC roles arrive as signed credentials attached to AT identities. Extend the
existing **ZTA credential** mechanism (`identity/zta/`), which is
X.509/attestation-shaped, to carry a KC role/scope. AT nodes then have `(earned
tier) × (conferred role)`. AT's Proof-of-Authority agreement can *optionally*
consult conferred KC authority instead of only topological rank, which is exactly
the "intentional structuring of the hierarchy" the reader found missing, added
without disturbing the emergent trust computation. Issuance and verification of
the overlay are KC-hosted, and AT need only carry an opaque credential in its ZTA
slot and offer the optional consult hook.

### M4: A standing projection in both directions, honoring the category change (fixes B1, E1)
Define the explicit, one-way-safe map between AT reputation and KC standing:
- **AT → KC is evidence, never verdict.** A sustained AT anomaly (the
  behaviour-governor's dwell state, `autonomous_trust.behaviour`, already
  human-on-the-loop with `auto_slash=False`) or a `SlashAttestation` produces a KC
  `ConductRecord`, specifically a `ConductKind::Concern`, the jester's
  *pre-indictment* flag: signed, attributable, CID'd, **carrying no ladder rung.**
  The Merkle-linked transaction history of AT becomes machine-attested *exhibits* with
  provenance. AT never sets KC standing.
- **KC → AT is enforcement, and authoritative.** A KC `InterimInjunction` /
  `Designation` latches into AT as a capability gate / reputation floor, written by
  the KC authority credential of the community. `has_capability(archive, did, cap)`
  becomes an input to AT negotiation-admission and gateway-forwarding for
  compact-facing capabilities. A KC anathema → AT exclusion; a KC re-entry → AT
  `REASON_REHABILITATE` readmit (fixes F2). Because the write is authoritative and
  signed, AT must verify the community authority credential before applying it,
  see open question Q3.

Both directions are implemented in `kith_covenant.integration`. The AT→KC producer
reads AT signals (imported) and appends KC records via the existing PyO3 authoring
surface, and the KC→AT writer reads KC archive projections and calls AT enforcement
hooks (the KC→AT half of M4 is the missing part of the bridge).

### M5: Evolve the observer into a bidirectional adapter, KC-hosted (fixes E1, E2, E3)
Turn the one-way observer into a two-way translator that lives in KC and imports
AT:
1. **Stand up `kith_covenant.integration`** (Phase 0) importing `autonomous_trust`,
   and move the observer logic there. The event/policy schema is already in the KC
   core library, so no shim is needed to hold it.
2. **Widen the event vocabulary** in `interface.py`: add `Sponsorship`,
   `Concern`/`Conduct`, `DisputeProposed`, `InjunctionIssued`/`Lifted`, `ReEntry`,
   `AttestationResult`.
3. **Add the return path**: a KC→AT enforcement writer that consumes KC archive
   projections (`ReputationView`, `has_capability`, `RecognitionStanding`) and
   emits AT reputation/capability writes. This is the missing half of the bridge.

### M6: A dispute/enforcement state machine spanning both tempos (fixes C1, C2)
Define the lifecycle that connects AT reputation events to the escalation ladder of KC
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

The bottom of the ladder is fed at machine tempo, the middle is governed at
deliberative tempo, and the return latch re-enters machine tempo. Neither side blocks
on the other. The state machine is a KC-hosted coordinator in
`kith_covenant.integration`.

### M7: Optional KC recognition gate at AT admission (fixes F1)
Extend `welcoming_committee` / `_zta_admit` in AT with an *optional* KC gate: a node
requesting **compact-facing** capabilities must present a KC sponsorship +
recognition (a vouch from its guardian plus the standing of its community). Purely
intra-community nodes keep the current emergent AT admission unchanged. The gate
decision is a KC call (`has_capability` / `RecognitionStanding` over the archive)
invoked from an AT admission hook, so the *policy* lives KC-side while the *hook* is a
minimal AT extension. This is backward-compatible and it honors the
boundary drawn in [Machine participation](machine-participation.md): "internal
fleet operation is the community's own affair; compact-facing participation is
guardian-bound and fan-out-capped."

### M8: Attestation as a first-class seam event (fixes E4)
Make continuous attestation a real signal, an `AttestationResult` event from AT
(extending the ZTA credential path and the stubbed `_zkp`/hardware attestation)
whose *failure* emits a KC `Concern` and triggers the "earlier-acting revocation
profile" from Facet 3, a **narrowed AT operating envelope** (tighter capability
gating, fresh attestation required for non-routine actions) rather than a full
revocation, so false positives are recoverable. This wires
Move 2/3 of [Machine participation](machine-participation.md) (weight integrity →
capability-gating) to a concrete AT signal, evaluated in `kith_covenant.integration`.

## The five concrete first couplings

These are the smallest realizations of M4 (KC→AT enforcement) and M7 (recognition
at admission), the earliest place the KC→AT return path earns its keep. Each maps
a KC never-target / reversibility / manufactured-event rule down into AT behavior.
Under the ownership inversion, the **policy fields and the compliance metric live
in the KC core library and the KC-hosted observer**, and the **enforcement stage
calls an AT hook** (a minimal exposed extension where AT lacks one) rather than
being patched into the internals of AT directly.

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
   `repprocess.py` slash/exclusion). That is invasive, so it is done second and always behind the
   flag.

The KC library already provides the base policy
(`min_reputation_to_negotiate`, `defection_cooldown_s`, `max_group_size`,
`bandwidth_priority_by_rank`) and the base events (`PeerAdmitted`,
`ReputationUpdate`, `NegotiationStarted`, `NegotiationOutcome`, `EvaluatorResult`);
the fields and events below are additions.

### Feature 4 analog: never-withhold floor (easiest)
- **Policy.** Add `courtesy_floor: frozenset[str] = frozenset()`, capability
  categories (e.g. `"safety"`, `"distress"`, `"relay"`) owed to any peer
  regardless of reputation/cooldown. `strict_policy()` sets
  `{"safety","distress","relay"}`; `null_policy()` leaves it empty.
- **Observer.** In `_on_negotiation_started`, if the task's capability category is
  in `courtesy_floor`, short-circuit the reputation/cooldown checks and count it
  compliant; record a *violation* only when a floor-category request was refused.
  Add `floor_honored` / `floor_denied` to `report()`.
- **Enforcement.** The AT negotiation-admission path that consults
  `min_reputation_to_negotiate` (and the multi-agency Negotiation rank-gate,
  `_bootstrap_gate` in `examples/multi_agency/coordinator.py`) must whitelist floor
  capabilities before applying the reputation cutoff. Mirrors the never-target rule
  of KC, category for category. Risk is lowest here, since the observer metric alone
  is useful and needs no AT change.

### Features 1/3: restoration-path completeness (easy)
- **Policy.** Add `restoration_required: bool = True` and
  `max_exclusion_s: float | None = None`. Together they forbid a permanent,
  unconditional exclusion: every exclusion must carry either a finite cooldown or a
  reputation-recovery path.
- **Observer.** Track, per excluded/slashed peer, whether a re-entry condition
  exists (a `defection_cooldown_s` was set, or reputation can climb back above
  threshold). Emit a violation on a permanent ban (`defection_cooldown_s is None`
  and no recovery path).
- **Enforcement.** Confirm the slash/exclusion path in `repprocess.py` always pairs
  exclusion with a re-entry condition, a slashed peer decays back toward
  `PREREP_NEUTRAL` and can re-earn standing (this already largely holds, and the work
  is to make it an *invariant* with a test that forbids an exclusion event without a
  matching re-entry condition). Matches the anathema-reversibility rule of KC.

### Feature 3 thin analog: pile-on damping (moderate)
- **Policy.** Add `min_corroborators: int = 1` and `collapse_delay_s: float | None
  = None`.
- **Observer.** Track anomaly/negative reports per subject and per reporter; flag a
  "pile-on" violation when a single uncorroborated report drives a reputation
  collapse below `min_reputation_to_negotiate`.
- **Enforcement.** Gate the *application* of a collapse-inducing slash on either
  (a) ≥ `min_corroborators` independent reporters, or (b) a structured
  `collapse_delay_s` hold during which corroboration must arrive, else the slash is
  dropped. In the demos this maps to the coordinator's validator path
  (`submit_reading_for_validation` → `COMPROMISE_DETECT`): require corroboration
  from ≥N sources before the anomaly cascades into reputation. Most invasive
  (touches core reputation weighting), so ship the observer metric first and put the
  hook behind the flag. This is the manufactured-event resistance of KC
  ([Capture resistance](../background/capture_resistance.md) move 4, structured
  skepticism) pushed down into AT.

### Feature 5: appeal-above channel (moderate)
- **Policy.** Add `require_human_confirmation_for_expulsion: bool = False`.
- **Interface.** Add an `ExpulsionProposed` event (and its companion
  `FloorRequest`).
- **Observer.** Track expulsions and whether each surfaced to a principal
  (`expulsions_surfaced` / `expulsions_autonomous` in `report()`).
- **Enforcement.** When the flag is set, an expulsion decision does not finalize
  autonomously, the peer enters a `pending-expulsion` hold and an
  `ExpulsionProposed` event is emitted to a human-principal channel; the exclusion
  commits only on confirmation (or, with no channel configured, after a timeout,
  preserving today's behavior when the flag is off). In the multi-agency/DoD demos
  the coordinator surfaces the proposal to the dashboard for operator confirmation
  (reusing the event-log / narration overlay path). Realizes "reputation consensus
  is never final for expulsion", the KC→AT enforcement of M4 in miniature.

### Feature 2: not addressed
Feature 2 of the source KC feature list is out of scope here, presumably either
already covered by AT or judged not easily addable; confirm against the source KC
feature list before implementing.

### Cross-cutting work for the five features
- Extend `to_dict`/`from_dict` for every new policy field.
  `test_from_dict_ignores_extra_keys` already guarantees old recordings/policies
  stay loadable.
- Add one observer metric per feature to `report()` and a corresponding weight in
  `compute_fitness` (rebalance `_DEFAULT_WEIGHTS` to keep the sum at 1.0).
- Every field defaults to a no-op, so `null_policy()` stays inert and existing
  behavior is unchanged until a policy opts in.
- Extend the three factories (`null`/`permissive`/`strict`) and their tests to
  cover the new fields, and add observer unit tests mirroring the existing
  compliance/violation cases.

## What is already aligned (reuse, don't rebuild)

- **Ed25519 everywhere.** The shared cryptographic base. AT `Signature` and KC
  `Signed<T>` are the same primitive.
- **The KC policy/event schema already in the core library.** `policy.py` and
  `interface.py` exist in `src/python/kith_covenant/`. The runtime just moves to a
  KC integration package that imports AT.
- **The observer + [Policy evaluation architecture](policy-evaluation.md).** The AT→KC
  direction already exists. It needs relocating to KC, widening, and a return path,
  not replacement.
- **The five features.** The never-withhold floor, restoration-path completeness,
  pile-on damping, and appeal-above channel are the concrete first couplings, and the
  appeal-above channel *is* that same KC→AT enforcement in miniature.
- **AT ZTA credentials:** the natural carrier for the M3 authority overlay.
- **The gateway-reputation-tree:** the natural community external face (M1)
  and a first answer to archive replication (G1).
- **The Operator + PIV/MFA.** The seed of the guardian (M2).
- **The behaviour-governor (human-on-the-loop, `auto_slash=False`):** the
  natural producer of KC `Concern` records (M4).
- **AT Merkle transaction history.** Machine-attested exhibits for KC proceedings.

## A phased path

- **Phase 0. The seam works end-to-end, KC-hosted.** Create the
  `kith_covenant.integration` package importing `autonomous_trust`, relocate the
  observer logic into it, widen `interface.py`, and add the KC→AT enforcement writer
  as a no-op-by-default sink. Low risk, and it unblocks everything. (The policy/event schema
  already lives in the KC core library, so no shim is needed.)
- **Phase 1. Identity.** `CommunityBinding` (M1) + the guardian reference on AT
  `Identity` (M2) + the authority overlay on ZTA credentials (M3).
- **Phase 2. Coupling.** Standing projection both ways (M4) + the capability latch
  into AT admission/forwarding. Realize the appeal-above and restoration-path
  features here.
- **Phase 3. Governance flow.** The dispute/enforcement state machine (M6),
  recognition-at-admission (M7), and attestation events (M8).

## Open questions (stated, not solved)

- **Q1. The reputation↔standing map.** What AT signal is strong enough to justify
  a KC `Concern`, and what deliberative threshold in KC is needed before a verdict
  latches back into AT? The category change (B1) forbids a numeric formula. It wants
  a designed policy.
- **Q2. Fan-out / fleet seam.** [Machine participation](machine-participation.md)
  admits this is "asserted rather than designed": a community may run thousands of
  AT nodes behind its boundary, but compact-facing participation is fan-out-capped.
  Where exactly is the boundary drawn in AT terms (per capability? per gateway?),
  and what happens when internal fleet conduct produces external effects?
- **Q3. Who signs the KC→AT enforcement, and how does AT trust it?** A KC
  injunction writing an AT capability gate must be verifiable by AT nodes. Does the
  KC authority credential of the community chain to something AT already verifies
  (ZTA issuer)? What stops a captured community from writing self-serving enforcement
  into its own subtree?
- **Q4. Attestation trust roots.** TPM/enclave/ZK each rest on assumptions
  ([Machine participation](machine-participation.md) Facet 1). Which does AT
  adopt, and how is the compact designed so the failure of a single root does not
  dissolve binding across the federation?
- **Q5. Archive over an AT mesh.** How do the append-only, content-addressed
  archive semantics of KC ride the partition-recovery and gateway trees of AT (G1)?

---

*This document supersedes the AT-repo notes `AT_KC_MELD.md` and `kc_support.md`.
Cross-references: [Machine participation](machine-participation.md),
[Protocol components](protocol-components.md),
[Policy evaluation architecture](policy-evaluation.md), and background
[Machine-to-machine interaction](../background/m2m_interaction.md),
[Capture resistance](../background/capture_resistance.md),
[Jester and dissent](../background/jester_and_dissent.md).*

---

*Next: [Build and toolchain](build-and-toolchain.md)*
