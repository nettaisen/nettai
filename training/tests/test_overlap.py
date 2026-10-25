"""The overlapped loop (train.py): each rollout is played by the weights it says (the update before; its stored
log-probabilities and GRU states are that network's), the lag is one update, and a run is the same from run to
run however the threads are timed. And the compact storage is the same input to the network."""

from __future__ import annotations

from pathlib import Path

import torch

from nettai_ai.config import load_config
from nettai_ai.envs import PoolView
from nettai_ai.fake import FakePool
from nettai_ai.model import Model, mask_logits, previous_action
from nettai_ai.rollout import StepBatch, tokens_to_obs
from nettai_ai.spec import parse_spec
from nettai_ai.train import Trainer

from conftest import TINY_MODEL

OVERRIDES = [
    'env.games=["exe6"]', "env.envs=8", "env.threads=1", "ppo.steps=16", "ppo.chunk=8", "ppo.epochs=2",
    "ppo.minibatch_chunks=4", "ppo.norm_warmup_steps=4", "run.updates=6", "run.checkpoint_every=1000",
    "run.device='cpu'", "league.snapshot_every=2", "league.self_play=0.5", "league.snapshots=0.3", "league.bots=0.2",
] + [f"model.{k}={v}" for k, v in TINY_MODEL.items()]


class Recording(Trainer):
    """Keeps every version of the weights it hands out, and replays each rollout's decisions with the version
    that played them before training on it."""

    def __init__(self, *a, **kw):
        self.versions: dict[int, dict] = {}
        self.checked: list[tuple[int, int, int]] = []  # (update, behavior version, decisions replayed)
        self.carried = None  # (streams with an open decision at the last rollout's end, the version that played it)
        super().__init__(*a, **kw)

    def weights(self) -> dict:
        w = super().weights()
        self.versions[self.update] = {k: v.clone() for k, v in w.items()}
        return w

    def _learn(self, item, loop, waited):
        self.checked.append((item.update, item.behavior, self._replay(item)))
        return super()._learn(item, loop, waited)

    @torch.no_grad()
    def _replay(self, item) -> int:
        ro = self.rollouts[item.buffer]
        nets = {}
        for v in {item.behavior} | ({self.carried[1]} if self.carried else set()):
            nets[v] = Model(self.spec, self.cfg.model)
            nets[v].load_state_dict(self.versions[v])
        checked = 0
        for s in range(ro.S):
            n = int(ro.cursor[s])
            for t in range(n):
                # A stream's first decision may be the last rollout's open one, played by that rollout's version.
                carried = t == 0 and self.carried is not None and bool(self.carried[0][s])
                net = nets[self.carried[1] if carried else item.behavior]
                fight, screen, _, h = net.step(
                    ro.obs(torch.tensor([s]), torch.tensor([t])), ro.h_in[s, t].unsqueeze(0), ro.previous[s, t].unsqueeze(0)
                )
                if int(ro.phase[s, t]) == 1:
                    logits = mask_logits(fight, ro.legal_fight[s, t].unsqueeze(0))
                else:
                    logits = mask_logits(screen, ro.legal_screen[s, t].unsqueeze(0))
                logp = torch.log_softmax(logits, -1)[0, int(ro.action[s, t])]
                assert torch.allclose(logp, ro.logp[s, t], atol=1e-5), (item.update, s, t, float(logp), float(ro.logp[s, t]))
                if t + 1 < n and not bool(ro.ep_start[s, t + 1]):
                    assert torch.allclose(h[0], ro.h_in[s, t + 1], atol=1e-5)  # the GRU state it went on with
                    # and the action it took is the next decision's previous action
                    took = previous_action(ro.action[s, t], ro.phase[s, t].long(), self.spec.fight_actions)
                    assert int(ro.previous[s, t + 1]) == int(took), (item.update, s, t)
                if bool(ro.ep_start[s, t]):
                    assert int(ro.previous[s, t]) == net.no_previous
                checked += 1
        self.carried = (ro.open >= 0, item.behavior)
        return checked


def test_stored_log_probs_are_the_behavior_networks(tmp_path: Path):
    tr = Recording(load_config(None, OVERRIDES), tmp_path / "run", fake=True, log=False)
    rows = tr.train()
    assert [u for u, _, _ in tr.checked] == list(range(6))
    assert [b for _, b, _ in tr.checked] == [0, 0, 1, 2, 3, 4]  # the first rollout the learner's own, then one behind
    assert all(n > 50 for _, _, n in tr.checked)
    assert [r["loop/policy_lag"] for r in rows] == [0, 1, 1, 1, 1, 1]
    assert [s.update for s in tr.league.snapshots] == [2, 4]  # as the collector took the versions up


def test_overlapped_runs_are_the_same(tmp_path: Path):
    def run(name):
        tr = Trainer(load_config(None, OVERRIDES), tmp_path / name, fake=True, log=False)
        rows = tr.train()
        return rows, {k: v.clone() for k, v in tr.model.state_dict().items()}

    rows_a, w_a = run("a")
    rows_b, w_b = run("b")
    keys = ["reward_per_decision", "learner_decisions", "opponent_decisions", "ppo/loss_policy", "ppo/loss_value", "ppo/behavior_kl"]
    for ra, rb in zip(rows_a, rows_b):
        assert [ra[k] for k in keys] == [rb[k] for k in keys]
    for k in w_a:
        assert torch.equal(w_a[k], w_b[k]), k


def test_compact_tokens_are_the_same_input(tiny_cfg):
    """A step's decisions gathered as present tokens give back the pool's observation where it is present, and
    the network computes the same from the packed form as from the pool's (FakePool fills absent tokens with
    noise, which neither reads)."""
    pool = FakePool(envs=12, seed=6)
    spec = parse_spec(pool.spec())
    view = PoolView(pool, spec)
    batch = StepBatch(view, spec, tiny_cfg.pointer_set, torch.device("cpu"))
    m = Model(spec, tiny_cfg)
    g = torch.Generator().manual_seed(0)
    full = {s.name: s.count for s in spec.sets}
    for _ in range(3):
        order = (view.phase != 0).nonzero().squeeze(-1).flip(0)  # any order
        n = len(order)
        prev = torch.randint(0, m.n_previous, (n,), generator=g)
        step = batch.load(order, prev)
        dense = tokens_to_obs(spec, tiny_cfg.pointer_set, step.tokens, step.off, step.cnt, full, dense=True)
        pool_obs = view.obs(order, torch.device("cpu"))
        for s in spec.sets:
            mask = pool_obs["mask"][s.name]
            assert torch.equal(dense["mask"][s.name], mask)
            assert torch.equal(dense["x"][s.name][mask], pool_obs["x"][s.name][mask])
            assert torch.equal(dense["ids"][s.name][mask], pool_obs["ids"][s.name][mask])
        h = torch.randn(n, tiny_cfg.hidden, generator=g)
        with torch.no_grad():
            a = m.step(step.obs(0, n), h, step.previous)
            b = m.step(pool_obs, h, prev)
        for x, y in zip(a, b):
            assert torch.allclose(torch.nan_to_num(x, neginf=-1e9), torch.nan_to_num(y, neginf=-1e9), atol=1e-5)
        acts = torch.zeros(view.S, dtype=torch.int32)
        for st in order.tolist():
            legal = view.legal_fight[st] if view.phase[st] == 1 else view.legal_screen[st]
            acts[st] = int(legal.nonzero()[0])
        pool.step(acts.view(-1, 2).numpy())
