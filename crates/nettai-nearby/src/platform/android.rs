//! Android's discovery: the system's NsdManager, from Kotlin (`android/nettai/nearby/NettaiNearby.kt`, which the
//! APK's build compiles in), started through JNI; what it finds, resolves and loses comes back through the native
//! methods registered here, on Java's threads, to the browse's channel. NsdManager's own service does the multicast,
//! so the app needs no permission beyond the internet's. Browsing alone: a phone doesn't offer.

use crate::Seen;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jint;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use nettai_android::java;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::sync::mpsc::Sender;

pub const OFFERS: bool = false;
pub const SEEKS: bool = true;

/// Never made: a phone doesn't offer.
pub struct Advertised;

pub fn advertise(_name: &str, _port: u16) -> Result<Advertised, String> {
    Err("a phone doesn't offer".into())
}

/// Where what is heard goes, while browsing.
static SEEN: Mutex<Option<Sender<Seen>>> = Mutex::new(None);

fn tell(seen: Seen) {
    if let Some(tell) = SEEN.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = tell.send(seen);
    }
}

/// Browsing, until dropped.
pub struct Browsing;

impl Drop for Browsing {
    fn drop(&mut self) {
        *SEEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let stopped = java(|env, activity| {
            let class = helper(env, activity)?;
            env.call_static_method(&class, jni_str!("stop"), jni_sig!("()V"), &[])?;
            Ok(())
        });
        if let Err(why) = stopped {
            log::warn!("nearby: stopping the browse: {why}");
        }
    }
}

/// Browse for devices offering; what is found and lost is told to `seen`.
pub fn browse(seen: Sender<Seen>) -> Result<Browsing, String> {
    *SEEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(seen);
    let started = java(|env, activity| {
        let class = helper(env, activity)?;
        let service = env.new_string(crate::SERVICE)?;
        env.call_static_method(&class, jni_str!("start"), jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)V"), &[JValue::Object(activity), JValue::Object(&service)])?;
        Ok(())
    });
    match started {
        Ok(()) => Ok(Browsing),
        Err(why) => {
            *SEEN.lock().unwrap_or_else(|e| e.into_inner()) = None;
            Err(why)
        }
    }
}

/// The device's name as its player set it (Settings, About phone), else its model's.
pub fn device_name() -> String {
    let name = java(|env, activity| {
        let class = helper(env, activity)?;
        let name = env.call_static_method(&class, jni_str!("deviceName"), jni_sig!("(Landroid/app/Activity;)Ljava/lang/String;"), &[JValue::Object(activity)])?.l()?;
        env.cast_local::<JString>(name)?.try_to_string(env)
    });
    match name {
        Ok(name) if !name.is_empty() => name,
        Ok(_) => "Android".into(),
        Err(why) => {
            log::warn!("nearby: the device's name: {why}");
            "Android".into()
        }
    }
}

/// `NettaiNearby`, by the activity's class loader (a native thread's `FindClass` sees only the system's classes),
/// its native methods registered (the library is the activity's, loaded natively, which Java's lookup by name
/// doesn't search).
fn helper<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
    let name = env.new_string("nettai.nearby.NettaiNearby")?;
    let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
    let class = env.cast_local::<JClass>(class)?;
    // SAFETY: each function's parameters are what its signature says: (JNIEnv*, jclass, then the Java ones).
    unsafe {
        env.register_native_methods(
            &class,
            &[
                NativeMethod::from_raw_parts(
                    jni_str!("found"),
                    jni_str!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;I)V"),
                    found as *mut std::ffi::c_void,
                ),
                NativeMethod::from_raw_parts(jni_str!("lost"), jni_str!("(Ljava/lang/String;)V"), lost as *mut std::ffi::c_void),
                NativeMethod::from_raw_parts(jni_str!("failed"), jni_str!("(Ljava/lang/String;)V"), failed as *mut std::ffi::c_void),
            ],
        )?;
    }
    Ok(class)
}

/// `NettaiNearby.found`: a service resolved: its key (the instance's name), the device's name (the TXT record's), and
/// an address of its host (said as text) and the port.
extern "system" fn found<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, key: JString<'local>, name: JString<'local>, host: JString<'local>, port: jint) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let (key, name, host) = (key.try_to_string(env)?, name.try_to_string(env)?, host.try_to_string(env)?);
        let addrs: Vec<SocketAddr> = host.parse::<IpAddr>().ok().map(|ip| SocketAddr::new(ip, port as u16)).into_iter().collect();
        let name = if name.is_empty() { key.clone() } else { name };
        tell(Seen::Found { key, name, addrs });
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

/// `NettaiNearby.lost`: a service found is gone, by its key.
extern "system" fn lost<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, key: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        tell(Seen::Lost { key: key.try_to_string(env)? });
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

/// `NettaiNearby.failed`: the browse couldn't start, or stopped: why.
extern "system" fn failed<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, why: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        tell(Seen::Failed { why: why.try_to_string(env)? });
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}
