//! What the custom screen reads: its slot layout (a rule), the Program
//! Advances (each with the chip it makes) and the link navis' own chips
//! (each with its navi).

use super::ChipCode;
use bn6_content_api::ChipHandle;
use serde::{Deserialize, Serialize};

/// What a slot of the starting grid holds before the chips are dealt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSlot {
    /// A place for a dealt chip (empty unless one is dealt).
    ChipPosition,
    /// The OK button.
    Ok,
    /// Nothing, unless a special button is put there.
    Hidden,
}

/// A slot of the starting grid and its neighbours (slot numbers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotLayout {
    pub kind: TemplateSlot,
    /// UP and DOWN both go here.
    pub vertical: u8,
    pub left: u8,
    pub right: u8,
}

/// The custom screen's slot grid (`dword_802A7CC`) and the lists its
/// neighbour fix-up scans (`sub_8027F42`): slots 0-4 are the top row,
/// 5-9 the bottom row, 10 OK and 11 the special button under it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CustomScreenLayout {
    pub slots: [SlotLayout; 12],
    /// Where the next slot to the left is looked for: from the top row
    /// and OK, and from the bottom row and slot 11.
    pub left_scan_top: Vec<u8>,
    pub left_scan_bottom: Vec<u8>,
    /// The same to the right.
    pub right_scan_top: Vec<u8>,
    pub right_scan_bottom: Vec<u8>,
    /// By slot: where its left and right scans start in their lists.
    pub left_scan_start: [u8; 12],
    pub right_scan_start: [u8; 12],
}

impl Default for CustomScreenLayout {
    fn default() -> CustomScreenLayout {
        let slot = SlotLayout { kind: TemplateSlot::Hidden, vertical: 0, left: 0, right: 0 };
        CustomScreenLayout {
            slots: [slot; 12],
            left_scan_top: Vec::new(),
            left_scan_bottom: Vec::new(),
            right_scan_top: Vec::new(),
            right_scan_bottom: Vec::new(),
            left_scan_start: [0; 12],
            right_scan_start: [0; 12],
        }
    }
}

/// How a Program Advance is spelled in a selection.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaRecipe {
    /// `count` of one chip (by key) with consecutive codes (`sub_80295C8`).
    CodeRun { chip: String, count: u8 },
    /// These chips (by key) in this order, whatever their codes
    /// (`sub_802961A`).
    Sequence(Vec<String>),
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

/// A Program Advance: the chip a recipe turns into, and the recipe, by
/// chip handles.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProgramAdvance {
    pub result: ChipHandle,
    pub recipe: Recipe,
}

/// A recipe by chip handles (see [`PaRecipe`]).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Recipe {
    /// `count` of one chip with consecutive codes.
    CodeRun { chip: ChipHandle, count: u8 },
    /// These chips in this order, whatever their codes.
    Sequence(Vec<ChipHandle>),
}

impl Recipe {
    /// How many chips the recipe takes.
    pub fn len(&self) -> usize {
        match self {
            Recipe::CodeRun { count, .. } => *count as usize,
            Recipe::Sequence(ids) => ids.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A recipe for the chip that holds it. In a content file,
/// `{ order = N, sequence = [...] }` or `{ order = N, code_run = { chip, count } }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProgramAdvanceRecipe {
    /// Where the recipe is tried among all the Program Advances (the
    /// original's table order): the first that matches wins.
    pub order: u8,
    #[serde(flatten)]
    pub recipe: PaRecipe,
}

/// What a modifier chip does to the chip picked before it (`sub_8029224`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipModifier {
    /// Adds its damage to a damaging chip's attack bonus (Atk+10, Atk+30).
    AttackPlus,
    /// Adds its damage to a navi chip's attack bonus (Navi+20).
    NaviPlus,
    /// Makes a damaging chip paralyze (WhiCapsl).
    Paralyze,
    /// Makes a damaging chip that isn't a dimming chip uninstall (Uninstll).
    Uninstall,
}

/// A chip (by key) with its code (a link navi's own chip).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodedChip {
    pub chip: String,
    pub code: ChipCode,
}
