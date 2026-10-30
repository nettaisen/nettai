//! The NaviCust tab of a build's editor (docs/design/app-ui.md §6, "NaviCust"; §2 item 10).
//!
//! What the board is, what a program's shape is, where a piece fits and how it turns and compresses is the game's
//! data, read by `crate::builds::grid` (the boards and the command line from its rules/navicust/board module, each
//! program's shape, colors and plus mark from its definition). What the NaviCust comes to is the game's rules'
//! (nettai-match's `Checked::overview`): what it does and the bugs it brings are the build's overview without its
//! patch cards (`own_overview`; a card's bugs are the Patch cards tab's and the build overview's: the user,
//! "navicust shouldn't display patch card bugs").
//!
//! **The list has each program once** (the user: "deduppe navicust program parts in the list and then have you select
//! by color"): a program's colors are its definition's (`GridData::colors`), and its row offers them where there is more
//! than one. Left and Right on the row (the pad's too) choose the color whose part Enter takes (past the first or the
//! last, the focus moves on as ever); a click or a tap on one of them chooses it and takes its part, as Enter would (the
//! user: "selecting the color in the ncp editor should also pick up the ncp"). It starts at the color of the copy the
//! build has placed first, else the first. What the row says (placed, how many copies, out of bounds) is that color's part's;
//! the strip shows that part.
//!
//! **The cursor goes over the board cell by cell.** Enter on a placed piece picks it up by that cell; Enter on a
//! program of the list takes a new copy of it in hand, its middle on the board's, however many copies are placed
//! already (no game's rules limit them: the user, "you should allow a navicust part to be placed multiple times").
//! While a piece is in hand it is where the cursor is on the board: the arrows move it (the cursor goes to the next
//! cell), the keys bound to L and R (A and S) and the pad's shoulders turn it about the cell held, C (the pad's
//! north button) compresses it in place (its other shape turned as it is, about the same middle, held by the same
//! cell where it still has it), Confirm (Enter, A) puts it down where it fits (else it shakes and the strip says why
//! at its top right, typed as the rules say it, as every tab's strip says a refusal; a phone's line under the key),
//! and Back (Esc, B) puts it away into the list, as the game's B does: so a placed program is taken off by picking it
//! up and putting it away. Del takes the piece under the cursor off at
//! once, or a program's last copy from the list, or puts the piece in hand away. The strip across the top describes
//! the program under the cursor: its shape and, where it compresses into another, its compressed shape, its name in
//! its color, the game's words for it and the colors it comes in; while the pointer is on a row of the list (and no
//! piece is in hand), that row's (docs/design/app-ui.md §2.64). The piece in hand is outlined white where it could be
//! put down, red where it can't. The board's pane's header has the color bar after its title, as the game's NaviCust
//! screen has it over its board (docs/design/app-ui.md §2.76): its slots (the game's rules/navicust/board's
//! `COLOR_BAR`, the colors placed without a bug) each a color placed, in the order placed, or empty, then each color
//! past them spilling out of its frame (the color bug, which the bugs list names). **CLEAR** at the header's right
//! end (where the game draws no board, the programs' title's line's; or **X**, with no piece in hand;
//! docs/design/app-ui.md §2.71, §2.72) takes every placed program off the board, as one change, asked first
//! (`editor::ask_clear`).
//!
//! **The mouse carries a piece by clicks** (the view's pointer areas, told here as `Tab::choose`'s "drag"; the user:
//! "dragging navicust piece is confusing because usually it's click to drop"). A click on a placed piece picks it up,
//! one on a program's row takes a new copy of it; the piece in hand follows the pointer over the board with no button
//! held, cell by cell (off the board it stays at the last cell it was over), and the wheel turns it. The next click on
//! the board puts it down (or it shakes and stays in hand); a click on the list puts it away into it ("drag it back to
//! where it came from"), the list showing so while the pointer is over it (`over-list`); a click anywhere else off
//! the board puts it back where it was (a new copy, which had no place, goes away). Each click acts as the button goes
//! down: a press that moves before it is let go is the same click, and nothing follows the held button (let go, the
//! piece comes under the pointer). A right click puts the piece in hand away, however it was taken, or takes off the
//! piece under the pointer. A key moves the piece in hand from where it is, and the pointer's next move takes it back
//! under the pointer.

use crate::builds::grid::{self, Edit, GridData, Held, tone};
use crate::builds::overview;
use crate::input::{Action, KeyPress};
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use nettai_battle::Content;
use nettai_battle::input::keys;
use nettai_content_api::{EntryHandle, Registry};
use nettai_match::Side;
use nettai_match::build::{Checked, Overview, Stat};
use nettai_match::ruleset::Item;
use nettai_ui::{BuildWords, CellKind, Crossing, ColorSlot, EditorTab, EffectKind, EffectLine, GridCell, NavicustView, PieceCell, ProgramColor, ProgramDetail, ProgramRow, ProgramState};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::sync::Arc;

/// The lines each of the effects' and the bugs' panels has room for under its title.
const LINES: usize = 10;

/// What the view's pointer areas tell (`Nav.choose("drag", phase, value)`): the left button down on a row of the list
/// (`value` its place) or on the board, let go (`value` where the pointer is: x * 65536 + y, the window's logical
/// pixels), the wheel (`value` its delta); with a piece in hand, the pointer moved with no button held (`value`
/// where), a right click (on the board, `value` where), and the left button down off the board and the list's rows
/// (`value` where).
const PRESS_ROW: i32 = 0;
const PRESS_BOARD: i32 = 1;
const RELEASE: i32 = 3;
const WHEEL: i32 = 4;
const HOVER: i32 = 7;
const RIGHT_CLICK: i32 = 8;
const PRESS_OFF: i32 = 9;

/// How far the wheel goes for a quarter turn (a mouse's notch is more).
const NOTCH: i32 = 40;

/// The most letters the programs' search has.
const SEARCH_LENGTH: usize = 24;

#[derive(Default)]
pub struct Navicust {
    /// The game's grid, read once (`Some(None)`: its data draw no board).
    data: Option<Option<GridData>>,
    /// The drawn grid: its first row and column on the engine's grid, and how many cells a side.
    first: i32,
    columns: i32,
    /// The game's programs, each once, in its library's order; the list, those of them whose names hold the search;
    /// and the color chosen for each, by its place among the program's (one not chosen yet: `color`).
    programs: Vec<u16>,
    search: String,
    rows: Vec<u16>,
    chosen: HashMap<u16, u8>,
    /// The piece in hand, and the cell of the grid the cursor holds it by.
    held: Option<Held>,
    at: (i32, i32),
    /// The cursor is in the list.
    in_list: bool,
    /// The list's row the pointer is on, which the strip describes in place of what the cursor is on
    /// (`editor::hover`).
    hovered: Option<usize>,
    /// The motion's counts: pieces taken up, quarter turns and the last one's way, pieces that couldn't be put
    /// down.
    lifted: i32,
    turned: i32,
    turn_way: i32,
    refused: i32,
    /// The piece in hand was moved last (it glides to the cell), not taken up (it is there at once).
    glide: bool,
    /// Why the last piece couldn't be put down, until it moves.
    notice: Option<String>,
    /// The bugs shown last (one that wasn't there pulses).
    bugs: Option<Vec<String>>,
    /// The wheel's way toward the next quarter turn.
    wheel: i32,
    /// The pointer is over the list with a piece in hand: a click there puts it away.
    over_list: bool,
}

/// What `board` has at a place of the engine's grid: `o` a cell, `f` its frame, `.` none.
fn kind_at(board: &[Vec<u8>], x: i32, y: i32) -> u8 {
    if x < 0 || y < 0 {
        return b'.';
    }
    board.get(y as usize).and_then(|r| r.get(x as usize)).copied().unwrap_or(b'.')
}

/// The part of the engine's grid drawn: its first row and column, and how many a side, the square any of the game's
/// boards has a cell in (EXE6's 7x7; EXE5's and EXE4's 5x5, in its middle).
fn drawn(d: &GridData) -> (i32, i32) {
    let (mut lo, mut hi) = (i32::MAX, i32::MIN);
    for board in &d.boards {
        for (y, row) in board.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                if c != b'.' {
                    lo = lo.min(x.min(y) as i32);
                    hi = hi.max(x.max(y) as i32);
                }
            }
        }
    }
    if lo <= hi { (lo, hi - lo + 1) } else { (0, 0) }
}

/// The overview's numbers, in the order of the stats' words (`NavicustView.stat`).
fn stats(o: &Overview) -> [Stat; 8] {
    [o.hp, o.attack, o.rapid, o.charge, o.custom, o.mega, o.giga, o.regular_memory]
}

/// What the NaviCust comes to: the overview of `side` (a build of `game`) without its patch cards, so the numbers
/// the NaviCust changes, the abilities its programs give and the bugs it brings, not a card's (the user: "navicust
/// shouldn't display patch card bugs"). `checked` is the whole build's, which is its own where the build has no
/// cards (or its game none). None: the round doesn't start.
fn own_overview(content: &Arc<Content>, game: &str, side: &Side, checked: &mut Checked) -> Option<Overview> {
    let bare = side.without_patch_cards(content);
    if bare == *side { checked.overview(content, side) } else { Checked::of(content, game, &bare).overview(content, &bare) }
}

/// The EFFECTS and BUGS panels' lines (NaviCust's, and Patch cards' of the cards: `effect_panels`), each under its
/// title, and on a phone a line each (their items, which the line lists: `I18n.join`).
pub(super) struct Panels {
    pub effects: Vec<EffectLine>,
    pub bugs: Vec<EffectLine>,
    pub effect_items: ModelRc<SharedString>,
    pub bug_items: ModelRc<SharedString>,
}

/// What overview `alone` comes to, as the EFFECTS and BUGS panels say it: each number it changes, the rules' own
/// stats among them ("Soul time +1"), what gives an ability, each bug by its full name and its level or amount. A bug
/// not among `was` (the bugs said last, which this sets) pulses.
pub(super) fn effect_panels(shell: &Shell, draft: &Draft, alone: Option<&Overview>, was: &mut Option<Vec<String>>) -> Panels {
    let ui = shell.ui();
    let view = ui.global::<NavicustView>();
    let say = ui.global::<BuildWords>();
    let (effects, bugs) = draft.with_words(shell, |words| {
        let mut effects: Vec<String> = alone
            .iter()
            .flat_map(|o| stats(o).into_iter().enumerate())
            .filter(|(_, s)| s.added() != 0)
            .map(|(i, s)| {
                // (The Regular memory's is in MB.)
                let n = s.added();
                let by = if i == 7 { format!("{}{}", if n > 0 { "+" } else { "" }, say.invoke_megabytes(n)) } else { format!("{n:+}") };
                view.invoke_stat(i as i32, by.into()).to_string()
            })
            .collect();
        effects.extend(alone.iter().flat_map(|o| o.stats.iter()).filter(|s| s.stat.added() != 0).map(|s| format!("{} {:+}", overview::stat_name(words, s), s.stat.added())));
        effects.extend(alone.iter().flat_map(|o| o.abilities.iter()).map(|&s| overview::source(words, s)));
        let bugs: Vec<(String, String)> = alone.iter().flat_map(|o| o.bugs.iter()).map(|b| overview::bug(words, b)).collect();
        (effects, bugs)
    });
    let said = |(n, e): &(String, String)| if e.is_empty() { n.clone() } else { format!("{n} {e}") };
    let before = was.replace(bugs.iter().map(said).collect());
    let effect_lines: Vec<EffectLine> = effects.iter().map(|e| EffectLine { kind: EffectKind::Effect, text: e.into(), ..Default::default() }).collect();
    let bug_lines: Vec<EffectLine> = bugs
        .iter()
        .map(|b| EffectLine { kind: EffectKind::Bug, text: b.0.as_str().into(), extent: b.1.as_str().into(), fresh: before.as_ref().is_some_and(|was| !was.contains(&said(b))) })
        .collect();
    let more = |n: i32| say.invoke_more(n);
    Panels {
        effects: fitted(EffectKind::Effects, effect_lines, more),
        bugs: fitted(EffectKind::Bugs, bug_lines, more),
        effect_items: overview::items(effects),
        bug_items: overview::items(bugs.iter().map(said)),
    }
}

/// A color by its name, as the app draws a program in it.
/// A shape's cells' box, row by row, and how many cells a row (none: a shape with no cells).
fn boxed(shape: &grid::Shape) -> (Vec<bool>, i32) {
    let cells: Vec<(usize, usize)> = shape.iter().enumerate().flat_map(|(y, r)| r.iter().enumerate().filter(|(_, c)| **c).map(move |(x, _)| (x, y))).collect();
    let (Some(left), Some(right)) = (cells.iter().map(|c| c.0).min(), cells.iter().map(|c| c.0).max()) else { return (Vec::new(), 0) };
    let (top, bottom) = (cells.iter().map(|c| c.1).min().unwrap_or(0), cells.iter().map(|c| c.1).max().unwrap_or(0));
    ((top..=bottom).flat_map(|y| (left..=right).map(move |x| shape[y][x])).collect(), (right - left + 1) as i32)
}

/// A piece's cells, each from (dx, dy), with the sides of it that are the piece's edge and whether the crossing at
/// its lower right corner is within it.
fn piece(cells: &[(i32, i32)], (dx, dy): (i32, i32)) -> Vec<PieceCell> {
    let has = |x: i32, y: i32| cells.contains(&(x, y));
    cells
        .iter()
        .map(|&(x, y)| PieceCell {
            x: x - dx,
            y: y - dy,
            top: !has(x, y - 1),
            right: !has(x + 1, y),
            bottom: !has(x, y + 1),
            left: !has(x - 1, y),
            corner: has(x + 1, y) && has(x, y + 1) && has(x + 1, y + 1),
        })
        .collect()
}

/// The color bar's boxes: `slots` of them, each one of `colors` (the colors placed, in the order placed) or empty, then
/// one for each color past them, which spills out of the bar (the color bug's).
fn color_bar(colors: &[String], slots: usize) -> Vec<ColorSlot> {
    (0..slots.max(colors.len())).map(|i| ColorSlot { tone: colors.get(i).map_or_else(Default::default, |c| tone(Some(c))), filled: i < colors.len() }).collect()
}

/// `items` under their title, in the panel's room: all of them, or as many as fit less one with how many more there
/// are; none as one line that says so.
fn fitted(title: EffectKind, items: Vec<EffectLine>, more: impl Fn(i32) -> slint::SharedString) -> Vec<EffectLine> {
    let mut out = vec![EffectLine { kind: title, ..Default::default() }];
    if items.is_empty() {
        out.push(EffectLine { kind: EffectKind::Nothing, ..Default::default() });
    } else if items.len() <= LINES {
        out.extend(items);
    } else {
        let n = items.len() - (LINES - 1);
        out.extend(items.into_iter().take(LINES - 1));
        out.push(EffectLine { kind: EffectKind::More, text: more(n as i32), ..Default::default() });
    }
    out
}

impl Navicust {
    /// The game's grid, read the first time it is asked for, and the part of the engine's grid drawn: what any of
    /// the game's boards has a cell on (EXE6's 7x7, EXE5's and EXE4's 5x5 in its middle).
    fn grid(&mut self, draft: &Draft) -> Option<&GridData> {
        if self.data.is_none() {
            let content = draft.content();
            let module = crate::builds::module_data(content, &draft.kit.game, grid::MODULE);
            let data = grid::read(content, module.as_ref());
            if let Some(d) = &data {
                (self.first, self.columns) = drawn(d);
            }
            self.data = Some(data);
        }
        self.data.as_ref().and_then(Option::as_ref)
    }

    /// A drawn cell's place on the engine's grid, by its index in the drawn grid.
    fn cell_at(&self, index: i32) -> (i32, i32) {
        let n = self.columns.max(1);
        (self.first + index.max(0) % n, self.first + index.max(0) / n)
    }

    /// The drawn cell's index of a place on the engine's grid.
    fn index_of(&self, (x, y): (i32, i32)) -> i32 {
        (y - self.first) * self.columns + (x - self.first)
    }

    /// The game's programs in its library's order, each once.
    fn list(&mut self, draft: &Draft) {
        let Some(d) = self.grid(draft).cloned() else { return self.programs.clear() };
        let content = draft.content();
        let mut programs: Vec<((), EntryHandle)> = content.defs.entries_of(&d.collection).into_iter().map(|h| ((), h)).collect();
        draft.kit.order.entries(content, &d.collection, &mut programs);
        self.programs = programs.iter().map(|&(_, h)| h.0).collect();
    }

    /// The list's rows: the programs whose names, as the window says them, hold the search (any case), all of them
    /// with none.
    fn search_list(&mut self, shell: &Shell, draft: &Draft) {
        let wanted = self.search.to_lowercase();
        self.rows = draft.with_words(shell, |words| {
            self.programs.iter().copied().filter(|&piece| wanted.is_empty() || words.names.entry(EntryHandle(piece)).to_lowercase().contains(&wanted)).collect()
        });
    }

    /// The color chosen for `piece` (its place among its colors): the one chosen on its row, else that of the copy
    /// the build has placed first, else its first.
    fn color(&self, draft: &Draft, piece: u16) -> u8 {
        if let Some(&c) = self.chosen.get(&piece) {
            return c;
        }
        let Some(Some(d)) = &self.data else { return 0 };
        let placed = grid::placed(draft.content(), draft.side()).into_iter().find(|p| p.piece == piece).and_then(|p| p.color);
        placed.and_then(|name| d.colors.get(&piece)?.iter().position(|c| *c == name)).unwrap_or(0) as u8
    }

    /// Row `row`'s program in its color chosen.
    fn row(&self, draft: &Draft, row: usize) -> Option<(u16, u8)> {
        self.rows.get(row).map(|&piece| (piece, self.color(draft, piece)))
    }

    /// How many colors `piece` comes in.
    fn colors(&self, piece: u16) -> usize {
        match &self.data {
            Some(Some(d)) => d.colors.get(&piece).map_or(1, Vec::len).max(1),
            _ => 1,
        }
    }

    /// Choose color `color` for row `row`'s program: its row and the strip show it, and the next copy taken is of it.
    fn choose_color(&mut self, shell: &mut Shell, draft: &Draft, row: usize, color: usize) {
        let Some(&piece) = self.rows.get(row) else { return };
        if color >= self.colors(piece) {
            return;
        }
        self.chosen.insert(piece, color as u8);
        self.show_list(shell, draft);
        self.show_strip(shell, draft);
    }

    /// The board of the build, as the game's data have it.
    fn board<'d>(d: &'d GridData, draft: &Draft) -> Option<&'d Vec<Vec<u8>>> {
        d.board_of(draft.content(), draft.side()).and_then(|b| d.boards.get(b))
    }

    /// The board, the pieces on it and the piece in hand.
    fn show_board(&self, shell: &Shell, draft: &Draft) {
        let ui = shell.ui();
        let view = ui.global::<NavicustView>();
        let Some(Some(d)) = &self.data else { return view.set_board(false) };
        let Some(board) = Navicust::board(d, draft) else { return view.set_board(false) };
        let parts = grid::placed(draft.content(), draft.side());
        let taken = d.occupied(&parts);
        let (first, n) = (self.first, self.columns);
        let mut cells = Vec::with_capacity((n * n).max(0) as usize);
        for y in first..first + n {
            for x in first..first + n {
                let kind = match kind_at(board, x, y) {
                    b'o' => CellKind::Board,
                    b'f' => CellKind::Out,
                    _ => CellKind::None,
                };
                let here = taken.get(&(x, y));
                let on = here.and_then(|&i| parts.get(i));
                // A side is the piece's edge where the cell past it isn't of the same piece; a corner's crossing is
                // within the piece where its four cells are all of it, else on its edge.
                let edge = |dx: i32, dy: i32| taken.get(&(x + dx, y + dy)) != here;
                let crossing = |dx: i32, dy: i32| match here {
                    None => Crossing::None,
                    Some(_) if !edge(dx, 0) && !edge(0, dy) && !edge(dx, dy) => Crossing::Within,
                    Some(_) => Crossing::Edge,
                };
                cells.push(GridCell {
                    kind,
                    filled: on.is_some(),
                    tone: on.map_or_else(Default::default, |p| tone(p.color.as_deref())),
                    plus: on.is_some_and(|p| d.plus.contains(&p.piece)),
                    top: edge(0, -1),
                    right: edge(1, 0),
                    bottom: edge(0, 1),
                    left: edge(-1, 0),
                    top_left: crossing(-1, -1),
                    top_right: crossing(1, -1),
                    bottom_left: crossing(-1, 1),
                    bottom_right: crossing(1, 1),
                });
            }
        }
        // The board's own cells, which a white line goes around where a ring of out-of-bounds cells is round them.
        let size = board.len() as i32;
        let own: Vec<(i32, i32)> = (0..size).flat_map(|y| (0..size).map(move |x| (x, y))).filter(|&(x, y)| kind_at(board, x, y) == b'o').collect();
        let low = |f: fn(&(i32, i32)) -> i32| own.iter().map(f).min().unwrap_or(0);
        let high = |f: fn(&(i32, i32)) -> i32| own.iter().map(f).max().unwrap_or(0);
        view.set_board(true);
        view.set_columns(n);
        view.set_cells(ModelRc::new(VecModel::from(cells)));
        view.set_command(d.command_line.map_or(-1, |r| r as i32 - first));
        view.set_ring(board.iter().any(|r| r.contains(&b'f')));
        view.set_board_x(low(|c| c.0) - first);
        view.set_board_y(low(|c| c.1) - first);
        view.set_board_columns(high(|c| c.0) - low(|c| c.0) + 1);
        view.set_board_rows(high(|c| c.1) - low(|c| c.1) + 1);
        view.set_placed(parts.len() as i32);
        view.set_clearable(!self.empty(draft));
        // The color bar, as the game's NaviCust screen has it (docs/design/app-ui.md §2.76): its slots each a color
        // placed, in the order placed, or empty; then each color past them, spilling out of it (the color bug).
        let (bar, slots) = match d.color_bar {
            Some(slots) => (color_bar(&grid::colors_placed(&parts), slots), slots),
            None => (Vec::new(), 0),
        };
        view.set_color_bar(ModelRc::new(VecModel::from(bar)));
        view.set_bar_slots(slots as i32);
        view.set_bar_room(slots.max(d.colors_in_all()) as i32);
        // The piece in hand, by its cells from the cell it is held by.
        match &self.held {
            Some(h) => {
                let shape = d.shape(h.piece, h.compressed, h.rotation);
                let (cx, cy) = (self.at.0 - h.grab.0, self.at.1 - h.grab.1);
                let held = piece(&grid::cells(&shape, cx, cy), self.at);
                let color = d.colors.get(&h.piece).and_then(|c| c.get(h.color as usize)).cloned();
                view.set_held_fits(d.misfit(board, &parts, &shape, cx, cy).is_none());
                view.set_held(ModelRc::new(VecModel::from(held)));
                view.set_held_tone(tone(color.as_deref()));
                view.set_held_plus(d.plus.contains(&h.piece));
                // (C's hint: Uncompress where it can be: the user, "compress key on the navicust view should say
                // uncompress if uncompress is possible".)
                view.set_held_compresses(d.compresses(h.piece));
                view.set_held_compressed(h.compressed && d.compresses(h.piece));
                view.set_glide(self.glide);
                view.set_grab_x(self.at.0 - first);
                view.set_grab_y(self.at.1 - first);
                view.set_holding(true);
            }
            None => view.set_holding(false),
        }
        view.set_over_list(self.held.is_some() && self.over_list);
        view.set_lifted(self.lifted);
        view.set_turned(self.turned);
        view.set_turn_way(self.turn_way);
        view.set_refused_seq(self.refused);
        view.set_notice(self.notice.clone().unwrap_or_default().into());
    }

    /// The build's problems about a placed copy of program `piece` (its place in the list, from 0), the ruleset's among
    /// them (a program it holds back, or a version's in a build that mixes versions under one version per build).
    fn problems<'d>(draft: &'d Draft, piece: u16) -> Vec<&'d nettai_match::check::Problem> {
        let parts = grid::placed(draft.content(), draft.side());
        let at: Vec<usize> = parts.iter().enumerate().filter(|(_, p)| p.piece == piece).map(|(i, _)| i).collect();
        let mixed = draft.mixing(Registry::Entry, &Item::of(piece)).filter(|_| !at.is_empty());
        let own = draft.checked.problems.iter().filter(|p| p.field.as_deref() == Some(grid::PIECES_FIELD) && at.iter().any(|&e| p.at(e)));
        own.chain(mixed).collect()
    }

    /// The NaviCust's problems about no placed program (its board's size, a NaviCust on a navi that compiles none, a
    /// fact of it a build made by hand states wrong): every program's strip says them after its own
    /// (docs/design/app-ui.md §2.66).
    fn unplaced<'d>(&self, draft: &'d Draft) -> Vec<&'d nettai_match::check::Problem> {
        let placed = grid::placed(draft.content(), draft.side()).len();
        let on_a_piece = |p: &nettai_match::check::Problem| {
            matches!(p.what, nettai_match::Wrong::Mixed { .. }) || (p.field.as_deref() == Some(grid::PIECES_FIELD) && p.entry.is_some_and(|e| e < placed))
        };
        draft.pane_problems(&self.fields(draft.content()), false).into_iter().filter(|p| !on_a_piece(p)).collect()
    }

    /// The list: each program in its color chosen, its colors, where that color's part is and how many copies of it
    /// are placed, its name red where the build's problems are about a placed copy; over it, on a phone, what the game's
    /// rules have against the NaviCust, a placed program or the whole (docs/design/app-ui.md §2.66).
    fn show_list(&self, shell: &Shell, draft: &Draft) {
        let ui = shell.ui();
        let view = ui.global::<NavicustView>();
        view.set_in_list(self.in_list);
        let Some(Some(d)) = &self.data else { return view.set_programs(ModelRc::default()) };
        let parts = grid::placed(draft.content(), draft.side());
        let board = Navicust::board(d, draft);
        let rows: Vec<ProgramRow> = draft.with_words(shell, |words| {
            self.rows
                .iter()
                .map(|&piece| {
                    let color = self.color(draft, piece);
                    let name = d.colors.get(&piece).and_then(|c| c.get(color as usize)).cloned();
                    // (The ruleset holds a program back whole: each of its colors' parts with it.)
                    let allowed = draft.allows(grid::PIECES_FIELD, piece);
                    let colors: Vec<ProgramColor> = d.colors.get(&piece).into_iter().flatten().map(|c| ProgramColor { tone: tone(Some(c)) }).collect();
                    let copies: Vec<&grid::Placed> = parts.iter().filter(|p| p.piece == piece && p.color == name).collect();
                    // (The build's problems are about a placed copy of it: the game's rules', or the ruleset holds it back.)
                    let flagged = !Navicust::problems(draft, piece).is_empty();
                    // (A copy over the ring is out of bounds: no error, the rules bug it.)
                    let out = copies.iter().any(|p| board.is_some_and(|b| grid::cells(&d.shape(p.piece, p.compressed, p.rotation), p.x, p.y).iter().any(|&(x, y)| kind_at(b, x, y) == b'f')));
                    let state = match &self.held {
                        Some(h) if h.piece == piece => ProgramState::Held,
                        _ if out => ProgramState::Out,
                        _ if !copies.is_empty() => ProgramState::Placed,
                        // (One the build's ruleset doesn't allow says so in the error red where it isn't
                        // placed, the row dimmed: it can't be taken. A build that has it placed all the same says
                        // where, its name in the error red.)
                        _ if !allowed => ProgramState::NotAllowed,
                        _ => ProgramState::None,
                    };
                    ProgramRow {
                        name: words.names.entry(EntryHandle(piece)).into(),
                        tone: tone(name.as_deref()),
                        plus: d.plus.contains(&piece),
                        state,
                        copies: copies.len() as i32,
                        colors: ModelRc::new(VecModel::from(colors)),
                        color: color as i32,
                        flagged,
                    }
                })
                .collect()
        });
        view.set_programs(ModelRc::new(VecModel::from(rows)));
        view.set_problems(draft.pane_line(shell, &self.fields(draft.content()), false).into());
        // (The rows' colors' column is as wide as the most of them, so their states line up.)
        view.set_most_colors(self.rows.iter().map(|&piece| self.colors(piece)).max().unwrap_or(1) as i32);
    }

    /// What the NaviCust does and the bugs it brings (`own_overview`: each number it changes, the rules' own stats
    /// among them ("Soul time +1"), the programs at work that give an ability, each bug by its full name and its
    /// level or amount); a patch card's bugs aren't its. A bug that wasn't there before pulses.
    fn show_effects(&mut self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let side = draft.side().clone();
        let game = draft.kit.game.clone();
        let alone = own_overview(&content, &game, &side, &mut draft.checked);
        let panels = effect_panels(shell, draft, alone.as_ref(), &mut self.bugs);
        let ui = shell.ui();
        let view = ui.global::<NavicustView>();
        view.set_effects(ModelRc::new(VecModel::from(panels.effects)));
        view.set_bugs(ModelRc::new(VecModel::from(panels.bugs)));
        view.set_effect_items(panels.effect_items);
        view.set_bug_items(panels.bug_items);
    }

    /// Everything shown anew: the build changed.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        self.show_board(shell, draft);
        self.show_list(shell, draft);
        self.show_effects(shell, draft);
        self.show_strip(shell, draft);
    }

    /// Apply a grid edit to the build (and the piece in hand): whether the build changed.
    fn edit(&mut self, draft: &mut Draft, edit: Edit) -> bool {
        let Some(Some(d)) = &self.data else { return false };
        let held = &mut self.held;
        draft.edit(|content, side| grid::edit(d, content, side, held, edit))
    }

    /// Take a new copy of row `row`'s program in hand, its middle on the board's, whether or not copies of it are
    /// placed: compressed where it compresses, unless a copy already placed in its color says otherwise (a save
    /// compresses every copy of a program in one color alike).
    fn take(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize) {
        let Some((piece, color)) = self.row(draft, row) else { return };
        // (One the build's ruleset holds back isn't taken: the strip says so, docs/design/app-ui.md §2.68.)
        if !draft.allows(grid::PIECES_FIELD, piece) {
            self.notice = Some(shell.ui().global::<BuildWords>().invoke_not_allowed_here().to_string());
            shell.sound.play(Cue::Refused);
            self.show_board(shell, draft);
            return;
        }
        let Some(Some(d)) = &self.data else { return };
        let name = d.colors.get(&piece).and_then(|c| c.get(color as usize)).cloned();
        // (Compressed, where it compresses (the user: "navicust pieces should default to compressed in the
        // editor"), unless a copy in its color is placed uncompressed: a save compresses every copy alike.)
        let copies: Vec<bool> = grid::placed(draft.content(), draft.side()).iter().filter(|p| p.piece == piece && p.color == name).map(|p| p.compressed).collect();
        let compressed = copies.first().copied().unwrap_or_else(|| d.compresses(piece));
        // (A piece already in hand from the board goes back where it was.)
        let changed = self.edit(draft, Edit::Hold(piece, color, compressed));
        let Some(board) = self.data.as_ref().and_then(Option::as_ref).and_then(|d| Navicust::board(d, draft)) else { return };
        let middle = board.len() as i32 / 2;
        self.at = (middle, middle);
        // (Where the pointer is, the mouse's next move says.)
        self.over_list = false;
        self.lifted += 1;
        self.glide = false;
        self.notice = None;
        shell.sound.play(Cue::Pick);
        if changed {
            self.changed(shell, draft);
        } else {
            self.show_board(shell, draft);
            self.show_list(shell, draft);
        }
        shell.nav.focus("cell", self.index_of(self.at));
    }

    /// Enter on cell (x, y): the piece in hand put down there (by the cell it is held by), or the piece there
    /// picked up by it. Whether the piece in hand couldn't be put down (it shakes, and the line says why).
    fn press(&mut self, shell: &mut Shell, draft: &mut Draft, at: (i32, i32)) -> bool {
        let Some(Some(d)) = &self.data else { return false };
        let parts = grid::placed(draft.content(), draft.side());
        match &self.held {
            Some(h) => {
                let (cx, cy) = (at.0 - h.grab.0, at.1 - h.grab.1);
                let shape = d.shape(h.piece, h.compressed, h.rotation);
                let misfit = Navicust::board(d, draft).and_then(|b| d.misfit(b, &parts, &shape, cx, cy));
                if let Some(wrong) = misfit {
                    self.refused += 1;
                    self.notice = Some(draft.with_words(shell, |words| words.wrong(&wrong)));
                    shell.sound.play(Cue::Refused);
                    self.show_board(shell, draft);
                    return true;
                }
                let dropped = piece(&grid::cells(&shape, cx, cy), (self.first, self.first));
                if self.edit(draft, Edit::Place(cx, cy)) {
                    self.notice = None;
                    shell.sound.play(Cue::Pick);
                    let ui = shell.ui();
                    let view = ui.global::<NavicustView>();
                    view.set_dropped(ModelRc::new(VecModel::from(dropped)));
                    view.set_dropped_seq(view.get_dropped_seq() + 1);
                    self.changed(shell, draft);
                }
            }
            None => {
                let Some(&i) = d.occupied(&parts).get(&at) else { return false };
                if self.edit(draft, Edit::PickUp(i, at.0, at.1)) {
                    self.at = at;
                    self.over_list = false;
                    self.lifted += 1;
                    self.glide = false;
                    shell.sound.play(Cue::Pick);
                    self.changed(shell, draft);
                }
            }
        }
        false
    }

    /// The placed piece at `i` taken off (Del or a right click on it, or on its program's row: its last copy).
    fn take_off(&mut self, shell: &mut Shell, draft: &mut Draft, i: Option<usize>) {
        match i {
            Some(i) if self.edit(draft, Edit::Remove(i)) => {
                shell.sound.play(Cue::Back);
                self.changed(shell, draft);
            }
            _ => shell.sound.play(Cue::Refused),
        }
    }

    /// Nothing is placed, nor picked up off the board: CLEAR has nothing to take off.
    fn empty(&self, draft: &Draft) -> bool {
        grid::placed(draft.content(), draft.side()).is_empty() && self.held.as_ref().is_none_or(|h| h.origin.is_none())
    }

    /// CLEAR, answered: every placed program taken off the board (the board kept), and the piece in hand dropped, as
    /// one change. The cursor goes to the list.
    fn clear(&mut self, shell: &mut Shell, draft: &mut Draft) {
        let changed = self.edit(draft, Edit::Clear);
        self.notice = None;
        self.over_list = false;
        if changed {
            shell.sound.play(Cue::Back);
        }
        self.changed(shell, draft);
        shell.nav.focus("programs", 0);
    }

    /// The last copy on the board of row `row`'s program, in its color.
    fn last_copy(&self, draft: &Draft, row: usize) -> Option<usize> {
        let Some(Some(d)) = &self.data else { return None };
        let (piece, color) = self.row(draft, row)?;
        let name = d.colors.get(&piece).and_then(|c| c.get(color as usize)).cloned();
        grid::placed(draft.content(), draft.side()).iter().rposition(|p| p.piece == piece && p.color == name)
    }

    /// The piece in hand back where it came from, nothing shown: one picked up off the board goes back on it, one
    /// taken from the list back into it. Whether the build changed.
    fn let_go(&mut self, draft: &mut Draft) -> bool {
        let Some(from_board) = self.held.as_ref().map(|h| h.origin.is_some()) else { return false };
        let changed = self.edit(draft, if from_board { Edit::PutBack } else { Edit::Drop });
        self.held = None;
        changed
    }

    /// The piece in hand back where it came from (a click off the board and the list).
    fn put_back(&mut self, shell: &mut Shell, draft: &mut Draft) {
        if self.held.is_none() {
            return;
        }
        self.let_go(draft);
        shell.sound.play(Cue::Back);
        self.changed(shell, draft);
    }

    /// The piece in hand over cell `at`, as the mouse takes it (the cursor goes there where it is a cell).
    fn follow(&mut self, shell: &mut Shell, draft: &Draft, at: (i32, i32)) {
        self.at = at;
        self.glide = true;
        self.notice = None;
        self.focus_cell(shell, at);
        self.show_board(shell, draft);
    }

    /// The cursor to cell `at`, where it is one (EXE6's corners aren't).
    fn focus_cell(&self, shell: &mut Shell, at: (i32, i32)) {
        let index = self.index_of(at);
        let real = shell.nav.registry().borrow().rect(&crate::nav::Key::new("cell", index)).is_some();
        if real {
            shell.nav.focus("cell", index);
        }
    }

    /// The drawn grid's cell under window point `at` (`x * 65536 + y`), from where the view lays its cells out;
    /// none off the grid.
    fn cell_under(&self, shell: &Shell, at: i32) -> Option<(i32, i32)> {
        let (x, y) = ((at >> 16) as f32, (at & 0xFFFF) as f32);
        let n = self.columns;
        let registry = shell.nav.registry();
        let registry = registry.borrow();
        let rect = |i: i32| registry.rect(&crate::nav::Key::new("cell", i));
        // (Two cells side by side give the grid's pitch: EXE6's corners are no cells.)
        let (i, a, b) = (0..n * n).filter(|i| i % n + 1 < n).find_map(|i| Some((i, rect(i)?, rect(i + 1)?)))?;
        let pitch = b.x - a.x;
        if pitch <= 0.0 {
            return None;
        }
        let (x0, y0) = (a.x - (i % n) as f32 * pitch, a.y - (i / n) as f32 * pitch);
        let (cx, cy) = (((x - x0) / pitch).floor() as i32, ((y - y0) / pitch).floor() as i32);
        ((0..n).contains(&cx) && (0..n).contains(&cy)).then_some((self.first + cx, self.first + cy))
    }

    /// Whether window point `at` (`x * 65536 + y`) is over the list (its rows and the room under them): a click
    /// there with a piece in hand puts it away into it.
    fn on_list(shell: &Shell, at: i32) -> bool {
        editor::over(shell, &["programs"], editor::pointer_at(at))
    }

    /// The pointer at `at` with no button held (or just let go), a piece in hand: over the board, the piece follows
    /// it from cell to cell (off the board it stays at the last cell it was over); over the list, the list shows that
    /// a click there puts it away.
    fn hover(&mut self, shell: &mut Shell, draft: &Draft, at: i32) {
        let cell = self.cell_under(shell, at);
        let over_list = cell.is_none() && Navicust::on_list(shell, at);
        let moved = cell.filter(|&c| c != self.at);
        let shown = over_list != self.over_list;
        self.over_list = over_list;
        match moved {
            Some(c) => self.follow(shell, draft, c),
            None if shown => self.show_board(shell, draft),
            None => {}
        }
    }

    /// What the mouse did (`PRESS_ROW` and the rest). A click acts as the button goes down, so a press that moves
    /// before it is let go is the same click, and the held button carries nothing.
    fn pointed(&mut self, shell: &mut Shell, draft: &mut Draft, phase: i32, value: i32) {
        match phase {
            // (A click on a program's row takes a new copy of it; with a piece in hand, it puts that away into the
            // list: "drag it back to where it came from".)
            PRESS_ROW => {
                shell.nav.used_pointer();
                self.wheel = 0;
                if self.held.is_some() {
                    self.put_away(shell, draft);
                } else {
                    self.activate(shell, draft, "programs", value);
                }
            }
            // (A click on the board puts the piece in hand down on the cell clicked, or it shakes and stays in hand;
            // with none, it picks up the placed piece there. One beside the cells puts the piece in hand back.)
            PRESS_BOARD => {
                shell.nav.used_pointer();
                self.wheel = 0;
                match (self.held.is_some(), self.cell_under(shell, value)) {
                    (true, Some(at)) => {
                        self.follow(shell, draft, at);
                        self.press(shell, draft, at);
                    }
                    (true, None) => self.put_back(shell, draft),
                    (false, Some(at)) => {
                        self.press(shell, draft, at);
                        self.focus_cell(shell, at);
                    }
                    (false, None) => {}
                }
            }
            // (Let go, the piece in hand comes under the pointer, and follows it.)
            RELEASE | HOVER if self.held.is_some() => self.hover(shell, draft, value),
            // (A notch turns it a quarter; a trackpad's small steps add up to one.)
            WHEEL if self.held.is_some() => {
                self.wheel += value;
                if self.wheel.abs() >= NOTCH {
                    self.turn(shell, draft, self.wheel < 0);
                    self.wheel = 0;
                }
            }
            // (A right click puts the piece in hand away into the list, whoever took it and from where: off the
            // board, never back where it was.)
            RIGHT_CLICK if self.held.is_some() => self.put_away(shell, draft),
            // (A click off the board and the list's rows with a piece in hand: on the list (the room under its rows),
            // it is put away into it; anywhere else, it goes back where it was, and a new copy, which had no place,
            // goes away.)
            PRESS_OFF if self.held.is_some() => {
                if Navicust::on_list(shell, value) {
                    self.put_away(shell, draft);
                } else {
                    self.put_back(shell, draft);
                }
            }
            // (With nothing in hand, a right click on a placed piece takes it off, as Del does; the cursor comes to
            // it first, as a click's does.)
            RIGHT_CLICK => {
                let Some(at) = self.cell_under(shell, value) else { return };
                shell.nav.used_pointer();
                self.focus_cell(shell, at);
                let Some(Some(d)) = &self.data else { return };
                let parts = grid::placed(draft.content(), draft.side());
                if let Some(&i) = d.occupied(&parts).get(&at) {
                    self.take_off(shell, draft, Some(i));
                }
            }
            _ => {}
        }
    }

    /// The piece in hand turned a quarter (`clockwise`, or back), about the cell it is held by.
    fn turn(&mut self, shell: &mut Shell, draft: &mut Draft, clockwise: bool) {
        self.edit(draft, Edit::Turn(clockwise));
        self.turned += 1;
        self.turn_way = if clockwise { 1 } else { -1 };
        self.notice = None;
        shell.sound.play(Cue::Cursor);
        self.show_board(shell, draft);
    }

    /// The piece in hand compressed, or not: refused where it has one shape alone.
    fn compress(&mut self, shell: &mut Shell, draft: &mut Draft) {
        let compresses = matches!((&self.data, &self.held), (Some(Some(d)), Some(h)) if d.compresses(h.piece));
        if !compresses {
            return shell.sound.play(Cue::Refused);
        }
        // (Its other shape is turned as it is and about the same middle, so it doesn't move: the cursor goes to the
        // cell it is held by, which is the one it was held by where the shape still has it.)
        let before = self.held.as_ref().map(|h| h.grab).unwrap_or_default();
        self.edit(draft, Edit::Compress);
        let after = self.held.as_ref().map(|h| h.grab).unwrap_or_default();
        self.at = (self.at.0 - before.0 + after.0, self.at.1 - before.1 + after.1);
        self.glide = false;
        self.notice = None;
        shell.sound.play(Cue::Cursor);
        self.show_board(shell, draft);
        if after != before {
            shell.nav.focus("cell", self.index_of(self.at));
        }
    }

    /// The piece in hand put away into the list (as the game's NaviCust does with B): one picked up off the board
    /// is taken off it.
    fn put_away(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.edit(draft, Edit::Drop);
        self.held = None;
        self.notice = None;
        shell.sound.play(Cue::Back);
        self.show_board(shell, draft);
        self.show_list(shell, draft);
        self.show_strip(shell, draft);
    }

    /// The list's row the cursor is on, if it is on one.
    fn focused_row(&self, shell: &Shell) -> Option<usize> {
        shell.nav.focused().filter(|k| k.id == "programs").map(|k| k.index.max(0) as usize)
    }

    /// The cell the cursor is on, if it is on one.
    fn focused_cell(&self, shell: &Shell) -> Option<(i32, i32)> {
        shell.nav.focused().filter(|k| k.id == "cell").map(|k| self.cell_at(k.index))
    }

    /// The program under the cursor, in its color (its place among the program's): the piece in hand, the piece
    /// placed on the cell the cursor is on, or the list's row; and whether it is a placed piece the cursor is on, or
    /// a placed program's row (Del takes it off, or its last copy).
    fn under_cursor(&self, shell: &Shell, draft: &Draft) -> (Option<(u16, u8)>, bool) {
        let Some(Some(d)) = &self.data else { return (None, false) };
        if let Some(h) = &self.held {
            return (Some((h.piece, h.color)), false);
        }
        let color_of = |piece: u16, name: &Option<String>| name.as_ref().and_then(|n| d.colors.get(&piece)?.iter().position(|c| c == n)).unwrap_or(0) as u8;
        let parts = grid::placed(draft.content(), draft.side());
        match shell.nav.focused() {
            Some(k) if k.id == "cell" => match d.occupied(&parts).get(&self.cell_at(k.index)).and_then(|&i| parts.get(i)) {
                Some(p) => (Some((p.piece, color_of(p.piece, &p.color))), true),
                None => (None, false),
            },
            Some(k) if k.id == "programs" => {
                let row = self.row(draft, k.index.max(0) as usize);
                let placed = row.is_some_and(|(piece, color)| {
                    let name = d.colors.get(&piece).and_then(|c| c.get(color as usize)).cloned();
                    parts.iter().any(|p| p.piece == piece && p.color == name)
                });
                (row, placed)
            }
            _ => (None, false),
        }
    }

    /// The strip across the top: the program under the cursor (while the pointer is on a row of the list and no piece
    /// is in hand, that row's), by its shape and compressed shape, its name in its color, the game's description of it
    /// and the colors it comes in, and at its top right where nothing was refused what the build's problems have against
    /// a placed copy of it, the ruleset's among them, then what is against the NaviCust and about no placed program
    /// (docs/design/app-ui.md §2.66); and the footer's hints
    /// for a placed piece (or a placed program's row) under the cursor.
    fn show_strip(&self, shell: &Shell, draft: &Draft) {
        let (program, on_piece) = self.under_cursor(shell, draft);
        let hovered = self.hovered.filter(|_| self.held.is_none()).and_then(|r| self.row(draft, r));
        let program = hovered.or(program);
        let ui = shell.ui();
        let view = ui.global::<NavicustView>();
        view.set_on_piece(on_piece);
        view.set_on_colors(self.held.is_none() && self.focused_row(shell).and_then(|r| self.rows.get(r)).is_some_and(|&p| self.colors(p) > 1));
        let fault = program.map(|(piece, _)| draft.line(shell, Navicust::problems(draft, piece).into_iter().chain(self.unplaced(draft)))).unwrap_or_default();
        view.set_fault(fault.into());
        let Some((Some(d), (piece, color))) = self.data.as_ref().zip(program) else { return view.set_detail(ProgramDetail::default()) };
        let detail = draft.with_words(shell, |words| {
            let colors = d.colors.get(&piece).cloned().unwrap_or_default();
            let (shape, columns) = boxed(&d.shape(piece, false, 0));
            // (A program of one shape alone has no compressed shape: "there's only one form of it".)
            let (compressed, compressed_columns) = if d.compresses(piece) { boxed(&d.shape(piece, true, 0)) } else { (Vec::new(), 0) };
            ProgramDetail {
                there: true,
                name: words.names.entry(EntryHandle(piece)).into(),
                tone: tone(colors.get(color as usize).map(String::as_str)),
                plus: d.plus.contains(&piece),
                description: words.names.description(EntryHandle(piece)).map(|t| crate::games::one_line(&t)).unwrap_or_default().into(),
                shape: ModelRc::new(VecModel::from(shape)),
                columns,
                compressed: ModelRc::new(VecModel::from(compressed)),
                compressed_columns,
                colors: ModelRc::new(VecModel::from(colors.iter().map(|c| tone(Some(c))).collect::<Vec<_>>())),
                color: color as i32,
            }
        });
        view.set_detail(detail);
    }
}

impl Tab for Navicust {
    fn id(&self) -> EditorTab {
        EditorTab::Navicust
    }

    /// The pieces placed, and the board they are on.
    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![grid::PIECES_FIELD.into(), grid::SIZE_FIELD.into()]
    }

    fn own_navi_only(&self) -> bool {
        true
    }

    fn main(&self) -> &'static str {
        "programs"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.grid(draft);
        if self.programs.is_empty() {
            self.list(draft);
        }
        self.search_list(shell, draft);
        shell.ui().global::<NavicustView>().set_search(self.search.as_str().into());
        self.changed(shell, draft);
    }

    /// A piece in hand isn't lost: one picked up off the board goes back where it was.
    fn hide(&mut self, shell: &mut Shell, draft: &mut Draft) {
        self.let_go(draft);
        self.notice = None;
        self.hovered = None;
        let _ = shell;
    }

    /// Enter on a program's row takes a new copy of it in hand, however many are placed (Del takes the last one
    /// off); on a cell, see `press`.
    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        match id {
            "programs" => self.take(shell, draft, index.max(0) as usize),
            "cell" => {
                let at = self.cell_at(index);
                self.press(shell, draft, at);
            }
            "clear" => {
                let question = shell.ui().global::<NavicustView>().get_clear_question();
                editor::ask_clear(shell, question, self.empty(draft));
            }
            _ => {}
        }
    }

    /// The mouse's clicks and moves (the view's pointer areas).
    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        match id {
            "drag" => self.pointed(shell, draft, index, choice),
            // (A click or a tap on one of a row's colors chooses it alone, as Left and Right do; a click on the row
            // takes the part. The user first asked that it take the part too, "selecting the color in the ncp editor
            // should also pick up the ncp", then: "selecting a color on the palette shouldn't pick up the navicust
            // part actually".)
            "color" => self.choose_color(shell, draft, index.max(0) as usize, choice.max(0) as usize),
            _ => {}
        }
    }

    /// Left and Right on a program's row with colors to choose from choose the one before or after; past the first or
    /// the last, the focus moves on as ever.
    fn route(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        let (Some(row), None) = (self.focused_row(shell), &self.held) else { return false };
        let Some(&piece) = self.rows.get(row) else { return false };
        let to = match action {
            Action::Left => (self.color(draft, piece) as usize).checked_sub(1),
            Action::Right => Some(self.color(draft, piece) as usize + 1),
            _ => None,
        };
        let Some(to) = to.filter(|&c| c < self.colors(piece)) else { return false };
        self.choose_color(shell, draft, row, to);
        true
    }

    /// The pointer on a row of the list: the strip describes its program.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        if id == "programs" && editor::hover(&mut self.hovered, index.max(0) as usize, on) {
            self.show_strip(shell, draft);
        }
    }

    /// The cursor on a cell: the piece in hand is there. In the list, its title is lit. The strip describes the
    /// program under the cursor (the focus is the last thing moved, until the pointer goes onto another row).
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        self.hovered = None;
        // (Why a program wasn't taken is said until the cursor moves.)
        if self.held.is_none() && self.notice.take().is_some() {
            self.show_board(shell, draft);
        }
        let in_list = id == "programs" || id == "programs.search";
        if in_list != self.in_list {
            self.in_list = in_list;
            shell.ui().global::<NavicustView>().set_in_list(in_list);
        }
        // (A cursor that comes where the piece is already, as after the mouse picks one up, doesn't move it.)
        if id == "cell" && self.held.is_some() && self.cell_at(index) != self.at {
            self.at = self.cell_at(index);
            self.glide = true;
            self.notice = None;
            self.show_board(shell, draft);
        }
        self.show_strip(shell, draft);
    }

    /// The piece in hand: A and S (the keys bound to L and R) turn it, C compresses it. With none, X is CLEAR.
    /// The programs' search typed in: the list is those it finds, from its top.
    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, _done: bool) {
        if id != "programs.search" {
            return;
        }
        self.search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
        self.search_list(shell, draft);
        self.hovered = None;
        self.show_list(shell, draft);
        self.show_strip(shell, draft);
        let ui = shell.ui();
        ui.global::<NavicustView>().set_search(self.search.as_str().into());
        shell.nav.list_to_top(&ui, "programs");
    }

    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &KeyPress) -> bool {
        if self.held.is_none() {
            if editor::clear_key(shell, key) {
                self.activate(shell, draft, "clear", 0);
                return true;
            }
            return false;
        }
        match (key.code, shell.bindings.of_key(key.code)) {
            (_, Some(keys::L)) => self.turn(shell, draft, false),
            (_, Some(keys::R)) => self.turn(shell, draft, true),
            (KeyCode::KeyC, _) if !key.repeat => self.compress(shell, draft),
            (KeyCode::KeyC, _) => {}
            _ => return false,
        }
        true
    }

    /// The piece in hand: the pad's shoulders turn it, its north button compresses it.
    fn button(&mut self, shell: &mut Shell, draft: &mut Draft, button: nettai_gamepad::Button) -> bool {
        if self.held.is_none() {
            return false;
        }
        match button {
            nettai_gamepad::Button::LeftShoulder => self.turn(shell, draft, false),
            nettai_gamepad::Button::RightShoulder => self.turn(shell, draft, true),
            nettai_gamepad::Button::North => self.compress(shell, draft),
            _ => return false,
        }
        true
    }

    /// Delete: the piece in hand put away, the piece under the cursor taken off, or a program's last copy.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        if action != Action::Delete {
            return false;
        }
        if self.held.is_some() {
            self.put_away(shell, draft);
            return true;
        }
        let Some(Some(d)) = &self.data else { return false };
        let parts = grid::placed(draft.content(), draft.side());
        let at = match (self.focused_cell(shell), shell.nav.focused()) {
            (Some(cell), _) => d.occupied(&parts).get(&cell).copied(),
            (None, Some(k)) if k.id == "programs" => self.last_copy(draft, k.index.max(0) as usize),
            _ => None,
        };
        self.take_off(shell, draft, at);
        true
    }

    /// Back (Esc, the pad's east button) with a piece in hand puts it away into the list.
    fn back(&mut self, shell: &mut Shell, draft: &mut Draft) -> bool {
        if self.held.is_none() {
            return false;
        }
        self.put_away(shell, draft);
        true
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice) {
            self.clear(shell, draft);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each game's grid is drawn as the design has it: EXE6's 7x7 (its ring round the board), EXE5's and EXE4's
    /// 5x5 in the engine's grid's middle, with the command line on its row.
    #[test]
    fn each_games_grid_is_drawn_whole() {
        for (content, game, first, columns) in [
            (nettai_match::testing::exe6_content(), "exe6", 0, 7),
            (nettai_match::testing::exe5_content(), "exe5", 1, 5),
            (nettai_match::testing::exe4_content(), "exe4", 1, 5),
        ] {
            let module = crate::builds::module_data(&content, game, grid::MODULE);
            let data = grid::read(&content, module.as_ref()).unwrap();
            assert_eq!(drawn(&data), (first, columns), "{game}");
            let last = data.boards.last().unwrap();
            assert_eq!(last.iter().any(|r| r.contains(&b'f')), game == "exe6", "{game}: a ring");
        }
    }

    /// The color bar: its four slots filled in the order the colors were first placed, the rest empty; a fifth and
    /// a sixth color spill past them.
    #[test]
    fn the_color_bar_fills_its_slots_and_spills_past_them() {
        let content = nettai_match::testing::exe6_content();
        let data = grid::read(&content, crate::builds::module_data(&content, "exe6", grid::MODULE).as_ref()).unwrap();
        let part = |color: &str| grid::Placed { piece: 0, x: 0, y: 0, rotation: 0, color: Some(color.into()), compressed: false, record: nettai_match::facts::Stated::Record(Vec::new()) };
        let parts: Vec<grid::Placed> = ["pink", "white", "pink", "blue"].into_iter().map(part).collect();
        let slots = data.color_bar.unwrap();
        let placed = grid::colors_placed(&parts);
        assert_eq!(placed, ["pink", "white", "blue"]);
        let filled = |bar: &[ColorSlot]| bar.iter().map(|s| s.filled).collect::<Vec<_>>();
        assert_eq!(filled(&color_bar(&placed, slots)), [true, true, true, false]);
        let more: Vec<String> = ["pink", "white", "blue", "red", "green", "yellow"].map(String::from).into();
        assert_eq!(filled(&color_bar(&more, slots)), [true; 6], "two past the four slots");
    }

    /// The bugs the tab lists are the NaviCust's own: UnderSht off the command line brings the Movement Bug, and
    /// Kettle's card the Custom Damage, which the whole build's overview has and the NaviCust's doesn't.
    #[test]
    fn the_bugs_are_the_navicusts_own() {
        use nettai_match::build::Bug;
        use nettai_match::testing::{PlacedProgram, patch_cards, program_colors, set_navicust};
        let content = nettai_match::testing::exe6_content();
        let builds = nettai_match::build::Builds::of(&content, "exe6").unwrap();
        let mut side = builds.new_build(&content, "falzar").unwrap();
        let undersht = nettai_match::ids::entry(&content, "exe6", "navicust_programs", "undersht").unwrap();
        let white = program_colors(&content, undersht).iter().position(|c| *c == "white").unwrap() as u8;
        set_navicust(&content, &mut side, &[PlacedProgram { program: undersht, color: white, x: 1, y: 2, rotation: 0, compressed: false }], 2);
        let bugs = |o: Option<Overview>| o.unwrap().bugs.into_iter().filter_map(|b| if let Bug::Named { key, .. } = b { Some(key) } else { None }).collect::<Vec<_>>();
        let own = |side: &Side| bugs(own_overview(&content, "exe6", side, &mut Checked::of(&content, "exe6", side)));
        assert_eq!(own(&side), ["step_bug"], "no cards: the build's bugs are the NaviCust's");
        side.set_fact(&content, "patch_cards", &patch_cards(&content, "exe6", "kettle")).unwrap();
        assert_eq!(bugs(Checked::of(&content, "exe6", &side).overview(&content, &side)), ["step_bug", "custom_damage"], "the whole build's");
        assert_eq!(own(&side), ["step_bug"], "Kettle's isn't the NaviCust's");
    }
}
