//! Renders a set of synthetic scenes exercising the PPU's features to PPM
//! files, for eyeballing.
//!
//! ```text
//! cargo run -p gba-ppu --example scenes -- [OUT_DIR]
//! ```
//!
//! `OUT_DIR` defaults to `$TMPDIR/gba-ppu-scenes`.

use gba_ppu::{HEIGHT, IO_SIZE, OAM_SIZE, PALETTE_SIZE, Ppu, VRAM_SIZE, WIDTH, regs};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};

/// Per-scanline register effect, as passed to `Ppu::render_frame`.
pub type LineEffect = fn(usize, &mut [u8; IO_SIZE]);

/// Memory and registers for one frame.
pub struct Scene {
    pub name: &'static str,
    pub io: Box<[u8; IO_SIZE]>,
    pub vram: Box<[u8; VRAM_SIZE]>,
    pub palette: Box<[u8; PALETTE_SIZE]>,
    pub oam: Box<[u8; OAM_SIZE]>,
    pub line_effect: Option<LineEffect>,
}

const OBJ_VRAM: usize = 0x10000;

// DISPCNT bits.
const OBJ_1D: u16 = 1 << 6;
const BG0_ON: u16 = 1 << 8;
const BG1_ON: u16 = 1 << 9;
const BG2_ON: u16 = 1 << 10;
const OBJ_ON: u16 = 1 << 12;
const WIN0_ON: u16 = 1 << 13;
const WIN1_ON: u16 = 1 << 14;
const OBJWIN_ON: u16 = 1 << 15;

/// BGxCNT value.
fn bgcnt(priority: u16, char_block: u16, screen_block: u16, color256: bool, size: u16) -> u16 {
    priority | (char_block << 2) | (u16::from(color256) << 7) | (screen_block << 8) | (size << 14)
}

fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}

/// Parses an 8x8 glyph: `.` is transparent, hex digits are color indices.
fn glyph(rows: [&str; 8]) -> [[u8; 8]; 8] {
    rows.map(|row| {
        let mut out = [0; 8];
        for (o, c) in out.iter_mut().zip(row.chars()) {
            *o = c.to_digit(16).unwrap_or(0) as u8;
        }
        out
    })
}

const GLYPH_F: [&str; 8] = [
    "........", ".111111.", ".122222.", ".12.....", ".11111..", ".12222..", ".12.....", ".12.....",
];
const GLYPH_RING: [&str; 8] = [
    "..3333..", ".344443.", "34....43", "34....43", "34....43", "34....43", ".344443.", "..3333..",
];
const GLYPH_CHECKER: [&str; 8] = [
    "55556666", "55556666", "55556666", "55556666", "66665555", "66665555", "66665555", "66665555",
];

impl Scene {
    fn new(name: &'static str) -> Scene {
        let mut scene = Scene {
            name,
            io: Box::new([0; IO_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            palette: Box::new([0; PALETTE_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
            line_effect: None,
        };
        // Hide every sprite.
        for i in 0..128 {
            scene.obj(i, 0x0200, 0, 0);
        }
        // Identity affine transforms.
        scene.reg(regs::BG2PA, 0x100);
        scene.reg(regs::BG2PD, 0x100);
        scene.reg(regs::BG3PA, 0x100);
        scene.reg(regs::BG3PD, 0x100);
        scene
    }

    fn reg(&mut self, address: usize, value: u16) {
        self.io[address..address + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn reg32(&mut self, address: usize, value: i32) {
        self.io[address..address + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn color(&mut self, index: usize, color: u16) {
        self.palette[index * 2..index * 2 + 2].copy_from_slice(&color.to_le_bytes());
    }

    fn vram16(&mut self, offset: usize, value: u16) {
        self.vram[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Writes a 4bpp tile at byte offset `base`.
    fn tile4(&mut self, base: usize, pixels: [[u8; 8]; 8]) {
        for (y, row) in pixels.iter().enumerate() {
            for (x, &p) in row.iter().enumerate() {
                let byte = &mut self.vram[base + y * 4 + x / 2];
                *byte |= p << (4 * (x % 2));
            }
        }
    }

    /// Writes an 8bpp tile at byte offset `base`.
    fn tile8(&mut self, base: usize, pixel: impl Fn(usize, usize) -> u8) {
        for y in 0..8 {
            for x in 0..8 {
                self.vram[base + y * 8 + x] = pixel(x, y);
            }
        }
    }

    /// Writes a 4bpp, 1D-mapped sprite image starting at OBJ tile `tile`.
    fn obj_image4(&mut self, tile: usize, w: usize, h: usize, pixel: impl Fn(usize, usize) -> u8) {
        for y in 0..h {
            for x in 0..w {
                let t = tile + (y / 8) * (w / 8) + x / 8;
                let byte = &mut self.vram[OBJ_VRAM + t * 32 + (y % 8) * 4 + (x % 8) / 2];
                *byte |= (pixel(x, y) & 0xF) << (4 * (x % 2));
            }
        }
    }

    /// Writes an 8bpp, 1D-mapped sprite image starting at OBJ tile `tile`
    /// (in 32-byte units).
    fn obj_image8(&mut self, tile: usize, w: usize, h: usize, pixel: impl Fn(usize, usize) -> u8) {
        for y in 0..h {
            for x in 0..w {
                let t = tile + 2 * ((y / 8) * (w / 8) + x / 8);
                self.vram[OBJ_VRAM + t * 32 + (y % 8) * 8 + x % 8] = pixel(x, y);
            }
        }
    }

    fn obj(&mut self, index: usize, attr0: u16, attr1: u16, attr2: u16) {
        for (i, v) in [attr0, attr1, attr2].into_iter().enumerate() {
            self.oam[index * 8 + i * 2..][..2].copy_from_slice(&v.to_le_bytes());
        }
    }

    /// Sets OBJ affine matrix `index` to rotate by `angle` (radians) and
    /// scale by `scale`.
    fn obj_matrix(&mut self, index: usize, angle: f64, scale: f64) {
        let [pa, pb, pc, pd] = affine(angle, scale);
        for (i, v) in [pa, pb, pc, pd].into_iter().enumerate() {
            self.oam[index * 32 + 6 + i * 8..][..2].copy_from_slice(&(v as u16).to_le_bytes());
        }
    }

    pub fn render(&self, ppu: &mut Ppu, out: &mut [u32; WIDTH * HEIGHT]) {
        match self.line_effect {
            Some(mut effect) => ppu.render_frame(
                &self.io,
                &self.vram,
                &self.palette,
                &self.oam,
                Some(&mut effect),
                out,
            ),
            None => ppu.render_frame(&self.io, &self.vram, &self.palette, &self.oam, None, out),
        }
    }
}

/// PA..PD (8.8 fixed point) mapping screen offsets to texture offsets for
/// a layer rotated by `angle` and scaled by `scale`.
fn affine(angle: f64, scale: f64) -> [i16; 4] {
    let (s, c) = angle.sin_cos();
    [c / scale, s / scale, -s / scale, c / scale].map(|v| (v * 256.0).round() as i16)
}

/// Four 4bpp BG palettes with distinct hues, a grayscale checker in
/// colors 5/6, and a 192-entry 8bpp gradient at 64..256.
fn standard_bg_palette(scene: &mut Scene) {
    let hues = [
        rgb(31, 8, 8),
        rgb(8, 28, 8),
        rgb(10, 14, 31),
        rgb(31, 26, 4),
    ];
    for (bank, &hue) in hues.iter().enumerate() {
        let base = bank * 16;
        scene.color(base + 1, hue);
        scene.color(base + 2, (hue >> 1) & 0x3DEF);
        scene.color(base + 3, rgb(31, 31, 31));
        scene.color(base + 4, rgb(20, 20, 24));
        scene.color(base + 5, rgb(12, 12, 14));
        scene.color(base + 6, rgb(18, 18, 20));
    }
    for i in 0..192 {
        let t = i as u16;
        scene.color(64 + i, rgb(t / 6, (t * 31 / 191 + 8) % 32, 31 - t / 7));
    }
}

fn standard_obj_palette(scene: &mut Scene) {
    let colors = [
        rgb(31, 31, 31),
        rgb(31, 4, 4),
        rgb(31, 28, 0),
        rgb(4, 28, 4),
        rgb(0, 24, 31),
    ];
    for bank in 0..16 {
        for (i, &c) in colors.iter().enumerate() {
            // Shift hues per bank so banks are distinguishable.
            let c = if bank == 0 {
                c
            } else {
                c.rotate_left(bank as u32 * 5) & 0x7FFF
            };
            scene.color(256 + bank * 16 + 1 + i, c);
        }
    }
    for i in 0..240 {
        scene.color(
            256 + 16 + i,
            rgb((i % 32) as u16, (i / 8) as u16, 31 - (i % 32) as u16),
        );
    }
}

/// Sprite image: border (1), diagonal (2), orientation marker in the
/// top-left (3), and a checker in the right half (4); otherwise transparent.
fn sprite_pixel(w: usize, h: usize) -> impl Fn(usize, usize) -> u8 {
    move |x, y| {
        if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            1
        } else if x * h == y * w {
            2
        } else if x < 4 && y < 4 {
            3
        } else if x >= w / 2 && (x / 2 + y / 2) % 2 == 0 {
            4
        } else {
            0
        }
    }
}

/// Mode 0: a 4bpp layer of flipped/palette-swapped glyphs over an 8bpp
/// gradient layer, both scrolled.
fn text_scene() -> Scene {
    let mut s = Scene::new("text");
    standard_bg_palette(&mut s);
    s.color(0, rgb(2, 2, 8));
    s.tile4(32, glyph(GLYPH_F));
    s.tile4(64, glyph(GLYPH_RING));
    for t in 0..4 {
        s.tile8(0x4000 + t * 64, |x, y| {
            let v = match t {
                0 => x * 8 + y,
                1 => y * 8 + x,
                2 => (x + y) * 4,
                _ => (x ^ y) * 8,
            };
            64 + (v * 3 % 192) as u8
        });
    }
    for y in 0..32 {
        for x in 0..32 {
            let entry = match (x + 2 * y) % 5 {
                0 => 1 | (((x as u16) & 3) << 10) | ((y as u16 & 3) << 12),
                1 => 2 | ((x as u16 & 3) << 12),
                _ => 0,
            };
            s.vram16(0xE000 + (y * 32 + x) * 2, entry);
        }
    }
    // BG1: 512x256 (two screen blocks).
    for y in 0..32 {
        for x in 0..64 {
            let block = 0xF000 + (x / 32) * 0x800;
            let entry = ((x + y) % 4) as u16 | (((x / 4) as u16 & 3) << 10);
            s.vram16(block + (y * 32 + x % 32) * 2, entry);
        }
    }
    s.reg(regs::DISPCNT, BG0_ON | BG1_ON);
    s.reg(regs::BG0CNT, bgcnt(0, 0, 28, false, 0));
    s.reg(regs::BG1CNT, bgcnt(1, 1, 30, true, 1));
    s.reg(regs::BG0HOFS, 4);
    s.reg(regs::BG0VOFS, 2);
    s.reg(regs::BG1HOFS, 200);
    s
}

/// Mode 0: every sprite shape and size, flips, 8bpp, affine and
/// double-size sprites, over a checkered background with a priority split.
fn sprites_scene() -> Scene {
    let mut s = Scene::new("sprites");
    standard_bg_palette(&mut s);
    standard_obj_palette(&mut s);
    s.color(0, rgb(4, 6, 10));
    s.tile4(32, glyph(GLYPH_CHECKER));
    for y in 0..32 {
        for x in 0..32 {
            // Left strip at priority 1 (BG0), the rest at priority 3 (BG1).
            let block = if x < 6 { 0xE000 } else { 0xE800 };
            s.vram16(block + (y * 32 + x) * 2, 1);
        }
    }
    s.reg(regs::DISPCNT, BG0_ON | BG1_ON | OBJ_ON | OBJ_1D);
    s.reg(regs::BG0CNT, bgcnt(1, 0, 28, false, 0));
    s.reg(regs::BG1CNT, bgcnt(3, 0, 29, false, 0));

    let sizes = [
        (0, 0, 8, 8),
        (0, 1, 16, 16),
        (0, 2, 32, 32),
        (0, 3, 64, 64),
        (1, 0, 16, 8),
        (1, 1, 32, 8),
        (1, 2, 32, 16),
        (1, 3, 64, 32),
        (2, 0, 8, 16),
        (2, 1, 8, 32),
        (2, 2, 16, 32),
        (2, 3, 32, 64),
    ];
    let mut tile = 0;
    let mut x = 2;
    for (i, &(shape, size, w, h)) in sizes.iter().enumerate() {
        s.obj_image4(tile, w, h, sprite_pixel(w, h));
        let y = if i < 4 {
            2
        } else if i < 8 {
            70
        } else {
            106
        };
        if i == 4 || i == 8 {
            x = 2;
        }
        // Priority 2: behind the left strip, in front of the rest.
        let prio = if i % 3 == 2 { 2 } else { 0 };
        s.obj(
            i,
            (shape << 14) | y,
            (size << 14) | x,
            (prio << 10) | tile as u16,
        );
        tile += w * h / 64;
        x += w as u16 + 4;
    }
    // Flips of a 16x16 sprite (tile 1 holds the 16x16 image).
    for (k, flips) in (0..4u16).enumerate() {
        let y = 104 + 18 * (k as u16 / 2);
        s.obj(
            12 + k,
            y,
            (1 << 14) | (flips << 12) | (180 + 18 * (k as u16 % 2)),
            1 | ((k as u16) << 12),
        );
    }
    // 8bpp 32x32 sprite.
    s.obj_image8(tile, 32, 32, |x, y| {
        if (x / 4 + y / 4) % 3 == 0 {
            0
        } else {
            (x * 7 + y) as u8 | 16
        }
    });
    s.obj(16, 0x2000 | 140, (2 << 14) | 190, tile as u16);
    // Affine sprites: rotated, and rotated + scaled with double size.
    s.obj_matrix(0, PI / 6.0, 1.0);
    s.obj_matrix(1, -PI / 4.0, 1.5);
    s.obj(17, 0x0100 | 24, (2 << 14) | 150, 5);
    s.obj(18, 0x0300 | 20, (1 << 9) | (2 << 14) | 176, 5);
    s
}

/// Mode 1: a rotated, scaled, wrapping affine layer with a per-scanline
/// perspective effect (rewriting PA/PC and the reference point each line),
/// under a text layer.
fn affine_scene() -> Scene {
    let mut s = Scene::new("affine");
    standard_bg_palette(&mut s);
    s.color(0, rgb(0, 0, 6));
    // BG0: text strip at the top.
    s.tile4(32, glyph(GLYPH_F));
    for x in 0..32 {
        for y in 0..3 {
            s.vram16(0xF800 + (y * 32 + x) * 2, 1 | ((x as u16 & 3) << 12));
        }
    }
    // BG2: 256x256 affine map (32x32 one-byte entries) of 8bpp tiles.
    for t in 0..4usize {
        s.tile8(0x4000 + t * 64, |x, y| {
            let edge = x == 0 || y == 0;
            64 + if edge {
                40 * t
            } else {
                ((x + y) * 6 + t * 48) % 192
            } as u8
        });
    }
    for y in 0..32 {
        for x in 0..32 {
            s.vram[0xE000 + y * 32 + x] = ((x / 2 + y / 2) % 4) as u8;
        }
    }
    s.reg(regs::DISPCNT, 1 | BG0_ON | BG2_ON);
    s.reg(regs::BG0CNT, bgcnt(0, 0, 31, false, 0));
    s.reg(regs::BG2CNT, bgcnt(1, 1, 28, true, 1) | (1 << 13));
    s.line_effect = Some(|y, io| {
        // Mode-7 style floor below line 40; rotated plane above.
        let set16 = |io: &mut [u8; IO_SIZE], a: usize, v: u16| {
            io[a..a + 2].copy_from_slice(&v.to_le_bytes())
        };
        let set32 = |io: &mut [u8; IO_SIZE], a: usize, v: i32| {
            io[a..a + 4].copy_from_slice(&v.to_le_bytes())
        };
        let angle = 0.5;
        if y < 40 {
            let [pa, pb, pc, pd] = affine(angle, 1.5);
            if y == 0 {
                for (i, v) in [pa, pb, pc, pd].into_iter().enumerate() {
                    set16(io, regs::BG2PA + 2 * i, v as u16);
                }
                set32(io, regs::BG2X, 0x4000);
                set32(io, regs::BG2Y, -0x2000);
            }
        } else {
            let depth = 2400.0 / (y as f64 - 30.0);
            let scale = 1.0 / (depth / 64.0);
            let [pa, _, pc, _] = affine(angle, scale);
            set16(io, regs::BG2PA, pa as u16);
            set16(io, regs::BG2PC, pc as u16);
            let (sin, cos) = angle.sin_cos();
            let cx = 128.0 * 256.0 - 120.0 * (pa as f64) + depth * 256.0 * sin;
            let cy = 128.0 * 256.0 - 120.0 * (pc as f64) - depth * 256.0 * cos;
            set32(io, regs::BG2X, cx as i32);
            set32(io, regs::BG2Y, cy as i32);
        }
    });
    s
}

/// Mode 0: alpha blending between two layers, semi-transparent sprites,
/// and a window where blending is off.
fn blend_scene() -> Scene {
    let mut s = Scene::new("blend");
    standard_bg_palette(&mut s);
    standard_obj_palette(&mut s);
    s.color(0, rgb(0, 0, 0));
    s.tile4(32, glyph(GLYPH_CHECKER));
    s.tile4(64, glyph(["11111111"; 8]));
    for y in 0..32 {
        for x in 0..32 {
            s.vram16(0xE000 + (y * 32 + x) * 2, 1);
            // Diagonal color bands on BG0.
            let band = ((x + y) / 4) % 5;
            let entry = if band == 4 {
                0
            } else {
                2 | ((band as u16) << 12)
            };
            s.vram16(0xE800 + (y * 32 + x) * 2, entry);
        }
    }
    for i in 0..4 {
        s.obj_image4(i * 16, 32, 32, |x, y| {
            let (dx, dy) = (x as i32 - 16, y as i32 - 16);
            if dx * dx + dy * dy < 225 {
                2 + (i as u8 % 3)
            } else {
                0
            }
        });
        // Semi-transparent sprites (mode 1), the last one opaque.
        let mode = if i < 3 { 1 << 10 } else { 0 };
        s.obj(
            i,
            mode | (20 + 30 * i as u16),
            (2 << 14) | (30 + 50 * i as u16),
            (i * 16) as u16,
        );
    }
    s.reg(regs::DISPCNT, BG0_ON | BG1_ON | OBJ_ON | OBJ_1D | WIN0_ON);
    s.reg(regs::BG0CNT, bgcnt(0, 0, 29, false, 0));
    s.reg(regs::BG1CNT, bgcnt(1, 0, 28, false, 0));
    // BG0 and OBJ over BG0, BG1 and the backdrop, 10/16 + 6/16.
    s.reg(
        regs::BLDCNT,
        0x0001 | 0x0010 | (1 << 6) | 0x0100 | 0x0200 | 0x2000,
    );
    s.reg(regs::BLDALPHA, 10 | (6 << 8));
    // WIN0 on the right: no blending there.
    s.reg(regs::WIN0H, (170 << 8) | 236);
    s.reg(regs::WIN0V, (10 << 8) | 150);
    s.reg(regs::WININ, 0x1F);
    s.reg(regs::WINOUT, 0x3F);
    s
}

/// Mode 0: brightness decrease outside a circular window whose shape is
/// set per scanline (H-blank DMA style), and a wavy background.
fn spotlight_scene() -> Scene {
    let mut s = text_scene();
    s.name = "spotlight";
    s.reg(regs::DISPCNT, BG0_ON | BG1_ON | WIN0_ON);
    s.reg(regs::WIN0V, 160);
    s.reg(regs::WININ, 0x1F);
    s.reg(regs::WINOUT, 0x3F);
    s.reg(regs::BLDCNT, 0x3F | (3 << 6));
    s.reg(regs::BLDY, 11);
    s.line_effect = Some(|y, io| {
        let (cx, cy, r) = (120.0, 80.0, 60.0);
        let dy = y as f64 - cy;
        let h = if dy.abs() < r {
            let half = (r * r - dy * dy).sqrt();
            (((cx - half) as u16) << 8) | (cx + half) as u16
        } else {
            0
        };
        io[regs::WIN0H..regs::WIN0H + 2].copy_from_slice(&h.to_le_bytes());
        let wave = (8.0 * (y as f64 * 2.0 * PI / 40.0).sin()) as i16 as u16;
        io[regs::BG1HOFS..regs::BG1HOFS + 2]
            .copy_from_slice(&(200u16.wrapping_add(wave)).to_le_bytes());
    });
    s
}

/// Mode 0: WIN0 and WIN1 rectangles and an OBJ window, each showing a
/// different set of layers, with brightening inside WIN1.
fn windows_scene() -> Scene {
    let mut s = text_scene();
    s.name = "windows";
    standard_obj_palette(&mut s);
    s.tile4(96, glyph(GLYPH_CHECKER));
    for y in 0..32 {
        for x in 0..32 {
            s.vram16(0xE800 + (y * 32 + x) * 2, 3 | (2 << 12));
        }
    }
    s.reg(regs::BG2CNT, bgcnt(2, 0, 29, false, 0));
    // A large ring sprite as the OBJ window.
    s.obj_image4(0, 64, 64, |x, y| {
        let (dx, dy) = (x as i32 - 32, y as i32 - 32);
        let d = dx * dx + dy * dy;
        u8::from((400..900).contains(&d))
    });
    s.obj_matrix(0, 0.0, 1.8);
    s.obj(0, 0x0300 | (2 << 10) | 10, (3 << 14) | 60, 0);
    s.reg(
        regs::DISPCNT,
        BG0_ON | BG1_ON | BG2_ON | OBJ_ON | OBJ_1D | WIN0_ON | WIN1_ON | OBJWIN_ON,
    );
    s.reg(regs::WIN0H, (20 << 8) | 110);
    s.reg(regs::WIN0V, (20 << 8) | 90);
    s.reg(regs::WIN1H, (80 << 8) | 220);
    s.reg(regs::WIN1V, (60 << 8) | 140);
    // WIN0: BG1 only. WIN1: BG0 + BG2 with effects. OBJ window: BG2.
    // Outside: BG0 + BG1.
    s.reg(regs::WININ, 0x02 | (0x25 << 8));
    s.reg(regs::WINOUT, 0x03 | (0x04 << 8));
    s.reg(regs::BLDCNT, 0x05 | (2 << 6));
    s.reg(regs::BLDY, 8);
    s
}

/// Mode 0 with BG and OBJ mosaic.
fn mosaic_scene() -> Scene {
    let mut s = sprites_scene();
    s.name = "mosaic";
    s.reg(regs::BG0CNT, bgcnt(1, 0, 28, false, 0) | 0x40);
    s.reg(regs::MOSAIC, 0x3 | (0x3 << 4) | (0x2 << 8) | (0x1 << 12));
    for i in 0..19 {
        s.oam[i * 8 + 1] |= 0x10;
    }
    s
}

/// Mode 3 gradient with sprites (tiles 512+) and a brightened window.
fn bitmap3_scene() -> Scene {
    let mut s = Scene::new("bitmap3");
    standard_obj_palette(&mut s);
    for y in 0..160 {
        for x in 0..240 {
            let c = rgb(
                (x * 31 / 239) as u16,
                (y * 31 / 159) as u16,
                ((x + y) % 32) as u16,
            );
            s.vram16((y * 240 + x) * 2, c);
        }
    }
    s.obj_image4(512, 32, 32, sprite_pixel(32, 32));
    s.obj(0, 60, (2 << 14) | 100, 512);
    s.reg(regs::DISPCNT, 3 | BG2_ON | OBJ_ON | OBJ_1D | WIN0_ON);
    s.reg(regs::WIN0H, (140 << 8) | 230);
    s.reg(regs::WIN0V, (100 << 8) | 150);
    s.reg(regs::WININ, 0x3F);
    s.reg(regs::WINOUT, 0x1F);
    s.reg(regs::BLDCNT, 0x04 | (2 << 6));
    s.reg(regs::BLDY, 10);
    s
}

/// Mode 4 (paletted bitmap, back frame) scaled 2x through the affine
/// registers.
fn bitmap4_scene() -> Scene {
    let mut s = Scene::new("bitmap4");
    standard_bg_palette(&mut s);
    for y in 0..160 {
        for x in 0..240 {
            let v = ((x / 10) ^ (y / 10)) % 12;
            s.vram[0xA000 + y * 240 + x] = if v == 0 { 0 } else { 64 + (v * 16) as u8 };
        }
    }
    s.color(0, rgb(31, 0, 31));
    s.reg(regs::DISPCNT, 4 | 0x10 | BG2_ON);
    s.reg(regs::BG2PA, 0x80);
    s.reg(regs::BG2PD, 0x80);
    s.reg32(regs::BG2X, 30 << 8);
    s
}

/// Mode 5 (160x128 direct color), rotated.
fn bitmap5_scene() -> Scene {
    let mut s = Scene::new("bitmap5");
    s.color(0, rgb(6, 6, 6));
    for y in 0..128 {
        for x in 0..160 {
            let c = if (x / 16 + y / 16) % 2 == 0 {
                rgb(31, (y / 4) as u16, 0)
            } else {
                rgb(0, (x / 5) as u16, 31)
            };
            s.vram16((y * 160 + x) * 2, c);
        }
    }
    let [pa, pb, pc, pd] = affine(0.3, 1.0);
    s.reg(regs::DISPCNT, 5 | BG2_ON);
    for (i, v) in [pa, pb, pc, pd].into_iter().enumerate() {
        s.reg(regs::BG2PA + 2 * i, v as u16);
    }
    s.reg32(regs::BG2X, -20 << 8);
    s.reg32(regs::BG2Y, 20 << 8);
    s
}

pub fn all_scenes() -> Vec<Scene> {
    vec![
        text_scene(),
        sprites_scene(),
        affine_scene(),
        blend_scene(),
        spotlight_scene(),
        windows_scene(),
        mosaic_scene(),
        bitmap3_scene(),
        bitmap4_scene(),
        bitmap5_scene(),
    ]
}

/// Writes `pixels` (0x00RRGGBB) as a binary PPM.
pub fn write_ppm(path: &Path, width: usize, height: usize, pixels: &[u32]) -> std::io::Result<()> {
    let mut data = format!("P6\n{width} {height}\n255\n").into_bytes();
    for &p in pixels {
        data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
    }
    std::fs::write(path, data)
}

#[allow(dead_code)]
fn main() -> std::io::Result<()> {
    let dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("gba-ppu-scenes"));
    std::fs::create_dir_all(&dir)?;
    let mut ppu = Ppu::new();
    let mut frame = Box::new([0u32; WIDTH * HEIGHT]);
    for scene in all_scenes() {
        scene.render(&mut ppu, &mut frame);
        let path = dir.join(format!("{}.ppm", scene.name));
        write_ppm(&path, WIDTH, HEIGHT, &frame[..])?;
        println!("{}", path.display());
    }
    Ok(())
}
