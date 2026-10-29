"""A checkpoint of an older layout as this code's (§8): its network warm-started (`model.warm_start`) on the
pool's observation spec now, for evaluation, export and `--init-from`. Its observation must be the pool's; what
may differ is what warm_start knows (the screen's actions after the slots seven keys, as the first two cloud runs'
were; a GRU that read no previous action). Not for `--resume`: the optimizer's state and the league are left out.

    uv run python -m nettai_ai.convert CHECKPOINT OUT

The recorded decisions export checks itself with (the checkpoint's fixture) come along: the slots' allowed
actions as they were, no entry of the form list allowed, and a previous action the new layout lacks (a key) read
as none.
"""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import torch

from . import checkpoint
from .config import config_from_dict
from .envs import make_pool, pool_spec
from .model import Model, count_params, warm_start
from .spec import parse_spec


def same_observation(a: dict[str, Any], b: dict[str, Any]) -> bool:
    """Whether two specs (raw, as the trainer reads them) describe the same observation: the names' buckets, the
    sets in order with their counts, widths, ids and features, and the trainer's split of them."""
    sets = lambda raw: [(n, s.get("count"), s.get("width"), s.get("ids"), s.get("features")) for n, s in raw["sets"].items()]
    return a["buckets"] == b["buckets"] and sets(a) == sets(b) and a.get("split") == b.get("split")


def convert(state: dict[str, Any], spec_raw: dict[str, Any]) -> dict[str, Any]:
    """Checkpoint `state` with its network on `spec_raw` (the pool's spec now)."""
    old = state["spec"]
    if not same_observation(old, spec_raw):
        raise SystemExit(f"convert: the checkpoint's observation (spec {old['hash']}) isn't the pool's ({spec_raw['hash']})")
    cfg = config_from_dict(state["config"])
    model = Model(parse_spec(spec_raw), cfg.model)
    warm_start(model, state["model"])
    out = {k: v for k, v in state.items() if k not in ("model", "spec", "optimizer", "league", "fixture")}
    out["model"] = {k: v.detach().cpu() for k, v in model.state_dict().items()}
    out["spec"] = spec_raw
    out["params"] = count_params(model)
    out["converted_from"] = {"spec_hash": old["hash"], "screen_actions": old["screen_actions"]}
    if state.get("fixture") is not None:
        out["fixture"] = convert_fixture(state["fixture"], int(old["screen_actions"]), model)
    return out


def convert_fixture(fx: dict[str, Any], old_screen: int, model: Model) -> dict[str, Any]:
    """Recorded decisions of the old layout (`old_screen` screen actions) for `model`'s."""
    fa, slots, sa = model.spec.fight_actions, model.n_slots, model.spec.screen_actions
    out = dict(fx)
    legal = torch.zeros(fx["legal_screen"].shape[0], sa, dtype=torch.bool)
    legal[:, :slots] = fx["legal_screen"][:, :slots]
    out["legal_screen"] = legal
    n = fx["reset"].shape[0]
    old_none = fa + old_screen
    previous = fx.get("previous", torch.full((n,), old_none, dtype=torch.int64)).clone()
    kept = previous < fa + slots
    out["previous"] = torch.where(kept, previous, torch.full_like(previous, model.no_previous))
    return out


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("checkpoint", help="a checkpoint file or a run directory (its latest checkpoint)")
    ap.add_argument("out", help="the converted checkpoint's file")
    a = ap.parse_args()
    state = checkpoint.load(a.checkpoint)
    cfg = config_from_dict(state["config"])
    pool = make_pool(cfg.env, False, games=list(cfg.env.games)[:1], envs=1, threads=1, hp=(1000, 1000))
    spec_raw = pool_spec(pool)
    if spec_raw["hash"] == state["spec"]["hash"]:
        raise SystemExit(f"convert: {a.checkpoint} is already of the pool's spec {spec_raw['hash']}")
    out = convert(state, spec_raw)
    Path(a.out).parent.mkdir(parents=True, exist_ok=True)
    torch.save(out, a.out)
    print(f"wrote {a.out}: update {state['update']}, spec {state['spec']['hash']} → {spec_raw['hash']}")


if __name__ == "__main__":
    main()
