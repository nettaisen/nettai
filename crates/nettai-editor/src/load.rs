//! The content the editor makes matches of: one game's, loaded as the
//! frontend loads it (`nettai_content::pack::load_game`: the game pack, the
//! support packs it uses, its asset pack), and the games there are to
//! choose from (`nettai_content::pack::games`).

use crate::pictures::Pictures;
use nettai_battle::Content;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A game's content, its chips' pictures (from its asset pack), and the
/// content directory it came from (its strings tables).
pub struct Loaded {
    pub content: Arc<Content>,
    pub pictures: Pictures,
    pub dir: PathBuf,
    pub game: String,
}

/// Show what loading found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

/// The packs found (`packs`: `--pack`, in place of the found one of its
/// game).
fn found(packs: &[PathBuf]) -> Result<Vec<nettai_content::pack::Found>, String> {
    let mut report = nettai_content::report::Report::default();
    let found = nettai_content::pack::find(&nettai_content::pack::packs_dir(), packs, &mut report);
    show(&report);
    found.ok_or_else(|| "can't read the packs given (--pack, $BN6_PACK)".into())
}

/// The content of `game` (`content_dir`: `--content`; `packs`: `--pack`),
/// with what loading found shown on the terminal.
pub fn load_game(content_dir: Option<&Path>, packs: &[PathBuf], game: &str) -> Result<Loaded, String> {
    let found = found(packs)?;
    let loaded = nettai_content::pack::load_game(content_dir, game, &found).map_err(|r| {
        show(&r);
        format!("can't load {game}'s battle content (--content, --pack)")
    })?;
    show(&loaded.report);
    let content = Arc::new(loaded.content);
    let pictures = Pictures::load(&content, std::slice::from_ref(&loaded.pack)).unwrap_or_else(|e| {
        eprintln!("{e}: the chips have no pictures");
        Pictures::default()
    });
    Ok(Loaded { content, pictures, dir: loaded.dir, game: loaded.game })
}

/// The games a match can be of: the content directory's games whose asset
/// pack is found.
pub fn games(content_dir: Option<&Path>, packs: &[PathBuf]) -> Vec<String> {
    let Ok(found) = found(packs) else { return Vec::new() };
    match nettai_content::pack::games(content_dir, &found) {
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
