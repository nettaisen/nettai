"""The league (§8): who plays each seat, frozen snapshots picked by PFSP, scripted players, win rates and Elo.

The pool takes a seat's controller "from the next round" (§6.6), so each env carries two plans: the current
round's, which the pool is already playing, and the next one's, already sent with `set_seat`. Whether a seat is
"policy" or a scripted player must be known a round ahead; which network plays a policy seat is Python's own
choice and is made when the round starts.

Streams (env · 2 + seat) are tagged for the current round: LEARNER, BOT, or a snapshot id (≥ 0).
"""

from __future__ import annotations

import math
from collections import OrderedDict, deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

import numpy as np
import torch

from .config import LeagueConfig
from .model import Model

LEARNER = -1
BOT = -2
SELF, SNAP, BOTS = 0, 1, 2
KIND_NAMES = {SELF: "self", SNAP: "snapshot", BOTS: "bot"}


@dataclass
class Snapshot:
    id: int
    path: str  # file name under the run's snapshots/
    update: int
    scores: deque = field(default_factory=deque)  # the learner's recent scores against it (1, 0.5, 0)

    def winrate(self) -> float:
        """The learner's win rate against it, with one win and one loss of prior."""
        return (sum(self.scores) + 1.0) / (len(self.scores) + 2.0)


def score(result: int) -> float:
    return 1.0 if result > 0 else (0.0 if result < 0 else 0.5)


def elo_expected(ra: float, rb: float) -> float:
    return 1.0 / (1.0 + 10.0 ** ((rb - ra) / 400.0))


class League:
    def __init__(
        self,
        cfg: LeagueConfig,
        pool,
        envs: int,
        run_dir: Path,
        device: torch.device,
        make_model: Callable[[], Model],
        rng: np.random.Generator,
    ):
        total = cfg.self_play + cfg.snapshots + cfg.bots
        if not math.isclose(total, 1.0, abs_tol=1e-6):
            raise ValueError(f"league: self_play + snapshots + bots = {total}, not 1")
        if cfg.bots > 0 and not cfg.bot_names:
            raise ValueError("league: a bot share with no bot_names")
        self.cfg = cfg
        self.pool = pool
        self.E = envs
        self.dir = Path(run_dir) / "snapshots"
        self.device = device
        self.make_model = make_model
        self.rng = rng
        self.snapshots: list[Snapshot] = []
        self.active: list[int] = []
        self.cache: OrderedDict[int, Model] = OrderedDict()
        self.ratings: dict[str, float] = {"learner": 1000.0}
        for b in cfg.bot_names:
            self.ratings[f"bot:{b}"] = 1000.0
        self.bot_scores: dict[str, deque] = {b: deque(maxlen=cfg.winrate_window) for b in cfg.bot_names}
        # Plans, current and next: learner seat, kind, opponent (snapshot id or bot index).
        self.seat = np.zeros(envs, np.int64)
        self.kind = np.zeros(envs, np.int64)
        self.opp = np.zeros(envs, np.int64)
        self.next_seat = np.zeros(envs, np.int64)
        self.next_kind = np.zeros(envs, np.int64)
        self.next_bot = np.zeros(envs, np.int64)
        self.ctrl = torch.full((envs * 2,), LEARNER, dtype=torch.int64)

    # Plans ---------------------------------------------------------------------------------------------

    def start(self) -> None:
        """The first round: the pool starts every seat as "policy", so it is self-play."""
        envs = np.arange(self.E)
        self.next_kind[envs] = SELF
        self.next_seat[envs] = 0
        self._begin(envs)
        self._draw_next(envs)
        self._send_next(envs)

    def _draw_next(self, envs: np.ndarray) -> None:
        n = len(envs)
        c = self.cfg
        p = np.array([c.self_play, c.snapshots, c.bots])
        self.next_kind[envs] = self.rng.choice(3, size=n, p=p / p.sum())
        self.next_seat[envs] = self.rng.integers(0, 2, n)
        self.next_bot[envs] = self.rng.integers(0, max(1, len(c.bot_names)), n)

    def _send_next(self, envs: np.ndarray) -> None:
        """set_seat for the next round of envs, grouped by (seat, controller)."""
        groups: dict[tuple[int, str], list[int]] = {}
        for e in envs:
            for s in (0, 1):
                if self.next_kind[e] == BOTS and s != self.next_seat[e]:
                    ctl = self.cfg.bot_names[self.next_bot[e]]
                else:
                    ctl = "policy"
                groups.setdefault((s, ctl), []).append(int(e))
        for (s, ctl), es in groups.items():
            self.pool.set_seat(np.asarray(es, np.int64), s, ctl)

    def _begin(self, envs: np.ndarray) -> None:
        """The next plan becomes the current round's; a snapshot round draws its opponent now."""
        self.seat[envs] = self.next_seat[envs]
        self.kind[envs] = self.next_kind[envs]
        self.opp[envs] = self.next_bot[envs]
        for e in envs:
            if self.kind[e] == SNAP:
                if self.active:
                    self.opp[e] = self.active[self.rng.integers(len(self.active))]
                else:
                    self.kind[e] = SELF
            ls, os_ = 2 * e + self.seat[e], 2 * e + 1 - self.seat[e]
            self.ctrl[ls] = LEARNER
            self.ctrl[os_] = {SELF: LEARNER, SNAP: int(self.opp[e]), BOTS: BOT}[int(self.kind[e])]

    def round_end(self, envs: np.ndarray, result: np.ndarray, games: np.ndarray, truncated: np.ndarray) -> list[tuple]:
        """Rounds of envs ended with result [E, 2]: record them, start the next plans, send the ones after.

        A round cut off (truncated: max_fight_ticks, a screen open too long, a content error) has no result and
        counts for nothing: not a draw, not a loss."""
        records = []
        for e in envs:
            k = int(self.kind[e])
            if k == SELF or truncated[e]:
                continue
            sc = score(int(result[e, self.seat[e]]))
            if k == SNAP:
                snap = self.snapshots[int(self.opp[e])]
                snap.scores.append(sc)
                while len(snap.scores) > self.cfg.winrate_window:
                    snap.scores.popleft()
                opp = f"snap:{snap.id}"
            else:
                name = self.cfg.bot_names[int(self.opp[e])]
                self.bot_scores[name].append(sc)
                opp = f"bot:{name}"
            self._elo("learner", opp, sc)
            records.append((KIND_NAMES[k], opp, sc, int(games[e])))
        self._begin(envs)
        self._draw_next(envs)
        self._send_next(envs)
        return records

    def opponent_label(self, e: int) -> str:
        """Who plays against the learner in env e's current round: "self", "snapshot" or "bot:<name>"."""
        k = int(self.kind[e])
        return f"bot:{self.cfg.bot_names[int(self.opp[e])]}" if k == BOTS else KIND_NAMES[k]

    def _elo(self, a: str, b: str, sa: float) -> None:
        ea = elo_expected(self.ratings[a], self.ratings[b])
        k = self.cfg.elo_k
        self.ratings[a] += k * (sa - ea)
        self.ratings[b] -= k * (sa - ea)

    # Snapshots -----------------------------------------------------------------------------------------

    def pfsp_weights(self) -> np.ndarray:
        """(1 − the learner's win rate) ** power: snapshots that still beat it are picked most."""
        return np.array([(1.0 - s.winrate()) ** self.cfg.pfsp_power for s in self.snapshots])

    def refresh(self) -> None:
        """Pick this rollout's active snapshots by PFSP weight, without replacement."""
        n = len(self.snapshots)
        if n == 0:
            self.active = []
            return
        w = self.pfsp_weights() + 1e-6
        k = min(self.cfg.active, n)
        self.active = [int(i) for i in self.rng.choice(n, size=k, replace=False, p=w / w.sum())]

    def add_snapshot(self, model: Model, update: int) -> Snapshot:
        self.dir.mkdir(parents=True, exist_ok=True)
        sid = len(self.snapshots)
        path = self.dir / f"snap_{sid:05d}.pt"
        torch.save({k: v.detach().cpu() for k, v in model.state_dict().items()}, path)
        snap = Snapshot(sid, path.name, update)
        self.snapshots.append(snap)
        self.ratings[f"snap:{sid}"] = self.ratings["learner"]
        return snap

    def model(self, sid: int) -> Model:
        if sid in self.cache:
            self.cache.move_to_end(sid)
            return self.cache[sid]
        m = self.make_model()
        m.load_state_dict(torch.load(self.dir / self.snapshots[sid].path, map_location=self.device, weights_only=True))
        m.eval()
        self.cache[sid] = m
        while len(self.cache) > max(self.cfg.cache, self.cfg.active):
            self.cache.popitem(last=False)
        return m

    # Reporting and resuming ----------------------------------------------------------------------------

    def bot_winrate(self, name: str) -> tuple[float, int]:
        s = self.bot_scores[name]
        return (sum(s) / len(s) if s else float("nan")), len(s)

    def state_dict(self) -> dict:
        return {
            "snapshots": [(s.id, s.path, s.update, list(s.scores)) for s in self.snapshots],
            "ratings": dict(self.ratings),
            "bot_scores": {k: list(v) for k, v in self.bot_scores.items()},
        }

    def load_state_dict(self, d: dict) -> None:
        self.snapshots = [Snapshot(i, p, u, deque(sc)) for i, p, u, sc in d["snapshots"]]
        self.ratings = dict(d["ratings"])
        for k, v in d["bot_scores"].items():
            if k in self.bot_scores:
                self.bot_scores[k].extend(v)
