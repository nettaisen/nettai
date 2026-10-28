"""Rollout storage and collection: every (env, seat) is its own stream of decisions.

A stream's decisions are stored in order, only where the learner plays that seat and the seat has a decision
(phase != 0). A decision is *open* until the stream's next event: its next decision (the reward since is
`hp_gain` there, and the next decision's value bootstraps it) or the round's end (`hp_gain` and `result`
there; terminal, or truncated, which bootstraps from the decision's own value since the pool's observation is
already the next round's). A decision still open when the rollout ends is carried into the next rollout as
its first entry, so it is trained on once it closes.

Storage is compact. A decision keeps only its present tokens, in their order, each with its position in its
set: the network reads no position but the pointer set's, which is laid out by position again when read
(`tokens_to_obs`), so the network's input is the same. On the pool's decisions that is about 34 tokens of 194
(9 KB of 57), so the rollouts of 2,048 envs fit on the GPU. The tokens and the GRU states are on the device;
the rest of a decision (legal masks, action, log-probability, value, reward, where its tokens are) is on the
CPU, where the collector writes it without waiting on the device, and the update copies what it reads.

Each step's deciding streams are gathered from the pool's buffers on the CPU (present tokens only) into pinned
memory and copied to the device without waiting; a step waits on the device once, for its actions.

Every stream the networks play (the learner's and the snapshots') carries its previous decision's action, which
the GRU reads (`model.previous_action`; none at a round's first decision, as the GRU state is zero there). The
rollout stores it with each decision, so the update reads exactly what the behavior network read.
"""

from __future__ import annotations

import threading
import time
from dataclasses import dataclass, field

import numpy as np
import torch

from .device import upload
from .envs import PoolView, read_round_stats
from .league import BOT, KIND_NAMES, LEARNER, SELF, League
from .model import Model, Obs, choose, map_obs, mask_logits, previous_action
from .spec import Spec


class Stopped(Exception):
    """Collection was asked to stop (the learner failed or was interrupted)."""


def tokens_to_obs(
    spec: Spec,
    pointer: str,
    tokens: dict[str, tuple[torch.Tensor, torch.Tensor, torch.Tensor]],
    off: dict[str, torch.Tensor],
    cnt: dict[str, torch.Tensor],
    keep: dict[str, int],
    dense: bool = False,
) -> Obs:
    """Decisions' observations from flat tokens: tokens[set] = (x [P, W], ids [P, I], pos [P]) with P ≥ 1, a
    decision's tokens in a row from off[set] [N], cnt[set] [N] of them; keep[set] (an int, ≥ 1 and every count)
    is the width read. A set comes out packed, [N, keep, ...], its present tokens first in their order (the
    network reads no position), except the pointer set (and with dense, every set), which is laid out by
    position at its full count. Absent tokens are zeros."""
    obs: Obs = {"x": {}, "ids": {}, "mask": {}}
    for s in spec.sets:
        x, ids, pos = tokens[s.name]
        c = cnt[s.name]
        N, k = c.shape[0], keep[s.name]
        j = torch.arange(k, device=x.device)
        valid = j < c[:, None]
        idx = torch.where(valid, off[s.name][:, None] + j, 0)
        tx = x[idx] * valid.unsqueeze(-1)
        ti = ids[idx] * valid.unsqueeze(-1)
        if dense or s.name == pointer:
            C = s.count
            at = torch.where(valid, pos[idx].long(), C)  # an absent token goes to a spare column, dropped
            rows = torch.arange(N, device=x.device)[:, None].expand(N, k)
            dx = x.new_zeros(N, C + 1, s.width)
            di = ids.new_zeros(N, C + 1, s.ids)
            dm = torch.zeros(N, C + 1, dtype=torch.bool, device=x.device)
            dx[rows, at] = tx
            di[rows, at] = ti
            dm[rows, at] = valid
            tx, ti, valid = dx[:, :C], di[:, :C], dm[:, :C]
        obs["x"][s.name], obs["ids"][s.name], obs["mask"][s.name] = tx, ti, valid
    return obs


def drop_names(ids: torch.Tensor, p: float, generator: torch.Generator | None) -> None:
    """With probability p per token, zero all of its name ids, in place (§7: it then plays from the numbers
    alone). ids [P, I], one row a token."""
    if p <= 0 or ids.shape[0] == 0 or ids.shape[1] == 0:
        return
    keep = torch.rand(ids.shape[0], generator=generator, device=ids.device) >= p
    ids.mul_(keep.unsqueeze(-1).to(ids.dtype))


class Tokens:
    """One set's stored tokens, flat on the device; grows when a rollout needs more (by a quarter at least, or to
    the room the collector projects)."""

    def __init__(self, width: int, ids: int, capacity: int, device: torch.device):
        capacity = max(1, capacity)
        self.x = torch.zeros(capacity, width, device=device)
        self.ids = torch.zeros(capacity, ids, dtype=torch.int32, device=device)
        self.pos = torch.zeros(capacity, dtype=torch.int16, device=device)
        self.n = 0  # tokens stored

    def parts(self) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        return self.x, self.ids, self.pos

    def put(self, x: torch.Tensor, ids: torch.Tensor, pos: torch.Tensor, room: int = 0) -> int:
        """Append tokens; returns the first one's row. If it must grow, to `room` tokens at least."""
        first, n = self.n, x.shape[0]
        cap = self.x.shape[0]
        if first + n > cap:
            new = max(first + n, cap + cap // 4, room)
            for name in ("x", "ids", "pos"):
                old = getattr(self, name)
                grown = old.new_zeros((new, *old.shape[1:]))
                grown[:first] = old[:first]
                setattr(self, name, grown)
        self.x[first : first + n] = x
        self.ids[first : first + n] = ids
        self.pos[first : first + n] = pos
        self.n = first + n
        return first

    def nbytes(self) -> int:
        return sum(t.numel() * t.element_size() for t in self.parts())


class Rollout:
    """One rollout's decisions, [S streams, Tc = steps + 1 decisions]: the per-decision tensors on the CPU (pinned
    on CUDA, so the update's copies don't wait), the tokens and GRU states on the device."""

    # What an open decision takes with it into the next rollout (its reward and successor come there).
    CARRIED = ("legal_fight", "legal_screen", "phase", "action", "logp", "value", "ep_start", "previous", "teacher")

    def __init__(self, spec: Spec, streams: int, steps: int, hidden: int, device: torch.device, pointer: str):
        S, Tc = streams, steps + 1
        self.S, self.Tc, self.spec, self.device, self.pointer = S, Tc, spec, device, pointer
        pin = device.type == "cuda"
        z = lambda *shape, dtype=torch.float32: torch.zeros(S, Tc, *shape, dtype=dtype, pin_memory=pin)
        self.legal_fight = z(spec.fight_actions, dtype=torch.bool)
        self.legal_screen = z(spec.screen_actions, dtype=torch.bool)
        self.phase = z(dtype=torch.int8)
        self.action = z(dtype=torch.int64)
        self.logp = z()
        self.value = z()
        self.ep_start = z(dtype=torch.bool)  # the first decision of its round (h_in is zero)
        self.previous = z(dtype=torch.int64)  # the stream's previous decision's action, as the GRU read it
        self.teacher = z(dtype=torch.int64)  # the teacher's label of a fight decision (env.teacher_rate), -1 none
        self.reward = z()
        self.term = z(dtype=torch.bool)
        self.trunc = z(dtype=torch.bool)
        self.next_value = z()
        self.closed = z(dtype=torch.bool)
        self.off = {s.name: z(dtype=torch.int64) for s in spec.sets}  # a decision's first row in tokens[set]
        self.cnt = {s.name: z(dtype=torch.int64) for s in spec.sets}  # its present tokens there
        self.cursor = torch.zeros(S, dtype=torch.int64)
        self.open = torch.full((S,), -1, dtype=torch.int64)
        self.h_in = torch.zeros(S, Tc, hidden, device=device)  # the GRU state before this decision
        # Room for one token a decision to start with; each set grows to what the rollouts need.
        self.tokens = {s.name: Tokens(s.width, s.ids, S * Tc, device) for s in spec.sets}

    def written(self) -> torch.Tensor:
        """[S, Tc] true where a decision is stored."""
        return torch.arange(self.Tc)[None, :] < self.cursor[:, None]

    def obs(self, s: torch.Tensor, t: torch.Tensor, include: torch.Tensor | None = None, dense: bool = False) -> Obs:
        """Stored decisions (s, t) (CPU index tensors of one shape) as the network's input, on the device; where
        include (CPU, that shape) is false, the decision comes out with no token at all. Sets are packed but the
        pointer set, unless dense (tokens_to_obs)."""
        shape = s.shape
        sf, tf = s.reshape(-1), t.reshape(-1)
        cnt, off, keep = [], [], {}
        for name in self.tokens:
            c = self.cnt[name][sf, tf]
            if include is not None:
                c = c * include.reshape(-1)
            cnt.append(c)
            off.append(self.off[name][sf, tf])
            keep[name] = max(1, int(c.max())) if c.numel() else 1
        meta = upload(torch.stack(cnt + off, 1), self.device)
        names = list(self.tokens)
        n = len(names)
        out = tokens_to_obs(
            self.spec, self.pointer, {k: v.parts() for k, v in self.tokens.items()},
            {k: meta[:, n + i] for i, k in enumerate(names)}, {k: meta[:, i] for i, k in enumerate(names)}, keep, dense,
        )
        return map_obs(out, lambda v: v.view(*shape, *v.shape[1:]))

    def take_open(self, src: "Rollout") -> None:
        """Start this rollout from src's open decisions (src may be this one): each moves to index 0 of its stream,
        tokens and GRU state with it; nothing else is kept."""
        s = (src.open >= 0).nonzero().squeeze(-1)
        t = src.open[s]
        for name, store in self.tokens.items():
            cnt = src.cnt[name][s, t]
            first = torch.cumsum(cnt, 0) - cnt
            rows = torch.repeat_interleave(src.off[name][s, t] - first, cnt) + torch.arange(int(cnt.sum()))
            moved = [p[upload(rows, self.device)] for p in src.tokens[name].parts()]  # copies: src may be this one
            store.n = 0
            store.put(*moved)
            self.off[name][s, 0] = first
            self.cnt[name][s, 0] = cnt
        for name in self.CARRIED:
            getattr(self, name)[s, 0] = getattr(src, name)[s, t]
        sd, td = upload(s, self.device), upload(t, self.device)
        self.h_in[sd, 0] = src.h_in[sd, td]
        for v in (self.reward, self.next_value, self.term, self.trunc, self.closed):
            v.zero_()
        self.cursor.zero_()
        self.cursor[s] = 1
        self.open.fill_(-1)
        self.open[s] = 0

    def clear(self) -> None:
        """Drop everything, open decisions too (after the normalization warmup)."""
        self.open.fill_(-1)
        self.take_open(self)

    def nbytes(self) -> dict[str, int]:
        host = [v for v in vars(self).values() if isinstance(v, torch.Tensor) and v is not self.h_in]
        host += [*self.off.values(), *self.cnt.values()]
        return {
            "host": sum(v.numel() * v.element_size() for v in host),
            "device": self.h_in.numel() * 4 + sum(store.nbytes() for store in self.tokens.values()),
        }


@dataclass
class Step:
    """One step's deciding streams on the device, with the CPU's counts. Decisions are in StepBatch.load's order."""

    n: int
    tokens: dict  # set → (x, ids, pos), each decision's present tokens in a row
    off: dict  # set → [n] first token
    cnt: dict  # set → [n] present tokens
    off_cpu: dict  # off and cnt on the CPU
    cnt_cpu: dict
    rows: dict  # set → tokens of all decisions (an int)
    streams: torch.Tensor  # [n]
    previous: torch.Tensor  # [n] each stream's previous action (`model.previous_action`)
    legal_fight: torch.Tensor
    legal_screen: torch.Tensor
    phase: torch.Tensor
    spec: Spec
    pointer: str

    def obs(self, a: int, b: int) -> Obs:
        """Decisions [a, b) as the network's input."""
        keep = {k: max(1, int(c[a:b].max())) if b > a else 1 for k, c in self.cnt_cpu.items()}
        return tokens_to_obs(
            self.spec, self.pointer, self.tokens, {k: v[a:b] for k, v in self.off.items()},
            {k: v[a:b] for k, v in self.cnt.items()}, keep,
        )


class StepBatch:
    """Pinned buffers (on CUDA) that a step's deciding streams are gathered into on the CPU, present tokens only,
    and copied to the device from without waiting."""

    def __init__(self, view: PoolView, spec: Spec, pointer: str, device: torch.device):
        S = view.S
        self.view, self.spec, self.pointer, self.device = view, spec, pointer, device
        pin = device.type == "cuda"
        e = lambda *shape, dtype=torch.float32: torch.zeros(*shape, dtype=dtype, pin_memory=pin)
        self.host = {
            s.name: (e(S * s.count, s.width), e(S * s.count, s.ids, dtype=torch.int32), e(S * s.count, dtype=torch.int16))
            for s in spec.sets
        }
        ns = len(spec.sets)
        # Per decision: each set's count, each set's first, the stream, its previous action.
        self.meta = e(S, 2 * ns + 2, dtype=torch.int64)
        self.legal = e(S, spec.fight_actions + spec.screen_actions, dtype=torch.bool)
        self.phase = e(S, dtype=torch.int64)
        self.copied = None  # the last load's copies (the buffers are rewritten once they're done)

    def load(self, order: torch.Tensor, previous: torch.Tensor) -> Step:
        """Gather streams `order` (CPU int64 [n]), with their previous actions (CPU int64 [n]), and copy them to
        the device."""
        v, dev, sets = self.view, self.device, self.spec.sets
        if self.copied is not None:
            self.copied.synchronize()
        n, ns = len(order), len(sets)
        tokens, rows = {}, {}
        for i, s in enumerate(sets):
            m = v.mask[s.name][order]  # [n, C]
            nz = m.nonzero()  # (decision, position), decision by decision, positions in order
            p = nz.shape[0]
            c = m.sum(1)
            src_x, src_ids, C, first = v.source[s.name]
            at = order[nz[:, 0]] * C + first + nz[:, 1]
            hx, hi, hp = self.host[s.name]
            torch.index_select(src_x, 0, at, out=hx[:p])
            torch.index_select(src_ids, 0, at, out=hi[:p])
            hp[:p] = nz[:, 1]
            self.meta[:n, i] = c
            self.meta[:n, ns + i] = torch.cumsum(c, 0) - c
            k = max(p, 1)
            tokens[s.name] = tuple(h[:k].to(dev, non_blocking=True) for h in (hx, hi, hp))
            rows[s.name] = p
        self.meta[:n, 2 * ns] = order
        self.meta[:n, 2 * ns + 1] = previous
        F = self.spec.fight_actions
        self.legal[:n, :F] = v.legal_fight[order]
        self.legal[:n, F:] = v.legal_screen[order]
        self.phase[:n] = v.phase[order]
        meta = self.meta[:n].to(dev, non_blocking=True)
        legal = self.legal[:n].to(dev, non_blocking=True)
        phase = self.phase[:n].to(dev, non_blocking=True)
        if dev.type == "cuda":
            self.copied = torch.cuda.Event()
            self.copied.record()
        names = [s.name for s in sets]
        return Step(
            n=n,
            tokens=tokens,
            off={k: meta[:, ns + i] for i, k in enumerate(names)},
            cnt={k: meta[:, i] for i, k in enumerate(names)},
            off_cpu={k: self.meta[:n, ns + i] for i, k in enumerate(names)},
            cnt_cpu={k: self.meta[:n, i] for i, k in enumerate(names)},
            rows=rows,
            streams=meta[:, 2 * ns],
            previous=meta[:, 2 * ns + 1],
            legal_fight=legal[:, :F],
            legal_screen=legal[:, F:],
            phase=phase,
            spec=self.spec,
            pointer=self.pointer,
        )



# How a round ended (the pool's `ending`, §6.6): a KO, the judge's ruling, the fight's limit.
BY_KO, BY_JUDGE, BY_LIMIT = 0, 1, 2


def round_end_reward(result: torch.Tensor, ending: torch.Tensor, result_weight: float, judged_cost: float, limit: float) -> torch.Tensor:
    """What a round's end is worth to each seat (§6.4): its result (+1, -1, 0) × result_weight; decided by the
    judge's ruling, `judged_cost` less for a seat that didn't lose; at the fight's limit, `limit`."""
    out = result * result_weight - torch.where((ending == BY_JUDGE) & (result >= 0), judged_cost, 0.0)
    return torch.where(ending == BY_LIMIT, torch.full_like(out, limit), out)


@dataclass
class CollectStats:
    steps: int = 0
    learner_decisions: int = 0
    opponent_decisions: int = 0
    rounds: int = 0
    truncated: int = 0
    judged: int = 0  # rounds decided by the judge's ruling
    limited: int = 0  # rounds that reached the fight's limit
    reward_sum: float = 0.0
    hp_gain_sum: float = 0.0
    step_seconds: float = 0.0  # in pool.step (the GIL let go)
    records: list = field(default_factory=list)  # (kind, opponent, learner's score, game) per counted round
    present: dict = field(default_factory=dict)  # set → summed present tokens over learner decisions
    measures: dict = field(default_factory=dict)  # (opponent kind, round_stats field) → [sum, rounds], learner seats


class Collector:
    """Plays the pool's policy seats, the learner's with `model` (the behavior network) and the others with the
    league's snapshots, one forward pass a network a step, and stores the learner's decisions in its rollout."""

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
        # Each stream's previous decision's action (CPU), none until it decides and at each round's start.
        self.no_previous = model.no_previous
        self.previous = torch.full((self.S,), self.no_previous, dtype=torch.int64)
        self.fresh = torch.ones(self.S, dtype=torch.bool)
        self.actions = np.zeros((self.E, 2), np.int32)
        self.name_dropout = name_dropout
        self.gen = torch.Generator(device=self.device)
        self.gen.manual_seed(seed)
        self.batch = StepBatch(self.view, spec, model.cfg.pointer_set, self.device)

    def begin(self, rollout: Rollout) -> None:
        """Collect into `rollout` next, starting from the current rollout's open decisions."""
        rollout.take_open(self.ro)
        self.ro = rollout

    @torch.no_grad()
    def collect(
        self,
        steps: int,
        hp_weight: float,
        result_weight: float,
        stop: threading.Event | None = None,
        judged_cost: float = 0.0,
        limit: float = 0.0,
    ) -> CollectStats:
        st = CollectStats(present={s.name: 0 for s in self.spec.sets})
        for _ in range(steps):
            if stop is not None and stop.is_set():
                raise Stopped()
            self._decide(st, hp_weight, (result_weight, judged_cost, limit))
            t = time.perf_counter()
            self.pool.step(self.actions)
            st.step_seconds += time.perf_counter() - t
            st.steps += 1
        return st

    def _decide(self, st: CollectStats, hp_weight: float, ending_weights: tuple[float, float, float]) -> None:
        v, ro, dev = self.view, self.ro, self.device
        phase = v.phase.to(torch.int64)
        done_e = v.done.clone()
        done = done_e.repeat_interleave(2)
        trunc = v.truncated.repeat_interleave(2)
        ending = v.ending.repeat_interleave(2)

        # 1. Close open learner decisions: at the stream's next decision, or at its round's end.
        closing = ((phase != 0) | done) & (ro.open >= 0)
        s = closing.nonzero().squeeze(-1)
        pending_s = pending_t = None
        if len(s):
            t = ro.open[s]
            d, tr = done[s], trunc[s]
            gain = v.hp_gain[s]
            end = d & ~tr
            r = hp_weight * gain + end * round_end_reward(v.result[s].to(torch.float32), ending[s], *ending_weights)
            ro.reward[s, t] = r
            ro.term[s, t] = end
            ro.trunc[s, t] = d & tr
            ro.closed[s, t] = True
            ro.next_value[s, t] = torch.where(d & tr, ro.value[s, t], 0.0)
            ro.open[s] = -1
            st.reward_sum += float(r.sum())
            st.hp_gain_sum += float(gain.sum())
            pending_s, pending_t = s[~d], t[~d]

        # 2. Rounds that ended: results to the league, the next plans, fresh GRU states and no previous action.
        if done_e.any():
            envs = done_e.nonzero().squeeze(-1).numpy()
            st.rounds += len(envs)
            st.truncated += int(v.truncated[envs].sum())
            st.judged += int((v.ending[envs] == BY_JUDGE).sum())
            st.limited += int((v.ending[envs] == BY_LIMIT).sum())
            games = v.game.numpy()
            self._measure(st, read_round_stats(self.pool))
            st.records += self.league.round_end(envs, v.result.view(self.E, 2).numpy(), games, v.truncated.numpy())
            streams = torch.from_numpy(np.stack([envs * 2, envs * 2 + 1], 1).ravel())
            self.hidden.index_fill_(0, upload(streams, dev), 0.0)
            self.previous[streams] = self.no_previous
            self.fresh[streams] = True

        # 3. Decisions: the learner's streams, then each snapshot's, gathered as one batch.
        ctrl = self.league.ctrl
        deciding = phase != 0
        if (deciding & (ctrl == BOT)).any():
            s = int((deciding & (ctrl == BOT)).nonzero()[0])
            raise RuntimeError(f"stream {s} is a scripted seat but has phase {int(phase[s])} (§6.6: always 0)")
        learner = (deciding & (ctrl == LEARNER)).nonzero().squeeze(-1)
        if pending_s is not None and len(pending_s):
            if not (ctrl[pending_s] == LEARNER).all() or not deciding[pending_s].all():
                raise RuntimeError("a learner decision closed at a decision the learner isn't making")
        groups = [(self.model, learner)]
        opp = deciding & (ctrl >= 0)
        if opp.any():
            for snap in torch.unique(ctrl[opp]).tolist():
                groups.append((self.league.model(snap), (opp & (ctrl == snap)).nonzero().squeeze(-1)))
        order = torch.cat([g[1] for g in groups])
        n, nl = len(order), len(learner)
        self.actions.fill(0)
        if n == 0:
            return
        t_l = ro.cursor[learner]
        if nl and int(t_l.max()) >= ro.Tc:
            raise RuntimeError("rollout storage overflow (a stream decided twice in one step?)")
        previous = self.previous[order]
        b = self.batch.load(order, previous)
        # The learner's tokens (the batch's first rows of each set): names dropped, then stored. A store that
        # must grow is given room for the whole rollout at this many tokens a decision (most slots fill).
        first = {}
        decided = max(1, int(ro.cursor.sum()) + nl)
        for name, (x, ids, pos) in b.tokens.items():
            p = int(b.cnt_cpu[name][:nl].sum())
            drop_names(ids[:p], self.name_dropout, self.gen)
            store = ro.tokens[name]
            first[name] = store.put(x[:p], ids[:p], pos[:p], room=(store.n + p) * ro.S * ro.Tc * 9 // (10 * decided))
            st.present[name] += p
        acts, logps, value_l = [], [], None
        at = 0
        for g, (net, idx) in enumerate(groups):
            a0, a1 = at, at + len(idx)
            at = a1
            if a1 == a0:
                continue
            sd = b.streams[a0:a1]
            h_in = self.hidden[sd]
            fight, screen, value, h = net.step(b.obs(a0, a1), h_in, b.previous[a0:a1])
            a, logp = choose(
                mask_logits(fight, b.legal_fight[a0:a1]), mask_logits(screen, b.legal_screen[a0:a1]), b.phase[a0:a1],
                generator=self.gen,
            )
            self.hidden[sd] = h
            if g == 0:
                ro.h_in[sd, upload(t_l, dev)] = h_in
                value_l = value
            acts.append(a.to(torch.float32))
            logps.append(logp)
        out = torch.cat(acts + logps + ([value_l] if value_l is not None else [])).cpu()  # the step's one wait
        a_all, logp_all, value_cpu = out[:n].to(torch.int64), out[n : 2 * n], out[2 * n :]
        if not torch.isfinite(logp_all).all():
            raise RuntimeError("collect: a decision has no legal action, or a network's outputs aren't finite")
        self.actions.reshape(-1)[order.numpy()] = a_all.numpy().astype(np.int32)
        self.previous[order] = previous_action(a_all, phase[order], self.spec.fight_actions)
        if nl:
            ro.action[learner, t_l] = a_all[:nl]
            ro.logp[learner, t_l] = logp_all[:nl]
            ro.value[learner, t_l] = value_cpu
            ro.legal_fight[learner, t_l] = v.legal_fight[learner]
            ro.legal_screen[learner, t_l] = v.legal_screen[learner]
            ro.teacher[learner, t_l] = v.teacher[learner].to(torch.int64)
            ro.phase[learner, t_l] = phase[learner].to(torch.int8)
            ro.ep_start[learner, t_l] = self.fresh[learner]
            ro.previous[learner, t_l] = previous[:nl]
            for name in ro.tokens:
                ro.off[name][learner, t_l] = first[name] + b.off_cpu[name][:nl]
                ro.cnt[name][learner, t_l] = b.cnt_cpu[name][:nl]
            ro.open[learner] = t_l
            ro.cursor[learner] = t_l + 1
            st.learner_decisions += nl
            if pending_s is not None and len(pending_s):
                vals = torch.zeros(self.S)
                vals[learner] = value_cpu
                ro.next_value[pending_s, pending_t] = vals[pending_s]
        self.fresh[order] = False
        st.opponent_decisions += n - nl

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
