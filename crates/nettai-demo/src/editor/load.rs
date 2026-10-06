//! The content the editor makes matches of: one game's, loaded as the
//! player loads it (nettai-frontend's `Found::find` and `Game::load`: the
//! game pack, the support packs it depends on, its asset pack), so a match
//! is played on the content it was edited on; and the games there are to
//! choose from (`nettai_content::pack::games`).

use crate::editor::pictures::Pictures;
use nettai_battle::Content;
use nettai_frontend::game::{Found, Game};
use std::path::{Path, PathBuf};

/// A game's content and where its packs are, and its chips' pictures (from
/// its asset pack).
pub struct Loaded {
    pub game: Game,
    pub pictures: Pictures,
}

/// Show what loading found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

/// The packs found (`packs`: `--pack`, in place of the found one of its
/// game).
fn found(packs: &[PathBuf]) -> Result<Found, String> {
    let found = Found::find(&nettai_content::pack::packs_dir(), packs).map_err(|e| {
        show(&e.report);
        format!("{e} (--pack)")
    })?;
    show(&found.report);
    Ok(found)
}

/// The content of `game` (`content_dir`: `--content`; `packs`: `--pack`),
/// with what loading found shown on the terminal.
pub fn load_game(content_dir: Option<&Path>, packs: &[PathBuf], game: &str) -> Result<Loaded, String> {
    let found = found(packs)?;
    let game = Game::load(&found, content_dir, game).map_err(|e| {
        show(&e.report);
        format!("{e} (--content, --pack)")
    })?;
    show(&game.report);
    let pictures = Pictures::load(&game.content, &game.packs).unwrap_or_else(|e| {
        eprintln!("{e}: the chips have no pictures");
        Pictures::default()
    });
    Ok(Loaded { game, pictures })
}

/// The games a match can be of: the content directory's games whose asset
/// pack is found.
pub fn games(content_dir: Option<&Path>, packs: &[PathBuf]) -> Vec<String> {
    let Ok(found) = found(packs) else { return Vec::new() };
    match nettai_content::pack::games(content_dir, &found.packs) {
        Ok(games) => games.into_iter().filter(|g| g.pack.is_some()).map(|g| g.game).collect(),
        Err(r) => {
            show(&r);
            Vec::new()
        }
    }
}

/// Whether `content` is `game`'s.
pub fn holds(content: &Content, game: &str) -> bool {
    content.game() == game
}
