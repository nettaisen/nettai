//! The updater, a module each system, chosen here and nowhere else:
//!
//! - [`desktop`] (Windows, macOS, Linux): Velopack's, for an app installed from one of its packages;
//! - [`android`]: the newest release's APK, installed by Android's `PackageInstaller`
//!   (`android/nettai/updater/NettaiUpdater.kt`, which the APK's build compiles in);
//! - [`none`] (a browser, iOS): no updater: nothing is asked.
//!
//! Each has the same items: `startup` (first thing in the app's `main`), `check` (which starts asking, and tells what
//! it found as [`crate::Told`]; false where nothing is asked), the update found (`Found`, with its `version`),
//! `download` (which tells how far, and how it ended) and `install` (which tells what kept the update from being
//! installed).

#[cfg(desktop)]
pub(crate) mod desktop;
#[cfg(android)]
#[allow(unsafe_code)]
pub(crate) mod android;
#[cfg(any(web, ios))]
pub(crate) mod none;

#[cfg(desktop)]
pub(crate) use desktop as system;
#[cfg(android)]
pub(crate) use android as system;
#[cfg(any(web, ios))]
pub(crate) use none as system;
