//! Drawing a battle: everything visible is derived from the engine's state
//! and the pack's graphics.

use crate::audit::Problems;
use crate::compose::{self, Fade, Fades, Layer, Palettes};
use bn6_battle::transform::{SequencerState, TransformPhase};
use crate::hud::HudState;
use crate::objects::{self, SpriteList, View};
use crate::stage::{Stage, StageClock};
use bn6_assets::Bundle;
use bn6_battle::Battle;
use bn6_battle::battle::{FadeMode, mode};

/// Draws battles; keeps its layer buffers between frames.
pub struct Renderer<'a> {
    pub assets: &'a Bundle,
    background: Layer,
    field: Layer,
    hud: Layer,
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
    /// camera a shake moves this tick (`camera_doShakeEffect_80301e8`).
    pub fn view(b: &Battle) -> View {
        let local = b.setup.local_side;
        let (x, y) = b.consoles[local as usize & 1].camera.jitter;
        View { camera: (x, y, 0), mirror: local & 1 == 1 }
    }

    /// Draw a battle as a 240x160 BGR555 frame.
    pub fn render(&mut self, b: &Battle) -> Vec<u16> {
        let assets = self.assets;
        let view = Self::view(b);
        let stage = Stage::new(assets, b.setup.settings.background, StageClock::of(b));
        self.background.clear();
        stage.draw_background(&mut self.background);
        self.field.clear();
        stage.draw_field(b, &mut self.field, b.setup.local_side, &view);
        self.hud.clear();
        let mut list = SpriteList::default();
        objects::queue_objects(b, assets, &view, &mut list, &mut self.problems);
        crate::hud::draw(b, assets, &self.hud_state, &mut self.hud, &mut list, &mut self.problems);
        let parts = list.into_parts();
        let backdrop = stage.palettes[0][0];
        // The transformation's fade takes every background palette, a
        // dimming's the stage's.
        let transform = layer_fade(b);
        let stage = if transform == Fade::None { dim_fade(b) } else { transform };
        let fades = Fades { stage, hud: transform, screen: screen_fade(b) };
        compose::compose(backdrop, &[&self.hud, &self.field, &self.background], &parts, fades)
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
/// out to black while the navis change form, and back in; the palette
/// flash whitens them.
pub fn layer_fade(b: &Battle) -> Fade {
    if objects::palette_flash(b) {
        return Fade::White(16);
    }
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
    let total = bn6_battle::battle::Fade::TICKS as u32;
    let left = (b.fade.remaining() as u32 + 1).min(total);
    if b.round.intro_bits & 0x01 == 0 {
        // The first battle of a set fades in from white, later ones from
        // black (`sub_80E0684`).
        let s = &b.setup.settings;
        let later = if s.effects & bn6_battle::setup::effects::SET != 0 {
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
    if b.round.mode == mode::FADE_OUT || b.round.top == bn6_battle::battle::top::END {
        // The fade starts on the state's second tick.
        if b.round.top != bn6_battle::battle::top::END && b.round.init == 0 {
            return Fade::None;
        }
        if b.fade.active() {
            return Fade::Black((16 - (left * 16) / total) as u8);
        }
        return Fade::Black(16);
    }
    Fade::None
}
