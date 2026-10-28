"""The teacher's lesson (§8, env.teacher_rate and ppo.teacher) through the whole pipeline: FakePool labels a share
of its fight decisions with the toy's target, and the cross-entropy toward the labels teaches the target faster
than PPO's own signal."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import torch

from nettai_ai.config import load_config
from nettai_ai.train import Trainer

UPDATES = 12


def fight_rate(row) -> float:
    n = row.get("behavior/bot/toy_fight_decisions", 0)
    return row.get("behavior/bot/toy_fight_correct", 0) / n if n else float("nan")


def run(tmp: Path, teacher: float) -> list[dict]:
    torch.set_num_threads(min(8, torch.get_num_threads()))
    over = [
        'env.games=["exe6"]', "env.envs=32", "env.threads=1", "env.teacher_rate=0.3",
        "ppo.steps=32", "ppo.chunk=16", "ppo.epochs=4", "ppo.minibatch_chunks=8", "ppo.lr=5e-4",
        "ppo.norm_warmup_steps=8", "ppo.entropy=0.003", "ppo.gamma=0.9", "ppo.lam=0.9", f"ppo.teacher={teacher}",
        "reward.result=0.0",
        "league.self_play=0.0", "league.snapshots=0.0", "league.bots=1.0", 'league.bot_names=["still"]',
        "model.d=32", "model.ffn=64", "model.token_hidden=32", "model.summary=64", "model.hidden=64",
        "model.head_hidden=64", "model.pointer_hidden=32",
        f"run.updates={UPDATES}", "run.checkpoint_every=1000", "run.device='cpu'",
        "curriculum.start=1000", "curriculum.band=0",
    ]
    return Trainer(load_config(None, over), tmp / f"run{teacher}", fake=True, log=False).train()


def test_the_teachers_labels_teach_the_target(tmp_path: Path):
    plain, taught = run(tmp_path, 0.0), run(tmp_path, 1.0)
    p, t = (np.nanmean([fight_rate(r) for r in rows[-3:]]) for rows in (plain, taught))
    labels = taught[-1]["ppo/teacher_labels"]
    print(f"the target pressed in the last 3 updates: {p:.2f} without the teacher, {t:.2f} with; {labels:.0f} labels an update, "
          f"accuracy {taught[0]['ppo/teacher_acc']:.2f} → {taught[-1]['ppo/teacher_acc']:.2f}")
    assert labels > 0
    assert plain[-1]["ppo/teacher_labels"] > 0, "labels are counted whatever the weight"
    assert t > p + 0.15, (p, t)
    assert taught[-1]["ppo/teacher_acc"] > taught[0]["ppo/teacher_acc"] + 0.2
