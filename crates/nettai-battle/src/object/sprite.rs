//! Sprite animation timing. Nothing is drawn here; the engine steps
//! animations because some behaviors end when an animation does (effect
//! lifetimes, some chip attacks). Frame durations come from the content's
//! sprite definitions.
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
/// frontend draws with it. Its fields are the local side's console's
/// (`RoundSetup::local_side`), which the recordings check; where the other
/// side's console draws the sprite otherwise, `other` says how.
///
/// The fields that can differ between the consoles (the palette, the
/// horizontal flip, the tint, the blending, the mosaic, the hidden parts)
/// are set through their setters, which set them for every console.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Look {
    /// `sprite_setPalette`: the palette within the frame's palette set.
    pub palette: u8,
    /// `sprite_setFlip`: mirrored horizontally / vertically.
    pub hflip: bool,
    pub vflip: bool,
    /// What the first part of each frame (the shadow) does.
    pub shadow: Shadow,
    /// `sprite_setField0x3Bit5` (the sprite's flag 0x20, which nothing
    /// clears): while the first part is a ground shadow, the rest of the
    /// sprite is drawn as it is, under every object's sprite, and all of
    /// it at the sprite's height: a mark on the floor.
    pub under_objects: bool,
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
    /// The frame (animation, frame) whose first part's palette offset the
    /// sprite is drawn with, added to `palette` (the sprite's +5,
    /// `oObjectSprite_Unk_05`): the frame each step ends on, and in EXE6
    /// the frame an animation starts on too (the arena's
    /// `effects.load_sets_part_palette`). None after loading: 0.
    pub part_palette: Option<(u8, u16)>,
    /// What the other side's console draws differently.
    pub other: OtherLook,
}

/// How the other side's console draws a sprite where it differs from the
/// local side's ([`Look`]'s own fields): where each console of the
/// original decides a look by its own side (the counter blink only the
/// player in Full Synchro sees, the round intro's fade-in of the other
/// side's navi, a mark turned to the console's own side, a palette by whose
/// panel it is, the parts a console hides). Each field none where the
/// consoles agree. Presentation: [`Battle::look_for`].
///
/// [`Battle::look_for`]: crate::battle::Battle::look_for
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OtherLook {
    pub palette: Option<u8>,
    pub hflip: Option<bool>,
    pub color_shader: Option<u16>,
    pub alpha: Option<Option<u8>>,
    pub mosaic: Option<Option<u8>>,
    pub hidden_parts: Option<u32>,
}

impl Default for Look {
    fn default() -> Look {
        Look {
            palette: 0,
            hflip: false,
            vflip: false,
            shadow: Shadow::Hidden,
            under_objects: false,
            white: false,
            color_shader: 0,
            alpha: None,
            mosaic: None,
            priority: 2,
            hidden_parts: 0,
            part_palette: None,
            other: OtherLook::default(),
        }
    }
}

impl Look {
    /// `sprite_setPalette`, on every console.
    pub fn set_palette(&mut self, palette: u8) {
        self.palette = palette;
        self.other.palette = None;
    }

    /// The horizontal flip, on every console.
    pub fn set_hflip(&mut self, on: bool) {
        self.hflip = on;
        self.other.hflip = None;
    }

    /// `sprite_setFlip` with the game's flip value (bit 0 horizontal, bit
    /// 1 vertical), on every console.
    pub fn set_flip(&mut self, flip: u8) {
        self.set_hflip(flip & 1 != 0);
        self.vflip = flip & 2 != 0;
    }

    /// `sprite_setColorShader`, on every console.
    pub fn set_color_shader(&mut self, shader: u16) {
        self.color_shader = shader;
        self.other.color_shader = None;
    }

    /// `sprite_setAlpha`, on every console.
    pub fn set_alpha(&mut self, alpha: Option<u8>) {
        self.alpha = alpha;
        self.other.alpha = None;
    }

    /// `sprite_setMosaicSize`, on every console.
    pub fn set_mosaic(&mut self, mosaic: Option<u8>) {
        self.mosaic = mosaic;
        self.other.mosaic = None;
    }

    /// The hidden parts, on every console.
    pub fn set_hidden_parts(&mut self, parts: u32) {
        self.hidden_parts = parts;
        self.other.hidden_parts = None;
    }

    /// `owner`'s palette, as each console draws it (an overlay in its
    /// owner's colors).
    pub fn copy_palette(&mut self, owner: &Look) {
        self.palette = owner.palette;
        self.other.palette = owner.other.palette;
    }

    /// `owner`'s tint, white flash, mosaic and blending, as each console
    /// draws them (an overlay drawn as its owner is).
    pub fn copy_shading(&mut self, owner: &Look) {
        self.color_shader = owner.color_shader;
        self.white = owner.white;
        self.mosaic = owner.mosaic;
        self.alpha = owner.alpha;
        self.other.color_shader = owner.other.color_shader;
        self.other.mosaic = owner.other.mosaic;
        self.other.alpha = owner.other.alpha;
    }

    /// The look as the other side's console draws it.
    pub fn for_other(&self) -> Look {
        let o = self.other;
        Look {
            palette: o.palette.unwrap_or(self.palette),
            hflip: o.hflip.unwrap_or(self.hflip),
            color_shader: o.color_shader.unwrap_or(self.color_shader),
            alpha: o.alpha.unwrap_or(self.alpha),
            mosaic: o.mosaic.unwrap_or(self.mosaic),
            hidden_parts: o.hidden_parts.unwrap_or(self.hidden_parts),
            other: OtherLook::default(),
            ..*self
        }
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
    fn frame_at(&self, content: &Content, i: u16) -> AnimFrame {
        // (An animation the sprite hasn't, or a frame past its last, is a
        // single held frame.)
        self.id.and_then(|id| content.animations.frame(id, self.anim, i)).unwrap_or(AnimFrame { duration: 1, flags: FRAME_LAST })
    }

    /// Load a sprite (resets the animation state).
    pub fn load(&mut self, id: SpriteId) {
        *self = Sprite { id: Some(id), ..Sprite::default() };
    }

    /// Start animation `anim` from its first frame (timing from
    /// `content`): `sprite_loadAnimationData`, which in EXE6 (`sub_3006730`)
    /// also takes the frame's palette offset and in EXE5 (0x03006898)
    /// leaves it to the next step.
    pub fn set_animation(&mut self, anim: u8, content: &Content) {
        self.anim = anim;
        self.frame = 0;
        let f = self.frame_at(content, 0);
        self.count = f.duration;
        self.frame_flags = f.flags;
        if content.rules().effects.load_sets_part_palette {
            self.look.part_palette = Some((anim, 0));
        }
    }

    /// Advance one tick (timing from `content`), ending on the palette
    /// offset of the frame it is on (`sub_3006792`'s end, EXE5's 0x03006948).
    pub fn update(&mut self, content: &Content) {
        self.step(content);
        self.look.part_palette = Some((self.anim, self.frame));
    }

    fn step(&mut self, content: &Content) {
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
