//! Loading a game to play: the packs found, the game's content, its
//! graphics in a language, the text's font and its sound.
//!
//! [`load`] is the whole of it in one call. Its steps are here too, for a
//! host that wants one at a time (the desktop program finds the packs
//! before it knows the game, and loads no sound for frames it only writes):
//! [`Found::find`], [`Game::load`], [`Game::graphics`], [`font`],
//! [`Game::sound`].
//!
//! Nothing here prints or exits. Every step gives a `Result`; what the
//! packs' loaders said on the way (their warnings, and the errors that say
//! why a step failed) comes back as a [`Report`] for the host to show, in
//! what was loaded and in the [`LoadError`].

use nettai_assets::Bundle;
use nettai_battle::Content;
use nettai_battle::content::{AssetNumbers, PackId};
use nettai_content::locale::{self, Strings};
use nettai_content::pack;
pub use nettai_content::report::Report;
use nettai_render::Renderer;
use nettai_render::packs::{OtherBackgrounds, PackGraphics};
pub use nettai_render::textlayer::GraphicsMode;
pub use nettai_render::vfont::VectorFont;
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
    /// The packs given by path can't be read.
    Packs,
    /// The game's battle content.
    Content { game: String },
    /// The content loaded names no pack of the game.
    /// A part of a pack: its `"graphics"`, its `"sound"`.
    Part { part: &'static str, pack: PathBuf },
    /// The language: the content has no strings in it, or the pack no
    /// lettering. The reason.
    Language(String),
    /// The text's font. The reason.
    Font(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.what {
            Failed::Packs => write!(f, "can't read the packs given"),
            Failed::Content { game } => write!(f, "can't load {game}'s battle content"),
            Failed::Part { part, pack } => {
                write!(f, "can't load the {part} of the content pack {} (extract it again: README.md, \"Getting started\")", pack.display())
            }
            Failed::Language(why) | Failed::Font(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// The packs found: every pack in a packs directory, each replaced by the
/// one of its game among those given by path.
#[derive(Clone, Debug)]
pub struct Found {
    /// The packs directory looked in (none: the packs given alone).
    pub dir: Option<PathBuf>,
    pub packs: Vec<pack::Found>,
    pub report: Report,
}

impl Found {
    /// The packs in `dir` (`nettai_content::pack::packs_dir` is where the
    /// desktop program looks) and those `given` by their directories.
    pub fn find(dir: &Path, given: &[PathBuf]) -> Result<Found, LoadError> {
        Found::find_in(Some(dir), given)
    }

    /// The packs in `dir`, if one is named, and those `given` by their
    /// directories (a host that names each pack it loads gives no `dir`).
    pub fn find_in(dir: Option<&Path>, given: &[PathBuf]) -> Result<Found, LoadError> {
        let mut report = Report::default();
        // (No folder to look in: a folder that has no pack.)
        let looked = dir.map_or_else(PathBuf::new, Path::to_path_buf);
        match pack::find(&looked, given, &mut report) {
            Some(packs) => Ok(Found { dir: dir.map(Path::to_path_buf), packs, report }),
            None => Err(LoadError { what: Failed::Packs, report }),
        }
    }
}

/// A game's battle content, loaded, and where its packs are.
#[derive(Clone, Debug)]
pub struct Game {
    /// The game (`exe6`).
    pub name: String,
    pub content: Arc<Content>,
    /// The content directory it came from (its strings' tables are a
    /// host's other languages).
    pub dir: PathBuf,
    /// The loaded asset packs' directories, by `PackId`.
    pub packs: Vec<PathBuf>,
    /// The game's own pack among them.
    pub own: PackId,
    /// What the game's pack holds of the content's assets (checked against
    /// the content as it loaded: `nettai_content::pack::pack_assets`).
    pub assets: Arc<AssetNumbers>,
    /// The game's pack's own index: every asset it has, by its name there.
    pub index: Arc<nettai_content_api::AssetIndex>,
    /// The other games' packs found beside its own, which the background
    /// pool's other backgrounds are drawn from ([`Game::graphics`]).
    pub others: Vec<pack::Found>,
    pub report: Report,
}

impl Game {
    /// Game `name`'s content, from the content directory `content` (none:
    /// the one the program comes with), drawn and heard from its pack
    /// among `found`. Its background pool is the backgrounds of the games
    /// of `pool`, each read from its own content directory (a host whose
    /// games each have content of their own: its store's bundles); none
    /// given, the content directory's own games'.
    pub fn load(found: &Found, content: Option<&Path>, name: &str, pool: &[(String, PathBuf)]) -> Result<Game, LoadError> {
        let mut loaded =
            pack::load_game(content, name, &found.packs).map_err(|report| LoadError { what: Failed::Content { game: name.to_string() }, report })?;
        if !pool.is_empty() {
            loaded.content.background_pool = locale::background_pool_of(pool);
        }
        let content = Arc::new(loaded.content);
        let others = found.packs.iter().filter(|f| f.game != name).cloned().collect();
        Ok(Game {
            name: loaded.game,
            content,
            dir: loaded.dir,
            packs: vec![loaded.pack],
            own: PackId::default(),
            assets: Arc::new(loaded.assets),
            index: Arc::new(loaded.index),
            others,
            report: loaded.report,
        })
    }

    /// The packs' graphics, the game's own in `lang`: the pack's lettering
    /// in it (fonts, HUD lines, pictures with words) and the content's
    /// strings table, if the language isn't the content's own
    /// (`nettai_content::locale::OWN`). A pack that has no lettering in
    /// `lang` (Chinese: every pack has English's and Japanese's) is drawn
    /// in its own language: its pictures with words are the ones it has,
    /// and the strings are still `lang`'s, which the renderer draws with
    /// the vector font in the enhanced mode and pixel fonts in the
    /// original (`Renderer::set_mode`, `pixeltext`). With them, the
    /// background pool's others (another game's backgrounds a round may be
    /// drawn on: [`Game::other_backgrounds`]).
    pub fn graphics(&self, lang: &str) -> Result<Graphics, LoadError> {
        let mut report = Report::default();
        let mut bundles = Vec::new();
        let mut strings = None;
        for (i, path) in self.packs.iter().enumerate() {
            let bundle = match pack::load_graphics(path) {
                Ok((bundle, r)) => {
                    report.issues.extend(r.issues);
                    bundle
                }
                Err(r) => {
                    report.issues.extend(r.issues);
                    return Err(LoadError { what: Failed::Part { part: "graphics", pack: path.clone() }, report });
                }
            };
            if i != self.own.index() {
                bundles.push(Arc::new(bundle));
                continue;
            }
            let failed = |why: String, report: Report| LoadError { what: Failed::Language(why), report };
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
            bundles.push(Arc::new(bundle.for_language(lang)));
        }
        let others = Arc::new(self.other_backgrounds(&mut report));
        Ok(Graphics { bundles, strings, own: self.own, others, assets: self.assets.clone(), report })
    }

    /// The background pool's backgrounds past the game's own asset table
    /// (docs/app.md §1, "The battle's background": `Content::background_pool`):
    /// other games', each from its game's pack among [`Game::others`], and
    /// the game's own its modules don't name, from its own pack; each read
    /// alone (its folder, `graphics/backgrounds/<name>`: a few kilobytes
    /// each). A game whose pack isn't there has none, and a round on one of
    /// its backgrounds is drawn on its stage's own
    /// (`Packs::round_background`); what reading one said is in `report`.
    pub fn other_backgrounds(&self, report: &mut Report) -> OtherBackgrounds {
        let mut out = OtherBackgrounds::default();
        let own = self.packs.get(self.own.index()).map(|dir| pack::Found { game: self.name.clone(), dir: dir.clone() });
        for found in self.others.iter().chain(own.as_ref()) {
            let c = &self.content;
            let names = c
                .background_pool
                .iter()
                .filter(|&(game, name)| game == found.game && !(game == c.game() && c.assets.handle(nettai_content_api::AssetKind::Background, name).is_some()))
                .map(|(_, name)| name);
            let (backgrounds, r) = pack::load_backgrounds(&found.dir, names);
            report.issues.extend(r.issues);
            out.insert(&found.game, backgrounds);
        }
        out
    }

    /// The packs' sound: each pack's bank, and the content's songs.
    pub fn sound(&self) -> Result<Sound, LoadError> {
        let mut report = Report::default();
        let mut banks = Vec::new();
        for path in &self.packs {
            match pack::load_sound(path) {
                Ok((bank, r)) => {
                    report.issues.extend(r.issues);
                    banks.push(Arc::new(bank));
                }
                Err(r) => {
                    report.issues.extend(r.issues);
                    return Err(LoadError { what: Failed::Part { part: "sound", pack: path.clone() }, report });
                }
            }
        }
        Ok(Sound { banks, songs: nettai_audio::Songs::of(&self.assets), report })
    }
}

/// A game's graphics, in a language. They are shared: every renderer made of
/// them draws from the same, and keeps them for as long as it lives.
#[derive(Clone)]
pub struct Graphics {
    /// Each loaded pack's, by `PackId`.
    pub bundles: Vec<Arc<Bundle>>,
    /// The background pool's backgrounds of other games, from their packs.
    pub others: Arc<OtherBackgrounds>,
    /// The content's strings in the language (none: the content's own).
    pub strings: Option<Arc<Strings>>,
    /// The game's own pack.
    pub own: PackId,
    /// What the game's pack holds of the content's assets.
    pub assets: Arc<AssetNumbers>,
    pub report: Report,
}

impl Graphics {
    /// The packs' graphics, as a renderer holds them.
    pub fn packs(&self) -> PackGraphics {
        PackGraphics::new(self.bundles.clone(), self.own).with_others(self.others.clone()).with_assets(self.assets.clone())
    }

    /// A renderer of these graphics, its text drawn in `text` mode
    /// (`font`: the enhanced mode's, [`font`]).
    pub fn renderer(&self, mode: GraphicsMode, font: Option<Arc<VectorFont>>) -> Renderer {
        let mut renderer = Renderer::with_packs(self.packs());
        renderer.set_strings(self.strings.clone());
        renderer.set_mode(mode, font);
        renderer
    }
}

/// A game's sound: each pack's bank, by `PackId`, and the content's songs.
pub struct Sound {
    pub banks: Vec<Arc<m4a::SoundBank>>,
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
            Some(theme) => nettai_audio::with_theme(self.banks.clone(), &self.songs, &content.battle_music(), theme),
            None => (self.banks.clone(), self.songs.clone()),
        }
    }
}

/// The font the `text` mode draws with: the file at `path`, else the one
/// the library comes with, its cut of `lang` (Noto Sans CJK's: `vfont::Cut`);
/// none in the original mode, which draws the pack's own fonts.
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
    /// The packs directory (none: the packs given alone), and packs given
    /// by their directories (each in place of the found one of its game).
    pub packs_dir: Option<PathBuf>,
    pub packs: Vec<PathBuf>,
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
    /// The packs directory the desktop program looks in
    /// (`nettai_content::pack::packs_dir`), the content's own language,
    /// the enhanced mode with the library's font, with sound.
    fn default() -> Options {
        Options { packs_dir: Some(pack::packs_dir()), packs: Vec::new(), content: None, pool: Vec::new(), lang: locale::OWN.to_string(), mode: GraphicsMode::Enhanced, font: None, reduce_motion: false, sound: true }
    }
}

/// A game, loaded to play ([`load`]).
pub struct Loaded {
    pub found: Found,
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
    /// [`Loaded::battle_themes`]): its song with its pack's bank, to play as
    /// a battle's music in place of its own ([`Sound::with_theme`]). None
    /// where the game has no such theme, or no sound loaded.
    pub fn theme(&self, song: &str) -> Option<nettai_audio::Theme> {
        let assets = &self.game.content.assets;
        let handle = assets.handle(nettai_content_api::AssetKind::Sound, song)?;
        if !self.game.content.battle_themes().contains(&nettai_battle::sound::SoundId(handle)) {
            return None;
        }
        let song = self.game.assets.sound(handle)?;
        let bank = self.sound.as_ref()?.banks.get(self.game.own.index())?.clone();
        Some(nettai_audio::Theme { bank, song })
    }

    /// Everything the loaders reported, in the order they loaded.
    pub fn report(&self) -> Report {
        let mut all = Report::default();
        let reports = [Some(&self.found.report), Some(&self.game.report), Some(&self.graphics.report), self.sound.as_ref().map(|s| &s.report)];
        for r in reports.into_iter().flatten() {
            all.issues.extend(r.issues.iter().cloned());
        }
        all
    }
}

/// Load game `name` to play it: the packs found, its content, its graphics
/// and strings in the language, the text's font and (if asked) its sound.
/// A failure carries what every step reported up to it. ([`Loading`] is
/// the same a step at a time, for a host that shows how far it is.)
pub fn load(name: &str, options: &Options) -> Result<Loaded, LoadError> {
    let mut loading = Loading::new(name, options.clone());
    while loading.step()? {}
    Ok(loading.finish().expect("every step ran"))
}

/// A part of a game's loading: a step of [`Loading`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The packs found.
    Packs,
    /// The game's battle content: its Luau modules read and defined.
    Content,
    /// The pack's graphics, and the content's strings in the language.
    Graphics,
    /// The text's font.
    Font,
    /// The pack's sound.
    Sound,
}

impl Part {
    /// Every part, in the order they load.
    pub const ALL: [Part; 5] = [Part::Packs, Part::Content, Part::Graphics, Part::Font, Part::Sound];

    /// The share of a game's loading the part takes, out of 100: what
    /// [`Loading::fraction`] counts a part done as. From the three games'
    /// packs (crates/nettai-frontend/examples/preload.rs prints each part's
    /// time; docs/frontend.md §9 has them): the content is more than half
    /// of every game's, the sound a quarter.
    pub fn share(self) -> u32 {
        match self {
            Part::Packs => 1,
            Part::Content => 62,
            Part::Graphics => 11,
            Part::Font => 1,
            Part::Sound => 25,
        }
    }
}

/// A game being loaded, a part at a time ([`Part::ALL`]): everything a
/// battle of it reads from disk, which is all read here and none of it
/// during a match (a pack loads whole; docs/frontend.md §9). A host that
/// shows a loading bar runs [`Loading::step`] until it says there is no
/// more, on a thread of its own or between its frames, shows
/// [`Loading::fraction`] meanwhile, and takes the game with
/// [`Loading::finish`]. Each step runs one part to its end: the content's
/// is the longest (seconds; it runs the game's Luau), so a host with
/// threads steps this on one and its window stays live.
///
/// What is left for the thread that will run the battles is that thread's
/// own: `crate::preload`.
pub struct Loading {
    name: String,
    options: Options,
    /// How many of [`Loading::parts`] are loaded.
    done: usize,
    found: Option<Found>,
    game: Option<Game>,
    graphics: Option<Graphics>,
    font: Option<Arc<VectorFont>>,
    sound: Option<Sound>,
}

impl Loading {
    /// Game `name`, to load as `options` say; nothing is read yet.
    pub fn new(name: &str, options: Options) -> Loading {
        Loading { name: name.to_string(), options, done: 0, found: None, game: None, graphics: None, font: None, sound: None }
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
        let reports = [self.found.as_ref().map(|f| &f.report), self.game.as_ref().map(|g| &g.report), self.graphics.as_ref().map(|g| &g.report)];
        let loaded = |what: &str| -> ! { unreachable!("the {what} loads before this part") };
        match part {
            Part::Packs => self.found = Some(Found::find_in(self.options.packs_dir.as_deref(), &self.options.packs)?),
            Part::Content => {
                let Some(found) = &self.found else { loaded("packs") };
                let game = Game::load(found, self.options.content.as_deref(), &self.name, &self.options.pool).map_err(|e| after(&reports, e))?;
                self.game = Some(game);
            }
            Part::Graphics => {
                let Some(game) = &self.game else { loaded("content") };
                let graphics = game.graphics(&self.options.lang).map_err(|e| after(&reports, e))?;
                self.graphics = Some(graphics);
            }
            Part::Font => self.font = font(self.options.mode, self.options.font.as_deref(), &self.options.lang).map_err(|e| after(&reports, e))?,
            Part::Sound => {
                let Some(game) = &self.game else { loaded("content") };
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
        Some(Loaded { found: self.found?, game: self.game?, graphics: self.graphics?, mode: self.options.mode, reduce_motion: self.options.reduce_motion, font: self.font, sound: self.sound })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game loads a part at a time, each counted by its share: the parts
    /// in their order, the sound only if asked for; a part that fails
    /// stops the loading there with what failed, and no game comes of it.
    #[test]
    fn a_game_loads_a_part_at_a_time() {
        let dir = std::env::temp_dir().join(format!("nettai-frontend-loading-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options { packs_dir: Some(dir.clone()), ..Options::default() };
        let mut loading = Loading::new("exe6", options.clone());
        assert_eq!(loading.parts().collect::<Vec<_>>(), Part::ALL);
        assert_eq!(Part::ALL.map(Part::share).iter().sum::<u32>(), 100);
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Packs), 0.0));
        // The packs directory is found (with no pack in it).
        assert!(loading.step().unwrap());
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Content), 0.01));
        // No pack of the game: its content can't load, and the loading
        // stays where it was.
        let Err(e) = loading.step() else { panic!("loaded exe6 with no pack of it") };
        assert_eq!(e.what, Failed::Content { game: "exe6".into() });
        assert_eq!((loading.next(), loading.fraction()), (Some(Part::Content), 0.01));
        assert!(loading.finish().is_none());
        // Without sound, the sound is no part and the others are the whole.
        let silent = Loading::new("exe6", Options { sound: false, ..options });
        assert_eq!(silent.parts().collect::<Vec<_>>(), [Part::Packs, Part::Content, Part::Graphics, Part::Font]);
        assert_eq!(silent.fraction(), 0.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A failure is a value that says what failed: with no pack of the
    /// game anywhere, its content can't load, and the report says why.
    #[test]
    fn a_game_with_no_pack_fails_as_a_value() {
        let dir = std::env::temp_dir().join(format!("nettai-frontend-no-packs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options { packs_dir: Some(dir.clone()), ..Options::default() };
        let Err(e) = load("exe6", &options) else { panic!("loaded exe6 with no pack of it") };
        assert_eq!(e.what, Failed::Content { game: "exe6".into() });
        assert_eq!(e.to_string(), "can't load exe6's battle content");
        assert!(e.report.has_errors(), "{}", e.report);
        // A pack given that isn't one.
        let options = Options { packs: vec![dir.join("nothing")], ..options };
        let Err(e) = load("exe6", &options) else { panic!("read a pack that isn't there") };
        assert_eq!(e.what, Failed::Packs);
        // A font file that isn't there.
        let Err(e) = font(GraphicsMode::Enhanced, Some(&dir.join("no.ttf")), "en") else { panic!("read a font that isn't there") };
        assert!(matches!(&e.what, Failed::Font(why) if why.starts_with("can't read the font")), "{e}");
        assert!(font(GraphicsMode::Original, Some(&dir.join("no.ttf")), "en").unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
