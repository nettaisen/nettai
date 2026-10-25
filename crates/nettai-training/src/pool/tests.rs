use super::*;
use crate::testing::GAMES;
use nettai_netplay::rng::SplitMix64;

fn pool(envs: usize, threads: usize, seed: u64, env: Config) -> Pool {
    pool_with(envs, threads, 0, seed, env)
}

fn pool_with(envs: usize, threads: usize, builders: usize, seed: u64, env: Config) -> Pool {
    let cfg = PoolConfig { games: GAMES.iter().map(|g| g.to_string()).collect(), envs, threads, seed, content: None, packs: None, env, cache: None, builders };
    Pool::with_games(cfg, crate::testing::games(&GAMES)).unwrap_or_else(|e| panic!("{e}"))
}

/// Random allowed actions for every seat the buffers ask.
fn actions(b: &Buffers, rng: &mut SplitMix64) -> Vec<i32> {
    (0..b.envs * 2)
        .map(|at| {
            let legal: &[bool] = match b.phase[at] {
                1 => &b.legal_fight[at * spec::FIGHT_ACTIONS..(at + 1) * spec::FIGHT_ACTIONS],
                2 => &b.legal_screen[at * spec::SCREEN_ACTIONS..(at + 1) * spec::SCREEN_ACTIONS],
                _ => return 0,
            };
            let allowed: Vec<usize> = (0..legal.len()).filter(|&i| legal[i]).collect();
            assert!(!allowed.is_empty(), "seat {at}: nothing allowed");
            allowed[rng.below(allowed.len() as u64) as usize] as i32
        })
        .collect()
}

/// What the buffers hold, as numbers, to compare.
fn snapshot(b: &Buffers) -> Vec<u64> {
    let mut out: Vec<u64> = Vec::new();
    for s in 0..spec::SETS.len() {
        out.extend(b.x[s].iter().map(|v| v.to_bits() as u64));
        out.extend(b.ids[s].iter().map(|&v| v as u64));
        out.extend(b.mask[s].iter().map(|&v| v as u64));
    }
    out.extend(b.phase.iter().map(|&v| v as u64));
    out.extend(b.legal_fight.iter().chain(&b.legal_screen).chain(&b.done).chain(&b.truncated).map(|&v| v as u64));
    out.extend(b.hp_gain.iter().chain(&b.credit_hp).chain(&b.dealt).chain(&b.taken).map(|v| v.to_bits() as u64));
    out.extend(b.same_row.iter().map(|&v| v as u64));
    out.extend(b.credit_lag.iter().map(|&v| v as u64));
    out.extend(b.result.iter().chain(&b.game).map(|&v| v as u64));
    out.extend(b.delay.iter().map(|&v| v as u64));
    out
}

/// The buffers fill as §6.6 says: every seat a policy plays is asked in
/// turn, with its observation (the global token there), what it may do,
/// and its game and delay; a round's end comes with its result; and the
/// same seed plays the same, whatever the number of threads, and with
/// threads of their own making the builds ahead (full lists every
/// randomized build: the slow ones).
#[test]
fn the_pools_buffers_fill_and_play_alike_on_any_threads() {
    let cfg = Config { max_fight_ticks: 1200, full_lists: 1.0, ..Config::default() };
    let mut one = pool(6, 1, 9, cfg.clone());
    let mut built = pool_with(6, 3, 2, 9, cfg.clone());
    let mut three = pool(6, 3, 9, cfg);
    let (mut rng, mut rng3) = (SplitMix64::new(1), SplitMix64::new(1));
    let mut seen = [0u32; 3];
    let mut ended = 0;
    let mut games = std::collections::BTreeSet::new();
    for step in 0..1500 {
        assert_eq!(snapshot(one.buffers()), snapshot(three.buffers()), "step {step}");
        assert_eq!(snapshot(one.buffers()), snapshot(built.buffers()), "step {step}: with builders");
        let b = one.buffers();
        for e in 0..b.envs {
            games.insert(b.game[e]);
            assert!((b.game[e] as usize) < GAMES.len());
            for s in 0..2 {
                let at = e * 2 + s;
                let phase = b.phase[at];
                seen[phase as usize] += 1;
                assert!((6..=30).contains(&b.delay[at]));
                if phase != 0 {
                    let g = &spec::SETS[spec::GENERAL];
                    assert!(b.mask[spec::GENERAL][at * g.count], "env {e} seat {s}: no general token");
                }
            }
            if b.done[e] {
                ended += 1;
                assert_eq!(b.result[e * 2] + b.result[e * 2 + 1], 0);
            }
        }
        let a = actions(one.buffers(), &mut rng);
        assert_eq!(a, actions(three.buffers(), &mut rng3));
        one.step(&a).unwrap_or_else(|e| panic!("{e}"));
        three.step(&a).unwrap_or_else(|e| panic!("{e}"));
        built.step(&a).unwrap_or_else(|e| panic!("{e}"));
    }
    let stats = one.round_stats().unwrap();
    println!("phases {seen:?}, {ended} rounds ended, games {games:?}, {} rounds' stats", stats.len());
    assert_eq!(stats.len(), ended);
    assert!(seen[1] > 0 && seen[2] > 0 && ended > 0 && games.len() > 1);
}

/// Seats set to scripted players from the next round are never asked;
/// with no policy seat, a step plays a whole round. A delay pinned is
/// played at; an action that isn't allowed is refused.
#[test]
fn scripted_seats_delays_and_refusals() {
    let mut p = pool(2, 2, 3, Config { max_fight_ticks: 1200, ..Config::default() });
    if p.buffers().phase[0] != 0 {
        let before = snapshot(p.buffers());
        assert!(p.step(&[99, 0, 0, 0]).is_err());
        assert_eq!(snapshot(p.buffers()), before, "a refused step moves nothing");
    }
    p.set_seat(&[0, 1], 0, controller("shooter").unwrap()).unwrap();
    p.set_seat(&[0, 1], 1, controller("dodger").unwrap()).unwrap();
    p.set_delay(&[1], 0, Some(9)).unwrap();
    assert!(controller("nobody").is_err());
    let mut rng = SplitMix64::new(4);
    let mut done = [0; 2];
    for _ in 0..3000 {
        let a = actions(p.buffers(), &mut rng);
        p.step(&a).unwrap();
        for (d, &ended) in done.iter_mut().zip(&p.buffers().done) {
            *d += ended as u32;
        }
        if done.iter().all(|&d| d >= 2) {
            break;
        }
    }
    assert!(done.iter().all(|&d| d >= 2), "rounds ended {done:?}");
    // (Past their first round, both seats are scripted: never asked.)
    assert!(p.buffers().phase.iter().all(|&ph| ph == 0));
    assert_eq!(p.buffers().delay[2], 9);
    let stats = p.round_stats().unwrap();
    assert!(stats.iter().any(|r| r.controllers == ["shooter".to_string(), "dodger".to_string()]));
}
