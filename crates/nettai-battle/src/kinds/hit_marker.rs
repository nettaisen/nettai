//! The mark over a navi a hit told something about (effect object #0x6B,
//! `sub_80E807C`): the "!!" of a damaging weakness hit (animation 0) or
//! an HP bug's mark (animation 3), at a fixed offset from the navi,
//! following it, for one run of its animation. Spawned by the status
//! routine (`status.rs`, `sub_80E8124`).

use crate::battle::Battle;
use crate::object::{ObjectRef, Vec3, flags, state};
use crate::object::sprite::FRAME_LAST;


/// Its animation (Param1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// A weakness hit that did damage.
    Weakness = 0,
    /// An HP bug hit.
    Bug = 3,
}

/// `sub_80E8124`: the mark over `owner`, `offset` from its position (16.16).
pub fn spawn(b: &mut Battle, owner: ObjectRef, offset: Vec3, mark: Mark) -> Option<ObjectRef> {
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::HitMarker, offset, [mark as u8, 0, 0, 0])?;
    b.objects.get_mut(r).related[0] = Some(owner);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E809C`: the mark's animation, stepped once; the spawn position
/// is kept as the offset from the owner (in the velocity).
fn init(b: &mut Battle, r: ObjectRef) {
    let anim = b.objects.get(r).params[0];
    let sprite = b.roles_for(r).sprite(crate::content::SpriteRole::HitMarker);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.set_animation(0, &b.content);
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = anim;
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    let o = b.objects.get_mut(r);
    o.set_visible(true);
    o.vel = o.pos;
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    follow(b, r);
}

/// `sub_80E80E0`: at the owner, plus the offset and a pixel up and back;
/// hidden and done after the animation's last frame. The sprite steps
/// every tick, paused or dimmed.
fn follow(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("a hit mark without its owner");
    let p = b.objects.get(owner).pos;
    let o = b.objects.get_mut(r);
    let v = o.vel;
    o.pos = Vec3 { x: p.x.wrapping_add(v.x), y: p.y.wrapping_add(v.y), z: p.z.wrapping_add(v.z) };
    // The whole-pixel halves (Z16, Y16) go up by one.
    o.pos.z = o.pos.z.wrapping_add(0x1_0000);
    o.pos.y = o.pos.y.wrapping_add(0x1_0000);
    if b.objects.sprite(r).frame_parameters() & FRAME_LAST != 0 {
        let o = b.objects.get_mut(r);
        o.set_visible(false);
        o.state = state::DESTROY;
        o.action = 0;
        o.phase = 0;
        o.phase_init = 0;
    }
    b.objects.sprite_mut(r).update(&b.content);
}
