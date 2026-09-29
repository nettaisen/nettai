//! Beast Out lock-on data: the row type of `lockon_generated` and typed
//! lookups.

use super::PanelOffset;
use super::lockon_generated as tables;

/// A lock-on mode that looks for a panel near the target to attack from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockonSearch {
    /// The chip's lock-on mode (`ChipData::lockon_mode`).
    pub mode: u8,
    /// Panels tried, relative to the target, dx toward the user's front.
    pub offsets: &'static [PanelOffset],
    /// Afterwards, the middle row of the chosen column is taken if free.
    pub prefers_middle_row: bool,
}

/// The search lock-on `mode` does (None for the modes that do something
/// else).
pub fn search(mode: u8) -> Option<&'static LockonSearch> {
    tables::SEARCHES.iter().find(|s| s.mode == mode)
}

/// Column shifts toward the user tried, in order, when no panel next to
/// the target fits.
pub fn column_shifts() -> &'static [i8] {
    &tables::COLUMN_SHIFTS
}
