//! nettai: the app. A title, offline play against the stand-in, the netplay
//! lobby and the replays, around a battle played through nettai-frontend,
//! in a Slint window (docs/app.md).
//!
//! The window's rendering is the clock: before each frame is drawn the app
//! runs (`App::frame`: the battle's ticks due, its picture presented for
//! that frame, the sound), and after it asks for the next frame, so it runs
//! at the display's rate. The battle's keys come from the window's events
//! before Slint's focus sees them; the menus are Slint's, navigated by the
//! keys, a gamepad, the mouse or touch alike.
//!
//! `NETTAI_PLAY_STATS` prints what the battle's picture costs and how late
//! it is, every two seconds; `NETTAI_KEY_PROBE` presses the right arrow now
//! and then to measure it; `NETTAI_TOUR=<folder>` walks every screen and
//! writes each to a PNG (`tour`).

mod app;
mod arenas;
mod games;
mod input;
mod lang;
mod lobby;
mod netplay;
mod replays;
mod sound;
mod stage;
mod tour;

use app::App;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, RenderingState};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

slint::include_modules!();

thread_local! {
    /// The app's state, on the UI's thread (work done on other threads
    /// comes back to it through the event loop: [`with_app`]).
    static APP: std::cell::OnceCell<Rc<RefCell<App>>> = const { std::cell::OnceCell::new() };
}

/// Run `f` on the app's state, on the UI's thread (from a closure given to
/// `slint::invoke_from_event_loop`).
pub fn with_app(f: impl FnOnce(&mut App)) {
    if let Some(app) = APP.with(|a| a.get().cloned()) {
        f(&mut app.borrow_mut());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // (An engine stop is the session's to report, as a value: kept off
    // stderr.)
    nettai_frontend::session::quiet_engine_panics();
    let tour = std::env::var_os("NETTAI_TOUR").map(std::path::PathBuf::from);
    let mut backend = slint::BackendSelector::new().backend_name("winit".into());
    // (`NETTAI_RENDERER`: skia, femtovg or software; else Slint's choice.)
    if let Ok(renderer) = std::env::var("NETTAI_RENDERER") {
        backend = backend.renderer_name(renderer);
    }
    if tour.is_some() {
        // (The tour's window doesn't take the keyboard.)
        backend = backend.with_winit_window_attributes_hook(|a| a.with_active(false));
    }
    backend.select()?;
    let ui = AppWindow::new()?;
    ui.set_version(format!("nettai {}", env!("CARGO_PKG_VERSION")).into());
    let app = Rc::new(RefCell::new(App::new(&ui)));
    APP.with(|a| a.set(app.clone())).unwrap_or_else(|_| unreachable!("the app starts once"));
    app.borrow_mut().load_games();
    wire(&ui, &app);

    // The clock: before each frame is drawn, the app's frame; after it, the
    // next frame asked for. (The renderer tells: femtovg's does, on OpenGL
    // and on wgpu. Slint's own frames, on Apple's systems, come from the
    // display link and draw directly, with no window event before them.)
    let (weak, state) = (ui.as_weak(), app.clone());
    ui.window()
        .set_rendering_notifier(move |s, _| match s {
            RenderingState::BeforeRendering => {
                let nav = state.borrow_mut().frame(Instant::now());
                if let Some(ui) = weak.upgrade() {
                    for a in nav {
                        ui.invoke_nav(a);
                    }
                }
            }
            RenderingState::AfterRendering => {
                state.borrow_mut().drawn();
                if let Some(ui) = weak.upgrade() {
                    ui.window().request_redraw();
                }
            }
            _ => {}
        })
        .map_err(|e| slint::PlatformError::Other(format!("the window can't tell the app when it draws: {e:?}")))?;

    // The battle's keys, by where they are, before Slint's focus.
    let state = app.clone();
    ui.window().on_winit_window_event(move |_, event| match event {
        winit::event::WindowEvent::KeyboardInput { event, .. } => {
            if state.borrow_mut().key(event) { EventResult::PreventDefault } else { EventResult::Propagate }
        }
        winit::event::WindowEvent::Focused(false) => {
            state.borrow_mut().keys.release();
            EventResult::Propagate
        }
        _ => EventResult::Propagate,
    });

    if std::env::var_os("NETTAI_KEY_PROBE").is_some() {
        key_probe();
    }
    if let Some(dir) = tour {
        tour::start(&ui, &app, dir);
    }
    ui.invoke_focus_keys();
    ui.run()
}

/// `NETTAI_KEY_PROBE`: the right arrow pressed every 300 to 400 ms (at no
/// particular point between two frames, through the event loop as a key's
/// event comes) and let go 100 ms later.
fn key_probe() {
    std::thread::spawn(|| {
        for i in 0u64.. {
            std::thread::sleep(Duration::from_micros(300_000 + i * 7_919 % 100_000));
            if slint::invoke_from_event_loop(|| with_app(|app| app.probe(true))).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
            if slint::invoke_from_event_loop(|| with_app(|app| app.probe(false))).is_err() {
                return;
            }
        }
    });
}

/// The screens' requests, to the app.
fn wire(ui: &AppWindow, app: &Rc<RefCell<App>>) {
    let on = |f: fn(&mut App)| {
        let app = app.clone();
        move || f(&mut app.borrow_mut())
    };
    let a = app.clone();
    ui.on_go(move |screen| a.borrow_mut().go(screen));
    ui.on_quit(|| {
        let _ = slint::quit_event_loop();
    });
    let a = app.clone();
    ui.on_sound(move |s| a.borrow_mut().sound.play(s));
    let a = app.clone();
    ui.on_play_game(move |i| a.borrow_mut().play_game(i as usize));
    let a = app.clone();
    ui.on_play_source(move |i| a.borrow_mut().play_source(i as usize));
    ui.on_play_reroll(on(App::play_reroll));
    ui.on_play_fight(on(App::play_fight));
    ui.on_battle_resume(on(App::battle_resume));
    ui.on_battle_restart(on(App::battle_restart));
    ui.on_battle_quit(on(App::battle_quit));
    ui.on_battle_rematch(on(App::battle_rematch));
    ui.on_battle_language(on(App::battle_language));
    let a = app.clone();
    ui.on_battle_touch(move |bit, down| {
        let mut app = a.borrow_mut();
        if down {
            app.touch |= bit as u16;
        } else {
            app.touch &= !(bit as u16);
        }
    });
    let a = app.clone();
    ui.on_lobby_mode(move |m| a.borrow_mut().lobby_mode(m));
    let a = app.clone();
    ui.on_lobby_code(move |c| a.borrow_mut().lobby_code(&c));
    let a = app.clone();
    ui.on_lobby_game(move |g| a.borrow_mut().lobby_game(g.max(0) as usize));
    ui.on_lobby_ready(on(App::lobby_ready));
    ui.on_lobby_copy(on(App::lobby_copy));
    let a = app.clone();
    ui.on_replays_watch(move |i, side| a.borrow_mut().replays_watch(i.max(0) as usize, side.clamp(0, 1) as u8));
    let a = app.clone();
    ui.on_settings_step(move |row, by| a.borrow_mut().settings_step(row, by));
    let a = app.clone();
    ui.on_set_name(move |n| a.borrow_mut().set_name(&n));
}
