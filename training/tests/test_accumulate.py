"""A minibatch taken in parts (ppo.accumulate) is the same step as in one pass: from the same network, optimizer and
rollout, the update leaves the same weights and reports the same statistics."""

from __future__ import annotations

import copy
from pathlib import Path

import torch

from nettai_ai.config import load_config
from nettai_ai.ppo import compute_gae, ppo_update
from nettai_ai.train import Trainer

from conftest import TINY_MODEL

OVERRIDES = [
    'env.games=["exe6"]', "env.envs=8", "env.threads=1", "ppo.steps=16", "ppo.chunk=8", "ppo.epochs=2",
    "ppo.minibatch_chunks=4", "ppo.norm_warmup_steps=4", "run.checkpoint_every=1000", "run.device='cpu'",
] + [f"model.{k}={v}" for k, v in TINY_MODEL.items()]


def test_a_minibatch_in_parts_is_the_same_step(tmp_path: Path):
    tr = Trainer(load_config(None, OVERRIDES), tmp_path / "run", fake=True, log=False)
    with tr.collect_lane.scope():
        tr.collector.collect(tr.cfg.ppo.steps, 1.0, 1.0)
    ro = tr.collector.ro
    adv, ret = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, tr.cfg.ppo.gamma, tr.cfg.ppo.lam)
    results = []
    for parts in (1, 2, 3):
        model = copy.deepcopy(tr.model)
        opt = torch.optim.Adam(model.parameters(), lr=tr.cfg.ppo.lr, eps=tr.cfg.ppo.adam_eps)
        opt.load_state_dict(tr.opt.state_dict())
        cfg = copy.deepcopy(tr.cfg.ppo)
        cfg.accumulate = parts
        gen = torch.Generator()
        gen.manual_seed(5)
        stats = ppo_update(model, opt, ro, adv, ret, cfg, gen)
        results.append((parts, {k: v.detach().clone() for k, v in model.state_dict().items()}, stats))
    (_, base, base_stats), *rest = results
    assert base_stats["minibatches"] > 1
    for parts, weights, stats in rest:
        for k, v in base.items():
            assert torch.allclose(weights[k], v, atol=2e-5, rtol=1e-4), (parts, k)
        for k in ("loss_policy", "loss_value", "entropy", "approx_kl", "clip_frac"):
            assert abs(stats[k] - base_stats[k]) < 1e-4 * max(1.0, abs(base_stats[k])), (parts, k, stats[k], base_stats[k])
