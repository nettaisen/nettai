//! Sprite animation timing. Nothing is drawn here; the engine steps
//! animations because some behaviors end when an animation does (effect
//! lifetimes, some chip attacks). Frame durations come from `data`.

use crate::data::{self, AnimFrame, SpriteId};

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
}

impl Sprite {
    fn frames(&self) -> &'static [AnimFrame] {
        self.id.map(|id| data::animation(id, self.anim)).unwrap_or(&[])
    }

    fn frame_at(&self, i: u16) -> AnimFrame {
        // A sprite without data behaves like a single held frame.
        self.frames().get(i as usize).copied().unwrap_or(AnimFrame { duration: 1, flags: FRAME_LAST })
    }

    /// Load a sprite (resets the animation state).
    pub fn load(&mut self, id: SpriteId) {
        *self = Sprite { id: Some(id), ..Sprite::default() };
    }

    /// Start animation `anim` from its first frame.
    pub fn set_animation(&mut self, anim: u8) {
        self.anim = anim;
        self.frame = 0;
        let f = self.frame_at(0);
        self.count = f.duration;
        self.frame_flags = f.flags;
    }

    /// Advance one tick.
    pub fn update(&mut self) {
        loop {
            let old = self.count;
            self.count = old.wrapping_sub(1);
            if old != 0 {
                return;
            }
            if self.frame_flags & FRAME_LAST != 0 {
                if self.frame_flags & FRAME_LOOP != 0 {
                    self.set_animation(self.anim);
                } else {
                    self.count = 1;
                }
            } else {
                self.frame += 1;
                let f = self.frame_at(self.frame);
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
