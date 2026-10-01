//! The game data the custom screen reads, behind one trait: chip records,
//! the Program Advances, the link navis' own chips and the screen's slot
//! layout. A battle's [`Content`] supplies them; tests use hand-made
//! libraries.

use super::folder::FolderChip;
use crate::content::{BannerId, ChipData, ChipRole, Content, CustomScreenLayout, ProgramAdvance};
use crate::setup::{Form, Navi};
use bn6_content_api::{ChipHandle, FormHandle, NaviHandle};

/// Game data for the custom screen.
pub trait Library {
    /// A chip's record.
    fn chip(&self, id: ChipHandle) -> &ChipData;
    /// The Beast Out chip the screen offers (`roles.chips.beast_out`), if
    /// the content has one, and the chip an illegal pick counts as
    /// (`roles.chips.invalid`).
    fn beast_out_chip(&self) -> Option<ChipHandle>;
    fn invalid_chip(&self) -> ChipHandle;
    /// A Program Advance's place among them (the bit a formed one takes in
    /// the round's record), by the chip it makes.
    fn advance_index(&self, result: ChipHandle) -> u8;
    /// The navi whose own chip `id` is (a link navi's chip), if any.
    fn own_chip_of(&self, id: ChipHandle) -> Option<NaviHandle>;
    /// A navi's and a form's numbers, and the pack's form with a number
    /// (the screen's numeric logic asks them until phase C).
    fn navi_number(&self, navi: NaviHandle) -> Navi;
    fn form_number(&self, form: FormHandle) -> Form;
    fn form_numbered(&self, form: Form) -> FormHandle;
    /// The Program Advances, in the order they are tried.
    fn program_advances(&self) -> &[ProgramAdvance];
    /// A link navi's own chip, offered once a round (none for MegaMan).
    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip>;
    /// The navi's no-running message: the characters in each of its lines
    /// (a line after the first with none isn't there).
    fn run_message(&self, navi: NaviHandle) -> [u8; 3];
    /// The screen's slot grid and neighbour scans.
    fn layout(&self) -> &CustomScreenLayout;
    /// Whether a banner stays up until let go (the Program Advance's).
    fn banner_holds(&self, id: BannerId) -> bool;
}

impl Library for Content {
    fn chip(&self, id: ChipHandle) -> &ChipData {
        Content::chip(self, id)
    }

    fn beast_out_chip(&self) -> Option<ChipHandle> {
        self.defs.roles.try_chip(ChipRole::BeastOut)
    }

    fn invalid_chip(&self) -> ChipHandle {
        self.defs.roles.chip(ChipRole::Invalid)
    }

    fn advance_index(&self, result: ChipHandle) -> u8 {
        self.chip_links(result).advance.expect("a Program Advance's result")
    }

    fn own_chip_of(&self, id: ChipHandle) -> Option<NaviHandle> {
        self.chip_links(id).own_chip_of
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

    fn program_advances(&self) -> &[ProgramAdvance] {
        Content::program_advances(self)
    }

    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip> {
        let (id, code) = Content::navi_chip(self, navi)?;
        Some(FolderChip { id, code })
    }

    fn run_message(&self, navi: NaviHandle) -> [u8; 3] {
        let lines = &self.navi(navi).run_message;
        std::array::from_fn(|i| lines.get(i).copied().unwrap_or(0))
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
    use crate::content::{ChipClass, ChipCode, ChipFamily, ChipFlags, ChipId, Element, ExtraChipFlags};

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
            description: None,
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
            formula: None,
            traits: Default::default(),
            trap: None,
            hp_bug: 0,
            dark_substitute: None,
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
    /// its number, with BN6's numbering where the screen names a chip by
    /// role: the Beast Out chip 0x13F, the invalid chip 0x185, the Program
    /// Advances from 0x140 and the link navis' own chips from 0x190 (navi
    /// 1's).
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

    /// The chips the screen names by role, as the test library numbers them.
    pub const BEAST_OUT: ChipId = 0x13F;
    pub const INVALID: ChipId = 0x185;
    pub const FIRST_ADVANCE: ChipId = 0x140;
    pub const FIRST_OWN_CHIP: ChipId = 0x190;

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
        fn beast_out_chip(&self) -> Option<ChipHandle> {
            Some(ChipHandle(BEAST_OUT))
        }
        fn invalid_chip(&self) -> ChipHandle {
            ChipHandle(INVALID)
        }
        fn advance_index(&self, result: ChipHandle) -> u8 {
            (result.0 - FIRST_ADVANCE) as u8
        }
        fn own_chip_of(&self, id: ChipHandle) -> Option<NaviHandle> {
            id.0.checked_sub(FIRST_OWN_CHIP - 1).filter(|&n| n >= 1).map(NaviHandle)
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
        fn program_advances(&self) -> &[ProgramAdvance] {
            &self.program_advances
        }
        fn navi_chip(&self, _navi: NaviHandle) -> Option<FolderChip> {
            None
        }
        fn run_message(&self, _navi: NaviHandle) -> [u8; 3] {
            [19, 12, 0]
        }
        fn layout(&self) -> &CustomScreenLayout {
            &self.layout
        }
        fn banner_holds(&self, id: BannerId) -> bool {
            matches!(id.0, 0x24 | 0x34)
        }
    }
}
