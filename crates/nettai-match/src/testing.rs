//! Content for tests: content/bn6's definitions (and with them, every
//! game's: [`every_game`]) on a made-up asset index
//! (`testing::asset_names_used`), every sprite timed as the test content's
//! navi is (nothing from a ROM), with their own strings.

use nettai_battle::Content;
use std::sync::{Arc, OnceLock};

/// Every sprite of `c` timed as the test content's navi is.
fn timed(c: &mut Content) {
    use nettai_battle::content::testing;
    let mut navi = testing::content().animations.sprites[&testing::sprite(testing::NAVI_SPRITE)].clone();
    navi.resize(0x40, navi[1].clone());
    for h in 0..c.assets.sprites.len() {
        c.animations.sprites.insert(nettai_battle::content::SpriteId(h as u16), navi.clone());
    }
}

/// The repository's content directory.
const CONTENT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content");

/// content/'s games `games` (each by its index and what it requires), with
/// their strings, on a made-up asset index (each game's names in its own
/// game's pack), every sprite timed as the test content's navi is: defined,
/// or why not.
pub fn defined(games: &[&str]) -> Result<Content, String> {
    let mut report = nettai_content::report::Report::default();
    let games: Vec<String> = games.iter().map(|g| g.to_string()).collect();
    let read = nettai_content::index::read(std::path::Path::new(CONTENT), &games, &mut report).ok_or_else(|| report.to_string())?;
    defined_of(read)
}

/// What a read of content/ read, defined as [`defined`] defines it.
fn defined_of(read: nettai_content::index::Read) -> Result<Content, String> {
    let mut c = Content::default();
    c.scripts = read.scripts();
    c.strings = read.strings;
    c.assets = nettai_battle::content::testing::asset_names_for(&c.scripts);
    timed(&mut c);
    c.define().map_err(|e| e.message)?;
    Ok(c)
}

/// content/'s BN6, defined once per test process.
pub fn bn6_content() -> Arc<Content> {
    static BN6: OnceLock<Arc<Content>> = OnceLock::new();
    BN6.get_or_init(|| Arc::new(defined(&["bn6"]).unwrap_or_else(|e| panic!("content/bn6: {e}")))).clone()
}

/// Every game of this repository's content/ (its game packs, BN5 and BN6), one namespace, as the loaders load it: each by its index (a
/// game's unported chips don't load). Defined once per test process.
pub fn every_game() -> Arc<Content> {
    static ALL: OnceLock<Arc<Content>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut report = nettai_content::report::Report::default();
        let read = nettai_content::index::read_all(std::path::Path::new(CONTENT), &mut report).unwrap_or_else(|| panic!("content/: {report}"));
        Arc::new(defined_of(read).unwrap_or_else(|e| panic!("content/: {e}")))
    })
    .clone()
}
