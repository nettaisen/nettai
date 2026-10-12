mod custom;
pub(super) mod graphics;
mod gregar;
mod hud;
mod jp;
mod lettering;
pub(super) mod names;
use crate::rom::{Place, Rom, Source};
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

/// The four ROMs, any of which may not be there. A pack has whatever one
/// of those given holds: what the four have alike is read from whichever
/// is there, a version's own from a ROM of that version, a language's
/// words from a ROM of that release, the US ROM's before the Japanese
/// one's wherever both have it.
pub(super) struct Roms<'a> {
    pub falzar: &'a Rom,
    pub gregar: &'a Rom,
    pub falzar_jp: &'a Rom,
    pub gregar_jp: &'a Rom,
}

impl<'a> Roms<'a> {
    fn first(&self, order: &[usize]) -> Source<'a> {
        Source::first([self.falzar, self.gregar, self.falzar_jp, self.gregar_jp], order)
    }

    /// A ROM for what the four have alike.
    pub fn any(&self) -> Source<'a> {
        self.first(&[0, 1, 2, 3])
    }

    /// A US ROM: the English words.
    pub fn us(&self) -> Source<'a> {
        self.first(&[0, 1])
    }

    /// A Japanese ROM: the Japanese words, and what the US release cut.
    pub fn jp(&self) -> Source<'a> {
        self.first(&[2, 3])
    }

    /// A ROM of `version`, for what is the version's own in both releases.
    pub fn of(&self, version: Version) -> Source<'a> {
        match version {
            Version::Falzar => self.first(&[0, 2]),
            Version::Gregar => self.first(&[1, 3]),
        }
    }

    /// A ROM of `version` if one is there, any otherwise: for what the two
    /// versions' ROMs have otherwise at the same place and the pack has one
    /// of.
    pub fn preferring(&self, version: Version) -> Source<'a> {
        let own = self.of(version);
        if own.is_present() { own } else { self.any() }
    }

    /// The US ROM of `version`: the version's own English words.
    pub fn us_of(&self, version: Version) -> Source<'a> {
        match version {
            Version::Falzar => self.first(&[0]),
            Version::Gregar => self.first(&[1]),
        }
    }

    /// The Japanese ROM of `version`.
    pub fn jp_of(&self, version: Version) -> Source<'a> {
        match version {
            Version::Falzar => self.first(&[2]),
            Version::Gregar => self.first(&[3]),
        }
    }
}
