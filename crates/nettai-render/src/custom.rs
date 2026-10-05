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
use crate::textlayer::{Align, Plane, Rect, TextItem, TextSink};
use crate::vfont::Role;
use nettai_assets::{Bundle, CustomScreen, Hud, MapEntry, Palette, Picture, Tiles, VersionPictures};
use nettai_battle::{Battle, Content};
use nettai_battle::battle::{FadeMode, mode};
use nettai_battle::content::{ChipFlags, ChipTraits};
use nettai_battle::custom::screen::{HiddenStage, OK_SLOT, SPECIAL_SLOT};
use nettai_battle::custom::{ButtonCell, FolderChip, GameVersion, Phase, Screen, Side, SlotKind, SlotState};
use nettai_content_api::{ChipHandle, FieldValue, FormHandle, NaviHandle};

/// The window: 15 columns of 20 rows at the HUD layer's top left.
const COLUMNS: usize = 15;
const ROWS: usize = 20;
/// Where the screen's tiles go in the HUD layer's character block: the
/// window frame from tile 1, the rest where the pack's game loads them
/// (`CustomScreen::layout`: EXE6's `CustomLayout::EXE6`, the chip window's
/// name from 0x9B, EXE5's from 0x59).
const WINDOW_TILE: u16 = 0x01;
/// A chip picture's tiles (7x6).
const PICTURE_TILES: u16 = 42;
/// The Crosses' names in the Cross window (9x2 each, `byte_8029DF8`,
/// from the layout's `cross_names`).
pub(crate) const CROSS_NAME_TILES: usize = 18;
/// The Cross window's maps: three opening steps, then the window with one
/// to five Crosses.
const CROSS_OPENING_MAPS: usize = 3;
/// The Program Advance animation's names (`sub_802B80C`): 9 cells of the
/// 8x16 font from the chip window's picture's first tile (EXE6's 0xAB), 18
/// tiles a name; a pick's code in its last cell; a name every 3 rows from
/// row 5, a column right of the layer's scroll; the recipe's in palette 10,
/// the others' in 13.
const ADVANCE_NAME_CELLS: usize = 9;
const ADVANCE_FIRST_ROW: i32 = 5;
/// The chips past the table's that the animation shows no code for (EXE6's
/// and EXE5's alike: `sub_802B80C`, 0x08027BC6).
pub(crate) const ADVANCE_NO_CODE_FROM: u16 = 0x160;
const LAYER_TILES: usize = 0x200;
/// The window's background colors: what the original copies over cells
/// the chip window leaves empty (`byte_802A6C0`, `byte_802A680`: solid 8
/// and 7; a hidden slot's, the layout's `slot_blank`).
const BLANK_8: u8 = 8;
const BLANK_7: u8 = 7;
/// The chip window's name: 8 cells, its pixels shifted to the window's
/// colors from 8 (`sub_80284E2`).
const NAME_CELLS: usize = 8;
const NAME_SHIFT: u8 = 8;
/// A code no chip has: the invalid chip's (blank in the slots).
pub(crate) const NO_CODE: u8 = 0x1B;
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
/// Whether this custom mode's screens are open: on the mode's first tick
/// they open (`sub_8026840`); until then a side's screen is the last one.
pub(crate) fn screens_open(b: &Battle) -> bool {
    b.round.mode == mode::CUSTOM && b.round.init != 0
}

pub fn local(b: &Battle) -> Option<(&Side, &Screen)> {
    if !screens_open(b) {
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
    if !screens_open(b) {
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
        Phase::Window { .. } | Phase::Description { window: Some(_), .. } | Phase::Scrapping { .. } | Phase::Redealing { .. }
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

/// The screen fade the local screen runs on the stage and the battle's
/// objects (`off_8006040`): Beast Out's darkens background palettes 0-13
/// (the stage and the HUD layer) and sprite palettes 0-10 half way (modes
/// 0x64 and 0x60); the Program Advance's and a dark chip's background
/// palettes 0-8 and sprite palettes 0-9 (0x14 and 0x10, 0x54 and 0x50); a
/// Cross's choice whitens every palette (4, and 0 back).
pub fn fade(b: &Battle) -> Option<Fade> {
    let (_, s) = local(b)?;
    let f = s.look.fade;
    let (shown, white) = match f.mode {
        FadeMode::BeastOut | FadeMode::ProgramAdvance | FadeMode::DarkChip => (true, false),
        // Back toward clear: once it gets there the original takes the
        // palettes' transform off.
        FadeMode::BeastOutBack | FadeMode::ProgramAdvanceBack | FadeMode::DarkChipBack => (f.active, false),
        FadeMode::EndToWhite => (true, true),
        FadeMode::IntroFromWhite => (f.active, true),
        _ => (false, false),
    };
    let n = (f.level >> 4).min(16) as u8;
    (shown && n > 0).then_some(if white { Fade::White(n) } else { Fade::Black(n) })
}

/// The battle's objects' share of the screen's fade (a white fade is every
/// sprite's: `sprite_fade`).
pub fn object_fade(b: &Battle) -> Option<Fade> {
    fade(b).filter(|f| !matches!(f, Fade::White(_)))
}

/// The screen's fade of every sprite palette: a Cross's choice's white.
pub fn sprite_fade(b: &Battle) -> Option<Fade> {
    fade(b).filter(|f| matches!(f, Fade::White(_)))
}

/// The second fade record's, which only a dark chip's hover runs (modes
/// 0x5C and 0x58): background palettes 9-13 (the window) and sprite
/// palettes 10-13 (the screen's sprites).
pub fn window_fade(b: &Battle) -> Option<Fade> {
    let (_, s) = local(b)?;
    let f = s.look.window_fade;
    let shown = match f.mode {
        FadeMode::DarkChipWindow => true,
        FadeMode::DarkChipWindowBack => f.active,
        _ => false,
    };
    let n = (f.level >> 4).min(16) as u8;
    (shown && n > 0).then_some(Fade::Black(n))
}

/// The HUD layer's fade: Beast Out's and a Cross's (its palettes are among
/// the first record's), else the second record's.
pub fn hud_fade(b: &Battle) -> Option<Fade> {
    let (_, s) = local(b)?;
    match s.look.fade.mode {
        FadeMode::BeastOut | FadeMode::BeastOutBack | FadeMode::EndToWhite | FadeMode::IntroFromWhite => fade(b),
        _ => window_fade(b),
    }
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

    /// Fill `n` tiles from `at` with one color index.
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
    /// The pictures of the Beast the navi goes into (`beast_pictures`):
    /// the console's version's, unless a setup's Cross list put the navi in
    /// the other game's Cross.
    beast: &'a VersionPictures,
    hud: &'a Hud,
    /// Every pack's graphics: a chip's icon and picture are its game's.
    packs: crate::packs::Packs<'a>,
    /// The console's region ("us", "jp": `Renderer::console_region`).
    region: &'a str,
}

/// A system's button as the frontend draws it (docs/design/rules-in-luau.md
/// §4.8: EXE6's, by name): its details picture (in a palette by its state, if
/// it has palettes), its tiles (`count` a state, selectable then
/// unavailable and picked), and the cursor over it.
struct ButtonLook<'a> {
    details: std::borrow::Cow<'a, Picture>,
    /// The details picture's palettes by the slot's state (none: its own).
    palettes: &'a [Palette],
    tiles: &'a Tiles,
    /// Its tiles a state, and which set a state shows.
    count: usize,
    sets: Sets,
    /// The tiles the slots after it start past (none: they overlap it).
    advance: u16,
    cursor: (i32, i32, CursorShape),
    /// Whether the chip window shows its uses left, in the damage's last
    /// cell (EXE5's Shuffle, 0x080245F2; the pack's layout says).
    uses_digit: bool,
    /// Where the icon of the chip it holds is drawn, a sprite over it
    /// (EXE5's Arm Change, 0x080254F4).
    held_at: Option<(i32, i32)>,
}

/// Which of a button's tile sets a slot in a state shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sets {
    /// One a state: selectable, unavailable, picked.
    Each,
    /// The second for unavailable and picked alike (the Beast Out button's
    /// two, EXE5's soul button's).
    Other,
    /// The second for unavailable alone (EXE5's Arm Change, 0x0802415A:
    /// picked, it looks on offer, the chip it holds drawn over it).
    Unavailable,
}

impl Sets {
    fn of(self, state: usize) -> usize {
        match self {
            Sets::Each => state,
            Sets::Other => (state != 0) as usize,
            Sets::Unavailable => (state == 1) as usize,
        }
    }
}

/// EXE5's Arm Change's chip over its button (0x08025508: the sprite's x
/// 0x45, y 0x84).
const HELD_CHIP: (i32, i32) = (0x45, 0x84);

impl ButtonLook<'_> {
    /// Its details picture for a slot in state `state`.
    fn details(&self, state: usize) -> Picture {
        let palette = self.palettes.get(state).or(self.palettes.first()).copied().unwrap_or(self.details.palette);
        Picture { palette, ..self.details.clone().into_owned() }
    }
}

impl<'a> View<'a> {
    /// The look of button `button`, by its name (`named_look`).
    fn button_look(&self, button: nettai_battle::content::ButtonHandle) -> Option<ButtonLook<'a>> {
        self.named_look(self.b.content.defs.button(button).name.as_str())
    }

    /// The look of the button named `name`: the pack's of that name
    /// (`CustomScreen::buttons`: EXE5's "soul", the special slot's under OK,
    /// its picture in a palette by its state), else EXE6's Beast Out (its
    /// game's pictures), ChpShufl re-deal (EXE5's Shuffle) and DustCross
    /// scrap, and EXE5's Arm Change. A name the frontend doesn't know is
    /// drawn as nothing; a button that shows a chip (`Slot::face`: EXE5's
    /// capsules) is drawn as that chip's slot whatever its name.
    fn named_look(&self, name: &str) -> Option<ButtonLook<'a>> {
        use std::borrow::Cow;
        let a = self.assets;
        if let Some(b) = a.button(name) {
            // (EXE5's soul button: gray when unavailable or picked,
            // 0x08024540.)
            let count = b.width as usize * b.height as usize;
            let cursor = cursor_at(&a.layout.special_cursor);
            return Some(ButtonLook {
                details: Cow::Borrowed(&b.picture),
                palettes: &b.palettes,
                tiles: &b.tiles,
                count,
                sets: Sets::Other,
                advance: 0,
                cursor,
                uses_digit: false,
                held_at: None,
            });
        }
        let wide = |details: &'a Picture, tiles: &'a Tiles| ButtonLook {
            details: Cow::Borrowed(details),
            palettes: &[],
            tiles,
            count: 12,
            sets: Sets::Each,
            advance: 12,
            cursor: (0x38, 0x80, BUTTON_CURSOR),
            uses_digit: false,
            held_at: None,
        };
        match name {
            "beast_out" => {
                let beast = self.beast;
                let details = Picture { palette: beast.beast_out_palettes.first().copied().unwrap_or([0; 16]), ..beast.beast_out.clone() };
                Some(ButtonLook {
                    details: Cow::Owned(details),
                    palettes: &[],
                    tiles: &beast.beast_buttons,
                    count: 8,
                    sets: Sets::Other,
                    advance: 0,
                    cursor: cursor_at(&a.layout.special_cursor),
                    uses_digit: false,
                    held_at: None,
                })
            }
            "redeal" => Some(ButtonLook { uses_digit: a.layout.button_uses, ..wide(&a.pictures.redeal, &a.redeal_buttons) }),
            "scrap" => Some(wide(&a.pictures.scrap, &a.scrap_buttons)),
            // EXE5's Arm Change (ColonelSoul's, 0x0802415A and 0x080245A0):
            // the tiles and the picture its pack has in the scrap button's
            // place.
            "arm_change" => {
                Some(ButtonLook { sets: Sets::Unavailable, held_at: Some(HELD_CHIP), ..wide(&a.pictures.scrap, &a.scrap_buttons) })
            }
            _ => None,
        }
    }
}

/// EXE5's soul button's offer and choice, as its souls system keeps them
/// (content/exe5/rules/souls/custom.luau, read by its fields' names): the
/// soul it offers or gave (its form; none: no offer) and whether it is Chaos
/// Unison (slot 11's +5 and +6), and the choice's step and count (the
/// screen's state 9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoulOffer {
    pub soul: Option<FormHandle>,
    pub chaos: bool,
    pub step: u8,
    pub count: u8,
}

/// The system EXE5's soul button and its window are (`SoulOffer`).
const SOULS_SYSTEM: &str = "souls";
/// The soul's choice's window (the screen's state 9, 0x080232D0).
const SOUL_WINDOW: &str = "soul_unison";

impl SoulOffer {
    /// Side `side`'s soul button's, when its ruleset has EXE5's souls system.
    pub fn of(b: &Battle, side: usize) -> Option<SoulOffer> {
        let (schema, state) = b.system_state(side as u8, SOULS_SYSTEM)?;
        let field = |name: &str| Some(state.get(schema, schema.index_of(name)?));
        let byte = |v: Option<FieldValue>| match v {
            Some(FieldValue::U8(n)) => Some(n),
            _ => None,
        };
        let flag = |v: Option<FieldValue>| match v {
            Some(FieldValue::Bool(b)) => Some(b),
            _ => None,
        };
        let soul = match field("offer")? {
            FieldValue::Ref(Some((nettai_content_api::Registry::Form, h))) => Some(FormHandle(h)),
            FieldValue::Ref(None) => None,
            _ => return None,
        };
        Some(SoulOffer {
            soul,
            chaos: flag(field("offer_chaos"))?,
            step: byte(field("unite_step"))?,
            count: byte(field("unite_count"))?,
        })
    }
}

/// Whether slot `slot` of side `side`'s screen is EXE5's soul button.
fn is_soul_button(b: &Battle, screen: &Screen, slot: u8) -> bool {
    matches!(screen.slots[slot as usize].kind, SlotKind::Button { button, .. } if b.content.defs.button(button).name == SOUL_BUTTON)
}

/// Whether the soul's choice is up on `screen` (the souls system's window).
fn soul_window_up(b: &Battle, screen: &Screen) -> bool {
    let Phase::Window { window, .. } = screen.phase else { return false };
    let d = b.content.defs.window(window);
    d.name == SOUL_WINDOW && b.content.defs.system(d.system).key == SOULS_SYSTEM
}

/// The icon of the soul EXE5's soul button offers or gave, if the special
/// slot is the soul button: the pack's `icons` and the icon's first tile in
/// them (the soul's, Chaos Unison's the 13th: 0x0802341C).
fn soul_icon<'a>(a: &'a CustomScreen, v: &View) -> Option<(&'a Tiles, usize)> {
    if !is_soul_button(v.b, v.screen, SPECIAL_SLOT) {
        return None;
    }
    let b = a.button(SOUL_BUTTON)?;
    let soul = SoulOffer::of(v.b, v.side as usize)?;
    let n = if soul.chaos { CHAOS_ICON } else { soul_place(v.b, v.side, soul.soul) };
    (b.icons.len() >= 4 * (n + 1)).then_some((&b.icons, 4 * n))
}

/// The Chaos Unison's icon among the soul button's.
const CHAOS_ICON: usize = 13;

/// A soul's icon among the pack's soul button's: its place among its
/// side's navi's souls, from 1 (the navi's `forms.souls` lists them in the
/// icons' order, the original's soul numbers'); 0, the empty icon, for no
/// soul or one the navi hasn't.
fn soul_place(b: &Battle, side: u8, soul: Option<FormHandle>) -> usize {
    let navi = b.stats[side as usize & 1].navi;
    match (soul, &b.content.navi(navi).forms) {
        (Some(f), Some(forms)) => forms.souls.iter().position(|&s| s == f).map_or(0, |i| i + 1),
        _ => 0,
    }
}

/// EXE5's soul choice (its state 9, 0x080232D0: the souls system's window
/// `soul_unison`, at its step `sub` and count `counter`): the soul's icon as
/// a 16x16 sprite (sprite palette 13) over the picked column's cell after
/// the picks (0x0802330C: y = 24 + 16 picks, x 0x60), drawn from the tick
/// after it is loaded (0x08023360) through the white flashes, rising 2
/// pixels a tick for 8 ticks onto the first cell (0x0802337A), whitened by
/// its flash (fades 0x34 and 0x30, the sprite palette's), until the soul
/// takes the first cell (0x080233E0).
fn soul_flight<'a>(a: &'a CustomScreen, v: &View, sub: u8, counter: u8) -> Option<SpritePart<'a>> {
    let (tiles, first) = soul_icon(a, v)?;
    flight(a, v, tiles, first, sub, counter)
}

/// EXE5's capsule's mix, as its souls system keeps it (content/exe5/rules/
/// souls/capsules.luau, read by its fields' names): the capsule being mixed
/// (1 or 2: the button `capsule_1` or `capsule_2`), and the sequence's step
/// and count (the screen's state 0x3C).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapsuleMix {
    pub capsule: u8,
    pub step: u8,
    pub count: u8,
}

/// The capsule's mix's window (the screen's state 0x3C, 0x0802373A).
const CAPSULE_WINDOW: &str = "capsule";

impl CapsuleMix {
    /// Side `side`'s, when its ruleset has EXE5's souls system.
    pub fn of(b: &Battle, side: usize) -> Option<CapsuleMix> {
        let (schema, state) = b.system_state(side as u8, SOULS_SYSTEM)?;
        let byte = |name: &str| match state.get(schema, schema.index_of(name)?) {
            FieldValue::U8(n) => Some(n),
            _ => None,
        };
        Some(CapsuleMix { capsule: byte("mix_capsule")?, step: byte("mix_step")?, count: byte("mix_count")? })
    }
}

/// EXE5's capsule's mix (its state 0x3C, 0x0802373A: the souls system's
/// window `capsule`): the capsule's icon, its chip's (0x08023770: the chip
/// records' icons are the table it loads from, 0x0874A738), flies to the
/// last pick's cell as the soul's icon does to the first (the soul's
/// choice's steps and sprite, 0x080254D8).
fn capsule_flight<'a>(a: &'a CustomScreen, v: &View, packs: &crate::packs::Packs<'a>, problems: &mut Problems) -> Option<SpritePart<'a>> {
    let Phase::Window { window, .. } = v.screen.phase else { return None };
    let d = v.b.content.defs.window(window);
    if d.name != CAPSULE_WINDOW || v.b.content.defs.system(d.system).key != SOULS_SYSTEM {
        return None;
    }
    let mix = CapsuleMix::of(v.b, v.side as usize)?;
    let name = format!("capsule_{}", mix.capsule);
    let chip = v.screen.slots.iter().find_map(|s| match s.kind {
        SlotKind::Button { button, .. } if v.b.content.defs.button(button).name == name => s.face,
        _ => None,
    })?;
    let (tiles, _) = crate::lookups::chip_icon(packs, &v.b.content, chip, problems)?;
    flight(a, v, tiles, 0, mix.step, mix.count)
}

/// The sprite of EXE5's soul's choice and capsule's mix (0x080254D8): the
/// icon `first` of `tiles`, at the sequence's step `sub` and count
/// `counter`, in the soul button's icons' palette.
fn flight<'a>(a: &'a CustomScreen, v: &View, tiles: &'a Tiles, first: usize, sub: u8, counter: u8) -> Option<SpritePart<'a>> {
    let screen = v.screen;
    let rise = match sub {
        4 if counter > 0 => 0,
        8 => 2 * counter as i32,
        12 | 16 | 20 => 16,
        _ => return None,
    };
    let b = a.button(SOUL_BUTTON)?;
    // (The console's version's: Team Colonel's outline is another color.)
    let own = v.packs.version().and_then(|version| b.icon_palettes.iter().find(|(name, _)| name == version));
    let colors = own.map_or(b.icon_palette, |(_, p)| *p);
    let f = screen.look.fade;
    let palette = match f.mode {
        nettai_battle::battle::FadeMode::SoulFlash | nettai_battle::battle::FadeMode::SoulFlashBack => {
            let n = (f.level >> 4).min(16) as u8;
            colors.map(|c| crate::compose::apply_fade(c, Fade::White(n)))
        }
        _ => colors,
    };
    let y = 24 + 16 * screen.selection().len() as i32 - rise;
    Some(SpritePart {
        x: 0x60,
        y: (y & 0xFF) as u8,
        width: 16,
        height: 16,
        tiles,
        first_tile: first,
        hflip: false,
        vflip: false,
        palette,
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    })
}

/// The name EXE5's soul button is drawn by (the souls system's button; the
/// pack's `CustomScreen::buttons`).
const SOUL_BUTTON: &str = "soul";

impl View<'_> {
    fn icon(&self, c: FolderChip, problems: &mut Problems) -> Option<&Tiles> {
        crate::lookups::chip_icon(&self.packs, &self.b.content, c.id, problems).map(|(icon, _)| icon)
    }

    /// The navi's number: its emblem and its emblem's palette (the
    /// cursor's too) are by it (`sub_802812C`), until the navi definitions
    /// name their own: compat has the numbers of the content's keys.
    fn navi_number(&self) -> usize {
        navi_number(self.b, self.side)
    }

    /// Sprite palette 11, as the second fade record leaves it.
    fn emblem_palette(&self) -> Palette {
        let a = self.assets;
        let i = a.emblem_palette_of.get(self.navi_number()).copied().unwrap_or(0) as usize;
        let p = a.emblem_palettes.get(i).copied().unwrap_or([0; 16]);
        match window_fade(self.b) {
            Some(f) => p.map(|c| crate::compose::apply_fade(c, f)),
            None => p,
        }
    }
}

/// A Cross's name pictures and colors in the Cross window, by the Cross's
/// own game (a Gregar Cross shows Gregar's name in any player's window):
/// its game's custom-screen pictures and its number among that game's
/// Crosses. Its name is `cross_names`' 18 tiles from `18 * number` on the
/// cursor's row (`18 * (number + 5)` on the others'), its colors
/// `cross_palettes[number]` (`[number + 5]` once used). `navi` is the
/// navi whose Cross it is.
pub fn cross_picture<'a>(c: &Content, a: &'a CustomScreen, navi: NaviHandle, form: FormHandle) -> Option<(&'a VersionPictures, usize)> {
    let game = exe6_compat::forms::game(c, form)?;
    let number = (0..5u8).find(|&i| exe6_compat::forms::cross(c, navi, game, i) == Some(form))?;
    Some((a.versioned.get(game_name(game)), number as usize))
}

/// The pack's name of a game version (`Versioned`).
pub fn game_name(version: GameVersion) -> &'static str {
    match version {
        GameVersion::Gregar => "gregar",
        GameVersion::Falzar => "falzar",
    }
}

/// The pictures of the Beast a side's navi goes into, or is in: the
/// Beast Out button, its picture in the chip window and the BeastOut
/// chip's. They are its game's (`exe6_compat::Unlocks::beast_game`, the beast
/// system's rule): the console's
/// version's, but with a setup's Cross list a Cross of the other game
/// goes into that game's Beast (docs/engine/custom-screen.md §4.1).
pub fn beast_pictures<'a>(b: &Battle, a: &'a CustomScreen, side: u8) -> &'a VersionPictures {
    let side = side as usize & 1;
    let game = exe6_compat::Unlocks::of_side(b, side as u8).beast_game(&*b.content, b.stats[side].form);
    a.versioned.get(game_name(game))
}

/// The pack's name of a console's game version (`Versioned`).
pub fn version_name(b: &Battle, side: u8) -> &'static str {
    game_name(exe6_compat::Unlocks::of_side(b, side).version)
}

/// A side's navi's number (see `View::navi_number`; its lookup is
/// `lookups::emblem`'s).
pub fn navi_number(b: &Battle, side: u8) -> usize {
    crate::lookups::navi_number_of(&b.content, b.stats[side as usize & 1].navi)
}

/// The window's map, the tiles and the palettes it draws with.
struct Window {
    map: [MapEntry; COLUMNS * ROWS],
    tiles: LayerTiles,
    palettes: [Palette; 16],
    /// Why the chip window's picture isn't the one the console shows, if
    /// it isn't: a known difference (`known_picture`).
    picture_known: Option<&'static str>,
    /// In the font text mode, the strings whose tiles were left blank for
    /// the text layer: the chip window's name, and the Program Advance
    /// animation's names by their place (each with the cells it has, and
    /// the pick's code).
    name: Option<(String, usize)>,
    advance_names: Vec<Option<(String, usize, Option<String>)>>,
    /// Where the blocks go among the layer's tile numbers (the pack's).
    layout: nettai_assets::CustomLayout,
}

/// A slot's state as the original's byte holds it.
fn state_number(s: SlotState) -> usize {
    match s {
        SlotState::Selectable => 0,
        SlotState::Unavailable => 1,
        SlotState::Selected => 2,
    }
}

/// EXE6's cross system (content/exe6/rules/cross), whose state and windows
/// the Cross window's look reads.
const CROSS_SYSTEM: &str = "cross";

/// The tick of a Cross's choice the white fade is over and the Cross put
/// on (`sub_8027AAE`; the cross system's `PUT_ON_TICK`): the window's map
/// is the chips' again.
const CROSS_PUT_ON_TICK: u16 = 25;

/// EXE6's Cross window as the cross system keeps it
/// (content/exe6/rules/cross/window.luau), read by its fields' names: the
/// Crosses offered (their places among the player's Crosses,
/// `Unlocks::cross_at`), how many, which is chosen, the entry under the
/// window's cursor, and the Cross chosen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CrossWindow {
    pub offered: [u8; 5],
    pub count: u8,
    pub marked: [bool; 5],
    pub cursor: u8,
    pub chosen: Option<u8>,
}

impl CrossWindow {
    /// Side `side`'s Cross window, when its ruleset has EXE6's cross system.
    pub fn of(b: &Battle, side: usize) -> Option<CrossWindow> {
        let (schema, state) = b.system_state(side as u8, CROSS_SYSTEM)?;
        let elem = |name: &str, k: usize| state.get_elem(schema, schema.index_of(name)?, k);
        let field = |name: &str| Some(state.get(schema, schema.index_of(name)?));
        let byte = |v: Option<FieldValue>| match v {
            Some(FieldValue::U8(n)) => Some(n),
            _ => None,
        };
        let flag = |v: Option<FieldValue>| match v {
            Some(FieldValue::Bool(b)) => Some(b),
            _ => None,
        };
        let mut w = CrossWindow {
            count: byte(field("offered_count"))?,
            cursor: byte(field("window_cursor"))?,
            chosen: flag(field("cross_chosen"))?.then_some(byte(field("chosen"))?),
            ..CrossWindow::default()
        };
        for k in 0..5 {
            w.offered[k] = byte(elem("offered", k))?;
            w.marked[k] = flag(elem("marked", k))?;
        }
        Some(w)
    }
}

/// Where EXE6's Cross window is: the cross system's window up (its tick),
/// or a description from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrossStage {
    /// `sub_8027834`, 12 ticks.
    Opening(u16),
    /// `sub_802794A`, or a Cross's description from it.
    Up,
    /// `sub_802790C`, 6 ticks.
    Closing(u16),
    /// `sub_8027A58`, 34 ticks.
    Chosen(u16),
}

/// The stage of EXE6's Cross window on `s`, side `side`'s screen, if it is
/// up.
pub fn cross_stage(b: &Battle, s: &Screen) -> Option<CrossStage> {
    let (window, tick) = match s.phase {
        Phase::Window { window, tick } => (window, tick),
        Phase::Description { window: Some(window), .. } => (window, 0),
        _ => return None,
    };
    let d = b.content.defs.window(window);
    if b.content.defs.system(d.system).key != CROSS_SYSTEM {
        return None;
    }
    Some(match d.name.as_str() {
        "cross_opening" => CrossStage::Opening(tick),
        "cross_window" => CrossStage::Up,
        "cross_closing" => CrossStage::Closing(tick),
        "cross_chosen" => CrossStage::Chosen(tick),
        _ => return None,
    })
}

/// The Cross window's map the screen shows, if it shows one: its opening
/// steps every 3 ticks (`sub_8027834`), then the window with its Crosses,
/// until it closes (`sub_802790C`, 5 ticks) or the Cross chosen is put on
/// (`sub_8027AAE`).
fn cross_map(v: &View) -> Option<usize> {
    let stage = cross_stage(v.b, v.screen)?;
    let count = CrossWindow::of(v.b, v.side as usize).map_or(0, |w| w.count);
    let full = CROSS_OPENING_MAPS + count.max(1) as usize - 1;
    match stage {
        CrossStage::Opening(tick) if tick >= 3 => Some(tick as usize / 3 - 1),
        CrossStage::Up | CrossStage::Closing(_) => Some(full),
        CrossStage::Chosen(tick) if tick < CROSS_PUT_ON_TICK => Some(full),
        _ => None,
    }
}

impl Window {
    fn build(v: &View, text: &TextSink, problems: &mut Problems) -> Window {
        let a = v.assets;
        let mut w = Window {
            map: [MapEntry::default(); COLUMNS * ROWS],
            tiles: LayerTiles::new(),
            palettes: [[0; 16]; 16],
            picture_known: None,
            name: None,
            advance_names: Vec::new(),
            layout: a.layout,
        };
        // sub_8026840: the window's map, with the Cross tab or without; or
        // the Cross window's.
        let cross = cross_map(v);
        let (map, patches) = match cross {
            Some(i) => (a.cross_maps.get(i), &a.cross_patches),
            // (A game without the Cross tab has one map: EXE5.)
            None => (a.window_maps.get(v.screen.look.cross_tab as usize).or(a.window_maps.first()), &a.window_patches),
        };
        if let Some(m) = map {
            for (cell, e) in w.map.iter_mut().zip(m) {
                *cell = *e;
            }
        }
        // Its patches, numbered on from their first tile.
        let mut tile = patches.first_tile;
        for p in &patches.patches {
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
        w.tiles.put(w.layout.column_cells, &a.column_cells);
        w.tiles.put(w.layout.turn_limit, &a.turn_limit);
        w.tiles.put(w.layout.name_bar, &a.name_bar);
        w.palettes[11] = a.icon_palette;
        w.palettes[12] = a.gray_palette;
        w.palettes[14] = a.other_palette;
        w.palettes[13] = v.hud.hp_palettes[0];
        w.chip_window(v, text, problems);
        w.slots(v, problems);
        w.column(v, problems);
        if cross.is_some_and(|i| i >= CROSS_OPENING_MAPS) {
            w.cross_names(v, problems);
        }
        if v.screen.look.turn_limit {
            // sub_8029D34: "FINAL TURN", 7x2 at column 15, row 4 (past the
            // window's columns: drawn on the layer apart).
        }
        w
    }

    /// The Program Advance animation's names on the layer (`sub_802B80C`:
    /// the picks', one every 8 ticks; `sub_802B8E0`: the recipe's taken
    /// off; `sub_802B920`: the Program Advance's in their place at 16
    /// ticks, all taken off at 96), as (name, row, palette) with each
    /// name's tiles copied in, and palette 10's colors.
    fn program_advance(&mut self, v: &View, text: &TextSink, problems: &mut Problems) -> Vec<(usize, i32, u8)> {
        use nettai_battle::custom::screen::ProgramAdvanceStep as S;
        let s = v.screen;
        let (Phase::ProgramAdvance { anim }, Some(pa)) = (s.phase, s.program_advance) else { return Vec::new() };
        if pa.len == 0 {
            return Vec::new();
        }
        let side = &v.b.custom.sides[v.side as usize];
        let picks: Vec<FolderChip> =
            side.built.as_ref().and_then(|(h, _)| h.as_ref()).map(|h| h.selection.iter().flatten().copied().collect()).unwrap_or_default();
        let in_recipe = |k: usize| (pa.start as usize..(pa.start + pa.len) as usize).contains(&k);
        let row = |k: usize| ADVANCE_FIRST_ROW + 3 * k as i32;
        let palette = |k: usize| if in_recipe(k) { 10 } else { 13 };
        let shown: Vec<usize> = match anim.step {
            S::Names => (0..picks.len().min((anim.timer as usize).div_ceil(8))).collect(),
            S::Pause => (0..picks.len()).collect(),
            S::Result => (0..picks.len()).filter(|&k| !in_recipe(k)).collect(),
            _ => Vec::new(),
        };
        let mut out = Vec::new();
        for k in shown {
            self.put_advance_name(v, k, picks[k], text, problems);
            out.push((k, row(k), palette(k)));
        }
        if matches!(anim.step, S::Result) && anim.timer >= 0x10 {
            let k = pa.start as usize;
            let name = text.strings.chip_name(&v.b.content, pa.chip);
            self.put_advance_text(v, k, pa.chip, name, None, text, problems);
            out.push((k, row(k), 10));
        }
        if let Some(c) = v.assets.advance_name_colors.get(v.screen.look.pa_palette as usize) {
            self.palettes[10][..4].copy_from_slice(c);
        }
        out
    }

    /// A pick's name and code into name `k`'s tiles.
    fn put_advance_name(&mut self, v: &View, k: usize, c: FolderChip, text: &TextSink, problems: &mut Problems) {
        let code = crate::lookups::advance_code(&v.b.content, c.id, problems).then_some(c.code.0);
        self.put_advance_text(v, k, c.id, text.strings.chip_name(&v.b.content, c.id), code, text, problems);
    }

    /// A name (chip `chip`'s, and a pick's code in its last cell) into name
    /// `k`'s tiles; in the font mode blank tiles, and the words for the
    /// text layer.
    #[allow(clippy::too_many_arguments)]
    fn put_advance_text(
        &mut self,
        v: &View,
        k: usize,
        chip: ChipHandle,
        name: &str,
        code: Option<u8>,
        text: &TextSink,
        problems: &mut Problems,
    ) {
        let mut glyphs = crate::lookups::advance_name(v.hud, chip, name, problems);
        let letter = code.map(|code| if code < 26 { char::from(b'A' + code).to_string() } else { "*".to_string() });
        let at = self.layout.art + (2 * ADVANCE_NAME_CELLS * k) as u16;
        if text.takes(name) && letter.as_deref().is_none_or(|l| text.takes(l)) {
            // (The name's box: the cells before the code's, which nothing
            // else uses.)
            let cells = ADVANCE_NAME_CELLS - 1;
            self.tiles.put(at, &fonts::cell_text(v.hud, &[], ADVANCE_NAME_CELLS, 0));
            if self.advance_names.len() <= k {
                self.advance_names.resize(k + 1, None);
            }
            self.advance_names[k] = Some((name.to_string(), cells, letter));
            return;
        }
        glyphs.resize(ADVANCE_NAME_CELLS, 0);
        if let Some(letter) = letter
            && let Some(&g) = fonts::cell_glyphs(v.hud, &letter).0.first()
        {
            glyphs[ADVANCE_NAME_CELLS - 1] = g;
        }
        self.tiles.put(at, &fonts::cell_text(v.hud, &glyphs, ADVANCE_NAME_CELLS, 0));
    }

    /// `sub_802794A`: the Crosses' names (`sub_8029D94`: the one under the
    /// cursor in its own look) over the Cross window's map, and palette 10
    /// the Cross under the cursor's (`sub_8029EAC`: a used one's darker).
    fn cross_names(&mut self, v: &View, problems: &mut Problems) {
        let Some(w) = CrossWindow::of(v.b, v.side as usize) else { return };
        let unlocks = exe6_compat::Unlocks::of_side(v.b, v.side);
        // Each Cross's name and colors are its own game's (a setup's Cross
        // list can offer the other game's: docs/engine/custom-screen.md
        // §4.1).
        let navi = v.b.stats[v.side as usize].navi;
        let mut picture = |slot: usize| {
            let form = unlocks.cross_at(&*v.b.content, navi, w.offered[slot])?;
            crate::lookups::cross_name(v.assets, &v.b.content, navi, form, problems)
        };
        for slot in 0..w.count.min(5) as usize {
            let Some((own, number)) = picture(slot) else { continue };
            let name = number + if slot == w.cursor as usize { 0 } else { 5 };
            let at = self.layout.cross_names + (CROSS_NAME_TILES * slot) as u16;
            self.tiles.put_part(at, &own.cross_names, CROSS_NAME_TILES * name, CROSS_NAME_TILES);
            for i in 0..CROSS_NAME_TILES {
                let (x, y) = (1 + i % 9, 1 + 2 * slot + i / 9);
                self.map[y * COLUMNS + x] = MapEntry { tile: at + i as u16, hflip: false, vflip: false, palette: 10 };
            }
        }
        let c = w.cursor as usize;
        if let Some((own, number)) = picture(c) {
            let index = number + if w.marked[c] { 5 } else { 0 };
            if let Some(p) = own.cross_palettes.get(index) {
                self.palettes[10] = *p;
            }
        }
    }

    /// `sub_8028476`: the chip window shows what it was last drawn for: a
    /// chip's name, picture, code, element and damage (`sub_80284E2`), or
    /// a button's picture (`sub_80286D4`, `sub_802871C`, `sub_80287A4`,
    /// `sub_802877C`) with the window's colors where the chip's details
    /// go.
    fn chip_window(&mut self, v: &View, text: &TextSink, problems: &mut Problems) {
        let a = v.assets;
        let cw = v.screen.look.chip_window;
        // Palette 11's colors from 10 are the last chip's element's.
        if let Some(c) = cw.last_chip {
            let family = v.b.content.chip(c.id).family as usize;
            if let Some(colors) = a.element_colors.get(family) {
                self.palettes[11][10..].copy_from_slice(colors);
            }
        }
        let slot = cw.slot.min(SPECIAL_SLOT);
        let blank_details = |w: &mut Window, picture: &Picture| {
            // sub_80287D2: the name's cells and the window's colors.
            w.tiles.fill(w.layout.name, 2 * NAME_CELLS, BLANK_8);
            w.palettes[9] = a.frame_palettes.first().copied().unwrap_or([0; 16]);
            w.tiles.put(w.layout.art, &picture.tiles);
            w.palettes[10] = picture.palette;
            // sub_802869E
            w.tiles.fill(w.layout.code, 2, BLANK_8);
            w.tiles.fill(w.layout.element, 4, BLANK_7);
            w.tiles.fill(w.layout.digits, 6, BLANK_8);
        };
        let state = state_number(v.screen.slots[slot as usize].state);
        match v.screen.slots[slot as usize].kind {
            SlotKind::Chip { .. } | SlotKind::NaviChip(_) => {
                let Some(c) = cw.last_chip else { return };
                self.chip_details(v, c, text, problems);
            }
            SlotKind::Ok => {
                let p = if cw.picks == 0 { &a.pictures.ok } else { &a.pictures.ok_picked };
                blank_details(self, p);
            }
            SlotKind::Button { button, cell } => {
                if let Some(chip) = v.screen.slots[slot as usize].face {
                    // EXE5's capsules (0x08024422): the chip's name and
                    // picture alone; the frame's colors stay the last
                    // drawn.
                    self.chip_name_and_art(v, chip, text, problems);
                    self.palettes[9] = self.frame_palette(v, cw.framed, problems);
                    // sub_802869E
                    self.tiles.fill(self.layout.code, 2, BLANK_8);
                    self.tiles.fill(self.layout.element, 4, BLANK_7);
                    self.tiles.fill(self.layout.digits, 6, BLANK_8);
                } else if let Some(look) = v.button_look(button) {
                    // (EXE5's soul button, 0x08024540: its picture in its
                    // first palette, a Chaos Unison's in its second, the
                    // slot's +6, whatever its state.)
                    let palette = if is_soul_button(v.b, v.screen, cw.slot) {
                        SoulOffer::of(v.b, v.side as usize).map_or(0, |o| o.chaos as usize)
                    } else {
                        state
                    };
                    blank_details(self, &look.details(palette));
                    if look.uses_digit {
                        // 0x080245C8: the uses left (the button's first
                        // cell's, +4), over the damage's last cell.
                        let first = if cell == ButtonCell::Right { slot - 1 } else { slot };
                        let uses = v.screen.slots[first as usize].uses_left as usize;
                        self.tiles.put_part(self.layout.digits + 4, &a.digits, 2 * uses.min(9), 2);
                    }
                }
            }
            SlotKind::Empty | SlotKind::Hidden => {}
        }
    }

    /// `sub_80284E2`: a chip's name (8 cells of the 8x16 font in the
    /// window's colors), its picture and palette, the window's colors by
    /// the chip's class, its code, its element's icon (and colors), and
    /// its damage if it shows ("???" for a chip that hides it as an A:
    /// `ChipTraits::HIDES_DAMAGE_AS_A`), right-aligned in three cells.
    fn chip_details(&mut self, v: &View, c: FolderChip, text: &TextSink, problems: &mut Problems) {
        let a = v.assets;
        let data = v.b.content.chip(c.id);
        self.chip_name_and_art(v, c.id, text, problems);
        // The frame's colors by class, a dark chip's (of the first
        // three classes) dark; the code's glyph, the element's icon.
        self.palettes[9] = self.frame_palette(v, Some(c), problems);
        let code = c.code.0.min(NO_CODE) as usize;
        self.tiles.put_part(self.layout.code, &a.codes, 2 * code, 2);
        let family = data.family as usize;
        if family < a.element_colors.len() {
            self.tiles.put_part(self.layout.element, &a.elements, 4 * family, 4);
        }
        let shows = data.flags.0 & (ChipFlags::HAS_DAMAGE | ChipFlags::DAMAGE_SHOWN_VARIABLE) != 0;
        let digits: Vec<usize> = if !shows {
            Vec::new()
        } else if data.traits.has(ChipTraits::HIDES_DAMAGE_AS_A) && c.code.0 == 0 {
            // (The original compares the whole chip word, number and code,
            // with Muramasa's number: only an A matches.)
            vec![DIGIT_UNKNOWN; 3]
        } else {
            let damage = nettai_battle::hand::chip_damage(v.b, Some(c.id), v.side);
            damage.min(999).to_string().bytes().map(|d| (d - b'0') as usize).collect()
        };
        let blanks = 3 - digits.len();
        self.tiles.fill(self.layout.digits, 2 * blanks, BLANK_8);
        for (i, &d) in digits.iter().enumerate() {
            self.tiles.put_part(self.layout.digits + 2 * (blanks + i) as u16, &a.digits, 2 * d, 2);
        }
    }

    /// The frame's colors (palette 9) for the chip that last set them: by
    /// its class, a dark chip's (of the first three classes) dark; the
    /// standard ones for none (OK's and a button's picture, `sub_80287D2`).
    fn frame_palette(&self, v: &View, c: Option<FolderChip>, problems: &mut Problems) -> Palette {
        let a = v.assets;
        let frame = c.map_or(0, |c| crate::lookups::chip_window(a, &v.b.content, c.id, c.code.0, problems));
        a.frame_palettes.get(frame).copied().unwrap_or([0; 16])
    }

    /// A chip's name (8 cells of the 8x16 font in the window's colors) and
    /// its picture and palette: `sub_80284E2`'s first part, and all a
    /// button's chip shows (EXE5's capsules, 0x08024422).
    fn chip_name_and_art(&mut self, v: &View, chip: ChipHandle, text: &TextSink, problems: &mut Problems) {
        let c = chip;
        let data = v.b.content.chip(c);
        let name = text.strings.chip_name(&v.b.content, c);
        let glyphs = crate::lookups::chip_name(v.hud, &v.b.content, c, name, problems);
        if text.takes(name) {
            // The name's cells in the window's color, the words on the
            // text layer in all eight of them (nothing follows the name).
            self.tiles.put(self.layout.name, &fonts::cell_text(v.hud, &[], NAME_CELLS, NAME_SHIFT));
            self.name = Some((name.to_string(), NAME_CELLS));
        } else {
            self.tiles.put(self.layout.name, &fonts::cell_text(v.hud, &glyphs, NAME_CELLS, NAME_SHIFT));
        }
        // (The Beast Out chip's picture is the Beast's the navi goes into.)
        let beast_out = v.b.roles().try_chip(nettai_battle::content::ChipRole::BeastOut) == Some(c);
        // A chip whose palette no ROM holds has its definition's
        // (`art_palette`). The picture of a chip the US release cut is the
        // Japanese ROMs': a US console shows a placeholder there. The
        // Gregar and Falzar chips' are each their own beast: a console
        // shows its own in both. A version's own chip's is its own ROM's:
        // a console of the other version shows its counterpart's.
        let art = if beast_out {
            Some((&v.beast.beast_out, None))
        } else {
            crate::lookups::chip_art(&v.packs, &v.b.content, c, problems).map(|art| (&art.picture, Some(art)))
        };
        if let Some((p, art)) = art {
            self.tiles.put(self.layout.art, &p.tiles);
            self.palettes[10] = if beast_out { p.palette } else { data.art_palette.unwrap_or(p.palette) };
            let console_version = v.packs.version().unwrap_or_else(|| version_name(v.b, v.side));
            self.picture_known = match art {
                Some(a) if a.region.as_deref().is_some_and(|r| r != v.region) => {
                    Some("the Japanese games' chip picture (a US console shows a placeholder)")
                }
                Some(a) if a.version.as_deref().is_some_and(|g| g != console_version) => Some(match a.region {
                    // (The Gregar and Falzar chips', which the US release
                    // cut: each Japanese ROM has its own beast in both.)
                    Some(_) => "the chip's own beast's picture (a console shows its own)",
                    None => crate::lookups::OTHER_VERSIONS_ART,
                }),
                _ => None,
            };
        }
    }

    /// `sub_8028250`: each slot's tiles, from tile 0xE1 on, as its kind
    /// takes them; and `sub_80283C8`: the chip slots' icon palettes by
    /// their state.
    fn slots(&mut self, v: &View, problems: &mut Problems) {
        let a = v.assets;
        let mut at = self.layout.slots;
        for (s, slot) in v.screen.slots.iter().enumerate() {
            let state = state_number(slot.state);
            match slot.kind {
                SlotKind::Chip { .. } | SlotKind::NaviChip(_) => {
                    let Some(c) = v.screen.look.slot_chips[s] else { continue };
                    if v.screen.look.slot_picked[s] {
                        self.tiles.put(at, &a.empty_icon);
                    } else if let Some(icon) = v.icon(c, problems) {
                        self.tiles.put(at, icon);
                    }
                    // sub_8028214: the code's glyph; the special codes
                    // show blank.
                    match c.code.0 {
                        0x1B | 0x1C => self.tiles.fill(at + 4, 2, self.layout.slot_blank),
                        code => self.tiles.put_part(at + 4, &a.slot_codes, 2 * code.min(EMPTY_SLOT_CODE) as usize, 2),
                    }
                    at += 6;
                }
                SlotKind::Ok | SlotKind::Button { cell: ButtonCell::Right, .. } => {}
                SlotKind::Button { button, .. } => {
                    if let Some(chip) = slot.face {
                        // A button that shows a chip (EXE5's capsules,
                        // 0x08024114): a chip's slot, its icon the empty
                        // one once it is used, its code blank.
                        if slot.state == SlotState::Selected {
                            self.tiles.put(at, &a.empty_icon);
                        } else if let Some((icon, _)) = crate::lookups::chip_icon(&v.packs, &v.b.content, chip, problems) {
                            self.tiles.put(at, icon);
                        }
                        self.tiles.fill(at + 4, 2, self.layout.slot_blank);
                        at += 6;
                    } else if let Some(look) = v.button_look(button) {
                        self.tiles.put_part(at, look.tiles, look.count * look.sets.of(state), look.count);
                        at += look.advance;
                    }
                }
                SlotKind::Empty => {
                    self.tiles.put(at, &a.empty_icon);
                    self.tiles.put_part(at + 4, &a.slot_codes, 2 * EMPTY_SLOT_CODE as usize, 2);
                    at += 6;
                }
                // (EXE6's Beast Out button's hidden look; a game without
                // the button, EXE5, leaves its 3x2 the window's fill, as a
                // hidden slot's: its handler for no special button,
                // 0x080240D8, copies nothing in.)
                SlotKind::Hidden if s as u8 == SPECIAL_SLOT && !v.beast.beast_buttons.is_empty() => {
                    self.tiles.put_part(at, &v.beast.beast_buttons, 24, 8)
                }
                SlotKind::Hidden if s as u8 == SPECIAL_SLOT => self.tiles.fill(at, 6, self.layout.slot_blank),
                SlotKind::Hidden => {
                    self.tiles.fill(at, 6, self.layout.slot_blank);
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
                SlotKind::Ok => continue,
                // (A button's chip: a chip's, 0x08024200.)
                SlotKind::Button { .. } if slot.face.is_some() => {
                    if slot.state == SlotState::Unavailable {
                        12
                    } else {
                        11
                    }
                }
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
            let at = self.layout.column_icons + 4 * i as u16;
            let icon = match v.screen.look.column[i] {
                Some(c) => v.icon(c, problems).map(|t| (t, 0)),
                // EXE5's soul, given for a chip: its icon (0x0802341C).
                None if picks.get(i) == Some(&SPECIAL_SLOT) => soul_icon(v.assets, v),
                None => None,
            };
            let (tiles, first) = icon.unwrap_or((&a.empty_icon, 0));
            self.tiles.put_part(at, tiles, first, 4);
            let filled = (i < picks.len() || v.screen.look.column_kept == Some(i as u8)) as u16;
            let tile = self.layout.column_cells + 2 * filled;
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

    /// The chip window's picture, where it shows, as a known difference.
    fn known_picture(&self, place: Placement, why: &'static str, problems: &mut Problems) {
        self.known_tiles(place, self.layout.art..self.layout.art + PICTURE_TILES, why, problems);
    }

    /// The icons of the other version's chips (`lookups::other_versions_icon`)
    /// in the slots and the picked chips' column, where they show, as known
    /// differences.
    fn known_icons(&self, v: &View, place: Placement, problems: &mut Problems) {
        let other = |c: FolderChip| crate::lookups::other_versions_icon(&v.packs, v.b, c.id);
        for (s, slot) in v.screen.slots.iter().enumerate().take(10) {
            let shown = matches!(slot.kind, SlotKind::Chip { .. } | SlotKind::NaviChip(_)) && !v.screen.look.slot_picked[s];
            if shown && v.screen.look.slot_chips[s].is_some_and(other) {
                // (The slot's 2x2 icon cells: `slots`.)
                let first = self.map[(13 + 3 * (s / 5)) * COLUMNS + 1 + 2 * (s % 5)].tile;
                self.known_tiles(place, first..first + 4, crate::lookups::OTHER_VERSIONS_ART, problems);
            }
        }
        for i in 0..5usize {
            if v.screen.look.column[i].is_some_and(other) {
                let first = self.layout.column_icons + 4 * i as u16;
                self.known_tiles(place, first..first + 4, crate::lookups::OTHER_VERSIONS_ART, problems);
            }
        }
    }

    /// The window's cells that show `tiles` of its block, where they show,
    /// as a known difference.
    fn known_tiles(&self, place: Placement, tiles: std::ops::Range<u16>, why: &'static str, problems: &mut Problems) {
        let mut rect: Option<[i32; 4]> = None;
        for y in 0..ROWS {
            for x in place.from..place.to {
                if !tiles.contains(&self.map[y * COLUMNS + x].tile) {
                    continue;
                }
                let Some(px) = screen_x(x as i32, place.scroll) else { continue };
                let py = 8 * y as i32;
                rect = Some(match rect {
                    None => [px, py, px + 8, py + 8],
                    Some(r) => [r[0].min(px), r[1].min(py), r[2].max(px + 8), r[3].max(py + 8)],
                });
            }
        }
        if let Some([x0, y0, x1, y1]) = rect {
            let (x0, x1) = (x0.max(0), x1.min(240));
            if x0 < x1 {
                problems.known(x0, y0, x1 - x0, y1 - y0, why);
            }
        }
    }

    /// Draw a tile of the layer's character block at a cell of the layer.
    fn cell(&self, layer: &mut Layer, e: MapEntry, col: i32, row: i32, scroll: u32) {
        if let Some(px) = screen_x(col, scroll) {
            layer.draw_tile(self.tiles.tile(e.tile), &self.palettes[e.palette as usize & 15], px, 8 * row, e.hflip, e.vflip);
        }
    }

    /// The chip window's name on the text layer (the font mode): over the
    /// name's cells where the window's map puts them, in their palette's
    /// colors from 8 (`NAME_SHIFT`), cut to the window's columns on the
    /// layer.
    fn name_item(&self, text: &mut TextSink, place: Placement) {
        let Some((name, cells)) = &self.name else { return };
        let Some(i) = self.map.iter().position(|e| e.tile == self.layout.name) else { return };
        let (col, row) = (i % COLUMNS, i / COLUMNS);
        let palette = &self.palettes[self.map[i].palette as usize & 15];
        let shift = NAME_SHIFT as usize;
        // The cells the window shows, and where: the string sits where its
        // last shown cell puts it (sliding in, its first cells are still
        // past the screen's left edge, which the layer wraps to its right).
        let shown: Vec<(usize, i32)> =
            (col..col + cells).filter(|c| (place.from..place.to).contains(c)).filter_map(|c| Some((c, screen_x(c as i32, place.scroll)?))).collect();
        let Some(&(last, last_x)) = shown.last() else { return };
        let x = last_x - 8 * (last - col) as i32;
        let along: Vec<i32> = shown.iter().filter(|&&(c, px)| px == x + 8 * (c - col) as i32).map(|&(_, px)| px).collect();
        let (Some(&x0), Some(&x1)) = (along.first(), along.last()) else { return };
        let item = TextItem::new(name.as_str(), Role::Cell, Rect::new(x, 8 * row as i32, 8 * *cells as i32, 16), palette[1 + shift], Some(palette[2 + shift]));
        let clip = Rect::new(x0, 0, x1 + 8 - x0, crate::compose::HEIGHT as i32);
        text.push(Plane::Hud, TextItem { clip, ..item });
    }
}

/// Where a column of the 256-pixel layer is on the screen, scrolled, as a
/// signed place (past the left edge is negative).
fn layer_x(col: i32, scroll: u32) -> i32 {
    let px = (8 * col - scroll as i32).rem_euclid(256);
    if px >= 240 { px - 256 } else { px }
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

fn draw_names(v: &View, w: &Window, hud_layer: &mut Layer, names_layer: &mut Layer, text: &mut TextSink, problems: &mut Problems) {
    let side = (v.side ^ 1) as usize;
    let other = v.b.stats[side].navi;
    // (Its variant name when its side's rules ask: EXE5's Hub Style in a link
    // battle, 0x0801AE3A's NameID 0xEA.)
    let variant = if v.b.looks[side].name_variant { text.strings.navi_variant_name(&v.b.content, other) } else { None };
    let name = variant.unwrap_or_else(|| text.strings.navi_name(&v.b.content, other));
    let glyphs = crate::lookups::navi_name(v.hud, other, variant.is_some(), name, problems);
    let len = glyphs.len().min(ENEMY_NAME_CELLS);
    let col = 0x1E - len as i32;
    let palette = &w.palettes[13];
    fonts::layer_text(text, Plane::Bg0, names_layer, v.hud, name, &glyphs, ENEMY_NAME_CELLS, palette, (8 * col, 0), Align::Left);
    // The bar: the slanted end, then one cell a glyph (eight at most; a
    // ninth glyph has bar cells from column 21 on, no end).
    let bar = |tile: u16| MapEntry { tile, hflip: false, vflip: false, palette: 13 };
    let cells = len.min(8);
    let mut col = 0x1D - cells as i32;
    if len < ENEMY_NAME_CELLS {
        w.cell(hud_layer, bar(w.layout.name_bar), col, 0, 0);
        w.cell(hud_layer, bar(w.layout.name_bar + 1), col, 1, 0);
        col += 1;
    }
    let n = if len < ENEMY_NAME_CELLS { len } else { ENEMY_NAME_CELLS };
    for k in 0..n as i32 {
        w.cell(hud_layer, bar(w.layout.name_bar + 2), col + k, 0, 0);
        w.cell(hud_layer, bar(w.layout.name_bar + 3), col + k, 1, 0);
    }
}

/// The cursor's corners (`sub_8028820`): where its slot's frame is
/// (`jt_802886C`'s routines, less 3) and the four 8x8 corners in each of
/// its two frames (`byte_80288B0` and the others: y, x, flips). OK's and
/// the special slot's are the pack's (`CustomLayout`: EXE5's sit otherwise).
#[derive(Clone, Copy)]
struct CursorShape {
    corners: [[(i32, i32, bool, bool); 4]; 2],
}

const CHIP_CURSOR: CursorShape = CursorShape {
    corners: [
        [(0, 0, false, false), (0, 0xE, true, false), (0xE, 0xE, true, true), (0xE, 0, false, true)],
        [(1, 1, false, false), (1, 0xC, true, false), (0xC, 0xC, true, true), (0xC, 1, false, true)],
    ],
};

/// The cursor at a place the pack gives (OK's, the special slot's).
fn cursor_at(p: &nettai_assets::CursorPlace) -> (i32, i32, CursorShape) {
    let corners = p.corners.map(|frame| frame.map(|(y, x, h, v)| (y as i32, x as i32, h, v)));
    (p.x as i32, p.y as i32, CursorShape { corners })
}

const BUTTON_CURSOR: CursorShape = CursorShape {
    corners: [
        [(2, 2, false, false), (2, 0x1C, true, false), (0x14, 0x1C, true, true), (0x14, 2, false, true)],
        [(4, 4, false, false), (4, 0x1A, true, false), (0x12, 0x1A, true, true), (0x12, 4, false, true)],
    ],
};

/// `sub_80289E4`: the Cross window's cursor, a box around the Cross under
/// it: four corners, then seven edge pieces above and below (`byte_8028A30`:
/// y, x, flips), in sprite palette 14.
fn cross_cursor_parts<'a>(v: &View, a: &'a CustomScreen, frame: u8) -> Vec<SpritePart<'a>> {
    let cursor = CrossWindow::of(v.b, v.side as usize).map_or(0, |w| w.cursor);
    let (x, y) = (5, 5 + 16 * cursor as i32);
    let corners = [(2, 3, false, false), (2, 0x43, true, false), (0xC, 0x43, true, true), (0xC, 3, false, true)];
    let edges = (0..7).map(|i| (2, 0xB + 8 * i, false, false)).chain((0..7).map(|i| (0xC, 0xB + 8 * i, false, true)));
    let pieces = corners.into_iter().map(|c| (c, 0)).chain(edges.map(|e| (e, 1)));
    let palette = a.cross_cursor_palette;
    pieces
        .map(|((dy, dx, hflip, vflip), edge)| SpritePart {
            x: ((x + dx) & 0x1FF) as u16,
            y: (y + dy) as u8,
            width: 8,
            height: 8,
            tiles: &a.cross_cursor,
            first_tile: 2 * (frame as usize & 1) + edge,
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

fn cursor_parts<'a>(v: &View, a: &'a CustomScreen, frame: u8) -> Vec<SpritePart<'a>> {
    let s = v.screen;
    let slot = s.cursor;
    let (x, y, shape) = match s.slots[slot as usize].kind {
        SlotKind::Chip { .. } | SlotKind::NaviChip(_) | SlotKind::Empty | SlotKind::Hidden => {
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            (16 * col + 8, 0x68 + 0x18 * row, CHIP_CURSOR)
        }
        SlotKind::Ok => cursor_at(&a.layout.ok_cursor),
        // (A button that shows a chip: a chip's cursor, 0x080246B8.)
        SlotKind::Button { .. } if s.slots[slot as usize].face.is_some() => {
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            (16 * col + 8, 0x68 + 0x18 * row, CHIP_CURSOR)
        }
        SlotKind::Button { button, .. } => v.button_look(button).map_or((0x38, 0x80, BUTTON_CURSOR), |l| l.cursor),
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
    let sine = &v.b.game_rules().sine;
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

/// The Regular chip's frame (`sub_802899C`): a 32x32 sprite around the
/// first slot.
/// The chip a button holds, over the button (EXE5's Arm Change, 0x080254F4):
/// its icon as a 16x16 sprite where the button's look says, in sprite
/// palette 10 (the HUD's icons').
fn held_part<'a>(v: &View, packs: &crate::packs::Packs<'a>, problems: &mut Problems) -> Option<SpritePart<'a>> {
    let h = v.screen.hold?;
    let SlotKind::Button { button, .. } = v.screen.slots[h.button as usize].kind else { return None };
    let (x, y) = v.button_look(button)?.held_at?;
    let chip = v.screen.look.slot_chips[h.chip as usize]?;
    let (tiles, palette) = crate::lookups::chip_icon(packs, &v.b.content, chip.id, problems)?;
    Some(SpritePart {
        x: (x & 0x1FF) as u16,
        y: (y & 0xFF) as u8,
        width: 16,
        height: 16,
        tiles,
        first_tile: 0,
        hflip: false,
        vflip: false,
        palette: *palette,
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    })
}

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
/// `emblem` holds the emblem sprite's tiles (`lookups::emblem`); the font
/// mode's strings go to `text`.
#[allow(clippy::too_many_arguments)]
pub fn draw<'a>(
    b: &'a Battle,
    assets: &'a Bundle,
    packs: &crate::packs::Packs<'a>,
    emblem: &'a Tiles,
    region: &str,
    hud_layer: &mut Layer,
    names_layer: &mut Layer,
    list: &mut SpriteList<'a>,
    text: &mut TextSink,
    problems: &mut Problems,
) {
    let Some((_, screen)) = local(b) else { return };
    let a = &assets.custom;
    if !crate::lookups::custom_graphics(a, problems) {
        return;
    }
    let side = b.setup.local_side & 1;
    let v = View {
        b,
        side,
        screen,
        assets: a,
        beast: beast_pictures(b, a, side),
        hud: &assets.hud,
        packs: packs.clone(),
        region,
    };
    let place = placement(screen);
    let mut w = Window::build(&v, text, problems);
    let advance_names = w.program_advance(&v, text, problems);
    w.draw(hud_layer, place);
    if let Some(why) = w.picture_known {
        w.known_picture(place, why, problems);
    }
    w.known_icons(&v, place, problems);
    w.name_item(text, place);
    // The Program Advance's names, a column right of the layer's scroll
    // (`sub_802BA18`), each 9x2 cells column by column.
    let col = (place.scroll >> 3) as i32 + 1;
    for (k, row, palette) in advance_names {
        let first = w.layout.art + (2 * ADVANCE_NAME_CELLS * k) as u16;
        for i in 0..2 * ADVANCE_NAME_CELLS as u16 {
            let e = MapEntry { tile: first + i, hflip: false, vflip: false, palette };
            w.cell(hud_layer, e, col + (i / 2) as i32, row + (i % 2) as i32, place.scroll);
        }
        // In the font mode, the name and the code on the text layer.
        if let Some(Some((name, cells, code))) = w.advance_names.get(k) {
            let colors = &w.palettes[palette as usize & 15];
            let at = |c: i32, n: usize| Rect::new(layer_x(col + c, place.scroll), 8 * row, 8 * n as i32, 16);
            text.push(Plane::Hud, TextItem::new(name.as_str(), Role::Cell, at(0, *cells), colors[1], Some(colors[2])));
            if let Some(code) = code {
                let last = ADVANCE_NAME_CELLS as i32 - 1;
                text.push(Plane::Hud, TextItem::new(code.as_str(), Role::Cell, at(last, 1), colors[1], Some(colors[2])));
            }
        }
    }
    if screen.look.turn_limit && place.to == COLUMNS {
        // sub_8029D34: 7x2 at column 15, row 4.
        for i in 0..14u16 {
            let e = MapEntry { tile: w.layout.turn_limit + i, hflip: false, vflip: false, palette: 9 };
            w.cell(hud_layer, e, 15 + (i % 7) as i32, 4 + (i / 7) as i32, place.scroll);
        }
    }
    if names_shown(b, screen) {
        draw_names(&v, &w, hud_layer, names_layer, text, problems);
    }
    // The sprites, as their routines queue them (each in front of the
    // one before).
    let drawn = screen.look.drawn;
    let mut queue: Vec<SpritePart<'a>> = Vec::new();
    // EXE5's soul choice's flying icon (its state 9's routines draw it
    // before the screen's others).
    if soul_window_up(b, screen)
        && let Some(o) = SoulOffer::of(b, v.side as usize)
    {
        queue.extend(soul_flight(a, &v, o.step, o.count));
    }
    // Its capsule's mix's (state 0x3C's).
    queue.extend(capsule_flight(a, &v, packs, problems));
    if let Some(frame) = drawn.cursor {
        queue.extend(cursor_parts(&v, a, frame));
    }
    if let Some((x, spin)) = drawn.emblem {
        queue.push(emblem_part(&v, emblem, x, spin));
    }
    if drawn.regular {
        queue.push(regular_part(&v, a));
    }
    if drawn.held {
        queue.extend(held_part(&v, packs, problems));
    }
    if let Some(frame) = drawn.cross_cursor {
        queue.extend(cross_cursor_parts(&v, a, frame));
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
