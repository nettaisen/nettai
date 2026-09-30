//! The bubble around a bubbled navi (effect object #0x3C, `sub_80E4A1C`):
//! it sits at the navi's sprite attach point 0x1E, shows and hides with
//! it, and pops (its animation 3, then gone) once the navi's link to it is
//! cleared or the navi is no longer bubbled. Spawned by the status routine
//! (`status.rs`), which links it in the navi's collision data. See
//! docs/engine/objects-and-player.md.

use crate::battle::Battle;
use crate::collision::{f1, link};
use crate::content::SpriteId;
use crate::object::{ObjectRef, flags, state};

pub const INDEX: u8 = 0x3C;

const SPRITE: SpriteId = SpriteId { category: 0x0C, index: 0x20 };
/// Floating, and popping.
const ANIM_FLOAT: u8 = 2;
const ANIM_POP: u8 = 3;
/// The navi's attach point it sits at, and how far it sits in front of
/// and below it (16.16).
const ATTACH_POINT: usize = 0x1E;
const OFFSET: i32 = 0x2_0000;

/// `sub_80E4B34`: a bubble on `owner`, linked in the owner's collision
/// data. It runs while paused and while dimmed. (The game spawns it at
/// the status routine's leftover registers; its init places it at once.)
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let pos = b.objects.get(owner).pos;
    let r = crate::kinds::spawn_engine(b, crate::kinds::EngineKind::BubbleVisual, pos, [0; 4])?;
    let (alliance, flip) = {
        let o = b.objects.get(owner);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.flip = flip;
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
    let c = b.objects.get(owner).collision.expect("a bubble on an object without collision data");
    b.collision.get_mut(c).links[link::BUBBLE] = Some(r);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => follow(b, r),
        _ => pop(b, r),
    }
    if b.objects.get(r).flags & flags::ACTIVE == 0 {
        return;
    }
    if b.objects.get(r).state == state::DESTROY {
        crate::kinds::common::step_sprite(b, r);
    } else {
        step(b, r);
    }
}

/// `sub_80E4A6E`: the floating bubble.
fn init(b: &mut Battle, r: ObjectRef) {
    let s = b.objects.sprite_mut(r);
    s.load(SPRITE);
    s.set_animation(0, &b.content);
    s.look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.flags = (o.flags | flags::VISIBLE) & !flags::NO_SPRITE_UPDATE;
    o.anim = ANIM_FLOAT;
    o.anim_loaded = ANIM_FLOAT;
    let flip = o.alliance ^ o.flip;
    let s = b.objects.sprite_mut(r);
    s.set_animation(ANIM_FLOAT, &b.content);
    s.update(&b.content);
    s.look.set_flip(flip);
    let o = b.objects.get_mut(r);
    o.state = state::UPDATE;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
    follow(b, r);
}

/// `sub_80E4AAE`: at the navi's attach point, 2 pixels nearer and lower;
/// hidden off the field; showing as the navi does; popped once the navi's
/// link to it is gone or the navi isn't bubbled. (It also takes the navi's
/// mosaic as its alpha: presentation.)
fn follow(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("a bubble without its owner");
    let (dx, dz) = crate::kinds::player::attach_point(b, owner, ATTACH_POINT);
    let p = b.objects.get(owner).pos;
    let o = b.objects.get_mut(r);
    o.pos.x = p.x.wrapping_add(dx << 16);
    o.pos.y = p.y.wrapping_sub(OFFSET);
    o.pos.z = p.z.wrapping_add(dz << 16).wrapping_sub(OFFSET);
    crate::kinds::common::set_panels_from_coordinates(b, r);
    let o = b.objects.get_mut(r);
    if !crate::field::is_valid(o.panel.x, o.panel.y) {
        o.flags &= !flags::VISIBLE;
    }
    let c = b.objects.get(owner).collision.expect("a bubble on an object without collision data");
    if b.collision.get(c).links[link::BUBBLE].is_some() {
        let shown = b.objects.get(owner).flags & flags::VISIBLE;
        let o = b.objects.get_mut(r);
        o.flags = (o.flags & !flags::VISIBLE) | shown;
        if b.collision.get(c).f1 & f1::BUBBLED != 0 {
            return;
        }
    }
    b.collision.get_mut(c).links[link::BUBBLE] = None;
    let o = b.objects.get_mut(r);
    o.state = state::DESTROY;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_80E4A4C`: pop, then go once the animation has played.
fn pop(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).action == 0 {
        let o = b.objects.get_mut(r);
        o.action = 3;
        o.anim = ANIM_POP;
        return;
    }
    if b.objects.sprite(r).frame_parameters() & crate::object::sprite::FRAME_LAST != 0 {
        b.objects.free(r);
    }
}

/// `sub_801BC24`: load a newly requested animation (without stepping it),
/// else step the sprite; not while paused, nor while dimmed unless it runs
/// while dimmed.
fn step(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get(r);
    if b.paused || o.flags & flags::ACTIVE == 0 || o.flags & flags::NO_SPRITE_UPDATE != 0 {
        return;
    }
    if o.flags & flags::RUN_WHILE_DIMMED == 0 && b.is_dimmed() {
        return;
    }
    let (anim, loaded) = (o.anim, o.anim_loaded);
    if anim != loaded {
        b.objects.sprite_mut(r).set_animation(anim, &b.content);
        b.objects.get_mut(r).anim_loaded = anim;
        return;
    }
    b.objects.sprite_mut(r).update(&b.content);
}
