//! BN6's content for tests: content/bn6's definitions (with the modules
//! it shares with BN5, content/common) on a made-up asset
//! index (`testing::asset_names_used`), every sprite timed as the test
//! content's navi is (nothing from a ROM), with its own strings.

use nettai_battle::Content;
use std::sync::{Arc, OnceLock};

/// content/bn6, defined once per test process.
pub fn bn6_content() -> Arc<Content> {
    use nettai_battle::content::testing;
    static BN6: OnceLock<Arc<Content>> = OnceLock::new();
    BN6.get_or_init(|| {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/bn6");
        let mut c = Content::default();
        c.scripts = testing::bn6_scripts();
        c.assets = testing::asset_names_for(&c.scripts);
        c.strings = nettai_content::locale::load(std::path::Path::new(dir), nettai_content::locale::OWN)
            .and_then(|s| s.ok_or_else(|| "no locales/en.toml".into()))
            .unwrap_or_else(|e| panic!("content/bn6: {e}"));
        let mut navi = testing::content().animations.sprites[&testing::sprite(testing::NAVI_SPRITE)].clone();
        navi.resize(0x40, navi[1].clone());
        for h in 0..c.assets.sprites.len() {
            c.animations.sprites.insert(nettai_battle::content::SpriteId(h as u16), navi.clone());
        }
        c.define().unwrap_or_else(|e| panic!("content/bn6: {e}"));
        Arc::new(c)
    })
    .clone()
}
