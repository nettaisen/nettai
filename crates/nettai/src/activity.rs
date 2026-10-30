//! The app's window on Android (docs/app.md §7).
//!
//! Slint's selector offers Android its own android-activity backend alone, which reads the activity's input itself
//! and lets the app hear none of it; so the app sets Slint's winit backend itself, its event loop the activity's
//! (handed over first: `nettai_android::init`), with femtovg on the activity's OpenGL ES.
//!
//! The activity has a window to draw on only from winit's `resumed` to its `suspended`: Android takes it away while
//! the app is in the background, and gives a new one when it is back. Slint's winit backend makes its windows as soon
//! as its loop runs, and keeps them. So the app's window waits for the activity's (made at the loop's first wait,
//! before the activity's window is up, it would find nothing to draw on, and Slint would drop it for good), and goes
//! with it: hidden at each suspend, which drops Slint's winit window with its OpenGL surface (Slint does that on a
//! hide when `SLINT_DESTROY_WINDOW_ON_HIDE` is set, which the activity sets before the library starts:
//! crates/nettai-android-app), and shown at each resume, made anew on the new window.

use i_slint_backend_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use i_slint_backend_winit::winit::platform::android::EventLoopBuilderExtAndroid;
use i_slint_backend_winit::{CustomApplicationHandler, EventResult, SlintEvent};
use nettai_ui::AppWindow;
use slint::ComponentHandle;
use std::cell::OnceCell;

thread_local! {
    /// The app's window, once made.
    static WINDOW: OnceCell<slint::Weak<AppWindow>> = const { OnceCell::new() };
}

/// Slint's winit backend on the activity's event loop.
pub(crate) fn select_backend() -> Result<(), slint::PlatformError> {
    let activity = nettai_android::app().map_err(slint::PlatformError::Other)?.clone();
    let mut events = EventLoop::<SlintEvent>::with_user_event();
    events.with_android_app(activity);
    let backend = i_slint_backend_winit::Backend::builder()
        .with_renderer_name("femtovg")
        .with_event_loop_builder(events)
        .with_custom_application_handler(Box::new(Activity { window: false }))
        .build()?;
    slint::platform::set_platform(Box::new(backend)).map_err(|e| slint::PlatformError::Other(e.to_string()))
}

/// The app's window shown and its loop run, till the app quits (`slint::quit_event_loop`): the window hidden while
/// the app is in the background doesn't end it.
pub(crate) fn run(ui: &AppWindow) -> Result<(), slint::PlatformError> {
    WINDOW.with(|w| w.set(ui.as_weak())).unwrap_or_else(|_| unreachable!("the app's window is made once"));
    ui.show()?;
    slint::run_event_loop_until_quit()?;
    ui.hide()
}

/// The safe area's insets into the UI (`safe-left` and the rest, Slint's `safe-area-insets` on the other systems,
/// which Slint's winit backend leaves empty on Android): how far the system bars and cutouts reach into the window,
/// which the activity hands over (`nettai_android::safe_area`). The window is the whole screen, under the bars
/// (GameActivity's content rect is all of it). Android changes them without a word from winit (as the phone turns,
/// a little after the window's new size), so they are read at each frame; they are nothing till Android first says.
pub(crate) fn safe_area(ui: &AppWindow) {
    let Some([left, top, right, bottom]) = nettai_android::safe_area() else { return };
    // (In the UI's units as they are now: Slint's scale is the system's times the unit.)
    let factor = ui.window().scale_factor();
    let edge = |pixels: i32| pixels.max(0) as f32 / factor;
    ui.set_safe_left(edge(left));
    ui.set_safe_top(edge(top));
    ui.set_safe_right(edge(right));
    ui.set_safe_bottom(edge(bottom));
}

fn with_window(f: impl FnOnce(&AppWindow)) {
    if let Some(ui) = WINDOW.with(|w| w.get().and_then(|w| w.upgrade())) {
        f(&ui);
    }
}

struct Activity {
    /// Whether the activity has its window.
    window: bool,
}

impl CustomApplicationHandler for Activity {
    fn resumed(&mut self, _: &ActiveEventLoop) -> EventResult {
        self.window = true;
        // (Slint makes it on the activity's window once this returns.)
        with_window(|ui| {
            if !ui.window().is_visible()
                && let Err(e) = ui.show()
            {
                crate::errors::log("showing the window again", &e);
            }
        });
        EventResult::Propagate
    }

    fn suspended(&mut self, _: &ActiveEventLoop) -> EventResult {
        self.window = false;
        with_window(|ui| {
            if let Err(e) = ui.hide() {
                crate::errors::log("hiding the window in the background", &e);
            }
        });
        EventResult::Propagate
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) -> EventResult {
        if self.window {
            return EventResult::Propagate;
        }
        // (Slint's own would make the app's window now: only the wait for its timers.)
        if let Some(next) = slint::platform::duration_until_next_timer_update() {
            event_loop.set_control_flow(ControlFlow::wait_duration(next));
        }
        EventResult::PreventDefault
    }
}
