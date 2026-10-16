//! The app's updates (docs/app.md §19, "Updates"). An installed app (a package Velopack made: packaging/) asks
//! nettai's GitHub releases at start whether a newer one is out, unless Settings turned that off; a build run from
//! the repository isn't installed, and asks nothing. One found, the player is asked at the main menu, once
//! (`UpdateWords`): UPDATE downloads it on a thread, the footer saying how far, and once it is down the app restarts
//! into it the next time the main menu shows with nothing over it, never in a battle.

use crate::shell::{Dialog, Shell, button};
use nettai_ui::{ButtonKind, UpdateWords};
use slint::ComponentHandle;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager};

/// Where the releases are.
const REPO: &str = "https://github.com/nettaisen/nettai";

/// The tag of the question asked when an update is found (`shell::App::answer`: 1 is UPDATE).
pub const ASK: &str = "update";

/// Where the app's update stands.
#[derive(Default)]
pub struct Updates {
    checking: bool,
    /// The update found, and whether the player was asked about it.
    found: Option<UpdateInfo>,
    asked: bool,
    /// It is downloading, or down and waiting for the main menu.
    downloading: bool,
    ready: bool,
}

/// Velopack's own start, first thing in `main`: run as an installer's or an updater's hook, it does its part and the
/// process ends there; otherwise it does nothing.
pub fn startup() {
    velopack::VelopackApp::build().run();
}

/// The installed app's update manager; none for an app that isn't installed.
fn manager() -> Option<UpdateManager> {
    UpdateManager::new(GithubSource::new(REPO, None, false), None, None).ok()
}

/// Told on the event loop from a thread of this module's.
enum Told {
    Found(UpdateInfo),
    Progress(i16),
    Downloaded(Result<(), String>),
}

fn tell(told: Told) {
    let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.shell.updates_told(told)));
}

impl Updates {
    /// Ask the releases whether a newer one is out, on a thread (once a session; again only once that one has said).
    pub fn check(&mut self) {
        if self.checking || self.found.is_some() {
            return;
        }
        self.checking = true;
        std::thread::spawn(|| {
            let Some(manager) = manager() else { return };
            match manager.check_for_updates() {
                Ok(UpdateCheck::UpdateAvailable(info)) => tell(Told::Found(*info)),
                Ok(_) => {}
                Err(why) => crate::errors::log("checking for an update", &why),
            }
        });
    }
}

impl Shell {
    fn updates_told(&mut self, told: Told) {
        let ui = self.ui();
        let words = ui.global::<UpdateWords>();
        let footer = ui.global::<nettai_ui::Shell>();
        match told {
            Told::Found(info) => {
                self.updates.checking = false;
                self.updates.found = Some(info);
            }
            Told::Progress(percent) => footer.set_updating(words.invoke_downloading(percent.clamp(0, 100) as i32)),
            Told::Downloaded(Ok(())) => {
                self.updates.downloading = false;
                self.updates.ready = true;
                footer.set_updating(words.get_ready());
            }
            Told::Downloaded(Err(why)) => {
                crate::errors::log("downloading an update", &why);
                self.updates.downloading = false;
                footer.set_updating(words.get_failed());
            }
        }
    }

    /// The main menu shows with nothing over it (`screens::menu`): an update found is asked about, once; one
    /// downloaded is put in place, and the app starts again in it.
    pub fn at_menu(&mut self) {
        if self.dialog.is_some() || self.battle.is_some() {
            return;
        }
        if self.updates.ready {
            self.updates.ready = false;
            let (Some(manager), Some(info)) = (manager(), &self.updates.found) else { return };
            // (It ends the process once the updater is started: the settings are written as each changes.)
            if let Err(why) = manager.apply_updates_and_restart(&info.TargetFullRelease) {
                crate::errors::log("restarting into an update", &why);
                self.ui().global::<nettai_ui::Shell>().set_updating(self.ui().global::<UpdateWords>().get_failed());
            }
            return;
        }
        let Some(info) = &self.updates.found else { return };
        if self.updates.asked || !self.settings.check_updates {
            return;
        }
        self.updates.asked = true;
        let ui = self.ui();
        let words = ui.global::<UpdateWords>();
        let dialog = Dialog {
            tag: ASK,
            title: words.invoke_title(info.TargetFullRelease.Version.as_str().into()).into(),
            message: words.get_message().into(),
            buttons: vec![button(&words.get_later(), ButtonKind::Normal), button(&words.get_update(), ButtonKind::Main)],
            focus: 1,
            cancel: 0,
        };
        self.open_dialog(dialog, None, false);
    }

    /// UPDATE: the update downloads on a thread, the footer saying how far.
    pub fn download_update(&mut self) {
        let Some(info) = self.updates.found.clone() else { return };
        if self.updates.downloading || self.updates.ready {
            return;
        }
        self.updates.downloading = true;
        self.updates_told(Told::Progress(0));
        std::thread::spawn(move || {
            let Some(manager) = manager() else { return };
            let (progress, heard) = std::sync::mpsc::channel::<i16>();
            let forward = std::thread::spawn(move || {
                for percent in heard {
                    tell(Told::Progress(percent));
                }
            });
            let done = manager.download_updates(&info, Some(progress)).map_err(|why| why.to_string());
            let _ = forward.join();
            tell(Told::Downloaded(done));
        });
    }
}
