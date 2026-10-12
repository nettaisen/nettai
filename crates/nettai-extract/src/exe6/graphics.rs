//! The battle graphics, decoded from the ROM into their typed form (see the
//! nettai-assets crate); nettai-content writes them as the pack's images and
//! JSON.

use crate::decode::gfx_anims;
use crate::exe6::{Place, Roms, Source, Version, u32at};
use crate::sprite::{archive, portrait_sheet, sheet as sprite_sheet};
use nettai_assets::*;

/// The battle graphics the ROMs given hold: what the four have alike from
/// whichever is there, a version's own from a ROM of that version
/// (`gregar`), each release's words from a ROM of that release, and what
/// the US release cut from a Japanese ROM (`jp`); the US ROM's wherever
/// both releases have their own. `names` gives the chip icons their keys
/// and the font its characters.
pub fn bundle(roms: &Roms, names: &nettai_content::names::AssetNames) -> Bundle {
    let art = roms.any();
    let mut hud = crate::exe6::hud::hud(roms, names);
    let mut custom = crate::exe6::custom::custom(roms, names);
    // The US ROMs' words are English; the Japanese ROMs' lettering is the
    // pack's Japanese (`lettering`).
    let ja = crate::exe6::lettering::LANGUAGE.to_string();
    hud.languages.push((
        ja.clone(),
        crate::exe6::lettering::hud(roms, &hud, names),
    ));
    hud.language = nettai_assets::BASE_LANGUAGE.into();
    custom.languages.push((ja, crate::exe6::lettering::custom(roms)));
    Bundle {
        sprites: sprites(roms, names),
        field: if art.is_present() {
            field(art)
        } else {
            Field::default()
        },
        backgrounds: if art.is_present() {
            backgrounds(art)
        } else {
            Vec::new()
        },
        hud,
        custom,
    }
}

// ---- Sprites -------------------------------------------------------------------

/// `SpritePointersList`: per category, a table of sprite archive pointers
/// (bit 31 = LZ77 compressed). Its pointer (`off_8002724`) is at the same
/// address in the four ROMs.
const SPRITE_LIST: Place = Place::pointer(0x0800_2724, 0x0800_2724);
/// Categories with battle sprites (the byte offsets 0x00..=0x14).
const BATTLE_CATEGORIES: u32 = 6;

/// The portraits' category (`mugshotSpritePtrs`, byte offset 0x20) and its
/// black placeholder (`mugshotBlack`), which each US ROM has in place of
/// the other game's link navis.
const PORTRAITS: u8 = 0x20;
const BLACK_PORTRAIT: u32 = 7;

fn sprites(roms: &Roms, names: &nettai_content::names::AssetNames) -> Vec<SpriteSheet> {
    let mut out = portraits(roms, names);
    // A US ROM before a Japanese one: the releases' sheets are the same but
    // for the six the US ROMs have a placeholder for (`jp::SPRITES`, the
    // Japanese ROMs' whichever this is) and the flames HeatMan wears
    // (10-19), whose throw (animation 6) has three frames of 5, 4 and 3
    // ticks in the US ROMs and four of 3 in the Japanese ones. And the
    // versions' sheets are the same but for the two slots each has a
    // placeholder in for the other's (`gregar::SPRITES`).
    let src = roms.any();
    if !src.is_present() {
        return out;
    }
    let (rom, list) = (src.rom, src.at(SPRITE_LIST));
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
            let slot = (category, index);
            let own = if crate::exe6::jp::SPRITES.contains(&slot) {
                roms.jp()
            } else if let Some(&(_, version)) = crate::exe6::gregar::SPRITES.iter().find(|(s, _)| *s == slot) {
                roms.of(version)
            } else {
                src
            };
            if !own.is_present() {
                continue;
            }
            let p = u32at(own.rom, u32at(own.rom, own.at(SPRITE_LIST) + category as u32) + 4 * idx);
            let Some(data) = archive(own.rom, p) else {
                continue;
            };
            if let Some(s) = sprite_sheet(&data, category, index) {
                out.push(s);
            }
        }
    }
    out.sort_by_key(|s| (s.category, s.index));
    out
}

/// The portraits content names (the chatbox's speakers), each from the
/// ROM that has its true face: the Falzar ROM's unless it has the black
/// placeholder there.
fn portraits(roms: &Roms, names: &nettai_content::names::AssetNames) -> Vec<SpriteSheet> {
    let sources = [
        roms.us_of(Version::Falzar),
        roms.us_of(Version::Gregar),
        roms.jp_of(Version::Falzar),
        roms.jp_of(Version::Gregar),
    ];
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
                    let table = u32at(rom, src.at(SPRITE_LIST) + PORTRAITS as u32);
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

/// The field's tiles (`dword_86DDBA0`, the battle's transfer list's).
const FIELD_TILES: Place = Place::pointer(0x0800_761C, 0x0800_7610);
/// Background palettes 1..=8 at battle start (`dword_86E08F8`, copied to
/// the palette buffer's slot 1 by `sub_80075CA`'s transfer list).
const FIELD_PALETTES: Place = Place::pointer(0x0800_75F0, 0x0800_75E4);
/// Panel blocks: 32 bytes (5x3 map entries) per 6 * type + 3 * owner + row - 1
/// (`byte_86DFA98`, read by `sub_800C01C`).
const PANEL_BLOCKS: Place = Place::pointer(0x0800_C07C, 0x0800_C698);
/// Highlighted panel blocks (`dword_86E0458`, and `dword_86E0478` after
/// it).
const HIGHLIGHT_BLOCKS: Place = Place::pointer(0x0800_C0F4, 0x0800_C710);
/// Front edges by owner (`byte_86E0498`, `sub_800C100`).
const FRONT_EDGES: Place = Place::pointer(0x0800_C16C, 0x0800_C788);
/// Panel palette animations run by `sub_800C192` (`off_800C1DC`).
const PANEL_PALETTE_ANIMS: Place = Place::pointer(0x0800_C1D4, 0x0800_C7F0);
/// The palette buffer address of background palette 0 (RAM: the same in
/// the four ROMs).
const PALETTE_BUFFER: u32 = 0x0300_1960;

fn field(src: Source) -> Field {
    crate::decode::field(
        src.rom,
        crate::decode::FieldAddresses {
            tiles: src.at(FIELD_TILES),
            palettes: src.at(FIELD_PALETTES),
            panels: src.at(PANEL_BLOCKS),
            highlights: [src.at(HIGHLIGHT_BLOCKS), src.at(HIGHLIGHT_BLOCKS) + 32],
            edges: src.at(FRONT_EDGES),
            palette_anims: src.at(PANEL_PALETTE_ANIMS),
            palette_buffer: PALETTE_BUFFER,
        },
        panel_names(),
    )
}

/// EXE6's 13 panel types' names by number (`word_3007924`'s order: its
/// blocks', a panel trail's byte's; content/exe6/rules/panels.luau's
/// `numbers`).
pub fn panel_names() -> Vec<String> {
    [
        "missing", "broken", "normal", "cracked", "poison", "holy", "grass", "ice", "volcano", "road_up", "road_down", "road_left",
        "road_right",
    ]
    .map(String::from)
    .to_vec()
}

// ---- Backgrounds ---------------------------------------------------------------

/// Background load data by background id (`off_8080F98`, `LoadBGAnimData`):
/// gfx source, gfx dest, tilemap source, tilemap dest offset, palette
/// source, palette dest, palette size. (The backgrounds' code is past
/// where the versions' code parts: its literals are at an address of their
/// own in each ROM.)
const BACKGROUNDS: Place = Place::pointer_in_each([0x0808_0F94, 0x0808_2054, 0x0808_3108, 0x0808_41C0]);
const BACKGROUND_COUNT: u32 = 22;
/// Per background: scroll callbacks (`off_8080E34`, 16 bytes each; the
/// second one scrolls BG1).
const BACKGROUND_SCROLL: Place = Place::pointer_in_each([0x0808_0E30, 0x0808_1EF0, 0x0808_2FA4, 0x0808_405C]);
/// Per background: its GFX animation list (`off_8081220`).
const BACKGROUND_ANIMS: Place = Place::pointer_in_each([0x0808_0E2C, 0x0808_1EEC, 0x0808_2FA0, 0x0808_4058]);

/// Scroll callbacks and their counter steps (1/16 pixel per frame): the
/// same routines at the same addresses in the four ROMs.
const SCROLLERS: [(u32, (i32, i32)); 5] = [
    (0x0800_19B4, (-8, -4)), // BGScrollCB_BG1Diagonal3to2Scroll
    (0x0800_19EC, (0, -4)),  // BGScrollCB_BG1UpScroll
    (0x0800_1A10, (0, 4)),   // BGScrollCB_BG1DownScroll
    (0x0800_1A34, (1, 0)),   // BGScrollCB_BG1SlowRightScroll
    (0x0800_1A58, (-8, 0)),  // BGScrollCB_BG1FastLeftScroll
];

fn backgrounds(src: Source) -> Vec<Option<Background>> {
    let rom = src.rom;
    let (loads, scrolls, anim_lists) = (src.at(BACKGROUNDS), src.at(BACKGROUND_SCROLL), src.at(BACKGROUND_ANIMS));
    (0..BACKGROUND_COUNT)
        .map(|id| {
            let (tiles, first_tile, map, w, h, palette) =
                crate::decode::background_picture(rom, u32at(rom, loads + 4 * id))?;
            let cb = u32at(rom, scrolls + 16 * id + 4) & !1;
            let scroll = SCROLLERS
                .iter()
                .find(|(a, _)| *a == cb)
                .map(|(_, v)| *v)
                .unwrap_or((0, 0));
            let anims = gfx_anims(rom, u32at(rom, anim_lists + 4 * id), PALETTE_BUFFER);
            Some(Background {
                tiles,
                first_tile,
                map,
                map_width: w,
                map_height: h,
                palette,
                scroll,
                speeding: None,
                anims,
            })
        })
        .collect()
}
