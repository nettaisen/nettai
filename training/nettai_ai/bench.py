"""Where training's time goes (§11): the simulation, the rollout's inference and the PPO update, on one
rollout of the real pool (or FakePool), and the update's cost under variants run on the same data.

    NETTAI_PACKS=../data uv run --no-sync python -m nettai_ai.bench [--device cpu] [--envs 64] [--steps 128]
        [--variants base,epochs1,...] [--profile]

Each variant starts from the same weights and optimizer state and reports learner decisions a second
through one PPO update (all its epochs). --profile breaks the base update down by kind of op.
"""

from __future__ import annotations

import argparse
import copy
import tempfile
import time
from collections import defaultdict

import numpy as np
import torch

from .config import load_config
from .ppo import compute_gae, ppo_update
from .train import Trainer

VARIANTS = {
    "base": {},
    "padded": {"pack": False},  # every slot at fixed shapes, as a compiled or GPU-graph run would
    "epochs2": {"epochs": 2},
    "epochs1": {"epochs": 1},
    "mb32": {"minibatch_chunks": 32},
    "mb64": {"minibatch_chunks": 64},
    "chunk32": {"chunk": 32, "minibatch_chunks": 64},
    "compile": {"compile": True},
    "compile_padded": {"compile": True, "pack": False},
    "bf16": {"autocast": torch.bfloat16},  # meaningful on CUDA; on a CPU without bf16 units, slower
}

CATEGORIES = [
    ("matmul", ("mm", "addmm", "bmm", "linear", "matmul", "baddbmm")),
    ("attention", ("scaled_dot_product", "_flash_attention", "_efficient_attention")),
    ("gru", ("gru", "_thnn_fused")),
    ("embedding", ("embedding",)),
    ("indexing", ("index", "gather", "scatter", "argsort", "sort", "nonzero", "masked_select")),
    ("layer_norm", ("layer_norm",)),
    ("optimizer", ("adam", "_foreach", "lerp", "addcdiv", "addcmul")),
    ("copies", ("copy_", "cat", "stack", "to", "contiguous", "clone", "fill_", "zero_", "expand", "pad")),
]


class TimedPool:
    """The pool with its step timed."""

    def __init__(self, pool):
        self.pool = pool
        self.step_s = 0.0

    def step(self, actions):
        t = time.perf_counter()
        self.pool.step(actions)
        self.step_s += time.perf_counter() - t

    def __getattr__(self, name):
        return getattr(self.pool, name)


def category(name: str) -> str:
    n = name.removeprefix("aten::")
    for cat, keys in CATEGORIES:
        if any(k in n for k in keys):
            return cat
    return "elementwise & other"


def run_update(tr: Trainer, adv, ret, variant: dict, state, opt_state, profile: bool = False) -> tuple[float, dict]:
    model, opt = tr.model, tr.opt
    model.load_state_dict(state)
    opt.load_state_dict(opt_state)
    model.pack = variant.get("pack", True)
    cfg = copy.replace(tr.cfg.ppo, **{k: v for k, v in variant.items() if k in ("epochs", "minibatch_chunks", "chunk")})
    fwd_model = model
    if variant.get("compile"):
        model.encode = torch.compile(model.encode, dynamic=model.pack)
    gen = torch.Generator().manual_seed(0)
    dev = tr.device
    ctx = torch.autocast(dev.type, dtype=variant["autocast"]) if "autocast" in variant else torch.autocast(dev.type, enabled=False)
    prof = None
    try:
        if variant.get("compile"):  # compile outside the timing: one update to warm up
            with ctx:
                ppo_update(fwd_model, opt, tr.rollout, adv, ret, cfg, gen)
            model.load_state_dict(state)
            opt.load_state_dict(opt_state)
        if profile:
            prof = torch.profiler.profile(activities=[torch.profiler.ProfilerActivity.CPU])
            prof.__enter__()
        if dev.type == "cuda":
            torch.cuda.synchronize()
        t = time.perf_counter()
        with ctx:
            stats = ppo_update(fwd_model, opt, tr.rollout, adv, ret, cfg, gen)
        if dev.type == "cuda":
            torch.cuda.synchronize()
        dt = time.perf_counter() - t
    finally:
        if prof is not None:
            prof.__exit__(None, None, None)
        if variant.get("compile"):
            del model.encode  # back to the method
        model.pack = True
    out = {"seconds": dt, "decisions": stats["decisions"], "minibatches": stats["minibatches"], "loss_v": stats["loss_value"]}
    if prof is not None:
        cats: dict[str, float] = defaultdict(float)
        total = 0.0
        for e in prof.key_averages():
            cats[category(e.key)] += e.self_cpu_time_total
            total += e.self_cpu_time_total
        out["profile"] = {k: v / total for k, v in sorted(cats.items(), key=lambda kv: -kv[1])}
    return dt, out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fake", action="store_true")
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--envs", type=int, default=64)
    ap.add_argument("--threads", type=int, default=6, help="the pool's threads")
    ap.add_argument("--torch-threads", type=int, default=4)
    ap.add_argument("--steps", type=int, default=128)
    ap.add_argument("--games", default="exe6")
    ap.add_argument("--variants", default="base,padded,epochs2,epochs1,mb32,mb64,chunk32,compile")
    ap.add_argument("--profile", action="store_true")
    ap.add_argument("--set", action="append", default=[])
    a = ap.parse_args()
    over = [
        f"env.games={a.games.split(',')!r}".replace("'", '"'), f"env.envs={a.envs}", f"env.threads={a.threads}",
        f"ppo.steps={a.steps}", f'run.device="{a.device}"', f"run.torch_threads={a.torch_threads}",
        "ppo.norm_warmup_steps=8", "league.snapshot_every=1000000",
    ] + a.set
    cfg = load_config(None, over)
    tr = Trainer(cfg, tempfile.mkdtemp(), a.fake, log=False)
    timed = TimedPool(tr.pool)
    tr.collector.pool = timed
    t = time.perf_counter()
    cs = tr.collector.collect(cfg.ppo.steps, 1.0, 1.0)
    collect_s = time.perf_counter() - t
    decisions = cs.learner_decisions + cs.opponent_decisions
    sim_s = timed.step_s
    print(f"collect: {decisions:,} decisions ({cs.learner_decisions:,} learner) in {collect_s:.2f} s")
    print(f"  simulation (pool.step, {a.threads} threads): {sim_s:.2f} s = {decisions / sim_s:,.0f} decisions/s")
    print(f"  inference and Python (the rest, {a.torch_threads} torch threads): {collect_s - sim_s:.2f} s = {decisions / (collect_s - sim_s):,.0f} decisions/s")
    ro = tr.rollout
    adv, ret = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, cfg.ppo.gamma, cfg.ppo.lam)
    present = {k: v / max(1, cs.learner_decisions) for k, v in cs.present.items()}
    print("  present tokens a decision: " + ", ".join(f"{k} {v:.1f}" for k, v in present.items()))
    state = copy.deepcopy(tr.model.state_dict())
    opt_state = copy.deepcopy(tr.opt.state_dict())
    print(f"update ({cfg.ppo.epochs} epochs, chunks of {cfg.ppo.chunk}, {cfg.ppo.minibatch_chunks} a minibatch unless the variant says):")
    for name in a.variants.split(","):
        v = VARIANTS[name]
        try:
            dt, out = run_update(tr, adv, ret, v, state, opt_state, profile=a.profile and name == "base")
        except Exception as e:  # a variant this machine can't run is reported, the others still run
            print(f"  {name:15s} failed: {type(e).__name__}: {str(e).splitlines()[0][:120]}")
            continue
        print(f"  {name:15s} {out['decisions'] / dt:8,.0f} decisions/s  ({dt:.2f} s, {out['minibatches']} minibatches)")
        if "profile" in out:
            print("    " + ", ".join(f"{k} {100 * v:.0f}%" for k, v in out["profile"].items()))


if __name__ == "__main__":
    main()
