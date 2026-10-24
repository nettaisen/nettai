"""The league's bound on snapshots in play, its turnover, and its pruning on disk (LeagueConfig.active)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import torch

from nettai_ai.config import LeagueConfig
from nettai_ai.league import SNAP, League


class SeatsOnly:
    """A pool that only takes seats."""

    def set_seat(self, envs, seat, controller) -> None:
        pass


def test_snapshots_in_play_stay_bounded_and_turn_over(tmp_path: Path):
    E = 256
    cfg = LeagueConfig(self_play=0.4, snapshots=0.5, bots=0.1, bot_names=["still"], active=2, draining=2, cache=4, keep=12)
    rng = np.random.default_rng(3)
    league = League(cfg, SeatsOnly(), E, tmp_path, torch.device("cpu"), lambda: torch.nn.Linear(2, 2), rng)
    league.start()
    net = torch.nn.Linear(2, 2)
    played: set[int] = set()
    # Rounds of mixed lengths: most end within a few rollouts, some last tens.
    left = rng.geometric(0.15, E)
    for k in range(600):
        if k % 5 == 0:
            league.add_snapshot(net, k)
        league.refresh()
        playing = league.in_play()
        assert len(playing) <= cfg.active + cfg.draining
        assert set(league.active) <= {s.id for s in league.snapshots if s.path}
        for sid in playing:
            assert league.snapshots[sid].path and (tmp_path / "snapshots" / league.snapshots[sid].path).exists()
        played |= playing
        left -= 1
        done = np.flatnonzero(left <= 0)
        if len(done):
            result = np.zeros((E, 2), np.int64)
            result[done, 0] = rng.choice([-1, 1], len(done))
            result[done, 1] = -result[done, 0]
            league.round_end(done, result, np.zeros(E, np.int64), np.zeros(E, bool))
            left[done] = rng.geometric(0.15, len(done))
    on_disk = [s for s in league.snapshots if s.path]
    assert len(on_disk) <= cfg.keep
    assert len(list((tmp_path / "snapshots").glob("snap_*.pt"))) == len(on_disk)
    # It turns over: far more snapshots have played than fit in play at once.
    assert len(played) >= 30, played
    # The snapshots the learner beats most are the ones deleted.
    gone = [league.pfsp_weight(s) for s in league.snapshots if not s.path and s.scores]
    kept = [league.pfsp_weight(s) for s in on_disk if s.scores]
    assert gone and kept and np.median(gone) <= np.median(kept)
    assert (league.kind == SNAP).any()
