pub(crate) use crate::rom::{Place, Rom, Roms, Source};

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

// The four ROMs (`Roms`, in this order): the US Team ProtoMan
// (`MEGAMAN5_TP_`, BRBE) and Team Colonel (`MEGAMAN5_TC_`, BRKE), the
// Japanese Team of Blues (`ROCKEXE5_TOB`, BRBJ) and Team of Colonel
// (`ROCKEXE5_TOC`, BRKJ). Team ProtoMan's is the base version.
impl crate::rom::Version for Version {
    fn is_other(self) -> bool {
        self == Version::Colonel
    }
}
