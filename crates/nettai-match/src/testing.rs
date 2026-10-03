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

/// content/bn6, defined once per test process.
pub fn bn6_content() -> Arc<Content> {
    use nettai_battle::content::testing;
    static BN6: OnceLock<Arc<Content>> = OnceLock::new();
    BN6.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
        let mut c = Content::default();
        c.scripts = nettai_battle::content::Scripts::root(nettai_battle::content::RootManifest::named("bn6"), testing::modules_under(dir));
        // (With the shared folder its modules require, content/common.)
        testing::add_shared(&mut c.scripts);
        c.assets = testing::asset_names_for(&c.scripts);
        c.strings = nettai_content::locale::load(std::path::Path::new(dir), nettai_content::locale::OWN)
            .and_then(|s| s.ok_or_else(|| "no locales/en.toml".into()))
            .unwrap_or_else(|e| panic!("content/bn6: {e}"));
        timed(&mut c);
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        Arc::new(c)
    })
    .clone()
}

/// Every folder of this repository's content/ (BN5's, BN6's and the
/// shared one), one namespace, as the loaders load it: each folder's chips
/// with no use yet left out (`nettai_content::pack::left_out_unported`).
/// Defined once per test process.
pub fn every_game() -> Arc<Content> {
    static ALL: OnceLock<Arc<Content>> = OnceLock::new();
    ALL.get_or_init(|| {
        let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../content"));
        let mut report = nettai_content::report::Report::default();
        let roots = nettai_content::root::read_all(dir, &mut report).unwrap_or_else(|| panic!("content/: {report}"));
        let mut c = Content::default();
        for mut root in roots {
            nettai_content::pack::left_out_unported(&mut root, &mut report);
            c.strings.merge(root.strings);
            c.scripts.add_root(root.manifest, root.modules);
        }
        c.assets = nettai_battle::content::testing::asset_names_for(&c.scripts);
        timed(&mut c);
        c.define().unwrap_or_else(|e| panic!("content/: {e}"));
        Arc::new(c)
    })
    .clone()
}
