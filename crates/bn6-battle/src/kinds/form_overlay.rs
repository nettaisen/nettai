//! A form overlay (actor object #0x57, `sub_80C4530`): a second sprite
//! layered on a navi, such as the Falzar beast head MegaMan wears in Beast
//! Out. It mirrors its owner's animation, position, visibility and facing
//! every tick. Purely visual, but it holds an actor slot and a place in the
//! update order. See docs/engine/objects-and-player.md §A.7.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::kinds::common::{self, Progress, set_progress};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x57;

/// The Falzar beast head (sprite 0x0C/0x0A).
pub const BEAST_HEAD: SpriteId = SpriteId { category: 0x0C, index: 0x0A };

/// How the overlay's sprite steps once it runs (Param3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stepping {
    /// `object_updateSprite`, and not in time stop. The overlay stops
    /// running while the battle is paused.
    #[default]
    Normal,
    /// `object_updateSpriteTimestop`; keeps running while paused.
    InTimeStop,
    /// `sub_801BCD0`, paused or not; keeps running while paused.
    Always,
}

/// Overlay-private state.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    pub sprite: Option<SpriteId>,
    /// Sits one pixel higher and nearer than its owner.
    pub nudged: bool,
    /// Added to the owner's animation.
    pub anim_offset: u8,
    pub stepping: Stepping,
    // (`sub_80C46C6` can make an overlay hold its sprite still while the
    // owner is dragged or paralyzed; no ported spawner uses it.)
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
    let params = [sprite.category, sprite.index, 0, 0];
    let r = b.objects.spawn(Pool::Actor, INDEX, Vec3::default(), params)?;
    let alliance = b.objects.get(owner).alliance;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.alliance = alliance;
    o.flags |= flags::RUN_WHILE_PAUSED;
    let v = vars(b, r);
    v.sprite = Some(sprite);
    v.nudged = nudged;
    Some(r)
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
    let sprite = vars(b, r).sprite.expect("form overlay has a sprite");
    let anim = b.objects.get(owner(b, r)).anim.wrapping_add(vars(b, r).anim_offset);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.set_animation(anim);
    s.update();
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = anim;
    set_progress(b, r, Progress::UPDATE);
    tick(b, r);
}

/// `sub_80C458C`: follow the owner, then (once the navis are in) step
/// the sprite.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = owner(b, r);
    let Vars { nudged, anim_offset, stepping, .. } = vars(b, r).clone();
    let (owner_anim, owner_pos, owner_flags, owner_flip) = {
        let o = b.objects.get(owner);
        (o.anim, o.pos, o.flags, o.flip)
    };
    let anim = owner_anim.wrapping_add(anim_offset);
    b.objects.get_mut(r).anim = anim;
    if anim != b.objects.get(r).anim_loaded {
        // Restarts every tick until the sprite step below records it.
        b.objects.sprite_mut(r).set_animation(anim);
    }
    let nudge = if nudged { 0x1_0000 } else { 0 };
    let o = b.objects.get_mut(r);
    o.pos = Vec3 { x: owner_pos.x, y: owner_pos.y.wrapping_sub(nudge), z: owner_pos.z.wrapping_sub(nudge) };
    o.flags = (o.flags & !flags::VISIBLE) | (owner_flags & flags::VISIBLE);
    o.flip = owner_flip;
    let alliance = o.alliance;
    // The owner's colour shader, white flash and mosaic, and its facing.
    let owner_look = b.objects.sprite(owner).look;
    let look = &mut b.objects.sprite_mut(r).look;
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
    match stepping {
        Stepping::Normal => {
            if !b.is_time_stop() {
                common::update_sprite(b, r);
            }
        }
        Stepping::InTimeStop => common::update_sprite_in_time_stop(b, r),
        Stepping::Always => common::step_sprite(b, r),
    }
}
