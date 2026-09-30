//! Object behaviors the engine implements itself, chosen by pool and
//! index, plus helpers shared by many behaviors (HP changes, damage
//! formulas). A kind the content pack's scripts implement runs as content
//! instead (`behavior`).

pub mod afterimage;
pub mod beast_over_burst;
pub mod body_overlay;
pub mod bubble_visual;
pub mod charge_glow;
pub mod common;
pub mod cross_merge;
pub mod effect;
pub mod eruption;
pub mod form_overlay;
pub mod full_synchro_aura;
pub mod heal;
pub mod hitbox;
pub mod ice_visual;
pub mod idle_overlay;
pub mod intro;
pub mod lockon_marker;
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
    FormOverlay(form_overlay::Vars),
    Afterimage(afterimage::Vars),
    LockonMarker(lockon_marker::Vars),
    PaletteFlash(palette_flash::Vars),
    CrossMerge(cross_merge::Vars),
    BodyOverlay(body_overlay::Vars),
    NaviChip(navi_chip::Vars),
    NaviWarp(navi_warp::Vars),
    StatusVisual(status_visual::Vars),
    IdleOverlay(idle_overlay::Vars),
    FullSynchroAura(full_synchro_aura::Vars),
    BeastOverBurst(beast_over_burst::Vars),
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
            (Pool::Actor, idle_overlay::INDEX) => Vars::IdleOverlay(Default::default()),
            (Pool::Actor, full_synchro_aura::INDEX) => Vars::FullSynchroAura(Default::default()),
            (Pool::Effect, beast_over_burst::INDEX) => Vars::BeastOverBurst(Default::default()),
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
        (Pool::Effect, bubble_visual::INDEX) => bubble_visual::update(b, r),
        (Pool::Effect, ice_visual::INDEX) => ice_visual::update(b, r),
        (Pool::Actor, form_overlay::INDEX) => form_overlay::update(b, r),
        (Pool::Effect, afterimage::INDEX) => afterimage::update(b, r),
        (Pool::Effect, lockon_marker::INDEX) => lockon_marker::update(b, r),
        (Pool::Effect, palette_flash::INDEX) => palette_flash::update(b, r),
        (Pool::Actor, cross_merge::INDEX) => cross_merge::update(b, r),
        (Pool::Actor, body_overlay::INDEX) => body_overlay::update(b, r),
        (Pool::Actor, idle_overlay::INDEX) => idle_overlay::update(b, r),
        (Pool::Actor, full_synchro_aura::INDEX) => full_synchro_aura::update(b, r),
        (Pool::Effect, beast_over_burst::INDEX) => beast_over_burst::update(b, r),
        (Pool::Effect, navi_chip::INDEX) => navi_chip::update(b, r),
        (Pool::Actor, navi_warp::INDEX) => navi_warp::update(b, r),
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
        0 => opponent_hp(b, side),
        1..=18 => sp_chip_damage(b, id, side, formula as usize - 1),
        19 => gauge_damage(b, side),
        20 => damage_taken(b, side),
        21 => hp_last_digits(b, side),
        22 => half_opponent_max_hp(b, side),
        23..=44 => navi_chip_damage(b, id, side),
        // The table ends at 44: the game jumps through the code after it.
        _ => panic!("damage formula {formula} (chip {id:#x}) reads past its table (off_80109DC)"),
    }
}

/// `BattleState+0x90`: side 1's alive actors, from which the single-player
/// formulas read the opponents (the first three for side 0, the first one
/// for side 1).
fn single_player_opponents(b: &Battle, side: u8) -> impl Iterator<Item = ObjectRef> + '_ {
    let n = if side & 1 == 0 { 3 } else { 1 };
    b.round.alive_actors[1][..n].iter().flatten().copied()
}

/// `sub_8010A90`: the opponent's HP, at most 500: in link battles the other
/// side's player's, else the highest of the opponents'.
fn opponent_hp(b: &Battle, side: u8) -> u16 {
    let hp = if b.setup.settings.effects & crate::setup::effects::LINK != 0 {
        let Some(p) = b.player(side ^ 1) else {
            panic!("damage formula 0 reads the HP of a missing navi (sub_8010A90)");
        };
        b.objects.get(p).hp
    } else {
        single_player_opponents(b, side).map(|r| b.objects.get(r).hp).max().unwrap_or(0)
    };
    hp.min(500)
}

/// `sub_8010B78`: damage by how full the custom gauge is (the side's own
/// gauge plus 0x1500 in the battle flag 0x40 mode): 10 to 32 over the first
/// half, to 128 by seven eighths, to 255 short of full; a full gauge (or
/// more) gives 10.
fn gauge_damage(b: &Battle, side: u8) -> u16 {
    let gauge = if b.round.flags & crate::battle::battle_flags::PER_PLAYER_GAUGES != 0 {
        b.sides[side as usize & 1].gauge as u32 + 0x1500
    } else {
        b.gauge.value as u32
    };
    let g = gauge >> 7;
    if g >= 0x80 {
        10
    } else if g <= 0x40 {
        (0x16 * g / 0x40 + 0xA) as u16
    } else if g <= 0x70 {
        (0x60 * (g - 0x40) / 0x30 + 0x20) as u16
    } else {
        (0x80 * (g - 0x70) / 0xF + 0x80) as u16
    }
}

/// `sub_8010BF0` (NumbrBl's): the last two digits of the side's player's
/// HP (0 without one).
fn hp_last_digits(b: &Battle, side: u8) -> u16 {
    b.player(side).map_or(0, |p| b.objects.get(p).hp % 100)
}

/// `sub_8010C06`: half the opponent's max HP, at most 999: in link battles
/// the other side's player's (0 without one), else the highest of the
/// opponents'.
fn half_opponent_max_hp(b: &Battle, side: u8) -> u16 {
    let max_hp = if b.setup.settings.effects & crate::setup::effects::LINK != 0 {
        let Some(p) = b.player(side ^ 1) else { return 0 };
        b.objects.get(p).max_hp
    } else {
        // Always the first three, whichever side.
        b.round.alive_actors[1][..3].iter().flatten().map(|&r| b.objects.get(r).max_hp).max().unwrap_or(0)
    };
    (max_hp >> 1).min(999)
}

/// `sub_8010BD0` (Muramasa's): the HP the side's player has lost, at most
/// 500. `sub_80103BC` (`Battle::player`) looks for the player among the
/// side's actors as spawned, but its loop never advances, so it only ever
/// checks the first slot four times: with no player there the damage is 0.
fn damage_taken(b: &Battle, side: u8) -> u16 {
    let Some(r) = b.player(side & 1) else { return 0 };
    let o = b.objects.get(r);
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

/// `sub_8010C50`: a link navi's chip's damage, from the side's player navi
/// (none: 0): its base, plus its step for each level of the navi's buster
/// attack (`sub_8012642`), up to 5.
fn navi_chip_damage(b: &Battle, id: u16, side: u8) -> u16 {
    let Some(navi) = b.player(side) else { return 0 };
    let d = b.content.chip(id).navi_damage.unwrap_or_else(|| panic!("link navi chip {id:#x} has no navi_damage"));
    let level = player::idle::buster_damage(b, navi).min(5);
    d.base as u16 + d.per_level as u16 * level
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
