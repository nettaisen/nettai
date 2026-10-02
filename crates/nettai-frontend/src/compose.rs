//! Putting a frame together: full-screen tile layers and sprite parts,
//! each with a priority, combined with the original's rules (a sprite
//! wins over a layer of the same priority; among sprites the lowest
//! priority, then the earliest, wins; semi-transparent sprites blend with
//! what is behind them). Colours are the original's 15-bit BGR555.

use nettai_assets::{Palette, Tiles};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;
pub const PIXELS: usize = WIDTH * HEIGHT;

/// A transparent pixel in a layer (colours use 15 bits).
pub const CLEAR: u16 = 0x8000;

/// A full-screen layer of tiles (the field, the background, the HUD).
#[derive(Clone)]
pub struct Layer {
    /// 0 is frontmost, 3 backmost.
    pub priority: u8,
    /// Breaks ties between layers of equal priority (lower is in front).
    pub order: u8,
    /// The palettes it draws with (which fades reach it).
    pub palettes: Palettes,
    pub pixels: Vec<u16>,
}

impl Layer {
    pub fn new(priority: u8, order: u8) -> Layer {
        Layer { priority, order, palettes: Palettes::Stage, pixels: vec![CLEAR; PIXELS] }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(CLEAR);
    }

    /// Move everything drawn by (dx, dy), as a scroll of the layer by
    /// (-dx, -dy) would; what moves in from past the edges is clear.
    pub fn shift(&mut self, dx: i32, dy: i32) {
        if (dx, dy) == (0, 0) {
            return;
        }
        let old = std::mem::replace(&mut self.pixels, vec![CLEAR; PIXELS]);
        for y in 0..HEIGHT as i32 {
            let sy = y - dy;
            if !(0..HEIGHT as i32).contains(&sy) {
                continue;
            }
            for x in 0..WIDTH as i32 {
                let sx = x - dx;
                if (0..WIDTH as i32).contains(&sx) {
                    self.pixels[y as usize * WIDTH + x as usize] = old[sy as usize * WIDTH + sx as usize];
                }
            }
        }
    }

    /// Draw one 8x8 tile at (x, y), clipped to the screen.
    pub fn draw_tile(&mut self, tile: &[u8], palette: &Palette, x: i32, y: i32, hflip: bool, vflip: bool) {
        for ty in 0..8 {
            let sy = y + ty;
            if !(0..HEIGHT as i32).contains(&sy) {
                continue;
            }
            let row = if vflip { 7 - ty } else { ty } as usize;
            for tx in 0..8 {
                let sx = x + tx;
                if !(0..WIDTH as i32).contains(&sx) {
                    continue;
                }
                let col = if hflip { 7 - tx } else { tx } as usize;
                let i = tile[row * 8 + col];
                if i != 0 {
                    self.pixels[sy as usize * WIDTH + sx as usize] = palette[i as usize] & 0x7FFF;
                }
            }
        }
    }
}

/// One sprite part as the original's hardware would draw it.
#[derive(Clone)]
pub struct SpritePart<'a> {
    /// Screen position, wrapped like the original's (x 0..512, y 0..256).
    pub x: u16,
    pub y: u8,
    pub width: u8,
    pub height: u8,
    pub tiles: &'a Tiles,
    /// First tile; the part's tiles follow row-major (1D layout).
    pub first_tile: usize,
    pub hflip: bool,
    pub vflip: bool,
    pub palette: Palette,
    pub priority: u8,
    /// Semi-transparent: blend weight (of 16) of this sprite over what is
    /// behind it.
    pub alpha: Option<u8>,
    /// Mosaic block size - 1.
    pub mosaic: Option<u8>,
    /// A vertical affine scale about the part's centre: texture rows
    /// step by `n / 256` per screen row (the banners' squash and stretch).
    pub vscale: Option<i32>,
    /// A rotated or scaled sprite (`sub_802FE7A`'s matrices).
    pub affine: Option<Affine>,
}

/// An affine sprite's matrix (8.8 fixed point) and its box: each pixel of
/// the box reads the texture at M x (pixel - the box's centre) + the
/// sprite's centre; a doubled box is twice the sprite's size, centred on
/// the same point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Affine {
    pub pa: i32,
    pub pb: i32,
    pub pc: i32,
    pub pd: i32,
    pub double: bool,
}

/// Screen-wide effects applied after composition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fade {
    #[default]
    None,
    /// Towards black by n/16.
    Black(u8),
    /// Towards white by n/16.
    White(u8),
}

/// Screen-wide fades. The original fades palettes: `stage` is a fade of the
/// background palettes the stage draws with (the background, the field and
/// the backdrop), `hud` one of the HUD layer's, `screen` one of every
/// palette, the sprites' too. Sprites keep their colours through the first
/// two.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fades {
    pub stage: Fade,
    pub hud: Fade,
    pub screen: Fade,
}

/// Which palettes a layer draws with, for [`Fades`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Palettes {
    #[default]
    Stage,
    Hud,
}

/// Combine layers and sprite parts (in hardware order: earlier parts are
/// in front) into a BGR555 frame.
pub fn compose(backdrop: u16, layers: &[&Layer], parts: &[SpritePart], fades: Fades) -> Vec<u16> {
    let backdrop = apply_fade(backdrop, fades.stage);
    // The sprite layer: per pixel the frontmost sprite's colour.
    let mut obj = vec![CLEAR; PIXELS];
    let mut obj_prio = vec![4u8; PIXELS];
    let mut obj_alpha = vec![0xFFu8; PIXELS];
    for p in parts {
        draw_part(p, &mut obj, &mut obj_prio, &mut obj_alpha);
    }
    let mut out = vec![0u16; PIXELS];
    for i in 0..PIXELS {
        // The two frontmost opaque candidates: (priority, rank, colour).
        let mut first: (u8, u8, u16) = (5, 0, backdrop);
        let mut second: (u8, u8, u16) = (5, 0, backdrop);
        let mut consider = |key: (u8, u8), c: u16| {
            if (key.0, key.1) < (first.0, first.1) {
                second = first;
                first = (key.0, key.1, c);
            } else if (key.0, key.1) < (second.0, second.1) {
                second = (key.0, key.1, c);
            }
        };
        if obj[i] != CLEAR {
            consider((obj_prio[i], 0), obj[i]);
        }
        for l in layers {
            let c = l.pixels[i];
            if c != CLEAR {
                let fade = match l.palettes {
                    Palettes::Stage => fades.stage,
                    Palettes::Hud => fades.hud,
                };
                consider((l.priority, 1 + l.order), apply_fade(c, fade));
            }
        }
        let mut c = first.2;
        if first.1 == 0 && obj_alpha[i] != 0xFF {
            c = blend(c, second.2, obj_alpha[i]);
        }
        out[i] = apply_fade(c, fades.screen);
    }
    out
}

fn draw_part(p: &SpritePart, obj: &mut [u16], obj_prio: &mut [u8], obj_alpha: &mut [u8]) {
    if let Some(m) = p.affine {
        draw_affine(p, m, obj, obj_prio, obj_alpha);
        return;
    }
    let (w, h) = (p.width as i32, p.height as i32);
    let tiles_per_row = (p.width / 8) as usize;
    let mosaic = p.mosaic.map(|m| m as i32 + 1).filter(|&n| n > 1);
    for row in 0..h {
        let sy = (p.y as i32 + row) & 0xFF;
        if sy >= HEIGHT as i32 {
            continue;
        }
        // Mosaic samples the row at the top of its screen block, clamped
        // to the sprite (as mGBA does).
        let row = match mosaic {
            Some(n) => (row - sy % n).clamp(0, h - 1),
            None => row,
        };
        let row = match p.vscale {
            Some(pd) => {
                let r = ((pd * (row - h / 2)) >> 8) + h / 2;
                if !(0..h).contains(&r) {
                    continue;
                }
                r
            }
            None => row,
        };
        // Horizontal mosaic samples the column at the start of each screen
        // block; the sprite's span extends to the end of its last block.
        let x0 = if p.x as i32 + w > 0x200 { p.x as i32 - 0x200 } else { p.x as i32 };
        let span = match mosaic {
            Some(n) => (x0 + w + n - 1).div_euclid(n) * n - x0,
            None => w,
        };
        for col in 0..span {
            let sx = x0 + col;
            if !(0..WIDTH as i32).contains(&sx) {
                continue;
            }
            let i = sy as usize * WIDTH + sx as usize;
            if obj_prio[i] <= p.priority {
                continue;
            }
            let col = match mosaic {
                Some(n) => (col - sx % n).clamp(0, w - 1),
                None => col,
            };
            let tx = if p.hflip { w - 1 - col } else { col } as usize;
            let ty = if p.vflip { h - 1 - row } else { row } as usize;
            let t = p.first_tile + (ty / 8) * tiles_per_row + tx / 8;
            let Some(tile) = p.tiles.get(t) else { continue };
            let index = tile[(ty % 8) * 8 + tx % 8];
            if index == 0 {
                continue;
            }
            obj[i] = p.palette[index as usize] & 0x7FFF;
            obj_prio[i] = p.priority;
            obj_alpha[i] = p.alpha.unwrap_or(0xFF);
        }
    }
}

/// An affine part (as mGBA draws it): the box, its pixels read through
/// the matrix about the centres.
fn draw_affine(p: &SpritePart, m: Affine, obj: &mut [u16], obj_prio: &mut [u8], obj_alpha: &mut [u8]) {
    let (w, h) = (p.width as i32, p.height as i32);
    let (bw, bh) = if m.double { (2 * w, 2 * h) } else { (w, h) };
    let tiles_per_row = (p.width / 8) as usize;
    let x0 = if p.x as i32 + bw > 0x200 { p.x as i32 - 0x200 } else { p.x as i32 };
    for by in 0..bh {
        let sy = (p.y as i32 + by) & 0xFF;
        if sy >= HEIGHT as i32 {
            continue;
        }
        let dy = by - bh / 2;
        for bx in 0..bw {
            let sx = x0 + bx;
            if !(0..WIDTH as i32).contains(&sx) {
                continue;
            }
            let dx = bx - bw / 2;
            let tx = ((m.pa * dx + m.pb * dy) >> 8) + w / 2;
            let ty = ((m.pc * dx + m.pd * dy) >> 8) + h / 2;
            if !(0..w).contains(&tx) || !(0..h).contains(&ty) {
                continue;
            }
            let i = sy as usize * WIDTH + sx as usize;
            if obj_prio[i] <= p.priority {
                continue;
            }
            let t = p.first_tile + (ty as usize / 8) * tiles_per_row + tx as usize / 8;
            let Some(tile) = p.tiles.get(t) else { continue };
            let index = tile[(ty as usize % 8) * 8 + tx as usize % 8];
            if index == 0 {
                continue;
            }
            obj[i] = p.palette[index as usize] & 0x7FFF;
            obj_prio[i] = p.priority;
            obj_alpha[i] = p.alpha.unwrap_or(0xFF);
        }
    }
}

fn channels(c: u16) -> [u16; 3] {
    [c & 31, (c >> 5) & 31, (c >> 10) & 31]
}

fn pack(ch: [u16; 3]) -> u16 {
    ch[0] | ch[1] << 5 | ch[2] << 10
}

/// Alpha blend: a * eva/16 + b * (16 - eva)/16, per channel, saturating.
pub fn blend(a: u16, b: u16, eva: u8) -> u16 {
    let (a, b) = (channels(a), channels(b));
    let (ea, eb) = (eva.min(16) as u16, 16 - eva.min(16) as u16);
    pack(std::array::from_fn(|k| ((a[k] * ea + b[k] * eb) >> 4).min(31)))
}

pub fn apply_fade(c: u16, fade: Fade) -> u16 {
    let ch = channels(c);
    match fade {
        Fade::None => c,
        Fade::Black(n) => {
            let n = n.min(16) as u16;
            pack(ch.map(|v| v - ((v * n) >> 4)))
        }
        Fade::White(n) => {
            let n = n.min(16) as u16;
            pack(ch.map(|v| v + (((31 - v) * n) >> 4)))
        }
    }
}

/// BGR555 to 0x00RRGGBB, widening each 5-bit channel as `v << 3 | v >> 2`.
pub fn to_rgb(c: u16) -> u32 {
    let w = |v: u16| ((v << 3) | (v >> 2)) as u32;
    let [r, g, b] = channels(c);
    w(r) << 16 | w(g) << 8 | w(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_tiles(n: usize, index: u8) -> Tiles {
        Tiles { pixels: vec![index; n * Tiles::TILE] }
    }

    fn part(tiles: &Tiles, x: u16, y: u8, prio: u8, colour: u16) -> SpritePart<'_> {
        let mut palette = [0u16; 16];
        palette[1] = colour;
        SpritePart {
            x,
            y,
            width: 8,
            height: 8,
            tiles,
            first_tile: 0,
            hflip: false,
            vflip: false,
            palette,
            priority: prio,
            alpha: None,
            mosaic: None,
            vscale: None,
            affine: None,
        }
    }

    #[test]
    fn an_affine_part_turns_about_its_centre() {
        // A 16x16 sprite with its top-left 8x8 quarter coloured; a half
        // turn (pa = pd = -1) shows it at the bottom right: box pixel x
        // reads texture column 16 - x (so the box's first column is past
        // the texture).
        let mut tiles = solid_tiles(4, 0);
        tiles.pixels[..64].fill(1);
        let mut p = part(&tiles, 0, 0, 2, 0x001F);
        (p.width, p.height) = (16, 16);
        p.affine = Some(Affine { pa: -0x100, pb: 0, pc: 0, pd: -0x100, double: false });
        let out = compose(0, &[], &[p.clone()], Fades::default());
        assert_eq!(out[0], 0);
        assert_eq!(out[15 * WIDTH + 15], 0x001F);
        assert_eq!(out[9 * WIDTH + 9], 0x001F);
        assert_eq!(out[8 * WIDTH + 8], 0);
        // A doubled box, unrotated: the sprite sits in its middle.
        p.affine = Some(Affine { pa: 0x100, pb: 0, pc: 0, pd: 0x100, double: true });
        let out = compose(0, &[], &[p], Fades::default());
        assert_eq!(out[8 * WIDTH + 8], 0x001F);
        assert_eq!(out[7 * WIDTH + 7], 0);
        assert_eq!(out[16 * WIDTH + 16], 0);
    }

    #[test]
    fn earlier_sprites_win_ties_and_lower_priority_wins() {
        let t = solid_tiles(1, 1);
        let parts = [part(&t, 0, 0, 2, 0x001F), part(&t, 4, 0, 2, 0x03E0), part(&t, 10, 0, 1, 0x7C00)];
        let out = compose(0, &[], &parts, Fades::default());
        assert_eq!(out[5], 0x001F, "the earlier part is in front at equal priority");
        assert_eq!(out[11], 0x7C00, "priority 1 beats priority 2");
        assert_eq!(out[8], 0x03E0);
        assert_eq!(out[20], 0, "the backdrop shows where nothing is drawn");
    }

    #[test]
    fn sprites_beat_layers_of_equal_priority_only() {
        let t = solid_tiles(1, 1);
        let mut front = Layer::new(1, 3);
        front.pixels[0] = 0x1234;
        front.pixels[1] = 0x1234;
        let mut behind = Layer::new(2, 2);
        behind.pixels[2] = 0x0042;
        let parts = [part(&t, 0, 0, 2, 0x001F)];
        let out = compose(0, &[&front, &behind], &parts, Fades::default());
        assert_eq!(out[0], 0x1234, "a priority-1 layer covers priority-2 sprites");
        assert_eq!(out[2], 0x001F, "a sprite covers a layer of its own priority");
    }

    #[test]
    fn positions_wrap_like_the_hardware() {
        let t = solid_tiles(1, 1);
        // y 252 shows its last 4 rows at the top; x 508 its last 4 columns.
        let out = compose(0, &[], &[part(&t, 508, 252, 2, 0x001F)], Fades::default());
        assert_eq!(out[3 * WIDTH + 3], 0x001F);
        assert_eq!(out[4 * WIDTH], 0);
        assert_eq!(out[4], 0);
    }

    #[test]
    fn semi_transparent_sprites_blend() {
        let t = solid_tiles(1, 1);
        let mut p = part(&t, 0, 0, 2, 31);
        p.alpha = Some(8);
        let out = compose(0, &[], &[p], Fades::default());
        assert_eq!(out[0], 15);
        assert_eq!(blend(0x7FFF, 0, 16), 0x7FFF);
    }

    #[test]
    fn a_stage_fade_leaves_the_hud_layer_and_the_sprites() {
        let t = solid_tiles(1, 1);
        let mut stage = Layer::new(2, 2);
        stage.pixels[0] = 0x7FFF;
        stage.pixels[1] = 0x7FFF;
        let mut hud = Layer::new(1, 3);
        hud.palettes = Palettes::Hud;
        hud.pixels[1] = 0x7FFF;
        let parts = [part(&t, 2, 0, 2, 0x7FFF)];
        let fades = Fades { stage: Fade::Black(4), ..Fades::default() };
        let out = compose(0x7FFF, &[&hud, &stage], &parts, fades);
        // 31 - (31 * 4 >> 4) = 24 a channel.
        let dimmed = 24 | 24 << 5 | 24 << 10;
        assert_eq!(out[0], dimmed);
        assert_eq!(out[1], 0x7FFF);
        assert_eq!(out[2], 0x7FFF);
        assert_eq!(out[20], dimmed, "the backdrop is the stage's");
        assert_eq!(apply_fade(0x7FFF, Fade::Black(16)), 0);
        assert_eq!(to_rgb(0x7FFF), 0xFFFFFF);
    }
}
