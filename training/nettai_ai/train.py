"""Training (§8): recurrent PPO by self-play in a league, on the pool's envs.

    uv run python -m nettai_ai.train --run-dir runs/NAME [--config configs/x.toml] [--size large]
        [--set ppo.lr=1e-4 ...] [--fake] [--resume] [--train-games exe5,exe6] [--eval-games exe4]

TensorBoard logs go to RUN_DIR/tb, checkpoints to RUN_DIR/checkpoints (each with the config, the git
commit and the spec), league snapshots to RUN_DIR/snapshots. --resume continues from the latest checkpoint
with its config (--set and --size still apply on top).
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch.utils.tensorboard import SummaryWriter

from . import checkpoint
from .config import LEVELS, Config, config_to_dict, load_config
from .curriculum import Curriculum
from .envs import make_pool, pool_spec
from .evaluate import Player, evaluate_vs_bots, summarize
from .league import League
from .model import TYPICAL, Model, budget_report, count_macs, count_params
from .ppo import RunningNorm, compute_gae, ppo_update
from .rollout import Collector, Rollout
from .spec import parse_spec


def pick_device(name: str) -> torch.device:
    if name == "auto":
        if torch.cuda.is_available():
            return torch.device("cuda")
        if torch.backends.mps.is_available():
            return torch.device("mps")
        return torch.device("cpu")
    return torch.device(name)


def lr_at(cfg: Config, update: int) -> float:
    frac = min(1.0, update / max(1, cfg.run.updates))
    return cfg.ppo.lr * (1.0 - (1.0 - cfg.ppo.lr_final) * frac)


def hp_weight_at(cfg: Config, update: int) -> float:
    r = cfg.reward
    frac = min(1.0, update / max(1, r.hp_anneal_updates))
    return r.hp_start + (r.hp_end - r.hp_start) * frac


class Trainer:
    def __init__(self, cfg: Config, run_dir: str | Path, fake: bool, resume: dict[str, Any] | None = None, log: bool = True):
        self.cfg = cfg
        self.run_dir = Path(run_dir)
        self.run_dir.mkdir(parents=True, exist_ok=True)
        self.fake = fake
        self.device = pick_device(cfg.run.device)
        if cfg.run.torch_threads > 0:
            torch.set_num_threads(cfg.run.torch_threads)
        self.collect_threads = torch.get_num_threads()
        self.update_threads = cfg.run.update_threads if cfg.run.update_threads > 0 else self.collect_threads
        torch.manual_seed(cfg.run.seed)
        self.rng = np.random.default_rng(cfg.run.seed)
        self.git = checkpoint.git_info()
        self.curriculum = Curriculum(cfg.curriculum, cfg.run.updates)
        if resume is not None:
            self.curriculum.load_state_dict(resume["curriculum"])
        self.pool = make_pool(cfg.env, fake, hp=self.curriculum.range(), holdout_mode="exclude")
        self.spec = parse_spec(pool_spec(self.pool))
        if resume is not None and resume["spec"]["hash"] != self.spec.hash:
            raise SystemExit(f"resume: the checkpoint's spec hash {resume['spec']['hash']} isn't the pool's {self.spec.hash}")
        self.model = Model(self.spec, cfg.model).to(self.device)
        self.opt = torch.optim.Adam(self.model.parameters(), lr=cfg.ppo.lr, eps=cfg.ppo.adam_eps)
        self.norm = RunningNorm(self.spec, self.device)
        E = cfg.env.envs
        self.league = League(
            cfg.league, self.pool, E, self.run_dir, self.device, lambda: Model(self.spec, cfg.model).to(self.device), self.rng
        )
        self.rollout = Rollout(self.spec, 2 * E, cfg.ppo.steps, cfg.model.hidden, self.device)
        self.collector = Collector(
            self.pool, self.spec, self.model, self.league, self.rollout, cfg.ppo.name_dropout, cfg.run.seed
        )
        self.gen = torch.Generator()
        self.gen.manual_seed(cfg.run.seed + 1)
        self.update = 0
        self.decisions = 0
        self.fixture: dict[str, Any] | None = None
        if resume is not None:
            self.model.load_state_dict(resume["model"])
            self.opt.load_state_dict(resume["optimizer"])
            self.norm.load_state_dict(resume["norm"])
            self.league.load_state_dict(resume["league"])
            self.update = resume["update"]
            self.decisions = resume["decisions"]
            self.fixture = resume.get("fixture")
        self.eval_pools: dict[str, Any] = {}
        self.writer = SummaryWriter(str(self.run_dir / "tb")) if log else None
        self.history: list[dict[str, Any]] = []
        self.league.start()
        if resume is None:
            self._warmup()
        (self.run_dir / "config.json").write_text(json.dumps(config_to_dict(cfg), indent=2) + "\n")

    def _warmup(self) -> None:
        """The input normalization's first statistics, from the untrained policy's observations."""
        self.collector.collect(self.cfg.ppo.norm_warmup_steps, 0.0, 0.0)
        self.norm.update(self.rollout)
        self.norm.apply(self.model)
        self.rollout.clear()

    def train(self, updates: int | None = None) -> list[dict[str, Any]]:
        cfg = self.cfg
        end = cfg.run.updates if updates is None else min(cfg.run.updates, self.update + updates)
        while self.update < end:
            u = self.update
            lr = lr_at(cfg, u)
            for g in self.opt.param_groups:
                g["lr"] = lr
            hp_w = hp_weight_at(cfg, u)
            self.league.refresh()
            t0 = time.perf_counter()
            cs = self.collector.collect(cfg.ppo.steps, hp_w, cfg.reward.result)
            t1 = time.perf_counter()
            ro = self.rollout
            adv, ret = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, cfg.ppo.gamma, cfg.ppo.lam)
            torch.set_num_threads(self.update_threads)
            us = ppo_update(self.model, self.opt, ro, adv, ret, cfg.ppo, self.gen)
            torch.set_num_threads(self.collect_threads)
            t2 = time.perf_counter()
            self.norm.update(ro)
            self.norm.apply(self.model)
            self.update = u + 1
            self.decisions += cs.learner_decisions
            due_ckpt = self.update % cfg.run.checkpoint_every == 0 or self.update == cfg.run.updates
            if due_ckpt:
                self.fixture = self._capture_fixture()
            ro.carry()
            if self.update % cfg.league.snapshot_every == 0:
                self.league.add_snapshot(self.model, self.update)
            self.curriculum.observe(cs.records)
            if self.curriculum.advance(self.update):
                self.pool.set_hp(*self.curriculum.range())
            row = self._log(cs, us, lr, hp_w, t1 - t0, t2 - t1)
            if cfg.eval.every > 0 and cfg.eval.games and self.update % cfg.eval.every == 0:
                row.update(self._evaluate())
            self.history.append(row)
            if due_ckpt:
                self.save()
        return self.history

    def _capture_fixture(self) -> dict[str, Any] | None:
        """A run of one seat's decisions from this rollout, for export's parity fixture."""
        ro = self.rollout
        n = self.cfg.run.fixture_decisions
        closed = ro.closed.sum(1)
        s = int(closed.argmax())
        k = min(n, int(closed[s]))
        if k == 0:
            return None
        t = torch.arange(k, device=ro.device)
        sv = torch.full_like(t, s)
        obs = ro.gather(sv, t)
        cpu = lambda v: v.detach().cpu().clone()
        return {
            "x": {n_: cpu(v) for n_, v in obs["x"].items()},
            "ids": {n_: cpu(v) for n_, v in obs["ids"].items()},
            "mask": {n_: cpu(v) for n_, v in obs["mask"].items()},
            "legal_fight": cpu(ro.legal_fight[sv, t]),
            "legal_screen": cpu(ro.legal_screen[sv, t]),
            "phase": cpu(ro.phase[sv, t]),
            "reset": cpu(ro.ep_start[sv, t]),
            "h0": cpu(ro.h_in[s, 0]),
        }

    def _log(self, cs, us, lr, hp_w, t_collect, t_update) -> dict[str, Any]:
        row: dict[str, Any] = {"update": self.update, "decisions": self.decisions}
        n = max(1, cs.learner_decisions)
        row["reward_per_decision"] = cs.reward_sum / n
        row["hp_gain_per_decision"] = cs.hp_gain_sum / n
        row["learner_decisions"] = cs.learner_decisions
        row["opponent_decisions"] = cs.opponent_decisions
        row["rounds"] = cs.rounds
        row["truncated"] = cs.truncated
        row["sps_decisions"] = (cs.learner_decisions + cs.opponent_decisions) / max(t_collect, 1e-9)
        row["time_collect"] = t_collect
        row["time_update"] = t_update
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
        row["elo/learner"] = self.league.ratings["learner"]
        for b in self.cfg.league.bot_names:
            wr, cnt = self.league.bot_winrate(b)
            if cnt:
                row[f"league/winrate_{b}"] = wr
        row["league/snapshots"] = len(self.league.snapshots)
        present = {k: v / n for k, v in cs.present.items()}
        if cs.learner_decisions:
            m = count_macs(self.spec, self.cfg.model, {k: round(v) for k, v in present.items()})
            row["model/macs_measured_fight"] = m["fight"]
            row["model/tokens_measured"] = sum(present.values())
        if self.writer is not None:
            for k, v in row.items():
                if isinstance(v, (int, float)) and k != "update":
                    self.writer.add_scalar(k, v, self.update)
            self.writer.flush()
            print(
                f"update {self.update:6d}  decisions {self.decisions:,}  reward/dec {row['reward_per_decision']:+.4f}  "
                f"entropy {us.get('entropy', 0):.3f}  kl {us.get('approx_kl', 0):.4f}  ev {us.get('explained_variance', 0):+.2f}  "
                f"{row['sps_decisions']:,.0f} dec/s  collect {t_collect:.1f}s update {t_update:.1f}s  hp {row['hp_level']}",
                flush=True,
            )
        return row

    def _evaluate(self) -> dict[str, Any]:
        """Periodic evaluation against the scripted players on cfg.eval.games (where held-out games go)."""
        cfg = self.cfg
        self.model.eval()
        player = Player("learner", self.model)
        records = evaluate_vs_bots(
            player, cfg.env, self.fake, cfg.eval.games, cfg.eval.bots, cfg.eval.envs, cfg.eval.threads,
            cfg.eval.rounds, LEVELS[cfg.eval.level], cfg.eval.hp, self.device, self.spec.hash, self.eval_pools,
        )
        summary = summarize(records, cfg.eval.games)
        out = {}
        for (g, me, other), row in summary.items():
            if me == "learner" and row["rounds"]:
                out[f"eval/{g}/{other}/winrate"] = (row["win"] + 0.5 * row["draw"]) / row["rounds"]
                out[f"eval/{g}/{other}/rounds"] = row["rounds"]
        print("eval " + "  ".join(f"{k[5:-8]} {v:.2f}" for k, v in sorted(out.items()) if k.endswith("/winrate")), flush=True)
        if self.writer is not None:
            for k, v in out.items():
                self.writer.add_scalar(k, v, self.update)
        return out

    def state(self) -> dict[str, Any]:
        return {
            "update": self.update,
            "decisions": self.decisions,
            "config": config_to_dict(self.cfg),
            "git": self.git,
            "spec": self.spec.raw,
            "model": {k: v.detach().cpu() for k, v in self.model.state_dict().items()},
            "optimizer": self.opt.state_dict(),
            "norm": {k: {n: v.cpu() for n, v in d.items()} for k, d in self.norm.state_dict().items()},
            "league": self.league.state_dict(),
            "curriculum": self.curriculum.state_dict(),
            "fixture": self.fixture,
            "params": count_params(self.model),
        }

    def save(self) -> Path:
        return checkpoint.save(self.run_dir, self.update, self.state())


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--run-dir", required=True)
    ap.add_argument("--config", default=None, help="a TOML file (see configs/)")
    ap.add_argument("--size", default=None, help="a named model size: small (default) or large")
    ap.add_argument("--set", action="append", default=[], metavar="KEY=VALUE", help="override a config value")
    ap.add_argument("--fake", action="store_true", help="train on FakePool instead of nettai_gym")
    ap.add_argument("--resume", action="store_true", help="continue the run's latest checkpoint")
    ap.add_argument("--train-games", default=None, help="comma-separated: the games trained on (env.games)")
    ap.add_argument("--eval-games", default=None, help="comma-separated: games evaluated every eval.every updates")
    ap.add_argument("--updates", type=int, default=None, help="stop after this many updates in this process")
    a = ap.parse_args()
    overrides = list(a.set)
    if a.train_games:
        overrides.append(f"env.games={json.dumps(a.train_games.split(','))}")
    if a.eval_games:
        overrides.append(f"eval.games={json.dumps(a.eval_games.split(','))}")
    resume = None
    if a.resume:
        resume = checkpoint.load(a.run_dir)
        cfg = load_config(a.config, overrides, a.size, base=resume["config"])
    else:
        if (Path(a.run_dir) / "checkpoints" / "latest").exists():
            raise SystemExit(f"{a.run_dir} already has checkpoints: pass --resume, or another --run-dir")
        cfg = load_config(a.config, overrides, a.size)
    trainer = Trainer(cfg, a.run_dir, a.fake, resume)
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
