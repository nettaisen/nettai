//! The content the editor makes matches of: a game's, loaded as the
//! frontend loads it (one game, its support folders, its pack). One
//! function each for the games there are and a game's content, so the
//! loader's one-game form (`nettai_content::pack::load_game` and `games`)
//! is a small switch: until it lands, every folder whose pack is found
//! loads (BN5's folder still requires BN6's modules), and a match names
//! its game's alone (`nettai_match::ids`).

use crate::pictures::Pictures;
use nettai_battle::Content;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A game's content, its pictures (from the packs) and its folders (their
/// strings tables).
pub struct Loaded {
    pub content: Arc<Content>,
    pub pictures: Pictures,
    pub roots: Vec<PathBuf>,
}

/// Show what loading found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
}

/// The content of `game` (`content_dir`: `--content`; `packs`: `--pack`),
/// with what loading found shown on the terminal.
pub fn load_game(content_dir: Option<&Path>, packs: &[PathBuf], game: &str) -> Result<Loaded, String> {
    let mut report = nettai_content::report::Report::default();
    let found = nettai_content::pack::find(&nettai_content::pack::packs_dir(), packs, &mut report);
    show(&report);
    let found = found.ok_or("can't read the packs given (--pack, $BN6_PACK)")?;
    let loaded = nettai_content::pack::load_found(content_dir, &found).map_err(|r| {
        show(&r);
        format!("can't load {game}'s battle content (--content, --pack)")
    })?;
    show(&loaded.report);
    for (root, why) in &loaded.left_out {
        eprintln!("the content folder {root} is left out: {why}");
    }
    let content = Arc::new(loaded.content);
    if !holds(&content, game) {
        return Err(format!("no {game} content is loaded (the games there are: {})", nettai_match::ids::games(&content).join(", ")));
    }
    let pictures = Pictures::load(&content, &loaded.packs).unwrap_or_else(|e| {
        eprintln!("{e}: the chips have no pictures");
        Pictures::default()
    });
    Ok(Loaded { content, pictures, roots: loaded.roots })
}

/// The games a match can be of: those whose content and pack are found
/// (`content`: the content loaded).
pub fn games(content: &Content) -> Vec<String> {
    nettai_match::ids::games(content)
}

/// Whether `content` is `game`'s.
pub fn holds(content: &Content, game: &str) -> bool {
    nettai_match::ids::games(content).iter().any(|g| g == game)
}
