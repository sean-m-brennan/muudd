#  Copyright 2026 Sean M. Brennan and contributors
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.

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
