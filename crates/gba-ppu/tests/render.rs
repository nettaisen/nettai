//! Rendering tests on small hand-built scenes with hand-computed results.

use gba_ppu::{HEIGHT, IO_SIZE, LineIo, OAM_SIZE, PALETTE_SIZE, Ppu, VRAM_SIZE, WIDTH, regs};

const OBJ_VRAM: usize = 0x10000;

struct Scene {
    io: Box<[u8; IO_SIZE]>,
    vram: Box<[u8; VRAM_SIZE]>,
    palette: Box<[u8; PALETTE_SIZE]>,
    oam: Box<[u8; OAM_SIZE]>,
}

fn rgb(r: u16, g: u16, b: u16) -> u16 {
    r | (g << 5) | (b << 10)
}

impl Scene {
    fn new() -> Scene {
        let mut s = Scene {
            io: Box::new([0; IO_SIZE]),
            vram: Box::new([0; VRAM_SIZE]),
            palette: Box::new([0; PALETTE_SIZE]),
            oam: Box::new([0; OAM_SIZE]),
        };
        for i in 0..128 {
            s.obj(i, 0x0200, 0, 0);
        }
        for reg in [regs::BG2PA, regs::BG2PD, regs::BG3PA, regs::BG3PD] {
            s.reg(reg, 0x100);
        }
        s
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

    /// Fills 4bpp tile data at `offset` with a pixel function.
    fn tile4(&mut self, offset: usize, pixel: impl Fn(usize, usize) -> u8) {
        for y in 0..8 {
            for x in 0..8 {
                self.vram[offset + y * 4 + x / 2] |= (pixel(x, y) & 0xF) << (4 * (x % 2));
            }
        }
    }

    fn obj(&mut self, index: usize, attr0: u16, attr1: u16, attr2: u16) {
        for (i, v) in [attr0, attr1, attr2].into_iter().enumerate() {
            self.oam[index * 8 + i * 2..][..2].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn obj_matrix(&mut self, index: usize, [pa, pb, pc, pd]: [i16; 4]) {
        for (i, v) in [pa, pb, pc, pd].into_iter().enumerate() {
            self.oam[index * 32 + 6 + i * 8..][..2].copy_from_slice(&v.to_le_bytes());
        }
    }

    fn render(&self) -> Vec<u16> {
        self.render_with(None)
    }

    fn render_with(&self, line_io: Option<LineIo<'_>>) -> Vec<u16> {
        let mut out = Box::new([0u16; WIDTH * HEIGHT]);
        Ppu::new().render_frame_bgr555(
            &self.io,
            &self.vram,
            &self.palette,
            &self.oam,
            line_io,
            &mut out,
        );
        out.to_vec()
    }
}

fn at(frame: &[u16], x: usize, y: usize) -> u16 {
    frame[y * WIDTH + x]
}

const RED: u16 = 0x001F;
const GREEN: u16 = 0x03E0;
const BLUE: u16 = 0x7C00;
const WHITE: u16 = 0x7FFF;
const BACKDROP: u16 = 0x1234;

/// Mode 0 with BG0 (4bpp, char block 0, screen block 31): a solid tile 1
/// of color index 1 at map position (0, 0), palette bank 1 red.
fn one_tile_scene() -> Scene {
    let mut s = Scene::new();
    s.color(0, BACKDROP);
    s.color(16 + 1, RED);
    s.tile4(32, |_, _| 1);
    s.vram16(0xF800, 1 | (1 << 12));
    s.reg(regs::DISPCNT, 0x0100);
    s.reg(regs::BG0CNT, 31 << 8);
    s
}

#[test]
fn backdrop_fills_empty_screen() {
    let mut s = Scene::new();
    s.color(0, BACKDROP);
    let frame = s.render();
    assert!(frame.iter().all(|&c| c == BACKDROP));
}

#[test]
fn forced_blank_is_white() {
    let mut s = one_tile_scene();
    s.reg(regs::DISPCNT, 0x0180);
    let frame = s.render();
    assert!(frame.iter().all(|&c| c == WHITE));
}

#[test]
fn rgb888_output_expands_channels() {
    let s = one_tile_scene();
    let mut out = Box::new([0u32; WIDTH * HEIGHT]);
    Ppu::new().render_frame(&s.io, &s.vram, &s.palette, &s.oam, None, &mut out);
    assert_eq!(out[0], 0xFF0000);
}

#[test]
fn text_bg_tile_and_scrolling() {
    let mut s = one_tile_scene();
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), RED);
    assert_eq!(at(&frame, 7, 7), RED);
    assert_eq!(at(&frame, 8, 0), BACKDROP);
    assert_eq!(at(&frame, 0, 8), BACKDROP);
    // The 256x256 map wraps.
    assert_eq!(at(&frame, 0, 158), BACKDROP);

    s.reg(regs::BG0HOFS, 3);
    s.reg(regs::BG0VOFS, 0x1FE); // -2 mod 512
    let frame = s.render();
    assert_eq!(at(&frame, 4, 2), RED);
    assert_eq!(at(&frame, 5, 2), BACKDROP);
    assert_eq!(at(&frame, 0, 1), BACKDROP);
    assert_eq!(at(&frame, 0, 9), RED);
    // The map wraps horizontally too: map x 256 is map x 0.
    s.reg(regs::BG0HOFS, 20);
    let frame = s.render();
    assert_eq!(at(&frame, 235, 2), BACKDROP);
    assert_eq!(at(&frame, 236, 2), RED);
}

#[test]
fn text_bg_flips() {
    let mut s = one_tile_scene();
    // Color 1 only in the tile's top-left pixel.
    s.vram[32..64].fill(0);
    s.tile4(32, |x, y| u8::from(x == 0 && y == 0));
    for (flips, (x, y)) in [(0, (0, 0)), (1, (7, 0)), (2, (0, 7)), (3, (7, 7))] {
        s.vram16(0xF800, 1 | (flips << 10) | (1 << 12));
        let frame = s.render();
        assert_eq!(at(&frame, x, y), RED, "flips {flips}");
        assert_eq!(frame.iter().filter(|&&c| c == RED).count(), 1);
    }
}

#[test]
fn text_bg_256_color() {
    let mut s = Scene::new();
    s.color(0x42, GREEN);
    s.vram[0x4000 + 64 + 3] = 0x42; // tile 1, pixel (3, 0)
    s.vram16(0xF800 + 2, 1); // map (1, 0)
    s.reg(regs::DISPCNT, 0x0100);
    s.reg(regs::BG0CNT, (1 << 2) | (1 << 7) | (31 << 8));
    let frame = s.render();
    assert_eq!(at(&frame, 11, 0), GREEN);
    assert_eq!(at(&frame, 10, 0), 0);
}

#[test]
fn text_bg_large_map_layout() {
    // 512x512: screen blocks 28..31 hold the quadrants TL, TR, BL, BR.
    let mut s = Scene::new();
    s.color(1, RED);
    s.color(2, GREEN);
    s.tile4(32, |_, _| 1);
    s.tile4(64, |_, _| 2);
    s.vram16(28 * 0x800 + 0x800, 1); // TR quadrant, tile (0, 0) -> map (256, 0)
    s.vram16(28 * 0x800 + 0x1000, 2); // BL quadrant, tile (0, 0) -> map (0, 256)
    s.reg(regs::DISPCNT, 0x0100);
    s.reg(regs::BG0CNT, (28 << 8) | (3 << 14));
    s.reg(regs::BG0HOFS, 256);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), RED);
    s.reg(regs::BG0HOFS, 0);
    s.reg(regs::BG0VOFS, 256);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), GREEN);
}

#[test]
fn bg_priority_and_index_order() {
    let mut s = one_tile_scene();
    s.color(2, GREEN);
    s.tile4(64, |_, _| 2);
    s.vram16(0xF000, 2); // BG1 on screen block 30
    s.reg(regs::DISPCNT, 0x0300);
    s.reg(regs::BG1CNT, 30 << 8);
    // Same priority: lower index wins.
    assert_eq!(at(&s.render(), 0, 0), RED);
    // BG1 at a better priority wins.
    s.reg(regs::BG0CNT, (31 << 8) | 1);
    assert_eq!(at(&s.render(), 0, 0), GREEN);
}

/// One 8x8 sprite (4bpp, OBJ palette bank 0 color 1 = blue) at (x, y).
fn add_sprite(s: &mut Scene, index: usize, x: u16, y: u16, priority: u16) {
    s.color(256 + 1, BLUE);
    s.tile4(OBJ_VRAM + 32, |_, _| 1);
    s.obj(index, y, x, 1 | (priority << 10));
}

#[test]
fn sprite_priority_against_bg() {
    let mut s = one_tile_scene();
    s.reg(regs::DISPCNT, 0x1100);
    add_sprite(&mut s, 0, 4, 4, 0);
    let frame = s.render();
    assert_eq!(at(&frame, 4, 4), BLUE);
    assert_eq!(at(&frame, 3, 3), RED);
    assert_eq!(at(&frame, 11, 11), BLUE);
    assert_eq!(at(&frame, 12, 12), BACKDROP);

    // Sprite priority 1 goes behind BG0 at priority 0.
    s.obj(0, 4, 4, 1 | (1 << 10));
    let frame = s.render();
    assert_eq!(at(&frame, 4, 4), RED);
    assert_eq!(at(&frame, 8, 8), BLUE);
}

#[test]
fn sprite_vs_sprite_prefers_lower_oam_index_then_priority() {
    let mut s = Scene::new();
    s.reg(regs::DISPCNT, 0x1000);
    s.color(256 + 16 + 1, GREEN);
    add_sprite(&mut s, 0, 0, 0, 1);
    s.obj(1, 0, 0, 1 | (1 << 12)); // palette bank 1, priority 0
    // A better priority wins even at a higher OAM index.
    assert_eq!(at(&s.render(), 0, 0), GREEN);
    // At equal priority the lower OAM index wins.
    s.obj(1, 0, 0, 1 | (1 << 12) | (1 << 10));
    assert_eq!(at(&s.render(), 0, 0), BLUE);
}

#[test]
fn sprite_shapes_flips_and_wraparound() {
    let mut s = Scene::new();
    s.reg(regs::DISPCNT, 0x1000 | 0x40); // 1D mapping
    s.color(256 + 1, BLUE);
    // 16x8 sprite (tiles 1 and 2), only its top-left pixel is set.
    s.tile4(OBJ_VRAM + 32, |x, y| u8::from(x == 0 && y == 0));
    s.obj(0, (1 << 14) | 10, 20, 1);
    assert_eq!(at(&s.render(), 20, 10), BLUE);
    // H+V flip moves it to the bottom-right corner.
    s.obj(0, (1 << 14) | 10, (3 << 12) | 20, 1);
    let frame = s.render();
    assert_eq!(at(&frame, 35, 17), BLUE);
    assert_eq!(frame.iter().filter(|&&c| c == BLUE).count(), 1);
    // X = 511 is -1; Y = 255 wraps to the top.
    s.tile4(OBJ_VRAM + 64, |_, _| 1);
    s.obj(0, 255, 511, 2);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), BLUE);
    assert_eq!(at(&frame, 6, 6), BLUE);
    // Palette entry 0 (black) shows through elsewhere.
    assert_eq!(at(&frame, 7, 0), 0);
}

#[test]
fn affine_sprite_double_size() {
    let mut s = Scene::new();
    s.reg(regs::DISPCNT, 0x1000);
    s.color(256 + 1, BLUE);
    s.tile4(OBJ_VRAM + 32, |_, _| 1);
    // 8x8 affine sprite scaled 2x, double-size box 16x16 at (50, 50).
    s.obj_matrix(0, [0x80, 0, 0, 0x80]);
    s.obj(0, 0x0300 | 50, 50, 1);
    let frame = s.render();
    let count = frame.iter().filter(|&&c| c == BLUE).count();
    assert_eq!(count, 16 * 16);
    assert_eq!(at(&frame, 50, 50), BLUE);
    assert_eq!(at(&frame, 65, 65), BLUE);
    // Without double size the box is 8x8 and shows the texture's center.
    s.obj(0, 0x0100 | 50, 50, 1);
    let frame = s.render();
    assert_eq!(frame.iter().filter(|&&c| c == BLUE).count(), 8 * 8);
}

#[test]
fn alpha_blend_first_over_second_target() {
    let mut s = one_tile_scene();
    s.color(2, rgb(0, 20, 0));
    s.color(16 + 1, rgb(16, 0, 0));
    s.tile4(64, |_, _| 2);
    s.vram16(0xF000, 2);
    s.reg(regs::DISPCNT, 0x0300);
    s.reg(regs::BG1CNT, (30 << 8) | 1);
    // BG0 1st target, BG1 2nd target, EVA 8, EVB 4.
    s.reg(regs::BLDCNT, 0x0001 | (1 << 6) | 0x0200);
    s.reg(regs::BLDALPHA, 8 | (4 << 8));
    assert_eq!(at(&s.render(), 0, 0), rgb(8, 5, 0));
    // EVA/EVB above 16 are clamped to 16: 16 + 20 saturates green at 31.
    s.reg(regs::BLDALPHA, 0x1F | (0x1F << 8));
    assert_eq!(at(&s.render(), 0, 0), rgb(16, 20, 0));
    s.color(16 + 1, rgb(16, 20, 0));
    assert_eq!(at(&s.render(), 0, 0), rgb(16, 31, 0));
}

#[test]
fn alpha_blend_with_backdrop_and_only_directly_behind() {
    let mut s = one_tile_scene();
    s.color(0, rgb(0, 0, 16));
    s.color(16 + 1, rgb(16, 0, 0));
    s.reg(regs::BLDCNT, 0x0001 | (1 << 6) | 0x2000);
    s.reg(regs::BLDALPHA, 8 | (8 << 8));
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), rgb(8, 0, 8));
    // Pixels without BG0 are the plain backdrop.
    assert_eq!(at(&frame, 8, 0), rgb(0, 0, 16));
}

#[test]
fn brightness_effects() {
    let mut s = one_tile_scene();
    s.color(16 + 1, rgb(16, 8, 0));
    s.reg(regs::BLDY, 8);
    s.reg(regs::BLDCNT, 0x0001 | (2 << 6));
    assert_eq!(at(&s.render(), 0, 0), rgb(23, 19, 15));
    s.reg(regs::BLDCNT, 0x0001 | (3 << 6));
    assert_eq!(at(&s.render(), 0, 0), rgb(8, 4, 0));
    // BLDY is clamped to 16.
    s.reg(regs::BLDY, 31);
    assert_eq!(at(&s.render(), 0, 0), 0);
    // The backdrop isn't a target here.
    assert_eq!(at(&s.render(), 8, 0), BACKDROP);
}

#[test]
fn semi_transparent_sprite_blends_without_blend_mode() {
    let mut s = one_tile_scene();
    s.color(16 + 1, rgb(0, 0, 16));
    s.reg(regs::DISPCNT, 0x1100);
    add_sprite(&mut s, 0, 0, 0, 0);
    s.color(256 + 1, rgb(16, 0, 0));
    // No effect selected, BG0 is a 2nd target, the sprite is
    // semi-transparent (mode 1).
    s.obj(0, 1 << 10, 0, 1);
    s.reg(regs::BLDCNT, 0x0100);
    s.reg(regs::BLDALPHA, 8 | (8 << 8));
    assert_eq!(at(&s.render(), 0, 0), rgb(8, 0, 8));
    // With brightness decrease selected, it still alpha-blends where a 2nd
    // target is behind it...
    s.reg(regs::BLDCNT, 0x0100 | (3 << 6));
    s.reg(regs::BLDY, 16);
    assert_eq!(at(&s.render(), 0, 0), rgb(8, 0, 8));
    // ...and gets the brightness effect elsewhere, as a forced 1st target.
    s.obj(0, 1 << 10, 8, 1);
    assert_eq!(at(&s.render(), 8, 0), 0);
    // Without the 2nd target anywhere, it's a plain sprite.
    s.reg(regs::BLDCNT, 3 << 6);
    assert_eq!(at(&s.render(), 8, 0), rgb(16, 0, 0));
}

#[test]
fn window_masks_layers_and_effects() {
    let mut s = one_tile_scene();
    // Fill the whole map with the red tile.
    for i in 0..1024 {
        s.vram16(0xF800 + i * 2, 1 | (1 << 12));
    }
    s.reg(regs::DISPCNT, 0x0100 | 0x2000);
    s.reg(regs::WIN0H, (10 << 8) | 20);
    s.reg(regs::WIN0V, (5 << 8) | 15);
    s.reg(regs::WININ, 0x00); // nothing inside WIN0
    s.reg(regs::WINOUT, 0x01);
    let frame = s.render();
    assert_eq!(at(&frame, 10, 5), BACKDROP);
    assert_eq!(at(&frame, 19, 14), BACKDROP);
    assert_eq!(at(&frame, 9, 5), RED);
    assert_eq!(at(&frame, 20, 5), RED);
    assert_eq!(at(&frame, 10, 15), RED);

    // Effects only inside the window; a wrapping window (start > end).
    s.reg(regs::WIN0H, (200 << 8) | 20);
    s.reg(regs::WININ, 0x21);
    s.reg(regs::BLDCNT, 0x0001 | (3 << 6));
    s.reg(regs::BLDY, 16);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 5), 0);
    assert_eq!(at(&frame, 239, 5), 0);
    assert_eq!(at(&frame, 100, 5), RED);
    assert_eq!(at(&frame, 0, 20), RED);
}

#[test]
fn obj_window() {
    let mut s = one_tile_scene();
    for i in 0..1024 {
        s.vram16(0xF800 + i * 2, 1 | (1 << 12));
    }
    s.reg(regs::DISPCNT, 0x0100 | 0x1000 | 0x8000);
    add_sprite(&mut s, 0, 30, 30, 0);
    s.obj(0, (2 << 10) | 30, 30, 1); // OBJ window sprite
    s.reg(regs::WINOUT, 0x01); // outside: BG0; OBJ window: nothing
    let frame = s.render();
    assert_eq!(at(&frame, 30, 30), BACKDROP);
    assert_eq!(at(&frame, 37, 37), BACKDROP);
    assert_eq!(at(&frame, 38, 30), RED);
}

#[test]
fn affine_bg_wraparound_and_per_line_step() {
    let mut s = Scene::new();
    s.color(1, RED);
    s.color(2, GREEN);
    // 128x128 affine map at screen block 31 of 8bpp tiles in char block 0:
    // tile 1 is red, tile 2 green; map entry (0, 0) = 1, (1, 0) = 2.
    s.vram[64..128].fill(1);
    s.vram[128..192].fill(2);
    s.vram[0xF800] = 1;
    s.vram[0xF801] = 2;
    s.reg(regs::DISPCNT, 1 | 0x0400);
    s.reg(regs::BG2CNT, 31 << 8);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), RED);
    assert_eq!(at(&frame, 8, 0), GREEN);
    assert_eq!(at(&frame, 128, 0), 0); // no wraparound
    s.reg(regs::BG2CNT, (31 << 8) | (1 << 13));
    let frame = s.render();
    assert_eq!(at(&frame, 128, 0), RED);
    assert_eq!(at(&frame, 0, 128), RED);

    // Reference point X = -8.0 shifts the map right by 8.
    s.reg32(regs::BG2X, -8 << 8);
    let frame = s.render();
    assert_eq!(at(&frame, 8, 0), RED);
    // PB = 1.0: each scanline advances X by one pixel (shear).
    s.reg32(regs::BG2X, 0);
    s.reg(regs::BG2PB, 0x100);
    let frame = s.render();
    assert_eq!(at(&frame, 0, 0), RED);
    assert_eq!(at(&frame, 0, 7), RED);
    assert_eq!(at(&frame, 1, 7), GREEN);
}

#[test]
fn line_io_relatches_affine_reference_point() {
    let mut s = Scene::new();
    s.color(1, RED);
    s.vram[64..128].fill(1);
    s.vram[0xF800] = 1; // only map entry (0, 0) is red
    s.reg(regs::DISPCNT, 1 | 0x0400);
    s.reg(regs::BG2CNT, 31 << 8);
    // Without a rewrite, Y advances by PD per line: rows 0..8 are red.
    let frame = s.render();
    assert_eq!(at(&frame, 0, 7), RED);
    assert_eq!(at(&frame, 0, 8), 0);
    // Rewriting BG2Y on every line keeps showing map row 0. (Alternate
    // between 0 and 1/256 so that each line is a real change.)
    let mut hold = |y: usize, io: &mut [u8; IO_SIZE]| {
        let value = y as i32 & 1;
        io[regs::BG2Y..regs::BG2Y + 4].copy_from_slice(&value.to_le_bytes());
    };
    let frame = s.render_with(Some(&mut hold));
    assert_eq!(at(&frame, 0, 100), RED);
    assert_eq!(at(&frame, 0, 3), RED);
}

#[test]
fn line_io_changes_scroll_per_line() {
    let s = one_tile_scene();
    let mut shift = |y: usize, io: &mut [u8; IO_SIZE]| {
        // Scroll each line so the tile's row y lands at x = 8 * y.
        let hofs = 512u16.wrapping_sub(8 * y as u16) & 0x1FF;
        io[regs::BG0HOFS..regs::BG0HOFS + 2].copy_from_slice(&hofs.to_le_bytes());
    };
    let frame = s.render_with(Some(&mut shift));
    for y in 0..8 {
        assert_eq!(at(&frame, 8 * y, y), RED, "line {y}");
        if y > 0 {
            assert_eq!(at(&frame, 8 * y - 1, y), BACKDROP);
        }
    }
}

#[test]
fn bg_enabled_mid_frame_appears_after_delay() {
    let mut s = one_tile_scene();
    for i in 0..1024 {
        s.vram16(0xF800 + i * 2, 1 | (1 << 12));
    }
    s.reg(regs::DISPCNT, 0);
    let mut enable = |y: usize, io: &mut [u8; IO_SIZE]| {
        if y == 50 {
            io[regs::DISPCNT..regs::DISPCNT + 2].copy_from_slice(&0x0100u16.to_le_bytes());
        }
    };
    let frame = s.render_with(Some(&mut enable));
    // Like mGBA: a text layer enabled mid-frame shows up three lines later.
    assert_eq!(at(&frame, 0, 49), BACKDROP);
    assert_eq!(at(&frame, 0, 52), BACKDROP);
    assert_eq!(at(&frame, 0, 53), RED);
}

#[test]
fn bitmap_modes() {
    let mut s = Scene::new();
    s.vram16((5 * 240 + 7) * 2, GREEN);
    s.reg(regs::DISPCNT, 3 | 0x0400);
    let frame = s.render();
    assert_eq!(at(&frame, 7, 5), GREEN);
    assert_eq!(at(&frame, 8, 5), 0);

    // Mode 4 back frame, palette index 9.
    let mut s = Scene::new();
    s.color(9, BLUE);
    s.color(0, BACKDROP);
    s.vram[0xA000 + 3 * 240 + 2] = 9;
    s.reg(regs::DISPCNT, 4 | 0x10 | 0x0400);
    let frame = s.render();
    assert_eq!(at(&frame, 2, 3), BLUE);
    assert_eq!(at(&frame, 0, 0), BACKDROP); // index 0 is transparent

    // Mode 5 is 160x128; the rest is backdrop.
    let mut s = Scene::new();
    s.color(0, BACKDROP);
    for i in 0..160 * 128 {
        s.vram16(i * 2, RED);
    }
    s.reg(regs::DISPCNT, 5 | 0x0400);
    let frame = s.render();
    assert_eq!(at(&frame, 159, 127), RED);
    assert_eq!(at(&frame, 160, 0), BACKDROP);
    assert_eq!(at(&frame, 0, 128), BACKDROP);
}

#[test]
fn bg_mosaic_repeats_block_origin() {
    let mut s = one_tile_scene();
    s.vram[32..64].fill(0);
    // Column 0 is color 1, the rest transparent.
    s.tile4(32, |x, _| u8::from(x == 0));
    s.reg(regs::BG0CNT, (31 << 8) | 0x40);
    s.reg(regs::MOSAIC, 0x0003); // 4-pixel-wide blocks
    let frame = s.render();
    for x in 0..4 {
        assert_eq!(at(&frame, x, 0), RED);
    }
    assert_eq!(at(&frame, 4, 0), BACKDROP);
}
