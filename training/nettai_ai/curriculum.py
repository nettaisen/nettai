"""The HP curriculum (§8): rounds start at low max HP and rise to the full 1000 as the learner improves."""

from __future__ import annotations

from .config import CurriculumConfig


class Curriculum:
    def __init__(self, cfg: CurriculumConfig, updates: int):
        if not 1 <= cfg.start <= cfg.final or cfg.step < 1 or cfg.band < 0:
            raise ValueError(f"curriculum: bad HP schedule {cfg}")
        self.cfg = cfg
        self.updates = updates
        self.level = cfg.start
        self.since = 0  # the update this level began
        self.score = 0.0  # learner's score against cfg.bot at this level
        self.rounds = 0
        self.final_stretch = False

    def range(self) -> tuple[int, int]:
        c = self.cfg
        if self.final_stretch:
            return (c.final, c.final)
        return (max(c.start, self.level - c.band), self.level)

    def observe(self, records: list[tuple]) -> None:
        for _kind, opp, sc, _game in records:
            if opp == f"bot:{self.cfg.bot}":
                self.score += sc
                self.rounds += 1

    def advance(self, update: int) -> bool:
        """After update `update`: true when the HP range changed (send it with pool.set_hp)."""
        c = self.cfg
        before = self.range()
        if update >= (1.0 - c.final_fraction) * self.updates:
            self.final_stretch = True
            self.level = c.final
        elif self.level < c.final:
            good = self.rounds >= c.min_rounds and self.score / self.rounds >= c.winrate
            stale = update - self.since >= c.max_updates
            if good or stale:
                self.level = min(c.final, self.level + c.step)
                self.since = update
                self.score = 0.0
                self.rounds = 0
        return self.range() != before

    def state_dict(self) -> dict:
        return {k: getattr(self, k) for k in ("level", "since", "score", "rounds", "final_stretch")}

    def load_state_dict(self, d: dict) -> None:
        for k, v in d.items():
            setattr(self, k, v)
