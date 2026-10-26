//! Each game's content (docs/design/content-deployment.md §5): the content directory its matches are played from,
//! which every reader of a game's content asks here (its loading, its names, its music's and backgrounds' names,
//! its look).
//!
//! A build run from the repository, or with `$NETTAI_CONTENT`, plays every game from that one content directory
//! (`nettai_content::index::development`).

use std::path::PathBuf;
use std::sync::OnceLock;

/// Where the games' content is.
enum Source {
    /// One content directory holding every game: development's.
    Dir(PathBuf),
}

/// The process's source, chosen the first time it is asked for.
fn source() -> &'static Source {
    static SOURCE: OnceLock<Source> = OnceLock::new();
    SOURCE.get_or_init(|| Source::Dir(nettai_content::index::development().unwrap_or_else(nettai_content::index::content)))
}

/// Game `game`'s content directory, if there is content of it.
pub fn dir(game: &str) -> Option<PathBuf> {
    match source() {
        Source::Dir(dir) => nettai_content::index::games(dir).ok()?.iter().any(|g| g == game).then(|| dir.clone()),
    }
}

/// The games there is content of, by id.
pub fn games() -> Result<Vec<String>, String> {
    match source() {
        Source::Dir(dir) => nettai_content::index::games(dir).map_err(|e| format!("{}: {e}", dir.display())),
    }
}

/// Each game with its content directory: what the background pool is read from (every game's backgrounds,
/// `nettai_content::locale::background_pool_of`).
pub fn pool() -> Vec<(String, PathBuf)> {
    games().unwrap_or_default().into_iter().filter_map(|game| Some((game.clone(), dir(&game)?))).collect()
}
