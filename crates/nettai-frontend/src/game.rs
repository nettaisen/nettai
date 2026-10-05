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
use nettai_battle::content::PackId;
use nettai_content::locale::{self, Strings};
use nettai_content::pack;
pub use nettai_content::report::Report;
use nettai_render::Renderer;
use nettai_render::packs::PackGraphics;
pub use nettai_render::textlayer::TextMode;
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
    NoPack { game: String },
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
            Failed::NoPack { game } => write!(f, "no {game} pack is loaded"),
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
    /// The packs directory looked in.
    pub dir: PathBuf,
    pub packs: Vec<pack::Found>,
    pub report: Report,
}

impl Found {
    /// The packs in `dir` (`nettai_content::pack::packs_dir` is where the
    /// desktop program looks) and those `given` by their directories.
    pub fn find(dir: &Path, given: &[PathBuf]) -> Result<Found, LoadError> {
        let mut report = Report::default();
        match pack::find(dir, given, &mut report) {
            Some(packs) => Ok(Found { dir: dir.to_path_buf(), packs, report }),
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
    pub report: Report,
}

impl Game {
    /// Game `name`'s content, from the content directory `content` (none:
    /// the one the program comes with), drawn and heard from its pack
    /// among `found`.
    pub fn load(found: &Found, content: Option<&Path>, name: &str) -> Result<Game, LoadError> {
        let loaded =
            pack::load_game(content, name, &found.packs).map_err(|report| LoadError { what: Failed::Content { game: name.to_string() }, report })?;
        let content = Arc::new(loaded.content);
        let Some(own) = content.assets.pack(name) else {
            return Err(LoadError { what: Failed::NoPack { game: name.to_string() }, report: loaded.report });
        };
        Ok(Game { name: loaded.game, content, dir: loaded.dir, packs: vec![loaded.pack], own, report: loaded.report })
    }

    /// The packs' graphics, the game's own in `lang`: the pack's lettering
    /// in it (fonts, HUD lines, pictures with words) and the content's
    /// strings table, if the language isn't the content's own
    /// (`nettai_content::locale::OWN`).
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
            match bundle.in_language(lang) {
                Ok(b) => bundles.push(Arc::new(b)),
                Err(e) => return Err(failed(format!("{e} (extract the pack again with the Japanese ROMs)"), report)),
            }
        }
        Ok(Graphics { bundles, strings, own: self.own, report })
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
        Ok(Sound { banks, songs: nettai_audio::Songs::of(&self.content.assets), report })
    }
}

/// A game's graphics, in a language. They are shared: every renderer made of
/// them draws from the same, and keeps them for as long as it lives.
#[derive(Clone)]
pub struct Graphics {
    /// Each loaded pack's, by `PackId`.
    pub bundles: Vec<Arc<Bundle>>,
    /// The content's strings in the language (none: the content's own).
    pub strings: Option<Arc<Strings>>,
    /// The game's own pack.
    pub own: PackId,
    pub report: Report,
}

impl Graphics {
    /// A renderer of these graphics, its text drawn in `text` mode
    /// (`font`: the font mode's, [`font`]).
    pub fn renderer(&self, text: TextMode, font: Option<Arc<VectorFont>>) -> Renderer {
        let mut renderer = Renderer::with_packs(PackGraphics::new(self.bundles.clone(), self.own));
        renderer.set_strings(self.strings.clone());
        renderer.set_text(text, font);
        renderer
    }
}

/// A game's sound: each pack's bank, by `PackId`, and the content's songs.
pub struct Sound {
    pub banks: Vec<Arc<m4a::SoundBank>>,
    pub songs: nettai_audio::Songs,
    pub report: Report,
}

/// The font the `text` mode draws with: the file at `path`, else the one
/// the library comes with; none in the original's text mode, which draws
/// the pack's own fonts.
pub fn font(text: TextMode, path: Option<&Path>) -> Result<Option<Arc<VectorFont>>, LoadError> {
    if text != TextMode::Font {
        return Ok(None);
    }
    match path {
        Some(path) => match VectorFont::load(path) {
            Ok(f) => Ok(Some(Arc::new(f))),
            Err(e) => Err(LoadError { what: Failed::Font(e), report: Report::default() }),
        },
        None => Ok(Some(Arc::new(VectorFont::bundled()))),
    }
}

/// What [`load`] loads.
#[derive(Clone, Debug)]
pub struct Options {
    /// The packs directory, and packs given by their directories (each in
    /// place of the found one of its game).
    pub packs_dir: PathBuf,
    pub packs: Vec<PathBuf>,
    /// The content directory (none: the one the program comes with).
    pub content: Option<PathBuf>,
    /// The language shown.
    pub lang: String,
    /// How the text is drawn, and the font mode's font file (none: the
    /// library's own).
    pub text: TextMode,
    pub font: Option<PathBuf>,
    /// Load the sound too (a host that plays none needn't).
    pub sound: bool,
}

impl Default for Options {
    /// The packs directory the desktop program looks in
    /// (`nettai_content::pack::packs_dir`), the content's own language,
    /// the font mode with the library's font, with sound.
    fn default() -> Options {
        Options { packs_dir: pack::packs_dir(), packs: Vec::new(), content: None, lang: locale::OWN.to_string(), text: TextMode::Font, font: None, sound: true }
    }
}

/// A game, loaded to play ([`load`]).
pub struct Loaded {
    pub found: Found,
    pub game: Game,
    pub graphics: Graphics,
    pub text: TextMode,
    pub font: Option<Arc<VectorFont>>,
    pub sound: Option<Sound>,
}

impl Loaded {
    /// A renderer of the game's graphics, the text as it was asked for.
    pub fn renderer(&self) -> Renderer {
        self.graphics.renderer(self.text, self.font.clone())
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
/// A failure carries what every step reported up to it.
pub fn load(name: &str, options: &Options) -> Result<Loaded, LoadError> {
    // (A later step's failure, with the earlier steps' reports before its own.)
    fn after(before: &[&Report], mut e: LoadError) -> LoadError {
        let mut issues: Vec<_> = before.iter().flat_map(|r| r.issues.iter().cloned()).collect();
        issues.append(&mut e.report.issues);
        e.report.issues = issues;
        e
    }
    let found = Found::find(&options.packs_dir, &options.packs)?;
    let game = Game::load(&found, options.content.as_deref(), name).map_err(|e| after(&[&found.report], e))?;
    let graphics = game.graphics(&options.lang).map_err(|e| after(&[&found.report, &game.report], e))?;
    let so_far = [&found.report, &game.report, &graphics.report];
    let font = font(options.text, options.font.as_deref()).map_err(|e| after(&so_far, e))?;
    let sound = if options.sound { Some(game.sound().map_err(|e| after(&so_far, e))?) } else { None };
    Ok(Loaded { found, game, graphics, text: options.text, font, sound })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A failure is a value that says what failed: with no pack of the
    /// game anywhere, its content can't load, and the report says why.
    #[test]
    fn a_game_with_no_pack_fails_as_a_value() {
        let dir = std::env::temp_dir().join(format!("nettai-frontend-no-packs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let options = Options { packs_dir: dir.clone(), ..Options::default() };
        let Err(e) = load("exe6", &options) else { panic!("loaded exe6 with no pack of it") };
        assert_eq!(e.what, Failed::Content { game: "exe6".into() });
        assert_eq!(e.to_string(), "can't load exe6's battle content");
        assert!(e.report.has_errors(), "{}", e.report);
        // A pack given that isn't one.
        let options = Options { packs: vec![dir.join("nothing")], ..options };
        let Err(e) = load("exe6", &options) else { panic!("read a pack that isn't there") };
        assert_eq!(e.what, Failed::Packs);
        // A font file that isn't there.
        let Err(e) = font(TextMode::Font, Some(&dir.join("no.ttf"))) else { panic!("read a font that isn't there") };
        assert!(matches!(&e.what, Failed::Font(why) if why.starts_with("can't read the font")), "{e}");
        assert!(font(TextMode::Original, Some(&dir.join("no.ttf"))).unwrap().is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
