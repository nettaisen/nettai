"""The network (§7): per-set token encoders, a small transformer, an attention pool, a GRU and three heads.

Only Linear, LayerNorm, ReLU, multi-head attention with masks, an embedding lookup-sum, GRUCell and softmax
are used, so the Rust forward pass can follow MODEL.md step by step. Every width and count comes from the spec.

Shapes below use N for a batch of decisions, T for the total token slots (the sum of the sets' counts).
"""

from __future__ import annotations

import dataclasses
import json
import math
from dataclasses import dataclass
from typing import Any

import torch
import torch.nn.functional as F
from torch import nn

from .config import Level, ModelConfig
from .spec import Spec, TokenSet, parse_spec

FORMAT = "nettai-ai"
FORMAT_VERSION = 1
LN_EPS = 1e-5
NAME_INIT_STD = 0.05

# §7's budget profile: a typical fight position, by set name (the coordinator's numbers, 2026-10-23).
TYPICAL = {
    "global": 1,
    "panels": 18,
    "objects": 10,
    "attack_log": 18,
    "hand": 6,
    "slots": 0,
    "used": 1,
    "rules": 10,
    "forms": 2,
}
BUDGET_TYPICAL = 3_000_000
BUDGET_MAX = 10_000_000

Obs = dict[str, Any]  # {"x": {set: f32 [..., C, W]}, "ids": {set: i32 [..., C, I]}, "mask": {set: bool [..., C]}}


class SetEncoder(nn.Module):
    """One token set's tokens to width d: normalize, two Linears with a ReLU between, plus the set's type."""

    def __init__(self, ts: TokenSet, cfg: ModelConfig):
        super().__init__()
        self.fc1 = nn.Linear(ts.width, cfg.token_hidden)
        self.fc2 = nn.Linear(cfg.token_hidden, cfg.d)
        self.type_emb = nn.Parameter(torch.randn(cfg.d) * 0.02)
        # Input normalization (set by the trainer from running statistics; identity until then).
        self.register_buffer("norm_mean", torch.zeros(ts.width))
        self.register_buffer("norm_inv_std", torch.ones(ts.width))
        self.clip = cfg.norm_clip

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        xn = ((x - self.norm_mean) * self.norm_inv_std).clamp(-self.clip, self.clip)
        return self.fc2(F.relu(self.fc1(xn))) + self.type_emb


class Block(nn.Module):
    """A pre-LayerNorm transformer layer: t += Attn(LN1(t)); t += FFN(LN2(t))."""

    def __init__(self, cfg: ModelConfig):
        super().__init__()
        d = cfg.d
        if d % cfg.heads:
            raise ValueError(f"model: d={d} is not a multiple of heads={cfg.heads}")
        self.heads = cfg.heads
        self.ln1 = nn.LayerNorm(d, eps=LN_EPS)
        self.q = nn.Linear(d, d)
        self.k = nn.Linear(d, d)
        self.v = nn.Linear(d, d)
        self.o = nn.Linear(d, d)
        self.ln2 = nn.LayerNorm(d, eps=LN_EPS)
        self.ff1 = nn.Linear(d, cfg.ffn)
        self.ff2 = nn.Linear(cfg.ffn, d)

    def forward(self, t: torch.Tensor, attend: torch.Tensor) -> torch.Tensor:
        """t [N, T, d]; attend [N, 1, 1, T] bool, true where a key may be attended to."""
        N, T, d = t.shape
        h, dh = self.heads, d // self.heads
        a = self.ln1(t)
        q = self.q(a).view(N, T, h, dh).transpose(1, 2)
        k = self.k(a).view(N, T, h, dh).transpose(1, 2)
        v = self.v(a).view(N, T, h, dh).transpose(1, 2)
        # softmax(q·kᵀ / sqrt(dh), over attended keys) · v, per head.
        o = F.scaled_dot_product_attention(q, k, v, attn_mask=attend)
        t = t + self.o(o.transpose(1, 2).reshape(N, T, d))
        return t + self.ff2(F.relu(self.ff1(self.ln2(t))))


def masked_softmax(scores: torch.Tensor, mask: torch.Tensor, dim: int) -> torch.Tensor:
    """Softmax over the entries where mask is true; all zeros where none is (never NaN)."""
    any_ = mask.any(dim, keepdim=True)
    s = scores.masked_fill(~mask, float("-inf")).masked_fill(~any_, 0.0)
    return torch.softmax(s, dim) * mask


class Model(nn.Module):
    def __init__(self, spec: Spec, cfg: ModelConfig):
        super().__init__()
        self.spec = spec
        self.cfg = cfg
        self.set_names = spec.names
        pointer = spec.set(cfg.pointer_set)
        self.n_slots = pointer.count
        self.n_keys = spec.screen_actions - self.n_slots
        if self.n_keys < 0:
            raise ValueError(f"model: screen_actions {spec.screen_actions} < the {pointer.name} set's {pointer.count}")
        start = 0
        for s in spec.sets:
            if s.name == cfg.pointer_set:
                self.slot_start = start
            start += s.count
        d = cfg.d
        self.enc = nn.ModuleDict({s.name: SetEncoder(s, cfg) for s in spec.sets})
        self.names = nn.Embedding(spec.buckets, d)
        nn.init.normal_(self.names.weight, std=NAME_INIT_STD)
        self.blocks = nn.ModuleList([Block(cfg) for _ in range(cfg.layers)])
        self.ln_f = nn.LayerNorm(d, eps=LN_EPS)
        self.pool_score = nn.Linear(d, cfg.pool_heads)
        # Sets of one token (the global state, the other side's last chip) are also read directly, in spec order.
        self.single = [i for i, s in enumerate(spec.sets) if s.count == 1]
        self.single_start = [sum(s.count for s in spec.sets[:i]) for i in self.single]
        self.pool_out = nn.Linear((cfg.pool_heads + len(self.single)) * d, cfg.summary)
        self.gru = nn.GRUCell(cfg.summary, cfg.hidden)
        core = cfg.hidden + cfg.summary
        self.fight1 = nn.Linear(core, cfg.head_hidden)
        self.fight2 = nn.Linear(cfg.head_hidden, spec.fight_actions)
        self.key1 = nn.Linear(core, cfg.head_hidden)
        self.key2 = nn.Linear(cfg.head_hidden, self.n_keys)
        self.ptr1 = nn.Linear(d + core, cfg.pointer_hidden)
        self.ptr2 = nn.Linear(cfg.pointer_hidden, 1)
        self.value1 = nn.Linear(core, cfg.head_hidden)
        self.value2 = nn.Linear(cfg.head_hidden, 1)
        for layer in (self.fight2, self.key2, self.ptr2):  # near-uniform policy at the start
            nn.init.normal_(layer.weight, std=0.01)
            nn.init.zeros_(layer.bias)

    # The forward computation (MODEL.md) ----------------------------------------------------------------

    def encode(self, obs: Obs) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """obs with batch shape [N] → summary z [N, summary], slot tokens [N, slots, d], slot present [N, slots]."""
        toks, masks = [], []
        for s in self.spec.sets:
            m = obs["mask"][s.name]
            t = self.enc[s.name](obs["x"][s.name])
            if s.ids:
                ids = obs["ids"][s.name]
                e = self.names(ids) * (ids != 0).unsqueeze(-1)
                t = t + e.sum(-2)
            toks.append(t * m.unsqueeze(-1))
            masks.append(m)
        t = torch.cat(toks, 1)  # [N, T, d]
        mask = torch.cat(masks, 1)  # [N, T]
        # Keys: the present tokens. A decision with no token at all attends to slot 0, so no softmax row is
        # empty; its pool below reads nothing either way (z = relu(pool_out.bias)).
        attend = mask.clone()
        attend[:, 0] |= ~mask.any(1)
        attend = attend[:, None, None, :]
        for b in self.blocks:
            t = b(t, attend)
        t = self.ln_f(t)
        w = masked_softmax(self.pool_score(t), mask.unsqueeze(-1), 1)  # [N, T, P]
        pooled = torch.einsum("ntp,ntd->npd", w, t).flatten(1)  # [N, P·d], head-major
        direct = [t[:, i] * mask[:, i : i + 1] for i in self.single_start]  # absent → zeros
        z = F.relu(self.pool_out(torch.cat([pooled, *direct], 1)))
        sl = slice(self.slot_start, self.slot_start + self.n_slots)
        return z, t[:, sl], mask[:, sl]

    def heads(
        self, h: torch.Tensor, z: torch.Tensor, slot_tok: torch.Tensor, slot_present: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """core = [h, z] → fight logits [N, 20], screen logits [N, 19] (absent slots -inf), value [N]."""
        core = torch.cat([h, z], -1)
        fight = self.fight2(F.relu(self.fight1(core)))
        keys = self.key2(F.relu(self.key1(core)))
        # ptr1 over [slot token, core]: the core's part is computed once for all slots.
        d = self.cfg.d
        w = self.ptr1.weight
        u = F.linear(slot_tok, w[:, :d]) + (F.linear(core, w[:, d:], self.ptr1.bias)).unsqueeze(-2)
        slots = self.ptr2(F.relu(u)).squeeze(-1).masked_fill(~slot_present, float("-inf"))
        value = self.value2(F.relu(self.value1(core))).squeeze(-1)
        return fight, torch.cat([slots, keys], -1), value

    def step(self, obs: Obs, h: torch.Tensor):
        """One decision per row: obs [N], h [N, H] → fight [N, 20], screen [N, 19], value [N], h' [N, H]."""
        z, slot_tok, slot_present = self.encode(obs)
        h = self.gru(z, h)
        fight, screen, value = self.heads(h, z, slot_tok, slot_present)
        return fight, screen, value, h

    def sequence(self, obs: Obs, h0: torch.Tensor, reset: torch.Tensor):
        """Sequences: obs [B, L] (flattened to [B·L] inside), h0 [B, H], reset [B, L] (zero h before that decision).

        Returns fight [B, L, 20], screen [B, L, 19], value [B, L], and the GRU states after each decision [B, L, H].
        """
        B, L = reset.shape
        flat = map_obs(obs, lambda t: t.flatten(0, 1))
        z, slot_tok, slot_present = self.encode(flat)
        zs = z.view(B, L, -1)
        h = h0
        hs = []
        for i in range(L):
            h = h * (~reset[:, i]).unsqueeze(-1).to(h.dtype)
            h = self.gru(zs[:, i], h)
            hs.append(h)
        hs = torch.stack(hs, 1)
        fight, screen, value = self.heads(hs.flatten(0, 1), z, slot_tok, slot_present)
        return fight.view(B, L, -1), screen.view(B, L, -1), value.view(B, L), hs

    # Description -------------------------------------------------------------------------------------

    def architecture(self) -> dict[str, Any]:
        return {
            "format": FORMAT,
            "format_version": FORMAT_VERSION,
            "model": dataclasses.asdict(self.cfg),
            "activation": "relu",
            "layer_norm_eps": LN_EPS,
            "slots": self.n_slots,
            "keys": self.n_keys,
            "spec": {
                "hash": self.spec.hash,
                "buckets": self.spec.buckets,
                "fight_actions": self.spec.fight_actions,
                "screen_actions": self.spec.screen_actions,
                "sets": [{"name": s.name, "count": s.count, "width": s.width, "ids": s.ids} for s in self.spec.sets],
            },
        }

    @staticmethod
    def from_architecture(arch: dict[str, Any] | str) -> "Model":
        if isinstance(arch, str):
            arch = json.loads(arch)
        if arch.get("format") != FORMAT or arch.get("format_version") != FORMAT_VERSION:
            raise ValueError(f"not a {FORMAT} v{FORMAT_VERSION} architecture: {arch.get('format')} {arch.get('format_version')}")
        sp = arch["spec"]
        raw = {k: sp[k] for k in ("hash", "buckets", "fight_actions", "screen_actions")}
        raw["sets"] = {s["name"]: {"count": s["count"], "width": s["width"], "ids": s["ids"]} for s in sp["sets"]}
        return Model(parse_spec(raw), ModelConfig(**arch["model"]))


def map_obs(obs: Obs, fn) -> Obs:
    return {k: {n: fn(t) for n, t in obs[k].items()} for k in ("x", "ids", "mask")}


def mask_logits(logits: torch.Tensor, legal: torch.Tensor) -> torch.Tensor:
    return logits.masked_fill(~legal, float("-inf"))


def gumbel_argmax(logits: torch.Tensor, generator: torch.Generator | None = None) -> torch.Tensor:
    """A sample from softmax(logits); -inf entries are never chosen."""
    u = torch.rand(logits.shape, generator=generator, device=logits.device).clamp_(1e-20, 1.0 - 1e-7)
    return (logits - torch.log(-torch.log(u))).argmax(-1)


def select(
    fight: torch.Tensor,
    screen: torch.Tensor,
    phase: torch.Tensor,
    level: Level | None = None,
    generator: torch.Generator | None = None,
) -> tuple[torch.Tensor, torch.Tensor]:
    """Actions for decisions whose logits are already masked. phase 1 picks from fight, 2 from screen.

    Returns (action, log-probability under the unmodified policy). With a level, the action follows its knobs
    (temperature, epsilon, greedy); without, it is a plain sample (training).
    """
    is_fight = (phase == 1).unsqueeze(-1)
    width = max(fight.shape[-1], screen.shape[-1])
    pad = lambda t: F.pad(t, (0, width - t.shape[-1]), value=float("-inf"))
    logits = torch.where(is_fight, pad(fight), pad(screen))
    if not torch.isfinite(logits).any(-1).all():
        raise RuntimeError("select: a decision has no legal action")
    if level is None:
        action = gumbel_argmax(logits, generator)
    elif level.greedy:
        action = logits.argmax(-1)
    else:
        action = gumbel_argmax(logits / level.temperature, generator)
        if level.epsilon > 0:
            uniform = gumbel_argmax(torch.where(torch.isfinite(logits), 0.0, float("-inf")), generator)
            explore = torch.rand(action.shape, generator=generator, device=action.device) < level.epsilon
            action = torch.where(explore, uniform, action)
    logp = torch.log_softmax(logits, -1).gather(-1, action.unsqueeze(-1)).squeeze(-1)
    if not torch.isfinite(logp).all():
        raise RuntimeError("select: an illegal action was chosen")
    return action, logp


# Cost ----------------------------------------------------------------------------------------------------


def count_macs(spec: Spec, cfg: ModelConfig, present: dict[str, int] | None = None) -> dict[str, int]:
    """Multiply-adds of one decision at play time, counting present tokens only (the Rust port skips absent
    ones). present maps every set to its present tokens; None means every slot is present (the max).

    Returns the fight decision's cost, the screen decision's (its slots as present says), and their parts.
    Additions, LayerNorm, softmax and activations are not multiply-adds and are left out.
    """
    if present is None:
        present = {s.name: s.count for s in spec.sets}
    missing = [s.name for s in spec.sets if s.name not in present]
    extra = [n for n in present if n not in spec.names]
    if missing or extra:
        raise ValueError(f"count_macs: the profile doesn't match the spec (missing {missing}, unknown {extra})")
    d, f, P = cfg.d, cfg.ffn, cfg.pool_heads
    n = {s.name: min(present[s.name], s.count) for s in spec.sets}
    N = sum(n.values())
    encoder = sum(n[s.name] * (s.width * cfg.token_hidden + cfg.token_hidden * d) for s in spec.sets)
    layers = cfg.layers * (N * (4 * d * d + 2 * d * f) + 2 * N * N * d)
    singles = sum(1 for s in spec.sets if s.count == 1)
    pool = N * d * P + P * N * d + (P + singles) * d * cfg.summary
    gru = 3 * (cfg.summary * cfg.hidden + cfg.hidden * cfg.hidden)
    core = cfg.hidden + cfg.summary
    fight = core * cfg.head_hidden + cfg.head_hidden * spec.fight_actions
    slots = n[cfg.pointer_set]
    keys = core * cfg.head_hidden + cfg.head_hidden * (spec.screen_actions - spec.set(cfg.pointer_set).count)
    pointer = core * cfg.pointer_hidden + slots * (d * cfg.pointer_hidden + cfg.pointer_hidden)
    trunk = encoder + layers + pool + gru
    return {
        "tokens": N,
        "encoder": encoder,
        "transformer": layers,
        "pool": pool,
        "gru": gru,
        "fight": trunk + fight,
        "screen": trunk + keys + pointer,
        "value_head": core * cfg.head_hidden + cfg.head_hidden,
    }


def budget_report(spec: Spec, cfg: ModelConfig, typical: dict[str, int] | None = None) -> dict[str, Any]:
    """§7's numbers: a typical fight decision, the same position on the custom screen (every slot present),
    and the max (every token present)."""
    typical = dict(TYPICAL if typical is None else typical)
    screen = dict(typical)
    screen[cfg.pointer_set] = spec.set(cfg.pointer_set).count
    t, s, m = count_macs(spec, cfg, typical), count_macs(spec, cfg, screen), count_macs(spec, cfg, None)
    return {
        "typical_fight": t["fight"],
        "typical_screen": s["screen"],
        "max": max(m["fight"], m["screen"]),
        "typical_tokens": t["tokens"],
        "max_tokens": m["tokens"],
    }


def count_params(model: nn.Module) -> int:
    return sum(p.numel() for p in model.parameters())


def _main() -> None:
    import argparse

    from .config import load_config
    from .fake import default_spec

    ap = argparse.ArgumentParser(description="Print a model size's cost per decision (MACs) and parameters.")
    ap.add_argument("--size", default=None)
    ap.add_argument("--config", default=None)
    ap.add_argument("--spec", default=None, help="a spec JSON file (default: the fake's)")
    a = ap.parse_args()
    cfg = load_config(a.config, size=a.size).model
    raw = json.load(open(a.spec)) if a.spec else default_spec()
    spec = parse_spec(raw)
    r = budget_report(spec, cfg)
    m = Model(spec, cfg)
    names = m.names.weight.numel()
    print(f"typical fight decision  {r['typical_fight'] / 1e6:6.2f} M MACs  ({r['typical_tokens']} tokens)")
    print(f"typical screen decision {r['typical_screen'] / 1e6:6.2f} M MACs")
    print(f"max                     {r['max'] / 1e6:6.2f} M MACs  ({r['max_tokens']} tokens)")
    print(f"parameters              {count_params(m):,} ({names:,} in the name table)")
    print(f"weights                 {count_params(m) * 4 / 2**20:.1f} MiB float32")


if __name__ == "__main__":
    _main()
