//! nettai: the app (docs/app.md). A main menu, Play, Training, Builds, Replays and Settings around a battle
//! played through nettai-frontend, in a Slint window whose UI is the nettai-ui crate.
//!
//! The window's rendering is the clock: before each frame is drawn the app runs (`App::frame`: the battle's
//! ticks due, its picture presented for that frame, the sound), and after it asks for the next frame, so it
//! runs at the display's rate. The keys come from the window's events before Slint sees them: the battle's by
//! where they are on the keyboard, timestamped; the menus' as the same few actions a gamepad's buttons are.
//!
//! `NETTAI_PLAY_STATS` prints what the battle's picture costs and how late it is, every two seconds;
//! `NETTAI_KEY_PROBE` presses the right arrow now and then to measure it; `NETTAI_TOUR=<folder>` walks every
//! screen and writes each to a PNG (`tour`).

mod arenas;
mod awake;
mod battle;
mod builds;
mod errors;
mod games;
mod gl;
mod input;
mod lang;
mod nav;
mod paths;
mod replays;
mod screens;
mod settings;
mod shell;
mod sound;
mod stage;
mod tour;

use nav::{Rect, Registry};
use nettai_ui::AppWindow;
use shell::{App, NavInput};
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};
use slint::{ComponentHandle, RenderingState};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

thread_local! {
    /// The app's state, on the window's thread (work done on other threads comes back to it through the event
    /// loop: [`with_app`]).
    static APP: std::cell::OnceCell<Rc<RefCell<App>>> = const { std::cell::OnceCell::new() };
    /// What the UI said while the app was busy (a callback that came in the middle of one of the app's own
    /// calls): taken up at the next frame.
    static WAITING: RefCell<Vec<NavInput>> = const { RefCell::new(Vec::new()) };
}

/// Run `f` on the app's state, on the window's thread (from a closure given to
/// `slint::invoke_from_event_loop`).
pub fn with_app(f: impl FnOnce(&mut App)) {
    if let Some(app) = APP.with(|a| a.get().cloned()) {
        f(&mut app.borrow_mut());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    // (An engine stop is the session's to report, as a value: kept off stderr.)
    nettai_frontend::session::quiet_engine_panics();
    errors::catch_library_logs();
    let tour = std::env::var_os("NETTAI_TOUR").map(std::path::PathBuf::from);
    // (The clock needs the renderer that tells when it draws: femtovg.)
    let mut backend = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("femtovg".into());
    if tour.is_some() {
        // (The screenshot pass's window doesn't take the keyboard.)
        backend = backend.with_winit_window_attributes_hook(|a| a.with_active(false));
    }
    backend.select()?;
    let ui = AppWindow::new()?;
    let app = Rc::new(RefCell::new(App::new(&ui, tour.is_some())));
    APP.with(|a| a.set(app.clone())).unwrap_or_else(|_| unreachable!("the app starts once"));
    wire(&ui, &app);
    app.borrow_mut().start();

    // The clock: before each frame is drawn, the app's frame; after it, the next frame asked for.
    let (weak, state) = (ui.as_weak(), app.clone());
    ui.window()
        .set_rendering_notifier(move |s, api| match s {
            // The window's OpenGL: the picture's texture is made in it.
            #[cfg(not(target_arch = "wasm32"))]
            RenderingState::RenderingSetup => {
                if let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api
                    && std::env::var_os("NETTAI_NO_TEXTURE").is_none()
                {
                    state.borrow_mut().shell.gl = Some(gl::GlPicture::new(get_proc_address));
                }
            }
            RenderingState::RenderingTeardown => state.borrow_mut().shell.gl = None,
            // (The app is busy only while one of the system's dialogs is up, which a screen opened and which
            // runs the window's loop itself: those frames are drawn as the last was.)
            RenderingState::BeforeRendering => {
                if let Ok(mut app) = state.try_borrow_mut() {
                    for input in WAITING.with(|w| std::mem::take(&mut *w.borrow_mut())) {
                        app.input(input);
                    }
                    app.frame(Instant::now());
                }
            }
            RenderingState::AfterRendering => {
                if let Ok(mut app) = state.try_borrow_mut() {
                    app.drawn();
                }
                if let Some(ui) = weak.upgrade() {
                    ui.window().request_redraw();
                }
            }
            _ => {}
        })
        .map_err(|e| slint::PlatformError::Other(format!("the window can't tell the app when it draws: {e:?}")))?;

    // The keys, before Slint's focus; what says the pointer was used.
    let state = app.clone();
    ui.window().on_winit_window_event(move |_, event| {
        use winit::event::WindowEvent;
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                // (The app is busy only inside a call of its own that reached the window: the key is Slint's.)
                let Ok(mut app) = state.try_borrow_mut() else { return EventResult::Propagate };
                if app.key(event) { EventResult::PreventDefault } else { EventResult::Propagate }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let held = modifiers.state();
                if let Ok(mut app) = state.try_borrow_mut() {
                    app.modifiers(held.shift_key(), held.control_key() || held.alt_key() || held.super_key());
                }
                EventResult::Propagate
            }
            WindowEvent::Ime(ime) => {
                let composing = match ime {
                    winit::event::Ime::Preedit(text, _) => Some(!text.is_empty()),
                    winit::event::Ime::Commit(_) | winit::event::Ime::Disabled => Some(false),
                    winit::event::Ime::Enabled => None,
                };
                if let (Some(composing), Ok(mut app)) = (composing, state.try_borrow_mut()) {
                    app.composing(composing);
                }
                EventResult::Propagate
            }
            WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } | WindowEvent::Touch(_) => {
                let taken = state.try_borrow_mut().map_or(true, |mut app| app.pointer());
                if taken { EventResult::Propagate } else { EventResult::PreventDefault }
            }
            // (After Slint has taken the new size: the layout's scale is set from it.)
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                let state = state.clone();
                slint::Timer::single_shot(Duration::ZERO, move || {
                    if let Ok(mut app) = state.try_borrow_mut() {
                        app.resized();
                    }
                });
                EventResult::Propagate
            }
            WindowEvent::Focused(false) => {
                if let Ok(mut app) = state.try_borrow_mut() {
                    app.unfocused();
                }
                EventResult::Propagate
            }
            // (A file dropped on the window is the screen's: a save, for a build of it.)
            WindowEvent::DroppedFile(path) => {
                let (path, state) = (path.clone(), state.clone());
                // (After this event: the screen may open one of the system's windows.)
                slint::Timer::single_shot(Duration::ZERO, move || state.borrow_mut().dropped(&path));
                EventResult::Propagate
            }
            _ => EventResult::Propagate,
        }
    });

    if std::env::var_os("NETTAI_KEY_PROBE").is_some() {
        key_probe();
    }
    if let Some(dir) = tour {
        tour::start(&ui, &app, dir);
    }
    ui.run()?;
    // (The screenshot pass found something cut or moved.)
    if tour::failed() {
        std::process::exit(1);
    }
    Ok(())
}

/// `NETTAI_KEY_PROBE`: the right arrow pressed every 300 to 400 ms (at no particular point between two frames,
/// through the event loop as a key's event comes) and let go 100 ms later.
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

/// What the UI tells the app: where its controls are (the focus's registry), and what was clicked, stepped and
/// typed.
fn wire(ui: &AppWindow, app: &Rc<RefCell<App>>) {
    let nav = ui.global::<nettai_ui::Nav>();
    let registry = app.borrow().shell.nav.registry();
    let r = registry.clone();
    nav.on_register(move |id, index, kind| Registry::register(&r, &id, index, kind));
    let r = registry.clone();
    nav.on_place(move |id, index, x, y, w, h, enabled| r.borrow_mut().place(&id, index, Rect { x, y, w, h }, enabled));
    let r = registry.clone();
    nav.on_page(move |id, x, y, w, h, _, _| r.borrow_mut().page(&id, Rect { x, y, w, h }));
    let r = registry.clone();
    nav.on_list(move |id, rows, pitch, row_height, scroll, row_width| r.borrow_mut().list(&id, rows, pitch, row_height, scroll, row_width));
    nav.on_scrolled(|| {});

    // What happened, to the app now, or at the next frame if it is in the middle of something.
    fn tell(app: &Rc<RefCell<App>>, input: NavInput) {
        match app.try_borrow_mut() {
            Ok(mut app) => app.input(input),
            Err(_) => WAITING.with(|w| w.borrow_mut().push(input)),
        }
    }
    let a = app.clone();
    nav.on_activate(move |id, index| tell(&a, NavInput::Activate(id.into(), index)));
    let a = app.clone();
    nav.on_step(move |id, index, by| tell(&a, NavInput::Step(id.into(), index, by)));
    let a = app.clone();
    nav.on_choose(move |id, index, choice| tell(&a, NavInput::Choose(id.into(), index, choice)));
    let a = app.clone();
    nav.on_point(move |id, index| tell(&a, NavInput::Point(id.into(), index)));
    let a = app.clone();
    nav.on_edit_began(move |_, _, text| tell(&a, NavInput::EditBegan(text.into())));
    let a = app.clone();
    nav.on_edited(move |_, _, text| tell(&a, NavInput::Edited(text.into())));
    let a = app.clone();
    nav.on_edit_done(move |_, _, text| tell(&a, NavInput::EditDone(text.into())));
    let a = app.clone();
    ui.global::<nettai_ui::Shell>().on_touch(move |bit, down| tell(&a, NavInput::Touch(bit, down)));
}
