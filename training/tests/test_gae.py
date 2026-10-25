"""GAE over per-seat decision streams, with phases that skip seats, round ends and truncation."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest
import torch

from nettai_ai.config import LeagueConfig
from nettai_ai.league import League
from nettai_ai.model import Model, previous_action
from nettai_ai.ppo import compute_gae
from nettai_ai.rollout import Collector, Rollout
from nettai_ai.spec import parse_spec

from conftest import tiny_spec


def test_gae_by_hand():
    """Two streams, γ = λ = 0.5. Stream 0: three decisions, the last terminal, then an open one. Stream 1: a
    truncated decision (bootstrapped from its own value), then one more closed decision and an open one."""
    g = lam = 0.5
    reward = torch.tensor([[1.0, 0.0, 2.0, 9.0], [1.0, 0.5, 0.0, 9.0]])
    value = torch.tensor([[0.5, 1.0, 1.5, 7.0], [2.0, 1.0, 3.0, 7.0]])
    # next_value as the collector sets it: the next decision's value; 0 for terminal; own value if truncated.
    next_value = torch.tensor([[1.0, 1.5, 0.0, 0.0], [2.0, 3.0, 0.0, 0.0]])
    term = torch.tensor([[False, False, True, False], [False, False, False, False]])
    trunc = torch.tensor([[False, False, False, False], [True, False, False, False]])
    closed = torch.tensor([[True, True, True, False], [True, True, False, False]])
    adv, ret = compute_gae(reward, value, next_value, term, trunc, closed, g, lam)
    # Stream 0, backwards: A2 = 2 - 1.5 = 0.5 (terminal); A1 = 0 + .5*1.5 - 1 + .25*.5 = -0.125;
    # A0 = 1 + .5*1 - .5 + .25*(-0.125) = 0.96875.
    # Stream 1: A1 = 0.5 + .5*3 - 1 = 1.0 (the decision after is open: bootstrap only);
    # A0 = 1 + .5*2 - 2 = 0.0 (truncated: its own value bootstraps, and the chain stops).
    expected = torch.tensor([[0.96875, -0.125, 0.5, 0.0], [0.0, 1.0, 0.0, 0.0]])
    assert torch.allclose(adv, expected)
    assert torch.allclose(ret[closed], (expected + value)[closed])


class StubPool:
    """One env, both seats played by the learner; buffers follow a script of frames (the state after each step)."""

    def __init__(self, frames: list[dict]):
        self.raw = tiny_spec()
        self.frames = frames
        self.i = 0
        sets = self.raw["sets"]
        self.buf = {
            "x": {n: np.zeros((1, 2, s["count"], s["width"]), np.float32) for n, s in sets.items()},
            "ids": {n: np.zeros((1, 2, s["count"], s["ids"]), np.int32) for n, s in sets.items()},
            "mask": {n: np.ones((1, 2, s["count"]), np.bool_) for n, s in sets.items()},
            "phase": np.zeros((1, 2), np.int8),
            "legal_fight": np.ones((1, 2, 3), np.bool_),
            "legal_screen": np.ones((1, 2, 4), np.bool_),
            "hp_gain": np.zeros((1, 2), np.float32),
            "credit_lag": np.zeros((1, 2, 4), np.uint16),
            "credit_hp": np.zeros((1, 2, 4), np.float32),
            "dealt": np.zeros((1, 2), np.float32),
            "taken": np.zeros((1, 2), np.float32),
            "same_row": np.full((1, 2), -1, np.int8),
            "done": np.zeros(1, np.bool_),
            "truncated": np.zeros(1, np.bool_),
            "ending": np.zeros(1, np.int8),
            "result": np.zeros((1, 2), np.int8),
            "game": np.zeros(1, np.int8),
            "delay": np.zeros((1, 2), np.int16),
        }
        self.load()
        self.seats = []

    def load(self):
        f = self.frames[self.i]
        b = self.buf
        b["phase"][0] = f["phase"]
        b["hp_gain"][0] = f.get("gain", (0.0, 0.0))
        b["credit_lag"][0] = 0
        b["credit_hp"][0] = 0.0
        for seat, entries in f.get("credit", {}).items():
            for c, (lag, hp) in enumerate(entries):
                b["credit_lag"][0, seat, c] = lag
                b["credit_hp"][0, seat, c] = hp
        b["dealt"][0] = f.get("dealt", (0.0, 0.0))
        b["taken"][0] = f.get("taken", (0.0, 0.0))
        b["same_row"][0] = f.get("row", (-1, -1))
        b["done"][0] = f.get("done", False)
        b["truncated"][0] = f.get("trunc", False)
        b["result"][0] = f.get("result", (0, 0))
        b["x"]["general"][0, :, 0, 0] = self.i  # something to tell decisions apart

    def spec(self):
        return self.raw

    def buffers(self):
        return self.buf

    def step(self, actions):
        self.i += 1
        self.load()

    def set_seat(self, envs, seat, controller):
        self.seats.append((list(envs), seat, controller))

    def round_stats(self):
        return [{"env": 0, "ticks": 10, "truncated": bool(self.buf["truncated"][0]), "seats": [{"result": 0}, {"result": 0}]}]


def test_collector_streams(tmp_path: Path, tiny_cfg):
    frames = [
        {"phase": (1, 1)},
        {"phase": (1, 0), "gain": (0.1, -0.1)},  # seat 1 busy: its decision stays open
        {"phase": (1, 1), "gain": (0.2, -0.3)},
        {"phase": (2, 1), "gain": (0.05, -0.05), "done": True, "result": (1, -1)},  # a KO; the next round's first obs
        {"phase": (1, 1)},
        {"phase": (1, 1), "gain": (0.3, -0.3), "done": True, "trunc": True},  # cut off
        {"phase": (1, 1)},
    ]
    pool = StubPool(frames)
    spec = parse_spec(pool.spec())
    model = Model(spec, tiny_cfg)
    league = League(
        LeagueConfig(self_play=1.0, snapshots=0.0, bots=0.0, bot_names=[]), pool, 1, tmp_path, torch.device("cpu"),
        lambda: Model(spec, tiny_cfg), np.random.default_rng(0),
    )
    league.start()
    ro = Rollout(spec, 2, 8, tiny_cfg.hidden, torch.device("cpu"), tiny_cfg.pointer_set)
    col = Collector(pool, spec, model, league, ro, name_dropout=0.0, seed=0)
    col.collect(6, hp_weight=1.0, result_weight=2.0)

    s0, s1 = 0, 1
    # The stored decisions' first general float (the frame they were made at), read back from the compact store.
    general = lambda r, s, k: r.obs(torch.full((k,), s), torch.arange(k), dense=True)["x"]["general"][:, 0, 0].tolist()
    assert ro.cursor.tolist() == [6, 5]
    assert ro.open.tolist() == [5, 4]
    # Seat 0 decided at frames 0-5; seat 1 at 0, 2, 3, 4, 5.
    assert general(ro, s0, 6) == [0, 1, 2, 3, 4, 5]
    assert general(ro, s1, 5) == [0, 2, 3, 4, 5]
    assert torch.allclose(ro.reward[s0, :5], torch.tensor([0.1, 0.2, 0.05 + 2.0, 0.0, 0.3]))
    assert torch.allclose(ro.reward[s1, :4], torch.tensor([-0.3, -0.05 - 2.0, 0.0, -0.3]))
    assert ro.term[s0, :5].tolist() == [False, False, True, False, False]
    assert ro.trunc[s0, :5].tolist() == [False, False, False, False, True]
    assert ro.term[s1, :4].tolist() == [False, True, False, False]
    assert ro.trunc[s1, :4].tolist() == [False, False, False, True]
    assert ro.closed[s0].tolist()[:6] == [True] * 5 + [False]
    assert ro.closed[s1].tolist()[:5] == [True] * 4 + [False]
    assert ro.ep_start[s0, :6].tolist() == [True, False, False, True, False, True]
    assert ro.ep_start[s1, :5].tolist() == [True, False, True, False, True]
    # Each decision's previous action: none at a round's first, else the action the stream took before.
    for s, n in ((s0, 6), (s1, 5)):
        for t in range(n):
            took = previous_action(ro.action[s, t - 1], ro.phase[s, t - 1].long(), spec.fight_actions) if t else None
            assert int(ro.previous[s, t]) == (model.no_previous if ro.ep_start[s, t] else int(took)), (s, t)
    v0, v1 = ro.value[s0], ro.value[s1]
    nv0 = [v0[1], v0[2], 0.0, v0[4], v0[4]]  # next decision's value; terminal 0; truncated: its own
    nv1 = [v1[1], 0.0, v1[3], v1[3]]
    assert torch.allclose(ro.next_value[s0, :5], torch.tensor([float(x) for x in nv0]))
    assert torch.allclose(ro.next_value[s1, :4], torch.tensor([float(x) for x in nv1]))
    # The GRU starts from zero at each round's first decision.
    assert float(ro.h_in[s0, 3].abs().sum()) == 0.0 and float(ro.h_in[s0, 1].abs().sum()) > 0.0

    g, lam = 0.9, 0.8
    adv, _ = compute_gae(ro.reward, ro.value, ro.next_value, ro.term, ro.trunc, ro.closed, g, lam)
    r0 = ro.reward[s0]
    a4 = r0[4] + g * v0[4] - v0[4]
    a3 = r0[3] + g * v0[4] - v0[3] + g * lam * a4
    a2 = r0[2] - v0[2]
    a1 = r0[1] + g * v0[2] - v0[1] + g * lam * a2
    a0 = r0[0] + g * v0[1] - v0[0] + g * lam * a1
    assert torch.allclose(adv[s0, :6], torch.stack([a0, a1, a2, a3, a4, torch.tensor(0.0)]), atol=1e-6)
    r1 = ro.reward[s1]
    b3 = r1[3] + g * v1[3] - v1[3]
    b2 = r1[2] + g * v1[3] - v1[2] + g * lam * b3
    b1 = r1[1] - v1[1]
    b0 = r1[0] + g * v1[1] - v1[0] + g * lam * b1
    assert torch.allclose(adv[s1, :5], torch.stack([b0, b1, b2, b3, torch.tensor(0.0)]), atol=1e-6)

    # The open decisions start the next rollout (the other of the two), tokens and GRU state with them; nothing
    # else survives.
    nxt = Rollout(spec, 2, 8, tiny_cfg.hidden, torch.device("cpu"), tiny_cfg.pointer_set)
    col.begin(nxt)
    assert nxt.cursor.tolist() == [1, 1] and nxt.open.tolist() == [0, 0]
    assert general(nxt, s0, 1) == [5] and general(nxt, s1, 1) == [5]
    assert not nxt.closed.any()
    assert torch.equal(nxt.h_in[:, 0], ro.h_in[[s0, s1], [5, 4]])
    assert nxt.logp[s0, 0] == ro.logp[s0, 5] and nxt.logp[s1, 0] == ro.logp[s1, 4]
    assert nxt.previous[s0, 0] == ro.previous[s0, 5] and nxt.previous[s1, 0] == ro.previous[s1, 4]
    assert sum(t.n for t in nxt.tokens.values()) == 2 * sum(spec.set(n).count for n in nxt.tokens)  # all present
    # And in place, as after the normalization warmup's rollout.
    ro.take_open(ro)
    assert ro.cursor.tolist() == [1, 1] and general(ro, s0, 1) == [5] and general(ro, s1, 1) == [5]


def test_credits_reach_earlier_decisions_and_the_round_still_played_is_carried(tmp_path: Path, tiny_cfg):
    """HP credited back (lag 2: the decision before the one closing) is added to that decision's reward, scaled as
    the gain is; one from before the stream's first stored decision is the closing one's. With defer, a stream's
    last decisions of the round still being played are carried into the next rollout, closed ones with what they
    have, and only the decisions before them (and an ended round's) are trained; credits arriving in the next
    rollout reach the carried ones there."""
    frames = [
        {"phase": (1, 1)},
        {"phase": (1, 1), "gain": (0.1, 0.0)},
        {"phase": (1, 1), "credit": {0: [(2, 0.2)], 1: [(9, 0.4)]}},  # seat 0: decision 0's; seat 1: before its first
        {"phase": (1, 1), "gain": (0.0, 0.1), "done": True, "result": (0, 0), "dealt": (0.5, 0.0), "row": (1, 0)},
        {"phase": (1, 1)},
        {"phase": (1, 1)},
        {"phase": (1, 1)},
        {"phase": (1, 1), "credit": {0: [(3, 0.3), (5, 0.25)]}},  # in the next rollout: a carried decision's
        {"phase": (1, 1)},
    ]
    pool = StubPool(frames)
    spec = parse_spec(pool.spec())
    league = League(
        LeagueConfig(self_play=1.0, snapshots=0.0, bots=0.0, bot_names=[]), pool, 1, tmp_path, torch.device("cpu"),
        lambda: Model(spec, tiny_cfg), np.random.default_rng(0),
    )
    league.start()
    ro = Rollout(spec, 2, 6, tiny_cfg.hidden, torch.device("cpu"), tiny_cfg.pointer_set, defer=1)
    col = Collector(pool, spec, Model(spec, tiny_cfg), league, ro, name_dropout=0.0, seed=0, defer=1)
    col.collect(6, hp_weight=0.5, result_weight=1.0)
    assert torch.allclose(ro.reward[0, :3], torch.tensor([0.05 + 0.1, 0.0, 0.0]))
    assert torch.allclose(ro.reward[1, :3], torch.tensor([0.0, 0.2, 0.05]))
    assert ro.dealt[0, 2] == 0.5 and ro.same_row[0, 3] == 1 and ro.same_row[1, 3] == 0
    # The first round ended at decision 2: all of it trained. Of the second (decisions 3-5, 5 open), the last
    # closed one and the open one are carried.
    assert ro.cursor.tolist() == [6, 6] and ro.carry_from.tolist() == [4, 4]
    assert ro.trainable[0].tolist()[:6] == [True] * 4 + [False] * 2
    nxt = Rollout(spec, 2, 6, tiny_cfg.hidden, torch.device("cpu"), tiny_cfg.pointer_set, defer=1)
    col.begin(nxt)
    assert nxt.cursor.tolist() == [2, 2] and nxt.open.tolist() == [1, 1]
    assert nxt.closed[:, :2].tolist() == [[True, False], [True, False]]
    assert torch.equal(nxt.h_in[:, :2], ro.h_in[:, 4:6])
    col.collect(2, hp_weight=0.5, result_weight=1.0)
    # Frame 7 closes decision 2 of nxt: lag 3 is decision 0 (carried, closed in the last rollout), lag 5 is from
    # before the stream's first stored decision.
    assert torch.allclose(nxt.reward[0, :3], torch.tensor([0.15, 0.0, 0.125]))


def test_scripted_seat_with_a_decision_is_an_error(tmp_path: Path, tiny_cfg):
    pool = StubPool([{"phase": (1, 1)}, {"phase": (1, 1)}])
    spec = parse_spec(pool.spec())
    league = League(
        LeagueConfig(self_play=0.0, snapshots=0.0, bots=1.0, bot_names=["still"]), pool, 1, tmp_path,
        torch.device("cpu"), lambda: Model(spec, tiny_cfg), np.random.default_rng(0),
    )
    league.start()
    # The first round is self-play; pretend the bot round began.
    league._begin(np.array([0]))
    col = Collector(pool, spec, Model(spec, tiny_cfg), league, Rollout(spec, 2, 4, tiny_cfg.hidden, torch.device("cpu"), tiny_cfg.pointer_set), 0.0, 0)
    with pytest.raises(RuntimeError, match="scripted seat"):
        col.collect(1, 1.0, 1.0)


def test_a_stall_costs():
    """§6.4: a KO is its result; on the judge's ruling a win is worth result − judged_cost, a draw −judged_cost,
    a loss as a KO's; the fight's limit is `limit` to each seat."""
    from nettai_ai.rollout import BY_JUDGE, BY_KO, BY_LIMIT, round_end_reward

    result = torch.tensor([1.0, -1.0, 1.0, -1.0, 0.0, 0.0, 0.0])
    ending = torch.tensor([BY_KO, BY_KO, BY_JUDGE, BY_JUDGE, BY_JUDGE, BY_LIMIT, BY_KO])
    got = round_end_reward(result, ending, 1.0, 0.75, -1.0)
    assert got.tolist() == [1.0, -1.0, 0.25, -1.0, -0.75, -1.0, 0.0]
