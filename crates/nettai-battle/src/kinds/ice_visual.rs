//! The ice block around a frozen navi (effect object #0x89, `sub_80E9AF0`):
//! it sits at the navi's sprite attach point 0x21, in the size that fits
//! the navi, shows and hides with it, and melts away (freed) once the
//! navi's link to it is cleared or the navi is no longer frozen. Spawned
//! by the status routine (`status.rs`), which links it in the navi's
//! collision data. See docs/engine/objects-and-player.md §H5.

use crate::battle::Battle;
use crate::collision::{f1, link};
use crate::object::{ObjectRef, flags, state};

/// The navi's attach point it sits at, and how far it sits behind and
/// above it (16.16).
const ATTACH_POINT: usize = 0x21;
const OFFSET: i32 = 0x6_0000;

/// `sub_80E9BDC`: an ice block on `owner`, linked in the owner's collision
/// data. It runs while paused and while dimmed. (The game spawns it at the
/// status routine's leftover registers; its init places it at once.)
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let pos = b.objects.get(owner).pos;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::IceVisual, pos, [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.flip = flip;
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    let c = b.objects.get(owner).collision.expect("an ice block on an object without collision data");
    b.collision.get_mut(c).links[link::FREEZE] = Some(r);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        // sub_8016C9C
        _ => b.objects.free(r),
    }
    crate::kinds::common::load_or_step_sprite(b, r);
}

/// `sub_80E9C06`: the block's size for the owner (its animation: 0 small,
/// 1 medium, 2 large): its identity's `ice` (the original's by the actor
/// record: `byte_80E9C30` for a virus, `byte_80E9C4E` for a navi or a
/// player, by AI index).
fn size(b: &Battle, owner: ObjectRef) -> u8 {
    b.content.identity(b.objects.get(owner).identity).ice as u8
}

/// `sub_80E9B14`: the block, in the owner's size.
fn init(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("an ice block without its owner");
    let anim = size(b, owner);
    let sprite = b.roles_for(r).sprite(crate::content::SpriteRole::Ice);
    let s = b.objects.sprite_mut(r);
    s.load(sprite);
    s.set_animation(0, &b.content);
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.set_visible(true);
    o.anim = anim;
    o.anim_loaded = anim;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.set_animation(anim, &b.content);
    s.update(&b.content);
    s.look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    follow(b, r);
}

/// `sub_80E9B56`: at the navi's attach point, 6 pixels behind and higher,
/// showing as the navi does (its panel check's hiding is overridden by
/// that); gone once the navi's link to it is gone or the navi isn't
/// frozen. (It also takes the navi's sprite priority and mosaic:
/// presentation.)
fn follow(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("an ice block without its owner");
    let (dx, dz) = crate::kinds::player::attach_point(b, owner, ATTACH_POINT);
    let p = b.objects.get(owner).pos;
    let o = b.objects.get_mut(r);
    o.pos.x = p.x.wrapping_add(dx << 16);
    o.pos.y = p.y.wrapping_add(OFFSET);
    o.pos.z = p.z.wrapping_add(dz << 16).wrapping_add(OFFSET);
    crate::kinds::common::set_panels_from_coordinates(b, r);
    let o = b.objects.get_mut(r);
    if !crate::field::is_valid(o.panel.x, o.panel.y) {
        o.set_visible(false);
    }
    b.copy_visibility(owner, r);
    let c = b.objects.get(owner).collision.expect("an ice block on an object without collision data");
    if b.collision.get(c).links[link::FREEZE].is_some() && b.collision.get(c).f1 & f1::FROZEN != 0 {
        return;
    }
    b.collision.get_mut(c).links[link::FREEZE] = None;
    let o = b.objects.get_mut(r);
    o.state = state::DESTROY;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}
