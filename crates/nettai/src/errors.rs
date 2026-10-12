//! What can go wrong, as the player reads it (docs/design/app-ui.md §7), and the log.
//!
//! No string from a library's error is ever shown. A failure the player can see is one of [`UserError`]'s
//! kinds, each a sentence of the catalogs (the UI's `Errors`, shell.slint); what the code said goes to the log
//! file alone ([`log`]: `crate::paths::log`). A problem with one field is shown in that field's reserved line
//! ([`UserError::line`]); a failure that ends what the player was doing is a dialog (`Shell::fail`).

use nettai_ui::{AppWindow, ErrorKind, Errors};
use slint::ComponentHandle;
use std::io::Write;
use std::sync::Mutex;

/// A failure the player can see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserError {
    /// The signaling server can't be reached.
    ServerUnreachable,
    /// No match has that invite code.
    NoSuchMatch,
    /// The match already has two players.
    MatchFull,
    /// The two apps differ.
    VersionMismatch,
    /// A build breaks a rule: `reason` is the rule's own sentence, from the catalogs ("2 Giga chips, limit 1").
    BuildUnusable { reason: String },
    /// The link dropped for good.
    ConnectionLost,
    /// A replay is from another version.
    ReplayVersion,
    /// A save is another game's: `game` is the game it was to be of, as shown (EXE6).
    WrongGameSave { game: String },
    /// Anything else: the details are in the log.
    Other,
}

impl UserError {
    fn kind(&self) -> (ErrorKind, &str) {
        match self {
            UserError::ServerUnreachable => (ErrorKind::ServerUnreachable, ""),
            UserError::NoSuchMatch => (ErrorKind::NoSuchMatch, ""),
            UserError::MatchFull => (ErrorKind::MatchFull, ""),
            UserError::VersionMismatch => (ErrorKind::VersionMismatch, ""),
            UserError::BuildUnusable { reason } => (ErrorKind::BuildUnusable, reason),
            UserError::ConnectionLost => (ErrorKind::ConnectionLost, ""),
            UserError::ReplayVersion => (ErrorKind::ReplayVersion, ""),
            UserError::WrongGameSave { game } => (ErrorKind::WrongGameSave, game),
            UserError::Other => (ErrorKind::Other, ""),
        }
    }

    /// The whole of it on one line, in the language shown: for a field's reserved line, or a row's end.
    pub fn line(&self, ui: &AppWindow) -> String {
        let (kind, detail) = self.kind();
        ui.global::<Errors>().invoke_line(kind, detail.into()).into()
    }

    /// A dialog's title and message for it.
    pub fn dialog(&self, ui: &AppWindow) -> (String, String) {
        let (kind, detail) = self.kind();
        let errors = ui.global::<Errors>();
        (errors.invoke_title(kind, detail.into()).into(), errors.invoke_body(kind, detail.into()).into())
    }
}

/// The log file, opened at the first line written.
static LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Write a line to the log: what was being done (`doing`) and what the code said of it. This is where an
/// error's own words go; the player reads a [`UserError`].
pub fn log(doing: &str, said: &dyn std::fmt::Display) {
    let line = format!("{} {doing}: {said}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
    let Ok(mut file) = LOG.lock() else { return };
    if file.is_none() {
        let path = crate::paths::log();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        *file = std::fs::OpenOptions::new().create(true).append(true).open(path).ok();
    }
    if let Some(f) = file.as_mut() {
        let _ = f.write_all(line.as_bytes());
    }
}

/// What the libraries say through the `log` crate (gamepad-facade's warnings), into the same file.
struct Sink;

impl ::log::Log for Sink {
    fn enabled(&self, metadata: &::log::Metadata) -> bool {
        metadata.level() <= ::log::Level::Warn
    }

    fn log(&self, record: &::log::Record) {
        if self.enabled(record.metadata()) {
            log(record.target(), record.args());
        }
    }

    fn flush(&self) {}
}

/// Send the libraries' warnings to the log file.
pub fn catch_library_logs() {
    if ::log::set_logger(&Sink).is_ok() {
        ::log::set_max_level(::log::LevelFilter::Warn);
    }
}
