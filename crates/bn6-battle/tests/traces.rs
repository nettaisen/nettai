//! Replays golden traces (data/traces/*.jsonl, regenerated with tools/difftest)
//! through the engine and compares observable state every tick. Traces are
//! large and not checked in; tests skip when they're absent.

use bn6_battle::trace;
use std::path::PathBuf;

fn trace_path(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/traces").join(name);
    p.exists().then_some(p)
}

/// Replay each round; report how far the engine gets.
fn replay(name: &str) -> Vec<(usize, usize)> {
    let Some(path) = trace_path(name) else { return Vec::new() };
    let rounds = trace::rounds(path).unwrap();
    let mut progress = Vec::new();
    for (n, round) in rounds.iter().enumerate() {
        let total = round.battle_frames().count();
        let (ok, diff) = trace::run_round(round);
        eprintln!("{name} round {}: {ok}/{total} frames match", n + 1);
        if let Some((frame, diffs)) = diff {
            eprintln!("  first difference at frame {frame}:");
            for d in diffs {
                eprintln!("    {d}");
            }
        }
        progress.push((ok, total));
    }
    progress
}

/// Each round must match at least `floors[n]` frames. Raised as the engine
/// grows; the goal is every frame of every round.
fn check(progress: &[(usize, usize)], floors: &[usize]) {
    for (n, (&(ok, total), &floor)) in progress.iter().zip(floors).enumerate() {
        assert!(ok <= total);
        assert!(ok >= floor, "round {} regressed: {ok} frames match, {floor} did before", n + 1);
    }
}

#[test]
fn machgun() {
    check(&replay("machgun.jsonl"), &[1074, 552]);
}
