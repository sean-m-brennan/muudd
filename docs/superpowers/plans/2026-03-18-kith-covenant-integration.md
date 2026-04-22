# Kith-Covenant Protocol Hooks Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the AT ↔ Kith-Covenant interface and evaluation framework so group protocol policies can be tested with automated feedback.

**Architecture:** Split-package: `lib/kith-covenant/src/python/kith_covenant/` owns stable types (events, policy, evaluator protocol); `evaluation/kith_covenant/` owns runtime (observer process, evaluator backends). The observer follows the MetricsCollector `_TeeQueue` pattern, generalized to support multiple observers.

**Tech Stack:** Python 3.10+, AT Process plugin system (`ProcMeta`/`add_worker()`), `_TeeQueue` for message observation, pytest.

**Spec:** `docs/superpowers/specs/2026-03-18-kith-covenant-integration-design.md`

**Path abbreviations used below:**
- `KITH_COVENANT` = `lib/muudd/lib/kith-covenant`
- `SIM` = `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator`
- `AT_SIM` = `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator`
- `TESTS` = `lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests`

**Test runner command pattern:**
```bash
cd lib/muudd/lib/autonomous_trust && \
PYTHONPATH="src/autonomous-trust:src/autonomous-trust-services:src/autonomous-trust-inspector:src/autonomous-trust-simulator:../../kith-covenant/src/python" \
python -m pytest <test-path> -v
```

---

## File Map

### New Files

| File | Responsibility |
|------|----------------|
| `KITH_COVENANT/python/kith_covenant/__init__.py` | Package init; re-exports public API (updated in Task 2) |
| `KITH_COVENANT/python/kith_covenant/interface.py` | Event frozen dataclasses, `EvaluatorResult`, `KithCovenantEvaluatorProtocol` |
| `KITH_COVENANT/python/kith_covenant/policy.py` | `KithCovenantPolicy` frozen dataclass, `NULL/PERMISSIVE/STRICT` factories, `to_dict()`/`from_dict()` |
| `SIM/evaluation/kith_covenant/__init__.py` | Package init |
| `SIM/evaluation/kith_covenant/observer.py` | `KithCovenantObserver(Process)` — tee-queue consumer, event instantiation, policy eval, report |
| `SIM/evaluation/kith_covenant/instrumented.py` | `KithCovenantInstrumentedAT(InstrumentedAT)` — wires observer + metrics |
| `SIM/evaluation/kith_covenant/evaluator.py` | `compute_fitness()`, `InProcessEvaluator`, `DockerEvaluator` |
| `TESTS/a_unit/test_kith_covenant_interface.py` | Event dataclass tests |
| `TESTS/a_unit/test_kith_covenant_policy.py` | Policy tests |
| `TESTS/a_unit/test_kith_covenant_observer.py` | Observer event parsing + policy eval tests |
| `TESTS/a_unit/test_tee_queue_fanout.py` | `_TeeQueue` multi-observer tests |
| `TESTS/b_integration/test_kith_covenant_evaluator.py` | Fitness scoring + InProcessEvaluator end-to-end |

### Modified Files

| File | Change |
|------|--------|
| `SIM/instrumented.py` | Generalize `_TeeQueue` to list of observers; dynamic observer discovery in `run_forever()` |
| `SIM/metrics/collector.py` | Add `is_tee_observer = True` class attribute |
| `AT_SIM/config/test-simulation-scenarios.sh` | Add `--kith-covenant-policy` flag; add kith-covenant to PYTHONPATH |

---

### Task 1: Kith-Covenant Library — Event Types and Evaluator Protocol

**Files:**
- Create: `KITH_COVENANT/python/kith_covenant/__init__.py`
- Create: `KITH_COVENANT/python/kith_covenant/interface.py`
- Test: `TESTS/a_unit/test_kith_covenant_interface.py`

- [ ] **Step 1: Create package directories**

```bash
mkdir -p lib/muudd/lib/kith-covenant/src/python/kith_covenant
```

- [ ] **Step 2: Write the failing test**

Create `TESTS/a_unit/test_kith_covenant_interface.py`:

```python
"""Tests for kith_covenant event dataclasses and evaluator protocol."""

from datetime import datetime

import pytest

try:
    from kith_covenant.interface import (
        PeerAdmitted, ReputationUpdate, NegotiationStarted,
        NegotiationOutcome, EvaluatorResult, KithCovenantEvaluatorProtocol,
    )
except ImportError:
    pytest.skip("kith_covenant package not on PYTHONPATH", allow_module_level=True)


class TestEventDataclasses:

    def test_peer_admitted_is_frozen(self):
        e = PeerAdmitted(peer_id='p1', timestamp=datetime(2026, 1, 1))
        with pytest.raises(AttributeError):
            e.peer_id = 'p2'

    def test_reputation_update_fields(self):
        e = ReputationUpdate(peer_id='p1', score=0.85, timestamp=datetime(2026, 1, 1))
        assert e.peer_id == 'p1'
        assert e.score == 0.85

    def test_negotiation_started_fields(self):
        e = NegotiationStarted(task_id='t1', initiator_id='p1',
                               timestamp=datetime(2026, 1, 1))
        assert e.task_id == 't1'
        assert e.initiator_id == 'p1'

    def test_negotiation_outcome_fields(self):
        e = NegotiationOutcome(task_id='t1', success=True,
                               timestamp=datetime(2026, 1, 1))
        assert e.success is True


class TestEvaluatorResult:

    def test_evaluator_result_is_frozen(self):
        # Use a sentinel for policy — KithCovenantPolicy is created in Task 2
        r = EvaluatorResult(
            policy=None, observer_report={},
            metrics_report={}, fitness_score=0.5)
        with pytest.raises(AttributeError):
            r.fitness_score = 1.0
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd lib/muudd/lib/autonomous_trust && PYTHONPATH="src/autonomous-trust:src/autonomous-trust-services:src/autonomous-trust-inspector:src/autonomous-trust-simulator:../../kith-covenant/src/python" python -m pytest src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_interface.py -v`
Expected: FAIL / skip (module not found)

- [ ] **Step 4: Implement interface.py**

Create `KITH_COVENANT/python/kith_covenant/__init__.py`:
```python
"""Kith-Covenant protocol interface library."""
```

Create `KITH_COVENANT/python/kith_covenant/interface.py`:

```python
"""Kith-Covenant event types and evaluator protocol.

These frozen dataclasses are the stable interface between the AT
simulator and Kith-Covenant policy evaluation.  Policies reason about events,
never raw AT Message objects.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from typing import TYPE_CHECKING, Protocol, runtime_checkable

if TYPE_CHECKING:
    from kith_covenant.policy import KithCovenantPolicy


@dataclass(frozen=True)
class PeerAdmitted:
    peer_id: str
    timestamp: datetime


@dataclass(frozen=True)
class ReputationUpdate:
    peer_id: str
    score: float
    timestamp: datetime


@dataclass(frozen=True)
class NegotiationStarted:
    task_id: str
    initiator_id: str
    timestamp: datetime


@dataclass(frozen=True)
class NegotiationOutcome:
    task_id: str
    success: bool
    timestamp: datetime


@dataclass(frozen=True)
class EvaluatorResult:
    policy: KithCovenantPolicy
    observer_report: dict
    metrics_report: dict
    fitness_score: float


@runtime_checkable
class KithCovenantEvaluatorProtocol(Protocol):
    def evaluate(self, policy: KithCovenantPolicy) -> EvaluatorResult: ...
```

- [ ] **Step 5: Run test to verify it passes**

Run: same command as step 3.
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add lib/muudd/lib/kith-covenant/src/python/ lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_interface.py
git commit -m "feat: add kith-covenant event dataclasses and evaluator protocol"
```

---

### Task 2: Kith-Covenant Library — KithCovenantPolicy Data Class

**Files:**
- Create: `KITH_COVENANT/python/kith_covenant/policy.py`
- Test: `TESTS/a_unit/test_kith_covenant_policy.py`

- [ ] **Step 1: Write the failing test**

Create `TESTS/a_unit/test_kith_covenant_policy.py`:

```python
"""Tests for KithCovenantPolicy dataclass, factories, and serialization."""

import pytest

try:
    from kith_covenant.policy import (
        KithCovenantPolicy, null_policy, permissive_policy, strict_policy,
    )
except ImportError:
    pytest.skip("kith_covenant package not on PYTHONPATH", allow_module_level=True)


class TestPolicyDefaults:

    def test_default_is_null(self):
        p = KithCovenantPolicy()
        assert p.min_reputation_to_negotiate is None
        assert p.defection_cooldown_s is None
        assert p.max_group_size is None
        assert p.bandwidth_priority_by_rank is False


class TestFactories:

    def test_null_policy(self):
        p = null_policy()
        assert p.min_reputation_to_negotiate is None

    def test_permissive_policy(self):
        p = permissive_policy()
        assert p.min_reputation_to_negotiate == 0.2
        assert p.defection_cooldown_s == 30.0
        assert p.max_group_size == 50
        assert p.bandwidth_priority_by_rank is False

    def test_strict_policy(self):
        p = strict_policy()
        assert p.min_reputation_to_negotiate == 0.5
        assert p.defection_cooldown_s == 120.0
        assert p.max_group_size == 10
        assert p.bandwidth_priority_by_rank is True


class TestSerialization:

    def test_round_trip(self):
        p = strict_policy()
        d = p.to_dict()
        p2 = KithCovenantPolicy.from_dict(d)
        assert p == p2

    def test_to_dict_keys(self):
        d = null_policy().to_dict()
        assert set(d.keys()) == {
            'min_reputation_to_negotiate', 'defection_cooldown_s',
            'max_group_size', 'bandwidth_priority_by_rank',
        }

    def test_from_dict_ignores_extra_keys(self):
        d = null_policy().to_dict()
        d['future_field'] = 42
        p = KithCovenantPolicy.from_dict(d)
        assert p == null_policy()

    def test_frozen(self):
        p = strict_policy()
        with pytest.raises(AttributeError):
            p.min_reputation_to_negotiate = 0.1
```

- [ ] **Step 2: Run test to verify it fails**

Run: same PYTHONPATH pattern.
Expected: FAIL (module not found)

- [ ] **Step 3: Implement policy.py**

Create `KITH_COVENANT/python/kith_covenant/policy.py`:

```python
"""KithCovenantPolicy data class and factory functions.

Defines group behavioral rules that the KithCovenantObserver checks
against incoming AT protocol events.
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, fields
from typing import Optional


@dataclass(frozen=True)
class KithCovenantPolicy:
    min_reputation_to_negotiate: Optional[float] = None
    defection_cooldown_s: Optional[float] = None
    max_group_size: Optional[int] = None
    bandwidth_priority_by_rank: bool = False

    def to_dict(self) -> dict:
        return asdict(self)

    @classmethod
    def from_dict(cls, d: dict) -> KithCovenantPolicy:
        valid = {f.name for f in fields(cls)}
        return cls(**{k: v for k, v in d.items() if k in valid})


def null_policy() -> KithCovenantPolicy:
    return KithCovenantPolicy()


def permissive_policy() -> KithCovenantPolicy:
    return KithCovenantPolicy(
        min_reputation_to_negotiate=0.2,
        defection_cooldown_s=30.0,
        max_group_size=50,
        bandwidth_priority_by_rank=False,
    )


def strict_policy() -> KithCovenantPolicy:
    return KithCovenantPolicy(
        min_reputation_to_negotiate=0.5,
        defection_cooldown_s=120.0,
        max_group_size=10,
        bandwidth_priority_by_rank=True,
    )
```

- [ ] **Step 4: Update __init__.py with re-exports**

Update `KITH_COVENANT/python/kith_covenant/__init__.py`:
```python
"""Kith-Covenant protocol interface library."""

from kith_covenant.interface import (
    PeerAdmitted, ReputationUpdate, NegotiationStarted,
    NegotiationOutcome, EvaluatorResult, KithCovenantEvaluatorProtocol,
)
from kith_covenant.policy import (
    KithCovenantPolicy, null_policy, permissive_policy, strict_policy,
)

__all__ = [
    'PeerAdmitted', 'ReputationUpdate', 'NegotiationStarted',
    'NegotiationOutcome', 'EvaluatorResult', 'KithCovenantEvaluatorProtocol',
    'KithCovenantPolicy', 'null_policy', 'permissive_policy', 'strict_policy',
]
```

- [ ] **Step 5: Run tests to verify they pass**

Run: same PYTHONPATH pattern, both test files.
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add lib/muudd/lib/kith-covenant/src/python/kith_covenant/ lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_policy.py
git commit -m "feat: add KithCovenantPolicy dataclass with factory functions and serialization"
```

---

### Task 3: Generalize _TeeQueue for Multi-Observer Fan-Out

**Files:**
- Modify: `SIM/instrumented.py` (lines 44-82 for `_TeeQueue`, lines 113-119 for wiring)
- Modify: `SIM/metrics/collector.py` (add class attribute)
- Test: `TESTS/a_unit/test_tee_queue_fanout.py`

- [ ] **Step 1: Write the failing test**

Create `TESTS/a_unit/test_tee_queue_fanout.py`:

```python
"""Tests for _TeeQueue multi-observer fan-out."""

import multiprocessing
import pytest

from autonomous_trust.simulator.instrumented import _TeeQueue


class TestTeeQueueSingleObserver:
    """Backward compatibility: single observer queue still works."""

    def test_put_copies_to_single_observer(self):
        real_q = multiprocessing.Queue()
        obs_q = multiprocessing.Queue()
        tee = _TeeQueue(real_q, obs_q)
        tee.put('hello')
        assert real_q.get_nowait() == 'hello'
        assert obs_q.get_nowait() == 'hello'

    def test_put_nowait_copies_to_single_observer(self):
        real_q = multiprocessing.Queue()
        obs_q = multiprocessing.Queue()
        tee = _TeeQueue(real_q, obs_q)
        tee.put_nowait('hello')
        assert real_q.get_nowait() == 'hello'
        assert obs_q.get_nowait() == 'hello'


class TestTeeQueueMultipleObservers:
    """Fan-out to multiple observer queues."""

    def test_put_copies_to_all_observers(self):
        real_q = multiprocessing.Queue()
        obs1 = multiprocessing.Queue()
        obs2 = multiprocessing.Queue()
        tee = _TeeQueue(real_q, [obs1, obs2])
        tee.put('hello')
        assert real_q.get_nowait() == 'hello'
        assert obs1.get_nowait() == 'hello'
        assert obs2.get_nowait() == 'hello'

    def test_put_nowait_copies_to_all_observers(self):
        real_q = multiprocessing.Queue()
        obs1 = multiprocessing.Queue()
        obs2 = multiprocessing.Queue()
        tee = _TeeQueue(real_q, [obs1, obs2])
        tee.put_nowait('hello')
        assert real_q.get_nowait() == 'hello'
        assert obs1.get_nowait() == 'hello'
        assert obs2.get_nowait() == 'hello'

    def test_observer_failure_does_not_block(self):
        """If one observer is full, others still receive."""
        real_q = multiprocessing.Queue()
        obs1 = multiprocessing.Queue(maxsize=1)
        obs2 = multiprocessing.Queue()
        obs1.put('filler')  # fill obs1
        tee = _TeeQueue(real_q, [obs1, obs2])
        tee.put('hello')
        assert real_q.get_nowait() == 'hello'
        assert obs2.get_nowait() == 'hello'
        # obs1 still has 'filler', 'hello' was dropped silently

    def test_get_reads_from_real_queue(self):
        real_q = multiprocessing.Queue()
        obs = multiprocessing.Queue()
        tee = _TeeQueue(real_q, [obs])
        real_q.put('direct')
        assert tee.get_nowait() == 'direct'
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd lib/muudd/lib/autonomous_trust && PYTHONPATH="src/autonomous-trust:src/autonomous-trust-services:src/autonomous-trust-inspector:src/autonomous-trust-simulator" python -m pytest src/autonomous-trust-simulator/tests/a_unit/test_tee_queue_fanout.py -v`
Expected: FAIL (single observer constructor doesn't accept list)

- [ ] **Step 3: Update _TeeQueue in instrumented.py**

In `SIM/instrumented.py`, replace `_TeeQueue` class (lines 44-82) with:

```python
class _TeeQueue:
    """Queue wrapper that copies put() calls to one or more observer queues."""

    def __init__(self, real_queue, observer_queues):
        self._real = real_queue
        if isinstance(observer_queues, list):
            self._observers = observer_queues
        else:
            self._observers = [observer_queues]

    def __getstate__(self):
        return {'_real': self._real, '_observers': self._observers}

    def __setstate__(self, state):
        self._real = state['_real']
        self._observers = state['_observers']

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

    def get(self, block=True, timeout=None):
        return self._real.get(block=block, timeout=timeout)

    def get_nowait(self):
        return self._real.get_nowait()

    def empty(self):
        return self._real.empty()

    def qsize(self):
        return self._real.qsize()
```

- [ ] **Step 4: Update observer discovery in run_forever()**

In `SIM/instrumented.py`, replace the hardcoded collector wiring (lines 113-119) with dynamic observer discovery:

```python
        # Wrap all queues (except observers') with TeeQueue
        observer_names = [
            p.name for p in procs
            if getattr(p, 'is_tee_observer', False)
        ]
        if observer_names:
            observer_qs = [queues[n] for n in observer_names]
            for name in list(queues):
                if name not in observer_names:
                    queues[name] = _TeeQueue(queues[name], observer_qs)
```

- [ ] **Step 5: Add is_tee_observer to MetricsCollector**

In `SIM/metrics/collector.py`, add class attribute after the class declaration (line 43):

```python
class MetricsCollector(Process, metaclass=ProcMeta,
                       proc_name='metrics-collector',
                       description='Protocol metrics collection'):
    is_tee_observer = True
```

- [ ] **Step 6: Run all tests**

Run: existing MetricsCollector tests + new fan-out tests.
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/instrumented.py \
        lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/metrics/collector.py \
        lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/a_unit/test_tee_queue_fanout.py
git commit -m "feat: generalize _TeeQueue for multi-observer fan-out"
```

---

### Task 4: KithCovenantObserver Process

**Files:**
- Create: `SIM/evaluation/kith_covenant/__init__.py`
- Create: `SIM/evaluation/kith_covenant/observer.py`
- Test: `TESTS/a_unit/test_kith_covenant_observer.py`

- [ ] **Step 1: Create package directory**

```bash
mkdir -p lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/evaluation/kith_covenant
```

- [ ] **Step 2: Write the failing test**

Create `TESTS/a_unit/test_kith_covenant_observer.py`:

```python
"""Tests for KithCovenantObserver event parsing and policy evaluation."""

from datetime import datetime, timedelta

import pytest

try:
    from kith_covenant.policy import null_policy, strict_policy
except ImportError:
    pytest.skip("kith_covenant package not on PYTHONPATH", allow_module_level=True)

from autonomous_trust.simulator.evaluation.kith_covenant.observer import KithCovenantObserver


class TestEventParsing:
    """Observer correctly parses AT message functions into events."""

    @pytest.fixture
    def observer(self):
        obs = KithCovenantObserver.__new__(KithCovenantObserver)
        obs.policy = null_policy()
        obs.output_path = None
        obs._init_state()
        return obs

    def test_handles_access_granted(self, observer):
        observer._handle_message('access_granted', peer_id='peer-a',
                                 timestamp=datetime(2026, 1, 1))
        assert observer._peer_count == 1

    def test_handles_reputation_response(self, observer):
        observer._handle_message('reputation_update', peer_id='peer-a',
                                 score=0.75, timestamp=datetime(2026, 1, 1))
        assert observer._peer_reputations['peer-a'] == 0.75

    def test_handles_invitation(self, observer):
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-a',
                                 timestamp=datetime(2026, 1, 1))
        assert 't1' in observer._pending_negotiations

    def test_handles_ack(self, observer):
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-a',
                                 timestamp=datetime(2026, 1, 1))
        observer._handle_message('negotiation_outcome', task_id='t1',
                                 success=True, timestamp=datetime(2026, 1, 1))
        assert observer._negotiation_outcomes == [True]

    def test_ignores_unknown_event(self, observer):
        """Messages that don't map to events are silently skipped."""
        observer._handle_message('unknown_function',
                                 timestamp=datetime(2026, 1, 1))
        assert observer._policy_violations == 0
        assert observer._policy_compliant == 0


class TestPolicyEvaluation:
    """Observer evaluates events against the active policy."""

    @pytest.fixture
    def observer(self):
        obs = KithCovenantObserver.__new__(KithCovenantObserver)
        obs.policy = strict_policy()  # min_reputation=0.5
        obs.output_path = None
        obs._init_state()
        return obs

    def test_negotiation_below_threshold_is_violation(self, observer):
        now = datetime(2026, 1, 1)
        observer._peer_reputations['peer-a'] = 0.3  # below 0.5
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-a', timestamp=now)
        assert observer._policy_violations == 1

    def test_negotiation_above_threshold_is_compliant(self, observer):
        now = datetime(2026, 1, 1)
        observer._peer_reputations['peer-a'] = 0.8  # above 0.5
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-a', timestamp=now)
        assert observer._policy_compliant == 1

    def test_negotiation_unknown_peer_is_violation(self, observer):
        """Peer with no reputation data violates min_reputation policy."""
        now = datetime(2026, 1, 1)
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-unknown', timestamp=now)
        assert observer._policy_violations == 1

    def test_cooldown_blocks_negotiation(self, observer):
        now = datetime(2026, 1, 1)
        observer._peer_reputations['peer-a'] = 0.8
        # Place peer-a in cooldown for 120s
        observer._cooldown_until['peer-a'] = now + timedelta(seconds=120)
        observer._handle_message('negotiation_started', task_id='t1',
                                 initiator_id='peer-a', timestamp=now)
        assert observer._policy_violations == 1

    def test_group_size_violation(self, observer):
        """Admitting peer beyond max_group_size is a violation."""
        now = datetime(2026, 1, 1)
        # Strict policy: max_group_size=10
        observer._peer_count = 10
        observer._handle_message('access_granted', peer_id='peer-11',
                                 timestamp=now)
        assert observer._policy_violations == 1


class TestReport:

    def test_report_structure(self):
        obs = KithCovenantObserver.__new__(KithCovenantObserver)
        obs.policy = null_policy()
        obs.output_path = None
        obs._init_state()
        obs._start_time = datetime(2026, 1, 1)
        obs._negotiation_outcomes = [True, True, False]
        obs._policy_compliant = 5
        obs._policy_violations = 2
        obs._peer_count = 3
        obs._group_size_max = 3

        report = obs.report()
        assert report['cooperation_rate'] == pytest.approx(2 / 3)
        assert report['policy_compliance_rate'] == pytest.approx(5 / 7)
        assert report['violation_count'] == 2
        assert report['group_size_peak'] == 3
        assert report['peer_count'] == 3
        assert 'policy' in report
        assert 'task_throughput' in report
```

- [ ] **Step 3: Run test to verify it fails**

Expected: FAIL (module not found)

- [ ] **Step 4: Implement observer.py**

Create `SIM/evaluation/kith_covenant/__init__.py`:
```python
"""Kith-Covenant protocol evaluation for AT simulator."""
```

Create `SIM/evaluation/kith_covenant/observer.py`:

```python
"""KithCovenantObserver: AT Process that evaluates protocol events against a KithCovenantPolicy.

Passive observer — reads the message stream via _TeeQueue but never
injects messages.  Follows the MetricsCollector pattern.
"""

import json
import queue
from datetime import datetime, timedelta
from typing import Any, Optional

from autonomous_trust.core import ProcMeta
from autonomous_trust.core._python.processes import Process, ProcessTracker
from autonomous_trust.core._python.network.message import Message
from autonomous_trust.core._python.negotiation.negotiation import TaskInfo
from autonomous_trust.core._python.reputation.reputation import Reputation
from autonomous_trust.core.system import CfgIds, QueueType

from kith_covenant.interface import (
    PeerAdmitted, ReputationUpdate, NegotiationStarted, NegotiationOutcome,
)
from kith_covenant.policy import KithCovenantPolicy


class KithCovenantObserver(Process, metaclass=ProcMeta,
                     proc_name='kith-covenant-observer',
                     description='Kith-Covenant policy observation'):
    is_tee_observer = True

    def __init__(self, configurations: dict[str, Any],
                 subsystems: ProcessTracker,
                 log_queue: QueueType,
                 dependencies: list[str] = None,
                 policy: Optional[KithCovenantPolicy] = None,
                 output_path: Optional[str] = None):
        super().__init__(configurations, subsystems, log_queue,
                         dependencies=dependencies)
        self.policy = policy or KithCovenantPolicy()
        self.output_path = output_path
        self._init_state()

    def _init_state(self):
        self._peer_reputations: dict[str, float] = {}
        self._cooldown_until: dict[str, datetime] = {}
        self._pending_negotiations: dict[str, str] = {}
        self._policy_violations: int = 0
        self._policy_compliant: int = 0
        self._negotiation_outcomes: list[bool] = []
        self._peer_count: int = 0
        self._group_size_max: int = 0
        self._start_time: Optional[datetime] = None

    def _handle_event(self, event):
        """Evaluate a typed kith-covenant event against the active policy."""
        if isinstance(event, PeerAdmitted):
            self._on_peer_admitted(event)
        elif isinstance(event, ReputationUpdate):
            self._on_reputation_update(event)
        elif isinstance(event, NegotiationStarted):
            self._on_negotiation_started(event)
        elif isinstance(event, NegotiationOutcome):
            self._on_negotiation_outcome(event)

    # Also accept string-based dispatch for direct unit testing
    def _handle_message(self, event_type: str, **kwargs):
        """Create a typed event from kwargs and dispatch it."""
        event = None
        if event_type == 'access_granted':
            event = PeerAdmitted(peer_id=kwargs['peer_id'],
                                 timestamp=kwargs['timestamp'])
        elif event_type == 'reputation_update':
            event = ReputationUpdate(peer_id=kwargs['peer_id'],
                                     score=kwargs['score'],
                                     timestamp=kwargs['timestamp'])
        elif event_type == 'negotiation_started':
            event = NegotiationStarted(task_id=kwargs['task_id'],
                                       initiator_id=kwargs['initiator_id'],
                                       timestamp=kwargs['timestamp'])
        elif event_type == 'negotiation_outcome':
            event = NegotiationOutcome(task_id=kwargs['task_id'],
                                       success=kwargs['success'],
                                       timestamp=kwargs['timestamp'])
        if event is not None:
            self._handle_event(event)

    def _on_peer_admitted(self, event: PeerAdmitted):
        self._peer_count += 1
        if self._peer_count > self._group_size_max:
            self._group_size_max = self._peer_count
        if (self.policy.max_group_size is not None
                and self._peer_count > self.policy.max_group_size):
            self._policy_violations += 1
        else:
            self._policy_compliant += 1

    def _on_reputation_update(self, event: ReputationUpdate):
        old_score = self._peer_reputations.get(event.peer_id)
        self._peer_reputations[event.peer_id] = event.score
        if (self.policy.min_reputation_to_negotiate is not None
                and old_score is not None
                and old_score >= self.policy.min_reputation_to_negotiate
                and event.score < self.policy.min_reputation_to_negotiate
                and self.policy.defection_cooldown_s is not None):
            self._cooldown_until[event.peer_id] = (
                event.timestamp + timedelta(
                    seconds=self.policy.defection_cooldown_s))

    def _on_negotiation_started(self, event: NegotiationStarted):
        self._pending_negotiations[event.task_id] = event.initiator_id
        if self.policy.min_reputation_to_negotiate is not None:
            rep = self._peer_reputations.get(event.initiator_id, 0.0)
            cooldown = self._cooldown_until.get(event.initiator_id)
            in_cooldown = (cooldown is not None
                           and event.timestamp < cooldown)
            if rep < self.policy.min_reputation_to_negotiate or in_cooldown:
                self._policy_violations += 1
                return
        self._policy_compliant += 1

    def _on_negotiation_outcome(self, event: NegotiationOutcome):
        self._negotiation_outcomes.append(event.success)
        self._pending_negotiations.pop(event.task_id, None)

    def report(self) -> dict:
        total_neg = len(self._negotiation_outcomes)
        successful = sum(1 for s in self._negotiation_outcomes if s)
        total_eval = self._policy_compliant + self._policy_violations

        duration_s = 0.0
        if self._start_time:
            duration_s = (datetime.now() - self._start_time).total_seconds()

        return {
            'policy': self.policy.to_dict(),
            'cooperation_rate': (successful / total_neg) if total_neg > 0 else None,
            'task_throughput': (total_neg / duration_s) if duration_s > 0 else None,
            'policy_compliance_rate': (
                self._policy_compliant / total_eval) if total_eval > 0 else None,
            'violation_count': self._policy_violations,
            'group_size_peak': self._group_size_max,
            'peer_count': self._peer_count,
        }

    def _write_report(self):
        r = self.report()
        if self.output_path:
            with open(self.output_path, 'w') as f:
                json.dump(r, f, indent=2, default=str)
        return r

    _SNAPSHOT_INTERVAL_S = 30.0

    def process(self, queues: dict[str, QueueType], signal: QueueType):
        self._start_time = datetime.now()
        last_snapshot = self._start_time
        while self.keep_running(signal):
            try:
                msg = queues[self.name].get(block=True, timeout=self.q_cadence)
            except queue.Empty:
                if (datetime.now() - last_snapshot).total_seconds() >= self._SNAPSHOT_INTERVAL_S:
                    self._write_report()
                    last_snapshot = datetime.now()
                continue

            if isinstance(msg, Message):
                now = datetime.now()
                self._dispatch_message(msg, now)
                if (now - last_snapshot).total_seconds() >= self._SNAPSHOT_INTERVAL_S:
                    self._write_report()
                    last_snapshot = now

        self._write_report()

    def _dispatch_message(self, msg: Message, now: datetime):
        """Parse an AT Message into a typed kith-covenant event and evaluate it."""
        peer_id = None
        if msg.from_whom:
            peer_id = str(msg.from_whom)
        elif msg.to_whom and isinstance(msg.to_whom, list) and len(msg.to_whom) > 0:
            peer_id = str(msg.to_whom[0])

        event = None

        if msg.function == 'access_granted' and peer_id:
            event = PeerAdmitted(peer_id=peer_id, timestamp=now)

        elif msg.function == 'reputation response':
            rep_peer_id, score = None, None
            if isinstance(msg.obj, Reputation):
                rep_peer_id = str(msg.obj.peer_id)
                score = float(msg.obj.score)
            elif isinstance(msg.obj, (tuple, list)) and len(msg.obj) >= 2:
                rep_peer_id = str(msg.obj[0])
                score = float(msg.obj[1])
            if rep_peer_id and score is not None:
                event = ReputationUpdate(
                    peer_id=rep_peer_id, score=score, timestamp=now)

        elif msg.function == 'invitation':
            if isinstance(msg.obj, TaskInfo):
                task_id = str(msg.obj.uuid)
            else:
                task_id = str(msg.obj) if msg.obj else 'unknown'
            initiator_id = str(msg.from_whom) if msg.from_whom else 'self'
            event = NegotiationStarted(
                task_id=task_id, initiator_id=initiator_id, timestamp=now)

        elif msg.function in ('ack', 'nack'):
            if isinstance(msg.obj, TaskInfo):
                task_id = str(msg.obj.uuid)
            else:
                task_id = str(msg.obj) if msg.obj else 'unknown'
            event = NegotiationOutcome(
                task_id=task_id, success=(msg.function == 'ack'),
                timestamp=now)

        if event is not None:
            self._handle_event(event)
```

- [ ] **Step 5: Run tests to verify they pass**

Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/evaluation/kith_covenant/ \
        lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_observer.py
git commit -m "feat: add KithCovenantObserver AT process plugin"
```

---

### Task 5: KithCovenantInstrumentedAT Wiring

**Files:**
- Create: `SIM/evaluation/kith_covenant/instrumented.py`

- [ ] **Step 1: Implement KithCovenantInstrumentedAT**

Create `SIM/evaluation/kith_covenant/instrumented.py`:

```python
"""Instrumented AT node with both MetricsCollector and KithCovenantObserver."""

from autonomous_trust.simulator.instrumented import InstrumentedAT
from autonomous_trust.simulator.evaluation.kith_covenant.observer import KithCovenantObserver


class KithCovenantInstrumentedAT(InstrumentedAT):
    """AT node that tees messages to both MetricsCollector and KithCovenantObserver."""

    _kith_covenant_policy = None
    _kith_covenant_output = None

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        if self._kith_covenant_policy:
            self.add_worker(KithCovenantObserver,
                            policy=self._kith_covenant_policy,
                            output_path=self._kith_covenant_output)
```

- [ ] **Step 2: Commit**

```bash
git add lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/evaluation/kith_covenant/instrumented.py
git commit -m "feat: add KithCovenantInstrumentedAT wiring class"
```

---

### Task 6: KithCovenantEvaluator with Fitness Scoring

**Files:**
- Create: `SIM/evaluation/kith_covenant/evaluator.py`
- Test: `TESTS/b_integration/test_kith_covenant_evaluator.py`

- [ ] **Step 1: Write the failing test**

Create `TESTS/b_integration/test_kith_covenant_evaluator.py`:

```python
"""Tests for KithCovenantEvaluator fitness scoring."""

import pytest

try:
    from kith_covenant.policy import null_policy, strict_policy
    from kith_covenant.interface import EvaluatorResult
except ImportError:
    pytest.skip("kith_covenant package not on PYTHONPATH", allow_module_level=True)

from autonomous_trust.simulator.evaluation.kith_covenant.evaluator import compute_fitness


class TestFitnessFunction:

    def test_perfect_scores(self):
        observer = {
            'cooperation_rate': 1.0,
            'task_throughput': 1.0,
            'policy_compliance_rate': 1.0,
        }
        metrics = {
            'identity_convergence_s': 0.0,
            'reputation_stability_stddev': 0.0,
            'bandwidth_overhead_fraction': 0.0,
        }
        score = compute_fitness(observer, metrics)
        assert score == pytest.approx(1.0)

    def test_zero_scores(self):
        observer = {
            'cooperation_rate': 0.0,
            'task_throughput': 0.0,
            'policy_compliance_rate': 0.0,
        }
        metrics = {
            'identity_convergence_s': 120.0,
            'reputation_stability_stddev': 0.5,
            'bandwidth_overhead_fraction': 1.0,
        }
        score = compute_fitness(observer, metrics)
        assert score == pytest.approx(0.0)

    def test_none_metrics_score_zero(self):
        observer = {
            'cooperation_rate': None,
            'task_throughput': None,
            'policy_compliance_rate': None,
        }
        metrics = {
            'identity_convergence_s': None,
            'reputation_stability_stddev': None,
            'bandwidth_overhead_fraction': None,
        }
        score = compute_fitness(observer, metrics)
        assert score == 0.0

    def test_strict_vs_null_policy_different_scores(self):
        """Different observer reports produce different fitness scores."""
        high = {
            'cooperation_rate': 0.9,
            'task_throughput': 0.5,
            'policy_compliance_rate': 0.95,
        }
        low = {
            'cooperation_rate': 0.4,
            'task_throughput': 0.1,
            'policy_compliance_rate': 0.3,
        }
        metrics = {
            'identity_convergence_s': 30.0,
            'reputation_stability_stddev': 0.02,
            'bandwidth_overhead_fraction': 0.1,
        }
        high_score = compute_fitness(high, metrics)
        low_score = compute_fitness(low, metrics)
        assert high_score > low_score
```

- [ ] **Step 2: Run test to verify it fails**

Expected: FAIL (module not found)

- [ ] **Step 3: Implement evaluator.py**

Create `SIM/evaluation/kith_covenant/evaluator.py`:

```python
"""KithCovenantEvaluator: orchestrates policy evaluation with fitness scoring.

Provides compute_fitness() for scoring and two evaluation backends:
InProcessEvaluator (fast, mock peers) and DockerEvaluator (full simulation).
"""

import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from typing import Optional

from kith_covenant.interface import EvaluatorResult, KithCovenantEvaluatorProtocol
from kith_covenant.policy import KithCovenantPolicy


# Default normalization bounds
_DEFAULT_MAX_CONVERGENCE_S = 120.0
_DEFAULT_MAX_STDDEV = 0.5
_DEFAULT_MAX_FRACTION = 1.0
_DEFAULT_MAX_THROUGHPUT = 1.0

# Default weights (sum to 1.0)
_DEFAULT_WEIGHTS = {
    'cooperation_rate': 0.30,
    'task_throughput': 0.10,
    'policy_compliance_rate': 0.25,
    'identity_convergence': 0.15,
    'reputation_stability': 0.15,
    'bandwidth_overhead': 0.05,
}


def compute_fitness(
    observer_report: dict,
    metrics_report: dict,
    weights: Optional[dict] = None,
    max_convergence_s: float = _DEFAULT_MAX_CONVERGENCE_S,
    max_stddev: float = _DEFAULT_MAX_STDDEV,
    max_fraction: float = _DEFAULT_MAX_FRACTION,
    max_throughput: float = _DEFAULT_MAX_THROUGHPUT,
) -> float:
    """Compute weighted fitness score from observer and metrics reports.

    Each metric is normalized to [0, 1] before weighting.
    None values score 0.
    """
    w = weights or _DEFAULT_WEIGHTS

    def _val(d, key):
        v = d.get(key)
        return v if v is not None else 0.0

    scores = {
        'cooperation_rate': _val(observer_report, 'cooperation_rate'),
        'task_throughput': min(
            _val(observer_report, 'task_throughput') / max_throughput, 1.0),
        'policy_compliance_rate': _val(
            observer_report, 'policy_compliance_rate'),
        'identity_convergence': 1.0 - min(
            _val(metrics_report, 'identity_convergence_s') / max_convergence_s,
            1.0),
        'reputation_stability': 1.0 - min(
            _val(metrics_report, 'reputation_stability_stddev') / max_stddev,
            1.0),
        'bandwidth_overhead': 1.0 - min(
            _val(metrics_report, 'bandwidth_overhead_fraction') / max_fraction,
            1.0),
    }

    return sum(w.get(k, 0.0) * v for k, v in scores.items())


class InProcessEvaluator:
    """Fast in-process evaluation using mock peers."""

    def __init__(self, duration_s: float = 30.0, **fitness_kwargs):
        self._duration_s = duration_s
        self._fitness_kwargs = fitness_kwargs

    def evaluate(self, policy: KithCovenantPolicy) -> EvaluatorResult:
        """Run a short in-process simulation and score the policy."""
        import tempfile
        from autonomous_trust.simulator.evaluation.kith_covenant.instrumented import (
            KithCovenantInstrumentedAT,
        )

        with tempfile.TemporaryDirectory() as td:
            metrics_path = os.path.join(td, 'metrics.json')
            observer_path = os.path.join(td, 'kith_covenant.json')

            KithCovenantInstrumentedAT._metrics_output = metrics_path
            KithCovenantInstrumentedAT._kith_covenant_policy = policy
            KithCovenantInstrumentedAT._kith_covenant_output = observer_path

            try:
                # Run with testing=True for short duration
                at = KithCovenantInstrumentedAT(
                    multiproc=True, testing=True)
                at.run_forever()
            except (KeyboardInterrupt, SystemExit):
                pass
            finally:
                KithCovenantInstrumentedAT._metrics_output = None
                KithCovenantInstrumentedAT._kith_covenant_policy = None
                KithCovenantInstrumentedAT._kith_covenant_output = None

            metrics_report = {}
            if os.path.exists(metrics_path):
                with open(metrics_path) as f:
                    metrics_report = json.load(f)

            observer_report = {}
            if os.path.exists(observer_path):
                with open(observer_path) as f:
                    observer_report = json.load(f)

            score = compute_fitness(
                observer_report, metrics_report, **self._fitness_kwargs)
            return EvaluatorResult(
                policy=policy,
                observer_report=observer_report,
                metrics_report=metrics_report,
                fitness_score=score,
            )


class DockerEvaluator:
    """Run full Docker simulation with a policy and collect results."""

    def __init__(self, script_path: str, **kwargs):
        self._script = script_path
        self._extra_args = kwargs

    def evaluate(self, policy: KithCovenantPolicy) -> EvaluatorResult:
        with tempfile.TemporaryDirectory() as td:
            policy_path = os.path.join(td, 'policy.json')
            with open(policy_path, 'w') as f:
                json.dump(policy.to_dict(), f)

            metrics_path = os.path.join(td, 'metrics.json')
            observer_path = os.path.join(td, 'kith_covenant.json')

            cmd = [
                'bash', self._script,
                '--python', '--quick',
                '--kith-covenant-policy', policy_path,
                '--kith-covenant-output', observer_path,
                '--output', metrics_path,
            ]
            subprocess.run(cmd, check=True, capture_output=True)

            metrics_report = {}
            if os.path.exists(metrics_path):
                with open(metrics_path) as f:
                    metrics_report = json.load(f)

            observer_report = {}
            if os.path.exists(observer_path):
                with open(observer_path) as f:
                    observer_report = json.load(f)

            score = compute_fitness(observer_report, metrics_report)
            return EvaluatorResult(
                policy=policy,
                observer_report=observer_report,
                metrics_report=metrics_report,
                fitness_score=score,
            )
```

- [ ] **Step 4: Run tests to verify they pass**

Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/autonomous_trust/simulator/evaluation/kith_covenant/evaluator.py \
        lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/tests/b_integration/test_kith_covenant_evaluator.py
git commit -m "feat: add KithCovenantEvaluator with fitness scoring and Docker backend"
```

---

### Task 7: Shell Script Integration

**Files:**
- Modify: `AT_SIM/config/test-simulation-scenarios.sh`

- [ ] **Step 1: Add PYTHONPATH for kith-covenant**

In `test-simulation-scenarios.sh`, after the existing PYTHONPATH export (line 22), add kith-covenant:

```bash
KITH_COVENANT_SRC="$(cd "$AT_ROOT/../kith-covenant/src/python" && pwd 2>/dev/null || echo "")"
if [ -n "$KITH_COVENANT_SRC" ]; then
    export PYTHONPATH="${KITH_COVENANT_SRC}:${PYTHONPATH}"
fi
```

- [ ] **Step 2: Add --kith-covenant-policy and --kith-covenant-output flags to argument parser**

In the `while` loop (after `--caldera-attacks`), add:

```bash
        --kith-covenant-policy)
            KITH_COVENANT_POLICY="$2"
            shift 2
            ;;
        --kith-covenant-output)
            KITH_COVENANT_OUTPUT="$2"
            shift 2
            ;;
```

Initialize `KITH_COVENANT_POLICY=""` and `KITH_COVENANT_OUTPUT=""` alongside the other variables.

- [ ] **Step 3: Add kith-covenant policy patching step**

After the CALDERA patching block (step 1b), add step 1c:

```bash
# Step 1c: Configure Kith-Covenant policy (if --kith-covenant-policy)
if [[ -n "$KITH_COVENANT_POLICY" ]]; then
    echo "Configuring Kith-Covenant policy ..."
    cp "$KITH_COVENANT_POLICY" "$WORK_DIR/kith-covenant-policy.json"
    # Default observer output to metrics dir if not specified
    KITH_COVENANT_OUTPUT="${KITH_COVENANT_OUTPUT:-$METRICS_DIR/kith_covenant.json}"
    echo "  Policy: $KITH_COVENANT_POLICY"
    echo "  Observer output: $KITH_COVENANT_OUTPUT"
fi
```

- [ ] **Step 4: Add kith-covenant info to banner output**

After the CALDERA info lines, add:

```bash
echo "Kith-Covenant: ${KITH_COVENANT_POLICY:-disabled}"
```

- [ ] **Step 5: Commit**

```bash
git add lib/muudd/lib/autonomous_trust/src/autonomous-trust-simulator/config/test-simulation-scenarios.sh
git commit -m "feat: add --kith-covenant-policy flag and PYTHONPATH to test harness"
```

---

### Task 8: Final Verification

- [ ] **Step 1: Run all kith-covenant-related tests**

```bash
cd lib/muudd/lib/autonomous_trust && \
PYTHONPATH="src/autonomous-trust:src/autonomous-trust-services:src/autonomous-trust-inspector:src/autonomous-trust-simulator:../../kith-covenant/src/python" \
python -m pytest \
  src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_interface.py \
  src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_policy.py \
  src/autonomous-trust-simulator/tests/a_unit/test_kith_covenant_observer.py \
  src/autonomous-trust-simulator/tests/a_unit/test_tee_queue_fanout.py \
  src/autonomous-trust-simulator/tests/b_integration/test_kith_covenant_evaluator.py \
  -v
```

Expected: all PASS

- [ ] **Step 2: Run existing tests to verify no regressions**

```bash
cd lib/muudd/lib/autonomous_trust && \
PYTHONPATH="src/autonomous-trust:src/autonomous-trust-services:src/autonomous-trust-inspector:src/autonomous-trust-simulator:../../kith-covenant/src/python" \
python -m pytest \
  src/autonomous-trust-simulator/tests/a_unit/test_caldera_compose.py \
  src/autonomous-trust-simulator/tests/b_integration/test_metrics_collector.py \
  -v
```

Expected: all PASS (existing tests unaffected)

- [ ] **Step 3: Commit all remaining changes**

Ensure everything is committed and clean.
