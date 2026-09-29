//! A player's buster charge glow (effect #8). Purely visual, but it holds a
//! pool slot and a place in the update order for the whole round, and its
//! lifetime follows the player's. See docs/engine/objects-and-player.md §A.5.

use crate::battle::Battle;
use crate::data::SpriteId;
use crate::object::{ObjectRef, Pool, Vec3, flags, state};
use crate::sound::SoundId;

#[derive(Clone, Debug, Default, Hash)]
pub struct Vars {
    /// Other effects can hide the glow.
    pub enabled: bool,
    /// The glow sprite loaded (index in category 0x14; 0 = none yet).
    pub sprite: u8,
    /// The owner's charge level this tick and last tick.
    pub level: u8,
    pub previous_level: u8,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut b.objects.get_mut(r).vars {
        crate::kinds::Vars::ChargeGlow(v) => v,
        v => panic!("charge glow with {v:?}"),
    }
}

/// Spawn the glow for `owner` (`sub_80E0F02`). Its initial X/Y/Z are
/// leftover registers in the game (§A.5); they are never read.
pub fn spawn(b: &mut Battle, owner: ObjectRef) -> Option<ObjectRef> {
    let actor = b.objects.get(owner).actor?;
    let r = b.objects.spawn(Pool::Effect, 8, Vec3::default(), [0; 4])?;
    let o = b.objects.get_mut(r);
    o.related[0] = Some(owner);
    o.flags |= flags::RUN_WHILE_PAUSED;
    b.actors.get_mut(actor).charge_glow = Some(r);
    Some(r)
}

/// `sub_80E0DF0`.
pub fn update(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).state {
        state::INIT => {
            // sub_80E0E10
            vars(b, r).enabled = true;
            b.objects.get_mut(r).state = state::UPDATE;
            tick(b, r);
        }
        state::UPDATE => tick(b, r),
        _ => b.objects.free(r),
    }
}

/// `sub_80E0E20`: follow the owner's charge: level as animation, shown
/// only while charging (and to the local side unless the B button
/// charges), at the owner's attach point 0.
fn tick(b: &mut Battle, r: ObjectRef) {
    let owner = b.objects.get(r).related[0].expect("charge glow has an owner");
    let actor = b.objects.get(owner).actor.expect("charge glow owner has actor data");
    if b.paused {
        if b.actors.get(actor).charge_source == 0 {
            b.objects.get_mut(r).flags &= !flags::VISIBLE;
        }
        return;
    }
    if b.actors.get(actor).charge_glow.is_none() {
        let o = b.objects.get_mut(r);
        o.state = state::DESTROY;
        o.action = 0;
        o.phase = 0;
        o.phase_init = 0;
        return;
    }
    let alliance = b.objects.get(owner).alliance;
    let panel = b.objects.get(owner).panel;
    let visible = viewer_sees(b, alliance) && vars(b, r).enabled && crate::field::is_valid(panel.x, panel.y);
    let source = b.actors.get(actor).charge_source;
    let shown_to_side = match source {
        0 => false,
        2 => true,
        _ => !b.is_remote(alliance),
    };
    set_visible(b, r, visible && shown_to_side);
    select_sprite(b, r, source);
    let level = b.actors.get(actor).charge_level;
    let v = vars(b, r);
    v.previous_level = v.level;
    v.level = level;
    b.objects.get_mut(r).anim = level;
    if level == 0 {
        set_visible(b, r, false);
    }
    charge_sound(b, r, alliance, source);
    let (dx, dz) = crate::kinds::player::attach_point(b, owner, 0);
    let p = b.objects.get(owner).pos;
    b.objects.get_mut(r).pos = Vec3 { x: p.x.wrapping_add(dx << 16), y: p.y, z: p.z.wrapping_add(dz << 16) };
    crate::kinds::player::update_sprite(b, r);
}

/// `sub_80E0F5E`: the charge sounds, as the charge starts and as it
/// completes; only the charging navi's player hears a charge unless it
/// comes from source 2.
fn charge_sound(b: &mut Battle, r: ObjectRef, alliance: u8, source: u8) {
    let v = vars(b, r);
    let id = match (v.level, v.previous_level) {
        (1, 0) => SoundId::BUSTER_CHARGE,
        (2, 1) => SoundId::BUSTER_CHARGED,
        _ => return,
    };
    if source == 2 {
        b.play_sound(id);
    } else {
        b.play_sound_for(alliance, id);
    }
}

fn set_visible(b: &mut Battle, r: ObjectRef, on: bool) {
    let o = b.objects.get_mut(r);
    if on {
        o.flags |= flags::VISIBLE;
    } else {
        o.flags &= !flags::VISIBLE;
    }
}

/// `sub_800EB6C`: the local player sees `alliance`'s effects unless they
/// belong to the other side and the local navi is blind.
fn viewer_sees(b: &Battle, alliance: u8) -> bool {
    if !b.is_remote(alliance) {
        return true;
    }
    let Some(viewer) = b.player(alliance ^ 1) else { return true };
    let blind = b.objects.get(viewer).collision.map(|c| b.collision.get(c).f1 & crate::collision::f1::BLIND != 0);
    !blind.unwrap_or(false)
}

/// `sub_80E0F2E`: the A charge glows differently from the B charge.
fn select_sprite(b: &mut Battle, r: ObjectRef, source: u8) {
    let wanted = if source == 1 { 0x15 } else { 8 };
    if vars(b, r).sprite == wanted {
        return;
    }
    vars(b, r).sprite = wanted;
    b.objects.sprite_mut(r).load(SpriteId { category: 0x14, index: wanted });
    b.objects.sprite_mut(r).look.shadow = crate::object::sprite::Shadow::WithSprite;
    let o = b.objects.get_mut(r);
    o.anim_loaded = 0xFF;
    o.flags &= !flags::NO_SPRITE_UPDATE;
}
