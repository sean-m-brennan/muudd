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
