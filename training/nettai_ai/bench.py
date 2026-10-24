"""Where training's time goes (§11): the simulation, the rollout's inference and the PPO update, on one
rollout of the real pool (or FakePool), the update's cost under variants run on the same data, and with --loop
the overlapped loop itself.

    NETTAI_PACKS=../data uv run --no-sync python -m nettai_ai.bench [--device cpu] [--envs 64] [--steps 128]
        [--variants base,epochs1,...] [--profile] [--loop N]

Each variant starts from the same weights and optimizer state and reports learner decisions a second
through one PPO update (all its epochs). --profile breaks the base update down by kind of op. --loop N then
runs N updates of the training loop (collection and update overlapped, as train.py runs) and reports decisions a
second, how busy the collector and the learner were, how much slower each was than alone (the GIL, the cores
and the device they share), and the policy lag.
"""

from __future__ import annotations

import argparse
import copy
import tempfile
import time
from collections import defaultdict

import torch

from .config import load_config
from .device import pin_torch_threads, synchronize
from .ppo import compute_gae, ppo_update
from .train import Trainer

VARIANTS = {
    "base": {},
    "padded": {"pack": False},  # every slot at fixed shapes, as a compiled or GPU-graph run would
    "epochs2": {"epochs": 2},
    "epochs1": {"epochs": 1},
    "mb32": {"minibatch_chunks": 32},
    "mb64": {"minibatch_chunks": 64},
    "mb128": {"minibatch_chunks": 128},
    "mb256": {"minibatch_chunks": 256},
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
    if variant.get("compile"):
        model.encode = torch.compile(model.encode, dynamic=model.pack)
    gen = torch.Generator().manual_seed(0)
    dev = tr.device
    ro = tr.collector.ro
    ctx = torch.autocast(dev.type, dtype=variant["autocast"]) if "autocast" in variant else torch.autocast(dev.type, enabled=False)
    prof = None
    try:
        if variant.get("compile"):  # compile outside the timing: one update to warm up
            with ctx:
                ppo_update(model, opt, ro, adv, ret, cfg, gen)
            model.load_state_dict(state)
            opt.load_state_dict(opt_state)
        if profile:
            prof = torch.profiler.profile(activities=[torch.profiler.ProfilerActivity.CPU])
            prof.__enter__()
        synchronize(dev)
        t = time.perf_counter()
        with ctx:
            stats = ppo_update(model, opt, ro, adv, ret, cfg, gen)
        synchronize(dev)
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


def report_loop(rows: list[dict], alone_collect: float, alone_rest: float, alone_rest_cpu: float, alone_update: float) -> None:
    skip = min(2, len(rows) - 1)  # the first updates fill the pipeline
    steady = rows[skip:]
    total = lambda k: sum(r[k] for r in steady)
    wall = total("time/wall")
    decisions = total("learner_decisions") + total("opponent_decisions")
    n = len(steady)
    collect, step, collect_cpu = total("time/collect") / n, total("time/step") / n, total("time/collect_cpu") / n
    update, update_cpu = total("time/update") / n, total("time/update_cpu") / n
    rest = collect - step
    print(f"loop: {len(rows)} updates; updates {skip + 1} to {len(rows)}: {decisions:,} decisions in {wall:.1f} s = "
          f"{decisions / wall:,.0f} decisions/s ({total('learner_decisions') / wall:,.0f} the learner's)")
    print(f"  collector busy {100 * total('time/collect') / wall:.0f}%: a rollout {collect:.2f} s (alone {alone_collect:.2f} s), "
          f"pool.step {step:.2f} s, the rest {rest:.2f} s (alone {alone_rest:.2f} s: {rest / max(alone_rest, 1e-9):.2f}×); "
          f"its thread on a CPU {collect_cpu:.2f} s = {100 * collect_cpu / max(rest, 1e-9):.0f}% of the rest (alone "
          f"{100 * alone_rest_cpu / max(alone_rest, 1e-9):.0f}%); waiting for a free rollout {100 * total('time/collector_wait') / wall:.0f}%")
    print(f"  learner busy {100 * total('time/update') / wall:.0f}%: an update {update:.2f} s (alone {alone_update:.2f} s: "
          f"{update / max(alone_update, 1e-9):.2f}×), its thread on a CPU {update_cpu:.2f} s = {100 * update_cpu / max(update, 1e-9):.0f}%; "
          f"waiting for a rollout {100 * total('time/learner_wait') / wall:.0f}%")
    lag = total("loop/policy_lag") / n
    print(f"  policy lag {lag:.2f} updates; the first epoch against the behavior policy: ratio "
          f"{total('ppo/behavior_ratio') / n:.4f}, KL {total('ppo/behavior_kl') / n:.5f} (all epochs: KL {total('ppo/approx_kl') / n:.5f})")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fake", action="store_true")
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--envs", type=int, default=64)
    ap.add_argument("--threads", type=int, default=6, help="the pool's threads")
    ap.add_argument("--torch-threads", type=int, default=4, help="the collector's torch CPU threads")
    ap.add_argument("--update-threads", type=int, default=0, help="the learner's (0: as the collector's)")
    ap.add_argument("--steps", type=int, default=128)
    ap.add_argument("--games", default="exe6")
    ap.add_argument("--variants", default="base,padded,epochs2,epochs1,mb32,mb64,chunk32,compile")
    ap.add_argument("--profile", action="store_true")
    ap.add_argument("--loop", type=int, default=0, metavar="N", help="then N updates of the overlapped training loop")
    ap.add_argument("--set", action="append", default=[])
    a = ap.parse_args()
    over = [
        f"env.games={a.games.split(',')!r}".replace("'", '"'), f"env.envs={a.envs}", f"env.threads={a.threads}",
        f"ppo.steps={a.steps}", f'run.device="{a.device}"', f"run.torch_threads={a.torch_threads}",
        f"run.update_threads={a.update_threads}", "ppo.norm_warmup_steps=8", "league.snapshot_every=1000000",
        f"run.updates={max(a.loop, 1)}", "run.checkpoint_every=1000000",
    ] + a.set
    cfg = load_config(None, over)
    tr = Trainer(cfg, tempfile.mkdtemp(), a.fake, log=False)
    pin_torch_threads(tr.collect_threads)
    with tr.collect_lane.scope():
        t, c = time.perf_counter(), time.thread_time()
        cs = tr.collector.collect(cfg.ppo.steps, 1.0, 1.0)
        synchronize(tr.device)
        collect_s, collect_cpu = time.perf_counter() - t, time.thread_time() - c
    decisions = cs.learner_decisions + cs.opponent_decisions
    sim_s = cs.step_seconds
    rest_s = collect_s - sim_s
    print(f"collect: {decisions:,} decisions ({cs.learner_decisions:,} learner) in {collect_s:.2f} s")
    print(f"  simulation (pool.step, {a.threads} threads): {sim_s:.2f} s = {decisions / sim_s:,.0f} decisions/s")
    print(f"  inference and Python (the rest, {tr.collect_threads} torch threads): {rest_s:.2f} s = {decisions / rest_s:,.0f} decisions/s")
    ro = tr.collector.ro
    adv, ret = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, cfg.ppo.gamma, cfg.ppo.lam)
    present = {k: v / max(1, cs.learner_decisions) for k, v in cs.present.items()}
    print("  present tokens a decision: " + ", ".join(f"{k} {v:.1f}" for k, v in present.items()))
    size = ro.nbytes()
    print(f"  a rollout's storage: {size['host'] / 2**20:,.0f} MiB on the CPU, {size['device'] / 2**20:,.0f} MiB on the {tr.device.type} (two are kept)")
    state = copy.deepcopy(tr.model.state_dict())
    opt_state = copy.deepcopy(tr.opt.state_dict())
    pin_torch_threads(tr.update_threads)
    names = [v for v in a.variants.split(",") if v]
    if a.loop and "base" not in names:
        names.insert(0, "base")  # the update alone, for the loop's comparison
    print(f"update ({cfg.ppo.epochs} epochs, chunks of {cfg.ppo.chunk}, {cfg.ppo.minibatch_chunks} a minibatch unless the variant says, "
          f"{tr.update_threads} torch threads):")
    alone_update = None
    for name in names:
        v = VARIANTS[name]
        try:
            dt, out = run_update(tr, adv, ret, v, state, opt_state, profile=a.profile and name == "base")
        except Exception as e:  # a variant this machine can't run (or fit) is reported, the others still run
            print(f"  {name:15s} failed: {type(e).__name__}: {str(e).splitlines()[0][:120]}")
            continue
        finally:
            torch.cuda.empty_cache()  # what a big variant held goes back before the next (nothing off CUDA)
        if name == "base":
            alone_update = dt
        print(f"  {name:15s} {out['decisions'] / dt:8,.0f} decisions/s  ({dt:.2f} s, {out['minibatches']} minibatches)")
        if "profile" in out:
            print("    " + ", ".join(f"{k} {100 * v:.0f}%" for k, v in out["profile"].items()))
    if a.loop:
        tr.model.load_state_dict(state)
        tr.opt.load_state_dict(opt_state)
        rows = tr.train(a.loop)
        report_loop(rows, collect_s, rest_s, collect_cpu, alone_update or float("nan"))


if __name__ == "__main__":
    main()
