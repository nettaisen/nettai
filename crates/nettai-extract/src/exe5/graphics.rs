//! The battle graphics, decoded into their typed form (nettai-assets) from
//! whichever of the four ROMs holds each: what they have alike from any, a
//! team's own from a ROM of that version, the US ROM's wherever both
//! releases have their own. The routines that read them are EXE6's, the
//! same or nearly (the verification workspace's tools/exe5 maps them);
//! each block is stated by where the game's own pointer to it is in each
//! ROM (`Place`: those routines' literals, mostly at an address of their
//! own in each ROM, as the two versions' code parts early).
//!
//! What an EXE5 pack has: the battle sprites, the field (panels, edges,
//! cycling palettes), the battle backgrounds, the chips' pictures and icons;
//! the HUD (hud.rs: its fonts, banners, faces and the chatbox) and the
//! custom screen (custom.rs), in EXE6's formats with what EXE5 lays out
//! otherwise said (`CustomLayout`, the faces' own boxes, the soul button).

use crate::decode::{background_picture, gfx_anims, speeding_scroll};
use crate::exe5::rom::{Place, Roms, Source, Version};
use nettai_assets::*;
use nettai_content::names::AssetNames;

/// `SpritePointersList` (EXE6 0x08031CC4; Team ProtoMan US's 0x08032728):
/// per category, a table of sprite archive pointers. Categories with battle
/// sprites are the byte offsets 0x00..=0x14, as in EXE6. Its pointer is at
/// the same address in the four ROMs.
const SPRITE_LIST: Place = Place::pointer(0x0800_2690, 0x0800_2690);
const BATTLE_CATEGORIES: usize = 6;

/// The field tiles (`sub_80075CA`'s, decompressed to VRAM 0x06001460;
/// Team ProtoMan US's 0x086F3BA0) and the background palettes 1..=8 its
/// transfer list copies (0x086F6970).
const FIELD_TILES: Place = Place::pointer(0x0800_6A34, 0x0800_6A34);
const FIELD_PALETTES: Place = Place::pointer(0x0800_6A08, 0x0800_6A08);
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row
/// - 1 (`sub_800C01C`'s table, 0x086F5CB0). EXE5 has 11 panel types (EXE6
/// 13).
const PANEL_BLOCKS: Place = Place::pointer_in_each([0x0800_A658, 0x0800_A650, 0x0800_A66C, 0x0800_A664]);
const PANEL_TYPES: u32 = 11;
/// The highlighted panel block (`sub_800C0BA`'s, 0x086F64F0): EXE5 has one,
/// which it draws for both highlights (EXE6 a table of two); the pack has
/// it as both, what EXE5 draws for each.
const HIGHLIGHT_BLOCK: Place = Place::pointer_in_each([0x0800_A6C8, 0x0800_A6C0, 0x0800_A6DC, 0x0800_A6D4]);
/// The front edges by owner (`sub_800C100`'s, 0x086F6510).
const FRONT_EDGES: Place = Place::pointer_in_each([0x0800_A73C, 0x0800_A734, 0x0800_A750, 0x0800_A748]);
/// The panel palette animations `sub_800C192`'s counterpart runs
/// (0x0800A7AC).
const PANEL_PALETTE_ANIMS: Place = Place::pointer_in_each([0x0800_A7A4, 0x0800_A79C, 0x0800_A7B8, 0x0800_A7B0]);
/// The palette buffer's background palette 0 (EXE6 0x03001960): RAM, the
/// same in the four ROMs.
const PALETTE_BUFFER: u32 = 0x0300_3960;

/// The backgrounds' tables (the loader's literals): the load data by
/// background id (EXE6 `off_8080F98`; Team ProtoMan US's 0x0808C578), the
/// scroll callbacks (16 bytes each, the second scrolls BG1; 0x0808C32C)
/// and the GFX animation lists (0x0808C96C). One background differs
/// between the releases: 0x05, Yai's HP, which the Japanese ROMs draw as
/// bubbles in the dark, still, its palette its animations' (the US ROMs':
/// goldfish, scrolling).
const BACKGROUNDS: Place = Place::pointer_in_each([0x0808_C574, 0x0808_C5E4, 0x0808_BEC0, 0x0808_BF30]);
const BACKGROUND_SCROLL: Place = Place::pointer_in_each([0x0808_C328, 0x0808_C398, 0x0808_BC74, 0x0808_BCE4]);
const BACKGROUND_ANIMS: Place = Place::pointer_in_each([0x0808_C324, 0x0808_C394, 0x0808_BC70, 0x0808_BCE0]);
const BACKGROUND_COUNT: u32 = 29;
/// The scroll callbacks and their counters' steps (1/16 pixel a frame): the
/// same code as EXE6's `BGScrollCB_*`. 0x08001A24 does nothing (drawn
/// still); EXE5's own at 0x080019EC (nebulagray's, 0x1B) is a `speeding`
/// scroll from the battle's first frame, read from its code
/// (`speeding_scroll`): while battle flag 0x40 is set (BattleState+0x5C,
/// which the battle's intro sets on its first frame, 0x080E06A0) it takes
/// 0x400 from its counter (0x0200A728, which the battle's init clears,
/// 0x0808C2B8), to -0x40000, and moves BG1 down by the counter's arithmetic
/// shift right 16 (BG1VOFS minus it). (The callbacks are at the same
/// addresses in the four ROMs.)
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_1936, (-8, -4)), // EXE6 BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_196E, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1992, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_19B6, (8, 0)),   // a fast right scroll (EXE6 has a slow one)
    (0x0800_19C8, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

/// The chip records (0x2C bytes, as EXE6's; the chip accessor's literal,
/// Team ProtoMan US's table 0x0801E214): +0x20 the icon, +0x24 the picture,
/// +0x28 its palette.
const CHIP_DATA: Place = Place::pointer_in_each([0x0801_DF1C, 0x0801_DF18, 0x0801_DED8, 0x0801_DED4]);
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
    let art = roms.any();
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
    // The pack's backgrounds are a US ROM's; a Japanese ROM's where no
    // US ROM is given (it has another picture of 0x05: the verification
    // knows what a Japanese console shows there). The sprites likewise
    // (the four ROMs have the same but for one label's words).
    let backgrounds = if art.is_present() { backgrounds(art) } else { Vec::new() };
    let mut sprites = if art.is_present() { sprites(art) } else { Vec::new() };
    sprites.extend(portraits(roms, names));
    sprites.sort_by_key(|s| (s.category, s.index));
    Bundle {
        sprites,
        field: if art.is_present() {
            field(art)
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

/// The battle sprites' archives in a ROM, by (category, index).
fn archives(src: Source) -> Vec<((u8, u8), Vec<u8>)> {
    let (rom, list) = (src.rom, src.at(SPRITE_LIST));
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

/// The battle sprites (the two versions' ROMs have the same ones).
fn sprites(src: Source) -> Vec<SpriteSheet> {
    let mut out: Vec<SpriteSheet> = archives(src)
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

/// The portraits content names (the chatbox's speakers: a no-running
/// message's `F5 00 n`), each from a ROM that has its face: a Team
/// ProtoMan ROM's unless it has the placeholder there, then a Team
/// Colonel ROM's (a version's Japanese ROM has the same pictures and the
/// same placeholders as its US one).
/// Team ProtoMan's ROM has the placeholder for four of Team Colonel's
/// navis, Team Colonel's for five of Team ProtoMan's: a console operates
/// its own team's navis alone (a save has a stats block for no other), so
/// no placeholder is ever shown, and the pack keeps no mark of whose a
/// face is.
fn portraits(roms: &Roms, names: &AssetNames) -> Vec<SpriteSheet> {
    let sources = [roms.of(Version::ProtoMan), roms.of(Version::Colonel)];
    names
        .sprites
        .keys()
        .filter(|(c, _)| *c == PORTRAITS)
        .filter_map(|&(category, index)| {
            sources
                .iter()
                .filter(|src| src.is_present())
                .find_map(|src| {
                    let rom = src.rom;
                    let table = rom.u32(src.at(SPRITE_LIST) + PORTRAITS as u32);
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

/// EXE5's panel types' names by number (compat/panels.toml).
pub fn panel_names() -> Vec<String> {
    let c = exe5_compat::Compat::exe5();
    (0..PANEL_TYPES as u8).map(|n| c.panel_name(n).unwrap_or_else(|e| panic!("EXE5 panel {n}: {e}")).to_string()).collect()
}

fn field(src: Source) -> Field {
    let panel_types = panel_names();
    crate::decode::field(
        src.rom,
        crate::decode::FieldAddresses {
            tiles: src.at(FIELD_TILES),
            palettes: src.at(FIELD_PALETTES),
            panels: src.at(PANEL_BLOCKS),
            highlights: [src.at(HIGHLIGHT_BLOCK); 2],
            edges: src.at(FRONT_EDGES),
            palette_anims: src.at(PANEL_PALETTE_ANIMS),
            palette_buffer: PALETTE_BUFFER,
        },
        panel_types,
    )
}

// ---- Backgrounds -------------------------------------------------------------------

/// The backgrounds by number, one picture a background: where compat's
/// assets.toml makes several numbers one background (a liberation's map's
/// number and its area's), the pack has the first's, and the others must be
/// the same (their load data, their scroll callbacks and their animations),
/// or the extraction fails.
fn backgrounds(src: Source) -> Vec<Option<Background>> {
    let rom = src.rom;
    let tables = (src.at(BACKGROUNDS), src.at(BACKGROUND_SCROLL), src.at(BACKGROUND_ANIMS));
    let mut out = pictures(src);
    let raw = |id: u32| {
        let d = rom.u32(tables.0 + 4 * id);
        let load: Vec<u32> = (0..7).map(|i| rom.u32(d + 4 * i)).collect();
        let scroll: Vec<u32> = (0..4).map(|i| rom.u32(tables.1 + 16 * id + 4 * i)).collect();
        (load, scroll)
    };
    for (name, numbers) in &exe5_compat::Compat::exe5().assets.backgrounds {
        let (&first, others) = numbers.all().split_first().expect("compat checks a background has a number");
        for &n in others {
            assert!(
                raw(n as u32) == raw(first as u32) && out.get(n as usize) == out.get(first as usize),
                "background {name}: {n:#04x} isn't {first:#04x}'s picture again (its load data, scroll or animations differ)"
            );
            out[n as usize] = None;
        }
    }
    out
}

/// Each background number's picture as its load data, scroll callback and
/// animations make it.
fn pictures(src: Source) -> Vec<Option<Background>> {
    let rom = src.rom;
    let (loads, scrolls, anim_lists) = (src.at(BACKGROUNDS), src.at(BACKGROUND_SCROLL), src.at(BACKGROUND_ANIMS));
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, map_width, map_height, palette) =
                background_picture(rom, rom.u32(loads + 4 * id))?;
            let cb = rom.u32(scrolls + 16 * id + 4) & !1;
            let scroll = SCROLLERS
                .iter()
                .find(|(a, _)| *a == cb)
                .map(|(_, v)| *v)
                .unwrap_or((0, 0));
            let speeding = speeding_scroll(rom, cb);
            let anims = gfx_anims(rom, rom.u32(anim_lists + 4 * id), PALETTE_BUFFER);
            Some(Background {
                tiles,
                first_tile,
                map,
                map_width,
                map_height,
                palette,
                scroll,
                speeding,
                anims,
            })
        })
        .collect()
}

// ---- Chips -------------------------------------------------------------------------

/// A chip's picture (tiles and palette), icon and whether its record has
/// them, in a version's ROM. A picture whose palette no ROM holds (the
/// e-Reader cards' LeadRaid and ChaosLrd: their records point at EWRAM,
/// 0x02001660 and 0x02001680, where the save's card data goes) gets a
/// black one; its definition gives the palette (`art_palette`), as EXE6's
/// gift chips' do.
fn chip_media(src: Source, id: u32) -> (Picture, Tiles) {
    if !src.is_present() {
        return (Picture::default(), Tiles::default());
    }
    let rom = src.rom;
    let r = src.at(CHIP_DATA) + 0x2c * id;
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

/// The ROM a chip's record is read from: one of `version` (its US ROM
/// before its Japanese one), or for a chip of both, which every ROM has
/// alike, a Team ProtoMan ROM where one is given.
fn chip_source<'a>(roms: &Roms<'a>, version: Option<Version>) -> Source<'a> {
    match version {
        Some(v) => roms.of(v),
        None => roms.preferring(Version::ProtoMan),
    }
}

/// The chips' pictures, each under its chip's key: a version chip's from
/// its own version's ROM, marked with it (a console of the other version
/// shows its counterpart's there); a team navi's own chip's from its team's
/// (`own_chips`); any other from a ROM that is there.
fn chip_art(roms: &Roms, names: &AssetNames) -> Vec<ChipArt> {
    let versions = VERSION_CHIPS;
    let team = OWN_CHIPS;
    chip_ids(names)
        .map(|id| {
            let own = version_of(&versions, id);
            // (A team navi's own chip's: its team's ROM's, with no
            // counterpart on the other version's console.)
            let (picture, _) = chip_media(chip_source(roms, own.or(version_of(&team, id))), id);
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
            let (_, icon) = chip_media(chip_source(roms, version_of(&versions, id)), id);
            ChipIcon {
                key: names.chip_icon(id as u16),
                tiles: icon,
            }
        })
        .collect()
}
