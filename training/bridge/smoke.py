"""A smoke test of the bridge (docs/design/training-ai.md §6.6): a pool on the
content, random allowed actions for every seat asked, N steps; prints the
spec's shapes, decisions a second, and the rounds that ended.

    cd training/bridge
    uv venv .venv && uv pip install --python .venv/bin/python maturin numpy
    VIRTUAL_ENV=$PWD/.venv PATH=$PWD/.venv/bin:$PATH maturin develop --release
    .venv/bin/python smoke.py [steps] [envs] [threads] [games...]
"""

import sys
import time

import numpy as np

import nettai_gym


def main() -> None:
    steps = int(sys.argv[1]) if len(sys.argv) > 1 else 300
    envs = int(sys.argv[2]) if len(sys.argv) > 2 else 16
    threads = int(sys.argv[3]) if len(sys.argv) > 3 else 4
    games = sys.argv[4:] or ["exe6", "exe5", "exe4"]

    made = time.perf_counter()
    pool = nettai_gym.Pool(games=games, envs=envs, threads=threads, seed=1)
    print(f"pool of {envs} envs on {threads} threads, {games}: made in {time.perf_counter() - made:.1f} s")

    spec = pool.spec()
    print(f"spec {spec['hash']}, {spec['buckets']} buckets, {spec['fight_actions']} fight and {spec['screen_actions']} screen actions")
    buf = pool.buffers()
    for name, s in spec["sets"].items():
        x, ids, mask = buf["x"][name], buf["ids"][name], buf["mask"][name]
        assert x.shape == (envs, 2, s["count"], s["width"]) and x.dtype == np.float32, (name, x.shape)
        assert ids.shape == (envs, 2, s["count"], s["ids"]) and ids.dtype == np.int32
        assert mask.shape == (envs, 2, s["count"]) and mask.dtype == np.bool_
        print(f"  {name}: {s['count']} x {s['width']} (+{s['ids']} ids)")
    assert nettai_gym.name_ids("exe6:cannon") == nettai_gym.name_ids("cannon") == (834, 2843)
    assert all(name.isidentifier() for name in spec["sets"]) and "slots" in spec["sets"]
    arrays = {k: id(v) for k, v in buf.items() if not isinstance(v, dict)}

    rng = np.random.default_rng(1)
    actions = np.zeros((envs, 2), dtype=np.int32)
    decisions = rounds = 0
    phase, legal_fight, legal_screen = buf["phase"], buf["legal_fight"], buf["legal_screen"]
    started = time.perf_counter()
    for _ in range(steps):
        # (A random allowed action for each seat asked: the buffers are the
        # pool's own memory, which each step refills.)
        noise = rng.random((envs, 2, max(spec["fight_actions"], spec["screen_actions"])), dtype=np.float32)
        fight = np.where(legal_fight, noise[..., : spec["fight_actions"]], -1.0).argmax(-1)
        screen = np.where(legal_screen, noise[..., : spec["screen_actions"]], -1.0).argmax(-1)
        actions[:] = np.where(phase == 1, fight, np.where(phase == 2, screen, 0))
        decisions += int((phase != 0).sum())
        pool.step(actions)
        rounds += int(buf["done"].sum())
    elapsed = time.perf_counter() - started

    assert buf["phase"] is phase and all(id(v) == arrays[k] for k, v in buf.items() if not isinstance(v, dict)), "the same arrays"
    p = pool.profile()
    print(f"{decisions} decisions in {elapsed:.1f} s: {decisions / elapsed:.0f} decisions/s; {rounds} rounds ended")
    print(f"  envs' time: encoding {p['encode_ns'] / 1e9:.2f} s, round starts {p['reset_ns'] / 1e9:.2f} s, key probes {p['probe_ns'] / 1e9:.2f} s, builds made ahead {p['prepare_ns'] / 1e9:.2f} s; {p['ticks']} ticks")
    pairs = pool.pairs()
    print("  pairs of chips:", {g: len(p) for g, p in pairs.items()}, "e.g.", [p[0] for p in pairs.values() if p])
    stats = pool.round_stats()
    seeded = [s for r in stats for s in r["seats"] if s["pair"]]
    print(f"  {len(seeded)} seats of {2 * len(stats)} seeded with a pair: picked {sum(s['pair_picked'] for s in seeded)}, in order {sum(s['pair_ordered'] for s in seeded)}, used in order {sum(s['pair_used'] for s in seeded)}")
    for r in stats[:6]:
        seats = r["seats"]
        print(f"  env {r['env']} {r['game']}: {[s['navi'] for s in seats]} result {[s['result'] for s in seats]} in {r['ticks']} ticks, {r['screens']} screens, chips picked {[s['chips_picked'] for s in seats]}, cut {r['truncated']} {r['error'] or ''}")
    assert len(stats) == rounds, (len(stats), rounds)


if __name__ == "__main__":
    main()
