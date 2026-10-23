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


class PoolView:
    """Torch views of the pool's buffers (shared memory, refilled by every step), seats flattened to streams:
    stream s = env · 2 + seat."""

    def __init__(self, pool, spec: Spec):
        buf = pool.buffers()
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
