pub(crate) use crate::rom::Rom;

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

/// Where a ROM has what the pack takes: Red Sun US's addresses and their
/// counterparts in the other three (the same literal of the same routine:
/// the verification workspace's target/exe4/romdata-<CODE>.tsv).
pub struct Addresses {
    /// The sprite list (EXE6 `SpritePointersList`): per category, a table
    /// of sprite archive pointers.
    pub sprite_list: u32,
    /// The chip records (0x2C bytes: +0x20 the icon, +0x24 the picture,
    /// +0x28 its palette).
    pub chip_table: u32,
    /// A pointer to the chip icons' palette (Tango's).
    pub icon_palette_pointer: u32,
    /// The 8x16 font (0x40 bytes a glyph).
    pub font: u32,
    /// The dialogue font (16x12, 0x60 bytes a glyph) and its advances (a
    /// word a glyph, after the word its routine's literal points at).
    pub dialogue_font: u32,
    pub dialogue_advances: u32,
    /// The HUD's text lines (EXE6 `TextScript86F0374`'s counterpart).
    pub texts: u32,
}

/// Red Sun US (`MEGAMANBN4RS`, B4WE).
pub const RED_SUN: Addresses = Addresses {
    sprite_list: 0x0802_793C,
    chip_table: 0x0801_97EC,
    icon_palette_pointer: 0x0801_5A78,
    font: 0x0868_DF5C,
    dialogue_font: 0x0869_4F5C,
    dialogue_advances: 0x0805_15E0,
    texts: 0x0874_91CC,
};

/// Blue Moon US (`MEGAMANBN4BM`, B4BE).
pub const BLUE_MOON: Addresses = Addresses {
    sprite_list: 0x0802_7940,
    chip_table: 0x0801_97EC,
    icon_palette_pointer: 0x0801_5A78,
    font: 0x0868_DD44,
    dialogue_font: 0x0869_4D44,
    dialogue_advances: 0x0805_15EC,
    texts: 0x0874_8CC0,
};

/// Red Sun Japan (`ROCK_EXE4_RS`, B4WJ).
pub const RED_SUN_JP: Addresses = Addresses {
    sprite_list: 0x0802_7890,
    chip_table: 0x0801_972C,
    icon_palette_pointer: 0x0801_59D4,
    font: 0x0868_E224,
    dialogue_font: 0x0869_5224,
    dialogue_advances: 0x0805_15B8,
    texts: 0x0874_6E04,
};

/// Blue Moon Japan (`ROCK_EXE4_BM`, B4BJ).
pub const BLUE_MOON_JP: Addresses = Addresses {
    sprite_list: 0x0802_7894,
    chip_table: 0x0801_972C,
    icon_palette_pointer: 0x0801_59D4,
    font: 0x0868_E004,
    dialogue_font: 0x0869_5004,
    dialogue_advances: 0x0805_15C4,
    texts: 0x0874_6940,
};

/// The four ROMs.
pub struct Roms<'a> {
    pub redsun: &'a Rom,
    pub bluemoon: &'a Rom,
    pub redsun_jp: &'a Rom,
    pub bluemoon_jp: &'a Rom,
}

impl<'a> Roms<'a> {
    /// The US ROMs present, each with its addresses: Red Sun's first.
    pub fn us(&self) -> Option<(&'a Rom, &'static Addresses)> {
        [(self.redsun, &RED_SUN), (self.bluemoon, &BLUE_MOON)].into_iter().find(|(r, _)| r.is_present())
    }

    /// The Japanese ROMs present, likewise.
    pub fn jp(&self) -> Option<(&'a Rom, &'static Addresses)> {
        [(self.redsun_jp, &RED_SUN_JP), (self.bluemoon_jp, &BLUE_MOON_JP)].into_iter().find(|(r, _)| r.is_present())
    }

    /// A version's ROM present (US first), else any present.
    pub fn of(&self, v: Version) -> Option<(&'a Rom, &'static Addresses)> {
        let own = match v {
            Version::RedSun => [(self.redsun, &RED_SUN), (self.redsun_jp, &RED_SUN_JP)],
            Version::BlueMoon => [(self.bluemoon, &BLUE_MOON), (self.bluemoon_jp, &BLUE_MOON_JP)],
        };
        own.into_iter().find(|(r, _)| r.is_present()).or_else(|| self.any())
    }

    /// Any ROM present: Red Sun US's first.
    pub fn any(&self) -> Option<(&'a Rom, &'static Addresses)> {
        self.us().or_else(|| self.jp())
    }
}
