"""Recurrent PPO over per-seat decision streams (§8): GAE, the clipped update, input normalization."""

from __future__ import annotations

import math

import torch
import torch.nn.functional as F

from .config import PPOConfig
from .device import upload
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
    """Per-set, per-feature mean and variance of present tokens' floats. Each rollout's moments are taken on
    the device (float32) and merged here on the CPU in float64 (MPS has no float64)."""

    def __init__(self, spec: Spec, device: torch.device, eps: float = 1e-6):
        self.eps = eps
        f64 = dict(dtype=torch.float64)
        self.count = {s.name: torch.zeros((), **f64) for s in spec.sets}
        self.mean = {s.name: torch.zeros(s.width, **f64) for s in spec.sets}
        self.m2 = {s.name: torch.zeros(s.width, **f64) for s in spec.sets}

    def update(self, ro: Rollout) -> None:
        """Merge the moments of the rollout's stored tokens (every stored decision's present tokens)."""
        for n in self.mean:
            store = ro.tokens[n]
            x = store.x[: store.n]
            if x.shape[0] == 0 or x.shape[1] == 0:
                continue
            mb_dev = x.mean(0)
            m2b = ((x - mb_dev) ** 2).sum(0).cpu().to(torch.float64)
            mb = mb_dev.cpu().to(torch.float64)
            nb = torch.tensor(float(x.shape[0]), dtype=torch.float64)
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
                getattr(self, k)[n] = v.to("cpu", torch.float64)


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
    """cfg.epochs passes over truncated-BPTT chunks of cfg.chunk decisions, each from its stored GRU state.

    adv and ret are [S, Tc] on the CPU (compute_gae over the rollout's tensors). Each decision is replayed with the
    previous action the behavior network read, as stored. The chunks are chosen on the CPU
    and each minibatch's indices go to the device without waiting; the statistics stay on the device until the
    end. The ratio is against the behavior policy, whose log-probabilities and values the rollout stores; the
    first epoch's mean ratio and KL (behavior_ratio, behavior_kl) say how far the learner had moved from it."""
    dev = ro.device
    L = cfg.chunk
    Tc = ro.Tc
    W = math.ceil(Tc / L)
    tgrid = torch.arange(W)[:, None] * L + torch.arange(L)[None, :]  # [W, L]
    tclamp = tgrid.clamp(max=Tc - 1)
    in_range = tgrid < Tc
    has = (ro.closed[:, tclamp] & in_range).any(-1)  # [S, W]
    cs, cw = has.nonzero(as_tuple=True)
    n_chunks = len(cs)
    names = ["loss_policy", "loss_value", "entropy", "approx_kl", "clip_frac", "grad_norm"]
    if n_chunks == 0:
        return {k: 0.0 for k in names} | {"behavior_ratio": 1.0, "behavior_kl": 0.0, "chunks": 0, "minibatches": 0}
    up = lambda t: upload(t, dev)
    legal_f, legal_s, phase_d = up(ro.legal_fight), up(ro.legal_screen), up(ro.phase)
    action_d, logp_d, value_d, reset_d = up(ro.action), up(ro.logp), up(ro.value), up(ro.ep_start)
    previous_d = up(ro.previous)
    adv_d, ret_d = up(adv), up(ret)
    fa, sa = legal_f.shape[-1], legal_s.shape[-1]
    acc = torch.zeros(len(names), device=dev)
    first = torch.zeros(2, device=dev)
    mbs = first_mbs = 0
    for epoch in range(cfg.epochs):
        perm = torch.randperm(n_chunks, generator=generator, device="cpu")
        for start in range(0, n_chunks, cfg.minibatch_chunks):
            mb = perm[start : start + cfg.minibatch_chunks]
            sc = cs[mb][:, None].expand(-1, L)  # [B, L]
            tc = tclamp[cw[mb]]
            vc = ro.closed[sc, tc] & in_range[cw[mb]]
            obs = ro.obs(sc, tc, vc)
            idx = up(torch.stack([sc, tc, vc.to(torch.int64)]))
            s, t, valid = idx[0], idx[1], idx[2].bool()
            h0 = ro.h_in[s[:, 0], t[:, 0]]
            reset = reset_d[s, t]
            fight, screen, value, _ = model.sequence(obs, h0, reset, previous_d[s, t])
            phase = phase_d[s, t].to(torch.int64)
            is_f = valid & (phase == 1)
            is_s = valid & (phase == 2)
            # Only a row's own head is read; the other head's logits are replaced by zeros so nothing is NaN.
            lf = torch.where(is_f.unsqueeze(-1), mask_logits(fight, legal_f[s, t]), 0.0)
            ls = torch.where(is_s.unsqueeze(-1), mask_logits(screen, legal_s[s, t]), 0.0)
            logp_f = F.log_softmax(lf, -1)
            logp_s = F.log_softmax(ls, -1)
            a = action_d[s, t]
            lp = torch.where(
                is_f,
                logp_f.gather(-1, a.clamp(0, fa - 1).unsqueeze(-1)).squeeze(-1),
                logp_s.gather(-1, a.clamp(0, sa - 1).unsqueeze(-1)).squeeze(-1),
            )
            ent = torch.where(is_f, _entropy(logp_f), _entropy(logp_s))
            nvalid = valid.sum().clamp(min=1)
            vmean = lambda x: (x * valid).sum() / nvalid
            log_ratio = torch.where(valid, lp - logp_d[s, t], 0.0)
            ratio = log_ratio.exp()
            A = adv_d[s, t]
            mu = vmean(A)
            sd = torch.sqrt(vmean((A - mu) ** 2) + 1e-8)
            A = (A - mu) / sd
            pg = -torch.minimum(ratio * A, ratio.clamp(1 - cfg.clip, 1 + cfg.clip) * A)
            old_v = value_d[s, t]
            R = ret_d[s, t]
            v_clip = old_v + (value - old_v).clamp(-cfg.value_clip, cfg.value_clip)
            vl = 0.5 * torch.maximum((value - R) ** 2, (v_clip - R) ** 2)
            loss_pg, loss_v, entropy = vmean(pg), vmean(vl), vmean(ent)
            loss = loss_pg + cfg.value_coef * loss_v - cfg.entropy * entropy
            opt.zero_grad(set_to_none=True)
            loss.backward()
            gn = torch.nn.utils.clip_grad_norm_(model.parameters(), cfg.max_grad_norm)
            opt.step()
            with torch.no_grad():
                kl = vmean((ratio - 1) - log_ratio)
                clip = vmean(((ratio - 1).abs() > cfg.clip).float())
                acc += torch.stack([loss_pg, loss_v, entropy, kl, clip, gn.to(acc.dtype)])
                if epoch == 0:
                    first += torch.stack([vmean(ratio), kl])
                    first_mbs += 1
            mbs += 1
    out = dict(zip(names, (acc / mbs).tolist()))
    out["behavior_ratio"], out["behavior_kl"] = (first / max(first_mbs, 1)).tolist()
    closed = ro.closed
    rv = ret[closed]
    vv = ro.value[closed]
    var = rv.var() if rv.numel() > 1 else torch.tensor(0.0)
    out["explained_variance"] = float(1 - (rv - vv).var() / var) if float(var) > 0 else 0.0
    out["value_mean"] = float(vv.mean()) if vv.numel() else 0.0
    out["adv_mean"] = float(adv[closed].mean()) if vv.numel() else 0.0
    out["chunks"] = n_chunks
    out["minibatches"] = mbs
    out["decisions"] = int(closed.sum())
    return out
