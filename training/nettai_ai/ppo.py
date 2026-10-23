"""Recurrent PPO over per-seat decision streams (§8): GAE, the clipped update, input normalization."""

from __future__ import annotations

import math

import torch
import torch.nn.functional as F

from .config import PPOConfig
from .model import Model, mask_logits
from .rollout import Rollout
from .spec import Spec


def compute_gae(
    reward: torch.Tensor,
    value: torch.Tensor,
    next_value: torch.Tensor,
    term: torch.Tensor,
    trunc: torch.Tensor,
    closed: torch.Tensor,
    gamma: float,
    lam: float,
) -> tuple[torch.Tensor, torch.Tensor]:
    """GAE over each stream's own decisions. All inputs are [S, T] along the stream's decision index.

    closed marks decisions whose reward and successor are known; the rest (the open last decision, unused
    slots) get advantage 0 and stop the recursion, so the decision before an open one is bootstrapped by
    next_value alone. A terminal decision has no bootstrap; a truncated one bootstraps from next_value (set by
    the collector to the decision's own value). Both end the chain.
    """
    S, T = reward.shape
    adv = torch.zeros_like(reward)
    last = torch.zeros(S, device=reward.device)
    for t in range(T - 1, -1, -1):
        end = term[:, t] | trunc[:, t]
        delta = reward[:, t] + gamma * next_value[:, t] * (~term[:, t]) - value[:, t]
        a = delta + gamma * lam * (~end) * last
        a = torch.where(closed[:, t], a, 0.0)
        adv[:, t] = a
        last = a
    return adv, adv + value


class RunningNorm:
    """Per-set, per-feature mean and variance of present tokens' floats (float64, merged per rollout)."""

    def __init__(self, spec: Spec, device: torch.device, eps: float = 1e-6):
        self.eps = eps
        self.count = {s.name: torch.zeros((), dtype=torch.float64, device=device) for s in spec.sets}
        self.mean = {s.name: torch.zeros(s.width, dtype=torch.float64, device=device) for s in spec.sets}
        self.m2 = {s.name: torch.zeros(s.width, dtype=torch.float64, device=device) for s in spec.sets}

    def update(self, ro: Rollout) -> None:
        written = ro.written()
        for n in self.mean:
            m = ro.mask[n] & written.unsqueeze(-1)
            x = ro.x[n][m].to(torch.float64)
            if x.shape[0] == 0 or x.shape[1] == 0:
                continue
            nb = torch.tensor(float(x.shape[0]), dtype=torch.float64, device=x.device)
            mb = x.mean(0)
            m2b = ((x - mb) ** 2).sum(0)
            na = self.count[n]
            tot = na + nb
            delta = mb - self.mean[n]
            self.mean[n] = self.mean[n] + delta * nb / tot
            self.m2[n] = self.m2[n] + m2b + delta**2 * na * nb / tot
            self.count[n] = tot

    def apply(self, model: Model) -> None:
        for n, enc in model.enc.items():
            c = self.count[n]
            if float(c) == 0:
                continue
            var = self.m2[n] / c
            enc.norm_mean.copy_(self.mean[n].to(torch.float32))
            enc.norm_inv_std.copy_((1.0 / torch.sqrt(var + self.eps)).to(torch.float32))

    def state_dict(self) -> dict:
        return {"count": self.count, "mean": self.mean, "m2": self.m2}

    def load_state_dict(self, d: dict) -> None:
        for k in ("count", "mean", "m2"):
            for n, v in d[k].items():
                getattr(self, k)[n] = v.to(getattr(self, k)[n].device)


def _entropy(logp: torch.Tensor) -> torch.Tensor:
    p = logp.exp()
    return -(p * logp.masked_fill(~torch.isfinite(logp), 0.0)).sum(-1)


def ppo_update(
    model: Model,
    opt: torch.optim.Optimizer,
    ro: Rollout,
    adv: torch.Tensor,
    ret: torch.Tensor,
    cfg: PPOConfig,
    generator: torch.Generator,
) -> dict[str, float]:
    """cfg.epochs passes over truncated-BPTT chunks of cfg.chunk decisions, each from its stored GRU state."""
    dev = ro.device
    L = cfg.chunk
    Tc = ro.Tc
    W = math.ceil(Tc / L)
    tgrid = torch.arange(W, device=dev)[:, None] * L + torch.arange(L, device=dev)[None, :]  # [W, L]
    tclamp = tgrid.clamp(max=Tc - 1)
    in_range = tgrid < Tc
    has = (ro.closed[:, tclamp] & in_range).any(-1)  # [S, W]
    cs, cw = has.nonzero(as_tuple=True)
    n_chunks = len(cs)
    stats = {"loss_policy": 0.0, "loss_value": 0.0, "entropy": 0.0, "approx_kl": 0.0, "clip_frac": 0.0, "grad_norm": 0.0}
    if n_chunks == 0:
        return stats | {"chunks": 0, "minibatches": 0}
    mbs = 0
    fa, sa = ro.legal_fight.shape[-1], ro.legal_screen.shape[-1]
    for _ in range(cfg.epochs):
        perm = torch.randperm(n_chunks, generator=generator, device="cpu").to(dev)
        for start in range(0, n_chunks, cfg.minibatch_chunks):
            mb = perm[start : start + cfg.minibatch_chunks]
            s = cs[mb][:, None]  # [B, 1]
            t = tclamp[cw[mb]]  # [B, L]
            valid = ro.closed[s, t] & in_range[cw[mb]]
            obs = ro.gather(s, t)
            h0 = ro.h_in[s[:, 0], t[:, 0]]
            reset = ro.ep_start[s, t]
            fight, screen, value, _ = model.sequence(obs, h0, reset)
            phase = ro.phase[s, t].to(torch.int64)
            is_f = valid & (phase == 1)
            is_s = valid & (phase == 2)
            # Only a row's own head is read; the other head's logits are replaced by zeros so nothing is NaN.
            lf = torch.where(is_f.unsqueeze(-1), mask_logits(fight, ro.legal_fight[s, t]), 0.0)
            ls = torch.where(is_s.unsqueeze(-1), mask_logits(screen, ro.legal_screen[s, t]), 0.0)
            logp_f = F.log_softmax(lf, -1)
            logp_s = F.log_softmax(ls, -1)
            a = ro.action[s, t]
            lp = torch.where(
                is_f,
                logp_f.gather(-1, a.clamp(0, fa - 1).unsqueeze(-1)).squeeze(-1),
                logp_s.gather(-1, a.clamp(0, sa - 1).unsqueeze(-1)).squeeze(-1),
            )
            ent = torch.where(is_f, _entropy(logp_f), _entropy(logp_s))
            nvalid = valid.sum().clamp(min=1)
            vmean = lambda x: (x * valid).sum() / nvalid
            old_lp = ro.logp[s, t]
            log_ratio = torch.where(valid, lp - old_lp, 0.0)
            ratio = log_ratio.exp()
            A = adv[s, t]
            mu = vmean(A)
            sd = torch.sqrt(vmean((A - mu) ** 2) + 1e-8)
            A = (A - mu) / sd
            pg = -torch.minimum(ratio * A, ratio.clamp(1 - cfg.clip, 1 + cfg.clip) * A)
            old_v = ro.value[s, t]
            R = ret[s, t]
            v_clip = old_v + (value - old_v).clamp(-cfg.value_clip, cfg.value_clip)
            vl = 0.5 * torch.maximum((value - R) ** 2, (v_clip - R) ** 2)
            loss_pg, loss_v, entropy = vmean(pg), vmean(vl), vmean(ent)
            loss = loss_pg + cfg.value_coef * loss_v - cfg.entropy * entropy
            opt.zero_grad(set_to_none=True)
            loss.backward()
            gn = torch.nn.utils.clip_grad_norm_(model.parameters(), cfg.max_grad_norm)
            opt.step()
            with torch.no_grad():
                stats["loss_policy"] += float(loss_pg)
                stats["loss_value"] += float(loss_v)
                stats["entropy"] += float(entropy)
                stats["approx_kl"] += float(vmean((ratio - 1) - log_ratio))
                stats["clip_frac"] += float(vmean(((ratio - 1).abs() > cfg.clip).float()))
                stats["grad_norm"] += float(gn)
            mbs += 1
    out = {k: v / mbs for k, v in stats.items()}
    with torch.no_grad():
        closed = ro.closed
        rv = ret[closed]
        vv = ro.value[closed]
        var = rv.var() if rv.numel() > 1 else torch.tensor(0.0)
        out["explained_variance"] = float(1 - (rv - vv).var() / var) if float(var) > 0 else 0.0
        out["value_mean"] = float(vv.mean()) if vv.numel() else 0.0
        out["adv_mean"] = float(adv[closed].mean()) if vv.numel() else 0.0
    out["chunks"] = n_chunks
    out["minibatches"] = mbs
    out["decisions"] = int(ro.closed.sum())
    return out
