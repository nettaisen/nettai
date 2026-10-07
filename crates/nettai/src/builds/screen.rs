//! The Builds screen's state and the build open in the creator. Their
//! navigation is the app's: the window passes each action (a key, a pad's
//! button) and each press of the mouse or a finger on a row, an option or a
//! cell, and the creator answers with what it did (`Did`); the app plays its
//! sound, saves and checks the build if it changed, and shows it again
//! (`crate::builds::view`). Every edit is saved at once, and the build
//! checked again (`nettai_match::check`: what the rules say of it, each
//! problem where it is).

use crate::builds::layout::{self, Layout, Tab};
use crate::builds::pictures::Pictures;
use crate::builds::{auto, edit, grid, import, order::Order, presets, store};
use crate::games::{Names, Ready};
use crate::{ActionButton, BuildAction, LeftKind, NavAction, RightKind, UiSound};
use nettai_battle::content::strings::Strings;
use nettai_battle::content::{ChipClass, Content, PlayerFact};
use nettai_battle::setup::NaviStats;
use nettai_content_api::{ChipHandle, EntryHandle, FieldType, FormHandle, NaviHandle, Registry};
use nettai_match::Side;
use nettai_match::check::Problem;
use nettai_match::facts::{self, Stated};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

/// What the creator keeps of a game: its pictures, its library order, and
/// what the grid and the auto battle view read of it.
pub struct Kit {
    pub ready: Rc<Ready>,
    pub pictures: Pictures,
    pub order: Order,
    pub grid: Option<grid::GridData>,
    pub auto: Option<auto::Layout>,
}

impl Kit {
    pub fn new(game: &str, ready: Rc<Ready>) -> Kit {
        let content = ready.content().clone();
        let modules = layout::modules(&content, game, &[grid::MODULE, auto::MODULE]);
        Kit {
            pictures: Pictures::new(ready.clone()),
            order: Order::load(&ready.loaded.game.dir, game),
            grid: grid::read(&content, modules[0].as_ref()),
            auto: auto::Layout::read(&content, modules[1].as_ref()),
            ready,
        }
    }

    pub fn content(&self) -> &Arc<Content> {
        self.ready.content()
    }
}

/// A build listed: its file, and its side as its game reads it, with what
/// the rules say of it.
pub struct Shown {
    pub listed: store::Listed,
    pub side: Result<Side, String>,
    pub problems: Vec<Problem>,
}

/// The Builds screen's state, and the build open in the creator.
#[derive(Default)]
pub struct BuildsState {
    /// The game shown, by its place among those ready.
    pub game: usize,
    pub shown: Vec<Shown>,
    pub cursor: usize,
    pub status: String,
    pub editor: Option<Editor>,
    pub kits: HashMap<String, Rc<Kit>>,
    /// The screen the creator goes back to (the Builds tab, or the tab whose
    /// strip opened it).
    pub from: crate::Screen,
}

/// The face the emotion window shows of a navi: its form's for an emotion
/// (by its name in the game's rules; of the form's second set where its
/// rules ask), or a navi's own where it doesn't change form.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Form(FormHandle, String, bool),
    Navi(NaviHandle),
}

/// What the round a build starts shows of it: its navi's stats, the chips
/// its rules let a folder hold, and the face its navi starts with (none:
/// no navi on the field by then).
pub struct Round {
    pub stats: NaviStats,
    pub pool: Vec<ChipHandle>,
    pub face: Option<Face>,
}

/// The ticks a round runs before its navi's face is read: the first puts
/// the navi on the field, whose start sets its mood (EXE5's light or dark
/// MegaMan's, 0x08010EC8).
const FACE_TICKS: u32 = 1;

/// What a build is checked as: the side on both sides of a match of its
/// game (what side 0's rules say of it; the round it starts, for its stats,
/// the chips its rules let a folder hold and the face its navi starts
/// with).
pub fn checked(content: &Arc<Content>, game: &str, side: &Side) -> (Vec<Problem>, Result<Round, String>) {
    let Ok(mut m) = nettai_match::Match::empty(content, game) else { return (Vec::new(), Err(String::new())) };
    m.sides = [side.clone(), side.clone()];
    let problems = nettai_match::check::problems(content, &m).into_iter().filter(|p| p.side != Some(1)).collect();
    let round = nettai_match::check::start(content, &m).map(|mut b| {
        let pool = nettai_match::folders::pool(content, game, &mut b, 0);
        let stats = b.stats[0];
        Round { stats, pool, face: starting_face(&mut b) }
    });
    (problems, round)
}

/// The face side 0's navi shows once the round has started.
fn starting_face(b: &mut nettai_battle::Battle) -> Option<Face> {
    use nettai_battle::kinds::player;
    for _ in 0..FACE_TICKS {
        b.tick(&Default::default(), Default::default());
    }
    b.player(0)?;
    let stats = &b.stats[0];
    if !b.content.navi(stats.navi).changes_form() {
        return Some(Face::Navi(stats.navi));
    }
    let emotion = b.game_rules().emotion.name(player::emotion(b, 0)).to_string();
    Some(Face::Form(stats.form, emotion, player::shows_face_variant(b, 0)))
}

/// A row of a rows tab: what it is.
#[derive(Clone, Debug, PartialEq)]
pub enum RowSpec {
    Name,
    Navi,
    Fact(String),
    /// A fact stated as one of its presets.
    Preset(String),
    FromSave,
    Duplicate,
    Delete,
    /// A list of times' entry.
    Time(String, usize),
}

/// An entry of an entries tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntrySpec {
    Folder(usize),
    Listed(usize),
    Place(usize),
    /// A list's heading (the auto battle data's).
    Heading(usize),
    Other(usize),
}

/// One of what a browser offers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PickSpec {
    Chip(ChipHandle),
    Entry(u16),
    Piece(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FilterSpec {
    Search,
    All,
    Class(ChipClass),
    /// What the auto battle data's list holds.
    List,
}

/// What is being typed.
#[derive(Clone, Debug, PartialEq)]
pub enum Editing {
    Name,
    Number(String),
    Time(String, usize),
    Search,
}

/// What an action did beyond the build, for the app to do after.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Then {
    #[default]
    Nothing,
    Leave,
    /// A save, for the whole build or the auto battle data alone.
    FromSave,
    AutoFromSave,
    Duplicate,
    Delete,
}

#[derive(Default)]
pub struct Did {
    pub sound: Option<UiSound>,
    /// The build changed: saved, checked, shown again.
    pub edited: bool,
    /// The lists to show again (another tab, a filter).
    pub lists: bool,
    pub then: Then,
}

impl Did {
    pub fn sound(s: UiSound) -> Did {
        Did { sound: Some(s), ..Did::default() }
    }

    pub fn cursor() -> Did {
        Did::sound(UiSound::Cursor)
    }

    pub fn refused() -> Did {
        Did::sound(UiSound::Refused)
    }

    /// An edit, if it changed the build (else refused).
    pub fn edit(changed: bool) -> Did {
        if changed { Did { sound: Some(UiSound::Pick), edited: true, ..Did::default() } } else { Did::refused() }
    }

    pub fn lists(sound: UiSound) -> Did {
        Did { sound: Some(sound), lists: true, ..Did::default() }
    }

    fn then(then: Then) -> Did {
        Did { sound: Some(UiSound::Pick), then, ..Did::default() }
    }
}

/// The build open in the creator.
pub struct Editor {
    pub kit: Rc<Kit>,
    pub game: String,
    pub path: PathBuf,
    pub name: String,
    pub side: Side,
    pub layout: Layout,
    pub tab: usize,
    /// The pane with the keys (0 the left, 1 the right), their rows (-1: the
    /// left's action bar, the right's filters), the option chosen; the left
    /// row's strip, open.
    pub pane: u8,
    pub cursor: i32,
    pub pick: i32,
    pub option: usize,
    pub strip: bool,
    pub search: String,
    pub filter: usize,
    pub held: Option<grid::Held>,
    pub cell: (i32, i32),
    pub editing: Option<Editing>,
    pub delete_armed: bool,
    pub problems: Vec<Problem>,
    pub round: Result<Round, String>,
    pub status: String,
    /// The tiles in a row, as the window fits them.
    pub columns: usize,
    /// The names in the window's language, for the searches.
    pub strings: Option<Arc<Strings>>,
    // What the tab shows, as worked out the last time it changed.
    pub rows: Vec<RowSpec>,
    pub entries: Vec<EntrySpec>,
    pub tiles: Vec<u16>,
    pub picks: Vec<PickSpec>,
    pub filters: Vec<FilterSpec>,
    pub actions: Vec<ActionButton>,
}

impl Editor {
    pub fn new(kit: Rc<Kit>, game: &str, path: PathBuf, name: String, side: Side) -> Editor {
        let mut e = Editor {
            kit,
            game: game.to_string(),
            path,
            name,
            side,
            layout: Layout::default(),
            tab: 0,
            pane: 0,
            cursor: 0,
            pick: 0,
            option: 0,
            strip: false,
            search: String::new(),
            filter: 0,
            held: None,
            cell: (3, 3),
            editing: None,
            delete_armed: false,
            problems: Vec::new(),
            round: Err(String::new()),
            status: String::new(),
            columns: 1,
            strings: None,
            rows: Vec::new(),
            entries: Vec::new(),
            tiles: Vec::new(),
            picks: Vec::new(),
            filters: Vec::new(),
            actions: Vec::new(),
        };
        // (A file that states its save facts otherwise is written again.)
        if e.check() {
            e.save();
        }
        e.work_out();
        e
    }

    pub fn content(&self) -> Arc<Content> {
        self.kit.content().clone()
    }

    pub fn tab(&self) -> &Tab {
        self.layout.tabs.get(self.tab).unwrap_or(&Tab::Navi)
    }

    /// Check the build again, and lay it out again (a navi of another kind
    /// has other tabs). Its save facts are first set to what every build
    /// in the app has (`layout::as_built`): whether that changed it.
    pub fn check(&mut self) -> bool {
        let content = self.content();
        let reset = layout::as_built(&content, &self.game, &mut self.side).changed;
        let (problems, round) = checked(&content, &self.game, &self.side);
        self.problems = problems;
        self.round = round;
        let key = self.tab().key().to_string();
        self.layout = layout::layout(&content, &self.game, &self.side, self.kit.grid.as_ref(), self.kit.auto.as_ref());
        self.tab = self.layout.tabs.iter().position(|t| t.key() == key).unwrap_or(0);
        reset
    }

    pub fn save(&mut self) {
        let content = self.content();
        if let Err(e) = store::write(&self.path, &content, &self.game, &self.name, &self.side) {
            self.status = e;
        }
    }

    /// The problems of setup field `field` at `entry` (none: of the whole
    /// field).
    pub fn said(&self, field: &str, entry: Option<usize>) -> Vec<String> {
        self.problems.iter().filter(|p| p.field.as_deref() == Some(field) && p.entry == entry).map(|p| p.text.clone()).collect()
    }

    /// The problems a tab shows: of its fields; the NAVI tab's, those tied
    /// to no field too.
    pub fn tab_problems(&self, tab: &Tab) -> Vec<&Problem> {
        let content = self.content();
        let fields = tab.fields(&content, &self.layout.rows);
        self.problems.iter().filter(|p| p.field.as_ref().map_or(*tab == Tab::Navi, |f| fields.contains(f))).collect()
    }

    pub fn stats(&self) -> Option<&NaviStats> {
        self.round.as_ref().ok().map(|r| &r.stats)
    }

    fn pool(&self) -> &[ChipHandle] {
        self.round.as_ref().map_or(&[], |r| &r.pool)
    }

    fn names(&self) -> Names<'_> {
        Names { content: self.kit.content(), strings: self.strings.as_deref() }
    }

    // ---- What a tab shows -------------------------------------------------------------

    /// Work out what the tab shows: its rows, entries, tiles, what its
    /// browser offers, its filters and actions.
    pub fn work_out(&mut self) {
        let content = self.content();
        let c = &*content;
        let tab = self.tab().clone();
        self.rows.clear();
        self.entries.clear();
        self.tiles.clear();
        self.actions.clear();
        let button = |action: BuildAction| ActionButton { action };
        match &tab {
            Tab::Navi => {
                self.rows.push(RowSpec::Name);
                self.rows.push(RowSpec::Navi);
                let game = &self.game;
                self.rows.extend(self.layout.rows.iter().map(|f| if presets::of(game, f).is_some() { RowSpec::Preset(f.clone()) } else { RowSpec::Fact(f.clone()) }));
                if import::reads_saves(&self.game) {
                    self.rows.push(RowSpec::FromSave);
                }
                self.rows.extend([RowSpec::Duplicate, RowSpec::Delete]);
            }
            Tab::Folder => self.entries = (0..nettai_battle::custom::folder::FOLDER_SIZE).map(EntrySpec::Folder).collect(),
            Tab::Checklist(f) => {
                if let Some(field) = facts::field(c, f) {
                    self.tiles = facts::offered(c, &self.game, &self.side, &field).unwrap_or_default();
                    let held = self.side.facts.get(c, f).map(|v| v.defs()).unwrap_or_default();
                    if !held.is_empty() {
                        self.actions.push(button(BuildAction::ClearList));
                    }
                    if !self.side.facts.is_default(c, f) {
                        self.actions.push(button(BuildAction::Default));
                    }
                    if facts::role_of(c, f) == Some(PlayerFact::FormList) && self.side.version(c).is_some() {
                        self.actions.push(button(BuildAction::Own));
                    }
                }
            }
            Tab::Grid => {}
            Tab::Entries { field, .. } => {
                let n = self.side.facts.get(c, field).map_or(0, |v| v.defs().len());
                self.entries = (0..n).map(EntrySpec::Listed).collect();
                if n > 0 {
                    self.actions.push(button(BuildAction::ClearList));
                }
            }
            Tab::Times(field) | Tab::Other(field) => {
                let n = match self.side.facts.get(c, field) {
                    Some(Stated::List(items)) => items.len(),
                    _ => 0,
                };
                if matches!(tab, Tab::Times(_)) {
                    self.rows = (0..n).map(|i| RowSpec::Time(field.clone(), i)).collect();
                } else {
                    self.entries = (0..n).map(EntrySpec::Other).collect();
                }
            }
            Tab::Places => {
                if let Some(l) = &self.kit.auto {
                    for (k, list) in l.lists.iter().enumerate() {
                        self.entries.push(EntrySpec::Heading(k));
                        self.entries.extend(list.places().map(EntrySpec::Place));
                    }
                }
                if import::reads_saves(&self.game) {
                    self.actions.push(button(BuildAction::FromSave));
                }
                if auto::AutoBattle::of_side(c, &self.side).entries() > 0 {
                    self.actions.push(button(BuildAction::ClearList));
                }
            }
        }
        let last = self.left_len() as i32 - 1;
        let first = if self.actions.is_empty() || self.left_kind() == LeftKind::Board { 0 } else { -1 };
        self.cursor = self.cursor.min(last).max(first);
        if self.left_kind() == LeftKind::Board {
            self.cursor = self.cursor.clamp(-1, 0);
        } else if self.cursor >= 0 && !self.focusable(self.cursor) {
            self.cursor = self.next_focusable(self.cursor, 1);
        }
        self.work_out_picks();
    }

    /// What the browser offers: its filters, and what they let through.
    pub fn work_out_picks(&mut self) {
        let content = self.content();
        let c = &*content;
        let names = self.names();
        let needle = self.search.to_lowercase();
        let found = |name: &str, key: &str| needle.is_empty() || name.to_lowercase().contains(&needle) || key.contains(&needle);
        let mut filters = vec![FilterSpec::Search];
        let mut picks = Vec::new();
        match self.tab().clone() {
            Tab::Folder => {
                let pool = self.pool().to_vec();
                filters.push(FilterSpec::All);
                for class in [ChipClass::Standard, ChipClass::Mega, ChipClass::Giga] {
                    if pool.iter().any(|&h| c.chip(h).class == class) {
                        filters.push(FilterSpec::Class(class));
                    }
                }
                let class = match filters.get(self.filter) {
                    Some(FilterSpec::Class(k)) => Some(*k),
                    _ => None,
                };
                let mut list: Vec<(String, ChipHandle)> = pool
                    .into_iter()
                    .filter(|&h| class.is_none_or(|k| c.chip(h).class == k))
                    .map(|h| (names.chip(h), h))
                    .filter(|(n, h)| found(n, nettai_match::ids::local(&c.defs.chip(*h).key)))
                    .collect();
                self.kit.order.chips(c, &mut list);
                picks = list.into_iter().map(|(_, h)| PickSpec::Chip(h)).collect();
            }
            Tab::Places => {
                // (What a place can hold: the game's chips a folder holds,
                // those with codes, and its program advances; by class, or
                // what the game writes in the place's list.)
                let chips: Vec<ChipHandle> = (0..c.defs.chips.len() as u16)
                    .map(ChipHandle)
                    .filter(|&h| nettai_match::ids::in_game(c, &self.game, &c.defs.chip(h).key))
                    .filter(|&h| {
                        let d = c.chip(h);
                        match d.class {
                            ChipClass::Standard | ChipClass::Mega | ChipClass::Giga => !d.codes.is_empty(),
                            ChipClass::ProgramAdvance => true,
                            ChipClass::Special => false,
                        }
                    })
                    .collect();
                filters.extend([FilterSpec::List, FilterSpec::All]);
                for class in [ChipClass::Standard, ChipClass::Mega, ChipClass::Giga, ChipClass::ProgramAdvance] {
                    if chips.iter().any(|&h| c.chip(h).class == class) {
                        filters.push(FilterSpec::Class(class));
                    }
                }
                let wanted = match (filters.get(self.filter), self.selected_place()) {
                    (Some(FilterSpec::List), Some(i)) => match self.kit.auto.as_ref().and_then(|l| l.list_of(i)).map(|l| l.holds) {
                        Some(auto::Holds::Chips(class)) => Some(class),
                        _ => None,
                    },
                    (Some(FilterSpec::Class(k)), _) => Some(*k),
                    _ => None,
                };
                let mut list: Vec<(String, ChipHandle)> = chips
                    .into_iter()
                    .filter(|&h| wanted.is_none_or(|k| c.chip(h).class == k))
                    .map(|h| (names.chip(h), h))
                    .filter(|(n, h)| found(n, nettai_match::ids::local(&c.defs.chip(*h).key)))
                    .collect();
                self.kit.order.chips(c, &mut list);
                picks = list.into_iter().map(|(_, h)| PickSpec::Chip(h)).collect();
            }
            Tab::Entries { field, .. } => {
                if let Some(FieldType::Ref(registry, of)) = facts::field(c, &field).map(|f| layout::element(f.ty).clone()) {
                    let mut list: Vec<(String, EntryHandle)> = edit::of_game(c, &self.game, registry, of.as_deref())
                        .into_iter()
                        .map(|h| (names.def(registry, h), EntryHandle(h)))
                        .filter(|(n, h)| registry != Registry::Entry || found(n, c.defs.entry(*h).id()))
                        .collect();
                    if let (Some(of), Registry::Entry) = (&of, registry) {
                        self.kit.order.entries(c, of, &mut list);
                    }
                    picks = list.into_iter().map(|(_, h)| PickSpec::Entry(h.0)).collect();
                }
            }
            Tab::Grid => {
                if let Some(g) = &self.kit.grid {
                    let mut list: Vec<(String, EntryHandle)> =
                        c.defs.entries_of(&g.collection).into_iter().map(|h| (names.entry(h), h)).filter(|(n, h)| found(n, c.defs.entry(*h).id())).collect();
                    self.kit.order.entries(c, &g.collection, &mut list);
                    picks = list.into_iter().map(|(_, h)| PickSpec::Piece(h.0)).collect();
                }
            }
            _ => filters.clear(),
        }
        self.filters = filters;
        self.picks = picks;
        self.filter = self.filter.min(self.filters.len().saturating_sub(1));
        self.pick = self.pick.min(self.picks.len() as i32 - 1);
    }

    pub fn left_kind(&self) -> LeftKind {
        match self.tab() {
            Tab::Navi | Tab::Times(_) => LeftKind::Rows,
            Tab::Checklist(_) => LeftKind::Tiles,
            Tab::Grid => LeftKind::Board,
            _ => LeftKind::Entries,
        }
    }

    pub fn right_kind(&self) -> RightKind {
        match self.tab() {
            Tab::Navi => RightKind::Summary,
            Tab::Folder | Tab::Grid | Tab::Entries { .. } | Tab::Places => RightKind::Browser,
            _ => RightKind::None,
        }
    }

    fn left_len(&self) -> usize {
        match self.left_kind() {
            LeftKind::Rows => self.rows.len(),
            LeftKind::Tiles => self.tiles.len(),
            LeftKind::Board => 1,
            LeftKind::Entries => self.entries.len(),
        }
    }

    fn focusable(&self, i: i32) -> bool {
        match self.left_kind() {
            LeftKind::Rows => i >= 0 && (i as usize) < self.rows.len(),
            LeftKind::Entries => self.entries.get(i as usize).is_some_and(|e| !matches!(e, EntrySpec::Heading(_))),
            _ => i >= 0 && (i as usize) < self.left_len(),
        }
    }

    /// The next focusable row from `i` going `by`, else `i`.
    pub fn next_focusable(&self, i: i32, by: i32) -> i32 {
        let mut j = i + by;
        while j >= 0 && (j as usize) < self.left_len() {
            if self.focusable(j) {
                return j;
            }
            j += by;
        }
        i
    }

    /// The auto battle data's place the left cursor is on.
    pub fn selected_place(&self) -> Option<usize> {
        match (self.tab(), self.entries.get(self.cursor.max(0) as usize)) {
            (Tab::Places, Some(EntrySpec::Place(i))) => Some(*i),
            _ => None,
        }
    }

    /// The strip of the left cursor's row, while it is open.
    pub fn strip(&self) -> Vec<ActionButton> {
        if !self.strip || self.pane != 0 {
            return Vec::new();
        }
        let content = self.content();
        let b = |action: BuildAction| ActionButton { action };
        match (self.tab(), self.entries.get(self.cursor.max(0) as usize)) {
            (Tab::Folder, Some(EntrySpec::Folder(_))) => {
                let mut out = vec![b(BuildAction::Chip)];
                if content.defs.fact_field(PlayerFact::RegularChip).is_some() {
                    out.push(b(BuildAction::Regular));
                }
                if content.defs.fact_field(PlayerFact::TagChips).is_some() {
                    out.push(b(BuildAction::Tag));
                }
                out.push(b(BuildAction::Clear));
                out
            }
            (Tab::Entries { .. }, Some(EntrySpec::Listed(_))) => vec![b(BuildAction::Up), b(BuildAction::Down), b(BuildAction::Remove)],
            (Tab::Places, Some(EntrySpec::Place(_))) => vec![b(BuildAction::Chip), b(BuildAction::Empty), b(BuildAction::Zero)],
            _ => Vec::new(),
        }
    }

    // ---- The keys ----------------------------------------------------------------------

    pub fn nav(&mut self, a: NavAction) -> Did {
        if self.editing.is_some() {
            return match a {
                NavAction::Back => {
                    self.editing = None;
                    Did::sound(UiSound::Back)
                }
                _ => Did::default(),
            };
        }
        if a != NavAction::Confirm {
            self.delete_armed = false;
        }
        // (The piece in hand has the keys.)
        if self.held.is_some() && self.tab() == &Tab::Grid {
            return self.nav_held(a);
        }
        match a {
            NavAction::Previous | NavAction::Next => self.switch_tab(if a == NavAction::Previous { -1 } else { 1 }),
            NavAction::Back => {
                if self.strip {
                    self.strip = false;
                    self.option = 0;
                    Did::sound(UiSound::Back)
                } else if self.pane == 1 {
                    self.leave_browser()
                } else {
                    Did { sound: Some(UiSound::Back), then: Then::Leave, ..Did::default() }
                }
            }
            NavAction::Remove => self.remove(),
            _ if self.pane == 1 => self.nav_right(a),
            _ => match self.left_kind() {
                LeftKind::Rows => self.nav_rows(a),
                LeftKind::Tiles => self.nav_tiles(a),
                LeftKind::Board => self.nav_board(a),
                LeftKind::Entries => self.nav_entries(a),
            },
        }
    }

    /// The focused thing on the left taken off: a folder's chip, a list's
    /// entry, a place's, the piece under the board's cursor.
    fn remove(&mut self) -> Did {
        if self.pane != 0 || self.cursor < 0 {
            return Did::default();
        }
        let content = self.content();
        let c = &*content;
        let key = self.tab().key().to_string();
        self.strip = false;
        match (self.tab(), self.entries.get(self.cursor as usize).copied()) {
            (Tab::Folder, Some(EntrySpec::Folder(i))) => Did::edit(edit::folder(c, &mut self.side, edit::FolderEdit::Clear(i))),
            (Tab::Entries { .. }, Some(EntrySpec::Listed(i))) => Did::edit(edit::list(c, &mut self.side, &key, edit::ListEdit::Remove(i))),
            (Tab::Places, Some(EntrySpec::Place(i))) => Did::edit(auto::update(c, &mut self.side, auto::Edit::Empty(i))),
            (Tab::Grid, _) => {
                let Some(g) = self.kit.grid.clone() else { return Did::default() };
                let parts = grid::placed(c, &self.side);
                match g.occupied(&parts).get(&self.cell) {
                    Some(&i) => Did::edit(grid::edit(&g, c, &mut self.side, &mut self.held, grid::Edit::Remove(i))),
                    None => Did::refused(),
                }
            }
            _ => Did::default(),
        }
    }

    fn switch_tab(&mut self, by: i32) -> Did {
        let n = self.layout.tabs.len() as i32;
        if n <= 1 {
            return Did::refused();
        }
        self.go_tab(((self.tab as i32 + by).rem_euclid(n)) as usize)
    }

    pub fn go_tab(&mut self, tab: usize) -> Did {
        self.tab = tab.min(self.layout.tabs.len().saturating_sub(1));
        self.pane = 0;
        self.cursor = 0;
        self.pick = 0;
        self.option = 0;
        self.strip = false;
        self.filter = 0;
        self.search.clear();
        self.held = None;
        self.work_out();
        Did::lists(UiSound::Cursor)
    }

    /// Up and down through the left rows, the action bar above them.
    fn move_left(&mut self, by: i32) -> Did {
        if self.cursor == -1 {
            if by > 0 && self.left_len() > 0 {
                self.cursor = if self.focusable(0) { 0 } else { self.next_focusable(0, 1) };
                self.option = 0;
                return self.moved();
            }
            return Did::default();
        }
        let next = self.next_focusable(self.cursor, by);
        if next == self.cursor {
            if by < 0 && !self.actions.is_empty() {
                self.cursor = -1;
                self.option = 0;
                return Did::cursor();
            }
            return Did::default();
        }
        self.cursor = next;
        self.option = 0;
        self.moved()
    }

    /// The left cursor moved: the auto battle data's browser follows what
    /// goes in the place.
    fn moved(&mut self) -> Did {
        if self.tab() == &Tab::Places {
            self.work_out_picks();
            return Did::lists(UiSound::Cursor);
        }
        Did::cursor()
    }

    /// The action bar's keys.
    fn nav_actions(&mut self, a: NavAction) -> Did {
        match a {
            NavAction::Left | NavAction::Right => {
                let n = self.actions.len().max(1);
                self.option = (self.option + if a == NavAction::Left { n - 1 } else { 1 }) % n;
                Did::cursor()
            }
            NavAction::Confirm => self.do_action(self.option),
            NavAction::Down => self.move_left(1),
            _ => Did::default(),
        }
    }

    fn nav_rows(&mut self, a: NavAction) -> Did {
        if self.cursor < 0 {
            return self.nav_actions(a);
        }
        let Some(row) = self.rows.get(self.cursor as usize).cloned() else { return Did::default() };
        match a {
            NavAction::Up => self.move_left(-1),
            NavAction::Down => self.move_left(1),
            NavAction::Left | NavAction::Right => self.step_row(&row, if a == NavAction::Left { -1 } else { 1 }),
            NavAction::Confirm => self.confirm_row(&row),
            _ => Did::default(),
        }
    }

    /// A row's value a step on.
    pub fn step_row(&mut self, row: &RowSpec, by: i32) -> Did {
        let content = self.content();
        let c = &*content;
        let quiet = |did: Did| Did { sound: did.sound.map(|s| if s == UiSound::Pick { UiSound::Cursor } else { s }), ..did };
        match row {
            RowSpec::Navi => {
                let navis = nettai_match::navis(c, &self.game);
                if navis.len() < 2 {
                    return Did::refused();
                }
                let now = navis.iter().position(|&n| Some(n) == self.side.stated_navi(c)).unwrap_or(0) as i32;
                let next = navis[(now + by).rem_euclid(navis.len() as i32) as usize];
                quiet(Did::edit(edit::switch_navi(c, &mut self.side, next)))
            }
            RowSpec::Fact(f) => quiet(Did::edit(edit::fact(c, &self.game, &mut self.side, f, &edit::FactEdit::Step(by as i64)))),
            RowSpec::Preset(f) => quiet(Did::edit(presets::step(c, &self.game, &mut self.side, f, by as i64))),
            RowSpec::Time(f, i) => {
                let frames = self.time_of(f, *i).unwrap_or(0) as i32;
                let next = (frames + by * 60).clamp(0, u16::MAX as i32) as u16;
                quiet(Did::edit(edit::time(c, &mut self.side, f, *i, next)))
            }
            _ => Did::default(),
        }
    }

    fn confirm_row(&mut self, row: &RowSpec) -> Did {
        let content = self.content();
        let c = &*content;
        match row {
            RowSpec::Name => self.start_editing(Editing::Name),
            RowSpec::Navi | RowSpec::Preset(_) => self.step_row(row, 1),
            RowSpec::Fact(f) => match facts::field(c, f).map(|f| f.ty.clone()) {
                Some(ty) if facts::range(&ty).is_some() => self.start_editing(Editing::Number(f.clone())),
                _ => self.step_row(row, 1),
            },
            RowSpec::Time(f, i) => self.start_editing(Editing::Time(f.clone(), *i)),
            RowSpec::FromSave => Did::then(Then::FromSave),
            RowSpec::Duplicate => Did::then(Then::Duplicate),
            RowSpec::Delete if self.delete_armed => Did::then(Then::Delete),
            RowSpec::Delete => {
                self.delete_armed = true;
                Did::sound(UiSound::Refused)
            }
        }
    }

    fn start_editing(&mut self, what: Editing) -> Did {
        self.editing = Some(what);
        Did::sound(UiSound::Pick)
    }

    fn nav_tiles(&mut self, a: NavAction) -> Did {
        if self.cursor < 0 {
            return self.nav_actions(a);
        }
        let columns = self.columns.max(1) as i32;
        let n = self.tiles.len() as i32;
        let to = match a {
            NavAction::Left => self.cursor - 1,
            NavAction::Right => self.cursor + 1,
            NavAction::Up if self.cursor < columns => {
                if self.actions.is_empty() {
                    return Did::default();
                }
                self.cursor = -1;
                self.option = 0;
                return Did::cursor();
            }
            NavAction::Up => self.cursor - columns,
            NavAction::Down => self.cursor + columns,
            NavAction::Confirm => return self.toggle_tile(self.cursor as usize),
            _ => return Did::default(),
        };
        if to < 0 || to >= n {
            return Did::default();
        }
        self.cursor = to;
        Did::cursor()
    }

    pub fn toggle_tile(&mut self, i: usize) -> Did {
        let Tab::Checklist(f) = self.tab().clone() else { return Did::default() };
        let Some(&h) = self.tiles.get(i) else { return Did::default() };
        let content = self.content();
        let on = self.side.facts.get(&content, &f).is_some_and(|v| v.defs().contains(&h));
        Did::edit(edit::fact(&content, &self.game, &mut self.side, &f, &edit::FactEdit::Listed(h, !on)))
    }

    fn nav_entries(&mut self, a: NavAction) -> Did {
        if self.cursor < 0 {
            return self.nav_actions(a);
        }
        if self.strip {
            let n = self.strip().len().max(1);
            return match a {
                NavAction::Left | NavAction::Right => {
                    self.option = (self.option + if a == NavAction::Left { n - 1 } else { 1 }) % n;
                    Did::cursor()
                }
                NavAction::Confirm => self.do_strip(self.option),
                NavAction::Up | NavAction::Down => {
                    self.strip = false;
                    self.option = 0;
                    self.move_left(if a == NavAction::Up { -1 } else { 1 })
                }
                _ => Did::default(),
            };
        }
        match a {
            NavAction::Up => self.move_left(-1),
            NavAction::Down => self.move_left(1),
            NavAction::Right if self.right_kind() == RightKind::Browser => self.to_browser(UiSound::Cursor),
            NavAction::Confirm => self.confirm_entry(),
            _ => Did::default(),
        }
    }

    fn to_browser(&mut self, sound: UiSound) -> Did {
        self.pane = 1;
        self.strip = false;
        self.option = 0;
        if self.pick < 0 && !self.picks.is_empty() {
            self.pick = 0;
        }
        Did::lists(sound)
    }

    fn leave_browser(&mut self) -> Did {
        self.pane = 0;
        self.option = 0;
        Did::lists(UiSound::Back)
    }

    fn confirm_entry(&mut self) -> Did {
        let content = self.content();
        match self.entries.get(self.cursor as usize).copied() {
            // (An empty entry is filled from the browser at once.)
            Some(EntrySpec::Folder(i)) if self.side.folder(&content).chips[i].is_none() => self.to_browser(UiSound::Pick),
            Some(EntrySpec::Place(_) | EntrySpec::Folder(_) | EntrySpec::Listed(_)) => {
                self.strip = true;
                self.option = 0;
                Did::sound(UiSound::Pick)
            }
            _ => Did::default(),
        }
    }

    pub fn do_strip(&mut self, k: usize) -> Did {
        let Some(b) = self.strip().get(k).cloned() else { return Did::default() };
        let content = self.content();
        let c = &*content;
        let key = self.tab().key().to_string();
        let did = match (self.entries.get(self.cursor.max(0) as usize).copied(), b.action) {
            (_, BuildAction::Chip) => return self.to_browser(UiSound::Pick),
            (Some(EntrySpec::Folder(i)), BuildAction::Regular) => Did::edit(edit::folder(c, &mut self.side, edit::FolderEdit::Regular(i))),
            (Some(EntrySpec::Folder(i)), BuildAction::Tag) => Did::edit(edit::folder(c, &mut self.side, edit::FolderEdit::Tag(i))),
            (Some(EntrySpec::Folder(i)), BuildAction::Clear) => Did::edit(edit::folder(c, &mut self.side, edit::FolderEdit::Clear(i))),
            (Some(EntrySpec::Listed(i)), BuildAction::Up | BuildAction::Down) => {
                let up = b.action == BuildAction::Up;
                let did = Did::edit(edit::list(c, &mut self.side, &key, edit::ListEdit::Move(i, up)));
                // (The strip stays with the entry moved.)
                if did.edited {
                    self.cursor += if up { -1 } else { 1 };
                }
                return did;
            }
            (Some(EntrySpec::Listed(i)), BuildAction::Remove) => Did::edit(edit::list(c, &mut self.side, &key, edit::ListEdit::Remove(i))),
            (Some(EntrySpec::Place(i)), BuildAction::Empty) => Did::edit(auto::update(c, &mut self.side, auto::Edit::Empty(i))),
            (Some(EntrySpec::Place(i)), BuildAction::Zero) => Did::edit(auto::update(c, &mut self.side, auto::Edit::Zero(i))),
            _ => Did::default(),
        };
        self.strip = false;
        self.option = 0;
        did
    }

    /// The action bar's button `k`.
    pub fn do_action(&mut self, k: usize) -> Did {
        let Some(b) = self.actions.get(k).cloned() else { return Did::default() };
        let content = self.content();
        let c = &*content;
        let key = self.tab().key().to_string();
        match b.action {
            BuildAction::ClearList => match self.tab() {
                Tab::Entries { .. } => {
                    let mut changed = false;
                    while edit::list(c, &mut self.side, &key, edit::ListEdit::Remove(0)) {
                        changed = true;
                    }
                    Did::edit(changed)
                }
                Tab::Places => Did::edit(auto::update(c, &mut self.side, auto::Edit::Clear)),
                _ => Did::edit(edit::fact(c, &self.game, &mut self.side, &key, &edit::FactEdit::Empty)),
            },
            BuildAction::Default => Did::edit(edit::fact(c, &self.game, &mut self.side, &key, &edit::FactEdit::Default)),
            BuildAction::Own => Did::edit(edit::fact(c, &self.game, &mut self.side, &key, &edit::FactEdit::Own)),
            BuildAction::FromSave => Did::then(Then::AutoFromSave),
            _ => Did::default(),
        }
    }

    /// The browser's keys.
    fn nav_right(&mut self, a: NavAction) -> Did {
        if self.pick < 0 {
            return match a {
                NavAction::Left | NavAction::Right => {
                    let n = self.filters.len().max(1);
                    self.option = (self.option + if a == NavAction::Left { n - 1 } else { 1 }) % n;
                    Did::cursor()
                }
                NavAction::Confirm => self.do_filter(self.option),
                NavAction::Down if !self.picks.is_empty() => {
                    self.pick = 0;
                    self.option = 0;
                    Did::cursor()
                }
                _ => Did::default(),
            };
        }
        match a {
            NavAction::Up => {
                self.pick -= 1;
                self.option = if self.pick < 0 { self.filter } else { 0 };
                Did::cursor()
            }
            NavAction::Down if self.pick + 1 < self.picks.len() as i32 => {
                self.pick += 1;
                self.option = 0;
                Did::cursor()
            }
            NavAction::Left | NavAction::Right => {
                let n = self.pick_options(self.pick as usize);
                if n <= 1 || (a == NavAction::Left && self.option == 0) {
                    return if a == NavAction::Left { self.leave_browser() } else { Did::default() };
                }
                self.option = (self.option + if a == NavAction::Left { n - 1 } else { 1 }) % n;
                Did::cursor()
            }
            NavAction::Confirm => self.put(self.pick as usize, self.option),
            _ => Did::default(),
        }
    }

    /// How many options pick `i` has (its codes, its colors).
    pub fn pick_options(&self, i: usize) -> usize {
        let content = self.content();
        match self.picks.get(i) {
            Some(PickSpec::Chip(h)) if self.tab() == &Tab::Folder => content.chip(*h).codes.len(),
            Some(PickSpec::Piece(h)) => self.kit.grid.as_ref().and_then(|g| g.colors.get(h)).map_or(1, Vec::len),
            _ => 1,
        }
    }

    pub fn do_filter(&mut self, i: usize) -> Did {
        match self.filters.get(i) {
            Some(FilterSpec::Search) => self.start_editing(Editing::Search),
            Some(_) => {
                self.filter = i;
                self.pick = -1;
                self.option = i;
                self.work_out_picks();
                Did::lists(UiSound::Pick)
            }
            None => Did::default(),
        }
    }

    /// Put pick `i` (in its option `option`) where the left cursor is.
    pub fn put(&mut self, i: usize, option: usize) -> Did {
        let Some(&p) = self.picks.get(i) else { return Did::default() };
        let content = self.content();
        let c = &*content;
        match (self.tab().clone(), p) {
            (Tab::Folder, PickSpec::Chip(h)) => {
                let Some(EntrySpec::Folder(entry)) = self.entries.get(self.cursor.max(0) as usize).copied() else { return Did::refused() };
                let Some(&code) = c.chip(h).codes.get(option) else { return Did::refused() };
                let did = Did::edit(edit::folder(c, &mut self.side, edit::FolderEdit::Put(entry, h, code)));
                // (On to the next entry, as one fills a folder.)
                if did.edited {
                    self.cursor = (self.cursor + 1) % nettai_battle::custom::folder::FOLDER_SIZE as i32;
                }
                did
            }
            (Tab::Entries { field, .. }, PickSpec::Entry(h)) => Did::edit(edit::list(c, &mut self.side, &field, edit::ListEdit::Add(h))),
            (Tab::Grid, PickSpec::Piece(h)) => {
                let Some(g) = &self.kit.grid else { return Did::default() };
                grid::edit(g, c, &mut self.side, &mut self.held, grid::Edit::Hold(h, option.min(g.colors.get(&h).map_or(1, Vec::len).saturating_sub(1)) as u8, false));
                self.pane = 0;
                self.cursor = 0;
                Did::lists(UiSound::Pick)
            }
            (Tab::Places, PickSpec::Chip(h)) => {
                let Some(place) = self.selected_place() else { return Did::refused() };
                let did = Did::edit(auto::update(c, &mut self.side, auto::Edit::Put(place, h)));
                // (On to the next place, as one fills a folder.)
                if did.edited {
                    self.cursor = self.next_focusable(self.cursor, 1);
                }
                did
            }
            _ => Did::default(),
        }
    }

    // ---- The board ---------------------------------------------------------------------

    pub fn board(&self) -> Option<&Vec<Vec<u8>>> {
        let g = self.kit.grid.as_ref()?;
        grid::board_index(self.kit.content(), &self.side).and_then(|b| g.boards.get(b))
    }

    pub fn board_n(&self) -> i32 {
        self.board().map_or(7, |b| b.len() as i32)
    }

    fn nav_board(&mut self, a: NavAction) -> Did {
        let n = self.board_n();
        if self.cursor < 0 {
            return match a {
                NavAction::Left | NavAction::Right => self.step_board(if a == NavAction::Left { -1 } else { 1 }),
                NavAction::Down => {
                    self.cursor = 0;
                    Did::cursor()
                }
                _ => Did::default(),
            };
        }
        let (x, y) = self.cell;
        match a {
            NavAction::Up if y == 0 => {
                self.cursor = -1;
                Did::cursor()
            }
            NavAction::Right if x + 1 >= n => self.to_browser(UiSound::Cursor),
            NavAction::Up | NavAction::Down | NavAction::Left | NavAction::Right => {
                let (dx, dy) = step(a);
                let to = ((x + dx).clamp(0, n - 1), (y + dy).clamp(0, n - 1));
                if to == self.cell {
                    return Did::default();
                }
                self.cell = to;
                Did::cursor()
            }
            NavAction::Confirm => self.press_cell(x, y),
            _ => Did::default(),
        }
    }

    pub fn step_board(&mut self, by: i32) -> Did {
        let Some(g) = self.kit.grid.clone() else { return Did::default() };
        let content = self.content();
        let now = grid::board_index(&content, &self.side).unwrap_or(0) as i32;
        let to = (now + by).clamp(0, g.boards.len() as i32 - 1) as usize;
        Did::edit(grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::Board(to)))
    }

    /// The board pressed at (x, y): the piece in hand put down, or the one
    /// there picked up; on an empty cell, the pieces to choose from.
    pub fn press_cell(&mut self, x: i32, y: i32) -> Did {
        let Some(g) = self.kit.grid.clone() else { return Did::default() };
        let content = self.content();
        if let Some((gx, gy)) = self.held.as_ref().map(|h| h.grab) {
            return Did::edit(grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::Place(x - gx, y - gy)));
        }
        let parts = grid::placed(&content, &self.side);
        match g.occupied(&parts).get(&(x, y)) {
            Some(&i) => {
                let did = Did::edit(grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::PickUp(i, x, y)));
                Did { sound: Some(UiSound::Pick), ..did }
            }
            None => self.to_browser(UiSound::Pick),
        }
    }

    /// The keys while a piece is in hand: it moves with the cursor, turns,
    /// compresses, is put down or back.
    fn nav_held(&mut self, a: NavAction) -> Did {
        let Some(g) = self.kit.grid.clone() else { return Did::default() };
        let content = self.content();
        let n = self.board_n();
        let (x, y) = self.cell;
        match a {
            NavAction::Previous | NavAction::Next => {
                grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::Turn(a == NavAction::Next));
                Did::cursor()
            }
            NavAction::Alternate => {
                let compresses = self.held.as_ref().is_some_and(|h| g.compresses(h.piece));
                grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::Compress);
                if compresses { Did::cursor() } else { Did::refused() }
            }
            NavAction::Back => {
                let from_board = self.held.as_ref().is_some_and(|h| h.origin.is_some());
                let edit = if from_board { grid::Edit::PutBack } else { grid::Edit::Drop };
                let changed = grid::edit(&g, &content, &mut self.side, &mut self.held, edit);
                Did { sound: Some(UiSound::Back), edited: changed, lists: true, ..Did::default() }
            }
            // (Taken off: one picked up off the board is no longer on it.)
            NavAction::Remove => {
                grid::edit(&g, &content, &mut self.side, &mut self.held, grid::Edit::Drop);
                Did::lists(UiSound::Back)
            }
            NavAction::Confirm => self.press_cell(x, y),
            NavAction::Up | NavAction::Down | NavAction::Left | NavAction::Right => {
                self.pane = 0;
                self.cursor = 0;
                let (dx, dy) = step(a);
                self.cell = ((x + dx).clamp(0, n - 1), (y + dy).clamp(0, n - 1));
                Did::cursor()
            }
        }
    }

    // ---- What was typed ----------------------------------------------------------------

    /// What is typed so far (a search goes as it is typed).
    pub fn typed(&mut self, text: &str) -> Did {
        if self.editing == Some(Editing::Search) {
            self.search = text.to_string();
            self.pick = 0;
            self.work_out_picks();
            return Did { lists: true, ..Did::default() };
        }
        Did::default()
    }

    pub fn typed_done(&mut self, text: &str) -> Did {
        let content = self.content();
        let c = &*content;
        let t = text.trim();
        let Some(what) = self.editing.take() else { return Did::default() };
        match what {
            Editing::Name => {
                let name: String = t.chars().filter(|c| !c.is_control()).take(40).collect();
                if name.is_empty() || name == self.name {
                    return Did::sound(UiSound::Back);
                }
                self.name = name;
                Did { sound: Some(UiSound::Pick), edited: true, ..Did::default() }
            }
            Editing::Number(f) => match t.parse::<i64>() {
                Ok(n) => Did::edit(edit::fact(c, &self.game, &mut self.side, &f, &edit::FactEdit::Number(n))),
                Err(_) if t.is_empty() => Did::edit(edit::fact(c, &self.game, &mut self.side, &f, &edit::FactEdit::Default)),
                Err(_) => Did::refused(),
            },
            Editing::Time(f, i) => match nettai_match::sp_times::parse(t) {
                Ok(frames) => Did::edit(edit::time(c, &mut self.side, &f, i, frames)),
                Err(_) => Did::refused(),
            },
            Editing::Search => {
                self.pane = 1;
                self.pick = if self.picks.is_empty() { -1 } else { 0 };
                Did { sound: Some(UiSound::Pick), lists: true, ..Did::default() }
            }
        }
    }

    pub fn time_of(&self, field: &str, i: usize) -> Option<u16> {
        let Some(Stated::List(items)) = self.side.facts.get(self.kit.content(), field) else { return None };
        let Some(Stated::Record(fields)) = items.get(i) else { return None };
        fields.iter().find_map(|(n, v)| match (n.as_str(), v) {
            ("frames", Stated::Number(f)) => Some(*f as u16),
            _ => None,
        })
    }

    /// The definition a list of times' entry `i` is of.
    pub fn time_def(&self, field: &str, i: usize) -> Option<(Registry, u16)> {
        let Some(Stated::List(items)) = self.side.facts.get(self.kit.content(), field) else { return None };
        let Some(Stated::Record(fields)) = items.get(i) else { return None };
        fields.iter().find_map(|(_, v)| match v {
            Stated::Def(r, Some(h)) => Some((*r, *h)),
            _ => None,
        })
    }

    /// What is being typed: its label, its text to start from, a hint.
    pub fn edit_label(&self) -> (String, String, String) {
        let c = self.kit.content();
        let names = self.names();
        match &self.editing {
            Some(Editing::Name) => ("name".into(), self.name.clone(), String::new()),
            Some(Editing::Number(f)) => {
                let range = facts::field(c, f).and_then(|f| facts::range(f.ty)).map_or(String::new(), |(lo, hi, _)| format!("{lo} – {hi}"));
                let now = match self.side.facts.get(c, f) {
                    Some(Stated::Number(n) | Stated::Optional(Some(n))) => n.to_string(),
                    _ => String::new(),
                };
                (layout::title(f).to_uppercase(), now, range)
            }
            Some(Editing::Time(f, i)) => {
                let name = self.time_def(f, *i).map(|(r, h)| names.def(r, h)).unwrap_or_default();
                (name.to_uppercase(), nettai_match::sp_times::format(self.time_of(f, *i).unwrap_or(0)), "mm:ss.cc".into())
            }
            Some(Editing::Search) => ("search".into(), self.search.clone(), String::new()),
            None => Default::default(),
        }
    }
}

/// A direction's step on the board.
fn step(a: NavAction) -> (i32, i32) {
    match a {
        NavAction::Up => (0, -1),
        NavAction::Down => (0, 1),
        NavAction::Left => (-1, 0),
        _ => (1, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The round a build starts shows the face its navi starts with:
    /// EXE5's dark MegaMan his dark one, the light one his plain one; a
    /// team navi its own.
    #[test]
    fn a_round_shows_the_face_its_navi_starts_with() {
        let five = nettai_match::testing::exe5_content();
        let m = nettai_match::pick::live(&five, "exe5", 3, None).unwrap();
        let face = |navi: &str, dark: bool| {
            let mut side = m.sides[0].clone();
            edit::switch_navi(&five, &mut side, nettai_match::ids::navi(&five, "exe5", navi).unwrap());
            layout::as_built(&five, "exe5", &mut side);
            if dark {
                presets::step(&five, "exe5", &mut side, "karma", 1);
            }
            checked(&five, "exe5", &side).1.map(|r| r.face).unwrap_or_else(|e| panic!("{navi}: {e}"))
        };
        let (light, dark) = (face("megaman", false), face("megaman", true));
        let (Some(Face::Form(form, plain, false)), Some(Face::Form(dark_form, gloom, false))) = (&light, &dark) else { panic!("{light:?} {dark:?}") };
        assert_eq!(form, dark_form);
        assert_ne!(plain, gloom, "the dark MegaMan's face is his own");
        let protoman = nettai_match::ids::navi(&five, "exe5", "protoman").unwrap();
        assert_eq!(face("protoman", true), Some(Face::Navi(protoman)));
    }
}
