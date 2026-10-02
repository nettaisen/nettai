//! Sprite animation timing. Nothing is drawn here; the engine steps
//! animations because some behaviors end when an animation does (effect
//! lifetimes, some chip attacks). Frame durations come from the content.
//! `Look` records how behaviors ask for the sprite to be drawn, for a
//! frontend.

use crate::content::{AnimFrame, Content, SpriteId};

/// Frame flag: this is the animation's last frame.
pub const FRAME_LAST: u8 = 0x80;
/// Frame flag: after the last frame, loop to the first.
pub const FRAME_LOOP: u8 = 0x40;

/// An object's sprite: which sprite is loaded and where its animation is.
#[derive(Clone, Debug, Default)]
pub struct Sprite {
    pub id: Option<SpriteId>,
    pub anim: u8,
    /// Index of the current frame within the animation.
    pub frame: u16,
    /// Ticks left on the current frame.
    pub count: u8,
    /// The current frame's flags.
    pub frame_flags: u8,
    /// How the sprite is drawn (presentation only; loading a sprite resets
    /// it).
    pub look: Look,
}

/// How a sprite is drawn. Nothing in the simulation reads this; behaviors
/// set it where the game calls the matching `sprite_*` routine, and a
/// frontend draws with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    /// `sprite_setPalette`: the palette within the frame's palette set.
    pub palette: u8,
    /// `sprite_setFlip`: mirrored horizontally / vertically.
    pub hflip: bool,
    pub vflip: bool,
    /// What the first part of each frame (the shadow) does.
    pub shadow: Shadow,
    /// `sprite_forceWhitePalette`: drawn solid white (a hit flash).
    pub white: bool,
    /// `sprite_setColorShader`: a palette tint (0 = none).
    pub color_shader: u16,
    /// `sprite_setAlpha`: blended over what is behind at alpha/16.
    pub alpha: Option<u8>,
    /// `sprite_setMosaicSize`: mosaic blocks of `n + 1` pixels.
    pub mosaic: Option<u8>,
    /// Hardware priority against the background layers (0 = frontmost;
    /// the field is 2).
    pub priority: u8,
    /// Parts not drawn: bit 31 - i hides part i (`sprite_setUnk0x2c`).
    pub hidden_parts: u32,
}

impl Default for Look {
    fn default() -> Look {
        Look {
            palette: 0,
            hflip: false,
            vflip: false,
            shadow: Shadow::Hidden,
            white: false,
            color_shader: 0,
            alpha: None,
            mosaic: None,
            priority: 2,
            hidden_parts: 0,
        }
    }
}

impl Look {
    /// `sprite_setFlip` with the game's flip value (bit 0 horizontal, bit
    /// 1 vertical).
    pub fn set_flip(&mut self, flip: u8) {
        self.hflip = flip & 1 != 0;
        self.vflip = flip & 2 != 0;
    }
}

/// The first part of every sprite frame is a shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Shadow {
    /// Not drawn (the state after loading).
    #[default]
    Hidden,
    /// `sprite_hasShadow`: drawn on the ground under the object, behind
    /// other sprites.
    Ground,
    /// `sprite_noShadow`: drawn with the rest of the sprite, at its height.
    WithSprite,
}

impl Sprite {
    fn frames<'c>(&self, content: &'c Content) -> &'c [AnimFrame] {
        self.id.map(|id| content.animation(id, self.anim)).unwrap_or(&[])
    }

    fn frame_at(&self, content: &Content, i: u16) -> AnimFrame {
        // A sprite without data behaves like a single held frame.
        self.frames(content).get(i as usize).copied().unwrap_or(AnimFrame { duration: 1, flags: FRAME_LAST })
    }

    /// Load a sprite (resets the animation state).
    pub fn load(&mut self, id: SpriteId) {
        *self = Sprite { id: Some(id), ..Sprite::default() };
    }

    /// Start animation `anim` from its first frame (timing from
    /// `content`).
    pub fn set_animation(&mut self, anim: u8, content: &Content) {
        self.anim = anim;
        self.frame = 0;
        let f = self.frame_at(content, 0);
        self.count = f.duration;
        self.frame_flags = f.flags;
    }

    /// Advance one tick (timing from `content`).
    pub fn update(&mut self, content: &Content) {
        loop {
            let old = self.count;
            self.count = old.wrapping_sub(1);
            if old != 0 {
                return;
            }
            if self.frame_flags & FRAME_LAST != 0 {
                if self.frame_flags & FRAME_LOOP != 0 {
                    self.set_animation(self.anim, content);
                } else {
                    self.count = 1;
                }
            } else {
                self.frame += 1;
                let f = self.frame_at(content, self.frame);
                self.count = f.duration;
                self.frame_flags = f.flags;
            }
        }
    }

    /// The current frame's flags as behaviors see them: the last-frame and
    /// loop bits only show once the frame's time is up.
    pub fn frame_parameters(&self) -> u8 {
        if self.count == 0 { self.frame_flags } else { self.frame_flags & !(FRAME_LAST | FRAME_LOOP) }
    }

    /// Whether a non-looping animation has finished.
    pub fn finished(&self) -> bool {
        self.frame_parameters() & FRAME_LAST != 0
    }
}
