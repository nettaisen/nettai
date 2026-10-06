//! The editor's state and what each message does to it.

use crate::editor::names::{Lang, Names};
use crate::editor::pictures::Pictures;
use iced::Task;
use nettai_battle::Content;
use nettai_battle::content::ChipCode;
use nettai_battle::custom::FolderChip;
use nettai_battle::setup::NaviStats;
use nettai_content_api::{ChipHandle, NaviHandle, StageHandle};
use nettai_frontend::game::Game;
use nettai_match::{Match, Side};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Where the editor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Arena,
    Navi(usize),
    Folder(usize),
    /// A list the game's rules take of a side that no pane of theirs
    /// shows, by its place among the game's facts (`crate::editor::facts`).
    List(usize, usize),
    /// A pane the game's rules declare (`crate::editor::panes`), by its
    /// place among them.
    Pane(usize, usize),
    AutoBattle(usize),
    Stats(usize),
}

impl Tab {
    /// The tab by its name (`--tab`): `arena`, or `left-` or `right-` and
    /// `navi`, `folder`, `auto-battle`, `stats`, a pane's title as the
    /// rules declare it (EXE6's `crosses`, `patch_cards`, `navicust`,
    /// `sp_times`; `-` and `_` alike), or the name of a list the rules take
    /// of a side that no pane shows.
    pub fn from_name(content: &Content, name: &str) -> Option<Tab> {
        if name == "arena" {
            return Some(Tab::Arena);
        }
        let (side, pane) = name.split_once('-')?;
        let side = match side {
            "left" => 0,
            "right" => 1,
            _ => return None,
        };
        Some(match pane {
            "navi" => Tab::Navi(side),
            "folder" => Tab::Folder(side),
            "auto-battle" => Tab::AutoBattle(side),
            "stats" => Tab::Stats(side),
            other => {
                let panes = nettai_match::panes::panes(content).unwrap_or_default();
                match panes.iter().position(|p| p.title.replace('_', "-") == other.replace('_', "-")) {
                    Some(i) => Tab::Pane(side, i),
                    None => Tab::List(side, crate::editor::facts::list_named(content, other)?),
                }
            }
        })
    }
}

/// An item of a pick list: what it shows, and what it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice<T> {
    pub label: String,
    pub value: T,
}

impl<T> std::fmt::Display for Choice<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

#[derive(Clone, Debug)]
pub enum Msg {
    Tab(Tab),
    /// A new match: the window asks which game it is of ([`App`]).
    New,
    /// The game chosen for a new match.
    Choose(String),
    Open,
    Save,
    SaveAs,
    /// A random match (live play's draw) from this seed.
    Draw,
    /// Play the match in the window (the window's: `crate::window`, which
    /// asks [`Editor::to_play`]).
    Play,
    Lang(Lang),
    // The arena.
    /// A place's stage (0 the first round's, 1 and 2 the later rounds').
    Stage(usize, Choice<StageHandle>),
    Background(usize, Choice<Option<String>>),
    LaterSame(bool),
    Seed(String),
    /// The match's game: a new match of it (the sides start over).
    Game(Choice<String>),
    // A side.
    Navi(usize, Choice<NaviHandle>),
    /// One of a side's facts (what its game's rules take of it), by its
    /// setup field's name.
    Fact(usize, String, crate::editor::facts::Edit),
    Level(usize, String),
    /// The side from a save file (an EXE6 save's version, unlocks, navi code
    /// level and SP times; an EXE5 save's karma, souls and auto battle
    /// data), into a match of the save's game.
    ImportSave(usize),
    // The folder.
    Entry(usize, usize),
    Put(usize, ChipHandle, ChipCode),
    /// The selected entry emptied.
    ClearEntry(usize),
    Regular(usize),
    Tag(usize),
    Search(String),
    /// A side's fact through a pane's view (the pane's path).
    Pane(usize, String, crate::editor::panes::Edit),
    // EXE5's auto battle data.
    AutoBattle(usize, crate::editor::auto_battle::Edit),
    // --screenshot.
    Frame,
    Shot(iced::window::Screenshot),
}

/// How the editor was started.
#[derive(Clone)]
pub struct Options {
    /// The content directory given (`--content`); else the repository's.
    pub content: Option<PathBuf>,
    /// The content directory loaded, and the game loaded (its strings
    /// tables).
    pub content_dir: PathBuf,
    pub games: Vec<String>,
    /// The packs given by directory (`--pack`), each in place of the found
    /// one of its game.
    pub packs: Vec<PathBuf>,
    /// The match file opened (`--edit`); none, a new match, whose game the
    /// window asks.
    pub file: Option<PathBuf>,
    pub lang: Lang,
    /// The pane to start on, by its name (`--tab`, `Tab::from_name`: a
    /// game's lists are known once its content is loaded).
    pub tab: Option<String>,
    pub screenshot: Option<PathBuf>,
}

/// The editor's window: a match being edited, or, before there is one, the
/// choice of its game. No game is preselected: a new match is of the game
/// chosen for it (the first screen, `--game`, or New), and an opened file
/// is of the game it names.
pub struct App {
    /// How the editor was started (a match's own options are a copy).
    pub options: Options,
    /// The games a match can be of: those whose asset pack is found.
    pub games: Vec<String>,
    pub editor: Option<Editor>,
    /// What the choice came to, when it made no match.
    pub status: String,
    frames: u32,
}

impl App {
    /// The window, on `editor` (a match opened or started on the command
    /// line), else on the choice of a game.
    pub fn new(options: Options, editor: Option<Editor>) -> App {
        let games = crate::editor::load::games(options.content.as_deref(), &options.packs);
        App { options, games, editor, status: String::new(), frames: 0 }
    }

    pub fn title(&self) -> String {
        match &self.editor {
            Some(e) => e.title(),
            None => "nettai editor: a new match".into(),
        }
    }

    /// A match of `game` to edit: `file`, or a new one.
    fn start(&mut self, game: &str, file: Option<PathBuf>) -> Result<(), String> {
        let loaded = crate::editor::load::load_game(self.options.content.as_deref(), &self.options.packs, game)?;
        let mut options = self.options.clone();
        options.file = file;
        (options.content_dir, options.games) = (loaded.game.dir.clone(), vec![loaded.game.name.clone()]);
        self.editor = Some(Editor::new(loaded.game, loaded.pictures, options));
        Ok(())
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        match (msg, &mut self.editor) {
            // A new match: of which game is asked again.
            (Msg::New, _) => {
                self.editor = None;
                self.status = String::new();
            }
            (Msg::Choose(game), _) => {
                self.status = match self.start(&game, None) {
                    Ok(()) => String::new(),
                    Err(e) => e,
                };
            }
            (msg, Some(editor)) => return editor.update(msg),
            // Before there is a match: a file opened is of the game it names.
            (Msg::Open, None) => {
                if let Some(path) = rfd::FileDialog::new().add_filter("match", &["toml"]).pick_file() {
                    let game = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| nettai_match::file::game_of(&t));
                    if let Err(e) = game.and_then(|game| self.start(&game, Some(path.clone()))) {
                        self.status = format!("can't open {}: {e}", path.display());
                    }
                }
            }
            (Msg::Frame, None) => {
                self.frames += 1;
                if self.frames == 20 && self.options.screenshot.is_some() {
                    return iced::window::latest().and_then(iced::window::screenshot).map(Msg::Shot);
                }
            }
            (Msg::Shot(shot), None) => {
                if let Some(path) = &self.options.screenshot {
                    match save_png(path, &shot) {
                        Ok(()) => eprintln!("wrote {}", path.display()),
                        Err(e) => eprintln!("can't write {}: {e}", path.display()),
                    }
                }
                return iced::exit();
            }
            (_, None) => {}
        }
        Task::none()
    }
}

pub struct Editor {
    /// The game the match is of, loaded as the player loads it (Play plays
    /// the match on it).
    pub game: Game,
    pub content: Arc<Content>,
    pub pictures: Pictures,
    pub names: Names,
    pub lang: Lang,
    pub options: Options,
    pub path: Option<PathBuf>,
    pub m: Match,
    pub dirty: bool,
    pub tab: Tab,
    /// The folder entry each side's chip list puts chips into.
    pub entry: [usize; 2],
    pub search: String,
    /// The games a match can be of.
    pub games: Vec<String>,
    /// What is typed into number fields, by field (side, name), until it
    /// reads as a number.
    pub typed: HashMap<(usize, &'static str), String>,
    /// What is typed into a pane's number fields (side, the view's path,
    /// or a row's column's), until it reads.
    pub pane_typed: HashMap<(usize, String), String>,
    /// The game's rules' panes of a side's setup.
    pub panes: Vec<nettai_match::panes::Pane>,
    /// What their functions answered for each side.
    pub answers: [crate::editor::panes::Answers; 2],
    /// Each side's grids' own state, by their views' paths.
    pub grids: [HashMap<String, crate::editor::panes::GridState>; 2],
    /// What is typed into a side's facts' number fields (side, the fact's
    /// name), until it parses.
    pub fact_typed: HashMap<(usize, String), String>,
    /// What is wrong with the match (none: it can be played).
    pub problems: Vec<String>,
    /// The stats each side's round starts with (after the NaviCust and the
    /// patch cards), or why the round doesn't start.
    pub round: Result<[NaviStats; 2], String>,
    /// The chips each side's rules let a folder hold.
    pub pool: [Vec<ChipHandle>; 2],
    /// The game's library order, which the lists of chips, programs and
    /// cards keep.
    pub order: crate::editor::order::Order,
    /// Each side's Auto battle pane's own state.
    pub auto_battle: [crate::editor::auto_battle::State; 2],
    pub status: String,
    frames: u32,
}

impl Editor {
    pub fn new(game: Game, pictures: Pictures, options: Options) -> Editor {
        let content = game.content.clone();
        // A new match is an empty one of the content's game (Random draws
        // one as live play does).
        let new = |content: &Arc<Content>| {
            nettai_match::Match::empty(content, content.game()).unwrap_or_else(|e| {
                eprintln!("the content makes no match: {e}");
                std::process::exit(1)
            })
        };
        let m = match &options.file {
            Some(path) => match std::fs::read_to_string(path).map_err(|e| vec![e.to_string()]).and_then(|t| read(&content, &t)) {
                Ok(m) => m,
                Err(problems) => {
                    eprintln!("{}: {}", path.display(), problems.join("; "));
                    new(&content)
                }
            },
            None => new(&content),
        };
        let mut e = Editor {
            names: Names::default(),
            lang: Lang::En,
            path: options.file.clone(),
            tab: Tab::Arena,
            m,
            dirty: false,
            entry: [0, 0],
            search: String::new(),
            games: crate::editor::load::games(options.content.as_deref(), &options.packs),
            typed: HashMap::new(),
            pane_typed: HashMap::new(),
            panes: Vec::new(),
            answers: Default::default(),
            grids: Default::default(),
            fact_typed: HashMap::new(),
            problems: Vec::new(),
            round: Err(String::new()),
            pool: Default::default(),
            order: Default::default(),
            auto_battle: Default::default(),
            status: String::new(),
            frames: 0,
            game,
            content,
            pictures,
            options,
        };
        if let Some(name) = e.options.tab.clone() {
            match Tab::from_name(&e.content, &name) {
                Some(tab) => e.tab = tab,
                None => e.status = format!("no pane {name:?} (--tab)"),
            }
        }
        e.set_lang(e.options.lang);
        e.load_order();
        e.load_panes();
        e.refresh();
        e
    }

    /// The file's name, and whether it has changes not saved.
    pub fn file_name(&self) -> String {
        let name = self.path.as_ref().and_then(|p| p.file_name()).map_or("untitled".into(), |n| n.to_string_lossy().into_owned());
        format!("{name}{}", if self.dirty { " (changed)" } else { "" })
    }

    pub fn title(&self) -> String {
        let name = self.path.as_ref().map_or("untitled".to_string(), |p| p.display().to_string());
        format!("nettai editor: {name}{}", if self.dirty { " *" } else { "" })
    }

    fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
        self.names.other = match lang {
            Lang::En => None,
            Lang::Ja => match nettai_content::locale::load_for(&self.options.content_dir, &self.options.games, lang.code()) {
                Ok(Some(s)) => Some(s),
                _ => {
                    self.status = format!("the content has no {} names", lang.code());
                    None
                }
            },
        };
    }

    /// The game's library order (its pack's library.toml).
    fn load_order(&mut self) {
        let (order, problem) = crate::editor::order::Order::load(&self.options.content_dir, self.content.game());
        self.order = order;
        if let Some(p) = problem {
            self.status = format!("the lists are by key: {p}");
        }
    }

    /// The game's rules' panes (a load's).
    fn load_panes(&mut self) {
        match nettai_match::panes::panes(&self.content) {
            Ok(p) => self.panes = p,
            Err(e) => {
                self.panes = Vec::new();
                self.status = format!("the rules' panes: {e}");
            }
        }
    }

    /// Check the match again, the stats its round starts with, and what
    /// the rules' panes' functions say of each side on it.
    pub fn refresh(&mut self) {
        self.problems = nettai_match::check_match(&self.content, &self.m);
        match nettai_match::check::start(&self.content, &self.m) {
            Ok(mut b) => {
                self.pool = [0u8, 1].map(|s| nettai_match::folders::pool(&self.content, self.m.game(), &mut b, s));
                self.round = Ok(b.stats);
                let content = self.content.clone();
                self.answers = [0, 1].map(|s| crate::editor::panes::ask(&content, &self.panes, &mut b, s));
            }
            // (No round: nothing says which chips its rules let a folder
            // hold, nor what its panes' functions say.)
            Err(e) => {
                self.pool = Default::default();
                self.answers = [0, 1].map(|_| crate::editor::panes::Answers { unasked: Some(e.clone()), ..Default::default() });
                self.round = Err(e);
            }
        }
    }

    fn edited(&mut self) {
        self.dirty = true;
        self.refresh();
    }

    /// Have `game`'s content, loading it if the content loaded hasn't it.
    fn content_of(&mut self, game: &str) -> Result<Arc<Content>, String> {
        if !crate::editor::load::holds(&self.content, game) {
            let loaded = crate::editor::load::load_game(self.options.content.as_deref(), &self.options.packs, game)?;
            self.content = loaded.game.content.clone();
            self.pictures = loaded.pictures;
            (self.options.content_dir, self.options.games) = (loaded.game.dir.clone(), vec![loaded.game.name.clone()]);
            self.game = loaded.game;
            self.set_lang(self.lang);
            self.load_order();
            self.load_panes();
        }
        Ok(self.content.clone())
    }

    /// Forget what was typed and picked for the sides (they are new).
    fn forget_sides(&mut self) {
        self.typed.retain(|&(s, _), _| s > 1);
        self.pane_typed.clear();
        self.fact_typed.clear();
        self.entry = [0, 0];
        // (A list's pane is its game's, and a declared one its rules'.)
        if matches!(self.tab, Tab::List(..) | Tab::Pane(..)) {
            self.tab = Tab::Arena;
        }
        self.grids = Default::default();
        self.auto_battle = Default::default();
    }

    pub fn side(&self, s: usize) -> &Side {
        &self.m.sides[s]
    }

    fn write_to(&mut self, path: PathBuf) {
        match std::fs::write(&path, nettai_match::write(&self.content, &self.m)) {
            Ok(()) => {
                self.status = format!("saved {}", path.display());
                self.path = Some(path);
                self.dirty = false;
            }
            Err(e) => self.status = format!("can't save {}: {e}", path.display()),
        }
    }

    /// The match to play and its game, if it can be played (else the
    /// status says why): the window plays it (`crate::window`). Its seed is
    /// the match's, else one from the clock.
    pub fn to_play(&mut self) -> Option<(Game, Match, u32)> {
        if !self.problems.is_empty() {
            self.status = "the match can't be played yet: see the problems".into();
            return None;
        }
        let seed = self.m.seed.unwrap_or_else(|| {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1)
        });
        self.status = format!("playing (seed {seed}): Esc comes back here");
        Some((self.game.clone(), self.m.clone(), seed))
    }

    pub fn update(&mut self, msg: Msg) -> Task<Msg> {
        let content = self.content.clone();
        match msg {
            Msg::Tab(t) => self.tab = t,
            // (The window's: a new match asks its game, `App::update`.)
            Msg::New | Msg::Choose(_) => {}
            Msg::Open => {
                if let Some(path) = rfd::FileDialog::new().add_filter("match", &["toml"]).pick_file() {
                    // (The file's game's content: loaded if it is another's.)
                    let opened = std::fs::read_to_string(&path).map_err(|e| vec![e.to_string()]).and_then(|t| {
                        let game = nettai_match::file::game_of(&t).map_err(|e| vec![e])?;
                        let content = self.content_of(&game).map_err(|e| vec![e])?;
                        read(&content, &t)
                    });
                    match opened {
                        Ok(m) => {
                            self.m = m;
                            self.path = Some(path);
                            self.dirty = false;
                            self.forget_sides();
                            self.typed.clear();
                            self.refresh();
                            self.status = "opened".into();
                        }
                        Err(p) => self.status = format!("can't open {}: {}", path.display(), p.join("; ")),
                    }
                }
            }
            Msg::Save => match self.path.clone() {
                Some(p) => self.write_to(p),
                None => return self.update(Msg::SaveAs),
            },
            Msg::SaveAs => {
                if let Some(path) = rfd::FileDialog::new().add_filter("match", &["toml"]).set_file_name("match.toml").save_file() {
                    self.write_to(path);
                }
            }
            Msg::Draw => {
                let seed = self.m.seed.unwrap_or(1).wrapping_mul(0x2545_F491).wrapping_add(7);
                if let Ok(m) = nettai_match::pick::live(&content, self.m.game(), seed, None) {
                    self.m = m;
                    self.forget_sides();
                    self.edited();
                }
            }
            // (The window plays it: `crate::window`.)
            Msg::Play => {}
            Msg::Lang(l) => self.set_lang(l),
            Msg::Stage(i, c) => {
                match i {
                    0 => self.m.arena.first.stage = c.value,
                    i => self.m.arena.later[i - 1].stage = c.value,
                }
                self.edited();
            }
            Msg::Background(i, c) => {
                match i {
                    0 => self.m.arena.first.background = c.value,
                    i => self.m.arena.later[i - 1].background = c.value,
                }
                self.edited();
            }
            Msg::LaterSame(same) => {
                if same {
                    self.m.arena.later = [self.m.arena.first.clone(), self.m.arena.first.clone()];
                } else if let Ok(a) = nettai_match::pick::arena(&content, &self.m.arena.game, &mut nettai_match::Picks::new(self.m.seed.unwrap_or(1)), None) {
                    self.m.arena.later = a.later;
                }
                self.edited();
            }
            Msg::Seed(t) => {
                self.m.seed = t.trim().parse().ok();
                self.typed.insert((2, "seed"), t);
                self.edited();
            }
            Msg::Game(c) => {
                // Everything below the game is the game's: a new match of
                // it, the seed kept.
                if c.value != self.m.arena.game {
                    match self.content_of(&c.value).and_then(|content| nettai_match::Match::empty(&content, &c.value)) {
                        Ok(mut m) => {
                            m.seed = self.m.seed;
                            self.m = m;
                            self.forget_sides();
                            self.status = format!("a match of {}: the sides start over", c.value);
                            self.edited();
                        }
                        Err(e) => self.status = format!("can't make a match of {}: {e}", c.value),
                    }
                }
            }
            Msg::Navi(s, c) => {
                crate::editor::levels::switch_navi(&content, &mut self.m.sides[s], c.value);
                self.typed.remove(&(s, "level"));
                self.fact_typed.retain(|(x, _), _| *x != s);
                self.edited();
            }
            Msg::Fact(s, name, edit) => {
                let game = self.m.arena.game.clone();
                let changed = crate::editor::facts::apply(&content, &game, &mut self.m.sides[s], &name, &edit);
                // (What is typed stays as typed until it is a number the
                // fact takes; any other edit shows the fact's value.)
                match edit {
                    crate::editor::facts::Edit::Number(typed) => {
                        self.fact_typed.insert((s, name), typed);
                    }
                    _ => {
                        self.fact_typed.remove(&(s, name));
                    }
                }
                if changed {
                    self.edited();
                }
            }
            Msg::Level(s, t) => {
                // A level, or none (MegaMan without a navi code; a link navi
                // always has one).
                let link_navi = !content.navi(self.m.sides[s].navi(&content)).changes_form();
                let level = match t.trim() {
                    "" if !link_navi => Some(None),
                    t => t.parse::<u8>().ok().map(Some),
                };
                if let Some(v) = level
                    && self.m.sides[s].set_level(&content, v).is_ok()
                {
                    self.edited();
                }
                self.typed.insert((s, "level"), t);
            }
            Msg::ImportSave(s) => {
                // An EXE6 save, or an EXE5 one (a .sav, or a raw image as
                // Tango's netplay templates hold): a match of its game.
                if let Some(path) = rfd::FileDialog::new().add_filter("EXE6 or EXE5 save", &["sav", "raw"]).pick_file() {
                    let read = std::fs::read(&path).map_err(|e| e.to_string());
                    let game = self.m.arena.game.clone();
                    // (The save's game's content: loaded if it is another's.)
                    let imported = read.and_then(|bytes| {
                        let content = self.content_of(nettai_match::save_game(&bytes)?)?;
                        self.m.import_save(&content, s, &bytes)
                    });
                    match imported {
                        Ok(notes) => {
                            if self.m.arena.game != game {
                                self.forget_sides();
                            }
                            self.typed.retain(|&(x, _), _| x != s);
                            self.pane_typed.retain(|(x, _), _| *x != s);
                            self.fact_typed.retain(|(x, _), _| *x != s);
                            self.edited();
                            let notes = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                            self.status = format!("imported {}{notes}", path.display());
                        }
                        Err(e) => self.status = format!("can't import {}: {e}", path.display()),
                    }
                }
            }
            Msg::Entry(s, i) => self.entry[s] = i,
            Msg::Put(s, chip, code) => {
                let i = self.entry[s];
                let mut f = self.m.sides[s].folder(&content);
                f.chips[i] = Some(FolderChip::new(chip, code));
                if self.m.sides[s].set_folder(&content, &f).is_ok() {
                    self.edited();
                }
                // On to the next entry, as one fills a folder.
                self.entry[s] = (i + 1) % f.chips.len();
            }
            Msg::ClearEntry(s) => {
                let i = self.entry[s] as u8;
                let mut f = self.m.sides[s].folder(&content);
                f.chips[i as usize] = None;
                // It is no longer the Regular or a tag chip.
                if f.regular == Some(i) {
                    f.regular = None;
                }
                if f.tags.is_some_and(|(a, b)| a == i || b == i) {
                    f.tags = None;
                }
                if self.m.sides[s].set_folder(&content, &f).is_ok() {
                    self.edited();
                }
            }
            Msg::Regular(s) => {
                let i = self.entry[s] as u8;
                let mut f = self.m.sides[s].folder(&content);
                f.regular = if f.regular == Some(i) { None } else { Some(i) };
                if self.m.sides[s].set_folder(&content, &f).is_ok() {
                    self.edited();
                }
            }
            Msg::Tag(s) => {
                let i = self.entry[s] as u8;
                let mut f = self.m.sides[s].folder(&content);
                f.tags = match f.tags {
                    Some((a, b)) if a == i || b == i => None,
                    // A tag chip waiting for its pair is tagged with itself.
                    Some((a, b)) if a == b => Some((a.min(i), a.max(i))),
                    _ => Some((i, i)),
                };
                if self.m.sides[s].set_folder(&content, &f).is_ok() {
                    self.edited();
                }
            }
            Msg::Search(t) => self.search = t,
            Msg::Pane(s, path, edit) => {
                if crate::editor::panes::apply(self, s, &path, edit) {
                    self.edited();
                }
            }
            // The auto battle data of an EXE5 save alone (a .sav, or a raw
            // image): the side's other things stay.
            Msg::AutoBattle(s, crate::editor::auto_battle::Edit::FromSave) => {
                if let Some(path) = rfd::FileDialog::new().add_filter("EXE5 save", &["sav", "raw"]).pick_file() {
                    let read = std::fs::read(&path).map_err(|e| e.to_string());
                    match read.and_then(|bytes| nettai_match::auto_battle::of_save(&content, self.m.game(), &bytes)) {
                        Ok((data, notes)) => match data.write(&content, &mut self.m.sides[s]) {
                            Ok(()) => {
                                self.auto_battle[s] = Default::default();
                                self.edited();
                                let notes = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                                self.status = format!("took the auto battle data of {}{notes}", path.display());
                            }
                            Err(e) => self.status = format!("can't take the auto battle data of {}: {e}", path.display()),
                        },
                        Err(e) => self.status = format!("can't take the auto battle data of {}: {e}", path.display()),
                    }
                }
            }
            Msg::AutoBattle(s, edit) => {
                if crate::editor::auto_battle::update(&content, &mut self.m.sides[s], &mut self.auto_battle[s], edit) {
                    self.edited();
                }
            }
            Msg::Frame => {
                self.frames += 1;
                if self.frames == 20 && self.options.screenshot.is_some() {
                    return iced::window::latest().and_then(iced::window::screenshot).map(Msg::Shot);
                }
            }
            Msg::Shot(shot) => {
                if let Some(path) = &self.options.screenshot {
                    match save_png(path, &shot) {
                        Ok(()) => eprintln!("wrote {}", path.display()),
                        Err(e) => eprintln!("can't write {}: {e}", path.display()),
                    }
                }
                return iced::exit();
            }
        }
        Task::none()
    }
}

/// A match file's text, resolved and as it is (its problems are the
/// editor's to show, not a reason not to open it).
fn read(content: &Content, text: &str) -> Result<Match, Vec<String>> {
    let f: nettai_match::file::MatchFile = toml::from_str(text).map_err(|e| vec![e.to_string()])?;
    nettai_match::file::resolve(content, &f)
}

fn save_png(path: &std::path::Path, shot: &iced::window::Screenshot) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), shot.size.width, shot.size.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(&shot.rgba).map_err(|e| e.to_string())
}

