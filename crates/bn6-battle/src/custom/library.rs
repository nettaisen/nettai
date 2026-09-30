//! The game data the custom screen reads, behind one trait: chip records,
//! the Program Advances, the link navis' own chips and the screen's slot
//! layout. A battle's [`Content`] supplies them; tests use hand-made
//! libraries.

use super::folder::FolderChip;
use crate::content::{ChipData, ChipId, Content, CustomScreenLayout, ProgramAdvance};
use crate::setup::Navi;

/// Game data for the custom screen.
pub trait Library {
    /// A chip's record.
    fn chip(&self, id: ChipId) -> &ChipData;
    /// The Program Advances, in the order they are tried.
    fn program_advances(&self) -> Vec<ProgramAdvance>;
    /// A link navi's own chip, offered once a round (none for MegaMan).
    fn navi_chip(&self, navi: Navi) -> Option<FolderChip>;
    /// The screen's slot grid and neighbour scans.
    fn layout(&self) -> &CustomScreenLayout;
}

impl Library for Content {
    fn chip(&self, id: ChipId) -> &ChipData {
        Content::chip(self, id)
    }

    fn program_advances(&self) -> Vec<ProgramAdvance> {
        Content::program_advances(self)
    }

    fn navi_chip(&self, navi: Navi) -> Option<FolderChip> {
        Content::navi_chip(self, navi).map(|c| FolderChip { id: c.chip, code: c.code })
    }

    fn layout(&self) -> &CustomScreenLayout {
        &self.rules.custom_screen
    }
}

/// Hand-made chip data for tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::content::{ChipClass, ChipCode, ChipFamily, ChipFlags, Element, ExtraChipFlags};

    /// A chip record with only what the custom screen reads.
    pub fn chip(class: ChipClass, codes: &[ChipCode], flags: u8, damage: u16) -> ChipData {
        ChipData {
            id: 0,
            name: String::new(),
            codes: codes.to_vec(),
            element: Element::Null,
            rarity: 0,
            family: ChipFamily::Fire,
            class,
            mb: 0,
            flags: ChipFlags(flags),
            hit_param: 0,
            action: 0,
            subtype: 0,
            beast_lockon: false,
            params: [0; 4],
            lockout: 0,
            extra_flags: ExtraChipFlags::default(),
            lockon_mode: 0,
            damage,
            library_number: 0,
            library_index: 0,
            sort_key: 0,
            slot_in_limit: 0,
            dark_substitute: None,
            sp_damage: None,
            navi_damage: None,
            modifier: None,
            program_advances: Vec::new(),
            gun_del_sol: None,
            script: None,
        }
    }

    /// A library of made-up chips: `chips` by id (others are plain
    /// standard chips in every code), and these Program Advances, on the
    /// test content's screen layout.
    pub struct TestLibrary {
        pub chips: Vec<(ChipId, ChipData)>,
        pub program_advances: Vec<ProgramAdvance>,
        pub plain: ChipData,
        pub layout: CustomScreenLayout,
    }

    pub const EVERY_CODE: &[ChipCode] = &[
        ChipCode(0), ChipCode(1), ChipCode(2), ChipCode(3), ChipCode(4), ChipCode(5), ChipCode(6), ChipCode(7),
        ChipCode(8), ChipCode(9), ChipCode(10), ChipCode(11), ChipCode(12), ChipCode(13), ChipCode(14),
        ChipCode(15), ChipCode(16), ChipCode(17), ChipCode(18), ChipCode(19), ChipCode(20), ChipCode(21),
        ChipCode(22), ChipCode(23), ChipCode(24), ChipCode(25), ChipCode(26),
    ];

    impl TestLibrary {
        pub fn new(chips: Vec<(ChipId, ChipData)>, program_advances: Vec<ProgramAdvance>) -> TestLibrary {
            TestLibrary {
                chips,
                program_advances,
                plain: chip(ChipClass::Standard, EVERY_CODE, 0, 10),
                layout: crate::content::testing::custom_screen_layout(),
            }
        }
    }

    impl Library for TestLibrary {
        fn chip(&self, id: ChipId) -> &ChipData {
            self.chips.iter().find(|(i, _)| *i == id).map(|(_, c)| c).unwrap_or(&self.plain)
        }
        fn program_advances(&self) -> Vec<ProgramAdvance> {
            self.program_advances.clone()
        }
        fn navi_chip(&self, _navi: Navi) -> Option<FolderChip> {
            None
        }
        fn layout(&self) -> &CustomScreenLayout {
            &self.layout
        }
    }
}
