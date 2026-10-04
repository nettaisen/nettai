//! The battle stage: the scrolling background (layer priority 3) and the
//! field's panels (priority 2), which share one set of tiles and the first
//! nine background palettes.
//!
//! The field is the match's game's pack's: its tiles, palettes and their
//! cycles, highlights and front edges. A match is of one game, so its
//! field draws what its panels are; a panel type or highlight the field
//! doesn't draw (a pack extracted before it had it) is a normal panel of
//! its owner's, tinted, never a hole, and never another pack's
//! ([`FieldArt`]).

use crate::audit::Problems;
use crate::compose::{Layer, HEIGHT, WIDTH};
use crate::lookups;
use crate::packs::Packs;
use nettai_assets::{AnimTarget, Background, Bundle, Field, GfxAnim, MapEntry, Palette, PaletteAnim};
use nettai_battle::Battle;
use nettai_battle::content::{Content, PackId, RootId};
use nettai_battle::field::{self, PanelType};

/// The field's first tile column per panel column x (0..=7), by whether
/// the view is mirrored (`byte_800C0AA`; -5 is off screen).
const PANEL_COLUMNS: [[i32; 8]; 2] = [[-5, 0, 5, 10, 15, 20, 25, 30], [30, 25, 20, 15, 10, 5, 0, -5]];

/// The color a panel no loaded pack draws is tinted toward (magenta), by
/// half: a normal panel that shows it is no panel of the field's own.
const TINT: [u16; 3] = [31, 0, 31];

/// Where a panel type's or a highlight's art comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// This pack's field (the arena's) draws it.
    Field(PackId),
    /// The arena's field doesn't draw it: a normal panel, tinted.
    Tint,
}

/// Whether the arena's field draws each panel type and each highlight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldArt {
    /// The arena's game's pack: the field's own art.
    pub arena: PackId,
    panels: [Art; PanelType::ALL.len()],
    highlights: [Art; 2],
}

impl FieldArt {
    /// The art of an arena of game `arena` (its root): a panel type or a
    /// highlight is its pack's field's if it draws it, else tinted.
    pub fn of(c: &Content, packs: &Packs, arena: RootId) -> FieldArt {
        let pack = packs.id_of_root(c, arena);
        let field = &packs.bundle(pack).field;
        let panels = PanelType::ALL.map(|t| if field.draws(t as u8) { Art::Field(pack) } else { Art::Tint });
        let highlights = [1, 2].map(|h| if field.highlights.len() >= h { Art::Field(pack) } else { Art::Tint });
        FieldArt { arena: pack, panels, highlights }
    }

    /// Panel type `t`'s art.
    pub fn panel(&self, t: PanelType) -> Art {
        self.panels[t as usize]
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
    /// The arena's game's pack's graphics: the field's own art.
    assets: &'a Bundle,
    /// The background, from its own pack.
    background: Option<&'a Background>,
    /// Background palettes 0..16.
    pub palettes: [Palette; 16],
    /// Tile replacements from running animations: (first tile, frame).
    tile_anims: Vec<(u16, &'a nettai_assets::Tiles)>,
    /// Background scroll in pixels.
    scroll: (i32, i32),
    /// Which panel types and highlights the field draws.
    art: FieldArt,
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

/// What a panel shows: a block of the arena's field, and whether tinted.
type Shown<'f> = (&'f [MapEntry], bool);

impl<'a> Stage<'a> {
    /// The field of the arena's game (`arena`, its root), of `packs`,
    /// behind `background` (none: the backdrop).
    pub fn new(packs: &Packs<'a>, c: &Content, arena: RootId, background: Option<&'a Background>, clock: StageClock) -> Stage<'a> {
        let art = FieldArt::of(c, packs, arena);
        let assets = packs.bundle(art.arena);
        let mut palettes = field_palettes(&assets.field, clock);
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
        Stage { assets, background, palettes, tile_anims, scroll, art }
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

    /// A map entry of the field, with the stage's tiles and palettes;
    /// `tinted`, its colors tinted ([`tint`]).
    fn put_field(&self, layer: &mut Layer, e: MapEntry, x: i32, y: i32, tinted: bool) {
        if !tinted {
            return self.put(layer, e, x, y);
        }
        let Some(tile) = self.tile(e.tile) else { return };
        layer.draw_tile(tile, &self.palettes[e.palette as usize & 15].map(tint), x, y, e.hflip, e.vflip);
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
                self.put_field(layer, e, (col + dx) * 8 - cx, (row + dy) * 8 - cy, tinted);
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
/// viewer's side (a road's direction mirrored on the right-hand console),
/// from the arena's `field` where `art` says it draws it, else a normal
/// panel tinted.
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
    let tinted = |what: u8, problems: &mut Problems| {
        lookups::panel_tint(c, art.arena, what, problems);
        match art.panel(PanelType::Normal) {
            Art::Field(from) => lookups::panel_block(c, field, from, PanelType::Normal as u8, owner, y, problems).map(|e| (e, true)),
            Art::Tint => None,
        }
    };
    if p.highlight != 0 && p.blink.is_none() {
        return match art.highlight(p.highlight) {
            Art::Field(_) => Some((&field.highlights[highlight_index(p.highlight)][..], false)),
            Art::Tint => tinted(lookups::HIGHLIGHT_TINT + highlight_index(p.highlight) as u8 + 1, problems),
        };
    }
    if local_side & 1 == 1 {
        kind = match kind {
            PanelType::RoadLeft => PanelType::RoadRight,
            PanelType::RoadRight => PanelType::RoadLeft,
            k => k,
        };
    }
    match art.panel(kind) {
        Art::Field(from) => lookups::panel_block(c, field, from, kind as u8, owner, y, problems).map(|e| (e, false)),
        Art::Tint => tinted(kind as u8, problems),
    }
}

/// The lookups `Stage::draw_field` makes, without drawing (`--audit`): what
/// each shown panel shows.
pub fn field_lookups(b: &Battle, packs: &Packs, local_side: u8, problems: &mut Problems) {
    let art = FieldArt::of(&b.content, packs, b.games.arena);
    let field = &packs.bundle(art.arena).field;
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
    use nettai_assets::{GfxAnimFrame, Tiles};
    use nettai_battle::content::{RootManifest, testing};

    const RED: u16 = 0x001F;
    const BLUE: u16 = 0x7C00;
    const GREEN: u16 = 0x03E0;

    /// A field of one solid tile (number 0xA3) drawing `types` (a block of
    /// palette slot 1 each, colored `colors[0]`), with `highlights`
    /// highlights (palette slot 2, `colors[1]`).
    fn field(types: &[PanelType], highlights: usize, colors: [u16; 2]) -> Field {
        let entry = |palette| MapEntry { tile: 0xA3, hflip: false, vflip: false, palette };
        let palette = |c: u16| std::array::from_fn(|i| if i == 1 { c } else { 0 });
        Field {
            tiles: Tiles { pixels: vec![1; Tiles::TILE] },
            first_tile: 0xA3,
            palettes: vec![palette(colors[0]), palette(colors[1])],
            first_palette: 1,
            panel_types: types.iter().map(|&t| t as u8).collect(),
            panels: vec![[entry(1); 15]; 6 * types.len()],
            highlights: vec![[entry(2); 15]; highlights],
            ..Field::default()
        }
    }

    /// A battle on the test content with a second game, `twin`, beside it
    /// (each with its own pack): the arena's game (the test root) names
    /// every panel type but the sea, which twin names; and a field with
    /// the arena's panels `types` at (1, 1), (2, 1) and (3, 1), and
    /// highlight 2 at (1, 2).
    fn battle(types: [PanelType; 3]) -> Battle {
        let mut c = testing::build();
        testing::add_pack(&mut c, "twin", Default::default(), Default::default());
        c.scripts.add_root(RootManifest::named("twin"), Default::default());
        c.define().unwrap_or_else(|e| panic!("{e}"));
        let (home, twin) = (c.defs.root_id(testing::ROOT).expect("the test game"), c.defs.root_id("twin").expect("twin"));
        for t in PanelType::ALL {
            c.rules[home.index()].panels.types[t as usize].named = t != PanelType::Sea;
            c.rules[twin.index()].panels.types[t as usize].named = t == PanelType::Sea;
        }
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(100));
        setup.content = c.hash();
        setup.settings = nettai_battle::BattleSettings::on(&c, c.stage_by_key(testing::LINK_BATTLE));
        let mut b = Battle::new(setup, std::sync::Arc::new(c));
        for (x, t) in (1..=3).zip(types) {
            b.field.panels[1][x].display_kind = t;
        }
        b.field.panels[2][1].highlight = 2;
        for row in &mut b.field.panels[1..=3] {
            for p in &mut row[1..=6] {
                p.visible = true;
            }
        }
        b
    }

    /// A panel type or highlight the arena's field doesn't draw is a normal
    /// panel, tinted: never another pack's, even one loaded whose game
    /// names it (a match is of one game).
    #[test]
    fn a_panel_the_field_doesnt_draw_is_tinted() {
        let b = battle([PanelType::Sea, PanelType::Lava, PanelType::Normal]);
        let c = &b.content;
        // The arena's field draws every type but the sea and lava, and one
        // highlight; twin's the sea, and two highlights.
        let own_types: Vec<PanelType> = PanelType::ALL.into_iter().filter(|t| !matches!(t, PanelType::Sea | PanelType::Lava)).collect();
        let (arena, twin) = (field(&own_types, 1, [RED, 0]), field(&[PanelType::Sea], 2, [BLUE, GREEN]));
        let (test_pack, twin_pack) = (c.assets.pack(testing::ROOT).unwrap(), c.assets.pack("twin").unwrap());
        let (a, t) = (Bundle { field: arena, ..Bundle::default() }, Bundle { field: twin, ..Bundle::default() });
        let mut by_pack = vec![&a, &a];
        by_pack[twin_pack.index()] = &t;
        let packs = Packs::new(by_pack, test_pack);

        let art = FieldArt::of(c, &packs, b.games.arena);
        assert_eq!(art.panel(PanelType::Normal), Art::Field(test_pack));
        assert_eq!((art.panel(PanelType::Sea), art.panel(PanelType::Lava)), (Art::Tint, Art::Tint), "not twin's sea");
        assert_eq!((art.highlight(1), art.highlight(2)), (Art::Field(test_pack), Art::Tint));

        let stage = Stage::new(&packs, c, b.games.arena, None, StageClock::default());
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
        assert_eq!(at(&layer, 20, 108), tint(RED), "highlight 2: tinted");
        assert!(problems.is_empty(), "{:?}", problems.lines());
        assert!(problems.said_lines().iter().any(|l| l.contains("panel type 14")), "{:?}", problems.said_lines());
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
