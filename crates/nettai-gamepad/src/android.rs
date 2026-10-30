//! The Android backend: the activity's gamepad events, which the host hands over ([`input`]).
//!
//! Android has no way to read a gamepad's state. A pad's buttons and sticks come as the activity's key and motion
//! events, and a NativeActivity's events are read by the app's own loop (android-activity's input events), not by a
//! library. So the host passes each one to [`input`] first, which keeps each pad's state from those of a gamepad
//! (their source a gamepad's or a joystick's) and says whether it took it; [`crate::next_event`] hands out what
//! changed, through the shared differ ([`crate::pads`]). Which pads are attached comes from the system's list of
//! input devices (`InputDevice.getDeviceIds`, through JNI: ndk-context's VM, which android-activity sets), looked at
//! when the queue is empty, once a second at most: a gamepad on it is connected, one gone from it disconnected. A pad
//! heard from before the list says it is connected at once.
//!
//! [`input`], [`crate::init`] and [`crate::next_event`] run on one thread, the app's loop's.

use crate::android_map::{is_gamepad, key, keyed, moved};
use crate::pads::Pads;
use crate::Event;
use android_activity::input::{InputEvent, KeyAction, MotionAction};
use jni::objects::{JIntArray, JValue};
use jni::{jni_sig, jni_str};
use std::cell::RefCell;
use std::time::{Duration, Instant};

/// How often the system's list of devices is looked at.
const LISTED_EVERY: Duration = Duration::from_secs(1);

struct Backend {
    /// The pads, by device id.
    pads: Pads<i32>,
    /// When the device list was last looked at.
    listed: Option<Instant>,
}

thread_local! {
    /// None until `init`, so nothing is taken or handed out before it, as with the other backends.
    static BACKEND: RefCell<Option<Backend>> = const { RefCell::new(None) };
}

pub fn init(_app_name: &str) {
    BACKEND.with(|b| *b.borrow_mut() = Some(Backend { pads: Pads::default(), listed: None }));
}

/// An input event of the activity's, handed over before the host handles it: true if it was a gamepad's, which this
/// took (the host need do nothing more with it); false if it is the host's.
pub fn input(event: &InputEvent) -> bool {
    BACKEND.with(|b| {
        let mut b = b.borrow_mut();
        let Some(b) = b.as_mut() else { return false };
        match event {
            InputEvent::KeyEvent(k) if is_gamepad(k.source().into()) => {
                let Some(key) = key(k.key_code().into()) else { return false };
                let down = match k.action() {
                    KeyAction::Down => true,
                    KeyAction::Up => false,
                    _ => return true,
                };
                let device = k.device_id();
                let now = keyed(b.pads.state(&device), key, down);
                b.pads.update(&device, now);
                true
            }
            InputEvent::MotionEvent(m) if is_gamepad(m.source().into()) && m.action() == MotionAction::Move => {
                let device = m.device_id();
                let pointer = m.pointer_at_index(0);
                let now = moved(b.pads.state(&device), |axis| pointer.axis_value(axis.into()));
                b.pads.update(&device, now);
                true
            }
            _ => false,
        }
    })
}

pub fn next_event() -> Option<Event> {
    BACKEND.with(|b| {
        let mut b = b.borrow_mut();
        let b = b.as_mut()?;
        if let Some(event) = b.pads.next() {
            return Some(event);
        }
        if b.listed.is_none_or(|at| at.elapsed() >= LISTED_EVERY) {
            b.listed = Some(Instant::now());
            match gamepads() {
                Ok(attached) => {
                    for gone in b.pads.keys().into_iter().filter(|d| !attached.contains(d)) {
                        b.pads.disconnect(&gone);
                    }
                    for device in attached {
                        b.pads.connect(&device);
                    }
                }
                Err(why) => log::warn!("gamepad: listing the input devices: {why}"),
            }
        }
        b.pads.next()
    })
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
