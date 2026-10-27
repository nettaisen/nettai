"""The parity fixture of net.rs (crates/nettai-training/src/net/tests.rs), made with the trainer's own model and
export: a network of `nettai_ai.model.Model` on the real pool's spec, small widths, its parameters drawn at random
(wider than the trainer's initialization, so its outputs differ from decision to decision), and decisions recorded
from the real pool (`nettai_gym`), of the three games, each with the previous action its stream took (none at a game's
first, where the GRU state is reset), with what the network computes for them, step by step from h0
(`nettai_ai.export.save_weights`, `save_fixture`).

    cd training && uv run --no-sync python \
        ../crates/nettai-training/tests/fixtures/make_net_fixture.py [out dir] [--size small|default] [--cache dir]

`small` (the default) is the committed fixture's: d 8, two heads, two layers, odd widths. `default` is the
trainer's own default widths (a check at full size, not committed: its name table alone is 2 MB).
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
from pathlib import Path

import numpy as np
import torch

import nettai_gym
from nettai_ai.config import LEVELS, EnvConfig, ModelConfig
from nettai_ai.envs import PoolView, pool_spec
from nettai_ai.export import save_fixture, save_weights
from nettai_ai.model import FORMAT, FORMAT_VERSION, Model, previous_action, previous_width
from nettai_ai.spec import parse_spec

SMALL = dict(d=8, heads=2, layers=2, ffn=16, token_hidden=8, pool_heads=2, summary=12, hidden=10, head_hidden=12, pointer_hidden=6)
GAMES = ["exe6", "exe5", "exe4"]
# Each game's decisions, in the order they were made: screen decisions and fight decisions.
SCREEN, FIGHT = 2, 3


def record(game: str, cache: str | None, seed: int) -> list[dict]:
    """Seat 0's decisions of one env of `game`, random allowed actions on both seats, through two custom screens,
    each with the previous action seat 0 took (none at its first)."""
    pool = nettai_gym.Pool(games=[game], envs=1, threads=8, seed=seed, cache=cache)
    spec = parse_spec(pool_spec(pool))
    view = PoolView(pool, spec)
    rng = np.random.default_rng(seed)
    out, screens, was_screen = [], 0, False
    previous = previous_width(spec) - 1
    for _ in range(4000):
        if view.phase[0] != 0:
            obs = view.obs(torch.tensor([0]), torch.device("cpu"))
            out.append({
                "obs": {k: {n: v[0].clone() for n, v in obs[k].items()} for k in obs},
                "legal_fight": view.legal_fight[0].clone(),
                "legal_screen": view.legal_screen[0].clone(),
                "phase": int(view.phase[0]),
                "previous": previous,
            })
            is_screen = int(view.phase[0]) == 2
            screens += is_screen and not was_screen
            was_screen = is_screen
            if screens > 2:
                break
        actions = np.zeros((1, 2), dtype=np.int32)
        for seat in range(2):
            s = seat
            phase = int(view.phase[s])
            legal = view.legal_fight[s] if phase == 1 else view.legal_screen[s]
            if phase:
                allowed = legal.nonzero().squeeze(-1).numpy()
                actions[0, seat] = int(rng.choice(allowed))
                if seat == 0:
                    previous = int(previous_action(torch.tensor(actions[0, 0]), torch.tensor(phase), spec.fight_actions))
        pool.step(actions)
        if bool(view.done[0]):  # a new round: no previous action
            previous = previous_width(spec) - 1
    return out


def pick(decisions: list[dict], spec) -> list[dict]:
    """A few of a game's decisions, in order: the screen decisions with the most allowed actions (one of them,
    where there is one, after a screen decision: a screen's first decision comes after the fight's), and the fight
    decisions with the most tokens present."""
    present = lambda d: sum(int(m.sum()) for m in d["obs"]["mask"].values())
    after_screen = lambda i: spec.fight_actions <= decisions[i]["previous"] < previous_width(spec) - 1
    screen = [i for i, d in enumerate(decisions) if d["phase"] == 2]
    fight = [i for i, d in enumerate(decisions) if d["phase"] == 1]
    screen = sorted(screen, key=lambda i: -int(decisions[i]["legal_screen"].sum()))
    screen = screen[: SCREEN - 1] + [next((i for i in screen[SCREEN - 1 :] if after_screen(i)), *screen[SCREEN - 1 : SCREEN])]
    fight = sorted(fight, key=lambda i: -present(decisions[i]))[:FIGHT]
    return [decisions[i] for i in sorted(screen + fight)]


def randomize(model: Model, gen: torch.Generator) -> None:
    """Every parameter and the input normalization drawn wider than the trainer's start, so the outputs vary."""
    with torch.no_grad():
        for name, p in model.named_parameters():
            if p.dim() == 2 and not name.startswith("names"):
                p.copy_(torch.randn(p.shape, generator=gen) * (1.5 / p.shape[1] ** 0.5))
            elif ".ln" in name or name.startswith("ln_f"):
                p.copy_((1.0 if name.endswith("weight") else 0.0) + torch.randn(p.shape, generator=gen) * 0.3)
            else:
                p.copy_(torch.randn(p.shape, generator=gen) * 0.5)
        for name, b in model.named_buffers():
            if name.endswith("norm_mean"):
                b.copy_(torch.randn(b.shape, generator=gen) * 0.3)
            elif name.endswith("norm_inv_std"):
                b.copy_(torch.exp(torch.randn(b.shape, generator=gen) * 0.5) * 1.5)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out", nargs="?", default=str(Path(__file__).parent))
    ap.add_argument("--size", default="small", choices=["small", "default"])
    ap.add_argument("--cache", default=os.environ.get("NETTAI_FINGERPRINTS"))
    a = ap.parse_args()
    out = Path(a.out)
    pool = nettai_gym.Pool(games=["exe6"], envs=1, threads=1, seed=1, cache=a.cache)
    spec = parse_spec(pool_spec(pool))
    seqs = [pick(record(g, a.cache, 11 + i), spec) for i, g in enumerate(GAMES)]
    cfg = ModelConfig(**SMALL) if a.size == "small" else ModelConfig()
    torch.manual_seed(1)
    model = Model(spec, cfg)
    model.eval()
    gen = torch.Generator().manual_seed(2)
    randomize(model, gen)
    flat = [d for seq in seqs for d in seq]
    starts = set()
    at = 0
    for seq in seqs:
        starts.add(at)
        at += len(seq)
    reset = torch.tensor([i in starts and i > 0 for i in range(len(flat))])
    # Each decision's previous action as its stream took it; none where the GRU state is reset.
    previous = torch.tensor([model.no_previous if reset[i] else d["previous"] for i, d in enumerate(flat)])
    fx = {
        k: {s.name: torch.stack([d["obs"][k][s.name] for d in flat]) for s in spec.sets} for k in ("x", "ids", "mask")
    } | {
        "legal_fight": torch.stack([d["legal_fight"] for d in flat]),
        "legal_screen": torch.stack([d["legal_screen"] for d in flat]),
        "phase": torch.tensor([d["phase"] for d in flat], dtype=torch.int8),
        "reset": reset,
        "previous": previous,
        "h0": torch.randn(cfg.hidden, generator=gen),
    }
    meta = {
        "format": FORMAT,
        "format_version": str(FORMAT_VERSION),
        "spec_hash": spec.hash,
        "architecture": json.dumps(model.architecture(), sort_keys=True),
        "levels": json.dumps({k: dataclasses.asdict(v) for k, v in LEVELS.items()}, sort_keys=True),
        "k": str(EnvConfig().k),
        "screen_decisions": str(EnvConfig().screen_decisions),
        "git_commit": "fixture",
        "git_dirty": "false",
        "update": "0",
        "decisions": "0",
    }
    name = "net" if a.size == "small" else "net-default"
    save_weights(model, out / f"{name}.safetensors", meta)
    save_fixture(model, fx, out / f"{name}.fixture.safetensors", {k: meta[k] for k in ("format", "format_version", "spec_hash")})
    phases = "".join("f" if d["phase"] == 1 else "s" for d in flat)
    kinds = "".join("n" if p == model.no_previous else ("f" if p < spec.fight_actions else "s") for p in previous.tolist())
    print(f"wrote {out}/{name}.safetensors and {name}.fixture.safetensors: {len(flat)} decisions ({phases}), previous actions ({kinds}), spec {spec.hash}")


if __name__ == "__main__":
    main()
