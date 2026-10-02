//! What the pack takes from the US Gregar ROM (`MEGAMAN6_GXXBR5E`). The
//! code is at the same places in both US ROMs;
//! the data it points at moved, so its addresses here are read off the same
//! code's literal pools.
//!
//! - Gregar's own faces: the Falzar ROM has none for Gregar's Crosses, its
//!   Beast and its link navis (its tables show the Falzar counterpart's).
//!   The pack has them as faces of their own, numbered after Falzar's, and
//!   Gregar's forms and navis name them: every form shows its true face, on
//!   either console.
//! - Five chips' pictures and icons, which the Falzar ROM has wrong (the
//!   other five Giga chips' copied in): Gregar's are right on either
//!   console.
//! - What a Gregar console shows of its own on the custom screen (the
//!   custom screen's versioned pictures, `custom::GREGAR`).

use crate::Rom;

/// The chips whose picture and icon the Falzar ROM has wrong and the
/// Gregar ROM right: Bass, BigHook, DeltaRay, ColForce, BugRSwrd (their
/// records' +0x20 icon, +0x24 picture and +0x28 palette).
pub const RIGHT_IN_GREGAR: std::ops::RangeInclusive<u32> = 0x12D..=0x131;

/// The emotion window's pictures Gregar has its own of (`off_801CD08`'s
/// 5 to 0x16: its Crosses, tired, in Beast Out, its Beast, Full Synchro
/// and Beast Over); the pack numbers them after the Falzar ROM's 23, from
/// 0x17 (compat/assets.toml names them).
pub const OWN_FACES: std::ops::RangeInclusive<u32> = 5..=0x16;
/// The Gregar ROM's mugshot palettes (`off_801CD08`'s pictures' palettes).
pub const MUGSHOT_PALETTES: u32 = 0x0872_D050;

/// The Gregar ROM's link navis' faces (`sub_801CC34`'s table) and their
/// palettes, of which the first five are Gregar's own (HeatMan, ElecMan,
/// SlashMan, EraseMan, ChargeMan); the pack numbers them after the Falzar
/// ROM's six.
pub const NAVI_MUGSHOTS: u32 = 0x0872_AFD0;
pub const NAVI_MUGSHOT_PALETTES: u32 = 0x0872_B5D0;
pub const OWN_NAVI_FACES: u32 = 5;

/// The ROM to read chip `id`'s picture and icon from: the Gregar ROM for
/// `RIGHT_IN_GREGAR` (the chip table is at the same address in both).
pub fn chip_source<'a>(rom: &'a Rom, gregar: &'a Rom, id: u32) -> &'a Rom {
    if RIGHT_IN_GREGAR.contains(&id) { gregar } else { rom }
}
