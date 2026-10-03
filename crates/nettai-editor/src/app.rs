//! The editor's state and what each message does to it.

use crate::names::{Lang, Names};
use crate::pictures::Pictures;
use iced::Task;
use nettai_battle::Content;
use nettai_battle::content::ChipCode;
use nettai_battle::custom::{FolderChip, GameVersion};
use nettai_battle::patch_cards::InstalledCard;
use nettai_battle::setup::NaviStats;
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle, PatchCardHandle, RulesetHandle, StageHandle};
use nettai_match::{CrossList, Match, Side, stats};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Where the editor is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Arena,
    Navi(usize),
    Folder(usize),
    Crosses(usize),
    Cards(usize),
    NaviCust(usize),
    Stats(usize),
}

impl Tab {
    /// The tab by its name (`--tab`): `arena`, or `left-` or `right-` and
    /// `navi`, `folder`, `crosses`, `cards`, `navicust`, `stats`.
    pub fn from_name(name: &str) -> Option<Tab> {
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
            "crosses" => Tab::Crosses(side),
            "cards" => Tab::Cards(side),
            "navicust" => Tab::NaviCust(side),
            "stats" => Tab::Stats(side),
            _ => return None,
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
    /// A new, empty match.
    New,
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
    // A side.
    Ruleset(usize, Choice<Option<RulesetHandle>>),
    Navi(usize, Choice<NaviHandle>),
    Game(usize, Choice<GameVersion>),
    Level(usize, String),
    BugFrags(usize, String),
    Glitch(usize, bool),
    /// An SP navi's deletion time (by its slot), as typed.
    SpTime(usize, usize, String),
    /// The side's game, unlocks, navi code level and SP times from a save
    /// file.
    ImportSave(usize),
    // The folder.
    Entry(usize, usize),
    Put(usize, ChipHandle, ChipCode),
    /// The selected entry emptied.
    ClearEntry(usize),
    Regular(usize),
    Tag(usize),
    Search(String),
    // The Crosses.
    OwnCrosses(usize, bool),
    Cross(usize, FormHandle, bool),
    // The patch cards.
    AddCard(usize, PatchCardHandle),
    CardOn(usize, usize, bool),
    CardMove(usize, usize, bool),
    CardRemove(usize, usize),
    // The stats.
    StatText(usize, &'static str, String),
    StatValue(usize, &'static str, stats::Value),
    StatsReset(usize),
    // The NaviCust.
    NaviCust(usize, crate::navicust::Edit),
    // --screenshot.
    Frame,
    Shot(iced::window::Screenshot),
}

/// How the editor was started.
pub struct Options {
    /// The content directory given (`--content`), which Play hands the
    /// frontend too; else the repository's.
    pub content: Option<PathBuf>,
    /// The content folders loaded (their strings tables).
    pub roots: Vec<PathBuf>,
    /// The packs given by directory (`--pack`), each in place of the found
    /// one of its game, which Play hands the frontend too.
    pub packs: Vec<PathBuf>,
    pub frontend: Option<PathBuf>,
    pub file: Option<PathBuf>,
    pub lang: Lang,
    pub tab: Tab,
    pub screenshot: Option<PathBuf>,
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
    /// What is typed into number fields, by field (side, name), until it
    /// reads as a number.
    pub typed: HashMap<(usize, &'static str), String>,
    /// What is typed into each side's SP deletion times (side, slot), until
    /// it reads as a time.
    pub sp_typed: HashMap<(usize, usize), String>,
    /// What is wrong with the match (none: it can be played).
    pub problems: Vec<String>,
    /// The stats each side's round starts with (after the NaviCust and the
    /// patch cards), or why the round doesn't start.
    pub round: Result<[NaviStats; 2], String>,
    /// The chips each side's rules let a folder hold.
    pub pool: [Vec<ChipHandle>; 2],
    /// Each side's NaviCust pane's own state.
    pub navicust: [crate::navicust::State; 2],
    pub status: String,
    frames: u32,
}

impl Editor {
    pub fn new(content: Arc<Content>, pictures: Pictures, options: Options) -> Editor {
        // A new match is an empty one (Random draws one as live play does).
        let new = |content: &Arc<Content>| {
            nettai_match::Match::empty(content).unwrap_or_else(|e| {
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
            tab: options.tab,
            m,
            dirty: false,
            entry: [0, 0],
            search: String::new(),
            typed: HashMap::new(),
            sp_typed: HashMap::new(),
            problems: Vec::new(),
            round: Err(String::new()),
            pool: Default::default(),
            navicust: Default::default(),
            status: String::new(),
            frames: 0,
            content,
            pictures,
            options,
        };
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
            Lang::Ja => match nettai_content::locale::load_many(&self.options.roots, lang.code()) {
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
                self.pool = [0u8, 1].map(|s| nettai_match::folders::pool(&self.content, &mut b, s));
                self.round = Ok(b.stats);
            }
            Err(e) => self.round = Err(e),
        }
    }

    fn edited(&mut self) {
        self.dirty = true;
        self.refresh();
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
        let exe = std::env::current_exe().ok().map(|e| e.with_file_name(format!("nettai-frontend{}", std::env::consts::EXE_SUFFIX)));
        exe.filter(|p| p.exists()).unwrap_or_else(|| PathBuf::from("nettai-frontend"))
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
                let p = std::env::temp_dir().join("nettai-editor-match.toml");
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
            Msg::New => {
                if let Ok(m) = nettai_match::Match::empty(&content) {
                    self.m = m;
                    self.path = None;
                    self.dirty = false;
                    self.typed.clear();
                    self.entry = [0, 0];
                    self.navicust = Default::default();
                    self.refresh();
                    self.status = "a new match".into();
                }
            }
            Msg::Open => {
                if let Some(path) = rfd::FileDialog::new().add_filter("match", &["toml"]).pick_file() {
                    match std::fs::read_to_string(&path).map_err(|e| vec![e.to_string()]).and_then(|t| read(&content, &t)) {
                        Ok(m) => {
                            self.m = m;
                            self.path = Some(path);
                            self.dirty = false;
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
                if let Ok(m) = nettai_match::draw::live(&content, seed, None).or_else(|_| nettai_match::draw::plain(&content, seed)) {
                    self.m = m;
                    self.typed.clear();
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
                } else if let Ok(a) = nettai_match::draw::arena(&content, &mut nettai_match::Draws::new(self.m.seed.unwrap_or(1)), None) {
                    self.m.arena.later = a.later;
                }
                self.edited();
            }
            Msg::Seed(t) => {
                self.m.seed = t.trim().parse().ok();
                self.typed.insert((2, "seed"), t);
                self.edited();
            }
            Msg::Ruleset(s, c) => {
                let side = &mut self.m.sides[s];
                side.ruleset = c.value;
                // What the new rules don't have goes.
                if !side.has_system(&content, nettai_match::FORMS_SYSTEM) {
                    side.crosses = None;
                }
                if !side.has_system(&content, nettai_match::PATCH_CARDS_SYSTEM) {
                    side.cards.clear();
                }
                self.edited();
            }
            Msg::Navi(s, c) => {
                // A link navi: its stats at the side's level, as the game switches.
                if crate::levels::switch_navi(&content, &mut self.m.sides[s], c.value) {
                    self.typed.retain(|&(x, k), _| x != s || stats::field(k).is_none());
                    self.edited();
                    return Task::none();
                }
                let side = &mut self.m.sides[s];
                let keep = stats::diff(&content, &Side::base_stats(&content, side.navi, side.game), &side.stats);
                side.navi = c.value;
                // (Operating MegaMan again clears the navi code received,
                // `sub_809CD60`.)
                side.navi_level = nettai_match::default_navi_level(&content, c.value);
                self.typed.remove(&(s, "level"));
                side.stats = Side::base_stats(&content, c.value, side.game);
                // The save's own fields carry over.
                let carried: std::collections::BTreeMap<String, toml::Value> =
                    keep.into_iter().filter(|(k, _)| ["hp", "regular_memory", "mood", "sun", "beast_out_counter"].contains(&k.as_str())).collect();
                stats::apply(&content, &carried, &mut side.stats);
                if content.navi(c.value).forms.is_none() {
                    side.crosses = None;
                }
                self.edited();
            }
            Msg::Game(s, c) => {
                let side = &mut self.m.sides[s];
                side.game = c.value;
                side.stats.version = nettai_match::version_byte(c.value);
                self.edited();
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
                    crate::levels::level_changed(&content, &mut self.m.sides[s]);
                    // (The stats' fields show the new values.)
                    self.typed.retain(|&(x, k), _| x != s || stats::field(k).is_none());
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
                if let Some(path) = rfd::FileDialog::new().add_filter("BN6 save", &["sav"]).pick_file() {
                    let read = std::fs::read(&path).map_err(|e| e.to_string());
                    match read.and_then(|bytes| self.m.sides[s].import_save(&content, &bytes)) {
                        Ok(notes) => {
                            self.typed.retain(|&(x, _), _| x != s);
                            self.sp_typed.retain(|&(x, _), _| x != s);
                            self.edited();
                            let notes = if notes.is_empty() { String::new() } else { format!(" ({})", notes.join("; ")) };
                            self.status = format!("the game, unlocks, navi code and SP times from {}{notes}", path.display());
                        }
                        Err(e) => self.status = format!("can't import {}: {e}", path.display()),
                    }
                }
            }
            Msg::BugFrags(s, t) => {
                if let Ok(v) = t.trim().parse() {
                    self.m.sides[s].bug_frags = v;
                    self.edited();
                }
                self.typed.insert((s, "bug_frags"), t);
            }
            Msg::Glitch(s, on) => {
                self.m.sides[s].emotion_window_glitch = on;
                self.edited();
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
            Msg::OwnCrosses(s, own) => {
                let side = &mut self.m.sides[s];
                side.crosses = if own { None } else { Some(CrossList::default()) };
                self.edited();
            }
            Msg::Cross(s, f, on) => {
                let side = &mut self.m.sides[s];
                let mut forms: Vec<FormHandle> = side.crosses.map(|l| l.forms().collect()).unwrap_or_default();
                forms.retain(|&x| x != f);
                if on && forms.len() < nettai_match::CROSSES {
                    forms.push(f);
                }
                // In the window's order: the navi's (Gregar's, then Falzar's).
                let order = nettai_match::navi_crosses(&content, side.navi).unwrap_or_default();
                forms.sort_by_key(|f| order.iter().position(|x| x == f));
                side.crosses = Some(CrossList::new(&forms));
                self.edited();
            }
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
            Msg::StatText(s, name, t) => {
                if let (Some(f), Ok(v)) = (stats::field(name), t.trim().parse::<u32>()) {
                    if let stats::Kind::Int(max) = f.kind
                        && v <= max
                    {
                        (f.set)(&mut self.m.sides[s].stats, stats::Value::Int(v));
                        self.edited();
                    }
                }
                self.typed.insert((s, name), t);
            }
            Msg::StatValue(s, name, v) => {
                if let Some(f) = stats::field(name) {
                    (f.set)(&mut self.m.sides[s].stats, v);
                    self.edited();
                }
            }
            Msg::StatsReset(s) => {
                let side = &mut self.m.sides[s];
                side.stats = crate::levels::reset(&content, side);
                self.typed.retain(|(x, _), _| *x != s);
                self.edited();
            }
            Msg::NaviCust(s, edit) => {
                if crate::navicust::update(&content, &mut self.m.sides[s], &mut self.navicust[s], edit) {
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
