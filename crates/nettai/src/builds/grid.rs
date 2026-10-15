//! The placement grid: the creator's own view of a list of pieces placed
//! on a board (EXE6's and EXE5's NaviCust), chosen by its data's names and
//! shape, never by a game's name. It is a grid where the data fit what it
//! reads, all of it as data (nothing of the content is called):
//!
//! - the setup's `navicust_programs`, a list of `{ program, color, x, y,
//!   rotation, compressed }` (`program` an entry of a collection, `color` an
//!   enum of color names), and `navicust_expansions` (the board, by its place
//!   among the boards) where the game's setup takes it: a game whose board
//!   never grows (EXE4's) has its one board;
//! - each piece's data: its `shape` (rows of `#` and `.`, centered),
//!   `compressed` (likewise, where it compresses), `colors` (names) and
//!   `plus` (a plus part's mark);
//! - the game's module `rules/navicust/board`, as it loaded
//!   (`Battle::module_data`): its `boards` (rows of `o` a cell, `f` its
//!   frame, `.` none) and `COMMAND_LINE` (a row, from 0).
//!
//! A piece fits where its cells are on the board (`o`) or its frame (`f`),
//! not all on the frame, and over no other. Where a game's data don't fit,
//! the pieces are a list like any other.

use nettai_battle::content::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Data, FieldType, Registry};
use nettai_match::Side;
use nettai_match::facts::{Stated, fact_of};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A shape: rows of cells, `true` a cell it covers, its center the middle
/// cell.
pub type Shape = Vec<Vec<bool>>;

/// The setup fields the grid edits.
pub const SIZE_FIELD: &str = "navicust_expansions";
pub const PIECES_FIELD: &str = "navicust_programs";
/// The game's module the boards are read from.
pub const MODULE: &str = "rules/navicust/board";

/// The record fields of a placed piece.
const PIECE: &str = "program";
const X: &str = "x";
const Y: &str = "y";
const ROTATION: &str = "rotation";
const COLOR: &str = "color";
const COMPRESSED: &str = "compressed";

/// The colors a piece comes in, by name, as the app draws them (docs/design/app-ui.md's palette: the text's white,
/// a Giga chip's pink, a yellow, player one's red, player two's blue, the good green).
pub const PALETTE: [(&str, u32); 6] =
    [("white", 0xEAF3FF), ("pink", 0xFF7EB6), ("yellow", 0xF2CF4A), ("red", 0xFF5D50), ("blue", 0x4A93FF), ("green", 0x45E0A0)];

/// A color by name: the palette's, else gray.
pub fn rgb(name: Option<&str>) -> u32 {
    name.and_then(|n| PALETTE.iter().find(|(k, _)| *k == n)).map_or(0x909090, |(_, c)| *c)
}

/// What the grid reads of a game: its boards (and whether a side says
/// which: `sized`), its command line, and its pieces' collection, shapes
/// (plain and compressed), colors and plus marks.
#[derive(Clone, Debug, Default)]
pub struct GridData {
    pub boards: Vec<Vec<Vec<u8>>>,
    pub sized: bool,
    pub command_line: Option<usize>,
    pub collection: String,
    pub shapes: HashMap<(u16, bool), Shape>,
    pub colors: HashMap<u16, Vec<String>>,
    pub plus: HashSet<u16>,
}


/// Rows of a shape or a board, from data of strings, each of `cells` alone.
fn rows(d: &Data, cells: &[u8]) -> Option<Vec<Vec<u8>>> {
    let Data::List(items) = d else { return None };
    let rows: Vec<Vec<u8>> = items.iter().map(|r| r.str().map(|s| s.bytes().collect())).collect::<Option<_>>()?;
    let n = rows.first()?.len();
    rows.iter().all(|r| r.len() == n && r.iter().all(|b| cells.contains(b))).then_some(rows)
}

fn shape(rows: &[Vec<u8>]) -> Shape {
    rows.iter().map(|r| r.iter().map(|&b| b == b'#').collect()).collect()
}

/// The element type of a list, else the type.
fn element(ty: &FieldType) -> &FieldType {
    match ty {
        FieldType::List(e, _) | FieldType::Array(e, _) => e,
        t => t,
    }
}

/// What the grid reads of the content (`module`: its board module, as
/// data), or none where its data don't fit.
pub fn read(content: &Arc<Content>, module: Option<&Data>) -> Option<GridData> {
    let field = |name: &str| nettai_match::facts::field(content, name).map(|f| f.ty.clone());
    let FieldType::Record(fields) = element(&field(PIECES_FIELD)?).clone() else { return None };
    let ty = |name: &str| fields.index_of(name).map(|i| fields.field(i).ty.clone());
    let Some(FieldType::Ref(Registry::Entry, Some(collection))) = ty(PIECE) else { return None };
    let number = |name: &str| ty(name).is_some_and(|t| nettai_match::facts::range(&t).is_some());
    if !(number(X) && number(Y) && number(ROTATION) && matches!(ty(COLOR), Some(FieldType::Enum(_))) && ty(COMPRESSED) == Some(FieldType::Bool)) {
        return None;
    }
    let sized = field(SIZE_FIELD).is_some();
    let module = module?;
    let Data::List(boards) = module.field("boards") else { return None };
    let boards: Vec<Vec<Vec<u8>>> = boards.iter().map(|b| rows(b, b"of.")).collect::<Option<_>>()?;
    if boards.is_empty() {
        return None;
    }
    let mut data = GridData { boards, sized, command_line: module.field("COMMAND_LINE").int().map(|r| r as usize), collection: collection.clone(), ..GridData::default() };
    for h in content.defs.entries_of(&collection) {
        let d = content.defs.definitions.get(Registry::Entry, &content.defs.entry(h).key)?;
        let plain = rows(d.spec.field("shape"), b"#.")?;
        let compressed = match d.spec.field("compressed") {
            Data::Nil => plain.clone(),
            c => rows(c, b"#.")?,
        };
        data.shapes.insert((h.0, false), shape(&plain));
        data.shapes.insert((h.0, true), shape(&compressed));
        let Data::List(colors) = d.spec.field("colors") else { return None };
        data.colors.insert(h.0, colors.iter().map(|c| c.str().map(str::to_string)).collect::<Option<_>>()?);
        if d.spec.field("plus") == &Data::Bool(true) {
            data.plus.insert(h.0);
        }
    }
    Some(data)
}

/// A board's size as it shows: its cells' columns by rows (`5x4`).
pub fn size(board: &[Vec<u8>]) -> String {
    let rows = board.iter().filter(|r| r.contains(&b'o')).count();
    let columns = (0..board.first().map_or(0, Vec::len)).filter(|&x| board.iter().any(|r| r[x] == b'o')).count();
    format!("{columns}x{rows}")
}

/// A piece placed: which, its center, its turns, its color (by name),
/// whether it is compressed; and its record as read, for the fields the
/// grid doesn't name.
#[derive(Clone, Debug)]
pub struct Placed {
    pub piece: u16,
    pub x: i32,
    pub y: i32,
    pub rotation: u8,
    pub color: Option<String>,
    pub compressed: bool,
    pub record: Stated,
}

/// A record read as a placed piece.
fn placed_of(record: &Stated) -> Option<Placed> {
    let Stated::Record(fields) = record else { return None };
    let get = |n: &str| fields.iter().find(|(k, _)| k == n).map(|(_, v)| v);
    let int = |n: &str| match get(n) {
        Some(Stated::Number(v)) => *v,
        _ => 0,
    };
    let Some(Stated::Def(_, Some(piece))) = get(PIECE) else { return None };
    Some(Placed {
        piece: *piece,
        x: int(X) as i32,
        y: int(Y) as i32,
        rotation: int(ROTATION) as u8,
        color: get(COLOR).and_then(|v| if let Stated::Variant(n) = v { n.clone() } else { None }),
        compressed: matches!(get(COMPRESSED), Some(Stated::Flag(true))),
        record: record.clone(),
    })
}

/// A placed piece's record: its read record with the grid's fields set.
fn record_of(p: &Placed) -> Stated {
    let mut fields = match &p.record {
        Stated::Record(f) => f.clone(),
        _ => Vec::new(),
    };
    let mut set = |name: &str, v: Stated| match fields.iter_mut().find(|(k, _)| k == name) {
        Some((_, x)) => *x = v,
        None => fields.push((name.to_string(), v)),
    };
    set(PIECE, Stated::Def(Registry::Entry, Some(p.piece)));
    set(X, Stated::Number(p.x as i64));
    set(Y, Stated::Number(p.y as i64));
    set(ROTATION, Stated::Number(p.rotation as i64));
    set(COLOR, Stated::Variant(p.color.clone()));
    set(COMPRESSED, Stated::Flag(p.compressed));
    Stated::Record(fields)
}

/// The pieces the side's list holds.
pub fn placed(c: &Content, side: &Side) -> Vec<Placed> {
    let Some(Stated::List(items)) = side.facts.get(c, PIECES_FIELD) else { return Vec::new() };
    items.iter().filter_map(placed_of).collect()
}

/// The side's board, by its place among the boards (none: no board).
pub fn board_index(c: &Content, side: &Side) -> Option<usize> {
    match side.facts.get(c, SIZE_FIELD) {
        Some(Stated::Optional(Some(n)) | Stated::Number(n)) => Some(n as usize),
        _ => None,
    }
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

impl GridData {
    /// The board of `side`, by its place among the boards: the one its
    /// setup says, else the last (the full board, every build's); a game
    /// whose board never grows has its one.
    pub fn board_of(&self, c: &Content, side: &Side) -> Option<usize> {
        let last = self.boards.len().checked_sub(1)?;
        Some(if self.sized { board_index(c, side).filter(|&b| b <= last).unwrap_or(last) } else { last })
    }

    /// A piece's shape as placed: compressed or not, turned.
    pub fn shape(&self, piece: u16, compressed: bool, rotation: u8) -> Shape {
        self.shapes.get(&(piece, compressed)).map_or_else(Vec::new, |s| rotate(s, rotation))
    }

    /// Whether the piece compresses into another shape.
    pub fn compresses(&self, piece: u16) -> bool {
        self.shapes.get(&(piece, false)) != self.shapes.get(&(piece, true))
    }

    /// The cells of the board each placed piece covers, by its place in
    /// the list.
    pub fn occupied(&self, parts: &[Placed]) -> HashMap<(i32, i32), usize> {
        let mut out = HashMap::new();
        for (i, p) in parts.iter().enumerate() {
            for cell in cells(&self.shape(p.piece, p.compressed, p.rotation), p.x, p.y) {
                out.insert(cell, i);
            }
        }
        out
    }

    /// Whether `shape` fits on `board` with its center on (x, y): its cells
    /// on the board or its frame, not all on the frame, over none of
    /// `others`'.
    pub fn fits(&self, board: &[Vec<u8>], others: &[Placed], shape: &Shape, x: i32, y: i32) -> bool {
        self.misfit(board, others, shape, x, y).is_none()
    }

    /// Why `shape` doesn't fit on `board` with its center on (x, y), as the
    /// rules type it of a build: a cell off the board (or every cell on its
    /// frame), or the piece of `others` it would cover. None: it fits.
    pub fn misfit(&self, board: &[Vec<u8>], others: &[Placed], shape: &Shape, x: i32, y: i32) -> Option<nettai_match::Wrong> {
        let cs = cells(shape, x, y);
        let at = |(cx, cy): (i32, i32)| board.get(cy as usize).and_then(|r| r.get(cx as usize)).copied().filter(|_| cx >= 0 && cy >= 0);
        if cs.is_empty() || cs.iter().any(|&c| !matches!(at(c), Some(b'o' | b'f'))) || cs.iter().all(|&c| at(c) == Some(b'f')) {
            return Some(nettai_match::Wrong::OffBoard);
        }
        let taken = self.occupied(others);
        cs.iter().find_map(|c| taken.get(c)).map(|&other| nettai_match::Wrong::Overlap { other })
    }
}

/// A piece in hand: which, its color's place among its colors, its turns,
/// compressed or not, the cell of it the cursor holds (from its center),
/// and the record it was on the board if it was picked off it (its place).
#[derive(Clone, Debug)]
pub struct Held {
    pub piece: u16,
    pub color: u8,
    pub rotation: u8,
    pub compressed: bool,
    pub grab: (i32, i32),
    pub origin: Option<(usize, Stated)>,
}

#[derive(Clone, Debug)]
pub enum Edit {
    /// A piece taken from the list, in one of its colors, compressed or not.
    Hold(u16, u8, bool),
    /// The placed piece at this place in the list, picked up by this cell.
    PickUp(usize, i32, i32),
    /// The piece in hand put down with its center on this cell.
    Place(i32, i32),
    /// The piece in hand back where it was (taken off, if from the list).
    PutBack,
    /// The piece in hand turned a quarter, clockwise or back.
    Turn(bool),
    /// The piece in hand compressed, or not.
    Compress,
    /// The placed piece at this place turned where it is.
    TurnAt(usize),
    /// The piece in hand dropped; the placed one at this place taken off.
    Drop,
    Remove(usize),
    /// The board, by its place among the boards.
    Board(usize),
}

/// Apply `edit` to the side's pieces, `held` the piece in hand: whether
/// the side changed.
pub fn edit(d: &GridData, c: &Content, side: &mut Side, held: &mut Option<Held>, edit: Edit) -> bool {
    let mut parts = placed(c, side);
    let colors = |h: u16| d.colors.get(&h).cloned().unwrap_or_default();
    match edit {
        Edit::Hold(piece, color, compressed) => {
            let back = held.take().and_then(|h| h.origin).and_then(|(i, record)| Some((i, placed_of(&record)?)));
            *held = Some(Held { piece, color, rotation: 0, compressed, grab: (0, 0), origin: None });
            match back {
                Some((i, p)) => parts.insert(i.min(parts.len()), p),
                None => return false,
            }
        }
        Edit::PickUp(i, x, y) => {
            if i >= parts.len() {
                return false;
            }
            let p = parts.remove(i);
            let color = p.color.as_ref().and_then(|n| colors(p.piece).iter().position(|x| x == n)).unwrap_or(0) as u8;
            *held = Some(Held { piece: p.piece, color, rotation: p.rotation, compressed: p.compressed, grab: (x - p.x, y - p.y), origin: Some((i, record_of(&p))) });
        }
        Edit::Place(x, y) => {
            let Some(h) = held.clone() else { return false };
            let Some(board) = d.board_of(c, side).and_then(|b| d.boards.get(b)) else { return false };
            let shape = d.shape(h.piece, h.compressed, h.rotation);
            if !d.fits(board, &parts, &shape, x, y) {
                return false;
            }
            let record = h.origin.as_ref().map_or(Stated::Record(Vec::new()), |(_, r)| r.clone());
            let color = colors(h.piece).get(h.color as usize).cloned();
            let p = Placed { piece: h.piece, x, y, rotation: h.rotation, color, compressed: h.compressed, record };
            match h.origin {
                Some((i, _)) => parts.insert(i.min(parts.len()), p),
                None => parts.push(p),
            }
            *held = None;
        }
        Edit::PutBack => {
            let Some(h) = held.take() else { return false };
            let Some((i, p)) = h.origin.and_then(|(i, record)| Some((i, placed_of(&record)?))) else { return false };
            parts.insert(i.min(parts.len()), p);
        }
        Edit::Turn(clockwise) => {
            if let Some(h) = held.as_mut() {
                // (The cell held turns with the piece.)
                if clockwise {
                    h.rotation = (h.rotation + 1) % 4;
                    h.grab = (-h.grab.1, h.grab.0);
                } else {
                    h.rotation = (h.rotation + 3) % 4;
                    h.grab = (h.grab.1, -h.grab.0);
                }
            }
            return false;
        }
        Edit::Compress => {
            if let Some(h) = held.as_mut().filter(|h| d.compresses(h.piece)) {
                h.compressed = !h.compressed;
                // (The other shape, turned as the piece is, about the same center: held by the same cell where it
                // still has it, else by its nearest.)
                let others = cells(&d.shape(h.piece, h.compressed, h.rotation), 0, 0);
                if !others.contains(&h.grab) {
                    let far = |&(x, y): &(i32, i32)| (x - h.grab.0).abs() + (y - h.grab.1).abs();
                    h.grab = others.iter().min_by_key(|c| far(c)).copied().unwrap_or((0, 0));
                }
            }
            return false;
        }
        Edit::TurnAt(i) => {
            let Some(p) = parts.get_mut(i) else { return false };
            p.rotation = (p.rotation + 1) % 4;
        }
        Edit::Drop => {
            *held = None;
            return false;
        }
        Edit::Remove(i) => {
            if i >= parts.len() {
                return false;
            }
            parts.remove(i);
        }
        Edit::Board(b) => {
            if !d.sized || b >= d.boards.len() || board_index(c, side) == Some(b) {
                return false;
            }
            return side.set_fact(c, SIZE_FIELD, &[Fact::Value(nettai_content_api::Value::Int(b as i64))]).is_ok();
        }
    }
    set_placed(c, side, &parts)
}

/// Make `parts` the side's list of pieces, as they are (nothing checks where they are: the rules say what is wrong
/// with them). Whether it took them.
pub fn set_placed(c: &Content, side: &mut Side, parts: &[Placed]) -> bool {
    let list = Stated::List(parts.iter().map(record_of).collect());
    let values = match fact_of(&list) {
        Fact::List(items) => items,
        f => vec![f],
    };
    side.set_fact(c, PIECES_FIELD, &values).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn read_game(content: &Arc<Content>, game: &str) -> Option<GridData> {
        read(content, crate::builds::module_data(content, game, MODULE).as_ref())
    }

    /// The three games' grids read: EXE6's and EXE5's three boards, EXE4's
    /// one, the command line row 3, every piece's shapes and colors.
    #[test]
    fn the_games_grids_read() {
        for (content, game, sizes) in [
            (nettai_match::testing::exe6_content(), "exe6", &["4x4", "5x4", "5x5"][..]),
            (nettai_match::testing::exe5_content(), "exe5", &["4x4", "5x4", "5x5"][..]),
            (nettai_match::testing::exe4_content(), "exe4", &["5x5"][..]),
        ] {
            let data = read_game(&content, game).unwrap_or_else(|| panic!("{game}: the data don't fit the grid"));
            assert_eq!((data.boards.len(), data.command_line, data.sized), (sizes.len(), Some(3), game != "exe4"), "{game}");
            let pieces = content.defs.entries_of(&data.collection).len();
            assert_eq!((data.colors.len(), data.shapes.len()), (pieces, 2 * pieces), "{game}");
            let read: Vec<String> = data.boards.iter().map(|b| size(b)).collect();
            assert_eq!(read, sizes, "{game}");
        }
    }

    /// Pieces taken from the list are put down where they fit (not over
    /// another), picked up by a cell and put back, turned, taken off; the
    /// match stays one the checks accept.
    #[test]
    fn grid_edits() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        nettai_match::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let data = read_game(&content, "exe6").unwrap();
        let mut held = None;
        let piece = |name: &str| nettai_match::ids::entry(&content, "exe6", &data.collection, name).unwrap().0;
        let side = &mut m.sides[0];
        let go = |side: &mut Side, held: &mut Option<Held>, e: Edit| edit(&data, &content, side, held, e);
        assert!(!go(side, &mut held, Edit::Hold(piece("suprarmr"), 0, false)));
        assert!(go(side, &mut held, Edit::Place(2, 3)));
        assert!(held.is_none());
        go(side, &mut held, Edit::Hold(piece("hp-50"), 1, false));
        assert!(!go(side, &mut held, Edit::Place(2, 3)), "not over SuprArmr");
        assert!(go(side, &mut held, Edit::Place(4, 2)));
        assert_eq!(placed(&content, side).len(), 2);
        let first = placed(&content, side)[0].clone();
        assert!(go(side, &mut held, Edit::PickUp(0, 2, 3)));
        assert_eq!(placed(&content, side).len(), 1);
        assert!(go(side, &mut held, Edit::PutBack));
        let back = placed(&content, side);
        assert_eq!((back[0].piece, back[0].x, back[0].y), (first.piece, first.x, first.y));
        assert!(go(side, &mut held, Edit::TurnAt(1)));
        assert_eq!(placed(&content, side)[1].rotation, 1);
        assert!(go(side, &mut held, Edit::Remove(1)));
        assert_eq!(placed(&content, side).len(), 1);
        // A piece picked off the board and put down again keeps its place
        // in the list.
        go(side, &mut held, Edit::Hold(piece("hp-50"), 0, false));
        assert!(go(side, &mut held, Edit::Place(4, 2)));
        assert!(go(side, &mut held, Edit::PickUp(0, 2, 3)));
        assert!(go(side, &mut held, Edit::Place(2, 2)));
        assert_eq!(placed(&content, side)[0].piece, piece("suprarmr"));
        assert!(go(side, &mut held, Edit::Board(1)) && !go(side, &mut held, Edit::Board(1)));
        assert_eq!(board_index(&content, side), Some(1));
        assert!(go(side, &mut held, Edit::Board(2)));
        assert_eq!(nettai_match::check_match(&content, &m), Vec::<String>::new());
    }

    /// A program is placed as many times as it is taken from the list: a second copy in the same color (and one in
    /// another) goes down beside the first, and each game's rules accept the copies (they limit none; a save
    /// compresses every copy of a program in one color alike, which `Navicust::take` keeps).
    #[test]
    fn a_program_is_placed_more_than_once() {
        for (content, game, version) in [
            (nettai_match::testing::exe6_content(), "exe6", "falzar"),
            (nettai_match::testing::exe5_content(), "exe5", "protoman"),
            (nettai_match::testing::exe4_content(), "exe4", "redsun"),
        ] {
            let builds = nettai_match::build::Builds::of(&content, game).unwrap();
            let mut side = builds.new_build(&content, version).unwrap();
            let data = read_game(&content, game).unwrap();
            let piece = nettai_match::ids::entry(&content, game, &data.collection, "hp-50").unwrap_or_else(|| panic!("{game}: no HP+50")).0;
            let board = data.board_of(&content, &side).map(|b| data.boards[b].clone()).unwrap();
            let shape = data.shape(piece, false, 0);
            let mut held = None;
            for i in 0..3 {
                // (The first two in the program's first color, the third in its last; each where it first fits
                // wholly on the board, reading from the top.)
                let color = if i < 2 { 0 } else { data.colors[&piece].len() as u8 - 1 };
                let parts = placed(&content, &side);
                let own = |&(x, y): &(i32, i32)| cells(&shape, x, y).iter().all(|&(cx, cy)| board.get(cy as usize).and_then(|r| r.get(cx as usize)) == Some(&b'o'));
                let (x, y) = (0..7).flat_map(|y| (0..7).map(move |x| (x, y))).find(|at| own(at) && data.fits(&board, &parts, &shape, at.0, at.1)).unwrap();
                edit(&data, &content, &mut side, &mut held, Edit::Hold(piece, color, false));
                assert!(edit(&data, &content, &mut side, &mut held, Edit::Place(x, y)), "{game}: copy {} at ({x}, {y})", i + 1);
            }
            let parts = placed(&content, &side);
            assert_eq!(parts.iter().filter(|p| p.piece == piece).count(), 3, "{game}");
            assert_eq!(parts[0].color, parts[1].color, "{game}: two copies in one color");
            let problems = nettai_match::build::Checked::of(&content, game, &side).problems;
            assert!(problems.is_empty(), "{game}: {problems:?}");
        }
    }

    /// Compressing a piece in hand keeps its turns, and the cell it is held by where its other shape still has it
    /// (else the nearest one it has).
    #[test]
    fn compressing_keeps_the_turn_and_the_cell_held() {
        let content = nettai_match::testing::exe6_content();
        let mut m = nettai_match::pick::live(&content, "exe6", 7, None).unwrap();
        nettai_match::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let data = read_game(&content, "exe6").unwrap();
        let armor = nettai_match::ids::entry(&content, "exe6", &data.collection, "suprarmr").unwrap().0;
        let side = &mut m.sides[0];
        let mut held = Some(Held { piece: armor, color: 0, rotation: 1, compressed: false, grab: (0, 0), origin: None });
        edit(&data, &content, side, &mut held, Edit::Compress);
        let h = held.clone().unwrap();
        assert_eq!((h.compressed, h.rotation, h.grab), (true, 1, (0, 0)), "the middle, which both shapes have");
        // (SuprArmr's plain shape has a column its compressed one hasn't: turned once, its left one.)
        let plain = cells(&data.shape(armor, false, 1), 0, 0);
        let squeezed = cells(&data.shape(armor, true, 1), 0, 0);
        let gone = *plain.iter().find(|c| !squeezed.contains(c)).unwrap();
        held = Some(Held { compressed: false, grab: gone, ..h });
        edit(&data, &content, side, &mut held, Edit::Compress);
        let h = held.unwrap();
        assert_eq!(h.rotation, 1);
        assert!(squeezed.contains(&h.grab) && (h.grab.0 - gone.0).abs() + (h.grab.1 - gone.1).abs() == 1, "{gone:?} to {:?}", h.grab);
    }

    #[test]
    fn shapes_turn() {
        let up: Shape = vec![vec![false, true, false], vec![false, true, false], vec![false, false, false]];
        let right: Shape = vec![vec![false, false, false], vec![false, true, true], vec![false, false, false]];
        assert_eq!(rotate(&up, 1), right);
        assert_eq!(rotate(&up, 4), up);
        assert_eq!(cells(&right, 5, 5), [(5, 5), (6, 5)]);
    }
}
