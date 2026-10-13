//! The Builds screens' steps of the screenshot pass (`crate::tour`; docs/app.md §13): the list with none and
//! with some, a new build's dialog, a build's rename and its deletion's question, and the editor's Navi and
//! Folder tabs (an entry marked the Regular chip, two tagged, a mark the rules refuse), with pairs a focus move
//! apart. The builds are the pass's own (`paths::keep_builds_in`): made
//! here through the screen, as a player makes them.

use crate::input::{Action, KeyPress};
use crate::screens::ScreenId;
use crate::shell::App;
use crate::tour::{Pair, Step};
use slint::winit_030::winit::keyboard::KeyCode;

/// The names of the builds made: the second in another script, the third long.
const NAMES: [&str; 3] = ["Falzar heat", "ジャッジ対策 v3", "Tournament build 10-18 with a long name"];

/// The Builds screens' steps in language `lang` at size `size`. `shot` takes a picture under a name (and
/// compares the two of a pair).
pub fn walk(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair), lang: &'static str, size: &'static str) {
    const SETTLE: u64 = crate::tour::SETTLE;
    let phone = size == "phone";
    // (A search finds chips by their names in the language shown.)
    let search = if lang == "ja" { "ソード" } else { "sword" };
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

    // The list with no builds (the pass's own folder, emptied), and a new build's dialog.
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

    // The editor's Navi tab: the game's navis, the focus a card on; another navi chosen (the tabs that are the
    // game's own navi's greyed), and the first again.
    add(SETTLE, press("tab", 0));
    add(SETTLE, focus("navi", 0));
    shot(add, "builds-navi", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "builds-navi-focus", Pair::Second);
    add(SETTLE, act(Action::Confirm));
    shot(add, "builds-navi-other", Pair::None);
    add(SETTLE, press("navi", 0));

    // The Folder tab: the folder, the strip for the entry under the cursor; the library's controls a focus
    // move apart; the library sorted so that what can't go in shows why; a chip put in; a chip taken out (the
    // header says what is wrong); the Regular chip; a search; the sort's list.
    add(SETTLE, press("tab", 1));
    add(SETTLE, focus("slot", 13));
    shot(add, "builds-folder", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "builds-folder-library", Pair::None);
    add(SETTLE, focus("lib.search", 0));
    shot(add, "builds-folder-search", Pair::First);
    add(SETTLE, act(Action::Right));
    shot(add, "builds-folder-search-focus", Pair::Second);
    add(SETTLE, screen(|s, shell| s.choose(shell, "lib.sort", 0, 3)));
    add(SETTLE, press("lib.direction", 0));
    add(SETTLE, focus("lib.rows", 2));
    shot(add, "builds-folder-sorted", Pair::None);
    add(SETTLE, act(Action::Confirm));
    shot(add, "builds-folder-refused", Pair::None);
    add(SETTLE, screen(move |s, shell| s.edit(shell, "lib.search", 0, search, true)));
    add(SETTLE, press("lib.rows", 1));
    shot(add, "builds-folder-put", Pair::None);
    // (The Regular chip: an entry the rules take as one, then one whose chip is past the Regular memory, which
    // isn't marked and says why. Two entries tagged, in a game whose folders have tag chips.)
    add(SETTLE, focus("slot", 0));
    add(SETTLE, letter(KeyCode::KeyR, "r"));
    add(SETTLE, focus("slot", 15));
    add(SETTLE, letter(KeyCode::KeyT, "t"));
    add(SETTLE, focus("slot", 16));
    add(SETTLE, letter(KeyCode::KeyT, "t"));
    shot(add, "builds-folder-marks", Pair::None);
    add(SETTLE, focus("slot", 13));
    add(SETTLE, letter(KeyCode::KeyR, "r"));
    shot(add, "builds-folder-mark-refused", Pair::None);
    add(SETTLE, focus("slot", 4));
    add(SETTLE, act(Action::Delete));
    shot(add, "builds-folder-short", Pair::None);
    add(SETTLE, act(Action::Confirm));
    add(SETTLE, press("lib.sort", 0));
    shot(add, "builds-folder-sort", Pair::None);
    add(SETTLE, act(Action::Back));

    // The tabs to come, as they are.
    add(SETTLE, Box::new(|_, app, _| leave_library(&mut app.borrow_mut())));
    add(SETTLE, press("tab", 2));
    shot(add, "builds-navicust", Pair::None);

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

/// Out of the library (a phone's sheet), back to the folder.
fn leave_library(app: &mut App) {
    if app.shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) {
        app.act(Action::Back);
    }
}
