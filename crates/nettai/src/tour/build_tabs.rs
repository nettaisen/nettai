//! The screenshot pass's steps for the build editor's NaviCust, Patch cards and Auto battle tabs (docs/app.md
//! §13): each tab in the game the pass walks, with pairs a focus move apart, a piece moved, turned, put down,
//! refused and put back, cards put in and refused, places filled; and the motion probe's bursts of what the tabs
//! move. A tab the game hasn't (auto battle data: EXE6's setup takes none) is passed over: a step names the tab by
//! what it is, and `on_tab` reports a picture taken on another.
//!
//! The walk makes a build of its own (the pass's builds folder, `paths::keep_builds_in`), as a player makes one.

use super::{Pair, SETTLE, Step};
use crate::input::{Action, KeyPress};
use crate::screens::ScreenId;
use crate::shell::App;
use nettai_ui::{BuildsPage, BuildsView, EditorTab, NavicustView};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, Model};
use std::cell::RefCell;
use std::rc::Rc;

type Shot<'a> = &'a dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair);

/// The walk is on tab `tab` of the editor; if it isn't, the report says so (as a text the pass counts against the
/// picture).
fn on_tab(tab: EditorTab, name: &'static str) -> Step {
    Box::new(move |ui, _, tour| {
        if tour.passing.get() {
            return;
        }
        let view = ui.global::<BuildsView>();
        if view.get_page() != BuildsPage::Editor || view.get_shown() != tab {
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
/// A key pressed on the Builds screen, as the window's keys reach it (before the menus read it).
fn key(code: KeyCode, text: &'static str) -> Step {
    screen(move |s, shell| {
        s.key(shell, &KeyPress { code, text: Some(text.into()), repeat: false, shift: false });
    })
}

/// Out of whatever of the Builds screen is open, back to its list.
fn leave(app: &mut App) {
    for _ in 0..4 {
        if app.shell.screen() == ScreenId::Builds {
            app.with_screen(ScreenId::Builds, |s, shell| {
                s.back(shell);
            });
        }
    }
}

/// A build of the game's first version, made through the screen in an emptied builds folder, and open in the
/// editor.
fn open_build(add: &mut dyn FnMut(u64, Step), phone: bool) {
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            if let Some(game) = app.shell.game().map(str::to_string) {
                let _ = std::fs::remove_dir_all(crate::builds::store::folder(&game));
            }
            leave(&mut app);
            app.go_now(ScreenId::Builds);
            app.shell.nav.show_cursor(true, false);
        }),
    );
    add(SETTLE, press("new", 0));
    add(SETTLE, screen(|s, shell| s.edit(shell, "name.name", 0, "Tab tour", true)));
    add(SETTLE, screen(|s, shell| s.choose(shell, "name.version", 0, 0)));
    add(SETTLE, press("name.confirm", 0));
    add(SETTLE, press("builds", 0));
    if phone {
        add(SETTLE, press("one.edit", 0));
    }
}

/// The editor's tab of kind `tab`, by its place in the bar. A tab the game hasn't is passed by: the steps after
/// this do nothing and take no picture, until the next tab is gone to or the walk ends.
fn to_tab(tab: EditorTab) -> Step {
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
fn take(nth: usize, which: fn(&nettai_ui::ProgramRow) -> bool) -> Step {
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

/// Where the piece in hand may go: off the command line, on the out-of-bounds ring, or over another piece.
#[derive(Clone, Copy, PartialEq)]
enum Spot {
    OffTheLine,
    Ring,
    Over,
}

/// Put the piece in hand down at the first cell where it would be at `spot` (and, but `Spot::Over`, fit), as the
/// cursor moved there and Enter does: found from what the view shows of the board, whatever the game's shapes.
fn put(spot: Spot) -> Step {
    Box::new(move |ui, app, tour| {
        if tour.passing.get() {
            return;
        }
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
                }
        };
        // (Off the command line on the board itself where the piece has the room, else jutting onto the ring.)
        let places = || (0..n).flat_map(|y| (0..n).map(move |x| (x, y))).filter(|&(x, y)| at(x, y).is_some_and(|c| c.kind != nettai_ui::CellKind::None));
        let found = places().find(|&(x, y)| fits(x, y, false)).or_else(|| places().find(|&(x, y)| fits(x, y, true)));
        let Some((gx, gy)) = found else { return };
        let mut app = app.borrow_mut();
        app.shell.nav.focus("cell", gy * n + gx);
        app.act(Action::Confirm);
    })
}

/// The next pair's second shot may differ in the strip too (the program under the cursor, which a focus move
/// changes), on a desktop: it is the tab's top, above the board, 104 tall and 12 above the board's panel, whose
/// cells start 16 into it. (A phone has no strip.)
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
        let bottom = top - 16.0 - 12.0;
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
fn program(r: &nettai_ui::ProgramRow) -> bool {
    !r.plus
}

/// The NaviCust tab's steps.
fn navicust(add: &mut dyn FnMut(u64, Step), shot: Shot) {
    add(SETTLE, to_tab(EditorTab::Navicust));
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
    // A placed program's row under the cursor (its hint: Enter takes it off), and Enter: its copy off the board.
    add(SETTLE, row(|r| r.state == nettai_ui::ProgramState::Placed));
    shot(add, "tabs-navicust-row-placed", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "tabs-navicust-row-taken-off", Pair::None);
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
        Point::Off => rect("programs", 0).map(|r| slint::LogicalPosition::new(r.x + r.w + 150.0, r.y + 200.0)),
        Point::Control(id, i) => rect(id, i).filter(|r| r.h > 0.0).map(|r| slint::LogicalPosition::new(r.x + r.w / 2.0, r.y + r.h / 2.0)),
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

/// A right click on control `id`'s `index`.
fn right_on(id: &'static str, index: i32) -> Step {
    pointer(move |_, _| Some(vec![(0, Ev::Right(Point::Control(id, index)))]))
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

/// A placed piece pressed, dragged two cells (one at a time) and let go there.
fn drag_two(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, way) = b.movable(&[2])?;
    Some(vec![
        (0, Ev::Press(cell_on(s, way, 0.0))),
        (150, Ev::Move(cell_on(s, way, 0.5))),
        (200, Ev::Move(cell_on(s, way, 1.0))),
        (350, Ev::Move(cell_on(s, way, 1.5))),
        (400, Ev::Move(cell_on(s, way, 2.0))),
        (550, Ev::Release(cell_on(s, way, 2.0))),
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

/// A program dragged out of its row (the first not placed) onto the board, and let go where it fits.
fn row_drag(_: &Board, ui: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let rows = ui.global::<NavicustView>().get_programs();
    let row = rows.iter().take(12).position(|r| r.state == nettai_ui::ProgramState::None && !r.plus)? as i32;
    let mid = Board::of(ui).n as f32 / 2.0 - 0.5;
    Some(vec![
        (0, Ev::Press(Point::Row(row))),
        (150, Ev::Move(Point::Cell(mid + 2.0, mid))),
        (300, Ev::Move(Point::Fit)),
        (450, Ev::Release(Point::Fit)),
        // (Where it found no fit, the drag ends off the board.)
        (600, Ev::Release(Point::Off)),
    ])
}

/// A placed piece dragged onto another and let go there: it shakes and goes back.
fn drag_refused(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, _) = b.movable(&[])?;
    let piece = b.piece_of(s);
    let other = b.filled().into_iter().find(|c| !piece.contains(c))?;
    let mid = Point::Cell((s.0 + other.0) as f32 / 2.0, (s.1 + other.1) as f32 / 2.0);
    let to = Point::Cell(other.0 as f32, other.1 as f32);
    Some(vec![(0, Ev::Press(cell_on(s, (0, 0), 0.0))), (120, Ev::Move(mid)), (240, Ev::Move(to)), (360, Ev::Release(to))])
}

/// A placed piece dragged a cell, a key pressed (the drag ends, the piece goes back), the pointer moved on and let
/// go (nothing more happens).
fn drag_key(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let (s, way) = b.movable(&[1]).or_else(|| b.movable(&[]))?;
    Some(vec![
        (0, Ev::Press(cell_on(s, way, 0.0))),
        (150, Ev::Move(cell_on(s, way, 1.0))),
        (300, Ev::Key(KeyCode::ArrowRight)),
        (450, Ev::Move(cell_on(s, way, 2.0))),
        (600, Ev::Release(cell_on(s, way, 2.0))),
    ])
}

/// A placed piece dragged toward the middle and right-clicked: put away into the list (off the board).
fn drag_right(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let mid = Point::Cell(b.n as f32 / 2.0 - 0.5, b.n as f32 / 2.0 - 0.5);
    Some(vec![(0, Ev::Press(cell_on(s, (0, 0), 0.0))), (150, Ev::Move(mid)), (300, Ev::Right(mid)), (450, Ev::Release(mid))])
}

/// A placed piece dragged off the board and let go there: put away into the list.
fn drag_off(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    Some(vec![(0, Ev::Press(cell_on(s, (0, 0), 0.0))), (150, Ev::Move(Point::Cell(b.n as f32, s.1 as f32))), (300, Ev::Move(Point::Off)), (450, Ev::Release(Point::Off))])
}

/// A placed piece clicked (taken up), turned twice by the wheel's notches, and right-clicked: put away.
fn click_wheel_right(b: &Board, _: &nettai_ui::AppWindow) -> Option<Vec<(u64, Ev)>> {
    let s = *b.filled().first()?;
    let at = cell_on(s, (0, 0), 0.0);
    Some(vec![(0, Ev::Press(at)), (50, Ev::Release(at)), (200, Ev::Wheel(at, -60.0)), (400, Ev::Wheel(at, -60.0)), (650, Ev::Right(at))])
}

/// Into the library from the build's cards (on a phone, its sheet), if the cursor isn't there; and back out.
fn library() -> Step {
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
fn out_of_library() -> Step {
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
}

/// The tabs' steps in language `lang` at size `size`. `shot` takes a picture under a name (and compares the two
/// of a pair).
pub(super) fn steps(add: &mut dyn FnMut(u64, Step), shot: Shot, _lang: &'static str, size: &'static str) {
    open_build(add, size == "phone");
    navicust(add, shot);
    patch_cards(add, shot);
    auto_battle(add, shot);
    add(SETTLE, end());
}

/// The tabs' bursts of the motion probe (`burst` takes one: a name, the milliseconds after its trigger at which a
/// shot is taken, and the trigger): a piece taken up (it lifts), turned (a quarter turn), put down (a pulse along
/// its cells), refused (it shakes), and a bug appearing (it pulses purple).
pub(super) fn motion(add: &mut dyn FnMut(u64, Step), burst: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, &[u64], Step)) {
    open_build(add, false);
    add(SETTLE, to_tab(EditorTab::Navicust));
    // A right click on a folder's entry, the cursor elsewhere: the cursor comes to it, and it is emptied.
    add(SETTLE, to_tab(EditorTab::Folder));
    add(SETTLE, focus("slot", 0));
    burst(add, "folder-right-click", &[0, 60, 120, 200, 320, 480], right_on("slot", 3));
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
    // A placed piece dragged with the mouse: pressed (it lifts), two cells on (it follows the pointer from cell to
    // cell, outlined where it would land), let go (put down there, with its pulse).
    burst(add, "navicust-drag", &[0, 60, 120, 170, 230, 300, 370, 430, 500, 580, 640, 720, 900], pointer(drag_two));
    // The mouse by clicks: a piece clicked up follows the pointer with no button held, cell by cell; a key moves it
    // from there and the pointer's next move takes it back under the pointer; a click puts it down.
    burst(add, "navicust-click-move", &[0, 120, 250, 350, 470, 570, 650, 760, 900, 1100, 1300], pointer(click_move));
    // A program dragged out of its row onto the board and let go where it fits; a piece let go over another
    // (it shakes and goes back); a key mid-drag (the piece goes back); a right click mid-drag and a drag off the
    // board (each puts the piece away); the wheel turning a clicked-up piece, and a right click putting it away.
    burst(add, "navicust-row-drag", &[0, 100, 200, 350, 500, 600, 800], pointer(row_drag));
    burst(add, "navicust-drag-refused", &[0, 100, 200, 300, 420, 500, 600, 700, 800, 1000], pointer(drag_refused));
    burst(add, "navicust-drag-key", &[0, 100, 200, 350, 500, 650, 800], pointer(drag_key));
    burst(add, "navicust-drag-right-click", &[0, 100, 200, 350, 500, 650], pointer(drag_right));
    burst(add, "navicust-drag-off", &[0, 100, 200, 350, 500, 650], pointer(drag_off));
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
    // A right click on that card's row in the library (taken out, flying back; in a game with slots, its slot
    // emptied), and it put back; then one on the build's card (taken out).
    burst(add, "cards-right-click-library", &[0, 30, 60, 90, 120, 160, 220, 320, 480], right_on("lib.rows", 0));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, out_of_library());
    burst(add, "cards-right-click", &[0, 30, 60, 90, 120, 160, 220, 320, 480], right_on("card", 0));
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
    // A right click on the place just filled, the cursor on the next: the cursor comes back to it, and it is emptied.
    burst(add, "auto-right-click", &[0, 60, 120, 200, 320, 480], right_on("places.place", 0));
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
