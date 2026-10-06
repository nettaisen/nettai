//! The NaviCust's pane: the editor's own view of a game's NaviCust, chosen
//! by its data's names and shape (`crate::editor::layout`), never by a
//! game's name. It is a grid where the data fit what it reads, all of it as
//! data (nothing of the content is called):
//!
//! - the setup's `navicust_expansions` (the board's size, by its place
//!   among the boards) and `navicust_programs`, a list of `{ program,
//!   color, x, y, rotation, compressed }` (`program` an entry of a
//!   collection, `color` an enum of color names);
//! - each program's data: its `shape` (rows of `#` and `.`, centered),
//!   `compressed` (likewise, where it compresses), `colors` (names) and
//!   `plus` (a plus part's mark);
//! - the game's module `rules/navicust/board`, as it loaded
//!   (`Battle::module_data`): its `boards` (rows of `o` a cell, `f` its
//!   frame, `.` none) and `COMMAND_LINE` (a row, from 0).
//!
//! The colors are the editor's own, by name. Where a game's data don't fit,
//! the programs are a list like any other (the layout's generic view).
//!
//! The grid is edited with the mouse as Tango's NaviCust editor is: a piece
//! is *held* and follows the cursor as a ghost, lit where it would land and
//! red where it doesn't fit. A color swatch in the list picks one up;
//! pressing a placed piece picks it up off the grid, by the cell pressed. A
//! click on the grid (or letting go of a drag over it) puts it down where it
//! fits; letting go of a drag off the grid, right-clicking or Delete takes
//! it off; Escape puts a piece picked off the grid back. The wheel or R
//! turns the held piece. Right-clicking a placed piece turns it where it
//! is. A piece fits where its cells are on the board (`o`) or its frame
//! (`f`), not all on the frame, and over no other. A piece the rules'
//! `validate` says something of is outlined red, with what they say below.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::editor::app::{Editor, Msg};
use crate::editor::facts::title;
use crate::editor::layout::{FieldPane, Grid, Pane, Pick, View, element, record_field};
use crate::editor::panes::{Edit, name_of, offered, values};
use crate::editor::view::{DIM, RED};
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Column, button, canvas as canvas_widget, checkbox, column, container, mouse_area, row, scrollable, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use nettai_battle::content::Content;
use nettai_content_api::{FieldType, Registry};
use nettai_match::Side;
use nettai_match::facts::Stated;

/// A shape: rows of cells, `true` a cell it covers, its center the middle
/// cell.
pub type Shape = Vec<Vec<bool>>;

/// The setup fields the grid edits, and the record's fields it reads.
const SIZE_FIELD: &str = "navicust_expansions";
const PROGRAMS_FIELD: &str = "navicust_programs";

/// The colors a program comes in, by name (the editor's own).
const PALETTE: [(&str, u32); 6] =
    [("white", 0xDEDEDE), ("pink", 0xF08CC8), ("yellow", 0xF0D840), ("red", 0xE85048), ("blue", 0x4C8CF0), ("green", 0x58C858)];

/// What the grid reads of a game's NaviCust: its boards, its command line,
/// and its programs' shapes (by whether compressed), colors and plus marks.
#[derive(Clone, Debug, Default)]
pub struct Data {
    pub boards: Vec<Vec<Vec<u8>>>,
    pub command_line: Option<usize>,
    pub shapes: HashMap<(u16, Vec<bool>), Shape>,
    pub colors: HashMap<u16, Vec<String>>,
    pub plus: HashSet<u16>,
}

/// Rows of a shape or a board, from data of strings, each of `cells` alone.
fn rows(d: &nettai_content_api::Data, cells: &[u8]) -> Option<Vec<Vec<u8>>> {
    let nettai_content_api::Data::List(items) = d else { return None };
    let rows: Vec<Vec<u8>> = items.iter().map(|r| r.str().map(|s| s.bytes().collect())).collect::<Option<_>>()?;
    let n = rows.first()?.len();
    rows.iter().all(|r| r.len() == n && r.iter().all(|b| cells.contains(b))).then_some(rows)
}

/// A shape from its rows of `#` and `.`.
fn shape(rows: &[Vec<u8>]) -> Shape {
    rows.iter().map(|r| r.iter().map(|&b| b == b'#').collect()).collect()
}

/// What the grid reads of `game`'s NaviCust (its board module from a round
/// of a random match of it, one that starts), or none where its data don't
/// fit the grid.
pub fn read(content: &Arc<Content>, game: &str) -> Option<Data> {
    // The setup's fields.
    let field = |name: &str| nettai_match::facts::field(content, name).map(|f| f.ty.clone());
    let FieldType::Record(fields) = element(&field(PROGRAMS_FIELD)?).clone() else { return None };
    let ty = |name: &str| fields.index_of(name).map(|i| fields.field(i).ty.clone());
    let Some(FieldType::Ref(Registry::Entry, Some(collection))) = ty("program") else { return None };
    let number = |name: &str| ty(name).is_some_and(|t| nettai_match::facts::range(&t).is_some());
    if !(number("x") && number("y") && number("rotation") && matches!(ty("color"), Some(FieldType::Enum(_))) && ty("compressed") == Some(FieldType::Bool)) {
        return None;
    }
    field(SIZE_FIELD)?;
    // The board module.
    let m = nettai_match::pick::live(content, game, 0, None).ok()?;
    let module = nettai_match::check::start(content, &m).ok()?.module_data(&format!("{game}:rules/navicust/board"))?.ok()?;
    let nettai_content_api::Data::List(boards) = module.field("boards") else { return None };
    let boards: Vec<Vec<Vec<u8>>> = boards.iter().map(|b| rows(b, b"of.")).collect::<Option<_>>()?;
    if boards.is_empty() {
        return None;
    }
    let mut data = Data { boards, command_line: module.field("COMMAND_LINE").int().map(|r| r as usize), ..Data::default() };
    // The programs.
    for h in content.defs.entries_of(&collection) {
        let d = content.defs.definitions.get(Registry::Entry, &content.defs.entry(h).key)?;
        let plain = rows(d.spec.field("shape"), b"#.")?;
        let compressed = match d.spec.field("compressed") {
            nettai_content_api::Data::Nil => plain.clone(),
            c => rows(c, b"#.")?,
        };
        data.shapes.insert((h.0, vec![false]), shape(&plain));
        data.shapes.insert((h.0, vec![true]), shape(&compressed));
        let nettai_content_api::Data::List(colors) = d.spec.field("colors") else { return None };
        data.colors.insert(h.0, colors.iter().map(|c| c.str().map(str::to_string)).collect::<Option<_>>()?);
        if d.spec.field("plus") == &nettai_content_api::Data::Bool(true) {
            data.plus.insert(h.0);
        }
    }
    Some(data)
}

/// A board's size as it shows: its cells' columns by rows (`5x4`).
fn size(board: &[Vec<u8>]) -> String {
    let rows = board.iter().filter(|r| r.contains(&b'o')).count();
    let columns = (0..board.first().map_or(0, Vec::len)).filter(|&x| board.iter().any(|r| r[x] == b'o')).count();
    format!("{columns}x{rows}")
}

/// The NaviCust's pane: the board's size, a pick of the boards; the
/// programs placed, a grid.
pub fn pane(_content: &Content, data: &Data) -> Option<Pane> {
    let choices = data.boards.iter().enumerate().map(|(i, b)| (i as i64, size(b))).collect();
    let grid = Grid {
        piece: "program".into(),
        x: "x".into(),
        y: "y".into(),
        rotation: "rotation".into(),
        color: "color".into(),
        toggles: vec!["compressed".into()],
        size: SIZE_FIELD.into(),
    };
    Some(Pane {
        key: "navicust".into(),
        title: "NaviCust".into(),
        fields: vec![
            FieldPane { field: SIZE_FIELD.into(), label: "Board".into(), view: View::Pick(Pick { choices }), path: SIZE_FIELD.into() },
            FieldPane { field: PROGRAMS_FIELD.into(), label: "Programs".into(), view: View::Grid(grid), path: PROGRAMS_FIELD.into() },
        ],
    })
}

/// What a side's grid shows: the board of its size, and the programs'
/// shapes, colors and marks.
#[derive(Clone, Debug, Default)]
pub struct Look {
    pub board: Option<Vec<Vec<u8>>>,
    pub shapes: HashMap<(u16, Vec<bool>), Shape>,
    pub colors: HashMap<u16, Vec<String>>,
    pub badges: HashMap<u16, String>,
}

/// The grid's look for side `side`.
pub fn look(data: Option<&Data>, content: &Content, side: &Side, grid: &Grid) -> Look {
    let Some(d) = data else { return Look::default() };
    let size = match side.facts.get(content, &grid.size) {
        Some(Stated::Optional(Some(n)) | Stated::Number(n)) => Some(n as usize),
        _ => None,
    };
    Look {
        board: size.and_then(|n| d.boards.get(n).cloned()),
        shapes: d.shapes.clone(),
        colors: d.colors.clone(),
        badges: d.plus.iter().map(|&h| (h, "+".to_string())).collect(),
    }
}

// ---- What a grid keeps, and its edits ------------------------------------------------

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

// ---- The grid's edits -----------------------------------------------------------------

/// A grid's piece placed: which, its center, its turns, its color (by
/// name), its toggles; and its record as read, for the fields the view
/// doesn't name.
#[derive(Clone, Debug)]
pub struct Placed {
    pub piece: u16,
    pub x: i32,
    pub y: i32,
    pub rotation: u8,
    pub color: Option<String>,
    pub toggles: Vec<bool>,
    pub record: Stated,
}

/// The pieces a grid's list holds.
pub fn placed(c: &Content, side: &Side, field: &str, grid: &Grid) -> Vec<Placed> {
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
                color: get(&grid.color).and_then(|v| if let Stated::Variant(n) = v { n.clone() } else { None }),
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
    set(&grid.color, Stated::Variant(p.color.clone()));
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
fn shape_of(answers: &Look, piece: u16, toggles: &[bool], rotation: u8) -> Shape {
    answers.shapes.get(&(piece, toggles.to_vec())).map_or_else(Vec::new, |s| rotate(s, rotation))
}

/// Whether `shape` fits with its center on (x, y): its cells on the board
/// or its margin, not all on the margin, and over none of `others`'.
fn fits(answers: &Look, others: &[Placed], shape: &Shape, x: i32, y: i32) -> bool {
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
pub fn grid_edit(c: &Content, side: &mut Side, field: &str, ty: &FieldType, grid: &Grid, answers: &Look, state: &mut GridState, edit: GridEdit) -> bool {
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
        color: get(&grid.color).and_then(|v| if let Stated::Variant(n) = v { n.clone() } else { None }),
        toggles: grid.toggles.iter().map(|t| matches!(get(t), Some(Stated::Flag(true)))).collect(),
        record: record.clone(),
    })
}

// ---- The grid, drawn ------------------------------------------------------------------

/// A cell's size on screen.
const CELL: f32 = 44.0;

/// A color by name: the palette's, else gray.
fn color_of(name: Option<&str>) -> Color {
    let rgb = name.and_then(|n| PALETTE.iter().find(|(k, _)| *k == n)).map_or(0x909090, |(_, c)| *c);
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// A placed piece as the grid draws it.
struct Part {
    cells: Vec<(i32, i32)>,
    color: Color,
    badge: Option<String>,
    picked: bool,
    /// The rules say something of it.
    bad: bool,
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
    answers: Look,
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
                } else if part.bad {
                    outline(&mut frame, x, y, 2.0, Color::from_rgb8(0xF0, 0x40, 0x40), 3.0);
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
pub fn grid_view<'a>(e: &'a Editor, s: usize, f: &'a FieldPane, grid: &'a Grid, ty: &FieldType) -> Element<'a, Msg> {
    let c = &e.content;
    // (A copy: it holds the view by reference.)
    let msg = move |edit: GridEdit| Msg::Pane(s, f.path.clone(), Edit::Grid(edit));
    let answers = look(e.navicust.as_ref(), c, e.side(s), grid);
    // What the rules say of the programs (by their places) and of the board.
    let said: Vec<&nettai_match::check::Problem> =
        e.problems.iter().filter(|p| p.side == Some(s) && (p.field.as_deref() == Some(&f.field) || p.field.as_deref() == Some(&grid.size))).collect();
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
                color: color_of(p.color.as_deref()),
                badge: answers.badges.get(&p.piece).cloned(),
                picked: state.held.is_none() && state.selected == Some(i),
                bad: said.iter().any(|x| x.field.as_deref() == Some(&f.field) && x.entry == Some(i)),
            }
        })
        .collect();
    let colors_of = |h: u16| answers.colors.get(&h).cloned().unwrap_or_default();
    let held = state.held.as_ref().map(|h| Ghost {
        shape: shape_of(&answers, h.piece, &h.toggles, h.rotation),
        grab: h.grab,
        color: color_of(colors_of(h.piece).get(h.color as usize).map(String::as_str)),
        badge: answers.badges.get(&h.piece).cloned(),
    });
    let n = answers.board.as_ref().map_or(0, |b| b.len());
    let board = answers.board.clone().unwrap_or_default();
    let canvas = Canvas {
        board,
        rows_of_note: e.navicust.as_ref().and_then(|d| d.command_line).into_iter().collect(),
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
    let notes: Vec<String> = e.navicust.as_ref().and_then(|d| d.command_line).map(|r| format!("Row {}: the command line", r + 1)).into_iter().collect();
    let color_buttons = |piece: u16, current: u8| {
        colors_of(piece).into_iter().enumerate().fold(row![].spacing(4), |r, (k, name)| {
            let b = button(text(name).size(12)).on_press(msg(GridEdit::Color(k as u8)));
            r.push(if k == current as usize { b.style(button::primary) } else { b.style(button::secondary) })
        })
    };
    let toggles = |on: &[bool]| {
        grid.toggles.iter().enumerate().fold(row![].spacing(10), |r, (k, t)| {
            let state = on.get(k).copied().unwrap_or(false);
            r.push(checkbox(state).label(title(t)).on_toggle(move |b| msg(GridEdit::Toggle(k, b))))
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
    let pieces = offered(e, registry, of.as_deref());
    let list = pieces.into_iter().fold(Column::new().spacing(2), |col, h| {
        let name = name_of(e, registry, h);
        if !(needle.is_empty() || name.to_lowercase().contains(&needle)) {
            return col;
        }
        let swatches = colors_of(h).into_iter().enumerate().fold(row![].spacing(3), |r, (k, cname)| {
            let holding = state.held.as_ref().is_some_and(|x| x.origin.is_none() && x.piece == h && x.color == k as u8);
            let shown = color_of(Some(&cname));
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
    for p in &said {
        let at = match p.entry {
            Some(i) if p.field.as_deref() == Some(&f.field) => format!("Program {}: ", i + 1),
            _ => String::new(),
        };
        left = left.push(text(format!("{at}{}", p.text)).size(13).color(RED));
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

    /// Both games' NaviCusts fit the grid: their boards from the board
    /// module (three, the command line row 3), every program's shapes,
    /// colors and plus mark from its data; the pane picks the board by its
    /// size.
    #[test]
    fn both_games_navicusts_read() {
        for (content, game) in [(nettai_match::testing::exe6_content(), "exe6"), (nettai_match::testing::exe5_content(), "exe5")] {
            let data = read(&content, game).unwrap_or_else(|| panic!("{game}: the NaviCust's data don't fit the grid"));
            assert_eq!((data.boards.len(), data.command_line), (3, Some(3)), "{game}");
            let programs = content.defs.entries_of("navicust_programs").len();
            assert_eq!((data.colors.len(), data.shapes.len()), (programs, 2 * programs), "{game}");
            let p = pane(&content, &data).unwrap();
            let View::Pick(pick) = &p.fields[0].view else { panic!("{game}: the board a pick") };
            let sizes: Vec<&str> = pick.choices.iter().map(|(_, s)| s.as_str()).collect();
            assert_eq!(sizes, ["4x4", "5x4", "5x5"], "{game}");
        }
        let six = nettai_match::testing::exe6_content();
        let data = read(&six, "exe6").unwrap();
        assert!(!data.plus.is_empty(), "EXE6's plus parts");
        assert_eq!(data.boards[1][1], b"fooooof".to_vec());
    }

    /// A grid's edits on EXE6's NaviCust: pieces are held from the list and
    /// put down where they fit (not over another), picked up by a cell and
    /// put back, turned, recolored, taken off; and the match stays one the
    /// checks accept.
    #[test]
    fn grid_edits() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        nettai_match::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let data = read(&content, "exe6").unwrap();
        let p = pane(&content, &data).unwrap();
        let f = p.fields[1].clone();
        let View::Grid(grid) = &f.view else { unreachable!() };
        let ty = nettai_match::facts::field(&content, &f.field).unwrap().ty.clone();
        let answers = look(Some(&data), &content, &m.sides[0], grid);
        assert_eq!(answers.board.as_ref().map(Vec::len), Some(7));
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
        // Turned, recolored and taken off, the selected one.
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
