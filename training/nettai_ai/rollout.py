"""Rollout storage and collection: every (env, seat) is its own stream of decisions.

A stream's decisions are stored in order, only where the learner plays that seat and the seat has a decision
(phase != 0). A decision is *open* until the stream's next event: its next decision (the reward since is
`hp_gain` there, and the next decision's value bootstraps it) or the round's end (`hp_gain` and `result`
there; terminal, or truncated, which bootstraps from the decision's own value since the pool's observation is
already the next round's). A decision still open when the rollout ends is carried into the next rollout as
its first entry, so it is trained on once it closes.
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
import torch

from .envs import PoolView, read_round_stats
from .league import BOT, KIND_NAMES, LEARNER, SELF, League
from .model import Model, Obs, mask_logits, select
from .spec import Spec


class Rollout:
    """Preallocated storage, [S streams, Tc = steps + 1 decisions], on the training device."""

    def __init__(self, spec: Spec, streams: int, steps: int, hidden: int, device: torch.device):
        S, Tc = streams, steps + 1
        self.S, self.Tc, self.spec, self.device = S, Tc, spec, device
        z = lambda *shape, dtype=torch.float32: torch.zeros(S, Tc, *shape, dtype=dtype, device=device)
        self.x = {s.name: z(s.count, s.width) for s in spec.sets}
        self.ids = {s.name: z(s.count, s.ids, dtype=torch.int32) for s in spec.sets}
        self.mask = {s.name: z(s.count, dtype=torch.bool) for s in spec.sets}
        self.legal_fight = z(spec.fight_actions, dtype=torch.bool)
        self.legal_screen = z(spec.screen_actions, dtype=torch.bool)
        self.phase = z(dtype=torch.int8)
        self.action = z(dtype=torch.int64)
        self.logp = z()
        self.value = z()
        self.h_in = z(hidden)  # the GRU state before this decision
        self.ep_start = z(dtype=torch.bool)  # the first decision of its round (h_in is zero)
        self.reward = z()
        self.term = z(dtype=torch.bool)
        self.trunc = z(dtype=torch.bool)
        self.next_value = z()
        self.closed = z(dtype=torch.bool)
        self.cursor = torch.zeros(S, dtype=torch.int64, device=device)
        self.open = torch.full((S,), -1, dtype=torch.int64, device=device)

    def gather(self, s: torch.Tensor, t: torch.Tensor) -> Obs:
        return {
            "x": {n: v[s, t] for n, v in self.x.items()},
            "ids": {n: v[s, t] for n, v in self.ids.items()},
            "mask": {n: v[s, t] for n, v in self.mask.items()},
        }

    def written(self) -> torch.Tensor:
        """[S, Tc] true where a decision is stored."""
        return torch.arange(self.Tc, device=self.device)[None, :] < self.cursor[:, None]

    def carry(self) -> None:
        """After an update: move each stream's open decision to index 0 and clear the rest."""
        s = (self.open >= 0).nonzero().squeeze(-1)
        t = self.open[s]
        tensors = [*self.x.values(), *self.ids.values(), *self.mask.values()]
        tensors += [self.legal_fight, self.legal_screen, self.phase, self.action, self.logp, self.value]
        tensors += [self.h_in, self.ep_start]
        for v in tensors:
            v[s, 0] = v[s, t]
        for v in (self.reward, self.next_value):
            v.zero_()
        for v in (self.term, self.trunc, self.closed):
            v.zero_()
        self.cursor.zero_()
        self.cursor[s] = 1
        self.open.fill_(-1)
        self.open[s] = 0

    def clear(self) -> None:
        """Drop everything, open decisions too (after the normalization warmup)."""
        self.open.fill_(-1)
        self.carry()


def drop_names(obs: Obs, p: float, generator: torch.Generator | None) -> Obs:
    """With probability p per token, zero all of its name ids (§7: it then plays from the numbers alone)."""
    if p <= 0:
        return obs
    ids = {}
    for n, v in obs["ids"].items():
        if v.shape[-1] == 0:
            ids[n] = v
            continue
        keep = torch.rand(v.shape[:-1], generator=generator, device=v.device) >= p
        ids[n] = v * keep.unsqueeze(-1).to(v.dtype)
    return {"x": obs["x"], "ids": ids, "mask": obs["mask"]}


@dataclass
class CollectStats:
    steps: int = 0
    learner_decisions: int = 0
    opponent_decisions: int = 0
    rounds: int = 0
    truncated: int = 0
    reward_sum: float = 0.0
    hp_gain_sum: float = 0.0
    records: list = field(default_factory=list)  # (kind, opponent, learner's score, game) per counted round
    present: dict = field(default_factory=dict)  # set → summed present tokens over learner decisions
    measures: dict = field(default_factory=dict)  # (opponent kind, round_stats field) → [sum, rounds], learner seats


class Collector:
    def __init__(self, pool, spec: Spec, model: Model, league: League, rollout: Rollout, name_dropout: float, seed: int):
        self.pool = pool
        self.view = PoolView(pool, spec)
        self.spec = spec
        self.model = model
        self.league = league
        self.ro = rollout
        self.device = rollout.device
        self.S = self.view.S
        self.E = self.view.E
        self.hidden = torch.zeros(self.S, model.cfg.hidden, device=self.device)
        self.fresh = torch.ones(self.S, dtype=torch.bool, device=self.device)
        self.actions = np.zeros((self.E, 2), np.int32)
        self.name_dropout = name_dropout
        self.gen = torch.Generator(device=self.device)
        self.gen.manual_seed(seed)

    @torch.no_grad()
    def collect(self, steps: int, hp_weight: float, result_weight: float) -> CollectStats:
        st = CollectStats(present={s.name: 0 for s in self.spec.sets})
        v, ro, dev = self.view, self.ro, self.device
        for _ in range(steps):
            phase = v.phase.to(torch.int64)
            done_e = v.done.clone()
            done = done_e.repeat_interleave(2)
            trunc = v.truncated.repeat_interleave(2)

            # 1. Close open learner decisions: at the stream's next decision, or at its round's end.
            closing = ((phase != 0) | done).to(dev) & (ro.open >= 0)
            pending_s = pending_t = None
            if closing.any():
                s = closing.nonzero().squeeze(-1)
                t = ro.open[s]
                sc = s.cpu()
                d, tr = done[sc].to(dev), trunc[sc].to(dev)
                gain = v.hp_gain[sc].to(dev)
                res = v.result[sc].to(dev, torch.float32)
                r = hp_weight * gain + result_weight * res * (d & ~tr)
                ro.reward[s, t] = r
                ro.term[s, t] = d & ~tr
                ro.trunc[s, t] = d & tr
                ro.closed[s, t] = True
                ro.next_value[s, t] = torch.where(d & tr, ro.value[s, t], 0.0)
                ro.open[s] = -1
                st.reward_sum += float(r.sum())
                st.hp_gain_sum += float(gain.sum())
                keep = ~d
                pending_s, pending_t = s[keep], t[keep]

            # 2. Rounds that ended: results to the league, the next plans, fresh GRU states.
            if done_e.any():
                envs = done_e.nonzero().squeeze(-1).numpy()
                st.rounds += len(envs)
                st.truncated += int(v.truncated[envs].sum())
                games = v.game.numpy()
                self._measure(st, read_round_stats(self.pool))
                st.records += self.league.round_end(envs, v.result.view(self.E, 2).numpy(), games, v.truncated.numpy())
                streams = torch.from_numpy(np.stack([envs * 2, envs * 2 + 1], 1).ravel()).to(dev)
                self.hidden[streams] = 0.0
                self.fresh[streams] = True

            # 3. Decisions.
            ctrl = self.league.ctrl
            deciding = phase != 0
            if (deciding & (ctrl == BOT)).any():
                s = int((deciding & (ctrl == BOT)).nonzero()[0])
                raise RuntimeError(f"stream {s} is a scripted seat but has phase {int(phase[s])} (§6.6: always 0)")
            self.actions.fill(0)
            acts = self.actions.reshape(-1)
            learner = (deciding & (ctrl == LEARNER)).nonzero().squeeze(-1)
            if len(learner):
                ld = learner.to(dev)
                obs = drop_names(v.obs(learner, dev), self.name_dropout, self.gen)
                h_in = self.hidden[ld]
                fight, screen, value, h = self.model.step(obs, h_in)
                lf, ls = v.legal_fight[learner].to(dev), v.legal_screen[learner].to(dev)
                ph = phase[learner].to(dev)
                a, logp = select(mask_logits(fight, lf), mask_logits(screen, ls), ph, generator=self.gen)
                t = ro.cursor[ld]
                if int(t.max()) >= ro.Tc:
                    raise RuntimeError("rollout storage overflow (a stream decided twice in one step?)")
                for n in ro.x:
                    ro.x[n][ld, t] = obs["x"][n]
                    ro.ids[n][ld, t] = obs["ids"][n]
                    ro.mask[n][ld, t] = obs["mask"][n]
                    st.present[n] += int(obs["mask"][n].sum())
                ro.legal_fight[ld, t] = lf
                ro.legal_screen[ld, t] = ls
                ro.phase[ld, t] = ph.to(torch.int8)
                ro.action[ld, t] = a
                ro.logp[ld, t] = logp
                ro.value[ld, t] = value
                ro.h_in[ld, t] = h_in
                ro.ep_start[ld, t] = self.fresh[ld]
                ro.open[ld] = t
                ro.cursor[ld] = t + 1
                self.hidden[ld] = h
                self.fresh[ld] = False
                acts[learner.numpy()] = a.to(torch.int32).cpu().numpy()
                st.learner_decisions += len(learner)
                if pending_s is not None and len(pending_s):
                    if not (ctrl[pending_s.cpu()] == LEARNER).all() or not deciding[pending_s.cpu()].all():
                        raise RuntimeError("a learner decision closed at a decision the learner isn't making")
                    vals = torch.zeros(self.S, device=dev)
                    vals[ld] = value
                    ro.next_value[pending_s, pending_t] = vals[pending_s]
            elif pending_s is not None and len(pending_s):
                raise RuntimeError("a learner decision closed at a decision that didn't come")
            opp = deciding & (ctrl >= 0)
            if opp.any():
                for snap in torch.unique(ctrl[opp]).tolist():
                    idx = (opp & (ctrl == snap)).nonzero().squeeze(-1)
                    di = idx.to(dev)
                    net = self.league.model(snap)
                    fight, screen, _, h = net.step(v.obs(idx, dev), self.hidden[di])
                    lf, ls = v.legal_fight[idx].to(dev), v.legal_screen[idx].to(dev)
                    a, _ = select(mask_logits(fight, lf), mask_logits(screen, ls), phase[idx].to(dev), generator=self.gen)
                    self.hidden[di] = h
                    self.fresh[di] = False
                    acts[idx.numpy()] = a.to(torch.int32).cpu().numpy()
                    st.opponent_decisions += len(idx)

            # 4. Advance.
            self.pool.step(self.actions)
            st.steps += 1
        return st

    def _measure(self, st: CollectStats, rounds: list[dict]) -> None:
        """§9's measures from round_stats() for the learner's seats, by opponent kind. Read before the league
        moves on, while its current plan still describes these rounds."""
        for r in rounds:
            if r["truncated"]:  # cut off (time, or a content error): no result, and not this policy's measures
                continue
            e = int(r["env"])
            label = self.league.opponent_label(e)
            k = int(self.league.kind[e])
            seats = (0, 1) if k == SELF else (int(self.league.seat[e]),)
            for s in seats:
                me, other = r["seats"][s], r["seats"][1 - s]
                vals = {n: v for n, v in me.items() if isinstance(v, (int, float)) and not isinstance(v, bool)}
                vals["ticks"] = r["ticks"]
                if "hp_lost" in me and "max_hp" in me:
                    # The round's HP difference as hp_gain counts it: the other's loss less its own, each by
                    # its own max HP.
                    vals["hp_gain"] = other["hp_lost"] / max(other["max_hp"], 1) - me["hp_lost"] / max(me["max_hp"], 1)
                for name, val in vals.items():
                    for key in (KIND_NAMES[k], label) if label != KIND_NAMES[k] else (label,):
                        acc = st.measures.setdefault((key, name), [0.0, 0])
                        acc[0] += float(val)
                        acc[1] += 1
