//! Object behaviors the engine implements itself (`EngineKind`), reached
//! through the content's kind registry (`content::defs`), plus helpers
//! shared by many behaviors (HP changes, damage formulas). A kind the
//! content pack's scripts implement runs as content instead (`behavior`).

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
pub mod hit_marker;
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

use bn6_content_api::{KindHandle, SpawnAt};

use crate::battle::Battle;
use crate::object::{New, ObjectRef, Pool, Vec3};

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
    Spark(spark::Vars),
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
    /// An engine kind's state at spawn.
    pub fn for_engine(kind: EngineKind) -> Vars {
        match kind {
            EngineKind::Intro => Vars::Intro(Default::default()),
            EngineKind::ChargeGlow => Vars::ChargeGlow(Default::default()),
            EngineKind::Effect => Vars::Effect(Default::default()),
            EngineKind::Spark => Vars::Spark(Default::default()),
            EngineKind::Hitbox => Vars::Hitbox(Default::default()),
            EngineKind::FormOverlay => Vars::FormOverlay(Default::default()),
            EngineKind::Afterimage => Vars::Afterimage(Default::default()),
            EngineKind::LockonMarker => Vars::LockonMarker(Default::default()),
            EngineKind::PaletteFlash => Vars::PaletteFlash(Default::default()),
            EngineKind::CrossMerge => Vars::CrossMerge(Default::default()),
            EngineKind::BodyOverlay => Vars::BodyOverlay(Default::default()),
            EngineKind::IdleOverlay => Vars::IdleOverlay(Default::default()),
            EngineKind::FullSynchroAura => Vars::FullSynchroAura(Default::default()),
            EngineKind::BeastOverBurst => Vars::BeastOverBurst(Default::default()),
            EngineKind::Player
            | EngineKind::BubbleVisual
            | EngineKind::NaviChip
            | EngineKind::NaviWarp
            | EngineKind::Eruption
            | EngineKind::StatusVisual
            | EngineKind::IceVisual
            | EngineKind::HitMarker => Vars::None,
        }
    }
}

/// Spawn an object of kind `kind` where `at` says in the update list, with
/// its kind's state at spawn: an engine kind's, or a content kind's zeroed
/// declared state. None when its pool is full.
pub fn spawn(b: &mut Battle, kind: KindHandle, at: SpawnAt, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    use crate::content::KindImpl;
    let k = b.content.defs.kind(kind);
    let vars = match k.implementation {
        KindImpl::Engine(e) => Vars::for_engine(e),
        KindImpl::Script { .. } => Vars::Content(bn6_content_api::ContentState::new(k.schema)),
    };
    let new = New { pool: k.pool, kind, vars, pos, params };
    match at {
        SpawnAt::AfterCurrent => b.objects.spawn(new),
        SpawnAt::First => b.objects.spawn_at_front(new),
        SpawnAt::End => b.objects.spawn_at_end(new),
    }
}

/// Spawn one of the engine's kinds, right after the object updating.
pub fn spawn_engine(b: &mut Battle, kind: EngineKind, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let h = b.content.defs.engine(kind);
    spawn(b, h, SpawnAt::AfterCurrent, pos, params)
}

/// Spawn the kind registration by number puts in object slot `index` of
/// `pool` (the numeric spawns of the pack's scripts). A slot nothing fills
/// is a kind not ported yet.
pub fn spawn_numbered(b: &mut Battle, pool: Pool, index: u8, at: SpawnAt, pos: Vec3, params: [u8; 4]) -> Option<ObjectRef> {
    let Some(kind) = b.content.defs.kind_at(pool, index) else {
        if !b.objects.has_room(pool) {
            return None;
        }
        panic!("object kind {pool:?} {index:#x} is not implemented yet")
    };
    spawn(b, kind, at, pos, params)
}

/// The object kinds the engine implements itself (the rest are content's:
/// `content::defs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EngineKind {
    Player,
    Intro,
    ChargeGlow,
    Effect,
    Hitbox,
    Spark,
    BubbleVisual,
    IceVisual,
    HitMarker,
    FormOverlay,
    Afterimage,
    LockonMarker,
    PaletteFlash,
    CrossMerge,
    BodyOverlay,
    IdleOverlay,
    FullSynchroAura,
    BeastOverBurst,
    NaviChip,
    NaviWarp,
    Eruption,
    StatusVisual,
}

/// The engine's kinds: their keys (`engine/...`), pools, and the object
/// slots they fill (the original's pool and index, which the traces
/// compare).
pub const ENGINE_KINDS: [(EngineKind, &str, Pool, u8); 22] = [
    (EngineKind::Player, "engine/player", Pool::Actor, 0),
    (EngineKind::Intro, "engine/intro", Pool::Effect, 2),
    (EngineKind::ChargeGlow, "engine/charge-glow", Pool::Effect, 8),
    (EngineKind::Effect, "engine/effect", Pool::Effect, 0),
    (EngineKind::Hitbox, "engine/hitbox", Pool::Attack, 3),
    (EngineKind::Spark, "engine/spark", Pool::Effect, 4),
    (EngineKind::BubbleVisual, "engine/bubble-visual", Pool::Effect, bubble_visual::INDEX),
    (EngineKind::IceVisual, "engine/ice-visual", Pool::Effect, ice_visual::INDEX),
    (EngineKind::HitMarker, "engine/hit-marker", Pool::Effect, hit_marker::INDEX),
    (EngineKind::FormOverlay, "engine/form-overlay", Pool::Actor, form_overlay::INDEX),
    (EngineKind::Afterimage, "engine/afterimage", Pool::Effect, afterimage::INDEX),
    (EngineKind::LockonMarker, "engine/lockon-marker", Pool::Effect, lockon_marker::INDEX),
    (EngineKind::PaletteFlash, "engine/palette-flash", Pool::Effect, palette_flash::INDEX),
    (EngineKind::CrossMerge, "engine/cross-merge", Pool::Actor, cross_merge::INDEX),
    (EngineKind::BodyOverlay, "engine/body-overlay", Pool::Actor, body_overlay::INDEX),
    (EngineKind::IdleOverlay, "engine/idle-overlay", Pool::Actor, idle_overlay::INDEX),
    (EngineKind::FullSynchroAura, "engine/full-synchro-aura", Pool::Actor, full_synchro_aura::INDEX),
    (EngineKind::BeastOverBurst, "engine/beast-over-burst", Pool::Effect, beast_over_burst::INDEX),
    (EngineKind::NaviChip, "engine/navi-chip", Pool::Effect, navi_chip::INDEX),
    (EngineKind::NaviWarp, "engine/navi-warp", Pool::Actor, navi_warp::INDEX),
    (EngineKind::Eruption, "engine/eruption", Pool::Attack, eruption::INDEX),
    (EngineKind::StatusVisual, "engine/status-visual", Pool::Effect, status_visual::INDEX),
];

/// Run one object's update: its kind's.
pub fn update(b: &mut Battle, r: ObjectRef) {
    use crate::content::KindImpl;
    let kind = b.objects.get(r).kind;
    match b.content.defs.kind(kind).implementation {
        KindImpl::Script { update } => crate::behavior::run_object(b, kind, update, r),
        KindImpl::Engine(k) => match k {
            EngineKind::Player => player::update(b, r),
            EngineKind::Intro => intro::update(b, r),
            EngineKind::ChargeGlow => charge_glow::update(b, r),
            EngineKind::Effect => effect::update(b, r),
            EngineKind::Hitbox => hitbox::update(b, r),
            EngineKind::Spark => spark::update(b, r),
            EngineKind::BubbleVisual => bubble_visual::update(b, r),
            EngineKind::IceVisual => ice_visual::update(b, r),
            EngineKind::HitMarker => hit_marker::update(b, r),
            EngineKind::FormOverlay => form_overlay::update(b, r),
            EngineKind::Afterimage => afterimage::update(b, r),
            EngineKind::LockonMarker => lockon_marker::update(b, r),
            EngineKind::PaletteFlash => palette_flash::update(b, r),
            EngineKind::CrossMerge => cross_merge::update(b, r),
            EngineKind::BodyOverlay => body_overlay::update(b, r),
            EngineKind::IdleOverlay => idle_overlay::update(b, r),
            EngineKind::FullSynchroAura => full_synchro_aura::update(b, r),
            EngineKind::BeastOverBurst => beast_over_burst::update(b, r),
            EngineKind::NaviChip => navi_chip::update(b, r),
            EngineKind::NaviWarp => navi_warp::update(b, r),
            EngineKind::Eruption => eruption::update(b, r),
            EngineKind::StatusVisual => status_visual::update(b, r),
        },
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

    /// NumbrBl's damage (formula 21) is the last two digits of its user's
    /// HP.
    #[test]
    fn formula_21_is_the_players_hp_mod_100() {
        use crate::content::testing;
        let stats = crate::setup::NaviStats { hp: 1000, max_hp: 1000, max_base_hp: 1000, ..Default::default() };
        let setup = testing::round_setup(testing::LINK_BATTLE, stats);
        let mut b = crate::battle::Battle::new(setup, testing::content());
        b.spawn_actors();
        b.run_objects();
        for (hp, want) in [(1234, 34), (100, 0), (99, 99)] {
            let p = b.player(1).unwrap();
            b.objects.get_mut(p).hp = hp;
            assert_eq!(super::chip_damage_formula(&b, testing::SUN_GUN_3, 1, 21), want, "HP {hp}");
        }
    }

    #[test]
    fn deletion_times_read_as_bcd_clock_times() {
        assert_eq!(time_bcd(600), 0x1000, "10 seconds");
        assert_eq!(time_bcd(203), 0x0338, "3.38 seconds");
        assert_eq!(time_bcd(216_000 + 3600 * 2 + 61), 0x0102_0101);
        assert_eq!(time_bcd(0x149_9728), 0x9959_5999, "capped");
    }
}
