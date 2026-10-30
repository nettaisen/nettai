//! The games the app plays: those there is content of (`crate::contents`: each game's content directory), each
//! loaded and kept (docs/frontend.md §9).
//!
//! A game is ready in two stretches, and the loading screen's bar covers both. Everything read from disk (the
//! content, its assets extracted from its ROMs, the text's font, and the strings and lettering of the language
//! shown) is read on a thread of the game's own, a part at a time (the library's `game::Loading`), so the
//! window stays live. Then the content's runtime is loaded on the window's thread, the one that runs the
//! battles and checks the builds: that takes about a second with the window still, and can't be done
//! elsewhere (`preload::runtime`). It is done the moment the selected game's parts are in; a game loaded behind
//! the menu takes it when the player steps to it, behind the bar again.
//!
//! Another language's graphics are loaded on a thread too ([`Games::load_language`]), never where they are
//! first asked for. Why a game can't be listed or loaded goes to the log.
//!
//! A game's assets are extracted from the player's Legacy Collection as it loads (docs/app.md §17): its ROMs are
//! read out of the collection's archives into memory, and its content's recipe (`nettai_extract::Recipe`) extracts
//! them there, on the game's loading thread, with the background pool's other games' for their backgrounds. Nothing
//! is kept on disk. A game whose ROMs the collection hasn't every one of (or with no collection found) can't be
//! played, and the player is told.
//!
//! The recipe loads with the game's content (its root gives it), so which ROMs a game needs is known once its content
//! is read: as the app starts (and as a game's newer content arrives), each game's content is read on a thread of its
//! own ([`State::Reading`]), and the Legacy Collection is looked in once every game's is. What was read is kept for
//! the game's first load, which goes on from it.

use nettai_extract::{Install, Recipe, RomSource};
use nettai_battle::Content;
use nettai_battle::content::strings::Strings;
use nettai_content::locale;
use nettai_frontend::game::{self, ContentLoaded, Graphics, Loaded};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

/// Where a game is.
pub enum State {
    /// Its content is being downloaded (the store has none of it yet: `crate::contents`): how far, 0 to 1.
    Downloading(f32),
    /// There is no content of it: its download failed (why is in the log), or none is published for the app's line
    /// yet (`unpublished`).
    NoContent { unpublished: bool },
    /// Its content is being read (defined, on a thread of its own), with the recipe that states its ROMs.
    Reading,
    /// Its ROMs aren't found: no Legacy Collection is, or the one found hasn't every ROM its recipe declares (or its
    /// content has no recipe that reads: why is in the log).
    NoRoms,
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

    /// The game's graphics in `lang` (their lettering and the content's strings in it), if they are
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
    /// The game's id (`exe6`).
    pub id: String,
    pub state: State,
    /// The content directory it is played from (`crate::contents::dir`).
    pub content: PathBuf,
    /// Its content's recipe, which states its ROMs and extracts them; none where its content has none that reads (or
    /// isn't read yet).
    recipe: Option<Arc<Recipe>>,
    /// Its content as it was read to learn its recipe ([`State::Reading`]), kept for its first load to go on from.
    read: Option<Box<ContentLoaded>>,
}

/// What the player is to be told of the games, once (each is given by [`Games::take_notice`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The game couldn't be loaded: its content, or its assets extracted from its ROMs.
    LoadFailed(String),
    /// No Legacy Collection is found to read the games' ROMs from.
    NoSource,
    /// The Legacy Collection found lacks some of a game's ROMs (its files are incomplete): the game's id.
    Incomplete(String),
    /// The game's content couldn't be downloaded (the store has none of it).
    DownloadFailed(String),
}

/// The games' content being downloaded (`crate::contents`), shared with the future that downloads it on the
/// window's loop: the catalog of games, read first (once a run), then the games to fetch, in turn, the one shown
/// first; how far the one being fetched is; and what came of each since [`Games::poll_downloads`] last looked.
#[derive(Default)]
struct Downloads {
    /// The catalog is being read; what came of it, once it has been.
    reading_catalog: bool,
    catalog: Option<Result<Vec<String>, String>>,
    /// Each game that has content is checked for a newer revision too (Settings' "Check for updates").
    newer: bool,
    queue: std::collections::VecDeque<String>,
    progress: Option<(String, f32)>,
    done: Vec<(String, Result<crate::contents::Newest, String>)>,
    /// The future is running.
    running: bool,
}

impl Downloads {
    fn enqueue(&mut self, game: &str) {
        if !self.queue.iter().any(|g| g == game) {
            self.queue.push_back(game.to_string());
        }
    }
}

/// The games there are, and how far each is loaded.
pub struct Games {
    pub list: Vec<Entry>,
    /// What the player hasn't been told yet.
    notices: Vec<Notice>,
    /// Where the games' ROMs are read from, once found or chosen: the Legacy Collection (or `$NETTAI_ROMS`'s, in
    /// development).
    source: Option<RomSource>,
    /// The games whose ROMs the source last found or chosen lacks some of.
    incomplete: Vec<String>,
    /// The folder the settings name the Legacy Collection in, looked in again when a game's first content arrives
    /// with no ROMs found for it yet.
    chosen: Option<PathBuf>,
    /// The games whose newer content arrived while they were loaded or loading: each is played from it once
    /// [`Games::apply_arrived`] says (at the main menu).
    arrived: Vec<String>,
    /// The games' content being downloaded, and whether the catalog was asked for this run, and couldn't be read.
    downloads: Rc<std::cell::RefCell<Downloads>>,
    catalog_asked: bool,
    catalog_failed: bool,
    /// Content of a game other than the one it plays (a replay's, made on another revision), loaded to play: each
    /// by its content directory, the last few asked for ([`Games::want_other`]).
    others: Vec<Entry>,
}

/// Where a browser or a phone keeps its copies of the Legacy Collection's ROMs ([`Games::keep_copies`]), and any device
/// those another sent it (`crate::nearby`): stowed in the data folder, not read as the page starts.
pub(crate) fn copies() -> PathBuf {
    crate::paths::data().join(nettai_files::STOWED).join("roms")
}

/// How many other contents are kept loaded: a replay's and the next one's.
const OTHERS: usize = 2;

impl Games {
    /// The content's games, each played if its ROMs are found: in the Legacy Collection, in the folder `chosen` (the
    /// settings') or where Steam has it; in development, `$NETTAI_ROMS`'s. With `own_data` (not the screenshot
    /// pass's, which leaves the player's data folder as it is), the packs folder of the app before it read its ROMs
    /// at each start is deleted, once.
    pub fn find(chosen: Option<&Path>, own_data: bool) -> Games {
        if own_data {
            remove_packs();
        }
        let mut games = Games { chosen: chosen.map(Path::to_path_buf), ..Games::read_of(Vec::new()) };
        let ids = match crate::contents::games() {
            Ok(ids) => ids,
            Err(why) => {
                crate::errors::log("listing the games", &why);
                return games;
            }
        };
        for id in ids {
            // (A game the store has no content of yet: it is downloaded, `Games::download`.)
            let Some(content) = crate::contents::dir(&id) else {
                games.list.push(Entry { id, state: State::Downloading(0.0), content: PathBuf::new(), recipe: None, read: None });
                continue;
            };
            spawn_read(&id, &content, false);
            games.list.push(Entry { id, state: State::Reading, content, recipe: None, read: None });
        }
        games.seek_source();
        games
    }

    /// Game `id`'s content in directory `dir` was read ([`spawn_read`]): its recipe states its ROMs, which are looked
    /// for once every game's content is read; what was read is kept for its first load. One that doesn't read can't
    /// be played. (A read of content the game no longer plays from is let go.)
    fn content_read(&mut self, id: &str, dir: &Path, read: Result<Box<ContentLoaded>, String>) {
        let Some(at) = self.list.iter().position(|e| e.id == id && e.content == dir && matches!(e.state, State::Reading)) else { return };
        match read {
            Ok(read) => {
                self.list[at].recipe = self.recipe_of(id, &read);
                self.list[at].read = Some(read);
                self.list[at].state = State::NoRoms;
            }
            Err(why) => {
                crate::errors::log(&format!("loading {id}"), &why);
                self.list[at].state = State::Failed;
                self.notices.push(Notice::LoadFailed(id.to_string()));
            }
        }
        self.seek_source();
    }

    /// Look for the games' ROMs where a game's aren't found yet: `$NETTAI_ROMS`'s (development), else the Legacy
    /// Collection in the folder chosen, else through Steam. Each game it has every ROM of can be played; none found is
    /// told the player.
    fn seek_source(&mut self) {
        // (Once every game's content is read: each states its ROMs.)
        if self.wanted().is_empty() || self.list.iter().any(|e| matches!(e.state, State::Reading)) {
            return;
        }
        if self.source.is_none() {
            match self.look() {
                Some(source) => {
                    self.found(source);
                }
                None if !nettai_dialogs::FOLDERS_READ_AGAIN => crate::errors::log("finding the Legacy Collection", &"no folder chosen, and no ROMs kept of one"),
                None => crate::errors::log("finding the Legacy Collection", &"not found through Steam, nor in the folder chosen"),
            }
        } else {
            self.check_roms();
        }
        // (A collection found that lacks a game's ROMs was said as it was found: `Games::found`.)
        if !self.wanted().is_empty() && self.source.is_none() && self.incomplete.is_empty() && !self.notices.contains(&Notice::NoSource) {
            self.notices.push(Notice::NoSource);
        }
    }

    /// Where the ROMs are: `$NETTAI_ROMS`'s (development); in a browser and on a phone, the copies kept of the
    /// collection chosen before ([`Games::keep_copies`]) or sent by another device (`crate::nearby`); else the Legacy
    /// Collection in the folder chosen, else through Steam, else the copies another device sent.
    fn look(&self) -> Option<RomSource> {
        if std::env::var_os(nettai_extract::source::ROMS).is_some() {
            return RomSource::from_env().map_err(|why| crate::errors::log("finding the ROMs", &why)).ok();
        }
        let kept = || (!nettai_files::stowed(&copies()).is_empty()).then(|| RomSource::Copies(copies()));
        if !nettai_dialogs::FOLDERS_READ_AGAIN {
            return kept();
        }
        Install::find(self.chosen.as_deref()).map(RomSource::Collection).or_else(kept)
    }

    /// Whether every game's content is read and no ROMs were found for any: where a computer nearby is looked for
    /// (`crate::nearby`). (While some game's content is still read, the ROMs aren't looked for yet.)
    pub fn without_roms(&self) -> bool {
        self.source.is_none() && !self.wanted().is_empty() && !self.list.iter().any(|e| matches!(e.state, State::Reading))
    }

    /// Where the games' ROMs are read from, once found: what a desktop offers to another device (`crate::nearby`).
    pub fn rom_source(&self) -> Option<&RomSource> {
        self.source.as_ref()
    }

    /// The game codes of every ROM the games' recipes declare, sorted: what is kept of a collection, and sent of one.
    pub fn rom_codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = self.list.iter().chain(&self.others).filter_map(|e| e.recipe.as_ref()).flat_map(|r| r.roms().iter().cloned()).collect();
        codes.sort();
        codes.dedup();
        codes
    }

    /// In a browser or on a phone, the collection just found in `folder` (the player's, its archives read into the
    /// app's files) kept as copies of its ROMs, every game's it has: the app can't read the player's folder again
    /// without asking, so the games are read from the copies from now on and at each start ([`RomSource::Copies`]: each
    /// ROM's archive entry as it was, about 51 MB for the twelve, read as a game wants it), and the archives are let
    /// go.
    pub fn keep_copies(&mut self, folder: &Path) {
        let Some(source) = self.source.clone().filter(|s| matches!(s, RomSource::Collection(_))) else { return };
        match source.keep_copies(&self.rom_codes(), &copies()) {
            Ok(kept) => {
                crate::errors::log("keeping the Legacy Collection's ROMs", &format_args!("{kept} kept"));
                self.source = Some(RomSource::Copies(copies()));
                if let Err(why) = nettai_files::remove_dir_all(folder) {
                    crate::errors::log("letting the Legacy Collection's archives go", &why);
                }
            }
            Err(why) => crate::errors::log("keeping the Legacy Collection's ROMs", &why),
        }
    }

    /// Game `id`'s recipe, from its content as it was read (none where its content has none, it doesn't read, or it
    /// declares a ROM another game listed before it declares: why is in the log).
    fn recipe_of(&self, id: &str, read: &ContentLoaded) -> Option<Arc<Recipe>> {
        match read.recipe() {
            Ok(recipe) => {
                let shared = self.list.iter().filter(|e| e.id != id).find_map(|e| {
                    let theirs = e.recipe.as_ref()?;
                    theirs.roms().iter().find(|code| recipe.roms().contains(code)).map(|code| (e.id.clone(), code.clone()))
                });
                if let Some((other, code)) = shared {
                    crate::errors::log(&format!("reading {id}'s recipe"), &format_args!("it declares {code}, which {other}'s declares: two games can't have one ROM"));
                    return None;
                }
                Some(recipe.clone())
            }
            Err(why) => {
                crate::errors::log(&format!("reading {id}'s recipe"), &why);
                None
            }
        }
    }

    /// The games whose ROMs aren't found yet, with their recipes.
    fn wanted(&self) -> Vec<(String, Arc<Recipe>)> {
        self.list.iter().filter(|e| matches!(e.state, State::NoRoms)).filter_map(|e| Some((e.id.clone(), e.recipe.clone()?))).collect()
    }

    /// Whether a game's ROMs aren't found: the Legacy Collection's folder would be taken.
    pub fn wants_source(&self) -> bool {
        !self.wanted().is_empty()
    }

    /// The ROMs of `source` are where the games' are read from, if it has every ROM of a game whose ROMs aren't found
    /// yet (every one its recipe declares): each such game can be played. How many there are. A game it lacks ROMs of
    /// is said in the log, and told the player once ([`Notice::Incomplete`]): its files are incomplete.
    pub fn found(&mut self, source: RomSource) -> usize {
        let before = std::mem::replace(&mut self.source, Some(source));
        let count = self.check_roms();
        if count == 0 {
            self.source = before;
        }
        count
    }

    /// Each game whose ROMs weren't found that the source has every ROM of can be played; one it lacks ROMs of is
    /// said in the log, and told the player once. How many can be played now.
    fn check_roms(&mut self) -> usize {
        let Some(source) = self.source.clone() else { return 0 };
        let mut count = 0;
        self.incomplete.clear();
        for (id, recipe) in self.wanted() {
            if let Err((missing, _)) = source.find(recipe.roms()) {
                crate::errors::log(&format!("reading {id}'s ROMs"), &format_args!("{source} is incomplete: it has no {} ROM", missing.join(", no ")));
                self.incomplete.push(id);
                continue;
            }
            if let Some(entry) = self.list.iter_mut().find(|e| e.id == id) {
                entry.state = State::NotLoaded;
                count += 1;
            }
        }
        if let Some(id) = self.incomplete.first()
            && !self.notices.iter().any(|n| matches!(n, Notice::Incomplete(_)))
        {
            self.notices.push(Notice::Incomplete(id.clone()));
        }
        count
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

    /// Game `id` is the one shown: if its content waits to be downloaded, it is downloaded first.
    pub fn prefer(&self, id: &str) {
        let mut downloads = self.downloads.borrow_mut();
        if let Some(at) = downloads.queue.iter().position(|g| g == id) {
            let game = downloads.queue.remove(at).expect("it is there");
            downloads.queue.push_front(game);
        }
    }

    /// The first game the Legacy Collection found lacks ROMs of, if one is.
    pub fn incomplete(&self) -> Option<&str> {
        self.incomplete.first().map(String::as_str)
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.list.iter().find(|e| e.id == id)
    }

    /// Game `id`, if it is ready to play: its parts read, and its runtime on the window's thread.
    pub fn ready(&self, id: &str) -> Option<Rc<Ready>> {
        self.read(id).filter(|r| r.runtime.get())
    }

    /// Games of `loaded`, each by its id, its parts read (a test's: no ROMs; with none, the empty list games start
    /// from).
    pub(crate) fn read_of(loaded: Vec<(&str, Loaded)>) -> Games {
        let list = loaded
            .into_iter()
            .map(|(id, loaded)| Entry {
                id: id.to_string(),
                content: loaded.game.dir.clone(),
                state: State::Ready(Rc::new(Ready { loaded, graphics: Default::default(), runtime: false.into() })),
                recipe: None,
                read: None,
            })
            .collect();
        Games {
            list,
            notices: Vec::new(),
            source: None,
            incomplete: Vec::new(),
            chosen: None,
            arrived: Vec::new(),
            downloads: Default::default(),
            catalog_asked: false,
            catalog_failed: false,
            others: Vec::new(),
        }
    }

    /// Game `id`, if its parts are read (its runtime may yet be to load: [`Games::load_runtime`]).
    pub fn read(&self, id: &str) -> Option<Rc<Ready>> {
        match self.get(id).map(|e| &e.state) {
            Some(State::Ready(r)) => Some(r.clone()),
            _ => None,
        }
    }

    /// The games that can be played or may yet be: those whose ROMs are found that haven't failed to load.
    pub fn playable(&self) -> Vec<String> {
        self.list.iter().filter(|e| !matches!(e.state, State::NoRoms | State::Failed | State::NoContent { .. })).map(|e| e.id.clone()).collect()
    }

    /// How far `id`'s loading is, 0 to 1 (1: it is ready); while its content is downloaded, how far that is.
    pub fn progress(&self, id: &str) -> f32 {
        match self.get(id).map(|e| &e.state) {
            Some(State::Loading(p)) | Some(State::Downloading(p)) => *p,
            Some(State::Ready(r)) if r.runtime.get() => 1.0,
            Some(State::Ready(_)) => READ,
            _ => 0.0,
        }
    }

    /// Start reading `id`'s parts on a thread, if it isn't loaded or loading, to be shown in `lang`. `told`
    /// gets each part's end and then the game, in order, on the window's thread.
    pub fn load(&mut self, id: &str, lang: &str, told: impl Fn(String, Step) + Send + Sync + 'static) {
        let options = self.options(id, None);
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        if !matches!(entry.state, State::NotLoaded) {
            return;
        }
        entry.state = State::Loading(0.0);
        let read = entry.read.take();
        let told = Arc::new(told);
        let named = id.to_string();
        spawn_load(id, lang, options, read, move |step| {
            let (id, told) = (named.clone(), told.clone());
            let _ = slint::invoke_from_event_loop(move || told(id, step));
        });
    }

    /// Game `id`'s content in content directory `dir` (a stored bundle of another revision than it plays: a
    /// replay's), to be played: its entry made where it hasn't one, and its next stretch started as the one before
    /// ends: its parts read on a thread, in `lang` (its assets extracted from the ROMs found); then its runtime, on
    /// this thread (a second, the window still). Asked each frame while it is wanted.
    pub fn want_other(&mut self, id: &str, dir: &Path, lang: &str) {
        if !self.others.iter().any(|e| e.content == dir) {
            // (The oldest that isn't on its way goes, past the few kept.)
            while self.others.len() >= OTHERS {
                match self.others.iter().position(|e| !matches!(e.state, State::Loading(_) | State::Reading)) {
                    Some(at) => drop(self.others.remove(at)),
                    None => break,
                }
            }
            // (Its content read first, for the ROMs its recipe states: `Games::other_read`.)
            spawn_read(id, dir, true);
            self.others.push(Entry { id: id.to_string(), state: State::Reading, content: dir.to_path_buf(), recipe: None, read: None });
        }
        let at = self.others.iter().position(|e| e.content == dir).expect("made above");
        match &self.others[at].state {
            State::NoRoms => {
                crate::errors::log(&format!("loading {id} from {}", dir.display()), &"its ROMs aren't found");
                self.others[at].state = State::Failed;
            }
            State::NotLoaded => {
                self.others[at].state = State::Loading(0.0);
                let read = self.others[at].read.take();
                let options = self.options(id, Some(dir));
                let dir = dir.to_path_buf();
                spawn_load(id, lang, options, read, move |step| {
                    let dir = dir.clone();
                    let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.shell.games.step_other(&dir, step)));
                });
            }
            State::Ready(ready) if !ready.runtime.get() => {
                let ready = ready.clone();
                if let Err(why) = nettai_frontend::preload::runtime(ready.content()) {
                    crate::errors::log(&format!("loading {id}'s runtime from {}", dir.display()), &why);
                    self.others[at].state = State::Failed;
                } else {
                    ready.runtime.set(true);
                }
            }
            _ => {}
        }
    }

    /// Other content `dir` was read ([`Games::want_other`]): its ROMs are looked for (and the Legacy Collection, if
    /// none is found yet); what was read is kept for its load.
    fn other_read(&mut self, dir: &Path, read: Result<Box<ContentLoaded>, String>) {
        let Some(at) = self.others.iter().position(|e| e.content == dir && matches!(e.state, State::Reading)) else { return };
        let id = self.others[at].id.clone();
        let read = match read {
            Ok(read) => read,
            Err(why) => {
                crate::errors::log(&format!("loading {id} from {}", dir.display()), &why);
                self.others[at].state = State::Failed;
                return;
            }
        };
        let recipe = match read.recipe() {
            Ok(r) => Some(r.clone()),
            Err(why) => {
                crate::errors::log(&format!("reading {id}'s recipe from {}", dir.display()), &why);
                None
            }
        };
        // (Its ROMs looked for if they weren't found.)
        if self.source.is_none()
            && let Some(source) = self.look()
        {
            self.found(source);
        }
        let there = recipe.as_ref().zip(self.source.as_ref()).is_some_and(|(r, source)| source.find(r.roms()).is_ok());
        let entry = &mut self.others[at];
        entry.state = if there { State::NotLoaded } else { State::NoRoms };
        entry.recipe = recipe;
        entry.read = Some(read);
    }

    /// Game content `dir`'s ([`Games::want_other`]), once it is ready to play.
    pub fn other_ready(&self, dir: &Path) -> Option<Rc<Ready>> {
        match &self.others.iter().find(|e| e.content == dir)?.state {
            State::Ready(ready) if ready.runtime.get() => Some(ready.clone()),
            _ => None,
        }
    }

    /// Whether game content `dir` couldn't be made ready (why is in the log).
    pub fn other_failed(&self, dir: &Path) -> bool {
        self.others.iter().any(|e| e.content == dir && matches!(e.state, State::Failed))
    }

    /// A part of other content `dir`'s loading ended.
    pub fn step_other(&mut self, dir: &Path, step: Step) {
        let Some(entry) = self.others.iter_mut().find(|e| e.content == dir) else { return };
        entry.state = match step {
            Step::Progress(p) => State::Loading(p),
            Step::Done(Ok(done)) => {
                let (loaded, other) = *done;
                State::Ready(Rc::new(Ready { loaded, graphics: std::cell::RefCell::new(other.into_iter().collect()), runtime: false.into() }))
            }
            Step::Done(Err(why)) => {
                crate::errors::log(&format!("loading {} from {}", entry.id, dir.display()), &why);
                State::Failed
            }
        };
    }
}

/// The folder the app kept its games' asset packs in before it extracted their ROMs at each start (`packs/` in the data
/// folder: a pack a game and recipe, `exe6-<fingerprint>`, beside `.new` and `.old`): nothing reads it now, so each
/// pack in it goes (a folder with a pack's manifest or stamp in it), and the folder with them once it is empty, to
/// give the disk space back. In the browser that frees the page's storage of them too. What fails is logged.
fn remove_packs() {
    let dir = crate::paths::data().join("packs");
    let Ok(read) = nettai_files::read_dir(&dir) else { return };
    let mut removed = 0;
    for entry in read.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let pack = ["content.toml", "stamp.toml"].iter().any(|f| nettai_files::is_file(path.join(f)));
        if !(nettai_files::is_dir(&path) && (pack || name == ".new" || name == ".old")) {
            continue;
        }
        match nettai_files::remove_dir_all(&path) {
            Ok(()) => removed += 1,
            Err(why) => crate::errors::log(&format!("removing the old pack {}", path.display()), &why),
        }
    }
    if nettai_files::read_dir(&dir).is_ok_and(|mut r| r.next().is_none()) {
        let _ = nettai_files::remove_dir(&dir);
    }
    if removed > 0 {
        crate::errors::log("removing the old packs", &format_args!("{removed} removed from {} (the games' ROMs are extracted as they load)", dir.display()));
    }
}

/// The state of game `id` in `list`, if its content is being downloaded.
fn waiting<'a>(list: &'a mut [Entry], id: &str) -> Option<&'a mut State> {
    list.iter_mut().find(|e| e.id == id && matches!(e.state, State::Downloading(_))).map(|e| &mut e.state)
}

/// Read game `id`'s content in directory `dir` on a thread of its own, with its recipe and the background pool as
/// the store has it now (`game::read_content`), for the window's thread to hear: as one of the games
/// ([`Games::content_read`]), or as `other` content of it ([`Games::other_read`]).
fn spawn_read(id: &str, dir: &Path, other: bool) {
    let (id, dir) = (id.to_string(), dir.to_path_buf());
    let pool = crate::contents::pool();
    let name = format!("reading {id}");
    let started = nettai_threads::spawn(&name, move || {
        let read = game::read_content(Some(&dir), &id, &pool).map(Box::new).map_err(|e| format!("{e}\n{}", e.report));
        let _ = slint::invoke_from_event_loop(move || {
            crate::with_app(|app| match other {
                false => app.shell.games.content_read(&id, &dir, read),
                true => app.shell.games.other_read(&dir, read),
            })
        });
    });
    if let Err(why) = started {
        crate::errors::log(&format!("starting {name}"), &why);
    }
}

/// Read game `id`'s parts on a thread of their own, as `options` say, to be shown in `lang`, telling `tell` each
/// part's end and then the game: what it plays checked against what it is published as. Its content as it was read
/// before (`read`, with the background pool `options` have) is gone on from; else it is read first.
fn spawn_load(id: &str, lang: &str, options: game::Options, read: Option<Box<ContentLoaded>>, tell: impl Fn(Step) + Send + Sync + 'static) {
    let (id, lang) = (id.to_string(), lang.to_string());
    let name = format!("loading {id}");
    let started = nettai_threads::spawn(&name, move || {
        let loaded = (|| {
            let said = |e: game::LoadError| format!("{e}\n{}", e.report);
            let read = read.filter(|r| r.pool() == options.pool);
            let mut loading = match read {
                Some(read) => game::Loading::with_content(*read, options),
                None => game::Loading::new(&id, options),
            };
            while loading.step().map_err(said)? {
                tell(Step::Progress(loading.fraction() * READ * 0.9));
            }
            let loaded = loading.finish().expect("every part is loaded");
            // (The content's sprites timed otherwise than the ROMs time them: the content's timing plays, and the
            // log says where they differ.)
            for difference in &loaded.game.timing_differences {
                crate::errors::log(&format!("{id}'s sprite timing"), difference);
            }
            // What it plays is what it is published as (a stored bundle's, the first time it is loaded).
            crate::contents::verify(&loaded.game.dir, loaded.game.content.hash())?;
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
        tell(Step::Done(loaded.map(Box::new)));
    });
    if let Err(why) = started {
        crate::errors::log(&format!("starting {name}"), &why);
    }
}

impl Games {
    /// What game `id` loads with: its content directory (`other`'s, other content of it, else its own); its ROMs,
    /// and the other games' whose backgrounds the background pool may draw from, from the source found; the
    /// background pool, every game's, each from its own content.
    fn options(&self, id: &str, other: Option<&Path>) -> game::Options {
        let entry = match other {
            Some(dir) => self.others.iter().find(|e| e.content == dir),
            None => self.get(id),
        };
        let content = entry.map(|e| e.content.clone());
        let pool = crate::contents::pool();
        game::Options { roms: self.source.clone(), content, pool, ..game::Options::default() }
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
                // (A bundle that wasn't what it is published as is out of the store: the game's newest is
                // another, which it loads instead. Else it can't be played.)
                if crate::contents::dir(id).is_some_and(|dir| dir != entry.content) {
                    self.retarget(id);
                    return;
                }
                self.notices.push(Notice::LoadFailed(id.to_string()));
                State::Failed
            }
        };
    }

    /// Game `id` plays from its content as it is now (`crate::contents::dir`: the newest bundle of its line): its
    /// entry is pointed at it and its recipe, to be loaded again. True if its content changed. A game the list
    /// hadn't yet (its first content arrived) is added.
    pub fn retarget(&mut self, id: &str) -> bool {
        let Some(content) = crate::contents::dir(id) else { return false };
        if self.get(id).is_some_and(|e| e.content == content) {
            return false;
        }
        spawn_read(id, &content, false);
        let entry = Entry { id: id.to_string(), state: State::Reading, content, recipe: None, read: None };
        match self.list.iter_mut().find(|e| e.id == id) {
            Some(old) => *old = entry,
            None => {
                self.list.push(entry);
                self.list.sort_by(|a, b| a.id.cmp(&b.id));
            }
        }
        // (Its ROMs looked for in the source found, else looked for now.)
        self.seek_source();
        true
    }

    /// Game `id` loaded again, on its content as it is now and the background pool as it is now (every game's
    /// newest, which the pool is read from as the game loads): pointed at its newest content where that changed
    /// ([`Games::retarget`]), else its parts read again.
    pub fn reload(&mut self, id: &str) {
        if self.retarget(id) {
            return;
        }
        if let Some(entry) = self.list.iter_mut().find(|e| e.id == id)
            && matches!(entry.state, State::Ready(_))
        {
            entry.state = State::NotLoaded;
        }
    }

    /// Newer content of game `id` arrived in the store: a game that isn't loaded or loading plays from it at once
    /// ([`Games::retarget`]); one that is, from when [`Games::apply_arrived`] says.
    pub fn arrived(&mut self, id: &str) {
        match self.get(id).map(|e| &e.state) {
            Some(State::Ready(_) | State::Loading(_)) => {
                if !self.arrived.iter().any(|a| a == id) {
                    self.arrived.push(id.to_string());
                }
            }
            _ => {
                self.retarget(id);
            }
        }
    }

    /// The games whose content arrived while they were loaded play from it now (the main menu shows, nothing over
    /// it: no match is being played, made or watched): each is loaded again when it is next shown. Those whose
    /// content changed. (One still loading waits for its end.)
    pub fn apply_arrived(&mut self) -> Vec<String> {
        let mut changed = Vec::new();
        for id in std::mem::take(&mut self.arrived) {
            if matches!(self.get(&id).map(|e| &e.state), Some(State::Loading(_))) {
                self.arrived.push(id);
            } else if self.retarget(&id) {
                changed.push(id);
            }
        }
        changed
    }

    /// Download the games' content, on the window's loop: the catalog of games read first, once a run
    /// (`crate::contents::fetch_catalog`), then each game's newest bundle where the store hasn't it
    /// (`crate::contents::fetch_newest`), one after the other, the one shown first: every game the store has no
    /// content of, whatever the settings say, and, with `newer` (Settings' "Check for updates"), each game that has
    /// some, where its index has a newer revision. Development's content downloads nothing.
    pub fn download(&mut self, newer: bool) {
        if crate::contents::development() {
            return;
        }
        let read_catalog = !std::mem::replace(&mut self.catalog_asked, true);
        let mut downloads = self.downloads.borrow_mut();
        downloads.newer |= newer;
        for e in &self.list {
            if matches!(e.state, State::Downloading(_)) || (newer && !e.content.as_os_str().is_empty()) {
                downloads.enqueue(&e.id);
            }
        }
        if downloads.running || (downloads.queue.is_empty() && !read_catalog) {
            return;
        }
        downloads.running = true;
        downloads.reading_catalog = read_catalog;
        drop(downloads);
        let shared = self.downloads.clone();
        let started = slint::spawn_local(async move {
            if read_catalog {
                let read = crate::contents::fetch_catalog().await;
                let mut downloads = shared.borrow_mut();
                if let Ok(games) = &read {
                    for game in games {
                        if crate::contents::dir(game).is_none() || downloads.newer {
                            downloads.enqueue(game);
                        }
                    }
                }
                downloads.catalog = Some(read);
                downloads.reading_catalog = false;
            }
            loop {
                let Some(game) = shared.borrow_mut().queue.pop_front() else { break };
                let (told, id) = (shared.clone(), game.clone());
                let done = crate::contents::fetch_newest(&game, move |p| told.borrow_mut().progress = Some((id.clone(), p))).await;
                shared.borrow_mut().done.push((game, done));
            }
            shared.borrow_mut().running = false;
        });
        if let Err(why) = started {
            crate::errors::log("downloading the games' content", &why);
            let mut downloads = self.downloads.borrow_mut();
            downloads.running = false;
            downloads.reading_catalog = false;
            let why = format!("the download couldn't start: {why}");
            if read_catalog {
                downloads.catalog = Some(Err(why.clone()));
            }
            let failed: Vec<String> = downloads.queue.drain(..).collect();
            downloads.done.extend(failed.into_iter().map(|game| (game, Err(why.clone()))));
        }
    }

    /// The catalog of games is being read, and no game is known yet: there is nothing to show but that.
    pub fn reading_catalog(&self) -> bool {
        self.list.is_empty() && self.downloads.borrow().reading_catalog
    }

    /// No game has content, and none can have, and why: none is published for this line yet (true), or the
    /// catalog couldn't be read and none was kept, or every game's download failed (false).
    pub fn no_content(&self) -> Option<bool> {
        if self.reading_catalog() {
            return None;
        }
        if self.list.is_empty() {
            return self.catalog_failed.then_some(false);
        }
        let mut unpublished = true;
        for e in &self.list {
            match e.state {
                State::NoContent { unpublished: not_yet } => unpublished &= not_yet,
                _ => return None,
            }
        }
        Some(unpublished)
    }

    /// What the downloads did since the last frame: how far the game being downloaded is; a game whose content
    /// arrived plays from it ([`Games::arrived`]); one with no content whose download failed has none (the player
    /// is told), and one that has some keeps it (the log says why); one with none published for the line yet has
    /// none (the loading screen says so where no game has any).
    pub fn poll_downloads(&mut self) {
        let (catalog, progress, done) = {
            let mut downloads = self.downloads.borrow_mut();
            (downloads.catalog.take(), downloads.progress.take(), std::mem::take(&mut downloads.done))
        };
        // The catalog's games this app plays: each one the list hasn't yet is to be downloaded (the future has it
        // waiting), or has content already (the store kept it from a catalog of before).
        match catalog {
            Some(Ok(games)) => {
                for id in games {
                    if self.get(&id).is_some() {
                        continue;
                    }
                    if !self.retarget(&id) {
                        self.list.push(Entry { id, state: State::Downloading(0.0), content: PathBuf::new(), recipe: None, read: None });
                        self.list.sort_by(|a, b| a.id.cmp(&b.id));
                    }
                }
            }
            Some(Err(why)) => {
                crate::errors::log("reading the catalog of games", &why);
                self.catalog_failed = true;
            }
            None => {}
        }
        if let Some((id, p)) = progress
            && let Some(entry) = self.list.iter_mut().find(|e| e.id == id && matches!(e.state, State::Downloading(_)))
        {
            entry.state = State::Downloading(p);
        }
        for (id, done) in done {
            match done {
                Ok(crate::contents::Newest::Downloaded) => self.arrived(&id),
                Ok(crate::contents::Newest::Had) => {}
                Ok(crate::contents::Newest::Unpublished) => {
                    crate::errors::log(&format!("downloading {id}'s content"), &format_args!("none is published for nettai {} yet", crate::contents::line()));
                    if let Some(state) = waiting(&mut self.list, &id) {
                        *state = State::NoContent { unpublished: true };
                    }
                }
                Err(why) => {
                    crate::errors::log(&format!("downloading {id}'s content"), &why);
                    if let Some(state) = waiting(&mut self.list, &id) {
                        *state = State::NoContent { unpublished: false };
                        self.notices.push(Notice::DownloadFailed(id));
                    }
                }
            }
        }
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

    /// Something the player hasn't been told of yet (a game that couldn't be loaded, no Legacy Collection to read
    /// the games' ROMs from, one that lacks some): each is given once. (Why is in the log.)
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
            let started = nettai_threads::spawn(&name.clone(), move || match game.graphics(&lang) {
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
        let mut games = Games::of(vec![entry("first"), entry("second")]);
        assert_eq!(games.take_notice(), None);
        games.step("second", Step::Done(Err("its assets name no such sprite".into())));
        assert_eq!(games.playable(), ["first"]);
        assert_eq!(games.take_notice(), Some(Notice::LoadFailed("second".into())));
        assert_eq!(games.take_notice(), None);
        games.step("first", Step::Progress(0.7));
        assert_eq!((games.playable(), games.take_notice()), (vec!["first".to_string()], None));
    }

    impl Entry {
        /// Game `id` in `state`, with no content or recipe of its own.
        fn of(id: &str, state: State) -> Entry {
            Entry { id: id.to_string(), state, content: PathBuf::new(), recipe: None, read: None }
        }
    }

    impl Games {
        fn with(mut self, list: Vec<Entry>) -> Games {
            self.list = list;
            self
        }

        /// Games of `list`, with no ROMs found yet.
        fn of(list: Vec<Entry>) -> Games {
            Games::read_of(Vec::new()).with(list)
        }
    }

    /// As the downloads go: the catalog's games the list hasn't are added, each to be downloaded (one whose
    /// content is there already plays it); the one being downloaded says how far it is; one whose download failed
    /// has no content, and is told the player once; and when no game has content, that is what the loading screen
    /// says. Until the catalog is read, with no game known, there is nothing to show but that.
    #[test]
    fn games_come_as_their_content_is_downloaded() {
        let mut games = Games::read_of(Vec::new());
        games.downloads.borrow_mut().reading_catalog = true;
        assert!(games.reading_catalog() && games.no_content().is_none());
        {
            let mut downloads = games.downloads.borrow_mut();
            downloads.reading_catalog = false;
            // (exe6 has content here, the repository's; "exe9" has none anywhere.)
            downloads.catalog = Some(Ok(vec!["exe6".into(), "exe9".into()]));
            downloads.progress = Some(("exe9".into(), 0.25));
        }
        games.poll_downloads();
        assert_eq!(games.list.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["exe6", "exe9"]);
        assert!(!matches!(games.get("exe6").unwrap().state, State::Downloading(_)), "its content is there");
        assert_eq!(games.progress("exe9"), 0.25);
        assert!(games.playable().contains(&"exe9".to_string()), "a game being downloaded can be stepped to");
        games.downloads.borrow_mut().done.push(("exe9".into(), Err("no connection".into())));
        games.poll_downloads();
        assert!(matches!(games.get("exe9").unwrap().state, State::NoContent { unpublished: false }));
        assert!(!games.playable().contains(&"exe9".to_string()));
        // (exe6's ROMs may or may not be found where this runs: a notice of that may come too.)
        let notices: Vec<Notice> = std::iter::from_fn(|| games.take_notice()).collect();
        let failed: Vec<&Notice> = notices.iter().filter(|n| matches!(n, Notice::DownloadFailed(_))).collect();
        assert_eq!(failed, [&Notice::DownloadFailed("exe9".into())], "{notices:?}");
        assert_eq!(games.no_content(), None, "exe6 has content");
        games.list.retain(|e| e.id == "exe9");
        assert_eq!(games.no_content(), Some(false), "it couldn't be downloaded");
        // (A game none of whose content is published for the line yet: none, and no notice; nothing but such games,
        // and the loading screen says it isn't out yet.)
        games.list.push(Entry::of("exe8", State::Downloading(0.0)));
        games.downloads.borrow_mut().done.push(("exe8".into(), Ok(crate::contents::Newest::Unpublished)));
        games.poll_downloads();
        assert!(matches!(games.get("exe8").unwrap().state, State::NoContent { unpublished: true }));
        assert!(!std::iter::from_fn(|| games.take_notice()).any(|n| matches!(n, Notice::DownloadFailed(_))));
        assert_eq!(games.no_content(), Some(false), "one couldn't be downloaded");
        games.list.retain(|e| e.id == "exe8");
        assert_eq!(games.no_content(), Some(true));
        // (The catalog that couldn't be read, and none kept: nothing to play.)
        let mut games = Games::read_of(Vec::new());
        games.downloads.borrow_mut().catalog = Some(Err("no connection".into()));
        games.poll_downloads();
        assert_eq!(games.no_content(), Some(false));
        assert!(games.list.is_empty());
    }

    /// The game opened on is the one remembered if it can be played, else BN6, else the first that can; one whose
    /// ROMs aren't found can't.
    #[test]
    fn the_first_game_is_the_remembered_one_else_bn6() {
        let entry = |id: &str, state: State| Entry::of(id, state);
        let mut games = Games::of(vec![entry("exe4", State::NotLoaded), entry("exe5", State::NotLoaded), entry("exe6", State::NotLoaded)]);
        assert_eq!(games.first_choice(None).as_deref(), Some("exe6"));
        assert_eq!(games.first_choice(Some("exe5")).as_deref(), Some("exe5"));
        assert_eq!(games.first_choice(Some("exe3")).as_deref(), Some("exe6"));
        games.list[2].state = State::NoRoms;
        assert_eq!(games.first_choice(None).as_deref(), Some("exe4"));
    }

    /// Game `id` of the repository's content, read as the app reads it before it looks for the game's ROMs.
    fn read(id: &str) -> Result<Box<ContentLoaded>, String> {
        game::read_content(None, id, &[]).map(Box::new).map_err(|e| format!("{e}\n{}", e.report))
    }

    /// A game's recipe is its content's; a game whose recipe states a ROM a game listed before it states has none
    /// (two games can't have one ROM).
    #[test]
    fn two_games_cant_declare_one_rom() {
        let (five, six) = (read("exe5").unwrap(), read("exe6").unwrap());
        let mut games = Games::of(Vec::new());
        let recipe = games.recipe_of("exe6", &six).expect("exe6's recipe");
        assert_eq!(recipe.roms(), ["BR6E", "BR5E", "BR6J", "BR5J"]);
        games.list = vec![Entry { recipe: Some(recipe), ..Entry::of("other", State::NoRoms) }];
        assert!(games.recipe_of("exe6", &six).is_none(), "BR6E is other's");
        assert!(games.recipe_of("exe5", &five).is_some());
    }

    /// Each game's content is read before its ROMs are looked for (its recipe states them), and they are looked for
    /// once every game's content is read: what was read is kept for the game's load; content that doesn't read can't
    /// be played; a read of content the game no longer plays from is let go.
    #[test]
    fn the_roms_are_looked_for_once_every_games_content_is_read() {
        use nettai_extract::source::testing::{archive, fake_rom};
        let install = tempfile::tempdir().unwrap();
        let roms: Vec<(String, Vec<u8>)> = ["BR6E", "BR5E", "BR6J", "BR5J"].iter().map(|c| (format!("{c}.srl"), fake_rom(c, 0x100))).collect();
        archive(&install.path().join("exe/data/exe6.dat"), &roms.iter().map(|(n, r)| (n.as_str(), r.as_slice())).collect::<Vec<_>>());
        let content = nettai_content::index::content();
        let mut games = Games::of(Vec::new());
        games.source = Some(RomSource::at(install.path()).unwrap());
        for id in ["exe5", "exe6"] {
            games.list.push(Entry { content: content.clone(), ..Entry::of(id, State::Reading) });
        }
        assert_eq!(games.playable(), ["exe5", "exe6"], "content being read may yet play");
        games.content_read("exe6", Path::new("elsewhere"), read("exe6"));
        assert!(matches!(games.get("exe6").unwrap().state, State::Reading), "not the content it plays from");
        games.content_read("exe6", &content, read("exe6"));
        assert!(matches!(games.get("exe6").unwrap().state, State::NoRoms), "its ROMs wait for exe5's content");
        games.content_read("exe5", &content, Err("exe5's content doesn't load".into()));
        assert!(matches!(games.get("exe6").unwrap().state, State::NotLoaded));
        assert!(games.get("exe6").unwrap().read.is_some(), "kept for its load");
        assert!(matches!(games.get("exe5").unwrap().state, State::Failed));
        assert_eq!(games.take_notice(), Some(Notice::LoadFailed("exe5".into())));
        assert_eq!(games.playable(), ["exe6"]);
    }

    /// The Legacy Collection found makes each game it has every ROM of playable; one it lacks ROMs of is told once
    /// as incomplete, and stays unplayable; a folder that has no game's whole is not taken, and the games stay as
    /// they were.
    #[test]
    fn a_game_plays_once_its_roms_are_found() {
        use nettai_extract::source::testing::{archive, fake_rom};
        let content = nettai_content::index::content();
        let mut games = Games::of(Vec::new());
        for id in ["exe5", "exe6"] {
            let recipe = games.recipe_of(id, &read(id).unwrap());
            games.list.push(Entry { recipe, content: content.clone(), ..Entry::of(id, State::NoRoms) });
        }
        assert!(games.wants_source());
        assert_eq!(games.playable(), Vec::<String>::new());
        // (A folder of one of EXE6's ROMs alone: no game whole.)
        let some = tempfile::tempdir().unwrap();
        archive(&some.path().join("exe/data/exe6.dat"), &[("a.srl", &fake_rom("BR6E", 0x100))]);
        assert_eq!(games.found(RomSource::at(some.path()).unwrap()), 0);
        assert_eq!((games.playable(), games.incomplete()), (Vec::<String>::new(), Some("exe5")));
        assert_eq!(games.take_notice(), Some(Notice::Incomplete("exe5".into())));
        // (A folder of EXE5's ROMs and half of EXE6's.)
        let install = tempfile::tempdir().unwrap();
        let roms: Vec<(String, Vec<u8>)> =
            ["BRBE", "BRKE", "BRBJ", "BRKJ", "BR6E", "BR5E"].iter().map(|c| (format!("{c}.srl"), fake_rom(c, 0x100))).collect();
        archive(&install.path().join("exe/data/exe6.dat"), &roms.iter().map(|(n, r)| (n.as_str(), r.as_slice())).collect::<Vec<_>>());
        assert_eq!(games.found(RomSource::at(install.path()).unwrap()), 1);
        assert_eq!((games.playable(), games.incomplete()), (vec!["exe5".to_string()], Some("exe6")));
        assert!(matches!(games.get("exe5").unwrap().state, State::NotLoaded));
        assert!(matches!(games.get("exe6").unwrap().state, State::NoRoms));
        assert_eq!(games.options("exe5", None).roms, Some(RomSource::at(install.path()).unwrap()), "exe5 loads from the folder found");
        assert!(games.wants_source(), "exe6's ROMs are still wanted");
    }
}
