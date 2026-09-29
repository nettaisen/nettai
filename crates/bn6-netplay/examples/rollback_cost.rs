//! What rollback costs per rendered frame on a synthetic netbattle: the
//! worst case of a 10-frame rollback every frame (restore a snapshot,
//! simulate 10 frames again saving each, then the new frame), against the
//! 16.7 ms a frame has at 60 fps.
//!
//!     cargo run --release -p bn6-netplay --example rollback_cost

use bn6_battle::Battle;
use bn6_netplay::Game;
use bn6_netplay::standin::{Masher, Rules, StandInBattle, hand, netbattle};
use std::time::{Duration, Instant};

const DEPTH: usize = 10;

fn main() {
    let rules = Rules {
        hands: [
            hand(&[(0x11, 13), (0xEC, 4), (0x0F, 2), (0xB1, 0), (0x11, 16)]),
            hand(&[(0x11, 13), (0x0F, 2), (0x10, 1), (0x11, 16), (0x0F, 12)]),
        ],
        min_ticks: 20,
        max_ticks: 90,
        beast_out: false,
    };
    let seed = 3u64;
    let mut g = StandInBattle::new(Battle::new(netbattle(500, seed as u32 ^ 0x1234_5678)), rules);
    let mut m = [Masher::new(seed), Masher::new(seed ^ 0xABCD)];
    let mut inputs = Vec::new();
    let mut states = Vec::new();
    while !g.is_over() {
        let i = [m[0].buttons(), m[1].buttons()];
        states.push(g.clone());
        g.advance(&i);
        inputs.push(i);
    }
    println!("battle: {} frames", inputs.len());
    println!("snapshot: {} bytes inline + {} bytes of object pools", std::mem::size_of::<StandInBattle>(), heap_bytes(&g.battle));

    let (mut save, mut restore, mut advance, mut digest, mut total) = (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut worst = Duration::ZERO;
    let mut snapshots: Vec<StandInBattle> = vec![states[0].clone(); DEPTH + 1];
    let mut frames = 0u32;
    // Every rendered frame from 100 on: roll back DEPTH frames.
    let mut all = Vec::new();
    for f in (100..inputs.len() - 1).step_by(3) {
        let mut game = states[f + 1].clone();
        let back = &states[f + 1 - DEPTH];
        let start = Instant::now();
        let t = Instant::now();
        game.clone_from(back);
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
        let e = start.elapsed();
        all.push(e);
        total += e;
        worst = worst.max(e);
        frames += 1;
    }
    let per = |d: Duration, n: u32| d.as_secs_f64() * 1e6 / n as f64;
    let n = frames;
    let steps = n * (DEPTH as u32 + 1);
    println!("{n} rendered frames, each a {DEPTH}-frame rollback:");
    println!("  restore  {:7.2} us", per(restore, n));
    println!("  advance  {:7.2} us per frame ({} per rendered frame)", per(advance, steps), DEPTH + 1);
    println!("  save     {:7.2} us per frame ({} per rendered frame)", per(save, steps), DEPTH + 1);
    println!("  digest   {:7.2} us", per(digest, n));
    all.sort();
    let p99 = all[all.len() * 99 / 100];
    println!(
        "  total    {:7.2} us per rendered frame (99th percentile {:.2} us, worst {:.2} us) of a 16667 us budget",
        per(total, n),
        p99.as_secs_f64() * 1e6,
        worst.as_secs_f64() * 1e6
    );
}

/// The object pools' heap part (the rest of a battle is inline).
fn heap_bytes(b: &Battle) -> usize {
    let slots = bn6_battle::object::SLOTS * 3;
    slots * (std::mem::size_of::<bn6_battle::object::Object>() + std::mem::size_of::<bn6_battle::object::sprite::Sprite>())
        + b.field.home_runs.len() * 8
}
