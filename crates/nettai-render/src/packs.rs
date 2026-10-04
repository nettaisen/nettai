//! The engine's assets as their packs hold them (docs/design/rules-in-luau.md
//! §7.4). The engine knows an asset by its handle over the loaded packs; the
//! frontend draws a pack's sprite, banner, mugshot or background by the
//! pack's own id for it, from that pack's graphics ([`Packs`]).

use nettai_assets::{Bundle, Hud, SpriteSheet};
use nettai_battle::content::{BackgroundId, BannerId, Content, InPack, MugshotId, PackId, PackSprite, SpriteId};
use nettai_content_api::AssetKind;

/// The loaded packs' graphics, by `PackId` (the content's `AssetNames::packs`
/// order), and the frontend's own game's pack's (BN6's, by name:
/// `nettai_match::DEFAULT_GAME`), which draws what belongs to no asset (the
/// HUD's frame, the custom screen's).
#[derive(Clone)]
pub struct Packs<'a> {
    bundles: Vec<&'a Bundle>,
    own: PackId,
    /// The console's version (`Renderer::console_version`): the one whose
    /// own art a chip each version draws its own way shows.
    version: Option<&'static str>,
}

impl<'a> Packs<'a> {
    /// One pack's graphics: every asset is its.
    pub fn one(bundle: &'a Bundle) -> Packs<'a> {
        Packs { bundles: vec![bundle], own: PackId(0), version: None }
    }

    /// Several packs' graphics, by `PackId`; `own` the frontend's own
    /// game's.
    pub fn new(bundles: Vec<&'a Bundle>, own: PackId) -> Packs<'a> {
        assert!(own.index() < bundles.len(), "the frontend's own pack is loaded");
        Packs { bundles, own, version: None }
    }

    /// The graphics of pack `pack` (the own pack's for one not loaded: a
    /// frontend of one pack).
    pub fn bundle(&self, pack: PackId) -> &'a Bundle {
        self.bundles.get(pack.index()).copied().unwrap_or(self.bundles[self.own.index()])
    }

    /// The own pack's graphics.
    pub fn own(&self) -> &'a Bundle {
        self.bundles[self.own.index()]
    }

    /// The own pack.
    pub fn own_pack(&self) -> PackId {
        self.own
    }

    /// The graphics of the game's pack (a match plays one game,
    /// docs/design/content-model-v2.md §4.0): its field, its custom screen,
    /// its chips' icons and pictures.
    pub fn game(&self, c: &Content) -> &'a Bundle {
        self.bundle(self.game_id(c))
    }

    /// The game's pack (the own pack when it isn't loaded).
    pub fn game_id(&self, c: &Content) -> PackId {
        c.assets.pack(c.game()).filter(|p| p.index() < self.bundles.len()).unwrap_or(self.own)
    }

    /// Draw a console of `version` (`Renderer::console_version`; None: the
    /// pack's first).
    pub fn set_version(&mut self, version: Option<&'static str>) {
        self.version = version;
    }

    /// A chip's icon: its game's pack's, under its key there, else its
    /// version's (`version_key`).
    pub fn chip_icon(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::Tiles> {
        let pack = self.game(c);
        let local = nettai_content_api::keys::local(key);
        pack.hud.chip_icon(local).or_else(|| pack.hud.chip_icon(&self.version_key(pack, local)?))
    }

    /// A chip's picture: its game's pack's, under its key there, else its
    /// version's (`version_key`).
    pub fn chip_art(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::ChipArt> {
        let pack = self.game(c);
        let local = nettai_content_api::keys::local(key);
        pack.custom.chip_art(local).or_else(|| pack.custom.chip_art(&self.version_key(pack, local)?))
    }

    /// The key of the console's version's art of a chip each version of
    /// `pack`'s game draws its own way (BN5's navi chips: the pack has it
    /// once a version, `{key}-{version}`, docs/design/bn5-map.md §11): the
    /// console's version if the pack has it, else the pack's first.
    fn version_key(&self, pack: &Bundle, local: &str) -> Option<String> {
        let mut versions = pack.custom.chip_art.iter().filter_map(|a| a.version.as_deref());
        let first = versions.clone().next()?;
        let version = self.version.filter(|&v| versions.any(|w| w == v)).unwrap_or(first);
        Some(format!("{local}-{version}"))
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
    use nettai_battle::content::testing;
    use nettai_content_api::keys;

    /// The test content, defined.
    fn content() -> Content {
        let mut c = testing::build();
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

    /// docs/design/rules-in-luau.md §7.4: an asset draws from its pack's
    /// graphics, the game's from the game's pack, and a chip's icon is
    /// the game's pack's under its key there.
    #[test]
    fn each_asset_draws_from_its_own_pack() {
        let c = content();
        let chip = c.defs.chip(c.defs.chip_by_key("gundels3").expect("a test chip")).key.clone();
        let a = bundle(1, keys::local(&chip));
        let packs = Packs::one(&a);
        let tiles = |s: Option<&SpriteSheet>| s.expect("a sheet").tilesets[0].pixels.len() / Tiles::TILE;
        assert_eq!(tiles(packs.sprite(&c, testing::sprite_named(&c, "test-navi"))), 1);
        assert!(std::ptr::eq(packs.game(&c), &a));
        assert!(std::ptr::eq(packs.own(), &a));
        // A chip's icon by its key in its pack ("gundels3", not
        // "gundels3").
        assert_eq!(packs.chip_icon(&c, &chip).map(|t| t.pixels[0]), Some(1));
    }
}
