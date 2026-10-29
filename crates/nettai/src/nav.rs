//! Focus and navigation (docs/design/app-ui.md §5, rules 5 to 10; docs/app.md §11).
//!
//! There is one focus, and it is here. Every focusable control tells this module where it is in the window (the
//! [`Registry`], which the UI's `Focusable`, `ScrollPage` and `ListView` fill), so the neighbor in a direction is
//! found from the rectangles: no screen writes key handling of its own, and a control added to a screen is
//! reachable by being there. Tab goes through the same rectangles in reading order. A screen's last focus is
//! remembered by the view's name ([`Nav::enter`]); a dialog or a menu over a screen is a scope, the prefix its
//! controls' names share, and while one is open the keys reach nothing else ([`Nav::open`]).
//!
//! What a key does to the focused control is decided here too ([`Nav::confirm`], [`Nav::go`]): the control's
//! kind says whether Left and Right change it or move on, and whether Enter activates it, takes it (a setting, which
//! Left and Right then change: [`Nav::grabbed`]) or starts an edit.
//! The app is told what happened ([`Did`]) and passes it to the screen's Rust module.

use crate::input::Scroll;
use nettai_ui::{AppWindow, NavKind};
use slint::{ComponentHandle, Model, ModelRc, ModelTracker};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// A rectangle in the window, in the UI's units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// A control: its name, and its index (a row's place in its list, else 0).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub id: String,
    pub index: i32,
}

impl Key {
    pub fn new(id: &str, index: i32) -> Key {
        Key { id: id.to_string(), index }
    }
}

/// What a control does with the keys (the UI's `NavKind`, less the kinds that aren't controls).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Button,
    /// A setting: Enter takes it, and Left and Right change it while it is taken.
    Stepper,
    Field,
    /// A row of a list (`ListView`).
    Row,
}

struct Item {
    rect: Rect,
    kind: Kind,
    enabled: bool,
    /// How many of it exist (a row made again exists twice for a moment).
    alive: u32,
}

/// A scrolling page: where its view is, how tall what it holds is, and how far it is scrolled. The controls whose
/// names begin with its name are in it.
struct Page {
    view: Rect,
    content: f32,
    scroll: f32,
    alive: u32,
}

/// A list of rows: its view and how tall its rows are together (never less than the view), and its rows' count,
/// pitch, height, width, and how far it is scrolled.
struct List {
    view: Rect,
    content: f32,
    rows: i32,
    pitch: f32,
    row_height: f32,
    row_width: f32,
    scroll: f32,
    alive: u32,
}

/// Where every focusable thing on screen is. The UI fills it (`main::wire`); a control leaves it when it is
/// destroyed ([`Token`]).
#[derive(Default)]
pub struct Registry {
    items: HashMap<Key, Item>,
    pages: HashMap<String, Page>,
    lists: HashMap<String, List>,
}

/// What a registered thing holds while it exists: a model of no rows, which the UI keeps in a property and
/// drops with the control. Dropping it takes the control out of the registry (Slint tells nothing when an
/// element is destroyed; a property's value being dropped is the one sign there is).
struct Token {
    registry: Weak<RefCell<Registry>>,
    key: Key,
    kind: NavKind,
}

impl Model for Token {
    type Data = i32;

    fn row_count(&self) -> usize {
        0
    }

    fn row_data(&self, _: usize) -> Option<i32> {
        None
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &()
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        let Some(registry) = self.registry.upgrade() else { return };
        // (A control is never destroyed inside one of the registry's own calls.)
        let Ok(mut r) = registry.try_borrow_mut() else { return };
        fn gone<K: std::hash::Hash + Eq, V>(map: &mut HashMap<K, V>, key: &K, alive: impl Fn(&mut V) -> &mut u32) {
            if let Some(v) = map.get_mut(key) {
                *alive(v) -= 1;
                if *alive(v) == 0 {
                    map.remove(key);
                }
            }
        }
        match self.kind {
            NavKind::Page => gone(&mut r.pages, &self.key.id, |p| &mut p.alive),
            NavKind::List => gone(&mut r.lists, &self.key.id, |l| &mut l.alive),
            _ => gone(&mut r.items, &self.key, |i| &mut i.alive),
        }
    }
}

impl Registry {
    /// A control (or a page, or a list) exists: the token to keep while it does.
    pub fn register(registry: &Rc<RefCell<Registry>>, id: &str, index: i32, kind: NavKind) -> ModelRc<i32> {
        let key = Key::new(id, index);
        {
            let mut r = registry.borrow_mut();
            match kind {
                NavKind::Page => r.pages.entry(key.id.clone()).or_insert(Page { view: Rect::default(), content: 0.0, scroll: 0.0, alive: 0 }).alive += 1,
                NavKind::List => {
                    let fresh = List { view: Rect::default(), content: 0.0, rows: 0, pitch: 0.0, row_height: 0.0, row_width: 0.0, scroll: 0.0, alive: 0 };
                    r.lists.entry(key.id.clone()).or_insert(fresh).alive += 1;
                }
                _ => {
                    let kind = match kind {
                        NavKind::Stepper => Kind::Stepper,
                        NavKind::Field => Kind::Field,
                        _ => Kind::Button,
                    };
                    let item = r.items.entry(key.clone()).or_insert(Item { rect: Rect::default(), kind, enabled: true, alive: 0 });
                    item.alive += 1;
                    item.kind = kind;
                }
            }
        }
        ModelRc::new(Token { registry: Rc::downgrade(registry), key, kind })
    }

    /// Where a control is, and whether the focus can go to it.
    pub fn place(&mut self, id: &str, index: i32, rect: Rect, enabled: bool) {
        if let Some(item) = self.items.get_mut(&Key::new(id, index)) {
            item.rect = rect;
            item.enabled = enabled;
        }
    }

    /// Where a page's (or a list's) view is, how tall what it holds is, and how far it is scrolled.
    pub fn page(&mut self, id: &str, view: Rect, content: f32, scroll: f32) {
        if let Some(p) = self.pages.get_mut(id) {
            (p.view, p.content, p.scroll) = (view, content, scroll);
        }
        if let Some(l) = self.lists.get_mut(id) {
            (l.view, l.content, l.scroll) = (view, content, scroll);
        }
    }

    /// A page's or a list's view, how far it is scrolled, and how far it can be, where it is laid out and can
    /// scroll.
    fn scrolling(&self, id: &str) -> Option<(Rect, f32, f32)> {
        let (view, content, scroll) = match self.lists.get(id) {
            Some(l) => (l.view, l.content, l.scroll),
            None => self.pages.get(id).map(|p| (p.view, p.content, p.scroll))?,
        };
        let span = content - view.h;
        (view.w > 0.0 && view.h > 0.0 && span > 0.5).then_some((view, scroll, span))
    }

    /// What a scroll that leaves the focus where it is moves ([`Nav::scroll`]): the page or the list `focus` is
    /// in, where it can scroll, else the one under `scope` with the most to scroll. Its name, its view, how far it
    /// is scrolled and how far it can be.
    fn scroll_target(&self, focus: Option<&Key>, scope: &str) -> Option<(&str, Rect, f32, f32)> {
        let focused = focus.and_then(|key| match self.lists.get_key_value(&key.id) {
            Some((id, _)) => Some(id.as_str()),
            None => self.page_of(&key.id).map(|(name, _)| name),
        });
        let most = || {
            let ids = self.lists.keys().chain(self.pages.keys()).filter(|id| id.starts_with(scope));
            // (Of two as far, the one first by name, so the choice doesn't change from one call to the next.)
            let spans = ids.filter_map(|id| self.scrolling(id).map(|(_, _, span)| (id.as_str(), span)));
            spans.max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(a.0))).map(|(id, _)| id)
        };
        let id = focused.filter(|id| self.scrolling(id).is_some()).or_else(most)?;
        let (view, at, span) = self.scrolling(id)?;
        Some((id, view, at, span))
    }

    /// A list's rows.
    pub fn list(&mut self, id: &str, rows: i32, pitch: f32, row_height: f32, scroll: f32, row_width: f32) {
        if let Some(l) = self.lists.get_mut(id) {
            (l.rows, l.pitch, l.row_height, l.scroll, l.row_width) = (rows, pitch, row_height, scroll, row_width);
        }
    }

    /// Every control's name and size, as laid out now (the screenshot pass's check that motion changes none).
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn sizes(&self) -> Vec<(Key, (f32, f32))> {
        self.items.iter().map(|(key, item)| (key.clone(), (item.rect.w, item.rect.h))).collect()
    }

    fn kind(&self, key: &Key) -> Option<Kind> {
        match self.lists.get(&key.id) {
            Some(_) => Some(Kind::Row),
            None => self.items.get(key).map(|i| i.kind),
        }
    }

    /// Whether the focus can be on `key`: it exists, it is enabled and laid out; a list's row is one of its rows.
    fn holds(&self, key: &Key) -> bool {
        match self.lists.get(&key.id) {
            Some(l) => (0..l.rows).contains(&key.index),
            None => self.items.get(key).is_some_and(|i| i.enabled && i.rect.w > 0.0 && i.rect.h > 0.0),
        }
    }

    /// Where `key` is: a control's rectangle, a list's row (what of it is in the list's view).
    pub fn rect(&self, key: &Key) -> Option<Rect> {
        match self.lists.get(&key.id) {
            Some(l) => {
                let y = l.view.y + key.index as f32 * l.pitch - l.scroll;
                let (top, bottom) = (y.max(l.view.y), (y + l.row_height).min(l.view.bottom()));
                Some(Rect { x: l.view.x, y: top, w: l.row_width, h: (bottom - top).max(0.0) })
            }
            None => self.items.get(key).map(|i| i.rect),
        }
    }

    /// Where a page's or a list's view is.
    pub fn view(&self, id: &str) -> Option<Rect> {
        self.lists.get(id).map(|l| l.view).or_else(|| self.pages.get(id).map(|p| p.view))
    }

    /// The row of list `id` at height `y` in the window, as it is scrolled (it may be past its rows: below 0, or
    /// `rows` or more).
    pub fn row_at(&self, id: &str, y: f32) -> Option<i32> {
        let l = self.lists.get(id).filter(|l| l.pitch > 0.0)?;
        Some(((y - l.view.y + l.scroll) / l.pitch).floor() as i32)
    }

    /// Where list `id`'s first row is in the window, as it is scrolled (above its view where it is scrolled down), and
    /// its rows' pitch.
    pub fn first_row(&self, id: &str) -> Option<(f32, f32)> {
        let l = self.lists.get(id).filter(|l| l.pitch > 0.0)?;
        Some((l.view.y - l.scroll, l.pitch))
    }

    /// The list's row or the control at `(x, y)` in the window, if there is one there.
    pub fn at(&self, (x, y): (f32, f32)) -> Option<Key> {
        let inside = |r: &Rect| x >= r.x && x < r.right() && y >= r.y && y < r.bottom();
        let row = self.lists.iter().filter(|(_, l)| inside(&l.view)).find_map(|(id, l)| self.row_at(id, y).filter(|r| (0..l.rows).contains(r)).map(|r| Key::new(id, r)));
        row.or_else(|| self.items.iter().find(|(_, i)| i.rect.w > 0.0 && inside(&i.rect)).map(|(k, _)| k.clone()))
    }

    /// The page `id` is in: the one with the longest name that begins `id`.
    fn page_of(&self, id: &str) -> Option<(&str, Rect)> {
        self.pages.iter().filter(|(name, _)| id.starts_with(name.as_str())).max_by_key(|(name, _)| name.len()).map(|(name, p)| (name.as_str(), p.view))
    }

    /// The control a direction moves the focus to from `key` (not a list's next row), under `scope`. Up and Down
    /// in a scrolling page go through its controls, those scrolled out of view too, before they leave it for what
    /// is beside it (the user: "it should scroll the entire length before moving to buttons underneath").
    fn next(&self, key: &Key, dir: Direction, scope: &str, row_of: &dyn Fn(&str) -> i32) -> Option<Key> {
        let from = self.rect(key)?;
        let candidates = self.candidates(scope, Some(key), row_of);
        let page = self.page_of(&key.id).map(|(name, _)| name.to_string());
        let within = page.filter(|_| matches!(dir, Direction::Up | Direction::Down)).and_then(|page| {
            let inside: Vec<(Key, Rect)> = candidates.iter().filter(|(k, _)| self.page_of(&k.id).is_some_and(|(name, _)| name == page)).cloned().collect();
            neighbor(from, dir, &inside)
        });
        within.or_else(|| neighbor(from, dir, &candidates))
    }

    /// What the focus can go to under `scope` from `from`: every control that holds it, and each list with
    /// rows, as its view (it is entered at the row `row_of` says). A page's controls scrolled out of its view
    /// are passed by, unless the focus is in that page already.
    fn candidates(&self, scope: &str, from: Option<&Key>, row_of: &dyn Fn(&str) -> i32) -> Vec<(Key, Rect)> {
        let from_page = from.and_then(|k| self.page_of(&k.id)).map(|(name, _)| name.to_string());
        let mut out = Vec::new();
        for (key, item) in &self.items {
            if !key.id.starts_with(scope) || !self.holds(key) || from == Some(key) {
                continue;
            }
            if let Some((page, view)) = self.page_of(&key.id)
                && from_page.as_deref() != Some(page)
            {
                let middle = item.rect.center().1;
                if middle < view.y || middle > view.bottom() {
                    continue;
                }
            }
            out.push((key.clone(), item.rect));
        }
        for (id, list) in &self.lists {
            if !id.starts_with(scope) || list.rows == 0 || from.is_some_and(|k| k.id == *id) {
                continue;
            }
            out.push((Key::new(id, row_of(id).clamp(0, list.rows - 1)), list.view));
        }
        // (In an order that doesn't change from one call to the next: equal scores go to the same one.)
        out.sort_by(|a, b| (a.1.y, a.1.x).partial_cmp(&(b.1.y, b.1.x)).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.0.id.cmp(&b.0.id)));
        out
    }
}

/// The neighbor of `from` in direction `dir` among `candidates`. One that lies in `from`'s beam (it overlaps
/// `from` across the direction: beside it, for Up and Down) is taken before any that doesn't; among those in
/// the beam the nearest along the direction, among the others the nearest counting the way across twice; of two
/// equally near, the one whose leading edge is nearest `from`'s.
pub fn neighbor(from: Rect, dir: Direction, candidates: &[(Key, Rect)]) -> Option<Key> {
    const EDGE: f32 = 0.5;
    let mut best: Option<(bool, f32, f32, &Key)> = None;
    for (key, c) in candidates {
        let (fx, fy) = from.center();
        let (cx, cy) = c.center();
        // Past `from` in the direction: its middle beyond `from`'s middle, its far edge beyond `from`'s.
        let (ahead, gap) = match dir {
            Direction::Down => (cy > fy + EDGE && c.bottom() > from.bottom() + EDGE, c.y - from.bottom()),
            Direction::Up => (cy < fy - EDGE && c.y < from.y - EDGE, from.y - c.bottom()),
            Direction::Right => (cx > fx + EDGE && c.right() > from.right() + EDGE, c.x - from.right()),
            Direction::Left => (cx < fx - EDGE && c.x < from.x - EDGE, from.x - c.right()),
        };
        if !ahead {
            continue;
        }
        let gap = gap.max(0.0);
        // How far apart the two are across the direction (0: they overlap, it is in the beam).
        let across = match dir {
            Direction::Up | Direction::Down => (c.x - from.right()).max(from.x - c.right()).max(0.0),
            Direction::Left | Direction::Right => (c.y - from.bottom()).max(from.y - c.bottom()).max(0.0),
        };
        let beam = across <= 0.0;
        let score = if beam { gap } else { gap + 2.0 * across };
        // Between two equally near, the one that starts where `from` does: under a wide item, the first of
        // the small ones in its row (Settings before Exit under REPLAYS), not the one nearest its middle.
        let off = match dir {
            Direction::Up | Direction::Down => (c.x - from.x).abs(),
            Direction::Left | Direction::Right => (c.y - from.y).abs(),
        };
        let better = match best {
            None => true,
            Some((b, s, o, _)) => (beam && !b) || (beam == b && (score < s - 0.01 || ((score - s).abs() <= 0.01 && off < o - 0.01))),
        };
        if better {
            best = Some((beam, score, off, key));
        }
    }
    best.map(|(_, _, _, key)| key.clone())
}

/// `candidates` in reading order: by rows from the top, each row from its left. Two are in one row when the
/// second's middle is within the first's height.
pub fn reading_order(mut candidates: Vec<(Key, Rect)>) -> Vec<Key> {
    candidates.sort_by(|a, b| a.1.center().1.partial_cmp(&b.1.center().1).unwrap_or(std::cmp::Ordering::Equal));
    let mut rows: Vec<Vec<(Key, Rect)>> = Vec::new();
    for (key, rect) in candidates {
        match rows.last_mut() {
            Some(row) if rect.center().1 < row[0].1.bottom() && rect.y < row[0].1.bottom() => row.push((key, rect)),
            _ => rows.push(vec![(key, rect)]),
        }
    }
    for row in &mut rows {
        row.sort_by(|a, b| a.1.x.partial_cmp(&b.1.x).unwrap_or(std::cmp::Ordering::Equal));
    }
    rows.into_iter().flatten().map(|(key, _)| key).collect()
}

/// `order` (reading order) with the controls named in `zones` first, in that order, and the rest after them as
/// they were.
pub fn in_zones(mut order: Vec<Key>, zones: &[&str]) -> Vec<Key> {
    order.sort_by_key(|k| zones.iter().position(|z| *z == k.id).unwrap_or(zones.len()));
    order
}

/// What a key did to the focused control, for the app to pass to the screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Did {
    /// Nothing the screen needs to hear of.
    Nothing,
    /// Enter, a click, a tap.
    Activate(Key),
    /// Left or Right on a setting taken, a click on its arrow: -1 or 1.
    Step(Key, i32),
    /// A field's text: as it is typed (`done` false), then as it is left (confirmed, or restored).
    Edit { key: Key, text: String, done: bool },
}

/// A text field being edited: the text it had (Esc restores it) and the text it has.
struct Edit {
    key: Key,
    before: String,
    text: String,
}

/// A scope over the screen (a dialog, a menu, a list of choices): its controls' prefix, and where the focus was
/// under it.
struct Scope {
    prefix: String,
    main: Key,
    under: Option<Key>,
}

/// The focus: where it is, where it was on each view, and what is open over the screen.
pub struct Nav {
    registry: Rc<RefCell<Registry>>,
    focus: Option<Key>,
    /// Where the focus should be once that control exists (a screen just shown, a dialog just opened).
    wanted: Option<Key>,
    view: String,
    main: Option<Key>,
    memory: HashMap<String, Key>,
    /// Each list's last focused row.
    rows: HashMap<String, i32>,
    scopes: Vec<Scope>,
    edit: Option<Edit>,
    /// The setting taken (Enter on a stepper): Left and Right change it until it is let go or the focus leaves it.
    grab: Option<Key>,
    edit_seq: i32,
    reveal_seq: i32,
    /// A keyboard or a pad was the last thing used (the cursor shows), and which.
    shown: bool,
    pad: bool,
    /// What the UI was last told: the focus and its place.
    told: Option<(Key, Rect)>,
    /// Where the cursor is drawn: on its way to the focus's place, and when it was last moved.
    drawn: Option<(Rect, nettai_platform::time::Instant)>,
    /// The focus was put somewhere by the app: it is scrolled into view once it is there.
    reveal_due: bool,
    /// A dialog opened on its text field ([`Nav::open_editing`]): the field's edit begins once the focus is on it.
    edit_due: bool,
    /// The screen shown is leaving: the focus waits for the next one's controls, on nothing meanwhile.
    held: bool,
    /// The presses of Enter on a control, counted for the UI (the control flashes, the cursor snaps in).
    presses: i32,
    /// The view shown keeps no cursor of its own ([`Nav::bare`]).
    bare: bool,
}

impl Default for Nav {
    fn default() -> Nav {
        Nav::new()
    }
}

impl Nav {
    pub fn new() -> Nav {
        Nav {
            registry: Rc::new(RefCell::new(Registry::default())),
            focus: None,
            wanted: None,
            view: String::new(),
            main: None,
            memory: HashMap::new(),
            rows: HashMap::new(),
            scopes: Vec::new(),
            edit: None,
            grab: None,
            edit_seq: 0,
            reveal_seq: 0,
            shown: false,
            pad: false,
            told: None,
            drawn: None,
            reveal_due: false,
            edit_due: false,
            held: false,
            presses: 0,
            bare: false,
        }
    }

    /// The registry, for the UI's callbacks to fill.
    pub fn registry(&self) -> Rc<RefCell<Registry>> {
        self.registry.clone()
    }

    /// Where the cursor is: the focused control's place, as the UI was last told it.
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn cursor(&self) -> Option<Rect> {
        self.told.as_ref().map(|(_, rect)| *rect)
    }

    /// The focused control.
    pub fn focused(&self) -> Option<&Key> {
        self.focus.as_ref()
    }

    /// The prefix the keys are held to: the scope open over the screen, else none.
    fn scope(&self) -> &str {
        self.scopes.last().map_or("", |s| s.prefix.as_str())
    }

    /// A view is shown (a screen, or a part of one that replaces the rest: a lobby, an editor's tab): the
    /// focus goes where it last was on it, else to `main`, its main control. The view left keeps its own.
    pub fn enter(&mut self, view: &str, main: &str) {
        if self.view == view {
            return;
        }
        self.leave_edit();
        self.grab = None;
        if let Some(key) = self.scopes.first().map_or(self.focus.take(), |s| s.under.clone()) {
            self.memory.insert(std::mem::take(&mut self.view), key);
        }
        self.scopes.clear();
        self.bare = false;
        self.view = view.to_string();
        self.main = Some(Key::new(main, 0));
        self.focus = None;
        self.wanted = self.memory.get(view).cloned().or_else(|| self.main.clone());
    }

    /// The view just entered shows no cursor of its own (a replay's viewer, whose keys are all its own:
    /// `Screen::claim`): the cursor shows only over a scope opened on it (a menu, a dialog, the export mode).
    pub fn bare(&mut self) {
        self.bare = true;
    }

    /// A scope opens over the screen (`prefix`: what its controls' names begin with), the focus on `main`.
    /// Until it closes the keys reach its controls alone.
    pub fn open(&mut self, prefix: &str, main: Key) {
        if self.scopes.iter().any(|s| s.prefix == prefix) {
            return;
        }
        self.leave_edit();
        self.grab = None;
        self.scopes.push(Scope { prefix: prefix.to_string(), main: main.clone(), under: self.focus.take() });
        self.wanted = Some(main);
    }

    /// A dialog opens on its text field `main` (a build's name, an address): as [`Nav::open`], and the field is
    /// edited as soon as the focus is on it, its text all selected, so what is typed replaces it (the user:
    /// "you should auto focus the primary text field on dialogs that pop up").
    pub fn open_editing(&mut self, prefix: &str, main: Key) {
        self.open(prefix, main);
        self.edit_due = true;
    }

    /// The scope closes: the focus goes back where it was under it.
    pub fn close(&mut self, prefix: &str) {
        let Some(at) = self.scopes.iter().position(|s| s.prefix == prefix) else { return };
        self.leave_edit();
        self.grab = None;
        let under = self.scopes[at].under.clone();
        self.scopes.truncate(at);
        self.focus = None;
        self.wanted = under;
        self.edit_due = false;
    }

    /// Whether the scope `prefix` is open.
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn is_open(&self, prefix: &str) -> bool {
        self.scopes.iter().any(|s| s.prefix == prefix)
    }

    /// Put the focus on a control (once it exists, if it doesn't yet), scrolled into view.
    pub fn focus(&mut self, id: &str, index: i32) {
        self.leave_edit();
        self.reveal_due = true;
        let key = Key::new(id, index);
        if self.registry.borrow().holds(&key) {
            self.take(key);
        } else {
            self.wanted = Some(key);
        }
    }

    /// The row the focus comes to list `id` at, the next time it comes from outside it: where the row is the
    /// screen's to say, not the one the focus last left (a box of a code's characters).
    pub fn set_row(&mut self, id: &str, row: i32) {
        self.rows.insert(id.to_string(), row);
    }

    fn take(&mut self, key: Key) {
        if self.registry.borrow().lists.contains_key(&key.id) {
            self.rows.insert(key.id.clone(), key.index);
        }
        if self.focus.as_ref() != Some(&key) {
            self.grab = None;
        }
        self.focus = Some(key);
        self.wanted = None;
    }

    /// Whether the focused control is a setting taken: Left and Right change it.
    pub fn grabbed(&self) -> bool {
        self.grab.is_some() && self.grab == self.focus
    }

    /// The setting taken is let go: the arrows move on again.
    pub fn let_go(&mut self) {
        self.grab = None;
    }

    /// While the screen shown leaves (`held`), the focus is on nothing: it goes to the next screen's control once
    /// that exists, never to one of the screen leaving.
    pub fn hold(&mut self, held: bool) {
        self.held = held;
    }

    /// Whether the focus is waiting for a control that doesn't exist yet (a screen just shown).
    pub fn waiting(&self) -> bool {
        self.focus.is_none() && self.wanted.is_some()
    }

    /// Enter was pressed on the focused control: the UI is told, for the control's flash and the cursor's snap.
    pub fn pressed(&mut self, ui: &AppWindow) {
        self.presses += 1;
        ui.global::<nettai_ui::Nav>().set_press_seq(self.presses);
    }

    /// A keyboard (`pad` false) or a pad was used: the cursor shows, the hints are that device's.
    pub fn used_keys(&mut self, pad: bool) {
        self.shown = true;
        self.pad = pad;
    }

    /// The pointer was used: the cursor hides.
    pub fn used_pointer(&mut self) {
        self.shown = false;
    }

    /// Show or hide the cursor whatever was used (the screenshot pass).
    pub fn show_cursor(&mut self, shown: bool, pad: bool) {
        self.shown = shown;
        self.pad = pad;
    }

    /// Whether a pad was the last of keyboard and pad used.
    pub fn pad(&self) -> bool {
        self.pad
    }

    /// Whether the pointer was the last thing used (the cursor is hidden): a click on a list's row may choose it
    /// where Enter acts on it.
    pub fn pointing(&self) -> bool {
        !self.shown
    }

    fn row_of(&self) -> impl Fn(&str) -> i32 + '_ {
        |id| self.rows.get(id).copied().unwrap_or(0)
    }

    /// Keep the one focus: if it is on nothing that exists (a screen just shown, a control gone), it goes to
    /// the control wanted, else the view's main control, else the first in reading order. Then tell the UI the
    /// focus, and where the cursor is at `now`: with `glide` it goes a share of the way to the focused control
    /// each frame (most of it in a tenth of a second), so a control that is itself moving is followed; without,
    /// it is there at once. The focus, when it moved to another control.
    pub fn settle(&mut self, ui: &AppWindow, now: nettai_platform::time::Instant, glide: bool) -> Option<Key> {
        let before = self.told.as_ref().map(|(k, _)| k.clone());
        // What the focus should be on: what is wanted, if it exists now; else where it is, if that still
        // exists; else somewhere that does.
        enum Next {
            Wanted(Key),
            Stay,
            Fallback(Option<Key>),
        }
        let next = {
            let registry = self.registry.borrow();
            let scope = self.scope();
            let fits = |key: &Key| key.id.starts_with(scope) && registry.holds(key);
            if self.held {
                Next::Fallback(None)
            } else if let Some(wanted) = self.wanted.clone().filter(|k| fits(k)) {
                Next::Wanted(wanted)
            } else if self.focus.as_ref().is_some_and(fits) {
                Next::Stay
            } else {
                // (A list whose rows went: its last row, while it has one.)
                let shrunk = self.focus.as_ref().and_then(|k| registry.lists.get(&k.id).filter(|l| l.rows > 0).map(|l| Key::new(&k.id, k.index.clamp(0, l.rows - 1))));
                let main = self.scopes.last().map(|s| s.main.clone()).or_else(|| self.main.clone()).filter(|k| fits(k));
                let first = || reading_order(registry.candidates(scope, None, &self.row_of())).into_iter().next();
                Next::Fallback(shrunk.or(main).or_else(first))
            }
        };
        if self.reveal_due && self.wanted.is_none() && self.focus.is_some() {
            self.reveal_due = false;
            self.reveal(ui);
        }
        match next {
            Next::Wanted(key) => {
                self.take(key);
                if std::mem::take(&mut self.reveal_due) {
                    self.reveal(ui);
                }
                if std::mem::take(&mut self.edit_due) && self.on_field() {
                    self.begin_edit(ui, "");
                }
            }
            Next::Stay => {}
            Next::Fallback(Some(key)) => {
                // (What was wanted stays wanted: it may exist a frame later.)
                let wanted = self.wanted.take();
                self.take(key);
                self.wanted = wanted;
            }
            Next::Fallback(None) => {
                self.focus = None;
                self.grab = None;
            }
        }
        let rect = self.focus.as_ref().and_then(|k| self.registry.borrow().rect(k));
        let focused = self.focus.clone().zip(rect);
        let nav = ui.global::<nettai_ui::Nav>();
        if focused != self.told {
            match &focused {
                Some((key, _)) => {
                    nav.set_id(key.id.as_str().into());
                    nav.set_index(key.index);
                }
                None => {
                    nav.set_id(Default::default());
                    nav.set_index(0);
                }
            }
            self.told = focused;
        }
        // The cursor: toward the focus's place, or there at once where it wasn't showing.
        const GLIDE: f32 = 0.032;
        let shown = self.shown && !(self.bare && self.scopes.is_empty());
        let target = self.told.as_ref().map(|(_, rect)| *rect).filter(|_| shown);
        let drawn = match (self.drawn, target) {
            (Some((at, last)), Some(to)) if glide => {
                let share = 1.0 - (-now.saturating_duration_since(last).as_secs_f32() / GLIDE).exp();
                let toward = |a: f32, b: f32| if (b - a).abs() < 0.3 { b } else { a + (b - a) * share };
                Some(Rect { x: toward(at.x, to.x), y: toward(at.y, to.y), w: toward(at.w, to.w), h: toward(at.h, to.h) })
            }
            (_, to) => to,
        };
        if let Some(rect) = drawn
            && self.drawn.map(|(r, _)| r) != Some(rect)
        {
            nav.set_x(rect.x);
            nav.set_y(rect.y);
            nav.set_w(rect.w);
            nav.set_h(rect.h);
        }
        self.drawn = drawn.map(|rect| (rect, now));
        nav.set_shown(shown);
        nav.set_pad(self.pad);
        nav.set_editing(self.edit.is_some());
        nav.set_grabbed(self.grabbed());
        let after = self.told.as_ref().map(|(k, _)| k.clone());
        if after != before { after } else { None }
    }

    /// Ask the page or the list the focus is in to scroll it into view (after a key moved it; a click is on
    /// what is in view already).
    fn reveal(&mut self, ui: &AppWindow) {
        let Some(key) = &self.focus else { return };
        let registry = self.registry.borrow();
        let (name, view, rect) = match registry.lists.get(&key.id) {
            Some(l) => {
                let y = l.view.y + key.index as f32 * l.pitch - l.scroll;
                (key.id.as_str(), l.view, Rect { x: l.view.x, y, w: l.row_width, h: l.row_height })
            }
            None => {
                let (Some((name, view)), Some(rect)) = (registry.page_of(&key.id), registry.rect(key)) else { return };
                // The page's first control shows the page's start, and its last the end (a note under it, the
                // room under that): the page clamps what is asked.
                // Otherwise there is room for the group's title over a control at the view's top.
                let others = || registry.items.iter().filter(|(k, _)| *k != key && k.id.starts_with(name)).map(|(_, item)| item.rect.y);
                let above = if others().all(|y| y > rect.y) { 10_000.0 } else { 12.0 };
                let below = if others().all(|y| y < rect.y) { 10_000.0 } else { 12.0 };
                (name, view, Rect { y: rect.y - above, h: rect.h + above + below, ..rect })
            }
        };
        let dy = if rect.y < view.y {
            rect.y - view.y
        } else if rect.bottom() > view.bottom() {
            (rect.bottom() - view.bottom()).min(rect.y - view.y)
        } else {
            return;
        };
        let name = name.to_string();
        drop(registry);
        self.ask_scroll(ui, &name, dy, true);
    }

    /// Ask the page or the list `id` to scroll by `dy`, slid there or at once (the UI clamps it to its ends).
    fn ask_scroll(&mut self, ui: &AppWindow, id: &str, dy: f32, slides: bool) {
        let nav = ui.global::<nettai_ui::Nav>();
        self.reveal_seq += 1;
        nav.set_reveal_id(id.into());
        nav.set_reveal_dy(dy);
        nav.set_reveal_slides(slides);
        nav.set_reveal_seq(self.reveal_seq);
    }

    /// A scroll that leaves the focus where it is (the right stick; Shift with Page Up, Page Down, Home or End), of
    /// the page or the list the focus is in where it can scroll, else of the one under the scope open that has the
    /// most to scroll. True if it moved.
    pub fn scroll(&mut self, ui: &AppWindow, by: Scroll) -> bool {
        let (id, dy) = {
            let registry = self.registry.borrow();
            let Some((id, view, at, span)) = registry.scroll_target(self.focus.as_ref(), self.scope()) else { return false };
            let to = by.to(at, span, view.h);
            if (to - at).abs() < 0.01 {
                return false;
            }
            (id.to_string(), to - at)
        };
        self.ask_scroll(ui, &id, dy, !matches!(by, Scroll::By(_)));
        true
    }

    /// The list `id` back at its top, as its rows changed under it (a search typed): scrolled to its first row,
    /// slid there as a reveal is, and entered at it next. The focus doesn't move.
    pub fn list_to_top(&mut self, ui: &AppWindow, id: &str) {
        self.rows.insert(id.to_string(), 0);
        let scroll = self.registry.borrow().lists.get(id).map_or(0.0, |l| l.scroll);
        if scroll <= 0.0 {
            return;
        }
        self.ask_scroll(ui, id, -scroll, true);
    }

    /// Scroll the list `id`'s row `index` into view, slid there as a reveal is, and enter the list at it next,
    /// without moving the focus (the replay viewer's queue, at the replay watched). False if the list isn't laid
    /// out yet.
    #[allow(dead_code)]
    pub fn reveal_row(&mut self, ui: &AppWindow, id: &str, index: i32) -> bool {
        self.rows.insert(id.to_string(), index);
        let dy = {
            let registry = self.registry.borrow();
            let Some(l) = registry.lists.get(id) else { return false };
            let y = l.view.y + index as f32 * l.pitch - l.scroll;
            if y < l.view.y {
                y - l.view.y
            } else if y + l.row_height > l.view.bottom() {
                y + l.row_height - l.view.bottom()
            } else {
                return true;
            }
        };
        self.ask_scroll(ui, id, dy, true);
        true
    }

    /// Scroll the page `id`'s control into view at its top (a settings group jumped to), without moving the
    /// focus.
    pub fn reveal_at_top(&mut self, ui: &AppWindow, id: &str, index: i32, room: f32) {
        let (name, dy) = {
            let registry = self.registry.borrow();
            let (Some((name, view)), Some(rect)) = (registry.page_of(id), registry.rect(&Key::new(id, index))) else { return };
            (name.to_string(), rect.y - room - view.y)
        };
        self.ask_scroll(ui, &name, dy, true);
    }

    /// An arrow: the focus goes to the neighbor in that direction, a list's row to the next one. On a setting
    /// taken Left and Right change its value instead ([`Did::Step`]); Up and Down let it go and move on.
    pub fn go(&mut self, ui: &AppWindow, dir: Direction) -> Did {
        let Some(key) = self.focus.clone() else { return Did::Nothing };
        let step = match dir {
            Direction::Left => -1,
            Direction::Right => 1,
            _ => 0,
        };
        let grabbed = self.grabbed();
        if step == 0 {
            self.grab = None;
        }
        let next = {
            let registry = self.registry.borrow();
            if step != 0 && grabbed {
                return Did::Step(key, step);
            }
            // A list's rows first.
            let row = registry.lists.get(&key.id).and_then(|l| {
                let to = match dir {
                    Direction::Up => key.index - 1,
                    Direction::Down => key.index + 1,
                    _ => return None,
                };
                (0..l.rows).contains(&to).then(|| Key::new(&key.id, to))
            });
            row.or_else(|| registry.next(&key, dir, self.scope(), &self.row_of()))
        };
        if let Some(next) = next {
            self.take(next);
            self.reveal(ui);
        }
        Did::Nothing
    }

    /// Tab (or Shift+Tab, `back`): the next control in reading order, around from the last to the first.
    pub fn tab(&mut self, ui: &AppWindow, back: bool) {
        self.tab_in(ui, back, &[]);
    }

    /// Tab as [`Nav::tab`], through the controls named in `zones` first, in that order (a list by its name),
    /// then the rest in reading order: a screen whose parts read in an order their places don't (Play's zones).
    pub fn tab_in(&mut self, ui: &AppWindow, back: bool, zones: &[&str]) {
        let order = in_zones(reading_order(self.registry.borrow().candidates(self.scope(), None, &self.row_of())), zones);
        if order.is_empty() {
            return;
        }
        // (A list is one stop: its row is found by the list's name.)
        let at = self.focus.as_ref().and_then(|k| order.iter().position(|o| o == k || (o.id == k.id && self.registry.borrow().lists.contains_key(&k.id))));
        let next = match (at, back) {
            (Some(at), false) => (at + 1) % order.len(),
            (Some(at), true) => (at + order.len() - 1) % order.len(),
            (None, false) => 0,
            (None, true) => order.len() - 1,
        };
        self.take(order[next].clone());
        self.reveal(ui);
    }

    /// Put the focus on the first of `ids` that can take it now (a list on its row), scrolled into view. False if
    /// none can.
    pub fn focus_first(&mut self, ui: &AppWindow, ids: &[&str]) -> bool {
        let to = {
            let row_of = self.row_of();
            ids.iter().map(|id| Key::new(id, row_of(id))).find(|key| self.registry.borrow().holds(key))
        };
        let Some(to) = to else { return false };
        self.leave_edit();
        self.take(to);
        self.reveal(ui);
        true
    }

    /// A list's page up or down, its first or last row (`to`: the row's change, clamped to the list).
    pub fn rows(&mut self, ui: &AppWindow, by: i32, page: bool) {
        let Some(key) = self.focus.clone() else { return };
        let to = {
            let registry = self.registry.borrow();
            let Some(l) = registry.lists.get(&key.id) else { return };
            let by = if page { by * ((l.view.h / l.pitch.max(1.0)).floor() as i32 - 1).max(1) } else { by };
            Key::new(&key.id, key.index.saturating_add(by).clamp(0, (l.rows - 1).max(0)))
        };
        self.take(to);
        self.reveal(ui);
    }

    /// Enter: the focused control is activated, a setting taken or let go, a field's edit begun.
    pub fn confirm(&mut self, ui: &AppWindow) -> Did {
        let Some(key) = self.focus.clone() else { return Did::Nothing };
        match self.registry.borrow().kind(&key) {
            Some(Kind::Field) => {}
            Some(Kind::Stepper) => {
                self.grab = if self.grabbed() { None } else { Some(key) };
                return Did::Nothing;
            }
            Some(_) => return Did::Activate(key),
            None => return Did::Nothing,
        }
        self.begin_edit(ui, "");
        Did::Nothing
    }

    /// A click or a tap on a control: as Enter on it.
    pub fn clicked(&mut self, ui: &AppWindow, id: &str, index: i32) -> Did {
        let key = Key::new(id, index);
        if self.registry.borrow().kind(&key) == Some(Kind::Field) {
            if self.focus.as_ref() != Some(&key) {
                self.take(key);
            }
            if self.edit.is_none() {
                self.begin_edit(ui, "");
            }
            return Did::Nothing;
        }
        Did::Activate(key)
    }

    /// The pointer went down on a control: the focus goes there (an edit elsewhere is confirmed first:
    /// [`Did::Edit`]).
    pub fn pointed(&mut self, id: &str, index: i32) -> Did {
        let key = Key::new(id, index);
        if !key.id.starts_with(self.scope()) || !self.registry.borrow().holds(&key) {
            return Did::Nothing;
        }
        let did = if self.edit.as_ref().is_some_and(|e| e.key != key) { self.end_edit(true) } else { Did::Nothing };
        self.take(key);
        did
    }

    // ---- A field's edit ---------------------------------------------------------------------------------

    /// Whether a field is being edited.
    pub fn editing(&self) -> bool {
        self.edit.is_some()
    }

    /// Whether the focus is on a field (a typed letter starts editing it).
    pub fn on_field(&self) -> bool {
        self.focus.as_ref().is_some_and(|k| self.registry.borrow().kind(k) == Some(Kind::Field))
    }

    /// Start editing the focused field: with `seed` in place of its text (the letter typed), or with its own
    /// text, all selected (`seed` empty).
    pub fn begin_edit(&mut self, ui: &AppWindow, seed: &str) {
        let Some(key) = self.focus.clone() else { return };
        self.edit = Some(Edit { key, before: String::new(), text: seed.to_string() });
        self.edit_seq += 1;
        let nav = ui.global::<nettai_ui::Nav>();
        // (The field is told it is the one focused now, not at the frame's settling: a click's focus and its edit
        // can come in one frame, and the field begins its edit only if it is focused as the edit begins.)
        nav.set_id(self.edit.as_ref().expect("set above").key.id.as_str().into());
        nav.set_index(self.edit.as_ref().expect("set above").key.index);
        nav.set_edit_seed(seed.into());
        nav.set_editing(true);
        nav.set_edit_seq(self.edit_seq);
    }

    /// The field says the text it had as its edit began. With a letter typed to begin it, that letter is its
    /// first change.
    pub fn edit_began(&mut self, before: &str) -> Did {
        let Some(e) = &mut self.edit else { return Did::Nothing };
        e.before = before.to_string();
        if e.text.is_empty() {
            e.text = before.to_string();
            return Did::Nothing;
        }
        Did::Edit { key: e.key.clone(), text: e.text.clone(), done: false }
    }

    /// The field's text changed.
    pub fn edited(&mut self, text: &str) -> Did {
        let Some(e) = &mut self.edit else { return Did::Nothing };
        e.text = text.to_string();
        Did::Edit { key: e.key.clone(), text: e.text.clone(), done: false }
    }

    /// The field being edited: where it is, its text as it stands, and whether that is the text the field had (all
    /// of it selected as the edit begins) rather than a letter typed to begin it.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub fn edited_field(&self) -> Option<(Rect, String, bool)> {
        let e = self.edit.as_ref()?;
        let rect = self.registry.borrow().rect(&e.key)?;
        Some((rect, e.text.clone(), e.text == e.before))
    }

    /// The edit ends: its text confirmed (`keep`), or the text it had restored.
    pub fn end_edit(&mut self, keep: bool) -> Did {
        let Some(e) = self.edit.take() else { return Did::Nothing };
        Did::Edit { text: if keep { e.text } else { e.before }, key: e.key, done: true }
    }

    /// (A view or a scope changes under an edit: it is dropped as it stands.)
    fn leave_edit(&mut self) {
        self.edit = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn at(id: &str, x: f32, y: f32, w: f32, h: f32) -> (Key, Rect) {
        (Key::new(id, 0), rect(x, y, w, h))
    }

    /// The main menu, as it is laid out at 1280 by 800: the four words in a column, the game at the header's
    /// right.
    fn menu() -> Vec<(Key, Rect)> {
        vec![
            at("game", 1092.0, 14.0, 156.0, 36.0),
            at("play", 32.0, 205.5, 416.0, 96.0),
            at("replays", 32.0, 309.5, 416.0, 96.0),
            at("settings", 32.0, 413.5, 416.0, 96.0),
            at("exit", 32.0, 517.5, 416.0, 96.0),
        ]
    }

    fn from(all: &[(Key, Rect)], id: &str, dir: Direction) -> Option<String> {
        let me = all.iter().find(|(k, _)| k.id == id).unwrap().1;
        let others: Vec<_> = all.iter().filter(|(k, _)| k.id != id).cloned().collect();
        neighbor(me, dir, &others).map(|k| k.id)
    }

    /// The arrows go to the neighbor in their direction: down the column and up it; nothing lies left of the
    /// column or below the last row.
    #[test]
    fn arrows_find_the_neighbor_in_their_direction() {
        let m = menu();
        assert_eq!(from(&m, "play", Direction::Down).as_deref(), Some("replays"));
        assert_eq!(from(&m, "settings", Direction::Down).as_deref(), Some("exit"));
        assert_eq!(from(&m, "exit", Direction::Up).as_deref(), Some("settings"));
        assert_eq!(from(&m, "play", Direction::Left), None);
        assert_eq!(from(&m, "exit", Direction::Down), None);
        // (Nothing is right above PLAY: the nearest thing upward is the game, across the header.)
        assert_eq!(from(&m, "play", Direction::Up).as_deref(), Some("game"));
        assert_eq!(from(&m, "game", Direction::Down).as_deref(), Some("play"));
    }

    /// One that is straight ahead is taken before a nearer one that is off to the side.
    #[test]
    fn the_beam_comes_first() {
        let all = vec![at("me", 100.0, 100.0, 100.0, 40.0), at("ahead", 150.0, 400.0, 100.0, 40.0), at("aside", 260.0, 150.0, 100.0, 40.0)];
        assert_eq!(from(&all, "me", Direction::Down).as_deref(), Some("ahead"));
    }

    /// Tab goes by rows from the top, each from its left, whatever order the controls were made in.
    #[test]
    fn tab_follows_reading_order() {
        let mut m = menu();
        m.reverse();
        let order: Vec<String> = reading_order(m).into_iter().map(|k| k.id).collect();
        assert_eq!(order, ["game", "play", "replays", "settings", "exit"]);
    }

    /// Play's builds, as they are laid out at 1280 by 800: Tab goes zone by zone (the builds, the chosen build,
    /// the ways to play it online) where reading order would go from + NEW BUILD to EDIT, at the same height, before
    /// the list; the arrows go by place, Right from a row of the list being the screen's own (`Screen::route`).
    #[test]
    fn play_tabs_by_its_zones() {
        let play = vec![
            at("new", 32.0, 88.0, 140.0, 40.0),
            at("edit", 1064.0, 112.0, 120.0, 40.0),
            at("more", 1192.0, 112.0, 40.0, 40.0),
            at("builds", 32.0, 140.0, 400.0, 496.0),
            at("invite", 456.0, 652.0, 120.0, 40.0),
            at("code", 600.0, 652.0, 208.0, 40.0),
            at("join", 816.0, 652.0, 120.0, 40.0),
            at("direct", 1090.0, 652.0, 158.0, 40.0),
        ];
        let reading: Vec<String> = reading_order(play.clone()).into_iter().map(|k| k.id).collect();
        assert_eq!(reading[..4], ["new", "edit", "more", "builds"]);
        let zones = ["new", "builds", "edit", "more", "invite", "code", "join", "direct"];
        let order: Vec<String> = in_zones(reading_order(play.clone()), &zones).into_iter().map(|k| k.id).collect();
        assert_eq!(order, zones);
        assert_eq!(from(&play, "new", Direction::Down).as_deref(), Some("builds"));
        assert_eq!(from(&play, "edit", Direction::Right).as_deref(), Some("more"));
        assert_eq!(from(&play, "builds", Direction::Down).as_deref(), Some("invite"));
        assert_eq!(from(&play, "invite", Direction::Right).as_deref(), Some("code"));
        assert_eq!(from(&play, "code", Direction::Right).as_deref(), Some("join"));
        assert_eq!(from(&play, "join", Direction::Right).as_deref(), Some("direct"));
        assert_eq!(from(&play, "direct", Direction::Left).as_deref(), Some("join"));
        assert_eq!(from(&play, "invite", Direction::Up).as_deref(), Some("builds"));
    }

    /// A control leaves the registry when the token it was given is dropped, and one made again before the old
    /// one went stays.
    #[test]
    fn a_dropped_token_takes_its_control_out() {
        let registry = Rc::new(RefCell::new(Registry::default()));
        let first = Registry::register(&registry, "ready", 0, NavKind::Button);
        registry.borrow_mut().place("ready", 0, rect(0.0, 0.0, 120.0, 40.0), true);
        assert!(registry.borrow().holds(&Key::new("ready", 0)));
        let second = Registry::register(&registry, "ready", 0, NavKind::Button);
        drop(first);
        assert!(registry.borrow().holds(&Key::new("ready", 0)), "made again: it stays");
        drop(second);
        assert!(!registry.borrow().holds(&Key::new("ready", 0)));
    }

    /// A list is entered at its remembered row, Up and Down are its rows' until its ends, and a row's place is
    /// its place in the list's view as scrolled.
    #[test]
    fn a_list_is_one_stop_with_rows() {
        let registry = Rc::new(RefCell::new(Registry::default()));
        let _list = Registry::register(&registry, "builds", 0, NavKind::List);
        let _button = Registry::register(&registry, "new", 0, NavKind::Button);
        {
            let mut r = registry.borrow_mut();
            r.place("new", 0, rect(32.0, 100.0, 120.0, 40.0), true);
            r.page("builds", rect(32.0, 160.0, 400.0, 200.0), 538.0, 54.0);
            r.list("builds", 10, 54.0, 52.0, 54.0, 388.0);
        }
        let r = registry.borrow();
        let candidates = r.candidates("", Some(&Key::new("new", 0)), &|_| 3);
        assert_eq!(candidates, vec![(Key::new("builds", 3), rect(32.0, 160.0, 400.0, 200.0))]);
        // Row 3 of a list scrolled by one row: two rows down from the view's top.
        assert_eq!(r.rect(&Key::new("builds", 3)), Some(rect(32.0, 160.0 + 2.0 * 54.0, 388.0, 52.0)));
        assert!(r.holds(&Key::new("builds", 9)) && !r.holds(&Key::new("builds", 10)));
    }

    /// A page's controls scrolled out of its view are passed by from outside it, and reached from inside it.
    #[test]
    fn a_page_hides_what_is_scrolled_out_from_outside() {
        let registry = Rc::new(RefCell::new(Registry::default()));
        let _tokens = [
            Registry::register(&registry, "set.", 0, NavKind::Page),
            Registry::register(&registry, "set.name", 0, NavKind::Field),
            Registry::register(&registry, "set.about", 0, NavKind::Button),
            Registry::register(&registry, "group", 0, NavKind::Button),
        ];
        {
            let mut r = registry.borrow_mut();
            r.page("set.", rect(292.0, 88.0, 900.0, 600.0), 900.0, 0.0);
            r.place("set.name", 0, rect(516.0, 120.0, 320.0, 40.0), true);
            r.place("set.about", 0, rect(516.0, 900.0, 160.0, 40.0), true);
            r.place("group", 0, rect(32.0, 88.0, 220.0, 44.0), true);
        }
        let r = registry.borrow();
        let ids = |from: &str| -> Vec<String> { r.candidates("", Some(&Key::new(from, 0)), &|_| 0).into_iter().map(|(k, _)| k.id).collect() };
        assert_eq!(ids("group"), ["set.name"]);
        assert_eq!(ids("set.name"), ["group", "set.about"]);
    }

    /// Down in a page goes to its next control, scrolled out of view under the page, before the button under
    /// the page that is nearer on screen; past the page's last control, to the button.
    #[test]
    fn a_page_is_gone_through_before_what_is_under_it() {
        let registry = Rc::new(RefCell::new(Registry::default()));
        let _tokens = [
            Registry::register(&registry, "menu.rows", 0, NavKind::Page),
            Registry::register(&registry, "menu.rows.a", 0, NavKind::Button),
            Registry::register(&registry, "menu.rows.b", 0, NavKind::Button),
            Registry::register(&registry, "menu.resume", 0, NavKind::Button),
        ];
        {
            let mut r = registry.borrow_mut();
            r.page("menu.rows", rect(40.0, 100.0, 400.0, 200.0), 290.0, 0.0);
            r.place("menu.rows.a", 0, rect(40.0, 250.0, 400.0, 44.0), true);
            r.place("menu.rows.b", 0, rect(40.0, 340.0, 400.0, 44.0), true);
            r.place("menu.resume", 0, rect(40.0, 310.0, 200.0, 40.0), true);
        }
        let r = registry.borrow();
        let next = |from: &str| r.next(&Key::new(from, 0), Direction::Down, "", &|_| 0).map(|k| k.id);
        assert_eq!(next("menu.rows.a").as_deref(), Some("menu.rows.b"));
        assert_eq!(next("menu.rows.b").as_deref(), None, "nothing under the page's last control here");
        assert_eq!(r.next(&Key::new("menu.rows.b", 0), Direction::Up, "", &|_| 0).map(|k| k.id).as_deref(), Some("menu.rows.a"));
    }

    /// A scroll that leaves the focus where it is moves the page or the list the focus is in, where that can
    /// scroll; else the one under the scope open with the most to scroll; else nothing.
    #[test]
    fn a_scroll_moves_what_the_focus_is_in_else_the_longest() {
        let registry = Rc::new(RefCell::new(Registry::default()));
        let _tokens = [
            Registry::register(&registry, "folder", 0, NavKind::List),
            Registry::register(&registry, "library", 0, NavKind::List),
            Registry::register(&registry, "short", 0, NavKind::List),
            Registry::register(&registry, "set.", 0, NavKind::Page),
            Registry::register(&registry, "set.name", 0, NavKind::Field),
            Registry::register(&registry, "dialog.list", 0, NavKind::List),
            Registry::register(&registry, "tab", 0, NavKind::Button),
        ];
        {
            let mut r = registry.borrow_mut();
            r.page("folder", rect(0.0, 0.0, 300.0, 400.0), 600.0, 20.0);
            r.page("library", rect(320.0, 0.0, 300.0, 400.0), 1000.0, 0.0);
            r.page("short", rect(640.0, 0.0, 300.0, 400.0), 400.0, 0.0);
            r.page("set.", rect(0.0, 420.0, 900.0, 200.0), 500.0, 0.0);
            r.place("set.name", 0, rect(20.0, 440.0, 320.0, 40.0), true);
            r.page("dialog.list", rect(100.0, 100.0, 300.0, 200.0), 260.0, 0.0);
            r.place("tab", 0, rect(0.0, 640.0, 100.0, 40.0), true);
        }
        let r = registry.borrow();
        let target = |focus: &str, scope: &str| r.scroll_target(Some(&Key::new(focus, 0)), scope).map(|(id, _, at, span)| (id.to_string(), at, span));
        assert_eq!(target("folder", ""), Some(("folder".into(), 20.0, 200.0)));
        assert_eq!(target("set.name", ""), Some(("set.".into(), 0.0, 300.0)), "a page's control: its page");
        assert_eq!(target("short", ""), Some(("library".into(), 0.0, 600.0)), "a list that fits: the longest");
        assert_eq!(target("tab", ""), Some(("library".into(), 0.0, 600.0)));
        assert_eq!(target("tab", "dialog."), Some(("dialog.list".into(), 0.0, 60.0)), "only under the scope open");
        assert_eq!(target("tab", "menu."), None);
    }
}
