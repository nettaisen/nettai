//! What the tests that run on the original ROMs share: the ROMs where the tools find them, the games asked for,
//! each game's recipe as the repository's content has it.
#![allow(dead_code)]

use nettai_extract::{Recipe, RomSet, RomSource};
use std::path::{Path, PathBuf};

/// The repository's content directory.
pub fn content() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

/// Game `game`'s recipe, as the repository's content has it: the game's content loaded, its recipe with it.
pub fn recipe(game: &str) -> Recipe {
    let (c, _) = nettai_content::load::load_battle(&content(), game).unwrap_or_else(|r| panic!("{game}: {r}"));
    nettai_content::load::recipe(&c).unwrap_or_else(|e| panic!("{e}"))
}

/// The ROMs as the tools find them: `$NETTAI_ROMS` (a folder of them, any file names: ~/Documents/Tango/roms serves
/// as it is; or the Legacy Collection's folder), else the Legacy Collection where Steam installed it.
pub fn roms() -> RomSource {
    RomSource::from_env().unwrap_or_else(|e| panic!("{e}"))
}

/// The full set of `recipe`'s ROMs (every one it states), from [`roms`].
pub fn full_set(recipe: &Recipe) -> RomSet {
    roms().read(recipe).unwrap_or_else(|e| panic!("{e}"))
}

/// The games asked for: `NETTAI_EXTRACT_GAMES` (`exe6`, `exe5,exe6`), else every game of the repository's content
/// (each has a recipe).
pub fn games() -> Vec<String> {
    match std::env::var("NETTAI_EXTRACT_GAMES") {
        Ok(list) => list.split([',', ' ']).filter(|s| !s.is_empty()).map(String::from).collect(),
        Err(_) => nettai_content::index::games(&content()).unwrap_or_else(|e| panic!("{e}")),
    }
}
