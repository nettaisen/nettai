"""Making a pool (`nettai_gym.Pool`, or `FakePool` with --fake) and reading its buffers as torch tensors.

The trainer runs the same code on both; only the constructor differs.
"""

from __future__ import annotations

import os
from typing import Any

import numpy as np
import torch

from .config import EnvConfig
from .fake import FakePool
from .model import Obs
from .spec import Spec


def make_pool(
    env: EnvConfig,
    fake: bool,
    *,
    games: list[str] | None = None,
    envs: int | None = None,
    threads: int | None = None,
    seed: int | None = None,
    hp: tuple[int, int],
    holdout_mode: str = "exclude",
):
    """A pool for `env`; games/envs/threads/seed override it (evaluation pools). holdout_mode is "exclude" for
    training (held-out chips never in builds) and "favor" for evaluation."""
    threads = env.threads if threads is None else threads
    kwargs: dict[str, Any] = dict(
        games=list(env.games if games is None else games),
        envs=env.envs if envs is None else envs,
        threads=threads if threads > 0 else (os.cpu_count() or 1),
        seed=env.seed if seed is None else seed,
        content=env.content,
        packs=env.packs,
        k=env.k,
        delay=tuple(env.delay),
        max_fight_ticks=env.max_fight_ticks,
        hp=tuple(hp),
        full_lists=env.full_lists,
        builders=env.builders,
        cache=env.cache,
        mirror=env.mirror,
        fixed_builds=env.fixed_builds,
    )
    if env.chip_holdout > 0:
        kwargs["chip_holdout"] = (env.chip_holdout, env.chip_holdout_seed, holdout_mode)
    if fake:
        return FakePool(**kwargs)
    try:
        import nettai_gym
    except ImportError as e:
        raise SystemExit(
            "nettai_gym isn't built in this environment. In training/, run "
            "`uv run maturin develop --release --manifest-path bridge/Cargo.toml`, or pass --fake."
        ) from e
    return nettai_gym.Pool(**kwargs)


def pool_spec(pool) -> dict[str, Any]:
    """The pool's spec as the trainer reads it. Where the spec gives `chip_tokens` (the bridge today: the hand,
    the screen's slots and the other's used chip as ranges of its `chips` set), those ranges become sets of
    their own, in place of `chips` and in that order, so the pointer set is `slots` and the used chip is a set
    of one. The split is recorded under "split" (the architecture carries it for the Rust side); the hash is
    the pool's own."""
    raw = pool.spec()
    if "chip_tokens" not in raw:
        return raw
    parts = sorted(raw["chip_tokens"].items(), key=lambda kv: kv[1][0])
    chips = raw["sets"]["chips"]
    covered = 0
    for _name, (start, count) in parts:
        if start != covered:
            raise ValueError(f"spec: chip_tokens {raw['chip_tokens']} don't tile the chips set")
        covered += count
    if covered != chips["count"]:
        raise ValueError(f"spec: chip_tokens cover {covered} of the chips set's {chips['count']} tokens")
    sets: dict[str, Any] = {}
    for name, s in raw["sets"].items():
        if name != "chips":
            sets[name] = s
            continue
        for part, (_start, count) in parts:
            if part in raw["sets"]:
                raise ValueError(f"spec: chip_tokens' {part!r} is also a set")
            sets[part] = {**{k: v for k, v in s.items() if k != "count"}, "count": count}
    out = {k: v for k, v in raw.items() if k != "chip_tokens"}
    out["sets"] = sets
    out["split"] = {"chips": [[part, start, count] for part, (start, count) in parts]}
    return out


def _arrays(buf: dict[str, Any], key: str, raw: dict[str, Any]) -> dict[str, np.ndarray]:
    """buf[key] by the trainer's sets: a split set is a view of its source's token range (no copy)."""
    out = dict(buf[key])
    for source, parts in raw.get("split", {}).items():
        a = out.pop(source)
        for part, start, count in parts:
            out[part] = a[:, :, start : start + count]
    return out


def read_round_stats(pool) -> list[dict[str, Any]]:
    """`pool.round_stats()`, checked for what the trainer reads: each round's "env", "ticks", "truncated" and
    "seats" (two dicts, each with "result")."""
    rounds = pool.round_stats()
    for r in rounds:
        if not {"env", "ticks", "truncated", "seats"} <= set(r) or len(r["seats"]) != 2:
            raise RuntimeError(f"round_stats(): a round without env, ticks, truncated and two seats: {sorted(r)}")
    return rounds


class PoolView:
    """Torch views of the pool's buffers (shared memory, refilled by every step), seats flattened to streams:
    stream s = env · 2 + seat."""

    def __init__(self, pool, spec: Spec):
        raw = dict(pool.buffers())
        buf = dict(raw)
        for key in ("x", "ids", "mask"):
            buf[key] = _arrays(buf, key, spec.raw)
        self.E = buf["phase"].shape[0]
        S = self.E * 2
        self.S = S
        self.spec = spec
        t = lambda a: torch.from_numpy(a)
        for s in spec.sets:
            for key, shape in (("x", (s.count, s.width)), ("ids", (s.count, s.ids)), ("mask", (s.count,))):
                a = buf[key][s.name]
                want = (self.E, 2) + shape
                if a.shape != want:
                    raise ValueError(f"pool buffer {key}[{s.name}] is {a.shape}, the spec says {want}")
        # Each set's tokens as rows of the buffer they live in: stream s's token j is row s·C + first + j of x and
        # ids, C the buffer's tokens a stream (a split set is a range of its source's tokens). The collector
        # gathers present tokens from these.
        split = {part: (src, first) for src, parts in spec.raw.get("split", {}).items() for part, first, _n in parts}
        self.source: dict[str, tuple[torch.Tensor, torch.Tensor, int, int]] = {}
        for s in spec.sets:
            src, first = split.get(s.name, (s.name, 0))
            xa, ia = raw["x"][src], raw["ids"][src]
            C = xa.shape[2]
            self.source[s.name] = (t(xa).view(S * C, s.width), t(ia).view(S * C, s.ids), C, first)
        # view, never reshape: these must stay the pool's memory, which every step refills.
        self.x = {s.name: t(buf["x"][s.name]).view(S, s.count, s.width) for s in spec.sets}
        self.ids = {s.name: t(buf["ids"][s.name]).view(S, s.count, s.ids) for s in spec.sets}
        self.mask = {s.name: t(buf["mask"][s.name]).view(S, s.count) for s in spec.sets}
        self.phase = t(buf["phase"]).view(S)
        self.legal_fight = t(buf["legal_fight"]).view(S, spec.fight_actions)
        self.legal_screen = t(buf["legal_screen"]).view(S, spec.screen_actions)
        self.hp_gain = t(buf["hp_gain"]).view(S)
        self.done = t(buf["done"])
        self.truncated = t(buf["truncated"])
        self.ending = t(buf["ending"])
        self.result = t(buf["result"]).view(S)
        self.game = t(buf["game"])
        self.delay = t(buf["delay"]).view(S)
        for name, a, dtype in (
            ("phase", buf["phase"], np.int8),
            ("hp_gain", buf["hp_gain"], np.float32),
            ("result", buf["result"], np.int8),
            ("done", buf["done"], np.bool_),
            ("ending", buf["ending"], np.int8),
        ):
            if a.dtype != dtype:
                raise ValueError(f"pool buffer {name} is {a.dtype}, §6.6 says {np.dtype(dtype)}")

    def obs(self, idx: torch.Tensor, device: torch.device) -> Obs:
        """The observations of streams idx (a CPU index tensor), copied to the device."""
        return {
            "x": {n: v[idx].to(device, non_blocking=True) for n, v in self.x.items()},
            "ids": {n: v[idx].to(device, non_blocking=True) for n, v in self.ids.items()},
            "mask": {n: v[idx].to(device, non_blocking=True) for n, v in self.mask.items()},
        }
