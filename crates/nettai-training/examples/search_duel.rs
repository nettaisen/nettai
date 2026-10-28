//! The lookahead against the network alone (training-ai.md §10.7): NetAi with
//! `search::Search` against the same weights' NetAi without it, both at one
//! level, over duplicate deals (each deal's builds and seed played twice, the
//! sides swapped) in each game asked for, the builds drawn as the env draws
//! them (randomize 0.75, full lists 0.5, combos 0.3, HP 1000).
//!
//!     cargo run --release -p nettai-training --features live --example search_duel -- WEIGHTS \
//!         [--deals N] [--games exe6,exe5,exe4] [--level hard] [--seed S] [--threads T] [--cache DIR] \
//!         [--candidates N] [--horizon T] [--prior P]
//!
//! It prints, by game, the deals the lookahead won both ways, lost both ways and split, and what a fight
//! decision's lookahead cost.

use nettai_battle::Battle;
use nettai_battle::battle::{fight, mode};
use nettai_frontend::driver::Ai;
use nettai_match::{Picks, Set};
use nettai_netplay::rng::SplitMix64;
use nettai_training::ai::{Fingerprinter, Level, NetAi};
use nettai_training::builds::{Combos, Holdout};
use nettai_training::env::{self, Game};
use nettai_training::fingerprint::Fingerprints;
use nettai_training::net::Net;
use nettai_training::search::Search;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_FIGHT_TICKS: u32 = 18000;
const MAX_SCREEN_TICKS: u32 = 3600;

struct Args {
    weights: PathBuf,
    deals: usize,
    games: Vec<String>,
    level: Level,
    seed: u64,
    threads: usize,
    cache: PathBuf,
    search: Search,
}

fn args() -> Args {
    let mut it = std::env::args().skip(1);
    let usage = "usage: search_duel <weights> [--deals N] [--games exe6,exe5,exe4] [--level hard] [--seed S] [--threads T] [--cache DIR] [--candidates N] [--horizon T] [--prior P]";
    let weights = PathBuf::from(it.next().expect(usage));
    let mut a = Args {
        weights,
        deals: 100,
        games: ["exe6", "exe5", "exe4"].map(String::from).to_vec(),
        level: Level::Hard,
        seed: 1,
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        cache: PathBuf::from("fingerprints"),
        search: Search::default(),
    };
    while let Some(flag) = it.next() {
        let value = it.next().expect(usage);
        match flag.as_str() {
            "--deals" => a.deals = value.parse().expect(usage),
            "--games" => a.games = value.split(',').map(String::from).collect(),
            "--level" => a.level = Level::ALL.into_iter().find(|l| l.name() == value).expect(usage),
            "--seed" => a.seed = value.parse().expect(usage),
            "--threads" => a.threads = value.parse().expect(usage),
            "--cache" => a.cache = PathBuf::from(value),
            "--candidates" => a.search.candidates = value.parse().expect(usage),
            "--horizon" => a.search.horizon = value.parse().expect(usage),
            "--prior" => a.search.prior = value.parse().expect(usage),
            _ => panic!("{usage}"),
        }
    }
    a
}

/// One game's: its content, fingerprints and build maker.
struct Played {
    game: Game,
    fingerprints: Arc<Fingerprints>,
}

/// A round's result for side 0 (+1 won, −1 lost, 0 neither), and the time
/// each side's AI took.
struct Round {
    result: i8,
    took: [Duration; 2],
    fight_ticks: u32,
}

/// Plays one round of `m` (seeded `seed`), the lookahead on side `searcher`.
fn round(p: &Played, net: &Arc<Net>, a: &Args, m: &nettai_match::Match, seed: u32, searcher: u8, ai_seed: u64) -> Round {
    let content = &p.game.content;
    let mut b: Battle = Set::of(content, m, seed).start();
    let make = |s: u8| {
        let ai = NetAi::of(net.clone(), a.level, ai_seed ^ s as u64, Fingerprinter::made(p.fingerprints.clone())).expect("the AI");
        if s == searcher { ai.with_search(a.search) } else { ai }
    };
    let mut ais = [make(0), make(1)];
    let mut took = [Duration::ZERO; 2];
    let (mut t, mut fight_ticks, mut screen_ticks) = (0u32, 0u32, 0u32);
    let result = loop {
        if matches!(b.round_end(), Some(nettai_battle::RoundEnd::Error(_))) {
            break 0;
        }
        if b.round.mode == mode::FIGHTING && matches!(b.fight.state, fight::WIN | fight::LOSE | fight::DRAW) {
            let local_won: i8 = match b.fight.state {
                fight::WIN => 1,
                fight::LOSE => -1,
                _ => 0,
            };
            break local_won * if b.round.local_side & 1 == 0 { 1 } else { -1 };
        }
        if fight_ticks > MAX_FIGHT_TICKS || screen_ticks > MAX_SCREEN_TICKS || t > 4 * (MAX_FIGHT_TICKS + MAX_SCREEN_TICKS) {
            break 0;
        }
        let custom = b.round.mode == mode::CUSTOM;
        screen_ticks = if custom { screen_ticks + 1 } else { 0 };
        let mut press = [0u16; 2];
        for s in 0..2u8 {
            let at = Instant::now();
            press[s as usize] = if custom { ais[s as usize].custom_screen(&b, s) } else { ais[s as usize].fight(&b, s, 0) };
            took[s as usize] += at.elapsed();
        }
        if !env::step_battle(&mut b, press) {
            break 0;
        }
        t += 1;
        fight_ticks += (b.round.mode == mode::FIGHTING) as u32;
    };
    Round { result, took, fight_ticks }
}

#[derive(Default, Clone, Copy)]
struct Tally {
    won_both: u32,
    lost_both: u32,
    split: u32,
    undecided: u32,
    took: [Duration; 2],
    fight_ticks: u64,
}

fn main() {
    let a = args();
    env::quiet_engine_panics();
    let net = Arc::new(Net::from_safetensors(&std::fs::read(&a.weights).expect("the weights")).expect("the network"));
    let played: Vec<Played> = a
        .games
        .iter()
        .map(|g| {
            let content = nettai_training::pool::load_game(None, g).expect("the game's content");
            let fingerprints = Fingerprints::for_game(&content, a.threads, Some(a.cache.as_path())).expect("the fingerprints");
            let game = Game::with_fingerprints(content, fingerprints.clone()).expect("the game");
            Played { game, fingerprints }
        })
        .collect();
    eprintln!("search_duel: {} deals a game in {:?}, {} level, lookahead {:?}", a.deals, a.games, a.level.name(), a.search);
    let jobs: Vec<(usize, usize)> = (0..played.len()).flat_map(|g| (0..a.deals).map(move |d| (g, d))).collect();
    let next = AtomicUsize::new(0);
    let tallies: Mutex<Vec<Tally>> = Mutex::new(vec![Tally::default(); played.len()]);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..a.threads.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(g, d)) = jobs.get(i) else { break };
                    let p = &played[g];
                    let mut rng = SplitMix64::new(a.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ ((g * 1_000_003 + d) as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
                    let combos = match p.game.names.fingerprints() {
                        Some(f) => Combos { pairs: &f.pairs, rate: 0.3 },
                        None => Combos::NONE,
                    };
                    let build = |rng: &mut SplitMix64| p.game.maker.random_with(&mut Picks::new(rng.next_u64() as u32), &Holdout::default(), 0.75, 0.5, combos).expect("a build");
                    let (b0, b1) = (build(&mut rng), build(&mut rng));
                    let seed = rng.next_u64() as u32;
                    let m = p.game.maker.round_with([b0.side.clone(), b1.side.clone()], Some(1000), seed).expect("a round");
                    let ai_seed = rng.next_u64();
                    // The lookahead on side 0, then on side 1: its result each time.
                    let first = round(p, &net, &a, &m, seed, 0, ai_seed);
                    let second = round(p, &net, &a, &m, seed, 1, ai_seed);
                    let (r0, r1) = (first.result, -second.result);
                    let mut all = tallies.lock().unwrap();
                    let t = &mut all[g];
                    match (r0, r1) {
                        (1, 1) => t.won_both += 1,
                        (-1, -1) => t.lost_both += 1,
                        (1, -1) | (-1, 1) => t.split += 1,
                        _ => t.undecided += 1,
                    }
                    // (The time by who played: the lookahead's, the network's alone.)
                    t.took[0] += first.took[0] + second.took[1];
                    t.took[1] += first.took[1] + second.took[0];
                    t.fight_ticks += (first.fight_ticks + second.fight_ticks) as u64;
                    let done: u32 = all.iter().map(|t| t.won_both + t.lost_both + t.split + t.undecided).sum();
                    if done % 20 == 0 {
                        eprintln!("{done}/{} deals ({:.0?})", jobs.len(), started.elapsed());
                    }
                }
            });
        }
    });
    let tallies = tallies.into_inner().unwrap();
    println!("lookahead {:?} against the network alone, {} level, {} deals a game (each played both ways):", a.search, a.level.name(), a.deals);
    println!("{:<6} {:>9} {:>10} {:>6} {:>10} {:>16} {:>16}", "game", "won both", "lost both", "split", "undecided", "lookahead ms/min", "network ms/min");
    let mut sum = Tally::default();
    for (p, t) in played.iter().zip(&tallies) {
        // (Each played every round: its time over the rounds' fight minutes.)
        let minutes = t.fight_ticks as f64 / 3600.0;
        println!(
            "{:<6} {:>9} {:>10} {:>6} {:>10} {:>16.1} {:>16.1}",
            p.game.content.game(),
            t.won_both,
            t.lost_both,
            t.split,
            t.undecided,
            t.took[0].as_secs_f64() * 1e3 / minutes.max(1e-9),
            t.took[1].as_secs_f64() * 1e3 / minutes.max(1e-9)
        );
        sum.won_both += t.won_both;
        sum.lost_both += t.lost_both;
        sum.split += t.split;
        sum.undecided += t.undecided;
    }
    println!("{:<6} {:>9} {:>10} {:>6} {:>10}", "all", sum.won_both, sum.lost_both, sum.split, sum.undecided);
}
