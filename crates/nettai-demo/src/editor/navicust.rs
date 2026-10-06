//! The NaviCust pane: a side's programs on the 7x7 grid as the game draws
//! them (the board of its expansions, its frame, the command line), each in
//! its color and shape, turned and compressed. What they give the navi (the
//! stats and the bugs) is the rules' compile, shown as the round starts it;
//! what's wrong (off the board, over another) the checks'.
//!
//! The grid is edited with the mouse as Tango's NaviCust editor is: a
//! program is *held* and follows the cursor as a ghost, lit where it would
//! land and red where it doesn't fit. A color swatch in the list picks one
//! up; pressing a placed program picks it up off the grid, by the cell
//! pressed. A click on the grid (or letting go of a drag over it) puts it
//! down where it fits; letting go of a drag off the grid, right-clicking or
//! Delete takes it off; Escape puts a program picked off the grid back. The
//! wheel or R turns the held program, C compresses it. Right-clicking a
//! placed program turns it where it is.

use crate::editor::app::{Choice, Editor, Msg};
use iced::keyboard;
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Column, button, canvas as canvas_widget, column, container, mouse_area, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use nettai_battle::Content;
use nettai_battle::content::{Board, BoardCell, NaviCustRules};
use nettai_battle::navicust::{SIZE, Shape, cells};
use nettai_content_api::EntryHandle as NaviCustProgramHandle;
use nettai_match::{Arena, Side};

// ---- The programs' data ----------------------------------------------------------------
//
// (Until the editor draws a side's facts by the views its rules declare, it
// reads the programs' shapes and colors from their data, the game's root's
// `navicust_programs`.)

/// The game's collection of NaviCust programs (its root's key).
pub const PROGRAMS: &str = "navicust_programs";

/// An entry's data (one of the game's collections').
pub fn entry_data(c: &Content, h: NaviCustProgramHandle) -> &nettai_content_api::Data {
    let key = &c.defs.entry(h).key;
    c.defs.definitions.get(nettai_content_api::Registry::Entry, key).map(|d| &d.spec).unwrap_or(&nettai_content_api::Data::Nil)
}

/// A whole number of an entry's data, 0 without one.
pub fn entry_int(c: &Content, h: NaviCustProgramHandle, field: &str) -> i64 {
    entry_data(c, h).field(field).int().unwrap_or(0)
}

/// Program `h`'s colors (in its variants' order), as its data names them.
pub fn program_colors(c: &Content, h: NaviCustProgramHandle) -> Vec<&str> {
    match entry_data(c, h).field("colors") {
        nettai_content_api::Data::List(l) => l.iter().filter_map(|x| x.str()).collect(),
        _ => Vec::new(),
    }
}

/// What the board draws of a program: its colors, whether it is a plus
/// part, its shape and its compressed one.
pub struct Program {
    pub key: String,
    pub colors: Vec<String>,
    pub plus: bool,
    pub shape: nettai_battle::navicust::Shape,
    pub compressed: Option<nettai_battle::navicust::Shape>,
}

impl Program {
    /// Its shape as placed: compressed or not, turned.
    pub fn placed_shape(&self, compressed: bool, rotation: u8) -> nettai_battle::navicust::Shape {
        let s = if compressed { self.compressed.as_ref().unwrap_or(&self.shape) } else { &self.shape };
        nettai_battle::navicust::rotate(s, rotation)
    }
}

/// Program `h`'s data, read.
pub fn program_of(c: &Content, h: NaviCustProgramHandle) -> Program {
    use nettai_content_api::Data;
    let d = entry_data(c, h);
    let colors = match d.field("colors") {
        Data::List(l) => l.iter().filter_map(|x| x.str().map(str::to_string)).collect(),
        _ => Vec::new(),
    };
    let shape = |f: &str| nettai_battle::navicust::read_shape(d.field(f)).ok();
    Program {
        key: c.defs.entry(h).key.clone(),
        colors,
        plus: *d.field("plus") == Data::Bool(true),
        shape: shape("shape").unwrap_or_default(),
        compressed: shape("compressed"),
    }
}

/// A cell's size on the screen.
const CELL: f32 = 46.0;

/// What the pane keeps of its own: the program held (picked up to place),
/// the one last placed (its colors and the like shown beside the grid),
/// the list's search, and whether the stats-and-bugs block is shown in
/// place of the grid.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub held: Option<Held>,
    pub selected: Option<usize>,
    pub search: String,
    pub show_stats: bool,
}

/// A program picked up: what it is and how it is turned and compressed,
/// the cell of it the cursor holds (as an offset from its center, so that
/// cell stays under the cursor), and where it was on the grid if it was
/// picked off it (its place in the list and the program as it was).
#[derive(Clone, Copy, Debug)]
pub struct Held {
    pub program: NaviCustProgramHandle,
    pub color: u8,
    pub rotation: u8,
    pub compressed: bool,
    pub grab: (i32, i32),
    pub origin: Option<(usize, PlacedProgram)>,
}

#[derive(Clone, Debug)]
pub enum Edit {
    /// Show what the programs make of the stats (the round's) in place of
    /// the grid, or the grid.
    ShowStats(bool),
    Expansions(u8),
    /// A program picked up from the list, in one of its colors.
    Hold(NaviCustProgramHandle, u8),
    /// The placed program at this place in the list picked up by the cell
    /// pressed.
    PickUp(usize, u8, u8),
    /// The held program put down with its center on this cell.
    Place(u8, u8),
    /// The held program put back where it was picked up from.
    PutBack,
    /// The placed program at this place in the list turned where it is.
    RotateAt(usize),
    /// The held program, else the last placed: turned, compressed,
    /// recolored, moved a cell, taken off.
    Rotate,
    Compress(bool),
    Color(u8),
    Move(i8, i8),
    Remove,
    Search(String),
}

/// A program color's look (the NaviCust's, as Tango draws them).
fn color(name: &str) -> Color {
    let (r, g, b) = match name {
        "white" => (0xDE, 0xDE, 0xDE),
        "yellow" => (0xDE, 0xDE, 0x00),
        "pink" => (0xDE, 0x8C, 0xC6),
        "red" => (0xDE, 0x10, 0x00),
        "blue" => (0x29, 0x84, 0xDE),
        "green" => (0x18, 0xC6, 0x00),
        "orange" => (0xDE, 0x7B, 0x00),
        "purple" => (0x94, 0x00, 0xCE),
        _ => (0x84, 0x84, 0x84),
    };
    Color::from_rgb8(r, g, b)
}

/// A program on the grid, as the pane edits it: an entry of the side's
/// `navicust_programs` (its color by its place in the definition's
/// `colors`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedProgram {
    pub program: NaviCustProgramHandle,
    pub color: u8,
    pub x: u8,
    pub y: u8,
    pub rotation: u8,
    pub compressed: bool,
}

/// The side's NaviCust (its rules' `navicust_expansions` and
/// `navicust_programs`): its board's expansions and its programs; none
/// where it states no board.
pub fn navicust_of(content: &Content, side: &Side) -> Option<(u8, Vec<PlacedProgram>)> {
    use nettai_content_api::{FieldValue, Registry};
    use nettai_match::facts::Stated;
    let expansions = match side.facts.fact(content, "navicust_expansions")?.value() {
        FieldValue::OptionalU8(n) => n?,
        _ => return None,
    };
    let Some(Stated::List(items)) = side.facts.get(content, "navicust_programs") else { return Some((expansions, Vec::new())) };
    let parts = items
        .iter()
        .filter_map(|item| {
            let Stated::Record(fields) = item else { return None };
            let get = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v);
            let program = match get("program")? {
                Stated::Def(Registry::Entry, Some(h)) => NaviCustProgramHandle(*h),
                _ => return None,
            };
            let number = |name: &str| match get(name) {
                Some(Stated::Number(n)) => *n as u8,
                _ => 0,
            };
            let color = match get("color")? {
                Stated::Variant(Some(name)) => program_of(content, program).colors.iter().position(|c| c == name).unwrap_or(0) as u8,
                _ => 0,
            };
            let compressed = matches!(get("compressed"), Some(Stated::Flag(true)));
            Some(PlacedProgram { program, color, x: number("x"), y: number("y"), rotation: number("rotation"), compressed })
        })
        .collect();
    Some((expansions, parts))
}

/// State the side's NaviCust: `parts` on the board of `expansions`.
fn set(content: &Content, side: &mut Side, parts: Vec<PlacedProgram>, expansions: u8) {
    use nettai_battle::rules::Fact;
    use nettai_content_api::{Registry, Value};
    let records: Vec<Fact> = parts
        .iter()
        .map(|p| {
            let color = program_colors(content, p.program).get(p.color as usize).copied().unwrap_or("white");
            Fact::Record(vec![
                ("program", Fact::Value(Value::Def(Registry::Entry, p.program.0))),
                ("color", Fact::Name(color)),
                ("x", Fact::Value(Value::Int(p.x as i64))),
                ("y", Fact::Value(Value::Int(p.y as i64))),
                ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
                ("compressed", Fact::Value(Value::Bool(p.compressed))),
            ])
        })
        .collect();
    let mut facts = side.facts.clone();
    let written = facts
        .set(content, "navicust_programs", &records)
        .and_then(|()| facts.set(content, "navicust_expansions", &[Fact::Value(Value::Int(expansions as i64))]));
    if written.is_ok() {
        side.facts = facts;
    }
}

/// Start a grid of programs on a side that places none yet: the largest
/// board.
fn start_grid(content: &Content, side: &mut Side, largest: u8) {
    set(content, side, Vec::new(), largest);
}

/// Whether `shape` can go down with its center on (x, y): on the board
/// (`NaviCustRules::fits`) and over no other program.
fn fits(content: &Content, board: Option<&Board>, parts: &[PlacedProgram], shape: &Shape, x: i32, y: i32) -> bool {
    let n = SIZE as i32;
    if !(0..n).contains(&x) || !(0..n).contains(&y) {
        return false;
    }
    let Some(board) = board else { return false };
    if !NaviCustRules::fits(board, shape, x as u8, y as u8) {
        return false;
    }
    let taken: std::collections::HashSet<(i32, i32)> = parts
        .iter()
        .flat_map(|p| cells(&program_of(content, p.program).placed_shape(p.compressed, p.rotation), p.x, p.y).collect::<Vec<_>>())
        .collect();
    cells(shape, x as u8, y as u8).all(|c| !taken.contains(&c))
}

/// Apply an edit to a side of a match on `arena`; whether the match
/// changed.
pub fn update(content: &Content, _arena: &Arena, side: &mut Side, state: &mut State, edit: Edit) -> bool {
    let rules = nettai_match::navicust_rules(content).clone();
    let largest = rules.boards.len().saturating_sub(1) as u8;
    match edit {
        Edit::ShowStats(on) => {
            state.show_stats = on;
            return false;
        }
        Edit::Search(t) => {
            state.search = t;
            return false;
        }
        Edit::Place(..) | Edit::Expansions(_) if navicust_of(content, side).is_none() => start_grid(content, side, largest),
        _ => {}
    }
    let had = navicust_of(content, side);
    let expansions = had.as_ref().map_or(largest, |n| n.0);
    let mut parts: Vec<PlacedProgram> = had.map(|n| n.1).unwrap_or_default();
    let mut changed = false;
    match edit {
        Edit::Expansions(x) => {
            set(content, side, parts, x);
            return true;
        }
        Edit::Hold(program, color) => {
            // The list's program held again is let go (as Tango's palette).
            if state.held.is_some_and(|h| h.origin.is_none() && h.program == program && h.color == color) {
                state.held = None;
                return false;
            }
            // One picked off the grid goes back first.
            if let Some((i, p)) = state.held.take().and_then(|h| h.origin) {
                parts.insert(i.min(parts.len()), p);
                changed = true;
            }
            // Compressed as its copies in that color are (a save keeps one
            // flag for them).
            let compressed = parts.iter().find(|p| p.program == program && p.color == color).is_some_and(|p| p.compressed);
            state.held = Some(Held { program, color, rotation: 0, compressed, grab: (0, 0), origin: None });
            if !changed {
                return false;
            }
        }
        Edit::PickUp(i, x, y) => {
            if i >= parts.len() {
                return false;
            }
            let p = parts.remove(i);
            state.held = Some(Held {
                program: p.program,
                color: p.color,
                rotation: p.rotation,
                compressed: p.compressed,
                grab: (x as i32 - p.x as i32, y as i32 - p.y as i32),
                origin: Some((i, p)),
            });
            state.selected = None;
        }
        Edit::Place(x, y) => {
            let Some(h) = state.held else { return false };
            let shape = program_of(content, h.program).placed_shape(h.compressed, h.rotation);
            if !fits(content, rules.board(expansions), &parts, &shape, x as i32, y as i32) {
                return false;
            }
            for p in parts.iter_mut().filter(|p| p.program == h.program && p.color == h.color) {
                p.compressed = h.compressed;
            }
            parts.push(PlacedProgram { program: h.program, color: h.color, x, y, rotation: h.rotation, compressed: h.compressed });
            state.selected = Some(parts.len() - 1);
            state.held = None;
        }
        Edit::PutBack => {
            let Some(h) = state.held.take() else { return false };
            let Some((i, p)) = h.origin else { return false };
            parts.insert(i.min(parts.len()), p);
            state.selected = Some(i.min(parts.len() - 1));
        }
        Edit::RotateAt(i) => {
            let Some(p) = parts.get_mut(i) else { return false };
            p.rotation = (p.rotation + 1) % 4;
            state.selected = Some(i);
        }
        Edit::Rotate => {
            if let Some(h) = state.held.as_mut() {
                // The held cell turns with the program (a quarter clockwise).
                h.rotation = (h.rotation + 1) % 4;
                h.grab = (-h.grab.1, h.grab.0);
                return false;
            }
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.rotation = (p.rotation + 1) % 4;
        }
        Edit::Compress(on) => {
            if let Some(h) = state.held.as_mut() {
                if program_of(content, h.program).compressed.is_some() {
                    h.compressed = on;
                    // Its shape is another: held by its center.
                    h.grab = (0, 0);
                }
                return false;
            }
            let Some(i) = state.selected.filter(|&i| i < parts.len()) else { return false };
            // A save compresses every copy of a program in one color.
            let (program, color) = (parts[i].program, parts[i].color);
            for p in parts.iter_mut().filter(|p| p.program == program && p.color == color) {
                p.compressed = on;
            }
        }
        Edit::Color(c) => {
            if let Some(h) = state.held.as_mut() {
                h.color = c;
                return false;
            }
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.color = c;
        }
        Edit::Move(dx, dy) => {
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.x = (p.x as i8 + dx).clamp(0, SIZE as i8 - 1) as u8;
            p.y = (p.y as i8 + dy).clamp(0, SIZE as i8 - 1) as u8;
        }
        Edit::Remove => {
            // The held program is taken off (it left the grid when picked up).
            if state.held.take().is_some() {
                return false;
            }
            let Some(i) = state.selected.take().filter(|&i| i < parts.len()) else { return false };
            parts.remove(i);
        }
        Edit::ShowStats(_) | Edit::Search(_) => unreachable!(),
    }
    if navicust_of(content, side).is_none() {
        return changed;
    }
    set(content, side, parts, expansions);
    true
}

/// A placed program as the grid draws it.
struct Part {
    cells: Vec<(i32, i32)>,
    color: Color,
    plus: bool,
    picked: bool,
}

/// The held program as the grid draws and puts it down.
struct Ghost {
    shape: Shape,
    grab: (i32, i32),
    color: Color,
    plus: bool,
    compressible: bool,
    compressed: bool,
}

/// The grid, drawn, with the mouse on it.
struct Grid<'a> {
    content: &'a Content,
    board: Option<Board>,
    command_line: u8,
    placed: Vec<PlacedProgram>,
    parts: Vec<Part>,
    /// Which program covers each cell (the last placed on top, as the grid
    /// keeps it).
    occupied: [[Option<usize>; SIZE]; SIZE],
    held: Option<Ghost>,
    side: usize,
}

/// What the grid keeps between events: the cell under the cursor, the
/// cell a press on the grid began on (a drag's start), and a trackpad's
/// scrolling toward the next turn.
#[derive(Default)]
struct Pointer {
    hovered: Option<(i32, i32)>,
    pressed: Option<(i32, i32)>,
    scrolled: f32,
}

/// A trackpad's scrolling that turns the held program once.
const SCROLL_PER_TURN: f32 = 40.0;

impl Grid<'_> {
    fn cell(p: Point) -> Option<(i32, i32)> {
        let (x, y) = ((p.x / CELL).floor() as i32, (p.y / CELL).floor() as i32);
        let n = SIZE as i32;
        ((0..n).contains(&x) && (0..n).contains(&y)).then_some((x, y))
    }

    fn at(&self, (x, y): (i32, i32)) -> Option<usize> {
        self.occupied.get(y as usize)?.get(x as usize).copied().flatten()
    }

    /// Where the held program's center goes with the held cell on `cell`,
    /// and whether it fits there.
    fn landing(&self, cell: (i32, i32)) -> Option<((i32, i32), bool)> {
        let g = self.held.as_ref()?;
        let (x, y) = (cell.0 - g.grab.0, cell.1 - g.grab.1);
        Some(((x, y), fits(self.content, self.board.as_ref(), &self.placed, &g.shape, x, y)))
    }

    fn msg(&self, edit: Edit) -> canvas::Action<Msg> {
        canvas::Action::publish(Msg::NaviCust(self.side, edit)).and_capture()
    }

    /// The held program put down with the held cell on `cell`, if it fits.
    fn place(&self, cell: (i32, i32)) -> Option<canvas::Action<Msg>> {
        match self.landing(cell)? {
            ((x, y), true) => Some(self.msg(Edit::Place(x as u8, y as u8))),
            _ => None,
        }
    }
}

fn cell_origin(x: i32, y: i32) -> Point {
    Point::new(x as f32 * CELL, y as f32 * CELL)
}

fn plus_mark(frame: &mut Frame, x: i32, y: i32, color: Color) {
    let mid = cell_origin(x, y) + Vector::new(CELL / 2.0, CELL / 2.0);
    let mark = Path::new(|p| {
        p.move_to(mid + Vector::new(-8.0, 0.0));
        p.line_to(mid + Vector::new(8.0, 0.0));
        p.move_to(mid + Vector::new(0.0, -8.0));
        p.line_to(mid + Vector::new(0.0, 8.0));
    });
    frame.stroke(&mark, Stroke::default().with_color(color).with_width(3.0));
}

fn outline(frame: &mut Frame, x: i32, y: i32, inset: f32, color: Color, width: f32) {
    let r = Path::rectangle(cell_origin(x, y) + Vector::new(inset, inset), Size::new(CELL - 2.0 * inset, CELL - 2.0 * inset));
    frame.stroke(&r, Stroke::default().with_color(color).with_width(width));
}

impl canvas::Program<Msg> for Grid<'_> {
    type State = Pointer;

    fn draw(&self, pointer: &Pointer, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(0x20, 0x28, 0x38));
        let cell = Size::new(CELL - 2.0, CELL - 2.0);
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                let kind = self.board.map_or(BoardCell::Off, |b| b[y as usize][x as usize]);
                let fill = match kind {
                    BoardCell::Off => continue,
                    BoardCell::Frame => Color::from_rgb8(0x38, 0x40, 0x58),
                    BoardCell::On if y == self.command_line as i32 => Color::from_rgb8(0x50, 0x68, 0x90),
                    BoardCell::On => Color::from_rgb8(0x5C, 0x60, 0x70),
                };
                frame.fill_rectangle(cell_origin(x, y) + Vector::new(1.0, 1.0), cell, fill);
            }
        }
        // The program under the cursor, when nothing is held: outlined, as
        // one to pick up.
        let hovered = if self.held.is_none() { pointer.hovered.and_then(|c| self.at(c)) } else { None };
        for (i, part) in self.parts.iter().enumerate() {
            for &(x, y) in &part.cells {
                frame.fill_rectangle(cell_origin(x, y) + Vector::new(4.0, 4.0), Size::new(CELL - 8.0, CELL - 8.0), part.color);
                if part.plus {
                    plus_mark(&mut frame, x, y, Color::WHITE);
                }
                if part.picked || hovered == Some(i) {
                    outline(&mut frame, x, y, 2.0, Color::WHITE, if part.picked { 3.0 } else { 2.0 });
                }
            }
        }
        // The held program where it would land: lit if it fits, red if not.
        if let (Some(g), Some(c)) = (&self.held, pointer.hovered)
            && let Some(((x, y), ok)) = self.landing(c)
        {
            let edge = if ok { Color::from_rgb8(0x60, 0xF0, 0x90) } else { Color::from_rgb8(0xF0, 0x40, 0x40) };
            let fill = Color { a: if ok { 0.8 } else { 0.45 }, ..g.color };
            let (n, mid) = (SIZE as i32, (SIZE / 2) as i32);
            // Its cells about a center that may be off the grid.
            for (cx, cy) in cells(&g.shape, mid as u8, mid as u8).map(|(cx, cy)| (cx - mid + x, cy - mid + y)) {
                if !(0..n).contains(&cx) || !(0..n).contains(&cy) {
                    continue;
                }
                frame.fill_rectangle(cell_origin(cx, cy) + Vector::new(6.0, 6.0), Size::new(CELL - 12.0, CELL - 12.0), fill);
                if g.plus {
                    plus_mark(&mut frame, cx, cy, Color { a: 0.8, ..Color::WHITE });
                }
                outline(&mut frame, cx, cy, 3.0, edge, 2.5);
            }
        }
        vec![frame.into_geometry()]
    }

    fn update(&self, pointer: &mut Pointer, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        let over = cursor.position_in(bounds).and_then(Self::cell);
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
                    // A click puts it down where it fits.
                    return self.place(cell).or(Some(canvas::Action::capture()));
                }
                let i = self.at(cell)?;
                return Some(self.msg(Edit::PickUp(i, cell.0 as u8, cell.1 as u8)));
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let pressed = pointer.pressed.take();
                self.held.as_ref()?;
                return match over {
                    // Let go over the grid after a drag (from another cell,
                    // or from the list): put down where it fits, else still
                    // held. Let go where it was pressed: a click, still held.
                    Some(cell) if pressed != Some(cell) => self.place(cell),
                    Some(_) => None,
                    // Dragged off the grid: taken off.
                    None if pressed.is_some() => Some(self.msg(Edit::Remove)),
                    None => None,
                };
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let cell = over?;
                if self.held.is_some() {
                    return Some(self.msg(Edit::Remove));
                }
                let i = self.at(cell)?;
                return Some(self.msg(Edit::RotateAt(i)));
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if self.held.is_some() && over.is_some() {
                    // A wheel's notch turns it once; a trackpad's scrolling
                    // once a stretch.
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
                    return Some(self.msg(Edit::Rotate));
                }
            }
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                let g = self.held.as_ref()?;
                let edit = match key.as_ref() {
                    keyboard::Key::Character(c) if c.eq_ignore_ascii_case("r") => Edit::Rotate,
                    keyboard::Key::Character(c) if c.eq_ignore_ascii_case("c") && g.compressible => Edit::Compress(!g.compressed),
                    keyboard::Key::Named(keyboard::key::Named::Escape) => Edit::PutBack,
                    keyboard::Key::Named(keyboard::key::Named::Delete | keyboard::key::Named::Backspace) => Edit::Remove,
                    _ => return None,
                };
                return Some(self.msg(edit));
            }
            _ => {}
        }
        None
    }

    fn mouse_interaction(&self, _: &Pointer, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        let Some(cell) = cursor.position_in(bounds).and_then(Self::cell) else { return mouse::Interaction::default() };
        if self.held.is_some() {
            // Carrying a program (Tango's closed hand).
            mouse::Interaction::Grabbing
        } else if self.at(cell).is_some() {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

pub fn view(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let state = &e.navicust[s];
    let tab = |label: &'static str, on: bool, stats: bool| {
        let b = button(text(label).size(14)).on_press(Msg::NaviCust(s, Edit::ShowStats(stats)));
        if on { b.style(button::primary) } else { b.style(button::secondary) }
    };
    let header = row![
        text(format!("{}: NaviCust", crate::editor::view::SIDES[s])).size(20),
        space().width(Length::Fill),
        tab("Grid", !state.show_stats, false),
        tab("Stats and bugs", state.show_stats, true),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if state.show_stats {
        let body: Element<Msg> = column![
            text("The stats and bugs are the NaviCust's: the programs on the grid make them as the round is set up.").size(13),
            crate::editor::view::round_stats(e, s),
        ]
        .spacing(8)
        .into();
        return column![header, body].spacing(8).into();
    }
    let rules = nettai_match::navicust_rules(c);
    let largest = rules.boards.len().saturating_sub(1) as u8;
    let had = navicust_of(c, side);
    let expansions = had.as_ref().map_or(largest, |n| n.0);
    let sizes: Vec<Choice<u8>> = (0..rules.boards.len() as u8)
        .map(|x| {
            let b = &rules.boards[x as usize];
            let (w, h) = (b.iter().map(|r| r.iter().filter(|&&c| c == BoardCell::On).count()).max().unwrap_or(0), b.iter().filter(|r| r.contains(&BoardCell::On)).count());
            Choice { label: format!("{w}x{h} ({x} expansions)"), value: x }
        })
        .collect();
    let size = sizes.iter().find(|x| x.value == expansions).cloned();
    let placed: Vec<PlacedProgram> = had.as_ref().map(|n| n.1.clone()).unwrap_or_default();
    let mut occupied = [[None; SIZE]; SIZE];
    let parts: Vec<Part> = placed
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let def = program_of(c, p.program);
            let cells: Vec<(i32, i32)> = cells(&def.placed_shape(p.compressed, p.rotation), p.x, p.y).collect();
            for &(x, y) in &cells {
                if let Some(o) = occupied.get_mut(y as usize).and_then(|r| r.get_mut(x as usize)) {
                    *o = Some(i);
                }
            }
            let name = def.colors.get(p.color as usize).cloned().unwrap_or_default();
            Part { cells, color: color(&name), plus: def.plus, picked: state.held.is_none() && state.selected == Some(i) }
        })
        .collect();
    let held = state.held.map(|h| {
        let def = program_of(c, h.program);
        Ghost {
            shape: def.placed_shape(h.compressed, h.rotation),
            grab: h.grab,
            color: color(def.colors.get(h.color as usize).map_or("", |x| x.as_str())),
            plus: def.plus,
            compressible: def.compressed.is_some(),
            compressed: h.compressed,
        }
    });
    let grid = canvas_widget(Grid { content: c, board: rules.board(expansions).copied(), command_line: rules.command_line, placed, parts, occupied, held, side: s })
        .width(Length::Fixed(CELL * SIZE as f32))
        .height(Length::Fixed(CELL * SIZE as f32));
    let colors = |program: NaviCustProgramHandle, current: u8| {
        program_of(c, program).colors.into_iter().enumerate().fold(row![].spacing(4), move |r, (k, name)| {
            let b = button(text(name).size(12)).on_press(Msg::NaviCust(s, Edit::Color(k as u8)));
            r.push(if k == current as usize { b.style(button::primary) } else { b.style(button::secondary) })
        })
    };
    let compress = |program: NaviCustProgramHandle, on: bool| -> Element<Msg> {
        if program_of(c, program).compressed.is_some() {
            iced::widget::checkbox(on).label("Compressed").on_toggle(move |b| Msg::NaviCust(s, Edit::Compress(b))).into()
        } else {
            space().into()
        }
    };
    // The held program, else the last placed one.
    let picked: Element<Msg> = if let Some(h) = state.held {
        let mut buttons = row![button("Turn").on_press(Msg::NaviCust(s, Edit::Rotate))].spacing(4);
        if h.origin.is_some() {
            buttons = buttons.push(button("Put back").on_press(Msg::NaviCust(s, Edit::PutBack)).style(button::secondary));
        }
        buttons = buttons.push(button("Take off").on_press(Msg::NaviCust(s, Edit::Remove)).style(button::danger));
        column![
            text(format!("Holding {}, turned {}", e.names.entry(c, h.program), h.rotation)).size(15),
            colors(h.program, h.color),
            row![buttons, compress(h.program, h.compressed)].spacing(12).align_y(Alignment::Center),
            text("Click a cell (or let go of a drag over one) to put it down where it shows lit. The wheel or R turns it, C compresses it; right-click, Delete or a drag off the grid takes it off; Esc puts it back.")
                .size(12),
        ]
        .spacing(6)
        .into()
    } else if let Some(p) = state.selected.and_then(|i| placed_at(c, side, i)) {
        column![
            text(format!("{} at ({}, {}), turned {}", e.names.entry(c, p.program), p.x, p.y, p.rotation)).size(15),
            colors(p.program, p.color),
            row![
                row![
                    button("←").on_press(Msg::NaviCust(s, Edit::Move(-1, 0))),
                    button("↑").on_press(Msg::NaviCust(s, Edit::Move(0, -1))),
                    button("↓").on_press(Msg::NaviCust(s, Edit::Move(0, 1))),
                    button("→").on_press(Msg::NaviCust(s, Edit::Move(1, 0))),
                    button("Turn").on_press(Msg::NaviCust(s, Edit::Rotate)),
                    button("Remove").on_press(Msg::NaviCust(s, Edit::Remove)).style(button::danger),
                ]
                .spacing(4),
                compress(p.program, p.compressed),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(6)
        .into()
    } else {
        text("Drag a color square from the list onto the grid (or click it, then a cell). Press a placed program to pick it up and drag it; right-click one to turn it.")
            .size(13)
            .into()
    };
    // The programs to pick up, searched.
    let needle = state.search.to_lowercase();
    // (The match's game's.)
    let mut programs: Vec<(String, NaviCustProgramHandle)> = c
        .defs
        .entries_of(PROGRAMS)
        .into_iter()
        .filter(|&h| nettai_match::ids::in_game(c, e.m.game(), &c.defs.entry(h).key))
        .map(|h| (e.names.entry(c, h), h))
        .filter(|(name, _)| needle.is_empty() || name.to_lowercase().contains(&needle))
        .collect();
    e.order.entries(c, PROGRAMS, &mut programs);
    let list = programs.into_iter().fold(Column::new().spacing(2), |col, (name, h)| {
        let def = program_of(c, h);
        let swatches = def.colors.iter().enumerate().fold(row![].spacing(3), |r, (k, cname)| {
            let holding = state.held.is_some_and(|x| x.origin.is_none() && x.program == h && x.color == k as u8);
            let shown = color(cname);
            let swatch = container(space().width(Length::Fixed(18.0)).height(Length::Fixed(18.0))).style(move |_: &Theme| container::Style {
                background: Some(shown.into()),
                border: iced::Border { color: if holding { Color::BLACK } else { Color::TRANSPARENT }, width: 2.0, radius: 3.0.into() },
                ..container::Style::default()
            });
            r.push(mouse_area(swatch).on_press(Msg::NaviCust(s, Edit::Hold(h, k as u8))).interaction(mouse::Interaction::Grab))
        });
        col.push(
            row![
                text(name).size(14).width(Length::Fill),
                text(if def.plus { "plus" } else { "" }).size(12).color(Color::from_rgb(0.5, 0.5, 0.55)).width(Length::Fixed(34.0)),
                container(swatches).width(Length::Fixed(80.0)),
                space().width(Length::Fixed(14.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
    });
    let note: Element<Msg> = match had {
        None => text("No NaviCust: the stats are as the setup gives them.").size(12).into(),
        Some(_) => text("The command line is the lit row.").size(12).into(),
    };
    let left = column![
        row![text("Board").size(14), pick_list(sizes, size, move |x: Choice<u8>| Msg::NaviCust(s, Edit::Expansions(x.value)))].spacing(8).align_y(Alignment::Center),
        grid,
        note,
        picked,
        crate::editor::view::round_stats(e, s),
    ]
    .spacing(8)
    .width(Length::Fixed(CELL * SIZE as f32 + 120.0));
    let right = column![
        text_input("search programs", &state.search).on_input(move |t| Msg::NaviCust(s, Edit::Search(t))),
        scrollable(list).height(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill);
    column![header, row![scrollable(left), right].spacing(16)].spacing(8).into()
}

fn placed_at(content: &Content, side: &Side, i: usize) -> Option<PlacedProgram> {
    navicust_of(content, side)?.1.get(i).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pane's edits on EXE6's content: programs are held from the list
    /// and put down
    /// where they fit (not over another), picked up by a cell and put back
    /// or down elsewhere, turned with the held cell, compressed by program
    /// and color, taken off; and the match stays one the checks accept.
    #[test]
    fn edits() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        let mut state = State::default();
        let arena = m.arena.clone();
        let side = &mut m.sides[0];
        let navicust = |side: &Side| navicust_of(&content, side).unwrap().1;
        assert_eq!(navicust_of(&content, side).unwrap().0, 2);
        let program = |name: &str| nettai_match::ids::entry(&content, "exe6", PROGRAMS, name).unwrap();
        // Held from the list: nothing changes until it is put down.
        assert!(!update(&content, &arena, side, &mut state, Edit::Hold(program("suprarmr"), 0)));
        assert!(update(&content, &arena, side, &mut state, Edit::Place(2, 3)));
        assert!(state.held.is_none());
        update(&content, &arena, side, &mut state, Edit::Hold(program("hp-50"), 1));
        // Not over SuprArmr.
        assert!(!update(&content, &arena, side, &mut state, Edit::Place(2, 3)));
        assert!(update(&content, &arena, side, &mut state, Edit::Place(4, 2)));
        update(&content, &arena, side, &mut state, Edit::Hold(program("hp-50"), 1));
        assert!(update(&content, &arena, side, &mut state, Edit::Place(5, 2)));
        let n = navicust(side);
        assert_eq!(n.len(), 3);
        assert_eq!(state.selected, Some(2));
        let problems = nettai_match::check_match(&content, &m);
        assert!(problems.is_empty(), "{problems:?}");
        let side = &mut m.sides[0];
        // Compressing one HP+50 compresses the other (one program, one color).
        update(&content, &arena, side, &mut state, Edit::Compress(true));
        let n = navicust(side);
        assert!(n.iter().filter(|p| p.program == program("hp-50")).all(|p| p.compressed));
        // Picked up by a cell it covers (off the grid while held), put back.
        let first = n[0];
        assert!(update(&content, &arena, side, &mut state, Edit::PickUp(0, first.x, first.y)));
        assert_eq!(navicust(side).len(), 2);
        assert!(update(&content, &arena, side, &mut state, Edit::PutBack));
        assert_eq!(navicust(side).first(), Some(&first));
        // Picked up, turned, put down a row lower; then taken off.
        update(&content, &arena, side, &mut state, Edit::PickUp(0, first.x, first.y));
        update(&content, &arena, side, &mut state, Edit::Rotate);
        assert!(update(&content, &arena, side, &mut state, Edit::Place(first.x, first.y + 1)));
        let moved = *navicust(side).last().unwrap();
        assert_eq!((moved.y, moved.rotation), (first.y + 1, 1));
        update(&content, &arena, side, &mut state, Edit::Remove);
        assert_eq!(navicust(side).len(), 2);
        let problems = nettai_match::check_match(&content, &m);
        assert!(problems.is_empty(), "{problems:?}");
    }
}
