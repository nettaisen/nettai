//! The player's settings, kept between runs: a TOML file in the app's data
//! folder (`crate::paths::settings`), written each time one changes and
//! read as the app starts. A setting the file leaves out (or one it can't
//! read) is the app's own default.

use serde::{Deserialize, Serialize};

/// What the file keeps.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    /// The language, by its code (`ja`).
    pub language: Option<String>,
    /// 0 to 10.
    pub volume: Option<u32>,
    pub menu_sounds: Option<bool>,
    /// The battle's text in the vector font (else the game's own).
    pub crisp_text: Option<bool>,
    /// The picture at the display's own density.
    pub sharp: Option<bool>,
    pub name: Option<String>,
}

/// The settings as the file has them (none where it isn't there or doesn't
/// read).
pub fn load() -> Saved {
    std::fs::read_to_string(crate::paths::settings()).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default()
}

/// Write the settings to the file.
pub fn save(s: &Saved) -> Result<(), String> {
    let path = crate::paths::settings();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = toml::to_string(s).map_err(|e| e.to_string())?;
    std::fs::write(&path, format!("# nettai's settings (docs/app.md §9).\n\n{text}")).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Settings read back as written; a file of other keys, or none, is
    /// the defaults.
    #[test]
    fn settings_read_back() {
        let s = Saved { language: Some("ja".into()), volume: Some(3), menu_sounds: Some(false), crisp_text: None, sharp: Some(true), name: Some("Lan".into()) };
        let text = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<Saved>(&text).unwrap(), s);
        assert_eq!(toml::from_str::<Saved>("volume = 4\nunknown = 1\n").unwrap(), Saved { volume: Some(4), ..Saved::default() });
    }
}
