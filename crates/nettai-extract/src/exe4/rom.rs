pub(crate) use crate::rom::{Place, Rom, Roms, Source};

/// A ROM's version: Red Sun or Blue Moon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    RedSun,
    BlueMoon,
}

impl Version {
    /// The name of a version's own asset (`redsun`, `bluemoon`), as
    /// compat/chips.toml writes a version's own chip's.
    pub fn name(self) -> &'static str {
        match self {
            Version::RedSun => "redsun",
            Version::BlueMoon => "bluemoon",
        }
    }
}

// The four ROMs (`Roms`, in this order): Red Sun US (`MEGAMANBN4RS`, B4WE),
// Blue Moon US (`MEGAMANBN4BM`, B4BE), Red Sun Japan (`ROCK_EXE4_RS`, B4WJ)
// and Blue Moon Japan (`ROCK_EXE4_BM`, B4BJ). Red Sun's is the base version.
// Each block the pack takes is stated where it is read (graphics.rs, hud.rs,
// custom.rs) by where the game's own pointer to it is in each ROM (`Place`):
// the same literal of the same routine, mostly at one address in both
// versions' code of a release.
impl crate::rom::Version for Version {
    fn is_other(self) -> bool {
        self == Version::BlueMoon
    }
}
