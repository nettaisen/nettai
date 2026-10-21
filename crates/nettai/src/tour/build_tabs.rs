//! The screenshot pass's steps for the build editor's NaviCust, Patch cards and Auto battle tabs (docs/app.md
//! §13): each tab in the game the pass walks, with pairs a focus move apart, a piece moved, turned, put down,
//! refused and put back, a program placed twice, cards put in and refused, a card's bug (the Patch cards tab's, not
//! the NaviCust's), places filled, a build's problems said on its tabs (two programs over each other, a card of the
//! other version); and the motion probe's bursts of what the tabs move. A tab the game hasn't (auto battle data: EXE6's setup takes none) is passed over: a step names the tab by
//! what it is, and `on_tab` reports a picture taken on another.
//!
//! The walk makes a build of its own (the pass's builds folder, `paths::keep_builds_in`), as a player makes one.

use super::{Pair, SETTLE, Step};
use crate::builds::auto::{self, Holds};
use crate::builds::store;
use crate::input::{Action, KeyPress};
use nettai_battle::content::ChipClass;
use crate::screens::ScreenId;
use crate::shell::App;
use nettai_ui::{BuildsView, EditorTab, NavicustView};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model};
use std::cell::RefCell;
use std::rc::Rc;

type Shot<'a> = &'a dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair);

/// The walk is on tab `tab` of the editor; if it isn't, the report says so (as a text the pass counts against the
/// picture).
fn on_tab(tab: EditorTab, name: &'static str) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let view = ui.global::<BuildsView>();
        if app.borrow().shell.screen() != ScreenId::Builds || view.get_shown() != tab {
            ui.global::<nettai_ui::Audit>().invoke_clipped(format!("(the walk isn't on the editor's {name} tab: this picture is of something else)").into());
        }
    })
}

// What the walk does: an action, the focus put on a control, a call of the screen's own module. Each does
// nothing while the pass passes by a tab the game hasn't (`to_tab`).
fn act(action: Action) -> Step {
    Box::new(move |_, app, tour| {
        if !tour.passing.get() {
            app.borrow_mut().act(action);
        }
    })
}
fn focus(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, tour| {
        if !tour.passing.get() {
            app.borrow_mut().shell.nav.focus(id, index);
        }
    })
}
fn press(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, tour| {
        if tour.passing.get() {
            return;
        }
        let mut app = app.borrow_mut();
        app.shell.nav.focus(id, index);
        app.act(Action::Confirm);
    })
}
fn screen(f: impl Fn(&mut dyn crate::screens::Screen, &mut crate::shell::Shell) + 'static) -> Step {
    Box::new(move |_, app, tour| {
        if !tour.passing.get() {
            app.borrow_mut().with_screen(ScreenId::Builds, &f);
        }
    })
}
/// A key pressed on the editor, as the window's keys reach it (before the menus read it).
fn key(code: KeyCode, text: &'static str) -> Step {
    screen(move |s, shell| {
        s.key(shell, &KeyPress { code, text: Some(text.into()), repeat: false, shift: false });
    })
}

/// Out of whatever of the builds is open, back to Play's builds.
fn leave(app: &mut App) {
    super::builds::leave(app);
}

/// A build of the game's first version, made through Play in an emptied builds folder, and open in the editor.
fn open_build(add: &mut dyn FnMut(u64, Step)) {
    add(SETTLE, super::builds::fresh());
    super::builds::make(add, "Tab tour", 0);
    add(SETTLE, press("edit", 0));
}

/// The editor's tab of kind `tab`, by its place in the bar. A tab the game hasn't is passed by: the steps after
/// this do nothing and take no picture, until the next tab is gone to or the walk ends.
pub(super) fn to_tab(tab: EditorTab) -> Step {
    Box::new(move |ui, app, tour| {
        let view = ui.global::<BuildsView>();
        let names = view.get_tab_names();
        let word = names.row_data(tab as usize).unwrap_or_default();
        let at = view.get_tabs().iter().position(|t| t.text == word && t.enabled);
        tour.passing.set(at.is_none());
        let Some(at) = at else { return };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("tab", at as i32);
        app.act(Action::Confirm);
    })
}

/// Take in hand the `nth` program of the NaviCust's list (from 0) that `which` picks, as Enter on its row does.
pub(super) fn take(nth: usize, which: fn(&nettai_ui::ProgramRow) -> bool) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let rows = ui.global::<NavicustView>().get_programs();
        let Some(at) = rows.iter().enumerate().filter(|(_, r)| which(r)).nth(nth).map(|(i, _)| i) else { return };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("programs", at as i32);
        app.act(Action::Confirm);
    })
}

/// Where the piece in hand may go: off the command line, on the out-of-bounds ring, over another piece, or anywhere
/// on the board itself.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Spot {
    OffTheLine,
    Ring,
    Over,
    Free,
}

/// Put the piece in hand down at the first cell where it would be at `spot` (and, but `Spot::Over`, fit), as the
/// cursor moved there and Enter does: found from what the view shows of the board, whatever the game's shapes.
pub(super) fn put(spot: Spot) -> Step {
    Box::new(move |ui, app, tour| {
        if !tour.passing.get() {
            put_now(ui, app, spot);
        }
    })
}

/// `put`'s work, done at once.
fn put_now(ui: &nettai_ui::AppWindow, app: &Rc<RefCell<App>>, spot: Spot) {
    let view = ui.global::<NavicustView>();
    let (n, command) = (view.get_columns(), view.get_command());
    let cells: Vec<nettai_ui::GridCell> = view.get_cells().iter().collect();
    let held: Vec<(i32, i32)> = view.get_held().iter().map(|c| (c.x, c.y)).collect();
    let at = |x: i32, y: i32| (x >= 0 && y >= 0 && x < n && y < n).then(|| &cells[(y * n + x) as usize]);
    let fits = |gx: i32, gy: i32, ring: bool| -> bool {
        let on: Vec<Option<&nettai_ui::GridCell>> = held.iter().map(|&(dx, dy)| at(gx + dx, gy + dy)).collect();
        let real = on.iter().all(|c| c.is_some_and(|c| c.kind != nettai_ui::CellKind::None));
        let out = on.iter().flatten().filter(|c| c.kind == nettai_ui::CellKind::Out).count();
        let over = on.iter().flatten().any(|c| c.filled);
        let line = held.iter().any(|&(_, dy)| gy + dy == command);
        real && out < held.len()
            && match spot {
                Spot::OffTheLine => !over && !line && (ring || out == 0),
                Spot::Ring => !over && out > 0,
                Spot::Over => over,
                Spot::Free => !over && out == 0,
            }
    };
    // (Off the command line on the board itself where the piece has the room, else jutting onto the ring.)
    let places = || (0..n).flat_map(|y| (0..n).map(move |x| (x, y))).filter(|&(x, y)| at(x, y).is_some_and(|c| c.kind != nettai_ui::CellKind::None));
    let found = places().find(|&(x, y)| fits(x, y, false)).or_else(|| places().find(|&(x, y)| fits(x, y, true)));
    let Some((gx, gy)) = found else { return };
    let mut app = app.borrow_mut();
    app.shell.nav.focus("cell", gy * n + gx);
    app.act(Action::Confirm);
}

/// The next pair's second shot may differ in the strip too (the program under the cursor, which a focus move
/// changes), on a desktop: it is the tab's top, above the board, 104 tall and 12 above the board's panel, whose
/// cells start 16 under its header, 44 tall. (A phone has no strip.)
fn strip_may_differ() -> Step {
    Box::new(|ui, app, tour| {
        let app = app.borrow();
        if tour.passing.get() || app.shell.scale.phone {
            return;
        }
        let n = ui.global::<NavicustView>().get_columns();
        let registry = app.shell.nav.registry();
        let registry = registry.borrow();
        // (The board's top, from any of its cells: they are 3 apart.)
        let Some(top) = (0..n * n).find_map(|k| registry.rect(&crate::nav::Key::new("cell", k)).map(|r| r.y - (k / n) as f32 * (r.h + 3.0))) else { return };
        let bottom = top - 16.0 - 44.0 - 12.0;
        // (`compare` lets a pair differ 12 around what it is given: this stops 13 short of the strip's bottom.)
        tour.may_differ(crate::nav::Rect { x: 0.0, y: bottom - 104.0, w: 10_000.0, h: 104.0 - 13.0 });
    })
}

/// The cursor on the first cell of the board a placed piece covers.
fn on_placed() -> Step {
    Box::new(|ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let cells = ui.global::<NavicustView>().get_cells();
        let Some(at) = cells.iter().position(|c| c.filled) else { return };
        app.borrow_mut().shell.nav.focus("cell", at as i32);
    })
}

fn plus(r: &nettai_ui::ProgramRow) -> bool {
    r.plus
}
pub(super) fn program(r: &nettai_ui::ProgramRow) -> bool {
    !r.plus
}

/// The NaviCust tab's steps.
fn navicust(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::Navicust));
    // The programs' search: a name typed, the list those it finds; cleared.
    add(SETTLE, focus("programs.search", 0));
    add(SETTLE, screen(|s, shell| s.edit(shell, "programs.search", 0, "custom", false)));
    shot(add, "tabs-navicust-search", Pair::None);
    add(SETTLE, screen(|s, shell| s.edit(shell, "programs.search", 0, "", true)));
    add(SETTLE, focus("programs", 0));
    shot(add, "tabs-navicust", Pair::First);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
    add(SETTLE, act(Action::Down));
    add(SETTLE, strip_may_differ());
    shot(add, "tabs-navicust-focus", Pair::Second);
    // A program taken in hand (it is on the board's middle, the command line's row), moved a cell and turned a
    // quarter; turned back and put down a row lower, on the command line still.
    add(SETTLE, take(0, program));
    shot(add, "tabs-navicust-held", Pair::None);
    add(SETTLE, act(Action::Right));
    add(SETTLE, key(KeyCode::KeyS, "s"));
    shot(add, "tabs-navicust-turned", Pair::None);
    add(SETTLE, key(KeyCode::KeyA, "a"));
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-placed", Pair::None);
    // A plus part at the left; another program at the top, off the command line, which brings its bug (purple,
    // with its level).
    add(SETTLE, take(0, plus));
    add(SETTLE, act(Action::Left));
    add(SETTLE, act(Action::Left));
    add(SETTLE, act(Action::Down));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, take(1, program));
    add(SETTLE, put(Spot::OffTheLine));
    shot(add, "tabs-navicust-bugs", Pair::None);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
    // A program put over another: refused, and why; put back.
    add(SETTLE, take(2, program));
    add(SETTLE, put(Spot::Over));
    // (Enter again where it is: the focus's move to the cell, told after the first, said nothing of why.)
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-refused", Pair::None);
    add(SETTLE, act(Action::Back));
    // A plus part on the ring, where the game's board has one (EXE6's: out of bounds, which is no error); a game
    // without one shows the piece in hand, at the board's middle.
    add(SETTLE, take(1, plus));
    add(SETTLE, put(Spot::Ring));
    shot(add, "tabs-navicust-ring", Pair::None);
    // A placed program under the cursor (the hints say Del takes it off), picked up (In hand), and put away into the
    // list with Back (no longer placed). (The piece in hand where it found no ring put away first: with none in
    // hand, Back would leave the editor.)
    add(SETTLE, Box::new(|ui, app, tour| {
        if !tour.passing.get() && ui.global::<NavicustView>().get_holding() {
            app.borrow_mut().act(Action::Back);
        }
    }));
    add(SETTLE, on_placed());
    shot(add, "tabs-navicust-on-piece", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-picked-up", Pair::None);
    add(SETTLE, act(Action::Back));
    shot(add, "tabs-navicust-put-away", Pair::None);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
    // A placed program's row under the cursor (its hints: Enter takes another copy, Del takes the last one off), and
    // Enter: a second copy in hand, put down where it fits. Two copies on the board, which the row counts. (A plus
    // part's, a small piece that finds room: one is placed first where none is.)
    add(SETTLE, Box::new(|ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let rows = ui.global::<NavicustView>().get_programs();
        if rows.iter().any(|r| r.plus && r.state == nettai_ui::ProgramState::Placed) {
            return;
        }
        let Some(at) = rows.iter().position(|r| r.plus) else { return };
        {
            let mut app = app.borrow_mut();
            app.shell.nav.focus("programs", at as i32);
            app.act(Action::Confirm);
        }
        put_now(ui, app, Spot::Free);
    }));
    add(SETTLE, row(|r| r.plus && r.state == nettai_ui::ProgramState::Placed));
    shot(add, "tabs-navicust-row-placed", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-row-taken", Pair::None);
    add(SETTLE, put(Spot::Free));
    add(SETTLE, row(|r| r.copies > 1));
    shot(add, "tabs-navicust-twice", Pair::None);
    add(SETTLE, expect(|ui, _| {
        let rows = ui.global::<NavicustView>().get_programs();
        (!rows.iter().any(|r| r.copies == 2)).then(|| "Enter on a placed program's row, and the copy put down: no program has two copies on the board".to_string())
    }));
    // Del on the row: its last copy off the board.
    add(SETTLE, act(Action::Delete));
    shot(add, "tabs-navicust-row-taken-off", Pair::None);
}

/// The NaviCust's board by the mouse's clicks (docs/app.md §8; the user: "dragging navicust piece is confusing because
/// usually it's click to drop"): the first placed piece clicked up and carried over the list, shot there (the remove
/// look: the piece faded on the board, the list washed under CLICK TO PUT AWAY), and a click on the list puts it away
/// (a program fewer on the board); the next clicked up, carried off the board elsewhere (the effects' panel), shot
/// there, and a click there puts it back where it was (the board as it was). A desktop's: a touch has no hover.
fn carries(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::Navicust));
    let kept: Rc<RefCell<(i32, Vec<(i32, i32)>)>> = Rc::default();
    let keep = |kept: &Rc<RefCell<(i32, Vec<(i32, i32)>)>>| -> Step {
        let kept = kept.clone();
        Box::new(move |ui, _, _| *kept.borrow_mut() = (ui.global::<NavicustView>().get_placed(), Board::of(ui).filled()))
    };
    let first = |b: &Board| b.filled().first().map(|&(x, y)| Point::Cell(x as f32, y as f32));
    add(SETTLE, keep(&kept));
    add(SETTLE, mouse_on(first, true));
    add(SETTLE, mouse_on(|_| Some(Point::Row(1)), false));
    add(SETTLE, expect(|ui, _| {
        let view = ui.global::<NavicustView>();
        (!view.get_holding() || !view.get_over_list()).then(|| format!("a placed piece clicked up and carried over the list: in hand {}, the list's remove look {}", view.get_holding(), view.get_over_list()))
    }));
    shot(add, "tabs-navicust-over-list", Pair::None);
    add(SETTLE, mouse_on(|_| Some(Point::Row(1)), true));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let view = ui.global::<NavicustView>();
        let (placed, before) = (view.get_placed(), was.borrow().0);
        (view.get_holding() || view.get_over_list() || placed + 1 != before).then(|| format!("a click on the list with a piece in hand: {placed} programs placed (were {before}), in hand {}", view.get_holding()))
    }));
    shot(add, "tabs-navicust-clicked-away", Pair::None);
    add(SETTLE, keep(&kept));
    add(SETTLE, mouse_on(first, true));
    add(SETTLE, mouse_on(|_| Some(Point::Off), false));
    shot(add, "tabs-navicust-carried-off", Pair::None);
    add(SETTLE, mouse_on(|_| Some(Point::Off), true));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let view = ui.global::<NavicustView>();
        let was = was.borrow();
        let filled = Board::of(ui).filled();
        (view.get_holding() || view.get_placed() != was.0 || filled != was.1).then(|| format!("a click off the board and the list with a piece in hand: in hand {}, {} placed (were {}), the board's cells {}", view.get_holding(), view.get_placed(), was.0, if filled == was.1 { "as they were" } else { "changed" }))
    }));
    shot(add, "tabs-navicust-put-back", Pair::None);
}

/// The pointer as the window has it, now, at the point `place` finds on the board as the view shows it (or any
/// point it names): the left button pressed and let go there (`click`), else the pointer moved there.
fn mouse_on(place: fn(&Board) -> Option<Point>, click: bool) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(position) = place(&Board::of(ui)).and_then(|p| locate(ui, app, p)) else {
            tour.key_wrong("the pointer on the NaviCust: no place for it (no piece on the board?)".to_string());
            return;
        };
        use slint::platform::{PointerEventButton as Button, WindowEvent as W};
        let events = if click { vec![W::PointerPressed { position, button: Button::Left }, W::PointerReleased { position, button: Button::Left }] } else { vec![W::PointerMoved { position }] };
        for event in events {
            ui.window().dispatch_event(event);
        }
    })
}

/// A patch card that brings a bug, put in as Enter on its library row does, where the build's cards bring none yet:
/// the first of the library's cards the build can take that brings one (by what the cards add up to: Together's
/// bugs, the build's without its NaviCust). One tried that brings none is taken out again. Its number is kept in
/// `put`, for `on_bug_card`.
fn bug_card(put: Rc<RefCell<Option<slint::SharedString>>>) -> Step {
    use nettai_ui::PatchCardsView;
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let view = ui.global::<PatchCardsView>();
        if !view.get_bugs_line().is_empty() {
            return;
        }
        let rows: Vec<nettai_ui::CardRow> = view.get_rows().iter().collect();
        let mut app = app.borrow_mut();
        for (i, row) in rows.iter().enumerate().filter(|(_, r)| !r.refused && r.note.is_empty()) {
            app.shell.nav.focus("lib.rows", i as i32);
            app.act(Action::Confirm);
            if !view.get_bugs_line().is_empty() {
                *put.borrow_mut() = Some(row.number.clone());
                return;
            }
            app.act(Action::Confirm);
        }
        tour.key_wrong("no card of the library that the build can take brings a bug".into());
    })
}

/// The cursor on the build's card `bug_card` put in.
fn on_bug_card(put: Rc<RefCell<Option<slint::SharedString>>>) -> Step {
    use nettai_ui::PatchCardsView;
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let cards = ui.global::<PatchCardsView>().get_installed();
        let Some(at) = cards.iter().position(|c| c.there && Some(&c.number) == put.borrow().as_ref()) else { return };
        app.borrow_mut().shell.nav.focus("card", at as i32);
    })
}

/// The bugs a build's patch cards bring are the Patch cards tab's (each card's bug lines, and Together's) and not
/// the NaviCust tab's, whose bugs are the NaviCust's own.
fn card_bugs(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, focus("card", 0));
    add(SETTLE, library());
    let put = Rc::new(RefCell::new(None));
    add(SETTLE, bug_card(put.clone()));
    add(SETTLE, out_of_library());
    add(SETTLE, on_bug_card(put));
    shot(add, "tabs-cards-bug", Pair::None);
    add(90, on_tab(EditorTab::PatchCards, "Patch cards"));
    add(SETTLE, to_tab(EditorTab::Navicust));
    add(SETTLE, focus("programs", 0));
    shot(add, "tabs-navicust-card-bug", Pair::None);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
}

/// The cursor on the first row of the NaviCust's list that `which` picks.
fn row(which: fn(&nettai_ui::ProgramRow) -> bool) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let rows = ui.global::<NavicustView>().get_programs();
        let Some(at) = rows.iter().position(|r| which(&r)) else { return };
        app.borrow_mut().shell.nav.focus("programs", at as i32);
    })
}

/// Where one of the motion probe's pointer events is: a cell of the NaviCust's drawn grid (by its column and row; a
/// fraction is between cells), a row of its list, off the board and the list (the effects' panel), or where the
/// piece in hand would fit nearest the board's middle (found when the event is due); or any control, by its key.
#[derive(Clone, Copy, Debug)]
enum Point {
    Cell(f32, f32),
    Row(i32),
    Off,
    Fit,
    Control(&'static str, i32),
    /// A control's middle, so far down from it (up, below 0).
    Below(&'static str, i32, f32),
    /// A list's (or a page's) view, so far down from its middle (up, below 0): wherever it is scrolled to.
    Middle(&'static str, f32),
    /// The first auto battle place shown that holds a chip.
    Filled,
    /// The box of color `color` (of `colors`) at program row `row`'s right edge.
    Color(i32, i32, i32),
}

/// One of the probe's pointer events: the left button pressed or let go, the pointer moved (no button held unless
/// one is), a right click, the wheel (its delta), or a key pressed and let go as the window's keys come.
#[derive(Clone, Copy, Debug)]
enum Ev {
    Press(Point),
    Release(Point),
    Move(Point),
    Right(Point),
    Wheel(Point, f32),
    Key(KeyCode),
}

/// The NaviCust's board as the view shows it, for a pointer script to find its pieces on.
struct Board {
    n: i32,
    cells: Vec<nettai_ui::GridCell>,
}

impl Board {
    fn of(ui: &nettai_ui::AppWindow) -> Board {
        let view = ui.global::<NavicustView>();
        Board { n: view.get_columns(), cells: view.get_cells().iter().collect() }
    }

    fn at(&self, x: i32, y: i32) -> Option<&nettai_ui::GridCell> {
        ((0..self.n).contains(&x) && (0..self.n).contains(&y)).then(|| &self.cells[(y * self.n + x) as usize])
    }

    /// The placed cells, in reading order.
    fn filled(&self) -> Vec<(i32, i32)> {
        (0..self.n * self.n).filter(|&i| self.cells[i as usize].filled).map(|i| (i % self.n, i / self.n)).collect()
    }

    /// A piece: the filled cells of one color joined to `start`.
    fn piece_of(&self, start: (i32, i32)) -> Vec<(i32, i32)> {
        let tone = self.at(start.0, start.1).map(|c| c.tone);
        let mut piece = vec![start];
        let mut i = 0;
        while i < piece.len() {
            let (x, y) = piece[i];
            for next in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                if self.at(next.0, next.1).is_some_and(|c| c.filled && Some(c.tone) == tone) && !piece.contains(&next) {
                    piece.push(next);
                }
            }
            i += 1;
        }
        piece
    }

    /// The first piece wholly on the board that has the room `ks` cells on one way, each: a cell of it, and the way.
    fn movable(&self, ks: &[i32]) -> Option<((i32, i32), (i32, i32))> {
        let board = |x: i32, y: i32| self.at(x, y).is_some_and(|c| c.kind == nettai_ui::CellKind::Board);
        self.filled().into_iter().find_map(|start| {
            let piece = self.piece_of(start);
            let room = |(dx, dy): (i32, i32), k: i32| piece.iter().all(|&(x, y)| board(x + k * dx, y + k * dy) && self.at(x + k * dx, y + k * dy).is_some_and(|c| !c.filled || piece.contains(&(x + k * dx, y + k * dy))));
            let way = [(-1, 0), (1, 0), (0, 1), (0, -1)].into_iter().find(|&way| ks.iter().all(|&k| room(way, k)))?;
            piece.iter().all(|&(x, y)| board(x, y)).then_some((start, way))
        })
    }
}

/// `(x, y)` gone `k` cells `way`, as a point.
fn cell_on((x, y): (i32, i32), (dx, dy): (i32, i32), k: f32) -> Point {
    Point::Cell(x as f32 + k * dx as f32, y as f32 + k * dy as f32)
}

/// Where the piece in hand fits nearest the drawn grid's middle (over no other, on the board, or jutting onto EXE6's
/// ring, which is no error): the cell it would be held by.
fn fit(ui: &nettai_ui::AppWindow) -> Option<(i32, i32)> {
    let board = Board::of(ui);
    let held: Vec<(i32, i32)> = ui.global::<NavicustView>().get_held().iter().map(|c| (c.x, c.y)).collect();
    let mid = board.n / 2;
    let on = |gx: i32, gy: i32, kind: nettai_ui::CellKind| held.iter().any(|&(dx, dy)| board.at(gx + dx, gy + dy).is_some_and(|c| c.kind == kind));
    let fits = |gx: i32, gy: i32| on(gx, gy, nettai_ui::CellKind::Board) && held.iter().all(|&(dx, dy)| board.at(gx + dx, gy + dy).is_some_and(|c| c.kind != nettai_ui::CellKind::None && !c.filled));
    (0..board.n).flat_map(|y| (0..board.n).map(move |x| (x, y))).filter(|&(x, y)| fits(x, y)).min_by_key(|&(x, y)| (x - mid).abs() + (y - mid).abs())
}

/// Where `point` is in the window, from where the view lays its controls out.
fn locate(ui: &nettai_ui::AppWindow, app: &Rc<RefCell<App>>, point: Point) -> Option<slint::LogicalPosition> {
    if let Point::Fit = point {
        let (x, y) = fit(ui)?;
        return locate(ui, app, Point::Cell(x as f32, y as f32));
    }
    let registry = app.borrow().shell.nav.registry();
    let registry = registry.borrow();
    let rect = |id: &str, i: i32| registry.rect(&crate::nav::Key::new(id, i));
    match point {
        Point::Cell(x, y) => {
            let n = Board::of(ui).n;
            // (Two cells side by side give the grid's pitch: EXE6's corners are no cells.)
            let (i, a, b) = (0..n * n).filter(|i| i % n + 1 < n).find_map(|i| Some((i, rect("cell", i)?, rect("cell", i + 1)?)))?;
            let pitch = b.x - a.x;
            let (x0, y0) = (a.x - (i % n) as f32 * pitch, a.y - (i / n) as f32 * pitch);
            Some(slint::LogicalPosition::new(x0 + x * pitch + a.w / 2.0, y0 + y * pitch + a.h / 2.0))
        }
        Point::Row(r) => rect("programs", r).map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0)),
        // (Each box 14 wide and 18 apart, the last 8 in from the row's right edge: navicust.slint's ProgramRowView.)
        Point::Color(row, color, colors) => rect("programs", row)
            .map(|r| slint::LogicalPosition::new(r.x + r.w - 8.0 - (colors * 18 - 4) as f32 + (color * 18) as f32 + 7.0, r.y + r.h / 2.0)),
        Point::Off => rect("programs", 0).map(|r| slint::LogicalPosition::new(r.x + r.w + 150.0, r.y + 200.0)),
        Point::Control(id, i) => rect(id, i).filter(|r| r.h > 0.0).map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0)),
        Point::Below(id, i, dy) => rect(id, i).filter(|r| r.h > 0.0).map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0 + dy)),
        Point::Middle(id, dy) => registry.view(id).filter(|r| r.h > 0.0).map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0 + dy)),
        Point::Filled => ui
            .global::<nettai_ui::AutoBattleView>()
            .get_lines()
            .iter()
            .filter(|l| !l.title && l.chip.there)
            .find_map(|l| rect("places.place", l.place).filter(|r| r.h > 0.0 && registry.view("places.").is_none_or(|v| r.y >= v.y && r.bottom() <= v.bottom())))
            .map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0)),
        Point::Fit => None,
    }
}

/// The pointer events `script` makes of the board as it is, each at its time (ms after the step), as the window's:
/// as Replays' bar is dragged.
fn pointer(script: impl Fn(&Board, &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> + 'static) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(events) = script(&Board::of(ui), ui) else {
            eprintln!("tour: the NaviCust's board has no piece for this burst's pointer");
            return;
        };
        for (ms, ev) in events {
            let (ui, app) = (ui.as_weak(), app.clone());
            slint::Timer::single_shot(std::time::Duration::from_millis(ms), move || {
                let Some(ui) = ui.upgrade() else { return };
                use slint::platform::{PointerEventButton as Button, WindowEvent as W};
                let point = match ev {
                    Ev::Key(code) => {
                        let mut app = app.borrow_mut();
                        app.key_of(code, true, None, false);
                        app.key_of(code, false, None, false);
                        return;
                    }
                    Ev::Press(p) | Ev::Release(p) | Ev::Move(p) | Ev::Right(p) | Ev::Wheel(p, _) => p,
                };
                let Some(position) = locate(&ui, &app, point) else {
                    eprintln!("tour: no place in the window for the pointer at {point:?}");
                    return;
                };
                let events = match ev {
                    Ev::Press(_) => vec![W::PointerPressed { position, button: Button::Left }],
                    Ev::Release(_) => vec![W::PointerReleased { position, button: Button::Left }],
                    Ev::Move(_) => vec![W::PointerMoved { position }],
                    Ev::Right(_) => vec![W::PointerPressed { position, button: Button::Right }, W::PointerReleased { position, button: Button::Right }],
                    Ev::Wheel(_, delta_y) => vec![W::PointerScrolled { position, delta_x: 0.0, delta_y }],
                    Ev::Key(_) => Vec::new(),
                };
                for event in events {
                    ui.window().dispatch_event(event);
                }
            });
        }
    })
}

/// A left click on control `id`'s `index`, then key `code` 300 ms on, as the window has them.
fn click_then(id: &'static str, index: i32, code: KeyCode) -> Step {
    let at = Point::Control(id, index);
    pointer(move |_, _| Some(vec![(0, Ev::Press(at)), (40, Ev::Release(at)), (300, Ev::Key(code))]))
}

/// A right click on the first placed piece of the board.
fn right_piece(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (x, y) = *b.filled().first()?;
    Some(vec![(0, Ev::Right(Point::Cell(x as f32, y as f32)))])
}

/// A right click on the first placed program's row.
fn right_row(_: &Board, ui: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let rows = ui.global::<NavicustView>().get_programs();
    let row = rows.iter().take(12).position(|r| r.state == nettai_ui::ProgramState::Placed)? as i32;
    Some(vec![(0, Ev::Right(Point::Row(row)))])
}

/// A placed piece pressed (it is taken up as the button goes down), moved two cells with the button held (it stays:
/// the held button carries nothing), let go there (it comes under the pointer), and clicked down there.
fn press_move(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, way) = b.movable(&[2])?;
    Some(vec![
        (0, Ev::Press(cell_on(s, way, 0.0))),
        (150, Ev::Move(cell_on(s, way, 0.5))),
        (200, Ev::Move(cell_on(s, way, 1.0))),
        (350, Ev::Move(cell_on(s, way, 1.5))),
        (400, Ev::Move(cell_on(s, way, 2.0))),
        (550, Ev::Release(cell_on(s, way, 2.0))),
        (700, Ev::Press(cell_on(s, way, 2.0))),
        (750, Ev::Release(cell_on(s, way, 2.0))),
    ])
}

/// A placed piece clicked (taken up), the pointer moved two cells with no button held, a key moving it a cell
/// across, the pointer moved back a cell (the piece comes back under it), and a click putting it down there.
fn click_move(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, way) = b.movable(&[1, 2])?;
    let key = if way.0 != 0 { if s.1 > 1 { KeyCode::ArrowUp } else { KeyCode::ArrowDown } } else if s.0 > 1 { KeyCode::ArrowLeft } else { KeyCode::ArrowRight };
    Some(vec![
        (0, Ev::Press(cell_on(s, way, 0.0))),
        (50, Ev::Release(cell_on(s, way, 0.0))),
        (200, Ev::Move(cell_on(s, way, 0.5))),
        (300, Ev::Move(cell_on(s, way, 1.0))),
        (420, Ev::Move(cell_on(s, way, 1.5))),
        (520, Ev::Move(cell_on(s, way, 2.0))),
        (680, Ev::Key(key)),
        (850, Ev::Move(cell_on(s, way, 1.0))),
        (1000, Ev::Press(cell_on(s, way, 1.0))),
        (1050, Ev::Release(cell_on(s, way, 1.0))),
    ])
}

/// A program's row clicked (the first not placed): a new copy in hand, carried onto the board with no button held,
/// and clicked down where it fits.
fn row_click(_: &Board, ui: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    click_out(ui, |r| r.state == nettai_ui::ProgramState::None && !r.plus)
}

/// A placed program's row clicked (the first placed): a new copy of it carried onto the board and clicked down where
/// it fits, the copy on the board staying where it is.
fn placed_row_click(_: &Board, ui: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    click_out(ui, |r| r.state == nettai_ui::ProgramState::Placed)
}

/// The first of the list's first rows that `which` picks clicked, the copy carried onto the board, and clicked down
/// where it fits.
fn click_out(ui: &nettai_ui::AppWindow, which: fn(&nettai_ui::ProgramRow) -> bool) -> Option<Vec<(u64, Ev)>> {
    let rows = ui.global::<NavicustView>().get_programs();
    let row = rows.iter().take(12).position(|r| which(&r))? as i32;
    let mid = Board::of(ui).n as f32 / 2.0 - 0.5;
    Some(vec![
        (0, Ev::Press(Point::Row(row))),
        (50, Ev::Release(Point::Row(row))),
        (150, Ev::Move(Point::Cell(mid + 2.0, mid))),
        (300, Ev::Move(Point::Fit)),
        (450, Ev::Press(Point::Fit)),
        (500, Ev::Release(Point::Fit)),
    ])
}

/// A placed piece clicked up, carried onto another and clicked there: it shakes and stays in hand; then a click off
/// the board puts it back where it was.
fn click_refused(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, _) = b.movable(&[])?;
    let piece = b.piece_of(s);
    let other = b.filled().into_iter().find(|c| !piece.contains(c))?;
    let at = cell_on(s, (0, 0), 0.0);
    let mid = Point::Cell((s.0 + other.0) as f32 / 2.0, (s.1 + other.1) as f32 / 2.0);
    let to = Point::Cell(other.0 as f32, other.1 as f32);
    Some(vec![
        (0, Ev::Press(at)),
        (50, Ev::Release(at)),
        (120, Ev::Move(mid)),
        (240, Ev::Move(to)),
        (360, Ev::Press(to)),
        (410, Ev::Release(to)),
        (700, Ev::Move(Point::Off)),
        (800, Ev::Press(Point::Off)),
        (850, Ev::Release(Point::Off)),
    ])
}

/// A placed piece clicked up, carried toward the middle and right-clicked: put away into the list (off the board).
fn click_right(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let at = cell_on(s, (0, 0), 0.0);
    let mid = Point::Cell(b.n as f32 / 2.0 - 0.5, b.n as f32 / 2.0 - 0.5);
    Some(vec![(0, Ev::Press(at)), (50, Ev::Release(at)), (150, Ev::Move(mid)), (300, Ev::Right(mid))])
}

/// A placed piece clicked up, carried off the board (the effects' panel) and clicked there: back where it was.
fn click_off(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let at = cell_on(s, (0, 0), 0.0);
    Some(vec![(0, Ev::Press(at)), (50, Ev::Release(at)), (150, Ev::Move(Point::Cell(b.n as f32, s.1 as f32))), (300, Ev::Move(Point::Off)), (450, Ev::Press(Point::Off)), (500, Ev::Release(Point::Off))])
}

/// A placed piece clicked up, carried over the list (the list's remove look fades in, the piece fades back) and
/// clicked there: put away into the list.
fn click_list(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let at = cell_on(s, (0, 0), 0.0);
    Some(vec![(0, Ev::Press(at)), (50, Ev::Release(at)), (150, Ev::Move(Point::Row(2))), (250, Ev::Move(Point::Row(1))), (450, Ev::Press(Point::Row(1))), (500, Ev::Release(Point::Row(1)))])
}

/// A placed piece clicked (taken up), turned twice by the wheel's notches, and right-clicked: put away.
fn click_wheel_right(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let at = cell_on(s, (0, 0), 0.0);
    Some(vec![(0, Ev::Press(at)), (50, Ev::Release(at)), (200, Ev::Wheel(at, -60.0)), (400, Ev::Wheel(at, -60.0)), (650, Ev::Right(at))])
}

/// Into the library from the build's cards (on a phone, its sheet), if the cursor isn't there; and back out.
pub(super) fn library() -> Step {
    Box::new(|_, app, tour| {
        if tour.passing.get() {
            return;
        }
        let mut app = app.borrow_mut();
        if !in_library(&app) {
            app.act(Action::Confirm);
        }
    })
}
pub(super) fn out_of_library() -> Step {
    Box::new(|_, app, tour| {
        if tour.passing.get() {
            return;
        }
        let mut app = app.borrow_mut();
        if in_library(&app) {
            app.act(Action::Back);
        }
    })
}
fn in_library(app: &App) -> bool {
    app.shell.nav.is_open("lib.") || app.shell.nav.focused().is_some_and(|k| k.id.starts_with("lib."))
}

/// The Patch cards tab's steps.
fn patch_cards(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, focus("card", 0));
    shot(add, "tabs-cards", Pair::None);
    add(90, on_tab(EditorTab::PatchCards, "Patch cards"));
    // The library, its controls a focus move apart; three cards put in, the cursor staying in the library.
    add(SETTLE, library());
    shot(add, "tabs-cards-library", Pair::None);
    add(SETTLE, focus("lib.search", 0));
    shot(add, "tabs-cards-search", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "tabs-cards-search-focus", Pair::Second);
    // The library scrolled down, then a search typed: what it finds is shown from its top; the search cleared.
    add(SETTLE, focus("lib.rows", 40));
    add(SETTLE, screen(|s, shell| s.edit(shell, "lib.search", 0, "e", false)));
    shot(add, "tabs-cards-search-top", Pair::None);
    add(SETTLE, screen(|s, shell| s.edit(shell, "lib.search", 0, "", true)));
    for row in 0..3 {
        add(SETTLE, focus("lib.rows", row));
        add(SETTLE, act(Action::Confirm));
    }
    shot(add, "tabs-cards-put", Pair::None);
    // The cursor on a card that is in (its hint: Remove), and Enter taking it out from there; put back in.
    add(SETTLE, focus("lib.rows", 2));
    shot(add, "tabs-cards-on-installed", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-cards-taken-out", Pair::None);
    add(SETTLE, act(Action::Confirm));
    // The library by MB (in a game with slots, by slot), the most first: a card the MB left can't take, refused
    // and why; in a game with slots, a card put in its slot in place of the one there.
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 2)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 0));
    shot(add, "tabs-cards-sorted", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-cards-refused", Pair::None);
    // A card of the build's under the cursor, moved up, taken out.
    add(SETTLE, out_of_library());
    add(SETTLE, focus("card", 1));
    shot(add, "tabs-cards-installed", Pair::None);
    add(90, on_tab(EditorTab::PatchCards, "Patch cards"));
    add(SETTLE, key(KeyCode::BracketLeft, "["));
    shot(add, "tabs-cards-moved", Pair::None);
    add(SETTLE, act(Action::Delete));
    shot(add, "tabs-cards-removed", Pair::None);
    // The sort's list.
    add(SETTLE, library());
    add(SETTLE, press("lib.sort", 0));
    shot(add, "tabs-cards-sort", Pair::None);
    add(SETTLE, act(Action::Back));
    add(SETTLE, out_of_library());
}

/// The Auto battle tab's steps (a game whose setup takes no auto battle data has no such tab: they show what
/// `on_tab` reports).
fn auto_battle(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::AutoBattle));
    add(SETTLE, focus("places.place", 0));
    shot(add, "tabs-auto", Pair::First);
    add(90, on_tab(EditorTab::AutoBattle, "Auto battle"));
    add(SETTLE, act(Action::Down));
    shot(add, "tabs-auto-focus", Pair::Second);
    // Three places filled from the library, the cursor going on to the next each time; a place emptied.
    add(SETTLE, focus("places.place", 0));
    for row in 0..3 {
        add(SETTLE, library());
        add(SETTLE, focus("lib.rows", row * 4));
        add(SETTLE, act(Action::Confirm));
    }
    shot(add, "tabs-auto-filled", Pair::None);
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 1));
    shot(add, "tabs-auto-library", Pair::None);
    add(SETTLE, out_of_library());
    add(SETTLE, focus("places.place", 1));
    add(SETTLE, act(Action::Delete));
    shot(add, "tabs-auto-emptied", Pair::None);
    // The places further down: the page scrolled to the last list.
    add(SETTLE, focus("places.place", 41));
    shot(add, "tabs-auto-last", Pair::None);
    // The library of a place in each list the rules hold to one class of chip: the Mega chips, the Giga chips, the
    // Program Advances (which no folder holds); a Program Advance put in its place.
    add(SETTLE, out_of_library());
    add(SETTLE, to_held(ChipClass::Mega));
    shot(add, "tabs-auto-mega", Pair::None);
    add(SETTLE, out_of_library());
    add(SETTLE, to_held(ChipClass::Giga));
    shot(add, "tabs-auto-giga", Pair::None);
    add(SETTLE, out_of_library());
    add(SETTLE, to_held(ChipClass::ProgramAdvance));
    shot(add, "tabs-auto-advance", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-auto-advance-put", Pair::None);
}

/// The first place of the auto battle data's list that the rules hold to chips of `class`, from 0 (none: the game
/// has no such list).
fn held(app: &App, class: ChipClass) -> Option<usize> {
    let kit = crate::builds::kit(&app.shell)?;
    kit.auto_layout()?.lists.iter().find(|l| l.holds == Holds::Chips(class)).map(|l| l.start)
}

/// Enter on the first place of the list held to chips of `class`: its library (on a phone, its sheet). A game with
/// no such list: the report says so.
fn to_held(class: ChipClass) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let mut app = app.borrow_mut();
        let Some(place) = held(&app, class) else {
            ui.global::<nettai_ui::Audit>().invoke_clipped(format!("(the game's auto battle data has no list of {class:?} chips: this picture is of something else)").into());
            return;
        };
        app.shell.nav.focus("places.place", place as i32);
        app.act(Action::Confirm);
    })
}

/// A build whose auto battle data holds, in each list the rules hold to one class of chip, a chip of another (as a
/// build file or a save made by hand may): Play's list counts its problems, and in the editor its tab is
/// flagged, the places are marked, each saying why at its end. (A new build of the game's first version, beside
/// the walk's.)
fn wrong_classes(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    const NAME: &str = "Wrong places";
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            if tour.passing.get() {
                return;
            }
            let mut app = app.borrow_mut();
            let Some(kit) = crate::builds::kit(&app.shell) else { return };
            let content = kit.content().clone();
            let Some(side) = kit.builds.groups.first().and_then(|v| kit.builds.new_build(&content, &v.id).ok()) else { return };
            let mut build = store::Build::new(&content, NAME, side);
            let folder = build.side.folder(&content);
            for list in kit.auto_layout().map(|l| l.lists.clone()).unwrap_or_default() {
                let Holds::Chips(class) = list.holds else { continue };
                let Some(chip) = folder.chips().map(|c| c.id).find(|&c| content.chip(c).class != class) else { continue };
                auto::update(&content, &mut build.side, auto::Edit::Put(list.start, chip));
            }
            let _ = store::write(&store::new_path(&kit.game, NAME), &content, &kit.game, &build);
            leave(&mut app);
            app.go_now(ScreenId::Play);
            super::builds::choose(&mut app, NAME);
        }),
    );
    shot(add, "tabs-auto-wrong-list", Pair::None);
    add(SETTLE, press("edit", 0));
    add(SETTLE, to_tab(EditorTab::AutoBattle));
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            if tour.passing.get() {
                return;
            }
            let mut app = app.borrow_mut();
            if let Some(place) = held(&app, ChipClass::Mega) {
                app.shell.nav.focus("places.place", place as i32);
            }
        }),
    );
    shot(add, "tabs-auto-wrong", Pair::None);
}

/// A build whose NaviCust has two programs over each other (the walk's own build's first program placed again where it
/// is, as a build file made by hand may have it: the editor never puts one so): on the NaviCust tab the program's row
/// says what is wrong (docs/design/app-ui.md §2.66), the tab's word marked (`tabs-navicust-overlap`). (A build of its own,
/// beside the walk's.)
fn overlap(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    const NAME: &str = "Overlap";
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            // (Whatever tab the steps before passed by: this build is every game's.)
            tour.passing.set(false);
            let mut app = app.borrow_mut();
            let Some(kit) = crate::builds::kit(&app.shell) else { return };
            let content = kit.content().clone();
            let Some(listed) = store::list(&kit.game).into_iter().find(|l| l.name == "Tab tour") else { return };
            let Ok(walked) = store::read(&content, &kit.game, &listed.text) else { return };
            let mut build = store::Build::new(&content, NAME, walked.side.clone());
            let mut parts = crate::builds::grid::placed(&content, &build.side);
            let Some(first) = parts.first().cloned() else { return };
            parts.push(first);
            crate::builds::grid::set_placed(&content, &mut build.side, &parts);
            let _ = store::write(&store::new_path(&kit.game, NAME), &content, &kit.game, &build);
            leave(&mut app);
            app.go_now(ScreenId::Play);
            super::builds::choose(&mut app, NAME);
        }),
    );
    add(SETTLE, press("edit", 0));
    add(SETTLE, to_tab(EditorTab::Navicust));
    add(SETTLE, focus("programs", 0));
    shot(add, "tabs-navicust-overlap", Pair::None);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));
}

/// A build of the game's first version given the other version's variant of a card that has one per version
/// (Bass-Cross MegaMan's, EXE5's teams'), which a ruleset of one version per build refuses (the Masters class preset's):
/// on the Patch cards tab the card's name in the error red and the strip of the card focused saying the mix
/// (`tabs-cards-mixed`). (A build of its own.)
fn card_mix(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    const NAME: &str = "Card mix";
    add(
        SETTLE,
        Box::new(|_, app, tour| {
            tour.passing.set(false);
            let mut app = app.borrow_mut();
            let Some(kit) = crate::builds::kit(&app.shell) else { return };
            let content = kit.content().clone();
            let Some(side) = kit.builds.groups.first().and_then(|v| kit.builds.new_build(&content, &v.id).ok()) else { return };
            let build = store::Build::new(&content, NAME, side);
            let _ = store::write(&store::new_path(&kit.game, NAME), &content, &kit.game, &build);
            leave(&mut app);
            app.go_now(ScreenId::Play);
            super::builds::choose(&mut app, NAME);
        }),
    );
    add(SETTLE, press("edit", 0));
    add(SETTLE, to_tab(EditorTab::PatchCards));
    // (A card of a version's variant put in from the library's row, as Enter there does; where it is the build's own
    // version's, the other's in its place. A tab with no cards has none to focus: Enter on the bar's word would go
    // nowhere it means.)
    add(SETTLE, put_variant(0));
    add(SETTLE, put_variant(1));
    add(SETTLE, out_of_library());
    add(SETTLE, focus("card", 0));
    shot(add, "tabs-cards-mixed", Pair::None);
    add(90, on_tab(EditorTab::PatchCards, "Patch cards"));
}

/// The library's `nth` card of a version's variant (a row with a version's mark) put in, unless the build mixes the
/// versions already (a card of the build's has its name in the error red).
fn put_variant(nth: usize) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() || (nth > 0 && ui.global::<nettai_ui::PatchCardsView>().get_installed().iter().any(|r| r.bad)) {
            return;
        }
        let rows = ui.global::<nettai_ui::PatchCardsView>().get_rows();
        let Some(at) = rows.iter().enumerate().filter(|(_, r)| !r.version.text.is_empty()).nth(nth).map(|(i, _)| i) else { return };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("lib.rows", at as i32);
        app.act(Action::Confirm);
    })
}

/// A left click on control `id`'s `index`, as the window has one (pressed and let go there).
fn click(id: &'static str, index: i32) -> Step {
    click_at(Point::Control(id, index))
}
fn click_at(point: Point) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(position) = locate(ui, app, point) else {
            tour.key_wrong(format!("a click on {point:?}: it isn't on the screen"));
            return;
        };
        use slint::platform::{PointerEventButton as Button, WindowEvent as W};
        ui.window().dispatch_event(W::PointerPressed { position, button: Button::Left });
        ui.window().dispatch_event(W::PointerReleased { position, button: Button::Left });
    })
}

/// The pointer moved onto control `id`'s `index`, no button held, as the window has it (the control is lit under it;
/// a row that describes what it holds is the strip's). The pointer is the last thing used, as a mouse's move makes it
/// (the pass takes no real pointer: `App::pointer`).
fn hover(id: &'static str, index: i32) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(position) = locate(ui, app, Point::Control(id, index)) else {
            tour.key_wrong(format!("the pointer moved onto {id} {index}: it isn't on the screen"));
            return;
        };
        app.borrow_mut().shell.nav.used_pointer();
        ui.window().dispatch_event(slint::platform::WindowEvent::PointerMoved { position });
    })
}

/// A program of the NaviCust's list with colors to choose from (each program is in the list once: the user, "deduppe
/// navicust program parts in the list and then have you select by color"): the cursor on its row (the footer's
/// Color), Right choosing its second color (the row's and the strip's), and Enter taking that color's part in hand,
/// put away after; then a click on its last color's box, which chooses it alone (`color-clicked`).
fn program_colors(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::Navicust));
    // (The row, and how many colors it has.)
    let at: Rc<RefCell<(i32, i32)>> = Rc::new(RefCell::new((-1, 0)));
    let kept = at.clone();
    add(SETTLE, Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let rows = ui.global::<NavicustView>().get_programs();
        let Some(at) = rows.iter().position(|r| r.colors.row_count() > 1) else { return tour.key_wrong("no program has two colors".into()) };
        *kept.borrow_mut() = (at as i32, rows.row_data(at).map_or(0, |r| r.colors.row_count() as i32));
        app.borrow_mut().shell.nav.focus("programs", at as i32);
    }));
    shot(add, "tabs-navicust-colors", Pair::None);
    add(SETTLE, act(Action::Right));
    shot(add, "tabs-navicust-color-chosen", Pair::None);
    add(SETTLE, expect(|ui, app| {
        let at = app.shell.nav.focused().filter(|k| k.id == "programs").map(|k| k.index)?;
        let row = ui.global::<NavicustView>().get_programs().row_data(at as usize)?;
        let detail = ui.global::<NavicustView>().get_detail();
        (row.color != 1 || detail.color != 1).then(|| format!("Right on a program's row of colors: its color is {}, the strip's {}, not the second", row.color, detail.color))
    }));
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-color-held", Pair::None);
    add(SETTLE, act(Action::Back));
    // A click on the row's last color: it is chosen, and nothing taken (the user: "selecting a color on the palette
    // shouldn't pick up the navicust part actually").
    for (wait, ev) in [(SETTLE, Ev::Press as fn(Point) -> Ev), (40, Ev::Release)] {
        let at = at.clone();
        add(wait, hand_at(move |_, _| {
            let (row, colors) = *at.borrow();
            (row >= 0 && colors > 1).then_some(Point::Color(row, colors - 1, colors))
        }, ev));
    }
    shot(add, "tabs-navicust-color-clicked", Pair::None);
    add(SETTLE, expect(move |ui, _| {
        let (row, colors) = *at.borrow();
        let view = ui.global::<NavicustView>();
        let chosen = view.get_programs().row_data(row.max(0) as usize).map(|r| r.color);
        (view.get_holding() || chosen != Some(colors - 1)).then(|| format!("a click on row {row}'s last color: holding {}, its color {chosen:?}", view.get_holding()))
    }));
    // (Put away: Back with nothing in hand would leave the editor.)
    add(SETTLE, Box::new(|ui, app, _| {
        if ui.global::<NavicustView>().get_holding() {
            app.borrow_mut().act(Action::Back);
        }
    }));
}

/// The pointer gone out of the window: nothing is lit under it.
fn pointer_gone() -> Step {
    Box::new(move |ui, _, tour| {
        if !tour.passing.get() {
            ui.window().dispatch_event(slint::platform::WindowEvent::PointerExited);
        }
    })
}

/// The pointer on a row describes what it holds in the strip, the focus staying where it is (docs/design/app-ui.md
/// §2.64): a chip of the folder's library, a program of the NaviCust's list, a card of the patch cards' library and
/// one of the build's (the cursor on the empty row after it); then the pointer gone, the strip the focused row's
/// again (a desktop's: a phone has no hover).
fn hovers(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    use nettai_ui::{FolderView, PatchCardsView};
    add(SETTLE, to_tab(EditorTab::Folder));
    add(SETTLE, focus("slot", 0));
    add(SETTLE, hover("lib.rows", 4));
    shot(add, "tabs-hover-chip", Pair::None);
    add(SETTLE, expect(|ui, app| {
        let (detail, row) = (ui.global::<FolderView>().get_detail(), ui.global::<FolderView>().get_rows().row_data(4));
        focus_on(app, "slot", 0).or_else(|| (Some(&detail.name) != row.as_ref().map(|r| &r.name)).then(|| format!("the pointer on the library's fifth chip: the strip shows {:?}", detail.name)))
    }));
    add(SETTLE, pointer_gone());
    add(SETTLE, expect(|ui, _| {
        let (detail, slot) = (ui.global::<FolderView>().get_detail(), ui.global::<FolderView>().get_slots().row_data(0));
        (Some(&detail.name) != slot.as_ref().map(|s| &s.name)).then(|| format!("the pointer gone: the strip shows {:?}, not the first entry's", detail.name))
    }));
    add(SETTLE, to_tab(EditorTab::Navicust));
    add(SETTLE, focus("programs", 0));
    add(SETTLE, hover("programs", 3));
    shot(add, "tabs-hover-program", Pair::None);
    add(SETTLE, expect(|ui, app| {
        let (detail, row) = (ui.global::<NavicustView>().get_detail(), ui.global::<NavicustView>().get_programs().row_data(3));
        focus_on(app, "programs", 0).or_else(|| (Some(&detail.name) != row.as_ref().map(|r| &r.name)).then(|| format!("the pointer on the fourth program: the strip shows {:?}", detail.name)))
    }));
    add(SETTLE, pointer_gone());
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, out_of_library());
    add(SETTLE, focus("card", 0));
    add(SETTLE, hover("lib.rows", 6));
    shot(add, "tabs-hover-card", Pair::None);
    add(SETTLE, expect(|ui, app| {
        let (detail, row) = (ui.global::<PatchCardsView>().get_detail(), ui.global::<PatchCardsView>().get_rows().row_data(6));
        focus_on(app, "card", 0).or_else(|| (Some(&detail.number) != row.as_ref().map(|r| &r.number)).then(|| format!("the pointer on the library's seventh card: the strip shows {:?}", detail.name)))
    }));
    // (The cursor on the empty row after the build's cards, the pointer on the first.)
    add(SETTLE, focus("card", 1));
    add(SETTLE, hover("card", 0));
    shot(add, "tabs-hover-installed", Pair::None);
    add(SETTLE, pointer_gone());
}

/// A key pressed and let go as the window has it (`App::key_of`, the window's hook).
fn press_key(code: KeyCode, text: &'static str) -> Step {
    Box::new(move |_, app, tour| {
        if tour.passing.get() {
            return;
        }
        let typed = (!text.is_empty()).then(|| text.to_string());
        app.borrow_mut().key_of(code, true, typed, false);
        app.borrow_mut().key_of(code, false, None, false);
    })
}

/// A check of what a click and the keys after it did: `wrong` says what isn't so, if anything (the pass reports it
/// with the keys that didn't do what they do, and fails).
fn expect(wrong: impl Fn(&nettai_ui::AppWindow, &App) -> Option<String> + 'static) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        if let Some(said) = wrong(ui, &app.borrow()) {
            tour.key_wrong(said);
        }
    })
}

/// The focus is on control `id`'s `index`, else what it is on.
fn focus_on(app: &App, id: &str, index: i32) -> Option<String> {
    let at = app.shell.nav.focused();
    (at.as_ref().map(|k| (k.id.as_str(), k.index)) != Some((id, index))).then(|| format!("the focus is on {at:?}, not {id} {index}"))
}

/// A click on one of the build's rows takes the focus there, so the keys after it act on what was clicked: in the
/// folder, R makes the clicked entry the Regular chip, T tags another clicked one, Down goes on from it, and Del
/// empties the entry the cursor came to; a patch card clicked and Del (it is taken out), and Down from one; an
/// auto battle place clicked and Del (it is emptied). Each tab's first click is shot with the pointer gone on to
/// another row: the cursor is hidden (the pointer was the last thing used), and the row clicked has the chosen
/// row's mark, the row under the pointer its hover. (A desktop's: a phone's click opens the library's sheet.)
fn clicks(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    use nettai_ui::{AutoBattleView, ChipMark, FolderView, PatchCardsView};
    add(SETTLE, to_tab(EditorTab::Folder));
    add(SETTLE, focus("slot", 0));
    add(SETTLE, click("slot", 3));
    add(SETTLE, hover("slot", 7));
    shot(add, "tabs-folder-clicked", Pair::None);
    add(SETTLE, press_key(KeyCode::KeyR, "r"));
    add(SETTLE, expect(|ui, app| {
        let mark = ui.global::<FolderView>().get_slots().row_data(3).map(|s| s.mark);
        focus_on(app, "slot", 3).or_else(|| (mark != Some(ChipMark::Regular)).then(|| format!("R after a click on entry 4: its mark is {mark:?}, not Regular")))
    }));
    // (T on two entries makes them the tag chips, where the game has them: each clicked first.)
    add(SETTLE, click("slot", 4));
    add(SETTLE, press_key(KeyCode::KeyT, "t"));
    add(SETTLE, click("slot", 5));
    add(SETTLE, press_key(KeyCode::KeyT, "t"));
    add(SETTLE, expect(|ui, app| {
        let view = ui.global::<FolderView>();
        let mark = |i: usize| view.get_slots().row_data(i).map(|s| s.mark);
        let tagged = |i: usize| matches!(mark(i), Some(ChipMark::Tag1 | ChipMark::Tag2));
        let wrong = view.get_counts().tags && !(tagged(4) && tagged(5));
        focus_on(app, "slot", 5).or_else(|| wrong.then(|| format!("T after clicks on entries 5 and 6: their marks are {:?} and {:?}, not the tags", mark(4), mark(5))))
    }));
    add(SETTLE, press_key(KeyCode::ArrowDown, ""));
    add(SETTLE, expect(|_, app| focus_on(app, "slot", 6).map(|s| format!("Down after a click on entry 6: {s}"))));
    add(SETTLE, press_key(KeyCode::Delete, ""));
    add(SETTLE, expect(|ui, _| {
        let there = ui.global::<FolderView>().get_slots().row_data(6).map(|s| s.there);
        (there != Some(false)).then(|| "Del after a click and Down: entry 7 isn't emptied".to_string())
    }));
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, out_of_library());
    fn cards(ui: &nettai_ui::AppWindow) -> usize {
        ui.global::<PatchCardsView>().get_installed().iter().filter(|c| c.there).count()
    }
    add(SETTLE, click("card", 1));
    add(SETTLE, expect(|_, app| focus_on(app, "card", 1).map(|s| format!("a click on the build's second card: {s}"))));
    add(SETTLE, hover("lib.rows", 1));
    shot(add, "tabs-cards-clicked", Pair::None);
    add(SETTLE, press_key(KeyCode::ArrowUp, ""));
    add(SETTLE, expect(|_, app| focus_on(app, "card", 0).map(|s| format!("Up after a click on the second card: {s}"))));
    let before = Rc::new(std::cell::Cell::new(0usize));
    let kept = before.clone();
    add(SETTLE, Box::new(move |ui, _, _| kept.set(cards(ui))));
    // (A click on the second row that holds a card, the cursor gone up to the first row: it is marked, and Del takes
    // it out. In a list it is the second row; in EXE4's slots, the second slot that holds one, an empty slot's Del
    // having nothing to take out.)
    fn second_card(ui: &nettai_ui::AppWindow) -> i32 {
        ui.global::<PatchCardsView>().get_installed().iter().enumerate().filter(|(_, c)| c.there).nth(1).map_or(1, |(i, _)| i as i32)
    }
    let clicked = Rc::new(std::cell::Cell::new(1));
    let chosen = clicked.clone();
    add(SETTLE, Box::new(move |ui, app, tour| {
        chosen.set(second_card(ui));
        click_at(Point::Control("card", chosen.get()))(ui, app, tour)
    }));
    add(SETTLE, press_key(KeyCode::Delete, ""));
    add(SETTLE, expect(move |ui, app| {
        let was = before.get();
        let at = clicked.get();
        focus_on(app, "card", at).or_else(|| (cards(ui) + 1 != was).then(|| format!("Del after a click on the second card (row {}): {} cards, not {}", at + 1, cards(ui), was.saturating_sub(1))))
    }));
    add(SETTLE, to_tab(EditorTab::AutoBattle));
    add(SETTLE, focus("places.place", 2));
    add(SETTLE, click("places.place", 0));
    add(SETTLE, hover("places.place", 3));
    shot(add, "tabs-auto-clicked", Pair::None);
    add(SETTLE, press_key(KeyCode::Delete, ""));
    add(SETTLE, expect(|ui, app| {
        let there = ui.global::<AutoBattleView>().get_lines().iter().find(|l| !l.title && l.place == 0).map(|l| l.chip.there);
        focus_on(app, "places.place", 0).or_else(|| (there != Some(false)).then(|| "Del after a click on place 1: it isn't emptied".to_string()))
    }));
}

/// Row `index` of `id` pressed, dragged `by` rows down (up, below 0) a row at a time, and let go there.
fn drag_rows(id: &'static str, index: i32, by: i32) -> Vec<(u64, Ev)> {
    let way = by.signum();
    let mut events = vec![(0, Ev::Press(Point::Control(id, index))), (60, Ev::Move(Point::Below(id, index, 8.0 * way as f32)))];
    for k in 1..=by.abs() {
        events.push((60 + 80 * k as u64, Ev::Move(Point::Control(id, index + k * way))));
    }
    events.push((60 + 80 * by.unsigned_abs() as u64 + 120, Ev::Release(Point::Control(id, index + by))));
    events
}

/// The pointer as the window has it, now: the left button pressed at `point`, moved there with it held, or let go.
fn hand(point: Point, ev: fn(Point) -> Ev) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(position) = locate(ui, app, point) else {
            tour.key_wrong(format!("the pointer at {point:?}: it isn't on the screen"));
            return;
        };
        use slint::platform::{PointerEventButton as Button, WindowEvent as W};
        let event = match ev(point) {
            Ev::Press(_) => W::PointerPressed { position, button: Button::Left },
            Ev::Release(_) => W::PointerReleased { position, button: Button::Left },
            _ => W::PointerMoved { position },
        };
        ui.window().dispatch_event(event);
    })
}

/// Row `index` of `id` pressed and dragged to row `to` (its middle `dy` down from that row's), a row at a time, held
/// there: `let_go` lets it go.
fn drag_to(add: &mut dyn FnMut(u64, Step), id: &'static str, index: i32, to: i32, dy: f32) {
    add(SETTLE, hand(Point::Control(id, index), Ev::Press));
    let way = (to - index).signum();
    add(60, hand(Point::Below(id, index, 8.0 * if way == 0 { 1.0 } else { way as f32 }), Ev::Move));
    let mut at = index;
    while at != to {
        at += way;
        add(60, hand(Point::Control(id, at), Ev::Move));
    }
    add(60, hand(Point::Below(id, to, dy), Ev::Move));
}
fn let_go(id: &'static str, to: i32, dy: f32) -> Step {
    hand(Point::Below(id, to, dy), Ev::Release)
}

/// The row at `from` pressed and dragged onto the library beside its list, held over the middle of its rows (wherever
/// it is scrolled to): `drop_on_library()` lets it go there.
fn onto_library(add: &mut dyn FnMut(u64, Step), from: Point) {
    add(SETTLE, hand(from, Ev::Press));
    add(60, hand(Point::Middle("lib.rows", 60.0), Ev::Move));
    add(60, hand(Point::Middle("lib.rows", 0.0), Ev::Move));
}
fn drop_on_library() -> Step {
    hand(Point::Middle("lib.rows", 0.0), Ev::Release)
}

/// The library's row a drag into the build's list takes, kept for the steps after it: its place, and what it shows
/// (its name, and a patch card's slot).
#[derive(Clone, Default)]
struct Picked {
    row: i32,
    name: String,
    slot: String,
}

/// The first of the library's rows `which` finds (its place, name and slot), brought into view (the focus on it), and
/// kept in `picked`; none found, the report says so.
fn pick(picked: &Rc<RefCell<Picked>>, which: impl Fn(&nettai_ui::AppWindow) -> Option<Picked> + 'static) -> Step {
    let picked = picked.clone();
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(found) = which(ui) else {
            *picked.borrow_mut() = Picked { row: -1, ..Default::default() };
            tour.key_wrong("no library row to drag in".to_string());
            return;
        };
        app.borrow_mut().shell.nav.focus("lib.rows", found.row);
        *picked.borrow_mut() = found;
    })
}

/// The pointer as the window has it, now, at the point `point` says (none: the report says so).
fn hand_at(point: impl Fn(&nettai_ui::AppWindow, &App) -> Option<Point> + 'static, ev: fn(Point) -> Ev) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(at) = point(ui, &app.borrow()) else {
            tour.key_wrong("the pointer has no place to go".to_string());
            return;
        };
        hand(at, ev)(ui, app, tour)
    })
}

/// The library's row `picked` pressed and dragged to `to`, held there: `hand_at(to, Ev::Release)` lets it go.
fn drag_in(add: &mut dyn FnMut(u64, Step), picked: &Rc<RefCell<Picked>>, to: fn(&nettai_ui::AppWindow, &App) -> Option<Point>) {
    let row = picked.clone();
    add(SETTLE, hand_at(move |_, _| Some(Point::Control("lib.rows", row.borrow().row)), Ev::Press));
    let row = picked.clone();
    add(60, hand_at(move |_, _| Some(Point::Below("lib.rows", row.borrow().row, 8.0)), Ev::Move));
    add(60, hand(Point::Middle("lib.rows", 0.0), Ev::Move));
    add(60, hand_at(to, Ev::Move));
    add(60, hand_at(to, Ev::Move));
}

/// The first of the build's patch cards (or slots) that holds a card.
fn first_card(ui: &nettai_ui::AppWindow, _: &App) -> Option<Point> {
    ui.global::<nettai_ui::PatchCardsView>().get_installed().iter().position(|c| c.there).map(|i| Point::Control("card", i as i32))
}

/// A right click on control `id`'s `index`, as the window has it.
fn right(id: &'static str, index: i32) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(position) = locate(ui, app, Point::Control(id, index)) else {
            tour.key_wrong(format!("a right click on {id} {index}: it isn't on the screen"));
            return;
        };
        use slint::platform::{PointerEventButton as Button, WindowEvent as W};
        ui.window().dispatch_event(W::PointerPressed { position, button: Button::Right });
        ui.window().dispatch_event(W::PointerReleased { position, button: Button::Right });
    })
}

/// What the build's lists hold, as their views show them: the folder's entries by name, the patch cards by name, the
/// auto battle places by name (by place).
fn names(ui: &nettai_ui::AppWindow) -> (Vec<String>, Vec<String>, Vec<(i32, String)>) {
    use nettai_ui::{AutoBattleView, FolderView, PatchCardsView};
    let folder = ui.global::<FolderView>().get_slots().iter().map(|s| s.name.to_string()).collect();
    let cards = ui.global::<PatchCardsView>().get_installed().iter().filter(|c| c.there).map(|c| c.name.to_string()).collect();
    let places = ui.global::<AutoBattleView>().get_lines().iter().filter(|l| !l.title).map(|l| (l.place, l.chip.name.to_string())).collect();
    (folder, cards, places)
}

/// The mouse on the build's lists (docs/app.md §8): clicks on a folder entry, a double click and one more, leave its
/// chip in (a click removes nothing); an entry dragged three rows down (shot as it is held there, the rows between
/// shifted up, and let go: it is there, those between one up); a patch card dragged above the one before it; an auto
/// battle place's chip dragged two places back in its list (those between shift on), then into another list's place
/// (the two trade), then into the Mega list, which the rules refuse it (it shakes, and nothing changes). On a phone,
/// the folder's drag alone, with the mouse (the folder is a list of its own there; the pass has no touches to give).
fn drags(add: &mut dyn FnMut(u64, Step), shot: Shot, phone: bool) {
    use nettai_ui::FolderView;
    let kept: Rc<RefCell<(Vec<String>, Vec<String>, Vec<(i32, String)>)>> = Rc::default();
    let keep = |kept: &Rc<RefCell<_>>| -> Step {
        let kept = kept.clone();
        Box::new(move |ui, _, _| *kept.borrow_mut() = names(ui))
    };
    add(SETTLE, to_tab(EditorTab::Folder));
    add(SETTLE, out_of_library());
    if !phone {
        // Clicks remove nothing ("remove double clicking to remove", "remove that too"): a double click on an entry
        // marks it, and a click on it marked leaves its chip in.
        add(SETTLE, focus("slot", 0));
        add(SETTLE, click("slot", 2));
        add(40, click("slot", 2));
        add(600, click("slot", 2));
        add(SETTLE, expect(|ui, app| {
            let there = ui.global::<FolderView>().get_slots().row_data(2).map(|s| s.there);
            focus_on(app, "slot", 2).or_else(|| (there != Some(true)).then(|| "clicks on entry 3 took its chip out".to_string()))
        }));
        add(SETTLE, hover("slot", 9));
        shot(add, "tabs-folder-clicked-twice", Pair::None);
    }
    // An entry dragged three rows down, held there: the rows between have shifted up, a gap where it would land.
    add(SETTLE, keep(&kept));
    drag_to(add, "slot", 5, 8, 0.0);
    shot(add, "tabs-folder-drag", Pair::None);
    add(SETTLE, let_go("slot", 8, 0.0));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).0;
        let was = &was.borrow().0;
        let want: Vec<&String> = [6, 7, 8, 5].iter().filter_map(|&i| was.get(i)).collect();
        let got: Vec<&String> = now.iter().skip(5).take(4).collect();
        (got != want).then(|| format!("entry 6 dragged to 9: entries 6 to 9 are {got:?}, not {want:?}"))
    }));
    shot(add, "tabs-folder-dragged", Pair::None);
    if phone {
        return;
    }
    // An entry dragged back onto the library ("drag it back to where it came from"), shot held there (the remove
    // look: the entry's line red and faded, the library washed under DROP TO REMOVE), and let go: its chip is taken
    // out, the entries after it where they were.
    add(SETTLE, keep(&kept));
    onto_library(add, Point::Control("slot", 10));
    shot(add, "tabs-folder-drag-remove", Pair::None);
    add(SETTLE, drop_on_library());
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).0;
        let was = &was.borrow().0;
        let there = ui.global::<FolderView>().get_slots().row_data(10).map(|s| s.there);
        let held = was.get(10).is_some_and(|n| !n.is_empty());
        (!held || there != Some(false) || now.get(11) != was.get(11)).then(|| format!("entry 11 ({:?}) dragged onto the library: it holds {there:?} (not emptied), or entry 12 changed ({:?})", was.get(10), now.get(11)))
    }));
    shot(add, "tabs-folder-drag-removed", Pair::None);
    // A right click on an entry that isn't marked (the cursor on another): the focus comes to it, and its chip is
    // taken out ("right click should remove everywhere").
    add(SETTLE, focus("slot", 0));
    add(SETTLE, right("slot", 12));
    add(SETTLE, expect(|ui, app| {
        let there = ui.global::<FolderView>().get_slots().row_data(12).map(|s| s.there);
        focus_on(app, "slot", 12).or_else(|| (there != Some(false)).then(|| "a right click on entry 13: its chip isn't taken out".to_string()))
    }));
    // A library's chip dragged onto an entry ("you should allow drag and dropping from the library into a slot"): shot
    // held over entry 11 (its ghost under the pointer, the entry outlined) and let go: it is in entry 11, the other
    // entries as they were, and entry 11 is marked.
    let picked: Rc<RefCell<Picked>> = Rc::default();
    add(SETTLE, keep(&kept));
    add(SETTLE, pick(&picked, |ui| {
        let rows = ui.global::<FolderView>().get_rows();
        rows.iter().position(|r| !r.refused).map(|i| Picked { row: i as i32, name: rows.row_data(i).unwrap().name.to_string(), ..Default::default() })
    }));
    drag_in(add, &picked, |_, _| Some(Point::Below("slot", 10, 10.0)));
    shot(add, "tabs-folder-drag-in", Pair::None);
    add(SETTLE, hand(Point::Below("slot", 10, 10.0), Ev::Release));
    let (was, chip) = (kept.clone(), picked.clone());
    add(SETTLE, expect(move |ui, app| {
        let now = names(ui).0;
        let mut want = was.borrow().0.clone();
        if let Some(entry) = want.get_mut(10) {
            *entry = chip.borrow().name.clone();
        }
        focus_on(app, "slot", 10).or_else(|| (now != want).then(|| format!("{:?} dragged onto entry 11: the entries are {now:?}, not {want:?}", chip.borrow().name)))
    }));
    shot(add, "tabs-folder-dragged-in", Pair::None);
    // One the rules keep out there dragged onto entry 11: refused, its row shakes, the folder is as it was, and the
    // strip says why. (Played by a ruleset of the player's own that allows of the folder's chips the game's first
    // version's own alone, as the page's quick choice makes one, so the other version's Giga chips are refused: a
    // build has no version (docs/design/app-ui.md §2.60) and no preset holds a version's chips back, so the folder's
    // rules alone may refuse nothing the walk's folder could take. It is deleted after.)
    let own_file: Rc<RefCell<Option<std::path::PathBuf>>> = Rc::default();
    let written = own_file.clone();
    add(SETTLE, Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        let Some(kit) = crate::builds::kit(&app.shell) else { return };
        let Some(rulesets) = kit.rulesets() else { return };
        let (Some(folder), Some(first)) = (rulesets.categories.iter().find(|c| c.fact == "folder"), rulesets.groups.first()) else { return };
        let mut own = rulesets.presets[0].clone();
        own.name = nettai_match::ruleset::Name::Own("Tour".into());
        own.allow_groups(&rulesets, folder, &[first.id.as_str()]);
        let path = crate::rulesets::store::new_path(&kit.game, "Tour");
        if crate::rulesets::store::write(&path, kit.content(), &kit.game, &own).is_err() {
            return;
        }
        *written.borrow_mut() = Some(path);
        if let Some(at) = crate::rulesets::pickable(&kit).iter().position(|p| p.ruleset.name == own.name) {
            app.with_screen(ScreenId::Builds, |s, shell| s.choose(shell, "ruleset", 0, at as i32));
        }
    }));
    add(SETTLE, keep(&kept));
    let other = picked.clone();
    add(SETTLE, pick(&picked, move |ui| {
        let rows = ui.global::<FolderView>().get_rows();
        let other = other.borrow().name.clone();
        // (A standard chip's first: a Mega chip refused for the Mega limit might go in where another left.)
        let refused = |standard: bool| rows.iter().position(|r| r.refused && (r.kind == nettai_ui::ChipKind::Standard) == standard && r.name != other.as_str());
        refused(true).or_else(|| refused(false)).map(|i| Picked { row: i as i32, name: rows.row_data(i).unwrap().name.to_string(), ..Default::default() })
    }));
    drag_in(add, &picked, |_, _| Some(Point::Below("slot", 10, 10.0)));
    add(SETTLE, hand(Point::Below("slot", 10, 10.0), Ev::Release));
    let (was, chip) = (kept.clone(), picked.clone());
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).0;
        let said = ui.global::<FolderView>().get_detail().problem;
        (now != was.borrow().0 || said.is_empty()).then(|| format!("{:?}, refused, dragged onto entry 11: the folder changed, or the strip says nothing ({said:?})", chip.borrow().name))
    }));
    shot(add, "tabs-folder-drag-in-refused", Pair::None);
    // (The build made for the game's first preset again, as the walk is, and the player's own ruleset gone.)
    add(SETTLE, screen(|s, shell| s.choose(shell, "ruleset", 0, 0)));
    add(SETTLE, Box::new(move |_, _, _| {
        if let Some(path) = own_file.borrow_mut().take() {
            let _ = nettai_platform::fs::remove_file(path);
        }
    }));
    // A patch card dragged above the one before it.
    // (Two more cards put in first, the library's first two that can go in: the walk has left one or none.)
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, focus("card", 0));
    add(SETTLE, library());
    for _ in 0..2 {
        add(
            SETTLE,
            Box::new(|ui, app, tour| {
                if tour.passing.get() {
                    return;
                }
                let rows = ui.global::<nettai_ui::PatchCardsView>().get_rows();
                let Some(row) = rows.iter().position(|r| !r.refused && r.note.is_empty()) else { return };
                let mut app = app.borrow_mut();
                app.shell.nav.focus("lib.rows", row as i32);
                app.act(Action::Confirm);
            }),
        );
    }
    add(SETTLE, out_of_library());
    add(SETTLE, keep(&kept));
    drag_to(add, "card", 1, 0, -10.0);
    shot(add, "tabs-cards-drag", Pair::None);
    add(SETTLE, let_go("card", 0, -10.0));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).1;
        let was = &was.borrow().1;
        // (EXE4's slot's card, dragged over another slot, goes back to its own: each card's slot is its own.)
        if ui.global::<nettai_ui::PatchCardsView>().get_counts().slotted {
            return (now != *was).then(|| format!("a slot's card dragged above another: the cards are {now:?}, were {was:?}"));
        }
        let want: Vec<&String> = was.get(..2).map(|w| vec![&w[1], &w[0]]).unwrap_or_default();
        (want.is_empty() || now.iter().take(2).collect::<Vec<_>>() != want).then(|| format!("the second card dragged above the first: the cards are {now:?}, were {was:?}"))
    }));
    shot(add, "tabs-cards-dragged", Pair::None);
    // The first card dragged onto the card library: taken out, the others closing up (in EXE4, the first slot that
    // holds a card emptied, the others as they were).
    add(SETTLE, keep(&kept));
    add(SETTLE, hand_at(first_card, Ev::Press));
    add(60, hand(Point::Middle("lib.rows", 60.0), Ev::Move));
    add(60, hand(Point::Middle("lib.rows", 0.0), Ev::Move));
    shot(add, "tabs-cards-drag-remove", Pair::None);
    add(SETTLE, drop_on_library());
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).1;
        let was = &was.borrow().1;
        (was.is_empty() || now.as_slice() != &was[1..]).then(|| format!("the first card dragged onto the library: the cards are {now:?}, were {was:?}"))
    }));
    shot(add, "tabs-cards-drag-removed", Pair::None);
    // A library's card that isn't in dragged onto the first of the build's: shot held there (its ghost, a gap where it
    // would go in, the cards from there on shifted down) and let go: it is the first, the others after it as they
    // were. In EXE4, whose cards go in slots, it goes in its own slot wherever over the slots it is let go (its slot
    // outlined while it is held there), in place of what that slot held.
    add(SETTLE, keep(&kept));
    add(SETTLE, pick(&picked, |ui| {
        let rows = ui.global::<nettai_ui::PatchCardsView>().get_rows();
        let i = rows.iter().position(|r| !r.refused && r.note.is_empty())?;
        let row = rows.row_data(i)?;
        Some(Picked { row: i as i32, name: row.name.to_string(), slot: row.slot.to_string() })
    }));
    drag_in(add, &picked, |_, _| Some(Point::Below("card", 0, -14.0)));
    shot(add, "tabs-cards-drag-in", Pair::None);
    add(SETTLE, hand(Point::Below("card", 0, -14.0), Ev::Release));
    let (was, card) = (kept.clone(), picked.clone());
    add(SETTLE, expect(move |ui, _| {
        let card = card.borrow();
        let view = ui.global::<nettai_ui::PatchCardsView>();
        if view.get_counts().slotted {
            let held = view.get_installed().iter().find(|c| c.slot == card.slot.as_str()).map(|c| c.name.to_string());
            return (held.as_deref() != Some(card.name.as_str())).then(|| format!("{:?} dragged onto the slots: its slot {} holds {held:?}", card.name, card.slot));
        }
        let now = names(ui).1;
        let want: Vec<String> = std::iter::once(card.name.clone()).chain(was.borrow().1.iter().cloned()).collect();
        (now != want).then(|| format!("{:?} dragged onto the first card: the cards are {now:?}, not {want:?}", card.name))
    }));
    shot(add, "tabs-cards-dragged-in", Pair::None);
    // An auto battle place's chip (the first list's third, where the walk left one) dragged two places back in its
    // list: the places between shift on.
    add(SETTLE, to_tab(EditorTab::AutoBattle));
    add(SETTLE, out_of_library());
    add(SETTLE, keep(&kept));
    drag_to(add, "places.place", 2, 0, 0.0);
    shot(add, "tabs-auto-drag", Pair::None);
    add(SETTLE, let_go("places.place", 0, 0.0));
    let at = |places: &[(i32, String)], p: i32| places.iter().find(|l| l.0 == p).map(|l| l.1.clone()).unwrap_or_default();
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).2;
        let was = &was.borrow().2;
        let want = [at(was, 2), at(was, 0), at(was, 1)];
        let got = [at(&now, 0), at(&now, 1), at(&now, 2)];
        (want[0].is_empty() || got != want).then(|| format!("place 3's chip dragged to place 1: places 1 to 3 hold {got:?}, not {want:?}"))
    }));
    shot(add, "tabs-auto-dragged", Pair::None);
    // Then into the standard list's second place: another list, the two places trade what they hold.
    add(SETTLE, keep(&kept));
    drag_to(add, "places.place", 0, 4, 0.0);
    shot(add, "tabs-auto-drag-swap", Pair::None);
    add(SETTLE, let_go("places.place", 4, 0.0));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).2;
        let was = &was.borrow().2;
        let want = [at(was, 4), at(was, 0)];
        let got = [at(&now, 0), at(&now, 4)];
        (got != want).then(|| format!("place 1's chip dragged to place 5, another list's: places 1 and 5 hold {got:?}, not {want:?}"))
    }));
    shot(add, "tabs-auto-swapped", Pair::None);
    // One dragged down past the page's foot (it scrolls on while the pointer is near its edge) into the Mega list,
    // which holds Mega chips alone: refused, it shakes, nothing changes, and the strip says why.
    add(SETTLE, keep(&kept));
    add(SETTLE, hand(Point::Control("places.place", 4), Ev::Press));
    add(60, hand(Point::Below("places.place", 4, 10.0), Ev::Move));
    add(60, hand(Point::Below("places.place", 4, 300.0), Ev::Move));
    add(1500, Box::new(|_, _, _| {}));
    shot(add, "tabs-auto-drag-scroll", Pair::None);
    let onto = |ev: fn(Point) -> Ev| -> Step {
        Box::new(move |ui, app, tour| {
            let Some(mega) = held(&app.borrow(), ChipClass::Mega) else { return };
            hand(Point::Control("places.place", mega as i32), ev)(ui, app, tour)
        })
    };
    add(SETTLE, onto(Ev::Move));
    add(60, onto(Ev::Release));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).2;
        let said = ui.global::<nettai_ui::AutoBattleView>().get_detail().problem;
        (now != was.borrow().2 || said.is_empty()).then(|| format!("a standard chip dragged into the Mega list: the places changed, or the strip says nothing ({said:?})"))
    }));
    shot(add, "tabs-auto-drag-refused", Pair::None);
    // Back at the first list (the page scrolled to its top), the first place holding a chip dragged onto the library:
    // that place is emptied, the others as they were.
    add(SETTLE, focus("places.place", 0));
    add(SETTLE, keep(&kept));
    onto_library(add, Point::Filled);
    shot(add, "tabs-auto-drag-remove", Pair::None);
    add(SETTLE, drop_on_library());
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let now = names(ui).2;
        let was = &was.borrow().2;
        let changed: Vec<(&(i32, String), Option<&(i32, String)>)> = was.iter().map(|w| (w, now.iter().find(|n| n.0 == w.0))).filter(|(w, n)| Some(*w) != *n).collect();
        let emptied = matches!(changed.as_slice(), [(w, Some(n))] if !w.1.is_empty() && n.1.is_empty());
        (!emptied).then(|| format!("the first place holding a chip dragged onto the library: the places that changed are {changed:?}, not one emptied"))
    }));
    shot(add, "tabs-auto-drag-removed", Pair::None);
    // A library's chip (one place 2 doesn't hold) dragged onto the first list's second place: shot held there (its
    // ghost, the place outlined) and let go: the place holds it, the others as they were, and it is marked.
    add(SETTLE, keep(&kept));
    let was = kept.clone();
    add(SETTLE, pick(&picked, move |ui| {
        let rows = ui.global::<nettai_ui::AutoBattleView>().get_chips();
        let held = was.borrow().2.iter().find(|l| l.0 == 1).map(|l| l.1.clone()).unwrap_or_default();
        let i = rows.iter().position(|r| r.name != held.as_str())?;
        Some(Picked { row: i as i32, name: rows.row_data(i)?.name.to_string(), ..Default::default() })
    }));
    drag_in(add, &picked, |_, _| Some(Point::Below("places.place", 1, 10.0)));
    shot(add, "tabs-auto-drag-in", Pair::None);
    add(SETTLE, hand(Point::Below("places.place", 1, 10.0), Ev::Release));
    let (was, chip) = (kept.clone(), picked.clone());
    add(SETTLE, expect(move |ui, app| {
        let now = names(ui).2;
        let want: Vec<(i32, String)> = was.borrow().2.iter().map(|l| if l.0 == 1 { (1, chip.borrow().name.clone()) } else { l.clone() }).collect();
        focus_on(app, "places.place", 1).or_else(|| (now != want).then(|| format!("{:?} dragged onto place 2: the places are {now:?}, not {want:?}", chip.borrow().name)))
    }));
    shot(add, "tabs-auto-dragged-in", Pair::None);
}

/// The tabs' steps in language `lang` at size `size`. `shot` takes a picture under a name (and compares the two
/// of a pair).
pub(super) fn steps(add: &mut dyn FnMut(u64, Step), shot: Shot, _lang: &'static str, size: &'static str) {
    open_build(add);
    navicust(add, shot);
    if size == "desktop" {
        carries(add, shot);
    }
    patch_cards(add, shot);
    card_bugs(add, shot);
    auto_battle(add, shot);
    if size == "desktop" {
        clicks(add, shot);
        hovers(add, shot);
        program_colors(add, shot);
    }
    drags(add, shot, size == "phone");
    wrong_classes(add, shot);
    overlap(add, shot);
    card_mix(add, shot);
    add(SETTLE, end());
}

/// The partners and their programs as PARTY's slots show them: each partner's name and its programs' names.
fn party_of(ui: &nettai_ui::AppWindow) -> Vec<(String, Vec<String>)> {
    use nettai_ui::PartySlotKind as K;
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for s in ui.global::<nettai_ui::PartyView>().get_slots().iter() {
        match s.kind {
            K::Head => out.push((s.name.to_string(), Vec::new())),
            K::Program => {
                if let Some(p) = out.last_mut() {
                    p.1.push(s.row.name.to_string());
                }
            }
            _ => {}
        }
    }
    out
}

/// What a slot of PARTY's is, for a step to find it by: a partner's head, its program, its Empty row (each by the
/// partner's place), or an Empty head (the first).
#[derive(Clone, Copy, Debug)]
enum PartySpot {
    Head(usize),
    Program(usize, usize),
    End(usize),
    Open,
}

/// The row of PARTY's slots that is `spot`.
fn party_row(ui: &nettai_ui::AppWindow, spot: PartySpot) -> Option<i32> {
    use nettai_ui::PartySlotKind as K;
    let (mut partner, mut k) = (-1i32, 0usize);
    for (row, s) in ui.global::<nettai_ui::PartyView>().get_slots().iter().enumerate() {
        match s.kind {
            K::Head => {
                partner += 1;
                k = 0;
            }
            K::Program => k += 1,
            _ => {}
        }
        let here = match (spot, s.kind) {
            (PartySpot::Head(i), K::Head) => partner == i as i32,
            (PartySpot::Program(i, at), K::Program) => partner == i as i32 && k == at + 1,
            (PartySpot::End(i), K::End) => partner == i as i32,
            (PartySpot::Open, K::Open) => true,
            _ => false,
        };
        if here {
            return Some(row as i32);
        }
    }
    None
}

/// The cursor on slot `spot` (it is marked, the library beside it of what it takes); with `enter`, Enter on it (into
/// the library). None there: the report says so.
fn to_slot(spot: PartySpot, enter: bool) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(row) = party_row(ui, spot) else {
            tour.key_wrong(format!("PARTY has no slot {spot:?}"));
            return;
        };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("slots", row);
        if enter {
            app.act(Action::Confirm);
        }
    })
}

/// The pointer at slot `spot`'s middle, `dy` down from it.
fn slot_point(spot: PartySpot, dy: f32) -> impl Fn(&nettai_ui::AppWindow, &App) -> Option<Point> + Clone + 'static {
    move |ui, _| party_row(ui, spot).map(|row| Point::Below("slots", row, dy))
}

/// Enter on the first of the library's rows `which` takes (none: the report says so).
fn pick_row(which: fn(&nettai_ui::PartyRow) -> bool) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(at) = ui.global::<nettai_ui::PartyView>().get_rows().iter().position(|r| which(&r)) else {
            tour.key_wrong("the Party tab's library has no row to pick".to_string());
            return;
        };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("lib.rows", at as i32);
        app.act(Action::Confirm);
    })
}

/// The first of the library's rows `which` takes (by its row as the view shows it), kept in `picked`: one off the
/// list's first two and last two where there is one (a press near its edge would scroll it).
fn pick_program(picked: &Rc<RefCell<Picked>>, which: fn(&nettai_ui::PartyRow) -> bool) -> Step {
    pick(picked, move |ui| {
        let rows = ui.global::<nettai_ui::PartyView>().get_rows();
        let n = rows.row_count();
        let found: Vec<usize> = rows.iter().enumerate().filter(|(_, r)| which(r)).map(|(i, _)| i).collect();
        let i = found.iter().copied().find(|&i| i >= 2 && i + 2 < n).or(found.first().copied())?;
        Some(Picked { row: i as i32, name: rows.row_data(i).unwrap().name.to_string(), ..Default::default() })
    })
}

/// The pointer pressed at `from`, moved off it and on to `to` (each found when it is due), and held there: `hand_at(..,
/// Release)` lets it go.
fn party_drag(add: &mut dyn FnMut(u64, Step), from: impl Fn(&nettai_ui::AppWindow, &App) -> Option<Point> + Clone + 'static, to: impl Fn(&nettai_ui::AppWindow, &App) -> Option<Point> + Clone + 'static) {
    let f = from.clone();
    add(SETTLE, hand_at(f, Ev::Press));
    add(60, hand_at(move |ui, app| from(ui, app).map(|p| match p {
        Point::Below(id, i, dy) => Point::Below(id, i, dy + 10.0),
        p => p,
    }), Ev::Move));
    add(60, hand_at(to.clone(), Ev::Move));
    add(60, hand_at(to, Ev::Move));
}

/// The ruleset page's category named `name` (as the page lists it, in English), by the choice its select makes; none,
/// the report says so.
fn rules_category(name: &'static str) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let Some(at) = ui.global::<nettai_ui::RulesetView>().get_categories().iter().position(|c| c.name == name) else {
            tour.key_wrong(format!("the ruleset's page has no category {name:?}"));
            return;
        };
        app.borrow_mut().with_screen(ScreenId::Builds, |s, shell| s.choose(shell, "rules.category", 0, at as i32));
    })
}

/// The EXE5 ruleset page (for the mockups of the party's pages): EDIT by the build's ruleset, DUPLICATE (the player's
/// own, every page editable), its Party page (paired: the navis, and their programs) as R2's example has it (KnightMan
/// not allowed, P.HP+300 held back for every navi, P.Btlpk2 for ProtoMan and Colonel, P.Spport for Meddy): ProtoMan
/// chosen, All navis chosen, KnightMan (not allowed) chosen, GyroMan (every navi's ban alone) chosen; its NaviCust
/// programs and Patch cards pages; then Back, the build made for the game's first preset again. On a phone, the Party
/// page with ProtoMan chosen alone.
fn party_rules(add: &mut dyn FnMut(u64, Step), shot: Shot, phone: bool) {
    add(SETTLE, press("tab", 0));
    add(SETTLE, press("ruleset-edit", 0));
    add(SETTLE, press("rules.duplicate", 0));
    add(SETTLE, rules_category("Party"));
    // (The keys: All navis, Team ProtoMan's six, Team Colonel's six; the programs in the library's order.)
    let (all, protoman, gyroman, meddy, colonel, knightman) = (0, 1, 2, 6, 7, 11);
    let (hp_300, btlpk_2, spport) = (3, 11, 12);
    add(SETTLE, focus("rules.pair.keys", all));
    add(SETTLE, press("rules.pair.programs", hp_300));
    add(SETTLE, press("rules.pair.keys", knightman));
    for (navi, program) in [(protoman, btlpk_2), (colonel, btlpk_2), (meddy, spport)] {
        add(SETTLE, focus("rules.pair.keys", navi));
        add(SETTLE, press("rules.pair.programs", program));
    }
    // (All navis first: the page back at its top, which a phone's KnightMan scrolled away from.)
    add(SETTLE, focus("rules.pair.keys", all));
    add(SETTLE, focus("rules.pair.keys", protoman));
    if phone {
        shot(add, "party-rules-party", Pair::None);
    } else {
        shot(add, "party-rules-party", Pair::None);
        add(SETTLE, focus("rules.pair.keys", all));
        shot(add, "party-rules-party-all", Pair::None);
        add(SETTLE, focus("rules.pair.keys", knightman));
        shot(add, "party-rules-party-not-allowed", Pair::None);
        add(SETTLE, focus("rules.pair.keys", gyroman));
        shot(add, "party-rules-party-inherited", Pair::None);
        add(SETTLE, rules_category("NaviCust programs"));
        add(SETTLE, focus("rules.items", 0));
        shot(add, "party-rules-navicust", Pair::None);
        add(SETTLE, rules_category("Patch cards"));
        add(SETTLE, focus("rules.items", 0));
        shot(add, "party-rules-cards", Pair::None);
    }
    add(SETTLE, act(Action::Back));
    add(SETTLE, screen(|s, shell| s.choose(shell, "ruleset", 0, 0)));
}

/// The Party tab (EXE5's party battle; a game without one passes these by), its slots' list and the library that
/// follows it: two partners put in from Empty heads by Enter (an Empty head marked first, its library the navis, the
/// other version's refused under the one version the ruleset allows: shot), a program slot marked (its library the
/// programs: shot); then every drag the tab takes, each let go and what it did checked (`report.txt` says what didn't),
/// the held states shot: a library's program into the first partner's Empty row, onto its head (its top), into the
/// second's; one the second's gauge has no room for (a gap all the same, the dragged row saying why; let go, nothing
/// changes); a program moved in its partner's list, onto the library (taken out), onto the other partner's head (its
/// top); a partner's head onto the other's (they trade places), onto the library (taken out); a library's navi onto the
/// Empty head that leaves (it goes in). Then, for comparing, the Patch cards library with a card refused, and the
/// NaviCust's list. On a phone, the ruleset's Party page, then the slots and the navis' sheet.
pub(super) fn party_steps(add: &mut dyn FnMut(u64, Step), shot: Shot, _lang: &'static str, size: &'static str) {
    use nettai_ui::PartyView;
    if size == "phone" {
        open_build(add);
        // (The ruleset's Party page first, its navis' list.)
        party_rules(add, shot, true);
        add(SETTLE, to_tab(EditorTab::Party));
        add(SETTLE, to_slot(PartySpot::Open, false));
        shot(add, "party-phone", Pair::None);
        add(SETTLE, to_slot(PartySpot::Open, true));
        shot(add, "party-phone-navis", Pair::None);
        add(SETTLE, pick_row(|r| r.why.is_empty()));
        add(SETTLE, act(Action::Back));
        add(SETTLE, to_slot(PartySpot::End(0), false));
        shot(add, "party-phone-programs-slot", Pair::None);
        add(SETTLE, end());
        return;
    }
    open_build(add);
    party_rules(add, shot, false);
    add(SETTLE, to_tab(EditorTab::Party));
    add(SETTLE, to_slot(PartySpot::Open, false));
    shot(add, "party-empty", Pair::None);
    add(90, on_tab(EditorTab::Party, "Party"));
    // The first partner by Enter on an Empty head and on a navi; the second Empty head marked (the navis, those of the
    // other version refused: shot, and one of them under the cursor: the strip says why); the second partner.
    add(SETTLE, to_slot(PartySpot::Open, true));
    add(SETTLE, pick_row(|r| r.why.is_empty()));
    add(SETTLE, expect(|ui, _| (party_of(ui).len() != 1).then(|| format!("a navi picked for an Empty head: the party is {:?}", party_of(ui)))));
    add(SETTLE, to_slot(PartySpot::Open, false));
    shot(add, "party-navi-empty", Pair::None);
    add(SETTLE, Box::new(|ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        if let Some(at) = ui.global::<PartyView>().get_rows().iter().position(|r| !r.why.is_empty()) {
            app.borrow_mut().shell.nav.focus("lib.rows", at as i32);
        }
    }));
    shot(add, "party-navi-refused", Pair::None);
    add(SETTLE, to_slot(PartySpot::Open, true));
    add(SETTLE, pick_row(|r| r.why.is_empty()));
    add(SETTLE, expect(|ui, _| (party_of(ui).len() != 2).then(|| format!("a navi picked for the second Empty head: the party is {:?}", party_of(ui)))));
    add(SETTLE, to_slot(PartySpot::Head(0), false));
    shot(add, "party-navi-filled", Pair::None);
    add(SETTLE, to_slot(PartySpot::End(0), false));
    shot(add, "party-program-slot", Pair::None);
    let small = |r: &nettai_ui::PartyRow| r.why.is_empty() && r.blocks <= 2;
    let picked: Rc<RefCell<Picked>> = Rc::default();
    // A library's program into the first partner's Empty row, held there, let go.
    add(SETTLE, pick_program(&picked, small));
    drag_in(add, &picked, |ui, app| slot_point(PartySpot::End(0), 0.0)(ui, app));
    shot(add, "party-drag-in-held", Pair::None);
    add(SETTLE, hand_at(|ui, app| slot_point(PartySpot::End(0), 0.0)(ui, app), Ev::Release));
    let got = picked.clone();
    add(SETTLE, expect(move |ui, _| {
        let party = party_of(ui);
        let want = vec![got.borrow().name.clone()];
        (party.first().map(|p| &p.1) != Some(&want)).then(|| format!("a library's program dragged onto the first partner's Empty row: it holds {:?}, not {want:?}", party.first().map(|p| &p.1)))
    }));
    // Another onto its head: it goes first.
    add(SETTLE, to_slot(PartySpot::End(0), false));
    add(SETTLE, pick_program(&picked, |r| r.why.is_empty() && r.blocks == 1));
    drag_in(add, &picked, |ui, app| slot_point(PartySpot::Head(0), 0.0)(ui, app));
    shot(add, "party-drag-in-top", Pair::None);
    add(SETTLE, hand_at(|ui, app| slot_point(PartySpot::Head(0), 0.0)(ui, app), Ev::Release));
    let got = picked.clone();
    add(SETTLE, expect(move |ui, _| {
        let party = party_of(ui);
        let first = party.first().and_then(|p| p.1.first()).cloned();
        (first.as_deref() != Some(got.borrow().name.as_str()) || party.first().map(|p| p.1.len()) != Some(2)).then(|| format!("a library's program dragged onto the first partner's head: its programs are {:?}", party.first().map(|p| &p.1)))
    }));
    // Into the second partner's Empty row.
    add(SETTLE, to_slot(PartySpot::End(1), false));
    add(SETTLE, pick_program(&picked, small));
    drag_in(add, &picked, |ui, app| slot_point(PartySpot::End(1), 0.0)(ui, app));
    add(SETTLE, hand_at(|ui, app| slot_point(PartySpot::End(1), 0.0)(ui, app), Ev::Release));
    add(SETTLE, expect(|ui, _| {
        let party = party_of(ui);
        (party.get(1).map_or(0, |p| p.1.len()) != 1).then(|| format!("a library's program dragged into the second partner's: the party is {party:?}"))
    }));
    // The second's gauge filled from its Empty row with the largest, by Enter, until one is refused; one it has no room
    // for dragged over it: a gap all the same, the dragged row saying why; let go, nothing changes.
    for _ in 0..3 {
        add(SETTLE, to_slot(PartySpot::End(1), false));
        add(SETTLE, Box::new(|ui, app, tour| {
            if tour.passing.get() {
                return;
            }
            let rows = ui.global::<PartyView>().get_rows();
            let Some(at) = rows.iter().enumerate().filter(|(_, r)| r.why.is_empty()).max_by_key(|(_, r)| r.blocks).map(|(i, _)| i) else { return };
            let mut app = app.borrow_mut();
            app.shell.nav.focus("lib.rows", at as i32);
            app.act(Action::Confirm);
        }));
    }
    let kept: Rc<RefCell<Vec<(String, Vec<String>)>>> = Rc::default();
    let keep = |kept: &Rc<RefCell<Vec<(String, Vec<String>)>>>| -> Step {
        let kept = kept.clone();
        Box::new(move |ui, _, _| *kept.borrow_mut() = party_of(ui))
    };
    add(SETTLE, to_slot(PartySpot::Head(1), false));
    add(SETTLE, to_slot(PartySpot::Program(1, 0), false));
    add(SETTLE, pick_program(&picked, |r| !r.why.is_empty()));
    add(SETTLE, keep(&kept));
    drag_in(add, &picked, |ui, app| slot_point(PartySpot::Program(1, 0), 0.0)(ui, app));
    add(SETTLE, expect(|ui, _| {
        let (why, to) = (ui.global::<PartyView>().get_drag_why(), ui.global::<nettai_ui::Drag>().get_to());
        (why.is_empty() || to < 0).then(|| format!("a program the second partner has no room for, dragged over it: the dragged row says {why:?}, the gap at {to}"))
    }));
    shot(add, "party-drag-refused", Pair::None);
    add(SETTLE, hand_at(|ui, app| slot_point(PartySpot::Program(1, 0), 0.0)(ui, app), Ev::Release));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| (party_of(ui) != *was.borrow()).then(|| format!("a refused program let go over the second partner: the party changed to {:?}", party_of(ui)))));
    // The first partner's first program moved past its second, in its list.
    add(SETTLE, keep(&kept));
    party_drag(add, slot_point(PartySpot::Program(0, 0), 0.0), slot_point(PartySpot::Program(0, 1), 8.0));
    shot(add, "party-drag-within", Pair::None);
    add(SETTLE, hand_at(slot_point(PartySpot::Program(0, 1), 8.0), Ev::Release));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let was = was.borrow();
        let mut want = was.first().map(|p| p.1.clone()).unwrap_or_default();
        want.reverse();
        let got = party_of(ui).first().map(|p| p.1.clone()).unwrap_or_default();
        (got != want).then(|| format!("the first partner's first program dragged past its second: its programs are {got:?}, not {want:?}"))
    }));
    // The second partner's first program onto the library: taken out.
    add(SETTLE, keep(&kept));
    party_drag(add, slot_point(PartySpot::Program(1, 0), 0.0), |_, _| Some(Point::Middle("lib.rows", 0.0)));
    shot(add, "party-drag-remove", Pair::None);
    add(SETTLE, drop_on_library());
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let (was, now) = (was.borrow(), party_of(ui));
        (now.get(1).map(|p| p.1.len() + 1) != was.get(1).map(|p| p.1.len())).then(|| format!("the second partner's first program dragged onto the library: the party is {now:?} (was {was:?})"))
    }));
    // The first partner's first program onto the second's head: at its top.
    add(SETTLE, keep(&kept));
    party_drag(add, slot_point(PartySpot::Program(0, 0), 0.0), slot_point(PartySpot::Head(1), 0.0));
    shot(add, "party-drag-between", Pair::None);
    add(SETTLE, hand_at(slot_point(PartySpot::Head(1), 0.0), Ev::Release));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let (was, now) = (was.borrow(), party_of(ui));
        let moved = was.first().and_then(|p| p.1.first().cloned());
        let ok = now.len() == 2 && was.len() == 2 && now[0].1.len() + 1 == was[0].1.len() && now[1].1.len() == was[1].1.len() + 1 && now[1].1.first() == moved.as_ref();
        (!ok).then(|| format!("the first partner's first program dragged onto the second's head: the party is {now:?} (was {was:?})"))
    }));
    // The second partner's head onto the first's: they trade places, their programs with them.
    add(SETTLE, keep(&kept));
    party_drag(add, slot_point(PartySpot::Head(1), 0.0), slot_point(PartySpot::Head(0), 0.0));
    shot(add, "party-drag-head", Pair::None);
    add(SETTLE, hand_at(slot_point(PartySpot::Head(0), 0.0), Ev::Release));
    let was = kept.clone();
    add(SETTLE, expect(move |ui, _| {
        let (was, now) = (was.borrow(), party_of(ui));
        let want: Vec<(String, Vec<String>)> = was.iter().rev().cloned().collect();
        (now != want).then(|| format!("the second partner's head dragged onto the first's: the party is {now:?}, not {want:?}"))
    }));
    // The second partner's head onto the library: taken out, an Empty head in its place.
    add(SETTLE, keep(&kept));
    party_drag(add, slot_point(PartySpot::Head(1), 0.0), |_, _| Some(Point::Middle("lib.rows", 0.0)));
    add(SETTLE, drop_on_library());
    add(SETTLE, expect(|ui, _| (party_of(ui).len() != 1).then(|| format!("the second partner's head dragged onto the library: the party is {:?}", party_of(ui)))));
    // A library's navi onto the Empty head: held there (outlined), let go: it goes in.
    add(SETTLE, to_slot(PartySpot::Open, false));
    add(SETTLE, pick_program(&picked, |r| r.why.is_empty()));
    drag_in(add, &picked, |ui, app| slot_point(PartySpot::Open, 0.0)(ui, app));
    shot(add, "party-drag-navi", Pair::None);
    add(SETTLE, hand_at(|ui, app| slot_point(PartySpot::Open, 0.0)(ui, app), Ev::Release));
    let got = picked.clone();
    add(SETTLE, expect(move |ui, _| {
        let party = party_of(ui);
        (party.len() != 2 || party[1].0 != got.borrow().name).then(|| format!("a library's navi dragged onto the Empty head: the party is {party:?}, not with {:?} second", got.borrow().name))
    }));
    add(SETTLE, to_slot(PartySpot::Head(0), false));
    shot(add, "party-dragged", Pair::None);
    // For comparing: the Patch cards library, three cards in, by MB the most first, one refused; the NaviCust's list.
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, library());
    for row in 0..3 {
        add(SETTLE, focus("lib.rows", row));
        add(SETTLE, act(Action::Confirm));
    }
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 2)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 0));
    add(SETTLE, act(Action::Confirm));
    shot(add, "party-cmp-cards", Pair::None);
    add(SETTLE, out_of_library());
    add(SETTLE, to_tab(EditorTab::Navicust));
    add(SETTLE, focus("programs", 0));
    shot(add, "party-cmp-navicust", Pair::None);
    add(SETTLE, end());
}

/// The tabs' bursts of the motion probe (`burst` takes one: a name, the milliseconds after its trigger at which a
/// shot is taken, and the trigger): a piece taken up (it lifts), turned (a quarter turn), put down (a pulse along
/// its cells), refused (it shakes), and a bug appearing (it pulses purple).
pub(super) fn motion(add: &mut dyn FnMut(u64, Step), burst: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, &[u64], Step)) {
    open_build(add);
    add(SETTLE, to_tab(EditorTab::Navicust));
    add(SETTLE, to_tab(EditorTab::Folder));
    // An entry dragged three rows down: it lifts under the pointer, those between slide up to leave a gap where it
    // would land, and let go it lands there with a flash.
    burst(add, "folder-drag", &[0, 60, 120, 200, 280, 360, 440, 520, 640, 800], pointer(|_, _| Some(drag_rows("slot", 5, 3))));
    // An entry dragged onto the library: over it, the entry's line goes red and it fades back, and the library fades
    // under DROP TO REMOVE; let go, its chip is taken out. Then a right click on another entry, not marked: emptied.
    let onto = |_: &Board, _: &nettai_ui::AppWindow| {
        let (from, to) = (Point::Control("slot", 12), Point::Middle("lib.rows", 0.0));
        Some(vec![(0, Ev::Press(from)), (60, Ev::Move(Point::Below("slot", 12, 10.0))), (140, Ev::Move(Point::Middle("lib.rows", 60.0))), (220, Ev::Move(to)), (420, Ev::Release(to))])
    };
    burst(add, "folder-drag-remove", &[0, 60, 140, 180, 220, 260, 320, 420, 480, 600], pointer(onto));
    burst(add, "folder-right-click", &[0, 60, 120, 200, 320], pointer(|_, _| Some(vec![(0, Ev::Right(Point::Control("slot", 14)))])));
    // A click on an entry and R (it is the Regular chip), a click on another and T (it is tagged): the keys act on
    // what was clicked.
    burst(add, "folder-click-r", &[0, 100, 250, 380, 520], click_then("slot", 8, KeyCode::KeyR));
    burst(add, "folder-click-t", &[0, 100, 250, 380, 520], click_then("slot", 10, KeyCode::KeyT));
    add(SETTLE, to_tab(EditorTab::Navicust));
    burst(add, "navicust-lift", &[0, 30, 60, 100, 160, 260], take(0, program));
    burst(add, "navicust-move", &[0, 30, 60, 100, 160], act(Action::Right));
    burst(add, "navicust-turn", &[0, 30, 60, 100, 160, 260], key(KeyCode::KeyS, "s"));
    // Compressed and back, turned as it is and in place.
    burst(add, "navicust-compress", &[0, 30, 60, 100, 160], key(KeyCode::KeyC, "c"));
    burst(add, "navicust-decompress", &[0, 30, 60, 100, 160], key(KeyCode::KeyC, "c"));
    add(SETTLE, key(KeyCode::KeyA, "a"));
    add(SETTLE, act(Action::Down));
    // The command line's arrows, pulsing by themselves.
    burst(add, "navicust-arrows", &[0, 150, 300, 450, 600, 750, 900], Box::new(|_, _, _| {}));
    burst(add, "navicust-drop", &[0, 30, 60, 90, 120, 160, 220, 320], act(Action::Confirm));
    add(SETTLE, take(1, program));
    burst(add, "navicust-bug", &[0, 40, 80, 120, 200, 320], put(Spot::OffTheLine));
    add(SETTLE, take(2, program));
    burst(add, "navicust-refused", &[0, 30, 60, 90, 120, 160, 220, 320], put(Spot::Over));
    add(SETTLE, act(Action::Back));
    // The mouse carries a piece by clicks ("usually it's click to drop"): a placed piece pressed (it lifts as the
    // button goes down), moved two cells with the button held (it stays), let go (it comes under the pointer), and
    // clicked down there (with its pulse).
    burst(add, "navicust-press-move", &[0, 60, 120, 170, 230, 300, 370, 430, 500, 580, 640, 720, 800, 900], pointer(press_move));
    // A piece clicked up follows the pointer with no button held, cell by cell; a key moves it from there and the
    // pointer's next move takes it back under the pointer; a click puts it down.
    burst(add, "navicust-click-move", &[0, 120, 250, 350, 470, 570, 650, 760, 900, 1100, 1300], pointer(click_move));
    // A program's row clicked, carried onto the board and clicked down where it fits, and a placed one's (a new copy
    // of it, the placed one staying); a piece clicked over another (it shakes and stays in hand) and then off the
    // board (back where it was); a right click with a piece in hand (put away); a piece clicked off the board (back
    // where it was) and onto the list (the list's remove look, then put away); the wheel turning a clicked-up piece,
    // and a right click putting it away.
    burst(add, "navicust-row-click", &[0, 100, 200, 350, 500, 600, 800], pointer(row_click));
    burst(add, "navicust-row-click-placed", &[0, 100, 200, 350, 500, 600, 800], pointer(placed_row_click));
    burst(add, "navicust-click-refused", &[0, 100, 200, 300, 420, 500, 600, 700, 800, 900, 1000], pointer(click_refused));
    burst(add, "navicust-click-right-click", &[0, 100, 200, 350, 500, 650], pointer(click_right));
    burst(add, "navicust-click-off", &[0, 100, 200, 350, 500, 650], pointer(click_off));
    burst(add, "navicust-click-list", &[0, 100, 200, 260, 320, 400, 500, 600, 750], pointer(click_list));
    burst(add, "navicust-click-wheel-right", &[0, 120, 260, 340, 460, 560, 700, 850], pointer(click_wheel_right));
    // Right clicks with nothing in hand: on a placed piece (taken off), and on a placed program's row (its copy off).
    add(SETTLE, take(0, program));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, take(1, program));
    add(SETTLE, put(Spot::OffTheLine));
    burst(add, "navicust-right-click", &[0, 60, 120, 200, 320, 480], pointer(right_piece));
    burst(add, "navicust-row-right-click", &[0, 60, 120, 200, 320, 480], pointer(right_row));
    // A card put in (it flies from its row to the build's, which flashes, and the MB tick), and one the MB left
    // can't take (its row shakes).
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, focus("card", 0));
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 0));
    burst(add, "cards-put", &[0, 30, 60, 90, 120, 160, 200, 260, 340, 480], act(Action::Confirm));
    // A click on that card's row in the library (taken out, flying back; in a game with slots, its slot emptied), and
    // it put back; then one on the build's card, marked (taken out).
    burst(add, "cards-click-library", &[0, 30, 60, 90, 120, 160, 220, 320, 480], click("lib.rows", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, out_of_library());
    add(SETTLE, focus("card", 0));
    burst(add, "cards-click-marked", &[0, 30, 60, 90, 120, 160, 220, 320, 480], click("card", 0));
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 0));
    add(SETTLE, act(Action::Confirm));
    // A click on the build's card, the cursor in the library, and Del: it is taken out.
    burst(add, "cards-click-del", &[0, 100, 250, 330, 420, 560], click_then("card", 0, KeyCode::Delete));
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 2)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 0));
    burst(add, "cards-refused", &[0, 30, 60, 90, 120, 160, 220, 320], act(Action::Confirm));
    add(SETTLE, out_of_library());
    // A chip put in a place (it flies to the place, which flashes, and the cursor goes on to the next), where the
    // game has auto battle data.
    add(SETTLE, to_tab(EditorTab::AutoBattle));
    add(SETTLE, focus("places.place", 0));
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 0));
    burst(add, "auto-put", &[0, 30, 60, 90, 120, 160, 200, 260, 340, 480], act(Action::Confirm));
    // A click on the place just filled, marked: it is emptied.
    add(SETTLE, focus("places.place", 0));
    burst(add, "auto-click-marked", &[0, 60, 120, 200, 320, 480], click("places.place", 0));
    add(SETTLE, end());
}

/// The walk's end: back to the main menu, no tab passed by.
fn end() -> Step {
    Box::new(|_, app, tour| {
        tour.passing.set(false);
        let mut app = app.borrow_mut();
        leave(&mut app);
        app.go_now(ScreenId::Menu);
    })
}
