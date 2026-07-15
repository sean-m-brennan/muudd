# Phase 4: Kith-Covenant Protocol Hooks Design Specification

**Date:** 2026-03-18
**Status:** Approved
**Depends on:** Phase 2 (AT Integration Testing)

## Objective

Define the AT ↔ Kith-Covenant interface and build an evaluation framework for group protocol policies. This phase does not implement Kith-Covenant protocols. It creates the scaffolding so that protocol development can proceed with automated feedback.

## Base Paths

All paths in this spec are relative to these roots:

- **Kith-Covenant library:** `lib/muudd/lib/kith-covenant/`
- **AT simulator:** `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/`
- **Simulator config:** `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/config/`
- **Simulator tests:** `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/`

## Architecture

Split-package design with clean ownership boundaries:

- **`lib/kith-covenant/src/python/kith_covenant/`**: Stable interface. Event types, policy schema, evaluator protocol. Owned by the Kith-Covenant library. Target for both hand-crafted policies and future ML-generated policies.
- **`evaluation/kith_covenant/`**: Runtime. AT process plugin (observer), evaluation harness, Docker integration. Owned by the AT simulator.

The simulator imports from the kith-covenant library. The kith-covenant library has no dependency on AT internals.

```
lib/kith-covenant/src/python/kith_covenant/    evaluation/kith_covenant/
┌─────────────────────┐              ┌──────────────────────────────────┐
│ interface.py         │◄────imports──│ observer.py                      │
│   Event dataclasses  │              │   KithCovenantObserver(Process)  │
│   EvaluatorResult    │              │                                  │
│   EvaluatorProtocol  │              │ evaluator.py                     │
├─────────────────────┤              │   InProcessEvaluator             │
│ policy.py            │◄────imports──│   DockerEvaluator                │
│   KithCovenantPolicy │              │                                  │
│   Null/Permissive/   │              │ instrumented.py                  │
│   Strict factories   │              │   KithCovenantInstrumentedAT     │
└─────────────────────┘              └──────────────────────────────────┘
```

Import style: `from kith_covenant.interface import PeerAdmitted, ...` and `from kith_covenant.policy import KithCovenantPolicy`.

## Event Model

The observer translates raw AT `Message` objects into typed events. These are the stable interface between AT and Kith-Covenant: policies reason about events, never raw messages.

Defined in `lib/kith-covenant/src/python/kith_covenant/interface.py` as frozen dataclasses:

| Event | Source `msg.function` | Fields |
|-------|----------------------|--------|
| `PeerAdmitted` | `access_granted` | `peer_id: str`, `timestamp: datetime` |
| `ReputationUpdate` | `reputation response` | `peer_id: str`, `score: float`, `timestamp: datetime` |
| `NegotiationStarted` | `invitation` | `task_id: str`, `initiator_id: str`, `timestamp: datetime` |
| `NegotiationOutcome` | `ack` / `nack` | `task_id: str`, `success: bool`, `timestamp: datetime` |

**Ignored message functions:** `request_access`, `peer_accepted` (internal identity protocol steps; only the final `access_granted` matters for policy), `haggle` (intermediate negotiation counter-offer; only the final `ack`/`nack` outcome matters for policy evaluation).

**Extracting fields from AT messages:**
- `peer_id`: From `msg.from_whom` (inbound) or `msg.to_whom[0]` (outbound), following the MetricsCollector pattern.
- `task_id`: From `msg.obj.uuid` if `isinstance(msg.obj, TaskInfo)`, else `str(msg.obj)`.
- `initiator_id`: From `msg.from_whom` for inbound `invitation` messages. For outbound invitations (this node is the initiator), use this node's own identity.

**Group channel metadata:** The system-simulation.md spec lists "group channel metadata" as an observer input. AT does not currently expose discrete group channel join/leave events. For Phase 4, group size is approximated by counting `PeerAdmitted` events. A `GroupMembershipChanged` event can be added in a future phase when AT gains explicit channel management.

Events carry only what a policy needs. Raw AT internals (message objects, queue references) stay inside the observer.

## KithCovenantPolicy Data Class

Defined in `lib/kith-covenant/src/python/kith_covenant/policy.py`. A frozen dataclass with numeric thresholds.

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `min_reputation_to_negotiate` | `float \| None` | Peers below this score are ineligible for negotiation. `None` = no restriction. |
| `defection_cooldown_s` | `float \| None` | After a peer drops below threshold, cooldown before re-eligibility. `None` = immediate. |
| `max_group_size` | `int \| None` | Maximum peers for consensus operations. `None` = unlimited. |
| `bandwidth_priority_by_rank` | `bool` | Higher-reputation peers get priority in bandwidth allocation metrics. |

### Shipped Policies

1. **`NullPolicy`**: All fields `None`/`False`. No rules. Baseline for comparison.
2. **`PermissivePolicy`**: `min_reputation=0.2`, `cooldown=30s`, `max_group=50`, priority off.
3. **`StrictPolicy`**: `min_reputation=0.5`, `cooldown=120s`, `max_group=10`, priority on.

### Serialization

`to_dict()` / `from_dict()` for JSON round-tripping. The evaluator stores the policy alongside results.

### ML Integration Point

The future Transformer generates a dict of these same fields. `KithCovenantPolicy.from_dict(model_output)` closes the loop. No adapter needed.

## KithCovenantObserver Process

`evaluation/kith_covenant/observer.py`. Registered via `add_worker()`, receives messages via `_TeeQueue`. Passive: reads the message stream but never injects messages.

```python
class KithCovenantObserver(Process, metaclass=ProcMeta,
                     proc_name='kith-covenant-observer',
                     description='Kith-Covenant policy observation'):
    is_tee_observer = True  # Discovered by InstrumentedAT for _TeeQueue wiring

    def __init__(self, configurations, subsystems, log_queue,
                 dependencies=None, policy=None, output_path=None):
        super().__init__(configurations, subsystems, log_queue,
                         dependencies=dependencies)
        self.policy = policy    # KithCovenantPolicy instance
        self.output_path = output_path
        self._init_state()
```

`MetricsCollector` also gains `is_tee_observer = True` (minor addition to existing class).

### Core Loop

1. Receive `Message` from tee'd queue.
2. Parse into a typed event. Skip messages that don't map to any event type.
3. Evaluate event against the active `KithCovenantPolicy`:
   - **`NegotiationStarted`**: Check if initiator's last-known reputation meets `min_reputation_to_negotiate` and isn't in cooldown. Record as compliant or violation. Store `task_id → initiator_id` mapping for outcome correlation.
   - **`ReputationUpdate`**: Update internal peer reputation map. Track if peer has crossed below threshold (triggers cooldown).
   - **`PeerAdmitted`**: Track group size against `max_group_size`.
   - **`NegotiationOutcome`**: Look up initiator via `pending_negotiations[task_id]`. Track success/failure rates, correlate with policy compliance.
4. Periodically snapshot results to JSON (30s interval, matching MetricsCollector).

### Internal State

| Field | Purpose |
|-------|---------|
| `peer_reputations: dict[str, float]` | Latest reputation score per peer |
| `cooldown_until: dict[str, datetime]` | Peers in defection cooldown |
| `pending_negotiations: dict[str, str]` | `task_id → initiator_id` for outcome correlation |
| `policy_violations: int` | Count of events that violated policy |
| `policy_compliant: int` | Count of events that met policy |
| `negotiation_outcomes: list[bool]` | Success/failure of each negotiation |
| `group_size_max: int` | Peak observed group size |

### Output Report

```python
{
    "policy": {...},                   # Active policy as dict
    "cooperation_rate": float,         # successful negotiations / total
    "task_throughput": float,          # completed negotiations / duration_s
    "policy_compliance_rate": float,   # compliant / (compliant + violations)
    "violation_count": int,
    "group_size_peak": int,
    "peer_count": int,
}
```

## _TeeQueue Fan-Out

The current `_TeeQueue` in `instrumented.py` supports only a single observer queue. Phase 4 requires two observers (MetricsCollector + KithCovenantObserver) receiving the full message stream.

**Solution:** Generalize `_TeeQueue` to accept a list of observer queues.

```python
class _TeeQueue:
    def __init__(self, real_queue, observer_queues):
        self._real = real_queue
        # Accept single queue (backward compat) or list
        if isinstance(observer_queues, list):
            self._observers = observer_queues
        else:
            self._observers = [observer_queues]

    def put(self, obj, block=True, timeout=None):
        self._real.put(obj, block=block, timeout=timeout)
        for obs in self._observers:
            try:
                obs.put_nowait(obj)
            except Exception:
                pass

    def put_nowait(self, obj):
        self._real.put_nowait(obj)
        for obs in self._observers:
            try:
                obs.put_nowait(obj)
            except Exception:
                pass

    def __getstate__(self):
        return {'real': self._real, 'observers': self._observers}

    def __setstate__(self, state):
        self._real = state['real']
        self._observers = state['observers']
```

The existing `get()`, `get_nowait()`, `empty()`, `qsize()` delegate to `self._real` and need no changes.

**Observer identification:** Observer processes declare themselves via a class attribute `is_tee_observer = True`. Both `MetricsCollector` and `KithCovenantObserver` set this. The `InstrumentedAT.run_forever()` wiring discovers observers by checking this attribute on instantiated worker processes, collects their queue references, and passes the list to `_TeeQueue`. This avoids conflating all `add_worker()` registrations with observers (e.g., `InspectorProcess` uses `add_worker()` but is not a passive observer).

This change is backward-compatible: existing code passing a single observer queue still works via the `isinstance` guard.

## KithCovenantInstrumentedAT

`evaluation/kith_covenant/instrumented.py`. Subclasses `InstrumentedAT` to register the KithCovenantObserver as an additional worker alongside MetricsCollector.

```python
class KithCovenantInstrumentedAT(InstrumentedAT):
    _kith_covenant_policy = None
    _kith_covenant_output = None

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        if self._kith_covenant_policy:
            self.add_worker(KithCovenantObserver,
                            policy=self._kith_covenant_policy,
                            output_path=self._kith_covenant_output)
```

No `run_forever()` override needed. The generalized `_TeeQueue` in the parent class handles fan-out to both observers.

## KithCovenantEvaluator

`evaluation/kith_covenant/evaluator.py`. Orchestrates the policy evaluation loop.

### Evaluator Protocol

Defined in `lib/kith-covenant/src/python/kith_covenant/interface.py`:

```python
@dataclass(frozen=True)
class EvaluatorResult:
    policy: KithCovenantPolicy
    observer_report: dict
    metrics_report: dict
    fitness_score: float

class KithCovenantEvaluatorProtocol(Protocol):
    def evaluate(self, policy: KithCovenantPolicy) -> EvaluatorResult: ...
```

### Fitness Function

Weighted sum of normalized metrics. Each metric is normalized to [0, 1] before weighting.

| Metric | Weight | Normalization | Source |
|--------|--------|---------------|--------|
| Cooperation rate | 0.30 | Already [0, 1] (ratio) | observer |
| Task throughput | 0.10 | `min(throughput / max_throughput, 1.0)`, `max_throughput` default 1.0 neg/s | observer |
| Policy compliance rate | 0.25 | Already [0, 1] (ratio) | observer |
| Identity convergence | 0.15 | `1 - min(convergence_s / max_convergence_s, 1.0)`, `max_convergence_s` default 120s | metrics collector |
| Reputation stability | 0.15 | `1 - min(stddev / max_stddev, 1.0)`, `max_stddev` default 0.5 | metrics collector |
| Bandwidth overhead | 0.05 | `1 - min(fraction / max_fraction, 1.0)`, `max_fraction` default 1.0 | metrics collector |

Weights and normalization bounds are constructor parameters, overridable. `None` metric values score 0.

### Backend A: InProcessEvaluator

- Instantiates `KithCovenantInstrumentedAT` directly.
- Uses mock/simulated peers (existing `mock.py` pattern).
- Runs for a configurable duration, collects reports.
- Fast: seconds per evaluation.

### Backend B: DockerEvaluator

- Extends `test-simulation-scenarios.sh` with a `--kith-covenant-policy <path>` flag.
- Launches full Docker simulation with one instrumented node running `KithCovenantObserver`.
- Reads observer + metrics JSON from bind-mounted output directory.
- Slow: minutes per evaluation, but tests real multi-node behavior.

Both backends return `EvaluatorResult`, so comparison across backends is straightforward.

## File Layout

### New Files: Kith-Covenant Library (`lib/kith-covenant/`)

| File | Contents |
|------|----------|
| `python/kith_covenant/__init__.py` | Package init |
| `python/kith_covenant/interface.py` | Event dataclasses, `EvaluatorResult`, `KithCovenantEvaluatorProtocol` |
| `python/kith_covenant/policy.py` | `KithCovenantPolicy`, factory functions, serialization |

### New Files: AT Simulator (`simulator/`)

| File | Contents |
|------|----------|
| `evaluation/kith_covenant/__init__.py` | Package init |
| `evaluation/kith_covenant/observer.py` | `KithCovenantObserver(Process)` with `proc_name='kith-covenant-observer'` |
| `evaluation/kith_covenant/evaluator.py` | `KithCovenantEvaluator`, `InProcessEvaluator`, `DockerEvaluator` |
| `evaluation/kith_covenant/instrumented.py` | `KithCovenantInstrumentedAT(InstrumentedAT)` |

### Modified Files

| File | Change |
|------|--------|
| `simulator/instrumented.py` | Generalize `_TeeQueue` to accept list of observer queues; discover observer processes dynamically in `run_forever()` |
| `config/test-simulation-scenarios.sh` | Add `--kith-covenant-policy <path>` flag; add `lib/kith-covenant/src/python` to PYTHONPATH |

### Tests (`tests/`)

| File | Scope |
|------|-------|
| `a_unit/test_kith_covenant_policy.py` | Policy dataclass, serialization, factory functions |
| `a_unit/test_kith_covenant_observer.py` | Observer event parsing, policy evaluation, report generation |
| `a_unit/test_tee_queue_fanout.py` | `_TeeQueue` with multiple observers |
| `b_integration/test_kith_covenant_evaluator.py` | InProcessEvaluator end-to-end: null vs strict policy produces different scores |

## PYTHONPATH

`lib/kith-covenant/src/python` added to `PYTHONPATH` in `test-simulation-scenarios.sh`:

```bash
KITH_COVENANT_SRC="$(cd "$AT_ROOT/../kith-covenant/src/python" && pwd)"
export PYTHONPATH="${KITH_COVENANT_SRC}:${AT_SRC}/autonomous-trust:${AT_SRC}/autonomous-trust-services:..."
```

`$AT_ROOT` resolves to `.../lib/muudd/lib/autonomous_trust`, so `$AT_ROOT/../kith-covenant/src/python` resolves to `.../lib/muudd/lib/kith-covenant/src/python/`. Docker images already mount all source directories.

## Verification Criteria

1. `KithCovenantObserver` receives events during AT simulation (unit test with mock messages).
2. Hand-crafted strict policy produces measurably different evaluator scores vs null policy.
3. `KithCovenantEvaluator` produces reproducible fitness scores across runs (within tolerance).
4. ML integration point documented: `KithCovenantPolicy.from_dict()` accepts the same schema the Transformer will output.
5. `_TeeQueue` fan-out works with both one and two observers (unit test).

## The Automated Loop (Future)

```
1. Define KithCovenantPolicy (hand-crafted → ML-generated)
2. Run AT simulation with policy active (via KithCovenantObserver)
3. Collect outcome metrics
4. Score policy via fitness function
5. (Future) Feed scores to ML → generate new policy
   └──→ repeat
```

Step 5 is future work. This spec delivers steps 1-4.
