"""The observation spec (§6.6): what `pool.spec()` returns, checked once and read everywhere.

Every width and count the trainer uses comes from here, never from a constant.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class TokenSet:
    name: str
    count: int  # token slots
    width: int  # floats per token
    ids: int  # hashed name ids per token (pairs: each name comes as two ids from two seeds)


@dataclass(frozen=True)
class Spec:
    hash: str
    buckets: int
    fight_actions: int
    screen_actions: int
    sets: tuple[TokenSet, ...]  # in the spec's fixed order
    raw: dict[str, Any]

    @property
    def names(self) -> list[str]:
        return [s.name for s in self.sets]

    def set(self, name: str) -> TokenSet:
        for s in self.sets:
            if s.name == name:
                return s
        raise KeyError(f"the spec has no token set {name!r} (it has: {', '.join(self.names)})")

    @property
    def tokens(self) -> int:
        return sum(s.count for s in self.sets)


def parse_spec(raw: dict[str, Any]) -> Spec:
    for key in ("hash", "buckets", "fight_actions", "screen_actions", "sets"):
        if key not in raw:
            raise ValueError(f"spec: missing {key!r}")
    sets = []
    for name, s in raw["sets"].items():
        if not name.isidentifier():
            raise ValueError(f"spec: set name {name!r} must be an identifier (it names weights)")
        for key in ("count", "width", "ids"):
            if key not in s:
                raise ValueError(f"spec: set {name!r} has no {key!r}")
        if s["count"] < 1 or s["width"] < 0 or s["ids"] < 0:
            raise ValueError(f"spec: set {name!r} has a bad shape {s}")
        sets.append(TokenSet(name, int(s["count"]), int(s["width"]), int(s["ids"])))
    return Spec(
        hash=str(raw["hash"]),
        buckets=int(raw["buckets"]),
        fight_actions=int(raw["fight_actions"]),
        screen_actions=int(raw["screen_actions"]),
        sets=tuple(sets),
        raw=raw,
    )


def spec_hash(raw: dict[str, Any]) -> str:
    """The fake's hash: sha256 of the spec without its hash, as canonical JSON. The real pool hashes its own
    encoder; the trainer only compares hashes, never recomputes the pool's."""
    body = {k: v for k, v in raw.items() if k != "hash"}
    return hashlib.sha256(json.dumps(body, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
