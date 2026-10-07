//! Stage A of the damage pipeline (`sub_801AC6C`): unregister the body
//! (resolving hits against whatever is on its panels), then turn this
//! tick's hit results into requests, statuses and the final damage. See
//! objects-and-player.md §H2 and field-collision-damage.md §4.1-§4.11.

use super::{
    ActorType, ai, ai_mut, battle_mode, clear_flag2, coll, coll_id, coll_mut, flag1, flag2, panel_kind,
    form_of, navi_of, refresh_abilities, reload_base_weapons, set_flag2, stats, stats_mut,
};
use crate::battle::{Battle, battle_flags};
use crate::collision::{CollisionData, f1, timer};
use crate::content::{EffectRole, SparkRole, StatusRole, StatusTimer};
use crate::object::{ObjectRef, Vec3};

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
    // (EXE5's lava burns first: 0x080178EC; EXE4's a player's, 0x08013128.)
    crate::kinds::common::panel_burn(b, r, true);
    barrier(b, r);
    standing_effects(b, r);
    // The side's rules, each tick (EXE5's light and dark: a dark MegaMan
    // clears the holy panel he stands on, 0x08017136; EXE6's call nothing).
    let side = b.objects.get(r).alliance;
    b.rules_navi_intake(side, r);
    slide_triggers(b, r);
    // (EXE5's takes the hit's NaviCust bug before the HP bug drains: the
    // navi's game's intake rules.)
    let bugs_first = b.game_rules().intake.bugs_before_drain;
    if bugs_first {
        bug_navicust(b, r);
    }
    hp_bug_drain(b, r);
    drop_cursor_trap(b, r);
    anti_damage_traps(b, r);
    hit_bug(b, r);
    if !bugs_first {
        bug_navicust(b, r);
    }
    hit_modifier_requests(b, r);
    counter_paralysis(b, r);
    navicust_hit_bug(b, r);
    let s = stats(b, r);
    // (TomahawkMan and TomahawkCross: the navi's and the form's
    // `status_immune`.)
    let immune = navi_of(b, r).traits.has(crate::content::NaviTraits::STATUS_IMMUNE)
        || form_of(b, r).traits.has(crate::content::FormTraits::STATUS_IMMUNE);
    if !(immune || s.bugs.status_immunity) {
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
    if b.game_rules().intake.no_charge_drive {
        no_charge_timer(b, r);
    }
}

/// EXE5's 0x0800DBE0 (its intake's last step): with the no-charge state,
/// unless the battle is dimmed or paused, the drive's ticks run down (from
/// any count but 0 and 0xFFFF); their end asks for the stun strike (EXE5's
/// action 0x49: the drive's end).
fn no_charge_timer(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).status & crate::actor::status::NO_CHARGE == 0 || b.is_dimmed() || b.paused {
        return;
    }
    let a = ai_mut(b, r);
    let t = a.no_charge_timer;
    if t == 0xFFFF || t == 0 {
        return;
    }
    a.no_charge_timer = t - 1;
    if t - 1 == 0 {
        a.requests |= crate::actor::request::STUN_STRIKE;
    }
}

/// The navi type's intake (EXE5's 0x08017688, EXE6's `sub_801A9B8`): a navi
/// no player controls takes its hits as a player does, without what only a
/// player has (the NaviCust's HP drain, hit bug and stat edits, the stunned
/// ticks and anger, the opponent's support Tango), with whatever status a
/// hit carries, and its drain hits credited to the other side's player
/// without its own healing (`sub_801A308`).
pub(super) fn collect_hits_navi(b: &mut Battle, r: ObjectRef) {
    b.objects.sprite_mut(r).look.white = false;
    if b.round.flags & battle_flags::FIGHTING == 0 {
        return;
    }
    b.remove_collision(coll_id(b, r));
    if b.is_battle_over() || flag1(b, r) & f1::DEAD != 0 {
        return;
    }
    crate::kinds::common::panel_burn(b, r, false);
    barrier(b, r);
    standing_effects(b, r);
    let side = b.objects.get(r).alliance;
    b.rules_navi_intake(side, r);
    slide_triggers(b, r);
    drop_cursor_trap(b, r);
    anti_damage_traps(b, r);
    hit_bug(b, r);
    hit_modifier_requests(b, r);
    counter_paralysis(b, r);
    apply_status(b, r);
    lose_chip(b, r);
    drain_credit(b, r);
    final_damage(b, r);
    tick_counter_window(b, r);
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
    // (EXE5's by-side modifiers go with it: an absorbed hit pushes nothing.)
    c.hit_mod_by_side = [0; 2];
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
    let rule = b.game_rules().intake.barrier;
    if b.paused && rule.stops_while_paused {
        return;
    }
    let dimmed = b.is_dimmed();
    let action = super::navi_action(b, r);
    let holy = {
        let c = coll(b, r);
        c.barrier != 0 && b.game_rules().panels.is_named(panel_kind(b, c.panel), "holy")
    };
    let c = coll_mut(b, r);
    let mut barrier = c.barrier;
    if barrier == 0 {
        return;
    }
    let regrowing = matches!(barrier, 8 | 0xA) && barrier_hp16(c) == 0;
    match rule.wind {
        crate::content::BarrierWind::Pops if !regrowing && (c.acc.raw_elements & 0x20 != 0 || c.acc.raw_hit_flags & 0xA20 != 0) => {
            // Popped (wind): absorbs everything until its visual clears it.
            barrier = 0x10;
            c.barrier = 0x10;
            set_barrier_hp16(c, 0);
            c.barrier_saved_hmf = c.hit_mod_final;
        }
        crate::content::BarrierWind::TakesAway if !regrowing && c.acc.raw_elements & 0x20 != 0 => {
            // Gone at once (EXE4's 0x08012E1E); the rest of the tick goes on
            // by the type it had.
            c.barrier = 0;
            set_barrier_hp16(c, 0);
        }
        _ => {}
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
/// element 5), and a panel that drains a body's element (EXE5's sea, fire
/// bodies: 0x08016C7E) the same; wood navis on grass heal. (EXE4's,
/// 0x08012FF2, has no pause test: its player stops for pauses once in
/// control, the rules' `paused_navi`, so it never runs paused. It heals on
/// the 20-tick count at any HP: the panel rules' `grass_heal_slows_at`.)
fn standing_effects(b: &mut Battle, r: ObjectRef) {
    if b.is_dimmed() || b.paused || coll(b, r).region.is_none() {
        return;
    }
    let p = coll(b, r).panel;
    let Some(t) = b.field.panel(p.x, p.y).map(|p| p.kind) else { return };
    let f = flag1(b, r);
    let drains = b.game_rules().panels.rule(t).drains;
    let on_grass;
    let panels = &b.game_rules().panels;
    let grass = panels.named("grass");
    if panels.is_named(t, "poison") || drains.is_some_and(|e| e == coll(b, r).element) {
        if f & (f1::UNTOUCHABLE | f1::FLOATSHOE | f1::INVULNERABLE) == 0 {
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
        on_grass = grass.is_some_and(|g| f == g.0 as u32);
    } else {
        on_grass = grass == Some(t);
    }
    coll_mut(b, r).poison_timer = 0;
    if !on_grass || b.objects.get(r).element & 0xF != 4 {
        return;
    }
    let slow = b.game_rules().panels.grass_heal_slows_at.is_some_and(|at| b.objects.get(r).hp <= at);
    let cycle = if slow { b.round.cycle180 } else { b.round.cycle20 };
    if cycle == 0 {
        add_hp(b, r, 1);
    }
}

/// `sub_801A36A`: start road slides, and what ice does at the end of a
/// move (consuming MOVE_COMPLETE): a slide, or EXE4's push (the rule `ice`).
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
        if b.game_rules().panels.is_road(panel_kind(b, coll(b, r).panel)) {
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
    let Some(kind) = b.field.panel(p.x, p.y).map(|p| p.kind) else { return };
    // EXE5's panels at a move's end (0x0801715E, after its own flag test):
    // metal slides the body, sea holds it.
    let rule = *b.game_rules().panels.rule(kind);
    if (rule.slide.is_some() || rule.holds.is_some()) && flag1(b, r) & 0x0010_0040 == 0 {
        if rule.slide.is_some() {
            return metal_slide(b, r);
        }
        if let Some(ticks) = rule.holds {
            return panel_hold(b, r, ticks);
        }
    }
    if !b.game_rules().panels.is_named(kind, "ice") {
        return;
    }
    // sub_801A3DA (EXE4's 0x0801335A)
    let f = flag1(b, r);
    if coll(b, r).element != 2 && f & 0x24 == 0 && f & f1::AFFECTED_BY_ICE != 0 {
        match b.game_rules().ice {
            crate::content::IceRule::Slide(_) => {
                set_flag2(b, r, 0x10);
                b.objects.get_mut(r).slide_type = 2;
            }
            // EXE4's: a push bit by side and direction into the final
            // modifier, which the hit modifiers' requests read below.
            crate::content::IceRule::Push(bits) => {
                let side = b.objects.get(r).alliance as usize & 1;
                let c = coll_mut(b, r);
                c.hit_mod_final |= bits[side].get(c.direction as usize).copied().unwrap_or(0);
            }
        }
    }
}

/// EXE5's 0x08017216: a move's end on metal slides the body (slide type
/// 3), unless it slid within the cooldown, is floating or slide-proof
/// (flags 0x24), or is a navi whose form stands on metal (EXE5's MagnetSoul).
fn metal_slide(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).road_cooldown != 0 || flag1(b, r) & 0x24 != 0 {
        return;
    }
    if form_of(b, r).traits.has(crate::content::FormTraits::STANDS_ON_METAL) {
        return;
    }
    set_flag2(b, r, 0x10);
    b.objects.get_mut(r).slide_type = 3;
}

/// EXE5's 0x080171C2: a move's end on a panel that holds (sea) holds the
/// body there for `ticks` (immobilized: EXE6's `sub_800EB18`) with a splash
/// (the arena's effect `panel_splash`), unless it floats, dives or is of
/// aqua.
fn panel_hold(b: &mut Battle, r: ObjectRef, ticks: u16) {
    let dives = b.objects.get(r).actor.is_some_and(|a| b.actors.get(a).status & crate::actor::status::DIVES != 0);
    if flag1(b, r) & f1::FLOATSHOE != 0 || dives || b.objects.get(r).element == 2 {
        return;
    }
    coll_mut(b, r).status_timers[crate::collision::timer::IMMOBILIZE] = ticks;
    super::set_flag1(b, r, f1::IMMOBILIZED);
    let pos = b.objects.get(r).pos;
    let look = b.roles().effect(crate::content::EffectRole::PanelSplash);
    crate::kinds::effect::spawn(b, pos, look, 0, 0, 0);
}

// ---- NaviCust bugs and traps ------------------------------------------------------

/// `sub_8010230` (EXE4's 0x0800C164): the NaviCust HP bug drains 1 HP
/// every so many ticks (never below 1), by the rule `hp_drain`: its period
/// by the bug's level or the stat itself, and whether a pause holds it.
fn hp_bug_drain(b: &mut Battle, r: ObjectRef) {
    let rule = b.game_rules().hp_drain;
    if b.is_dimmed() || (rule.stops_while_paused && b.paused) || b.objects.get(r).hp <= 1 {
        return;
    }
    let period = rule.periods.period(stats(b, r).bugs.hp_drain);
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
        b.sound(crate::content::SoundRole::Fade);
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
            b.sound(crate::content::SoundRole::Guard);
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

/// `sub_801A6B4` and `sub_801A720`: the bugs a hit's code brings any
/// navi, which its side's rules say (`hit_bug`: EXE6's HP bug codes and its
/// paralyzing, blinding one), when a hit brought a code.
fn hit_bug(b: &mut Battle, r: ObjectRef) {
    if coll(b, r).acc.inflicted_bugs & 0xFF == 0 {
        return;
    }
    let side = b.objects.get(r).alliance;
    b.rules_hit_bug(side, r);
}

/// `sub_8014080` and `sub_80140EE` (an uninstall, without `undershirt`):
/// MegaMan's body programs go: SuperArmor, FloatShoe (the body back on the
/// ground), Undershirt, AirShoe and the B+Back special (in base form, the
/// navi's too). Link navis keep theirs. Whether the navi changes form (a
/// game's bug codes 0xFB and 0xF8 call it: their rules).
pub(crate) fn strip_body_programs(b: &mut Battle, r: ObjectRef, undershirt: bool) -> bool {
    if !super::is_megaman(b, r) {
        return false;
    }
    super::clear_flag1(b, r, f1::SUPERARMOR);
    stats_mut(b, r).super_armor = false;
    super::clear_flag1(b, r, f1::FLOATSHOE);
    let hm = super::body_hit_modifier(b);
    super::reset_body_types(b, r, false, hm);
    stats_mut(b, r).float_shoes = false;
    if undershirt {
        super::clear_flag1(b, r, f1::UNDERSHIRT);
        stats_mut(b, r).undershirt = false;
    }
    super::clear_flag1(b, r, f1::AIRSHOE);
    stats_mut(b, r).air_shoes = false;
    stats_mut(b, r).weapons.back_special = None;
    if super::in_base_form(b, r) {
        ai_mut(b, r).back_special = None;
    }
    true
}

/// `sub_80139F6` (EXE5's 0x0801103E): the navi takes its hit's NaviCust
/// bug (the code its collision's `inflicted_bugs` holds), which its side's
/// rules say what it does to (`navi_bug`: a game's table of its codes, a
/// stat each code below 0x64 names, by name); then, unless they spare it,
/// its weapon routines are reloaded, and where they edited its stats its
/// abilities and form flags come back first. The rules are asked on a tick
/// something hit it (a code, or the hit flags their answer may read: EXE5's
/// light MegaMan's).
fn bug_navicust(b: &mut Battle, r: ObjectRef) {
    let c = coll(b, r);
    if c.acc.inflicted_bugs != 0 || c.acc.hit_flags != 0 {
        let side = b.objects.get(r).alliance;
        match b.rules_navi_bug(side, r) {
            crate::rules::NaviBug::Spared => return,
            crate::rules::NaviBug::Edited => {
                refresh_abilities(b, r);
                super::form::refresh_form_flags(b, r);
            }
            crate::rules::NaviBug::Untouched => {}
        }
    }
    reload_base_weapons(b, r);
}

/// For tests and tools: side `side`'s navi takes bug `bugs` (the code, and its
/// argument in the high byte) as a hit's ([`bug_navicust`]).
pub fn take_navi_bug(b: &mut Battle, side: u8, bugs: u16) {
    let r = b.player(side).expect("the side's navi");
    coll_mut(b, r).acc.inflicted_bugs = bugs;
    bug_navicust(b, r);
}

// ---- Hit results -------------------------------------------------------------------

/// `sub_801AEB0` (EXE4's in 0x08013858): hit modifier bits to requests: 1
/// flinch (not with SuperArmor or anger), 2 flash, a push (the rules'
/// push bits: EXE6's 0x04-0x20, EXE4's 0x04-0x80) a slide, or with the
/// rules' drag bit (EXE6's 0x40, EXE4's the flinch bit) a drag. (EXE4's
/// sets no slide type: its slides read the push whatever the type, and
/// none but the push's is set in it.)
fn hit_modifier_requests(b: &mut Battle, r: ObjectRef) {
    let hm = coll(b, r).hit_mod_final;
    if flag1(b, r) & (f1::SUPERARMOR | f1::ANGER) == 0 && hm & 1 != 0 {
        set_flag2(b, r, 0x4);
    }
    if hm & 2 != 0 {
        set_flag2(b, r, 0x2);
    }
    let reading = &b.game_rules().push_reading;
    if hm & reading.mask() == 0 {
        return;
    }
    if hm & reading.drag_bit != 0 {
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
    coll_mut(b, r).status_final = Some(b.roles().status(StatusRole::CounterParalysis));
    set_flag2(b, r, 0x4000);
    clear_flag2(b, r, 0x6);
}

/// `sub_8013F1E` (EXE5's 0x0801156E): a hit that asks the navi to flinch
/// or be dragged (the requests 0x104) is the side's rules' to hear of
/// (`navi_flinched`: the NaviCust's hit bug, the `hit_status` stat, behind
/// the game's gate, which may put a status in place of the hit's).
fn navicust_hit_bug(b: &mut Battle, r: ObjectRef) {
    if flag2(b, r) & 0x104 == 0 {
        return;
    }
    let side = b.objects.get(r).alliance;
    b.rules_navi_flinched(side, r);
}

/// `sub_801A554`: apply the status the hit carried: set its timer and
/// raise its request (§4.8).
fn apply_status(b: &mut Battle, r: ObjectRef) {
    if let Some(s) = coll(b, r).status_final {
        take_status(b, r, s);
    }
}

/// [`apply_status`]'s work for status `s`: its timer set and its requests
/// raised (the rules' `take_status` too: a bug's paralysis and blindness,
/// `sub_801A77A`).
pub(crate) fn take_status(b: &mut Battle, r: ObjectRef, s: nettai_content_api::StatusHandle) {
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

/// `sub_801A2CC`: chip destruction, a hit (self bit 0x10) that destroys the
/// current chip.
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
    drain_credit(b, r);
    let a = ai_mut(b, r);
    let credits = a.drain_heal_credits as u32;
    a.drain_heal_credits = 0;
    let heal = (b.objects.get(r).max_hp / 10) as u32 * credits;
    if heal == 0 {
        return;
    }
    add_hp(b, r, heal);
    let pos = b.objects.get(r).pos;
    let look = b.roles().effect(EffectRole::Recovery);
    crate::kinds::effect::spawn(b, pos, look, 0, 0, 0);
    b.sound(crate::content::SoundRole::Recovery);
}

/// `sub_801A308`: drain hits credit the other side's player (`sub_801A324`'s
/// first part).
fn drain_credit(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance;
    let opponent = b.player(side ^ 1).expect("the opponent's player");
    let hits = coll(b, r).acc.drain_hits;
    let o = ai_mut(b, opponent);
    o.drain_heal_credits = o.drain_heal_credits.wrapping_add(hits as u8);
}

/// `object_calculateFinalDamage1`: sum the element damage (halved,
/// rounding up, per element on a holy panel under the object), through
/// the side's damage-carry record.
fn final_damage(b: &mut Battle, r: ObjectRef) {
    let holy = b.game_rules().panels.is_named(panel_kind(b, b.objects.get(r).panel), "holy");
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

/// `sub_80142DC`: anger after 120 stunned ticks or, where the rules say
/// (`emotion.anger_damage`: EXE6's and EXE5's 300; EXE4's 0x0800C540 none),
/// a tick's hits of that much damage (base MegaMan only).
fn anger_trigger(b: &mut Battle, r: ObjectRef) {
    if battle_mode(b) == 1 || !super::is_megaman(b, r) || !super::in_base_form(b, r) || flag1(b, r) & f1::ANGER != 0 {
        return;
    }
    let damage = b.game_rules().emotion.anger_damage;
    let hurt = damage.is_some_and(|d| coll(b, r).acc.final_damage >> 1 >= d >> 1);
    if ai(b, r).stun_ticks as i32 >= 0x78 || hurt {
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
    b.sound(crate::content::SoundRole::Guard);
    let p = b.objects.get(r).pos;
    let pos = crate::kinds::spark::jitter(b, 0xF, Vec3 { z: p.z.wrapping_add(0x10_0000), ..p });
    let spark = b.roles().spark(SparkRole::Guard);
    crate::kinds::spark::spawn(b, r, pos, spark);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Content, IceRule, testing};
    use std::sync::Arc;

    /// A fight on the test content whose ice does `ice`: the battle and side
    /// 1's navi (at (5, 2)), standing on ice at a move's end, the move's
    /// direction `direction`.
    fn on_ice(ice: IceRule, direction: u8) -> (Battle, ObjectRef) {
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        c.rules_mut().ice = ice;
        let c = Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        crate::content::testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        b.spawn_actors();
        b.run_objects();
        b.round.flags |= battle_flags::FIGHTING;
        let r = b.player(1).unwrap();
        b.set_panel_type(5, 2, crate::content::testing::panel("ice"));
        super::super::set_flag1(&mut b, r, f1::AFFECTED_BY_ICE | f1::MOVE_COMPLETE);
        let c = coll_mut(&mut b, r);
        c.direction = direction;
        c.hit_mod_final = 0;
        (b, r)
    }

    /// docs/design/exe4-map.md §18 item 1: EXE4's ice is a push (0x0801335A):
    /// a move's end on it ORs the push bit of the side and the direction into
    /// the final modifier, and starts no ice slide; EXE6's slides.
    #[test]
    fn ice_pushes_or_slides_by_the_rule() {
        let push = IceRule::Push([[0, 0x40, 0x80, 0x20, 0x10], [0, 0x40, 0x80, 0x10, 0x20]]);
        let (mut b, r) = on_ice(push, 3);
        slide_triggers(&mut b, r);
        assert_eq!(coll(&b, r).hit_mod_final, 0x10, "side 1's move left: its push forward");
        assert_eq!((flag2(&b, r) & 0x10, b.objects.get(r).slide_type), (0, 0), "no ice slide");
        assert_eq!(flag1(&b, r) & f1::MOVE_COMPLETE, 0, "the move's end is taken");
        // An aqua body stays.
        let (mut b, r) = on_ice(push, 1);
        coll_mut(&mut b, r).element = 2;
        slide_triggers(&mut b, r);
        assert_eq!(coll(&b, r).hit_mod_final, 0);
        // The test content's ice (EXE6's) slides.
        let (mut b, r) = on_ice(testing::rules().ice, 3);
        slide_triggers(&mut b, r);
        assert_eq!((coll(&b, r).hit_mod_final, flag2(&b, r) & 0x10, b.objects.get(r).slide_type), (0, 0x10, 2));
    }

    /// docs/design/exe4-map.md §18 item 7: the HP bug drains by the rule:
    /// EXE6's by level and held while paused, EXE4's every stat ticks
    /// through a pause.
    #[test]
    fn the_hp_bug_drains_by_the_rule() {
        use crate::content::{DrainPeriods, HpDrainRule, StatIsPeriod};
        let lost = |rule: Option<HpDrainRule>, paused: bool| {
            let (mut b, r) = on_ice(testing::rules().ice, 0);
            if let Some(rule) = rule {
                let mut c = (*b.content).clone();
                c.rules_mut().hp_drain = rule;
                b.content = Arc::new(c);
            }
            b.paused = paused;
            let side = b.objects.get(r).alliance as usize;
            b.stats[side].bugs.hp_drain = 3;
            b.objects.get_mut(r).hp = 100;
            let hp = b.objects.get(r).hp;
            for _ in 0..60 {
                hp_bug_drain(&mut b, r);
            }
            hp - b.objects.get(r).hp
        };
        let exe4 = HpDrainRule { periods: DrainPeriods::Stat(StatIsPeriod::Stat), stops_while_paused: false };
        assert_eq!(lost(Some(exe4), true), 20, "every third tick, paused or not");
        assert_eq!(lost(None, false), 60 / 40, "the test content's level 3: 40 ticks");
        assert_eq!(lost(None, true), 0, "held while paused");
    }

    /// docs/design/exe4-map.md §18 items 2 and 3: a push is a drag with the
    /// rules' drag bit: EXE6's 0x40 (its push bits 0x04 to 0x20), EXE4's the
    /// flinch bit (its push bits 0x04 to 0x80, 0x40 a push up). The requests
    /// of modifier `hm`: (flinch, slide, drag), and the slide type.
    #[test]
    fn a_push_drags_by_the_rules_bit() {
        let requests = |exe4: bool, hm: u8| {
            let (mut b, r) = on_ice(testing::rules().ice, 0);
            if exe4 {
                let mut c: Content = testing::build();
                c.define().unwrap_or_else(|e| panic!("{e}"));
                let rules = c.rules_mut();
                rules.push_reading.bits = 6;
                rules.push_reading.drag_bit = 0x01;
                rules.push_reading.shift = None;
                b.content = Arc::new(c);
            }
            coll_mut(&mut b, r).hit_mod_final = hm;
            hit_modifier_requests(&mut b, r);
            let f = flag2(&b, r);
            ((f & 0x4 != 0, f & 0x10 != 0, f & 0x100 != 0), b.objects.get(r).slide_type)
        };
        assert_eq!(requests(false, 0x44), ((false, false, true), 1));
        assert_eq!(requests(false, 0x05), ((true, true, false), 1));
        assert_eq!(requests(false, 0x41), ((true, false, false), 0), "0x40 alone is no push");
        assert_eq!(requests(true, 0x41), ((false, false, true), 1), "EXE4's push up with the flinch bit drags");
        assert_eq!(requests(true, 0x40), ((false, true, false), 1), "EXE4's push up alone slides");
        assert_eq!(requests(true, 0x01), ((true, false, false), 0));
    }

    /// A fight on the test content, its rules changed by `change`: the
    /// battle and side 1's navi, fighting, its body's region anchored.
    fn fight_with(change: impl FnOnce(&mut crate::content::Rules)) -> (Battle, ObjectRef) {
        let mut c: Content = testing::build();
        c.define().unwrap_or_else(|e| panic!("{e}"));
        change(c.rules_mut());
        let c = Arc::new(c);
        let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(&c));
        crate::content::testing::on(&mut setup, &c);
        let mut b = Battle::new(setup, c);
        b.spawn_actors();
        b.run_objects();
        b.round.flags |= battle_flags::FIGHTING;
        let r = b.player(1).unwrap();
        (b, r)
    }

    /// docs/design/exe4-map.md §18 item 13: EXE4's lava (`burn`) burns a
    /// player while the battle is dimmed, wears 20 off its mood, and spares
    /// only the bits it names; another body waits out the dimming. The test
    /// content's (EXE5's) burns no one dimmed and wears no mood.
    #[test]
    fn a_burn_is_the_games() {
        use crate::content::BurnRule;
        let exe4 = BurnRule { damage: 50, spared_by: 0x206, mood: 20, players_while_dimmed: true };
        let run = |burn: Option<BurnRule>, dimmed: bool, player: bool, status: u32| {
            let (mut b, r) = fight_with(|rules| {
                if let Some(burn) = burn {
                    rules.panels.types[crate::content::testing::panel("lava").0 as usize].burn = Some(burn);
                }
            });
            let p = coll(&b, r).panel;
            b.set_panel_type(p.x, p.y, crate::content::testing::panel("lava"));
            if dimmed {
                b.round.flags |= battle_flags::DIMMED;
            }
            super::super::set_flag1(&mut b, r, status);
            crate::kinds::common::panel_burn(&mut b, r, player);
            let c = coll(&b, r);
            (c.acc.element_damage[1], c.acc.mood_damage, b.field.panels[p.y as usize][p.x as usize].kind == crate::content::testing::panel("lava"))
        };
        assert_eq!(run(Some(exe4), true, true, 0), (50, 20, false), "EXE4's player, dimmed");
        assert_eq!(run(Some(exe4), true, false, 0), (0, 0, true), "another body waits");
        assert_eq!(run(Some(exe4), false, true, 0x8000_0000), (50, 20, false), "EXE5's 0x80000000 spares none");
        assert_eq!(run(None, true, true, 0), (0, 0, true), "EXE5's waits");
        assert_eq!(run(None, false, true, 0x8000_0000), (0, 0, true));
        assert_eq!(run(None, false, true, 0), (50, 0, false));
    }

    /// docs/design/exe4-map.md §18 item 13: EXE4's grass heals on the
    /// 20-tick count at any HP (`grass_heal_slows_at` none); EXE6's (the
    /// test content's) on the 180-tick count at 9 HP or less.
    #[test]
    fn grass_heals_by_the_games_count() {
        let healed = |exe4: bool| {
            let (mut b, r) = fight_with(|rules| {
                if exe4 {
                    rules.panels.grass_heal_slows_at = None;
                }
            });
            let p = coll(&b, r).panel;
            b.set_panel_type(p.x, p.y, crate::content::testing::panel("grass"));
            let o = b.objects.get_mut(r);
            o.element = 4;
            (o.hp, o.max_hp) = (5, 100);
            (b.round.cycle20, b.round.cycle180) = (0, 1);
            standing_effects(&mut b, r);
            b.objects.get(r).hp
        };
        assert_eq!((healed(true), healed(false)), (6, 5), "at 5 HP: EXE4's 20-tick count, EXE6's 180");
    }
}
