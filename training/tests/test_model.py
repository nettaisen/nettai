"""The network's forward: sequences equal repeated steps, absent tokens are never read, no token at all is fine,
name dropout and the MAC budget (§7)."""

from __future__ import annotations

import dataclasses

import pytest
import torch

from nettai_ai.config import SIZES, ModelConfig
from nettai_ai.fake import default_spec
from nettai_ai.model import BUDGET_MAX, BUDGET_TYPICAL, TYPICAL, Model, budget_report, count_macs, warm_start
from nettai_ai.rollout import drop_names
from nettai_ai.spec import parse_spec

from conftest import tiny_spec


def random_obs(spec, n, g, present=0.6):
    obs = {"x": {}, "ids": {}, "mask": {}}
    for s in spec.sets:
        obs["x"][s.name] = torch.randn(n, s.count, s.width, generator=g)
        obs["ids"][s.name] = torch.randint(0, spec.buckets, (n, s.count, s.ids), generator=g, dtype=torch.int32)
        obs["mask"][s.name] = torch.rand(n, s.count, generator=g) < present
    return obs


def random_previous(m, shape, g):
    return torch.randint(0, m.n_previous, shape, generator=g)


def test_sequence_equals_steps(tiny_cfg):
    spec = parse_spec(tiny_spec())
    m = Model(spec, tiny_cfg)
    g = torch.Generator().manual_seed(0)
    B, L = 3, 5
    obs = random_obs(spec, B * L, g)
    seq = {k: {n: v.view(B, L, *v.shape[1:]) for n, v in d.items()} for k, d in obs.items()}
    reset = torch.zeros(B, L, dtype=torch.bool)
    reset[1, 2] = True
    h0 = torch.randn(B, tiny_cfg.hidden, generator=g)
    prev = random_previous(m, (B, L), g)
    with torch.no_grad():
        f, s, v, hs = m.sequence(seq, h0, reset, prev)
        h = h0.clone()
        for t in range(L):
            h = h * (~reset[:, t]).unsqueeze(-1)
            o = {k: {n: x[:, t] for n, x in d.items()} for k, d in seq.items()}
            f1, s1, v1, h = m.step(o, h, prev[:, t])
            assert torch.allclose(f[:, t], f1, atol=1e-5)
            assert torch.allclose(s[:, t], s1, atol=1e-5)
            assert torch.allclose(v[:, t], v1, atol=1e-5)
            assert torch.allclose(hs[:, t], h, atol=1e-5)


def test_absent_tokens_are_not_read(tiny_cfg):
    spec = parse_spec(tiny_spec())
    m = Model(spec, tiny_cfg)
    g = torch.Generator().manual_seed(1)
    obs = random_obs(spec, 8, g)
    obs["mask"]["general"][:] = True
    h = torch.zeros(8, tiny_cfg.hidden)
    prev = random_previous(m, (8,), g)
    with torch.no_grad():
        a = m.step(obs, h, prev)
        for s in spec.sets:
            absent = ~obs["mask"][s.name]
            obs["x"][s.name][absent] = 1e6
            obs["ids"][s.name][absent] = 7
        b = m.step(obs, h, prev)
    for x, y in zip(a, b):
        assert torch.equal(torch.nan_to_num(x, neginf=-1), torch.nan_to_num(y, neginf=-1))


def test_no_token_at_all(tiny_cfg):
    spec = parse_spec(tiny_spec())
    m = Model(spec, tiny_cfg)
    obs = random_obs(spec, 2, torch.Generator().manual_seed(2), present=0.0)
    with torch.no_grad():
        f, s, v, h = m.step(obs, torch.zeros(2, tiny_cfg.hidden), torch.full((2,), m.no_previous))
        z, _, _ = m.encode(obs)
    assert torch.isfinite(f).all() and torch.isfinite(v).all() and torch.isfinite(h).all()
    assert torch.isinf(s[:, : m.n_slots]).all() and torch.isfinite(s[:, m.n_slots :]).all()
    assert torch.allclose(z, torch.relu(m.pool_out.bias).expand_as(z))


def test_name_dropout_zeroes_whole_tokens():
    """Name dropout works on stored tokens (a row each): a token loses both its ids or neither, at the rate."""
    g = torch.Generator().manual_seed(3)
    ids = torch.randint(1, 8192, (20000, 6), generator=g, dtype=torch.int32)
    before = ids.clone()
    drop_names(ids, 0.2, g)
    dropped = (ids == 0).all(-1)
    assert ((ids == 0).any(-1) == dropped).all()
    assert torch.equal(ids[~dropped], before[~dropped])
    assert 0.18 < float(dropped.float().mean()) < 0.22
    drop_names(ids, 0.0, g)
    assert (ids == 0).all(-1).sum() == dropped.sum()


def test_mac_budget():
    """§7: the default fits 3 M multiply-adds at the typical fight position and 10 M with every token present."""
    spec = parse_spec(default_spec())
    assert set(TYPICAL) == set(spec.names)
    r = budget_report(spec, ModelConfig())
    assert r["typical_fight"] <= BUDGET_TYPICAL
    assert r["max"] <= BUDGET_MAX
    # The large size exists and costs more.
    large = ModelConfig(**SIZES["large"])
    assert budget_report(spec, large)["typical_fight"] > r["typical_fight"]


def test_macs_follow_the_spec():
    raw = default_spec()
    spec = parse_spec(raw)
    cfg = ModelConfig()
    base = count_macs(spec, cfg, TYPICAL)
    wider = dict(raw, sets={**raw["sets"], "objects": dict(raw["sets"]["objects"], width=raw["sets"]["objects"]["width"] + 10)})
    w = count_macs(parse_spec(wider), cfg, TYPICAL)
    assert w["fight"] - base["fight"] == TYPICAL["objects"] * 10 * cfg.token_hidden
    # Absent tokens cost nothing.
    fewer = dict(TYPICAL, objects=TYPICAL["objects"] - 1)
    assert count_macs(spec, cfg, fewer)["fight"] < base["fight"]
    with pytest.raises(ValueError):
        count_macs(spec, cfg, {"general": 1})


def test_macs_match_a_direct_count(tiny_cfg):
    """Count the multiply-adds of one decision by hooking every Linear, GRU and attention product."""
    spec = parse_spec(tiny_spec())
    cfg = dataclasses.replace(tiny_cfg)
    Model(spec, cfg)  # builds for this spec
    present = {"general": 1, "objects": 2, "slots": 2, "rules": 3}
    obs = {"x": {}, "ids": {}, "mask": {}}
    for s in spec.sets:
        obs["x"][s.name] = torch.randn(1, s.count, s.width)
        obs["ids"][s.name] = torch.ones(1, s.count, s.ids, dtype=torch.int32)
        mask = torch.zeros(1, s.count, dtype=torch.bool)
        mask[0, : present[s.name]] = True
        obs["mask"][s.name] = mask
    N = sum(present.values())
    Nt = N - present["rules"]  # rules is pool-only: not in the transformer
    d = cfg.d
    # Linear layers per present token / once, as the Rust port runs them.
    enc = sum(present[s.name] * (s.width * cfg.token_hidden + cfg.token_hidden * d) for s in spec.sets)
    attn = Nt * 4 * d * d + 2 * Nt * Nt * d
    ffn = Nt * 2 * d * cfg.ffn
    singles = sum(1 for s in spec.sets if s.count == 1)
    pool = N * d * cfg.pool_heads + cfg.pool_heads * N * d + (cfg.pool_heads + singles) * d * cfg.summary
    gru = 3 * (cfg.summary * cfg.hidden + cfg.hidden * cfg.hidden)
    core = cfg.hidden + cfg.summary
    fight = core * cfg.head_hidden + cfg.head_hidden * spec.fight_actions
    assert count_macs(spec, cfg, present)["fight"] == enc + cfg.layers * (attn + ffn) + pool + gru + fight


def test_pool_only_sets_skip_the_transformer(tiny_cfg):
    """A pool-only set's tokens change the summary but not the other tokens' transformer outputs."""
    spec = parse_spec(tiny_spec())
    m = Model(spec, tiny_cfg)
    assert tiny_cfg.pool_only == ["rules"]
    g = torch.Generator().manual_seed(4)
    obs = random_obs(spec, 4, g)
    obs["mask"]["general"][:] = True
    obs["mask"]["rules"][:] = True
    with torch.no_grad():
        z1, slots1, _ = m.encode(obs)
        obs["x"]["rules"] += 3.0
        z2, slots2, _ = m.encode(obs)
    assert torch.equal(slots1, slots2)
    assert not torch.allclose(z1, z2)


def test_packing_is_the_same_computation():
    """Cutting sets to their present tokens (the default) computes what every slot at fixed shapes does."""
    spec = parse_spec(default_spec())
    m = Model(spec, ModelConfig())
    g = torch.Generator().manual_seed(5)
    obs = random_obs(spec, 16, g, present=0.3)
    h = torch.randn(16, m.cfg.hidden, generator=g)
    prev = random_previous(m, (16,), g)
    with torch.no_grad():
        a = m.step(obs, h, prev)
        m.pack = False
        b = m.step(obs, h, prev)
    for x, y in zip(a, b):
        assert torch.allclose(torch.nan_to_num(x, neginf=-1e9), torch.nan_to_num(y, neginf=-1e9), atol=1e-5)


def test_the_gru_reads_the_previous_action(tiny_cfg):
    """The previous decision's action, one-hot over the fight's actions, the screen's and none, is the GRU's input
    after the summary: it changes the GRU state and so the outputs; the summary is the same."""
    spec = parse_spec(tiny_spec())
    m = Model(spec, tiny_cfg)
    assert m.n_previous == spec.fight_actions + spec.screen_actions + 1 and m.no_previous == m.n_previous - 1
    assert m.gru.weight_ih_l0.shape == (3 * tiny_cfg.hidden, tiny_cfg.summary + m.n_previous)
    assert m.architecture()["previous_action"] == m.n_previous
    g = torch.Generator().manual_seed(6)
    obs = random_obs(spec, 1, g)
    obs = {k: {n: v.expand(m.n_previous, *v.shape[1:]) for n, v in d.items()} for k, d in obs.items()}
    h = torch.randn(1, tiny_cfg.hidden, generator=g).expand(m.n_previous, -1)
    with torch.no_grad():
        _, _, _, h1 = m.step(obs, h, torch.arange(m.n_previous))
    assert len({tuple(r.tolist()) for r in h1}) == m.n_previous  # each previous action its own state


def test_warm_start_plays_as_the_network_it_came_from(tiny_cfg):
    """A network that read no previous action (its GRU's input the summary alone, as the first cloud run's),
    warm-started: every other tensor copied, the input normalization among them, the previous action's columns
    zero, so it computes what that network did whatever the previous action; a checkpoint of this network's own
    widths is copied whole; other widths are refused."""
    spec = parse_spec(tiny_spec())
    src = Model(spec, tiny_cfg)
    g = torch.Generator().manual_seed(7)
    with torch.no_grad():
        for p in src.parameters():
            p.add_(torch.randn(p.shape, generator=g) * 0.3)
        for name, b in src.named_buffers():
            b.copy_(torch.rand(b.shape, generator=g) + 0.5)
    old = {k: v.clone() for k, v in src.state_dict().items()}
    old["gru.weight_ih_l0"] = old["gru.weight_ih_l0"][:, : tiny_cfg.summary].clone()  # read no previous action

    def old_step(obs, h):
        """The old network's decision: the GRU over the summary alone."""
        z, slot_tok, slot_present = src.encode(obs)
        gru = torch.nn.GRU(tiny_cfg.summary, tiny_cfg.hidden, batch_first=True)
        gru.load_state_dict({k[4:]: v for k, v in old.items() if k.startswith("gru.")})
        _, h = gru(z.unsqueeze(1), h.unsqueeze(0))
        return (*src.heads(h[0], z, slot_tok, slot_present), h[0])

    warm = Model(spec, tiny_cfg)
    warm_start(warm, old)
    for k, v in warm.state_dict().items():
        if k == "gru.weight_ih_l0":
            assert torch.equal(v[:, : tiny_cfg.summary], old[k]) and not v[:, tiny_cfg.summary :].any()
        else:
            assert torch.equal(v, old[k]), k
    obs = random_obs(spec, 12, g)
    h = torch.randn(12, tiny_cfg.hidden, generator=g)
    with torch.no_grad():
        want = old_step(obs, h)
        for prev in (torch.full((12,), warm.no_previous), random_previous(warm, (12,), g)):
            got = warm.step(obs, h, prev)
            for (f, x), y in zip(zip(("fight", "screen", "value", "h"), got), want):
                assert torch.allclose(torch.nan_to_num(x, neginf=-1e9), torch.nan_to_num(y, neginf=-1e9), atol=1e-6), f
    again = Model(spec, tiny_cfg)
    warm_start(again, src.state_dict())
    assert all(torch.equal(v, src.state_dict()[k]) for k, v in again.state_dict().items())
    wider = Model(spec, dataclasses.replace(tiny_cfg, hidden=tiny_cfg.hidden + 2))
    with pytest.raises(ValueError, match="warm start"):
        warm_start(wider, old)
