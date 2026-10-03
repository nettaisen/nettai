//! The battle stage: the scrolling background (layer priority 3) and the
//! field's panels (priority 2), which share one set of tiles and the first
//! nine background palettes.
//!
//! The field is the arena's game's pack's (docs/design/rules-in-luau.md
//! §7.4, as built): its tiles, palettes and their cycles, highlights and
//! front edges. A panel type that field doesn't draw (a BN5 sea panel in a
//! BN6 arena) comes from the field of the pack of the first loaded game
//! whose `panels` section names the type, the simulation's own fallback,
//! with that field's own tiles, palettes and cycles; a type no loaded
//! pack's field draws is a normal panel of its owner's, tinted, never a
//! hole. A highlight the arena's field lacks is borrowed, or falls back,
//! the same way ([`FieldArt`]).

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
    /// This pack's field draws it.
    Field(PackId),
    /// No loaded pack's field draws it: a normal panel, tinted.
    Tint,
}

/// Which pack's field draws each panel type and each highlight in an
/// arena (docs/design/rules-in-luau.md §7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldArt {
    /// The arena's game's pack: the field's own art.
    pub arena: PackId,
    panels: [Art; PanelType::ALL.len()],
    highlights: [Art; 2],
}

impl FieldArt {
    /// The art of an arena of game `arena` (its root). A panel type is the
    /// arena's pack's field's if it draws the type; else the field of the
    /// pack of the first loaded game, in root order, whose `panels` section
    /// names the type (the order the simulation takes a type's rule in,
    /// `sections::fill_panel_types`), if it draws it; else tinted. A
    /// highlight is the arena's field's if it has it; else the first loaded
    /// game's field's, in root order, that has it; else tinted.
    pub fn of(c: &Content, packs: &Packs, arena: RootId) -> FieldArt {
        let pack = packs.id_of_root(c, arena);
        let field = |p: PackId| &packs.bundle(p).field;
        let roots = || (0..c.rules.len()).map(|i| RootId(i as u8));
        let panels = PanelType::ALL.map(|t| {
            if field(pack).draws(t as u8) {
                return Art::Field(pack);
            }
            let named = roots().find(|r| c.rules_of(*r).panels.types.get(t as usize).is_some_and(|rule| rule.named));
            match named.map(|r| packs.id_of_root(c, r)) {
                Some(p) if field(p).draws(t as u8) => Art::Field(p),
                _ => Art::Tint,
            }
        });
        let highlights = [1, 2].map(|h| {
            let has = |p: PackId| field(p).highlights.len() >= h;
            if has(pack) {
                return Art::Field(pack);
            }
            roots().map(|r| packs.id_of_root(c, r)).find(|&p| has(p)).map_or(Art::Tint, Art::Field)
        });
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

    /// The packs other than the arena's whose fields it borrows from.
    fn borrowed(&self) -> Vec<PackId> {
        let mut out = Vec::new();
        for a in self.panels.iter().chain(&self.highlights) {
            if let Art::Field(p) = *a
                && p != self.arena
                && !out.contains(&p)
            {
                out.push(p);
            }
        }
        out
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
    /// Which pack's field draws each panel type and highlight.
    art: FieldArt,
    /// The other packs' fields the art borrows from, each with its own
    /// palettes and their cycles at this moment (a palette set of its own:
    /// the arena's are left as they are).
    borrowed: Vec<(PackId, &'a Field, [Palette; 16])>,
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

/// What a panel shows: a pack's block, drawn from that pack's field, and
/// whether tinted.
type Shown<'f> = (PackId, &'f [MapEntry], bool);

impl<'a> Stage<'a> {
    /// The field of the arena's game (`arena`, its root) and the fields it
    /// borrows from, of `packs`, behind `background` (none: the backdrop).
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
        let borrowed = art
            .borrowed()
            .into_iter()
            .map(|p| {
                let f = &packs.bundle(p).field;
                (p, f, field_palettes(f, clock))
            })
            .collect();
        Stage { assets, background, palettes, tile_anims, scroll, art, borrowed }
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

    /// Pack `p`'s field: the arena's, or one the art borrows from.
    fn field_of(&self, p: PackId) -> &'a Field {
        match self.borrowed.iter().find(|(q, ..)| *q == p) {
            Some((_, f, _)) => f,
            None => &self.assets.field,
        }
    }

    /// A map entry of pack `from`'s field: the arena's with the stage's
    /// tiles and palettes, a borrowed field's with its own; `tinted`, its
    /// colors tinted ([`tint`]).
    fn put_from(&self, layer: &mut Layer, from: PackId, e: MapEntry, x: i32, y: i32, tinted: bool) {
        let (tile, palette) = match self.borrowed.iter().find(|(q, ..)| *q == from) {
            Some((_, f, palettes)) => {
                (e.tile.checked_sub(f.first_tile).and_then(|i| f.tiles.get(i as usize)), &palettes[e.palette as usize & 15])
            }
            None if !tinted => return self.put(layer, e, x, y),
            None => (self.tile(e.tile), &self.palettes[e.palette as usize & 15]),
        };
        let Some(tile) = tile else { return };
        match tinted {
            true => layer.draw_tile(tile, &palette.map(tint), x, y, e.hflip, e.vflip),
            false => layer.draw_tile(tile, palette, x, y, e.hflip, e.vflip),
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
        let block = |layer: &mut Layer, from: PackId, entries: &[MapEntry], col: i32, row: i32, w: i32, tinted: bool| {
            for (i, &e) in entries.iter().enumerate() {
                let (dx, dy) = (i as i32 % w, i as i32 / w);
                self.put_from(layer, from, e, (col + dx) * 8 - cx, (row + dy) * 8 - cy, tinted);
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
        let field_of = |p: PackId| self.field_of(p);
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
                if let Some((from, entries, tinted)) = shown(&b.content, &self.art, &field_of, p, local_side, y, problems) {
                    block(layer, from, entries, col, row, 5, tinted);
                }
                if p.front_edge {
                    let owner = (p.display_alliance ^ local_side) as usize & 1;
                    block(layer, self.art.arena, &f.front_edges[owner], col, edge_row, 5, false);
                }
            }
        }
    }
}

/// What panel `p` (in row `y`) shows, with the lookups drawing it makes: a
/// highlight (a blink, `object_setPanelTypeBlink`, shows before one), else
/// its block by its displayed type (or its blink's) and owner, from the
/// viewer's side (a road's direction mirrored on the right-hand console),
/// each from the field `art` says, or a normal panel tinted.
fn shown<'f>(
    c: &Content,
    art: &FieldArt,
    field_of: &dyn Fn(PackId) -> &'f Field,
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
            Art::Field(from) => {
                lookups::panel_block(c, field_of(from), from, PanelType::Normal as u8, owner, y, problems).map(|e| (from, e, true))
            }
            Art::Tint => None,
        }
    };
    if p.highlight != 0 && p.blink.is_none() {
        return match art.highlight(p.highlight) {
            Art::Field(from) => Some((from, &field_of(from).highlights[highlight_index(p.highlight)][..], false)),
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
        Art::Field(from) => lookups::panel_block(c, field_of(from), from, kind as u8, owner, y, problems).map(|e| (from, e, false)),
        Art::Tint => tinted(kind as u8, problems),
    }
}

/// The lookups `Stage::draw_field` makes, without drawing (`--audit`): what
/// each shown panel shows.
pub fn field_lookups(b: &Battle, packs: &Packs, local_side: u8, problems: &mut Problems) {
    let art = FieldArt::of(&b.content, packs, b.games.arena);
    if packs.bundle(art.arena).field.panels.is_empty() {
        return;
    }
    let field_of = |p: PackId| &packs.bundle(p).field;
    for y in 0..5u8 {
        for x in 0..8u8 {
            let p = &b.field.panels[y as usize][x as usize];
            if field::is_valid(x, y) && p.visible {
                shown(&b.content, &art, &field_of, p, local_side, y, problems);
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

    /// docs/design/rules-in-luau.md §7.4: a panel type the arena's field
    /// doesn't draw comes from the field of the game that names it, in its
    /// own palettes; one no loaded field draws is a normal panel, tinted; a
    /// highlight the arena's field lacks is another's.
    #[test]
    fn a_mixed_field_borrows_the_art_it_lacks_or_tints_a_normal_panel() {
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
        assert_eq!(art.panel(PanelType::Sea), Art::Field(twin_pack), "the sea is twin's, whose game names it");
        assert_eq!(art.panel(PanelType::Lava), Art::Tint, "no loaded field draws lava");
        assert_eq!((art.highlight(1), art.highlight(2)), (Art::Field(test_pack), Art::Field(twin_pack)));

        let stage = Stage::new(&packs, c, b.games.arena, None, StageClock::default());
        let mut layer = Layer::new(2, 2);
        let view = crate::objects::View { camera: (0, 0, 0), mirror: false, fade: Default::default() };
        let mut problems = Problems::default();
        stage.draw_field(&b, &mut layer, 0, &view, &mut problems);
        // Row 1's panels are 24 pixels from y 72, columns 40 wide from
        // x -40 (column 0 is off screen).
        let at = |layer: &Layer, x: usize, y: usize| layer.pixels[y * WIDTH + x];
        assert_eq!(at(&layer, 20, 84), BLUE, "the sea, in twin's colors");
        assert_eq!(at(&layer, 60, 84), tint(RED), "lava: a normal panel, tinted");
        assert_ne!(tint(RED), RED);
        assert_eq!(at(&layer, 100, 84), RED, "a normal panel");
        assert_eq!(at(&layer, 20, 108), GREEN, "highlight 2, twin's");
        assert!(problems.is_empty(), "{:?}", problems.lines());
        assert!(problems.said_lines().iter().any(|l| l.contains("panel type 14")), "{:?}", problems.said_lines());

        // Without twin's pack loaded, the sea is tinted too, and so is
        // highlight 2.
        let one = Packs::one(&a);
        let art = FieldArt::of(c, &one, b.games.arena);
        assert_eq!((art.panel(PanelType::Sea), art.highlight(2)), (Art::Tint, Art::Tint));
        let stage = Stage::new(&one, c, b.games.arena, None, StageClock::default());
        layer.clear();
        stage.draw_field(&b, &mut layer, 0, &view, &mut problems);
        assert_eq!((at(&layer, 20, 84), at(&layer, 20, 108)), (tint(RED), tint(RED)));
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
