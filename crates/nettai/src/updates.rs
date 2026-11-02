//! The app's updates (docs/app.md §19, "Updates"). Where the app has an updater of its own, it asks nettai's GitHub
//! releases at start whether a newer one is out, unless Settings turned that off: the desktop's installed app (a
//! package Velopack made: packaging/; a build run from the repository isn't installed, and asks nothing), and
//! Android's (the release's APK). The loading screen waits for the answer ([`CHECK_WAIT`] at most: releases that
//! don't answer by then are let go, and asked again next start). One found is downloaded there, the bar saying how
//! far, and then put in place: on the desktop by Velopack's updater, without a window of its own, which starts the
//! app again in it; on Android by the system's installer, which ends the app. Nothing is asked of the player (but by
//! Android, the first time), and nothing is updated during a session. A browser's page is new each time it is loaded,
//! and iOS's app is its sideloading tool's: neither has an updater, and none is found there.
//!
//! Each system's module has the same few things: `startup`, `check` (which tells what it found, [`Told`]), `Found`
//! (the update found), `download` and `apply`.

use crate::shell::Shell;
use nettai_ui::UpdateWords;
use slint::ComponentHandle;
use std::time::Duration;
use web_time::Instant;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
mod velopack;
#[cfg(target_os = "android")]
mod android;
// (The release's JSON read, which the tests run on the host too.)
#[cfg(any(target_os = "android", test))]
mod release;
#[cfg(any(target_arch = "wasm32", target_os = "ios"))]
mod none;

#[cfg(not(any(target_arch = "wasm32", target_os = "ios", target_os = "android")))]
use velopack as system;
#[cfg(target_os = "android")]
use android as system;
#[cfg(any(target_arch = "wasm32", target_os = "ios"))]
use none as system;

/// Run first thing in `main` (Velopack's hook on the desktop: run as an installer's or an updater's, it does its part
/// and the process ends there).
pub use system::startup;

/// How long the loading screen waits for the releases to say whether there is an update.
const CHECK_WAIT: Duration = Duration::from_secs(5);

/// Where the app's update stands.
#[derive(Default)]
pub struct Updates {
    stage: Stage,
}

#[derive(Default)]
enum Stage {
    /// Not asked (yet, or this session: Settings turned it off).
    #[default]
    Idle,
    /// The releases are asked, since then.
    Checking(Instant),
    /// An update is downloading, this far (percent).
    Downloading(Box<system::Found>, i16),
    /// The update to this version is downloaded, and being put in place (Android's installer, which may ask the
    /// player first): the app ends once it is.
    Applying(String),
    /// Nothing more this session: no update, the releases didn't answer in time, or the update failed.
    Settled,
}

/// What the loading screen waits on.
pub enum Holding {
    /// The answer of the releases.
    Checking,
    /// The update to `version`, downloaded this far (0 to 1).
    Downloading { version: String, progress: f32 },
}

/// Told on the event loop by a system's module.
#[cfg_attr(any(target_arch = "wasm32", target_os = "ios"), allow(dead_code))]
enum Told {
    Found(system::Found),
    /// No update: none is out, the app isn't installed, or the releases couldn't be asked.
    None,
    Progress(i16),
    Downloaded(Result<(), String>),
    /// The update wasn't put in place (Android's installer said so; it ends the app when it is): the player turned
    /// it down (`declined`), or it failed.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    NotApplied { why: String, declined: bool },
}

#[cfg_attr(any(target_arch = "wasm32", target_os = "ios"), allow(dead_code))]
fn tell(told: Told) {
    let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.shell.updates_told(told)));
}

impl Updates {
    /// Ask the releases whether a newer one is out: as the app starts, once.
    pub fn check(&mut self) {
        if !matches!(self.stage, Stage::Idle) {
            return;
        }
        self.stage = if system::check() { Stage::Checking(Instant::now()) } else { Stage::Settled };
    }

    /// What the loading screen waits on, if anything: the releases' answer, for [`CHECK_WAIT`] at most (then let
    /// go: an answer after it is taken up next start), or the update downloading and being put in place.
    pub fn holding(&mut self) -> Option<Holding> {
        match &self.stage {
            Stage::Checking(since) if since.elapsed() < CHECK_WAIT => Some(Holding::Checking),
            Stage::Checking(_) => {
                log::info!("no answer about an update in {} seconds: asked again next start", CHECK_WAIT.as_secs());
                self.stage = Stage::Settled;
                None
            }
            Stage::Downloading(found, percent) => Some(Holding::Downloading {
                version: found.version().to_owned(),
                progress: f32::from((*percent).clamp(0, 100)) / 100.0,
            }),
            Stage::Applying(version) => Some(Holding::Downloading { version: version.clone(), progress: 1.0 }),
            Stage::Idle | Stage::Settled => None,
        }
    }
}

impl Shell {
    fn updates_told(&mut self, told: Told) {
        match told {
            Told::Found(found) => {
                if !matches!(self.updates.stage, Stage::Checking(_)) {
                    log::info!("nettai {} is out: it is downloaded next start", found.version());
                    return;
                }
                system::download(&found);
                self.updates.stage = Stage::Downloading(Box::new(found), 0);
            }
            Told::None => {
                if matches!(self.updates.stage, Stage::Checking(_)) {
                    self.updates.stage = Stage::Settled;
                }
            }
            Told::Progress(percent) => {
                if let Stage::Downloading(_, at) = &mut self.updates.stage {
                    *at = percent;
                }
            }
            Told::Downloaded(Ok(())) => {
                let Stage::Downloading(found, _) = std::mem::replace(&mut self.updates.stage, Stage::Settled) else { return };
                self.updates.stage = Stage::Applying(found.version().to_owned());
                if let Err(why) = system::apply(*found) {
                    self.updates.stage = Stage::Settled;
                    self.update_failed("starting the update", &why);
                }
            }
            Told::Downloaded(Err(why)) => {
                self.updates.stage = Stage::Settled;
                self.update_failed("downloading an update", &why);
            }
            Told::NotApplied { why, declined } => {
                if !matches!(self.updates.stage, Stage::Applying(_)) {
                    return;
                }
                self.updates.stage = Stage::Settled;
                if declined {
                    crate::errors::log("installing the update", &format!("turned down: {why}"));
                } else {
                    self.update_failed("installing the update", &why);
                }
            }
        }
    }

    /// The update couldn't be put in place: logged, and said in the footer; the app goes on as it is.
    fn update_failed(&mut self, doing: &str, why: &str) {
        crate::errors::log(doing, &why);
        let ui = self.ui();
        ui.global::<nettai_ui::Shell>().set_updating(ui.global::<UpdateWords>().get_failed());
    }
}
