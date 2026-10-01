//! A body overlay (actor object #0x56, `sub_80C4348`): a second sprite
//! layered on a navi, such as the helmet and arm a Cross adds to MegaMan.
//! It follows its owner's animation, position, visibility and facing
//! every tick, and is drawn in front of the owner or one pixel further
//! back depending on the owner's animation. Purely visual, but it holds an
//! actor slot and a place in the update order.
//! See docs/engine/objects-and-player.md §12.10.

use crate::battle::Battle;
use crate::content::BodyOverlay;
use crate::kinds::common;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Vec3, flags, state};

/// Overlay-private state (the spawn parameters, and ExtraVars[0]).
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct Vars {
    /// Param1: which overlay (`ObjectData::body_overlays`).
    pub variant: u8,
    /// Param2: the overlay keeps its own palette rather than its owner's
    /// (presentation only).
    pub own_palette: bool,
    /// Param3: step the sprite even while paused (`sub_801BCD0`), not
    /// just like any object (`object_updateSprite`, and not while
    /// dimmed).
    pub always_step: bool,
    /// Param4: added to the owner's animation.
    pub anim_offset: u8,
    /// ExtraVars[0] (`sub_80C4526`): drawn one pixel in front whatever
    /// the owner's animation.
    pub forced_front: bool,
}

fn vars(b: &Battle, r: ObjectRef) -> &Vars {
    match &b.objects.get(r).vars {
        crate::kinds::Vars::BodyOverlay(v) => v,
        v => panic!("body overlay with {v:?}"),
    }
}

fn vars_mut(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::BodyOverlay(v) => v,
        v => panic!("body overlay with {v:?}"),
    }
}

/// `sub_80C44A8`: layer overlay `variant` on `owner`. It runs its first
/// update right after the owner's, and keeps running while paused.
pub fn spawn(b: &mut Battle, owner: ObjectRef, variant: u8, own_palette: bool) -> Option<ObjectRef> {
    spawn_with(b, owner, Vars { variant, own_palette, ..Vars::default() })
}

/// `sub_80C44A8` with all its parameters (`forced_front` is ignored: it
/// is set later, by `sub_80C4526`).
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, spec: Vars) -> Option<ObjectRef> {
    let Vars { variant, own_palette, always_step, anim_offset, .. } = spec;
    let params = [variant, own_palette as u8, always_step as u8, anim_offset];
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::BodyOverlay, Vec3::default(), params)?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flip = flip;
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    *vars_mut(b, r) = Vars { forced_front: false, ..spec };
    Some(r)
}

/// `sub_80C4526(overlay, 1)`: draw it in front whatever the owner's
/// animation.
pub fn force_front(b: &mut Battle, r: ObjectRef) {
    vars_mut(b, r).forced_front = true;
}

/// `sub_80C44C8`: remove the overlay at its next update.
pub fn remove(b: &mut Battle, r: ObjectRef) {
    common::set_progress(b, r, common::Progress::DESTROY);
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

fn overlay(b: &Battle, r: ObjectRef) -> &BodyOverlay {
    b.content.objects.body_overlay(vars(b, r).variant)
}

fn owner(b: &Battle, r: ObjectRef) -> ObjectRef {
    b.objects.get(r).related[0].expect("body overlay has an owner")
}

/// `sub_80C4368`: load the sprite, then follow the owner.
fn init(b: &mut Battle, r: ObjectRef) {
    let sprite = overlay(b, r).sprite;
    let anim = vars(b, r).anim_offset;
    let own_palette = vars(b, r).own_palette;
    let owner_palette = b.objects.sprite(owner(b, r)).look.palette;
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    // sprite_noShadow; its palette 0, or its owner's.
    s.look.shadow = Shadow::WithSprite;
    s.look.palette = if own_palette { 0 } else { owner_palette };
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    // A halfword store: the animation, and 0 as the loaded one.
    o.anim = anim;
    o.anim_loaded = 0;
    common::set_progress(b, r, common::Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C43C4`: follow the owner, then step the sprite.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let Vars { anim_offset, forced_front, always_step, .. } = *vars(b, r);
    let (owner_anim, owner_pos, owner_flags, owner_flip) = {
        let o = b.objects.get(owner);
        (o.anim, o.pos, o.flags, o.flip)
    };
    let in_front = |anim: u8| {
        *overlay(b, r).in_front.get(anim as usize).unwrap_or_else(|| {
            panic!("the body overlay depth for animation {anim:#x} reads past the depth tables into their pointers (sub_80C43C4)")
        })
    };
    let nudge = if forced_front {
        0x1_0000
    } else if !in_front(owner_anim) {
        -0x1_0000
    } else {
        0
    };
    let o = b.objects.get_mut(r);
    o.anim = owner_anim.wrapping_add(anim_offset);
    o.pos = Vec3 { x: owner_pos.x, y: owner_pos.y.wrapping_add(nudge), z: owner_pos.z.wrapping_add(nudge) };
    // phase_init set (`sub_80C44E4` / `sub_80C44FA`) holds the visibility.
    if o.phase_init == 0 {
        o.flags = (o.flags & !flags::VISIBLE) | (owner_flags & flags::VISIBLE);
    }
    o.flip = owner_flip;
    let alliance = o.alliance;
    // The owner's palette (unless it has its own), colour shader, white
    // flash, alpha and facing.
    let own_palette = vars(b, r).own_palette;
    let owner_look = b.objects.sprite(owner).look;
    let look = &mut b.objects.sprite_mut(r).look;
    if !own_palette {
        look.palette = owner_look.palette;
    }
    look.color_shader = owner_look.color_shader;
    look.white = owner_look.white;
    look.alpha = owner_look.alpha;
    look.set_flip(alliance ^ owner_flip);
    // Action 0, `sub_80C4484`.
    if always_step {
        common::step_sprite(b, r);
    } else if !b.is_dimmed() {
        common::update_sprite(b, r);
    }
}
