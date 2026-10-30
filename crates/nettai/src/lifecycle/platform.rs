//! The system's side of the app's life, one module chosen here: macOS's activity, iOS's idle timer and
//! notifications, Android's window kept on (through Kotlin), and nothing elsewhere (Windows, Linux, a browser).
//! Each has `Held` (what an [`crate::lifecycle::Awake`] holds, let go as it is dropped) and `watch` (what `resumed` starts);
//! what only one has is public from here as the module's own (Android's `came_back`).

#[cfg(target_os = "macos")]
pub(crate) mod macos;
#[cfg(target_os = "ios")]
pub(crate) mod ios;
#[cfg(target_os = "android")]
pub(crate) mod android;
#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
pub(crate) mod none;

#[cfg(target_os = "macos")]
pub(crate) use macos as system;
#[cfg(target_os = "ios")]
pub(crate) use ios as system;
#[cfg(target_os = "android")]
pub(crate) use android as system;
#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
pub(crate) use none as system;

#[cfg(target_os = "android")]
pub use android::came_back;
