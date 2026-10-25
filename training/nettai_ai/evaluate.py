"""Evaluation (§9): a checkpoint against each scripted player in each game, and against other checkpoints,
with seats mirrored; a table of results and §9's measures from `pool.round_stats()`, and Elo over everyone.

    uv run python -m nettai_ai.evaluate runs/a [--vs runs/b ...] [--games exe6,exe5,exe4] [--level normal]

round_stats() entries are read for "env", "ticks", "truncated" and "seats" (two dicts, each with "result" and any
numeric measures). Every numeric seat field is reported per minute of play. A round cut off (truncated) is
counted apart and has no result: not a draw, not a loss.

Each env's first round is discarded: seats and delays are set "from the next round" (§6.6).
"""

from __future__ import annotations

import argparse
import dataclasses
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
import torch

from . import checkpoint
from .config import LEVELS, EnvConfig, Level, config_from_dict
from .envs import PoolView, make_pool, pool_spec, read_round_stats
from .model import Model, mask_logits, previous_action, previous_width, select, warm_start
from .spec import Spec, parse_spec

TICKS_PER_MINUTE = 3600


@dataclass
class Player:
    name: str
    model: Model | None = None  # None: a scripted player
    bot: str | None = None
    k: int | None = None  # ticks its fight decisions are held, as in its training (None: the pool's)

    @property
    def scripted(self) -> bool:
        return self.model is None


def bot_player(name: str) -> Player:
    return Player(f"bot:{name}", None, name)


def load_player(path: str | Path, device: torch.device, name: str | None = None) -> tuple[Player, dict]:
    """A checkpoint's network; one from before the GRU read the previous action plays as it did (`warm_start`)."""
    state = checkpoint.load(path, device)
    cfg = config_from_dict(state["config"])
    model = Model(parse_spec(state["spec"]), cfg.model).to(device)
    warm_start(model, state["model"])
    model.eval()
    return Player(name or f"{Path(path).name}@{state['update']}", model, k=cfg.env.k), state


@torch.no_grad()
def run_matches(
    pool,
    spec: Spec,
    pairs: list[tuple[Player, Player]],
    rounds: int,
    level: Level,
    device: torch.device,
    seed: int = 1,
    duplicate: bool = False,
) -> list[dict[str, Any]]:
    """Env e plays pairs[e] = (seat 0, seat 1) until it has `rounds` counted rounds after its first.

    Returns one record per counted round: env, game index, the two players' names, the result and the
    round_stats() entry. `duplicate`: each env is dealt the same rounds as the env of the same pair the other
    way round (`pool.set_deal`), so each deal is played twice with the seats swapped; a record's "deal" is
    (its key, the round's number), None otherwise.
    """
    view = PoolView(pool, spec)
    E, S = view.E, view.S
    if len(pairs) != E:
        raise ValueError(f"run_matches: {len(pairs)} pairs for {E} envs")
    players: list[Player] = []
    index: dict[int, int] = {}
    stream_player = np.zeros(S, np.int64)
    for e, pair in enumerate(pairs):
        for seat, p in enumerate(pair):
            if id(p) not in index:
                index[id(p)] = len(players)
                players.append(p)
            stream_player[2 * e + seat] = index[id(p)]
    for seat in (0, 1):
        groups: dict[str, list[int]] = {}
        for e, pair in enumerate(pairs):
            groups.setdefault(pair[seat].bot or "policy", []).append(e)
        for ctl, es in groups.items():
            pool.set_seat(np.asarray(es, np.int64), seat, ctl)
            pool.set_delay(np.asarray(es, np.int64), seat, None if ctl != "policy" else level.delay)
        # Each network decides at the k it was trained at.
        ks: dict[int | None, list[int]] = {}
        for e, pair in enumerate(pairs):
            ks.setdefault(pair[seat].k, []).append(e)
        for k, es in ks.items():
            pool.set_k(np.asarray(es, np.int64), seat, k)
    keys = duplicate_keys(pairs) if duplicate else [None] * E
    if duplicate:
        by_key: dict[int, list[int]] = {}
        for e, k in enumerate(keys):
            by_key.setdefault(k, []).append(e)
        for k, es in by_key.items():
            pool.set_deal(np.asarray(es, np.int64), k)
    hidden = {i: torch.zeros(S, p.model.cfg.hidden, device=device) for i, p in enumerate(players) if not p.scripted}
    # Each stream's previous decision's action (none at a round's start), as the network reads it.
    no_previous = previous_width(spec) - 1
    previous = torch.full((S,), no_previous, dtype=torch.int64)
    gen = torch.Generator(device=device)
    gen.manual_seed(seed)
    started = np.zeros(E, np.int64)  # rounds ended per env
    counted = np.zeros(E, np.int64)
    actions = np.zeros((E, 2), np.int32)
    acts = actions.reshape(-1)
    sp = torch.from_numpy(stream_player)
    records: list[dict[str, Any]] = []
    while (counted < rounds).any():
        if view.done.any():
            stats = {int(r["env"]): r for r in read_round_stats(pool)}
            for e in view.done.nonzero().squeeze(-1).tolist():
                if started[e] >= 1 and counted[e] < rounds:
                    if e not in stats:
                        raise RuntimeError(f"round_stats() has no entry for env {e}'s round")
                    records.append(
                        {
                            "env": e,
                            "game": int(view.game[e]),
                            "players": (pairs[e][0].name, pairs[e][1].name),
                            "result": (int(view.result[2 * e]), int(view.result[2 * e + 1])),
                            "truncated": bool(view.truncated[e]),
                            "stats": stats[e],
                            "deal": None if keys[e] is None else (keys[e], int(counted[e])),
                        }
                    )
                    counted[e] += 1
                started[e] += 1
                for h in hidden.values():
                    h[2 * e : 2 * e + 2] = 0.0
                previous[2 * e : 2 * e + 2] = no_previous
        phase = view.phase.to(torch.int64)
        deciding = phase != 0
        actions.fill(0)
        for i, p in enumerate(players):
            idx = (deciding & (sp == i)).nonzero().squeeze(-1)
            if len(idx) == 0:
                continue
            lf, ls = view.legal_fight[idx].to(device), view.legal_screen[idx].to(device)
            ph = phase[idx].to(device)
            if p.scripted:
                # Only in an env's first, discarded round, before set_seat has taken effect.
                fl = torch.zeros(lf.shape, device=device)
                sl = torch.zeros(ls.shape, device=device)
                a, _ = select(mask_logits(fl, lf), mask_logits(sl, ls), ph, generator=gen)
                if (started[(idx // 2).numpy()] >= 1).any():
                    raise RuntimeError(f"{p.name}'s seat has a decision after its first round")
            else:
                di = idx.to(device)
                fight, screen, _, h = p.model.step(view.obs(idx, device), hidden[i][di], previous[idx].to(device))
                hidden[i][di] = h
                a, _ = select(mask_logits(fight, lf), mask_logits(screen, ls), ph, level, gen)
            a = a.cpu()
            acts[idx.numpy()] = a.to(torch.int32).numpy()
            previous[idx] = previous_action(a, phase[idx], spec.fight_actions)
        pool.step(actions)
    return records


def duplicate_keys(pairs: list[tuple[Player, Player]]) -> list[int]:
    """A deal's key for each env, shared with one env of the same pair the other way round."""
    keys: list[int | None] = [None] * len(pairs)
    n = 0
    for e, (a, b) in enumerate(pairs):
        if keys[e] is not None:
            continue
        partner = next((f for f in range(e + 1, len(pairs)) if keys[f] is None and pairs[f][0] is b and pairs[f][1] is a), None)
        if partner is None:
            raise ValueError(f"duplicate: env {e}'s pair ({a.name}, {b.name}) has no env the other way round")
        keys[e] = keys[partner] = n
        n += 1
    return keys  # type: ignore[return-value]


def duplicate_summary(records: list[dict[str, Any]], games: list[str]) -> dict[tuple, dict[str, int]]:
    """By (game, player, opponent), the players in name order: of the deals played twice with the seats
    swapped, how many the first won both times, the second both times, each once (the deal decided), and with
    a draw or a cut round."""
    by_deal: dict[tuple, list[dict[str, Any]]] = {}
    for r in records:
        if r.get("deal") is not None:
            # (Each call of run_matches keys its deals from 0: its players tell them apart.)
            by_deal.setdefault((r["game"], tuple(sorted(r["players"])), tuple(r["deal"])), []).append(r)
    out: dict[tuple, dict[str, int]] = {}
    for (g, _, _), rs in by_deal.items():
        if len(rs) != 2:
            raise RuntimeError(f"duplicate: a deal played {len(rs)} times")
        p, q = sorted(rs[0]["players"])
        row = out.setdefault((games[g], p, q), {"deals": 0, "first": 0, "second": 0, "split": 0, "other": 0})
        row["deals"] += 1
        if any(r["truncated"] for r in rs):
            row["other"] += 1
            continue
        mine = [r["result"][r["players"].index(p)] if r["players"][0] != r["players"][1] else r["result"][0] for r in rs]
        if 0 in mine:
            row["other"] += 1
        elif mine[0] > 0 and mine[1] > 0:
            row["first"] += 1
        elif mine[0] < 0 and mine[1] < 0:
            row["second"] += 1
        else:
            row["split"] += 1
    return out


def format_duplicates(summary: dict[tuple, dict[str, int]]) -> str:
    head = ["game", "player", "opponent", "deals", "won both", "lost both", "split", "draw/cut", "split%"]
    rows = []
    totals = {"first": 0, "second": 0, "split": 0}
    for (g, p, q), row in sorted(summary.items()):
        decided = row["first"] + row["second"] + row["split"]
        for k in totals:
            totals[k] += row[k]
        rows.append([g, p, q, str(row["deals"]), str(row["first"]), str(row["second"]), str(row["split"]), str(row["other"]), f"{100 * row['split'] / max(decided, 1):.0f}"])
    widths = [max(len(h), *(len(r[i]) for r in rows)) if rows else len(h) for i, h in enumerate(head)]
    line = lambda cells: "  ".join(c.ljust(w) for c, w in zip(cells, widths))
    decided = sum(totals.values())
    foot = f"all: {decided} deals decided both times, split {100 * totals['split'] / max(decided, 1):.0f}% (50% if the deal didn't matter and the two were even; 100% if the deal decided every round)"
    return "\n".join([line(head), line(["-" * w for w in widths])] + [line(r) for r in rows] + ["", foot])


def bot_pairs(player: Player, bots: list[str], envs: int) -> list[tuple[Player, Player]]:
    """Envs split evenly over the bots and both seats."""
    if not bots:
        raise ValueError("evaluation: no scripted players to play against")
    if envs < 2 * len(bots) or envs % (2 * len(bots)):
        raise ValueError(f"evaluation: envs ({envs}) must be a multiple of 2 × {len(bots)} bots")
    bp = [bot_player(b) for b in bots]
    out = []
    for e in range(envs):
        b = bp[e % len(bots)]
        out.append((player, b) if (e // len(bots)) % 2 == 0 else (b, player))
    return out


def round_robin_pairs(players: list[Player], envs: int) -> list[tuple[Player, Player]]:
    matchups = [(a, b) for i, a in enumerate(players) for b in players[i + 1 :]]
    if not matchups:
        return []
    if envs < 2 * len(matchups) or envs % (2 * len(matchups)):
        raise ValueError(f"evaluation: envs ({envs}) must be a multiple of 2 × {len(matchups)} pairings")
    out = []
    for e in range(envs):
        a, b = matchups[e % len(matchups)]
        out.append((a, b) if (e // len(matchups)) % 2 == 0 else (b, a))
    return out


def summarize(records: list[dict[str, Any]], games: list[str]) -> dict[tuple, dict[str, Any]]:
    """By (game, player, opponent): rounds, wins, draws, losses, ticks, and each numeric measure's sum."""
    out: dict[tuple, dict[str, Any]] = {}
    for r in records:
        g = games[r["game"]]
        for seat in (0, 1):
            me, other = r["players"][seat], r["players"][1 - seat]
            row = out.setdefault((g, me, other), {"rounds": 0, "win": 0, "draw": 0, "loss": 0, "cut": 0, "ticks": 0, "measures": {}})
            if r["truncated"]:  # cut off: no result, no measures
                row["cut"] += 1
                continue
            res = r["result"][seat]
            row["rounds"] += 1
            row["win" if res > 0 else ("loss" if res < 0 else "draw")] += 1
            row["ticks"] += int(r["stats"]["ticks"])
            for k, val in r["stats"]["seats"][seat].items():
                if k != "result" and isinstance(val, (int, float)) and not isinstance(val, bool):
                    row["measures"][k] = row["measures"].get(k, 0.0) + float(val)
    return out


def elo(results: list[tuple[str, str, float]], anchors: list[str], prior: float = 0.01, iters: int = 3000) -> dict[str, float]:
    """Bradley–Terry ratings by maximum likelihood with a small L2 prior (so a perfect record stays finite),
    on the Elo scale; the anchors' mean is 1000 (all players' mean when there are no anchors)."""
    names = sorted({a for a, _, _ in results} | {b for _, b, _ in results})
    if not names:
        return {}
    ix = {n: i for i, n in enumerate(names)}
    a = np.array([ix[x] for x, _, _ in results])
    b = np.array([ix[y] for _, y, _ in results])
    s = np.array([z for _, _, z in results], np.float64)
    r = np.zeros(len(names))
    lr = 1.0 / max(1, len(results)) * len(names)
    for _ in range(iters):
        p = 1.0 / (1.0 + np.exp(-(r[a] - r[b])))
        g = np.zeros_like(r)
        np.add.at(g, a, s - p)
        np.add.at(g, b, p - s)
        g -= prior * r
        r += lr * g
    scale = 400.0 / math.log(10.0)
    ref = [ix[n] for n in anchors if n in ix] or list(range(len(names)))
    r = r - r[ref].mean()
    return {n: 1000.0 + scale * r[ix[n]] for n in names}


def elo_results(records: list[dict[str, Any]]) -> list[tuple[str, str, float]]:
    out = []
    for r in records:
        if r["truncated"]:
            continue
        res = r["result"][0]
        out.append((r["players"][0], r["players"][1], 1.0 if res > 0 else (0.0 if res < 0 else 0.5)))
    return out


def format_table(summary: dict[tuple, dict[str, Any]], players: list[str]) -> str:
    measures = sorted({k for row in summary.values() for k in row["measures"]})
    head = ["game", "player", "opponent", "rounds", "cut", "win%", "draw%", "loss%"] + [f"{m}/min" for m in measures]
    rows = []
    for (g, me, other), row in sorted(summary.items()):
        if me not in players:
            continue
        n = max(row["rounds"], 1)
        minutes = max(row["ticks"], 1) / TICKS_PER_MINUTE
        rows.append(
            [g, me, other, str(row["rounds"]), str(row["cut"])]
            + [f"{100 * row[k] / n:.1f}" for k in ("win", "draw", "loss")]
            + [f"{row['measures'].get(m, 0.0) / minutes:.1f}" for m in measures]
        )
    widths = [max(len(h), *(len(r[i]) for r in rows)) if rows else len(h) for i, h in enumerate(head)]
    line = lambda cells: "  ".join(c.ljust(w) for c, w in zip(cells, widths))
    return "\n".join([line(head), line(["-" * w for w in widths])] + [line(r) for r in rows])


def evaluate_vs_bots(
    player: Player,
    env: EnvConfig,
    fake: bool,
    games: list[str],
    bots: list[str],
    envs: int,
    threads: int,
    rounds: int,
    level: Level,
    hp: int,
    device: torch.device,
    spec_hash: str,
    pools: dict[str, Any] | None = None,
    duplicate: bool = False,
) -> list[dict[str, Any]]:
    """player against each bot in each game (one pool per game; pass `pools` to keep them between calls)."""
    records = []
    for gi, g in enumerate(games):
        pool = pools.get(g) if pools is not None else None
        if pool is None:
            pool = make_pool(env, fake, games=[g], envs=envs, threads=threads, seed=env.seed + 1000 + gi, hp=(hp, hp), holdout_mode="favor")
            if pools is not None:
                pools[g] = pool
        spec = parse_spec(pool_spec(pool))
        if spec.hash != spec_hash:
            raise RuntimeError(f"evaluation: the pool's spec hash {spec.hash} isn't the network's {spec_hash}")
        recs = run_matches(pool, spec, bot_pairs(player, bots, envs), rounds, level, device, duplicate=duplicate)
        for r in recs:
            r["game"] = gi
        records += recs
    return records


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("checkpoint", help="a checkpoint file or a run directory (its latest checkpoint)")
    ap.add_argument("--vs", nargs="*", default=[], help="other checkpoints to play (round robin with the first)")
    ap.add_argument("--games", default=None, help="comma-separated (default: the checkpoint's training games)")
    ap.add_argument("--bots", default="still,mover,masher,shooter,dodger")
    ap.add_argument("--no-bots", action="store_true")
    ap.add_argument("--rounds", type=int, default=4, help="counted rounds per env")
    ap.add_argument("--envs", type=int, default=40)
    ap.add_argument("--threads", type=int, default=0)
    ap.add_argument("--level", default="normal", choices=sorted(LEVELS))
    ap.add_argument("--delay", type=int, default=None, help="the level's reaction delay in ticks, overridden")
    ap.add_argument("--hp", type=int, default=1000)
    ap.add_argument("--fake", action="store_true", help="play FakePool instead of nettai_gym")
    ap.add_argument("--duplicate", action="store_true", help="each deal twice, the seats swapped (pool.set_deal)")
    ap.add_argument("--records", default=None, help="write every counted round's record here (JSON lines)")
    ap.add_argument("--device", default="cpu")
    a = ap.parse_args()
    device = torch.device(a.device)
    main_player, state = load_player(a.checkpoint, device)
    cfg = config_from_dict(state["config"])
    games = a.games.split(",") if a.games else list(cfg.env.games)
    bots = [] if a.no_bots else [b for b in a.bots.split(",") if b]
    level = LEVELS[a.level]
    if a.delay is not None:
        level = dataclasses.replace(level, delay=a.delay)
    others = [load_player(p, device)[0] for p in a.vs]
    everyone = [main_player] + others
    for p in others:
        if p.model.spec.hash != main_player.model.spec.hash:
            raise SystemExit(f"{p.name} was trained on another observation spec than {main_player.name}")
    records = []
    for p in everyone if bots else []:
        records += evaluate_vs_bots(
            p, cfg.env, a.fake, games, bots, a.envs, a.threads, a.rounds, level, a.hp, device, main_player.model.spec.hash,
            duplicate=a.duplicate,
        )
    if others:
        for gi, g in enumerate(games):
            pool = make_pool(cfg.env, a.fake, games=[g], envs=a.envs, threads=a.threads, seed=cfg.env.seed + 2000 + gi, hp=(a.hp, a.hp), holdout_mode="favor")
            spec = parse_spec(pool_spec(pool))
            if spec.hash != main_player.model.spec.hash:
                raise SystemExit(f"the pool's spec hash {spec.hash} isn't the network's {main_player.model.spec.hash}")
            recs = run_matches(pool, spec, round_robin_pairs(everyone, a.envs), a.rounds, level, device, duplicate=a.duplicate)
            for r in recs:
                r["game"] = gi
            records += recs
    if a.records:
        import json

        with open(a.records, "w") as f:
            for r in records:
                f.write(json.dumps({**r, "game": games[r["game"]]}, default=str) + "\n")
    summary = summarize(records, games)
    print(f"level {level.name}: delay {level.delay} ticks, temperature {level.temperature}, epsilon {level.epsilon}, greedy {level.greedy}")
    print(format_table(summary, [p.name for p in everyone]))
    if a.duplicate:
        print("\nDuplicate deals (each played twice, the seats swapped):")
        print(format_duplicates(duplicate_summary(records, games)))
    ratings = elo(elo_results(records), [f"bot:{b}" for b in bots])
    print("\nElo (bots' mean = 1000)" if bots else "\nElo (mean = 1000)")
    for n, r in sorted(ratings.items(), key=lambda kv: -kv[1]):
        print(f"  {r:7.0f}  {n}")


if __name__ == "__main__":
    main()
