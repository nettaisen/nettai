"""FakePool keeps §6.6's contract: shapes and dtypes from the spec, scripted seats never decide, seats and HP
change from the next round, hp_gain is a fraction of the round's max HP."""

from __future__ import annotations

import numpy as np
import pytest

from nettai_ai.fake import FakePool


def legal_actions(pool):
    b = pool.buffers()
    a = np.zeros(b["phase"].shape, np.int32)
    for (e, s), ph in np.ndenumerate(b["phase"]):
        if ph == 1:
            a[e, s] = np.flatnonzero(b["legal_fight"][e, s])[0]
        elif ph == 2:
            a[e, s] = np.flatnonzero(b["legal_screen"][e, s])[0]
    return a


def test_buffers_match_the_spec():
    pool = FakePool(envs=5)
    spec, b = pool.spec(), pool.buffers()
    for name, s in spec["sets"].items():
        assert b["x"][name].shape == (5, 2, s["count"], s["width"]) and b["x"][name].dtype == np.float32
        assert b["ids"][name].shape == (5, 2, s["count"], s["ids"]) and b["ids"][name].dtype == np.int32
        assert b["mask"][name].shape == (5, 2, s["count"]) and b["mask"][name].dtype == np.bool_
    expected = {
        "phase": ((5, 2), np.int8), "legal_fight": ((5, 2, 20), np.bool_), "legal_screen": ((5, 2, 19), np.bool_),
        "hp_gain": ((5, 2), np.float32), "done": ((5,), np.bool_), "truncated": ((5,), np.bool_),
        "result": ((5, 2), np.int8), "game": ((5,), np.int8), "delay": ((5, 2), np.int16),
    }
    for k, (shape, dtype) in expected.items():
        assert b[k].shape == shape and b[k].dtype == dtype, k


def test_scripted_seats_and_next_round():
    pool = FakePool(envs=8, seed=2)
    b = pool.buffers()
    pool.set_seat(range(8), 1, "masher")
    pool.set_hp(300, 300)
    assert (b["phase"][:, 1] != 0).any()  # the current round is still policy against policy
    seen_round_two = np.zeros(8, bool)
    for _ in range(400):
        pool.step(legal_actions(pool))
        seen_round_two |= b["done"]
        assert (b["phase"][seen_round_two, 1] == 0).all()
        assert (pool.maxhp[seen_round_two] == 300).all()
    assert seen_round_two.all()
    stats = pool.round_stats()
    assert {"env", "ticks", "seats"} <= set(stats[0]) and len(stats[0]["seats"]) == 2


def test_hp_gain_is_a_fraction_of_max_hp():
    """Pressing the target hits for 5% of the round's max HP, whatever the HP: hp_gain is +0.05 either way."""
    for hp in (250, 1000):
        pool = FakePool(envs=16, seed=4, hp=(hp, hp))
        b = pool.buffers()
        pool.set_seat(range(16), 1, "still")
        second = np.zeros(16, bool)
        while not second.all():
            pool.step(legal_actions(pool))
            second |= b["done"]
        checked = 0
        for _ in range(40):
            a = legal_actions(pool)
            fight = b["phase"][:, 0] == 1
            a[fight, 0] = pool.target[fight, 0]
            keys = b["legal_screen"][:, 0, pool.n_slots :]
            for e in np.flatnonzero((b["phase"][:, 0] == 2) & keys.any(1)):
                a[e, 0] = pool.n_slots + int(np.flatnonzero(keys[e])[0])  # a key: no hit
            pool.step(a)
            hit = fight & (b["phase"][:, 0] != 0) & ~b["done"]
            assert np.allclose(b["hp_gain"][hit, 0], 0.05)
            checked += int(hit.sum())
        assert checked > 50
        assert (pool.maxhp == hp).all()


def test_illegal_action_is_refused():
    pool = FakePool(envs=2, seed=5)
    b = pool.buffers()
    a = legal_actions(pool)
    e, s = np.argwhere(b["phase"] == 2)[0]
    a[e, s] = int(np.flatnonzero(~b["legal_screen"][e, s])[0])
    with pytest.raises(ValueError, match="illegal"):
        pool.step(a)
