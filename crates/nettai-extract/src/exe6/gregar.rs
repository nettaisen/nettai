//! What is a version's own, which the pack takes from a ROM of that
//! version (its US ROM, `MEGAMAN6_FXXBR6E` or `MEGAMAN6_GXXBR5E`, before
//! its Japanese one). The same code reaches them in every ROM, so the
//! places are the other modules'; here is which of them are a version's.
//!
//! - A version's own faces: a ROM has none for the other version's
//!   Crosses, its Beast and its link navis (its tables show its own
//!   counterpart's). The pack has the Gregar ROMs' as faces of their own,
//!   numbered after the Falzar ROMs', and Gregar's forms and navis name
//!   them: every form shows its true face, on either console.
//! - Gregar's five Giga chips' pictures and icons. Each US ROM draws its own
//!   version's five and has them again at the other version's (Bass and
//!   BassAnly, BigHook and MetrKnuk, DeltaRay and CrossDiv, ColForce and
//!   HubBatc, BugRSwrd and BgDthThd share one picture, palette and icon in
//!   each ROM): the pack has each chip's from its own version's ROM, on
//!   either console, marked with that version (`chip_version`).
//! - What a console shows of its own version on the custom screen (the
//!   custom screen's versioned pictures, `custom::version_pictures`).

use crate::exe6::{Roms, Source, Version};

/// The chips whose picture and icon the Falzar ROM has wrong and the
/// Gregar ROM right: Bass, BigHook, DeltaRay, ColForce, BugRSwrd (their
/// records' +0x20 icon, +0x24 picture and +0x28 palette).
pub const RIGHT_IN_GREGAR: std::ops::RangeInclusive<u32> = 0x12D..=0x131;
/// Their counterparts, which the Falzar ROM has right and the Gregar ROM
/// wrong: BassAnly, MetrKnuk, CrossDiv, HubBatc, BgDthThd.
pub const RIGHT_IN_FALZAR: std::ops::RangeInclusive<u32> = 0x132..=0x136;

/// The emotion window's pictures a version has its own of (`off_801CD08`'s
/// 5 to 0x16: its Crosses, tired, in Beast Out, its Beast, Full Synchro
/// and Beast Over; the five before them are MegaMan's own, the same in
/// every ROM). The pack numbers Gregar's after the Falzar ROMs' 23, from
/// 0x17 (compat/assets.toml names them).
pub const OWN_FACES: std::ops::RangeInclusive<u32> = 5..=0x16;

/// How many of the link navis' faces (`sub_801CC34`'s table) are a
/// version's own: the first five (Gregar's HeatMan, ElecMan, SlashMan,
/// EraseMan, ChargeMan); the sixth is the same in every ROM. The pack
/// numbers Gregar's after the Falzar ROMs' six.
pub const OWN_NAVI_FACES: u32 = 5;

/// The version chip `id` belongs to, whose ROMs alone have its picture
/// and icon (the library flag of its record, +0x09's 0x40, is set there);
/// none for a chip of both.
pub fn chip_version(id: u32) -> Option<Version> {
    if RIGHT_IN_GREGAR.contains(&id) {
        Some(Version::Gregar)
    } else if RIGHT_IN_FALZAR.contains(&id) {
        Some(Version::Falzar)
    } else {
        None
    }
}

/// The ROM chip `id`'s record is read from: one of its own version (a
/// chip of both: any), the US ROM before the Japanese one (the pack has
/// the US games' art where the releases differ; the Japanese ROMs'
/// pictures of the chips the US ROMs have none of are
/// `jp::CHIP_PICTURES`).
pub fn chip_source<'a>(roms: &Roms<'a>, id: u32) -> Source<'a> {
    match chip_version(id) {
        Some(version) => roms.of(version),
        None => roms.any(),
    }
}
