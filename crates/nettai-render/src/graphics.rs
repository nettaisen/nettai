//! The engine's assets as the game's graphics hold them (docs/design/rules-in-luau.md
//! §7.4). The engine knows an asset by its handle over the names the
//! content gives; the frontend draws a sprite, banner, mugshot or
//! background by the game's own number for it ([`AssetNumbers`], which
//! the loader made when it checked the game's extracted assets against the
//! content), from the game's graphics ([`Graphics`]). A round may be drawn
//! on another game's background (the content's background pool: docs/app.md
//! §1, "The battle's background"), extracted from that game's ROMs: those are
//! [`OtherBackgrounds`], beside the game's graphics.

use nettai_assets::{Background, Bundle, Hud, SpriteSheet};
use nettai_battle::content::{AssetNumbers, BackgroundId, BannerId, Content, MugshotId, SpriteId};
use nettai_battle::setup::BattleSettings;
use nettai_content_api::AssetKind;
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// What makes one game's backgrounds, by name: the host's extraction of
/// that game's ROMs ([`OtherGame::new`]).
pub type Extract = Box<dyn FnOnce() -> Result<BTreeMap<String, Background>, String> + Send>;

/// One other game's backgrounds, by name: extracted from that game's ROMs
/// the first time they are wanted ([`OtherGame::want`]), on a thread of
/// their own, once; none until then. A host keeps them for its run and
/// shares them between the games it loads, so a game is extracted once.
pub struct OtherGame {
    /// The extraction, until it is started.
    extract: Mutex<Option<Extract>>,
    /// Its backgrounds once extracted, or why they couldn't be.
    got: OnceLock<Result<BTreeMap<String, Background>, String>>,
    /// Told when `got` is set.
    done: (Mutex<()>, Condvar),
}

impl OtherGame {
    /// A game's backgrounds, to be extracted by `extract` when first wanted.
    pub fn new(extract: Extract) -> Arc<OtherGame> {
        Arc::new(OtherGame { extract: Mutex::new(Some(extract)), got: OnceLock::new(), done: Default::default() })
    }

    /// A game's backgrounds, extracted already.
    pub fn extracted(backgrounds: BTreeMap<String, Background>) -> Arc<OtherGame> {
        let other = OtherGame { extract: Mutex::new(None), got: OnceLock::new(), done: Default::default() };
        let _ = other.got.set(Ok(backgrounds));
        Arc::new(other)
    }

    /// Its backgrounds, extracted already (the game's own extraction, which
    /// a host has as it loads the game): taken in place of extracting them,
    /// unless they are being extracted now.
    pub fn provide(&self, backgrounds: BTreeMap<String, Background>) {
        let Ok(mut extract) = nettai_threads::lock(&self.extract) else { return };
        if extract.take().is_some() {
            self.set(Ok(backgrounds));
        }
    }

    /// Start extracting its backgrounds, on a thread of their own, unless
    /// they are extracted or being extracted.
    pub fn want(self: &Arc<Self>) {
        let Some(extract) = nettai_threads::lock(&self.extract).ok().and_then(|mut e| e.take()) else { return };
        let other = self.clone();
        if let Err(e) = nettai_threads::spawn("other backgrounds", move || other.set(extract())) {
            self.set(Err(format!("can't start a thread to extract them: {e}")));
        }
    }

    fn set(&self, got: Result<BTreeMap<String, Background>, String>) {
        let _ = self.got.set(got);
        let _lock = nettai_threads::lock(&self.done.0);
        self.done.1.notify_all();
    }

    /// Whether its backgrounds are extracted, or couldn't be.
    pub fn ready(&self) -> bool {
        self.got.get().is_some()
    }

    /// Wait until [`OtherGame::ready`], having wanted them. Never on a
    /// page's main thread, which may not wait: a host there asks `ready`
    /// between its frames.
    pub fn wait(self: &Arc<Self>) {
        self.want();
        if self.ready() {
            return;
        }
        let Ok(mut lock) = self.done.0.lock() else { return };
        while !self.ready() {
            lock = match self.done.1.wait(lock) {
                Ok(l) => l,
                Err(_) => return,
            };
        }
    }

    /// Why its backgrounds couldn't be extracted, once that is known.
    pub fn failure(&self) -> Option<&str> {
        self.got.get()?.as_ref().err().map(String::as_str)
    }

    /// Background `name`, once extracted.
    pub fn get(&self, name: &str) -> Option<&Background> {
        self.got.get()?.as_ref().ok()?.get(name)
    }
}

/// Other games' backgrounds, by game: what a round drawn on one of the
/// background pool's others draws (`Content::other_background`), each game's
/// extracted from its own ROMs when first wanted ([`OtherGame`]). A host has
/// those of the games whose ROMs it has (nettai-frontend's `Game::graphics`);
/// a background of a game it hasn't, or not extracted (yet), isn't here, and
/// a round on it is drawn on its stage's own ([`Graphics::round_background`]).
#[derive(Clone, Default)]
pub struct OtherBackgrounds(BTreeMap<String, Arc<OtherGame>>);

impl OtherBackgrounds {
    /// Game `game`'s backgrounds.
    pub fn insert(&mut self, game: &str, other: Arc<OtherGame>) {
        self.0.insert(game.to_string(), other);
    }

    /// Game `game`'s background `name`, if it is here and extracted.
    pub fn get(&self, game: &str, name: &str) -> Option<&Background> {
        self.0.get(game)?.get(name)
    }

    /// Game `game`'s backgrounds, if it is here.
    pub fn game(&self, game: &str) -> Option<&Arc<OtherGame>> {
        self.0.get(game)
    }
}

/// The other games' backgrounds among `ids`, backgrounds of content `c`: each
/// game once.
fn other_games<'c>(c: &'c Content, ids: impl IntoIterator<Item = BackgroundId>) -> Vec<&'c str> {
    let mut games: Vec<&str> = ids.into_iter().filter(|&id| !c.is_own_background(id)).filter_map(|id| Some(c.background_of(id)?.0)).collect();
    games.sort_unstable();
    games.dedup();
    games
}

/// The backgrounds a set of rounds starting with `first` is drawn on: its
/// first round's and each one's after it (`RoundSetup::later_stages`).
pub fn set_backgrounds(first: &nettai_battle::setup::RoundSetup) -> Vec<BackgroundId> {
    std::iter::once(first.settings.background).chain(first.later_stages.iter().map(|s| s.background)).collect()
}

/// A game's graphics, held: shared, with the other games' backgrounds beside
/// them and the numbers the content's assets have in them. A renderer keeps
/// one, so it borrows from nobody (a host keeps a renderer wherever it keeps
/// its state, and several renderers share the same graphics); what a frame
/// draws from is a [`Graphics`] over them ([`GameGraphics::view`]).
#[derive(Clone)]
pub struct GameGraphics {
    bundle: Arc<Bundle>,
    others: Arc<OtherBackgrounds>,
    numbers: Arc<AssetNumbers>,
}

impl GameGraphics {
    /// The game's graphics `bundle`: every asset is its.
    pub fn new(bundle: Arc<Bundle>) -> GameGraphics {
        GameGraphics { bundle, others: Default::default(), numbers: Default::default() }
    }

    /// The drawings (`Bundle::drawings`), each once; the chatbox's
    /// coverages made now (`shapes::coverage`).
    pub fn drawings(&self) -> Arc<Vec<nettai_assets::shapes::Drawing>> {
        let mut seen = std::collections::HashSet::new();
        let all: Vec<nettai_assets::shapes::Drawing> =
            self.bundle.drawings().into_iter().filter(|d| d.hash != 0 && seen.insert(d.hash)).map(|d| nettai_assets::shapes::Drawing { words: Vec::new(), ..d.clone() }).collect();
        for d in self.bundle.hud.shapes.chatbox.iter().flatten().flat_map(|m| m.parts.iter().map(|p| &p.drawing)) {
            crate::shapes::coverage(d);
        }
        Arc::new(all)
    }

    /// These graphics, with other games' backgrounds beside them (the
    /// background pool's: [`OtherBackgrounds`]).
    pub fn with_others(self, others: Arc<OtherBackgrounds>) -> GameGraphics {
        GameGraphics { others, ..self }
    }

    /// These graphics, drawing the content's assets as the game's assets
    /// number them (`nettai_content::load::asset_numbers`).
    pub fn with_numbers(self, numbers: Arc<AssetNumbers>) -> GameGraphics {
        GameGraphics { numbers, ..self }
    }

    /// The game's numbers for the content's assets.
    pub fn numbers(&self) -> &Arc<AssetNumbers> {
        &self.numbers
    }

    /// The other games' backgrounds beside these graphics.
    pub fn others(&self) -> &Arc<OtherBackgrounds> {
        &self.others
    }

    /// Start extracting the other games' backgrounds among `ids` (backgrounds
    /// of content `c`), each game on a thread of its own ([`OtherGame::want`]).
    pub fn want_backgrounds(&self, c: &Content, ids: impl IntoIterator<Item = BackgroundId>) {
        for game in other_games(c, ids) {
            self.others.game(game).inspect(|o| o.want());
        }
    }

    /// Whether the other games' backgrounds among `ids` are extracted (or
    /// couldn't be, or their games aren't here): a round drawn on one draws
    /// what it will draw from now on.
    pub fn backgrounds_ready(&self, c: &Content, ids: impl IntoIterator<Item = BackgroundId>) -> bool {
        other_games(c, ids).into_iter().all(|game| self.others.game(game).is_none_or(|o| o.ready()))
    }

    /// Extract the other games' backgrounds among `ids` and wait for them
    /// ([`OtherGame::wait`]: never on a page's main thread).
    pub fn wait_backgrounds(&self, c: &Content, ids: impl IntoIterator<Item = BackgroundId> + Clone) {
        self.want_backgrounds(c, ids.clone());
        for game in other_games(c, ids) {
            self.others.game(game).inspect(|o| o.wait());
        }
    }

    /// What a frame draws from.
    pub fn view(&self) -> Graphics<'_> {
        Graphics { bundle: &self.bundle, others: &self.others, numbers: &self.numbers, version: None }
    }
}

/// No other game's backgrounds.
static NO_OTHERS: std::sync::LazyLock<OtherBackgrounds> = std::sync::LazyLock::new(OtherBackgrounds::default);

/// No asset's number.
static NO_NUMBERS: std::sync::LazyLock<AssetNumbers> = std::sync::LazyLock::new(AssetNumbers::default);

/// The game's graphics as a frame draws them: the match's game's (a match
/// plays one game, docs/design/content-model-v2.md §4.0), which draw its
/// assets and what belongs to no asset (the HUD's frame, the custom
/// screen's), with the other games' backgrounds beside them.
#[derive(Clone)]
pub struct Graphics<'a> {
    bundle: &'a Bundle,
    /// Other games' backgrounds (the background pool's).
    others: &'a OtherBackgrounds,
    /// The game's numbers for the content's assets.
    numbers: &'a AssetNumbers,
    /// The console's version (`Renderer::console_version`): a chip of
    /// another version's shows that version's ROM's art, which this console
    /// draws otherwise (`ChipArt::version`).
    version: Option<&'static str>,
}

impl<'a> Graphics<'a> {
    /// The graphics `bundle`: every asset is its.
    pub fn of(bundle: &'a Bundle) -> Graphics<'a> {
        Graphics { bundle, others: &NO_OTHERS, numbers: &NO_NUMBERS, version: None }
    }

    /// These graphics, with other games' backgrounds beside them.
    pub fn with_others(self, others: &'a OtherBackgrounds) -> Graphics<'a> {
        Graphics { others, ..self }
    }

    /// These graphics, drawing the content's assets as the game's assets
    /// number them.
    pub fn with_numbers(self, numbers: &'a AssetNumbers) -> Graphics<'a> {
        Graphics { numbers, ..self }
    }

    /// The game's numbers for the content's assets.
    pub fn numbers(&self) -> &'a AssetNumbers {
        self.numbers
    }

    /// The game's graphics: its field, its HUD, its custom screen, its
    /// chips' icons and pictures.
    pub fn bundle(&self) -> &'a Bundle {
        self.bundle
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

    /// A chip's icon, under its key in the game's graphics.
    pub fn chip_icon(&self, key: &str) -> Option<&'a nettai_assets::Tiles> {
        self.bundle.hud.chip_icon(nettai_content_api::keys::local(key))
    }

    /// A chip's picture, under its key in the game's graphics.
    pub fn chip_art(&self, key: &str) -> Option<&'a nettai_assets::ChipArt> {
        self.bundle.custom.chip_art(nettai_content_api::keys::local(key))
    }

    /// Sprite `id`'s sheet.
    pub fn sprite(&self, id: SpriteId) -> Option<&'a SpriteSheet> {
        let s = self.numbers.sprite(id.0)?;
        self.bundle.sprite(s.category, s.index)
    }

    /// Banner `id`'s HUD and its number there.
    pub fn banner(&self, id: BannerId) -> Option<(&'a Hud, u8)> {
        let n = self.numbers.number(AssetKind::Banner, id.0)?;
        Some((&self.bundle.hud, n as u8))
    }

    /// Mugshot `id`'s number in the game's HUD.
    pub fn mugshot(&self, id: MugshotId) -> Option<u8> {
        self.numbers.number(AssetKind::Mugshot, id.0).map(|n| n as u8)
    }

    /// Background `id`: one of the game's own, by the number its asset table
    /// gives it; one of the background pool's others, from its own game's,
    /// by its name there ([`OtherBackgrounds`]). None: its game's graphics
    /// haven't it, or aren't here.
    pub fn background(&self, c: &Content, id: BackgroundId) -> Option<&'a Background> {
        if !c.is_own_background(id) {
            let (game, name) = c.background_of(id)?;
            return self.others.get(game, name);
        }
        let n = self.numbers.number(AssetKind::Background, id.0)?;
        self.bundle.background(n as u8)
    }

    /// The background a round of `settings` is drawn on: its own
    /// (`settings.background`), or, where that is another game's that isn't
    /// here (a player without that game's ROMs: [`OtherBackgrounds`]), its
    /// stage's own, the background the game itself draws the stage on.
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

    /// A game's graphics, told apart by their navi sheet's tile count, with
    /// an icon for the chip `chip` (its key there).
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

    /// docs/design/rules-in-luau.md §7.4: an asset draws from the game's
    /// graphics by its number there, and a chip's icon is the game's under
    /// its key there.
    #[test]
    fn each_asset_draws_from_the_games_graphics() {
        let c = content();
        let chip = c.defs.chip(c.defs.chip_by_key("gundels3").expect("a test chip")).key.clone();
        let a = bundle(1, keys::local(&chip));
        let assets = testing::asset_numbers(&c);
        let graphics = Graphics::of(&a).with_numbers(&assets);
        let tiles = |s: Option<&SpriteSheet>| s.expect("a sheet").tilesets[0].pixels.len() / Tiles::TILE;
        assert_eq!(tiles(graphics.sprite(testing::sprite_named(&c, "test-navi"))), 1);
        assert!(std::ptr::eq(graphics.bundle(), &a));
        // A chip's icon by its key there (its local key).
        assert_eq!(graphics.chip_icon(&chip).map(|t| t.pixels[0]), Some(1));
    }

    /// A round drawn on another game's background (the background pool's) draws it from that game's
    /// backgrounds beside the game's graphics; a round whose other background isn't here is drawn on its stage's
    /// own; the game's own backgrounds draw from its graphics as ever.
    #[test]
    fn another_games_background_draws_from_its_own_game() {
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
        others.insert("zzz", OtherGame::extracted([("far-area".to_string(), picture(9))].into()));
        let graphics = Graphics::of(&a).with_others(&others).with_numbers(&assets);
        assert_eq!(graphics.background(&c, own).map(|b| b.first_tile), Some(7), "the game's own");
        assert_eq!(graphics.background(&c, far).map(|b| b.first_tile), Some(9), "another game's, from its own");
        assert_eq!(graphics.background(&c, gone), None, "its game hasn't it");
        let settings = |background| BattleSettings { stage, background, effects: 0 };
        assert_eq!(graphics.round_background(&c, &settings(far)), far);
        assert_eq!(graphics.round_background(&c, &settings(gone)), own, "the stage's own");
        assert_eq!(graphics.round_background(&c, &settings(own)), own);
        assert_eq!(name(&c, AssetKind::Background, far.0), Some("zzz:far-area"));
        assert_eq!(Graphics::of(&a).round_background(&c, &settings(far)), own, "no other game's here");
    }

    /// Another game's backgrounds are extracted the first time they are wanted, on a thread of their own, and
    /// once: wanted again, or by other graphics that share them, nothing is extracted again. Until then none is
    /// drawn (a round on one, its stage's own); a game's own given in their place is never extracted; a failure is
    /// ready too, and says why.
    #[test]
    fn another_games_backgrounds_are_extracted_once_when_wanted() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let mut c = content();
        let stage = c.stage_by_key(testing::LINK_BATTLE);
        let own = c.stage(stage).background;
        c.background_pool = nettai_battle::content::BackgroundPool::new([("zzz".to_string(), "far-area".to_string())]);
        let far = c.background_handle("zzz", "far-area").unwrap();
        let made = Arc::new(AtomicUsize::new(0));
        let counted = made.clone();
        let other = OtherGame::new(Box::new(move || {
            counted.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(20));
            Ok([("far-area".to_string(), Background { first_tile: 9, ..Background::default() })].into())
        }));
        let mut others = OtherBackgrounds::default();
        others.insert("zzz", other.clone());
        let held = GameGraphics::new(Arc::new(bundle(1, "none"))).with_others(Arc::new(others)).with_numbers(Arc::new(testing::asset_numbers(&c)));
        let shared = held.clone();
        let settings = BattleSettings { stage, background: far, effects: 0 };
        assert!(!held.backgrounds_ready(&c, [far]) && held.backgrounds_ready(&c, [own]), "not wanted yet; the game's own needs none");
        assert_eq!(held.view().round_background(&c, &settings), own, "not extracted: the stage's own");
        held.want_backgrounds(&c, [far]);
        shared.wait_backgrounds(&c, [far, own]);
        assert!(held.backgrounds_ready(&c, [far]));
        assert_eq!(held.view().background(&c, far).map(|b| b.first_tile), Some(9));
        assert_eq!(held.view().round_background(&c, &settings), far);
        held.wait_backgrounds(&c, [far]);
        assert_eq!(made.load(Ordering::SeqCst), 1, "extracted once");
        let given = OtherGame::new(Box::new(|| panic!("a game's own backgrounds given aren't extracted")));
        given.provide(BTreeMap::new());
        given.wait();
        assert!(given.ready() && given.failure().is_none());
        let failed = OtherGame::new(Box::new(|| Err("no ROMs".to_string())));
        failed.wait();
        assert_eq!(failed.failure(), Some("no ROMs"));
    }
}
