//! Game content, extracted from the original ROM by `bn6-extract` into
//! typed tables. The files named `*_generated.rs` are written by that tool;
//! regenerate them rather than editing by hand.

mod chips_generated;

pub use chips_generated::CHIPS;

/// A chip code: A-Z are 0-25, `*` is 26.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChipCode(pub u8);

impl ChipCode {
    pub const ASTERISK: ChipCode = ChipCode(26);

    pub fn letter(self) -> char {
        if self.0 == 26 { '*' } else { (b'A' + self.0) as char }
    }
}

/// One battle chip's data record. Fields not yet understood keep their
/// record offset in the name and are kept so nothing is lost.
#[derive(Clone, Copy, Debug)]
pub struct ChipData {
    /// Codes the chip comes in (up to four).
    pub codes: &'static [ChipCode],
    pub element: u8,
    /// 0-4 stars minus one.
    pub rarity: u8,
    pub unk_06: u8,
    pub unk_07: u8,
    /// Folder memory cost.
    pub mb: u8,
    pub unk_09: u8,
    pub unk_0a: u8,
    pub unk_0b: u8,
    pub unk_0c_17: [u8; 12],
    pub unk_18: u16,
    /// Base damage.
    pub damage: u16,
    pub unk_1c: u16,
    pub unk_1e: u8,
}
