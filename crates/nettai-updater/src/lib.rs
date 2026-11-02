//! The app's own updates, from nettai's GitHub releases (docs/app.md §19, "Updates"), where the app has an updater:
//! the desktop's installed app (a package Velopack made: packaging/; a build run from the repository isn't installed,
//! and asks nothing), and Android's (the release's APK). A browser's page is new each time it is loaded, and iOS's app
//! is its sideloading tool's: neither asks, and no update is found there.
//!
//! [`Updater::check`] asks the releases, once, whether a newer one is out; [`Updater::poll`] says where the update
//! stands, on the app's thread (the work is on threads of this crate's): asked ([`CHECK_WAIT`] at most: releases that
//! don't answer by then are let go, and asked again next start), the one found downloading, then being installed. The
//! app ends once it is: on the desktop Velopack's updater puts it in place and starts the app again; Android's
//! installer ends the app. What goes wrong is logged (`log`, a warning) and said once ([`Status::Failed`]); the app
//! goes on as it is, and it is tried again next start. Nothing is asked of the player (but by Android, the first time).
//!
//! Each system is a module (`src/platform.rs`). Unsafe code is Android's JNI alone (`src/platform/android.rs`).
#![deny(unsafe_code)]

mod platform;
#[cfg(any(android, test))]
mod release;

use platform::system;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// How long the releases are waited on for their answer.
pub const CHECK_WAIT: Duration = Duration::from_secs(5);

/// Run first thing in the app's `main`: on the desktop Velopack's hook, which, run by its installer or its updater,
/// does its part and ends the process there; elsewhere nothing.
pub fn startup() {
    system::startup();
}

/// Where the update stands ([`Updater::poll`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Nothing is going on: not asked (by the app, or on a system with no updater), or nothing more this session (no
    /// update, no answer in time, or it failed).
    Idle,
    /// The releases are asked whether a newer one is out.
    Checking,
    /// The update to `version` is downloading, this far (0 to 1).
    Downloading { version: String, progress: f32 },
    /// The update to `version` is downloaded and being installed: the app ends once it is (Android's installer may
    /// ask the player first).
    Installing { version: String },
    /// The update failed, just now (logged): said once, [`Status::Idle`] after.
    Failed,
}

/// What a system's module tells, from its threads (or Java's).
#[cfg_attr(any(web, ios), allow(dead_code))]
pub(crate) enum Told {
    Found(system::Found),
    /// No update: none is out, the app isn't installed, or the releases couldn't be asked (logged).
    None,
    /// The download, this far (0 to 1).
    Progress(f32),
    Downloaded(Result<(), String>),
    /// The update wasn't installed (Android's installer said so: it ends the app when it is): the player turned it
    /// down (`declined`), or it failed.
    #[cfg_attr(not(android), allow(dead_code))]
    NotInstalled { why: String, declined: bool },
}

/// The app's update: one an app, asked as it starts.
pub struct Updater {
    stage: Stage,
    tell: Sender<Told>,
    heard: Receiver<Told>,
}

enum Stage {
    /// Not asked yet.
    Idle,
    /// The releases are asked, since then.
    Checking(Instant),
    /// The update found is downloading, this far.
    Downloading(Box<system::Found>, f32),
    /// The update to this version is being installed.
    Installing(String),
    /// Nothing more this session.
    Settled,
}

impl Default for Updater {
    fn default() -> Self {
        Updater::new()
    }
}

impl Updater {
    pub fn new() -> Updater {
        let (tell, heard) = mpsc::channel();
        Updater { stage: Stage::Idle, tell, heard }
    }

    /// Ask the releases whether a newer one is out: once, as the app starts. Where the system has no updater nothing
    /// is asked.
    pub fn check(&mut self) {
        if !matches!(self.stage, Stage::Idle) {
            return;
        }
        // (Only a system with an updater asks, and keeps the time: a browser's `Instant` has none.)
        self.stage = if system::check(self.tell.clone()) { Stage::Checking(Instant::now()) } else { Stage::Settled };
    }

    /// Where the update stands, with what the system said since: an update found is downloaded, one downloaded
    /// installed.
    pub fn poll(&mut self) -> Status {
        let mut failed = false;
        while let Ok(told) = self.heard.try_recv() {
            failed |= self.take(told);
        }
        if let Stage::Checking(since) = self.stage
            && since.elapsed() >= CHECK_WAIT
        {
            log::info!("no answer about an update in {} seconds: asked again next start", CHECK_WAIT.as_secs());
            self.stage = Stage::Settled;
        }
        if failed {
            return Status::Failed;
        }
        match &self.stage {
            Stage::Checking(_) => Status::Checking,
            Stage::Downloading(found, progress) => {
                Status::Downloading { version: found.version().to_owned(), progress: *progress }
            }
            Stage::Installing(version) => Status::Installing { version: version.clone() },
            Stage::Idle | Stage::Settled => Status::Idle,
        }
    }

    /// What a system said, taken: whether the update failed.
    fn take(&mut self, told: Told) -> bool {
        match told {
            Told::Found(found) => {
                if !matches!(self.stage, Stage::Checking(_)) {
                    log::info!("nettai {} is out: it is downloaded next start", found.version());
                    return false;
                }
                system::download(&found, self.tell.clone());
                self.stage = Stage::Downloading(Box::new(found), 0.0);
                false
            }
            Told::None => {
                if matches!(self.stage, Stage::Checking(_)) {
                    self.stage = Stage::Settled;
                }
                false
            }
            Told::Progress(progress) => {
                if let Stage::Downloading(_, at) = &mut self.stage {
                    *at = progress.clamp(0.0, 1.0);
                }
                false
            }
            Told::Downloaded(done) => {
                let Stage::Downloading(found, _) = std::mem::replace(&mut self.stage, Stage::Settled) else { return false };
                if let Err(why) = done {
                    log::warn!("downloading the update to nettai {}: {why}", found.version());
                    return true;
                }
                self.stage = Stage::Installing(found.version().to_owned());
                match system::install(*found, self.tell.clone()) {
                    Ok(()) => false,
                    Err(why) => {
                        log::warn!("installing the update: {why}");
                        self.stage = Stage::Settled;
                        true
                    }
                }
            }
            Told::NotInstalled { why, declined } => {
                let Stage::Installing(version) = std::mem::replace(&mut self.stage, Stage::Settled) else { return false };
                if declined {
                    log::info!("the update to nettai {version} was turned down: {why}");
                } else {
                    log::warn!("installing the update to nettai {version}: {why}");
                }
                !declined
            }
        }
    }
}
