//! What the learned AI costs in live play (docs/design/training-ai.md §10),
//! on the packs, for each game:
//!
//! - the chips' fingerprints: the whole game's (what live play makes, once a
//!   content) on one thread and on several, against both folders' chips alone
//!   (and their Program Advances); reading them back from the cache file;
//!   the names made of them;
//! - a decision: the view encoded, and the network's pass, at the occupancy
//!   decisions of random play have (the median and the most tokens present)
//!   and with every token present.
//!
//! `NETTAI_PACKS=<packs> cargo run --release -p nettai-training --example ai_costs -- <weights.safetensors> [threads...]`
//!
//! The weights only shape the pass (a network of the trainer's default
//! widths: tests/fixtures/make_net_fixture.py --size default makes one).

use nettai_content_api::ChipHandle;
use nettai_netplay::rng::SplitMix64;
use nettai_training::env::{Config, Env, Game};
use nettai_training::fingerprint::{self, Fingerprints};
use nettai_training::net::{Heads, Net};
use nettai_training::observe::{self, Extra, Names, Obs};
use nettai_training::pool::load_game;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

fn ms(nanos: u128) -> f64 {
    nanos as f64 / 1e6
}

/// Both folders' chips of a round of random builds, and the Program
/// Advances they make.
fn folders(content: &Arc<nettai_battle::Content>, seed: u32) -> Vec<ChipHandle> {
    let maker = nettai_training::builds::Maker::new(content).unwrap();
    let mut picks = nettai_match::Picks::new(seed);
    let b = [0, 1].map(|_| maker.random(&mut picks, &Default::default(), 0.75, nettai_training::builds::Combos::NONE).unwrap());
    let m = maker.round([b[0].side.clone(), b[1].side.clone()], 1000, seed).unwrap();
    let set = nettai_match::Set::of(content, &m, seed);
    let mut chips: BTreeSet<u16> = set.folders().iter().flat_map(|f| f.chips.iter().map(|c| c.id.0)).collect();
    for pa in content.program_advances() {
        let ok = match &pa.recipe {
            nettai_battle::content::Recipe::CodeRun { chip, .. } => chips.contains(&chip.0),
            nettai_battle::content::Recipe::Sequence(v) => v.iter().all(|c| chips.contains(&c.0)),
        };
        if ok {
            chips.insert(pa.result.0);
        }
    }
    chips.into_iter().map(ChipHandle).collect()
}

fn percentile(v: &mut [u128], p: f64) -> u128 {
    v.sort_unstable();
    v[((v.len() - 1) as f64 * p) as usize]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let weights = std::fs::read(args.next().expect("a weights file")).unwrap();
    let threads: Vec<usize> = args.map(|a| a.parse().unwrap()).collect();
    let threads = if threads.is_empty() { vec![1, 4, std::thread::available_parallelism().map_or(4, |n| n.get())] } else { threads };
    let net = Net::from_safetensors(&weights).unwrap();
    let packs = nettai_content::pack::packs_dir();
    nettai_training::env::quiet_engine_panics();
    for game in ["exe6", "exe5", "exe4"] {
        let content = load_game(None, &packs, game).unwrap();
        nettai_battle::behavior::Behaviors::for_content(&content).unwrap();
        let all = fingerprint::game_chips(&content);
        // The chips' fingerprints.
        let mut whole = None;
        for &t in &threads {
            let f = Fingerprints::compute(&content, &all, t).unwrap();
            println!("{game}: the whole game's {} chips on {t} threads: {:.1} s ({:.0} ms of work a chip, {} runs)", all.len(), f.cost.wall_nanos as f64 / 1e9, f.cost.nanos as f64 / all.len() as f64 / 1e6, f.cost.runs);
            whole = Some(f);
        }
        let whole = Arc::new(whole.unwrap());
        for seed in [3, 4] {
            let chips = folders(&content, seed);
            for &t in &threads[..threads.len().min(2)] {
                let f = Fingerprints::compute(&content, &chips, t).unwrap();
                let differ = chips.iter().filter(|&&c| f.of(c) != whole.of(c)).count();
                println!("{game}: both folders' {} chips alone on {t} threads: {:.1} s ({differ} differ from the whole game's)", chips.len(), f.cost.wall_nanos as f64 / 1e9);
            }
        }
        let bytes = serde_json::to_vec(&*whole).unwrap();
        let started = Instant::now();
        let back: Fingerprints = serde_json::from_slice(&bytes).unwrap();
        let read = started.elapsed().as_nanos();
        assert_eq!(back.chips.len(), whole.chips.len());
        let started = Instant::now();
        let names = Names::with_fingerprints(&content, whole.clone());
        let named = started.elapsed().as_nanos();
        println!("{game}: the cache file {} KB, read in {:.1} ms; the names made of them in {:.1} ms", bytes.len() / 1024, ms(read), ms(named));

        // Decisions: random play, both seats, the view and the pass of each of seat 0's.
        let games = Arc::new(vec![Game::with_fingerprints(content.clone(), whole.clone()).unwrap()]);
        let mut env = Env::new(games, Config::default(), 7).unwrap();
        let mut rng = SplitMix64::new(5);
        let mut mem = net.memory();
        let mut obs = Obs::new();
        let (mut fight, mut screen, mut encode, mut probes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let mut actions = [0i32; 2];
        while fight.len() < 3000 {
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
            if out.phase[0] == 0 {
                continue;
            }
            let started = Instant::now();
            observe::encode(env.battle(), 0, &names, &Extra { delay: 16, ..Extra::default() }, &mut obs);
            encode.push(started.elapsed().as_nanos());
            if out.phase[0] == 2 && probes.len() < 200 {
                let started = Instant::now();
                nettai_training::env::probe_keys(env.battle(), 0);
                probes.push(started.elapsed().as_nanos());
            }
            let tokens: usize = env.obs[0].sets.iter().map(|s| s.mask.iter().filter(|&&m| m).count()).sum();
            let heads = if out.phase[0] == 1 { Heads::Fight } else { Heads::Screen };
            let started = Instant::now();
            net.forward(&env.obs[0], &mut mem, heads);
            let t = started.elapsed().as_nanos();
            if out.phase[0] == 1 { fight.push((tokens, t)) } else { screen.push((tokens, t)) }
        }
        let mut every = Obs::new();
        for buf in &mut every.sets {
            buf.mask.fill(true);
            for (i, x) in buf.x.iter_mut().enumerate() {
                *x = ((i * 7919) % 13) as f32 / 13.0;
            }
            for (i, id) in buf.ids.iter_mut().enumerate() {
                *id = 1 + ((i * 104729) % 8190) as i32;
            }
        }
        let mut worst = Vec::new();
        for _ in 0..200 {
            let started = Instant::now();
            net.forward(&every, &mut mem, Heads::Screen);
            worst.push(started.elapsed().as_nanos());
        }
        for (name, v) in [("fight", &mut fight), ("screen", &mut screen)] {
            v.sort_by_key(|(tokens, _)| *tokens);
            let typical = v[v.len() / 2].0;
            let most = v.last().unwrap().0;
            let at = |n: usize| -> Vec<u128> { v.iter().filter(|(t, _)| *t == n).map(|(_, t)| *t).collect() };
            let mut typ = at(typical);
            let mut max = at(most);
            let mut all: Vec<u128> = v.iter().map(|(_, t)| *t).collect();
            println!(
                "{game}: {name} decisions ({}): the pass {:.3} ms (fastest {:.3}) at the median occupancy ({typical} tokens), {:.3} ms (fastest {:.3}) at the most seen ({most} tokens); over all, median {:.3} ms, 99th percentile {:.3} ms",
                v.len(),
                ms(percentile(&mut typ, 0.5)),
                ms(percentile(&mut typ, 0.0)),
                ms(percentile(&mut max, 0.5)),
                ms(percentile(&mut max, 0.0)),
                ms(percentile(&mut all, 0.5)),
                ms(percentile(&mut all, 0.99)),
            );
        }
        let tokens: usize = nettai_training::spec::SETS.iter().map(|s| s.count).sum();
        println!(
            "{game}: the pass with every token present ({tokens}): {:.3} ms (fastest {:.3}); a view encoded: median {:.3} ms, 99th percentile {:.3} ms (one each k = 4 ticks: {:.3} ms a tick)",
            ms(percentile(&mut worst, 0.5)),
            ms(percentile(&mut worst, 0.0)),
            ms(percentile(&mut encode.clone(), 0.5)),
            ms(percentile(&mut encode.clone(), 0.99)),
            ms(percentile(&mut encode, 0.5)) / 4.0,
        );
        println!("{game}: the key probe (once a custom screen): median {:.2} ms, most {:.2} ms", ms(percentile(&mut probes, 0.5)), ms(percentile(&mut probes, 1.0)));
    }
}
