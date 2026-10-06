//! The battle graphics, decoded from the US Team ProtoMan ROM into their
//! typed form (nettai-assets), with Team Colonel's own chips. The routines
//! that read them are EXE6's, the same or nearly (the verification
//! workspace's tools/exe5 maps them); the addresses are where those routines'
//! literals point in this ROM.
//!
//! What an EXE5 pack has: the battle sprites, the field (panels, edges,
//! cycling palettes), the battle backgrounds, the chips' pictures and icons;
//! the HUD (hud.rs: its fonts, banners, faces and the chatbox) and the
//! custom screen (custom.rs), in EXE6's formats with what EXE5 lays out
//! otherwise said (`CustomLayout`, the faces' own boxes, the soul button).

use crate::decode::{background_picture, gfx_anims};
use crate::exe5::rom::{Rom, Roms, Version};
use nettai_assets::*;
use nettai_content::names::AssetNames;

/// `SpritePointersList` (EXE6 0x08031CC4): per category, a table of sprite
/// archive pointers. Categories with battle sprites are the byte offsets
/// 0x00..=0x14, as in EXE6.
const SPRITE_LIST: u32 = 0x0803_2728;
const BATTLE_CATEGORIES: usize = 6;

/// The field tiles (`sub_80075CA`'s, decompressed to VRAM 0x06001460) and
/// the background palettes 1..=8 its transfer list copies.
const FIELD_TILES: u32 = 0x086F_3BA0;
const FIELD_PALETTES: u32 = 0x086F_6970;
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row
/// - 1 (`sub_800C01C`'s table). EXE5 has 11 panel types (EXE6 13).
const PANEL_BLOCKS: u32 = 0x086F_5CB0;
const PANEL_TYPES: u32 = 11;
/// The highlighted panel block (`sub_800C0BA`'s): EXE5 has one, which it
/// draws for both highlights (EXE6 a table of two); the pack has it as both,
/// what EXE5 draws for each.
const HIGHLIGHT_BLOCK: u32 = 0x086F_64F0;
/// The front edges by owner (`sub_800C100`'s).
const FRONT_EDGES: u32 = 0x086F_6510;
/// The panel palette animations `sub_800C192`'s counterpart runs.
const PANEL_PALETTE_ANIMS: u32 = 0x0800_A7AC;
/// The palette buffer's background palette 0 (EXE6 0x03001960).
const PALETTE_BUFFER: u32 = 0x0300_3960;

/// The backgrounds' load data by background id (EXE6 `off_8080F98`), their
/// scroll callbacks (16 bytes each, the second scrolls BG1) and their GFX
/// animation lists.
const BACKGROUNDS: u32 = 0x0808_C578;
const BACKGROUND_COUNT: u32 = 29;
const BACKGROUND_SCROLL: u32 = 0x0808_C32C;
const BACKGROUND_ANIMS: u32 = 0x0808_C96C;
/// The scroll callbacks and their counters' steps (1/16 pixel a frame): the
/// same code as EXE6's `BGScrollCB_*`. EXE5's own at 0x080019EC follows the
/// joypad (a background the player scrolls) and 0x08001A24 does nothing;
/// both are drawn still.
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_1936, (-8, -4)), // EXE6 BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_196E, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1992, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_19B6, (8, 0)),   // a fast right scroll (EXE6 has a slow one)
    (0x0800_19C8, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

/// The chip records (0x2C bytes, as EXE6's): +0x20 the icon, +0x24 the
/// picture, +0x28 its palette. Team Colonel's table is 4 bytes lower.
const CHIP_DATA: [u32; 2] = [0x0801_E214, 0x0801_E210];
pub const CHIP_COUNT: u32 = 368;

/// The chips with art in the pack: the named ones' table, and past it the
/// ones the content names (the invalid chip, 0x185: its record's art is
/// the blank icon and picture the unused records have, what EXE5 draws).
fn chip_ids(names: &AssetNames) -> impl Iterator<Item = u32> + '_ {
    (0..CHIP_COUNT).chain(
        names
            .chips
            .keys()
            .map(|&id| id as u32)
            .filter(|&id| id >= CHIP_COUNT),
    )
}
const PICTURE_BYTES: usize = 0x540;

/// Everything the pack draws with: `names` gives the chips their keys, the
/// fonts their characters and the faces their names.
pub fn bundle(roms: &Roms, names: &AssetNames) -> Bundle {
    let rom = &roms.protoman;
    let mut hud = crate::exe5::hud::hud(roms, names, chip_icons(roms, names));
    let mut custom = crate::exe5::custom::custom(roms, names, chip_art(roms, names));
    // The US ROMs' words are English; the Japanese ROMs' lettering is the
    // pack's Japanese (`lettering`).
    let ja = crate::exe5::lettering::LANGUAGE.to_string();
    hud.languages
        .push((ja.clone(), crate::exe5::lettering::hud(roms, &hud, names)));
    hud.language = nettai_assets::BASE_LANGUAGE.into();
    custom
        .languages
        .push((ja, crate::exe5::lettering::custom(roms)));
    // The pack's backgrounds are the US ROMs' (the Japanese ROMs have
    // another picture of 0x05: the verification knows what their consoles
    // show there).
    let backgrounds = if rom.is_present() {
        backgrounds(rom)
    } else {
        Vec::new()
    };
    let mut sprites = [
        (&roms.protoman, SPRITE_LIST),
        (&roms.colonel, COLONEL_SPRITE_LIST),
        (&roms.protoman_jp, JP_SPRITE_LISTS[0]),
        (&roms.colonel_jp, JP_SPRITE_LISTS[1]),
    ]
    .into_iter()
    .find(|(r, _)| r.is_present())
    .map(|(r, list)| sprites(r, list))
    .unwrap_or_default();
    sprites.extend(portraits(roms, names));
    sprites.sort_by_key(|s| (s.category, s.index));
    Bundle {
        sprites,
        field: if rom.is_present() {
            field(rom)
        } else {
            Field::default()
        },
        backgrounds,
        hud,
        custom,
    }
}

use crate::exe5::hud::{palette, tiles};

// ---- Sprites -------------------------------------------------------------------

/// The Japanese ROMs' sprite lists (Team of Blues, Team of Colonel).
const JP_SPRITE_LISTS: [u32; 2] = [0x0803_26C0, 0x0803_26C4];

/// The battle sprites' archives in a ROM whose sprite list is at `list`, by
/// (category, index).
fn archives(rom: &Rom, list: u32) -> Vec<((u8, u8), Vec<u8>)> {
    let cats: Vec<u32> = (0..10).map(|i| rom.u32(list + 4 * i)).collect();
    let mut out = Vec::new();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES) {
        let next = cats
            .iter()
            .copied()
            .filter(|&s| s > c)
            .min()
            .unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            if let Some(data) = crate::exe5::sprite::archive(rom, rom.u32(c + 4 * idx)) {
                out.push((((ci * 4) as u8, idx as u8), data));
            }
        }
    }
    out
}

/// The battle sprites (the two US ROMs have the same ones).
fn sprites(rom: &Rom, list: u32) -> Vec<SpriteSheet> {
    let mut out: Vec<SpriteSheet> = archives(rom, list)
        .into_iter()
        .filter_map(|((c, i), data)| crate::exe5::sprite::sheet(&data, c, i))
        .collect();
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// The portraits' category (the sprite list's byte offset 0x20, EXE6's
/// `mugshotSpritePtrs`) and an entry every ROM has its placeholder in.
const PORTRAITS: u8 = 0x20;
const PLACEHOLDER_PORTRAIT: u32 = 0x2A;
/// Team Colonel's US ROM's sprite list.
const COLONEL_SPRITE_LIST: u32 = 0x0803_272C;

/// The portraits content names (the chatbox's speakers: a no-running
/// message's `F5 00 n`), each from the ROM that has its face: Team
/// ProtoMan's unless it has the placeholder there, then Team Colonel's
/// (the Japanese ROMs have the same pictures and the same placeholders).
/// Team ProtoMan's ROM has the placeholder for four of Team Colonel's
/// navis, Team Colonel's for five of Team ProtoMan's: a console operates
/// its own team's navis alone (a save has a stats block for no other), so
/// no placeholder is ever shown, and the pack keeps no mark of whose a
/// face is.
fn portraits(roms: &Roms, names: &AssetNames) -> Vec<SpriteSheet> {
    let sources = [
        (roms.protoman, SPRITE_LIST),
        (roms.colonel, COLONEL_SPRITE_LIST),
        (roms.protoman_jp, JP_SPRITE_LISTS[0]),
        (roms.colonel_jp, JP_SPRITE_LISTS[1]),
    ];
    names
        .sprites
        .keys()
        .filter(|(c, _)| *c == PORTRAITS)
        .filter_map(|&(category, index)| {
            sources
                .iter()
                .filter(|(r, _)| r.is_present())
                .find_map(|&(rom, list)| {
                    let table = rom.u32(list + PORTRAITS as u32);
                    let p = rom.u32(table + 4 * index as u32);
                    if p == rom.u32(table + 4 * PLACEHOLDER_PORTRAIT) {
                        return None;
                    }
                    let data = crate::sprite::archive(rom, p)?;
                    crate::sprite::portrait_sheet(&data, category, index)
                })
        })
        .collect()
}

// ---- Field -----------------------------------------------------------------------

fn field(rom: &Rom) -> Field {
    let panel_types = (0..PANEL_TYPES as u8)
        .map(|n| match exe5_compat::Compat::exe5().panel_type(n) {
            Ok(Some(t)) => t as u8,
            other => panic!("EXE5 panel {n}: {other:?}"),
        })
        .collect();
    crate::decode::field(
        rom,
        crate::decode::FieldAddresses {
            tiles: FIELD_TILES,
            palettes: FIELD_PALETTES,
            panels: PANEL_BLOCKS,
            highlights: [HIGHLIGHT_BLOCK; 2],
            edges: FRONT_EDGES,
            palette_anims: PANEL_PALETTE_ANIMS,
            palette_buffer: PALETTE_BUFFER,
        },
        panel_types,
    )
}

// ---- Backgrounds -------------------------------------------------------------------

fn backgrounds(rom: &Rom) -> Vec<Option<Background>> {
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, map_width, map_height, palette) =
                background_picture(rom, rom.u32(BACKGROUNDS + 4 * id))?;
            let cb = rom.u32(BACKGROUND_SCROLL + 16 * id + 4) & !1;
            let scroll = SCROLLERS
                .iter()
                .find(|(a, _)| *a == cb)
                .map(|(_, v)| *v)
                .unwrap_or((0, 0));
            let anims = gfx_anims(rom, rom.u32(BACKGROUND_ANIMS + 4 * id), PALETTE_BUFFER);
            Some(Background {
                tiles,
                first_tile,
                map,
                map_width,
                map_height,
                palette,
                scroll,
                anims,
            })
        })
        .collect()
}

// ---- Chips -------------------------------------------------------------------------

/// A chip's record in a version's ROM.
fn chip_source<'a>(roms: &'a Roms, v: Version) -> (&'a Rom, u32) {
    let i = (v == Version::Colonel) as usize;
    if roms.us(v).is_present() {
        (roms.us(v), CHIP_DATA[i])
    } else {
        (roms.jp(v), [0x0801_e1d0, 0x0801_e1cc][i])
    }
}

/// A chip's picture (tiles and palette), icon and whether its record has
/// them, in a version's ROM. A picture whose palette no ROM holds (the
/// e-Reader cards' LeadRaid and ChaosLrd: their records point at EWRAM,
/// 0x02001660 and 0x02001680, where the save's card data goes) gets a
/// black one; its definition gives the palette (`art_palette`), as EXE6's
/// gift chips' do.
fn chip_media(roms: &Roms, v: Version, id: u32) -> (Picture, Tiles) {
    let (rom, table) = chip_source(roms, v);
    if !rom.is_present() {
        return (Picture::default(), Tiles::default());
    }
    let r = table + 0x2c * id;
    let (icon, gfx, pal) = (rom.u32(r + 0x20), rom.u32(r + 0x24), rom.u32(r + 0x28));
    let picture = if rom.contains(gfx) {
        let palette = if rom.contains(pal) {
            palette(rom, pal)
        } else {
            Default::default()
        };
        Picture {
            tiles: tiles(rom, gfx, PICTURE_BYTES),
            palette,
        }
    } else {
        Picture::default()
    };
    let icon = if rom.contains(icon) {
        tiles(rom, icon, 0x80)
    } else {
        Tiles::default()
    };
    (picture, icon)
}

/// Ownership from the original libraries. EXE5 interleaves the teams' Giga
/// chips, unlike EXE6; it must not depend on having both ROMs to compare.
const VERSION_CHIPS: [(u32, Version); 12] = [
    (0x12d, Version::ProtoMan),
    (0x12e, Version::ProtoMan),
    (0x12f, Version::ProtoMan),
    (0x130, Version::Colonel),
    (0x131, Version::Colonel),
    (0x132, Version::Colonel),
    (0x133, Version::ProtoMan),
    (0x134, Version::ProtoMan),
    (0x135, Version::Colonel),
    (0x136, Version::Colonel),
    (0x139, Version::Colonel),
    (0x13a, Version::ProtoMan),
];

/// The version chip `id` belongs to; none for a chip of both.
fn version_of(versions: &[(u32, Version)], id: u32) -> Option<Version> {
    versions
        .iter()
        .find(|(chip, _)| *chip == id)
        .map(|&(_, v)| v)
}

/// The team navis' own chips (the custom screen's table of them,
/// 0x08025EA0: two halfwords a navi, navis 1 to 12, each the chip's id
/// under its code), each with its navi's team's version. The two US ROMs
/// hold an own chip's picture and icon alike but under different palettes:
/// a console shows its own team's navis' chips in the palette its ROM has
/// for them, and never shows the other team's (their navis aren't its
/// PET's). So an own chip's picture is its team's ROM's, on any console.
const OWN_CHIPS: [(u32, Version); 12] = [
    (0x191, Version::ProtoMan),
    (0x192, Version::ProtoMan),
    (0x195, Version::ProtoMan),
    (0x196, Version::ProtoMan),
    (0x198, Version::ProtoMan),
    (0x19a, Version::ProtoMan),
    (0x19d, Version::Colonel),
    (0x19e, Version::Colonel),
    (0x1a1, Version::Colonel),
    (0x1a2, Version::Colonel),
    (0x1a5, Version::Colonel),
    (0x1a6, Version::Colonel),
];

/// The chips' pictures, each under its chip's key: a version chip's from
/// its own version's ROM, marked with it (a console of the other version
/// shows its counterpart's there); a team navi's own chip's from its team's
/// (`own_chips`); any other from Team ProtoMan's.
fn chip_art(roms: &Roms, names: &AssetNames) -> Vec<ChipArt> {
    let versions = VERSION_CHIPS;
    let team = OWN_CHIPS;
    chip_ids(names)
        .map(|id| {
            let own = version_of(&versions, id);
            // (A team navi's own chip's: its team's ROM's, with no
            // counterpart on the other version's console.)
            let from = own.or(version_of(&team, id)).unwrap_or(
                if chip_source(roms, Version::ProtoMan).0.is_present() {
                    Version::ProtoMan
                } else {
                    Version::Colonel
                },
            );
            let (picture, _) = chip_media(roms, from, id);
            ChipArt {
                key: names.chip_icon(id as u16),
                picture,
                version: own.map(|v| v.name().into()),
            }
        })
        .collect()
}

/// The chips' icons, each under its chip's key, from the ROM its picture
/// is from.
fn chip_icons(roms: &Roms, names: &AssetNames) -> Vec<ChipIcon> {
    let versions = VERSION_CHIPS;
    chip_ids(names)
        .map(|id| {
            let (_, icon) = chip_media(
                roms,
                version_of(&versions, id).unwrap_or(
                    if chip_source(roms, Version::ProtoMan).0.is_present() {
                        Version::ProtoMan
                    } else {
                        Version::Colonel
                    },
                ),
                id,
            );
            ChipIcon {
                key: names.chip_icon(id as u16),
                tiles: icon,
            }
        })
        .collect()
}
