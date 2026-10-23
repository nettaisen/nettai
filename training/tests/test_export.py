"""Export round trip: the safetensors file rebuilds the same network, and the fixture holds its outputs."""

from __future__ import annotations

import json
from pathlib import Path

import torch
from safetensors import safe_open

from nettai_ai import checkpoint
from nettai_ai.config import load_config
from nettai_ai.envs import PoolView
from nettai_ai.export import export, load_weights, model_from_checkpoint, run_fixture
from nettai_ai.model import FORMAT
from nettai_ai.train import Trainer

from conftest import TINY_MODEL


def tiny_run(tmp_path: Path) -> Trainer:
    over = [
        'env.games=["exe6"]', "env.envs=6", "env.threads=1", "ppo.steps=12", "ppo.chunk=6", "ppo.epochs=1",
        "ppo.minibatch_chunks=4", "ppo.norm_warmup_steps=4", "run.updates=2", "run.checkpoint_every=1",
        "run.device='cpu'", "run.fixture_decisions=8",
    ] + [f"model.{k}={v}" for k, v in TINY_MODEL.items()]
    tr = Trainer(load_config(None, over), tmp_path / "run", fake=True, log=False)
    tr.train()
    return tr


def test_export_round_trip(tmp_path: Path):
    tr = tiny_run(tmp_path)
    out, fx = tmp_path / "w.safetensors", tmp_path / "w.fixture.safetensors"
    export(tr.run_dir, out, fx)

    state = checkpoint.load(tr.run_dir)
    original = model_from_checkpoint(state)
    rebuilt, meta = load_weights(out)
    assert meta["format"] == FORMAT and meta["spec_hash"] == tr.spec.hash
    assert json.loads(meta["architecture"])["spec"]["hash"] == tr.spec.hash
    assert set(json.loads(meta["levels"])) == {"easy", "normal", "hard"}
    assert meta["git_commit"] == state["git"]["commit"]
    assert meta["k"] == "4" and meta["screen_decisions"] == "24"

    # Same logits on fresh observations from the pool.
    view = PoolView(tr.pool, tr.spec)
    idx = (view.phase != 0).nonzero().squeeze(-1)
    obs = view.obs(idx, torch.device("cpu"))
    h = torch.randn(len(idx), original.cfg.hidden)
    with torch.no_grad():
        a, b = original.step(obs, h), rebuilt.step(obs, h)
    for x, y in zip(a, b):
        assert torch.equal(x, y)

    # The fixture: its outputs are what the network computes from its inputs, step by step.
    with safe_open(str(fx), framework="pt") as f:
        t = {k: f.get_tensor(k) for k in f.keys()}
    n = t["reset"].shape[0]
    assert n == 8 and t["fight"].shape == (n, 20) and t["screen"].shape == (n, 19) and t["hidden"].shape[0] == n
    fixture = {
        k: {s.name: t[f"{k}.{s.name}"] for s in tr.spec.sets} for k in ("x", "ids", "mask")
    } | {k: t[k] for k in ("legal_fight", "legal_screen", "phase", "reset", "h0")}
    again = run_fixture(rebuilt, fixture)
    for k in ("fight_raw", "screen_raw", "fight", "screen", "value", "hidden"):
        assert torch.equal(again[k], t[k]), k
    # Masked entries are -inf exactly where the legal mask is false.
    assert torch.equal(torch.isinf(t["fight"]), ~t["legal_fight"])
