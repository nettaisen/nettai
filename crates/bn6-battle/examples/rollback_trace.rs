//! Rollback netplay over a golden-trace round, GGPO style: one side's
//! input is on time, the other's arrives `k` frames late and is predicted
//! until then (the last one received). Every confirmed frame's state digest
//! is checked against a straight run, which is itself checked against the
//! trace. Then the costs, without digests.
//!
//! cargo run --release -p bn6-battle --example rollback_trace --features trace,luau -- <trace.jsonl> [round]
//!
//! (`--features trace` alone runs the built-in kinds; `trace,rust-content`
//! the Rust content.)

use std::collections::VecDeque;
use std::time::Instant;

use bn6_battle::battle::{Battle, TickEvents};
use bn6_battle::input::PlayerTick;
use bn6_battle::rollback::{self, CueEvent, Session};
use bn6_battle::trace::{self, Round};

/// The round's battle as the trace starts it (as `trace::run_round`).
fn start(round: &Round) -> Battle {
    let mut b = Battle::new(round.round_setup());
    let bs = trace::unhex(&round.setup.battle_state);
    b.round.frames = u32::from_le_bytes(bs[0x60..0x64].try_into().unwrap());
    b.round.ticks = u32::from_le_bytes(bs[0x64..0x68].try_into().unwrap());
    b
}

type Inputs = Vec<([PlayerTick; 2], TickEvents)>;

/// Run a session over the first `played` frames with side `on_time`'s
/// input on time and the other side's `delay` frames late.
fn session(round: &Round, inputs: &Inputs, played: usize, on_time: usize, delay: usize, digests: bool) -> Session {
    let mut s = Session::new(start(round), on_time as u8);
    if digests {
        s.record_digests();
    }
    for f in 0..played {
        let (input, events) = &inputs[f];
        s.advance(input[on_time].clone(), events.clone());
        if f + 1 >= delay {
            s.receive_remote(inputs[f + 1 - delay].0[1 - on_time].clone());
        }
        // A frontend would hand these to its audio layer.
        let _: Vec<CueEvent> = s.take_cue_events();
    }
    for (input, _) in &inputs[played + 1 - delay..played] {
        s.receive_remote(input[1 - on_time].clone());
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: rollback_trace <trace.jsonl> [round]");
    let n: usize = args.get(2).map_or(1, |s| s.parse().expect("a round number"));
    let rounds = trace::rounds(path).expect("a readable trace");
    let round = &rounds[n - 1];
    let frames: Vec<&trace::Frame> = round.battle_frames().collect();
    let inputs: Inputs = (0..frames.len()).map(|i| round.tick_inputs(i, &frames)).collect();

    // Straight, checking the trace and recording each frame's digest. Stop
    // where the engine does (an unported feature panics).
    let mut b = start(round);
    let runtime = b.content.runtime().to_string();
    let mut want = Vec::new();
    let mut matched = None;
    for (i, (input, events)) in inputs.iter().enumerate() {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.tick(input, events.clone())));
        if r.is_err() {
            break;
        }
        if matched.is_none() && !trace::compare(&b, frames[i]).is_empty() {
            matched = Some(i);
        }
        want.push(rollback::digest(&b));
    }
    let played = want.len();
    println!(
        "round {n}, {runtime} content: straight run {}/{} frames match the trace ({played} ran)",
        matched.unwrap_or(played),
        frames.len()
    );

    // Predict the real remote side, then (harder: it is the side firing
    // GunDelSol) the trace's local side.
    let local_side = round.round_setup().local_side as usize;
    for predicted in [1 - local_side, local_side] {
        let who = if predicted == local_side { "local" } else { "remote" };
        println!("  predicting side {predicted} (the trace's {who} player):");
        for delay in [2usize, 5, 10] {
            let s = session(round, &inputs, played, 1 - predicted, delay, true);
            let have = s.confirmed_digests();
            let first_diff = have.iter().zip(&want).position(|(a, b)| a != b);
            let verdict = match (first_diff, have.len() == want.len()) {
                (None, true) => format!("all {} confirmed frames equal the straight run", have.len()),
                (Some(f), _) => format!("DIVERGES at frame {f}"),
                (None, false) => format!("only {} of {} frames confirmed", have.len(), want.len()),
            };
            println!(
                "    {delay:>2} frames late: {verdict}; {} rollbacks, {} frames re-simulated",
                s.stats.rollbacks, s.stats.resimulated_frames
            );
        }
    }

    // Costs, without digests.
    let t = Instant::now();
    let mut b = start(round);
    for (input, events) in &inputs[..played] {
        b.tick(input, events.clone());
    }
    let straight = t.elapsed().as_secs_f64() * 1e6 / played as f64;
    let t = Instant::now();
    let s = session(round, &inputs, played, 1 - local_side, 10, false);
    let late = t.elapsed().as_secs_f64() * 1e6 / played as f64;
    // Worst case: every frame restores the snapshot from 10 frames back and
    // re-simulates, as when every prediction fails.
    let mut b = start(round);
    let mut ring: VecDeque<Battle> = VecDeque::new();
    let t = Instant::now();
    for f in 0..played {
        ring.push_back(b.clone());
        if ring.len() > 10 {
            ring.pop_front();
        }
        if ring.len() == 10 {
            b.clone_from(&ring[0]);
            for (input, events) in &inputs[f + 1 - 10..f] {
                b.tick(input, events.clone());
            }
        }
        let (input, events) = &inputs[f];
        b.tick(input, events.clone());
    }
    let worst = t.elapsed().as_secs_f64() * 1e6 / played as f64;
    println!(
        "  per frame: straight {straight:.1} µs; session, local side predicted 10 late ({} rollbacks) {late:.1} µs; \
         every frame rolling back 10: {worst:.1} µs (of a 16,667 µs frame)",
        s.stats.rollbacks
    );
}
