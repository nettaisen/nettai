//! nettai: the app (docs/app.md). A main menu, Play, Practice, Builds, Replays and Settings around a battle
//! played through nettai-frontend, in a Slint window whose UI is the nettai-ui crate.
//!
//! The window's rendering is the clock: before each frame is drawn the app runs (`App::frame`: the battle's
//! ticks due, its picture presented for that frame, the sound), and after it asks for the next frame, so it
//! runs at the display's rate. The keys come from the window's events before Slint sees them: the battle's by
//! where they are on the keyboard, timestamped; the menus' as the same few actions a gamepad's buttons are.
//!
//! `NETTAI_PLAY_STATS` prints what the battle's picture costs and how late it is, every two seconds;
//! `NETTAI_KEY_PROBE` presses the right arrow now and then to measure it; `NETTAI_WINDOW_PROBE` minimizes or
//! hides the window for a while; `NETTAI_TOUR=<folder>` walks every
//! screen and writes each to a PNG, and with `NETTAI_REEL=<out.mp4>` makes a showreel video of a scripted walk
//! (`tour`: a build with the `tour` feature's alone).

// A release build on Windows opens no console window beside its own (what it prints goes nowhere; the log is the
// data folder's nettai.log). A development build keeps one, for what it prints.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod awake;
mod backgrounds;
mod battle;
mod bnlc;
mod clock;
mod builds;
mod errors;
mod games;
mod gl;
mod input;
mod lang;
mod look;
mod music;
mod nav;
mod packs;
mod paths;
mod quit;
mod replays;
mod screens;
mod settings;
mod shell;
mod sound;
mod stage;
// (The screenshot pass is the `tour` feature's: a player's build has none of it. The tests compile it all the
// same, so that it keeps compiling.)
#[cfg(any(test, feature = "tour"))]
#[cfg_attr(not(feature = "tour"), allow(dead_code, unused_imports))]
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
    // (The showreel: the pass's walk, made a video. It runs in the pass's folder.)
    let reel = std::env::var_os("NETTAI_REEL").map(std::path::PathBuf::from);
    if cfg!(not(feature = "tour")) && (tour.is_some() || reel.is_some()) {
        eprintln!("nettai: this build has no screenshot pass (NETTAI_TOUR, NETTAI_REEL): build it with `--features tour`");
        std::process::exit(2);
    }
    if reel.is_some() && tour.is_none() {
        eprintln!("nettai: the showreel (NETTAI_REEL) runs in the screenshot pass's folder: give it NETTAI_TOUR=<folder> too");
        std::process::exit(2);
    }
    let reeling = cfg!(feature = "tour") && reel.is_some();
    if reeling {
        // (The same backend as below, its clock the reel's.)
        #[cfg(feature = "tour")]
        tour::reel::platform()?;
    } else {
        // (The clock needs the renderer that tells when it draws: femtovg.)
        let mut backend = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("femtovg".into());
        if tour.is_some() {
            // (The screenshot pass's window doesn't take the keyboard.)
            backend = backend.with_winit_window_attributes_hook(|a| a.with_active(false));
        }
        backend.select()?;
    }
    let ui = AppWindow::new()?;
    let app = Rc::new(RefCell::new(App::new(&ui, tour.is_some())));
    APP.with(|a| a.set(app.clone())).unwrap_or_else(|_| unreachable!("the app starts once"));
    wire(&ui, &app);
    app.borrow_mut().start();
    // (After the app's own start: the gamepads' library has taken the terminate requests by now.)
    quit::watch();

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
            // (In the showreel, only the frames it captures run the app's: the window's own draws show them.)
            RenderingState::BeforeRendering if frame_due() => {
                if let Ok(mut app) = state.try_borrow_mut() {
                    for input in WAITING.with(|w| std::mem::take(&mut *w.borrow_mut())) {
                        app.input(input);
                    }
                    app.frame(clock::now());
                }
            }
            RenderingState::AfterRendering => {
                if let Ok(mut app) = state.try_borrow_mut() {
                    app.drawn();
                }
                if let Some(ui) = weak.upgrade()
                    && !reeling
                {
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
            // A finger: the on-screen pad's if it is on one of its buttons (each finger by itself, where Slint
            // follows one pointer), else Slint's, as a pointer.
            WindowEvent::Touch(touch) => {
                let Ok(mut app) = state.try_borrow_mut() else { return EventResult::Propagate };
                if app.touched(touch.id, touch.phase, touch.location.x, touch.location.y) {
                    EventResult::PreventDefault
                } else if app.pointer() {
                    EventResult::Propagate
                } else {
                    EventResult::PreventDefault
                }
            }
            WindowEvent::CursorMoved { .. } | WindowEvent::MouseInput { .. } | WindowEvent::MouseWheel { .. } => {
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
            // (`NETTAI_PLAY_STATS`: whether the system says the window is covered, for the clock's figures.)
            WindowEvent::Occluded(covered) => {
                if std::env::var_os("NETTAI_PLAY_STATS").is_some() {
                    eprintln!("window: the system says it is {}", if *covered { "covered" } else { "in view" });
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

    // The clock when the window isn't drawn (minimized, hidden, covered): a timer runs the app's frame while a
    // battle or a link must go on, and asks for a redraw so the window's frames take over again when it shows.
    // (`NETTAI_NO_TIMER_CLOCK` leaves it out, to see what the window does alone. The showreel runs every frame
    // itself.)
    let idle = slint::Timer::default();
    if std::env::var_os("NETTAI_NO_TIMER_CLOCK").is_none() && !reeling {
        let (weak, state) = (ui.as_weak(), app.clone());
        idle.start(slint::TimerMode::Repeated, Duration::from_millis(16), move || {
            let ran = state.try_borrow_mut().is_ok_and(|mut app| app.idle(Instant::now()));
            if ran && let Some(ui) = weak.upgrade() {
                ui.window().request_redraw();
            }
        });
    }
    if std::env::var_os("NETTAI_KEY_PROBE").is_some() {
        key_probe();
    }
    if let Ok(probe) = std::env::var("NETTAI_WINDOW_PROBE") {
        window_probe(&ui, &probe);
    }
    #[cfg(feature = "tour")]
    if let Some(dir) = tour {
        match reel {
            Some(out) => tour::reel::start(&ui, &app, dir, out),
            None => tour::start(&ui, &app, dir),
        }
    }
    ui.run()?;
    // (The screenshot pass found something cut or moved, or was ended before its walk was done.)
    #[cfg(feature = "tour")]
    tour::ended(quit::asked());
    #[cfg(feature = "tour")]
    if tour::failed() {
        std::process::exit(1);
    }
    Ok(())
}

/// Whether the window's rendering runs the app's frame: always, but in the showreel (`tour::reel`).
fn frame_due() -> bool {
    #[cfg(feature = "tour")]
    if !tour::reel::frame_due() {
        return false;
    }
    true
}

/// `NETTAI_WINDOW_PROBE=<minimize|hide>:<after>:<for>`: the window minimized or hidden `after` seconds from the
/// start, and brought back `for` seconds later, each said on the terminal: with `NETTAI_PLAY_STATS`, what the
/// battle's clock does meanwhile.
fn window_probe(ui: &AppWindow, probe: &str) {
    let mut parts = probe.split(':');
    let what = parts.next().unwrap_or_default().to_string();
    let mut seconds = || parts.next().and_then(|s| s.parse::<u64>().ok()).unwrap_or(5);
    let (after, long) = (seconds(), seconds());
    let set = move |ui: &AppWindow, away: bool| {
        eprintln!("window probe: {}", if away { format!("{what} now") } else { "back".to_string() });
        slint::winit_030::WinitWindowAccessor::with_winit_window(ui.window(), |w| match what.as_str() {
            "minimize" => w.set_minimized(away),
            _ => w.set_visible(!away),
        });
    };
    let (weak, away) = (ui.as_weak(), set.clone());
    slint::Timer::single_shot(Duration::from_secs(after), move || {
        if let Some(ui) = weak.upgrade() {
            away(&ui, true);
        }
    });
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_secs(after + long), move || {
        if let Some(ui) = weak.upgrade() {
            set(&ui, false);
        }
    });
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
    let a = app.clone();
    ui.global::<nettai_ui::Shell>().on_pad_key(move |bit, x, y, w, h| tell(&a, NavInput::PadKey(bit, Rect { x, y, w, h })));
}
