//! The games the app plays: those the content has (`pack::games`), each
//! loaded on a thread of its own the first time it's wanted and kept, and
//! its graphics in each language shown, loaded as each is first asked for.
//! A game loads in steps (its packs found, its content, its graphics, the
//! text's font, its sound) and says how far it is after each: the loading
//! screen's bar. Why a game can't be listed or loaded goes to the log.

use nettai_battle::Content;
use nettai_frontend::game::{self, Graphics, Loaded, TextMode};
use nettai_battle::content::strings::Strings;
use nettai_content::locale;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Where a game is.
pub enum State {
    /// No asset pack of it is found.
    NoPack,
    NotLoaded,
    /// Loading: how far, 0 to 1.
    Loading(f32),
    Ready(Rc<Ready>),
    /// It couldn't be loaded (why is in the log).
    Failed,
}

/// How far a game's loading is after each of its steps: its packs found, its content, its graphics, the
/// text's font, its sound.
const STEPS: [f32; 5] = [0.05, 0.55, 0.8, 0.85, 1.0];

/// What a loading game tells the UI's thread: how far it is, then the game (or why there is none).
pub enum Step {
    Progress(f32),
    Done(Result<Box<Loaded>, String>),
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
}

impl Games {
    /// The content's games, each with its asset pack if one is found.
    pub fn find() -> Games {
        let found = match game::Found::find(&nettai_content::pack::packs_dir(), &[]) {
            Ok(f) => f,
            Err(e) => {
                crate::errors::log("finding the packs", &format_args!("{e}: {}", e.report));
                return Games { list: Vec::new() };
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
            },
            Err(report) => {
                crate::errors::log("listing the games", &report);
                Games { list: Vec::new() }
            }
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

    /// The games that can be played or may yet be: those with a pack that hasn't failed to load.
    pub fn playable(&self) -> Vec<String> {
        self.list.iter().filter(|e| !matches!(e.state, State::NoPack | State::Failed)).map(|e| e.id.clone()).collect()
    }

    /// How far `id`'s loading is, 0 to 1 (1: it is ready).
    pub fn progress(&self, id: &str) -> f32 {
        match self.get(id).map(|e| &e.state) {
            Some(State::Loading(p)) => *p,
            Some(State::Ready(_)) => 1.0,
            _ => 0.0,
        }
    }

    /// Start loading `id` on a thread, if it isn't loaded or loading, to be shown in `lang`. `told` gets each
    /// step's end and then the game, in order, on the UI's thread.
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
            let loaded = load_in_steps(&id, &lang, &|done| tell(Step::Progress(STEPS[done])));
            tell(Step::Done(loaded.map(Box::new).map_err(|e| format!("{e}\n{}", e.report))));
        });
    }

    /// A step of `id`'s loading ended: how far it is now, or the game (or why there is none: to the log).
    pub fn step(&mut self, id: &str, step: Step) {
        let Some(entry) = self.list.iter_mut().find(|e| e.id == id) else { return };
        entry.state = match step {
            Step::Progress(p) => State::Loading(p),
            Step::Done(Ok(loaded)) => State::Ready(Rc::new(Ready { loaded: *loaded, graphics: Default::default() })),
            Step::Done(Err(why)) => {
                crate::errors::log(&format!("loading {id}"), &why);
                State::Failed
            }
        };
    }
}

/// Load game `name` as `game::load` does, to be shown in `lang` (the content's own where it has no strings in
/// it), saying after each step that it is done (by its place in [`STEPS`]).
fn load_in_steps(name: &str, lang: &str, done: &dyn Fn(usize)) -> Result<Loaded, game::LoadError> {
    // (A later step's failure, with the earlier steps' reports before its own.)
    fn after(before: &[&game::Report], mut e: game::LoadError) -> game::LoadError {
        let mut issues: Vec<_> = before.iter().flat_map(|r| r.issues.iter().cloned()).collect();
        issues.append(&mut e.report.issues);
        e.report.issues = issues;
        e
    }
    let found = game::Found::find(&nettai_content::pack::packs_dir(), &[])?;
    done(0);
    let game = game::Game::load(&found, None, name).map_err(|e| after(&[&found.report], e))?;
    done(1);
    let lang = if locale::languages(&game.dir).iter().any(|l| l == lang) { lang } else { locale::OWN };
    let graphics = game.graphics(lang).map_err(|e| after(&[&found.report, &game.report], e))?;
    done(2);
    let so_far = [&found.report, &game.report, &graphics.report];
    let font = game::font(TextMode::Font, None).map_err(|e| after(&so_far, e))?;
    done(3);
    let sound = game.sound().map_err(|e| after(&so_far, e))?;
    done(4);
    Ok(Loaded { found, game, graphics, text: TextMode::Font, font, sound: Some(sound) })
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
