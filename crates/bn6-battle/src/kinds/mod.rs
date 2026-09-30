//! Object behaviors the engine implements itself, chosen by pool and
//! index, plus helpers shared by many behaviors (HP changes, damage
//! formulas). A kind the content pack's scripts implement runs as content
//! instead (`behavior`).

pub mod absorbed_obstacle;
pub mod afterimage;
pub mod body_overlay;
pub mod charge_glow;
pub mod common;
pub mod cross_merge;
pub mod effect;
pub mod elmnt_man;
pub mod eruption;
pub mod form_overlay;
pub mod heal;
pub mod hitbox;
pub mod intro;
pub mod lockon_marker;
pub mod meteor;
pub mod navi_chip;
pub mod navi_warp;
pub mod obstacle;
pub mod palette_flash;
pub mod player;
pub mod spark;
pub mod status_visual;

use crate::battle::Battle;
use crate::object::{ObjectRef, Pool};

/// Behavior-private state. The game gives every object 0x2C (actors,
/// attacks) or 0x1C (effects) bytes of scratch; here each behavior gets a
/// typed struct, zeroed at spawn like the game's.
#[derive(Clone, Debug, Default, Hash)]
pub enum Vars {
    #[default]
    None,
    Intro(intro::Vars),
    ChargeGlow(charge_glow::Vars),
    Effect(effect::Vars),
    Hitbox(hitbox::Vars),
    AbsorbedObstacle(absorbed_obstacle::Vars),
    FormOverlay(form_overlay::Vars),
    Afterimage(afterimage::Vars),
    LockonMarker(lockon_marker::Vars),
    PaletteFlash(palette_flash::Vars),
    CrossMerge(cross_merge::Vars),
    BodyOverlay(body_overlay::Vars),
    NaviChip(navi_chip::Vars),
    NaviWarp(navi_warp::Vars),
    ElmntMan(elmnt_man::Vars),
    Meteor(meteor::Vars),
    StatusVisual(status_visual::Vars),
    /// A content kind's declared state (see `content`).
    Content(bn6_content_api::ContentState),
}

impl Vars {
    pub fn for_kind(pool: Pool, index: u8) -> Vars {
        match (pool, index) {
            (Pool::Effect, 2) => Vars::Intro(Default::default()),
            (Pool::Effect, 8) => Vars::ChargeGlow(Default::default()),
            (Pool::Effect, 0) => Vars::Effect(Default::default()),
            (Pool::Attack, 3) => Vars::Hitbox(Default::default()),
            (Pool::Actor, form_overlay::INDEX) => Vars::FormOverlay(Default::default()),
            (Pool::Effect, afterimage::INDEX) => Vars::Afterimage(Default::default()),
            (Pool::Effect, lockon_marker::INDEX) => Vars::LockonMarker(Default::default()),
            (Pool::Effect, palette_flash::INDEX) => Vars::PaletteFlash(Default::default()),
            (Pool::Actor, cross_merge::INDEX) => Vars::CrossMerge(Default::default()),
            (Pool::Actor, body_overlay::INDEX) => Vars::BodyOverlay(Default::default()),
            (Pool::Actor, 0) => Vars::None,
            _ => Vars::None,
        }
    }
}

/// Run one object's update.
pub fn update(b: &mut Battle, r: ObjectRef) {
    let index = b.objects.get(r).index;
    if let Some(kind) = b.behaviors.object_kind(r.pool, index) {
        return crate::behavior::run_object(b, kind, r);
    }
    match (r.pool, index) {
        (Pool::Actor, 0) => player::update(b, r),
        (Pool::Effect, 2) => intro::update(b, r),
        (Pool::Effect, 8) => charge_glow::update(b, r),
        (Pool::Effect, 0) => effect::update(b, r),
        (Pool::Attack, 3) => hitbox::update(b, r),
        (Pool::Effect, 4) => spark::update(b, r),
        (Pool::Effect, absorbed_obstacle::INDEX) => absorbed_obstacle::update(b, r),
        (Pool::Actor, form_overlay::INDEX) => form_overlay::update(b, r),
        (Pool::Effect, afterimage::INDEX) => afterimage::update(b, r),
        (Pool::Effect, lockon_marker::INDEX) => lockon_marker::update(b, r),
        (Pool::Effect, palette_flash::INDEX) => palette_flash::update(b, r),
        (Pool::Actor, cross_merge::INDEX) => cross_merge::update(b, r),
        (Pool::Actor, body_overlay::INDEX) => body_overlay::update(b, r),
        (Pool::Effect, navi_chip::INDEX) => navi_chip::update(b, r),
        (Pool::Actor, navi_warp::INDEX) => navi_warp::update(b, r),
        (Pool::Actor, elmnt_man::INDEX) => elmnt_man::update(b, r),
        (Pool::Attack, meteor::INDEX) => meteor::update(b, r),
        (Pool::Attack, eruption::INDEX) => eruption::update(b, r),
        (Pool::Effect, status_visual::INDEX) => status_visual::update(b, r),
        (pool, index) => panic!("object kind {pool:?} {index:#x} is not implemented yet"),
    }
}

/// `object_genericDestroy`: drop reservations, free collision, free.
pub fn generic_destroy(b: &mut Battle, r: ObjectRef) {
    b.release_reservations(r);
    if let Some(c) = b.objects.get(r).collision {
        b.collision.free(c);
    }
    b.objects.free(r);
}

/// `object_subtractHP`.
pub fn subtract_hp(b: &mut Battle, r: ObjectRef, amount: u16) {
    let o = b.objects.get_mut(r);
    o.hp = o.hp.saturating_sub(amount);
}

/// `sub_801A29A`: thaw a frozen object.
pub fn thaw(b: &mut Battle, r: ObjectRef) {
    let Some(c) = b.objects.get(r).collision else { return };
    let s = b.collision.get_mut(c);
    s.f1 &= !crate::collision::f1::FROZEN;
    s.f2 &= !crate::collision::f1::FROZEN;
    s.status_timers[crate::collision::timer::FREEZE] = 0;
}

/// `sub_800AF84`: busting level at the end of a round.
pub fn busting_level(_b: &Battle) -> u8 {
    // Only stored for the results screen; netbattles saw 0x0B.
    0x0B
}

/// `sub_802CEC8`: a defensive-chip record goes when the navi that used
/// the chip is deleted (its HP reaches 0).
pub fn update_linked_registry(b: &mut Battle) {
    for side in 0..2 {
        if let Some(owner) = b.linked[side].owner
            && b.objects.get(owner).hp == 0
        {
            let alliance = b.objects.get(owner).alliance;
            b.clear_linked(alliance);
        }
    }
}

/// `sub_802CDFE`: age the per-side damage-carry records.
pub fn shift_damage_carry(b: &mut Battle) {
    for side in 0..2 {
        let c = &mut b.damage_carry[side];
        c.previous = c.this_tick;
        c.this_tick = 0;
    }
}

/// Damage formulas for chips whose damage is 1000 or more (`off_80109DC`).
pub fn chip_damage_formula(b: &Battle, id: u16, side: u8, formula: u16) -> u16 {
    match formula {
        1..=18 => sp_chip_damage(b, id, side, formula as usize - 1),
        20 => damage_taken(b, side),
        _ => panic!("damage formula {formula} (chip {id:#x}) is not implemented yet"),
    }
}

/// `sub_8010BD0` (Muramasa's): the HP the side's player has lost, at most
/// 500. `sub_80103BC` looks for the player among the side's alive actors,
/// but its loop never advances, so it only ever checks the first slot four
/// times: with no player there the damage is 0.
fn damage_taken(b: &Battle, side: u8) -> u16 {
    let Some(r) = b.round.alive_actors[side as usize & 1][0] else { return 0 };
    let o = b.objects.get(r);
    if b.content.navi_record(o.name_id).actor_type != crate::actor::ActorType::Player {
        return 0;
    }
    // A signed difference, capped at 500 (an HP above the maximum would
    // give a negative damage, cut to 16 bits).
    let lost = o.max_hp as i32 - o.hp as i32;
    lost.min(500) as u16
}

/// `sub_8010AE4`: an SP navi chip's damage, lower the slower its user
/// deleted that SP navi (a step per two seconds past ten). `n` is the SP
/// navi (the chip's damage formula - 1).
fn sp_chip_damage(b: &Battle, id: u16, side: u8, n: usize) -> u16 {
    let time = time_bcd(b.setup.sp_times[side as usize].frames(n) as u32);
    let step = b.content.rules.sp_deletion_times.iter().take_while(|&&t| time > t).count();
    let damage = b.content.chip(id).sp_damage.as_ref();
    damage.unwrap_or_else(|| panic!("SP chip {id:#x} has no damage by deletion time"))[step]
}

/// `sub_8000D84`: frames as a BCD time, hours:minutes:seconds.hundredths
/// (a byte each), capped at 99:59:59.99.
fn time_bcd(frames: u32) -> u32 {
    if frames > 0x149_9727 {
        return 0x9959_5999;
    }
    let bcd = |v: u32| ((v / 10) << 4) | (v % 10);
    let (hours, rest) = (frames / 216_000, frames % 216_000);
    let (minutes, rest) = (rest / 3600, rest % 3600);
    let (seconds, frames) = (rest / 60, rest % 60);
    (bcd(hours) << 24) | (bcd(minutes) << 16) | (bcd(seconds) << 8) | bcd(frames * 100 / 60)
}

#[cfg(test)]
mod tests {
    use super::time_bcd;

    #[test]
    fn deletion_times_read_as_bcd_clock_times() {
        assert_eq!(time_bcd(600), 0x1000, "10 seconds");
        assert_eq!(time_bcd(203), 0x0338, "3.38 seconds");
        assert_eq!(time_bcd(216_000 + 3600 * 2 + 61), 0x0102_0101);
        assert_eq!(time_bcd(0x149_9728), 0x9959_5999, "capped");
    }
}
