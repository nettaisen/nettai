//! The desktop's updates: Velopack's, for an app installed from one of its packages (packaging/), from the project's
//! GitHub releases (`velopack::sources::GithubSource`: the ten newest published, unauthenticated, one request a
//! start), on its system's and architecture's channel ([`options`]). A build run from the repository isn't
//! installed: its manager can't be made, and it asks nothing. The update is downloaded into Velopack's packages
//! folder, and Velopack's updater puts it in place once the app has ended, silently (no window of its own), and starts
//! the app again.

use crate::Told;
use std::sync::mpsc::{self, Sender};
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions};

/// Where the releases are.
const REPO: &str = "https://github.com/nettaisen/nettai";

/// An update found: Velopack's.
pub struct Found(UpdateInfo);

impl Found {
    pub fn version(&self) -> &str {
        &self.0.TargetFullRelease.Version
    }
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

/// The channel the app asks for its updates on, whatever channel its package was made on (`vpk pack --channel`,
/// which Velopack's manager asks otherwise): the system's and the architecture's, as the program was built for them
/// (`macos-arm64`, `macos-x64`, `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`: packaging/), so that an app from
/// a package on a channel before them takes its own at its next update: the channels up to 0.19.0, a Mac's
/// `osx-arm64` and `osx-x64`, Windows' `win` and Linux's `linux` (x86_64's alone), made again in 0.19.1 as the bridge;
/// and a Mac's universal `osx` (its only one up to 0.12.0, made up to 0.18.1), the slice it runs choosing
/// (docs/app.md §19, "Updates").
fn options(os: &str, arch: &str) -> UpdateOptions {
    let channel = match (os, arch) {
        ("macos", "aarch64") => Some("macos-arm64"),
        ("macos", "x86_64") => Some("macos-x64"),
        ("windows", "x86_64") => Some("win-x64"),
        ("windows", "aarch64") => Some("win-arm64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        _ => None,
    };
    UpdateOptions { ExplicitChannel: channel.map(str::to_owned), ..UpdateOptions::default() }
}

/// Ask the releases on a thread (always: whether the app is installed is the thread's to find out).
pub fn check(tell: Sender<Told>) -> bool {
    std::thread::spawn(move || {
        let Some(manager) = manager() else { return drop(tell.send(Told::None)) };
        let told = match manager.check_for_updates() {
            Ok(UpdateCheck::UpdateAvailable(info)) => Told::Found(Found(*info)),
            Ok(_) => Told::None,
            Err(why) => {
                log::warn!("checking for an update: {why}");
                Told::None
            }
        };
        let _ = tell.send(told);
    });
    true
}

/// Download `found`'s update on a thread, telling how far.
pub fn download(found: &Found, tell: Sender<Told>) {
    let info = found.0.clone();
    std::thread::spawn(move || {
        let Some(manager) = manager() else { return drop(tell.send(Told::Downloaded(Err("the app isn't installed".into())))) };
        let (progress, heard) = mpsc::channel::<i16>();
        let forward = {
            let tell = tell.clone();
            std::thread::spawn(move || {
                for percent in heard {
                    let _ = tell.send(Told::Progress(f32::from(percent.clamp(0, 100)) / 100.0));
                }
            })
        };
        let done = manager.download_updates(&info, Some(progress)).map_err(|why| why.to_string());
        let _ = forward.join();
        let _ = tell.send(Told::Downloaded(done));
    });
}

/// The updater waits for this process to end, then puts the update in place silently (no window of its own) and
/// starts the app again. The process ends here (the app writes its settings as each changes).
pub fn install(found: Found, _tell: Sender<Told>) -> Result<(), String> {
    let manager = manager().ok_or("the app isn't installed")?;
    match manager.wait_exit_then_apply_updates(&found.0.TargetFullRelease, true, true, Vec::<String>::new()) {
        Ok(()) => std::process::exit(0),
        Err(why) => Err(why.to_string()),
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
    const WINDOWS_X64: (&str, &str) = ("windows", "x86_64");
    const WINDOWS_ARM64: (&str, &str) = ("windows", "aarch64");
    const LINUX_X64: (&str, &str) = ("linux", "x86_64");
    const LINUX_ARM64: (&str, &str) = ("linux", "aarch64");

    /// Each system and architecture's channel, and an app's system.
    const CHANNELS: [(&str, (&str, &str)); 6] = [
        ("macos-arm64", APPLE_SILICON),
        ("macos-x64", INTEL_MAC),
        ("win-x64", WINDOWS_X64),
        ("win-arm64", WINDOWS_ARM64),
        ("linux-x64", LINUX_X64),
        ("linux-arm64", LINUX_ARM64),
    ];

    /// The channels before, with an app's system and the channel it is to take: the bridge's (up to 0.19.0 the only
    /// ones, made again in 0.19.1), and the Mac's universal one (made up to 0.18.1), the slice it runs choosing. An app
    /// installed from a package made on one asks its architecture's channel, where the releases since are alone.
    const BEFORE: [(&str, (&str, &str), &str); 6] = [
        ("osx-arm64", APPLE_SILICON, "macos-arm64"),
        ("osx-x64", INTEL_MAC, "macos-x64"),
        ("win", WINDOWS_X64, "win-x64"),
        ("linux", LINUX_X64, "linux-x64"),
        ("osx", APPLE_SILICON, "macos-arm64"),
        ("osx", INTEL_MAC, "macos-x64"),
    ];

    /// A release of `versions` on every architecture's channel, and on none before.
    fn release(versions: &'static [&'static str]) -> Vec<(&'static str, &'static [&'static str])> {
        CHANNELS.iter().map(|(channel, _)| (*channel, versions)).collect()
    }

    #[test]
    fn an_app_on_a_channel_before_takes_its_architectures_package() {
        for (before, system, channel) in BEFORE {
            let update = update_to("0.19.1", before, system, &release(&["0.20.0"]));
            assert_eq!(update, full("0.20.0", channel), "{before}, {system:?}");
        }
    }

    #[test]
    fn an_app_on_a_channel_before_waits_for_a_newer_version() {
        // (The same version on its architecture's channel is no update: an app the bridge updated moves at the next
        // one, whichever channel its package was made on.)
        for (before, system, _) in BEFORE {
            assert_eq!(update_to("0.20.0", before, system, &release(&["0.20.0"])), None, "{before}, {system:?}");
        }
    }

    #[test]
    fn an_architectures_app_stays_on_its_channel() {
        for (channel, system) in CHANNELS {
            let update = update_to("0.20.0", channel, system, &release(&["0.21.0"]));
            assert_eq!(update, full("0.21.0", channel), "{channel}");
        }
    }
}
