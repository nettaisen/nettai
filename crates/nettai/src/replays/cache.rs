//! What playing replays out gave, kept between the app's runs (docs/app.md §16), so that a list's scores aren't
//! worked out again each time the app starts: a file of the app's own beside its settings
//! (`replays-played.toml`), never part of a replay. Each outcome is kept by the hash of the replay file's bytes
//! and the hash of the content it was played on, all of them of one engine. A file whose bytes changed has another
//! hash, so it is played out again; what is kept of files no longer listed is dropped as it is written. A cache
//! that can't be read (or of another engine, or written by another version of this module) is no cache: it is
//! made again.
//!
//! Only what played to an outcome is kept. A replay that isn't played here is told so before any of it is played
//! (`nettai_frontend::replay::set_of`), which costs little to find again.

use nettai_battle::BattleResult;
use nettai_frontend::replay::{ENGINE, End, Outcome};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hasher;
use std::path::PathBuf;

/// The layout of the file this module writes: one written by another is made again.
const LAYOUT: u32 = 1;

/// What an outcome is kept by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    /// The replay file's bytes, hashed ([`digest`]).
    pub file: u64,
    /// The content it was played on (`Content::hash`).
    pub content: u64,
}

/// A file's bytes, hashed: the engine's stable hash, the same on every machine and every build of the app.
pub fn digest(bytes: &[u8]) -> u64 {
    let mut h = nettai_battle::digest::StableHasher::new();
    h.write(bytes);
    h.write_usize(bytes.len());
    h.finish()
}

/// The outcomes kept, as read from the file and added to since.
pub struct Cache {
    path: PathBuf,
    kept: HashMap<Id, Outcome>,
    /// Something was added or dropped since the file was read or written.
    changed: bool,
}

/// The file.
#[derive(Serialize, Deserialize)]
struct Kept {
    layout: u32,
    engine: String,
    #[serde(default)]
    played: Vec<Row>,
}

/// An outcome in the file.
#[derive(Serialize, Deserialize)]
struct Row {
    /// The two hashes, in hex.
    file: String,
    content: String,
    ticks: u64,
    /// Each round played to its end, its winner by side; 2 for a draw.
    rounds: Vec<u8>,
    /// The set's result for side 0 (the engine's code), if the set was played to its end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    result: Option<u8>,
    /// How the recording ends: "set", "open" or "cut".
    end: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diverged: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stopped: Option<String>,
}

fn result_of(code: u8) -> Option<BattleResult> {
    [BattleResult::Won, BattleResult::Lost, BattleResult::Drawn, BattleResult::Escaped, BattleResult::CommError, BattleResult::Terminated, BattleResult::TerminatedA]
        .into_iter()
        .find(|r| *r as u8 == code)
}

const ENDS: [(End, &str); 3] = [(End::Set, "set"), (End::Open, "open"), (End::Cut, "cut")];

impl Row {
    fn of(id: &Id, o: &Outcome) -> Row {
        Row {
            file: format!("{:016x}", id.file),
            content: format!("{:016x}", id.content),
            ticks: o.ticks,
            rounds: o.rounds.iter().map(|w| w.unwrap_or(2)).collect(),
            result: o.result.map(|r| r as u8),
            end: ENDS.iter().find(|(e, _)| *e == o.end).map_or("open", |(_, name)| name).to_string(),
            diverged: o.diverged.clone(),
            stopped: o.stopped.clone(),
        }
    }

    /// The outcome it says, if all of it reads.
    fn read(self) -> Option<(Id, Outcome)> {
        let hex = |s: &str| u64::from_str_radix(s, 16).ok();
        let id = Id { file: hex(&self.file)?, content: hex(&self.content)? };
        let rounds = self.rounds.iter().map(|&w| (w <= 2).then_some((w < 2).then_some(w))).collect::<Option<Vec<_>>>()?;
        let result = match self.result {
            Some(code) => Some(result_of(code)?),
            None => None,
        };
        let end = ENDS.iter().find(|(_, name)| *name == self.end)?.0;
        Some((id, Outcome { ticks: self.ticks, rounds, result, end, diverged: self.diverged, stopped: self.stopped }))
    }
}

impl Cache {
    /// The cache kept at `path`: empty where there is none, or none that reads (which the log is told).
    pub fn load(path: PathBuf) -> Cache {
        let mut cache = Cache { path, kept: HashMap::new(), changed: false };
        let text = match nettai_fs::read_to_string(&cache.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return cache,
            Err(e) => {
                crate::errors::log(&format!("reading {} (it is made again)", cache.path.display()), &e);
                return cache;
            }
        };
        match toml::from_str::<Kept>(&text) {
            Ok(kept) if kept.layout == LAYOUT && kept.engine == ENGINE => cache.kept = kept.played.into_iter().filter_map(Row::read).collect(),
            // (Another engine's outcomes are of no use here.)
            Ok(_) => {}
            Err(e) => crate::errors::log(&format!("reading {} (it is made again)", cache.path.display()), &e),
        }
        cache
    }

    /// How many outcomes are kept.
    pub fn len(&self) -> usize {
        self.kept.len()
    }

    /// What playing the replay `id` says out gave, if that is kept.
    pub fn get(&self, id: &Id) -> Option<&Outcome> {
        self.kept.get(id)
    }

    /// Keep what playing a replay out gave.
    pub fn put(&mut self, id: Id, outcome: Outcome) {
        if self.kept.get(&id) != Some(&outcome) {
            self.kept.insert(id, outcome);
            self.changed = true;
        }
    }

    /// Write the file, if anything changed, keeping only the outcomes of files `listed` says are in the folder.
    /// (A file that can't be written is told to the log, and is tried again at the next change.)
    pub fn save(&mut self, listed: impl Fn(u64) -> bool) {
        let before = self.kept.len();
        self.kept.retain(|id, _| listed(id.file));
        if !self.changed && self.kept.len() == before {
            return;
        }
        let mut played: Vec<Row> = self.kept.iter().map(|(id, o)| Row::of(id, o)).collect();
        played.sort_by(|a, b| (&a.file, &a.content).cmp(&(&b.file, &b.content)));
        let kept = Kept { layout: LAYOUT, engine: ENGINE.to_string(), played };
        let text = match toml::to_string(&kept) {
            Ok(text) => format!("# What playing each replay out gave, by the hash of its file's bytes and of the content it was played on:\n# nettai's own, made again whenever it can't be read.\n{text}"),
            Err(e) => return crate::errors::log("writing the replays' outcomes", &e),
        };
        // (Written beside it, then put in its place: a cache is whole or not there.)
        let part = self.path.with_extension("toml.part");
        let written = self.path.parent().map_or(Ok(()), nettai_fs::create_dir_all).and_then(|()| nettai_fs::write(&part, text)).and_then(|()| nettai_fs::rename(&part, &self.path));
        match written {
            Ok(()) => self.changed = false,
            Err(e) => crate::errors::log(&format!("writing {}", self.path.display()), &e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(ticks: u64) -> Outcome {
        Outcome { ticks, rounds: vec![Some(0), None, Some(1)], result: Some(BattleResult::Drawn), end: End::Set, diverged: None, stopped: Some("the engine stopped".into()) }
    }

    /// What was kept reads back as it was, and only what the folder still lists is written; a file that isn't
    /// a cache, or one of another engine, is an empty cache; one with an outcome that doesn't read keeps the rest.
    #[test]
    fn outcomes_are_kept_by_the_files_hash_and_read_back() {
        let dir = std::env::temp_dir().join(format!("nettai-replays-cache-{}", std::process::id()));
        let _ = nettai_fs::remove_dir_all(&dir);
        let path = dir.join("replays-played.toml");
        let (a, b, gone) = (Id { file: 1, content: 7 }, Id { file: 2, content: 7 }, Id { file: 3, content: 7 });
        let mut cache = Cache::load(path.clone());
        assert_eq!(cache.len(), 0);
        cache.put(a, outcome(100));
        cache.put(b, Outcome { result: None, end: End::Cut, rounds: Vec::new(), diverged: Some("tick 9".into()), stopped: None, ..outcome(5) });
        cache.put(gone, outcome(1));
        cache.save(|file| file != 3);
        let back = Cache::load(path.clone());
        assert_eq!((back.len(), back.get(&a), back.get(&b).map(|o| (o.end, o.diverged.as_deref()))), (2, Some(&outcome(100)), Some((End::Cut, Some("tick 9")))));
        assert!(back.get(&gone).is_none() && back.get(&Id { file: 1, content: 8 }).is_none());
        assert!(!nettai_fs::exists(dir.join("replays-played.toml.part")));

        let text = nettai_fs::read_to_string(&path).unwrap();
        nettai_fs::write(&path, text.replace("ticks = 100", "ticks = \"many\"")).unwrap();
        assert_eq!(Cache::load(path.clone()).len(), 0);
        nettai_fs::write(&path, text.replace("rounds = [0, 2, 1]", "rounds = [0, 5, 1]")).unwrap();
        assert_eq!(Cache::load(path.clone()).len(), 1);
        nettai_fs::write(&path, text.replace(&format!("engine = \"{ENGINE}\""), "engine = \"another\"")).unwrap();
        assert_eq!(Cache::load(path.clone()).len(), 0);
        nettai_fs::write(&path, "not a cache").unwrap();
        let mut again = Cache::load(path.clone());
        assert_eq!(again.len(), 0);
        again.put(a, outcome(100));
        again.save(|_| true);
        assert_eq!(Cache::load(path).len(), 1);
        let _ = nettai_fs::remove_dir_all(&dir);
    }

    /// A file's hash changes with any byte of it.
    #[test]
    fn a_files_hash_is_of_all_its_bytes() {
        let bytes = b"NTRP\x01a replay's bytes".to_vec();
        let mut other = bytes.clone();
        other[9] ^= 1;
        assert_ne!(digest(&bytes), digest(&other));
        assert_ne!(digest(&bytes), digest(&[bytes.as_slice(), &[0]].concat()));
        assert_eq!(digest(&bytes), digest(&bytes.clone()));
    }
}
