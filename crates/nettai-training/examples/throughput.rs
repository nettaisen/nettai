//! The pool's throughput on the packs (docs/design/training-ai.md §6.5):
//! decisions a second for each game, on one thread and on all, both seats
//! a policy taking random allowed actions; what a round's start costs; the
//! share of an env's time spent encoding observations.
//!
//! `NETTAI_PACKS=<packs dir> cargo run --release -p nettai-training --example throughput [seconds]`

use nettai_training::env::{Config, Env, Game};
use nettai_training::pool::{Pool, PoolConfig, load_game};
use nettai_netplay::rng::SplitMix64;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn random_actions(b: &nettai_training::pool::Buffers, rng: &mut SplitMix64, out: &mut Vec<i32>) {
    use nettai_training::spec::{FIGHT_ACTIONS, SCREEN_ACTIONS};
    out.clear();
    for at in 0..b.envs * 2 {
        let legal: &[bool] = match b.phase[at] {
            1 => &b.legal_fight[at * FIGHT_ACTIONS..(at + 1) * FIGHT_ACTIONS],
            2 => &b.legal_screen[at * SCREEN_ACTIONS..(at + 1) * SCREEN_ACTIONS],
            _ => {
                out.push(0);
                continue;
            }
        };
        let n = legal.iter().filter(|&&l| l).count() as u64;
        let mut k = rng.below(n);
        let a = legal.iter().position(|&l| {
            if l {
                if k == 0 {
                    return true;
                }
                k -= 1;
            }
            false
        });
        out.push(a.unwrap() as i32);
    }
}

fn main() {
    let seconds: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let packs = nettai_content::pack::packs_dir();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    println!("packs {}, {threads} threads, {seconds} s a measure", packs.display());
    let mut rows = Vec::new();
    for game in ["exe6", "exe5", "exe4"] {
        let content = load_game(None, &packs, game).unwrap_or_else(|e| panic!("{e}"));
        // One env on this thread: where its time goes.
        let games = Arc::new(vec![Game::new(content.clone()).unwrap()]);
        nettai_battle::behavior::Behaviors::for_content(&content).unwrap();
        // What a build costs, at the rules' defaults and with every fact drawn.
        for randomize in [0.0, 1.0] {
            let maker = nettai_training::builds::Maker::new(&content).unwrap();
            let mut picks = nettai_match::Picks::new(5);
            for _ in 0..20 {
                maker.random(&mut picks, &Default::default(), randomize).unwrap();
            }
            let k = maker.counts();
            println!(
                "{game}: builds {}: {:.1} ms each, {} tries for {} ({:.0}% accepted whole), list entries kept {:.0}%",
                if randomize > 0.0 { "with every fact drawn" } else { "at the defaults" },
                k.nanos as f64 / k.builds as f64 / 1e6,
                k.tries,
                k.builds,
                100.0 * k.builds as f64 / k.tries as f64,
                100.0 * k.entries_kept as f64 / k.entries_tried.max(1) as f64
            );
        }
        let mut env = Env::new(games, Config::default(), 1).unwrap();
        let mut rng = SplitMix64::new(1);
        let mut actions = [0i32; 2];
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(seconds) {
            let out = env.step(actions).unwrap();
            for s in 0..2 {
                actions[s] = match out.phase[s] {
                    1 => rng.below(20) as i32,
                    2 => {
                        let l = &env.legal_screen[s];
                        let allowed: Vec<usize> = (0..l.len()).filter(|&i| l[i]).collect();
                        allowed[rng.below(allowed.len() as u64) as usize] as i32
                    }
                    _ => 0,
                };
            }
        }
        let wall = started.elapsed().as_nanos() as f64;
        let p = env.profile;
        let one = p.decisions as f64 / (wall / 1e9);
        println!(
            "{game}: one env inline: {:.0} decisions/s, {:.1} µs a tick, encode {:.1}%, probes {:.1}%, resets {:.1}% ({:.1} ms a round start, {} rounds, {:.0} ticks a round)",
            one,
            wall / p.ticks as f64 / 1e3,
            100.0 * p.encode as f64 / wall,
            100.0 * p.probe as f64 / wall,
            100.0 * p.reset as f64 / wall,
            p.reset as f64 / p.rounds as f64 / 1e6,
            p.rounds,
            p.ticks as f64 / p.rounds.max(1) as f64,
        );
        // The pool, on one thread and on all.
        let mut per = Vec::new();
        for (envs, t) in [(8, 1), (32, 4), (threads * 8, threads)] {
            let cfg = PoolConfig { games: vec![game.to_string()], envs, threads: t, seed: 1, content: None, packs: None, env: Config::default() };
            let made = Instant::now();
            let mut pool = Pool::with_contents(cfg, vec![content.clone()]).unwrap();
            let start_cost = made.elapsed();
            let mut rng = SplitMix64::new(2);
            let mut a = Vec::new();
            let mut decisions = 0u64;
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(seconds) {
                random_actions(pool.buffers(), &mut rng, &mut a);
                pool.step(&a).unwrap();
                decisions += pool.buffers().phase.iter().filter(|&&p| p != 0).count() as u64;
            }
            let rate = decisions as f64 / started.elapsed().as_secs_f64();
            println!("{game}: pool of {envs} envs on {t} threads: {rate:.0} decisions/s (made in {:.1} s)", start_cost.as_secs_f64());
            per.push(rate);
        }
        rows.push((game, one, per, p.reset as f64 / p.rounds as f64 / 1e6, 100.0 * p.encode as f64 / wall));
    }
    println!("\n| Game | One env inline | Pool, 1 thread | Pool, 4 threads | Pool, {threads} threads | Round start | Encoding (one env) |\n|---|---|---|---|---|---|---|");
    for (game, one, per, reset, encode) in rows {
        println!("| {game} | {one:.0} | {:.0} | {:.0} | {:.0} | {reset:.1} ms | {encode:.1}% |", per[0], per[1], per[2]);
    }
}
