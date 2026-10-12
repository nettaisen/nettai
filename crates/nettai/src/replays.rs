//! The replays in the replays folder, each read (the main menu's reel plays the latest; the Replays screen
//! lists them, and plays each out on a thread with the library's `play_out` for who won).

// (The Replays screen isn't built yet: what it will call isn't called.)
#![allow(dead_code)]

use nettai_frontend::replay::{Outcome, Replay};
use std::path::{Path, PathBuf};

/// A replay file, read, and what playing it out made.
pub struct Entry {
    pub path: PathBuf,
    pub replay: Result<Replay, String>,
    /// None while it is being played out (or its game isn't loaded).
    pub outcome: Option<Result<Outcome, String>>,
}

/// The replays in `dir`, newest first.
pub fn scan(dir: &Path) -> Vec<Entry> {
    let Ok(read) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<Entry> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "ntrp"))
        .map(|path| {
            let replay = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| Replay::read(&bytes));
            Entry { path, replay, outcome: None }
        })
        .collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.replay.as_ref().map_or(0, |r| r.info.when)));
    out
}

/// The tick (counted through the set) each round of `replay` starts on.
pub fn round_starts(replay: &Replay) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, t) in replay.ticks.iter().enumerate() {
        if t.round_ended && i + 1 < replay.ticks.len() {
            starts.push(i + 1);
        }
    }
    starts
}
