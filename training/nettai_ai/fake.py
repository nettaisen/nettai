"""`FakePool`: a pure-numpy stand-in for `nettai_gym.Pool` with exactly §6.6's API, shapes and dtypes.

It plays a toy game that a policy can learn through the whole pipeline (encoder, transformer, pool, GRU,
heads, PPO, league):

- In the fight, the general token's first `fight_actions` floats are a one-hot "target" action (always
  legal). Pressing it deals 5% of the round's max HP to the other side.
- On the custom screen, picking the legal slot whose first float is highest deals 8%. Keys do nothing.
- Scripted seats deal 5% now and then (masher most often), never decide, and always have phase 0.

Everything else is noise, including the floats of absent tokens, so a model that reads a masked token is
wrong in a way the toy punishes. Phases vary per seat (a fight seat is sometimes busy), rounds end at random,
on a KO, or are cut off at `max_fight_ticks` (truncated). Rewards come out as §6.6 says: `hp_gain` is
(HP the other lost − HP it lost) / the round's max HP since the seat's last decision, and `result` at `done`.
"""

from __future__ import annotations

from typing import Any, Iterable

import numpy as np

from .spec import spec_hash

CONTROLLERS = ("policy", "still", "mover", "masher", "shooter", "dodger")
# Chance per fight step that a scripted seat lands a hit.
BOT_HIT = {"still": 0.0, "mover": 0.01, "masher": 0.03, "shooter": 0.05, "dodger": 0.02}

FIGHT_HIT = 0.05  # of max HP, for pressing the target
SCREEN_HIT = 0.08  # of max HP, for picking the best legal slot
END_CHANCE = 1.0 / 40.0  # per fight step
BUSY_CHANCE = 0.15  # a fight seat with no decision this step


def default_spec(buckets: int = 8192) -> dict[str, Any]:
    """§6.6's format with the pool's sets, counts and widths (spec c4d9e4a61b0f54ba, its chips set split into
    hand and used as the trainer reads it; the attack log is in the panels' tokens). Names come as id pairs."""
    sets = {
        "general": {"count": 1, "width": 71, "ids": 4},
        "panels": {"count": 18, "width": 47, "ids": 2},
        "objects": {"count": 48, "width": 119, "ids": 6},
        "hand": {"count": 6, "width": 250, "ids": 12},  # a chip token, its fingerprint included
        "used": {"count": 1, "width": 250, "ids": 12},
        "slots": {"count": 12, "width": 250, "ids": 12},
        "rules": {"count": 96, "width": 14, "ids": 4},
        "forms": {"count": 12, "width": 22, "ids": 2},
    }
    raw = {"buckets": buckets, "fight_actions": 20, "screen_actions": 19, "sets": sets}
    raw["hash"] = spec_hash(raw)
    return raw


# Present tokens per set (lo, hi) in the fight and on the screen, as the pool shows them; sets not named here
# draw from (1, count).
OCCUPANCY = {
    "general": ((1, 1), (1, 1)),
    "panels": ((18, 18), (18, 18)),
    "objects": ((2, 16), (2, 8)),
    "hand": ((0, 6), (0, 0)),
    "slots": ((0, 0), (4, 12)),
    "used": ((0, 1), (0, 1)),
    "rules": ((43, 79), (43, 79)),
    "forms": ((1, 3), (1, 12)),
}


class FakePool:
    def __init__(
        self,
        games: Iterable[str] = ("exe6", "exe5", "exe4"),
        envs: int = 8,
        threads: int = 1,
        seed: int = 1,
        content: str | None = None,
        packs: str | None = None,
        k: int = 4,
        delay: tuple[int, int] = (6, 30),
        max_fight_ticks: int = 18000,
        hp: tuple[int, int] = (1000, 1000),
        chip_holdout: tuple[float, int, str] | None = None,
        cache: str | None = None,
        spec: dict[str, Any] | None = None,
    ):
        self.games = list(games)
        if not self.games:
            raise ValueError("FakePool: no games")
        if envs < 1 or threads < 0 or k < 1:
            raise ValueError("FakePool: envs ≥ 1, threads ≥ 0, k ≥ 1")
        if chip_holdout is not None:
            fraction, _seed, mode = chip_holdout
            if not 0.0 <= fraction < 1.0 or mode not in ("exclude", "favor"):
                raise ValueError(f"FakePool: bad chip_holdout {chip_holdout!r}")
        self.chip_holdout = chip_holdout
        self.E = envs
        self.k = k
        self.delay_range = (int(delay[0]), int(delay[1]))
        self.max_fight_ticks = max_fight_ticks
        self.rng = np.random.default_rng(seed)
        self._spec = spec if spec is not None else default_spec()
        self.set_hp(*hp)
        sets = self._spec["sets"]
        self.fa = int(self._spec["fight_actions"])
        self.sa = int(self._spec["screen_actions"])
        self.n_slots = sets["slots"]["count"]
        self.n_keys = self.sa - self.n_slots
        if self.n_keys < 0:
            raise ValueError("FakePool: screen_actions < the slots set's count")
        if sets["general"]["width"] < self.fa + 4:
            raise ValueError("FakePool: the general set must be at least fight_actions + 4 wide")
        E = envs
        self.buf: dict[str, Any] = {
            "x": {n: np.zeros((E, 2, s["count"], s["width"]), np.float32) for n, s in sets.items()},
            "ids": {n: np.zeros((E, 2, s["count"], s["ids"]), np.int32) for n, s in sets.items()},
            "mask": {n: np.zeros((E, 2, s["count"]), np.bool_) for n, s in sets.items()},
            "phase": np.zeros((E, 2), np.int8),
            "legal_fight": np.zeros((E, 2, self.fa), np.bool_),
            "legal_screen": np.zeros((E, 2, self.sa), np.bool_),
            "hp_gain": np.zeros((E, 2), np.float32),
            "done": np.zeros(E, np.bool_),
            "truncated": np.zeros(E, np.bool_),
            "result": np.zeros((E, 2), np.int8),
            "game": np.zeros(E, np.int8),
            "delay": np.zeros((E, 2), np.int16),
        }
        self.ctrl = np.zeros((E, 2), np.int64)  # index into CONTROLLERS, this round
        self.ctrl_next = np.zeros((E, 2), np.int64)
        self.delay_pin = np.full((E, 2), -1, np.int64)
        self.maxhp = np.zeros(E, np.float64)
        self.hp = np.zeros((E, 2), np.float64)
        self.stage = np.zeros(E, np.int64)  # 0 custom screen, 1 fight
        self.screen_left = np.zeros((E, 2), np.int64)
        self.fight_steps = np.zeros(E, np.int64)
        self.ticks = np.zeros(E, np.int64)
        self.acc = np.zeros((E, 2), np.float64)
        self.target = np.zeros((E, 2), np.int64)
        self.slot_value = np.zeros((E, 2, self.n_slots), np.float32)
        self.counts = np.zeros((E, 2, 6), np.int64)  # dealt, lost, fight ok, fight n, screen ok, screen n
        self._stats: list[dict[str, Any]] = []
        self._start_rounds(np.arange(E))
        self._next_phases(np.arange(E))
        self._observe()

    # §6.6 API ------------------------------------------------------------------------------------------

    def spec(self) -> dict[str, Any]:
        return self._spec

    def buffers(self) -> dict[str, Any]:
        return self.buf

    def set_seat(self, envs, seat: int, controller: str) -> None:
        if controller not in CONTROLLERS:
            raise ValueError(f"set_seat: unknown controller {controller!r} (known: {', '.join(CONTROLLERS)})")
        if seat not in (0, 1):
            raise ValueError(f"set_seat: seat {seat} is not 0 or 1")
        self.ctrl_next[np.asarray(envs, np.int64), seat] = CONTROLLERS.index(controller)

    def set_delay(self, envs, seat: int, ticks: int | None) -> None:
        self.delay_pin[np.asarray(envs, np.int64), seat] = -1 if ticks is None else int(ticks)

    def set_hp(self, lo: int, hi: int) -> None:
        if not 1 <= lo <= hi:
            raise ValueError(f"set_hp: need 1 ≤ lo ≤ hi, got ({lo}, {hi})")
        self._hp_next = (int(lo), int(hi))

    def round_stats(self) -> list[dict[str, Any]]:
        out, self._stats = self._stats, []
        return out

    def step(self, actions) -> None:
        a = np.asarray(actions)
        if a.shape != (self.E, 2):
            raise ValueError(f"step: actions must be [{self.E}, 2], got {a.shape}")
        b = self.buf
        phase = b["phase"]
        E = self.E
        rows = np.arange(E)[:, None]
        cols = np.arange(2)[None, :]
        fight = phase == 1
        screen = phase == 2
        fa = np.clip(a, 0, self.fa - 1)
        sa = np.clip(a, 0, self.sa - 1)
        bad = (fight & ((a < 0) | (a >= self.fa) | ~b["legal_fight"][rows, cols, fa])) | (
            screen & ((a < 0) | (a >= self.sa) | ~b["legal_screen"][rows, cols, sa])
        )
        if bad.any():
            e, s = np.argwhere(bad)[0]
            raise ValueError(f"step: illegal action {a[e, s]} for env {e} seat {s} (phase {phase[e, s]})")

        dealt = np.zeros((E, 2))
        # Fight: the target action hits.
        ok_f = fight & (a == self.target)
        dealt += ok_f * FIGHT_HIT
        self.counts[..., 2] += ok_f
        self.counts[..., 3] += fight
        # Screen: the best legal slot hits; every screen decision uses one of the seat's screen turns.
        slot_legal = b["legal_screen"][..., : self.n_slots]
        best = np.where(slot_legal, self.slot_value, -np.inf).argmax(-1)
        ok_s = screen & (a == best)
        dealt += ok_s * SCREEN_HIT
        self.counts[..., 4] += ok_s
        self.counts[..., 5] += screen
        self.screen_left -= screen
        # Scripted seats hit now and then in the fight.
        in_fight = self.stage == 1
        for name, p in BOT_HIT.items():
            c = CONTROLLERS.index(name)
            hits = (self.ctrl == c) & in_fight[:, None] & (self.rng.random((E, 2)) < p)
            dealt += hits * FIGHT_HIT

        dmg = dealt * self.maxhp[:, None]
        self.hp -= dmg[:, ::-1]
        self.counts[..., 0] += dmg.astype(np.int64)
        self.counts[..., 1] += dmg[:, ::-1].astype(np.int64)
        self.acc += (dealt - dealt[:, ::-1])

        # Time.
        self.ticks += self.k
        self.fight_steps += in_fight
        policy = self.ctrl == 0
        screen_over = (self.stage == 0) & ~((self.screen_left > 0) & policy).any(1)
        self.stage[screen_over] = 1
        ko = (self.hp <= 0).any(1)
        cut = in_fight & (self.fight_steps * self.k >= self.max_fight_ticks) & ~ko
        ended = in_fight & (self.rng.random(E) < END_CHANCE)
        done = ko | cut | ended

        b["done"][:] = done
        b["truncated"][:] = cut
        diff = np.sign(self.hp - self.hp[:, ::-1]).astype(np.int8)
        b["result"][:] = np.where(done[:, None] & ~cut[:, None], diff, 0)
        if done.any():
            de = np.flatnonzero(done)
            for e in de:
                self._stats.append(self._round_stat(int(e), bool(cut[e]), b["result"][e]))
            self._start_rounds(de)
        self._next_phases(np.arange(E))
        # hp_gain: since the seat's last decision; read and cleared where it decides now or the round ended.
        b["hp_gain"][:] = self.acc
        cleared = (b["phase"] != 0) | done[:, None]
        self.acc[cleared] = 0.0
        self._observe()

    # The toy game --------------------------------------------------------------------------------------

    def _round_stat(self, e: int, truncated: bool, result) -> dict[str, Any]:
        seats = []
        for s in range(2):
            c = self.counts[e, s]
            seats.append(
                {
                    "controller": CONTROLLERS[self.ctrl[e, s]],
                    "result": int(result[s]),
                    "hp_dealt": int(c[0]),
                    "hp_lost": int(c[1]),
                    "toy_fight_correct": int(c[2]),
                    "toy_fight_decisions": int(c[3]),
                    "toy_screen_correct": int(c[4]),
                    "toy_screen_decisions": int(c[5]),
                }
            )
        return {
            "env": e,
            "game": self.games[self.buf["game"][e]],
            "ticks": int(self.ticks[e]),
            "truncated": truncated,
            "hp": int(self.maxhp[e]),
            "seats": seats,
        }

    def _start_rounds(self, envs: np.ndarray) -> None:
        n = len(envs)
        self.ctrl[envs] = self.ctrl_next[envs]
        lo, hi = self._hp_next
        self.maxhp[envs] = self.rng.integers(lo, hi + 1, n)
        self.hp[envs] = self.maxhp[envs, None]
        self.stage[envs] = 0
        self.screen_left[envs] = np.where(self.ctrl[envs] == 0, self.rng.integers(1, 4, (n, 2)), 0)
        self.fight_steps[envs] = 0
        self.ticks[envs] = 0
        self.counts[envs] = 0
        self.buf["game"][envs] = self.rng.integers(0, len(self.games), n)
        d = self.rng.integers(self.delay_range[0], self.delay_range[1] + 1, (n, 2))
        pin = self.delay_pin[envs]
        self.buf["delay"][envs] = np.where(pin >= 0, pin, d)

    def _next_phases(self, envs: np.ndarray) -> None:
        b = self.buf
        policy = self.ctrl[envs] == 0
        on_screen = (self.stage[envs] == 0)[:, None]
        screen = on_screen & policy & (self.screen_left[envs] > 0)
        fight = ~on_screen & policy & (self.rng.random((len(envs), 2)) >= BUSY_CHANCE)
        # A step runs until some policy seat decides: if both fight seats came out busy, the first one decides.
        none = ~on_screen[:, 0] & policy.any(1) & ~fight.any(1)
        first = np.argmax(policy, axis=1)
        fight[np.flatnonzero(none), first[none]] = True
        b["phase"][envs] = np.where(screen, 2, np.where(fight, 1, 0)).astype(np.int8)

    def _observe(self) -> None:
        b = self.buf
        rng = self.rng
        E = self.E
        sets = self._spec["sets"]
        phase = b["phase"]
        on_screen = (self.stage == 0)[:, None].repeat(2, 1)
        for name, s in sets.items():
            C, I = s["count"], s["ids"]
            x = b["x"][name]
            rng.standard_normal(x.shape, dtype=np.float32, out=x)
            fight_occ, screen_occ = OCCUPANCY.get(name, ((1, C), (1, C)))
            lo = np.where(on_screen, screen_occ[0], fight_occ[0])
            hi = np.where(on_screen, screen_occ[1], fight_occ[1])
            n = rng.integers(lo, hi + 1)
            if name == "slots":
                present = np.arange(C)[None, None, :] < n[..., None]
            else:
                # Present tokens at random positions, so absent ones sit between them.
                order = rng.random((E, 2, C)).argsort(-1)
                present = order < n[..., None]
            b["mask"][name][:] = present
            if I:
                ids = rng.integers(1, int(self._spec["buckets"]), (E, 2, C, I), dtype=np.int32)
                ids[~present] = 0
                ids[rng.random((E, 2, C)) < 0.1] = 0  # some tokens carry no name
                b["ids"][name][:] = ids
        g = b["x"]["general"]
        fa = self.fa
        self.target = rng.integers(0, fa, (E, 2))
        g[..., 0, :fa] = 0.0
        g[..., 0, :fa][np.arange(E)[:, None], np.arange(2)[None, :], self.target] = np.where(phase == 1, 1.0, 0.0)
        g[..., 0, fa] = b["delay"] / 30.0
        g[..., 0, fa + 1] = on_screen
        g[..., 0, fa + 2] = self.hp / self.maxhp[:, None]
        g[..., 0, fa + 3] = self.hp[:, ::-1] / self.maxhp[:, None]

        legal_f = (rng.random((E, 2, fa)) < 0.7) | (np.arange(fa)[None, None, :] == self.target[..., None])
        b["legal_fight"][:] = legal_f & (phase == 1)[..., None]

        self.slot_value[:] = rng.random((E, 2, self.n_slots), dtype=np.float32)
        b["x"]["slots"][..., 0] = self.slot_value
        present = b["mask"]["slots"]
        slot_legal = present & (rng.random((E, 2, self.n_slots)) < 0.8)
        nothing = ~slot_legal.any(-1)
        slot_legal[..., 0] |= nothing & present[..., 0]
        keys = rng.random((E, 2, self.n_keys)) < 0.5
        legal_s = np.concatenate([slot_legal, keys], -1)
        b["legal_screen"][:] = legal_s & (phase == 2)[..., None]
