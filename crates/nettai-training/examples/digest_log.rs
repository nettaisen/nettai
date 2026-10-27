//! A log of battle digests two builds of the engine compare: for each game,
//! rounds drawn from seeds 0.., both seats taking random allowed actions
//! (seeded too), and after every decision the battle's digest with the
//! content's hash left out of its setup (a build whose content hashes
//! otherwise, the same battles: the same log). The digest holds asset
//! handles, so two builds compare where they number the assets alike.
//!
//! `cargo run --release -p nettai-training --example digest_log -- OUT [ROUNDS]`
//! (`NETTAI_CONTENT`, the content directory).
//!
//! (Written for the move of the sprites' timing into content: its build
//! before, on the content before and packs whose index listed just the
//! names the content after gives, logged the same 90,146 digests of 48
//! rounds as the build after.)

use nettai_netplay::rng::SplitMix64;
use nettai_training::env::{Config, Env, FIGHT, Game, SCREEN};
use std::io::Write;
use std::sync::Arc;

fn pick(rng: &mut SplitMix64, legal: &[bool]) -> i32 {
    let n = legal.iter().filter(|&&l| l).count() as u64;
    let mut k = rng.below(n);
    legal
        .iter()
        .position(|&l| {
            if l {
                if k == 0 {
                    return true;
                }
                k -= 1;
            }
            false
        })
        .expect("an allowed action") as i32
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("usage: digest_log OUT [ROUNDS]");
    let rounds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(8);
    let mut log = std::io::BufWriter::new(std::fs::File::create(&out).unwrap_or_else(|e| panic!("{out}: {e}")));
    for game in ["exe6", "exe5", "exe4"] {
        let content = nettai_training::pool::load_game(None, game).unwrap_or_else(|e| panic!("{e}"));
        let games = Arc::new(vec![Game::new(content).unwrap_or_else(|e| panic!("{e}"))]);
        let mut decisions = 0u64;
        for seed in 0..rounds {
            let mut env = Env::new(games.clone(), Config::default(), seed).unwrap_or_else(|e| panic!("{game} {seed}: {e}"));
            let mut rng = SplitMix64::new(seed ^ 0x5eed);
            let mut actions = [0i32; 2];
            for step in 0..40_000u32 {
                let o = env.step(actions).unwrap_or_else(|e| panic!("{game} {seed} step {step}: {e}"));
                let mut b = env.battle().clone();
                b.setup.content = nettai_battle::content::ContentHash(0);
                writeln!(log, "{game} {seed} {step} {:016x}", b.digest()).unwrap();
                decisions += 1;
                if o.done {
                    break;
                }
                for s in 0..2 {
                    actions[s] = match o.phase[s] {
                        FIGHT => pick(&mut rng, &env.legal_fight[s]),
                        SCREEN => pick(&mut rng, &env.legal_screen[s]),
                        _ => 0,
                    };
                }
            }
        }
        eprintln!("{game}: {rounds} rounds, {decisions} digests");
    }
}
