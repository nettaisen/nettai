//! Stage A of the damage pipeline (`sub_801AC6C`): unregister the body
//! (resolving hits against whatever is on its panels), then turn this
//! tick's hit results into requests, statuses and the final damage. See
//! objects-and-player.md §H2 and field-collision-damage.md §4.1-§4.11.

use super::{
    ActorType, ai, ai_mut, battle_mode, clear_flag2, coll, coll_id, coll_mut, flag1, flag2, panel_kind,
    form_of, navi_of, refresh_navicust_state, reload_base_weapons, set_flag2, stats, stats_mut,
};
use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionData, f1, timer};
use crate::content::{StatusRole, StatusTimer};
use crate::field::PanelType;
use crate::object::{ObjectRef, Vec3};
use crate::setup::{Form, Navi};

/// `sub_801AC6C`.
pub(super) fn collect_hits(b: &mut Battle, r: ObjectRef) {
    // sprite_clearFinalPalette.
    b.objects.sprite_mut(r).look.white = false;
    if b.round.flags & battle_flags::FIGHTING == 0 {
        return;
    }
    b.remove_collision(coll_id(b, r));
    if b.is_battle_over() || flag1(b, r) & f1::DEAD != 0 {
        return;
    }
    barrier(b, r);
    standing_effects(b, r);
    slide_triggers(b, r);
    hp_bug_drain(b, r);
    drop_cursor_trap(b, r);
    anti_damage_traps(b, r);
    bug_hp_level(b, r);
    bug_paralyze_blind(b, r);
    bug_navicust(b, r);
    hit_modifier_requests(b, r);
    counter_paralysis(b, r);
    navicust_hit_bug(b, r);
    let s = stats(b, r);
    if !(navi_of(b, r) == Navi(7) || matches!(form_of(b, r).0, 7 | 0x13) || s.bugs.status_immunity) {
        apply_status(b, r);
    }
    lose_chip(b, r);
    drain_heal(b, r);
    final_damage(b, r);
    tick_counter_window(b, r);
    count_stun_ticks(b, r);
    anger_trigger(b, r);
    hit_ends_submerged(b, r);
    pierce_ends_flash(b, r);
    guard_spark(b, r);
}

/// `object_addHP`: heal, up to MaxHP.
pub(super) fn add_hp(b: &mut Battle, r: ObjectRef, n: u32) {
    let o = b.objects.get_mut(r);
    o.hp = (o.hp as u32 + n).min(o.max_hp as u32) as u16;
}

/// Zero this tick's damage and hit results (a barrier or trap absorbed
/// the hit). `+0x8C` (element 5 / poison) is not touched.
fn absorb_hit(c: &mut CollisionData) {
    for d in &mut c.acc.element_damage[..5] {
        *d = 0;
    }
    c.acc.mood_damage = 0;
    c.acc.counter = 0;
    c.acc.drain_hits = 0;
    c.hit_mod_final = 0;
    c.status_final = None;
    c.acc.inflicted_bugs = 0;
}

// ---- Barrier ---------------------------------------------------------------

/// A barrier's u16 view of HP (+0x16) and threshold (+0x17).
fn barrier_hp16(c: &CollisionData) -> u16 {
    c.barrier_hp as u16 | (c.barrier_threshold as u16) << 8
}

fn set_barrier_hp16(c: &mut CollisionData, v: u16) {
    c.barrier_hp = v as u8;
    c.barrier_threshold = (v >> 8) as u8;
}

/// `sub_801A802`: barriers and auras absorb this tick's hits (fed by the
/// raw channel), wear down, time out and regrow (§4.2).
fn barrier(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let dimmed = b.is_dimmed();
    let action = super::navi_action(b, r);
    let holy = {
        let c = coll(b, r);
        c.barrier != 0 && panel_kind(b, c.panel) == PanelType::Holy
    };
    let c = coll_mut(b, r);
    let mut barrier = c.barrier;
    if barrier == 0 {
        return;
    }
    let regrowing = matches!(barrier, 8 | 0xA) && barrier_hp16(c) == 0;
    if !regrowing && (c.acc.raw_elements & 0x20 != 0 || c.acc.raw_hit_flags & 0xA20 != 0) {
        // Popped (wind): absorbs everything until its visual clears it.
        barrier = 0x10;
        c.barrier = 0x10;
        set_barrier_hp16(c, 0);
        c.barrier_saved_hmf = c.hit_mod_final;
    }
    match barrier {
        8 => {
            if barrier_hp16(c) == 0 {
                // Regrows after 240 ticks (not while dimmed, frozen or bubbled).
                if dimmed || matches!(action, super::NaviAction::Freeze | super::NaviAction::Bubble) {
                    return;
                }
                c.barrier_timer = c.barrier_timer.wrapping_add(1);
                if c.barrier_timer >= 0xF0 {
                    set_barrier_hp16(c, 1);
                }
                return;
            }
        }
        0xA => {
            let hp = barrier_hp16(c);
            if hp == 0 {
                if dimmed {
                    return;
                }
                let t = ((c.barrier_timer >> 8) as u8).wrapping_add(1);
                c.barrier_timer = (c.barrier_timer & 0xFF) | (t as u16) << 8;
                if t >= 0x78 {
                    set_barrier_hp16(c, 0xC8);
                }
                return;
            }
            // Regenerates 1 HP every 6 ticks up to 200. At full HP the low
            // timer byte gets a stale register (the raw hit flags).
            let t = if hp < 0xC8 {
                let t = (c.barrier_timer as u8).wrapping_add(1);
                if t >= 6 {
                    set_barrier_hp16(c, hp + 1);
                    0
                } else {
                    t
                }
            } else {
                c.acc.raw_hit_flags as u8
            };
            c.barrier_timer = (c.barrier_timer & 0xFF00) | t as u16;
        }
        // A popped barrier times out even while dimmed.
        _ if (c.barrier == 0x10 || !dimmed) && c.barrier_timer != 0xFFFF => {
            let t = c.barrier_timer as i32 - 1;
            c.barrier_timer = t as u16;
            if t <= 0 {
                c.barrier = 0;
            }
        }
        _ => {}
    }
    // Elec breaks a type-8 barrier (and its damage counts double); the
    // weak element passes through.
    let breaks = if barrier == 8 && c.acc.raw_element_damage[3] != 0 {
        c.acc.element_damage[3] = c.acc.element_damage[3].wrapping_add(c.acc.elec_damage);
        true
    } else {
        let e = c.barrier_weak as usize;
        e != 0 && *c.acc.raw_element_damage.get(e).expect("barrier weak element") != 0
    };
    if breaks {
        c.barrier = 0;
        c.barrier_timer = 0;
        c.barrier_hp = 0;
        return;
    }
    let mut sum: u32 = c.acc.raw_element_damage[..5].iter().map(|&d| d as u32).sum();
    if holy {
        sum = (sum + 1) >> 1;
    }
    if sum >= c.barrier_threshold as u32 {
        let left = c.barrier_hp as i32 - sum as i32;
        c.barrier_hp = left as u8;
        if left <= 0 {
            if !matches!(barrier, 8 | 0xA) && c.barrier != 0x10 {
                c.barrier = 0;
            }
            c.barrier_timer = 0;
            c.barrier_hp = 0;
        }
    }
    absorb_hit(c);
    c.acc.exclamation = 0;
    c.acc.damage_multiplier = 0;
    c.acc.damage_elements = 0;
    c.acc.hit_flags &= !0x50;
}

// ---- Panels ------------------------------------------------------------------

/// `sub_801A186`: poison panels hurt 1 HP every 7 ticks (through
/// element 5); wood navis on grass heal.
fn standing_effects(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() || b.paused || coll(b, r).region == 0 {
        return;
    }
    let p = coll(b, r).panel;
    let Some(t) = b.field.panel(p.x, p.y).map(|p| p.kind) else { return };
    let f = flag1(b, r);
    let on_grass;
    if t == PanelType::Poison {
        if f & (0x0800_0000 | f1::FLOATSHOE | f1::INVULNERABLE) == 0 {
            let c = coll_mut(b, r);
            let v = c.poison_timer as i32 - 1;
            c.poison_timer = v as u8;
            if v < 0 {
                c.poison_timer = 6;
                c.acc.element_damage[5] = c.acc.element_damage[5].wrapping_add(1);
            }
            return;
        }
        // Immune: the game's grass test then compares the status flags
        // word, not the panel type, against the grass type.
        on_grass = f == PanelType::Grass as u32;
    } else {
        on_grass = t == PanelType::Grass;
    }
    coll_mut(b, r).poison_timer = 0;
    if !on_grass || b.objects.get(r).element & 0xF != 4 {
        return;
    }
    let cycle = if b.objects.get(r).hp > 9 { b.round.cycle20 } else { b.round.cycle180 };
    if cycle == 0 {
        add_hp(b, r, 1);
    }
}

/// `sub_801A36A`: start road slides, and ice slides at the end of a move
/// (consuming MOVE_COMPLETE).
fn slide_triggers(b: &mut Battle, r: ObjectRef) {
    let mut cooldown_ended = false;
    if !b.paused && !b.is_dimmed() && ai(b, r).road_cooldown != 0 {
        let a = ai_mut(b, r);
        a.road_cooldown -= 1;
        cooldown_ended = a.road_cooldown == 0;
    }
    if !cooldown_ended {
        let f = flag1(b, r);
        if f & (f1::DRAG | f1::MOVING) != 0 {
            return;
        }
        if panel_kind(b, coll(b, r).panel).is_road() {
            // sub_801A400
            if ai(b, r).road_cooldown == 0 && f & 0x24 == 0 {
                set_flag2(b, r, 0x10);
                b.objects.get_mut(r).slide_type = 3;
            }
            return;
        }
        if f & f1::MOVE_COMPLETE == 0 {
            return;
        }
    }
    super::clear_flag1(b, r, f1::MOVE_COMPLETE);
    let p = coll(b, r).panel;
    if b.field.panel(p.x, p.y).map(|p| p.kind) != Some(PanelType::Ice) {
        return;
    }
    // sub_801A3DA
    let f = flag1(b, r);
    if coll(b, r).element != 2 && f & 0x24 == 0 && f & f1::AFFECTED_BY_ICE != 0 {
        set_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_type = 2;
    }
}

// ---- NaviCust bugs and traps ------------------------------------------------------

/// `sub_8010230`: the NaviCust HP bug drains 1 HP every so many ticks
/// (never below 1).
fn hp_bug_drain(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() || b.paused || b.objects.get(r).hp <= 1 {
        return;
    }
    let level = stats(b, r).bugs.hp_drain as usize;
    let period = *b.content.rules.hp_bug_periods.get(level).expect("HP bug level");
    let a = ai_mut(b, r);
    if period != 0 {
        a.hp_drain_counter = a.hp_drain_counter.wrapping_add(1);
        if a.hp_drain_counter < period {
            return;
        }
        crate::kinds::subtract_hp(b, r, 1);
    }
    ai_mut(b, r).hp_drain_counter = 0;
}

/// `sub_802CFF8`: a cursor hit cancels the side's defensive chip.
fn drop_cursor_trap(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    if coll(b, r).acc.damage_elements & 0x40 != 0 && b.linked[side as usize].chip.is_some() {
        b.clear_linked(side);
        b.play_sound(crate::sound::SoundId(0x8E));
    }
}

/// `sub_802CEF4`: anti-damage traps (and the heat trap) swallow the hit
/// and raise a trap request (§4.3).
fn anti_damage_traps(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    if ai(b, r).status & crate::actor::status::HEAT_TRAP != 0 && coll(b, r).acc.element_damage[1] == 0 {
        let d = &coll(b, r).acc.element_damage;
        if d[0] | d[2] | d[3] | d[4] != 0 {
            ai_mut(b, r).attack.marker = 1;
            b.play_sound(crate::sound::SoundId(0x6E));
        }
        zero_trapped_hit(b, r);
        return;
    }
    // The trap the side's defensive-chip record holds.
    use crate::content::Trap;
    let chip = b.linked_trap(side as u8);
    let (trap, min) = if let Some(trap @ (Trap::AntiDamage | Trap::BodyGuard)) = chip {
        (trap, 10)
    } else if ai(b, r).status & crate::actor::status::TRAP_ARMED != 0 {
        (Trap::AntiDamage, 1)
    } else {
        if chip == Some(Trap::AntiSword) {
            use crate::actor::request::ANTI_SWORD_TRIGGERED;
            if ai(b, r).requests & ANTI_SWORD_TRIGGERED != 0 {
                zero_trapped_hit(b, r);
                return;
            }
            let ffc = coll(b, r).acc.hit_flags;
            if ffc & 0x2000 != 0 && ffc & 0x2_0000 == 0 {
                ai_mut(b, r).requests |= ANTI_SWORD_TRIGGERED;
                zero_trapped_hit(b, r);
            }
        }
        return;
    };
    if ai(b, r).requests & (crate::actor::request::ANTI_DAMAGE_TRIGGERED | crate::actor::request::BODY_GUARD_TRIGGERED) != 0 {
        zero_trapped_hit(b, r);
        return;
    }
    if coll(b, r).acc.hit_flags & 0xFF80_0000 == 0 {
        return;
    }
    let sum: u32 = coll(b, r).acc.element_damage[..5].iter().map(|&d| d as u32).sum();
    if sum < min {
        if min == 1 {
            zero_trapped_hit(b, r);
        }
        return;
    }
    ai_mut(b, r).requests |=
        if trap == Trap::AntiDamage { crate::actor::request::ANTI_DAMAGE_TRIGGERED } else { crate::actor::request::BODY_GUARD_TRIGGERED };
    zero_trapped_hit(b, r);
}

/// The trap's ZERO path: the hit and any running paralysis, freeze,
/// bubble and pending requests are dropped.
fn zero_trapped_hit(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    absorb_hit(c);
    c.acc.exclamation = 0;
    c.acc.damage_multiplier = 0;
    c.acc.damage_elements = 0;
    c.acc.raw_elements = 0;
    c.status_timers[timer::PARALYZE] = 0;
    c.status_timers[timer::FREEZE] = 0;
    c.status_timers[timer::BUBBLE] = 0;
    c.acc.hit_flags &= !0x40;
    c.f2 &= !0x301BE;
}

/// `sub_801A6B4`: bug codes 0xF4 (and 0xF7 when HP has a decimal 4)
/// raise the HP-bug level.
fn bug_hp_level(b: &mut Battle, r: ObjectRef) {
    let code = coll(b, r).acc.inflicted_bugs as u8;
    let hit = match code {
        0xF4 => true,
        0xF7 => {
            let has_four = b.objects.get(r).hp.to_string().contains('4');
            if !has_four {
                coll_mut(b, r).acc.inflicted_bugs &= 0xFF00;
            }
            has_four
        }
        _ => false,
    };
    if hit {
        let bugs = &mut stats_mut(b, r).bugs;
        bugs.hp_drain = (bugs.hp_drain + 1).min(7);
    }
}

/// `sub_801A720`: bug code 0xF6 raises two bug levels, paralyzes, and
/// blinds (except NameIDs 0x173..=0x17E).
fn bug_paralyze_blind(b: &mut Battle, r: ObjectRef) {
    if coll(b, r).acc.inflicted_bugs as u8 != 0xF6 {
        return;
    }
    let bugs = &mut stats_mut(b, r).bugs;
    bugs.hp_drain = (bugs.hp_drain + 2).min(7);
    bugs.custom_drain = (bugs.custom_drain + 2).min(7);
    // sub_801A77A
    set_flag2(b, r, 0x8);
    coll_mut(b, r).status_timers[timer::PARALYZE] = 150;
    if !(0x173..=0x17E).contains(&b.objects.get(r).name_id) {
        set_flag2(b, r, 0x20);
        coll_mut(b, r).status_timers[timer::BLIND] = 1200;
    }
    coll_mut(b, r).acc.inflicted_bugs &= 0xFF00;
}

/// The spark an uninstall shows.
const UNINSTALL_SPARK: u8 = 0xE;

/// `sub_8014080` (bug code 0xFB) and `sub_80140EE` (an uninstall, without
/// `undershirt`): MegaMan's body programs go: SuperArmor, FloatShoe (the
/// body back on the ground), Undershirt, AirShoe and the B+Back special
/// (in base form, the navi's too). Link navis keep theirs.
fn strip_programs(b: &mut Battle, r: ObjectRef, undershirt: bool) {
    if navi_of(b, r) != crate::setup::Navi::MEGAMAN {
        return;
    }
    super::clear_flag1(b, r, f1::SUPERARMOR);
    stats_mut(b, r).super_armor = false;
    super::clear_flag1(b, r, f1::FLOATSHOE);
    let hm = super::body_hit_modifier(b);
    b.reset_collision_types(r, 1, 2, hm);
    stats_mut(b, r).float_shoes = false;
    if undershirt {
        super::clear_flag1(b, r, f1::UNDERSHIRT);
        stats_mut(b, r).undershirt = false;
    }
    super::clear_flag1(b, r, f1::AIRSHOE);
    stats_mut(b, r).air_shoes = false;
    stats_mut(b, r).weapons.back_special = None;
    if form_of(b, r) == crate::setup::Form::NONE {
        ai_mut(b, r).back_special = None;
    }
}

/// `sub_80139F6`: bug codes that edit the NaviCust stats (the code names
/// the stat byte; a few codes are special); the weapon routines are
/// reloaded every tick.
fn bug_navicust(b: &mut Battle, r: ObjectRef) {
    let bugs = coll(b, r).acc.inflicted_bugs;
    let (code, arg) = (bugs as u8, (bugs >> 8) as u8);
    let mut edited = false;
    let content = b.content.clone();
    let s = stats_mut(b, r);
    match code {
        0 => {}
        0x18 => {
            s.bugs.hp_drain = (s.bugs.hp_drain as u32 + arg as u32).min(7) as u8;
            edited = true;
        }
        0x19 => {
            s.bugs.custom_drain = (s.bugs.custom_drain as u32 + arg as u32).min(7) as u8;
            edited = true;
        }
        0x54 => {
            // A byte store of the halfword plus the argument.
            let low = (s.bugs.custom_damage as u32 + arg as u32) as u8;
            s.bugs.custom_damage = (s.bugs.custom_damage & 0xFF00) | low as u16;
            edited = true;
        }
        0xFF => {
            s.bugs.buster_blanks = 4;
            edited = true;
        }
        0xFE => (s.bugs.panel_trail_kind, s.bugs.panel_trail_level) = (4, 4),
        0xFA => (s.bugs.panel_trail_kind, s.bugs.panel_trail_level) = (4, 2),
        0xF9 => (s.bugs.panel_trail_kind, s.bugs.panel_trail_level) = (4, 1),
        0xF5 => (s.bugs.panel_trail_kind, s.bugs.panel_trail_level) = (3, 1),
        // sub_8014080: MegaMan loses his body programs.
        0xFB => strip_programs(b, r, true),
        0xF8 => {
            // sub_80140EE: the same but Undershirt, then the form's flags
            // come back (`sub_801469C`); a spark of effect 0xE 16 pixels up
            // (sub_80E08C4) and sound 0x8E.
            if navi_of(b, r) == crate::setup::Navi::MEGAMAN {
                strip_programs(b, r, false);
                super::form::refresh_form_flags(b, r);
            }
            let pos = b.objects.get(r).pos;
            let at = crate::object::Vec3 { z: pos.z.wrapping_add(0x10_0000), ..pos };
            crate::kinds::spark::spawn(b, r, at, UNINSTALL_SPARK);
            b.play_sound(crate::sound::SoundId(0x8E));
        }
        0x64.. => {}
        _ => {
            s.set_byte_by_bug_code(code, arg, &content);
            edited = true;
        }
    }
    if edited {
        refresh_navicust_state(b, r);
        super::form::refresh_form_flags(b, r);
    }
    reload_base_weapons(b, r);
}

// ---- Hit results -------------------------------------------------------------------

/// `sub_801AEB0`: hit modifier bits to requests: 1 flinch (not with
/// SuperArmor or anger), 2 flash, 0x04-0x20 push, with 0x40 drag.
fn hit_modifier_requests(b: &mut Battle, r: ObjectRef) {
    let hm = coll(b, r).hit_mod_final;
    if flag1(b, r) & (f1::SUPERARMOR | f1::ANGER) == 0 && hm & 1 != 0 {
        set_flag2(b, r, 0x4);
    }
    if hm & 2 != 0 {
        set_flag2(b, r, 0x2);
    }
    if hm & 0x3C == 0 {
        return;
    }
    if hm & 0x40 != 0 {
        set_flag2(b, r, 0x100);
        clear_flag2(b, r, 0x4);
        b.objects.get_mut(r).slide_type = 1;
    } else if flag1(b, r) & (f1::DRAG | f1::MOVING) == 0 {
        set_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_type = 1;
    }
}

/// `sub_800EB26`: a counter hit paralyzes (150 ticks) instead of flinching
/// or flashing, unless a bubble status landed.
fn counter_paralysis(b: &mut Battle, r: ObjectRef) {
    let c = coll(b, r);
    if c.acc.hit_flags & 0x40 == 0 || c.status_final.is_some_and(|s| b.content.status(s).survives_counter) {
        return;
    }
    coll_mut(b, r).status_final = Some(b.content.defs.roles.status(StatusRole::CounterParalysis));
    set_flag2(b, r, 0x4000);
    clear_flag2(b, r, 0x6);
}

/// `sub_8013F1E`: the NaviCust on-hit bug (stat 0x16), once per hit
/// sequence while damage lands.
fn navicust_hit_bug(b: &mut Battle, r: ObjectRef) {
    if flag2(b, r) & 0x104 == 0 {
        return;
    }
    let c = coll(b, r);
    if c.acc.hit_flags == 0 {
        return;
    }
    // Three unaligned u32 loads from +0x82: the ARM rotation pairs each
    // damage slot with its predecessor.
    let d = &c.acc.element_damage;
    let word = |lo: u16, hi: u16| lo as u32 | (hi as u32) << 16;
    let sum = word(d[0], c.acc.final_damage).wrapping_add(word(d[2], d[1])).wrapping_add(word(d[4], d[3]));
    if sum == 0 {
        return;
    }
    let latched = b.objects.get(r).prevent_anim != 0;
    let a = ai_mut(b, r);
    if !latched {
        a.hit_bug_latched = false;
    } else if a.hit_bug_latched {
        return;
    } else {
        a.hit_bug_latched = true;
    }
    match stats(b, r).bugs.hit_status {
        0 => {}
        1 => coll_mut(b, r).status_final = Some(b.content.defs.roles.status(StatusRole::HitBugBlind)),
        2 => coll_mut(b, r).status_final = Some(b.content.defs.roles.status(StatusRole::HitBugConfuse)),
        3 => {
            let bugs = &mut stats_mut(b, r).bugs;
            if bugs.hp_drain < 7 {
                bugs.hp_drain += 1;
            }
        }
        n => panic!("NaviCust hit bug {n} reads past its table"),
    }
}

/// `sub_801A554`: apply the status the hit carried: set its timer and
/// raise its request (§4.8).
fn apply_status(b: &mut Battle, r: ObjectRef) {
    let Some(s) = coll(b, r).status_final else {
        return;
    };
    let e = b.content.status(s);
    let c = coll_mut(b, r);
    let t = match e.timer {
        StatusTimer::Paralyze => timer::PARALYZE,
        StatusTimer::Confuse => timer::CONFUSE,
        StatusTimer::Blind => timer::BLIND,
        StatusTimer::Immobilize => timer::IMMOBILIZE,
        StatusTimer::Flash => timer::FLASH,
        StatusTimer::Submerged => timer::SUBMERGED,
        StatusTimer::Invulnerable => timer::INVULNERABLE,
        StatusTimer::Freeze => timer::FREEZE,
        StatusTimer::Bubble => timer::BUBBLE,
        StatusTimer::CollisionPanel => {
            // Garbage entries (the original's statuses past a group's end)
            // write the collision panel.
            c.panel.x = e.duration as u8;
            c.panel.y = (e.duration >> 8) as u8;
            return set_flag2(b, r, e.requests);
        }
        StatusTimer::Other(off) => panic!("status {} writes CollisionData+{off:#x}", b.content.defs.statuses[s.index()].key),
    };
    c.status_timers[t] = e.duration;
    set_flag2(b, r, e.requests);
    if e.cancels_flinch {
        clear_flag2(b, r, 0x6);
    }
}

/// `sub_801A2CC`: a chip-erasing hit (self bit 0x10) loses the current
/// chip.
fn lose_chip(b: &mut Battle, r: ObjectRef) {
    if coll(b, r).acc.hit_flags & 0x10 == 0 {
        return;
    }
    b.objects.get_mut(r).chip = None;
    if ai(b, r).actor_type != ActorType::Player {
        b.objects.get_mut(r).chips_held = 0;
        return;
    }
    let hand = &mut b.hands[b.objects.get(r).alliance as usize];
    if hand.ids.get(hand.cursor as usize).is_some_and(|id| id.is_some()) {
        hand.cursor += 1;
    }
}

/// `sub_801A324`: drain hits credit the attacker, who heals MaxHP/10 per
/// credit on its own next stage A.
fn drain_heal(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    let opponent = b.player(side ^ 1).expect("the opponent's player");
    let hits = coll(b, r).acc.drain_hits;
    let o = ai_mut(b, opponent);
    o.drain_heal_credits = o.drain_heal_credits.wrapping_add(hits as u8);
    let a = ai_mut(b, r);
    let credits = a.drain_heal_credits as u32;
    a.drain_heal_credits = 0;
    let heal = (b.objects.get(r).max_hp / 10) as u32 * credits;
    if heal == 0 {
        return;
    }
    add_hp(b, r, heal);
    let pos = b.objects.get(r).pos;
    crate::kinds::effect::spawn(b, pos, 6, 0, 0, 0);
    b.play_sound(crate::sound::SoundId(0x8A));
}

/// `object_calculateFinalDamage1`: sum the element damage (halved,
/// rounding up, per element on a holy panel under the object), through
/// the side's damage-carry record.
fn final_damage(b: &mut Battle, r: ObjectRef) {
    let holy = panel_kind(b, b.objects.get(r).panel) == PanelType::Holy;
    let k = holy as u32;
    let c = coll_mut(b, r);
    let mut sum = 0u32;
    for d in &mut c.acc.element_damage[..5] {
        let v = (*d as u32 + (1 << k) - 1) >> k;
        *d = v as u16;
        sum += v;
    }
    let sum = damage_carry(b, r, sum);
    coll_mut(b, r).acc.final_damage = sum as u16;
}

/// `sub_802CE10`: the carry record adds last tick's damage to its target,
/// and otherwise records the side's largest damage this tick.
fn damage_carry(b: &mut Battle, r: ObjectRef, sum: u32) -> u32 {
    let rec = &mut b.damage_carry[b.objects.get(r).alliance as usize];
    if rec.target == Some(r) {
        return sum + rec.previous as u32;
    }
    if sum > rec.this_tick as u32 {
        rec.this_tick = sum as u16;
    }
    sum
}

/// `sub_801A420`: the counter window closes.
fn tick_counter_window(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    c.counter_timer = c.counter_timer.saturating_sub(1);
}

/// `sub_80143FC`: count the ticks spent flinching or paralyzed.
fn count_stun_ticks(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() || b.paused {
        return;
    }
    let stunned = flag1(b, r) & (f1::FLINCHING | f1::PARALYZED) != 0;
    let a = ai_mut(b, r);
    a.stun_ticks = if stunned { a.stun_ticks.wrapping_add(1) } else { 0 };
}

/// `sub_80142DC`: anger after 120 stunned ticks or a 300+ damage hit
/// (base MegaMan only).
fn anger_trigger(b: &mut Battle, r: ObjectRef) {
    if battle_mode(b) == 1 || navi_of(b, r) != Navi::MEGAMAN || form_of(b, r) != Form::NONE || flag1(b, r) & f1::ANGER != 0 {
        return;
    }
    if ai(b, r).stun_ticks as i32 >= 0x78 || coll(b, r).acc.final_damage >> 1 >= 0x96 {
        set_flag2(b, r, 0x200);
    }
}

/// `sub_8010198`: any hit ends the timed submerged state.
fn hit_ends_submerged(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    if c.status_timers[timer::SUBMERGED] != 0 && c.acc.hit_flags != 0 {
        c.status_timers[timer::SUBMERGED] = 0;
    }
}

/// `sub_801A648`: a piercing hit (self bit 4, without 0x1000) ends the
/// flash.
fn pierce_ends_flash(b: &mut Battle, r: ObjectRef) {
    if b.paused {
        return;
    }
    let c = coll_mut(b, r);
    let ffc = c.acc.hit_flags;
    if c.status_timers[timer::FLASH] != 0 && ffc & 4 != 0 && ffc & 0x1000 == 0 {
        c.status_timers[timer::FLASH] = 0;
    }
}

/// `object_spawnHiteffect`: a blocked hit shows a guard spark (one RNG
/// draw).
fn guard_spark(b: &mut Battle, r: ObjectRef) {
    if b.paused || coll(b, r).acc.hit_flags & 0x2_0000 == 0 {
        return;
    }
    b.play_sound(crate::sound::SoundId(0x6E));
    let p = b.objects.get(r).pos;
    let pos = crate::kinds::spark::jitter(b, 0xF, Vec3 { z: p.z.wrapping_add(0x10_0000), ..p });
    crate::kinds::spark::spawn(b, r, pos, 8);
}
