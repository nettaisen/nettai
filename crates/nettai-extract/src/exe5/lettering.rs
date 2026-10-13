//! The pack's Japanese lettering, from the Japanese ROMs: what a battle
//! shows in words in the Japanese games, where the US games show English
//! (nettai-assets `lettering`; docs/design/text-rendering.md §10), as
//! the EXE6 module's `lettering` has EXE6's.
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
//! backgrounds a netbattle has, the sprites; and its frame comparison knows
//! what a Japanese console shows otherwise: background 0x05, the battle mode
//! 1 label 14-17). Team of Blues' and Team of Colonel's lettering is the
//! same: the pack's is whichever's is given. The same code reaches each
//! block as in the US ROMs: hud.rs's and custom.rs's places.

use crate::exe5::rom::Roms;
use nettai_assets::{ButtonLettering, CustomLettering, Hud, HudLettering, SlotPictures};
use nettai_content::names::AssetNames;

/// The language the Japanese ROMs' lettering is.
pub const LANGUAGE: &str = "ja";

/// The Japanese ROMs' notice of the other player customizing: seven tiles
/// wide, for the US ROMs' eight.
const WAITING_BYTES: usize = 0x1C0;

/// The HUD's Japanese lettering, from a Japanese ROM: placeholders without
/// one, but the gauge's tiles, which are the same in the four ROMs. `base`
/// is the pack's own HUD (a banner whose glyphs are the same is left to
/// it).
pub fn hud(roms: &Roms, base: &Hud, names: &AssetNames) -> HudLettering {
    use crate::exe5::hud::{
        BANNER_COUNT, BANNER_PALETTE, FONT, FONT_GLYPHS, GAUGE_BYTES, GAUGE_TILES, TEXTS, WAITING, banner, dialogue_font, palette, texts,
        tiles,
    };
    let (jp, any) = (roms.jp(), roms.any());
    let gauge_tiles = if any.is_present() {
        tiles(any.rom, any.at(GAUGE_TILES), GAUGE_BYTES)
    } else {
        crate::placeholders::tiles(GAUGE_BYTES / 32)
    };
    if !jp.is_present() {
        return HudLettering { gauge_tiles, ..crate::placeholders::lettering(base, names, LANGUAGE) };
    }
    let (rom, at) = (jp.rom, |place| jp.at(place));
    let base_present = roms.us().is_present();
    let (cell, dialogue) = names
        .language_glyphs
        .get(LANGUAGE)
        .cloned()
        .unwrap_or_default();
    assert_eq!(base.banners.len(), BANNER_COUNT as usize);
    let banners = base
        .banners
        .iter()
        .enumerate()
        .map(|(id, own)| {
            let b = banner(jp, id as u32);
            if base_present {
                assert_eq!(
                    (b.kind, b.number_at, b.glyphs.len()),
                    (own.kind, own.number_at, own.glyphs.len()),
                    "the Japanese ROM's banner {:#04x} is another kind",
                    4 * id
                );
            }
            (b != *own).then_some(b)
        })
        .collect();
    HudLettering {
        font: tiles(rom, at(FONT), 0x40 * FONT_GLYPHS),
        font_chars: (0..FONT_GLYPHS)
            .map(|k| cell.get(k).cloned().unwrap_or_else(|| format!("[{k:03x}]")))
            .collect(),
        dialogue_font: dialogue_font(jp, (&cell, &dialogue)),
        texts: texts(rom, at(TEXTS)),
        banners,
        banner_palette: palette(rom, at(BANNER_PALETTE)),
        waiting: tiles(rom, at(WAITING), WAITING_BYTES),
        waiting_palette: palette(rom, at(BANNER_PALETTE)),
        gauge_tiles,
        chatbox_text: None,
    }
}

/// The custom screen's Japanese lettering, from a Japanese ROM: the chip
/// window's worded pictures and the soul button's label. (Arm Change's
/// picture, which the lettering has too, has no words: any ROM's.)
pub fn custom(roms: &Roms) -> CustomLettering {
    use crate::exe5::custom::{
        ARM_CHANGE_BUTTON, OK, OK_PICKED, OTHER, REDEAL, REDEAL_BUTTON, SCRAP, SOUL_BUTTON, SOUL_BUTTONS, picture, tiles,
    };
    let (jp, any) = (roms.jp(), roms.any());
    CustomLettering {
        pictures: SlotPictures {
            ok: picture(jp, OK),
            ok_picked: picture(jp, OK_PICKED),
            other: picture(jp, OTHER),
        },
        form_names: Vec::new(),
        buttons: vec![
            (
                ARM_CHANGE_BUTTON.into(),
                ButtonLettering {
                    tiles: None,
                    picture: Some(picture(any, SCRAP)),
                },
            ),
            (
                REDEAL_BUTTON.into(),
                ButtonLettering {
                    tiles: None,
                    picture: Some(picture(jp, REDEAL)),
                },
            ),
            (
                SOUL_BUTTON.into(),
                ButtonLettering {
                    tiles: Some(tiles(jp, SOUL_BUTTONS)),
                    picture: None,
                },
            ),
        ],
    }
}
