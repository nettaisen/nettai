//! The Android backend: a gamepad's key and motion events, which the app's activity hands over in Kotlin
//! (`android/nettai/gamepad/NettaiGamepads.kt`) before anything else hears them.
//!
//! Android has no way to read a gamepad's state: a pad's buttons and sticks come as the activity's key and motion
//! events. The app's activity is a GameActivity, whose Java side sees input before native code; its
//! `dispatchKeyEvent` and `dispatchGenericMotionEvent` give every event of a gamepad's or a joystick's source to
//! `NettaiGamepads.take`, which hands it here through the native methods below ([`key`], [`motion`]) and keeps it from
//! the rest of the app (whose window, winit's, would make touches of a stick's motion). Those calls come on Java's
//! main thread, so each waits in a queue till [`crate::next_event`], on the app's thread, takes them into each pad's
//! state; the shared differ ([`crate::pads`]) hands out what changed. Which pads are attached comes from the system's
//! list of input devices (`InputDevice.getDeviceIds`, through JNI: ndk-context's VM, which android-activity sets),
//! looked at when the queue is empty, once a second at most: a gamepad on it is connected, and one that was on it
//! and is gone is disconnected. A pad heard from before the list says so is connected at once.

use crate::android_map::{axis, is_gamepad, key as key_of, keyed, moved};
use crate::pads::Pads;
use crate::Event;
use jni::objects::{JClass, JIntArray, JValue};
use jni::sys::{jboolean, jfloat, jint};
use jni::{EnvUnowned, jni_sig, jni_str};
use std::cell::RefCell;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How often the system's list of devices is looked at.
const LISTED_EVERY: Duration = Duration::from_secs(1);

/// What the activity handed over, not taken into the pads yet.
enum Input {
    Key { device: i32, code: u32, down: bool },
    /// The axes a motion event says, by `AXIS_*` number.
    Motion { device: i32, axes: [(u32, f32); 10] },
}

/// The input handed over, from Java's main thread.
static HANDED: Mutex<Vec<Input>> = Mutex::new(Vec::new());

struct Backend {
    /// The pads, by device id.
    pads: Pads<i32>,
    /// When the device list was last looked at, and the gamepads it had.
    listed: Option<Instant>,
    attached: Vec<i32>,
}

thread_local! {
    /// None until `init`, so nothing is handed out before it, as with the other backends.
    static BACKEND: RefCell<Option<Backend>> = const { RefCell::new(None) };
}

pub fn init(_app_name: &str) {
    BACKEND.with(|b| *b.borrow_mut() = Some(Backend { pads: Pads::default(), listed: None, attached: Vec::new() }));
}

pub fn next_event() -> Option<Event> {
    BACKEND.with(|b| {
        let mut b = b.borrow_mut();
        let b = b.as_mut()?;
        let handed = std::mem::take(&mut *HANDED.lock().unwrap_or_else(|e| e.into_inner()));
        for input in handed {
            match input {
                Input::Key { device, code, down } => {
                    if let Some(key) = key_of(code) {
                        let now = keyed(b.pads.state(&device), key, down);
                        b.pads.update(&device, now);
                    }
                }
                Input::Motion { device, axes } => {
                    let value = |a: u32| axes.iter().find(|(n, _)| *n == a).map_or(0.0, |(_, v)| *v);
                    let now = moved(b.pads.state(&device), value);
                    b.pads.update(&device, now);
                }
            }
        }
        if let Some(event) = b.pads.next() {
            return Some(event);
        }
        if b.listed.is_none_or(|at| at.elapsed() >= LISTED_EVERY) {
            b.listed = Some(Instant::now());
            match gamepads() {
                Ok(attached) => {
                    for gone in b.attached.iter().filter(|d| !attached.contains(d)) {
                        b.pads.disconnect(gone);
                    }
                    for device in &attached {
                        b.pads.connect(device);
                    }
                    b.attached = attached;
                }
                Err(why) => log::warn!("gamepad: listing the input devices: {why}"),
            }
        }
        b.pads.next()
    })
}

/// `NettaiGamepads.key`: a gamepad's key went down or up. Whether it is one of a gamepad's keys, which the activity
/// then keeps from the rest of the app (any other key goes on, as a keyboard's).
#[unsafe(no_mangle)]
extern "system" fn Java_nettai_gamepad_NettaiGamepads_key<'local>(_env: EnvUnowned<'local>, _class: JClass<'local>, device: jint, code: jint, down: jboolean) -> jboolean {
    let Some(_) = key_of(code as u32) else { return false };
    HANDED.lock().unwrap_or_else(|e| e.into_inner()).push(Input::Key { device, code: code as u32, down });
    true
}

/// `NettaiGamepads.motion`: a gamepad's or a joystick's axes as they are now (the sticks, the hat, both pairs of
/// triggers).
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
extern "system" fn Java_nettai_gamepad_NettaiGamepads_motion<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    device: jint,
    x: jfloat,
    y: jfloat,
    z: jfloat,
    rz: jfloat,
    hat_x: jfloat,
    hat_y: jfloat,
    ltrigger: jfloat,
    rtrigger: jfloat,
    brake: jfloat,
    gas: jfloat,
) {
    let axes = [
        (axis::X, x),
        (axis::Y, y),
        (axis::Z, z),
        (axis::RZ, rz),
        (axis::HAT_X, hat_x),
        (axis::HAT_Y, hat_y),
        (axis::LTRIGGER, ltrigger),
        (axis::RTRIGGER, rtrigger),
        (axis::BRAKE, brake),
        (axis::GAS, gas),
    ];
    HANDED.lock().unwrap_or_else(|e| e.into_inner()).push(Input::Motion { device, axes });
}

/// The ids of the gamepads attached: the system's input devices whose sources are a gamepad's or a joystick's.
fn gamepads() -> Result<Vec<i32>, String> {
    let context = ndk_context::android_context();
    // SAFETY: the process's VM, which android-activity hands ndk-context as the app starts.
    let vm = unsafe { jni::JavaVM::from_raw(context.vm().cast()) };
    vm.attach_current_thread(|env| -> jni::errors::Result<Vec<i32>> {
        let class = env.find_class(jni_str!("android/view/InputDevice"))?;
        let ids = env.call_static_method(&class, jni_str!("getDeviceIds"), jni_sig!("()[I"), &[])?.l()?;
        let ids = env.cast_local::<JIntArray>(ids)?;
        let mut all = vec![0; ids.len(env)?];
        ids.get_region(env, 0, &mut all)?;
        let mut pads = Vec::new();
        for id in all {
            let device = env.call_static_method(&class, jni_str!("getDevice"), jni_sig!("(I)Landroid/view/InputDevice;"), &[JValue::Int(id)])?.l()?;
            if device.is_null() {
                continue;
            }
            let sources = env.call_method(&device, jni_str!("getSources"), jni_sig!("()I"), &[])?.i()?;
            if is_gamepad(sources as u32) {
                pads.push(id);
            }
        }
        Ok(pads)
    })
    .map_err(|e: jni::errors::Error| e.to_string())
}
