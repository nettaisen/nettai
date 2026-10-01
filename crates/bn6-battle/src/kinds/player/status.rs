//! Stage B of the damage pipeline (`sub_801AF44`): apply the damage,
//! turn requests into reactions (death, drag, slide, flinch), run the
//! flash and status timers, then dispatch the current action. See
//! objects-and-player.md §H3-§H5 and field-collision-damage.md §4.

use super::{
    actions, ai, ai_mut, attach_point, clear_bubble, clear_flag1, clear_flag2, clear_freeze, clear_paralysis, coll,
    Emotion, coll_mut, cross_protected, emotion, entry, exit_attack_state, flag1, flag2, idle, is_link, per_player_gauges, navi_record,
    coordinates_to_panel, panel_kind, reactions, reset_attack_links, save_state_word, set_attack, navi_action,
    set_navi_action, NaviAction,
    set_coordinates_from_panel, set_flag1, set_flag2, set_mood,
};
use crate::actor::{ActorType, request, status as ai_status};
use crate::battle::{Battle, battle_flags};
use crate::collision::{f1, link, timer};
use crate::field::PanelType;
use crate::object::{DragStep, ObjectRef, PanelPos, Vec3, flags};
use crate::content::FormKind;

/// `sub_801AF44`, including the action dispatch (`sub_801B9E6`).
pub(super) fn update(b: &mut Battle, r: ObjectRef) {
    if !b.paused || navi_action(b, r) == NaviAction::Entry {
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
    /// The visuals and the pause/dimming/dispatch choice (`loc_801B142`).
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
    if navi_action(b, r) == NaviAction::Bubble && coll(b, r).acc.final_damage != 0 {
        clear_bubble(b, r);
    }
    if flag1(b, r) & f1::DEAD != 0 {
        return Flow::Tail;
    }
    if flag2(b, r) & 1 != 0 {
        clear_flag2(b, r, 1);
        set_flag1(b, r, f1::DEAD);
        set_attack(b, r, NaviAction::Deletion, 0);
        return Flow::Tail;
    }
    let st = ai(b, r).status;
    if st & (ai_status::CROSS_KNOCKOUT | ai_status::VOLLEY | ai_status::UNINTERRUPTIBLE) != 0 {
        return Flow::Dispatch;
    }
    if st & ai_status::CROSS_BREAKING != 0 && cross_lane(b, r) {
        return Flow::Return;
    }
    if b.is_dimmed() {
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
        set_navi_action(b, r, NaviAction::Drag);
        return Flow::Tail;
    }
    b.objects.get_mut(r).drag_step = DragStep::Start;
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
    tick_submerged(b, r);
    tick_anger(b, r);
    drain_hp(b, r);
    // sub_802E1D8: the side's Cross special runs down.
    let side = &mut b.sides[b.objects.get(r).alliance as usize];
    side.cross_special_ticks = side.cross_special_ticks.saturating_sub(1);
    Flow::Tail
}

/// `loc_801B142`: visibility, then the action (or the pause / dimming
/// handler).
fn tail(b: &mut Battle, r: ObjectRef) {
    clear_flag2(b, r, 0x4000);
    status_shader(b, r);
    update_visibility(b, r);
    counter_shader(b, r);
    if flag1(b, r) & f1::DEAD != 0 {
        return dispatch(b, r);
    }
    if b.paused && navi_action(b, r) != NaviAction::Entry {
        return pause_requests(b, r);
    }
    if b.is_dimmed() {
        return while_dimmed(b, r);
    }
    dispatch(b, r);
}

/// `sub_801B9E6`: run the current action (§12.0 action table).
pub(super) fn dispatch(b: &mut Battle, r: ObjectRef) {
    let action = navi_action(b, r);
    if action.is_attack(&b.content.defs) {
        if ai(b, r).attack.beast_lockon == 1 {
            return actions::beast_rush::update(b, r);
        }
        return actions::dispatch(b, r, action);
    }
    match action {
        NaviAction::Entry => entry::entry(b, r),
        NaviAction::TakeControl => entry::take_control(b, r),
        NaviAction::Deletion => reactions::deletion(b, r),
        NaviAction::Flinch => reactions::flinch(b, r),
        NaviAction::Paralysis => reactions::paralysis(b, r),
        NaviAction::Drag => reactions::drag(b, r),
        NaviAction::Freeze => reactions::freeze(b, r),
        NaviAction::Bubble => reactions::bubble(b, r),
        NaviAction::Idle => idle::control(b, r),
        // The link navis' own actions (`off_80EA4C8[AIIndex]` past idle):
        // content's.
        NaviAction::Content(h) => crate::behavior::run_action(b, h, r),
        NaviAction::Unported(n) if ai(b, r).ai_index != 0 => panic!("form action {n} is not implemented yet"),
        NaviAction::Unported(n) => panic!("player action {n} is past MegaMan's action table"),
        NaviAction::Engine(_) => unreachable!("the ruleset's actions are attacks"),
    }
}

// ---- Damage ---------------------------------------------------------------------

/// Spawn the weakness / bug "!" marker (`sub_80E8124`, effect #0x6B) at
/// the navi's attach point 5.
fn spawn_marker(b: &mut Battle, r: ObjectRef, mark: crate::kinds::hit_marker::Mark) {
    let (dx, dy) = attach_point(b, r, 5);
    let offset = Vec3 { x: dx << 16, y: 0, z: dy << 16 };
    crate::kinds::hit_marker::spawn(b, r, offset, mark);
}

/// `sub_801A42E`: a weakness hit that did damage shows "!!".
fn weakness_effect(b: &mut Battle, r: ObjectRef) {
    let c = coll(b, r);
    if c.acc.exclamation != 0 && c.acc.final_damage != 0 {
        spawn_marker(b, r, crate::kinds::hit_marker::Mark::Weakness);
    }
}

/// `sub_801A4A6`: an HP-bug hit shows its marker.
fn bug_effect(b: &mut Battle, r: ObjectRef) {
    if matches!(coll(b, r).acc.inflicted_bugs as u8, 0xF4 | 0xF7) {
        spawn_marker(b, r, crate::kinds::hit_marker::Mark::Bug);
    }
}

/// `sub_801A45C`: a counter hit fills the attacker's per-side gauge (flag
/// 0x40 mode), is counted, and closes the counter window.
fn counter_hit_bookkeeping(b: &mut Battle, r: ObjectRef) {
    if coll(b, r).acc.hit_flags & 0x40 == 0 {
        return;
    }
    let opp = b.objects.get(r).alliance ^ 1;
    if per_player_gauges(b) {
        // sub_802E032
        let s = &mut b.sides[opp as usize];
        s.gauge = (s.gauge as u32 + 0x1500).min(0x4000) as u16;
    }
    b.bump_side_stat(opp, 8, 1);
    coll_mut(b, r).counter_timer = 0;
    // Unless the battle is over: the HUD's "COUNTER HIT!" and a sound.
    if !b.is_battle_over() {
        b.show_message(crate::hud::Message::CounterHit);
        b.sound(crate::content::SoundRole::CounterHit);
    }
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
        // A player hears another sound when their own navi is hit;
        // sprite_forceWhitePalette.
        let player = navi_record(b, r).actor_type == ActorType::Player;
        let alliance = b.objects.get(r).alliance;
        for side in 0..2 {
            let own = player && side == alliance;
            b.sound_for(side, if own { crate::content::SoundRole::OwnHit } else { crate::content::SoundRole::Hit });
        }
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
    let opp_form = b.form(opp as usize).kind;
    if coll(b, r).acc.counter & 0x8000 != 0 && matches!(opp_form, FormKind::Base | FormKind::Beast) {
        let a = ai(b, r);
        if !a.beast_out_spent && !a.beast_over_exhausted {
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

/// `sub_8015766`: a weakness hit breaks the Cross; true while it runs.
fn cross_lane(b: &mut Battle, r: ObjectRef) -> bool {
    actions::transform::break_cross(b, r)
}

/// The Cross/Beast requests in `ai.requests` (none fire for base
/// MegaMan).
fn cross_requests(b: &mut Battle, r: ObjectRef) -> Option<Flow> {
    let f = ai(b, r).requests;
    if f & request::CROSS_DEATH != 0 {
        ai_mut(b, r).requests &= !request::CROSS_DEATH;
        ai_mut(b, r).status |= ai_status::CROSS_KNOCKOUT;
        let death = super::role_action(b, crate::content::ActionRole::CrossDeath);
        set_attack(b, r, death, 0);
        return Some(Flow::Dispatch);
    }
    if f & request::VOLLEY != 0 {
        ai_mut(b, r).requests &= !request::VOLLEY;
        ai_mut(b, r).status |= ai_status::VOLLEY;
        let volley = super::role_action(b, crate::content::ActionRole::Volley);
        set_attack(b, r, volley, 0);
        return Some(Flow::Dispatch);
    }
    if f & request::WEAKNESS_HIT != 0 {
        ai_mut(b, r).requests &= !request::WEAKNESS_HIT;
        // A Cross, Beast Out or a Cross in Beast Out (NameIDs 0x1AC..=0x1C1).
        use crate::content::IdentityClass;
        let class = b.content.identity(b.objects.get(r).identity).class;
        if matches!(class, IdentityClass::Cross | IdentityClass::Beast | IdentityClass::CrossBeast) {
            ai_mut(b, r).status |= ai_status::CROSS_BREAKING;
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
    let keep_paralysis = f & 0x4000 != 0 || (f & 2 == 0 && navi_action(b, r) == NaviAction::Paralysis);
    if !keep_paralysis {
        clear_paralysis(b, r);
    }
    clear_flag2(b, r, 0x4000);
    let o = b.objects.get_mut(r);
    o.phase = 0;
    o.drag_step = DragStep::Start;
    set_navi_action(b, r, NaviAction::Drag);
}

/// `slide_state` values: where the slide machine is.
mod slide_state {
    /// Start a slide (`sub_80166D0`).
    pub const START: u8 = 0;
    /// Sliding toward the destination panel (`sub_8016730`).
    pub const SLIDING: u8 = 4;
}

/// Ticks a slide takes per panel.
const SLIDE_TICKS: u8 = 4;

/// `sub_80166B6`: the ice / road / push slide machine: a panel every 4
/// ticks, the navi's panel following its coordinates on the way.
fn slide(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).slide_state {
        slide_state::START => start_slide(b, r),
        _ => continue_slide(b, r),
    }
}

/// `object_setCollisionPanelsToCurrent`: the collision anchor moves to the
/// navi's panel (the move direction is left alone).
fn anchor_collision(b: &mut Battle, r: ObjectRef) {
    let p = b.objects.get(r).panel;
    coll_mut(b, r).panel = p;
}

/// `sub_80166D0`: onto the destination panel, then off in the slide's
/// direction, if its first panel is open.
fn start_slide(b: &mut Battle, r: ObjectRef) {
    set_flag1(b, r, f1::SLIDING);
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    set_coordinates_from_panel(b, r);
    anchor_collision(b, r);
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    let v = reactions::slide_vector(b, r);
    let o = b.objects.get_mut(r);
    o.slide_dx = v.dx as u8;
    o.slide_dy = v.dy as u8;
    o.slide_tiles = v.tiles;
    if v.tiles != 0 {
        let target = offset(o.panel, v.dx, v.dy);
        o.future_panel = target;
        b.reserve_panel(r, target.x, target.y);
        let o = b.objects.get_mut(r);
        o.slide_timer = SLIDE_TICKS;
        o.slide_state = slide_state::SLIDING;
        return;
    }
    if o.slide_type == 3 {
        ai_mut(b, r).road_cooldown = 5;
    }
    b.objects.get_mut(r).slide_type = 0;
    clear_flag1(b, r, f1::SLIDING);
}

/// `sub_8016730`: move toward the destination; there, go on (one panel
/// more for each ice panel, onto a road the road's way) or stop.
fn continue_slide(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let left = o.slide_timer as i32 - 1;
    o.slide_timer = left as u8;
    if left > 0 {
        let (dx, dy) = (o.slide_dx as i8 as i32, o.slide_dy as i8 as i32);
        o.pos.x = o.pos.x.wrapping_add(dx * 0xA_0000);
        o.pos.y = o.pos.y.wrapping_add(dy * 0x6_0000);
        o.panel = coordinates_to_panel(o.pos.x, o.pos.y);
        anchor_collision(b, r);
        return;
    }
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    b.objects.get_mut(r).panel = fp;
    set_coordinates_from_panel(b, r);
    let kind = panel_kind(b, fp);
    let mut go_on = true;
    if kind == PanelType::Ice && coll(b, r).element != 2 {
        let o = b.objects.get_mut(r);
        o.slide_tiles = o.slide_tiles.wrapping_add(1);
    } else if kind.is_road() && flag1(b, r) & 0x24 == 0 {
        if b.objects.get(r).slide_type == 3 {
            ai_mut(b, r).road_cooldown = 5;
            go_on = false;
        } else {
            // Onto a road: it takes over.
            let o = b.objects.get(r);
            let dir = slide_direction(o.slide_dx as i8, o.slide_dy as i8, o.alliance);
            coll_mut(b, r).direction = dir;
            b.objects.get_mut(r).slide_type = 3;
            let v = reactions::slide_vector(b, r);
            let o = b.objects.get_mut(r);
            o.slide_dx = v.dx as u8;
            o.slide_dy = v.dy as u8;
            if v.tiles == 0 {
                go_on = false;
            } else {
                o.slide_tiles = o.slide_tiles.wrapping_add(1);
            }
        }
    }
    if go_on {
        let o = b.objects.get_mut(r);
        let left = o.slide_tiles as i32 - 1;
        o.slide_tiles = left as u8;
        if left > 0 {
            let next = offset(o.future_panel, o.slide_dx as i8, o.slide_dy as i8);
            if reactions::can_slide_to(b, r, next) {
                let o = b.objects.get_mut(r);
                o.slide_timer = SLIDE_TICKS;
                o.future_panel = next;
                b.reserve_panel(r, next.x, next.y);
                return;
            }
        }
    }
    let o = b.objects.get(r);
    let dir = slide_direction(o.slide_dx as i8, o.slide_dy as i8, o.alliance);
    coll_mut(b, r).direction = dir;
    let o = b.objects.get_mut(r);
    o.slide_state = slide_state::START;
    clear_flag1(b, r, f1::SLIDING);
    b.objects.get_mut(r).slide_type = 0;
    anchor_collision(b, r);
}

/// The panel `(dx, dy)` from `p`.
fn offset(p: PanelPos, dx: i8, dy: i8) -> PanelPos {
    PanelPos { x: (p.x as i8).wrapping_add(dx) as u8, y: (p.y as i8).wrapping_add(dy) as u8 }
}

/// `sub_801683C`: the collision direction a slide leaves (1 up, 2 down,
/// 3 back, 4 forward, 5 none).
fn slide_direction(dx: i8, dy: i8, alliance: u8) -> u8 {
    let dx = if alliance != 0 { -dx } else { dx };
    match (dx.signum(), dy.signum()) {
        (1, _) => 4,
        (-1, _) => 3,
        (_, -1) => 1,
        (_, 1) => 2,
        _ => 5,
    }
}

/// Stage B's flinch request (`F2 & 4`): flinch unless paralyzed or frozen
/// by a hit without the flash bit (§4.7).
fn flinch_request(b: &mut Battle, r: ObjectRef) {
    clear_flag2(b, r, 4);
    reset_attack_links(b, r);
    let action = navi_action(b, r);
    // sub_801BA92
    let mut a = 0;
    if action == NaviAction::Paralysis {
        a = 1;
        let f = flag2(b, r);
        if f & 0x4000 == 0 && f & 2 != 0 {
            clear_paralysis(b, r);
            a = 2;
        }
    }
    // sub_801BABE
    let mut c = 0;
    if action == NaviAction::Freeze {
        c = 1;
        if flag2(b, r) & 2 != 0 {
            clear_freeze(b, r);
            c = 2;
        }
    }
    let both = a | c;
    if both == 0 || both & 2 != 0 {
        set_attack(b, r, NaviAction::Flinch, 0);
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
        if flag1(b, r) & f1::INVISIBLE != 0 {
            b.sound(crate::content::SoundRole::Appear);
        }
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
fn enter_status_action(b: &mut Battle, r: ObjectRef, action: NaviAction) {
    save_state_word(b, r);
    super::set_action(b, r, action);
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
        enter_status_action(b, r, NaviAction::Paralysis);
        let c = coll_mut(b, r);
        c.status_timers[timer::CONFUSE] = 0;
        c.status_timers[timer::FREEZE] = 0;
        c.status_timers[timer::BUBBLE] = 0;
    }
    clear_flag1(b, r, f1::BUBBLED | f1::FROZEN | f1::CONFUSED);
    if flag1(b, r) & f1::PARALYZED == 0 {
        set_flag1(b, r, f1::PARALYZED);
        enter_status_action(b, r, NaviAction::Paralysis);
    }
}

/// Freeze: like paralysis with action 6 and the ice visual. Returns true
/// when a stale ice visual makes the game jump into the bubble's active
/// branch (after spawning one it goes on to the bubble's countdown).
fn tick_freeze(b: &mut Battle, r: ObjectRef, f2: u32) -> bool {
    if !count_down(b, r, timer::FREEZE) {
        clear_flag1(b, r, f1::FROZEN);
        return false;
    }
    if f2 & 0x1_0000 != 0 {
        clear_flag2(b, r, 0x3_0080);
        enter_status_action(b, r, NaviAction::Freeze);
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
    enter_status_action(b, r, NaviAction::Freeze);
    if coll(b, r).links[link::FREEZE].is_some() {
        return true;
    }
    crate::kinds::ice_visual::spawn(b, r);
    false
}

/// The bubble's active branch. Returns false when a stale bubble visual
/// makes the game jump into the confusion's active branch.
fn bubble_active(b: &mut Battle, r: ObjectRef, f2: u32) -> bool {
    if f2 & 0x2_0000 != 0 {
        clear_flag2(b, r, 0x2_0080);
        enter_status_action(b, r, NaviAction::Bubble);
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
    enter_status_action(b, r, NaviAction::Bubble);
    if coll(b, r).links[link::BUBBLE].is_some() {
        return false;
    }
    crate::kinds::bubble_visual::spawn(b, r);
    true
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
        crate::kinds::status_visual::spawn(b, r, crate::kinds::status_visual::Status::Confusion);
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
            crate::kinds::status_visual::spawn(b, r, crate::kinds::status_visual::Status::Blindness);
        }
    }
    if count_down(b, r, timer::INVULNERABLE) {
        set_flag1(b, r, f1::INVULNERABLE);
    } else {
        clear_flag1(b, r, f1::INVULNERABLE);
    }
}

/// `sub_8010162`: the timed submerged state (0xFFFF = indefinite);
/// the flag is off while an action runs.
fn tick_submerged(b: &mut Battle, r: ObjectRef) {
    let t = coll(b, r).status_timers[timer::SUBMERGED];
    if t != 0xFFFF {
        let t = t as i32 - 1;
        if t < 0 {
            clear_flag1(b, r, f1::SUBMERGED);
            return;
        }
        coll_mut(b, r).status_timers[timer::SUBMERGED] = t as u16;
        if t == 0 {
            b.sound(crate::content::SoundRole::Appear);
        }
    }
    if flag1(b, r) & f1::USING_ACTION != 0 {
        clear_flag1(b, r, f1::SUBMERGED);
    } else {
        set_flag1(b, r, f1::SUBMERGED);
    }
}

/// `sub_8014326`: anger lasts 600 ticks (player navis whose emotion
/// allows it).
fn tick_anger(b: &mut Battle, r: ObjectRef) {
    if navi_record(b, r).actor_type != ActorType::Player {
        return;
    }
    let side = b.objects.get(r).alliance;
    if matches!(emotion(b, side), Emotion::WornOut | Emotion::Tired) {
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
pub(super) fn end_anger(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.stats[side].mood = 0x80;
    clear_flag1(b, r, f1::ANGER);
    clear_flag2(b, r, 0x200);
    let a = ai_mut(b, r);
    a.anger = 0;
    a.stun_ticks = 0;
}

/// `sub_8014498`: exhausted after Beast Over, lose 1 HP per tick (never
/// to 0).
fn drain_hp(b: &mut Battle, r: ObjectRef) {
    if b.is_battle_over() || !ai(b, r).beast_over_exhausted {
        return;
    }
    let o = b.objects.get_mut(r);
    if o.hp.wrapping_sub(1) != 0 {
        o.hp = o.hp.wrapping_sub(1);
    }
}

// ---- Tail ---------------------------------------------------------------------------

/// The glow of an invulnerable navi and of a SELECT special (`byte_80168A8`,
/// `byte_80172C8`): up and down over 32 ticks of the battle time.
const GLOW: [u16; 32] = [
    0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 30, 28, 26, 24, 22, 20, 18, 16, 14, 12, 10, 8, 6, 4, 2, 0,
];

/// The ChargeCross tackle's action number, during which an invulnerable
/// navi doesn't glow (`sub_8016860` reads CurAction; an action content
/// defines without a number isn't recognized).
const TACKLE_ACTION: u8 = 0x56;

/// The navi's colour shader for its statuses (presentation only;
/// `loc_801B142`: `sprite_zeroColorShader`, then `sub_80143E4`,
/// `sub_801690A`, `sub_8016860`, `sub_80168C8`, `sub_80168F0`, the later
/// ones over the earlier): red while angry, a black blink while
/// immobilized, a green glow while invulnerable (red in battle mode 1), a
/// yellow blink while paralyzed, pale blue while frozen. The blinks are two
/// ticks of the battle time on and two off.
fn status_shader(b: &mut Battle, r: ObjectRef) {
    let t = b.round.battle_time;
    let blink = |shader: u16| if t & 2 != 0 { shader } else { 0 };
    let f = flag1(b, r);
    let mut shader = 0;
    if ai(b, r).anger != 0 {
        shader = 0x000F;
    }
    if f & f1::IMMOBILIZED != 0 {
        shader = blink(0xFFFF);
    }
    let action = navi_action(b, r);
    if f & f1::INVULNERABLE != 0
        && !super::form_of(b, r).kind.is_beast_over()
        && action != NaviAction::Entry
        && action.number(&b.content.defs) != TACKLE_ACTION
    {
        let glow = GLOW[(t & 0x1F) as usize];
        shader = if super::battle_mode(b) == 1 { glow } else { glow << 5 };
    }
    if f & f1::PARALYZED != 0 {
        shader = blink(0x03FF);
    }
    if f & f1::FROZEN != 0 {
        shader = 0x7E94;
    }
    b.objects.sprite_mut(r).look.color_shader = shader;
}

/// The shaders after the visibility (presentation only; `sub_8016CA4`,
/// `sub_801728E`): the other player's navi blinks blue while it can be
/// countered, to a local player in Full Synchro; in the per-player gauges'
/// mode a navi glows yellow while its SELECT special runs.
fn counter_shader(b: &mut Battle, r: ObjectRef) {
    let t = b.round.battle_time;
    let alliance = b.objects.get(r).alliance;
    if !b.is_battle_over()
        && b.is_remote(alliance)
        && b.player(alliance ^ 1).is_some()
        && emotion(b, alliance ^ 1) == Emotion::FullSynchro
        && coll(b, r).counter_timer != 0
    {
        b.objects.sprite_mut(r).look.color_shader = if t & 2 != 0 { 0x7C00 } else { 0 };
    }
    if per_player_gauges(b)
        && navi_action(b, r) != NaviAction::Entry
        && b.sides[alliance as usize & 1].select_special != 0
    {
        let glow = GLOW[(t & 0x1F) as usize];
        b.objects.sprite_mut(r).look.color_shader = glow | glow << 5;
    }
}

/// `sub_8016934`: visible unless flashing (2 ticks off, 2 on) or the
/// local navi is blind and this is the other side's.
fn update_visibility(b: &mut Battle, r: ObjectRef) {
    if !b.is_dimmed() {
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
        return actions::transform::form_change(b, r);
    }
    if st & ai_status::REVERTING_FORM != 0 {
        return actions::transform::revert(b, r);
    }
    if st & ai_status::CHANGING_CROSS != 0 {
        return actions::cross_change::change(b, r);
    }
    if st & ai_status::CROSS_KNOCKOUT != 0 {
        ai_mut(b, r).attack.variant = 0;
        return actions::cross_change::knock_out(b, r);
    }
    let f = ai(b, r).requests;
    let (bit, state) = if f & request::FORM_CHANGE != 0 {
        (request::FORM_CHANGE, ai_status::FORM_CHANGE)
    } else if f & request::REVERT_FORM != 0 {
        // Saves the state word after zeroing it.
        ai_mut(b, r).saved_word = None;
        save_state_word(b, r);
        (request::REVERT_FORM, ai_status::REVERTING_FORM)
    } else if f & request::CROSS_CHANGE != 0 {
        (request::CROSS_CHANGE, ai_status::CHANGING_CROSS)
    } else if f & request::CROSS_DEATH != 0 {
        (request::CROSS_DEATH, ai_status::CROSS_KNOCKOUT)
    } else {
        return;
    };
    let a = ai_mut(b, r);
    a.requests &= !bit;
    a.status |= state;
    set_attack(b, r, super::EngineAction::FormChange, 0);
}

/// `sub_800BEDA`: the navi may cut in on the other side's dimming: its own
/// side isn't freezing (or it is waiting on a counter), and the other
/// side's chip, which can be cut in on, is showing its telop.
fn can_cut_in(b: &Battle, r: ObjectRef) -> bool {
    use crate::dimming::DimmingState;
    let side = b.objects.get(r).alliance as usize;
    let own = b.dimming[side];
    let other = b.dimming[side ^ 1];
    own.user.is_none_or(|u| u == r)
        && matches!(own.state, DimmingState::Idle | DimmingState::Waiting)
        && !other.no_cut_in
        && other.state == DimmingState::ShowingName
}

/// `sub_8017AB4`'s counter cut-in: the next chip is used without leaving
/// the current action (its attack variables are a scratch copy); a dimming
/// chip's (action 0x15) or navi chip's (0x1B) controller is spawned and
/// takes the dimming over (`loc_800BF30`), with the cut-in flash, and the
/// hand moves on. A chip of any other action registers nothing and stays
/// in the hand. See docs/engine/chips.md §3.6.5.
fn cut_in(b: &mut Battle, r: ObjectRef) {
    let (action, a) = super::chip_use::prepare_detached(b, r);
    let controller = match action {
        NaviAction::Engine(super::EngineAction::DimmingChip) => actions::dimming_chip::spawn_controller(b, r, &a),
        NaviAction::Engine(super::EngineAction::NaviChip) => actions::navi_chip::spawn_controller(b, r, &a),
        _ => return,
    };
    let side = b.objects.get(r).alliance;
    b.cut_in_dimming(side, controller, r);
    crate::dimming::cut_in_flash(b, side);
    // sub_800FC7C
    b.hands[side as usize].advance();
}

/// `sub_8017AB4`: while dimmed the navi only shakes while hit (one RNG
/// draw per shaking tick).
fn while_dimmed(b: &mut Battle, r: ObjectRef) {
    let player = navi_record(b, r).actor_type == ActorType::Player;
    if player && is_link(b) && ai(b, r).requests & request::CUT_IN != 0 {
        // The next chip must be a dimming chip too.
        let chip = super::next_chip(b, r);
        let freezes = chip.is_some_and(|c| b.content.chip(c).flags.has(crate::content::ChipFlags::DIMMING));
        if can_cut_in(b, r) && freezes {
            cut_in(b, r);
        }
        ai_mut(b, r).requests &= !(request::CUT_IN | request::CHARGED_CHIP | request::CHIP);
    }
    let o = b.objects.get_mut(r);
    if o.prevent_anim == 0 {
        o.shake_origin_x = (o.pos.x >> 16) as i16;
        o.shake_origin_z = (o.pos.z >> 16) as i16;
        o.shake_timer = 0;
        o.prevent_anim = 4;
    }
    if coll(b, r).acc.final_damage != 0 {
        b.objects.get_mut(r).shake_timer = 30;
    }
    let o = b.objects.get(r);
    if o.shake_timer != 0 {
        let base = Vec3 { x: (o.shake_origin_x as i32) << 16, y: o.pos.y, z: (o.shake_origin_z as i32) << 16 };
        b.objects.get_mut(r).shake_timer -= 1;
        let pos = crate::kinds::spark::jitter(b, 3, base);
        b.objects.get_mut(r).pos = pos;
    } else {
        let o = b.objects.get_mut(r);
        o.pos.x = (o.pos.x & 0xFFFF) | ((o.shake_origin_x as i32) << 16);
        o.pos.z = (o.pos.z & 0xFFFF) | ((o.shake_origin_z as i32) << 16);
    }
}
