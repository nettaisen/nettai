//! What a language changes of the HUD and the custom screen: the fonts
//! with what their glyphs draw, the HUD's text lines, and the pictures with
//! words in them (banners, "Cstmzing...", the gauge's label, the chip
//! window's pictures for the slots that aren't chips, the Cross window's
//! names, a named button's label). The pack's own (its base language's, `Hud::language`) are in
//! the HUD's and the custom screen's fields; another language's are kept
//! beside them (`Hud::languages`, `CustomScreen::languages`) and swapped
//! in by [`Bundle::in_language`], so what draws the HUD draws whichever
//! language the bundle is in. Strings that content defines (chip names,
//! descriptions) aren't assets: their translations are the content root's
//! (docs/design/text-rendering.md §10).

use crate::{BannerLayout, Bundle, CustomScreen, DialogueFont, Hud, Palette, Picture, SlotPictures, Tiles};

/// The language of a pack that doesn't say (one extracted from the US
/// ROMs).
pub const BASE_LANGUAGE: &str = "en";

/// The HUD's lettering in a language other than the pack's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HudLettering {
    /// The 8x16 font and what each glyph draws (`Hud::font`,
    /// `Hud::font_chars`).
    pub font: Tiles,
    pub font_chars: Vec<String>,
    pub dialogue_font: DialogueFont,
    /// The HUD's text lines in this font's glyphs (`Hud::texts`).
    pub texts: Vec<Vec<u16>>,
    /// The banners whose words differ, by banner id / 4 (none: the pack's
    /// own banner), each with its place (a longer name starts further
    /// left), and the banners' palette.
    pub banners: Vec<Option<BannerLayout>>,
    pub banner_palette: Palette,
    /// "Cstmzing..." (two rows of tiles, as wide as its words) and its
    /// palette.
    pub waiting: Tiles,
    pub waiting_palette: Palette,
    /// The custom gauge's tiles (its "L or R").
    pub gauge_tiles: Tiles,
}

/// The custom screen's lettering in a language other than the pack's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CustomLettering {
    /// The chip window's pictures for the slots that are neither chips
    /// nor buttons (OK's).
    pub pictures: SlotPictures,
    /// The form list window's names (`VersionPictures::form_names`) by game
    /// version; a version not listed keeps the pack's own.
    pub form_names: Vec<(String, Tiles)>,
    /// The named buttons that say something, by the button's name (in the
    /// names' order, as a pack keeps them); a button not listed keeps the
    /// pack's own.
    pub buttons: Vec<(String, ButtonLettering)>,
}

/// What a language has of its own for a named button: its tiles
/// (`ButtonPictures::tiles`: EXE5's soul button, "UNITE" in English) and
/// its picture in the chip window (`ButtonPictures::picture`: the re-deal
/// button's); none: the pack's own.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ButtonLettering {
    pub tiles: Option<Tiles>,
    pub picture: Option<Picture>,
}

impl Hud {
    /// The language the HUD's lettering is in.
    pub fn language(&self) -> &str {
        if self.language.is_empty() { BASE_LANGUAGE } else { &self.language }
    }

    /// Swap `lang`'s lettering in (the one in use goes to its place among
    /// `languages`). False if the HUD has no lettering in `lang`.
    pub fn in_language(&mut self, lang: &str) -> bool {
        if lang == self.language() {
            return true;
        }
        let Some(k) = self.languages.iter().position(|(l, _)| l == lang) else { return false };
        let (_, mut l) = self.languages.remove(k);
        let own = self.language().to_string();
        std::mem::swap(&mut self.font, &mut l.font);
        std::mem::swap(&mut self.font_chars, &mut l.font_chars);
        std::mem::swap(&mut self.dialogue_font, &mut l.dialogue_font);
        std::mem::swap(&mut self.texts, &mut l.texts);
        for (b, own) in self.banners.iter_mut().zip(l.banners.iter_mut()) {
            if let Some(own) = own {
                std::mem::swap(b, own);
            }
        }
        std::mem::swap(&mut self.banner_palette, &mut l.banner_palette);
        std::mem::swap(&mut self.waiting, &mut l.waiting);
        std::mem::swap(&mut self.waiting_palette, &mut l.waiting_palette);
        std::mem::swap(&mut self.gauge_tiles, &mut l.gauge_tiles);
        self.languages.push((own, l));
        self.language = lang.to_string();
        true
    }

    /// The languages the HUD has lettering in, its own first.
    pub fn languages(&self) -> Vec<&str> {
        std::iter::once(self.language()).chain(self.languages.iter().map(|(l, _)| l.as_str())).collect()
    }
}

impl CustomScreen {
    /// Swap `from`'s lettering out for `to`'s (the screen keeps no name for
    /// its own language: the HUD's is the bundle's). False if it has none
    /// in `to`.
    fn in_language(&mut self, from: &str, to: &str) -> bool {
        let Some(k) = self.languages.iter().position(|(l, _)| l == to) else { return false };
        let (_, mut l) = self.languages.remove(k);
        std::mem::swap(&mut self.pictures, &mut l.pictures);
        for (version, names) in &mut l.form_names {
            let own = if *version == self.versioned.base_version {
                Some(&mut self.versioned.base)
            } else {
                self.versioned.versions.iter_mut().find(|(v, _)| v == version).map(|(_, p)| p)
            };
            if let Some(p) = own {
                std::mem::swap(&mut p.form_names, names);
            }
        }
        for (name, own) in &mut l.buttons {
            let Some((_, b)) = self.buttons.iter_mut().find(|(n, _)| n == name) else { continue };
            if let Some(tiles) = &mut own.tiles {
                std::mem::swap(&mut b.tiles, tiles);
            }
            if let Some(picture) = &mut own.picture {
                std::mem::swap(&mut b.picture, picture);
            }
        }
        self.languages.push((from.to_string(), l));
        true
    }
}

impl Bundle {
    /// The bundle in another language: `lang`'s fonts, HUD lines and
    /// pictures with words where the pack's own were. The pack's own
    /// language is the bundle as it is.
    pub fn in_language(mut self, lang: &str) -> Result<Bundle, String> {
        let own = self.hud.language().to_string();
        if lang == own {
            return Ok(self);
        }
        if !self.hud.in_language(lang) {
            return Err(format!("the pack has no lettering in {lang:?} (it has {})", self.hud.languages().join(", ")));
        }
        // A bundle without the custom screen's graphics (a test's) has no
        // lettering of them either.
        if !self.custom.is_empty() && !self.custom.in_language(&own, lang) {
            return Err(format!("the pack has the HUD in {lang:?} but not the custom screen (extract it again)"));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ButtonPictures, VersionPictures, Versioned};

    fn tiles(v: u8, n: usize) -> Tiles {
        Tiles { pixels: vec![v; n * Tiles::TILE] }
    }

    #[test]
    fn a_language_swaps_in_and_back() {
        let hud = Hud {
            font: tiles(1, 2),
            font_chars: vec!["A".into()],
            texts: vec![vec![0]],
            banners: vec![BannerLayout { glyphs: tiles(1, 2), ..Default::default() }, BannerLayout { glyphs: tiles(1, 2), ..Default::default() }],
            waiting: tiles(1, 16),
            gauge_tiles: tiles(1, 1),
            languages: vec![(
                "ja".into(),
                HudLettering {
                    font: tiles(2, 2),
                    font_chars: vec!["ア".into()],
                    texts: vec![vec![5]],
                    banners: vec![None, Some(BannerLayout { x: 4, glyphs: tiles(2, 2), ..Default::default() })],
                    banner_palette: [7; 16],
                    waiting: tiles(2, 14),
                    waiting_palette: [7; 16],
                    gauge_tiles: tiles(2, 1),
                    ..Default::default()
                },
            )],
            ..Hud::default()
        };
        let mut custom = CustomScreen {
            window_tiles: tiles(1, 1),
            pictures: SlotPictures { ok: Picture { tiles: tiles(1, 42), palette: [0; 16] }, ..Default::default() },
            versioned: Versioned::new("falzar", VersionPictures { form_names: tiles(1, 18), ..Default::default() }),
            languages: vec![(
                "ja".into(),
                CustomLettering {
                    pictures: SlotPictures { ok: Picture { tiles: tiles(2, 42), palette: [0; 16] }, ..Default::default() },
                    form_names: vec![("falzar".into(), tiles(2, 18)), ("gregar".into(), tiles(3, 18))],
                    buttons: vec![
                        ("soul".into(), ButtonLettering { tiles: Some(tiles(2, 18)), picture: None }),
                        ("redeal".into(), ButtonLettering { tiles: None, picture: Some(Picture { tiles: tiles(2, 42), palette: [3; 16] }) }),
                    ],
                },
            )],
            buttons: vec![
                ("soul".into(), ButtonPictures { width: 3, height: 2, tiles: tiles(1, 18), ..Default::default() }),
                ("other".into(), ButtonPictures { width: 3, height: 2, tiles: tiles(5, 18), ..Default::default() }),
                (
                    "redeal".into(),
                    ButtonPictures {
                        width: 2,
                        height: 3,
                        tiles: tiles(6, 36),
                        picture: Picture { tiles: tiles(1, 42), palette: [1; 16] },
                        ..Default::default()
                    },
                ),
            ],
            ..Default::default()
        };
        custom.versioned.versions.push(("gregar".into(), VersionPictures { form_names: tiles(4, 18), ..Default::default() }));
        let en = Bundle { hud, custom, ..Bundle::default() };
        assert_eq!(en.hud.languages(), ["en", "ja"]);
        let ja = en.clone().in_language("ja").unwrap();
        assert_eq!((ja.hud.language(), ja.hud.font_chars.as_slice()), ("ja", &["ア".to_string()][..]));
        assert_eq!(ja.hud.banners[0].glyphs, tiles(1, 2), "a banner the language doesn't change stays");
        assert_eq!((ja.hud.banners[1].x, &ja.hud.banners[1].glyphs), (4, &tiles(2, 2)));
        assert_eq!((ja.hud.waiting.len(), ja.hud.banner_palette), (14, [7; 16]));
        assert_eq!(ja.custom.pictures.ok.tiles, tiles(2, 42));
        assert_eq!(ja.custom.versioned.base.form_names, tiles(2, 18));
        assert_eq!(ja.custom.versioned.get("gregar").form_names, tiles(3, 18));
        assert_eq!((&ja.custom.buttons[0].1.tiles, &ja.custom.buttons[1].1.tiles), (&tiles(2, 18), &tiles(5, 18)), "a button's own label, the others as they are");
        let redeal = &ja.custom.buttons[2].1;
        assert_eq!((&redeal.tiles, &redeal.picture), (&tiles(6, 36), &Picture { tiles: tiles(2, 42), palette: [3; 16] }), "a button's own picture, its tiles as they are");
        assert_eq!(ja.hud.languages(), ["ja", "en"]);
        // And back: the pack as it was, but for which language is its own.
        let back = ja.in_language("en").unwrap();
        assert_eq!((&back.hud.font, &back.hud.texts, &back.custom.pictures), (&en.hud.font, &en.hud.texts, &en.custom.pictures));
        assert_eq!(back.custom.versioned, en.custom.versioned);
        assert_eq!(back.custom.buttons, en.custom.buttons);
        assert!(en.clone().in_language("en").is_ok());
        assert!(en.in_language("fr").unwrap_err().contains("en, ja"));
    }
}
