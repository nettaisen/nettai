//! What the pack takes from the Japanese ROMs (`ROCKEXE6_RXXBR6J`, JP
//! Falzar, and `ROCKEXE6_GXXBR5J`, JP Gregar): what the US release cut
//! and left a placeholder in (docs/engine/jp-differences.md §4.4, §4.5).
//! Its code and data moved against the US ROMs', so the addresses here
//! are the Japanese ROMs' own (both have these tables at the same
//! places).
//!
//! - **Sprites**: six slots of the sprite list hold the US ROMs' shared
//!   placeholder archive and the Japanese ROMs' own (the same archive in
//!   both): Count (HackJack's navi), Django, Otenko's statue, the Falzar
//!   and Gregar chips' beasts, and battle mode 1's "Blocking" label.
//! - **Chip pictures**: eleven chips the US release cut have a placeholder
//!   picture (a purple block) in the US ROMs; the Japanese ROMs have their
//!   art. Their icons are the same in all four ROMs, so the US ones stay.
//!
//! Each asset taken from the Japanese ROMs is marked with the region it is
//! from (nettai-assets `SpriteSheet::region`, `ChipArt::region`): a US
//! console shows the placeholder there, a difference the frame comparison
//! knows.

use crate::exe6::{Rom, u32at};

/// The region the Japanese ROMs' assets are marked with.
pub const REGION: &str = "jp";

/// The Japanese ROMs' sprite list (`SpritePointersList`; the US ROMs' is
/// at 0x08031CC4).
pub const SPRITE_LIST: u32 = 0x0803_2C80;

/// The sprite slots (category's byte offset, index) the US ROMs fill with
/// a placeholder archive and the Japanese ROMs with their own, which the
/// pack takes from the Japanese Falzar ROM.
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

/// The Japanese ROMs' chip records (the table of the chip accessor at 0x08021EB8; the US ROMs'
/// is at 0x08021DA8): 0x2C bytes a chip, +0x24 its picture, +0x28 the
/// picture's palette.
pub const CHIP_DATA: u32 = 0x0802_21BC;

/// Which ROM a cut chip's picture comes from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Falzar,
    Gregar,
}

/// The chips whose picture each Japanese ROM has its own of: the Gregar
/// and Falzar chips, which show the console's own beast.
pub const VERSIONED: [u32; 2] = [0x138, 0x139];

/// The chips whose picture is a placeholder in the US ROMs, and the
/// Japanese ROM each picture comes from: GunDelEX, Otenko, HackJack (Count)
/// ×3, Django ×3, DblBeast, and the Gregar and Falzar chips. A Japanese
/// console shows its own beast in both the Gregar and the Falzar chip
/// (its ROM has one picture for both); the pack has each chip's own,
/// Gregar's from the Japanese Gregar ROM, Falzar's from the Japanese
/// Falzar ROM, on either console.
pub const CHIP_PICTURES: [(u32, Source); 11] = [
    (0x012, Source::Falzar),
    (0x099, Source::Falzar),
    (0x113, Source::Falzar),
    (0x114, Source::Falzar),
    (0x115, Source::Falzar),
    (0x116, Source::Falzar),
    (0x117, Source::Falzar),
    (0x118, Source::Falzar),
    (0x137, Source::Falzar),
    (0x138, Source::Gregar),
    (0x139, Source::Falzar),
];

/// DblBeast's palette. Its record points at EWRAM (0x02000AF0), which a
/// gift received over the link fills (an e-Reader card's, Card e+:
/// `0x0813006C` copies the card's palette to 0x02000AF0 + 32 × slot, which
/// the save keeps); this palette, the card's, sits orphaned in the
/// Japanese Falzar ROM among the chip palettes, in DblBeast's place. The
/// Gregar and Falzar chips' palettes (slot 1, 0x02000B10) are in no ROM:
/// their definitions give them (`art_palette`), and the pack's pictures
/// have a black one.
pub const DBLBEAST_PALETTE: u32 = 0x0874_9C78;

/// A cut chip's picture: the Japanese ROM `CHIP_PICTURES` names, the
/// picture's address there, its palette's (`None`: in no ROM), and the
/// game version it is of (`VERSIONED`'s); `None` for a chip that isn't
/// cut.
pub fn chip_picture<'a>(
    roms: &'a crate::exe6::Roms,
    id: u32,
) -> Option<(&'a Rom, u32, Option<u32>, Option<&'static str>)> {
    let &(_, source) = CHIP_PICTURES.iter().find(|(c, _)| *c == id)?;
    let (mut rom, version) = match source {
        Source::Falzar => (&roms.falzar_jp, "falzar"),
        Source::Gregar => (&roms.gregar_jp, "gregar"),
    };
    if !rom.is_present() && !VERSIONED.contains(&id) {
        rom = [&roms.falzar_jp, &roms.gregar_jp]
            .into_iter()
            .find(|r| r.is_present())?;
    }
    if !rom.is_present() {
        return None;
    }
    let record = CHIP_DATA + 0x2C * id;
    let (picture, palette) = (u32at(rom, record + 0x24), u32at(rom, record + 0x28));
    let palette = match palette {
        0x0200_0AF0 if std::ptr::eq(*rom, roms.falzar_jp) => Some(DBLBEAST_PALETTE),
        p if (0x0800_0000..0x0A00_0000).contains(&p) => Some(p),
        _ => None,
    };
    Some((
        rom,
        picture,
        palette,
        VERSIONED.contains(&id).then_some(version),
    ))
}
