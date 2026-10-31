//! The app's updates (docs/app.md §19, "Updates"). An installed app (a package Velopack made: packaging/) asks
//! nettai's GitHub releases at start whether a newer one is out, unless Settings turned that off; a build run from
//! the repository isn't installed, and asks nothing. The loading screen waits for the answer ([`CHECK_WAIT`] at
//! most: releases that don't answer by then are let go, and asked again next start). One found is downloaded there,
//! the bar saying how far, and Velopack's updater then puts it in place without a window of its own and starts the
//! app again in it. Nothing is asked of the player, and nothing is updated during a session.

use crate::shell::Shell;
use nettai_ui::UpdateWords;
use slint::ComponentHandle;
use std::time::{Duration, Instant};
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

/// Where the releases are.
const REPO: &str = "https://github.com/nettaisen/nettai";

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
    Downloading(Box<UpdateInfo>, i16),
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
/// packaging/macos), so that an app from the universal package (`osx`, a Mac's only one up to 0.12.0, made up to
/// 0.18.1) takes its own at its next update, the slice it runs choosing (docs/app.md §19, "Updates").
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
    /// No update: none is out, the app isn't installed, or the releases couldn't be asked.
    None,
    Progress(i16),
    Downloaded(Result<(), String>),
}

fn tell(told: Told) {
    let _ = slint::invoke_from_event_loop(move || crate::with_app(|app| app.shell.updates_told(told)));
}

impl Updates {
    /// Ask the releases whether a newer one is out, on a thread: as the app starts, once.
    pub fn check(&mut self) {
        if !matches!(self.stage, Stage::Idle) {
            return;
        }
        self.stage = Stage::Checking(Instant::now());
        std::thread::spawn(|| {
            let Some(manager) = manager() else { return tell(Told::None) };
            match manager.check_for_updates() {
                Ok(UpdateCheck::UpdateAvailable(info)) => tell(Told::Found(*info)),
                Ok(_) => tell(Told::None),
                Err(why) => {
                    crate::errors::log("checking for an update", &why);
                    tell(Told::None);
                }
            }
        });
    }

    /// What the loading screen waits on, if anything: the releases' answer, for [`CHECK_WAIT`] at most (then let
    /// go: an answer after it is taken up next start), or the update downloading.
    pub fn holding(&mut self) -> Option<Holding> {
        match &self.stage {
            Stage::Checking(since) if since.elapsed() < CHECK_WAIT => Some(Holding::Checking),
            Stage::Checking(_) => {
                log::info!("no answer about an update in {} seconds: asked again next start", CHECK_WAIT.as_secs());
                self.stage = Stage::Settled;
                None
            }
            Stage::Downloading(info, percent) => Some(Holding::Downloading {
                version: info.TargetFullRelease.Version.clone(),
                progress: f32::from((*percent).clamp(0, 100)) / 100.0,
            }),
            Stage::Idle | Stage::Settled => None,
        }
    }
}

impl Shell {
    fn updates_told(&mut self, told: Told) {
        match told {
            Told::Found(info) => {
                if !matches!(self.updates.stage, Stage::Checking(_)) {
                    log::info!("nettai {} is out: it is downloaded next start", info.TargetFullRelease.Version);
                    return;
                }
                self.updates.stage = Stage::Downloading(Box::new(info.clone()), 0);
                download(info);
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
                let Stage::Downloading(info, _) = std::mem::replace(&mut self.updates.stage, Stage::Settled) else { return };
                let Some(manager) = manager() else { return };
                // The updater waits for this process to end, then puts the update in place silently (no window of
                // its own) and starts the app again. (The process ends here: the settings are written as each
                // changes.)
                match manager.wait_exit_then_apply_updates(&info.TargetFullRelease, true, true, Vec::<String>::new()) {
                    Ok(()) => std::process::exit(0),
                    Err(why) => self.update_failed("starting the update", &why.to_string()),
                }
            }
            Told::Downloaded(Err(why)) => {
                self.updates.stage = Stage::Settled;
                self.update_failed("downloading an update", &why);
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

/// Download `info`'s update on a thread, telling how far.
fn download(info: UpdateInfo) {
    std::thread::spawn(move || {
        let Some(manager) = manager() else { return tell(Told::Downloaded(Err("the app isn't installed".into()))) };
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

    /// A release with the universal package beside the architectures' (as up to 0.18.1).
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
