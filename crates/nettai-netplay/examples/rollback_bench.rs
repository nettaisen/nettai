//! What rollback costs the engine on a recorded match, measured with
//! `nettai_netplay::cost` (which says how): in the fight and in the custom
//! screens, a step, a save (time and bytes), a restore and rollbacks of
//! several depths; then whole frames of a session whose remote input
//! arrives over three simulated links; then what the session's states
//! hold, and what a saved state is made of, part by part.
//!
//! cargo run --release -p nettai-netplay --example rollback_bench -- <trace.jsonl> <pack> [options]
//!
//! (`<trace.jsonl>`: a golden trace of an EXE6 match, whose rounds give the
//! players' inputs; `<pack>`: the EXE6 content pack whose assets the
//! trace's battle names, from `exe6-extract content`. The battle content is
//! this repository's content/, or `$NETTAI_CONTENT`.)
//!
//! - `--rounds 1,3`: those rounds of the trace (all of them);
//! - `--frames N`: the first N frames of each round (all of them);
//! - `--points N`: ticks measured in each phase of each round (20);
//! - `--reps N`: rollbacks of each depth at each of them (5);
//! - `--depths 1,2,4`: the depths (1,2,4,8,12,16);
//! - `--delay N`: the sessions' present delay (2, a frontend's);
//! - `--max-lead N`: their stall guard (30, a frontend's);
//! - `--runs N`: sessions over each link (3).
//!
//! The world is a peer's: the battle on the engine's input record, with
//! the sound cue feed a frontend gives it (`CueFeed`), for the side of the
//! console the trace was recorded on. Nothing is drawn and no sound is
//! made: a frontend draws a frame from the state it is handed and plays
//! the cue feed's actions, neither of which is the simulation's.
//!
//! The verification workspace's tools/rollback-bench measures Tango's
//! emulated pair beside the engine with the same loops.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use exe6_compat::trace;
use nettai_battle::Battle;
use nettai_netplay::BattleWorld;
use nettai_netplay::battle::{CueFeed, PlayerInput};
use nettai_netplay::cost::{self, Advanced, Costs, Counting, Drive, Phase, Point, Recorded, RecordedSession};

#[global_allocator]
static HEAP: Counting = Counting;

/// Frames a cue a re-simulation makes again may move and still be the one
/// played, as a frontend's sound has it.
const CUE_TOLERANCE: u32 = 3;

struct Options {
    rounds: Option<Vec<usize>>,
    frames: usize,
    points: usize,
    reps: usize,
    depths: Vec<usize>,
    delay: u32,
    max_lead: usize,
    runs: usize,
}

/// A round of the trace: where it starts, the players' inputs, and the
/// side of the console that recorded it.
struct Round {
    number: usize,
    start: Battle,
    inputs: Vec<[PlayerInput; 2]>,
    side: usize,
    phases: Vec<Phase>,
    /// The state's digest at each tick, `0..=inputs.len()`, from a plain
    /// run.
    digests: Vec<u64>,
}

impl Round {
    fn world(&self) -> BattleWorld<Battle, CueFeed> {
        BattleWorld::with_observer(self.start.clone(), self.side, CueFeed::new(self.side as u8, CUE_TOLERANCE))
    }
}

fn main() {
    let usage = "usage: rollback_bench <trace.jsonl> <pack> [--rounds A,B] [--frames N] [--points N] [--reps N] [--depths A,B] [--delay N] [--max-lead N] [--runs N]";
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut o = Options { rounds: None, frames: usize::MAX, points: 20, reps: 5, depths: vec![1, 2, 4, 8, 12, 16], delay: 2, max_lead: 30, runs: 3 };
    let mut take = |name: &str| -> Option<String> {
        let i = args.iter().position(|a| a == name)?;
        let value = args.get(i + 1).cloned().expect(usage);
        args.drain(i..=i + 1);
        Some(value)
    };
    let list = |text: String| -> Vec<usize> { text.split(',').map(|n| n.parse().expect(usage)).collect() };
    o.rounds = take("--rounds").map(list);
    if let Some(n) = take("--frames") {
        o.frames = n.parse().expect(usage);
    }
    if let Some(n) = take("--points") {
        o.points = n.parse().expect(usage);
    }
    if let Some(n) = take("--reps") {
        o.reps = n.parse().expect(usage);
    }
    if let Some(n) = take("--depths") {
        o.depths = list(n);
    }
    if let Some(n) = take("--delay") {
        o.delay = n.parse().expect(usage);
    }
    if let Some(n) = take("--max-lead") {
        o.max_lead = n.parse().expect(usage);
    }
    if let Some(n) = take("--runs") {
        o.runs = n.parse().expect(usage);
    }
    let [path, pack] = &args[..] else { panic!("{usage}") };
    let deepest = *o.depths.iter().max().expect(usage);

    let (content, _) =
        nettai_content::pack::load_battle(&nettai_content::index::content(), std::path::Path::new(pack)).unwrap_or_else(|r| panic!("{pack}: {r}"));
    let content = Arc::new(content);
    let compat = exe6_compat::Compat::exe6();
    let quiet = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let recorded = trace::rounds(path).expect("a readable trace");
    let rounds: Vec<Round> = recorded
        .iter()
        .enumerate()
        .filter(|(n, _)| o.rounds.as_ref().is_none_or(|r| r.contains(&(n + 1))))
        .map(|(n, round)| {
            let frames: Vec<&trace::Frame> = round.battle_frames().collect();
            // (As far as the engine follows the recording: all of it, on a
            // recording the trace tests pass.)
            let (limit, _) = trace::run_round(round, &content, compat);
            let inputs: Vec<[PlayerInput; 2]> = (0..limit.min(o.frames))
                .map(|i| {
                    let (players, events) = round.tick_inputs(i, &frames);
                    [PlayerInput { tick: players[0], events }, PlayerInput { tick: players[1], events: Default::default() }]
                })
                .collect();
            let start = round.start(content.clone(), compat);
            let phases = cost::phases(&start, &inputs);
            let mut plain = start.clone();
            let mut digests = vec![plain.digest()];
            for [a, b] in &inputs {
                nettai_netplay::step_game(&mut plain, [a, b]);
                digests.push(plain.digest());
            }
            Round { number: n + 1, side: start.round.local_side as usize & 1, start, inputs, phases, digests }
        })
        .collect();
    std::panic::set_hook(quiet);
    let runtime = nettai_battle::behavior::Behaviors::for_content(&content).unwrap().runtime().to_string();
    println!("# What rollback costs the engine\n");
    println!("{path} ({runtime} content), the side of the console that recorded it:\n");
    for r in &rounds {
        let count = |phase| r.phases.iter().filter(|&&p| p == phase).count();
        println!("- round {}: side {}, {} frames ({} in the custom screens, {} in the fight)", r.number, r.side, r.inputs.len(), count(Phase::Custom), count(Phase::Fight));
    }
    println!("\nLoad average before: {}.", load());

    // ---- The calls ----------------------------------------------------------
    let started = Instant::now();
    let mut costs: BTreeMap<Phase, Costs> = BTreeMap::new();
    let mut measured: BTreeMap<Phase, usize> = BTreeMap::new();
    for r in &rounds {
        let mut at: BTreeMap<usize, Phase> = BTreeMap::new();
        for phase in [Phase::Custom, Phase::Fight] {
            at.extend(cost::spread(&cost::within(&r.phases, phase, deepest), o.points).into_iter().map(|tick| (tick, phase)));
        }
        let mut subject = Recorded { world: r.world(), inputs: r.inputs.clone() };
        for tick in 0..r.inputs.len() {
            if let Some(&phase) = at.get(&tick) {
                let costs = costs.entry(phase).or_default();
                let mut point = Point::open(&mut subject, tick, deepest, costs);
                for _ in 0..o.reps {
                    point.run(&mut subject, &o.depths, costs);
                }
                point.close(&mut subject);
                *measured.entry(phase).or_default() += 1;
            }
            subject.settle(tick);
        }
    }
    println!("\n## A world's calls\n");
    println!(
        "At ticks spread over each phase of each round, the calls a session makes of its world, on the match's own inputs: \
         {} rollbacks of each depth at each tick, every depth re-simulating the ticks that end {deepest} after it ({:.1} s).",
        o.reps,
        started.elapsed().as_secs_f64()
    );
    for (phase, costs) in &costs {
        println!("\n### In the {} ({} ticks)\n", phase.name(), measured[phase]);
        print!("{}", cost::costs_table(&[("nettai", costs)]));
    }

    // ---- Sessions -----------------------------------------------------------
    let started = Instant::now();
    // By link and phase, each run's frames.
    let mut frames: Vec<BTreeMap<Phase, Vec<Vec<Advanced>>>> = vec![BTreeMap::new(); cost::LINKS.len()];
    for run in 0..o.runs {
        for (link, (_, network)) in cost::LINKS.iter().enumerate() {
            for phase in [Phase::Custom, Phase::Fight] {
                frames[link].entry(phase).or_default().push(Vec::new());
            }
            for r in &rounds {
                let ticks = r.inputs.len();
                let arrivals = cost::arrivals(*network, seed(link, run, r.number), ticks);
                let mut peer = RecordedSession { session: r.world().session(o.delay), inputs: r.inputs.clone() };
                let mut drive = Drive::new(ticks, o.delay as usize);
                let mut wall = 0;
                while drive.frame(&mut peer, arrivals.get(wall).copied().unwrap_or(ticks), o.max_lead) {
                    wall += 1;
                }
                assert_eq!(drive.parked, 0, "the stall guard held a peer: the link is worse than --max-lead {} allows", o.max_lead);
                // The session's settled state is the plain run's.
                let settled = peer.session.settled_state();
                assert_eq!(settled.battle().digest(), r.digests[settled.tick() as usize], "round {}'s session settled another state", r.number);
                for f in &drive.frames {
                    // The newest tick the frame simulated, and the phase it
                    // left the round in.
                    let Some(newest) = (f.tick as usize).checked_sub(o.delay as usize + 1) else { continue };
                    if let Some(of) = frames[link].get_mut(&r.phases[newest + 1]) {
                        of[run].push(*f);
                    }
                }
            }
        }
    }
    println!("\n## A session's frames\n");
    println!(
        "A getgud session on the same world with a present delay of {}, the other player's inputs arriving as a simulated link \
         delivers them (both players' clocks agree; a datagram a frame over rennet's streams), {} sessions over each link, \
         their turns interleaved ({:.1} s). A frame is taking the inputs that arrived and the advance. Microseconds.\n",
        o.delay,
        o.runs,
        started.elapsed().as_secs_f64()
    );
    let mut rows = Vec::new();
    for (link, (name, network)) in cost::LINKS.iter().enumerate() {
        for (phase, runs) in &frames[link] {
            rows.push((format!("{name} ({}), {}", cost::describe(network), phase.name()), runs.iter().map(|r| &r[..]).collect()));
        }
    }
    print!("{}", cost::frames_table(&rows));

    // ---- Memory -------------------------------------------------------------
    println!("\n## What a session's states hold\n");
    println!("A session keeps the settled state and one state a speculated tick. Bytes are the heap's, with the state's own.\n");
    println!("| | a state | the most a session held | at the stall guard ({} states) |\n|---|---|---|---|", o.max_lead + 1);
    for (link, (name, _)) in cost::LINKS.iter().enumerate() {
        for (phase, runs) in &frames[link] {
            let Some(state) = costs.get(phase).map(|c| c.state_bytes.summary().max) else { continue };
            let held = 1 + runs.iter().flatten().map(|f| f.speculation).max().unwrap_or(0) as u64;
            println!("| {name}, {} | {state} | {held} states, {} | {} |", phase.name(), state * held, state * (o.max_lead as u64 + 1));
        }
    }

    // ---- What a state is made of --------------------------------------------
    // The busiest state of the first round (the most objects).
    let r = &rounds[0];
    let mut subject = Recorded { world: r.world(), inputs: r.inputs.clone() };
    let mut busiest = (0, r.start.clone());
    for tick in 0..r.inputs.len() {
        subject.settle(tick);
        let b = subject.world.game();
        if b.objects.in_order().count() > busiest.1.objects.in_order().count() {
            busiest = (tick + 1, b.clone());
        }
    }
    let (tick, b) = &busiest;
    println!("\n## What a saved state is made of\n");
    println!(
        "Round {}'s state with the most objects ({}, after {tick} ticks), part by part: what a copy of the part holds \
         (heap and its own bytes), the allocations the copy makes, and the time of a copy (the median of 2001, with the \
         copy's freeing).\n",
        r.number,
        b.objects.in_order().count()
    );
    println!("| part | bytes | allocations | a copy, microseconds |\n|---|---|---|---|");
    macro_rules! part {
        ($name:expr, $value:expr) => {{
            let (bytes, allocations, nanos) = copy_of($value);
            println!("| {} | {bytes} | {allocations} | {} |", $name, cost::micros(nanos));
        }};
    }
    part!("the whole battle (a snapshot's `Box<Battle>`)", &Box::new(b.clone()));
    part!("`setup` (the round's setup: both players' folders and stats)", &b.setup);
    part!("`stats` and `reserves`", &(b.stats, b.reserves));
    part!("`objects`", &b.objects);
    part!("`actors`", &b.actors);
    part!("`collision`", &b.collision);
    part!("`field`", &b.field);
    part!("`custom` (the custom screens)", &b.custom);
    part!("`hands`", &b.hands);
    part!("`inputs`", &b.inputs);
    part!("`link`", &b.link);
    part!("`sides`", &b.sides);
    part!("`rules`", &b.rules);
    part!("`auto_battle`", &b.auto_battle);
    {
        // Saving into a state a session is done with (`save_state_into`),
        // which a world's `recycle` would allow.
        let mut into = b.save_state();
        let mut samples = cost::Samples::default();
        for _ in 0..2001 {
            let t = Instant::now();
            b.save_state_into(&mut into);
            samples.push(t.elapsed().as_nanos() as u64);
        }
        let before = cost::heap();
        b.save_state_into(&mut into);
        let after = cost::heap();
        println!(
            "| the whole battle, saved into a state already made (`save_state_into`) | - | {} | {} |",
            after.allocations - before.allocations,
            cost::micros(samples.summary().median)
        );
    }
    {
        // The state's digest: not on a session's path, but what a check of
        // the peers' settled states against each other would add a frame.
        let mut samples = cost::Samples::default();
        for _ in 0..2001 {
            let t = Instant::now();
            std::hint::black_box(b.digest());
            samples.push(t.elapsed().as_nanos() as u64);
        }
        println!("| the whole battle's digest (`Battle::digest`: no copy) | - | - | {} |", cost::micros(samples.summary().median));
    }
    println!("\nLoad average after: {}.", load());
}

/// What a copy of `value` holds (the heap's bytes and its own), the
/// allocations it makes, and the median time of a copy and its freeing.
fn copy_of<T: Clone>(value: &T) -> (u64, u64, u64) {
    let before = cost::heap();
    let copy = value.clone();
    let after = cost::heap();
    drop(copy);
    let mut samples = cost::Samples::default();
    for _ in 0..2001 {
        let t = Instant::now();
        drop(std::hint::black_box(value.clone()));
        samples.push(t.elapsed().as_nanos() as u64);
    }
    (after.live - before.live + size_of::<T>() as u64, after.allocations - before.allocations, samples.summary().median)
}

fn seed(link: usize, run: usize, round: usize) -> u64 {
    0x5EED ^ (link as u64) << 8 ^ (run as u64) << 16 ^ (round as u64) << 24
}

fn load() -> String {
    cost::load_average().map_or("unknown".to_string(), |[a, b, c]| format!("{a:.2} {b:.2} {c:.2} (1, 5 and 15 minutes)"))
}
