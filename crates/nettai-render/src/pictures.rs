//! A chip's pictures on their own, outside a frame: its icon and its
//! custom screen picture as RGBA images, for a tool that shows them (the
//! build creator). They are looked up as a frame looks them up
//! ([`crate::lookups`]: a chip's game's pack's, under its key there) and
//! colored as a frame colors them.

use crate::audit::Problems;
use crate::compose::to_rgb;
use crate::graphics::Graphics;
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
pub fn chip_icon(graphics: &Graphics, c: &Content, chip: ChipHandle) -> Option<Image> {
    let (tiles, palette) = crate::lookups::chip_icon(graphics, c, chip, &mut Problems::default())?;
    image(tiles, 2, 2, palette)
}

/// A chip's 56x48 picture in the chip window, in its definition's
/// `art_palette` where it has one (a chip whose palette no ROM holds), else
/// its own.
pub fn chip_art(graphics: &Graphics, c: &Content, chip: ChipHandle) -> Option<Image> {
    let art = crate::lookups::chip_art(graphics, c, chip, &mut Problems::default())?;
    image(&art.picture.tiles, 7, 6, &c.chip(chip).art_palette.unwrap_or(art.picture.palette))
}

/// The palette a chip family's element icon is drawn in: the icons' with
/// the family's six colors from color 10, or the icon's own where the pack
/// draws it as a sprite. None: the pack has no icon for the family.
fn element_palette(a: &nettai_assets::CustomScreen, family: usize) -> Option<Palette> {
    match &a.element_sprite {
        Some(sprite) => Some(sprite.palette),
        None => {
            let mut palette = a.icon_palette;
            palette[10..].copy_from_slice(a.element_colors.get(family)?);
            Some(palette)
        }
    }
}

/// A chip family's 16x16 element icon, as the game's pack's chip window
/// shows it for a chip of the family: its frame with its symbol over it
/// (`CustomScreen::element_icon`), in its palette. None: the game's graphics
/// have no icon for the family.
pub fn element_icon(graphics: &Graphics, family: nettai_battle::content::ChipFamily) -> Option<Image> {
    let a = &graphics.bundle().custom;
    let family = family.0 as usize;
    let palette = element_palette(a, family)?;
    image(&a.element_icon(family)?, 2, 2, &palette)
}

/// A chip family's element icon as an SVG of 16x16 frame pixels: the frame
/// the pack draws (`shapes.element_frame`) in the family's colors, smooth
/// at any size, with the symbol over it as its pixels, square-edged (the
/// icon's pixels inside the frame's ring: `CustomScreen::element_inside`).
/// The user: "you should draw the element icons in the ui with the smooth
/// frame as well". None: the graphics have no drawing of the frame, or no icon
/// for the family ([`element_icon`] is the picture).
pub fn element_icon_svg(graphics: &Graphics, family: nettai_battle::content::ChipFamily) -> Option<String> {
    let a = &graphics.bundle().custom;
    let family = family.0 as usize;
    let d = a.shapes.element_frame.as_ref().filter(|d| d.size == [16, 16])?;
    let palette = element_palette(a, family)?;
    let inside = a.element_inside()?;
    let icon = a.elements.pixels.get(4 * family * Tiles::TILE..4 * (family + 1) * Tiles::TILE)?;
    let hex = |c: u16| format!("#{:06x}", to_rgb(c));
    // (A pixel's index in the icon, by x and y; a symbol's pixel's index,
    // none outside the ring.)
    let at = |x: usize, y: usize| (2 * (y / 8) + x / 8) * Tiles::TILE + 8 * (y % 8) + x % 8;
    let symbol = |x: usize, y: usize| Some(icon[at(x, y)]).filter(|&p| p != 0 && inside[at(x, y)]);
    // (Each row's runs of one color, a rectangle each.)
    let mut rects = String::new();
    for y in 0..16 {
        let mut x = 0;
        while x < 16 {
            let Some(p) = symbol(x, y) else {
                x += 1;
                continue;
            };
            let end = (x + 1..16).find(|&e| symbol(e, y) != Some(p)).unwrap_or(16);
            rects += &format!("<rect x=\"{x}\" y=\"{y}\" width=\"{}\" height=\"1\" fill=\"{}\"/>", end - x, hex(palette[p as usize & 15]));
            x = end;
        }
    }
    let frame = nettai_assets::shapes::recolor(&d.source, &|_, k| palette[k as usize & 15]);
    let end = frame.rfind("</svg>")?;
    Some(format!("{}<g shape-rendering=\"crispEdges\">{rects}</g>\n{}", &frame[..end], &frame[end..]))
}

/// A form's face in the emotion window (its definition's mugshot, the
/// plain emotion's), 32x16, in its first palette: a soul's or a Cross's
/// picture for a tool.
pub fn form_face(graphics: &Graphics, c: &Content, form: nettai_content_api::FormHandle) -> Option<Image> {
    // (The emotion when nothing else holds: the plain one.)
    let (_, face) = crate::lookups::form_face(graphics, c, form, nettai_battle::content::Emotion::default(), false, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}

/// A form's face in the emotion window for `emotion` (by its name in the
/// game's rules), of its second set with `variant`, 32x16, in its first
/// palette: what a round shows of a navi, for a tool.
pub fn face(graphics: &Graphics, c: &Content, form: nettai_content_api::FormHandle, emotion: &str, variant: bool) -> Option<Image> {
    let emotion = c.rules().emotion.by_name(emotion)?;
    let (_, face) = crate::lookups::form_face(graphics, c, form, emotion, variant, &mut Problems::default());
    let (tiles, palettes) = face?;
    image(tiles, 4, 2, palettes.first()?)
}
