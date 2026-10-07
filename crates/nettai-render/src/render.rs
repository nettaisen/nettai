//! Drawing a battle: everything visible is derived from the engine's state
//! and the pack's graphics.

use crate::audit::Problems;
use crate::compose::{self, Fade, Fades, Layer, Palettes};
use crate::hud::HudState;
use crate::objects::{self, SpriteList, View};
use crate::packs::PackGraphics;
use crate::stage::{Stage, StageClock};
use crate::textlayer::{Plane, TextItem, TextMode, TextSink};
use crate::vfont::VectorFont;
use nettai_assets::Bundle;
use nettai_battle::Battle;
use nettai_battle::battle::{FadeMode, mode};
use nettai_battle::content::PaletteFlashOrder;
use nettai_battle::transform::{SequencerState, TransformPhase};
use std::sync::Arc;

/// A drawn frame: the 240x160 picture (BGR555), per pixel the depth key
/// of what won it (`compose::depth_key`), and the text items to draw over
/// it at the output's resolution (none in the original text mode).
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub pixels: Vec<u16>,
    pub depth: Vec<u32>,
    pub text: Vec<TextItem>,
}

/// Picks battles; keeps its layer buffers between frames. It owns what it
/// draws from (the packs' graphics are shared), so it borrows nothing.
pub struct Renderer {
    /// Every loaded pack's graphics, for the assets of each
    /// (docs/design/rules-in-luau.md §7.4), and the content's own pack's
    /// (the HUD's and the custom screen's frames).
    graphics: PackGraphics,
    background: Layer,
    field: Layer,
    hud: Layer,
    /// BG0, in front of everything: the custom screen's enemy names.
    names: Layer,
    /// BG0's chatbox, apart for its palette (background palette 15: the
    /// custom screen's fades of the window's palettes leave it be).
    dialogue: Layer,
    /// Rolling HUD numbers; follow every tick with `observe`.
    pub hud_state: HudState,
    /// What the frames drawn so far named that the pack doesn't have.
    pub problems: Problems,
    /// The version of the console whose screen is drawn, as its game's
    /// pack names its versions (EXE5's "protoman", "colonel"), for a game
    /// whose versions the engine doesn't tell apart: which chips are the
    /// other version's, whose art its ROM draws otherwise
    /// (`ChipArt::version`), and the soul icon's outline. None: the
    /// version the console's player brought, or the pack's base version
    /// (`custom::console_version`).
    pub console_version: Option<&'static str>,
    /// How text is drawn, and the font of the font mode.
    text_mode: TextMode,
    font: Option<Arc<VectorFont>>,
    /// The font mode's layouts, for what the frame places after a string
    /// (a chip's damage after its name).
    measure: Option<std::cell::RefCell<crate::vfont::TextRenderer>>,
    /// The player's language's strings table (`--lang`), if not the
    /// content's own.
    strings: Option<Arc<nettai_content::locale::Strings>>,
    /// Make only the frame's lookups (`--audit`): nothing is drawn or
    /// composed, and `render` returns an empty frame.
    lookups_only: bool,
}

impl Renderer {
    /// A renderer of one pack's graphics, in the original text mode.
    pub fn new(assets: Arc<Bundle>) -> Renderer {
        Renderer::with_packs(PackGraphics::one(assets))
    }

    /// A renderer of several packs' graphics.
    pub fn with_packs(graphics: PackGraphics) -> Renderer {
        Renderer {
            graphics,
            // Background priority 3 (BG1), field 2 (BG2), HUD 1 (BG3: the
            // game's pack's `hud_priority`, set each frame).
            background: Layer::new(3, 1),
            field: Layer::new(2, 2),
            hud: Layer { palettes: Palettes::Hud, ..Layer::new(1, 3) },
            names: Layer { palettes: Palettes::Hud, ..Layer::new(0, 0) },
            dialogue: Layer { palettes: Palettes::Dialogue, ..Layer::new(0, 0) },
            hud_state: HudState::default(),
            problems: Problems::default(),
            console_version: None,
            text_mode: TextMode::Original,
            font: None,
            measure: None,
            strings: None,
            lookups_only: false,
        }
    }

    /// From now on make only the lookups a frame makes (`--audit`): what
    /// the frame names is asked of the packs and the content as drawing
    /// asks it, and no pixel is drawn.
    pub fn set_lookups_only(&mut self, on: bool) {
        self.lookups_only = on;
    }

    /// Show content's display text from a language's strings table (the
    /// content's own strings where it has none); `None`, the content's
    /// own.
    pub fn set_strings(&mut self, strings: Option<Arc<nettai_content::locale::Strings>>) {
        self.strings = strings;
        self.problems.recheck();
    }

    /// Draw text in `mode`; the font mode hands the strings `font` has to
    /// the text layer (without a font it draws as the original does).
    pub fn set_text(&mut self, mode: TextMode, font: Option<Arc<VectorFont>>) {
        self.text_mode = mode;
        self.measure = font.clone().map(|f| std::cell::RefCell::new(crate::vfont::TextRenderer::new(f)));
        self.font = font;
    }

    pub fn text_mode(&self) -> TextMode {
        self.text_mode
    }

    /// The graphics it draws from (another renderer of the same: a clone).
    pub fn graphics(&self) -> &PackGraphics {
        &self.graphics
    }

    /// Draw from other graphics from the next frame on: the same packs'
    /// in another language (with [`Renderer::set_strings`], its strings).
    /// Nothing the renderer follows over time is touched (the HUD's rolling
    /// numbers and timers are the battle's, not the graphics'), so a swap
    /// mid-battle is no reset. The frame's lookups are checked again
    /// against what is drawn from now.
    pub fn set_graphics(&mut self, graphics: PackGraphics) {
        self.graphics = graphics;
        self.problems.recheck();
    }

    /// Follow a tick of the battle being shown (call after every tick).
    pub fn observe(&mut self, b: &Battle) {
        self.hud_state.tick(b);
    }

    /// Forget presentation state (a new battle starts).
    pub fn reset(&mut self) {
        self.hud_state = HudState::default();
    }

    /// The view a battle is seen from: the local player's console's, whose
    /// camera a shake moves this tick (`camera_doShakeEffect_80301e8`) and
    /// the custom screen's window moves down while it is in.
    pub fn view(b: &Battle) -> View {
        let local = b.setup.local_side;
        let (x, y) = b.consoles[local as usize & 1].camera.jitter;
        let fade = crate::custom::object_fade(b).unwrap_or_default();
        View { camera: (x, y + crate::custom::camera_y(b), 0), mirror: local & 1 == 1, fade }
    }

    /// Draw a battle as a 240x160 frame with its text items.
    pub fn render(&mut self, b: &Battle) -> Frame {
        self.problems.known.clear();
        self.problems.marks.clear();
        // (The packs for this frame: the layers and the problems are the
        // renderer's own fields, written while these are read.)
        let mut packs = self.graphics.packs();
        packs.set_version(self.console_version);
        let view = Self::view(b);
        // (The background is its own pack's; the field, the game's pack's:
        // `FieldArt`.)
        let background = crate::lookups::background(&packs, &b.content, b.setup.settings.background, &mut self.problems);
        // (The background, behind everything, where the caller asks.)
        if background.is_some()
            && !self.problems.marking.is_empty()
            && let Some(name) = crate::packs::name(&b.content, nettai_content_api::AssetKind::Background, b.setup.settings.background.0)
        {
            let what = format!("background:{name}");
            if self.problems.wants(&what) {
                self.problems.mark([0, 0, compose::WIDTH as i32, compose::HEIGHT as i32], what);
            }
        }
        let draw = !self.lookups_only;
        let stage = draw.then(|| Stage::new(&packs, &b.content, background, StageClock::of(b)));
        match &stage {
            Some(stage) => {
                self.background.clear();
                stage.draw_background(&mut self.background);
                self.field.clear();
                stage.draw_field(b, &mut self.field, b.setup.local_side, &view, &mut self.problems);
            }
            None => crate::stage::field_lookups(b, &packs, b.setup.local_side, &mut self.problems),
        }
        self.hud.drawn = draw;
        self.names.drawn = draw;
        self.dialogue.drawn = draw;
        self.hud.clear();
        self.names.clear();
        self.dialogue.clear();
        let mut text =
            TextSink::new(self.text_mode, self.font.as_deref()).measuring(self.measure.as_ref()).with_language(self.strings.as_deref());
        // (The local player's custom screen and chatbox: the game's pack's.)
        let local = b.setup.local_side as usize & 1;
        let own_game = packs.game(&b.content);
        let emblem = crate::lookups::emblem(&own_game.custom, &b.content, b.stats[local].navi, &mut self.problems);
        let (emblem, emblem_palette) = (crate::custom::emblem_sprite(emblem), emblem.map_or([0; 16], |e| e.palette));
        let chatbox = crate::chatbox::prepare(b, own_game, &packs, &text, &mut self.problems);
        let mut list = SpriteList::default();
        objects::queue_objects(b, &packs, &view, &mut list, &mut self.problems, !draw);
        crate::custom::draw(
            b,
            own_game,
            &packs,
            &emblem,
            emblem_palette,
            &mut self.hud,
            &mut self.names,
            &mut list,
            &mut text,
            &mut self.problems,
        );
        if let Some(c) = chatbox.as_ref().filter(|_| draw) {
            crate::chatbox::draw(c, own_game, &mut self.dialogue, &mut list, &mut text);
        }
        crate::hud::draw(b, own_game, &packs, &self.hud_state, &mut self.hud, &mut list, &mut text, &mut self.problems);
        let Some(stage) = stage else {
            note_missing_strings(&mut text, &mut self.problems);
            return Frame::default();
        };
        let (jx, jy) = crate::custom::hud_jitter(b);
        self.hud.shift(-jx, -jy);
        // (The HUD's BG3 is the game's priority: EXE4's 0 covers the HP
        // numbers' priority 1 where the custom screen is.)
        self.hud.priority = own_game.hud.layout.hud_priority;
        let (parts, tags) = list.into_tagged_parts();
        let backdrop = stage.palettes[0][0];
        // The transformation's fade takes every background palette, a
        // dimming's the stage's; the custom screen's Beast Out the stage's
        // and the HUD's, its other fades the stage's (a dark chip's second
        // the HUD's).
        let transform = layer_fade(b);
        let custom = crate::custom::fade(b).unwrap_or_default();
        let custom_hud = crate::custom::hud_fade(b).unwrap_or_default();
        let flash = objects::palette_flash(b);
        // (A flash fills the stage's palettes with its white through a
        // palette transform's slot, and a dimming is one of the fade
        // system's transforms: where the game's flash comes before the
        // fades, EXE5's, a dimming darkens the white with the rest, a
        // quarter down; where it comes after, EXE6's, the white stands.)
        let flash_order = b.content.rules().effects.palette_flash_order;
        let stage = if flash.is_some() {
            match (flash_order, dim_fade(b)) {
                (PaletteFlashOrder::BeforeFades, Fade::Black(n)) => Fade::Flash(n),
                _ => Fade::White(16),
            }
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
        let hud = match flash {
            Some(1) => Fade::White(16),
            _ if transform != Fade::None => transform,
            _ => custom_hud,
        };
        let sprites = if flash == Some(1) { Fade::White(16) } else { crate::custom::sprite_fade(b).unwrap_or_default() };
        // (The chatbox's palette is past every ranged fade's palettes: a
        // dimming's 0-8, the custom screen's 0-13 and 9-13, the two-layer
        // flash's 0-14. The fades of every palette reach it: the
        // transformation's, and the custom screen's white.)
        let dialogue = if transform != Fade::None { transform } else { crate::custom::sprite_fade(b).unwrap_or_default() };
        let fades = Fades { stage, hud, dialogue, sprites, screen: screen_fade(b) };
        let layers = [&self.dialogue, &self.names, &self.hud, &self.field, &self.background];
        let (pixels, depth) = compose::compose_with_depth(backdrop, &layers, &parts, fades);
        // Each item's depth and fades: its layer's (the HUD layer's moved
        // with its shake), or its sprite parts' (an item whose parts the
        // sprite limit dropped isn't drawn).
        note_missing_strings(&mut text, &mut self.problems);
        let text = text
            .into_items()
            .into_iter()
            .filter_map(|(plane, mut item)| {
                let (depth, fade) = match plane {
                    Plane::Hud => {
                        item.rect = item.rect.offset(-jx, -jy);
                        item.clip = item.clip.offset(-jx, -jy);
                        (compose::layer_depth(&self.hud), fades.hud)
                    }
                    Plane::Bg0 => (compose::layer_depth(&self.names), fades.hud),
                    Plane::Sprite(tag) => {
                        let i = tags.iter().position(|&t| t == Some(tag))?;
                        (compose::sprite_depth(parts[i].priority, i), fades.sprites)
                    }
                };
                item.depth = depth;
                item.fades = [fade, fades.screen];
                Some(item)
            })
            .collect();
        Frame { pixels, depth, text }
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
        // The first battle of a set fades in from white, later ones from
        // black (`sub_80E0684`); a console whose game's flow says
        // `intro_from_black` (EXE5's), every battle from black.
        let s = &b.setup.settings;
        let later = if s.effects & nettai_battle::setup::effects::SET != 0 {
            b.round.round > 1
        } else {
            b.content.stage(s.stage).battle_number >= 2
        };
        let own_game = b.content.rules();
        let fade = if later || own_game.flow.intro_from_black { Fade::Black } else { Fade::White };
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
