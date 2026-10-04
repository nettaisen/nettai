//! The game data the custom screen reads, behind one trait: chip records,
//! the Program Advances, the link navis' own chips and the screen's slot
//! layout. A battle's [`Content`] supplies them; tests use hand-made
//! libraries.

use super::folder::FolderChip;
use super::GameVersion;
use crate::content::{BannerId, ChipData, ChipRole, Content, CustomScreenLayout, FormTraits, ProgramAdvance};
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};

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
    /// Whether a navi changes form (MegaMan): the screen offers it its
    /// Crosses and Beast Out.
    fn changes_form(&self, navi: NaviHandle) -> bool;
    /// A navi's forms in a game: the Cross with this number on the screen,
    /// Beast Out and Beast Over (none: the content has no such form).
    fn cross_form(&self, navi: NaviHandle, version: GameVersion, cross: u8) -> Option<FormHandle>;
    fn beast_out_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle>;
    fn beast_over_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle>;
    /// BN5's Soul Unison: the navi has souls (the custom screen's soul
    /// button), and the soul a chip of `family` given up gives (its number
    /// and form).
    /// The words a custom screen's result takes on the link, a tick each
    /// (the sending side's game's flow: `FlowRules::result_words`).
    fn result_words(&self) -> u32 {
        super::SEND_TICKS
    }
    fn has_souls(&self, _navi: NaviHandle) -> bool {
        false
    }
    fn soul_for_family(&self, _navi: NaviHandle, _family: crate::content::ChipFamily) -> Option<(u8, FormHandle)> {
        None
    }
    /// What the screen asks of a form, and whether it is a soul (BN5's).
    fn form_traits(&self, form: FormHandle) -> FormTraits;
    fn form_is_soul(&self, _form: FormHandle) -> bool {
        false
    }
    /// The Program Advances, in the order they are tried.
    fn program_advances(&self) -> &[ProgramAdvance];
    /// A link navi's own chip, offered once a round (none for MegaMan).
    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip>;
    /// The navi's no-running message: the characters in each of its lines
    /// (a line after the first with none isn't there).
    fn run_message(&self, navi: NaviHandle) -> [u8; 3];
    /// Which of its characters move the speaker's mouth, by line
    /// (`RunMessage::talking`). Presentation.
    fn run_message_talking(&self, _navi: NaviHandle) -> [u32; 3] {
        [0; 3]
    }
    /// The lines of a form's description (`FormData::description_lines`:
    /// BN6's Crosses').
    fn form_description_lines(&self, _form: FormHandle) -> u8 {
        3
    }
    /// The screen's slot grid and neighbor scans.
    fn layout(&self) -> &CustomScreenLayout;
    /// Whether a banner stays up until let go (the Program Advance's).
    fn banner_holds(&self, id: BannerId) -> bool;
    /// The banner of a Program Advance (`made`), or of a selection that
    /// makes none.
    fn program_advance_banner(&self, made: bool) -> BannerId;
}

impl Library for Content {
    fn chip(&self, id: ChipHandle) -> &ChipData {
        Content::chip(self, id)
    }

    // (Content read with no player, by tools: the first game that has
    // what is asked; a player's screen reads its game's, `GameLibrary`.)
    fn beast_out_chip(&self) -> Option<ChipHandle> {
        self.defs.roles.iter().find_map(|r| r.try_chip(ChipRole::BeastOut))
    }

    fn invalid_chip(&self) -> ChipHandle {
        self.defs.roles.iter().find_map(|r| r.try_chip(ChipRole::Invalid)).expect("no game fills the role chips.invalid")
    }

    fn advance_index(&self, result: ChipHandle) -> u8 {
        self.chip_links(result).advance.expect("a Program Advance's result")
    }

    fn own_chip_of(&self, id: ChipHandle) -> Option<NaviHandle> {
        self.chip_links(id).own_chip_of
    }

    fn changes_form(&self, navi: NaviHandle) -> bool {
        self.navi(navi).changes_form()
    }

    fn cross_form(&self, navi: NaviHandle, version: GameVersion, cross: u8) -> Option<FormHandle> {
        self.navi(navi).forms.as_ref()?.of(version).crosses.get(cross as usize).copied()
    }

    fn beast_out_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
        self.navi(navi).forms.as_ref()?.of(version).beast_out
    }

    fn beast_over_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
        self.navi(navi).forms.as_ref()?.of(version).beast_over
    }

    fn has_souls(&self, navi: NaviHandle) -> bool {
        self.navi(navi).forms.as_ref().is_some_and(|f| !f.souls.is_empty())
    }

    fn soul_for_family(&self, navi: NaviHandle, family: crate::content::ChipFamily) -> Option<(u8, FormHandle)> {
        let forms = self.navi(navi).forms.as_ref()?;
        forms.souls.iter().find_map(|&f| self.form(f).soul.filter(|s| s.family == family).map(|s| (s.number, f)))
    }

    fn form_traits(&self, form: FormHandle) -> FormTraits {
        self.form(form).traits
    }

    fn form_is_soul(&self, form: FormHandle) -> bool {
        self.form(form).soul.is_some()
    }

    fn program_advances(&self) -> &[ProgramAdvance] {
        Content::program_advances(self)
    }

    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip> {
        let (id, code) = Content::navi_chip(self, navi)?;
        Some(FolderChip { id, code })
    }

    fn run_message(&self, navi: NaviHandle) -> [u8; 3] {
        let lines = &self.navi(navi).run_message.counts;
        std::array::from_fn(|i| lines.get(i).copied().unwrap_or(0))
    }

    fn run_message_talking(&self, navi: NaviHandle) -> [u32; 3] {
        self.navi(navi).run_message.talking
    }

    fn form_description_lines(&self, form: FormHandle) -> u8 {
        self.form(form).description_lines
    }

    fn layout(&self) -> &CustomScreenLayout {
        let game = self.rules.iter().position(|r| r.custom_screen != CustomScreenLayout::default()).unwrap_or(0);
        &self.rules_of(crate::content::RootId(game as u8)).custom_screen
    }

    fn banner_holds(&self, id: BannerId) -> bool {
        self.rules.iter().any(|r| r.banner_holds(id))
    }

    fn program_advance_banner(&self, made: bool) -> BannerId {
        use crate::content::BannerRole;
        let role = if made { BannerRole::ProgramAdvance } else { BannerRole::ProgramAdvanceEmpty };
        self.defs.roles.iter().find_map(|r| r.banners.get(&role).copied()).expect("no game fills the program advance banners")
    }
}

/// The content as a player's custom screen reads it: its game's data (its
/// layout, its Beast Out and invalid chips, its banners; docs/design/
/// rules-in-luau.md §2.3), the content's records for the rest.
/// (`Library for Content`, tools' and tests' with no player, reads the first
/// game that has it.)
pub struct GameLibrary<'a> {
    pub content: &'a Content,
    pub game: crate::content::RootId,
    /// The side's ruleset, whose own sections (a mix's) the screen reads
    /// over its game's.
    pub ruleset: Option<nettai_content_api::RulesetHandle>,
}

impl Library for GameLibrary<'_> {
    fn result_words(&self) -> u32 {
        self.content.rules_of(self.game).flow.result_words as u32
    }

    fn chip(&self, id: ChipHandle) -> &ChipData {
        self.content.chip(id)
    }

    fn beast_out_chip(&self) -> Option<ChipHandle> {
        self.content.defs.roles(self.game).try_chip(ChipRole::BeastOut)
    }

    fn invalid_chip(&self) -> ChipHandle {
        self.content.defs.roles(self.game).chip(ChipRole::Invalid)
    }

    fn advance_index(&self, result: ChipHandle) -> u8 {
        self.content.advance_index(result)
    }

    fn own_chip_of(&self, id: ChipHandle) -> Option<NaviHandle> {
        self.content.own_chip_of(id)
    }

    fn changes_form(&self, navi: NaviHandle) -> bool {
        self.content.changes_form(navi)
    }

    fn cross_form(&self, navi: NaviHandle, version: GameVersion, cross: u8) -> Option<FormHandle> {
        self.content.cross_form(navi, version, cross)
    }

    fn beast_out_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
        self.content.beast_out_form(navi, version)
    }

    fn beast_over_form(&self, navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
        self.content.beast_over_form(navi, version)
    }

    fn has_souls(&self, navi: NaviHandle) -> bool {
        Library::has_souls(self.content, navi)
    }

    fn soul_for_family(&self, navi: NaviHandle, family: crate::content::ChipFamily) -> Option<(u8, FormHandle)> {
        Library::soul_for_family(self.content, navi, family)
    }

    fn form_traits(&self, form: FormHandle) -> FormTraits {
        self.content.form_traits(form)
    }

    fn form_is_soul(&self, form: FormHandle) -> bool {
        self.content.form_is_soul(form)
    }

    fn program_advances(&self) -> &[ProgramAdvance] {
        self.content.program_advances()
    }

    fn navi_chip(&self, navi: NaviHandle) -> Option<FolderChip> {
        Library::navi_chip(self.content, navi)
    }

    fn run_message(&self, navi: NaviHandle) -> [u8; 3] {
        self.content.run_message(navi)
    }

    fn run_message_talking(&self, navi: NaviHandle) -> [u32; 3] {
        self.content.run_message_talking(navi)
    }

    fn form_description_lines(&self, form: FormHandle) -> u8 {
        self.content.form_description_lines(form)
    }

    fn layout(&self) -> &CustomScreenLayout {
        &self.content.side_rules(self.ruleset, self.game).custom_screen
    }

    fn banner_holds(&self, id: BannerId) -> bool {
        self.content.side_rules(self.ruleset, self.game).banner_holds(id)
    }

    fn program_advance_banner(&self, made: bool) -> BannerId {
        use crate::content::BannerRole;
        self.content.defs.roles(self.game).banner(if made { BannerRole::ProgramAdvance } else { BannerRole::ProgramAdvanceEmpty })
    }
}

/// Hand-made chip data for tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::content::{ChipClass, ChipCode, ChipFamily, ChipFlags, Element, ExtraChipFlags};

    /// A test library's chip: the number its handle is.
    pub type ChipId = u16;

    /// A chip record with only what the custom screen reads.
    pub fn chip(class: ChipClass, codes: &[ChipCode], flags: u8, damage: u16) -> ChipData {
        ChipData {
            description_lines: 3,
            codes: codes.to_vec(),
            element: Element::Null,
            rarity: 0,
            family: ChipFamily::Fire,
            class,
            art_palette: None,
            mb: 0,
            flags: ChipFlags(flags),
            hit_param: 0,
            lockout: 0,
            extra_flags: ExtraChipFlags::default(),
            damage,
            library_number: 0,
            library_index: 0,
            sort_key: 0,
            slot_in_limit: 0,
            formula: None,
            traits: Default::default(),
            trap: None,
            modifier: None,
            program_advances: Vec::new(),
        }
    }

    /// A library of made-up chips: `chips` by number (others are plain
    /// standard chips in every code), and these Program Advances, on the
    /// test content's screen layout. A chip's, navi's or form's handle is
    /// a number of the library's own, with BN6's numbering: the Beast Out
    /// chip 0x13F, the invalid chip 0x185, the Program Advances from 0x140
    /// and the link navis' own chips from 0x190 (navi 1's); navi 0 changes
    /// form; forms 1 to 5 are Gregar's Crosses and 6 to 10 Falzar's (5 and
    /// 10 with ChargeCross's and DustCross's screens), 0xB and 0xC the
    /// Beasts, a Cross's form in Beast Out 0xC past it, 0x17 and 0x18 Beast
    /// Over.
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

    /// The test library's forms the tests name.
    pub const DUST_CROSS: FormHandle = FormHandle(0x0A);
    pub const GREGAR_BEAST: FormHandle = FormHandle(0x0B);
    pub const FALZAR_BEAST: FormHandle = FormHandle(0x0C);

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
        fn changes_form(&self, navi: NaviHandle) -> bool {
            navi.0 == 0
        }
        fn cross_form(&self, _navi: NaviHandle, version: GameVersion, cross: u8) -> Option<FormHandle> {
            Some(FormHandle(cross as u16 + if version == GameVersion::Gregar { 1 } else { 6 }))
        }
        fn beast_out_form(&self, _navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
            Some(if version == GameVersion::Gregar { GREGAR_BEAST } else { FALZAR_BEAST })
        }
        fn beast_over_form(&self, _navi: NaviHandle, version: GameVersion) -> Option<FormHandle> {
            Some(FormHandle(if version == GameVersion::Gregar { 0x17 } else { 0x18 }))
        }
        fn form_traits(&self, _form: FormHandle) -> FormTraits {
            FormTraits::default()
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
        /// Form f's description has f % 3 + 1 lines, so a test can tell
        /// whose description a chatbox shows.
        fn form_description_lines(&self, form: FormHandle) -> u8 {
            (form.0 % 3) as u8 + 1
        }
        fn layout(&self) -> &CustomScreenLayout {
            &self.layout
        }
        fn banner_holds(&self, id: BannerId) -> bool {
            matches!(id.0, 0x24 | 0x34)
        }
        fn program_advance_banner(&self, made: bool) -> BannerId {
            BannerId(if made { 0x24 } else { 0x34 })
        }
    }
}
