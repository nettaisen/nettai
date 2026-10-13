mod custom;
pub(super) mod graphics;
mod gregar;
mod hud;
mod jp;
mod lettering;
pub(super) mod names;
use crate::rom::{Place, Rom, Roms, Source};
fn u32at(rom: &Rom, a: u32) -> u32 {
    rom.u32(a)
}

/// A ROM's version, which some of what it holds is its own.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Version {
    Falzar,
    Gregar,
}

impl Version {
    /// A version's name as the pack has what is its own under.
    pub fn name(self) -> &'static str {
        match self {
            Version::Falzar => "falzar",
            Version::Gregar => "gregar",
        }
    }
}

impl crate::rom::Version for Version {
    fn is_other(self) -> bool {
        self == Version::Gregar
    }
}
