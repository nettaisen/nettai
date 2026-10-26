//! The games the app plays: those there is content of (`crate::contents`: each game's content directory), each
//! loaded and kept (docs/frontend.md §9).
//!
//! A game is ready in two stretches, and the loading screen's bar covers both. Everything read from disk (the
//! content, the pack's graphics and sound, the text's font, and the strings and lettering of the language
//! shown) is read on a thread of the game's own, a part at a time (the library's `game::Loading`), so the
//! window stays live. Then the content's runtime is loaded on the window's thread, the one that runs the
//! battles and checks the builds: that takes about a second with the window still, and can't be done
//! elsewhere (`preload::runtime`). It is done the moment the selected game's parts are in; a game loaded behind
//! the menu takes it when the player steps to it, behind the bar again.
//!
//! Another language's graphics are loaded on a thread too ([`Games::load_language`]), never where they are
//! first asked for. Why a game can't be listed or loaded goes to the log.
//!
//! Before all that, a game whose pack is missing, or was made by another build, has it extracted from the
//! player's Legacy Collection, on a thread of the extraction's own (`crate::packs`; docs/app.md §17): the bar is
//! that first, and the game loads as its pack lands. Its pack is the one its content's tables name
//! (`packs/<game>-<fingerprint>`, `nettai_extract::Tables`), extracted by them.

use crate::bnlc::Install;
use crate::packs::{self, Extractor, Job, Kept, Told};
use nettai_extract::Tables;
use nettai_battle::Content;
use nettai_battle::content::strings::Strings;
use nettai_content::locale;
use nettai_frontend::game::{self, Graphics, Loaded};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// Where a game is.
pub enum State {
    /// No asset pack of it is found (and none is being extracted).
    NoPack,
    /// Its pack is missing, or another build's, and the Legacy Collection found has its ROMs: it is extracted
    /// when the game is first loaded (the user: "you should lazily extract games on first load").
    Extractable,
    /// Its pack is being extracted, or waits to be: how far it is, 0 to 1.
    Extracting(f32),
    NotLoaded,
    /// Its parts are being read: how far the whole of its loading is, 0 to 1.
    Loading(f32),
    Ready(Rc<Ready>),
    /// It couldn't be loaded (why is in the log).
    Failed,
}

/// The game the app opens on the first time, when it can be played.
const FIRST_GAME: &str = "exe6";

/// How much of the bar the parts read on the game's thread are; the rest is the runtime's.
const READ: f32 = 0.8;

/// What a loading game tells the window's thread: how far it is, then the game with its graphics in the
/// language asked for where that isn't the content's own (or why there is no game).
pub enum Step {
    Progress(f32),
    Done(Result<Box<(Loaded, Option<(String, Graphics)>)>, String>),
}

/// A game, loaded: its content, sound and font, and its graphics by language.
pub struct Ready {
    pub loaded: Loaded,
    graphics: std::cell::RefCell<HashMap<String, Graphics>>,
    /// The window's thread has the content's runtime: battles are made and builds checked without a wait.
    runtime: std::cell::Cell<bool>,
}

impl Ready {
    pub fn content(&self) -> &Arc<Content> {
        &self.loaded.game.content
    }

    /// The game's graphics in `lang` (its pack's lettering and the content's strings in it), if they are
    /// loaded; else the content's own. Nothing is read here: [`Games::load_language`] loads a language.
    pub fn graphics(&self, lang: &str) -> Graphics {
        self.graphics.borrow().get(lang).cloned().unwrap_or_else(|| self.loaded.graphics.clone())
    }

    /// Whether `lang`'s graphics need no loading: they are loaded, or the content has none in it.
    pub fn has_language(&self, lang: &str) -> bool {
        lang == locale::OWN || self.graphics.borrow().contains_key(lang) || !locale::languages(&self.loaded.game.dir).iter().any(|l| l == lang)
    }
}

pub struct Entry {
    /// The game pack's id (`exe6`).
    pub id: String,
    pub state: State,
    /// The content directory it is played from (`crate::contents::dir`).
    pub content: PathBuf,
    /// Its asset pack, where it is or is to be; none where there is none to find or make.
    pack: Option<Pack>,
}

/// A game's asset pack: its folder, and, among the app's own packs, the content's tables it is extracted by and
/// their fingerprint (its folder's name: `crate::packs::folder`).
#[derive(Clone)]
struct Pack {
    dir: PathBuf,
    own: Option<(Arc<Tables>, String)>,
}

impl Pack {
    /// Whether it is there.
    fn is_there(&self) -> bool {
        nettai_platform::fs::is_file(self.dir.join(nettai_content::pack::MANIFEST))
    }

    /// Its folder's name among the app's own packs (`exe6-<fingerprint>`).
    fn folder(&self) -> Option<&str> {
        self.own.as_ref()?;
        self.dir.file_name()?.to_str()
    }
}

/// What the player is to be told of the games, once (each is given by [`Games::take_notice`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The game's pack is there and couldn't be loaded.
    LoadFailed(String),
    /// The game's pack couldn't be extracted.
    ExtractFailed(String),
    /// A game has no pack, and no Legacy Collection is found to extract it from.
    NoSource,
}

/// The games there are, and how far each is loaded.
pub struct Games {
    pub list: Vec<Entry>,
    /// What the player hasn't been told yet.
    notices: Vec<Notice>,
    /// The packs folder, and whether its packs are the app's own, extracted here (else `$NETTAI_PACKS`'s:
    /// nothing is extracted).
    pub packs: PathBuf,
    pub extracts: bool,
    /// The games whose pack another build made: it is extracted again, or, where it can't be, loaded as it is.
    stale: Vec<String>,
    /// A Legacy Collection was found at start-up, or chosen since.
    pub found_source: bool,
    /// The Legacy Collection the packs are extracted from, once one is found or chosen.
    source: Option<Install>,
    /// The extraction, once the window's loop runs ([`Games::start`]); the jobs asked for before it does.
    extractor: Option<Extractor>,
    waiting: Vec<Job>,
}

impl Games {
    /// The content's games, each with its asset pack if one is found. With the app's own packs (`extracts`), a
    /// game whose pack is missing or another build's can be extracted from the Legacy Collection, in the folder
    /// `chosen` (the settings') or through Steam: it is, when it is first loaded ([`Games::extract`]).
    pub fn find(chosen: Option<&Path>, may_extract: bool) -> Games {
        let (packs, own) = packs::dir();
        let mut games =
            Games { list: Vec::new(), notices: Vec::new(), packs, extracts: own && may_extract, stale: Vec::new(), found_source: false, source: None, extractor: None, waiting: Vec::new() };
        if games.extracts {
            packs::tidy(&games.packs);
        }
        // (`$NETTAI_PACKS`'s packs, by the game each says.)
        let found = if own {
            Vec::new()
        } else {
            match game::Found::find(&games.packs, &[]) {
                Ok(f) => f.packs,
                Err(e) => {
                    crate::errors::log("finding the packs", &format_args!("{e}: {}", e.report));
                    return games;
                }
            }
        };
        let ids = match crate::contents::games() {
            Ok(ids) => ids,
            Err(why) => {
                crate::errors::log("listing the games", &why);
                return games;
            }
        };
        for id in ids {
            let Some(content) = crate::contents::dir(&id) else { continue };
            let pack = if own {
                games.own_pack(&id, &content)
            } else {
                let pack = found.iter().find(|f| f.game == id).map(|f| Pack { dir: f.dir.clone(), own: None });
                if pack.is_none() {
                    crate::errors::log(&format!("{id}'s pack"), &format_args!("no pack of {id} in {}", games.packs.display()));
                }
                pack
            };
            let state = if pack.as_ref().is_some_and(Pack::is_there) { State::NotLoaded } else { State::NoPack };
            games.list.push(Entry { id, state, content, pack });
        }
        if !games.extracts {
            return games;
        }
        games.stale = games
            .list
            .iter()
            .filter(|e| e.pack.as_ref().and_then(|p| p.own.as_ref()).is_some_and(|(_, print)| packs::kept(&games.packs, &e.id, print) == Kept::Stale))
            .map(|e| e.id.clone())
            .collect();
        if games.wanted().is_empty() {
            return games;
        }
        match Install::find(chosen) {
            Some(install) => {
                games.found(install);
            }
            None if cfg!(target_arch = "wasm32") => crate::errors::log("finding the Legacy Collection", &"no folder chosen"),
            None => crate::errors::log("finding the Legacy Collection", &"not found through Steam, nor in the folder chosen"),
        }
        if games.list.iter().any(|e| matches!(e.state, State::NoPack) && nettai_extract::Game::named(&e.id).is_some()) {
            games.notices.push(Notice::NoSource);
        }
        games
    }

    /// Game `id`'s own pack, for its content `content`: the one its content's tables name, among the app's packs
    /// (none where the extractor makes no pack of the game, or its tables don't read: why is in the log).
    fn own_pack(&self, id: &str, content: &Path) -> Option<Pack> {
        let game = nettai_extract::Game::named(id)?;
        match Tables::of_content(game, content) {
            Ok(tables) => {
                let print = nettai_extract::fingerprint(&tables);
                Some(Pack { dir: self.packs.join(packs::folder(id, &print)), own: Some((Arc::new(tables), print)) })
            }
            Err(why) => {
                crate::errors::log(&format!("reading {id}'s compat tables"), &why);
                None
            }
        }
    }

    /// The games whose pack is to be made: those the extractor makes a pack of with none, or with another
    /// build's that isn't loaded.
    fn wanted(&self) -> Vec<(String, nettai_extract::Game)> {
        self.list
            .iter()
            .filter_map(|e| {
                e.pack.as_ref()?.own.as_ref()?;
                let game = nettai_extract::Game::named(&e.id)?;
                let missing = matches!(e.state, State::NoPack);
                let stale = self.stale.contains(&e.id) && matches!(e.state, State::NotLoaded | State::Failed);
                (missing || stale).then(|| (e.id.clone(), game))
            })
            .collect()
    }

    /// Whether a game's pack is wanted (missing, or another build's) and none is being made of it: the Legacy
    /// Collection's folder would be taken.
    pub fn wants_source(&self) -> bool {
        self.extracts && !self.wanted().is_empty()
    }

    /// The Legacy Collection `install` is where the packs wanted come from: each game whose ROMs it has can be
    /// extracted ([`State::Extractable`]) when it is first loaded. How many there are.
    pub fn found(&mut self, install: Install) -> usize {
        let roms = install.roms();
        let mut count = 0;
        for (id, game) in self.wanted() {
            if !roms.iter().any(|r| r.game == game) {
                crate::errors::log(&format!("extracting {id}"), &format_args!("{} has no ROM of it", install.data.display()));
                continue;
            }
            if let Some(entry) = self.list.iter_mut().find(|e| e.id == id) {
                entry.state = State::Extractable;
                count += 1;
            }
        }
        if count > 0 {
            self.found_source = true;
            self.source = Some(install);
        }
        // (In the browser the archives are the page's only while it is open: every game is extracted now, not as
        // it is first loaded.)
        if cfg!(target_arch = "wasm32") {
            let ids: Vec<String> = self.list.iter().filter(|e| matches!(e.state, State::Extractable)).map(|e| e.id.clone()).collect();
            for id in ids {
                self.extract(&id);
            }
        }
        count
    }

    /// Extract game `id`'s pack, if it is [`State::Extractable`]: it is being loaded for the first time. (Until
    /// [`Games::start`], it waits for the extraction's thread.)
    pub fn extract(&mut self, id: &str) {
        let (Some(install), Some(game)) = (&self.source, nettai_extract::Game::named(id)) else { return };
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        let Some((tables, _)) = entry.pack.as_ref().and_then(|p| p.own.clone()) else { return };
        if !matches!(entry.state, State::Extractable) {
            return;
        }
        entry.state = State::Extracting(0.0);
        let roms: Vec<_> = install.roms().into_iter().filter(|r| r.game == game).collect();
        let job = Job::new(tables, roms);
        match &self.extractor {
            Some(extractor) => extractor.add(vec![job]),
            None => self.waiting.push(job),
        }
    }

    /// The game to open on: `remembered` (the settings') if it can be played, else BN6 (the user: "you should
    /// default to bn6 as the first game though"), else the first that can.
    pub fn first_choice(&self, remembered: Option<&str>) -> Option<String> {
        let games = self.playable();
        remembered
            .filter(|g| games.iter().any(|p| p == g))
            .or_else(|| games.iter().map(String::as_str).find(|g| *g == FIRST_GAME))
            .or_else(|| games.first().map(String::as_str))
            .map(str::to_string)
    }

    /// Start the extraction of the packs found wanted, now that the window's loop runs: each game's progress
    /// and end come to the window's thread (`Shell::extracting`).
    pub fn start(&mut self) {
        if !self.extracts || self.extractor.is_some() {
            return;
        }
        let told: packs::Teller = Arc::new(|pack: &str, told: Told| {
            let pack = pack.to_string();
            let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.shell.extracting(&pack, told)));
        });
        let extractor = Extractor::new(self.packs.clone(), told);
        extractor.add(std::mem::take(&mut self.waiting));
        self.extractor = Some(extractor);
    }

    /// The extraction of the pack of folder `pack` says how far it is, or that it ended: the game whose pack it
    /// is, if one is, and true if its pack landed (it can be loaded now). A failure is told the player, and the
    /// game's old pack, if it has one, is loaded as it is.
    pub fn extracting(&mut self, pack: &str, told: Told) -> Option<(String, bool)> {
        let entry = self.list.iter_mut().find(|e| e.pack.as_ref().and_then(Pack::folder) == Some(pack))?;
        let has_pack = entry.pack.as_ref().is_some_and(Pack::is_there);
        let id = entry.id.clone();
        let landed = Games::extracting_entry(entry, &mut self.stale, &mut self.notices, has_pack, told);
        Some((id, landed))
    }

    fn extracting_entry(entry: &mut Entry, stale: &mut Vec<String>, notices: &mut Vec<Notice>, has_pack: bool, told: Told) -> bool {
        let id = entry.id.clone();
        match told {
            Told::Progress(p) => {
                if matches!(entry.state, State::Extracting(_)) {
                    entry.state = State::Extracting(p);
                }
                false
            }
            Told::Done(Ok(())) => {
                entry.state = State::NotLoaded;
                stale.retain(|s| *s != id);
                true
            }
            Told::Done(Err(_)) => {
                // (Why is in the log: the extraction's thread wrote it.)
                entry.state = if has_pack { State::NotLoaded } else { State::NoPack };
                notices.push(Notice::ExtractFailed(id));
                false
            }
        }
    }

    /// Game `id` is the one shown: if its pack waits to be extracted, it is extracted before the others waiting.
    pub fn prefer(&self, id: &str) {
        if let (Some(extractor), Some(pack)) = (&self.extractor, self.get(id).and_then(|e| e.pack.as_ref()?.folder())) {
            extractor.first(pack);
        }
    }

    /// Whether game `id`'s pack is another build's (it couldn't be made again).
    pub fn is_stale(&self, id: &str) -> bool {
        self.stale.iter().any(|s| s == id)
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.list.iter().find(|e| e.id == id)
    }

    /// Game `id`, if it is ready to play: its parts read, and its runtime on the window's thread.
    pub fn ready(&self, id: &str) -> Option<Rc<Ready>> {
        self.read(id).filter(|r| r.runtime.get())
    }

    /// Games of `loaded`, each by its id, its parts read (a test's: no pack, no extraction).
    #[cfg(test)]
    pub(crate) fn read_of(loaded: Vec<(&str, Loaded)>) -> Games {
        let list = loaded
            .into_iter()
            .map(|(id, loaded)| Entry {
                id: id.to_string(),
                content: loaded.game.dir.clone(),
                state: State::Ready(Rc::new(Ready { loaded, graphics: Default::default(), runtime: false.into() })),
                pack: None,
            })
            .collect();
        Games { list, notices: Vec::new(), packs: PathBuf::new(), extracts: false, stale: Vec::new(), found_source: false, source: None, extractor: None, waiting: Vec::new() }
    }

    /// Game `id`, if its parts are read (its runtime may yet be to load: [`Games::load_runtime`]).
    pub fn read(&self, id: &str) -> Option<Rc<Ready>> {
        match self.get(id).map(|e| &e.state) {
            Some(State::Ready(r)) => Some(r.clone()),
            _ => None,
        }
    }

    /// The games that can be played or may yet be: those with a pack that hasn't failed to load.
    pub fn playable(&self) -> Vec<String> {
        self.list.iter().filter(|e| !matches!(e.state, State::NoPack | State::Failed)).map(|e| e.id.clone()).collect()
    }

    /// How far `id`'s loading is, 0 to 1 (1: it is ready); while its pack is extracted, how far that is.
    pub fn progress(&self, id: &str) -> f32 {
        match self.get(id).map(|e| &e.state) {
            Some(State::Loading(p)) | Some(State::Extracting(p)) => *p,
            Some(State::Ready(r)) if r.runtime.get() => 1.0,
            Some(State::Ready(_)) => READ,
            _ => 0.0,
        }
    }

    /// Start reading `id`'s parts on a thread, if it isn't loaded or loading, to be shown in `lang`. `told`
    /// gets each part's end and then the game, in order, on the window's thread.
    pub fn load(&mut self, id: &str, lang: &str, told: impl Fn(String, Step) + Send + Sync + 'static) {
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        if !matches!(entry.state, State::NotLoaded) {
            return;
        }
        entry.state = State::Loading(0.0);
        let (id, lang) = (id.to_string(), lang.to_string());
        let options = self.options(&id);
        let told = Arc::new(told);
        let name = format!("loading {id}");
        let started = nettai_platform::thread::spawn(&name, move || {
            let tell = |step: Step| {
                let (id, told) = (id.clone(), told.clone());
                let _ = slint::invoke_from_event_loop(move || told(id, step));
            };
            let loaded = (|| {
                let mut loading = game::Loading::new(&id, options);
                while loading.step()? {
                    tell(Step::Progress(loading.fraction() * READ * 0.9));
                }
                let loaded = loading.finish().expect("every part is loaded");
                // The language shown, where the content has it and it isn't its own.
                let other = (lang != locale::OWN && locale::languages(&loaded.game.dir).iter().any(|l| *l == lang)).then(|| loaded.game.graphics(&lang));
                let other = match other {
                    Some(Ok(graphics)) => Some((lang.clone(), graphics)),
                    Some(Err(e)) => {
                        crate::errors::log(&format!("loading {id} in {lang}"), &format_args!("{e}\n{}", e.report));
                        None
                    }
                    None => None,
                };
                Ok((loaded, other))
            })();
            tell(Step::Done(loaded.map(Box::new).map_err(|e: game::LoadError| format!("{e}\n{}", e.report))));
        });
        if let Err(why) = started {
            crate::errors::log(&format!("starting {name}"), &why);
        }
    }

    /// What game `id` loads with: its content directory, and its pack (with `$NETTAI_PACKS`, its folder's, by the
    /// game each says; else its own, beside the others' that are there, whose backgrounds the background pool may
    /// draw from); the background pool, every game's, each from its own content.
    fn options(&self, id: &str) -> game::Options {
        let content = self.get(id).map(|e| e.content.clone());
        let pool = crate::contents::pool();
        if !self.list.iter().any(|e| e.pack.as_ref().is_some_and(|p| p.own.is_some())) {
            return game::Options { packs_dir: Some(self.packs.clone()), content, pool, ..game::Options::default() };
        }
        // (The game's own pack last: it stands in place of any other of its game.)
        let mut packs: Vec<PathBuf> = self.list.iter().filter(|e| e.id != id).filter_map(|e| e.pack.as_ref()).filter(|p| p.is_there()).map(|p| p.dir.clone()).collect();
        packs.extend(self.get(id).and_then(|e| e.pack.as_ref()).map(|p| p.dir.clone()));
        game::Options { packs_dir: None, packs, content, pool, ..game::Options::default() }
    }

    /// A part of `id`'s loading ended: how far it is now, or the game (or why there is none: to the log).
    pub fn step(&mut self, id: &str, step: Step) {
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        entry.state = match step {
            Step::Progress(p) => State::Loading(p),
            Step::Done(Ok(done)) => {
                let (loaded, other) = *done;
                State::Ready(Rc::new(Ready { loaded, graphics: std::cell::RefCell::new(other.into_iter().collect()), runtime: false.into() }))
            }
            Step::Done(Err(why)) => {
                crate::errors::log(&format!("loading {id}"), &why);
                self.notices.push(Notice::LoadFailed(id.to_string()));
                State::Failed
            }
        };
    }

    /// Load `id`'s runtime on this thread, the window's, if its parts are read and it hasn't it: about a
    /// second, the window still. True if the game is ready after it.
    pub fn load_runtime(&mut self, id: &str) -> bool {
        let Some(ready) = self.read(id) else { return false };
        if ready.runtime.get() {
            return true;
        }
        match nettai_frontend::preload::runtime(ready.content()) {
            Ok(_) => ready.runtime.set(true),
            Err(why) => {
                crate::errors::log(&format!("loading {id}'s runtime"), &why);
                if let Some(entry) = self.list.iter_mut().find(|e| e.id == id) {
                    entry.state = State::Failed;
                }
                self.notices.push(Notice::LoadFailed(id.to_string()));
            }
        }
        ready.runtime.get()
    }

    /// Something the player hasn't been told of yet (a game whose pack couldn't be loaded or extracted, no
    /// Legacy Collection to extract one from): each is given once. (Why is in the log.)
    pub fn take_notice(&mut self) -> Option<Notice> {
        (!self.notices.is_empty()).then(|| self.notices.remove(0))
    }

    /// Load every read game's graphics in `lang`, each on a thread, where they aren't loaded: `told` gets
    /// each game's on the window's thread ([`Games::language`] keeps them).
    pub fn load_language(&self, lang: &str, told: impl Fn(String, String, Graphics) + Send + Sync + 'static) {
        let told = Arc::new(told);
        for entry in &self.list {
            let State::Ready(ready) = &entry.state else { continue };
            if ready.has_language(lang) {
                continue;
            }
            let (id, lang, game, told) = (entry.id.clone(), lang.to_string(), ready.loaded.game.clone(), told.clone());
            let name = format!("loading {id} in {lang}");
            let started = nettai_platform::thread::spawn(&name.clone(), move || match game.graphics(&lang) {
                Ok(graphics) => {
                    let _ = slint::invoke_from_event_loop(move || told(id, lang, graphics));
                }
                Err(e) => crate::errors::log(&name, &format_args!("{e}\n{}", e.report)),
            });
            if let Err(why) = started {
                crate::errors::log("starting a language's loading", &why);
            }
        }
    }

    /// `id`'s graphics in `lang`, loaded.
    pub fn language(&mut self, id: &str, lang: &str, graphics: Graphics) {
        if let Some(ready) = self.read(id) {
            ready.graphics.borrow_mut().insert(lang.to_string(), graphics);
        }
    }
}

/// Names in a language: its strings table's where it has them, else the
/// content's own, else the definition's key; one line (a two-line name
/// joined; the games' stacked marks, EX and DS, as their letters).
pub struct Names<'a> {
    pub content: &'a Content,
    pub strings: Option<&'a nettai_battle::content::strings::Strings>,
}

// (The screens that show chips, forms and cards aren't built yet: what they will call isn't called.)
#[allow(dead_code)]
impl<'a> Names<'a> {
    /// The names of `content` in `graphics`'s language.
    pub fn of(content: &'a Content, graphics: &'a Graphics) -> Names<'a> {
        Names { content, strings: graphics.strings.as_deref() }
    }

    fn line(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match (c, nettai_render::vfont::stacked_letters(c)) {
                ('\n', _) => out.push(' '),
                (_, Some(letters)) => out.extend(letters),
                (c, None) => out.push(c),
            }
        }
        out
    }

    /// A name by `key` from the language's table, else the content's, else
    /// the key, on one line.
    fn pick(&self, key: &str, get: impl Fn(&Strings, &str) -> Option<String>) -> String {
        self.found(key, get).unwrap_or_else(|| nettai_match::ids::local(key).to_string())
    }

    /// A name by `key` from the language's table, else the content's, on one line; none if neither has one.
    fn found(&self, key: &str, get: impl Fn(&Strings, &str) -> Option<String>) -> Option<String> {
        self.strings.and_then(|s| get(s, key)).or_else(|| get(&self.content.strings, key)).map(|s| Names::line(&s))
    }

    pub fn navi(&self, navi: nettai_content_api::NaviHandle) -> String {
        self.pick(&self.content.defs.navi(navi).key, |s, k| s.navi(k).and_then(|n| n.name.clone()))
    }

    /// A navi's whole name, where the game's is cut short to its 8 letters ("MagnetMan" for "MagnetMn"): the
    /// language's table's (its whole name, else its name), else the content's.
    pub fn navi_long(&self, navi: nettai_content_api::NaviHandle) -> String {
        self.pick(&self.content.defs.navi(navi).key, |s, k| s.navi(k).and_then(|n| n.long_name.clone().or_else(|| n.name.clone())))
    }

    pub fn chip(&self, chip: nettai_content_api::ChipHandle) -> String {
        self.pick(&self.content.defs.chip(chip).key, |s, k| s.chip(k).and_then(|n| n.name.clone()))
    }

    pub fn form(&self, form: nettai_content_api::FormHandle) -> String {
        self.pick(&self.content.defs.form(form).key, |s, k| s.form(k).and_then(|n| n.name.clone()))
    }

    /// A background's name as a list shows it, any game's (the background pool's): its game's name and its own,
    /// its area's as that game's menus name it ("BN4 ACDC Area": `crate::backgrounds::name`).
    pub fn background(&self, background: nettai_battle::content::BackgroundId) -> String {
        self.content.background_of(background).map(|(game, name)| crate::backgrounds::name(game, name)).unwrap_or_default()
    }

    /// An entry of one of the game's collections (a card, a program).
    pub fn entry(&self, entry: nettai_content_api::EntryHandle) -> String {
        let d = self.content.defs.entry(entry);
        self.pick(d.id(), |s, k| s.entry(&d.collection, k).and_then(|c| c.name.clone()))
    }

    /// An entry's description, its lines as the game's box breaks them;
    /// none where the strings have none.
    pub fn description(&self, entry: nettai_content_api::EntryHandle) -> Option<String> {
        let d = self.content.defs.entry(entry);
        let get = |s: &Strings| s.entry(&d.collection, d.id()).and_then(|x| x.description.clone());
        self.strings.and_then(get).or_else(|| get(&self.content.strings))
    }

    /// Key `key` of the game's text table `table`, on one line; none where
    /// neither the language's strings nor the content's have it.
    pub fn text(&self, table: &str, key: &str) -> Option<String> {
        self.strings.and_then(|s| s.text(table, key)).or_else(|| self.content.strings.text(table, key)).map(Names::line)
    }

    /// A definition of a registry, by its handle.
    pub fn def(&self, registry: nettai_content_api::Registry, h: u16) -> String {
        use nettai_content_api::Registry;
        match registry {
            Registry::Chip => self.chip(nettai_content_api::ChipHandle(h)),
            Registry::Form => self.form(nettai_content_api::FormHandle(h)),
            Registry::Navi => self.navi(nettai_content_api::NaviHandle(h)),
            Registry::Entry => self.entry(nettai_content_api::EntryHandle(h)),
            _ => nettai_match::ids::key_of(self.content, registry, h).map_or_else(|| h.to_string(), |k| nettai_match::ids::local(k).to_string()),
        }
    }
}

#[allow(dead_code)]
/// A description's lines as one: a space where a break falls between two
/// Latin letters (`MegaBstr\nAttck +1`), none between others (Japanese
/// lines run on: `ロックバスターの\n攻撃力が1アップ!`).
pub fn one_line(d: &str) -> String {
    let chars: Vec<char> = d.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c != '\n' {
            out.push(c);
        } else if i > 0 && chars[i - 1].is_ascii() && chars.get(i + 1).is_some_and(|n| n.is_ascii()) {
            out.push(' ');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game whose load fails is no longer one to play, and is given once for the player to be told of; the
    /// others stay.
    #[test]
    fn a_game_that_fails_to_load_is_told_once() {
        let entry = |id: &str| Entry::of(id, State::Loading(0.5));
        let mut games = Games::of(vec![entry("first"), entry("second")], PathBuf::new());
        assert_eq!(games.take_notice(), None);
        games.step("second", Step::Done(Err("its pack names no such sprite".into())));
        assert_eq!(games.playable(), ["first"]);
        assert_eq!(games.take_notice(), Some(Notice::LoadFailed("second".into())));
        assert_eq!(games.take_notice(), None);
        games.step("first", Step::Progress(0.7));
        assert_eq!((games.playable(), games.take_notice()), (vec!["first".to_string()], None));
    }

    impl Entry {
        /// Game `id` in `state`, with no content or pack of its own.
        fn of(id: &str, state: State) -> Entry {
            Entry { id: id.to_string(), state, content: PathBuf::new(), pack: None }
        }
    }

    impl Games {
        /// Games of `list`, their own packs in `packs`, with no extraction under way.
        fn of(list: Vec<Entry>, packs: PathBuf) -> Games {
            Games { list, notices: Vec::new(), packs, extracts: true, stale: Vec::new(), found_source: false, source: None, extractor: None, waiting: Vec::new() }
        }
    }

    /// The game opened on is the one remembered if it can be played, else BN6, else the first that can; a game
    /// that can be extracted can be played (it is extracted as it is first loaded), one with no pack can't, and
    /// nothing is extracted with no collection found.
    #[test]
    fn the_first_game_is_the_remembered_one_else_bn6() {
        let entry = |id: &str, state: State| Entry::of(id, state);
        let mut games = Games::of(vec![entry("exe4", State::NotLoaded), entry("exe5", State::Extractable), entry("exe6", State::Extractable)], PathBuf::new());
        assert_eq!(games.first_choice(None).as_deref(), Some("exe6"));
        assert_eq!(games.first_choice(Some("exe5")).as_deref(), Some("exe5"));
        assert_eq!(games.first_choice(Some("exe3")).as_deref(), Some("exe6"));
        games.extract("exe6");
        assert!(matches!(games.get("exe6").unwrap().state, State::Extractable), "no collection: nothing to extract from");
        games.list[2].state = State::NoPack;
        assert_eq!(games.first_choice(None).as_deref(), Some("exe4"));
    }

    /// A game whose pack is extracted can be stepped to, its bar the extraction's, and loads once its pack lands;
    /// one whose extraction fails is told once, and is played on its old pack if it has one, else not at all.
    #[test]
    fn a_game_plays_once_its_pack_lands() {
        // (exe4's pack lands; exe5's extraction fails and it has another build's pack; exe6's fails and it has
        // none.)
        let packs = tempfile::tempdir().unwrap();
        let content = nettai_content::index::content();
        let games_of = |packs: &Path| Games::of(Vec::new(), packs.to_path_buf());
        let entry = |games: &Games, id: &str| Entry { pack: games.own_pack(id, &content), ..Entry::of(id, State::Extracting(0.0)) };
        let mut games = games_of(packs.path());
        games.list = vec![entry(&games, "exe4"), entry(&games, "exe5"), entry(&games, "exe6")];
        let folder = |id: &str| games.get(id).unwrap().pack.as_ref().unwrap().folder().unwrap().to_string();
        let (four, five, six) = (folder("exe4"), folder("exe5"), folder("exe6"));
        assert!(four.starts_with("exe4-") && four.len() == "exe4-".len() + 16, "{four}");
        nettai_content::pack::write_file(packs.path(), &format!("{five}/content.toml"), b"").unwrap();
        games.stale.push("exe5".into());
        assert_eq!(games.playable(), ["exe4", "exe5", "exe6"]);
        assert!(games.wanted().is_empty(), "nothing is wanted while it is extracted");
        assert_eq!(games.extracting(&four, Told::Progress(0.4)), Some(("exe4".to_string(), false)));
        assert_eq!(games.progress("exe4"), 0.4);
        assert_eq!(games.extracting(&four, Told::Done(Ok(()))), Some(("exe4".to_string(), true)));
        assert!(matches!(games.get("exe4").unwrap().state, State::NotLoaded));
        assert_eq!(games.extracting("exe4-0123456789abcdef", Told::Done(Ok(()))), None, "no game's pack");
        assert_eq!(games.extracting(&five, Told::Done(Err("the disk is full".into()))), Some(("exe5".to_string(), false)));
        assert_eq!(games.extracting(&six, Told::Done(Err("the archive doesn't read".into()))), Some(("exe6".to_string(), false)));
        assert!(matches!(games.get("exe5").unwrap().state, State::NotLoaded), "its old pack, as it is");
        assert!(games.is_stale("exe5") && !games.is_stale("exe4"));
        assert_eq!(games.playable(), ["exe4", "exe5"]);
        assert_eq!(games.take_notice(), Some(Notice::ExtractFailed("exe5".into())));
        assert_eq!(games.take_notice(), Some(Notice::ExtractFailed("exe6".into())));
        assert_eq!(games.take_notice(), None);
        // Wanted again (a folder chosen): the game without a pack, and the one with another build's; not the one
        // that landed.
        let wanted: Vec<String> = games.wanted().into_iter().map(|(id, _)| id).collect();
        assert_eq!(wanted, ["exe5", "exe6"]);
    }
}
