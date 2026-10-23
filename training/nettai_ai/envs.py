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


def read_round_stats(pool, done_envs: np.ndarray) -> list[dict[str, Any]]:
    """`pool.round_stats()` as §6.6 has it: each round {"env", "ticks", "seats": [seat 0's, seat 1's], ...}.

    The bridge today gives each round's per-seat measures as pairs and no env; its rounds come in env order,
    one per env that ended a round in the last step (the trainer reads them after every step that ends one),
    so they are matched to `done_envs` in order and regrouped by seat."""
    rounds = pool.round_stats()
    if all("seats" in r for r in rounds):
        return rounds
    if len(rounds) != len(done_envs):
        raise RuntimeError(f"round_stats() gave {len(rounds)} rounds for {len(done_envs)} ended")
    out = []
    for e, r in zip(sorted(int(x) for x in done_envs), rounds):
        pairs = {k: v for k, v in r.items() if isinstance(v, (list, tuple)) and len(v) == 2}
        seats = [{k: v[s] for k, v in pairs.items()} for s in (0, 1)]
        out.append({**{k: v for k, v in r.items() if k not in pairs}, "env": e, "seats": seats})
    return out


class PoolView:
    """Torch views of the pool's buffers (shared memory, refilled by every step), seats flattened to streams:
    stream s = env · 2 + seat."""

    def __init__(self, pool, spec: Spec):
        buf = dict(pool.buffers())
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
        self.result = t(buf["result"]).view(S)
        self.game = t(buf["game"])
        self.delay = t(buf["delay"]).view(S)
        for name, a, dtype in (
            ("phase", buf["phase"], np.int8),
            ("hp_gain", buf["hp_gain"], np.float32),
            ("result", buf["result"], np.int8),
            ("done", buf["done"], np.bool_),
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
