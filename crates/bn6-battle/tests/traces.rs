//! Replays golden traces (data/traces/*.jsonl, regenerated with tools/difftest)
//! through the engine. Traces are large and not checked in; tests skip when
//! they're absent.

use bn6_battle::trace::{self, Line};
use std::path::PathBuf;

fn trace_path(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/traces").join(name);
    p.exists().then_some(p)
}

#[test]
fn trace_rng2_matches_step_function() {
    // Sanity check of the trace format and the RNG: between consecutive
    // frames, RNG2 advances by a whole number of steps.
    for name in ["machgun.jsonl", "soundmod.jsonl"] {
        let Some(path) = trace_path(name) else { continue };
        let (steps, frames_with_steps) = rng2_steps(path);
        eprintln!("{name}: max RNG2 steps per frame {steps}, {frames_with_steps} frames advance it");
    }
}

fn rng2_steps(path: PathBuf) -> (u32, u32) {
    let mut advancing = 0;
    let mut prev: Option<u32> = None;
    let mut max_steps = 0;
    for line in trace::read(path).unwrap() {
        let f = match line {
            // Each round reseeds RNG2 from the link master's seed.
            Line::Setup(_) => {
                prev = None;
                continue;
            }
            Line::Frame(f) => f,
        };
        if f.state[0] != 4 {
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            let mut r = bn6_battle::Rng::new(p);
            let mut n = 0;
            while r.state != f.rng2 {
                r.next();
                n += 1;
                assert!(n < 10_000, "rng2 at frame {} isn't reachable from the previous frame", f.frame);
            }
            max_steps = max_steps.max(n);
            advancing += (n > 0) as u32;
        }
        prev = Some(f.rng2);
    }
    (max_steps, advancing)
}
