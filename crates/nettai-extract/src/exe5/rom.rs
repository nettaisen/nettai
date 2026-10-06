pub(crate) use crate::rom::Rom;

/// A ROM's version: Team ProtoMan's or Team Colonel's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    ProtoMan,
    Colonel,
}

impl Version {
    /// The suffix of an asset that differs by version (`-protoman`,
    /// `-colonel`), as EXE6's packs name theirs `-falzar`/`-gregar`.
    pub fn name(self) -> &'static str {
        match self {
            Version::ProtoMan => "protoman",
            Version::Colonel => "colonel",
        }
    }
}

/// The four ROMs: the US Team ProtoMan (`MEGAMAN5_TP_`, BRBE) and Team
/// Colonel (`MEGAMAN5_TC_`, BRKE), the Japanese Team of Blues
/// (`ROCKEXE5_TOB`, BRBJ) and Team of Colonel (`ROCKEXE5_TOC`, BRKJ).
pub struct Roms<'a> {
    pub protoman: &'a Rom,
    pub colonel: &'a Rom,
    pub protoman_jp: &'a Rom,
    pub colonel_jp: &'a Rom,
}

impl Roms<'_> {
    pub fn us(&self, v: Version) -> &Rom {
        match v {
            Version::ProtoMan => self.protoman,
            Version::Colonel => self.colonel,
        }
    }

    pub fn jp(&self, v: Version) -> &Rom {
        match v {
            Version::ProtoMan => self.protoman_jp,
            Version::Colonel => self.colonel_jp,
        }
    }
}
