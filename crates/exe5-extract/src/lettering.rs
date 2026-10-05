//! The pack's Japanese lettering, from the Japanese ROMs: what a battle
//! shows in words in the Japanese games, where the US games show English
//! (nettai-assets `lettering`; docs/design/text-rendering.md §10), as
//! exe6-extract's `lettering` has EXE6's.
//!
//! - **The fonts**: the 8x16 font and the dialogue font with its advances,
//!   in the Japanese encoding (compat/text.toml's `[jp]`).
//! - **The HUD's text lines**, in the Japanese font's glyphs.
//! - **Banners**: the ten whose words differ (ROCKMAN and BLUES for MegaMan
//!   and ProtoMan, the Program Advance's and the others' katakana), each
//!   where it starts, and the banners' palette.
//! - **"Cstmzing..."**: カスタム中…, seven tiles wide where the US's is
//!   eight.
//! - **The custom gauge** (the same tiles as the US ROMs', by the Japanese
//!   ROMs' own address).
//! - **The chip window's pictures** for OK ("no data selected", "chip data
//!   transmission") and the re-deal button, with their palettes; the scrap
//!   button's is the US ROMs'.
//! - **The soul button**: "uni son" in two rows where the US's says
//!   "UNITE".
//!
//! Everything else a battle shows is the same pictures in all four ROMs
//! (the verification workspace's tools/exe5/jpassets.py compares them: the
//! HUD's and the custom screen's blocks, every chip's icon and picture, the
//! backgrounds a netbattle has; the sprites `graphics::japanese_differences`
//! does). Team of Blues' and Team of Colonel's lettering is the same, which
//! is checked here: the pack's is Team of Blues'. The addresses are the
//! Japanese ROMs' own (their data moved against the US ROMs'): where the
//! routines the two builds share load what the US ROMs' load at
//! hud.rs's and custom.rs's addresses.

use crate::rom::{Rom, Roms, Version};
use nettai_assets::{CustomLettering, Hud, HudLettering, SlotPictures};
use nettai_content::names::AssetNames;

/// The language the Japanese ROMs' lettering is.
pub const LANGUAGE: &str = "ja";

/// Where a Japanese ROM has its words.
struct Addresses {
    /// The 8x16 font (the US Team ProtoMan ROM's 0x086CBA68).
    font: u32,
    /// The dialogue font (0x086C14C8) and its advances (0x080426B4).
    dialogue_font: (u32, u32),
    /// The HUD's text lines (0x0873ACF0).
    texts: u32,
    /// The banners and their glyph filler (0x0801B810, 0x0801C6F4), and
    /// their palette (0x0873D1F8).
    banners: (u32, u32),
    banner_palette: u32,
    /// "Cstmzing..." (the first of the three transfer lists before the
    /// banners: 7x2 tiles here).
    waiting: (u32, usize),
    /// The gauge's tiles (0x086FA2CC).
    gauge_tiles: u32,
    /// The chip window's pictures and their palettes (as custom.rs's `OK`,
    /// `OK_PICKED`, `OTHER`, `REDEAL`, `SCRAP`).
    ok: (u32, u32),
    ok_picked: (u32, u32),
    other: (u32, u32),
    redeal: (u32, u32),
    scrap: (u32, u32),
    /// The soul button's tiles (0x086FBB64).
    soul_buttons: u32,
}

/// Team of Blues' (`ROCKEXE5_TOB`, BRBJ).
const BLUES: Addresses = Addresses {
    font: 0x086E_20FC,
    dialogue_font: (0x086D_7B5C, 0x0804_25C0),
    texts: 0x0875_133C,
    banners: (0x0801_B7DC, 0x0801_C6B0),
    banner_palette: 0x0875_3638,
    waiting: (0x0875_2DF8, 0x1C0),
    gauge_tiles: 0x0871_08C4,
    ok: (0x0874_83A0, 0x0874_B3A0),
    ok_picked: (0x0874_7E60, 0x0874_B320),
    other: (0x0874_7E60, 0x0874_B300),
    redeal: (0x0874_83A0, 0x0874_B380),
    scrap: (0x0876_533C, 0x0876_589C),
    soul_buttons: 0x0871_215C,
};

/// Team of Colonel's (`ROCKEXE5_TOC`, BRKJ): the same data, 0x1470 bytes
/// further on mostly.
const COLONEL: Addresses = Addresses {
    font: 0x086E_3590,
    dialogue_font: (0x086D_8FF0, 0x0804_25C8),
    texts: 0x0875_27AC,
    banners: (0x0801_B7D8, 0x0801_C6AC),
    banner_palette: 0x0875_4AA8,
    waiting: (0x0875_4268, 0x1C0),
    gauge_tiles: 0x0871_1D34,
    ok: (0x0874_9810, 0x0874_C810),
    ok_picked: (0x0874_92D0, 0x0874_C790),
    other: (0x0874_92D0, 0x0874_C770),
    redeal: (0x0874_9810, 0x0874_C7F0),
    scrap: (0x0876_67DC, 0x0876_6D3C),
    soul_buttons: 0x0871_35CC,
};

/// A Japanese ROM's HUD lettering; `base` is the pack's own HUD (a banner
/// whose glyphs are the same is left to it).
fn hud_of(rom: &Rom, a: &Addresses, base: &Hud, names: &AssetNames) -> HudLettering {
    use crate::hud::{BANNER_COUNT, FONT_GLYPHS, GAUGE_BYTES, banner_at, dialogue_font, palette, texts, tiles};
    let (cell, dialogue) = names.language_glyphs.get(LANGUAGE).cloned().unwrap_or_default();
    assert_eq!(base.banners.len(), BANNER_COUNT as usize);
    let banners = base
        .banners
        .iter()
        .enumerate()
        .map(|(id, own)| {
            let b = banner_at(rom, a.banners, id as u32);
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
        font: tiles(rom, a.font, 0x40 * FONT_GLYPHS),
        font_chars: (0..FONT_GLYPHS).map(|k| cell.get(k).cloned().unwrap_or_else(|| format!("[{k:03x}]"))).collect(),
        dialogue_font: dialogue_font(rom, a.dialogue_font, (&cell, &dialogue)),
        texts: texts(rom, a.texts),
        banners,
        banner_palette: palette(rom, a.banner_palette),
        waiting: tiles(rom, a.waiting.0, a.waiting.1),
        waiting_palette: palette(rom, a.banner_palette),
        gauge_tiles: tiles(rom, a.gauge_tiles, GAUGE_BYTES),
    }
}

/// A Japanese ROM's custom-screen lettering.
fn custom_of(rom: &Rom, a: &Addresses) -> CustomLettering {
    use crate::custom::{SOUL_BUTTON, SOUL_BUTTON_BYTES, picture};
    CustomLettering {
        pictures: SlotPictures {
            ok: picture(rom, a.ok),
            ok_picked: picture(rom, a.ok_picked),
            redeal: picture(rom, a.redeal),
            scrap: picture(rom, a.scrap),
            other: picture(rom, a.other),
        },
        // (EXE5 has no Cross window.)
        cross_names: Vec::new(),
        buttons: vec![(SOUL_BUTTON.into(), crate::hud::tiles(rom, a.soul_buttons, SOUL_BUTTON_BYTES))],
    }
}

/// The HUD's Japanese lettering (Team of Blues'; Team of Colonel's is the
/// same).
pub fn hud(roms: &Roms, base: &Hud, names: &AssetNames) -> HudLettering {
    let l = hud_of(roms.jp(Version::ProtoMan), &BLUES, base, names);
    assert!(l == hud_of(roms.jp(Version::Colonel), &COLONEL, base, names), "Team of Colonel's HUD lettering is another");
    l
}

/// The custom screen's Japanese lettering (likewise).
pub fn custom(roms: &Roms) -> CustomLettering {
    let l = custom_of(roms.jp(Version::ProtoMan), &BLUES);
    assert!(l == custom_of(roms.jp(Version::Colonel), &COLONEL), "Team of Colonel's custom-screen lettering is another");
    l
}
