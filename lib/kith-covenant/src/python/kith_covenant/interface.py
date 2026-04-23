"""Kith-Covenant event types and evaluator protocol.

These frozen dataclasses are the stable interface between the AT
simulator and Kith-Covenant policy evaluation.  Policies reason about
events, never raw AT Message objects.
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
