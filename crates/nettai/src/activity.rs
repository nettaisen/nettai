//! The app on Android (docs/app.md §7): the library the APK's activity loads (`libnettai.so`, this crate's library
//! built as one: packaging/android/package.sh; the manifest's `android.app.lib_name`; the activity is a
//! GameActivity: android/nettai/app/NettaiActivity.kt), whose [`android_main`] android-activity calls on a thread of
//! its own once the activity is made; the activity, kept for the app's calls into Java ([`java`]); the safe area it
//! hands over; and the app's window on it. android-activity puts what the app writes to stdout and stderr on logcat
//! (tagged RustStdoutStderr), and a panic of this thread too (RustPanic).
//!
//! Slint's selector offers Android its own android-activity backend alone, which reads the activity's input itself
//! and lets the app hear none of it; so the app sets Slint's winit backend itself, its event loop the activity's
//! (kept first: [`android_main`]), with femtovg on the activity's OpenGL ES.
//!
//! The activity has a window to draw on only from winit's `resumed` to its `suspended`: Android takes it away while
//! the app is in the background, and gives a new one when it is back. Slint's winit backend makes its windows as soon
//! as its loop runs, and keeps them. So the app's window waits for the activity's (made at the loop's first wait,
//! before the activity's window is up, it would find nothing to draw on, and Slint would drop it for good), and goes
//! with it: hidden at each suspend, which drops Slint's winit window with its OpenGL surface (Slint does that on a
//! hide when `SLINT_DESTROY_WINDOW_ON_HIDE` is set, which the activity sets before the library starts), and shown at
//! each resume, made anew on the new window.

use i_slint_backend_winit::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use i_slint_backend_winit::winit::platform::android::EventLoopBuilderExtAndroid;
use i_slint_backend_winit::winit::platform::android::activity::AndroidApp;
use i_slint_backend_winit::{CustomApplicationHandler, EventResult, SlintEvent};
use nettai_ui::AppWindow;
use slint::ComponentHandle;
use jni::EnvUnowned;
use jni::objects::JObject;
use jni::sys::jint;
use std::cell::OnceCell;
use std::sync::{Mutex, OnceLock};

/// The app's activity, as android-activity hands it over ([`android_main`]).
static ACTIVITY: OnceLock<AndroidApp> = OnceLock::new();

/// The safe area's insets in the window's pixels (left, top, right, bottom), as the activity last handed them over
/// ([`safe_area`]): none till it first has.
static SAFE_AREA: Mutex<Option<[i32; 4]>> = Mutex::new(None);

thread_local! {
    /// The app's window, once made.
    static WINDOW: OnceCell<slint::Weak<AppWindow>> = const { OnceCell::new() };
}

/// The app, under tokio as the desktop's `main` runs it (main.rs), on this thread, which runs the activity's event
/// loop (winit's, through Slint) till the activity is gone. First the activity is kept (the window's loop is made on
/// it, and the app's calls into Java go through it), and the certificate verifier given it.
#[unsafe(no_mangle)]
fn android_main(activity: AndroidApp) {
    let _ = ACTIVITY.set(activity);
    let verifier = java(|env, activity| {
        let context = env.new_local_ref(activity)?;
        rustls_platform_verifier::android::init_with_env(env, context)
    });
    if let Err(why) = verifier {
        eprintln!("nettai: setting up the certificate verifier: {why}");
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build() {
        Ok(runtime) => runtime,
        Err(why) => return eprintln!("nettai: starting tokio: {why}"),
    };
    if let Err(why) = runtime.block_on(async { tokio::task::block_in_place(crate::run) }) {
        eprintln!("nettai: the app stopped: {why}");
    }
}

/// The app's activity, as [`android_main`] kept it.
fn activity() -> Result<&'static AndroidApp, String> {
    ACTIVITY.get().ok_or_else(|| "the app's activity wasn't handed over (android_main)".to_string())
}

/// `call` run with Java's environment (this thread attached to the VM, for as long as it lives) and the activity (an
/// `android.app.Activity`, which is a `Context`). What Java threw, or what went wrong, is the error, said.
pub(crate) fn java<T>(call: impl FnOnce(&mut jni::Env, &JObject) -> jni::errors::Result<T>) -> Result<T, String> {
    let app = activity()?;
    // SAFETY: the VM is the process's, valid for its life.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: android-activity holds the activity as a global reference for the app's life.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        call(env, &activity)
    })
    .map_err(|e: jni::errors::Error| e.to_string())
}

/// Slint's winit backend on the activity's event loop.
pub(crate) fn select_backend() -> Result<(), slint::PlatformError> {
    let activity = activity().map_err(slint::PlatformError::Other)?.clone();
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
/// which the activity hands over (`NettaiActivity.safeArea`, below). The window is the whole screen, under the bars
/// (GameActivity's content rect is all of it). Android changes them without a word from winit (as the phone turns,
/// a little after the window's new size), so they are read at each frame; they are nothing till Android first says.
pub(crate) fn safe_area(ui: &AppWindow) {
    let Some([left, top, right, bottom]) = *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner()) else { return };
    // (In the UI's units as they are now: Slint's scale is the system's times the unit.)
    let factor = ui.window().scale_factor();
    let edge = |pixels: i32| pixels.max(0) as f32 / factor;
    ui.set_safe_left(edge(left));
    ui.set_safe_top(edge(top));
    ui.set_safe_right(edge(right));
    ui.set_safe_bottom(edge(bottom));
}

/// `NettaiActivity.safeArea`: the safe area's insets as Android says them now, which the activity hands over on its
/// main thread whenever they change.
#[unsafe(no_mangle)]
extern "system" fn Java_nettai_app_NettaiActivity_safeArea<'local>(_env: EnvUnowned<'local>, _activity: JObject<'local>, left: jint, top: jint, right: jint, bottom: jint) {
    *SAFE_AREA.lock().unwrap_or_else(|e| e.into_inner()) = Some([left, top, right, bottom]);
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
