//! What rollback costs on a golden-trace round, measured as the
//! verification workspace's `soundmod_rollback_cost` test does: over the
//! 2000 frames around the round's busiest one, the worst case of a
//! 10-frame rollback every rendered frame, through the battle's getgud
//! `World` the way a session drives it (load the settled state, step the
//! corrected tick and the 10 speculated after it, saving each, then digest
//! the settled state), against the 16.7 ms a frame has at 60 fps.
//!
//! cargo run --release -p bn6-netplay --example rollback_cost -- <trace.jsonl> <pack> [round]
//!
//! (`<pack>`: the BN6 content pack the trace's battle runs on, from
//! `bn6-extract content`.)
//!
//! The pack's scripts (Luau) run the content they implement.

use std::time::{Duration, Instant};

use bn6_compat::trace;
use bn6_netplay::bn6::Bn6Input;
use bn6_netplay::getgud::World;
use bn6_netplay::{BattleState, BattleWorld};

const DEPTH: usize = 10;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let usage = "usage: rollback_cost <trace.jsonl> <pack> [round]";
    let path = args.get(1).expect(usage);
    let pack = args.get(2).expect(usage);
    let (content, _) = bn6_content::pack::load_battle(std::path::Path::new(pack)).unwrap_or_else(|r| panic!("{pack}: {r}"));
    let content = std::sync::Arc::new(content);
    let n: usize = args.get(3).map_or(1, |s| s.parse().expect("a round number"));
    std::panic::set_hook(Box::new(|_| {}));
    let rounds = trace::rounds(path).expect("a readable trace");
    let round = &rounds[n - 1];
    let frames: Vec<&trace::Frame> = round.battle_frames().collect();
    let compat = bn6_compat::Compat::bn6();
    let (limit, _) = trace::run_round(round, &content, compat);
    let ids = bn6_compat::codec::Ids::new(&content, compat);
    let inputs: Vec<[Bn6Input; 2]> = (0..limit)
        .map(|i| {
            let (players, events) = round.tick_inputs(i, &frames, &ids);
            [Bn6Input { tick: players[0], events }, Bn6Input { tick: players[1], events: Default::default() }]
        })
        .collect();
    // Side 0's world: its input is `local`, side 1's the remote.
    let mut world = BattleWorld::new(round.start(content.clone(), compat), 0);
    let runtime = bn6_battle::behavior::Behaviors::for_content(&world.game().content).unwrap().runtime().to_string();
    let step = |w: &mut BattleWorld<_>, [a, b]: &[Bn6Input; 2]| {
        let Ok(()) = w.step(a, std::slice::from_ref(b));
    };
    let save = |w: &mut BattleWorld<_>| -> BattleState {
        let Ok(s) = w.save();
        s
    };
    let mut states = vec![save(&mut world)];
    for i in &inputs {
        step(&mut world, i);
        states.push(save(&mut world));
    }
    let objects = |s: &BattleState| s.battle().objects.in_order().count();
    let busiest = (DEPTH..limit - 1).max_by_key(|&f| objects(&states[f])).unwrap();
    let window = busiest.saturating_sub(1000).max(DEPTH)..(busiest + 1000).min(limit - 1);
    let (mut restore, mut advance, mut save_time, mut digest) =
        (Duration::ZERO, Duration::ZERO, Duration::ZERO, Duration::ZERO);
    let mut all = Vec::new();
    let mut speculated: Vec<BattleState> = Vec::with_capacity(DEPTH + 1);
    for f in window.clone() {
        let start = Instant::now();
        let t = Instant::now();
        let Ok(()) = world.load(&states[f + 1 - DEPTH]);
        restore += t.elapsed();
        speculated.clear();
        for i in &inputs[f + 1 - DEPTH..=f + 1] {
            let t = Instant::now();
            step(&mut world, i);
            advance += t.elapsed();
            let t = Instant::now();
            speculated.push(save(&mut world));
            save_time += t.elapsed();
        }
        // The first of them is the new settled state.
        let t = Instant::now();
        std::hint::black_box(speculated[0].battle().digest());
        digest += t.elapsed();
        all.push(start.elapsed());
        assert_eq!(world.game().digest(), states[f + 2].battle().digest(), "a re-simulated frame differs");
    }
    let count = all.len() as f64;
    let us = |d: Duration, k: f64| d.as_secs_f64() * 1e6 / k;
    all.sort();
    let total: Duration = all.iter().sum();
    println!(
        "{runtime} content, round {n} frames {window:?} (up to {} objects): restore {:.2} us, advance {:.2} us/frame, \
         save {:.2} us/frame, digest {:.2} us; per rendered frame {:.1} us (99th percentile {:.1} us, worst {:.1} us) of 16667 us",
        objects(&states[busiest]),
        us(restore, count),
        us(advance, count * (DEPTH + 1) as f64),
        us(save_time, count * (DEPTH + 1) as f64),
        us(digest, count),
        us(total, count),
        us(all[all.len() * 99 / 100], 1.0),
        us(*all.last().unwrap(), 1.0),
    );
}
