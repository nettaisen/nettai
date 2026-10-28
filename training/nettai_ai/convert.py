"""A checkpoint of an older layout as this code's (§8): its network warm-started (`model.warm_start`) on the
pool's observation spec now, for evaluation, export and `--init-from`. Its observation must be the pool's, or the
pool's but for features a set gained (the danger map's in the panels, §3.2): those come in as columns the network
reads nothing of (zero weights) until trained to. What may differ besides is what warm_start knows (the screen's
actions after the slots seven keys, as the first two cloud runs' were; a GRU that read no previous action). Not for
`--resume`: the optimizer's state and the league are left out.

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


# The input statistics' weight a widened set keeps (in tokens, a small share of one rollout's): a gained feature's
# start (mean 0, variance 1) gives way to what the first rollouts measure, and the others' to the same as before.
WIDENED_COUNT = 100_000


def widening(old: dict[str, Any], new: dict[str, Any]) -> dict[str, tuple[int, list[int]]] | None:
    """How `new`'s observation only adds features to `old`'s sets: by set that gained some, its width now and each
    of its old columns' place now (a feature found by name and width). None where they differ otherwise (another
    set, count, ids, buckets or split, a feature gone or of another width)."""
    if old["buckets"] != new["buckets"] or old.get("split") != new.get("split") or list(old["sets"]) != list(new["sets"]):
        return None
    out: dict[str, tuple[int, list[int]]] = {}
    for name, o in old["sets"].items():
        n = new["sets"][name]
        if (o.get("count"), o.get("ids")) != (n.get("count"), n.get("ids")):
            return None
        if o.get("features") == n.get("features"):
            if o.get("width") != n.get("width"):
                return None
            continue
        if o.get("features") is None or n.get("features") is None:
            return None
        at, col = {}, 0
        for f, w in n["features"]:
            at[f] = (col, w)
            col += w
        cols: list[int] = []
        for f, w in o["features"]:
            if at.get(f, (0, -1))[1] != w:
                return None
            cols += range(at[f][0], at[f][0] + w)
        out[name] = (int(n["width"]), cols)
    return out


def widen_model(weights: dict[str, torch.Tensor], wide: dict[str, tuple[int, list[int]]]) -> dict[str, torch.Tensor]:
    """A network's weights with each widened set's input columns where they are now: its first layer's gained
    columns zero, their input normalization the identity."""
    out = dict(weights)
    for name, (width, cols) in wide.items():
        idx = torch.tensor(cols, dtype=torch.long)
        w = weights[f"enc.{name}.fc1.weight"]
        fc1 = torch.zeros(w.shape[0], width, dtype=w.dtype)
        fc1[:, idx] = w
        out[f"enc.{name}.fc1.weight"] = fc1
        for key, fill in (("norm_mean", 0.0), ("norm_inv_std", 1.0)):
            v = weights[f"enc.{name}.{key}"]
            new = torch.full((width,), fill, dtype=v.dtype)
            new[idx] = v
            out[f"enc.{name}.{key}"] = new
    return out


def widen_norm(norm: dict[str, Any], wide: dict[str, tuple[int, list[int]]]) -> dict[str, Any]:
    """The trainer's input statistics (`ppo.RunningNorm`'s) with each widened set's columns where they are now: a
    gained feature's mean 0 and variance 1, and the set's weight cut to `WIDENED_COUNT` tokens (its variances
    kept), so what the run measures of the gained features soon counts."""
    out = {k: dict(v) for k, v in norm.items()}
    for name, (width, cols) in wide.items():
        idx = torch.tensor(cols, dtype=torch.long)
        count = float(norm["count"][name])
        keep = min(count, WIDENED_COUNT)
        mean = torch.zeros(width, dtype=torch.float64)
        mean[idx] = norm["mean"][name].to(torch.float64)
        m2 = torch.full((width,), keep, dtype=torch.float64)
        m2[idx] = norm["m2"][name].to(torch.float64) * (keep / count if count else 0.0)
        out["count"][name] = torch.tensor(keep, dtype=torch.float64)
        out["mean"][name] = mean
        out["m2"][name] = m2
    return out


def widen_fixture(fx: dict[str, Any], wide: dict[str, tuple[int, list[int]]]) -> dict[str, Any]:
    """Recorded decisions with each widened set's features where they are now, the gained ones zero."""
    if not wide:
        return fx
    out = dict(fx)
    out["x"] = dict(fx["x"])
    for name, (width, cols) in wide.items():
        x = fx["x"][name]
        new = torch.zeros(*x.shape[:-1], width, dtype=x.dtype)
        new[..., torch.tensor(cols, dtype=torch.long)] = x
        out["x"][name] = new
    return out


def convert(state: dict[str, Any], spec_raw: dict[str, Any]) -> dict[str, Any]:
    """Checkpoint `state` with its network on `spec_raw` (the pool's spec now)."""
    old = state["spec"]
    wide = {} if same_observation(old, spec_raw) else widening(old, spec_raw)
    if wide is None:
        raise SystemExit(f"convert: the checkpoint's observation (spec {old['hash']}) isn't the pool's ({spec_raw['hash']}), nor the pool's but for features it gained")
    cfg = config_from_dict(state["config"])
    model = Model(parse_spec(spec_raw), cfg.model)
    warm_start(model, widen_model(state["model"], wide))
    out = {k: v for k, v in state.items() if k not in ("model", "spec", "optimizer", "league", "fixture", "norm")}
    if state.get("norm") is not None:
        out["norm"] = widen_norm(state["norm"], wide)
    out["model"] = {k: v.detach().cpu() for k, v in model.state_dict().items()}
    out["spec"] = spec_raw
    out["params"] = count_params(model)
    out["converted_from"] = {"spec_hash": old["hash"], "screen_actions": old["screen_actions"]}
    if state.get("fixture") is not None:
        out["fixture"] = convert_fixture(widen_fixture(state["fixture"], wide), int(old["screen_actions"]), model)
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
