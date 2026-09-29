//! Hit reactions: deletion (action 2), flinch (3), paralysis (4), drag
//! (5), freeze (6) and bubble (7). See objects-and-player.md §H4-§H6.

use super::{
    ai, ai_mut, cancel_semi_intangible, clear_bubble, clear_flag1, clear_flag2, clear_freeze, clear_invulnerable,
    clear_paralysis, coll, coll_mut, coordinates_to_panel, flag1, panel_coordinates, panel_kind, refresh_form_overlay,
    reset_charge, set_flag1, snap_to_future_panel,
};
use crate::actor::{request, status as ai_status};
use crate::battle::Battle;
use crate::collision::{f1, timer};
use crate::data::player::SlideVector;
use crate::data::player_generated::{BUBBLE_BOB, ICE_VECTORS, PUSH_VECTORS, ROAD_VECTORS};
use crate::field::{self, PanelType};
use crate::object::{DragStep, ObjectRef, PanelPos, flags, state};

// ---- Deletion (action 2) -------------------------------------------------------

/// Action 2, `sub_80173F4`: deletion (§H6).
pub(super) fn deletion(b: &mut Battle, r: ObjectRef) {
    // sprite_forceWhitePalette every tick.
    match b.objects.get(r).phase {
        0 => begin_deletion(b, r),
        4 => explode(b, r),
        8 => {
            // sub_80174AA
            let o = b.objects.get_mut(r);
            o.timer = o.timer.wrapping_sub(1);
            if o.timer == 0 {
                o.phase = 0xC;
                o.phase_init = 0;
            }
        }
        _ => fade_out(b, r),
    }
}

/// `sub_801741C`: stop colliding, drop chips, reservations, barrier and
/// the charge glow, and leave the side's alive list: the battle is over
/// from here.
fn begin_deletion(b: &mut Battle, r: ObjectRef) {
    let c = coll_mut(b, r);
    c.region = 0;
    // sub_801A5E2
    c.links[crate::collision::link::CONFUSE] = None;
    c.links[crate::collision::link::BLIND] = None;
    cancel_semi_intangible(b, r);
    reset_charge(b, r);
    // sub_801DC36: HUD.
    let o = b.objects.get_mut(r);
    o.chips_held = 0;
    o.chip = 0xFFFF;
    let fp = o.future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    // sub_801A7F4: the barrier goes (and the game forgets its visual,
    // AIData+0x60).
    coll_mut(b, r).barrier = 0;
    // The charge glow sees its slot cleared and ends itself.
    ai_mut(b, r).charge_glow = None;
    if b.objects.get(r).params[1] < 1 {
        remove_from_alive(b, r);
    }
    if super::is_mode_40(b) {
        panic!("deletion link bookkeeping (sub_802EF5C) is not implemented yet");
    }
    let o = b.objects.get_mut(r);
    o.phase = 4;
    o.phase_init = 0;
}

/// `sub_800A11C`: one fewer alive navi on the side, and out of the alive
/// actor lists.
fn remove_from_alive(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.round.alive[side] = b.round.alive[side].wrapping_sub(1);
    for slot in b.round.alive_actors.iter_mut().flatten() {
        if *slot == Some(r) {
            *slot = None;
        }
    }
}

/// `sub_801746E`: two explosions (not in time stop).
fn explode(b: &mut Battle, r: ObjectRef) {
    if b.is_time_stop() {
        return;
    }
    let o = b.objects.get_mut(r);
    o.anim = 2;
    o.related[0] = None;
    o.prevent_anim = 0;
    let pos = o.pos;
    let a = ai_mut(b, r);
    a.full_synchro_aura = None;
    a.overlay = None;
    // Sound 0x6C. The second call reuses whatever registers the first
    // left: its X/Y are GBA list-node addresses (objects-and-player.md
    // §A.3), not modeled.
    crate::kinds::effect::spawn(b, pos, 3, 0, 0, 0);
    crate::kinds::effect::spawn(b, pos, 3, 0, 0, 0);
    let o = b.objects.get_mut(r);
    o.timer = 0x15;
    o.phase = 8;
    o.phase_init = 0;
}

/// `sub_80174BE`: fade out over 32 ticks, then the destroy state.
fn fade_out(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    o.timer = o.timer.wrapping_add(1);
    if o.timer != 0x20 {
        // Mosaic and alpha follow the timer.
        return;
    }
    o.flags &= !flags::VISIBLE;
    // sub_802CDD0: the side's damage-carry record forgets this navi.
    let side = o.alliance as usize;
    if b.damage_carry[side].target == Some(r) {
        b.damage_carry[side].target = None;
    }
    death_hook(b, r);
    let o = b.objects.get_mut(r);
    o.state = state::DESTROY;
    o.action = 0;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_8011020`: per-navi death hook (MegaMan's ends the form overlay).
fn death_hook(b: &mut Battle, r: ObjectRef) {
    let rec = super::navi_record(b, r);
    match rec.ai_index {
        0 => {
            // sub_80111B8
            if let Some(overlay) = b.objects.get_mut(r).related[1].take() {
                let o = b.objects.get_mut(overlay);
                o.state = state::DESTROY;
                o.action = 0;
                o.phase = 0;
                o.phase_init = 0;
            }
        }
        1 | 6 | 9 | 13 | 14 | 16 | 18 | 19 | 24 | 25.. => {
            panic!("navi death hook for AI index {} is not implemented yet", rec.ai_index)
        }
        _ => {}
    }
}

// ---- Common reaction entry ---------------------------------------------------------

/// The entry the paralysis, freeze and bubble actions share: using an
/// action, not guarding, moving, flinching or dragged, charge and state
/// bits dropped, snapped onto the destination panel (unless sliding),
/// reaction animation.
fn enter_reaction(b: &mut Battle, r: ObjectRef, anim: u8) {
    set_flag1(b, r, f1::USING_ACTION);
    clear_flag1(b, r, f1::DRAG | f1::FLINCHING | f1::MOVING | f1::GUARD);
    cancel_semi_intangible(b, r);
    ai_mut(b, r).status &= !0x20_005F;
    reset_charge(b, r);
    if flag1(b, r) & f1::SLIDING == 0 {
        snap_to_future_panel(b, r);
        b.objects.get_mut(r).pos.z = 0;
    }
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = 0xFF;
}

/// The end of the reaction entry: links dropped, the reaction counted.
fn finish_reaction_entry(b: &mut Battle, r: ObjectRef) {
    b.objects.get_mut(r).related[0] = None;
    ai_mut(b, r).overlay = None;
    let side = b.objects.get(r).alliance;
    b.bump_side_stat(side, 3, 1);
    b.objects.get_mut(r).phase_init = 4;
}

/// Leave a reaction for the idle action.
fn end_reaction(b: &mut Battle, r: ObjectRef) {
    ai_mut(b, r).requests &= !(request::ATTACKS | request::MODE9_A);
    clear_flag1(b, r, f1::USING_ACTION);
    let o = b.objects.get_mut(r);
    o.anim = 0;
    o.action = 8;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_80F06CE`: MegaMan's flinch and drag hook resets the form
/// overlay.
fn reset_form_overlay(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).related[1].is_some() {
        panic!("form overlay reset (sub_80C44D2) is not implemented yet");
    }
}

/// `sub_800F3E8`: the per-form flinch hook.
fn flinch_hook(b: &mut Battle, r: ObjectRef) {
    match ai(b, r).ai_index {
        0 => reset_form_overlay(b, r),
        1 => panic!("flinch hook sub_80F0700 is not implemented yet"),
        _ => {}
    }
}

/// `sub_800F404`: the per-form drag hook.
fn drag_hook(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).ai_index == 0 {
        reset_form_overlay(b, r);
    }
}

// ---- Flinch (action 3) --------------------------------------------------------------

/// Action 3, `sub_80174FE`: 24 ticks of flinching; ends paralysis,
/// freeze and bubble.
pub(super) fn flinch(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        flinch_hook(b, r);
        set_flag1(b, r, f1::USING_ACTION | f1::FLINCHING);
        clear_paralysis(b, r);
        clear_freeze(b, r);
        clear_bubble(b, r);
        cancel_semi_intangible(b, r);
        ai_mut(b, r).status &= !0x20_005F;
        clear_flag1(b, r, f1::DRAG | f1::MOVING | f1::GUARD);
        reset_charge(b, r);
        if flag1(b, r) & f1::SLIDING == 0 {
            snap_to_future_panel(b, r);
            b.objects.get_mut(r).pos.z = 0;
        }
        let o = b.objects.get_mut(r);
        o.anim = 1;
        o.anim_loaded = 0xFF;
        refresh_form_overlay(b, r);
        finish_reaction_entry(b, r);
        b.objects.get_mut(r).timer = 0x17;
    }
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t >= 0 {
        return;
    }
    clear_flag1(b, r, f1::USING_ACTION | f1::FLINCHING);
    ai_mut(b, r).requests &= !(request::ATTACKS | request::TRAP_400 | request::MODE9_A);
    let o = b.objects.get_mut(r);
    o.anim = 0;
    o.action = 8;
    o.phase = 0;
    o.phase_init = 0;
}

// ---- Paralysis, freeze, bubble (actions 4, 6, 7) ------------------------------------

/// Mashing buttons takes an extra tick off a status timer; returns true
/// once the status flag is gone.
fn mash(b: &mut Battle, r: ObjectRef, t: usize, flag: u32) -> bool {
    if ai(b, r).pad.pressed != 0 {
        let c = coll_mut(b, r);
        let v = c.status_timers[t] as i16 as i32 - 1;
        c.status_timers[t] = if v > 0 { v as u16 } else { 0 };
        if v <= 0 {
            clear_flag1(b, r, flag);
        }
    }
    flag1(b, r) & flag == 0
}

/// Action 4, `sub_80175B8`: paralyzed until the timer (or mashing) ends
/// it.
pub(super) fn paralysis(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        // sub_800F394: no per-form hook.
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r);
    }
    if mash(b, r, timer::PARALYZE, f1::PARALYZED) {
        end_reaction(b, r);
    }
}

/// Action 6, `sub_8017688`: frozen; loses the flash and invulnerability.
pub(super) fn freeze(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        // sub_800F3B0: no per-form hook.
        coll_mut(b, r).status_timers[timer::FLASH] = 0;
        clear_invulnerable(b, r);
        // Sound 0x118.
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r);
    }
    if mash(b, r, timer::FREEZE, f1::FROZEN) {
        end_reaction(b, r);
    }
}

/// Action 7, `sub_8017768`: bubbled, bobbing up and down.
pub(super) fn bubble(b: &mut Battle, r: ObjectRef) {
    if b.objects.get(r).phase_init == 0 {
        // sub_800F3CC: no per-form hook.
        clear_invulnerable(b, r);
        // Sound 0x12D.
        enter_reaction(b, r, 2);
        finish_reaction_entry(b, r);
    }
    let popped = mash(b, r, timer::BUBBLE, f1::BUBBLED);
    let t = coll(b, r).status_timers[timer::BUBBLE] as i16 as i32;
    b.objects.get_mut(r).pos.z = (BUBBLE_BOB[((t >> 2) & 0x1F) as usize] as i32) << 16;
    if popped {
        b.objects.get_mut(r).pos.z = 0;
        // Sound 0x124.
        end_reaction(b, r);
    }
}

// ---- Drag (action 5) --------------------------------------------------------------

/// Action 5, `sub_80178B6`: knocked back along the push vector, then a
/// short recovery (§H4.3).
pub(super) fn drag(b: &mut Battle, r: ObjectRef) {
    match b.objects.get(r).drag_step {
        DragStep::Start => start_drag(b, r),
        DragStep::Slide => step_drag(b, r),
        DragStep::Recover => recover_from_drag(b, r),
    }
}

/// `sub_80178D4`.
fn start_drag(b: &mut Battle, r: ObjectRef) {
    drag_hook(b, r);
    set_flag1(b, r, f1::USING_ACTION | f1::DRAG);
    b.objects.get_mut(r).related[0] = None;
    ai_mut(b, r).overlay = None;
    let f = flag1(b, r);
    let anim = if f & f1::PARALYZED != 0 {
        2
    } else if f & f1::SUPERARMOR != 0 {
        0
    } else {
        1
    };
    let o = b.objects.get_mut(r);
    o.anim = anim;
    o.anim_loaded = 0xFF;
    refresh_form_overlay(b, r);
    reset_charge(b, r);
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    super::set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    let o = b.objects.get_mut(r);
    o.pos.z &= !0xFFFF;
    clear_flag1(b, r, f1::SLIDING | f1::FLINCHING | f1::MOVING | f1::GUARD);
    cancel_semi_intangible(b, r);
    let fp = b.objects.get(r).future_panel;
    b.unreserve_panel(r, fp.x, fp.y);
    let side = b.objects.get(r).alliance;
    b.bump_side_stat(side, 3, 1);
    let v = slide_vector(b, r);
    let o = b.objects.get_mut(r);
    o.slide_dx = v.dx as u8;
    o.slide_dy = v.dy as u8;
    o.timer2 = v.tiles as u16;
    if v.tiles != 0 {
        let p = o.panel;
        let target = PanelPos { x: (p.x as i8 + v.dx) as u8, y: (p.y as i8 + v.dy) as u8 };
        if can_slide_to(b, r, target) {
            let o = b.objects.get_mut(r);
            o.vel.x = v.dx as i32 * 0xA_0000;
            o.vel.y = v.dy as i32 * 0x6_0000;
            o.future_panel = target;
            b.reserve_panel(r, target.x, target.y);
            b.objects.get_mut(r).drag_step = DragStep::Slide;
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.timer = 0x18;
    o.drag_step = DragStep::Recover;
}

/// `sub_8017992`: move toward the destination; on arrival continue (one
/// more tile on ice) or stop.
fn step_drag(b: &mut Battle, r: ObjectRef) {
    let fp = b.objects.get(r).future_panel;
    let (tx, ty) = panel_coordinates(fp.x, fp.y);
    let o = b.objects.get_mut(r);
    let old_x = o.pos.x;
    o.pos.x = o.pos.x.wrapping_add(o.vel.x);
    let mut arrived = passed(o.pos.x, old_x, tx);
    if !arrived {
        let old_y = o.pos.y;
        o.pos.y = o.pos.y.wrapping_add(o.vel.y);
        arrived = passed(o.pos.y, old_y, ty);
    }
    if !arrived {
        let o = b.objects.get_mut(r);
        o.panel = coordinates_to_panel(o.pos.x, o.pos.y);
        b.update_collision_panels(r);
        return;
    }
    b.unreserve_panel(r, fp.x, fp.y);
    if panel_kind(b, fp) == PanelType::Ice && coll(b, r).element != 2 {
        let o = b.objects.get_mut(r);
        o.timer2 = o.timer2.wrapping_add(1);
    }
    let o = b.objects.get_mut(r);
    let left = o.timer2 as i32 - 1;
    o.timer2 = left as u16;
    if left > 0 {
        let (dx, dy) = (o.slide_dx as i8, o.slide_dy as i8);
        let next = PanelPos { x: (fp.x as i8 + dx) as u8, y: (fp.y as i8 + dy) as u8 };
        if can_slide_to(b, r, next) {
            b.objects.get_mut(r).future_panel = next;
            b.reserve_panel(r, next.x, next.y);
            return;
        }
    }
    let o = b.objects.get_mut(r);
    o.panel = o.future_panel;
    super::set_coordinates_from_panel(b, r);
    b.update_collision_panels(r);
    let o = b.objects.get_mut(r);
    o.timer = 0x14;
    o.drag_step = DragStep::Recover;
}

/// `sub_800E6E8`: whether a step from `old` to `new` reached `target`
/// (moving left, exactly reaching it does not count).
fn passed(new: i32, old: i32, target: i32) -> bool {
    if new > old { target > old && target <= new } else { target > new && target <= old }
}

/// `sub_8017A38`: wait, then back to idle (or to paralysis).
fn recover_from_drag(b: &mut Battle, r: ObjectRef) {
    let o = b.objects.get_mut(r);
    let t = o.timer as i32 - 1;
    o.timer = t as u16;
    if t >= 0 {
        return;
    }
    if flag1(b, r) & f1::PARALYZED != 0 {
        clear_flag1(b, r, f1::DRAG);
        let o = b.objects.get_mut(r);
        o.action = 4;
        o.phase = 0;
        o.phase_init = 0;
        return;
    }
    clear_flag1(b, r, f1::USING_ACTION | f1::DRAG | f1::SLIDING | f1::PARALYZED);
    let a = ai_mut(b, r);
    a.requests &= !(request::ATTACKS | request::TRAP_400 | request::MODE9_A);
    a.status &= !ai_status::HEAT_TRAP;
    clear_flag2(b, r, 0x10);
    let o = b.objects.get_mut(r);
    o.slide_state = 0;
    o.anim = 0;
    o.anim_loaded = 0xFF;
    refresh_form_overlay(b, r);
    let o = b.objects.get_mut(r);
    o.action = 8;
    o.phase = 0;
    o.phase_init = 0;
}

/// `sub_800E468`: the slide vector for the slide type, none when the
/// first panel is not open.
fn slide_vector(b: &Battle, r: ObjectRef) -> SlideVector {
    let o = b.objects.get(r);
    let front = if o.alliance == 0 { 1 } else { -1 };
    let facing = |v: SlideVector| SlideVector { dx: v.dx * front, ..v };
    let v = match o.slide_type {
        0 => SlideVector::NONE,
        1 => {
            // sub_800E548: from the hit modifier bits.
            let hm = coll(b, r).hit_mod_final;
            let off = if hm & 0x80 != 0 { 5 } else { 0 };
            let bits = (hm & 0x7F) >> 2;
            let i = (0..4).find(|&i| bits & (1 << i) != 0).unwrap_or(4);
            facing(PUSH_VECTORS[i + off])
        }
        2 => facing(*ICE_VECTORS.get(coll(b, r).direction as usize).expect("ice slide direction")),
        3 => {
            panel_kind(b, o.panel).road_index().map_or(SlideVector::NONE, |i| ROAD_VECTORS[i])
        }
        t => panic!("slide type {t} reads past its table"),
    };
    let p = o.panel;
    let first = PanelPos { x: (p.x as i8 + v.dx) as u8, y: (p.y as i8 + v.dy) as u8 };
    if can_slide_to(b, r, first) { v } else { SlideVector::NONE }
}

/// `sub_800E5AC`: a slide may enter (x, y): like a step, but the floor
/// rule depends only on AirShoes.
fn can_slide_to(b: &Battle, r: ObjectRef, p: PanelPos) -> bool {
    if !field::is_valid(p.x, p.y) {
        return false;
    }
    let airshoes = flag1(b, r) & f1::AIRSHOE != 0;
    b.field.meets(p.x, p.y, field::step_rule(airshoes, b.objects.get(r).alliance))
}
