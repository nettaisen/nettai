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

/// Back, if a piece is in hand (it goes back where it came from); nothing, if none is (Back would leave the editor).
fn put_back() -> Step {
    Box::new(|ui, app, tour| {
        if !tour.passing.get() && ui.global::<NavicustView>().get_holding() {
            app.borrow_mut().act(Action::Back);
        }
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
    // list with Back (no longer placed). (The piece on the ring is placed: Back puts one in hand back, where the
    // game has no ring, and with nothing in hand would leave the editor.)
    add(SETTLE, put_back());
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

/// The mouse pressed on a placed piece of the board, moved two cells (one at a time, where the piece fits two cells
/// on: right, left, down or up) and let go, as the window's pointer events: each a timer's, `at` ms on.
fn drag_piece() -> Step {
    Box::new(|ui, app, tour| {
        if tour.passing.get() {
            return;
        }
        let view = ui.global::<NavicustView>();
        let n = view.get_columns();
        let cells: Vec<nettai_ui::GridCell> = view.get_cells().iter().collect();
        let Some(start) = cells.iter().position(|c| c.filled) else { return };
        // (The piece: the filled cells of its color joined to the one pressed.)
        let (x0, y0) = (start as i32 % n, start as i32 / n);
        let mut piece = vec![(x0, y0)];
        let mut i = 0;
        while i < piece.len() {
            let (x, y) = piece[i];
            for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                let same = (0..n).contains(&nx) && (0..n).contains(&ny) && {
                    let c = &cells[(ny * n + nx) as usize];
                    c.filled && c.tone == cells[start].tone
                };
                if same && !piece.contains(&(nx, ny)) {
                    piece.push((nx, ny));
                }
            }
            i += 1;
        }
        let free = |x: i32, y: i32| (0..n).contains(&x) && (0..n).contains(&y) && {
            let c = &cells[(y * n + x) as usize];
            c.kind == nettai_ui::CellKind::Board && (!c.filled || piece.contains(&(x, y)))
        };
        let Some((dx, dy)) = [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().find(|&(dx, dy)| piece.iter().all(|&(x, y)| free(x + 2 * dx, y + 2 * dy))) else { return };
        let registry = app.borrow().shell.nav.registry();
        let Some(r) = registry.borrow().rect(&crate::nav::Key::new("cell", start as i32)) else { return };
        // (The board's cells are 3 apart.)
        let pitch = r.w + 3.0;
        let at =|k: f32| slint::LogicalPosition::new(r.x + r.w / 2.0 + k * dx as f32 * pitch, r.y + r.h / 2.0 + k * dy as f32 * pitch);
        use slint::platform::{PointerEventButton, WindowEvent};
        let button = PointerEventButton::Left;
        let events = [
            (0, WindowEvent::PointerPressed { position: at(0.0), button }),
            (150, WindowEvent::PointerMoved { position: at(0.5) }),
            (200, WindowEvent::PointerMoved { position: at(1.0) }),
            (350, WindowEvent::PointerMoved { position: at(1.5) }),
            (400, WindowEvent::PointerMoved { position: at(2.0) }),
            (550, WindowEvent::PointerReleased { position: at(2.0), button }),
        ];
        for (ms, event) in events {
            let ui = ui.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(ms), move || {
                if let Some(ui) = ui.upgrade() {
                    ui.window().dispatch_event(event);
                }
            });
        }
    })
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
    burst(add, "navicust-drag", &[0, 60, 120, 170, 230, 300, 370, 430, 500, 580, 640, 720, 900], drag_piece());
    // A card put in (it flies from its row to the build's, which flashes, and the MB tick), and one the MB left
    // can't take (its row shakes).
    add(SETTLE, to_tab(EditorTab::PatchCards));
    add(SETTLE, focus("card", 0));
    add(SETTLE, library());
    add(SETTLE, focus("lib.rows", 0));
    burst(add, "cards-put", &[0, 30, 60, 90, 120, 160, 200, 260, 340, 480], act(Action::Confirm));
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
