//! Custom-screen data from the original game: the slot grid a screen
//! starts from (`dword_802A7CC`) and the lists the neighbour fix-up scans
//! (`sub_8027F42`), which are the screen's layout; and the Program
//! Advances (`off_802BCB0`) and the link navis' own chips
//! (`word_802A828`), which are game content (a content pack will supply
//! them; `custom::Library` reads them).

use super::ChipId;

/// What a slot of the starting grid holds before the chips are dealt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TemplateSlot {
    /// A place for a dealt chip (empty unless one is dealt).
    ChipPosition,
    /// The OK button.
    Ok,
    /// Nothing, unless a special button is put there.
    Hidden,
}

/// A slot of the starting grid and its neighbours (slot numbers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SlotLayout {
    pub kind: TemplateSlot,
    /// UP and DOWN both go here.
    pub vertical: u8,
    pub left: u8,
    pub right: u8,
}

/// How a Program Advance is spelled in a selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaRecipe {
    /// `count` of one chip with consecutive codes (`sub_80295C8`).
    CodeRun { chip: ChipId, count: u8 },
    /// These chips in this order, whatever their codes (`sub_802961A`).
    Sequence(&'static [ChipId]),
}

/// A Program Advance: the chip a recipe turns into.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProgramAdvance {
    pub result: ChipId,
    pub recipe: PaRecipe,
}

impl PaRecipe {
    /// How many chips the recipe takes.
    pub fn len(&self) -> usize {
        match self {
            PaRecipe::CodeRun { count, .. } => *count as usize,
            PaRecipe::Sequence(ids) => ids.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The slot grid every custom screen starts from: 0-4 the top row, 5-9 the bottom row,
/// 10 OK, 11 the special button under it.
pub static SLOT_TEMPLATE: [SlotLayout; 12] = [
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 5, left: 10, right: 1 }, // 0
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 6, left: 0, right: 2 }, // 1
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 7, left: 1, right: 3 }, // 2
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 8, left: 2, right: 4 }, // 3
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 9, left: 3, right: 10 }, // 4
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 0, left: 11, right: 6 }, // 5
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 1, left: 5, right: 7 }, // 6
    SlotLayout { kind: TemplateSlot::ChipPosition, vertical: 2, left: 6, right: 8 }, // 7
    SlotLayout { kind: TemplateSlot::Hidden, vertical: 3, left: 7, right: 9 }, // 8
    SlotLayout { kind: TemplateSlot::Hidden, vertical: 4, left: 8, right: 11 }, // 9
    SlotLayout { kind: TemplateSlot::Ok, vertical: 11, left: 4, right: 0 }, // 10
    SlotLayout { kind: TemplateSlot::Hidden, vertical: 10, left: 9, right: 5 }, // 11
];

/// Where the next slot to the left is looked for, top row and OK.
pub static LEFT_SCAN_TOP: [u8; 8] = [4, 9, 3, 8, 2, 1, 0, 10];
/// Where the next slot to the left is looked for, bottom row and slot 11.
pub static LEFT_SCAN_BOTTOM: [u8; 19] = [9, 8, 7, 6, 5, 11, 10, 7, 6, 5, 2, 1, 0, 11, 10, 6, 5, 11, 10];
/// Where the next slot to the right is looked for, top row and OK.
pub static RIGHT_SCAN_TOP: [u8; 8] = [0, 1, 2, 3, 8, 4, 9, 10];
/// Where the next slot to the right is looked for, bottom row and slot 11.
pub static RIGHT_SCAN_BOTTOM: [u8; 7] = [5, 6, 7, 8, 9, 11, 10];
/// By slot: where its left scan starts in its list.
pub static LEFT_SCAN_START: [u8; 12] = [7, 6, 5, 4, 2, 17, 16, 15, 7, 1, 0, 0];
/// By slot: where its right scan starts in its list.
pub static RIGHT_SCAN_START: [u8; 12] = [1, 2, 3, 5, 7, 1, 2, 3, 4, 5, 0, 0];

/// Each link navi's own chip (`code << 9 | id`), by navi (1-11).
pub static NAVI_CHIPS: [u16; 11] = [0x0f90, 0x0991, 0x2592, 0x1593, 0x0594, 0x0195, 0x2796, 0x2797, 0x0d98, 0x0799, 0x039a];

/// The Program Advances, in the order they are tried.
pub static PROGRAM_ADVANCES: [ProgramAdvance; 43] = [
    ProgramAdvance { result: 0x153, recipe: PaRecipe::Sequence(&[0x047, 0x048, 0x049]) }, // LifeSrd
    ProgramAdvance { result: 0x153, recipe: PaRecipe::Sequence(&[0x047, 0x04a, 0x04b]) }, // LifeSrd
    ProgramAdvance { result: 0x156, recipe: PaRecipe::Sequence(&[0x046, 0x046, 0x098]) }, // PoisPhar
    ProgramAdvance { result: 0x157, recipe: PaRecipe::Sequence(&[0x0ba, 0x0bc, 0x0bb]) }, // BodyGrd
    ProgramAdvance { result: 0x14e, recipe: PaRecipe::Sequence(&[0x022, 0x023, 0x024]) }, // DestPuls
    ProgramAdvance { result: 0x158, recipe: PaRecipe::Sequence(&[0x04a, 0x04b, 0x0e2]) }, // DblHero
    ProgramAdvance { result: 0x159, recipe: PaRecipe::Sequence(&[0x096, 0x096, 0x12d]) }, // Darkness
    ProgramAdvance { result: 0x159, recipe: PaRecipe::Sequence(&[0x096, 0x096, 0x132]) }, // Darkness
    ProgramAdvance { result: 0x15b, recipe: PaRecipe::Sequence(&[0x08b, 0x0c3, 0x0b9]) }, // SunMoon
    ProgramAdvance { result: 0x15c, recipe: PaRecipe::Sequence(&[0x0e2, 0x0ba, 0x110]) }, // TwinLdrs
    ProgramAdvance { result: 0x15c, recipe: PaRecipe::Sequence(&[0x112, 0x0ba, 0x0e0]) }, // TwinLdrs
    ProgramAdvance { result: 0x15d, recipe: PaRecipe::Sequence(&[0x116, 0x117, 0x118]) }, // CrosOver
    ProgramAdvance { result: 0x15a, recipe: PaRecipe::Sequence(&[0x06d, 0x03f, 0x024, 0x027]) }, // MstrCros
    ProgramAdvance { result: 0x140, recipe: PaRecipe::CodeRun { chip: 0x001, count: 3 } }, // GigaCan1
    ProgramAdvance { result: 0x141, recipe: PaRecipe::CodeRun { chip: 0x002, count: 3 } }, // GigaCan2
    ProgramAdvance { result: 0x142, recipe: PaRecipe::CodeRun { chip: 0x003, count: 3 } }, // GigaCan3
    ProgramAdvance { result: 0x151, recipe: PaRecipe::CodeRun { chip: 0x017, count: 3 } }, // SuprSpr
    ProgramAdvance { result: 0x143, recipe: PaRecipe::CodeRun { chip: 0x014, count: 3 } }, // WideBrn1
    ProgramAdvance { result: 0x144, recipe: PaRecipe::CodeRun { chip: 0x015, count: 3 } }, // WideBrn2
    ProgramAdvance { result: 0x145, recipe: PaRecipe::CodeRun { chip: 0x016, count: 3 } }, // WideBrn3
    ProgramAdvance { result: 0x146, recipe: PaRecipe::CodeRun { chip: 0x06b, count: 3 } }, // FlmHook1
    ProgramAdvance { result: 0x147, recipe: PaRecipe::CodeRun { chip: 0x06c, count: 3 } }, // FlmHook2
    ProgramAdvance { result: 0x148, recipe: PaRecipe::CodeRun { chip: 0x06d, count: 3 } }, // FlmHook3
    ProgramAdvance { result: 0x152, recipe: PaRecipe::CodeRun { chip: 0x009, count: 3 } }, // H-Burst
    ProgramAdvance { result: 0x152, recipe: PaRecipe::CodeRun { chip: 0x00a, count: 3 } }, // H-Burst
    ProgramAdvance { result: 0x152, recipe: PaRecipe::CodeRun { chip: 0x00b, count: 3 } }, // H-Burst
    ProgramAdvance { result: 0x155, recipe: PaRecipe::CodeRun { chip: 0x032, count: 3 } }, // PitHocky
    ProgramAdvance { result: 0x154, recipe: PaRecipe::CodeRun { chip: 0x013, count: 3 } }, // GreatYo
    ProgramAdvance { result: 0x14f, recipe: PaRecipe::CodeRun { chip: 0x090, count: 3 } }, // TimeBom+
    ProgramAdvance { result: 0x14f, recipe: PaRecipe::CodeRun { chip: 0x0c8, count: 3 } }, // TimeBom+
    ProgramAdvance { result: 0x14f, recipe: PaRecipe::CodeRun { chip: 0x0c9, count: 3 } }, // TimeBom+
    ProgramAdvance { result: 0x149, recipe: PaRecipe::CodeRun { chip: 0x05c, count: 3 } }, // PwrWave1
    ProgramAdvance { result: 0x14a, recipe: PaRecipe::CodeRun { chip: 0x05d, count: 3 } }, // PwrWave2
    ProgramAdvance { result: 0x14b, recipe: PaRecipe::CodeRun { chip: 0x05e, count: 3 } }, // PwrWave3
    ProgramAdvance { result: 0x14c, recipe: PaRecipe::CodeRun { chip: 0x040, count: 3 } }, // CornFsta
    ProgramAdvance { result: 0x14c, recipe: PaRecipe::CodeRun { chip: 0x041, count: 3 } }, // CornFsta
    ProgramAdvance { result: 0x14c, recipe: PaRecipe::CodeRun { chip: 0x042, count: 3 } }, // CornFsta
    ProgramAdvance { result: 0x14d, recipe: PaRecipe::CodeRun { chip: 0x07b, count: 3 } }, // ParaShl
    ProgramAdvance { result: 0x14d, recipe: PaRecipe::CodeRun { chip: 0x07c, count: 3 } }, // ParaShl
    ProgramAdvance { result: 0x14d, recipe: PaRecipe::CodeRun { chip: 0x07d, count: 3 } }, // ParaShl
    ProgramAdvance { result: 0x150, recipe: PaRecipe::CodeRun { chip: 0x05f, count: 3 } }, // StreamHd
    ProgramAdvance { result: 0x150, recipe: PaRecipe::CodeRun { chip: 0x060, count: 3 } }, // StreamHd
    ProgramAdvance { result: 0x150, recipe: PaRecipe::CodeRun { chip: 0x061, count: 3 } }, // StreamHd
];
