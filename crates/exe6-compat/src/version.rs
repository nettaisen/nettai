//! Which version of EXE6 a player plays, as the original tells them apart:
//! what its saves, its traces and its link data say (Gregar 0, Falzar 1).
//! The engine has no such type. It and the content know a version by the
//! name the game's rules declare (`version = { "gregar", "falzar" }` in the
//! cross and beast parts' setups), and a match's side holds that name;
//! this is those names at compat's boundary.

/// A version of EXE6: it decides a player's Crosses and Beast form. Neither
/// is a default: whoever makes a player says which.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameVersion {
    Gregar,
    Falzar,
}

impl GameVersion {
    /// Both, in the original's order (0 Gregar, 1 Falzar).
    pub const ALL: [GameVersion; 2] = [GameVersion::Gregar, GameVersion::Falzar];

    /// The name EXE6's rules declare for it (their setups' `version`, a
    /// form's `version`, a navi's `forms`).
    pub fn name(self) -> &'static str {
        match self {
            GameVersion::Gregar => "gregar",
            GameVersion::Falzar => "falzar",
        }
    }

    /// The version named `name`, if it is one of EXE6's.
    pub fn from_name(name: &str) -> Option<GameVersion> {
        GameVersion::ALL.into_iter().find(|v| v.name() == name)
    }

    /// The navi's version as NaviStats+0x20 has it (which MstrCros reads).
    pub fn stats_byte(self) -> u8 {
        match self {
            GameVersion::Gregar => 0,
            GameVersion::Falzar => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_is_its_name() {
        for v in GameVersion::ALL {
            assert_eq!(GameVersion::from_name(v.name()), Some(v));
        }
        assert_eq!((GameVersion::from_name("azure"), GameVersion::from_name("Gregar")), (None, None));
        assert_eq!(GameVersion::ALL.map(GameVersion::stats_byte), [0, 1]);
    }
}
