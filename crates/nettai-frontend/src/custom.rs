//! The custom screen as the original draws it on the local player's
//! console (`sub_8026A28` and its states): the window on the HUD layer
//! (BG3) with the chip under the cursor, the dealt chips and the picked
//! ones; the cursor, the navi's emblem and the Regular chip's frame as
//! sprites; the enemy names on BG0; and what the screen does to the rest
//! of the picture (the HUD and the field move aside, Beast Out's fade).
//!
//! It draws from the engine's `Screen` and its presentation state
//! (`Screen::look`), the content (chip names, codes, elements, damage) and
//! the pack's custom-screen graphics, composing the tile numbers the
//! original's window map names: the tiles the battle loads at fixed places
//! and the ones the screen copies in as it runs. docs/frontend.md §3.

use crate::audit::Problems;
use crate::compose::{Affine, Fade, Layer, SpritePart};
use crate::fonts;
use crate::objects::SpriteList;
use nettai_assets::{Bundle, CustomScreen, Hud, MapEntry, Palette, Picture, Tiles};
use nettai_battle::Battle;
use nettai_battle::battle::{FadeMode, mode};
use nettai_battle::content::{ChipClass, ChipFlags, DamageFormula};
use nettai_battle::custom::screen::{HiddenStage, OK_SLOT, SPECIAL_SLOT};
use nettai_battle::custom::{FolderChip, Phase, Screen, Side, SlotKind, SlotState};

/// The window: 15 columns of 20 rows at the HUD layer's top left.
const COLUMNS: usize = 15;
const ROWS: usize = 20;
/// Where the screen's tiles go in the HUD layer's character block.
const WINDOW_TILE: u16 = 0x01;
const COLUMN_CELL_TILE: u16 = 0x89;
const TURN_LIMIT_TILE: u16 = 0x8D;
const NAME_TILE: u16 = 0x9B;
const ART_TILE: u16 = 0xAB;
const CODE_TILE: u16 = 0xD5;
const ELEMENT_TILE: u16 = 0xD7;
const DIGIT_TILE: u16 = 0xDB;
const SLOT_TILE: u16 = 0xE1;
const COLUMN_ICON_TILE: u16 = 0x125;
const NAME_BAR_TILE: u16 = 0x1D6;
const LAYER_TILES: usize = 0x200;
/// The window's background colours: what the original copies over cells
/// the chip window leaves empty (`byte_802A6C0`, `byte_802A680`,
/// `byte_802A700`: solid 8, 7 and 1).
const BLANK_8: u8 = 8;
const BLANK_7: u8 = 7;
const BLANK_1: u8 = 1;
/// The chip window's name: 8 cells, its pixels shifted to the window's
/// colours from 8 (`sub_80284E2`).
const NAME_CELLS: usize = 8;
const NAME_SHIFT: u8 = 8;
/// A code no chip has: the invalid chip's (blank in the slots).
const NO_CODE: u8 = 0x1B;
/// The slot code glyph an empty slot shows.
const EMPTY_SLOT_CODE: u8 = 0x1B;
/// The damage digits' '?' (`0xAAA` for Muramasa).
const DIGIT_UNKNOWN: usize = 10;
/// The enemy names' cells (`sub_801E574`: 9 a name, 4 names at most).
const ENEMY_NAME_CELLS: usize = 9;
/// The window's offset off the screen.
const SLIDE: u32 = 0x78;
/// The field and the sprites move down 1.5 pixels a tick of the slide
/// (`sub_8026B04`: the camera's Y, 16.16).
const CAMERA_STEP: i32 = 0x18000;
const SLIDE_TICKS: i32 = 10;
/// The sprite layer and bucket the screen's sprites go into (the raw OAM
/// list `sub_8009FF8` fills: layer 1, bucket 0).
const SPRITE_LAYER: usize = 1;

/// The local player's custom screen while the battle is on the custom
/// screen.
pub fn local(b: &Battle) -> Option<(&Side, &Screen)> {
    if b.round.mode != mode::CUSTOM {
        return None;
    }
    let side = &b.custom.sides[b.setup.local_side as usize & 1];
    Some((side, side.screen.as_ref()?))
}

/// Where the window is: the HUD layer's horizontal scroll (BG3HOFS) and
/// the columns of the window map on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub scroll: u32,
    /// Window columns on the layer: from..15.
    pub from: usize,
    pub to: usize,
    /// Slide ticks the window has made in (0-10): the camera follows.
    pub slid: i32,
}

impl Placement {
    const IN: Placement = Placement { scroll: 0, from: 0, to: COLUMNS, slid: SLIDE_TICKS };
    const OUT: Placement = Placement { scroll: SLIDE, from: 0, to: 0, slid: 0 };
}

/// Columns the slide in has copied after `k` ticks (`sub_8026B04`: one or
/// two a tick, by bit 2 of the offset, from the right).
fn copied_in(k: u32) -> usize {
    (1..=k).map(|i| ((SLIDE - 12 * i) & 4) as usize / 4 + 1).sum::<usize>().min(COLUMNS)
}

/// Columns the slide out has cleared after `k` ticks (`sub_8026BF4`, from
/// the left).
fn cleared_out(k: u32) -> usize {
    (1..=k).map(|i| ((12 * i) & 4) as usize / 4 ^ 1).map(|n| n + 1).sum::<usize>().min(COLUMNS)
}

/// The window's placement for a screen in this phase.
pub fn placement(s: &Screen) -> Placement {
    match s.phase {
        Phase::Opening { tick: 0 } => Placement { scroll: 0, from: COLUMNS, to: COLUMNS, slid: 0 },
        Phase::Opening { tick } => {
            let k = tick as u32;
            Placement { scroll: SLIDE - 12 * k, from: COLUMNS - copied_in(k), to: COLUMNS, slid: tick as i32 }
        }
        Phase::Closing { tick } => {
            let k = tick as u32;
            Placement { scroll: 12 * k, from: cleared_out(k), to: COLUMNS, slid: SLIDE_TICKS - tick as i32 }
        }
        // SELECT: the tick after it takes the window off the layer and
        // scrolls the layer out; a key brings it back.
        Phase::Hidden { stage: HiddenStage::Waiting } => Placement { slid: SLIDE_TICKS, ..Placement::OUT },
        Phase::ProgramAdvance { .. } | Phase::Sending { .. } => Placement::OUT,
        _ => Placement::IN,
    }
}

/// How far the HUD's HP box and mugshot move right while the custom
/// screen is up (the HUD's `+0x12` and the HP box at column 15 under the
/// layer's scroll): 120 less the scroll, from the slide's first tick until
/// the result is sent.
pub fn hud_shift(b: &Battle) -> i32 {
    let Some((_, s)) = local(b) else { return 0 };
    match s.phase {
        Phase::Opening { tick: 0 } | Phase::ProgramAdvance { .. } | Phase::Sending { .. } => 0,
        _ => SLIDE as i32 - placement(s).scroll as i32,
    }
}

/// The form whose face the emotion window shows while `side`'s screen is
/// up (the Beast Out or Cross chosen there).
pub fn face(b: &Battle, side: usize) -> Option<nettai_content_api::FormHandle> {
    if b.round.mode != mode::CUSTOM {
        return None;
    }
    b.custom.sides[side & 1].screen.as_ref()?.look.face
}

/// The HUD layer's shake: in the screen's sub-screens that let it
/// (`sub_80269E2`: Beast Out, the Cross window, the scrap, the re-deal),
/// the camera's jitter moves the HUD layer too (`sub_8030158`), in whole
/// pixels.
pub fn hud_jitter(b: &Battle) -> (i32, i32) {
    let Some((_, s)) = local(b) else { return (0, 0) };
    let shakes = matches!(
        s.phase,
        Phase::BeastOutChosen { .. }
            | Phase::BeastOutChipChosen { .. }
            | Phase::CrossWindowOpening { .. }
            | Phase::CrossWindow { .. }
            | Phase::CrossWindowClosing { .. }
            | Phase::CrossChosen { .. }
            | Phase::Description { from_cross_window: true, .. }
            | Phase::Scrapping { .. }
            | Phase::Redealing { .. }
    );
    if !shakes {
        return (0, 0);
    }
    let (x, y) = b.consoles[b.setup.local_side as usize & 1].camera.jitter;
    (x >> 16, y >> 16)
}

/// The custom gauge stays off the local console's HUD from the screen's
/// opening until its own result is sent (HUD task 4: `sub_8026840` stops
/// it, `sub_8027D78` starts it), even once the other player's result has
/// restarted the battle's gauge.
pub fn gauge_held(b: &Battle) -> bool {
    local(b).is_some_and(|(_, s)| !matches!(s.phase, Phase::Sending { started: true }))
}

/// The camera's offset (16.16, added to its Y) while the window is in.
pub fn camera_y(b: &Battle) -> i32 {
    local(b).map_or(0, |(_, s)| -CAMERA_STEP * placement(s).slid)
}

/// The screen fade the local screen runs: Beast Out's darkens background
/// palettes 0-13 (the stage and the HUD layer) and sprite palettes 0-10
/// (the battle's objects), half way (`off_8006040`, modes 0x64 and 0x60).
pub fn fade(b: &Battle) -> Option<Fade> {
    let (_, s) = local(b)?;
    let f = s.look.fade;
    let shown = match f.mode {
        FadeMode::BeastOut | FadeMode::ProgramAdvance | FadeMode::DarkChip => true,
        // Back toward clear: once it gets there the original takes the
        // palettes' transform off.
        FadeMode::BeastOutBack | FadeMode::ProgramAdvanceBack | FadeMode::DarkChipBack => f.active,
        _ => false,
    };
    let n = (f.level >> 4).min(16) as u8;
    (shown && n > 0).then_some(Fade::Black(n))
}

/// The custom screen's tiles of the HUD layer, by tile number.
struct LayerTiles {
    pixels: Vec<u8>,
}

impl LayerTiles {
    fn new() -> LayerTiles {
        LayerTiles { pixels: vec![0; LAYER_TILES * Tiles::TILE] }
    }

    /// Copy `tiles` to tile number `at` on.
    fn put(&mut self, at: u16, tiles: &Tiles) {
        let from = at as usize * Tiles::TILE;
        let n = tiles.pixels.len().min(self.pixels.len().saturating_sub(from));
        self.pixels[from..from + n].copy_from_slice(&tiles.pixels[..n]);
    }

    /// Copy tiles `first..first + n` of `tiles` to tile number `at` on.
    fn put_part(&mut self, at: u16, tiles: &Tiles, first: usize, n: usize) {
        let src = &tiles.pixels[(first * Tiles::TILE).min(tiles.pixels.len())..((first + n) * Tiles::TILE).min(tiles.pixels.len())];
        self.put(at, &Tiles { pixels: src.to_vec() });
    }

    /// Fill `n` tiles from `at` with one colour index.
    fn fill(&mut self, at: u16, n: usize, index: u8) {
        let from = at as usize * Tiles::TILE;
        let to = (from + n * Tiles::TILE).min(self.pixels.len());
        self.pixels[from..to].fill(index);
    }

    fn tile(&self, t: u16) -> &[u8] {
        let from = (t as usize % LAYER_TILES) * Tiles::TILE;
        &self.pixels[from..from + Tiles::TILE]
    }
}

/// What the screen draws this frame, worked out once.
struct View<'a> {
    b: &'a Battle,
    side: u8,
    screen: &'a Screen,
    assets: &'a CustomScreen,
    hud: &'a Hud,
}

impl View<'_> {
    fn icon(&self, c: FolderChip, problems: &mut Problems) -> Option<&Tiles> {
        let def = self.b.content.defs.chip(c.id);
        let icon = self.hud.chip_icon(&def.key);
        if icon.is_none() {
            problems.note(format!("chip {:?} ({}) has no icon in the pack", def.key, def.record.name));
        }
        icon
    }

    /// The navi's number: its emblem and its emblem's palette (the
    /// cursor's too) are by it (`sub_802812C`), until the navi definitions
    /// name their own: compat has the numbers of the content's keys.
    fn navi_number(&self) -> usize {
        navi_number(self.b, self.side)
    }

    fn emblem_palette(&self) -> Palette {
        let a = self.assets;
        let i = a.emblem_palette_of.get(self.navi_number()).copied().unwrap_or(0) as usize;
        a.emblem_palettes.get(i).copied().unwrap_or([0; 16])
    }
}

/// A side's navi's number (see `View::navi_number`).
pub fn navi_number(b: &Battle, side: u8) -> usize {
    let key = &b.content.defs.navi(b.stats[side as usize & 1].navi).key;
    bn6_compat::Compat::bn6().navis.get(key).map_or(0, |n| n.navi as usize)
}

/// The window's map, the tiles and the palettes it draws with.
struct Window {
    map: [MapEntry; COLUMNS * ROWS],
    tiles: LayerTiles,
    palettes: [Palette; 16],
}

/// A slot's state as the original's byte holds it.
fn state_number(s: SlotState) -> usize {
    match s {
        SlotState::Selectable => 0,
        SlotState::Unavailable => 1,
        SlotState::Selected => 2,
    }
}

impl Window {
    fn build(v: &View, problems: &mut Problems) -> Window {
        let a = v.assets;
        let mut w = Window { map: [MapEntry::default(); COLUMNS * ROWS], tiles: LayerTiles::new(), palettes: [[0; 16]; 16] };
        // sub_8026840: the window's map, with the Cross tab or without.
        if let Some(m) = a.window_maps.get(v.screen.look.cross_tab as usize) {
            for (cell, e) in w.map.iter_mut().zip(m) {
                *cell = *e;
            }
        }
        // Its patches, numbered on from their first tile.
        let mut tile = a.window_patches.first_tile;
        for p in &a.window_patches.patches {
            for j in 0..p.height as u16 {
                for i in 0..p.width as u16 {
                    let n = if p.by_column { i * p.height as u16 + j } else { j * p.width as u16 + i };
                    let (x, y) = (p.x as usize + i as usize, p.y as usize + j as usize);
                    if x < COLUMNS && y < ROWS {
                        w.map[y * COLUMNS + x] = MapEntry { tile: tile + n, hflip: false, vflip: false, palette: p.palette };
                    }
                }
            }
            tile += p.width as u16 * p.height as u16;
        }
        w.tiles.put(WINDOW_TILE, &a.window_tiles);
        w.tiles.put(COLUMN_CELL_TILE, &a.column_cells);
        w.tiles.put(TURN_LIMIT_TILE, &a.turn_limit);
        w.tiles.put(NAME_BAR_TILE, &a.name_bar);
        w.palettes[11] = a.icon_palette;
        w.palettes[12] = a.grey_palette;
        w.palettes[14] = a.other_palette;
        w.palettes[13] = v.hud.hp_palettes[0];
        w.chip_window(v, problems);
        w.slots(v, problems);
        w.column(v, problems);
        if v.screen.look.turn_limit {
            // sub_8029D34: "FINAL TURN", 7x2 at column 15, row 4 (past the
            // window's columns: drawn on the layer apart).
        }
        w
    }

    /// `sub_8028476`: the chip window shows what it was last drawn for: a
    /// chip's name, picture, code, element and damage (`sub_80284E2`), or
    /// a button's picture (`sub_80286D4`, `sub_802871C`, `sub_80287A4`,
    /// `sub_802877C`) with the window's colours where the chip's details
    /// go.
    fn chip_window(&mut self, v: &View, problems: &mut Problems) {
        let a = v.assets;
        let cw = v.screen.look.chip_window;
        // Palette 11's colours from 10 are the last chip's element's.
        if let Some(c) = cw.last_chip {
            let family = v.b.content.chip(c.id).family as usize;
            if let Some(colours) = a.element_colours.get(family) {
                self.palettes[11][10..].copy_from_slice(colours);
            }
        }
        let slot = cw.slot.min(SPECIAL_SLOT);
        let blank_details = |w: &mut Window, picture: &Picture| {
            // sub_80287D2: the name's cells and the window's colours.
            w.tiles.fill(NAME_TILE, 2 * NAME_CELLS, BLANK_8);
            w.palettes[9] = a.frame_palettes.first().copied().unwrap_or([0; 16]);
            w.tiles.put(ART_TILE, &picture.tiles);
            w.palettes[10] = picture.palette;
            // sub_802869E
            w.tiles.fill(CODE_TILE, 2, BLANK_8);
            w.tiles.fill(ELEMENT_TILE, 4, BLANK_7);
            w.tiles.fill(DIGIT_TILE, 6, BLANK_8);
        };
        match v.screen.slots[slot as usize].kind {
            SlotKind::Chip { .. } | SlotKind::NaviChip(_) => {
                let Some(c) = cw.last_chip else { return };
                self.chip_details(v, c, problems);
            }
            SlotKind::Ok => {
                let p = if cw.picks == 0 { &a.pictures.ok } else { &a.pictures.ok_picked };
                blank_details(self, p);
            }
            SlotKind::BeastOut => {
                let p = Picture { palette: a.pictures.beast_out_palettes.first().copied().unwrap_or([0; 16]), ..a.pictures.beast_out.clone() };
                blank_details(self, &p);
            }
            SlotKind::Redeal { .. } => blank_details(self, &a.pictures.redeal),
            SlotKind::Scrap { .. } => blank_details(self, &a.pictures.scrap),
            SlotKind::Empty | SlotKind::Hidden => {}
        }
    }

    /// `sub_80284E2`: a chip's name (8 cells of the 8x16 font in the
    /// window's colours), its picture and palette, the window's colours by
    /// the chip's class, its code, its element's icon (and colours), and
    /// its damage if it shows (Muramasa's as "???"), right-aligned in three
    /// cells.
    fn chip_details(&mut self, v: &View, c: FolderChip, problems: &mut Problems) {
        let a = v.assets;
        let def = v.b.content.defs.chip(c.id);
        let data = v.b.content.chip(c.id);
        let (glyphs, missing) = fonts::cell_glyphs(v.hud, &data.name);
        if !missing.is_empty() {
            problems.note(format!("chip {:?} is named {:?}, but the pack's font has no glyph for {missing:?}", def.key, data.name));
        }
        self.tiles.put(NAME_TILE, &fonts::cell_text(v.hud, &glyphs, NAME_CELLS, NAME_SHIFT));
        match a.chip_art(&def.key) {
            Some(p) => {
                self.tiles.put(ART_TILE, &p.tiles);
                self.palettes[10] = p.palette;
            }
            None => problems.note(format!("chip {:?} ({}) has no picture in the pack", def.key, data.name)),
        }
        let class = match data.class {
            ChipClass::Standard => 0,
            ChipClass::Mega => 1,
            ChipClass::Giga => 2,
            _ => 0,
        };
        self.palettes[9] = a.frame_palettes.get(class).copied().unwrap_or([0; 16]);
        let code = c.code.0.min(NO_CODE) as usize;
        self.tiles.put_part(CODE_TILE, &a.codes, 2 * code, 2);
        let family = data.family as usize;
        if family < a.element_colours.len() {
            self.tiles.put_part(ELEMENT_TILE, &a.elements, 4 * family, 4);
        }
        let shows = data.flags.0 & (ChipFlags::HAS_DAMAGE | ChipFlags::DAMAGE_SHOWN_VARIABLE) != 0;
        let digits: Vec<usize> = if !shows {
            Vec::new()
        } else if matches!(data.formula, Some(DamageFormula::HpLost)) {
            // (The original knows Muramasa by its number.)
            vec![DIGIT_UNKNOWN; 3]
        } else {
            let damage = nettai_battle::hand::chip_damage(v.b, Some(c.id), v.side);
            damage.min(999).to_string().bytes().map(|d| (d - b'0') as usize).collect()
        };
        let blanks = 3 - digits.len();
        self.tiles.fill(DIGIT_TILE, 2 * blanks, BLANK_8);
        for (i, &d) in digits.iter().enumerate() {
            self.tiles.put_part(DIGIT_TILE + 2 * (blanks + i) as u16, &a.digits, 2 * d, 2);
        }
    }

    /// `sub_8028250`: each slot's tiles, from tile 0xE1 on, as its kind
    /// takes them; and `sub_80283C8`: the chip slots' icon palettes by
    /// their state.
    fn slots(&mut self, v: &View, problems: &mut Problems) {
        let a = v.assets;
        let mut at = SLOT_TILE;
        for (s, slot) in v.screen.slots.iter().enumerate() {
            let state = state_number(slot.state);
            match slot.kind {
                SlotKind::Chip { .. } | SlotKind::NaviChip(_) => {
                    let Some(c) = v.screen.look.slot_chips[s] else { continue };
                    if slot.state == SlotState::Selected {
                        self.tiles.put(at, &a.empty_icon);
                    } else if let Some(icon) = v.icon(c, problems) {
                        self.tiles.put(at, icon);
                    }
                    // sub_8028214: the code's glyph; the special codes
                    // show blank.
                    match c.code.0 {
                        0x1B | 0x1C => self.tiles.fill(at + 4, 2, BLANK_1),
                        code => self.tiles.put_part(at + 4, &a.slot_codes, 2 * code.min(EMPTY_SLOT_CODE) as usize, 2),
                    }
                    at += 6;
                }
                SlotKind::Ok | SlotKind::Redeal { right_half: true } | SlotKind::Scrap { right_half: true } => {}
                SlotKind::BeastOut => self.tiles.put_part(at, &a.beast_buttons, 8 * (state != 0) as usize, 8),
                SlotKind::Redeal { right_half: false } => {
                    self.tiles.put_part(at, &a.redeal_buttons, 12 * state, 12);
                    at += 12;
                }
                SlotKind::Scrap { right_half: false } => {
                    self.tiles.put_part(at, &a.scrap_buttons, 12 * state, 12);
                    at += 12;
                }
                SlotKind::Empty => {
                    self.tiles.put(at, &a.empty_icon);
                    self.tiles.put_part(at + 4, &a.slot_codes, 2 * EMPTY_SLOT_CODE as usize, 2);
                    at += 6;
                }
                SlotKind::Hidden if s as u8 == SPECIAL_SLOT => self.tiles.put_part(at, &a.beast_buttons, 24, 8),
                SlotKind::Hidden => {
                    self.tiles.fill(at, 6, BLANK_1);
                    at += 6;
                }
            }
        }
        // The icons' palettes (the 2x2 icon cells of slots 0-9).
        for s in 0..10usize {
            let slot = v.screen.slots[s];
            let palette = match slot.kind {
                SlotKind::Empty => 11,
                SlotKind::Chip { .. } | SlotKind::NaviChip(_) if slot.state == SlotState::Unavailable => 12,
                SlotKind::Chip { .. } | SlotKind::NaviChip(_) => 11,
                SlotKind::Ok | SlotKind::BeastOut => continue,
                _ => 9,
            };
            let (x, y) = (1 + 2 * (s % 5), 13 + 3 * (s / 5));
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                self.map[(y + dy) * COLUMNS + x + dx].palette = palette;
            }
        }
    }

    /// The picked chips' column: the icons the screen copied in
    /// (`sub_80281D4`), and the cells' frames (`sub_8029CD4`).
    fn column(&mut self, v: &View, problems: &mut Problems) {
        let a = v.assets;
        let picks = v.screen.selection();
        for i in 0..5usize {
            let at = COLUMN_ICON_TILE + 4 * i as u16;
            let icon = v.screen.look.column[i].and_then(|c| v.icon(c, problems));
            self.tiles.put(at, icon.unwrap_or(&a.empty_icon));
            let filled = (i < picks.len()) as u16;
            let tile = COLUMN_CELL_TILE + 2 * filled;
            for (x, hflip) in [(11, false), (14, true)] {
                self.map[(3 + 2 * i) * COLUMNS + x] = MapEntry { tile, hflip, vflip: false, palette: 9 };
                self.map[(4 + 2 * i) * COLUMNS + x] = MapEntry { tile: tile + 1, hflip, vflip: false, palette: 9 };
            }
        }
    }

    /// Draw window columns `from..to` on the layer, scrolled.
    fn draw(&self, layer: &mut Layer, place: Placement) {
        for y in 0..ROWS {
            for x in place.from..place.to {
                let e = self.map[y * COLUMNS + x];
                let Some(px) = screen_x(x as i32, place.scroll) else { continue };
                layer.draw_tile(self.tiles.tile(e.tile), &self.palettes[e.palette as usize & 15], px, 8 * y as i32, e.hflip, e.vflip);
            }
        }
    }

    /// Draw a tile of the layer's character block at a cell of the layer.
    fn cell(&self, layer: &mut Layer, e: MapEntry, col: i32, row: i32, scroll: u32) {
        if let Some(px) = screen_x(col, scroll) {
            layer.draw_tile(self.tiles.tile(e.tile), &self.palettes[e.palette as usize & 15], px, 8 * row, e.hflip, e.vflip);
        }
    }
}

/// Where a column of the 256-pixel layer shows, scrolled: what scrolls off
/// the left comes back on the right, past the screen's edge.
fn screen_x(col: i32, scroll: u32) -> Option<i32> {
    let px = (8 * col - scroll as i32).rem_euclid(256);
    match px {
        248.. => Some(px - 256),
        240.. => None,
        _ => Some(px),
    }
}

/// The enemy names (`sub_801E574` renders them, HUD task 13 shows them):
/// on the round's first screen, once the window is in, until it slides out
/// or hides. Each name's 9 cells of the 8x16 font on BG0 end at column 30,
/// on a bar on the HUD layer (`sub_801E4F4`: its slanted end, then a cell
/// a glyph).
fn names_shown(b: &Battle, s: &Screen) -> bool {
    b.round.turn == 1
        && !matches!(
            s.phase,
            Phase::Opening { .. }
                | Phase::Closing { tick: 1.. }
                | Phase::Hidden { stage: HiddenStage::Waiting }
                | Phase::ProgramAdvance { .. }
                | Phase::Sending { .. }
        )
}

fn draw_names(v: &View, w: &Window, hud_layer: &mut Layer, names_layer: &mut Layer, problems: &mut Problems) {
    let other = v.side ^ 1;
    let name = &v.b.content.navi(v.b.stats[other as usize].navi).name;
    let (glyphs, missing) = fonts::cell_glyphs(v.hud, name);
    if !missing.is_empty() {
        problems.note(format!("the navi named {name:?}: the pack's font has no glyph for {missing:?}"));
    }
    let len = glyphs.len().min(ENEMY_NAME_CELLS);
    let text = fonts::cell_text(v.hud, &glyphs, ENEMY_NAME_CELLS, 0);
    let pal = &w.palettes[13];
    let col = 0x1E - len as i32;
    for k in 0..ENEMY_NAME_CELLS {
        for half in 0..2 {
            if let Some(t) = text.get(2 * k + half) {
                names_layer.draw_tile(t, pal, 8 * (col + k as i32), 8 * half as i32, false, false);
            }
        }
    }
    // The bar: the slanted end, then one cell a glyph (eight at most; a
    // ninth glyph has bar cells from column 21 on, no end).
    let bar = |tile: u16| MapEntry { tile, hflip: false, vflip: false, palette: 13 };
    let cells = len.min(8);
    let mut col = 0x1D - cells as i32;
    if len < ENEMY_NAME_CELLS {
        w.cell(hud_layer, bar(NAME_BAR_TILE), col, 0, 0);
        w.cell(hud_layer, bar(NAME_BAR_TILE + 1), col, 1, 0);
        col += 1;
    }
    let n = if len < ENEMY_NAME_CELLS { len } else { ENEMY_NAME_CELLS };
    for k in 0..n as i32 {
        w.cell(hud_layer, bar(NAME_BAR_TILE + 2), col + k, 0, 0);
        w.cell(hud_layer, bar(NAME_BAR_TILE + 3), col + k, 1, 0);
    }
}

/// The cursor's corners (`sub_8028820`): where its slot's frame is
/// (`jt_802886C`'s routines, less 3) and the four 8x8 corners in each of
/// its two frames (`byte_80288B0` and the others: y, x, flips).
struct CursorShape {
    corners: [[(i32, i32, bool, bool); 4]; 2],
}

const CHIP_CURSOR: CursorShape = CursorShape {
    corners: [
        [(0, 0, false, false), (0, 0xE, true, false), (0xE, 0xE, true, true), (0xE, 0, false, true)],
        [(1, 1, false, false), (1, 0xC, true, false), (0xC, 0xC, true, true), (0xC, 1, false, true)],
    ],
};
const OK_CURSOR: CursorShape = CursorShape {
    corners: [
        [(2, 1, false, false), (2, 0x16, true, false), (0x14, 0x16, true, true), (0x14, 1, false, true)],
        [(4, 3, false, false), (4, 0x14, true, false), (0x12, 0x14, true, true), (0x12, 3, false, true)],
    ],
};
const BEAST_OUT_CURSOR: CursorShape = CursorShape {
    corners: [
        [(2, 1, false, false), (2, 0x16, true, false), (0xE, 0x16, true, true), (0xE, 1, false, true)],
        [(3, 2, false, false), (3, 0x15, true, false), (0xD, 0x15, true, true), (0xD, 2, false, true)],
    ],
};
const BUTTON_CURSOR: CursorShape = CursorShape {
    corners: [
        [(2, 2, false, false), (2, 0x1C, true, false), (0x14, 0x1C, true, true), (0x14, 2, false, true)],
        [(4, 4, false, false), (4, 0x1A, true, false), (0x12, 0x1A, true, true), (0x12, 4, false, true)],
    ],
};

fn cursor_parts<'a>(v: &View, a: &'a CustomScreen, frame: u8) -> Vec<SpritePart<'a>> {
    let s = v.screen;
    let slot = s.cursor;
    let (x, y, shape) = match s.slots[slot as usize].kind {
        SlotKind::Chip { .. } | SlotKind::NaviChip(_) | SlotKind::Empty | SlotKind::Hidden => {
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            (16 * col + 8, 0x68 + 0x18 * row, &CHIP_CURSOR)
        }
        SlotKind::Ok => (0x58 + 3, 0x70 - 2, &OK_CURSOR),
        SlotKind::BeastOut => (0x58 + 3, 0x88 - 1, &BEAST_OUT_CURSOR),
        SlotKind::Redeal { .. } | SlotKind::Scrap { .. } => (0x38, 0x80, &BUTTON_CURSOR),
    };
    let palette = v.emblem_palette();
    // Queued last corner first (`sub_8028820`).
    shape.corners[frame as usize & 1]
        .iter()
        .rev()
        .map(|&(dy, dx, hflip, vflip)| SpritePart {
            x: ((x - 3 + dx) & 0x1FF) as u16,
            y: (y - 3 + dy) as u8,
            width: 8,
            height: 8,
            tiles: &a.cursor,
            first_tile: frame as usize & 1,
            hflip,
            vflip,
            palette,
            priority: 1,
            alpha: None,
            mosaic: None,
            vscale: None,
            affine: None,
        })
        .collect()
}

/// The emblem (`sub_8029C08`): a 32x32 affine sprite with the navi's
/// emblem in its middle, over the picked column, at the window's offset;
/// nudged while it spins; turned and scaled by its matrix.
fn emblem_part<'a>(v: &View, tiles: &'a Tiles, x_slide: u32, spin: u8) -> SpritePart<'a> {
    let mut x = (0x57 - x_slide as i32) & 0x1FF;
    let mut y = 0xFCi32;
    if (2..=6).contains(&spin) {
        y -= 1;
        if (3..=4).contains(&spin) {
            y -= 1;
        }
    }
    if (5..=0xB).contains(&spin) {
        x += 1;
    } else if (2..=3).contains(&spin) {
        x -= 1;
    }
    let (angle, scale) = v.screen.look.emblem_matrix;
    let sine = &v.b.content.rules.sine;
    let at = |i: usize| sine.get(i % 256).copied().unwrap_or(0) as i32;
    let (sin, cos) = (at(angle as usize), at(angle as usize + 64));
    // sub_802FE7A: the angle's sine and cosine times a quarter of the
    // scale, over 16.
    let q = scale as i32 >> 2;
    let m = |v: i32| (v * q) >> 4;
    SpritePart {
        x: (x & 0x1FF) as u16,
        y: (y & 0xFF) as u8,
        width: 32,
        height: 32,
        tiles,
        first_tile: 0,
        hflip: false,
        vflip: false,
        palette: v.emblem_palette(),
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: Some(Affine { pa: m(cos), pb: m(sin), pc: m(-sin), pd: m(cos), double: false }),
    }
}

/// The emblem's 32x32 sprite: blank but for the navi's emblem in its
/// middle 2x2 tiles (`off_802A744`, `sub_802812C`).
pub fn emblem_tiles(a: &CustomScreen, navi_number: usize) -> Tiles {
    let e = a.emblem_of.get(navi_number).copied().unwrap_or(0) as usize;
    let mut t = Tiles { pixels: vec![0; 16 * Tiles::TILE] };
    for (k, place) in [5usize, 6, 9, 10].into_iter().enumerate() {
        if let Some(src) = a.emblems.get(4 * e + k) {
            t.pixels[place * Tiles::TILE..(place + 1) * Tiles::TILE].copy_from_slice(src);
        }
    }
    t
}

/// The Regular chip's frame (`sub_802899C`): a 32x32 sprite around the
/// first slot.
fn regular_part<'a>(v: &View, a: &'a CustomScreen) -> SpritePart<'a> {
    SpritePart {
        x: 2,
        y: 100,
        width: 32,
        height: 32,
        tiles: &a.regular,
        first_tile: 16 * (v.screen.look.regular_frame as usize & 1),
        hflip: false,
        vflip: false,
        palette: v.emblem_palette(),
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    }
}

/// Draw the local player's custom screen: the window on the HUD layer, the
/// enemy names on `names_layer` (BG0), the sprites into `list`.
/// `emblem` holds the emblem sprite's tiles (`emblem_tiles`).
pub fn draw<'a>(
    b: &'a Battle,
    assets: &'a Bundle,
    emblem: &'a Tiles,
    hud_layer: &mut Layer,
    names_layer: &mut Layer,
    list: &mut SpriteList<'a>,
    problems: &mut Problems,
) {
    let Some((_, screen)) = local(b) else { return };
    let a = &assets.custom;
    if a.is_empty() {
        problems.note("the pack has no custom screen graphics (extract it again)".into());
        return;
    }
    let v = View { b, side: b.setup.local_side & 1, screen, assets: a, hud: &assets.hud };
    let place = placement(screen);
    let w = Window::build(&v, problems);
    w.draw(hud_layer, place);
    if screen.look.turn_limit && place.to == COLUMNS {
        // sub_8029D34: 7x2 at column 15, row 4.
        for i in 0..14u16 {
            let e = MapEntry { tile: TURN_LIMIT_TILE + i, hflip: false, vflip: false, palette: 9 };
            w.cell(hud_layer, e, 15 + (i % 7) as i32, 4 + (i / 7) as i32, place.scroll);
        }
    }
    if names_shown(b, screen) {
        draw_names(&v, &w, hud_layer, names_layer, problems);
    }
    // The sprites, as their routines queue them (each in front of the
    // one before).
    let drawn = screen.look.drawn;
    let mut queue: Vec<SpritePart<'a>> = Vec::new();
    if let Some(frame) = drawn.cursor {
        queue.extend(cursor_parts(&v, a, frame));
    }
    if let Some((x, spin)) = drawn.emblem {
        queue.push(emblem_part(&v, emblem, x, spin));
    }
    if drawn.regular {
        queue.push(regular_part(&v, a));
    }
    for part in queue {
        list.insert_at(SPRITE_LAYER, 0, vec![part]);
    }
    let _ = OK_SLOT;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_slides_in_and_out_a_column_or_two_a_tick() {
        let ins: Vec<usize> = (1..=10).map(copied_in).collect();
        assert_eq!(ins, [2, 3, 5, 6, 8, 9, 11, 12, 14, 15]);
        let outs: Vec<usize> = (1..=10).map(cleared_out).collect();
        assert_eq!(outs, [1, 3, 4, 6, 7, 9, 10, 12, 13, 15]);
    }
}
