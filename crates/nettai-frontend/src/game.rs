//! Loading a game to play: its content, its assets extracted from its ROMs,
//! its graphics in a language, the text's font and its sound.
//!
//! [`load`] is the whole of it in one call. Its steps are here too, for a
//! host that wants one at a time (the desktop program loads no sound for
//! frames it only writes): [`Game::load`], [`Game::graphics`], [`font`],
//! [`Game::sound`]; [`Loading`] is [`load`] a part at a time.
//!
//! A game's assets are extracted from its ROMs as it loads (`nettai_extract`:
//! the game's recipe, which its content's root gives and which loads with
//! the content, run on the ROMs a [`RomSource`] has), in memory; no asset is
//! read from a file of its own. If the ROMs can't be read, the game can't
//! load, and the error says why. A host that must know a game's ROMs before
//! it loads the rest reads its content first ([`read_content`]: the recipe
//! states them) and goes on from it ([`Loading::with_content`]).
//!
//! Nothing here prints or exits. Every step gives a `Result`; what the
//! loaders said on the way (their warnings, and the errors that say why a
//! step failed) comes back as a [`Report`] for the host to show, in what
//! was loaded and in the [`LoadError`].

use nettai_assets::{Background, Bundle};
use nettai_battle::Content;
use nettai_battle::content::AssetNumbers;
use nettai_content::locale::{self, Strings};
pub use nettai_content::report::Report;
use nettai_content_api::AssetIndex;
pub use nettai_extract::RomSource;
use nettai_extract::{Extraction, Recipe};
use nettai_render::Renderer;
use nettai_render::graphics::{GameGraphics, OtherBackgrounds, OtherGame};
pub use nettai_render::textlayer::GraphicsMode;
pub use nettai_render::vfont::VectorFont;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A step of loading that failed.
#[derive(Debug)]
pub struct LoadError {
    pub what: Failed,
    /// What the loaders reported up to there: the warnings before it, and
    /// the errors that say why in detail.
    pub report: Report,
}

/// What couldn't be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failed {
    /// The game's battle content.
    Content { game: String },
    /// The game's assets: its ROMs couldn't be found, read or extracted, or
    /// what they extract to hasn't what the content names.
    Assets { game: String },
    /// The language: the content has no strings in it. The reason.
    Language(String),
    /// The text's font. The reason.
    Font(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.what {
            Failed::Content { game } => write!(f, "can't load {game}'s battle content"),
            Failed::Assets { game } => write!(f, "can't extract {game}'s assets from its ROMs"),
            Failed::Language(why) | Failed::Font(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// A game's battle content, and its assets extracted from its ROMs.
#[derive(Clone)]
pub struct Game {
    /// The game (`exe6`).
    pub name: String,
    pub content: Arc<Content>,
    /// The content directory it came from (its strings' tables are a
    /// host's other languages).
    pub dir: PathBuf,
    /// The game's graphics, extracted from its ROMs, with the lettering of
    /// every language they have ([`Game::graphics`] takes one).
    pub graphics: Arc<Bundle>,
    /// Its sound, extracted from its ROMs.
    pub sound: Arc<m4a::SoundBank>,
    /// What its assets hold of the content's: each handle's number there
    /// (checked against the content as it loaded:
    /// `nettai_content::load::asset_numbers`).
    pub numbers: Arc<AssetNumbers>,
    /// Its assets' index: every asset it has, by its name (what a host
    /// plays that no content names: the app's title music).
    pub index: Arc<AssetIndex>,
    /// The background pool's backgrounds past the game's own asset table
    /// (docs/app.md §1, "The battle's background": `Content::background_pool`):
    /// the game's own, extracted with it, and each other game's, extracted
    /// from its own ROMs the first time one of them is wanted ([`others`]).
    pub others: Arc<OtherBackgrounds>,
    /// Where the content times its sprites otherwise than the ROMs
    /// (`nettai_extract::Extraction::timing_differences`), each named:
    /// none, and the content's timing is the ROMs'. A battle is timed by
    /// the content's whatever this says.
    pub timing_differences: Vec<String>,
    pub report: Report,
}

/// A game's battle content, defined, with its extraction recipe, before its assets are extracted ([`Loading`]'s
/// first part, [`read_content`]).
pub struct ContentLoaded {
    name: String,
    content: Content,
    /// Its recipe, which loaded with it (`nettai_content::load::recipe`); why there is none.
    recipe: Result<Arc<Recipe>, String>,
    dir: PathBuf,
    /// The games of the background pool, each with the content directory it is loaded from.
    pool: Vec<(String, PathBuf)>,
    report: Report,
}

impl ContentLoaded {
    /// The game's recipe, which states its ROMs and extracts them; why it has none that reads.
    pub fn recipe(&self) -> Result<&Arc<Recipe>, &str> {
        self.recipe.as_ref().map_err(String::as_str)
    }

    /// The background pool it was read with: the games, each with its content directory.
    pub fn pool(&self) -> &[(String, PathBuf)] {
        &self.pool
    }
}

/// Game `name`'s content, from the content directory `content` (none: the one the program comes with), defined, with
/// its recipe ([`Loading`]'s first part, for a host that must know the game's ROMs before it loads the rest:
/// [`Loading::with_content`]). Its background pool is the backgrounds of the games of `pool`, each read from its own
/// content directory (a host whose games each have content of their own: its store's bundles); none given, the
/// content directory's own games'.
pub fn read_content(content: Option<&Path>, name: &str, pool: &[(String, PathBuf)]) -> Result<ContentLoaded, LoadError> {
    let dir = content.map(Path::to_path_buf).unwrap_or_else(nettai_content::index::content);
    let failed = |report: Report| LoadError { what: Failed::Content { game: name.to_string() }, report };
    let mut report = Report::default();
    let games = match nettai_content::index::games(&dir) {
        Ok(g) if g.iter().any(|x| x == name) => g,
        Ok(g) => {
            report.error(dir.display().to_string(), format!("no game {name} in the content (its games: {})", g.join(", ")));
            return Err(failed(report));
        }
        Err(e) => {
            report.error(dir.display().to_string(), e);
            return Err(failed(report));
        }
    };
    let (mut c, report) = nettai_content::load::battle_content(&dir, name).map_err(failed)?;
    let pool: Vec<(String, PathBuf)> = if pool.is_empty() { games.into_iter().map(|g| (g, dir.clone())).collect() } else { pool.to_vec() };
    if !pool.is_empty() {
        c.background_pool = locale::background_pool_of(&pool);
    }
    let mut report = report;
    if let Err(e) = c.define() {
        report.error(dir.display().to_string(), e.message);
        return Err(failed(report));
    }
    let recipe = nettai_content::load::recipe(&c).map(Arc::new);
    Ok(ContentLoaded { name: name.to_string(), content: c, recipe, dir, pool, report })
}

/// Other games' backgrounds as this run has them, by game, the content directory it is loaded from and the ROMs (as
/// they say themselves): each game's extracted once a run, the first time a game loaded wants one ([`others`]).
type OtherGames = BTreeMap<(String, PathBuf, String), Arc<OtherGame>>;
static OTHER_GAMES: std::sync::LazyLock<std::sync::Mutex<OtherGames>> = std::sync::LazyLock::new(Default::default);

/// Game `game`'s backgrounds, by the recipe of its content in directory `dir` (the game loaded for it: its recipe
/// loads with its content), its ROMs `roms`'s: this run's, else to be extracted when first wanted (and kept for the
/// run).
pub fn others(game: &str, dir: &Path, roms: &RomSource) -> Arc<OtherGame> {
    let key = (game.to_string(), dir.to_path_buf(), format!("{roms:?}"));
    let mut games = nettai_threads::lock(&OTHER_GAMES).unwrap_or_else(|poisoned| poisoned.into_inner());
    let (game, dir, roms) = (game.to_string(), dir.to_path_buf(), roms.clone());
    games
        .entry(key)
        .or_insert_with(|| {
            OtherGame::new(Box::new(move || {
                let extracted = nettai_content::load::extract(&dir, &game, &roms);
                extracted.map(|(_, e)| backgrounds_of(&e)).map_err(|e| {
                    log::warn!("the background pool's {game} backgrounds aren't drawn: {e}");
                    e
                })
            }))
        })
        .clone()
}

/// Every background extracted, by its name in the assets' index.
fn backgrounds_of(e: &Extraction) -> BTreeMap<String, Background> {
    e.index.backgrounds.iter().filter_map(|(name, &n)| Some((name.clone(), e.graphics.background(n)?.clone()))).collect()
}

impl ContentLoaded {
    /// The game, its assets extracted from `roms` and checked against its content; the background pool's other
    /// games' to be extracted when first wanted ([`others`]).
    fn extract(self, roms: &RomSource) -> Result<Game, LoadError> {
        let ContentLoaded { name, content, recipe, dir, pool, mut report } = self;
        let failed = |report: Report| LoadError { what: Failed::Assets { game: name.clone() }, report };
        let extracted = recipe.and_then(|recipe| recipe.extract(&roms.read(&recipe).map_err(|e| e.to_string())?).map_err(|e| e.to_string()));
        let own = match extracted {
            Ok(own) => own,
            Err(e) => {
                report.error(format!("{name}'s ROMs"), e.to_string());
                return Err(failed(report));
            }
        };
        for warning in &own.warnings {
            report.note(format!("{name}'s assets"), warning.clone());
        }
        let numbers = match nettai_content::load::asset_numbers(&content, &own.index, &own.graphics) {
            Ok((numbers, r)) => {
                report.issues.extend(r.issues);
                numbers
            }
            Err(r) => {
                report.issues.extend(r.issues);
                return Err(failed(report));
            }
        };
        let timing_differences = own.timing_differences(&content.assets, &content.animations.sprites, &numbers);
        // The background pool's: the game's own, extracted now (and kept for the run: another game's pool has them
        // too), and each other game's, when first wanted.
        let mut others = OtherBackgrounds::default();
        let own_backgrounds = self::others(&name, &dir, roms);
        own_backgrounds.provide(backgrounds_of(&own));
        others.insert(&name, own_backgrounds);
        for (game, from) in &pool {
            if *game != name && content.background_pool.iter().any(|(g, _)| g == game) {
                others.insert(game, self::others(game, from, roms));
            }
        }
        let Extraction { graphics, sound, index, .. } = own;
        Ok(Game {
            name,
            content: Arc::new(content),
            dir,
            graphics: Arc::new(graphics),
            sound: Arc::new(sound),
            numbers: Arc::new(numbers),
            index: Arc::new(index),
            others: Arc::new(others),
            timing_differences,
            report,
        })
    }
}

impl Game {
    /// Game `name`'s content, from the content directory `content` (none:
    /// the one the program comes with), and its assets extracted from the
    /// ROMs of `roms`, checked against it. Its background pool is the
    /// backgrounds of the games of `pool`, each read from its own content
    /// directory (a host whose games each have content of their own: its
    /// store's bundles), each other game's extracted from its own ROMs when
    /// a round or a picture first wants one ([`others`]); none given, the
    /// content directory's own games'.
    pub fn load(roms: &RomSource, content: Option<&Path>, name: &str, pool: &[(String, PathBuf)]) -> Result<Game, LoadError> {
        read_content(content, name, pool)?.extract(roms)
    }

    /// The game's graphics in `lang`: their lettering in it (fonts, HUD
    /// lines, pictures with words) and the content's strings table, if the
    /// language isn't the content's own (`nettai_content::locale::OWN`).
    /// Graphics that have no lettering in `lang` (Chinese: every game's
    /// have English's and Japanese's) are drawn in their own language:
    /// their pictures with words are the ones they have, and the strings
    /// are still `lang`'s, which the renderer draws with the vector font in
    /// the enhanced mode and pixel fonts in the original
    /// (`Renderer::set_mode`, `pixeltext`). With them, the background
    /// pool's others ([`Game::others`]).
    pub fn graphics(&self, lang: &str) -> Result<Graphics, LoadError> {
        let report = Report::default();
        let failed = |why: String, report: Report| LoadError { what: Failed::Language(why), report };
        let mut strings = None;
        if lang != locale::OWN {
            strings = match locale::load_for(&self.dir, std::slice::from_ref(&self.name), lang) {
                Ok(Some(s)) => Some(Arc::new(s)),
                Ok(None) => {
                    let have = locale::languages(&self.dir);
                    return Err(failed(format!("the content has no strings in {lang:?} (it has {})", have.join(", ")), report));
                }
                Err(e) => return Err(failed(e, report)),
            };
        }
        let bundle = Arc::new((*self.graphics).clone().for_language(lang));
        Ok(Graphics { bundle, strings, others: self.others.clone(), numbers: self.numbers.clone(), report })
    }

    /// The game's sound: its bank, and the content's songs.
    pub fn sound(&self) -> Result<Sound, LoadError> {
        Ok(Sound { bank: self.sound.clone(), songs: nettai_audio::Songs::of(&self.numbers), report: Report::default() })
    }
}

/// A game's graphics, in a language. They are shared: every renderer made of
/// them draws from the same, and keeps them for as long as it lives.
#[derive(Clone)]
pub struct Graphics {
    /// The game's, in the language.
    pub bundle: Arc<Bundle>,
    /// The background pool's backgrounds of other games, from their ROMs.
    pub others: Arc<OtherBackgrounds>,
    /// The content's strings in the language (none: the content's own).
    pub strings: Option<Arc<Strings>>,
    /// What the game's assets hold of the content's.
    pub numbers: Arc<AssetNumbers>,
    pub report: Report,
}

impl Graphics {
    /// The graphics, as a renderer holds them.
    pub fn game_graphics(&self) -> GameGraphics {
        GameGraphics::new(self.bundle.clone()).with_others(self.others.clone()).with_numbers(self.numbers.clone())
    }

    /// A renderer of these graphics, its text drawn in `text` mode
    /// (`font`: the enhanced mode's, [`font`]).
    pub fn renderer(&self, mode: GraphicsMode, font: Option<Arc<VectorFont>>) -> Renderer {
        let mut renderer = Renderer::with_graphics(self.game_graphics());
        renderer.set_strings(self.strings.clone());
        renderer.set_mode(mode, font);
        renderer
    }
}

/// A game's sound: its bank, and the content's songs in it.
pub struct Sound {
    pub bank: Arc<m4a::SoundBank>,
    pub songs: nettai_audio::Songs,
    pub report: Report,
}

impl Sound {
    /// The parts of a battle's sound (as [`crate::preload::Preload::with`]
    /// takes them) of `content`, this sound's game, with `theme` as the
    /// battle's music in place of the game's own (the songs a round starts:
    /// `Content::battle_music`); with none, the game's own. Presentation
    /// alone: the battle, its cues, a replay and the wire are the same
    /// whatever plays. Its rollback is the cues', as the game's own music's.
    pub fn parts(&self, content: &Content, theme: Option<&nettai_audio::Theme>) -> crate::preload::SoundParts {
        match theme {
            Some(theme) => nettai_audio::with_theme(vec![self.bank.clone()], &self.songs, &content.battle_music(), theme),
            None => (vec![self.bank.clone()], self.songs.clone()),
        }
    }
}

/// The font the `text` mode draws with: the file at `path`, else the one
/// the library comes with, its cut of `lang` (Noto Sans CJK's: `vfont::Cut`);
/// none in the original mode, which draws the game's own fonts.
pub fn font(mode: GraphicsMode, path: Option<&Path>, lang: &str) -> Result<Option<Arc<VectorFont>>, LoadError> {
    if mode != GraphicsMode::Enhanced {
        return Ok(None);
    }
    match path {
        Some(path) => match VectorFont::load(path) {
            Ok(f) => Ok(Some(Arc::new(f))),
            Err(e) => Err(LoadError { what: Failed::Font(e), report: Report::default() }),
        },
        None => Ok(Some(nettai_render::vfont::bundled_font(nettai_render::vfont::Cut::of(lang)).clone())),
    }
}

/// What [`load`] loads.
#[derive(Clone, Debug)]
pub struct Options {
    /// Where the games' ROMs are read from (none: [`RomSource::from_env`],
    /// `$NETTAI_ROMS`, else the Legacy Collection where Steam installed it).
    pub roms: Option<RomSource>,
    /// The content directory (none: the one the program comes with).
    pub content: Option<PathBuf>,
    /// The games whose backgrounds the background pool has, each with its
    /// content directory ([`Game::load`]); none: the content directory's.
    pub pool: Vec<(String, PathBuf)>,
    /// The language shown.
    pub lang: String,
    /// How the battle is drawn, and the enhanced mode's font file (none: the
    /// library's own).
    pub mode: GraphicsMode,
    pub font: Option<PathBuf>,
    /// The enhanced mode without its motion (`Renderer::reduce_motion`).
    pub reduce_motion: bool,
    /// Load the sound too (a host that plays none needn't).
    pub sound: bool,
}

impl Default for Options {
    /// The ROMs the tools read ([`RomSource::from_env`]), the content the
    /// program comes with, in its own language, the enhanced mode with the
    /// library's font, with sound.
    fn default() -> Options {
        Options { roms: None, content: None, pool: Vec::new(), lang: locale::OWN.to_string(), mode: GraphicsMode::Enhanced, font: None, reduce_motion: false, sound: true }
    }
}

/// A game, loaded to play ([`load`]).
pub struct Loaded {
    pub game: Game,
    pub graphics: Graphics,
    pub mode: GraphicsMode,
    /// The enhanced mode without its motion (`Renderer::reduce_motion`).
    pub reduce_motion: bool,
    pub font: Option<Arc<VectorFont>>,
    pub sound: Option<Sound>,
}

impl Loaded {
    /// The enhanced mode's font for text in `lang`: the bundled font's cut of
    /// it (Noto Sans CJK's: the Chinese have their own hanzi), or a font file
    /// of the player's as it is; none where none was loaded.
    pub fn font_in(&self, lang: &str) -> Option<Arc<VectorFont>> {
        let font = self.font.as_ref()?;
        Some(if font.is_bundled() { nettai_render::vfont::bundled_font(nettai_render::vfont::Cut::of(lang)).clone() } else { font.clone() })
    }

    /// A renderer of the game's graphics, the text as it was asked for.
    pub fn renderer(&self) -> Renderer {
        let mut renderer = self.graphics.renderer(self.mode, self.font.clone());
        renderer.reduce_motion = self.reduce_motion;
        renderer
    }

    /// The game's battle themes (its rules' `battle_themes`), by their songs'
    /// asset names, in its order: what a player may pick as a battle's music,
    /// in this game's battles or another's ([`Loaded::theme`]).
    pub fn battle_themes(&self) -> Vec<&str> {
        let names = self.game.content.assets.names(nettai_content_api::AssetKind::Sound);
        self.game.content.battle_themes().iter().filter_map(|t| names.get(t.0 as usize).copied()).collect()
    }

    /// Battle theme `song` (a song asset's name, one of
    /// [`Loaded::battle_themes`]): its song with its game's bank, to play as
    /// a battle's music in place of its own ([`Sound::parts`]). None where
    /// the game has no such theme, or no sound loaded.
    pub fn theme(&self, song: &str) -> Option<nettai_audio::Theme> {
        let assets = &self.game.content.assets;
        let handle = assets.handle(nettai_content_api::AssetKind::Sound, song)?;
        if !self.game.content.battle_themes().contains(&nettai_battle::sound::SoundId(handle)) {
            return None;
        }
        let song = self.game.numbers.sound(handle)?;
        let bank = self.sound.as_ref()?.bank.clone();
        Some(nettai_audio::Theme { bank, song })
    }

    /// Everything the loaders reported, in the order they loaded.
    pub fn report(&self) -> Report {
        let mut all = Report::default();
        let reports = [Some(&self.game.report), Some(&self.graphics.report), self.sound.as_ref().map(|s| &s.report)];
        for r in reports.into_iter().flatten() {
            all.issues.extend(r.issues.iter().cloned());
        }
        all
    }
}

/// Load game `name` to play it: its content, its assets extracted from its
/// ROMs, its graphics and strings in the language, the text's font and
/// (if asked) its sound. A failure carries what every step reported up to
/// it. ([`Loading`] is the same a step at a time, for a host that shows how
/// far it is.)
pub fn load(name: &str, options: &Options) -> Result<Loaded, LoadError> {
    let mut loading = Loading::new(name, options.clone());
    while loading.step()? {}
    Ok(loading.finish().expect("every step ran"))
}

/// A part of a game's loading: a step of [`Loading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The game's battle content: its Luau modules read and defined.
    Content,
    /// Its assets, extracted from its ROMs, checked against the content.
    Assets,
    /// The graphics in the language, and the content's strings in it.
    Graphics,
    /// The text's font.
    Font,
    /// The sound.
    Sound,
}

impl Part {
    /// Every part, in the order they load.
    pub const ALL: [Part; 5] = [Part::Content, Part::Assets, Part::Graphics, Part::Font, Part::Sound];

    /// The share of a game's loading the part takes, out of 100: what
    /// [`Loading::fraction`] counts a part done as. The content is more than
    /// half of every game's (it runs the game's Luau); extracting the ROMs,
    /// about a tenth of a second, most of the rest; the graphics in a
    /// language and the sound are copies and lists.
    pub fn share(self) -> u32 {
        match self {
            Part::Content => 62,
            Part::Assets => 30,
            Part::Graphics => 5,
            Part::Font => 1,
            Part::Sound => 2,
        }
    }
}

/// A game being loaded, a part at a time ([`Part::ALL`]): everything a
/// battle of it reads, which is all read here and none of it during a match
/// (docs/frontend.md §9). A host that shows a loading bar runs
/// [`Loading::step`] until it says there is no more, on a thread of its own
/// or between its frames, shows [`Loading::fraction`] meanwhile, and takes
/// the game with [`Loading::finish`]. Each step runs one part to its end:
/// the content's is the longest (seconds; it runs the game's Luau), so a
/// host with threads steps this on one and its window stays live.
///
/// What is left for the thread that will run the battles is that thread's
/// own: `crate::preload`.
pub struct Loading {
    name: String,
    options: Options,
    /// How many of [`Loading::parts`] are loaded.
    done: usize,
    content: Option<ContentLoaded>,
    game: Option<Game>,
    graphics: Option<Graphics>,
    font: Option<Arc<VectorFont>>,
    sound: Option<Sound>,
}

impl Loading {
    /// Game `name`, to load as `options` say; nothing is read yet.
    pub fn new(name: &str, options: Options) -> Loading {
        Loading { name: name.to_string(), options, done: 0, content: None, game: None, graphics: None, font: None, sound: None }
    }

    /// The game whose content `content` is, read before ([`read_content`]: `options`' content directory and
    /// background pool are the ones it was read with), to load the rest of as `options` say: its first part is done.
    pub fn with_content(content: ContentLoaded, options: Options) -> Loading {
        let name = content.name.clone();
        Loading { done: 1, content: Some(content), ..Loading::new(&name, options) }
    }

    /// The parts this loads, in order (the sound only if asked for).
    pub fn parts(&self) -> impl Iterator<Item = Part> + '_ {
        Part::ALL.into_iter().filter(|&p| p != Part::Sound || self.options.sound)
    }

    /// The part the next step loads; none once all are.
    pub fn next(&self) -> Option<Part> {
        self.parts().nth(self.done)
    }

    /// How far the loading is, from 0 to 1: the shares of the parts loaded
    /// ([`Part::share`]) over all its parts'.
    pub fn fraction(&self) -> f32 {
        let all: u32 = self.parts().map(Part::share).sum();
        let done: u32 = self.parts().take(self.done).map(Part::share).sum();
        done as f32 / all as f32
    }

    /// Load the next part. True while a part is left after it; a failure
    /// carries what every part reported up to it, and the loading can't go
    /// on.
    pub fn step(&mut self) -> Result<bool, LoadError> {
        // (A later part's failure, with the earlier parts' reports before
        // its own.)
        fn after(before: &[Option<&Report>], mut e: LoadError) -> LoadError {
            let mut issues: Vec<_> = before.iter().flatten().flat_map(|r| r.issues.iter().cloned()).collect();
            issues.append(&mut e.report.issues);
            e.report.issues = issues;
            e
        }
        let Some(part) = self.next() else { return Ok(false) };
        let reports = [self.game.as_ref().map(|g| &g.report), self.graphics.as_ref().map(|g| &g.report)];
        let loaded = |what: &str| -> ! { unreachable!("the {what} loads before this part") };
        match part {
            Part::Content => self.content = Some(read_content(self.options.content.as_deref(), &self.name, &self.options.pool)?),
            Part::Assets => {
                let Some(content) = self.content.take() else { loaded("content") };
                let roms = match &self.options.roms {
                    Some(roms) => roms.clone(),
                    None => RomSource::from_env().map_err(|e| {
                        let mut report = content.report.clone();
                        report.error("ROMs", e.to_string());
                        LoadError { what: Failed::Assets { game: self.name.clone() }, report }
                    })?,
                };
                self.game = Some(content.extract(&roms)?);
            }
            Part::Graphics => {
                let Some(game) = &self.game else { loaded("assets") };
                let graphics = game.graphics(&self.options.lang).map_err(|e| after(&reports, e))?;
                self.graphics = Some(graphics);
            }
            Part::Font => self.font = font(self.options.mode, self.options.font.as_deref(), &self.options.lang).map_err(|e| after(&reports, e))?,
            Part::Sound => {
                let Some(game) = &self.game else { loaded("assets") };
                let sound = game.sound().map_err(|e| after(&reports, e))?;
                self.sound = Some(sound);
            }
        }
        self.done += 1;
        Ok(self.next().is_some())
    }

    /// The game, once every part is loaded; none before.
    pub fn finish(self) -> Option<Loaded> {
        if self.next().is_some() {
            return None;
        }
        Some(Loaded { game: self.game?, graphics: self.graphics?, mode: self.options.mode, reduce_motion: self.options.reduce_motion, font: self.font, sound: self.sound })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ROMs that aren't the game's: a folder of one ROM of another game (its header alone).
    fn no_roms() -> (tempfile::TempDir, RomSource) {
        let dir = tempfile::tempdir().unwrap();
        let mut other = vec![0u8; nettai_extract::HEADER_LEN];
        other[0xac..0xb0].copy_from_slice(b"AREE");
        std::fs::write(dir.path().join("other.gba"), other).unwrap();
        let roms = RomSource::at(dir.path()).unwrap();
        (dir, roms)
    }

    /// A game loads a part at a time, each counted by its share: the parts
    /// in their order, the sound only if asked for; a part that fails
    /// stops the loading there with what failed, and no game comes of it.
    #[test]
    fn a_game_loads_a_part_at_a_time() {
        let (_dir, roms) = no_roms();
        let options = Options { roms: Some(roms), ..Options::default() };
        let mut loading = Loading::new("exe6", options.clone());
        assert_eq!(loading.parts().collect::<Vec<_>>(), Part::ALL);
        assert_eq!(Part::ALL.map(Part::share).iter().sum::<u32>(), 100);
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Content), 0.0));
        // The content loads (the repository's).
        assert!(loading.step().unwrap());
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Assets), 0.62));
        // No ROMs of the game: its assets can't be extracted, and the loading
        // stays where it was.
        let Err(e) = loading.step() else { panic!("loaded exe6 with no ROMs") };
        assert_eq!(e.what, Failed::Assets { game: "exe6".into() });
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Assets), 0.62));
        assert!(loading.finish().is_none());
        // Without sound, the sound is no part and the others are the whole.
        let silent = Loading::new("exe6", Options { sound: false, ..options });
        assert_eq!(silent.parts().collect::<Vec<_>>(), [Part::Content, Part::Assets, Part::Graphics, Part::Font]);
        assert_eq!(silent.fraction(), 0.0);
    }

    /// A failure is a value that says what failed: with no ROMs of the
    /// game, its assets can't be extracted, and the report says which are
    /// missing; with no such game, its content can't load.
    #[test]
    fn a_game_with_no_roms_fails_as_a_value() {
        let (_dir, roms) = no_roms();
        let options = Options { roms: Some(roms), sound: false, ..Options::default() };
        let Err(e) = load("exe6", &options) else { panic!("loaded exe6 with no ROMs") };
        assert_eq!(e.what, Failed::Assets { game: "exe6".into() });
        assert_eq!(e.to_string(), "can't extract exe6's assets from its ROMs");
        assert!(e.report.has_errors() && e.report.to_string().contains("BR6E, BR5E, BR6J, BR5J missing from the ROM files in"), "{}", e.report);
        let Err(e) = load("exe9", &options) else { panic!("loaded a game that isn't there") };
        assert_eq!(e.what, Failed::Content { game: "exe9".into() });
        // A font file that isn't there.
        let dir = tempfile::tempdir().unwrap();
        let Err(e) = font(GraphicsMode::Enhanced, Some(&dir.path().join("no.ttf")), "en") else { panic!("read a font that isn't there") };
        assert!(matches!(&e.what, Failed::Font(why) if why.starts_with("can't read the font")), "{e}");
        assert!(font(GraphicsMode::Original, Some(&dir.path().join("no.ttf")), "en").unwrap().is_none());
    }
}
