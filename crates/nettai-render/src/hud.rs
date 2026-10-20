//! The battle HUD: the player's HP box, the custom gauge and the chip name
//! on the HUD layer (the game's priority: 1, EXE4's 0); the mugshot, the
//! opponent's HP, the chip icons over the navis and the banners as sprites.
//!
//! The rolling numbers are presentation state the engine doesn't keep, so
//! `HudState` follows them tick by tick (`sub_801C840`, `sub_801C168`).

use crate::audit::Problems;
use crate::compose::{Layer, SpritePart};
use crate::fonts;
use crate::objects::{SpriteList, View, project_hud, project_hud_fine};
use crate::smooth::Smooth;
use crate::textlayer::{Align, Plane, Rect, TextItem, GraphicsMode, TextSink};
use crate::vfont::{LetteringFont, Lettering, Role};
use nettai_assets::letters::Digits;
use nettai_assets::{Bundle, Hud, MapEntry, Palette, TelopLook, TelopTiles, Tiles};
use nettai_battle::Battle;
use nettai_battle::actor::status;
use nettai_battle::transform::{SequencerState, TransformPhase};
use nettai_battle::battle::{fight, mode, top};
use nettai_battle::content::{BannerId, ChipFlags, ChipIcons};
use nettai_battle::kinds::player::{Emotion, EmotionRole, emotion, emotion_role};
use nettai_content_api::FormHandle;
use nettai_content_api::ChipHandle;
use nettai_battle::hud::{Banner, HpNumber};
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
    /// The custom screen showed the form chosen there in the emotion window
    /// as it was last up (EXE6's Beast Out and Crosses: `eStruct2035280`
    /// +0x4C, which takes the window down as the screen closes; EXE5's soul
    /// choice shows none).
    form_face_shown: bool,
    /// What last loaded the sprite tiles and the palette the banners share
    /// with the telops (`TelopTiles`: EXE4's).
    shared: SharedLoads,
}

/// What loaded a part of the sprite tiles or the palette the banners
/// share with the telops (`TelopTiles`) last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Load {
    #[default]
    Nothing,
    /// A banner as it started (0x0801617E): its glyphs, the banners'
    /// palette.
    Banner(BannerId),
    /// A telop as it started (0x0801650C): its name, the '×' and '2'
    /// pieces, the telops' palette.
    Telop(TelopName),
    /// "PAUSE" as it started (0x080166BC): its tiles, the banners' palette
    /// where the pack says (`pause_loads_banner_palette`).
    Pause,
}

/// The loads into the tiles and the palette the banners share with the
/// telops, as the HUD starts its pieces, and what showed the tick before
/// (to see each start).
#[derive(Clone, Copy, Debug, Default)]
struct SharedLoads {
    /// The right-hand telop's name tiles.
    right: Load,
    /// The '×' and '2' pieces' tiles.
    signs: Load,
    palette: Load,
    was_banner: Option<(BannerId, u8)>,
    /// The telops (`Battle::telops_for`: the banner block's, the other
    /// player's block's), each with its block's step.
    was_telops: [Option<(ShownTelop, u8)>; 2],
    was_used: Option<(ChipHandle, u8)>,
    was_paused: bool,
}

impl SharedLoads {
    /// Follow a tick: "PAUSE" (the fight's state), a banner (the flow's),
    /// a telop on the banner block, one on the other player's block and the
    /// other player's used chip (the navis' chips), in the order a tick
    /// starts them; each start loads what it shares (the original reloads
    /// nothing while a piece shows).
    fn tick(&mut self, b: &Battle) {
        let local = b.setup.local_side;
        let paused = b.round.mode == mode::FIGHTING && b.fight.state == fight::PAUSE;
        if paused && !self.was_paused {
            self.palette = Load::Pause;
        }
        self.was_paused = paused;
        let restarted = |step: u8, was: Option<u8>| was.is_some_and(|w| step == 0 && w != 0);
        let telops = b.telops_for(local);
        let step = b.banner.step;
        let banner = b.banner_for(local).filter(|_| telops[0].is_none());
        if let Some(id) = banner
            && (self.was_banner.map(|(w, _)| w) != Some(id) || restarted(step, self.was_banner.map(|(_, s)| s)))
        {
            (self.right, self.signs, self.palette) = (Load::Banner(id), Load::Banner(id), Load::Banner(id));
        }
        self.was_banner = banner.map(|id| (id, step));
        for (k, telop) in telops.into_iter().enumerate() {
            let was = self.was_telops[k];
            self.was_telops[k] = telop.map(|(t, block)| (t, block.step));
            if let Some((t, block)) = telop
                && (was.map(|(w, _)| w) != Some(t) || restarted(block.step, was.map(|(_, s)| s)))
            {
                self.telop_started(t.name, t.remote);
            }
        }
        let used = b.used_chip_for(local);
        if let Some(u) = used
            && self.was_used.is_none_or(|(chip, ticks)| chip != u.chip || u.ticks > ticks)
        {
            self.telop_started(TelopName::Chip(u.chip), true);
        }
        self.was_used = used.map(|u| (u.chip, u.ticks));
    }

    /// A telop's start (0x0801650C): its name into its place's tiles, the
    /// '×' and '2' pieces, the telops' palette.
    fn telop_started(&mut self, name: TelopName, right: bool) {
        if right {
            self.right = Load::Telop(name);
        }
        (self.signs, self.palette) = (Load::Telop(name), Load::Telop(name));
    }
}

/// The glyphs of the banner and of the right-hand telop that the other's
/// loads have made (`TelopTiles`: EXE4's banners and telops share sprite
/// tiles 0x380 to 0x387), and the palette each shows where the other
/// loaded the one they share. Made before the frame's sprites, whose parts
/// borrow its tiles.
#[derive(Default)]
pub struct SharedGlyphs {
    tiles: Tiles,
    /// The emotion window's count box, its digits taken off (the enhanced
    /// mode's count in the numbers' font: `count_blank`).
    count_blank: Tiles,
    /// The banner's glyphs (by number) drawn from `tiles` (from their first
    /// tile there).
    banner: Vec<(usize, usize)>,
    /// The right-hand telop's name glyphs drawn from `tiles`.
    right: Vec<(usize, usize)>,
    /// A telop's '×' and '2' drawn from `tiles` (their first tiles there).
    signs: Option<[usize; 2]>,
    /// The palette the banner shows, and the telops', where not their own.
    banner_palette: Option<Palette>,
    telop_palette: Option<Palette>,
}

impl SharedGlyphs {
    /// Make up a glyph of two tiles; its first tile in `tiles`.
    fn push(&mut self, halves: [(&Tiles, usize); 2]) -> usize {
        let first = self.tiles.len();
        for (tiles, t) in halves {
            let px = tiles.pixels.get(t * Tiles::TILE..(t + 1) * Tiles::TILE).unwrap_or(&[0; Tiles::TILE]);
            self.tiles.pixels.extend_from_slice(px);
        }
        first
    }

    /// A telop's style in the palette the telops show (the banners', where
    /// a banner or "PAUSE" loaded it last).
    fn telop_style<'a>(&self, style: TelopStyle<'a>) -> TelopStyle<'a> {
        TelopStyle { palette: self.telop_palette.unwrap_or(style.palette), ..style }
    }

    /// Draw a telop's '×' and '2' (its parts `at` and `at + 1`) from the
    /// made-up tiles.
    fn patch_signs<'a>(&'a self, group: &mut [SpritePart<'a>], at: Option<usize>) {
        if let (Some(at), Some(made)) = (at, self.signs) {
            for (k, first) in made.into_iter().enumerate() {
                if let Some(part) = group.get_mut(at + k) {
                    (part.tiles, part.first_tile) = (&self.tiles, first);
                }
            }
        }
    }

    /// Draw a group's glyphs from the made-up tiles: glyph `k` of `made` is
    /// the group's part `at + k`, of its first `count`.
    fn patch<'a>(&'a self, group: &mut [SpritePart<'a>], made: &[(usize, usize)], at: usize, count: usize) {
        for &(k, first) in made.iter().filter(|(k, _)| *k < count) {
            if let Some(part) = group.get_mut(at + k) {
                (part.tiles, part.first_tile) = (&self.tiles, first);
            }
        }
    }
}

/// Which of a banner's glyph tiles (two a glyph: its top, its bottom) is its
/// VRAM tile `o` from its first: its twenty glyphs are five 32x16 sprites,
/// each four tiles of tops then four of bottoms (0x0801617E's transfers).
fn banner_tile(o: usize) -> usize {
    2 * (4 * (o / 8) + o % 4) + (o % 8) / 4
}

/// What the banners and the telops show of each other's loads this frame
/// (`TelopTiles`): the tiles a load put where the other's glyphs are (a
/// banner's 40 tiles from `banner` as five 32x16 sprites: tile `o` glyph
/// 4 (o / 8) + o % 4, its top half for o % 8 < 4; a telop's name two
/// tiles a glyph from its place's tile, padded to eight with glyph 0; the
/// '×' and '2' pieces a 16x16 sprite), and the palette.
pub fn shared_glyphs<'a>(
    b: &Battle,
    assets: &'a Bundle,
    packs: &crate::packs::Packs<'a>,
    state: &HudState,
    text: &TextSink,
    problems: &mut Problems,
) -> SharedGlyphs {
    let mut out = SharedGlyphs::default();
    if text.draws_numbers() {
        out.count_blank = assets.hud.letters.counts.blank.clone();
    }
    let hud = &assets.hud;
    let Some((look, t)) = hud.layout.telop.as_ref().and_then(|l| l.tiles.map(|t| (l, t))) else { return out };
    let s = &state.shared;
    let local = b.setup.local_side;
    let banner_palette = matches!(s.palette, Load::Banner(_)) || s.palette == Load::Pause && t.pause_loads_banner_palette;
    out.telop_palette = banner_palette.then_some(hud.banner_palette);
    out.banner_palette = matches!(s.palette, Load::Telop(_)).then_some(look.palette);
    // A telop's name glyphs, as its start renders them (none in the enhanced
    // mode, whose name is the text layer's).
    let name_tiles = |name: TelopName, problems: &mut Problems| -> Option<Vec<(&'a Tiles, usize)>> {
        let codes = match name {
            TelopName::Chip(chip) => {
                let words = text.strings.chip_name(&b.content, chip);
                if text.takes(words) {
                    return None;
                }
                name_glyphs(b, hud, words, chip, problems)
            }
            TelopName::Hidden => fonts::cell_glyphs(hud, "????").0,
            TelopName::Unknown => Vec::new(),
        };
        Some((0..8).map(|k| fonts::cell_glyph(hud, codes.get(k).copied().unwrap_or(0))).collect())
    };
    let in_banner = |tile: u16| tile.checked_sub(t.banner).filter(|&o| o < TelopTiles::BANNER_TILES).map(|o| o as usize);
    let telops = b.telops_for(local);
    // The right-hand telop's name where a banner loaded over it.
    let right_shown = b.used_chip_for(local).is_some() || telops.iter().flatten().any(|(x, _)| x.remote);
    if right_shown
        && let Load::Banner(id) = s.right
        && let Some((_, layout, _)) = crate::lookups::banner(packs, &b.content, id, problems)
    {
        // (A glyph whose two tiles the banner's load covers: EXE4's first
        // two, tiles 0x384 to 0x387, whose glyphs fall in or out whole.)
        for k in 0..(TelopTiles::NAME_TILES / 2) as usize {
            let tiles = [t.names[1] + 2 * k as u16, t.names[1] + 2 * k as u16 + 1];
            let (Some(top), Some(bottom)) = (in_banner(tiles[0]), in_banner(tiles[1])) else { continue };
            let first = out.push([(&layout.glyphs, banner_tile(top)), (&layout.glyphs, banner_tile(bottom))]);
            out.right.push((k, first));
        }
    }
    // A telop's '×' and '2' (either place's: one 16x16 sprite at `signs`,
    // the '×' its left column, 0x08015522) where a banner loaded over them.
    // (The enhanced mode's telops keep their glyphs: their names are the text
    // layer's.)
    let telop_shown = b.used_chip_for(local).is_some() || telops.iter().any(Option::is_some);
    if telop_shown
        && text.mode == GraphicsMode::Original
        && let Load::Banner(id) = s.signs
        && let Some((_, layout, _)) = crate::lookups::banner(packs, &b.content, id, problems)
    {
        let column = |c: u16| Some([in_banner(t.signs + c)?, in_banner(t.signs + c + 2)?].map(banner_tile));
        if let (Some(times), Some(two)) = (column(0), column(1)) {
            out.signs = Some([times, two].map(|[top, bottom]| out.push([(&layout.glyphs, top), (&layout.glyphs, bottom)])));
        }
    }
    // The banner's glyphs where a telop loaded over them.
    let banner_shown = b.banner_for(local).is_some() && telops[0].is_none();
    let signs = matches!(s.signs, Load::Telop(_));
    let right = match s.right {
        Load::Telop(name) => name_tiles(name, problems),
        _ => None,
    };
    if banner_shown
        && (signs || right.is_some())
        && let Some(id) = b.banner_for(local)
        && let Some((_, layout, _)) = crate::lookups::banner(packs, &b.content, id, problems)
    {
        let sign = |o: u16| -> Option<(&'a Tiles, usize)> {
            // (The 16x16 sprite's tiles: the '×' and the '2' side by side,
            // top row first.)
            let g = TelopLook::TIMES + (o % 2) as usize;
            signs.then_some((&look.glyphs, 2 * g + (o / 2) as usize))
        };
        for g in 0..20usize {
            let offsets = [8 * (g / 4) + g % 4, 8 * (g / 4) + g % 4 + 4].map(|o| t.banner + o as u16);
            let halves = offsets.map(|v| {
                let from_signs = v.checked_sub(t.signs).filter(|&o| o < TelopTiles::SIGN_TILES).and_then(sign);
                let from_name = v
                    .checked_sub(t.names[1])
                    .filter(|&o| o < TelopTiles::NAME_TILES)
                    .and_then(|o| right.as_ref().map(|r| (r[o as usize / 2].0, r[o as usize / 2].1 + o as usize % 2)));
                from_signs.or(from_name)
            });
            if halves.iter().all(Option::is_none) {
                continue;
            }
            let own = |h: usize| (&layout.glyphs, 2 * g + h);
            let first = out.push([halves[0].unwrap_or(own(0)), halves[1].unwrap_or(own(1))]);
            out.banner.push((g, first));
        }
    }
    out
}

/// The local navi's face in the emotion window (`sub_801E6A8`): what its
/// form's definition shows for its emotion (`mugshot`), and the count
/// beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Face {
    /// The mugshot's pack and number there (none: the form names no face).
    picture: Option<nettai_battle::content::InPack<u8>>,
    /// One of MegaMan's own faces (the base form's: pictures 0..=4 in
    /// EXE6), which blink when they change.
    own: bool,
    full_synchro: bool,
    count: u8,
    /// The form and the emotion it is the face of (its lookup's), and
    /// whether of the form's second set by the side's rules (the chaos
    /// look's is the moment's, as the window draws).
    form: FormHandle,
    emotion: Emotion,
    /// The base form's second set, as the side's rules ask
    /// (`base_face_variant`: EXE5's Hub Style).
    base_variant: bool,
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
        let variant = nettai_battle::kinds::player::shows_face_variant(b, side);
        let base_variant = nettai_battle::kinds::player::base_face_variant(b, side);
        let f = b.content.form(form);
        Face {
            picture: f.mugshot.as_ref().and_then(|faces| crate::packs::mugshot(&b.content, faces.shown(&b.game_rules().emotion, emotion, variant))),
            own: f.base,
            full_synchro: b.game_rules().emotion.role(emotion) == Some(EmotionRole::FullSynchro),
            // (The count the emotion window's box shows: a stat of the
            // game's own, by its role: EXE6's Beast Out turns.)
            count: match b.stat(side, nettai_battle::content::StatRole::WindowCount) {
                Some(nettai_content_api::FieldValue::U8(n)) => n,
                _ => 0,
            },
            form,
            emotion,
            base_variant,
        }
    }
}

/// The emotion window's face and, for 12 ticks after its picture changes,
/// its blink (`sub_801CB38`): back to the picture (and count) before on
/// two ticks of every four; to Full Synchro, white on those instead; to
/// Hub Style's Full Synchro, both, a tick of the twelve each. The forms'
/// pictures change at once.
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
    color: u8,
    hold: u8,
}

/// An HP number as it rolls (`sub_801C168`, an entry of `byte_203EB50`).
#[derive(Clone, Copy, Debug)]
struct HpNumberShown {
    /// What its place numbers (a new one starts over).
    number: HpNumber,
    shown: u16,
    /// 0 normal, 1 dropping, 2 rising.
    color: u8,
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
        self.shared.tick(b);
        if crate::custom::screens_open(b) {
            self.form_face_shown = crate::custom::face(b, b.setup.local_side as usize).is_some();
        }
        (self.was_over, self.gauge_was_on) = (self.is_over, self.gauge_is_on);
        if let Some(n) = waiting_ticks(b) {
            self.frame = (n & 0x3F) as u8;
        } else if gauge_shown(b, self) && b.gauge_for(b.setup.local_side) >= FULL && !b.late_turns() {
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
                if m.now.full_synchro && m.now.base_variant {
                    // The second set's Full Synchro face (EXE5's Hub
                    // Style's, picture 14, 0x08019614) is among the faces
                    // that blink back to
                    // the picture before and among those that flash white,
                    // and each takes a tick off the blink: six ticks,
                    // white and the picture before by turns.
                    m.flash = on;
                    m.white = m.blink & 2 != 0;
                    m.blink = m.blink.saturating_sub(1);
                } else if m.now.full_synchro {
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
            let s = self.hp.get_or_insert(RollingHp { shown: h, color: 0, hold: 0 });
            if h > s.shown {
                s.color = 1;
                s.shown = roll(s.shown, h, 4);
                s.hold = 15;
            } else if h < s.shown {
                s.color = 2;
                s.shown = roll(s.shown, h, 4);
                s.hold = 15;
            } else {
                if s.hold > 0 {
                    s.hold -= 1;
                }
                if s.hold == 0 {
                    s.color = if h <= m / 4 { 2 } else { 0 };
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
                _ => shown.insert(HpNumberShown { number, shown: number.hp, color: 0, timer: 0 }),
            };
            let o = b.objects.get(number.object);
            let poisoned = o.collision.is_some_and(|c| b.collision.get(c).standing_count != 0);
            if e.shown < o.hp {
                e.color = 2;
                e.timer = 10;
                e.shown = roll(e.shown, o.hp, 2);
            } else if e.shown > o.hp {
                e.color = 1;
                e.timer = 1;
                e.shown = roll(e.shown, o.hp, 2);
            } else if poisoned {
                e.color = 1;
                e.timer = 2;
            } else if e.timer > 0 {
                e.timer -= 1;
                if e.timer == 0 {
                    e.color = 0;
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

/// The custom gauge's frame: from column 6 of the top two rows, 18 columns
/// wide.
const GAUGE_COLUMN: i32 = 6;
const GAUGE_COLUMNS: i32 = 18;
/// Where the custom gauge is drawn, in pixels (x, y, width, height): its
/// frame's tiles.
pub const GAUGE_AREA: (i32, i32, i32, i32) = (GAUGE_COLUMN * 8, 0, GAUGE_COLUMNS * 8, 16);

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
        && !transform_hides(b, state).1
}

/// The gauge counter's units a pixel of its bar: its 0x4000 over the
/// bar's 16 cells of 8 pixels (a cell's tile fills by eighths: `draw`).
const GAUGE_UNITS_A_PIXEL: u32 = (FULL as u32) / (16 * 8);

/// Where the custom gauge's fill ends past its whole pixels, in sixteenths
/// of a pixel (0: on a whole pixel, full, or not drawn). The original draws
/// the fill to the counter's whole pixels, a cell's tile by eighths, so it
/// grows a pixel every 2, 4 or 8 ticks (the counter's 0x40, 0x20 or 0x10 a
/// tick: 128 a pixel); the enhanced mode's motion shows the counter's part
/// of a pixel at the output's resolution (`Frame::nudge`).
pub fn gauge_sub(b: &Battle, state: &HudState, custom_gauge: bool) -> u8 {
    let g = b.gauge_for(b.setup.local_side) as u32;
    if !custom_gauge || !gauge_shown(b, state) || g >= FULL as u32 {
        return 0;
    }
    (g % GAUGE_UNITS_A_PIXEL * 16 / GAUGE_UNITS_A_PIXEL) as u8
}

/// The round's result has started: the fight's win, loss or draw past its
/// first tick, which hides the HUD's parts (`sub_80081A4`, `sub_800825A`
/// and `sub_80082DC`: `sub_801DACC(0xE4C53)`; EXE4's 0x080071A6, 0x0800725C
/// and 0x080072FA with 0x33449A), the defensive chips' marks among them;
/// the damage judge before it hides none of them.
fn result_started(b: &Battle) -> bool {
    b.round.top == top::END
        || b.round.mode == mode::FADE_OUT
        || (b.round.mode == mode::FIGHTING && matches!(b.fight.state, fight::WIN | fight::LOSE | fight::DRAW) && b.fight.init != 0)
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
/// from the tick the fight resumes when the screen showed the form's face
/// in the window: closing takes the window down with it (`sub_802A0F8`;
/// EXE5's close has no such step, its soul choice shows no face).
fn transform_hides(b: &Battle, state: &HudState) -> (bool, bool) {
    let chose = b.transform_requests[b.setup.local_side as usize & 1].form.is_some() && state.form_face_shown;
    match b.transform_seq.state {
        SequencerState::Transform { phase: TransformPhase::FadeOut, started } => (started || chose, false),
        SequencerState::Transform { .. } => (true, true),
        _ => (chose && b.round.mode == mode::FIGHTING && b.round.init == 0, false),
    }
}

/// Draw the HUD layer and queue the HUD sprites. What the HUD needs and
/// the pack or the content doesn't have (a chip's icon, a character of its
/// name, a banner's glyphs) goes to `problems`. Without
/// `custom_gauge`, the custom gauge (`Renderer::custom_gauge`). `smooth`
/// is the enhanced mode's motion: the gauge's fill drawn between whole
/// pixels.
#[allow(clippy::too_many_arguments)]
pub fn draw<'a>(
    b: &Battle,
    assets: &'a Bundle,
    packs: &crate::packs::Packs<'a>,
    state: &HudState,
    shared: &'a SharedGlyphs,
    custom_gauge: bool,
    smooth: Smooth,
    layer: &mut Layer,
    list: &mut SpriteList<'a>,
    text: &mut TextSink,
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
    let color = state.hp.map(|h| h.color).unwrap_or(0) as usize;

    let (hide_mugshot, hide_boxes) = transform_hides(b, state);
    // HP box, top left (moved right with the custom screen's window).
    let shift = crate::custom::hud_shift(b);
    if let Some(r) = player.filter(|_| !hide_boxes && !b.hud_hidden.hp_box) {
        let shown = state.hp.map(|h| h.shown).unwrap_or(b.objects.get(r).hp);
        let pal = &hud.hp_palettes[color.min(2)];
        for (i, &e) in hud.hp_box.iter().enumerate() {
            put_px(layer, hud, pal, e, shift + 8 * (i as i32 % 6), 8 * (i as i32 / 6));
        }
        // In the enhanced mode the HP in the numbers' font over the cells
        // left blank (the box's glyph 10).
        let lettered = hp_box_number(hud, pal, shown, (shift + 8, 0), text, problems);
        for (k, d) in digits4(shown).into_iter().enumerate() {
            let d = if lettered.is_some() { HP_BLANK } else { d };
            let top = MapEntry { tile: hud.first_tile + 2 * d as u16, hflip: false, vflip: false, palette: 13 };
            put_px(layer, hud, pal, top, shift + 8 + 8 * k as i32, 0);
            put_px(layer, hud, pal, MapEntry { tile: top.tile + 1, ..top }, shift + 8 + 8 * k as i32, 8);
        }
        for item in lettered.into_iter().flatten() {
            text.push(Plane::Hud, item);
        }
    }

    // Custom gauge, top center.
    if custom_gauge && gauge_shown(b, state) {
        let pal = &hud.gauge_palette;
        // In the enhanced mode the frame's word in the lettering font, in
        // its look, its cells with their letters taken off (their plate).
        let word = gauge_word(b, hud, text, problems);
        for (i, &e) in hud.gauge_frame.iter().enumerate() {
            let (col, row) = (GAUGE_COLUMN + (i as i32 % GAUGE_COLUMNS), i as i32 / GAUGE_COLUMNS);
            match &word {
                Some((cells, erased, _)) if cells.contains(&i) => {
                    let k = cells.iter().position(|&c| c == i).unwrap_or(0);
                    if let Some(t) = erased.get(k) {
                        layer.draw_tile(t, pal, col * 8, row * 8, e.hflip, e.vflip);
                    }
                }
                _ => put(layer, hud, pal, e, col, row),
            }
        }
        if let Some((_, _, item)) = word {
            text.push(Plane::Hud, item);
        }
        // (The local console's: EXE4's keeps it full until its own send.)
        let g = b.gauge_for(b.setup.local_side);
        let g0 = hud.gauge_first_tile;
        let cell = |tile: u16| MapEntry { tile, hflip: false, vflip: false, palette: 9 };
        // (Its fill a pixel longer, for the output to show the part of a
        // pixel the counter is past its whole pixels: `gauge_sub`. A gauge
        // a pixel short of full shows its cells all filled, not full.)
        let fill = if smooth.nudged && g < FULL { g as u32 + GAUGE_UNITS_A_PIXEL } else { g as u32 };
        for i in 0..16u32 {
            let tile = if g >= FULL {
                g0 + GAUGE_FULL + ((state.frame as u16 / 7) & 3)
            } else if i < fill >> 10 {
                g0 + GAUGE_FILLED
            } else if i == fill >> 10 {
                g0 + ((fill >> 7) & 7) as u16
            } else {
                g0
            };
            put(layer, hud, pal, cell(tile), 7 + i as i32, 1);
        }
        if g >= FULL {
            let frame = (state.frame & 8 != 0) as usize;
            let first = g0 + GAUGE_L_OR_R + 4 * frame as u16;
            // In the enhanced mode the cells with their letters taken off
            // (the gauge's own ground), and the words in the lettering font
            // in this frame's look.
            let found = hud.letters.words.l_or_r[frame].as_ref();
            let words = lettered_word(b, text, "gauge.l-or-r", found.map(|f| &f.letters), pal, Lettering::wide, problems);
            match words.zip(found) {
                Some(((words, look), found)) => {
                    let erased = &found.blank;
                    for i in 0..4 {
                        if let Some(t) = erased.get(i) {
                            layer.draw_tile(t, pal, (13 + i as i32) * 8, 8, false, false);
                        }
                    }
                    // (A ring mark is drawn about the capitals, 1.15 times
                    // their height: the picture's ring is the first letter.)
                    let look = ringed(look, words);
                    text.push(Plane::Hud, look.in_place(words, (13 * 8, 8), 8));
                }
                None => {
                    for i in 0..4 {
                        put(layer, hud, pal, cell(first + i), 13 + i as i32, 1);
                    }
                }
            }
        }
    }

    // "Cstmzing...": once the local player's result is sent, while waiting
    // for the opponent's; it blinks every 32 frames.
    // (Two rows as wide as its words: eight tiles in English, seven in
    // Japanese, where `sub_801CA34` copies one column fewer.)
    if waiting_ticks(b).is_some_and(|n| (n / 32) % 2 == 0) {
        let columns = (hud.waiting.len() / 2).max(1);
        // In the enhanced mode its words in the lettering font, in its look.
        let words = lettered_word(b, text, "waiting", hud.letters.words.waiting.as_ref(), &hud.waiting_palette, Lettering::display, problems);
        let lettered = words.is_some();
        if let Some((words, look)) = words {
            text.push(Plane::Hud, look.in_place(words, (22 * 8, 4 * 8), 16));
        }
        for i in (0..2 * columns).filter(|_| !lettered) {
            if let Some(t) = hud.waiting.get(i) {
                layer.draw_tile(t, &hud.waiting_palette, (22 + (i % columns) as i32) * 8, (4 + (i / columns) as i32) * 8, false, false);
            }
        }
    }

    // "????" beside the mugshot while the local side has a defensive chip
    // set (a trap, a barrier chip's record), and at the right edge while
    // the other side has (`sub_801C984`, `sub_801C9A4`: four '?' of the
    // HUD layer's tiles). The custom screen's opening takes them off
    // (`sub_801DACC(0x400)`), and they are back with the gauge, once the
    // local result is sent. A message takes them off too, for as long as it
    // shows (`sub_801E1E4`: "COUNTER HIT!" is laid from column 7, over the
    // local side's; the same routine in EXE5), and for good where its end
    // finds the battle over (`sub_801E1A4`: `Battle::hud_hidden`). The
    // round's result takes them off with the gauge (`result_started`).
    // (Their tiles keep the HUD's plain colors while the HP box's turn with
    // the HP: `byte_801D964`'s entries, laid once.)
    let marks_hidden = b.message.is_some() || b.hud_hidden.defensive_marks || result_started(b);
    if !open && !hide_boxes && !crate::custom::gauge_held(b) && !marks_hidden {
        let pal = &hud.hp_palettes[0];
        for (side, column) in [(local, 6), (local ^ 1, 26)] {
            if b.linked[side as usize & 1].chip.is_none() {
                continue;
            }
            // In the enhanced mode the marks as a telop draws a hidden chip's
            // name: the text layer's cell role in the palette's face and
            // shadow (the user: "the ???? trap font should use the telop
            // font").
            if text.takes("????") {
                text.push(Plane::Hud, TextItem::new("????", Role::Cell, Rect::new(8 * column, 16, 32, 16), pal[1], Some(pal[2])));
                continue;
            }
            for i in 0..8i32 {
                let e = MapEntry { tile: hud.first_tile + 2 * HIDDEN_GLYPH as u16 + (i / 4) as u16, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, column + i % 4, 2 + i / 4);
            }
        }
    }

    // The judge's numbers under its banner: the damage each side dealt,
    // the local side's on the left, around "VS" (`sub_801D048`: the HUD
    // layer's damage digits, the left number ending at column 13, the
    // right one starting at column 17), until the banner slides out.
    // (The banner's layout is its pack's, by its number there; the
    // banner's lookup is the banner's own, below.)
    // (The pack says when: EXE4's only from the banner's hold, until its
    // release slides it out.)
    let judge_shown = if hud.layout.judge_from_hold { b.banner.step == 4 } else { b.banner.step < 8 };
    if b.banner_for(local).is_some_and(|id| crate::lookups::is_judge(packs, &b.content, id)) && judge_shown {
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
                let e = MapEntry { tile: hud.first_tile + 2 * (DAMAGE_DIGIT as u16 + (d - b'0') as u16) + half, hflip: false, vflip: false, palette: 13 };
                put(layer, hud, pal, e, col, 5 + half as i32);
            }
        };
        let (left, right) = (left.min(9999).to_string(), right.min(9999).to_string());
        // In the enhanced mode the numbers in the numbers' font, in the
        // digits' look, a digit a cell: the left one ending where the
        // original's does, the right one starting where it does.
        let set = text
            .draws_numbers()
            .then(|| crate::lookups::letters(hud.letters.damage_digits.as_ref(), || "the damage digits".into(), problems))
            .flatten()
            .map(|d| crate::numbers::NumberSet::of(d, pal));
        let lettered = set.and_then(|set| {
            let mut items = crate::numbers::items(text, &[(left.clone(), &set)], (8 * (13 - left.len() as i32), 5 * 8))?;
            items.extend(crate::numbers::items(text, &[(right.clone(), &set)], (17 * 8, 5 * 8))?);
            Some(items)
        });
        match lettered {
            Some(items) => items.into_iter().for_each(|item| text.push(Plane::Hud, item)),
            None => {
                for (i, d) in left.bytes().enumerate() {
                    digit(layer, d, 13 - left.len() as i32 + i as i32);
                }
                for (i, d) in right.bytes().enumerate() {
                    digit(layer, d, 17 + i as i32);
                }
            }
        }
        let (vs, _) = fonts::cell_glyphs(hud, "VS");
        fonts::layer_text(text, Plane::Hud, layer, hud, "VS", &vs, vs.len(), pal, (14 * 8, 5 * 8), (Role::Cell, Align::Left));
    }

    // The HUD's text lines, in the HP box's palette as the chip's name is.
    // The turn timer's seconds (from the 15th turn of a netbattle), then
    // "TIME UP!", at the top (`sub_801E398`: 8 glyphs from column 11),
    // while the fight runs.
    // "TIME UP!" stays for the judge's first second.
    let text_palette = &hud.hp_palettes[color.min(2)];
    let timed = match b.fight.state {
        fight::FIGHTING | fight::PAUSE => b.fight.turn_timer != TURN_TICKS,
        fight::JUDGE => b.fight.sub == 0,
        _ => false,
    };
    // The timer's text and a message's are rendered into one line of tiles,
    // the timer's eight glyphs over the message's first eight
    // (`sub_801E398` and `sub_801E270`, both to 0x6009560; EXE4's
    // 0x08016362 and 0x080162CC to 0x06009580): the timer's every tick the
    // fight runs from the 15th turn (`sub_800A1AA`'s caller; EXE4's
    // 0x08008094), the message's as it starts, later in its tick. So on a
    // message's first frame the timer's row reads the message's first
    // eight glyphs, and from the next the message's row starts with the
    // timer's ("    1    HIT!", "TIME UP! HIT!"). (The original's tiles:
    // the enhanced mode's lines stay whole.)
    let timer_line = TEXT_TIME_UP + (b.fight.turn_timer / 60) as usize;
    let late = b.late_turns() && b.round.mode == mode::FIGHTING;
    let one_line = late && text.mode == GraphicsMode::Original;
    let message = b.message.map(|m| {
        let line = match m.message {
            nettai_battle::hud::Message::CounterHit => TEXT_COUNTER_HIT,
        };
        (line, m.ticks == nettai_battle::hud::MessageLine::SHOWN_TICKS - 1)
    });
    if late && timed {
        let line = match message {
            Some((line, true)) if one_line => line,
            _ => timer_line,
        };
        draw_text(layer, text, hud, text_palette, line, (11, 0, 8), problems);
    }
    // The custom screen's time limit (nettai's own, not the original's:
    // `RoundSetup::custom_screen_limit`): the seconds left on the local
    // player's screen, the whole time it counts (`time_left`).
    if let Some(left) = b.custom_time_left(local) {
        time_left(layer, text, hud, left);
    }
    // The message under the gauge (`sub_801E270`: 17 glyphs from column 7;
    // the pack's place).
    if let Some((line, first)) = message {
        let (column, row, width) = hud.layout.message;
        let over = (one_line && !first).then_some(timer_line);
        draw_text_under(layer, text, hud, text_palette, line, over, (column as i32, row as i32, width as usize), problems);
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
            let end = draw_chip_name(b, layer, text, hud, &hud.hp_palettes[color.min(2)], hand, chip, (bonus, doubled), problems);
            // (Shown by the screen's close, before the navi's first
            // decision, where the caller asks.)
            const AT_CLOSE: &str = "chip-window-at-close";
            if b.chip_hud_for(local).from_close && problems.wants(AT_CLOSE) {
                problems.mark([0, 18 * 8, end, 20 * 8], AT_CLOSE.to_string());
            }
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
        // local navi's status is reset (`sub_80144C0`), unless its form's
        // reset puts it back (EXE5's SearchSoul, 0x08011C0C: the chip HUD's
        // `opponent`).
        let link = b.setup.settings.effects & nettai_battle::setup::effects::LINK != 0;
        let shows_opponent = b.chip_hud_for(local).opponent;
        for side in 0..2u8 {
            if let Some(r) = b.player(side).filter(|_| side == local || !link || shows_opponent) {
                icon_parts(b, packs, r, &view, list, problems);
            }
        }
    }
    // The HP numbers, each place in turn (`sub_801C202`), but none while
    // the console's navi is blind: its visibility routine (`sub_8016934`,
    // EXE4's 0x08010430) shows their draw task each tick and ends it while
    // the navi is blind (`sub_801DA48`, `sub_801DACC` with task 2; EXE6's
    // and EXE5's only while the navi has an action, as a fighting navi
    // has). The places stay, and their numbers roll on (the update task).
    let blind = b
        .player(local)
        .and_then(|p| b.objects.get(p).collision)
        .is_some_and(|c| b.collision.get(c).f1 & nettai_battle::collision::f1::BLIND != 0);
    for e in state.hp_numbers.iter().flatten().filter(|_| !blind) {
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
        // place without centering (flag 8).
        let value = if number.damage { o.max_hp.wrapping_sub(e.shown) } else { e.shown };
        let n = value.min(9999).to_string().len() as i32;
        let x = p.x + number.dx as i32 + if number.damage { 0 } else { 4 * n - 32 };
        let y = p.y + number.dy as i32;
        let digits = &hud.enemy_digits[e.color.min(2) as usize];
        let fine = project_hud_fine((o.pos.x, o.pos.y, o.pos.z), &view);
        // In the enhanced mode the number (the HP, or the damage taken) in
        // the numbers' font, in the digits' look, a digit a cell where the
        // original's are, with blank parts in their places.
        let words = value.min(9999).to_string();
        let first = 4 - words.len() as i32;
        let found = text.draws_numbers().then(|| hud.letters.enemy_digits[e.color.min(2) as usize].as_ref()).flatten();
        let lettered = found
            .map(|d| crate::numbers::NumberSet::of(d, &hud.enemy_palette))
            .and_then(|set| crate::numbers::items(text, &[(words, &set)], (x + 8 * first, y)));
        let mut group = Vec::new();
        for (k, d) in digits4(value).into_iter().enumerate() {
            if d == 10 {
                continue;
            }
            let (tiles, d) = if lettered.is_some() { (&*BLANK_GLYPH, 0) } else { (digits, d) };
            group.push(glyph(tiles, d, hud.enemy_palette, x + 8 * k as i32, y, hud.layout.hp_number_priority, None));
        }
        let tag = lettered.as_ref().map(|_| text.tag());
        list.insert_placed(FIELD_LAYER, HP_BUCKET, group, tag, (fine.x, fine.y));
        for item in lettered.into_iter().flatten() {
            // (Its true place between the frame's pixels, where the
            // enhanced mode's motion draws the digits there.)
            let item = if smooth.on { TextItem { fine: (item.fine.0 + fine.x, item.fine.1 + fine.y), ..item } } else { item };
            text.push(Plane::Sprite(tag.unwrap_or_default()), item);
        }
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
        let mut count = mugshot_parts(b, hud, packs, state, r, shift, (shared, text), &mut group, problems);
        if window.flickers != 0 && (flicker + 1) & 2 != 0 {
            for part in &mut group {
                part.palette = [0; 16];
            }
            // (The count blacks out with the window.)
            if let Some(f) = count.as_mut().and_then(|c| c.fitted.as_mut()) {
                f.fill.iter_mut().for_each(|c| *c = 0);
                f.rim = f.rim.map(|_| 0);
                f.shadow = f.shadow.map(|_| 0);
            }
        }
        match count {
            Some(item) => {
                let tag = text.tag();
                list.insert_tagged(FRONT_LAYER, 0, group, Some(tag));
                text.push(Plane::Sprite(tag), item);
            }
            None => list.insert_at(FRONT_LAYER, 0, group),
        }
    }
    // The banner block's banner, where no telop lays on it. (The banner's
    // own pack's HUD draws it, by its number there.)
    let banner = b.banner_for(local);
    let telops = b.telops_for(local);
    if let (Some(id), None) = (banner, telops[0]) {
        let mut group = Vec::new();
        let mut words = Vec::new();
        if let Some((h, layout, number)) = crate::lookups::banner(packs, &b.content, id, problems) {
            // In the enhanced mode its words in the lettering font over its
            // bar, where the strings have them; blank parts stand in for
            // its glyphs. (Not where a telop's load changed its glyphs or
            // its palette: `TelopTiles`, EXE4's.)
            let clean = shared.banner.is_empty() && shared.banner_palette.is_none();
            words = clean.then(|| lettered_banner(b, h, layout, number, id, text, problems)).flatten().unwrap_or_default();
            banner_parts(b, h, layout, number, !words.is_empty(), &mut group);
            if words.is_empty() {
                // (The glyphs and the palette a telop loaded over the
                // banner's: `TelopTiles`.)
                let at = if layout.number_at.is_some() { 2 } else { 0 };
                shared.patch(&mut group, &shared.banner, at, 20);
                if let Some(p) = shared.banner_palette {
                    group.iter_mut().for_each(|part| part.palette = p);
                }
            }
        }
        insert_named(list, text, 0, group, words);
    }
    // The telops: the banner block's, and the other player's on its own
    // block where telops run apart from the banners (beside a banner, then).
    // (A telop shows whether or not its banner has an id: a game whose telop
    // has no banner of its own, EXE4's, draws it in its HUD's own look.)
    for (telop, block) in telops.into_iter().flatten() {
        let mut group = Vec::new();
        let style = match (&hud.layout.telop, banner) {
            (Some(look), _) => Some(TelopStyle::own(hud, look, telop.remote)),
            (None, Some(id)) => crate::lookups::telop(packs, &b.content, id, problems).map(|(h, layout, _)| TelopStyle::banner(h, layout)),
            (None, None) => {
                crate::lookups::telop_without_look(problems);
                None
            }
        };
        let style = style.map(|s| shared.telop_style(s));
        let named = style.map(|style| telop_parts(b, style, telop, &block, &mut group, text, problems));
        let laid = named.unwrap_or_default();
        if telop.remote {
            shared.patch(&mut group, &shared.right, 0, laid.glyphs);
        }
        shared.patch_signs(&mut group, laid.signs);
        insert_named(list, text, NAME_BUCKET, group, laid.name);
    }
    if let Some(used) = b.used_chip_for(local) {
        let mut group = Vec::new();
        // (In the place of the other player's telop: the game's own, or its
        // telop banner's.)
        let style = match &hud.layout.telop {
            Some(look) => Some(TelopStyle::own(hud, look, true)),
            None => match b.roles().try_banner(nettai_battle::content::BannerRole::TelopRemote) {
                Some(remote) => {
                    crate::lookups::telop(packs, &b.content, remote, problems).map(|(_, layout, _)| TelopStyle::banner(hud, layout))
                }
                None => {
                    crate::lookups::telop_without_look(problems);
                    None
                }
            },
        };
        if let Some(style) = style.map(|s| shared.telop_style(s)) {
            let laid = used_chip_parts(b, style, used, &mut group, text, problems);
            shared.patch(&mut group, &shared.right, 0, laid.glyphs);
            shared.patch_signs(&mut group, laid.signs);
            insert_named(list, text, NAME_BUCKET, group, laid.name);
        }
    }
    // "PAUSE" in the middle while a player holds the battle (`sub_801C9E4`:
    // a 32x16 and an 8x16 sprite at its place, EXE6's (100, 63), in the
    // opponents' HP digits' palette).
    if b.round.mode == mode::FIGHTING && b.fight.state == fight::PAUSE {
        let (x, y) = (hud.layout.pause.0 as i32, hud.layout.pause.1 as i32);
        // In the enhanced mode its word in the lettering font, in its look,
        // with blank parts in its glyphs' places.
        let word = lettered_word(b, text, "pause", hud.letters.pause.as_ref(), &hud.enemy_palette, Lettering::display, problems)
            .map(|(words, look)| look.in_place(words, (x, y), 16));
        let tiles = if word.is_some() { &*BLANK_GLYPH } else { &hud.pause };
        let group = (0..5).map(|k| glyph(tiles, if word.is_some() { 0 } else { k }, hud.enemy_palette, x + 8 * k as i32, y, 0, None)).collect();
        insert_named(list, text, NAME_BUCKET, group, word);
    }
    warning_parts(b, hud, &view, list, problems);
}

/// Queue a front-layer group; one whose words are on the text layer
/// (`items`: a chip's name, its numbers, a banner's words) is tagged, so
/// the items are as deep as its parts.
fn insert_named<'a>(list: &mut SpriteList<'a>, text: &mut TextSink, bucket: usize, group: Vec<SpritePart<'a>>, items: impl IntoIterator<Item = TextItem>) {
    let mut items = items.into_iter().peekable();
    if items.peek().is_none() {
        return list.insert_at(FRONT_LAYER, bucket, group);
    }
    let tag = text.tag();
    list.insert_tagged(FRONT_LAYER, bucket, group, Some(tag));
    for item in items {
        text.push(Plane::Sprite(tag), item);
    }
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
        let ((x, y), fine) = match w.at {
            None => (GAUGE_WARNING, (0, 0)),
            Some(at) => {
                let p = project_hud((at.x, at.y, at.z), view);
                let f = project_hud_fine((at.x, at.y, at.z), view);
                ((p.x, p.y), (f.x, f.y))
            }
        };
        if !(0..0xFF - 16).contains(&x) || !(0..0xB0 - 16).contains(&y) {
            continue;
        }
        if !crate::lookups::warning(hud, problems) {
            continue;
        }
        let frame = if b.consoles[console].frames & 8 != 0 { 4 } else { 0 };
        let part = SpritePart { first_tile: frame, priority: 0, ..block(&hud.warning, 16, 16, hud.warning_palette, x, y) };
        list.insert_placed(FRONT_LAYER, 0, vec![part], None, fine);
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
    // (The gauge's tiles follow the HUD layer's in EXE6's and EXE5's VRAM,
    // and come before them in EXE4's: a tile is the block's it falls in.)
    let tile = e
        .tile
        .checked_sub(hud.gauge_first_tile)
        .and_then(|i| hud.gauge_tiles.get(i as usize))
        .or_else(|| e.tile.checked_sub(hud.first_tile).and_then(|i| hud.tiles.get(i as usize)));
    if let Some(t) = tile {
        layer.draw_tile(t, pal, x, y, e.hflip, e.vflip);
    }
}

/// A chip's name in the font's glyphs: its display text (the content's
/// own, or the player's language's: `DisplayText::chip_name`), written in the
/// characters the pack's font has (`sub_8027D10`'s text for the chip, at
/// most eight glyphs).
fn name_glyphs(b: &Battle, hud: &Hud, name: &str, chip: ChipHandle, problems: &mut Problems) -> Vec<u16> {
    let mut glyphs = crate::lookups::chip_name(hud, &b.content, chip, name, problems);
    glyphs.truncate(8);
    glyphs
}

/// Whether a chip's damage shows after its name.
fn shows_damage(b: &Battle, chip: ChipHandle) -> bool {
    b.content.chip(chip).flags.0 & ChipFlags::HAS_DAMAGE != 0
}

#[allow(clippy::too_many_arguments)]
fn draw_chip_name(
    b: &Battle,
    layer: &mut Layer,
    text: &mut TextSink,
    hud: &Hud,
    pal: &Palette,
    hand: &nettai_battle::hand::ChipHand,
    chip: ChipHandle,
    (bonus, doubled): (u16, bool),
    problems: &mut Problems,
) -> i32 {
    let words = text.strings.chip_name(&b.content, chip);
    let name = name_glyphs(b, hud, words, chip, problems);
    // (The text layer's box: the cells its glyphs take, or a character a
    // cell, eight at most, where the pack's font lacks the language's.)
    let cells = if text.takes(words) { fonts::box_cells(hud, words, name.len(), name.len().max(8)) } else { name.len() };
    fonts::layer_text(text, Plane::Hud, layer, hud, words, &name, cells, pal, (0, 18 * 8), (Role::FittedCell, Align::Left));
    // The damage follows the name: after the cells its glyphs take, or in
    // the enhanced mode a pixel after the name as the text layer draws it.
    let mut x = 8 * cells as i32;
    if text.takes(words)
        && let Some(w) = text.fitted_width(words, Role::FittedCell, 8 * cells as i32)
    {
        x = w.ceil() as i32 + 1;
    }
    if !shows_damage(b, chip) {
        return x;
    }
    let i = hand.cursor as usize;
    let damage = hand.damage.get(i).copied().unwrap_or(0);
    // In the enhanced mode the numbers in the lettering font, in their
    // glyphs' looks, from where the original's start.
    if let Some(looks) = NumberSets::of(hud_number_letters(hud), pal, text)
        && let Some((items, end)) = looks.items(text, (damage, bonus, doubled), (x, 18 * 8))
    {
        for item in items {
            text.push(Plane::Hud, item);
        }
        return end;
    }
    let number = |layer: &mut Layer, v: u16, x: &mut i32| {
        for c in v.to_string().bytes() {
            let d = (c - b'0') as u16;
            for half in 0..2u16 {
                let e = MapEntry { tile: hud.first_tile + 2 * (DAMAGE_DIGIT as u16 + d) + half, hflip: false, vflip: false, palette: 13 };
                put_px(layer, hud, pal, e, *x, (18 + half as i32) * 8);
            }
            *x += 8;
        }
    };
    number(layer, damage, &mut x);
    if bonus != 0 {
        for half in 0..2u16 {
            let e = MapEntry { tile: hud.first_tile + 2 * PLUS_GLYPH as u16 + half, hflip: false, vflip: false, palette: 13 };
            put_px(layer, hud, pal, e, x, (18 + half as i32) * 8);
        }
        x += 8;
        number(layer, bonus, &mut x);
    }
    // "x2" while the use would double it (two glyphs: `TIMES_GLYPH`).
    if doubled {
        for k in 0..2u16 {
            for half in 0..2u16 {
                let e = MapEntry { tile: hud.first_tile + 2 * (TIMES_GLYPH as u16 + k) + half, hflip: false, vflip: false, palette: 13 };
                put_px(layer, hud, pal, e, x + 8 * k as i32, (18 + half as i32) * 8);
            }
        }
        x += 16;
    }
    x
}

/// The pack's text lines (`Hud::texts`): "TIME UP!", then the seconds 1-10;
/// and "COUNTER HIT!".
pub const TEXT_TIME_UP: usize = 3;
/// The turn timer's start: it shows once it has counted.
const TURN_TICKS: u16 = 0xA5 * 4 - 1;
pub const TEXT_COUNTER_HIT: usize = 14;

/// Draw the pack's text line `line` on the HUD layer: up to `width` glyphs
/// from tile column `col`, on tile rows `row` and `row + 1`.
/// The row of the emotion window under the HP box (`mugshot_parts`): the
/// custom screen's time left's (`time_left`).
const EMOTION_WINDOW_Y: i32 = 18;

/// The custom screen's time limit's seconds left (nettai's own; the user:
/// "just put it in a plain box under the name on the right side right
/// aligned", "align it with the emotion window", "maybe no box and use the
/// regular font and don't change the color", "it should have the same
/// offset as the enemy name"): the seconds of `left` (in ticks) in the 8x16
/// font, in the HP box's plain colors, set as the custom screen's enemy name
/// is (`custom::draw_names`: its cells ending at the screen's right edge, the
/// text layer's words from their left), on the emotion window's row: under
/// the opponent's name, level with the local player's emotion window.
fn time_left(layer: &mut Layer, text: &mut TextSink, hud: &Hud, left: u16) {
    let seconds = left.div_ceil(60).to_string();
    let (glyphs, _) = fonts::cell_glyphs(hud, &seconds);
    let cells = if text.takes(&seconds) { fonts::box_cells(hud, &seconds, glyphs.len(), 4) } else { glyphs.len() };
    let x = crate::compose::WIDTH as i32 - 8 * cells as i32;
    let pal = &hud.hp_palettes[0];
    fonts::layer_text(text, Plane::Hud, layer, hud, &seconds, &glyphs, cells, pal, (x, EMOTION_WINDOW_Y), (Role::FittedCell, Align::Left));
}

fn draw_text(
    layer: &mut Layer,
    text: &mut TextSink,
    hud: &Hud,
    pal: &Palette,
    line: usize,
    (col, row, width): (i32, i32, usize),
    problems: &mut Problems,
) {
    let Some(glyphs) = crate::lookups::text_line(hud, line, problems) else { return };
    let shown = &glyphs[..glyphs.len().min(width)];
    fonts::layer_line(text, Plane::Hud, layer, hud, shown, pal, (col * 8, row * 8));
}

/// [`draw_text`], with the pack's text line `over` (if any) in the place
/// of the line's first glyphs: the lines' tiles are one line's (the turn
/// timer's eight glyphs, rendered over a message's).
#[allow(clippy::too_many_arguments)]
fn draw_text_under(
    layer: &mut Layer,
    text: &mut TextSink,
    hud: &Hud,
    pal: &Palette,
    line: usize,
    over: Option<usize>,
    (col, row, width): (i32, i32, usize),
    problems: &mut Problems,
) {
    let Some(glyphs) = crate::lookups::text_line(hud, line, problems) else { return };
    let over = over.and_then(|o| crate::lookups::text_line(hud, o, problems));
    let shown = line_under(glyphs, over.unwrap_or(&[]), width);
    fonts::layer_line(text, Plane::Hud, layer, hud, &shown, pal, (col * 8, row * 8));
}

/// The first `width` glyphs of a line whose first glyphs the line `over`
/// was rendered over.
fn line_under(glyphs: &[u16], over: &[u16], width: usize) -> Vec<u16> {
    let mut shown = glyphs[..glyphs.len().min(width)].to_vec();
    for (cell, &g) in shown.iter_mut().zip(over) {
        *cell = g;
    }
    shown
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
    packs: &crate::packs::Packs<'a>,
    state: &HudState,
    r: ObjectRef,
    x: i32,
    (shared, text): (&'a SharedGlyphs, &TextSink),
    out: &mut Vec<SpritePart<'a>>,
    problems: &mut Problems,
) -> Option<TextItem> {
    let side = b.objects.get(r).alliance as usize;
    let stats = &b.stats[side];
    // A link navi's own face, its definition's (MegaMan's, the navi that
    // changes form, is his form's): in its second palette in Full Synchro.
    let navi = b.content.navi(stats.navi);
    if !navi.changes_form() {
        // (The mugshot's own pack's HUD holds its picture.)
        let Some((tiles, palettes, _)) = crate::lookups::navi_face(packs, &b.content, stats.navi, problems) else { return None };
        let full_synchro = emotion_role(b, side as u8) == Some(EmotionRole::FullSynchro);
        let pal = window_faded(b, palettes.get(full_synchro as usize).or(palettes.first()).copied().unwrap_or_default());
        out.push(block(tiles, 32, 16, pal, x, EMOTION_WINDOW_Y));
        out.push(block(&hud.navi_box, 16, 16, pal, x + 32, EMOTION_WINDOW_Y));
        note_true_face(b, packs, side, navi.version.as_deref(), x, problems);
        return None;
    }
    let mut face = state.mood.map(|m| m.shown()).unwrap_or_else(|| Face::of(b, r));
    // A form chosen on the custom screen shows in the window until the
    // screens close (`sub_802A040`, `sub_802A088`).
    if let Some(form) = crate::custom::face(b, side) {
        face = Face::in_form(b, r, form);
    }
    let (picture, Some((gfx, palettes))) = crate::lookups::form_face(
        packs,
        &b.content,
        face.form,
        face.emotion,
        face.base_variant || nettai_battle::kinds::player::face_charged(b, side as u8),
        problems,
    ) else { return None };
    // (The white of a change to Full Synchro: `byte_801CD80`.)
    let pal = if state.mood.is_some_and(|m| m.white) { [0x7FFF; 16] } else { palettes.first().copied().unwrap_or_default() };
    let pal = window_faded(b, pal);
    out.push(block(gfx, 32, 16, pal, x, EMOTION_WINDOW_Y));
    // The box beside it: the face's own (EXE5's), a count, or the box
    // without one. A game whose faces bring their boxes has no box
    // without a count: a face that brings none (EXE5's souls') shows its
    // side's count (`window_count`).
    let face_hud = picture.map_or(hud, |m| packs.mugshot(m).0);
    let (tiles, count) = match picture.and_then(|m| face_hud.mugshot_box(m.id)) {
        Some(own) => (Some(own), None),
        None if face_hud.count_box.is_empty() => {
            let n = window_count(b, side as u8).map(|n| n as usize);
            (n.and_then(|n| face_hud.counts.get(n)), n.filter(|&n| n < face_hud.counts.len()).map(|n| (face_hud, n)))
        }
        None if b.looks[side & 1].window_count => {
            let n = face.count as usize;
            (hud.counts.get(n).or(Some(&hud.count_box)), (n < hud.counts.len()).then_some((hud, n)))
        }
        None => (Some(&hud.count_box), None),
    };
    // In the enhanced mode a count in the numbers' font on the empty box
    // (the counts' own box, their digits taken off: `count_blank`; the HUD's
    // box without a count is another picture).
    let number = count.and_then(|(h, n)| count_number(&h.letters.counts, n, &pal, (x + 32, EMOTION_WINDOW_Y), text));
    let tiles = match (&number, tiles) {
        (Some(_), Some(_)) => Some(&shared.count_blank),
        (_, tiles) => tiles,
    };
    if let Some(tiles) = tiles {
        out.push(block(tiles, 16, 16, pal, x + 32, EMOTION_WINDOW_Y));
    }
    note_true_face(b, packs, side, b.content.form(face.form).version.as_deref(), x, problems);
    number
}

/// The emotion window's count `n` in the numbers' font, if the text layer
/// draws numbers and the pack has its digits' places (`CountLetters`: each
/// a single digit's box, the union of theirs; a count of two digits, its
/// own two): in their colors in `palette`, on the box at (x, y).
fn count_number(counts: &nettai_assets::letters::CountLetters, n: usize, palette: &Palette, (x, y): (i32, i32), text: &TextSink) -> Option<TextItem> {
    let chars = n.to_string();
    if !text.draws_numbers() || !crate::numbers::font().covers(&chars) {
        return None;
    }
    let d = counts.counts.get(n)?.as_ref()?;
    if d.glyphs.len() != chars.chars().count() {
        return None;
    }
    let color = |c: u8| palette[c as usize & 15];
    let glyphs = chars.chars().zip(&d.glyphs).map(|(c, b)| (c, b.map(|v| v as f32))).collect();
    let fitted = crate::numbers::Fitted { glyphs, set: crate::numbers::DIGITS.into(), family: Vec::new(), fill: d.fill.iter().map(|&c| color(c)).collect(), rim: d.rim.map(color), shadow: d.shadow.map(color) };
    Some(TextItem::fitted(fitted, Rect::new(x, y, 16, 16)))
}

/// The emotion window's palette (sprite palette 12: `byte_30016D0`) as
/// the custom screen's second fade record leaves it: a dark chip's hover
/// darkens sprite palettes 10-13 (`custom::window_fade`), the window's face
/// among them.
fn window_faded(b: &Battle, palette: Palette) -> Palette {
    match crate::custom::window_fade(b) {
        Some(f) => palette.map(|c| crate::compose::apply_fade(c, f)),
        None => palette,
    }
}

/// The count a side's emotion window shows beside a face that brings no
/// box, in a game whose faces bring their own (EXE5's souls': the turns
/// left, AIData +0x0F): the turns left in the form the side's button that
/// offers a form gave (`Battle::form_turns`), if it has such a button.
fn window_count(b: &Battle, side: u8) -> Option<u8> {
    b.form_turns(side)
}

/// A console shows every form's and navi's true face, where the original
/// has none for another version's (a form or link navi that says its
/// `version`: `owner`, its faces being that version's ROM's own) and shows
/// its own counterpart's (deliberately: docs/frontend.md §5): the face and
/// the box beside it, in the face's palette, are a known difference.
fn note_true_face(b: &Battle, packs: &crate::packs::Packs, side: usize, owner: Option<&str>, x: i32, problems: &mut Problems) {
    let console = crate::custom::console_version(b, packs, b.setup.local_side);
    if owner.is_some_and(|v| v != console) && side == b.setup.local_side as usize & 1 {
        problems.known(x, 18, 48, 16, "the other game's face on this console (the true face)");
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
    packs: &crate::packs::Packs<'a>,
    r: ObjectRef,
    view: &View,
    list: &mut SpriteList<'a>,
    problems: &mut Problems,
) {
    let o = b.objects.get(r);
    if b.game_rules().effects.chip_icons == ChipIcons::NaviOffset {
        return own_icon_parts(b, packs, r, view, list, problems);
    }
    let vanished = o.actor.is_some_and(|a| b.actors.get(a).status & status::VANISHED != 0);
    if o.chips_held == 0 || vanished || !b.chip_hud_for(o.alliance).icons {
        return;
    }
    let hand = &b.hands[o.alliance as usize];
    let Some(chip) = hand.ids.get(hand.cursor as usize).copied().flatten() else { return };
    // A chip's icon is its game's pack's image under the chip's key, in
    // that HUD's icon palette; the other player's in a link battle outside
    // battle mode 1, the hidden chip's (EXE5's 0x08018B48: its entries of
    // the remote navi).
    let link = b.setup.settings.effects & nettai_battle::setup::effects::LINK != 0;
    let hidden = link && o.alliance != b.setup.local_side && b.round.mode_copy != 1;
    let (tiles, icon_palette) = if hidden {
        let hud = &packs.game(&b.content).hud;
        (&hud.hidden_icon, &hud.icon_palette)
    } else {
        let Some(icon) = crate::lookups::chip_icon(packs, &b.content, chip, problems) else { return };
        icon
    };
    let p = project_hud((o.pos.x, o.pos.y, o.pos.z), view);
    if !on_screen(p) {
        return;
    }
    let fine = project_hud_fine((o.pos.x, o.pos.y, o.pos.z), view);
    let fine = (fine.x, fine.y);
    // (The console's own direction: `object_getAllianceDirection` of the
    // local side, whichever navi the icons are over.)
    let a = if b.setup.local_side & 1 == 0 { 1 } else { -1 };
    let f = nettai_battle::kinds::common::facing(o.alliance, o.flip);
    // Attach point 3 of the navi's sprite (player NameIDs 0x1A0..=0x1C3).
    let (ax, ay) = if b.content.identity(o.identity).class.is_player() {
        let Some(p) = b.content.try_attach_point(o.identity, 3) else {
            problems.note(format!("identity {} has no attach point 3: its chip icons aren't drawn", b.content.identity(o.identity).key));
            return;
        };
        (p.x as i32, p.y as i32)
    } else {
        (8, 48)
    };
    let (x0, y0) = (p.x + a * (ax * f - 1) - 8, p.y - ay - 8);
    // The first icon is the front one; each next is a bucket back.
    let count = o.chips_held.min(6) as i32;
    // (The other version's chip's icon is its own ROM's: the stack's reach
    // is a known difference.)
    if !hidden && crate::lookups::other_versions_icon(packs, b, chip) {
        let back = 2 * (count - 1);
        let x = if a * f > 0 { x0 - back } else { x0 };
        problems.known(x, y0 - back, 16 + back, 16 + back, crate::lookups::OTHER_VERSIONS_ART);
    }
    // (The stack's reach, where the caller asks for the chip's icon.)
    if !hidden && !problems.marking.is_empty() {
        let what = crate::lookups::chip_icon_mark(&b.content, chip);
        if problems.wants(&what) {
            let back = 2 * (count - 1);
            let x = if a * f > 0 { x0 - back } else { x0 };
            problems.mark([x, y0 - back, x + 16 + back, y0 + 16], what);
        }
    }
    for k in 0..count {
        let icon = block(tiles, 16, 16, *icon_palette, x0 - 2 * k * a * f, y0 - 2 * k);
        list.insert_placed(FIELD_LAYER, ICON_BUCKET + (count - k) as usize, vec![icon], None, fine);
    }
}

/// The chip icons over the local navi where each chip shows its own
/// (`ChipIcons::NaviOffset`: EXE4's 0x08014860 places them and loads each
/// icon, 0x08015000 draws them): while the icons show, at the navi's
/// identity's `chip_icons_at` from its place on the screen (none while
/// its place is off it), each chip in the hand from the next one on by
/// its own icon, the next one in front and each after it two pixels up
/// and two left, a bucket further back. A next chip that shows no icon
/// there (its trait `no_icon_when_next`: EXE4's souls' chips, 0x0801502E)
/// leaves the icons to the chips after it.
fn own_icon_parts<'a>(
    b: &Battle,
    packs: &crate::packs::Packs<'a>,
    r: ObjectRef,
    view: &View,
    list: &mut SpriteList<'a>,
    problems: &mut Problems,
) {
    let o = b.objects.get(r);
    let side = o.alliance;
    if side != b.setup.local_side || !b.chip_hud_for(side).icons {
        return;
    }
    let p = project_hud((o.pos.x, o.pos.y, o.pos.z), view);
    if !on_screen(p) {
        return;
    }
    let fine = project_hud_fine((o.pos.x, o.pos.y, o.pos.z), view);
    let fine = (fine.x, fine.y);
    let identity = b.content.identity(o.identity);
    let Some((dx, dy)) = identity.chip_icons_at else {
        problems.note(format!("identity {} has no `chip_icons_at`: its chip icons aren't drawn", identity.key));
        return;
    };
    let hand = &b.hands[side as usize];
    let mut chips: Vec<ChipHandle> = hand.ids.iter().skip(hand.cursor as usize).map_while(|c| *c).collect();
    if chips.first().is_some_and(|&c| b.content.chip(c).traits.has(nettai_battle::content::ChipTraits::NO_ICON_WHEN_NEXT)) {
        chips.remove(0);
    }
    let count = chips.len();
    let (x0, y0) = (p.x + dx as i32, p.y + dy as i32);
    for (k, &chip) in chips.iter().enumerate() {
        let Some((tiles, palette)) = crate::lookups::chip_icon(packs, &b.content, chip, problems) else { continue };
        let (x, y) = (x0 - 2 * k as i32, y0 - 2 * k as i32);
        if crate::lookups::other_versions_icon(packs, b, chip) {
            problems.known(x, y, 16, 16, crate::lookups::OTHER_VERSIONS_ART);
        }
        if !problems.marking.is_empty() {
            let what = crate::lookups::chip_icon_mark(&b.content, chip);
            if problems.wants(&what) {
                problems.mark([x, y, x + 16, y + 16], what);
            }
        }
        let icon = SpritePart { priority: OWN_ICON_PRIORITY, ..block(tiles, 16, 16, *palette, x, y) };
        list.insert_placed(FIELD_LAYER, count - k, vec![icon], None, fine);
    }
}

/// The priority of `own_icon_parts`' sprites (EXE4's 0x08015000: 1).
const OWN_ICON_PRIORITY: u8 = 1;

/// The banner's five 32x16 sprites (as 8x16 glyphs) with its vertical
/// squash: grow over 5 frames, hold, shrink (`sub_801CE28`).
/// `layout` is banner `id`'s (its number in `hud`'s pack).
/// `blank`: blank parts in the glyphs' places, as many (the enhanced mode's
/// lettering draws the words: `lettered_banner`).
fn banner_parts<'a>(b: &Battle, hud: &'a Hud, layout: &'a nettai_assets::BannerLayout, id: u8, blank: bool, out: &mut Vec<SpritePart<'a>>) {
    let vscale = Some(banner_scale(&b.banner));
    let pal = hud.banner_palette;
    let tiles = |own: &'a Tiles| if blank { &*BLANK_GLYPH } else { own };
    let k_of = |k: usize| if blank { 0 } else { k };
    for k in 0..20usize {
        out.push(glyph(tiles(&layout.glyphs), k_of(k), pal, layout.x as i32 + 8 * k as i32, layout.y as i32, 0, vscale));
    }
    if let Some((nx, ny)) = layout.number_at {
        let n = banner_number(b, id);
        let (tens, ones) = (n / 10 % 10, n % 10);
        let (tens, nx) = if tens == 0 { (10, nx as i32 - 4) } else { (tens, nx as i32) };
        // The number is drawn in front of the text.
        out.insert(0, glyph(tiles(&hud.banner_digits), k_of(ones), pal, nx + 8, ny as i32, 0, vscale));
        out.insert(0, glyph(tiles(&hud.banner_digits), k_of(tens), pal, nx, ny as i32, 0, vscale));
    }
}

/// The HP box's blank cell: glyph 10 of its digits' (`digits4`'s blank).
const HP_BLANK: usize = Hud::HP_BLANK;

/// The HP box's number in the enhanced mode, if the lettering font draws
/// it: right-aligned in the box's four cells from (x, y), each digit in its
/// cell, in the numbers' font in the look of the box's digits (glyphs 0 to
/// 9 of the HUD layer's tiles, on the colors of the blank cell, glyph 10).
fn hp_box_number(hud: &Hud, palette: &Palette, shown: u16, (x, y): (i32, i32), text: &TextSink, problems: &mut Problems) -> Option<Vec<TextItem>> {
    if !text.draws_numbers() {
        return None;
    }
    let digits = crate::lookups::letters(hud.letters.hp_digits.as_ref(), || "the HP box's digits".into(), problems)?;
    let set = crate::numbers::NumberSet::of(digits, palette);
    let words = shown.min(9999).to_string();
    let cells = words.len() as i32;
    crate::numbers::items(text, &[(words, &set)], (x + crate::numbers::CELL * (4 - cells), y))
}

/// The custom gauge's frame's word in the enhanced mode, if the strings and
/// a font have it: the cells of the frame's top row that draw it (the pack's
/// `GaugeWord`), their tiles with the letters taken off (their plate's
/// color), and the word laid out as text where the cells' letters are, in
/// their look (the user: "the text above the custom gauge CUSTOM looks
/// really bad too"): in the numbers' font, as the window's labels are (the
/// lettering font where it lacks a character), its capitals as high as the
/// picture's letters' ink, at the font's own spacing, narrowed as a whole
/// where wider than their columns, standing as they stand in the cells.
fn gauge_word(b: &Battle, hud: &Hud, text: &TextSink, problems: &mut Problems) -> Option<(Vec<usize>, Tiles, TextItem)> {
    let words = text.strings.lettering(&b.content, "gauge.custom")?;
    let Some(font) = [GAUGE_WORD_FONT, LetteringFont::Display].into_iter().find(|&f| text.takes_lettering_in(words, f)) else {
        text.lettering_in(words, LetteringFont::Display, || "the picture of \"gauge.custom\"".into(), problems);
        return None;
    };
    let found = crate::lookups::letters(hud.letters.words.gauge.as_ref(), || "the picture of \"gauge.custom\"".into(), problems)?;
    let cells: Vec<usize> = found.cells.iter().map(|&c| c as usize).collect();
    let x = (GAUGE_COLUMN + *cells.first()? as i32) * 8;
    let l = &found.letters;
    let lettering = Lettering { font, ..Lettering::wide(l.top, l.cap) };
    let look = crate::lettering::Look { role: Role::Lettering(lettering), ..crate::lettering::Look::of(l, &hud.gauge_palette, Lettering::wide) };
    let [from, to] = l.span.map(i32::from);
    let item = look.item(words, Rect::new(x + from, 0, to - from, 8), crate::custom::align_of(found.align));
    Some((cells, found.blank.clone(), item))
}

/// The font of the gauge's word (CUSTOM): the window's labels'.
const GAUGE_WORD_FONT: LetteringFont = LetteringFont::Numbers;

/// A look whose picture's first letter is a button's ring (Ⓛ) where the
/// words have one the text layer draws (`vfont`'s marks): its capitals
/// made as high as the ring, drawn 1.15 times their height about their
/// middle, needs them, on the same middle.
fn ringed(look: crate::lettering::Look, words: &str) -> crate::lettering::Look {
    let Role::Lettering(l) = look.role else { return look };
    let font = crate::vfont::lettering_font();
    if !words.chars().next().is_some_and(|c| font.draws_ring(c)) {
        return look;
    }
    let cap = (l.cap as f32 / crate::vfont::BUTTON_RING).round().max(1.0) as u8;
    let top = l.top + (l.cap - cap.min(l.cap)) / 2;
    crate::lettering::Look { role: Role::Lettering(Lettering { top, cap, ..l }), ..look }
}

/// A lettered picture's words and their look, where the enhanced mode draws
/// them in its place: the strings' words for it (`lettering`'s `key`) where
/// the lettering font draws them, and the look of the picture's letters
/// (the pack's `letters`; none is a problem) in `palette`.
pub(crate) fn lettered_word<'b>(
    b: &'b Battle,
    text: &TextSink<'b>,
    key: &str,
    letters: Option<&nettai_assets::letters::Letters>,
    palette: &Palette,
    lettering: fn(u8, u8) -> Lettering,
    problems: &mut Problems,
) -> Option<(&'b str, crate::lettering::Look)> {
    let words = text.strings.lettering(&b.content, key)?;
    if !text.lettering_in(words, LetteringFont::Display, || format!("the picture of {key:?}"), problems) {
        return None;
    }
    let letters = crate::lookups::letters(letters, || format!("the picture of {key:?}"), problems)?;
    Some((words, crate::lettering::Look::of(letters, palette, lettering)))
}

/// The number a numbered banner shows (`sub_801CE28`): the turn's on the
/// turn's banner (its pack's number 0x0C), else the round's.
fn banner_number(b: &Battle, id: u8) -> usize {
    (if id == 0x0C { b.round.turn } else { b.round.round }) as usize
}

/// A banner as the enhanced mode draws it, if the content's strings have
/// its words (`lettering`'s `banner.NAME`, the number where the words
/// say `{n}`) and the lettering font draws them: the words in the lettering
/// font, centered on the picture's letters (and its number), in the
/// picture's look (its capitals' rows, its face's colors down them, its
/// rim), over its bar drawn as a shape (read off the picture: none for a
/// picture with no bar), with the banners' squash. `number` is its number
/// in `hud`'s pack.
fn lettered_banner(
    b: &Battle,
    hud: &Hud,
    layout: &nettai_assets::BannerLayout,
    number: u8,
    id: BannerId,
    text: &TextSink,
    problems: &mut Problems,
) -> Option<Vec<TextItem>> {
    let name = crate::packs::name(&b.content, nettai_content_api::AssetKind::Banner, id.0)?;
    let words = text.strings.lettering(&b.content, &format!("banner.{}", nettai_content_api::keys::local(name)))?;
    let what = || format!("banner {name:?}");
    // (By its place in the pack's banners: its number there over 4.)
    let found = crate::lookups::letters(hud.letters.words.banners.get(number as usize / 4).and_then(Option::as_ref), what, problems)?;
    let pal = &hud.banner_palette;
    let letters = &found.letters;
    let (x, y) = (layout.x as i32, layout.y as i32);
    let vscale = Some(banner_scale(&b.banner));
    let (face, shadow) = crate::lettering::face(letters, pal);
    // A run of the words, centered on the picture's letters in its columns
    // `span` (where it has letters there).
    let words_in = |words: &str, span: Option<[i16; 2]>| -> Option<TextItem> {
        let [from, to] = span?.map(i32::from);
        let rect = Rect::new(x + from, y, to - from, 16);
        let item = TextItem::new(words, crate::lettering::role(letters, Lettering::display), rect, face, shadow);
        Some(TextItem { align: Align::Center, vscale, ink: Some(crate::lettering::ink(letters, pal)), ..item })
    };
    let mut items = Vec::new();
    match (layout.number_at, words.split_once("{n}")) {
        // A numbered banner: its words on either side of the number, which
        // is drawn in the numbers' font in the banner digits' look where the
        // picture's number is (`banner_parts`).
        (Some((nx, ny)), Some((before, after))) => {
            let n = banner_number(b, number);
            let (tens, nx) = if n / 10 % 10 == 0 { (None, nx as i32 - 4) } else { (Some(n / 10 % 10), nx as i32) };
            let digits = tens.map_or(format!("{}", n % 10), |t| format!("{t}{}", n % 10));
            let (before, after) = (before.trim_end(), after.trim_start());
            if ![before, after, &digits].iter().all(|w| w.is_empty() || text.lettering_in(w, LetteringFont::Display, what, problems)) {
                return None;
            }
            let set = crate::lookups::letters(hud.letters.banner_digits.as_ref(), || "the banners' digits".into(), problems)?;
            let set = crate::numbers::NumberSet::of(set, pal);
            let at = if tens.is_some() { nx } else { nx + 8 };
            let number = crate::numbers::items(text, &[(digits, &set)], (at, ny as i32))?;
            // (Its words either side of where the number's picture is: the
            // pack's columns of the letters there, for one digit or two.)
            let [before_span, after_span] = if tens.is_some() { found.two_digits? } else { found.one_digit? };
            items.extend((!before.is_empty()).then(|| words_in(before, before_span)).flatten());
            items.extend((!after.is_empty()).then(|| words_in(after, after_span)).flatten());
            items.extend(number.into_iter().map(|item| TextItem { vscale, ..item }));
        }
        _ => {
            let words = words.replace("{n}", &banner_number(b, number).to_string());
            if !text.lettering_in(&words, LetteringFont::Display, what, problems) {
                return None;
            }
            items.extend(words_in(&words, Some(letters.span)));
        }
    }
    // Every item cut to the banner's box.
    items.first()?;
    for item in &mut items {
        item.clip = Rect::new(x, 0, 160, crate::compose::HEIGHT as i32);
    }
    Some(items)
}

/// A banner's vertical scale this frame (the texture rows step by this
/// over 256 per screen row): sliding in, from 0xC0 at its first tick to
/// 0x40 (whole) at its last, a line, and back so sliding out (EXE6's
/// `sub_801CE28`: 0xE0 less 0x20 a tick, 5 ticks; EXE4's 0x08014994: 0xD0
/// less 0x10, 9 ticks); holding, whole, but for a game's bounce (EXE6's:
/// 0x34 and 0x38 at its hold's first two ticks and last two). `block`: the
/// block it runs on (a telop's may be the other player's).
fn banner_scale(block: &Banner) -> i32 {
    let t = block.timer as i32;
    let steps = block.steps;
    let slope = |ticks: Option<u8>| 0x80 / (ticks.unwrap_or(5) as i32 - 1).max(1);
    let scale = match block.step {
        0 => {
            let n = steps.map_or(5, |s| s.slide_in as i32);
            0x40 + slope(steps.map(|s| s.slide_in)) * (n - t)
        }
        4 if steps.is_none_or(|s| s.bounces) => {
            let last = steps.map_or(0x30, |s| s.hold as i32);
            match t {
                _ if t == 1 || t == last - 1 => 0x34,
                _ if t == 2 || t == last - 2 => 0x38,
                _ => 0x40,
            }
        }
        4 => 0x40,
        _ => 0x40 + slope(steps.map(|s| s.slide_out)) * t,
    };
    scale * 4
}

/// How a telop is laid out and drawn: the place its glyphs are centered
/// from (as fifteen glyphs), its palette, the HUD whose font writes the
/// chip's name, and the glyphs of its numbers (the digits from `digit`,
/// '+', the two of "x2").
#[derive(Clone, Copy)]
struct TelopStyle<'a> {
    hud: &'a Hud,
    at: (u8, u8),
    palette: Palette,
    numbers: &'a Tiles,
    digit: usize,
    plus: usize,
    times: usize,
    /// The looks of those glyphs (the pack's letters).
    letters: Option<[&'a Digits; 3]>,
}

impl<'a> TelopStyle<'a> {
    /// A telop banner's (EXE6's `sub_801E95C`): at the banner's place, in
    /// the HP box's palette, with the HUD layer's damage digits and signs.
    fn banner(hud: &'a Hud, layout: &nettai_assets::BannerLayout) -> TelopStyle<'a> {
        TelopStyle {
            hud,
            at: (layout.x, layout.y),
            palette: hud.hp_palettes[0],
            numbers: &hud.tiles,
            digit: DAMAGE_DIGIT,
            plus: PLUS_GLYPH,
            times: TIMES_GLYPH,
            letters: hud_number_letters(hud),
        }
    }

    /// The game's own (`HudLayout::telop`: EXE4's 0x0801650C), on its
    /// user's console or on the other player's (`remote`).
    fn own(hud: &'a Hud, look: &'a TelopLook, remote: bool) -> TelopStyle<'a> {
        TelopStyle {
            hud,
            at: look.places[remote as usize],
            palette: look.palette,
            numbers: &look.glyphs,
            digit: 0,
            plus: TelopLook::PLUS,
            times: TelopLook::TIMES,
            letters: hud.letters.telop.as_ref().map(|[d, p, t]| [d, p, t]),
        }
    }
}

/// A telop (`sub_801E95C` lays it out, `sub_801CF9E` draws it): the chip's
/// name in the font's glyphs, then for a chip whose damage shows the
/// damage, "+bonus" and "x2", centered in the viewer's half of the screen,
/// with the banners' squash, its block's (`block`).
fn telop_parts<'a>(
    b: &Battle,
    style: TelopStyle<'a>,
    telop: ShownTelop,
    block: &Banner,
    out: &mut Vec<SpritePart<'a>>,
    text: &TextSink,
    problems: &mut Problems,
) -> Laid {
    let hud = style.hud;
    let name = match telop.name {
        TelopName::Chip(chip) => {
            let words = text.strings.chip_name(&b.content, chip);
            (words, name_glyphs(b, hud, words, chip, problems))
        }
        TelopName::Hidden => ("????", fonts::cell_glyphs(hud, "????").0),
        TelopName::Unknown => {
            crate::lookups::telop_unknown(problems);
            ("", Vec::new())
        }
    };
    let numbers = (telop.damage, telop.bonus, telop.doubled);
    name_parts(style, name, numbers, telop.remote, Some(banner_scale(block)), out, text)
}

/// The chip the other player just used (`sub_801EB18` lays it out as a
/// telop on the right, `sub_801D1F6` draws it; EXE4's 0x080164B4, a banner
/// of the second block shown without sliding): its name and numbers,
/// without the banners' squash, in the other player's telop's place.
fn used_chip_parts<'a>(
    b: &Battle,
    style: TelopStyle<'a>,
    used: nettai_battle::hud::UsedChip,
    out: &mut Vec<SpritePart<'a>>,
    text: &TextSink,
    problems: &mut Problems,
) -> Laid {
    let words = text.strings.chip_name(&b.content, used.chip);
    let name = (words, name_glyphs(b, style.hud, words, used.chip, problems));
    name_parts(style, name, (used.damage, used.bonus, used.doubled), true, None, out, text)
}

/// Two blank tiles: the part that stands in for a glyph the text layer
/// draws, so the sprite limit counts the parts the original's glyphs take.
static BLANK_GLYPH: std::sync::LazyLock<Tiles> = std::sync::LazyLock::new(|| Tiles { pixels: vec![0; 2 * Tiles::TILE] });

/// A chip's name (its words and the font's glyphs for them) with its
/// numbers (damage, bonus, doubled) as `sub_801E95C` lays them out from a
/// telop's place (`style`): centered as fifteen glyphs are; the other
/// player's (`remote`) moves over for the "x2". In the enhanced mode the name
/// is a text item in the cells its glyphs take, returned (`Laid`), and
/// blank parts stand in for the glyphs.
#[allow(clippy::too_many_arguments)]
fn name_parts<'a>(
    style: TelopStyle<'a>,
    (words, name): (&str, Vec<u16>),
    (number, bonus_value, doubled): (u16, u16, bool),
    remote: bool,
    vscale: Option<i32>,
    out: &mut Vec<SpritePart<'a>>,
    text: &TextSink,
) -> Laid {
    let hud = style.hud;
    // Glyph d of the damage digits, '+' before a bonus.
    let digits = |v: u16| -> Vec<usize> { v.to_string().bytes().map(|c| style.digit + (c - b'0') as usize).collect() };
    let (damage, bonus, doubled) = match number {
        0 => (Vec::new(), Vec::new(), false),
        d => (digits(d), if bonus_value != 0 { std::iter::once(style.plus).chain(digits(bonus_value)).collect() } else { Vec::new() }, doubled),
    };
    let font = !words.is_empty() && text.takes(words);
    let cells = if font { fonts::box_cells(hud, words, name.len(), 8) } else { name.len() };
    let width = (cells + damage.len() + bonus.len()) as i32;
    let mut x = (style.at.0 as i32 + (15 - width) * 4) & 0xFF;
    if remote && doubled {
        x = (x - 16) & 0xFF;
    }
    let y = style.at.1 as i32;
    let pal = style.palette;
    let mut items: Vec<TextItem> = font
        .then(|| {
            let item = TextItem::new(words, Role::Cell, Rect::new(x, y, 8 * cells as i32, 16), pal[1], Some(pal[2]));
            TextItem { align: Align::Center, vscale, ..item }
        })
        .into_iter()
        .collect();
    let after = x + 8 * cells as i32;
    let glyphs = if font { 0 } else { name.len() };
    for c in name {
        let (tiles, first) = if font { (&*BLANK_GLYPH, 0) } else { fonts::cell_glyph(hud, c) };
        out.push(SpritePart { first_tile: first, ..glyph(tiles, 0, pal, x, y, 0, vscale) });
        x += 8;
    }
    x = after;
    // In the enhanced mode the numbers in the numbers' font, in their
    // glyphs' looks, with blank parts in their glyphs' places. With the
    // name on the text layer they follow it a pixel after its words, as the
    // hand window's do, and the two are centered where the original's name
    // and numbers are (its name's glyphs fill their cells; the text layer's
    // words, narrower, centered in them, stood apart from the numbers).
    let sets = (!damage.is_empty()).then(|| NumberSets::of(style.letters, &pal, text)).flatten();
    let mut at = after;
    if let (Some(item), true) = (items.first_mut(), sets.is_some())
        && let Some(w) = text.fitted_width(words, Role::Cell, 8 * cells as i32)
    {
        let shown = w.ceil() as i32 + 1 + 8 * (damage.len() + bonus.len()) as i32;
        let start = item.rect.x + (8 * width - shown) / 2;
        *item = TextItem { align: Align::Left, rect: Rect { x: start, ..item.rect }, ..item.clone() };
        at = start + w.ceil() as i32 + 1;
    }
    let numbers = sets.and_then(|looks| looks.items(text, (number, bonus_value, doubled), (at, y)));
    let lettered = numbers.is_some();
    if let Some((numbers, _)) = numbers {
        items.extend(numbers.into_iter().map(|item| TextItem { vscale, ..item }));
    }
    let numbers = |g: usize| if lettered { (&*BLANK_GLYPH, 0) } else { (style.numbers, g) };
    for g in damage.into_iter().chain(bonus) {
        let (tiles, g) = numbers(g);
        out.push(glyph(tiles, g, pal, x, y, 0, vscale));
        x += 8;
    }
    let signs = (doubled && !lettered).then_some(out.len());
    if doubled {
        let ((tiles, times), (_, two)) = (numbers(style.times), numbers(style.times + 1));
        out.push(glyph(tiles, times, pal, x, y, 0, vscale));
        out.push(glyph(tiles, two, pal, x + 8, y, 0, vscale));
    }
    Laid { name: items, glyphs, signs }
}

/// The looks of a chip's damage, its bonus's '+' and the "x2" of a doubled
/// chip, from the glyphs that draw them in the original (the HUD layer's,
/// a telop's own: `numbers::NumberSet`), for the enhanced mode's numbers:
/// none in the original mode, or where the glyphs have no ink.
pub(crate) struct NumberSets {
    digits: crate::numbers::NumberSet,
    plus: crate::numbers::NumberSet,
    times: crate::numbers::NumberSet,
}

impl NumberSets {
    /// The digits', the '+''s and the "x2"'s looks (the pack's, read off
    /// their glyphs), in `palette`.
    pub(crate) fn of(letters: Option<[&Digits; 3]>, palette: &Palette, text: &TextSink) -> Option<NumberSets> {
        if !text.draws_numbers() {
            return None;
        }
        let [digits, plus, times] = letters?.map(|d| crate::numbers::NumberSet::of(d, palette));
        Some(NumberSets { digits, plus: plus.with_set("+"), times: times.with_set(&format!("{}2", crate::numbers::times())) })
    }

    /// A chip's damage, then "+bonus" (where it has one) and "x2" (where
    /// the use doubles it), in the numbers' font from `x` on cells' rows `y`,
    /// a character a cell as the original's glyphs stand, and where they end.
    pub(crate) fn items(&self, text: &TextSink, (damage, bonus, doubled): (u16, u16, bool), (x, y): (i32, i32)) -> Option<(Vec<TextItem>, i32)> {
        let mut parts = vec![(damage.to_string(), &self.digits)];
        if bonus != 0 {
            parts.push(("+".to_string(), &self.plus));
            parts.push((bonus.to_string(), &self.digits));
        }
        if doubled {
            parts.push((format!("{}2", crate::numbers::times()), &self.times));
        }
        let cells: i32 = parts.iter().map(|p| p.0.chars().count() as i32).sum();
        Some((crate::numbers::items(text, &parts, (x, y))?, x + crate::numbers::CELL * cells))
    }
}

/// A telop as `name_parts` laid it out: the text layer's item for its name
/// (the enhanced mode's), how many of its parts, from the first, are the
/// name's glyphs (none in the enhanced mode), and where among them the '×' of
/// a doubled chip is (the '2' after it).
#[derive(Default)]
struct Laid {
    name: Vec<TextItem>,
    glyphs: usize,
    signs: Option<usize>,
}

/// Glyphs of the HUD layer's tiles (tile `first_tile + 2k`; EXE6's from
/// 0x1A0, EXE5's from 0x180): the HP digits (from the first), the damage
/// digits (EXE6's tiles 0x1B8..), the '?' of a defensive chip's "????"
/// (0x1CC), '+' (0x1CE), and the 'x' and '2' of a doubled chip (0x1D2,
/// 0x1D4).
const DAMAGE_DIGIT: usize = Hud::DAMAGE_DIGIT;
const HIDDEN_GLYPH: usize = Hud::HIDDEN_GLYPH;
const PLUS_GLYPH: usize = Hud::PLUS_GLYPH;
const TIMES_GLYPH: usize = Hud::TIMES_GLYPH;

/// The looks of the HUD layer's damage digits, '+' and "x2" (the pack's
/// letters), where it has all three.
fn hud_number_letters(hud: &Hud) -> Option<[&Digits; 3]> {
    let l = &hud.letters;
    Some([l.damage_digits.as_ref()?, l.plus.as_ref()?, l.times.as_ref()?])
}
/// The custom gauge's tiles (from `gauge_first_tile`: EXE6's 0x222, EXE5's
/// 0x202): its cell filling by eighths (from the first), filled, full (four
/// frames), and its "L or R" (two frames of four).
const GAUGE_FILLED: u16 = Hud::GAUGE_FILLED;
const GAUGE_FULL: u16 = Hud::GAUGE_FULL;
const GAUGE_L_OR_R: u16 = Hud::GAUGE_L_OR_R;

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_assets::BannerLayout;

    use nettai_battle::content::{BannerId, testing};
    use nettai_battle::hud::{Telop, TelopHidden};

    const CHARS: &str = " 0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz?";

    /// A HUD whose font draws `CHARS` (glyph k its kth character) and
    /// whose telops sit where EXE6's do.
    fn hud() -> Hud {
        let mut banners = vec![BannerLayout::default(); 21];
        banners[0x4C / 4] = BannerLayout { x: 0, y: 32, kind: 3, ..BannerLayout::default() };
        banners[0x50 / 4] = BannerLayout { x: 120, y: 32, kind: 3, ..BannerLayout::default() };
        let mut hud = Hud {
            tiles: Tiles { pixels: vec![1; 0x36 * Tiles::TILE] },
            first_tile: 0x1A0,
            font: Tiles { pixels: vec![1; 2 * CHARS.len() * Tiles::TILE] },
            font_chars: CHARS.chars().map(String::from).collect(),
            banners,
            ..Hud::default()
        };
        // (Its glyphs' letters and digits, as a pack has them.)
        hud.letters = nettai_content::letters::hud_letters(&hud, &mut |_| {});
        hud
    }

    fn battle() -> Battle {
        Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(100)), testing::content())
    }

    fn glyph_of(c: char) -> usize {
        2 * CHARS.find(c).unwrap()
    }

    /// The turn timer's eight glyphs are rendered over a message's first
    /// eight (EXE4's " COUNTER HIT! " under "    1   ").
    #[test]
    fn the_timers_text_takes_a_messages_first_glyphs() {
        let message: Vec<u16> = (10..24).collect();
        let timer = [0, 0, 0, 0, 2, 0, 0, 0];
        assert_eq!(line_under(&message, &timer, 14), [0, 0, 0, 0, 2, 0, 0, 0, 18, 19, 20, 21, 22, 23]);
        assert_eq!(line_under(&message, &[], 14), message);
        assert_eq!(line_under(&message, &timer, 6), [0, 0, 0, 0, 2, 0], "cut to the message's place");
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
        // Its user's console: "SunGun3" "120" "+10" is 13 glyphs, centered as
        // 15 are, then "x2".
        let mut parts = Vec::new();
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &TextSink::original(), &mut problems);
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
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x50 / 4]), b.telop_for(1).unwrap(), &b.banner, &mut parts, &TextSink::original(), &mut problems);
        assert_eq!(parts[0].x, 120 + 8 - 16);
        assert!(problems.is_empty(), "{:?}", problems.lines());
    }

    /// EXE4's shared tiles: a banner's 40 from 0x360, the right-hand
    /// telop's name from 0x384.
    #[test]
    fn a_banners_tiles_are_its_sprites_rows() {
        // Glyph 0's top and bottom, glyph 4's (the second sprite's first).
        assert_eq!([banner_tile(0), banner_tile(4), banner_tile(8), banner_tile(12)], [0, 1, 8, 9]);
        // Tiles 0x384 to 0x387, the right-hand telop's first two glyphs:
        // the bottoms of the banner's glyphs 16 to 19.
        assert_eq!([36, 37, 38, 39].map(banner_tile), [33, 35, 37, 39]);
    }

    #[test]
    fn the_last_start_owns_the_shared_tiles_and_palette() {
        let mut b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        let used = nettai_battle::hud::UsedChip { chip, damage: 0, doubled: false, bonus: 0, ticks: 33 };
        let local = b.setup.local_side as usize;
        let mut loads = SharedLoads::default();
        // The other player's chip, then a banner over it: the banner's.
        b.used_chips[local ^ 1] = Some(used);
        loads.tick(&b);
        let name = Load::Telop(TelopName::Chip(chip));
        assert_eq!((loads.right, loads.signs, loads.palette), (name, name, name));
        assert!(b.start_banner(BannerId(0x10)));
        loads.tick(&b);
        let banner = Load::Banner(BannerId(0x10));
        assert_eq!((loads.right, loads.signs, loads.palette), (banner, banner, banner));
        // The name runs on (fewer ticks left): no new load.
        b.used_chips[local ^ 1] = Some(nettai_battle::hud::UsedChip { ticks: 32, ..used });
        loads.tick(&b);
        assert_eq!(loads.right, banner);
        // The chip used again: its telop's start loads over the banner's.
        b.used_chips[local ^ 1] = Some(used);
        loads.tick(&b);
        assert_eq!((loads.right, loads.signs, loads.palette), (name, name, name));
        // "PAUSE" loads the palette alone.
        b.round.mode = mode::FIGHTING;
        b.fight.state = fight::PAUSE;
        loads.tick(&b);
        assert_eq!((loads.right, loads.palette), (name, Load::Pause));
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
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x50 / 4]), b.telop_for(1).unwrap(), &b.banner, &mut parts, &TextSink::original(), &mut problems);
        assert_eq!(parts.iter().map(|p| p.first_tile).collect::<Vec<_>>(), [glyph_of('?'); 4]);
        // Four glyphs centered as fifteen are, from the right banner's place.
        assert_eq!(parts[0].x, 120 + 44);
        let mut parts = Vec::new();
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &TextSink::original(), &mut problems);
        assert_eq!(parts.len(), 7, "its user sees the name");
    }

    #[test]
    fn a_font_mode_telop_is_a_text_item_over_blank_parts() {
        let mut b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        assert!(b.start_banner(BannerId(0x4C)));
        b.banner.telop =
            Some(Telop { side: 0, chip: Some(chip), damage: 120, doubled: true, bonus: 10, hidden: TelopHidden::No });
        let hud = hud();
        let font = crate::vfont::VectorFont::bundled();
        let text = TextSink::new(crate::textlayer::GraphicsMode::Enhanced, Some(&font));
        let mut problems = Problems::default();
        let mut parts = Vec::new();
        let item = telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &text, &mut problems).name.into_iter().next().unwrap();
        // The same parts where the original's go (so the sprite limit
        // counts the same), the name's blank; the item in the name's cells.
        let xs: Vec<u16> = parts.iter().map(|p| p.x).collect();
        assert_eq!(xs, (0..15).map(|i| 8 + 8 * i).collect::<Vec<u16>>());
        assert!(parts[..7].iter().all(|p| p.tiles.pixels.iter().all(|&v| v == 0)));
        // (Its numbers in the numbers' font: their parts blank too.)
        assert!(parts[7..].iter().all(|p| p.tiles.pixels.iter().all(|&v| v == 0)));
        assert_eq!((item.text.as_str(), item.rect), ("SunGun3", Rect::new(8, 32, 56, 16)));
        assert_eq!((item.align, item.vscale), (Align::Center, parts[0].vscale));
        assert_eq!((item.face, item.shadow), (hud.hp_palettes[0][1], Some(hud.hp_palettes[0][2])));
        // In the original mode there is none.
        let mut parts = Vec::new();
        assert!(telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &TextSink::original(), &mut problems).name.is_empty());
    }

    #[test]
    fn a_telop_names_the_chip_in_the_players_language() {
        let mut b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        assert!(b.start_banner(BannerId(0x4C)));
        b.banner.telop =
            Some(Telop { side: 0, chip: Some(chip), damage: 0, doubled: false, bonus: 0, hidden: TelopHidden::No });
        let hud = hud();
        let key = b.content.defs.chip(chip).key.clone();
        let table = format!("language = \"xx\"\n[chips]\n\"{key}\" = {{ name = \"Sol\" }}\n");
        let strings = nettai_content::locale::parse(&table, "xx.toml").unwrap();
        let text = TextSink::original().with_language(Some(&strings));
        let mut problems = Problems::default();
        let mut parts = Vec::new();
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &text, &mut problems);
        assert_eq!(parts.iter().map(|p| p.first_tile).collect::<Vec<_>>(), "Sol".chars().map(glyph_of).collect::<Vec<_>>());
        // Centered as fifteen glyphs are: the translation's three.
        assert_eq!(parts[0].x, 6 * 8);
        assert!(text.strings.take_missing().is_empty());
        // A chip the table has no name for shows the definition's, noted.
        let empty = nettai_content::locale::parse("language = \"xx\"\n", "xx.toml").unwrap();
        let text = TextSink::original().with_language(Some(&empty));
        let mut parts = Vec::new();
        telop_parts(&b, TelopStyle::banner(&hud, &hud.banners[0x4C / 4]), b.telop_for(0).unwrap(), &b.banner, &mut parts, &text, &mut problems);
        assert_eq!(parts.len(), 7);
        assert_eq!(text.strings.take_missing(), [format!("chips.{key}.name")]);
    }

    #[test]
    fn a_chip_name_the_font_cannot_write_is_a_problem() {
        let b = battle();
        let chip = testing::chip_in(&b.content, testing::SUN_GUN_3);
        let mut hud = hud();
        hud.font_chars.retain(|c| c != "G");
        let mut problems = Problems::default();
        assert_eq!(name_glyphs(&b, &hud, "SunGun3", chip, &mut problems).len(), 6);
        assert_eq!(problems.len(), 1);
        assert!(problems.lines()[0].contains("no glyph for ['G']"), "{:?}", problems.lines());
    }

    /// EXE4's BATTLE START (its banner 0: twenty 8x16 glyphs at (52, 64)
    /// whose outlines reach the glyphs' first and last rows, as all of
    /// EXE4's banners' do; EXE5's and EXE6's leave those rows clear but
    /// their PROGRAM ADVANCE) in Enhanced, at three and at five times the
    /// frame: on every tick of its 9 in, 30 held and 9 out
    /// (content/exe4/rules/effects.luau), each glyph's overlay draws every
    /// frame pixel the frame's own sprite draws, the first output pixel of
    /// it in the frame's texel, and while the banner is whole, the whole
    /// pixel and nothing else: the last row's outline whole, not cut by the
    /// box.
    #[test]
    fn exe4s_battle_start_overlay_covers_its_sprite_on_every_tick() {
        use crate::compose::{BACKDROP_DEPTH, Fade, Fades, HEIGHT, WIDTH, compose_with_depth, sprite_depth, to_rgb};
        use crate::present::Placement;
        use crate::smooth::{Overlay, draw_overlays};
        use nettai_battle::content::{BannerRelease, BannerSteps};

        // Every texel opaque, colored by its row (rows 0 and 15 alike).
        let mut pixels = vec![0u8; 40 * Tiles::TILE];
        for g in 0..20 {
            for row in 0..16 {
                for col in 0..8 {
                    pixels[(2 * g + row / 8) * Tiles::TILE + (row % 8) * 8 + col] = 1 + (row % 15) as u8;
                }
            }
        }
        let layout = BannerLayout { x: 52, y: 64, kind: 0, glyphs: Tiles { pixels }, number_at: None, bar: None };
        let hud = Hud { banner_palette: std::array::from_fn(|i| i as u16 * 0x0421), ..hud() };
        let steps = BannerSteps { slide_in: 9, hold: 30, slide_out: 9, release: BannerRelease::SlidesOut, bounces: false };
        let mut b = battle();
        assert!(b.banner.start(BannerId(0), false, steps));
        const UNDRAWN: u32 = u32::MAX;
        let mut ticks = 0;
        while b.banner.active {
            let mut parts = Vec::new();
            banner_parts(&b, &hud, &layout, 0, false, &mut parts);
            assert_eq!(parts.len(), 20);
            let whole = banner_scale(&b.banner) == 0x100;
            for part in &parts {
                let (framed, _) = compose_with_depth(0, &[], std::slice::from_ref(part), Fades::default());
                let overlay = Overlay::of(part, sprite_depth(0, 0), [Fade::None; 2], (0, 0)).expect("a squashed part is an overlay");
                for scale in [3, 5] {
                    let (w, h) = (scale * WIDTH, scale * HEIGHT);
                    let mut out = vec![UNDRAWN; w * h];
                    draw_overlays(std::slice::from_ref(&overlay), &vec![BACKDROP_DEPTH; WIDTH * HEIGHT], None, None, &Placement::fit(w, h), &mut out, w);
                    for fy in 0..HEIGHT {
                        for fx in 0..WIDTH {
                            let color = framed[fy * WIDTH + fx];
                            let out = &out;
                            let mut block = (0..scale).flat_map(|j| (0..scale).map(move |i| out[(scale * fy + j) * w + scale * fx + i]));
                            let here = || format!("tick {ticks} (step {}), the glyph at x {}, frame pixel ({fx}, {fy}) at {scale} times", b.banner.step, part.x);
                            if color == 0 {
                                assert!(!whole || block.all(|c| c == UNDRAWN), "{}: drawn where the frame's sprite isn't", here());
                                continue;
                            }
                            assert_eq!(out[scale * fy * w + scale * fx], to_rgb(color), "{}: the frame's texel", here());
                            assert!(!whole || block.all(|c| c == to_rgb(color)), "{}: not all of the frame's pixel", here());
                        }
                    }
                }
            }
            b.banner.tick();
            ticks += 1;
        }
        assert!(ticks >= 9 + 30 + 9, "{ticks} ticks");
    }
}
