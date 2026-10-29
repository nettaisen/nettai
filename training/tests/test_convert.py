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
