//! The editor's state and what each message does to it.

use crate::names::{Lang, Names};
use crate::pictures::Pictures;
use iced::Task;
use nettai_battle::Content;
use nettai_battle::content::ChipCode;
use nettai_battle::custom::FolderChip;
use nettai_battle::patch_cards::InstalledCard;
use nettai_battle::setup::NaviStats;
use nettai_content_api::{ChipHandle, NaviHandle, PatchCardHandle, StageHandle};
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
    /// A list the game's rules take of a side (EXE6's Crosses, EXE5's
    /// souls), by its place among the game's facts (`crate::facts`).
    List(usize, usize),
    AutoBattle(usize),
    Cards(usize),
    NaviCust(usize),
    Stats(usize),
}

impl Tab {
    /// The tab by its name (`--tab`): `arena`, or `left-` or `right-` and
    /// `navi`, `folder`, `auto-battle`, `cards`, `navicust`, `stats`, or
    /// the name of a list the content's game's rules take of a side
    /// (`crosses`, `souls`).
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
            "cards" => Tab::Cards(side),
            "navicust" => Tab::NaviCust(side),
            "stats" => Tab::Stats(side),
            list => Tab::List(side, crate::facts::list_named(content, list)?),
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
    Fact(usize, String, crate::facts::Edit),
    Level(usize, String),
    /// An SP navi's deletion time (by its slot), as typed.
    SpTime(usize, usize, String),
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
    // The patch cards.
    AddCard(usize, PatchCardHandle),
    CardOn(usize, usize, bool),
    CardMove(usize, usize, bool),
    CardRemove(usize, usize),
    // The NaviCust.
    NaviCust(usize, crate::navicust::Edit),
    // EXE5's auto battle data.
    AutoBattle(usize, crate::auto_battle::Edit),
    // --screenshot.
    Frame,
    Shot(iced::window::Screenshot),
}

/// How the editor was started.
#[derive(Clone)]
pub struct Options {
    /// The content directory given (`--content`), which Play hands the
    /// frontend too; else the repository's.
    pub content: Option<PathBuf>,
    /// The content directory loaded, and the game loaded (its strings
    /// tables).
    pub content_dir: PathBuf,
    pub games: Vec<String>,
    /// The packs given by directory (`--pack`), each in place of the found
    /// one of its game, which Play hands the frontend too.
    pub packs: Vec<PathBuf>,
    pub frontend: Option<PathBuf>,
    pub file: Option<PathBuf>,
    /// The game of a new match, when the command line says it (`--game`):
    /// else the window asks.
    pub game: Option<String>,
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
        let games = crate::load::games(options.content.as_deref(), &options.packs);
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
        let loaded = crate::load::load_game(self.options.content.as_deref(), &self.options.packs, game)?;
        let mut options = self.options.clone();
        options.file = file;
        (options.content_dir, options.games) = (loaded.dir, vec![loaded.game]);
        self.editor = Some(Editor::new(loaded.content, loaded.pictures, options));
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
    /// What is typed into each side's SP deletion times (side, slot), until
    /// it reads as a time.
    pub sp_typed: HashMap<(usize, usize), String>,
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
    /// Each side's NaviCust pane's own state.
    pub navicust: [crate::navicust::State; 2],
    /// Each side's Auto battle pane's own state.
    pub auto_battle: [crate::auto_battle::State; 2],
    pub status: String,
    frames: u32,
}

impl Editor {
    pub fn new(content: Arc<Content>, pictures: Pictures, options: Options) -> Editor {
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
            games: crate::load::games(options.content.as_deref(), &options.packs),
            typed: HashMap::new(),
            sp_typed: HashMap::new(),
            fact_typed: HashMap::new(),
            problems: Vec::new(),
            round: Err(String::new()),
            pool: Default::default(),
            navicust: Default::default(),
            auto_battle: Default::default(),
            status: String::new(),
            frames: 0,
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

    /// Check the match again, and the stats its round starts with.
    pub fn refresh(&mut self) {
        self.problems = nettai_match::check_match(&self.content, &self.m);
        match nettai_match::check::start(&self.content, &self.m) {
            Ok(mut b) => {
                self.pool = [0u8, 1].map(|s| nettai_match::folders::pool(&self.content, self.m.game(), &mut b, s));
                self.round = Ok(b.stats);
            }
            // (No round: nothing says which chips its rules let a folder
            // hold.)
            Err(e) => {
                self.pool = Default::default();
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
        if !crate::load::holds(&self.content, game) {
            let loaded = crate::load::load_game(self.options.content.as_deref(), &self.options.packs, game)?;
            self.content = loaded.content;
            self.pictures = loaded.pictures;
            (self.options.content_dir, self.options.games) = (loaded.dir, vec![loaded.game]);
            self.set_lang(self.lang);
        }
        Ok(self.content.clone())
    }

    /// Forget what was typed and picked for the sides (they are new).
    fn forget_sides(&mut self) {
        self.typed.retain(|&(s, _), _| s > 1);
        self.sp_typed.clear();
        self.fact_typed.clear();
        self.entry = [0, 0];
        // (A list's pane is its game's.)
        if matches!(self.tab, Tab::List(..)) {
            self.tab = Tab::Arena;
        }
        self.navicust = Default::default();
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

    /// The frontend's program: the option's, else beside the editor's.
    fn frontend(&self) -> PathBuf {
        if let Some(p) = &self.options.frontend {
            return p.clone();
        }
        let exe = std::env::current_exe().ok().map(|e| e.with_file_name(format!("nettai-demo{}", std::env::consts::EXE_SUFFIX)));
        exe.filter(|p| p.exists()).unwrap_or_else(|| PathBuf::from("nettai-demo"))
    }

    fn play(&mut self) {
        if !self.problems.is_empty() {
            self.status = "the match can't be played yet: see the problems".into();
            return;
        }
        let path = match (&self.path, self.dirty) {
            (Some(p), false) => p.clone(),
            (Some(p), true) => {
                let p = p.clone();
                self.write_to(p.clone());
                p
            }
            (None, _) => {
                let p = std::env::temp_dir().join("nettai-demo-editor-match.toml");
                if let Err(e) = std::fs::write(&p, nettai_match::write(&self.content, &self.m)) {
                    self.status = format!("can't write {}: {e}", p.display());
                    return;
                }
                p
            }
        };
        let program = self.frontend();
        let mut command = std::process::Command::new(&program);
        command.arg("--match").arg(&path);
        if let Some(root) = &self.options.content {
            command.arg("--content").arg(root);
        }
        for pack in &self.options.packs {
            command.arg("--pack").arg(pack);
        }
        let started = command.arg("--lang").arg(self.lang.code()).spawn();
        self.status = match started {
            Ok(_) => format!("playing {} with {}", path.display(), program.display()),
            Err(e) => format!("can't start {} ({e}): give its path with --frontend", program.display()),
        };
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
            Msg::Play => self.play(),
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
                crate::levels::switch_navi(&content, &mut self.m.sides[s], c.value);
                self.typed.remove(&(s, "level"));
                self.fact_typed.retain(|(x, _), _| *x != s);
                self.edited();
            }
            Msg::Fact(s, name, edit) => {
                let game = self.m.arena.game.clone();
                let changed = crate::facts::apply(&content, &game, &mut self.m.sides[s], &name, &edit);
                // (What is typed stays as typed until it is a number the
                // fact takes; any other edit shows the fact's value.)
                match edit {
                    crate::facts::Edit::Number(typed) => {
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
                let link_navi = !content.navi(self.m.sides[s].navi).changes_form();
                let level = match t.trim() {
                    "" if !link_navi => Some(None),
                    t => t.parse::<u8>().ok().map(Some),
                };
                if let Some(v) = level {
                    self.m.sides[s].navi_level = v;
                    self.edited();
                }
                self.typed.insert((s, "level"), t);
            }
            Msg::SpTime(s, slot, t) => {
                let frames = if t.trim().is_empty() { Ok(0) } else { nettai_match::sp_times::parse(&t) };
                if let Ok(f) = frames {
                    self.m.sides[s].sp_times.0[slot] = f;
                    self.edited();
                }
                self.sp_typed.insert((s, slot), t);
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
                            self.sp_typed.retain(|&(x, _), _| x != s);
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
                self.m.sides[s].folder.chips[i] = Some(FolderChip::new(chip, code));
                // On to the next entry, as one fills a folder.
                self.entry[s] = (i + 1) % self.m.sides[s].folder.chips.len();
                self.edited();
            }
            Msg::ClearEntry(s) => {
                let i = self.entry[s] as u8;
                let f = &mut self.m.sides[s].folder;
                f.chips[i as usize] = None;
                // It is no longer the Regular or a tag chip.
                if f.regular == Some(i) {
                    f.regular = None;
                }
                if f.tags.is_some_and(|(a, b)| a == i || b == i) {
                    f.tags = None;
                }
                self.edited();
            }
            Msg::Regular(s) => {
                let i = self.entry[s] as u8;
                let f = &mut self.m.sides[s].folder;
                f.regular = if f.regular == Some(i) { None } else { Some(i) };
                self.edited();
            }
            Msg::Tag(s) => {
                let i = self.entry[s] as u8;
                let f = &mut self.m.sides[s].folder;
                f.tags = match f.tags {
                    Some((a, b)) if a == i || b == i => None,
                    // A tag chip waiting for its pair is tagged with itself.
                    Some((a, b)) if a == b => Some((a.min(i), a.max(i))),
                    _ => Some((i, i)),
                };
                self.edited();
            }
            Msg::Search(t) => self.search = t,
            Msg::AddCard(s, card) => {
                let cards = &mut self.m.sides[s].cards;
                if !cards.iter().any(|c| c.card == card) {
                    cards.push(InstalledCard { card, enabled: true });
                    self.edited();
                }
            }
            Msg::CardOn(s, i, on) => {
                self.m.sides[s].cards[i].enabled = on;
                self.edited();
            }
            Msg::CardMove(s, i, up) => {
                let cards = &mut self.m.sides[s].cards;
                let j = if up { i.checked_sub(1) } else { (i + 1 < cards.len()).then_some(i + 1) };
                if let Some(j) = j {
                    cards.swap(i, j);
                    self.edited();
                }
            }
            Msg::CardRemove(s, i) => {
                self.m.sides[s].cards.remove(i);
                self.edited();
            }
            Msg::NaviCust(s, edit) => {
                if crate::navicust::update(&content, &self.m.arena, &mut self.m.sides[s], &mut self.navicust[s], edit) {
                    self.edited();
                }
            }
            // The auto battle data of an EXE5 save alone (a .sav, or a raw
            // image): the side's other things stay.
            Msg::AutoBattle(s, crate::auto_battle::Edit::FromSave) => {
                if let Some(path) = rfd::FileDialog::new().add_filter("EXE5 save", &["sav", "raw"]).pick_file() {
                    let read = std::fs::read(&path).map_err(|e| e.to_string());
                    match read.and_then(|bytes| nettai_match::auto_battle::of_save(&content, self.m.game(), &bytes)) {
                        Ok((data, notes)) => {
                            self.m.sides[s].auto_battle = data;
                            self.auto_battle[s] = Default::default();
                            self.edited();
                            let notes = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                            self.status = format!("took the auto battle data of {}{notes}", path.display());
                        }
                        Err(e) => self.status = format!("can't take the auto battle data of {}: {e}", path.display()),
                    }
                }
            }
            Msg::AutoBattle(s, edit) => {
                if crate::auto_battle::update(&content, &mut self.m.sides[s], &mut self.auto_battle[s], edit) {
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
