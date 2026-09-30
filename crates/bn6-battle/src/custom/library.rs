//! The game data the custom screen reads, behind one trait: chip records,
//! the Program Advances, the link navis' own chips and the screen's slot
//! layout. A battle's [`Content`] supplies them; tests use hand-made
//! libraries.

use super::folder::FolderChip;
use crate::content::{BannerId, ChipData, ChipId, Content, CustomScreenLayout, ProgramAdvance};
use crate::setup::{Form, Navi};
use bn6_content_api::{ChipHandle, FormHandle, NaviHandle};

/// Game data for the custom screen.
pub trait Library {
    /// A chip's record.
    fn chip(&self, id: ChipHandle) -> &ChipData;
    /// A chip's number in the pack's table (the screen's numeric logic
    /// asks it until phase C); none for a chip content defines.
    fn chip_number(&self, id: ChipHandle) -> Option<ChipId>;
    /// The pack's chip with this number.
    fn chip_numbered(&self, id: ChipId) -> Option<ChipHandle>;
    /// A navi's and a form's numbers, and the pack's form with a number
    /// (the screen's numeric logic asks them until phase C).
    fn navi_number(&self, navi: NaviHandle) -> Navi;
    fn form_number(&self, form: FormHandle) -> Form;
    fn form_numbered(&self, form: Form) -> FormHandle;
    /// The Program Advances, in the order they are tried.
    fn program_advances(&self) -> Vec<ProgramAdvance>;
    /// A link navi's own chip, offered once a round (none for MegaMan).
    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip>;
    /// The screen's slot grid and neighbour scans.
    fn layout(&self) -> &CustomScreenLayout;
    /// Whether a banner stays up until let go (the Program Advance's).
    fn banner_holds(&self, id: BannerId) -> bool;
}

impl Library for Content {
    fn chip(&self, id: ChipHandle) -> &ChipData {
        Content::chip(self, id)
    }

    fn chip_number(&self, id: ChipHandle) -> Option<ChipId> {
        Content::chip_number(self, id)
    }

    fn chip_numbered(&self, id: ChipId) -> Option<ChipHandle> {
        Content::chip_numbered(self, id)
    }

    fn navi_number(&self, navi: NaviHandle) -> Navi {
        Content::navi_number(self, navi)
    }

    fn form_number(&self, form: FormHandle) -> Form {
        Content::form_number(self, form)
    }

    fn form_numbered(&self, form: Form) -> FormHandle {
        Content::form_numbered(self, form)
    }

    fn program_advances(&self) -> Vec<ProgramAdvance> {
        Content::program_advances(self)
    }

    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip> {
        let c = Content::navi_chip(self, navi)?;
        let id = self.chip_numbered(c.chip).unwrap_or_else(|| panic!("a navi's own chip is chip {:#x}, which isn't in the content", c.chip));
        Some(FolderChip { id, code: c.code })
    }

    fn layout(&self) -> &CustomScreenLayout {
        &self.rules.custom_screen
    }

    fn banner_holds(&self, id: BannerId) -> bool {
        self.rules.banner_holds(id)
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
            id: None,
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
            recovery: None,
            sword: None,
            script: None,
        }
    }

    /// A library of made-up chips: `chips` by number (others are plain
    /// standard chips in every code), and these Program Advances, on the
    /// test content's screen layout. A chip's, navi's or form's handle is
    /// its number.
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
        fn chip(&self, id: ChipHandle) -> &ChipData {
            self.chips.iter().find(|(i, _)| *i == id.0).map(|(_, c)| c).unwrap_or(&self.plain)
        }
        fn chip_number(&self, id: ChipHandle) -> Option<ChipId> {
            Some(id.0)
        }
        fn chip_numbered(&self, id: ChipId) -> Option<ChipHandle> {
            Some(ChipHandle(id))
        }
        fn navi_number(&self, navi: NaviHandle) -> Navi {
            Navi(navi.0 as u8)
        }
        fn form_number(&self, form: FormHandle) -> Form {
            Form(form.0 as u8)
        }
        fn form_numbered(&self, form: Form) -> FormHandle {
            FormHandle(form.0 as u16)
        }
        fn program_advances(&self) -> Vec<ProgramAdvance> {
            self.program_advances.clone()
        }
        fn navi_chip(&self, _navi: NaviHandle) -> Option<FolderChip> {
            None
        }
        fn layout(&self) -> &CustomScreenLayout {
            &self.layout
        }
        fn banner_holds(&self, id: BannerId) -> bool {
            matches!(id.0, 0x24 | 0x34)
        }
    }
}
