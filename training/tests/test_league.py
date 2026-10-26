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


def test_a_run_resumed_without_its_snapshots_files_draws_none_of_them(tmp_path: Path):
    """Snapshots whose files aren't on this machine are as deleted ones: their records kept, never in play."""
    cfg = LeagueConfig(self_play=0.4, snapshots=0.5, bots=0.1, bot_names=["still"], active=2, draining=2, cache=4, keep=12)
    make = lambda: League(cfg, SeatsOnly(), 64, tmp_path, torch.device("cpu"), lambda: torch.nn.Linear(2, 2), np.random.default_rng(1))
    league = make()
    league.start()
    for k in range(4):
        league.add_snapshot(torch.nn.Linear(2, 2), k)
    state = league.state_dict()
    gone = [s.path for s in league.snapshots[:3]]
    for path in gone:
        (tmp_path / "snapshots" / path).unlink()
    moved = make()
    moved.load_state_dict(state)
    assert [s.path for s in moved.snapshots[:3]] == ["", "", ""] and moved.snapshots[3].path == league.snapshots[3].path
    assert [s.update for s in moved.snapshots] == [0, 1, 2, 3]
    moved.start()
    for _ in range(20):
        moved.refresh()
        assert moved.in_play() <= {moved.snapshots[3].id}


def test_external_opponents_are_another_runs_networks(tmp_path: Path):
    """An external share draws its opponents from another run's weight files (a snapshot's state dict, or a
    checkpoint with its "model"), by PFSP: their streams are tagged EXTERNAL + index, model() loads them, results
    land on them, a file written again is read again, and resuming keeps their records."""
    from nettai_ai.league import EXT, EXTERNAL

    E = 64
    ext = tmp_path / "other" / "snapshots"
    ext.mkdir(parents=True)
    net = torch.nn.Linear(2, 2)
    torch.save(net.state_dict(), ext / "snap_00000.pt")
    torch.save({"model": net.state_dict(), "update": 7}, ext / "main.pt")
    cfg = LeagueConfig(self_play=0.2, snapshots=0.0, bots=0.0, external=0.8, external_dir=str(ext), cache=8)
    rng = np.random.default_rng(5)
    league = League(cfg, SeatsOnly(), E, tmp_path / "run", torch.device("cpu"), lambda: torch.nn.Linear(2, 2), rng)
    league.start()
    league.refresh()
    assert sorted(x.name for x in league.externals) == ["main.pt", "snap_00000.pt"]
    # The first round is self-play; the ones after draw external opponents.
    result = np.zeros((E, 2), np.int64)
    result[:, 0], result[:, 1] = 1, -1
    league.round_end(np.arange(E), result, np.zeros(E, np.int64), np.zeros(E, bool))
    ext_envs = np.flatnonzero(league.kind == EXT)
    assert len(ext_envs) > E // 2
    tags = {int(league.ctrl[2 * e + 1 - league.seat[e]]) for e in ext_envs}
    assert tags <= {EXTERNAL, EXTERNAL + 1} and len(tags) == 2
    for t in tags:
        m = league.model(t)
        assert torch.equal(m.weight, net.weight)
    league.round_end(ext_envs, result, np.zeros(E, np.int64), np.zeros(E, bool))
    assert sum(len(x.scores) for x in league.externals) == len(ext_envs)
    assert any(k.startswith("ext:") for k in league.ratings)
    # A file written again is read again.
    other = torch.nn.Linear(2, 2)
    import os, time
    torch.save({"model": other.state_dict()}, ext / "main.pt")
    later = time.time() + 5
    os.utime(ext / "main.pt", (later, later))
    league.refresh()
    i = [x.name for x in league.externals].index("main.pt")
    assert torch.equal(league.model(EXTERNAL + i).weight, other.weight)
    # Resuming keeps them.
    again = League(cfg, SeatsOnly(), E, tmp_path / "run", torch.device("cpu"), lambda: torch.nn.Linear(2, 2), rng)
    again.load_state_dict(league.state_dict())
    assert [(x.name, list(x.scores)) for x in again.externals] == [(x.name, list(x.scores)) for x in league.externals]


def test_external_opponents_in_play_stay_bounded_and_turn_over(tmp_path: Path):
    """Another run's snapshots, many and growing, are bounded in play as the run's own are (active + draining),
    turn over, and one whose file goes away leaves the active set."""
    from nettai_ai.league import EXT

    E = 256
    ext = tmp_path / "other"
    ext.mkdir()
    cfg = LeagueConfig(self_play=0.4, snapshots=0.0, bots=0.0, external=0.6, external_dir=str(ext), active=2, draining=2, cache=8)
    rng = np.random.default_rng(4)
    league = League(cfg, SeatsOnly(), E, tmp_path / "run", torch.device("cpu"), lambda: torch.nn.Linear(2, 2), rng)
    league.start()
    net = torch.nn.Linear(2, 2)
    played: set[int] = set()
    left = rng.geometric(0.15, E)
    for k in range(400):
        if k % 5 == 0:
            torch.save(net.state_dict(), ext / f"snap_{k // 5:05d}.pt")
        if k == 200:
            for i in list(league.ext_active):
                (ext / league.externals[i].name).unlink()
            gone = set(league.ext_active)
        league.refresh()
        if k == 200:
            assert not gone & set(league.ext_active)
        playing = league.ext_in_play()
        assert len(playing) <= cfg.active + cfg.draining
        played |= playing
        left -= 1
        done = np.flatnonzero(left <= 0)
        if len(done):
            result = np.zeros((E, 2), np.int64)
            result[done, 0] = rng.choice([-1, 1], len(done))
            result[done, 1] = -result[done, 0]
            league.round_end(done, result, np.zeros(E, np.int64), np.zeros(E, bool))
            left[done] = rng.geometric(0.15, len(done))
    assert len(league.externals) == 80
    # It turns over: far more have played than fit in play at once.
    assert len(played) >= 16, played
    assert (league.kind == EXT).any()


def test_a_run_plays_external_opponents(tmp_path: Path):
    """End to end on the toy pool: a run with an external share plays another run's saved network (its checkpoint
    in the external directory) and trains."""
    from nettai_ai.config import load_config
    from nettai_ai.train import Trainer

    from conftest import TINY_MODEL

    base = [
        'env.games=["exe6"]', "env.envs=8", "env.threads=1", "ppo.steps=16", "ppo.chunk=8", "ppo.epochs=1",
        "ppo.minibatch_chunks=4", "ppo.norm_warmup_steps=4", "run.checkpoint_every=2", "run.device='cpu'",
        "run.updates=2",
    ] + [f"model.{k}={v}" for k, v in TINY_MODEL.items()]
    main = Trainer(load_config(None, base), tmp_path / "main", fake=True, log=False)
    main.train()
    ckpt = sorted((tmp_path / "main" / "checkpoints").glob("ckpt_*.pt"))[-1]
    ext = tmp_path / "for_exploiter"
    ext.mkdir()
    (ext / "main.pt").write_bytes(ckpt.read_bytes())
    over = base + ["league.self_play=0.0", "league.snapshots=0.0", "league.bots=0.0", "league.external=1.0", f"league.external_dir='{ext}'", "run.updates=3"]
    exploiter = Trainer(load_config(None, over), tmp_path / "exploiter", fake=True, log=False)
    rows = exploiter.train()
    assert len(rows) == 3
    assert [x.name for x in exploiter.league.externals] == ["main.pt"]
    assert len(exploiter.league.externals[0].scores) > 0, "no round against the external opponent ended"
