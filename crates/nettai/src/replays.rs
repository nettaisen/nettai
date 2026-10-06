//! The replays in the replays folder: each read, then played out to its end
//! on a thread (the library's `play_out`: who won each round and the set,
//! and whether it plays back as it was played), and shown as rows.

use crate::games::{Games, Names};
use crate::{ReplayCheck, ReplayRow};
use chrono::{Datelike, Timelike};
use nettai_battle::BattleResult;
use nettai_frontend::player::FRAME_RATE;
use nettai_frontend::replay::{Outcome, Replay};
use slint::{ModelRc, VecModel};
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

/// The row of `entry`, its names in `lang` if its game is loaded.
pub fn row(entry: &Entry, games: &Games, lang: &str) -> ReplayRow {
    let file = entry.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let replay = match &entry.replay {
        Ok(r) => r,
        Err(why) => return ReplayRow { file: file.into(), check: ReplayCheck::Unplayable, note: why.as_str().into(), ..ReplayRow::default() },
    };
    let mut row = ReplayRow {
        file: file.into(),
        game: replay.head.game.to_ascii_uppercase().into(),
        left_name: replay.info.names[0].as_str().into(),
        right_name: replay.info.names[1].as_str().into(),
        seconds: (replay.ticks.len() as f64 / FRAME_RATE) as i32,
        recorder: replay.info.side as i32,
        check: ReplayCheck::Checking,
        ..ReplayRow::default()
    };
    if let Some(when) = chrono::DateTime::from_timestamp(replay.info.when as i64, 0) {
        let t = when.with_timezone(&chrono::Local);
        (row.year, row.month, row.day, row.hour, row.minute) = (t.year(), t.month() as i32, t.day() as i32, t.hour() as i32, t.minute() as i32);
    }
    // The navis, by the content the replay's game loaded.
    if let Some(ready) = games.ready(&replay.head.game) {
        let content = ready.content();
        let graphics = ready.graphics(lang);
        let names = Names::of(content, &graphics);
        if let Ok(m) = nettai_match::binary::read_match(content, &replay.match_bytes) {
            row.left_navi = names.navi(m.sides[0].navi(content)).into();
            row.right_navi = names.navi(m.sides[1].navi(content)).into();
        }
    }
    match &entry.outcome {
        None => {}
        Some(Err(why)) => {
            row.check = ReplayCheck::Unplayable;
            row.note = why.as_str().into();
        }
        Some(Ok(o)) => {
            let rounds: Vec<i32> = o.rounds.iter().map(|w| match w {
                Some(0) => 1,
                Some(_) => 2,
                None => 3,
            }).collect();
            row.rounds = ModelRc::new(VecModel::from(rounds));
            row.result = match o.result {
                Some(BattleResult::Won) => 1,
                Some(BattleResult::Lost) => 2,
                Some(BattleResult::Drawn) => 3,
                _ => 0,
            };
            (row.check, row.note) = match (&o.diverged, &o.stopped) {
                (Some(d), _) => (ReplayCheck::Differs, d.as_str().into()),
                (None, Some(s)) => (ReplayCheck::Differs, s.as_str().into()),
                (None, None) => (ReplayCheck::Reproduces, Default::default()),
            };
        }
    }
    row
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
