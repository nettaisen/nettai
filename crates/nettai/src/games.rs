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
//! Before all that, a game whose pack is missing (or unfinished) has it extracted from the player's Legacy
//! Collection, on a thread of the extraction's own (`crate::packs`; docs/app.md §17): the bar is that first, and the
//! game loads as its pack lands. Its pack is the one its content's extraction recipe names
//! (`packs/<game>-<fingerprint>`, `nettai_extract::Recipe`), extracted by it from every ROM the recipe declares: a
//! collection that lacks one is told the player.

use crate::bnlc::Install;
use crate::packs::{self, Extractor, Job, Kept, Told};
use nettai_extract::Recipe;
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
    /// Its content is being downloaded (the store has none of it yet: `crate::contents`): how far, 0 to 1.
    Downloading(f32),
    /// There is no content of it: its download failed (why is in the log), or none is published for the app's line
    /// yet (`unpublished`).
    NoContent { unpublished: bool },
    /// No asset pack of it is found (and none is being extracted).
    NoPack,
    /// Its pack is missing, or unfinished, and the Legacy Collection found has all its ROMs: it is extracted when
    /// the game is first loaded (the user: "you should lazily extract games on first load").
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

/// A game's asset pack: its folder, and, among the app's own packs, the content's recipe it is extracted by and
/// its fingerprint (its folder's name: `crate::packs::folder`).
#[derive(Clone)]
struct Pack {
    dir: PathBuf,
    own: Option<(Arc<Recipe>, String)>,
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
    /// A game has no pack, and the Legacy Collection found lacks some of its ROMs (its files are incomplete): the
    /// game's id.
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
    /// The packs folder, and whether its packs are the app's own, extracted here (else `$NETTAI_PACKS`'s:
    /// nothing is extracted).
    pub packs: PathBuf,
    pub extracts: bool,
    /// The games whose pack isn't stamped with its fingerprint (unfinished, or of a stamp before): it is extracted
    /// again, or, where it can't be, loaded as it is.
    stale: Vec<String>,
    /// A Legacy Collection was found at start-up, or chosen since.
    pub found_source: bool,
    /// The Legacy Collection the packs are extracted from, once one is found or chosen.
    source: Option<Install>,
    /// The games whose pack is wanted that the Legacy Collection last found or chosen lacks some ROMs of.
    incomplete: Vec<String>,
    /// The extraction, once the window's loop runs ([`Games::start`]); the jobs asked for before it does.
    extractor: Option<Extractor>,
    waiting: Vec<Job>,
    /// The folder the settings name the Legacy Collection in, looked in again when a game's first content arrives
    /// with a pack to make.
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

/// How many other contents are kept loaded: a replay's and the next one's.
const OTHERS: usize = 2;

impl Games {
    /// The content's games, each with its asset pack if one is found. With the app's own packs (`extracts`), a
    /// game whose pack is missing or another build's can be extracted from the Legacy Collection, in the folder
    /// `chosen` (the settings') or through Steam: it is, when it is first loaded ([`Games::extract`]).
    pub fn find(chosen: Option<&Path>, may_extract: bool) -> Games {
        let (packs, own) = packs::dir();
        let mut games = Games { packs, extracts: own && may_extract, chosen: chosen.map(Path::to_path_buf), ..Games::read_of(Vec::new()) };
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
            // (A game the store has no content of yet: it is downloaded, `Games::download`.)
            let Some(content) = crate::contents::dir(&id) else {
                games.list.push(Entry { id, state: State::Downloading(0.0), content: PathBuf::new(), pack: None });
                continue;
            };
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
        games.seek_source();
        games
    }

    /// Look for the Legacy Collection (in the folder chosen, else through Steam), where a game's pack is wanted and
    /// none is found yet: the games it has the ROMs of can be extracted. None found is told the player.
    fn seek_source(&mut self) {
        if !self.extracts || self.source.is_some() || self.wanted().is_empty() {
            return;
        }
        match Install::find(self.chosen.as_deref()) {
            Some(install) => {
                self.found(install);
            }
            None if cfg!(target_arch = "wasm32") => crate::errors::log("finding the Legacy Collection", &"no folder chosen"),
            None => crate::errors::log("finding the Legacy Collection", &"not found through Steam, nor in the folder chosen"),
        }
        // (A collection found that lacks a game's ROMs was said as it was found: `Games::found`.)
        let unmade = self.list.iter().any(|e| matches!(e.state, State::NoPack) && e.pack.as_ref().is_some_and(|p| p.own.is_some()));
        if unmade && self.source.is_none() && self.incomplete.is_empty() && !self.notices.contains(&Notice::NoSource) {
            self.notices.push(Notice::NoSource);
        }
    }

    /// Game `id`'s own pack, for its content `content`: the one its content's recipe names, among the app's packs
    /// (none where its content declares no recipe, the recipe doesn't read, or it declares a ROM another game listed
    /// before it declares: why is in the log).
    fn own_pack(&self, id: &str, content: &Path) -> Option<Pack> {
        match Recipe::of_content(content, id) {
            Ok(recipe) => {
                let shared = self.list.iter().filter(|e| e.id != id).find_map(|e| {
                    let (theirs, _) = e.pack.as_ref()?.own.as_ref()?;
                    theirs.roms().iter().find(|code| recipe.roms().contains(code)).map(|code| (e.id.clone(), code.clone()))
                });
                if let Some((other, code)) = shared {
                    crate::errors::log(&format!("reading {id}'s extraction recipe"), &format_args!("it declares {code}, which {other}'s declares: two games can't have one ROM"));
                    return None;
                }
                let print = recipe.fingerprint();
                Some(Pack { dir: self.packs.join(packs::folder(id, &print)), own: Some((Arc::new(recipe), print)) })
            }
            Err(why) => {
                crate::errors::log(&format!("reading {id}'s extraction recipe"), &why);
                None
            }
        }
    }

    /// The games whose pack is to be made, with their recipes: those with a recipe and no pack, or one that isn't
    /// stamped with its fingerprint and isn't loaded.
    fn wanted(&self) -> Vec<(String, Arc<Recipe>)> {
        self.list
            .iter()
            .filter_map(|e| {
                let (recipe, _) = e.pack.as_ref()?.own.as_ref()?;
                let missing = matches!(e.state, State::NoPack);
                let stale = self.stale.contains(&e.id) && matches!(e.state, State::NotLoaded | State::Failed);
                (missing || stale).then(|| (e.id.clone(), recipe.clone()))
            })
            .collect()
    }

    /// Whether a game's pack is wanted (missing, or unstamped) and none is being made of it: the Legacy
    /// Collection's folder would be taken.
    pub fn wants_source(&self) -> bool {
        self.extracts && !self.wanted().is_empty()
    }

    /// The Legacy Collection `install` is where the packs wanted come from: each game it has every ROM of (every
    /// one its recipe declares) can be extracted ([`State::Extractable`]) when it is first loaded. How many there
    /// are. A game it lacks ROMs of is said in the log, and told the player once ([`Notice::Incomplete`]): its
    /// files are incomplete.
    pub fn found(&mut self, install: Install) -> usize {
        let mut count = 0;
        self.incomplete.clear();
        for (id, recipe) in self.wanted() {
            if let Err((missing, _)) = install.roms(recipe.roms()) {
                crate::errors::log(
                    &format!("extracting {id}"),
                    &format_args!("the Legacy Collection in {} is incomplete: it has no {} ROM", install.data.display(), missing.join(", no ")),
                );
                self.incomplete.push(id);
                continue;
            }
            if let Some(entry) = self.list.iter_mut().find(|e| e.id == id) {
                entry.state = State::Extractable;
                count += 1;
            }
        }
        if let Some(id) = self.incomplete.first()
            && !self.notices.iter().any(|n| matches!(n, Notice::Incomplete(_)))
        {
            self.notices.push(Notice::Incomplete(id.clone()));
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
        let Some(install) = &self.source else { return };
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        let Some((recipe, _)) = entry.pack.as_ref().and_then(|p| p.own.clone()) else { return };
        if !matches!(entry.state, State::Extractable) {
            return;
        }
        entry.state = State::Extracting(0.0);
        // (Every ROM is there: `Games::found` made it extractable. One gone since fails the extraction, said.)
        let roms = install.roms(recipe.roms()).unwrap_or_else(|(_, found)| found);
        let job = Job::new(recipe, roms);
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
        let of = |e: &Entry| e.pack.as_ref().and_then(Pack::folder) == Some(pack);
        // (Another content's entry with the same pack goes on with it, its failure told as the game's own is.)
        for entry in self.others.iter_mut().filter(|e| of(e)) {
            let has_pack = entry.pack.as_ref().is_some_and(Pack::is_there);
            let mut notices = Vec::new();
            Games::extracting_entry(entry, &mut Vec::new(), &mut notices, has_pack, told.clone());
        }
        let entry = self.list.iter_mut().find(|e| of(e))?;
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
        // (And downloaded first, if its content waits to be.)
        let mut downloads = self.downloads.borrow_mut();
        if let Some(at) = downloads.queue.iter().position(|g| g == id) {
            let game = downloads.queue.remove(at).expect("it is there");
            downloads.queue.push_front(game);
        }
    }

    /// The first game whose pack is wanted that the Legacy Collection found lacks ROMs of, if one is.
    pub fn incomplete(&self) -> Option<&str> {
        self.incomplete.first().map(String::as_str)
    }

    /// Whether game `id`'s pack isn't stamped with its fingerprint (it couldn't be made again).
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

    /// Games of `loaded`, each by its id, its parts read (a test's: no pack, no extraction; with none, the empty
    /// list games start from).
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
        Games {
            list,
            notices: Vec::new(),
            packs: PathBuf::new(),
            extracts: false,
            stale: Vec::new(),
            found_source: false,
            source: None,
            incomplete: Vec::new(),
            extractor: None,
            waiting: Vec::new(),
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

    /// The games that can be played or may yet be: those with a pack that hasn't failed to load.
    pub fn playable(&self) -> Vec<String> {
        self.list.iter().filter(|e| !matches!(e.state, State::NoPack | State::Failed | State::NoContent { .. })).map(|e| e.id.clone()).collect()
    }

    /// How far `id`'s loading is, 0 to 1 (1: it is ready); while its pack is extracted, how far that is.
    pub fn progress(&self, id: &str) -> f32 {
        match self.get(id).map(|e| &e.state) {
            Some(State::Loading(p)) | Some(State::Extracting(p)) | Some(State::Downloading(p)) => *p,
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
        let options = self.options(id, None);
        let told = Arc::new(told);
        let named = id.to_string();
        spawn_load(id, lang, options, move |step| {
            let (id, told) = (named.clone(), told.clone());
            let _ = slint::invoke_from_event_loop(move || told(id, step));
        });
    }

    /// Game `id`'s content in content directory `dir` (a stored bundle of another revision than it plays: a
    /// replay's), to be played: its entry made where it hasn't one, and its next stretch started as the one before
    /// ends: its pack extracted, where it is the Legacy Collection's to make; its parts read on a thread, in
    /// `lang`; then its runtime, on this thread (a second, the window still). Asked each frame while it is wanted.
    pub fn want_other(&mut self, id: &str, dir: &Path, lang: &str) {
        if !self.others.iter().any(|e| e.content == dir) {
            let (_, own) = packs::dir();
            let pack = if own { self.own_pack(id, dir) } else { self.get(id).and_then(|e| e.pack.clone()) };
            let state = if pack.as_ref().is_some_and(Pack::is_there) { State::NotLoaded } else { State::NoPack };
            // (The oldest that isn't on its way goes, past the few kept.)
            while self.others.len() >= OTHERS {
                match self.others.iter().position(|e| !matches!(e.state, State::Loading(_) | State::Extracting(_))) {
                    Some(at) => drop(self.others.remove(at)),
                    None => break,
                }
            }
            self.others.push(Entry { id: id.to_string(), state, content: dir.to_path_buf(), pack });
        }
        let at = self.others.iter().position(|e| e.content == dir).expect("made above");
        match &self.others[at].state {
            State::NoPack => {
                // (Its pack is the Legacy Collection's to make, looked for if it wasn't.)
                let wanted = self.others[at].pack.as_ref().is_some_and(|p| p.own.is_some());
                if wanted && self.source.is_none() && self.extracts {
                    let chosen = self.chosen.clone();
                    if let Some(install) = Install::find(chosen.as_deref()) {
                        self.found(install);
                    }
                }
                let recipe = self.others[at].pack.as_ref().and_then(|p| p.own.as_ref()).map(|(r, _)| r.clone());
                let roms = recipe.as_ref().zip(self.source.as_ref()).and_then(|(r, i)| i.roms(r.roms()).ok());
                match (recipe, roms) {
                    (Some(recipe), Some(roms)) if self.extracts => {
                        self.others[at].state = State::Extracting(0.0);
                        let job = Job::new(recipe, roms);
                        match &self.extractor {
                            Some(extractor) => extractor.add(vec![job]),
                            None => self.waiting.push(job),
                        }
                    }
                    _ => {
                        crate::errors::log(&format!("loading {id} from {}", dir.display()), &"its pack isn't there, and there is nothing to extract it from");
                        self.others[at].state = State::Failed;
                    }
                }
            }
            State::NotLoaded => {
                self.others[at].state = State::Loading(0.0);
                let options = self.options(id, Some(dir));
                let dir = dir.to_path_buf();
                spawn_load(id, lang, options, move |step| {
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

/// The state of game `id` in `list`, if its content is being downloaded.
fn waiting<'a>(list: &'a mut [Entry], id: &str) -> Option<&'a mut State> {
    list.iter_mut().find(|e| e.id == id && matches!(e.state, State::Downloading(_))).map(|e| &mut e.state)
}

/// Read game `id`'s parts on a thread of their own, as `options` say, to be shown in `lang`, telling `tell` each
/// part's end and then the game: what it plays checked against what it is published as.
fn spawn_load(id: &str, lang: &str, options: game::Options, tell: impl Fn(Step) + Send + Sync + 'static) {
    let (id, lang) = (id.to_string(), lang.to_string());
    let name = format!("loading {id}");
    let started = nettai_platform::thread::spawn(&name, move || {
        let loaded = (|| {
            let said = |e: game::LoadError| format!("{e}\n{}", e.report);
            let mut loading = game::Loading::new(&id, options);
            while loading.step().map_err(said)? {
                tell(Step::Progress(loading.fraction() * READ * 0.9));
            }
            let loaded = loading.finish().expect("every part is loaded");
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
    /// What game `id` loads with: its content directory (`other`'s, other content of it, else its own), and its
    /// pack (with `$NETTAI_PACKS`, its folder's, by the game each says; else the one its content names, beside the
    /// other games' that are there, whose backgrounds the background pool may draw from); the background pool,
    /// every game's, each from its own content.
    fn options(&self, id: &str, other: Option<&Path>) -> game::Options {
        let entry = match other {
            Some(dir) => self.others.iter().find(|e| e.content == dir),
            None => self.get(id),
        };
        let content = entry.map(|e| e.content.clone());
        let pool = crate::contents::pool();
        if !self.list.iter().chain(&self.others).any(|e| e.pack.as_ref().is_some_and(|p| p.own.is_some())) {
            return game::Options { packs_dir: Some(self.packs.clone()), content, pool, ..game::Options::default() };
        }
        // (The game's own pack last: it stands in place of any other of its game.)
        let mut packs: Vec<PathBuf> = self.list.iter().filter(|e| e.id != id).filter_map(|e| e.pack.as_ref()).filter(|p| p.is_there()).map(|p| p.dir.clone()).collect();
        packs.extend(entry.and_then(|e| e.pack.as_ref()).map(|p| p.dir.clone()));
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
    /// entry is pointed at it and its pack, to be loaded again (or its pack extracted first). True if its content
    /// changed. A game the list hadn't yet (its first content arrived) is added.
    pub fn retarget(&mut self, id: &str) -> bool {
        let Some(content) = crate::contents::dir(id) else { return false };
        if self.get(id).is_some_and(|e| e.content == content) {
            return false;
        }
        let (_, own) = packs::dir();
        let pack = if own { self.own_pack(id, &content) } else { self.get(id).and_then(|e| e.pack.clone()) };
        let there = pack.as_ref().is_some_and(Pack::is_there);
        let stale = self.extracts && pack.as_ref().and_then(|p| p.own.as_ref()).is_some_and(|(_, print)| packs::kept(&self.packs, id, print) == Kept::Stale);
        let entry = Entry { id: id.to_string(), state: if there { State::NotLoaded } else { State::NoPack }, content, pack };
        match self.list.iter_mut().find(|e| e.id == id) {
            Some(old) => *old = entry,
            None => {
                self.list.push(entry);
                self.list.sort_by(|a, b| a.id.cmp(&b.id));
            }
        }
        self.stale.retain(|s| s != id);
        if stale {
            self.stale.push(id.to_string());
        }
        // (Its pack to be made, where the collection found has its ROMs; looked for now if it wasn't.)
        self.seek_source();
        if let Some(install) = self.source.clone()
            && let Some((_, recipe)) = self.wanted().into_iter().find(|(wanted, _)| wanted == id)
        {
            if install.roms(recipe.roms()).is_ok()
                && let Some(entry) = self.list.iter_mut().find(|e| e.id == id)
            {
                entry.state = State::Extractable;
            }
        }
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
            Some(State::Ready(_) | State::Loading(_) | State::Extracting(_)) => {
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
    /// content changed. (One still loading or extracting waits for its end.)
    pub fn apply_arrived(&mut self) -> Vec<String> {
        let mut changed = Vec::new();
        for id in std::mem::take(&mut self.arrived) {
            if matches!(self.get(&id).map(|e| &e.state), Some(State::Loading(_) | State::Extracting(_))) {
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
                        self.list.push(Entry { id, state: State::Downloading(0.0), content: PathBuf::new(), pack: None });
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
        fn with(mut self, list: Vec<Entry>) -> Games {
            self.list = list;
            self
        }

        /// Games of `list`, their own packs in `packs`, with no extraction under way.
        fn of(list: Vec<Entry>, packs: PathBuf) -> Games {
            Games { packs, extracts: true, ..Games::read_of(Vec::new()) }.with(list)
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
        assert_eq!(games.take_notice(), Some(Notice::DownloadFailed("exe9".into())));
        assert_eq!(games.take_notice(), None);
        assert_eq!(games.no_content(), None, "exe6 has content");
        games.list.retain(|e| e.id == "exe9");
        assert_eq!(games.no_content(), Some(false), "it couldn't be downloaded");
        // (A game none of whose content is published for the line yet: none, and no notice; nothing but such games,
        // and the loading screen says it isn't out yet.)
        games.list.push(Entry::of("exe8", State::Downloading(0.0)));
        games.downloads.borrow_mut().done.push(("exe8".into(), Ok(crate::contents::Newest::Unpublished)));
        games.poll_downloads();
        assert!(matches!(games.get("exe8").unwrap().state, State::NoContent { unpublished: true }));
        assert_eq!(games.take_notice(), None);
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

    /// A game's own pack is its content's recipe's, under the recipe's fingerprint; a game whose recipe declares a
    /// ROM a game listed before it declares has none (two games can't have one ROM).
    #[test]
    fn two_games_cant_declare_one_rom() {
        let (content, packs) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        for (id, roms) in [("g", "\"AAAA\", \"BBBB\""), ("h", "\"CCCC\", \"BBBB\""), ("i", "\"DDDD\"")] {
            let manifest = format!("id = \"{id}\"\nkind = \"game\"\n[extract]\nroms = [{roms}]\nsize = 0x800000\n");
            nettai_content::pack::write_file(content.path(), &format!("{id}/manifest.toml"), manifest.as_bytes()).unwrap();
            nettai_content::pack::write_file(content.path(), &format!("{id}/extract/init.luau"), b"return function(set) return {} end\n").unwrap();
        }
        let mut games = Games::of(Vec::new(), packs.path().to_path_buf());
        let g = games.own_pack("g", content.path()).expect("g's recipe");
        assert!(g.folder().is_some_and(|f| f.starts_with("g-")));
        games.list = vec![Entry { pack: Some(g), ..Entry::of("g", State::NoPack) }];
        assert!(games.own_pack("h", content.path()).is_none(), "BBBB is g's");
        assert!(games.own_pack("i", content.path()).is_some());
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
