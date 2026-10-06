//! The battle graphics, decoded from the ROM into their typed form (see the
//! nettai-assets crate); nettai-content writes them as the pack's images and
//! JSON.

use crate::decode::gfx_anims;
use crate::exe6::{Rom, u32at};
use crate::sprite::{archive, portrait_sheet, sheet as sprite_sheet};
use nettai_assets::*;

/// The battle graphics of the US Falzar ROM, with what the US Gregar ROM
/// has of its own or right (`gregar`) and what the Japanese ROMs have that
/// the US release cut (`jp`); `names` gives the chip icons their keys and
/// the font its characters.
pub fn bundle(roms: &crate::exe6::Roms, names: &nettai_content::names::AssetNames) -> Bundle {
    let rom = &roms.falzar;
    let mut hud = crate::exe6::hud::hud(roms, names);
    let mut custom = crate::exe6::custom::custom(roms, names);
    // The US ROMs' words are English; the Japanese ROMs' lettering is the
    // pack's Japanese (`lettering`).
    let ja = crate::exe6::lettering::LANGUAGE.to_string();
    hud.languages.push((
        ja.clone(),
        crate::exe6::lettering::hud(roms.falzar_jp, &hud, names, roms.falzar.is_present()),
    ));
    hud.language = nettai_assets::BASE_LANGUAGE.into();
    custom.languages.push((
        ja,
        crate::exe6::lettering::custom(roms.falzar_jp, roms.gregar_jp),
    ));
    Bundle {
        sprites: sprites(roms, names),
        field: if rom.is_present() {
            field(rom)
        } else {
            Field::default()
        },
        backgrounds: if rom.is_present() {
            backgrounds(rom)
        } else {
            Vec::new()
        },
        hud,
        custom,
    }
}

// ---- Sprites -------------------------------------------------------------------

/// `SpritePointersList`: per category, a table of sprite archive pointers
/// (bit 31 = LZ77 compressed).
const SPRITE_LIST: u32 = 0x0803_1CC4;
/// Categories with battle sprites (the byte offsets 0x00..=0x14).
const BATTLE_CATEGORIES: u32 = 6;

/// The portraits' category (`mugshotSpritePtrs`, byte offset 0x20) and its
/// black placeholder (`mugshotBlack`), which each US ROM has in place of
/// the other game's link navis.
const PORTRAITS: u8 = 0x20;
const BLACK_PORTRAIT: u32 = 7;

fn sprites(
    roms: &crate::exe6::Roms,
    names: &nettai_content::names::AssetNames,
) -> Vec<SpriteSheet> {
    let mut out = portraits(roms, names);
    let Some((rom, list)) = [
        (roms.falzar, SPRITE_LIST),
        (roms.gregar, SPRITE_LIST),
        (roms.falzar_jp, crate::exe6::jp::SPRITE_LIST),
        (roms.gregar_jp, crate::exe6::jp::SPRITE_LIST),
    ]
    .into_iter()
    .find(|(r, _)| r.is_present()) else {
        return out;
    };
    let cats: Vec<u32> = (0..10).map(|i| u32at(rom, list + 4 * i)).collect();
    for (ci, &c) in cats.iter().enumerate().take(BATTLE_CATEGORIES as usize) {
        let next = cats
            .iter()
            .copied()
            .filter(|&s| s > c)
            .min()
            .unwrap_or(c + 0x400);
        for idx in 0..((next - c) / 4).min(256) {
            let (category, index) = ((ci * 4) as u8, idx as u8);
            let jp = crate::exe6::jp::SPRITES.contains(&(category, index));
            let (source, p) = if jp {
                let Some(rom) = [roms.falzar_jp, roms.gregar_jp]
                    .into_iter()
                    .find(|r| r.is_present())
                else {
                    continue;
                };
                (
                    rom,
                    u32at(
                        rom,
                        u32at(rom, crate::exe6::jp::SPRITE_LIST + category as u32) + 4 * idx,
                    ),
                )
            } else {
                (rom, u32at(rom, c + 4 * idx))
            };
            let Some(data) = archive(source, p) else {
                continue;
            };
            if let Some(mut s) = sprite_sheet(&data, category, index) {
                s.region = jp.then(|| crate::exe6::jp::REGION.to_string());
                out.push(s);
            }
        }
    }
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// The portraits content names (the chatbox's speakers), each from the
/// ROM that has its true face: the Falzar ROM's unless it has the black
/// placeholder there (the table is at the same place in both).
fn portraits(
    roms: &crate::exe6::Roms,
    names: &nettai_content::names::AssetNames,
) -> Vec<SpriteSheet> {
    let sources = [
        (roms.falzar, SPRITE_LIST),
        (roms.gregar, SPRITE_LIST),
        (roms.falzar_jp, crate::exe6::jp::SPRITE_LIST),
        (roms.gregar_jp, crate::exe6::jp::SPRITE_LIST),
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
                    let table = u32at(rom, list + PORTRAITS as u32);
                    let p = u32at(rom, table + 4 * index as u32);
                    if p == u32at(rom, table + 4 * BLACK_PORTRAIT) {
                        return None;
                    }
                    let data = archive(rom, p)?;
                    portrait_sheet(&data, category, index)
                })
        })
        .collect()
}

const FIELD_TILES: u32 = 0x086D_DBA0;
/// Background palettes 1..=8 at battle start (`dword_86E08F8`, copied to
/// the palette buffer's slot 1 by `sub_80075CA`'s transfer list).
const FIELD_PALETTES: u32 = 0x086E_08F8;
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row - 1
/// (`byte_86DFA98`, read by `sub_800C01C`).
const PANEL_BLOCKS: u32 = 0x086D_FA98;
/// Highlighted panel blocks (`dword_86E0458`, `dword_86E0478`).
const HIGHLIGHT_BLOCKS: u32 = 0x086E_0458;
/// Front edges by owner (`byte_86E0498`, `sub_800C100`).
const FRONT_EDGES: u32 = 0x086E_0498;
/// Panel palette animations run by `sub_800C192` (`off_800C1DC`).
const PANEL_PALETTE_ANIMS: u32 = 0x0800_C1DC;
/// The palette buffer address of background palette 0.
const PALETTE_BUFFER: u32 = 0x0300_1960;

fn field(rom: &Rom) -> Field {
    crate::decode::field(
        rom,
        crate::decode::FieldAddresses {
            tiles: FIELD_TILES,
            palettes: FIELD_PALETTES,
            panels: PANEL_BLOCKS,
            highlights: [HIGHLIGHT_BLOCKS, HIGHLIGHT_BLOCKS + 32],
            edges: FRONT_EDGES,
            palette_anims: PANEL_PALETTE_ANIMS,
            palette_buffer: PALETTE_BUFFER,
        },
        (0..13).collect(),
    )
}

// ---- Backgrounds ---------------------------------------------------------------

/// Background load data by background id (`off_8080F98`, `LoadBGAnimData`):
/// gfx source, gfx dest, tilemap source, tilemap dest offset, palette
/// source, palette dest, palette size.
const BACKGROUNDS: u32 = 0x0808_0F98;
const BACKGROUND_COUNT: u32 = 22;
/// Per background: scroll callbacks (`off_8080E34`, 16 bytes each; the
/// second one scrolls BG1).
const BACKGROUND_SCROLL: u32 = 0x0808_0E34;
/// Per background: its GFX animation list (`off_8081220`).
const BACKGROUND_ANIMS: u32 = 0x0808_1220;

/// Scroll callbacks and their counter steps (1/16 pixel per frame).
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_19B4, (-8, -4)), // BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_19EC, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1A10, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_1A34, (1, 0)),   // BGScrollCB_BG1SlowRightScroll
    (0x0800_1A58, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

fn backgrounds(rom: &Rom) -> Vec<Option<Background>> {
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, w, h, palette) =
                crate::decode::background_picture(rom, u32at(rom, BACKGROUNDS + 4 * id))?;
            let cb = u32at(rom, BACKGROUND_SCROLL + 16 * id + 4) & !1;
            let scroll = SCROLLERS
                .iter()
                .find(|(a, _)| *a == cb)
                .map(|(_, v)| *v)
                .unwrap_or((0, 0));
            let anims = gfx_anims(rom, u32at(rom, BACKGROUND_ANIMS + 4 * id), PALETTE_BUFFER);
            Some(Background {
                tiles,
                first_tile,
                map,
                map_width: w,
                map_height: h,
                palette,
                scroll,
                anims,
                region: None,
            })
        })
        .collect()
}
