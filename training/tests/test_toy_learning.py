"""PPO learns FakePool's toy rule through the whole pipeline (§6.6 pool → encoder → transformer → pool →
GRU → heads → recurrent PPO → league), in a short CPU run.

The fight's target action is a one-hot in the global token; the screen's best slot is the one whose first
float is highest. A random policy presses the target about 7% of the time and picks the best slot about 10%.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
import torch

from nettai_ai.config import load_config
from nettai_ai.train import Trainer

UPDATES = 40


def rate(row, what):
    n = row.get(f"behavior/bot/toy_{what}_decisions", 0)
    return row.get(f"behavior/bot/toy_{what}_correct", 0) / n if n else float("nan")


def test_ppo_learns_the_toy_rule(tmp_path: Path):
    torch.set_num_threads(min(8, torch.get_num_threads()))
    over = [
        'env.games=["exe6"]', "env.envs=32", "env.threads=1",
        "ppo.steps=32", "ppo.chunk=16", "ppo.epochs=4", "ppo.minibatch_chunks=8", "ppo.lr=5e-4",
        "ppo.norm_warmup_steps=8", "ppo.entropy=0.003", "ppo.gamma=0.9", "ppo.lam=0.9",
        # (The toy's hits land at once and it has no rows: holding decisions for credits to come, and the auxiliary
        # predictions, only slow it.)
        "ppo.defer=0", "ppo.aux_coef=0.0",
        # The toy's ±1 result is noise to it (rounds end at random); the HP shaping carries its signal.
        "reward.result=0.0",
        "league.self_play=0.0", "league.snapshots=0.0", "league.bots=1.0", 'league.bot_names=["still"]',
        "model.d=32", "model.ffn=64", "model.token_hidden=32", "model.summary=64", "model.hidden=64",
        "model.head_hidden=64", "model.pointer_hidden=32",
        f"run.updates={UPDATES}", "run.checkpoint_every=1000", "run.device='cpu'",
        "curriculum.start=1000", "curriculum.band=0",
    ]
    tr = Trainer(load_config(None, over), tmp_path / "run", fake=True, log=False)
    rows = tr.train()
    first, last = rows[:4], rows[-4:]
    fight0 = np.nanmean([rate(r, "fight") for r in first])
    fight1 = np.nanmean([rate(r, "fight") for r in last])
    screen0 = np.nanmean([rate(r, "screen") for r in first])
    screen1 = np.nanmean([rate(r, "screen") for r in last])
    reward0 = np.mean([r["reward_per_decision"] for r in first])
    reward1 = np.mean([r["reward_per_decision"] for r in last])
    print(f"fight {fight0:.2f} → {fight1:.2f}, screen {screen0:.2f} → {screen1:.2f}, reward/decision {reward0:.4f} → {reward1:.4f}")
    # (Over seeds 1 to 4 the fight's targets come to 0.24 to 0.33 by update 40, 0.24 to 0.30 since the GRU reads the
    # previous action, which every random draw after it moved: 0.28 at seed 1. Three times the random policy's.)
    assert fight0 < 0.12 and fight1 > 0.21
    assert screen1 > screen0 + 0.1
    assert reward1 > 3 * reward0
