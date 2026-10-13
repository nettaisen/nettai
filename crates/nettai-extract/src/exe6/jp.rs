//! What the pack takes from the Japanese ROMs (`ROCKEXE6_RXXBR6J`, JP
//! Falzar, and `ROCKEXE6_GXXBR5J`, JP Gregar): what the US release cut
//! and left a placeholder in (docs/engine/jp-differences.md §4.4, §4.5).
//! The same code reaches it as in the US ROMs (the sprite list, the chip
//! records: the other modules' places); here is which of it the pack
//! takes from a Japanese ROM, either of them unless stated.
//!
//! - **Sprites**: six slots of the sprite list hold the US ROMs' shared
//!   placeholder archive and the Japanese ROMs' own (the same archive in
//!   both): Count (HackJack's navi), Django, Otenko's statue, the Falzar
//!   and Gregar chips' beasts, and battle mode 1's "Blocking" label.
//! - **Chip pictures**: fifteen chips have a placeholder picture (a purple
//!   block) in the US ROMs: the eleven the US release cut, and BatCan1 to
//!   BatCan4, which it kept the names of. The Japanese ROMs have their
//!   art.
//! - **The chip records otherwise**: a chip's icon and picture are its US
//!   ROM's where there is one (`gregar::chip_source`). Two differ in the
//!   Japanese ROMs: the Falzar ROMs' icon for Bass and BassAnly (one
//!   pixel, (0, 8): color 3 in the Japanese ROM, 9 in the US one) and the
//!   Gregar ROMs' Beast Out picture (two pixels, (53, 31) and (52, 32):
//!   clear in the Japanese ROM, color 14 in the US one).
//!
//! The pack has one picture of each: nettai draws it on every console. A US
//! console shows the placeholder there, which the verification's frame
//! comparison knows (its table of what a console shows otherwise); the
//! pack keeps no mark of whose ROM a picture is from. Everything else is
//! the US ROMs' where the releases differ, and the Japanese ROMs' in a pack
//! of no US ROM.

use crate::exe6::hud::CHIPS;
use crate::exe6::{Rom, Roms, Version, u32at};

/// The sprite slots (category's byte offset, index) the US ROMs fill with
/// a placeholder archive and the Japanese ROMs with their own, which the
/// pack takes from a Japanese ROM.
pub const SPRITES: [(u8, u8); 6] = [
    // Count (HackJack's navi, and his lances).
    (0x08, 0x16),
    // Django: the Django chips' navi and bike, CrosOver's partner and his
    // gun.
    (0x0C, 0x0F),
    // Otenko's statue (in the US ROMs the statue loads 0C-00 instead), and
    // how it looks absorbed and thrown as junk.
    (0x0C, 0x49),
    // The Falzar and Gregar chips' beasts.
    (0x0C, 0x66),
    (0x0C, 0x68),
    // The effect table's entry 0x1D in the Japanese ROMs: battle mode 1's
    // "Blocking" label, which no netbattle shows.
    (0x14, 0x17),
];

/// The chips whose picture each Japanese ROM has its own of: the Gregar
/// and Falzar chips, which show the console's own beast.
pub const VERSIONED: [(u32, Version); 2] = [(0x138, Version::Gregar), (0x139, Version::Falzar)];

/// The chips whose picture is a placeholder in the US ROMs: GunDelEX,
/// Otenko, HackJack (Count) ×3, Django ×3, DblBeast, the Gregar and Falzar
/// chips, and BatCan1 to BatCan4 (each the same in both Japanese ROMs but
/// `VERSIONED`'s). A Japanese console shows its own beast in both the
/// Gregar and the Falzar chip (its ROM has one picture for both); the
/// pack has each chip's own, Gregar's from the Japanese Gregar ROM,
/// Falzar's from the Japanese Falzar ROM, on either console.
pub const CHIP_PICTURES: [u32; 15] =
    [0x012, 0x099, 0x113, 0x114, 0x115, 0x116, 0x117, 0x118, 0x137, 0x138, 0x139, 0x13B, 0x13C, 0x13D, 0x13E];

/// Where DblBeast's record says its palette is: EWRAM, which a gift
/// received over the link fills (an e-Reader card's, Card e+: `0x0813006C`
/// copies the card's palette to 0x02000AF0 + 32 × slot, which the save
/// keeps). That palette, the card's, sits orphaned in the Japanese ROMs
/// among the chip palettes, in DblBeast's place: after the palette of the
/// chip before it. The Gregar and Falzar chips' palettes (slot 1,
/// 0x02000B10) are in no ROM: their definitions give them (`art_palette`),
/// and the pack's pictures have a black one.
const GIFT_PALETTE: u32 = 0x0200_0AF0;

/// A cut chip's picture: the Japanese ROM it is read from (the chip's own
/// version's for `VERSIONED`'s, either for the rest), the picture's
/// address there and its palette's (`None`: in no ROM). `None` for a chip
/// that isn't cut, and for one no ROM given has.
pub fn chip_picture<'a>(roms: &Roms<'a>, id: u32) -> Option<(&'a Rom, u32, Option<u32>)> {
    if !CHIP_PICTURES.contains(&id) {
        return None;
    }
    let src = match VERSIONED.iter().find(|(chip, _)| *chip == id) {
        Some(&(_, version)) => roms.jp_of(version),
        None => roms.jp(),
    };
    if !src.is_present() {
        return None;
    }
    let rom = src.rom;
    let record = |chip: u32| src.at(CHIPS) + 0x2C * chip;
    let (picture, palette) = (u32at(rom, record(id) + 0x24), u32at(rom, record(id) + 0x28));
    let palette = match palette {
        GIFT_PALETTE => Some(u32at(rom, record(id - 1) + 0x28) + 0x20),
        p if (0x0800_0000..0x0A00_0000).contains(&p) => Some(p),
        _ => None,
    };
    Some((rom, picture, palette))
}
