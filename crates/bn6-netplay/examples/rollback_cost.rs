//! What rollback costs on a golden-trace round, measured as the
//! verification workspace's `soundmod_rollback_cost` test does: over the
//! 2000 frames around the round's busiest one, the worst case of a
//! 10-frame rollback every rendered frame (restore a snapshot, simulate 10
//! frames again saving each, then the new frame, then digest), against the
//! 16.7 ms a frame has at 60 fps.
//!
//! cargo run --release -p bn6-netplay --example rollback_cost --features trace -- <trace.jsonl> [round]
//!
//! Add `luau` or `rust-content` to measure the GunDelSol slice as content.

use std::time::{Duration, Instant};

use bn6_battle::{Battle, trace};
use bn6_netplay::Game;
use bn6_netplay::bn6::Bn6Input;

const DEPTH: usize = 10;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: rollback_cost <trace.jsonl> [round]");
    let n: usize = args.get(2).map_or(1, |s| s.parse().expect("a round number"));
    std::panic::set_hook(Box::new(|_| {}));
    let rounds = trace::rounds(path).expect("a readable trace");
    let round = &rounds[n - 1];
    let frames: Vec<&trace::Frame> = round.battle_frames().collect();
    let (limit, _) = trace::run_round(round);
    let inputs: Vec<[Bn6Input; 2]> = (0..limit)
        .map(|i| {
            let (players, events) = round.tick_inputs(i, &frames);
            [Bn6Input { tick: players[0], events }, Bn6Input { tick: players[1], events: Default::default() }]
        })
        .collect();
    let mut g = Battle::new(round.round_setup());
    let bs = trace::unhex(&round.setup.battle_state);
    g.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
    g.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
    let runtime = g.behaviors.runtime().to_string();
    let mut states = vec![g.clone()];
    for i in &inputs {
        g.advance(i);
        states.push(g.clone());
    }
    let busiest = (DEPTH..limit - 1).max_by_key(|&f| states[f].objects.in_order().count()).unwrap();
    let window = busiest.saturating_sub(1000).max(DEPTH)..(busiest + 1000).min(limit - 1);
    let (mut restore, mut advance, mut save, mut digest) =
        (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut all = Vec::new();
    let mut snapshots = vec![states[0].clone(); DEPTH + 1];
    for f in window.clone() {
        let mut game = states[f + 1].clone();
        let start = Instant::now();
        let t = Instant::now();
        game.clone_from(&states[f + 1 - DEPTH]);
        restore += t.elapsed();
        for (k, i) in inputs[f + 1 - DEPTH..=f + 1].iter().enumerate() {
            let t = Instant::now();
            game.advance(i);
            advance += t.elapsed();
            let t = Instant::now();
            snapshots[k % (DEPTH + 1)] = game.clone();
            save += t.elapsed();
        }
        let t = Instant::now();
        std::hint::black_box(game.digest());
        digest += t.elapsed();
        all.push(start.elapsed());
        assert_eq!(game.digest(), states[f + 2].digest(), "a re-simulated frame differs");
    }
    let count = all.len() as f64;
    let us = |d: Duration, k: f64| d.as_secs_f64() * 1e6 / k;
    all.sort();
    let total: Duration = all.iter().sum();
    println!(
        "{runtime} content, round {n} frames {window:?} (up to {} objects): restore {:.2} us, advance {:.2} us/frame, \
         save {:.2} us/frame, digest {:.2} us; per rendered frame {:.1} us (99th percentile {:.1} us, worst {:.1} us) of 16667 us",
        states[busiest].objects.in_order().count(),
        us(restore, count),
        us(advance, count * (DEPTH + 1) as f64),
        us(save, count * (DEPTH + 1) as f64),
        us(digest, count),
        us(total, count),
        us(all[all.len() * 99 / 100], 1.0),
        us(*all.last().unwrap(), 1.0),
    );
}
