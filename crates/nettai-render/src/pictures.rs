//! A chip's pictures on their own, outside a frame: its icon and its
//! custom screen picture as RGBA images, for a tool that shows them (the
//! build creator). They are looked up as a frame looks them up
//! ([`crate::lookups`]: a chip's game's pack's, under its key there) and
//! colored as a frame colors them.

use crate::audit::Problems;
use crate::compose::to_rgb;
use crate::packs::Packs;
use nettai_assets::{Palette, Tiles};
use nettai_battle::Content;
use nettai_content_api::ChipHandle;

/// An RGBA image, row by row; index 0 of a palette is see-through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// `tiles` laid out row by row, `w` by `h` tiles, in `palette`; none if
/// there are fewer tiles.
pub fn image(tiles: &Tiles, w: usize, h: usize, palette: &Palette) -> Option<Image> {
    if tiles.len() < w * h {
        return None;
    }
    let (pw, ph) = (w * 8, h * 8);
    let mut rgba = vec![0u8; pw * ph * 4];
    for t in 0..w * h {
        let tile = tiles.get(t)?;
        let (tx, ty) = (t % w * 8, t / w * 8);
        for (i, &p) in tile.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let (x, y) = (tx + i % 8, ty + i / 8);
            let c = to_rgb(palette[p as usize]);
            rgba[(y * pw + x) * 4..][..4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8, 0xFF]);
        }
    }
    Some(Image { width: pw as u32, height: ph as u32, rgba })
}

/// A chip's 16x16 icon, in its pack's HUD's icon palette (the slots'
/// unpicked look).
pub fn chip_icon(packs: &Packs, c: &Content, chip: ChipHandle) -> Option<Image> {
    let (tiles, palette) = crate::lookups::chip_icon(packs, c, chip, &mut Problems::default())?;
    image(tiles, 2, 2, palette)
}

/// A chip's 56x48 picture in the chip window, in its definition's
/// `art_palette` where it has one (a chip whose palette no ROM holds), else
/// its own.
pub fn chip_art(packs: &Packs, c: &Content, chip: ChipHandle) -> Option<Image> {
    let art = crate::lookups::chip_art(packs, c, chip, &mut Problems::default())?;
    image(&art.picture.tiles, 7, 6, &c.chip(chip).art_palette.unwrap_or(art.picture.palette))
}

/// A form's face in the emotion window (its definition's mugshot, the
/// plain emotion's), 32x16, in its first palette: a soul's or a Cross's
/// picture for a tool.
pub fn form_face(packs: &Packs, c: &Content, form: nettai_content_api::FormHandle) -> Option<Image> {
    // (The emotion when nothing else holds: the plain one.)
    let (_, face) = crate::lookups::form_face(packs, c, form, nettai_battle::content::Emotion::default(), false, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}

/// A form's face in the emotion window for `emotion` (by its name in the
/// game's rules), of its second set with `variant`, 32x16, in its first
/// palette: what a round shows of a navi, for a tool.
pub fn face(packs: &Packs, c: &Content, form: nettai_content_api::FormHandle, emotion: &str, variant: bool) -> Option<Image> {
    let emotion = c.rules().emotion.by_name(emotion)?;
    let (_, face) = crate::lookups::form_face(packs, c, form, emotion, variant, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}
