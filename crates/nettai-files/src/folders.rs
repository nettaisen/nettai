//! The system's folders for the app: where apps keep their data, and where files go for a while. Each system's are
//! its own module's (`desktop`, `ios`, `android`, `web`), chosen here.

use std::path::PathBuf;

#[cfg(desktop)]
mod desktop;
#[cfg(ios)]
mod ios;
#[cfg(android)]
mod android;
#[cfg(web)]
mod web;

#[cfg(desktop)]
use desktop as system;
#[cfg(ios)]
use ios as system;
#[cfg(android)]
use android as system;
#[cfg(web)]
use web as system;

/// The folder the system gives apps for their data, the app's own folder to be made in it: Windows' `%APPDATA%`,
/// macOS's `~/Library/Application Support`, Linux's `$XDG_DATA_HOME` or `~/.local/share`; on iOS the app's own
/// `Library/Application Support` (in its container, which the player doesn't see); on Android the app's files
/// folder (`Context.getFilesDir`). None in a browser (the page keeps the app's files itself), and where the system
/// says none (no `HOME`; Android's activity not handed over yet, which is logged).
pub fn app_data() -> Option<PathBuf> {
    system::app_data()
}

/// A folder for files kept a while (those the player picked, read in; one made to hand over): on Android the app's
/// cache folder (`Context.getCacheDir`), which the system may empty while the app isn't running; elsewhere the
/// system's temporary folder. None in a browser, which has none, and where Android's activity isn't handed over yet
/// (logged).
pub fn temporary() -> Option<PathBuf> {
    system::temporary()
}
