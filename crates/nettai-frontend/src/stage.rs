//! The battle stage: the scrolling background (layer priority 3) and the
//! field's panels (priority 2), which share one set of tiles and the first
//! nine background palettes.

use crate::compose::{Layer, HEIGHT, WIDTH};
use nettai_assets::{AnimTarget, Background, Bundle, GfxAnim, MapEntry, Palette, PaletteAnim};
use nettai_battle::Battle;
use nettai_battle::field::{self, PanelType};

/// The field's first tile column per panel column x (0..=7), by whether
/// the view is mirrored (`byte_800C0AA`; -5 is off screen).
const PANEL_COLUMNS: [[i32; 8]; 2] = [[-5, 0, 5, 10, 15, 20, 25, 30], [30, 25, 20, 15, 10, 5, 0, -5]];

/// Background tiles and palettes at one moment.
pub struct Stage<'a> {
    assets: &'a Bundle,
    background: Option<&'a Background>,
    /// Background palettes 0..16.
    pub palettes: [Palette; 16],
    /// Tile replacements from running animations: (first tile, frame).
    tile_anims: Vec<(u16, &'a nettai_assets::Tiles)>,
    /// Background scroll in pixels.
    scroll: (i32, i32),
}

/// Frame counters that drive the stage's animations.
#[derive(Clone, Copy, Debug, Default)]
pub struct StageClock {
    /// Frames the background has scrolled and animated for.
    pub background: u32,
    /// Times the field has been drawn (panel palettes cycle with it).
    pub field: u32,
}

impl StageClock {
    /// The clock for a battle. The background has moved once per frame
    /// since the round's init began (one frame more in the first round of
    /// a set, whose background is set up a frame earlier; measured against
    /// the original); the field is drawn once per running tick.
    pub fn of(b: &Battle) -> StageClock {
        let head_start = (b.setup.score.round == 0) as u32;
        StageClock { background: b.round.frames.wrapping_add(head_start), field: b.round.ticks }
    }
}

impl<'a> Stage<'a> {
    pub fn new(assets: &'a Bundle, background_id: u8, clock: StageClock) -> Stage<'a> {
        let background = assets.background(background_id);
        let mut palettes = [[0u16; 16]; 16];
        let f = &assets.field;
        for (i, p) in f.palettes.iter().enumerate() {
            if let Some(slot) = palettes.get_mut(f.first_palette as usize + i) {
                *slot = *p;
            }
        }
        for anim in &f.palette_anims {
            if let (Some(p), Some(slot)) = (panel_palette(anim, clock.field), palettes.get_mut(anim.slot as usize)) {
                *slot = p;
            }
        }
        let mut tile_anims = Vec::new();
        let mut scroll = (0, 0);
        if let Some(bg) = background {
            if let Some(p) = bg.palette {
                palettes[0] = p;
            }
            let n = clock.background;
            let counter = |step: i32| (((step.wrapping_mul(n as i32)) as u32 >> 4) & 0xFFFF) as i32;
            scroll = (counter(bg.scroll.0), counter(bg.scroll.1));
            for anim in &bg.anims {
                let Some(frame) = anim_frame(anim, n) else { continue };
                let fr = &anim.frames[frame];
                match anim.target {
                    AnimTarget::Tiles { first, .. } => tile_anims.push((first, &fr.tiles)),
                    AnimTarget::Palettes { first, .. } => {
                        for (i, p) in fr.palettes.iter().enumerate() {
                            if let Some(slot) = palettes.get_mut(first as usize + i) {
                                *slot = *p;
                            }
                        }
                    }
                    AnimTarget::Nothing => {}
                }
            }
        }
        Stage { assets, background, palettes, tile_anims, scroll }
    }

    /// A tile of the shared tile block by number.
    fn tile(&self, n: u16) -> Option<&'a [u8]> {
        for &(first, tiles) in self.tile_anims.iter().rev() {
            if n >= first {
                if let Some(t) = tiles.get((n - first) as usize) {
                    return Some(t);
                }
            }
        }
        if let Some(bg) = self.background {
            if n >= bg.first_tile {
                if let Some(t) = bg.tiles.get((n - bg.first_tile) as usize) {
                    return Some(t);
                }
            }
        }
        let f = &self.assets.field;
        n.checked_sub(f.first_tile).and_then(|i| f.tiles.get(i as usize))
    }

    fn put(&self, layer: &mut Layer, e: MapEntry, x: i32, y: i32) {
        if let Some(t) = self.tile(e.tile) {
            layer.draw_tile(t, &self.palettes[e.palette as usize & 15], x, y, e.hflip, e.vflip);
        }
    }

    /// The background layer.
    pub fn draw_background(&self, layer: &mut Layer) {
        let Some(bg) = self.background else { return };
        let (w, h) = (bg.map_width as i32 * 8, bg.map_height as i32 * 8);
        if w == 0 || h == 0 {
            return;
        }
        // Draw the tiles overlapping the screen, then let the layer clip.
        let (sx, sy) = (self.scroll.0.rem_euclid(w), self.scroll.1.rem_euclid(h));
        for ty in 0..=(HEIGHT as i32 / 8 + 1) {
            for tx in 0..=(WIDTH as i32 / 8 + 1) {
                let (px, py) = (tx * 8 - sx % 8, ty * 8 - sy % 8);
                let mx = ((sx / 8 + tx) % bg.map_width as i32) as usize;
                let my = ((sy / 8 + ty) % bg.map_height as i32) as usize;
                let e = bg.map[my * bg.map_width as usize + mx];
                self.put(layer, e, px, py);
            }
        }
    }

    /// The field layer, as `sub_800C5E0` draws it every frame: each
    /// panel's 5x3 tile block by its displayed type and owner (from the
    /// viewer's side), highlights, missing panels, and front edges. The
    /// layer scrolls with the camera (a shake moves it by whole pixels).
    pub fn draw_field(&self, b: &Battle, layer: &mut Layer, local_side: u8, view: &crate::objects::View) {
        let (cx, cy) = (view.camera.0 >> 16, view.camera.1 >> 16);
        let f = &self.assets.field;
        if f.panels.is_empty() {
            return;
        }
        let mirror = (local_side & 1) as usize;
        let block = |layer: &mut Layer, entries: &[MapEntry], col: i32, row: i32, w: i32| {
            for (i, &e) in entries.iter().enumerate() {
                let (dx, dy) = (i as i32 % w, i as i32 / w);
                self.put(layer, e, (col + dx) * 8 - cx, (row + dy) * 8 - cy);
            }
        };
        let blank = |layer: &mut Layer, col: i32, row: i32, w: i32, h: i32| {
            for y in row * 8 - cy..(row + h) * 8 - cy {
                for x in col * 8 - cx..(col + w) * 8 - cx {
                    if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                        layer.pixels[y as usize * WIDTH + x as usize] = crate::compose::CLEAR;
                    }
                }
            }
        };
        for y in 0..5u8 {
            for x in 0..8u8 {
                if !field::is_valid(x, y) {
                    continue;
                }
                let p = &b.field.panels[y as usize][x as usize];
                let col = PANEL_COLUMNS[mirror][x as usize];
                let row = 3 * y as i32 + 6;
                let edge_row = 3 * (y as i32 + 1) + 6;
                if !p.visible {
                    blank(layer, col, row, 5, 3);
                    blank(layer, col, edge_row, 5, 1);
                    continue;
                }
                // A blink (`object_setPanelTypeBlink`) shows before a
                // highlight.
                if p.highlight != 0 && p.blink.is_none() {
                    let h = &f.highlights[(p.highlight as usize - 1).min(1)];
                    block(layer, h, col, row, 5);
                } else {
                    let (mut kind, alliance) = p.blink.unwrap_or((p.display_kind, p.display_alliance));
                    if mirror == 1 {
                        kind = match kind {
                            PanelType::RoadLeft => PanelType::RoadRight,
                            PanelType::RoadRight => PanelType::RoadLeft,
                            k => k,
                        };
                    }
                    let owner = (alliance ^ local_side) as usize & 1;
                    let i = 6 * kind as usize + 3 * owner + y as usize - 1;
                    if let Some(entries) = f.panels.get(i) {
                        block(layer, entries, col, row, 5);
                    }
                }
                if p.front_edge {
                    let owner = (p.display_alliance ^ local_side) as usize & 1;
                    block(layer, &f.front_edges[owner], col, edge_row, 5);
                }
            }
        }
    }
}

/// Which frame of a panel palette animation shows after `calls` draws
/// (`sub_800C192`: when the timer runs out the next frame shows and holds
/// for its duration). None = the palette loaded at battle start still
/// shows.
fn panel_palette(anim: &PaletteAnim, calls: u32) -> Option<Palette> {
    let n = anim.frames.len();
    if n == 0 || calls < anim.initial_timer.max(1) as u32 {
        return None;
    }
    let mut frame = 0usize;
    let mut timer = anim.initial_timer as i32;
    for _ in 0..calls {
        timer -= 1;
        if timer > 0 {
            continue;
        }
        frame = (frame + 1) % n;
        timer = anim.frames[frame].1 as i32;
    }
    Some(anim.frames[frame].0)
}

/// Which frame of a GFX animation shows after `calls` updates
/// (`LoadGFXAnim` shows the first frame at once; `ProcessGFXAnims` moves
/// on when a frame's delay runs out).
fn anim_frame(anim: &GfxAnim, calls: u32) -> Option<usize> {
    if anim.frames.is_empty() {
        return None;
    }
    let mut i = 0usize;
    let mut timer = anim.frames[0].delay as i32;
    for _ in 0..calls {
        timer -= 1;
        if timer > 0 {
            continue;
        }
        if i + 1 < anim.frames.len() {
            i += 1;
        } else if let Some(r) = anim.repeat_from {
            i = r.min(anim.frames.len() - 1);
        } else {
            // Ended: the last frame stays.
            timer = i32::MAX;
            continue;
        }
        timer = anim.frames[i].delay as i32;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::GfxAnimFrame;

    #[test]
    fn panel_palettes_switch_when_the_timer_runs_out() {
        let a = PaletteAnim { slot: 2, frames: vec![([0; 16], 3), ([1; 16], 2), ([2; 16], 4)], initial_timer: 2 };
        assert_eq!(panel_palette(&a, 0), None);
        assert_eq!(panel_palette(&a, 1), None);
        assert_eq!(panel_palette(&a, 2).unwrap()[0], 1);
        assert_eq!(panel_palette(&a, 3).unwrap()[0], 1);
        assert_eq!(panel_palette(&a, 4).unwrap()[0], 2);
        assert_eq!(panel_palette(&a, 7).unwrap()[0], 2);
        assert_eq!(panel_palette(&a, 8).unwrap()[0], 0);
    }

    #[test]
    fn gfx_anims_loop_or_hold() {
        let f = |d| GfxAnimFrame { delay: d, ..Default::default() };
        let mut a = GfxAnim { target: AnimTarget::Nothing, frames: vec![f(2), f(1), f(3)], repeat_from: Some(1) };
        let seq: Vec<usize> = (0..9).map(|n| anim_frame(&a, n).unwrap()).collect();
        assert_eq!(seq, vec![0, 0, 1, 2, 2, 2, 1, 2, 2]);
        a.repeat_from = None;
        assert_eq!(anim_frame(&a, 50), Some(2));
    }
}
