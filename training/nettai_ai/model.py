"""The network (§7): per-set token encoders, a small transformer, an attention pool, a GRU and three heads.

Only Linear, LayerNorm, ReLU, multi-head attention with masks, an embedding lookup-sum, a GRU cell's equations
and softmax are used, so the Rust forward pass can follow MODEL.md step by step. Every width and count comes from the spec.

Shapes below use N for a batch of decisions, T for the total token slots (the sum of the sets' counts).
"""

from __future__ import annotations

import dataclasses
import json
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

# §7's budget profile: a typical fight position, by set name (the pool's sets as the trainer reads them: its chips
# set split into hand and used). Objects and hand as the coordinator put them (2026-10-23); rules as the pool
# shows them (43 in EXE6, 79 in EXE5, 49 in EXE4, every decision); the attack log is in the panels' tokens.
TYPICAL = {
    "general": 1,
    "panels": 18,
    "objects": 10,
    "hand": 6,
    "used": 1,
    "slots": 0,
    "rules": 57,
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
        d = cfg.d
        for n in cfg.pool_only:
            spec.set(n)  # a name the spec doesn't have is an error
        if cfg.pointer_set in cfg.pool_only:
            raise ValueError("model: the pointer set can't be pool-only")
        self.pack = True  # a run-time choice, not part of the weights (see encode)
        self.enc = nn.ModuleDict({s.name: SetEncoder(s, cfg) for s in spec.sets})
        self.names = nn.Embedding(spec.buckets, d)
        nn.init.normal_(self.names.weight, std=NAME_INIT_STD)
        self.blocks = nn.ModuleList([Block(cfg) for _ in range(cfg.layers)])
        self.ln_f = nn.LayerNorm(d, eps=LN_EPS)
        self.pool_score = nn.Linear(d, cfg.pool_heads)
        # Sets of one token (the global state, the other side's last chip) are also read directly, in spec order.
        singles = sum(1 for s in spec.sets if s.count == 1)
        self.pool_out = nn.Linear((cfg.pool_heads + singles) * d, cfg.summary)
        # One GRU layer (PyTorch's GRU equations, gates r, z, n). nn.GRU rather than nn.GRUCell so sequences
        # in training run as one fused call (cuDNN on CUDA); one decision is a sequence of one.
        self.gru = nn.GRU(cfg.summary, cfg.hidden, batch_first=True)
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
        """obs with batch shape [N] → summary z [N, summary], slot tokens [N, slots, d], slot present [N, slots].

        With `self.pack` (the default), each set but the pointer set is first cut to the batch's most present
        tokens, present ones first in their order (no position is read, so this is the same computation; the
        pointer set keeps its slots, which are its actions). Without it, every slot is computed at fixed shapes
        (for compiled or graph-captured runs)."""
        sets = self.spec.sets
        keep = [s.count for s in sets]
        if self.pack:
            counts = torch.stack([obs["mask"][s.name].sum(1).max() for s in sets]).tolist()
            keep = [s.count if s.name == self.cfg.pointer_set else max(1, int(c)) for s, c in zip(sets, counts)]
        toks, masks, tmasks = [], [], []
        at = 0
        single_at, slot_at = [], 0
        for s, k in zip(sets, keep):
            x, ids, m = obs["x"][s.name], obs["ids"][s.name], obs["mask"][s.name]
            if k < s.count:
                order = torch.argsort((~m).to(torch.uint8), dim=1, stable=True)[:, :k]
                x = x.gather(1, order.unsqueeze(-1).expand(-1, -1, x.shape[-1]))
                ids = ids.gather(1, order.unsqueeze(-1).expand(-1, -1, ids.shape[-1]))
                m = m.gather(1, order)
            t = self.enc[s.name](x)
            if s.ids:
                e = self.names(ids) * (ids != 0).unsqueeze(-1)
                t = t + e.sum(-2)
            toks.append(t * m.unsqueeze(-1))
            masks.append(m)
            tmasks.append(m if s.name not in self.cfg.pool_only else torch.zeros_like(m))
            if s.count == 1:
                single_at.append(at)
            if s.name == self.cfg.pointer_set:
                slot_at = at
            at += k
        t = torch.cat(toks, 1)  # [N, T, d]
        mask = torch.cat(masks, 1)  # [N, T]
        if self.blocks:
            t = self._transform(t, torch.cat(tmasks, 1))
        t = self.ln_f(t)
        w = masked_softmax(self.pool_score(t), mask.unsqueeze(-1), 1)  # [N, T, P]
        pooled = torch.einsum("ntp,ntd->npd", w, t).flatten(1)  # [N, P·d], head-major
        direct = [t[:, i] * mask[:, i : i + 1] for i in single_at]  # absent → zeros
        z = F.relu(self.pool_out(torch.cat([pooled, *direct], 1)))
        sl = slice(slot_at, slot_at + self.n_slots)
        return z, t[:, sl], mask[:, sl]

    def _transform(self, t: torch.Tensor, mask: torch.Tensor) -> torch.Tensor:
        """The transformer layers over the present tokens of the sets it reads (mask). They are moved to the
        front of each row and the batch is cut to its most present tokens (the network has no positions, so
        this is the same computation, only cheaper), then put back; the other tokens come out as they went in.

        Keys are the present tokens. A decision with no token at all attends to its first slot, so no softmax
        row is empty; the pool reads nothing of it either way (z = relu(pool_out.bias) then)."""
        N, T, d = t.shape
        tp = max(1, int(mask.sum(1).max())) if self.pack else T
        order = torch.argsort((~mask).to(torch.uint8), dim=1, stable=True)[:, :tp]  # present first, in order
        idx = order.unsqueeze(-1).expand(N, tp, d)
        x = t.gather(1, idx)
        m = mask.gather(1, order)
        attend = m.clone()
        attend[:, 0] |= ~m.any(1)
        attend = attend[:, None, None, :]
        for b in self.blocks:
            x = b(x, attend)
        x = torch.where(m.unsqueeze(-1), x, t.gather(1, idx))  # padding slots keep what they were
        return t.scatter(1, idx, x)

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
        _, h = self.gru(z.unsqueeze(1), h.unsqueeze(0).contiguous())
        h = h[0]
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
        # The GRU runs over segments cut wherever any row resets: inside a segment no row resets, so each is
        # one fused call; at a segment's start the rows that reset begin from zero.
        cuts = [0] + [int(i) for i in reset[:, 1:].any(0).nonzero().squeeze(-1).add(1).tolist()] + [L]
        h = h0
        hs = []
        for a, b in zip(cuts[:-1], cuts[1:]):
            h = h * (~reset[:, a]).unsqueeze(-1).to(h.dtype)
            out, hn = self.gru(zs[:, a:b], h.unsqueeze(0).contiguous())
            h = hn[0]
            hs.append(out)
        hs = torch.cat(hs, 1)
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
                # Sets the trainer cut from one of the pool's (name: [[part, first token, count], ...]).
                "split": self.spec.raw.get("split", {}),
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
        if sp.get("split"):
            raw["split"] = sp["split"]
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
    Nt = sum(v for k, v in n.items() if k not in cfg.pool_only)  # the transformer's tokens
    encoder = sum(n[s.name] * (s.width * cfg.token_hidden + cfg.token_hidden * d) for s in spec.sets)
    layers = cfg.layers * (Nt * (4 * d * d + 2 * d * f) + 2 * Nt * Nt * d)
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
        "transformer_tokens": Nt,
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
        "typical_transformer_tokens": t["transformer_tokens"],
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
    ap.add_argument("--spec", default=None, help="a spec JSON file (default: the fake's, laid out as the pool's)")
    a = ap.parse_args()
    cfg = load_config(a.config, size=a.size).model
    raw = json.load(open(a.spec)) if a.spec else default_spec()
    spec = parse_spec(raw)
    r = budget_report(spec, cfg)
    m = Model(spec, cfg)
    names = m.names.weight.numel()
    print(f"typical fight decision  {r['typical_fight'] / 1e6:6.2f} M MACs  ({r['typical_tokens']} tokens, {r['typical_transformer_tokens']} in the transformer)")
    print(f"typical screen decision {r['typical_screen'] / 1e6:6.2f} M MACs")
    print(f"max                     {r['max'] / 1e6:6.2f} M MACs  ({r['max_tokens']} tokens)")
    print(f"parameters              {count_params(m):,} ({names:,} in the name table)")
    print(f"weights                 {count_params(m) * 4 / 2**20:.1f} MiB float32")


if __name__ == "__main__":
    _main()
