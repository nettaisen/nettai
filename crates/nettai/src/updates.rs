//! The app's updates (docs/app.md §19, "Updates"). An installed app (a package Velopack made: packaging/) asks
//! nettai's GitHub releases at start whether a newer one is out, unless Settings turned that off; a build run from
//! the repository isn't installed, and asks nothing. One found, the player is asked at the main menu, once
//! (`UpdateWords`): UPDATE downloads it on a thread, the footer saying how far, and once it is down the app restarts
//! into it the next time the main menu shows with nothing over it, never in a battle.

use crate::shell::{Dialog, Shell, button};
use nettai_ui::{ButtonKind, UpdateWords};
use slint::ComponentHandle;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

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
    let options = options(std::env::consts::OS, std::env::consts::ARCH);
    UpdateManager::new(GithubSource::new(REPO, None, false), Some(options), None).ok()
}

/// The channel the app asks for its updates on, when it isn't the one its package was made on (`vpk pack
/// --channel`, which Velopack's manager asks otherwise): on a Mac, the architecture's (`osx-arm64`, `osx-x64`:
/// packaging/macos), so that an app from the universal package (`osx`, a Mac's only one up to 0.12.0) takes its
/// own at its next update, the slice it runs choosing (docs/app.md §19, "Updates").
fn options(os: &str, arch: &str) -> UpdateOptions {
    let channel = match (os, arch) {
        ("macos", "aarch64") => Some("osx-arm64"),
        ("macos", "x86_64") => Some("osx-x64"),
        _ => None,
    };
    UpdateOptions { ExplicitChannel: channel.map(str::to_owned), ..UpdateOptions::default() }
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

#[cfg(test)]
mod tests {
    use super::options;
    use velopack::locator::VelopackLocatorConfig;
    use velopack::sources::FileSource;
    use velopack::{UpdateCheck, UpdateManager};

    /// The full package an installed app of `version`, packaged on `channel` and running on `os`/`arch`, updates
    /// to from a release's feeds (`releases.<channel>.json`, each channel's versions), if any: Velopack's own check,
    /// with the app's options.
    fn update_to(version: &str, channel: &str, (os, arch): (&str, &str), feeds: &[(&str, &[&str])]) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("nettai.app");
        let programs = app.join("Contents/MacOS");
        let (packages, release) = (dir.path().join("packages"), dir.path().join("release"));
        for folder in [&programs, &packages, &release] {
            std::fs::create_dir_all(folder).unwrap();
        }
        // (What `vpk pack` puts beside the program, where Velopack's locator reads them: the manifest and the updater.)
        let manifest = programs.join("sq.version");
        std::fs::write(
            &manifest,
            format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd"><metadata>
<id>nettai</id><version>{version}</version><channel>{channel}</channel><mainExe>nettai</mainExe>
</metadata></package>"#
            ),
        )
        .unwrap();
        let updater = programs.join("UpdateMac");
        std::fs::write(&updater, []).unwrap();
        for (channel, versions) in feeds {
            let assets: Vec<_> = versions
                .iter()
                .map(|version| {
                    serde_json::json!({
                        "PackageId": "nettai",
                        "Version": version,
                        "Type": "Full",
                        "FileName": format!("nettai-{version}-{channel}-full.nupkg"),
                    })
                })
                .collect();
            let feed = serde_json::json!({ "Assets": assets }).to_string();
            std::fs::write(release.join(format!("releases.{channel}.json")), feed).unwrap();
        }
        let locator = VelopackLocatorConfig {
            RootAppDir: app,
            UpdateExePath: updater,
            PackagesDir: packages,
            ManifestPath: manifest,
            CurrentBinaryDir: programs,
            IsPortable: true,
        };
        let manager = UpdateManager::new(FileSource::new(&release), Some(options(os, arch)), Some(locator)).unwrap();
        match manager.check_for_updates().unwrap() {
            UpdateCheck::UpdateAvailable(info) => Some(info.TargetFullRelease.FileName),
            UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => None,
        }
    }

    /// The full package of a version on a channel, named as the feeds above name it.
    fn full(version: &str, channel: &str) -> Option<String> {
        Some(format!("nettai-{version}-{channel}-full.nupkg"))
    }

    const APPLE_SILICON: (&str, &str) = ("macos", "aarch64");
    const INTEL_MAC: (&str, &str) = ("macos", "x86_64");

    /// A release with the universal package beside the architectures' (the macOS job's three).
    const RELEASE: &[(&str, &[&str])] = &[("osx", &["0.14.0"]), ("osx-arm64", &["0.14.0"]), ("osx-x64", &["0.14.0"])];

    #[test]
    fn a_universal_app_takes_its_architectures_package() {
        assert_eq!(update_to("0.13.0", "osx", APPLE_SILICON, RELEASE), full("0.14.0", "osx-arm64"));
        assert_eq!(update_to("0.13.0", "osx", INTEL_MAC, RELEASE), full("0.14.0", "osx-x64"));
    }

    #[test]
    fn a_universal_app_waits_for_a_newer_version() {
        // (The same version on its architecture's channel is no update: it moves at the next one.)
        let same = &[("osx", &["0.13.0"][..]), ("osx-arm64", &["0.13.0"][..])];
        assert_eq!(update_to("0.13.0", "osx", APPLE_SILICON, same), None);
    }

    #[test]
    fn an_architectures_app_stays_on_its_channel() {
        let newer = &[("osx-arm64", &["0.15.0"][..]), ("osx-x64", &["0.15.0"][..])];
        assert_eq!(update_to("0.14.0", "osx-arm64", APPLE_SILICON, newer), full("0.15.0", "osx-arm64"));
        assert_eq!(update_to("0.14.0", "osx-x64", INTEL_MAC, newer), full("0.15.0", "osx-x64"));
    }

    #[test]
    fn other_systems_ask_their_packages_channel() {
        let release = &[("win", &["0.14.0"][..]), ("linux", &["0.14.0"][..])];
        assert_eq!(update_to("0.13.0", "win", ("windows", "x86_64"), release), full("0.14.0", "win"));
        assert_eq!(update_to("0.13.0", "linux", ("linux", "x86_64"), release), full("0.14.0", "linux"));
    }
}
