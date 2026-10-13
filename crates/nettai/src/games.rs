//! The games the app plays: those the content has (`pack::games`), each
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

use nettai_battle::Content;
use nettai_battle::content::strings::Strings;
use nettai_content::locale;
use nettai_frontend::game::{self, Graphics, Loaded};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Where a game is.
pub enum State {
    /// No asset pack of it is found.
    NoPack,
    NotLoaded,
    /// Its parts are being read: how far the whole of its loading is, 0 to 1.
    Loading(f32),
    Ready(Rc<Ready>),
    /// It couldn't be loaded (why is in the log).
    Failed,
}

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
}

/// The games there are, and how far each is loaded.
pub struct Games {
    pub list: Vec<Entry>,
    /// The games whose pack is there and couldn't be loaded, each until the player is told
    /// ([`Games::take_failed`]).
    failed: Vec<String>,
}

impl Games {
    /// The content's games, each with its asset pack if one is found.
    pub fn find() -> Games {
        let found = match game::Found::find(&nettai_content::pack::packs_dir(), &[]) {
            Ok(f) => f,
            Err(e) => {
                crate::errors::log("finding the packs", &format_args!("{e}: {}", e.report));
                return Games { list: Vec::new(), failed: Vec::new() };
            }
        };
        match nettai_content::pack::games(None, &found.packs) {
            Ok(choices) => Games {
                list: choices
                    .into_iter()
                    .map(|c| {
                        if let Some(why) = &c.why_not {
                            crate::errors::log(&format!("{}'s pack", c.game), why);
                        }
                        Entry { state: if c.pack.is_some() { State::NotLoaded } else { State::NoPack }, id: c.game }
                    })
                    .collect(),
                failed: Vec::new(),
            },
            Err(report) => {
                crate::errors::log("listing the games", &report);
                Games { list: Vec::new(), failed: Vec::new() }
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.list.iter().find(|e| e.id == id)
    }

    /// Game `id`, if it is ready to play: its parts read, and its runtime on the window's thread.
    pub fn ready(&self, id: &str) -> Option<Rc<Ready>> {
        self.read(id).filter(|r| r.runtime.get())
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

    /// How far `id`'s loading is, 0 to 1 (1: it is ready).
    pub fn progress(&self, id: &str) -> f32 {
        match self.get(id).map(|e| &e.state) {
            Some(State::Loading(p)) => *p,
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
        let told = Arc::new(told);
        std::thread::spawn(move || {
            let tell = |step: Step| {
                let (id, told) = (id.clone(), told.clone());
                let _ = slint::invoke_from_event_loop(move || told(id, step));
            };
            let loaded = (|| {
                let mut loading = game::Loading::new(&id, game::Options::default());
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
                self.failed.push(id.to_string());
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
                self.failed.push(id.to_string());
            }
        }
        ready.runtime.get()
    }

    /// A game whose pack is there and couldn't be loaded, that the player hasn't been told of: each is given
    /// once. (Why is in the log.)
    pub fn take_failed(&mut self) -> Option<String> {
        (!self.failed.is_empty()).then(|| self.failed.remove(0))
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
            std::thread::spawn(move || match game.graphics(&lang) {
                Ok(graphics) => {
                    let _ = slint::invoke_from_event_loop(move || told(id, lang, graphics));
                }
                Err(e) => crate::errors::log(&format!("loading {id} in {lang}"), &format_args!("{e}\n{}", e.report)),
            });
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
        self.strings
            .and_then(|s| get(s, key))
            .or_else(|| get(&self.content.strings, key))
            .map(|s| Names::line(&s))
            .unwrap_or_else(|| nettai_match::ids::local(key).to_string())
    }

    pub fn navi(&self, navi: nettai_content_api::NaviHandle) -> String {
        self.pick(&self.content.defs.navi(navi).key, |s, k| s.navi(k).and_then(|n| n.name.clone()))
    }

    pub fn chip(&self, chip: nettai_content_api::ChipHandle) -> String {
        self.pick(&self.content.defs.chip(chip).key, |s, k| s.chip(k).and_then(|n| n.name.clone()))
    }

    pub fn form(&self, form: nettai_content_api::FormHandle) -> String {
        self.pick(&self.content.defs.form(form).key, |s, k| s.form(k).and_then(|n| n.name.clone()))
    }

    /// A background's name (its area's, as the game's menus name it), else
    /// its asset name.
    pub fn background(&self, background: nettai_battle::content::BackgroundId) -> String {
        let key = nettai_match::ids::background_name(self.content, background).unwrap_or_default();
        self.pick(key, |s, k| s.background(k).and_then(|b| b.name.clone()))
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
        let entry = |id: &str| Entry { id: id.to_string(), state: State::Loading(0.5) };
        let mut games = Games { list: vec![entry("first"), entry("second")], failed: Vec::new() };
        assert_eq!(games.take_failed(), None);
        games.step("second", Step::Done(Err("its pack names no such sprite".into())));
        assert_eq!(games.playable(), ["first"]);
        assert_eq!(games.take_failed().as_deref(), Some("second"));
        assert_eq!(games.take_failed(), None);
        games.step("first", Step::Progress(0.7));
        assert_eq!((games.playable(), games.take_failed()), (vec!["first".to_string()], None));
    }
}
