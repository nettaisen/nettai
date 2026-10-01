//! The battle HUD: the player's HP box, the custom gauge and the chip name
//! on the HUD layer (priority 1); the mugshot, the opponent's HP, the chip
//! icons over the navis and the banners as sprites.
//!
//! The rolling numbers are presentation state the engine doesn't keep, so
//! `HudState` follows them tick by tick (`sub_801C840`, `sub_801C168`).

use crate::audit::Problems;
use crate::compose::{Layer, SpritePart};
use crate::objects::{SpriteList, View, project};
use bn6_assets::{Bundle, Hud, MapEntry, Palette, Tiles};
use bn6_battle::Battle;
use bn6_battle::actor::status;
use bn6_battle::transform::{SequencerState, TransformPhase};
use bn6_battle::battle::{fight, mode, top};
use bn6_battle::content::ChipFlags;
use bn6_content_api::ChipHandle;
use bn6_battle::object::{ObjectRef, flags};
use bn6_battle::perspective::{ShownTelop, TelopName};

/// The player navi's action while it stands waiting for input.
const FULL: u16 = bn6_battle::hud::CustomGauge::FULL;

/// HUD presentation state that rolls from frame to frame.
#[derive(Clone, Debug, Default)]
pub struct HudState {
    hp: Option<RollingHp>,
    enemies: Vec<EnemyHp>,
    /// Counts draws (drives the full gauge's animation; wraps at 0x70).
    frame: u8,
    /// The chip name window is on: the idle controller's entry shows it
    /// (`sub_801DA48`), using a chip hides it.
    chip_name: bool,
    /// The mugshot's mood and its change blink.
    mood: Option<Mood>,
    /// As of the previous tick: whether the round was decided (the HUD
    /// thins out a frame after the decision) and the gauge was running.
    was_over: bool,
    gauge_was_on: bool,
    is_over: bool,
    gauge_is_on: bool,
}

/// The mugshot's mood (0 normal, 1 Beast Out spent, 2 Full Synchro,
/// 3 angry, 5 worn out) and, for 12 frames after it changes, a blink back
/// to the previous one (`sub_801CB38`).
#[derive(Clone, Copy, Debug)]
struct Mood {
    now: u8,
    before: u8,
    blink: u8,
    /// Show `before` this frame.
    flash: bool,
}

impl Mood {
    fn shown(self) -> u8 {
        if self.flash { self.before } else { self.now }
    }
}

/// `sub_80139C8`'s emotion as the mugshot reads it.
fn mood_index(b: &Battle, r: ObjectRef) -> u8 {
    let o = b.objects.get(r);
    let mood = b.stats[o.alliance as usize].mood;
    let Some(a) = o.actor.map(|a| b.actors.get(a)) else { return 0 };
    if a.beast_over_exhausted || mood == 0 {
        5
    } else if a.anger != 0 {
        3
    } else if a.beast_out_spent {
        1
    } else if mood == 0xFF {
        2
    } else {
        0
    }
}

#[derive(Clone, Copy, Debug)]
struct RollingHp {
    shown: u16,
    /// 0 normal, 1 healing, 2 hurt or low.
    colour: u8,
    hold: u8,
}

#[derive(Clone, Copy, Debug)]
struct EnemyHp {
    object: ObjectRef,
    shown: u16,
    /// 0 normal, 1 dropping, 2 rising.
    colour: u8,
    timer: u8,
}

/// A step toward `target` of an eighth of the distance plus `extra`.
fn roll(shown: u16, target: u16, extra: u16) -> u16 {
    if target > shown {
        (shown + ((target - shown) >> 3) + extra).min(target)
    } else {
        shown.saturating_sub(((shown - target) >> 3) + extra).max(target)
    }
}

impl HudState {
    /// Follow one tick of the battle.
    pub fn tick(&mut self, b: &Battle) {
        self.frame = (self.frame + 1) % 0x70;
        (self.was_over, self.gauge_was_on) = (self.is_over, self.gauge_is_on);
        if let Some(r) = b.player(b.setup.local_side) {
            let now = mood_index(b, r);
            let m = self.mood.get_or_insert(Mood { now, before: now, blink: 0, flash: false });
            if now != m.now {
                *m = Mood { now, before: m.now, blink: 12, flash: false };
            }
            m.flash = false;
            if m.now < 5 && m.now != 3 && m.blink > 0 {
                m.flash = m.blink & 2 != 0;
                m.blink -= 1;
            }
        }
        (self.is_over, self.gauge_is_on) = (decided(b), b.gauge.enabled);
        if let Some(r) = b.player(b.setup.local_side) {
            let o = b.objects.get(r);
            if bn6_battle::kinds::player::navi_action(b, r) == bn6_battle::kinds::player::NaviAction::Idle
                && (o.phase != 0 || o.phase_init != 0)
            {
                self.chip_name = true;
            }
            let using_chip = o.actor.is_some_and(|a| b.actors.get(a).status & status::CHIP_IN_PROGRESS != 0);
            if using_chip || b.round.mode != mode::FIGHTING || decided(b) {
                self.chip_name = false;
            }
        }
        let local = b.setup.local_side;
        // The local navi's HP box.
        if let Some(r) = b.player(local) {
            let o = b.objects.get(r);
            let (h, m) = (o.hp, o.max_hp);
            let s = self.hp.get_or_insert(RollingHp { shown: h, colour: 0, hold: 0 });
            if h > s.shown {
                s.colour = 1;
                s.shown = roll(s.shown, h, 4);
                s.hold = 15;
            } else if h < s.shown {
                s.colour = 2;
                s.shown = roll(s.shown, h, 4);
                s.hold = 15;
            } else {
                if s.hold > 0 {
                    s.hold -= 1;
                }
                if s.hold == 0 {
                    s.colour = if h <= m / 4 { 2 } else { 0 };
                }
            }
        }
        // Opponents' HP: shown once their entry is over, until deleted.
        for side in 0..2u8 {
            if side == local {
                continue;
            }
            let Some(r) = b.player(side) else { continue };
            let o = b.objects.get(r);
            if bn6_battle::kinds::player::navi_action(b, r) != bn6_battle::kinds::player::NaviAction::Entry
                && o.hp > 0
                && !self.enemies.iter().any(|e| e.object == r)
            {
                self.enemies.push(EnemyHp { object: r, shown: o.hp, colour: 0, timer: 0 });
            }
        }
        self.enemies.retain_mut(|e| {
            let o = b.objects.get(e.object);
            if o.flags & flags::ACTIVE == 0 || o.hp == 0 {
                return false;
            }
            let poisoned = o.collision.is_some_and(|c| b.collision.get(c).poison_timer != 0);
            if e.shown < o.hp {
                e.colour = 2;
                e.timer = 10;
                e.shown = roll(e.shown, o.hp, 2);
            } else if e.shown > o.hp {
                e.colour = 1;
                e.timer = 1;
                e.shown = roll(e.shown, o.hp, 2);
            } else if poisoned {
                e.colour = 1;
                e.timer = 2;
            } else if e.timer > 0 {
                e.timer -= 1;
                if e.timer == 0 {
                    e.colour = 0;
                }
            }
            true
        });
    }
}

/// Decimal digits right-aligned in four places (10 = blank).
fn digits4(v: u16) -> [usize; 4] {
    let s = v.min(9999).to_string();
    let mut out = [10usize; 4];
    for (i, c) in s.bytes().enumerate() {
        out[4 - s.len() + i] = (c - b'0') as usize;
    }
    out
}

/// Whether the local player's custom screen covers the left of the screen.
fn custom_open(b: &Battle) -> bool {
    b.round.mode == mode::CUSTOM && b.custom.sides[b.setup.local_side as usize].in_custom
}

/// Whether the round has been decided (the HUD thins out).
fn decided(b: &Battle) -> bool {
    b.round.top == top::END
        || b.round.mode == mode::FADE_OUT
        || (b.round.mode == mode::FIGHTING
            && matches!(b.fight.state, fight::WIN | fight::LOSE | fight::DRAW | fight::JUDGE))
}

/// While the navis change form the HUD steps aside: the mugshot from the
/// start of the fade out, the HP box and gauge once the screen is dark.
fn transform_hides(b: &Battle) -> (bool, bool) {
    match b.transform_seq.state {
        SequencerState::Transform { phase: TransformPhase::FadeOut, .. } => (true, false),
        SequencerState::Transform { .. } => (true, true),
        _ => (false, false),
    }
}

/// Draw the HUD layer and queue the HUD sprites. What the HUD needs and
/// the pack or the content doesn't have (a chip's icon, a character of its
/// name, a banner's glyphs) goes to `problems`.
pub fn draw<'a>(
    b: &Battle,
    assets: &'a Bundle,
    state: &HudState,
    layer: &mut Layer,
    list: &mut SpriteList<'a>,
    problems: &mut Problems,
) {
    let hud = &assets.hud;
    if hud.tiles.is_empty() {
        return;
    }
    let view = crate::render::Renderer::view(b);
    let local = b.setup.local_side;
    let player = b.player(local);
    let open = custom_open(b);
    let colour = state.hp.map(|h| h.colour).unwrap_or(0) as usize;

    let (hide_mugshot, hide_boxes) = transform_hides(b);
    // HP box, top left (x 120 while the custom screen is open).
    if let Some(r) = player.filter(|_| !hide_boxes) {
        let shown = state.hp.map(|h| h.shown).unwrap_or(b.objects.get(r).hp);
        let x0 = if open { 15 } else { 0 };
        let pal = &hud.hp_palettes[colour.min(2)];
        for (i, &e) in hud.hp_box.iter().enumerate() {
            put(layer, hud, pal, e, x0 + (i as i32 % 6), i as i32 / 6);
        }
        for (k, d) in digits4(shown).into_iter().enumerate() {
            let top = MapEntry { tile: 0x1A0 + 2 * d as u16, hflip: false, vflip: false, palette: 13 };
            put(layer, hud, pal, top, x0 + 1 + k as i32, 0);
            put(layer, hud, pal, MapEntry { tile: top.tile + 1, ..top }, x0 + 1 + k as i32, 1);
        }
    }

    // Custom gauge, top centre.
    if (b.gauge.enabled || state.gauge_was_on) && !state.was_over && !open && !hide_boxes {
        let pal = &hud.gauge_palette;
        for (i, &e) in hud.gauge_frame.iter().enumerate() {
            put(layer, hud, pal, e, 6 + (i as i32 % 18), i as i32 / 18);
        }
        let g = b.gauge.value;
        let cell = |tile: u16| MapEntry { tile, hflip: false, vflip: false, palette: 9 };
        for i in 0..16u16 {
            let tile = if g >= FULL {
                0x232 + ((state.frame as u16 / 7) & 3)
            } else if i < g >> 10 {
                0x22A
            } else if i == g >> 10 {
                0x222 + ((g >> 7) & 7)
            } else {
                0x222
            };
            put(layer, hud, pal, cell(tile), 7 + i as i32, 1);
        }
        if g >= FULL {
            let first = if state.frame & 8 == 0 { 0x236 } else { 0x23A };
            for i in 0..4 {
                put(layer, hud, pal, cell(first + i), 13 + i as i32, 1);
            }
        }
    }

    // "Cstmzing...": once the local player's result is sent, while waiting
    // for the opponent's; it blinks every 32 frames.
    let sent = b.custom.sides[local as usize].sent.as_ref().filter(|_| b.round.mode == mode::CUSTOM);
    if let Some(n) = sent.map(|s| b.round.ticks.saturating_sub(s.sent_at + 1)) {
        if !b.custom.committed && (n / 32) % 2 == 0 {
            for i in 0..16usize {
                if let Some(t) = hud.waiting.get(i) {
                    layer.draw_tile(t, &hud.waiting_palette, (22 + (i % 8) as i32) * 8, (4 + (i / 8) as i32) * 8, false, false);
                }
            }
        }
    }

    // The next chip's name (and damage) at the bottom left, while the
    // navi stands holding chips.
    if let Some(r) = player {
        let o = b.objects.get(r);
        let hand = &b.hands[local as usize];
        if state.chip_name
            && o.chips_held != 0
            && let Some(chip) = hand.ids.get(hand.cursor as usize).copied().flatten()
        {
            draw_chip_name(b, layer, hud, &hud.hp_palettes[colour.min(2)], hand, chip, problems);
        }
    }

    // Sprites: the banner in front, then the mugshot, the opponents' HP
    // and the chip icons (all in front of the field's sprites).
    let mut group = Vec::new();
    if let Some(id) = b.banner_for(local) {
        match b.telop_for(local) {
            Some(telop) => telop_parts(b, hud, id.0, telop, &mut group, problems),
            None => banner_parts(b, hud, id.0, &mut group, problems),
        }
    }
    if let Some(r) = player.filter(|_| !state.was_over && !hide_mugshot) {
        mugshot_parts(b, hud, state, r, if open { 120 } else { 0 }, &mut group);
    }
    for e in &state.enemies {
        let o = b.objects.get(e.object);
        if o.flags & flags::VISIBLE == 0 {
            continue;
        }
        let p = project((o.pos.x, o.pos.y, o.pos.z), &view);
        let n = e.shown.to_string().len() as i32;
        let digits = &hud.enemy_digits[e.colour.min(2) as usize];
        for (k, d) in digits4(e.shown).into_iter().enumerate() {
            if d == 10 {
                continue;
            }
            group.push(glyph(digits, d, hud.enemy_palette, p.x + 4 * n - 32 + 8 * k as i32, p.y, 2, None));
        }
    }
    if !state.was_over {
        for side in 0..2u8 {
            if let Some(r) = b.player(side) {
                icon_parts(b, hud, r, side == local, &view, &mut group, problems);
            }
        }
    }
    list.insert_front(group);
}

fn put(layer: &mut Layer, hud: &Hud, pal: &Palette, e: MapEntry, tx: i32, ty: i32) {
    let tile = if e.tile >= hud.gauge_first_tile {
        hud.gauge_tiles.get((e.tile - hud.gauge_first_tile) as usize)
    } else {
        e.tile.checked_sub(hud.first_tile).and_then(|i| hud.tiles.get(i as usize))
    };
    if let Some(t) = tile {
        layer.draw_tile(t, pal, tx * 8, ty * 8, e.hflip, e.vflip);
    }
}

/// A chip's name in the font's glyphs: the content's name for it, written
/// in the characters the pack's font has (`sub_8027D10`'s text for the
/// chip, at most eight glyphs).
fn name_glyphs(b: &Battle, hud: &Hud, chip: ChipHandle, problems: &mut Problems) -> Vec<u16> {
    let name = &b.content.chip(chip).name;
    let (mut glyphs, missing) = hud.glyphs(name);
    if !missing.is_empty() {
        let key = &b.content.defs.chip(chip).key;
        problems.note(format!("chip {key:?} is named {name:?}, but the pack's font has no glyph for {missing:?}"));
    }
    glyphs.truncate(8);
    glyphs
}

/// Whether a chip's damage shows after its name.
fn shows_damage(b: &Battle, chip: ChipHandle) -> bool {
    b.content.chip(chip).flags.0 & ChipFlags::HAS_DAMAGE != 0
}

fn draw_chip_name(
    b: &Battle,
    layer: &mut Layer,
    hud: &Hud,
    pal: &Palette,
    hand: &bn6_battle::hand::ChipHand,
    chip: ChipHandle,
    problems: &mut Problems,
) {
    let mut col = 0;
    for c in name_glyphs(b, hud, chip, problems) {
        for half in 0..2 {
            if let Some(t) = hud.font.get(2 * c as usize + half) {
                layer.draw_tile(t, pal, col * 8, (18 + half as i32) * 8, false, false);
            }
        }
        col += 1;
    }
    if !shows_damage(b, chip) {
        return;
    }
    let i = hand.cursor as usize;
    let number = |layer: &mut Layer, v: u16, col: &mut i32| {
        for c in v.to_string().bytes() {
            let d = (c - b'0') as u16;
            for half in 0..2u16 {
                let e = MapEntry { tile: 0x1B8 + 2 * d + half, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, *col, 18 + half as i32);
            }
            *col += 1;
        }
    };
    let damage = hand.damage.get(i).copied().unwrap_or(0);
    number(layer, damage, &mut col);
    let bonus = hand.attack_bonus.get(i).copied().unwrap_or(0) + hand.charge_bonus.get(i).copied().unwrap_or(0);
    if bonus != 0 {
        for half in 0..2u16 {
            let e = MapEntry { tile: 0x1CE + half, hflip: false, vflip: false, palette: 13 };
            put(layer, hud, pal, e, col, 18 + half as i32);
        }
        col += 1;
        number(layer, bonus, &mut col);
    }
}

/// An 8x16 glyph (tiles 2k and 2k + 1 of `tiles`) as a sprite.
fn glyph(tiles: &Tiles, k: usize, palette: Palette, x: i32, y: i32, priority: u8, vscale: Option<i32>) -> SpritePart<'_> {
    SpritePart {
        x: (x & 0x1FF) as u16,
        y: y as u8,
        width: 8,
        height: 16,
        tiles,
        first_tile: 2 * k,
        hflip: false,
        vflip: false,
        palette,
        priority,
        alpha: None,
        mosaic: None,
        vscale,
    }
}

fn block(tiles: &Tiles, w: u8, h: u8, palette: Palette, x: i32, y: i32) -> SpritePart<'_> {
    SpritePart { width: w, height: h, first_tile: 0, ..glyph(tiles, 0, palette, x, y, 2, None) }
}

/// The mugshot (by the navi's mood and form) and the count box beside it.
fn mugshot_parts<'a>(b: &Battle, hud: &'a Hud, state: &HudState, r: ObjectRef, x: i32, out: &mut Vec<SpritePart<'a>>) {
    let side = b.objects.get(r).alliance as usize;
    let stats = &b.stats[side];
    let m = state.mood.map(|m| m.shown()).unwrap_or_else(|| mood_index(b, r));
    let mut e = [0u8, 2, 3, 1, 5, 4][m as usize];
    // A Beast Out chosen on the custom screen shows before it happens.
    let form = b.transform_requests[side].form.filter(|f| f.0 <= 0x18).unwrap_or(stats.form).0;
    if form != 0 {
        let base = hud.form_emotions.get(form as usize).copied().unwrap_or(0);
        e = match form {
            11 | 12 if e == 3 => base + 1,
            1..=10 if e == 2 => base + 5,
            _ => base,
        };
    }
    let Some((gfx, pal)) = hud.mugshots.get(e as usize) else { return };
    out.push(block(gfx, 32, 16, *pal, x, 18));
    let count = stats.beast_out_counter as usize;
    let tiles = hud.counts.get(count).unwrap_or(&hud.count_box);
    out.push(block(tiles, 16, 16, *pal, x + 32, 18));
}

/// The chip icons stacked over a navi: the local player sees its next
/// chip; the opponent's show as hidden.
fn icon_parts<'a>(
    b: &Battle,
    hud: &'a Hud,
    r: ObjectRef,
    local: bool,
    view: &View,
    out: &mut Vec<SpritePart<'a>>,
    problems: &mut Problems,
) {
    let o = b.objects.get(r);
    if o.flags & flags::VISIBLE == 0 || o.chips_held == 0 {
        return;
    }
    let tiles = if local {
        let hand = &b.hands[o.alliance as usize];
        let Some(chip) = hand.ids.get(hand.cursor as usize).copied().flatten() else { return };
        // A chip's icon is the pack's image under the chip's key (a record
        // the pack numbers is found by its number).
        let def = b.content.defs.chip(chip);
        match hud.chip_icon(&def.key, def.record.id) {
            Some(t) => t,
            None => {
                problems.note(format!("chip {:?} ({}) has no icon in the pack", def.key, def.record.name));
                return;
            }
        }
    } else {
        &hud.hidden_icon
    };
    let p = project((o.pos.x, o.pos.y, o.pos.z), view);
    let a = if o.alliance == b.setup.local_side { 1 } else { -1 };
    let f = bn6_battle::kinds::common::facing(o.alliance, o.flip);
    // Attach point 3 of the navi's sprite (player NameIDs 0x1A0..=0x1C3).
    let (ax, ay) = if (0x1A0..=0x1C3).contains(&o.name_id) {
        let p = b.content.attach_point(o.name_id, 3);
        (p.x as i32, p.y as i32)
    } else {
        (8, 48)
    };
    let (x0, y0) = (p.x + a * (ax * f - 1) - 8, p.ground - ay - 8);
    // A navi off the field (Beast Out moves it far below) shows none.
    if !(-16..160).contains(&y0) || !(-16..240).contains(&x0) {
        return;
    }
    for k in 0..o.chips_held.min(6) as i32 {
        out.push(block(tiles, 16, 16, hud.icon_palette, x0 - 2 * k * a * f, y0 - 2 * k));
    }
}

/// The banner's five 32x16 sprites (as 8x16 glyphs) with its vertical
/// squash: grow over 5 frames, hold, shrink (`sub_801CE28`).
fn banner_parts<'a>(b: &Battle, hud: &'a Hud, id: u8, out: &mut Vec<SpritePart<'a>>, problems: &mut Problems) {
    let Some(layout) = hud.banners.get(id as usize / 4).filter(|l| !l.glyphs.is_empty()) else {
        problems.note(format!("banner {id:#04x} has no glyphs in the pack"));
        return;
    };
    let vscale = Some(banner_scale(b));
    let pal = hud.banner_palette;
    for k in 0..20usize {
        out.push(glyph(&layout.glyphs, k, pal, layout.x as i32 + 8 * k as i32, layout.y as i32, 0, vscale));
    }
    if let Some((nx, ny)) = layout.number_at {
        let n = if id == 0x0C { b.round.turn } else { b.round.round } as usize;
        let (tens, ones) = (n / 10 % 10, n % 10);
        let (tens, nx) = if tens == 0 { (10, nx as i32 - 4) } else { (tens, nx as i32) };
        // The number is drawn in front of the text.
        out.insert(0, glyph(&hud.banner_digits, ones, pal, nx + 8, ny as i32, 0, vscale));
        out.insert(0, glyph(&hud.banner_digits, tens, pal, nx, ny as i32, 0, vscale));
    }
}

/// A banner's vertical scale this frame (`sub_801CE28`: the texture rows
/// step by this over 256 per screen row).
fn banner_scale(b: &Battle) -> i32 {
    let t = b.banner.timer as i32;
    let scale = match b.banner.step {
        0 => 0xE0 - 0x20 * t,
        4 => match t {
            1 | 0x2F => 0x34,
            2 | 0x2E => 0x38,
            _ => 0x40,
        },
        _ => 0x40 + 0x20 * t,
    };
    scale * 4
}

/// A telop (`sub_801E95C` lays it out, `sub_801CF9E` draws it): the chip's
/// name in the font's glyphs, then for a chip whose damage shows the
/// damage, "+bonus" and "x2", centred in the viewer's half of the screen,
/// with the banners' squash.
fn telop_parts<'a>(
    b: &Battle,
    hud: &'a Hud,
    id: u8,
    telop: ShownTelop,
    out: &mut Vec<SpritePart<'a>>,
    problems: &mut Problems,
) {
    let Some(layout) = hud.banners.get(id as usize / 4) else {
        problems.note(format!("the telop's banner {id:#04x} is not in the pack"));
        return;
    };
    let name = match telop.name {
        TelopName::Chip(chip) => name_glyphs(b, hud, chip, problems),
        TelopName::Hidden => hud.glyphs("????").0,
        TelopName::Unknown => {
            problems.note("a telop names a chip the engine wasn't told (a dimming content starts itself)".into());
            Vec::new()
        }
    };
    // Digits right to left, as the game's BCD: glyph d of the HUD layer's
    // damage digits, '+' before a bonus.
    let digits = |v: u16| -> Vec<usize> { v.to_string().bytes().map(|c| DAMAGE_DIGIT + (c - b'0') as usize).collect() };
    let damage = if telop.damage != 0 { digits(telop.damage) } else { Vec::new() };
    let bonus = if telop.damage != 0 && telop.bonus != 0 {
        std::iter::once(PLUS_GLYPH).chain(digits(telop.bonus)).collect()
    } else {
        Vec::new()
    };
    let doubled = telop.doubled && telop.damage != 0;
    // The name's place: centred as fifteen glyphs are; the other player's
    // moves over for the "x2".
    let width = (name.len() + damage.len() + bonus.len()) as i32;
    let mut x = (layout.x as i32 + (15 - width) * 4) & 0xFF;
    if telop.remote && doubled {
        x = (x - 16) & 0xFF;
    }
    let y = layout.y as i32;
    let vscale = Some(banner_scale(b));
    let pal = hud.hp_palettes[0];
    for c in name {
        out.push(glyph(&hud.font, c as usize, pal, x, y, 0, vscale));
        x += 8;
    }
    for g in damage.into_iter().chain(bonus) {
        out.push(glyph(&hud.tiles, g, pal, x, y, 0, vscale));
        x += 8;
    }
    if doubled {
        out.push(glyph(&hud.tiles, TIMES_GLYPH, pal, x, y, 0, vscale));
        out.push(glyph(&hud.tiles, TIMES_GLYPH + 1, pal, x + 8, y, 0, vscale));
    }
}

/// Glyphs of the HUD layer's tiles (tile `first_tile + 2k`): the damage
/// digits (tiles 0x1B8..), '+' (0x1CE), and the 'x' and '2' of a doubled
/// chip (0x1D2, 0x1D4).
const DAMAGE_DIGIT: usize = (0x1B8 - 0x1A0) / 2;
const PLUS_GLYPH: usize = (0x1CE - 0x1A0) / 2;
const TIMES_GLYPH: usize = (0x1D2 - 0x1A0) / 2;
