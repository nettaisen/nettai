//! The player's settings, kept between runs: a TOML file in the app's data folder (`crate::paths::settings`),
//! written each time one changes and read as the app starts. A setting the file leaves out (or one it can't
//! read) is the app's own default.
//!
//! A screen that remembers something of its own (the builds Practice last used, the rounds last played) keeps
//! it by a key in [`Settings::remembered`], so it adds no field here.

use crate::input::{BUTTONS, Bindings, key_name, key_named, pad_name, pad_named};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The most a name is, as the lobby carries it.
pub const NAME_LENGTH: usize = nettai_frontend::lobby::NAME_LENGTH;

/// What the file keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The player's name, shown to their opponent.
    pub name: String,
    /// The language, by its code (`ja`); none: the system's.
    pub language: Option<String>,
    /// The game last used: the one the app opens on.
    pub game: Option<String>,
    /// The most the battle's picture is, in multiples of 240 by 160; 0: the largest that fits.
    pub picture_size: u32,
    pub full_screen: bool,
    /// Motion turned down: everything is a cut. None: as the system asks.
    pub reduce_motion: Option<bool>,
    /// An installed app asks nettai's GitHub releases at start whether a newer one is out (`updates.rs`).
    pub check_updates: bool,
    /// The battle drawn enhanced (its text at the window's resolution, the background's glide, the gauge's smooth
    /// fill, the banners' squash at the window's resolution), else as the game draws it: `GraphicsMode`.
    pub enhanced: bool,
    /// 0 to 100.
    pub music: u32,
    pub effects: u32,
    /// Netplay's input delay, in frames: the value the lobby starts with (the library's
    /// `NetOptions::input_delay`).
    pub input_delay: u32,
    /// The folder the player chose the Legacy Collection's install in, where Steam doesn't list it (docs/app.md
    /// §17); none: Steam's alone.
    pub legacy_collection: Option<std::path::PathBuf>,
    /// The keys and the pad buttons bound, by the GBA button's name; one left out is the default.
    pub keys: BTreeMap<String, String>,
    pub pad: BTreeMap<String, String>,
    /// What screens remember, by their own keys (`play.build.<game>`).
    pub remembered: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            name: system_name(),
            language: None,
            game: None,
            picture_size: 0,
            full_screen: false,
            reduce_motion: None,
            check_updates: true,
            enhanced: true,
            music: 80,
            effects: 100,
            input_delay: nettai_frontend::netplay::DEFAULT_INPUT_DELAY,
            keys: BTreeMap::new(),
            legacy_collection: None,
            pad: BTreeMap::new(),
            remembered: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// How battles are drawn: enhanced, or as the game draws them.
    pub fn graphics(&self) -> nettai_frontend::game::GraphicsMode {
        use nettai_frontend::game::GraphicsMode;
        if self.enhanced { GraphicsMode::Enhanced } else { GraphicsMode::Original }
    }
}

/// The most the input delay can be set to: the library's.
pub const MAX_INPUT_DELAY: u32 = *nettai_frontend::netplay::INPUT_DELAY.end();

/// Whether the system asks for less motion (its accessibility setting), where that can be asked without a
/// library of its own: macOS's and GNOME's.
pub fn system_reduces_motion() -> bool {
    let said = |program: &str, args: &[&str]| std::process::Command::new(program).args(args).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if cfg!(target_os = "macos") {
        said("defaults", &["read", "com.apple.universalaccess", "reduceMotion"]).is_some_and(|v| v == "1")
    } else if cfg!(all(unix, not(target_os = "macos"))) {
        said("gsettings", &["get", "org.gnome.desktop.interface", "enable-animations"]).is_some_and(|v| v == "false")
    } else {
        false
    }
}

/// The name a player has before they choose one: their account's on this machine.
fn system_name() -> String {
    let name = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
    clean_name(&name)
}

/// A name as it is kept: no control characters, 16 at most.
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(NAME_LENGTH).collect()
}

impl Settings {
    /// The settings as the file has them (the defaults where it isn't there or doesn't read).
    pub fn load() -> Settings {
        let mut s: Settings = nettai_fs::read_to_string(crate::paths::settings()).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default();
        s.name = clean_name(&s.name);
        s.music = s.music.min(100);
        s.effects = s.effects.min(100);
        s.input_delay = s.input_delay.min(MAX_INPUT_DELAY);
        s
    }

    /// Write the settings to the file.
    pub fn save(&self) -> Result<(), String> {
        let path = crate::paths::settings();
        if let Some(dir) = path.parent() {
            nettai_fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        nettai_fs::write(&path, format!("# nettai's settings (docs/app.md §9).\n\n{text}")).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The battle's buttons as bound: the defaults, with what the file binds otherwise.
    pub fn bindings(&self) -> Bindings {
        let mut b = Bindings::default();
        for (i, (_, name)) in BUTTONS.iter().enumerate() {
            if let Some(key) = self.keys.get(*name).and_then(|k| key_named(k)) {
                b.keys[i] = key;
            }
            if let Some(button) = self.pad.get(*name).and_then(|p| pad_named(p)) {
                b.pad[i] = button;
            }
        }
        b
    }

    /// Keep `bindings`: what differs from the defaults.
    pub fn set_bindings(&mut self, bindings: &Bindings) {
        let usual = Bindings::default();
        self.keys.clear();
        self.pad.clear();
        for (i, (_, name)) in BUTTONS.iter().enumerate() {
            if bindings.keys[i] != usual.keys[i]
                && let Some(key) = key_name(bindings.keys[i])
            {
                self.keys.insert(name.to_string(), key.to_string());
            }
            if bindings.pad[i] != usual.pad[i]
                && let Some(button) = pad_name(bindings.pad[i])
            {
                self.pad.insert(name.to_string(), button.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::winit_030::winit::keyboard::KeyCode;

    /// Settings read back as written; a file of other keys, or none, is the defaults.
    #[test]
    fn settings_read_back() {
        let mut s = Settings {
            name: "Lan".into(),
            language: Some("ja".into()),
            music: 30,
            full_screen: true,
            input_delay: 2,
            legacy_collection: Some("Steam Library/Legacy Collection Vol. 2".into()),
            ..Settings::default()
        };
        s.remembered.insert("play.build.exe6".into(), "Falzar heat".into());
        s.keys.insert("a".into(), "KeyJ".into());
        let text = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        let other: Settings = toml::from_str("music = 40\nunknown = 1\n").unwrap();
        assert_eq!(other, Settings { music: 40, ..Settings::default() });
        assert_eq!(Settings::default().input_delay, 4, "the input delay is 4 until the player sets another");
    }

    /// The bindings kept are those that differ from the defaults, and read back.
    #[test]
    fn bindings_read_back() {
        let mut s = Settings::default();
        let mut b = s.bindings();
        assert_eq!(b, Bindings::default());
        b.bind_key(4, KeyCode::KeyJ);
        b.bind_pad(5, gamepad_facade::Button::East);
        s.set_bindings(&b);
        assert_eq!((s.keys.len(), s.pad.len()), (1, 1));
        assert_eq!(s.bindings(), b);
    }
}
