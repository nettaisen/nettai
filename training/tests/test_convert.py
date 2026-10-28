"""convert (§8): a checkpoint whose screen actions after the slots were seven keys, on the spec now."""

from __future__ import annotations

import pytest
import torch

from nettai_ai.config import Config, config_to_dict
from nettai_ai.convert import convert, same_observation
from nettai_ai.model import Model
from nettai_ai.spec import parse_spec, spec_hash

from conftest import tiny_spec


def keyed(raw: dict) -> dict:
    """The spec `raw` as it was with seven keys after the slots."""
    old = {k: v for k, v in raw.items() if k != "hash"}
    old["screen_actions"] = 2 + 7
    old["hash"] = spec_hash(old)
    return old


def test_a_keyed_checkpoint_converts_to_the_spec_now(tiny_cfg):
    """Its network on the spec now (the entries' head fresh, warm_start's mapping), the optimizer and league left
    out, the rest kept; its recorded decisions with the slots' allowed actions as they were, no entry allowed, and
    a key as the previous action read as none. Another observation is refused."""
    new = tiny_spec()
    spec = parse_spec(new)
    src = Model(spec, tiny_cfg)
    head = tiny_cfg.summary + spec.fight_actions + src.n_slots
    old_weights = {k: v.clone() for k, v in src.state_dict().items() if not k.startswith("entry")}
    w = old_weights["gru.weight_ih_l0"]
    old_weights["gru.weight_ih_l0"] = torch.cat([w[:, :head], torch.randn(w.shape[0], 7), w[:, -1:]], 1)
    old_weights |= {"key1.weight": src.entry1.weight.clone(), "key1.bias": src.entry1.bias.clone()}
    old_weights |= {"key2.weight": torch.randn(7, tiny_cfg.head_hidden), "key2.bias": torch.randn(7)}
    fa, old_none = spec.fight_actions, spec.fight_actions + 9
    state = {
        "update": 1000, "decisions": 5, "config": config_to_dict(Config(model=tiny_cfg)), "git": {"commit": "x", "dirty": False},
        "spec": keyed(new), "model": old_weights, "optimizer": {"state": {}}, "league": {"snapshots": []}, "norm": {},
        "curriculum": {}, "init_from": None, "params": 0,
        "fixture": {
            "legal_screen": torch.tensor([[True, False] + [True] * 7, [False, True] + [False] * 7]),
            "previous": torch.tensor([old_none, fa + 1 + 2]),  # none, then a key
            "reset": torch.tensor([False, False]),
        },
    }
    out = convert(state, new)
    assert "optimizer" not in out and "league" not in out
    assert out["spec"] == new and out["converted_from"] == {"spec_hash": state["spec"]["hash"], "screen_actions": 9}
    assert out["update"] == 1000 and out["git"] == state["git"]
    m = Model(spec, tiny_cfg)
    m.load_state_dict(out["model"], strict=True)
    assert torch.equal(m.fight2.weight, src.fight2.weight) and torch.equal(m.state_dict()["gru.weight_ih_l0"][:, :head], w[:, :head])
    fx = out["fixture"]
    assert fx["legal_screen"].tolist() == [[True, False, False, False], [False, True, False, False]]
    assert fx["previous"].tolist() == [m.no_previous, m.no_previous]
    other = tiny_spec()
    other["sets"]["slots"] = {"count": 2, "width": 3, "ids": 2}
    other["hash"] = spec_hash({k: v for k, v in other.items() if k != "hash"})
    assert not same_observation(state["spec"], other)
    with pytest.raises(SystemExit, match="observation"):
        convert(state, other)


def with_features(raw: dict, objects: list) -> dict:
    """`raw` with its objects set's features named (and its width theirs)."""
    out = {k: v for k, v in raw.items() if k != "hash"}
    out["sets"] = {**raw["sets"], "objects": {**raw["sets"]["objects"], "features": objects, "width": sum(w for _, w in objects)}}
    out["hash"] = spec_hash(out)
    return out


def test_a_set_that_gained_features_converts_and_plays_as_it_did(tiny_cfg):
    """A checkpoint of a spec whose objects set then gained a feature (in the middle): its network reads nothing of
    the gained columns (it plays as it did whatever they hold), its input statistics keep the old features' means
    and variances where they are now with the gained one's mean 0 and variance 1 at a small weight, and its
    recorded decisions have the gained columns zero. A feature gone or of another width is refused."""
    old_raw = with_features(tiny_spec(), [["a", 1], ["b", 2]])
    new_raw = with_features(tiny_spec(), [["a", 1], ["d", 2], ["b", 2]])
    src = Model(parse_spec(old_raw), tiny_cfg)
    with torch.no_grad():
        src.enc["objects"].norm_mean.copy_(torch.tensor([0.5, -1.0, 2.0]))
        src.enc["objects"].norm_inv_std.copy_(torch.tensor([2.0, 3.0, 0.5]))
    count = 1e9
    norm = {
        "count": {"objects": torch.tensor(count, dtype=torch.float64)},
        "mean": {"objects": torch.tensor([0.5, -1.0, 2.0], dtype=torch.float64)},
        "m2": {"objects": torch.tensor([1.0, 4.0, 9.0], dtype=torch.float64) * count},
    }
    x_old = torch.randn(2, 3, 3)
    state = {
        "update": 3250, "decisions": 5, "config": config_to_dict(Config(model=tiny_cfg)), "git": {"commit": "x", "dirty": False},
        "spec": old_raw, "model": {k: v.clone() for k, v in src.state_dict().items()}, "optimizer": {}, "league": {},
        "norm": norm, "curriculum": {}, "init_from": None, "params": 0,
        "fixture": {"x": {"objects": x_old}, "legal_screen": torch.ones(2, 4, dtype=torch.bool), "reset": torch.tensor([False, False])},
    }
    out = convert(state, new_raw)
    m = Model(parse_spec(new_raw), tiny_cfg)
    m.load_state_dict(out["model"], strict=True)
    enc = m.enc["objects"]
    assert torch.equal(enc.fc1.weight[:, [0, 3, 4]], src.enc["objects"].fc1.weight)
    assert torch.equal(enc.fc1.weight[:, 1:3], torch.zeros(enc.fc1.weight.shape[0], 2))
    assert enc.norm_mean.tolist() == [0.5, 0.0, 0.0, -1.0, 2.0] and enc.norm_inv_std.tolist() == [2.0, 1.0, 1.0, 3.0, 0.5]
    x_new = torch.zeros(2, 3, 5)
    x_new[..., [0, 3, 4]] = x_old
    with torch.no_grad():
        want = src.enc["objects"](x_old)
        assert torch.allclose(enc(x_new), want)
        x_new[..., 1:3] = torch.randn(2, 3, 2) * 10
        assert torch.allclose(enc(x_new), want), "the gained columns are read as nothing"
    n = out["norm"]
    keep = float(n["count"]["objects"])
    assert keep < count
    var = (n["m2"]["objects"] / keep).tolist()
    assert n["mean"]["objects"].tolist() == [0.5, 0.0, 0.0, -1.0, 2.0]
    assert [round(v, 9) for v in var] == [1.0, 1.0, 1.0, 4.0, 9.0]
    assert torch.equal(out["fixture"]["x"]["objects"][..., [0, 3, 4]], x_old)
    assert torch.equal(out["fixture"]["x"]["objects"][..., 1:3], torch.zeros(2, 3, 2))
    for bad in ([["a", 1], ["d", 2]], [["a", 1], ["b", 3]]):
        with pytest.raises(SystemExit, match="observation"):
            convert(state, with_features(tiny_spec(), bad))
