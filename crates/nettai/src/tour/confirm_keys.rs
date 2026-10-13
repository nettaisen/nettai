//! The keys in the screenshot pass, pressed as the window presses them (`App::key_of`, the window's hook with
//! the event's parts; what it lets through goes on to Slint, as the hook lets it): the GBA's A and B confirm
//! and go back by the keys bound to them (the user: "i feel like the GBA A and B buttons should be the
//! universal confirm/cancel buttons everywhere, instead of enter+esc"), a text field takes its letters and
//! finishes with Enter, leaves with Esc, and A rebound confirms by its new key. Each step checks what the key
//! did ([`Tour::key_wrong`]). (A module of its own name: the pass's `keys` is the GBA's.)

use super::{Pair, SETTLE, Step, Tour};
use crate::errors::UserError;
use crate::input::{Bindings, button_index};
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

/// The steps, after the shell's dialog: `add` takes a step and the time it is given first; `shot` a shot's
/// steps.
pub fn steps(add: &mut dyn FnMut(u64, Step), shot: &dyn Fn(&mut dyn FnMut(u64, Step), &'static str, Pair)) {
    // Z (A) answers a dialog.
    add(SETTLE, step(|app| app.shell.fail(UserError::Other)));
    add(SETTLE, press(KeyCode::KeyZ, "z"));
    add(SETTLE, check(|app| app.shell.dialog.is_some().then(|| "Z (A) didn't answer the dialog".into())));
    // X (B) goes back from a screen: Settings, the focus off its name's field.
    add(SETTLE, step(|app| app.go_now(ScreenId::Settings)));
    add(SETTLE, step(|app| app.shell.nav.focus("set.picture", 0)));
    add(SETTLE, press(KeyCode::KeyX, "x"));
    add(SETTLE, check(|app| (app.shell.screen() != ScreenId::Menu).then(|| format!("X (B) didn't go back from Settings: {:?}", app.shell.screen()))));
    // A text field: a letter starts editing it, X types (it doesn't go back), Enter finishes it; Esc leaves
    // another edit as it was. The field's own hints show while it is typed in.
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
        check(move |app| {
            let wanted = "qx";
            (app.shell.nav.editing() || app.shell.settings.name != wanted)
                .then(|| format!("Enter didn't finish the name as typed: editing {}, {:?}", app.shell.nav.editing(), app.shell.settings.name))
                .or_else(|| (app.shell.screen() != ScreenId::Settings).then(|| "X in the field went back".into()))
        }),
    );
    add(SETTLE, press(KeyCode::KeyW, "w"));
    add(SETTLE, press(KeyCode::Escape, ""));
    add(SETTLE, check(|app| (app.shell.nav.editing() || app.shell.settings.name != "qx").then(|| format!("Esc didn't leave the name as it was: {:?}", app.shell.settings.name))));
    let back = name.clone();
    add(SETTLE, step(move |app| app.shell.set_name(&back.borrow())));
    // A rebound to J: J answers a dialog, Z no longer does, and the hints say J.
    add(
        SETTLE,
        step(|app| {
            let mut bindings = Bindings::default();
            bindings.bind_key(button_index(keys::A), KeyCode::KeyJ);
            app.shell.set_bindings(bindings);
            app.shell.nav.focus("set.picture", 0);
        }),
    );
    shot(add, "keys-rebound", Pair::None);
    add(SETTLE, step(|app| app.shell.fail(UserError::Other)));
    add(SETTLE, press(KeyCode::KeyZ, "z"));
    add(SETTLE, check(|app| app.shell.dialog.is_none().then(|| "Z answered the dialog with A on J".into())));
    add(SETTLE, press(KeyCode::KeyJ, "j"));
    add(SETTLE, check(|app| app.shell.dialog.is_some().then(|| "J (A, rebound) didn't answer the dialog".into())));
    add(SETTLE, step(|app| app.shell.set_bindings(Bindings::default())));
}
