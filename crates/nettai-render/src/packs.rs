//! The engine's assets as their packs hold them (docs/design/rules-in-luau.md
//! §7.4). The engine knows an asset by its handle over the loaded packs; the
//! frontend draws a pack's sprite, banner, mugshot or background by the
//! pack's own id for it, from that pack's graphics ([`Packs`]).

use nettai_assets::{Bundle, Hud, SpriteSheet};
use nettai_battle::content::{BackgroundId, BannerId, Content, InPack, MugshotId, PackId, PackSprite, RootId, SpriteId};
use nettai_content_api::AssetKind;

/// The loaded packs' graphics, by `PackId` (the content's `AssetNames::packs`
/// order), and the content's own pack's, which draws what belongs to no
/// asset (the HUD's frame, the custom screen's).
#[derive(Clone)]
pub struct Packs<'a> {
    bundles: Vec<&'a Bundle>,
    own: PackId,
}

impl<'a> Packs<'a> {
    /// One pack's graphics: every asset is its.
    pub fn one(bundle: &'a Bundle) -> Packs<'a> {
        Packs { bundles: vec![bundle], own: PackId(0) }
    }

    /// Several packs' graphics, by `PackId`; `own` the content's own.
    pub fn new(bundles: Vec<&'a Bundle>, own: PackId) -> Packs<'a> {
        assert!(own.index() < bundles.len(), "the content's own pack is loaded");
        Packs { bundles, own }
    }

    /// The graphics of pack `pack` (the content's own for one not loaded:
    /// a frontend of one pack).
    pub fn bundle(&self, pack: PackId) -> &'a Bundle {
        self.bundles.get(pack.index()).copied().unwrap_or(self.bundles[self.own.index()])
    }

    /// The content's own pack's graphics.
    pub fn own(&self) -> &'a Bundle {
        self.bundles[self.own.index()]
    }

    /// The graphics of root `root`'s assets pack (a game's: the arena's,
    /// a side's).
    pub fn of_root(&self, c: &Content, root: RootId) -> &'a Bundle {
        self.bundle(self.id_of_root(c, root))
    }

    /// Root `root`'s assets pack (the content's own for a root without
    /// one loaded).
    pub fn id_of_root(&self, c: &Content, root: RootId) -> PackId {
        let game = c.scripts.roots.get(root.index()).map(|r| r.assets());
        game.and_then(|g| c.assets.pack(g)).filter(|p| p.index() < self.bundles.len()).unwrap_or(self.own)
    }

    /// The graphics of the pack a definition's root names its assets in
    /// (a chip's icon and picture are its game's, under its own key).
    pub fn of_key(&self, c: &Content, key: &str) -> &'a Bundle {
        self.of_root(c, c.defs.root_of(key))
    }

    /// A chip's icon: its game's pack's, under its key there.
    pub fn chip_icon(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::Tiles> {
        self.of_key(c, key).hud.chip_icon(nettai_content_api::keys::local(key))
    }

    /// A chip's picture: its game's pack's, under its key there.
    pub fn chip_art(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::ChipArt> {
        self.of_key(c, key).custom.chip_art(nettai_content_api::keys::local(key))
    }

    /// Sprite `id`'s sheet, from its pack.
    pub fn sprite(&self, c: &Content, id: SpriteId) -> Option<&'a SpriteSheet> {
        let a = c.assets.sprite(id.0)?;
        self.bundle(a.pack).sprite(a.id.category, a.id.index)
    }

    /// Banner `id`'s pack's HUD and its number there.
    pub fn banner(&self, c: &Content, id: BannerId) -> Option<(&'a Hud, u8)> {
        let a = c.assets.number(AssetKind::Banner, id.0)?;
        Some((&self.bundle(a.pack).hud, a.id as u8))
    }

    /// Mugshot `m`'s pack's HUD and its number there.
    pub fn mugshot(&self, m: InPack<u8>) -> (&'a Hud, u8) {
        (&self.bundle(m.pack).hud, m.id)
    }

    /// Background `id`'s pack's graphics and its number there.
    pub fn background(&self, c: &Content, id: BackgroundId) -> Option<(&'a Bundle, u8)> {
        let a = c.assets.number(AssetKind::Background, id.0)?;
        Some((self.bundle(a.pack), a.id as u8))
    }
}

/// Sprite `id` as its pack holds it.
pub fn sprite(c: &Content, id: SpriteId) -> Option<PackSprite> {
    c.assets.sprite(id.0).map(|a| a.id)
}

/// The pack and number of `kind` asset `h`.
fn number(c: &Content, kind: AssetKind, h: u16) -> Option<InPack<u16>> {
    c.assets.number(kind, h)
}

/// Banner `id`'s number in its pack.
pub fn banner(c: &Content, id: BannerId) -> Option<u8> {
    number(c, AssetKind::Banner, id.0).map(|n| n.id as u8)
}

/// Mugshot `id`'s pack and number there.
pub fn mugshot(c: &Content, id: MugshotId) -> Option<InPack<u8>> {
    number(c, AssetKind::Mugshot, id.0).map(|n| InPack { pack: n.pack, id: n.id as u8 })
}

/// Background `id`'s number in its pack.
pub fn background(c: &Content, id: BackgroundId) -> Option<u8> {
    number(c, AssetKind::Background, id.0).map(|n| n.id as u8)
}

/// The name content gives `kind` asset `h` (`bn6:bomb`), for a problem's
/// text.
pub fn name(c: &Content, kind: AssetKind, h: u16) -> Option<&str> {
    let names = match kind {
        AssetKind::Sprite => c.assets.sprites.keys().nth(h as usize),
        AssetKind::Sound => c.assets.sounds.keys().nth(h as usize),
        AssetKind::Banner => c.assets.banners.keys().nth(h as usize),
        AssetKind::Background => c.assets.backgrounds.keys().nth(h as usize),
        AssetKind::Mugshot => c.assets.mugshots.keys().nth(h as usize),
    };
    names.map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::{ChipIcon, Tiles};
    use nettai_battle::content::{RootManifest, testing};
    use nettai_content_api::keys;

    /// The test content with a `twin` root and pack beside it: twin's
    /// sprite `navi` is its pack's 0-0, the same number as the test pack's
    /// navi's.
    fn content() -> Content {
        let mut c = testing::build();
        let mut index = nettai_content_api::PackIndex::default();
        index.sprites.insert("navi".into(), testing::NAVI_SPRITE);
        index.mugshots.insert("face".into(), 3);
        let frame = nettai_battle::content::AnimFrame { duration: 4, flags: nettai_battle::object::sprite::FRAME_LAST };
        testing::add_pack(&mut c, "twin", index, [(testing::NAVI_SPRITE, vec![vec![frame]])].into_iter().collect());
        let manifest = RootManifest { name: "twin".into(), assets: None, requires: vec![testing::ROOT.into()] };
        c.scripts.add_root(manifest, Default::default());
        c.define().unwrap_or_else(|e| panic!("{e}"));
        c
    }

    /// A pack's graphics, told apart by their navi sheet's tile count, with
    /// an icon for the chip `chip` (its key in the pack).
    fn bundle(tiles: usize, chip: &str) -> Bundle {
        let sheet = SpriteSheet {
            category: testing::NAVI_SPRITE.category,
            index: testing::NAVI_SPRITE.index,
            tilesets: vec![Tiles { pixels: vec![1; tiles * Tiles::TILE] }],
            ..SpriteSheet::default()
        };
        let mut b = Bundle { sprites: vec![sheet], ..Bundle::default() };
        b.hud.chip_icons.push(ChipIcon { key: chip.into(), tiles: Tiles { pixels: vec![tiles as u8; 4 * Tiles::TILE] } });
        b
    }

    /// docs/design/rules-in-luau.md §7.4: an asset draws from its own
    /// pack's graphics, a root's game from its pack's, and a chip's icon
    /// is its game's pack's under its key there.
    #[test]
    fn each_asset_draws_from_its_own_pack() {
        let c = content();
        let (test, twin) = (c.assets.pack(testing::ROOT).unwrap(), c.assets.pack("twin").unwrap());
        let chip = c.defs.chip(c.defs.chip_by_key("gundels3").expect("a test chip")).key.clone();
        let (a, b) = (bundle(1, keys::local(&chip)), bundle(2, keys::local(&chip)));
        let mut by_pack = vec![&a, &a];
        by_pack[twin.index()] = &b;
        let packs = Packs::new(by_pack, test);
        let tiles = |s: Option<&SpriteSheet>| s.expect("a sheet").tilesets[0].pixels.len() / Tiles::TILE;
        assert_eq!(tiles(packs.sprite(&c, testing::sprite_named(&c, "test-navi"))), 1);
        assert_eq!(tiles(packs.sprite(&c, testing::sprite_named(&c, "twin:navi"))), 2, "twin's sprite is twin's pack's");
        let root = |name: &str| c.defs.root_id(name).expect("a root");
        assert!(std::ptr::eq(packs.of_root(&c, root("twin")), &b));
        assert!(std::ptr::eq(packs.of_root(&c, root(testing::ROOT)), &a));
        assert!(std::ptr::eq(packs.own(), &a));
        // A chip's icon by its key in its pack ("gundels3", not
        // "test:gundels3").
        assert_eq!(packs.chip_icon(&c, &chip).map(|t| t.pixels[0]), Some(1));
        // A mugshot is its pack's number in its pack's HUD.
        let face = c.assets.handle(AssetKind::Mugshot, "twin:face").expect("twin's mugshot");
        let m = mugshot(&c, MugshotId(face)).expect("a mugshot");
        assert_eq!((m.pack, m.id), (twin, 3));
        assert!(std::ptr::eq(packs.mugshot(m).0, &b.hud));
        // One pack's frontend draws everything from it.
        let one = Packs::one(&a);
        assert_eq!(tiles(one.sprite(&c, testing::sprite_named(&c, "twin:navi"))), 1);
    }
}
