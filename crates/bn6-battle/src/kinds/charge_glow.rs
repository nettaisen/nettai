//! A player's buster charge glow (effect #8). Purely visual, but it holds a
//! pool slot and a place in the update order for the whole round, and its
//! lifetime follows the player's. See docs/engine/objects-and-player.md §A.5.

use crate::battle::Battle;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};

#[derive(Clone, Debug, Default)]
pub struct Vars {
    pub enabled: u8,
    pub level: u8,
    pub previous_level: u8,
}

/// Spawn the glow for `owner` (`sub_80E0F02`).
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let actor = b.objects.get(owner).actor?;
    let r = b.objects.spawn(Pool::Effect, 8, Vec3::default(), [0; 4])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED;
    b.actors.get_mut(actor).charge_glow = Some(r);
    Some(r)
}

pub fn update(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).state == state::INIT {
        if let crate::kinds::Vars::ChargeGlow(v) = &mut b.objects.get_mut(r).vars {
            v.enabled = 1;
        }
        b.objects.get_mut(r).state = state::UPDATE;
    }
    if b.objects.get(r).state == state::DESTROY {
        b.objects.free(r);
        return;
    }
    let Some(owner) = b.objects.get(r).related[0] else { return };
    let Some(actor) = b.objects.get(owner).actor else { return };
    if b.paused {
        if b.actors.get(actor).charge_source == 0 {
            b.objects.get_mut(r).flags &= !flags::VISIBLE;
        }
        return;
    }
    if b.actors.get(actor).charge_glow != Some(r) {
        b.objects.get_mut(r).state = state::DESTROY;
        return;
    }
    let level = b.actors.get(actor).charge_level;
    let o = b.objects.get(owner);
    let (x, y, z) = (o.pos.x, o.pos.y, o.pos.z);
    let g = b.objects.get_mut(r);
    if let crate::kinds::Vars::ChargeGlow(v) = &mut g.vars {
        v.previous_level = v.level;
        v.level = level;
    }
    g.anim = level;
    // The sprite's attach offset is presentation; the trace compares it,
    // so it is filled in by the player behavior's sprite table later.
    g.pos.x = x;
    g.pos.y = y;
    g.pos.z = z;
}
