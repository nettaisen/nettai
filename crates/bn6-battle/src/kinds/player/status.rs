//! Stage B of the damage pipeline (`sub_801AF44`): apply the damage,
//! turn requests into reactions (death, drag, slide, flinch), run the
//! flash and status timers, then dispatch the current action. See
//! objects-and-player.md §H3-§H5 and field-collision-damage.md §4.

use super::{
    actions, ai, ai_mut, attach_point, clear_bubble, clear_flag1, clear_flag2, clear_freeze, clear_paralysis, coll,
    coll_mut, cross_protected, emotion, entry, exit_attack_state, flag1, flag2, idle, is_link, is_mode_40, navi_record,
    reactions, reset_attack_links, save_state_word, set_attack, set_flag1, set_flag2, set_mood,
};
use crate::actor::{ActorType, request, status as ai_status};
use crate::battle::{Battle, battle_flags};
use crate::collision::{f1, link, timer};
use crate::object::{ObjectRef, Pool, Vec3, flags};
use crate::setup::Form;

/// `sub_801AF44`, including the action dispatch (`sub_801B9E6`).
pub(super) fn update(b: &mut Battle, r: ObjectRef) {
    if !b.paused || b.objects.get(r).action == 0 {
        match apply(b, r) {
            Flow::Tail => {}
            Flow::Dispatch => return dispatch(b, r),
            Flow::Return => return,
        }
    }
    tail(b, r);
}

/// Where the top block of `sub_801AF44` continues.
enum Flow {
    /// The visuals and the pause/time-stop/dispatch choice (`loc_801B142`).
    Tail,
    /// Straight to the action dispatch.
    Dispatch,
    /// Nothing more this tick.
    Return,
}

/// The top block: damage, death, special states, drag, slides, flinch,
/// flash and status timers.
fn apply(b: &mut Battle, r: ObjectRef) -> Flow {
    weakness_effect(b, r);
    bug_effect(b, r);
    counter_hit_bookkeeping(b, r);
    weakness_request(b, r);
    apply_damage(b, r);
    // sub_801BADE: a damaging hit pops the bubble.
    if b.objects.get(r).action == 7 && coll(b, r).acc.final_damage != 0 {
        clear_bubble(b, r);
    }
    if flag1(b, r) & f1::DEAD != 0 {
        return Flow::Tail;
    }
    if flag2(b, r) & 1 != 0 {
        clear_flag2(b, r, 1);
        set_flag1(b, r, f1::DEAD);
        set_attack(b, r, 2, 0);
        return Flow::Tail;
    }
    let st = ai(b, r).status;
    if st & (ai_status::CROSS_2000 | ai_status::CROSS_10000 | ai_status::CROSS_20000) != 0 {
        return Flow::Dispatch;
    }
    if st & ai_status::CROSS_40000 != 0 && cross_lane(b, r) {
        return Flow::Return;
    }
    if b.is_time_stop() {
        return Flow::Tail;
    }
    b.objects.get_mut(r).prevent_anim = 0;
    if let Some(flow) = cross_requests(b, r) {
        return flow;
    }
    if flag2(b, r) & 0x100 != 0 {
        start_drag(b, r);
        return Flow::Tail;
    }
    if flag1(b, r) & f1::DRAG != 0 {
        b.objects.get_mut(r).action = 5;
        return Flow::Tail;
    }
    b.objects.get_mut(r).unk_0d = 0;
    if flag2(b, r) & 0x10 != 0 {
        clear_flag2(b, r, 0x10);
        slide(b, r);
    } else if flag1(b, r) & f1::SLIDING != 0 {
        slide(b, r);
    } else {
        b.objects.get_mut(r).slide_state = 0;
    }
    if flag2(b, r) & 4 != 0 {
        flinch_request(b, r);
    }
    tick_flash(b, r);
    tick_statuses(b, r);
    tick_flag4_timer(b, r);
    tick_anger(b, r);
    drain_hp(b, r);
    // sub_802E1D8
    let side = b.objects.get(r).alliance as usize;
    b.sides[side].unk_30 = b.sides[side].unk_30.saturating_sub(1);
    Flow::Tail
}

/// `loc_801B142`: visibility, then the action (or the pause / time-stop
/// handler).
fn tail(b: &mut Battle, r: ObjectRef) {
    // sprite_zeroColorShader and the colour-shader helpers are
    // presentation.
    clear_flag2(b, r, 0x4000);
    update_visibility(b, r);
    if flag1(b, r) & f1::DEAD != 0 {
        return dispatch(b, r);
    }
    if b.paused && b.objects.get(r).action != 0 {
        return pause_requests(b, r);
    }
    if b.is_time_stop() {
        return time_stop(b, r);
    }
    dispatch(b, r);
}

/// `sub_801B9E6`: run the current action (§12.0 action table).
pub(super) fn dispatch(b: &mut Battle, r: ObjectRef) {
    let action = b.objects.get(r).action;
    if action >= 0x10 {
        if ai(b, r).attack.beast_lockon == 1 {
            panic!("Beast Out attack routine (sub_80EAD9C) is not implemented yet");
        }
        return actions::dispatch(b, r, action);
    }
    if ai(b, r).ai_index != 0 && action > 8 {
        panic!("form action {action} is not implemented yet");
    }
    match action {
        0 => entry::entry(b, r),
        1 => entry::take_control(b, r),
        2 => reactions::deletion(b, r),
        3 => reactions::flinch(b, r),
        4 => reactions::paralysis(b, r),
        5 => reactions::drag(b, r),
        6 => reactions::freeze(b, r),
        7 => reactions::bubble(b, r),
        8 => idle::control(b, r),
        _ => panic!("player action {action} is past MegaMan's action table"),
    }
}

// ---- Damage ---------------------------------------------------------------------

/// Spawn the weakness / bug "!" marker (`sub_80E8124`, effect #0x6B) at
/// the navi's attach point 5.
fn spawn_marker(b: &mut Battle, r: ObjectRef, param: u8) {
    let (dx, dy) = attach_point(b, r, 5);
    let pos = Vec3 { x: dx << 16, y: 0, z: dy << 16 };
    if let Some(m) = b.objects.spawn(Pool::Effect, 0x6B, pos, [param, 0, 0, 0]) {
        b.objects.get_mut(m).related[0] = Some(r);
    }
}

/// `sub_801A42E`: a weakness hit that did damage shows "!!".
fn weakness_effect(b: &mut Battle, r: ObjectRef) {
    let c = coll(b, r);
    if c.acc.exclamation != 0 && c.acc.final_damage != 0 {
        spawn_marker(b, r, 0);
    }
}

/// `sub_801A4A6`: an HP-bug hit shows its marker.
fn bug_effect(b: &mut Battle, r: ObjectRef) {
    if matches!(coll(b, r).acc.inflicted_bugs as u8, 0xF4 | 0xF7) {
        spawn_marker(b, r, 3);
    }
}

/// `sub_801A45C`: a counter hit fills the attacker's per-side gauge (flag
/// 0x40 mode), is counted, and closes the counter window.
fn counter_hit_bookkeeping(b: &mut Battle, r: ObjectRef) {
    if coll(b, r).acc.hit_flags & 0x40 == 0 {
        return;
    }
    let opp = b.objects.get(r).alliance ^ 1;
    if is_mode_40(b) {
        // sub_802E032
        let s = &mut b.sides[opp as usize];
        s.gauge = (s.gauge as u32 + 0x1500).min(0x4000) as u16;
    }
    b.bump_side_stat(opp, 8, 1);
    coll_mut(b, r).counter_timer = 0;
    // Unless the battle is over: the "COUNTER" HUD text and sound 0x86.
}

/// `sub_801A506`: note a damaging weakness hit.
fn weakness_request(b: &mut Battle, r: ObjectRef) {
    let c = coll(b, r);
    if c.acc.damage_multiplier != 0 && c.acc.final_damage != 0 {
        ai_mut(b, r).requests |= request::WEAKNESS_HIT;
    }
}

/// `applyDamageToPlayer_801ba12`: subtract the final damage (Undershirt
/// keeps 1 HP), then the element-5 damage; at 0 HP request deletion
/// (§4.5). Runs every tick, even once dead or after the battle ends.
fn apply_damage(b: &mut Battle, r: ObjectRef) {
    let mut d = coll(b, r).acc.final_damage;
    let mut dead = false;
    if d != 0 {
        let a = ai_mut(b, r);
        a.total_damage_taken = (a.total_damage_taken as u32 + d as u32).min(0xFFFF) as u16;
        let hp = b.objects.get(r).hp;
        if hp > 1 && flag1(b, r) & f1::UNDERSHIRT != 0 && hp <= d {
            d = hp - 1;
        }
        crate::kinds::subtract_hp(b, r, d);
        // Sound 0x6B (local player) or 0x6D; sprite_forceWhitePalette.
        b.objects.sprite_mut(r).look.white = true;
        dead = b.objects.get(r).hp == 0;
    }
    if !dead {
        let d5 = coll(b, r).acc.element_damage[5];
        crate::kinds::subtract_hp(b, r, d5);
        dead = b.objects.get(r).hp == 0;
    }
    if dead {
        if cross_protected(b, r) {
            ai_mut(b, r).requests |= request::CROSS_DEATH;
        } else {
            set_flag2(b, r, 1);
        }
    }
    counter_and_mood(b, r);
}

/// `sub_801A200`: a counter landed on this navi gives the attacker Full
/// Synchro; hits wear down this navi's mood.
fn counter_and_mood(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() {
        return;
    }
    let side = b.objects.get(r).alliance;
    let opp = side ^ 1;
    let opp_form = b.stats[opp as usize].form;
    if coll(b, r).acc.counter & 0x8000 != 0 && matches!(opp_form, Form::NONE | Form::GREGAR_BEAST | Form::FALZAR_BEAST)
    {
        let a = ai(b, r);
        if a.unk_32 == 0 && a.unk_36 == 0 {
            set_mood(b, opp, 0xFF);
        }
    }
    // sub_8015C12
    let loss = coll(b, r).acc.mood_damage;
    let s = &mut b.stats[side as usize];
    if s.mood != 0 {
        s.mood = (s.mood as i32 - loss as i32).max(1) as u8;
    }
}

// ---- Special states ------------------------------------------------------------

/// `sub_8015766`: Cross lanes.
fn cross_lane(_b: &mut Battle, _r: ObjectRef) -> bool {
    panic!("Cross lanes (sub_8015766) are not implemented yet");
}

/// The Cross/Beast requests in `ai.requests` (none fire for base
/// MegaMan).
fn cross_requests(b: &mut Battle, r: ObjectRef) -> Option<Flow> {
    let f = ai(b, r).requests;
    if f & request::CROSS_DEATH != 0 {
        ai_mut(b, r).requests &= !request::CROSS_DEATH;
        ai_mut(b, r).status |= ai_status::CROSS_2000;
        set_attack(b, r, 0x4C, 0);
        return Some(Flow::Dispatch);
    }
    if f & request::ACTION_30 != 0 {
        ai_mut(b, r).requests &= !request::ACTION_30;
        ai_mut(b, r).status |= ai_status::CROSS_10000;
        set_attack(b, r, 0x30, 0);
        return Some(Flow::Dispatch);
    }
    if f & request::WEAKNESS_HIT != 0 {
        ai_mut(b, r).requests &= !request::WEAKNESS_HIT;
        if (0x1AC..=0x1C1).contains(&b.objects.get(r).name_id) {
            ai_mut(b, r).status |= ai_status::CROSS_40000;
            exit_attack_state(b, r);
            cross_lane(b, r);
            return Some(Flow::Return);
        }
    }
    None
}

/// Stage B's drag request (`F2 & 0x100`): save the state, clear freeze
/// and bubble (and paralysis, unless kept) and start action 5.
fn start_drag(b: &mut Battle, r: ObjectRef) {
    clear_flag2(b, r, 0x100);
    save_state_word(b, r);
    reset_attack_links(b, r);
    clear_freeze(b, r);
    clear_bubble(b, r);
    let f = flag2(b, r);
    let keep_paralysis = f & 0x4000 != 0 || (f & 2 == 0 && b.objects.get(r).action == 4);
    if !keep_paralysis {
        clear_paralysis(b, r);
    }
    clear_flag2(b, r, 0x4000);
    let o = b.objects.get_mut(r);
    o.action = 5;
    o.phase = 0;
    o.unk_0d = 0;
}

/// `sub_80166B6`: the ice / road / push slide machine.
fn slide(_b: &mut Battle, _r: ObjectRef) {
    panic!("slides (sub_80166B6) are not implemented yet");
}

/// Stage B's flinch request (`F2 & 4`): flinch unless paralyzed or frozen
/// by a hit without the flash bit (§4.7).
fn flinch_request(b: &mut Battle, r: ObjectRef) {
    clear_flag2(b, r, 4);
    reset_attack_links(b, r);
    let action = b.objects.get(r).action;
    // sub_801BA92
    let mut a = 0;
    if action == 4 {
        a = 1;
        let f = flag2(b, r);
        if f & 0x4000 == 0 && f & 2 != 0 {
            clear_paralysis(b, r);
            a = 2;
        }
    }
    // sub_801BABE
    let mut c = 0;
    if action == 6 {
        c = 1;
        if flag2(b, r) & 2 != 0 {
            clear_freeze(b, r);
            c = 2;
        }
    }
    let both = a | c;
    if both == 0 || both & 2 != 0 {
        set_attack(b, r, 3, 0);
    }
    clear_flag2(b, r, 0x4000);
}

// ---- Timers ------------------------------------------------------------------------

/// `sub_801A5EE`: mercy invincibility: a flash request starts 120 ticks
/// of FLASHING (not extended by new requests).
fn tick_flash(b: &mut Battle, r: ObjectRef) {
    if b.round.flags & battle_flags::FIGHTING == 0 {
        return;
    }
    if coll(b, r).status_timers[timer::FLASH] == 0 && flag2(b, r) & 2 != 0 {
        coll_mut(b, r).status_timers[timer::FLASH] = 120;
    }
    clear_flag2(b, r, 2);
    let t = coll(b, r).status_timers[timer::FLASH];
    if t != 0 {
        let t = t as i32 - 1;
        coll_mut(b, r).status_timers[timer::FLASH] = t as u16;
        if t > 0 {
            set_flag1(b, r, f1::FLASHING);
            return;
        }
        // Sound 0x94 if invisible.
    }
    clear_flag1(b, r, f1::FLASHING | f1::INVISIBLE);
}

/// Count a status timer down (`t - 1` on the u16); true while it runs.
fn count_down(b: &mut Battle, r: ObjectRef, i: usize) -> bool {
    let t = coll(b, r).status_timers[i] as i32 - 1;
    coll_mut(b, r).status_timers[i] = if t > 0 { t as u16 } else { 0 };
    t > 0
}

/// Enter a status action at phase 0, saving the state word first.
fn enter_status_action(b: &mut Battle, r: ObjectRef, action: u8) {
    save_state_word(b, r);
    let o = b.objects.get_mut(r);
    o.action = action;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_800E730`: the status timers and their requests (§H5). `F2` is
/// read once at entry. Returns early while paused.
fn tick_statuses(b: &mut Battle, r: ObjectRef) {
    let f2 = flag2(b, r);
    if b.paused {
        return;
    }
    tick_paralysis(b, r, f2);
    if !tick_freeze(b, r, f2) {
        if !count_down(b, r, timer::BUBBLE) {
            clear_flag1(b, r, f1::BUBBLED);
            coll_mut(b, r).links[link::BUBBLE] = None;
        } else if !bubble_active(b, r, f2) {
            // A stale bubble visual skips the confusion countdown.
            confusion_active(b, r, f2);
            return tick_minor_statuses(b, r, f2);
        }
    } else {
        // A stale ice visual skips the bubble countdown.
        if !bubble_active(b, r, f2) {
            confusion_active(b, r, f2);
            return tick_minor_statuses(b, r, f2);
        }
    }
    if count_down(b, r, timer::CONFUSE) {
        confusion_active(b, r, f2);
    } else {
        clear_flag1(b, r, f1::CONFUSED);
        clear_flag2(b, r, 0x80);
        coll_mut(b, r).links[link::CONFUSE] = None;
    }
    tick_minor_statuses(b, r, f2);
}

/// Paralysis: a request stops the other statuses and enters action 4.
fn tick_paralysis(b: &mut Battle, r: ObjectRef, f2: u32) {
    if !count_down(b, r, timer::PARALYZE) {
        clear_flag1(b, r, f1::PARALYZED);
        clear_flag2(b, r, 0x8);
        return;
    }
    if f2 & 0x8 != 0 {
        clear_flag2(b, r, 0x88);
        enter_status_action(b, r, 4);
        let c = coll_mut(b, r);
        c.status_timers[timer::CONFUSE] = 0;
        c.status_timers[timer::FREEZE] = 0;
        c.status_timers[timer::BUBBLE] = 0;
    }
    clear_flag1(b, r, f1::BUBBLED | f1::FROZEN | f1::CONFUSED);
    if flag1(b, r) & f1::PARALYZED == 0 {
        set_flag1(b, r, f1::PARALYZED);
        enter_status_action(b, r, 4);
    }
}

/// Freeze: like paralysis with action 6 and the ice visual. Returns true
/// when a stale ice visual makes the game jump into the bubble's active
/// branch.
fn tick_freeze(b: &mut Battle, r: ObjectRef, f2: u32) -> bool {
    if !count_down(b, r, timer::FREEZE) {
        clear_flag1(b, r, f1::FROZEN);
        return false;
    }
    if f2 & 0x1_0000 != 0 {
        clear_flag2(b, r, 0x3_0080);
        enter_status_action(b, r, 6);
        let c = coll_mut(b, r);
        c.status_timers[timer::CONFUSE] = 0;
        c.status_timers[timer::PARALYZE] = 0;
        c.status_timers[timer::BUBBLE] = 0;
    }
    clear_flag1(b, r, f1::BUBBLED | f1::CONFUSED);
    if flag1(b, r) & f1::FROZEN != 0 {
        return false;
    }
    set_flag1(b, r, f1::FROZEN);
    enter_status_action(b, r, 6);
    if coll(b, r).links[link::FREEZE].is_some() {
        return true;
    }
    panic!("the ice visual (sub_80E9BDC) is not implemented yet");
}

/// The bubble's active branch. Returns false when a stale bubble visual
/// makes the game jump into the confusion's active branch.
fn bubble_active(b: &mut Battle, r: ObjectRef, f2: u32) -> bool {
    if f2 & 0x2_0000 != 0 {
        clear_flag2(b, r, 0x2_0080);
        enter_status_action(b, r, 7);
        let c = coll_mut(b, r);
        c.status_timers[timer::CONFUSE] = 0;
        c.status_timers[timer::PARALYZE] = 0;
        c.status_timers[timer::FREEZE] = 0;
    }
    clear_flag1(b, r, f1::CONFUSED);
    if flag1(b, r) & f1::BUBBLED != 0 {
        return true;
    }
    set_flag1(b, r, f1::BUBBLED);
    enter_status_action(b, r, 7);
    if coll(b, r).links[link::BUBBLE].is_some() {
        return false;
    }
    panic!("the bubble visual (sub_80E4B34) is not implemented yet");
}

/// The confusion's active branch.
fn confusion_active(b: &mut Battle, r: ObjectRef, f2: u32) {
    if f2 & 0x80 != 0 {
        clear_flag2(b, r, 0x3_0088);
        let c = coll_mut(b, r);
        c.status_timers[timer::PARALYZE] = 0;
        c.status_timers[timer::FREEZE] = 0;
        c.status_timers[timer::BUBBLE] = 0;
    }
    set_flag1(b, r, f1::CONFUSED);
    clear_flag1(b, r, f1::BUBBLED | f1::FROZEN | f1::PARALYZED);
    if coll(b, r).links[link::CONFUSE].is_none() {
        panic!("the confusion visual (sub_80E09EE) is not implemented yet");
    }
}

/// Immobilize, blind and the invulnerability timer.
fn tick_minor_statuses(b: &mut Battle, r: ObjectRef, f2: u32) {
    if !count_down(b, r, timer::IMMOBILIZE) {
        clear_flag1(b, r, f1::IMMOBILIZED);
    } else if f2 & 0x40 != 0 {
        clear_flag2(b, r, 0x40);
        set_flag1(b, r, f1::IMMOBILIZED);
    }
    if !count_down(b, r, timer::BLIND) {
        clear_flag1(b, r, f1::BLIND);
        coll_mut(b, r).links[link::BLIND] = None;
    } else if f2 & 0x20 != 0 {
        clear_flag2(b, r, 0x20);
        set_flag1(b, r, f1::BLIND);
        if coll(b, r).links[link::BLIND].is_none() {
            panic!("the blindness visual (sub_80E09EE) is not implemented yet");
        }
    }
    if count_down(b, r, timer::INVULNERABLE) {
        set_flag1(b, r, f1::INVULNERABLE);
    } else {
        clear_flag1(b, r, f1::INVULNERABLE);
    }
}

/// `sub_8010162`: the timed flag-4 state (0xFFFF = indefinite); the flag
/// is off while an action runs.
fn tick_flag4_timer(b: &mut Battle, r: ObjectRef) {
    let t = coll(b, r).status_timers[timer::UNK_26];
    if t != 0xFFFF {
        let t = t as i32 - 1;
        if t < 0 {
            clear_flag1(b, r, f1::UNK_4);
            return;
        }
        coll_mut(b, r).status_timers[timer::UNK_26] = t as u16;
        // At 0: sound 0x94.
    }
    if flag1(b, r) & f1::USING_ACTION != 0 {
        clear_flag1(b, r, f1::UNK_4);
    } else {
        set_flag1(b, r, f1::UNK_4);
    }
}

/// `sub_8014326`: anger lasts 600 ticks (player navis whose emotion
/// allows it).
fn tick_anger(b: &mut Battle, r: ObjectRef) {
    if navi_record(b, r).actor_type != ActorType::Player {
        return;
    }
    let side = b.objects.get(r).alliance;
    if matches!(emotion(b, side), 5 | 1) {
        clear_flag2(b, r, 0x200);
        clear_flag1(b, r, f1::ANGER);
        return;
    }
    if flag1(b, r) & f1::ANGER == 0 {
        if flag2(b, r) & 0x200 != 0 {
            clear_flag2(b, r, 0x200);
            set_flag1(b, r, f1::ANGER);
            ai_mut(b, r).anger = 600;
            set_mood(b, side, 0x80);
        }
        return;
    }
    clear_flag2(b, r, 0x200);
    let a = ai_mut(b, r);
    if a.anger != 0 {
        a.anger -= 1;
        if a.anger > 0 {
            return;
        }
    }
    end_anger(b, r);
}

/// `sub_80143A6`: calm down.
fn end_anger(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.stats[side].mood = 0x80;
    clear_flag1(b, r, f1::ANGER);
    clear_flag2(b, r, 0x200);
    let a = ai_mut(b, r);
    a.anger = 0;
    a.unk_4c = 0;
}

/// `sub_8014498`: while `Unk_36` is set, lose 1 HP per tick (never to 0).
fn drain_hp(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() || ai(b, r).unk_36 == 0 {
        return;
    }
    let o = b.objects.get_mut(r);
    if o.hp.wrapping_sub(1) != 0 {
        o.hp = o.hp.wrapping_sub(1);
    }
}

// ---- Tail ---------------------------------------------------------------------------

/// `sub_8016934`: visible unless flashing (2 ticks off, 2 on) or the
/// local navi is blind and this is the other side's.
fn update_visibility(b: &mut Battle, r: ObjectRef) {
    if !b.is_time_stop() {
        b.objects.get_mut(r).flags |= flags::VISIBLE;
    }
    let f = flag1(b, r);
    if f & f1::DEAD != 0 {
        return;
    }
    if f & (f1::FLASHING | f1::INVISIBLE) != 0 && coll(b, r).status_timers[timer::FLASH] & 2 != 0 {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
    }
    let alliance = b.objects.get(r).alliance;
    if !b.is_remote(alliance) {
        // The local navi: HUD markers only.
        return;
    }
    let Some(viewer) = b.player(alliance ^ 1) else { return };
    let viewer_blind = b.objects.get(viewer).collision.map(|c| b.collision.get(c).f1 & f1::BLIND != 0).unwrap_or(false);
    if viewer_blind {
        b.objects.get_mut(r).flags &= !flags::VISIBLE;
    }
}

/// `sub_8017BC0`: while paused, requests start the pause-time action 0x1C
/// (form changes and the like).
fn pause_requests(b: &mut Battle, r: ObjectRef) {
    let st = ai(b, r).status;
    if st & ai_status::FORM_CHANGE != 0 {
        panic!("form change (sub_8014A38) is not implemented yet");
    }
    if st & 0x100 != 0 {
        panic!("pause action (sub_8015614) is not implemented yet");
    }
    if st & 0x1000 != 0 {
        panic!("pause action (sub_802D714) is not implemented yet");
    }
    if st & 0x2000 != 0 {
        panic!("pause action (sub_802D926) is not implemented yet");
    }
    let f = ai(b, r).requests;
    let (bit, state) = if f & request::FORM_CHANGE != 0 {
        (request::FORM_CHANGE, ai_status::FORM_CHANGE)
    } else if f & request::PAUSE_40 != 0 {
        // Saves the state word after zeroing it.
        b.objects.get_mut(r).saved_state = None;
        save_state_word(b, r);
        (request::PAUSE_40, 0x100)
    } else if f & request::PAUSE_4000000 != 0 {
        (request::PAUSE_4000000, 0x1000)
    } else if f & request::CROSS_DEATH != 0 {
        (request::CROSS_DEATH, 0x2000)
    } else {
        return;
    };
    let a = ai_mut(b, r);
    a.requests &= !bit;
    a.status |= state;
    set_attack(b, r, 0x1C, 0);
}

/// `sub_8017AB4`: in time stop the navi only shakes while hit (one RNG
/// draw per shaking tick).
fn time_stop(b: &mut Battle, r: ObjectRef) {
    let player = navi_record(b, r).actor_type == ActorType::Player;
    if player && is_link(b) && ai(b, r).requests & request::TIMESTOP_CHIP != 0 {
        panic!("time-stop counter chips (sub_8017AB4) are not implemented yet");
    }
    let o = b.objects.get_mut(r);
    if o.prevent_anim == 0 {
        o.unk_30 = (o.pos.x >> 16) as u16;
        o.unk_32 = (o.pos.z >> 16) as u16;
        o.unk_19 = 0;
        o.prevent_anim = 4;
    }
    if coll(b, r).acc.final_damage != 0 {
        b.objects.get_mut(r).unk_19 = 30;
    }
    let o = b.objects.get(r);
    if o.unk_19 != 0 {
        let base = Vec3 { x: (o.unk_30 as i32) << 16, y: o.pos.y, z: (o.unk_32 as i32) << 16 };
        b.objects.get_mut(r).unk_19 -= 1;
        let pos = crate::kinds::spark::jitter(b, 3, base);
        b.objects.get_mut(r).pos = pos;
    } else {
        let o = b.objects.get_mut(r);
        o.pos.x = (o.pos.x & 0xFFFF) | ((o.unk_30 as i32) << 16);
        o.pos.z = (o.pos.z & 0xFFFF) | ((o.unk_32 as i32) << 16);
    }
}
