"""Legal masks: an illegal action is never sampled, at any level, and an absent slot is never pointed at."""

from __future__ import annotations

import pytest
import torch

from nettai_ai.config import LEVELS
from nettai_ai.fake import FakePool
from nettai_ai.envs import PoolView
from nettai_ai.model import Model, mask_logits, select
from nettai_ai.spec import parse_spec


def random_legal(n: int, width: int, g: torch.Generator) -> torch.Tensor:
    legal = torch.rand(n, width, generator=g) < 0.2
    legal[torch.arange(n), torch.randint(0, width, (n,), generator=g)] = True  # at least one
    return legal


@pytest.mark.parametrize("level", [None, *LEVELS.values()], ids=lambda l: "train" if l is None else l.name)
def test_select_never_illegal(level):
    g = torch.Generator().manual_seed(1)
    n = 20000
    lf, ls = random_legal(n, 20, g), random_legal(n, 19, g)
    # Large logits on illegal actions, so a mask that leaked would show at once.
    fight = torch.randn(n, 20, generator=g) * 5 + 20 * (~lf)
    screen = torch.randn(n, 19, generator=g) * 5 + 20 * (~ls)
    phase = torch.randint(1, 3, (n,), generator=g)
    a, logp = select(mask_logits(fight, lf), mask_logits(screen, ls), phase, level, g)
    legal = torch.where((phase == 1).unsqueeze(-1), lf, torch.nn.functional.pad(ls, (0, 1)))
    assert legal.gather(1, a.unsqueeze(1)).all()
    assert torch.isfinite(logp).all()


def test_select_refuses_no_legal_action():
    f = torch.zeros(1, 20)
    s = torch.zeros(1, 19)
    with pytest.raises(RuntimeError):
        select(mask_logits(f, torch.zeros(1, 20, dtype=torch.bool)), s, torch.tensor([1]))


def test_model_on_pool_never_illegal(tiny_cfg):
    pool = FakePool(envs=32, seed=3)
    spec = parse_spec(pool.spec())
    model = Model(spec, tiny_cfg)
    view = PoolView(pool, spec)
    g = torch.Generator().manual_seed(0)
    h = torch.zeros(view.S, tiny_cfg.hidden)
    for _ in range(60):
        phase = view.phase.long()
        idx = (phase != 0).nonzero().squeeze(-1)
        f, s, _, h_new = model.step(view.obs(idx, torch.device("cpu")), h[idx])
        h[idx] = h_new.detach()
        a, _ = select(mask_logits(f, view.legal_fight[idx]), mask_logits(s, view.legal_screen[idx]), phase[idx], None, g)
        acts = torch.zeros(view.S, dtype=torch.int32)
        acts[idx] = a.to(torch.int32)
        slot = (phase[idx] == 2) & (a < model.n_slots)
        if slot.any():  # the pointer only ever lands on a present slot
            present = view.mask["slots"][idx[slot]]
            assert present.gather(1, a[slot].unsqueeze(1)).all()
        pool.step(acts.view(-1, 2).numpy())  # FakePool raises on an illegal action
