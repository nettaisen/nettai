//! An absorbed obstacle (effect object #0x87): what's left of an obstacle
//! the absorbing chip pulled in. It flies in 9 ticks to a point 20 pixels
//! in front of the absorbing navi and, if that navi is still absorbing,
//! joins its absorbed list. See docs/engine/field-objects.md.

use crate::actor::AbsorbedObstacle;
use crate::battle::Battle;
use crate::kinds::common::{self, Progress};
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

pub const INDEX: u8 = 0x87;

/// The absorbing navi's action while it absorbs (player action 0x58).
const ABSORBING_ACTION: u8 = 0x58;

/// Ticks to reach the navi.
const FLIGHT: u16 = 9;

/// An absorbed obstacle's navi keeps at most this many.
const MAX_ABSORBED: usize = 8;

/// What flies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spec {
    /// Obstacle kind (`data::ABSORBED_SPRITES`).
    pub kind: u8,
    /// The absorbing side.
    pub side: u8,
    /// The obstacle's animation.
    pub anim: u8,
    /// The obstacle's palette (drawn only).
    pub palette: u8,
}

/// Behavior state.
#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    pub kind: u8,
    pub side: u8,
    pub anim: u8,
    /// Where it flies to.
    pub target_x: i32,
    pub target_y: i32,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::AbsorbedObstacle(v) => v,
        _ => unreachable!("absorbed obstacle without its state"),
    }
}

/// `sub_80E996E`: spawned by `obstacle` at `pos`.
pub fn spawn(b: &mut Battle, obstacle: ObjectRef, pos: Vec3, spec: Spec) -> Option<ObjectRef> {
    let params = [spec.kind, spec.side, spec.anim, spec.palette];
    let r = b.objects.spawn(Pool::Effect, INDEX, pos, params)?;
    let (alliance, flip) = {
        let o = b.objects.get(obstacle);
        (o.alliance, o.flip)
    };
    let o = b.objects.get_mut(r);
    o.alliance = alliance;
    o.flip = flip;
    o.flags &= !(flags::RUN_WHILE_PAUSED | flags::RUN_IN_TIME_STOP);
    o.vars = crate::kinds::Vars::AbsorbedObstacle(Vars {
        kind: spec.kind,
        side: spec.side,
        anim: spec.anim,
        ..Default::default()
    });
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => init(b, r),
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E9810`: aim at the absorbing side's navi.
fn init(b: &mut Battle, r: ObjectRef) {
    common::set_panels_from_coordinates(b, r);
    let Vars { kind, side, anim, .. } = *vars(b, r);
    let sprite = b.objects.sprite_mut(r);
    sprite.load(crate::data::ABSORBED_SPRITES[kind as usize]);
    sprite.set_animation(anim);
    sprite.update();
    let o = b.objects.get_mut(r);
    o.flags &= !flags::NO_SPRITE_UPDATE;
    o.anim = anim;
    o.anim_loaded = anim;
    let navi = b.player(side).expect("the absorbing side has a navi");
    let n = b.objects.get(navi);
    let target_x = n.pos.x.wrapping_add((common::facing(n.alliance, n.flip) * 20) << 16);
    let target_y = n.pos.y;
    let v = vars(b, r);
    v.target_x = target_x;
    v.target_y = target_y;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(navi);
    o.timer = FLIGHT;
    let t = FLIGHT as i32;
    o.vel = Vec3 {
        x: target_x.wrapping_sub(o.pos.x) / t,
        y: target_y.wrapping_sub(o.pos.y) / t,
        z: (0x10_i32 << 16).wrapping_sub(o.pos.z) / t,
    };
    o.flags |= flags::VISIBLE;
    common::set_progress(b, r, Progress::UPDATE);
}

/// `sub_80E98DE`: fly; puff away when the battle ends or it leaves the
/// field.
fn tick(b: &mut Battle, r: ObjectRef) {
    let p = b.objects.get(r).panel;
    if b.is_battle_over() || !crate::field::is_valid(p.x, p.y) {
        let o = b.objects.get(r);
        let pos = Vec3 { z: o.pos.z.wrapping_add(0x10 << 16), ..o.pos };
        crate::kinds::effect::spawn(b, pos, 0x12, 0, 0, 0);
        common::set_progress(b, r, Progress::DESTROY);
        return;
    }
    // `sub_80E991C`, its only action.
    let o = b.objects.get_mut(r);
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    o.pos.y = o.pos.y.wrapping_add(o.vel.y);
    o.pos.z = o.pos.z.wrapping_add(o.vel.z);
    common::set_panels_from_coordinates(b, r);
    let navi = b.objects.get(r).related[0].expect("absorbed obstacle aims at a navi");
    if b.objects.get(navi).action == ABSORBING_ACTION {
        let o = b.objects.get_mut(r);
        let t = o.timer as i32 - 1;
        o.timer = t as u16;
        if t <= 0 {
            arrive(b, r, navi);
        }
    }
    b.objects.sprite_mut(r).update();
}

/// Join the navi's absorbed list, if it has room.
fn arrive(b: &mut Battle, r: ObjectRef, navi: ObjectRef) {
    let Vars { kind, .. } = *vars(b, r);
    let anim = b.objects.get(r).anim;
    let actor = b.objects.get(navi).actor.expect("absorbing navi has actor data");
    let absorbed = &mut b.actors.get_mut(actor).absorbed;
    if absorbed.len() < MAX_ABSORBED {
        absorbed.push(AbsorbedObstacle { kind, anim });
        common::set_progress(b, r, Progress::DESTROY);
    }
}
