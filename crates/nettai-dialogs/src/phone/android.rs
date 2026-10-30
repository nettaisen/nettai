//! Android's pickers, save dialog and clipboard: the system's own, started by `nettai.dialogs.NettaiFiles`
//! (crates/nettai-dialogs/android, which the APK's build compiles in), whose answers come through the native methods
//! registered here, on Java's threads.
//!
//! Files picked are copied into their lot through the activity's content resolver (`NettaiFiles.copy`); of a folder
//! picked, its files are listed as deep as the deepest wanted path, and those wanted are copied the same way, each
//! lot on a thread of its own. What is handed over goes to the save dialog (`ACTION_CREATE_DOCUMENT`): the player
//! says where, and the file is copied there. A picker filters by MIME type, which a save's extension has none of, so
//! it offers any file.

use super::{Answer, answer, lot};
use nettai_android::java;
use crate::Filter;
use crate::wanted::{depth, is_wanted};
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jlong;
use jni::{Env, EnvUnowned, NativeMethod, jni_sig, jni_str};
use std::path::PathBuf;
use std::sync::Mutex;

/// The folders asked for: each request's wanted paths, until its listing comes.
static FOLDERS: Mutex<Vec<(u64, Vec<String>)>> = Mutex::new(Vec::new());

/// Nothing of a request is held here once it is heard.
pub fn answered(_request: u64) {}

pub fn open_files(request: u64, filter: Filter) {
    let _ = filter;
    let started = java(|env, activity| {
        let class = helper(env, activity)?;
        env.call_static_method(&class, jni_str!("openFiles"), jni_sig!("(Landroid/app/Activity;J)V"), &[JValue::Object(activity), JValue::Long(request as jlong)])?;
        Ok(())
    });
    if let Err(why) = started {
        log::warn!("opening the picker: {why}");
        answer(request, Answer::Files(Vec::new()));
    }
}

pub fn open_folder(request: u64, wanted: Vec<String>) {
    let deep = depth(&wanted) as i32;
    FOLDERS.lock().unwrap_or_else(|e| e.into_inner()).push((request, wanted));
    let started = java(|env, activity| {
        let class = helper(env, activity)?;
        env.call_static_method(&class, jni_str!("openTree"), jni_sig!("(Landroid/app/Activity;JI)V"), &[JValue::Object(activity), JValue::Long(request as jlong), JValue::Int(deep)])?;
        Ok(())
    });
    if let Err(why) = started {
        log::warn!("opening the folder picker: {why}");
        FOLDERS.lock().unwrap_or_else(|e| e.into_inner()).retain(|(r, _)| *r != request);
        answer(request, Answer::Folder(None));
    }
}

/// The file at `path` to the save dialog, under its own name; it is removed once copied (or the dialog closed).
pub fn hand_over(path: PathBuf) {
    let name = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let started = java(|env, activity| {
        let class = helper(env, activity)?;
        let (source, name) = (env.new_string(path.to_string_lossy())?, env.new_string(&name)?);
        env.call_static_method(
            &class,
            jni_str!("save"),
            jni_sig!("(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)V"),
            &[JValue::Object(activity), JValue::Object(&source), JValue::Object(&name)],
        )?;
        Ok(())
    });
    if let Err(why) = started {
        log::warn!("handing over {}: {why}", path.display());
    }
}

pub fn copy(text: &str) {
    let copied = java(|env, activity| {
        let class = helper(env, activity)?;
        let text = env.new_string(text)?;
        env.call_static_method(&class, jni_str!("copyText"), jni_sig!("(Landroid/app/Activity;Ljava/lang/String;)V"), &[JValue::Object(activity), JValue::Object(&text)])?;
        Ok(())
    });
    if let Err(why) = copied {
        log::warn!("copying to the clipboard: {why}");
    }
}

/// `NettaiFiles`, by the activity's class loader (a native thread's `FindClass` sees only the system's classes),
/// its native methods registered (the library is the activity's, loaded natively, which Java's lookup by name
/// doesn't search).
fn helper<'local>(env: &mut Env<'local>, activity: &JObject) -> jni::errors::Result<JClass<'local>> {
    let loader = env.call_method(activity, jni_str!("getClassLoader"), jni_sig!("()Ljava/lang/ClassLoader;"), &[])?.l()?;
    let name = env.new_string("nettai.dialogs.NettaiFiles")?;
    let class = env.call_method(&loader, jni_str!("loadClass"), jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"), &[JValue::Object(&name)])?.l()?;
    let class = env.cast_local::<JClass>(class)?;
    // SAFETY: each function's parameters are what its signature says: (JNIEnv*, jclass, jlong, jstring).
    unsafe {
        env.register_native_methods(
            &class,
            &[
                NativeMethod::from_raw_parts(jni_str!("picked"), jni_str!("(JLjava/lang/String;)V"), picked as *mut std::ffi::c_void),
                NativeMethod::from_raw_parts(jni_str!("listed"), jni_str!("(JLjava/lang/String;)V"), listed as *mut std::ffi::c_void),
            ],
        )?;
    }
    Ok(class)
}

/// `NettaiFiles.picked`: the files picked (none: none were).
extern "system" fn picked<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, request: jlong, listing: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let listing = if listing.is_null() { None } else { Some(listing.try_to_string(env)?) };
        read_in(request as u64, listing.as_deref().map(lines).unwrap_or_default(), false);
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

/// `NettaiFiles.listed`: the files of the folder picked (none: none was).
extern "system" fn listed<'local>(mut env: EnvUnowned<'local>, _class: JClass<'local>, request: jlong, listing: JString<'local>) {
    env.with_env(|env| -> jni::errors::Result<()> {
        let request = request as u64;
        let wanted = {
            let mut folders = FOLDERS.lock().unwrap_or_else(|e| e.into_inner());
            let at = folders.iter().position(|(r, _)| *r == request);
            at.map(|at| folders.remove(at).1)
        };
        match (wanted, listing.is_null()) {
            (Some(wanted), false) => {
                let files = lines(&listing.try_to_string(env)?).into_iter().filter(|(path, _)| is_wanted(path, &wanted)).collect();
                read_in(request, files, true);
            }
            _ => answer(request, Answer::Folder(None)),
        }
        Ok(())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

/// A listing's lines: each a path (or name) and its document's URI.
fn lines(listing: &str) -> Vec<(String, String)> {
    listing.lines().filter_map(|l| l.split_once('\t')).map(|(path, uri)| (path.to_string(), uri.to_string())).collect()
}

/// `files` (paths in the lot, and their documents) copied into request `request`'s lot on a thread of its own, and
/// answered: the files, or the folder they make.
fn read_in(request: u64, files: Vec<(String, String)>, folder: bool) {
    let reading = std::thread::Builder::new().name("nettai-picked".into()).spawn(move || {
        let made = (|| -> Result<(PathBuf, Vec<PathBuf>), String> {
            let dir = lot(request)?;
            let mut copied = Vec::new();
            for (path, uri) in files {
                // (A path of names the listing made: no `..`, no root.)
                if path.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
                    continue;
                }
                let to = dir.join(&path);
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("making {}: {e}", parent.display()))?;
                }
                java(|env, activity| {
                    let class = helper(env, activity)?;
                    let (uri, to) = (env.new_string(&uri)?, env.new_string(to.to_string_lossy())?);
                    env.call_static_method(
                        &class,
                        jni_str!("copy"),
                        jni_sig!("(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;)V"),
                        &[JValue::Object(activity), JValue::Object(&uri), JValue::Object(&to)],
                    )?;
                    Ok(())
                })
                .map_err(|why| format!("copying {path}: {why}"))?;
                copied.push(to);
            }
            Ok((dir, copied))
        })();
        let made = made.map_err(|why| log::warn!("reading in what was picked: {why}")).ok();
        answer(request, if folder { Answer::Folder(made.map(|(dir, _)| dir)) } else { Answer::Files(made.map(|(_, files)| files).unwrap_or_default()) });
    });
    if let Err(why) = reading {
        log::warn!("reading in what was picked: no thread: {why}");
        answer(request, if folder { Answer::Folder(None) } else { Answer::Files(Vec::new()) });
    }
}
