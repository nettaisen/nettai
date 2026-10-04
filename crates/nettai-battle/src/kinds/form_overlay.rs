//! A form overlay (actor object #0x57, `sub_80C4530`): a second sprite
//! layered on a navi, such as the Falzar beast head MegaMan wears in Beast
//! Out. It mirrors its owner's animation, position, visibility and facing
//! every tick. Purely visual, but it holds an actor slot and a place in the
//! update order. See docs/engine/objects-and-player.md §A.7.

use crate::battle::Battle;
use crate::content::SpriteId;
use crate::kinds::common::{self, Progress, set_progress};
use crate::object::{ObjectRef, Vec3, flags, state};

/// How the overlay's sprite steps once it runs (Param3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Stepping {
    /// `object_updateSprite`, and not while dimmed. The overlay stops
    /// running while the battle is paused.
    #[default]
    Normal = 0,
    /// `object_updateSpriteTimestop`; keeps running while paused.
    WhileDimmed = 1,
    /// `sub_801BCD0`, paused or not; keeps running while paused.
    Always = 2,
}

impl Stepping {
    /// Param3's value (anything but 1 and 2 steps normally).
    pub fn from_param(v: u8) -> Stepping {
        match v {
            1 => Stepping::WhileDimmed,
            2 => Stepping::Always,
            _ => Stepping::Normal,
        }
    }
}

/// The palette the overlay wears (ExtraVars+4 and +8, `sub_80C46CC`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Palette {
    /// Its owner's (ExtraVars+4 = 0xFF).
    Owner,
    /// Its own.
    #[default]
    Own,
    /// Its own, but 0xC while its side's mood is Full Synchro
    /// (`sub_80C46C0`: the Falzar beast head).
    Mood,
}

/// Overlay-private state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub sprite: Option<SpriteId>,
    /// ExtraVars[0]: sits one pixel higher and nearer than its owner.
    pub nudged: bool,
    /// Added to the owner's animation.
    pub anim_offset: u8,
    /// Param3.
    pub stepping: Stepping,
    pub palette: Palette,
    /// ExtraVars+4: its own palette.
    pub palette_index: u8,
    /// ExtraVars+0xC (`sub_80C46C6`): the sprite holds still while the
    /// owner is dragged or paralyzed.
    pub holds_while_stunned: bool,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::FormOverlay(v) => v,
        v => panic!("form overlay with {v:?}"),
    }
}

/// `sub_80C468C`: layer `sprite` on `owner`. It runs its first update
/// right after the owner's, even while paused.
pub fn spawn(b: &mut Battle, owner: ObjectRef, sprite: SpriteId, nudged: bool) -> Option<ObjectRef> {
    spawn_with(b, owner, Vars { sprite: Some(sprite), nudged, ..Vars::default() })
}

/// `sub_80C468C` with all of its parameters (Param3 from `stepping`,
/// ExtraVars from the rest).
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, spec: Vars) -> Option<ObjectRef> {
    let sprite = spec.sprite.expect("a form overlay has a sprite");
    // (The sprite's handle in the first two parameters.)
    let [hi, lo] = sprite.0.to_be_bytes();
    let params = [hi, lo, spec.stepping as u8, spec.anim_offset];
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::FormOverlay, Vec3::default(), params)?;
    let alliance = b.objects.get(owner).alliance;
    let paused = b.game_rules().effects.overlays_run_while_paused;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    if paused {
        o.flags |= flags::RUN_WHILE_PAUSED;
    }
    *vars(b, r) = spec;
    Some(r)
}

/// `sub_80C4526(overlay, 1)`: sit one pixel higher and nearer.
pub fn nudge(b: &mut Battle, r: ObjectRef) {
    vars(b, r).nudged = true;
}

/// `sub_80C46CC`: the palette the overlay shows.
fn palette(b: &Battle, r: ObjectRef, v: &Vars) -> u8 {
    match v.palette {
        Palette::Owner => b.objects.sprite(owner(b, r)).look.palette,
        Palette::Own => v.palette_index,
        Palette::Mood => {
            let side = b.objects.get(r).alliance as usize;
            if b.stats[side].mood == 0xFF { 0x0C } else { v.palette_index }
        }
    }
}

/// Make the overlay step its sprite `stepping`'s way from now on.
pub fn set_stepping(b: &mut Battle, r: ObjectRef, stepping: Stepping) {
    vars(b, r).stepping = stepping;
}

/// `sub_80C44D2`: restart the overlay's current animation and step it
/// (after its owner restarted an animation).
pub fn restart(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).anim_loaded = 0xFF;
    common::step_sprite(b, r);
}

/// `sub_80C4530`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("form overlay has an owner")
}

/// `sub_80C4550`: load the sprite on the owner's animation.
fn init(b: &mut Battle, r: ObjectRef) {
    let v = vars(b, r).clone();
    let sprite = v.sprite.expect("form overlay has a sprite");
    let sum = u16::from(b.objects.get(owner(b, r)).anim) + u16::from(v.anim_offset);
    let anim = sum as u8;
    let palette = palette(b, r, &v);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    s.look.palette = palette;
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    // One halfword store sets both: the loaded animation gets the sum's
    // carry, so unless the animation is 0 the first tick restarts it,
    // undoing the step above (which keeps the overlay in step with its
    // owner).
    o.anim_loaded = (sum >> 8) as u8;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C458C`: follow the owner, then (once the navis are in) step
/// the sprite.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let v = vars(b, r).clone();
    let Vars { nudged, anim_offset, stepping, holds_while_stunned, .. } = v;
    let palette = palette(b, r, &v);
    let (owner_anim, owner_pos, owner_flip) = {
        let o = b.objects.get(owner);
        (o.anim, o.pos, o.flip)
    };
    let owner_shown = [0u8, 1].map(|v| b.visible_to(owner, v));
    let anim = owner_anim.wrapping_add(anim_offset);
    b.objects.get_mut(r).anim = anim;
    if anim != b.objects.get(r).anim_loaded {
        // Restarts every tick until the sprite step below records it.
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
    }
    let nudge = if nudged { 0x1_0000 } else { 0 };
    b.set_visible_by_viewer(r, owner_shown);
    let o = b.objects.get_mut(r);
    o.pos = Vec3 { x: owner_pos.x, y: owner_pos.y.wrapping_sub(nudge), z: owner_pos.z.wrapping_sub(nudge) };
    o.flip = owner_flip;
    let alliance = o.alliance;
    // The owner's color shader, white flash and mosaic, and its facing.
    let owner_look = b.objects.sprite(owner).look;
    let look = &mut b.objects.sprite_mut(r).look;
    look.palette = palette;
    look.color_shader = owner_look.color_shader;
    look.white = owner_look.white;
    look.mosaic = owner_look.mosaic;
    look.set_flip(alliance ^ owner_flip);
    let o = b.objects.get_mut(r);
    if o.action == 0 {
        // Wait for every navi to be in.
        if b.round.intro_bits & 0x02 == 0 {
            return;
        }
        let o = b.objects.get_mut(r);
        if stepping == Stepping::Normal {
            o.flags &= !flags::RUN_WHILE_PAUSED;
        }
        o.action = 4;
        o.phase = 0;
        o.phase_init = 0;
    }
    // sub_80C464C: held still while the owner is dragged or paralyzed.
    if holds_while_stunned {
        let stunned = b.objects.get(owner).collision.is_some_and(|c| b.collision.get(c).f1 & (crate::collision::f1::DRAG | crate::collision::f1::PARALYZED) != 0);
        if stunned {
            return;
        }
    }
    match stepping {
        Stepping::Normal => {
            if !b.is_dimmed() {
                common::update_sprite(b, r);
            }
        }
        Stepping::WhileDimmed => common::update_sprite_while_dimmed(b, r),
        Stepping::Always => common::step_sprite(b, r),
    }
}
