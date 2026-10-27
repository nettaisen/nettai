//! What the tests that run on the original ROMs share: the ROMs found in a folder by their headers, the games
//! asked for, each game's tables as the repository's content has them.
#![allow(dead_code)]

use nettai_extract::{Game, RomSet, Tables};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The repository's content directory.
pub fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// `game`'s tables, as the repository's content has them.
pub fn tables(game: Game) -> Tables {
    Tables::of_content(game, &content()).unwrap_or_else(|e| panic!("{e}"))
}

/// The ROMs of the folder `NETTAI_EXTRACT_ROM_DIR` (any file names: ~/Documents/Tango/roms serves as it is), by
/// the code in their headers, of every game the extractor takes.
pub fn roms_by_code() -> BTreeMap<String, PathBuf> {
    let dir = PathBuf::from(std::env::var_os("NETTAI_EXTRACT_ROM_DIR").expect("NETTAI_EXTRACT_ROM_DIR: a folder of the original ROMs"));
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())).flatten() {
        let path = entry.path();
        let Ok(file) = std::fs::File::open(&path) else { continue };
        let mut header = Vec::new();
        if file.take(nettai_extract::HEADER_LEN as u64).read_to_end(&mut header).is_err() {
            continue;
        }
        if let Some((_, code)) = nettai_extract::recognize(&header) {
            out.entry(code.to_string()).or_insert(path);
        }
    }
    out
}

/// The full set of `game`'s ROMs from `NETTAI_EXTRACT_ROM_DIR`.
pub fn full_set(game: Game) -> RomSet {
    let found = roms_by_code();
    let mut roms = RomSet::default();
    for code in game.codes() {
        let path = found.get(code).unwrap_or_else(|| panic!("{}: no {code} ROM in NETTAI_EXTRACT_ROM_DIR", game.id()));
        roms.insert(std::fs::read(path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
    roms
}

/// The games asked for: `NETTAI_EXTRACT_GAMES` (`exe6`, `exe5,exe6`), else all three.
pub fn games() -> Vec<Game> {
    match std::env::var("NETTAI_EXTRACT_GAMES") {
        Ok(list) => list
            .split([',', ' '])
            .filter(|s| !s.is_empty())
            .map(|s| Game::named(s).unwrap_or_else(|| panic!("NETTAI_EXTRACT_GAMES: no game {s}")))
            .collect(),
        Err(_) => Game::ALL.to_vec(),
    }
}
