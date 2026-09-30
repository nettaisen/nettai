//! The game data the custom screen reads, behind one trait: chip records,
//! the Program Advances, and the link navis' own chips. Today they come
//! from the engine's tables (`BuiltIn`); a content pack can supply them
//! instead.

use super::folder::FolderChip;
use crate::data::custom::{NAVI_CHIPS, PROGRAM_ADVANCES, ProgramAdvance};
use crate::data::{self, ChipData, ChipId};
use crate::setup::Navi;

/// Game data for the custom screen.
pub trait Library {
    /// A chip's record.
    fn chip(&self, id: ChipId) -> &ChipData;
    /// The Program Advances, in the order they are tried.
    fn program_advances(&self) -> &[ProgramAdvance];
    /// A link navi's own chip, offered once a round (none for MegaMan).
    fn navi_chip(&self, navi: Navi) -> Option<FolderChip>;
}

/// The engine's built-in tables.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuiltIn;

impl Library for BuiltIn {
    fn chip(&self, id: ChipId) -> &ChipData {
        data::chip(id)
    }

    fn program_advances(&self) -> &[ProgramAdvance] {
        &PROGRAM_ADVANCES
    }

    fn navi_chip(&self, navi: Navi) -> Option<FolderChip> {
        let i = (navi.0 as usize).checked_sub(1)?;
        NAVI_CHIPS.get(i).map(|&c| FolderChip::from_packed(c))
    }
}

/// Hand-made chip data for tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::data::{ChipClass, ChipCode, ChipFlags, Element};

    /// A chip record with only what the custom screen reads.
    pub fn chip(class: ChipClass, codes: &'static [ChipCode], flags: u8, damage: u16) -> ChipData {
        ChipData {
            name: "",
            codes,
            element: Element::Null,
            rarity: 0,
            family: 0,
            class,
            mb: 0,
            flags: ChipFlags(flags),
            hit_param: 0,
            action: 0,
            subtype: 0,
            beast_lockon: 0,
            params: 0,
            lockout: 0,
            lib_index: 0,
            flags2: 0,
            lockon_mode: 0,
            sort_key: 0,
            damage,
            library_no: 0,
            slotin_max: 0,
            dark_subst: 0xFF,
        }
    }

    /// A library of made-up chips: `chips` by id (others are plain
    /// standard chips in every code), and these Program Advances.
    pub struct TestLibrary {
        pub chips: Vec<(ChipId, ChipData)>,
        pub program_advances: Vec<ProgramAdvance>,
        pub plain: ChipData,
    }

    pub const EVERY_CODE: &[ChipCode] = &[
        ChipCode(0), ChipCode(1), ChipCode(2), ChipCode(3), ChipCode(4), ChipCode(5), ChipCode(6), ChipCode(7),
        ChipCode(8), ChipCode(9), ChipCode(10), ChipCode(11), ChipCode(12), ChipCode(13), ChipCode(14),
        ChipCode(15), ChipCode(16), ChipCode(17), ChipCode(18), ChipCode(19), ChipCode(20), ChipCode(21),
        ChipCode(22), ChipCode(23), ChipCode(24), ChipCode(25), ChipCode(26),
    ];

    impl TestLibrary {
        pub fn new(chips: Vec<(ChipId, ChipData)>, program_advances: Vec<ProgramAdvance>) -> TestLibrary {
            TestLibrary { chips, program_advances, plain: chip(ChipClass::Standard, EVERY_CODE, 0, 10) }
        }
    }

    impl Library for TestLibrary {
        fn chip(&self, id: ChipId) -> &ChipData {
            self.chips.iter().find(|(i, _)| *i == id).map(|(_, c)| c).unwrap_or(&self.plain)
        }
        fn program_advances(&self) -> &[ProgramAdvance] {
            &self.program_advances
        }
        fn navi_chip(&self, _navi: Navi) -> Option<FolderChip> {
            None
        }
    }
}
