//! The Builds screens' steps of the screenshot pass (`crate::tour`; docs/app.md §13): the list with none and
//! with some, a new build's dialog, a file that is no save dropped on the list, a build's rename and its
//! deletion's question, and the editor's Navi and
//! Folder tabs (an entry marked the Regular chip, two tagged, a mark the rules refuse), with pairs a focus move
//! apart. The builds are the pass's own (`paths::keep_builds_in`): made
//! here through the screen, as a player makes them.

use crate::input::{Action, KeyPress};
use crate::screens::ScreenId;
use crate::shell::App;
use crate::tour::{Pair, Step};
use nettai_ui::{BuildsPage, BuildsView, EditorTab};
use slint::ComponentHandle;
use slint::winit_030::winit::keyboard::KeyCode;

/// The walk is where its last picture's name says: on tab `tab` of the editor. If it isn't, the report says so
/// (as a text the pass counts against the picture, so the pass fails): a picture of another tab under this name
/// would pass for it, the pass checking only what moves and what is cut.
fn on_tab(tab: EditorTab, name: &'static str) -> Step {
    Box::new(move |ui, _, _| {
        let view = ui.global::<BuildsView>();
        if view.get_page() != BuildsPage::Editor || view.get_shown() != tab {
            ui.global::<nettai_ui::Audit>().invoke_clipped(format!("(the walk isn't on the editor's {name} tab: this picture is of something else)").into());
        }
    })
}

/// The names of the builds made: the second in another script, the third long.
const NAMES: [&str; 3] = ["Falzar heat", "ジャッジ対策 v3", "Tournament build 10-18 with a long name"];

const SETTLE: u64 = crate::tour::SETTLE;

// What the walk does: an action, the focus put on a control, and a call of the screen's own module.
fn act(action: Action) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().act(action))
}
fn focus(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().shell.nav.focus(id, index))
}
fn press(id: &'static str, index: i32) -> Step {
    Box::new(move |_, app, _| {
        let mut app = app.borrow_mut();
        app.shell.nav.focus(id, index);
        app.act(Action::Confirm);
    })
}
fn screen(f: impl Fn(&mut dyn crate::screens::Screen, &mut crate::shell::Shell) + 'static) -> Step {
    Box::new(move |_, app, _| app.borrow_mut().with_screen(ScreenId::Builds, &f))
}
fn letter(code: KeyCode, text: &'static str) -> Step {
    screen(move |s, shell| {
        s.key(shell, &KeyPress { code, text: Some(text.into()), repeat: false, shift: false });
    })
}
/// Make a build named `name` of the game's version `version` (its place in the game's order).
fn make(add: &mut dyn FnMut(u64, Step), name: &'static str, version: i32) {
    add(SETTLE, press("new", 0));
    add(SETTLE, screen(move |s, shell| s.edit(shell, "name.name", 0, name, true)));
    add(SETTLE, screen(move |s, shell| s.choose(shell, "name.version", 0, version)));
    add(SETTLE, press("name.confirm", 0));
}
/// The Builds screen with no builds (the pass's own folder, emptied).
fn fresh() -> Step {
    Box::new(|_, app, _| {
        let mut app = app.borrow_mut();
        if let Some(game) = app.shell.game().map(str::to_string) {
            let _ = std::fs::remove_dir_all(crate::builds::store::folder(&game));
        }
        leave(&mut app);
        app.go_now(ScreenId::Builds);
        app.shell.nav.show_cursor(true, false);
    })
}

/// The Builds screens' bursts of the motion probe (`burst` takes one: a name, the milliseconds after its
/// trigger at which a shot is taken, and the trigger): the editor's body arriving as the tab changes, and the
/// Folder tab's own pieces: a chip put in (it flies from its row to its entry, the entry flashes, and a count
/// that changed ticks), and a chip the rules refuse (its row shakes).
pub fn motion(add: &mut dyn FnMut(u64, Step), burst: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, &[u64], Step)) {
    add(SETTLE, fresh());
    make(add, NAMES[0], 1);
    add(SETTLE, press("builds", 0));
    add(SETTLE, press("tab", 0));
    burst(add, "builds-tab", &[0, 30, 60, 100, 160, 260], press("tab", 1));
    add(90, on_tab(EditorTab::Folder, "Folder"));
    // The library by MB, the most first: its third row is a Giga chip of this build's version (the folder has
    // none: the count goes from 0 to 1), its fourth the other version's.
    add(SETTLE, focus("slot", 13));
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 3)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 3));
    burst(add, "builds-refused", &[0, 30, 60, 90, 120, 160, 220, 320], act(Action::Confirm));
    add(SETTLE, focus("lib.rows", 2));
    burst(add, "builds-put", &[0, 30, 60, 90, 120, 160, 200, 260, 340, 480, 700], act(Action::Confirm));
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            leave(&mut app);
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// The Builds screens' steps in language `lang` at size `size`. `shot` takes a picture under a name (and
/// compares the two of a pair).
pub fn walk(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair), lang: &'static str, size: &'static str) {
    let phone = size == "phone";
    // (A search finds chips by their names in the language shown.)
    let search = if lang == "ja" { "ソード" } else { "sword" };

    // The list with no builds (the pass's own folder, emptied), and a new build's dialog.
    add(SETTLE, fresh());
    shot(add, "builds-empty", Pair::None);
    add(SETTLE, press("new", 0));
    shot(add, "builds-new", Pair::First);
    add(SETTLE, act(Action::Down));
    shot(add, "builds-new-focus", Pair::Second);
    add(SETTLE, act(Action::Right));
    shot(add, "builds-new-version", Pair::None);
    add(SETTLE, act(Action::Back));
    make(add, NAMES[0], 1);
    make(add, NAMES[1], 0);
    make(add, NAMES[2], 1);

    // The list: the chosen build's overview beside it; its buttons a focus move apart; its dialogs.
    add(SETTLE, focus("builds", 0));
    shot(add, "builds-list", Pair::None);
    // A file that is no save of the game, dropped on the list: the typed refusal's dialog.
    add(
        SETTLE,
        Box::new(|_, app, _| {
            let folder = crate::paths::builds();
            let path = folder.join("not-a-save.sav");
            let _ = std::fs::create_dir_all(&folder);
            let _ = std::fs::write(&path, b"no save of any game");
            app.borrow_mut().dropped(&path);
        }),
    );
    shot(add, "builds-import-refused", Pair::None);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, focus("builds", 0));
    if phone {
        add(SETTLE, act(Action::Confirm));
        add(SETTLE, focus("one.edit", 0));
        shot(add, "builds-one", Pair::First);
        add(SETTLE, act(Action::Right));
        shot(add, "builds-one-focus", Pair::Second);
        add(SETTLE, press("one.rename", 0));
        shot(add, "builds-rename", Pair::None);
        add(SETTLE, act(Action::Back));
        add(SETTLE, press("one.delete", 0));
        shot(add, "builds-delete", Pair::None);
        add(SETTLE, act(Action::Confirm));
        add(SETTLE, press("one.edit", 0));
    } else {
        add(SETTLE, focus("edit", 0));
        shot(add, "builds-buttons", Pair::First);
        add(SETTLE, act(Action::Right));
        shot(add, "builds-buttons-focus", Pair::Second);
        add(SETTLE, press("rename", 0));
        shot(add, "builds-rename", Pair::None);
        add(SETTLE, act(Action::Back));
        add(SETTLE, press("delete", 0));
        shot(add, "builds-delete", Pair::None);
        add(SETTLE, act(Action::Confirm));
        add(SETTLE, press("builds", 0));
    }

    // The editor's Navi tab. A game may have one navi and no choice, or thirteen and one: each step names the
    // control it is for, and does nothing where the game has none (an arrow from a lone card goes to whatever is
    // nearest that way, a tab's word: a walk by arrows took its later pictures on another tab).
    fn there(app: &App, id: &str, index: i32) -> bool {
        app.shell.nav.registry().borrow().rect(&crate::nav::Key::new(id, index)).is_some()
    }
    fn focus_if(id: &'static str, index: i32) -> Step {
        Box::new(move |_, app, _| {
            let mut app = app.borrow_mut();
            if there(&app, id, index) {
                app.shell.nav.focus(id, index);
            }
        })
    }
    fn press_if(id: &'static str, index: i32, action: Action) -> Step {
        Box::new(move |_, app, _| {
            let mut app = app.borrow_mut();
            if there(&app, id, index) {
                app.shell.nav.focus(id, index);
                app.act(action);
            }
        })
    }
    // The game's navis, the focus a card on; another navi chosen (the tabs that are the game's own navi's
    // greyed), and the first again.
    add(SETTLE, press("tab", 0));
    add(SETTLE, focus("navi", 0));
    shot(add, "builds-navi", Pair::First);
    add(90, on_tab(EditorTab::Navi, "Navi"));
    add(SETTLE, focus_if("navi", 1));
    shot(add, "builds-navi-focus", Pair::Second);
    add(SETTLE, press_if("navi", 1, Action::Confirm));
    shot(add, "builds-navi-other", Pair::None);
    add(90, on_tab(EditorTab::Navi, "Navi"));
    add(SETTLE, press("navi", 0));
    // In a game whose builds have a choice (Light or Dark): its other preset, with the face the round then
    // starts the navi with; and the first again.
    add(SETTLE, press_if("choice", 0, Action::Right));
    shot(add, "builds-navi-choice", Pair::None);
    add(90, on_tab(EditorTab::Navi, "Navi"));
    add(SETTLE, press_if("choice", 0, Action::Left));

    // The Folder tab. The library is beside the folder on a desktop and a sheet over it on a phone, so each step
    // says where it is: `library` goes from the entry under the cursor into the library if the cursor isn't
    // there, `folder` back out of it.
    fn library() -> Step {
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            if !in_library(&app) {
                app.act(Action::Confirm);
            }
        })
    }
    fn folder() -> Step {
        Box::new(|_, app, _| leave_library(&mut app.borrow_mut()))
    }
    // The folder, and the strip for the entry under the cursor; the library, its controls a focus move apart.
    add(SETTLE, press("tab", 1));
    add(SETTLE, focus("slot", 13));
    shot(add, "builds-folder", Pair::None);
    add(90, on_tab(EditorTab::Folder, "Folder"));
    add(SETTLE, library());
    shot(add, "builds-folder-library", Pair::None);
    add(SETTLE, focus("lib.search", 0));
    shot(add, "builds-folder-search", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "builds-folder-search-focus", Pair::Second);
    // The library by MB, the most first: the Giga chips, of which another version's say whose they are. A chip
    // that can't go in, asked for: it stays out (its row shakes, where things move). One that can: it is in
    // the entry, and the cursor on the next entry.
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 3)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 3));
    shot(add, "builds-folder-sorted", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "builds-folder-refused", Pair::None);
    add(SETTLE, library());
    add(SETTLE, press("lib.rows", 2));
    shot(add, "builds-folder-put", Pair::None);
    // A search, and one of what it finds put in.
    add(SETTLE, folder());
    add(SETTLE, library());
    add(SETTLE, screen(move |s, shell| s.edit(shell, "lib.search", 0, search, true)));
    shot(add, "builds-folder-found", Pair::None);
    add(SETTLE, press("lib.rows", 1));
    add(SETTLE, folder());
    // The Regular chip, an entry the rules take as one; two entries tagged, in a game whose folders have tag
    // chips. Then an entry whose chip is past the Regular memory: it isn't marked, and the strip says why.
    add(SETTLE, focus("slot", 0));
    add(SETTLE, letter(KeyCode::KeyR, "r"));
    add(SETTLE, focus("slot", 15));
    add(SETTLE, letter(KeyCode::KeyT, "t"));
    add(SETTLE, focus("slot", 16));
    add(SETTLE, letter(KeyCode::KeyT, "t"));
    shot(add, "builds-folder-marks", Pair::None);
    add(90, on_tab(EditorTab::Folder, "Folder"));
    add(SETTLE, focus("slot", 13));
    add(SETTLE, letter(KeyCode::KeyR, "r"));
    shot(add, "builds-folder-mark-refused", Pair::None);
    // A chip taken out: the header says what is wrong, and the tab is marked.
    add(SETTLE, focus("slot", 4));
    add(SETTLE, act(Action::Delete));
    shot(add, "builds-folder-short", Pair::None);
    add(90, on_tab(EditorTab::Folder, "Folder"));
    // The sort's list.
    add(SETTLE, library());
    add(SETTLE, press("lib.sort", 0));
    shot(add, "builds-folder-sort", Pair::None);
    add(SETTLE, act(Action::Back));

    // The tabs to come, as they are.
    add(SETTLE, folder());
    // (The tab after Folder, by the key that goes to it: a tab's place in the bar is its game's.)
    add(SETTLE, act(Action::Next));
    shot(add, "builds-navicust", Pair::None);
    add(90, on_tab(EditorTab::Navicust, "NaviCust"));

    add(
        SETTLE,
        Box::new(|_, app, _| {
            let mut app = app.borrow_mut();
            leave(&mut app);
            app.go_now(ScreenId::Menu);
        }),
    );
}

/// Out of whatever of the Builds screen is open (a dialog, the library, the editor), back to its list.
fn leave(app: &mut App) {
    for _ in 0..4 {
        if app.shell.screen() == ScreenId::Builds {
            app.with_screen(ScreenId::Builds, |s, shell| {
                s.back(shell);
            });
        }
    }
}

/// Whether the cursor is in the Folder tab's library (on a phone, its sheet is open).
fn in_library(app: &App) -> bool {
    app.shell.nav.is_open("lib.") || app.shell.nav.focused().is_some_and(|k| k.id.starts_with("lib."))
}

/// Out of the library (a phone's sheet), back to the folder.
fn leave_library(app: &mut App) {
    if in_library(app) {
        app.act(Action::Back);
    }
}
