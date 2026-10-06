//! Action 0x1C as a form's revert (`sub_8015614`, the same code in EXE5) and
//! a Cross breaking (`sub_8015766`): the framework's. The change into a form
//! (`sub_8014A38`, the original's action 0x1C too) is the action the form
//! names (`FormData::change`): EXE6's five sequences are EXE6's rules/forms's,
//! content/exe6/rules/forms (docs/design/rules-in-luau.md §3.1). See
//! docs/engine/battle-flow.md §3.4.1 and objects-and-player.md §12.9-§12.10.

use crate::actor::status;
use crate::battle::{Battle, battle_flags};
use crate::collision::{f1, link, timer};
use crate::content::{EffectRole, SoundRole};
use super::ActionVars;
use crate::kinds::player::{
    ai, ai_mut, clear_flag1, clear_flag2, clear_invulnerable, coll_mut, exit_attack_state, form,
    reset_status, set_mood, snap_to_future_panel, stats, stats_mut,
};
use crate::kinds::{effect, full_synchro_aura};
use crate::object::{ObjectRef, Vec3, flags};

/// The form change's own state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vars {
    /// AIAttackVars+0x10: ticks left in the current step.
    pub timer: u16,
}

fn vars(b: &mut Battle, r: ObjectRef) -> &mut Vars {
    match &mut ai_mut(b, r).attack.action {
        ActionVars::FormChange(v) => v,
        v => panic!("form change without its state ({v:?})"),
    }
}

fn set_timer(b: &mut Battle, r: ObjectRef, ticks: u16) {
    vars(b, r).timer = ticks;
}

/// Count the timer down; true while it is still above 0.
fn timer_running(b: &mut Battle, r: ObjectRef) -> bool {
    let v = vars(b, r);
    v.timer = v.timer.wrapping_sub(1);
    v.timer as i16 > 0
}

/// FuturePanel → panel, the reservation dropped, coordinates and
/// collision panels from it.
fn land(b: &mut Battle, r: ObjectRef) {
    snap_to_future_panel(b, r);
}

/// `sub_800F46C` + `sub_800F2C6`: face the default way under the standard
/// column patterns, and the sprite with it.
pub(crate) fn face_default(b: &mut Battle, r: ObjectRef) {
    if matches!(b.panel_pattern(), 0x38 | 0x30 | 0x3C) {
        b.objects.get_mut(r).flip = 0;
    }
    let o = b.objects.get(r);
    let facing = o.alliance ^ o.flip;
    b.objects.sprite_mut(r).look.set_flip(facing);
}

/// `sub_80C4C3A` on AIData+0x5C: the Full Synchro aura goes (with none,
/// the game's stores land in BIOS memory).
pub(crate) fn end_full_synchro_aura(b: &mut Battle, r: ObjectRef) {
    if let Some(aura) = ai(b, r).full_synchro_aura {
        full_synchro_aura::end(b, aura);
    }
}

/// The overlay's Param3 = 1 and flags 0x14, if there is one.
fn keep_overlay_stepping(b: &mut Battle, r: ObjectRef) {
    if let Some(o) = b.objects.get(r).related[1] {
        form::keep_overlay_stepping(b, o);
    }
}

/// `sub_80158FA`: movement, reactions and the slower statuses end.
pub(crate) fn drop_statuses(b: &mut Battle, r: ObjectRef) {
    clear_flag1(
        b,
        r,
        f1::BUBBLED | f1::DRAG | f1::FROZEN | f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING,
    );
    // The slide request.
    clear_flag2(b, r, 0x10);
    b.objects.get_mut(r).slide_state = 0;
    ai_mut(b, r).status &= !(status::HEAT_TRAP | status::TRAP_ARMED);
    let c = coll_mut(b, r);
    c.status_timers[timer::PARALYZE] = 0;
    c.status_timers[timer::FREEZE] = 0;
    c.status_timers[timer::BUBBLE] = 0;
    c.links[link::FREEZE] = None;
    c.links[link::BUBBLE] = None;
}

/// EXE5's soul change's first step (0x08011FAC): the part of
/// `sub_80158FA` it does itself (flags 0x1C40, the slide request, the
/// slide's step).
pub(crate) fn stop_moving(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING);
    clear_flag2(b, r, 0x10);
    b.objects.get_mut(r).slide_state = 0;
}

// ---- Reverting ---------------------------------------------------------------

/// `sub_8015614` / `sub_801562C`: back to base MegaMan (the Beast Out ran
/// out, or a Cross navi ended). A navi paralyzed, frozen or bubbled when
/// the reversion started goes back to that afterwards.
pub(in crate::kinds::player) fn revert(b: &mut Battle, r: ObjectRef) {
    if ai(b, r).attack.step != 0 {
        panic!("form reversion step {:#x} runs off its table (sub_8015614)", ai(b, r).attack.step);
    }
    if !matches!(ai(b, r).attack.action, ActionVars::FormChange(_)) {
        ai_mut(b, r).attack.action = ActionVars::FormChange(Vars::default());
    }
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        if let Some(o) = b.objects.get(r).related[1] {
            b.objects.get_mut(o).flags |= flags::RUN_WHILE_PAUSED | flags::RUN_WHILE_DIMMED;
        }
        use crate::kinds::player::NaviAction as A;
        let resumes = ai(b, r).saved_word.is_some_and(|s| matches!(s.action, A::Paralysis | A::Freeze | A::Bubble));
        ai_mut(b, r).attack.step_init = if resumes { 2 } else { 1 };
        b.sound(SoundRole::Fade);
        land(b, r);
        face_default(b, r);
        let pos = b.objects.get(r).pos;
        let look = b.roles().effect(EffectRole::Deletion);
        if let Some(e) = effect::spawn(b, Vec3 { z: pos.z.wrapping_add(0x14_0000), ..pos }, look, 0, 0, 0) {
            b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        }
        clear_flag1(b, r, f1::UNTOUCHABLE | f1::SLIDING | f1::FLINCHING | f1::MOVING);
        clear_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_state = 0;
        let current = stats(b, r).form;
        form::take_off_overlay(b, r, current);
        let base = b.content.base_form_for(stats(b, r).navi);
        let sprite = b.content.form(base).sprite;
        let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
        let s = b.objects.sprite_mut(r);
        s.load(sprite);
        s.look.shadow = crate::object::sprite::Shadow::Ground;
        if !resumes {
            let o = b.objects.get_mut(r);
            o.anim = 0;
            o.anim_loaded = 0xFF;
        }
        let anim = b.objects.get(r).anim;
        let s = b.objects.sprite_mut(r);
        s.set_animation(anim, &b.content);
        s.look.set_flip(flip);
        s.look.white = true;
        b.objects.get_mut(r).flags &= !flags::NO_SPRITE_UPDATE;
        spend_form(b, r);
        stats_mut(b, r).form = base;
        // sub_8015B22(0): the navi's own identity again (MegaMan's: only
        // a navi that changes form has a form to revert).
        let navi = stats(b, r).navi;
        b.objects.get_mut(r).identity = b.content.form_identity(navi, base);
        reset_status(b, r);
        // sub_80143B4: anger ends (the mood is left alone).
        calm_down(b, r);
        clear_invulnerable(b, r);
        form::put_on_overlay(b, r, base);
        b.objects.get_mut(r).related[0] = None;
        let a = ai_mut(b, r);
        a.overlay = None;
        // The lock-on marker frees itself once unlinked.
        a.target_marker = None;
        set_timer(b, r, 0x1E);
    }
    if timer_running(b, r) {
        return;
    }
    ai_mut(b, r).requests &= !0x1000_863D;
    ai_mut(b, r).status &= !(status::HEAT_TRAP | status::TRAP_ARMED | status::REVERTING_FORM);
    if ai(b, r).attack.step_init != 2 {
        return exit_attack_state(b, r);
    }
    // Back to the saved status action.
    let s = ai_mut(b, r).saved_word.take().unwrap_or_default();
    crate::kinds::player::set_navi_action(b, r, s.action);
    let o = b.objects.get_mut(r);
    o.state = s.state;
    o.phase = s.phase;
    o.phase_init = s.phase_init;
}

/// `sub_80158CC`: mood 0x80 (stored directly); then the side's rules'
/// `form_reverted` (EXE6's: outside battle mode 1, a Beast Out is used up,
/// and Beast Over exhausts the navi).
fn spend_form(b: &mut Battle, r: ObjectRef) {
    let side = b.objects.get(r).alliance as usize;
    b.stats[side].mood = 0x80;
    b.rules_form_reverted(side as u8, r);
}

/// `sub_80143B4`: anger ends, without touching the mood.
fn calm_down(b: &mut Battle, r: ObjectRef) {
    clear_flag1(b, r, f1::ANGER);
    clear_flag2(b, r, 0x200);
    let a = ai_mut(b, r);
    a.anger = 0;
    a.stun_ticks = 0;
}

// ---- A Cross breaking --------------------------------------------------------

/// `sub_8015766`: a weakness hit breaks the Cross (from action dispatch,
/// with `status::CROSS_BREAKING`): dimming falls while the navi drops to
/// base form (a Cross Beast to its Beast), then 30 ticks later it goes on.
/// True while it runs. EXE5's (0x080122C8, `FormBreak::AnyForm`: a dark chip
/// used in a soul) drops any form to the base form, and lacks animation 2
/// and the overlay's refresh, the overlay's kept stepping, the collision
/// region's removal and return, and the flags 0x80110000 and statuses
/// 0x200800 it clears.
pub(in crate::kinds::player) fn break_cross(b: &mut Battle, r: ObjectRef) -> bool {
    let any_form = b.game_rules().form_break == crate::content::FormBreak::AnyForm;
    if !matches!(ai(b, r).attack.action, ActionVars::FormChange(_)) {
        ai_mut(b, r).attack.action = ActionVars::FormChange(Vars::default());
    }
    b.objects.sprite_mut(r).look.white = true;
    if ai(b, r).attack.step_init == 0 {
        b.set_flags(battle_flags::DIMMED);
        b.sound(SoundRole::Fade);
        land(b, r);
        if !any_form {
            let o = b.objects.get_mut(r);
            o.anim = 2;
            o.anim_loaded = 0xFF;
            crate::kinds::player::refresh_form_overlay(b, r);
        }
        face_default(b, r);
        let pos = b.objects.get(r).pos;
        let look = b.roles().effect(EffectRole::Deletion);
        if let Some(e) = effect::spawn(b, Vec3 { z: pos.z.wrapping_add(0x14_0000), ..pos }, look, 0, 0, 0) {
            b.objects.get_mut(e).flags |= flags::RUN_WHILE_PAUSED;
        }
        let moving = f1::SLIDING | f1::PARALYZED | f1::FLINCHING | f1::MOVING;
        clear_flag1(b, r, if any_form { moving } else { moving | f1::BUBBLED | f1::DRAG | f1::FROZEN });
        clear_flag2(b, r, 0x10);
        b.objects.get_mut(r).slide_state = 0;
        if !any_form {
            keep_overlay_stepping(b, r);
        }
        let current = stats(b, r).form;
        form::take_off_overlay(b, r, current);
        // What the form drops to (its `breaks_to`; the original's by the
        // form's number: the base form from a Cross, the game's Beast
        // from a Cross in Beast Out); a form without one stays. EXE5's: the
        // base form.
        let new = if any_form {
            b.content.base_form_for(stats(b, r).navi)
        } else {
            b.content.form(current).breaks_to.unwrap_or(current)
        };
        // sub_800FC9E(MegaMan, the new form).
        let sprite = b.content.form(new).sprite;
        let flip = b.objects.get(r).alliance ^ b.objects.get(r).flip;
        let s = b.objects.sprite_mut(r);
        s.load(sprite);
        s.look.shadow = crate::object::sprite::Shadow::Ground;
        let o = b.objects.get_mut(r);
        o.flags &= !flags::NO_SPRITE_UPDATE;
        o.anim = 0;
        o.anim_loaded = 0xFF;
        let s = b.objects.sprite_mut(r);
        s.set_animation(0, &b.content);
        s.look.set_flip(flip);
        s.look.white = true;
        stats_mut(b, r).form = new;
        // sub_8015B22
        let navi = stats(b, r).navi;
        b.objects.get_mut(r).identity = b.content.form_identity(navi, new);
        let side = b.objects.get(r).alliance;
        set_mood(b, side, 0x80);
        reset_status(b, r);
        calm_down(b, r);
        form::put_on_overlay(b, r, new);
        if !any_form {
            keep_overlay_stepping(b, r);
        }
        clear_invulnerable(b, r);
        b.objects.get_mut(r).related[0] = None;
        ai_mut(b, r).overlay = None;
        if !any_form {
            // object_clearCollisionRegion
            coll_mut(b, r).region = None;
        }
        set_timer(b, r, 0x1E);
        ai_mut(b, r).attack.step_init = 4;
    }
    if timer_running(b, r) {
        return true;
    }
    if any_form {
        ai_mut(b, r).status &= !status::CROSS_BREAKING;
        b.clear_flags(battle_flags::DIMMED);
        let a = &mut ai_mut(b, r).attack;
        a.step = 0;
        a.step_init = 0;
        return false;
    }
    ai_mut(b, r).status &= !(status::CROSS_BREAKING | status::HEAT_TRAP | status::TRAP_ARMED);
    if let Some(o) = b.objects.get(r).related[1] {
        form::normal_overlay_stepping(b, o);
    }
    b.clear_flags(battle_flags::DIMMED);
    // object_setCollisionRegion(1)
    coll_mut(b, r).region = b.anchor_region();
    let a = &mut ai_mut(b, r).attack;
    a.step = 0;
    a.step_init = 0;
    false
}
