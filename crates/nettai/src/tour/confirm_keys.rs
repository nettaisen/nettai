//! The menus' keys in the screenshot pass, pressed as the window presses them (`App::key_of`, the window's
//! hook with the event's parts; what it lets through goes on to Slint, as the hook lets it): Enter confirms
//! and Esc goes back, and the keys bound to the GBA's A and B (Z and X) do neither (the user, after trying
//! them: "actually z and x for navigation is not very good, enter + esc was fine"); a text field takes its
//! letters, finishes with Enter and is left with Esc. A pad's buttons can't be pressed so (they come from the
//! system's gamepad events): its bound A and B are asked of the bindings and acted, as `App::read_pad` acts
//! them, rebound too. Each step checks what was done ([`Tour::key_wrong`]). (A module of its own name: the
//! pass's `keys` is the GBA's.)

use super::{Pair, SETTLE, Step, Tour};
use crate::errors::UserError;
use crate::input::{Bindings, button_index};
use gamepad_facade::Button;
use crate::screens::ScreenId;
use crate::shell::App;
use nettai_battle::input::keys;
use slint::ComponentHandle;
use slint::platform::{Key, WindowEvent};
use slint::winit_030::winit::keyboard::KeyCode;
use std::cell::RefCell;
use std::rc::Rc;

/// A key pressed and let go, as the window has it: the app first; what it doesn't take, Slint (a text field
/// being typed in).
fn press(code: KeyCode, text: &'static str) -> Step {
    Box::new(move |ui, app, _| {
        let typed = (!text.is_empty()).then(|| text.to_string());
        let taken = app.borrow_mut().key_of(code, true, typed, false);
        if !taken {
            let text: slint::SharedString = match code {
                KeyCode::Enter => Key::Return.into(),
                KeyCode::Escape => Key::Escape.into(),
                _ => text.into(),
            };
            ui.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
            ui.window().dispatch_event(WindowEvent::KeyReleased { text });
        }
        app.borrow_mut().key_of(code, false, None, false);
    })
}

/// A check of what the keys did: `wrong` says what isn't so, if anything.
fn check(wrong: impl Fn(&App) -> Option<String> + 'static) -> Step {
    Box::new(move |_, app, tour: &Rc<Tour>| {
        if let Some(said) = wrong(&app.borrow()) {
            tour.key_wrong(said);
        }
    })
}

fn step(f: impl Fn(&mut App) + 'static) -> Step {
    Box::new(move |_, app, _| f(&mut app.borrow_mut()))
}

/// A pad's button, acted as `App::read_pad` acts it: what the bindings make of it in the menus.
fn pad(button: Button) -> Step {
    Box::new(move |_, app, _| {
        let action = app.borrow().shell.bindings.pad_action(button);
        if let Some(action) = action {
            app.borrow_mut().act(action);
        }
    })
}

/// The steps, after the shell's dialog: `add` takes a step and the time it is given first; `shot` a shot's
/// steps.
pub fn steps(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair)) {
    // (From a known place: the game loaded (alone, these steps may come before it is), no battle, which would
    // take the keys, the default bindings.)
    add(SETTLE, Box::new(|_, _, tour: &Rc<Tour>| tour.wait_until(|app| app.shell.ready().is_some())));
    add(
        SETTLE,
        step(|app| {
            crate::screens::practice::end(&mut app.shell);
            if app.shell.battle.is_some() {
                app.shell.end_battle();
            }
            app.shell.set_bindings(Bindings::default());
            app.go_now(ScreenId::Menu);
        }),
    );
    // A dialog: Z (A's key) doesn't answer it; Enter does.
    add(SETTLE, step(|app| app.shell.fail(UserError::Other)));
    add(SETTLE, press(KeyCode::KeyZ, "z"));
    add(SETTLE, check(|app| app.shell.dialog.is_none().then(|| "Z (A's key) answered the dialog".into())));
    add(SETTLE, press(KeyCode::Enter, ""));
    add(SETTLE, check(|app| app.shell.dialog.is_some().then(|| "Enter didn't answer the dialog".into())));
    // A screen: X (B's key) doesn't go back from Settings, the focus off its name's field; Esc does.
    add(SETTLE, step(|app| app.go_now(ScreenId::Settings)));
    add(SETTLE, step(|app| app.shell.nav.focus("set.picture", 0)));
    shot(add, "keys-hints", Pair::None);
    // (The Controls, whose A and B say they are the pad's confirm and back.)
    add(SETTLE, step(|app| app.shell.nav.focus("set.key", 4)));
    shot(add, "keys-controls", Pair::None);
    add(SETTLE, step(|app| app.shell.nav.focus("set.picture", 0)));
    add(SETTLE, press(KeyCode::KeyX, "x"));
    add(SETTLE, check(|app| (app.shell.screen() != ScreenId::Settings).then(|| "X (B's key) went back from Settings".into())));
    add(SETTLE, press(KeyCode::Escape, ""));
    add(SETTLE, check(|app| (app.shell.screen() != ScreenId::Menu).then(|| format!("Esc didn't go back from Settings: {:?}", app.shell.screen()))));
    // A text field: a letter starts editing it, X types, Enter finishes it; Esc leaves another edit as it was.
    // The field's own hints show while it is typed in.
    let name: Rc<RefCell<String>> = Default::default();
    let kept = name.clone();
    add(
        SETTLE,
        step(move |app| {
            *kept.borrow_mut() = app.shell.settings.name.clone();
            app.go_now(ScreenId::Settings);
        }),
    );
    add(SETTLE, step(|app| app.shell.nav.focus("set.name", 0)));
    add(SETTLE, press(KeyCode::KeyQ, "q"));
    add(SETTLE, press(KeyCode::KeyX, "x"));
    shot(add, "keys-field", Pair::None);
    add(SETTLE, check(|app| (!app.shell.nav.editing()).then(|| "Q didn't start editing the name, or X left it".into())));
    add(SETTLE, press(KeyCode::Enter, ""));
    add(
        SETTLE,
        check(|app| {
            (app.shell.nav.editing() || app.shell.settings.name != "qx")
                .then(|| format!("Enter didn't finish the name as typed: editing {}, {:?}", app.shell.nav.editing(), app.shell.settings.name))
        }),
    );
    add(SETTLE, press(KeyCode::KeyW, "w"));
    add(SETTLE, press(KeyCode::Escape, ""));
    add(SETTLE, check(|app| (app.shell.nav.editing() || app.shell.settings.name != "qx").then(|| format!("Esc didn't leave the name as it was: {:?}", app.shell.settings.name))));
    let back = name.clone();
    add(SETTLE, step(move |app| app.shell.set_name(&back.borrow())));
    // The pad: its A (south) answers a dialog and its B (west) goes back; A rebound to east, east answers and
    // south doesn't.
    add(SETTLE, step(|app| app.shell.nav.focus("set.picture", 0)));
    add(SETTLE, step(|app| app.shell.fail(UserError::Other)));
    add(SETTLE, pad(Button::South));
    add(SETTLE, check(|app| app.shell.dialog.is_some().then(|| "the pad's A (south) didn't answer the dialog".into())));
    add(SETTLE, pad(Button::West));
    add(SETTLE, check(|app| (app.shell.screen() != ScreenId::Menu).then(|| format!("the pad's B (west) didn't go back from Settings: {:?}", app.shell.screen()))));
    add(
        SETTLE,
        step(|app| {
            let mut bindings = Bindings::default();
            bindings.bind_pad(button_index(keys::A), Button::East);
            app.shell.set_bindings(bindings);
        }),
    );
    add(SETTLE, step(|app| app.shell.fail(UserError::Other)));
    add(SETTLE, pad(Button::South));
    add(SETTLE, check(|app| app.shell.dialog.is_none().then(|| "south answered the dialog with the pad's A on east".into())));
    add(SETTLE, pad(Button::East));
    add(SETTLE, check(|app| app.shell.dialog.is_some().then(|| "east (the pad's A, rebound) didn't answer the dialog".into())));
    add(SETTLE, step(|app| app.shell.set_bindings(Bindings::default())));
}
