//! The battle HUD: the player's HP box, the custom gauge and the chip name
//! on the HUD layer (priority 1); the mugshot, the opponent's HP, the chip
//! icons over the navis and the banners as sprites.
//!
//! The rolling numbers are presentation state the engine doesn't keep, so
//! `HudState` follows them tick by tick (`sub_801C840`, `sub_801C168`).

use crate::audit::Problems;
use crate::compose::{Layer, SpritePart};
use crate::objects::{SpriteList, View, project_hud};
use nettai_assets::{Bundle, Hud, MapEntry, Palette, Tiles};
use nettai_battle::Battle;
use nettai_battle::actor::status;
use nettai_battle::transform::{SequencerState, TransformPhase};
use nettai_battle::battle::{fight, mode, top};
use nettai_battle::content::{ChipFlags, FormKind};
use nettai_battle::kinds::player::{Emotion, emotion};
use nettai_content_api::FormHandle;
use nettai_content_api::ChipHandle;
use nettai_battle::hud::HpNumber;
use nettai_battle::object::ObjectRef;
use nettai_battle::perspective::{ShownTelop, TelopName};

/// The player navi's action while it stands waiting for input.
const FULL: u16 = nettai_battle::hud::CustomGauge::FULL;

/// HUD presentation state that rolls from frame to frame.
#[derive(Clone, Debug, Default)]
pub struct HudState {
    hp: Option<RollingHp>,
    /// The local console's HP numbers under objects, by place.
    hp_numbers: [Option<HpNumberShown>; HpNumber::PLACES],
    /// The HUD's animation counter (`eStruct2035280` +0): "Cstmzing..."
    /// counts it from 0 around 0x40 while it waits, and every draw of the
    /// full gauge counts it on, around 0x70 (`sub_801CA28`, `sub_801C4E4`).
    /// The full gauge's stripes and its "L or R" go by it, from wherever
    /// the last wait left it.
    frame: u8,
    /// The mugshot's mood and its change blink.
    mood: Option<Mood>,
    /// As of the previous tick: whether the round was decided (the HUD
    /// thins out a frame after the decision) and the gauge was running.
    was_over: bool,
    gauge_was_on: bool,
    is_over: bool,
    gauge_is_on: bool,
}

/// The local navi's face in the emotion window (`sub_801E6A8`): what its
/// form's definition shows for its emotion (`mugshot`), and the count
/// beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Face {
    /// The mugshot's number (none: the form names no face).
    picture: Option<u8>,
    /// One of MegaMan's own faces (the base form's: pictures 0..=4 in
    /// BN6), which blink when they change.
    own: bool,
    full_synchro: bool,
    count: u8,
}

impl Face {
    fn of(b: &Battle, r: ObjectRef) -> Face {
        let side = b.objects.get(r).alliance;
        Face::in_form(b, r, b.stats[side as usize].form)
    }

    /// The face `r`'s navi shows in `form`.
    fn in_form(b: &Battle, r: ObjectRef, form: FormHandle) -> Face {
        let side = b.objects.get(r).alliance;
        let emotion = emotion(b, side);
        let f = b.content.form(form);
        Face {
            picture: f.mugshot.map(|faces| faces.of(emotion)),
            own: f.kind == FormKind::Base,
            full_synchro: emotion == Emotion::FullSynchro,
            count: b.stats[side as usize].beast_out_counter,
        }
    }
}

/// The emotion window's face and, for 12 ticks after its picture changes,
/// its blink (`sub_801CB38`): back to the picture (and count) before on
/// two ticks of every four; to Full Synchro, white on those instead. The
/// forms' pictures change at once.
#[derive(Clone, Copy, Debug)]
struct Mood {
    now: Face,
    before: Face,
    blink: u8,
    /// Show `before` this frame.
    flash: bool,
    /// Show the face white this frame.
    white: bool,
}

impl Mood {
    fn shown(self) -> Face {
        if self.flash { self.before } else { self.now }
    }
}

#[derive(Clone, Copy, Debug)]
struct RollingHp {
    shown: u16,
    /// 0 normal, 1 healing, 2 hurt or low.
    colour: u8,
    hold: u8,
}

/// An HP number as it rolls (`sub_801C168`, an entry of `byte_203EB50`).
#[derive(Clone, Copy, Debug)]
struct HpNumberShown {
    /// What its place numbers (a new one starts over).
    number: HpNumber,
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
        (self.was_over, self.gauge_was_on) = (self.is_over, self.gauge_is_on);
        if let Some(n) = waiting_ticks(b) {
            self.frame = (n & 0x3F) as u8;
        } else if gauge_shown(b, self) && b.gauge.value >= FULL && !b.late_turns() {
            self.frame = if self.frame + 1 >= 0x70 { 0 } else { self.frame + 1 };
        }
        if let Some(r) = b.player(b.setup.local_side) {
            let now = Face::of(b, r);
            let m = self.mood.get_or_insert(Mood { now, before: now, blink: 0, flash: false, white: false });
            if now.picture != m.now.picture {
                *m = Mood { now, before: m.now, blink: 12, flash: false, white: false };
            } else if now.count != m.now.count {
                m.before.count = m.now.count;
                m.now = now;
            }
            (m.flash, m.white) = (false, false);
            if m.now.own && m.blink > 0 {
                let on = m.blink & 2 != 0;
                m.blink -= 1;
                if m.now.full_synchro {
                    m.white = on;
                } else {
                    m.flash = on;
                }
            }
        }
        (self.is_over, self.gauge_is_on) = (decided(b), b.gauge.enabled);
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
        // The HP numbers under objects (the opponent's navi, LilBoiler):
        // from the HP when asked for, rolling to it.
        for (place, shown) in b.hp_numbers[local as usize & 1].iter().zip(&mut self.hp_numbers) {
            let Some(number) = *place else {
                *shown = None;
                continue;
            };
            let e = match shown {
                Some(e) if e.number == number => e,
                _ => shown.insert(HpNumberShown { number, shown: number.hp, colour: 0, timer: 0 }),
            };
            let o = b.objects.get(number.object);
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
        }
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

/// While the local player's result is sent and the opponent's isn't in:
/// the ticks "Cstmzing..." has been up (`sub_801E474` starts it).
fn waiting_ticks(b: &Battle) -> Option<u32> {
    let sent = b.custom.sides[b.setup.local_side as usize].sent.as_ref()?;
    // (On the tick the custom mode starts the screens haven't opened yet:
    // what was sent is the last screen's.)
    (crate::custom::screens_open(b) && !b.custom.committed).then(|| b.round.ticks.saturating_sub(sent.sent_at + 1))
}

/// Whether the custom gauge is drawn (a chip's effect may hide it:
/// `Battle::hud_hidden`).
fn gauge_shown(b: &Battle, state: &HudState) -> bool {
    (b.gauge.enabled || state.gauge_was_on)
        && !b.hud_hidden.gauge
        && !state.was_over
        && !custom_open(b)
        && !crate::custom::gauge_held(b)
        && !transform_hides(b).1
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
/// The mugshot of a player who chose a form on the custom screen is gone
/// from the tick the fight resumes: the screen showed the form's face in
/// the window, and closing takes the window down with it (`sub_802A0F8`).
fn transform_hides(b: &Battle) -> (bool, bool) {
    let chose = b.transform_requests[b.setup.local_side as usize & 1].form.is_some();
    match b.transform_seq.state {
        SequencerState::Transform { phase: TransformPhase::FadeOut, started } => (started || chose, false),
        SequencerState::Transform { .. } => (true, true),
        _ => (chose && b.round.mode == mode::FIGHTING && b.round.init == 0, false),
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
    // HP box, top left (moved right with the custom screen's window).
    let shift = crate::custom::hud_shift(b);
    if let Some(r) = player.filter(|_| !hide_boxes) {
        let shown = state.hp.map(|h| h.shown).unwrap_or(b.objects.get(r).hp);
        let pal = &hud.hp_palettes[colour.min(2)];
        for (i, &e) in hud.hp_box.iter().enumerate() {
            put_px(layer, hud, pal, e, shift + 8 * (i as i32 % 6), 8 * (i as i32 / 6));
        }
        for (k, d) in digits4(shown).into_iter().enumerate() {
            let top = MapEntry { tile: 0x1A0 + 2 * d as u16, hflip: false, vflip: false, palette: 13 };
            put_px(layer, hud, pal, top, shift + 8 + 8 * k as i32, 0);
            put_px(layer, hud, pal, MapEntry { tile: top.tile + 1, ..top }, shift + 8 + 8 * k as i32, 8);
        }
    }

    // Custom gauge, top centre.
    if gauge_shown(b, state) {
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
    if waiting_ticks(b).is_some_and(|n| (n / 32) % 2 == 0) {
        for i in 0..16usize {
            if let Some(t) = hud.waiting.get(i) {
                layer.draw_tile(t, &hud.waiting_palette, (22 + (i % 8) as i32) * 8, (4 + (i / 8) as i32) * 8, false, false);
            }
        }
    }

    // "????" beside the mugshot while the local side has a defensive chip
    // set (a trap, a barrier chip's record), and at the right edge while
    // the other side has (`sub_801C984`, `sub_801C9A4`: four '?' of the
    // HUD layer's tiles). The custom screen's opening takes them off
    // (`sub_801DACC(0x400)`), and they are back with the gauge, once the
    // local result is sent.
    if !open && !hide_boxes && !crate::custom::gauge_held(b) {
        let pal = &hud.hp_palettes[colour.min(2)];
        for (side, column) in [(local, 6), (local ^ 1, 26)] {
            if b.linked[side as usize & 1].chip.is_none() {
                continue;
            }
            for i in 0..8i32 {
                let e = MapEntry { tile: 0x1CC + (i / 4) as u16, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, column + i % 4, 2 + i / 4);
            }
        }
    }

    // The judge's numbers under its banner: the damage each side dealt,
    // the local side's on the left, around "VS" (`sub_801D048`: the HUD
    // layer's damage digits, the left number ending at column 13, the
    // right one starting at column 17), until the banner slides out.
    if b.banner_for(local).is_some_and(|id| hud.banners.get(id.0 as usize / 4).is_some_and(|l| l.kind == 4))
        && b.banner.step < 8
    {
        let pal = &hud.hp_palettes[0];
        let j = &b.fight.judge;
        // While the digits roll the numbers are random; then the damage
        // the other side took and the local side took. The numbers are
        // drawn on the ticks that roll and the ticks that show the damage:
        // the tick between leaves what was there.
        let rolling = j.step == 4 && (j.sub == 4 && j.timer != 0x3C || j.sub == 8 && !j.sub_init);
        let taken = |side: u8| j.damage[1 - (side as usize & 1)];
        let (left, right) = if rolling { (j.rolled[0], j.rolled[1]) } else { (taken(local ^ 1), taken(local)) };
        let digit = |layer: &mut Layer, d: u8, col: i32| {
            for half in 0..2u16 {
                let e = MapEntry { tile: 0x1B8 + 2 * (d - b'0') as u16 + half, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, col, 5 + half as i32);
            }
        };
        let left = left.min(9999).to_string();
        for (i, d) in left.bytes().enumerate() {
            digit(layer, d, 13 - left.len() as i32 + i as i32);
        }
        for (i, c) in hud.glyphs("VS").0.into_iter().enumerate() {
            for half in 0..2 {
                if let Some(t) = hud.font.get(2 * c as usize + half) {
                    layer.draw_tile(t, pal, (14 + i as i32) * 8, (5 + half as i32) * 8, false, false);
                }
            }
        }
        for (i, d) in right.min(9999).to_string().bytes().enumerate() {
            digit(layer, d, 17 + i as i32);
        }
    }

    // The HUD's text lines, in the HP box's palette as the chip's name is.
    // The turn timer's seconds (from the 15th turn of a netbattle), then
    // "TIME UP!", at the top (`sub_801E398`: 8 glyphs from column 11),
    // while the fight runs.
    // "TIME UP!" stays for the judge's first second.
    let text_palette = &hud.hp_palettes[colour.min(2)];
    let timed = match b.fight.state {
        fight::FIGHTING | fight::PAUSE => b.fight.turn_timer != TURN_TICKS,
        fight::JUDGE => b.fight.sub == 0,
        _ => false,
    };
    if b.late_turns() && b.round.mode == mode::FIGHTING && timed {
        draw_text(layer, hud, text_palette, TEXT_TIME_UP + (b.fight.turn_timer / 60) as usize, 11, 0, 8, problems);
    }
    // The message (`sub_801E270`: 17 glyphs from column 7, under the gauge).
    if let Some(m) = b.message {
        let text = match m.message {
            nettai_battle::hud::Message::CounterHit => TEXT_COUNTER_HIT,
        };
        draw_text(layer, hud, text_palette, text, 7, 2, 17, problems);
    }

    // The next chip's name (and damage) at the bottom left, while the
    // navi stands holding chips.
    if let Some(r) = player {
        let o = b.objects.get(r);
        let hand = &b.hands[local as usize];
        if b.chip_hud_for(local).window
            && o.chips_held != 0
            && let Some(chip) = hand.ids.get(hand.cursor as usize).copied().flatten()
        {
            let bonus = nettai_battle::kinds::player::next_chip_bonus(b, r);
            let doubled = nettai_battle::kinds::player::next_chip_doubles(b, r);
            draw_chip_name(b, layer, hud, &hud.hp_palettes[colour.min(2)], hand, chip, (bonus, doubled), problems);
        }
    }

    // Sprites, in the HUD tasks' order (`sub_801BF64`), each where its task
    // puts it among the sprite layers: the chip icons and the opponents' HP
    // among the field's sprites, by depth bucket (an object low on the
    // screen covers them); the mugshot, the banner and the other player's
    // chip name in the front layer.
    if !state.was_over {
        // In a netbattle the other player's navi has no icons: its entry is
        // taken out as it is set up (`sub_80172F0`), and again whenever the
        // local navi's status is reset (`sub_80144C0`).
        let link = b.setup.settings.effects & nettai_battle::setup::effects::LINK != 0;
        for side in 0..2u8 {
            if let Some(r) = b.player(side).filter(|_| side == local || !link) {
                icon_parts(b, hud, r, &view, list, problems);
            }
        }
    }
    // The HP numbers, each place in turn (`sub_801C202`).
    for e in state.hp_numbers.iter().flatten() {
        // Under the object wherever its position is on the screen, seen or
        // not (`sub_800362C`): a navi that blinks after a hit or is
        // invisible keeps its number.
        let number = e.number;
        let o = b.objects.get(number.object);
        let p = project_hud((o.pos.x, o.pos.y, o.pos.z), &view);
        if !on_screen(p) {
            continue;
        }
        // The damage taken instead (`sub_801C296`'s flag 0x10), from the
        // place without centring (flag 8).
        let value = if number.damage { o.max_hp.wrapping_sub(e.shown) } else { e.shown };
        let n = value.min(9999).to_string().len() as i32;
        let x = p.x + number.dx as i32 + if number.damage { 0 } else { 4 * n - 32 };
        let y = p.y + number.dy as i32;
        let digits = &hud.enemy_digits[e.colour.min(2) as usize];
        let mut group = Vec::new();
        for (k, d) in digits4(value).into_iter().enumerate() {
            if d == 10 {
                continue;
            }
            group.push(glyph(digits, d, hud.enemy_palette, x + 8 * k as i32, y, 2, None));
        }
        list.insert_at(FIELD_LAYER, HP_BUCKET, group);
    }
    // A flickering emotion window is black two ticks of every four
    // (`sub_801CC94`: its palette blanks), and isn't drawn on the middle
    // two of the flicker's twelve (`sub_801CDEC`).
    let window = b.consoles[local as usize & 1].emotion_window;
    let flicker = window.flicker_ticks;
    // (The window stays through the damage judge, under its banner: its
    // task stops with the round's result.)
    let over = state.was_over && !window.running;
    let hidden = b.hud_hidden.emotion_window;
    if let Some(r) = player.filter(|_| !over && !hidden && !hide_mugshot && !matches!(flicker, 5 | 6)) {
        let mut group = Vec::new();
        mugshot_parts(b, hud, state, r, shift, &mut group, problems);
        if window.flickers != 0 && (flicker + 1) & 2 != 0 {
            for part in &mut group {
                part.palette = [0; 16];
            }
        }
        list.insert_at(FRONT_LAYER, 0, group);
    }
    if let Some(id) = b.banner_for(local) {
        let mut group = Vec::new();
        let bucket = match b.telop_for(local) {
            Some(telop) => {
                telop_parts(b, hud, id.0, telop, &mut group, problems);
                NAME_BUCKET
            }
            None => {
                banner_parts(b, hud, id.0, &mut group, problems);
                0
            }
        };
        list.insert_at(FRONT_LAYER, bucket, group);
    }
    if let Some(used) = b.used_chip_for(local) {
        let mut group = Vec::new();
        used_chip_parts(b, hud, used, &mut group, problems);
        list.insert_at(FRONT_LAYER, NAME_BUCKET, group);
    }
    // "PAUSE" in the middle while a player holds the battle (`sub_801C9E4`:
    // a 32x16 and an 8x16 sprite at (100, 63), in the opponents' HP
    // digits' palette).
    if b.round.mode == mode::FIGHTING && b.fight.state == fight::PAUSE {
        let group = (0..5).map(|k| glyph(&hud.pause, k, hud.enemy_palette, 100 + 8 * k as i32, 63, 0, None)).collect();
        list.insert_at(FRONT_LAYER, NAME_BUCKET, group);
    }
    warning_parts(b, hud, &view, list, problems);
}

/// Where a warning marker over the custom gauge is.
const GAUGE_WARNING: (i32, i32) = (0x78, 0x0C);

/// The warning markers the console shows this tick (`sub_800AE90`), which
/// the main loop queues after the HUD's sprites (`sub_8009FCC`): a 16x16
/// arrow over the custom gauge or over a place on the field (projected as
/// the HUD's pieces are), the second frame while bit 3 of the console's
/// frame counter is set; none unless its place is within 16 pixels of the
/// screen's top left and its bottom right below (x + 16, y + 16) < (0xFF,
/// 0xB0). (A place left of or above the screen, within 16 pixels, makes
/// the original write a garbled sprite: not drawn here, and no netbattle
/// marker goes there.)
fn warning_parts<'a>(b: &Battle, hud: &'a Hud, view: &View, list: &mut SpriteList<'a>, problems: &mut Problems) {
    let console = b.setup.local_side as usize & 1;
    for w in &b.warnings[console] {
        let (x, y) = match w.at {
            None => GAUGE_WARNING,
            Some(at) => {
                let p = project_hud((at.x, at.y, at.z), view);
                (p.x, p.y)
            }
        };
        if !(0..0xFF - 16).contains(&x) || !(0..0xB0 - 16).contains(&y) {
            continue;
        }
        if hud.warning.is_empty() {
            problems.note("the pack has no warning marker (extract it again)".into());
            continue;
        }
        let frame = if b.consoles[console].frames & 8 != 0 { 4 } else { 0 };
        let part = SpritePart { first_tile: frame, priority: 0, ..block(&hud.warning, 16, 16, hud.warning_palette, x, y) };
        list.insert_at(FRONT_LAYER, 0, vec![part]);
    }
}

/// The sprite layers the HUD's tasks insert into (`sub_802FE28`'s r2), and
/// their depth buckets (r3): the front layer's first bucket for the mugshot
/// and the banners, its bucket 6 for a telop and the other player's chip
/// name; among the field's sprites, bucket 0xDF for the opponents' HP and
/// 0xD0 plus the chips left for each chip icon.
const FRONT_LAYER: usize = 0;
const FIELD_LAYER: usize = 2;
const NAME_BUCKET: usize = 6;
const HP_BUCKET: usize = 0xDF;
const ICON_BUCKET: usize = 0xD0;

fn put(layer: &mut Layer, hud: &Hud, pal: &Palette, e: MapEntry, tx: i32, ty: i32) {
    put_px(layer, hud, pal, e, tx * 8, ty * 8);
}

/// `put` at a pixel position.
fn put_px(layer: &mut Layer, hud: &Hud, pal: &Palette, e: MapEntry, x: i32, y: i32) {
    let tile = if e.tile >= hud.gauge_first_tile {
        hud.gauge_tiles.get((e.tile - hud.gauge_first_tile) as usize)
    } else {
        e.tile.checked_sub(hud.first_tile).and_then(|i| hud.tiles.get(i as usize))
    };
    if let Some(t) = tile {
        layer.draw_tile(t, pal, x, y, e.hflip, e.vflip);
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
    hand: &nettai_battle::hand::ChipHand,
    chip: ChipHandle,
    (bonus, doubled): (u16, bool),
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
    if bonus != 0 {
        for half in 0..2u16 {
            let e = MapEntry { tile: 0x1CE + half, hflip: false, vflip: false, palette: 13 };
            put(layer, hud, pal, e, col, 18 + half as i32);
        }
        col += 1;
        number(layer, bonus, &mut col);
    }
    // "x2" while the use would double it (two glyphs: tiles 0x1D2..).
    if doubled {
        for k in 0..2u16 {
            for half in 0..2u16 {
                let e = MapEntry { tile: 0x1D2 + 2 * k + half, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, col + k as i32, 18 + half as i32);
            }
        }
    }
}

/// The pack's text lines (`Hud::texts`): "TIME UP!", then the seconds 1-10;
/// and "COUNTER HIT!".
const TEXT_TIME_UP: usize = 3;
/// The turn timer's start: it shows once it has counted.
const TURN_TICKS: u16 = 0xA5 * 4 - 1;
const TEXT_COUNTER_HIT: usize = 14;

/// Draw the pack's text line `text` on the HUD layer: up to `width` glyphs
/// from tile column `col`, on tile rows `row` and `row + 1`.
fn draw_text(
    layer: &mut Layer,
    hud: &Hud,
    pal: &Palette,
    text: usize,
    col: i32,
    row: i32,
    width: usize,
    problems: &mut Problems,
) {
    let Some(glyphs) = hud.texts.get(text) else {
        problems.note(format!("the pack has no HUD text line {text}"));
        return;
    };
    for (i, &g) in glyphs.iter().take(width).enumerate() {
        for half in 0..2 {
            if let Some(t) = hud.font.get(2 * g as usize + half) {
                layer.draw_tile(t, pal, (col + i as i32) * 8, (row + half as i32) * 8, false, false);
            }
        }
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
        affine: None,
    }
}

fn block(tiles: &Tiles, w: u8, h: u8, palette: Palette, x: i32, y: i32) -> SpritePart<'_> {
    SpritePart { width: w, height: h, first_tile: 0, ..glyph(tiles, 0, palette, x, y, 2, None) }
}

/// The mugshot (by the navi's emotion and form) and the count box beside
/// it; a link navi's own face (plain, or in Full Synchro) and the box
/// beside it (`sub_801CC34`).
fn mugshot_parts<'a>(
    b: &Battle,
    hud: &'a Hud,
    state: &HudState,
    r: ObjectRef,
    x: i32,
    out: &mut Vec<SpritePart<'a>>,
    problems: &mut Problems,
) {
    let side = b.objects.get(r).alliance as usize;
    let stats = &b.stats[side];
    // A link navi's own face, its definition's (MegaMan's, the navi that
    // changes form, is his form's): in its second palette in Full Synchro.
    let navi = b.content.navi(stats.navi);
    if !navi.changes_form() {
        let Some((tiles, palettes)) = navi.mugshot.and_then(|n| hud.mugshot(n)) else {
            problems.note(format!("navi {:?} has no mugshot in the pack", b.content.defs.navi(stats.navi).key));
            return;
        };
        let full_synchro = emotion(b, side as u8) == Emotion::FullSynchro;
        let pal = palettes.get(full_synchro as usize).or(palettes.first()).copied().unwrap_or_default();
        out.push(block(tiles, 32, 16, pal, x, 18));
        out.push(block(&hud.navi_box, 16, 16, pal, x + 32, 18));
        note_true_face(b, side, navi.mugshot, x, problems);
        return;
    }
    let mut face = state.mood.map(|m| m.shown()).unwrap_or_else(|| Face::of(b, r));
    // A form chosen on the custom screen shows in the window until the
    // screens close (`sub_802A040`, `sub_802A088`).
    if let Some(form) = crate::custom::face(b, side) {
        face = Face::in_form(b, r, form);
    }
    let Some((gfx, palettes)) = face.picture.and_then(|n| hud.mugshot(n)) else {
        problems.note(format!("form {:?} has no mugshot in the pack", b.content.defs.form(b.stats[side].form).key));
        return;
    };
    // (The white of a change to Full Synchro: `byte_801CD80`.)
    let pal = if state.mood.is_some_and(|m| m.white) { [0x7FFF; 16] } else { palettes.first().copied().unwrap_or_default() };
    out.push(block(gfx, 32, 16, pal, x, 18));
    let tiles = if beast_count_shown(b, side as u8) { hud.counts.get(face.count as usize) } else { None };
    out.push(block(tiles.unwrap_or(&hud.count_box), 16, 16, pal, x + 32, 18));
    note_true_face(b, side, face.picture, x, problems);
}

/// The faces Gregar has of its own (the pack's, from the Gregar ROM: its
/// emotion-window pictures from 0x17, its link navis' after the Falzar
/// ROM's six).
fn gregar_face(picture: u8) -> bool {
    (0x17..0x80).contains(&picture) || (nettai_assets::NAVI_MUGSHOTS + 6..nettai_assets::NAVI_MUGSHOTS + 11).contains(&picture)
}

/// A Falzar console shows every form's and navi's true face, where the
/// original Falzar console has none for Gregar's and shows the Falzar
/// counterpart's (deliberately: docs/frontend.md §5): the face and the box
/// beside it, in the face's palette, are a known difference.
fn note_true_face(b: &Battle, side: usize, picture: Option<u8>, x: i32, problems: &mut Problems) {
    let falzar_console = b.custom.sides[b.setup.local_side as usize & 1].unlocks.version == nettai_battle::custom::GameVersion::Falzar;
    if falzar_console && picture.is_some_and(gregar_face) && side == b.setup.local_side as usize & 1 {
        problems.known(x, 18, 48, 16, "a Gregar face on a Falzar console (the true face)");
    }
}

/// `sub_801D814`: whether the emotion window shows the Beast Out count
/// (else its empty box): always in battle mode 5, never in mode 1, and
/// otherwise while the console's save has Beast Out (event flag 0xE0) and
/// hasn't sealed it (0x163), in a battle without a gauge for each player
/// (battle flag 0x40) that isn't random (effects 0x200000).
fn beast_count_shown(b: &Battle, side: u8) -> bool {
    use nettai_battle::battle::battle_flags;
    use nettai_battle::setup::effects;
    match b.round.mode_copy {
        5 => true,
        1 => false,
        _ => {
            let u = b.custom.sides[side as usize & 1].unlocks;
            u.beast_out
                && !u.beast_out_sealed
                && b.round.flags & battle_flags::PER_PLAYER_GAUGES == 0
                && b.setup.settings.effects & effects::RANDOM == 0
        }
    }
}

/// Whether an object's HUD pieces show (`sub_800362C`): its position
/// projects onto the screen or its margin.
fn on_screen(p: crate::objects::Projected) -> bool {
    (-0x20..0x110).contains(&p.x) && (-0x20..0xE0).contains(&p.y)
}

/// The chip icons stacked over a navi (`sub_801C082`): its next chip's
/// icon, once for every chip it holds, wherever the navi's position is on
/// the screen (seen or not), but not while its navi chip's navi stands in
/// for it.
fn icon_parts<'a>(
    b: &Battle,
    hud: &'a Hud,
    r: ObjectRef,
    view: &View,
    list: &mut SpriteList<'a>,
    problems: &mut Problems,
) {
    let o = b.objects.get(r);
    let vanished = o.actor.is_some_and(|a| b.actors.get(a).status & status::VANISHED != 0);
    if o.chips_held == 0 || vanished || !b.chip_hud_for(o.alliance).icons {
        return;
    }
    let hand = &b.hands[o.alliance as usize];
    let Some(chip) = hand.ids.get(hand.cursor as usize).copied().flatten() else { return };
    // A chip's icon is the pack's image under the chip's key.
    let def = b.content.defs.chip(chip);
    let Some(tiles) = hud.chip_icon(&def.key) else {
        problems.note(format!("chip {:?} ({}) has no icon in the pack", def.key, def.record.name));
        return;
    };
    let p = project_hud((o.pos.x, o.pos.y, o.pos.z), view);
    if !on_screen(p) {
        return;
    }
    // (The console's own direction: `object_getAllianceDirection` of the
    // local side, whichever navi the icons are over.)
    let a = if b.setup.local_side & 1 == 0 { 1 } else { -1 };
    let f = nettai_battle::kinds::common::facing(o.alliance, o.flip);
    // Attach point 3 of the navi's sprite (player NameIDs 0x1A0..=0x1C3).
    let (ax, ay) = if b.content.identity(o.identity).class.is_player() {
        let p = b.content.attach_point(o.identity, 3);
        (p.x as i32, p.y as i32)
    } else {
        (8, 48)
    };
    let (x0, y0) = (p.x + a * (ax * f - 1) - 8, p.y - ay - 8);
    // The first icon is the front one; each next is a bucket back.
    let count = o.chips_held.min(6) as i32;
    for k in 0..count {
        let icon = block(tiles, 16, 16, hud.icon_palette, x0 - 2 * k * a * f, y0 - 2 * k);
        list.insert_at(FIELD_LAYER, ICON_BUCKET + (count - k) as usize, vec![icon]);
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
    let numbers = (telop.damage, telop.bonus, telop.doubled);
    name_parts(hud, layout, name, numbers, telop.remote, Some(banner_scale(b)), out);
}

/// The chip the other player just used (`sub_801EB18` lays it out as a
/// telop on the right, `sub_801D1F6` draws it): its name and numbers,
/// without the banners' squash.
fn used_chip_parts<'a>(
    b: &Battle,
    hud: &'a Hud,
    used: nettai_battle::hud::UsedChip,
    out: &mut Vec<SpritePart<'a>>,
    problems: &mut Problems,
) {
    // (Its place is the banner of the other player's telop.)
    let remote_telop = b.content.defs.roles.banner(nettai_battle::content::BannerRole::TelopRemote).0 as usize;
    let Some(layout) = hud.banners.get(remote_telop / 4) else {
        problems.note(format!("the telop's banner {remote_telop:#04x} is not in the pack"));
        return;
    };
    let name = name_glyphs(b, hud, used.chip, problems);
    name_parts(hud, layout, name, (used.damage, used.bonus, used.doubled), true, None, out);
}

/// A chip's name with its numbers (damage, bonus, doubled) as `sub_801E95C`
/// lays them out from a telop banner's place: centred as fifteen glyphs
/// are; the other player's (`remote`) moves over for the "x2".
fn name_parts<'a>(
    hud: &'a Hud,
    layout: &nettai_assets::BannerLayout,
    name: Vec<u16>,
    (damage, bonus, doubled): (u16, u16, bool),
    remote: bool,
    vscale: Option<i32>,
    out: &mut Vec<SpritePart<'a>>,
) {
    // Glyph d of the HUD layer's damage digits, '+' before a bonus.
    let digits = |v: u16| -> Vec<usize> { v.to_string().bytes().map(|c| DAMAGE_DIGIT + (c - b'0') as usize).collect() };
    let (damage, bonus, doubled) = match damage {
        0 => (Vec::new(), Vec::new(), false),
        d => (digits(d), if bonus != 0 { std::iter::once(PLUS_GLYPH).chain(digits(bonus)).collect() } else { Vec::new() }, doubled),
    };
    let width = (name.len() + damage.len() + bonus.len()) as i32;
    let mut x = (layout.x as i32 + (15 - width) * 4) & 0xFF;
    if remote && doubled {
        x = (x - 16) & 0xFF;
    }
    let y = layout.y as i32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::BannerLayout;

    use nettai_battle::content::{BannerId, testing};
    use nettai_battle::hud::{Telop, TelopHidden};

    const CHARS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz?";

    /// A HUD whose font draws `CHARS` (glyph k its kth character) and
    /// whose telops sit where BN6's do.
    fn hud() -> Hud {
        let mut banners = vec![BannerLayout::default(); 21];
        banners[0x4C / 4] = BannerLayout { x: 0, y: 32, kind: 3, ..BannerLayout::default() };
        banners[0x50 / 4] = BannerLayout { x: 120, y: 32, kind: 3, ..BannerLayout::default() };
        Hud {
            tiles: Tiles { pixels: vec![1; 0x36 * Tiles::TILE] },
            first_tile: 0x1A0,
            font: Tiles { pixels: vec![1; 2 * CHARS.len() * Tiles::TILE] },
            font_chars: CHARS.chars().map(String::from).collect(),
            banners,
            ..Hud::default()
        }
    }

    fn battle() -> Battle {
        Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(100)), testing::content())
    }

    fn glyph_of(c: char) -> usize {
        2 * CHARS.find(c).unwrap()
    }

    #[test]
    fn a_telop_names_the_chip_with_its_damage_and_bonus() {
        let mut b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        assert!(b.start_banner(BannerId(0x4C)));
        b.banner.telop =
            Some(Telop { side: 0, chip: Some(chip), damage: 120, doubled: true, bonus: 10, hidden: TelopHidden::No });
        let hud = hud();
        let mut problems = Problems::default();
        // Its user's console: "SunGun3" "120" "+10" is 13 glyphs, centred as
        // 15 are, then "x2".
        let mut parts = Vec::new();
        telop_parts(&b, &hud, 0x4C, b.telop_for(0).unwrap(), &mut parts, &mut problems);
        let xs: Vec<u16> = parts.iter().map(|p| p.x).collect();
        assert_eq!(xs, (0..15).map(|i| 8 + 8 * i).collect::<Vec<u16>>());
        assert!(parts.iter().all(|p| p.y == 32 && p.priority == 0 && p.vscale.is_some()));
        let name: Vec<usize> = parts[..7].iter().map(|p| p.first_tile).collect();
        assert_eq!(name, "SunGun3".chars().map(glyph_of).collect::<Vec<_>>());
        // The digits and the signs are the HUD layer's tiles.
        let after: Vec<usize> = parts[7..].iter().map(|p| p.first_tile).collect();
        let digit = |d: usize| 2 * (DAMAGE_DIGIT + d);
        let signs = [2 * PLUS_GLYPH, 2 * TIMES_GLYPH, 2 * TIMES_GLYPH + 2];
        assert_eq!(after, [digit(1), digit(2), digit(0), signs[0], digit(1), digit(0), signs[1], signs[2]]);
        // The other player's console: on the right, moved over for the "x2".
        let mut parts = Vec::new();
        telop_parts(&b, &hud, 0x50, b.telop_for(1).unwrap(), &mut parts, &mut problems);
        assert_eq!(parts[0].x, 120 + 8 - 16);
        assert!(problems.is_empty(), "{:?}", problems.lines());
    }

    #[test]
    fn a_hidden_telop_shows_question_marks_to_the_other_player() {
        let mut b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        assert!(b.start_banner(BannerId(0x4C)));
        let hidden = TelopHidden::FromOpponent;
        b.banner.telop = Some(Telop { side: 0, chip: Some(chip), damage: 0, doubled: false, bonus: 0, hidden });
        let hud = hud();
        let mut problems = Problems::default();
        let mut parts = Vec::new();
        telop_parts(&b, &hud, 0x50, b.telop_for(1).unwrap(), &mut parts, &mut problems);
        assert_eq!(parts.iter().map(|p| p.first_tile).collect::<Vec<_>>(), [glyph_of('?'); 4]);
        // Four glyphs centred as fifteen are, from the right banner's place.
        assert_eq!(parts[0].x, 120 + 44);
        let mut parts = Vec::new();
        telop_parts(&b, &hud, 0x4C, b.telop_for(0).unwrap(), &mut parts, &mut problems);
        assert_eq!(parts.len(), 7, "its user sees the name");
    }

    #[test]
    fn a_chip_name_the_font_cannot_write_is_a_problem() {
        let b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        let mut hud = hud();
        hud.font_chars.retain(|c| c != "G");
        let mut problems = Problems::default();
        assert_eq!(name_glyphs(&b, &hud, chip, &mut problems).len(), 6);
        assert_eq!(problems.len(), 1);
        assert!(problems.lines()[0].contains("no glyph for ['G']"), "{:?}", problems.lines());
    }
}
