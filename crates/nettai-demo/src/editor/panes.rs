//! The panes the game's rules declare of a side's setup
//! (`nettai_match::panes`: their `panes`), drawn by their views' kinds and
//! nothing else: a flag, a number, a pick of one, a list, a grid. The
//! editor names no game's feature here: a game that declares another pane
//! gets it without a line of this crate changing.
//!
//! What a pane's functions say (whether a side's shows it, what a list or a
//! pick offers, a list's summary, a grid's board and its pieces' shapes,
//! colors and badges) is asked of a round of the match as the editor starts
//! it to check the match ([`ask`], `Editor::refresh`), each edit; where the
//! round doesn't start, the pane says why. Whether a side's setup is right
//! is the rules' `validate`'s, in the problems below the panes.
//!
//! A grid is edited with the mouse as Tango's NaviCust editor is: a piece is
//! *held* and follows the cursor as a ghost, lit where it would land and red
//! where it doesn't fit. A color swatch in the list picks one up; pressing a
//! placed piece picks it up off the grid, by the cell pressed. A click on the
//! grid (or letting go of a drag over it) puts it down where it fits; letting
//! go of a drag off the grid, right-clicking or Delete takes it off; Escape
//! puts a piece picked off the grid back. The wheel or R turns the held
//! piece. Right-clicking a placed piece turns it where it is. A piece fits
//! where its cells are on the board (`o`) or its margin (`f`), not all on the
//! margin, and over no other.

use std::collections::HashMap;

use crate::editor::app::{Choice, Editor, Msg};
use crate::editor::view::{DIM, RED, SIDES, heading};
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{
    Column, Row, button, canvas as canvas_widget, checkbox, column, container, image, mouse_area, pick_list, row, scrollable, space, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use nettai_battle::Battle;
use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Data, FieldType, Registry, Value};
use nettai_match::Side;
use nettai_match::facts::{Stated, fact_of};
use nettai_match::panes::{FieldPane, Grid, List, Pane, Pick, View};

/// A shape: rows of cells, `true` a cell it covers, its center the middle
/// cell.
pub type Shape = Vec<Vec<bool>>;

/// What the rules' functions answered for a side, asked of the round the
/// editor started (`Editor::refresh`).
#[derive(Clone, Debug, Default)]
pub struct Answers {
    /// Why nothing was asked: the round doesn't start.
    pub unasked: Option<String>,
    /// Whether each pane is shown (by its place among the panes).
    pub shown: Vec<bool>,
    /// What a view offers, by its path: definitions by handle.
    pub offered: HashMap<String, Vec<u16>>,
    /// A list's summary, by its path.
    pub summary: HashMap<String, String>,
    pub grids: HashMap<String, GridAnswers>,
    /// What a function said that isn't an answer.
    pub errors: Vec<String>,
}

/// What a grid's functions answered.
#[derive(Clone, Debug, Default)]
pub struct GridAnswers {
    /// The board's rows (`o`, `f`, `.`); none: no board.
    pub board: Option<Vec<Vec<u8>>>,
    /// A piece's shape by its toggles, before turning.
    pub shapes: HashMap<(u16, Vec<bool>), Shape>,
    pub colors: HashMap<u16, Vec<String>>,
    pub badges: HashMap<u16, String>,
}

/// The definition `key` of `registry` names, by its handle.
fn handle_of_key(c: &Content, registry: Registry, key: &str) -> Option<u16> {
    let d = &c.defs;
    match registry {
        Registry::Chip => d.chip_by_key(key).map(|h| h.0),
        Registry::Navi => d.navi_by_key(key).map(|h| h.0),
        Registry::Form => d.form_by_key(key).map(|h| h.0),
        Registry::Stage => d.stage_by_key(key).map(|h| h.0),
        Registry::Weapon => d.weapon_by_key(key).map(|h| h.0),
        Registry::Record => d.record(key).map(|h| h.0),
        Registry::Entry => d.entry_by_key(key).map(|h| h.0),
        _ => None,
    }
}

/// The definitions a list of answers names, by handle.
fn handles(c: &Content, d: &Data) -> Vec<u16> {
    match d {
        Data::List(items) => items.iter().filter_map(|x| if let Data::Ref(r, k) = x { handle_of_key(c, *r, k) } else { None }).collect(),
        _ => Vec::new(),
    }
}

/// Rows of a shape or a board, from an answer of strings.
fn rows(d: &Data) -> Option<Vec<Vec<u8>>> {
    match d {
        Data::List(items) => items.iter().map(|r| r.str().map(|s| s.bytes().collect())).collect(),
        _ => None,
    }
}

/// The type of a setup field's element (a list's or an array's), else its own.
fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// A record field's type, of a list of records.
fn record_field<'t>(ty: &'t FieldType, name: &str) -> Option<&'t FieldType> {
    let FieldType::Record(fields) = element(ty) else { return None };
    fields.index_of(name).map(|i| &fields.field(i).ty)
}

/// Ask the rules' panes' functions of round `b`, for side `side`.
pub fn ask(c: &Content, panes: &[Pane], b: &mut Battle, side: usize) -> Answers {
    let mut a = Answers::default();
    let s = side as u8;
    let side_arg = [Value::Int(side as i64)];
    let mut call = |path: &str, args: &[Value], a: &mut Answers| -> Option<Data> {
        match b.call_pane(path, s, args)? {
            Ok(d) => Some(d),
            Err(e) => {
                a.errors.push(e);
                None
            }
        }
    };
    let setup = c.defs.rules().map(|r| c.defs.schema(r.setup));
    for p in panes {
        let shown = match &p.shown {
            Some(f) => call(f, &side_arg, &mut a).is_some_and(|d| d == Data::Bool(true)),
            None => true,
        };
        a.shown.push(shown);
        for f in &p.fields {
            let ty = setup.and_then(|s| s.index_of(&f.field).map(|i| s.field(i).ty.clone()));
            match &f.view {
                View::Pick(Pick { offered: Some(o), .. }) | View::List(List { offered: Some(o), .. }) => {
                    if let Some(d) = call(o, &side_arg, &mut a) {
                        a.offered.insert(f.path.clone(), handles(c, &d));
                    }
                }
                _ => {}
            }
            if let View::List(List { summary: Some(sm), .. }) = &f.view
                && let Some(d) = call(sm, &side_arg, &mut a)
            {
                a.summary.insert(f.path.clone(), d.str().map(str::to_string).unwrap_or_default());
            }
            if let (View::Grid(g), Some(ty)) = (&f.view, ty) {
                let mut ga = GridAnswers { board: call(&g.board, &side_arg, &mut a).as_ref().and_then(rows), ..GridAnswers::default() };
                let Some(FieldType::Ref(registry, of)) = record_field(&ty, &g.piece) else { continue };
                let pieces = nettai_match::ids::all_of(c, *registry, of.as_deref());
                let combos: Vec<Vec<bool>> = (0..1u32 << g.toggles.len()).map(|m| (0..g.toggles.len()).map(|k| m & (1 << k) != 0).collect()).collect();
                for h in pieces {
                    let piece = Value::Def(*registry, h);
                    for t in &combos {
                        let mut args = vec![piece];
                        args.extend(t.iter().map(|&b| Value::Bool(b)));
                        if let Some(shape) = call(&g.shape, &args, &mut a).as_ref().and_then(rows) {
                            ga.shapes.insert((h, t.clone()), shape.iter().map(|r| r.iter().map(|&b| b == b'#').collect()).collect());
                        }
                    }
                    if let Some(f) = &g.colors
                        && let Some(Data::List(names)) = call(f, &[piece], &mut a)
                    {
                        ga.colors.insert(h, names.iter().filter_map(|n| n.str().map(str::to_string)).collect());
                    }
                    if let Some(f) = &g.badge
                        && let Some(Data::Str(badge)) = call(f, &[piece], &mut a)
                    {
                        ga.badges.insert(h, badge);
                    }
                }
                a.grids.insert(f.path.clone(), ga);
            }
        }
    }
    a
}

/// A pane's or a label's words: the locales' `[setup]` (the editor's
/// language, else the content's own), else the key said as it is.
pub fn said(e: &Editor, key: &str) -> String {
    e.names
        .other
        .as_ref()
        .and_then(|s| s.setup.get(key))
        .or_else(|| e.content.strings.setup.get(key))
        .cloned()
        .unwrap_or_else(|| crate::editor::facts::title(key))
}

/// The view at `path` among the panes, and the type of its setup field.
fn view_at<'a>(e: &'a Editor, path: &str) -> Option<(&'a FieldPane, FieldType)> {
    let f = e.panes.iter().flat_map(|p| &p.fields).find(|f| f.path == path)?;
    let setup = e.content.defs.schema(e.content.defs.rules()?.setup);
    let ty = setup.field(setup.index_of(&f.field)?).ty.clone();
    Some((f, ty))
}

// ---- Edits ----------------------------------------------------------------------------

/// An edit of a fact through its view.
#[derive(Clone, Debug)]
pub enum Edit {
    Flag(bool),
    /// A number, as typed.
    Number(String),
    /// One picked: an enum's variant's place, a definition's handle, a
    /// number, an entry's place in another list; none.
    Pick(Option<i64>),
    /// A definition into a list, at its end.
    Add(u16),
    Remove(usize),
    /// A row up (true) or down.
    Move(usize, bool),
    /// A definition into a checklist, or out of it.
    Check(u16, bool),
    /// A list's row's column, by its field.
    Cell(usize, String, Box<Edit>),
    Grid(GridEdit),
    /// The search of a list's or a grid's pieces.
    Search(String),
}

#[derive(Clone, Debug)]
pub enum GridEdit {
    /// A piece picked up from the list, in one of its colors.
    Hold(u16, u8),
    /// The placed piece at this place in the list picked up by this cell.
    PickUp(usize, i32, i32),
    /// The held piece put down with its center on this cell.
    Place(i32, i32),
    PutBack,
    RotateAt(usize),
    /// The held piece, else the selected one: turned, a toggle set, its
    /// color, moved a cell, taken off.
    Rotate,
    Toggle(usize, bool),
    Color(u8),
    Nudge(i32, i32),
    Remove,
}

/// What a grid keeps of its own: the piece held, the one last placed, the
/// list's search.
#[derive(Clone, Debug, Default)]
pub struct GridState {
    pub held: Option<Held>,
    pub selected: Option<usize>,
    pub search: String,
}

/// A piece picked up: which, its color's place in its colors, its turns,
/// its toggles, the cell of it the cursor holds (from its center), and the
/// record it was on the grid if it was picked off it (its place).
#[derive(Clone, Debug)]
pub struct Held {
    pub piece: u16,
    pub color: u8,
    pub rotation: u8,
    pub toggles: Vec<bool>,
    pub grab: (i32, i32),
    pub origin: Option<(usize, Stated)>,
}

/// Apply `edit` to side `s`'s fact through the view at `path`; whether the
/// match changed.
pub fn apply(e: &mut Editor, s: usize, path: &str, edit: Edit) -> bool {
    let Some((f, ty)) = view_at(e, path).map(|(f, ty)| (f.clone(), ty)) else { return false };
    let content = e.content.clone();
    let c = &*content;
    let side = &mut e.m.sides[s];
    let current = side.facts.get(c, &f.field);
    let write = |side: &mut Side, value: &Stated| side.set_fact(c, &f.field, &values(value)).is_ok();
    match (edit, &f.view) {
        (Edit::Search(t), View::Grid(_)) => {
            e.grids[s].entry(path.to_string()).or_default().search = t;
            false
        }
        (Edit::Search(t), _) => {
            e.search = t;
            false
        }
        (Edit::Flag(on), _) => side.set_fact(c, &f.field, &[Fact::Value(Value::Bool(on))]).is_ok(),
        (Edit::Number(t), View::Number { time }) => {
            let key = (s, path.to_string());
            let parsed = number(&t, *time, &ty);
            e.pane_typed.insert(key.clone(), t);
            match parsed {
                Some(v) if side.set_fact(c, &f.field, &[Fact::Value(v)]).is_ok() => {
                    e.pane_typed.remove(&key);
                    true
                }
                _ => false,
            }
        }
        (Edit::Pick(v), View::Pick(_)) => side.set_fact(c, &f.field, &[pick_fact(&ty, v)]).is_ok(),
        (edit, View::List(l)) => {
            let Some(Stated::List(mut items)) = current else { return false };
            let room = match &ty {
                FieldType::List(_, n) => *n as usize,
                FieldType::Array(_, n) => *n as usize,
                _ => 0,
            };
            let registry = match element(&ty) {
                FieldType::Ref(r, _) => Some(*r),
                _ => None,
            };
            match edit {
                Edit::Add(h) if !l.fixed && items.len() < room => items.push(Stated::Def(registry.unwrap_or(Registry::Chip), Some(h))),
                Edit::Remove(i) if !l.fixed && i < items.len() => {
                    items.remove(i);
                }
                Edit::Move(i, up) if !l.fixed => {
                    let j = if up { i.checked_sub(1) } else { (i + 1 < items.len()).then_some(i + 1) };
                    let Some(j) = j else { return false };
                    items.swap(i, j);
                }
                Edit::Check(h, on) => {
                    let Some(r) = registry else { return false };
                    items.retain(|x| *x != Stated::Def(r, Some(h)));
                    if on && items.len() < room {
                        items.push(Stated::Def(r, Some(h)));
                    }
                    // (In the order the list offers them.)
                    let order = e.answers[s].offered.get(path).cloned().unwrap_or_default();
                    items.sort_by_key(|x| x.defs().first().and_then(|h| order.iter().position(|o| o == h)));
                }
                Edit::Cell(i, column, cell) => {
                    let Some(Stated::Record(fields)) = items.get_mut(i) else { return false };
                    let Some(cty) = record_field(&ty, &column).cloned() else { return false };
                    let view = l.columns.iter().find(|(n, _)| *n == column).map(|(_, v)| v.clone());
                    let new = match (*cell, view) {
                        (Edit::Number(t), Some(View::Number { time })) => {
                            let key = (s, format!("{path}.{i}.{column}"));
                            let parsed = number(&t, time, &cty);
                            e.pane_typed.insert(key.clone(), t);
                            let Some(v) = parsed else { return false };
                            e.pane_typed.remove(&key);
                            stated_of(&cty, v)
                        }
                        (Edit::Number(t), _) => {
                            let Some(v) = number(&t, false, &cty) else { return false };
                            stated_of(&cty, v)
                        }
                        (Edit::Flag(b), _) => Stated::Flag(b),
                        (Edit::Pick(v), _) => match (&cty, v) {
                            (FieldType::Enum(names), Some(i)) => Stated::Variant(names.get(i as usize).cloned()),
                            (FieldType::Ref(r, _), v) => Stated::Def(*r, v.map(|h| h as u16)),
                            (_, Some(n)) => Stated::Number(n),
                            (_, None) => Stated::Optional(None),
                        },
                        _ => return false,
                    };
                    match fields.iter_mut().find(|(n, _)| *n == column) {
                        Some((_, v)) => *v = new,
                        None => return false,
                    }
                }
                _ => return false,
            }
            write(side, &Stated::List(items))
        }
        (Edit::Grid(g), View::Grid(grid)) => {
            let answers = e.answers[s].grids.get(path).cloned().unwrap_or_default();
            let state = e.grids[s].entry(path.to_string()).or_default();
            grid_edit(c, side, &f.field, &ty, grid, &answers, state, g)
        }
        _ => false,
    }
}

/// A number as typed into a field of type `ty` (`time`: mm:ss.cc, its
/// frames); none: it reads as none of its.
fn number(t: &str, time: bool, ty: &FieldType) -> Option<Value> {
    let t = t.trim();
    if t.is_empty() && matches!(ty, FieldType::OptionalU8) {
        return Some(Value::Nil);
    }
    if time {
        return nettai_match::sp_times::parse(t).ok().map(|f| Value::Int(f as i64));
    }
    let n: i64 = t.parse().ok()?;
    match nettai_match::facts::range(ty) {
        Some((lo, hi, _)) if !(lo..=hi).contains(&n) => None,
        _ => Some(Value::Int(n)),
    }
}

/// A value as a record's field reads it.
fn stated_of(ty: &FieldType, v: Value) -> Stated {
    match (ty, v) {
        (FieldType::OptionalU8, Value::Nil) => Stated::Optional(None),
        (FieldType::OptionalU8, Value::Int(n)) => Stated::Optional(Some(n)),
        (_, Value::Int(n)) => Stated::Number(n),
        (_, Value::Bool(b)) => Stated::Flag(b),
        _ => Stated::Other,
    }
}

/// A pick as the facts' writer takes it.
fn pick_fact(ty: &FieldType, v: Option<i64>) -> Fact<'_> {
    match (ty, v) {
        (FieldType::Enum(names), Some(i)) => names.get(i as usize).map_or(Fact::Value(Value::Nil), |n| Fact::Name(n)),
        (FieldType::Ref(r, _), Some(h)) => Fact::Value(Value::Def(*r, h as u16)),
        (_, Some(n)) => Fact::Value(Value::Int(n)),
        (_, None) => Fact::Value(Value::Nil),
    }
}

/// A read value's facts for the writer: a list's entries, else the value.
fn values(v: &Stated) -> Vec<Fact<'_>> {
    match fact_of(v) {
        Fact::List(items) => items,
        f => vec![f],
    }
}

// ---- The grid's edits -----------------------------------------------------------------

/// A grid's piece placed: which, its center, its turns, its color (by
/// name), its toggles; and its record as read, for the fields the view
/// doesn't name.
#[derive(Clone, Debug)]
struct Placed {
    piece: u16,
    x: i32,
    y: i32,
    rotation: u8,
    color: Option<String>,
    toggles: Vec<bool>,
    record: Stated,
}

/// The pieces a grid's list holds.
fn placed(c: &Content, side: &Side, field: &str, grid: &Grid) -> Vec<Placed> {
    let Some(Stated::List(items)) = side.facts.get(c, field) else { return Vec::new() };
    items
        .iter()
        .filter_map(|item| {
            let Stated::Record(fields) = item else { return None };
            let get = |n: &str| fields.iter().find(|(k, _)| k == n).map(|(_, v)| v);
            let int = |n: &str| match get(n) {
                Some(Stated::Number(v)) => *v,
                _ => 0,
            };
            let Some(Stated::Def(_, Some(piece))) = get(&grid.piece) else { return None };
            Some(Placed {
                piece: *piece,
                x: int(&grid.x) as i32,
                y: int(&grid.y) as i32,
                rotation: int(&grid.rotation) as u8,
                color: grid.color.as_deref().and_then(get).and_then(|v| if let Stated::Variant(n) = v { n.clone() } else { None }),
                toggles: grid.toggles.iter().map(|t| matches!(get(t), Some(Stated::Flag(true)))).collect(),
                record: item.clone(),
            })
        })
        .collect()
}

/// A placed piece's record: its read record with the view's fields set.
fn record_of(p: &Placed, grid: &Grid, registry: Registry) -> Stated {
    let mut fields = match &p.record {
        Stated::Record(f) => f.clone(),
        _ => Vec::new(),
    };
    let mut set = |name: &str, v: Stated| match fields.iter_mut().find(|(k, _)| k == name) {
        Some((_, x)) => *x = v,
        None => fields.push((name.to_string(), v)),
    };
    set(&grid.piece, Stated::Def(registry, Some(p.piece)));
    set(&grid.x, Stated::Number(p.x as i64));
    set(&grid.y, Stated::Number(p.y as i64));
    set(&grid.rotation, Stated::Number(p.rotation as i64));
    if let Some(c) = &grid.color {
        set(c, Stated::Variant(p.color.clone()));
    }
    for (t, on) in grid.toggles.iter().zip(&p.toggles) {
        set(t, Stated::Flag(*on));
    }
    Stated::Record(fields)
}

/// `shape` turned a quarter clockwise `rotation` times about its center.
pub fn rotate(shape: &Shape, rotation: u8) -> Shape {
    let n = shape.len();
    if n == 0 {
        return Vec::new();
    }
    let m = n - 1;
    let mut out = vec![vec![false; n]; n];
    for (y, row) in shape.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate().take(n) {
            let (oy, ox) = match rotation & 3 {
                0 => (y, x),
                1 => (x, m - y),
                2 => (m - y, m - x),
                _ => (m - x, y),
            };
            out[oy][ox] = cell;
        }
    }
    out
}

/// The cells `shape` covers with its center on (x, y).
pub fn cells(shape: &Shape, x: i32, y: i32) -> Vec<(i32, i32)> {
    let c = (shape.len() / 2) as i32;
    let mut out = Vec::new();
    for (j, row) in shape.iter().enumerate() {
        for (i, &on) in row.iter().enumerate() {
            if on {
                out.push((x - c + i as i32, y - c + j as i32));
            }
        }
    }
    out
}

/// A piece's shape as placed: its toggles', turned.
fn shape_of(answers: &GridAnswers, piece: u16, toggles: &[bool], rotation: u8) -> Shape {
    answers.shapes.get(&(piece, toggles.to_vec())).map_or_else(Vec::new, |s| rotate(s, rotation))
}

/// Whether `shape` fits with its center on (x, y): its cells on the board
/// or its margin, not all on the margin, and over none of `others`'.
fn fits(answers: &GridAnswers, others: &[Placed], shape: &Shape, x: i32, y: i32) -> bool {
    let Some(board) = &answers.board else { return false };
    let cs = cells(shape, x, y);
    let at = |(cx, cy): (i32, i32)| board.get(cy as usize).and_then(|r| r.get(cx as usize)).copied().filter(|_| cx >= 0 && cy >= 0);
    if cs.is_empty() || cs.iter().any(|&c| !matches!(at(c), Some(b'o' | b'f'))) || cs.iter().all(|&c| at(c) == Some(b'f')) {
        return false;
    }
    let taken: std::collections::HashSet<(i32, i32)> =
        others.iter().flat_map(|p| cells(&shape_of(answers, p.piece, &p.toggles, p.rotation), p.x, p.y)).collect();
    cs.iter().all(|c| !taken.contains(c))
}

#[allow(clippy::too_many_arguments)]
fn grid_edit(c: &Content, side: &mut Side, field: &str, ty: &FieldType, grid: &Grid, answers: &GridAnswers, state: &mut GridState, edit: GridEdit) -> bool {
    let Some(FieldType::Ref(registry, _)) = record_field(ty, &grid.piece).cloned() else { return false };
    let mut parts = placed(c, side, field, grid);
    let mut changed = false;
    let colors = |h: u16| answers.colors.get(&h).cloned().unwrap_or_default();
    let blank = || Stated::Record(Vec::new());
    match edit {
        GridEdit::Hold(piece, color) => {
            if state.held.as_ref().is_some_and(|h| h.origin.is_none() && h.piece == piece && h.color == color) {
                state.held = None;
                return false;
            }
            if let Some((i, record)) = state.held.take().and_then(|h| h.origin) {
                let back = placed_of(c, &record, grid);
                if let Some(b) = back {
                    parts.insert(i.min(parts.len()), b);
                    changed = true;
                }
            }
            state.held = Some(Held { piece, color, rotation: 0, toggles: vec![false; grid.toggles.len()], grab: (0, 0), origin: None });
            if !changed {
                return false;
            }
        }
        GridEdit::PickUp(i, x, y) => {
            if i >= parts.len() {
                return false;
            }
            let p = parts.remove(i);
            let color = p.color.as_ref().and_then(|n| colors(p.piece).iter().position(|x| x == n)).unwrap_or(0) as u8;
            state.held = Some(Held {
                piece: p.piece,
                color,
                rotation: p.rotation,
                toggles: p.toggles.clone(),
                grab: (x - p.x, y - p.y),
                origin: Some((i, record_of(&p, grid, registry))),
            });
            state.selected = None;
        }
        GridEdit::Place(x, y) => {
            let Some(h) = state.held.clone() else { return false };
            let shape = shape_of(answers, h.piece, &h.toggles, h.rotation);
            if !fits(answers, &parts, &shape, x, y) {
                return false;
            }
            let record = h.origin.as_ref().map_or_else(blank, |(_, r)| r.clone());
            let color = colors(h.piece).get(h.color as usize).cloned();
            parts.push(Placed { piece: h.piece, x, y, rotation: h.rotation, color, toggles: h.toggles.clone(), record });
            state.selected = Some(parts.len() - 1);
            state.held = None;
        }
        GridEdit::PutBack => {
            let Some(h) = state.held.take() else { return false };
            let Some((i, record)) = h.origin else { return false };
            let Some(p) = placed_of(c, &record, grid) else { return false };
            parts.insert(i.min(parts.len()), p);
            state.selected = Some(i.min(parts.len() - 1));
        }
        GridEdit::RotateAt(i) => {
            let Some(p) = parts.get_mut(i) else { return false };
            p.rotation = (p.rotation + 1) % 4;
            state.selected = Some(i);
        }
        GridEdit::Rotate => {
            if let Some(h) = state.held.as_mut() {
                // (The held cell turns with the piece.)
                h.rotation = (h.rotation + 1) % 4;
                h.grab = (-h.grab.1, h.grab.0);
                return false;
            }
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.rotation = (p.rotation + 1) % 4;
        }
        GridEdit::Toggle(k, on) => {
            if let Some(h) = state.held.as_mut() {
                if let Some(t) = h.toggles.get_mut(k) {
                    *t = on;
                    // (Its shape is another: held by its center.)
                    h.grab = (0, 0);
                }
                return false;
            }
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            let Some(t) = p.toggles.get_mut(k) else { return false };
            *t = on;
        }
        GridEdit::Color(k) => {
            if let Some(h) = state.held.as_mut() {
                h.color = k;
                return false;
            }
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.color = colors(p.piece).get(k as usize).cloned();
        }
        GridEdit::Nudge(dx, dy) => {
            let n = answers.board.as_ref().map_or(0, |b| b.len() as i32);
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.x = (p.x + dx).clamp(0, (n - 1).max(0));
            p.y = (p.y + dy).clamp(0, (n - 1).max(0));
        }
        GridEdit::Remove => {
            if state.held.take().is_some() {
                return false;
            }
            let Some(i) = state.selected.take().filter(|&i| i < parts.len()) else { return false };
            parts.remove(i);
        }
    }
    let list = Stated::List(parts.iter().map(|p| record_of(p, grid, registry)).collect());
    side.set_fact(c, field, &values(&list)).is_ok()
}

/// A record read back as a placed piece.
fn placed_of(c: &Content, record: &Stated, grid: &Grid) -> Option<Placed> {
    let _ = c;
    let Stated::Record(fields) = record else { return None };
    let get = |n: &str| fields.iter().find(|(k, _)| k == n).map(|(_, v)| v);
    let int = |n: &str| match get(n) {
        Some(Stated::Number(v)) => *v,
        _ => 0,
    };
    let Some(Stated::Def(_, Some(piece))) = get(&grid.piece) else { return None };
    Some(Placed {
        piece: *piece,
        x: int(&grid.x) as i32,
        y: int(&grid.y) as i32,
        rotation: int(&grid.rotation) as u8,
        color: grid.color.as_deref().and_then(get).and_then(|v| if let Stated::Variant(n) = v { n.clone() } else { None }),
        toggles: grid.toggles.iter().map(|t| matches!(get(t), Some(Stated::Flag(true)))).collect(),
        record: record.clone(),
    })
}

// ---- Drawing --------------------------------------------------------------------------

/// A definition's name in the editor's language.
fn name_of(e: &Editor, registry: Registry, h: u16) -> String {
    let c = &e.content;
    match registry {
        Registry::Chip => e.names.chip(c, nettai_content_api::ChipHandle(h)),
        Registry::Form => e.names.form(c, nettai_content_api::FormHandle(h)),
        Registry::Navi => e.names.navi(c, nettai_content_api::NaviHandle(h)),
        Registry::Entry => e.names.entry(c, nettai_content_api::EntryHandle(h)),
        _ => nettai_match::ids::name_of(c, registry, h).map_or_else(|| h.to_string(), str::to_string),
    }
}

/// The definitions a view offers: its function's answer, else every one of
/// the field's registry (or collection) of the match's game, in the
/// library's order where it has one.
fn offered(e: &Editor, s: usize, f: &FieldPane, registry: Registry, of: Option<&str>) -> Vec<u16> {
    if let Some(o) = e.answers[s].offered.get(&f.path) {
        return o.clone();
    }
    let c = &e.content;
    let mut all: Vec<(String, u16)> = nettai_match::ids::all_of(c, registry, of)
        .into_iter()
        .filter(|&h| nettai_match::ids::key_of(c, registry, h).is_some_and(|k| nettai_match::ids::in_game(c, e.m.game(), k)))
        .map(|h| (name_of(e, registry, h), h))
        .collect();
    match (registry, of) {
        (Registry::Entry, Some(collection)) => {
            let mut by: Vec<(String, nettai_content_api::EntryHandle)> = all.iter().map(|(n, h)| (n.clone(), nettai_content_api::EntryHandle(*h))).collect();
            e.order.entries(c, collection, &mut by);
            all = by.into_iter().map(|(n, h)| (n, h.0)).collect();
        }
        (Registry::Chip, _) => {
            let mut by: Vec<(String, nettai_content_api::ChipHandle)> = all.iter().map(|(n, h)| (n.clone(), nettai_content_api::ChipHandle(*h))).collect();
            e.order.chips(c, &mut by);
            all = by.into_iter().map(|(n, h)| (n, h.0)).collect();
        }
        _ => {}
    }
    all.into_iter().map(|(_, h)| h).collect()
}

/// The panes a side's setup is shown in: their places among the rules'
/// panes, those its side shows.
pub fn shown(e: &Editor, s: usize) -> Vec<usize> {
    (0..e.panes.len()).filter(|&i| e.answers[s].shown.get(i).copied().unwrap_or(true)).collect()
}

/// Pane `i` of side `s`.
pub fn view(e: &Editor, s: usize, i: usize) -> Element<'_, Msg> {
    let Some(p) = e.panes.get(i) else { return space().into() };
    let mut col = column![heading(format!("{}: {}", SIDES[s], said(e, &p.title)))].spacing(10);
    if let Some(why) = &e.answers[s].unasked {
        col = col.push(text(format!("The round doesn't start, so the rules say nothing of this side ({why}).")).size(13).color(RED));
    }
    for err in &e.answers[s].errors {
        col = col.push(text(err.clone()).size(13).color(RED));
    }
    let mut grid: Option<Element<Msg>> = None;
    for f in &p.fields {
        let Some((_, ty)) = view_at(e, &f.path) else { continue };
        let label = said(e, f.label.as_deref().unwrap_or(&f.field));
        match &f.view {
            View::Grid(g) => grid = Some(grid_view(e, s, f, g, &ty)),
            View::List(l) => col = col.push(list_view(e, s, f, l, &ty, &label)),
            _ => col = col.push(row![text(label).size(14).width(Length::Fixed(160.0)), control(e, s, f, &f.view, &ty, None)].spacing(8).align_y(Alignment::Center)),
        }
    }
    match grid {
        Some(g) => column![col, g].spacing(10).into(),
        None => scrollable(col).into(),
    }
}

/// A one-value view's control: of the side's fact, or (`cell`: its row and
/// column) of a list's record's field.
fn control<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, view: &View, ty: &FieldType, cell: Option<(usize, String, Stated)>) -> Element<'a, Msg> {
    let c = &e.content;
    let path = f.path.clone();
    let value = match &cell {
        Some((_, _, v)) => Some(v.clone()),
        None => e.side(s).facts.get(c, &f.field),
    };
    let msg = {
        let cell = cell.clone();
        move |edit: Edit| match &cell {
            Some((i, column, _)) => Msg::Pane(s, path.clone(), Edit::Cell(*i, column.clone(), Box::new(edit))),
            None => Msg::Pane(s, path.clone(), edit),
        }
    };
    match view {
        View::Flag => {
            let on = matches!(value, Some(Stated::Flag(true)));
            checkbox(on).on_toggle(move |b| msg(Edit::Flag(b))).into()
        }
        View::Number { time } => {
            let key = match &cell {
                Some((i, column, _)) => format!("{}.{i}.{column}", f.path),
                None => f.path.clone(),
            };
            let shown = e.pane_typed.get(&(s, key)).cloned().unwrap_or_else(|| match &value {
                Some(Stated::Number(n)) if *time => nettai_match::sp_times::format(*n as u16),
                Some(Stated::Number(n)) => n.to_string(),
                Some(Stated::Optional(Some(n))) => n.to_string(),
                _ => String::new(),
            });
            let hint = if *time { "00:00.00" } else { "" };
            text_input(hint, &shown).on_input(move |t| msg(Edit::Number(t))).width(Length::Fixed(100.0)).into()
        }
        View::Pick(p) => {
            let choices = pick_choices(e, s, f, p, ty);
            let now = match &value {
                Some(Stated::Variant(Some(n))) => match ty {
                    FieldType::Enum(names) => names.iter().position(|x| x == n).map(|i| i as i64),
                    _ => None,
                },
                Some(Stated::Def(_, h)) => h.map(|h| h as i64),
                Some(Stated::Number(n)) => Some(*n),
                Some(Stated::Optional(n)) => *n,
                _ => None,
            };
            let picked = choices.iter().find(|x| x.value == now).cloned();
            pick_list(choices, picked, move |x: Choice<Option<i64>>| msg(Edit::Pick(x.value))).placeholder("choose one").into()
        }
        View::List(_) | View::Grid(_) => space().into(),
    }
}

/// What a pick offers, each by its label: an enum's variants, the
/// definitions offered, the numbers by name, another list's entries; "none"
/// first where the field may hold none.
fn pick_choices(e: &Editor, s: usize, f: &FieldPane, p: &Pick, ty: &FieldType) -> Vec<Choice<Option<i64>>> {
    let c = &e.content;
    let mut out = Vec::new();
    let none = Choice { label: "none".to_string(), value: None };
    match ty {
        FieldType::Enum(names) => out.extend(names.iter().enumerate().map(|(i, n)| Choice { label: n.clone(), value: Some(i as i64) })),
        FieldType::Ref(r, of) => {
            out.push(none);
            out.extend(offered(e, s, f, *r, of.as_deref()).into_iter().map(|h| Choice { label: name_of(e, *r, h), value: Some(h as i64) }));
        }
        _ => {
            if matches!(ty, FieldType::OptionalU8) {
                out.push(none);
            }
            if let Some(list) = &p.of {
                // (Another list's entries, by their place, from 0.)
                if let Some(Stated::List(items)) = e.side(s).facts.get(c, list) {
                    for (i, item) in items.iter().enumerate() {
                        let said = match item {
                            Stated::Record(fields) => fields
                                .iter()
                                .find_map(|(_, v)| if let Stated::Def(r, Some(h)) = v { Some(name_of(e, *r, *h)) } else { None })
                                .unwrap_or_default(),
                            Stated::Def(r, Some(h)) => name_of(e, *r, *h),
                            _ => String::new(),
                        };
                        out.push(Choice { label: format!("{}: {said}", i + 1), value: Some(i as i64) });
                    }
                }
            }
            out.extend(p.choices.iter().map(|(n, name)| Choice { label: name.clone(), value: Some(*n) }));
        }
    }
    out
}

/// A list's view: its summary; a checklist's boxes; a list's rows (each a
/// definition's name, or a record's fields side by side), each moved up or
/// down or taken out, and what may be added.
fn list_view<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, l: &'a List, ty: &FieldType, label: &str) -> Element<'a, Msg> {
    let c = &e.content;
    let path = f.path.clone();
    let msg = move |edit: Edit| Msg::Pane(s, path.clone(), edit);
    let items = match e.side(s).facts.get(c, &f.field) {
        Some(Stated::List(items)) => items,
        _ => Vec::new(),
    };
    let room = match ty {
        FieldType::List(_, n) => *n as usize,
        FieldType::Array(_, n) => *n as usize,
        _ => 0,
    };
    let mut col = Column::new().spacing(6);
    let mut top = row![text(label.to_string()).size(16)].spacing(12).align_y(Alignment::Center);
    if let Some(sm) = e.answers[s].summary.get(&f.path) {
        top = top.push(text(sm.clone()).size(13).color(DIM));
    }
    top = top.push(text(format!("{} of {room}", items.len())).size(13).color(DIM));
    col = col.push(top);
    let elem = element(ty).clone();
    match &elem {
        FieldType::Ref(registry, of) if l.checklist => {
            let held: Vec<u16> = items.iter().flat_map(Stated::defs).collect();
            for h in offered(e, s, f, *registry, of.as_deref()) {
                let on = held.contains(&h);
                let key = nettai_match::ids::key_of(c, *registry, h).unwrap_or_default();
                let face: Element<Msg> = match e.pictures.face(key) {
                    Some(picture) => image(picture.clone()).width(64).height(32).filter_method(image::FilterMethod::Nearest).into(),
                    None => space().width(64).height(32).into(),
                };
                let mut tick = checkbox(on);
                if on || held.len() < room {
                    let msg = msg.clone();
                    tick = tick.on_toggle(move |b| msg(Edit::Check(h, b)));
                }
                col = col.push(row![tick, face, text(name_of(e, *registry, h)).size(15)].spacing(10).align_y(Alignment::Center));
            }
            return scrollable(col).into();
        }
        FieldType::Ref(registry, of) => {
            for (i, item) in items.iter().enumerate() {
                let name = item.defs().first().map(|&h| name_of(e, *registry, h)).unwrap_or_default();
                let mut line = row![text(name).size(14).width(Length::Fill)].spacing(6).align_y(Alignment::Center);
                if !l.fixed {
                    line = line
                        .push(button(text("↑").size(12)).on_press(msg(Edit::Move(i, true))).style(button::text))
                        .push(button(text("↓").size(12)).on_press(msg(Edit::Move(i, false))).style(button::text))
                        .push(button(text("remove").size(12)).on_press(msg(Edit::Remove(i))).style(button::danger));
                }
                col = col.push(line);
            }
            if !l.fixed && items.len() < room {
                let needle = e.search.to_lowercase();
                let held: Vec<u16> = items.iter().flat_map(Stated::defs).collect();
                let mut add = Column::new().spacing(1);
                for h in offered(e, s, f, *registry, of.as_deref()) {
                    let name = name_of(e, *registry, h);
                    if held.contains(&h) || !(needle.is_empty() || name.to_lowercase().contains(&needle)) {
                        continue;
                    }
                    add = add.push(row![button(text("add").size(12)).on_press(msg(Edit::Add(h))).style(button::secondary), text(name).size(14)].spacing(6).align_y(Alignment::Center));
                }
                col = col.push(text_input("search", &e.search).on_input(move |t| Msg::Pane(s, f.path.clone(), Edit::Search(t))));
                col = col.push(scrollable(add).height(Length::Fixed(260.0)));
            }
        }
        FieldType::Record(fields) => {
            for (i, item) in items.iter().enumerate() {
                let Stated::Record(values) = item else { continue };
                let mut line = Row::new().spacing(8).align_y(Alignment::Center);
                for fd in fields.fields() {
                    let v = values.iter().find(|(n, _)| *n == fd.name).map(|(_, v)| v.clone()).unwrap_or(Stated::Other);
                    let declared = l.columns.iter().find(|(n, _)| *n == fd.name).map(|(_, v)| v.clone());
                    // (A fixed list's definitions are its rows' names.)
                    let cell: Element<Msg> = match (&v, &declared) {
                        (Stated::Def(r, h), None) if l.fixed => text(h.map(|h| name_of(e, *r, h)).unwrap_or_default()).size(14).width(Length::Fixed(160.0)).into(),
                        _ => {
                            let view = declared.unwrap_or_else(|| default_view(&fd.ty));
                            control(e, s, f, &view, &fd.ty, Some((i, fd.name.clone(), v)))
                        }
                    };
                    line = line.push(cell);
                }
                if !l.fixed {
                    line = line
                        .push(button(text("↑").size(12)).on_press(msg(Edit::Move(i, true))).style(button::text))
                        .push(button(text("↓").size(12)).on_press(msg(Edit::Move(i, false))).style(button::text))
                        .push(button(text("remove").size(12)).on_press(msg(Edit::Remove(i))).style(button::danger));
                }
                col = col.push(line);
            }
        }
        _ => {}
    }
    col.into()
}

/// The view a field of type `ty` has where none is declared.
fn default_view(ty: &FieldType) -> View {
    match nettai_match::panes::default_kind(ty) {
        "flag" => View::Flag,
        "pick" => View::Pick(Pick::default()),
        _ => View::Number { time: false },
    }
}

// ---- The grid, drawn ------------------------------------------------------------------

/// A cell's size on screen.
const CELL: f32 = 44.0;

/// A color by name: the view's palette's, else gray.
fn color_of(grid: &Grid, name: Option<&str>) -> Color {
    let rgb = name.and_then(|n| grid.palette.iter().find(|(k, _)| k == n)).map_or(0x909090, |(_, c)| *c);
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A placed piece as the grid draws it.
struct Part {
    cells: Vec<(i32, i32)>,
    color: Color,
    badge: Option<String>,
    picked: bool,
}

/// The held piece as the grid draws and puts it down.
struct Ghost {
    shape: Shape,
    grab: (i32, i32),
    color: Color,
    badge: Option<String>,
}

/// The grid, drawn, with the mouse on it.
struct Canvas {
    board: Vec<Vec<u8>>,
    rows_of_note: Vec<usize>,
    answers: GridAnswers,
    placed: Vec<Placed>,
    parts: Vec<Part>,
    occupied: HashMap<(i32, i32), usize>,
    held: Option<Ghost>,
    side: usize,
    path: String,
}

/// What the grid keeps between events: the cell under the cursor, the cell
/// a press began on, and a trackpad's scrolling toward the next turn.
#[derive(Default)]
struct Pointer {
    hovered: Option<(i32, i32)>,
    pressed: Option<(i32, i32)>,
    scrolled: f32,
}

/// A trackpad's scrolling that turns the held piece once.
const SCROLL_PER_TURN: f32 = 40.0;

impl Canvas {
    fn n(&self) -> i32 {
        self.board.len() as i32
    }

    fn cell(&self, p: Point) -> Option<(i32, i32)> {
        let (x, y) = ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32);
        ((0..self.n()).contains(&x) && (0..self.n()).contains(&y)).then_some((x, y))
    }

    fn landing(&self, cell: (i32, i32)) -> Option<((i32, i32), bool)> {
        let g = self.held.as_ref()?;
        let (x, y) = (cell.0 - g.grab.0, cell.1 - g.grab.1);
        Some(((x, y), fits(&self.answers, &self.placed, &g.shape, x, y)))
    }

    fn msg(&self, edit: GridEdit) -> canvas::Action<Msg> {
        canvas::Action::publish(Msg::Pane(self.side, self.path.clone(), Edit::Grid(edit))).and_capture()
    }

    fn place(&self, cell: (i32, i32)) -> Option<canvas::Action<Msg>> {
        match self.landing(cell)? {
            ((x, y), true) => Some(self.msg(GridEdit::Place(x, y))),
            _ => None,
        }
    }
}

fn cell_origin(x: i32, y: i32) -> Point {
    Point::new(x as f32 * CELL, y as f32 * CELL)
}

fn outline(frame: &mut Frame, x: i32, y: i32, inset: f32, color: Color, width: f32) {
    let r = Path::rectangle(cell_origin(x, y) + Vector::new(inset, inset), Size::new(CELL - 2.0 * inset, CELL - 2.0 * inset));
    frame.stroke(&r, Stroke::default().with_color(color).with_width(width));
}

fn badge(frame: &mut Frame, x: i32, y: i32, mark: &str, color: Color) {
    if mark == "+" {
        let mid = cell_origin(x, y) + Vector::new(CELL / 2.0, CELL / 2.0);
        let path = Path::new(|p| {
            p.move_to(mid + Vector::new(-8.0, 0.0));
            p.line_to(mid + Vector::new(8.0, 0.0));
            p.move_to(mid + Vector::new(0.0, -8.0));
            p.line_to(mid + Vector::new(0.0, 8.0));
        });
        frame.stroke(&path, Stroke::default().with_color(color).with_width(3.0));
    } else {
        frame.fill_text(canvas::Text {
            content: mark.to_string(),
            position: cell_origin(x, y) + Vector::new(CELL / 2.0 - 5.0, CELL / 2.0 - 9.0),
            color,
            size: 18.0.into(),
            ..canvas::Text::default()
        });
    }
}

impl canvas::Program<Msg> for Canvas {
    type State = Pointer;

    fn draw(&self, pointer: &Pointer, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(0x20, 0x28, 0x38));
        let cell = Size::new(CELL - 2.0, CELL - 2.0);
        for (y, row) in self.board.iter().enumerate() {
            for (x, &kind) in row.iter().enumerate() {
                let fill = match kind {
                    b'f' => Color::from_rgb8(0x38, 0x40, 0x58),
                    b'o' if self.rows_of_note.contains(&y) => Color::from_rgb8(0x50, 0x68, 0x90),
                    b'o' => Color::from_rgb8(0x5C, 0x60, 0x70),
                    _ => continue,
                };
                frame.fill_rectangle(cell_origin(x as i32, y as i32) + Vector::new(1.0, 1.0), cell, fill);
            }
        }
        let hovered = if self.held.is_none() { pointer.hovered.and_then(|c| self.occupied.get(&c).copied()) } else { None };
        for (i, part) in self.parts.iter().enumerate() {
            for &(x, y) in &part.cells {
                frame.fill_rectangle(cell_origin(x, y) + Vector::new(4.0, 4.0), Size::new(CELL - 8.0, CELL - 8.0), part.color);
                if let Some(b) = &part.badge {
                    badge(&mut frame, x, y, b, Color::WHITE);
                }
                if part.picked || hovered == Some(i) {
                    outline(&mut frame, x, y, 2.0, Color::WHITE, if part.picked { 3.0 } else { 2.0 });
                }
            }
        }
        if let (Some(g), Some(c)) = (&self.held, pointer.hovered)
            && let Some(((x, y), ok)) = self.landing(c)
        {
            let edge = if ok { Color::from_rgb8(0x60, 0xF0, 0x90) } else { Color::from_rgb8(0xF0, 0x40, 0x40) };
            let fill = Color { a: if ok { 0.8 } else { 0.45 }, ..g.color };
            for (cx, cy) in cells(&g.shape, x, y) {
                if !(0..self.n()).contains(&cx) || !(0..self.n()).contains(&cy) {
                    continue;
                }
                frame.fill_rectangle(cell_origin(cx, cy) + Vector::new(6.0, 6.0), Size::new(CELL - 12.0, CELL - 12.0), fill);
                if let Some(b) = &g.badge {
                    badge(&mut frame, cx, cy, b, Color { a: 0.8, ..Color::WHITE });
                }
                outline(&mut frame, cx, cy, 3.0, edge, 2.5);
            }
        }
        vec![frame.into_geometry()]
    }

    fn update(&self, pointer: &mut Pointer, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let over = cursor.position_in(bounds).and_then(|p| self.cell(p));
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if over != pointer.hovered {
                    pointer.hovered = over;
                    return Some(canvas::Action::request_redraw());
                }
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => {
                if pointer.hovered.take().is_some() {
                    return Some(canvas::Action::request_redraw());
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                pointer.pressed = over;
                let cell = over?;
                if self.held.is_some() {
                    return self.place(cell).or(Some(canvas::Action::capture()));
                }
                let i = *self.occupied.get(&cell)?;
                return Some(self.msg(GridEdit::PickUp(i, cell.0, cell.1)));
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = pointer.pressed.take();
                self.held.as_ref()?;
                return match over {
                    Some(cell) if pressed != Some(cell) => self.place(cell),
                    Some(_) => None,
                    None if pressed.is_some() => Some(self.msg(GridEdit::Remove)),
                    None => None,
                };
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let cell = over?;
                if self.held.is_some() {
                    return Some(self.msg(GridEdit::Remove));
                }
                let i = *self.occupied.get(&cell)?;
                return Some(self.msg(GridEdit::RotateAt(i)));
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if self.held.is_some() && over.is_some() {
                    let turn = match delta {
                        mouse::ScrollDelta::Lines { .. } => true,
                        mouse::ScrollDelta::Pixels { x, y } => {
                            pointer.scrolled += x.abs().max(y.abs());
                            pointer.scrolled >= SCROLL_PER_TURN
                        }
                    };
                    if !turn {
                        return Some(canvas::Action::capture());
                    }
                    pointer.scrolled = 0.0;
                    return Some(self.msg(GridEdit::Rotate));
                }
            }
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                self.held.as_ref()?;
                let edit = match key.as_ref() {
                    keyboard::Key::Character(c) if c.eq_ignore_ascii_case("r") => GridEdit::Rotate,
                    keyboard::Key::Named(keyboard::key::Named::Escape) => GridEdit::PutBack,
                    keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace) => GridEdit::Remove,
                    _ => return None,
                };
                return Some(self.msg(edit));
            }
            _ => {}
        }
        None
    }

    fn mouse_interaction(&self, _: &Pointer, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        let Some(cell) = cursor.position_in(bounds).and_then(|p| self.cell(p)) else { return mouse::Interaction::default() };
        if self.held.is_some() {
            mouse::Interaction::Grabbing
        } else if self.occupied.contains_key(&cell) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

/// A grid's view: the board with its pieces, the held or selected piece's
/// controls, and the pieces to pick up.
fn grid_view<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, grid: &'a Grid, ty: &FieldType) -> Element<'a, Msg> {
    let c = &e.content;
    // (A copy: it holds the view by reference.)
    let msg = move |edit: GridEdit| Msg::Pane(s, f.path.clone(), Edit::Grid(edit));
    let answers = e.answers[s].grids.get(&f.path).cloned().unwrap_or_default();
    let state = e.grids[s].get(&f.path).cloned().unwrap_or_default();
    let Some(FieldType::Ref(registry, of)) = record_field(ty, &grid.piece).cloned() else { return space().into() };
    let placed = placed(c, e.side(s), &f.field, grid);
    let mut occupied = HashMap::new();
    let parts: Vec<Part> = placed
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let cs = cells(&shape_of(&answers, p.piece, &p.toggles, p.rotation), p.x, p.y);
            for &cell in &cs {
                occupied.insert(cell, i);
            }
            Part {
                cells: cs,
                color: color_of(grid, p.color.as_deref()),
                badge: answers.badges.get(&p.piece).cloned(),
                picked: state.held.is_none() && state.selected == Some(i),
            }
        })
        .collect();
    let colors_of = |h: u16| answers.colors.get(&h).cloned().unwrap_or_default();
    let held = state.held.as_ref().map(|h| Ghost {
        shape: shape_of(&answers, h.piece, &h.toggles, h.rotation),
        grab: h.grab,
        color: color_of(grid, colors_of(h.piece).get(h.color as usize).map(String::as_str)),
        badge: answers.badges.get(&h.piece).cloned(),
    });
    let n = answers.board.as_ref().map_or(0, |b| b.len());
    let board = answers.board.clone().unwrap_or_default();
    let canvas = Canvas {
        board,
        rows_of_note: grid.rows_of_note.iter().map(|(r, _)| *r).collect(),
        answers: answers.clone(),
        placed: placed.clone(),
        parts,
        occupied,
        held,
        side: s,
        path: f.path.clone(),
    };
    let board_widget: Element<Msg> = if n == 0 {
        text("No board: nothing is placed (see the field above).").size(13).into()
    } else {
        canvas_widget(canvas).width(Length::Fixed(CELL * n as f32)).height(Length::Fixed(CELL * n as f32)).into()
    };
    let notes: Vec<String> = grid.rows_of_note.iter().map(|(r, why)| format!("Row {}: {}", r + 1, said(e, why))).collect();
    let color_buttons = |piece: u16, current: u8| {
        colors_of(piece).into_iter().enumerate().fold(row![].spacing(4), |r, (k, name)| {
            let b = button(text(name).size(12)).on_press(msg(GridEdit::Color(k as u8)));
            r.push(if k == current as usize { b.style(button::primary) } else { b.style(button::secondary) })
        })
    };
    let toggles = |on: &[bool]| {
        grid.toggles.iter().enumerate().fold(row![].spacing(10), |r, (k, t)| {
            let state = on.get(k).copied().unwrap_or(false);
            r.push(checkbox(state).label(said(e, t)).on_toggle(move |b| msg(GridEdit::Toggle(k, b))))
        })
    };
    let picked: Element<Msg> = if let Some(h) = &state.held {
        let mut buttons = row![button("Turn").on_press(msg(GridEdit::Rotate))].spacing(4);
        if h.origin.is_some() {
            buttons = buttons.push(button("Put back").on_press(msg(GridEdit::PutBack)).style(button::secondary));
        }
        buttons = buttons.push(button("Take off").on_press(msg(GridEdit::Remove)).style(button::danger));
        column![
            text(format!("Holding {}, turned {}", name_of(e, registry, h.piece), h.rotation)).size(15),
            color_buttons(h.piece, h.color),
            row![buttons, toggles(&h.toggles)].spacing(12).align_y(Alignment::Center),
            text("Click a cell (or let go of a drag over one) to put it down where it shows lit. The wheel or R turns it; right-click, Delete or a drag off the grid takes it off; Esc puts it back.")
                .size(12),
        ]
        .spacing(6)
        .into()
    } else if let Some(p) = state.selected.and_then(|i| placed.get(i)) {
        let current = p.color.as_ref().and_then(|n| colors_of(p.piece).iter().position(|x| x == n)).unwrap_or(0) as u8;
        column![
            text(format!("{} at ({}, {}), turned {}", name_of(e, registry, p.piece), p.x, p.y, p.rotation)).size(15),
            color_buttons(p.piece, current),
            row![
                row![
                    button("←").on_press(msg(GridEdit::Nudge(-1, 0))),
                    button("↑").on_press(msg(GridEdit::Nudge(0, -1))),
                    button("↓").on_press(msg(GridEdit::Nudge(0, 1))),
                    button("→").on_press(msg(GridEdit::Nudge(1, 0))),
                    button("Turn").on_press(msg(GridEdit::Rotate)),
                    button("Remove").on_press(msg(GridEdit::Remove)).style(button::danger),
                ]
                .spacing(4),
                toggles(&p.toggles),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(6)
        .into()
    } else {
        text("Drag a color square from the list onto the grid (or click it, then a cell). Press a placed piece to pick it up and drag it; right-click one to turn it.")
            .size(13)
            .into()
    };
    // The pieces to pick up, searched.
    let needle = state.search.to_lowercase();
    let pieces = offered(e, s, f, registry, of.as_deref());
    let list = pieces.into_iter().fold(Column::new().spacing(2), |col, h| {
        let name = name_of(e, registry, h);
        if !(needle.is_empty() || name.to_lowercase().contains(&needle)) {
            return col;
        }
        let swatches = colors_of(h).into_iter().enumerate().fold(row![].spacing(3), |r, (k, cname)| {
            let holding = state.held.as_ref().is_some_and(|x| x.origin.is_none() && x.piece == h && x.color == k as u8);
            let shown = color_of(grid, Some(&cname));
            let swatch = container(space().width(Length::Fixed(18.0)).height(Length::Fixed(18.0))).style(move |_: &Theme| container::Style {
                background: Some(shown.into()),
                border: iced::Border { color: if holding { Color::BLACK } else { Color::TRANSPARENT }, width: 2.0, radius: 3.0.into() },
                ..container::Style::default()
            });
            r.push(mouse_area(swatch).on_press(msg(GridEdit::Hold(h, k as u8))).interaction(mouse::Interaction::Grab))
        });
        let mark = answers.badges.get(&h).cloned().unwrap_or_default();
        col.push(
            row![
                text(name).size(14).width(Length::Fill),
                text(mark).size(12).color(DIM).width(Length::Fixed(20.0)),
                container(swatches).width(Length::Fixed(110.0)),
                space().width(Length::Fixed(14.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
    });
    let mut left = column![board_widget].spacing(8).width(Length::Fixed(CELL * n.max(7) as f32 + 120.0));
    for note in notes {
        left = left.push(text(note).size(12));
    }
    left = left.push(picked);
    left = left.push(crate::editor::view::round_stats(e, s));
    let search_path = f.path.clone();
    let right = column![
        text_input("search", &state.search).on_input(move |t| Msg::Pane(s, search_path.clone(), Edit::Search(t))),
        scrollable(list).height(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill);
    row![scrollable(left), right].spacing(16).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EXE6's panes, asked of a round of a random match: the side's
    /// MegaMan shows them all; the Crosses offered are his ten; the
    /// NaviCust's grid has its board and every program's shapes, colors
    /// and badges; the patch cards their summary.
    #[test]
    fn exe6s_panes_answer() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        nettai_match::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let panes = nettai_match::panes::panes(&content).unwrap();
        let mut b = nettai_match::check::start(&content, &m).unwrap();
        let a = ask(&content, &panes, &mut b, 0);
        assert_eq!(a.errors, Vec::<String>::new());
        assert!(a.shown.iter().all(|&x| x), "{:?}", a.shown);
        let crosses = panes.iter().flat_map(|p| &p.fields).find(|f| f.field == "crosses").unwrap();
        assert_eq!(a.offered[&crosses.path].len(), 10);
        let grid = panes.iter().flat_map(|p| &p.fields).find(|f| matches!(f.view, View::Grid(_))).unwrap();
        let g = &a.grids[&grid.path];
        assert_eq!(g.board.as_ref().map(Vec::len), Some(7));
        let programs = content.defs.entries_of("navicust_programs").len();
        assert_eq!((g.colors.len(), g.shapes.len()), (programs, 2 * programs));
        assert!(!g.badges.is_empty(), "the plus parts' badge");
        let cards = panes.iter().flat_map(|p| &p.fields).find(|f| f.field == "patch_cards").unwrap();
        assert_eq!(a.summary[&cards.path], "0 MB of 80");
    }

    /// A grid's edits on EXE6's NaviCust: pieces are held from the list and
    /// put down where they fit (not over another), picked up by a cell and
    /// put back, turned, toggled, recolored, taken off; and the match stays
    /// one the checks accept.
    #[test]
    fn grid_edits() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        nettai_match::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let panes = nettai_match::panes::panes(&content).unwrap();
        let mut b = nettai_match::check::start(&content, &m).unwrap();
        let a = ask(&content, &panes, &mut b, 0);
        let f = panes.iter().flat_map(|p| &p.fields).find(|f| matches!(f.view, View::Grid(_))).unwrap().clone();
        let View::Grid(grid) = &f.view else { unreachable!() };
        let setup = content.defs.schema(content.defs.rules().unwrap().setup);
        let ty = setup.field(setup.index_of(&f.field).unwrap()).ty.clone();
        let answers = a.grids[&f.path].clone();
        let mut state = GridState::default();
        let program = |name: &str| nettai_match::ids::entry(&content, "exe6", "navicust_programs", name).unwrap().0;
        let side = &mut m.sides[0];
        let edit = |side: &mut Side, state: &mut GridState, e: GridEdit| grid_edit(&content, side, &f.field, &ty, grid, &answers, state, e);
        let count = |side: &Side| placed(&content, side, &f.field, grid).len();
        assert!(!edit(side, &mut state, GridEdit::Hold(program("suprarmr"), 0)));
        assert!(edit(side, &mut state, GridEdit::Place(2, 3)));
        assert_eq!((count(side), state.held.is_none(), state.selected), (1, true, Some(0)));
        edit(side, &mut state, GridEdit::Hold(program("hp-50"), 1));
        assert!(!edit(side, &mut state, GridEdit::Place(2, 3)), "not over SuprArmr");
        assert!(edit(side, &mut state, GridEdit::Place(4, 2)));
        assert_eq!(count(side), 2);
        // Picked up by a cell and put back as it was.
        let first = placed(&content, side, &f.field, grid)[0].clone();
        assert!(edit(side, &mut state, GridEdit::PickUp(0, 2, 3)));
        assert_eq!(count(side), 1);
        assert!(edit(side, &mut state, GridEdit::PutBack));
        let back = placed(&content, side, &f.field, grid);
        assert_eq!((back[0].piece, back[0].x, back[0].y), (first.piece, first.x, first.y));
        // Turned, recolored, toggled and taken off, the selected one.
        state.selected = Some(1);
        assert!(edit(side, &mut state, GridEdit::Rotate));
        assert_eq!(placed(&content, side, &f.field, grid)[1].rotation, 1);
        assert!(edit(side, &mut state, GridEdit::Color(0)));
        assert!(edit(side, &mut state, GridEdit::Remove));
        assert_eq!(count(side), 1);
        assert_eq!(nettai_match::check_match(&content, &m), Vec::<String>::new());
    }

    /// A shape turns about its center a quarter clockwise; four turns are
    /// none.
    #[test]
    fn shapes_turn() {
        let up: Shape = vec![vec![false, true, false], vec![false, true, false], vec![false, false, false]];
        let right: Shape = vec![vec![false, false, false], vec![false, true, true], vec![false, false, false]];
        assert_eq!(rotate(&up, 1), right);
        assert_eq!(rotate(&up, 4), up);
        assert_eq!(cells(&right, 5, 5), [(5, 5), (6, 5)]);
    }
}
