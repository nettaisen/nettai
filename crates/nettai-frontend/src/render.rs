//! Drawing a battle: everything visible is derived from the engine's state
//! and the pack's graphics.

use crate::audit::Problems;
use crate::compose::{self, Fade, Fades, Layer, Palettes};
use nettai_battle::transform::{SequencerState, TransformPhase};
use crate::hud::HudState;
use crate::objects::{self, SpriteList, View};
use crate::stage::{Stage, StageClock};
use nettai_assets::Bundle;
use nettai_battle::Battle;
use nettai_battle::battle::{FadeMode, mode};

/// Draws battles; keeps its layer buffers between frames.
pub struct Renderer<'a> {
    pub assets: &'a Bundle,
    background: Layer,
    field: Layer,
    hud: Layer,
    /// BG0, in front of everything: the custom screen's enemy names.
    names: Layer,
    /// Rolling HUD numbers; follow every tick with `observe`.
    pub hud_state: HudState,
    /// What the frames drawn so far named that the pack doesn't have.
    pub problems: Problems,
}

impl<'a> Renderer<'a> {
    pub fn new(assets: &'a Bundle) -> Renderer<'a> {
        Renderer {
            assets,
            // Background priority 3 (BG1), field 2 (BG2), HUD 1 (BG3).
            background: Layer::new(3, 1),
            field: Layer::new(2, 2),
            hud: Layer { palettes: Palettes::Hud, ..Layer::new(1, 3) },
            names: Layer { palettes: Palettes::Hud, ..Layer::new(0, 0) },
            hud_state: HudState::default(),
            problems: Problems::default(),
        }
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

    /// Draw a battle as a 240x160 BGR555 frame.
    pub fn render(&mut self, b: &Battle) -> Vec<u16> {
        let assets = self.assets;
        self.problems.known.clear();
        let view = Self::view(b);
        let stage = Stage::new(assets, b.setup.settings.background, StageClock::of(b));
        self.background.clear();
        stage.draw_background(&mut self.background);
        self.field.clear();
        stage.draw_field(b, &mut self.field, b.setup.local_side, &view);
        self.hud.clear();
        self.names.clear();
        let navi = crate::custom::navi_number(b, b.setup.local_side);
        let emblem = crate::custom::emblem_tiles(&assets.custom, crate::custom::version_name(b, b.setup.local_side), navi);
        let chatbox = crate::chatbox::prepare(b, assets, &mut self.problems);
        let mut list = SpriteList::default();
        objects::queue_objects(b, assets, &view, &mut list, &mut self.problems);
        crate::custom::draw(b, assets, &emblem, &mut self.hud, &mut self.names, &mut list, &mut self.problems);
        if let Some(c) = &chatbox {
            crate::chatbox::draw(c, assets, &mut self.names, &mut list);
        }
        crate::hud::draw(b, assets, &self.hud_state, &mut self.hud, &mut list, &mut self.problems);
        let (jx, jy) = crate::custom::hud_jitter(b);
        self.hud.shift(-jx, -jy);
        let parts = list.into_parts();
        let backdrop = stage.palettes[0][0];
        // The transformation's fade takes every background palette, a
        // dimming's the stage's; the custom screen's Beast Out the stage's
        // and the HUD's, its other fades the stage's (a dark chip's second
        // the HUD's).
        let transform = layer_fade(b);
        let custom = crate::custom::fade(b).unwrap_or_default();
        let custom_hud = crate::custom::hud_fade(b).unwrap_or_default();
        let flash = objects::palette_flash(b);
        let stage = if flash.is_some() {
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
        // leaves them be in variant 0.)
        let hud = match flash {
            Some(1) => Fade::White(16),
            Some(_) => custom_hud,
            None if transform != Fade::None => transform,
            None => custom_hud,
        };
        let sprites = if flash == Some(1) { Fade::White(16) } else { crate::custom::sprite_fade(b).unwrap_or_default() };
        let fades = Fades { stage, hud, sprites, screen: screen_fade(b) };
        compose::compose(backdrop, &[&self.names, &self.hud, &self.field, &self.background], &parts, fades)
    }
}

/// A dimming (`object_dimScreen`, `object_undimScreen`: fade modes 0x3C
/// and 0x38) darkens the first nine background palettes, the stage's, by a
/// sixteenth for every 0x10 of the fade's level: a quarter when dimmed.
/// The HUD's palettes and the sprites keep their colours.
pub fn dim_fade(b: &Battle) -> Fade {
    if !matches!(b.fade.mode, FadeMode::Dim | FadeMode::Undim) {
        return Fade::None;
    }
    Fade::Black((b.fade.level >> 4).min(16) as u8)
}

/// The transformation sequencer fades the tile layers (not the sprites)
/// out to black while the navis change form, and back in (a palette flash
/// takes its place: `objects::palette_flash`).
pub fn layer_fade(b: &Battle) -> Fade {
    let left = b.fade.remaining();
    match b.transform_seq.state {
        SequencerState::Transform { phase: TransformPhase::FadeOut, started: true } => Fade::Black(16u8.saturating_sub(left)),
        SequencerState::Transform { phase: TransformPhase::Change, .. } => Fade::Black(16),
        SequencerState::Transform { phase: TransformPhase::FadeIn, started } => Fade::Black(if started { left.min(16) } else { 16 }),
        _ => Fade::None,
    }
}

/// The screen fade: the battle fades in while the intro's fade runs
/// (`Fade::TICKS` frames) and fades out to black at the end. The screen
/// shows the fade one frame late (it is applied to the palettes at the
/// next vblank).
pub fn screen_fade(b: &Battle) -> Fade {
    let total = nettai_battle::battle::Fade::TICKS as u32;
    let left = (b.fade.remaining() as u32 + 1).min(total);
    if b.round.intro_bits & 0x01 == 0 {
        // The first battle of a set fades in from white, later ones from
        // black (`sub_80E0684`).
        let s = &b.setup.settings;
        let later = if s.effects & nettai_battle::setup::effects::SET != 0 {
            b.round.round > 1
        } else {
            b.content.stage(s.stage).battle_number >= 2
        };
        let fade = if later { Fade::Black } else { Fade::White };
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
