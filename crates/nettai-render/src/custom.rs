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
use nettai_assets::{Bundle, ButtonPictures, ButtonSets, CustomScreen, Hud, MapEntry, Palette, Picture, Tiles, VersionPictures};
use nettai_battle::{Battle, Content};
use nettai_battle::battle::{FadeMode, mode};
use nettai_battle::content::{ButtonView, ChipFlags, ChipTraits, PlayerFact, WindowView};
use nettai_battle::rules::Flight;
use nettai_battle::custom::screen::{HiddenStage, OK_SLOT, SPECIAL_SLOT};
use nettai_battle::custom::{ButtonCell, FolderChip, Phase, Screen, Side, SlotKind, SlotState};
use nettai_content_api::{ChipHandle, FormHandle, NaviHandle};

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
/// The forms' names in the form list's window (9x2 each, EXE6's Cross
/// window's, `byte_8029DF8`, from the layout's `form_names`).
pub(crate) const FORM_NAME_TILES: usize = 18;
/// The form list window's maps: three opening steps, then the window with
/// one to five forms (EXE6's Cross window's).
const FORM_LIST_OPENING_MAPS: usize = 3;
/// The Program Advance animation's names (`sub_802B80C`): 9 cells of the
/// 8x16 font from the chip window's picture's first tile (EXE6's 0xAB), 18
/// tiles a name; a pick's code in its last cell; a name every 3 rows from
/// row 5, a column right of the layer's scroll; the recipe's in palette 10,
/// the others' in 13.
const ADVANCE_NAME_CELLS: usize = 9;
const ADVANCE_FIRST_ROW: i32 = 5;
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
        FadeMode::HalfOut | FadeMode::ProgramAdvance | FadeMode::Shade => (true, false),
        // Back toward clear: once it gets there the original takes the
        // palettes' transform off.
        FadeMode::HalfOutBack | FadeMode::ProgramAdvanceBack | FadeMode::ShadeBack => (f.active, false),
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
        FadeMode::ShadeWindow => true,
        FadeMode::ShadeWindowBack => f.active,
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
        FadeMode::HalfOut | FadeMode::HalfOutBack | FadeMode::EndToWhite | FadeMode::IntroFromWhite => fade(b),
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
    /// The version pictures the side's buttons draw (`button_pictures`): the
    /// player's version's, unless its navi is in a form of another version
    /// (EXE6's Beast Out button: the other game's Cross's Beast). A button's
    /// look is this version's own, if it has one.
    buttons: &'a VersionPictures,
    /// The navi's emblem's palette (`lookups::emblem`; zeros: it has no
    /// emblem), which is sprite palette 11: the cursor's and the Regular
    /// chip's frame's too (`sub_802812C`).
    emblem_palette: Palette,
    hud: &'a Hud,
    /// Every pack's graphics: a chip's icon and picture are its game's.
    packs: crate::packs::Packs<'a>,
}

/// A button of the rules as the frontend draws it (docs/design/rules-in-luau.md
/// §4.8): the pack's look of the name its content registers it under
/// (`ButtonPictures`: its tiles by set, which set a state shows, the cursor
/// over it, its picture in the chip window), and its tiles a set, its
/// cells' (the content's `cells`).
#[derive(Clone, Copy)]
struct ButtonLook<'a> {
    pack: &'a ButtonPictures,
    count: usize,
}

impl<'a> ButtonLook<'a> {
    /// The first tile of the set a slot in state `state` shows.
    fn set(&self, state: usize) -> usize {
        self.count
            * match self.pack.sets {
                ButtonSets::Each => state,
                ButtonSets::Other => (state != 0) as usize,
                ButtonSets::Unavailable => (state == 1) as usize,
            }
    }

    /// Its picture in the chip window, in the `palette`th of its palettes
    /// (0: its own).
    fn details(&self, palette: usize) -> Picture {
        let own = self.pack.picture.palette;
        let palette = if palette == 0 { own } else { self.pack.palettes.get(palette).copied().unwrap_or(own) };
        Picture { palette, ..self.pack.picture.clone() }
    }

    fn cursor(&self) -> (i32, i32, CursorShape) {
        cursor_at(&self.pack.cursor)
    }
}

impl<'a> View<'a> {
    /// The look of button `button`: the pack's of its name (the Beast's
    /// version's own first: `VersionPictures::buttons`). A name the pack
    /// has no look for is drawn as nothing (`lookups::button` says so); a
    /// button that shows a chip (`Slot::face`: EXE5's capsules) is drawn as
    /// that chip's slot whatever its name.
    fn button_look(&self, button: nettai_battle::content::ButtonHandle) -> Option<ButtonLook<'a>> {
        let d = self.b.content.defs.button(button);
        let pack = crate::lookups::button_of(self.assets, self.buttons, &d.name)?;
        Some(ButtonLook { pack, count: pack.width as usize * pack.height as usize * d.cells.max(1) as usize })
    }

    /// What the special slot shows with no button in it: the hidden set of
    /// a button its content registers there, if the pack's look has one
    /// (EXE6's Beast Out button's fourth; EXE5's soul button has none: the
    /// window's fill, as its handler for no special button copies nothing
    /// in, 0x080240D8).
    fn hidden_special(&self) -> Option<(&'a ButtonPictures, usize)> {
        self.b.content.defs.buttons.iter().filter(|d| d.slot == SPECIAL_SLOT).find_map(|d| {
            let pack = crate::lookups::button_of(self.assets, self.buttons, &d.name)?;
            Some((pack, pack.hidden? as usize))
        })
    }
}

/// Whether slot `slot` of the screen is a button that offers a form
/// (`ButtonView::FormOffer`: EXE5's soul button).
fn offers_form(b: &Battle, screen: &Screen, slot: u8) -> bool {
    matches!(screen.slots[slot as usize].kind, SlotKind::Button { button, .. } if b.content.defs.button(button).view == Some(ButtonView::FormOffer))
}

/// What the window up on `screen` shows (its `view`), if one is up and has
/// a view.
fn window_view(b: &Battle, screen: &Screen) -> Option<WindowView> {
    let Phase::Window { window, .. } = screen.phase else { return None };
    b.content.defs.window(window).view
}

/// The pack's look of the content's button with view `view` (by the
/// button's name, the key of its look).
fn view_look<'a>(v: &View<'a>, view: ButtonView) -> Option<&'a ButtonPictures> {
    let d = v.b.content.defs.buttons.iter().find(|d| d.view == Some(view))?;
    crate::lookups::button_of(v.assets, v.buttons, &d.name)
}

/// The icon of the form the button that offers a form (the view
/// `form_offer`) offers or gave, if the special slot is that button: the
/// pack's `icons` of the button and the icon's first tile in them, the
/// offer's place (`offer_icon`). (EXE5's soul button: the soul's icon,
/// Chaos Unison's the 13th, 0x0802341C.)
fn offered_icon<'a>(v: &View<'a>) -> Option<(&'a Tiles, usize)> {
    if !offers_form(v.b, v.screen, SPECIAL_SLOT) {
        return None;
    }
    let b = view_look(v, ButtonView::FormOffer)?;
    let n = offer_icon(v.b, v.side, v.b.offer(v.side)?);
    (b.icons.len() >= 4 * (n + 1)).then_some((&b.icons, 4 * n))
}

/// An offer's icon among its button's: the offered form's place in the
/// side's navi's form list that holds it, from 1 (the list is in the icons'
/// order: EXE5's souls, the original's soul numbers), and the form's
/// alternate (`offer_chaos`) the one after the list's; 0, the empty icon,
/// for no form or one none of the navi's lists holds.
fn offer_icon(b: &Battle, side: u8, offer: nettai_battle::rules::Offer) -> usize {
    match form_place(b, side, offer.form) {
        Some((_, len)) if offer.alternate => len + 1,
        Some((place, _)) => place + 1,
        None => 0,
    }
}

/// Where `form` is among side `side`'s navi's forms: its place in the
/// first of the navi's form lists that holds it (from 0), and the list's
/// length.
fn form_place(b: &Battle, side: u8, form: Option<FormHandle>) -> Option<(usize, usize)> {
    let navi = b.stats[side as usize & 1].navi;
    let (place, list) = b.content.navi(navi).forms.as_ref()?.holding(form?)?;
    Some((place, list.len()))
}

/// The flight of the offered form's icon (EXE5's soul choice, its state 9,
/// 0x080232D0: the window whose view is `offer_flight`, at its step and
/// count, `at`): the icon as a 16x16 sprite (sprite palette 13) over the
/// picked column's cell after the picks (0x0802330C: y = 24 + 16 picks,
/// x 0x60), drawn from the tick after it is loaded (0x08023360) through the
/// white flashes, rising 2 pixels a tick for 8 ticks onto the first cell
/// (0x0802337A), whitened by its flash (fades 0x34 and 0x30, the sprite
/// palette's), until the form takes the first cell (0x080233E0).
fn offered_flight<'a>(v: &View<'a>, at: Flight, problems: &mut Problems) -> Option<SpritePart<'a>> {
    let (tiles, first) = offered_icon(v)?;
    let form = v.b.offer(v.side)?.form;
    flight(v, tiles, first, at, form, problems)
}

/// Why a console draws an offered form's flying icon otherwise than the
/// frontend: the game draws every form's in its own version's outline, the
/// frontend each form's in the form's version's (EXE5's souls).
pub const OTHER_VERSIONS_ICON: &str = "an offered form's icon in its own version's outline (the console's ROM draws every one in the console's)";

/// The row of the offer button's icon palettes that is `form`'s own
/// version's, of `rows` (the pack's base version's, then each other
/// version's: `icon_palettes`). A form's version is its place's in the
/// navi's form list that holds it, which lists a version after another in
/// equal runs (EXE5's souls, the original's soul numbers: Team ProtoMan's
/// six, then Team Colonel's): no field restates it. No form, or one none of
/// the navi's lists holds: the base version's.
fn icon_palette_row(b: &Battle, side: u8, form: Option<FormHandle>, rows: usize) -> usize {
    match form_place(b, side, form) {
        Some((place, len)) => version_run(Some(place), len, rows),
        None => 0,
    }
}

/// Which of `versions` equal runs of a list of `len` place `place` (from
/// 0) is in: the version of a form by its place in its navi's list. No
/// place: the first.
fn version_run(place: Option<usize>, len: usize, versions: usize) -> usize {
    let run = len / versions.max(1);
    match place {
        Some(place) if run > 0 => (place / run).min(versions - 1),
        _ => 0,
    }
}

/// EXE5's capsule's mix (its state 0x3C, 0x0802373A: the window whose view
/// is `chip_flight`): the capsule's icon, its chip's (0x08023770: the chip
/// records' icons are the table it loads from, 0x0874A738), flies to the
/// last pick's cell as the soul's icon does to the first (the soul's
/// choice's steps and sprite, 0x080254D8). The chip is the one the flight's
/// button shows: which of the screen's buttons that show a chip, in the
/// slots' order.
fn capsule_flight<'a>(v: &View<'a>, packs: &crate::packs::Packs<'a>, problems: &mut Problems) -> Option<SpritePart<'a>> {
    if window_view(v.b, v.screen) != Some(WindowView::ChipFlight) {
        return None;
    }
    let (button, at) = v.b.chip_flight(v.side)?;
    let mut shown = v.screen.slots.iter().filter_map(|s| match s.kind {
        SlotKind::Button { cell: ButtonCell::Right, .. } => None,
        SlotKind::Button { .. } => s.face,
        _ => None,
    });
    let chip = shown.nth((button as usize).checked_sub(1)?)?;
    let (tiles, _) = crate::lookups::chip_icon(packs, &v.b.content, chip, problems)?;
    // (In the palette of the form the side is in: the capsules' soul's.)
    let form = Some(v.b.stats[v.side as usize & 1].form);
    flight(v, tiles, 0, at, form, problems)
}

/// The sprite of an icon's flight (EXE5's soul's choice and capsule's mix,
/// 0x080254D8): the icon `first` of `tiles`, at the sequence's step and
/// count (`at`), in the icons' palette of the button that offers a form:
/// `form`'s own version's (`icon_palette_row`), where the game draws its
/// console's version's, so on a console of another version (a recording's)
/// it is a known difference.
fn flight<'a>(v: &View<'a>, tiles: &'a Tiles, first: usize, at: Flight, form: Option<FormHandle>, problems: &mut Problems) -> Option<SpritePart<'a>> {
    let screen = v.screen;
    let rise = match at.step {
        4 if at.count > 0 => 0,
        8 => 2 * at.count as i32,
        12 | 16 | 20 => 16,
        _ => return None,
    };
    let b = view_look(v, ButtonView::FormOffer)?;
    // (The form's version's: Team Colonel's souls' outline is another color.)
    let row = icon_palette_row(v.b, v.side, form, 1 + b.icon_palettes.len());
    let colors = row.checked_sub(1).and_then(|i| b.icon_palettes.get(i)).map_or(b.icon_palette, |(_, p)| *p);
    // The row a console of the recording's version draws every icon in.
    let console = v.packs.version().map(|version| b.icon_palettes.iter().position(|(name, _)| name == version).map_or(0, |i| i + 1));
    let f = screen.look.fade;
    let palette = match f.mode {
        nettai_battle::battle::FadeMode::Flash | nettai_battle::battle::FadeMode::FlashBack => {
            let n = (f.level >> 4).min(16) as u8;
            colors.map(|c| crate::compose::apply_fade(c, Fade::White(n)))
        }
        _ => colors,
    };
    let y = 24 + 16 * screen.selection().len() as i32 - rise;
    if console.is_some_and(|console| console != row) {
        problems.known(0x60, y & 0xFF, 16, 16, OTHER_VERSIONS_ICON);
    }
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

impl View<'_> {
    fn icon(&self, c: FolderChip, problems: &mut Problems) -> Option<&Tiles> {
        crate::lookups::chip_icon(&self.packs, &self.b.content, c.id, problems).map(|(icon, _)| icon)
    }

    /// Sprite palette 11, as the second fade record leaves it.
    fn emblem_palette(&self) -> Palette {
        self.faded(self.emblem_palette)
    }

    /// The cursor's and the Regular chip's frame's palette: the screen's
    /// own (EXE4's sprite palette 13, `CustomScreen::cursor_palette`), else
    /// the emblem's (sprite palette 11).
    fn cursor_palette(&self) -> Palette {
        self.assets.cursor_palette.map_or_else(|| self.emblem_palette(), |p| self.faded(p))
    }

    /// A sprite palette of the screen's, as the second fade record leaves
    /// it.
    fn faded(&self, p: Palette) -> Palette {
        match window_fade(self.b) {
            Some(f) => p.map(|c| crate::compose::apply_fade(c, f)),
            None => p,
        }
    }
}

/// A navi's forms of a version of its game: its form list named for the
/// version, in the order its definition lists them (`NaviForms::listed`:
/// EXE6's Crosses), the order of a pack version's names and colors.
pub fn version_forms<'c>(c: &'c Content, navi: NaviHandle, version: &str) -> &'c [FormHandle] {
    c.navi(navi).forms.as_ref().map_or(&[], |f| f.listed(version))
}

/// A form's name pictures and colors in the form list's window, by the
/// form's own version (a Gregar Cross shows Gregar's name in any player's
/// window): its version's custom-screen pictures (the form's `version`) and
/// its number among that version's forms as `navi`, whose form it is, lists
/// them (`version_forms`). Its name is `form_names`' 18 tiles from
/// `18 * number` on the cursor's row (`18 * (number + 5)` on the others'),
/// its colors `form_name_palettes[number]` (`[number + 5]` once used). None: a
/// form of no version, or one the navi doesn't list.
pub fn form_name_picture<'a>(c: &Content, a: &'a CustomScreen, navi: NaviHandle, form: FormHandle) -> Option<(&'a VersionPictures, usize)> {
    let version = c.form(form).version.as_deref()?;
    let number = version_forms(c, navi, version).iter().position(|&f| f == form)?;
    Some((a.versioned.get(version), number))
}

/// The version pictures a side's buttons draw (a button's look, its picture
/// in the chip window, its chip's): those of its navi's form's version when
/// the form isn't the base form and has one (another's than the player's
/// when their Crosses hold one of its), else the player's version's. (EXE6's
/// Beast Out button: the Beast its navi goes into, or is in; rules/beast's
/// rule, docs/engine/custom-screen.md §4.1.)
pub fn button_pictures<'a>(b: &Battle, a: &'a CustomScreen, side: u8) -> &'a VersionPictures {
    let side = side & 1;
    let form = b.content.form(b.stats[side as usize].form);
    let version = match form.version.as_deref() {
        Some(own) if !form.base => Some(own),
        _ => version_name(b, side),
    };
    version.map_or(&a.versioned.base, |v| a.versioned.get(v))
}

/// The version of their game a side's player brought, by the name the
/// game's pack keeps a version's pictures under (`Versioned`):
/// `PlayerFact::Version` (EXE6's "gregar" or "falzar"); none for a game
/// whose players bring none.
pub fn version_name(b: &Battle, side: u8) -> Option<&str> {
    b.fact(side, PlayerFact::Version)?.name()
}

/// The version a side's console is of, by that name: what the frontend
/// says of the console (`Packs::version`: a recording's), else the side's
/// player's (`version_name`), else the base version of the game's pack.
pub fn console_version<'x, 'a: 'x>(b: &'x Battle, packs: &crate::packs::Packs<'a>, side: u8) -> &'x str {
    packs.version().or_else(|| version_name(b, side)).unwrap_or(&packs.game(&b.content).custom.versioned.base_version)
}

/// The emblem's sprite (`sub_8029C08`): 4x4 tiles with a navi's emblem
/// (2x2) in the middle four; empty for a navi with none.
pub fn emblem_sprite(emblem: Option<&nettai_assets::Emblem>) -> Tiles {
    let mut t = Tiles { pixels: vec![0; 16 * Tiles::TILE] };
    for (k, place) in [5usize, 6, 9, 10].into_iter().enumerate() {
        if let Some(src) = emblem.and_then(|e| e.tiles.get(k)) {
            t.pixels[place * Tiles::TILE..(place + 1) * Tiles::TILE].copy_from_slice(src);
        }
    }
    t
}

/// Why a navi's emblem isn't the one the console shows, if it isn't: a
/// console has no emblem for another version's link navi (a navi that says
/// its `version`) and shows its own counterpart's picture in the navi's
/// colors; the pack has every navi's own (deliberately: docs/frontend.md).
fn known_emblem(b: &Battle, packs: &crate::packs::Packs, side: u8) -> Option<&'static str> {
    let navi = b.content.navi(b.stats[side as usize & 1].navi);
    let console = console_version(b, packs, side);
    navi.version.as_deref().is_some_and(|v| v != console).then_some("another version's link navi's emblem (its own)")
}

/// The window's map, the tiles and the palettes it draws with.
struct Window {
    map: [MapEntry; COLUMNS * ROWS],
    tiles: LayerTiles,
    palettes: [Palette; 16],
    /// Why the chip window's picture isn't the one the console shows, if
    /// it isn't: a known difference (`known_picture`).
    picture_known: Option<&'static str>,
    /// The chip window's picture's mark, where the caller names its chip
    /// (`chip:KEY`, `Problems::marking`).
    picture_mark: Option<String>,
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

/// The tick of a form's choice the white fade is over and the form put
/// on (EXE6's Cross, `sub_8027AAE`; rules/cross's `PUT_ON_TICK`): the
/// window's map is the chips' again.
const FORM_PUT_ON_TICK: u16 = 25;

/// Where a form list's window is (EXE6's Cross window: the windows whose
/// views are `form_list_opening`, `form_list`, `form_list_closing` and
/// `form_chosen`): the one up (its tick), or a description from it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormListStage {
    /// `sub_8027834`, 12 ticks.
    Opening(u16),
    /// `sub_802794A`, or a form's description from it.
    Up,
    /// `sub_802790C`, 6 ticks.
    Closing(u16),
    /// `sub_8027A58`, 34 ticks.
    Chosen(u16),
}

/// The stage of the form list's window on screen `s`, if one is up.
pub fn form_list_stage(b: &Battle, s: &Screen) -> Option<FormListStage> {
    let (window, tick) = match s.phase {
        Phase::Window { window, tick } => (window, tick),
        Phase::Description { window: Some(window), .. } => (window, 0),
        _ => return None,
    };
    Some(match b.content.defs.window(window).view? {
        WindowView::FormListOpening => FormListStage::Opening(tick),
        WindowView::FormList => FormListStage::Up,
        WindowView::FormListClosing => FormListStage::Closing(tick),
        WindowView::FormChosen => FormListStage::Chosen(tick),
        WindowView::OfferFlight | WindowView::ChipFlight => return None,
    })
}

/// The form list window's map the screen shows, if it shows one: its
/// opening steps every 3 ticks (`sub_8027834`), then the window with its
/// forms, until it closes (`sub_802790C`, 5 ticks) or the form chosen is
/// put on (`sub_8027AAE`).
fn form_list_map(v: &View) -> Option<usize> {
    let stage = form_list_stage(v.b, v.screen)?;
    let count = v.b.form_list(v.side).map_or(0, |w| w.count);
    let full = FORM_LIST_OPENING_MAPS + count.max(1) as usize - 1;
    match stage {
        FormListStage::Opening(tick) if tick >= 3 => Some(tick as usize / 3 - 1),
        FormListStage::Up | FormListStage::Closing(_) => Some(full),
        FormListStage::Chosen(tick) if tick < FORM_PUT_ON_TICK => Some(full),
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
            picture_mark: None,
            name: None,
            advance_names: Vec::new(),
            layout: a.layout,
        };
        // sub_8026840: the window's map, with the form list's tab (EXE6's
        // Cross tab) or without; or the form list window's.
        let form_list = form_list_map(v);
        let (map, patches) = match form_list {
            Some(i) => (a.form_list_maps.get(i), &a.form_list_patches),
            // (A game without the tab has one map: EXE5.)
            None => (a.window_maps.get(v.screen.look.form_list_tab as usize).or(a.window_maps.first()), &a.window_patches),
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
        w.window_emblem(v);
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
        if form_list.is_some_and(|i| i >= FORM_LIST_OPENING_MAPS) {
            w.form_names(v, problems);
        }
        if v.screen.look.turn_limit {
            // sub_8029D34: "FINAL TURN", 7x2 at column 15, row 4 (past the
            // window's columns: drawn on the layer apart).
        }
        w
    }

    /// The screen's own emblem on the window's map (EXE4's orb, 0x08020028),
    /// in the frame its turn's step shows: the step the emblem was drawn
    /// with this tick, else the screen's (0 at rest).
    fn window_emblem(&mut self, v: &View) {
        let Some(e) = &v.assets.window_emblem else { return };
        self.tiles.put(e.first_tile, &e.tiles);
        let step = v.screen.look.drawn.emblem.map_or(v.screen.look.spin, |(_, spin)| spin);
        let Some(frame) = e.frames.get(e.frame(step as usize)) else { return };
        for (k, entry) in frame.iter().enumerate() {
            let (x, y) = (e.x as usize + k % e.width as usize, e.y as usize + k / e.width.max(1) as usize);
            if x < COLUMNS && y < ROWS {
                self.map[y * COLUMNS + x] = *entry;
            }
        }
    }

    /// A button's cells at its own place on the window's map
    /// (`ButtonPictures::place`: EXE4's UNITE button, 0x0801FF14), its set
    /// `set`'s tiles from the place's first tile.
    fn placed_button(&mut self, b: &ButtonPictures, p: nettai_assets::ButtonPlace, set: usize) {
        let n = b.width as usize * b.height as usize;
        self.tiles.put_part(p.first_tile, &b.tiles, n * set, n);
        for k in 0..n {
            let (x, y) = (p.x as usize + k % b.width as usize, p.y as usize + k / b.width.max(1) as usize);
            if x < COLUMNS && y < ROWS {
                self.map[y * COLUMNS + x] = MapEntry { tile: p.first_tile + k as u16, hflip: false, vflip: false, palette: p.palette };
            }
        }
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

    /// A pick's name and code into name `k`'s tiles. (The original leaves
    /// the code off a chip numbered 0x160 or more, `sub_802B80C`, EXE5's
    /// 0x08027BC6: no recipe of either game names one.)
    fn put_advance_name(&mut self, v: &View, k: usize, c: FolderChip, text: &TextSink, problems: &mut Problems) {
        self.put_advance_text(v, k, c.id, text.strings.chip_name(&v.b.content, c.id), Some(c.code.0), text, problems);
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

    /// `sub_802794A`: the offered forms' names (`sub_8029D94`: the one
    /// under the cursor in its own look) over the form list window's map,
    /// and palette 10 the form under the cursor's (`sub_8029EAC`: a used
    /// one's darker).
    fn form_names(&mut self, v: &View, problems: &mut Problems) {
        let Some(w) = v.b.form_list(v.side) else { return };
        // Each form's name and colors are its own version's (a player's
        // Crosses can hold another's: docs/engine/custom-screen.md §4.1).
        let navi = v.b.stats[v.side as usize].navi;
        let mut picture = |slot: usize| crate::lookups::form_name(v.assets, &v.b.content, navi, w.forms[slot]?, problems);
        for slot in 0..w.count.min(5) as usize {
            let Some((own, number)) = picture(slot) else { continue };
            let name = number + if slot == w.cursor as usize { 0 } else { 5 };
            let at = self.layout.form_names + (FORM_NAME_TILES * slot) as u16;
            self.tiles.put_part(at, &own.form_names, FORM_NAME_TILES * name, FORM_NAME_TILES);
            for i in 0..FORM_NAME_TILES {
                let (x, y) = (1 + i % 9, 1 + 2 * slot + i / 9);
                self.map[y * COLUMNS + x] = MapEntry { tile: at + i as u16, hflip: false, vflip: false, palette: 10 };
            }
        }
        let c = w.cursor as usize;
        if let Some((own, number)) = picture(c) {
            let index = number + if w.marked[c] { 5 } else { 0 };
            if let Some(p) = own.form_name_palettes.get(index) {
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
            w.tiles.fill(w.layout.code, 2, w.layout.detail_blank);
            w.blank_element(a);
            w.tiles.fill(w.layout.digits, 6, w.layout.detail_blank);
        };
        match v.screen.slots[slot as usize].kind {
            SlotKind::Chip { .. } | SlotKind::Offered(_) => {
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
                    self.tiles.fill(self.layout.code, 2, self.layout.detail_blank);
                    self.blank_element(a);
                    self.tiles.fill(self.layout.digits, 6, self.layout.detail_blank);
                } else if let Some(look) = v.button_look(button) {
                    // (EXE5's soul button, 0x08024540: its picture in its
                    // first palette, a Chaos Unison's in its second, the
                    // slot's +6, whatever its state.)
                    let palette = if offers_form(v.b, v.screen, cw.slot) {
                        v.b.offer(v.side).map_or(0, |o| o.alternate as usize)
                    } else {
                        0
                    };
                    blank_details(self, &look.details(palette));
                    if look.pack.uses_digit {
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

    /// `sub_802869E`'s element cells, blank (a screen whose element icon is
    /// a sprite has none: EXE4's, `CustomScreen::element_sprite`).
    fn blank_element(&mut self, a: &CustomScreen) {
        if a.element_sprite.is_none() {
            self.tiles.fill(self.layout.element, 4, BLANK_7);
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
        self.tiles.fill(self.layout.digits, 2 * blanks, self.layout.detail_blank);
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
        // (The button chip's picture is its button's: EXE6's BeastOut's, the
        // Beast's the navi goes into.)
        let beast_out = v.b.roles().try_chip(nettai_battle::content::ChipRole::ButtonChip) == Some(c);
        // A chip whose palette no ROM holds has its definition's
        // (`art_palette`). A version's own chip's picture is its own ROM's:
        // a console of the other version shows its counterpart's. (The
        // picture is marked where the caller names its chip: `chip:KEY`.)
        let art = if beast_out {
            view_look(v, ButtonView::ChipPicture).map(|b| (&b.picture, None))
        } else {
            crate::lookups::chip_art(&v.packs, &v.b.content, c, problems).map(|art| (&art.picture, Some(art)))
        };
        if let Some((p, art)) = art {
            self.tiles.put(self.layout.art, &p.tiles);
            self.palettes[10] = if beast_out { p.palette } else { data.art_palette.unwrap_or(p.palette) };
            let console_version = console_version(v.b, &v.packs, v.side);
            self.picture_known = match art {
                Some(a) if a.version.as_deref().is_some_and(|g| g != console_version) => Some(crate::lookups::OTHER_VERSIONS_ART),
                _ => None,
            };
            let what = format!("chip:{}", nettai_content_api::keys::local(&v.b.content.defs.chip(c).key));
            if art.is_some() && problems.wants(&what) {
                self.picture_mark = Some(what);
            }
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
                SlotKind::Chip { .. } | SlotKind::Offered(_) => {
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
                    } else if crate::lookups::button(a, v.buttons, &v.b.content, button, problems).is_some()
                        && let Some(look) = v.button_look(button)
                    {
                        match look.pack.place {
                            // (At its own place: EXE4's UNITE button.)
                            Some(p) => self.placed_button(look.pack, p, look.set(state) / look.count.max(1)),
                            // (The slots after it start past its tiles; the
                            // special slot is the last.)
                            None => {
                                self.tiles.put_part(at, &look.pack.tiles, look.set(state), look.count);
                                at += look.count as u16;
                            }
                        }
                    }
                }
                SlotKind::Empty => {
                    self.tiles.put(at, &a.empty_icon);
                    self.tiles.put_part(at + 4, &a.slot_codes, 2 * EMPTY_SLOT_CODE as usize, 2);
                    at += 6;
                }
                // (The special slot's button's hidden look, if it has one;
                // else the window's fill, as a hidden slot's.)
                SlotKind::Hidden if s as u8 == SPECIAL_SLOT => match v.hidden_special() {
                    Some((b, set)) => match b.place {
                        Some(p) => self.placed_button(b, p, set),
                        None => {
                            let n = b.width as usize * b.height as usize;
                            self.tiles.put_part(at, &b.tiles, n * set, n)
                        }
                    },
                    None => self.tiles.fill(at, 6, self.layout.slot_blank),
                },
                SlotKind::Hidden => {
                    self.tiles.fill(at, 6, self.layout.slot_blank);
                    at += 6;
                }
            }
        }
        // The icons' palettes (the 2x2 icon cells of slots 0-9).
        for s in 0..10usize {
            let slot = v.screen.slots[s];
            // (The empty icon in the pack's palette for it: EXE4's 9.)
            let empty = self.layout.empty_palette.unwrap_or(11);
            let palette = match slot.kind {
                SlotKind::Empty => empty,
                SlotKind::Chip { .. } | SlotKind::Offered(_) if slot.state == SlotState::Unavailable => 12,
                SlotKind::Chip { .. } | SlotKind::Offered(_) if v.screen.look.slot_picked[s] => empty,
                SlotKind::Chip { .. } | SlotKind::Offered(_) => 11,
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
                // The offered form, given for a chip: its icon (EXE5's
                // soul's, 0x0802341C).
                None if picks.get(i) == Some(&SPECIAL_SLOT) => offered_icon(v),
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
            // (An empty cell's icon in the empty icon's palette, where the
            // pack has one: EXE4's 0x0801FB6E.)
            if let Some(p) = self.layout.empty_palette.filter(|_| filled == 0) {
                for (x, y) in [(12, 3 + 2 * i), (13, 3 + 2 * i), (12, 4 + 2 * i), (13, 4 + 2 * i)] {
                    self.map[y * COLUMNS + x].palette = p;
                }
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
            let shown = matches!(slot.kind, SlotKind::Chip { .. } | SlotKind::Offered(_)) && !v.screen.look.slot_picked[s];
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
        if let Some([x0, y0, x1, y1]) = self.tiles_rect(place, tiles) {
            problems.known(x0, y0, x1 - x0, y1 - y0, why);
        }
    }

    /// Where the window's cells that show `tiles` of its block are, on the
    /// screen (none: none shows).
    fn tiles_rect(&self, place: Placement, tiles: std::ops::Range<u16>) -> Option<[i32; 4]> {
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
        let [x0, y0, x1, y1] = rect?;
        let (x0, x1) = (x0.max(0), x1.min(240));
        (x0 < x1).then_some([x0, y0, x1, y1])
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
/// its two frames (`byte_80288B0` and the others: y, x, flips). OK's and a
/// button's are the pack's (`CustomLayout::ok_cursor`,
/// `ButtonPictures::cursor`: EXE5's sit otherwise).
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

/// The cursor at a place the pack gives (OK's, a button's).
fn cursor_at(p: &nettai_assets::CursorPlace) -> (i32, i32, CursorShape) {
    let corners = p.corners.map(|frame| frame.map(|(y, x, h, v)| (y as i32, x as i32, h, v)));
    (p.x as i32, p.y as i32, CursorShape { corners })
}

/// `sub_80289E4`: the form list window's cursor, a box around the form
/// under it: four corners, then seven edge pieces above and below
/// (`byte_8028A30`: y, x, flips), in sprite palette 14.
fn form_list_cursor_parts<'a>(v: &View, a: &'a CustomScreen, frame: u8) -> Vec<SpritePart<'a>> {
    let cursor = v.b.form_list(v.side).map_or(0, |w| w.cursor);
    let (x, y) = (5, 5 + 16 * cursor as i32);
    let corners = [(2, 3, false, false), (2, 0x43, true, false), (0xC, 0x43, true, true), (0xC, 3, false, true)];
    let edges = (0..7).map(|i| (2, 0xB + 8 * i, false, false)).chain((0..7).map(|i| (0xC, 0xB + 8 * i, false, true)));
    let pieces = corners.into_iter().map(|c| (c, 0)).chain(edges.map(|e| (e, 1)));
    let palette = a.form_list_cursor_palette;
    pieces
        .map(|((dy, dx, hflip, vflip), edge)| SpritePart {
            x: ((x + dx) & 0x1FF) as u16,
            y: (y + dy) as u8,
            width: 8,
            height: 8,
            tiles: &a.form_list_cursor,
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
        SlotKind::Chip { .. } | SlotKind::Offered(_) | SlotKind::Empty | SlotKind::Hidden => {
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            (16 * col + 8, 0x68 + 0x18 * row, CHIP_CURSOR)
        }
        SlotKind::Ok => cursor_at(&a.layout.ok_cursor),
        // (A button that shows a chip: a chip's cursor, 0x080246B8.)
        SlotKind::Button { .. } if s.slots[slot as usize].face.is_some() => {
            let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
            (16 * col + 8, 0x68 + 0x18 * row, CHIP_CURSOR)
        }
        // (A button the pack has no look for: a chip's cursor on its slot.)
        SlotKind::Button { button, .. } => match v.button_look(button) {
            Some(look) => look.cursor(),
            None => {
                let (col, row) = ((slot % 5) as i32, (slot / 5) as i32);
                (16 * col + 8, 0x68 + 0x18 * row, CHIP_CURSOR)
            }
        },
    };
    let palette = v.cursor_palette();
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

/// The chip window's element icon as a sprite (EXE4's, 0x0801EECC: 16x16
/// in its own palette, at the window's left edge less its scroll while the
/// scroll is 40 or less), while the cursor is on a chip, from the screen's
/// open on (0x0801E314, 0x08020DB4): the last chip's element's.
fn element_part<'a>(v: &View, a: &'a CustomScreen, place: Placement) -> Option<SpritePart<'a>> {
    let e = a.element_sprite?;
    let s = v.screen;
    if !matches!(s.slots[s.cursor as usize].kind, SlotKind::Chip { .. } | SlotKind::Offered(_)) || place.scroll > ELEMENT_SHOWN_TO {
        return None;
    }
    let c = s.look.chip_window.last_chip?;
    let family = v.b.content.chip(c.id).family as usize;
    if a.elements.len() < 4 * (family + 1) {
        return None;
    }
    Some(SpritePart {
        x: ((e.x as i32 - place.scroll as i32) & 0x1FF) as u16,
        y: e.y as u8,
        width: 16,
        height: 16,
        tiles: &a.elements,
        first_tile: 4 * family,
        hflip: false,
        vflip: false,
        palette: v.faded(e.palette),
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    })
}

/// The window's scroll past which the element sprite isn't drawn
/// (0x0801EECC).
const ELEMENT_SHOWN_TO: u32 = 40;

/// The Regular chip's frame (`sub_802899C`): a 32x32 sprite around the
/// first slot.
/// The chip a button holds, over the button (EXE5's Arm Change, 0x080254F4):
/// its icon as a 16x16 sprite where the button's look says, in sprite
/// palette 10 (the HUD's icons').
fn held_part<'a>(v: &View, packs: &crate::packs::Packs<'a>, problems: &mut Problems) -> Option<SpritePart<'a>> {
    let h = v.screen.hold?;
    let SlotKind::Button { button, .. } = v.screen.slots[h.button as usize].kind else { return None };
    let (x, y) = v.button_look(button)?.pack.held_at.map(|(x, y)| (x as i32, y as i32))?;
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
        palette: v.cursor_palette(),
        priority: 1,
        alpha: None,
        mosaic: None,
        vscale: None,
        affine: None,
    }
}

/// Draw the local player's custom screen: the window on the HUD layer, the
/// enemy names on `names_layer` (BG0), the sprites into `list`.
/// `emblem` holds the emblem sprite's tiles (`emblem_sprite`) and
/// `emblem_palette` the emblem's palette (`lookups::emblem`); the font mode's
/// strings go to `text`.
#[allow(clippy::too_many_arguments)]
pub fn draw<'a>(
    b: &'a Battle,
    assets: &'a Bundle,
    packs: &crate::packs::Packs<'a>,
    emblem: &'a Tiles,
    emblem_palette: Palette,
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
        buttons: button_pictures(b, a, side),
        emblem_palette,
        hud: &assets.hud,
        packs: packs.clone(),
    };
    let place = placement(screen);
    let mut w = Window::build(&v, text, problems);
    let advance_names = w.program_advance(&v, text, problems);
    w.draw(hud_layer, place);
    if let Some(why) = w.picture_known {
        w.known_picture(place, why, problems);
    }
    if let Some(what) = w.picture_mark.take() {
        let tiles = w.layout.art..w.layout.art + PICTURE_TILES;
        if let Some(rect) = w.tiles_rect(place, tiles) {
            problems.mark(rect, what);
        }
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
    // (A screen without the names' bar draws no names: EXE4's.)
    if names_shown(b, screen) && !a.name_bar.is_empty() {
        draw_names(&v, &w, hud_layer, names_layer, text, problems);
    }
    // The sprites, as their routines queue them (each in front of the
    // one before).
    let drawn = screen.look.drawn;
    let mut queue: Vec<SpritePart<'a>> = Vec::new();
    // The offered form's flying icon (EXE5's soul choice's: its state 9's
    // routines draw it before the screen's others).
    if window_view(b, screen) == Some(WindowView::OfferFlight)
        && let Some(at) = b.offer_flight(v.side)
    {
        queue.extend(offered_flight(&v, at, problems));
    }
    // Its capsule's mix's (state 0x3C's).
    queue.extend(capsule_flight(&v, packs, problems));
    if let Some(frame) = drawn.cursor {
        queue.extend(cursor_parts(&v, a, frame));
    }
    queue.extend(element_part(&v, a, place));
    // (A screen with an emblem of its own draws it on the window's map.)
    if let Some((x, spin)) = drawn.emblem.filter(|_| a.window_emblem.is_none()) {
        let part = emblem_part(&v, emblem, x, spin);
        if let Some(why) = known_emblem(b, packs, side) {
            // (Where the sprite is, its coordinates wrapped as the
            // hardware's.)
            let (x, y) = (part.x as i32, part.y as i32);
            problems.known(if x >= 240 { x - 512 } else { x }, if y >= 160 { y - 256 } else { y }, 32, 32, why);
        }
        queue.push(part);
    }
    if drawn.regular {
        queue.push(regular_part(&v, a));
    }
    if drawn.held {
        queue.extend(held_part(&v, packs, problems));
    }
    if let Some(frame) = drawn.form_list_cursor {
        queue.extend(form_list_cursor_parts(&v, a, frame));
    }
    for part in queue {
        list.insert_at(SPRITE_LAYER, 0, vec![part]);
    }
    let _ = OK_SLOT;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A form's version by its place in its navi's list: of EXE5's twelve
    /// souls, Team ProtoMan's six (the pack's base version's palette, row
    /// 0), then Team Colonel's (row 1); no form, the base's; a pack with
    /// one version's palette alone, that one.
    #[test]
    fn a_forms_version_is_its_places_run() {
        let rows: Vec<usize> = (0..12).map(|place| version_run(Some(place), 12, 2)).collect();
        assert_eq!(rows, [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1]);
        assert_eq!((version_run(None, 12, 2), version_run(Some(11), 12, 1), version_run(Some(3), 0, 2)), (0, 0, 0));
        // (A list that doesn't divide evenly: the last run takes the rest.)
        assert_eq!(version_run(Some(12), 13, 2), 1);
    }

    #[test]
    fn the_window_slides_in_and_out_a_column_or_two_a_tick() {
        let ins: Vec<usize> = (1..=10).map(copied_in).collect();
        assert_eq!(ins, [2, 3, 5, 6, 8, 9, 11, 12, 14, 15]);
        let outs: Vec<usize> = (1..=10).map(cleared_out).collect();
        assert_eq!(outs, [1, 3, 4, 6, 7, 9, 10, 12, 13, 15]);
    }
}
