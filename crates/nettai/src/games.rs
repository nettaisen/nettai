//! The games the app plays: those the content has (`pack::games`), each
//! loaded on a thread of its own the first time it's wanted and kept, and
//! its graphics in each language shown, loaded as each is first asked for.

use nettai_battle::Content;
use nettai_frontend::game::{self, Graphics, Loaded, Options};
use nettai_content::locale;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Where a game is.
pub enum State {
    /// No asset pack of it is found: the command that writes one.
    NoPack(String),
    NotLoaded,
    Loading,
    Ready(Rc<Ready>),
    /// It couldn't be loaded: why (the library's words).
    Failed(String),
}

/// A game, loaded: its content, sound and font, and its graphics by
/// language.
pub struct Ready {
    pub loaded: Loaded,
    graphics: std::cell::RefCell<HashMap<String, Graphics>>,
}

impl Ready {
    pub fn content(&self) -> &Arc<Content> {
        &self.loaded.game.content
    }

    /// The game's graphics in `lang`: its pack's lettering and the
    /// content's strings in it, else the content's own (a language the
    /// content or the pack hasn't).
    pub fn graphics(&self, lang: &str) -> Graphics {
        let lang = if locale::languages(&self.loaded.game.dir).iter().any(|l| l == lang) { lang } else { locale::OWN };
        if let Some(g) = self.graphics.borrow().get(lang) {
            return g.clone();
        }
        let g = match self.loaded.game.graphics(lang) {
            Ok(g) => g,
            // (A pack without the language's lettering: its own.)
            Err(_) => self.loaded.graphics.clone(),
        };
        self.graphics.borrow_mut().insert(lang.to_string(), g.clone());
        g
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
    /// Why none can be listed (no content, no packs directory).
    pub problem: Option<String>,
}

impl Games {
    /// The content's games, each with its asset pack if one is found.
    pub fn find() -> Games {
        let found = match game::Found::find(&nettai_content::pack::packs_dir(), &[]) {
            Ok(f) => f,
            Err(e) => return Games { list: Vec::new(), problem: Some(format!("{e}: {}", e.report)) },
        };
        match nettai_content::pack::games(None, &found.packs) {
            Ok(choices) => Games {
                list: choices
                    .into_iter()
                    .map(|c| Entry { state: if c.pack.is_some() { State::NotLoaded } else { State::NoPack(c.why_not.unwrap_or_default()) }, id: c.game })
                    .collect(),
                problem: None,
            },
            Err(report) => Games { list: Vec::new(), problem: Some(report.to_string()) },
        }
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.list.iter().find(|e| e.id == id)
    }

    pub fn ready(&self, id: &str) -> Option<Rc<Ready>> {
        match self.get(id).map(|e| &e.state) {
            Some(State::Ready(r)) => Some(r.clone()),
            _ => None,
        }
    }

    /// Start loading `id` on a thread, if it isn't loaded or loading;
    /// `done` gets it on the UI's thread.
    pub fn load(&mut self, id: &str, lang: &str, done: impl FnOnce(String, Result<Loaded, String>) + Send + 'static) {
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        if !matches!(entry.state, State::NotLoaded | State::Failed(_)) {
            return;
        }
        entry.state = State::Loading;
        let id = id.to_string();
        let options = Options { lang: lang.to_string(), ..Options::default() };
        std::thread::spawn(move || {
            let loaded = game::load(&id, &options).map_err(|e| format!("{e}\n{}", e.report));
            let _ = slint::invoke_from_event_loop(move || done(id, loaded));
        });
    }

    /// A game loaded (or not) on its thread.
    pub fn loaded(&mut self, id: &str, loaded: Result<Loaded, String>) {
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        entry.state = match loaded {
            Ok(loaded) => State::Ready(Rc::new(Ready { loaded, graphics: Default::default() })),
            Err(why) => State::Failed(why),
        };
    }
}

/// Names in a language: its strings table's where it has them, else the
/// content's own, else the definition's key; one line (a two-line name
/// joined; the games' stacked marks, EX and DS, as their letters).
pub struct Names<'a> {
    pub content: &'a Content,
    pub strings: Option<&'a nettai_battle::content::strings::Strings>,
}

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

    pub fn navi(&self, navi: nettai_content_api::NaviHandle) -> String {
        let key = &self.content.defs.navi(navi).key;
        self.strings
            .and_then(|s| s.navi(key))
            .and_then(|n| n.name.clone())
            .or_else(|| self.content.strings.navi(key).and_then(|n| n.name.clone()))
            .map(|s| Names::line(&s))
            .unwrap_or_else(|| key.clone())
    }
}
