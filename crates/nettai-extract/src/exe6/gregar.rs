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
//! - The Cybeasts' sprites (`SPRITES`): a ROM has its own version's and a
//!   placeholder archive in the other's slot.
//! - One portrait the Gregar ROMs hold of a Falzar navi, DustMan's, that
//!   their table doesn't point at (`DUSTMAN_PORTRAIT`).

use crate::exe6::{Place, Roms, Source, Version};

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

/// The sprite slots (category's byte offset, index) only one version's
/// ROMs fill, the other's holding the placeholder archive there (the one
/// the US ROMs have in `jp::SPRITES`' slots): the Cybeasts, Gregar and
/// Falzar. The pack has each from a ROM of its version.
pub const SPRITES: [((u8, u8), Version); 2] = [((0x08, 0x14), Version::Gregar), ((0x08, 0x15), Version::Falzar)];

/// DustMan's portrait (the portraits' index 0x4E) in the Gregar ROMs. Its
/// archive is there, after SlashMan's, though DustMan is Falzar's and the
/// Gregar ROMs have none of Falzar's other four link navis' portraits; no
/// entry of the portraits' table points at it (theirs for DustMan is the
/// black one), and an archive doesn't say where it ends, so the place
/// can't be reckoned from the one before. It is stated: the US Gregar
/// ROM's and the Japanese one's.
pub const DUSTMAN_PORTRAIT: (u8, Place) = (0x4E, Place::stated([0, 0x084F_2AE8, 0, 0x0850_2254]));

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

/// The Falzar chip, whose icon the Falzar ROMs alone have: the Gregar
/// ROMs' record has a placeholder that reads "13A". (The Gregar chip's
/// icon is in all four.)
const FALZAR_CHIP: u32 = 0x139;

/// The ROM chip `id`'s record is read from: one of its own version, the
/// US ROM before the Japanese one (the pack has the US games' art where
/// the releases differ; the Japanese ROMs' pictures of the chips the US
/// ROMs have none of are `jp::CHIP_PICTURES`). A chip of both is the same
/// in every ROM but for the BeastOut chip's picture, which is the
/// version's Beast Out button's: the pack has the Falzar ROMs', its base
/// version's, so a Falzar ROM is read where one is given.
pub fn chip_source<'a>(roms: &Roms<'a>, id: u32) -> Source<'a> {
    match chip_version(id) {
        Some(version) => roms.of(version),
        None => roms.preferring(Version::Falzar),
    }
}

/// The ROM chip `id`'s icon is read from: its record's (`chip_source`),
/// but the Falzar chip's, which is a Falzar ROM's alone.
pub fn icon_source<'a>(roms: &Roms<'a>, id: u32) -> Source<'a> {
    if id == FALZAR_CHIP { roms.of(Version::Falzar) } else { chip_source(roms, id) }
}
