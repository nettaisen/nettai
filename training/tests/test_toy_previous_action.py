"""The network learns a rule it can play only knowing what it just pressed (ComboPool: a finisher that must match
the opener it was made to press, which its view shows two decisions late), with its previous action as the GRU's
input (§7), and, as a check of the toy, not without it (the previous action hidden: always "none"): then it learns
to press one of the finishers, right 1 time in 4."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import torch

import nettai_ai.envs as envs
from nettai_ai.config import load_config
from nettai_ai.fake import COMBO, ComboPool
from nettai_ai.model import Model
from nettai_ai.train import Trainer

# Measured over 100 updates (rates over 5 updates): with its previous action 0.06, 0.11, 0.23, 0.46, 0.82, 0.94, 0.98
# at 50 and 0.97 at 100; without, 0.06 to 0.25 (one of the 4 finishers) by 50, and 0.25 to the end.
UPDATES = 60


def rate(row):
    n = row.get("behavior/bot/toy_combo_decisions", 0)
    return row.get("behavior/bot/toy_combo_correct", 0) / n if n else float("nan")


def run(tmp_path: Path, name: str, monkeypatch) -> tuple[float, float]:
    monkeypatch.setattr(envs, "FakePool", ComboPool)  # the trainer's --fake pool
    over = [
        'env.games=["exe6"]', "env.envs=32", "env.threads=1",
        "ppo.steps=32", "ppo.chunk=16", "ppo.epochs=4", "ppo.minibatch_chunks=8", "ppo.lr=5e-4",
        "ppo.norm_warmup_steps=8", "ppo.entropy=0.003", "ppo.gamma=0.9", "ppo.lam=0.9",
        # (The toy's hits land at once and it has no rows: holding decisions for credits to come, and the auxiliary
        # predictions, only slow it.)
        "ppo.defer=0", "ppo.aux_coef=0.0",
        "reward.result=0.0",
        "league.self_play=0.0", "league.snapshots=0.0", "league.bots=1.0", 'league.bot_names=["still"]',
        "model.d=32", "model.ffn=64", "model.token_hidden=32", "model.summary=64", "model.hidden=64",
        "model.head_hidden=64", "model.pointer_hidden=32",
        f"run.updates={UPDATES}", "run.checkpoint_every=1000", "run.device='cpu'",
        "curriculum.start=1000", "curriculum.band=0",
    ]
    tr = Trainer(load_config(None, over), tmp_path / name, fake=True, log=False)
    rows = tr.train()
    first = np.nanmean([rate(r) for r in rows[:4]])
    last = np.nanmean([rate(r) for r in rows[-4:]])
    print(f"{name}: finishers right {first:.2f} → {last:.2f}")
    return first, last


def test_the_network_learns_from_its_previous_action(tmp_path: Path, monkeypatch):
    torch.set_num_threads(min(8, torch.get_num_threads()))
    first, last = run(tmp_path, "with", monkeypatch)
    assert first < 0.15 and last > 0.6, (first, last)


def test_without_its_previous_action_it_cannot(tmp_path: Path, monkeypatch):
    torch.set_num_threads(min(8, torch.get_num_threads()))
    gru_input = Model.gru_input
    monkeypatch.setattr(Model, "gru_input", lambda self, z, previous: gru_input(self, z, torch.full_like(previous, self.no_previous)))
    first, last = run(tmp_path, "without", monkeypatch)
    assert last < 1.0 / COMBO + 0.08, (first, last)
