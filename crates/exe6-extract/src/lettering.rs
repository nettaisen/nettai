//! The pack's Japanese lettering, from the Japanese ROMs: what a battle
//! shows in words in the Japanese games, where the US games show English
//! (nettai-assets `lettering`; docs/design/text-rendering.md §10).
//!
//! - **The fonts**: the 8x16 font and the dialogue font with its advances,
//!   in the Japanese encoding (compat/text.toml's `[jp]`).
//! - **The HUD's text lines**: the same words as the US ROMs' ("TIME
//!   UP!", "COUNTER HIT!"), in the Japanese font's glyphs.
//! - **Banners**: the ten whose words differ (ROCKMAN, KILLERMAN, AQUAMAN
//!   and BLUES for MegaMan, EraseMan, SpoutMan and ProtoMan; the Program
//!   Advance's katakana), each where it starts (KILLERMAN DELETED further
//!   left), and the banners' palette.
//! - **"Cstmzing..."**: カスタム中…, seven tiles wide where the US's is
//!   eight (`sub_801CA34` copies one column fewer).
//! - **The custom gauge** (its "L or R").
//! - **The chip window's pictures** for OK ("no data selected", "chip data
//!   transmission"), the re-deal and the scrap buttons.
//! - **The Cross window's names**, by game: Falzar's from the Japanese
//!   Falzar ROM, Gregar's from the Japanese Gregar ROM.
//!
//! Everything else a battle shows is the same pictures in all four ROMs
//! (tools/jp/locale/pictures.py in the verification workspace compares
//! them). The two Japanese ROMs' fonts, lines, banners and pictures are the
//! same; only the Cross names are by game. The addresses are the Japanese
//! ROMs' own (their data moved against the US ROMs').

use crate::Rom;
use nettai_assets::{CustomLettering, Hud, HudLettering, SlotPictures};
use nettai_content::names::AssetNames;

/// The language the Japanese ROMs' lettering is.
pub const LANGUAGE: &str = "ja";

/// The 8x16 font (`dword_86B7AE0` in the US ROMs).
const FONT: u32 = 0x086D_6C00;
/// The dialogue font (`byte_86ACD60`) and its advances (`byte_8043CA4`).
const DIALOGUE_FONT: u32 = 0x086C_BE80;
const DIALOGUE_ADVANCES: u32 = 0x0804_4EEC;
/// The HUD's text lines (`TextScript86F0374`).
const TEXTS: u32 = 0x0871_2DB8;
/// The banners (`pt_801EF84`), their glyph filler and palette.
const BANNERS: (u32, u32) = (0x0801_F3A8, 0x0802_01D4);
const BANNER_PALETTE: u32 = 0x0871_5120;
/// "Cstmzing..." (`sub_801EC90`'s transfer; 7x2 tiles).
const WAITING: (u32, usize) = (0x0871_48E0, 0x1C0);
/// The gauge's tiles (`sub_801DED0`).
const GAUGE_TILES: (u32, usize) = (0x0870_72E4, 0x380);
/// The chip window's pictures and their palettes (as the US ROMs'
/// `OK`, `OK_PICKED`, `OTHER`, `REDEAL`, `SCRAP`).
const OK: (u32, u32) = (0x0874_7098, 0x0874_9FB8);
const OK_PICKED: (u32, u32) = (0x0874_6B58, 0x0874_9F38);
const OTHER: (u32, u32) = (0x0874_6B58, 0x0874_9F18);
const REDEAL: (u32, u32) = (0x0874_7098, 0x0874_9F98);
const SCRAP: (u32, u32) = (0x0875_8698, 0x0875_8BF8);
/// The Cross window's names (`dword_86E7DCC`): the Japanese Falzar ROM's
/// and the Japanese Gregar ROM's.
const CROSS_NAMES_FALZAR: u32 = 0x0870_A814;
const CROSS_NAMES_GREGAR: u32 = 0x0870_8774;

/// The HUD's Japanese lettering; `base` is the pack's own HUD (a banner
/// whose glyphs are the same is left to it).
pub fn hud(jp: &Rom, base: &Hud, names: &AssetNames) -> HudLettering {
    use crate::hud::{FONT_GLYPHS, banner_at, dialogue_font, palette, texts, tiles};
    let (cell, dialogue) = names.language_glyphs.get(LANGUAGE).cloned().unwrap_or_default();
    let banners = base
        .banners
        .iter()
        .enumerate()
        .map(|(id, own)| {
            let b = banner_at(jp, BANNERS, id as u32);
            // The words differ, and where a banner starts with them (a
            // longer name further left); not what kind of banner it is.
            assert_eq!(
                (b.kind, b.number_at, b.glyphs.len()),
                (own.kind, own.number_at, own.glyphs.len()),
                "the Japanese ROM's banner {:#04x} is another kind",
                4 * id
            );
            (b != *own).then_some(b)
        })
        .collect();
    HudLettering {
        font: tiles(jp, FONT, 0x40 * FONT_GLYPHS as usize),
        font_chars: cell.iter().take(FONT_GLYPHS as usize).cloned().collect(),
        dialogue_font: dialogue_font(jp, (DIALOGUE_FONT, DIALOGUE_ADVANCES), (&cell, &dialogue)),
        texts: texts(jp, TEXTS),
        banners,
        banner_palette: palette(jp, BANNER_PALETTE),
        waiting: tiles(jp, WAITING.0, WAITING.1),
        waiting_palette: palette(jp, BANNER_PALETTE),
        gauge_tiles: tiles(jp, GAUGE_TILES.0, GAUGE_TILES.1),
    }
}

/// The custom screen's Japanese lettering.
pub fn custom(falzar: &Rom, gregar: &Rom) -> CustomLettering {
    use crate::custom::{CROSS_NAMES, picture};
    let names = |rom: &Rom, a: u32| crate::hud::tiles(rom, a, CROSS_NAMES.0 * CROSS_NAMES.1);
    CustomLettering {
        pictures: SlotPictures {
            ok: picture(falzar, OK),
            ok_picked: picture(falzar, OK_PICKED),
            redeal: picture(falzar, REDEAL),
            scrap: picture(falzar, SCRAP),
            other: picture(falzar, OTHER),
        },
        cross_names: vec![("falzar".into(), names(falzar, CROSS_NAMES_FALZAR)), ("gregar".into(), names(gregar, CROSS_NAMES_GREGAR))],
    }
}
