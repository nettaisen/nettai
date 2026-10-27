//! What the tests that run on the original ROMs share: the ROMs found in a folder by their headers, the games
//! asked for, each game's recipe as the repository's content has it.
#![allow(dead_code)]

use nettai_extract::{Recipe, RomSet};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The repository's content directory.
pub fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// Game `game`'s recipe, as the repository's content has it.
pub fn recipe(game: &str) -> Recipe {
    Recipe::of_content(&content(), game).unwrap_or_else(|e| panic!("{e}"))
}

/// The ROMs of the folder `NETTAI_EXTRACT_ROM_DIR` (any file names: ~/Documents/Tango/roms serves as it is), by
/// the game code in their headers.
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
        if let Some(code) = nettai_extract::header_code(&header) {
            out.entry(code.to_string()).or_insert(path);
        }
    }
    out
}

/// The full set of `recipe`'s ROMs (every one it states) from `NETTAI_EXTRACT_ROM_DIR`.
pub fn full_set(recipe: &Recipe) -> RomSet {
    let found = roms_by_code();
    let mut roms = recipe.rom_set();
    for code in recipe.roms() {
        let path = found.get(code).unwrap_or_else(|| panic!("{}: no {code} ROM in NETTAI_EXTRACT_ROM_DIR", recipe.game()));
        roms.insert(std::fs::read(path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
    roms
}

/// The games asked for: `NETTAI_EXTRACT_GAMES` (`exe6`, `exe5,exe6`), else every game of the repository's content
/// that has a recipe.
pub fn games() -> Vec<String> {
    use nettai_content_api::packs;
    match std::env::var("NETTAI_EXTRACT_GAMES") {
        Ok(list) => list.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect(),
        Err(_) => packs::packs(&content())
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .filter(|p| p.kind == packs::PackKind::Game && packs::has_recipe(&content(), &p.id))
            .map(|p| p.id)
            .collect(),
    }
}
