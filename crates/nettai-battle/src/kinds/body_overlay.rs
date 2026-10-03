//! A body overlay (actor object #0x56, `sub_80C4348`): a second sprite
//! layered on a navi, such as the helmet and arm a Cross adds to MegaMan.
//! It follows its owner's animation, position, visibility and facing
//! every tick, and is drawn in front of the owner or one pixel further
//! back depending on the owner's animation. Purely visual, but it holds an
//! actor slot and a place in the update order.
//! See docs/engine/objects-and-player.md §12.10.

use crate::battle::Battle;
use crate::content::{BodyPart, Parts};
use crate::kinds::common;
use crate::object::sprite::Shadow;
use crate::object::{ObjectRef, Vec3, flags, state};

/// Overlay-private state (the spawn parameters, and ExtraVars[0]).
#[derive(Clone, Copy, Debug, Default, Hash)]
pub struct Vars {
    /// Param1: which overlay, as the body part of the identity that wears
    /// it (of two, the second).
    pub identity: Option<nettai_content_api::IdentityHandle>,
    pub second: bool,
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

/// `sub_80C44A8`: layer the body overlay `spec` names on `owner`
/// (`forced_front` is ignored: it is set later, by `sub_80C4526`). It runs
/// its first update right after the owner's, and keeps running while
/// paused.
pub fn spawn_with(b: &mut Battle, owner: ObjectRef, spec: Vars) -> Option<ObjectRef> {
    let Vars { own_palette, always_step, anim_offset, .. } = spec;
    // (Param1, the overlay's number, is the identity's part here.)
    let params = [0, own_palette as u8, always_step as u8, anim_offset];
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

fn overlay(b: &Battle, r: ObjectRef) -> &BodyPart {
    let v = vars(b, r);
    match (&b.content.identity(v.identity).parts, v.second) {
        (Some(Parts::Body(part)), false) | (Some(Parts::Bodies(part, _)), false) | (Some(Parts::Bodies(_, part)), true) => part,
        _ => panic!("body overlay without the identity's part it is"),
    }
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
    let (owner_anim, owner_pos, owner_flip) = {
        let o = b.objects.get(owner);
        (o.anim, o.pos, o.flip)
    };
    let owner_shown = [0u8, 1].map(|v| b.visible_to(owner, v));
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
    let holds = o.phase_init != 0;
    o.flip = owner_flip;
    let alliance = o.alliance;
    if !holds {
        b.set_visible_by_viewer(r, owner_shown);
    }
    // The owner's palette (unless it has its own), color shader, white
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
