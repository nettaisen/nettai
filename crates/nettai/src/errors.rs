//! What can go wrong, as the player reads it (docs/design/app-ui.md §7), and the log (docs/app.md §12).
//!
//! No string from a library's error is ever shown. A failure the player can see is one of [`UserError`]'s
//! kinds, each a sentence of the catalogs (the UI's `Errors`, shell.slint); what the code said goes to the log
//! file alone ([`log`]: `crate::paths::log`). A problem with one field is shown in that field's reserved line
//! ([`UserError::line`]); a failure that ends what the player was doing is a dialog (`Shell::fail`).
//!
//! The netplay libraries' errors are typed the same way, a kind for the player and a detail in English for
//! the log (docs/design/rollback.md §4.9): [`UserError::of_link`], [`UserError::of_lobby`] and
//! [`UserError::of_stop`] write the detail to the log and give the kind's sentence.

use nettai_frontend::driver::StopReason;
use nettai_frontend::lobby::{Failure, Reason};
use nettai_ui::{AppWindow, ErrorKind, Errors};
use slint::ComponentHandle;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

/// A failure the player can see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserError {
    /// The signaling server can't be reached.
    ServerUnreachable,
    /// The signaling server speaks a newer protocol than this app.
    AppTooOld,
    /// No match has that invite code.
    NoSuchMatch,
    /// The match already has two players.
    MatchFull,
    /// The two apps (or their packs) differ.
    VersionMismatch,
    /// The opponent chose another game: `game` is theirs, as shown (EXE5).
    OtherGame { game: String },
    /// This player's build breaks a rule: `reason` is the rule's own sentence, from the catalogs ("2 Giga
    /// chips, limit 1"), where the app's own check found it; none where the other side refused it.
    BuildUnusable { reason: Option<String> },
    /// There is no build of the game to play with.
    NoBuild,
    /// The opponent's build breaks a rule.
    TheirBuildUnusable,
    /// The opponent left the lobby or the match.
    OpponentLeft,
    /// The opponent never answered.
    NoAnswer,
    /// No connection to the opponent could be made.
    CantConnect,
    /// A direct address names no machine.
    BadAddress,
    /// The port to host on is taken.
    PortInUse,
    /// The link dropped for good.
    ConnectionLost,
    /// A replay is from another version.
    ReplayVersion,
    /// A file isn't a replay.
    NotAReplay,
    /// A replay is damaged: it doesn't read, or its match isn't one.
    ReplayDamaged,
    /// A replay is of another game than the one selected: `game` is its own, as shown.
    ReplayOtherGame { game: String },
    /// A video can't be made because ffmpeg can't be started.
    NoFfmpeg,
    /// A video's file can't be written.
    CantWriteVideo,
    /// A video couldn't be made for another reason: the details are in the log.
    ExportFailed,
    /// A save is another game's: `game` is the game it was to be of, as shown (EXE6).
    WrongGameSave { game: String },
    /// A file isn't a ruleset of the game: `game` is the game, as shown. Why is in the log.
    NotARuleset { game: String },
    /// A game's pack is there and can't be loaded (it was extracted for another version of the game's
    /// content): `game` is the game, as shown. The other games play.
    GameOutdated { game: String },
    /// A game has no pack, and there is no Legacy Collection to extract it from: none is found through Steam,
    /// nor in the folder the player chose (docs/app.md §17). Its dialog offers to choose the folder.
    NoLegacyCollection,
    /// A game's pack couldn't be extracted from the Legacy Collection (its archives didn't read, or the
    /// extraction failed): `game` is the game, as shown.
    ExtractFailed { game: String },
    /// Anything else: the details are in the log.
    Other,
}

impl UserError {
    fn kind(&self) -> (ErrorKind, &str) {
        match self {
            UserError::ServerUnreachable => (ErrorKind::ServerUnreachable, ""),
            UserError::AppTooOld => (ErrorKind::AppTooOld, ""),
            UserError::NoSuchMatch => (ErrorKind::NoSuchMatch, ""),
            UserError::MatchFull => (ErrorKind::MatchFull, ""),
            UserError::VersionMismatch => (ErrorKind::VersionMismatch, ""),
            UserError::OtherGame { game } => (ErrorKind::OtherGame, game),
            UserError::BuildUnusable { reason: Some(reason) } => (ErrorKind::BuildUnusable, reason),
            UserError::BuildUnusable { reason: None } => (ErrorKind::BuildRefused, ""),
            UserError::NoBuild => (ErrorKind::NoBuild, ""),
            UserError::TheirBuildUnusable => (ErrorKind::TheirBuildUnusable, ""),
            UserError::OpponentLeft => (ErrorKind::OpponentLeft, ""),
            UserError::NoAnswer => (ErrorKind::NoAnswer, ""),
            UserError::CantConnect => (ErrorKind::CantConnect, ""),
            UserError::BadAddress => (ErrorKind::BadAddress, ""),
            UserError::PortInUse => (ErrorKind::PortInUse, ""),
            UserError::ConnectionLost => (ErrorKind::ConnectionLost, ""),
            UserError::ReplayVersion => (ErrorKind::ReplayVersion, ""),
            UserError::NotAReplay => (ErrorKind::NotAReplay, ""),
            UserError::ReplayDamaged => (ErrorKind::ReplayDamaged, ""),
            UserError::ReplayOtherGame { game } => (ErrorKind::ReplayOtherGame, game),
            UserError::NoFfmpeg => (ErrorKind::NoFfmpeg, ""),
            UserError::CantWriteVideo => (ErrorKind::CantWriteVideo, ""),
            UserError::ExportFailed => (ErrorKind::ExportFailed, ""),
            UserError::WrongGameSave { game } => (ErrorKind::WrongGameSave, game),
            UserError::NotARuleset { game } => (ErrorKind::NotARuleset, game),
            UserError::GameOutdated { game } => (ErrorKind::GameOutdated, game),
            UserError::NoLegacyCollection => (ErrorKind::NoLegacyCollection, ""),
            UserError::ExtractFailed { game } => (ErrorKind::ExtractFailed, game),
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

    /// A link's error (nettai-rtc's), while `doing` something: its detail to the log, its kind as the player
    /// reads it.
    pub fn of_link(doing: &str, error: &nettai_rtc::Error) -> UserError {
        use nettai_rtc::ErrorKind as K;
        log(doing, &error.detail);
        match error.kind {
            K::ServerUnreachable => UserError::ServerUnreachable,
            K::ServerVersion => UserError::AppTooOld,
            K::NoSuchRoom => UserError::NoSuchMatch,
            K::RoomFull => UserError::MatchFull,
            K::BadAddress => UserError::BadAddress,
            K::PortUnavailable => UserError::PortInUse,
            K::CantConnect => UserError::CantConnect,
            K::ConnectionLost => UserError::ConnectionLost,
            K::Internal => UserError::Other,
        }
    }

    /// A lobby that ended without a match, as [`UserError::of_link`]. None where this player left it: nothing
    /// went wrong.
    pub fn of_lobby(failure: &Failure) -> Option<UserError> {
        if failure.reason == Reason::Left {
            return None;
        }
        log("the lobby ended", &failure.detail);
        Some(match &failure.reason {
            Reason::DifferentVersion | Reason::DifferentContent => UserError::VersionMismatch,
            Reason::DifferentGame { theirs } => UserError::OtherGame { game: crate::shell::Shell::game_name(theirs) },
            Reason::YourBuild { .. } => UserError::BuildUnusable { reason: None },
            Reason::TheirBuild { .. } => UserError::TheirBuildUnusable,
            Reason::OpponentLeft => UserError::OpponentLeft,
            Reason::NoAnswer => UserError::NoAnswer,
            Reason::Left | Reason::Protocol => UserError::Other,
        })
    }

    /// Why a file doesn't read as a replay (nettai-replay's), while `doing` something.
    pub fn of_replay_read(doing: &str, error: &nettai_frontend::replay::ReadError) -> UserError {
        use nettai_frontend::replay::ReadError as E;
        log(doing, error);
        match error {
            E::NotAReplay => UserError::NotAReplay,
            E::OtherLayout { .. } => UserError::ReplayVersion,
            E::Damaged(_) => UserError::ReplayDamaged,
        }
    }

    /// Why a replay isn't played on the content loaded, while `doing` something.
    pub fn of_refusal(doing: &str, refused: &nettai_frontend::replay::Refused) -> UserError {
        use nettai_frontend::replay::Refused as R;
        log(doing, refused);
        match refused {
            R::OtherGame { replay, .. } => UserError::ReplayOtherGame { game: crate::shell::Shell::game_name(replay) },
            R::OtherVersion(_) => UserError::ReplayVersion,
            R::BadMatch(_) => UserError::ReplayDamaged,
            R::NoSide(_) => UserError::Other,
        }
    }

    /// Why a video wasn't made (nettai-video's). None where the player canceled it: nothing went wrong.
    pub fn of_export(error: &nettai_video::Error) -> Option<UserError> {
        use nettai_video::Error as E;
        let said = match error {
            E::Canceled => return None,
            E::Replay(refused) => return Some(UserError::of_refusal("exporting a video", refused)),
            E::NoFfmpeg(_) => UserError::NoFfmpeg,
            E::Write(_) | E::Disk(_) => UserError::CantWriteVideo,
            E::NoSuchPart(_) | E::Scale(_) | E::Stopped { .. } | E::FfmpegFailed { .. } => UserError::ExportFailed,
        };
        log("exporting a video", error);
        Some(said)
    }

    /// Why a battle stopped, as the player reads it. None where it came to its end: nothing went wrong. (The
    /// shell writes the detail to the log as the battle stops.)
    pub fn of_stop(reason: StopReason) -> Option<UserError> {
        match reason {
            StopReason::Finished => None,
            StopReason::OpponentLeft => Some(UserError::OpponentLeft),
            StopReason::ConnectionLost => Some(UserError::ConnectionLost),
            StopReason::Internal => Some(UserError::Other),
        }
    }
}

/// The log file, opened at the first line written.
static LOG: Mutex<Option<nettai_fs::File>> = Mutex::new(None);

/// The local time's offset from UTC, read as the libraries' logs are caught ([`catch_library_logs`], at the
/// start), which the log's times are told in. Not read again at each line: chrono reads the local zone through a
/// thread-local, and a line can come as the process exits, after the main thread's thread-locals are gone (a
/// connection libdatachannel still holds when the player quits says it closes from a static destructor), where
/// reading it panics, and a panic in libdatachannel's log callback aborts.
static OFFSET: OnceLock<chrono::FixedOffset> = OnceLock::new();

/// Write a line to the log: what was being done (`doing`) and what the code said of it. This is where an
/// error's own words go; the player reads a [`UserError`].
pub fn log(doing: &str, said: &dyn std::fmt::Display) {
    let offset = OFFSET.get().copied().unwrap_or(chrono::FixedOffset::east_opt(0).expect("UTC"));
    let line = format!("{} {doing}: {said}\n", chrono::Utc::now().with_timezone(&offset).format("%Y-%m-%d %H:%M:%S"));
    let Ok(mut file) = LOG.lock() else { return };
    if file.is_none() {
        let path = crate::paths::log();
        if let Some(dir) = path.parent() {
            let _ = nettai_fs::create_dir_all(dir);
        }
        *file = nettai_fs::OpenOptions::new().create(true).append(true).open(path).ok();
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
        // (A library may log from a C callback, where a panic aborts the app: none comes past here.)
        if self.enabled(record.metadata()) {
            let _ = std::panic::catch_unwind(|| log(record.target(), record.args()));
        }
    }

    fn flush(&self) {}
}

/// Send the libraries' warnings to the log file.
pub fn catch_library_logs() {
    let _ = OFFSET.set(*chrono::Local::now().offset());
    if ::log::set_logger(&Sink).is_ok() {
        ::log::set_max_level(::log::LevelFilter::Warn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind the netplay libraries have maps to a sentence of the player's: none falls through to a
    /// library's own words, and only what nobody can mend is "something went wrong".
    #[test]
    fn every_library_kind_has_the_players_sentence() {
        use nettai_rtc::ErrorKind as K;
        let link = |kind| UserError::of_link("testing", &nettai_rtc::Error { kind, detail: String::new() });
        assert_eq!(link(K::NoSuchRoom), UserError::NoSuchMatch);
        assert_eq!(link(K::ServerVersion), UserError::AppTooOld);
        assert_eq!(link(K::PortUnavailable), UserError::PortInUse);
        for kind in [K::ServerUnreachable, K::RoomFull, K::BadAddress, K::CantConnect, K::ConnectionLost] {
            assert_ne!(link(kind), UserError::Other, "{kind:?}");
        }
        assert_eq!(link(K::Internal), UserError::Other);
        let lobby = |reason| UserError::of_lobby(&Failure { reason, detail: String::new() });
        assert_eq!(lobby(Reason::Left), None, "leaving is no failure");
        assert_eq!(lobby(Reason::DifferentGame { theirs: "exe5".into() }), Some(UserError::OtherGame { game: "EXE5".into() }));
        assert_eq!(lobby(Reason::YourBuild { problems: Vec::new() }), Some(UserError::BuildUnusable { reason: None }));
        assert_eq!(lobby(Reason::TheirBuild { problems: Vec::new() }), Some(UserError::TheirBuildUnusable));
        assert_eq!(lobby(Reason::Protocol), Some(UserError::Other));
        use nettai_frontend::replay::{ReadError, Refused};
        assert_eq!(UserError::of_replay_read("testing", &ReadError::OtherLayout { layout: 2 }), UserError::ReplayVersion);
        assert_eq!(UserError::of_replay_read("testing", &ReadError::NotAReplay), UserError::NotAReplay);
        assert_eq!(UserError::of_refusal("testing", &Refused::OtherGame { replay: "exe5".into(), here: "exe6".into() }), UserError::ReplayOtherGame { game: "EXE5".into() });
        assert_eq!(UserError::of_export(&nettai_video::Error::Canceled), None, "canceling is no failure");
        assert_eq!(UserError::of_export(&nettai_video::Error::NoFfmpeg(std::io::ErrorKind::NotFound.into())), Some(UserError::NoFfmpeg));
        assert_eq!(UserError::of_stop(StopReason::Finished), None);
        assert_eq!(UserError::of_stop(StopReason::OpponentLeft), Some(UserError::OpponentLeft));
    }
}
