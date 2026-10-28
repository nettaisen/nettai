"""Training (§8): recurrent PPO by self-play in a league, on the pool's envs.

    uv run python -m nettai_ai.train --run-dir runs/NAME [--config configs/x.toml] [--size large]
        [--set ppo.lr=1e-4 ...] [--fake] [--resume | --init-from CHECKPOINT] [--train-games exe5,exe6]
        [--eval-games exe4]

TensorBoard logs go to RUN_DIR/tb, checkpoints to RUN_DIR/checkpoints (each with the config, the git
commit and the spec), league snapshots to RUN_DIR/snapshots. --resume continues from the latest checkpoint
with its config (--set and --size still apply on top). --init-from starts a new run (update 0, a fresh
optimizer, league and curriculum) whose network is another run's checkpoint's (`model.warm_start`: its input
normalization too, and the GRU's previous-action columns zero where the checkpoint's network had none, so it
plays as that network did), with that checkpoint's model widths and the rest of the config as a new run's.

Collection and the update overlap. A collector thread owns the pool, the league, the curriculum and the
behavior network (a copy of the learner's weights) and plays rollout N+1 while the learner, the calling
thread, updates on rollout N. Two rollouts are kept and handed back and forth: a full one to the learner, and
with each free one the learner's weights after its update, which the collector plays the next rollout with. So
rollout k is always played by the weights after update k − 1 (the policy lag is one update; the first rollout
of a run is the learner's own), however the threads are timed, and a run is the same from run to run.
Evaluation runs on a thread and pools of its own; checkpoints are written by the learner, from the league and
curriculum as the collector last handed them over.
"""

from __future__ import annotations

import argparse
import json
import queue
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch.utils.tensorboard import SummaryWriter

from . import checkpoint
from .config import LEVELS, Config, config_to_dict, load_config
from .curriculum import Curriculum
from .device import Lane, pick_device, pin_torch_threads, synchronize
from .envs import make_pool, pool_spec
from .evaluate import Player, evaluate_vs_bots, summarize
from .league import League
from .model import TYPICAL, Model, budget_report, count_macs, count_params, warm_start
from .ppo import RunningNorm, compute_gae, ppo_update
from .rollout import CollectStats, Collector, Rollout, Stopped
from .spec import parse_spec


def lr_at(cfg: Config, update: int) -> float:
    frac = min(1.0, update / max(1, cfg.run.updates))
    return cfg.ppo.lr * (1.0 - (1.0 - cfg.ppo.lr_final) * frac)


def hp_weight_at(cfg: Config, update: int) -> float:
    r = cfg.reward
    frac = min(1.0, update / max(1, r.hp_anneal_updates))
    return r.hp_start + (r.hp_end - r.hp_start) * frac


@dataclass
class Collected:
    """A full rollout, collector → learner."""

    update: int  # the update that trains on it
    buffer: int  # which of the two rollouts
    stats: CollectStats
    behavior: int  # the version of the weights that played it (the update they came out of)
    mark: Any  # the collector's device work on it
    seconds: float  # the collector's time on it
    cpu_seconds: float  # the collector thread's CPU time on it
    wait_seconds: float  # the collector's wait for it to be free
    league: dict  # what the log shows of the league after it (League.summary)
    shared: dict | None  # the league's and curriculum's states, when a checkpoint follows this update


@dataclass
class Release:
    """A free rollout, learner → collector, with the weights to play the next one with (None: as before)."""

    buffer: int
    version: int | None
    weights: dict | None
    mark: Any  # the learner's device work so far (on the rollout, and copying the weights)


class Loop:
    """The collector thread and its two handoffs with the learner."""

    def __init__(self, tr: "Trainer", start: int, end: int):
        self.tr, self.start_update, self.end = tr, start, end
        self.full: queue.Queue = queue.Queue()
        self.free: queue.Queue = queue.Queue()
        self.stop = threading.Event()
        # The first two rollouts: both free, the first played by the learner's weights as they are now.
        weights = tr.weights()
        mark = tr.learn_lane.mark()
        self.free.put(Release(start % 2, start, weights, mark))
        self.free.put(Release((start + 1) % 2, None, None, mark))
        self.thread = threading.Thread(target=self._collect, name="collector", daemon=True)
        self.thread.start()

    def take(self) -> Collected:
        item = self.full.get()
        if isinstance(item, BaseException):
            raise RuntimeError("the collector thread failed") from item
        return item

    def release(self, buffer: int, version: int, weights: dict, mark) -> None:
        self.free.put(Release(buffer, version, weights, mark))

    def close(self) -> None:
        self.stop.set()
        self.free.put(None)
        self.thread.join()

    def _collect(self) -> None:
        tr, cfg = self.tr, self.tr.cfg
        try:
            tr.collect_lane.enter()
            pin_torch_threads(tr.collect_threads)
            prev: CollectStats | None = None
            for k in range(self.start_update, self.end):
                t0 = time.perf_counter()
                rel = self.free.get()
                if rel is None or self.stop.is_set():
                    return
                Lane.wait(rel.mark)
                t1 = time.perf_counter()
                c1 = time.thread_time()
                if rel.weights is not None:
                    tr.behave(rel.version, rel.weights)
                buffer = rel.buffer
                del rel
                if prev is not None:
                    tr.rounds_seen(prev.records, k)
                tr.league.refresh()
                tr.collector.begin(tr.rollouts[buffer])
                cs = tr.collector.collect(
                    cfg.ppo.steps, hp_weight_at(cfg, k), cfg.reward.result, self.stop, cfg.reward.judged_cost, cfg.reward.limit
                )
                mark = tr.collect_lane.mark()
                shared = tr.shared_state() if tr.checkpoint_due(k + 1) else None
                self.full.put(
                    Collected(
                        k, buffer, cs, tr.behavior_version, mark, time.perf_counter() - t1, time.thread_time() - c1, t1 - t0,
                        tr.league.summary(), shared,
                    )
                )
                prev = cs
        except Stopped:
            return
        except BaseException as e:  # handed to the learner, which raises it
            self.full.put(e)


class Evaluator:
    """Evaluation against the scripted players (cfg.eval) on a thread and pools of its own, so the loop doesn't
    wait for it. It plays a copy of the learner's weights at the update it started at, and its results join
    that update's row (and TensorBoard at that step) when they come. One runs at a time: starting the next
    waits for the last, which holds the learner only if an evaluation takes longer than eval.every updates."""

    def __init__(self, tr: "Trainer"):
        self.tr = tr
        self.thread: threading.Thread | None = None
        self.error: BaseException | None = None
        self.model: Model | None = None
        self.pools: dict[str, Any] = {}

    def start(self, update: int, weights: dict, mark, row: dict) -> None:
        self.finish()
        self.thread = threading.Thread(target=self._run, args=(update, weights, mark, row), name="evaluator", daemon=True)
        self.thread.start()

    def finish(self) -> None:
        if self.thread is not None:
            self.thread.join()
            self.thread = None
        if self.error is not None:
            e, self.error = self.error, None
            raise RuntimeError("evaluation failed") from e

    def _run(self, update: int, weights: dict, mark, row: dict) -> None:
        tr, cfg = self.tr, self.tr.cfg
        try:
            tr.eval_lane.enter()
            pin_torch_threads(1)  # a few dozen envs' inference
            Lane.wait(mark)
            if self.model is None:
                self.model = Model(tr.spec, cfg.model).to(tr.device)
                self.model.eval()
            self.model.load_state_dict(weights)
            tr.eval_lane.synchronize()  # copied before the learner's tensors are let go
            del weights
            records = evaluate_vs_bots(
                Player("learner", self.model), cfg.env, tr.fake, cfg.eval.games, cfg.eval.bots, cfg.eval.envs,
                cfg.eval.threads, cfg.eval.rounds, LEVELS[cfg.eval.level], cfg.eval.hp, tr.device, tr.spec.hash, self.pools,
                duplicate=True,  # each deal both ways: the builds' luck cancels (§9)
            )
            out = {}
            for (g, me, other), r in summarize(records, cfg.eval.games).items():
                if me == "learner" and r["rounds"]:
                    out[f"eval/{g}/{other}/winrate"] = (r["win"] + 0.5 * r["draw"]) / r["rounds"]
                    out[f"eval/{g}/{other}/rounds"] = r["rounds"]
            with tr.log_lock:
                row.update(out)
                if tr.writer is not None:
                    for k, v in out.items():
                        tr.writer.add_scalar(k, v, update)
                    tr.writer.flush()
                    print(f"eval @{update} " + "  ".join(f"{k[5:-8]} {v:.2f}" for k, v in sorted(out.items()) if k.endswith("/winrate")), flush=True)
        except BaseException as e:
            self.error = e


class Trainer:
    def __init__(
        self,
        cfg: Config,
        run_dir: str | Path,
        fake: bool,
        resume: dict[str, Any] | None = None,
        log: bool = True,
        init: dict[str, Any] | None = None,
    ):
        """resume: a checkpoint of this run to continue; init: another run's checkpoint whose network a new run
        starts from (`model.warm_start`), with "path" naming it."""
        self.cfg = cfg
        self.run_dir = Path(run_dir)
        self.run_dir.mkdir(parents=True, exist_ok=True)
        self.fake = fake
        self.device = pick_device(cfg.run.device)
        # torch's CPU threads, per thread: the collector's (the rollout's inference) and the learner's (the
        # update; this thread). They run at once.
        self.collect_threads = cfg.run.torch_threads
        self.update_threads = cfg.run.update_threads if cfg.run.update_threads > 0 else cfg.run.torch_threads
        pin_torch_threads(self.update_threads)
        self.collect_lane = Lane(self.device, priority=-1)
        self.learn_lane = Lane(self.device, default=True)
        self.eval_lane = Lane(self.device)
        torch.manual_seed(cfg.run.seed)
        self.rng = np.random.default_rng(cfg.run.seed)
        self.git = checkpoint.git_info()
        self.curriculum = Curriculum(cfg.curriculum, cfg.run.updates)
        if resume is not None:
            self.curriculum.load_state_dict(resume["curriculum"])
        self.pool = make_pool(cfg.env, fake, hp=self.curriculum.range(), holdout_mode="exclude")
        self.spec = parse_spec(pool_spec(self.pool))
        for what, state in (("resume", resume), ("init-from", init)):
            if state is not None and state["spec"]["hash"] != self.spec.hash:
                raise SystemExit(f"{what}: the checkpoint's spec hash {state['spec']['hash']} isn't the pool's {self.spec.hash}")
        self.model = Model(self.spec, cfg.model).to(self.device)
        self.opt = torch.optim.Adam(self.model.parameters(), lr=cfg.ppo.lr, eps=cfg.ppo.adam_eps)
        self.norm = RunningNorm(self.spec, self.device)
        E = cfg.env.envs
        self.update = 0
        self.decisions = 0
        self.fixture: dict[str, Any] | None = None
        # The checkpoint a warm-started run's network came from (provenance, kept in its checkpoints).
        self.init_from: dict[str, Any] | None = None
        if init is not None:
            warm_start(self.model, init["model"])
            self.norm.load_state_dict(init["norm"])
            self.init_from = {"path": str(init["path"]), "update": init["update"], "decisions": init["decisions"], "git": init["git"]}
        if resume is not None:
            self.model.load_state_dict(resume["model"])
            self.opt.load_state_dict(resume["optimizer"])
            self.norm.load_state_dict(resume["norm"])
            self.update = resume["update"]
            self.decisions = resume["decisions"]
            self.fixture = resume.get("fixture")
            self.init_from = resume.get("init_from")
        synchronize(self.device)
        # The collector's side, made on its lane: the league (its snapshots' networks), the behavior network,
        # the two rollouts.
        with self.collect_lane.scope():
            make_model = lambda: Model(self.spec, cfg.model).to(self.device)
            self.league = League(cfg.league, self.pool, E, self.run_dir, self.device, make_model, self.rng)
            if resume is not None:
                self.league.load_state_dict(resume["league"])
            self.behavior = make_model()
            self.behavior.load_state_dict(self.model.state_dict())
            self.behavior.eval()
            self.behavior_version = self.update
            self.rollouts = [
                Rollout(self.spec, 2 * E, cfg.ppo.steps, cfg.model.hidden, self.device, cfg.model.pointer_set) for _ in range(2)
            ]
            self.collector = Collector(
                self.pool, self.spec, self.behavior, self.league, self.rollouts[0], cfg.ppo.name_dropout, cfg.run.seed
            )
        synchronize(self.device)
        self.gen = torch.Generator()
        self.gen.manual_seed(cfg.run.seed + 1)
        self.writer = SummaryWriter(str(self.run_dir / "tb")) if log else None
        self.log_lock = threading.Lock()  # the writer and the history's rows (the evaluator adds to them)
        self.history: list[dict[str, Any]] = []
        self.evaluator = Evaluator(self)
        self.league.start()
        if resume is None and init is None:  # (a warm start keeps its network's normalization)
            self._warmup()
        (self.run_dir / "config.json").write_text(json.dumps(config_to_dict(cfg), indent=2) + "\n")

    def _warmup(self) -> None:
        """The input normalization's first statistics, from the untrained policy's observations."""
        with self.collect_lane.scope():
            self.collector.collect(self.cfg.ppo.norm_warmup_steps, 0.0, 0.0)
        synchronize(self.device)
        self.norm.update(self.collector.ro)
        self.norm.apply(self.model)
        with self.collect_lane.scope():
            self.collector.ro.clear()
        synchronize(self.device)

    # The collector's side (its thread) ----------------------------------------------------------------

    def behave(self, version: int, weights: dict) -> None:
        """Play with the learner's weights after update `version` from now on; a snapshot every
        league.snapshot_every of them."""
        self.behavior.load_state_dict(weights)
        self.collect_lane.synchronize()  # copied before the learner's tensors are let go
        self.behavior_version = version
        snaps = self.league.snapshots
        if version > 0 and version % self.cfg.league.snapshot_every == 0 and (not snaps or snaps[-1].update != version):
            self.league.add_snapshot(self.behavior, version)

    def rounds_seen(self, records: list, update: int) -> None:
        """A rollout's rounds to the curriculum, after update `update`; a new HP range from the next round."""
        self.curriculum.observe(records)
        if self.curriculum.advance(update):
            self.pool.set_hp(*self.curriculum.range())

    def shared_state(self) -> dict:
        return {"league": self.league.state_dict(), "curriculum": self.curriculum.state_dict()}

    def checkpoint_due(self, update: int) -> bool:
        return update % self.cfg.run.checkpoint_every == 0 or update == self.cfg.run.updates

    # The learner's side (the calling thread) ----------------------------------------------------------

    def weights(self) -> dict:
        """A copy of the learner's weights for another thread (on the learner's lane: mark it after)."""
        return {k: v.detach().clone() for k, v in self.model.state_dict().items()}

    def train(self, updates: int | None = None) -> list[dict[str, Any]]:
        cfg = self.cfg
        end = cfg.run.updates if updates is None else min(cfg.run.updates, self.update + updates)
        if self.update >= end:
            return self.history
        self.learn_lane.enter()
        pin_torch_threads(self.update_threads)
        loop = Loop(self, self.update, end)
        last = None
        try:
            t_prev = time.perf_counter()
            while self.update < end:
                t0 = time.perf_counter()
                item = loop.take()
                t1 = time.perf_counter()
                row = self._learn(item, loop, t1 - t0)
                t2 = time.perf_counter()
                row["time/wall"] = t2 - t_prev
                row["sps_loop"] = (item.stats.learner_decisions + item.stats.opponent_decisions) / max(t2 - t_prev, 1e-9)
                t_prev = t2
                self._after(row, item)
                last = item
        finally:
            loop.close()
            self.evaluator.finish()
        self.rounds_seen(last.stats.records, self.update)
        return self.history

    def _learn(self, item: Collected, loop: Loop, waited: float) -> dict[str, Any]:
        cfg = self.cfg
        u = self.update
        if item.update != u:
            raise RuntimeError(f"the collector sent rollout {item.update} for update {u}")
        t0, c0 = time.perf_counter(), time.thread_time()
        self.learn_lane.follow(item.mark)
        ro = self.rollouts[item.buffer]
        lr = lr_at(cfg, u)
        for g in self.opt.param_groups:
            g["lr"] = lr
        adv, ret = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, cfg.ppo.gamma, cfg.ppo.lam)
        us = ppo_update(self.model, self.opt, ro, adv, ret, cfg.ppo, self.gen)
        self.norm.update(ro)
        self.norm.apply(self.model)
        self.update = u + 1
        self.decisions += item.stats.learner_decisions
        if self.checkpoint_due(self.update):
            self.fixture = self._capture_fixture(ro)
        weights = self.weights()
        mark = self.learn_lane.mark()
        loop.release(item.buffer, self.update, weights, mark)  # the collector may play the next rollout now
        row = self._log(item, us, lr, hp_weight_at(cfg, u))
        row["time/update"] = time.perf_counter() - t0
        row["time/update_cpu"] = time.thread_time() - c0
        row["time/learner_wait"] = waited
        if cfg.eval.every > 0 and cfg.eval.games and self.update % cfg.eval.every == 0:
            self.evaluator.start(self.update, weights, mark, row)
        return row

    def _after(self, row: dict[str, Any], item: Collected) -> None:
        with self.log_lock:
            self.history.append(row)
            if self.writer is not None:
                for k, v in row.items():
                    if isinstance(v, (int, float)) and k != "update":
                        self.writer.add_scalar(k, v, row["update"])
                self.writer.flush()
                us = {k[4:]: v for k, v in row.items() if k.startswith("ppo/")}
                print(
                    f"update {row['update']:6d}  decisions {row['decisions']:,}  reward/dec {row['reward_per_decision']:+.4f}  "
                    f"entropy {us.get('entropy', 0):.3f}  kl {us.get('approx_kl', 0):.4f}  ev {us.get('explained_variance', 0):+.2f}  "
                    f"{row['sps_loop']:,.0f} dec/s  collect {row['time/collect']:.1f}s update {row['time/update']:.1f}s  "
                    f"lag {row['loop/policy_lag']}  hp {row['hp_level']}"
                    + (f"  taught {us['teacher_labels']:,.0f} acc {us['teacher_acc']:.2f}" if us.get("teacher_labels") else ""),
                    flush=True,
                )
        if self.checkpoint_due(self.update):
            self.save(item.shared)

    def _capture_fixture(self, ro: Rollout) -> dict[str, Any] | None:
        """A run of one seat's decisions from this rollout, for export's parity fixture."""
        n = self.cfg.run.fixture_decisions
        closed = ro.closed.sum(1)
        s = int(closed.argmax())
        k = min(n, int(closed[s]))
        if k == 0:
            return None
        t = torch.arange(k)
        obs = ro.obs(torch.full_like(t, s), t, dense=True)
        cpu = lambda v: v.detach().cpu().clone()
        return {
            "x": {n_: cpu(v) for n_, v in obs["x"].items()},
            "ids": {n_: cpu(v) for n_, v in obs["ids"].items()},
            "mask": {n_: cpu(v) for n_, v in obs["mask"].items()},
            "legal_fight": ro.legal_fight[s, :k].clone(),
            "legal_screen": ro.legal_screen[s, :k].clone(),
            "phase": ro.phase[s, :k].clone(),
            "reset": ro.ep_start[s, :k].clone(),
            "previous": ro.previous[s, :k].clone(),
            "h0": cpu(ro.h_in[s, 0]),
        }

    def _log(self, item: Collected, us: dict, lr: float, hp_w: float) -> dict[str, Any]:
        cs = item.stats
        row: dict[str, Any] = {"update": self.update, "decisions": self.decisions}
        n = max(1, cs.learner_decisions)
        row["reward_per_decision"] = cs.reward_sum / n
        row["hp_gain_per_decision"] = cs.hp_gain_sum / n
        row["learner_decisions"] = cs.learner_decisions
        row["opponent_decisions"] = cs.opponent_decisions
        row["rounds"] = cs.rounds
        row["truncated"] = cs.truncated
        row["judged"] = cs.judged
        row["limited"] = cs.limited
        row["sps_decisions"] = (cs.learner_decisions + cs.opponent_decisions) / max(item.seconds, 1e-9)
        row["time/collect"] = item.seconds
        row["time/collect_cpu"] = item.cpu_seconds
        row["time/step"] = cs.step_seconds
        row["time/collector_wait"] = item.wait_seconds
        row["loop/policy_lag"] = self.update - 1 - item.behavior
        row["lr"] = lr
        row["hp_weight"] = hp_w
        row["hp_level"] = self.curriculum.range()[1]
        row.update({f"ppo/{k}": v for k, v in us.items()})
        by_kind: dict[str, list[float]] = {}
        for kind, opp, sc, _g in cs.records:
            by_kind.setdefault(kind, []).append(sc)
            by_kind.setdefault(opp, []).append(sc)
        for k, v in by_kind.items():
            row[f"winrate/{k}"] = float(np.mean(v))
        for (kind, name), (tot, cnt) in cs.measures.items():
            row[f"behavior/{kind}/{name}"] = tot / max(cnt, 1)
        row.update(item.league)
        present = {k: v / n for k, v in cs.present.items()}
        if cs.learner_decisions:
            m = count_macs(self.spec, self.cfg.model, {k: round(v) for k, v in present.items()})
            row["model/macs_measured_fight"] = m["fight"]
            row["model/tokens_measured"] = sum(present.values())
        return row

    def state(self, shared: dict | None = None) -> dict[str, Any]:
        """The trainer's state; the league's and curriculum's from `shared` (as the collector handed them over)
        while the loop runs, else read here."""
        if shared is None:
            shared = self.shared_state()
        return {
            "update": self.update,
            "decisions": self.decisions,
            "config": config_to_dict(self.cfg),
            "git": self.git,
            "spec": self.spec.raw,
            "model": {k: v.detach().cpu() for k, v in self.model.state_dict().items()},
            "optimizer": self.opt.state_dict(),
            "norm": {k: {n: v.cpu() for n, v in d.items()} for k, d in self.norm.state_dict().items()},
            "league": shared["league"],
            "curriculum": shared["curriculum"],
            "fixture": self.fixture,
            "init_from": self.init_from,
            "params": count_params(self.model),
        }

    def save(self, shared: dict | None = None) -> Path:
        return checkpoint.save(self.run_dir, self.update, self.state(shared))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--config", default=None, help="a TOML file (see configs/)")
    ap.add_argument("--size", default=None, help="a named model size: small (default) or large")
    ap.add_argument("--set", action="append", default=[], metavar="KEY=VALUE", help="override a config value")
    ap.add_argument("--fake", action="store_true", help="train on FakePool instead of nettai_gym")
    ap.add_argument("--resume", action="store_true", help="continue the run's latest checkpoint")
    ap.add_argument(
        "--init-from", default=None, metavar="CHECKPOINT",
        help="start a new run whose network is this checkpoint's (a file, or a run directory's latest): update 0, "
        "a fresh optimizer, league and curriculum, the checkpoint's model widths and input normalization",
    )
    ap.add_argument("--train-games", default=None, help="comma-separated: the games trained on (env.games)")
    ap.add_argument("--eval-games", default=None, help="comma-separated: games evaluated every eval.every updates")
    ap.add_argument("--updates", type=int, default=None, help="stop after this many updates in this process")
    a = ap.parse_args()
    overrides = list(a.set)
    if a.train_games:
        overrides.append(f"env.games={json.dumps(a.train_games.split(','))}")
    if a.eval_games:
        overrides.append(f"eval.games={json.dumps(a.eval_games.split(','))}")
    if a.resume and a.init_from:
        raise SystemExit("--resume continues this run; --init-from starts a new one: not both")
    resume = init = None
    if a.resume:
        resume = checkpoint.load(a.run_dir)
        cfg = load_config(a.config, overrides, a.size, base=resume["config"])
    else:
        if (Path(a.run_dir) / "checkpoints" / "latest").exists():
            raise SystemExit(f"{a.run_dir} already has checkpoints: pass --resume, or another --run-dir")
        base = None
        if a.init_from:
            init = checkpoint.load(a.init_from)
            init["path"] = checkpoint.resolve(a.init_from)
            base = {"model": init["config"]["model"]}
        cfg = load_config(a.config, overrides, a.size, base=base)
    trainer = Trainer(cfg, a.run_dir, a.fake, resume, init=init)
    if init is not None:
        print(f"warm start from {init['path']} (update {init['update']}, {init['decisions']:,} decisions)")
    m = trainer.model
    print(f"device {trainer.device}, {count_params(m):,} parameters, spec {trainer.spec.hash[:12]}, git {trainer.git['commit'][:10]}{' (dirty)' if trainer.git['dirty'] else ''}")
    worst = count_macs(trainer.spec, cfg.model, None)
    print(f"MACs a decision, every token present: {max(worst['fight'], worst['screen']) / 1e6:.2f} M", end="")
    if set(TYPICAL) == set(trainer.spec.names):
        print(f"; typical fight position: {budget_report(trainer.spec, cfg.model)['typical_fight'] / 1e6:.2f} M")
    else:
        print(" (the typical profile names other sets; model/macs_measured_fight logs the measured cost)")
    trainer.train(a.updates)
    trainer.save()


if __name__ == "__main__":
    main()
