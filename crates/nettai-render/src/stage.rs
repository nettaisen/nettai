//! The battle stage: the scrolling background (layer priority 3) and the
//! field's panels (priority 2), which share one set of tiles and the first
//! nine background palettes.
//!
//! The field is the game's (docs/design/rules-in-luau.md §7.4, as
//! built; a match plays one game, docs/design/content-model-v2.md §4.0):
//! its tiles, palettes and their cycles, highlights and front edges. A
//! panel type that field doesn't draw is a normal panel of its owner's,
//! tinted, never a hole; so is a highlight it lacks ([`FieldArt`]).

use crate::audit::Problems;
use crate::compose::{Layer, HEIGHT};
use crate::lookups;
use crate::graphics::Graphics;
use nettai_assets::{AnimTarget, Background, Bundle, Field, GfxAnim, MapEntry, Palette, PaletteAnim};
use nettai_battle::Battle;
use nettai_battle::content::Content;
use nettai_battle::field::{self, PanelType};

/// The field's first tile column per panel column x (0..=7), by whether
/// the view is mirrored (`byte_800C0AA`; -5 is off screen).
const PANEL_COLUMNS: [[i32; 8]; 2] = [[-5, 0, 5, 10, 15, 20, 25, 30], [30, 25, 20, 15, 10, 5, 0, -5]];

/// The color a panel the field doesn't draw is tinted toward (magenta), by
/// half: a normal panel that shows it is no panel of the field's own.
const TINT: [u16; 3] = [31, 0, 31];

/// Where a panel type's or a highlight's art comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// The game's field draws it.
    Field,
    /// The field doesn't draw it: a normal panel, tinted.
    Tint,
}

/// Whether the game's field draws each panel type and each highlight
/// (docs/design/rules-in-luau.md §7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldArt {
    /// By the game's panel types' numbers.
    panels: Vec<Art>,
    highlights: [Art; 2],
}

impl FieldArt {
    /// The art of the game's arena: a panel type is the game's field's if
    /// it draws the type, else tinted; so is a highlight.
    pub fn of(c: &Content, graphics: &Graphics) -> FieldArt {
        let field = &graphics.bundle().field;
        let panels = c.rules().panels.names.iter().map(|t| if field.draws(t) { Art::Field } else { Art::Tint }).collect();
        let highlights = [1, 2].map(|h| if field.highlights.len() >= h { Art::Field } else { Art::Tint });
        FieldArt { panels, highlights }
    }

    /// Panel type `t`'s art (a type its game hasn't, tinted).
    pub fn panel(&self, t: PanelType) -> Art {
        self.panels.get(t.0 as usize).copied().unwrap_or(Art::Tint)
    }

    /// Highlight `h`'s art (1 or 2; a larger number reads as 2, as the
    /// original reads it).
    pub fn highlight(&self, h: u8) -> Art {
        self.highlights[highlight_index(h)]
    }
}

/// A highlight's place in a field's `highlights` (`highlight - 1`, at most
/// the second).
fn highlight_index(h: u8) -> usize {
    (h.max(1) as usize - 1).min(1)
}

/// Background tiles and palettes at one moment.
pub struct Stage<'a> {
    /// The game's graphics: the field's own art.
    assets: &'a Bundle,
    /// The background, from its own game's graphics.
    background: Option<&'a Background>,
    /// Background palettes 0..16.
    pub palettes: [Palette; 16],
    /// Tile replacements from running animations: (first tile, frame).
    tile_anims: Vec<(u16, &'a nettai_assets::Tiles)>,
    /// Background scroll in pixels.
    scroll: (i32, i32),
    /// The part of a pixel past `scroll`, in sixteenths, of a scroll by
    /// sixteenths of a pixel a frame (the game's scroll counters keep it:
    /// the screen shows the whole pixels; the enhanced mode's glide shows
    /// it, `Stage::draw_background_wide`).
    sub: (u8, u8),
    /// Whether the game's field draws each panel type and highlight.
    art: FieldArt,
}

/// Frame counters that drive the stage's animations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StageClock {
    /// Frames the background has scrolled and animated for.
    pub background: u32,
    /// Times the field has been drawn (panel palettes cycle with it): the
    /// battle's ticks, one a frame from its first (whose intro starts a
    /// speeding scroll, `nettai_assets::Speeding`), through the custom
    /// screen and the pause.
    pub field: u32,
}

impl StageClock {
    /// The clock for a battle. The background has moved once per frame
    /// since the round's init began (one frame more in the first round of
    /// a set of a game whose intro waits a tick after its init, EXE6's and
    /// EXE5's: the background is set up a frame earlier; none in EXE4's,
    /// whose intro goes on from its init on the same tick, `flow`'s
    /// `intro_steps_on_init`; measured against the originals); the field is
    /// drawn once per running tick.
    pub fn of(b: &Battle) -> StageClock {
        let head_start = (b.setup.score.round == 0 && !b.game_rules().flow.intro_steps_on_init) as u32;
        StageClock { background: b.round.frames.wrapping_add(head_start), field: b.round.ticks }
    }

    /// This clock moved on by `by`'s frames, each counter wrapping as the
    /// battle's do.
    pub fn ahead(self, by: StageClock) -> StageClock {
        StageClock { background: self.background.wrapping_add(by.background), field: self.field.wrapping_add(by.field) }
    }

    /// A frame more, as a tick of `b` would have moved [`StageClock::of`]
    /// with nothing else of it changing: the background's frame, and the
    /// field's where the round runs (in a round's end state the battle's
    /// ticks stand still, `round.ticks`, and its field's clock with them).
    pub fn step(&mut self, b: &Battle) {
        self.background = self.background.wrapping_add(1);
        if b.round.top == nettai_battle::battle::top::RUNNING {
            self.field = self.field.wrapping_add(1);
        }
    }
}

/// A field's palettes at a moment: its own from `first_palette` on, and
/// its cycling panel palettes' frames.
fn field_palettes(f: &Field, clock: StageClock) -> [Palette; 16] {
    let mut palettes = [[0u16; 16]; 16];
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
    palettes
}

/// A color tinted toward [`TINT`] by half.
fn tint(c: u16) -> u16 {
    let ch = |shift: u16, to: u16| (((c >> shift) & 31) + to) / 2;
    ch(0, TINT[0]) | ch(5, TINT[1]) << 5 | ch(10, TINT[2]) << 10
}

/// What a panel shows: a block of the game's field, and whether tinted.
type Shown<'f> = (&'f [MapEntry], bool);

/// How many palettes, from background palette 0, a palette flash fills
/// with white (`sub_80E10C0`, EXE4's 0x080E2A34: the stage's nine).
const FLASH_PALETTES: usize = 9;

impl<'a> Stage<'a> {
    /// The field of the arena's game (`arena`, its root) and the fields it
    /// borrows from, of `packs`, behind `background` (none: the backdrop).
    /// `flashed`: a palette flash that comes before the fades fills the
    /// stage's palettes with white this frame, in the palette transforms'
    /// first slot: after the palettes' own frames, before the background's
    /// palette shifts (EXE4's 0x0800258C runs its slots in order: the
    /// flash's 0, a background's shift 3, the dimming's 18), so a shifted
    /// palette shows the shifted white.
    pub fn new(graphics: &Graphics<'a>, c: &Content, background: Option<&'a Background>, clock: StageClock, flashed: bool) -> Stage<'a> {
        let art = FieldArt::of(c, graphics);
        let assets = graphics.bundle();
        let mut palettes = field_palettes(&assets.field, clock);
        let mut tile_anims = Vec::new();
        let mut scroll = (0, 0);
        let mut sub = (0, 0);
        let mut shifts = Vec::new();
        if let Some(bg) = background {
            if let Some(p) = bg.palette {
                palettes[0] = p;
            }
            let n = clock.background;
            let counter = |step: i32| (((step.wrapping_mul(n as i32)) as u32 >> 4) & 0xFFFF) as i32;
            // (A speeding scroll on the background's clock, or from the
            // battle's first frame on its ticks: the background's runs from
            // the round's init, which a recording starts past.)
            scroll = match bg.speeding {
                Some(s) => s.offset(if s.from_battle { clock.field } else { n }),
                None => (counter(bg.scroll.0), counter(bg.scroll.1)),
            };
            // (The counter's low four bits, below the pixel it shows. A
            // speeding scroll moves by whole pixels a frame: none.)
            let part = |step: i32| (step.wrapping_mul(n as i32) & 15) as u8;
            if bg.speeding.is_none() {
                sub = (part(bg.scroll.0), part(bg.scroll.1));
            }
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
                    // (After the palettes' own frames: EXE4's 0x0800258C
                    // shifts the palettes as shown.)
                    AnimTarget::PaletteShift { first, count, darken } => shifts.push((first, count, darken, fr.shift)),
                    AnimTarget::Nothing => {}
                }
            }
        }
        if flashed {
            for slot in palettes.iter_mut().take(FLASH_PALETTES) {
                *slot = [0x7FFF; 16];
            }
        }
        for (first, count, darken, by) in shifts {
            for slot in palettes.iter_mut().skip(first as usize).take(count as usize) {
                *slot = slot.map(|c| nettai_assets::shift_color(c, by, darken));
            }
        }
        Stage { assets, background, palettes, tile_anims, scroll, sub, art }
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

    /// A map entry of the game's field, with the stage's tiles and
    /// palettes; `tinted`, its colors tinted ([`tint`]).
    fn put_from(&self, layer: &mut Layer, e: MapEntry, x: i32, y: i32, tinted: bool) {
        if !tinted {
            return self.put(layer, e, x, y);
        }
        let (tile, palette) = (self.tile(e.tile), &self.palettes[e.palette as usize & 15]);
        let Some(tile) = tile else { return };
        match tinted {
            true => layer.draw_tile(tile, &palette.map(tint), x, y, e.hflip, e.vflip),
            false => layer.draw_tile(tile, palette, x, y, e.hflip, e.vflip),
        }
    }

    /// The background layer, across its whole width (it wraps): a layer
    /// wider than the original's screen shows it on past both of that
    /// screen's edges.
    pub fn draw_background(&self, layer: &mut Layer) {
        let Some(bg) = self.background else { return };
        let (w, h) = (bg.map_width as i32 * 8, bg.map_height as i32 * 8);
        if w == 0 || h == 0 {
            return;
        }
        // Draw the tiles overlapping the layer, from its first column (on
        // the original's screen), then let the layer clip.
        let (sx, sy) = (self.scroll.0.rem_euclid(w), self.scroll.1.rem_euclid(h));
        let left = -layer.origin();
        let first = left - (left + sx).rem_euclid(8);
        for ty in 0..=(HEIGHT as i32 / 8 + 1) {
            for tx in 0..=(layer.width as i32 / 8 + 1) {
                let (px, py) = (first + tx * 8, ty * 8 - sy % 8);
                let mx = ((px + sx) / 8).rem_euclid(bg.map_width as i32) as usize;
                let my = ((sy / 8 + ty) % bg.map_height as i32) as usize;
                let e = bg.map[my * bg.map_width as usize + mx];
                self.put(layer, e, px, py);
            }
        }
    }

    /// The part of a pixel the background has scrolled past what the
    /// screen shows, in sixteenths of a pixel on each axis.
    pub fn background_sub(&self) -> (u8, u8) {
        self.sub
    }

    /// The background as [`Stage::draw_background`] draws it, `w` by `h`
    /// pixels from the top of the original screen's column `left` (wider
    /// and taller than the frame: the pixels a glide shows past its edges),
    /// its colors through `color`, `backdrop` where it is clear.
    pub fn draw_background_wide(&self, left: i32, w: usize, h: usize, backdrop: u16, color: impl Fn(u16) -> u16) -> Vec<u16> {
        let mut out = vec![color(backdrop); w * h];
        let Some(bg) = self.background else { return out };
        let (mw, mh) = (bg.map_width as i32 * 8, bg.map_height as i32 * 8);
        if mw == 0 || mh == 0 {
            return out;
        }
        let (sx, sy) = (self.scroll.0.rem_euclid(mw), self.scroll.1.rem_euclid(mh));
        // (Its columns from `out`'s first; the tiles from the one that
        // holds it.)
        let first = -(left + sx).rem_euclid(8);
        for ty in 0..=(h as i32 / 8 + 1) {
            for tx in 0..=(w as i32 / 8 + 1) {
                let (px, py) = (first + tx * 8, ty * 8 - sy % 8);
                let mx = ((left + px + sx) / 8).rem_euclid(bg.map_width as i32) as usize;
                let my = ((sy / 8 + ty) % bg.map_height as i32) as usize;
                let e = bg.map[my * bg.map_width as usize + mx];
                let Some(tile) = self.tile(e.tile) else { continue };
                let palette = &self.palettes[e.palette as usize & 15];
                for row in 0..8 {
                    let y = py + row;
                    if !(0..h as i32).contains(&y) {
                        continue;
                    }
                    let r = if e.vflip { 7 - row } else { row } as usize;
                    for col in 0..8 {
                        let x = px + col;
                        if !(0..w as i32).contains(&x) {
                            continue;
                        }
                        let c = if e.hflip { 7 - col } else { col } as usize;
                        let i = tile[r * 8 + c];
                        if i != 0 {
                            out[y as usize * w + x as usize] = color(palette[i as usize] & 0x7FFF);
                        }
                    }
                }
            }
        }
        out
    }

    /// The field layer, as `sub_800C5E0` draws it every frame: each
    /// panel's 5x3 tile block by its displayed type and owner (from the
    /// viewer's side), highlights, missing panels, and front edges, each
    /// from the field [`FieldArt`] says. The layer scrolls with the camera
    /// (a shake moves it by whole pixels).
    pub fn draw_field(&self, b: &Battle, layer: &mut Layer, local_side: u8, view: &crate::objects::View, problems: &mut Problems) {
        let (cx, cy) = (view.camera.0 >> 16, view.camera.1 >> 16);
        let f = &self.assets.field;
        if f.panels.is_empty() {
            return;
        }
        let mirror = (local_side & 1) as usize;
        let block = |layer: &mut Layer, entries: &[MapEntry], col: i32, row: i32, w: i32, tinted: bool| {
            for (i, &e) in entries.iter().enumerate() {
                let (dx, dy) = (i as i32 % w, i as i32 / w);
                self.put_from(layer, e, (col + dx) * 8 - cx, (row + dy) * 8 - cy, tinted);
            }
        };
        let blank = |layer: &mut Layer, col: i32, row: i32, w: i32, h: i32| {
            for y in row * 8 - cy..(row + h) * 8 - cy {
                for x in col * 8 - cx..(col + w) * 8 - cx {
                    layer.clear_pixel(x, y);
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
                    // A highlighted panel the game's drawing left alone
                    // shows what it last did (`Panel::kept`).
                    if let Some(kept) = kept_look(&b.content, p)
                        && let Some((entries, tinted)) = shown(&b.content, &self.art, f, &kept, local_side, y, problems)
                    {
                        block(layer, entries, col, row, 5, tinted);
                    }
                    continue;
                }
                if let Some((entries, tinted)) = shown(&b.content, &self.art, f, p, local_side, y, problems) {
                    block(layer, entries, col, row, 5, tinted);
                }
                if p.front_edge {
                    let owner = (p.display_alliance ^ local_side) as usize & 1;
                    block(layer, &f.front_edges[owner], col, edge_row, 5, false);
                }
            }
        }
    }
}

/// What panel `p` (in row `y`) shows, with the lookups drawing it makes: a
/// highlight (a blink, `object_setPanelTypeBlink`, shows before one), else
/// its block by its displayed type (or its blink's) and owner, from the
/// viewer's side (a road's direction mirrored on the right-hand console: the
/// road carrying the other way), each from the game's `field` where `art`
/// says it draws it, or a normal panel tinted.
fn shown<'f>(
    c: &Content,
    art: &FieldArt,
    field: &'f Field,
    p: &field::Panel,
    local_side: u8,
    y: u8,
    problems: &mut Problems,
) -> Option<Shown<'f>> {
    let (mut kind, alliance) = p.blink.unwrap_or((p.display_kind, p.display_alliance));
    let owner = (alliance ^ local_side) as usize & 1;
    // A tinted normal panel of the owner's (`what`: the type or highlight
    // no field draws).
    let normal = c.rules().panels.roles.normal;
    let tinted = |what: u8, problems: &mut Problems| {
        lookups::panel_tint(c, what, problems);
        match art.panel(normal) {
            Art::Field => lookups::panel_block(c, field, normal, owner, y, problems).map(|e| (e, true)),
            Art::Tint => None,
        }
    };
    // A blink shows over a highlight where the panel holds what it is drawn
    // as (`sub_800C5E0` reads the blink's latch first); where the game keeps
    // that in tables of its own, its drawing reads the highlight first
    // (`PanelDrawn::OwnTables`), and the highlight shows through a blink.
    let highlight_first = c.rules().panels.drawn == nettai_battle::content::PanelDrawn::OwnTables;
    if p.highlight != 0 && (p.blink.is_none() || highlight_first) {
        return match art.highlight(p.highlight) {
            Art::Field => Some((&field.highlights[highlight_index(p.highlight)][..], false)),
            Art::Tint => tinted(lookups::HIGHLIGHT_TINT + highlight_index(p.highlight) as u8 + 1, problems),
        };
    }
    if local_side & 1 == 1 {
        kind = c.rules().panels.mirrored(kind);
    }
    match art.panel(kind) {
        Art::Field => lookups::panel_block(c, field, kind, owner, y, problems).map(|e| (e, false)),
        Art::Tint => tinted(kind.0, problems),
    }
}

/// The panel a highlighted panel that isn't visible still shows, where the
/// game's drawing leaves such a panel as it was (`PanelDrawn::OwnTables`):
/// what it last showed, if it was visible then (`Panel::kept`).
fn kept_look(c: &Content, p: &field::Panel) -> Option<field::Panel> {
    let own_tables = c.rules().panels.drawn == nettai_battle::content::PanelDrawn::OwnTables;
    let k = p.kept;
    (own_tables && p.highlight != 0 && k.visible).then(|| field::Panel {
        visible: true,
        highlight: k.highlight,
        blink: None,
        display_kind: k.kind,
        display_alliance: k.alliance,
        ..*p
    })
}

/// The lookups `Stage::draw_field` makes, without drawing (`--audit`): what
/// each shown panel shows.
pub fn field_lookups(b: &Battle, graphics: &Graphics, local_side: u8, problems: &mut Problems) {
    let art = FieldArt::of(&b.content, graphics);
    let field = &graphics.bundle().field;
    if field.panels.is_empty() {
        return;
    }
    for y in 0..5u8 {
        for x in 0..8u8 {
            let p = &b.field.panels[y as usize][x as usize];
            if field::is_valid(x, y) && p.visible {
                shown(&b.content, &art, field, p, local_side, y, problems);
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
    use crate::compose::WIDTH;
    use nettai_assets::{GfxAnimFrame, Tiles};
    use nettai_battle::content::testing;

    const RED: u16 = 0x001F;

    /// A field of one solid tile (number 0xA3) drawing `types` (a block of
    /// palette slot 1 each, colored `colors[0]`), with `highlights`
    /// highlights (palette slot 2, `colors[1]`).
    fn field(types: &[&str], highlights: usize, colors: [u16; 2]) -> Field {
        let entry = |palette| MapEntry { tile: 0xA3, hflip: false, vflip: false, palette };
        let palette = |c: u16| std::array::from_fn(|i| if i == 1 { c } else { 0 });
        Field {
            tiles: Tiles { pixels: vec![1; Tiles::TILE] },
            first_tile: 0xA3,
            palettes: vec![palette(colors[0]), palette(colors[1])],
            first_palette: 1,
            panel_types: types.iter().map(|t| t.to_string()).collect(),
            panels: vec![[entry(1); 15]; 6 * types.len()],
            highlights: vec![[entry(2); 15]; highlights],
            ..Field::default()
        }
    }

    /// A battle on the test content, with a field with the panels `types`
    /// (by name) at (1, 1), (2, 1) and (3, 1), and highlight 2 at (1, 2).
    fn battle(types: [&str; 3]) -> Battle {
        let mut c = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(100));
        testing::on(&mut setup, &c);
        setup.settings = nettai_battle::BattleSettings::on(&c, c.stage_by_key(testing::LINK_BATTLE));
        let mut b = Battle::new(setup, std::sync::Arc::new(c));
        for (x, t) in (1..=3).zip(types) {
            b.field.panels[1][x].display_kind = testing::panel(t);
        }
        b.field.panels[2][1].highlight = 2;
        for row in &mut b.field.panels[1..=3] {
            for p in &mut row[1..=6] {
                p.visible = true;
            }
        }
        b
    }

    /// docs/design/rules-in-luau.md §7.4: a panel type the game's field
    /// doesn't draw is a normal panel, tinted; so is a highlight it lacks
    /// (a match plays one game: nothing is borrowed from another's field).
    #[test]
    fn a_panel_the_field_doesnt_draw_is_a_tinted_normal_panel() {
        let b = battle(["sea", "lava", "normal"]);
        let c = &b.content;
        // The field draws every type but the sea and lava, and one
        // highlight.
        let own_types: Vec<&str> = c.rules().panels.names.iter().map(String::as_str).filter(|t| !matches!(*t, "sea" | "lava")).collect();
        let a = Bundle { field: field(&own_types, 1, [RED, 0]), ..Bundle::default() };
        let graphics = Graphics::of(&a);
        let art = FieldArt::of(c, &graphics);
        assert_eq!(art.panel(testing::panel("normal")), Art::Field);
        assert_eq!((art.panel(testing::panel("sea")), art.panel(testing::panel("lava"))), (Art::Tint, Art::Tint));
        assert_eq!((art.highlight(1), art.highlight(2)), (Art::Field, Art::Tint));

        let stage = Stage::new(&graphics, c, None, StageClock::default(), false);
        let mut layer = Layer::new(2, 2);
        let view = crate::objects::View { camera: (0, 0, 0), mirror: false, fade: Default::default() };
        let mut problems = Problems::default();
        stage.draw_field(&b, &mut layer, 0, &view, &mut problems);
        // Row 1's panels are 24 pixels from y 72, columns 40 wide from
        // x -40 (column 0 is off screen).
        let at = |layer: &Layer, x: usize, y: usize| layer.pixels[y * WIDTH + x];
        assert_eq!(at(&layer, 20, 84), tint(RED), "the sea: a normal panel, tinted");
        assert_eq!(at(&layer, 60, 84), tint(RED), "lava: a normal panel, tinted");
        assert_ne!(tint(RED), RED);
        assert_eq!(at(&layer, 100, 84), RED, "a normal panel");
        assert_eq!(at(&layer, 20, 108), tint(RED), "highlight 2, tinted");
    }

    /// Where a game keeps what its panels are drawn as in tables of its own,
    /// its drawing reads a panel's highlight first: a highlight shows
    /// through a blink, and a highlighted panel that isn't visible keeps
    /// what it last showed. Where the panel holds what it is drawn as, the
    /// blink shows, and a panel that isn't visible is drawn as none.
    #[test]
    fn a_highlight_comes_first_where_panels_are_drawn_from_their_own_tables() {
        use nettai_battle::content::PanelDrawn;
        const GREEN: u16 = 0x03E0;
        let draw = |drawn: PanelDrawn| {
            let mut b = battle(["normal", "normal", "normal"]);
            let mut c = (*b.content).clone();
            c.rules_mut().panels.drawn = drawn;
            b.content = std::sync::Arc::new(c);
            // (1, 1): highlighted under a blink. (2, 1): highlighted while
            // visible, then not visible, still highlighted.
            let normal = testing::panel("normal");
            b.field.panels[1][1].highlight = 1;
            b.field.panels[1][1].blink = Some((normal, 0));
            b.field.panels[1][2].highlight = 1;
            b.keep_panel_looks();
            b.field.panels[1][2].visible = false;
            b.keep_panel_looks();
            let c = &b.content;
            let types: Vec<&str> = c.rules().panels.names.iter().map(String::as_str).collect();
            let a = Bundle { field: field(&types, 1, [RED, GREEN]), ..Bundle::default() };
            let graphics = Graphics::of(&a);
            let stage = Stage::new(&graphics, c, None, StageClock::default(), false);
            let mut layer = Layer::new(2, 2);
            let view = crate::objects::View { camera: (0, 0, 0), mirror: false, fade: Default::default() };
            stage.draw_field(&b, &mut layer, 0, &view, &mut Problems::default());
            let at = |x: usize, y: usize| layer.pixels[y * WIDTH + x];
            (at(20, 84), at(60, 84))
        };
        assert_eq!(draw(PanelDrawn::OwnTables), (GREEN, GREEN), "the highlight through the blink; the highlight kept");
        assert_eq!(draw(PanelDrawn::InPanel), (RED, crate::compose::CLEAR), "the blink; none");
    }

    /// A flash before the fades fills the stage's nine palettes with white
    /// before a background's palette shift: the shifted palette shows the
    /// shifted white (EXE4's darksoul under SparkMan's flash), the others
    /// white, those past them as they are; without it, the shift is of the
    /// background's own colors.
    #[test]
    fn a_flash_before_the_fades_fills_before_the_background_shifts() {
        let b = battle(["normal", "normal", "normal"]);
        let c = &b.content;
        let a = Bundle { field: field(&["normal"], 0, [RED, 0]), ..Bundle::default() };
        let graphics = Graphics::of(&a);
        let shift = 0x2268;
        let frame = GfxAnimFrame { shift, delay: 1, ..Default::default() };
        let bg = Background {
            palette: Some([0x1234; 16]),
            anims: vec![GfxAnim { target: AnimTarget::PaletteShift { first: 0, count: 1, darken: true }, frames: vec![frame], repeat_from: None }],
            ..Background::default()
        };
        let flashed = Stage::new(&graphics, c, Some(&bg), StageClock::default(), true);
        assert_eq!(flashed.palettes[0], [nettai_assets::shift_color(0x7FFF, shift, true); 16]);
        assert_eq!(flashed.palettes[0][0], 23 | 12 << 5 | 23 << 10, "white less (8, 19, 8)");
        assert!(flashed.palettes[1..9].iter().all(|p| *p == [0x7FFF; 16]));
        assert_eq!(flashed.palettes[9], [0; 16]);
        let plain = Stage::new(&graphics, c, Some(&bg), StageClock::default(), false);
        assert_eq!(plain.palettes[0][0], nettai_assets::shift_color(0x1234, shift, true));
        assert_eq!(plain.palettes[1][1], RED);
    }

    /// A scroll by sixteenths of a pixel a frame: the screen shows its whole
    /// pixels, and the stage keeps the part of a pixel past them (the
    /// counter's low bits, rounding down: left and up too). The wide
    /// background is the layer's where the two overlap, and goes on past it.
    #[test]
    fn a_scroll_by_sixteenths_keeps_the_part_of_a_pixel_past_the_screens() {
        let b = battle(["normal", "normal", "normal"]);
        let c = &b.content;
        let a = Bundle { field: field(&["normal"], 0, [RED, 0]), ..Bundle::default() };
        let graphics = Graphics::of(&a);
        // (Two tiles of 8x8, columns colored by their x: a picture that
        // shows where it is.)
        let tiles = Tiles { pixels: (0..2 * Tiles::TILE).map(|i| (1 + (i / 64) * 7 + i % 8) as u8).collect() };
        let entry = |tile| MapEntry { tile, hflip: false, vflip: false, palette: 0 };
        let palette: [u16; 16] = std::array::from_fn(|i| i as u16 * 0x421);
        let bg = Background {
            tiles,
            first_tile: 0x200,
            map: (0..32 * 32).map(|i| entry(0x200 + (i % 2) as u16)).collect(),
            map_width: 32,
            map_height: 32,
            palette: Some(palette),
            scroll: (-8, -4),
            ..Background::default()
        };
        let at = |frames: u32| Stage::new(&graphics, c, Some(&bg), StageClock { background: frames, field: 0 }, false);
        // Half a pixel left and a quarter up a frame.
        assert_eq!(at(0).background_sub(), (0, 0));
        assert_eq!((at(1).scroll, at(1).background_sub()), ((0xFFFF, 0xFFFF), (8, 12)));
        assert_eq!((at(2).scroll, at(2).background_sub()), ((0xFFFF, 0xFFFF), (0, 8)));
        assert_eq!((at(4).scroll, at(4).background_sub()), ((0xFFFE, 0xFFFF), (0, 0)));
        let stage = at(3);
        let mut layer = Layer::new(3, 1);
        stage.draw_background(&mut layer);
        let wide = stage.draw_background_wide(0, WIDTH + 1, HEIGHT + 1, 0x7C00, |c| c);
        for y in 0..HEIGHT {
            assert_eq!(&wide[y * (WIDTH + 1)..y * (WIDTH + 1) + WIDTH], &layer.pixels[y * WIDTH..(y + 1) * WIDTH], "row {y}");
        }
        // The column past the screen's edge goes on with the picture.
        assert_eq!(wide[WIDTH], wide[WIDTH - 16]);
    }

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
