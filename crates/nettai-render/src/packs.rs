//! The engine's assets as the game's graphics hold them (docs/design/rules-in-luau.md
//! §7.4). The engine knows an asset by its handle over the names the
//! content gives; the frontend draws a sprite, banner, mugshot or
//! background by the game's own number for it ([`AssetNumbers`], which
//! the loader made when it checked the game's extracted assets against the
//! content), from the game's graphics ([`Packs`]: a name from when they were
//! asset packs, by `PackId`, of which a host loads one). A round may be drawn
//! on another game's background (the content's background pool: docs/app.md
//! §1, "The battle's background"), extracted from that game's ROMs: those are
//! [`OtherBackgrounds`], beside the game's graphics.

use nettai_assets::{Background, Bundle, Hud, SpriteSheet};
use nettai_battle::content::{BackgroundId, BannerId, Content, InPack, MugshotId, AssetNumbers, PackId, SpriteId};
use nettai_battle::setup::BattleSettings;
use nettai_content_api::AssetKind;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Other games' backgrounds, each from its own game's pack, by its game and
/// its name there: what a round drawn on one of the background pool's
/// others draws (`Content::other_background`). A host loads those of the
/// packs it has (nettai-frontend's `Game::graphics`); one whose pack it
/// hasn't isn't here, and the round is drawn on its stage's own
/// ([`Packs::round_background`]).
#[derive(Clone, Debug, Default)]
pub struct OtherBackgrounds(BTreeMap<String, BTreeMap<String, Background>>);

impl OtherBackgrounds {
    /// Game `game`'s backgrounds, each by its name in its pack.
    pub fn insert(&mut self, game: &str, backgrounds: BTreeMap<String, Background>) {
        self.0.entry(game.to_string()).or_default().extend(backgrounds);
    }

    /// Game `game`'s background `name`, if its pack is here and has it.
    pub fn get(&self, game: &str, name: &str) -> Option<&Background> {
        self.0.get(game)?.get(name)
    }

    /// How many there are, of every game.
    pub fn len(&self) -> usize {
        self.0.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The loaded packs' graphics, held: by `PackId`, each shared, and which is
/// the frontend's own. A renderer keeps one, so it borrows from nobody (a
/// host keeps a renderer wherever it keeps its state, and several renderers
/// share the same graphics); what a frame draws from is a [`Packs`] over
/// them ([`PackGraphics::packs`]).
#[derive(Clone)]
pub struct PackGraphics {
    bundles: Vec<Arc<Bundle>>,
    own: PackId,
    others: Arc<OtherBackgrounds>,
    assets: Arc<AssetNumbers>,
}

impl PackGraphics {
    /// Every pack's drawings (`Bundle::drawings`), each once; the
    /// chatbox's coverages made now (`shapes::coverage`).
    pub fn drawings(&self) -> Arc<Vec<nettai_assets::shapes::Drawing>> {
        let mut seen = std::collections::HashSet::new();
        let all: Vec<nettai_assets::shapes::Drawing> =
            self.bundles.iter().flat_map(|b| b.drawings()).filter(|d| d.hash != 0 && seen.insert(d.hash)).map(|d| nettai_assets::shapes::Drawing { words: Vec::new(), ..d.clone() }).collect();
        for b in &self.bundles {
            for d in b.hud.shapes.chatbox.iter().flatten().flat_map(|m| m.parts.iter().map(|p| &p.drawing)) {
                crate::shapes::coverage(d);
            }
        }
        Arc::new(all)
    }

    /// One pack's graphics: every asset is its.
    pub fn one(bundle: Arc<Bundle>) -> PackGraphics {
        PackGraphics { bundles: vec![bundle], own: PackId(0), others: Default::default(), assets: Default::default() }
    }

    /// Several packs' graphics, by `PackId`; `own` the frontend's own
    /// game's.
    pub fn new(bundles: Vec<Arc<Bundle>>, own: PackId) -> PackGraphics {
        assert!(own.index() < bundles.len(), "the frontend's own pack is loaded");
        PackGraphics { bundles, own, others: Default::default(), assets: Default::default() }
    }

    /// These graphics, with other games' backgrounds beside them (the
    /// background pool's: [`OtherBackgrounds`]).
    pub fn with_others(self, others: Arc<OtherBackgrounds>) -> PackGraphics {
        PackGraphics { others, ..self }
    }

    /// These graphics, drawing the content's assets as the game's assets
    /// number them (`nettai_content::load::asset_numbers`).
    pub fn with_assets(self, assets: Arc<AssetNumbers>) -> PackGraphics {
        PackGraphics { assets, ..self }
    }

    /// The game's pack's numbers for the content's assets.
    pub fn assets(&self) -> &Arc<AssetNumbers> {
        &self.assets
    }

    /// The packs to draw from.
    pub fn packs(&self) -> Packs<'_> {
        Packs { bundles: self.bundles.iter().map(|b| &**b).collect(), own: self.own, others: &self.others, assets: &self.assets, version: None }
    }
}

/// No other game's backgrounds.
static NO_OTHERS: std::sync::LazyLock<OtherBackgrounds> = std::sync::LazyLock::new(OtherBackgrounds::default);

/// No asset's number.
static NO_ASSETS: std::sync::LazyLock<AssetNumbers> = std::sync::LazyLock::new(AssetNumbers::default);

/// The loaded packs' graphics, by `PackId` (the content's `AssetNames::packs`
/// order), and the match's game's pack's (the frontend's own), which draws
/// what belongs to no asset (the HUD's frame, the custom screen's).
#[derive(Clone)]
pub struct Packs<'a> {
    bundles: Vec<&'a Bundle>,
    own: PackId,
    /// Other games' backgrounds (the background pool's).
    others: &'a OtherBackgrounds,
    /// The game's pack's numbers for the content's assets.
    assets: &'a AssetNumbers,
    /// The console's version (`Renderer::console_version`): a chip of
    /// another version's shows that version's ROM's art, which this console
    /// draws otherwise (`ChipArt::version`).
    version: Option<&'static str>,
}

impl<'a> Packs<'a> {
    /// One pack's graphics: every asset is its.
    pub fn one(bundle: &'a Bundle) -> Packs<'a> {
        Packs { bundles: vec![bundle], own: PackId(0), others: &NO_OTHERS, assets: &NO_ASSETS, version: None }
    }

    /// Several packs' graphics, by `PackId`; `own` the frontend's own
    /// game's.
    pub fn new(bundles: Vec<&'a Bundle>, own: PackId) -> Packs<'a> {
        assert!(own.index() < bundles.len(), "the frontend's own pack is loaded");
        Packs { bundles, own, others: &NO_OTHERS, assets: &NO_ASSETS, version: None }
    }

    /// These packs, with other games' backgrounds beside them.
    pub fn with_others(self, others: &'a OtherBackgrounds) -> Packs<'a> {
        Packs { others, ..self }
    }

    /// These packs, drawing the content's assets as the game's pack numbers
    /// them.
    pub fn with_assets(self, assets: &'a AssetNumbers) -> Packs<'a> {
        Packs { assets, ..self }
    }

    /// The game's pack's numbers for the content's assets.
    pub fn assets(&self) -> &'a AssetNumbers {
        self.assets
    }

    /// How many packs are loaded.
    pub fn loaded(&self) -> usize {
        self.bundles.len()
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

    /// The game's pack: the own pack (a match plays the frontend's game).
    pub fn game_id(&self, _c: &Content) -> PackId {
        self.own
    }

    /// Draw a console of `version` (`Renderer::console_version`; None: the
    /// engine's).
    pub fn set_version(&mut self, version: Option<&'static str>) {
        self.version = version;
    }

    /// The console's version, when the frontend says it (`set_version`).
    pub fn version(&self) -> Option<&'static str> {
        self.version
    }

    /// A chip's icon: its game's pack's, under its key there.
    pub fn chip_icon(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::Tiles> {
        self.game(c).hud.chip_icon(nettai_content_api::keys::local(key))
    }

    /// A chip's picture: its game's pack's, under its key there.
    pub fn chip_art(&self, c: &Content, key: &str) -> Option<&'a nettai_assets::ChipArt> {
        self.game(c).custom.chip_art(nettai_content_api::keys::local(key))
    }

    /// Sprite `id`'s sheet, from the game's pack.
    pub fn sprite(&self, c: &Content, id: SpriteId) -> Option<&'a SpriteSheet> {
        let s = self.assets.sprite(id.0)?;
        self.game(c).sprite(s.category, s.index)
    }

    /// Banner `id`'s pack's HUD and its number there.
    pub fn banner(&self, c: &Content, id: BannerId) -> Option<(&'a Hud, u8)> {
        let n = self.assets.number(AssetKind::Banner, id.0)?;
        Some((&self.game(c).hud, n as u8))
    }

    /// Mugshot `id`'s pack and its number there.
    pub fn mugshot_of(&self, id: MugshotId) -> Option<InPack<u8>> {
        let n = self.assets.number(AssetKind::Mugshot, id.0)?;
        Some(InPack { pack: self.own, id: n as u8 })
    }

    /// Mugshot `m`'s pack's HUD and its number there.
    pub fn mugshot(&self, m: InPack<u8>) -> (&'a Hud, u8) {
        (&self.bundle(m.pack).hud, m.id)
    }

    /// Background `id`: one of the game's own from its pack, by the number
    /// its asset table gives it; one of the background pool's others from
    /// its own game's pack, by its name there ([`OtherBackgrounds`]). None:
    /// its pack hasn't it, or isn't here.
    pub fn background(&self, c: &Content, id: BackgroundId) -> Option<&'a Background> {
        if !c.is_own_background(id) {
            let (game, name) = c.background_of(id)?;
            return self.others.get(game, name);
        }
        let n = self.assets.number(AssetKind::Background, id.0)?;
        self.game(c).background(n as u8)
    }

    /// The background a round of `settings` is drawn on: its own
    /// (`settings.background`), or, where that is another game's whose pack
    /// isn't here (a player without that game's pack: [`OtherBackgrounds`]),
    /// its stage's own, the background the game itself draws the stage on.
    pub fn round_background(&self, c: &Content, settings: &BattleSettings) -> BackgroundId {
        let id = settings.background;
        match c.is_own_background(id) || self.background(c, id).is_some() {
            true => id,
            false => c.stage(settings.stage).background,
        }
    }
}

/// The name content gives `kind` asset `h` (`bomb`), for a problem's
/// text; a background of the pool's others, its game and its name
/// (`exe4:acdc-area`).
pub fn name(c: &Content, kind: AssetKind, h: u16) -> Option<&str> {
    match kind {
        AssetKind::Background if !c.is_own_background(BackgroundId(h)) => c.other_background(BackgroundId(h)),
        _ => c.assets.name(kind, h),
    }
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
        let assets = testing::asset_numbers(&c);
        let packs = Packs::one(&a).with_assets(&assets);
        let tiles = |s: Option<&SpriteSheet>| s.expect("a sheet").tilesets[0].pixels.len() / Tiles::TILE;
        assert_eq!(tiles(packs.sprite(&c, testing::sprite_named(&c, "test-navi"))), 1);
        assert!(std::ptr::eq(packs.game(&c), &a));
        assert!(std::ptr::eq(packs.own(), &a));
        // A chip's icon by its key in its pack ("gundels3", not
        // "gundels3").
        assert_eq!(packs.chip_icon(&c, &chip).map(|t| t.pixels[0]), Some(1));
    }

    /// A round drawn on another game's background (the background pool's) draws it from that game's pack's
    /// backgrounds beside the packs; a round whose other background's pack isn't here is drawn on its stage's
    /// own; the game's own backgrounds draw from its pack as ever.
    #[test]
    fn another_games_background_draws_from_its_own_pack() {
        let mut c = content();
        let stage = c.stage_by_key(testing::LINK_BATTLE);
        let own = c.stage(stage).background;
        let assets = testing::asset_numbers(&c);
        let number = assets.number(AssetKind::Background, own.0).expect("the stage's background's number") as usize;
        c.background_pool = nettai_battle::content::BackgroundPool::new([("zzz".to_string(), "far-area".to_string()), ("zzz".to_string(), "gone".to_string())]);
        let far = c.background_handle("zzz", "far-area").unwrap();
        let gone = c.background_handle("zzz", "gone").unwrap();
        let picture = |first_tile: u16| Background { first_tile, ..Background::default() };
        let mut a = bundle(1, "none");
        a.backgrounds = vec![None; number + 1];
        a.backgrounds[number] = Some(picture(7));
        let mut others = OtherBackgrounds::default();
        others.insert("zzz", [("far-area".to_string(), picture(9))].into());
        let packs = Packs::one(&a).with_others(&others).with_assets(&assets);
        assert_eq!(packs.background(&c, own).map(|b| b.first_tile), Some(7), "the game's own, from its pack");
        assert_eq!(packs.background(&c, far).map(|b| b.first_tile), Some(9), "another game's, from its pack");
        assert_eq!(packs.background(&c, gone), None, "its pack hasn't it");
        let settings = |background| BattleSettings { stage, background, effects: 0 };
        assert_eq!(packs.round_background(&c, &settings(far)), far);
        assert_eq!(packs.round_background(&c, &settings(gone)), own, "the stage's own");
        assert_eq!(packs.round_background(&c, &settings(own)), own);
        assert_eq!(name(&c, AssetKind::Background, far.0), Some("zzz:far-area"));
        assert_eq!(Packs::one(&a).round_background(&c, &settings(far)), own, "no other game's pack here");
    }
}
