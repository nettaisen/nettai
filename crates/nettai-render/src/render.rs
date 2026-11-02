//! Drawing a battle: everything visible is derived from the engine's state
//! and the game's graphics.

use crate::audit::Problems;
use crate::compose::{self, Fade, Fades, Layer, Palettes, SpritePart};
use crate::custom::Layers;
use crate::smooth::{Lift, Nudge, Overlay, Smooth};
use crate::hud::HudState;
use crate::objects::{self, SpriteList, View};
use crate::graphics::GameGraphics;
use crate::seat::{Consoles, Seat, Shows};
use crate::stage::{Stage, StageClock};
use crate::textlayer::{Plane, Rect, TextItem, TextSink};
use crate::vfont::VectorFont;
use nettai_assets::Bundle;
use nettai_battle::Battle;
use nettai_battle::battle::{FadeMode, mode};
use nettai_battle::content::PaletteFlashOrder;
use nettai_battle::transform::{SequencerState, TransformPhase};
use std::sync::Arc;

/// Which of the graphics enhancements a battle is drawn with (the app's
/// "Graphics enhancements"). Each draws something the original draws more
/// finely, and each can be on or off alone; with none of them
/// ([`Enhancements::NONE`], the default) the frame is exactly the
/// original's: the reference the frame comparison and the tests use.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Enhancements {
    /// Sharp text: the strings (chip names, the telops, the chatbox's
    /// descriptions and messages, the enemy names) in a vector font at the
    /// output's resolution, on a layer over the scaled frame
    /// (docs/design/text-rendering.md). Off, the game's own fonts draw them
    /// into the frame (where the game's graphics have no fonts of the
    /// language shown, pixel fonts: `pixeltext`).
    pub text: bool,
    /// Sharp HUD: the pictures' words in the lettering fonts (`lettering`),
    /// the numbers in the numbers' font (`numbers`), and the frames the
    /// game's graphics have drawings of drawn as shapes (`shapes`), at the
    /// output's resolution.
    pub hud: bool,
    /// Smooth motion: what the original moves in whole pixels or coarse
    /// steps drawn between them at the output's resolution (`smooth`, the
    /// background's glide: `Frame::glide`).
    pub motion: bool,
    /// Fill the window: the frame as large as `Renderer::size` asks, the
    /// HUD's pieces kept to its edges; off, the original's 240 by 160.
    pub fill: bool,
}

impl Enhancements {
    /// The original's picture exactly.
    pub const NONE: Enhancements = Enhancements { text: false, hud: false, motion: false, fill: false };
    /// Every enhancement.
    pub const ALL: Enhancements = Enhancements { text: true, hud: true, motion: true, fill: true };
    /// The enhancements by name, as `FromStr` reads them.
    pub const NAMES: [&str; 4] = ["text", "hud", "motion", "fill"];

    /// Whether the frame has anything finer than its own pixels to draw
    /// (text, shapes, motion between them), so is worth presenting at more
    /// than one output pixel a frame pixel.
    pub fn fine(self) -> bool {
        self.text || self.hud || self.motion
    }

    /// Whether the vector font is wanted: for the text, and for the HUD's
    /// lettering (its fonts are the text font's cut's).
    pub fn wants_font(self) -> bool {
        self.text || self.hud
    }
}

impl std::str::FromStr for Enhancements {
    type Err = String;

    /// "original" (none), "enhanced" (all; "font" too: the tools' `--text`
    /// named it so first), or some of `NAMES` joined by commas
    /// ("text,hud").
    fn from_str(s: &str) -> Result<Enhancements, String> {
        match s {
            "original" | "none" => return Ok(Enhancements::NONE),
            "enhanced" | "font" | "all" => return Ok(Enhancements::ALL),
            _ => {}
        }
        let mut e = Enhancements::NONE;
        for name in s.split(',').map(str::trim) {
            match name {
                "text" => e.text = true,
                "hud" => e.hud = true,
                "motion" => e.motion = true,
                "fill" => e.fill = true,
                _ => return Err(format!("no graphics enhancement {name:?} (original, enhanced, or some of {} joined by commas)", Enhancements::NAMES.join(", "))),
            }
        }
        Ok(e)
    }
}

/// A drawn frame: the picture (BGR555), the original's 240 by 160 or more
/// ([`Frame::size`], `Renderer::size`), per pixel the depth key of what won
/// it (`compose::depth_key`), and the text items to draw over it at the
/// output's resolution (none without the sharp text or HUD).
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// Its columns and rows.
    pub size: compose::Size,
    pub pixels: Vec<u16>,
    pub depth: Vec<u32>,
    pub text: Vec<TextItem>,
    /// The background at the part of a pixel it has scrolled past the
    /// frame's whole pixels (the smooth motion): `present` shows
    /// it where the frame shows the background.
    pub glide: Option<Glide>,
    /// The frame drawn again with the custom gauge's fill a pixel longer
    /// (the smooth motion), for `present` to show between the two.
    pub nudge: Option<crate::smooth::Nudge>,
    /// The frame drawn again with the field layer a pixel up, where the
    /// camera's Y is between whole pixels (the smooth motion: the
    /// custom screen's slide), for `present` to show between the two.
    pub lift: Option<crate::smooth::Lift>,
    /// The squashed and turned sprites the smooth motion draws at
    /// the output's resolution (left out of `pixels`), front first.
    pub overlays: Vec<crate::smooth::Overlay>,
    /// The drawings (`Bundle::drawings`), for the text layer to have
    /// them rasterized before it draws the first of them (`present`).
    pub drawings: Option<Arc<Vec<nettai_assets::shapes::Drawing>>>,
}

impl Frame {
    /// Its columns.
    pub fn width(&self) -> usize {
        self.size.width
    }

    /// Its rows.
    pub fn height(&self) -> usize {
        self.size.height
    }
}

/// A background between two of the frame's pixels: `present` draws the
/// frame's background pixels from `pixels` (`width` by [`Glide::height`],
/// from the frame's top left, faded as the frame's are) moved on by `sub`
/// sixteenths of a pixel, at the output's resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Glide {
    pub pixels: Vec<u16>,
    /// Its columns: a pixel more than the frame each way, for the part of
    /// one past it.
    pub width: usize,
    pub sub: (u8, u8),
    /// The background layer's depth key (`compose::layer_depth`): the
    /// frame's pixels it won, and the backdrop's, show the glide.
    pub depth: u32,
}

impl Glide {
    /// Its rows: the frame's and one.
    pub fn height(&self) -> usize {
        self.pixels.len() / self.width.max(1)
    }
}

/// Picks battles; keeps its layer buffers between frames. It owns what it
/// draws from (the game's graphics are shared), so it borrows nothing.
pub struct Renderer {
    /// The game's graphics: its assets (docs/design/rules-in-luau.md §7.4),
    /// the HUD's and the custom screen's frames, and the other games'
    /// backgrounds.
    graphics: GameGraphics,
    background: Layer,
    field: Layer,
    hud: Layer,
    /// BG0, in front of everything: the custom screen's enemy names.
    names: Layer,
    /// On a screen of both, the right player's console's HUD layer and BG0
    /// (`Consoles::Both`): its pieces, shaken and faded as its console
    /// shakes and fades them.
    hud_right: Layer,
    names_right: Layer,
    /// BG0's chatbox, apart for its palette (background palette 15: the
    /// custom screen's fades of the window's palettes leave it be).
    dialogue: Layer,
    /// Rolling HUD numbers, the local console's; follow every tick with
    /// `observe`.
    pub hud_state: HudState,
    /// The other console's, followed alike: what a screen of both draws of
    /// it ([`Consoles::Both`]), and what a host shows of it at once.
    pub other_hud: HudState,
    /// Which consoles a frame shows: the local side's screen (the default),
    /// or both players' on one screen, the local side's on the left.
    pub consoles: Consoles,
    /// Layers a piece is laid out in before it is put in its place (the
    /// right seat's chip name, its custom screen's window and names).
    scratch: Layer,
    scratch_names: Layer,
    /// What the frames drawn so far named that the game's assets don't have.
    pub problems: Problems,
    /// What the last frame's sprite draw took of its budget and left out
    /// (`objects::SpriteDraw`).
    pub sprite_drawn: objects::SpriteDrawn,
    /// What a pointer finds where on the last frame's custom screens: each
    /// seat's that shows its own console's as it is, by side, in the frame's
    /// pixels (`Renderer::target_at`).
    targets: Vec<(u8, nettai_battle::custom::layer::Target, Rect)>,
    /// The version of the console whose screen is drawn, as its game's
    /// pack names its versions (EXE5's "protoman", "colonel"), for a game
    /// whose versions the engine doesn't tell apart: which chips are the
    /// other version's, whose art its ROM draws otherwise
    /// (`ChipArt::version`), and the soul icon's outline. None: the
    /// version the console's player brought, or the game's base version
    /// (`custom::console_version`).
    pub console_version: Option<&'static str>,
    /// The enhancements the battle is drawn with, and the font of the sharp
    /// text and the sharp HUD's lettering.
    enhancements: Enhancements,
    font: Option<Arc<VectorFont>>,
    /// The font's layouts, for what the frame places after a string (a
    /// chip's damage after its name) and where a lettering's ink is.
    measure: Option<std::cell::RefCell<crate::vfont::TextRenderer>>,
    /// What the frame draws strings with without the sharp text where the game's graphics have no
    /// lettering in the language shown (`render`): the pixel fonts in that
    /// language's cut, made when first needed.
    stand_in: Option<std::cell::RefCell<crate::pixeltext::PixelText>>,
    /// The player's language's strings table (`--lang`), if not the
    /// content's own.
    strings: Option<Arc<nettai_content::locale::Strings>>,
    /// Make only the frame's lookups (`--audit`): nothing is drawn or
    /// composed, and `render` returns an empty frame.
    lookups_only: bool,
    /// How far the stage's scenery (the background's animation and scroll,
    /// the panels' own animations) is drawn ahead of the battle's clock: the
    /// frames it moved on while a host held the battle where it stands
    /// ([`Renderer::scenery_frame`]). Kept after the hold, so nothing jumps
    /// back; forgotten as a round or a battle starts ([`Renderer::reset`]).
    /// Drawing alone: the battle never sees it. None (zero) for a battle
    /// never held, whose frames are the clock's alone.
    scenery: StageClock,
    /// The HUD draws the custom gauge where the game does (the default);
    /// off, it is left out, full or filling, and nothing else of the frame
    /// changes: a host's picture of a fight no one plays (Practice's
    /// arena preview).
    pub custom_gauge: bool,
    /// The drawings, gathered as they are set.
    drawings: Arc<Vec<nettai_assets::shapes::Drawing>>,
    /// How large the frames are with the window filled (the original's 240
    /// by 160, the default, or as large as a window shows them): without it
    /// the original's, whatever this says.
    pub size: compose::Size,
}

impl Renderer {
    /// A renderer of a game's graphics `assets` alone, with no enhancements.
    pub fn new(assets: Arc<Bundle>) -> Renderer {
        Renderer::with_graphics(GameGraphics::new(assets))
    }

    /// A renderer of a game's graphics, with no enhancements.
    pub fn with_graphics(graphics: GameGraphics) -> Renderer {
        let drawings = graphics.drawings();
        Renderer {
            drawings,
            graphics,
            // Background priority 3 (BG1), field 2 (BG2), HUD 1 (BG3: the
            // game's `hud_priority`, set each frame).
            background: Layer::new(3, 1),
            field: Layer::new(2, 2),
            hud: Layer { palettes: Palettes::Hud, ..Layer::new(1, 3) },
            names: Layer { palettes: Palettes::Hud, ..Layer::new(0, 0) },
            hud_right: Layer { palettes: Palettes::OtherHud, ..Layer::new(1, 3) },
            names_right: Layer { palettes: Palettes::OtherHud, ..Layer::new(0, 0) },
            dialogue: Layer { palettes: Palettes::Dialogue, ..Layer::new(0, 0) },
            hud_state: HudState::default(),
            other_hud: HudState::default(),
            consoles: Consoles::One,
            scratch: Layer { palettes: Palettes::Hud, ..Layer::new(1, 3) },
            scratch_names: Layer { palettes: Palettes::Hud, ..Layer::new(0, 0) },
            problems: Problems::default(),
            sprite_drawn: objects::SpriteDrawn::default(),
            targets: Vec::new(),
            console_version: None,
            enhancements: Enhancements::NONE,
            font: None,
            measure: None,
            stand_in: None,
            strings: None,
            lookups_only: false,
            scenery: StageClock::default(),
            custom_gauge: true,
            size: compose::Size::ORIGINAL,
        }
    }

    /// From now on make only the lookups a frame makes (`--audit`): what
    /// the frame names is asked of the game's graphics and the content as drawing
    /// asks it, and no pixel is drawn.
    pub fn set_lookups_only(&mut self, on: bool) {
        self.lookups_only = on;
    }

    /// Show content's display text from a language's strings table (the
    /// content's own strings where it has none); `None`, the content's
    /// own.
    pub fn set_strings(&mut self, strings: Option<Arc<nettai_content::locale::Strings>>) {
        self.strings = strings;
        // (The pixel fonts are the language's cut: made again for the new one.)
        self.stand_in = None;
        self.problems.recheck();
    }

    /// The language's strings table it shows the content's text from
    /// (`None`: the content's own).
    pub fn strings(&self) -> Option<&Arc<nettai_content::locale::Strings>> {
        self.strings.as_ref()
    }

    /// Draw with `enhancements` from now on. With the sharp text, the
    /// strings `font` has go to the text layer (without a font they are drawn
    /// as the original draws them); with the sharp HUD, the pictures' words
    /// the lettering fonts (`font`'s cut's) have, the numbers and the frames
    /// with drawings. Without the sharp text every string is drawn into the
    /// frame in the game's graphics' fonts, and with graphics that have no
    /// lettering in the language shown (their fonts are another language's)
    /// in the pixel fonts, at the frame's resolution (`render`, `pixeltext`).
    pub fn set_enhancements(&mut self, enhancements: Enhancements, font: Option<Arc<VectorFont>>) {
        self.enhancements = enhancements;
        self.measure = font.clone().map(|f| std::cell::RefCell::new(crate::vfont::TextRenderer::new(f)));
        self.font = font;
        self.stand_in = None;
    }

    pub fn enhancements(&self) -> Enhancements {
        self.enhancements
    }

    /// Whether the frames draw what the original moves between its whole
    /// pixels and steps (the smooth motion).
    pub fn motion(&self) -> bool {
        self.enhancements.motion
    }

    /// How large the frames are: with the window filled as `size` says,
    /// else the original's.
    pub fn frame_size(&self) -> compose::Size {
        if self.enhancements.fill { self.size } else { compose::Size::ORIGINAL }
    }

    /// The strings the frames drawn so far had to lay out to place what
    /// follows them (the sharp text's: a chip's damage after its name), each
    /// the first time it was met.
    pub fn layouts_made(&self) -> u64 {
        self.measure.as_ref().map_or(0, |m| m.borrow().made().layouts)
    }

    /// The graphics it draws from (another renderer of the same: a clone).
    pub fn graphics(&self) -> &GameGraphics {
        &self.graphics
    }

    /// Draw from other graphics from the next frame on: the same packs'
    /// in another language (with [`Renderer::set_strings`], its strings).
    /// Nothing the renderer follows over time is touched (the HUD's rolling
    /// numbers and timers are the battle's, not the graphics'), so a swap
    /// mid-battle is no reset. The frame's lookups are checked again
    /// against what is drawn from now.
    pub fn set_graphics(&mut self, graphics: GameGraphics) {
        self.drawings = graphics.drawings();
        self.graphics = graphics;
        self.problems.recheck();
    }

    /// Follow a tick of the battle being shown (call after every tick): each
    /// console's HUD.
    pub fn observe(&mut self, b: &Battle) {
        let local = b.setup.local_side & 1;
        self.hud_state.tick(b, local);
        self.other_hud.tick(b, local ^ 1);
    }

    /// Forget presentation state (a new battle or round starts): the HUDs',
    /// and how far the scenery was ahead of the battle's clock.
    pub fn reset(&mut self) {
        self.hud_state = HudState::default();
        self.other_hud = HudState::default();
        self.scenery = StageClock::default();
    }

    /// A frame of a battle held where it stands (a host's menu over it: no
    /// tick steps it): the stage's scenery moves on a frame as the battle's
    /// tick would have moved it, the background's animation and its scroll
    /// (the smooth motion's glide with it) and the panels' own animations,
    /// as the game's own pause shows them; nothing else of the frame moves.
    /// Drawing alone: `b` is read for where its round is, never changed.
    pub fn scenery_frame(&mut self, b: &Battle) {
        self.scenery.step(b);
    }

    /// The clock `b`'s stage is drawn on: its own ([`StageClock::of`]), and
    /// the frames its scenery moved on while it was held.
    pub fn stage_clock(&self, b: &Battle) -> StageClock {
        StageClock::of(b).ahead(self.scenery)
    }

    /// The view a battle is seen from: the local player's console's, whose
    /// camera a shake moves this tick (`camera_doShakeEffect_80301e8`) and
    /// the custom screen's window moves down while it is in.
    pub fn view(b: &Battle) -> View {
        Self::view_for(b, Consoles::One, [None; 2])
    }

    /// The view a frame showing `consoles` sees the battle from: the local
    /// console's, mirrored as it mirrors the field; its camera that of the
    /// console the field is seen through (`seat::field_side`: on a screen
    /// of both where the other player alone chooses, the other's, its
    /// shake the other way, as the mirror shows it).
    pub fn view_for(b: &Battle, consoles: Consoles, layers: Layers) -> View {
        let local = b.setup.local_side & 1;
        let field = crate::seat::field_side(consoles, b);
        let (x, y) = b.consoles[field as usize].camera.jitter;
        let x = if field == local { x } else { -x };
        let layer = layers[field as usize];
        let fade = crate::custom::object_fade(b, layer, field).unwrap_or_default();
        View { camera: (x, y + crate::custom::camera_y(b, layer, field), 0), mirror: local == 1, fade }
    }

    /// The part of a pixel (16.16) the camera's Y is past its whole
    /// pixels: what the projection leaves of the custom screen's slide,
    /// 1.5 pixels a tick (`custom::camera_y`). (A shake's jitter is whole
    /// pixels, and the camera's X has no part of a pixel.) `layers`: each
    /// side's custom screen layer, as `render` takes them.
    /// What a pointer finds on side `side`'s custom screen in the last
    /// frame drawn, with each target's box in the frame's pixels (none
    /// while no screen of its is at rest there: docs/design/custom-screen-actions.md §5).
    pub fn targets(&self, side: u8) -> impl Iterator<Item = (nettai_battle::custom::layer::Target, Rect)> + '_ {
        self.targets.iter().filter(move |(s, _, _)| *s == side & 1).map(|&(_, t, r)| (t, r))
    }

    /// The target a pointer at `(x, y)` of the last frame (its pixels) is
    /// on, of side `side`'s custom screen: the first whose box holds it.
    pub fn target_at(&self, side: u8, x: i32, y: i32) -> Option<nettai_battle::custom::layer::Target> {
        self.targets(side).find(|(_, r)| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h).map(|(t, _)| t)
    }

    pub fn camera_sub(&self, b: &Battle, layers: Layers) -> i32 {
        Self::view_for(b, self.consoles, layers).camera.1 & 0xFFFF
    }

    /// Draw a battle as a frame (`Renderer::frame_size`) with its text items.
    /// `layers`: each side's custom screen layer (`nettai_battle::custom::
    /// Layer`), by side, which draws its cursor, its chip window, its
    /// window's counters and sprites, its hover's shade and what it has up
    /// over the chips; a side without one is drawn without them.
    pub fn render(&mut self, b: &Battle, layers: Layers) -> Frame {
        // (The smooth motion: the frame, and where the camera is
        // between whole pixels, the frame again with the field a pixel up;
        // where the gauge's fill ends between whole pixels, the frame again
        // with it a pixel longer. The later drawings' problems and sprite
        // count are the first's.)
        let on = self.motion() && !self.lookups_only;
        let mut frame = self.draw(b, layers, Smooth { on, ..Smooth::default() });
        if !on || frame.pixels.is_empty() {
            return frame;
        }
        let (problems, drawn) = (self.problems.clone(), self.sprite_drawn.clone());
        let camera = self.camera_sub(b, layers);
        if camera != 0 {
            let again = self.draw(b, layers, Smooth { on, lifted: true, ..Smooth::default() });
            frame.lift = Some(Lift { pixels: again.pixels, depth: again.depth, sub: camera });
        }
        // (The gauge is off the HUD while the custom screen's window
        // slides, so a lift and a nudge never meet; were they to, the gauge
        // would show at its whole pixels on the lifted rows.)
        let sub = crate::hud::gauge_sub(b, &self.hud_state, self.custom_gauge);
        if sub != 0 {
            let again = self.draw(b, layers, Smooth { on, nudged: true, ..Smooth::default() });
            frame.nudge = Some(Nudge { pixels: again.pixels, depth: again.depth, sub });
        }
        (self.problems, self.sprite_drawn) = (problems, drawn);
        frame
    }

    /// Draw a battle as a frame with its text items, its motion as `smooth`
    /// says.
    fn draw(&mut self, b: &Battle, screen_layers: Layers, smooth: Smooth) -> Frame {
        self.problems.known.clear();
        self.problems.marks.clear();
        self.targets.clear();
        // (Every layer as large as the frame; what is drawn on the
        // original's screen in the middle of it, the background on to all
        // four edges, the HUD's pieces each where it keeps to: `hud::draw`,
        // `custom::draw`.)
        let size = self.frame_size();
        let layers = [
            &mut self.background,
            &mut self.field,
            &mut self.hud,
            &mut self.names,
            &mut self.hud_right,
            &mut self.names_right,
            &mut self.dialogue,
            &mut self.scratch,
            &mut self.scratch_names,
        ];
        for layer in layers {
            layer.set_size(size);
            layer.keep_to(compose::Anchor::CENTER);
        }
        // (The graphics for this frame: the layers and the problems are the
        // renderer's own fields, written while these are read.)
        let mut graphics = self.graphics.view();
        graphics.set_version(self.console_version);
        let view = Self::view_for(b, self.consoles, screen_layers);
        // (The background is its own game's, another game's too (the
        // background pool's), or where that game's isn't here, the stage's own;
        // the field, the game's: `FieldArt`.)
        let shown = graphics.round_background(&b.content, &b.setup.settings);
        let background = crate::lookups::background(&graphics, &b.content, shown, &mut self.problems);
        // (The background, behind everything, where the caller asks.)
        if background.is_some()
            && !self.problems.marking.is_empty()
            && let Some(name) = crate::graphics::name(&b.content, nettai_content_api::AssetKind::Background, shown.0)
        {
            let what = format!("background:{name}");
            if self.problems.wants(&what) {
                self.problems.mark([0, 0, compose::WIDTH as i32, compose::HEIGHT as i32], what);
            }
        }
        let draw = !self.lookups_only;
        // (A flash fills the stage's palettes with its white through a
        // palette transform's slot, and a dimming is one of the fade
        // system's transforms: where the game's flash comes before the
        // fades, EXE5's and EXE4's, its white is the stage's palettes, which
        // a background's palette shift shifts, a dimming darkens a quarter
        // down and a transformation's fade takes to black (EXE4's
        // JunkSoul's change's flash, 0x080EBB50: black until the fade
        // lifts); where it comes after, EXE6's, the white stands.)
        let flash = objects::palette_flash(b);
        let flash_order = b.content.rules().effects.palette_flash_order;
        let flashed = flash.is_some() && flash_order == PaletteFlashOrder::BeforeFades;
        let clock = self.stage_clock(b);
        let stage = draw.then(|| Stage::new(&graphics, &b.content, background, clock, flashed));
        match &stage {
            Some(stage) => {
                self.background.clear();
                stage.draw_background(&mut self.background);
                self.field.clear();
                // (The lifted drawing's field a pixel up: `Lift`.)
                let field_view = match smooth.lifted {
                    true => View { camera: (view.camera.0, view.camera.1 + 0x10000, view.camera.2), ..view },
                    false => view,
                };
                stage.draw_field(b, &mut self.field, b.setup.local_side, &field_view, &mut self.problems);
            }
            None => crate::stage::field_lookups(b, &graphics, b.setup.local_side, &mut self.problems),
        }
        let both = self.consoles == Consoles::Both;
        for layer in [&mut self.hud, &mut self.names, &mut self.dialogue, &mut self.scratch, &mut self.scratch_names, &mut self.hud_right, &mut self.names_right] {
            layer.drawn = draw;
        }
        self.hud.clear();
        self.names.clear();
        self.dialogue.clear();
        if both {
            self.hud_right.clear();
            self.names_right.clear();
        }
        // (The local player's custom screen and chatbox: the game's.)
        let local = b.setup.local_side as usize & 1;
        let own_game = graphics.bundle();
        // Graphics with no lettering in the language shown (Chinese: every
        // pack has English's and Japanese's) is drawn in its own
        // (`Bundle::for_language`): its pictures with words are that
        // language's, and so are its fonts, which lack most of the shown
        // language's characters. Without the sharp text the frame then lays
        // the strings out as the sharp text does, in boxes as wide as the
        // text, and draws them into the frame in pixel fonts (`pixeltext`):
        // the frame still carries all its text.
        let shown = self.strings.as_ref().map_or(nettai_content::locale::OWN, |s| s.language.as_str());
        let e = self.enhancements;
        let stand_in = !e.text && own_game.hud.language() != shown;
        if stand_in && self.stand_in.is_none() {
            self.stand_in = crate::pixeltext::PixelText::new(crate::vfont::Cut::of(shown)).map(std::cell::RefCell::new);
        }
        let pixels = self.stand_in.as_ref().filter(|_| stand_in);
        if let Some(p) = pixels {
            p.borrow_mut().use_pack(&own_game.hud);
        }
        // (The font goes with the sharp HUD too, whose lettering fonts are
        // its cut's; it takes strings only with the sharp text.)
        let mut text = TextSink::new(e, self.font.as_deref()).measuring(self.measure.as_ref()).with_pixels(pixels).with_language(self.strings.as_deref());
        // (The consoles whose pieces the frame draws, each with its HUD's
        // state: the local one's, and on a screen of both the other's.)
        let seats = Seat::of(self.consoles, b);
        // (Each one's emblem, for its custom screen: the navi the screen is
        // for, which a switch on it changes, `sub_802812C`.)
        let emblems: Vec<(nettai_assets::Tiles, nettai_assets::Palette)> = seats
            .iter()
            .map(|seat| {
                let side = seat.side as usize & 1;
                let navi = b.custom.sides[side].screen.as_ref().map_or(b.stats[side].navi, |s| crate::custom::screen_navi(b, seat.side, s));
                let emblem = crate::lookups::emblem(&own_game.custom, &b.content, navi, &mut self.problems);
                (crate::custom::emblem_sprite(emblem), emblem.map_or([0; 16], |e| e.palette))
            })
            .collect();
        // (A chatbox the custom screen runs: the left player's, else the
        // right's, as its console draws it: it spans the screen.)
        let chatbox = seats
            .iter()
            .filter(|s| s.shows != Shows::Nothing)
            .find_map(|s| crate::chatbox::prepare(b, screen_layers[s.side as usize & 1], s.side, own_game, &graphics, &text, &mut self.problems));
        let huds = (&self.hud_state, &self.other_hud);
        let hud_of = |seat: Seat| if seat.side == local as u8 { huds.0 } else { huds.1 };
        // (The glyphs the banners' and the telops' loads made of each
        // other's tiles, which the sprites below borrow; each console's.)
        let shared: Vec<_> = seats.iter().map(|&seat| crate::hud::shared_glyphs(b, own_game, &graphics, hud_of(seat), seat, &text, &mut self.problems)).collect();
        let mut list = SpriteList::default();
        // (On a screen of both, what either player sees.)
        let viewers: Vec<u8> = seats.iter().map(|s| s.side).collect();
        self.sprite_drawn = objects::queue_objects(b, &graphics, &view, &viewers, &mut list, &mut self.problems, !draw);
        let mut targets = Vec::new();
        for (k, &seat) in seats.iter().enumerate() {
            if seat.shows == Shows::Nothing {
                continue;
            }
            let mark = text.mark();
            let (hud, names) = if seat.mirrored() { (&mut self.hud_right, &mut self.names_right) } else { (&mut self.hud, &mut self.names) };
            let (emblem, emblem_palette) = &emblems[k];
            let scratch = (&mut self.scratch, &mut self.scratch_names);
            // (The sprites' fade of its console: on the right of a screen of
            // both its own, the frame's being the field's console's.)
            let screen_layer = screen_layers[seat.side as usize & 1];
            let sprite_fade = if flash == Some(1) { Fade::White(16) } else { crate::custom::sprite_fade(b, screen_layer, seat.side).unwrap_or_default() };
            // (Kept to its part of a screen of both, where not all of it:
            // `Seat::clip`.)
            let clip = seat.clip(b, screen_layers, &seats, size);
            list.clip = clip;
            let found = crate::custom::draw(b, screen_layer, own_game, &graphics, emblem, *emblem_palette, (seat, sprite_fade), smooth, (&mut *hud, &mut *names), scratch, &mut list, &mut text, &mut self.problems);
            targets.extend(found.into_iter().map(|(t, r)| (seat.side, t, r)));
            crate::hud::draw(b, own_game, &graphics, hud_of(seat), &shared[k], (seat, &view, screen_layer), self.custom_gauge, smooth, (&mut *hud, &mut self.scratch), &mut list, &mut text, &mut self.problems);
            list.clip = None;
            if let Some([x0, x1]) = clip {
                hud.keep_columns(x0 as i32..x1 as i32);
                names.keep_columns(x0 as i32..x1 as i32);
                // (The frame's columns, all its rows, on the original's
                // screen where each item is laid out.)
                let kept = Rect::new(x0 as i32, 0, (x1 - x0) as i32, size.height as i32);
                for (_, item) in text.since(mark) {
                    let (ox, oy) = size.origin(item.anchor);
                    item.clip = item.clip.intersect(kept.offset(-ox, -oy));
                }
            }
            // (The right one's own HUD layer and BG0.)
            if seat.mirrored() {
                for (plane, _) in text.since(mark) {
                    *plane = match *plane {
                        Plane::Hud => Plane::OtherHud,
                        Plane::Bg0 => Plane::OtherBg0,
                        p => p,
                    };
                }
            }
        }
        // (While both players' screens are each in its half: C1, and both
        // windows sliding in or out together.)
        if seats.iter().all(|s| s.shows == Shows::Half) {
            crate::hud::time_between(b, own_game, &mut self.hud, &mut text, &mut self.problems);
        }
        // (The chatbox with the field, both ways.)
        if let Some(c) = chatbox.as_ref().filter(|_| draw) {
            (list.anchor, text.anchor) = (compose::Anchor::CENTER, compose::Anchor::CENTER);
            crate::chatbox::draw(c, own_game, &mut self.dialogue, &mut list, &mut text);
        }
        let Some(stage) = stage else {
            note_missing_strings(&mut text, &mut self.problems);
            return Frame::default();
        };
        // (Each console's HUD layer shaken as its console shakes it: the
        // right one's the other way, as the mirror shows it.)
        let (jx, jy) = crate::custom::hud_jitter(b, screen_layers[local], local as u8);
        self.hud.shift(-jx, -jy);
        // (The window's places in the frame: from its anchor, shaken as the
        // HUD layer is.)
        let (ox, oy) = size.origin(compose::Anchor::TOP_LEFT);
        self.targets = targets.into_iter().map(|(side, t, r)| (side, t, r.offset(ox - jx, oy - jy))).collect();
        let (rx, ry) = if both { crate::custom::hud_jitter(b, screen_layers[local ^ 1], local as u8 ^ 1) } else { (0, 0) };
        self.hud_right.shift(rx, -ry);
        // (The HUD's BG3 is the game's priority: EXE4's 0 covers the HP
        // numbers' priority 1 where the custom screen is.)
        self.hud.priority = own_game.hud.layout.hud_priority;
        self.hud_right.priority = own_game.hud.layout.hud_priority;
        let (parts, tags, fines) = list.into_placed_parts();
        let backdrop = stage.palettes[0][0];
        let scene = &stage;
        // The transformation's fade takes every background palette, a
        // dimming's the stage's; the custom screen's Beast Out the stage's
        // and the HUD's, its other fades the stage's (a dark chip's second
        // the HUD's).
        let transform = layer_fade(b);
        // (The custom screen's fades of the field and the sprites the
        // console's the field is seen through, each HUD layer's its own
        // console's.)
        let field = crate::seat::field_side(self.consoles, b);
        let field_layer = screen_layers[field as usize & 1];
        let custom = crate::custom::fade(b, field_layer, field).unwrap_or_default();
        let custom_hud = crate::custom::hud_fade(b, screen_layers[local], local as u8).unwrap_or_default();
        let other_custom_hud = crate::custom::hud_fade(b, screen_layers[local ^ 1], local as u8 ^ 1).unwrap_or_default();
        let stage = if flashed {
            // (The fade system's first record, a transformation's or a
            // dimming's: its slots come after the flash's.)
            if transform != Fade::None { transform } else { dim_fade(b) }
        } else if flash.is_some() {
            Fade::White(16)
        } else if transform != Fade::None {
            transform
        } else if custom != Fade::None {
            custom
        } else {
            dim_fade(b)
        };
        // (The flash takes the palette transform the transformation's fade
        // uses: on its frames the HUD's palettes are the flash's, which
        // leaves them be in variant 0: as the transformation's fade left
        // them, black through a form change. A Japanese console's chip
        // window shows there, drawn black over the white stage.)
        let hud_fade_of = |custom_hud: Fade| match flash {
            Some(1) => Fade::White(16),
            _ if transform != Fade::None => transform,
            _ => custom_hud,
        };
        let (hud, other_hud) = (hud_fade_of(custom_hud), hud_fade_of(other_custom_hud));
        // (The custom screen's fades of palettes 0-13 and 9-13 leave the
        // HUD's 14 and 15: the flash's, the transformation's and the custom
        // screen's white, which takes every palette, reach them.)
        let past_ranged_of = |side: u8| match flash {
            Some(1) => Fade::White(16),
            _ if transform != Fade::None => transform,
            _ => crate::custom::sprite_fade(b, screen_layers[side as usize & 1], side).unwrap_or_default(),
        };
        let (hud_past_ranged, other_hud_past_ranged) = (past_ranged_of(local as u8), past_ranged_of(local as u8 ^ 1));
        let sprites = if flash == Some(1) { Fade::White(16) } else { crate::custom::sprite_fade(b, field_layer, field).unwrap_or_default() };
        // (The chatbox's palette is past every ranged fade's palettes: a
        // dimming's 0-8, the custom screen's 0-13 and 9-13, the two-layer
        // flash's 0-14. The fades of every palette reach it: the
        // transformation's, and the custom screen's white.)
        let dialogue = if transform != Fade::None { transform } else { crate::custom::sprite_fade(b, field_layer, field).unwrap_or_default() };
        let fades = Fades { stage, hud, hud_past_ranged, other_hud, other_hud_past_ranged, dialogue, sprites, screen: screen_fade(b) };
        let layers: Vec<&Layer> = match both {
            true => vec![&self.dialogue, &self.names, &self.names_right, &self.hud, &self.hud_right, &self.field, &self.background],
            false => vec![&self.dialogue, &self.names, &self.hud, &self.field, &self.background],
        };
        // (The squashed and turned sprites, and those whose true places
        // are between the frame's pixels, at the output's resolution, in
        // the smooth motion: left out of the frame, their places
        // in the order kept.)
        let overlays: Vec<(usize, Overlay)> = match smooth.on {
            true => parts
                .iter()
                .enumerate()
                .filter_map(|(i, p)| Some((i, Overlay::of(p, size, compose::sprite_depth(p.priority, i), [p.fade.unwrap_or(fades.sprites), fades.screen], fines[i])?)))
                .collect(),
            false => Vec::new(),
        };
        let (pixels, depth) = if overlays.is_empty() {
            compose::compose_sized(size, backdrop, &layers, &parts, fades)
        } else {
            let mut kept = parts.clone();
            for &(i, _) in &overlays {
                kept[i] = SpritePart { height: 0, ..kept[i].clone() };
            }
            compose::compose_sized(size, backdrop, &layers, &kept, fades)
        };
        let overlays = overlays.into_iter().map(|(_, o)| o).collect();
        // Each item's depth and fades: its layer's (the HUD layer's moved
        // with its shake), or its sprite parts' (an item whose parts the
        // sprite limit dropped isn't drawn); and its place in the frame,
        // where what keeps to its anchor goes (a sprite's item, its parts').
        note_missing_strings(&mut text, &mut self.problems);
        let text = text
            .into_items()
            .into_iter()
            .filter_map(|(plane, mut item)| {
                let (depth, fade) = match plane {
                    Plane::Hud => {
                        item.rect = item.rect.offset(-jx, -jy);
                        item.clip = item.clip.offset(-jx, -jy);
                        (compose::layer_depth(&self.hud), if item.past_ranged { fades.hud_past_ranged } else { fades.hud })
                    }
                    Plane::OtherHud => {
                        item.rect = item.rect.offset(rx, -ry);
                        item.clip = item.clip.offset(rx, -ry);
                        (compose::layer_depth(&self.hud_right), if item.past_ranged { fades.other_hud_past_ranged } else { fades.other_hud })
                    }
                    Plane::Bg0 => (compose::layer_depth(&self.names), fades.hud),
                    Plane::OtherBg0 => (compose::layer_depth(&self.names_right), fades.other_hud),
                    Plane::Sprite(tag) => {
                        let i = tags.iter().position(|&t| t == Some(tag))?;
                        item.anchor = parts[i].anchor;
                        (compose::sprite_depth(parts[i].priority, i), parts[i].fade.unwrap_or(fades.sprites))
                    }
                };
                item.depth = depth;
                item.fades = [fade, fades.screen];
                Some(item.placed(size))
            })
            .collect();
        // (The background between the whole pixels the frame shows, for the
        // output to glide: a scroll by sixteenths of a pixel a frame shows
        // a step every few frames otherwise.)
        let glide = (self.motion() && scene.background_sub() != (0, 0)).then(|| {
            let color = |c: u16| compose::apply_fade(compose::apply_fade(c, fades.stage), fades.screen);
            let (ox, oy) = self.background.origin();
            let width = size.width + 1;
            let pixels = scene.draw_background_wide((-ox, -oy), (width, size.height + 1), backdrop, color);
            Glide { pixels, width, sub: scene.background_sub(), depth: compose::layer_depth(&self.background) }
        });
        let drawings = (!Vec::is_empty(&text)).then(|| self.drawings.clone());
        let mut frame = Frame { size, pixels, depth, text, glide, nudge: None, lift: None, overlays, drawings };
        if let Some(p) = self.stand_in.as_ref().filter(|_| stand_in) {
            p.borrow_mut().draw(&mut frame);
        }
        frame
    }
}

/// Note what the frame asked of the player's language's strings table that
/// it doesn't have.
fn note_missing_strings(text: &mut TextSink, problems: &mut Problems) {
    if let Some(lang) = text.strings.language() {
        let lang = lang.to_string();
        for what in text.strings.take_missing() {
            problems.note(format!("the {lang} strings table has no {what}: shown in the content's own"));
        }
    }
}

/// A dimming (`object_dimScreen`, `object_undimScreen`: fade modes 0x3C
/// and 0x38) darkens the first nine background palettes, the stage's, by a
/// sixteenth for every 0x10 of the fade's level: a quarter when dimmed.
/// The HUD's palettes and the sprites keep their colors. The Gregar and
/// Falzar chips' black-out (modes 0x88 and 0x84) darkens the same palettes
/// all the way.
pub fn dim_fade(b: &Battle) -> Fade {
    if !matches!(b.fade.mode, FadeMode::Dim | FadeMode::Undim | FadeMode::BlackOut | FadeMode::BlackOutBack) {
        return Fade::None;
    }
    Fade::Black((b.fade.level >> 4).min(16) as u8)
}

/// The transformation sequencer fades the tile layers (not the sprites)
/// out to black while the navis change form, and back in (a palette flash
/// takes its place: `objects::palette_flash`). The same two fades started
/// outside the sequencer (`battle.screen_fade`: EXE5's dark MegaMan's last
/// stand, his action 0x30) are drawn by the fade's own record: out as it
/// runs and black until the fade back in, which clears as it runs.
pub fn layer_fade(b: &Battle) -> Fade {
    let left = shown_remaining(b);
    match b.transform_seq.state {
        SequencerState::Transform { phase: TransformPhase::FadeOut, started: true } => Fade::Black(16u8.saturating_sub(left)),
        SequencerState::Transform { phase: TransformPhase::Change, .. } => Fade::Black(16),
        SequencerState::Transform { phase: TransformPhase::FadeIn, started } => Fade::Black(if started { left.min(16) } else { 16 }),
        _ => match b.fade.mode {
            FadeMode::TransformOut => Fade::Black(16u8.saturating_sub(left)),
            FadeMode::TransformIn if b.fade.active() => Fade::Black(left.min(16)),
            _ => Fade::None,
        },
    }
}

/// What the screen shows of the running fade: its steps left as a fade
/// that ends at its target counts them. A game whose fade toward clear ends
/// a step later (`effects.fade_clear`'s `past_target`: EXE4's) steps through
/// the same levels on the same frames and holds the clear screen that step
/// more: drawn alike.
fn shown_remaining(b: &Battle) -> u8 {
    b.fade.remaining(nettai_battle::content::FadeClear::AtTarget)
}

/// The screen fade: the battle fades in while the intro's fade runs
/// (`Fade::TICKS` frames) and fades out to black at the end. The screen
/// shows the fade one frame late (it is applied to the palettes at the
/// next vblank).
pub fn screen_fade(b: &Battle) -> Fade {
    let total = nettai_battle::battle::Fade::TICKS as u32;
    let left = (shown_remaining(b) as u32 + 1).min(total);
    if b.round.intro_bits & 0x01 == 0 {
        // Every battle fades in from black, as an EXE5 console shows it (its
        // `sub_80E0684` counterpart, 0x080E0698, tests an open-bus value,
        // never a first battle): EXE6's and EXE4's first battle of a set
        // faded in from white, a flash nettai leaves out (the user: "remove
        // the white flash at the start of battles"). The simulation's fade
        // (`kinds::intro`) is the original's, white or black, and only
        // its timing is drawn.
        let fade = Fade::Black;
        // Before the intro fade starts the screen is fully faded.
        if b.round.intro_bits & 0x10 == 0 {
            return fade(16);
        }
        return fade(((left * 16) / total) as u8);
    }
    if b.round.mode == mode::FADE_OUT || b.round.top == nettai_battle::battle::top::END {
        // The fade starts on the state's second tick.
        if b.round.top != nettai_battle::battle::top::END && b.round.init == 0 {
            return Fade::None;
        }
        if b.fade.active() {
            return Fade::Black((16 - (left * 16) / total) as u8);
        }
        return Fade::Black(16);
    }
    Fade::None
}
