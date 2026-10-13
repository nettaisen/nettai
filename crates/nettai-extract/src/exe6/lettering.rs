//! The Japanese ROMs' lettering (`ROCKEXE6_RXXBR6J`, `ROCKEXE6_GXXBR5J`):
//! the pack's Japanese, beside the US ROMs' English. The same code reaches
//! each block as in the US ROMs (the HUD's and the custom screen's places),
//! and both Japanese ROMs have the same words but for the forms' names,
//! which are each version's own.

use crate::exe6::{Roms, Version};
use nettai_assets::{ButtonLettering, CustomLettering, Hud, HudLettering, SlotPictures};
use nettai_content::names::AssetNames;

pub const LANGUAGE: &str = "ja";

/// The Japanese ROMs' notice of the other player customizing, on the HUD
/// layer: fourteen tiles, for the US ROMs' sixteen.
const WAITING_BYTES: usize = 0x1C0;

/// The HUD's Japanese words, from a Japanese ROM: placeholders without
/// one. A banner is kept where it isn't the base's (`base`: the pack's own
/// words, a US ROM's).
pub fn hud(roms: &Roms, base: &Hud, names: &AssetNames) -> HudLettering {
    let jp = roms.jp();
    if !jp.is_present() {
        return crate::placeholders::lettering(base, names, LANGUAGE);
    }
    use crate::exe6::hud::{
        BANNER_PALETTE, FONT, FONT_GLYPHS, GAUGE_BYTES, GAUGE_TILES, TEXTS, WAITING, banner, dialogue_font, palette, texts, tiles,
    };
    let (rom, at) = (jp.rom, |place| jp.at(place));
    let (cell, dialogue) = names
        .language_glyphs
        .get(LANGUAGE)
        .cloned()
        .unwrap_or_default();
    let base_present = roms.us().is_present();
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
        font: tiles(rom, at(FONT), 0x40 * FONT_GLYPHS as usize),
        font_chars: cell.iter().take(FONT_GLYPHS as usize).cloned().collect(),
        dialogue_font: dialogue_font(jp, (&cell, &dialogue)),
        texts: texts(rom, at(TEXTS)),
        banners,
        banner_palette: palette(rom, at(BANNER_PALETTE)),
        waiting: tiles(rom, at(WAITING), WAITING_BYTES),
        waiting_palette: palette(rom, at(BANNER_PALETTE)),
        gauge_tiles: tiles(rom, at(GAUGE_TILES), GAUGE_BYTES),
        chatbox_text: None,
    }
}

/// The custom screen's Japanese words: the chip window's pictures from a
/// Japanese ROM, each version's forms' names from its own.
pub fn custom(roms: &Roms) -> CustomLettering {
    use crate::exe6::custom::{FORM_NAMES, OK, OK_PICKED, OTHER, REDEAL, REDEAL_BUTTON, SCRAP, SCRAP_BUTTON, picture, tiles};
    let jp = roms.jp();
    let button = |name: &str, details| {
        (
            name.to_string(),
            ButtonLettering {
                tiles: None,
                picture: Some(picture(jp, details)),
            },
        )
    };
    CustomLettering {
        pictures: SlotPictures {
            ok: picture(jp, OK),
            ok_picked: picture(jp, OK_PICKED),
            other: picture(jp, OTHER),
        },
        form_names: [Version::Falzar, Version::Gregar]
            .map(|v| (v.name().to_string(), tiles(roms.jp_of(v), FORM_NAMES)))
            .into(),
        buttons: vec![button(REDEAL_BUTTON, REDEAL), button(SCRAP_BUTTON, SCRAP)],
    }
}
